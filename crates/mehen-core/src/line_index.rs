// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use serde::{Deserialize, Serialize};

/// Maps byte offsets to 1-based line numbers within a source file.
///
/// This exists in `mehen-core` rather than each analyzer crate because every
/// analyzer needs a single canonical byte/line mapping implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineIndex {
    /// Byte offsets at which each line starts. `line_starts[0]` is always 0.
    line_starts: Vec<u32>,
}

impl Default for LineIndex {
    fn default() -> Self {
        Self {
            line_starts: vec![0],
        }
    }
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = Vec::with_capacity(text.len() / 32 + 1);
        line_starts.push(0u32);
        let bytes = text.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                let next = (i + 1) as u32;
                line_starts.push(next);
            }
        }
        Self { line_starts }
    }

    /// Returns the 1-based line number containing `byte_offset`.
    pub fn line_at(&self, byte_offset: u32) -> u32 {
        // Binary search for the largest `line_starts[i] <= byte_offset`.
        match self.line_starts.binary_search(&byte_offset) {
            Ok(i) => (i + 1) as u32,
            Err(i) => i.max(1) as u32,
        }
    }

    /// Total line count (a final blank line is included).
    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// Returns `(start_byte, end_byte)` for a 1-based line number, exclusive
    /// of the trailing newline. Returns `None` for out-of-range lines.
    pub fn line_byte_range(&self, line: u32, total_len: u32) -> Option<(u32, u32)> {
        if line == 0 || (line as usize) > self.line_starts.len() {
            return None;
        }
        let idx = (line - 1) as usize;
        let start = self.line_starts[idx];
        let end = self
            .line_starts
            .get(idx + 1)
            .map(|next| next.saturating_sub(1))
            .unwrap_or(total_len);
        Some((start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_one_line() {
        let idx = LineIndex::new("");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.line_at(0), 1);
    }

    #[test]
    fn line_at_boundaries() {
        // bytes:    0 1 2 3 4 5 6 7 8 9
        // text:     a b \n c d \n e f \n
        let idx = LineIndex::new("ab\ncd\nef\n");
        assert_eq!(idx.line_at(0), 1);
        assert_eq!(idx.line_at(2), 1); // '\n' on line 1
        assert_eq!(idx.line_at(3), 2);
        assert_eq!(idx.line_at(5), 2);
        assert_eq!(idx.line_at(6), 3);
    }

    #[test]
    fn byte_range_for_line() {
        let text = "ab\ncd\nef";
        let idx = LineIndex::new(text);
        assert_eq!(idx.line_byte_range(1, text.len() as u32), Some((0, 2)));
        assert_eq!(idx.line_byte_range(2, text.len() as u32), Some((3, 5)));
        assert_eq!(idx.line_byte_range(3, text.len() as u32), Some((6, 8)));
    }
}
