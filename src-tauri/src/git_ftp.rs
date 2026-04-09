use crate::diagnostics;
use crate::executables;
use crate::git;
use crate::models::{
    CommandProbeResult, DeploymentProfile, LogStream, ProfileDraft, ProfileValidationResult,
    RemoteCleanupResult, RunAction, RunOptions, SnapshotBootstrapResult,
    SnapshotProgressEvent, SnapshotProgressStatus, ValidationMessage, ValidationMessageKind,
};
use crate::process::CommandSpec;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::fs;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use uuid::Uuid;

pub const SNAPSHOT_PROGRESS_EVENT: &str = "snapshot-progress";
const UNSAFE_DEPLOY_PATH_CHARS: [char; 7] = [' ', '%', '?', '[', ']', '{', '}'];

pub fn timestamp_now() -> String {
    Utc::now().to_rfc3339()
}

pub fn merge_profile(existing: Option<&DeploymentProfile>, draft: ProfileDraft, secret_ref: String) -> DeploymentProfile {
    let now = timestamp_now();
    DeploymentProfile {
        id: draft
            .id
            .unwrap_or_else(|| existing.map(|profile| profile.id.clone()).unwrap_or_else(|| Uuid::new_v4().to_string())),
        name: draft.name.trim().to_string(),
        protocol: draft.protocol,
        host: draft.host.trim().to_string(),
        port: draft.port,
        username: draft.username.trim().to_string(),
        remote_path: draft.remote_path.trim().to_string(),
        secret_ref,
        use_git_config_defaults: draft.use_git_config_defaults,
        flags: draft.flags,
        last_deployed_at: existing.and_then(|profile| profile.last_deployed_at.clone()),
        created_at: existing
            .map(|profile| profile.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    }
}

pub fn duplicate_profile(profile: &DeploymentProfile, secret_ref: String) -> DeploymentProfile {
    let now = timestamp_now();
    DeploymentProfile {
        id: Uuid::new_v4().to_string(),
        name: format!("{} Copy", profile.name),
        protocol: profile.protocol.clone(),
        host: profile.host.clone(),
        port: profile.port,
        username: profile.username.clone(),
        remote_path: profile.remote_path.clone(),
        secret_ref,
        use_git_config_defaults: profile.use_git_config_defaults,
        flags: profile.flags.clone(),
        last_deployed_at: None,
        created_at: now.clone(),
        updated_at: now,
    }
}

pub fn validate_profile(
    app: &AppHandle,
    repo_path: &str,
    draft: &ProfileDraft,
    secret: Option<&str>,
    probe_remote: bool,
) -> Result<ProfileValidationResult> {
    let diagnostics = diagnostics::startup_diagnostics(app);
    let repo = git::inspect_repository(repo_path)?;

    let mut messages = Vec::new();
    if !diagnostics.git_available {
        messages.push(message(
            ValidationMessageKind::Error,
            "Git is not available. Repository validation and deployment are disabled.",
        ));
    }
    if !diagnostics.git_ftp_available {
        messages.push(message(
            ValidationMessageKind::Error,
            "git-ftp is not available. Install it before running deployment actions.",
        ));
    }
    if !repo.is_git_repo {
        messages.push(message(
            ValidationMessageKind::Error,
            "The selected folder is not a valid Git repository.",
        ));
    }

    validate_draft(draft, secret.is_some(), &mut messages);
    let encoded_sync_paths = find_paths_requiring_encoded_sync(repo_path, draft.flags.syncroot.as_deref())
        .unwrap_or_default();
    if !encoded_sync_paths.is_empty() {
        messages.push(message(
            ValidationMessageKind::Warning,
            format_encoded_sync_notice(&encoded_sync_paths),
        ));
    }

    let valid = !messages
        .iter()
        .any(|item| matches!(item.kind, ValidationMessageKind::Error));

    let command_preview = if valid {
        let preview = build_command_preview(draft, RunAction::Push, &RunOptions { dry_run: true, verbose: false });
        Some(preview)
    } else {
        None
    };

    let probe_result = if valid && probe_remote {
        Some(run_dry_probe(app, repo_path, draft, secret.unwrap_or_default())?)
    } else {
        None
    };

    Ok(ProfileValidationResult {
        valid: valid
            && probe_result
                .as_ref()
                .map(|probe| probe.success)
                .unwrap_or(true),
        command_preview,
        messages,
        repo,
        diagnostics,
        probe_result,
    })
}

pub fn build_command_spec(
    app: &AppHandle,
    repo_path: &str,
    profile: &DeploymentProfile,
    action: RunAction,
    options: &RunOptions,
    secret: String,
) -> Result<CommandSpec> {
    if should_use_encoded_sync_fallback(repo_path, profile, &action)? {
        return build_encoded_sync_command_spec(app, repo_path, profile, action, options, secret);
    }

    let git_ftp = executables::require_git_ftp(app)
        .context("Could not resolve git-ftp from the bundled resources or the local system.")?;
    let mut args = git_ftp.args_prefix.clone();
    args.extend(build_args(profile, &action, options));
    let command_preview = format_command_preview(&git_ftp.executable, &args);
    let record_id = Uuid::new_v4().to_string();

    Ok(CommandSpec {
        run_id: record_id.clone(),
        executable: git_ftp.executable,
        args,
        envs: build_command_envs(app, profile, &secret)?,
        current_dir: PathBuf::from(repo_path),
        record: crate::models::RunRecord {
            id: record_id,
            repo_path: repo_path.to_string(),
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            action,
            started_at: timestamp_now(),
            finished_at: None,
            success: false,
            exit_code: None,
            command_preview,
            logs: Vec::new(),
            cancelled: false,
            changed_files: Vec::new(),
        },
    })
}

pub async fn bootstrap_snapshot(
    app: &AppHandle,
    draft: &ProfileDraft,
    password: &str,
    local_path: &str,
    operation_id: &str,
) -> Result<SnapshotBootstrapResult> {
    if executables::resolve_lftp(app).is_none() {
        return Err(anyhow!(
            "`git-ftp snapshot` requires `lftp`, but it was not found in PATH or common install locations."
        ));
    }

    if password.trim().is_empty() {
        return Err(anyhow!("A password is required before downloading from the remote FTP site."));
    }

    let mut messages = Vec::new();
    validate_draft(draft, true, &mut messages);
    if let Some(error) = messages
        .into_iter()
        .find(|item| matches!(item.kind, ValidationMessageKind::Error))
    {
        return Err(anyhow!(error.text));
    }

    let git_ftp = executables::require_git_ftp(app)
        .context("Could not resolve git-ftp from the bundled resources or the local system.")?;
    let local_path = local_path.trim();
    if local_path.is_empty() {
        return Err(anyhow!("Choose a local destination folder before downloading the remote site."));
    }

    let destination_existed = Path::new(local_path).exists();
    if destination_existed {
        let mut entries = std::fs::read_dir(local_path)
            .with_context(|| format!("Could not inspect local destination `{local_path}`"))?;
        if entries.next().transpose()?.is_some() {
            return Err(anyhow!(
                "The selected local destination is not empty. Choose an empty folder for the snapshot download."
            ));
        }
    }

    let profile = merge_profile(None, draft.clone(), Uuid::new_v4().to_string());
    let mut args = git_ftp.args_prefix.clone();
    args.extend(build_snapshot_args(&profile, local_path));
    let command_preview = format_command_preview(&git_ftp.executable, &args);
    emit_snapshot_progress(
        app,
        operation_id,
        SnapshotProgressStatus::Running,
        8,
        "Preparing remote snapshot…",
        "Validating the FTP connection settings and local destination folder.",
        Some(command_preview.clone()),
        None,
        None,
    );
    let temp_repo = TempSnapshotRepo::create()?;
    emit_snapshot_progress(
        app,
        operation_id,
        SnapshotProgressStatus::Running,
        18,
        "Preparing local repository…",
        "Initialized a temporary local Git repository and starting the remote download.",
        Some(command_preview.clone()),
        None,
        None,
    );

    let mut child = Command::new(&git_ftp.executable)
        .args(&args)
        .current_dir(temp_repo.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null())
        .envs(build_command_envs(app, &profile, password)?)
        .spawn()
        .with_context(|| "Failed to start `git-ftp snapshot`.")?;

    let stdout_buffer = Arc::new(Mutex::new(Vec::<String>::new()));
    let stderr_buffer = Arc::new(Mutex::new(Vec::<String>::new()));
    let tracker = Arc::new(Mutex::new(SnapshotProgressTracker::new()));

    let stdout_task = child.stdout.take().map(|pipe| {
        let app = app.clone();
        let operation_id = operation_id.to_string();
        let command_preview = command_preview.clone();
        let stdout_buffer = Arc::clone(&stdout_buffer);
        let tracker = Arc::clone(&tracker);
        tokio::spawn(async move {
            read_snapshot_stream(
                app,
                operation_id,
                command_preview,
                pipe,
                LogStream::Stdout,
                stdout_buffer,
                tracker,
            )
            .await;
        })
    });

    let stderr_task = child.stderr.take().map(|pipe| {
        let app = app.clone();
        let operation_id = operation_id.to_string();
        let command_preview = command_preview.clone();
        let stderr_buffer = Arc::clone(&stderr_buffer);
        let tracker = Arc::clone(&tracker);
        tokio::spawn(async move {
            read_snapshot_stream(
                app,
                operation_id,
                command_preview,
                pipe,
                LogStream::Stderr,
                stderr_buffer,
                tracker,
            )
            .await;
        })
    });

    let status = child.wait().await?;

    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }

    let stdout = stdout_buffer.lock().await.join("\n");
    let mut stderr = stderr_buffer.lock().await.join("\n");
    let success = status.success();

    if success {
        emit_snapshot_progress(
            app,
            operation_id,
            SnapshotProgressStatus::Success,
            100,
            "Remote snapshot completed.",
            "The remote files were downloaded and the local Git repository is ready.",
            Some(command_preview.clone()),
            None,
            None,
        );
    } else {
        let cleanup_note = match cleanup_snapshot_destination(local_path, destination_existed) {
            Ok(true) => Some(
                "The partial local snapshot was removed, so you can retry with the same destination folder."
                    .to_string(),
            ),
            Ok(false) => None,
            Err(error) => Some(format!(
                "The snapshot failed and the app could not fully clean the local destination: {error}"
            )),
        };

        if let Some(note) = cleanup_note.as_ref() {
            if !stderr.trim().is_empty() {
                stderr.push_str("\n\n");
            }
            stderr.push_str(note);
        }

        let progress = tracker.lock().await.progress.max(24);
        emit_snapshot_progress(
            app,
            operation_id,
            SnapshotProgressStatus::Error,
            progress,
            "Remote snapshot failed.",
            cleanup_note
                .as_deref()
                .unwrap_or("git-ftp reported an error before the initial download could finish."),
            Some(command_preview.clone()),
            None,
            None,
        );
    }

    let repo = if success {
        git::inspect_repository(local_path)?
    } else {
        crate::models::RepoInfo {
            path: local_path.to_string(),
            ..crate::models::RepoInfo::default()
        }
    };

    Ok(SnapshotBootstrapResult {
        success,
        local_path: local_path.to_string(),
        command_preview,
        stdout,
        stderr,
        exit_code: status.code(),
        repo,
        profile,
    })
}

