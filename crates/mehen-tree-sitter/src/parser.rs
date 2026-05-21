// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use core::fmt;

use mehen_core::ParseDiagnostic;
use tree_sitter::{Language, Node, Parser, Tree};

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

/// Walk `root` and emit one `ParseDiagnostic::error` per recovered
/// `ERROR` / missing node, capped at `max_diagnostics`. Tree-sitter
/// always returns a tree (even on syntax errors), so per the diagnostic
/// contract (plan §9.3) callers must surface these as `error` to make
/// `mehen metrics` exit 1 and `analyze_diff` record them under
/// `analysis_errors`. Returns an empty `Vec` for clean parses.
///
/// `code` is the language-namespaced diagnostic code, e.g.
/// `"go.syntax_error"`. `max_diagnostics` bounds the noise on heavily
/// corrupted input.
pub fn collect_recovered_errors(
    root: Node<'_>,
    code: &str,
    max_diagnostics: usize,
) -> Vec<ParseDiagnostic> {
    let mut out = Vec::new();
    if !root.has_error() {
        return out;
    }
    let mut cursor = root.walk();
    walk_for_errors(&mut cursor, code, max_diagnostics, &mut out);
    out
}

fn walk_for_errors(
    cursor: &mut tree_sitter::TreeCursor<'_>,
    code: &str,
    max: usize,
    out: &mut Vec<ParseDiagnostic>,
) {
    if out.len() >= max {
        return;
    }
    let node = cursor.node();
    if node.is_error() || node.is_missing() {
        let kind = if node.is_missing() {
            "missing"
        } else {
            "error"
        };
        let line = node.start_position().row + 1;
        out.push(ParseDiagnostic::error(
            code.to_string(),
            format!("tree-sitter {kind} node at line {line}"),
        ));
        if out.len() >= max {
            return;
        }
    }
    if cursor.goto_first_child() {
        loop {
            walk_for_errors(cursor, code, max, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}
