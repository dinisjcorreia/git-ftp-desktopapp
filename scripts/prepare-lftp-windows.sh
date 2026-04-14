#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-src-tauri}"
TARGET_TRIPLE="${2:?target triple required}"
MSYS_BIN_DIR="${3:?MSYS bin dir required}"
LFTP_VERSION="${4:-4.9.3}"

BIN_DIR="${ROOT_DIR}/binaries"
LICENSE_DIR="${ROOT_DIR}/resources/third-party/licenses/lftp"
DEPENDENCY_LICENSE_DIR="${LICENSE_DIR}/windows-dependencies"

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

cp "${LFTP_BINARY}" "${BIN_DIR}/lftp-${TARGET_TRIPLE}.exe"

normalize_path() {
  local path="${1//$'\r'/}"

  if [[ "${path}" == *":"* && "${path}" == *"\\"* ]] && command -v cygpath >/dev/null 2>&1; then
    cygpath -u "${path}" 2>/dev/null || printf '%s\n' "${path}"
    return 0
  fi

  printf '%s\n' "${path}"
}

list_binary_dll_dependencies() {
  local binary_path="${1}"
  local line=""
  local dependency_path=""
  local dependency_name=""

  if ! command -v ldd >/dev/null 2>&1; then
    echo "ldd is required to identify lftp runtime DLL dependencies." >&2
    exit 1
  fi

  while IFS= read -r line; do
    dependency_path=""

    if [[ "${line}" == *"not found"* ]]; then
      echo "Missing DLL dependency for ${binary_path}: ${line}" >&2
      exit 1
    fi

    if [[ "${line}" == *"=>"* ]]; then
      dependency_path="${line#*=> }"
      dependency_path="${dependency_path%% (*}"
    elif [[ "${line}" == /* || "${line}" == [A-Za-z]:\\* ]]; then
      dependency_path="${line%% (*}"
    fi

    dependency_path="$(normalize_path "${dependency_path}")"
    [[ -n "${dependency_path}" && -f "${dependency_path}" ]] || continue

    dependency_name="$(basename "${dependency_path}")"
    [[ "${dependency_name,,}" == *.dll ]] || continue
    case "${dependency_path,,}" in
      /c/windows/*|/cygdrive/c/windows/*)
        continue
        ;;
    esac

    printf '%s\n' "${dependency_path}"
  done < <(ldd "${binary_path}")
}

copy_lftp_runtime_dlls() {
  local queue=("${LFTP_BINARY}")
  local index=0
  local binary_path=""
  local dependency_path=""

  declare -A seen_binaries=()
  declare -gA LFTP_RUNTIME_DLLS=()

  while [[ "${index}" -lt "${#queue[@]}" ]]; do
    binary_path="${queue[$index]}"
    index=$((index + 1))

    [[ -n "${seen_binaries[${binary_path}]:-}" ]] && continue
    seen_binaries["${binary_path}"]=1

    while IFS= read -r dependency_path; do
      [[ -n "${dependency_path}" ]] || continue
      if [[ -z "${LFTP_RUNTIME_DLLS[${dependency_path}]:-}" ]]; then
        LFTP_RUNTIME_DLLS["${dependency_path}"]=1
        queue+=("${dependency_path}")
      fi
    done < <(list_binary_dll_dependencies "${binary_path}")
  done

  for dependency_path in "${!LFTP_RUNTIME_DLLS[@]}"; do
    cp -L "${dependency_path}" "${BIN_DIR}/$(basename "${dependency_path}")"
  done
}

copy_windows_dependency_licenses() {
  local dependency_path=""
  local owner=""
  local owner_dir=""
  local source_dir=""
  local copied_any=0

  if ! command -v pacman >/dev/null 2>&1; then
    echo "pacman is required to collect MSYS2 package license notices for lftp DLLs." >&2
    exit 1
  fi

  rm -rf "${DEPENDENCY_LICENSE_DIR}"
  mkdir -p "${DEPENDENCY_LICENSE_DIR}"

  declare -A seen_packages=()

  for dependency_path in "${!LFTP_RUNTIME_DLLS[@]}"; do
    owner="$(pacman -Qo "${dependency_path}" 2>/dev/null | sed -E 's/.* is owned by ([^ ]+) .*/\1/' || true)"
    if [[ -z "${owner}" ]]; then
      echo "Could not identify MSYS2 package owner for ${dependency_path}." >&2
      exit 1
    fi

    [[ -n "${seen_packages[${owner}]:-}" ]] && continue
    seen_packages["${owner}"]=1
    owner_dir="${DEPENDENCY_LICENSE_DIR}/${owner}"
    mkdir -p "${owner_dir}"
    copied_any=0

    for source_dir in "/usr/share/licenses/${owner}" "/usr/share/doc/${owner}"; do
      if [[ -d "${source_dir}" ]]; then
        while IFS= read -r license_file; do
          cp "${license_file}" "${owner_dir}/$(basename "${license_file}")"
          copied_any=1
        done < <(
          find "${source_dir}" -maxdepth 2 -type f \
            \( -iname 'LICENSE*' -o -iname 'COPYING*' -o -iname 'NOTICE*' -o -iname 'COPYRIGHT*' \) \
            | sort
        )
      fi
    done

    pacman -Qi "${owner}" > "${owner_dir}/PACKAGE.txt"

    if [[ "${copied_any}" -eq 0 ]]; then
      echo "No explicit license file found for ${owner}; wrote package metadata to ${owner_dir}/PACKAGE.txt." >&2
    fi
  done
}

copy_lftp_runtime_dlls
copy_windows_dependency_licenses

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
  echo "Could not collect upstream lftp COPYING license text." >&2
  echo "Install lftp docs locally or allow downloading lftp-${LFTP_VERSION} source before packaging." >&2
  exit 1
fi

if [[ ! -s "${LICENSE_DIR}/COPYING" ]]; then
  echo "Collected lftp COPYING license is missing or empty." >&2
  exit 1
fi
