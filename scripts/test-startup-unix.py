#!/usr/bin/env python3
"""Exercise the installed shell hooks through actual Unix terminal devices."""
import argparse
import errno
import fcntl
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import statistics
import struct
import subprocess
import tempfile
import termios
import time
import unicodedata


def terminal(argv, env, size=(80, 25), exchanges=()):
    """Send input only after an interactive shell has reached its profile marker."""
    pid, master = pty.fork()
    if pid == 0:
        try:
            cols, rows = size
            fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            attrs = termios.tcgetattr(0)
            attrs[3] &= ~termios.ECHO
            termios.tcsetattr(0, termios.TCSANOW, attrs)
            os.execvpe(str(argv[0]), list(map(str, argv)), env)
        finally:
            os._exit(127)
    data = bytearray()
    offset = 0
    pending = list(exchanges)
    deadline = time.monotonic() + 45
    try:
        while True:
            timeout = deadline - time.monotonic()
            if timeout <= 0 or not select.select([master], [], [], timeout)[0]:
                raise AssertionError(f"Timed out running {argv}: {data[-2000:]!r}")
            try:
                chunk = os.read(master, 65536)
            except OSError as exc:
                if exc.errno != errno.EIO:
                    raise
                break
            if not chunk:
                break
            data.extend(chunk)
            if pending:
                expected, reply = pending[0]
                pos = data.find(expected, offset)
                if pos >= 0:
                    pending.pop(0)
                    offset = pos + len(expected)
                    os.write(master, reply)
        _, status = os.waitpid(pid, 0)
        pid = None
        assert os.waitstatus_to_exitcode(status) == 0 and not pending, (argv, pending, data)
        return data.decode().replace("\r", "")
    finally:
        os.close(master)
        if pid:
            os.kill(pid, signal.SIGKILL)
            os.waitpid(pid, 0)


READY = b"TR300_READY"
EXIT = [(READY, b"exit\n")]


def install_fixture(home, binary, env):
    # Create the login file before install so Bash and the installer select it.
    (home / ".bash_profile").touch()
    subprocess.run([binary, "install"], check=True, env=env, capture_output=True)
    for profile in [".bashrc", ".bash_profile", ".zshrc"]:
        with (home / profile).open("a") as stream:
            stream.write("\ncase $- in *i*) printf 'TR300_READY\\n';; esac\n")
    with (home / ".config/fish/config.fish").open("a") as stream:
        stream.write("\nif status is-interactive; echo TR300_READY; end\n")


