// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Pulldown-cmark backed Markdown document facts.
//!
//! Metric passes that need Markdown semantics should consume this module
//! directly instead of reconstructing those semantics from structural node
//! walks. Cursor-style passes can still use the compact structural tree for
//! nested spans, while links, anchors, reference definitions, footnotes, and
//! code blocks stay native pulldown data here.

use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{
    BrokenLink, CodeBlockKind as PulldownCodeBlockKind, CowStr, Event, HeadingLevel, LinkType,
    Options, Parser, Tag, TagEnd,
};

use crate::source_text::normalize_line_endings;

#[derive(Debug)]
pub(crate) struct MarkdownDocument {
    pub(crate) headings: Vec<Heading>,
    pub(crate) links: Vec<LinkUse>,
    pub(crate) reference_definitions: Vec<ReferenceDefinition>,
    pub(crate) footnote_references: Vec<FootnoteReference>,
    pub(crate) footnote_definitions: Vec<FootnoteDefinition>,
    pub(crate) code_blocks: Vec<CodeBlock>,
    code_block_start_lines: HashMap<u64, usize>,
}

#[derive(Debug)]
pub(crate) struct Heading {
    pub(crate) text: String,
}

#[derive(Debug)]
pub(crate) struct LinkUse {
    pub(crate) line: u64,
    pub(crate) kind: LinkUseKind,
    pub(crate) destination: String,
    pub(crate) reference_label: Option<String>,
    pub(crate) text: String,
    pub(crate) is_image: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkUseKind {
    Inline,
    Reference,
    ReferenceUnknown,
    Collapsed,
    CollapsedUnknown,
    Shortcut,
    ShortcutUnknown,
    Autolink,
    Email,
    WikiLink,
}

impl LinkUseKind {
    pub(crate) fn is_reference_style(self) -> bool {
        matches!(
            self,
            Self::Reference
                | Self::ReferenceUnknown
                | Self::Collapsed
                | Self::CollapsedUnknown
                | Self::Shortcut
                | Self::ShortcutUnknown
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ReferenceDefinition {
    pub(crate) line: u64,
    pub(crate) label: String,
    pub(crate) destination: String,
    pub(crate) span: Range<usize>,
    pub(crate) label_span: Range<usize>,
    pub(crate) destination_span: Range<usize>,
    pub(crate) title_span: Option<Range<usize>>,
}

#[derive(Debug)]
pub(crate) struct FootnoteReference {
    pub(crate) line: u64,
    pub(crate) label: String,
}

#[derive(Debug)]
pub(crate) struct FootnoteDefinition {
    pub(crate) label: String,
}

#[derive(Debug)]
pub(crate) struct CodeBlock {
    pub(crate) kind: CodeBlockKind,
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) language: Option<String>,
    pub(crate) content: String,
}

impl CodeBlock {
    pub(crate) fn is_fenced(&self) -> bool {
        self.kind == CodeBlockKind::Fenced
    }

    pub(crate) fn content_line_count(&self) -> usize {
        self.content.lines().count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodeBlockKind {
    Fenced,
    Indented,
}

#[cfg(test)]
pub(crate) fn parse_document(source: &str) -> MarkdownDocument {
    let reference_definitions = reference_definitions_from_source(source);
    let mut builder = DocumentBuilder::new(source, reference_definitions);
    let parser = Parser::new_with_broken_link_callback(
        source,
        markdown_options(),
        Some(preserve_broken_reference_link),
    );
    let offset_iter = parser.into_offset_iter();

    for (event, range) in offset_iter {
        builder.handle_event(event, range);
    }

    builder.finish()
}

pub(crate) fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_WIKILINKS
}

pub(crate) fn preserve_broken_reference_link<'a>(
    _link: BrokenLink<'a>,
) -> Option<(CowStr<'a>, CowStr<'a>)> {
    Some(("".into(), "".into()))
}

pub(crate) fn line_starts(source: &str) -> Vec<usize> {
    let mut out = vec![0];
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            out.push(idx + 1);
        }
    }
    out
}

pub(crate) fn row_at(line_starts: &[usize], source_len: usize, byte: usize) -> usize {
    let byte = byte.min(source_len);
    match line_starts.binary_search(&byte) {
        Ok(row) => row,
        Err(0) => 0,
        Err(row) => row - 1,
    }
}

pub(crate) fn normalize_reference_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(crate) fn code_language(info: &str) -> Option<String> {
    let head = info
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .next()
        .unwrap_or("")
        .trim();
    (!head.is_empty()).then(|| head.to_ascii_lowercase())
}

pub(crate) fn is_diagram_language(lang: &str) -> bool {
    matches!(
        lang,
        "mermaid"
            | "plantuml"
            | "puml"
            | "dot"
            | "graphviz"
            | "d2"
            | "vega-lite"
            | "vegalite"
            | "vl"
            | "vega"
    )
}

pub(crate) fn reference_definitions_from_source(source: &str) -> Vec<ReferenceDefinition> {
    let line_starts = line_starts(source);
    let source_blocks = SourceBlockSpans::collect(source);
    let mut definitions = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        let line = next_line_range(source, cursor);
        let line_without_eol = trim_line_ending(source, line.clone());
        let Some(content_start) = definition_content_start(source, &line_without_eol) else {
            cursor = line.end;
            continue;
        };

        if source_blocks.suppresses_reference_definition(content_start) {
            cursor = line.end;
            continue;
        }

        if let Some(definition) =
            parse_reference_definition_at(source, line.start, content_start, &line_starts)
        {
            cursor = next_line_start_after(source, definition.span.end);
            definitions.push(definition);
        } else {
            cursor = line.end;
        }
    }

