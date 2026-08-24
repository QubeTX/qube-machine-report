#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
compiler=${CC:-cc}
if [ -x /usr/bin/true ]; then
    system_true=/usr/bin/true
elif [ -x /bin/true ]; then
    system_true=/bin/true
else
    echo 'rollback fixture requires an absolute system true binary' >&2
    exit 1
fi
work_dir=$(mktemp -d "${TMPDIR:-/tmp}/tr300-pkg-rollback-test.XXXXXXXX")
work_dir=$(cd -- "$work_dir" && pwd -P)
trap 'rm -rf "$work_dir"' EXIT INT TERM

helper="${work_dir}/tr300-pkg-rollback"
"$compiler" -std=c11 -DTR300_ROLLBACK_TESTING -Wall -Wextra -Werror -O2 \
    "${repo_root}/scripts/macos-pkg-rollback.c" -o "$helper"
if [ "$(uname -s)" = 'Darwin' ]; then
    production_helper="${work_dir}/tr300-pkg-rollback-universal"
    xcrun clang -std=c11 -Wall -Wextra -Werror -O2 \
        -mmacosx-version-min=11.0 -arch arm64 -arch x86_64 \
        "${repo_root}/scripts/macos-pkg-rollback.c" -o "$production_helper"
    lipo "$production_helper" -verify_arch arm64 x86_64
    privileged_test_helper="${work_dir}/tr300-pkg-rollback-privileged-test"
    xcrun clang -std=c11 -DTR300_ROLLBACK_TESTING \
        -DTR300_ROLLBACK_PRIVILEGED_TESTING -Wall -Wextra -Werror -O2 \
        "${repo_root}/scripts/macos-pkg-rollback.c" \
        -o "$privileged_test_helper"
fi

user_home="${work_dir}/user-home"
binary="${user_home}/.cargo/bin/tr300"
receipt="${user_home}/.config/tr300/tr300-receipt.json"
cleanup_fixture="${work_dir}/cleanup-fixture.sh"
cleanup_marker="${work_dir}/cleanup-marker"
binary_canary="${work_dir}/privileged-binary-canary"
receipt_canary="${work_dir}/privileged-receipt-canary"
preflight_state="${work_dir}/preflight-state"
state_canary="${work_dir}/preflight-state-canary"
switched_home="${work_dir}/switched-user-home"
switched_binary="${switched_home}/.cargo/bin/tr300"
switched_receipt="${switched_home}/.config/tr300/tr300-receipt.json"

mkdir -p "$(dirname -- "$binary")" "$(dirname -- "$receipt")"
mkdir -p "$(dirname -- "$switched_binary")" \
    "$(dirname -- "$switched_receipt")"
cp "$system_true" "$binary"
cp "$system_true" "$switched_binary"
printf '%s\n' '{"provider":{"source":"cargo-dist"},"source":{"app_name":"tr300"}}' \
    > "$receipt"
printf '%s\n' '{"provider":{"source":"cargo-dist"},"source":{"app_name":"tr300"}}' \
    > "$switched_receipt"
chmod 751 "$binary"
chmod 751 "$switched_binary"
chmod 640 "$receipt"
chmod 640 "$switched_receipt"
printf '%s\n' 'privileged binary target' > "$binary_canary"
printf '%s\n' 'privileged receipt target' > "$receipt_canary"
printf '%s\n' 'state collision target' > "$state_canary"

has_xattr=0
has_acl=0
has_flags=0
if [ "$(uname -s)" = 'Darwin' ]; then
    if xattr -w com.qubetx.tr300.rollback-test binary-metadata "$binary" &&
        xattr -w com.qubetx.tr300.rollback-test receipt-metadata "$receipt"; then
        has_xattr=1
    fi
    acl_user=$(id -un)
    if chmod +a "user:${acl_user} allow read" "$binary" 2>/dev/null &&
        chmod +a "user:${acl_user} allow read" "$receipt" 2>/dev/null; then
        has_acl=1
    else
        chmod -N "$binary" "$receipt" 2>/dev/null || true
    fi
    if chflags hidden "$binary" "$receipt" 2>/dev/null; then
        has_flags=1
    fi
    if [ "${TR300_REQUIRE_DARWIN_METADATA:-0}" = '1' ] &&
        { [ "$has_xattr" -ne 1 ] || [ "$has_acl" -ne 1 ] ||
            [ "$has_flags" -ne 1 ]; }; then
        echo "required Darwin ACL/xattr/flag fixture setup is unsupported" >&2
        exit 1
    fi
