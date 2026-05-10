use std::ffi::OsStr;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Timeout budget for collector subprocesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTimeout {
    /// Fast-mode budget for shell auto-run paths.
    Fast,
    /// Default collector budget for ordinary small commands.
    Normal,
    /// Full-mode budget for deliberately slow platform probes.
    Slow,
    /// Test/custom override.
    Custom(Duration),
}

impl CommandTimeout {
    fn duration(self) -> Duration {
        match self {
            Self::Fast => Duration::from_millis(300),
            Self::Normal => Duration::from_millis(1500),
            Self::Slow => Duration::from_millis(5000),
            Self::Custom(duration) => duration,
        }
    }
}

/// Run a small collector subprocess and return its output, or `None` on
/// spawn failure, timeout, or wait/read failure.
///
/// Collector callers intentionally degrade silently; the report should keep
/// rendering when optional host tooling is absent, slow, or broken.
pub fn run_output<I, S>(program: &str, args: I, timeout: CommandTimeout) -> Option<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + timeout.duration();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// Run a collector subprocess with no arguments.
pub fn run_output_no_args(program: &str, timeout: CommandTimeout) -> Option<Output> {
    run_output(program, std::iter::empty::<&str>(), timeout)
}

/// Run a collector subprocess and return stdout as a lossy UTF-8 string only
/// when the command exits successfully.
pub fn run_stdout<I, S>(program: &str, args: I, timeout: CommandTimeout) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output(program, args, timeout)?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Run a no-argument collector subprocess and return successful stdout.
pub fn run_stdout_no_args(program: &str, timeout: CommandTimeout) -> Option<String> {
    run_stdout(program, std::iter::empty::<&str>(), timeout)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{run_output, CommandTimeout};

    #[test]
    fn command_helper_returns_successful_output() {
        #[cfg(unix)]
        let output = run_output("sh", ["-c", "printf ok"], CommandTimeout::Normal)
            .expect("command should produce output");

        #[cfg(windows)]
        let output = run_output("cmd", ["/C", "echo ok"], CommandTimeout::Normal)
            .expect("command should produce output");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("ok"));
    }

    #[test]
    fn command_helper_times_out_and_kills_child() {
        let started = Instant::now();

        #[cfg(unix)]
        let output = run_output(
            "sh",
            ["-c", "sleep 2; printf late"],
            CommandTimeout::Custom(Duration::from_millis(75)),
        );

        #[cfg(windows)]
        let output = run_output(
            "powershell",
            [
                "-NoProfile",
                "-Command",
                "Start-Sleep -Seconds 2; Write-Output late",
            ],
            CommandTimeout::Custom(Duration::from_millis(75)),
        );

        assert!(output.is_none());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