    definitions
}

#[derive(Default)]
struct SourceBlockSpans {
    code_or_html: Vec<Range<usize>>,
    paragraphs: Vec<Range<usize>>,
}

impl SourceBlockSpans {
    fn collect(source: &str) -> Self {
        let mut spans = SourceBlockSpans::default();
        let mut paragraph: Option<Range<usize>> = None;
        let mut code_block: Option<Range<usize>> = None;
        let mut html_block: Option<Range<usize>> = None;

        let parser = Parser::new_with_broken_link_callback(
            source,
            markdown_options(),
            Some(preserve_broken_reference_link),
        );

        for (event, range) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Paragraph) => paragraph = Some(range),
                Event::End(TagEnd::Paragraph) => {
                    push_open_span(&mut spans.paragraphs, paragraph.take(), range)
                }
                Event::Start(Tag::CodeBlock(_)) => code_block = Some(range),
                Event::End(TagEnd::CodeBlock) => {
                    push_open_span(&mut spans.code_or_html, code_block.take(), range)
                }
                Event::Start(Tag::HtmlBlock) => html_block = Some(range),
                Event::End(TagEnd::HtmlBlock) => {
                    push_open_span(&mut spans.code_or_html, html_block.take(), range)
                }
                Event::Html(_) => {
                    if let Some(active) = html_block.as_mut() {
                        active.end = active.end.max(range.end);
                    } else {
                        spans.code_or_html.push(range);
                    }
                }
                Event::Text(_)
                | Event::Code(_)
                | Event::InlineMath(_)
                | Event::DisplayMath(_)
                | Event::InlineHtml(_)
                | Event::FootnoteReference(_)
                | Event::SoftBreak
                | Event::HardBreak
                | Event::Rule
                | Event::TaskListMarker(_) => {
                    if let Some(active) = paragraph.as_mut() {
                        active.end = active.end.max(range.end);
                    }
                    if let Some(active) = code_block.as_mut() {
                        active.end = active.end.max(range.end);
                    }
                    if let Some(active) = html_block.as_mut() {
                        active.end = active.end.max(range.end);
                    }
                }
                Event::Start(_) | Event::End(_) => {}
            }
        }

        spans
    }

    fn suppresses_reference_definition(&self, byte: usize) -> bool {
        self.code_or_html
            .iter()
            .any(|span| contains_byte(span, byte))
            || self.paragraphs.iter().any(|span| contains_byte(span, byte))
    }
}