fi

digest_file() {
    case "$(uname -s)" in
        Darwin) shasum -a 256 "$1" | awk '{print $1}' ;;
        *) sha256sum "$1" | awk '{print $1}' ;;
    esac
}

acl_entries() {
    # `ls -le` is the native macOS ACL renderer; every fixture path is fixed.
    # shellcheck disable=SC2012
    ls -lde "$1" | sed -n '2,$p'
}

original_binary=$(digest_file "$binary")
original_receipt=$(digest_file "$receipt")
binary_canary_before=$(digest_file "$binary_canary")
receipt_canary_before=$(digest_file "$receipt_canary")
if [ "$has_xattr" -eq 1 ]; then
    binary_xattr=$(xattr -p com.qubetx.tr300.rollback-test "$binary")
    receipt_xattr=$(xattr -p com.qubetx.tr300.rollback-test "$receipt")
fi
if [ "$has_acl" -eq 1 ]; then
    binary_acl=$(acl_entries "$binary")
    receipt_acl=$(acl_entries "$receipt")
fi
if [ "$has_flags" -eq 1 ]; then
    binary_flags=$(stat -f '%Sf' "$binary")
    receipt_flags=$(stat -f '%Sf' "$receipt")
fi

cat > "$cleanup_fixture" <<'FIXTURE'
#!/bin/sh
set -eu
marker=${TR300_ROLLBACK_MARKER:?}
if [ "${1:-}" = 'test-hook' ]; then
    user_home=$2
    printf '%s\n' 'test-hook' >> "$marker"
    binary="${user_home}/.cargo/bin/tr300"
    receipt="${user_home}/.config/tr300/tr300-receipt.json"
    case "${TR300_ROLLBACK_FIXTURE_CASE:-}" in
        fail)
            exit 1
            ;;
        symlink-fail)
            ln -s "$TR300_BINARY_CANARY" "$binary"
            ln -s "$TR300_RECEIPT_CANARY" "$receipt"
            exit 1
            ;;
        directory-rebind)
            mv "${user_home}/.cargo/bin" "${user_home}/.cargo/bin.bound"
            mkdir "${user_home}/.cargo/bin"
            ln -s "$TR300_BINARY_CANARY" "$binary"
            mv "${user_home}/.config/tr300" \
                "${user_home}/.config/tr300.bound"
            mkdir "${user_home}/.config/tr300"
            ln -s "$TR300_RECEIPT_CANARY" "$receipt"
            exit 1
            ;;
        signal)
            kill -TERM "$PPID"
            exit 0
            ;;
        staged-mutate)
            for staged in \
                "${user_home}/.cargo/bin"/.tr300-pkg-rollback-* \
                "${user_home}/.config/tr300"/.tr300-pkg-rollback-*; do
                [ -f "$staged" ] || exit 65
                printf '%s\n' 'attacker-modified-staged-inode' > "$staged"
            done
            exit 1
            ;;
        staged-delete)
            for staged in \
                "${user_home}/.cargo/bin"/.tr300-pkg-rollback-* \
                "${user_home}/.config/tr300"/.tr300-pkg-rollback-*; do
                [ -f "$staged" ] || exit 65
                rm -f "$staged"
            done
            exit 1
            ;;
        success|partial-commit|commit-signal)
            exit 0
            ;;
        *)
            exit 64
            ;;
    esac
fi

