use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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
