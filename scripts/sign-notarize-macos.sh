#!/usr/bin/env bash
# Sign and notarize one cargo-dist macOS archive before it is uploaded.
#
# Required environment variables are GitHub Actions secrets/variables. Never
# print them, persist decoded credentials outside the ephemeral work directory,
# or add a fallback that hosts unsigned macOS artifacts.

set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: $0 <apple-target> <dist-manifest.json> <artifact-directory>" >&2
    exit 64
fi

target=$1
manifest=$2
artifact_dir=$3

case "$target" in
    aarch64-apple-darwin | x86_64-apple-darwin) ;;
    *)
        echo "refusing to sign unsupported target: $target" >&2
        exit 64
        ;;
esac

required_vars=(
    APPLE_CERTIFICATE_P12_BASE64
    APPLE_CERTIFICATE_PASSWORD
    APPLE_API_KEY_P8_BASE64
    APPLE_API_KEY_ID
    APPLE_API_ISSUER_ID
    APPLE_SIGNING_IDENTITY
    APPLE_TEAM_ID
)
for name in "${required_vars[@]}"; do
    if [[ -z ${!name:-} ]]; then
        echo "required Apple release credential is unavailable: $name" >&2
        exit 78
    fi
done

if [[ ! -f $manifest ]]; then
    echo "cargo-dist manifest not found: $manifest" >&2
    exit 66
fi

archive_name="tr300-${target}.tar.xz"
archive="${artifact_dir}/${archive_name}"
sidecar="${archive}.sha256"
archive_root="tr300-${target}"

if [[ ! -f $archive ]]; then
    echo "cargo-dist macOS archive not found: $archive" >&2
    exit 66
fi

runner_temp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
work_dir=$(mktemp -d "${runner_temp%/}/tr300-notary-${target}.XXXXXX")
credential_dir="${work_dir}/credentials"
keychain="${work_dir}/tr300-signing.keychain-db"
mkdir -m 700 "$credential_dir"
chmod 700 "$work_dir"

cleanup() {
    security delete-keychain "$keychain" >/dev/null 2>&1 || true
    rm -rf "$work_dir"
}
trap cleanup EXIT INT TERM

p12_path="${credential_dir}/developer-id.p12"
api_key_path="${credential_dir}/AuthKey_${APPLE_API_KEY_ID}.p8"
keychain_password=$(openssl rand -base64 32)

printf '%s' "$APPLE_CERTIFICATE_P12_BASE64" | /usr/bin/base64 -D > "$p12_path"
printf '%s' "$APPLE_API_KEY_P8_BASE64" | /usr/bin/base64 -D > "$api_key_path"
chmod 600 "$p12_path" "$api_key_path"

security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
security import "$p12_path" \
    -k "$keychain" \
    -P "$APPLE_CERTIFICATE_PASSWORD" \
    -T /usr/bin/codesign \
    -T /usr/bin/security
security set-key-partition-list \
    -S apple-tool:,apple: \
    -s \
    -k "$keychain_password" \
    "$keychain" >/dev/null

# Resolve the imported identity inside this ephemeral keychain and sign by its
# certificate fingerprint. Signing by display name can be ambiguous when the
# same Developer ID certificate also exists in the runner's login keychain.
identity_lines=$(security find-identity -v -p codesigning "$keychain" \
    | grep -E '^[[:space:]]*[0-9]+\) [[:xdigit:]]{40} "[^"]+"$' || true)
identity_count=$(printf '%s\n' "$identity_lines" | grep -c . || true)
if [[ $identity_count -ne 1 ]]; then
    echo "expected exactly one Developer ID identity in the ephemeral keychain; found ${identity_count}" >&2
    exit 1
fi
signing_fingerprint=$(printf '%s\n' "$identity_lines" | awk '{print $2}')
if [[ ! $signing_fingerprint =~ ^[[:xdigit:]]{40}$ ]]; then
    echo "could not resolve the imported Developer ID certificate fingerprint" >&2
    exit 1
fi
imported_identity=$(printf '%s\n' "$identity_lines" | sed -E 's/^[^"]*"([^"]+)".*$/\1/')
if [[ $APPLE_SIGNING_IDENTITY != "$signing_fingerprint" \
    && $APPLE_SIGNING_IDENTITY != "$imported_identity" ]]; then
    echo "configured Apple signing identity does not match the sole imported identity" >&2
    exit 1
fi

