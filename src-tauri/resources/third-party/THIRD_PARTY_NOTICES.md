# Third-Party Notices

This application keeps `git` as a user-installed prerequisite.

Release artifacts may bundle the following third-party components:

- `git-ftp`
  - Source: https://github.com/git-ftp/git-ftp
  - License: GPL-3.0-or-later
  - Packaging model: bundled script

- `lftp`
  - Source: https://lftp.yar.ru/
  - License: GPL-3.0-or-later
  - Packaging model: bundled executable
  - Windows builds also bundle the runtime DLL dependencies reported by `ldd` for `lftp.exe`.

The release preparation scripts replace the placeholder tool files in this directory with the
actual packaged assets and copy the upstream license texts into:

- `resources/third-party/licenses/git-ftp/`
- `resources/third-party/licenses/lftp/`
- `resources/third-party/licenses/lftp/windows-dependencies/` for MSYS2 package metadata and
  available license notices for bundled Windows runtime DLLs

For Windows ARM64 builds, the release workflow currently assumes x64 `lftp` binaries may run under
Windows-on-Arm x64 emulation unless a native ARM64 package is supplied to the workflow.
