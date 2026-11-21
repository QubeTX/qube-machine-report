"""Request an actual Windows OpenSSH shell in a dedicated interactive console."""
import ctypes as C
from ctypes import wintypes as W
import os
from pathlib import Path
import subprocess
import sys
import time
import traceback


class Coord(C.Structure):
    _fields_ = [("x", C.c_short), ("y", C.c_short)]


class BufferInfo(C.Structure):
    _fields_ = [("size", Coord), ("cursor", Coord), ("attributes", W.WORD),
                ("window", W.SMALL_RECT), ("maximum", Coord)]


def shell(ssh, config):
    kernel = C.WinDLL("kernel32", use_last_error=True)
    kernel.GetStdHandle.restype = W.HANDLE
    handle = kernel.GetStdHandle(-11)

    def call(name, types, *args):
        fn = getattr(kernel, name)
        fn.argtypes = [W.HANDLE, *types]
        if not fn(handle, *args):
            raise C.WinError(C.get_last_error())

    rect = W.SMALL_RECT(0, 0, 79, 24)
    call("SetConsoleWindowInfo", [W.BOOL, C.POINTER(W.SMALL_RECT)], True, C.byref(rect))
    call("SetConsoleScreenBufferSize", [Coord], Coord(80, 512))
    proc = subprocess.Popen([ssh, "-F", config, "-tt", "tr300-fixture"])
    try:
        deadline = time.monotonic() + 40
        while True:
            info = BufferInfo()
            call("GetConsoleScreenBufferInfo", [C.POINTER(BufferInfo)], C.byref(info))
            rows = []
            for row in range(info.window.Top, info.window.Bottom + 1):
                buf = C.create_unicode_buffer(81)
                count = W.DWORD()
                call("ReadConsoleOutputCharacterW", [W.LPWSTR, W.DWORD, Coord, C.POINTER(W.DWORD)],
                     buf, 80, Coord(0, row), C.byref(count))
                rows.append(buf.value.rstrip())
            view = "\n".join(rows)
            if "SERVER IP" in view and "127.0.0.1" in view and view.count("PROCESSOR") == 1:
                return
            if proc.poll() is not None or time.monotonic() > deadline:
                raise AssertionError(f"Expected SSH startup facts on the first remote shell:\n{view}")
            time.sleep(0.2)
    finally:
        if proc.poll() is None:
            proc.terminate()
        proc.wait(timeout=10)


if __name__ == "__main__":
    ssh, config, logfile = sys.argv[1:4]
    if sys.argv[4:] == ["child"]:
        try:
            shell(ssh, config)
        except Exception:
            Path(logfile).write_text(traceback.format_exc(), encoding="utf8")
            sys.exit(1)
    else:
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = subprocess.SW_HIDE
        result = subprocess.run([sys.executable, __file__, ssh, config, logfile, "child"],
                                creationflags=subprocess.CREATE_NEW_CONSOLE, startupinfo=startup, timeout=65)
        if result.returncode:
            raise RuntimeError(Path(logfile).read_text(encoding="utf8"))