fn push_open_span(target: &mut Vec<Range<usize>>, open: Option<Range<usize>>, end: Range<usize>) {
    if let Some(mut span) = open {
        span.end = span.end.max(end.end);
        if span.start < span.end {
            target.push(span);
        }
    }
}

fn contains_byte(span: &Range<usize>, byte: usize) -> bool {
    span.start <= byte && byte < span.end
}

fn definition_content_start(source: &str, range: &Range<usize>) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = range.start;
    cursor = skip_spaces_up_to(source, cursor, range.end, 3).0;

    loop {
        if cursor >= range.end {
            return Some(cursor);
        }

        if bytes[cursor] == b'>' {
            cursor += 1;
            if cursor < range.end && matches!(bytes[cursor], b' ' | b'\t') {
                cursor += 1;
            }
            cursor = skip_spaces_up_to(source, cursor, range.end, 3).0;
            continue;
        }

        if let Some(after_marker) = list_marker_content_start(source, cursor, range.end) {
            cursor = skip_spaces_up_to(source, after_marker, range.end, 3).0;
            continue;
        }

        break;
    }

    Some(cursor)
}

fn list_marker_content_start(source: &str, cursor: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let marker_end = match bytes.get(cursor)? {
        b'-' | b'+' | b'*' => cursor + 1,
        byte if byte.is_ascii_digit() => {
            let mut cursor = cursor;
            let mut digits = 0;
            while cursor < end && bytes[cursor].is_ascii_digit() && digits < 9 {
                cursor += 1;
                digits += 1;
            }
            if digits == 0 || !matches!(bytes.get(cursor), Some(b'.' | b')')) {
                return None;
            }
            cursor + 1
        }
        _ => return None,
    };
    if !matches!(bytes.get(marker_end), Some(b' ' | b'\t')) {
        return None;
    }
    Some(skip_spaces(source, marker_end))
}

fn skip_spaces_up_to(source: &str, mut cursor: usize, end: usize, limit: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let mut spaces = 0;
    while cursor < end && bytes[cursor] == b' ' && spaces < limit {
        cursor += 1;
        spaces += 1;
    }
    (cursor, spaces)
}

fn next_line_range(source: &str, start: usize) -> Range<usize> {
    let tail = &source[start..];
    match tail.find('\n') {
        Some(offset) => start..start + offset + 1,
        None => start..source.len(),
    }
}

fn next_line_start_after(source: &str, byte: usize) -> usize {
    if byte >= source.len() {
        return source.len();
    }
    source[byte..]
        .find('\n')
        .map(|offset| byte + offset + 1)
        .unwrap_or(source.len())
}

fn trim_line_ending(source: &str, range: Range<usize>) -> Range<usize> {
    let mut end = range.end;
    if end > range.start && source.as_bytes()[end - 1] == b'\n' {
        end -= 1;
    }
    if end > range.start && source.as_bytes()[end - 1] == b'\r' {
        end -= 1;
    }
    range.start..end
}

fn parse_reference_definition_at(
    source: &str,
    line_start: usize,
    content_start: usize,
    line_starts: &[usize],
) -> Option<ReferenceDefinition> {
    let bytes = source.as_bytes();
    let source_len = source.len();
    let mut cursor = content_start;
    if bytes.get(cursor) != Some(&b'[') {
        return None;
    }

    let (label_span, label, after_label) = parse_link_label(source, cursor)?;
    if label.starts_with('^') {
        return None;
    }
    if bytes.get(after_label) != Some(&b':') {
        return None;
    }
    cursor = after_label + 1;
    cursor = skip_spaces_and_one_linebreak(source, cursor)?;

    let (destination_span, destination, after_destination) =
        parse_link_destination(source, cursor)?;
    cursor = after_destination;
    let destination_end = cursor;

    let mut title_span = None;
    let mut span_end = destination_end;
    if let Some(after_space) = skip_optional_title_space(source, cursor)
        && let Some((parsed_title_span, after_title)) = parse_link_title(source, after_space)
        && only_blank_until_line_end(source, after_title)
    {
        title_span = Some(parsed_title_span);
        span_end = after_title;
    } else if !only_blank_until_line_end(source, cursor) {
        return None;
    }

    Some(ReferenceDefinition {
        line: row_at(line_starts, source_len, line_start) as u64 + 1,
        label: normalize_reference_label(&label),
        destination,
        span: line_start..span_end,
        label_span,
        destination_span,
        title_span,
    })
}

