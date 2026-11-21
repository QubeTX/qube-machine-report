[CmdletBinding()]
param([Parameter(Mandatory)][bool]$Registered)

$ErrorActionPreference = 'Stop'
Write-Host "Checking Global shell registration (expected registered: $Registered)"
$systemDirectory = [Environment]::SystemDirectory
$programs = [Environment]::GetFolderPath('ProgramFiles')
$profiles = @((Join-Path $systemDirectory 'WindowsPowerShell\v1.0\profile.ps1'))
$hosts = @((Join-Path $systemDirectory 'WindowsPowerShell\v1.0\powershell.exe'))
$pwsh = Join-Path $programs 'PowerShell\7\pwsh.exe'
if (Test-Path -LiteralPath $pwsh -PathType Leaf) {
    $hosts += $pwsh
    $profiles += Join-Path (Split-Path -Parent $pwsh) 'profile.ps1'
}
for ($i = 0; $i -lt $profiles.Count; $i++) {
    $profilePath = $profiles[$i]
    $policy = @(& $hosts[$i] -NoProfile -Command "Get-ExecutionPolicy -Scope MachinePolicy; Get-ExecutionPolicy -Scope LocalMachine")
    if ($LASTEXITCODE -ne 0 -or $policy.Count -ne 2) { throw "Missing durable machine policy for $profilePath" }
    $effective = $policy | Where-Object { $_ -ne 'Undefined' } | Select-Object -First 1
    if (-not $effective) {
        $product = [Microsoft.Win32.Registry]::GetValue('HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\ProductOptions','ProductType','')
        $effective = if ($product -in 'ServerNT','LanmanNT') { 'RemoteSigned' } else { 'Restricted' }
    }
    $expectProfile = $Registered -and $effective -in 'RemoteSigned','Bypass','Unrestricted'

    $markers = if (Test-Path -LiteralPath $profilePath -PathType Leaf) {
        @(Select-String -LiteralPath $profilePath -Pattern '^# TR-300 Machine Report \(all users\)$').Count
    } else { 0 }
    if ($markers -ne [int]$expectProfile) { throw "Unexpected Global startup markers ($markers) in $profilePath" }
}
$reg = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('Software\Microsoft\Command Processor')
try {
    $autorun = if ($reg) { $reg.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) }
} finally { if ($reg) { $reg.Dispose() } }
$hook = '"' + (Join-Path $programs 'tr300\bin\tr300.exe') + '" --startup-hook'
$count = [regex]::Matches([string]$autorun,[regex]::Escape($hook)).Count
if ($count -ne [int]$Registered) { throw "Unexpected count of Global Command Prompt startup registrations ($count)" }

$active = [Microsoft.Win32.Registry]::GetValue('HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Active Setup\Installed Components\{3315638A-7A5F-4591-A9FE-79312ED7859B}', 'StubPath', '')
$expectedStub = '"' + (Join-Path $programs 'tr300\bin\tr300.exe') + '" configure-startup-account'
if (($Registered -and $active -cne $expectedStub) -or (-not $Registered -and $active)) {
    throw 'Unexpected Global account bootstrap registration'
}

if ($Registered) {
    # Run through native profile/AutoRun discovery. These noninteractive
    # commands must produce exactly their payload and preserve its exit code.
    foreach ($shell in $hosts) {
        $output = @(& $shell -NoLogo -Command "Write-Output 'tr300-script-proof'; exit 37" 2>&1)
        if ($LASTEXITCODE -ne 37 -or $output.Count -ne 1 -or $output[0] -cne 'tr300-script-proof') {
            throw "Global startup interfered with scripted execution in $shell"
        }
    }
    $output = @(& (Join-Path $systemDirectory 'cmd.exe') /c 'echo tr300-script-proof&exit /b 37')
    if ($LASTEXITCODE -ne 37 -or $output.Count -ne 1 -or $output[0] -cne 'tr300-script-proof') {
        throw 'Global startup interfered with cmd.exe /c'
    }
}

Write-Host "Verified Global shell registration (registered: $Registered)"

# The diagnostic children deliberately exit 37. Mark a successful assertion
# script successful for the calling workflow's native-exit propagation.
exit 0
