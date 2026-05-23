// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Small Markdown syntax tree used internally by the metric passes.
//!
//! The analyzer consumes a compact owned tree built from `pulldown-cmark`
//! events. It exposes only the cursor API, byte spans, row positions, field
//! lookup, and generated `Markdown` kind ids needed by the metric modules.

use std::ops::Range;

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, LinkType, MetadataBlockKind,
    Options, Parser, Tag, TagEnd,
};

use crate::grammar::Markdown;

#[derive(Debug)]
pub(crate) struct Tree {
    nodes: Vec<NodeData>,
}

#[derive(Clone, Debug)]
struct NodeData {
    kind: Markdown,
    start_byte: usize,
    end_byte: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
    children: Vec<usize>,
    fields: Vec<(&'static str, usize)>,
}

struct RefDefData {
    label: String,
    dest: String,
    title: Option<String>,
    span: Range<usize>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Node<'a> {
    tree: &'a Tree,
    index: usize,
}

impl Tree {
    pub(crate) fn root(&self) -> Node<'_> {
        Node {
            tree: self,
            index: 0,
        }
    }
}

impl<'a> Node<'a> {
    fn data(&self) -> &NodeData {
        &self.tree.nodes[self.index]
    }

    pub(crate) fn kind_id(&self) -> u16 {
        self.data().kind as u16
    }

    pub(crate) fn start_byte(&self) -> usize {
        self.data().start_byte
    }

    pub(crate) fn end_byte(&self) -> usize {
        self.data().end_byte
    }

    #[allow(dead_code)]
    pub(crate) fn start_position(&self) -> (usize, usize) {
        (self.data().start_row, self.data().start_col)
    }

    pub(crate) fn end_position(&self) -> (usize, usize) {
        (self.data().end_row, self.data().end_col)
    }

    pub(crate) fn start_row(&self) -> usize {
        self.data().start_row
    }

    pub(crate) fn child_by_field_name(&self, name: &str) -> Option<Node<'_>> {
        self.data()
            .fields
            .iter()
            .find_map(|(field, idx)| (*field == name).then_some(*idx))
            .map(|index| Node {
                tree: self.tree,
                index,
            })
    }

    pub(crate) fn cursor(&self) -> Cursor<'a> {
        Cursor {
            tree: self.tree,
            parent: self.index,
            pos: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Cursor<'a> {
    tree: &'a Tree,
    parent: usize,
    pos: Option<usize>,
}

impl<'a> Cursor<'a> {
    pub(crate) fn goto_next_sibling(&mut self) -> bool {
        let Some(pos) = self.pos else {
            return false;
        };
        let next = pos + 1;
        if next < self.tree.nodes[self.parent].children.len() {
            self.pos = Some(next);
            true
        } else {
            false
        }
    }

    pub(crate) fn goto_first_child(&mut self) -> bool {
        if self.tree.nodes[self.parent].children.is_empty() {
            false
        } else {
            self.pos = Some(0);
            true
        }
    }

    pub(crate) fn node(&self) -> Node<'a> {
        let pos = self.pos.expect("cursor has no current node");
        let index = self.tree.nodes[self.parent].children[pos];
        Node {
            tree: self.tree,
            index,
        }
    }
}

pub(crate) fn parse(source: &str) -> Tree {
    Builder::new(source).parse()
}

struct Builder<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
    nodes: Vec<NodeData>,
    stack: Vec<usize>,
}

impl<'a> Builder<'a> {
    fn new(source: &'a str) -> Self {
        let line_starts = line_starts(source);
        let mut builder = Self {
            source,
            line_starts,
            nodes: Vec::new(),
            stack: Vec::new(),
        };
        let root = builder.new_node(Markdown::Document, 0..source.len());
        builder.stack.push(root);
        builder
    }

    fn parse(mut self) -> Tree {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
            | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
            | Options::ENABLE_MATH
            | Options::ENABLE_GFM
            | Options::ENABLE_WIKILINKS;

        let parser = Parser::new_ext(self.source, options);
        let offset_iter = parser.into_offset_iter();
        let refdefs: Vec<RefDefData> = offset_iter
            .reference_definitions()
            .iter()
            .map(|(label, def)| RefDefData {
                label: label.to_string(),
                dest: def.dest.to_string(),
                title: def.title.as_ref().map(ToString::to_string),
                span: def.span.clone(),
            })
            .collect();

        for (event, range) in offset_iter {
            self.handle_event(event, range);
        }

        self.add_reference_definitions(refdefs);
        self.recompute_empty_spans();
        self.wrap_sections();
        self.recompute_all_spans();

        Tree { nodes: self.nodes }
    }