fn cleanup_snapshot_destination(local_path: &str, destination_existed: bool) -> Result<bool> {
    let path = Path::new(local_path);
    if !path.exists() {
        return Ok(false);
    }

    if destination_existed {
        let mut removed_any = false;
        for entry in std::fs::read_dir(path)
            .with_context(|| format!("Could not inspect partially downloaded snapshot at `{local_path}`"))?
        {
            let entry = entry?;
            let entry_path = entry.path();
            let file_type = entry.file_type()?;
            removed_any = true;

            if file_type.is_dir() {
                std::fs::remove_dir_all(&entry_path).with_context(|| {
                    format!(
                        "Could not remove partially downloaded directory `{}`",
                        entry_path.display()
                    )
                })?;
            } else {
                std::fs::remove_file(&entry_path).with_context(|| {
                    format!(
                        "Could not remove partially downloaded file `{}`",
                        entry_path.display()
                    )
                })?;
            }
        }

        Ok(removed_any)
    } else {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("Could not remove partially downloaded snapshot folder `{local_path}`"))?;
        Ok(true)
    }
}

pub async fn remove_remote_git_ftp_log(
    app: &AppHandle,
    draft: &ProfileDraft,
    password: &str,
) -> Result<RemoteCleanupResult> {
    if executables::resolve_lftp(app).is_none() {
        return Err(anyhow!(
            "Removing the remote .git-ftp.log requires `lftp`, but it was not found in PATH or common install locations."
        ));
    }

    if password.trim().is_empty() {
        return Err(anyhow!("A password is required before changing files on the remote FTP site."));
    }

    let mut messages = Vec::new();
    validate_draft(draft, true, &mut messages);
    if let Some(error) = messages
        .into_iter()
        .find(|item| matches!(item.kind, ValidationMessageKind::Error))
    {
        return Err(anyhow!(error.text));
    }

    let profile = merge_profile(None, draft.clone(), Uuid::new_v4().to_string());
    let lftp_path = executables::require_lftp(app)
        .context("Could not resolve lftp from the bundled resources or the local system.")?;
    let remote_log = remote_git_ftp_log_path(&profile);
    let lftp_command = build_remote_cleanup_script(&profile, password);
    let command_preview = format!("{} --norc <stdin>", lftp_path.to_string_lossy());

    let mut child = Command::new(&lftp_path)
        .args(["--norc"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::piped())
        .env("PATH", build_runtime_path(app)?)
        .spawn()
        .with_context(|| format!("Failed to remove remote state file `{remote_log}`."))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Failed to open stdin for the `lftp` cleanup command."))?;
    stdin
        .write_all(lftp_command.as_bytes())
        .await
        .with_context(|| "Failed to send the cleanup script to `lftp`.")?;
    stdin
        .shutdown()
        .await
        .with_context(|| "Failed to finish the cleanup script for `lftp`.")?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .with_context(|| format!("Failed to remove remote state file `{remote_log}`."))?;

    Ok(RemoteCleanupResult {
        success: output.status.success(),
        command_preview,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code(),
    })
}

async fn read_snapshot_stream<R>(
    app: AppHandle,
    operation_id: String,
    command_preview: String,
    pipe: R,
    stream: LogStream,
    buffer: Arc<Mutex<Vec<String>>>,
    tracker: Arc<Mutex<SnapshotProgressTracker>>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(pipe).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        buffer.lock().await.push(line.clone());
        let update = tracker.lock().await.observe(&line, &stream);
        emit_snapshot_progress(
            &app,
            &operation_id,
            SnapshotProgressStatus::Running,
            update.progress,
            &update.title,
            &update.detail,
            Some(command_preview.clone()),
            Some(stream.clone()),
            Some(line),
        );
    }
}

