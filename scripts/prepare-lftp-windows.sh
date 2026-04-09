#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-src-tauri}"
TARGET_TRIPLE="${2:?target triple required}"
MSYS_BIN_DIR="${3:?MSYS bin dir required}"
LFTP_VERSION="${4:-4.9.3}"

BIN_DIR="${ROOT_DIR}/binaries"
LICENSE_DIR="${ROOT_DIR}/resources/third-party/licenses/lftp"
REPO_LICENSE_FALLBACK="LICENSE"

mkdir -p "${BIN_DIR}" "${LICENSE_DIR}"

resolve_lftp_binary() {
  local candidate=""

  for candidate in \
    "${MSYS_BIN_DIR}/lftp.exe" \
    "${MSYS_BIN_DIR}/lftp" \
    "/usr/bin/lftp.exe" \
    "/usr/bin/lftp"
  do
    if [[ -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  if command -v lftp >/dev/null 2>&1; then
    candidate="$(command -v lftp)"
    if [[ -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
    if [[ -f "${candidate}.exe" ]]; then
      printf '%s\n' "${candidate}.exe"
      return 0
    fi
  fi

  return 1
}

LFTP_BINARY="$(resolve_lftp_binary)" || {
  echo "Could not locate lftp in '${MSYS_BIN_DIR}' or PATH." >&2
  exit 1
}
LFTP_BIN_PARENT="$(dirname "${LFTP_BINARY}")"

cp "${LFTP_BINARY}" "${BIN_DIR}/lftp-${TARGET_TRIPLE}.exe"

find "${LFTP_BIN_PARENT}" -maxdepth 1 -type f -name '*.dll' -exec cp {} "${BIN_DIR}/" \;

copy_local_license() {
  local candidate=""

  for candidate in \
    "${MSYS_BIN_DIR}/../share/licenses/lftp/COPYING" \
    "${MSYS_BIN_DIR}/../share/doc/lftp/COPYING" \
    "/usr/share/licenses/lftp/COPYING" \
    "/usr/share/doc/lftp/COPYING"
  do
    if [[ -f "${candidate}" ]]; then
      cp "${candidate}" "${LICENSE_DIR}/COPYING"
      return 0
    fi
  done

  return 1
}

download_upstream_license() {
  local tmp_dir archive_url archive_path
  tmp_dir="$(mktemp -d)"

  for archive_url in \
    "https://ftp.gnu.org/gnu/lftp/lftp-${LFTP_VERSION}.tar.xz" \
    "https://lftp.yar.ru/ftp/lftp-${LFTP_VERSION}.tar.xz"
  do
    archive_path="${tmp_dir}/lftp.tar.xz"
    if curl --retry 5 --retry-all-errors --connect-timeout 10 -fsSL "${archive_url}" -o "${archive_path}"; then
      tar -xJf "${archive_path}" -C "${tmp_dir}"
      cp "${tmp_dir}/lftp-${LFTP_VERSION}/COPYING" "${LICENSE_DIR}/COPYING"
      rm -rf "${tmp_dir}"
      return 0
    fi
  done

  rm -rf "${tmp_dir}"
  return 1
}

if ! copy_local_license && ! download_upstream_license; then
  cp "${REPO_LICENSE_FALLBACK}" "${LICENSE_DIR}/COPYING"
fi