fn parse_link_label(source: &str, start: usize) -> Option<(Range<usize>, String, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'[') {
        return None;
    }
    let mut cursor = start + 1;
    let content_start = cursor;
    while cursor < source.len() {
        match bytes[cursor] {
            b'\\' if next_is_escapable_punctuation(bytes, cursor) => cursor += 2,
            b'\\' => cursor += 1,
            b'[' => return None,
            b']' => {
                let raw = &source[content_start..cursor];
                let label = unescape_markdown(raw);
                if label.trim().is_empty() {
                    return None;
                }
                return Some((start..cursor + 1, label, cursor + 1));
            }
            _ => cursor += 1,
        }
    }
    None
}

fn skip_spaces_and_one_linebreak(source: &str, mut cursor: usize) -> Option<usize> {
    cursor = skip_spaces(source, cursor);
    let bytes = source.as_bytes();
    let newline_len = match bytes.get(cursor) {
        Some(b'\n') => 1,
        Some(b'\r') if bytes.get(cursor + 1) == Some(&b'\n') => 2,
        Some(b'\r') => 1,
        _ => return Some(cursor),
    };
    cursor += newline_len;
    let next = skip_spaces(source, cursor);
    if next.saturating_sub(cursor) > 3 {
        None
    } else {
        Some(next)
    }
}

fn skip_optional_title_space(source: &str, cursor: usize) -> Option<usize> {
    let after_spaces = skip_spaces(source, cursor);
    if after_spaces > cursor {
        return Some(after_spaces);
    }
    let bytes = source.as_bytes();
    let newline_len = match bytes.get(cursor) {
        Some(b'\n') => 1,
        Some(b'\r') if bytes.get(cursor + 1) == Some(&b'\n') => 2,
        Some(b'\r') => 1,
        _ => return None,
    };
    let next_line = cursor + newline_len;
    let after_line_spaces = skip_spaces(source, next_line);
    (after_line_spaces.saturating_sub(next_line) <= 3).then_some(after_line_spaces)
}

fn skip_spaces(source: &str, mut cursor: usize) -> usize {
    while matches!(source.as_bytes().get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }
    cursor
}

fn parse_link_destination(source: &str, start: usize) -> Option<(Range<usize>, String, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start) == Some(&b'<') {
        let mut cursor = start + 1;
        while cursor < source.len() {
            match bytes[cursor] {
                b'\\' if next_is_escapable_punctuation(bytes, cursor) => cursor += 2,
                b'\\' => cursor += 1,
                b'>' => {
                    let inner = start + 1..cursor;
                    let destination = unescape_markdown(&source[inner.clone()]);
                    return Some((inner, destination, cursor + 1));
                }
                b'<' => return None,
                b'\n' | b'\r' => return None,
                _ => cursor += 1,
            }
        }
        return None;
    }

    let mut cursor = start;
    let mut depth = 0usize;
    while cursor < source.len() {
        match bytes[cursor] {
            b'\\' if next_is_escapable_punctuation(bytes, cursor) => cursor += 2,
            b'\\' => cursor += 1,
            b'(' => {
                depth += 1;
                cursor += 1;
            }
            b')' if depth > 0 => {
                depth -= 1;
                cursor += 1;
            }
            b' ' | b'\t' | b'\n' | b'\r' => break,
            _ => cursor += 1,
        }
    }

    (cursor > start).then(|| {
        let destination = unescape_markdown(&source[start..cursor]);
        (start..cursor, destination, cursor)
    })
}

