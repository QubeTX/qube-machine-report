# Invoked only by the hosted policy fixture under its new, disposable account.
# The alternate-credential Windows PowerShell 5.1 child can inherit a
# PowerShell 7 module search path from the Actions runner. Import native
# cmdlets only from its own PSHOME, like the production policy probe.
$env:PSModulePath = [IO.Path]::Combine($PSHOME,'Modules')
$ErrorActionPreference = 'Stop'
$stage = 'checking the disposable account'
trap {
    [Console]::Error.WriteLine("Account fixture failed while ${stage} at line $($_.InvocationInfo.ScriptLineNumber): $($_.Exception.Message)")
    exit 51
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
if ($identity.Name -notmatch '\\TR3[0-9a-f]{12}$' -or
    [Security.Principal.WindowsPrincipal]::new($identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { exit 41 }
$bin = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'tr300\bin\tr300.exe'
$machineKey = 'Software\Microsoft\Active Setup\Installed Components\{3315638A-7A5F-4591-A9FE-79312ED7859B}'
# CreateProcessWithLogonW loads this account's hive but Start-Process
# inherits the runner's environment. Restore the token SID's own OS-registered
# profile before expanding its known folders, as Explorer would at logon.
$stage = 'establishing the standard account environment'
$sidProfile = 'HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\' + $identity.User.Value
$homePath = [Microsoft.Win32.Registry]::GetValue($sidProfile,'ProfileImagePath','')
if (-not [IO.Path]::IsPathRooted($homePath)) { throw 'Missing OS profile directory for the disposable token SID' }
$mixed = $env:USERPROFILE -ine $homePath
if ($mixed) {
    & $bin configure-startup-account
    if ($LASTEXITCODE -ne 0) { throw 'Global account bootstrap did not safely skip mixed account context' }
}
$env:USERPROFILE = $homePath
$env:HOMEDRIVE = [IO.Path]::GetPathRoot($homePath).TrimEnd('\')
$env:HOMEPATH = $homePath.Substring($env:HOMEDRIVE.Length)
if ($mixed) {
    # Start a fresh Windows PowerShell after fixing USERPROFILE. Shell APIs
    # and $PROFILE may already have cached the launching account's location.
    $launcher = [IO.Path]::Combine([Environment]::SystemDirectory,'WindowsPowerShell','v1.0','powershell.exe')
    $loader = '& ([scriptblock]::Create([IO.File]::ReadAllText($env:TR300_POLICY_FIXTURE)))'
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($loader))
    & $launcher -NoProfile -EncodedCommand $encoded
    exit $LASTEXITCODE
}
foreach ($folder in 'MyDocuments','ApplicationData','LocalApplicationData') {
    $path = [Environment]::GetFolderPath($folder,[Environment+SpecialFolderOption]::DoNotVerify)
    if (-not $path.StartsWith($homePath.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)) {
        throw "Disposable account $folder resolved outside the token SID's profile: $path"
    }
    if ($folder -eq 'ApplicationData') { $env:APPDATA = $path }
    if ($folder -eq 'LocalApplicationData') { $env:LOCALAPPDATA = $path }
}
$env:TEMP = [IO.Path]::Combine($env:LOCALAPPDATA,'Temp')
$env:TMP = $env:TEMP
[IO.Directory]::CreateDirectory($env:TEMP) | Out-Null
# Re-probe in a new host. This existing host initialized $PROFILE before its
# inherited environment was corrected.
$ps = [IO.Path]::Combine([Environment]::SystemDirectory,'WindowsPowerShell','v1.0','powershell.exe')
$profilePath = [string](& $ps -NoProfile -Command '[Console]::WriteLine([string]$PROFILE.CurrentUserCurrentHost)')
if (-not [IO.Path]::IsPathRooted($profilePath)) {
    $documents = [Environment]::GetFolderPath([Environment+SpecialFolder]::MyDocuments,[Environment+SpecialFolderOption]::DoNotVerify)
    if (-not [IO.Path]::IsPathRooted($documents)) { throw 'The disposable account has no registered Documents known folder' }
    $profilePath = [IO.Path]::Combine($documents,'WindowsPowerShell','Microsoft.PowerShell_profile.ps1')
}
$stage = 'testing explicit Restricted'
Microsoft.PowerShell.Security\Set-ExecutionPolicy -Scope CurrentUser Restricted -Force
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0 -or (Microsoft.PowerShell.Security\Get-ExecutionPolicy -Scope CurrentUser) -ne 'Restricted' -or (Test-Path -LiteralPath $profilePath)) { exit 42 }

$stage = 'verifying first-use Documents enrollment'
Microsoft.PowerShell.Security\Set-ExecutionPolicy -Scope CurrentUser RemoteSigned -Force
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0 -or -not [IO.File]::Exists($profilePath)) { throw "Global registration returned $LASTEXITCODE and the expected first-use profile was $profilePath (exists: $([IO.File]::Exists($profilePath)))" }
$ps = Join-Path ([Environment]::SystemDirectory) 'WindowsPowerShell\v1.0\powershell.exe'
$seen = @(& $ps -NoLogo -Command '[Console]::WriteLine($PROFILE.CurrentUserCurrentHost)' 2>&1)
if ($LASTEXITCODE -ne 0 -or $seen.Count -ne 1 -or $seen[0] -ine $profilePath) { throw "First-use PowerShell returned $LASTEXITCODE and discovered '$seen', expected '$profilePath'" }
& $bin remove-startup-account
if ($LASTEXITCODE -ne 0 -or [IO.File]::Exists($profilePath)) { exit 55 }
$stage = 'seeding legacy opt-in'
$newline = [Environment]::NewLine
$legacy = @('# TR-300 Machine Report', 'if (-not $env:TR300_AUTORUN_RAN) { [Console]::WriteLine("legacy-full-report") }', '# End TR-300','') -join $newline
[IO.File]::WriteAllText($profilePath,$legacy)
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0) { exit 43 }
$stage = 'verifying startup markers'
$text = [IO.File]::ReadAllText($profilePath)
if (-not $text.StartsWith('# TR-300 Machine Report (Global account)') -or -not $text.EndsWith($legacy)) { exit 44 }

