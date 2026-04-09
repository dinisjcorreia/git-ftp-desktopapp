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

    let status_raw = run_git(repo_path, ["status", "--porcelain"]).unwrap_or_default();
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

pub fn commit_tracked_changes(repo_path: &str, message: &str) -> Result<CommitResult> {
    let trimmed_message = message.trim();
    if trimmed_message.is_empty() {
        anyhow::bail!("Enter a commit message before committing tracked changes.");
    }

    let repo = ensure_valid_git_repo(repo_path)?;
    let tracked_changes = repo
        .status_summary
        .iter()
        .filter(|line| !line.starts_with("??"))
        .count();

    if tracked_changes == 0 {
        anyhow::bail!("No tracked changes are available to commit.");
    }

    run_git_slice(repo_path, &["add", "-u"])?;
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
