use camino::Utf8PathBuf;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{Language, LineIndex};

/// One source artifact handed to a language analyzer.
///
/// `SourceFile` is owned. Holding it does not borrow from any parser arena
/// or buffer the analyzer might construct internally.
#[derive(Debug, Clone, Serialize)]
pub struct SourceFile {
    /// Repository-relative or filesystem path. Always forward-slash
    /// separated when serialized for snapshots and reports — see the
    /// path normalization rule in the rewrite plan §4.8.
    pub path: Utf8PathBuf,
    pub language: Language,
    pub text: String,
    /// Pre-computed byte→line index. Reconstructible from `text`, but the
    /// engine builds it once and reuses it for diagnostics, span->line
    /// translation, and line classification across all metrics. Skipped
    /// when serializing — see the custom `Deserialize` impl below, which
    /// rebuilds the index from `text` so deserialized `SourceFile`s have
    /// a populated `line_index` rather than the empty default.
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

#[derive(Deserialize)]
struct SourceFileWire {
    path: Utf8PathBuf,
    language: Language,
    text: String,
}

impl<'de> Deserialize<'de> for SourceFile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = SourceFileWire::deserialize(deserializer)?;
        Ok(SourceFile::new(wire.path, wire.language, wire.text))
    }
}
