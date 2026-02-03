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

// All patterns to detect (TR-100 and TR-200 variants)
const ALL_MARKERS = [
    'TR-200 Machine Report (npm)',       // npm-installed
    'TR-200 Machine Report configuration', // install.sh installed
    'TR-200 Machine Report - run on login', // .profile/.zprofile
    'TR-200 Machine Report - run on bash login', // .bash_profile
    'Run Machine Report only when in interactive mode', // TR-100 (original upstream)
    'TR-100 Machine Report',             // TR-100 header
    '# Machine Report alias',            // TR-100 alias marker
];

// Clean a profile file of ALL TR-100/TR-200 entries
function cleanProfileFile(filePath) {
    if (!fs.existsSync(filePath)) return false;

    let content = fs.readFileSync(filePath, 'utf8');
    const originalContent = content;

    // Check if any markers exist
    const hasMarkers = ALL_MARKERS.some(marker => content.includes(marker));
    if (!hasMarkers && !content.includes('machine_report')) {
        return false;
    }

    // Remove npm-style TR-200 blocks (Unix)
    content = content.replace(/\n?# TR-200 Machine Report \(npm\) - auto-run\nif command -v tr200 &> \/dev\/null; then\n    tr200\nfi\n?/g, '');

    // Remove npm-style TR-200 blocks (PowerShell)
    content = content.replace(/\n?# TR-200 Machine Report \(npm\) - auto-run\nif \(Get-Command tr200 -ErrorAction SilentlyContinue\) \{\n    tr200\n\}\n?/g, '');

    // Remove install.sh TR-200 blocks (matches the full block including aliases)
    content = content.replace(/\n?# TR-200 Machine Report configuration\nalias report=.*\nalias uninstall=.*\n\n?# Auto-run on interactive (?:bash|zsh) shell.*\nif \[\[.*\]\]; then\n    clear\n    ~\/.machine_report\.sh\nfi\n?/g, '');

    // Remove TR-200 login blocks (.profile/.zprofile/.bash_profile)
    content = content.replace(/\n?# TR-200 Machine Report - run on (?:login|bash login).*\nif \[ -x "\$HOME\/.machine_report\.sh" \]; then\n    clear\n    "\$HOME\/.machine_report\.sh"\nfi\n?/g, '');

    // Remove TR-100 blocks (original upstream pattern)
    content = content.replace(/\n?# Run Machine Report only when in interactive mode\nif \[\[ \$- == \*i\* \]\]; then\n    ~\/.machine_report\.sh\nfi\n?/g, '');

    // Remove any stray alias lines
    content = content.replace(/\n?alias report=.*machine_report.*\n?/g, '\n');
    content = content.replace(/\n?alias uninstall=.*machine_report.*\n?/g, '\n');

    // Remove TR-100 alias marker
    content = content.replace(/\n?# Machine Report alias\n?/g, '\n');

    // Clean up excessive blank lines (more than 2 consecutive)
    content = content.replace(/\n{3,}/g, '\n\n');
    content = content.trim();

    if (content !== originalContent.trim()) {
        fs.writeFileSync(filePath, content + '\n', 'utf8');
        return true;
    }
    return false;
}

// Shell profile paths for INSTALLATION (minimal set to avoid duplicates)
function getProfilePaths() {
    if (isWindows) {
        return [
            path.join(homeDir, 'Documents', 'PowerShell', 'Microsoft.PowerShell_profile.ps1'),
            path.join(homeDir, 'Documents', 'WindowsPowerShell', 'Microsoft.PowerShell_profile.ps1')
        ];
    } else if (isMac) {
        // macOS: only .zshrc needed (zsh is default since Catalina)
        // SSH login sources .zshrc for interactive sessions
        return [
            path.join(homeDir, '.zshrc')
        ];
    } else {
        // Linux/BSD - only .bashrc needed
        // .profile sources .bashrc on most distros, so adding to both causes duplicates
        return [
            path.join(homeDir, '.bashrc')
        ];
    }
}

// ALL profile paths for CLEANUP (removes duplicates from old installs)
function getAllProfilePaths() {
    if (isWindows) {
        return [
            path.join(homeDir, 'Documents', 'PowerShell', 'Microsoft.PowerShell_profile.ps1'),
            path.join(homeDir, 'Documents', 'WindowsPowerShell', 'Microsoft.PowerShell_profile.ps1')
        ];
    } else {
        // Clean ALL Unix profile files regardless of OS
        return [
            path.join(homeDir, '.bashrc'),
            path.join(homeDir, '.zshrc'),
            path.join(homeDir, '.profile'),
            path.join(homeDir, '.bash_profile'),
            path.join(homeDir, '.zprofile')
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

// Check if npm config already exists in file
function hasConfig(filePath) {
    if (!fs.existsSync(filePath)) return false;
    const content = fs.readFileSync(filePath, 'utf8');
    return content.includes(CONFIG_MARKER);
}

// Check if ANY TR-100/TR-200 config exists (for cleanup detection)
function hasAnyConfig(filePath) {
    if (!fs.existsSync(filePath)) return false;
    const content = fs.readFileSync(filePath, 'utf8');
    return ALL_MARKERS.some(marker => content.includes(marker)) ||
           content.includes('machine_report');
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

    // First, clean up any existing TR-100/TR-200 configurations from ALL profiles
    // This removes duplicates from old installs that used multiple profile files
    console.log('Cleaning previous installations from all profiles...');
    const allProfiles = getAllProfilePaths();
    let cleaned = 0;
    for (const profilePath of allProfiles) {
        if (cleanProfileFile(profilePath)) {
            console.log(`  [cleaned] ${profilePath}`);
            cleaned++;
        }
    }
    if (cleaned > 0) {
        console.log(`  Cleaned ${cleaned} profile(s)`);
    }
    console.log('');

    let installed = 0;
    let skipped = 0;

    for (const profilePath of profiles) {
        // Check if already configured (with npm marker specifically)
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
    // Use ALL profiles for uninstall to ensure complete cleanup
    const profiles = getAllProfilePaths();

    console.log('\nTR-200 Machine Report - Remove Auto-Run\n');
    console.log('This will remove ALL TR-100/TR-200 configurations from your shell profile(s).');
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

        // Use the comprehensive cleanup function
        if (cleanProfileFile(profilePath)) {
            console.log(`  [done] ${profilePath}`);
            removed++;
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
TR-200 Machine Report v2.0.7

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
    console.log('2.0.7');
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
