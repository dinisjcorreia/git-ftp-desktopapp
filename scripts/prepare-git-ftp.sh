#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-src-tauri}"
GIT_FTP_VERSION="${2:-1.6.0}"

TOOLS_DIR="${ROOT_DIR}/resources/third-party/tools/git-ftp"
LICENSE_DIR="${ROOT_DIR}/resources/third-party/licenses/git-ftp"

mkdir -p "${TOOLS_DIR}" "${LICENSE_DIR}"

curl -fsSL "https://raw.githubusercontent.com/git-ftp/git-ftp/${GIT_FTP_VERSION}/git-ftp" \
  -o "${TOOLS_DIR}/git-ftp"
chmod +x "${TOOLS_DIR}/git-ftp"

curl -fsSL "https://raw.githubusercontent.com/git-ftp/git-ftp/${GIT_FTP_VERSION}/LICENSE" \
  -o "${LICENSE_DIR}/LICENSE"
