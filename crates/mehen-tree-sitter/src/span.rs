// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use mehen_core::{LineIndex, SourceSpan, byte_offset_clamped};
use tree_sitter::Node;

/// Convert a tree-sitter `Node` into mehen's owned `SourceSpan`.
///
/// `LineIndex` is required because tree-sitter only directly exposes byte
/// offsets via `start_byte`/`end_byte`; the row helpers it provides are
/// 0-based, while mehen's `SourceSpan` is 1-based for parity with the
/// existing report shape.
pub fn node_span(node: &Node<'_>, line_index: &LineIndex) -> SourceSpan {
    let start_byte = byte_offset_clamped(node.start_byte());
    let end_byte = byte_offset_clamped(node.end_byte());
    SourceSpan {
        start_byte,
        end_byte,
        start_line: line_index.line_at(start_byte),
        end_line: line_index.line_at(end_byte.saturating_sub(1).max(start_byte)),
    }
}

/// Returns the source text covered by `node`.
///
/// Falls back to an empty `&str` if the node's byte range is out of bounds —
/// the same behavior tree-sitter gives via `utf8_text`. Out-of-bounds is
/// possible only when the source has been truncated after parsing, which
/// the analyzer crates do not do.
pub fn text_of<'src>(node: &Node<'_>, source: &'src [u8]) -> &'src str {
    let start = node.start_byte();
    let end = node.end_byte().min(source.len());
    if start >= end {
        return "";
    }
    core::str::from_utf8(&source[start..end]).unwrap_or("")
}
