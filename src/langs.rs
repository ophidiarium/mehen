use std::path::Path;
use std::sync::Arc;
use tree_sitter::Language;

use crate::languages::{Go, Python, Rust, Tsx, Typescript};
use crate::macros::{
    get_language, mk_action, mk_code, mk_emacs_mode, mk_extensions, mk_lang, mk_langs,
};
use crate::ops::{Ops, operands_and_operators};
use crate::parser::Parser;
use crate::preproc::PreprocResults;
use crate::spaces::{FuncSpace, metrics};
use crate::traits::{Callback, LanguageInfo, ParserTrait};

mk_langs!(
    // 1) Name for enum
    // 2) Language description
    // 3) Display name
    // 4) Empty struct name to implement
    // 5) Parser name
    // 6) tree-sitter function to call to get a Language
    // 7) file extensions
    // 8) emacs modes
    (
        Rust,
        "The `Rust` language",
        "rust",
        RustCode,
        RustParser,
        tree_sitter_rust,
        [rs],
        ["rust"]
    ),
    (
        Python,
        "The `Python` language",
        "python",
        PythonCode,
        PythonParser,
        tree_sitter_python,
        [py],
        ["python"]
    ),
    (
        Tsx,
        "The `Tsx` language incorporates the `JSX` syntax inside `TypeScript`",
        "typescript",
        TsxCode,
        TsxParser,
        tree_sitter_tsx,
        [tsx],
        []
    ),
    (
        Typescript,
        "The `TypeScript` language",
        "typescript",
        TypescriptCode,
        TypescriptParser,
        tree_sitter_typescript,
        [ts, jsw, jsmw],
        ["typescript"]
    ),
    (
        Go,
        "The `Go` language",
        "go",
        GoCode,
        GoParser,
        tree_sitter_go,
        [go],
        ["go"]
    )
);

pub(crate) mod fake {
    pub(crate) fn get_true<'a>(_ext: &str, _mode: &str) -> Option<&'a str> {
        None
    }
}