user_home=''
dry_run=0
while [ "$#" -gt 0 ]; do
    [ "$1" != '--dry-run' ] || dry_run=1
    if [ "$1" = '--user-profile' ]; then
        user_home=$2
        shift 2
        continue
    fi
    shift
done
[ -n "$user_home" ]
if [ "$dry_run" -eq 1 ]; then
    printf '%s\n' 'strict-dry-run' >> "$marker"
    if [ "${TR300_ROLLBACK_FIXTURE_CASE:-}" = 'dry-run-inplace' ]; then
        printf '%s\n' '{"modified_during_dry_run":true}' \
            > "${user_home}/.config/tr300/tr300-receipt.json"
    fi
    exit 0
fi
printf '%s\n' 'MUTATING-CLEANUP-INVOKED' >> "$marker"
exit 70
FIXTURE
chmod 755 "$cleanup_fixture"

file_mode() {
    stat -c '%a' "$1" 2>/dev/null || stat -f '%Lp' "$1"
}

file_uid() {
    stat -c '%u' "$1" 2>/dev/null || stat -f '%u' "$1"
}

file_gid() {
    stat -c '%g' "$1" 2>/dev/null || stat -f '%g' "$1"
}

file_nlink() {
    stat -c '%h' "$1" 2>/dev/null || stat -f '%l' "$1"
}

original_binary_uid=$(file_uid "$binary")
original_binary_gid=$(file_gid "$binary")
original_receipt_uid=$(file_uid "$receipt")
original_receipt_gid=$(file_gid "$receipt")

expect_transaction_failure() {
    local fixture_case=$1
    local transaction_state=${2:-}
    local helper_args=(run "$user_home" "$cleanup_fixture")
    if [ -n "$transaction_state" ]; then
        helper_args+=("$transaction_state")
    fi
    rm -f "$cleanup_marker"
    set +e
    TR300_ROLLBACK_FIXTURE_CASE=$fixture_case \
        TR300_BINARY_CANARY=$binary_canary \
        TR300_RECEIPT_CANARY=$receipt_canary \
        TR300_ROLLBACK_MARKER=$cleanup_marker \
        "$helper" "${helper_args[@]}"
    local status=$?
    set -e
    if [ "$status" -eq 0 ]; then
        echo "rollback fixture unexpectedly committed: $fixture_case" >&2
        exit 1
    fi
    [ "$(sed -n '1p' "$cleanup_marker")" = 'strict-dry-run' ]
    [ "$(sed -n '2p' "$cleanup_marker")" = 'test-hook' ]
    ! grep -Fq 'MUTATING-CLEANUP-INVOKED' "$cleanup_marker" || exit 1
}

assert_prior_state_restored() {
    [ -f "$binary" ]
    [ ! -L "$binary" ]
    [ -f "$receipt" ]
    [ ! -L "$receipt" ]
    [ "$(digest_file "$binary")" = "$original_binary" ]
    [ "$(digest_file "$receipt")" = "$original_receipt" ]
    [ "$(file_mode "$binary")" = 751 ]
    [ "$(file_mode "$receipt")" = 640 ]
    [ "$(file_uid "$binary")" = "$original_binary_uid" ]
    [ "$(file_gid "$binary")" = "$original_binary_gid" ]
    [ "$(file_uid "$receipt")" = "$original_receipt_uid" ]
    [ "$(file_gid "$receipt")" = "$original_receipt_gid" ]
    if [ "$has_xattr" -eq 1 ]; then
        [ "$(xattr -p com.qubetx.tr300.rollback-test "$binary")" = \
            "$binary_xattr" ]
        [ "$(xattr -p com.qubetx.tr300.rollback-test "$receipt")" = \
            "$receipt_xattr" ]
    fi
    if [ "$has_acl" -eq 1 ]; then
        [ "$(acl_entries "$binary")" = "$binary_acl" ]
        [ "$(acl_entries "$receipt")" = "$receipt_acl" ]
    fi
    if [ "$has_flags" -eq 1 ]; then
        [ "$(stat -f '%Sf' "$binary")" = "$binary_flags" ]
        [ "$(stat -f '%Sf' "$receipt")" = "$receipt_flags" ]
    fi
}

