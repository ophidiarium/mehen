// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! §9.4 embedded-code adjustment.
//!
//! For every supported fenced code block — `rust`, `ts`/`tsx`, `py`, `go`,
//! `rb`, `c`, `kotlin`, `pwsh`/`powershell` — run the fence body through
//! the source-language analysis pipeline and accumulate:
//!
//! ```text
//! embedded_volume = Σ 0.20 * sqrt(volume_c)
//!                 + 0.50 * cognitive_c
//!                 + 0.10 * loc_c
//! ```
//!
//! The dispatch is decoupled from this crate via [`set_legacy_dispatch`]:
//! the markdown crate doesn't depend on the per-language analyzers
//! directly. The legacy `mehen` library (during the v1 transition) and
//! the `mehen-engine` registry (post-transition, plan §4.7
//! `LanguageDispatcher` seam) both supply a callback that maps a
//! fence-language code + body to numeric volume/cognitive/sloc.

use std::sync::OnceLock;

use crate::grammar::Markdown;
use crate::legacy_node::Node;

/// Languages a fenced code block can declare. Mirrors the pre-1.0
/// `LANG` enum, but kept local to this crate so we don't depend on
/// `mehen::langs` at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceLanguage {
    Rust,
    Python,
    Typescript,
    Tsx,
    Go,
    Ruby,
    Kotlin,
    Powershell,
    C,
    Php,
}

/// Metrics extracted from one fenced code block. Returned by the
/// dispatch callback registered through [`set_legacy_dispatch`].
#[derive(Clone, Copy, Debug, Default)]
pub struct EmbeddedFenceMetrics {
    pub volume: f64,
    pub cognitive_sum: f64,
    pub sloc: f64,
}

type DispatchFn = fn(FenceLanguage, String) -> Option<EmbeddedFenceMetrics>;

static DISPATCH: OnceLock<DispatchFn> = OnceLock::new();

/// Register the embedded-code dispatch callback.
///
/// Called by `mehen::init_markdown` (the legacy library) at startup so
/// the physically-moved markdown analyzer can still drive
/// `langs::get_function_spaces` for fence bodies. The post-transition
/// `mehen-engine` will register a `LanguageDispatcher`-backed callback
/// here instead.
pub fn set_legacy_dispatch(f: DispatchFn) {
    let _ = DISPATCH.set(f);
}

/// Public entry: walk the AST, find every fenced code block whose info
/// string maps to a supported [`FenceLanguage`], and sum the §9.4
/// contributions.
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
        if let (Some(lang), Some(mut body)) = (lang, fenced_code_content(node, source)) {
            if matches!(lang, FenceLanguage::Php) {
                let leading = body.trim_start();
                if !leading.starts_with("<?php") && !leading.starts_with("<?=") {
                    body.insert_str(0, "<?php\n");
                }
            }
            *total += analyze_fence(lang, body);
        }
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

fn analyze_fence(lang: FenceLanguage, body: String) -> f64 {
    let Some(dispatch) = DISPATCH.get() else {
        return 0.0;
    };
    let Some(m) = dispatch(lang, body) else {
        return 0.0;
    };
    let v = if m.volume.is_finite() && m.volume > 0.0 {
        0.20 * m.volume.sqrt()
    } else {
        0.0
    };
    let c = if m.cognitive_sum.is_finite() {
        0.50 * m.cognitive_sum
    } else {
        0.0
    };
    let l = if m.sloc.is_finite() {
        0.10 * m.sloc
    } else {
        0.0
    };
    v + c + l
}

fn map_fence_to_lang(info: &str) -> Option<FenceLanguage> {
    let head = info
        .split([' ', '\t', ','])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    Some(match head.as_str() {
        "rust" | "rs" => FenceLanguage::Rust,
        "python" | "py" => FenceLanguage::Python,
        "typescript" | "ts" => FenceLanguage::Typescript,
        "tsx" | "jsx" => FenceLanguage::Tsx,
        "javascript" | "js" => FenceLanguage::Typescript,
        "go" => FenceLanguage::Go,
        "ruby" | "rb" => FenceLanguage::Ruby,
        "kotlin" | "kt" | "kts" => FenceLanguage::Kotlin,
        "powershell" | "pwsh" | "ps1" => FenceLanguage::Powershell,
        "c" => FenceLanguage::C,
        "php" => FenceLanguage::Php,
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
            // Fallback: take the entire info string text and split by ws.
            let bytes = source.as_bytes();
            let start = child.start_byte();
            let end = child.end_byte();
            if end <= bytes.len() && start < end {
                let raw = std::str::from_utf8(&bytes[start..end]).ok()?.trim();
                if !raw.is_empty() {
                    return Some(raw.to_string());
                }
            }
            return None;
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
    let mut out = String::new();
    let mut found = false;
    loop {
        let child = cursor.node();
        if matches!(child.kind_id().into(), Markdown::CodeFenceContent) {
            let bytes = source.as_bytes();
            let start = child.start_byte();
            let end = child.end_byte();
            if end <= bytes.len() && start < end {
                let chunk = std::str::from_utf8(&bytes[start..end]).ok()?;
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(chunk);
                found = true;
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    if found { Some(out) } else { None }
}