    fn handle_event(&mut self, event: Event<'a>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start_tag(tag, range),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(_) => self.add_text(range),
            Event::Code(_) => self.add_inline_code(range),
            Event::InlineMath(_) => self.add_math_inline(range),
            Event::DisplayMath(_) => self.add_math_block(range),
            Event::Html(_) => self.add_html(range, false),
            Event::InlineHtml(_) => self.add_html(range, true),
            Event::FootnoteReference(label) => self.add_footnote_reference(&label, range),
            Event::SoftBreak | Event::HardBreak => {
                self.add_child(Markdown::Newline, range);
            }
            Event::Rule => {
                self.add_child(Markdown::ThematicBreak, range);
            }
            Event::TaskListMarker(checked) => self.add_task_marker(checked, range),
        }
    }

    fn start_tag(&mut self, tag: Tag<'a>, range: Range<usize>) {
        match tag {
            Tag::Paragraph => self.push(Markdown::Paragraph, range),
            Tag::Heading { level, .. } => self.push_heading(level, range),
            Tag::BlockQuote(kind) => self.push_blockquote(kind, range),
            Tag::CodeBlock(CodeBlockKind::Fenced(info)) => self.push_fenced_code(&info, range),
            Tag::CodeBlock(CodeBlockKind::Indented) => {
                self.push(Markdown::IndentedCodeBlock, range)
            }
            Tag::HtmlBlock => self.push(Markdown::HtmlBlock, range),
            Tag::List(start) => self.push_list(start, range),
            Tag::Item => self.push_list_item(range),
            Tag::FootnoteDefinition(label) => self.push_footnote_definition(&label, range),
            Tag::Table(alignments) => self.push_table(alignments, range),
            Tag::TableHead => self.push(Markdown::PipeTableHeader, range),
            Tag::TableRow => self.push(Markdown::PipeTableRow, range),
            Tag::TableCell => self.push(Markdown::PipeTableCell, range),
            Tag::Emphasis => self.push(Markdown::Emphasis, range),
            Tag::Strong => self.push(Markdown::Strong, range),
            Tag::Strikethrough => self.push(Markdown::Strikethrough, range),
            Tag::Superscript | Tag::Subscript => self.push(Markdown::Emphasis, range),
            Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            } => self.push_link(link_type, &dest_url, &title, range, false),
            Tag::Image {
                link_type,
                dest_url,
                title,
                ..
            } => self.push_link(link_type, &dest_url, &title, range, true),
            Tag::MetadataBlock(kind) => {
                let kind = match kind {
                    MetadataBlockKind::YamlStyle => Markdown::MinusMetadata,
                    MetadataBlockKind::PlusesStyle => Markdown::PlusMetadata,
                };
                self.push(kind, range);
            }
            Tag::DefinitionList => self.push(Markdown::List, range),
            Tag::DefinitionListTitle | Tag::DefinitionListDefinition => {
                self.push(Markdown::ListItem, range)
            }
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.pop_if(Markdown::AtxHeadingContent);
                self.pop_one_of(&[
                    Markdown::AtxHeading,
                    Markdown::AtxHeading2,
                    Markdown::AtxHeading3,
                    Markdown::AtxHeading4,
                    Markdown::AtxHeading5,
                    Markdown::AtxHeading6,
                    Markdown::SetextHeading,
                    Markdown::SetextHeading2,
                ]);
            }
            TagEnd::Item => {
                self.pop_one_of(&[Markdown::ListItemContent, Markdown::TaskListItemContent]);
                self.pop_one_of(&[
                    Markdown::ListItem,
                    Markdown::ListItem2,
                    Markdown::ListItem3,
                    Markdown::ListItem4,
                    Markdown::ListItem5,
                    Markdown::TaskListItem,
                    Markdown::TaskListItem2,
                    Markdown::TaskListItem3,
                    Markdown::TaskListItem4,
                    Markdown::TaskListItem5,
                ]);
            }
            TagEnd::Link | TagEnd::Image => {
                self.pop_if(Markdown::LinkLabel);
                self.pop_one_of(&[Markdown::Link, Markdown::Image, Markdown::Autolink]);
            }
            TagEnd::Paragraph => self.pop_if(Markdown::Paragraph),
            TagEnd::BlockQuote(_) => self.pop_one_of(&[
                Markdown::BlockQuote,
                Markdown::PlainBlockQuote,
                Markdown::Callout,
            ]),
            TagEnd::CodeBlock => {
                self.pop_one_of(&[Markdown::FencedCodeBlock, Markdown::IndentedCodeBlock]);
            }
            TagEnd::HtmlBlock => self.pop_if(Markdown::HtmlBlock),
            TagEnd::List(_) => self.pop_if(Markdown::List),
            TagEnd::FootnoteDefinition => self.pop_if(Markdown::FootnoteDefinition),
            TagEnd::Table => self.pop_if(Markdown::PipeTable),
            TagEnd::TableHead => self.pop_if(Markdown::PipeTableHeader),
            TagEnd::TableRow => self.pop_if(Markdown::PipeTableRow),
            TagEnd::TableCell => self.pop_if(Markdown::PipeTableCell),
            TagEnd::Emphasis | TagEnd::Superscript | TagEnd::Subscript => {
                self.pop_if(Markdown::Emphasis)
            }
            TagEnd::Strong => self.pop_if(Markdown::Strong),
            TagEnd::Strikethrough => self.pop_if(Markdown::Strikethrough),
            TagEnd::MetadataBlock(kind) => {
                let kind = match kind {
                    MetadataBlockKind::YamlStyle => Markdown::MinusMetadata,
                    MetadataBlockKind::PlusesStyle => Markdown::PlusMetadata,
                };
                self.pop_if(kind);
            }
            TagEnd::DefinitionList => self.pop_if(Markdown::List),
            TagEnd::DefinitionListTitle | TagEnd::DefinitionListDefinition => {
                self.pop_if(Markdown::ListItem)
            }
        }
    }

    fn push_heading(&mut self, level: HeadingLevel, range: Range<usize>) {
        let (heading_kind, marker_kind, setext) = heading_kinds(level, self.source, &range);
        let heading = self.add_child(heading_kind, range.clone());
        if let Some(marker_range) = heading_marker_range(self.source, &range, setext) {
            let marker = self.add_child_to(heading, marker_kind, marker_range);
            self.nodes[heading].fields.push(("level", marker));
        }
        let content =
            self.add_child_to(heading, Markdown::AtxHeadingContent, empty_at(range.start));
        self.nodes[heading]
            .fields
            .push(("heading_content", content));
        self.stack.push(heading);
        self.stack.push(content);
    }

    fn push_blockquote(&mut self, kind: Option<BlockQuoteKind>, range: Range<usize>) {
        let node_kind = if kind.is_some() {
            Markdown::Callout
        } else {
            Markdown::BlockQuote
        };
        let node = self.add_child(node_kind, range.clone());
        self.add_child_to(node, Markdown::BlockQuoteMarker, first_byte(range.start));
        if let Some(kind) = kind {
            let marker =
                callout_type_range(self.source, &range).unwrap_or_else(|| first_byte(range.start));
            self.add_child_to(
                node,
                Markdown::CalloutMarkerOpen,
                marker.start..marker.start + 1,
            );
            self.add_child_to(node, Markdown::CalloutType, marker);
            let close = range.start.saturating_add(1).min(range.end);
            self.add_child_to(node, Markdown::CalloutMarkerClose, close..close);
            let _ = kind;
        }
        self.stack.push(node);
    }

    fn push_fenced_code(&mut self, info: &str, range: Range<usize>) {
        let node = self.add_child(Markdown::FencedCodeBlock, range.clone());
        if !info.trim().is_empty() {
            let info_range = find_in_range(self.source, &range, info).unwrap_or_else(|| {
                let start = range.start.min(range.end);
                start..start
            });
            let info_node = self.add_child_to(node, Markdown::InfoString, info_range.clone());
            let lang_end = info
                .find(|c: char| c.is_whitespace() || c == ',' || c == '{')
                .unwrap_or(info.len());
            let lang = &info[..lang_end];
            if !lang.is_empty() {
                let lang_range =
                    find_in_range(self.source, &info_range, lang).unwrap_or(info_range);
                self.add_child_to(info_node, Markdown::Language, lang_range);
            }
        }
        self.stack.push(node);
    }

    fn push_list(&mut self, start: Option<u64>, range: Range<usize>) {
        let node = self.add_child(Markdown::List, range.clone());
        let _ = start;
        self.stack.push(node);
    }

    fn push_list_item(&mut self, range: Range<usize>) {
        let item = self.add_child(Markdown::ListItem, range.clone());
        let marker_kind =
            list_item_marker_kind(self.source, &range).unwrap_or(Markdown::ListMarkerMinus);
        self.add_child_to(
            item,
            marker_kind,
            list_item_marker_range(self.source, &range),
        );
        let content = self.add_child_to(item, Markdown::ListItemContent, empty_at(range.start));
        self.stack.push(item);
        self.stack.push(content);
    }

    fn push_footnote_definition(&mut self, label: &str, range: Range<usize>) {
        let node = self.add_child(Markdown::FootnoteDefinition, range.clone());
        if let Some(label_range) = find_footnote_label_range(self.source, &range, label) {
            self.add_child_to(node, Markdown::FootnoteLabel, label_range);
        }
        self.stack.push(node);
    }

    fn push_table(&mut self, alignments: Vec<Alignment>, range: Range<usize>) {
        let table = self.add_child(Markdown::PipeTable, range.clone());
        let delim = self.add_child_to(
            table,
            Markdown::PipeTableDelimiterRow,
            empty_at(range.start),
        );
        for align in alignments {
            let cell = self.add_child_to(
                delim,
                Markdown::PipeTableDelimiterCell,
                empty_at(range.start),
            );
            match align {
                Alignment::Left => {
                    self.add_child_to(cell, Markdown::PipeTableAlignLeft, empty_at(range.start));
                }
                Alignment::Right => {
                    self.add_child_to(cell, Markdown::PipeTableAlignRight, empty_at(range.start));
                }
                Alignment::Center => {
                    self.add_child_to(cell, Markdown::PipeTableAlignLeft, empty_at(range.start));
                    self.add_child_to(cell, Markdown::PipeTableAlignRight, empty_at(range.start));
                }
                Alignment::None => {}
            }
        }
        self.stack.push(table);
    }

    fn push_link(
        &mut self,
        link_type: LinkType,
        dest_url: &str,
        title: &str,
        range: Range<usize>,
        image: bool,
    ) {
        if !image && matches!(link_type, LinkType::Autolink | LinkType::Email) {
            let node = self.add_child(Markdown::Autolink, range.clone());
            let kind = if matches!(link_type, LinkType::Email) {
                Markdown::Email
            } else {
                Markdown::Uri
            };
            let dest_range = visible_autolink_range(self.source, &range, dest_url)
                .or_else(|| find_in_range(self.source, &range, dest_url))
                .unwrap_or_else(|| range.clone());
            self.add_child_to(node, kind, dest_range);
            self.stack.push(node);
            return;
        }

        let node = self.add_child(
            if image {
                Markdown::Image
            } else {
                Markdown::Link
            },
            range.clone(),
        );
        if matches!(
            link_type,
            LinkType::Inline
                | LinkType::ReferenceUnknown
                | LinkType::CollapsedUnknown
                | LinkType::ShortcutUnknown
                | LinkType::WikiLink { .. }
        ) && !dest_url.is_empty()
            && let Some(dest_range) = find_in_range(self.source, &range, dest_url)
        {
            self.add_child_to(node, Markdown::LinkDestination, dest_range);
        }
        if !title.is_empty()
            && let Some(title_range) = find_in_range(self.source, &range, title)
        {
            self.add_child_to(node, Markdown::LinkTitle, title_range);
        }
        let label = self.add_child_to(node, Markdown::LinkLabel, empty_at(range.start));
        self.stack.push(node);
        self.stack.push(label);
    }

    fn add_text(&mut self, range: Range<usize>) {
        if range.start >= range.end {
            return;
        }
        let parent = self.current();
        let parent_kind = self.nodes[parent].kind;
        if matches!(parent_kind, Markdown::FencedCodeBlock) {
            self.add_child_to(parent, Markdown::CodeFenceContent, range);
            return;
        }
        if matches!(parent_kind, Markdown::IndentedCodeBlock) {
            self.add_child_to(parent, Markdown::IndentedChunk, range);
            return;
        }
        self.tokenize_text(range);
    }

    fn add_inline_code(&mut self, range: Range<usize>) {
        let node = self.add_child(Markdown::InlineCode, range.clone());
        self.add_child_to(node, Markdown::InlineCodeContent, range);
    }

    fn add_math_inline(&mut self, range: Range<usize>) {
        let node = self.add_child(Markdown::MathInline, range.clone());
        self.add_child_to(node, Markdown::MathInlineContent, range.clone());
        self.tokenize_text_into(node, range);
    }

    fn add_math_block(&mut self, range: Range<usize>) {
        let node = self.add_child(Markdown::MathBlock, range.clone());
        self.add_child_to(node, Markdown::MathBlockDelimiter, first_byte(range.start));
        self.add_child_to(node, Markdown::MathBlockContent, range.clone());
        self.tokenize_text_into(node, range);
    }

    fn add_html(&mut self, range: Range<usize>, inline: bool) {
        let text = self.source.get(range.clone()).unwrap_or("");
        if text.trim().is_empty() {
            return;
        }
        let parent = self.current();
        let node = if inline || !matches!(self.nodes[parent].kind, Markdown::HtmlBlock) {
            self.add_child(
                if inline {
                    Markdown::HtmlInline
                } else {
                    Markdown::HtmlBlock
                },
                range.clone(),
            )
        } else {
            parent
        };
        let kind = classify_html(text);
        self.add_child_to(node, kind, range);
    }

    fn add_footnote_reference(&mut self, label: &str, range: Range<usize>) {
        let node = self.add_child(Markdown::FootnoteReference, range.clone());
        if let Some(label_range) = find_footnote_label_range(self.source, &range, label) {
            self.add_child_to(node, Markdown::FootnoteReferenceLabel, label_range);
        }
    }

    fn add_task_marker(&mut self, checked: bool, range: Range<usize>) {
        let marker = if checked {
            Markdown::TaskListMarkerChecked
        } else {
            Markdown::TaskListMarkerUnchecked
        };
        self.add_child(marker, range);
        for &idx in self.stack.iter().rev() {
            match self.nodes[idx].kind {
                Markdown::ListItem => {
                    self.nodes[idx].kind = Markdown::TaskListItem;
                    break;
                }
                Markdown::ListItemContent => {
                    self.nodes[idx].kind = Markdown::TaskListItemContent;
                }
                _ => {}
            }
        }
    }

    fn tokenize_text(&mut self, range: Range<usize>) {
        let parent = self.current();
        self.tokenize_text_into(parent, range);
    }

    fn tokenize_text_into(&mut self, parent: usize, range: Range<usize>) {
        let Some(text) = self.source.get(range.clone()) else {
            return;
        };
        let chars: Vec<_> = text.char_indices().collect();
        let mut token_start: Option<usize> = None;
        for (idx, (offset, ch)) in chars.iter().copied().enumerate() {
            let abs = range.start + offset;
            let prev = idx
                .checked_sub(1)
                .and_then(|prev_idx| chars.get(prev_idx))
                .map(|(_, ch)| *ch);
            let next = chars.get(idx + 1).map(|(_, ch)| *ch);
            if is_token_char(ch, prev, next) {
                token_start.get_or_insert(abs);
                continue;
            }
            if let Some(start) = token_start.take() {
                self.add_wordish_token(parent, start..abs);
            }
            if !ch.is_whitespace() {
                let end = abs + ch.len_utf8();
                if let Some(kind) = punctuation_kind(ch) {
                    self.add_child_to(parent, kind, abs..end);
                }
            }
        }
        if let Some(start) = token_start {
            self.add_wordish_token(parent, start..range.end);
        }
    }

    fn add_wordish_token(&mut self, parent: usize, range: Range<usize>) {
        let text = self.source.get(range.clone()).unwrap_or("");
        let kind = classify_wordish(text);
        self.add_child_to(parent, kind, range);
    }

    fn add_reference_definitions(&mut self, refdefs: Vec<RefDefData>) {
        for def in refdefs {
            let node = self.add_child_to(0, Markdown::LinkReferenceDefinition, def.span.clone());
            if let Some(label_range) =
                find_label_definition_range(self.source, &def.span, &def.label)
            {
                self.add_child_to(node, Markdown::LinkLabel, label_range);
            }
            if !def.dest.is_empty()
                && let Some(dest_range) = find_in_range(self.source, &def.span, &def.dest)
            {
                self.add_child_to(node, Markdown::LinkDestination, dest_range);
            }
            if let Some(title) = def.title.as_ref()
                && let Some(title_range) = find_in_range(self.source, &def.span, title)
            {
                self.add_child_to(node, Markdown::LinkTitle, title_range);
            }
        }
    }

    fn wrap_sections(&mut self) {
        let mut top = self.nodes[0].children.clone();
        top.sort_by_key(|idx| (self.nodes[*idx].start_byte, self.nodes[*idx].end_byte));
        self.nodes[0].children.clear();

        let mut section_stack: Vec<(u8, usize)> = Vec::new();
        for child in top {
            if let Some(level) = heading_level_kind(self.nodes[child].kind) {
                while section_stack
                    .last()
                    .map(|(stack_level, _)| *stack_level >= level)
                    .unwrap_or(false)
                {
                    section_stack.pop();
                }
                let section_kind = section_kind(level);
                let section = self.new_node(
                    section_kind,
                    self.nodes[child].start_byte..self.nodes[child].end_byte,
                );
                self.nodes[section].children.push(child);
                if let Some((_, parent)) = section_stack.last().copied() {
                    self.nodes[parent].children.push(section);
                } else {
                    self.nodes[0].children.push(section);
                }
                section_stack.push((level, section));
            } else if let Some((_, section)) = section_stack.last().copied() {
                self.nodes[section].children.push(child);
            } else {
                self.nodes[0].children.push(child);
            }
        }
    }

    fn recompute_empty_spans(&mut self) {
        for idx in 0..self.nodes.len() {
            if self.nodes[idx].start_byte == self.nodes[idx].end_byte {
                self.refresh_span_from_children(idx);
            }
        }
    }

    fn recompute_all_spans(&mut self) {
        self.recompute_span_rec(0);
    }

    fn recompute_span_rec(&mut self, idx: usize) -> Option<Range<usize>> {
        let children = self.nodes[idx].children.clone();
        let mut start = self.nodes[idx].start_byte;
        let mut end = self.nodes[idx].end_byte;
        for child in children {
            if let Some(child_range) = self.recompute_span_rec(child) {
                start = start.min(child_range.start);
                end = end.max(child_range.end);
            }
        }
        if !self.nodes[idx].children.is_empty()
            && matches!(
                self.nodes[idx].kind,
                Markdown::Section
                    | Markdown::Section1
                    | Markdown::Section2
                    | Markdown::Section3
                    | Markdown::Section4
                    | Markdown::Section5
                    | Markdown::Section6
                    | Markdown::AtxHeadingContent
                    | Markdown::LinkLabel
                    | Markdown::ListItemContent
                    | Markdown::TaskListItemContent
                    | Markdown::PipeTableDelimiterRow
                    | Markdown::PipeTableDelimiterCell
            )
        {
            self.set_range(idx, start..end);
        }
        Some(self.nodes[idx].start_byte..self.nodes[idx].end_byte)
    }

    fn refresh_span_from_children(&mut self, idx: usize) {
        let children = self.nodes[idx].children.clone();
        let Some(first) = children.first().copied() else {
            return;
        };
        let mut start = self.nodes[first].start_byte;
        let mut end = self.nodes[first].end_byte;
        for child in children.iter().copied().skip(1) {
            start = start.min(self.nodes[child].start_byte);
            end = end.max(self.nodes[child].end_byte);
        }
        self.set_range(idx, start..end);
    }

    fn push(&mut self, kind: Markdown, range: Range<usize>) {
        let node = self.add_child(kind, range);
        self.stack.push(node);
    }

    fn add_child(&mut self, kind: Markdown, range: Range<usize>) -> usize {
        let parent = self.current();
        self.add_child_to(parent, kind, range)
    }

    fn add_child_to(&mut self, parent: usize, kind: Markdown, range: Range<usize>) -> usize {
        let node = self.new_node(kind, range);
        self.nodes[parent].children.push(node);
        node
    }

    fn new_node(&mut self, kind: Markdown, range: Range<usize>) -> usize {
        let range = clamp_range(range, self.source.len());
        let (start_row, start_col) = self.position(range.start);
        let (end_row, end_col) = self.position(range.end);
        let idx = self.nodes.len();
        self.nodes.push(NodeData {
            kind,
            start_byte: range.start,
            end_byte: range.end,
            start_row,
            start_col,
            end_row,
            end_col,
            children: Vec::new(),
            fields: Vec::new(),
        });
        idx
    }

    fn set_range(&mut self, idx: usize, range: Range<usize>) {
        let range = clamp_range(range, self.source.len());
        let (start_row, start_col) = self.position(range.start);
        let (end_row, end_col) = self.position(range.end);
        let node = &mut self.nodes[idx];
        node.start_byte = range.start;
        node.end_byte = range.end;
        node.start_row = start_row;
        node.start_col = start_col;
        node.end_row = end_row;
        node.end_col = end_col;
    }

    fn current(&self) -> usize {
        *self.stack.last().expect("builder stack is empty")
    }

    fn pop_if(&mut self, kind: Markdown) {
        if self.stack.last().map(|idx| self.nodes[*idx].kind) == Some(kind) {
            self.stack.pop();
        }
    }

    fn pop_one_of(&mut self, kinds: &[Markdown]) {
        while self.stack.len() > 1 {
            let idx = *self.stack.last().unwrap();
            if kinds.contains(&self.nodes[idx].kind) {
                self.stack.pop();
                return;
            }
            self.stack.pop();
        }
    }

    fn position(&self, byte: usize) -> (usize, usize) {
        let byte = byte.min(self.source.len());
        let row = match self.line_starts.binary_search(&byte) {
            Ok(row) => row,
            Err(0) => 0,
            Err(row) => row - 1,
        };
        (row, byte.saturating_sub(self.line_starts[row]))
    }
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut out = vec![0];
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            out.push(idx + 1);
        }
    }
    out
}

