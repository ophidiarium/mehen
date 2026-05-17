use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{Language, LineIndex};

/// One source artifact handed to a language analyzer.
///
/// `SourceFile` is owned. Holding it does not borrow from any parser arena
/// or buffer the analyzer might construct internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    /// Repository-relative or filesystem path. Always forward-slash
    /// separated when serialized for snapshots and reports — see the
    /// path normalization rule in the rewrite plan §4.8.
    pub path: Utf8PathBuf,
    pub language: Language,
    pub text: String,
    /// Pre-computed byte→line index. Reconstructible from `text`, but the
    /// engine builds it once and reuses it for diagnostics, span->line
    /// translation, and line classification across all metrics.
    #[serde(skip)]
    pub line_index: LineIndex,
}

impl SourceFile {
    /// Build a source file by computing the line index from `text`.
    pub fn new(path: Utf8PathBuf, language: Language, text: String) -> Self {
        let line_index = LineIndex::new(&text);
        Self {
            path,
            language,
            text,
            line_index,
        }
    }
}
