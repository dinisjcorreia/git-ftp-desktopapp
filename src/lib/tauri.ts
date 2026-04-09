import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { confirm, open } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';
import type {
  CommitResult,
  DebugReport,
  DeploymentProfile,
  LogEvent,
  ProfileDraft,
  ProfileValidationResult,
  RemoteCleanupResult,
  RepoInfo,
  RunRecord,
  RunRequest,
  SavedRepository,
  SnapshotProgressEvent,
  SnapshotBootstrapResult,
  StartupDiagnostics
} from '../types';

const normalizePath = (value: string): string => {
  if (value.startsWith('file://')) {
    return decodeURIComponent(new URL(value).pathname);
  }
  return value;
};

export const chooseRepoDirectory = async (): Promise<string | null> => {
  const selected = await open({
    directory: true,
    multiple: false
  });

  if (!selected || Array.isArray(selected)) {
    return null;
  }

  return normalizePath(selected);
};

export const confirmDialog = (
  message: string,
  options?: string | { title?: string; kind?: 'info' | 'warning' | 'error' }
): Promise<boolean> => confirm(message, options);

export const openExternalUrl = (url: string): Promise<void> => openUrl(url);

export const onRunLog = async (handler: (event: LogEvent) => void): Promise<UnlistenFn> =>
  listen<LogEvent>('deployment-log', ({ payload }) => handler(payload));

export const onSnapshotProgress = async (
  handler: (event: SnapshotProgressEvent) => void
): Promise<UnlistenFn> => listen<SnapshotProgressEvent>('snapshot-progress', ({ payload }) => handler(payload));

export const startupDiagnostics = (): Promise<StartupDiagnostics> =>
  invoke('startup_diagnostics');

export const validateRepo = (repoPath: string): Promise<RepoInfo> =>
  invoke('validate_repo', { repoPath });

export const listProfiles = (repoPath: string): Promise<DeploymentProfile[]> =>
  invoke('list_profiles', { repoPath });

export const listKnownRepositories = (): Promise<SavedRepository[]> =>
  invoke('list_known_repositories');

export const removeKnownRepository = (
  repoPath: string,
  deleteFolder: boolean
): Promise<void> => invoke('remove_known_repository', { repoPath, deleteFolder });

export const saveProfile = (
  repoPath: string,
  draft: ProfileDraft,
  password?: string
): Promise<DeploymentProfile> =>
  invoke('save_profile', {
    repoPath,
    draft,
    password: password?.trim() ? password : null
  });

export const deleteProfile = (repoPath: string, profileId: string): Promise<void> =>
  invoke('delete_profile', { repoPath, profileId });

export const duplicateProfile = (
  repoPath: string,
  profileId: string
): Promise<DeploymentProfile> => invoke('duplicate_profile', { repoPath, profileId });

export const validateProfile = (
  repoPath: string,
  draft: ProfileDraft,
  password: string,
  probeRemote = true
): Promise<ProfileValidationResult> =>
  invoke('validate_profile', {
    repoPath,
    draft,
    password: password.trim() ? password : null,
    probeRemote
  });

export const commitTrackedChanges = (repoPath: string, message: string): Promise<CommitResult> =>
  invoke('commit_tracked_changes', { repoPath, message });

export const runGitFtpAction = (request: RunRequest): Promise<RunRecord> =>
  invoke('run_git_ftp_action', { request });

export const cancelRunningProcess = (): Promise<boolean> =>
  invoke('cancel_running_process');

export const fetchRunHistory = (repoPath?: string | null): Promise<RunRecord[]> =>
  invoke('fetch_run_history', { repoPath: repoPath ?? null });

export const generateDebugReport = (repoPath?: string | null): Promise<DebugReport> =>
  invoke('generate_debug_report', { repoPath: repoPath ?? null });

export const bootstrapRemoteSnapshot = (
  draft: ProfileDraft,
  password: string,
  localPath: string,
  operationId: string
): Promise<SnapshotBootstrapResult> =>
  invoke('bootstrap_remote_snapshot', {
    request: {
      draft,
      password,
      localPath,
      operationId
    }
  });

export const removeRemoteGitFtpLog = (
  draft: ProfileDraft,
  password: string
): Promise<RemoteCleanupResult> =>
  invoke('remove_remote_git_ftp_log', {
    request: {
      draft,
      password
    }
  });
