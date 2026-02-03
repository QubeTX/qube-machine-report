# AGENTS.md - TR-200 Machine Report (usgc-machine-report)

This file is a comprehensive, implementation-level description of what this
program is, what machines it is configured to run on, what it does, and how it
is configured to run by default. It is intended for agents and maintainers who
need precise operational context.

---------------------------------------------------------------------------
PROGRAM OVERVIEW
---------------------------------------------------------------------------
Name: TR-200 Machine Report (usgc-machine-report)
Type: Lightweight system information report that renders a fixed, tabular
      "machine report" in the terminal.
Goal: Provide an immediate snapshot of OS, network, CPU, disk, memory, last
      login, and uptime at login time or on demand.
Philosophy: Single-file script per platform, no config files, direct source
            editing for customization.

Core implementations:
1) Unix/macOS: machine_report.sh (bash)
2) Windows: WINDOWS/TR-200-MachineReport.ps1 (PowerShell)

Installers and launchers:
- install.sh: main cross-platform installer for Unix/macOS/Linux.
- install_linux.sh: GUI-friendly Linux launcher for install.sh.
- install_mac.command: macOS Finder-friendly launcher for install.sh.
- WINDOWS/install_windows.ps1: Windows installer for the PowerShell version.
- WINDOWS/build_tr100_exe.ps1: optional helper to build a Windows .exe wrapper.

---------------------------------------------------------------------------
SUPPORTED MACHINES AND ENVIRONMENTS
---------------------------------------------------------------------------
UNIX / LINUX / macOS (machine_report.sh)
Designed for:
- Originally: Debian, ZFS root, AMD EPYC, VMware.
Now supports (as implemented and documented):
- Debian 12/13 (Bookworm/Trixie)
- Ubuntu 24.04+
- Raspberry Pi OS (Debian Trixie) on ARM64
- Arch / Manjaro
- Fedora / RHEL 9
- Alpine Linux (best-effort)
- BSD variants (FreeBSD/OpenBSD/NetBSD) with partial support
- macOS 10.13+ (High Sierra or newer)

Architectures:
- x86_64
- ARM64 (Raspberry Pi)

Filesystems:
- ZFS on Linux (detected automatically)
- Standard Linux/macOS/BSD filesystems: ext4, xfs, btrfs, apfs, etc.

Shell requirements:
- Bash 4+ recommended. Bash 3.2 (macOS default) works with warnings and minor
  formatting differences.

Windows (PowerShell version)
Supported OS:
- Windows 10/11 (and Server equivalents)
Shells:
- Windows PowerShell 5.1
- PowerShell 7+ (pwsh)

Terminal requirements for best visuals:
- Unicode-capable terminal and font (Windows Terminal + Cascadia Code or similar).

---------------------------------------------------------------------------
WHAT THE PROGRAM DOES (FUNCTIONAL OUTPUT)
---------------------------------------------------------------------------
Both implementations render a box-drawn table with a fixed layout. The report
is intended to be shown on login and on-demand.

Common output sections (conceptual):
- Header: report title and "TR-200 MACHINE REPORT"
- OS and kernel
- Network: hostname, machine IP, client IP (SSH), DNS servers, current user
- CPU: model, cores/sockets, hypervisor, frequency, load/usage graphs
- Disk: usage and bar graph (ZFS-aware on Linux; system drive on Windows)
- Memory: usage and bar graph
- Last login (Unix/macOS only) and uptime

Linux/macOS/BSD details (machine_report.sh):
- OS name:
  - macOS: sw_vers
  - Linux: /etc/os-release
  - Fallback: uname
- Kernel: uname -s/-r
- Hostname: hostname -f or hostname or /etc/hosts or uname -n
- Machine IP: ifconfig or ip addr (IPv4 preferred, IPv6 fallback)
- Client IP: who am i (SSH sessions)
- DNS: scutil (macOS) or /etc/resolv.conf (Linux)
- CPU:
  - lscpu (Linux), sysctl (macOS), fallback to nproc/getconf
  - Hypervisor: lscpu "Hypervisor vendor", else "Bare Metal"
  - Frequency: /proc/cpuinfo or sysfs on ARM; sysctl on macOS
- Load averages:
  - /proc/loadavg (Linux)
  - sysctl vm.loadavg (macOS)
  - fallback: uptime parsing
- Memory:
  - /proc/meminfo (Linux)
  - vm_stat + sysctl hw.memsize (macOS)
- Disk:
  - ZFS: uses zpool status -x and zfs get for "zroot/ROOT/os"
  - Non-ZFS: df against root partition (default "/")
- Last login:
  - lastlog2 (preferred) or lastlog (fallback)
- Uptime:
  - uptime -p when available
  - sysctl kern.boottime on macOS
  - fallback: uptime parsing

Windows details (TR-200-MachineReport.ps1):
- OS name and kernel: Win32_OperatingSystem (CIM)
- Hostname: Win32_ComputerSystem
- Machine IP: Get-NetIPAddress (IPv4 preferred, IPv6 fallback), WMI fallback
- DNS: Get-DnsClientServerAddress or WMI
- Client IP: SSH_CLIENT/SSH_CONNECTION env var
- CPU: Win32_Processor
- Hypervisor: Win32_ComputerSystem HypervisorPresent and Model heuristics
- CPU usage: Get-Counter "\Processor(_Total)\% Processor Time"
- Memory: Win32_OperatingSystem (TotalVisibleMemorySize/FreePhysicalMemory)
- Disk: Win32_LogicalDisk for system drive (default C:)
- Uptime: Get-Uptime or LastBootUpTime
- Last login: not implemented on Windows (displays "Login tracking unavailable")

