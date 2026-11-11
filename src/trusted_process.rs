//! Process boundaries for implicit system probes and verified maintenance payloads.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve a system tool without consulting the caller's search path.
pub(crate) fn system_command(program: &str) -> io::Result<Command> {
    let requested = Path::new(program);
    let name = requested
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "system program has no valid filename",
            )
        })?;
    #[cfg(windows)]
    let executable = {
        let system = windows_system_directory()?;
        let programs = windows_program_files_x64().ok();
        let candidates = windows_candidates(name, &system, programs.as_deref());
        select_candidate(requested, candidates)?
    };
    #[cfg(unix)]
    let executable = select_candidate(
        requested,
        ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
            .map(|directory| Path::new(directory).join(name))
            .to_vec(),
    )?;
    #[cfg(not(any(windows, unix)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "system process paths are unsupported",
    ));

    let mut command = Command::new(&executable);
    configure_environment(&mut command)?;
    #[cfg(windows)]
    {
        command.current_dir(executable.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "system program has no parent")
        })?);
        if executable
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.eq_ignore_ascii_case("powershell.exe") || name.eq_ignore_ascii_case("pwsh.exe")
            })
        {
            command.env("PSModulePath", executable.parent().unwrap().join("Modules"));
        }
    }
    #[cfg(unix)]
    command.current_dir("/");
    Ok(command)
}

#[cfg(windows)]
pub(crate) fn powershell_command_for(host: &str, script: &str) -> io::Result<Command> {
    if !matches!(host, "powershell" | "pwsh") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unrecognized PowerShell host",
        ));
    }
    let mut command = system_command(host)?;
    // PowerShell 7 prepends user module paths during startup even when the
    // inherited value is pinned. Reset it before the first cmdlet can autoload.
    let guarded = format!("$env:PSModulePath=[IO.Path]::Combine($PSHOME,'Modules'); {script}");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &guarded,
    ]);
    Ok(command)
}

fn select_candidate(requested: &Path, candidates: Vec<PathBuf>) -> io::Result<PathBuf> {
    let bare =
        requested.components().count() == 1 && requested.file_name() == Some(requested.as_os_str());
    candidates
        .into_iter()
        .find(|candidate| {
            let matches = bare || requested == candidate;
            matches && candidate.is_absolute() && candidate.is_file()
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "trusted system executable is unavailable: {}",
                    requested.display()
                ),
            )
        })
}

/// Sanitize nested execution while leaving the verified caller's cwd and arguments intact.
pub(crate) fn configure_environment(command: &mut Command) -> io::Result<()> {
    // Ownership checks need the invocation context that existed before the
    // child's execution path/cwd is restricted. These values never select tools.
    let invocation_path = std::env::var_os("PATH").unwrap_or_default();
    #[cfg(windows)]
    let invocation_path = windows_expand_environment(&invocation_path)?;
    command
        .env("TR300_INVOCATION_PATH", invocation_path)
        .env("TR300_INVOCATION_CWD", std::env::current_dir()?);
    for key in [
        "BASH_ENV",
        "ENV",
        "ZDOTDIR",
        "ZSH_ENV",
        "SHELLOPTS",
        "BASHOPTS",
        "BASH_XTRACEFD",
        "PS4",
        "CDPATH",
        "GLOBIGNORE",
        "PYTHONPATH",
        "PYTHONHOME",
        "PYTHONSTARTUP",
        "PERL5OPT",
        "PERL5LIB",
        "RUBYOPT",
        "RUBYLIB",
        "NODE_OPTIONS",
        "NODE_PATH",
        "DEVELOPER_DIR",
        "TOOLCHAINS",
        "SDKROOT",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "LD_AUDIT",
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "GCONV_PATH",
        "OPENSSL_CONF",
        "OPENSSL_CONF_INCLUDE",
        "OPENSSL_MODULES",
        "OPENSSL_ENGINES",
        "CURL_HOME",
        "WGETRC",
    ] {
        command.env_remove(key);
    }
    let overrides = command
        .get_envs()
        .map(|(key, _)| key.to_os_string())
        .collect::<Vec<_>>();
    for key in std::env::vars_os().map(|(key, _)| key).chain(overrides) {
        let name = key.to_string_lossy();
        if name.starts_with("BASH_FUNC_") || name.starts_with("LD_") || name.starts_with("DYLD_") {
            command.env_remove(key);
        }
    }
    #[cfg(unix)]
    command.env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
    #[cfg(windows)]
    {
        let system = windows_system_directory()?;
        let windows = system
            .parent()
            .filter(|path| path.is_absolute())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Windows system directory has no absolute parent",
                )
            })?;
        let powershell = system.join("WindowsPowerShell").join("v1.0");
        let mut paths = vec![
            system.clone(),
            system.join("Wbem"),
            windows.to_path_buf(),
            powershell.clone(),
        ];
        command.env("SystemRoot", windows).env("windir", windows);
        command.env("COMSPEC", system.join("cmd.exe"));
        command.env("PSModulePath", powershell.join("Modules"));
        match windows_program_files_x64() {
            Ok(programs) => {
                command
                    .env("ProgramFiles", &programs)
                    .env("ProgramW6432", &programs);
                paths.push(programs.join("NVIDIA Corporation").join("NVSMI"));
                paths.push(programs.join("PowerShell").join("7"));
            }
            Err(_) => {
                command
                    .env_remove("ProgramFiles")
                    .env_remove("ProgramW6432");
            }
        }
        command.env(
            "PATH",
            std::env::join_paths(paths)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
        );
    }
    Ok(())
}