fn clamp_range(range: Range<usize>, len: usize) -> Range<usize> {
    let start = range.start.min(len);
    let end = range.end.min(len).max(start);
    start..end
}

fn empty_at(byte: usize) -> Range<usize> {
    byte..byte
}

fn first_byte(byte: usize) -> Range<usize> {
    byte..byte.saturating_add(1)
}

fn heading_kinds(
    level: HeadingLevel,
    source: &str,
    range: &Range<usize>,
) -> (Markdown, Markdown, bool) {
    let setext = is_setext_heading(source, range);
    match (level, setext) {
        (HeadingLevel::H1, false) => (Markdown::AtxHeading, Markdown::AtxH1Marker, false),
        (HeadingLevel::H2, false) => (Markdown::AtxHeading2, Markdown::AtxH2Marker, false),
        (HeadingLevel::H3, false) => (Markdown::AtxHeading3, Markdown::AtxH3Marker, false),
        (HeadingLevel::H4, false) => (Markdown::AtxHeading4, Markdown::AtxH4Marker, false),
        (HeadingLevel::H5, false) => (Markdown::AtxHeading5, Markdown::AtxH5Marker, false),
        (HeadingLevel::H6, false) => (Markdown::AtxHeading6, Markdown::AtxH6Marker, false),
        (HeadingLevel::H1, true) => (Markdown::SetextHeading, Markdown::SetextH1Underline, true),
        (HeadingLevel::H2, true) => (Markdown::SetextHeading2, Markdown::SetextH2Underline, true),
        (_, true) => (Markdown::AtxHeading, Markdown::AtxH1Marker, false),
    }
}

