use crate::diagnostics;
use crate::git;
use crate::git_ftp;
use crate::models::{
    CommitResult, DebugReport, DeploymentProfile, ProfileDraft, RemoteCleanupRequest, RemoteCleanupResult,
    RunRequest, SavedRepository, SnapshotBootstrapRequest, SnapshotBootstrapResult,
};
use crate::state::AppState;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use uuid::Uuid;

fn into_string_error<T, E>(result: Result<T, E>) -> Result<T, String>
where
    E: std::fmt::Display,
{
    result.map_err(|error| error.to_string())
}

fn persist_secret(state: &AppState, secret_ref: &str, password: &str) -> Result<(), String> {
    into_string_error(state.secrets.save(secret_ref, password))?;
    Ok(())
}

async fn confirm_destructive_action(
    app: AppHandle,
    title: &str,
    message: &str,
) -> Result<bool, String> {
    let title = title.to_string();
    let message = message.to_string();

    tokio::task::spawn_blocking(move || {
        app.dialog()
            .message(message)
            .title(title)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Continue".to_string(),
                "Cancel".to_string(),
            ))
            .blocking_show()
    })
    .await
    .map_err(|error| format!("Could not show confirmation dialog. {error}"))
}

fn reject_symlink_path(repo_path: &str) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(repo_path)
        .map_err(|error| format!("Could not inspect `{repo_path}` before deleting it. {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err(
            "Deleting symlinked repository paths is blocked for safety. Remove the symlink manually if needed."
                .to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn startup_diagnostics(app: AppHandle) -> Result<crate::models::StartupDiagnostics, String> {
    Ok(diagnostics::startup_diagnostics(&app))
}

#[tauri::command]
pub async fn validate_repo(repo_path: String, state: State<'_, AppState>) -> Result<crate::models::RepoInfo, String> {
    let repo = into_string_error(git::inspect_repository(&repo_path))?;
    if repo.is_git_repo {
        into_string_error(state.profiles.mark_repo_opened(&repo_path).await)?;
    }
    Ok(repo)
}

#[tauri::command]
pub async fn list_profiles(
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<DeploymentProfile>, String> {
    into_string_error(state.profiles.list_profiles(&repo_path).await)
}

#[tauri::command]
pub async fn list_known_repositories(
    state: State<'_, AppState>,
) -> Result<Vec<SavedRepository>, String> {
    into_string_error(state.profiles.list_known_repositories().await)
}

#[tauri::command]
pub async fn remove_known_repository(
    app: AppHandle,
    repo_path: String,
    delete_folder: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !into_string_error(state.profiles.has_repository(&repo_path).await)? {
        return Err("Only repositories already registered with the app can be removed.".to_string());
    }

    if delete_folder {
        let path = std::path::Path::new(&repo_path);
        if path.exists() {
            reject_symlink_path(&repo_path)?;

            let confirmed = confirm_destructive_action(
                app,
                "Delete repository folder",
                "Delete this repository folder from disk? This cannot be undone.",
            )
            .await?;

            if !confirmed {
                return Err("Folder deletion was cancelled.".to_string());
            }
        }
    }

    let removed = into_string_error(state.profiles.remove_repository(&repo_path).await)?;

    if let Some(profiles) = removed {
        for profile in profiles {
            let _ = state.secrets.delete(&profile.secret_ref);
        }
    }

    if delete_folder {
        let path = std::path::Path::new(&repo_path);
        if path.exists() {
            if !path.is_dir() {
                return Err("Registered repositories must point to folders. Refusing to delete a non-directory path.".to_string());
            }

            into_string_error(tokio::fs::remove_dir_all(path).await)?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn save_profile(
    repo_path: String,
    draft: ProfileDraft,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<DeploymentProfile, String> {
    let existing = if let Some(profile_id) = draft.id.as_deref() {
        state.profiles.get_profile(&repo_path, profile_id).await.ok()
    } else {
        None
    };

    let secret_ref = existing
        .as_ref()
        .map(|profile| profile.secret_ref.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let existing_secret_available = existing
        .as_ref()
        .and_then(|profile| state.secrets.exists(&profile.secret_ref).ok())
        .unwrap_or(false);

    if let Some(password) = password.as_ref().filter(|value| !value.trim().is_empty()) {
        persist_secret(&state, &secret_ref, password)?;
    } else if existing.is_none() {
        return Err("A password is required when creating a new profile.".to_string());
    } else if !existing_secret_available {
        return Err("No password is saved for this profile. Enter the password and save the profile to store it securely.".to_string());
    }

    let profile = git_ftp::merge_profile(existing.as_ref(), draft, secret_ref);
    let saved = into_string_error(state.profiles.save_profile(&repo_path, profile).await)?;
    into_string_error(
        state
            .profiles
            .set_last_selected_profile(&repo_path, Some(&saved.id))
            .await,
    )?;
    Ok(saved)
}

#[tauri::command]
pub async fn delete_profile(
    repo_path: String,
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let removed = into_string_error(state.profiles.delete_profile(&repo_path, &profile_id).await)?;
    if let Some(profile) = removed {
        let _ = state.secrets.delete(&profile.secret_ref);
    }
    Ok(())
}

#[tauri::command]
pub async fn duplicate_profile(
    repo_path: String,
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<DeploymentProfile, String> {
    let existing = into_string_error(state.profiles.get_profile(&repo_path, &profile_id).await)?;
    let source_secret = into_string_error(state.secrets.read(&existing.secret_ref))?;
    let new_secret_ref = Uuid::new_v4().to_string();
    persist_secret(&state, &new_secret_ref, &source_secret)?;
    let duplicated = git_ftp::duplicate_profile(&existing, new_secret_ref);
    let saved = into_string_error(state.profiles.save_profile(&repo_path, duplicated).await)?;
    Ok(saved)
}

#[tauri::command]
pub async fn validate_profile(
    repo_path: String,
    draft: ProfileDraft,
    password: Option<String>,
    probe_remote: bool,
    state: State<'_, AppState>,
) -> Result<crate::models::ProfileValidationResult, String> {
    let existing_profile = match draft.id.as_deref() {
        Some(profile_id) => state.profiles.get_profile(&repo_path, profile_id).await.ok(),
        None => None,
    };

    let resolved_secret = password
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| {
            existing_profile
                .as_ref()
                .and_then(|profile| state.secrets.read(&profile.secret_ref).ok())
        });

    if draft.use_git_config_defaults && git::ensure_valid_git_repo(&repo_path).is_ok() {
        let existing_secret_ref = existing_profile
            .as_ref()
            .map(|profile| profile.secret_ref.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let hydrated = git_ftp::merge_profile(existing_profile.as_ref(), draft.clone(), existing_secret_ref);
        let _ = git_ftp::apply_git_config_defaults(&state.app, &repo_path, &hydrated);
    }

    into_string_error(git_ftp::validate_profile(
        &state.app,
        &repo_path,
        &draft,
        resolved_secret.as_deref().filter(|value| !value.trim().is_empty()),
        probe_remote,
    ))
}

#[tauri::command]
pub async fn commit_tracked_changes(
    repo_path: String,
    message: String,
) -> Result<CommitResult, String> {
    into_string_error(git::commit_tracked_changes(&repo_path, &message))
}

#[tauri::command]
pub async fn run_git_ftp_action(
    request: RunRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::models::RunRecord, String> {
    if !into_string_error(state.profiles.has_repository(&request.repo_path).await)? {
        return Err("Only repositories already registered with the app can run deployment actions.".to_string());
    }

    let repo = into_string_error(git::ensure_valid_git_repo(&request.repo_path))?;
    if !repo.is_git_repo {
        return Err("The selected folder is not a valid Git repository.".to_string());
    }

    let profile = into_string_error(
        state
            .profiles
            .get_profile(&request.repo_path, &request.profile_id)
            .await,
    )?;
    let typed_password = request
        .password
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    let secret = if let Some(password) = typed_password {
        persist_secret(&state, &profile.secret_ref, password)?;
        password.to_string()
    } else {
        if !into_string_error(state.secrets.exists(&profile.secret_ref))? {
            return Err("No password is saved for this profile. Enter the password in the profile editor and save the profile before running git-ftp.".to_string());
        }
        into_string_error(state.secrets.read(&profile.secret_ref))?
    };

    if profile.use_git_config_defaults {
        into_string_error(git_ftp::apply_git_config_defaults(&state.app, &request.repo_path, &profile))?;
    }

    if matches!(request.action, crate::models::RunAction::Download) {
        if !request.confirm_destructive_download {
            return Err("Remote sync downloads require explicit destructive-action confirmation.".to_string());
        }

        let confirmed = confirm_destructive_action(
            app.clone(),
            "Sync with remote",
            "This will permanently discard local tracked edits and untracked files in this repository before downloading the current remote state.",
        )
        .await?;

        if !confirmed {
            return Err("Remote sync was cancelled.".to_string());
        }

        into_string_error(git::discard_local_changes(&request.repo_path))?;
    }

    let spec = into_string_error(git_ftp::build_command_spec(
        &state.app,
        &request.repo_path,
        &profile,
        request.action.clone(),
        &request.options,
        secret,
    ))?;

    let mut result = into_string_error(state.process.run(app, spec).await)?;

    if matches!(request.action, crate::models::RunAction::Download) && result.success {
        if let Ok(repo) = git::inspect_repository(&request.repo_path) {
            result.changed_files = repo
                .status_summary
                .iter()
                .filter(|entry| !entry.starts_with("??"))
                .map(|entry| entry.trim().to_string())
                .collect();
        }
    }

    if result.success {
        let _ = state
            .profiles
            .update_profile_last_deployed(&request.repo_path, &request.profile_id, &git_ftp::timestamp_now())
            .await;
    }
    let _ = state.profiles.append_run_history(result.clone()).await;

    Ok(result)
}

#[tauri::command]
pub async fn cancel_running_process(state: State<'_, AppState>) -> Result<bool, String> {
    into_string_error(state.process.cancel().await)
}

#[tauri::command]
pub async fn fetch_run_history(
    repo_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::RunRecord>, String> {
    into_string_error(state.profiles.get_run_history(repo_path.as_deref()).await)
}

#[tauri::command]
pub async fn generate_debug_report(
    repo_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<DebugReport, String> {
    let diagnostics = diagnostics::startup_diagnostics(&state.app);
    let repo = match repo_path.as_deref() {
        Some(path) => git::inspect_repository(path).ok(),
        None => None,
    };
    let recent_history = into_string_error(state.profiles.get_run_history(repo_path.as_deref()).await)?;
    Ok(DebugReport {
        generated_at: git_ftp::timestamp_now(),
        diagnostics,
        repo,
        recent_history,
    })
}

#[tauri::command]
pub async fn bootstrap_remote_snapshot(
    app: AppHandle,
    request: SnapshotBootstrapRequest,
    state: State<'_, AppState>,
) -> Result<SnapshotBootstrapResult, String> {
    let result = into_string_error(git_ftp::bootstrap_snapshot(
        &app,
        &request.draft,
        &request.password,
        &request.local_path,
        &request.operation_id,
    )
    .await)?;

    if result.success {
        persist_secret(&state, &result.profile.secret_ref, &request.password)?;
        into_string_error(
            state
                .profiles
                .save_profile(&result.local_path, result.profile.clone())
                .await,
        )?;
        let _ = state.profiles.mark_repo_opened(&result.local_path).await;
    }

    Ok(result)
}

#[tauri::command]
pub async fn remove_remote_git_ftp_log(
    app: AppHandle,
    request: RemoteCleanupRequest,
) -> Result<RemoteCleanupResult, String> {
    into_string_error(git_ftp::remove_remote_git_ftp_log(
        &app,
        &request.draft,
        &request.password,
    )
    .await)
}
