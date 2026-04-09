export type DependencyStatus = {
  name: string;
  command: string;
  required: boolean;
  installed: boolean;
  resolvedPath?: string | null;
  version?: string | null;
  stdout: string;
  stderr: string;
  installHint: string;
};

export type StartupDiagnostics = {
  os: string;
  arch: string;
  gitAvailable: boolean;
  gitFtpAvailable: boolean;
  overallReady: boolean;
  dependencies: DependencyStatus[];
};

export type RepoInfo = {
  path: string;
  isGitRepo: boolean;
  currentBranch?: string | null;
  remoteOrigin?: string | null;
  dirty: boolean;
  gitDir?: string | null;
  statusSummary: string[];
};

export type SavedRepository = {
  path: string;
  lastOpenedAt?: string | null;
  profileCount: number;
  lastSelectedProfileId?: string | null;
};

export type DeploymentProtocol = 'ftp' | 'sftp' | 'ftps' | 'ftpes';

export type ProfileFlags = {
  syncroot?: string | null;
  remoteRoot?: string | null;
  activeMode: boolean;
  disableEpsv: boolean;
  insecure: boolean;
  autoInit: boolean;
  useAll: boolean;
};

export type DeploymentProfile = {
  id: string;
  name: string;
  protocol: DeploymentProtocol;
  host: string;
  port: number;
  username: string;
  remotePath: string;
  secretRef: string;
  useGitConfigDefaults: boolean;
  flags: ProfileFlags;
  lastDeployedAt?: string | null;
  createdAt: string;
  updatedAt: string;
};

export type ProfileDraft = {
  id?: string | null;
  name: string;
  protocol: DeploymentProtocol;
  host: string;
  port: number;
  username: string;
  remotePath: string;
  useGitConfigDefaults: boolean;
  flags: ProfileFlags;
};

export type RunAction = 'init' | 'push' | 'catchup' | 'download';

export type RunOptions = {
  dryRun: boolean;
  verbose: boolean;
};

export type RunRequest = {
  repoPath: string;
  profileId: string;
  action: RunAction;
  options: RunOptions;
  password?: string | null;
  confirmDestructiveDownload: boolean;
};

export type CommitResult = {
  commitSha: string;
  summary: string;
  repo: RepoInfo;
};

export type LogStream = 'stdout' | 'stderr' | 'system';
export type LogKind = 'info' | 'warning' | 'error' | 'system';

export type RunLogEntry = {
  timestamp: string;
  stream: LogStream;
  kind: LogKind;
  line: string;
};

export type RunRecord = {
  id: string;
  repoPath: string;
  profileId: string;
  profileName: string;
  action: RunAction;
  startedAt: string;
  finishedAt?: string | null;
  success: boolean;
  exitCode?: number | null;
  commandPreview: string;
  logs: RunLogEntry[];
  cancelled: boolean;
  changedFiles: string[];
};

export type LogEvent = {
  runId: string;
  entry: RunLogEntry;
};

export type ValidationMessageKind = 'info' | 'warning' | 'error';

export type ValidationMessage = {
  kind: ValidationMessageKind;
  text: string;
};

export type CommandProbeResult = {
  success: boolean;
  exitCode?: number | null;
  stdout: string;
  stderr: string;
};

export type ProfileValidationResult = {
  valid: boolean;
  commandPreview?: string | null;
  messages: ValidationMessage[];
  repo: RepoInfo;
  diagnostics: StartupDiagnostics;
  probeResult?: CommandProbeResult | null;
};

export type DebugReport = {
  generatedAt: string;
  diagnostics: StartupDiagnostics;
  repo?: RepoInfo | null;
  recentHistory: RunRecord[];
};

export type SnapshotBootstrapRequest = {
  draft: ProfileDraft;
  password: string;
  localPath: string;
  operationId: string;
};

export type SnapshotBootstrapResult = {
  success: boolean;
  localPath: string;
  commandPreview: string;
  stdout: string;
  stderr: string;
  exitCode?: number | null;
  repo: RepoInfo;
  profile: DeploymentProfile;
};

export type RemoteCleanupRequest = {
  draft: ProfileDraft;
  password: string;
};

export type RemoteCleanupResult = {
  success: boolean;
  commandPreview: string;
  stdout: string;
  stderr: string;
  exitCode?: number | null;
};

export type SnapshotProgressStatus = 'running' | 'success' | 'error';

export type SnapshotProgressEvent = {
  operationId: string;
  status: SnapshotProgressStatus;
  progress: number;
  title: string;
  detail: string;
  commandPreview?: string | null;
  stream?: LogStream | null;
  line?: string | null;
};

export type ToastTone = 'success' | 'error' | 'info';

export type ToastMessage = {
  id: string;
  title: string;
  detail?: string;
  tone: ToastTone;
};
