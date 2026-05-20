//! Per-space Halstead token routing.
//!
//! Most Mehen analyzers compute Halstead by walking a flat token stream
//! after the AST walk has already opened (and possibly closed) every
//! function / class / closure space. A naive token sweep that records
//! every event onto the unit space is wrong: the per-space `MetricSpace`
//! entries in the JSON report end up with zero Halstead even though the
//! root rollup is correct.
//!
//! This module owns the bookkeeping that maps each token to the deepest
//! enclosing scope and propagates the per-space counts through the
//! parent chain via set-union (matching what
//! [`crate::HalsteadBuilder::merge`] does on every other code path).
//!
//! # Walker flow
//!
//! 1. As the walker's `open_space` (or equivalent) hook runs, it calls
//!    [`SpaceRangeTracker::record_open`] with the `SpaceId` minted by
//!    [`crate::MetricTreeBuilder::open`] and the AST node's byte range.
//!    The tracker uses insertion-vs-still-active order to recover the
//!    parent link — each new entry's parent is the most recent entry
//!    whose byte range encloses it.
//! 2. As the walker's `close_space` runs (still during the AST walk),
//!    it calls [`SpaceRangeTracker::record_close`] with the closing
//!    space's `LocStats` and `CyclomaticStats`. These are the inputs
//!    the [Maintainability Index][mi] needs alongside Halstead, so we
//!    stash them now while they're still in scope.
//! 3. After the AST walk finishes, the walker iterates the source's
//!    token stream; for each operator/operand event it calls
//!    [`SpaceRangeTracker::observe_operator`] / `observe_operand` with
//!    the token's byte range. The tracker routes the event to the
//!    deepest still-open scope at that range, falling back to the unit
//!    [`HalsteadBuilder`] when no recorded entry encloses it.
//! 4. The walker calls [`SpaceRangeTracker::finalize_into_tree`] which:
//!    - propagates each entry's `HalsteadBuilder` up its parent chain
//!      (set-union for `n1`/`n2`, sum for `N1`/`N2`),
//!    - merges the rolled-up sets into `unit_halstead` so the unit
//!      space's keys reflect the file-wide rollup,
//!    - overwrites the Halstead-derived metric keys (and the
//!      Halstead-dependent MI keys) inside the matching `MetricSpace`
//!      of the `tree`.
//!
//! [mi]: crate::MiStats

use std::collections::HashMap;

use mehen_core::{MetricKey, MetricSet, MetricSpace, SpaceId};

use crate::cyclomatic::CyclomaticStats;
use crate::halstead::HalsteadStats;
use crate::halstead_builder::{HalsteadBuilder, HalsteadOperand, HalsteadOperator};
use crate::keys;
use crate::loc::LocStats;
use crate::mi::MiStats;
use crate::state::publish_halstead;

/// Tracks every space opened during the AST walk so a post-AST token
/// sweep can route each operator/operand event to the deepest enclosing
/// scope.
///
/// The unit (`SpaceId(0)`) is implicit — any token that does not fall
/// inside a recorded entry routes to the caller-supplied unit
/// [`HalsteadBuilder`].
#[derive(Debug, Default)]
pub struct SpaceRangeTracker {
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    space_id: SpaceId,
    start: u32,
    end: u32,
    /// Index of the parent entry in `entries`, or `None` when the
    /// parent is the unit scope. `record_open` is called in source
    /// order while the AST walk is still descending; the most recent
    /// still-active entry whose byte range encloses ours is the
    /// parent.
    parent: Option<usize>,
    halstead: HalsteadBuilder,
    /// AST-driven LOC + cyclomatic snapshots taken at space-close
    /// time. The overlay recomputes Halstead-derived keys + MI from
    /// these against the post-token-sweep Halstead, and folds
    /// `loc_token_events` into a final per-space LocStats so the
    /// overlay also corrects per-space PLOC/CLOC keys (PR #95
    /// discussion_r3265962147 — without routing tokens into the
    /// active space, post-AST token sweeps left
    /// `root.spaces[*].metrics["loc.ploc"]` at zero).
    loc: LocStats,
    cyclomatic: CyclomaticStats,
    /// Token-driven LOC events routed to this space. Always merged
    /// into `loc` (and propagated up the parent chain) by
    /// [`SpaceRangeTracker::finalize_into_tree`] before the LOC keys
    /// are overlaid.
    loc_token_events: LocStats,
}

