use tree_sitter::Node as OtherNode;
use tree_sitter::Tree as OtherTree;
use tree_sitter::{Parser, TreeCursor};

use crate::legacy::checker::Checker;
use crate::legacy::traits::{LanguageInfo, Search};

#[derive(Clone, Debug)]
pub struct Tree(OtherTree);

impl Tree {
    pub fn new<T: LanguageInfo>(code: &[u8]) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&T::get_lang().get_ts_language())
            .unwrap();

        Self(parser.parse(code, None).unwrap())
    }

    pub fn get_root(&self) -> Node<'_> {
        Node(self.0.root_node())
    }
}

/// An `AST` node.
///
/// The inner `tree_sitter::Node` is exposed for advanced use cases
/// where direct access to the underlying tree-sitter API is needed.
#[derive(Clone, Copy, Debug)]
pub struct Node<'a>(pub OtherNode<'a>);

impl<'a> Node<'a> {
    /// Checks if a node represents a syntax error or contains any syntax errors
    /// anywhere within it.
    pub fn has_error(&self) -> bool {
        self.0.has_error()
    }

    pub fn id(&self) -> usize {
        self.0.id()
    }

    pub fn kind(&self) -> &'static str {
        self.0.kind()
    }

    pub fn kind_id(&self) -> u16 {
        self.0.kind_id()
    }

    pub fn start_byte(&self) -> usize {
        self.0.start_byte()
    }

    pub fn end_byte(&self) -> usize {
        self.0.end_byte()
    }

    pub fn start_position(&self) -> (usize, usize) {
        let temp = self.0.start_position();
        (temp.row, temp.column)
    }

    pub fn end_position(&self) -> (usize, usize) {
        let temp = self.0.end_position();
        (temp.row, temp.column)
    }

    pub fn start_row(&self) -> usize {
        self.0.start_position().row
    }

    pub fn end_row(&self) -> usize {
        self.0.end_position().row
    }

    pub fn parent(&self) -> Option<Node<'a>> {
        self.0.parent().map(Node)
    }

    #[inline(always)]
    pub fn has_sibling(&self, id: u16) -> bool {
        self.0.parent().is_some_and(|parent| {
            self.0
                .children(&mut parent.walk())
                .any(|child| child.kind_id() == id)
        })
    }

    #[inline(always)]
    pub fn is_child(&self, id: u16) -> bool {
        self.0
            .children(&mut self.0.walk())
            .any(|child| child.kind_id() == id)
    }

    pub fn child_count(&self) -> usize {
        self.0.child_count()
    }

    pub fn child_by_field_name(&self, name: &str) -> Option<Node<'_>> {
        self.0.child_by_field_name(name).map(Node)
    }

    pub fn child(&self, pos: usize) -> Option<Node<'a>> {
        self.0.child(pos as u32).map(Node)
    }

    pub fn children(&self) -> impl ExactSizeIterator<Item = Node<'a>> + use<'a> {
        let mut cursor = self.cursor();
        cursor.goto_first_child();
        (0..self.child_count()).map(move |_| {
            let result = cursor.node();
            cursor.goto_next_sibling();
            result
        })
    }

    pub fn cursor(&self) -> Cursor<'a> {
        Cursor(self.0.walk())
    }

    pub fn count_specific_ancestors<T: crate::legacy::traits::ParserTrait>(
        &self,
        check: fn(&Node) -> bool,
        stop: fn(&Node) -> bool,
    ) -> usize {
        let mut count = 0;
        let mut node = *self;
        while let Some(parent) = node.parent() {
            if stop(&parent) {
                break;
            }
            if check(&parent) && !T::Checker::is_else_if(&parent) {
                count += 1;
            }
            node = parent;
        }
        count
    }

    pub fn has_ancestors(&self, typ: fn(&Node) -> bool, typs: fn(&Node) -> bool) -> bool {
        let mut res = false;
        let mut node = *self;
        if let Some(parent) = node.parent()
            && typ(&parent)
        {
            node = parent;
        }
        if let Some(parent) = node.parent()
            && typs(&parent)
        {
            res = true;
        }
        res
    }
}

/// An `AST` cursor.
#[derive(Clone)]
pub struct Cursor<'a>(TreeCursor<'a>);

impl<'a> Cursor<'a> {
    pub fn reset(&mut self, node: &Node<'a>) {
        self.0.reset(node.0);
    }

    pub fn goto_next_sibling(&mut self) -> bool {
        self.0.goto_next_sibling()
    }

    pub fn goto_first_child(&mut self) -> bool {
        self.0.goto_first_child()
    }

    pub fn node(&self) -> Node<'a> {
        Node(self.0.node())
    }
}

impl<'a> Search<'a> for Node<'a> {
    fn act_on_node(&self, action: &mut dyn FnMut(&Node<'a>)) {
        let mut cursor = self.cursor();
        let mut stack = Vec::new();
        let mut children = Vec::new();

        stack.push(*self);

        while let Some(node) = stack.pop() {
            action(&node);
            cursor.reset(&node);
            if cursor.goto_first_child() {
                loop {
                    children.push(cursor.node());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                for child in children.drain(..).rev() {
                    stack.push(child);
                }
            }
        }
    }

    fn act_on_child(&self, action: &mut dyn FnMut(&Node<'a>)) {
        for child in self.children() {
            action(&child);
        }
    }
}
