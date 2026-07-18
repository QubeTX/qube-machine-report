#!/usr/bin/env bash
# Build the universal, signed, notarized TR-300 PKG-in-DMG distribution.
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
work_dir=$(mktemp -d "${runner_temp%/}/tr300-dmg.XXXXXX")
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
    -A -t cert -f pkcs12
security import "$installer_p12" -k "$keychain" \
    -P "$APPLE_INSTALLER_CERTIFICATE_PASSWORD" -A -t cert -f pkcs12
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
lipo -verify_arch arm64 x86_64 "$universal"
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
        [[ -n $submission ]] && xcrun notarytool log "$submission" \
            --key "$api_key" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER_ID" || true
        echo "Apple notarization failed for $(basename "$artifact"): ${status:-unknown}" >&2
        exit 1
    fi
    echo "Apple notarization accepted for $(basename "$artifact") (${submission})."
}

binary_zip="${work_dir}/tr300-universal-notary.zip"
/usr/bin/ditto -c -k --keepParent "$universal" "$binary_zip"
notarize "$binary_zip"

payload="${work_dir}/payload"
install -d -m 755 "${payload}/usr/local/bin"
install -m 755 "$universal" "${payload}/usr/local/bin/tr300"
pkg="${work_dir}/tr300.pkg"
pkgbuild --root "$payload" \
    --identifier com.qubetx.tr300.pkg \
    --version "$version" \
    --install-location / \
    --sign "$APPLE_INSTALLER_SIGNING_IDENTITY" \
    --keychain "$keychain" \
    "$pkg"
pkgutil --check-signature "$pkg"
notarize "$pkg"
xcrun stapler staple "$pkg"
xcrun stapler validate "$pkg"
spctl --assess --type install --verbose=4 "$pkg"

dmg_root="${work_dir}/dmg-root"
mkdir "$dmg_root"
cp "$pkg" "${dmg_root}/tr300.pkg"
cat > "${dmg_root}/README.txt" <<'EOF'
TR-300 installer

Open tr300.pkg and follow Apple Installer. The signed package installs the
versionless `tr300` command system-wide at /usr/local/bin/tr300.

If installation is blocked or cancelled, download a fresh installer from:
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

echo "Built signed, notarized, stapled universal DMG: $dmg"
