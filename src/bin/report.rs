// Full command alias for TR-300. Delegate every argument to the exact adjacent
// payload so reporting and maintenance use one implementation without PATH lookup.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

#[cfg(windows)]
const REPORT_BINARY: &str = "report.exe";
#[cfg(not(windows))]
const REPORT_BINARY: &str = "report";

#[cfg(windows)]
const TR300_BINARY: &str = "tr300.exe";
#[cfg(not(windows))]
const TR300_BINARY: &str = "tr300";

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();

    let exit_code = match launch_adjacent_tr300(&args[1..]) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("report: {error}");
            1
        }
    };

    process::exit(exit_code);
}

fn launch_adjacent_tr300(args: &[OsString]) -> Result<i32, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("could not resolve the current executable: {error}"))?;
    let tr300 = adjacent_tr300_path(&current_exe)?;

    let metadata = std::fs::metadata(&tr300).map_err(|error| {
        format!(
            "adjacent TR-300 executable is unavailable at {}: {error}",
            tr300.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "adjacent TR-300 executable is not a file: {}",
            tr300.display()
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Replace the launcher so signals and exit status belong directly to
        // tr300, and package updates never encounter a loaded report image.
        let error = Command::new(&tr300).args(args).exec();
        Err(format!("could not launch {}: {error}", tr300.display()))
    }
    #[cfg(not(unix))]
    {
        let status = Command::new(&tr300)
            .args(args)
            .status()
            .map_err(|error| format!("could not launch {}: {error}", tr300.display()))?;
        Ok(status.code().unwrap_or(1))
    }
}

fn adjacent_tr300_path(current_exe: &Path) -> Result<PathBuf, String> {
    if current_exe.file_name() != Some(OsStr::new(REPORT_BINARY)) {
        return Err(format!(
            "refusing to delegate from an unexpected executable name: {}",
            current_exe.display()
        ));
    }

    let parent = current_exe.parent().ok_or_else(|| {
        format!(
            "could not resolve the installation directory for {}",
            current_exe.display()
        )
    })?;
    Ok(parent.join(TR300_BINARY))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_the_exact_adjacent_tr300_payload() {
        let launcher = Path::new("payload").join(REPORT_BINARY);
        assert_eq!(
            adjacent_tr300_path(&launcher).expect("report name should be accepted"),
            Path::new("payload").join(TR300_BINARY)
        );
    }

    #[test]
    fn rejects_a_renamed_launcher() {
        let launcher = Path::new("payload").join("machine-report-copy");
        assert!(adjacent_tr300_path(&launcher).is_err());
    }
}