impl SpaceRangeTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a newly-opened space's `SpaceId` and byte range. Call
    /// from the walker's `open_space` hook, right after the
    /// [`MetricTreeBuilder::open`][crate::MetricTreeBuilder::open]
    /// call has minted the `SpaceId`.
    pub fn record_open(&mut self, space_id: SpaceId, start: u32, end: u32) {
        let parent = self.deepest_enclosing_index(start, end);
        self.entries.push(Entry {
            space_id,
            start,
            end,
            parent,
            halstead: HalsteadBuilder::new(),
            loc: LocStats::default(),
            cyclomatic: CyclomaticStats::default(),
            loc_token_events: LocStats::default(),
        });
    }

    /// Stash the LOC and cyclomatic snapshots needed to recompute MI
    /// after the token sweep. Call from the walker's `close_space`
    /// hook with the about-to-be-published state's values. Quietly
    /// no-ops when the `space_id` was not previously recorded via
    /// [`record_open`] — the unit scope is implicit.
    ///
    /// The captured AST `LocStats` also seeds the entry's
    /// `loc_token_events.ploc_lines` so that subsequent
    /// `observe_comment` calls correctly classify a comment as
    /// "code-comment" (same line as code) vs. "only-comment" — without
    /// the seed, every token-stream comment inside a function body
    /// reads as only-comment because the tracker's accumulator started
    /// fresh and did not see the AST-walk's PLOC observations.
    pub fn record_close(
        &mut self,
        space_id: SpaceId,
        loc: &LocStats,
        cyclomatic: &CyclomaticStats,
    ) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.space_id == space_id) {
            entry.loc = loc.clone();
            entry.cyclomatic = cyclomatic.clone();
            // Seed the token accumulator's `ploc_lines` from the AST
            // snapshot so `observe_comment`'s "after-code on same
            // line" check sees the function's existing code lines.
            entry.loc_token_events.seed_ploc_lines(loc);
        }
    }

    /// Return the deepest entry whose range strictly encloses
    /// `[start, end)`, or `None` if no such entry exists.
    fn deepest_enclosing_index(&self, start: u32, end: u32) -> Option<usize> {
        // Reverse insertion order — the deepest still-active entry is
        // the most recent one whose range encloses ours.
        self.entries
            .iter()
            .enumerate()
            .rev()
            .find(|(_, e)| e.start <= start && end <= e.end)
            .map(|(i, _)| i)
    }

    /// Observe an operator event into the deepest scope containing
    /// `[span_start, span_end)`, falling back to `unit_halstead` when
    /// no recorded entry encloses it.
    pub fn observe_operator(
        &mut self,
        span_start: u32,
        span_end: u32,
        unit_halstead: &mut HalsteadBuilder,
        op: HalsteadOperator,
    ) {
        match self.deepest_enclosing_index(span_start, span_end) {
            Some(idx) => self.entries[idx].halstead.observe_operator(op),
            None => unit_halstead.observe_operator(op),
        }
    }

    /// Observe an operand event into the deepest scope containing
    /// `[span_start, span_end)`, falling back to `unit_halstead`.
    pub fn observe_operand(
        &mut self,
        span_start: u32,
        span_end: u32,
        unit_halstead: &mut HalsteadBuilder,
        op: HalsteadOperand,
    ) {
        match self.deepest_enclosing_index(span_start, span_end) {
            Some(idx) => self.entries[idx].halstead.observe_operand(op),
            None => unit_halstead.observe_operand(op),
        }
    }

    /// Route a PLOC code-line observation to the deepest scope
    /// containing `[span_start, span_end)`, falling back to
    /// `unit_loc`. Lines are deduplicated per scope by the underlying
    /// `LocStats::observe_code_line` (set semantics).
    pub fn observe_code_line(
        &mut self,
        span_start: u32,
        span_end: u32,
        unit_loc: &mut LocStats,
        start_row: u32,
    ) {
        match self.deepest_enclosing_index(span_start, span_end) {
            Some(idx) => self.entries[idx]
                .loc_token_events
                .observe_code_line(start_row),
            None => unit_loc.observe_code_line(start_row),
        }
    }

    /// Route a comment observation to the deepest scope containing
    /// `[span_start, span_end)`, falling back to `unit_loc`.
    pub fn observe_comment(
        &mut self,
        span_start: u32,
        span_end: u32,
        unit_loc: &mut LocStats,
        start_row: u32,
        end_row: u32,
    ) {
        match self.deepest_enclosing_index(span_start, span_end) {
            Some(idx) => self.entries[idx]
                .loc_token_events
                .observe_comment(start_row, end_row),
            None => unit_loc.observe_comment(start_row, end_row),
        }
    }

    /// Propagate each entry's per-space Halstead counts up its parent
    /// chain (set-union for `n1`/`n2`, sum for `N1`/`N2`), merge them
    /// into `unit_halstead`, and overwrite the Halstead-derived keys
    /// (and the Halstead-dependent MI keys) for every matching space
    /// inside `tree`.
    ///
    /// The unit space's metrics are written by the caller via
    /// [`crate::apply_state_to`] using `unit_halstead` after this
    /// function returns; this overlay only touches recorded child
    /// spaces.
    pub fn finalize_into_tree(
        mut self,
        tree: &mut MetricSpace,
        unit_halstead: &mut HalsteadBuilder,
        unit_loc: &mut LocStats,
    ) {
        // Walk deepest-first so each parent has absorbed every
        // descendant by the time we touch it. `record_open` pushes in
        // source-prefix order, so iterating `entries` in reverse
        // visits children before parents.
        for i in (0..self.entries.len()).rev() {
            // Halstead — merge child's set-based counts into parent.
            let child_h = std::mem::take(&mut self.entries[i].halstead);
            // LOC token events — merge child's token-only LocStats
            // into parent's token-only LocStats so the parent's
            // overlay sees the file-wide rollup. Uses
            // `merge_token_observations` to avoid touching min/max
            // bounds (those were finalized at AST close time).
            let child_loc_token = std::mem::take(&mut self.entries[i].loc_token_events);
            match self.entries[i].parent {
                Some(p) => {
                    self.entries[p].halstead.merge(&child_h);
                    self.entries[p]
                        .loc_token_events
                        .merge_token_observations(&child_loc_token);
                }
                None => {
                    unit_halstead.merge(&child_h);
                    unit_loc.merge_token_observations(&child_loc_token);
                }
            }
            self.entries[i].halstead = child_h;
            self.entries[i].loc_token_events = child_loc_token;
        }

        // Build a `SpaceId -> overlay inputs` lookup so the recursive
        // overlay pass below is a simple `get`. Each entry's `loc` is
        // cloned and folded with `loc_token_events` into a final
        // per-space LocStats; the overlay writes the LOC headline
        // keys and recomputes MI from the combined value.
        let mut by_space: HashMap<SpaceId, OverlayInputs> = HashMap::new();
        for entry in &self.entries {
            let mut combined = entry.loc.clone();
            combined.merge_token_observations(&entry.loc_token_events);
            by_space.insert(
                entry.space_id,
                OverlayInputs {
                    halstead: entry.halstead.clone(),
                    loc: combined,
                    cyclomatic: entry.cyclomatic.clone(),
                },
            );
        }
        overlay(tree, &by_space);
    }
}

