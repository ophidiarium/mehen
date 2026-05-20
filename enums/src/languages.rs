use tree_sitter::Language;

mk_langs!(
    // 1) Name for enum
    // 2) tree-sitter function to call to get a Language
    (Rust, tree_sitter_rust),
    (Python, tree_sitter_python),
    (Tsx, tree_sitter_tsx),
    (Typescript, tree_sitter_typescript),
    (Kotlin, tree_sitter_kotlin),
    (Powershell, tree_sitter_pwsh),
    (Php, tree_sitter_php),
    (Markdown, tree_sitter_markdown_text)
);
