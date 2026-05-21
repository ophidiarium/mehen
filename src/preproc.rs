use std::collections::{HashMap, HashSet, hash_map};
use std::path::{Path, PathBuf};

use petgraph::{
    Direction, algo::kosaraju_scc, graph::NodeIndex, stable_graph::StableGraph, visit::Dfs,
};
use serde::{Deserialize, Serialize};

use crate::tools::*;

/// Preprocessor data of a `C/C++` file.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PreprocFile {
    /// The set of include directives explicitly written in a file
    pub direct_includes: HashSet<String>,
    /// The set of include directives implicitly imported in a file
    /// from other files
    pub indirect_includes: HashSet<String>,
    /// The set of macros of a file
    pub macros: HashSet<String>,
}

/// Preprocessor data of a series of `C/C++` files.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PreprocResults {
    /// The preprocessor data of each `C/C++` file
    pub files: HashMap<PathBuf, PreprocFile>,
}

impl PreprocFile {
    /// Adds new macros to the set of macro of a file.
    pub fn new_macros(macros: &[&str]) -> Self {
        let mut pf = Self::default();
        for m in macros {
            pf.macros.insert((*m).to_string());
        }
        pf
    }
}

/// Returns the macros contained in a `C/C++` file.
pub fn get_macros<S: ::std::hash::BuildHasher>(
    file: &Path,
    files: &HashMap<PathBuf, PreprocFile, S>,
) -> HashSet<String> {
    let mut macros = HashSet::new();
    if let Some(pf) = files.get(file) {
        for m in pf.macros.iter() {
            macros.insert(m.to_string());
        }
        for f in pf.indirect_includes.iter() {
            if let Some(pf) = files.get(&PathBuf::from(f)) {
                for m in pf.macros.iter() {
                    macros.insert(m.to_string());
                }
            }
        }
    }
    macros
}

/// Constructs a dependency graph of the include directives
/// in a `C/C++` file.
///
/// The dependency graph is built using both preprocessor data and not
/// extracted from the considered `C/C++` files.
pub fn fix_includes<S: ::std::hash::BuildHasher>(
    files: &mut HashMap<PathBuf, PreprocFile, S>,
    all_files: &HashMap<String, Vec<PathBuf>, S>,
) {
    let mut nodes: HashMap<PathBuf, NodeIndex> = HashMap::new();
    // Since we'll remove strong connected components we need to have a stable graph
    // in order to use the nodes we've in the nodes HashMap.
    let mut g = StableGraph::new();

    // First we build a graph of include dependencies
    for (file, pf) in files.iter() {
        let node = match nodes.entry(file.clone()) {
            hash_map::Entry::Occupied(l) => *l.get(),
            hash_map::Entry::Vacant(p) => *p.insert(g.add_node(file.clone())),
        };
        let direct_includes = &pf.direct_includes;
        for i in direct_includes {
            let possibilities = guess_file(file, i, all_files);
            for i in possibilities {
                if &i != file {
                    let i = match nodes.entry(i.clone()) {
                        hash_map::Entry::Occupied(l) => *l.get(),
                        hash_map::Entry::Vacant(p) => *p.insert(g.add_node(i)),
                    };
                    g.add_edge(node, i, 0);
                } else {
                    // TODO: add an option to display warning
                    eprintln!("Warning: possible self inclusion {file:?}");
                }
            }
        }
    }

    // In order to walk in the graph without issues due to cycles
    // we replace strong connected components by a unique node
    // All the paths in a scc finally represents a kind of unique file containing
    // all the files in the scc.
    let mut scc = kosaraju_scc(&g);
    let mut scc_map: HashMap<NodeIndex, HashSet<String>> = HashMap::new();
    for component in scc.iter_mut() {
        if component.len() > 1 {
            // For Firefox, there are only few scc and all of them are pretty small
            // So no need to take a hammer here (for 'contains' stuff).
            // TODO: in some case a hammer can be useful: check perf Vec vs HashSet
            let mut incoming = Vec::new();
            let mut outgoing = Vec::new();
            let mut paths = HashSet::new();

            for c in component.iter() {
                for i in g.neighbors_directed(*c, Direction::Incoming) {
                    if !component.contains(&i) && !incoming.contains(&i) {
                        incoming.push(i);
                    }
                }
                for o in g.neighbors_directed(*c, Direction::Outgoing) {
                    if !component.contains(&o) && !outgoing.contains(&o) {
                        outgoing.push(o);
                    }
                }
            }

            let replacement = g.add_node(PathBuf::from(""));
            for i in incoming.drain(..) {
                g.add_edge(i, replacement, 0);
            }
            for o in outgoing.drain(..) {
                g.add_edge(replacement, o, 0);
            }
            for c in component.drain(..) {
                let path = g.remove_node(c).unwrap();
                paths.insert(path.to_str().unwrap().to_string());
                *nodes.get_mut(&path).unwrap() = replacement;
            }

            eprintln!("Warning: possible include cycle:");
            for p in paths.iter() {
                eprintln!("  - {p:?}");
            }
            eprintln!();

            scc_map.insert(replacement, paths);
        }
    }

    for (path, node) in nodes {
        let mut dfs = Dfs::new(&g, node);
        if let Some(pf) = files.get_mut(&path) {
            let x_inc = &mut pf.indirect_includes;
            while let Some(node) = dfs.next(&g) {
                let w = g.node_weight(node).unwrap();
                if w == &PathBuf::from("") {
                    let paths = scc_map.get(&node);
                    if let Some(paths) = paths {
                        for p in paths {
                            x_inc.insert(p.to_string());
                        }
                    } else {
                        eprintln!("DEBUG: {path:?} {node:?}");
                    }
                } else {
                    x_inc.insert(w.to_str().unwrap().to_string());
                }
            }
        } else {
            eprintln!(
                "Warning: included file which has not been preprocessed: {:?}",
                path
            );
        }
    }
}

/// This function is deprecated and no longer functional as preprocessor support
/// for C/C++ has been removed.
///
/// [`PreprocResults`]: struct.PreprocResults.html
#[deprecated(note = "Preprocessor support removed with C/C++ language removal")]
#[allow(dead_code)]
pub fn preprocess(_path: &Path, _results: &mut PreprocResults) {
    // No-op: Preprocessor support removed with C/C++ language removal
}