fn emit_snapshot_progress(
    app: &AppHandle,
    operation_id: &str,
    status: SnapshotProgressStatus,
    progress: u8,
    title: impl Into<String>,
    detail: impl Into<String>,
    command_preview: Option<String>,
    stream: Option<LogStream>,
    line: Option<String>,
) {
    let _ = app.emit(
        SNAPSHOT_PROGRESS_EVENT,
        SnapshotProgressEvent {
            operation_id: operation_id.to_string(),
            status,
            progress,
            title: title.into(),
            detail: detail.into(),
            command_preview,
            stream,
            line,
        },
    );
}

struct SnapshotProgressTracker {
    progress: u8,
}

struct SnapshotProgressUpdate {
    progress: u8,
    title: String,
    detail: String,
}

impl SnapshotProgressTracker {
    fn new() -> Self {
        Self { progress: 18 }
    }

    fn observe(&mut self, line: &str, stream: &LogStream) -> SnapshotProgressUpdate {
        let lowered = line.to_ascii_lowercase();
        let (floor, ceiling, step, title) = if lowered.contains("initialized empty git repository") {
            (26, 34, 6, "Preparing local repository…")
        } else if lowered.contains("certificate verification")
            || lowered.contains("fatal error")
            || lowered.contains("fatal:")
            || matches!(stream, LogStream::Stderr)
        {
            (self.progress.max(24), self.progress.max(32), 0, "Download reported an error.")
        } else if lowered.contains("mirror")
            || lowered.contains("transferring")
            || lowered.contains("get ")
            || lowered.contains("making directory")
            || lowered.contains("mkdir")
            || lowered.contains("put ")
        {
            (40, 94, 4, "Downloading remote files…")
        } else if lowered.contains("cd:")
            || lowered.contains("listing")
            || lowered.contains("ftp")
            || lowered.contains("connected")
        {
            (30, 78, 3, "Connecting to the FTP server…")
        } else {
            (34, 90, 2, "Downloading remote files…")
        };

        self.progress = self.progress.max(floor);
        if step > 0 {
            self.progress = self.progress.saturating_add(step).min(ceiling);
        }

        SnapshotProgressUpdate {
            progress: self.progress,
            title: title.to_string(),
            detail: line.to_string(),
        }
    }
}

pub fn apply_git_config_defaults(app: &AppHandle, repo_path: &str, profile: &DeploymentProfile) -> Result<()> {
    let url = profile_url(profile);
    git_config_set(app, repo_path, "git-ftp.url", &url)?;
    git_config_set(app, repo_path, "git-ftp.user", &profile.username)?;

    set_optional_git_config(app, repo_path, "git-ftp.syncroot", profile.flags.syncroot.as_deref())?;
    set_optional_git_config(
        app,
        repo_path,
        "git-ftp.remote-root",
        profile.flags.remote_root.as_deref(),
    )?;
    git_config_set(app, repo_path, "git-ftp.insecure", bool_to_git(profile.flags.insecure))?;
    git_config_set(
        app,
        repo_path,
        "git-ftp.disable-epsv",
        bool_to_git(profile.flags.disable_epsv),
    )?;

    Ok(())
}

