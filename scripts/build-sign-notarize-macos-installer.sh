#!/usr/bin/env bash
# Build the universal, signed, notarized TR-300 PKG distribution.
#
# The direct PKG is the primary fresh-install and current-updater artifact. A
# byte-identical copy remains inside a signed DMG solely so immutable v4.1.x
# updaters can cross the transition without changing installation channels.
# Runs only on an ephemeral native macOS GitHub runner.

set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: $0 <version> <arm64-archive> <x86_64-archive> <output-dir>" >&2
    exit 64
fi

version=${1#v}
arm_archive=$2
x86_archive=$3
output_dir=$4

required_vars=(
    APPLE_CERTIFICATE_P12_BASE64
    APPLE_CERTIFICATE_PASSWORD
    APPLE_INSTALLER_CERTIFICATE_P12_BASE64
    APPLE_INSTALLER_CERTIFICATE_PASSWORD
    APPLE_API_KEY_P8_BASE64
    APPLE_API_KEY_ID
    APPLE_API_ISSUER_ID
    APPLE_SIGNING_IDENTITY
    APPLE_INSTALLER_SIGNING_IDENTITY
    APPLE_TEAM_ID
)
for name in "${required_vars[@]}"; do
    if [[ -z ${!name:-} ]]; then
        echo "required Apple release credential is unavailable: $name" >&2
        exit 78
    fi
done

for archive in "$arm_archive" "$x86_archive"; do
    if [[ ! -f $archive ]]; then
        echo "required macOS archive is missing: $archive" >&2
        exit 66
    fi
done

runner_temp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
work_dir=$(mktemp -d "${runner_temp%/}/tr300-macos-installer.XXXXXX")
keychain="${work_dir}/tr300-release.keychain-db"
keychain_password=$(openssl rand -base64 32)
credential_dir="${work_dir}/credentials"
mkdir -m 700 "$credential_dir" "$output_dir"
chmod 700 "$work_dir"

original_user_keychains=()
while IFS= read -r line; do
    path=${line#*\"}
    path=${path%\"*}
    [[ -n $path ]] && original_user_keychains+=("$path")
done < <(security list-keychains -d user)

cleanup() {
    security list-keychains -d user -s "${original_user_keychains[@]}" >/dev/null 2>&1 || true
    security delete-keychain "$keychain" >/dev/null 2>&1 || true
    rm -rf "$work_dir"
}
trap cleanup EXIT INT TERM

app_p12="${credential_dir}/developer-id-application.p12"
installer_p12="${credential_dir}/developer-id-installer.p12"
api_key="${credential_dir}/AuthKey_${APPLE_API_KEY_ID}.p8"
printf '%s' "$APPLE_CERTIFICATE_P12_BASE64" | /usr/bin/base64 -D > "$app_p12"
printf '%s' "$APPLE_INSTALLER_CERTIFICATE_P12_BASE64" | /usr/bin/base64 -D > "$installer_p12"
printf '%s' "$APPLE_API_KEY_P8_BASE64" | /usr/bin/base64 -D > "$api_key"
chmod 600 "$app_p12" "$installer_p12" "$api_key"

security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
# Explicit PKCS#12 selection follows GitHub's hosted-runner import pattern.
# `-A` applies only to this disposable keychain; the partition list below
# enables non-interactive Apple tools and cleanup deletes the keychain.
security import "$app_p12" -k "$keychain" -P "$APPLE_CERTIFICATE_PASSWORD" \
    -A -f pkcs12
security import "$installer_p12" -k "$keychain" \
    -P "$APPLE_INSTALLER_CERTIFICATE_PASSWORD" -A -f pkcs12
security set-key-partition-list -S apple-tool:,apple: -s -k "$keychain_password" "$keychain" >/dev/null
security list-keychains -d user -s "$keychain" "${original_user_keychains[@]}"

application_identities=$(security find-identity -v -p codesigning "$keychain")
if ! grep -Fq "$APPLE_SIGNING_IDENTITY" <<< "$application_identities"; then
    echo "configured Developer ID Application identity was not found in the ephemeral keychain" >&2
    exit 1
fi
# Installer identities are package-signing certificates, not code-signing
# identities, so `security find-identity -p codesigning` will not list them.
# The repository variable stores the full Developer ID Installer common name;
# require that exact certificate in the isolated keychain before pkgbuild.
if ! security find-certificate -c "$APPLE_INSTALLER_SIGNING_IDENTITY" \
    "$keychain" >/dev/null; then
    echo "configured Developer ID Installer certificate was not found in the ephemeral keychain" >&2
    exit 1
fi

arm_dir="${work_dir}/arm64"
x86_dir="${work_dir}/x86_64"
mkdir "$arm_dir" "$x86_dir"
COPYFILE_DISABLE=1 tar -xJf "$arm_archive" -C "$arm_dir"
COPYFILE_DISABLE=1 tar -xJf "$x86_archive" -C "$x86_dir"
arm_binary=$(find "$arm_dir" -type f -name tr300 -perm -111 -print -quit)
x86_binary=$(find "$x86_dir" -type f -name tr300 -perm -111 -print -quit)
if [[ -z $arm_binary || -z $x86_binary ]]; then
    echo "could not locate both architecture-specific tr300 binaries" >&2
    exit 65
fi

universal="${work_dir}/tr300"
lipo -create "$arm_binary" "$x86_binary" -output "$universal"
chmod 755 "$universal"
# Xcode 16.4 requires the input file before -verify_arch. Keep this ordering
# in lockstep with every post-install validation call in the hosted workflow.
lipo "$universal" -verify_arch arm64 x86_64
codesign --force --identifier com.qubetx.tr300 --options runtime --timestamp \
    --keychain "$keychain" --sign "$APPLE_SIGNING_IDENTITY" "$universal"
codesign --verify --strict --verbose=4 "$universal"
details=$(codesign -d --verbose=4 "$universal" 2>&1)
grep -Fqx 'Identifier=com.qubetx.tr300' <<< "$details"
grep -Fqx "TeamIdentifier=${APPLE_TEAM_ID}" <<< "$details"
grep -Eq '^CodeDirectory .*flags=.*\(runtime\)' <<< "$details"
grep -Eq '^Timestamp=.+' <<< "$details"

notarize() {
    local artifact=$1
    local result
    result="${work_dir}/notary-$(basename "$artifact").json"
    xcrun notarytool submit "$artifact" \
        --key "$api_key" \
        --key-id "$APPLE_API_KEY_ID" \
        --issuer "$APPLE_API_ISSUER_ID" \
        --wait --output-format json > "$result"
    local status submission
    status=$(jq -r '.status // empty' "$result")
    submission=$(jq -r '.id // empty' "$result")
    if [[ $status != Accepted ]]; then
        if [[ -n $submission ]]; then
            xcrun notarytool log "$submission" \
                --key "$api_key" --key-id "$APPLE_API_KEY_ID" \
                --issuer "$APPLE_API_ISSUER_ID" || true
        fi
        echo "Apple notarization failed for $(basename "$artifact"): ${status:-unknown}" >&2
        exit 1
    fi
    echo "Apple notarization accepted for $(basename "$artifact") (${submission})."
}

verify_pkg_signature() {
    local artifact=$1
    local signature
    signature=$(pkgutil --check-signature "$artifact" 2>&1)
    printf '%s\n' "$signature"
    grep -Fq 'Status: signed by a developer certificate' <<< "$signature"
    grep -Fq "$APPLE_INSTALLER_SIGNING_IDENTITY" <<< "$signature"
    grep -Fq "(${APPLE_TEAM_ID})" <<< "$signature"
}

binary_zip="${work_dir}/tr300-universal-notary.zip"
/usr/bin/ditto -c -k --keepParent "$universal" "$binary_zip"
notarize "$binary_zip"

payload="${work_dir}/payload"
install -d -m 755 "${payload}/usr/local/bin"
install -m 755 "$universal" "${payload}/usr/local/bin/tr300"
pkg_scripts="${work_dir}/pkg-scripts"
mkdir -m 755 "$pkg_scripts"
cat > "${pkg_scripts}/preinstall" <<'PREINSTALL'
#!/bin/sh
# Apple Installer does not guarantee payload rollback after a postinstall
# failure. Reject managed-shell/Cargo ownership here, before Installer lays
# down /usr/local/bin/tr300, rather than mutating another channel afterward.
set -u
umask 077

reject_managed_state() {
    home=$1
    binary="${home}/.cargo/bin/tr300"
    if [ -f "$binary" ] && [ ! -L "$binary" ] && [ -x "$binary" ]; then
        echo "TR-300: an existing managed/Cargo installation at \"${home}\" was found and preserved; the direct PKG stopped before installing its payload. Rerun the managed installer to refresh this copy to a receipt-aware version, then run \"${binary}\" uninstall and choose Complete before retrying the PKG, or keep using the managed installer." >&2
    else
        echo "TR-300: managed receipt or link evidence at \"${home}\" was found and preserved, but no regular runnable managed binary is available; the direct PKG stopped before installing its payload. Rerun the managed installer to restore a verifiable managed owner, then run \"${binary}\" uninstall and choose Complete before retrying the PKG." >&2
    fi
    exit 1
}

fail_closed() {
    echo "TR-300: $1 The direct PKG stopped before installing its payload." >&2
    exit 1
}

fail_uninspectable_home() {
    echo "TR-300: $1 Make the declared home available and inspectable, or repair the local account's Directory Service home record; otherwise keep using the managed installer. The direct PKG stopped before installing its payload." >&2
    exit 1
}

# PackageKit passes the selected target volume as argument 3. This component
# package is for the running system's /usr/local only; scanning host accounts
# while writing another volume would violate the ownership check.
case "${3-}" in
    /) ;;
    *) fail_closed 'supports only the current system volume (/); select the startup disk and retry.' ;;
esac

# macOS normally allocates local login accounts from UID 501 upward; those
# records are mandatory and fail closed across Open Directory's complete
# unsigned 32-bit UniqueID width. Positive lower UIDs are system territory, but
# any such record (including a manually assigned UID 500 user) with one safely
# inspectable home is scanned defensively. UID 0 and negative sentinels
# (including nobody=-2) are excluded. Values outside UInt32 are malformed and
# block the package instead of becoming an ownership blind spot.
minimum_human_uid=501
maximum_human_uid=4294967295

preinstall_state=$(/usr/bin/mktemp -d "/private/tmp/tr300-pkg-preinstall.XXXXXXXX") ||
    fail_closed 'could not create private inspection state.'
case "$preinstall_state" in
    /private/tmp/tr300-pkg-preinstall.*) ;;
    *) fail_closed 'received an unsafe private inspection path.' ;;
