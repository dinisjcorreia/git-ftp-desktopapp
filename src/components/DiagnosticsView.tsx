import type { StartupDiagnostics } from '../types';

type Props = {
  diagnostics: StartupDiagnostics | null;
};

export function DiagnosticsView({ diagnostics }: Props) {
  if (!diagnostics) {
    return (
      <div className="diagnostic-card">
        <p className="eyebrow">Environment</p>
        <h2>Checking local tooling…</h2>
      </div>
    );
  }

  const missingRequired = diagnostics.dependencies.filter((dependency) => dependency.required && !dependency.installed);
  const missingOptional = diagnostics.dependencies.filter((dependency) => !dependency.required && !dependency.installed);

  if (missingRequired.length === 0 && missingOptional.length === 0) {
    return (
      <div className="diagnostic-banner diagnostic-banner--ok">
        <div>
          <p className="eyebrow">Environment ready</p>
          <strong>
            {diagnostics.os} / {diagnostics.arch}
          </strong>
        </div>
        <div className="dependency-pill-row">
          {diagnostics.dependencies.map((dependency) => (
            <span key={dependency.name} className="dependency-pill dependency-pill--ok">
              {dependency.name} · {dependency.version ?? 'available'}
            </span>
          ))}
        </div>
      </div>
    );
  }

  if (missingRequired.length === 0) {
    return (
      <section className="diagnostic-card">
        <p className="eyebrow">Environment warning</p>
        <h2>Core tools are ready, but snapshot downloads need one more dependency</h2>
        <p className="diagnostic-intro">
          Standard git-ftp deployment actions are available. Remote bootstrap downloads via
          <code> git-ftp snapshot </code>
          need the optional dependency below.
        </p>
        <div className="dependency-grid">
          {diagnostics.dependencies.map((dependency) => (
            <article
              key={dependency.name}
              className={`dependency-card ${dependency.installed ? 'dependency-card--ok' : 'dependency-card--error'}`}
            >
              <div className="dependency-card__header">
                <strong>{dependency.name}</strong>
                <span>{dependency.installed ? 'Installed' : dependency.required ? 'Missing' : 'Optional'}</span>
              </div>
              <code>{dependency.command}</code>
              <p>{dependency.installed ? dependency.version ?? dependency.stdout : dependency.installHint}</p>
              {dependency.resolvedPath ? (
                <p className="muted">Resolved path: {dependency.resolvedPath}</p>
              ) : null}
              {dependency.stderr ? <pre>{dependency.stderr}</pre> : null}
            </article>
          ))}
        </div>
      </section>
    );
  }

  return (
    <section className="diagnostic-card">
      <p className="eyebrow">Setup required</p>
      <h2>Required local tools are missing</h2>
      <p className="diagnostic-intro">
        The app will stay usable for profile setup and documentation, but deployment actions stay disabled until
        the required tools are available on your PATH.
      </p>
      <div className="dependency-grid">
        {diagnostics.dependencies.map((dependency) => (
          <article
            key={dependency.name}
            className={`dependency-card ${dependency.installed ? 'dependency-card--ok' : 'dependency-card--error'}`}
          >
            <div className="dependency-card__header">
              <strong>{dependency.name}</strong>
              <span>{dependency.installed ? 'Installed' : dependency.required ? 'Missing' : 'Optional'}</span>
            </div>
            <code>{dependency.command}</code>
            <p>{dependency.installed ? dependency.version ?? dependency.stdout : dependency.installHint}</p>
            {dependency.resolvedPath ? (
              <p className="muted">Resolved path: {dependency.resolvedPath}</p>
            ) : null}
            {dependency.stderr ? <pre>{dependency.stderr}</pre> : null}
          </article>
        ))}
      </div>
    </section>
  );
}
