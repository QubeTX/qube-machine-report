[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'Execution policy lifecycle testing requires a disposable GitHub-hosted Windows runner.'
}
$programs = [Environment]::GetFolderPath('ProgramFiles')
$bin = Join-Path $programs 'tr300\bin\tr300.exe'
$hosts = @((Join-Path ([Environment]::SystemDirectory) 'WindowsPowerShell\v1.0\powershell.exe'))
$pwsh = Join-Path $programs 'PowerShell\7\pwsh.exe'
if (Test-Path -LiteralPath $pwsh) { $hosts += $pwsh }

function Read-Policy($shell,$scope) {
    $value = @(& $shell -NoProfile -Command "Get-ExecutionPolicy -Scope $scope")
    if ($LASTEXITCODE -ne 0 -or $value.Count -ne 1) { throw "Could not read $scope in $shell" }
    return [string]$value[0]
}
function Set-FixturePolicy($shell,$scope,$value) {
    if ($scope -notin 'LocalMachine','CurrentUser' -or $value -notin 'Undefined','Restricted','AllSigned','RemoteSigned','Unrestricted','Bypass') {
        throw 'Invalid fixture execution policy.'
    }
    # A higher scope can emit ExecutionPolicyOverride even after this scope
    # was successfully written. Re-read the actual configured scope.
    $code = "try { Set-ExecutionPolicy -Scope $scope -ExecutionPolicy $value -Force -ErrorAction Stop } catch { }; if ((Get-ExecutionPolicy -Scope $scope) -ne '$value') { exit 8 }"
    & $shell -NoProfile -Command $code
    if ($LASTEXITCODE -ne 0) { throw "Could not establish $scope = $value in $shell" }
}
$processPolicy = [Environment]::GetEnvironmentVariable('PSExecutionPolicyPreference')
$states = @()
try {
    [Environment]::SetEnvironmentVariable('PSExecutionPolicyPreference',$null)
    foreach ($shell in $hosts) {
        foreach ($scope in 'MachinePolicy','UserPolicy') {
            if ((Read-Policy $shell $scope) -ne 'Undefined') { throw 'Hosted fixture requires an unmanaged runner.' }
        }
        $states += [pscustomobject]@{ Shell=$shell; Machine=(Read-Policy $shell 'LocalMachine'); User=(Read-Policy $shell 'CurrentUser') }
    }
    foreach ($s in $states) {
        Set-FixturePolicy $s.Shell 'CurrentUser' 'Restricted'
        Set-FixturePolicy $s.Shell 'LocalMachine' 'Restricted'
    }
    # Policy changed after install. Re-registration must remove our previously
    # usable shared file without creating an unsigned replacement.
    & $bin configure-startup-machine
    if ($LASTEXITCODE -ne 0) { throw 'Restricted-policy Global reconciliation failed' }
    foreach ($s in $states) {
        $profileFile = Join-Path (Split-Path -Parent $s.Shell) 'profile.ps1'
        if ((Test-Path -LiteralPath $profileFile) -and
            (Select-String -LiteralPath $profileFile -SimpleMatch '# TR-300 Machine Report (all users)' -Quiet)) {
            throw "A blocked host retained its TR-300 profile: $profileFile"
        }
        $output = @(& $s.Shell -NoLogo -Command "[Console]::WriteLine('restricted-script-proof'); exit 31" 2>&1)
        if ($LASTEXITCODE -ne 31 -or $output.Count -ne 1 -or $output[0] -cne 'restricted-script-proof') {
            throw "Restricted host emitted profile-load diagnostics or changed script output: $($s.Shell) $output"
        }
    }
    # Exercise Explorer's StubPath under a disposable standard user token and
    # loaded HKCU. The fixture code is transported inline so Restricted never
    # needs to execute the fixture itself as a .ps1 file.
    $name = 'TR3' + [guid]::NewGuid().ToString('N').Substring(0,12)
    $secret = ConvertTo-SecureString ("Tr3!" + [guid]::NewGuid().ToString('N')) -AsPlainText -Force
    $account = New-LocalUser -Name $name -Password $secret -ErrorAction Stop
    try {
        $users = Get-LocalGroup -SID 'S-1-5-32-545'
        if (-not (Get-LocalGroupMember -Group $users | Where-Object { $_.SID -eq $account.SID })) {
            Add-LocalGroupMember -Group $users -Member $account
        }
        Start-Service seclogon
        $credential = [pscredential]::new("$env:COMPUTERNAME\$name",$secret)
        # CreateProcessWithLogonW caps the whole command line at 1024 UTF-16
        # characters. The installed package ACL protects this temporary test
        # source while permitting the standard account to read it.
        $fixturePath = Join-Path (Split-Path -Parent $bin) ('tr300-account-test-' + [guid]::NewGuid().ToString('N') + '.txt')
        Copy-Item -LiteralPath "$PSScriptRoot/test-windows-startup-account.ps1" -Destination $fixturePath
        $literal = $fixturePath.Replace("'", "''")
        $command = '$env:TR300_POLICY_FIXTURE=' + "'$literal'" + '; & ([scriptblock]::Create([IO.File]::ReadAllText($env:TR300_POLICY_FIXTURE)))'
        $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command))
        $stderr = Join-Path $env:RUNNER_TEMP "$name-stderr.txt"
        $stdout = Join-Path $env:RUNNER_TEMP "$name-stdout.txt"
        $p=Start-Process -RedirectStandardError $stderr -RedirectStandardOutput $stdout -FilePath $hosts[0] -Credential $credential -LoadUserProfile -WindowStyle Hidden -Wait -PassThru -WorkingDirectory ([Environment]::SystemDirectory) -ArgumentList @('-NoLogo','-NoProfile','-EncodedCommand',$encoded)
        if ($p.ExitCode -ne 0) {
            Get-Content -LiteralPath $stdout,$stderr -ErrorAction SilentlyContinue | Write-Host
            throw "Standard account bootstrap failed at checkpoint $($p.ExitCode)"
        }
    } finally {
        if ($fixturePath -and (Test-Path -LiteralPath $fixturePath)) { Remove-Item -LiteralPath $fixturePath }
        Remove-LocalUser -SID $account.SID
    }
    foreach ($s in $states) {
        if ((Read-Policy $s.Shell 'CurrentUser') -ne 'Restricted' -or
            (Read-Policy $s.Shell 'LocalMachine') -ne 'Restricted') {
            throw "Global registration overrode an explicit policy in $($s.Shell)"
        }
    }
} finally {
    foreach ($s in $states) {
        Set-FixturePolicy $s.Shell 'LocalMachine' $s.Machine
        Set-FixturePolicy $s.Shell 'CurrentUser' $s.User
    }
    [Environment]::SetEnvironmentVariable('PSExecutionPolicyPreference',$processPolicy)
    & $bin configure-startup-machine
    if ($LASTEXITCODE -ne 0) { throw 'Could not restore runner startup baseline' }
}
exit 0
