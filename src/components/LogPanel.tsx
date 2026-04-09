import clsx from 'clsx';
import type { RunRecord } from '../types';
import { formatDateTime, formatRunTitle, stringifyLogs } from '../lib/utils';

type Props = {
  activeRun: RunRecord | null;
  runHistory: RunRecord[];
  selectedRunId: string | null;
  onSelectRun: (runId: string) => void;
};

export function LogPanel({ activeRun, runHistory, selectedRunId, onSelectRun }: Props) {
  const displayedRun =
    activeRun && !activeRun.finishedAt
      ? activeRun
      : runHistory.find((run) => run.id === selectedRunId) ?? runHistory[0] ?? activeRun;

  return (
    <section className="panel panel--paper history-shell">
      <div className="panel__header">
        <div>
          <p className="eyebrow">History</p>
          <h2>Recent git-ftp activity</h2>
        </div>
        {displayedRun ? (
          <button className="ghost-button" onClick={() => navigator.clipboard.writeText(stringifyLogs(displayedRun))} type="button">
            Copy logs
          </button>
        ) : null}
      </div>

      <div className="log-shell">
        <div className="log-shell__history">
          <div className="section-heading">
            <span>Runs</span>
            <span>{runHistory.length}</span>
          </div>
          <div className="history-list">
            {runHistory.length === 0 ? (
              <div className="sidebar-empty sidebar-empty--compact">
                <p>No deployment history yet.</p>
              </div>
            ) : (
              runHistory.map((run) => (
                <button
                  key={run.id}
                  type="button"
                  className={clsx('history-card', run.id === displayedRun?.id && 'history-card--active')}
                  onClick={() => onSelectRun(run.id)}
                >
                  <div className="history-card__line">
                    <strong>{formatRunTitle(run)}</strong>
                    <span className={run.success ? 'history-pill history-pill--ok' : 'history-pill history-pill--error'}>
                      {run.success ? 'OK' : 'FAIL'}
                    </span>
                  </div>
                  <p>{formatDateTime(run.startedAt)}</p>
                </button>
              ))
            )}
          </div>
        </div>

        <div className="log-shell__console">
          <div className="console-header">
            <div>
              <p className="eyebrow">Console</p>
              <h3>{displayedRun ? formatRunTitle(displayedRun) : 'Awaiting deployment activity'}</h3>
            </div>
          </div>

          <div className="console-body">
            {displayedRun ? (
              <>
                <div className="console-command">{displayedRun.commandPreview}</div>
                {displayedRun.action === 'download' ? (
                  <details className="console-change-summary">
                    <summary className="console-change-summary__header">
                      <strong>Remote changes</strong>
                      <span>{displayedRun.changedFiles.length}</span>
                    </summary>
                    {displayedRun.changedFiles.length > 0 ? (
                      <div className="console-change-summary__list">
                        {displayedRun.changedFiles.map((file) => (
                          <code key={file}>{file}</code>
                        ))}
                      </div>
                    ) : (
                      <p className="console-change-summary__empty">No tracked remote file changes were reported for this sync.</p>
                    )}
                  </details>
                ) : null}
                <div className="console-lines">
                  {displayedRun.logs.map((entry, index) => (
                    <div key={`${entry.timestamp}-${index}`} className={`console-line console-line--${entry.kind}`}>
                      <span className="console-line__meta">{new Date(entry.timestamp).toLocaleTimeString()}</span>
                      <span className="console-line__stream">{entry.stream}</span>
                      <span>{entry.line}</span>
                    </div>
                  ))}
                </div>
              </>
            ) : (
              <div className="console-empty">
                <p>The terminal panel will stream stdout and stderr here in real time.</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
