#!/usr/bin/env bash
# Use only on the disposable GitHub-hosted Linux/macOS runner. No shared sshd configuration is changed.
set -euo pipefail
[[ ${GITHUB_ACTIONS:-} == true && ${RUNNER_ENVIRONMENT:-} == github-hosted && $(id -u) != 0 ]]
fixture=$(cd "$(dirname "$0")" && pwd)/test-startup-unix.py
binary=$(realpath "$1")
mode=${2:-managed}
platform=$(uname -s)
[[ $mode == managed || $mode == machine ]]
if [[ ! -x /usr/sbin/sshd ]]; then
    sudo apt-get update -qq
    sudo apt-get install -y --no-install-recommends openssh-server
fi
tmp=$(mktemp -d /tmp/tr300-ssh.XXXXXXXX)
chmod 755 "$tmp"
account="tr300ci$(openssl rand -hex 4)"
created=false
cleanup() {
    if [[ -f $tmp/sshd.pid ]]; then sudo kill "$(cat "$tmp/sshd.pid")" || :; fi
    if [[ $created == true ]]; then
        if [[ $platform == Darwin ]]; then sudo dscl . -delete "/Users/$account"; else sudo userdel "$account"; fi
    fi
    sudo rm -rf -- "$tmp"
}
trap cleanup EXIT
if [[ $platform == Darwin ]]; then
    # Directory Service account with no prior login and no existing home.
    fixture_uid=$(dscl . -list /Users UniqueID | awk '$2 > highest && $2 < 60000 { highest=$2 } END { print highest+1 }')
    ! id "$account" >/dev/null 2>&1 || exit 1
    sudo dscl . -create "/Users/$account"
    created=true
    sudo dscl . -create "/Users/$account" UniqueID "$fixture_uid"
    sudo dscl . -create "/Users/$account" PrimaryGroupID 20
    sudo dscl . -create "/Users/$account" NFSHomeDirectory "$tmp/home"
    sudo dscl . -create "/Users/$account" UserShell /bin/bash
    sudo dscl . -create "/Users/$account" RealName "TR300 SSH Fixture"
    sudo dscl . -passwd "/Users/$account" "$(openssl rand -hex 32)"
else
    sudo useradd -M -d "$tmp/home" -s /bin/bash -p x "$account"
    created=true
fi
sudo install -d -m 0700 -o "$account" -g "$(id -gn "$account")" "$tmp/home"
install -m 0755 "$binary" "$tmp/tr300"
cp "$fixture" "$tmp/test.py"
if [[ $mode == managed ]]; then
sudo -u "$account" env -i HOME="$tmp/home" PATH=/usr/bin:/bin SHELL=/bin/bash \
    python3 "$tmp/test.py" setup "$tmp/tr300"
else
    # The login file signals readiness only; all startup behavior must come from the PKG.
    printf '%s\n' "printf 'TR300_READY\\n'" | sudo -u "$account" tee "$tmp/home/.bash_profile" >/dev/null
fi
ssh-keygen -q -t ed25519 -N '' -f "$tmp/client"
ssh-keygen -q -t ed25519 -N '' -f "$tmp/host"
if [[ $platform == Linux ]]; then sudo install -d -m 0755 /run/sshd; fi
port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
cat > "$tmp/server.conf" <<EOF
ListenAddress 127.0.0.1
Port $port
HostKey $tmp/host
PidFile $tmp/sshd.pid
AllowUsers $account
AuthorizedKeysFile .ssh/authorized_keys
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
UsePAM no
PrintMotd no
PrintLastLog no
EOF
printf '[127.0.0.1]:%s %s\n' "$port" "$(cat "$tmp/host.pub")" > "$tmp/known_hosts"
cat > "$tmp/client.conf" <<EOF
Host tr300-fixture
    HostName 127.0.0.1
    Port $port
    User $account
    IdentityFile $tmp/client
    IdentitiesOnly yes
    BatchMode yes
    UserKnownHostsFile $tmp/known_hosts
    StrictHostKeyChecking yes
    LogLevel ERROR
EOF
sudo install -d -m 0700 -o "$account" "$tmp/home/.ssh"
sudo install -m 0600 -o "$account" "$tmp/client.pub" "$tmp/home/.ssh/authorized_keys"
sudo /usr/sbin/sshd -f "$tmp/server.conf" -E "$tmp/server.log"
python3 "$tmp/test.py" ssh "$tmp/client.conf" || { sudo cat "$tmp/server.log"; exit 1; }
