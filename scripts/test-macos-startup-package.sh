#!/bin/bash
# Exercise the package lifecycle on a disposable native runner before tagging.
set -euo pipefail
[[ ${GITHUB_ACTIONS:-} == true && ${RUNNER_ENVIRONMENT:-} == github-hosted && $(uname -s) == Darwin ]]
tmp=$(mktemp -d "$RUNNER_TEMP/tr300-startup-pkg.XXXXXXXX")
runtime=/Library/PrivilegedHelperTools/com.qubetx.tr300.startup
agent=/Library/LaunchAgents/com.qubetx.tr300.startup.plist
builder=scripts/build-sign-notarize-macos-installer.sh
for path in "$runtime" "$agent" /usr/local/bin/tr300 /usr/local/bin/report; do
    [[ ! -e $path && ! -L $path ]] || exit 1
done
if pkgutil --pkg-info com.qubetx.tr300.pkg >/dev/null 2>&1; then exit 1; fi
mkdir -p "$tmp/scripts" "$tmp/root/usr/local/bin" "$tmp/root/Library/PrivilegedHelperTools" "$tmp/root/Library/LaunchAgents"
install -m 0755 target/release/tr300 "$tmp/root/usr/local/bin/tr300"
install -m 0755 target/release/report "$tmp/root/usr/local/bin/report"
install -m 0755 target/release/tr300 "$tmp/root$runtime"
# Reuse the actual release's literal templates; no signer is required.
sed -n "/<<'PREINSTALL'/,/^PREINSTALL$/p" "$builder" | sed '1d;$d' > "$tmp/scripts/preinstall"
python3 - "$builder" "$tmp/scripts/postinstall" "$tmp/root$agent" <<'PY'
import pathlib, shlex, sys
source, post, agent = map(pathlib.Path, sys.argv[1:])
def template(dest, output):
    lines = [s for s in source.read_text().splitlines() if s.startswith("printf ") and s.endswith(' > "' + dest + '"')]
    assert len(lines) == 1, dest
    tokens = shlex.split(lines[0].split(" > ")[0])
    assert tokens[:2] == ["printf", "%s\\n"]
    output.write_text("\n".join(tokens[2:]) + "\n")
template("$pkg_scripts/postinstall", post)
template("${payload}/Library/LaunchAgents/com.qubetx.tr300.startup.plist", agent)
PY
chmod 0755 "$tmp/scripts/"*
pkgbuild --root "$tmp/root" --scripts "$tmp/scripts" --identifier com.qubetx.tr300.pkg --version 4.5.0 --install-location / "$tmp/test.pkg"
sudo install -d -m 0755 /Library/PrivilegedHelperTools /Library/LaunchAgents
printf '%s\n' 'foreign data to preserve' > "$tmp/foreign"
for obstacle in "$runtime" "$agent"; do
    sudo install -m 0644 "$tmp/foreign" "$obstacle"
    if sudo installer -pkg "$tmp/test.pkg" -target /; then
        echo "PKG replaced a foreign file at $obstacle" >&2
        exit 1
    fi
    cmp "$tmp/foreign" "$obstacle"
    [[ ! -e /usr/local/bin/tr300 && ! -e /usr/local/bin/report ]]
    if pkgutil --pkg-info com.qubetx.tr300.pkg >/dev/null 2>&1; then exit 1; fi
    sudo rm "$obstacle"
done
sudo installer -pkg "$tmp/test.pkg" -target /
cmp "$runtime" target/release/tr300
cmp "$agent" "$tmp/root$agent"
grep -Fqx '# TR-300 system startup' /private/etc/zshrc
grep -Fqx '# TR-300 package Bash startup' "$HOME/.bashrc"
bash scripts/test-startup-unix-ssh.sh /usr/local/bin/tr300 machine
sudo "$runtime" configure-startup-machine
[[ $(grep -Fxc '# TR-300 system startup' /private/etc/zshrc) -eq 1 ]]
sudo "$runtime" remove-startup-machine
if grep -Fqx '# TR-300 system startup' /private/etc/zshrc || grep -Fqx '# TR-300 package Bash startup' "$HOME/.bashrc"; then exit 1; fi
sudo rm "$runtime" "$agent" /usr/local/bin/tr300 /usr/local/bin/report
sudo pkgutil --forget com.qubetx.tr300.pkg
