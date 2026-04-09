mod commands;
mod diagnostics;
mod executables;
mod git;
mod git_ftp;
mod models;
mod process;
mod profiles;
mod secrets;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(AppState::new(app.handle())?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup_diagnostics,
            commands::validate_repo,
            commands::list_profiles,
            commands::list_known_repositories,
            commands::remove_known_repository,
            commands::save_profile,
            commands::delete_profile,
            commands::duplicate_profile,
            commands::validate_profile,
            commands::commit_tracked_changes,
            commands::run_git_ftp_action,
            commands::cancel_running_process,
            commands::fetch_run_history,
            commands::generate_debug_report,
            commands::bootstrap_remote_snapshot,
            commands::remove_remote_git_ftp_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