fn parse_link_title(source: &str, start: usize) -> Option<(Range<usize>, usize)> {
    let bytes = source.as_bytes();
    let (open, close) = match bytes.get(start)? {
        b'\'' => (b'\'', b'\''),
        b'"' => (b'"', b'"'),
        b'(' => (b'(', b')'),
        _ => return None,
    };

    let mut cursor = start + 1;
    let content_start = cursor;
    while cursor < source.len() {
        match bytes[cursor] {
            b'\\' if next_is_escapable_punctuation(bytes, cursor) => cursor += 2,
            b'\\' => cursor += 1,
            byte if byte == open && open == b'(' => return None,
            byte if byte == close => {
                let inner = content_start..cursor;
                return Some((inner, cursor + 1));
            }
            _ => cursor += 1,
        }
    }
    None
}

fn only_blank_until_line_end(source: &str, mut cursor: usize) -> bool {
    let bytes = source.as_bytes();
    while let Some(byte) = bytes.get(cursor) {
        match *byte {
            b' ' | b'\t' => cursor += 1,
            b'\r' | b'\n' => return true,
            _ => return false,
        }
    }
    true
}

fn next_is_escapable_punctuation(bytes: &[u8], cursor: usize) -> bool {
    bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation)
}

fn unescape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next_if(char::is_ascii_punctuation) {
                out.push(next);
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definitions(source: &str) -> Vec<ReferenceDefinition> {
        reference_definitions_from_source(source)
    }

    fn labels(source: &str) -> Vec<String> {
        definitions(source)
            .into_iter()
            .map(|definition| definition.label)
            .collect()
    }

    #[test]
    fn reference_definition_cannot_interrupt_paragraph() {
        assert!(definitions("Foo\n[bar]: /baz\n").is_empty());
    }

    #[test]
    fn reference_definitions_inside_block_containers_are_recognized() {
        let source = "> [quoted]: /quote\n\n- [listed]: /list\n";

        assert_eq!(labels(source), vec!["quoted", "listed"]);
    }

    #[test]
    fn malformed_reference_definition_with_trailing_text_is_rejected() {
        assert!(definitions("[foo]: /url oops\n").is_empty());
    }

    #[test]
    fn html_blocks_suppress_reference_definition_scanning() {
        let source = "<script>\n[foo]: /url\n</script>\n";

        assert!(definitions(source).is_empty());
    }

    #[test]
    fn container_scoped_fenced_code_suppresses_reference_definitions() {
        let source = "> ```\n> [fake]: /url\n> ```\n";

        assert!(definitions(source).is_empty());
    }

    #[test]
    fn reference_label_rejects_nested_brackets() {
        assert!(definitions("[a[b]]: /url\n").is_empty());
    }

    #[test]
    fn reference_definitions_preserve_non_escapable_backslashes() {
        let source = "[foo\\q]: <https://example.com/a\\q>\n[bar\\]]: /ok\\q\n";
        let definitions = definitions(source);

        assert_eq!(definitions[0].label, "foo\\q");
        assert_eq!(definitions[0].destination, "https://example.com/a\\q");
        assert_eq!(definitions[1].label, "bar]");
        assert_eq!(definitions[1].destination, "/ok\\q");
    }

    #[test]
    fn unescape_markdown_only_unescapes_commonmark_punctuation() {
        assert_eq!(
            unescape_markdown("foo\\] bar\\q baz\\\\"),
            "foo] bar\\q baz\\"
        );
    }

    #[test]
    fn angle_destination_rejects_unescaped_lt() {
        assert!(definitions("[id]: <a<b>\n").is_empty());
    }
}

pub(crate) struct DocumentBuilder<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
    headings: Vec<Heading>,
    links: Vec<LinkUse>,
    reference_definitions: Vec<ReferenceDefinition>,
    footnote_references: Vec<FootnoteReference>,
    footnote_definitions: Vec<FootnoteDefinition>,
    code_blocks: Vec<CodeBlock>,
    heading_stack: Vec<HeadingFrame>,
    link_stack: Vec<LinkFrame>,
    code_block_stack: Vec<CodeBlockFrame>,
}

