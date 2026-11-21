"""Read the visible rows of a disposable classic Windows console after TR-300 runs."""
import ctypes as C
from ctypes import wintypes as W
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import traceback


class Coord(C.Structure):
    _fields_ = [("x", C.c_short), ("y", C.c_short)]


class BufferInfo(C.Structure):
    _fields_ = [
        ("size", Coord), ("cursor", Coord), ("attributes", W.WORD),
        ("window", W.SMALL_RECT), ("maximum", Coord),
    ]


def exercise(exe, shell, cols, rows, directory):
    kernel = C.WinDLL("kernel32", use_last_error=True)
    kernel.GetStdHandle.restype = W.HANDLE
    handle = kernel.GetStdHandle(-11)

    def call(name, types, *args):
        fn = getattr(kernel, name)
        fn.argtypes = [W.HANDLE, *types]
        if not fn(handle, *args):
            raise C.WinError(C.get_last_error())

    rect = W.SMALL_RECT(0, 0, cols - 1, rows - 1)
    call("SetConsoleWindowInfo", [W.BOOL, C.POINTER(W.SMALL_RECT)], True, C.byref(rect))
    call("SetConsoleScreenBufferSize", [Coord], Coord(cols, 512))
    # Start with pre-existing shell output. The header must survive scrolling.
    call("SetConsoleCursorPosition", [Coord], Coord(0, rows - 2))
    if shell == "cmd":
        batch = directory / "interactive.cmd"
        batch.write_text('@echo off\n"%TR300_TARGET%" --startup-hook\n'
                         'if errorlevel 1 exit 90\n"%TR300_TARGET%" --startup-hook\n'
                         'if not errorlevel 3 exit 91\nexit 0\n', encoding="ascii")
        command = [os.environ["COMSPEC"], "/d", "/q", "/k", str(batch)]
        quiet = [os.environ["COMSPEC"], "/d", "/c", str(exe), "--startup-hook"]
    else:
        host = (Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/powershell.exe"
                if shell == "ps5" else shutil.which("pwsh"))
        if not host:
            raise RuntimeError("PowerShell 7 unavailable")
        args = [str(host), "-NoLogo", "-NoProfile"]
        run = "& $env:TR300_TARGET --startup-hook"
        command = [*args, "-NoExit", "-Command", run + "; if ($LASTEXITCODE -ne 0) { exit 90 }; "
                   + run + "; if ($LASTEXITCODE -ne 3) { exit 91 }; exit 0"]
        quiet = [*args, "-Command", run + "; exit $LASTEXITCODE"]
    env = os.environ.copy()
    env["TR300_TARGET"] = str(exe)
    # A noninteractive command can still have real console handles.
    if subprocess.run(quiet, env=env, timeout=30).returncode != 3:
        raise AssertionError(f"{shell} emitted a banner in command mode")
    subprocess.run(command, env=env, check=True, timeout=30)
    print("Profiles loaded (test)", flush=True)
    print("Prompt> ", end="", flush=True)
    info = BufferInfo()
    call("GetConsoleScreenBufferInfo", [C.POINTER(BufferInfo)], C.byref(info))
    lines = []
    for row in range(info.window.Top, info.window.Bottom + 1):
        buf = C.create_unicode_buffer(cols + 1)
        count = W.DWORD()
        call("ReadConsoleOutputCharacterW", [W.LPWSTR, W.DWORD, Coord, C.POINTER(W.DWORD)],
             buf, cols, Coord(info.window.Left, row), C.byref(count))
        lines.append(buf.value.rstrip())
    visible = "\n".join(lines)
    if socket.gethostname().casefold() not in visible.casefold() or "Prompt>" not in visible or "Profiles loaded" not in visible:
        raise AssertionError(f"{shell} {cols}x{rows} lost heading or prompt:\n{visible}")


def main():
    if len(sys.argv) > 2:
        exe, shell, width, height, tmp = sys.argv[1:]
        try:
            exercise(Path(exe), shell, int(width), int(height), Path(tmp))
        except Exception:
            (Path(tmp) / "failure.txt").write_text(traceback.format_exc(), encoding="utf-8")
            return 1
        return 0
    exe = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory() as tmp:
        for shell in ("cmd", "ps5", "ps7"):
            for width, height in ((80, 25), (42, 12)):
                startup = subprocess.STARTUPINFO()
                startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                startup.wShowWindow = subprocess.SW_HIDE
                result = subprocess.run([sys.executable, __file__, str(exe), shell,
                                         str(width), str(height), tmp], startupinfo=startup,
                                        creationflags=subprocess.CREATE_NEW_CONSOLE, timeout=90)
                if result.returncode:
                    raise RuntimeError((Path(tmp) / "failure.txt").read_text(encoding="utf-8"))
                print(f"{shell} {width}x{height}: header and prompt visible, once per console")
    return 0


if __name__ == "__main__":
    sys.exit(main())