fn is_setext_heading(source: &str, range: &Range<usize>) -> bool {
    let Some(slice) = source.get(range.clone()) else {
        return false;
    };
    let mut non_empty = slice.lines().filter(|line| !line.trim().is_empty());
    let _first = non_empty.next();
    let Some(second) = non_empty.next() else {
        return false;
    };
    let trimmed = second.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '=' || c == '-')
}

fn heading_marker_range(source: &str, range: &Range<usize>, setext: bool) -> Option<Range<usize>> {
    let slice = source.get(range.clone())?;
    if setext {
        let mut offset = range.start;
        for line in slice.split_inclusive('\n') {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.chars().all(|c| c == '=' || c == '-') {
                let ws = line.len() - line.trim_start().len();
                let len = trimmed.len();
                return Some(offset + ws..offset + ws + len);
            }
            offset += line.len();
        }
        return None;
    }
    let line = slice.lines().next().unwrap_or(slice);
    let leading = line.len() - line.trim_start().len();
    let hashes = line[leading..].bytes().take_while(|b| *b == b'#').count();
    (hashes > 0).then_some(range.start + leading..range.start + leading + hashes)
}

fn heading_level_kind(kind: Markdown) -> Option<u8> {
    Some(match kind {
        Markdown::AtxHeading | Markdown::SetextHeading => 1,
        Markdown::AtxHeading2 | Markdown::SetextHeading2 => 2,
        Markdown::AtxHeading3 => 3,
        Markdown::AtxHeading4 => 4,
        Markdown::AtxHeading5 => 5,
        Markdown::AtxHeading6 => 6,
        _ => return None,
    })
}

