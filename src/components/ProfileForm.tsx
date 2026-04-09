import type { ProfileDraft, ProfileValidationResult } from '../types';

type Props = {
  draft: ProfileDraft;
  passwordInput: string;
  validation: ProfileValidationResult | null;
  disabled: boolean;
  onChange: (draft: ProfileDraft) => void;
  onPasswordChange: (value: string) => void;
  onSave: () => void;
  onValidate: () => void;
};

export function ProfileForm({
  draft,
  passwordInput,
  validation,
  disabled,
  onChange,
  onPasswordChange,
  onSave,
  onValidate
}: Props) {
  const update = <K extends keyof ProfileDraft>(key: K, value: ProfileDraft[K]) => {
    onChange({ ...draft, [key]: value });
  };

  const updateFlag = (key: keyof ProfileDraft['flags'], value: string | boolean) => {
    onChange({
      ...draft,
      flags: {
        ...draft.flags,
        [key]: value
      }
    });
  };

  return (
    <section className="panel panel--paper settings-form">
      <div className="panel__header">
        <div>
          <p className="eyebrow">Profile editor</p>
          <h2>{draft.id ? 'Deployment profile details' : 'New deployment profile'}</h2>
        </div>
        <div className="button-row">
          <button className="ghost-button" type="button" onClick={onValidate} disabled={disabled}>
            Validate
          </button>
          <button className="primary-button" type="button" onClick={onSave} disabled={disabled}>
            Save
          </button>
        </div>
      </div>

      <section className="form-section">
        <div className="form-section__header">
          <p className="eyebrow">Connection</p>
          <span>Core FTP settings</span>
        </div>
        <div className="profile-form-grid">
          <label>
            <span>Profile name</span>
            <input value={draft.name} onChange={(event) => update('name', event.target.value)} placeholder="Production" />
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
              placeholder={draft.id ? 'Leave blank to keep stored secret' : 'Required for new profiles'}
              type="password"
            />
          </label>
        </div>
      </section>

      <section className="form-section">
        <div className="form-section__header">
          <p className="eyebrow">Behavior</p>
          <span>Deployment defaults</span>
        </div>
        <div className="profile-form-grid">
          <label>
            <span>Upload scope</span>
            <select
              value={draft.flags.useAll ? 'all' : 'delta'}
              onChange={(event) => updateFlag('useAll', event.target.value === 'all')}
            >
              <option value="delta">Only changed tracked files</option>
              <option value="all">All tracked files</option>
            </select>
          </label>
          <label>
            <span>TLS handling</span>
            <select
              value={draft.flags.insecure ? 'relaxed' : 'strict'}
              onChange={(event) => updateFlag('insecure', event.target.value === 'relaxed')}
            >
              <option value="strict">Strict verification</option>
              <option value="relaxed">Allow insecure bypass</option>
            </select>
          </label>
        </div>

        <div className="switch-grid switch-grid--single">
          <label className="switch-card">
            <input
              checked={draft.useGitConfigDefaults}
              onChange={(event) => update('useGitConfigDefaults', event.target.checked)}
              type="checkbox"
            />
            <div>
              <strong>Use git config defaults</strong>
              <p>Persist the stable connection values in repo-local git config before execution.</p>
            </div>
          </label>
          <label className="switch-card">
            <input checked={draft.flags.autoInit} onChange={(event) => updateFlag('autoInit', event.target.checked)} type="checkbox" />
            <div>
              <strong>Enable auto-init</strong>
              <p>Let `git-ftp push` initialize automatically when the remote log does not exist yet.</p>
            </div>
          </label>
        </div>
      </section>

      <details className="advanced-panel">
        <summary>Advanced transport settings</summary>
        <div className="profile-form-grid">
          <label>
            <span>Syncroot</span>
            <input
              value={draft.flags.syncroot ?? ''}
              onChange={(event) => updateFlag('syncroot', event.target.value)}
              placeholder="dist"
            />
          </label>
          <label>
            <span>Remote root override</span>
            <input
              value={draft.flags.remoteRoot ?? ''}
              onChange={(event) => updateFlag('remoteRoot', event.target.value)}
              placeholder="htdocs"
            />
          </label>
          <label className="switch-inline">
            <input checked={draft.flags.activeMode} onChange={(event) => updateFlag('activeMode', event.target.checked)} type="checkbox" />
            <span>Use active FTP mode</span>
          </label>
          <label className="switch-inline">
            <input
              checked={draft.flags.disableEpsv}
              onChange={(event) => updateFlag('disableEpsv', event.target.checked)}
              type="checkbox"
            />
            <span>Disable EPSV fallback</span>
          </label>
        </div>
      </details>

      {validation ? (
        <section className="validation-panel">
          <div className="validation-panel__header">
            <strong>{validation.valid ? 'Validation passed' : 'Validation needs attention'}</strong>
            {validation.commandPreview ? <code>{validation.commandPreview}</code> : null}
          </div>
          <div className="validation-messages">
            {validation.messages.map((message, index) => (
              <div key={`${message.kind}-${index}`} className={`validation-message validation-message--${message.kind}`}>
                {message.text}
              </div>
            ))}
            {validation.probeResult ? (
              <>
                <div className={`validation-message ${validation.probeResult.success ? 'validation-message--info' : 'validation-message--error'}`}>
                  Probe command exit code: {validation.probeResult.exitCode ?? 'unknown'}
                </div>
                {validation.probeResult.stdout?.trim() ? (
                  <div className="validation-output">
                    <span className="validation-output__label">Probe stdout</span>
                    <pre>{validation.probeResult.stdout.trim()}</pre>
                  </div>
                ) : null}
                {validation.probeResult.stderr?.trim() ? (
                  <div className="validation-output validation-output--error">
                    <span className="validation-output__label">Probe stderr</span>
                    <pre>{validation.probeResult.stderr.trim()}</pre>
                  </div>
                ) : null}
              </>
            ) : null}
          </div>
        </section>
      ) : null}
    </section>
  );
}
