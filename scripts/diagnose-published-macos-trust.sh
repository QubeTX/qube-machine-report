#!/bin/bash
# Read-only investigation of the published v4.4.4 legacy DMG trust failure.
# No installer, stapling, signing, or packaged executable is invoked.
set -euo pipefail
umask 077
original_path=${PATH:-}
original_xcrun_command=$(command -v xcrun || true)
original_xcrun=$(type -P xcrun || true)
PATH=/usr/bin:/bin:/usr/sbin:/sbin
export PATH

if [[ $(/usr/bin/uname -s) != Darwin ]]; then
  printf '%s\n' 'This diagnostic requires a native macOS runner.' >&2
  exit 1
fi
if [[ $original_xcrun != /usr/bin/xcrun || ! -f $original_xcrun ]]; then
  printf 'Unexpected original xcrun selection: command-v=%s physical=%s\n' \
    "$original_xcrun_command" "$original_xcrun" >&2
  exit 1
fi

readonly release_tag=v4.4.4
readonly release_source=9f5110a3e8665f5fa2eb2e1744778c8025dc241f
readonly release_url="https://github.com/QubeTX/qube-machine-report/releases/download/$release_tag"
readonly pkg_name=tr300-universal-apple-darwin.pkg
readonly pkg_size=4378615
readonly pkg_sha=72ca19878649b1e60bb83b03297acaca662b19bdfd003f68a2ec0c1526e0ff92
readonly dmg_name=tr300-universal-apple-darwin.dmg
readonly dmg_size=5045219
readonly dmg_sha=9ceb5e145ec8e6adef8d5ce14562cd8d8ad0c5e350d32022e94734eb93f24742

# Match the immutable client's OS-selected per-user temporary parent. The
# separate 0755 control below deliberately has no private ancestor.
native_temp=$(/usr/bin/getconf DARWIN_USER_TEMP_DIR)
native_temp=${native_temp%/}
stage=$(/usr/bin/mktemp -d "$native_temp/tr300-published-trust.XXXXXX")
/bin/chmod 0700 "$stage"
readonly stage
canonical_stage=$(cd -P "$stage" && pwd -P)
summary="$stage/summary.md"
mount_path="$stage/mounted"
canonical_mount_path="$canonical_stage/mounted"
control=''
mount_attempted=0
failures=0
last_status=0
printf '| Check | Exit status | Required |\n| --- | ---: | --- |\n' > "$summary"

record_command() {
  local label=$1 required=$2 status log
  shift 2
  log="$stage/$label.log"
  printf '\n=== %s ===\nCommand:' "$label"
  printf ' %q' "$@"
  printf '\n'
  if "$@" > "$log" 2>&1; then
    status=0
  else
    status=$?
  fi
  /bin/cat "$log"
  printf '\n%s exit status: %s\n' "$label" "$status"
  printf '| %s | %s | %s |\n' "$label" "$status" "$required" >> "$summary"
  if [[ $required == yes && $status -ne 0 ]]; then
    failures=$((failures + 1))
  fi
  last_status=$status
}

