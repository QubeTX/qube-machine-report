$ErrorActionPreference = 'Stop'

$oldTestOnly = $env:TR300_MANAGED_INSTALLER_TEST_ONLY
$oldXdg = $env:XDG_CONFIG_HOME
$oldInstallDir = $env:TR300_INSTALL_DIR
$oldPath = $env:PATH
$oldLocation = Get-Location
$fixture = Join-Path ([IO.Path]::GetTempPath()) ("tr300-managed-powershell-test-" + [guid]::NewGuid().ToString('N'))
try {
    $env:TR300_MANAGED_INSTALLER_TEST_ONLY = '1'
    . (Join-Path $PSScriptRoot 'managed-installers\tr300-installer.ps1')

    # A planted current-directory/PATH executable must never be selected for
    # either elevated Global or user-scoped Corporate MSI removal.
    $plant = Join-Path $fixture 'search-path-plant'
    New-Item -ItemType Directory -Force -Path $plant | Out-Null
    $plantedMsiExec = Join-Path $plant 'msiexec.exe'
    Set-Content -LiteralPath $plantedMsiExec -Value 'planted fixture' -NoNewline
    $env:PATH = "$plant;$oldPath"
    Push-Location -LiteralPath $plant
    try {
        $trustedMsiExec = Get-Tr300TrustedMsiExecPath
    } finally {
        Pop-Location
    }
    $expectedMsiExec = [IO.Path]::GetFullPath((Join-Path ([Environment]::SystemDirectory) 'msiexec.exe'))
    if (-not $trustedMsiExec.Equals($expectedMsiExec, [StringComparison]::OrdinalIgnoreCase) -or
        $trustedMsiExec.Equals($plantedMsiExec, [StringComparison]::OrdinalIgnoreCase) -or
        -not [IO.Path]::IsPathRooted($trustedMsiExec)) {
        throw "trusted Windows Installer resolution selected an unsafe path: $trustedMsiExec"
    }

    # Pin the actual uninstall launch boundary as well as the resolver. A
    # hostile internal product value must not redirect the MSI subprocess.
    $script:capturedTr300Processes = @()
    function Start-Process {
        param(
            [string]$FilePath,
            [string[]]$ArgumentList,
            [switch]$Wait,
            [switch]$PassThru,
            [string]$WindowStyle,
            [string]$Verb,
            [string]$WorkingDirectory
        )
        $script:capturedTr300Processes += [pscustomobject]@{
            FilePath = $FilePath
            Arguments = @($ArgumentList)
            Elevated = ($Verb -eq 'RunAs')
            WorkingDirectory = $WorkingDirectory
            Wait = [bool]$Wait
            PassThru = [bool]$PassThru
            WindowStyle = $WindowStyle
        }
        return [pscustomobject]@{ ExitCode = 0 }
    }
    $globalInno = [IO.Path]::GetFullPath((Join-Path $env:ProgramFiles 'tr300\unins000.exe'))
    $globalInnoDirectory = [IO.Path]::GetDirectoryName($globalInno)
    Push-Location -LiteralPath $plant
    try {
        Remove-Tr300NativeProduct ([pscustomobject]@{
            Kind = 'msi'
            Channel = 'msi-global'
            Elevated = $true
            ProductCode = '{00000000-0000-0000-0000-000000000000}'
            Uninstaller = $plantedMsiExec
        })
        Remove-Tr300NativeProduct ([pscustomobject]@{
            Kind = 'msi'
            Channel = 'msi-corporate'
            Elevated = $false
            ProductCode = '{11111111-1111-1111-1111-111111111111}'
            Uninstaller = $plantedMsiExec
        })
        Remove-Tr300NativeProduct ([pscustomobject]@{
            Kind = 'inno'
            Channel = 'exe-global'
            Elevated = $true
            ProductCode = $null
            Uninstaller = $globalInno
        })
    } finally {
        Pop-Location
    }
    $globalLaunch, $corporateLaunch, $globalInnoLaunch = @($script:capturedTr300Processes)
    if (@($script:capturedTr300Processes).Count -ne 3 -or
        -not $globalLaunch.FilePath.Equals($expectedMsiExec, [StringComparison]::OrdinalIgnoreCase) -or
        -not $globalLaunch.WorkingDirectory.Equals([Environment]::SystemDirectory, [StringComparison]::OrdinalIgnoreCase) -or
        -not $globalLaunch.Elevated -or
        -not $globalLaunch.Wait -or
        -not $globalLaunch.PassThru -or
        $globalLaunch.WindowStyle -ne 'Hidden' -or
        ($globalLaunch.Arguments -join ' ') -ne '/x {00000000-0000-0000-0000-000000000000} /passive /norestart' -or
        -not $corporateLaunch.FilePath.Equals($expectedMsiExec, [StringComparison]::OrdinalIgnoreCase) -or
        -not $corporateLaunch.WorkingDirectory.Equals([Environment]::SystemDirectory, [StringComparison]::OrdinalIgnoreCase) -or
        $corporateLaunch.Elevated -or
        -not $corporateLaunch.Wait -or
        -not $corporateLaunch.PassThru -or
        $corporateLaunch.WindowStyle -ne 'Hidden' -or
        ($corporateLaunch.Arguments -join ' ') -ne '/x {11111111-1111-1111-1111-111111111111} /passive /norestart' -or
        -not $globalInnoLaunch.FilePath.Equals($globalInno, [StringComparison]::OrdinalIgnoreCase) -or
        -not $globalInnoLaunch.WorkingDirectory.Equals($globalInnoDirectory, [StringComparison]::OrdinalIgnoreCase) -or
        $globalInnoLaunch.WorkingDirectory.Equals($plant, [StringComparison]::OrdinalIgnoreCase) -or
        -not $globalInnoLaunch.Elevated -or
        -not $globalInnoLaunch.Wait -or
        -not $globalInnoLaunch.PassThru -or
        $globalInnoLaunch.WindowStyle -ne 'Hidden' -or
        ($globalInnoLaunch.Arguments -join ' ') -ne '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART') {
        throw 'managed Global/Corporate native removal did not preserve its trusted executable, working directory, elevation, and argument contract'
    }
    Remove-Item -LiteralPath Function:\Start-Process
    $env:PATH = $oldPath

    $env:XDG_CONFIG_HOME = Join-Path $fixture 'config'
    $env:TR300_INSTALL_DIR = Join-Path $fixture 'managed new'
    $oldPrefix = Join-Path $fixture 'managed old'
    $receiptPath = Get-Tr300ReceiptPath
    $oldBinary = Join-Path $oldPrefix 'bin\tr300.exe'
    $newBinary = Join-Path $env:TR300_INSTALL_DIR 'bin\tr300.exe'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $receiptPath), (Split-Path -Parent $oldBinary), (Split-Path -Parent $newBinary) | Out-Null
    Set-Content -LiteralPath $oldBinary -Value 'old-receipt-binary' -NoNewline
    Set-Content -LiteralPath $newBinary -Value 'old-raw-cargo-binary' -NoNewline
    [pscustomobject]@{
        install_prefix = $oldPrefix
        provider = [pscustomobject]@{ source = 'cargo-dist'; version = '0.31.0' }
        source = [pscustomobject]@{ app_name = 'tr300' }
        version = '4.1.3'
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $receiptPath

    $backup = Join-Path $fixture 'backup'
    New-Item -ItemType Directory -Path $backup | Out-Null
    $state = Save-Tr300ManagedState $backup
    $env:PATH = Split-Path -Parent $newBinary
    Assert-Tr300NoUnknownPathOwners @() $state
    $unknownDir = Join-Path $fixture 'portable'
    New-Item -ItemType Directory -Path $unknownDir | Out-Null
    Set-Content -LiteralPath (Join-Path $unknownDir 'tr300.exe') -Value 'unknown' -NoNewline
    $env:PATH = $unknownDir
    try {
        Assert-Tr300NoUnknownPathOwners @() $state
        throw 'unknown PATH owner was accepted'
    } catch {
        if ($_.Exception.Message -eq 'unknown PATH owner was accepted') { throw }
    }
    $env:PATH = $oldPath
    Set-Content -LiteralPath $newBinary -Value 'candidate' -NoNewline
    Remove-Item -LiteralPath $oldBinary -Force
    Set-Content -LiteralPath $receiptPath -Value 'candidate-receipt' -NoNewline
    Restore-Tr300ManagedState $state

    if ((Get-Content -LiteralPath $oldBinary -Raw) -ne 'old-receipt-binary') {
        throw 'receipt-owned binary was not restored'
    }
    if ((Get-Content -LiteralPath $newBinary -Raw) -ne 'old-raw-cargo-binary') {
        throw 'prior Cargo-path binary was not restored'
    }
    $restored = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    if ($restored.version -ne '4.1.3' -or $restored.install_prefix -ne $oldPrefix) {
        throw 'prior managed receipt was not restored'
    }

    Set-Content -LiteralPath $receiptPath -Value '{"provider":{"source":"other"},"source":{"app_name":"tr300"},"install_prefix":"C:\\tmp"}'
    try {
        $null = Save-Tr300ManagedState (Join-Path $fixture 'invalid-backup')
        throw 'invalid managed receipt was accepted'
    } catch {
        if ($_.Exception.Message -eq 'invalid managed receipt was accepted') { throw }
    }

    $rawDistInstaller = Join-Path $fixture 'tr300-dist-installer.ps1'
    Set-Content -LiteralPath $rawDistInstaller -Value '# exact cargo-dist fixture bytes' -NoNewline
    $Tr300DistInstallerSha256 = Get-Tr300Sha256 -Path $rawDistInstaller
    Assert-Tr300DistInstallerHash -Path $rawDistInstaller
    $Tr300DistInstallerSha256 = '0' * 64
    $mismatchExecutionMarker = Join-Path $fixture 'mismatch-executed'
    $mismatchMessage = $null
    try {
        Assert-Tr300DistInstallerHash -Path $rawDistInstaller
        Set-Content -LiteralPath $mismatchExecutionMarker -Value executed -NoNewline
    } catch {
        $mismatchMessage = $_.Exception.Message
    }
    if ($mismatchMessage -ne 'the downloaded cargo-dist installer checksum did not match this release' -or
        (Test-Path -LiteralPath $mismatchExecutionMarker)) {
        throw 'mismatched cargo-dist installer was not rejected before execution'
    }
    Write-Host 'managed PowerShell transaction fixtures: PASS'
} finally {
    $env:TR300_MANAGED_INSTALLER_TEST_ONLY = $oldTestOnly
    $env:XDG_CONFIG_HOME = $oldXdg
    $env:TR300_INSTALL_DIR = $oldInstallDir
    $env:PATH = $oldPath
    Set-Location -LiteralPath $oldLocation
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
}