fn run_dry_probe(app: &AppHandle, repo_path: &str, draft: &ProfileDraft, secret: &str) -> Result<CommandProbeResult> {
    let git_ftp = executables::require_git_ftp(app)
        .context("Could not resolve git-ftp from the bundled resources or the local system.")?;
    let profile = DeploymentProfile {
        id: String::new(),
        name: draft.name.clone(),
        protocol: draft.protocol.clone(),
        host: draft.host.clone(),
        port: draft.port,
        username: draft.username.clone(),
        remote_path: draft.remote_path.clone(),
        secret_ref: String::new(),
        use_git_config_defaults: draft.use_git_config_defaults,
        flags: draft.flags.clone(),
        last_deployed_at: None,
        created_at: String::new(),
        updated_at: String::new(),
    };
    let mut args = git_ftp.args_prefix.clone();
    args.extend(build_args(
        &profile,
        &RunAction::Push,
        &RunOptions {
            dry_run: true,
            verbose: false,
        },
    ));

    let output = StdCommand::new(git_ftp.executable)
        .args(args)
        .current_dir(repo_path)
        .envs(build_command_envs(app, &profile, secret)?)
        .output()?;

    Ok(CommandProbeResult {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn validate_draft(draft: &ProfileDraft, secret_present: bool, messages: &mut Vec<ValidationMessage>) {
    if draft.name.trim().is_empty() {
        messages.push(message(
            ValidationMessageKind::Error,
            "Profile name is required.",
        ));
    }
    if draft.host.trim().is_empty() {
        messages.push(message(
            ValidationMessageKind::Error,
            "FTP host is required.",
        ));
    }
    if draft.username.trim().is_empty() {
        messages.push(message(
            ValidationMessageKind::Error,
            "Username is required.",
        ));
    }
    if draft.port == 0 {
        messages.push(message(
            ValidationMessageKind::Error,
            "Port must be greater than zero.",
        ));
    }
    if !secret_present {
        messages.push(message(
            ValidationMessageKind::Error,
            "A password must be saved in secure storage before this profile can run.",
        ));
    }

    for value in [&draft.name, &draft.host, &draft.username, &draft.remote_path] {
        if contains_control_chars(value) {
            messages.push(message(
                ValidationMessageKind::Error,
                "Fields cannot contain control characters.",
            ));
            break;
        }
    }
}

fn first_unsafe_deploy_path_char(path: &str) -> Option<char> {
    path.chars()
        .find(|character| UNSAFE_DEPLOY_PATH_CHARS.contains(character))
}

fn format_encoded_sync_notice(paths: &[String]) -> String {
    let preview = paths
        .iter()
        .take(3)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");

    let remainder = paths.len().saturating_sub(3);
    let suffix = if remainder == 0 {
        String::new()
    } else if remainder == 1 {
        ", plus 1 more path".to_string()
    } else {
        format!(", plus {remainder} more paths")
    };

    format!(
        "Tracked files include exact names that require encoded-path upload handling. The app will use its built-in fallback sync for paths that use: {}. Example affected paths: {}{}.",
        UNSAFE_DEPLOY_PATH_CHARS
            .iter()
            .map(|character| format!("`{character}`"))
            .collect::<Vec<_>>()
            .join(", "),
        preview,
        suffix
    )
}

fn should_use_encoded_sync_fallback(
    repo_path: &str,
    profile: &DeploymentProfile,
    action: &RunAction,
) -> Result<bool> {
    if !matches!(action, RunAction::Init | RunAction::Push) {
        return Ok(false);
    }

    Ok(!find_paths_requiring_encoded_sync(repo_path, profile.flags.syncroot.as_deref())?.is_empty())
}

fn find_paths_requiring_encoded_sync(repo_path: &str, syncroot: Option<&str>) -> Result<Vec<String>> {
    let tracked_files = git::tracked_files(repo_path)?;
    Ok(filter_paths_requiring_encoded_sync(tracked_files.iter().map(String::as_str), syncroot))
}

fn filter_paths_requiring_encoded_sync<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    syncroot: Option<&str>,
) -> Vec<String> {
    let syncroot_prefix = normalized_syncroot_prefix(syncroot);

    paths.into_iter()
        .filter(|path| {
            syncroot_prefix
                .as_deref()
                .map(|prefix| path.starts_with(prefix))
                .unwrap_or(true)
        })
        .filter(|path| first_unsafe_deploy_path_char(path).is_some())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalized_syncroot_prefix(syncroot: Option<&str>) -> Option<String> {
    let trimmed = syncroot?.trim();
    if trimmed.is_empty() || trimmed == "." {
        return None;
    }

    let without_prefix = trimmed.trim_start_matches("./").trim_end_matches('/');
    if without_prefix.is_empty() {
        None
    } else {
        Some(format!("{without_prefix}/"))
    }
}

fn build_encoded_sync_command_spec(
    app: &AppHandle,
    repo_path: &str,
    profile: &DeploymentProfile,
    action: RunAction,
    options: &RunOptions,
    secret: String,
) -> Result<CommandSpec> {
    let bash_path = executables::resolve_bash().ok_or_else(|| {
        anyhow!("Could not resolve `bash`, which is required for encoded-path FTP sync fallback.")
    })?;

    let script_path = write_encoded_sync_script()?;
    let command_preview = format!(
        "{} <encoded-path git-ftp fallback> {}",
        bash_path.to_string_lossy(),
        action.as_str()
    );
    let record_id = Uuid::new_v4().to_string();

    Ok(CommandSpec {
        run_id: record_id.clone(),
        executable: bash_path,
        args: vec![script_path.to_string_lossy().to_string()],
        envs: build_encoded_sync_envs(app, profile, &action, options, &secret)?,
        current_dir: PathBuf::from(repo_path),
        record: crate::models::RunRecord {
            id: record_id,
            repo_path: repo_path.to_string(),
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            action,
            started_at: timestamp_now(),
            finished_at: None,
            success: false,
            exit_code: None,
            command_preview,
            logs: Vec::new(),
            cancelled: false,
            changed_files: Vec::new(),
        },
    })
}

fn write_encoded_sync_script() -> Result<PathBuf> {
    let script_path = std::env::temp_dir().join(format!("git-ftp-desktop-encoded-sync-{}.sh", Uuid::new_v4()));
    fs::write(&script_path, encoded_sync_script())
        .with_context(|| format!("Could not write fallback sync script to `{}`", script_path.display()))?;
    Ok(script_path)
}

fn build_encoded_sync_envs(
    app: &AppHandle,
    profile: &DeploymentProfile,
    action: &RunAction,
    options: &RunOptions,
    secret: &str,
) -> Result<Vec<(String, String)>> {
    let mut envs = vec![
        ("GFD_ACTION".to_string(), action.as_str().to_string()),
        ("GFD_PROTOCOL".to_string(), profile.protocol.scheme().to_string()),
        ("GFD_HOST".to_string(), profile.host.trim().to_string()),
        ("GFD_PORT".to_string(), profile.port.to_string()),
        ("GFD_USER".to_string(), profile.username.clone()),
        ("GFD_PASSWORD".to_string(), secret.to_string()),
        ("GFD_REMOTE_PATH".to_string(), effective_remote_path(profile)),
        (
            "GFD_SYNCROOT".to_string(),
            profile
                .flags
                .syncroot
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(".")
                .to_string(),
        ),
        (
            "GFD_ACTIVE_MODE".to_string(),
            if profile.flags.active_mode { "1" } else { "0" }.to_string(),
        ),
        (
            "GFD_DISABLE_EPSV".to_string(),
            if profile.flags.disable_epsv { "1" } else { "0" }.to_string(),
        ),
        (
            "GFD_INSECURE".to_string(),
            if profile.flags.insecure { "1" } else { "0" }.to_string(),
        ),
        (
            "GFD_AUTO_INIT".to_string(),
            if profile.flags.auto_init { "1" } else { "0" }.to_string(),
        ),
        (
            "GFD_VERBOSE".to_string(),
            if options.verbose { "1" } else { "0" }.to_string(),
        ),
        (
            "GFD_DRY_RUN".to_string(),
            if options.dry_run { "1" } else { "0" }.to_string(),
        ),
    ];
    envs.push(("PATH".to_string(), build_runtime_path(app)?.to_string_lossy().to_string()));
    Ok(envs)
}

fn effective_remote_path(profile: &DeploymentProfile) -> String {
    let mut remote_path = profile.remote_path.trim().trim_start_matches('/').to_string();

    if let Some(remote_root) = profile
        .flags
        .remote_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let mut normalized_root = remote_root.trim_start_matches('/').to_string();
        if !normalized_root.ends_with('/') {
            normalized_root.push('/');
        }
        remote_path = format!("{normalized_root}{remote_path}");
    }

    if !remote_path.is_empty() && !remote_path.ends_with('/') {
        remote_path.push('/');
    }

    remote_path
}

fn encoded_sync_script() -> &'static str {
    r#"#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH="$0"
GFD_TMP_DIR="$(mktemp -d -t git-ftp-desktop-encoded-sync-XXXXXX)"
TMP_UPLOAD="$GFD_TMP_DIR/upload.zlist"
TMP_DELETE="$GFD_TMP_DIR/delete.zlist"
TMP_INCLUDE="$GFD_TMP_DIR/include.zlist"

cleanup() {
  rm -rf "$GFD_TMP_DIR"
  rm -f "$SCRIPT_PATH"
}

trap cleanup EXIT

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fatal "$1 is required for encoded-path deployment fallback."
}

fatal() {
  printf 'fatal: %s\n' "$1" >&2
  exit 4
}

print_info() {
  printf '%s\n' "$1"
}

write_log() {
  if [ "${GFD_VERBOSE:-0}" = "1" ]; then
    printf '%s %s\n' "$(date)" "$1"
  fi
}

urlencode() {
  local string="${1}"
  local keepset='[-_.~a-zA-Z0-9]'
  [ $# -gt 1 ] && keepset="${2}"
  local encoded=""
  local pos c o
  local strlen=${#string}
  for (( pos=0; pos<strlen; pos++ )); do
    c=${string:$pos:1}
    case "$c" in
      $keepset) o="${c}" ;;
      *) printf -v o '%%%02x' "'$c" ;;
    esac
    encoded+="${o}"
  done
  printf '%s' "${encoded}"
}

bool_env() {
  [ "${1:-0}" = "1" ]
}

default_curl_args() {
  CURL_ARGS=(--globoff --silent --show-error)
  if bool_env "${GFD_ACTIVE_MODE:-0}"; then
    CURL_ARGS+=(-P "-")
  elif bool_env "${GFD_DISABLE_EPSV:-0}"; then
    CURL_ARGS+=(--disable-epsv)
  fi
  if [ "${GFD_PROTOCOL}" = "ftpes" ]; then
    CURL_ARGS+=(--ssl)
  fi
  if bool_env "${GFD_INSECURE:-0}"; then
    CURL_ARGS+=(-k)
  fi
}

build_remote_base_url() {
  local curl_protocol="${GFD_PROTOCOL}"
  if [ "$curl_protocol" = "ftpes" ]; then
    curl_protocol="ftp"
  fi

  local enc_user enc_pass
  enc_user="$(urlencode "${GFD_USER}")"
  enc_pass="$(urlencode "${GFD_PASSWORD}")"
  REMOTE_BASE_URL="${curl_protocol}://${enc_user}:${enc_pass}@${GFD_HOST}:${GFD_PORT}"
}

set_syncroot() {
  GIT_SYNCROOT="${GFD_SYNCROOT:-.}"
  [ -d "$GIT_SYNCROOT" ] || fatal "'$GIT_SYNCROOT' is not a directory."

  if [ "$GIT_SYNCROOT" = "." ] || [ "$GIT_SYNCROOT" = "./" ]; then
    SYNCROOT_PREFIX=""
  else
    SYNCROOT_PREFIX="${GIT_SYNCROOT#./}"
    SYNCROOT_PREFIX="${SYNCROOT_PREFIX%/}/"
  fi
}

relative_destination() {
  local file="$1"
  local relative="$file"
  if [ -n "$SYNCROOT_PREFIX" ] && [[ "$relative" == "$SYNCROOT_PREFIX"* ]]; then
    relative="${relative#"$SYNCROOT_PREFIX"}"
  fi
  relative="${relative#./}"
  printf '%s' "$relative"
}

remote_url_for_path() {
  local relative
  relative="$(relative_destination "$1")"
  printf '%s/%s%s' "$REMOTE_BASE_URL" "${GFD_REMOTE_PATH}" "$(urlencode "$relative" '/-_.~a-zA-Z0-9')"
}

get_file_content() {
  default_curl_args
  curl "${CURL_ARGS[@]}" "$(remote_url_for_path "$1")"
}

check_is_dirty_repository() {
  if [ -n "$(git status -uno --porcelain)" ]; then
    fatal "Dirty repository. Commit or stash your changes first."
  fi
}

glob_filter() {
  local patterns="$1"
  while IFS= read -r -d '' filename; do
    local hasmatch=0
    while IFS= read -r pattern; do
      case "$filename" in
        $pattern) hasmatch=1; break ;;
      esac
    done < "$patterns"
    test "$hasmatch" = 1 || printf '%s\0' "$filename"
  done
}