fn section_kind(level: u8) -> Markdown {
    match level {
        1 => Markdown::Section1,
        2 => Markdown::Section2,
        3 => Markdown::Section3,
        4 => Markdown::Section4,
        5 => Markdown::Section5,
        6 => Markdown::Section6,
        _ => Markdown::Section,
    }
}

fn list_item_marker_kind(source: &str, range: &Range<usize>) -> Option<Markdown> {
    let line = source.get(range.clone())?.lines().next().unwrap_or("");
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker.is_ascii_digit() {
        return Some(if trimmed.contains(')') {
            Markdown::ListMarkerParenthesis
        } else {
            Markdown::ListMarkerDot
        });
    }
    Some(match marker {
        '+' => Markdown::ListMarkerPlus,
        '*' => Markdown::ListMarkerStar,
        '-' => Markdown::ListMarkerMinus,
        _ => return None,
    })
}

fn list_item_marker_range(source: &str, range: &Range<usize>) -> Range<usize> {
    let Some(line) = source
        .get(range.clone())
        .and_then(|slice| slice.lines().next())
    else {
        return first_byte(range.start);
    };
    let leading = line.len() - line.trim_start().len();
    let trimmed = &line[leading..];
    let len = if trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        trimmed
            .find(|ch| ['.', ')'].contains(&ch))
            .map(|idx| idx + 1)
            .unwrap_or(1)
    } else {
        1
    };
    range.start + leading..range.start + leading + len
}

