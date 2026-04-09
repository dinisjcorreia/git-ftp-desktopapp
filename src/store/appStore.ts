import { create } from 'zustand';
import type {
  CommitResult,
  DebugReport,
  DeploymentProfile,
  LogEvent,
  RepoInfo,
  RunAction,
  RunOptions,
  RunRecord,
  SavedRepository,
  StartupDiagnostics,
  ToastMessage
} from '../types';
import * as api from '../lib/tauri';
import { getRunFailureDetail } from '../lib/utils';

type AppState = {
  diagnostics: StartupDiagnostics | null;
  selectedRepoPath: string | null;
  repoInfo: RepoInfo | null;
  knownRepositories: SavedRepository[];
  profiles: DeploymentProfile[];
  selectedProfileId: string | null;
  runHistory: RunRecord[];
  activeRun: RunRecord | null;
  pendingRunAction: RunAction | null;
  selectedRunId: string | null;
  isBootstrapping: boolean;
  isRunning: boolean;
  isCommitting: boolean;
  toasts: ToastMessage[];
  bootstrap: () => Promise<void>;
  clearWorkspace: () => void;
  setSelectedProfileId: (profileId: string | null) => void;
  loadRepo: (repoPath: string) => Promise<void>;
  pickRepo: () => Promise<void>;
  refreshKnownRepositories: () => Promise<void>;
  refreshRepoInfo: () => Promise<void>;
  refreshProfiles: () => Promise<void>;
  removeKnownRepository: (repoPath: string, deleteFolder: boolean) => Promise<void>;
  saveProfile: (draft: Parameters<typeof api.saveProfile>[1], password?: string) => Promise<DeploymentProfile>;
  deleteProfile: (profileId: string) => Promise<void>;
  duplicateProfile: (profileId: string) => Promise<DeploymentProfile>;
  commitTrackedChanges: (message: string) => Promise<CommitResult>;
  runAction: (profileId: string, action: RunAction, options: RunOptions, password?: string) => Promise<RunRecord>;
  cancelRun: () => Promise<void>;
  refreshHistory: () => Promise<void>;
  receiveLogEvent: (event: LogEvent) => void;
  receiveLogEvents: (events: LogEvent[]) => void;
  dismissToast: (toastId: string) => void;
  copyDebugReport: () => Promise<DebugReport>;
};

const MAX_ACTIVE_LOG_LINES = 1200;

const pushToast = (
  set: (partial: Partial<AppState> | ((state: AppState) => Partial<AppState>)) => void,
  toast: Omit<ToastMessage, 'id'>
) => {
  const id = crypto.randomUUID();
  set((state) => ({
    toasts: [...state.toasts, { id, ...toast }]
  }));
  window.setTimeout(() => {
    set((state) => ({
      toasts: state.toasts.filter((item) => item.id !== id)
    }));
  }, 4200);
};

