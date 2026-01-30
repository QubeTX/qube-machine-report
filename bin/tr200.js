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

const isWindows = process.platform === 'win32';

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

        const child = spawn(command, args, {
            stdio: 'inherit',
            shell: false
        });

        child.on('error', (err) => {
            // If pwsh not found, try Windows PowerShell
            if (err.code === 'ENOENT') {
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
            process.exit(code || 0);
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
TR-200 Machine Report v2.0.0

Usage: tr200 [options]
       report [options]

Displays system information in a formatted table with Unicode box-drawing.

Options:
  -h, --help      Show this help message
  -v, --version   Show version number

More info: https://github.com/RealEmmettS/usgc-machine-report
`);
    process.exit(0);
}

// Handle version flag
if (process.argv.includes('--version') || process.argv.includes('-v')) {
    console.log('2.0.0');
    process.exit(0);
}

// Run the report
runReport();