fn callout_type_range(source: &str, range: &Range<usize>) -> Option<Range<usize>> {
    let slice = source.get(range.clone())?;
    let local = slice.find("[!")?;
    let start = range.start + local + 2;
    let end = source[start..].find(']').map(|n| start + n)?;
    Some(start..end)
}

fn visible_autolink_range(source: &str, range: &Range<usize>, dest: &str) -> Option<Range<usize>> {
    let slice = source.get(range.clone())?;
    let inner = slice.trim().trim_start_matches('<').trim_end_matches('>');
    if inner.is_empty() {
        return None;
    }
    find_in_range(source, range, inner).or_else(|| find_in_range(source, range, dest))
}

fn find_footnote_label_range(
    source: &str,
    range: &Range<usize>,
    label: &str,
) -> Option<Range<usize>> {
    find_in_range(source, range, &format!("[^{label}]"))
}

fn find_label_definition_range(
    source: &str,
    range: &Range<usize>,
    label: &str,
) -> Option<Range<usize>> {
    find_in_range(source, range, &format!("[{label}]")).or_else(|| {
        let slice = source.get(range.clone())?;
        let open = slice.find('[')?;
        let close = slice[open..].find(']')?;
        Some(range.start + open..range.start + open + close + 1)
    })
}

fn find_in_range(source: &str, range: &Range<usize>, needle: &str) -> Option<Range<usize>> {
    if needle.is_empty() {
        return None;
    }
    let slice = source.get(range.clone())?;
    let local = slice.find(needle)?;
    Some(range.start + local..range.start + local + needle.len())
}

