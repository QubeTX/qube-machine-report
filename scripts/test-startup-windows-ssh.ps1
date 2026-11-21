param([Parameter(Mandatory)][string] $Binary)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'This acceptance test requires a disposable GitHub-hosted runner.'
}
$server = "$env:WINDIR\System32\OpenSSH\sshd.exe"
if (!(Test-Path $server)) {
    $capability = Get-WindowsCapability -Online -Name 'OpenSSH.Server*'
    if ($capability.State -ne 'Installed') {
        Add-WindowsCapability -Online -Name $capability.Name | Out-Null
    }
}
if (!(Test-Path $server)) { throw 'Windows OpenSSH server is unavailable.' }
$ssh = "$env:WINDIR\System32\OpenSSH\ssh.exe"
$keygen = "$env:WINDIR\System32\OpenSSH\ssh-keygen.exe"
$tag = [guid]::NewGuid().ToString('N').Substring(0,8)
$account = "tr300ci$tag"
$task = "TR300-SSH-Fixture-$tag"
$dir = Join-Path $env:ProgramData $task
$client = Join-Path $env:RUNNER_TEMP $task
$registered = $false
$created = $false
function Invoke-Checked([string] $Exe, [string[]] $Arguments) {
    & $Exe @Arguments
    if ($LASTEXITCODE) { throw "$Exe failed ($LASTEXITCODE)" }
}
function Protect([string] $Path, [string[]] $Grants) {
    Invoke-Checked icacls.exe (@($Path, '/inheritance:r', '/grant:r') + $Grants) | Out-Null
    Invoke-Checked icacls.exe @($Path, '/setowner', '*S-1-5-32-544') | Out-Null
}
try {
    New-Item -ItemType Directory $dir,$client | Out-Null
    Protect $dir @('*S-1-5-18:(OI)(CI)F','*S-1-5-32-544:(OI)(CI)F','*S-1-5-11:RX')
    $password = ConvertTo-SecureString ('aA1!'+[guid]::NewGuid().ToString('N')) -AsPlainText -Force
    $user = New-LocalUser -Name $account -Password $password
    $created = $true
    $reader = '*' + $user.SID.Value + ':RX'
    Copy-Item -LiteralPath $Binary -Destination "$dir\tr300.exe"
    Protect "$dir\tr300.exe" @('*S-1-5-18:F','*S-1-5-32-544:F',$reader)
    # pwsh 7's Standard native-argument mode passes the empty passphrase.
    $PSNativeCommandArgumentPassing = 'Standard'
    Invoke-Checked $keygen @('-q','-t','ed25519','-N','','-f',"$client\key")
    Invoke-Checked $keygen @('-q','-t','ed25519','-N','','-f',"$dir\host")
    Protect "$dir\host" @('*S-1-5-18:F','*S-1-5-32-544:F')
    Copy-Item "$client\key.pub" "$dir\authorized_keys"
    Protect "$dir\authorized_keys" @('*S-1-5-18:F','*S-1-5-32-544:F',$reader)
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,0)
    $listener.Start()
    $port = $listener.LocalEndpoint.Port
    $listener.Stop()
    $unixDir = $dir.Replace('\','/')
    $unixClient = $client.Replace('\','/')
    @"
ListenAddress 127.0.0.1
Port $port
HostKey "$unixDir/host"
PidFile "$unixDir/sshd.pid"
AllowUsers $account
AuthorizedKeysFile "$unixDir/authorized_keys"
PasswordAuthentication no
PubkeyAuthentication yes
StrictModes no
"@ | Set-Content -Encoding ascii "$dir\server.conf"
    ("[127.0.0.1]:$port " + (Get-Content "$dir\host.pub" -Raw).Trim()) |
        Set-Content -Encoding ascii "$client\known_hosts"
    @"
Host tr300-fixture
    HostName 127.0.0.1
    Port $port
    User $account
    IdentityFile "$unixClient/key"
    IdentitiesOnly yes
    BatchMode yes
    UserKnownHostsFile "$unixClient/known_hosts"
    StrictHostKeyChecking yes
    LogLevel ERROR
"@ | Set-Content -Encoding ascii "$client\client.conf"
    $argsString = "-D -f `"$dir\server.conf`" -E `"$dir\server.log`""
    $action = New-ScheduledTaskAction -Execute $server -Argument $argsString
    Register-ScheduledTask -TaskName $task -Action $action -User SYSTEM -RunLevel Highest | Out-Null
    $registered = $true
    Start-ScheduledTask -TaskName $task
    $deadline = (Get-Date).AddSeconds(30)
    do {
        $conn = [Net.Sockets.TcpClient]::new()
        try { $conn.Connect('127.0.0.1',$port) } catch {} finally { $ready=$conn.Connected; $conn.Dispose() }
        if (!$ready) { Start-Sleep -Milliseconds 300 }
    } until ($ready -or (Get-Date) -ge $deadline)
    if (!$ready) { throw 'Disposable OpenSSH listener did not start.' }
    $base = @('-F',"$client\client.conf")
    Invoke-Checked $ssh ($base + @('-T','tr300-fixture',"$dir\tr300.exe install"))
    Invoke-Checked python @("$PSScriptRoot\test-startup-windows-ssh.py",$ssh,"$client\client.conf","$client\error.txt")
    foreach ($tty in '-T','-tt') {
        $output = Invoke-Checked $ssh ($base + @($tty,'tr300-fixture','echo REMOTE_COMMAND_OK')) | Out-String
        if ($output -notmatch 'REMOTE_COMMAND_OK' -or $output -match 'PROCESSOR|SERVER IP') { throw $output }
    }
    Write-Output 'Passed Windows OpenSSH shell, noninteractive command and forced-PTY command.'
} catch {
    if (Test-Path "$dir\server.log") { Get-Content "$dir\server.log" }
    throw
} finally {
    if ($registered) {
        Stop-ScheduledTask -TaskName $task
        Unregister-ScheduledTask -TaskName $task -Confirm:$false
    }
    if ($created) { Remove-LocalUser -Name $account }
    foreach ($root in @($env:ProgramData, $env:RUNNER_TEMP)) {
        $path = [IO.Path]::GetFullPath((Join-Path $root $task))
        if ([IO.Path]::GetDirectoryName($path) -ne [IO.Path]::GetFullPath($root).TrimEnd('\')) { throw 'Unexpected test directory' }
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force -Recurse }
    }
}
