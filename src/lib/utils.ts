import type { DeploymentProfile, ProfileDraft, RunRecord } from '../types';

type RunProgressTone = 'running' | 'success' | 'error';

export type RunProgressState = {
  progress: number;
  title: string;
  detail: string;
  tone: RunProgressTone;
};

export const createBlankProfileDraft = (): ProfileDraft => ({
  name: '',
  protocol: 'ftp',
  host: '',
  port: 21,
  username: '',
  remotePath: '/public_html',
  useGitConfigDefaults: false,
  flags: {
    syncroot: '',
    remoteRoot: '',
    activeMode: false,
    disableEpsv: false,
    insecure: false,
    autoInit: false,
    useAll: false
  }
});

export const draftFromProfile = (profile: DeploymentProfile): ProfileDraft => ({
  id: profile.id,
  name: profile.name,
  protocol: profile.protocol,
  host: profile.host,
  port: profile.port,
  username: profile.username,
  remotePath: profile.remotePath,
  useGitConfigDefaults: profile.useGitConfigDefaults,
  flags: {
    syncroot: profile.flags.syncroot ?? '',
    remoteRoot: profile.flags.remoteRoot ?? '',
    activeMode: profile.flags.activeMode,
    disableEpsv: profile.flags.disableEpsv,
    insecure: profile.flags.insecure,
    autoInit: profile.flags.autoInit,
    useAll: profile.flags.useAll
  }
});

export const formatDateTime = (value?: string | null): string => {
  if (!value) {
    return 'Never';
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short'
  }).format(date);
};

export const formatRunTitle = (run: RunRecord): string =>
  `${run.action.toUpperCase()} · ${run.profileName}`;

export const stringifyLogs = (run?: RunRecord | null): string =>
  run?.logs.map((entry) => `[${entry.timestamp}] ${entry.stream.toUpperCase()} ${entry.line}`).join('\n') ?? '';

export const getRunFailureDetail = (run?: RunRecord | null): string => {
  if (!run) {
    return 'git-ftp failed before it could report a detailed error.';
  }

  const lines = run.logs
    .map((entry) => entry.line.trim())
    .filter(Boolean);
  const normalizedLines = lines.map((line) => line.toLowerCase());

  const initialPushHintIndex = normalizedLines.findIndex(
    (line) =>
      line.includes("use 'git ftp init' for the initial push") ||
      line.includes("use 'git-ftp init' for the initial push") ||
      line.includes('could not get last commit')
  );
  if (initialPushHintIndex >= 0) {
    return 'This FTP path does not have a remote .git-ftp.log yet. Run Initialize remote tracking for the first upload, then use Push changed files afterward.';
  }

  const resourceMissingIndex = normalizedLines.findIndex(
    (line) => line.includes('the file does not exist') || line.includes('the resource does not exist')
  );
  if (resourceMissingIndex >= 0 && run.action === 'push') {
    return 'The remote path does not contain the git-ftp tracking marker yet. Run Initialize remote tracking before your first push to this server path.';
  }

  const fatalLine = [...lines].reverse().find((line) => line.toLowerCase().includes('fatal:'));
  if (fatalLine) {
    return fatalLine.replace(/^fatal:\s*/i, '');
  }

  const stderrLine = [...run.logs]
    .reverse()
    .find((entry) => entry.stream === 'stderr' && entry.line.trim())
    ?.line.trim();
  if (stderrLine) {
    return stderrLine;
  }

  return lines[lines.length - 1] ?? `git-ftp exited with code ${run.exitCode ?? 'unknown'}.`;
};

export const deriveSyncProgress = (run?: RunRecord | null): RunProgressState | null => {
  if (!run || run.action !== 'download') {
    return null;
  }

  const lines = run.logs
    .map((entry) => entry.line.trim())
    .filter(Boolean);
  const latestLine = lines[lines.length - 1] ?? 'Discarding local edits and preparing the remote download.';

  let progress = Math.min(18 + lines.length * 6, 88);
  let title = 'Preparing remote sync';
  let detail = 'Discarding local edits and establishing the remote session.';

  for (const entry of run.logs) {
    const line = entry.line.trim();
    const normalized = line.toLowerCase();
    if (!normalized) {
      continue;
    }

    if (normalized.includes('starting ')) {
      progress = Math.max(progress, 8);
      title = 'Preparing remote sync';
      detail = 'Discarding local edits before downloading the latest remote state.';
      continue;
    }
    if (normalized.includes('git-ftp version')) {
      progress = Math.max(progress, 18);
      title = 'Launching git-ftp';
      detail = 'The download process is up and negotiating the session.';
      continue;
    }
    if (normalized.includes('insecure ssl/tls connection allowed')) {
      progress = Math.max(progress, 26);
      title = 'Connecting to the FTP server';
      detail = 'The remote server accepted the session settings and the download is continuing.';
      continue;
    }
    if (normalized.includes('syncroot is')) {
      progress = Math.max(progress, 34);
      title = 'Inspecting repository state';
      detail = 'git-ftp is comparing the local checkout with the remote deployment history.';
      continue;
    }
    if (
      normalized.includes('download') ||
      normalized.includes('retr') ||
      normalized.includes('listing') ||
      normalized.includes('fetch') ||
      normalized.includes('mirror') ||
      normalized.includes('receiving')
    ) {
      progress = Math.max(progress, 66);
      title = 'Downloading remote files';
      detail = line;
      continue;
    }
    if (
      normalized.includes('updated') ||
      normalized.includes('created') ||
      normalized.includes('changed') ||
      normalized.includes('writing') ||
      normalized.includes('applying')
    ) {
      progress = Math.max(progress, 82);
      title = 'Applying remote changes locally';
      detail = line;
    }
  }

  if (run.finishedAt) {
    if (run.success) {
      return {
        progress: 100,
        title: 'Remote sync finished',
        detail:
          run.changedFiles.length > 0
            ? `${run.changedFiles.length} file${run.changedFiles.length === 1 ? '' : 's'} changed from the remote server.`
            : 'The remote sync finished without tracked file changes.',
        tone: 'success'
      };
    }

    return {
      progress: Math.max(progress, 92),
      title: 'Remote sync failed',
      detail: latestLine,
      tone: 'error'
    };
  }

  return {
    progress,
    title,
    detail: detail === 'Discarding local edits and establishing the remote session.' ? latestLine : detail,
    tone: 'running'
  };
};

export const hasDraftChanges = (
  draft: ProfileDraft,
  profile?: DeploymentProfile | null
): boolean => {
  if (!profile) {
    return Boolean(
      draft.name ||
        draft.host ||
        draft.username ||
        (draft.remotePath && draft.remotePath !== '/public_html')
    );
  }

  return JSON.stringify(draftFromProfile(profile)) !== JSON.stringify(draft);
};