extract_dir="${work_dir}/archive"
mkdir "$extract_dir"
COPYFILE_DISABLE=1 tar -xJf "$archive" -C "$extract_dir"

binary="${extract_dir}/${archive_root}/tr300"
if [[ ! -f $binary || ! -x $binary ]]; then
    echo "expected executable is missing from ${archive_name}: ${archive_root}/tr300" >&2
    exit 65
fi

echo "Signing ${archive_name} with hardened runtime..."
codesign \
    --force \
    --identifier com.qubetx.tr300 \
    --options runtime \
    --timestamp \
    --keychain "$keychain" \
    --sign "$signing_fingerprint" \
    "$binary"
codesign --verify --strict --verbose=4 "$binary"

# Verify identity metadata as well as the cryptographic envelope. This catches
# a keychain/import mix-up where a valid but wrong Developer ID identity signs
# the release. Keep the expected identifier and team explicit release inputs.
signature_details=$(codesign -d --verbose=4 "$binary" 2>&1)
if ! printf '%s\n' "$signature_details" | grep -Fqx 'Identifier=com.qubetx.tr300'; then
    echo "signed binary identifier mismatch for ${target}" >&2
    exit 1
fi
if ! printf '%s\n' "$signature_details" | grep -Fqx "TeamIdentifier=${APPLE_TEAM_ID}"; then
    echo "signed binary Team ID mismatch for ${target}" >&2
    exit 1
fi
if ! printf '%s\n' "$signature_details" | grep -Fqx "Authority=${imported_identity}"; then
    echo "signed binary Developer ID authority mismatch for ${target}" >&2
    exit 1
fi
if ! printf '%s\n' "$signature_details" | grep -Eq '^CodeDirectory .*flags=.*\(runtime\)'; then
    echo "signed binary is missing the hardened-runtime flag for ${target}" >&2
    exit 1
fi
if ! printf '%s\n' "$signature_details" | grep -Eq '^Timestamp=.+'; then
    echo "signed binary is missing a secure timestamp for ${target}" >&2
    exit 1
fi
echo "Verified Developer ID identity, Team ID, hardened runtime, and timestamp for ${target}."

notary_zip="${work_dir}/${archive_root}-notary.zip"
/usr/bin/ditto -c -k --keepParent "$binary" "$notary_zip"

notary_result="${work_dir}/notary-result.json"
echo "Submitting ${target} to Apple Notary Service..."
xcrun notarytool submit "$notary_zip" \
    --key "$api_key_path" \
    --key-id "$APPLE_API_KEY_ID" \
    --issuer "$APPLE_API_ISSUER_ID" \
    --wait \
    --output-format json > "$notary_result"

submission_id=$(jq -r '.id // empty' "$notary_result")
notary_status=$(jq -r '.status // empty' "$notary_result")
if [[ $notary_status != Accepted ]]; then
    echo "Apple notarization failed for ${target}: ${notary_status:-unknown status}" >&2
    if [[ -n $submission_id ]]; then
        xcrun notarytool log "$submission_id" \
            --key "$api_key_path" \
            --key-id "$APPLE_API_KEY_ID" \
            --issuer "$APPLE_API_ISSUER_ID" || true
    fi
    exit 1
fi
echo "Apple notarization accepted for ${target} (submission ${submission_id})."

# A standalone CLI has no staplable .app/.pkg container. Apple's accepted
# submission binds the notarization record to this Developer ID signature.
# Preserve those exact signed bytes when rebuilding cargo-dist's archive.
replacement="${work_dir}/${archive_name}"
COPYFILE_DISABLE=1 tar -cJf "$replacement" -C "$extract_dir" "$archive_root"
mv "$replacement" "$archive"

archive_sha=$(shasum -a 256 "$archive" | awk '{print $1}')
printf '%s *%s\n' "$archive_sha" "$archive_name" > "$sidecar"

manifest_tmp="${work_dir}/dist-manifest.json"
jq \
    --arg archive "$archive_name" \
    --arg sha "$archive_sha" \
    'if .artifacts[$archive] == null then
         error("archive missing from cargo-dist manifest: " + $archive)
     else
         .artifacts[$archive].checksums.sha256 = $sha
     end' \
    "$manifest" > "$manifest_tmp"
mv "$manifest_tmp" "$manifest"

(
    cd "$artifact_dir"
    shasum -a 256 -c "${archive_name}.sha256"
)

echo "Signed, notarized, and rehashed ${archive_name}."
