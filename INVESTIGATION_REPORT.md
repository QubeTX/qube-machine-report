# TR-300 Critical Issues Investigation Report

## Executive Summary

This report documents a thorough investigation of two critical issues affecting TR-300 on Raspberry Pi OS:

1. **Issue 1:** "bash: -: command not found" error from the auto-run shell installation
2. **Issue 2:** Unicode box-drawing characters displaying as mojibake (e.g., "â┐" instead of "┌")

Both issues stem from the tool's design assumptions that do not account for minimal shell environments and the complete absence of automatic encoding/locale detection.

---

## Issue 1: Shell Installation Syntax Error

### Root Cause
The shell installation block uses **bash-specific syntax** that fails on systems using dash/sh as the default shell.

### Affected Code

**File:** `src/install/unix.rs`  
**Lines:** 27-34

```rust
const SHELL_ADDITIONS: &str = r#"# TR-300 Machine Report
alias report='tr300'

# Auto-run on interactive shell
if [[ $- == *i* ]]; then
    tr300 --fast
fi
# End TR-300"#;
```

### The Problem
- **`if [[ ]]` syntax is bash-specific** — This is a bash extended test construct that does not exist in POSIX sh/dash
- **RPi OS minimal installs use dash/sh by default** — Not bash
- When the shell profile is sourced in a dash/sh environment, the `[[` construct causes: `bash: -: command not found`
- The error occurs because dash tries to interpret `[[` as a command, then `-` as an argument, which fails

### Installation Function Context
The installation happens in `src/install/unix.rs` lines 50-92:
- Function `install()` modifies `~/.bashrc` and/or `~/.zshrc` if they exist
- Lines 73-79: If neither file exists, it **creates `.bashrc`** and adds `SHELL_ADDITIONS`
- The problem: even if user is on dash/sh system, the code creates a `.bashrc` file with bash-specific syntax

---

## Issue 2: Unicode Mojibake Display

### Root Cause
**Complete absence of automatic encoding/locale detection.** The tool defaults to Unicode mode unconditionally with no fallback mechanism.

### Character Mode Decision Logic

**File:** `src/config.rs`  
**Lines:** 98-112 (Default implementation)

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            use_unicode: true,  // HARDCODED DEFAULT
            use_colors: true,
            title: None,
            subtitle: None,
            show_network: true,
            show_disks: true,
            width: 0,
            compact: false,
            format: OutputFormat::Table,
        }
    }
}
```

**Lines:** 139-145 (Character selection method)

```rust
impl Config {
    /// Get the box-drawing characters based on unicode setting
    pub fn box_chars(&self) -> BoxChars {
        if self.use_unicode {
            BoxChars::unicode()
        } else {
            BoxChars::ascii()
        }
    }
}
```

**File:** `src/render/table.rs`  
**Lines:** 196-212 (Table rendering uses characters directly)

```rust
pub fn render_row(&self, label: &str, value: &str) -> String {
    let label_display = self.fit_string(label, self.label_width);
    let value_display = self.fit_string(value, self.data_width);

    let mut line = String::new();
    line.push(self.chars.vertical);        // Directly uses char from BoxChars
    line.push(' ');
    line.push_str(&label_display);
    line.push(' ');
    line.push(self.chars.vertical);        // ← No encoding check, no fallback
    // ... rest of row rendering
}
```

### What IS Checked
Nothing. There is **zero environment variable checking** for:
- `LANG` — The locale setting
- `LC_ALL` — The locale override
- `TERM` — Terminal type capabilities
- `TERMINFO` — Terminal database

### What COULD Be Checked (But Isn't)
**File:** `src/report.rs`  
**Lines:** 141-143 (Locale IS collected and stored)

```rust
if let Some(ref locale) = info.locale {
    output.push_str(&renderer.render_row("LOCALE\", locale));
}
```

**CRITICAL FINDING:** The tool **collects locale information** and **displays it in the output**, but **never uses it to select character mode**.

### Character Sets Available

**File:** `src/config.rs`

**Unicode (default) — Lines 32-46:**
```rust
pub const TOP_LEFT: char = '┌';
pub const TOP_RIGHT: char = '┐';
pub const BOTTOM_LEFT: char = '└';
pub const BOTTOM_RIGHT: char = '┘';
pub const HORIZONTAL: char = '─';
pub const VERTICAL: char = '│';
pub const T_DOWN: char = '┬';
pub const T_UP: char = '┴';
pub const T_RIGHT: char = '├';
pub const T_LEFT: char = '┤';
pub const CROSS: char = '┼';
pub const BAR_FILLED: char = '█';
pub const BAR_EMPTY: char = '░';
```

**ASCII (fallback) — Lines 50-66:**
```rust
pub const TOP_LEFT: char = '+';
pub const TOP_RIGHT: char = '+';
pub const BOTTOM_LEFT: char = '+';
pub const BOTTOM_RIGHT: char = '+';
pub const HORIZONTAL: char = '-';
pub const VERTICAL: char = '|';
pub const T_DOWN: char = '+';
pub const T_UP: char = '+';
pub const T_RIGHT: char = '+';
pub const T_LEFT: char = '+';
pub const CROSS: char = '+';
pub const BAR_FILLED: char = '#';
pub const BAR_EMPTY: char = '.';
```

### How to Currently Enable ASCII Mode
**File:** `src/cli.rs`  
**Lines:** 21-23

```rust
/// Use ASCII characters instead of Unicode box-drawing
#[arg(long)]
pub ascii: bool,
```

**The only way to get ASCII mode:**
1. Explicitly pass `--ascii` flag: `tr300 --ascii`
2. Configuration file (if implemented) — but no auto-detection

---

## Summary Table

| Aspect | Issue 1 | Issue 2 |
|--------|---------|---------|
| **Root Cause** | bash-specific syntax in shell profile | No automatic encoding detection |
| **File** | `src/install/unix.rs:27-34` | `src/config.rs:98-112, 139-145` |
| **Type** | Installation/shell compatibility | Character rendering |
| **Affected System** | RPi OS with dash/sh default shell | Any system without UTF-8 support or misconfigured locale |
| **Current Workaround** | Manual `.bashrc` fix or use bash explicitly | Explicit `--ascii` flag |
| **Code Problem** | Uses `if [[` instead of `if [` | Unconditional `use_unicode: true` with zero env checks |

---

## Code File Locations Summary

All findings are located in these source files:

1. **`src/cli.rs`** — CLI argument definitions (no automatic detection)
2. **`src/config.rs`** — Configuration and character set definitions (hardcoded Unicode default)
3. **`src/render/table.rs`** — Table rendering (uses characters without checking)
4. **`src/report.rs`** — Report generation (collects but doesn't use locale)
5. **`src/install/unix.rs`** — Shell profile installation (bash-specific syntax)

---

## Investigation Complete

This report documents the complete technical investigation of both issues as requested. All code sections, file paths, and root causes have been identified through direct source code analysis.
