//! Native diagram parsers used by §12.2 Diagram Complexity.
//!
//! Mermaid is parsed structurally (nodes, edges, connected components,
//! cycles). PlantUML, DOT, D2, and Vega-Lite fall back to a token-proxy
//! metric that counts identifier-looking lines as nodes — enough to size
//! the diagram for §12.2 without dragging in a full parser.

pub(crate) mod mermaid;

/// Parsed diagram signals consumed by §12.2 and the artifact-debt pipeline.
#[derive(Debug, Default, Clone)]
pub(crate) struct DiagramSignal {
    pub(crate) nodes: u64,
    pub(crate) edges: u64,
    pub(crate) components: u64,
    pub(crate) cycles: u64,
    pub(crate) parse_error: bool,
    pub(crate) has_title: bool,
}

/// Parse `body` as `language`. Returns a best-effort [`DiagramSignal`].
/// Unknown languages return `parse_error = true` with zeroed counts so the
/// per-diagram burden still gets the +2.0 parse-error term from §12.2.
pub(crate) fn parse_diagram(language: &str, body: &str) -> DiagramSignal {
    let lang = language.to_lowercase();
    match lang.as_str() {
        "mermaid" => mermaid::parse(body),
        "plantuml" | "puml" => parse_plantuml(body),
        "dot" | "graphviz" => parse_dot(body),
        "d2" => parse_d2(body),
        "vega-lite" | "vegalite" | "vl" | "vega" => parse_vega_like(body),
        _ => DiagramSignal {
            parse_error: true,
            ..DiagramSignal::default()
        },
    }
}

/// PlantUML proxy: count identifiers on the LHS of `:`/`-->`/`->`/`<--`/`<-`.
/// Titles come from leading `title` / `@startuml title`.
fn parse_plantuml(body: &str) -> DiagramSignal {
    let mut nodes: std::collections::BTreeSet<String> = Default::default();
    let mut edges: u64 = 0;
    let mut has_title = false;

    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('\'')
            || line.starts_with("/'")
            || line.starts_with("@startuml")
            || line.starts_with("@enduml")
            || line.starts_with("skinparam")
        {
            if line.starts_with("@startuml ") || line.starts_with("title ") {
                has_title = true;
            }
            continue;
        }
        if line.starts_with("title ") {
            has_title = true;
            continue;
        }
        for (lhs, rhs) in split_edge(line) {
            if let Some(n) = ident(lhs) {
                nodes.insert(n);
            }
            if let Some(n) = ident(rhs) {
                nodes.insert(n);
            }
            edges += 1;
        }
        if !contains_edge(line)
            && let Some(n) = ident(line)
        {
            nodes.insert(n);
        }
    }

    let components = connected_components(&nodes, &[]);
    DiagramSignal {
        nodes: nodes.len() as u64,
        edges,
        components,
        cycles: 0,
        parse_error: false,
        has_title,
    }
}

/// DOT / Graphviz proxy. Same technique as plantuml but with `->` / `--`.
fn parse_dot(body: &str) -> DiagramSignal {
    let mut nodes: std::collections::BTreeSet<String> = Default::default();
    let mut edges: u64 = 0;
    let mut has_title = false;
    for raw in body.lines() {
        let line = raw.trim().trim_end_matches(';');
        if line.is_empty()
            || line.starts_with("//")
            || line.starts_with('#')
            || line.starts_with("digraph")
            || line.starts_with("graph ")
            || line == "}"
            || line == "{"
        {
            if line.contains("label=") {
                has_title = true;
            }
            continue;
        }
        for (lhs, rhs) in split_edge(line) {
            if let Some(n) = ident(lhs) {
                nodes.insert(n);
            }
            if let Some(n) = ident(rhs) {
                nodes.insert(n);
            }
            edges += 1;
        }
        if !contains_edge(line)
            && let Some(n) = ident(line)
        {
            nodes.insert(n);
        }
    }
    let components = connected_components(&nodes, &[]);
    DiagramSignal {
        nodes: nodes.len() as u64,
        edges,
        components,
        cycles: 0,
        parse_error: false,
        has_title,
    }
}

