# Contributing to Git FTP Desktop

Thanks for taking the time to contribute.

This repository contains a cross-platform Tauri desktop app that provides a
GUI around `git-ftp` workflows. The app keeps `git` as a user-installed
prerequisite, bundles `git-ftp` and `lftp` for release builds, stores secrets
through the OS credential store, and routes deployment work through a Rust
backend.

## Before You Start

Please read these files before making changes:

* [README.md](README.md)
* [AGENTS.md](AGENTS.md)
* [docs/release-packaging.md](docs/release-packaging.md)
* [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

If your change affects bundled third-party tools, release packaging, or
licensing, also review:

* [src-tauri/resources/third-party/THIRD_PARTY_NOTICES.md](src-tauri/resources/third-party/THIRD_PARTY_NOTICES.md)

## Development Setup

### Requirements

* Node.js 20+
* npm 10+
* Rust stable
* Tauri v2 system prerequisites for your platform
* `git` installed locally

### Install dependencies

```bash
npm install
```

### Run the app

```bash
npm run tauri dev
```

## Validation Commands

Run these before opening a pull request:

### Frontend build and typecheck

```bash
npm run build
```

### Rust backend check

```bash
cargo check --manifest-path src-tauri/Cargo.toml --color always
```

### Desktop build

Use this when your change may affect packaging or desktop integration:

```bash
npm run tauri build
```

### Packaging script syntax checks

Use this when editing release helper scripts:

```bash
bash -n scripts/prepare-git-ftp.sh
bash -n scripts/prepare-lftp-unix.sh
bash -n scripts/prepare-lftp-windows.sh
```

## Project Structure

Important files and directories:

* Frontend app shell: `src/App.tsx`
* Frontend state: `src/store/appStore.ts`
* Tauri commands: `src-tauri/src/commands.rs`
* Deployment logic: `src-tauri/src/git_ftp.rs`
* Tool resolution and fallback behavior: `src-tauri/src/executables.rs`
* Release workflow: `.github/workflows/release.yml`

## Contribution Rules

### Keep `git` external

Do not change the app to bundle `git`. It is intentionally a user-installed
prerequisite.

### Respect the existing tool resolution flow

If you change how `git-ftp` or `lftp` are discovered or executed, prefer the
existing backend integration paths already defined in
`src-tauri/src/executables.rs`.

### Use Tauri dialogs

Do not use browser-native `window.confirm` dialogs for desktop confirmation
flows. Use Tauri dialog APIs instead.

### Do not store secrets in tracked files

FTP passwords and similar secrets must go through the backend keyring flow.
Do not persist them in repository files, sample configs, fixtures, or logs.

### Keep docs in sync

If you change release packaging behavior, update:

* [docs/release-packaging.md](docs/release-packaging.md)
* [src-tauri/resources/third-party/THIRD_PARTY_NOTICES.md](src-tauri/resources/third-party/THIRD_PARTY_NOTICES.md)

## Licensing and Redistribution

This project is licensed under GPL-3.0-only.

Release builds also bundle third-party tools and their notices. If your change
affects bundled assets, you must keep the licensing story accurate and complete.

Current release packaging expectations include:

* `git-ftp` license text
* `lftp` license text
* Windows `lftp` runtime DLL package metadata and available notices when those
  DLLs are bundled

Do not replace missing upstream license material with placeholder project
license text. Packaging should fail closed if required upstream notices cannot
be collected.

## Pull Requests

Please keep pull requests focused and easy to review.

A good pull request should include:

* a clear description of what changed
* why the change was needed
* any relevant screenshots for UI changes
* notes about packaging or license impact when applicable
* confirmation of the commands you ran to validate the change

If you changed UI behavior, mention the affected screen or workflow.

If you changed release packaging, mention the target platform and what bundled
assets or notices changed.

## Commit Style

Use GitHub-style conventional commits:

```text
<type>(<scope>): <subject>
```

Preferred types in this repository:

* `fix`
* `feat`
* `docs`
* `build`
* `ci`
* `ref`
* `chore`

Example:

```text
fix(ui): Keep successful deploy runs on changes tab
```

## Reporting Security Issues

Please do not open public issues for sensitive security problems, credential
exposure, or vulnerabilities involving deployment flows or bundled tooling.

Report security-sensitive issues privately through the repository owner's
GitHub profile:

https://github.com/dinisjcorreia

## Questions

If you are unsure whether a change belongs in this repository, open an issue
or draft pull request first and describe the use case before doing larger work.
