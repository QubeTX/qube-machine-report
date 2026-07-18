#!/bin/sh
# TR-300 managed shell installer.
#
# The release workflow renders the immutable tag/version placeholders, keeps the
# cargo-dist generated installer as tr300-dist-installer.sh, and publishes this
# stable-name wrapper as tr300-installer.sh. On macOS a deliberately launched
# fresh wrapper safely supersedes an exact, receipt-owned TR-300 PKG install.

set -u

tr300_tag='@TR300_TAG@'
tr300_version='@TR300_VERSION@'
tr300_release_base="https://github.com/QubeTX/qube-machine-report/releases/download/${tr300_tag}"
tr300_recovery_url='https://github.com/QubeTX/qube-machine-report/releases/latest'
tr300_temp=''
tr300_transaction_started=0
tr300_committed=0
tr300_receipt_existed=0
tr300_intended_binary_existed=0
tr300_prior_binary_existed=0
tr300_prior_binary=''
tr300_pkg_present=0
tr300_pkg_payload_removed=0
tr300_pkg_receipt_forgotten=0

tr300_cleanup() {
    if [ "$tr300_transaction_started" -eq 1 ] && [ "$tr300_committed" -eq 0 ]; then
        if [ "$tr300_pkg_receipt_forgotten" -eq 1 ]; then
            # pkgutil has no supported inverse that recreates an exact signed
            # package receipt. The new managed binary/receipt were verified
            # before forgetting it, so retain that working owner rather than
            # deleting it and restoring an orphaned /usr/local payload.
            printf '%s\n' 'TR-300 warning: the old PKG receipt was already retired; retaining the verified managed copy for recovery' >&2
        else
            tr300_restore_managed_state ||
                printf '%s\n' 'TR-300 warning: restoring the prior managed/Cargo state also failed' >&2
            if [ "$tr300_pkg_payload_removed" -eq 1 ] && [ -f "$tr300_temp/prior-pkg-tr300" ]; then
                sudo /usr/bin/ditto "$tr300_temp/prior-pkg-tr300" /usr/local/bin/tr300 >/dev/null 2>&1 ||
                    printf '%s\n' 'TR-300 warning: restoring the prior PKG payload also failed' >&2
            fi
        fi
        tr300_committed=1
    fi
    if [ -n "$tr300_temp" ]; then
        rm -rf "$tr300_temp" >/dev/null 2>&1 || true
    fi
}
trap tr300_cleanup EXIT HUP INT TERM

tr300_fail() {
    printf '%s\n' "TR-300 managed install failed safely: $*" >&2
    printf '%s\n' "Download a fresh installer: ${tr300_recovery_url}" >&2
    exit 1
}

tr300_download() {
    url=$1
    output=$2
    if command -v curl >/dev/null 2>&1; then
        auth_token=${TR300_GITHUB_TOKEN:-${GITHUB_TOKEN:-${GH_TOKEN:-}}}
        if [ -n "$auth_token" ]; then
            curl --proto '=https' --tlsv1.2 -fLsS \
                -H "Authorization: Bearer ${auth_token}" "$url" -o "$output"
        else
            curl --proto '=https' --tlsv1.2 -fLsS "$url" -o "$output"
        fi
    elif command -v wget >/dev/null 2>&1; then
        auth_token=${TR300_GITHUB_TOKEN:-${GITHUB_TOKEN:-${GH_TOKEN:-}}}
        if [ -n "$auth_token" ]; then
            wget -q --header="Authorization: Bearer ${auth_token}" -O "$output" "$url"
        else
            wget -q -O "$output" "$url"
        fi
    else
        tr300_fail 'curl or wget is required'
    fi
}

tr300_install_prefix() {
    if [ -n "${TR300_INSTALL_DIR:-}" ]; then
        printf '%s\n' "$TR300_INSTALL_DIR"
    elif [ -n "${CARGO_DIST_FORCE_INSTALL_DIR:-}" ]; then
        printf '%s\n' "$CARGO_DIST_FORCE_INSTALL_DIR"
    elif [ -n "${CARGO_HOME:-}" ]; then
        printf '%s\n' "$CARGO_HOME"
    elif [ -n "${HOME:-}" ]; then
        printf '%s\n' "$HOME/.cargo"
    else
        tr300_fail 'HOME or CARGO_HOME is required to verify the managed install'
    fi
}