esac
accounts_file="${preinstall_state}/accounts"
record_plist="${preinstall_state}/record.plist"
seen_homes="${preinstall_state}/seen-homes"
directory_listing="${preinstall_state}/directory-listing"
entry_matches="${preinstall_state}/entry-matches"

# Invoked indirectly by the exit trap below.
# shellcheck disable=SC2329
cleanup_preinstall_state() {
    /bin/rm -f "$accounts_file" "$record_plist" "$seen_homes" \
        "$directory_listing" "$entry_matches"
    /bin/rmdir "$preinstall_state" 2>/dev/null || true
}
trap cleanup_preinstall_state 0
trap 'exit 1' HUP INT TERM
: > "$seen_homes" || fail_closed 'could not initialize private inspection state.'

read_single_plist_string() {
    plist_path=$1
    plist_key=$2
    case "$plist_key" in
        dsAttrTypeStandard:UniqueID)
            plist_entry=':dsAttrTypeStandard\:UniqueID'
            ;;
        dsAttrTypeStandard:NFSHomeDirectory)
            plist_entry=':dsAttrTypeStandard\:NFSHomeDirectory'
            ;;
        *) return 1 ;;
    esac
    # Keep the PKG compatible with the project's pre-macOS-12 floor: newer
    # typed plutil extraction is unavailable there. plutil validates the native
    # plist, then the long-shipped PlistBuddy reads escaped-key array indices.
    /usr/bin/plutil -lint "$plist_path" >/dev/null 2>&1 || return 1
    plist_capture_sentinel='__TR300_PLIST_CAPTURE_END__'
    plist_newline='
