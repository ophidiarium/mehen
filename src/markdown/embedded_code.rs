//! §9.4 embedded-code adjustment.
//!
//! For every supported fenced code block — `rust`, `ts`/`tsx`, `py`, `go`,
//! `rb`, `c`, `kotlin`, `pwsh`/`powershell` — run the fence body through the
//! existing `spaces::metrics()` pipeline and accumulate:
//!
//! ```text
//! embedded_volume = Σ 0.20 * sqrt(volume_c)
//!                 + 0.50 * cognitive_c
//!                 + 0.10 * loc_c
//! ```
//!
//! Code volume is square-rooted because raw embedded Halstead volume can
//! dwarf Markdown-level signals. Cognitive complexity remains linear — a
//! cognitively complex example genuinely warrants review.

use std::path::Path;

use crate::langs::LANG;
use crate::languages::Markdown;
use crate::node::Node;

/// Public entry: walk the AST, find every fenced code block whose info
/// string maps to a supported `LANG`, and sum the §9.4 contributions.
pub(crate) fn embedded_volume(root: &Node<'_>, source: &str) -> f64 {
    let mut total = 0.0;
    visit(root, source, &mut total);
    total
}

fn visit(node: &Node<'_>, source: &str, total: &mut f64) {
    let kind: Markdown = node.kind_id().into();
    if matches!(kind, Markdown::FencedCodeBlock) {
        let info = fence_info_tag(node, source);
        let lang = info.as_deref().and_then(map_fence_to_lang);
        if let (Some(lang), Some(body)) = (lang, fenced_code_content(node, source)) {
            *total += analyze_fence(lang, &body);
        }
        // Do not descend — we've handled the fence.
        return;
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            visit(&cursor.node(), source, total);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn analyze_fence(lang: LANG, body: &str) -> f64 {
    let bytes = body.as_bytes().to_vec();
    let path: std::path::PathBuf = synthetic_path(lang);
    // `get_function_spaces` is `pub(crate)` and returns an aggregated
    // `FuncSpace` with `metrics.halstead`, `.cognitive`, `.loc` filled in.
    let Some(space) = crate::langs::get_function_spaces(&lang, bytes, Path::new(&path), None)
    else {
        return 0.0;
    };
    let volume = space.metrics.halstead.volume();
    let cognitive = space.metrics.cognitive.cognitive_sum();
    let loc = space.metrics.loc.sloc();

    let v = if volume.is_finite() && volume > 0.0 {
        0.20 * volume.sqrt()
    } else {
        0.0
    };
    let c = if cognitive.is_finite() {
        0.50 * cognitive
    } else {
        0.0
    };
    let l = if loc.is_finite() { 0.10 * loc } else { 0.0 };
    v + c + l
}

fn synthetic_path(lang: LANG) -> std::path::PathBuf {
    // The filename drives language detection for error messages only — the
    // `LANG` parameter is what `get_function_spaces` actually dispatches on.
    let name = match lang {
        LANG::Rust => "fence.rs",
        LANG::Python => "fence.py",
        LANG::Typescript => "fence.ts",
        LANG::Tsx => "fence.tsx",
        LANG::Go => "fence.go",
        LANG::Ruby => "fence.rb",
        LANG::Kotlin => "fence.kt",
        LANG::Powershell => "fence.ps1",
        LANG::C => "fence.c",
        #[cfg(feature = "markdown")]
        LANG::Markdown => "fence.md",
    };
    std::path::PathBuf::from(name)
}

fn map_fence_to_lang(info: &str) -> Option<LANG> {
    // Extract the first token of the info string — e.g. `rust,ignore` → `rust`.
    let head = info
        .split([' ', '\t', ','])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    Some(match head.as_str() {
        "rust" | "rs" => LANG::Rust,
        "python" | "py" => LANG::Python,
        "typescript" | "ts" => LANG::Typescript,
        "tsx" | "jsx" => LANG::Tsx,
        "javascript" | "js" => LANG::Typescript,
        "go" => LANG::Go,
        "ruby" | "rb" => LANG::Ruby,
        "kotlin" | "kt" | "kts" => LANG::Kotlin,
        "powershell" | "pwsh" | "ps1" => LANG::Powershell,
        "c" => LANG::C,
        _ => return None,
    })
}

fn fence_info_tag(node: &Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        if matches!(child.kind_id().into(), Markdown::InfoString) {
            let mut c2 = child.cursor();
            if c2.goto_first_child() {
                loop {
                    let inner = c2.node();
                    if matches!(inner.kind_id().into(), Markdown::Language) {
                        let bytes = source.as_bytes();
                        let start = inner.start_byte();
                        let end = inner.end_byte();
                        if end <= bytes.len() && start < end {
                            let tag = std::str::from_utf8(&bytes[start..end])
                                .ok()?
                                .trim()
                                .to_string();
                            if !tag.is_empty() {
                                return Some(tag);
                            }
                        }
                    }
                    if !c2.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    None
}

fn fenced_code_content(node: &Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        if matches!(child.kind_id().into(), Markdown::CodeFenceContent) {
            let bytes = source.as_bytes();
            let start = child.start_byte();
            let end = child.end_byte();
            if end <= bytes.len() && start <= end {
                return std::str::from_utf8(&bytes[start..end])
                    .ok()
                    .map(str::to_string);
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_markdown_text::LANGUAGE.into())
            .unwrap();
        parser.parse(src, None).unwrap()
    }

    #[test]
    fn no_fences_returns_zero() {
        let src = "# Hi\n\nno code.\n";
        let tree = parse(src);
        let root = crate::node::Node(tree.root_node());
        assert_eq!(embedded_volume(&root, src), 0.0);
    }

    #[test]
    fn unsupported_language_returns_zero() {
        let src = "```sql\nSELECT 1;\n```\n";
        let tree = parse(src);
        let root = crate::node::Node(tree.root_node());
        assert_eq!(embedded_volume(&root, src), 0.0);
    }

    #[test]
    fn rust_fence_produces_positive_volume() {
        let src = "```rust\nfn main() { let x = 1 + 2; }\n```\n";
        let tree = parse(src);
        let root = crate::node::Node(tree.root_node());
        let v = embedded_volume(&root, src);
        assert!(v > 0.0, "expected positive embedded volume, got {v}");
    }

    #[test]
    fn large_code_fence_scales_with_sqrt() {
        // A 100-LOC fence must produce embedded_volume that scales roughly
        // with sqrt(internal volume) — demonstrate via monotonicity.
        let small = "```rust\nfn a() { 1 + 1; }\n```\n".to_string();
        let mut big_body = String::from("```rust\n");
        for _ in 0..50 {
            big_body.push_str("fn a() { let x = 1 + 1; let y = x + 2; }\n");
        }
        big_body.push_str("```\n");

        let t1 = parse(&small);
        let r1 = crate::node::Node(t1.root_node());
        let v1 = embedded_volume(&r1, &small);

        let t2 = parse(&big_body);
        let r2 = crate::node::Node(t2.root_node());
        let v2 = embedded_volume(&r2, &big_body);

        // 50× lines ⇒ ~50× internal volume ⇒ ~sqrt(50) ≈ 7× in the sqrt
        // term alone. Plus cognitive and loc scale linearly, so we expect
        // strict monotonic growth but not blow-up.
        assert!(
            v2 > v1,
            "expected larger fence to produce more volume: v1={v1}, v2={v2}"
        );
        // Guard against the scale actually being multiplicative by hundreds.
        // With 50× LOC, the cognitive + loc term alone can be a large
        // multiple, so we use a generous ceiling but still something finite.
        assert!(v2 < 200.0 * v1 + 1000.0);
    }
}
