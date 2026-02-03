# TR-200 Machine Report: Rust Rewrite Feasibility Analysis

> **Document Status**: Planning Phase
> **Date**: February 2, 2026
> **Purpose**: Analyze the feasibility of rewriting TR-200 Machine Report as a native Rust application

---

## Executive Summary

**Verdict: Highly Feasible**

Rewriting TR-200 Machine Report in Rust is not only feasible but advantageous. The Rust ecosystem provides mature crates for all required functionality, and a Rust implementation would deliver:

- **Single-binary distribution** (no Node.js, Bash, or PowerShell runtime dependencies)
- **True cross-platform support** from a unified codebase
- **Faster startup and execution**
- **Smaller deployment footprint**
- **Improved reliability** through compile-time guarantees

---

## Part 1: Current Implementation Analysis

### Features to Replicate

The original TR-200 collects and displays:

| Category | Information | Unix Method | Windows Method |
|----------|-------------|-------------|----------------|
| **OS** | Name, version, kernel | `/etc/os-release`, `sw_vers`, `uname` | `Win32_OperatingSystem` WMI |
| **CPU** | Model, cores, sockets, frequency | `lscpu`, `sysctl`, `/proc/cpuinfo` | `Win32_Processor` WMI |
| **CPU Load** | 1m, 5m, 15m averages | `/proc/loadavg`, `sysctl vm.loadavg` | Performance counters (instantaneous) |
| **Memory** | Total, used, available, % | `/proc/meminfo`, `vm_stat` | `Win32_OperatingSystem` WMI |
| **Disk** | Total, used, %, ZFS support | `df`, `zpool`/`zfs` | `Win32_LogicalDisk` WMI |
| **Network** | Hostname, IP, DNS servers | `hostname`, `ifconfig`/`ip`, `/etc/resolv.conf` | `Get-NetIPAddress`, `Win32_NetworkAdapterConfiguration` |
| **Session** | User, client IP (SSH) | `whoami`, `$SSH_CLIENT` | `[Environment]::UserName`, `$env:SSH_CLIENT` |
| **Activity** | Last login, uptime | `lastlog`/`lastlog2`, `uptime` | WMI boot time (no last login) |

### Output Format

- Unicode box-drawing table (`┌ ┐ └ ┘ ├ ┤ ─ │`)
- Bar graphs using block characters (`█ ░`)
- Dynamic column sizing (5-13 char labels, 20-32 char data)
- Consistent appearance across all platforms

### Current Distribution Complexity

The original uses:
- 1 Bash script (machine_report.sh) - 514 lines
- 1 PowerShell script (TR-200-MachineReport.ps1) - ~400 lines
- 1 Node.js CLI wrapper (tr200.js) - 401 lines
- 3 installation scripts (Unix, Linux GUI, macOS) - 827 lines
- 1 Windows installer (PowerShell) - 551 lines
- **Total: ~2,700 lines across 7+ files and 4 languages**

A Rust rewrite could consolidate this into a single binary.

---

## Part 2: Rust Ecosystem Assessment

### Core System Information

**Primary Crate: `sysinfo`**
- **Status**: Mature, actively maintained, 6,000+ GitHub stars
- **Platforms**: Windows, macOS, Linux, FreeBSD, Android, iOS
- **Provides**:
  - CPU: model, vendor, core count, frequency, usage per core
  - Memory: total, available, used, swap
  - Disks: mount points, total/available space, filesystem type
  - Network: interfaces, received/transmitted bytes
  - Processes: full process list with details
  - System: hostname, uptime, boot time, OS name/version

**Coverage Assessment**:
| Feature | `sysinfo` Coverage | Gap? |
|---------|-------------------|------|
| OS name/version | ✅ Full | No |
| Kernel version | ✅ Full | No |
| CPU model | ✅ Full | No |
| CPU cores | ✅ Full | No |
| CPU frequency | ✅ Full | No |
| CPU load averages | ⚠️ Partial | Linux/macOS: Yes, Windows: No (use alternative) |
| Memory stats | ✅ Full | No |
| Disk usage | ✅ Full | No |
| Hostname | ✅ Full | No |
| Uptime | ✅ Full | No |
| Network interfaces | ✅ Full | No |
| IP addresses | ⚠️ Partial | Interface data available, need IP extraction |

