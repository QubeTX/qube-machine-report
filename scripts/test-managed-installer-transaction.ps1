$ErrorActionPreference = 'Stop'

$oldTestOnly = $env:TR300_MANAGED_INSTALLER_TEST_ONLY
$oldXdg = $env:XDG_CONFIG_HOME
$oldInstallDir = $env:TR300_INSTALL_DIR
$oldDistInstallerPath = $env:TR300_DIST_INSTALLER_PATH
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
    $oldReport = Join-Path $oldPrefix 'bin\report.exe'
    $newBinary = Join-Path $env:TR300_INSTALL_DIR 'bin\tr300.exe'
    $newReport = Join-Path $env:TR300_INSTALL_DIR 'bin\report.exe'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $receiptPath), (Split-Path -Parent $oldBinary), (Split-Path -Parent $newBinary) | Out-Null
    Set-Content -LiteralPath $oldBinary -Value 'old-receipt-binary' -NoNewline
    Set-Content -LiteralPath $oldReport -Value 'old-receipt-report' -NoNewline
    Set-Content -LiteralPath $newBinary -Value 'old-raw-cargo-binary' -NoNewline
    Set-Content -LiteralPath $newReport -Value 'old-raw-cargo-report' -NoNewline
    [pscustomobject]@{
        install_prefix = $oldPrefix
        provider = [pscustomobject]@{ source = 'cargo-dist'; version = '0.31.0' }
        source = [pscustomobject]@{ app_name = 'tr300' }
        version = '4.1.3'
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $receiptPath

    $backup = Join-Path $fixture 'backup'
    New-Item -ItemType Directory -Path $backup | Out-Null
    try {
        $null = Save-Tr300ManagedState $backup
        throw 'foreign report destination was accepted'
    } catch {
        if ($_.Exception.Message -eq 'foreign report destination was accepted') { throw }
        if ($_.Exception.Message -notlike '*unowned report command*') { throw }
    }
    if ((Get-Content -LiteralPath $newReport -Raw) -ne 'old-raw-cargo-report') {
        throw 'foreign report destination was changed'
    }
    Remove-Item -LiteralPath $newReport
    $legacyState = Save-Tr300ManagedState $backup
    if ($legacyState.PriorReportOwned -or @($legacyState.Binaries.Path) -contains $oldReport) {
        throw 'legacy single-binary receipt claimed a foreign report sibling'
    }
    $legacyReceiptText = Get-Content -LiteralPath $receiptPath -Raw
    $savedIntendedPrefix = $env:TR300_INSTALL_DIR
    foreach ($malformedInventory in @(
        '"report.exe"',
        '{"0":"tr300.exe","1":"report.exe"}',
        '[["tr300.exe","report.exe"]]',
        '[{"binaries":["tr300.exe","report.exe"]}]',
        '["report.exe"]',
        '["tr300.exe"]',
        'null'
    )) {
        $malformedReceipt = $legacyReceiptText.TrimEnd()
        $malformedReceipt = $malformedReceipt.Substring(0, $malformedReceipt.Length - 1) +
            ',"binaries":' + $malformedInventory + '}'
        Set-Content -LiteralPath $receiptPath -Value $malformedReceipt
        $malformedState = Save-Tr300ManagedState $backup
        if ($malformedState.PriorReportOwned -or @($malformedState.Binaries.Path) -contains $oldReport) {
            throw "malformed receipt inventory authorized old report deletion: $malformedInventory"
        }
        $env:TR300_INSTALL_DIR = $oldPrefix
        try {
            $null = Save-Tr300ManagedState $backup
            throw 'malformed receipt inventory authorized report overwrite'
        } catch {
            if ($_.Exception.Message -notlike '*unowned report command*') { throw }
        } finally {
            $env:TR300_INSTALL_DIR = $savedIntendedPrefix
        }
        if ((Get-Content -LiteralPath $oldReport -Raw) -ne 'old-receipt-report' -or
            (Get-Content -LiteralPath $oldBinary -Raw) -ne 'old-receipt-binary') {
            throw 'malformed receipt fixture modified existing payload'
        }
    }
    Set-Content -LiteralPath $receiptPath -Value $legacyReceiptText
    $ownedReceipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    $ownedReceipt | Add-Member -NotePropertyName binaries -NotePropertyValue @('tr300.exe', 'report.exe')
    $ownedReceipt | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $receiptPath
    $state = Save-Tr300ManagedState $backup
    if (-not $state.PriorReportOwned -or @($state.Binaries.Path) -notcontains $oldReport) {
        throw 'paired receipt inventory did not authorize its owned report'
    }
    $env:TR300_INSTALL_DIR = $oldPrefix
    try {
        $samePrefixBackup = Join-Path $fixture 'same-prefix-backup'
        New-Item -ItemType Directory -Path $samePrefixBackup | Out-Null
        $ownedSamePrefixState = Save-Tr300ManagedState $samePrefixBackup
        if (-not $ownedSamePrefixState.PriorReportOwned) {
            throw 'paired receipt inventory did not authorize same-prefix replacement'
        }
    } finally {
        $env:TR300_INSTALL_DIR = $savedIntendedPrefix
    }
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
    Set-Content -LiteralPath $newReport -Value 'candidate-report' -NoNewline
    Remove-Item -LiteralPath $oldBinary -Force
    Remove-Item -LiteralPath $oldReport -Force
    Set-Content -LiteralPath $receiptPath -Value 'candidate-receipt' -NoNewline
    Restore-Tr300ManagedState $state

    if ((Get-Content -LiteralPath $oldBinary -Raw) -ne 'old-receipt-binary') {
        throw 'receipt-owned binary was not restored'
    }
    if ((Get-Content -LiteralPath $newBinary -Raw) -ne 'old-raw-cargo-binary') {
        throw 'prior Cargo-path binary was not restored'
    }
    if ((Get-Content -LiteralPath $oldReport -Raw) -ne 'old-receipt-report') {
        throw 'receipt-owned report command was not restored'
    }
    if (Test-Path -LiteralPath $newReport) {
        throw 'candidate report command was not removed during rollback'
    }
    $restored = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    if ($restored.version -ne '4.1.3' -or $restored.install_prefix -ne $oldPrefix) {
        throw 'prior managed receipt was not restored'
    }

    $savedCargoHome = $env:CARGO_HOME
    $savedInstallDirectory = $env:TR300_INSTALL_DIR
    $savedXdg = $env:XDG_CONFIG_HOME
    try {
        $env:CARGO_HOME = Join-Path $fixture 'raw cargo'
        $env:TR300_INSTALL_DIR = $env:CARGO_HOME
        $env:XDG_CONFIG_HOME = Join-Path $fixture 'raw config'
        New-Item -ItemType Directory -Force -Path (Join-Path $env:CARGO_HOME 'bin') | Out-Null
        $cargoInventory = Join-Path $env:CARGO_HOME '.crates2.json'
        foreach ($invalid in @(
            '{"installs":{"foreign 1.0.0 (registry)":{"bins":["tr300.exe","report.exe"]}}}',
            '{"installs":{"tr300 4.4.0 (registry)":{"bins":{"0":"tr300.exe","1":"report.exe"}}}}',
            '{"installs":{"tr300 4.4.0 (registry)":{"bins":["tr300.exe"],"nested":{"bins":["report.exe"]}}}}',
            '{"installs":{"tr300 4.4.0 (registry)":{"bins":["tr300.exe","report.exe"]}}'
        )) {
            Set-Content -LiteralPath $cargoInventory -Value $invalid
            if (Test-Tr300CargoOwnsReport $env:CARGO_HOME) { throw 'invalid Cargo inventory accepted' }
        }
        Set-Content -LiteralPath $cargoInventory -Value '{"installs":{"tr300 4.4.0 (registry+https://github.com/rust-lang/crates.io-index)":{"bins":["tr300.exe","report.exe"]}}}'
        if (-not (Test-Tr300CargoOwnsReport $env:CARGO_HOME)) { throw 'valid Cargo inventory rejected' }
        if (Test-Tr300CargoOwnsReport (Join-Path $fixture 'foreign prefix')) { throw 'foreign Cargo prefix accepted' }
        Set-Content -LiteralPath (Join-Path $env:CARGO_HOME 'bin\report.exe') -Value 'must never execute' -NoNewline
        $cargoBackup = Join-Path $fixture 'raw backup'
        New-Item -ItemType Directory -Path $cargoBackup | Out-Null
        $cargoState = Save-Tr300ManagedState $cargoBackup
        if ($cargoState.ReceiptExisted) { throw 'Cargo conversion invented a managed receipt' }
    } finally {
        $env:CARGO_HOME = $savedCargoHome
        $env:TR300_INSTALL_DIR = $savedInstallDirectory
        $env:XDG_CONFIG_HOME = $savedXdg
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

    $localSourceDirectory = Join-Path $fixture 'local source'
    $localStagingDirectory = Join-Path $fixture 'local staging'
    New-Item -ItemType Directory -Path $localSourceDirectory, $localStagingDirectory | Out-Null
    $localDistInstaller = Join-Path $localSourceDirectory 'tr300-dist-installer.ps1'
    Copy-Item -LiteralPath $rawDistInstaller -Destination $localDistInstaller
    $stagedDistInstaller = Join-Path $localStagingDirectory 'tr300-dist-installer.ps1'
    $script:tr300UnexpectedDownload = $false
    function Invoke-WebRequest {
        $script:tr300UnexpectedDownload = $true
        throw 'local override unexpectedly attempted a network download'
    }
    try {
        $env:TR300_DIST_INSTALLER_PATH = $localDistInstaller
        Copy-Tr300DistInstallerToPrivateStaging -Destination $stagedDistInstaller
    } finally {
        Remove-Item -LiteralPath Function:\Invoke-WebRequest
    }
    if ($script:tr300UnexpectedDownload -or
        -not (Test-Path -LiteralPath $stagedDistInstaller -PathType Leaf) -or
        (Get-Tr300Sha256 -Path $stagedDistInstaller) -cne $Tr300DistInstallerSha256) {
        throw 'valid local cargo-dist installer was not copied and rebound in private staging'
    }
    Assert-Tr300DistInstallerHash -Path $stagedDistInstaller

    $malformedLocalPaths = @('   ', 'relative-installer.ps1', $localSourceDirectory)
    for ($index = 0; $index -lt $malformedLocalPaths.Count; $index++) {
        $env:TR300_DIST_INSTALLER_PATH = $malformedLocalPaths[$index]
        $rejectedDestination = Join-Path $localStagingDirectory "rejected-$index.ps1"
        $rejected = $false
        try {
            Copy-Tr300DistInstallerToPrivateStaging -Destination $rejectedDestination
        } catch {
            $rejected = $true
        }
        if (-not $rejected -or (Test-Path -LiteralPath $rejectedDestination)) {
            throw 'malformed local cargo-dist installer path was accepted'
        }
    }

    $env:TR300_DIST_INSTALLER_PATH = $localDistInstaller
    $localMismatchInstaller = Join-Path $localStagingDirectory 'local-mismatch.ps1'
    Copy-Tr300DistInstallerToPrivateStaging -Destination $localMismatchInstaller
    $Tr300DistInstallerSha256 = '0' * 64
    $mismatchExecutionMarker = Join-Path $fixture 'mismatch-executed'
    $mismatchMessage = $null
    try {
        Assert-Tr300DistInstallerHash -Path $localMismatchInstaller
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
    $env:TR300_DIST_INSTALLER_PATH = $oldDistInstallerPath
    $env:PATH = $oldPath
    Set-Location -LiteralPath $oldLocation
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
}
