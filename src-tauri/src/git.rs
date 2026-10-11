use crate::executables;
use crate::models::{CommitResult, RepoInfo};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn inspect_repository(repo_path: &str) -> Result<RepoInfo> {
    let repo = Path::new(repo_path);
    if !repo.exists() {
        return Ok(RepoInfo {
            path: repo_path.to_string(),
            ..RepoInfo::default()
        });
    }

    if executables::resolve_git().is_none() {
        return Ok(RepoInfo {
            path: repo_path.to_string(),
            ..RepoInfo::default()
        });
    }

    let is_git_repo = run_git(repo_path, ["rev-parse", "--is-inside-work-tree"])
        .map(|value| value.trim() == "true")
        .unwrap_or(false);

    if !is_git_repo {
        return Ok(RepoInfo {
            path: repo_path.to_string(),
            is_git_repo: false,
            ..RepoInfo::default()
        });
    }

    let current_branch = run_git(repo_path, ["branch", "--show-current"])
        .ok()
        .map(|branch| branch.trim().to_string())
        .filter(|branch| !branch.is_empty())
        .or_else(|| {
            run_git(repo_path, ["rev-parse", "--abbrev-ref", "HEAD"])
                .ok()
                .map(|branch| branch.trim().to_string())
                .filter(|branch| !branch.is_empty())
        });

    let remote_origin = run_git(repo_path, ["remote", "get-url", "origin"])
        .ok()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty());

    let status_raw = run_git(repo_path, ["status", "--porcelain", "--untracked-files=all"])?;
    let status_summary = status_raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let git_dir = run_git(repo_path, ["rev-parse", "--git-dir"])
        .ok()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty());

    Ok(RepoInfo {
        path: repo_path.to_string(),
        is_git_repo: true,
        current_branch,
        remote_origin,
        dirty: !status_summary.is_empty(),
        git_dir,
        status_summary,
    })
}

pub fn ensure_valid_git_repo(repo_path: &str) -> Result<RepoInfo> {
    let info = inspect_repository(repo_path)?;
    if !info.is_git_repo {
        anyhow::bail!("The selected folder is not a valid Git repository.");
    }
    Ok(info)
}

pub fn commit_changes(repo_path: &str, message: &str) -> Result<CommitResult> {
    let trimmed_message = message.trim();
    if trimmed_message.is_empty() {
        anyhow::bail!("Enter a commit message before committing changes.");
    }

    let repo = ensure_valid_git_repo(repo_path)?;
    if !repo.dirty {
        anyhow::bail!("No changes are available to commit.");
    }

    run_git_slice(repo_path, &["add", "-A"])?;
    let summary = run_git_slice(repo_path, &["commit", "-m", trimmed_message])?;
    let commit_sha = run_git_slice(repo_path, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    let refreshed_repo = inspect_repository(repo_path)?;

    Ok(CommitResult {
        commit_sha,
        summary: summary.trim().to_string(),
        repo: refreshed_repo,
    })
}

pub fn tracked_files(repo_path: &str) -> Result<Vec<String>> {
    ensure_valid_git_repo(repo_path)?;

    Ok(run_git_slice(repo_path, &["ls-files", "-z"])?
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

pub fn discard_local_changes(repo_path: &str) -> Result<()> {
    ensure_valid_git_repo(repo_path)?;
    run_git_slice(repo_path, &["reset", "--hard", "HEAD"])?;
    run_git_slice(repo_path, &["clean", "-fd"])?;
    Ok(())
}

fn run_git<const N: usize>(repo_path: &str, args: [&str; N]) -> Result<String> {
    run_git_slice(repo_path, &args)
}

fn run_git_slice(repo_path: &str, args: &[&str]) -> Result<String> {
    let git_path = executables::require_git()?;
    let output = Command::new(&git_path)
        .args(args)
        .current_dir(repo_path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .with_context(|| format!("Failed to run git in {repo_path}"))?;

    if !output.status.success() {
        anyhow::bail!(
            "{}",
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn commits_new_and_tracked_files_but_leaves_ignored_files_alone() -> Result<()> {
        struct TestRepo(std::path::PathBuf);
        impl Drop for TestRepo {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let repo = TestRepo(std::env::temp_dir().join(format!("git-ftp-commit-{}", uuid::Uuid::new_v4())));
        fs::create_dir_all(&repo.0)?;
        let path = repo.0.to_str().unwrap();
        run_git_slice(path, &["init"])?;
        run_git_slice(path, &["config", "user.name", "Commit test"])?;
        run_git_slice(path, &["config", "user.email", "commit@example.com"])?;
        run_git_slice(path, &["config", "commit.gpgsign", "false"])?;
        fs::write(repo.0.join(".gitignore"), "ignored.txt\n")?;
        fs::write(repo.0.join("modified.txt"), "before")?;
        fs::write(repo.0.join("deleted.txt"), "before")?;
        commit_changes(path, "Initial commit")?;

        fs::create_dir_all(repo.0.join("assets"))?;
        fs::write(repo.0.join("assets/new image.png"), "image")?;
        fs::write(repo.0.join("assets/second.txt"), "second")?;
        fs::write(repo.0.join("ignored.txt"), "must stay ignored")?;
        let info = inspect_repository(path)?;
        assert_eq!(info.status_summary.len(), 2);
        assert!(info.status_summary.iter().all(|entry| entry.starts_with("??")));
        assert!(info.status_summary.iter().any(|entry| entry.contains("new image.png")));
        assert!(commit_changes(path, "  ").is_err());
        let result = commit_changes(path, "Add new files")?;
        assert!(!result.repo.dirty);
        assert!(tracked_files(path)?.contains(&"assets/new image.png".to_string()));
        assert!(!tracked_files(path)?.contains(&"ignored.txt".to_string()));

        fs::write(repo.0.join("modified.txt"), "after")?;
        fs::remove_file(repo.0.join("deleted.txt"))?;
        fs::write(repo.0.join("staged.txt"), "staged")?;
        run_git_slice(path, &["add", "staged.txt"])?;
        fs::write(repo.0.join("new.txt"), "new")?;
        let result = commit_changes(path, "Commit all changes")?;
        assert!(!result.repo.dirty);
        assert!(result.repo.status_summary.is_empty());
        let files = tracked_files(path)?;
        assert!(files.contains(&"staged.txt".to_string()));
        assert!(files.contains(&"new.txt".to_string()));
        assert!(!files.contains(&"deleted.txt".to_string()));
        assert_eq!(run_git_slice(path, &["show", "HEAD:modified.txt"])?, "after");
        assert!(repo.0.join("ignored.txt").exists());
        assert!(commit_changes(path, "Nothing left").is_err());
        Ok(())
    }
}
