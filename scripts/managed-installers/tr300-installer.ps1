# TR-300 managed PowerShell installer.
#
# The release workflow renders the immutable tag/version placeholders, keeps the
# cargo-dist generated installer as tr300-dist-installer.ps1, and publishes this
# stable-name wrapper as tr300-installer.ps1. A deliberately launched fresh
# wrapper is authoritative: after cargo-dist installs and verifies the managed
# PowerShell channel, recognized native MSI/Inno products are uninstalled.

param (
    [switch]$NoModifyPath,
    [switch]$Help
)

$ErrorActionPreference = 'Stop'
$InformationPreference = 'Continue'
$Tr300Tag = '@TR300_TAG@'
$Tr300Version = '@TR300_VERSION@'
$Tr300ReleaseBase = "https://github.com/QubeTX/qube-machine-report/releases/download/$Tr300Tag"
$Tr300RecoveryUrl = 'https://github.com/QubeTX/qube-machine-report/releases/latest'

if ($Help) {
    Write-Information 'TR-300 managed PowerShell installer'
    Write-Information 'Installs the latest managed CLI channel and safely supersedes recognized TR-300 MSI/EXE installs.'
    return
}

function Get-Tr300MsiProducts {
    if (-not ('Tr300.ManagedInstaller.NativeMsi' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
namespace Tr300.ManagedInstaller {
    public static class NativeMsi {
        [DllImport("msi.dll", CharSet = CharSet.Unicode)]
        public static extern uint MsiEnumRelatedProducts(
            string upgradeCode,
            uint reserved,
            uint productIndex,
            StringBuilder productCode);
    }
}
'@
    }

    $families = @(
        [pscustomobject]@{
            UpgradeCode = '{5CD540A8-AD16-4B0F-8CE4-51FF641DE181}'; Channel = 'msi-global'; Elevated = $true
            Binary = (Join-Path $env:ProgramFiles 'tr300\bin\tr300.exe')
        },
        [pscustomobject]@{
            UpgradeCode = '{93F465CB-7F66-4930-A773-FDA017E8FD64}'; Channel = 'msi-corporate'; Elevated = $false
            Binary = (Join-Path $env:LOCALAPPDATA 'Programs\tr300\bin\tr300.exe')
        }
    )
    $products = @()
    foreach ($family in $families) {
        for ($index = 0; ; $index++) {
            $productCode = New-Object System.Text.StringBuilder 39
            $result = [Tr300.ManagedInstaller.NativeMsi]::MsiEnumRelatedProducts(
                $family.UpgradeCode,
                0,
                [uint32]$index,
                $productCode
            )
            if ($result -eq 259) { break }
            if ($result -ne 0) {
                throw "MsiEnumRelatedProducts failed for $($family.Channel) with code $result"
            }
            $products += [pscustomobject]@{
                Kind = 'msi'
                Channel = $family.Channel
                Elevated = $family.Elevated
                ProductCode = $productCode.ToString()
                Uninstaller = 'msiexec.exe'
                Binary = $family.Binary
            }
        }
    }
    return $products
}

function ConvertFrom-Tr300UninstallString([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
    if ($Value -match '^\s*"([^"]+)"') { return $Matches[1] }
    return ($Value.Trim() -split '\s+', 2)[0]
}

function Get-Tr300InnoProducts {
    $families = @(
        [pscustomobject]@{
            Key = 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{AB14223F-2693-4EC2-824F-BF53CC32D061}_is1'
            Channel = 'exe-global'; Elevated = $true; Root = (Join-Path $env:ProgramFiles 'tr300')
            DisplayName = 'tr300'; Publisher = 'Emmett S'
        },
        [pscustomobject]@{
            Key = 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\{AB14223F-2693-4EC2-824F-BF53CC32D061}_is1'
            Channel = 'exe-global'; Elevated = $true; Root = (Join-Path $env:ProgramFiles 'tr300')
            DisplayName = 'tr300'; Publisher = 'Emmett S'
        },
        [pscustomobject]@{
            Key = 'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{76A253EB-3A17-4730-9C54-5BE755A9BC4C}_is1'
            Channel = 'exe-corporate'; Elevated = $false; Root = (Join-Path $env:LOCALAPPDATA 'Programs\tr300')
            DisplayName = 'tr300 (Corporate Edition)'; Publisher = 'Emmett S'
        }
    )
    $products = @()
    $seen = @{}
    foreach ($family in $families) {
        if (-not (Test-Path -LiteralPath $family.Key)) { continue }
        $entry = Get-ItemProperty -LiteralPath $family.Key
        $uninstaller = ConvertFrom-Tr300UninstallString ([string]$entry.UninstallString)
        if (-not $uninstaller) {
            throw "recognized $($family.Channel) registration has no uninstaller"
        }
        if ([string]$entry.DisplayName -ne $family.DisplayName -or
            [string]$entry.Publisher -ne $family.Publisher) {
            throw "recognized $($family.Channel) registration has conflicting display/publisher evidence"
        }
        $full = [IO.Path]::GetFullPath($uninstaller)
        $root = [IO.Path]::GetFullPath($family.Root).TrimEnd('\') + '\'
        $leaf = [IO.Path]::GetFileName($full)
        if (-not $full.StartsWith($root, [StringComparison]::OrdinalIgnoreCase) -or
            $leaf -notmatch '^unins\d+\.exe$') {
            throw "recognized $($family.Channel) registration points outside its exact install root; refusing $full"
        }
        if ($entry.InstallLocation) {
            $registeredRoot = [IO.Path]::GetFullPath([string]$entry.InstallLocation).TrimEnd('\') + '\'
            if (-not $registeredRoot.Equals($root, [StringComparison]::OrdinalIgnoreCase)) {
                throw "recognized $($family.Channel) registration has a conflicting install root"
            }
        }
        if (-not $seen.ContainsKey($full.ToLowerInvariant())) {
            $seen[$full.ToLowerInvariant()] = $true
            $products += [pscustomobject]@{
                Kind = 'inno'
                Channel = $family.Channel
                Elevated = $family.Elevated
                ProductCode = $null
                Uninstaller = $full
                Binary = (Join-Path $family.Root 'bin\tr300.exe')
            }
        }
    }
    return $products
}

function Get-Tr300NativeProducts {
    $products = @()
    $products += @(Get-Tr300MsiProducts)
    $products += @(Get-Tr300InnoProducts)
    return $products
}

function Invoke-Tr300Process([string]$FilePath, [string[]]$Arguments, [bool]$Elevated) {
    $params = @{
        FilePath = $FilePath
        ArgumentList = $Arguments
        Wait = $true
        PassThru = $true
        WindowStyle = 'Hidden'
    }
    if ($Elevated) { $params.Verb = 'RunAs' }
    return Start-Process @params
}

function Remove-Tr300NativeProduct($Product) {
    Write-Information "Switching TR-300 ownership from $($Product.Channel) to powershell-installer..."
    if ($Product.Kind -eq 'msi') {
        $process = Invoke-Tr300Process 'msiexec.exe' @('/x', $Product.ProductCode, '/passive', '/norestart') $Product.Elevated
        if ($process.ExitCode -notin @(0, 1605, 1614, 1641, 3010)) {
            throw "$($Product.Channel) uninstall exited with Windows Installer code $($process.ExitCode)"
        }
    } else {
        $process = Invoke-Tr300Process $Product.Uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') $Product.Elevated
        if ($process.ExitCode -ne 0) {
            throw "$($Product.Channel) uninstall exited with code $($process.ExitCode)"
        }
    }
}

function Get-Tr300ReceiptPath {
    $root = if ($env:XDG_CONFIG_HOME) { $env:XDG_CONFIG_HOME } else { $env:LOCALAPPDATA }
    if (-not $root) { throw 'LOCALAPPDATA is unavailable; cannot verify the managed installer receipt' }
    return Join-Path $root 'tr300\tr300-receipt.json'
}

function Get-Tr300InstallPrefix {
    if ($env:TR300_INSTALL_DIR) { return [IO.Path]::GetFullPath($env:TR300_INSTALL_DIR) }
    if ($env:CARGO_DIST_FORCE_INSTALL_DIR) { return [IO.Path]::GetFullPath($env:CARGO_DIST_FORCE_INSTALL_DIR) }
    if ($env:CARGO_HOME) { return [IO.Path]::GetFullPath($env:CARGO_HOME) }
    if (-not $env:USERPROFILE) { throw 'USERPROFILE is unavailable; cannot resolve the managed install prefix' }
    return [IO.Path]::GetFullPath((Join-Path $env:USERPROFILE '.cargo'))
}

function Save-Tr300ManagedState([string]$BackupRoot) {
    $receiptPath = Get-Tr300ReceiptPath
    $priorPrefix = $null
    $receiptExisted = Test-Path -LiteralPath $receiptPath -PathType Leaf
    if ($receiptExisted) {
        $priorReceipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
        if ($priorReceipt.provider.source -ne 'cargo-dist' -or
            $priorReceipt.source.app_name -ne 'tr300' -or
            [string]::IsNullOrWhiteSpace([string]$priorReceipt.install_prefix)) {
            throw 'the existing TR-300 managed receipt is ambiguous; preserving it'
        }
        $priorPrefix = [IO.Path]::GetFullPath([string]$priorReceipt.install_prefix)
        Copy-Item -LiteralPath $receiptPath -Destination (Join-Path $BackupRoot 'receipt.json')
    }

    $binaryPaths = @((Join-Path (Get-Tr300InstallPrefix) 'bin\tr300.exe'))
    if ($priorPrefix) { $binaryPaths += (Join-Path $priorPrefix 'bin\tr300.exe') }
    $seen = @{}
    $binaries = @()
    foreach ($candidate in $binaryPaths) {
        $full = [IO.Path]::GetFullPath($candidate)
        $key = $full.ToLowerInvariant()
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $existed = Test-Path -LiteralPath $full -PathType Leaf
        $backup = $null
        if ($existed) {
            $backup = Join-Path $BackupRoot ("binary-$($binaries.Count).exe")
            Copy-Item -LiteralPath $full -Destination $backup
        }
        $binaries += [pscustomobject]@{ Path = $full; Existed = $existed; Backup = $backup }
    }

    return [pscustomobject]@{
        ReceiptPath = $receiptPath
        ReceiptExisted = $receiptExisted
        ReceiptBackup = (Join-Path $BackupRoot 'receipt.json')
        PriorPrefix = $priorPrefix
        Binaries = $binaries
    }
}

function Restore-Tr300ManagedState($State) {
    foreach ($binary in $State.Binaries) {
        if ($binary.Existed) {
            $null = New-Item -ItemType Directory -Path (Split-Path -Parent $binary.Path) -Force
            Copy-Item -LiteralPath $binary.Backup -Destination $binary.Path -Force
        } else {
            Remove-Item -LiteralPath $binary.Path -Force -ErrorAction SilentlyContinue
        }
    }
    if ($State.ReceiptExisted) {
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $State.ReceiptPath) -Force
        Copy-Item -LiteralPath $State.ReceiptBackup -Destination $State.ReceiptPath -Force
    } else {
        Remove-Item -LiteralPath $State.ReceiptPath -Force -ErrorAction SilentlyContinue
    }
}

function Assert-Tr300NoUnknownPathOwners($NativeProducts, $ManagedState) {
    $allowed = @{}
    foreach ($binary in $ManagedState.Binaries) {
        $allowed[[IO.Path]::GetFullPath($binary.Path).ToLowerInvariant()] = $true
    }
    foreach ($product in $NativeProducts) {
        $allowed[[IO.Path]::GetFullPath($product.Binary).ToLowerInvariant()] = $true
    }
    $commands = @(Get-Command tr300 -All -CommandType Application -ErrorAction SilentlyContinue)
    foreach ($command in $commands) {
        $source = if ($command.Source) { $command.Source } else { $command.Path }
        if (-not $source) { continue }
        $full = [IO.Path]::GetFullPath([string]$source)
        if (-not $allowed.ContainsKey($full.ToLowerInvariant())) {
            throw "an unrecognized portable/PATH copy is active at $full; preserving it instead of guessing ownership"
        }
    }
}

function Assert-Tr300ManagedPath([string]$Binary, [bool]$SkipPathChange) {
    if ($SkipPathChange) { return }
    $expected = [IO.Path]::GetFullPath((Split-Path -Parent $Binary)).TrimEnd('\')
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $matched = $false
    foreach ($entry in @(([string]$userPath -split ';'))) {
        $candidate = [Environment]::ExpandEnvironmentVariables($entry.Trim().Trim('"'))
        if ([string]::IsNullOrWhiteSpace($candidate)) { continue }
        try {
            $candidate = [IO.Path]::GetFullPath($candidate).TrimEnd('\')
        } catch {
            continue
        }
        if ($candidate.Equals($expected, [StringComparison]::OrdinalIgnoreCase)) {
            $matched = $true
            break
        }
    }
    if (-not $matched) {
        throw "managed installer did not persist its binary directory in user PATH: $expected"
    }
}

function Get-Tr300ManagedBinary {
    $receiptPath = Get-Tr300ReceiptPath
    if (-not (Test-Path -LiteralPath $receiptPath)) {
        throw "managed installer receipt is missing: $receiptPath"
    }
    $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    if ($receipt.provider.source -ne 'cargo-dist' -or
        $receipt.source.app_name -ne 'tr300' -or
        $receipt.version -ne $Tr300Version) {
        throw 'managed installer receipt does not identify the exact TR-300 release'
    }
    $actualPrefix = [IO.Path]::GetFullPath([string]$receipt.install_prefix)
    $expectedPrefix = Get-Tr300InstallPrefix
    if (-not $actualPrefix.Equals($expectedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "managed installer receipt recorded $actualPrefix, expected $expectedPrefix"
    }
    $binary = Join-Path $actualPrefix 'bin\tr300.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "managed TR-300 binary is missing: $binary"
    }
    $reported = (& $binary --version | Select-Object -First 1)
    if ($LASTEXITCODE -ne 0 -or $reported -ne "tr300 $Tr300Version") {
        throw "managed TR-300 binary did not report the expected version $Tr300Version"
    }
    return $binary
}

if ($env:TR300_MANAGED_INSTALLER_TEST_ONLY -eq '1') {
    return
}

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("tr300-managed-install-" + [guid]::NewGuid().ToString('N'))
$managedState = $null
$transactionStarted = $false
$committed = $false
$nativeRemovedCount = 0
try {
    $null = New-Item -ItemType Directory -Path $tempRoot -Force
    $native = @(Get-Tr300NativeProducts)
    $managedState = Save-Tr300ManagedState $tempRoot
    Assert-Tr300NoUnknownPathOwners $native $managedState
    $distInstaller = Join-Path $tempRoot 'tr300-dist-installer.ps1'
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    $headers = @{}
    $token = if ($env:TR300_GITHUB_TOKEN) {
        $env:TR300_GITHUB_TOKEN
    } elseif ($env:GITHUB_TOKEN) {
        $env:GITHUB_TOKEN
    } elseif ($env:GH_TOKEN) {
        $env:GH_TOKEN
    } else {
        $null
    }
    if ($token) { $headers.Authorization = "Bearer $token" }
    Invoke-WebRequest -UseBasicParsing -Headers $headers -Uri "$Tr300ReleaseBase/tr300-dist-installer.ps1" -OutFile $distInstaller

    $transactionStarted = $true
    $launcher = if ($PSVersionTable.PSEdition -eq 'Core') {
        Join-Path $PSHOME 'pwsh.exe'
    } else {
        Join-Path $PSHOME 'powershell.exe'
    }
    $childArgs = @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $distInstaller)
    if ($NoModifyPath) { $childArgs += '-NoModifyPath' }
    & $launcher @childArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo-dist installation exited with code $LASTEXITCODE"
    }

    $binary = Get-Tr300ManagedBinary
    Assert-Tr300ManagedPath $binary ([bool]$NoModifyPath)
    foreach ($product in $native) {
        Remove-Tr300NativeProduct $product
        $nativeRemovedCount++
    }
    $remaining = @(Get-Tr300NativeProducts)
    if ($remaining.Count -ne 0) {
        throw "native installer takeover is incomplete: $($remaining.Channel -join ', ') remains registered"
    }
    foreach ($product in $native) {
        if (Test-Path -LiteralPath $product.Binary -PathType Leaf) {
            throw "native installer takeover left its executable behind: $($product.Binary)"
        }
    }

    if ($native.Count -gt 0) {
        $markerKey = 'Registry::HKEY_CURRENT_USER\Software\TR300'
        foreach ($name in @('InstallSource', 'InstallSourceGlobal', 'InstallSourceCorporate')) {
            Remove-ItemProperty -LiteralPath $markerKey -Name $name -ErrorAction SilentlyContinue
        }
    }
    if ($managedState.PriorPrefix) {
        $priorBinary = Join-Path $managedState.PriorPrefix 'bin\tr300.exe'
        $sameBinary = [IO.Path]::GetFullPath($priorBinary).Equals(
            [IO.Path]::GetFullPath($binary),
            [StringComparison]::OrdinalIgnoreCase
        )
        if (-not $sameBinary -and (Test-Path -LiteralPath $priorBinary -PathType Leaf)) {
            Remove-Item -LiteralPath $priorBinary -Force -ErrorAction Stop
        }
    }
    $binary = Get-Tr300ManagedBinary
    Assert-Tr300ManagedPath $binary ([bool]$NoModifyPath)
    Assert-Tr300NoUnknownPathOwners @() $managedState
    $committed = $true
    Write-Information "TR-300 $Tr300Version is installed through the managed PowerShell channel: $binary"
} catch {
    $failure = $_.Exception.Message
    if ($transactionStarted -and -not $committed -and $managedState) {
        if ($nativeRemovedCount -eq 0) {
            try {
                Restore-Tr300ManagedState $managedState
            } catch {
                $failure += "; restoring the prior managed/Cargo path also failed: $($_.Exception.Message)"
            }
        } else {
            # Native uninstallers own their registrations and have no supported
            # generic rollback API. Once one commits, deleting the already-
            # verified new managed copy could strand the machine with no TR-300.
            # Keep it, report the partial native state, and let recovery rerun
            # this idempotent wrapper instead of counterfeiting atomicity.
            $failure += "; $nativeRemovedCount native uninstall(s) already committed, so the verified managed copy was retained for safe recovery"
        }
    }
    [Console]::Error.WriteLine("TR-300 managed install failed safely: $failure")
    [Console]::Error.WriteLine("Download a fresh installer: $Tr300RecoveryUrl")
    exit 1
} finally {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
