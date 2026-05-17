use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Preprocessor data of a `C/C++` file.
#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct PreprocFile {
    /// The set of include directives explicitly written in a file
    pub(crate) direct_includes: HashSet<String>,
    /// The set of include directives implicitly imported in a file
    /// from other files
    pub(crate) indirect_includes: HashSet<String>,
    /// The set of macros of a file
    pub(crate) macros: HashSet<String>,
}

/// Preprocessor data of a series of `C/C++` files.
#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct PreprocResults {
    /// The preprocessor data of each `C/C++` file
    pub(crate) files: HashMap<PathBuf, PreprocFile>,
}