# Preinstall validation uses the same exact-copy and credential-drop launch
# boundary as postinstall, but performs no staging. A successful check invokes
# only the strict dry-run and leaves both names untouched.
rm -f "$cleanup_marker"
TR300_ROLLBACK_FIXTURE_CASE=success \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" check "$user_home" "$cleanup_fixture"
[ "$(sed -n '1p' "$cleanup_marker")" = 'strict-dry-run' ]
[ "$(sed -n '2p' "$cleanup_marker")" = '' ]
assert_prior_state_restored

# Preinstall persists the bound home UID/device/inode in an exclusive state
# file. A simulated fast-user switch to a different home with the same UID must
# fail before the postinstall dry-run or either managed-name mutation.
rm -f "$cleanup_marker" "$preflight_state"
TR300_ROLLBACK_FIXTURE_CASE=success \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" check "$user_home" "$cleanup_fixture" "$preflight_state"
[ "$(file_mode "$preflight_state")" = 600 ]
[ "$(file_uid "$preflight_state")" = "$(id -u)" ]
[ "$(file_nlink "$preflight_state")" = 1 ]
preflight_state_before=$(digest_file "$preflight_state")
rm -f "$cleanup_marker"
set +e
TR300_ROLLBACK_FIXTURE_CASE=success \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" check "$user_home" "$cleanup_fixture" "$preflight_state"
stale_state_status=$?
set -e
[ "$stale_state_status" -ne 0 ]
[ "$(digest_file "$preflight_state")" = "$preflight_state_before" ]
switched_binary_before=$(digest_file "$switched_binary")
switched_receipt_before=$(digest_file "$switched_receipt")
rm -f "$cleanup_marker"
set +e
TR300_ROLLBACK_FIXTURE_CASE=success \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" run "$switched_home" "$cleanup_fixture" "$preflight_state"
switch_status=$?
set -e
[ "$switch_status" -ne 0 ]
[ ! -e "$cleanup_marker" ]
[ "$(digest_file "$switched_binary")" = "$switched_binary_before" ]
[ "$(digest_file "$switched_receipt")" = "$switched_receipt_before" ]
assert_prior_state_restored

# A matching postinstall consumes the one-use token before staging. Even when
# cleanup then fails and rolls back, no stale/replayable transaction token is
# left in the package-scripts directory.
expect_transaction_failure fail "$preflight_state"
[ ! -e "$preflight_state" ]
assert_prior_state_restored

# The token name is exclusive and no-follow. A pre-existing symlink is a hard
# collision: preinstall fails closed and never overwrites its target.
state_canary_before=$(digest_file "$state_canary")
ln -s "$state_canary" "$preflight_state"
rm -f "$cleanup_marker"
set +e
TR300_ROLLBACK_FIXTURE_CASE=success \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" check "$user_home" "$cleanup_fixture" "$preflight_state"
state_collision_status=$?
set -e
[ "$state_collision_status" -ne 0 ]
[ -L "$preflight_state" ]
[ "$(digest_file "$state_canary")" = "$state_canary_before" ]
rm -f "$preflight_state"

# The probe launch is descriptor-bound too: a symlink is never accepted as the
# executable that validates user-owned migration state.
cleanup_symlink="${work_dir}/cleanup-symlink"
ln -s "$cleanup_fixture" "$cleanup_symlink"
rm -f "$cleanup_marker"
set +e
TR300_ROLLBACK_FIXTURE_CASE=success \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" check "$user_home" "$cleanup_symlink"
probe_symlink_status=$?
set -e
[ "$probe_symlink_status" -ne 0 ]
[ ! -e "$cleanup_marker" ]
rm -f "$cleanup_symlink"

