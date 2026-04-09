use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub name: String,
    pub command: String,
    pub required: bool,
    pub installed: bool,
    pub resolved_path: Option<String>,
    pub version: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub install_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupDiagnostics {
    pub os: String,
    pub arch: String,
    pub git_available: bool,
    pub git_ftp_available: bool,
    pub overall_ready: bool,
    pub dependencies: Vec<DependencyStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub path: String,
    pub is_git_repo: bool,
    pub current_branch: Option<String>,
    pub remote_origin: Option<String>,
    pub dirty: bool,
    pub git_dir: Option<String>,
    pub status_summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRepository {
    pub path: String,
    pub last_opened_at: Option<String>,
    pub profile_count: usize,
    pub last_selected_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentProtocol {
    Ftp,
    Sftp,
    Ftps,
    Ftpes,
}

impl Default for DeploymentProtocol {
    fn default() -> Self {
        Self::Ftp
    }
}

impl DeploymentProtocol {
    pub fn scheme(&self) -> &'static str {
        match self {
            Self::Ftp => "ftp",
            Self::Sftp => "sftp",
            Self::Ftps => "ftps",
            Self::Ftpes => "ftpes",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFlags {
    pub syncroot: Option<String>,
    pub remote_root: Option<String>,
    pub active_mode: bool,
    pub disable_epsv: bool,
    pub insecure: bool,
    pub auto_init: bool,
    pub use_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentProfile {
    pub id: String,
    pub name: String,
    pub protocol: DeploymentProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub remote_path: String,
    pub secret_ref: String,
    pub use_git_config_defaults: bool,
    pub flags: ProfileFlags,
    pub last_deployed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDraft {
    pub id: Option<String>,
    pub name: String,
    pub protocol: DeploymentProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub remote_path: String,
    pub use_git_config_defaults: bool,
    pub flags: ProfileFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunAction {
    Init,
    Push,
    Catchup,
    Download,
}

impl RunAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Push => "push",
            Self::Catchup => "catchup",
            Self::Download => "download",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunOptions {
    pub dry_run: bool,
    pub verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub repo_path: String,
    pub profile_id: String,
    pub action: RunAction,
    pub options: RunOptions,
    pub password: Option<String>,
    #[serde(default)]
    pub confirm_destructive_download: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitResult {
    pub commit_sha: String,
    pub summary: String,
    pub repo: RepoInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogKind {
    Info,
    Warning,
    Error,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunLogEntry {
    pub timestamp: String,
    pub stream: LogStream,
    pub kind: LogKind,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: String,
    pub repo_path: String,
    pub profile_id: String,
    pub profile_name: String,
    pub action: RunAction,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub command_preview: String,
    pub logs: Vec<RunLogEntry>,
    pub cancelled: bool,
    #[serde(default)]
    pub changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub run_id: String,
    pub entry: RunLogEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationMessageKind {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationMessage {
    pub kind: ValidationMessageKind,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandProbeResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileValidationResult {
    pub valid: bool,
    pub command_preview: Option<String>,
    pub messages: Vec<ValidationMessage>,
    pub repo: RepoInfo,
    pub diagnostics: StartupDiagnostics,
    pub probe_result: Option<CommandProbeResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugReport {
    pub generated_at: String,
    pub diagnostics: StartupDiagnostics,
    pub repo: Option<RepoInfo>,
    pub recent_history: Vec<RunRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotBootstrapRequest {
    pub draft: ProfileDraft,
    pub password: String,
    pub local_path: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotBootstrapResult {
    pub success: bool,
    pub local_path: String,
    pub command_preview: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub repo: RepoInfo,
    pub profile: DeploymentProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCleanupRequest {
    pub draft: ProfileDraft,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCleanupResult {
    pub success: bool,
    pub command_preview: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotProgressStatus {
    Running,
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotProgressEvent {
    pub operation_id: String,
    pub status: SnapshotProgressStatus,
    pub progress: u8,
    pub title: String,
    pub detail: String,
    pub command_preview: Option<String>,
    pub stream: Option<LogStream>,
    pub line: Option<String>,
}
