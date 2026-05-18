use std::path::Path;
use std::sync::Arc;
use tree_sitter::Language;

#[cfg(feature = "markdown")]
use crate::legacy::languages::Markdown;
use crate::legacy::languages::{C, Go, Kotlin, Php, Ruby};
use crate::legacy::macros::{
    get_language, mk_action, mk_code, mk_emacs_mode, mk_extensions, mk_lang, mk_langs,
};
use crate::legacy::parser::Parser;
use crate::legacy::preproc::PreprocResults;
use crate::legacy::spaces::{FuncSpace, metrics};
use crate::legacy::traits::{Callback, LanguageInfo, ParserTrait};

#[cfg(feature = "markdown")]
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
        C,
        "The `C` language",
        "c",
        CCode,
        CParser,
        tree_sitter_c,
        [c, h],
        ["c"]
    ),
    (
        Php,
        "The `PHP` language",
        "php",
        PhpCode,
        PhpParser,
        tree_sitter_php,
        [php, php3, php4, php5, php7, php8, phtml],
        ["php"]
    ),
    (
        Markdown,
        "The `Markdown` language (for documentation metrics; code metrics are not applicable).",
        "markdown",
        MarkdownCode,
        MarkdownParser,
        tree_sitter_markdown_text,
        [md, markdown, mdown, mkd, mkdn, mdx],
        ["markdown", "gfm", "mdx"]
    )
);

#[cfg(not(feature = "markdown"))]
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
        C,
        "The `C` language",
        "c",
        CCode,
        CParser,
        tree_sitter_c,
        [c, h],
        ["c"]
    ),
    (
        Php,
        "The `PHP` language",
        "php",
        PhpCode,
        PhpParser,
        tree_sitter_php,
        [php, php3, php4, php5, php7, php8, phtml],
        ["php"]
    )
);

pub(crate) mod fake {
    #[allow(dead_code)]
    pub(crate) fn get_true<'a>(_ext: &str, _mode: &str) -> Option<&'a str> {
        None
    }
}
