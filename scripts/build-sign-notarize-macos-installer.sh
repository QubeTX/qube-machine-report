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
install -m 755 "$universal" "${pkg_scripts}/tr300-migration-probe"
cat > "${pkg_scripts}/preinstall" <<'PREINSTALL'
#!/bin/sh
# Fail closed before Installer lays down the PKG payload when an existing
# managed-shell/Cargo owner cannot be proven safe to converge. The embedded
# probe is the exact signed universal binary that the package will install.
set -u

console_user=$(/usr/bin/stat -f '%Su' /dev/console 2>/dev/null || true)
case "$console_user" in
    ''|root|loginwindow|_mbsetupuser)
        if [ -n "${SUDO_USER:-}" ] && [ "${SUDO_USER}" != root ]; then
            console_user=$SUDO_USER
        else
            candidates=''
            for home in /Users/*; do
                if [ ! -e "${home}/.cargo/bin/tr300" ] && \
                    [ ! -e "${home}/.config/tr300/tr300-receipt.json" ]; then
                    continue
                fi
                candidates="${candidates}${home}\n"
            done
            candidate_count=$(printf '%b' "$candidates" | /usr/bin/grep -c '^/Users/' || true)
            [ "$candidate_count" -ne 0 ] || exit 0
            if [ "$candidate_count" -ne 1 ]; then
                echo "TR-300: multiple per-user CLI owners exist and no console user identifies the intended one; preserving them and rejecting PKG takeover before payload installation." >&2
                exit 1
            fi
            user_home=$(printf '%b' "$candidates" | /usr/bin/sed -n '1p')
            console_user=${user_home##*/}
        fi
        ;;
esac

if [ -z "${user_home:-}" ]; then
    user_home=$(/usr/bin/dscl . -read "/Users/${console_user}" NFSHomeDirectory 2>/dev/null \
        | /usr/bin/awk '{print $2}')
fi
if [ -z "$user_home" ] || [ ! -d "$user_home" ]; then
    echo "TR-300: could not resolve the active user's home; rejecting PKG takeover before payload installation." >&2
    exit 1
fi

script_dir=$(/usr/bin/dirname -- "$0") || exit 1
script_dir=$(unset CDPATH; cd -- "$script_dir" && pwd -P) || exit 1
"${script_dir}/tr300-migration-probe" migrate-cleanup --quiet --strict --dry-run \
    --cargo-copy --user-profile "$user_home"
PREINSTALL
chmod 755 "${pkg_scripts}/preinstall"
cat > "${pkg_scripts}/postinstall" <<'POSTINSTALL'
#!/bin/sh
# A deliberately launched PKG is the user's newest install-channel choice.
# Remove only the active console user's allowlisted Cargo/cargo-dist copy; the
# installed Rust helper validates the exact binary name and standard per-user
# cargo-dist receipt. A custom receipt root is not guessed or searched broadly.
set -u

console_user=$(/usr/bin/stat -f '%Su' /dev/console 2>/dev/null || true)
case "$console_user" in
    ''|root|loginwindow|_mbsetupuser)
        if [ -n "${SUDO_USER:-}" ] && [ "${SUDO_USER}" != root ]; then
            console_user=$SUDO_USER
        else
            # Headless native CI/MDM has no logged-in console user. Recover only
            # when exactly one normal home contains the allowlisted CLI copy or
            # its standard receipt. Zero means there is nothing to converge;
            # multiple user owners are ambiguous and must fail the package.
            candidates=''
            for home in /Users/*; do
                if [ ! -e "${home}/.cargo/bin/tr300" ] && \
                    [ ! -e "${home}/.config/tr300/tr300-receipt.json" ]; then
                    continue
                fi
                candidates="${candidates}${home}\n"
            done
            candidate_count=$(printf '%b' "$candidates" | /usr/bin/grep -c '^/Users/' || true)
            [ "$candidate_count" -ne 0 ] || exit 0
            if [ "$candidate_count" -ne 1 ]; then
                echo "TR-300: multiple per-user CLI owners exist and no console user identifies the intended one; preserving them and failing PKG takeover." >&2
                exit 1
            fi
            user_home=$(printf '%b' "$candidates" | /usr/bin/sed -n '1p')
            console_user=${user_home##*/}
        fi
        ;;
esac

if [ -z "${user_home:-}" ]; then
    user_home=$(/usr/bin/dscl . -read "/Users/${console_user}" NFSHomeDirectory 2>/dev/null \
        | /usr/bin/awk '{print $2}')
fi
if [ -z "$user_home" ] || [ ! -d "$user_home" ]; then
    echo "TR-300: could not resolve the active user's home; preserving any CLI install." >&2
    exit 0
fi

managed_binary="${user_home}/.cargo/bin/tr300"
managed_receipt="${user_home}/.config/tr300/tr300-receipt.json"
rollback_dir=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/tr300-pkg-takeover.XXXXXXXX") || exit 1
managed_binary_existed=0
managed_receipt_existed=0
takeover_committed=0
# Invoked indirectly by the EXIT/HUP/INT/TERM trap below.
# ShellCheck versions classify that indirection as SC2317 or SC2329.
# shellcheck disable=SC2317,SC2329
rollback_managed() {
    if [ "$takeover_committed" -eq 0 ]; then
        if [ "$managed_binary_existed" -eq 1 ]; then
            /bin/mkdir -p "$(/usr/bin/dirname "$managed_binary")"
            /bin/cp -p "$rollback_dir/tr300" "$managed_binary" || true
        fi
        if [ "$managed_receipt_existed" -eq 1 ]; then
            /bin/mkdir -p "$(/usr/bin/dirname "$managed_receipt")"
            /bin/cp -p "$rollback_dir/tr300-receipt.json" "$managed_receipt" || true
        fi
    fi
    /bin/rm -rf "$rollback_dir"
    takeover_committed=1
}
trap rollback_managed EXIT HUP INT TERM
if [ -f "$managed_binary" ]; then
    /bin/cp -p "$managed_binary" "$rollback_dir/tr300" || exit 1
    managed_binary_existed=1
fi
if [ -f "$managed_receipt" ]; then
    /bin/cp -p "$managed_receipt" "$rollback_dir/tr300-receipt.json" || exit 1
    managed_receipt_existed=1
fi

/usr/local/bin/tr300 migrate-cleanup --quiet --strict --cargo-copy --user-profile "$user_home"
if [ -e "$managed_binary" ]; then
    echo "TR-300: the prior Cargo-path copy could not be removed; PKG takeover is incomplete." >&2
    exit 1
fi
if [ -e "$managed_receipt" ]; then
    echo "TR-300: the matching managed-installer receipt could not be removed; PKG takeover is incomplete." >&2
    exit 1
fi
takeover_committed=1
exit 0
POSTINSTALL
chmod 755 "${pkg_scripts}/postinstall"
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
