use std::fmt;
use std::path::{Path, PathBuf};

use gix::diff::tree::recorder::Change;
use gix::objs::TreeRefIter;

use crate::tools::remove_blank_lines;

#[derive(Debug)]
pub(crate) enum GitError {
    RepoNotFound,
    ShallowClone {
        hint: String,
    },
    RefNotFound(String),
    #[allow(dead_code)]
    BlobNotFound {
        rev: String,
        path: PathBuf,
    },
    Internal(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepoNotFound => write!(f, "Not a git repository."),
            Self::ShallowClone { hint } => write!(f, "Shallow clone detected. {hint}"),
            Self::RefNotFound(r) => write!(f, "Could not resolve ref '{r}'."),
            Self::BlobNotFound { rev, path } => {
                write!(f, "Could not find '{}' at rev '{rev}'.", path.display())
            }
            Self::Internal(msg) => write!(f, "Git error: {msg}"),
        }
    }
}

impl std::error::Error for GitError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChangeStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
pub(crate) struct ChangedFile {
    pub path: PathBuf,
    pub status: ChangeStatus,
}

/// Discover a git repository from the current working directory.
/// Fails fast on shallow clones.
pub(crate) fn open_repo() -> Result<gix::Repository, GitError> {
    let repo = gix::discover(".").map_err(|_| GitError::RepoNotFound)?;

    if repo.is_shallow() {
        return Err(GitError::ShallowClone {
            hint: "Use 'actions/checkout' with 'fetch-depth: 0' for full history.".to_string(),
        });
    }

    Ok(repo)
}

/// List files changed between two revisions via tree-to-tree diff.
pub(crate) fn changed_files(
    repo: &gix::Repository,
    from: &str,
    to: &str,
) -> Result<Vec<ChangedFile>, GitError> {
    let from_tree = resolve_tree(repo, from)?;
    let to_tree = resolve_tree(repo, to)?;

    let mut recorder = gix::diff::tree::Recorder::default();
    gix::diff::tree(
        TreeRefIter::from_bytes(&from_tree.data),
        TreeRefIter::from_bytes(&to_tree.data),
        gix::diff::tree::State::default(),
        repo.objects.clone(),
        &mut recorder,
    )
    .map_err(|e| GitError::Internal(e.to_string()))?;

    let files = recorder
        .records
        .into_iter()
        .map(|change| {
            let (path, status) = match change {
                Change::Addition { path, .. } => {
                    (PathBuf::from(path.to_string()), ChangeStatus::Added)
                }
                Change::Deletion { path, .. } => {
                    (PathBuf::from(path.to_string()), ChangeStatus::Deleted)
                }
                Change::Modification { path, .. } => {
                    (PathBuf::from(path.to_string()), ChangeStatus::Modified)
                }
            };
            ChangedFile { path, status }
        })
        .collect();

    Ok(files)
}

/// Read file content at a specific revision. Returns `None` if the path
/// doesn't exist at that revision (e.g. newly added file with no baseline).
pub(crate) fn read_blob(
    repo: &gix::Repository,
    rev: &str,
    path: &Path,
) -> Result<Option<Vec<u8>>, GitError> {
    let tree = resolve_tree(repo, rev)?;

    let entry = tree
        .lookup_entry_by_path(path)
        .map_err(|e| GitError::Internal(e.to_string()))?;

    let Some(entry) = entry else {
        return Ok(None);
    };

    let object = entry
        .object()
        .map_err(|e| GitError::Internal(e.to_string()))?;

    let mut data = object.detach().data;
    remove_blank_lines(&mut data);
    Ok(Some(data))
}

/// Try to resolve a rev string to a friendly symbolic branch name.
///
/// Resolves `rev` to a commit OID, then scans local and remote branches for
/// one that points at the same commit.  Returns the short branch name
/// (e.g. `"main"`) on a match, or falls back to `rev` unchanged.
pub(crate) fn friendly_ref_label(repo: &gix::Repository, rev: &str) -> String {
    let Ok(id) = repo.rev_parse_single(rev) else {
        return rev.to_string();
    };
    let Ok(obj) = id.object() else {
        return rev.to_string();
    };
    let Ok(commit) = obj.peel_to_commit() else {
        return rev.to_string();
    };
    let commit_id = commit.id;

    let Ok(refs) = repo.references() else {
        return rev.to_string();
    };

    // Prefer local branches, fall back to remote tracking branches.
    if let Some(name) = find_branch_for_commit(&refs, commit_id, true) {
        return name;
    }
    if let Some(name) = find_branch_for_commit(&refs, commit_id, false) {
        return name;
    }

    rev.to_string()
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

/// Strip standard ref prefixes to produce a short branch name.
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