$stage = 'running guarded child PowerShell'
$env:TR300_AUTORUN_RAN = $null
$ps = Join-Path ([Environment]::SystemDirectory) 'WindowsPowerShell\v1.0\powershell.exe'
$output = @(& $ps -NoLogo -Command "[Console]::WriteLine('no-legacy'); exit 23" 2>&1)
if ($LASTEXITCODE -ne 23 -or $output.Count -ne 1 -or $output[0] -cne 'no-legacy') { exit 45 }
$stage = 'testing the per-account opt-out'
$roaming = [Environment]::GetFolderPath([Environment+SpecialFolder]::ApplicationData,[Environment+SpecialFolderOption]::DoNotVerify)
if (-not [IO.Path]::IsPathRooted($roaming)) { throw 'The disposable account has no registered roaming application data folder' }
$pref = [IO.Path]::Combine($roaming,'tr300','startup-disabled')
[IO.Directory]::CreateDirectory((Split-Path -Parent $pref)) | Out-Null
[IO.File]::WriteAllText($pref,'disabled')
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0) { exit 46 }
$output = @(& $ps -NoLogo -Command "[Console]::WriteLine('disabled'); exit 24" 2>&1)
if ($LASTEXITCODE -ne 24 -or $output.Count -ne 1 -or $output[0] -cne 'disabled') { exit 47 }

$stage = 'testing cleanup and re-enrollment'
$stamp = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($machineKey)
$stamp.SetValue('Version','1,0,0,0')
$stamp.Dispose()
& $bin remove-startup-account
if ($LASTEXITCODE -ne 0 -or [IO.File]::ReadAllText($profilePath) -cne $legacy) { exit 48 }
if ([Microsoft.Win32.Registry]::GetValue("HKEY_CURRENT_USER\$machineKey",'Version','')) { exit 49 }
[IO.File]::Delete($pref)
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0 -or -not [IO.File]::ReadAllText($profilePath).StartsWith('# TR-300 Machine Report (Global account)')) { exit 50 }
exit 0
