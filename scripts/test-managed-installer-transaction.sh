#!/bin/sh
# shellcheck shell=sh
set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
TR300_MANAGED_INSTALLER_TEST_ONLY=1
export TR300_MANAGED_INSTALLER_TEST_ONLY
# shellcheck source=scripts/managed-installers/tr300-installer.sh
. "$script_dir/managed-installers/tr300-installer.sh"

fixture=$(mktemp -d "${TMPDIR:-/tmp}/tr300-managed-shell-test.XXXXXXXX")
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
export HOME="$fixture/home"
export XDG_CONFIG_HOME="$fixture/config"
export CARGO_HOME="$fixture/managed new"
old_prefix="$fixture/managed old"
old_path=$PATH
receipt="$XDG_CONFIG_HOME/tr300/tr300-receipt.json"
mkdir -p "$old_prefix/bin" "$CARGO_HOME/bin" "$(dirname "$receipt")"
printf '%s\n' old-receipt-binary > "$old_prefix/bin/tr300"
printf '%s\n' old-receipt-report > "$old_prefix/bin/report"
printf '%s\n' old-raw-cargo-binary > "$CARGO_HOME/bin/tr300"
printf '%s\n' old-raw-cargo-report > "$CARGO_HOME/bin/report"
chmod 755 "$old_prefix/bin/tr300" "$old_prefix/bin/report" \
    "$CARGO_HOME/bin/tr300" "$CARGO_HOME/bin/report"
printf '%s\n' "{\"install_prefix\":\"$old_prefix\",\"provider\":{\"source\":\"cargo-dist\",\"version\":\"0.31.0\"},\"source\":{\"app_name\":\"tr300\"},\"version\":\"4.1.3\"}" > "$receipt"

tr300_temp="$fixture/backup"
mkdir "$tr300_temp"
if (tr300_save_managed_state); then
    printf '%s\n' 'foreign report destination was accepted' >&2
    exit 1
fi
grep -Fxq old-raw-cargo-report "$CARGO_HOME/bin/report"
rm "$CARGO_HOME/bin/report"
tr300_save_managed_state
[ -z "$tr300_prior_report" ] || { echo 'old receipt claimed foreign report' >&2; exit 1; }
printf '%s\n' "{\"install_prefix\":\"$old_prefix\",\"binaries\":[\"tr300\",\"report\"],\"provider\":{\"source\":\"cargo-dist\",\"version\":\"0.31.0\"},\"source\":{\"app_name\":\"tr300\"},\"version\":\"4.1.3\"}" > "$receipt"
tr300_save_managed_state
PATH="$CARGO_HOME/bin:/usr/bin:/bin"
tr300_assert_no_unknown_path_owners
mkdir "$fixture/portable"
printf '%s\n' unknown > "$fixture/portable/tr300"
chmod 755 "$fixture/portable/tr300"
PATH="$fixture/portable:/usr/bin:/bin"
if (tr300_assert_no_unknown_path_owners); then
    printf '%s\n' 'unknown PATH owner was accepted' >&2
    exit 1
fi
PATH=$old_path
tr300_transaction_started=1
printf '%s\n' candidate > "$tr300_intended_binary"
printf '%s\n' candidate-report > "$tr300_intended_report"
rm -f "$tr300_prior_binary"
rm -f "$tr300_prior_report"
printf '%s\n' candidate-receipt > "$tr300_receipt"
tr300_restore_managed_state
grep -Fxq old-receipt-binary "$old_prefix/bin/tr300"
grep -Fxq old-raw-cargo-binary "$CARGO_HOME/bin/tr300"
grep -Fxq old-receipt-report "$old_prefix/bin/report"
[ ! -e "$CARGO_HOME/bin/report" ]
grep -Fq '"version":"4.1.3"' "$receipt"

