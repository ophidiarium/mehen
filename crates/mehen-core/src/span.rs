use serde::{Deserialize, Serialize};

/// Byte- and line-resolved location inside a source file.
///
/// Both byte and line are kept on the struct so consumers don't need to
/// re-derive one from the other. Producers (analyzers) populate both during
/// the parse walk; the `LineIndex` makes byte→line conversion cheap.
///
/// Byte offsets are stored as `u32`. mehen does not analyze sources larger
/// than `u32::MAX` bytes (~4 GiB); use [`byte_offset_clamped`] or
/// [`byte_offset_checked`] when converting from `usize` to surface or
/// silence the limit explicitly.
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

/// Convert a `usize` byte offset into the `u32` shape used by [`SourceSpan`],
/// clamping to `u32::MAX` for sources that would otherwise overflow.
///
/// Use this when the caller is fine with a clamp (the only real-world case
/// is "we don't analyze sources larger than 4 GiB; the upper edge is fine").
pub fn byte_offset_clamped(offset: usize) -> u32 {
    u32::try_from(offset).unwrap_or(u32::MAX)
}

/// Same as [`byte_offset_clamped`] but returns `None` on overflow so the
/// caller can decline to produce a span at all.
pub fn byte_offset_checked(offset: usize) -> Option<u32> {
    u32::try_from(offset).ok()
}