tr300_receipt_path() {
    if [ -n "${XDG_CONFIG_HOME:-}" ]; then
        printf '%s\n' "${XDG_CONFIG_HOME%/}/tr300/tr300-receipt.json"
    elif [ -n "${HOME:-}" ]; then
        printf '%s\n' "${HOME%/}/.config/tr300/tr300-receipt.json"
    else
        tr300_fail 'HOME or XDG_CONFIG_HOME is required to verify the managed receipt'
    fi
}

tr300_receipt_prefix() {
    receipt=$1
    prefix=$(sed -n 's/.*"install_prefix"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$receipt")
    [ -n "$prefix" ] || return 1
    [ "$(printf '%s\n' "$prefix" | wc -l | tr -d ' ')" = 1 ] || return 1
    case "$prefix" in
        /*) ;;
        *) return 1 ;;
    esac
    case "$prefix" in
        *\\*) return 1 ;;
    esac
    printf '%s\n' "$prefix"
}

tr300_receipt_version() {
    receipt=$1
    version=$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$receipt" | tail -n 1)
    [ -n "$version" ] || return 1
    printf '%s\n' "$version"
}

tr300_receipt_is_exact_app() {
    receipt=$1
    grep -Eq '"source"[[:space:]]*:[[:space:]]*"cargo-dist"' "$receipt" || return 1
    grep -Eq '"app_name"[[:space:]]*:[[:space:]]*"tr300"' "$receipt" || return 1
    tr300_receipt_prefix "$receipt" >/dev/null 2>&1 || return 1
}

tr300_save_managed_state() {
    tr300_intended_prefix=$(tr300_install_prefix) || tr300_fail 'could not resolve the managed install prefix'
    case "$tr300_intended_prefix" in
        /*) ;;
        *) tr300_fail 'the managed install prefix must be an absolute path' ;;
    esac
    tr300_intended_binary="${tr300_intended_prefix%/}/bin/tr300"
    tr300_receipt=$(tr300_receipt_path) || tr300_fail 'could not resolve the managed receipt path'

    if [ -f "$tr300_receipt" ]; then
        tr300_receipt_is_exact_app "$tr300_receipt" ||
            tr300_fail 'the existing TR-300 managed receipt is ambiguous; preserving it'
        tr300_receipt_existed=1
        cp -p "$tr300_receipt" "$tr300_temp/prior-receipt.json" ||
            tr300_fail 'could not back up the existing managed receipt'
        tr300_prior_prefix=$(tr300_receipt_prefix "$tr300_receipt") ||
            tr300_fail 'could not read the existing managed install prefix'
        tr300_prior_binary="${tr300_prior_prefix%/}/bin/tr300"
    fi

    if [ -f "$tr300_intended_binary" ]; then
        tr300_intended_binary_existed=1
        cp -p "$tr300_intended_binary" "$tr300_temp/prior-intended-tr300" ||
            tr300_fail 'could not back up the existing managed/Cargo binary'
    fi
    if [ -n "$tr300_prior_binary" ] && [ "$tr300_prior_binary" != "$tr300_intended_binary" ] &&
        [ -f "$tr300_prior_binary" ]; then
        tr300_prior_binary_existed=1
        cp -p "$tr300_prior_binary" "$tr300_temp/prior-receipt-tr300" ||
            tr300_fail 'could not back up the receipt-owned managed binary'
    fi
}

tr300_restore_one_binary() {
    path=$1
    existed=$2
    backup=$3
    if [ "$existed" -eq 1 ]; then
        mkdir -p "$(dirname "$path")" && cp -p "$backup" "$path"
    else
        rm -f "$path"
    fi
}

tr300_restore_managed_state() {
    tr300_restore_one_binary "$tr300_intended_binary" "$tr300_intended_binary_existed" \
        "$tr300_temp/prior-intended-tr300" || return 1
    if [ -n "$tr300_prior_binary" ] && [ "$tr300_prior_binary" != "$tr300_intended_binary" ]; then
        tr300_restore_one_binary "$tr300_prior_binary" "$tr300_prior_binary_existed" \
            "$tr300_temp/prior-receipt-tr300" || return 1
    fi
    if [ "$tr300_receipt_existed" -eq 1 ]; then
        mkdir -p "$(dirname "$tr300_receipt")" &&
            cp -p "$tr300_temp/prior-receipt.json" "$tr300_receipt"
    else
        rm -f "$tr300_receipt"
    fi
}

tr300_normalize_path() {
    candidate=$1
    directory=$(dirname "$candidate")
    leaf=$(basename "$candidate")
    [ -d "$directory" ] || return 1
    physical_dir=$(CDPATH='' cd -- "$directory" 2>/dev/null && pwd -P) || return 1
    printf '%s/%s\n' "${physical_dir%/}" "$leaf"
}

tr300_assert_no_unknown_path_owners() {
    intended=$(tr300_normalize_path "$tr300_intended_binary" 2>/dev/null || printf '%s\n' "$tr300_intended_binary")
    prior=''
    if [ -n "$tr300_prior_binary" ]; then
        prior=$(tr300_normalize_path "$tr300_prior_binary" 2>/dev/null || printf '%s\n' "$tr300_prior_binary")
    fi
    native=''
    if [ "$tr300_pkg_present" -eq 1 ]; then
        native=/usr/local/bin/tr300
    fi

    old_ifs=$IFS
    IFS=:
    for directory in ${PATH:-}; do
        [ -n "$directory" ] || directory=.
        candidate="${directory%/}/tr300"
        if [ ! -x "$candidate" ]; then
            # Git Bash cannot represent Unix executable bits in the isolated
            # fixture; production macOS/Linux calls still require `-x`.
            if [ "${TR300_MANAGED_INSTALLER_TEST_ONLY:-}" != 1 ] || [ ! -f "$candidate" ]; then
                continue
            fi
        fi
        full=$(tr300_normalize_path "$candidate") || {
            IFS=$old_ifs
            tr300_fail "could not resolve active PATH copy: $candidate"
        }
        if [ "$full" != "$intended" ] && [ "$full" != "$prior" ] && [ "$full" != "$native" ]; then
            IFS=$old_ifs
            tr300_fail "an unrecognized portable/PATH copy is active at $full; preserving it instead of guessing ownership"
        fi
    done
    IFS=$old_ifs
}

tr300_verify_receipt() {
    [ -f "$tr300_receipt" ] || tr300_fail "managed installer receipt is missing: $tr300_receipt"
    tr300_receipt_is_exact_app "$tr300_receipt" ||
        tr300_fail 'managed installer receipt does not identify TR-300 cargo-dist ownership'
    receipt_prefix=$(tr300_receipt_prefix "$tr300_receipt") ||
        tr300_fail 'managed installer receipt has no exact install prefix'
    [ "$receipt_prefix" = "$tr300_intended_prefix" ] ||
        tr300_fail 'managed installer receipt does not identify the requested install prefix'
    receipt_version=$(tr300_receipt_version "$tr300_receipt") ||
        tr300_fail 'managed installer receipt has no exact version'
    [ "$receipt_version" = "$tr300_version" ] ||
        tr300_fail "managed installer receipt does not identify ${tr300_version}"
}

tr300_verify_binary() {
    binary=$tr300_intended_binary
    [ -x "$binary" ] || tr300_fail "managed TR-300 binary is missing: $binary"
    reported=$($binary --version 2>/dev/null) || tr300_fail 'managed TR-300 binary did not run'
    [ "$reported" = "tr300 ${tr300_version}" ] \
        || tr300_fail "managed TR-300 binary did not report ${tr300_version}"
    printf '%s\n' "$binary"
}

tr300_pkg_is_exact_product() {
    package_info=$(pkgutil --pkg-info com.qubetx.tr300.pkg 2>/dev/null) || return 1
    payload_files=$(pkgutil --files com.qubetx.tr300.pkg 2>/dev/null) || return 1
    file_info=$(pkgutil --file-info /usr/local/bin/tr300 2>/dev/null) || return 1
    signature=$(codesign -d --verbose=4 /usr/local/bin/tr300 2>&1) || return 1
    codesign --verify --strict /usr/local/bin/tr300 >/dev/null 2>&1 || return 1

    package_version=$(printf '%s\n' "$package_info" | sed -n 's/^version:[[:space:]]*//p')
    file_version=$(printf '%s\n' "$file_info" | sed -n 's/^pkg-version:[[:space:]]*//p')
    package_location=$(printf '%s\n' "$package_info" | sed -n 's/^location:[[:space:]]*//p')
    reported=$(/usr/local/bin/tr300 --version 2>/dev/null) || return 1

    printf '%s\n' "$package_info" | grep -Eq '^package-id:[[:space:]]*com\.qubetx\.tr300\.pkg$' || return 1
    printf '%s\n' "$package_info" | grep -Eq '^volume:[[:space:]]*/$' || return 1
    [ -z "$package_location" ] || [ "$package_location" = / ] || return 1
    [ -n "$package_version" ] && [ "$package_version" = "$file_version" ] || return 1
    [ "$reported" = "tr300 ${package_version}" ] || return 1
    printf '%s\n' "$payload_files" | sed 's#^\./##; s#^/##' | grep -Fxq 'usr/local/bin/tr300' || return 1
    printf '%s\n' "$file_info" | grep -Eq '^(pkgid|package-id):[[:space:]]*com\.qubetx\.tr300\.pkg$' || return 1
    printf '%s\n' "$file_info" | grep -Eq '^path:[[:space:]]*/usr/local/bin/tr300$' || return 1
    printf '%s\n' "$file_info" | grep -Eq '^volume:[[:space:]]*/$' || return 1
    printf '%s\n' "$signature" | grep -Fxq 'Identifier=com.qubetx.tr300' || return 1
    printf '%s\n' "$signature" | grep -Fxq 'TeamIdentifier=M9D5379H93' || return 1
    printf '%s\n' "$signature" | grep -Eq '^Authority=Developer ID Application:' || return 1
}

tr300_prepare_macos_pkg() {
    [ "$(uname -s)" = Darwin ] || return 0
    command -v pkgutil >/dev/null 2>&1 || tr300_fail 'pkgutil is unavailable on macOS'
    if ! pkgutil --pkg-info com.qubetx.tr300.pkg >/dev/null 2>&1; then
        return 0
    fi
    [ -e /usr/local/bin/tr300 ] || tr300_fail 'the TR-300 PKG receipt exists but its payload is missing'
    tr300_pkg_is_exact_product || tr300_fail 'the PKG receipt/payload/signature evidence conflicts; preserving it'
    /usr/bin/ditto /usr/local/bin/tr300 "$tr300_temp/prior-pkg-tr300" ||
        tr300_fail 'could not back up the receipt-owned PKG payload'
    tr300_pkg_present=1
}

tr300_take_over_macos_pkg() {
    [ "$tr300_pkg_present" -eq 1 ] || return 0
    tr300_pkg_is_exact_product || tr300_fail 'the PKG receipt/payload/signature evidence conflicts; preserving it'

    printf '%s\n' 'Switching TR-300 ownership from macos-dmg-pkg to shell-installer...'
    sudo -v || tr300_fail 'administrator authorization was cancelled; the existing PKG was preserved'
    sudo rm -f /usr/local/bin/tr300 || tr300_fail 'could not remove the receipt-owned PKG payload'
    tr300_pkg_payload_removed=1
    sudo pkgutil --forget com.qubetx.tr300.pkg >/dev/null \
        || tr300_fail 'could not forget the TR-300 PKG receipt'
    tr300_pkg_receipt_forgotten=1
    if [ -e /usr/local/bin/tr300 ] || pkgutil --pkg-info com.qubetx.tr300.pkg >/dev/null 2>&1; then
        tr300_fail 'PKG takeover did not converge; the managed shell install remains available'
    fi
    tr300_pkg_present=0
}

tr300_main() {
    tr300_temp=$(mktemp -d "${TMPDIR:-/tmp}/tr300-managed-install.XXXXXXXX") \
        || tr300_fail 'could not create a private staging directory'
    tr300_prepare_macos_pkg
    tr300_save_managed_state
    tr300_assert_no_unknown_path_owners
    dist_installer="$tr300_temp/tr300-dist-installer.sh"
    tr300_download "${tr300_release_base}/tr300-dist-installer.sh" "$dist_installer" \
        || tr300_fail 'could not download the immutable managed installer'
    chmod 700 "$dist_installer" || tr300_fail 'could not protect the managed installer'

    tr300_transaction_started=1
    sh "$dist_installer" "$@" || tr300_fail 'cargo-dist installation did not complete'
    tr300_verify_receipt
    managed_binary=$(tr300_verify_binary) || tr300_fail 'managed TR-300 verification did not complete'
    tr300_take_over_macos_pkg
    if [ -n "$tr300_prior_binary" ] && [ "$tr300_prior_binary" != "$managed_binary" ]; then
        rm -f "$tr300_prior_binary" || tr300_fail 'could not remove the prior managed install path'
    fi
    tr300_verify_receipt
    tr300_verify_binary >/dev/null || tr300_fail 'final managed TR-300 verification did not complete'
    tr300_assert_no_unknown_path_owners
    tr300_committed=1
    printf '%s\n' "TR-300 ${tr300_version} is installed through the managed shell channel: ${managed_binary}"
}

if [ "${TR300_MANAGED_INSTALLER_TEST_ONLY:-}" != 1 ]; then
    tr300_main "$@"
fi
