//! `Node`/`Cursor` wrapper used internally by the Markdown analyzer.
//!
//! Ported from the pre-1.0 `src/node.rs` with the macro-generated
//! `LanguageInfo` / `Search` trait dependencies stripped — the Markdown
//! analyzer only uses the inherent methods. Plan §8.3 says the global
//! wrapper goes away in the long run; during the v1 transition it
//! lives here as a crate-private helper so the moved analyzer code is
//! byte-for-byte compatible with its pre-1.0 callers.

use tree_sitter::Node as OtherNode;
use tree_sitter::Tree as OtherTree;
use tree_sitter::{Parser, TreeCursor};

#[derive(Clone, Debug)]
pub(crate) struct Tree(OtherTree);

impl Tree {
    pub(crate) fn parse(language: &tree_sitter::Language, code: &[u8]) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .expect("set_language must succeed");
        Self(parser.parse(code, None).expect("parse must succeed"))
    }

    pub(crate) fn get_root(&self) -> Node<'_> {
        Node(self.0.root_node())
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Node<'a>(pub(crate) OtherNode<'a>);

impl<'a> Node<'a> {
    pub(crate) fn has_error(&self) -> bool {
        self.0.has_error()
    }
    pub(crate) fn id(&self) -> usize {
        self.0.id()
    }
    pub(crate) fn kind(&self) -> &'static str {
        self.0.kind()
    }
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
    pub(crate) fn end_row(&self) -> usize {
        self.0.end_position().row
    }
    pub(crate) fn parent(&self) -> Option<Node<'a>> {
        self.0.parent().map(Node)
    }

    #[inline(always)]
    pub(crate) fn has_sibling(&self, id: u16) -> bool {
        self.0.parent().is_some_and(|parent| {
            self.0
                .children(&mut parent.walk())
                .any(|child| child.kind_id() == id)
        })
    }

    #[inline(always)]
    pub(crate) fn is_child(&self, id: u16) -> bool {
        self.0
            .children(&mut self.0.walk())
            .any(|child| child.kind_id() == id)
    }

    pub(crate) fn child_count(&self) -> usize {
        self.0.child_count()
    }

    pub(crate) fn child_by_field_name(&self, name: &str) -> Option<Node<'_>> {
        self.0.child_by_field_name(name).map(Node)
    }

    pub(crate) fn child(&self, pos: usize) -> Option<Node<'a>> {
        self.0.child(pos as u32).map(Node)
    }

    pub(crate) fn children(&self) -> impl ExactSizeIterator<Item = Node<'a>> + use<'a> {
        let mut cursor = self.cursor();
        cursor.goto_first_child();
        (0..self.child_count()).map(move |_| {
            let result = cursor.node();
            cursor.goto_next_sibling();
            result
        })
    }

    pub(crate) fn cursor(&self) -> Cursor<'a> {
        Cursor(self.0.walk())
    }
}

#[derive(Clone)]
pub(crate) struct Cursor<'a>(TreeCursor<'a>);

impl<'a> Cursor<'a> {
    pub(crate) fn reset(&mut self, node: &Node<'a>) {
        self.0.reset(node.0);
    }
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