tr300_version=4.2.0
printf '%s\n' "{\"install_prefix\":\"$CARGO_HOME\",\"provider\":{\"source\":\"cargo-dist\",\"version\":\"0.31.0\"},\"source\":{\"app_name\":\"tr300\"},\"version\":\"4.2.0\"}" > "$receipt"
tr300_intended_prefix=$CARGO_HOME
tr300_verify_receipt
printf '%s\n' '{"provider":{"source":"other"},"source":{"app_name":"tr300"},"install_prefix":"/tmp"}' > "$fixture/invalid.json"
if tr300_receipt_is_exact_app "$fixture/invalid.json"; then
    printf '%s\n' 'invalid managed receipt was accepted' >&2
    exit 1
fi

tr300_transaction_started=0
tr300_committed=1
# Only the top-level, unambiguous receipt inventory owns a report sibling.
for invalid in \
    '{"binaries":["tr300"],"extra":{"binaries":["report"]}}' \
    '{"binaries":{"0":"report"}}' \
    '{"binaries":["report"],"binaries":["tr300"]}' \
    '{"binaries":["report"],"\u0062inaries":["tr300"]}'; do
    printf '%s\n' "$invalid" > "$fixture/receipt-inventory.json"
    if tr300_receipt_owns_report "$fixture/receipt-inventory.json"; then
        echo 'ambiguous receipt inventory accepted' >&2
        exit 1
    fi
done
printf '%s\n' '{"binaries":["tr300","report"],"other":{"binaries":["unrelated"]}}' > "$fixture/receipt-inventory.json"
tr300_receipt_owns_report "$fixture/receipt-inventory.json"
# Explicit Cargo registration permits conversion without a managed receipt.
# Neither file is run to infer ownership, and unrelated/malformed inventory
# or a different intended prefix must not authorize the report command.
(
    CARGO_HOME="$fixture/raw cargo"
    XDG_CONFIG_HOME="$fixture/raw config"
    export CARGO_HOME XDG_CONFIG_HOME
    tr300_intended_prefix=$CARGO_HOME
    mkdir -p "$CARGO_HOME/bin" "$fixture/raw backup"
    printf '%s\n' 'foreign file that must never execute' > "$CARGO_HOME/bin/report"
    printf '%s\n' 'old Cargo tr300' > "$CARGO_HOME/bin/tr300"
    for invalid in \
        '{"installs":{"foreign 1.0.0 (registry)":{"bins":["tr300","report"]}}}' \
        '{"installs":{"tr300 4.4.0 (registry)":{"bins":{"0":"tr300","1":"report"}}}}' \
        '{"installs":{"tr300 4.4.0 (registry)":{"bins":["tr300"],"other":{"bins":["report"]}}}}' \
        '{"installs":{"tr300 4.4.0 (registry)":{"bins":["tr300","report"]}},"\u0069nstalls":{}}' \
        '{"installs":{"tr300 4.4.0 (registry)":{"bins":["tr300","report"]}}' \
        '{"installs":{"tr300 4.4.0 (registry)":{"bins":["tr300","report"]}}} trailing'; do
        printf '%s\n' "$invalid" > "$CARGO_HOME/.crates2.json"
        if tr300_cargo_owns_report; then echo 'invalid Cargo inventory accepted' >&2; exit 1; fi
    done
    printf '%s\n' '{"installs":{"tr300 4.4.0 (registry+https://github.com/rust-lang/crates.io-index)":{"bins":["tr300","report"],"rustc":"rustc 1.95\nhost: test"}}}' > "$CARGO_HOME/.crates2.json"
    tr300_cargo_owns_report
    tr300_intended_prefix="$fixture/another prefix"
    if tr300_cargo_owns_report; then echo 'foreign Cargo prefix accepted' >&2; exit 1; fi
    tr300_receipt_existed=0
    tr300_prior_report=''
    tr300_temp="$fixture/raw backup"
    tr300_save_managed_state
    grep -Fxq 'foreign file that must never execute' "$CARGO_HOME/bin/report"
)
# Record the exact commands at sudo without touching any system files. An
# immutable old PKG owns only tr300, regardless of a neighboring report.
(
    sudo() { printf '%s\n' "$*" >> "$fixture/pkg-removals"; }
    tr300_pkg_report_present=0
    tr300_remove_owned_pkg_payloads
    [ "$(cat "$fixture/pkg-removals")" = 'rm -f /usr/local/bin/tr300' ]
    : > "$fixture/pkg-removals"
    tr300_pkg_report_present=1
    tr300_remove_owned_pkg_payloads
    grep -Fxq 'rm -f /usr/local/bin/tr300' "$fixture/pkg-removals"
    grep -Fxq 'rm -f /usr/local/bin/report' "$fixture/pkg-removals"
    [ "$(wc -l < "$fixture/pkg-removals" | tr -d ' ')" = 2 ]
)
# Both commands may perform maintenance, so installer verification must use
# read-only options. The spaced prefix also exercises quoted invocation.
mkdir -p "$fixture/read only/bin"
cat > "$fixture/read only/bin/tr300" <<'READONLY'
#!/bin/sh
case "$1" in
    --version) echo 'tr300 4.4.0' ;;
    --help) echo 'full canonical help including update/install/uninstall' ;;
    --fast) echo '{}' ;;
    *) echo 'unexpected mutating verification' >&2; exit 99 ;;
