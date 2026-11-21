use super::{profile_error, MARKER_END, MARKER_START};
use crate::error::Result;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) enum ProfileSyntax {
    Posix,
    Fish,
    PowerShell,
}

pub(super) fn profile_candidates(home: &Path) -> Vec<(PathBuf, ProfileSyntax, bool)> {
    let config = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    let zdir = env::var_os("ZDOTDIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.to_owned());
    let mut paths = vec![
        (home.join(".bashrc"), ProfileSyntax::Posix, true),
        (zdir.join(".zshrc"), ProfileSyntax::Posix, true),
        (
            config.join("fish/conf.d/tr300.fish"),
            ProfileSyntax::Fish,
            true,
        ),
        (
            config.join("powershell/profile.ps1"),
            ProfileSyntax::PowerShell,
            true,
        ),
    ];
    if zdir != home {
        paths.push((home.join(".zshrc"), ProfileSyntax::Posix, false));
    }
    let login = [".bash_profile", ".bash_login", ".profile"];
    let selected = login
        .iter()
        .find(|name| home.join(name).is_file())
        .unwrap_or(&".profile");
    for name in login {
        paths.push((home.join(name), ProfileSyntax::Posix, name == *selected));
    }
    paths
}

impl ProfileSyntax {
    pub(super) fn block(self, executable: &Path) -> Result<String> {
        self.render(executable, true)
    }

    fn render(self, executable: &Path, with_path: bool) -> Result<String> {
        if !executable.is_absolute() {
            return Err(profile_error("startup executable must be absolute"));
        }
        let quote = |path: &Path| -> Result<String> {
            let value = path
                .to_str()
                .filter(|s| !s.contains(['\n', '\r']))
                .ok_or_else(|| profile_error("unsupported shell profile executable path"))?;
            let escaped = match self {
                Self::Posix => value.replace('\'', "'\\''"),
                Self::Fish => value.replace('\\', "\\\\").replace('\'', "\\'"),
                Self::PowerShell => value.replace('\'', "''"),
            };
            Ok(format!("'{escaped}'"))
        };
        let parent = quote(
            executable
                .parent()
                .ok_or_else(|| profile_error("missing executable directory"))?,
        )?;
        let binary = quote(executable)?;
        let body = match self {
            Self::Posix => {
                r#"case "$-" in *i*)
    if [ -t 0 ] && [ -t 1 ] && [ -x @BIN@ ]; then
@PATH@
        if _tr300_tty=$(/usr/bin/tty 2>/dev/null) && [ "${TR300_AUTORUN_TTY-}" != "$_tr300_tty" ]; then
            if @BIN@ --startup-hook; then export TR300_AUTORUN_TTY="$_tr300_tty"; fi
        fi
        unset _tr300_tty
    fi
    ;;
esac"#
            }
            Self::Fish => {
                r#"if status is-interactive; and test -t 0; and test -t 1; and test -x @BIN@
@PATH@
    set -l tr300_tty (/usr/bin/tty 2>/dev/null)
    if test $status -eq 0; and test "$TR300_AUTORUN_TTY" != "$tr300_tty"
        if @BIN@ --startup-hook
            set -gx TR300_AUTORUN_TTY $tr300_tty
        end
    end
end"#
            }
            Self::PowerShell => {
                r#"& {
    if ([Console]::IsInputRedirected -or [Console]::IsOutputRedirected -or
        -not (Test-Path -LiteralPath @BIN@ -PathType Leaf)) { return }
    $cli = [Environment]::GetCommandLineArgs()
    for ($i = 1; $i -lt $cli.Length; $i++) {
        switch -Regex ($cli[$i]) {
            '^--?$' { return }
            '^-(?:login|l|nologo|nol|noexit|noe|noprofile|nop|sta|mta)$' { continue }
            '^-(?:workingdirectory|wd|configurationname|conf|settingsfile|custompipename|executionpolicy|ep|windowstyle|w)$' { $i++; continue }
            default { return }
        }
    }
@PATH@
    $savedExit = Get-Variable LASTEXITCODE -Scope Global -ErrorAction SilentlyContinue
    $priorExit = if ($null -ne $savedExit) { $savedExit.Value }
    try {
        $ttyName = & /usr/bin/tty 2>$null
        if ($LASTEXITCODE -eq 0 -and $env:TR300_AUTORUN_TTY -cne $ttyName) {
            & @BIN@ --startup-hook
            if ($LASTEXITCODE -eq 0) { $env:TR300_AUTORUN_TTY = $ttyName }
        }
    } finally {
        if ($null -ne $savedExit) { $global:LASTEXITCODE = $priorExit }
        else { Remove-Variable LASTEXITCODE -Scope Global -ErrorAction SilentlyContinue }
    }
}"#
            }
        };
        let path_setup = if with_path {
            match self {
                Self::Posix => r#"        case ":${PATH-}:" in *:@DIR@:*) ;; *) PATH=@DIR@${PATH:+:"$PATH"}; export PATH ;; esac"#,
                Self::Fish => "    if not contains -- @DIR@ $PATH\n        set -gx PATH @DIR@ $PATH\n    end",
                Self::PowerShell => "    if (@DIR@ -cnotin ($env:PATH -split ':')) { $env:PATH = @DIR@ + ':' + $env:PATH }",
            }
        } else {
            ""
        };
        let body = body.replace("@PATH@", path_setup);
        // Substitute in a single pass so literal placeholder text in a supplied
        // path cannot be interpreted as another substitution.
        let mut rendered = String::new();
        for (i, piece) in body.split("@BIN@").enumerate() {
            if i != 0 {
                rendered.push_str(&binary);
            }
            rendered.push_str(&piece.replace("@DIR@", &parent));
        }
        Ok(format!("{MARKER_START}\n{rendered}\n{MARKER_END}"))
    }
}
#[cfg(target_os = "macos")]
mod machine {
    use super::{profile_error, ProfileSyntax, Result};
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::Path;
    use std::process::Command;

