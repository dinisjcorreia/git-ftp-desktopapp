#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-src-tauri}"
TARGET_TRIPLE="${2:?target triple required}"
LFTP_VERSION="${3:-4.9.3}"

BIN_DIR="${ROOT_DIR}/binaries"
LICENSE_DIR="${ROOT_DIR}/resources/third-party/licenses/lftp"

mkdir -p "${BIN_DIR}" "${LICENSE_DIR}"

LFTP_PATH="$(command -v lftp)"
cp "${LFTP_PATH}" "${BIN_DIR}/lftp-${TARGET_TRIPLE}"
chmod +x "${BIN_DIR}/lftp-${TARGET_TRIPLE}"

collect_linux_lftp_runtime_libraries() {
  local binary_path="${1}"
  local line=""
  local dependency_path=""
  local dependency_name=""

  command -v ldd >/dev/null 2>&1 || return 0

  ldd "${binary_path}" | while IFS= read -r line; do
    dependency_path=""

    if [[ "${line}" == *"=>"*"/"* ]]; then
      dependency_path="${line#*=> }"
      dependency_path="${dependency_path%% (*}"
    elif [[ "${line}" == /* ]]; then
      dependency_path="${line%% (*}"
    fi

    [[ -n "${dependency_path}" && -f "${dependency_path}" ]] || continue

    dependency_name="$(basename "${dependency_path}")"
    if [[ "${dependency_name}" == liblftp*.so* || "${dependency_path}" == *"/lftp/"* ]]; then
      printf '%s\n' "${dependency_path}"
    fi
  done | sort -u
}

vendor_linux_lftp_runtime_libraries() {
  local binary_path="${1}"
  local copied_paths=()
  local dependency_path=""
  local copied_path=""

  while IFS= read -r dependency_path; do
    [[ -n "${dependency_path}" ]] || continue
    copied_path="${BIN_DIR}/$(basename "${dependency_path}")"
    cp -L "${dependency_path}" "${copied_path}"
    copied_paths+=("${copied_path}")
  done < <(collect_linux_lftp_runtime_libraries "${binary_path}")

  if [[ "${#copied_paths[@]}" -eq 0 ]]; then
    return 0
  fi

  if ! command -v patchelf >/dev/null 2>&1; then
    echo "patchelf is required to relink bundled lftp runtime libraries on Linux." >&2
    exit 1
  fi

  patchelf --set-rpath '$ORIGIN' "${BIN_DIR}/lftp-${TARGET_TRIPLE}"
  for copied_path in "${copied_paths[@]}"; do
    patchelf --set-rpath '$ORIGIN' "${copied_path}" || true
  done
}

if [[ "${TARGET_TRIPLE}" == *"-unknown-linux-gnu" ]]; then
  vendor_linux_lftp_runtime_libraries "${LFTP_PATH}"
fi

copy_local_license() {
  local candidate=""
  for candidate in \
    "/usr/share/licenses/lftp/COPYING" \
    "/usr/share/doc/lftp/COPYING" \
    "/opt/homebrew/share/doc/lftp/COPYING" \
    "/usr/local/share/doc/lftp/COPYING"
  do
    if [[ -f "${candidate}" ]]; then
      cp "${candidate}" "${LICENSE_DIR}/COPYING"
      return 0
    fi
  done

  if command -v brew >/dev/null 2>&1; then
    candidate="$(brew --prefix lftp 2>/dev/null)/share/doc/lftp/COPYING"
    if [[ -f "${candidate}" ]]; then
      cp "${candidate}" "${LICENSE_DIR}/COPYING"
      return 0
    fi
  fi

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
  echo "Could not collect upstream lftp COPYING license text." >&2
  echo "Install lftp docs locally or allow downloading lftp-${LFTP_VERSION} source before packaging." >&2
  exit 1
fi

if [[ ! -s "${LICENSE_DIR}/COPYING" ]]; then
  echo "Collected lftp COPYING license is missing or empty." >&2
  exit 1
fi