fn classify_html(text: &str) -> Markdown {
    let trimmed = text.trim_start();
    if trimmed.starts_with("<!--") {
        Markdown::HtmlComment
    } else if trimmed.starts_with("<![CDATA[") {
        Markdown::HtmlCdata
    } else if trimmed.starts_with("<?") {
        Markdown::HtmlProcessingInstruction
    } else if trimmed.starts_with("<!") {
        Markdown::HtmlDeclaration
    } else if trimmed.starts_with("</") {
        Markdown::HtmlCloseTag
    } else {
        Markdown::HtmlOpenTag
    }
}

fn is_token_char(ch: char, prev: Option<char>, next: Option<char>) -> bool {
    if ch.is_alphanumeric() || ch == '_' {
        return true;
    }
    let prev_word = prev.is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'));
    let next_word = next.is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'));
    match ch {
        '-' => prev_word && next_word,
        '.' => {
            (prev_word && next_word)
                || matches!(next, Some('/' | '.'))
                || matches!(prev, Some('.')) && next_word
        }
        '/' | '\\' => prev_word || next_word || matches!(prev, Some(':' | '/' | '\\')),
        ':' => {
            prev_word
                && (matches!(next, Some('/' | ':')) || next.is_some_and(|c| c.is_alphanumeric()))
        }
        '@' => prev_word && next_word,
        _ => false,
    }
}

