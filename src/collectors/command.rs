use std::ffi::OsStr;
use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const MAX_PIPE_BYTES: usize = 8 * 1024 * 1024;

/// Timeout budget for collector subprocesses.
#[non_exhaustive]
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
    run_output_with_env(program, args, std::iter::empty::<(&str, &str)>(), timeout)
}

/// Like `run_output` but lets the caller set environment variables on
/// the child process.
///
/// Required for subprocesses whose output is parsed by string match —
/// `lscpu`, `lastlog`, `last`, and similar tools localize their column
/// labels per `LC_MESSAGES` / `LC_ALL`. Calling them with `LC_ALL=C`
/// forces the C locale and the English-language output our parsers
/// expect, avoiding silent misses on non-English systems. (audit
/// finding F19, v3.15.8+)
pub fn run_output_with_env<I, S, E, K, V>(
    program: &str,
    args: I,
    envs: E,
    timeout: CommandTimeout,
) -> Option<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    E: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(envs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Give Unix probes their own process group so timeout cleanup also stops
    // helper processes they spawned. Without this, a shell grandchild can
    // keep the inherited pipes open after the direct child is killed.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command.spawn().ok()?;

    // A Windows Job Object gives timeout cleanup tree semantics equivalent to
    // the Unix process group above. Assignment can legitimately fail inside a
    // restrictive parent job, so this is best-effort and direct-child kill
    // remains the fallback.
    #[cfg(windows)]
    let child_job = ChildJob::assign(&child);

    // Drain both pipes while the child is running. Waiting for process exit
    // before reading can deadlock when either pipe fills its OS buffer: the
    // child blocks on write, never exits, and is misreported as a timeout.
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    let stdout_reader = read_pipe_async(stdout);
    let stderr_reader = read_pipe_async(stderr);

    let deadline = Instant::now() + timeout.duration();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Closing a successful Windows job after the direct command
                // exits terminates any stray descendants before we wait for
                // inherited pipe handles to close.
                #[cfg(windows)]
                drop(child_job);

                let stdout = match receive_pipe(&stdout_reader, deadline) {
                    Some(bytes) => bytes,
                    None => {
                        terminate_child(&mut child);
                        return None;
                    }
                };
                let stderr = match receive_pipe(&stderr_reader, deadline) {
                    Some(bytes) => bytes,
                    None => {
                        terminate_child(&mut child);
                        return None;
                    }
                };
                return Some(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    #[cfg(windows)]
                    if let Some(job) = &child_job {
                        job.terminate();
                    }
                    terminate_child(&mut child);
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                #[cfg(windows)]
                if let Some(job) = &child_job {
                    job.terminate();
                }
                terminate_child(&mut child);
                return None;
            }
        }
    }
}

fn read_pipe(mut pipe: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.by_ref()
        .take(MAX_PIPE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PIPE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "collector output exceeded the 8 MiB safety limit",
        ));
    }
    Ok(bytes)
}

fn read_pipe_async(pipe: impl Read + Send + 'static) -> mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(read_pipe(pipe));
    });
    receiver
}

fn receive_pipe(
    receiver: &mpsc::Receiver<std::io::Result<Vec<u8>>>,
    deadline: Instant,
) -> Option<Vec<u8>> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    receiver.recv_timeout(remaining).ok()?.ok()
}

#[cfg(windows)]
struct ChildJob(winapi::shared::ntdef::HANDLE);

#[cfg(windows)]
impl ChildJob {
    fn assign(child: &Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;
        use winapi::um::jobapi2::{
            AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        };
        use winapi::um::winnt::{
            JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        let handle = unsafe { CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
        if handle.is_null() {
            return None;
        }

        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &mut limits as *mut _ as *mut _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } != 0;
        let assigned = configured
            && unsafe { AssignProcessToJobObject(handle, child.as_raw_handle() as *mut _) } != 0;
        if !assigned {
            unsafe { winapi::um::handleapi::CloseHandle(handle) };
            return None;
        }
        Some(Self(handle))
    }

    fn terminate(&self) {
        unsafe {
            winapi::um::jobapi2::TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for ChildJob {
    fn drop(&mut self) {
        unsafe {
            winapi::um::handleapi::CloseHandle(self.0);
        }
    }
}

fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        // SAFETY: the child was spawned as leader of its own process group
        // above, so the negative PID targets only that probe's group.
        let _ = libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
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

/// Like `run_stdout`, but forces `LC_ALL=C` on the child process so
/// label-based parsers don't break under a non-English locale.
///
/// Tools like `lscpu` and `lastlog` localize their column labels per
/// `LC_MESSAGES` (`Socket(s):` becomes `Sockel:` in German, `Never
/// logged in` becomes `Nie eingeloggt`). When the parser is matching
/// English strings, an unset `LC_ALL=C` means the parser silently
/// misses on those systems. (audit finding F19, v3.15.8+)
pub fn run_stdout_c_locale<I, S>(program: &str, args: I, timeout: CommandTimeout) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output_with_env(program, args, [("LC_ALL", "C")], timeout)?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
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

    #[test]
    fn command_helper_drains_output_larger_than_pipe_capacity() {
        #[cfg(unix)]
        let output = run_output(
            "dd",
            ["if=/dev/zero", "bs=262144", "count=1"],
            CommandTimeout::Normal,
        )
        .expect("large output should be drained while the child runs");

        #[cfg(windows)]
        let output = run_output(
            "powershell",
            [
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Out.Write('x' * 262144)",
            ],
            CommandTimeout::Normal,
        )
        .expect("large output should be drained while the child runs");

        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 262_144);
    }

    #[cfg(unix)]
    #[test]
    fn command_helper_rejects_unbounded_output() {
        let output = run_output(
            "dd",
            ["if=/dev/zero", "bs=1048576", "count=9"],
            CommandTimeout::Slow,
        );
        assert!(output.is_none());
    }
}
