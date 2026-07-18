$ErrorActionPreference = 'Stop'

$oldTestOnly = $env:TR300_MANAGED_INSTALLER_TEST_ONLY
$oldXdg = $env:XDG_CONFIG_HOME
$oldInstallDir = $env:TR300_INSTALL_DIR
$oldPath = $env:PATH
$fixture = Join-Path ([IO.Path]::GetTempPath()) ("tr300-managed-powershell-test-" + [guid]::NewGuid().ToString('N'))
try {
    $env:TR300_MANAGED_INSTALLER_TEST_ONLY = '1'
    . (Join-Path $PSScriptRoot 'managed-installers\tr300-installer.ps1')

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
    Write-Host 'managed PowerShell transaction fixtures: PASS'
} finally {
    $env:TR300_MANAGED_INSTALLER_TEST_ONLY = $oldTestOnly
    $env:XDG_CONFIG_HOME = $oldXdg
    $env:TR300_INSTALL_DIR = $oldInstallDir
    $env:PATH = $oldPath
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
}