### Platform-Specific Gaps and Solutions

#### 1. CPU Load Averages (Windows)
**Gap**: Windows doesn't have Unix-style 1m/5m/15m load averages.

**Solutions**:
- **Option A**: Calculate running averages from CPU usage samples (like the original does)
- **Option B**: Show instantaneous CPU % (current Windows behavior)
- **Option C**: Use Windows performance counters for historical data

**Recommended**: Option B (matches original Windows behavior)

#### 2. ZFS Support (Linux)
**Gap**: `sysinfo` doesn't natively query ZFS pools.

**Solutions**:
- **Option A**: Shell out to `zfs`/`zpool` commands (like original)
- **Option B**: Use `libzfs` FFI bindings (more complex)
- **Option C**: Read from `/proc/spl/kstat/zfs` (Linux only)

**Recommended**: Option A initially, consider Option C for deeper integration

**Crate**: None needed, use `std::process::Command`

#### 3. Hypervisor Detection
**Gap**: Not provided by `sysinfo`.

**Solutions**:
- **Linux**: Parse `lscpu` output or read `/sys/hypervisor/type`
- **macOS**: Typically bare metal (hardcode or check for VM signatures)
- **Windows**: Query `Win32_ComputerSystem.HypervisorPresent` via WMI

**Crate for Windows**: `wmi` (excellent Rust WMI bindings)

#### 4. DNS Server Detection
**Gap**: Not provided by `sysinfo`.

**Solutions**:
- **Linux**: Parse `/etc/resolv.conf`
- **macOS**: Parse `scutil --dns` output
- **Windows**: Query `Get-DnsClientServerAddress` or WMI

**Implementation**: File parsing + optional command execution

#### 5. Last Login Information
**Gap**: Not provided by `sysinfo`.

**Solutions**:
- **Linux**: Parse `lastlog` binary file or call `lastlog`/`lastlog2`
- **macOS**: Parse `last` command output
- **Windows**: Not available (match original behavior: "Login tracking unavailable")

**Recommended**: Shell out to `lastlog`/`lastlog2` initially

#### 6. Client IP (SSH Session Detection)
**Gap**: Not a system property, it's an environment variable.

**Solution**: Read `SSH_CLIENT` or `SSH_CONNECTION` environment variables
```rust
std::env::var("SSH_CLIENT").ok()
```

**Implementation**: Trivial

### Additional Crates Needed

| Crate | Purpose | Maturity |
|-------|---------|----------|
| `sysinfo` | Primary system information | ⭐⭐⭐⭐⭐ Excellent |
| `wmi` | Windows WMI queries | ⭐⭐⭐⭐ Very Good |
| `crossterm` | Terminal manipulation, colors | ⭐⭐⭐⭐⭐ Excellent |
| `clap` | CLI argument parsing | ⭐⭐⭐⭐⭐ Excellent |
| `dirs` | Platform-specific directories | ⭐⭐⭐⭐⭐ Excellent |
| `thiserror` | Error handling | ⭐⭐⭐⭐⭐ Excellent |

---

## Part 3: Feature-by-Feature Implementation Plan

### Tier 1: Core Features (Week 1-2)

These can be implemented almost entirely with `sysinfo`:

| Feature | Implementation | Difficulty |
|---------|---------------|------------|
| OS name/version | `sysinfo::System::name()`, `os_version()` | Easy |
| Kernel version | `sysinfo::System::kernel_version()` | Easy |
| CPU model | `sysinfo::System::cpus()[0].brand()` | Easy |
| CPU core count | `sysinfo::System::cpus().len()` | Easy |
| CPU frequency | `sysinfo::System::cpus()[0].frequency()` | Easy |
| Memory stats | `sysinfo::System::total_memory()`, `used_memory()` | Easy |
| Disk usage | `sysinfo::Disks::list()` | Easy |
| Hostname | `sysinfo::System::host_name()` | Easy |
| Uptime | `sysinfo::System::uptime()` | Easy |
| Current user | `std::env::var("USER")` or `whoami` crate | Easy |
| SSH client IP | `std::env::var("SSH_CLIENT")` | Trivial |