def local(binary):
    with tempfile.TemporaryDirectory() as folder:
        home = Path(folder)
        env = {"PATH": os.environ["PATH"], "HOME": folder, "XDG_CONFIG_HOME": str(home / ".config"),
               "TERM": "xterm", "LC_ALL": "C", "PS1": "$ ", "SHELL": "/bin/bash"}
        install_fixture(home, binary, env)
        for shell in [["bash", "-i"], ["bash", "--login", "-i"]]:
            out = terminal(shell, env, exchanges=[(READY, b"bash -i\n"), (READY, b"exit\nexit\n")])
            assert out.count("PROCESSOR") == 1, out
        for shell in ["zsh", "fish"]:
            if shutil.which(shell):
                out = terminal([shell, "-i"], env, exchanges=EXIT)
                assert out.count("PROCESSOR") == 1, out
        # A raw PTY has no xterm display to answer .NET cursor queries. Make
        # the first actual PowerShell prompt read the exit from the terminal;
        # the editor never takes over input before the test can close the host.
        with (home / ".config/powershell/profile.ps1").open("a") as stream:
            stream.write("\nRemove-Module PSReadLine -ErrorAction SilentlyContinue\n"
                         "function global:prompt {\n"
                         "    [Console]::WriteLine('TR300_READY')\n"
                         "    if ([IO.StreamReader]::new([Console]::OpenStandardInput()).ReadLine() -ceq 'exit') { [Environment]::Exit(0) }\n"
                         "    throw 'Expected exit from the interactive PTY'\n}\n")
        ps_env = {**env, "TERM": "dumb", "POWERSHELL_TELEMETRY_OPTOUT": "1"}
        pwsh = shutil.which("pwsh")
        assert pwsh, "PowerShell 7 is required on this hosted qualification runner"
        for switches in [[], ["-i"]]:
            out = terminal([pwsh, "-NoLogo", *switches], ps_env, exchanges=EXIT)
            assert out.count("PROCESSOR") == 1, out
        script = home / "command.ps1"
        script.write_text("Write-Output 'PS_COMMAND_OK'\n")
        for switches in [["-Command", "'PS_COMMAND_OK'"], ["-File", str(script)], [str(script)]]:
            out = terminal([pwsh, "-NoLogo", *switches], ps_env)
            assert "PS_COMMAND_OK" in out and "PROCESSOR" not in out, out
        piped = subprocess.run([pwsh, "-NoLogo", "-i"], input=b"exit\n", env=ps_env,
                               timeout=20, check=True, capture_output=True)
        assert b"PROCESSOR" not in piped.stdout, piped.stdout
        print("Passed native PowerShell interactive, -i, command/script and redirection cases")
        for width, height in [(80, 25), (26, 10), (11, 25), (80, 4), (0, 0)]:
            out = terminal([binary, "--ascii", "--startup"], env, (width, height))
            lines = out.splitlines()
            if not width:
                assert len(lines) == 1 and len(lines[0]) <= 24, out
            elif width < 12 or height <= 4:
                assert not lines, out
            else:
                assert 1 <= len(lines) <= height - 4, out
                for line in lines:
                    cells = sum(0 if unicodedata.combining(c) else
                                2 if unicodedata.east_asian_width(c) in "WF" else 1 for c in line)
                    assert cells < width, (width, line)
        out = terminal(["bash", "-c", 'source "$HOME/.bashrc"; echo COMMAND_OK'], env)
        assert out.strip() == "COMMAND_OK", out
        target = home / "redirected"
        terminal(["bash", "-c", 'exec "$1" --startup > "$2"', "bash", binary, target], env)
        assert target.read_bytes() == b""
        out = terminal(["bash", "-c", 'exec "$1" --startup </dev/null', "bash", binary], env)
        assert not out, out
        # Exercise the same profile-only opt-out and installer refresh used by updates.
        subprocess.run([binary, "uninstall"], input=b"1\n", check=True, capture_output=True, env=env)
        subprocess.run([binary, "configure-startup-user"], check=True, capture_output=True, env=env)
        assert not terminal([binary, "--startup"], env)
        assert "PROCESSOR" not in terminal(["bash", "-i"], env, exchanges=EXIT)
        assert "PROCESSOR" in terminal([binary, "--fast"], env)
        subprocess.run([binary, "install"], check=True, capture_output=True, env=env)
        assert terminal(["bash", "-i"], env, exchanges=EXIT).count("PROCESSOR") == 1

        timings = {"--fast": [], "--startup": []}
        for _ in range(3):
            for flag, samples in timings.items():
                start = time.monotonic()
                assert "PROCESSOR" in terminal([binary, flag], env)
                samples.append((time.monotonic() - start) * 1000)
        print("Native 80x25 PTY median (3 samples): " + ", ".join(
            f"{flag} {statistics.median(samples):.1f} ms" for flag, samples in timings.items()))
        print("Passed local shell, nested PTY, viewport, redirection and noninteractive cases")


def remote(ssh_args):
    env = dict(os.environ, TERM="xterm")
    out = terminal(["ssh", *ssh_args, "-tt", "tr300-fixture"], env, exchanges=EXIT)
    assert out.count("PROCESSOR") == 1 and "SERVER IP" in out and "127.0.0.1" in out, out
    for tty in ["-T", "-tt"]:
        args = ["ssh", *ssh_args, tty, "tr300-fixture", "printf '%s\\n' REMOTE_COMMAND_OK"]
        out = terminal(args, env)
        assert out.strip() == "REMOTE_COMMAND_OK", out
    print("Passed OpenSSH PTY login, command without PTY and forced-PTY command")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["local", "setup", "ssh"])
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    if args.mode == "local":
        local(args.path.resolve())
    elif args.mode == "setup":
        install_fixture(Path.home(), args.path.resolve(), os.environ)
    else:
        remote(["-F", str(args.path)])