'
    plist_framed=$(
        /usr/libexec/PlistBuddy -c "Print ${plist_entry}:0" \
            "$plist_path" 2>/dev/null || exit 1
        printf '%s' "$plist_capture_sentinel"
    ) || return 1
    case "$plist_framed" in
        *"$plist_capture_sentinel") ;;
        *) return 1 ;;
    esac
    plist_with_terminator=${plist_framed%"$plist_capture_sentinel"}
    case "$plist_with_terminator" in
        *"$plist_newline") ;;
        *) return 1 ;;
    esac
    # PlistBuddy writes one record terminator. Remove exactly that byte while
    # preserving any newline that belongs to the Directory Service value.
    single_plist_value=${plist_with_terminator%"$plist_newline"}
    if /usr/libexec/PlistBuddy -c "Print ${plist_entry}:1" \
        "$plist_path" >/dev/null 2>&1; then
        return 1
    fi
    return 0
}

list_standard_directory() {
    listed_directory=$1
    listed_home=$2
    listed_description=$3
    if [ -L "$listed_directory" ] || [ ! -d "$listed_directory" ]; then
        fail_uninspectable_home "could not safely inspect ${listed_description} for local home \"${listed_home}\"."
    fi
    # Enumerate only this exact directory level. A successful listing lets an
    # absent fixed child be distinguished from a broken, permission-denied, or
    # I/O-failed lookup without recursively traversing any user-owned tree.
    if ! LC_ALL=C /bin/ls -1A "$listed_directory" \
        > "$directory_listing" 2>/dev/null; then
        fail_uninspectable_home "could not list ${listed_description} for local home \"${listed_home}\"."
    fi
}

