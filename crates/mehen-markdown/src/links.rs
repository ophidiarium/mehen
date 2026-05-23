// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Link classification, debt, and scent metrics per §11.
//!
//! This module classifies pulldown-cmark document facts for every link,
//! image, autolink, footnote reference, and link reference definition. It
//! computes the aggregate scores in §11.2-§11.4. Internal anchors are resolved
//! against a GFM-style heading slug table, and relative paths are resolved
//! against the filesystem (scanning relative to the directory of the source
//! file).
//! External URLs are never checked on the network by default — they are
//! tagged `unchecked` (`resolved = None`) and a future `--link-check` flag
//! will wire up active probing.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::document::{LinkUse, LinkUseKind, MarkdownDocument, normalize_reference_label};
use crate::mathops::{clamp01, sat};
use crate::types::{LinkClass, LinkRecord, Links, Section};

/// Entry point. Classifies every link/image/autolink/footnote fact, resolves
/// anchors + relative paths, and returns a deterministic record vector plus
/// the aggregate Links struct.
pub(crate) fn analyze_links(
    document: &MarkdownDocument,
    file_path: &Path,
    sections: &[Section],
    same_repo_prefixes: &[String],
) -> (Vec<LinkRecord>, Links) {
    let anchors = collect_anchor_slugs(document);
    let footnote_labels = collect_footnote_labels(document);
    let base_dir = file_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut records: Vec<LinkRecord> = Vec::new();
    let definitions = document.reference_definition_labels();

    for definition in &document.reference_definitions {
        records.push(LinkRecord {
            line: definition.line,
            class: LinkClass::ReferenceDefinition,
            destination: definition.destination.clone(),
            text: definition.label.clone(),
            is_image: false,
            is_bare_url: false,
            resolved: None,
        });
    }

    for link in &document.links {
        if let Some(record) = classify_link_or_image(link) {
            records.push(record);
        }
    }

    for footnote in &document.footnote_references {
        records.push(LinkRecord {
            line: footnote.line,
            class: LinkClass::Footnote,
            destination: footnote.label.clone(),
            text: format!("[^{}]", footnote.label),
            is_image: false,
            is_bare_url: false,
            resolved: None,
        });
    }

    // Resolve internal anchors + relative paths + reference shortcuts.
    for r in records.iter_mut() {
        match r.class {
            LinkClass::Internal => {
                let slug = slugify_fragment(&r.destination);
                r.resolved = Some(anchors.contains(&slug));
            }
            LinkClass::Relative => {
                if r.destination.trim().is_empty() {
                    r.resolved = Some(false);
                } else {
                    let (path_part, fragment) = split_fragment(&r.destination);
                    let file_ok = resolve_relative(&base_dir, path_part);
                    let fragment_ok = fragment_ok_same_file(fragment, &anchors, path_part);
                    r.resolved = Some(file_ok && fragment_ok);
                }
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
            LinkClass::UnresolvedReferenceUse => {
                let resolved =
                    definitions.contains_key(normalize_reference_label(&r.text).as_str());
                r.resolved = Some(resolved);
            }
            LinkClass::ReferenceDefinition => {
                r.resolved = None;
            }
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

fn classify_link_or_image(link: &LinkUse) -> Option<LinkRecord> {
    let (class, destination, text) =
        if link.kind.is_reference_style() && link.destination.is_empty() {
            let reference = link
                .reference_label
                .as_deref()
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| link.text.trim());
            (
                LinkClass::UnresolvedReferenceUse,
                String::new(),
                reference.to_string(),
            )
        } else {
            let destination = link.destination.clone();
            let class = match link.kind {
                LinkUseKind::Email => LinkClass::External,
                _ => classify_destination(&destination),
            };
            (class, destination, link.text.clone())
        };

    if destination.is_empty() && text.is_empty() {
        return None;
    }

    let is_bare_url = matches!(link.kind, LinkUseKind::Autolink | LinkUseKind::Email)
        || (!text.is_empty() && text.trim() == destination.trim() && looks_like_url(&destination));

    Some(LinkRecord {
        line: link.line,
        class,
        destination,
        text,
        is_image: link.is_image,
        is_bare_url,
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

/// Compute GitHub-style anchor slugs from pulldown heading text.
fn collect_anchor_slugs(document: &MarkdownDocument) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    for heading in &document.headings {
        // GitHub-style de-duplication: the first heading with a given slug
        // keeps the bare slug; subsequent headings get `-1`, `-2`, etc.
        // appended. Anchor links like `#intro-1` in a doc with two
        // `## Intro` headings should resolve to the second one.
        let base = slugify(&heading.text);
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
    out
}

fn collect_footnote_labels(document: &MarkdownDocument) -> HashSet<String> {
    document
        .footnote_definitions
        .iter()
        .map(|definition| definition.label.clone())
        .collect()
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
            LinkClass::UnresolvedReferenceUse => {
                links.relative += 1;
            }
            LinkClass::ReferenceDefinition => {
                // Reference definitions are anchors for the reference-style
                // `[abc]` links, not outbound links of their own. They are
                // tracked via `records` for shortcut resolution but not in
                // the `total`/`broken` aggregates.
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
                !matches!(
                    r.class,
                    LinkClass::ReferenceDefinition | LinkClass::UnresolvedReferenceUse
                ) && is_descriptive_text(&r.text)
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
        let document = crate::document::parse_document(src);
        let slugs = collect_anchor_slugs(&document);
        assert!(slugs.contains("intro"), "base slug present");
        assert!(slugs.contains("intro-1"), "second occurrence gets -1");
        assert!(slugs.contains("intro-2"), "third occurrence gets -2");
        // `-3` should NOT be generated unless there's a fourth heading.
        assert!(!slugs.contains("intro-3"));
    }

    #[test]
    fn unresolved_reference_link_does_not_resolve_against_itself() {
        let src = "See [missing][nope].\n";
        let document = crate::document::parse_document(src);
        let (records, _) = analyze_links(&document, Path::new("README.md"), &[], &[]);

        let missing = records
            .iter()
            .find(|record| record.text == "nope")
            .expect("nope reference link record");
        assert_eq!(missing.class, LinkClass::UnresolvedReferenceUse);
        assert_eq!(missing.resolved, Some(false));
    }

    #[test]
    fn unresolved_reference_image_counts_as_image_and_broken() {
        let src = "![alt][missing]\n";
        let document = crate::document::parse_document(src);
        let (records, aggregate) = analyze_links(&document, Path::new("README.md"), &[], &[]);

        let image = records
            .iter()
            .find(|record| record.is_image)
            .expect("unresolved reference image record");
        assert_eq!(image.class, LinkClass::UnresolvedReferenceUse);
        assert_eq!(image.resolved, Some(false));
        assert_eq!(aggregate.image, 1);
        assert_eq!(aggregate.broken, 1);
    }

    #[test]
    fn empty_inline_link_is_unresolved() {
        let src = "See [placeholder]().\n";
        let document = crate::document::parse_document(src);
        let (records, aggregate) = analyze_links(&document, Path::new("README.md"), &[], &[]);

        let placeholder = records
            .iter()
            .find(|record| record.text == "placeholder")
            .expect("empty inline link record");
        assert_eq!(placeholder.class, LinkClass::Relative);
        assert_eq!(placeholder.resolved, Some(false));
        assert_eq!(aggregate.broken, 1);
    }

    #[test]
    fn autolinks_are_bare_links() {
        let src = "See <https://example.com> and <team@example.com>.\n";
        let document = crate::document::parse_document(src);
        let (records, aggregate) = analyze_links(&document, Path::new("README.md"), &[], &[]);

        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|record| record.is_bare_url));
        assert_eq!(aggregate.bare_url, 2);
    }

    #[test]
    fn duplicate_reference_definitions_are_counted() {
        let src = "[dup]: /one\n[dup]: /two\n";
        let document = crate::document::parse_document(src);
        let (records, _) = analyze_links(&document, Path::new("README.md"), &[], &[]);

        let destinations = records
            .iter()
            .filter(|record| record.class == LinkClass::ReferenceDefinition)
            .map(|record| record.destination.as_str())
            .collect::<Vec<_>>();
        assert_eq!(destinations, vec!["/one", "/two"]);
    }

    #[test]
    fn escaped_reference_label_resolves() {
        let src = "# Target\n\n[foo\\]]: #target\n\nSee [visible][foo\\]].\n";
        let document = crate::document::parse_document(src);
        let (records, aggregate) = analyze_links(&document, Path::new("README.md"), &[], &[]);

        let link_use = records
            .iter()
            .find(|record| record.text == "visible")
            .expect("escaped-label reference use");
        assert_eq!(link_use.class, LinkClass::Internal);
        assert_eq!(link_use.destination, "#target");
        assert_eq!(link_use.resolved, Some(true));
        assert_eq!(aggregate.broken, 0);
    }

    #[test]
    fn full_reference_link_resolves_with_reference_key_not_visible_text() {
        let src = "# Target\n\nSee [visible][ref].\n\n[ref]: #target\n";
        let document = crate::document::parse_document(src);
        let (records, aggregate) = analyze_links(&document, Path::new("README.md"), &[], &[]);

        let link_use = records
            .iter()
            .find(|record| record.line == 3)
            .expect("reference link use");
        assert_eq!(link_use.class, LinkClass::Internal);
        assert_eq!(link_use.destination, "#target");
        assert_eq!(link_use.text, "visible");
        assert_eq!(link_use.resolved, Some(true));
        assert_eq!(aggregate.broken, 0);
    }
}
