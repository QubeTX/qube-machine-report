//! Cross-platform constants and helpers for the install / uninstall
//! flow.
//!
//! Before v3.15.3 the marker text and the block-removal parser were
//! duplicated byte-for-byte across `unix.rs` and `windows.rs`. This
//! module is the single source of truth — both platform modules import
//! these constants and the parser by reference so a future rename
//! touches one place, not four.

/// Opening line of the TR-300 block injected into rc files. The full
/// literal string must appear on a line of the `SHELL_ADDITIONS` /
/// `POWERSHELL_ADDITIONS` snippet in each platform module — the
/// `shell_additions_contains_shared_markers` tests in each module
/// pin this contract.
pub(crate) const MARKER_START: &str = "# TR-300 Machine Report";

/// Closing line of the TR-300 block. The hand-edit hazard this module
/// guards against (see `install::check_marker_balance`) is the user
/// removing exactly this line — without which the block parser would
/// silently drop everything from `MARKER_START` to EOF.
pub(crate) const MARKER_END: &str = "# End TR-300";

// The three constants below are referenced only from per-platform
// snippet-content tests, not from production code (the snippets are
// raw string literals that inline the same values). They live here
// so any future rename touches one place — tests then pin the
// snippet's contract that it matches these canonical values.

/// User-facing convenience command. Wired up as a `Set-Alias` on
/// Windows and an `alias` on POSIX shells.
#[allow(dead_code)]
pub(crate) const ALIAS_NAME: &str = "report";

/// Crate / binary name. Same as `Cargo.toml` `name` and the `[[bin]]`
/// `name` field.
#[allow(dead_code)]
pub(crate) const BINARY_NAME: &str = "tr300";

/// Process-scoped env var set by the auto-run snippet on first
/// invocation in an interactive shell. Child processes (`bash -i -c
/// ...`, `pwsh -Command ...`, vim's `:term`, a Makefile's nested
/// shell) inherit it and the snippet's guard short-circuits — no
/// recursion, no table render into a CI log.
#[allow(dead_code)]
pub(crate) const AUTORUN_SENTINEL_VAR: &str = "TR300_AUTORUN_RAN";

/// Walk `lines` and return everything OUTSIDE the marker block.
///
/// The caller is responsible for verifying marker balance via
/// `install::check_marker_balance` BEFORE invoking this function. With
/// balance verified, a well-formed file has 0 or more clean
/// `START ... END` blocks and this safely drops them. Without that
/// pre-check, a hand-edited rc file missing `MARKER_END` would silently
/// lose every line from `MARKER_START` to EOF.
pub(crate) fn remove_delimited_block<'a>(
    lines: &[&'a str],
    start: &str,
    end: &str,
) -> Vec<&'a str> {
    let mut result = Vec::new();
    let mut in_block = false;

    for line in lines {
        if line.contains(start) {
            in_block = true;
            continue;
        }
        if line.contains(end) {
            in_block = false;
            continue;
        }
        if !in_block {
            result.push(*line);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{remove_delimited_block, MARKER_END, MARKER_START};

    #[test]
    fn removes_a_single_well_formed_block() {
        let lines: Vec<&str> = vec![
            "export FOO=bar",
            "",
            MARKER_START,
            "alias report='tr300'",
            MARKER_END,
            "alias ll='ls -la'",
        ];
        let kept = remove_delimited_block(&lines, MARKER_START, MARKER_END);
        assert_eq!(kept, vec!["export FOO=bar", "", "alias ll='ls -la'"]);
    }

    #[test]
    fn removes_two_blocks() {
        // Pathological but balanced — verify both blocks are stripped.
        let lines: Vec<&str> = vec![
            MARKER_START,
            "block A",
            MARKER_END,
            "between",
            MARKER_START,
            "block B",
            MARKER_END,
        ];
        let kept = remove_delimited_block(&lines, MARKER_START, MARKER_END);
        assert_eq!(kept, vec!["between"]);
    }

    #[test]
    fn passes_through_a_clean_file() {
        let lines: Vec<&str> = vec!["export FOO=bar", "alias ll='ls -la'"];
        let kept = remove_delimited_block(&lines, MARKER_START, MARKER_END);
        assert_eq!(kept, vec!["export FOO=bar", "alias ll='ls -la'"]);
    }
}