filter_file() {
  local patterns="$1"
  local target="$2"
  glob_filter "$patterns" < "$target" > "$GFD_TMP_DIR/filtered.zlist"
  mv "$GFD_TMP_DIR/filtered.zlist" "$target"
}

filter_ignore_files() {
  [ -f ".git-ftp-ignore" ] || return 0
  local patterns="$GFD_TMP_DIR/ignore-patterns"
  grep -v '^#.*$\|^\s*$' ".git-ftp-ignore" | tr -d '\r' > "$patterns" || true
  [ -s "$patterns" ] || return 0
  filter_file "$patterns" "$TMP_UPLOAD"
  filter_file "$patterns" "$TMP_DELETE"
}

add_include_file() {
  local target="$1"
  if [ -e "$target" ]; then
    if [ -d "$target" ]; then
      find "$target" -type f -print0 >> "$TMP_UPLOAD"
    elif [ -f "$target" ]; then
      printf '%s\0' "$target" >> "$TMP_UPLOAD"
    fi
  else
    if ! printf '%s' "$target" | grep -q '/$'; then
      printf '%s\0' "$target" >> "$TMP_DELETE"
    fi
  fi
}

add_include_files() {
  [ -f ".git-ftp-include" ] || return 0
  grep -v '^#.*$\|^\s*$' ".git-ftp-include" | tr -d '\r' > "$TMP_INCLUDE" || true
  [ -s "$TMP_INCLUDE" ] || return 0
  (grep '^!' "$TMP_INCLUDE" || true) | sed 's/^!//' | while IFS= read -r target; do
    add_include_file "$target"
  done

  local against="${DEPLOYED_SHA1:-"$(git hash-object -t tree /dev/null)"}"
  (grep ':' "$TMP_INCLUDE" || true) | while IFS= read -r line; do
    local target="${line%%:*}"
    local source="${line#*:}"
    if printf '%s' "$source" | grep -q '^/'; then
      source="${source#/}"
    elif [ -n "$SYNCROOT_PREFIX" ]; then
      source="${SYNCROOT_PREFIX}${source}"
    fi

    if ! git diff --quiet "$against" -- "$source"; then
      add_include_file "$target"
    fi
  done
}

