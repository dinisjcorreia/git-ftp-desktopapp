use crate::executables;
use crate::models::{DependencyStatus, StartupDiagnostics};
use std::process::Command;
use tauri::AppHandle;

pub fn startup_diagnostics(app: &AppHandle) -> StartupDiagnostics {
    let git = inspect_dependency("git", &["--version"], true);
    let git_ftp = inspect_git_ftp(app);
    let lftp = inspect_lftp(app);

    StartupDiagnostics {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        git_available: git.installed,
        git_ftp_available: git_ftp.installed,
        overall_ready: git.installed && git_ftp.installed,
        dependencies: vec![git, git_ftp, lftp],
    }
}

fn inspect_dependency(name: &str, version_args: &[&str], required: bool) -> DependencyStatus {
    let resolved = match name {
        "git" => executables::resolve_git(),
        _ => None,
    };
    let resolved_path = resolved.as_ref().map(|path| path.to_string_lossy().to_string());
    let command = format!("{name} {}", version_args.join(" "));
    let install_hint = install_hint(name);

    if resolved_path.is_none() {
        return DependencyStatus {
            name: name.to_string(),
            command,
            required,
            installed: false,
            resolved_path: None,
            version: None,
            stdout: String::new(),
            stderr: "Executable not found in PATH.".to_string(),
            install_hint,
        };
    }

    let output = Command::new(resolved.expect("resolved path checked above"))
        .args(version_args)
        .output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let version = stdout
                .lines()
                .next()
                .map(ToOwned::to_owned)
                .filter(|line| !line.is_empty())
                .or_else(|| stderr.lines().next().map(ToOwned::to_owned).filter(|line| !line.is_empty()));
            DependencyStatus {
                name: name.to_string(),
                command,
                required,
                installed: output.status.success(),
                resolved_path,
                version,
                stdout,
                stderr,
                install_hint,
            }
        }
        Err(error) => DependencyStatus {
            name: name.to_string(),
            command,
            required,
            installed: false,
            resolved_path,
            version: None,
            stdout: String::new(),
            stderr: error.to_string(),
            install_hint,
        },
    }
}

fn inspect_git_ftp(app: &AppHandle) -> DependencyStatus {
    match executables::resolve_git_ftp(app) {
        Some(program) => inspect_resolved_dependency(
            "git-ftp",
            format_command(&program.executable, &program.args_prefix, &["--version"]),
            true,
            Some(program.executable.to_string_lossy().to_string()),
            &program.executable,
            &combined_args(&program.args_prefix, &["--version"]),
        ),
        None => {
            if let Some(path) = executables::bundled_git_ftp_placeholder(app) {
                return DependencyStatus {
                    name: "git-ftp".to_string(),
                    command: "git-ftp --version".to_string(),
                    required: true,
                    installed: false,
                    resolved_path: Some(path.to_string_lossy().to_string()),
                    version: None,
                    stdout: String::new(),
                    stderr: "Bundled git-ftp placeholder found. Run the release prep script to replace it, or install git-ftp locally for development builds.".to_string(),
                    install_hint: install_hint("git-ftp"),
                };
            }

            missing_dependency("git-ftp", "git-ftp --version", true)
        }
    }
}

fn inspect_lftp(app: &AppHandle) -> DependencyStatus {
    match executables::resolve_lftp(app) {
        Some(path) => inspect_resolved_dependency(
            "lftp",
            format!("{} --version", path.to_string_lossy()),
            false,
            Some(path.to_string_lossy().to_string()),
            &path,
            &["--version".to_string()],
        ),
        None => {
            if let Some(path) = executables::bundled_lftp_placeholder(app) {
                return DependencyStatus {
                    name: "lftp".to_string(),
                    command: "lftp --version".to_string(),
                    required: false,
                    installed: false,
                    resolved_path: Some(path.to_string_lossy().to_string()),
                    version: None,
                    stdout: String::new(),
                    stderr: "Bundled lftp placeholder found. Run the release prep script to replace it, or install lftp locally for development builds.".to_string(),
                    install_hint: install_hint("lftp"),
                };
            }

            missing_dependency("lftp", "lftp --version", false)
        }
    }
}

fn inspect_resolved_dependency(
    name: &str,
    command: String,
    required: bool,
    resolved_path: Option<String>,
    executable: &std::path::Path,
    args: &[String],
) -> DependencyStatus {
    let install_hint = install_hint(name);
    let output = Command::new(executable)
        .args(args)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let version = stdout
                .lines()
                .next()
                .map(ToOwned::to_owned)
                .filter(|line| !line.is_empty())
                .or_else(|| stderr.lines().next().map(ToOwned::to_owned).filter(|line| !line.is_empty()));
            DependencyStatus {
                name: name.to_string(),
                command,
                required,
                installed: output.status.success(),
                resolved_path,
                version,
                stdout,
                stderr,
                install_hint,
            }
        }
        Err(error) => DependencyStatus {
            name: name.to_string(),
            command,
            required,
            installed: false,
            resolved_path,
            version: None,
            stdout: String::new(),
            stderr: error.to_string(),
            install_hint,
        },
    }
}

fn missing_dependency(name: &str, command: &str, required: bool) -> DependencyStatus {
    DependencyStatus {
        name: name.to_string(),
        command: command.to_string(),
        required,
        installed: false,
        resolved_path: None,
        version: None,
        stdout: String::new(),
        stderr: "Executable not found in PATH, bundled tools, or common install locations.".to_string(),
        install_hint: install_hint(name),
    }
}

fn combined_args(prefix: &[String], suffix: &[&str]) -> Vec<String> {
    let mut args = prefix.to_vec();
    args.extend(suffix.iter().map(|arg| (*arg).to_string()));
    args
}

fn format_command(executable: &std::path::Path, prefix: &[String], suffix: &[&str]) -> String {
    let mut parts = vec![executable.to_string_lossy().to_string()];
    parts.extend(prefix.iter().cloned());
    parts.extend(suffix.iter().map(|arg| (*arg).to_string()));
    parts.join(" ")
}

fn install_hint(name: &str) -> String {
    match (std::env::consts::OS, name) {
        ("macos", "git") => "Install Git with Homebrew: brew install git".to_string(),
        ("macos", "git-ftp") => {
            "Install git-ftp with Homebrew after Git: brew install git-ftp".to_string()
        }
        ("macos", "lftp") => "Install lftp with Homebrew: brew install lftp".to_string(),
        ("windows", "git") => "Install Git for Windows: winget install Git.Git".to_string(),
        ("windows", "git-ftp") => {
            "Install git-ftp in Git Bash or Cygwin. See the upstream install guide for Windows details.".to_string()
        }
        ("windows", "lftp") => {
            "Install lftp and ensure it is available in PATH. Snapshot downloads depend on it.".to_string()
        }
        (_, "git") => "Install Git using your package manager, for example: sudo apt install git".to_string(),
        (_, "git-ftp") => {
            "Install git-ftp using your package manager when available, or from https://github.com/git-ftp/git-ftp".to_string()
        }
        (_, "lftp") => "Install lftp using your package manager, for example: sudo apt install lftp".to_string(),
        _ => "Check the project documentation for installation instructions.".to_string(),
    }
}
