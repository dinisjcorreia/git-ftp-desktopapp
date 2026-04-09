use anyhow::{anyhow, Context, Result};
use std::fs;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tauri::{path::BaseDirectory, AppHandle, Manager};

const BUNDLED_GIT_FTP_SCRIPT: &str = "resources/third-party/tools/git-ftp/git-ftp";
const BUNDLED_LFTP_BASE: &str = "binaries/lftp";
const GIT_FTP_PLACEHOLDER_MARKER: &str = "Bundled git-ftp placeholder.";
const LFTP_PLACEHOLDER_MARKER: &str = "Bundled lftp sidecar placeholder.";

#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub executable: PathBuf,
    pub args_prefix: Vec<String>,
}

pub fn resolve_git() -> Option<PathBuf> {
    resolve_system_executable("git")
}

pub fn require_git() -> Result<PathBuf> {
    resolve_git().ok_or_else(|| anyhow!("Could not resolve `git` in PATH or common install locations."))
}

pub fn resolve_bash() -> Option<PathBuf> {
    resolve_system_executable("bash")
}

pub fn resolve_git_ftp(app: &AppHandle) -> Option<ResolvedProgram> {
    if let Some(script_path) = resolve_bundled_git_ftp_script(app) {
        if let Some(bash_path) = resolve_bash() {
            return Some(ResolvedProgram {
                executable: bash_path,
                args_prefix: vec![script_path.to_string_lossy().to_string()],
            });
        }
    }

    resolve_system_executable("git-ftp").map(|path| ResolvedProgram {
        executable: path,
        args_prefix: Vec::new(),
    })
}

pub fn require_git_ftp(app: &AppHandle) -> Result<ResolvedProgram> {
    resolve_git_ftp(app).ok_or_else(|| {
        anyhow!("Could not resolve `git-ftp` from the bundled resources or the local system.")
    })
}

pub fn resolve_lftp(app: &AppHandle) -> Option<PathBuf> {
    resolve_bundled_lftp(app).or_else(|| resolve_system_executable("lftp"))
}

pub fn require_lftp(app: &AppHandle) -> Result<PathBuf> {
    resolve_lftp(app)
        .ok_or_else(|| anyhow!("Could not resolve `lftp` from the bundled resources or the local system."))
}

pub fn resolve_bundled_git_ftp_script(app: &AppHandle) -> Option<PathBuf> {
    resolve_resource(app, BUNDLED_GIT_FTP_SCRIPT)
        .filter(|path| path.is_file())
        .filter(|path| !is_placeholder_asset(path, GIT_FTP_PLACEHOLDER_MARKER))
}

pub fn resolve_bundled_lftp(app: &AppHandle) -> Option<PathBuf> {
    let relative_path = if cfg!(target_os = "windows") {
        format!("{}-{}.exe", BUNDLED_LFTP_BASE, target_triple())
    } else {
        format!("{}-{}", BUNDLED_LFTP_BASE, target_triple())
    };

    resolve_resource(app, &relative_path)
        .filter(|path| path.is_file())
        .filter(|path| !is_placeholder_asset(path, LFTP_PLACEHOLDER_MARKER))
}

pub fn bundled_git_ftp_placeholder(app: &AppHandle) -> Option<PathBuf> {
    resolve_resource(app, BUNDLED_GIT_FTP_SCRIPT)
        .filter(|path| path.is_file())
        .filter(|path| is_placeholder_asset(path, GIT_FTP_PLACEHOLDER_MARKER))
}

pub fn bundled_lftp_placeholder(app: &AppHandle) -> Option<PathBuf> {
    let relative_path = if cfg!(target_os = "windows") {
        format!("{}-{}.exe", BUNDLED_LFTP_BASE, target_triple())
    } else {
        format!("{}-{}", BUNDLED_LFTP_BASE, target_triple())
    };

    resolve_resource(app, &relative_path)
        .filter(|path| path.is_file())
        .filter(|path| is_placeholder_asset(path, LFTP_PLACEHOLDER_MARKER))
}

pub fn prepend_to_path(_app: &AppHandle, entries: &[PathBuf]) -> Result<OsString> {
    let mut paths: Vec<PathBuf> = entries
        .iter()
        .filter(|path| path.is_dir())
        .cloned()
        .collect();

    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));

    std::env::join_paths(paths).with_context(|| "Could not construct PATH for bundled tools.")
}

fn resolve_system_executable(executable: &str) -> Option<PathBuf> {
    which::which(executable)
        .ok()
        .or_else(|| fallback_executable_locations(executable).into_iter().find(|path| path.is_file()))
}

fn resolve_resource(app: &AppHandle, relative_path: &str) -> Option<PathBuf> {
    app.path().resolve(relative_path, BaseDirectory::Resource).ok()
}

fn is_placeholder_asset(path: &Path, marker: &str) -> bool {
    const SAMPLE_BYTES: usize = 4096;

    fs::read(path)
        .ok()
        .map(|bytes| {
            let sample = &bytes[..bytes.len().min(SAMPLE_BYTES)];
            String::from_utf8_lossy(sample).contains(marker)
        })
        .unwrap_or(false)
}

fn fallback_executable_locations(executable: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.push(Path::new("/opt/homebrew/bin").join(executable));
        candidates.push(Path::new("/opt/homebrew/sbin").join(executable));
        candidates.push(Path::new("/usr/local/bin").join(executable));
        candidates.push(Path::new("/usr/local/sbin").join(executable));
        candidates.push(Path::new("/usr/bin").join(executable));
        candidates.push(Path::new("/bin").join(executable));
    }

    if cfg!(target_os = "linux") {
        candidates.push(Path::new("/usr/local/bin").join(executable));
        candidates.push(Path::new("/usr/local/sbin").join(executable));
        candidates.push(Path::new("/usr/bin").join(executable));
        candidates.push(Path::new("/usr/sbin").join(executable));
        candidates.push(Path::new("/bin").join(executable));
        candidates.push(Path::new("/sbin").join(executable));
        candidates.push(Path::new("/snap/bin").join(executable));
    }

    if cfg!(target_os = "windows") {
        candidates.push(Path::new("C:\\Program Files\\Git\\bin").join(format!("{executable}.exe")));
        candidates.push(Path::new("C:\\Program Files\\Git\\usr\\bin").join(format!("{executable}.exe")));
        candidates.push(Path::new("C:\\Program Files (x86)\\Git\\bin").join(format!("{executable}.exe")));
        candidates.push(Path::new("C:\\Program Files (x86)\\Git\\usr\\bin").join(format!("{executable}.exe")));
    }

    candidates
}

pub fn target_triple() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        _ => "unknown-target",
    }
}