#[cfg(windows)]
fn windows_expand_environment(value: &std::ffi::OsStr) -> io::Result<std::ffi::OsString> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use winapi::um::processenv::ExpandEnvironmentStringsW;

    const MAX_UNITS: usize = 32_768;
    let source: Vec<u16> = value.encode_wide().chain(Some(0)).collect();
    if source.len() > MAX_UNITS || source[..source.len() - 1].contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invocation PATH exceeds the bounded Windows environment format",
        ));
    }
    let mut buffer = vec![0_u16; source.len().max(260)];
    loop {
        // SAFETY: source is terminated; the separate output buffer is writable
        // for the supplied size. Expansion reads the actual parent environment.
        let length = unsafe {
            ExpandEnvironmentStringsW(source.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32)
        };
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        let length = length as usize;
        if length <= buffer.len() {
            // The returned length includes the terminating NUL. Unknown names
            // stay unexpanded, and relative/empty PATH entries stay unchanged.
            buffer.truncate(length - 1);
            return Ok(std::ffi::OsString::from_wide(&buffer));
        }
        if length > MAX_UNITS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expanded invocation PATH exceeds the bounded Windows environment format",
            ));
        }
        buffer.resize(length, 0);
    }
}

#[cfg(windows)]
fn windows_candidates(name: &str, system: &Path, programs: Option<&Path>) -> Vec<PathBuf> {
    let name = if name.to_ascii_lowercase().ends_with(".exe") {
        name.to_string()
    } else {
        format!("{name}.exe")
    };
    match name.to_ascii_lowercase().as_str() {
        "powershell.exe" => vec![system.join("WindowsPowerShell").join("v1.0").join(name)],
        "pwsh.exe" => programs
            .map(|root| vec![root.join("PowerShell").join("7").join(name)])
            .unwrap_or_default(),
        "nvidia-smi.exe" => {
            let mut paths = vec![system.join(&name)];
            if let Some(root) = programs {
                paths.push(root.join("NVIDIA Corporation").join("NVSMI").join(name));
            }
            paths
        }
        _ => vec![system.join(name)],
    }
}

#[cfg(windows)]
pub(crate) fn windows_system_directory() -> io::Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::sysinfoapi::GetSystemDirectoryW;

    let mut buffer = vec![0_u16; 260];
    loop {
        // SAFETY: buffer is writable for the supplied UTF-16 code-unit count.
        let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        let length = length as usize;
        if length < buffer.len() {
            buffer.truncate(length);
            let directory = PathBuf::from(OsString::from_wide(&buffer));
            if !directory.is_absolute() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Windows returned a non-absolute system directory",
                ));
            }
            return Ok(directory);
        }
        let required = length
            .checked_add(1)
            .filter(|size| *size > buffer.len() && *size <= 32_768);
        let Some(required) = required else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Windows returned an invalid system-directory length",
            ));
        };
        buffer.resize(required, 0);
    }
}

