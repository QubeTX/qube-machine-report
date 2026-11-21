[CmdletBinding()]
param([Parameter(Mandatory)][bool]$Registered)

$ErrorActionPreference = 'Stop'
$systemDirectory = [Environment]::SystemDirectory
$programs = [Environment]::GetFolderPath('ProgramFiles')
$profiles = @((Join-Path $systemDirectory 'WindowsPowerShell\v1.0\profile.ps1'))
$hosts = @((Join-Path $systemDirectory 'WindowsPowerShell\v1.0\powershell.exe'))
$pwsh = Join-Path $programs 'PowerShell\7\pwsh.exe'
if (Test-Path -LiteralPath $pwsh -PathType Leaf) {
    $hosts += $pwsh
    $profiles += Join-Path (Split-Path -Parent $pwsh) 'profile.ps1'
}
foreach ($profilePath in $profiles) {
    $markers = if (Test-Path -LiteralPath $profilePath -PathType Leaf) {
        @(Select-String -LiteralPath $profilePath -Pattern '^# TR-300 Machine Report \(all users\)$').Count
    } else { 0 }
    if ($markers -ne [int]$Registered) { throw "Unexpected Global startup markers ($markers) in $profilePath" }
}
$reg = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('Software\Microsoft\Command Processor')
try {
    $autorun = if ($reg) { $reg.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) }
} finally { if ($reg) { $reg.Dispose() } }
$hook = '"' + (Join-Path $programs 'tr300\bin\tr300.exe') + '" --startup-hook'
$count = [regex]::Matches([string]$autorun,[regex]::Escape($hook)).Count
if ($count -ne [int]$Registered) { throw "Unexpected count of Global Command Prompt startup registrations ($count)" }

if ($Registered) {
    # Run through native profile/AutoRun discovery. These noninteractive
    # commands must produce exactly their payload and preserve its exit code.
    foreach ($shell in $hosts) {
        $output = @(& $shell -NoLogo -Command "Write-Output 'tr300-script-proof'; exit 37")
        if ($LASTEXITCODE -ne 37 -or $output.Count -ne 1 -or $output[0] -cne 'tr300-script-proof') {
            throw "Global startup interfered with scripted execution in $shell"
        }
    }
    $output = @(& (Join-Path $systemDirectory 'cmd.exe') /c 'echo tr300-script-proof&exit /b 37')
    if ($LASTEXITCODE -ne 37 -or $output.Count -ne 1 -or $output[0] -cne 'tr300-script-proof') {
        throw 'Global startup interfered with cmd.exe /c'
    }
}
