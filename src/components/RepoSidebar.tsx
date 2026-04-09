import clsx from 'clsx';
import type { MouseEvent } from 'react';
import { openExternalUrl } from '../lib/tauri';
import type { RepoInfo, SavedRepository } from '../types';
import { formatDateTime } from '../lib/utils';
import appLogo from '../assets/app-logo.png';

type Props = {
  repoInfo: RepoInfo | null;
  knownRepositories: SavedRepository[];
  selectedRepoPath: string | null;
  profileCount: number;
  environmentLabel: string;
  environmentTone: 'neutral' | 'success' | 'warning' | 'danger';
  onChooseRepo: () => void;
  onSelectRepository: (repoPath: string) => void;
  onShowBootstrapImport: () => void;
  onDeleteRepository: (repoPath: string) => void;
};

export function RepoSidebar({
  repoInfo,
  knownRepositories,
  selectedRepoPath,
  profileCount,
  environmentLabel,
  environmentTone,
  onChooseRepo,
  onSelectRepository,
  onShowBootstrapImport,
  onDeleteRepository
}: Props) {
  const handleCreditsLinkClick = (event: MouseEvent<HTMLAnchorElement>) => {
    event.preventDefault();
    void openExternalUrl('https://github.com/dinisjcorreia');
  };

  return (
    <aside className="sidebar">
      <div className="sidebar__masthead">
        <div className="sidebar__brand">
          <img alt="Git FTP Desktop" className="sidebar__logo" src={appLogo} />
          <div>
            <p className="sidebar__label">Git FTP Desktop</p>
            <h1>Deploy</h1>
          </div>
        </div>
        <p className="sidebar__copy">A quieter, GitHub Desktop-style workspace for repos, profiles, and FTP sync runs.</p>
      </div>

      <section className="sidebar__section repo-panel repo-panel--current">
        <div className="section-heading">
          <span>Current repository</span>
          <button className="ghost-button" onClick={onChooseRepo} type="button">
            {repoInfo ? 'Switch' : 'Attach'}
          </button>
        </div>

        {repoInfo ? (
          <div className="repo-summary">
            <strong className="repo-summary__name">{repoInfo.path.split(/[\\/]/).filter(Boolean).pop() ?? repoInfo.path}</strong>
            <p className="repo-summary__path">{repoInfo.path}</p>
            <div className="repo-meta">
              <span className="repo-clean">{repoInfo.currentBranch ?? 'Detached HEAD'}</span>
              <span className={repoInfo.dirty ? 'repo-dirty' : 'repo-clean'}>{repoInfo.dirty ? 'Changes pending' : 'Clean'}</span>
            </div>
            {repoInfo.remoteOrigin ? <p className="repo-remote">{repoInfo.remoteOrigin}</p> : null}
          </div>
        ) : (
          <div className="sidebar-empty sidebar-empty--compact">
            <p>No repository attached yet. Import from FTP or point the app at an existing local Git repo.</p>
          </div>
        )}
      </section>

      <section className="sidebar__section sidebar__section--stats">
        <div className={`sidebar-stat sidebar-stat--${environmentTone}`}>
          <span>Environment</span>
          <strong>{environmentLabel}</strong>
        </div>
        <div className="sidebar-stat">
          <span>Profiles</span>
          <strong>{profileCount}</strong>
        </div>
      </section>

      <section className="sidebar__section">
        <div className="section-heading">
          <span>Quick actions</span>
        </div>
        <div className="sidebar-action-list">
          <button className="ghost-button ghost-button--block" onClick={onShowBootstrapImport} type="button">
            Import from FTP
          </button>
          <button className="ghost-button ghost-button--block" onClick={onChooseRepo} type="button">
            Open local repo
          </button>
        </div>
      </section>

      <section className="sidebar__section sidebar__section--grow">
        <div className="section-heading">
          <span>Recent repositories</span>
          <span>{knownRepositories.length}</span>
        </div>
        <div className="profile-list">
          {knownRepositories.length === 0 ? (
            <div className="sidebar-empty sidebar-empty--compact">
              <p>Your recently opened repositories will appear here for quick switching.</p>
            </div>
          ) : (
            knownRepositories.map((repository) => (
              <div
                key={repository.path}
                className={clsx('repo-card', selectedRepoPath === repository.path && 'repo-card--active')}
              >
                <button
                  type="button"
                  className="repo-card__body"
                  onClick={() => onSelectRepository(repository.path)}
                >
                  <div className="repo-card__topline">
                    <strong>{repository.path.split(/[\\/]/).filter(Boolean).pop() ?? repository.path}</strong>
                    <span>{repository.profileCount}</span>
                  </div>
                  <p>{repository.path}</p>
                  <small>Last opened {formatDateTime(repository.lastOpenedAt)}</small>
                </button>
                <button
                  aria-label={`Delete ${repository.path}`}
                  className="repo-card__delete"
                  onClick={() => onDeleteRepository(repository.path)}
                  type="button"
                >
                  Delete
                </button>
              </div>
            ))
          )}
        </div>
      </section>

      <section className="sidebar__section sidebar__credits">
        <div className="section-heading">
          <span>Credits</span>
        </div>
        <p className="sidebar__copy">
          Created by{' '}
          <a
            className="sidebar__link"
            href="https://github.com/dinisjcorreia"
            onClick={handleCreditsLinkClick}
            rel="noreferrer"
          >
            Dinis Correia
          </a>
          .
        </p>
      </section>
    </aside>
  );
}
