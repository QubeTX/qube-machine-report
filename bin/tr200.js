#!/usr/bin/env node

/**
 * TR-200 Machine Report - CLI Wrapper
 *
 * Cross-platform Node.js wrapper that detects the OS and runs
 * the appropriate script (bash on Unix, PowerShell on Windows).
 *
 * Copyright 2026, ES Development LLC (https://emmetts.dev)
 * BSD 3-Clause License
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');
const readline = require('readline');

const isWindows = process.platform === 'win32';
const isMac = process.platform === 'darwin';
const homeDir = os.homedir();

// Config block markers for shell profiles
const CONFIG_MARKER = 'TR-200 Machine Report (npm)';
const UNIX_CONFIG = `
# ${CONFIG_MARKER} - auto-run
if command -v tr200 &> /dev/null; then
    tr200
fi
`;
const PS_CONFIG = `
# ${CONFIG_MARKER} - auto-run
if (Get-Command tr200 -ErrorAction SilentlyContinue) {
    tr200
}
`;

// Shell profile paths
function getProfilePaths() {
    if (isWindows) {
        return [
            path.join(homeDir, 'Documents', 'PowerShell', 'Microsoft.PowerShell_profile.ps1'),
            path.join(homeDir, 'Documents', 'WindowsPowerShell', 'Microsoft.PowerShell_profile.ps1')
        ];
    } else if (isMac) {
        return [
            path.join(homeDir, '.zshrc'),
            path.join(homeDir, '.bash_profile')
        ];
    } else {
        // Linux/BSD
        return [
            path.join(homeDir, '.bashrc'),
            path.join(homeDir, '.profile')
        ];
    }
}

// Prompt user for confirmation
function askConfirmation(question) {
    return new Promise((resolve) => {
        const rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout
        });
        rl.question(question, (answer) => {
            rl.close();
            resolve(answer.toLowerCase().startsWith('y'));
        });
    });
}

// Check if config already exists in file
function hasConfig(filePath) {
    if (!fs.existsSync(filePath)) return false;
    const content = fs.readFileSync(filePath, 'utf8');
    return content.includes(CONFIG_MARKER);
}

// Install auto-run to shell profiles
async function installAutoRun() {
    const profiles = getProfilePaths();
    const configBlock = isWindows ? PS_CONFIG : UNIX_CONFIG;

    console.log('\nTR-200 Machine Report - Install Auto-Run\n');
    console.log('This will configure tr200 to run automatically when you open a terminal.');
    console.log('Profile(s) to modify:');
    profiles.forEach(p => console.log(`  - ${p}`));
    console.log('');

    const confirmed = await askConfirmation('Proceed with installation? (y/N): ');
    if (!confirmed) {
        console.log('Installation cancelled.');
        process.exit(0);
    }

    let installed = 0;
    let skipped = 0;

    for (const profilePath of profiles) {
        // Check if already configured
        if (hasConfig(profilePath)) {
            console.log(`  [skip] ${profilePath} - already configured`);
            skipped++;
            continue;
        }

        // Ensure directory exists
        const dir = path.dirname(profilePath);
        if (!fs.existsSync(dir)) {
            fs.mkdirSync(dir, { recursive: true });
        }

        // Append config block
        try {
            fs.appendFileSync(profilePath, configBlock, 'utf8');
            console.log(`  [done] ${profilePath}`);
            installed++;
        } catch (err) {
            console.error(`  [error] ${profilePath}: ${err.message}`);
        }
    }

    console.log('');
    if (installed > 0) {
        console.log(`Success! Auto-run configured in ${installed} profile(s).`);
        console.log('Open a new terminal window to see the report on startup.');
    } else if (skipped === profiles.length) {
        console.log('Auto-run was already configured in all profiles.');
    }
    process.exit(0);
}

// Uninstall auto-run from shell profiles
async function uninstallAutoRun() {
    const profiles = getProfilePaths();

    console.log('\nTR-200 Machine Report - Remove Auto-Run\n');
    console.log('This will remove the auto-run configuration from your shell profile(s).');
    console.log('Profile(s) to check:');
    profiles.forEach(p => console.log(`  - ${p}`));
    console.log('');

    const confirmed = await askConfirmation('Proceed with removal? (y/N): ');
    if (!confirmed) {
        console.log('Removal cancelled.');
        process.exit(0);
    }

    let removed = 0;

    for (const profilePath of profiles) {
        if (!fs.existsSync(profilePath)) {
            continue;
        }

        const content = fs.readFileSync(profilePath, 'utf8');
        if (!content.includes(CONFIG_MARKER)) {
            continue;
        }

        // Remove the config block (handles both Unix and PowerShell formats)
        // Match from the comment line through the closing fi/}
        const unixPattern = /\n?# TR-200 Machine Report \(npm\) - auto-run\nif command -v tr200 &> \/dev\/null; then\n    tr200\nfi\n?/g;
        const psPattern = /\n?# TR-200 Machine Report \(npm\) - auto-run\nif \(Get-Command tr200 -ErrorAction SilentlyContinue\) \{\n    tr200\n\}\n?/g;

        let newContent = content.replace(unixPattern, '');
        newContent = newContent.replace(psPattern, '');

        if (newContent !== content) {
            try {
                fs.writeFileSync(profilePath, newContent, 'utf8');
                console.log(`  [done] ${profilePath}`);
                removed++;
            } catch (err) {
                console.error(`  [error] ${profilePath}: ${err.message}`);
            }
        }
    }

    console.log('');
    if (removed > 0) {
        console.log(`Success! Auto-run removed from ${removed} profile(s).`);
    } else {
        console.log('No auto-run configuration found in any profile.');
    }
    console.log('\nNote: To completely remove tr200, run: npm uninstall -g tr200');
    process.exit(0);
}

// Locate the script files relative to this wrapper
const packageRoot = path.resolve(__dirname, '..');
const bashScript = path.join(packageRoot, 'machine_report.sh');
const psScript = path.join(packageRoot, 'WINDOWS', 'TR-200-MachineReport.ps1');

function runReport() {
    let command, args, scriptPath;

    if (isWindows) {
        scriptPath = psScript;

        if (!fs.existsSync(scriptPath)) {
            console.error(`Error: PowerShell script not found at ${scriptPath}`);
            process.exit(1);
        }

        // Try pwsh (PowerShell 7+) first, fall back to powershell (5.1)
        command = 'pwsh';
        args = ['-ExecutionPolicy', 'Bypass', '-NoProfile', '-File', scriptPath];

        let pwshFailed = false;

        const child = spawn(command, args, {
            stdio: 'inherit',
            shell: false
        });

        child.on('error', (err) => {
            // If pwsh not found, try Windows PowerShell
            if (err.code === 'ENOENT') {
                pwshFailed = true;
                const fallback = spawn('powershell', args, {
                    stdio: 'inherit',
                    shell: false
                });

                fallback.on('error', (fallbackErr) => {
                    console.error('Error: PowerShell not found. Please ensure PowerShell is installed.');
                    process.exit(1);
                });

                fallback.on('close', (code) => {
                    process.exit(code || 0);
                });
            } else {
                console.error(`Error running report: ${err.message}`);
                process.exit(1);
            }
        });

        child.on('close', (code) => {
            // Only handle close if pwsh didn't fail (otherwise fallback handles it)
            if (!pwshFailed) {
                process.exit(code || 0);
            }
        });

    } else {
        // Unix (Linux, macOS, BSD)
        scriptPath = bashScript;

        if (!fs.existsSync(scriptPath)) {
            console.error(`Error: Bash script not found at ${scriptPath}`);
            process.exit(1);
        }

        command = 'bash';
        args = [scriptPath];

        const child = spawn(command, args, {
            stdio: 'inherit',
            shell: false
        });

        child.on('error', (err) => {
            if (err.code === 'ENOENT') {
                console.error('Error: Bash not found. Please ensure bash is installed.');
            } else {
                console.error(`Error running report: ${err.message}`);
            }
            process.exit(1);
        });

        child.on('close', (code) => {
            process.exit(code || 0);
        });
    }
}

// Handle help flag
if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.log(`
TR-200 Machine Report v2.0.1

Usage: tr200 [options]
       report [options]

Displays system information in a formatted table with Unicode box-drawing.

Options:
  -h, --help      Show this help message
  -v, --version   Show version number
  --install       Set up auto-run on terminal/shell startup
  --uninstall     Remove auto-run from shell startup

More info: https://github.com/RealEmmettS/usgc-machine-report
`);
    process.exit(0);
}

// Handle version flag
if (process.argv.includes('--version') || process.argv.includes('-v')) {
    console.log('2.0.2');
    process.exit(0);
}

// Handle install flag
if (process.argv.includes('--install')) {
    installAutoRun();
} else if (process.argv.includes('--uninstall')) {
    // Handle uninstall flag
    uninstallAutoRun();
} else {
    // Run the report
    runReport();
}
