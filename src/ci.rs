use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CiProvider {
    GitHubActions,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct CiContext {
    pub provider: CiProvider,
    pub event_name: String,
    pub base_ref: Option<String>,
    pub head_sha: Option<String>,
    pub changed_files: Option<Vec<PathBuf>>,
    pub pr_number: Option<u64>,
    pub repository: Option<String>,
}

pub(crate) fn detect() -> Option<CiContext> {
    detect_github_actions()
}

fn detect_github_actions() -> Option<CiContext> {
    if std::env::var("GITHUB_ACTIONS").ok()?.as_str() != "true" {
        return None;
    }

    let event_name = std::env::var("GITHUB_EVENT_NAME").unwrap_or_default();
    let head_sha = std::env::var("GITHUB_SHA").ok();
    let repository = std::env::var("GITHUB_REPOSITORY").ok();

    let mut base_ref = std::env::var("GITHUB_BASE_REF")
        .ok()
        .filter(|s| !s.is_empty());
    let mut changed_files = None;
    let mut pr_number = None;

    if let Ok(event_path) = std::env::var("GITHUB_EVENT_PATH")
        && let Ok(data) = std::fs::read_to_string(&event_path)
        && let Ok(payload) = serde_json::from_str::<serde_json::Value>(&data)
    {
        match event_name.as_str() {
            "push" => {
                changed_files = extract_push_changed_files(&payload);
            }
            "pull_request" => {
                if let Some(pr) = payload.get("pull_request") {
                    if base_ref.is_none() {
                        base_ref = pr
                            .get("base")
                            .and_then(|b| b.get("ref"))
                            .and_then(|r| r.as_str())
                            .map(|s| s.to_string());
                    }
                    pr_number = payload.get("number").and_then(|n| n.as_u64());
                }
            }
            "merge_group" => {
                if let Some(mg) = payload.get("merge_group")
                    && base_ref.is_none()
                {
                    base_ref = mg
                        .get("base_ref")
                        .and_then(|r| r.as_str())
                        .map(|s| s.to_string());
                }
            }
            _ => {}
        }
    }

    Some(CiContext {
        provider: CiProvider::GitHubActions,
        event_name,
        base_ref,
        head_sha,
        changed_files,
        pr_number,
        repository,
    })
}

fn extract_push_changed_files(payload: &serde_json::Value) -> Option<Vec<PathBuf>> {
    let commits = payload.get("commits")?.as_array()?;
    let mut files = std::collections::HashSet::new();

    for commit in commits {
        for key in &["added", "modified", "removed"] {
            if let Some(arr) = commit.get(key).and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(path) = item.as_str() {
                        files.insert(PathBuf::from(path));
                    }
                }
            }
        }
    }

    if files.is_empty() {
        None
    } else {
        let mut sorted: Vec<PathBuf> = files.into_iter().collect();
        sorted.sort();
        Some(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_push_changed_files() {
        let payload = serde_json::json!({
            "commits": [
                {
                    "added": ["src/new.rs"],
                    "modified": ["src/main.rs"],
                    "removed": ["src/old.rs"]
                },
                {
                    "added": [],
                    "modified": ["src/main.rs", "src/lib.rs"],
                    "removed": []
                }
            ]
        });

        let files = extract_push_changed_files(&payload).unwrap();
        assert_eq!(
            files,
            vec![
                PathBuf::from("src/lib.rs"),
                PathBuf::from("src/main.rs"),
                PathBuf::from("src/new.rs"),
                PathBuf::from("src/old.rs"),
            ]
        );
    }

    #[test]
    fn test_extract_push_no_commits() {
        let payload = serde_json::json!({});
        assert!(extract_push_changed_files(&payload).is_none());
    }

    #[test]
    fn test_extract_push_empty_commits() {
        let payload = serde_json::json!({
            "commits": []
        });
        assert!(extract_push_changed_files(&payload).is_none());
    }

    #[test]
    fn test_detect_not_github() {
        // Ensure GITHUB_ACTIONS is not set for this test
        // SAFETY: single-threaded test context; no other thread reads this var concurrently
        unsafe {
            std::env::remove_var("GITHUB_ACTIONS");
        }
        assert!(detect().is_none());
    }
}
