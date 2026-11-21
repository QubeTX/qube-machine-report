use crate::trusted_process::powershell_command_for;
use std::path::PathBuf;

pub(super) const HOSTS: [&str; 2] = ["powershell", "pwsh"];

#[derive(Debug)]
struct Policy {
    machine_group: String,
    user_group: String,
    user: String,
    machine: String,
    default: String,
    profile: PathBuf,
}

fn permits(policy: &str) -> bool {
    matches!(policy, "RemoteSigned" | "Unrestricted" | "Bypass")
}

impl Policy {
    fn effective(&self, machine_only: bool) -> &str {
        let scopes = [
            Some(self.machine_group.as_str()),
            (!machine_only).then_some(self.user_group.as_str()),
            (!machine_only).then_some(self.user.as_str()),
            Some(self.machine.as_str()),
            Some(self.default.as_str()),
        ];
        scopes
            .into_iter()
            .flatten()
            .find(|s| *s != "Undefined")
            .unwrap_or("Restricted")
    }

    fn can_enroll(&self) -> bool {
        [
            &self.machine_group,
            &self.user_group,
            &self.user,
            &self.machine,
        ]
        .into_iter()
        .all(|s| s == "Undefined")
            && self.default == "Restricted"
    }
}

const READ: &str = r#"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
foreach ($scope in 'MachinePolicy','UserPolicy','CurrentUser','LocalMachine') {
    [Console]::WriteLine([string](Microsoft.PowerShell.Security\Get-ExecutionPolicy -Scope $scope))
}
$product = [Microsoft.Win32.Registry]::GetValue('HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\ProductOptions','ProductType','')
$default = switch ($product) { 'WinNT' { 'Restricted' } 'ServerNT' { 'RemoteSigned' } 'LanmanNT' { 'RemoteSigned' } default { 'Unknown' } }
[Console]::WriteLine($default)
$profilePath = [string]$PROFILE.CurrentUserCurrentHost
if (-not [IO.Path]::IsPathRooted($profilePath)) {
    # GetFolderPath without DoNotVerify returns an empty string for a newly
    # created account until its Documents folder actually exists. Respect
    # the OS destination, including OneDrive and network redirects.
    $documents = [Environment]::GetFolderPath([Environment+SpecialFolder]::MyDocuments,[Environment+SpecialFolderOption]::DoNotVerify)
    if ([IO.Path]::IsPathRooted($documents)) {
        $directory = if ($PSVersionTable.PSEdition -eq 'Core') { 'PowerShell' } else { 'WindowsPowerShell' }
        $profilePath = [IO.Path]::Combine($documents,$directory,'Microsoft.PowerShell_profile.ps1')
    }
}
[Console]::WriteLine($profilePath)
"#;

fn read(host: &str) -> Option<Policy> {
    let output = powershell_command_for(host, READ).ok()?.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&output.stdout).ok()?;
    let fields: Vec<_> = text.lines().collect();
    let [machine_group, user_group, user, machine, default, profile] = fields.as_slice() else {
        return None;
    };
    let profile = PathBuf::from(profile);
    Some(Policy {
        machine_group: machine_group.to_string(),
        user_group: user_group.to_string(),
        user: user.to_string(),
        machine: machine.to_string(),
        default: default.to_string(),
        profile,
    })
}

pub(super) fn machine_enabled(host: &str) -> bool {
    read(host).is_some_and(|p| permits(p.effective(true)))
}

// Set only the undefined personal default. Recheck every durable scope inside
// the setter as an explicitly configured restriction must survive a race.
const ENROLL: &str = r#"
$ErrorActionPreference='Stop'
foreach ($scope in 'MachinePolicy','UserPolicy','CurrentUser','LocalMachine') {
    if ((Microsoft.PowerShell.Security\Get-ExecutionPolicy -Scope $scope) -ne 'Undefined') { exit 0 }
}
Microsoft.PowerShell.Security\Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force
"#;

pub(super) struct AccountPlan {
    pub profiles: Vec<(PathBuf, bool)>,
    enroll: Vec<(&'static str, usize)>,
}

pub(super) fn account_plan(
    enable: bool,
    only_without_shared: bool,
    may_enroll: bool,
) -> AccountPlan {
    let mut plan = AccountPlan {
        profiles: Vec::new(),
        enroll: Vec::new(),
    };
    for host in HOSTS {
        let Some(policy) = read(host) else { continue };
        if !policy.profile.is_absolute() {
            continue;
        }
        let requested = enable && !(only_without_shared && permits(policy.effective(true)));
        let adjust = requested && may_enroll && policy.can_enroll();
        if adjust {
            plan.enroll.push((host, plan.profiles.len()));
        }
        plan.profiles.push((
            policy.profile.clone(),
            requested && (adjust || permits(policy.effective(false))),
        ));
    }
    plan
}

impl AccountPlan {
    /// Profile and registry edits, including signature checks, must succeed
    /// first. If a concurrent policy change disallows the host, clean our block.
    pub fn apply(
        mut self,
        mut write: impl FnMut(&[(PathBuf, bool)]) -> crate::Result<()>,
    ) -> crate::Result<()> {
        write(&self.profiles)?;
        let mut changed = false;
        for (host, index) in self.enroll {
            if let Ok(mut command) = powershell_command_for(host, ENROLL) {
                let _ = command.output();
            }
            if !read(host).is_some_and(|p| permits(p.effective(false))) {
                self.profiles[index].1 = false;
                changed = true;
            }
        }
        if changed {
            write(&self.profiles)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(
        machine_group: &str,
        user_group: &str,
        user: &str,
        machine: &str,
        default: &str,
    ) -> Policy {
        Policy {
            machine_group: machine_group.into(),
            user_group: user_group.into(),
            user: user.into(),
            machine: machine.into(),
            default: default.into(),
            profile: "test.ps1".into(),
        }
    }

    #[test]
    fn machine_baseline_excludes_the_installing_account_and_process() {
        let client = policy(
            "Undefined",
            "Undefined",
            "Bypass",
            "Undefined",
            "Restricted",
        );
        assert!(!permits(client.effective(true)));
        assert!(permits(client.effective(false)));
        let server = policy(
            "Undefined",
            "AllSigned",
            "Undefined",
            "Undefined",
            "RemoteSigned",
        );
        assert!(permits(server.effective(true)));
        assert!(!permits(server.effective(false)));
        let managed = policy(
            "Restricted",
            "Undefined",
            "Bypass",
            "RemoteSigned",
            "RemoteSigned",
        );
        assert!(!permits(managed.effective(true)));
        assert!(!permits(managed.effective(false)));
        let local_machine = policy(
            "Undefined",
            "Undefined",
            "Undefined",
            "AllSigned",
            "RemoteSigned",
        );
        assert!(!permits(local_machine.effective(true)));
    }

    #[test]
    fn only_the_implicit_personal_client_default_can_be_adjusted() {
        let unset = "Undefined";
        assert!(policy(unset, unset, unset, unset, "Restricted").can_enroll());
        assert!(!policy(unset, unset, unset, unset, "RemoteSigned").can_enroll());
        for explicit in ["Restricted", "AllSigned", "Unknown", "RemoteSigned"] {
            for config in [
                policy(explicit, unset, unset, unset, "Restricted"),
                policy(unset, explicit, unset, unset, "Restricted"),
                policy(unset, unset, explicit, unset, "Restricted"),
                policy(unset, unset, unset, explicit, "Restricted"),
            ] {
                assert!(!config.can_enroll());
            }
        }
    }
}
