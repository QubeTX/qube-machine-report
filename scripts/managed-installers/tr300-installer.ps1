# TR-300 managed PowerShell installer.
#
# The release workflow renders the immutable tag, version, and exact raw
# cargo-dist installer SHA-256 placeholders, keeps the generated installer as
# tr300-dist-installer.ps1, and publishes this stable-name wrapper as
# tr300-installer.ps1. A deliberately launched fresh wrapper is authoritative:
# after cargo-dist installs and verifies the managed PowerShell channel,
# recognized native MSI/Inno products are uninstalled.

param (
    [switch]$NoModifyPath,
    [switch]$Help
)

$Tr300OriginalPSModulePath = $env:PSModulePath
$env:PSModulePath = [IO.Path]::Combine($PSHOME, 'Modules')
$ErrorActionPreference = 'Stop'
$InformationPreference = 'Continue'
$Tr300Tag = '@TR300_TAG@'
$Tr300Version = '@TR300_VERSION@'
$Tr300DistInstallerSha256 = '@TR300_DIST_INSTALLER_SHA256@'
$Tr300WindowsArchiveSha256 = '@TR300_WINDOWS_ARCHIVE_SHA256@'
$Tr300ReleaseBase = "https://github.com/QubeTX/qube-machine-report/releases/download/$Tr300Tag"
$Tr300RecoveryUrl = 'https://github.com/QubeTX/qube-machine-report/releases/latest'
$Tr300EntryPath = [string]$env:PATH
$Tr300EntryDirectory = (Get-Location).ProviderPath
$Tr300InvocationPath = if ($null -ne $env:TR300_INVOCATION_PATH) { [string]$env:TR300_INVOCATION_PATH } else { [Environment]::ExpandEnvironmentVariables($Tr300EntryPath) }
$Tr300InvocationDirectory = if ($env:TR300_INVOCATION_CWD) { $env:TR300_INVOCATION_CWD } else { $Tr300EntryDirectory }

if ($Help) {
    Write-Information 'TR-300 managed PowerShell installer'
    Write-Information 'Installs the latest managed CLI channel and safely supersedes recognized TR-300 MSI/EXE installs.'
    $env:PSModulePath = $Tr300OriginalPSModulePath
    return
}

