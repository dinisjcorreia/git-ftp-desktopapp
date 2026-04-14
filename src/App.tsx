import { useEffect, useMemo, useRef, useState } from 'react';
import { DiagnosticsView } from './components/DiagnosticsView';
import { RepoSidebar } from './components/RepoSidebar';
import { ProfileForm } from './components/ProfileForm';
import { ActionPanel } from './components/ActionPanel';
import { LogPanel } from './components/LogPanel';
import { ToastViewport } from './components/ToastViewport';
import { ConnectionBootstrapPanel, type BootstrapFeedback } from './components/ConnectionBootstrapPanel';
import * as api from './lib/tauri';
import { createBlankProfileDraft, draftFromProfile, hasDraftChanges } from './lib/utils';
import { useAppStore } from './store/appStore';
import type { ProfileDraft, ProfileValidationResult, RunAction, RunOptions, StartupDiagnostics } from './types';

type WorkspaceTab = 'changes' | 'history' | 'settings';
type SettingsTab = 'profile' | 'environment';

export default function App() {
  const {
    diagnostics,
    selectedRepoPath,
    repoInfo,
    knownRepositories,
    profiles,
    selectedProfileId,
    runHistory,
    activeRun,
    selectedRunId,
    isBootstrapping,
    isRunning,
    isCommitting,
    toasts,
    bootstrap,
    clearWorkspace,
    setSelectedProfileId,
    loadRepo,
    pickRepo,
    refreshRepoInfo,
    removeKnownRepository,
    saveProfile,
    deleteProfile,
    duplicateProfile,
    commitTrackedChanges,
    runAction,
    cancelRun,
    receiveLogEvents,
    dismissToast,
    copyDebugReport
  } = useAppStore();

  const [workspaceTab, setWorkspaceTab] = useState<WorkspaceTab>('changes');
  const [settingsTab, setSettingsTab] = useState<SettingsTab>('profile');
  const [draft, setDraft] = useState<ProfileDraft>(createBlankProfileDraft());
  const [passwordInput, setPasswordInput] = useState('');
  const [validation, setValidation] = useState<ProfileValidationResult | null>(null);
  const [runOptions, setRunOptions] = useState<RunOptions>({ dryRun: false, verbose: true });
  const [commitMessage, setCommitMessage] = useState('Update remote site');
  const [bootstrapLocalPath, setBootstrapLocalPath] = useState('');
  const [isBootstrappingRemote, setIsBootstrappingRemote] = useState(false);
  const [isRemovingRemoteState, setIsRemovingRemoteState] = useState(false);
  const [showBootstrapFlow, setShowBootstrapFlow] = useState(false);
  const bootstrapOperationIdRef = useRef<string | null>(null);
  const bufferedLogEventsRef = useRef<import('./types').LogEvent[]>([]);
  const logFlushTimerRef = useRef<number | null>(null);
  const [bootstrapFeedback, setBootstrapFeedback] = useState<BootstrapFeedback>({
    tone: 'idle',
    title: 'No remote download started yet.',
    detail: 'Fill in the FTP connection and local destination, then run the snapshot download.',
    progress: undefined,
    canRemoveRemoteState: false
  });

  const selectedProfile = profiles.find((profile) => profile.id === selectedProfileId) ?? null;
  const repoName = selectedRepoPath?.split(/[\\/]/).filter(Boolean).pop() ?? null;
  const isSetupMode = !selectedRepoPath || showBootstrapFlow;
  const canGoHome = Boolean(selectedRepoPath || showBootstrapFlow);
  const unsavedChanges = hasDraftChanges(draft, selectedProfile);
  const dependencySummary = useMemo(() => summarizeDependencies(diagnostics), [diagnostics]);
  const trackedChangeCount = useMemo(
    () => (repoInfo?.statusSummary ?? []).filter((entry) => !entry.startsWith('??')).length,
    [repoInfo?.statusSummary]
  );

  useEffect(() => {
    bootstrap();
  }, [bootstrap]);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    const flushBufferedLogs = () => {
      logFlushTimerRef.current = null;
      if (!mounted || bufferedLogEventsRef.current.length === 0) {
        return;
      }

      const batch = bufferedLogEventsRef.current;
      bufferedLogEventsRef.current = [];
      receiveLogEvents(batch);
    };

    void api.onRunLog((event) => {
      if (mounted) {
        bufferedLogEventsRef.current.push(event);
        if (logFlushTimerRef.current == null) {
          logFlushTimerRef.current = window.setTimeout(flushBufferedLogs, 80);
        }
      }
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      mounted = false;
      if (logFlushTimerRef.current != null) {
        window.clearTimeout(logFlushTimerRef.current);
        logFlushTimerRef.current = null;
      }
      bufferedLogEventsRef.current = [];
      unlisten?.();
    };
  }, [receiveLogEvents]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void api.onSnapshotProgress((event) => {
      if (event.operationId !== bootstrapOperationIdRef.current) {
        return;
      }

      setBootstrapFeedback((current) => ({
        ...current,
        tone: event.status === 'success' ? 'success' : event.status === 'error' ? 'error' : 'running',
        title: event.title,
        detail: event.detail,
        commandPreview: event.commandPreview ?? current.commandPreview,
        stdout: event.stream === 'stdout' ? appendConsoleLine(current.stdout, event.line) : current.stdout,
        stderr: event.stream === 'stderr' ? appendConsoleLine(current.stderr, event.line) : current.stderr,
        progress: event.progress
      }));
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (selectedProfile) {
      setDraft(draftFromProfile(selectedProfile));
      setPasswordInput('');
      setValidation(null);
      return;
    }

    if (profiles.length === 0) {
      setDraft(createBlankProfileDraft());
      setPasswordInput('');
      setValidation(null);
    }
  }, [selectedProfile, profiles.length]);

  useEffect(() => {
    if (!selectedRepoPath || showBootstrapFlow || isRunning || workspaceTab === 'history') {
      return;
    }

    void refreshRepoInfo();
    const timer = window.setInterval(() => {
      void refreshRepoInfo();
    }, 4000);

    return () => {
      window.clearInterval(timer);
    };
  }, [selectedRepoPath, showBootstrapFlow, isRunning, workspaceTab, refreshRepoInfo]);

  const handleCreateProfile = () => {
    setWorkspaceTab('settings');
    setSettingsTab('profile');
    setSelectedProfileId(null);
    setDraft(createBlankProfileDraft());
    setPasswordInput('');
    setValidation(null);
  };

  const handlePickRepo = async () => {
    setShowBootstrapFlow(false);
    await pickRepo();
    setWorkspaceTab('changes');
  };

  const handleSelectKnownRepository = async (repoPath: string) => {
    setShowBootstrapFlow(false);
    await loadRepo(repoPath);
    setWorkspaceTab('changes');
  };

  const handleShowBootstrapFlow = () => {
    setShowBootstrapFlow(true);
    setBootstrapFeedback({
      tone: 'idle',
      title: 'No remote download started yet.',
      detail: 'Fill in the FTP connection and local destination, then run the snapshot download.',
      commandPreview: null,
      stdout: '',
      stderr: '',
      progress: undefined,
      canRemoveRemoteState: false
    });
  };

  const handleGoHome = () => {
    clearWorkspace();
    setShowBootstrapFlow(false);
    setWorkspaceTab('changes');
    setSettingsTab('profile');
    setDraft(createBlankProfileDraft());
    setPasswordInput('');
    setValidation(null);
    setRunOptions({ dryRun: false, verbose: true });
    setCommitMessage('Update remote site');
    setBootstrapLocalPath('');
    setIsBootstrappingRemote(false);
    setIsRemovingRemoteState(false);
    bootstrapOperationIdRef.current = null;
    setBootstrapFeedback({
      tone: 'idle',
      title: 'No remote download started yet.',
      detail: 'Fill in the FTP connection and local destination, then run the snapshot download.',
      commandPreview: null,
      stdout: '',
      stderr: '',
      progress: undefined,
      canRemoveRemoteState: false
    });
  };

  const handleChooseLocalPath = async () => {
    const selected = await api.chooseRepoDirectory();
    if (selected) {
      setBootstrapLocalPath(selected);
    }
  };

  const handleBootstrapRemoteRepository = async () => {
    if (!draft.name.trim()) {
      setBootstrapFeedback({
        tone: 'error',
        title: 'Profile name is required.',
        detail: 'Add a profile name before starting the remote snapshot.',
        commandPreview: null,
        stdout: '',
        stderr: '',
        progress: undefined,
        canRemoveRemoteState: false
      });
      return;
    }

    if (!draft.host.trim() || !draft.username.trim() || !draft.remotePath.trim()) {
      setBootstrapFeedback({
        tone: 'error',
        title: 'Connection details are incomplete.',
        detail: 'Host, username, and remote path are required before downloading from the remote site.',
        commandPreview: null,
        stdout: '',
        stderr: '',
        progress: undefined,
        canRemoveRemoteState: false
      });
      return;
    }

    if (!passwordInput.trim()) {
      setBootstrapFeedback({
        tone: 'error',
        title: 'Password is required.',
        detail: 'Enter the FTP password before starting the remote snapshot.',
        commandPreview: null,
        stdout: '',
        stderr: '',
        progress: undefined,
        canRemoveRemoteState: false
      });
      return;
    }

    if (!bootstrapLocalPath.trim()) {
      setBootstrapFeedback({
        tone: 'error',
        title: 'Local destination folder is required.',
        detail: 'Choose an empty local folder where the downloaded Git repository should be created.',
        commandPreview: null,
        stdout: '',
        stderr: '',
        progress: undefined,
        canRemoveRemoteState: false
      });
      return;
    }

    const operationId = crypto.randomUUID();
    bootstrapOperationIdRef.current = operationId;
    setIsBootstrappingRemote(true);
    setBootstrapFeedback({
      tone: 'running',
      title: 'Starting remote snapshot…',
      detail: 'Preparing git-ftp snapshot and validating the local destination folder.',
      commandPreview: null,
      stdout: '',
      stderr: '',
      progress: 4,
      canRemoveRemoteState: false
    });
    await new Promise((resolve) => window.setTimeout(resolve, 0));
    try {
      const result = await api.bootstrapRemoteSnapshot(draft, passwordInput, bootstrapLocalPath, operationId);
      if (!result.success) {
        const failure = summarizeSnapshotFailure(result.stdout, result.stderr, draft.flags.insecure);
        setBootstrapFeedback((current) => ({
          ...current,
          tone: 'error',
          title: failure.title,
          detail: failure.detail,
          commandPreview: result.commandPreview,
          stdout: failure.stdout,
          stderr: failure.stderr,
          canRemoveRemoteState: failure.canRemoveRemoteState
        }));
        return;
      }

      setBootstrapFeedback({
        tone: 'success',
        title: 'Remote snapshot completed.',
        detail: `Downloaded into ${result.localPath}. Loading the repository into the workspace.`,
        commandPreview: result.commandPreview,
        stdout: result.stdout,
        stderr: result.stderr,
        progress: 100,
        canRemoveRemoteState: false
      });

      await loadRepo(result.localPath);
      setShowBootstrapFlow(false);
      setWorkspaceTab('changes');
      setSelectedProfileId(result.profile.id);
      setDraft(draftFromProfile(result.profile));
      setPasswordInput('');
      setValidation(null);
      setBootstrapFeedback((current) => ({
        ...current,
        detail: `Downloaded into ${result.localPath} and loaded into the workspace.`
      }));
    } catch (error) {
      setBootstrapFeedback({
        tone: 'error',
        title: 'Remote snapshot could not start.',
        detail: error instanceof Error ? error.message : String(error),
        commandPreview: null,
        stdout: '',
        stderr: '',
        progress: undefined,
        canRemoveRemoteState: false
      });
    } finally {
      if (bootstrapOperationIdRef.current === operationId) {
        bootstrapOperationIdRef.current = null;
      }
      setIsBootstrappingRemote(false);
    }
  };

  const handleRemoveRemoteState = async () => {
    if (!passwordInput.trim()) {
      setBootstrapFeedback((current) => ({
        ...current,
        tone: 'error',
        title: 'Password is required.',
        detail: 'Enter the FTP password before removing the remote .git-ftp.log file.',
        canRemoveRemoteState: true
      }));
      return;
    }

    const confirmed = await api.confirmDialog(
      'Remove the remote .git-ftp.log file? This will break the old repository deployment history for that FTP path and let this app start a new git-ftp source of truth.',
      { title: 'Remove remote state', kind: 'warning' }
    );
    if (!confirmed) {
      return;
    }

    setIsRemovingRemoteState(true);
    try {
      const result = await api.removeRemoteGitFtpLog(draft, passwordInput);
      if (!result.success) {
        setBootstrapFeedback((current) => ({
          ...current,
          tone: 'error',
          title: 'Could not remove remote .git-ftp.log.',
          detail: 'The cleanup command failed. Review the command output below.',
          commandPreview: result.commandPreview,
          stdout: result.stdout,
          stderr: result.stderr,
          canRemoveRemoteState: true
        }));
        return;
      }

      setBootstrapFeedback((current) => ({
        ...current,
        tone: 'success',
        title: 'Remote .git-ftp.log removed.',
        detail: 'The old deployment marker was deleted. You can retry the snapshot download now.',
        commandPreview: result.commandPreview,
        stdout: result.stdout,
        stderr: result.stderr,
        canRemoveRemoteState: false
      }));
    } catch (error) {
      setBootstrapFeedback((current) => ({
        ...current,
        tone: 'error',
        title: 'Could not remove remote .git-ftp.log.',
        detail: error instanceof Error ? error.message : String(error),
        canRemoveRemoteState: true
      }));
    } finally {
      setIsRemovingRemoteState(false);
    }
  };

  const handleSaveProfile = async () => {
    const saved = await saveProfile(draft, passwordInput);
    setDraft(draftFromProfile(saved));
    setPasswordInput('');
    setValidation(null);
  };

  const handleValidateProfile = async () => {
    if (!selectedRepoPath) {
      return;
    }
    const result = await api.validateProfile(selectedRepoPath, draft, passwordInput, true);
    setValidation(result);
  };

  const handleDeleteProfile = async () => {
    if (!selectedProfileId || !selectedProfile) {
      return;
    }
    const confirmed = await api.confirmDialog(
      `Delete the "${selectedProfile.name}" profile? This also removes the stored secret reference.`,
      { title: 'Delete profile', kind: 'warning' }
    );
    if (!confirmed) {
      return;
    }
    await deleteProfile(selectedProfileId);
    setPasswordInput('');
    setValidation(null);
  };

  const handleDeleteRepository = async (repoPath?: string) => {
    const targetRepoPath = repoPath ?? selectedRepoPath;
    if (!targetRepoPath) {
      return;
    }

    const repoLabel = targetRepoPath.split(/[\\/]/).filter(Boolean).pop() ?? targetRepoPath;
    const confirmed = await api.confirmDialog(
      `Remove "${repoLabel}" from Git FTP Desktop? This forgets its saved profiles and run history in the app.`,
      { title: 'Remove repository', kind: 'warning' }
    );
    if (!confirmed) {
      return;
    }

    const deleteFolder = await api.confirmDialog(
      `Also delete the repository folder from disk?\n\nPress OK to delete the folder too.\nPress Cancel to keep the folder and only remove it from the app.`,
      { title: 'Delete folder too?', kind: 'warning' }
    );

    await removeKnownRepository(targetRepoPath, deleteFolder);
  };

  const handleDuplicateProfile = async () => {
    if (!selectedProfileId) {
      return;
    }
    const duplicated = await duplicateProfile(selectedProfileId);
    setWorkspaceTab('settings');
    setSettingsTab('profile');
    setDraft(draftFromProfile(duplicated));
    setPasswordInput('');
    setValidation(null);
  };

  const handleRunAction = async (action: RunAction) => {
    if (!selectedProfileId) {
      return;
    }
    if (action !== 'push' && action !== 'download') {
      const confirmed = await api.confirmDialog(
        `Run ${action.toUpperCase()} for "${selectedProfile?.name ?? 'this profile'}"?`,
        { title: `Confirm ${action}`, kind: 'warning' }
      );
      if (!confirmed) {
        return;
      }
    }

    const startedFromTab = workspaceTab;
    setWorkspaceTab('history');
    try {
      const result = await runAction(selectedProfileId, action, runOptions, passwordInput);
      if (startedFromTab === 'changes' && result.success) {
        setWorkspaceTab('changes');
      }
    } catch (_error) {
      // The store owns failure toasts; keep the history tab open so the log stays visible.
    }
  };

  const handleCommitTrackedChanges = async () => {
    await commitTrackedChanges(commitMessage);
  };

  const handleProfileSelect = (value: string) => {
    if (value === '__new__') {
      handleCreateProfile();
      return;
    }

    setSelectedProfileId(value || null);
  };

  const renderWorkspaceContent = () => {
    if (isSetupMode) {
      return (
        <section className="workspace-stack">
          <DiagnosticsView diagnostics={diagnostics} />
          <ConnectionBootstrapPanel
            draft={draft}
            passwordInput={passwordInput}
            localPath={bootstrapLocalPath}
            isBusy={isBootstrappingRemote || isRemovingRemoteState}
            feedback={bootstrapFeedback}
            onChange={setDraft}
            onPasswordChange={setPasswordInput}
            onLocalPathChange={setBootstrapLocalPath}
            onChooseLocalPath={handleChooseLocalPath}
            onDownload={handleBootstrapRemoteRepository}
            onRemoveRemoteState={handleRemoveRemoteState}
          />
        </section>
      );
    }

    if (workspaceTab === 'history') {
      return (
        <LogPanel
          activeRun={activeRun}
          runHistory={runHistory}
          selectedRunId={selectedRunId}
          onSelectRun={(runId) => useAppStore.setState({ selectedRunId: runId })}
        />
      );
    }

    if (workspaceTab === 'settings') {
      return (
        <section className="settings-shell">
          <aside className="settings-nav">
            <button
              className={`settings-nav__button ${settingsTab === 'profile' ? 'settings-nav__button--active' : ''}`}
              onClick={() => setSettingsTab('profile')}
              type="button"
            >
              Profile
            </button>
            <button
              className={`settings-nav__button ${settingsTab === 'environment' ? 'settings-nav__button--active' : ''}`}
              onClick={() => setSettingsTab('environment')}
              type="button"
            >
              Environment
            </button>
          </aside>

          <div className="settings-shell__content">
            {settingsTab === 'profile' ? (
              <>
                <section className="panel panel--paper settings-summary">
                  <div className="panel__header">
                    <div>
                      <p className="eyebrow">Selected profile</p>
                      <h2>{selectedProfile?.name ?? 'Create a deployment profile'}</h2>
                    </div>
                    <div className="button-row">
                      <button className="ghost-button" disabled={!selectedProfile} onClick={handleDuplicateProfile} type="button">
                        Duplicate
                      </button>
                      <button
                        className="ghost-button ghost-button--danger"
                        disabled={!selectedProfile}
                        onClick={handleDeleteProfile}
                        type="button"
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                  <div className="summary-grid">
                    <div className="metric-card">
                      <span>Host</span>
                      <strong>{selectedProfile ? `${selectedProfile.host}:${selectedProfile.port}` : 'Not configured'}</strong>
                    </div>
                    <div className="metric-card">
                      <span>Remote path</span>
                      <strong>{selectedProfile?.remotePath ?? 'Not configured'}</strong>
                    </div>
                    <div className="metric-card">
                      <span>Last deployment</span>
                      <strong>{selectedProfile?.lastDeployedAt ? new Date(selectedProfile.lastDeployedAt).toLocaleString() : 'Never'}</strong>
                    </div>
                  </div>
                </section>

                <ProfileForm
                  draft={draft}
                  passwordInput={passwordInput}
                  validation={validation}
                  disabled={!repoInfo?.isGitRepo}
                  onChange={setDraft}
                  onPasswordChange={setPasswordInput}
                  onSave={handleSaveProfile}
                  onValidate={handleValidateProfile}
                />
              </>
            ) : (
              <DiagnosticsView diagnostics={diagnostics} />
            )}
          </div>
        </section>
      );
    }

    return (
      <section className="workspace-stack">
        {!diagnostics?.overallReady ? <DiagnosticsView diagnostics={diagnostics} /> : null}
        <ActionPanel
          diagnostics={diagnostics}
          repoInfo={repoInfo}
          profile={selectedProfile}
          activeRun={activeRun}
          options={runOptions}
          hasUnsavedChanges={unsavedChanges}
          isRunning={isRunning}
          isCommitting={isCommitting}
          commitMessage={commitMessage}
          onOptionsChange={setRunOptions}
          onCommitMessageChange={setCommitMessage}
          onCommitTrackedChanges={handleCommitTrackedChanges}
          onRun={handleRunAction}
          onCancel={cancelRun}
          onCopyDebugReport={() => {
            void copyDebugReport();
          }}
        />
      </section>
    );
  };

  return (
    <div className="app-shell">
      <ToastViewport toasts={toasts} onDismiss={dismissToast} />

      <RepoSidebar
        repoInfo={repoInfo}
        knownRepositories={knownRepositories}
        selectedRepoPath={selectedRepoPath}
        profileCount={profiles.length}
        environmentLabel={dependencySummary.label}
        environmentTone={dependencySummary.tone}
        onChooseRepo={handlePickRepo}
        onSelectRepository={handleSelectKnownRepository}
        onShowBootstrapImport={handleShowBootstrapFlow}
        onDeleteRepository={handleDeleteRepository}
      />

      <main className={`workspace ${isSetupMode ? 'workspace--setup' : ''}`}>
        <header className="workspace__header">
          <div className="workspace__titlebar">
            <div className="workspace__identity">
              <p className="eyebrow">Workspace</p>
              <h1>{repoName ?? 'Git FTP Desktop'}</h1>
              <p className="workspace__subtitle">
                {isSetupMode
                  ? 'Connect to the server and download it into a local Git repository.'
                  : 'A GitHub Desktop-inspired dark workspace with changes, history, and settings tucked into focused views.'}
              </p>
            </div>

            <div className="workspace__toolbar">
              <label className="toolbar-select">
                <span>Repository</span>
                <select
                  disabled={knownRepositories.length === 0}
                  onChange={(event) => {
                    if (event.target.value) {
                      void handleSelectKnownRepository(event.target.value);
                    }
                  }}
                  value={selectedRepoPath ?? ''}
                >
                  <option value="">Select repository</option>
                  {knownRepositories.map((repository) => (
                    <option key={repository.path} value={repository.path}>
                      {repository.path.split(/[\\/]/).filter(Boolean).pop() ?? repository.path}
                    </option>
                  ))}
                </select>
              </label>

              {!isSetupMode ? (
                <label className="toolbar-select">
                  <span>Profile</span>
                  <select
                    disabled={!repoInfo?.isGitRepo}
                    onChange={(event) => handleProfileSelect(event.target.value)}
                    value={selectedProfileId ?? ''}
                  >
                    <option value="">No profile selected</option>
                    {profiles.map((profile) => (
                      <option key={profile.id} value={profile.id}>
                        {profile.name}
                      </option>
                    ))}
                    <option value="__new__">Create new profile…</option>
                  </select>
                </label>
              ) : null}

              <div className="toolbar-actions">
                {canGoHome ? (
                  <button className="ghost-button" onClick={handleGoHome} type="button">
                    Home
                  </button>
                ) : null}
                <button className="ghost-button" onClick={() => void handlePickRepo()} type="button">
                  Open repo
                </button>
                <button className="ghost-button" onClick={handleShowBootstrapFlow} type="button">
                  Import from FTP
                </button>
                {!isSetupMode ? (
                  <button
                    className="ghost-button ghost-button--danger"
                    disabled={!selectedRepoPath}
                    onClick={() => void handleDeleteRepository()}
                    type="button"
                  >
                    Delete repo
                  </button>
                ) : null}
                {!isSetupMode ? (
                  <button
                    className="ghost-button ghost-button--danger"
                    disabled={!selectedProfile}
                    onClick={() => void handleDeleteProfile()}
                    type="button"
                  >
                    Delete profile
                  </button>
                ) : null}
                {!isSetupMode ? (
                  <button className="primary-button" onClick={handleCreateProfile} type="button">
                    New profile
                  </button>
                ) : null}
              </div>
            </div>
          </div>

          <div className="workspace__statusbar">
            <div className="status-pill">
              <span>Branch</span>
              <strong>{repoInfo?.currentBranch ?? 'Not loaded'}</strong>
            </div>
            <div className={`status-pill status-pill--${repoInfo?.dirty ? 'warning' : 'neutral'}`}>
              <span>Working tree</span>
              <strong>{repoInfo?.isGitRepo ? (repoInfo?.dirty ? 'Changes pending' : 'Clean') : 'Unavailable'}</strong>
            </div>
            <div className="status-pill">
              <span>Tracked files</span>
              <strong>{trackedChangeCount}</strong>
            </div>
            <div className={`status-pill status-pill--${dependencySummary.tone}`}>
              <span>Environment</span>
              <strong>{dependencySummary.label}</strong>
            </div>
          </div>

          {!isSetupMode ? (
            <nav className="workspace-tabs" aria-label="Workspace sections">
              {[
                ['changes', 'Changes'],
                ['history', 'History'],
                ['settings', 'Settings']
              ].map(([value, label]) => (
                <button
                  key={value}
                  className={`workspace-tab ${workspaceTab === value ? 'workspace-tab--active' : ''}`}
                  onClick={() => setWorkspaceTab(value as WorkspaceTab)}
                  type="button"
                >
                  {label}
                </button>
              ))}
            </nav>
          ) : null}
        </header>

        <section className="workspace__body">{renderWorkspaceContent()}</section>
      </main>

      {isBootstrapping ? <div className="boot-overlay">Loading diagnostics and repositories…</div> : null}
    </div>
  );
}