struct OverlayInputs {
    halstead: HalsteadBuilder,
    loc: LocStats,
    cyclomatic: CyclomaticStats,
}

fn overlay(space: &mut MetricSpace, by_space: &HashMap<SpaceId, OverlayInputs>) {
    if let Some(inputs) = by_space.get(&space.id) {
        let counts = inputs.halstead.counts();
        let token_halstead_observed = counts.big_n1 > 0 || counts.big_n2 > 0;
        // Pattern A walkers (Go, Ruby) record Halstead *during* the
        // AST walk via `current()`, so the per-space MetricSet already
        // has the correct Halstead keys — `apply_state_to` at close
        // wrote them, and the AST close path rolled them up via
        // `merge_child_into_parent`. The tracker's `halstead` for
        // those walkers is empty, so we must NOT overwrite the
        // already-correct keys with zeros.
        //
        // Pattern B walkers (Python, TypeScript, Rust, PHP) emit
        // Halstead in a post-AST token sweep into the tracker, so the
        // tracker's `halstead` is the source of truth and the overlay
        // is what makes per-space JSON entries non-zero.
        if token_halstead_observed {
            let halstead = HalsteadStats::from_counts(counts);
            publish_halstead(&halstead, &mut space.metrics);
            // MI re-computation depends on Halstead volume — only
            // recompute when Halstead actually changed; otherwise the
            // MI keys written by `apply_state_to` at AST close are
            // already correct.
            let mi = MiStats::compute(&inputs.loc, &inputs.cyclomatic, &halstead);
            space
                .metrics
                .insert(MetricKey::new(keys::MI_VS), mi.mi_visual_studio);
            space
                .metrics
                .insert(MetricKey::new(keys::MI_ORIGINAL), mi.mi_original);
            space
                .metrics
                .insert(MetricKey::new(keys::MI_SEI), mi.mi_sei);
        }
        write_loc_token_keys(&inputs.loc, &mut space.metrics);
    }
    for child in &mut space.spaces {
        overlay(child, by_space);
    }
}