function Get-Tr300TrustedMsiExecPath {
    # Environment.SystemDirectory is backed by the Windows system-directory
    # API. Do not replace this with Get-Command, a bare executable name, PATH,
    # or the current directory: Global MSI removal may run with a UAC token.
    $systemDirectory = [Environment]::SystemDirectory
    if ([string]::IsNullOrWhiteSpace($systemDirectory)) {
        throw 'Windows did not provide its trusted system directory'
    }
    $systemDirectory = [IO.Path]::GetFullPath($systemDirectory).TrimEnd('\')
    if (-not [IO.Path]::IsPathRooted($systemDirectory)) {
        throw 'Windows returned a non-absolute system directory'
    }
    $msiexec = [IO.Path]::GetFullPath((Join-Path $systemDirectory 'msiexec.exe'))
    $parent = [IO.Path]::GetDirectoryName($msiexec).TrimEnd('\')
    if (-not $parent.Equals($systemDirectory, [StringComparison]::OrdinalIgnoreCase) -or
        -not (Test-Path -LiteralPath $msiexec -PathType Leaf)) {
        throw "the trusted Windows Installer executable is unavailable: $msiexec"
    }
    return $msiexec
}

function Get-Tr300Sha256 {
    param([Parameter(Mandatory = $true)][string]$Path)

    $stream = [IO.File]::Open(
        $Path,
        [IO.FileMode]::Open,
        [IO.FileAccess]::Read,
        [IO.FileShare]::Read
    )
    try {
        $algorithm = [Security.Cryptography.SHA256]::Create()
        try {
            return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
        } finally {
            $algorithm.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Assert-Tr300DistInstallerHash {
    param([Parameter(Mandatory = $true)][string]$Path)

    if ($Tr300DistInstallerSha256 -cnotmatch '\A[0-9a-f]{64}\z') {
        throw 'the managed installer has an invalid embedded cargo-dist checksum'
    }
    $actual = Get-Tr300Sha256 -Path $Path
    if ($actual -cne $Tr300DistInstallerSha256) {
        throw 'the downloaded cargo-dist installer checksum did not match this release'
    }
}

function New-Tr300GuardedDistInstaller([string]$Source, [string]$Destination) {
    if ($Tr300WindowsArchiveSha256 -cnotmatch '\A[0-9a-f]{64}\z') {
        throw 'the managed installer has an invalid embedded Windows archive checksum'
    }
    $stream = [IO.File]::Open($Source, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try {
        $algorithm = [Security.Cryptography.SHA256]::Create()
        try {
            $actual = ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
        } finally { $algorithm.Dispose() }
        if ($actual -cne $Tr300DistInstallerSha256) {
            throw 'the downloaded cargo-dist installer checksum did not match this release'
        }
        $stream.Position = 0
        $reader = New-Object IO.StreamReader($stream)
        try { $sourceText = $reader.ReadToEnd() } finally { $reader.Dispose() }
    } finally { $stream.Dispose() }
    $downloadAnchor = '  $wc.downloadFile($url, $dir_path)'
    $extractedAnchor = '  # Let the next step know what to copy'
    $startupAnchor = '$app_name = ''tr300'''
    foreach ($anchor in @($startupAnchor, $downloadAnchor, $extractedAnchor)) {
        if ([regex]::Matches($sourceText, ('(?m)^' + [regex]::Escape($anchor) + '\r?$')).Count -ne 1) {
            throw 'the pinned cargo-dist Windows archive guard anchors changed'
        }
    }
    if ($sourceText.IndexOf($downloadAnchor) -ge $sourceText.IndexOf($extractedAnchor)) {
        throw 'the pinned cargo-dist Windows archive guard anchors are out of order'
    }
    $sourceText = $sourceText.Replace($startupAnchor, ('$env:PSModulePath = [IO.Path]::Combine($PSHOME, ''Modules'')' + "`n" + $startupAnchor))
    $guard = @'
  $tr300ArchiveGuard = [IO.File]::Open($dir_path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
  try {
    $tr300ArchiveAlgorithm = [Security.Cryptography.SHA256]::Create()
    try {
      $tr300ArchiveHash = ([BitConverter]::ToString($tr300ArchiveAlgorithm.ComputeHash($tr300ArchiveGuard))).Replace('-', '').ToLowerInvariant()
    } finally { $tr300ArchiveAlgorithm.Dispose() }
    if ($tr300ArchiveHash -cne '@ARCHIVE_SHA256@') {
      throw 'the downloaded Windows archive checksum did not match this release'
    }
'@
    $guard = $guard.Replace('@ARCHIVE_SHA256@', $Tr300WindowsArchiveSha256)
    $closeGuard = @'
  } finally { $tr300ArchiveGuard.Dispose() }
'@
    $sourceText = $sourceText.Replace($downloadAnchor, ($downloadAnchor + "`n" + $guard))
    $sourceText = $sourceText.Replace($extractedAnchor, ($closeGuard + "`n" + $extractedAnchor))
    [IO.File]::WriteAllText($Destination, $sourceText, (New-Object Text.UTF8Encoding($false)))
}

function ConvertTo-Tr300ProcessArgument([string]$Value) {
    # Windows argv quoting; no shell interprets the resulting argument string.
    return '"' + ([regex]::Replace(([regex]::Replace($Value, '(\\*)"', '$1$1\"')), '(\\+)$', '$1$1')) + '"'
}

function Invoke-Tr300DistInstaller([string]$Script, [string]$TempRoot, [bool]$SkipPathChange) {
    $launcher = Join-Path $PSHOME $(if ($PSVersionTable.PSEdition -eq 'Core') { 'pwsh.exe' } else { 'powershell.exe' })
    if (-not [IO.Path]::IsPathRooted($launcher) -or -not [IO.File]::Exists($launcher)) {
        throw 'the current trusted PowerShell host is unavailable'
    }
    $system = [Environment]::SystemDirectory
    $windows = [IO.Directory]::GetParent($system).FullName
    $programs = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $launcher
    $info.UseShellExecute = $false
    $info.WorkingDirectory = $PSHOME
    $childArgs = @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $Script)
    if ($SkipPathChange) { $childArgs += '-NoModifyPath' }
    $info.Arguments = ($childArgs | ForEach-Object { ConvertTo-Tr300ProcessArgument $_ }) -join ' '
    $info.EnvironmentVariables['PATH'] = (@($system, (Join-Path $system 'Wbem'), $windows, $PSHOME) -join ';')
    $info.EnvironmentVariables['PSModulePath'] = Join-Path $PSHOME 'Modules'
    $info.EnvironmentVariables['SystemRoot'] = $windows
    $info.EnvironmentVariables['windir'] = $windows
    $info.EnvironmentVariables['COMSPEC'] = Join-Path $system 'cmd.exe'
    $info.EnvironmentVariables['ProgramFiles'] = $programs
    $info.EnvironmentVariables['ProgramW6432'] = $programs
    $info.EnvironmentVariables['TEMP'] = $TempRoot
    $info.EnvironmentVariables['TMP'] = $TempRoot
    $guardedScript = [IO.File]::Open($Script, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try {
        $process = [Diagnostics.Process]::Start($info)
        try { $process.WaitForExit(); return $process.ExitCode } finally { $process.Dispose() }
    } finally { $guardedScript.Dispose() }
}

function New-Tr300PrivateStaging {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    $elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    $base = if ($elevated) {
        [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
    } else {
        [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
    }
    if ([string]::IsNullOrWhiteSpace($base) -or -not [IO.Path]::IsPathRooted($base)) {
        throw 'Windows did not provide an absolute private staging base'
    }
    $path = Join-Path $base ('tr300-managed-install-' + [guid]::NewGuid().ToString('N'))
    if ([IO.Directory]::Exists($path) -or [IO.File]::Exists($path)) {
        throw 'the private staging path already exists'
    }
    $security = New-Object Security.AccessControl.DirectorySecurity
    $security.SetAccessRuleProtection($true, $false)
    if ($elevated) {
        $security.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
    } else {
        $security.SetOwner($identity.User)
    }
    $owners = @((New-Object Security.Principal.SecurityIdentifier('S-1-5-18')), (New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
    if (-not $elevated) { $owners += $identity.User }
    foreach ($sid in $owners) {
        $rule = New-Object Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
        $security.AddAccessRule($rule)
    }
    if ($PSVersionTable.PSEdition -eq 'Core') {
        [IO.FileSystemAclExtensions]::Create((New-Object IO.DirectoryInfo($path)), $security)
    } else {
        $null = [IO.Directory]::CreateDirectory($path, $security)
    }
    return $path
}

function Copy-Tr300DistInstallerToPrivateStaging {
    param([Parameter(Mandatory = $true)][string]$Destination)

    $localInstaller = [Environment]::GetEnvironmentVariable('TR300_DIST_INSTALLER_PATH', 'Process')
    if ($null -ne $localInstaller) {
        if ([string]::IsNullOrWhiteSpace($localInstaller) -or
            $localInstaller -cnotmatch '\A[A-Za-z]:[\\/]' -or
            $localInstaller.Substring(2).Contains(':')) {
            throw 'TR300_DIST_INSTALLER_PATH must be an absolute local file path'
        }
        $localInstaller = [IO.Path]::GetFullPath($localInstaller)
        $source = Get-Item -LiteralPath $localInstaller -Force -ErrorAction Stop
        if (-not ($source -is [IO.FileInfo]) -or
            (($source.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
            throw 'TR300_DIST_INSTALLER_PATH must name a regular file, not a reparse point'
        }
        Copy-Item -LiteralPath $source.FullName -Destination $Destination -ErrorAction Stop
        $staged = Get-Item -LiteralPath $Destination -Force -ErrorAction Stop
        if (-not ($staged -is [IO.FileInfo]) -or
            (($staged.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
            throw 'the staged local cargo-dist installer is not a regular file'
        }
        return
    }

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
    Invoke-WebRequest -UseBasicParsing -Headers $headers -Uri "$Tr300ReleaseBase/tr300-dist-installer.ps1" -OutFile $Destination
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
        [DllImport("msi.dll", CharSet = CharSet.Unicode)]
        public static extern int MsiGetComponentPath(
            string productCode, string componentCode,
            StringBuilder path, ref uint length);
    }
}
'@
    }

    $families = @(
        [pscustomobject]@{
            UpgradeCode = '{5CD540A8-AD16-4B0F-8CE4-51FF641DE181}'; Channel = 'msi-global'; Elevated = $true
            Binary = (Join-Path $env:ProgramFiles 'tr300\bin\tr300.exe')
            Report = (Join-Path $env:ProgramFiles 'tr300\bin\report.exe')
            ReportComponent = '{8281A044-4A65-476C-890B-EF7A012E5F25}'
        },
        [pscustomobject]@{
            UpgradeCode = '{93F465CB-7F66-4930-A773-FDA017E8FD64}'; Channel = 'msi-corporate'; Elevated = $false
            Binary = (Join-Path $env:LOCALAPPDATA 'Programs\tr300\bin\tr300.exe')
            Report = (Join-Path $env:LOCALAPPDATA 'Programs\tr300\bin\report.exe')
            ReportComponent = '{2EB04231-F655-42C0-863E-D5ED46E6C41B}'
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
            $componentPath = New-Object System.Text.StringBuilder 32768
            [uint32]$componentLength = $componentPath.Capacity
            $componentState = [Tr300.ManagedInstaller.NativeMsi]::MsiGetComponentPath(
                $productCode.ToString(), $family.ReportComponent, $componentPath, [ref]$componentLength)
            $reportOwned = $componentState -eq 3 -and
                $componentPath.ToString().Equals($family.Report, [StringComparison]::OrdinalIgnoreCase)
            $products += [pscustomobject]@{
                Kind = 'msi'
                Channel = $family.Channel
                Elevated = $family.Elevated
                ProductCode = $productCode.ToString()
                Uninstaller = Get-Tr300TrustedMsiExecPath
                Binary = $family.Binary
                Report = $family.Report
                ReportOwned = $reportOwned
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
            $installedVersion = $null
            $reportOwned = [version]::TryParse([string]$entry.DisplayVersion, [ref]$installedVersion) -and
                $installedVersion -ge [version]'4.4.0'
            $products += [pscustomobject]@{
                Kind = 'inno'
                Channel = $family.Channel
                Elevated = $family.Elevated
                ProductCode = $null
                Uninstaller = $full
                Binary = (Join-Path $family.Root 'bin\tr300.exe')
                Report = (Join-Path $family.Root 'bin\report.exe')
                ReportOwned = $reportOwned
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

function Invoke-Tr300Process(
    [string]$FilePath,
    [string[]]$Arguments,
    [bool]$Elevated,
    [string]$WorkingDirectory = ''
) {
    $params = @{
        FilePath = $FilePath
        ArgumentList = $Arguments
        Wait = $true
        PassThru = $true
        WindowStyle = 'Hidden'
    }
    if ($WorkingDirectory) {
        if (-not [IO.Path]::IsPathRooted($WorkingDirectory)) {
            throw 'a process working directory must be absolute'
        }
        $params.WorkingDirectory = [IO.Path]::GetFullPath($WorkingDirectory)
    }
    if ($Elevated) { $params.Verb = 'RunAs' }
    return Start-Process @params
}

function Remove-Tr300NativeProduct($Product) {
    Write-Information "Switching TR-300 ownership from $($Product.Channel) to powershell-installer..."
    if ($Product.Kind -eq 'msi') {
        # Resolve again at the launch boundary rather than trusting PATH or a
        # mutable product object carried forward from discovery. Pin the child
        # working directory to the same OS-owned directory; do not clear the
        # inherited environment because Windows Installer depends on it.
        $msiexec = Get-Tr300TrustedMsiExecPath
        $systemDirectory = [IO.Path]::GetDirectoryName($msiexec)
        $process = Invoke-Tr300Process $msiexec @('/x', $Product.ProductCode, '/passive', '/norestart') $Product.Elevated $systemDirectory
        if ($process.ExitCode -notin @(0, 1605, 1614, 1641, 3010)) {
            throw "$($Product.Channel) uninstall exited with Windows Installer code $($process.ExitCode)"
        }
    } else {
        # The Global Inno uninstaller crosses UAC. Keep its current directory
        # beside the already validated absolute uninstaller rather than
        # inheriting the directory from which this wrapper happened to run.
        $uninstaller = [IO.Path]::GetFullPath([string]$Product.Uninstaller)
        $uninstallerDirectory = [IO.Path]::GetDirectoryName($uninstaller)
        if ([string]::IsNullOrWhiteSpace($uninstallerDirectory) -or
            -not [IO.Path]::IsPathRooted($uninstallerDirectory)) {
            throw "recognized $($Product.Channel) uninstaller has no absolute working directory"
        }
        $process = Invoke-Tr300Process $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') $Product.Elevated $uninstallerDirectory
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

function Test-Tr300CargoOwnsReport([string]$IntendedPrefix) {
    $cargoPrefix = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    if (-not ([IO.Path]::GetFullPath($cargoPrefix).Equals(
        [IO.Path]::GetFullPath($IntendedPrefix), [StringComparison]::OrdinalIgnoreCase))) { return $false }
    $inventoryPath = Join-Path $cargoPrefix '.crates2.json'
    if (-not (Test-Path -LiteralPath $inventoryPath -PathType Leaf)) { return $false }
    if ((Get-Item -LiteralPath $inventoryPath -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { return $false }
    try { $inventory = Get-Content -LiteralPath $inventoryPath -Raw | ConvertFrom-Json -ErrorAction Stop }
    catch { return $false }
    foreach ($package in $inventory.installs.PSObject.Properties) {
        if ($package.Name -cmatch '^tr300 [^ ]+ \([^()]+\)$' -and
            $package.Value.bins -is [array] -and
            @($package.Value.bins) -ccontains 'tr300.exe' -and
            @($package.Value.bins) -ccontains 'report.exe') { return $true }
    }
    return $false
}

function Save-Tr300ManagedState([string]$BackupRoot) {
    $receiptPath = Get-Tr300ReceiptPath
    $priorPrefix = $null
    $priorReportOwned = $false
    $receiptExisted = Test-Path -LiteralPath $receiptPath -PathType Leaf
    if ($receiptExisted) {
        $priorReceipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
        if ($priorReceipt.provider.source -ne 'cargo-dist' -or
            $priorReceipt.source.app_name -ne 'tr300' -or
            [string]::IsNullOrWhiteSpace([string]$priorReceipt.install_prefix)) {
            throw 'the existing TR-300 managed receipt is ambiguous; preserving it'
        }
        $priorPrefix = [IO.Path]::GetFullPath([string]$priorReceipt.install_prefix)
        $priorReportOwned = $priorReceipt.binaries -is [array] -and
            $priorReceipt.binaries -ccontains 'tr300.exe' -and
            $priorReceipt.binaries -ccontains 'report.exe'
        Copy-Item -LiteralPath $receiptPath -Destination (Join-Path $BackupRoot 'receipt.json')
    }

    $intendedPrefix = Get-Tr300InstallPrefix
    $intendedReport = Join-Path $intendedPrefix 'bin\report.exe'
    if (Test-Path -LiteralPath $intendedReport) {
        $samePrefix = $priorPrefix -and $intendedPrefix.Equals($priorPrefix, [StringComparison]::OrdinalIgnoreCase)
        $cargoOwned = -not $receiptExisted -and (Test-Tr300CargoOwnsReport $intendedPrefix)
        $reportItem = Get-Item -LiteralPath $intendedReport -Force
        if ((-not ($samePrefix -and $priorReportOwned) -and -not $cargoOwned) -or $reportItem.PSIsContainer -or
            ($reportItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'an unowned report command occupies the intended install path; preserving it'
        }
    }
    $binaryPaths = @(
        (Join-Path $intendedPrefix 'bin\tr300.exe'),
        (Join-Path $intendedPrefix 'bin\report.exe')
    )
    if ($priorPrefix) {
        $binaryPaths += (Join-Path $priorPrefix 'bin\tr300.exe')
        if ($priorReportOwned) { $binaryPaths += (Join-Path $priorPrefix 'bin\report.exe') }
    }
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
        PriorReportOwned = $priorReportOwned
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
    $scans = @(
        @{ Path = [string]$env:PATH; Directory = (Get-Location).ProviderPath; Expand = $true },
        @{ Path = $Tr300EntryPath; Directory = $Tr300EntryDirectory; Expand = $true },
        @{ Path = $Tr300InvocationPath; Directory = $Tr300InvocationDirectory; Expand = $false }
    )
    foreach ($scan in $scans) {
        foreach ($entry in ([string]$scan.Path).Split(';')) {
            $directory = $entry.Trim().Trim('"')
            if ($scan.Expand) { $directory = [Environment]::ExpandEnvironmentVariables($directory) }
            if (-not $directory) { $directory = $scan.Directory }
            if (-not [IO.Path]::IsPathRooted($directory)) { $directory = Join-Path $scan.Directory $directory }
            foreach ($extension in @('.exe', '.com', '.cmd', '.bat')) {
                $full = [IO.Path]::GetFullPath((Join-Path $directory ('tr300' + $extension)))
                if ([IO.File]::Exists($full) -and -not $allowed.ContainsKey($full.ToLowerInvariant())) {
                    throw "an unrecognized portable/PATH copy is active at $full; preserving it instead of guessing ownership"
                }
            }
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
    $versionOutput = @(& $binary --version)
    $versionExitCode = $LASTEXITCODE
    $reported = $versionOutput | Select-Object -First 1
    if ($versionExitCode -ne 0 -or $reported -ne "tr300 $Tr300Version") {
        throw "managed TR-300 binary did not report the expected version $Tr300Version"
    }
    $report = Join-Path $actualPrefix 'bin\report.exe'
    if (-not (Test-Path -LiteralPath $report -PathType Leaf)) {
        throw "managed report command is missing: $report"
    }
    $reportVersionOutput = @(& $report --version)
    $reportVersionExitCode = $LASTEXITCODE
    $reportVersion = $reportVersionOutput | Select-Object -First 1
    if ($reportVersionExitCode -ne 0 -or $reportVersion -ne "tr300 $Tr300Version") {
        throw "managed report command did not report the expected version $Tr300Version"
    }
    $null = & $report --fast --json
    if ($LASTEXITCODE -ne 0) {
        throw 'managed report command could not delegate to adjacent tr300'
    }
    $reportHelp = (& $report --help) -join "`n"
    if ($LASTEXITCODE -ne 0) {
        throw 'managed report help failed'
    }
    $canonicalHelp = (& $binary --help) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $reportHelp -cne $canonicalHelp) {
        throw 'report help differs from TR-300'
    }
    return $binary
}

if ($env:TR300_MANAGED_INSTALLER_TEST_ONLY -eq '1') {
    $env:PSModulePath = $Tr300OriginalPSModulePath
    return
}

$tempRoot = $null
$managedState = $null
$transactionStarted = $false
$committed = $false
$nativeRemovedCount = 0
try {
    $tempRoot = New-Tr300PrivateStaging
    $native = @(Get-Tr300NativeProducts)
    $managedState = Save-Tr300ManagedState $tempRoot
    Assert-Tr300NoUnknownPathOwners $native $managedState
    $distInstaller = Join-Path $tempRoot 'tr300-dist-installer.ps1'
    Copy-Tr300DistInstallerToPrivateStaging -Destination $distInstaller
    Assert-Tr300DistInstallerHash -Path $distInstaller
    $guardedInstaller = Join-Path $tempRoot 'tr300-verified-dist-installer.ps1'
    New-Tr300GuardedDistInstaller $distInstaller $guardedInstaller

    $transactionStarted = $true
    $installerExitCode = Invoke-Tr300DistInstaller $guardedInstaller $tempRoot ([bool]$NoModifyPath)
    if ($installerExitCode -ne 0) {
        throw "cargo-dist installation exited with code $installerExitCode"
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
        if ($product.ReportOwned -and (Test-Path -LiteralPath $product.Report -PathType Leaf)) {
            throw "native installer takeover left its report command behind: $($product.Report)"
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
        $priorReport = Join-Path $managedState.PriorPrefix 'bin\report.exe'
        $sameBinary = [IO.Path]::GetFullPath($priorBinary).Equals(
            [IO.Path]::GetFullPath($binary),
            [StringComparison]::OrdinalIgnoreCase
        )
        if (-not $sameBinary -and (Test-Path -LiteralPath $priorBinary -PathType Leaf)) {
            Remove-Item -LiteralPath $priorBinary -Force -ErrorAction Stop
        }
        $currentReport = Join-Path (Split-Path -Parent $binary) 'report.exe'
        $sameReport = [IO.Path]::GetFullPath($priorReport).Equals(
            [IO.Path]::GetFullPath($currentReport),
            [StringComparison]::OrdinalIgnoreCase
        )
        if ($managedState.PriorReportOwned -and -not $sameReport -and (Test-Path -LiteralPath $priorReport -PathType Leaf)) {
            Remove-Item -LiteralPath $priorReport -Force -ErrorAction Stop
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
    if ($tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue }
    $env:PSModulePath = $Tr300OriginalPSModulePath
}
