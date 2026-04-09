# Git FTP Desktop

Git FTP Desktop is a cross-platform Tauri desktop app for teams that already rely on [`git-ftp`](https://github.com/git-ftp/git-ftp) and want a safer, more legible GUI around it. The app keeps `git` as a system prerequisite, bundles `git-ftp` and `lftp` in release builds, stores secrets in the OS credential store, and runs deployment commands through a Rust backend.

Created by [Dinis Correia](https://github.com/dinisjcorreia).

## Current Status

- Tauri v2 desktop app with React + TypeScript frontend and Rust backend
- Release workflow scaffolded for:
  - macOS Apple Silicon
  - macOS Intel
  - Windows x64
  - Windows ARM64
  - Ubuntu x64 and ARM64
  - Debian x64 and ARM64
  - Fedora x64 and ARM64
- Release packaging keeps `git` external, bundles `git-ftp` and `lftp`, and includes third-party notices and license files

## Key Features

- Environment diagnostics for `git`, `git-ftp`, and `lftp`
- Local repository selection, validation, and remembered repository list
- Multiple deployment profiles per repository
- Profile validation with command previews and optional remote probe feedback
- Secure password storage via OS keychain / credential vault
- Optional repo-local `git config` defaults for stable connection values
- Tracked-change review with an in-app commit flow before deployment
- Live stdout and stderr streaming from the backend into the UI
- Support for `git ftp init`, `git ftp push`, `git ftp catchup`, and remote snapshot bootstrap flows
- Destructive remote sync flow that discards local edits and downloads the current server state after confirmation
- Remote snapshot recovery tooling, including remote `.git-ftp.log` cleanup support
- Dry-run and verbose execution controls
- Run history with redacted command previews and copyable debug reports
- Repository removal controls, including optional folder deletion

## Runtime Requirements

### End-user requirements

- `git` must be installed on the target machine
- Release builds are designed to bundle:
  - `git-ftp`
  - `lftp`

### Development requirements

- Node.js 20+
- npm 10+
- Rust stable
- Tauri v2 system prerequisites for your platform

### Local development tooling notes

- `git` is required for repository inspection, commits, and all deployment actions
- `git-ftp` must be available either from a prepared bundled asset or from your local `PATH`
- `lftp` is only required for snapshot download / remote bootstrap flows and remote cleanup operations
- The repository ships placeholder bundled binaries for development, so fresh local clones usually rely on system-installed `git-ftp` and `lftp` until the release prep scripts are run

### Typical `git` install examples

#### macOS

```bash
brew install git
```

#### Debian / Ubuntu

```bash
sudo apt install git
```

#### Fedora

```bash
sudo dnf install git
```

#### Windows

Install Git for Windows and make sure the final executable is available to GUI apps, not only one shell session.

## Development

### Install dependencies

```bash
npm install
```

### Run the desktop app

```bash
npm run tauri dev
```

### Build the frontend

```bash
npm run build
```

### Check the Rust backend

```bash
cargo check --manifest-path src-tauri/Cargo.toml --color always
```

### Build the desktop bundle

```bash
npm run tauri build
```

## Release Packaging

### Tooling policy

- `git` stays external
- `git-ftp` is bundled from `src-tauri/resources/third-party/tools/git-ftp/git-ftp`
- `lftp` is bundled from `src-tauri/binaries/`

### Release prep scripts

- [prepare-git-ftp.sh](scripts/prepare-git-ftp.sh)
- [prepare-lftp-unix.sh](scripts/prepare-lftp-unix.sh)
- [prepare-lftp-windows.sh](scripts/prepare-lftp-windows.sh)

These scripts fetch or copy the release-time bundled tools and their upstream license texts before public artifacts are built.

### Release workflow

The GitHub Actions workflow lives at [release.yml](.github/workflows/release.yml).

It currently includes jobs for:

- macOS `.dmg` bundles for Apple Silicon and Intel
- Windows installer bundles for x64 and ARM64
- Ubuntu `.deb` bundles for x64 and ARM64
- Debian `.deb` bundles for x64 and ARM64
- Fedora `.rpm` bundles for x64 and ARM64

### Signing requirements

#### macOS

To remove Gatekeeper warnings, configure these GitHub secrets:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `KEYCHAIN_PASSWORD`

Without those secrets, the workflow falls back to ad-hoc signing for macOS builds so the bundled app
still has a valid code signature, but Gatekeeper may still require a manual allow because the build is
not notarized.

#### Windows

To sign installers and reduce SmartScreen warnings, configure:

- `WINDOWS_CERTIFICATE_PFX`
- `WINDOWS_CERTIFICATE_PASSWORD`

### Known release caveat

- Windows ARM64 currently assumes x64 `lftp.exe` may run under Windows-on-Arm x64 emulation unless a native ARM64 `lftp` package is supplied to the workflow

## Licensing and Redistribution

Release packaging includes third-party notices and license files for bundled tools.

- Notices file: [THIRD_PARTY_NOTICES.md](src-tauri/resources/third-party/THIRD_PARTY_NOTICES.md)
- Release packaging guide: [release-packaging.md](docs/release-packaging.md)

Bundled third-party components currently documented:

- `git-ftp`
- `lftp`

## App Behavior

### Startup diagnostics

On launch, the app:

1. Repairs the desktop process `PATH` where possible
2. Resolves required tools from bundled resources, `PATH`, and common install locations
3. Captures version and path information for diagnostics
4. Loads remembered repositories and run history

### Repository and profile behavior

- Repositories can be opened, remembered, removed, or deleted from disk
- Multiple deployment profiles can be stored per repository
- Passwords are stored separately from profile metadata using the backend secret store
- Profiles can be validated before saving, including redacted command previews and optional remote probe output
- Profiles can persist stable connection settings into repo-local Git config before execution
- Remembered repositories are sorted by most recently opened activity

### Deployment actions

Supported workflows include:

- `git ftp init`
- `git ftp push`
- `git ftp catchup`
- destructive remote sync into an existing local repository after confirmation
- remote snapshot bootstrap into a local Git repository
- remote `.git-ftp.log` cleanup when replacing an old source of truth

### Repo-level deploy cleanup

The repository includes a root [`.git-ftp-ignore`](.git-ftp-ignore) file so large runtime and export artifacts do not get redeployed during `git ftp init` or `git ftp push`.

Current exclusions focus on deployment noise rather than source files:

- `sessions/`
- `*.log`
- `sync_state.json`
- `wc-product-export-*.csv`
- `*.WordPress.*.xml`
- desktop metadata such as `.DS_Store` and `Thumbs.db`

### Commit and history behavior

- The main changes view focuses on tracked files only and keeps untracked files out of the commit review list
- Deployment actions are intended to run from a clean working tree after committing tracked changes
- Run history stores redacted command previews, streamed logs, exit status, and changed-file summaries
- The persisted run history is capped to the most recent 60 runs

### Snapshot retry behavior

If snapshot bootstrap fails before completion, the backend removes partial local content so the same destination folder can be reused immediately on the next attempt.

## Security Model

- Passwords are not stored in tracked files
- Secrets go through the Rust backend and OS credential store
- Command execution uses argument vectors, not one giant shell string
- Command previews redact sensitive values
- Debug logs may still contain operationally sensitive hostnames, usernames, paths, and server responses

## Project Structure

```text
.
├── src
│   ├── components
│   ├── lib
│   ├── store
│   ├── App.tsx
│   ├── main.tsx
│   ├── styles.css
│   └── types.ts
├── src-tauri
│   ├── binaries
│   ├── capabilities
│   ├── icons
│   ├── resources
│   ├── src
│   ├── Cargo.toml
│   └── tauri.conf.json
├── scripts
├── docs
└── .github/workflows
```