standard_entry_present() {
    entry_parent=$1
    entry_name=$2
    entry_home=$3
    entry_description=$4
    list_standard_directory "$entry_parent" "$entry_home" "$entry_description"
    # Default APFS/HFS+ volumes resolve ASCII case variants to the same path.
    # Match fixed component names case-insensitively even on case-sensitive
    # volumes so a variant can only over-block, never hide managed evidence.
    if LC_ALL=C /usr/bin/grep -Fix -e "$entry_name" "$directory_listing" \
        > "$entry_matches"; then
        entry_match_count=$(/usr/bin/wc -l < "$entry_matches" |
            /usr/bin/tr -d '[:space:]')
        if [ "$entry_match_count" -ne 1 ]; then
            fail_uninspectable_home "found multiple ASCII-case-insensitive entries matching \"${entry_name}\" in ${entry_description} for local home \"${entry_home}\"."
        fi
        return 0
    else
        entry_match_status=$?
        if [ "$entry_match_status" -ne 1 ]; then
            fail_uninspectable_home "could not compare ${entry_description} entries for local home \"${entry_home}\"."
        fi
        return 1
    fi
}

standard_leaf_present() {
    leaf_home=$1
    first_component=$2
    second_component=$3
    leaf_component=$4
    leaf_description=$5
    first_path="${leaf_home}/${first_component}"
    second_path="${first_path}/${second_component}"

    standard_entry_present "$leaf_home" "$first_component" "$leaf_home" \
        'home directory' || return 1
    if [ -L "$first_path" ] || [ ! -d "$first_path" ]; then
        fail_uninspectable_home "found an abnormal ${leaf_description} parent at \"${first_path}\"."
    fi
    standard_entry_present "$first_path" "$second_component" "$leaf_home" \
        "${leaf_description} parent" || return 1
    if [ -L "$second_path" ] || [ ! -d "$second_path" ]; then
        fail_uninspectable_home "found an abnormal ${leaf_description} parent at \"${second_path}\"."
    fi
    standard_entry_present "$second_path" "$leaf_component" "$leaf_home" \
        "${leaf_description} directory"
}

managed_state_present() {
    home=$1
    standard_leaf_present "$home" '.cargo' 'bin' 'tr300' \
        'managed binary' ||
        standard_leaf_present "$home" '.config' 'tr300' \
            'tr300-receipt.json' 'managed receipt'
}

