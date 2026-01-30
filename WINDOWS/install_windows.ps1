<#!
.SYNOPSIS
    Windows installer for TR-200 Machine Report (PowerShell version).

.DESCRIPTION
    Copies the TR-200 PowerShell script to a per-user install directory,
    wires up a `report` command for both PowerShell and Command Prompt,
    configures your PowerShell profile so the report is easy to run
    and automatically displays on login/SSH sessions, and sets up a
    Windows Task Scheduler task for login-time execution.

.NOTES
    Copyright 2026, ES Development LLC (https://emmetts.dev)
    Based on original work by U.S. Graphics, LLC (BSD-3-Clause)

    Run this on the Windows machine (not from WSL) using:

      pwsh   -File WINDOWS/install_windows.ps1
      # or
      powershell -ExecutionPolicy Bypass -File WINDOWS/install_windows.ps1

    The script uses only per-user locations and does not require admin rights.
#>

param(
    [switch]$NoAutoRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host '=========================================='
Write-Host 'TR-200 Machine Report - Windows Installer'
Write-Host '=========================================='
Write-Host ''

# Resolve paths relative to this script
$scriptRoot   = Split-Path -Parent $MyInvocation.MyCommand.Path
$sourceScript = Join-Path $scriptRoot 'TR-200-MachineReport.ps1'

if (-not (Test-Path $sourceScript)) {
    Write-Error "TR-200-MachineReport.ps1 not found next to this installer: $sourceScript"
    exit 1
}

# Choose install directory under the user profile so both PowerShell and Cmd can see it
$home = $HOME
if (-not $home) { $home = $env:USERPROFILE }
if (-not $home) {
    Write-Error 'Unable to determine user home directory.'
    exit 1
}

$installDir     = Join-Path $home 'TR200'
$targetScript   = Join-Path $installDir 'TR-200-MachineReport.ps1'
$batchShim      = Join-Path $installDir 'report.cmd'
$uninstallScript = Join-Path $installDir 'uninstall.ps1'

Write-Host "Install directory: $installDir"

if (-not (Test-Path $installDir)) {
    Write-Host 'Creating install directory...'
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

Write-Host 'Copying TR-200 PowerShell script...'
Copy-Item -Path $sourceScript -Destination $targetScript -Force

# Ensure the script is readable
if (-not (Test-Path $targetScript)) {
    Write-Error "Failed to install script to $targetScript"
    exit 1
}

Write-Host '✓ Script installed.'
Write-Host ''

# Create a batch shim so `report` works from Command Prompt and PowerShell via PATH
Write-Host 'Creating report.cmd shim...'
$batchContent = @"
@echo off
REM TR-200 Machine Report launcher
REM First try PowerShell 7 (pwsh), then Windows PowerShell 5.1

where pwsh >nul 2>&1
if %errorlevel%==0 (
    pwsh -NoLogo -NoProfile -File "%~dp0TR-200-MachineReport.ps1" %*
    goto :EOF
)

where powershell >nul 2>&1
if %errorlevel%==0 (
    powershell -NoLogo -NoProfile -File "%~dp0TR-200-MachineReport.ps1" %*
    goto :EOF
)

echo Could not find pwsh.exe or powershell.exe in PATH.
exit /b 1
"@

Set-Content -Path $batchShim -Value $batchContent -Encoding ASCII
Write-Host "✓ Created: $batchShim"
Write-Host ''

# Create uninstall.cmd shim
$uninstallBatch = Join-Path $installDir 'uninstall.cmd'
$uninstallBatchContent = @"
@echo off
REM TR-200 Machine Report Uninstaller
REM First try PowerShell 7 (pwsh), then Windows PowerShell 5.1

where pwsh >nul 2>&1
if %errorlevel%==0 (
    pwsh -NoLogo -NoProfile -File "%~dp0uninstall.ps1" %*
    goto :EOF
)

where powershell >nul 2>&1
if %errorlevel%==0 (
    powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0uninstall.ps1" %*
    goto :EOF
)

echo Could not find pwsh.exe or powershell.exe in PATH.
exit /b 1
"@
Set-Content -Path $uninstallBatch -Value $uninstallBatchContent -Encoding ASCII
Write-Host "✓ Created: $uninstallBatch"

# Ensure install directory is on the per-user PATH so `report` works everywhere
Write-Host ''
Write-Host 'Configuring user PATH so `report` and `uninstall` are available in Cmd/PowerShell...'
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ([string]::IsNullOrWhiteSpace($userPath)) {
    $userPath = ''
}

$pathEntries = $userPath.Split(';') | Where-Object { $_ -ne '' }
$alreadyInPath = $pathEntries -contains $installDir

if (-not $alreadyInPath) {
    if ($userPath -and -not $userPath.EndsWith(';')) {
        $userPath += ';'
    }
    $userPath += $installDir
    [Environment]::SetEnvironmentVariable('Path', $userPath, 'User')
    Write-Host "✓ Added to user PATH: $installDir"
    Write-Host '  (You may need to open a new terminal/console for this to take effect.)'
} else {
    Write-Host '✓ Install directory already present in user PATH.'
}

Write-Host ''

# ============================================================================
# CREATE TASK SCHEDULER TASK (login-time execution)
# ============================================================================
Write-Host 'Configuring Windows Task Scheduler for login-time execution...'

$taskName = 'TR200-MachineReport'

try {
    # Check if task already exists
    $existingTask = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue

    if ($existingTask) {
        Write-Host '  Removing existing scheduled task...'
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    }

    # Determine which PowerShell to use
    $pwshPath = Get-Command pwsh -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
    if (-not $pwshPath) {
        $pwshPath = Get-Command powershell -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
    }

    if ($pwshPath) {
        $taskAction = New-ScheduledTaskAction -Execute $pwshPath `
            -Argument "-NoLogo -NoProfile -WindowStyle Hidden -File `"$targetScript`""

        $taskTrigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME

        $taskSettings = New-ScheduledTaskSettings -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable

        Register-ScheduledTask -TaskName $taskName -Action $taskAction `
            -Trigger $taskTrigger -Settings $taskSettings -Force | Out-Null

        Write-Host "✓ Scheduled task '$taskName' created (runs at Windows login)"
    } else {
        Write-Warning 'Could not find PowerShell executable for scheduled task.'
    }
} catch {
    Write-Warning "Could not create scheduled task: $_"
    Write-Warning 'The report will still run in new PowerShell sessions.'
}

Write-Host ''

# ============================================================================
# CREATE UNINSTALL SCRIPT
# ============================================================================
Write-Host 'Creating uninstall script...'

$uninstallContent = @'
# TR-200 Machine Report Uninstaller for Windows
# Copyright 2026, ES Development LLC (https://emmetts.dev)

Write-Host '=========================================='
Write-Host 'TR-200 Machine Report Uninstaller'
Write-Host '=========================================='
Write-Host ''

$confirmation = Read-Host 'Are you sure you want to uninstall TR-200 Machine Report? (y/N)'
if ($confirmation -notmatch '^[Yy]') {
    Write-Host 'Uninstallation cancelled.'
    exit 0
}

Write-Host ''
Write-Host 'Uninstalling TR-200 Machine Report...'
Write-Host ''

$installDir = Join-Path $HOME 'TR200'

# Remove scheduled task
$taskName = 'TR200-MachineReport'
try {
    $task = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    if ($task) {
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
        Write-Host "✓ Removed scheduled task: $taskName"
    }
} catch {
    Write-Warning "Could not remove scheduled task: $_"
}

# Remove from PATH
try {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath) {
        $newPath = ($userPath.Split(';') | Where-Object { $_ -ne $installDir -and $_ -notlike '*TR200*' }) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Host '✓ Removed from user PATH'
    }
} catch {
    Write-Warning "Could not update PATH: $_"
}

# Remove PowerShell profile entries
try {
    $profilePath = $PROFILE.CurrentUserAllHosts
    if (-not $profilePath) { $profilePath = $PROFILE }

    if (Test-Path $profilePath) {
        $content = Get-Content $profilePath -Raw -ErrorAction SilentlyContinue
        if ($content) {
            # Remove TR-200 configuration block
            $pattern = '(?s)# >>> TR-200 Machine Report configuration >>>.*?# <<< TR-200 Machine Report configuration <<<\r?\n?'
            $newContent = $content -replace $pattern, ''

            # Also remove any stray function definitions
            $newContent = $newContent -replace '(?m)^function report \{[^}]+\}\r?\n?', ''
            $newContent = $newContent -replace '(?m)^function uninstall \{[^}]+\}\r?\n?', ''

            Set-Content $profilePath $newContent.Trim()
            Write-Host "✓ Cleaned PowerShell profile: $profilePath"
        }
    }
} catch {
    Write-Warning "Could not clean profile: $_"
}

# Self-destruct: schedule removal of install directory
Write-Host ''
Write-Host '=========================================='
Write-Host 'TR-200 Machine Report Uninstalled'
Write-Host '=========================================='
Write-Host ''
Write-Host 'Note: You may need to close and reopen PowerShell/Command Prompt'
Write-Host '      for all changes to take effect.'
Write-Host ''
Write-Host "Install directory will be removed: $installDir"
Write-Host ''

# Remove install directory (including this script)
try {
    # Use Start-Process to delete after this script exits
    $deleteCmd = "Start-Sleep -Seconds 2; Remove-Item -Recurse -Force '$installDir' -ErrorAction SilentlyContinue"
    Start-Process -FilePath 'powershell' -ArgumentList "-NoProfile -WindowStyle Hidden -Command `"$deleteCmd`"" -WindowStyle Hidden
    Write-Host '✓ Cleanup scheduled'
} catch {
    Write-Warning "Could not schedule cleanup. Please manually delete: $installDir"
}
'@

Set-Content -Path $uninstallScript -Value $uninstallContent -Encoding UTF8
Write-Host "✓ Created uninstall script: $uninstallScript"
Write-Host "  Run 'uninstall' command to remove TR-200 Machine Report"

Write-Host ''

# ============================================================================
# CONFIGURE POWERSHELL PROFILE
# ============================================================================
Write-Host 'Configuring PowerShell profile...'
try {
    $profilePath = $PROFILE.CurrentUserAllHosts
} catch {
    $profilePath = $PROFILE
}

if (-not $profilePath) {
    Write-Warning 'Could not determine PowerShell profile path; skipping profile configuration.'
} else {
    $profileDir = Split-Path -Parent $profilePath
    if (-not (Test-Path $profileDir)) {
        New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
    }

    if (-not (Test-Path $profilePath)) {
        New-Item -ItemType File -Path $profilePath -Force | Out-Null
    }

    $profileContent = Get-Content -Path $profilePath -ErrorAction SilentlyContinue
    $marker = '# >>> TR-200 Machine Report configuration >>>'

    if ($profileContent -and ($profileContent -contains $marker)) {
        Write-Host '✓ Profile already configured for TR-200.'
    } else {
        Write-Host "Adding TR-200 configuration to profile: $profilePath"

        $autoRunLogic = if ($NoAutoRun) {
            '        # Auto-run disabled by installer switch'
        } else {
            @'
        # Auto-run TR-200 on interactive or SSH sessions (clear screen first)
        $isSSH = $env:SSH_CLIENT -or $env:SSH_CONNECTION
        $isInteractiveHost = $Host.Name -ne 'ServerRemoteHost'
        if ($isSSH -or $isInteractiveHost) {
            try {
                Clear-Host
                Show-TR200Report
            } catch { }
        }
'@
        }

        $profileAppend = @"

$marker
# Load TR-200 Machine Report script and define commands
if (Test-Path '$targetScript') {
    try {
        . '$targetScript'

        function report {
            [CmdletBinding()]
            param()
            Show-TR200Report
        }

        function uninstall {
            [CmdletBinding()]
            param()
            & '$uninstallScript'
        }

$autoRunLogic
    } catch {
        Write-Warning 'Failed to load TR-200 Machine Report script from profile.'
    }
} else {
    Write-Warning 'TR-200 Machine Report script not found at $targetScript.'
}
# <<< TR-200 Machine Report configuration <<<
"@

        Add-Content -Path $profilePath -Value $profileAppend
        Write-Host '✓ Profile updated. New PowerShell sessions will have `report` and `uninstall` available.'
    }
}

Write-Host ''

# ============================================================================
# TEST INSTALLATION
# ============================================================================
Write-Host '=========================================='
Write-Host 'Testing installed TR-200 Machine Report...'
Write-Host '=========================================='
Write-Host ''

try {
    # Prefer pwsh if available, otherwise invoke directly in this host
    if (Get-Command pwsh -ErrorAction SilentlyContinue) {
        pwsh -NoLogo -NoProfile -File $targetScript
    } else {
        # In the current PowerShell process
        . $targetScript
    }
    Write-Host ''
    Write-Host '✓ Test completed.'
} catch {
    Write-Warning 'Test run encountered an error:'
    Write-Warning $_
}

Write-Host ''
Write-Host '=========================================='
Write-Host 'Installation Complete!'
Write-Host '=========================================='
Write-Host ''
Write-Host 'The TR-200 Machine Report will now run:'
Write-Host '  • At Windows login (via Task Scheduler)'
Write-Host '  • On every new PowerShell window'
Write-Host '  • On SSH connections to this machine'
Write-Host ''
Write-Host 'Available commands:'
Write-Host '  report    - Run the machine report manually (PowerShell or CMD)'
Write-Host '  uninstall - Remove TR-200 Machine Report completely'
Write-Host ''
Write-Host 'To disable auto-run but keep the `report` command:'
Write-Host '  • Edit your PowerShell profile and remove the auto-run section.'
Write-Host ''
Write-Host "Install directory: $installDir"
Write-Host "Profile file:      $profilePath"
Write-Host ''