export const useAppStore = create<AppState>((set, get) => ({
  diagnostics: null,
  selectedRepoPath: null,
  repoInfo: null,
  knownRepositories: [],
  profiles: [],
  selectedProfileId: null,
  runHistory: [],
  activeRun: null,
  pendingRunAction: null,
  selectedRunId: null,
  isBootstrapping: true,
  isRunning: false,
  isCommitting: false,
  toasts: [],

  bootstrap: async () => {
    set({ isBootstrapping: true });
    try {
      const [diagnostics, runHistory, knownRepositories] = await Promise.all([
        api.startupDiagnostics(),
        api.fetchRunHistory(null),
        api.listKnownRepositories()
      ]);
      set({
        diagnostics,
        knownRepositories,
        runHistory,
        selectedRunId: runHistory[0]?.id ?? null
      });

      const mostRecentRepo = knownRepositories[0]?.path;
      if (mostRecentRepo) {
        try {
          await get().loadRepo(mostRecentRepo);
          await get().refreshKnownRepositories();
        } catch (_error) {
          // Keep remembered repositories visible even if the latest one cannot be reopened automatically.
        }
      }

      set({ isBootstrapping: false });
    } catch (error) {
      set({ isBootstrapping: false });
      pushToast(set, {
        tone: 'error',
        title: 'Startup diagnostics failed',
        detail: error instanceof Error ? error.message : String(error)
      });
    }
  },

  clearWorkspace: () => {
    set({
      selectedRepoPath: null,
      repoInfo: null,
      profiles: [],
      selectedProfileId: null,
      runHistory: [],
      activeRun: null,
      pendingRunAction: null,
      selectedRunId: null
    });
  },

  setSelectedProfileId: (selectedProfileId) => set({ selectedProfileId }),

  loadRepo: async (repoPath) => {
    const [repoInfo, profiles, runHistory] = await Promise.all([
      api.validateRepo(repoPath),
      api.listProfiles(repoPath),
      api.fetchRunHistory(repoPath)
    ]);

    const selectedProfileId =
      profiles.find((profile) => profile.id === get().selectedProfileId)?.id ?? profiles[0]?.id ?? null;

    set({
      selectedRepoPath: repoPath,
      repoInfo,
      knownRepositories: get().knownRepositories,
      profiles,
      selectedProfileId,
      runHistory,
      selectedRunId: runHistory[0]?.id ?? null
    });
    await get().refreshKnownRepositories();
  },

  pickRepo: async () => {
    const selected = await api.chooseRepoDirectory();
    if (!selected) {
      return;
    }

    try {
      await get().loadRepo(selected);
      pushToast(set, {
        tone: 'info',
        title: 'Repository loaded',
        detail: selected
      });
    } catch (error) {
      pushToast(set, {
        tone: 'error',
        title: 'Could not load repository',
        detail: error instanceof Error ? error.message : String(error)
      });
    }
  },

  refreshKnownRepositories: async () => {
    const knownRepositories = await api.listKnownRepositories();
    set({ knownRepositories });
  },

  refreshRepoInfo: async () => {
    const repoPath = get().selectedRepoPath;
    if (!repoPath) {
      return;
    }

    const repoInfo = await api.validateRepo(repoPath);
    set({ repoInfo });
  },

  refreshProfiles: async () => {
    const repoPath = get().selectedRepoPath;
    if (!repoPath) {
      return;
    }

    const profiles = await api.listProfiles(repoPath);
    set((state) => ({
      profiles,
      selectedProfileId:
        profiles.find((profile) => profile.id === state.selectedProfileId)?.id ?? profiles[0]?.id ?? null
    }));
  },

  removeKnownRepository: async (repoPath, deleteFolder) => {
    await api.removeKnownRepository(repoPath, deleteFolder);
    const knownRepositories = await api.listKnownRepositories();

    if (get().selectedRepoPath === repoPath) {
      const nextRepoPath = knownRepositories[0]?.path ?? null;
      if (nextRepoPath) {
        await get().loadRepo(nextRepoPath);
      } else {
        set({
          selectedRepoPath: null,
          repoInfo: null,
          profiles: [],
          selectedProfileId: null,
          runHistory: [],
          activeRun: null,
          pendingRunAction: null,
          selectedRunId: null
        });
      }
    }

    set({ knownRepositories });
    pushToast(set, {
      tone: 'info',
      title: deleteFolder ? 'Repository removed and folder deleted' : 'Repository removed from app',
      detail: repoPath
    });
  },

  saveProfile: async (draft, password) => {
    const repoPath = get().selectedRepoPath;
    if (!repoPath) {
      throw new Error('Select a repository before saving a profile.');
    }

    try {
      const saved = await api.saveProfile(repoPath, draft, password);
      await get().refreshProfiles();
      set({ selectedProfileId: saved.id });
      pushToast(set, {
        tone: 'success',
        title: 'Profile saved',
        detail: `${saved.name} is ready to deploy.`
      });
      return saved;
    } catch (error) {
      pushToast(set, {
        tone: 'error',
        title: 'Could not save profile',
        detail: error instanceof Error ? error.message : String(error)
      });
      throw error;
    }
  },

  deleteProfile: async (profileId) => {
    const repoPath = get().selectedRepoPath;
    if (!repoPath) {
      return;
    }

    await api.deleteProfile(repoPath, profileId);
    await get().refreshProfiles();
    pushToast(set, {
      tone: 'info',
      title: 'Profile removed'
    });
  },

  duplicateProfile: async (profileId) => {
    const repoPath = get().selectedRepoPath;
    if (!repoPath) {
      throw new Error('Select a repository before duplicating a profile.');
    }

    const duplicated = await api.duplicateProfile(repoPath, profileId);
    await get().refreshProfiles();
    set({ selectedProfileId: duplicated.id });
    pushToast(set, {
      tone: 'success',
      title: 'Profile duplicated',
      detail: duplicated.name
    });
    return duplicated;
  },

  commitTrackedChanges: async (message) => {
    const repoPath = get().selectedRepoPath;
    if (!repoPath) {
      throw new Error('Select a repository before creating a commit.');
    }

    set({ isCommitting: true });
    try {
      const result = await api.commitTrackedChanges(repoPath, message);
      await Promise.all([get().refreshRepoInfo(), get().refreshKnownRepositories()]);
      pushToast(set, {
        tone: 'success',
        title: 'Tracked changes committed',
        detail: result.commitSha.slice(0, 7)
      });
      set({ isCommitting: false });
      return result;
    } catch (error) {
      set({ isCommitting: false });
      pushToast(set, {
        tone: 'error',
        title: 'Commit failed',
        detail: error instanceof Error ? error.message : String(error)
      });
      throw error;
    }
  },

  runAction: async (profileId, action, options, password) => {
    const repoPath = get().selectedRepoPath;
    if (!repoPath) {
      throw new Error('Select a repository before running git-ftp.');
    }

    set({ isRunning: true, pendingRunAction: action });
    try {
      const result = await api.runGitFtpAction({
        repoPath,
        profileId,
        action,
        options,
        password: password?.trim() ? password : null,
        confirmDestructiveDownload: action === 'download'
      });
      set((state) => ({
        activeRun: result,
        runHistory: [result, ...state.runHistory.filter((run) => run.id !== result.id)].slice(0, 60),
        selectedRunId: result.id,
        pendingRunAction: null,
        isRunning: false
      }));
      await Promise.all([get().refreshProfiles(), get().refreshHistory(), get().refreshRepoInfo(), get().refreshKnownRepositories()]);
      pushToast(set, {
        tone: result.success ? 'success' : 'error',
        title:
          result.success
            ? action === 'download'
              ? 'Remote sync completed'
              : `${action.toUpperCase()} completed`
            : action === 'download'
              ? 'Remote sync failed'
              : `${action.toUpperCase()} failed`,
        detail:
          action === 'download' && result.success
            ? result.changedFiles.length > 0
              ? `Downloaded remote changes for ${result.changedFiles.length} file${result.changedFiles.length === 1 ? '' : 's'}.`
              : 'Remote sync completed without tracked file changes.'
            : result.success
              ? result.commandPreview
              : getRunFailureDetail(result)
      });
      return result;
    } catch (error) {
      set({ isRunning: false, pendingRunAction: null });
      pushToast(set, {
        tone: 'error',
        title: `${action.toUpperCase()} failed`,
        detail: error instanceof Error ? error.message : String(error)
      });
      throw error;
    }
  },

  cancelRun: async () => {
    const cancelled = await api.cancelRunningProcess();
    if (cancelled) {
      pushToast(set, {
        tone: 'info',
        title: 'Cancellation requested'
      });
    }
  },

  refreshHistory: async () => {
    const repoPath = get().selectedRepoPath;
    const runHistory = await api.fetchRunHistory(repoPath);
    set((state) => ({
      runHistory,
      selectedRunId:
        runHistory.find((run) => run.id === state.selectedRunId)?.id ?? runHistory[0]?.id ?? null
    }));
  },

  receiveLogEvent: (event) => {
    get().receiveLogEvents([event]);
  },

  receiveLogEvents: (events) => {
    if (events.length === 0) {
      return;
    }

    set((state) => {
      const latestEvent = events[events.length - 1];
      const activeRun =
        state.activeRun && state.activeRun.id === latestEvent.runId
          ? state.activeRun
          : {
              id: latestEvent.runId,
              repoPath: state.selectedRepoPath ?? '',
              profileId: state.selectedProfileId ?? '',
              profileName: state.profiles.find((profile) => profile.id === state.selectedProfileId)?.name ?? 'Running profile',
              action: state.pendingRunAction ?? 'push',
              startedAt: latestEvent.entry.timestamp,
              finishedAt: null,
              success: false,
              exitCode: null,
              commandPreview: 'git-ftp …',
              logs: [],
              cancelled: false,
              changedFiles: []
            };

      const nextLogs = [...activeRun.logs, ...events.map((event) => event.entry)];
      const trimmedLogs =
        nextLogs.length > MAX_ACTIVE_LOG_LINES
          ? nextLogs.slice(nextLogs.length - MAX_ACTIVE_LOG_LINES)
          : nextLogs;

      return {
        activeRun: {
          ...activeRun,
          logs: trimmedLogs
        },
        selectedRunId: latestEvent.runId
      };
    });
  },

  dismissToast: (toastId) =>
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== toastId)
    })),

  copyDebugReport: async () => {
    const report = await api.generateDebugReport(get().selectedRepoPath);
    await navigator.clipboard.writeText(JSON.stringify(report, null, 2));
    pushToast(set, {
      tone: 'success',
      title: 'Debug report copied'
    });
    return report;
  }
}));