inspect_home_once() {
    inspected_home=$1
    inspection_required=$2
    inspected_home_lines=$(printf '%s\n' "$inspected_home" | /usr/bin/wc -l \
        | /usr/bin/tr -d '[:space:]')
    case "$inspected_home" in
        /)
            [ "$inspection_required" -eq 0 ] && return 0
            fail_uninspectable_home 'an eligible local account declared the filesystem root as its home.'
            ;;
        /*) ;;
        *)
            [ "$inspection_required" -eq 0 ] && return 0
            fail_uninspectable_home 'an eligible local account did not declare an absolute home.'
            ;;
    esac
    if [ "$inspected_home_lines" -ne 1 ] || [ -L "$inspected_home" ] ||
        [ ! -d "$inspected_home" ]; then
        [ "$inspection_required" -eq 0 ] && return 0
        fail_uninspectable_home "could not inspect eligible local home \"${inspected_home}\"."
    fi
    if ! LC_ALL=C /bin/ls -1A "$inspected_home" \
        > "$directory_listing" 2>/dev/null; then
        [ "$inspection_required" -eq 0 ] && return 0
        fail_uninspectable_home "could not list eligible local home \"${inspected_home}\"."
    fi
    if /usr/bin/grep -Fqx -e "$inspected_home" "$seen_homes"; then
        return 0
    fi
    printf '%s\n' "$inspected_home" >> "$seen_homes" ||
        fail_closed 'could not record an inspected local home.'
    managed_state_present "$inspected_home" && reject_managed_state "$inspected_home"
}

# The globs below are meaningful only after the directory that expands them has
# itself been proven stable and enumerable. Otherwise an inaccessible /Users
# could leave literal unmatched patterns and silently hide unregistered state.
if [ -L /Users ] || [ ! -d /Users ]; then
    fail_closed 'could not safely inspect the local /Users directory.'
fi
if ! LC_ALL=C /bin/ls -1A /Users > "$directory_listing" 2>/dev/null; then
    fail_closed 'could not list the local /Users directory.'
fi

# The PKG owns a system-wide path. Scan every conventional home, including
# dot-prefixed and unregistered residue, then enumerate all local human
# Directory Service records so a non-console account with a custom home cannot
# be hidden by the package launch environment. These three safe globs cover
# ordinary entries plus dot entries while excluding . and ..; inspect_home_once
# deduplicates overlapping homes.
for home in /Users/* /Users/.[!.]* /Users/..?*; do
    if [ -d "$home" ] || [ -L "$home" ]; then
        inspect_home_once "$home" 1
    fi
done

if ! /usr/bin/dscl . -list /Users > "$accounts_file"; then
    fail_closed 'could not enumerate local Directory Service accounts.'
fi
[ -s "$accounts_file" ] || fail_closed 'found no local Directory Service accounts.'

while IFS= read -r account; do
    case "$account" in
        ''|.|..|*[!A-Za-z0-9._-]*)
            fail_closed 'could not safely address a local Directory Service account.'
            ;;
    esac
    if ! /usr/bin/dscl -plist . -read "/Users/${account}" UniqueID > "$record_plist"; then
        fail_closed "could not resolve the UID for local account \"${account}\"."
    fi
    if ! read_single_plist_string "$record_plist" \
        'dsAttrTypeStandard:UniqueID'; then
        fail_closed "local account \"${account}\" did not have exactly one UID."
    fi
    account_uid=$single_plist_value
    account_uid_sign=positive
    case "$account_uid" in
        -*)
            account_uid_sign=negative
            account_uid=${account_uid#-}
            ;;
        +*)
            fail_closed "local account \"${account}\" had a malformed UID."
            ;;
    esac
    case "$account_uid" in
        ''|*[!0-9]*)
            fail_closed "local account \"${account}\" had a malformed UID."
            ;;
    esac
    account_uid=$(printf '%s\n' "$account_uid" | /usr/bin/sed 's/^0*//;s/^$/0/')
    if [ "$account_uid_sign" = negative ] || [ "$account_uid" = 0 ]; then
        continue
    fi
    account_uid_digits=$(printf '%s' "$account_uid" | /usr/bin/wc -c \
        | /usr/bin/tr -d '[:space:]')
    if [ "$account_uid_digits" -gt 10 ]; then
        fail_closed "local account \"${account}\" had a UID outside the supported unsigned 32-bit range."
    fi
    if [ "$account_uid" -gt "$maximum_human_uid" ]; then
        fail_closed "local account \"${account}\" had a UID outside the supported unsigned 32-bit range."
    fi
    inspection_required=0
    if [ "$account_uid" -ge "$minimum_human_uid" ]; then
        inspection_required=1
    fi
    if ! /usr/bin/dscl -plist . -read "/Users/${account}" \
        NFSHomeDirectory > "$record_plist"; then
        [ "$inspection_required" -eq 0 ] && continue
        fail_uninspectable_home "could not resolve the home for eligible local account \"${account}\"."
    fi
    if ! read_single_plist_string "$record_plist" \
        'dsAttrTypeStandard:NFSHomeDirectory'; then
        [ "$inspection_required" -eq 0 ] && continue
        fail_uninspectable_home "eligible local account \"${account}\" did not have exactly one home."
    fi
    account_home=$single_plist_value
    inspect_home_once "$account_home" "$inspection_required"
