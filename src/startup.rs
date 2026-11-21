//! The automatic report only runs in a terminal. Manual reports are independent
//! of terminal geometry, startup preferences and shell nesting.

use std::io::{self, IsTerminal, Write};
use tr300::{install, report, CollectMode, Config, Result, SystemInfo};

pub(super) fn run(config: &Config, hook: bool) -> Result<bool> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(false);
    }
    if install::startup_disabled().unwrap_or(true) {
        return Ok(false);
    }
    #[cfg(windows)]
    if hook && (windows::already_displayed() || !windows::interactive_parent()) {
        return Ok(false);
    }
    #[cfg(not(windows))]
    let _ = hook;
    let info = SystemInfo::collect_with_mode(CollectMode::Fast)?;
    #[cfg(windows)]
    let info = {
        let mut info = info;
        tr300::collectors::platform::windows::supplement_startup(&mut info);
        info
    };
    let output = report::generate_startup(&info, config, viewport());
    if output.is_empty() {
        return Ok(false);
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(output.as_bytes())?;
    stdout.flush()?;
    #[cfg(windows)]
    if hook {
        windows::mark_displayed();
    }
    Ok(true)
}

fn viewport() -> Option<(u16, u16)> {
    // size() uses the visible console rectangle on Windows. On Unix its
    // error path spawns tput, so use the direct terminal ioctl wrapper there.
    #[cfg(windows)]
    let size = crossterm::terminal::size().ok()?;
    #[cfg(unix)]
    let size = {
        let window = crossterm::terminal::window_size().ok()?;
        (window.columns, window.rows)
    };
    (size.0 > 0 && size.1 > 0).then_some(size)
}

#[cfg(windows)]
mod windows {
    use std::ffi::OsString;
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    use winapi::um::wincon::{AddConsoleAliasW, GetConsoleAliasW};

    // Console aliases are attached to the console (including ConPTY). A
    // private executable namespace keeps this marker out of CMD's aliases.
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    pub(super) fn already_displayed() -> bool {
        let mut result = [0u16; 8];
        // SAFETY: all Win32 strings are NUL terminated and the supplied
        // capacity is in bytes. The API writes only into the result buffer.
        unsafe {
            GetConsoleAliasW(
                wide("shown").as_mut_ptr(),
                result.as_mut_ptr(),
                std::mem::size_of_val(&result) as u32,
                wide("QubeTX.TR300.Startup").as_mut_ptr(),
            ) != 0
        }
    }

    pub(super) fn mark_displayed() {
        // SAFETY: AddConsoleAliasW reads these three NUL-terminated strings
        // for the duration of the call.
        unsafe {
            AddConsoleAliasW(
                wide("shown").as_mut_ptr(),
                wide("1").as_mut_ptr(),
                wide("QubeTX.TR300.Startup").as_mut_ptr(),
            );
        }
    }

    pub(super) fn interactive_parent() -> bool {
        let own_pid = Pid::from_u32(std::process::id());
        let mut processes = System::new();
        processes.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[own_pid]),
            false,
            ProcessRefreshKind::new(),
        );
        let Some(parent) = processes.process(own_pid).and_then(|p| p.parent()) else {
            return false;
        };
        processes.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[parent]),
            false,
            ProcessRefreshKind::new().with_cmd(UpdateKind::Always),
        );
        processes
            .process(parent)
            .is_some_and(|p| interactive_arguments(&p.name().to_string_lossy(), p.cmd()))
    }

    fn interactive_arguments(name: &str, args: &[OsString]) -> bool {
        if args.is_empty() {
            return false;
        }
        let mut options = args
            .iter()
            .skip(1)
            .map(|s| s.to_string_lossy().to_ascii_lowercase());
        if name.eq_ignore_ascii_case("cmd.exe") {
            for arg in options {
                // CMD also accepts joined switches such as /d/ccommand.
                for option in arg.strip_prefix('/').unwrap_or("").split('/') {
                    if option.starts_with('c') {
                        return false;
                    }
                    if option.starts_with('k') {
                        return true;
                    }
                }
            }
            return true;
        }
        if !(name.eq_ignore_ascii_case("pwsh.exe") || name.eq_ignore_ascii_case("powershell.exe")) {
            return false;
        }
        let mut no_exit = false;
        while let Some(arg) = options.next() {
            let opt = arg.trim_start_matches(['-', '/']);
            if opt == arg {
                return no_exit;
            }
            match opt {
                "noexit" | "noe" => no_exit = true,
                "noprofile" | "nop" | "nologo" | "nol" | "mta" | "sta" | "login" | "l"
                | "interactive" | "i" => {}
                "workingdirectory" | "wd" | "executionpolicy" | "ep" | "ex" | "windowstyle"
                | "w" => {
                    options.next();
                }
                _ if opt.len() >= 4 && "noninteractive".starts_with(opt) => return false,
                "sshs" | "sshservermode" | "servermode" => return false,
                _ if "command".starts_with(opt)
                    || "file".starts_with(opt)
                    || "encodedcommand".starts_with(opt)
                    || "commandwithargs".starts_with(opt) =>
                {
                    return no_exit
                }
                _ => return false,
            }
        }
        true
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn distinguishes_shells_from_commands_even_with_a_console() {
            for (name, args, allowed) in [
                ("cmd.exe", vec!["/q"], true),
                ("cmd.exe", vec!["/d/cecho"], false),
                ("cmd.exe", vec!["/k", "title", "/c"], true),
                (
                    "pwsh.exe",
                    vec!["-WorkingDirectory", "C:/", "-NoLogo"],
                    true,
                ),
                ("pwsh.exe", vec!["-Command", "echo", "-NoExit"], false),
                ("powershell.exe", vec!["-File", "task.ps1"], false),
                ("pwsh.exe", vec!["-NoExit", "-Command", "setup"], true),
                ("pwsh.exe", vec!["-NoExit", "-NonInteractive"], false),
                ("pwsh.exe", vec!["-NoExit", "-NonIn"], false),
            ] {
                let args = std::iter::once(name)
                    .chain(args)
                    .map(OsString::from)
                    .collect::<Vec<_>>();
                assert_eq!(interactive_arguments(name, &args), allowed);
            }
        }
    }
}
