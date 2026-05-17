//! `mehen-git` — git/repository operations.
//!
//! Per the rewrite plan §4.8, all repository-relative paths returned from
//! this crate are forward-slash UTF-8 (`Utf8PathBuf`) so serialized JSON,
//! Markdown tables, snapshots, and sticky comments never emit
//! backslash-separated paths on Windows. Internally, callers may convert to
//! filesystem `PathBuf` for IO, but report-level identity uses `Utf8PathBuf`.

#![forbid(unsafe_code)]

use core::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use gix::diff::tree::recorder::Change;
use gix::objs::TreeRefIter;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    /// Repository-relative path, forward-slash separated, UTF-8.
    pub path: Utf8PathBuf,
    pub status: ChangeStatus,
}

/// Discover a git repository from the current working directory.
/// Fails fast on shallow clones.
pub fn open_repo() -> Result<gix::Repository, GitError> {
    let repo = gix::discover(".").map_err(|_| GitError::RepoNotFound)?;
    if repo.is_shallow() {
        return Err(GitError::ShallowClone {
            hint: "Use 'actions/checkout' with 'fetch-depth: 0' for full history.".to_string(),
        });
    }
    Ok(repo)
}

/// List files changed between two revisions via tree-to-tree diff.
pub fn changed_files(
    repo: &gix::Repository,
    from: &str,
    to: &str,
) -> Result<Vec<ChangedFile>, GitError> {
    let from_tree = resolve_tree(repo, from)?;
    let to_tree = resolve_tree(repo, to)?;

    let mut recorder = gix::diff::tree::Recorder::default();
    gix::diff::tree(
        TreeRefIter::from_bytes(&from_tree.data, from_tree.id.kind()),
        TreeRefIter::from_bytes(&to_tree.data, to_tree.id.kind()),
        gix::diff::tree::State::default(),
        repo.objects.clone(),
        &mut recorder,
    )
    .map_err(|e| GitError::Internal(e.to_string()))?;

    Ok(recorder
        .records
        .into_iter()
        .map(|change| {
            let (path, status) = match change {
                Change::Addition { path, .. } => (path.to_string(), ChangeStatus::Added),
                Change::Deletion { path, .. } => (path.to_string(), ChangeStatus::Deleted),
                Change::Modification { path, .. } => (path.to_string(), ChangeStatus::Modified),
            };
            ChangedFile {
                path: normalize_repo_relative(&path),
                status,
            }
        })
        .collect())
}

/// Read file content at a specific revision. Returns `None` if the path
/// doesn't exist at that revision (e.g. newly added file with no baseline).
pub fn read_blob(
    repo: &gix::Repository,
    rev: &str,
    path: &Utf8Path,
) -> Result<Option<Vec<u8>>, GitError> {
    let tree = resolve_tree(repo, rev)?;
    let entry = tree
        .lookup_entry_by_path(path.as_std_path())
        .map_err(|e| GitError::Internal(e.to_string()))?;

    let Some(entry) = entry else {
        return Ok(None);
    };

    let object = entry
        .object()
        .map_err(|e| GitError::Internal(e.to_string()))?;
    let mut data = object.detach().data;
    normalize_trailing_newlines(&mut data);
    Ok(Some(data))
}

/// Resolve `rev` to a friendly symbolic branch name (`main`, `feature/x`),
/// or fall back to `rev` unchanged if no matching ref is found.
pub fn friendly_ref_label(repo: &gix::Repository, rev: &str) -> String {
    (|| {
        let id = repo.rev_parse_single(rev).ok()?;
        let commit = id.object().ok()?.peel_to_commit().ok()?;
        let refs = repo.references().ok()?;
        find_branch_for_commit(&refs, commit.id, true)
            .or_else(|| find_branch_for_commit(&refs, commit.id, false))
    })()
    .unwrap_or_else(|| rev.to_string())
}

/// Normalize a filesystem-style path string to the report path shape:
/// repository-relative, forward-slash separated, UTF-8.
pub fn normalize_repo_relative(path: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(path.replace('\\', "/"))
}

/// Replace trailing `\n`/`\r` with a single `\n`. Used to normalize blob
/// contents fetched from the object database, where line endings may be
/// preserved verbatim from the working tree.
fn normalize_trailing_newlines(data: &mut Vec<u8>) {
    let count_trailing = data
        .iter()
        .rev()
        .take_while(|&c| *c == b'\n' || *c == b'\r')
        .count();
    if count_trailing > 0 {
        data.truncate(data.len() - count_trailing);
    }
    data.push(b'\n');
}

fn find_branch_for_commit(
    refs: &gix::reference::iter::Platform<'_>,
    commit_id: gix::ObjectId,
    local: bool,
) -> Option<String> {
    let iter = if local {
        refs.local_branches().ok()?
    } else {
        refs.remote_branches().ok()?
    };
    let peeled = iter.peeled().ok()?;
    for reference in peeled.flatten() {
        if reference.id() == commit_id {
            let full = reference.name().as_bstr().to_string();
            return Some(shorten_ref_name(&full).to_string());
        }
    }
    None
}

fn shorten_ref_name(full: &str) -> &str {
    full.strip_prefix("refs/heads/")
        .or_else(|| full.strip_prefix("refs/remotes/origin/"))
        .or_else(|| {
            full.strip_prefix("refs/remotes/")
                .and_then(|s: &str| s.split_once('/').map(|(_, branch)| branch))
        })
        .unwrap_or(full)
}

fn resolve_tree<'a>(repo: &'a gix::Repository, rev: &str) -> Result<gix::Tree<'a>, GitError> {
    let id = repo
        .rev_parse_single(rev)
        .map_err(|_| GitError::RefNotFound(rev.to_string()))?;
    let object = id.object().map_err(|e| GitError::Internal(e.to_string()))?;
    let commit = object
        .peel_to_commit()
        .map_err(|e| GitError::Internal(e.to_string()))?;
    commit.tree().map_err(|e| GitError::Internal(e.to_string()))
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

    #[test]
    fn normalize_trailing_newlines_collapses_run() {
        let mut data = b"line\n\n\n".to_vec();
        normalize_trailing_newlines(&mut data);
        assert_eq!(data, b"line\n");
    }

    #[test]
    fn normalize_trailing_newlines_handles_crlf() {
        let mut data = b"line\r\n".to_vec();
        normalize_trailing_newlines(&mut data);
        assert_eq!(data, b"line\n");
    }
}