impl<'a> DocumentBuilder<'a> {
    pub(crate) fn new(source: &'a str, reference_definitions: Vec<ReferenceDefinition>) -> Self {
        Self {
            source,
            line_starts: line_starts(source),
            headings: Vec::new(),
            links: Vec::new(),
            reference_definitions,
            footnote_references: Vec::new(),
            footnote_definitions: Vec::new(),
            code_blocks: Vec::new(),
            heading_stack: Vec::new(),
            link_stack: Vec::new(),
            code_block_stack: Vec::new(),
        }
    }

    pub(crate) fn finish(self) -> MarkdownDocument {
        let code_block_start_lines = self
            .code_blocks
            .iter()
            .enumerate()
            .map(|(index, block)| (block.start_line, index))
            .collect();
        MarkdownDocument {
            headings: self.headings,
            links: self.links,
            reference_definitions: self.reference_definitions,
            footnote_references: self.footnote_references,
            footnote_definitions: self.footnote_definitions,
            code_blocks: self.code_blocks,
            code_block_start_lines,
        }
    }

    pub(crate) fn handle_event(&mut self, event: Event<'a>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start_tag(tag, range),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.push_text_event(&text, range),
            Event::Code(text) => self.push_text(&text),
            Event::InlineMath(text) | Event::DisplayMath(text) => self.push_text(&text),
            Event::FootnoteReference(label) => self.add_footnote_reference(&label, range),
            Event::SoftBreak | Event::HardBreak => self.push_text(" "),
            Event::Html(_) | Event::InlineHtml(_) | Event::Rule | Event::TaskListMarker(_) => {}
        }
    }

    fn start_tag(&mut self, tag: Tag<'a>, range: Range<usize>) {
        match tag {
            Tag::Heading { level, .. } => self.push_heading(level),
            Tag::Link {
                link_type,
                dest_url,
                id,
                ..
            } => self.push_link(link_type, &dest_url, &id, range, false),
            Tag::Image {
                link_type,
                dest_url,
                id,
                ..
            } => self.push_link(link_type, &dest_url, &id, range, true),
            Tag::CodeBlock(kind) => self.push_code_block(kind, range),
            Tag::FootnoteDefinition(label) => self.push_footnote_definition(&label),
            Tag::Paragraph
            | Tag::BlockQuote(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::MetadataBlock(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Superscript
            | Tag::Subscript => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => self.pop_heading(),
            TagEnd::Link | TagEnd::Image => self.pop_link(),
            TagEnd::CodeBlock => self.pop_code_block(),
            TagEnd::Paragraph
            | TagEnd::BlockQuote(_)
            | TagEnd::HtmlBlock
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::FootnoteDefinition
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell
            | TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::MetadataBlock(_)
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::Superscript
            | TagEnd::Subscript => {}
        }
    }

    fn push_heading(&mut self, _level: HeadingLevel) {
        self.heading_stack.push(HeadingFrame {
            text: String::new(),
        });
    }

    fn pop_heading(&mut self) {
        if let Some(frame) = self.heading_stack.pop() {
            let text = frame.text.trim().to_string();
            if !text.is_empty() {
                self.headings.push(Heading { text });
            }
        }
    }

    fn push_link(
        &mut self,
        link_type: LinkType,
        destination: &str,
        reference_id: &str,
        range: Range<usize>,
        is_image: bool,
    ) {
        self.link_stack.push(LinkFrame {
            line: self.line_for(range.start),
            kind: link_type.into(),
            destination: destination.to_string(),
            reference_label: (!reference_id.is_empty())
                .then(|| normalize_reference_label(reference_id)),
            text: String::new(),
            is_image,
        });
    }

    fn pop_link(&mut self) {
        if let Some(frame) = self.link_stack.pop() {
            self.links.push(LinkUse {
                line: frame.line,
                kind: frame.kind,
                destination: frame.destination,
                reference_label: frame.reference_label,
                text: frame.text.trim().to_string(),
                is_image: frame.is_image,
            });
        }
    }

    fn push_footnote_definition(&mut self, label: &str) {
        self.footnote_definitions.push(FootnoteDefinition {
            label: label.to_string(),
        });
    }

    fn add_footnote_reference(&mut self, label: &str, range: Range<usize>) {
        let label = label.to_string();
        self.footnote_references.push(FootnoteReference {
            line: self.line_for(range.start),
            label: label.clone(),
        });
        self.push_text(&format!("[^{label}]"));
    }

    fn push_code_block(&mut self, kind: PulldownCodeBlockKind<'a>, range: Range<usize>) {
        let (kind, language) = match kind {
            PulldownCodeBlockKind::Fenced(info) => (CodeBlockKind::Fenced, code_language(&info)),
            PulldownCodeBlockKind::Indented => (CodeBlockKind::Indented, None),
        };
        self.code_block_stack.push(CodeBlockFrame {
            kind,
            start_line: self.line_for(range.start),
            end_line: self.end_line_for(range.clone()),
            language,
            content: String::new(),
        });
    }

    fn pop_code_block(&mut self) {
        if let Some(frame) = self.code_block_stack.pop() {
            self.code_blocks.push(CodeBlock {
                kind: frame.kind,
                start_line: frame.start_line,
                end_line: frame.end_line,
                language: frame.language,
                content: normalize_line_endings(&frame.content),
            });
        }
    }

    fn push_text_event(&mut self, text: &str, _range: Range<usize>) {
        if let Some(code) = self.code_block_stack.last_mut() {
            code.content.push_str(text);
            return;
        }
        self.push_text(text);
    }

    fn push_text(&mut self, text: &str) {
        if let Some(heading) = self.heading_stack.last_mut() {
            heading.text.push_str(text);
        }
        if let Some(link) = self.link_stack.last_mut() {
            link.text.push_str(text);
        }
    }

    pub(crate) fn line_for(&self, byte: usize) -> u64 {
        row_at(&self.line_starts, self.source.len(), byte) as u64 + 1
    }

    fn end_line_for(&self, range: Range<usize>) -> u64 {
        let start_row = row_at(&self.line_starts, self.source.len(), range.start);
        let mut end_row = row_at(&self.line_starts, self.source.len(), range.end);
        let end_col = range
            .end
            .saturating_sub(*self.line_starts.get(end_row).unwrap_or(&range.end));
        if end_row > start_row && end_col == 0 {
            end_row -= 1;
        }
        end_row as u64 + 1
    }
}

#[derive(Debug)]
struct HeadingFrame {
    text: String,
}

#[derive(Debug)]
struct LinkFrame {
    line: u64,
    kind: LinkUseKind,
    destination: String,
    reference_label: Option<String>,
    text: String,
    is_image: bool,
}

#[derive(Debug)]
struct CodeBlockFrame {
    kind: CodeBlockKind,
    start_line: u64,
    end_line: u64,
    language: Option<String>,
    content: String,
}

impl From<LinkType> for LinkUseKind {
    fn from(value: LinkType) -> Self {
        match value {
            LinkType::Inline => Self::Inline,
            LinkType::Reference => Self::Reference,
            LinkType::ReferenceUnknown => Self::ReferenceUnknown,
            LinkType::Collapsed => Self::Collapsed,
            LinkType::CollapsedUnknown => Self::CollapsedUnknown,
            LinkType::Shortcut => Self::Shortcut,
            LinkType::ShortcutUnknown => Self::ShortcutUnknown,
            LinkType::Autolink => Self::Autolink,
            LinkType::Email => Self::Email,
            LinkType::WikiLink { .. } => Self::WikiLink,
        }
    }
}

impl MarkdownDocument {
    pub(crate) fn code_block_by_start_row(&self, start_row: usize) -> Option<&CodeBlock> {
        self.code_block_start_lines
            .get(&(start_row as u64 + 1))
            .and_then(|index| self.code_blocks.get(*index))
    }

    pub(crate) fn reference_definition_labels(&self) -> HashMap<&str, &ReferenceDefinition> {
        self.reference_definitions
            .iter()
            .map(|definition| (definition.label.as_str(), definition))
            .collect()
    }
}
