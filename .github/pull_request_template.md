## Summary

Describe the change in a few sentences.

## Why

Explain the problem this PR solves or the reason for the change.

## Changes

- 
- 
- 

## Validation

List the commands you ran and the result for each.

- [ ] `npm run build`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml --color always`
- [ ] `npm run tauri build` (if packaging or desktop integration changed)
- [ ] `bash -n scripts/prepare-git-ftp.sh` (if edited)
- [ ] `bash -n scripts/prepare-lftp-unix.sh` (if edited)
- [ ] `bash -n scripts/prepare-lftp-windows.sh` (if edited)

## UI Notes

- [ ] No UI changes
- [ ] UI changed and screenshots are included below

## Screenshots

Add screenshots or short recordings for user-facing UI changes.

## Packaging and Licensing Impact

- [ ] No packaging or licensing impact
- [ ] Bundled tool behavior changed
- [ ] Third-party notices or license files changed
- [ ] Release packaging docs were updated

If checked, describe what changed:

-

## Security Notes

- [ ] No security-sensitive changes
- [ ] Change touches credential storage, command execution, path resolution, or bundled tooling

If checked, describe any special review concerns:

-

## Checklist

- [ ] I read `CONTRIBUTING.md`
- [ ] I kept secrets out of tracked files and logs
- [ ] I updated docs if behavior or packaging changed
- [ ] I used the repository commit style
