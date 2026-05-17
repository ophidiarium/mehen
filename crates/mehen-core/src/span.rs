use serde::{Deserialize, Serialize};

/// Byte- and line-resolved location inside a source file.
///
/// Both byte and line are kept on the struct so consumers don't need to
/// re-derive one from the other. Producers (analyzers) populate both during
/// the parse walk; the `LineIndex` makes byte→line conversion cheap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_line: u32,
    pub end_line: u32,
}

impl SourceSpan {
    pub fn new(start_byte: u32, end_byte: u32, start_line: u32, end_line: u32) -> Self {
        Self {
            start_byte,
            end_byte,
            start_line,
            end_line,
        }
    }

    pub fn empty() -> Self {
        Self::new(0, 0, 1, 1)
    }
}