# A successful dry-run is not authority to stage a file that changed in place
# while the child ran. The helper rejects it before mutation and does not
# overwrite the user's concurrent receipt edit with the earlier snapshot.
rm -f "$cleanup_marker"
set +e
TR300_ROLLBACK_FIXTURE_CASE=dry-run-inplace \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" run "$user_home" "$cleanup_fixture"
dry_run_change_status=$?
set -e
[ "$dry_run_change_status" -ne 0 ]
[ "$(sed -n '1p' "$cleanup_marker")" = 'strict-dry-run' ]
[ "$(sed -n '2p' "$cleanup_marker")" = '' ]
! grep -Fq 'MUTATING-CLEANUP-INVOKED' "$cleanup_marker" || exit 1
[ "$(digest_file "$binary")" = "$original_binary" ]
[ "$(cat "$receipt")" = '{"modified_during_dry_run":true}' ]
printf '%s\n' '{"provider":{"source":"cargo-dist"},"source":{"app_name":"tr300"}}' \
    > "$receipt"
chmod 640 "$receipt"
[ "$(digest_file "$receipt")" = "$original_receipt" ]

# Ordinary strict-cleanup failure must restore the exact prior pair.
expect_transaction_failure fail
assert_prior_state_restored

# Replacing both destinations with symlinks during cleanup must replace the
# symlink names inside the already-bound user directories, never follow them.
expect_transaction_failure symlink-fail
assert_prior_state_restored
[ "$(digest_file "$binary_canary")" = "$binary_canary_before" ]
[ "$(digest_file "$receipt_canary")" = "$receipt_canary_before" ]

# Replacing both managed directories cannot retarget descriptor-relative
# rollback into the replacement directories. The original directories receive
# the restored bytes, while canonical revalidation fails the transaction.
expect_transaction_failure directory-rebind
[ -L "$binary" ]
[ -L "$receipt" ]
[ "$(digest_file "$binary_canary")" = "$binary_canary_before" ]
[ "$(digest_file "$receipt_canary")" = "$receipt_canary_before" ]
[ "$(digest_file "${user_home}/.cargo/bin.bound/tr300")" = "$original_binary" ]
[ "$(digest_file "${user_home}/.config/tr300.bound/tr300-receipt.json")" = \
    "$original_receipt" ]
rm -f "$binary" "$receipt"
rmdir "${user_home}/.cargo/bin" "${user_home}/.config/tr300"
mv "${user_home}/.cargo/bin.bound" "${user_home}/.cargo/bin"
mv "${user_home}/.config/tr300.bound" "${user_home}/.config/tr300"
assert_prior_state_restored

# A signal before the irreversible commit boundary rolls the exact pair back.
expect_transaction_failure signal
assert_prior_state_restored

# A failure after the first managed identity has been unlinked exercises the
# true partial-commit branch: the first member is rebuilt from its anonymous
# backup and the second is restored from its still-staged original.
expect_transaction_failure partial-commit
assert_prior_state_restored

# A user-held writable descriptor can alter each staged inode. Rollback must
# ignore those bytes and rebuild the exact prior data plus macOS metadata from
# the anonymous descriptor-bound snapshots.
expect_transaction_failure staged-mutate
assert_prior_state_restored

# Deleting the staged originals cannot destroy rollback state: the anonymous
# snapshots retain data, ACLs, xattrs, and safe file flags without a pathname.
expect_transaction_failure staged-delete
assert_prior_state_restored