list_all_files() {
  git ls-files -z -- "$GIT_SYNCROOT" > "$TMP_UPLOAD"
  : > "$TMP_DELETE"
}

list_changed_files() {
  git diff --name-only --no-renames --diff-filter=AM -z "$DEPLOYED_SHA1" -- "$GIT_SYNCROOT" > "$TMP_UPLOAD"
  git diff --name-only --no-renames --diff-filter=D -z "$DEPLOYED_SHA1" -- "$GIT_SYNCROOT" > "$TMP_DELETE"

  if [ "$LOCAL_SHA1" = "$DEPLOYED_SHA1" ]; then
    print_info "No changed files for ${GFD_HOST}/${GFD_REMOTE_PATH}. Everything up-to-date."
    exit 0
  fi

  if [ ! -s "$TMP_UPLOAD" ] && [ ! -s "$TMP_DELETE" ]; then
    write_log "No changed files, but different commit ID. Changed files ignored or commit amended."
  fi
}

set_changed_files() {
  if [ "${IGNORE_DEPLOYED:-0}" = "1" ]; then
    write_log "Taking all files."
    list_all_files
  else
    list_changed_files
  fi
  add_include_files
  filter_ignore_files
}

upload_file() {
  local file="$1"
  default_curl_args
  local remote_url
  remote_url="$(remote_url_for_path "$file")"
  CURL_ARGS+=(-T "$file" --ftp-create-dirs "$remote_url")
  curl "${CURL_ARGS[@]}" >/dev/null
}

delete_file() {
  local relative
  relative="$(relative_destination "$1")"
  default_curl_args
  CURL_ARGS+=(-Q "DELE ${GFD_REMOTE_PATH}${relative}" "$REMOTE_BASE_URL")
  if ! curl "${CURL_ARGS[@]}" >/dev/null 2>&1; then
    write_log "WARNING: Could not delete ${GFD_REMOTE_PATH}${relative}, continuing."
  fi
}

upload_local_sha1() {
  print_info "Updating remote deployment marker."
  if bool_env "${GFD_DRY_RUN:-0}"; then
    print_info "Dry run: would upload .git-ftp.log with ${LOCAL_SHA1}."
    return
  fi

  default_curl_args
  CURL_ARGS+=(-T - --ftp-create-dirs "$(remote_url_for_path ".git-ftp.log")")
  printf '%s' "$LOCAL_SHA1" | curl "${CURL_ARGS[@]}" >/dev/null
}

handle_file_sync() {
  if [ ! -s "$TMP_UPLOAD" ] && [ ! -s "$TMP_DELETE" ]; then
    print_info "There are no files to sync."
    return
  fi

  local total_items
  total_items=$(( $(tr -cd '\0' < "$TMP_UPLOAD" | wc -c | tr -d ' ') + $(tr -cd '\0' < "$TMP_DELETE" | wc -c | tr -d ' ') ))
  local done_items=0
  print_info "${total_items} file$([ "$total_items" -ne 1 ] && printf 's') to sync:"

  while IFS= read -r -d '' file_name; do
    done_items=$((done_items + 1))
    print_info "[$done_items of $total_items] Uploading '$file_name'."
    if ! bool_env "${GFD_DRY_RUN:-0}"; then
      upload_file "$file_name"
    fi
  done < "$TMP_UPLOAD"

  while IFS= read -r -d '' file_name; do
    done_items=$((done_items + 1))
    print_info "[$done_items of $total_items] Deleting '$file_name'."
    if ! bool_env "${GFD_DRY_RUN:-0}"; then
      delete_file "$file_name"
    fi
  done < "$TMP_DELETE"
}