### Tier 2: Platform-Specific Features (Week 2-3)

| Feature | Platform | Implementation | Difficulty |
|---------|----------|---------------|------------|
| Load averages | Linux/macOS | Parse `/proc/loadavg` or `sysctl` | Easy |
| Load averages | Windows | CPU % sample (current behavior) | Easy |
| DNS servers | Linux | Parse `/etc/resolv.conf` | Easy |
| DNS servers | macOS | Parse `scutil --dns` output | Medium |
| DNS servers | Windows | WMI query | Medium |
| Machine IP | All | Filter network interfaces | Medium |
| Hypervisor | Linux | Parse `/sys/hypervisor/type` or `lscpu` | Easy |
| Hypervisor | Windows | WMI `HypervisorPresent` | Easy |

### Tier 3: Advanced Features (Week 3-4)

| Feature | Platform | Implementation | Difficulty |
|---------|----------|---------------|------------|
| ZFS support | Linux | Shell out to `zfs`/`zpool` | Medium |
| Last login | Linux | Shell out to `lastlog2`/`lastlog` | Medium |
| Last login | macOS | Parse `last` command | Medium |
| Socket count | Linux | Parse `lscpu` | Easy |
| Socket count | Windows | WMI `SocketDesignation` | Easy |

### Tier 4: Output & Distribution (Week 4-5)

| Task | Implementation | Difficulty |
|------|---------------|------------|
| Box-drawing table | Custom render function | Easy |
| Bar graphs | String concatenation | Trivial |
| Dynamic column sizing | Calculate max lengths | Easy |
| CLI arguments | `clap` crate | Easy |
| Cross-compilation | GitHub Actions + cargo | Medium |
| Installation script | Self-extracting or shell scripts | Medium |

---

## Part 4: Technical Challenges and Mitigations

### Challenge 1: Windows WMI Access

**Issue**: Some WMI queries require specific permissions or may fail on restricted systems.

**Mitigation**:
- Use `wmi` crate with proper error handling
- Implement graceful fallbacks (like original does)
- Test on various Windows configurations

**Example**:
```rust
use wmi::{COMLibrary, WMIConnection};

fn get_hypervisor() -> Option<String> {
    let com = COMLibrary::new().ok()?;
    let wmi = WMIConnection::new(com).ok()?;

    #[derive(Deserialize)]
    struct ComputerSystem {
        HypervisorPresent: bool,
    }

    let results: Vec<ComputerSystem> = wmi.query().ok()?;
    results.first().map(|cs| {
        if cs.HypervisorPresent { "Virtualized" } else { "Bare Metal" }.to_string()
    })
}
```

### Challenge 2: Cross-Platform Compilation

**Issue**: Building for Windows, macOS, and Linux from a single CI.

**Mitigation**:
- Use GitHub Actions with `cross` for Linux ARM
- Native builds on Windows/macOS runners
- Static linking where possible (`musl` for Linux)

**GitHub Actions Matrix**:
```yaml
strategy:
  matrix:
    include:
      - os: ubuntu-latest
        target: x86_64-unknown-linux-musl
      - os: ubuntu-latest
        target: aarch64-unknown-linux-musl
      - os: macos-latest
        target: x86_64-apple-darwin
      - os: macos-latest
        target: aarch64-apple-darwin
      - os: windows-latest
        target: x86_64-pc-windows-msvc
```

### Challenge 3: Terminal Encoding on Windows

**Issue**: Windows terminals may not display Unicode box-drawing characters correctly.

**Mitigation**:
- Use `crossterm` for terminal capability detection
- Set console mode to enable UTF-8 on Windows
- Provide ASCII fallback mode (`--ascii` flag)

```rust
#[cfg(windows)]
fn enable_utf8() {
    use std::os::windows::io::AsRawHandle;
    use winapi::um::consoleapi::SetConsoleOutputCP;
    unsafe { SetConsoleOutputCP(65001); } // UTF-8
}
```