/// D2 proxy: `a -> b`, `a <-> b`, `a: label`.
fn parse_d2(body: &str) -> DiagramSignal {
    let mut nodes: std::collections::BTreeSet<String> = Default::default();
    let mut edges: u64 = 0;
    let mut has_title = false;
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("title:") {
            has_title = true;
            continue;
        }
        for (lhs, rhs) in split_edge(line) {
            if let Some(n) = ident(lhs) {
                nodes.insert(n);
            }
            if let Some(n) = ident(rhs) {
                nodes.insert(n);
            }
            edges += 1;
        }
        if !contains_edge(line)
            && let Some((lhs, _)) = line.split_once(':')
            && let Some(n) = ident(lhs)
        {
            nodes.insert(n);
        }
    }
    let components = connected_components(&nodes, &[]);
    DiagramSignal {
        nodes: nodes.len() as u64,
        edges,
        components,
        cycles: 0,
        parse_error: false,
        has_title,
    }
}

/// Vega-Lite proxy: count top-level `marks`, `encoding`, `data` objects as
/// nodes. This is far too crude to be semantically accurate but suffices as
/// a size-proxy for §12.2.
fn parse_vega_like(body: &str) -> DiagramSignal {
    let mut nodes: u64 = 0;
    let mut has_title = false;
    for raw in body.lines() {
        let line = raw.trim();
        if line.starts_with("\"title\"") {
            has_title = true;
        }
        if line.ends_with('{') || line.ends_with('[') {
            nodes += 1;
        }
    }
    DiagramSignal {
        nodes,
        edges: 0,
        components: if nodes == 0 { 0 } else { 1 },
        cycles: 0,
        parse_error: false,
        has_title,
    }
}

fn split_edge(line: &str) -> Vec<(&str, &str)> {
    let seps = ["-->", "<--", "->", "<-", "==>", "<==", "---", "==="];
    for sep in seps {
        if let Some(idx) = line.find(sep) {
            let lhs = &line[..idx];
            let rhs = &line[idx + sep.len()..];
            return vec![(lhs.trim(), rhs.trim())];
        }
    }
    Vec::new()
}

fn contains_edge(line: &str) -> bool {
    ["-->", "<--", "->", "<-", "==>", "<==", "---", "==="]
        .iter()
        .any(|s| line.contains(s))
}

fn ident(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Strip brackets, pipes, and trailing labels.
    let stripped: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    }
}

pub(crate) fn connected_components(
    nodes: &std::collections::BTreeSet<String>,
    edges: &[(String, String)],
) -> u64 {
    if nodes.is_empty() {
        return 0;
    }
    let mut parent: std::collections::BTreeMap<String, String> =
        nodes.iter().map(|n| (n.clone(), n.clone())).collect();
    fn find(parent: &mut std::collections::BTreeMap<String, String>, x: &str) -> String {
        let p = parent.get(x).cloned().unwrap_or_else(|| x.to_string());
        if p == x {
            p
        } else {
            let root = find(parent, &p);
            parent.insert(x.to_string(), root.clone());
            root
        }
    }
    for (a, b) in edges {
        if !parent.contains_key(a) {
            parent.insert(a.clone(), a.clone());
        }
        if !parent.contains_key(b) {
            parent.insert(b.clone(), b.clone());
        }
        let ra = find(&mut parent, a);
        let rb = find(&mut parent, b);
        if ra != rb {
            parent.insert(ra, rb);
        }
    }
    let mut scratch = parent.clone();
    let roots: std::collections::BTreeSet<String> =
        parent.keys().map(|k| find(&mut scratch, k)).collect();
    roots.len() as u64
}
