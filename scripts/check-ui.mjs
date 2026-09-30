// Browser smoke check. Tauri is mocked: no local files or remote servers are changed.
// First run: npx playwright install chromium
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const server = await createServer({ server: { host: '127.0.0.1', port: 1421, strictPort: true } });
await server.listen();
let browser;
try {
  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1240, height: 760 } });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.addInitScript(() => {
    window.uiCalls = [];
    const flags = {
      syncroot: '',
      remoteRoot: '',
      activeMode: false,
      disableEpsv: false,
      insecure: false,
      autoInit: false,
      useAll: false
    };
    const profile = {
      id: 'prod',
      name: 'Production',
      protocol: 'sftp',
      host: 'sftp.example.com',
      port: 22,
      username: 'deploy',
      remotePath: '/public_html',
      secretRef: 'test',
      useGitConfigDefaults: false,
      flags,
      lastDeployedAt: '2026-09-30T09:12:00Z',
      createdAt: '2026-09-01T10:00:00Z',
      updatedAt: '2026-09-30T09:12:00Z'
    };
    const repo = {
      path: '/Users/demo/Projects/studio-website',
      isGitRepo: true,
      currentBranch: 'main',
      remoteOrigin: 'https://github.com/example/studio-website',
      dirty: true,
      statusSummary: [
        ' M src/pages/index.tsx',
        ' M src/styles/site.scss',
        ' A public/images/project-cover.webp',
        ' D public/old-banner.jpg',
        ' R src/components/Menu.tsx -> src/components/Navigation.tsx'
      ]
    };
    const repositories = location.search.includes('empty')
      ? []
      : [
          { path: repo.path, profileCount: 2, lastOpenedAt: '2026-09-30T09:12:00Z' },
          { path: '/Users/demo/Projects/client-portal', profileCount: 1, lastOpenedAt: '2026-09-28T09:12:00Z' }
        ];
    const diagnostics = {
      os: 'macOS',
      arch: 'arm64',
      gitAvailable: true,
      gitFtpAvailable: true,
      overallReady: true,
      dependencies: ['git', 'git-ftp', 'lftp'].map((name) => ({
        name,
        command: name,
        required: true,
        installed: true,
        version: 'available',
        stdout: '',
        stderr: '',
        installHint: ''
      }))
    };
    const history = [
      {
        id: 'run1',
        repoPath: repo.path,
        profileId: 'prod',
        profileName: 'Production',
        action: 'push',
        startedAt: '2026-09-30T09:12:00Z',
        finishedAt: '2026-09-30T09:12:04Z',
        success: true,
        exitCode: 0,
        commandPreview: 'git ftp push --scope production',
        logs: [
          {
            timestamp: '2026-09-30T09:12:00Z',
            stream: 'system',
            kind: 'system',
            line: 'Connecting to sftp.example.com…'
          },
          {
            timestamp: '2026-09-30T09:12:01Z',
            stream: 'stdout',
            kind: 'info',
            line: 'Uploading 4 files from your latest commit.'
          },
          { timestamp: '2026-09-30T09:12:02Z', stream: 'stdout', kind: 'info', line: 'Uploading: src/pages/index.tsx' },
          {
            timestamp: '2026-09-30T09:12:04Z',
            stream: 'stdout',
            kind: 'info',
            line: 'Deployment complete. Remote is up to date.'
          }
        ],
        cancelled: false,
        changedFiles: []
      }
    ];
    window.testData = { profile, repo, repositories, diagnostics, history };
    window.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (cmd, args) => {
        window.uiCalls.push({ cmd, args });
        if (cmd === 'startup_diagnostics') return diagnostics;
        if (cmd === 'list_known_repositories') return repositories;
        if (cmd === 'validate_repo') return { ...repo, path: args.repoPath };
        if (cmd === 'list_profiles') return [profile];
        if (cmd === 'fetch_run_history') return history;
        if (cmd === 'plugin:event|listen') return 1;
        if (cmd === 'plugin:event|unlisten') return;
        if (cmd === 'plugin:dialog|open') return repo.path;
        if (cmd === 'plugin:dialog|confirm') return false;
        if (cmd === 'validate_profile')
          return { valid: true, messages: [{ kind: 'info', text: 'Connection successful.' }], repo, diagnostics };
        if (cmd === 'save_profile') return Object.assign(profile, args.draft);
        if (cmd === 'commit_tracked_changes') {
          repo.dirty = false;
          repo.statusSummary = [];
          return { commitSha: 'abc1234', summary: args.message, repo };
        }
        if (cmd === 'run_git_ftp_action') return { ...history[0], id: 'run2', action: args.request.action };
        if (cmd === 'generate_debug_report') return { diagnostics, repo, recentHistory: history };
        throw new Error(`Unexpected command ${cmd}`);
      }
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  });

  await page.goto('http://127.0.0.1:1421');
  await page.getByRole('heading', { name: 'Review your changes' }).waitFor();
  assert.equal(await page.locator('.change-review__item').count(), 5);
  assert.equal(await page.getByRole('button', { name: 'Push changes', exact: true }).isEnabled(), false);
  await page.getByLabel('Commit message', { exact: true }).fill('Update site');
  await page.getByRole('button', { name: 'Commit tracked changes', exact: true }).click();
  await page.getByRole('heading', { name: 'All caught up' }).waitFor();
  assert.equal(await page.getByRole('button', { name: 'Push changes', exact: true }).isEnabled(), true);
  assert.equal(await page.getByRole('button', { name: 'Commit tracked changes', exact: true }).isEnabled(), false);
  await page.getByLabel('Dry run', { exact: true }).check();
  await page.getByRole('button', { name: 'Push changes', exact: true }).click();
  await page.getByRole('heading', { name: 'All caught up' }).waitFor();
  assert.equal(
    await page.evaluate(
      () => window.uiCalls.find(({ cmd }) => cmd === 'run_git_ftp_action').args.request.options.dryRun
    ),
    true
  );

  await page.getByRole('button', { name: 'History', exact: true }).click();
  await page.getByRole('heading', { name: 'Deployment history' }).waitFor();
  assert.match(await page.locator('.console-body').innerText(), /Deployment complete/);
  await page.evaluate(async () => {
    const { useAppStore } = await import('/src/store/appStore.ts');
    const run = window.testData.history[0];
    useAppStore.setState({
      runHistory: [
        { ...run, id: 'running', finishedAt: null, success: false },
        { ...run, id: 'cancelled', cancelled: true, success: false },
        { ...run, id: 'failed', success: false }
      ],
      activeRun: null
    });
  });
  await page.getByText('Running', { exact: true }).waitFor();
  await page.getByText('Cancelled', { exact: true }).waitFor();
  await page.getByText('Failed', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByLabel('Profile name', { exact: true }).fill('Staging');
  await page.getByRole('button', { name: 'Save profile', exact: true }).click();
  await page.getByRole('heading', { name: 'Staging', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Test connection', exact: true }).click();
  await page.getByText('Validation passed', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Environment', exact: true }).click();
  await page.getByText('Environment ready', { exact: true }).waitFor();

  await page.getByRole('button', { name: 'Import from FTP', exact: true }).click();
  await page.getByRole('button', { name: 'Download site', exact: true }).click();
  await page.getByRole('heading', { name: 'Password is required.', exact: true }).waitFor();
  await page.getByText('Advanced transport settings', { exact: true }).click();
  await page.getByRole('checkbox', { name: /Allow insecure TLS/ }).check();
  assert.equal(await page.getByRole('checkbox', { name: /Allow insecure TLS/ }).isChecked(), true);

  // Empty onboarding and narrow layouts must remain usable without horizontal scrolling.
  await page.goto('http://127.0.0.1:1421?empty');
  await page.getByRole('heading', { name: 'Connect your site' }).waitFor();
  await page.getByLabel('Profile name', { exact: true }).focus();
  await page.keyboard.press('Tab');
  assert.equal(
    await page.getByRole('combobox', { name: /^Protocol/ }).evaluate((element) => element === document.activeElement),
    true
  );
  await page.getByRole('button', { name: 'Download site', exact: true }).click();
  await page.getByRole('heading', { name: 'Profile name is required.', exact: true }).waitFor();
  await page.setViewportSize({ width: 390, height: 844 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.getByRole('button', { name: 'Open repository', exact: true }).click();
  await page.getByRole('heading', { name: 'Review your changes' }).waitFor();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.getByRole('button', { name: 'History', exact: true }).click();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.getByRole('button', { name: 'Go to start', exact: true }).click();
  await page.getByRole('heading', { name: 'Connect your site' }).waitFor();
  assert.deepEqual(errors, []);
  console.log(
    'UI smoke check passed: review, commit, dry run, history, profiles, import validation, responsive layouts.'
  );
} finally {
  await browser?.close();
  await server.close();
}