set_local_sha1() {
  LOCAL_SHA1="$(git log -n 1 --pretty=format:%H)"
}

set_deployed_sha1_failable() {
  DEPLOYED_SHA1="$(get_file_content ".git-ftp.log" 2>/dev/null || true)"
}

run_init() {
  set_deployed_sha1_failable
  if [ -n "$DEPLOYED_SHA1" ]; then
    fatal "Commit found, use 'push' to sync."
  fi
  IGNORE_DEPLOYED=1
  set_local_sha1
  set_changed_files
  handle_file_sync
  upload_local_sha1
}

run_push() {
  set_deployed_sha1_failable
  if [ -z "$DEPLOYED_SHA1" ]; then
    if bool_env "${GFD_AUTO_INIT:-0}"; then
      IGNORE_DEPLOYED=1
    else
      fatal "Could not get last commit. Use 'init' for the initial push."
    fi
  fi
  set_local_sha1
  set_changed_files
  handle_file_sync
  upload_local_sha1
}

main() {
  require_cmd git
  require_cmd curl
  build_remote_base_url
  set_syncroot
  check_is_dirty_repository

  case "${GFD_ACTION}" in
    init) run_init ;;
    push) run_push ;;
    *) fatal "Encoded-path fallback only supports init and push." ;;
  esac
}

main "$@"
"#
}

fn build_args(profile: &DeploymentProfile, action: &RunAction, options: &RunOptions) -> Vec<String> {
    let mut args = vec![action.as_str().to_string()];

    if options.dry_run {
        args.push("--dry-run".to_string());
    }
    if options.verbose {
        args.push("-v".to_string());
    }
    if profile.flags.active_mode {
        args.push("--active".to_string());
    }
    if profile.flags.disable_epsv {
        args.push("--disable-epsv".to_string());
    }
    if profile.flags.insecure {
        args.push("--insecure".to_string());
    }
    if profile.flags.auto_init {
        args.push("--auto-init".to_string());
    }
    if profile.flags.use_all {
        args.push("--all".to_string());
    }
    if matches!(action, RunAction::Download) {
        args.push("--force".to_string());
    }
    if let Some(syncroot) = profile.flags.syncroot.as_ref().filter(|value| !value.trim().is_empty()) {
        args.push("--syncroot".to_string());
        args.push(syncroot.trim().to_string());
    }
    if let Some(remote_root) = profile
        .flags
        .remote_root
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        args.push("--remote-root".to_string());
        args.push(remote_root.trim().to_string());
    }

    if !profile.use_git_config_defaults {
        args.push("--user".to_string());
        args.push(profile.username.clone());
        args.push(profile_url(profile));
    }

    args
}

fn build_snapshot_args(profile: &DeploymentProfile, local_path: &str) -> Vec<String> {
    let mut args = vec!["snapshot".to_string()];

    if profile.flags.active_mode {
        args.push("--active".to_string());
    }
    if profile.flags.disable_epsv {
        args.push("--disable-epsv".to_string());
    }
    if profile.flags.insecure {
        args.push("--insecure".to_string());
    }
    args.push(profile_url(profile));
    args.push(local_path.to_string());

    args
}

fn build_remote_cleanup_script(profile: &DeploymentProfile, password: &str) -> String {
    let mut parts = Vec::new();
    if profile.protocol.scheme() == "ftpes" {
        parts.push("set ftp:ssl-force true".to_string());
        parts.push("set ftp:ssl-protect-data true".to_string());
        parts.push("set ftp:ssl-protect-list true".to_string());
    }
    if profile.flags.insecure {
        parts.push("set ssl:verify-certificate no".to_string());
    }
    parts.push("set ftp:list-options -a".to_string());
    if profile.flags.disable_epsv {
        parts.push("set ftp:prefer-epsv false".to_string());
    }
    parts.push(format!(
        "open -u {},{} {}",
        shell_quote(&profile.username),
        shell_quote(password),
        shell_quote(&lftp_base_url(profile))
    ));
    if let Some(remote_path) = normalized_remote_path(profile).filter(|value| *value != "/") {
        parts.push(format!("cd {}", shell_quote(remote_path)));
    }
    parts.push("rm .git-ftp.log".to_string());
    parts.push("bye".to_string());
    format!("{}\n", parts.join("\n"))
}

fn build_command_preview(draft: &ProfileDraft, action: RunAction, options: &RunOptions) -> String {
    let profile = DeploymentProfile {
        id: String::new(),
        name: draft.name.clone(),
        protocol: draft.protocol.clone(),
        host: draft.host.clone(),
        port: draft.port,
        username: draft.username.clone(),
        remote_path: draft.remote_path.clone(),
        secret_ref: String::new(),
        use_git_config_defaults: draft.use_git_config_defaults,
        flags: draft.flags.clone(),
        last_deployed_at: None,
        created_at: String::new(),
        updated_at: String::new(),
    };
    format_command_preview(&PathBuf::from("git-ftp"), &build_args(&profile, &action, options))
}

