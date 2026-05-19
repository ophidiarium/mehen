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
    /// LOC + cyclomatic snapshots taken at space-close time, used by
    /// [`SpaceRangeTracker::finalize_into_tree`] to recompute MI on the
    /// overlay. Default values mean the close hook never fired —
    /// overlay treats those spaces as having empty inputs (matching
    /// the existing zero-valued MI keys written by `apply_state_to`).
    loc: LocStats,
    cyclomatic: CyclomaticStats,
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
        });
    }

    /// Stash the LOC and cyclomatic snapshots needed to recompute MI
    /// after the token sweep. Call from the walker's `close_space`
    /// hook with the about-to-be-published state's values. Quietly
    /// no-ops when the `space_id` was not previously recorded via
    /// [`record_open`] — the unit scope is implicit.
    pub fn record_close(
        &mut self,
        space_id: SpaceId,
        loc: &LocStats,
        cyclomatic: &CyclomaticStats,
    ) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.space_id == space_id) {
            entry.loc = loc.clone();
            entry.cyclomatic = cyclomatic.clone();
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
    ) {
        // Walk deepest-first so each parent has absorbed every
        // descendant by the time we touch it. `record_open` pushes in
        // source-prefix order, so iterating `entries` in reverse
        // visits children before parents.
        for i in (0..self.entries.len()).rev() {
            let child = std::mem::take(&mut self.entries[i].halstead);
            match self.entries[i].parent {
                Some(p) => self.entries[p].halstead.merge(&child),
                None => unit_halstead.merge(&child),
            }
            self.entries[i].halstead = child;
        }

        // Build a `SpaceId -> (HalsteadBuilder, LocStats, CyclomaticStats)`
        // lookup so the recursive overlay pass below is a simple `get`.
        let by_space: HashMap<SpaceId, OverlayInputs<'_>> = self
            .entries
            .iter()
            .map(|e| {
                (
                    e.space_id,
                    OverlayInputs {
                        halstead: &e.halstead,
                        loc: &e.loc,
                        cyclomatic: &e.cyclomatic,
                    },
                )
            })
            .collect();

        overlay_halstead(tree, &by_space);
    }
}

struct OverlayInputs<'a> {
    halstead: &'a HalsteadBuilder,
    loc: &'a LocStats,
    cyclomatic: &'a CyclomaticStats,
}

fn overlay_halstead(space: &mut MetricSpace, by_space: &HashMap<SpaceId, OverlayInputs<'_>>) {
    if let Some(inputs) = by_space.get(&space.id) {
        let halstead = HalsteadStats::from_counts(inputs.halstead.counts());
        write_halstead_keys(&halstead, &mut space.metrics);
        let mi = MiStats::compute(inputs.loc, inputs.cyclomatic, &halstead);
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
    for child in &mut space.spaces {
        overlay_halstead(child, by_space);
    }
}

fn write_halstead_keys(stats: &HalsteadStats, target: &mut MetricSet) {
    target.insert(MetricKey::new(keys::HALSTEAD_VOLUME), stats.volume());
    target.insert(
        MetricKey::new(keys::HALSTEAD_DIFFICULTY),
        stats.difficulty(),
    );
    target.insert(MetricKey::new(keys::HALSTEAD_EFFORT), stats.effort());
    target.insert(
        MetricKey::new(keys::HALSTEAD_VOCABULARY),
        stats.vocabulary(),
    );
    target.insert(MetricKey::new(keys::HALSTEAD_LENGTH), stats.length());
    target.insert(
        MetricKey::new(format!("{}.n1", keys::HALSTEAD)),
        stats.u_operators as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.N1", keys::HALSTEAD)),
        stats.operators as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.n2", keys::HALSTEAD)),
        stats.u_operands as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.N2", keys::HALSTEAD)),
        stats.operands as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.length", keys::HALSTEAD)),
        stats.length(),
    );
    target.insert(
        MetricKey::new(format!("{}.estimated_program_length", keys::HALSTEAD)),
        stats.estimated_program_length(),
    );
    target.insert(
        MetricKey::new(format!("{}.purity_ratio", keys::HALSTEAD)),
        stats.purity_ratio(),
    );
    target.insert(
        MetricKey::new(format!("{}.vocabulary", keys::HALSTEAD)),
        stats.vocabulary(),
    );
    target.insert(
        MetricKey::new(format!("{}.level", keys::HALSTEAD)),
        stats.level(),
    );
    target.insert(
        MetricKey::new(format!("{}.time", keys::HALSTEAD)),
        stats.time(),
    );
    target.insert(
        MetricKey::new(format!("{}.bugs", keys::HALSTEAD)),
        stats.bugs(),
    );
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

        t.finalize_into_tree(&mut tree, &mut unit);

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
        t.finalize_into_tree(&mut tree, &mut unit);

        let outer_n1 = tree.spaces[0]
            .metrics
            .get(&MetricKey::new(format!("{}.N1", keys::HALSTEAD)))
            .unwrap()
            .as_f64();
        assert_eq!(outer_n1, 0.0);
        assert_eq!(unit.counts().big_n1, 1);
    }
}
