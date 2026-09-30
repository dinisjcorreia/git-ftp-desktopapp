import clsx from 'clsx';
import type { MouseEvent } from 'react';
import { openExternalUrl } from '../lib/tauri';
import type { SavedRepository } from '../types';
import { Icon } from './Icon';
import appLogo from '../assets/app-logo.svg';

type Props = {
  knownRepositories: SavedRepository[];
  selectedRepoPath: string | null;
  environmentLabel: string;
  environmentTone: 'neutral' | 'success' | 'warning' | 'danger';
  onGoHome: () => void;
  onChooseRepo: () => void;
  onSelectRepository: (repoPath: string) => void;
  onShowBootstrapImport: () => void;
  onDeleteRepository: (repoPath: string) => void;
};

export function RepoSidebar({
  knownRepositories,
  selectedRepoPath,
  environmentLabel,
  environmentTone,
  onGoHome,
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
    <aside className="sidebar" aria-label="Repositories">
      <button className="sidebar__brand" type="button" onClick={onGoHome} aria-label="Go to start">
        <img className="brand-mark" src={appLogo} alt="" />
        <div>
          <strong>Git FTP</strong>
          <span>Desktop</span>
        </div>
      </button>

      <div className="sidebar-action-list">
        <button className="sidebar-action" onClick={onChooseRepo} type="button">
          <Icon name="folder" /> Open repository <Icon name="plus" />
        </button>
        <button className="sidebar-action" onClick={onShowBootstrapImport} type="button">
          <Icon name="download" /> Import from FTP
        </button>
      </div>

      <section className="sidebar__repositories">
        <div className="section-heading">
          <h2>Repositories</h2>
          <span>{knownRepositories.length}</span>
        </div>
        <div className="profile-list">
          {knownRepositories.length === 0 ? (
            <div className="sidebar-empty">
              <Icon name="folder" />
              <p>A place for your projects.</p>
              <small>Open a local repository or import your site to get started.</small>
            </div>
          ) : (
            knownRepositories.map((repository) => (
              <div
                key={repository.path}
                className={clsx('repo-card', selectedRepoPath === repository.path && 'repo-card--active')}
              >
                <button
                  className="repo-card__body"
                  type="button"
                  onClick={() => onSelectRepository(repository.path)}
                  aria-current={selectedRepoPath === repository.path ? 'true' : undefined}
                  title={repository.path}
                >
                  <Icon name="folder" />
                  <div>
                    <strong>{repository.path.split(/[\\/]/).filter(Boolean).pop() ?? repository.path}</strong>
                    <small>
                      {repository.profileCount} {repository.profileCount === 1 ? 'profile' : 'profiles'}
                    </small>
                  </div>
                </button>
                <button
                  className="repo-card__delete"
                  aria-label={`Remove ${repository.path}`}
                  title="Remove repository"
                  onClick={() => onDeleteRepository(repository.path)}
                  type="button"
                >
                  <Icon name="close" />
                </button>
              </div>
            ))
          )}
        </div>
      </section>

      <footer className="sidebar__footer">
        <div className={`environment-status environment-status--${environmentTone}`}>
          <span className="status-dot" />
          <span>Environment</span>
          <strong>{environmentLabel}</strong>
        </div>
        <p>
          Made by{' '}
          <a href="https://github.com/dinisjcorreia" onClick={handleCreditsLinkClick} rel="noreferrer">
            Dinis Correia
          </a>
        </p>
      </footer>
    </aside>
  );
}