fn classify_wordish(text: &str) -> Markdown {
    let trimmed = text.trim_matches(|c: char| c == '-' || c == '_' || c == '.');
    if trimmed.is_empty() {
        return Markdown::WordToken;
    }
    if is_numeric_like(trimmed) {
        Markdown::NumericToken
    } else if is_path_like(trimmed) {
        Markdown::PathLikeToken
    } else if is_identifier_like(trimmed) {
        Markdown::IdentifierLikeToken
    } else {
        Markdown::WordToken
    }
}

fn is_numeric_like(text: &str) -> bool {
    let s = text
        .strip_prefix('v')
        .or(text.strip_prefix('V'))
        .unwrap_or(text);
    let mut has_digit = false;
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            has_digit = true;
        } else if ch != '.' && ch != '_' && ch != '-' {
            return false;
        }
    }
    has_digit
}

fn is_path_like(text: &str) -> bool {
    text.contains('/')
        || text.contains('\\')
        || text.starts_with("./")
        || text.starts_with("../")
        || text
            .rsplit_once('.')
            .map(|(_, ext)| ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
            .unwrap_or(false)
}

fn is_identifier_like(text: &str) -> bool {
    text.contains('_')
        || text.contains("::")
        || text.contains('@')
        || text.chars().any(|c| c.is_ascii_digit())
        || has_camel_hump(text)
}

fn has_camel_hump(text: &str) -> bool {
    let mut prev_lower = false;
    for ch in text.chars() {
        if prev_lower && ch.is_ascii_uppercase() {
            return true;
        }
        prev_lower = ch.is_ascii_lowercase();
    }
    false
}

fn punctuation_kind(ch: char) -> Option<Markdown> {
    Some(match ch {
        '.' | '?' | '!' | '。' | '…' => Markdown::Terminator,
        ',' | ';' | ':' => Markdown::Separator,
        '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' => Markdown::Bracket,
        '=' | '+' | '-' | '*' | '/' | '|' | '&' | '^' | '%' | '~' => Markdown::OperatorLike,
        _ => return None,
    })
}