esac
READONLY
cp "$fixture/read only/bin/tr300" "$fixture/read only/bin/report"
chmod 755 "$fixture/read only/bin/tr300" "$fixture/read only/bin/report"
(
    tr300_version=4.4.0
    tr300_intended_binary="$fixture/read only/bin/tr300"
    tr300_intended_report="$fixture/read only/bin/report"
    tr300_verify_binaries >/dev/null
)
raw_dist_installer="$fixture/tr300-dist-installer.sh"
printf '%s\n' '# exact cargo-dist fixture bytes' > "$raw_dist_installer"
tr300_dist_installer_sha256=$(tr300_sha256 "$raw_dist_installer")
tr300_verify_dist_installer "$raw_dist_installer"

local_source_directory="$fixture/local source"
local_staging_directory="$fixture/local staging"
mkdir "$local_source_directory" "$local_staging_directory"
local_dist_installer="$local_source_directory/tr300-dist-installer.sh"
cp "$raw_dist_installer" "$local_dist_installer"
staged_dist_installer="$local_staging_directory/tr300-dist-installer.sh"
download_marker="$fixture/local-override-downloaded"
(
    TR300_DIST_INSTALLER_PATH=$local_dist_installer
    export TR300_DIST_INSTALLER_PATH
    tr300_download() {
        printf '%s\n' called > "$download_marker"
        return 1
    }
    tr300_stage_dist_installer "$staged_dist_installer"
    cmp "$local_dist_installer" "$staged_dist_installer"
    [ -f "$staged_dist_installer" ] && [ ! -L "$staged_dist_installer" ]
    tr300_verify_dist_installer "$staged_dist_installer"
)
[ ! -e "$download_marker" ]

for malformed_path in '' relative-installer.sh "$local_source_directory"; do
    malformed_log="$fixture/malformed-local-path.log"
    if (
        TR300_DIST_INSTALLER_PATH=$malformed_path
        export TR300_DIST_INSTALLER_PATH
        tr300_stage_dist_installer "$local_staging_directory/rejected-installer.sh"
    ) 2> "$malformed_log"; then
        printf '%s\n' 'malformed local cargo-dist installer path was accepted' >&2
        exit 1
    fi
    grep -Fq 'TR-300 managed install failed safely:' "$malformed_log"
done

local_symlink="$fixture/local-installer-link"
if ln -s "$local_dist_installer" "$local_symlink" && [ -L "$local_symlink" ]; then
    if (
        TR300_DIST_INSTALLER_PATH=$local_symlink
        export TR300_DIST_INSTALLER_PATH
        tr300_stage_dist_installer "$local_staging_directory/rejected-link.sh"
    ) 2> "$fixture/local-symlink.log"; then
        printf '%s\n' 'symbolic-link cargo-dist installer override was accepted' >&2
        exit 1
    fi
    grep -Fq 'must name a readable regular file, not a symbolic link' \
        "$fixture/local-symlink.log"
