$ErrorActionPreference = 'Stop'
$trusted = @(
    'S-1-5-18'        # SYSTEM
    'S-1-5-32-544'    # Administrators
    'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464' # TrustedInstaller
)
# Writable directory entries can replace otherwise protected children.
$write = [int64][Security.AccessControl.FileSystemRights]::Write -bor
         [int64][Security.AccessControl.FileSystemRights]::Delete -bor
         [int64][Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor
         [int64][Security.AccessControl.FileSystemRights]::ChangePermissions -bor
         [int64][Security.AccessControl.FileSystemRights]::TakeOwnership -bor 0x50000000
function Test-ProtectedAcl($acl, [int64]$mask, [string]$rights) {
    $sid = [Security.Principal.SecurityIdentifier]
    if ($trusted -notcontains $acl.GetOwner($sid).Value) { exit 1 }
    # A null DACL grants everyone access. An empty DACL grants no access.
    $descriptor = [Security.AccessControl.RawSecurityDescriptor]::new($acl.GetSecurityDescriptorBinaryForm(),0)
    if ($null -eq $descriptor.DiscretionaryAcl) { exit 1 }
    foreach ($rule in $acl.GetAccessRules($true,$true,$sid)) {
        if ($rule.AccessControlType -ne 'Allow') { continue }
        # CREATOR OWNER grants apply only to future children of the
        # administrator creating them. Other inherited write grants could
        # produce a user-writable profile under a protected directory.
        if (($rule.PropagationFlags -band [Security.AccessControl.PropagationFlags]::InheritOnly) -and $rule.IdentityReference.Value -eq 'S-1-3-0') { continue }
        if (($mask -band [int64]$rule.$rights) -and $trusted -notcontains $rule.IdentityReference.Value) { exit 1 }
    }
}
foreach ($path in $paths) {
    Test-ProtectedAcl (Microsoft.PowerShell.Security\Get-Acl -LiteralPath $path) $write 'FileSystemRights'
}
# The command key can be missing. Check its protected parent before allowing
# creation and check the existing key before modifying its AutoRun value.
$registry = [Microsoft.Win32.Registry]::LocalMachine
foreach ($path in @('SOFTWARE\Microsoft', 'SOFTWARE\Microsoft\Command Processor')) {
    $key = $registry.OpenSubKey($path)
    if ($null -eq $key) {
        if ($path -eq 'SOFTWARE\Microsoft') { exit 1 }
        continue
    }
    try {
        # SetValue, CreateSubKey, CreateLink, DELETE, WRITE_DAC, WRITE_OWNER,
        # GENERIC_WRITE and GENERIC_ALL.
        Test-ProtectedAcl $key.GetAccessControl() 0x500D0026 'RegistryRights'
    } finally { $key.Dispose() }
}
exit 0
