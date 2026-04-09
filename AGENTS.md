# Agent Instructions

## Package Manager
- Use `npm`: `npm install`, `npm run tauri dev`, `npm run build`
- Backend commands run from the repo root with `--manifest-path src-tauri/Cargo.toml`

## File-Scoped Commands
- No dedicated per-file lint or test commands are configured

| Task | Command |
|------|---------|
| Frontend build/typecheck | `npm run build` |
| Backend check | `cargo check --manifest-path src-tauri/Cargo.toml --color always` |
| Tauri desktop build | `npm run tauri build` |

## Key Conventions
- Keep `git` as a user-installed prerequisite
- Prefer bundled `git-ftp` and `lftp` integration paths already defined in `src-tauri/src/executables.rs`
- Update release packaging docs when changing bundled tool behavior: `docs/release-packaging.md`
- Keep third-party notices in sync when bundling external tools: `src-tauri/resources/third-party/THIRD_PARTY_NOTICES.md`
- Use Tauri dialog APIs instead of browser `window.confirm` dialogs
- Do not store FTP passwords in tracked files; secrets go through the backend keyring flow

## Commit Conventions
- Use GitHub-style conventional commits: `<type>(<scope>): <subject>`
- Prefer these types here: `fix`, `feat`, `docs`, `build`, `ci`, `ref`, `chore`
- Keep the subject imperative and concise, for example: `fix(ui): Explain initial git-ftp push failures`
- When useful, add a short body describing what changed and why

## Project Structure
- Frontend app shell: `src/App.tsx`
- Frontend state: `src/store/appStore.ts`
- Tauri commands: `src-tauri/src/commands.rs`
- Deployment logic: `src-tauri/src/git_ftp.rs`
- Tool resolution and PATH handling: `src-tauri/src/executables.rs`
- Release workflow: `.github/workflows/release.yml`