#[cfg(windows)]
pub(crate) fn windows_program_files_x64() -> io::Result<PathBuf> {
    windows_known_folder(&winapi::um::knownfolders::FOLDERID_ProgramFilesX64)
}

#[cfg(windows)]
pub(crate) fn windows_known_folder(id: &winapi::shared::guiddef::GUID) -> io::Result<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::combaseapi::CoTaskMemFree;
    use winapi::um::shlobj::SHGetKnownFolderPath;

    let mut directory = std::ptr::null_mut();
    // SAFETY: Windows allocates a NUL-terminated path for the fixed known-folder ID.
    let status = unsafe { SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut directory) };
    if directory.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "native known-folder directory is unavailable",
        ));
    }
    let path = if status >= 0 {
        (0..32_768)
            .find(|index| unsafe { *directory.add(*index) == 0 })
            .map(|length| {
                // SAFETY: the successful API path ends at the NUL found above.
                let units = unsafe { std::slice::from_raw_parts(directory, length) };
                PathBuf::from(OsString::from_wide(units))
            })
            .filter(|path| path.is_absolute())
    } else {
        None
    };
    // SAFETY: every nonnull allocation returned by this API is released here.
    unsafe { CoTaskMemFree(directory.cast()) };
    path.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "native known-folder directory is not a valid absolute path",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collectors::command::{run_prepared_output, CommandTimeout};

    #[test]
    fn rejects_configured_shells_outside_protected_system_directories() {
        let fixture = tempfile::tempdir().unwrap();
        let shell = fixture
            .path()
            .join(if cfg!(windows) { "cmd.exe" } else { "bash" });
        std::fs::write(&shell, b"not a system executable").unwrap();
        assert!(system_command(shell.to_str().unwrap()).is_err());
        assert!(system_command("../bash").is_err());
    }

    #[test]
    fn child_environment_removes_code_discovery_hooks_and_preserves_report_data() {
        let mut command = Command::new("not-executed");
        for name in [
            "BASH_ENV",
            "ENV",
            "BASH_FUNC_probe%%",
            "LD_PRELOAD",
            "LD_CUSTOM_TEST",
            "DYLD_CUSTOM_TEST",
            "PS4",
            "CDPATH",
            "GLOBIGNORE",
            "PYTHONPATH",
            "NODE_OPTIONS",
            "DEVELOPER_DIR",
            "TOOLCHAINS",
            "SDKROOT",
            "OPENSSL_CONF",
            "OPENSSL_MODULES",
        ] {
            command.env(name, "caller-selected");
        }
        command
            .env("DISPLAY", ":test")
            .env("LANG", "en_TEST.UTF-8")
            .env("TR300_INVOCATION_PATH", "preseeded-path")
            .env("TR300_INVOCATION_CWD", "preseeded-directory");
        configure_environment(&mut command).unwrap();
        let env: std::collections::HashMap<_, _> = command.get_envs().collect();
        for name in [
            "BASH_ENV",
            "ENV",
            "BASH_FUNC_probe%%",
            "LD_PRELOAD",
            "LD_CUSTOM_TEST",
            "DYLD_CUSTOM_TEST",
            "PS4",
            "CDPATH",
            "GLOBIGNORE",
            "PYTHONPATH",
            "NODE_OPTIONS",
            "DEVELOPER_DIR",
            "TOOLCHAINS",
            "SDKROOT",
            "OPENSSL_CONF",
            "OPENSSL_MODULES",
        ] {
            assert_eq!(
                env[std::ffi::OsStr::new(name)],
                None,
                "startup hook survived: {name}"
            );
        }
        assert_eq!(
            env[std::ffi::OsStr::new("DISPLAY")],
            Some(std::ffi::OsStr::new(":test"))
        );
        assert_eq!(
            env[std::ffi::OsStr::new("LANG")],
            Some(std::ffi::OsStr::new("en_TEST.UTF-8"))
        );
        let invocation_path = std::env::var_os("PATH").unwrap_or_default();
        #[cfg(windows)]
        let invocation_path = windows_expand_environment(&invocation_path).unwrap();
        assert_eq!(
            env[std::ffi::OsStr::new("TR300_INVOCATION_PATH")],
            Some(invocation_path.as_os_str())
        );
        assert_eq!(
            env[std::ffi::OsStr::new("TR300_INVOCATION_CWD")],
            Some(std::env::current_dir().unwrap().as_os_str())
        );
    }

    #[cfg(windows)]
    #[test]
    fn invocation_path_expands_original_environment_before_sanitizing_child() {
        const CHILD: &str = "TR300_TEST_INVOCATION_EXPANSION_CHILD";
        const UNKNOWN: &str = "TR300_TEST_UNDEFINED_INVOCATION_ENVIRONMENT";
        if let Some(marker) = std::env::var_os(CHILD) {
            let original_programs = std::env::var_os("ProgramFiles").unwrap();
            let expected = format!(
                "{}\\portable;;relative;%{UNKNOWN}%",
                original_programs.to_string_lossy()
            );
            let original_directory = std::env::current_dir().unwrap();
            let mut command = Command::new("not-executed");
            command
                .env("ProgramFiles", "preseeded-replacement")
                .env("TR300_INVOCATION_PATH", "preseeded-path")
                .current_dir(original_directory.join("child-directory"));
            configure_environment(&mut command).unwrap();
            let environment: std::collections::HashMap<_, _> = command.get_envs().collect();
            assert_eq!(
                environment[std::ffi::OsStr::new("TR300_INVOCATION_PATH")],
                Some(std::ffi::OsStr::new(&expected))
            );
            assert_eq!(
                environment[std::ffi::OsStr::new("TR300_INVOCATION_CWD")],
                Some(original_directory.as_os_str())
            );
            assert_eq!(
                command.get_current_dir(),
                Some(original_directory.join("child-directory").as_path())
            );
            assert_ne!(
                environment[std::ffi::OsStr::new("PATH")],
                Some(expected.as_ref())
            );
            assert!(windows_expand_environment(std::ffi::OsStr::new(&"a".repeat(32_768))).is_err());
            std::fs::write(marker, b"expanded original invocation context").unwrap();
            return;
        }

        let fixture = tempfile::tempdir().unwrap();
        let marker = fixture.path().join("expanded-context");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "trusted_process::tests::invocation_path_expands_original_environment_before_sanitizing_child"])
            .current_dir(fixture.path())
            .env("PATH", format!("%ProgramFiles%\\portable;;relative;%{UNKNOWN}%"))
            .env("ProgramFiles", fixture.path().join("original-programs"))
            .env_remove(UNKNOWN)
            .env(CHILD, &marker);
        let output = run_prepared_output(command, CommandTimeout::Slow).unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(marker.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn windows_dormant_probe_builders_bind_native_executables_and_environment() {
        let system = windows_system_directory().unwrap();
        for name in ["powershell", "cmd", "bcdedit", "ipconfig", "reg"] {
            let command = system_command(name).unwrap();
            let executable = Path::new(command.get_program());
            assert!(executable.starts_with(&system));
            assert_eq!(command.get_current_dir(), executable.parent());
            let environment: std::collections::HashMap<_, _> = command.get_envs().collect();
            assert_eq!(
                environment[std::ffi::OsStr::new("SystemRoot")],
                system.parent().map(Path::as_os_str)
            );
            assert_eq!(
                environment[std::ffi::OsStr::new("windir")],
                system.parent().map(Path::as_os_str)
            );
            assert!(environment[std::ffi::OsStr::new("PATH")].is_some());
            if name == "powershell" {
                assert_eq!(
                    environment[std::ffi::OsStr::new("PSModulePath")],
                    Some(executable.parent().unwrap().join("Modules").as_os_str())
                );
            }
        }
        if let Ok(command) = system_command("pwsh") {
            let programs = windows_program_files_x64().unwrap();
            let executable = programs.join("PowerShell").join("7").join("pwsh.exe");
            assert_eq!(command.get_program(), executable.as_os_str());
            assert!(command.get_envs().any(|(key, value)| key == "PSModulePath"
                && value == Some(executable.parent().unwrap().join("Modules").as_os_str())));
        }
    }

    #[cfg(windows)]
    #[test]
    fn powershell_runtime_reset_precedes_module_autoload_in_both_available_hosts() {
        let fixture = tempfile::tempdir().unwrap();
        let module = fixture.path().join("Microsoft.PowerShell.Archive");
        std::fs::create_dir(&module).unwrap();
        std::fs::write(module.join("Microsoft.PowerShell.Archive.psd1"), "@{RootModule='Microsoft.PowerShell.Archive.psm1';ModuleVersion='999.0';FunctionsToExport=@('Expand-Archive')}").unwrap();
        std::fs::write(module.join("Microsoft.PowerShell.Archive.psm1"), "[IO.File]::WriteAllText($env:TR300_TEST_MODULE_MARKER,'executed'); function Expand-Archive { param($LiteralPath,$DestinationPath,$ErrorAction) }; Export-ModuleMember -Function Expand-Archive").unwrap();
        let marker = fixture.path().join("module-executed");
        let quote = |path: &Path| path.to_string_lossy().replace('\'', "''");
        let script = format!("try {{ Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -ErrorAction Stop }} catch {{}}; [Console]::Write($env:PSModulePath)", quote(&fixture.path().join("missing.zip")), quote(&fixture.path().join("unused")));
        for host in ["powershell", "pwsh"] {
            let mut command = match powershell_command_for(host, &script) {
                Ok(command) => command,
                Err(error) if host == "pwsh" && error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => panic!("required PowerShell host unavailable: {error}"),
            };
            let modules = Path::new(command.get_program())
                .parent()
                .unwrap()
                .join("Modules");
            // Deliberately bypass the prelaunch setting: the first script
            // statement must also defeat paths inserted by host startup.
            command
                .env("PSModulePath", fixture.path())
                .env("TR300_TEST_MODULE_MARKER", &marker);
            let output = run_prepared_output(command, CommandTimeout::Slow).unwrap();
            assert!(
                output.status.success(),
                "{host}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout)
                .eq_ignore_ascii_case(&modules.to_string_lossy()));
            assert!(!marker.exists(), "{host} autoloaded the caller's module");
        }
    }

    #[cfg(windows)]
    #[test]
    fn nvidia_candidates_preserve_both_documented_locations_without_path_fallback() {
        let fixture = tempfile::tempdir().unwrap();
        let system = fixture.path().join("system");
        let programs = fixture.path().join("programs");
        let legacy = programs.join("NVIDIA Corporation").join("NVSMI");
        std::fs::create_dir_all(&system).unwrap();
        std::fs::create_dir_all(&legacy).unwrap();
        let system_exe = system.join("nvidia-smi.exe");
        let legacy_exe = legacy.join("nvidia-smi.exe");
        std::fs::write(&system_exe, b"system fixture").unwrap();
        std::fs::write(&legacy_exe, b"legacy fixture").unwrap();
        let choose = || {
            select_candidate(
                Path::new("nvidia-smi"),
                windows_candidates("nvidia-smi", &system, Some(&programs)),
            )
        };
        assert_eq!(choose().unwrap(), system_exe);
        std::fs::remove_file(&system_exe).unwrap();
        assert_eq!(choose().unwrap(), legacy_exe);
        std::fs::remove_file(&legacy_exe).unwrap();
        assert!(choose().is_err());
    }

    #[test]
    fn fast_collection_ignores_caller_helpers_shell_and_startup_hooks() {
        const CHILD: &str = "TR300_TEST_SYSTEM_PROBE_CHILD";
        const ATTACK: &str = "TR300_TEST_SYSTEM_PROBE_ATTACK";
        if let Some(marker) = std::env::var_os(CHILD) {
            std::fs::write(marker, b"child started").unwrap();
            let info =
                crate::collectors::SystemInfo::collect_with_mode(crate::CollectMode::Fast).unwrap();
            assert_eq!(info.mode, crate::CollectMode::Fast);
            #[cfg(unix)]
            {
                let mut command = system_command("bash").unwrap();
                command.args(["-c", "printf safe-probe"]);
                let output = run_prepared_output(command, CommandTimeout::Normal).unwrap();
                assert!(output.status.success());
                assert_eq!(output.stdout, b"safe-probe");
            }
            #[cfg(windows)]
            {
                let mut command = system_command("powershell").unwrap();
                command.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "[Console]::Write('safe-probe')",
                ]);
                let output = run_prepared_output(command, CommandTimeout::Slow).unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, b"safe-probe");
            }
            return;
        }
        let fixture = tempfile::tempdir().unwrap();
        let marker = fixture.path().join("child-started");
        let attack = fixture.path().join("untrusted-executed");
        let helpers = [
            "bash",
            "zsh",
            "fish",
            "sw_vers",
            "vm_stat",
            "last",
            "lastlog",
            "lastlog2",
            "ip",
            "hostname",
            "resolvectl",
            "uname",
            "ps",
            "locale",
            "lspci",
            "ioreg",
            "sysctl",
            "pmset",
            "defaults",
            "system_profiler",
            "scutil",
            "ipconfig",
            "route",
            "fdesetup",
            "systemd-detect-virt",
            "xrandr",
            "wlr-randr",
            "zpool",
            "findmnt",
            "zfs",
            "lsblk",
            "dmidecode",
        ];
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for helper in helpers {
                let path = fixture.path().join(helper);
                std::fs::write(
                    &path,
                    b"#!/bin/sh\nprintf executed > \"$TR300_TEST_SYSTEM_PROBE_ATTACK\"\n",
                )
                .unwrap();
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        #[cfg(windows)]
        {
            let _ = helpers;
            let source = fixture.path().join("hostile.rs");
            std::fs::write(&source, r#"fn main() { if let Some(p) = std::env::var_os("TR300_TEST_SYSTEM_PROBE_ATTACK") { std::fs::write(p,b"executed").unwrap(); } }"#).unwrap();
            let executable = fixture.path().join("hostile.exe");
            let compile = Command::new("rustc")
                .arg(&source)
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(
                compile.status.success(),
                "{}",
                String::from_utf8_lossy(&compile.stderr)
            );
            for name in [
                "powershell",
                "pwsh",
                "cmd",
                "bcdedit",
                "ipconfig",
                "reg",
                "nvidia-smi",
            ] {
                std::fs::copy(&executable, fixture.path().join(format!("{name}.exe"))).unwrap();
            }
        }
        let startup = fixture.path().join("startup");
        std::fs::write(
            &startup,
            b"printf startup > \"$TR300_TEST_SYSTEM_PROBE_ATTACK\"\n",
        )
        .unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "trusted_process::tests::fast_collection_ignores_caller_helpers_shell_and_startup_hooks"])
            .current_dir(fixture.path()).env("PATH", fixture.path())
            .env("SHELL", fixture.path().join("bash"))
            .env("BASH_ENV", &startup).env("ENV", &startup).env("ZDOTDIR", fixture.path())
            .env("BASH_FUNC_printf%%", "() { printf executed > \"$TR300_TEST_SYSTEM_PROBE_ATTACK\"; }")
            .env("SystemRoot", fixture.path()).env("windir", fixture.path())
            .env("ProgramFiles", fixture.path()).env("ProgramW6432", fixture.path())
            .env("PSModulePath", fixture.path()).env(CHILD, &marker).env(ATTACK, &attack);
        let output = run_prepared_output(
            command,
            CommandTimeout::Custom(std::time::Duration::from_secs(30)),
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            marker.is_file(),
            "the complete fast collector did not execute"
        );
        assert!(
            !attack.exists(),
            "a caller-selected helper or startup hook executed"
        );
    }
}
