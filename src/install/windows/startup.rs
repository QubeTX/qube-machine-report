use super::{fail_install, InstallStep, MARKER_END, MARKER_START};
use crate::error::{AppError, Result};
use crate::trusted_process::{windows_program_files_x64, windows_system_directory};
use std::fs;
use std::io;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SZ,
};
use winreg::{RegKey, RegValue};

const CMD_KEY: &str = r"Software\Microsoft\Command Processor";
const CREATED_PROFILE: &str = "# This profile was created by TR-300.";
fn cmd_hook(path: &str) -> String {
    format!(
        r#"(if exist "{path}" ("{path}" --startup-hook & set "TR300_AUTORUN_RAN=1")) & ver >nul"#
    )
}

const MACHINE_START: &str = "# TR-300 Machine Report (all users)";
const MACHINE_END: &str = "# End TR-300 (all users)";

pub(crate) fn install_machine_startup() -> Result<()> {
    configure_machine(true)
}
pub(crate) fn remove_machine_startup() -> Result<()> {
    configure_machine(false)
}

struct MachinePaths {
    exe: PathBuf,
    profiles: Vec<PathBuf>,
}

fn machine_paths() -> Result<MachinePaths> {
    let programs = windows_program_files_x64()?;
    let exe = programs.join(r"tr300\bin\tr300.exe");
    let system = windows_system_directory()?;
    let mut profiles = vec![system.join(r"WindowsPowerShell\v1.0\profile.ps1")];
    let ps7 = programs.join(r"PowerShell\7");
    if ps7.join("pwsh.exe").is_file() || ps7.join("profile.ps1").exists() {
        profiles.push(ps7.join("profile.ps1"));
    }
    let mut inspect = Vec::new();
    for path in std::iter::once(&exe).chain(profiles.iter()) {
        reject_reparse(path)?;
        let root = if path.starts_with(&programs) {
            &programs
        } else {
            &system
        };
        for ancestor in path.ancestors().take_while(|p| p.starts_with(root)) {
            if ancestor.try_exists()? && !inspect.contains(&ancestor) {
                inspect.push(ancestor);
            }
        }
    }
    let paths = inspect
        .into_iter()
        .map(|p| {
            let path = p
                .to_str()
                .ok_or_else(|| AppError::platform("Invalid Global startup path"))?;
            Ok(format!("'{}'", path.replace('\'', "''")))
        })
        .collect::<Result<Vec<_>>>()?
        .join(",");
    let script = format!(
        "$paths=@({paths}); {}",
        include_str!("startup-permissions.ps1")
    );
    let status = crate::trusted_process::powershell_command_for("powershell", &script)?.status()?;
    if !status.success() {
        return Err(AppError::platform("Global startup requires administrator-owned directories, binaries and profiles protected from unprivileged writes"));
    }
    Ok(MachinePaths { exe, profiles })
}