### Challenge 4: ZFS Detection Without Root

**Issue**: ZFS commands may require elevated privileges on some systems.

**Mitigation**:
- Check if `zfs` command exists and is accessible
- Use `zfs get` with `-H` (scripting mode) to avoid permission issues
- Fall back to standard `df` if ZFS queries fail

### Challenge 5: Distribution and Auto-Run

**Issue**: Users expect auto-run on shell startup, but Rust binaries don't self-install.

**Mitigation Options**:
1. **Separate install script**: Small shell/PowerShell scripts that add the binary to PATH and profile
2. **Built-in install command**: `qube-report --install` modifies shell profiles
3. **Package managers**: Publish to Homebrew, Chocolatey, AUR, cargo

**Recommended**: Option 2 + Option 3 (built-in install with package manager support)

---

## Part 5: Architecture Recommendation

### Proposed Structure

```
qube-machine-report/
├── Cargo.toml
├── src/
│   ├── main.rs                 # CLI entry point
│   ├── lib.rs                  # Library exports
│   ├── config.rs               # Configuration and defaults
│   ├── report.rs               # Report generation orchestration
│   ├── render/
│   │   ├── mod.rs
│   │   ├── table.rs            # Unicode table rendering
│   │   └── bar.rs              # Bar graph rendering
│   ├── collectors/
│   │   ├── mod.rs
│   │   ├── os.rs               # OS information
│   │   ├── cpu.rs              # CPU information
│   │   ├── memory.rs           # Memory information
│   │   ├── disk.rs             # Disk/storage information
│   │   ├── network.rs          # Network information
│   │   ├── session.rs          # User/login information
│   │   └── platform/
│   │       ├── mod.rs
│   │       ├── linux.rs        # Linux-specific collectors
│   │       ├── macos.rs        # macOS-specific collectors
│   │       └── windows.rs      # Windows-specific collectors
│   └── install/
│       ├── mod.rs
│       ├── unix.rs             # Unix installation logic
│       └── windows.rs          # Windows installation logic
├── tests/
│   └── integration/
├── .github/
│   └── workflows/
│       └── release.yml         # Cross-platform build and release
└── README.md
```

### Key Design Principles

1. **Trait-based collectors**: Each info category implements a `Collector` trait
2. **Platform abstraction**: Platform-specific code isolated in `platform/` modules
3. **Graceful degradation**: Every collector returns `Option<T>` or `Result<T, E>`
4. **Zero required dependencies**: Core functionality works without external commands
5. **Optional enhancements**: ZFS, last login, etc. only attempt if commands exist

### Example Trait Design

```rust
pub trait Collector {
    type Output;
    fn collect(&self) -> Result<Self::Output, CollectorError>;
    fn name(&self) -> &'static str;
}

pub struct CpuCollector {
    system: sysinfo::System,
}

impl Collector for CpuCollector {
    type Output = CpuInfo;

    fn collect(&self) -> Result<CpuInfo, CollectorError> {
        Ok(CpuInfo {
            model: self.system.cpus().first()
                .map(|c| c.brand().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            cores: self.system.cpus().len(),
            frequency_ghz: self.system.cpus().first()
                .map(|c| c.frequency() as f64 / 1000.0),
            // ...
        })
    }

    fn name(&self) -> &'static str { "CPU" }
}
```

---

## Part 6: Risk Assessment

### Low Risk (Straightforward Implementation)

| Feature | Confidence | Notes |
|---------|------------|-------|
| OS/kernel info | 99% | Fully supported by `sysinfo` |
| CPU info (model, cores, freq) | 99% | Fully supported by `sysinfo` |
| Memory stats | 99% | Fully supported by `sysinfo` |
| Disk usage | 95% | Supported, may need filtering |
| Hostname | 99% | Fully supported |
| Uptime | 99% | Fully supported |
| Unicode output | 99% | Standard Rust strings |
| CLI args | 99% | `clap` is excellent |

### Medium Risk (Requires Platform-Specific Code)