# Build and install a real component PKG with both preinstall and postinstall.
# Preinstall writes the one-use token beside the embedded helper; postinstall
# reads that exact shared script-directory token and consumes it before running
# the production transaction path. Only the deterministic post-stage failure
# hook is test-only. Installer must remove its just-laid payload and receipt and
# leave the exact managed pair restored. The marker owner proves all probe
# invocations ran after the helper dropped to the target user.
if [ "$(uname -s)" = 'Darwin' ]; then
    if ! sudo -n true; then
        echo 'native PKG rollback fixture requires non-interactive sudo' >&2
        exit 1
    fi
    native_payload="${work_dir}/native-payload"
    native_scripts="${work_dir}/native-scripts"
    native_install_root="${work_dir}/native-install-root"
    native_pkg="${work_dir}/native-rollback-fixture.pkg"
    native_identifier="com.qubetx.tr300.rollback-fixture.$(id -u).$$"
    native_state_evidence="${work_dir}/native-preflight-state-evidence"
    mkdir -p "$native_payload" "$native_scripts" "$native_install_root"
    printf '%s\n' 'payload must be rolled back' \
        > "${native_payload}/payload-marker"
    cp "$privileged_test_helper" \
        "${native_scripts}/tr300-pkg-rollback"
    cp "$cleanup_fixture" "${native_scripts}/cleanup-fixture.sh"
    chmod 755 "${native_scripts}/tr300-pkg-rollback" \
        "${native_scripts}/cleanup-fixture.sh"
    for fixed_path in "$user_home" "$cleanup_marker" "$binary_canary" \
        "$receipt_canary" "$native_state_evidence"; do
        case "$fixed_path" in
            *[!A-Za-z0-9_./-]*)
                echo "native PKG fixture path is not safely embeddable: $fixed_path" >&2
                exit 1
                ;;
        esac
    done
    cat > "${native_scripts}/preinstall" <<PREINSTALL
#!/bin/sh
set -u
script_dir=\$(/usr/bin/dirname -- "\$0") || exit 1
script_dir=\$(unset CDPATH; cd -- "\$script_dir" && pwd -P) || exit 1
export TR300_ROLLBACK_FIXTURE_CASE=success
export TR300_ROLLBACK_MARKER='$cleanup_marker'
export TR300_BINARY_CANARY='$binary_canary'
export TR300_RECEIPT_CANARY='$receipt_canary'
"\${script_dir}/tr300-pkg-rollback" check '$user_home' \
    "\${script_dir}/cleanup-fixture.sh" \
    "\${script_dir}/tr300-preflight-state"
PREINSTALL
    cat > "${native_scripts}/postinstall" <<POSTINSTALL
#!/bin/sh
set -u
script_dir=\$(/usr/bin/dirname -- "\$0") || exit 1
script_dir=\$(unset CDPATH; cd -- "\$script_dir" && pwd -P) || exit 1
preflight_state="\${script_dir}/tr300-preflight-state"
/usr/bin/stat -f '%u:%Lp:%l' "\$preflight_state" > '$native_state_evidence'
export TR300_ROLLBACK_FIXTURE_CASE=partial-commit
export TR300_ROLLBACK_MARKER='$cleanup_marker'
export TR300_BINARY_CANARY='$binary_canary'
export TR300_RECEIPT_CANARY='$receipt_canary'
"\${script_dir}/tr300-pkg-rollback" run '$user_home' \
    "\${script_dir}/cleanup-fixture.sh" "\$preflight_state"
rollback_status=\$?
[ ! -e "\$preflight_state" ] || exit 70
exit "\$rollback_status"
POSTINSTALL
    chmod 755 "${native_scripts}/preinstall" "${native_scripts}/postinstall"
    pkgbuild --root "$native_payload" --scripts "$native_scripts" \
        --identifier "$native_identifier" --version 1.0.0 \
        --install-location "$native_install_root" "$native_pkg"
    rm -f "$cleanup_marker"
    set +e
    sudo -n installer -pkg "$native_pkg" -target /
    native_status=$?
    set -e
    if [ "$native_status" -eq 0 ]; then
        echo 'native postinstall failure fixture unexpectedly committed' >&2
        exit 1
    fi
    [ "$(sed -n '1p' "$cleanup_marker")" = 'strict-dry-run' ]
    [ "$(sed -n '2p' "$cleanup_marker")" = 'strict-dry-run' ]
    [ "$(sed -n '3p' "$cleanup_marker")" = 'test-hook' ]
    [ "$(stat -f '%u' "$cleanup_marker")" = "$(id -u)" ]
    [ "$(cat "$native_state_evidence")" = '0:600:1' ]
    ! grep -Fq 'MUTATING-CLEANUP-INVOKED' "$cleanup_marker" || exit 1
    assert_prior_state_restored
    [ ! -e "${native_install_root}/payload-marker" ]
    if pkgutil --pkg-info "$native_identifier" >/dev/null 2>&1; then
        echo 'failed native fixture left an Installer receipt behind' >&2
        exit 1
    fi