done < "$accounts_file"

exit 0
PREINSTALL
chmod 755 "${pkg_scripts}/preinstall"
pkg="${work_dir}/tr300.pkg"
pkgbuild --root "$payload" \
    --scripts "$pkg_scripts" \
    --identifier com.qubetx.tr300.pkg \
    --version "$version" \
    --install-location / \
    --sign "$APPLE_INSTALLER_SIGNING_IDENTITY" \
    --keychain "$keychain" \
    "$pkg"
verify_pkg_signature "$pkg"
notarize "$pkg"
xcrun stapler staple "$pkg"
xcrun stapler validate "$pkg"
spctl --assess --type install --verbose=4 "$pkg"

direct_pkg="${output_dir}/tr300-universal-apple-darwin.pkg"
cp "$pkg" "$direct_pkg"
verify_pkg_signature "$direct_pkg"
xcrun stapler validate "$direct_pkg"
spctl --assess --type install --verbose=4 "$direct_pkg"

pkg_sha=$(shasum -a 256 "$direct_pkg" | awk '{print $1}')
printf '%s *%s\n' "$pkg_sha" "$(basename "$direct_pkg")" > "${direct_pkg}.sha256"
(
    cd "$output_dir"
    shasum -a 256 -c "$(basename "$direct_pkg").sha256"
)

dmg_root="${work_dir}/dmg-root"
mkdir "$dmg_root"
cp "$pkg" "${dmg_root}/tr300.pkg"
cat > "${dmg_root}/README.txt" <<'EOF'
TR-300 legacy updater compatibility package

Current users should download the direct signed package instead:
https://github.com/QubeTX/qube-machine-report/releases/latest/download/tr300-universal-apple-darwin.pkg

This disk image remains available so TR-300 v4.1.x can update safely. It
contains the exact same signed package, which installs the versionless `tr300`
command system-wide at /usr/local/bin/tr300.

If installation is blocked or cancelled, open the latest release:
https://github.com/QubeTX/qube-machine-report/releases/latest
EOF

dmg="${output_dir}/tr300-universal-apple-darwin.dmg"
hdiutil create -volname "TR-300" -srcfolder "$dmg_root" -format UDZO -ov "$dmg"
codesign --force --timestamp --keychain "$keychain" --sign "$APPLE_SIGNING_IDENTITY" "$dmg"
codesign --verify --deep --strict --verbose=4 "$dmg"
notarize "$dmg"
xcrun stapler staple "$dmg"
xcrun stapler validate "$dmg"
spctl --assess --type open --context context:primary-signature --verbose=4 "$dmg"

sha=$(shasum -a 256 "$dmg" | awk '{print $1}')
printf '%s *%s\n' "$sha" "$(basename "$dmg")" > "${dmg}.sha256"
(
    cd "$output_dir"
    shasum -a 256 -c "$(basename "$dmg").sha256"
)

cmp "$direct_pkg" "${dmg_root}/tr300.pkg"
echo "Built signed, notarized, stapled universal PKG: $direct_pkg"
echo "Built legacy v4.1.x compatibility DMG from the identical PKG: $dmg"
