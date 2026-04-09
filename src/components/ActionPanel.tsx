import { useMemo, useState } from 'react';
import type { DeploymentProfile, RepoInfo, RunAction, RunOptions, RunRecord, StartupDiagnostics } from '../types';
import { deriveSyncProgress, formatDateTime } from '../lib/utils';

type Props = {
  diagnostics: StartupDiagnostics | null;
  repoInfo: RepoInfo | null;
  profile: DeploymentProfile | null;
  activeRun: RunRecord | null;
  options: RunOptions;
  hasUnsavedChanges: boolean;
  isRunning: boolean;
  isCommitting: boolean;
  commitMessage: string;
  onOptionsChange: (options: RunOptions) => void;
  onCommitMessageChange: (value: string) => void;
  onCommitTrackedChanges: () => void;
  onRun: (action: RunAction) => void;
  onCancel: () => void;
  onCopyDebugReport: () => void;
};

export function ActionPanel({
  diagnostics,
  repoInfo,
  profile,
  activeRun,
  options,
  hasUnsavedChanges,
  isRunning,
  isCommitting,
  commitMessage,
  onOptionsChange,
  onCommitMessageChange,
  onCommitTrackedChanges,
  onRun,
  onCancel,
  onCopyDebugReport
}: Props) {
  const [selectedAction, setSelectedAction] = useState<Exclude<RunAction, 'download'>>('push');
  const trackedChanges = useMemo(
    () =>
      (repoInfo?.statusSummary ?? [])
        .filter((entry) => !entry.startsWith('??'))
        .map((entry) => parseTrackedChange(entry)),
    [repoInfo?.statusSummary]
  );
  const hasDirtyRepo = Boolean(repoInfo?.dirty);
  const hasTrackedChanges = trackedChanges.length > 0;
  const canDeploy =
    Boolean(diagnostics?.gitAvailable) &&
    Boolean(diagnostics?.gitFtpAvailable) &&
    Boolean(repoInfo?.isGitRepo) &&
    Boolean(profile) &&
    !hasUnsavedChanges &&
    !hasDirtyRepo;
  const canCommitTrackedChanges =
    Boolean(repoInfo?.isGitRepo) &&
    hasTrackedChanges &&
    !hasUnsavedChanges &&
    !isCommitting &&
    !isRunning &&
    Boolean(commitMessage.trim());
  const canSyncWithRemote =
    Boolean(diagnostics?.gitAvailable) &&
    Boolean(diagnostics?.gitFtpAvailable) &&
    Boolean(repoInfo?.isGitRepo) &&
    Boolean(profile) &&
    !hasUnsavedChanges &&
    !isCommitting &&
    !isRunning;
  const syncProgress = deriveSyncProgress(activeRun);
  const showSyncProgress = syncProgress && activeRun?.action === 'download' && !activeRun.finishedAt;

  return (
    <section className="panel panel--paper changes-workbench">
      <div className="panel__header changes-workbench__header">
        <div>
          <p className="eyebrow">Changes</p>
          <h2>{trackedChanges.length > 0 ? `${trackedChanges.length} tracked changes ready to review` : 'Working tree is clean'}</h2>
          <p className="workspace__subtitle">
            Keep the main workflow focused: review tracked files, write the commit, and run the selected git-ftp command when you are ready.
          </p>
        </div>

        <div className="changes-workbench__toolbar">
          <label className="switch-inline">
            <input
              checked={options.dryRun}
              onChange={(event) => onOptionsChange({ ...options, dryRun: event.target.checked })}
              type="checkbox"
            />
            <span>Dry run</span>
          </label>
          <label className="switch-inline">
            <input
              checked={options.verbose}
              onChange={(event) => onOptionsChange({ ...options, verbose: event.target.checked })}
              type="checkbox"
            />
            <span>Verbose</span>
          </label>
          <button className="ghost-button" onClick={onCopyDebugReport} type="button">
            Copy debug report
          </button>
        </div>
      </div>

      <div className="changes-workbench__body">
        <section className="changes-list">
          <div className="changes-list__header">
            <div>
              <p className="eyebrow">Tracked files</p>
              <h3>Pending commit set</h3>
            </div>
            <span className="change-review__count">{trackedChanges.length}</span>
          </div>

          {trackedChanges.length > 0 ? (
            <div className="change-review__list">
              {trackedChanges.map((change) => (
                <div key={`${change.status}:${change.path}`} className="change-review__item">
                  <span className={`change-review__status change-review__status--${change.tone}`}>{change.status}</span>
                  <code>{change.path}</code>
                </div>
              ))}
            </div>
          ) : (
            <div className="change-review__empty-state">
              <p className="change-review__empty">No tracked edits are pending right now. Untracked files stay out of this list so the review stays calm.</p>
            </div>
          )}
        </section>

        <aside className="changes-sidebar">
          <section className="panel panel--ink changes-sidebar__section">
            <div className="mini-metrics">
              <div className="metric-card">
                <span>Selected profile</span>
                <strong>{profile?.name ?? 'No profile selected'}</strong>
              </div>
              <div className="metric-card">
                <span>Last deployment</span>
                <strong>{formatDateTime(profile?.lastDeployedAt)}</strong>
              </div>
            </div>

            {hasUnsavedChanges ? (
              <p className="warning-copy">Save the profile first so deploy commands use the latest connection settings.</p>
            ) : null}

            {hasDirtyRepo ? (
              <>
                <p className="warning-copy">
                  git-ftp only deploys committed changes. Commit or stash local changes first, then run the FTP sync.
                </p>
                <button
                  className="action-button action-button--download"
                  disabled={!canSyncWithRemote}
                  onClick={() => onRun('download')}
                  type="button"
                >
                  <span>Sync with remote</span>
                  <small>Discard local edits, download the current server state, and capture the changed remote files.</small>
                </button>
              </>
            ) : null}

            {showSyncProgress ? (
              <section className="run-progress" aria-live="polite">
                <div className="run-progress__meta">
                  <strong>{syncProgress.title}</strong>
                  <span>{Math.round(syncProgress.progress)}%</span>
                </div>
                <div className="run-progress__track" aria-hidden="true">
                  <div
                    className={`run-progress__fill run-progress__fill--${syncProgress.tone}`}
                    style={{ width: `${syncProgress.progress}%` }}
                  />
                </div>
                <p className="run-progress__detail">{syncProgress.detail}</p>
              </section>
            ) : null}

            <div className="commit-box">
              <label className="commit-box__field">
                <span>Commit message</span>
                <input
                  value={commitMessage}
                  onChange={(event) => onCommitMessageChange(event.target.value)}
                  placeholder="Update remote site"
                />
              </label>
              <button
                className="primary-button primary-button--block"
                disabled={!canCommitTrackedChanges}
                onClick={onCommitTrackedChanges}
                type="button"
              >
                {isCommitting ? 'Committing…' : 'Commit tracked changes'}
              </button>
            </div>
          </section>

          <section className="panel panel--paper changes-sidebar__section">
            <div className="changes-command">
              <div>
                <p className="eyebrow">Deploy command</p>
                <h3>Choose one action</h3>
              </div>
              <label className="toolbar-select toolbar-select--stacked">
                <span>Command</span>
                <select
                  onChange={(event) => setSelectedAction(event.target.value as Exclude<RunAction, 'download'>)}
                  value={selectedAction}
                >
                  <option value="push">Push changed files</option>
                  <option value="init">Initialize remote tracking</option>
                  <option value="catchup">Catch up without uploading</option>
                </select>
              </label>
              <button
                className="primary-button primary-button--block"
                disabled={!canDeploy || isRunning || isCommitting}
                onClick={() => onRun(selectedAction)}
                type="button"
              >
                {isRunning ? 'Running…' : 'Run command'}
              </button>
              <p className="changes-command__hint">{describeAction(selectedAction)}</p>
            </div>

            <div className="action-footer">
              <p>Commands run in the Rust backend with the installed `git-ftp`, and the app switches to an encoded-path fallback when exact filenames need spaces or similar characters preserved.</p>
              <button className="ghost-button ghost-button--block" disabled={!isRunning} onClick={onCancel} type="button">
                Cancel active run
              </button>
            </div>
          </section>
        </aside>
      </div>
    </section>
  );
}

function describeAction(action: Exclude<RunAction, 'download'>) {
  if (action === 'init') {
    return 'Uploads all tracked files and creates the initial remote deployment marker.';
  }
  if (action === 'catchup') {
    return 'Updates the remote git-ftp log without uploading files.';
  }
  return 'Deploys only changed files and deletions since the previous successful run.';
}

function parseTrackedChange(entry: string) {
  const normalized = entry.trim();
  const match = normalized.match(/^([A-Z?]{1,2})\s+(.+)$/);
  const code = match?.[1] ?? normalized.slice(0, 2).trim();
  const path = match?.[2]?.trim() ?? normalized;

  if (code.includes('A') || code.includes('C')) {
    return { status: 'Added', path, tone: 'added' as const };
  }
  if (code.includes('D')) {
    return { status: 'Deleted', path, tone: 'deleted' as const };
  }
  if (code.includes('R')) {
    return { status: 'Renamed', path, tone: 'renamed' as const };
  }
  if (code.includes('U')) {
    return { status: 'Conflict', path, tone: 'warning' as const };
  }
  return { status: 'Modified', path, tone: 'modified' as const };
}