fi

# A symlink at snapshot time is not a managed file identity and must fail
# before the cleanup fixture can run or mutate the canary.
rm -f "$binary"
ln -s "$binary_canary" "$binary"
rm -f "$cleanup_marker"
set +e
"$helper" check "$user_home"
check_status=$?
set -e
[ "$check_status" -ne 0 ]
[ ! -e "$cleanup_marker" ]
[ -L "$binary" ]
[ "$(digest_file "$binary_canary")" = "$binary_canary_before" ]
rm -f "$binary"
cp "$system_true" "$binary"
chmod 751 "$binary"

# A multiply-linked source is not an exclusive managed identity. Both hardlink
# names and their bytes must survive a check-mode rejection unchanged.
hardlink_source="${work_dir}/hardlink-source"
rm -f "$binary"
cp "$system_true" "$hardlink_source"
chmod 751 "$hardlink_source"
ln "$hardlink_source" "$binary"
hardlink_digest=$(digest_file "$hardlink_source")
rm -f "$cleanup_marker"
set +e
"$helper" check "$user_home"
hardlink_status=$?
set -e
[ "$hardlink_status" -ne 0 ]
[ ! -e "$cleanup_marker" ]
[ -f "$binary" ]
[ -f "$hardlink_source" ]
[ "$(digest_file "$binary")" = "$hardlink_digest" ]
[ "$(digest_file "$hardlink_source")" = "$hardlink_digest" ]
rm -f "$binary" "$hardlink_source"
cp "$system_true" "$binary"
chmod 751 "$binary"

# A successful strict cleanup must commit with neither managed name left.
TR300_ROLLBACK_FIXTURE_CASE=success \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" run "$user_home" "$cleanup_fixture"
[ "$(sed -n '1p' "$cleanup_marker")" = 'strict-dry-run' ]
[ "$(sed -n '2p' "$cleanup_marker")" = 'test-hook' ]
! grep -Fq 'MUTATING-CLEANUP-INVOKED' "$cleanup_marker" || exit 1
[ ! -e "$binary" ]
[ ! -L "$binary" ]
[ ! -e "$receipt" ]
[ ! -L "$receipt" ]

# Once the signal mask is blocked and termination handlers are deliberately
# ignored, a signal delivered after the first unlink must not strand a partial
# pair. The test-only hook raises SIGTERM at that exact boundary; commit remains
# atomic and removes both managed names.
cp "$system_true" "$binary"
printf '%s\n' \
    '{"provider":{"source":"cargo-dist"},"source":{"app_name":"tr300"}}' \
    > "$receipt"
chmod 751 "$binary"
chmod 640 "$receipt"
rm -f "$cleanup_marker"
TR300_ROLLBACK_FIXTURE_CASE=commit-signal \
    TR300_BINARY_CANARY=$binary_canary \
    TR300_RECEIPT_CANARY=$receipt_canary \
    TR300_ROLLBACK_MARKER=$cleanup_marker \
    "$helper" run "$user_home" "$cleanup_fixture"
[ "$(sed -n '1p' "$cleanup_marker")" = 'strict-dry-run' ]
[ "$(sed -n '2p' "$cleanup_marker")" = 'test-hook' ]
! grep -Fq 'MUTATING-CLEANUP-INVOKED' "$cleanup_marker" || exit 1
[ ! -e "$binary" ]
[ ! -L "$binary" ]
[ ! -e "$receipt" ]
[ ! -L "$receipt" ]

printf '%s\n' \
    "macOS PKG rollback descriptor fixture passed (xattr=${has_xattr}, acl=${has_acl}, flags=${has_flags})."