else
    rm -f "$local_symlink" "$local_staging_directory/rejected-link.sh"
fi

expected_dist_installer_sha256=$tr300_dist_installer_sha256
local_mismatch_execution_marker="$fixture/local-mismatch-executed"
if (
    TR300_DIST_INSTALLER_PATH=$local_dist_installer
    export TR300_DIST_INSTALLER_PATH
    tr300_dist_installer_sha256=0000000000000000000000000000000000000000000000000000000000000000
    local_mismatch_staged="$local_staging_directory/local-mismatch.sh"
    tr300_stage_dist_installer "$local_mismatch_staged"
    tr300_verify_dist_installer "$local_mismatch_staged"
    printf '%s\n' executed > "$local_mismatch_execution_marker"
); then
    printf '%s\n' 'mismatched local cargo-dist installer was accepted' >&2
    exit 1
fi
[ ! -e "$local_mismatch_execution_marker" ]

tr300_prepare_sha256sum "$raw_dist_installer"
shim_sha256=$(
    "$tr300_sha256sum_directory/sha256sum" -b "$raw_dist_installer" |
        /usr/bin/awk '{ print $1 }'
)
[ "$shim_sha256" = "$expected_dist_installer_sha256" ]
if [ "$(uname -s)" = Darwin ]; then
    # shellcheck disable=SC2016
    grep -Fq 'exec /usr/bin/shasum -a 256 "$1"' \
        "$tr300_sha256sum_directory/sha256sum"
fi
mismatch_execution_marker="$fixture/mismatch-executed"
if (
    tr300_dist_installer_sha256=0000000000000000000000000000000000000000000000000000000000000000
    tr300_verify_dist_installer "$raw_dist_installer"
    printf '%s\n' executed > "$mismatch_execution_marker"
); then
    printf '%s\n' 'mismatched cargo-dist installer was accepted' >&2
    exit 1
fi
[ ! -e "$mismatch_execution_marker" ]
tr300_temp=''

# A signal immediately after a PKG receipt is retired must exit instead of
# resuming toward the commit assignment. EXIT owns cleanup, so the forgotten-
# receipt recovery branch runs exactly once and the transaction stays
# uncommitted.
signal_fixture="$fixture/signal-fixture.sh"
signal_state="$fixture/signal-state"
mkdir "$signal_state"
cat > "$signal_fixture" <<'TR300_SIGNAL_FIXTURE'
#!/bin/sh
set -eu
TR300_MANAGED_INSTALLER_TEST_ONLY=1
export TR300_MANAGED_INSTALLER_TEST_ONLY
. "$1"
tr300_temp="$2/temp"
mkdir "$tr300_temp"
tr300_transaction_started=1
tr300_committed=0
tr300_pkg_receipt_forgotten=1
kill -TERM "$$"
printf '%s\n' resumed > "$2/resumed"
tr300_committed=1
printf '%s\n' committed > "$2/committed"
TR300_SIGNAL_FIXTURE
chmod 700 "$signal_fixture"
signal_log="$signal_state/stderr"
if /bin/sh "$signal_fixture" \
    "$script_dir/managed-installers/tr300-installer.sh" "$signal_state" \
    > "$signal_state/stdout" 2> "$signal_log"; then
    printf '%s\n' 'signal fixture unexpectedly succeeded' >&2
    exit 1
fi
[ ! -e "$signal_state/resumed" ]
[ ! -e "$signal_state/committed" ]
[ ! -e "$signal_state/temp" ]
[ "$(grep -Fxc \
    'TR-300 warning: the old PKG receipt was already retired; retaining the verified managed copy for recovery' \
    "$signal_log")" -eq 1 ]

printf '%s\n' 'managed shell transaction fixtures: PASS'
