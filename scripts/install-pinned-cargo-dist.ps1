[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$installer = Join-Path $env:RUNNER_TEMP 'cargo-dist-installer-v0.31.0.ps1'
Invoke-WebRequest -UseBasicParsing `
    -Uri 'https://github.com/axodotdev/cargo-dist/releases/download/v0.31.0/cargo-dist-installer.ps1' `
    -OutFile $installer

$actual = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -cne 'ffec5b52cfbe29465d831150b01f8a254668fc271e5102fab7aea7da5d51ec69') {
    throw "cargo-dist installer checksum mismatch: $actual"
}

& $installer
if ($LASTEXITCODE) {
    exit $LASTEXITCODE
}

$distPath = Join-Path ([Environment]::GetFolderPath('UserProfile')) '.cargo\bin\dist.exe'
$distPath = [IO.Path]::GetFullPath($distPath)
if (-not (Test-Path -LiteralPath $distPath -PathType Leaf)) {
    throw 'cargo-dist executable is missing or not a regular file'
}

$distItem = Get-Item -LiteralPath $distPath -Force
$isReparsePoint = ($distItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
if ($distItem.PSIsContainer -or $isReparsePoint) {
    throw 'cargo-dist executable is not a regular non-reparse-point file'
}

$distVersion = (& $distPath --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $distVersion -cne 'cargo-dist 0.31.0') {
    throw "unexpected cargo-dist version: $distVersion"
}