| Feature | Confidence | Notes |
|---------|------------|-------|
| Load averages | 85% | Need platform-specific parsing |
| DNS servers | 80% | Need file/command parsing |
| IP address filtering | 85% | Need to filter loopback/docker |
| Hypervisor detection | 80% | WMI on Windows, file parsing on Linux |
| Socket count | 75% | Not in `sysinfo`, need `lscpu`/WMI |

### Higher Risk (Complex or External Dependencies)

| Feature | Confidence | Notes |
|---------|------------|-------|
| ZFS support | 70% | Requires shelling out, may fail |
| Last login | 65% | Platform-specific, may require commands |
| Windows terminal encoding | 75% | Historical compatibility issues |
| Auto-install to shell profile | 70% | Need to handle many shell types |

---

## Part 7: Recommended Development Phases

### Phase 1: Core MVP (Weeks 1-2)
- Basic system info via `sysinfo`
- Unicode table output
- CLI with `--help` and `--version`
- Works on all platforms with base features

### Phase 2: Feature Parity (Weeks 3-4)
- Platform-specific collectors (DNS, hypervisor, load avg)
- ZFS support (Linux)
- Last login (Linux/macOS)
- Full output matching original

### Phase 3: Distribution (Week 5)
- GitHub Actions cross-compilation
- `--install` and `--uninstall` commands
- Package manager submissions (Homebrew, Chocolatey)
- Binary releases

### Phase 4: Polish (Week 6+)
- ASCII fallback mode
- Configuration file support
- Color themes
- Additional output formats (JSON, plain text)

---

## Part 8: Conclusion

### Feasibility Rating: **9/10**

A Rust rewrite of TR-200 Machine Report is highly feasible and would result in a superior product:

**Advantages**:
- Single binary, no runtime dependencies
- Faster startup (~10ms vs ~100ms+)
- Better error handling and reliability
- Easier distribution (one file per platform)
- Modern tooling (cargo, clippy, rustfmt)
- Memory safety guarantees
- Smaller binary size (~2-5MB stripped)

**Challenges**:
- Platform-specific code requires careful testing
- Some features need shell command fallbacks
- Windows terminal compatibility needs attention
- Auto-install feature requires shell script generation

**Recommendation**: Proceed with the Rust rewrite. Start with the core `sysinfo`-based implementation, then layer in platform-specific features. The resulting tool will be easier to maintain, faster to run, and simpler to distribute than the current multi-language implementation.

---

## Appendix A: Crate Versions (as of Feb 2026)

```toml
[dependencies]
sysinfo = "0.32"          # System information
clap = { version = "4.5", features = ["derive"] }  # CLI
crossterm = "0.28"        # Terminal manipulation
dirs = "5.0"              # Platform directories
thiserror = "2.0"         # Error handling

[target.'cfg(windows)'.dependencies]
wmi = "0.14"              # Windows WMI
winapi = { version = "0.3", features = ["consoleapi"] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"              # Unix system calls (optional)
```

## Appendix B: Research Items for Implementation

### Items Requiring Further Investigation

1. **`sysinfo` CPU socket detection**: Verify if available or needs `lscpu`
2. **macOS ARM frequency**: Check if `sysinfo` provides on Apple Silicon
3. **Windows load average alternatives**: Research PDH counters for historical CPU data
4. **ZFS on macOS**: Determine if OpenZFS macOS support is needed
5. **Raspberry Pi quirks**: Test ARM frequency detection via sysfs
6. **BSD support level**: Verify FreeBSD/OpenBSD `sysinfo` compatibility

### Commands to Test During Development

```bash
# Linux ZFS check
zpool list -H -o name,health 2>/dev/null
zfs get -o value -Hp available,used zroot/ROOT/os

# Linux hypervisor
cat /sys/hypervisor/type 2>/dev/null
systemd-detect-virt

# macOS DNS
scutil --dns | grep nameserver

# Windows hypervisor (PowerShell)
(Get-CimInstance Win32_ComputerSystem).HypervisorPresent
```

---

*Document prepared by Claude Code for qube-machine-report project planning.*
