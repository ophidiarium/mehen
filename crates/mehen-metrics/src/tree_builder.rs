use mehen_core::{MetricSet, MetricSpace, SourceSpan, SpaceId, SpaceKind};

/// Helper that assembles a `MetricSpace` tree with stable, monotonically
/// increasing `SpaceId`s.
///
/// Per the rewrite plan §4.3, this is the kind of plumbing that belongs in
/// `mehen-metrics` so language analyzer crates do not each re-implement
/// id-allocation and parent-child wiring. The crate ships the builder; the
/// language crate decides what spaces to emit.
pub struct MetricTreeBuilder {
    next_id: u32,
    stack: Vec<MetricSpace>,
}

impl MetricTreeBuilder {
    /// Begin a new tree with a `Unit` space at the root.
    pub fn new(unit_span: SourceSpan) -> Self {
        let mut stack = Vec::with_capacity(8);
        stack.push(MetricSpace::new(SpaceId(0), SpaceKind::Unit, unit_span));
        Self { next_id: 1, stack }
    }

    /// Open a child space, becoming the new innermost space.
    pub fn open(&mut self, kind: SpaceKind, span: SourceSpan, name: Option<String>) -> SpaceId {
        let id = SpaceId(self.next_id);
        self.next_id += 1;
        let mut space = MetricSpace::new(id, kind, span);
        space.name = name;
        self.stack.push(space);
        id
    }

    /// Close the innermost space and attach it to its parent.
    ///
    /// Panics if there is no innermost space — calls must balance with
    /// `open`. The Phase 1 implementation is intentionally strict about
    /// this so a regression in the analyzer's tree-walk is loud.
    pub fn close(&mut self) {
        let child = self
            .stack
            .pop()
            .expect("MetricTreeBuilder: no space to close");
        let parent = self
            .stack
            .last_mut()
            .expect("MetricTreeBuilder: cannot close the root unit");
        parent.spaces.push(child);
    }

    /// Mutable access to the innermost space's metric set.
    pub fn metrics_mut(&mut self) -> &mut MetricSet {
        &mut self
            .stack
            .last_mut()
            .expect("MetricTreeBuilder: stack is empty")
            .metrics
    }

    /// `SpaceId` of the innermost open space, or `None` when only the
    /// unit scope is on the stack. Walkers reach for this in their
    /// `close_space` hook to associate the about-to-close state with
    /// the space they're publishing into (e.g. for the
    /// [`crate::SpaceRangeTracker`] post-AST Halstead overlay).
    pub fn current_id(&self) -> Option<SpaceId> {
        // The unit space is at index 0 — anything above it is a real
        // child scope.
        if self.stack.len() <= 1 {
            None
        } else {
            self.stack.last().map(|s| s.id)
        }
    }

    /// Drop the unit-level outer scope and yield the assembled tree.
    ///
    /// Panics if the open/close calls are unbalanced. Failing fast surfaces
    /// analyzer-walker bugs (a missing `close()` after a scope-opening node)
    /// instead of silently emitting a tree with collapsed spaces.
    pub fn finish(mut self) -> MetricSpace {
        assert_eq!(
            self.stack.len(),
            1,
            "MetricTreeBuilder: unbalanced open/close calls (stack depth = {})",
            self.stack.len()
        );
        self.stack
            .pop()
            .expect("MetricTreeBuilder: empty after open")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_span() -> SourceSpan {
        SourceSpan::empty()
    }

    #[test]
    fn assigns_monotonic_ids() {
        let mut b = MetricTreeBuilder::new(empty_span());
        let f1 = b.open(SpaceKind::Function, empty_span(), Some("f".into()));
        b.close();
        let f2 = b.open(SpaceKind::Function, empty_span(), Some("g".into()));
        b.close();
        let root = b.finish();
        assert_eq!(root.id, SpaceId(0));
        assert_eq!(root.spaces.len(), 2);
        assert_eq!(f1, SpaceId(1));
        assert_eq!(f2, SpaceId(2));
    }

    #[test]
    fn nested_scopes_attach_correctly() {
        let mut b = MetricTreeBuilder::new(empty_span());
        b.open(SpaceKind::Class, empty_span(), Some("C".into()));
        b.open(SpaceKind::Function, empty_span(), Some("m".into()));
        b.close();
        b.close();
        let root = b.finish();
        assert_eq!(root.spaces.len(), 1);
        assert_eq!(root.spaces[0].kind, SpaceKind::Class);
        assert_eq!(root.spaces[0].spaces.len(), 1);
        assert_eq!(root.spaces[0].spaces[0].kind, SpaceKind::Function);
    }
}
