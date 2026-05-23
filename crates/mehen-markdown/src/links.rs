// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Link classification, debt, and scent metrics per §11.
//!
//! This module walks the AST for every `link`, `image`, `autolink`,
//! `footnote_reference`, and `link_reference_definition` node, classifies
//! them per §11.1, and computes the aggregate scores in §11.2–§11.4. Internal
//! anchors are resolved against the heading slug table (GFM rules) derived
//! directly from source bytes, and relative paths are resolved against the
//! filesystem (scanning relative to the directory of the source file).
//! External URLs are never checked on the network by default — they are
//! tagged `unchecked` (`resolved = None`) and a future `--link-check` flag
//! will wire up active probing.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::grammar::Markdown;
use crate::mathops::{clamp01, sat};
use crate::syntax_tree::Node;
use crate::tree_helpers::{find_first, node_text};
use crate::types::{LinkClass, LinkRecord, Links, Section};

/// Entry point. Walks the tree, classifies every link/image/autolink/footnote
/// node, resolves anchors + relative paths, and returns a deterministic
/// record vector plus the aggregate Links struct.
pub(crate) fn analyze_links(
    root: &Node<'_>,
    source: &str,
    file_path: &Path,
    sections: &[Section],
    same_repo_prefixes: &[String],
) -> (Vec<LinkRecord>, Links) {
    let anchors = collect_anchor_slugs(root, source);
    let footnote_labels = collect_footnote_labels(root, source);
    let base_dir = file_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let definition_labels = collect_reference_definition_labels(root, source);
    let mut records: Vec<LinkRecord> = Vec::new();
    collect_links(root, source, &mut records);

    // Resolve internal anchors + relative paths + reference shortcuts.
    for r in records.iter_mut() {
        match r.class {
            LinkClass::Internal => {
                let slug = slugify_fragment(&r.destination);
                r.resolved = Some(anchors.contains(&slug));
            }
            LinkClass::Relative => {
                let (path_part, fragment) = split_fragment(&r.destination);
                let file_ok = resolve_relative(&base_dir, path_part);
                let fragment_ok = fragment_ok_same_file(fragment, &anchors, path_part);
                r.resolved = Some(file_ok && fragment_ok);
            }
            LinkClass::AbsoluteSameRepo
            | LinkClass::External
            | LinkClass::ExternalVendor
            | LinkClass::Scholarly
            | LinkClass::IssuePr => {
                r.resolved = None;
            }
            LinkClass::Footnote => {
                r.resolved = Some(footnote_labels.contains(&r.destination));
            }
            LinkClass::ReferenceDefinition => {
                r.resolved = None;
            }
        }

        // If this is a reference-style link (shortcut or collapsed `[abc]`)
        // that we classified as ReferenceDefinition because we couldn't see
        // a destination on the link itself, re-classify based on whether
        // the label matches a known definition.
        if r.class == LinkClass::ReferenceDefinition && !r.is_bare_url && r.destination.is_empty() {
            let resolved = definition_labels.contains(&r.text.to_lowercase());
            r.resolved = Some(resolved);
        }

        // Promote plain `External` URLs that point back at the same repo.
        if matches!(r.class, LinkClass::External)
            && !same_repo_prefixes.is_empty()
            && same_repo_prefixes
                .iter()
                .any(|p| r.destination.starts_with(p.as_str()))
        {
            r.class = LinkClass::AbsoluteSameRepo;
        }
    }

    // Determinism: sort by line, then destination, then class.
    records.sort_by(|a, b| {
        a.line
            .cmp(&b.line)
            .then(a.destination.cmp(&b.destination))
            .then((a.class as u8).cmp(&(b.class as u8)))
    });

    let aggregate = aggregate_links(&records, sections);
    (records, aggregate)
}

fn collect_reference_definition_labels(root: &Node<'_>, source: &str) -> HashSet<String> {
    let mut labels = HashSet::new();
    collect_reference_definition_labels_rec(root, source, &mut labels);
    labels
}

