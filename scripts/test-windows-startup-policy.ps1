[CmdletBinding()]
param([Parameter(Mandatory)][string]$Package)

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
$msi = (Resolve-Path -LiteralPath $Package).Path
$installer = [IO.Path]::Combine([Environment]::SystemDirectory,'msiexec.exe')
function Repair-GlobalMsi([switch]$ExpectLockedProfile) {
    $log = Join-Path $env:RUNNER_TEMP ('tr300-repair-' + [guid]::NewGuid().ToString('N') + '.log')
    $repair = Start-Process -FilePath $installer -WorkingDirectory ([Environment]::SystemDirectory) -WindowStyle Hidden -PassThru -Wait -ArgumentList @('/fomus', ('"{0}"' -f $msi), '/qn', '/norestart', '/L*v', ('"{0}"' -f $log))
    if ($ExpectLockedProfile) {
        if ($repair.ExitCode -eq 1603 -and (Select-String -LiteralPath $log -Pattern 'CustomAction RepairMachineStartup returned actual error code [1-9]' -Quiet)) { return }
    } elseif ($repair.ExitCode -eq 0 -and (Select-String -LiteralPath $log -Pattern 'ActionStart\(Name=RepairMachineStartup|Action start [^:]*: RepairMachineStartup|CustomActionSchedule\(Action=RepairMachineStartup' -Quiet)) { return }
    Get-Content -LiteralPath $log -Tail 140 | Write-Host
    throw "Unexpected Global MSI repair result $($repair.ExitCode) (locked profile: $ExpectLockedProfile)"
}
$processPolicy = [Environment]::GetEnvironmentVariable('PSExecutionPolicyPreference')
$states = @()
$prefDir = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'tr300'
$optoutPath = Join-Path $prefDir 'startup-disabled'
$createdOptout = $false
try {
    if (-not [IO.File]::Exists($optoutPath)) {
        [IO.Directory]::CreateDirectory($prefDir) | Out-Null
        $stream = [IO.File]::Open($optoutPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
        try { $stream.WriteByte(42) } finally { $stream.Dispose() }
        $createdOptout = $true
    }
    $prefHash = (Get-FileHash -LiteralPath $optoutPath).Hash
    [Environment]::SetEnvironmentVariable('PSExecutionPolicyPreference',$null)
    foreach ($shell in $hosts) {
        foreach ($scope in 'MachinePolicy','UserPolicy') {
            if ((Read-Policy $shell $scope) -ne 'Undefined') { throw 'Hosted fixture requires an unmanaged runner.' }
        }
        $states += [pscustomobject]@{ Shell=$shell; Machine=(Read-Policy $shell 'LocalMachine'); User=(Read-Policy $shell 'CurrentUser') }
    }
    # Guarantee the fixture starts with both installed hosts eligible, even
    # when a runner image changes its default policy.
    foreach ($s in $states) { Set-FixturePolicy $s.Shell 'LocalMachine' 'RemoteSigned' }
    Repair-GlobalMsi
    $profiles = @($states | ForEach-Object { Join-Path (Split-Path -Parent $_.Shell) 'profile.ps1' })
    $hashes = @($profiles | ForEach-Object { (Get-FileHash -LiteralPath $_).Hash })
    $commandKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('Software\Microsoft\Command Processor',$true)
    $kind = $commandKey.GetValueKind('AutoRun')
    $originalCmd = $commandKey.GetValue('AutoRun',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $activeKey = 'HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Active Setup\Installed Components\{3315638A-7A5F-4591-A9FE-79312ED7859B}'
    $activeVersion = [Microsoft.Win32.Registry]::GetValue($activeKey,'Version',$null)
    $activeStub = [Microsoft.Win32.Registry]::GetValue($activeKey,'StubPath',$null)
    foreach ($s in $states) {
        Set-FixturePolicy $s.Shell 'CurrentUser' 'Restricted'
        Set-FixturePolicy $s.Shell 'LocalMachine' 'Restricted'
    }
    # Make the last file readable at preflight, but unavailable for deletion
    # at commit. Earlier profiles, Active Setup, and CMD must survive rollback.
    $held = [IO.File]::Open($profiles[-1],[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
    try { Repair-GlobalMsi -ExpectLockedProfile } finally { $held.Dispose() }
    for ($i=0; $i -lt $profiles.Count; $i++) {
        if ((Get-FileHash -LiteralPath $profiles[$i]).Hash -cne $hashes[$i]) { throw "Failed repair changed $($profiles[$i])" }
    }
    if ((Get-FileHash -LiteralPath $optoutPath).Hash -cne $prefHash) { throw 'MSI repair changed a user opt-out' }
    if ($commandKey.GetValue('AutoRun',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) -cne $originalCmd -or
        [Microsoft.Win32.Registry]::GetValue($activeKey,'Version',$null) -cne $activeVersion -or
        [Microsoft.Win32.Registry]::GetValue($activeKey,'StubPath',$null) -cne $activeStub) {
        throw 'Failed repair changed a retained Global registration'
    }

    # Remove only the owned hook. Preserve any pre-existing runner command.
    $hook = '(if exist "'+$bin+'" ("'+$bin+'" --startup-hook & set "TR300_AUTORUN_RAN=1")) & ver >nul'
    if ($originalCmd -ceq $hook) { $commandKey.DeleteValue('AutoRun') }
    elseif ($originalCmd.StartsWith($hook+' & ',[StringComparison]::Ordinal)) {
        $commandKey.SetValue('AutoRun',$originalCmd.Substring($hook.Length+3),$kind)
    } else { throw 'Fixture could not identify the exact TR-300 AutoRun hook' }
    Repair-GlobalMsi
    if ((Get-FileHash -LiteralPath $optoutPath).Hash -cne $prefHash) { throw 'MSI repair changed a user opt-out' }
    if ($commandKey.GetValue('AutoRun',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) -cne $originalCmd -or
        $commandKey.GetValueKind('AutoRun') -ne $kind) { throw 'Successful repair did not restore the original CMD registration' }
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
    if ($commandKey) { $commandKey.Dispose() }
    try { Repair-GlobalMsi } finally { if ($createdOptout) { [IO.File]::Delete($optoutPath) } }
}
exit 0
