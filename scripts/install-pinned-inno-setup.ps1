[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$expectedVersion = '6.7.3'
$releaseTag = 'is-6_7_3'
$releaseRepository = 'jrsoftware/issrc'
$assetName = 'innosetup-6.7.3.exe'
$assetUrl = "https://github.com/$releaseRepository/releases/download/$releaseTag/$assetName"
$expectedBytes = 10592232L
$expectedSha256 = '9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732'
$expectedSigner = 'CN=Pyrsys B.V., O=Pyrsys B.V., S=Noord-Holland, C=NL'

if (-not $IsWindows) {
    throw 'the pinned Inno Setup installer is supported only on Windows'
}

$runnerTemp = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
    [IO.Path]::GetTempPath()
} else {
    $env:RUNNER_TEMP
}
$runnerTemp = [IO.Path]::GetFullPath($runnerTemp)
if (-not [IO.Path]::IsPathRooted($runnerTemp) -or
    -not (Test-Path -LiteralPath $runnerTemp -PathType Container)) {
    throw "invalid runner temporary directory: $runnerTemp"
}

$downloadDirectory = [IO.Path]::GetFullPath((Join-Path $runnerTemp 'tr300-inno-setup-6.7.3'))
if (-not ([IO.Path]::GetDirectoryName($downloadDirectory)).Equals(
        $runnerTemp.TrimEnd([IO.Path]::DirectorySeparatorChar),
        [StringComparison]::OrdinalIgnoreCase
    )) {
    throw 'the Inno Setup staging directory escaped the runner temporary directory'
}
New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null
$installer = [IO.Path]::GetFullPath((Join-Path $downloadDirectory $assetName))
if (-not ([IO.Path]::GetDirectoryName($installer)).Equals(
        $downloadDirectory,
        [StringComparison]::OrdinalIgnoreCase
    )) {
    throw 'the Inno Setup asset path escaped its private staging directory'
}

Invoke-WebRequest -UseBasicParsing -Uri $assetUrl -OutFile $installer
$installerItem = Get-Item -LiteralPath $installer
if (-not $installerItem.PSIsContainer -and
    -not $installerItem.LinkType -and
    $installerItem.Length -eq $expectedBytes) {
    # Continue only for the exact immutable release asset.
} else {
    throw "unexpected Inno Setup asset type or size: $($installerItem.Length) bytes"
}
$actualSha256 = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSha256 -cne $expectedSha256) {
    throw "Inno Setup asset checksum mismatch: $actualSha256"
}
$installerVersion = $installerItem.VersionInfo.ProductVersion.Trim()
if ($installerVersion -cne $expectedVersion) {
    throw "unexpected Inno Setup installer ProductVersion: $installerVersion"
}

$installerSignature = Get-AuthenticodeSignature -LiteralPath $installer
if ($installerSignature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
    $null -eq $installerSignature.SignerCertificate -or
    $installerSignature.SignerCertificate.Subject -cne $expectedSigner) {
    throw "Inno Setup asset has an invalid publisher signature: $($installerSignature.Status)"
}

# GitHub's release attestation is an independent binding between these bytes
# and JRSoftware's immutable is-6_7_3 release. The repository-pinned SHA-256
# and Authenticode publisher checks above remain mandatory even if this CLI
# surface changes in the future.
$gh = Get-Command gh -CommandType Application -ErrorAction Stop
$ghPath = [IO.Path]::GetFullPath($gh.Source)
if (-not (Test-Path -LiteralPath $ghPath -PathType Leaf)) {
    throw 'the GitHub CLI executable is unavailable'
}
& $ghPath release verify-asset $releaseTag $installer --repo $releaseRepository --format json | Out-Null
$verificationExitCode = $LASTEXITCODE

# The official installer is third-party code and must not inherit the token
# needed only by GitHub CLI for the release-attestation check above.
Remove-Item -LiteralPath Env:GH_TOKEN -ErrorAction SilentlyContinue
if (Test-Path -LiteralPath Env:GH_TOKEN) {
    throw 'could not remove GitHub authorization before launching Inno Setup'
}
if ($verificationExitCode -ne 0) {
    throw 'GitHub release attestation verification failed for Inno Setup'
}

$installerProcess = Start-Process -FilePath $installer `
    -WorkingDirectory $downloadDirectory `
    -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-') `
    -Wait -PassThru -WindowStyle Hidden
if ($installerProcess.ExitCode -ne 0) {
    throw "Inno Setup installation failed with exit code $($installerProcess.ExitCode)"
}

$isccDirectory = [IO.Path]::GetFullPath((Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6'))
$iscc = [IO.Path]::GetFullPath((Join-Path $isccDirectory 'ISCC.exe'))
if (-not ([IO.Path]::GetDirectoryName($iscc)).Equals(
        $isccDirectory,
        [StringComparison]::OrdinalIgnoreCase
    ) -or
    -not (Test-Path -LiteralPath $iscc -PathType Leaf)) {
    throw "the pinned Inno Setup compiler is unavailable: $iscc"
}
$isccItem = Get-Item -LiteralPath $iscc
if ($isccItem.LinkType) {
    throw 'the Inno Setup compiler must be a regular non-link file'
}
$isccSignature = Get-AuthenticodeSignature -LiteralPath $iscc
if ($isccSignature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
    $null -eq $isccSignature.SignerCertificate -or
    $isccSignature.SignerCertificate.Subject -cne $expectedSigner) {
    throw "the Inno Setup compiler has an invalid publisher signature: $($isccSignature.Status)"
}

# ISCC.exe intentionally carries 0.0.0.0 in both Windows version-resource
# fields, so VersionInfo cannot establish the compiler release. Ask the now-
# authenticated compiler's own preprocessor to prove its exact encoded version
# before any repository installer source is compiled.
if ($expectedVersion -notmatch '^(?<major>\d+)\.(?<minor>\d+)\.(?<revision>\d+)$') {
    throw "invalid pinned Inno Setup version: $expectedVersion"
}
$majorVersion = [int]$Matches.major
$minorVersion = [int]$Matches.minor
$revisionVersion = [int]$Matches.revision
$versionProbe = [IO.Path]::GetFullPath((Join-Path $downloadDirectory 'verify-iscc-version.iss'))
$versionProbeOutput = [IO.Path]::GetFullPath((Join-Path $downloadDirectory 'version-probe-output'))
New-Item -ItemType Directory -Force -Path $versionProbeOutput | Out-Null
$versionProbeSource = @"
#preproc ispp
#if Ver != EncodeVer($majorVersion, $minorVersion, $revisionVersion)
  #error "Unexpected Inno Setup compiler version"
#endif

[Setup]
AppName=TR-300 Compiler Version Probe
AppVersion=0.0.0
DefaultDirName={tmp}\TR300CompilerProbe
PrivilegesRequired=lowest
Uninstallable=no
OutputBaseFilename=tr300-compiler-version-probe
"@
[IO.File]::WriteAllText(
    $versionProbe,
    $versionProbeSource,
    [Text.UTF8Encoding]::new($false)
)
& $iscc '/Qp' "/O$versionProbeOutput" $versionProbe
if ($LASTEXITCODE -ne 0) {
    throw "unexpected Inno Setup compiler version; expected $expectedVersion"
}

Write-Host "Verified Inno Setup $expectedVersion at $iscc"