    const START: &str = "# TR-300 system startup";
    const END: &str = "# End TR-300 system startup";
    const BINARY: &str = "/Library/PrivilegedHelperTools/com.qubetx.tr300.startup";

    fn block(shell: ProfileSyntax) -> Result<String> {
        Ok(shell
            .render(Path::new(BINARY), false)?
            .replace(super::MARKER_START, START)
            .replace(super::MARKER_END, END))
    }

    // A root process must never follow links or write through a directory
    // another account can rename. Homebrew prefixes are commonly user-owned.
    fn protected(path: &Path) -> Result<bool> {
        for parent in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
            let metadata = match fs::symlink_metadata(parent) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(error) => return Err(error.into()),
            };
            if metadata.file_type().is_symlink()
                || metadata.uid() != 0
                || metadata.mode() & 0o022 != 0
            {
                return Ok(false);
            }
            let listing = Command::new("/bin/ls")
                .env_clear()
                .env("LC_ALL", "C")
                .arg("-lde")
                .arg(parent)
                .output()?;
            if !listing.status.success() {
                return Ok(false);
            }
            let listing = String::from_utf8_lossy(&listing.stdout);
            if listing
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(" allow "))
                .any(|(_, rights)| {
                    rights.split([',', ' ']).any(|right| {
                        right.starts_with("write")
                            || matches!(
                                right,
                                "add_file"
                                    | "add_subdirectory"
                                    | "delete"
                                    | "delete_child"
                                    | "append"
                                    | "chown"
                            )
                    })
                })
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Homebrew's designated configuration directory normally belongs to the
    /// installing user. Make the dedicated Fish fragment with that account's
    /// privileges, including when the PKG is first installed over SSH.
    fn install_fish_fragment(config: &Path, enable: bool) -> Result<bool> {
        use std::io::Write;
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};

        let metadata = match fs::symlink_metadata(config) {
            Ok(metadata) if metadata.is_dir() => metadata,
            Ok(_) => {
                return Err(profile_error(format!(
                    "invalid Fish directory {}",
                    config.display()
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if metadata.uid() == 0 && !protected(config)? {
            return Err(profile_error(format!(
                "unsafe parent directory for {}",
                config.display()
            )));
        }
        for parent in config.ancestors() {
            let ancestor = fs::symlink_metadata(parent)?;
            if !ancestor.is_dir() || (ancestor.uid() != 0 && ancestor.uid() != metadata.uid()) {
                return Err(profile_error(format!(
                    "ambiguous ownership for {}",
                    config.display()
                )));
            }
        }
        let owner = uzers::get_user_by_uid(metadata.uid())
            .ok_or_else(|| profile_error("unknown Fish configuration owner"))?;
        // Compare against the exact generated fragment before replacement or
        // removal. ln publishes without overwriting a concurrent creator.
        const SCRIPT: &str = r#"set -eu
umask 022
cd "$1"
if [ ! -d conf.d ] && [ "$2" = remove ]; then exit 0; fi
if [ ! -d conf.d ]; then mkdir conf.d; fi
[ ! -L conf.d ]
cd conf.d
dest=tr300-machine-startup.fish
temp=$(mktemp .tr300-startup.XXXXXXXX)
trap 'rm -f "$temp"' EXIT
cat > "$temp"
chmod 644 "$temp"
if [ -e "$dest" ] || [ -L "$dest" ]; then
    [ ! -L "$dest" ] && [ -f "$dest" ] && cmp -s "$temp" "$dest" || exit 1
    if [ "$2" = remove ]; then rm -f "$dest"; exit 10; fi
elif [ "$2" = install ]; then
    ln "$temp" "$dest"
    exit 10
fi"#;
        if metadata.uid() == 0
            && config.join("conf.d").exists()
            && !protected(&config.join("conf.d"))?
        {
            return Err(profile_error("unsafe global Fish fragment directory"));
        }
        let mut child = Command::new("/bin/sh")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .args(["-c", SCRIPT, "tr300-fish"])
            .arg(config)
            .arg(if enable { "install" } else { "remove" })
            .uid(metadata.uid())
            .gid(owner.primary_group_id())
            .current_dir("/")
            .stdin(Stdio::piped())
            .spawn()?;
        let block = block(ProfileSyntax::Fish)?;
        let write = writeln!(child.stdin.take().expect("piped stdin"), "{block}");
        let status = child.wait()?;
        if !enable && status.success() {
            return Ok(false);
        }
        write?;
        if status.code() == Some(10) {
            return Ok(true);
        }
        if !status.success() {
            return Err(profile_error(format!(
                "unable to reconcile TR-300's Fish fragment at {}",
                config.display()
            )));
        }
        Ok(false)
    }

    fn local_accounts() -> Result<Vec<(u32, u32, std::path::PathBuf)>> {
        use uzers::os::unix::UserExt;
        let result = Command::new("/usr/bin/dscl")
            .env_clear()
            .args([".", "-list", "/Users", "UniqueID"])
            .output()?;
        if !result.status.success() {
            return Err(profile_error(
                "cannot enumerate local accounts for Bash startup",
            ));
        }
        let mut accounts = Vec::new();
        for line in String::from_utf8_lossy(&result.stdout).lines() {
            let Some(id) = line
                .split_ascii_whitespace()
                .last()
                .and_then(|id| id.parse::<i64>().ok())
            else {
                return Err(profile_error("invalid local account UID"));
            };
            if !(501..=u32::MAX as i64).contains(&id) {
                continue;
            }
            let uid = id as u32;
            let user = uzers::get_user_by_uid(uid)
                .ok_or_else(|| profile_error("local account lookup failed"))?;
            let home = user.home_dir();
            if !home.is_absolute() || !home.is_dir() {
                continue;
            }
            accounts.push((uid, user.primary_group_id(), home.to_owned()));
        }
        Ok(accounts)
    }

    fn change_account_as(uid: u32, gid: u32, home: &Path, enable: bool) -> Result<bool> {
        use std::os::unix::process::CommandExt;
        let command = if enable {
            "configure-startup-machine"
        } else {
            "remove-startup-machine"
        };
        let result = Command::new(BINARY)
            .env_clear()
            .env("HOME", home)
            .env("TR300_ACCOUNT_ACK", "1")
            .env("PATH", "/usr/bin:/bin")
            .arg(command)
            .current_dir("/")
            .uid(uid)
            .gid(gid)
            .output()?;
        if !result.status.success() {
            return Err(profile_error(format!(
                "could not configure Bash for UID {uid}: {}",
                String::from_utf8_lossy(&result.stderr)
            )));
        }
        Ok(result.stdout == b"TR300_ACCOUNT_CHANGED\n")
    }

    /// Apple's Bash only reads /etc/bashrc on login via /etc/profile. Run as
    /// the account itself for its non-login .bashrc, including LaunchAgent
    /// enrollment for users created after the package was installed.
    fn change_own_bash_profile(enable: bool) -> Result<bool> {
        let home = dirs::home_dir().ok_or_else(|| profile_error("cannot determine Bash home"))?;
        if enable && crate::install::startup_disabled()? {
            return Ok(false);
        }
        let path = home.join(".bashrc");
        let (path, before) = match fs::symlink_metadata(&path) {
            Ok(_) => {
                let (resolved, _) = super::super::resolve_profile_target(&path)?;
                let snapshot =
                    super::super::read_profile_snapshot(&path, &resolved, "Bash profile")?;
                (resolved, Some(snapshot.contents))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (path, None),
            Err(error) => return Err(error.into()),
        };
        let before_text = before.as_deref().unwrap_or("");
        let marker = "# TR-300 package Bash startup";
        let end = "# End TR-300 package Bash startup";
        let block = block(ProfileSyntax::Posix)?
            .replace(START, marker)
            .replace(END, end);
        let mut after = before_text.to_owned();
        let owned = format!("{block}\n");
        if after.contains(marker) || after.contains(end) {
            if !after.contains(&owned) {
                return Err(profile_error("modified package Bash startup block"));
            }
            if enable {
                return Ok(false);
            }
            after = after.replace(&owned, "");
        } else if enable {
            if !after.is_empty() && !after.ends_with('\n') {
                after.push('\n');
            }
            after.push_str(&owned);
        } else {
            return Ok(false);
        }
        super::super::super::atomic_write(&path, &after)?;
        Ok(true)
    }

    pub(in crate::install::unix) fn change(enable: bool) -> Result<()> {
        if unsafe { libc::geteuid() } != 0 {
            let changed = change_own_bash_profile(enable)?;
            if changed && std::env::var_os("TR300_ACCOUNT_ACK").is_some() {
                println!("TR300_ACCOUNT_CHANGED");
            }
            return Ok(());
        }
        if !protected(Path::new(BINARY))? {
            return Err(profile_error(
                "the machine startup runtime is not protected",
            ));
        }
        let mut changes = Vec::new();
        for (name, shell) in [
            ("/private/etc/profile", ProfileSyntax::Posix),
            ("/private/etc/bashrc", ProfileSyntax::Posix),
            ("/private/etc/zshrc", ProfileSyntax::Posix),
            ("/private/etc/fish/config.fish", ProfileSyntax::Fish),
            (
                "/usr/local/microsoft/powershell/7/profile.ps1",
                ProfileSyntax::PowerShell,
            ),
            (
                "/usr/local/microsoft/powershell/7-preview/profile.ps1",
                ProfileSyntax::PowerShell,
            ),
        ] {
            let path = Path::new(name);
            let parent = path.parent().unwrap();
            if !protected(parent)? {
                if parent.exists() {
                    eprintln!(
                        "TR-300: skipped unprotected shell configuration directory {}",
                        parent.display()
                    );
                }
                continue;
            }
            let before = match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_file() && protected(path)? => {
                    Some(fs::read_to_string(path)?)
                }
                Ok(_) => {
                    return Err(profile_error(format!(
                        "unsafe machine shell profile {name}"
                    )))
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            let original = before.as_deref().unwrap_or("");
            if original.contains("# SIG # Begin signature block") {
                return Err(profile_error(format!(
                    "cannot edit signed shell profile {name}"
                )));
            }
            let mut after = String::new();
            let mut inside = false;
            for line in original.split_inclusive('\n') {
                match line.trim_end_matches(['\r', '\n']) {
                    START if !inside => inside = true,
                    END if inside => inside = false,
                    START | END => {
                        return Err(profile_error(format!("invalid startup markers in {name}")))
                    }
                    _ if !inside => after.push_str(line),
                    _ => {}
                }
            }
            if inside {
                return Err(profile_error(format!(
                    "unterminated startup marker in {name}"
                )));
            }
            if enable {
                if !after.is_empty() && !after.ends_with('\n') {
                    after.push('\n');
                }
                let mut block = block(shell)?;
                if matches!(name, "/private/etc/profile" | "/private/etc/zshrc") {
                    let enroll = format!(
                        r#"case "$-" in *i*)
    if [ -t 0 ] && [ -t 1 ] && [ -x '{BINARY}' ] && [ "${{EUID:-$(/usr/bin/id -u)}}" -ge 501 ] &&
       ! /usr/bin/grep -Fqx '# TR-300 package Bash startup' "$HOME/.bashrc" 2>/dev/null; then
        '{BINARY}' configure-startup-machine >/dev/null 2>&1 || :
    fi
    ;;
esac"#
                    );
                    block = block.replacen(START, &format!("{START}\n{enroll}"), 1);
                }
                after.push_str(&block);
                after.push('\n');
            }
            if after != original {
                changes.push((path, before, after));
            }
        }
        let persist = |path: &Path, contents: Option<&str>| -> Result<()> {
            let Some(contents) = contents else {
                fs::remove_file(path)?;
                return Ok(());
            };
            let parent = path.parent().expect("absolute machine profile");
            let mut file = tempfile::NamedTempFile::new_in(parent)?;
            if path.exists() {
                // macOS cp -p preserves ACLs, flags, ownership, and extended
                // attributes. Keep this metadata through replacement.
                if !Command::new("/bin/cp")
                    .args(["-p"])
                    .arg(path)
                    .arg(file.path())
                    .status()?
                    .success()
                {
                    return Err(profile_error(format!(
                        "cannot preserve metadata for {}",
                        path.display()
                    )));
                }
                file.as_file().set_len(0)?;
            } else {
                file.as_file()
                    .set_permissions(fs::Permissions::from_mode(0o644))?;
            }
            file.write_all(contents.as_bytes())?;
            file.as_file().sync_all()?;
            file.persist(path).map_err(|error| error.error)?;
            Ok(())
        };
        let mut profiles_changed = 0;
        let mut accounts_changed = Vec::new();
        let mut fish_changed = Vec::new();
        let result = (|| {
            for (path, before, after) in &changes {
                if fs::read_to_string(path).ok().as_ref() != before.as_ref() {
                    return Err(profile_error(format!(
                        "machine shell profile changed: {}",
                        path.display()
                    )));
                }
                persist(path, Some(after))?;
                profiles_changed += 1;
            }
            for config in [
                "/usr/local/etc/fish",
                "/opt/homebrew/etc/fish",
                "/opt/local/etc/fish",
            ] {
                if install_fish_fragment(Path::new(config), enable)? {
                    fish_changed.push(config);
                }
            }
            for (uid, gid, home) in local_accounts()? {
                if change_account_as(uid, gid, &home, enable)? {
                    accounts_changed.push((uid, gid, home));
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            for (uid, gid, home) in accounts_changed.into_iter().rev() {
                if let Err(restore) = change_account_as(uid, gid, &home, !enable) {
                    eprintln!("TR-300: could not restore Bash startup for UID {uid}: {restore}");
                }
            }
            for config in fish_changed.into_iter().rev() {
                if let Err(restore) = install_fish_fragment(Path::new(config), !enable) {
                    eprintln!("TR-300: could not restore {config}: {restore}");
                }
            }
            for (path, before, after) in changes[..profiles_changed].iter().rev() {
                if fs::read_to_string(path).ok().as_ref() == Some(after) {
                    if let Err(restore) = persist(path, before.as_deref()) {
                        eprintln!("TR-300: could not restore {}: {restore}", path.display());
                    }
                }
            }
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub(super) use machine::change as change_machine_startup;
