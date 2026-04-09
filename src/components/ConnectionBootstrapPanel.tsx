import type { ProfileDraft } from '../types';

export type BootstrapFeedback = {
  tone: 'idle' | 'running' | 'success' | 'error';
  title: string;
  detail?: string;
  commandPreview?: string | null;
  stdout?: string;
  stderr?: string;
  progress?: number;
  canRemoveRemoteState?: boolean;
};

type Props = {
  draft: ProfileDraft;
  passwordInput: string;
  localPath: string;
  isBusy: boolean;
  feedback: BootstrapFeedback;
  onChange: (draft: ProfileDraft) => void;
  onPasswordChange: (value: string) => void;
  onLocalPathChange: (value: string) => void;
  onChooseLocalPath: () => void;
  onDownload: () => void;
  onRemoveRemoteState: () => void;
};

export function ConnectionBootstrapPanel({
  draft,
  passwordInput,
  localPath,
  isBusy,
  feedback,
  onChange,
  onPasswordChange,
  onLocalPathChange,
  onChooseLocalPath,
  onDownload,
  onRemoveRemoteState
}: Props) {
  const update = <K extends keyof ProfileDraft>(key: K, value: ProfileDraft[K]) => {
    onChange({ ...draft, [key]: value });
  };
  const hasOutput = Boolean(feedback.commandPreview || feedback.stdout || feedback.stderr || feedback.tone !== 'idle');
  const progressValue =
    typeof feedback.progress === 'number' ? Math.max(0, Math.min(100, Math.round(feedback.progress))) : null;

  const outputLines = [
    ...(feedback.detail ? [{ kind: 'system' as const, line: feedback.detail }] : []),
    ...splitLines(feedback.stdout).map((line) => ({ kind: 'info' as const, line })),
    ...splitLines(feedback.stderr).map((line) => ({ kind: 'error' as const, line }))
  ];
  return (
    <section className="panel panel--paper connection-panel">
      <div className="panel__header">
        <div>
          <p className="eyebrow">Step 1 · Remote connection</p>
          <h2>Connect to the FTP site and download it locally</h2>
        </div>
        <button className="primary-button" onClick={() => void onDownload()} disabled={isBusy} type="button">
          {isBusy ? `Downloading…${progressValue !== null ? ` ${progressValue}%` : ''}` : 'Download into local Git repo'}
        </button>
      </div>

      <p className="workspace__subtitle">
        Enter the FTP host, login details, remote folder, and an empty local destination folder first. When you run the download action, the app uses the real
        <code> git-ftp snapshot </code>
        flow to pull the remote site into a local Git repository, then loads that downloaded repository into the workspace.
      </p>

      <div className="profile-form-grid">
        <label>
          <span>Profile name</span>
          <input value={draft.name} onChange={(event) => update('name', event.target.value)} placeholder="Production import" />
        </label>
        <label>
          <span>Protocol</span>
          <select value={draft.protocol} onChange={(event) => update('protocol', event.target.value as ProfileDraft['protocol'])}>
            <option value="ftp">FTP</option>
            <option value="sftp">SFTP</option>
            <option value="ftps">FTPS</option>
            <option value="ftpes">FTPES</option>
          </select>
        </label>
        <label>
          <span>Host</span>
          <input value={draft.host} onChange={(event) => update('host', event.target.value)} placeholder="ftp.example.com" />
        </label>
        <label>
          <span>Port</span>
          <input
            value={draft.port}
            min={1}
            max={65535}
            onChange={(event) => update('port', Number(event.target.value || 0))}
            type="number"
          />
        </label>
        <label>
          <span>Username</span>
          <input value={draft.username} onChange={(event) => update('username', event.target.value)} placeholder="deploy" />
        </label>
        <label>
          <span>Remote path</span>
          <input value={draft.remotePath} onChange={(event) => update('remotePath', event.target.value)} placeholder="/public_html" />
        </label>
        <label className="profile-form-grid__full">
          <span>Password</span>
          <input
            value={passwordInput}
            onChange={(event) => onPasswordChange(event.target.value)}
            placeholder="Required to fetch the remote site"
            type="password"
          />
        </label>
        <label className="profile-form-grid__full">
          <span>Local destination folder</span>
          <div className="path-picker">
            <input
              value={localPath}
              onChange={(event) => onLocalPathChange(event.target.value)}
              placeholder="Choose an empty folder for the downloaded repository"
            />
            <button className="ghost-button" onClick={onChooseLocalPath} type="button">
              Browse
            </button>
          </div>
        </label>
      </div>

      <details className="advanced-panel connection-panel__advanced">
        <summary>Advanced transport settings</summary>
        <div className="switch-grid">
          <label className="switch-card">
            <input
              checked={draft.flags.insecure}
              onChange={(event) =>
                onChange({
                  ...draft,
                  flags: {
                    ...draft.flags,
                    insecure: event.target.checked
                  }
                })
              }
              type="checkbox"
            />
            <div>
              <strong>Allow insecure TLS / fingerprint bypass</strong>
              <p>Use only when the FTP host requires it and you trust the endpoint.</p>
            </div>
          </label>
          <label className="switch-card">
            <input
              checked={draft.flags.disableEpsv}
              onChange={(event) =>
                onChange({
                  ...draft,
                  flags: {
                    ...draft.flags,
                    disableEpsv: event.target.checked
                  }
                })
              }
              type="checkbox"
            />
            <div>
              <strong>Disable EPSV fallback</strong>
              <p>Helpful when the remote server has network quirks with passive FTP negotiation.</p>
            </div>
          </label>
        </div>
      </details>

      {hasOutput ? (
        <details className="bootstrap-inspector" aria-live="polite" open={feedback.tone === 'error' || feedback.tone === 'running'}>
          <summary className="bootstrap-inspector__summary">
            <div>
              <p className="eyebrow">Download inspector</p>
              <h3>{feedback.title}</h3>
            </div>
            <span className={`bootstrap-pill bootstrap-pill--${feedback.tone}`}>{feedback.tone}</span>
          </summary>

          {progressValue !== null ? (
            <div className="bootstrap-progress" aria-label="Remote snapshot progress">
              <div className="bootstrap-progress__meta">
                <strong>{progressValue}%</strong>
                <span>{feedback.tone === 'success' ? 'Snapshot complete' : feedback.tone === 'error' ? 'Stopped before completion' : 'Receiving live snapshot output'}</span>
              </div>
              <div
                className="bootstrap-progress__track"
                aria-valuemax={100}
                aria-valuemin={0}
                aria-valuenow={progressValue}
                role="progressbar"
              >
                <div
                  className={`bootstrap-progress__fill bootstrap-progress__fill--${feedback.tone}`}
                  style={{ width: `${progressValue}%` }}
                />
              </div>
            </div>
          ) : null}

          {feedback.canRemoveRemoteState ? (
            <div className="bootstrap-recovery">
              <p>This remote already has a deployment marker from another git-ftp repo. If you want this new local repo to take over, remove the remote marker and retry.</p>
              <button className="ghost-button" onClick={() => void onRemoveRemoteState()} type="button">
                Remove remote .git-ftp.log
              </button>
            </div>
          ) : null}

          {feedback.commandPreview ? <div className="console-command">{feedback.commandPreview}</div> : null}

          <div className="bootstrap-inspector__body">
            {outputLines.length > 0 ? (
              <div className="console-lines console-lines--compact">
                {outputLines.map((entry, index) => (
                  <div key={`${entry.kind}-${index}`} className={`console-line console-line--${entry.kind}`}>
                    <span className="console-line__stream">{entry.kind}</span>
                    <span>{entry.line}</span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="console-empty">
                <p>The first remote download will show validation notes, the redacted command preview, and any stdout or stderr here.</p>
              </div>
            )}
          </div>
        </details>
      ) : null}
    </section>
  );
}

function splitLines(value?: string) {
  return (value ?? '')
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter(Boolean);
}
