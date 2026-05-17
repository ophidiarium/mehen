use core::fmt;

use tree_sitter::{Language, Parser, Tree};

/// Errors from setting up or driving a tree-sitter parser.
#[derive(Debug)]
pub enum TreeSitterError {
    SetLanguage(String),
    Parse,
}

impl fmt::Display for TreeSitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeSitterError::SetLanguage(s) => write!(f, "set_language failed: {s}"),
            TreeSitterError::Parse => write!(f, "tree-sitter returned no tree"),
        }
    }
}

impl core::error::Error for TreeSitterError {}

/// Owns a parsed tree and the source bytes it indexes into.
///
/// Owning the source here keeps tree-sitter's `Tree` self-consistent for the
/// life of this struct without forcing analyzer crates to manage two
/// parallel buffers.
pub struct TreeSitterParser {
    source: Vec<u8>,
    tree: Tree,
}

impl TreeSitterParser {
    pub fn new(language: Language, source: Vec<u8>) -> Result<Self, TreeSitterError> {
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| TreeSitterError::SetLanguage(e.to_string()))?;
        let tree = parser.parse(&source, None).ok_or(TreeSitterError::Parse)?;
        Ok(Self { source, tree })
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn source(&self) -> &[u8] {
        &self.source
    }

    pub fn root(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }
}
