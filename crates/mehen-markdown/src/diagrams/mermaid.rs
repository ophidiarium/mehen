//! Line-based Mermaid parser covering `graph`, `flowchart`, `stateDiagram`,
//! and `sequenceDiagram` shapes. This is intentionally narrow: §12.2 only
//! needs node/edge/connected-component/cycle counts, not semantic fidelity.
//!
//! Supported shapes (sufficient for §12.2):
//!
//! - `graph <dir>` / `flowchart <dir>` — ASCII DAGs.
//! - `stateDiagram-v2` — states and transitions.
//! - `sequenceDiagram` — participants and interactions.
//! - Basic `classDiagram`, `erDiagram`, `journey`, `gantt` are treated as
//!   unknown subgraphs but still parse cleanly.

use std::collections::BTreeSet;

use super::DiagramSignal;

pub fn parse(body: &str) -> DiagramSignal {
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut has_title = false;
    let mut saw_header = false;
    let mut kind = DiagramKind::Unknown;

    for raw in body.lines() {
        let line = strip_mermaid_comment(raw);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Title lines (`title My Diagram`) anywhere count as labels.
        if trimmed.starts_with("title ") || trimmed.starts_with("title:") {
            has_title = true;
            continue;
        }
        if trimmed.starts_with("%%") {
            continue;
        }

        if !saw_header && let Some(k) = detect_header(trimmed) {
            kind = k;
            saw_header = true;
            continue;
        }

        match kind {
            DiagramKind::Graph => parse_graph_line(trimmed, &mut nodes, &mut edges),
            DiagramKind::State => parse_state_line(trimmed, &mut nodes, &mut edges),
            DiagramKind::Sequence => parse_sequence_line(trimmed, &mut nodes, &mut edges),
            DiagramKind::ClassOrEr => parse_class_line(trimmed, &mut nodes, &mut edges),
            DiagramKind::Unknown => {
                // Fallback: if there's a `-->` / `->` edge on the line, still
                // pick it up so generic mermaid flavors don't silently zero.
                parse_graph_line(trimmed, &mut nodes, &mut edges);
            }
        }
    }

    // Cycles per §12.2: max(0, E - N + P)
    let components = super::connected_components(&nodes, &edges);
    let n = nodes.len() as i64;
    let e = edges.len() as i64;
    let p = components as i64;
    let cycles_i = e - n + p;
    let cycles = if cycles_i > 0 { cycles_i as u64 } else { 0 };

    DiagramSignal {
        nodes: nodes.len() as u64,
        edges: edges.len() as u64,
        components,
        cycles,
        parse_error: false,
        has_title,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagramKind {
    Graph,
    State,
    Sequence,
    ClassOrEr,
    Unknown,
}

fn detect_header(s: &str) -> Option<DiagramKind> {
    let first = s.split_whitespace().next().unwrap_or("");
    match first {
        "graph" | "flowchart" => Some(DiagramKind::Graph),
        "stateDiagram" | "stateDiagram-v2" => Some(DiagramKind::State),
        "sequenceDiagram" => Some(DiagramKind::Sequence),
        "classDiagram" | "erDiagram" => Some(DiagramKind::ClassOrEr),
        _ => None,
    }
}

fn strip_mermaid_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn parse_graph_line(line: &str, nodes: &mut BTreeSet<String>, edges: &mut Vec<(String, String)>) {
    // Split the line on any recognized mermaid edge terminator. We walk from
    // left to right, extracting a node reference, then consuming the edge
    // tokens (with optional `|label|` or `-- text --` body) that follow.
    let mut rest = line.trim();
    let Some((first_id, after_first)) = extract_node_ref(rest) else {
        return;
    };
    rest = after_first.trim();
    let mut last = first_id;
    nodes.insert(last.clone());

    while let Some(remainder) = consume_edge(rest) {
        let Some((next_id, after_next)) = extract_node_ref(remainder.trim_start()) else {
            break;
        };
        nodes.insert(next_id.clone());
        edges.push((last.clone(), next_id.clone()));
        last = next_id;
        rest = after_next.trim();
    }
}

/// Consumes the edge tokens starting at `input` — including optional
/// `|label|` or `-- text --` annotations — and returns the remainder of
/// the string after the trailing arrow. Returns `None` if `input` does
/// not begin with an edge.
fn consume_edge(input: &str) -> Option<&str> {
    let s = input.trim_start();
    if s.is_empty() {
        return None;
    }
    let first = s.as_bytes()[0];
    if !matches!(first, b'-' | b'=' | b'<' | b'~' | b'.') {
        return None;
    }
    // Scan through the edge prefix: any run of `-`, `=`, `<`, `>`, `.`, `~`.
    // Then optionally skip `|…|` OR `identifier text -->` segment.
    let mut idx = 0;
    while idx < s.len() && matches!(s.as_bytes()[idx], b'-' | b'=' | b'<' | b'>' | b'.' | b'~') {
        idx += 1;
    }
    let arrow_head = &s[..idx];
    let mut after = &s[idx..];
    // Must have seen at least one `>`-style termination OR a bare `--`/`==`
    // edge on this pass. We accept any arrow-like shape that contains at
    // least one `-` or `=` — the flexibility is deliberate because Phase C
    // needs approximate counts, not exact mermaid validation.
    if !arrow_head.contains('-') && !arrow_head.contains('=') {
        return None;
    }
    // Optional `|label|`.
    if after.starts_with('|')
        && let Some(close) = after[1..].find('|')
    {
        after = &after[close + 2..];
    }
    // Optional `-- text --` middle segment: if what follows is a word and
    // then another arrow, consume through that arrow.
    let bytes = after.as_bytes();
    if !bytes.is_empty() && !bytes[0].is_ascii_alphanumeric() {
        return Some(after);
    }
    // Look for a trailing arrow later on the line. If there is one, treat
    // the intervening characters as a label.
    if let Some(next_arrow) = find_next_arrow(after) {
        return Some(&after[next_arrow..]);
    }
    Some(after)
}

fn find_next_arrow(s: &str) -> Option<usize> {
    // Look for `-->`, `==>`, `->`, `-.->`, `--o`, etc. Return the byte index
    // of the character AFTER the arrow.
    for pat in ["-->|", "-->", "==>|", "==>", "-.->", "-->o", "--o", "->"] {
        if let Some(idx) = s.find(pat) {
            let end = idx + pat.len();
            // If the arrow was `-->|` we still need to skip the `label|`.
            if pat.ends_with('|')
                && let Some(close) = s[end..].find('|')
            {
                return Some(end + close + 1);
            }
            return Some(end);
        }
    }
    None
}

fn extract_node_ref(input: &str) -> Option<(String, &str)> {
    let s = input.trim_start();
    if s.is_empty() {
        return None;
    }
    let bytes = s.as_bytes();
    // Identifier starts with alnum / underscore.
    if !bytes[0].is_ascii_alphanumeric() && bytes[0] != b'_' {
        return None;
    }
    let mut end = 0;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    let id = s[..end].to_string();
    // Skip an optional shape suffix `[label]`, `(label)`, `{label}`, `>label]`.
    let mut rest = &s[end..];
    for (open, close) in [('[', ']'), ('(', ')'), ('{', '}')] {
        if rest.starts_with(open)
            && let Some(idx) = rest.find(close)
        {
            rest = &rest[idx + 1..];
            break;
        }
    }
    Some((id, rest))
}

fn parse_state_line(line: &str, nodes: &mut BTreeSet<String>, edges: &mut Vec<(String, String)>) {
    // `A --> B`, `A --> B : event`, `[*] --> B`.
    if let Some((lhs, rhs)) = line.split_once("-->") {
        let l = clean_state_id(lhs);
        let r = rhs.split(':').next().unwrap_or("");
        let r = clean_state_id(r);
        if !l.is_empty() {
            nodes.insert(l.clone());
        }
        if !r.is_empty() {
            nodes.insert(r.clone());
        }
        if !l.is_empty() && !r.is_empty() {
            edges.push((l, r));
        }
    } else {
        let id = clean_state_id(line);
        if !id.is_empty() {
            nodes.insert(id);
        }
    }
}

fn clean_state_id(s: &str) -> String {
    let s = s.trim();
    if s == "[*]" {
        return "__START_OR_END__".to_string();
    }
    s.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

fn parse_sequence_line(
    line: &str,
    nodes: &mut BTreeSet<String>,
    edges: &mut Vec<(String, String)>,
) {
    // `participant Alice`
    if let Some(name) = line.strip_prefix("participant ") {
        let id = sequence_actor(name);
        if !id.is_empty() {
            nodes.insert(id);
        }
        return;
    }
    if let Some(name) = line.strip_prefix("actor ") {
        let id = sequence_actor(name);
        if !id.is_empty() {
            nodes.insert(id);
        }
        return;
    }
    // `A->>B: msg`, `A->B`, `A-->>B`, `A-)B`, etc.
    const ARROWS: &[&str] = &["->>", "-->>", "-)", "-x", "--x", "--)", "->", "-->"];
    for arrow in ARROWS {
        if let Some(idx) = line.find(arrow) {
            let lhs = sequence_actor(&line[..idx]);
            let rest = &line[idx + arrow.len()..];
            let rhs = sequence_actor(rest.split(':').next().unwrap_or(""));
            if !lhs.is_empty() {
                nodes.insert(lhs.clone());
            }
            if !rhs.is_empty() {
                nodes.insert(rhs.clone());
            }
            if !lhs.is_empty() && !rhs.is_empty() {
                edges.push((lhs, rhs));
            }
            return;
        }
    }
}

fn sequence_actor(s: &str) -> String {
    s.trim()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

fn parse_class_line(line: &str, nodes: &mut BTreeSet<String>, edges: &mut Vec<(String, String)>) {
    // `class Foo`, `Foo <|-- Bar`, `Foo : +method()`.
    if let Some(name) = line.strip_prefix("class ") {
        let id = class_ident(name);
        if !id.is_empty() {
            nodes.insert(id);
        }
        return;
    }
    const RELATIONS: &[&str] = &[
        "<|--", "--|>", "*--", "--*", "o--", "--o", "<--", "-->", "..|>", "<|..", "..", "--",
    ];
    for rel in RELATIONS {
        if let Some(idx) = line.find(rel) {
            let lhs = class_ident(&line[..idx]);
            let rhs = class_ident(&line[idx + rel.len()..]);
            if !lhs.is_empty() {
                nodes.insert(lhs.clone());
            }
            if !rhs.is_empty() {
                nodes.insert(rhs.clone());
            }
            if !lhs.is_empty() && !rhs.is_empty() {
                edges.push((lhs, rhs));
            }
            return;
        }
    }
    let id = class_ident(line);
    if !id.is_empty() {
        nodes.insert(id);
    }
}

fn class_ident(s: &str) -> String {
    s.trim()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_td_two_nodes_two_edges_one_cycle() {
        let src = "graph TD\n  A --> B\n  B --> A\n";
        let sig = parse(src);
        assert_eq!(sig.nodes, 2);
        assert_eq!(sig.edges, 2);
        assert_eq!(sig.components, 1);
        assert_eq!(sig.cycles, 1);
        assert!(!sig.parse_error);
    }

    #[test]
    fn graph_td_linear() {
        let src = "graph TD\n  A --> B --> C\n";
        let sig = parse(src);
        assert_eq!(sig.nodes, 3);
        assert_eq!(sig.edges, 2);
        assert_eq!(sig.components, 1);
        assert_eq!(sig.cycles, 0);
    }

    #[test]
    fn sequence_participants_counted() {
        let src = "sequenceDiagram\n  participant Alice\n  participant Bob\n  Alice->>Bob: hi\n";
        let sig = parse(src);
        assert_eq!(sig.nodes, 2);
        assert_eq!(sig.edges, 1);
    }

    #[test]
    fn state_diagram_counts_transitions() {
        let src = "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Running\n  Running --> Idle\n";
        let sig = parse(src);
        // States: __START_OR_END__, Idle, Running = 3 nodes.
        assert_eq!(sig.nodes, 3);
        assert_eq!(sig.edges, 3);
    }
}
