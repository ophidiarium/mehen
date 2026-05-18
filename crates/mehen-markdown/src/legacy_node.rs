//! `Node`/`Cursor` wrapper used internally by the Markdown analyzer.
//!
//! Ported from the pre-1.0 `src/node.rs` with the macro-generated
//! `LanguageInfo` / `Search` trait dependencies stripped — the Markdown
//! analyzer only uses the inherent methods. Plan §8.3 says the global
//! wrapper goes away in the long run; during the v1 transition it
//! lives here as a crate-private helper so the moved analyzer code is
//! byte-for-byte compatible with its pre-1.0 callers.

use tree_sitter::Node as OtherNode;
use tree_sitter::TreeCursor;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Node<'a>(pub(crate) OtherNode<'a>);

impl<'a> Node<'a> {
    pub(crate) fn kind_id(&self) -> u16 {
        self.0.kind_id()
    }
    pub(crate) fn start_byte(&self) -> usize {
        self.0.start_byte()
    }
    pub(crate) fn end_byte(&self) -> usize {
        self.0.end_byte()
    }
    pub(crate) fn start_position(&self) -> (usize, usize) {
        let p = self.0.start_position();
        (p.row, p.column)
    }
    pub(crate) fn end_position(&self) -> (usize, usize) {
        let p = self.0.end_position();
        (p.row, p.column)
    }
    pub(crate) fn start_row(&self) -> usize {
        self.0.start_position().row
    }

    pub(crate) fn child_by_field_name(&self, name: &str) -> Option<Node<'_>> {
        self.0.child_by_field_name(name).map(Node)
    }

    pub(crate) fn cursor(&self) -> Cursor<'a> {
        Cursor(self.0.walk())
    }
}

#[derive(Clone)]
pub(crate) struct Cursor<'a>(TreeCursor<'a>);

impl<'a> Cursor<'a> {
    pub(crate) fn goto_next_sibling(&mut self) -> bool {
        self.0.goto_next_sibling()
    }
    pub(crate) fn goto_first_child(&mut self) -> bool {
        self.0.goto_first_child()
    }
    pub(crate) fn node(&self) -> Node<'a> {
        Node(self.0.node())
    }
}
