use std::path::Path;
use std::sync::Arc;
use tree_sitter::Language;

use crate::languages::{C, Go, Kotlin, Powershell, Python, Ruby, Rust, Tsx, Typescript};
use crate::macros::{
    get_language, mk_action, mk_code, mk_emacs_mode, mk_extensions, mk_lang, mk_langs,
};
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
        "The `Tsx` language incorporates the `JSX` syntax inside `TypeScript`. Also used for `JSX` files since `TypeScript` is a superset of `JavaScript`.",
        "typescript",
        TsxCode,
        TsxParser,
        tree_sitter_tsx,
        [tsx, jsx],
        []
    ),
    (
        Typescript,
        "The `TypeScript` language. Also used for `JavaScript` files since `TypeScript` is a superset of `JavaScript`.",
        "typescript",
        TypescriptCode,
        TypescriptParser,
        tree_sitter_typescript,
        [ts, mts, cts, js, mjs, cjs],
        ["typescript", "javascript", "js"]
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
    ),
    (
        Ruby,
        "The `Ruby` language",
        "ruby",
        RubyCode,
        RubyParser,
        tree_sitter_ruby,
        [rb],
        ["ruby"]
    ),
    (
        Kotlin,
        "The `Kotlin` language",
        "kotlin",
        KotlinCode,
        KotlinParser,
        tree_sitter_kotlin,
        [kt, kts],
        ["kotlin"]
    ),
    (
        Powershell,
        "The `PowerShell` language",
        "powershell",
        PowershellCode,
        PowershellParser,
        tree_sitter_pwsh,
        [ps1, psm1, psd1],
        ["powershell"]
    ),
    (
        C,
        "The `C` language",
        "c",
        CCode,
        CParser,
        tree_sitter_c,
        [c, h],
        ["c"]
    )
);

pub(crate) mod fake {
    pub(crate) fn get_true<'a>(_ext: &str, _mode: &str) -> Option<&'a str> {
        None
    }
}