---------------------------------------------------------------------------
DEFAULT CONFIGURATION (UNIX/macOS/BSD)
---------------------------------------------------------------------------
Key defaults in machine_report.sh:
- report_title="UNITED STATES GRAPHICS COMPANY"
- last_login_ip_present=0 (internal flag)
- zfs_present=0 (auto-detected at runtime)
- zfs_filesystem="zroot/ROOT/os"
- root_partition="/" (used when ZFS is not detected)
- Column sizing:
  - MIN_NAME_LEN=5, MAX_NAME_LEN=13
  - MIN_DATA_LEN=20, MAX_DATA_LEN=32
  - BORDERS_AND_PADDING=7

Runtime behavior:
- If lastlog2 is present, it is used; otherwise lastlog is used; if neither is
  present, last login is reported as unavailable.
- If ZFS is detected (Linux with zfs + mounts), ZFS health and usage are shown.
  Otherwise, standard filesystem usage is shown for root_partition.
- If CPU frequency is not available (common on some ARM systems), the field may
  be blank.
- Box-drawing characters are used for table borders and bar graphs.

---------------------------------------------------------------------------
DEFAULT CONFIGURATION (WINDOWS)
---------------------------------------------------------------------------
Key defaults in TR-200-MachineReport.ps1:
- ReportTitle = "UNITED STATES GRAPHICS COMPANY"
- ReportSubtitle = "TR-200 MACHINE REPORT"
- Column sizing:
  - min label width 5, max label width 13
  - min data width 20, max data width 32
- Disk target: system drive from $env:SystemDrive (default C:)
- CPU load bars: based on instantaneous CPU percent from Get-Counter
- Unicode box-drawing characters enabled (tries to set UTF-8 on PowerShell 5.1)

---------------------------------------------------------------------------
HOW IT IS CONFIGURED TO RUN BY DEFAULT
---------------------------------------------------------------------------
Unix/macOS/Linux default installation (install.sh):
1) Copies machine_report.sh to: ~/.machine_report.sh
2) Marks it executable: chmod +x ~/.machine_report.sh
3) Adds to ~/.bashrc:
   - alias report='~/.machine_report.sh'
   - Auto-run on interactive shells:
     if [[ $- == *i* ]]; then
         ~/.machine_report.sh
     fi
4) Runs a test invocation once at install time.

Linux/macOS GUI launchers:
- install_linux.sh and install_mac.command are wrappers that open a terminal
  (if needed) and run install.sh in-place.

Windows default installation (WINDOWS/install_windows.ps1):
1) Copies the PowerShell script to: %USERPROFILE%\TR200\TR-200-MachineReport.ps1
2) Creates a batch shim: %USERPROFILE%\TR200\report.cmd
3) Adds %USERPROFILE%\TR200 to the user PATH
4) Updates the PowerShell profile (CurrentUserAllHosts):
   - Dot-sources TR-200-MachineReport.ps1
   - Defines a report function that calls Show-TR200Report
   - Auto-runs the report for interactive or SSH sessions
5) Executes a test run at the end of installation

Disabling auto-run:
- Unix: remove or comment the auto-run block from ~/.bashrc
- Windows: install with -NoAutoRun or remove the auto-run block in $PROFILE

---------------------------------------------------------------------------
WHAT "RUNS BY DEFAULT" MEANS IN PRACTICE
---------------------------------------------------------------------------
After a standard install:
- Unix/macOS: Opening a new interactive shell or SSH session will immediately
  render the report once, before the prompt. The `report` alias is also available.
- Windows: Opening PowerShell (interactive) or connecting via SSH to the machine
  will render the report once. The `report` command is available in PowerShell
  and Command Prompt due to PATH + report.cmd.

Direct execution behavior:
- Running machine_report.sh directly prints the report once.
- Running TR-200-MachineReport.ps1 directly prints the report once.
- Dot-sourcing the PowerShell script only loads functions (no auto-run) unless
  the profile or a wrapper invokes Show-TR200Report.

---------------------------------------------------------------------------
CUSTOMIZATION MODEL (IMPORTANT)
---------------------------------------------------------------------------
There is intentionally no config file. Customization is done by editing the
source directly:
- Change report_title in machine_report.sh for Unix/macOS.
- Change ReportTitle/ReportSubtitle in TR-200-MachineReport.ps1 for Windows.
- Change zfs_filesystem or root_partition to target different disks.
- Adjust column width constants for layout changes.

This is by design and is part of the project philosophy.

---------------------------------------------------------------------------
REPOSITORY ENTRY POINTS (FILES OF INTEREST)
---------------------------------------------------------------------------
- machine_report.sh                 (Unix/macOS/BSD report generator)
- install.sh                        (primary Unix/macOS installer)
- install_linux.sh                  (GUI-friendly Linux launcher)
- install_mac.command               (GUI-friendly macOS launcher)
- WINDOWS/TR-200-MachineReport.ps1  (Windows report generator)
- WINDOWS/install_windows.ps1       (Windows installer)
- WINDOWS/README_WINDOWS.md         (Windows-specific docs)
- README.md                         (general project documentation)

---------------------------------------------------------------------------
LICENSE
---------------------------------------------------------------------------
BSD 3-Clause License. Original project by U.S. Graphics, LLC. Forked and
extended by RealEmmettS.