/// Overwrite the LOC headline keys (`loc.ploc`, `loc.cloc`,
/// `loc.sloc`, `loc.lloc`, `loc.blank`, `loc`) on a `MetricSet` from
/// the combined `LocStats`. The min/max/avg keys are intentionally
/// not rewritten — those reflect AST-walk roll-ups across spaces and
/// were already published correctly by `apply_state_to` at close
/// time. This overlay corrects the *per-space* PLOC / CLOC counts
/// the post-AST token sweep contributed (PR #95
/// discussion_r3265962147).
fn write_loc_token_keys(stats: &LocStats, target: &mut MetricSet) {
    target.insert(MetricKey::new(keys::LOC_PLOC), stats.ploc() as i64);
    target.insert(MetricKey::new(keys::LOC_CLOC), stats.cloc() as i64);
    target.insert(MetricKey::new(keys::LOC_LLOC), stats.lloc() as i64);
    target.insert(MetricKey::new(keys::LOC_SLOC), stats.sloc() as i64);
    target.insert(MetricKey::new(keys::LOC_BLANK), stats.blank() as i64);
    target.insert(MetricKey::new(keys::LOC), stats.sloc() as i64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mehen_core::{SourceSpan, SpaceKind};
    use smol_str::SmolStr;

    fn op(kind: &str) -> HalsteadOperator {
        HalsteadOperator {
            kind: SmolStr::new(kind),
            text: None,
        }
    }
    fn opd(text: &str) -> HalsteadOperand {
        HalsteadOperand {
            kind: SmolStr::new("Operand"),
            text: Some(SmolStr::new(text)),
        }
    }

    fn span(start: u32, end: u32) -> SourceSpan {
        SourceSpan {
            start_byte: start,
            end_byte: end,
            start_line: 1,
            end_line: 1,
        }
    }

    #[test]
    fn route_picks_deepest_enclosing_entry() {
        let mut t = SpaceRangeTracker::new();
        t.record_open(SpaceId(1), 0, 100); // outer function
        t.record_open(SpaceId(2), 20, 80); // nested function
        let mut unit = HalsteadBuilder::new();

        // Inside both — deepest is SpaceId(2).
        t.observe_operator(50, 51, &mut unit, op("+"));
        // Inside SpaceId(1) only.
        t.observe_operator(5, 6, &mut unit, op("-"));
        // Outside everything — unit.
        t.observe_operator(200, 201, &mut unit, op("*"));
        // Inside SpaceId(1) only (after the inner range ends).
        t.observe_operand(85, 88, &mut unit, opd("foo"));

        let inner_n1 = t
            .entries
            .iter()
            .find(|e| e.space_id == SpaceId(2))
            .unwrap()
            .halstead
            .counts();
        assert_eq!(inner_n1.big_n1, 1, "deepest entry got the deepest token");

        let outer = t
            .entries
            .iter()
            .find(|e| e.space_id == SpaceId(1))
            .unwrap()
            .halstead
            .counts();
        assert_eq!(outer.big_n1, 1);
        assert_eq!(outer.big_n2, 1);
        assert_eq!(unit.counts().big_n1, 1);
    }

    #[test]
    fn finalize_propagates_counts_up_parent_chain_and_overlays_tree() {
        let mut t = SpaceRangeTracker::new();
        t.record_open(SpaceId(1), 0, 100);
        t.record_open(SpaceId(2), 20, 80);
        // Stash the MI inputs the close hook would supply.
        t.record_close(
            SpaceId(1),
            &LocStats::default(),
            &CyclomaticStats::default(),
        );
        t.record_close(
            SpaceId(2),
            &LocStats::default(),
            &CyclomaticStats::default(),
        );

        let mut unit = HalsteadBuilder::new();
        t.observe_operator(50, 51, &mut unit, op("+"));
        t.observe_operator(5, 6, &mut unit, op("-"));

        // Build a tree: unit > SpaceId(1) > SpaceId(2).
        let mut tree = MetricSpace::new(SpaceId(0), SpaceKind::Unit, span(0, 100));
        let mut outer = MetricSpace::new(SpaceId(1), SpaceKind::Function, span(0, 100));
        let inner = MetricSpace::new(SpaceId(2), SpaceKind::Function, span(20, 80));
        outer.spaces.push(inner);
        tree.spaces.push(outer);

        let mut unit_loc = LocStats::default();
        t.finalize_into_tree(&mut tree, &mut unit, &mut unit_loc);

        let inner_n1 = tree.spaces[0].spaces[0]
            .metrics
            .get(&MetricKey::new(format!("{}.N1", keys::HALSTEAD)))
            .unwrap()
            .as_f64();
        let outer_n1 = tree.spaces[0]
            .metrics
            .get(&MetricKey::new(format!("{}.N1", keys::HALSTEAD)))
            .unwrap()
            .as_f64();
        assert_eq!(inner_n1, 1.0, "inner observed only the +");
        assert_eq!(outer_n1, 2.0, "outer rolls up inner's + plus its own -");
        assert_eq!(unit.counts().big_n1, 2, "unit absorbs the rolled-up outer");
    }

    #[test]
    fn unit_only_token_does_not_touch_recorded_entries() {
        let mut t = SpaceRangeTracker::new();
        t.record_open(SpaceId(1), 0, 100);
        t.record_close(
            SpaceId(1),
            &LocStats::default(),
            &CyclomaticStats::default(),
        );

        let mut unit = HalsteadBuilder::new();
        t.observe_operator(500, 501, &mut unit, op("+"));

        let mut tree = MetricSpace::new(SpaceId(0), SpaceKind::Unit, span(0, 1000));
        let outer = MetricSpace::new(SpaceId(1), SpaceKind::Function, span(0, 100));
        tree.spaces.push(outer);
        let mut unit_loc = LocStats::default();
        t.finalize_into_tree(&mut tree, &mut unit, &mut unit_loc);

        // The outer space received no tokens, so the overlay must
        // leave its Halstead keys alone — Pattern A walkers (Go,
        // Ruby) record Halstead via `current()` during the AST walk
        // and rely on the overlay NOT clobbering those values with
        // tracker-derived zeros.
        let outer_n1 = tree.spaces[0]
            .metrics
            .get(&MetricKey::new(format!("{}.N1", keys::HALSTEAD)));
        assert!(
            outer_n1.is_none(),
            "overlay must skip Halstead keys for tracker entries with zero tokens, got {outer_n1:?}"
        );
        assert_eq!(unit.counts().big_n1, 1);
    }

    /// Regression: PLOC code-line observations route to the deepest
    /// enclosing scope. Without routing, every line ends up on the
    /// unit and the per-space `loc.ploc` reads as 0.
    #[test]
    fn loc_code_lines_route_to_deepest_enclosing_scope() {
        use crate::keys;
        let mut t = SpaceRangeTracker::new();
        t.record_open(SpaceId(1), 0, 100);
        t.record_open(SpaceId(2), 20, 80);
        t.record_close(
            SpaceId(1),
            &LocStats::default(),
            &CyclomaticStats::default(),
        );
        t.record_close(
            SpaceId(2),
            &LocStats::default(),
            &CyclomaticStats::default(),
        );

        let mut unit_h = HalsteadBuilder::new();
        let mut unit_loc = LocStats::default();
        // Line 5 — inside SpaceId(2) (the inner).
        t.observe_code_line(50, 51, &mut unit_loc, 5);
        t.observe_code_line(60, 61, &mut unit_loc, 6);
        // Line 9 — inside SpaceId(1) only (between 80 and 100).
        t.observe_code_line(85, 86, &mut unit_loc, 9);
        // Line 99 — outside both.
        t.observe_code_line(500, 501, &mut unit_loc, 99);

        let mut tree = MetricSpace::new(SpaceId(0), SpaceKind::Unit, span(0, 1000));
        let mut outer = MetricSpace::new(SpaceId(1), SpaceKind::Function, span(0, 100));
        let inner = MetricSpace::new(SpaceId(2), SpaceKind::Function, span(20, 80));
        outer.spaces.push(inner);
        tree.spaces.push(outer);

        t.finalize_into_tree(&mut tree, &mut unit_h, &mut unit_loc);

        let inner_ploc = tree.spaces[0].spaces[0]
            .metrics
            .get(&MetricKey::new(keys::LOC_PLOC))
            .unwrap()
            .as_f64();
        assert_eq!(inner_ploc, 2.0, "inner sees lines 5 and 6");

        let outer_ploc = tree.spaces[0]
            .metrics
            .get(&MetricKey::new(keys::LOC_PLOC))
            .unwrap()
            .as_f64();
        assert_eq!(
            outer_ploc, 3.0,
            "outer rolls up inner's two lines + own line 9"
        );
        assert_eq!(unit_loc.ploc(), 4, "unit absorbs all four lines");
    }
}