function summarizeDependencies(diagnostics: StartupDiagnostics | null) {
  if (!diagnostics) {
    return { label: 'Checking…', tone: 'neutral' as const };
  }

  const missingRequired = diagnostics.dependencies.filter((dependency) => dependency.required && !dependency.installed).length;
  const missingOptional = diagnostics.dependencies.filter((dependency) => !dependency.required && !dependency.installed).length;

  if (missingRequired > 0) {
    return { label: `${missingRequired} required missing`, tone: 'danger' as const };
  }

  if (missingOptional > 0) {
    return { label: `${missingOptional} optional missing`, tone: 'warning' as const };
  }

  return { label: 'Ready', tone: 'success' as const };
}

function summarizeSnapshotFailure(stdout: string, stderr: string, insecureEnabled: boolean) {
  const combined = `${stdout}\n${stderr}`;
  const normalizedStdout = dedupeLines(
    stdout
      .split(/\r?\n/)
      .filter((line) => !line.includes('Git: error committing the changes'))
      .join('\n')
  );
  const normalizedStderr = dedupeLines(
    stderr
      .split(/\r?\n/)
      .filter((line) => !line.includes('Git: error committing the changes'))
      .join('\n')
  );

  if (combined.includes('Certificate verification: subjectAltName does not match')) {
    return {
      title: 'TLS certificate verification failed.',
      detail: insecureEnabled
        ? 'The FTP server certificate still does not match the hostname you entered. Use the hostname the certificate was issued for, or confirm that bypassing certificate checks is acceptable for this server.'
        : 'The FTP server certificate does not match the hostname you entered. Use the hostname the certificate was issued for, or enable "Allow insecure TLS / fingerprint bypass" in Advanced transport settings if you trust this server.',
      stdout: normalizedStdout,
      stderr: normalizedStderr,
      canRemoveRemoteState: false
    };
  }

  if (combined.includes('The remote directory is managed by another Git repository already.')) {
    return {
      title: 'Remote already has git-ftp history.',
      detail:
        'This FTP path already contains a .git-ftp.log from another repository, so git-ftp snapshot refuses to create a new local source of truth. Attach the original local repo if you have it, or delete the remote .git-ftp.log only if you intentionally want this new repo to replace the old deployment history.',
      stdout: normalizedStdout,
      stderr: normalizedStderr,
      canRemoveRemoteState: true
    };
  }

  return {
    title: 'Remote snapshot failed.',
    detail:
      normalizedStderr.trim() || normalizedStdout.trim()
        ? 'git-ftp returned a non-zero exit code. Review the command output below.'
        : 'git-ftp returned a non-zero exit code without additional output.',
    stdout: normalizedStdout,
    stderr: normalizedStderr,
    canRemoveRemoteState: false
  };
}

function dedupeLines(value: string) {
  const seen = new Set<string>();
  return value
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter((line) => {
      if (!line) {
        return false;
      }
      if (seen.has(line)) {
        return false;
      }
      seen.add(line);
      return true;
    })
    .join('\n');
}

function appendConsoleLine(existing: string | undefined, line?: string | null) {
  const next = line?.trimEnd();
  if (!next) {
    return existing ?? '';
  }

  const lines = (existing ?? '')
    .split(/\r?\n/)
    .map((entry) => entry.trimEnd())
    .filter(Boolean);

  if (lines[lines.length - 1] === next) {
    return existing ?? '';
  }

  return [...lines, next].join('\n');
}
