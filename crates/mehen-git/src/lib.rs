//! `mehen-git` — git/repository operations.
//!
//! Phase 1 scope: the surface a Phase 5 `mehen diff` orchestrator will need
//! to detect changed files between two refs and read their content at each
//! revision.
//!
//! All paths returned from this crate are forward-slash UTF-8, including on
//! Windows — see the path normalization rule in the rewrite plan §4.8 and
//! the §3.7 parity contract.

#![forbid(unsafe_code)]

use core::fmt;

use camino::Utf8PathBuf;

#[derive(Debug)]
pub enum GitError {
    RepoNotFound,
    ShallowClone { hint: String },
    RefNotFound(String),
    BlobNotFound { rev: String, path: Utf8PathBuf },
    Internal(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepoNotFound => write!(f, "Not a git repository."),
            Self::ShallowClone { hint } => write!(f, "Shallow clone detected. {hint}"),
            Self::RefNotFound(r) => write!(f, "Could not resolve ref '{r}'."),
            Self::BlobNotFound { rev, path } => {
                write!(f, "Could not find '{path}' at rev '{rev}'.")
            }
            Self::Internal(msg) => write!(f, "Git error: {msg}"),
        }
    }
}

impl core::error::Error for GitError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug)]
pub struct ChangedFile {
    /// Repository-relative path with forward-slash separators on every OS.
    pub path: Utf8PathBuf,
    pub status: ChangeStatus,
}

/// Discover a git repository from `cwd`. Fails fast on shallow clones.
///
/// Phase 1 keeps the existing pre-1.0 behavior, but moved out of the root
/// `mehen` crate so engine-side callers don't need a transitive dependency
/// on the legacy CLI module tree.
pub fn open_repo() -> Result<gix::Repository, GitError> {
    let repo = gix::discover(".").map_err(|_| GitError::RepoNotFound)?;
    if repo.is_shallow() {
        return Err(GitError::ShallowClone {
            hint: "Use 'actions/checkout' with 'fetch-depth: 0' for full history.".to_string(),
        });
    }
    Ok(repo)
}

/// Normalize a filesystem-style path to the report path shape:
/// repository-relative, forward-slash separated, UTF-8.
pub fn normalize_repo_relative(path: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(path.replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_translates_backslashes() {
        assert_eq!(
            normalize_repo_relative("src\\foo\\bar.rs"),
            Utf8PathBuf::from("src/foo/bar.rs")
        );
    }
}
