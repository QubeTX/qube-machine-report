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
foreach ($path in $paths) {
    $acl = Microsoft.PowerShell.Security\Get-Acl -LiteralPath $path
    $sid = [Security.Principal.SecurityIdentifier]
    if ($trusted -notcontains $acl.GetOwner($sid).Value) { exit 1 }
    # A null DACL grants everyone access. An empty DACL grants no access.
    $descriptor = [Security.AccessControl.RawSecurityDescriptor]::new($acl.GetSecurityDescriptorBinaryForm(),0)
    if ($null -eq $descriptor.DiscretionaryAcl) { exit 1 }
    foreach ($rule in $acl.GetAccessRules($true,$true,$sid)) {
        if ($rule.AccessControlType -ne 'Allow') { continue }
        if ($rule.PropagationFlags -band [Security.AccessControl.PropagationFlags]::InheritOnly) { continue }
        if (($write -band [int64]$rule.FileSystemRights) -and $trusted -notcontains $rule.IdentityReference.Value) { exit 1 }
    }
}
exit 0