fn profile_url(profile: &DeploymentProfile) -> String {
    let path = profile.remote_path.trim();
    let normalized_path = if path.is_empty() {
        String::new()
    } else if path.starts_with('/') || path.starts_with("~/") {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!(
        "{}://{}:{}{}",
        profile.protocol.scheme(),
        profile.host.trim(),
        profile.port,
        normalized_path
    )
}

fn lftp_base_url(profile: &DeploymentProfile) -> String {
    let protocol = if profile.protocol.scheme() == "ftpes" {
        "ftp"
    } else {
        profile.protocol.scheme()
    };

    format!(
        "{}://{}:{}",
        protocol,
        profile.host.trim(),
        profile.port
    )
}

fn normalized_remote_path(profile: &DeploymentProfile) -> Option<&str> {
    let path = profile.remote_path.trim();
    if path.is_empty() {
        None
    } else if path.starts_with('/') || path.starts_with("~/") {
        Some(path)
    } else {
        None
    }
}

fn remote_git_ftp_log_path(profile: &DeploymentProfile) -> String {
    match normalized_remote_path(profile) {
        Some(path) if path.ends_with('/') => format!("{path}.git-ftp.log"),
        Some(path) => format!("{path}/.git-ftp.log"),
        None => "/.git-ftp.log".to_string(),
    }
}

pub fn format_command_preview(executable: &PathBuf, args: &[String]) -> String {
    let mut parts = vec![executable.to_string_lossy().to_string()];
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            parts.push("<redacted>".to_string());
            redact_next = false;
            continue;
        }
        if arg == "--password-command" {
            parts.push(arg.clone());
            redact_next = true;
            continue;
        }
        parts.push(arg.clone());
    }
    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn build_command_envs(app: &AppHandle, profile: &DeploymentProfile, secret: &str) -> Result<Vec<(String, String)>> {
    let entries = [
        ("git-ftp.password".to_string(), secret.to_string()),
        ("git-ftp.user".to_string(), profile.username.clone()),
        ("git-ftp.url".to_string(), profile_url(profile)),
    ];

    let mut envs = vec![("GIT_CONFIG_COUNT".to_string(), entries.len().to_string())];
    for (index, (key, value)) in entries.into_iter().enumerate() {
        envs.push((format!("GIT_CONFIG_KEY_{index}"), key));
        envs.push((format!("GIT_CONFIG_VALUE_{index}"), value));
    }
    envs.push(("PATH".to_string(), build_runtime_path(app)?.to_string_lossy().to_string()));
    Ok(envs)
}

fn git_config_set(_app: &AppHandle, repo_path: &str, key: &str, value: &str) -> Result<()> {
    let git_path = executables::require_git()
        .context("Could not resolve git in PATH or common install locations.")?;
    let status = StdCommand::new(&git_path)
        .args(["config", "--local", key, value])
        .current_dir(repo_path)
        .status()
        .with_context(|| format!("Failed to write git config key `{key}`"))?;

    if !status.success() {
        return Err(anyhow!("Could not write git config key `{key}`."));
    }
    Ok(())
}

fn set_optional_git_config(app: &AppHandle, repo_path: &str, key: &str, value: Option<&str>) -> Result<()> {
    match value.filter(|value| !value.trim().is_empty()) {
        Some(value) => git_config_set(app, repo_path, key, value),
        None => {
            if let Ok(git_path) = executables::require_git() {
                let _ = StdCommand::new(&git_path)
                    .args(["config", "--local", "--unset", key])
                    .current_dir(repo_path)
                    .status();
            }
            Ok(())
        }
    }
}

fn bool_to_git(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn contains_control_chars(value: &str) -> bool {
    value.chars().any(|char| char.is_control() && char != '\n' && char != '\t')
}

fn message(kind: ValidationMessageKind, text: impl Into<String>) -> ValidationMessage {
    ValidationMessage {
        kind,
        text: text.into(),
    }
}

struct TempSnapshotRepo {
    path: PathBuf,
}

impl TempSnapshotRepo {
    fn create() -> Result<Self> {
        let path = std::env::temp_dir().join(format!("git-ftp-desktop-snapshot-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path)
            .with_context(|| format!("Could not create temporary snapshot config directory `{}`", path.display()))?;

        let git_path = executables::require_git()
            .context("Could not resolve git in PATH or common install locations.")?;
        let init_status = StdCommand::new(&git_path)
            .args(["init", "-q"])
            .current_dir(&path)
            .status()
            .with_context(|| "Failed to initialize temporary git repository for snapshot.")?;

        if !init_status.success() {
            return Err(anyhow!("Could not initialize temporary git repository for snapshot."));
        }

        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSnapshotRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn build_runtime_path(app: &AppHandle) -> Result<OsString> {
    let mut entries = Vec::new();
    if let Some(lftp_path) = executables::resolve_lftp(app) {
        if let Some(parent) = lftp_path.parent() {
            entries.push(parent.to_path_buf());
        }
    }

    executables::prepend_to_path(app, &entries)
}

#[cfg(test)]
mod tests {
    use super::{
        filter_paths_requiring_encoded_sync, first_unsafe_deploy_path_char, format_encoded_sync_notice,
        normalized_syncroot_prefix, UNSAFE_DEPLOY_PATH_CHARS,
    };

    #[test]
    fn allows_hash_character_in_upload_paths() {
        assert_eq!(
            first_unsafe_deploy_path_char("imagens/produtos/YM35TK#K041.jpg"),
            None
        );
    }

    #[test]
    fn ignores_normal_deploy_paths() {
        assert_eq!(
            first_unsafe_deploy_path_char("imagens/produtos/YM35TK-K041.jpg"),
            None
        );
    }

    #[test]
    fn formats_a_helpful_validation_message() {
        let message = format_encoded_sync_notice(&[
            "one bad/file #1.txt".to_string(),
            "two bad/file%2.txt".to_string(),
            "three bad/file?.txt".to_string(),
            "four bad/file[4].txt".to_string(),
        ]);

        assert!(message.contains("encoded-path upload handling"));
        assert!(message.contains("`one bad/file #1.txt`"));
        assert!(message.contains("plus 1 more path"));

        for character in UNSAFE_DEPLOY_PATH_CHARS {
            assert!(message.contains(&format!("`{character}`")));
        }
    }

    #[test]
    fn scopes_encoded_sync_paths_to_syncroot() {
        let paths = filter_paths_requiring_encoded_sync(
            [
                "imagens/produtos/CHAVE ATRA.jpg",
                "docs/release notes.txt",
                "imagens/produtos/clean-file.jpg",
            ],
            Some("imagens/produtos"),
        );

        assert_eq!(paths, vec!["imagens/produtos/CHAVE ATRA.jpg".to_string()]);
    }

    #[test]
    fn normalizes_syncroot_prefix() {
        assert_eq!(normalized_syncroot_prefix(Some("./public/assets/")), Some("public/assets/".to_string()));
        assert_eq!(normalized_syncroot_prefix(Some(".")), None);
        assert_eq!(normalized_syncroot_prefix(None), None);
    }
}
