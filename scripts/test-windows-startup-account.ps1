# Invoked only by the hosted policy fixture under its new, disposable account.
$ErrorActionPreference = 'Stop'
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
if ($identity.Name -notmatch '\\TR3[0-9a-f]{12}$' -or
    [Security.Principal.WindowsPrincipal]::new($identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { exit 41 }
$bin = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'tr300\bin\tr300.exe'
$machineKey = 'Software\Microsoft\Active Setup\Installed Components\{3315638A-7A5F-4591-A9FE-79312ED7859B}'
$profilePath = $PROFILE.CurrentUserCurrentHost
Set-ExecutionPolicy -Scope CurrentUser Restricted -Force
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0 -or (Get-ExecutionPolicy -Scope CurrentUser) -ne 'Restricted' -or (Test-Path -LiteralPath $profilePath)) { exit 42 }

Set-ExecutionPolicy -Scope CurrentUser RemoteSigned -Force
[IO.Directory]::CreateDirectory((Split-Path -Parent $profilePath)) | Out-Null
$newline = [Environment]::NewLine
$legacy = @('# TR-300 Machine Report', 'if (-not $env:TR300_AUTORUN_RAN) { [Console]::WriteLine("legacy-full-report") }', '# End TR-300','') -join $newline
[IO.File]::WriteAllText($profilePath,$legacy)
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0) { exit 43 }
$text = [IO.File]::ReadAllText($profilePath)
if (-not $text.StartsWith('# TR-300 Machine Report (Global account)') -or -not $text.EndsWith($legacy)) { exit 44 }

$env:TR300_AUTORUN_RAN = $null
$ps = Join-Path ([Environment]::SystemDirectory) 'WindowsPowerShell\v1.0\powershell.exe'
$output = @(& $ps -NoLogo -Command "[Console]::WriteLine('no-legacy'); exit 23" 2>&1)
if ($LASTEXITCODE -ne 23 -or $output.Count -ne 1 -or $output[0] -cne 'no-legacy') { exit 45 }
$pref = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'tr300\startup-disabled'
[IO.Directory]::CreateDirectory((Split-Path -Parent $pref)) | Out-Null
[IO.File]::WriteAllText($pref,'disabled')
& $bin configure-startup-account
if ($LASTEXITCODE -ne 0) { exit 46 }
$output = @(& $ps -NoLogo -Command "[Console]::WriteLine('disabled'); exit 24" 2>&1)
if ($LASTEXITCODE -ne 24 -or $output.Count -ne 1 -or $output[0] -cne 'disabled') { exit 47 }

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