# shellcheck disable=SC2329 # Invoked by the EXIT trap.
finish() {
  local status=$?
  trap - EXIT
  # An interrupted or unsuccessful attach can still leave a mounted volume.
  # Only this random private mount point is eligible for detachment.
  if [[ $mount_attempted -eq 1 ]]; then
    record_command cleanup-mount-state yes /sbin/mount
    if [[ $last_status -eq 0 ]]; then
      if /usr/bin/grep -Fq " on $mount_path (" "$stage/cleanup-mount-state.log" ||
          /usr/bin/grep -Fq " on $canonical_mount_path (" "$stage/cleanup-mount-state.log"; then
        record_command detach-readonly-volume yes /usr/bin/hdiutil detach "$mount_path"
      fi
    fi
  fi
  printf '\nPinned release: %s (%s)\nEvidence directory: %s\n' "$release_tag" "$release_source" "$stage"
  if [[ -n $control ]]; then printf 'Public-data 0755 control: %s\n' "$control"; fi
  /bin/cat "$summary"
  printf 'Required checks that failed: %s\n' "$failures"
  if [[ -n ${GITHUB_STEP_SUMMARY:-} ]]; then
    {
      printf '\n### Published macOS trust diagnostic: %s\n\n' "$release_tag"
      printf 'Pinned source: %s\n\n' "$release_source"
      /bin/cat "$summary"
      printf '\nEvidence directory: %s\n\nRequired failures: %s\n' "$stage" "$failures"
    } >> "$GITHUB_STEP_SUMMARY"
  fi
  if [[ $failures -ne 0 ]]; then status=1; fi
  exit "$status"
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# shellcheck disable=SC2329 # Invoked through record_command.
check_asset() {
  local path=$1 expected_size=$2 expected_sha=$3 actual_size actual_sha
  if [[ ! -f $path || -L $path ]]; then
    printf 'Expected a regular data file: %s\n' "$path" >&2
    return 1
  fi
  actual_size=$(/usr/bin/stat -f '%z' "$path")
  actual_sha=$(/usr/bin/shasum -a 256 "$path")
  actual_sha=${actual_sha%% *}
  printf 'File: %s\nSize: %s (expected %s)\nSHA256: %s (expected %s)\n' \
    "$path" "$actual_size" "$expected_size" "$actual_sha" "$expected_sha"
  [[ $actual_size == "$expected_size" && $actual_sha == "$expected_sha" ]]
}

# shellcheck disable=SC2329 # Invoked through record_command.
path_metadata() {
  local path=$1 parent status=0
  /bin/ls -ldeO@ "$path" || status=1
  /usr/bin/stat -f 'path=%N mode=%Sp octal=%Lp uid=%u gid=%g flags=%Sf size=%z inode=%i device=%d' "$path" || status=1
  /usr/bin/xattr -l "$path" || status=1
  parent=${path%/*}
  while [[ -n $parent ]]; do
    /usr/bin/stat -f 'ancestor=%N mode=%Sp octal=%Lp uid=%u gid=%g flags=%Sf' "$parent" || status=1
    [[ $parent == / ]] && break
    parent=${parent%/*}
    if [[ -z $parent ]]; then parent=/; fi
  done
  return "$status"
}

# shellcheck disable=SC2329 # Invoked through record_command.
check_alias_identity() {
  local raw=$1 canonical=$2 raw_id canonical_id
  raw_id=$(/usr/bin/stat -f '%d:%i' "$raw")
  canonical_id=$(/usr/bin/stat -f '%d:%i' "$canonical")
  printf 'Raw path: %s\nCanonical path: %s\nRaw device:inode: %s\nCanonical device:inode: %s\n' \
    "$raw" "$canonical" "$raw_id" "$canonical_id"
  [[ -n $raw_id && $raw_id == "$canonical_id" ]]
}

# shellcheck disable=SC2329 # Invoked through record_command.
legacy_quiet_command() {
  # Match the immutable Rust client's JSON child stdout and inherited PATH.
  # record_command retains stderr and the exit status, not discarded stdout.
  PATH=$original_path "$@" > /dev/null
}

# shellcheck disable=SC2329 # Invoked through record_command.
original_path_command() {
  PATH=$original_path "$@"
}

validate_package() {
  local label=$1 path=$2
  record_command "$label-metadata" yes path_metadata "$path"
  record_command "$label-pkgutil" yes /usr/sbin/pkgutil --check-signature "$path"
  record_command "$label-stapler" yes /usr/bin/xcrun stapler validate -v "$path"
  record_command "$label-spctl" yes /usr/sbin/spctl --assess --type install --verbose=4 "$path"
}

printf 'Read-only public-asset diagnostic for %s, source %s\n' "$release_tag" "$release_source"
printf 'Raw native temporary base: %s\nRaw private stage: %s\nCanonical private stage: %s\n' \
  "$native_temp" "$stage" "$canonical_stage"
printf 'Original PATH: %s\nOriginal command-v xcrun: %s\nOriginal physical xcrun: %s\n' \
  "$original_path" "$original_xcrun_command" "$original_xcrun"
printf 'DEVELOPER_DIR=%s\nTOOLCHAINS=%s\nSDKROOT=%s\n' \
  "${DEVELOPER_DIR:-<unset>}" "${TOOLCHAINS:-<unset>}" "${SDKROOT:-<unset>}"
record_command os-version yes /usr/bin/sw_vers
record_command native-architecture yes /usr/bin/uname -m
record_command xcode-selection yes /usr/bin/xcode-select -p
record_command native-temp-metadata yes path_metadata "$stage"
record_command private-stage-alias-identity yes check_alias_identity "$stage" "$canonical_stage"
record_command private-stage-mode yes /bin/test "$(/usr/bin/stat -f '%Lp' "$stage")" = 700

/bin/mkdir "$stage/download" "$mount_path" "$stage/writable"
pkg="$stage/download/$pkg_name"
dmg="$stage/download/$dmg_name"
record_command download-direct-pkg yes /usr/bin/curl --disable --fail --silent --show-error \
  --location --proto '=https' --proto-redir '=https' --connect-timeout 20 --max-time 120 \
  --output "$pkg" "$release_url/$pkg_name"
record_command download-dmg yes /usr/bin/curl --disable --fail --silent --show-error \
  --location --proto '=https' --proto-redir '=https' --connect-timeout 20 --max-time 120 \
  --output "$dmg" "$release_url/$dmg_name"
record_command verify-direct-pkg-before yes check_asset "$pkg" "$pkg_size" "$pkg_sha"
pkg_verified=$last_status
record_command verify-dmg-before yes check_asset "$dmg" "$dmg_size" "$dmg_sha"
dmg_verified=$last_status
if [[ $pkg_verified -ne 0 || $dmg_verified -ne 0 ]]; then
  printf '%s\n' 'Pinned input verification failed; no trust tool will consume unverified assets.' >&2
  exit 1
fi

# Exercise the legacy client's DMG and mounted-PKG sequence before any PKG
# control can prime Apple's trust or notarization caches.
record_command dmg-metadata yes path_metadata "$dmg"
record_command dmg-codesign yes legacy_quiet_command /usr/bin/codesign --verify --deep --strict "$dmg"
record_command dmg-stapler-legacy-quiet yes legacy_quiet_command "$original_xcrun" stapler validate "$dmg"
dmg_quiet_status=$last_status
record_command dmg-spctl yes legacy_quiet_command /usr/sbin/spctl --assess --type open --context context:primary-signature --verbose=4 "$dmg"
mount_attempted=1
inner_verified=0
record_command mount-readonly yes legacy_quiet_command /usr/bin/hdiutil attach -nobrowse -readonly -mountpoint "$mount_path" "$dmg"
if [[ $last_status -eq 0 ]]; then
  inner="$mount_path/tr300.pkg"
  canonical_inner="$canonical_mount_path/tr300.pkg"
  record_command verify-mounted-before yes check_asset "$inner" "$pkg_size" "$pkg_sha"
  if [[ $last_status -eq 0 ]]; then
    inner_verified=1
    record_command verify-canonical-mounted-before yes check_asset "$canonical_inner" "$pkg_size" "$pkg_sha"
    record_command mounted-alias-identity-before yes check_alias_identity "$inner" "$canonical_inner"
    record_command compare-direct-mounted yes /usr/bin/cmp "$pkg" "$inner"
    record_command mounted-private-metadata yes path_metadata "$inner"
    record_command mounted-private-pkgutil yes legacy_quiet_command /usr/sbin/pkgutil --check-signature "$inner"
    # Run quiet A/B before any verbose diagnostic can prime a ticket cache.
    # A's required failure stays latched even if B or a later retry succeeds.
    record_command mounted-raw-A-quiet yes legacy_quiet_command "$original_xcrun" stapler validate "$inner"
    raw_status=$last_status
    record_command mounted-canonical-B-quiet yes legacy_quiet_command "$original_xcrun" stapler validate "$canonical_inner"
    canonical_status=$last_status
    if [[ $raw_status -ne 0 && $canonical_status -eq 0 ]]; then
      record_command mounted-raw-A-informational-recheck no legacy_quiet_command "$original_xcrun" stapler validate "$inner"
    fi
    if [[ $raw_status -ne 0 ]]; then
      record_command mounted-raw-verbose-after-failure no original_path_command "$original_xcrun" stapler validate -v "$inner"
    fi
    record_command mounted-private-stapler yes /usr/bin/xcrun stapler validate -v "$canonical_inner"
    record_command mounted-private-spctl yes legacy_quiet_command /usr/sbin/spctl --assess --type install --verbose=4 "$inner"
    copy="$canonical_stage/writable/tr300.pkg"
    /bin/cp "$inner" "$copy"
    /bin/chmod 0600 "$copy"
    record_command verify-copy-before yes check_asset "$copy" "$pkg_size" "$pkg_sha"
    record_command compare-mounted-copy yes /usr/bin/cmp "$inner" "$copy"
    validate_package copy-private "$copy"
  fi
fi
record_command resolved-stapler yes /usr/bin/xcrun --find stapler
if [[ $dmg_quiet_status -ne 0 ]]; then
  record_command dmg-verbose-after-failure no original_path_command "$original_xcrun" stapler validate -v "$dmg"
fi
validate_package direct-private "$canonical_stage/download/$pkg_name"

if [[ $inner_verified -eq 1 ]]; then
  # Only fixed public package bytes are exposed here. Unlike the native-user
  # temporary tree, /private/tmp provides a traversable ancestor chain.
  control=$(/usr/bin/mktemp -d /private/tmp/tr300-public-trust-control.XXXXXX)
  /bin/chmod 0755 "$control"
  control_pkg="$control/tr300.pkg"
  /bin/cp "$inner" "$control_pkg"
  /bin/chmod 0644 "$control_pkg"
  record_command control-directory-mode yes /bin/test "$(/usr/bin/stat -f '%Lp' "$control")" = 755
  record_command verify-control-before yes check_asset "$control_pkg" "$pkg_size" "$pkg_sha"
  record_command compare-mounted-control yes /usr/bin/cmp "$inner" "$control_pkg"
  validate_package copy-traversable "$control_pkg"

  record_command verify-mounted-after yes check_asset "$inner" "$pkg_size" "$pkg_sha"
  record_command verify-canonical-mounted-after yes check_asset "$canonical_inner" "$pkg_size" "$pkg_sha"
  record_command mounted-alias-identity-after yes check_alias_identity "$inner" "$canonical_inner"
  record_command verify-copy-after yes check_asset "$copy" "$pkg_size" "$pkg_sha"
  record_command verify-control-after yes check_asset "$control_pkg" "$pkg_size" "$pkg_sha"
fi
record_command verify-direct-pkg-after yes check_asset "$pkg" "$pkg_size" "$pkg_sha"
record_command verify-dmg-after yes check_asset "$dmg" "$dmg_size" "$dmg_sha"
# The EXIT trap detaches this diagnostic's volume, prints all statuses, and
# preserves the first failure instead of retrying or rewriting its evidence.
exit 0
