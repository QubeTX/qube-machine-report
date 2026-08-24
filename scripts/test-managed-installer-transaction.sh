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
printf '%s\n' old-raw-cargo-binary > "$CARGO_HOME/bin/tr300"
chmod 755 "$old_prefix/bin/tr300" "$CARGO_HOME/bin/tr300"
printf '%s\n' "{\"install_prefix\":\"$old_prefix\",\"provider\":{\"source\":\"cargo-dist\",\"version\":\"0.31.0\"},\"source\":{\"app_name\":\"tr300\"},\"version\":\"4.1.3\"}" > "$receipt"

tr300_temp="$fixture/backup"
mkdir "$tr300_temp"
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
rm -f "$tr300_prior_binary"
printf '%s\n' candidate-receipt > "$tr300_receipt"
tr300_restore_managed_state
grep -Fxq old-receipt-binary "$old_prefix/bin/tr300"
grep -Fxq old-raw-cargo-binary "$CARGO_HOME/bin/tr300"
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
raw_dist_installer="$fixture/tr300-dist-installer.sh"
printf '%s\n' '# exact cargo-dist fixture bytes' > "$raw_dist_installer"
tr300_dist_installer_sha256=$(tr300_sha256 "$raw_dist_installer")
tr300_verify_dist_installer "$raw_dist_installer"
tr300_prepare_sha256sum "$raw_dist_installer"
shim_sha256=$(
    "$tr300_sha256sum_directory/sha256sum" -b "$raw_dist_installer" |
        /usr/bin/awk '{ print $1 }'
)
[ "$shim_sha256" = "$tr300_dist_installer_sha256" ]
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