pub(crate) fn check_machine_startup() -> Result<()> {
    let MachinePaths { exe, profiles } = machine_paths()?;
    let block = ps_block(&exe, MACHINE_START, MACHINE_END)?;
    for path in &profiles {
        match fs::read(path) {
            Ok(old) => {
                edited_profile(&old, &block, MACHINE_START, MACHINE_END, true)?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    let hive = RegKey::predef(HKEY_LOCAL_MACHINE);
    let current = match hive
        .open_subkey_with_flags(CMD_KEY, KEY_QUERY_VALUE)
        .and_then(|k| k.get_raw_value("AutoRun"))
    {
        Ok(value) => Some(value),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    cmd_value(current.as_ref(), &exe, true)?;
    Ok(())
}

fn configure_machine(enable: bool) -> Result<()> {
    let MachinePaths { exe, profiles } = machine_paths()?;
    if enable && !exe.is_file() {
        return Err(AppError::platform(
            "Installed Global TR-300 executable is missing",
        ));
    }
    let block = ps_block(&exe, MACHINE_START, MACHINE_END)?;
    configure(
        &profiles,
        &block,
        MACHINE_START,
        MACHINE_END,
        RegKey::predef(HKEY_LOCAL_MACHINE),
        &exe,
        enable,
    )
}

fn reject_reparse(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => {
                return Err(AppError::platform(format!(
                    "Reparse point in Global startup path {}",
                    ancestor.display()
                )))
            }
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

pub(super) fn configure_user(profiles: &[PathBuf], enable: bool) -> Result<()> {
    let exe = std::env::current_exe()?;
    let block = ps_block(&exe, MARKER_START, MARKER_END)?;
    configure(
        profiles,
        &block,
        MARKER_START,
        MARKER_END,
        RegKey::predef(HKEY_CURRENT_USER),
        &exe,
        enable,
    )
}

fn ps_block(exe: &Path, start: &str, end: &str) -> Result<String> {
    let path = exe
        .to_str()
        .ok_or_else(|| AppError::platform("Unrepresentable PowerShell startup executable path"))?;
    // PS 5.1 treats BOM-less source as ANSI. Keep generated code ASCII even
    // for profiles in legacy encodings and non-ASCII Windows account names.
    let binary = if path.is_ascii() {
        format!("'{}'", path.replace('\'', "''"))
    } else {
        let chars = path
            .encode_utf16()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("-join [char[]]@({chars})")
    };
    let body = format!(
        r#"{start}
& {{
    $binary = {binary}
    if ([IO.File]::Exists($binary)) {{
        $PSNativeCommandUseErrorActionPreference = $false
        $previous = $ExecutionContext.SessionState.PSVariable.Get('global:LASTEXITCODE')
        $hadExit = $null -ne $previous
        if ($hadExit) {{ $savedExit = $previous.Value }}
        try {{
            & $binary --startup-hook
            if ($LASTEXITCODE -in 0, 3) {{ $env:TR300_AUTORUN_RAN = '1' }}
        }} finally {{
            if ($hadExit) {{ $global:LASTEXITCODE = $savedExit }}
            else {{ $ExecutionContext.SessionState.PSVariable.Remove('global:LASTEXITCODE') }}
        }}
    }}
}}
{end}"#
    );
    Ok(body.replace('\n', "\r\n"))
}

// Store the exact REG_SZ / REG_EXPAND_SZ representation, preserving the user's
// existing command and type. Only remove the exact command this install owns.
fn cmd_value(before: Option<&RegValue>, exe: &Path, enable: bool) -> Result<Option<RegValue>> {
    let path = exe
        .to_str()
        .filter(|s| !s.contains(['%', '!', '"', '\r', '\n']))
        .ok_or_else(|| {
            AppError::platform("Executable path cannot be quoted safely in Command Prompt AutoRun")
        })?;
    let hook = cmd_hook(path);
    let mut old = String::new();
    if let Some(raw) = before {
        if !matches!(raw.vtype, REG_SZ | REG_EXPAND_SZ) || raw.bytes.len() % 2 != 0 {
            return Err(AppError::platform(
                "Invalid Command Prompt AutoRun registry value",
            ));
        }
        let mut units = raw
            .bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>();
        if units.last() == Some(&0) {
            units.pop();
        }
        if units.contains(&0) {
            return Err(AppError::platform("Invalid embedded NUL in AutoRun"));
        }
        old = String::from_utf16(&units)
            .map_err(|_| AppError::platform("Invalid UTF-16 in AutoRun"))?;
    }
    let remainder = strip_cmd_hook(&old);
    let next = if enable && remainder.is_empty() {
        hook
    } else if enable {
        format!("{hook} & {remainder}")
    } else {
        remainder.into_owned()
    };
    if next.encode_utf16().count() > 8191 && next != old {
        return Err(AppError::platform(
            "The Command Prompt AutoRun command exceeds cmd.exe's supported length",
        ));
    }
    if next == old {
        return Ok(before.map(|r| RegValue {
            bytes: r.bytes.clone(),
            vtype: r.vtype.clone(),
        }));
    }
    if next.is_empty() {
        return Ok(None);
    }
    Ok(Some(RegValue {
        bytes: next
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect(),
        vtype: before.map_or(REG_SZ, |r| r.vtype.clone()),
    }))
}

// Decode only top-level sequential command boundaries. Keep every byte of
// foreign commands, including quoted ampersands and parenthesized commands.
fn strip_cmd_hook(text: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    let mut quoted = false;
    let mut depth = 0u32;
    let mut escaped = false;
    let mut command_start = 0;
    let mut cursor = 0;
    let data = text.as_bytes();
    while cursor < data.len() {
        if cursor == command_start {
            let rest = &text[cursor..];
            if let Some((path, _)) = rest
                .strip_prefix("(if exist \"")
                .and_then(|s| s.split_once('"'))
            {
                let owned = cmd_hook(path);
                if let Some(tail) = rest.strip_prefix(&owned) {
                    if tail.is_empty() {
                        return Cow::Borrowed(
                            text[..cursor]
                                .strip_suffix(" & ")
                                .unwrap_or(&text[..cursor]),
                        );
                    }
                    if let Some(after) = tail.strip_prefix(" & ") {
                        return Cow::Owned(format!("{}{after}", &text[..cursor]));
                    }
                }
            }
        }
        let c = data[cursor];
        if escaped {
            escaped = false;
        } else if c == b'^' && !quoted {
            escaped = true;
        } else if c == b'"' {
            quoted = !quoted;
        } else if !quoted {
            match c {
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                b' ' if depth == 0 && data[cursor..].starts_with(b" & ") => {
                    cursor += 3;
                    command_start = cursor;
                    continue;
                }
                _ => (),
            }
        }
        cursor += 1;
    }
    Cow::Borrowed(text)
}

fn profile_text(old: &str, block: &str, start: &str, end: &str, enable: bool) -> Result<String> {
    let mut inside = false;
    let mut cleaned = String::new();
    for line in old.split_inclusive('\n') {
        match line.trim_end_matches(['\r', '\n']) {
            marker if marker == start && !inside => inside = true,
            marker if marker == end && inside => inside = false,
            marker if marker == start || marker == end => {
                return Err(AppError::platform(
                    "Unbalanced TR-300 PowerShell profile markers",
                ))
            }
            _ if !inside => cleaned.push_str(line),
            _ => (),
        }
    }
    if inside {
        return Err(AppError::platform(
            "Unbalanced TR-300 PowerShell profile markers",
        ));
    }
    if enable {
        if !cleaned.is_empty() && !cleaned.ends_with('\n') {
            cleaned.insert_str(0, &format!("{block}\r\n"));
            return checked_profile(old, cleaned);
        }
        cleaned.push_str(block);
        cleaned.push_str("\r\n");
    }
    checked_profile(old, cleaned)
}

fn checked_profile(old: &str, next: String) -> Result<String> {
    if next != old && old.contains("# SIG # Begin signature block") {
        return Err(AppError::platform(
            "Signed PowerShell profile requires administrator-managed integration",
        ));
    }
    Ok(next)
}

/// PowerShell 5 commonly saves UTF-16LE with a BOM. Keep the original
/// encoding and BOM, and fail before mutation on invalid text.
fn edited_profile(
    bytes: &[u8],
    block: &str,
    start: &str,
    end: &str,
    enable: bool,
) -> Result<Vec<u8>> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little = bytes[0] == 0xff;
        let payload = &bytes[2..];
        if !payload.len().is_multiple_of(2) {
            return Err(AppError::platform("Incomplete UTF-16 profile"));
        }
        let units: Vec<_> = payload
            .chunks_exact(2)
            .map(|b| {
                if little {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect();
        let old =
            String::from_utf16(&units).map_err(|_| AppError::platform("Invalid UTF-16 profile"))?;
        let next = profile_text(&old, block, start, end, enable)?;
        let mut out = bytes[..2].to_vec();
        out.extend(next.encode_utf16().flat_map(|c| {
            if little {
                c.to_le_bytes()
            } else {
                c.to_be_bytes()
            }
        }));
        return Ok(out);
    }
    let (bom, payload) = bytes
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .map_or((&[][..], bytes), |payload| (&bytes[..3], payload));
    let old = match std::str::from_utf8(payload) {
        Ok(old) => old,
        Err(_) if bom.is_empty() && !payload.contains(&0) && block.is_ascii() => {
            // Unmarked ANSI profiles are widespread in Windows PowerShell 5.
            // Use a reversible one-byte mapping to modify only ASCII lines.
            let mapped: String = payload.iter().map(|&b| char::from(b)).collect();
            let next = profile_text(&mapped, block, start, end, enable)?;
            return Ok(next.chars().map(|c| c as u8).collect());
        }
        Err(_) => {
            return Err(AppError::platform(
                "Unsupported PowerShell profile encoding",
            ))
        }
    };
    let next = profile_text(old, block, start, end, enable)?;
    Ok([bom, next.as_bytes()].concat())
}

fn configure(
    profiles: &[PathBuf],
    block: &str,
    start: &str,
    end: &str,
    hive: RegKey,
    exe: &Path,
    enable: bool,
) -> Result<()> {
    // Plan every edit and validate every marker before applying anything.
    let mut edits = Vec::new();
    for path in profiles {
        let old = match fs::read(path) {
            Ok(content) => Some(content),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(fail_install(InstallStep::ReadProfile, path, e)),
        };
        let bytes = old.as_deref().unwrap_or_default();
        let with_creation = format!("{start}\r\n{CREATED_PROFILE}{}", &block[start.len()..]);
        let owned = old.is_none()
            || bytes
                .windows(CREATED_PROFILE.len())
                .any(|s| s == CREATED_PROFILE.as_bytes());
        let next = edited_profile(
            bytes,
            if owned { &with_creation } else { block },
            start,
            end,
            enable,
        )?;
        let remove_file = !enable && owned && next.is_empty();

        if next != bytes {
            edits.push((path, old, next, remove_file));
        }
    }
    let key = match hive.open_subkey_with_flags(CMD_KEY, KEY_QUERY_VALUE | KEY_SET_VALUE) {
        Ok(key) => Some(key),
        Err(e) if e.kind() == io::ErrorKind::NotFound && !enable => None,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Some(hive.create_subkey(CMD_KEY)?.0),
        Err(e) => return Err(e.into()),
    };
    let before = match key.as_ref().map(|k| k.get_raw_value("AutoRun")).transpose() {
        Ok(raw) => raw,
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let after = cmd_value(before.as_ref(), exe, enable)?;
    let mut applied = 0;
    let result = (|| -> Result<()> {
        for (path, old, next, remove_file) in &edits {
            let latest = match fs::read(path) {
                Ok(data) => Some(data),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
            if latest != *old {
                return Err(AppError::platform(format!(
                    "PowerShell profile changed during configuration: {}",
                    path.display()
                )));
            }

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| fail_install(InstallStep::CreateProfileDir, parent, e))?;
            }
            if old.is_some() && start == MARKER_START {
                let _ = super::super::backup_once(path);
            }
            if *remove_file {
                fs::remove_file(path)?;
            } else {
                super::super::atomic_write_bytes(path, next)
                    .map_err(|e| fail_install(InstallStep::WriteProfile, path, e))?;
            }
            applied += 1;
        }
        if after != before {
            if let Some(key) = &key {
                let latest = match key.get_raw_value("AutoRun") {
                    Ok(raw) => Some(raw),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                    Err(e) => return Err(e.into()),
                };
                if latest != before {
                    return Err(AppError::platform(
                        "Command Prompt AutoRun changed during configuration",
                    ));
                }
                match &after {
                    Some(value) => key.set_raw_value("AutoRun", value)?,
                    None => key.delete_value("AutoRun")?,
                }
            }
        }
        Ok(())
    })();
    if let Err(cause) = result {
        for (path, old, _, _) in edits[..applied].iter().rev() {
            match old {
                Some(text) => super::super::atomic_write_bytes(path, text)?,
                None => fs::remove_file(path)?,
            }
        }
        return Err(cause);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn round_trip_profile_encodings_and_markers() {
        let block = ps_block(
            Path::new(r"C:\Users\Zoë\tr300.exe"),
            MARKER_START,
            MARKER_END,
        )
        .unwrap();
        assert!(block.is_ascii());
        assert!(block.contains("--startup-hook"));
        let text = "# Original Ω line\r\n$answer = 42";
        let mut cases = vec![
            text.as_bytes().to_vec(),
            [b"\xef\xbb\xbf".as_slice(), text.as_bytes()].concat(),
            b"# caf\xe9".to_vec(),
        ];
        for (prefix, little) in [([0xff, 0xfe], true), ([0xfe, 0xff], false)] {
            let mut raw = prefix.to_vec();
            raw.extend(text.encode_utf16().flat_map(|c| {
                if little {
                    c.to_le_bytes()
                } else {
                    c.to_be_bytes()
                }
            }));
            cases.push(raw);
        }
        for original in cases {
            let enabled =
                edited_profile(&original, &block, MARKER_START, MARKER_END, true).unwrap();
            assert_eq!(
                edited_profile(&enabled, &block, MARKER_START, MARKER_END, true).unwrap(),
                enabled
            );
            assert_eq!(
                edited_profile(&enabled, &block, MARKER_START, MARKER_END, false).unwrap(),
                original
            );
        }
        assert!(edited_profile(
            MARKER_START.as_bytes(),
            &block,
            MARKER_START,
            MARKER_END,
            true
        )
        .is_err());
        assert!(edited_profile(
            b"# SIG # Begin signature block",
            &block,
            MARKER_START,
            MARKER_END,
            true
        )
        .is_err());
    }

    fn registry_text(value: &RegValue) -> String {
        let units = value
            .bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|c| *c != 0)
            .collect::<Vec<_>>();
        String::from_utf16(&units).unwrap()
    }

    fn registry_value(text: &str) -> RegValue {
        RegValue {
            vtype: REG_EXPAND_SZ,
            bytes: text
                .encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_le_bytes)
                .collect(),
        }
    }

    #[test]
    fn command_prompt_preserves_user_commands_and_registration_type() {
        let path = Path::new(r"C:\Program Files\tr300\bin\tr300.exe");
        let foreign = r#"if exist "%TEMP%\a & b" (echo "existing & value")"#;
        let original = registry_value(foreign);
        let registered = cmd_value(Some(&original), path, true).unwrap().unwrap();
        assert_eq!(registered.vtype, REG_EXPAND_SZ);
        assert!(cmd_value(Some(&registered), path, true).unwrap() == Some(registered));
        let edited = registry_value(&format!(
            "echo before & {} & {foreign}",
            cmd_hook(path.to_str().unwrap())
        ));
        let removed = cmd_value(Some(&edited), path, false).unwrap().unwrap();
        assert_eq!(registry_text(&removed), format!("echo before & {foreign}"));
        let relocated = cmd_value(Some(&edited), Path::new(r"C:\new\tr300.exe"), true)
            .unwrap()
            .unwrap();
        assert_eq!(
            registry_text(&relocated).matches("--startup-hook").count(),
            1
        );
    }

    #[test]
    fn isolated_profile_and_registry_transaction() {
        let temp = tempfile::Builder::new()
            .prefix("tr300-reg-fixture-")
            .tempdir()
            .unwrap();
        let sandbox = temp.path().file_name().unwrap().to_owned();
        let base = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software", winreg::enums::KEY_ALL_ACCESS)
            .unwrap();
        let (root, disposition) = base.create_subkey(&sandbox).unwrap();
        assert_eq!(disposition, winreg::enums::REG_CREATED_NEW_KEY);
        struct Cleanup(RegKey, std::ffi::OsString);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete_subkey_all(&self.1);
            }
        }
        let _cleanup = Cleanup(base, sandbox);
        let get_root = || {
            root.open_subkey_with_flags("", winreg::enums::KEY_ALL_ACCESS)
                .unwrap()
        };
        let profile = temp.path().join("pwsh.ps1");
        let second = temp.path().join("powershell.ps1");
        let exe = temp.path().join("tr300.exe");
        let block = ps_block(&exe, MARKER_START, MARKER_END).unwrap();
        let original = b"# this user code stays exactly as written";
        fs::write(&profile, original).unwrap();
        let paths = [profile.clone(), second.clone()];
        let apply = |enabled| {
            configure(
                &paths,
                &block,
                MARKER_START,
                MARKER_END,
                get_root(),
                &exe,
                enabled,
            )
        };
        apply(true).unwrap();
        let key = root
            .open_subkey_with_flags(CMD_KEY, winreg::enums::KEY_ALL_ACCESS)
            .unwrap();
        let first = key.get_raw_value("AutoRun").unwrap();
        let bytes = fs::read(&profile).unwrap();
        apply(true).unwrap();
        assert_eq!(bytes, fs::read(&profile).unwrap());
        assert!(key.get_raw_value("AutoRun").unwrap() == first);
        apply(false).unwrap();
        assert_eq!(fs::read(&profile).unwrap(), original);
        assert!(!second.exists());
        assert_eq!(
            key.get_raw_value("AutoRun").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );

        // Force a second write to fail after the first was applied. The first
        // profile is restored exactly, and no registry command is published.
        let blocked = temp.path().join("locked-profile.ps1");
        fs::write(&blocked, "user content").unwrap();
        use std::os::windows::fs::OpenOptionsExt;
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&blocked)
            .unwrap();
        let invalid_paths = [profile.clone(), blocked];
        assert!(configure(
            &invalid_paths,
            &block,
            MARKER_START,
            MARKER_END,
            get_root(),
            &exe,
            true
        )
        .is_err());
        assert_eq!(fs::read(&profile).unwrap(), original);
        assert_eq!(
            key.get_raw_value("AutoRun").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn powershell_preserves_exit_code_and_suppresses_legacy_after_skip() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("return three.cmd");
        fs::write(&fake, b"@exit /b 3\r\n").unwrap();
        let hook = ps_block(&fake, MARKER_START, MARKER_END).unwrap();
        for host in ["powershell", "pwsh"] {
            for preexisting in [true, false] {
                let initial = if preexisting {
                    "$global:LASTEXITCODE=37"
                } else {
                    "Remove-Variable LASTEXITCODE -Scope Global -ErrorAction Ignore"
                };
                let verify = if preexisting {
                    "$global:LASTEXITCODE -ne 37"
                } else {
                    "$null -ne (Get-Variable LASTEXITCODE -Scope Global -ErrorAction Ignore)"
                };
                let script = format!("$ErrorActionPreference='Stop'; $PSNativeCommandUseErrorActionPreference=$true; {initial}; $env:TR300_AUTORUN_RAN=$null\n{hook}\nif (({verify}) -or $env:TR300_AUTORUN_RAN -ne '1') {{ exit 11 }}; exit 0");
                let Ok(mut command) = crate::trusted_process::powershell_command_for(host, &script)
                else {
                    continue;
                };
                let output = command.output().unwrap();
                assert!(
                    output.status.success(),
                    "{host} {preexisting}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(output.stdout.is_empty());
            }
        }
    }

    #[test]
    fn cmd_missing_binary_is_quiet_and_foreign_exit_status_wins() {
        let folder = tempfile::tempdir().unwrap();
        let missing = folder.path().join("missing.exe");
        let script = format!("{} & exit /b 47", cmd_hook(missing.to_str().unwrap()));
        let output = Command::new(windows_system_directory().unwrap().join("cmd.exe"))
            .args(["/d", "/c"])
            .raw_arg(script)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(47));
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
    use std::os::windows::process::CommandExt;
}