fn collect_reference_definition_labels_rec(
    node: &Node<'_>,
    source: &str,
    labels: &mut HashSet<String>,
) {
    let kind: Markdown = node.kind_id().into();
    if matches!(kind, Markdown::LinkReferenceDefinition) {
        if let Some(label) = find_first(node, Markdown::LinkLabel)
            .and_then(|label| text_inside_label(&label, source))
        {
            labels.insert(label.to_lowercase());
        }
        return;
    }

    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            collect_reference_definition_labels_rec(&cursor.node(), source, labels);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn collect_links(node: &Node<'_>, source: &str, records: &mut Vec<LinkRecord>) {
    use Markdown::*;
    let kind: Markdown = node.kind_id().into();
    match kind {
        Link | Image | ImageBlock => {
            let is_image = matches!(kind, Image | ImageBlock);
            if let Some(record) = classify_link_or_image(node, source, is_image) {
                records.push(record);
            }
            // Don't recurse — nested link labels would otherwise be
            // double-counted. Reference-style shortcuts like `[abc][def]`
            // surface as two Link nodes the grammar already splits for us.
            return;
        }
        Autolink => {
            if let Some(record) = classify_autolink(node, source) {
                records.push(record);
            }
            return;
        }
        FootnoteReference => {
            if let Some(record) = classify_footnote_reference(node, source) {
                records.push(record);
            }
            return;
        }
        LinkReferenceDefinition => {
            if let Some(record) = classify_reference_definition(node, source) {
                records.push(record);
            }
            return;
        }
        _ => {}
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            collect_links(&cursor.node(), source, records);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn classify_link_or_image(node: &Node<'_>, source: &str, is_image: bool) -> Option<LinkRecord> {
    let line = (node.start_row() as u64) + 1;
    let label = find_first(node, Markdown::LinkLabel).and_then(|n| text_inside_label(&n, source));
    let destination = find_first(node, Markdown::LinkDestination).map(|n| node_text(&n, source));
    let (class, dest, text) = if is_full_reference_style_link(node, source)
        && let Some(reference_key) = destination.as_ref().map(|dest| dest.trim().to_string())
    {
        (LinkClass::ReferenceDefinition, String::new(), reference_key)
    } else if let Some(dest) = destination {
        let text = label.clone().unwrap_or_default();
        (classify_destination(&dest), dest, text)
    } else if let Some(label_text) = label.clone() {
        // Shortcut / collapsed reference: `[abc]` or `[abc][]`. The label
        // doubles as the reference key. Resolution happens against the
        // definition table in `analyze_links`.
        (LinkClass::ReferenceDefinition, String::new(), label_text)
    } else {
        return None;
    };

    let is_bare_url = !text.is_empty() && text.trim() == dest.trim() && looks_like_url(&dest);

    Some(LinkRecord {
        line,
        class,
        destination: dest,
        text,
        is_image,
        is_bare_url,
        resolved: None,
    })
}

fn is_full_reference_style_link(node: &Node<'_>, source: &str) -> bool {
    let text = node_text(node, source);
    let Some(close) = text.rfind(']') else {
        return false;
    };
    let Some(open) = text[..close].rfind('[') else {
        return false;
    };
    open > 0 && text[..open].ends_with(']') && !text[open + 1..close].trim().is_empty()
}

fn classify_autolink(node: &Node<'_>, source: &str) -> Option<LinkRecord> {
    let line = (node.start_row() as u64) + 1;
    let uri = find_first(node, Markdown::Uri)
        .or_else(|| find_first(node, Markdown::Email))
        .map(|n| node_text(&n, source))?;
    let class = if uri.contains('@') && !uri.starts_with("mailto:") {
        LinkClass::External
    } else {
        classify_destination(&uri)
    };
    Some(LinkRecord {
        line,
        class,
        destination: uri.clone(),
        text: uri,
        is_image: false,
        is_bare_url: true,
        resolved: None,
    })
}

fn classify_footnote_reference(node: &Node<'_>, source: &str) -> Option<LinkRecord> {
    let line = (node.start_row() as u64) + 1;
    let raw_label =
        find_first(node, Markdown::FootnoteReferenceLabel).map(|n| node_text(&n, source))?;
    let label = normalize_footnote_label(&raw_label);
    Some(LinkRecord {
        line,
        class: LinkClass::Footnote,
        destination: label.clone(),
        text: format!("[^{label}]"),
        is_image: false,
        is_bare_url: false,
        resolved: None,
    })
}

fn normalize_footnote_label(label: &str) -> String {
    label
        .trim()
        .trim_start_matches("[^")
        .trim_start_matches('^')
        .trim_end_matches(']')
        .trim()
        .to_string()
}

fn classify_reference_definition(node: &Node<'_>, source: &str) -> Option<LinkRecord> {
    let line = (node.start_row() as u64) + 1;
    let label =
        find_first(node, Markdown::LinkLabel).and_then(|n| text_inside_label(&n, source))?;
    let destination = find_first(node, Markdown::LinkDestination).map(|n| node_text(&n, source))?;
    Some(LinkRecord {
        line,
        class: LinkClass::ReferenceDefinition,
        destination,
        text: label,
        is_image: false,
        is_bare_url: false,
        resolved: None,
    })
}

fn classify_destination(dest: &str) -> LinkClass {
    let trimmed = dest.trim();
    if trimmed.is_empty() {
        return LinkClass::Relative;
    }
    if trimmed.starts_with('#') {
        return LinkClass::Internal;
    }
    if looks_like_absolute_url(trimmed) {
        if is_scholarly(trimmed) {
            return LinkClass::Scholarly;
        }
        if is_issue_pr(trimmed) {
            return LinkClass::IssuePr;
        }
        if is_external_vendor(trimmed) {
            return LinkClass::ExternalVendor;
        }
        return LinkClass::External;
    }
    LinkClass::Relative
}

fn looks_like_absolute_url(s: &str) -> bool {
    if s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("ftp://")
        || s.starts_with("ftps://")
        || s.starts_with("file://")
        || s.starts_with("data:")
        || s.starts_with("mailto:")
        || s.starts_with("tel:")
    {
        return true;
    }
    if let Some((scheme, rest)) = s.split_once("://")
        && rest.starts_with(|c: char| !c.is_whitespace())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+')
    {
        return true;
    }
    false
}

fn looks_like_url(s: &str) -> bool {
    looks_like_absolute_url(s) || s.starts_with("www.")
}

fn is_scholarly(s: &str) -> bool {
    let host = host_of(s).unwrap_or("");
    matches!(
        host,
        "doi.org"
            | "dx.doi.org"
            | "arxiv.org"
            | "www.arxiv.org"
            | "datatracker.ietf.org"
            | "rfc-editor.org"
            | "www.rfc-editor.org"
            | "tools.ietf.org"
            | "www.w3.org"
            | "w3.org"
            | "pubmed.ncbi.nlm.nih.gov"
            | "www.ncbi.nlm.nih.gov"
            | "ncbi.nlm.nih.gov"
    )
}

fn is_issue_pr(s: &str) -> bool {
    let host = host_of(s).unwrap_or("");
    let after_host = after_host(s).unwrap_or("");
    match host {
        "github.com" | "www.github.com" | "gitlab.com" | "www.gitlab.com" => {
            let segs: Vec<&str> = after_host.split('/').filter(|s| !s.is_empty()).collect();
            if segs.len() >= 4 {
                let kind = segs[2];
                let id = segs[3];
                let is_int = id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty();
                let kind_ok = matches!(kind, "issues" | "pull" | "pulls" | "merge_requests");
                return is_int && kind_ok;
            }
            false
        }
        _ => {
            s.contains("/browse/")
                || (host.contains("atlassian.net") && s.contains("/browse/"))
                || (host == "linear.app" && s.contains("/issue/"))
        }
    }
}

fn is_external_vendor(s: &str) -> bool {
    let host = host_of(s).unwrap_or("");
    matches!(
        host,
        "docs.aws.amazon.com"
            | "aws.amazon.com"
            | "developer.mozilla.org"
            | "doc.rust-lang.org"
            | "docs.rs"
            | "crates.io"
            | "rust-lang.org"
            | "www.rust-lang.org"
            | "learn.microsoft.com"
            | "docs.microsoft.com"
            | "kubernetes.io"
            | "docs.python.org"
            | "nodejs.org"
            | "reactjs.org"
            | "react.dev"
            | "developer.apple.com"
            | "developer.android.com"
            | "cloud.google.com"
            | "cloud.ibm.com"
            | "azure.microsoft.com"
            | "spec.commonmark.org"
            | "github.github.com"
            | "docs.github.com"
            | "docs.gitlab.com"
    )
}

fn host_of(s: &str) -> Option<&str> {
    let after_scheme = s.split_once("://").map(|(_, rest)| rest)?;
    let end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    Some(&after_scheme[..end])
}

fn after_host(s: &str) -> Option<&str> {
    let after_scheme = s.split_once("://").map(|(_, rest)| rest)?;
    let host_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    Some(&after_scheme[host_end..])
}

fn split_fragment(s: &str) -> (&str, &str) {
    match s.find('#') {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, ""),
    }
}

fn fragment_ok_same_file(fragment: &str, anchors: &HashSet<String>, path_part: &str) -> bool {
    if fragment.is_empty() {
        return true;
    }
    if path_part.is_empty() {
        return anchors.contains(&slugify(fragment));
    }
    // Cross-file fragment: we don't currently re-parse the target, so we
    // optimistically accept once the file itself resolves. Marking it as
    // true here avoids a false positive in the broken-link count.
    true
}

fn resolve_relative(base_dir: &Path, rel: &str) -> bool {
    if rel.is_empty() {
        return true;
    }
    // Strip a single leading `/` so absolute-style relatives resolve from
    // the Markdown file's directory. Do NOT strip leading `.` — `./foo.md`
    // and `../bar.md` are valid relative paths that Path::join handles
    // natively. Previously trimming `.` turned them into `/foo.md` and
    // reported valid sibling/parent links as broken (Codex P1).
    let rel = rel.strip_prefix('/').unwrap_or(rel);
    let candidate = base_dir.join(rel);
    candidate.exists()
}

/// GFM-style slug used by GitHub's Markdown renderer. The algorithm is:
///
/// 1. Lowercase (for ASCII).
/// 2. Strip punctuation except `-`, `_`, and alphanumerics.
/// 3. Replace whitespace runs with a single `-`.
fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_dash = false;
    for ch in text.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_alphanumeric() || lower == '_' {
            out.push(lower);
            prev_dash = false;
        } else if lower.is_whitespace() || lower == '-' {
            if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
        } else {
            continue;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn slugify_fragment(s: &str) -> String {
    slugify(s.trim_start_matches('#'))
}

/// Walk every heading in the document and compute its slug from the heading
/// text bytes in the source. This is independent of the Phase-A section
/// tree, which does not currently extract heading text.
fn collect_anchor_slugs(root: &Node<'_>, source: &str) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    collect_heading_slugs(root, source, &mut out);
    out
}

fn collect_heading_slugs(node: &Node<'_>, source: &str, out: &mut HashSet<String>) {
    use Markdown::*;
    let kind: Markdown = node.kind_id().into();
    let is_heading = matches!(
        kind,
        AtxHeading
            | AtxHeading2
            | AtxHeading3
            | AtxHeading4
            | AtxHeading5
            | AtxHeading6
            | SetextHeading
            | SetextHeading2
    );
    if is_heading && let Some(text) = heading_text(node, source) {
        // GitHub-style de-duplication: the first heading with a given slug
        // keeps the bare slug; subsequent headings get `-1`, `-2`, etc.
        // appended. Anchor links like `#intro-1` in a doc with two
        // `## Intro` headings should resolve to the second one.
        let base = slugify(&text);
        if out.insert(base.clone()) {
            // first occurrence
        } else {
            for n in 1.. {
                let candidate = format!("{base}-{n}");
                if out.insert(candidate) {
                    break;
                }
            }
        }
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            collect_heading_slugs(&cursor.node(), source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn heading_text(heading: &Node<'_>, source: &str) -> Option<String> {
    // Walk the heading's children, strip the leading `#*` / underline markers,
    // and concatenate word-like token text. This is an approximation that
    // works well enough for slugification in Phase C.
    let start = heading.start_byte();
    let end = heading.end_byte();
    let raw = &source.as_bytes()[start..end];
    let s = String::from_utf8_lossy(raw);
    let trimmed = s.trim();
    // Strip setext underline rows ("===", "---") if present.
    let line = trimmed.lines().next().unwrap_or("").trim();
    // Drop leading `#` markers and any trailing `#` markers per CommonMark.
    let text = line.trim_start_matches('#').trim();
    let text = text.trim_end_matches('#').trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn collect_footnote_labels(root: &Node<'_>, source: &str) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    collect_footnote_labels_rec(root, source, &mut out);
    out
}

fn collect_footnote_labels_rec(node: &Node<'_>, source: &str, out: &mut HashSet<String>) {
    use Markdown::*;
    let kind: Markdown = node.kind_id().into();
    if matches!(kind, FootnoteDefinition)
        && let Some(label) = find_first(node, FootnoteLabel)
    {
        let text = node_text(&label, source);
        out.insert(normalize_footnote_label(&text));
        return;
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            collect_footnote_labels_rec(&cursor.node(), source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn text_inside_label(label: &Node<'_>, source: &str) -> Option<String> {
    let s = node_text(label, source);
    let trimmed = s
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_start_matches('^')
        .trim()
        .to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn aggregate_links(records: &[LinkRecord], sections: &[Section]) -> Links {
    let mut links = Links::default();

    for r in records {
        match r.class {
            LinkClass::Internal => links.internal += 1,
            LinkClass::Relative => links.relative += 1,
            LinkClass::External => links.external += 1,
            LinkClass::ExternalVendor => {
                links.external += 1;
                links.external_vendor += 1;
            }
            LinkClass::Scholarly => {
                links.external += 1;
                links.scholarly += 1;
            }
            LinkClass::IssuePr => {
                links.external += 1;
                links.issue_pr += 1;
            }
            LinkClass::AbsoluteSameRepo => {
                links.absolute_same_repo += 1;
            }
            LinkClass::Footnote => links.footnote += 1,
            LinkClass::ReferenceDefinition => {
                // Reference definitions are anchors for the reference-style
                // `[abc]` links, not outbound links of their own. They are
                // tracked via `records` for shortcut resolution but not in
                // the `total`/`broken` aggregates.
                if r.destination.is_empty() {
                    // This is a shortcut/collapsed use of an undefined ref,
                    // so it still counts toward the total as a relative-ish
                    // broken link.
                    links.relative += 1;
                    links.total += 1;
                    if matches!(r.resolved, Some(false)) {
                        links.broken += 1;
                    }
                }
                continue;
            }
        }
        links.total += 1;
        if r.is_image {
            links.image += 1;
        }
        if r.is_bare_url {
            links.bare_url += 1;
        }
        if matches!(r.resolved, Some(false)) {
            links.broken += 1;
        }
    }

    let total = links.total.max(1) as f64;
    let total_internal = links.internal.max(1) as f64;
    let l_broken = links.broken as f64;
    let l_ext = links.external as f64;

    let broken_rate = l_broken / total;
    let bare_rate = links.bare_url as f64 / total;
    let external_rate = l_ext / total;

    let missing_internal_anchors = records
        .iter()
        .filter(|r| r.class == LinkClass::Internal && matches!(r.resolved, Some(false)))
        .count() as f64;
    let anchor_miss_rate = missing_internal_anchors / total_internal;

    let words = sections.iter().map(|s| s.word_count).sum::<u64>().max(1) as f64;
    let link_density_per_100w = links.total as f64 / (words / 100.0).max(1.0);

    links.link_debt_score = clamp01(
        0.45 * sat(broken_rate, 0.00, 0.10)
            + 0.20 * sat(anchor_miss_rate, 0.00, 0.10)
            + 0.15 * sat(bare_rate, 0.05, 0.30)
            + 0.10 * sat(external_rate, 0.60, 0.90)
            + 0.10 * sat(link_density_per_100w, 6.0, 14.0),
    );

    let descriptive_rate = if links.total == 0 {
        0.0
    } else {
        records
            .iter()
            .filter(|r| {
                !matches!(r.class, LinkClass::ReferenceDefinition) && is_descriptive_text(&r.text)
            })
            .count() as f64
            / links.total as f64
    };
    let resolved_relative_rate = if links.relative == 0 {
        0.0
    } else {
        records
            .iter()
            .filter(|r| r.class == LinkClass::Relative && matches!(r.resolved, Some(true)))
            .count() as f64
            / links.relative as f64
    };
    let anchor_success_rate = if links.internal == 0 {
        0.0
    } else {
        records
            .iter()
            .filter(|r| r.class == LinkClass::Internal && matches!(r.resolved, Some(true)))
            .count() as f64
            / links.internal as f64
    };
    let reference_section_present = if has_reference_section(sections) {
        1.0
    } else {
        0.0
    };
    links.information_scent_score = clamp01(
        0.30 * descriptive_rate
            + 0.30 * resolved_relative_rate
            + 0.20 * anchor_success_rate
            + 0.20 * reference_section_present,
    );

    links.review_burden = 0.3 * links.internal as f64
        + 0.8 * links.relative as f64
        + 1.0 * l_ext
        + 2.5 * l_broken
        + 0.5 * links.footnote as f64;

    links
}

fn is_descriptive_text(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return false;
    }
    if looks_like_url(&t) {
        return false;
    }
    !matches!(
        t.as_str(),
        "here" | "link" | "click here" | "this" | "read more" | "more" | ">" | "..." | "…"
    )
}

fn has_reference_section(sections: &[Section]) -> bool {
    sections.iter().any(|s| {
        s.heading_text
            .as_deref()
            .map(|t| {
                let l = t.trim().to_lowercase();
                l == "references"
                    || l == "bibliography"
                    || l == "works cited"
                    || l == "further reading"
                    || l == "see also"
            })
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_applies_gfm_rules() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Section 1!"), "section-1");
        assert_eq!(slugify("  Leading & Trailing  "), "leading-trailing");
        assert_eq!(slugify("snake_case"), "snake_case");
        assert_eq!(slugify("Dash-Case"), "dash-case");
    }

    #[test]
    fn classify_detects_external() {
        assert_eq!(
            classify_destination("https://example.com"),
            LinkClass::External
        );
        assert_eq!(classify_destination("#top"), LinkClass::Internal);
        assert_eq!(classify_destination("docs/api.md"), LinkClass::Relative);
        assert_eq!(
            classify_destination("https://doi.org/10.1/abc"),
            LinkClass::Scholarly
        );
        assert_eq!(
            classify_destination("https://github.com/foo/bar/issues/1"),
            LinkClass::IssuePr
        );
        assert_eq!(
            classify_destination("https://docs.aws.amazon.com/lambda/"),
            LinkClass::ExternalVendor
        );
    }

    #[test]
    fn host_and_after_host_split_url() {
        assert_eq!(host_of("https://foo.bar/baz?x=1"), Some("foo.bar"));
        assert_eq!(after_host("https://foo.bar/baz?x=1"), Some("/baz?x=1"));
    }

    #[test]
    fn resolve_relative_preserves_dot_prefixed_paths() {
        // Codex P1 on PR #84: `./foo.md` and `../bar.md` must resolve
        // against `base_dir`, not against the filesystem root. Previously
        // `trim_start_matches('.')` turned them into absolute-style paths
        // that reported valid sibling/parent links as broken.
        let tmp = tempfile::tempdir().expect("tempdir");
        let base_dir = tmp.path();
        let sibling = base_dir.join("foo.md");
        std::fs::write(&sibling, b"# Foo\n").expect("write sibling");
        let parent = base_dir.parent().expect("parent dir exists");
        // `./foo.md` should resolve inside base_dir.
        assert!(
            resolve_relative(base_dir, "./foo.md"),
            "./foo.md must resolve against base_dir"
        );
        // `../<leaf>` with a non-existent target should NOT resolve…
        assert!(
            !resolve_relative(base_dir, "../definitely-not-a-real-file-xyz.md"),
            "missing parent file must report unresolved"
        );
        // …but a real parent path does.
        if let Some(parent_name) = base_dir.file_name().and_then(|s| s.to_str()) {
            // Create a sibling of base_dir so `../<parent_name>/foo.md` exists.
            let nested = parent.join(parent_name).join("foo.md");
            assert!(nested.exists());
            assert!(resolve_relative(
                base_dir,
                &format!("../{}/foo.md", parent_name)
            ));
        }
    }

    #[test]
    fn collect_heading_slugs_dedups_with_github_numeric_suffix() {
        // Codex P2 on PR #84: GitHub appends `-1`, `-2`, … to duplicate
        // heading slugs. Anchor collection must match so `#intro-1` on
        // a doc with two `## Intro` headings resolves cleanly.
        let src = "## Intro\n\nfirst\n\n## Intro\n\nsecond\n\n## Intro\n\nthird\n";
        let tree = crate::syntax_tree::parse(src);
        let root = tree.root();
        let slugs = collect_anchor_slugs(&root, src);
        assert!(slugs.contains("intro"), "base slug present");
        assert!(slugs.contains("intro-1"), "second occurrence gets -1");
        assert!(slugs.contains("intro-2"), "third occurrence gets -2");
        // `-3` should NOT be generated unless there's a fourth heading.
        assert!(!slugs.contains("intro-3"));
    }

    #[test]
    fn unresolved_reference_link_does_not_resolve_against_itself() {
        let src = "See [missing][nope].\n";
        let tree = crate::syntax_tree::parse(src);
        let root = tree.root();
        let (records, _) = analyze_links(&root, src, Path::new("README.md"), &[], &[]);

        let missing = records
            .iter()
            .find(|record| record.text == "nope")
            .expect("nope reference link record");
        assert_eq!(missing.resolved, Some(false));
    }

    #[test]
    fn full_reference_link_resolves_with_reference_key_not_visible_text() {
        let src = "See [visible][ref].\n\n[ref]: docs.md\n";
        let tree = crate::syntax_tree::parse(src);
        let root = tree.root();
        let (records, aggregate) = analyze_links(&root, src, Path::new("README.md"), &[], &[]);

        let link_use = records
            .iter()
            .find(|record| record.line == 1)
            .expect("reference link use");
        assert_eq!(link_use.class, LinkClass::ReferenceDefinition);
        assert_eq!(link_use.text, "ref");
        assert_eq!(link_use.resolved, Some(true));
        assert_eq!(aggregate.broken, 0);
    }
}
