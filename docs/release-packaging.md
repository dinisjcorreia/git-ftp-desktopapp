# Release Packaging

## Tooling policy

- `git` remains a user-installed prerequisite on every platform.
- `git-ftp` is bundled as a script under `src-tauri/resources/third-party/tools/git-ftp/git-ftp`.
- `lftp` is bundled as a platform-specific executable under `src-tauri/binaries/`.

## Release matrix

- macOS
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
- Windows
  - `x86_64-pc-windows-msvc`
  - `aarch64-pc-windows-msvc`
- Ubuntu
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
- Debian
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
- Fedora
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`

## Release workflow

- Workflow file: `.github/workflows/release.yml`
- Trigger modes:
  - manual `workflow_dispatch`
  - Git tag push matching `v*`
- Expected artifact types:
  - macOS: `.dmg`
  - Windows: `.msi`, NSIS `.exe`
  - Ubuntu / Debian: `.deb`
  - Fedora: `.rpm`

## Release preparation

Run or reuse these scripts before building public artifacts:

- `scripts/prepare-git-ftp.sh`
- `scripts/prepare-lftp-unix.sh`
- `scripts/prepare-lftp-windows.sh`

They are responsible for:

- populating the bundled `git-ftp` script
- populating the bundled `lftp` binary for the target platform
- copying Linux-specific `liblftp` runtime libraries next to the sidecar binary and rewriting the
  sidecar rpath so Tauri can resolve them during packaging
- copying only the Windows runtime DLLs reported by `ldd` for `lftp.exe`, not every DLL from the
  MSYS2 bin directory
- copying upstream license texts into the packaged resources tree

## Licensing

Before publishing release artifacts, the release workflow must populate:

- `src-tauri/resources/third-party/licenses/git-ftp/LICENSE`
- `src-tauri/resources/third-party/licenses/lftp/COPYING`
- `src-tauri/resources/third-party/licenses/lftp/windows-dependencies/` when Windows DLL
  dependencies are bundled

The packaged app also includes `src-tauri/resources/third-party/THIRD_PARTY_NOTICES.md`.

Current documented bundled third-party components:

- `git-ftp`
- `lftp`
- MSYS2 packages that own copied Windows `lftp` runtime DLLs

## Signing inputs

### macOS

Set these GitHub secrets to remove Gatekeeper warnings:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `KEYCHAIN_PASSWORD`

These secrets are used for:

- codesigning
- notarization
- reducing Gatekeeper warnings on distributed builds

If those secrets are missing, the workflow falls back to ad-hoc signing for the macOS app bundle so
the release still contains a signed binary inside the `.dmg`, but Gatekeeper may still require manual
approval because the app is not notarized.

### Windows

Set these GitHub secrets to reduce SmartScreen and sign installers:

- `WINDOWS_CERTIFICATE_PFX`
- `WINDOWS_CERTIFICATE_PASSWORD`

These are used to sign `.exe` and `.msi` artifacts in the workflow.

## Runtime expectations

- Release builds are intended to ship `git-ftp` and `lftp`
- `git` must still be installed on the target machine
- Desktop `PATH` repair and executable fallback resolution are implemented in the Rust backend to support GUI launches outside interactive shells

## Placeholders in the repository

The repository intentionally contains placeholder bundled files so local development builds remain valid before release-time assets are injected:

- `src-tauri/resources/third-party/tools/git-ftp/git-ftp`
- `src-tauri/binaries/lftp-aarch64-apple-darwin`
- `src-tauri/binaries/placeholder.dll`

## Windows ARM64 note

The workflow is prepared to build a Windows ARM64 app target. Until a native ARM64 `lftp` package is
supplied, the packaging flow assumes an x64 `lftp.exe` can run under Windows-on-Arm x64 emulation.

## Known workflow caveats

- The workflow scaffolding is in place, but the full matrix has not been executed and verified from this local session
- Linux ARM64 packaging relies on emulation in GitHub Actions
- Windows ARM64 packaging needs a native ARM64 `lftp` source for the cleanest final release story
