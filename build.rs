use clap::CommandFactory;

include!("src/cli.rs");

fn main() {
    let out_dir = match std::env::var("OUT_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => return,
    };

    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();
    man.render(&mut buffer).expect("Failed to render man page");
    let rendered = String::from_utf8(buffer).expect("Generated man page should be UTF-8");
    let normalized = rendered
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let buffer = normalized.into_bytes();

    // Write to OUT_DIR for packaging (authoritative — OUT_DIR is always writable).
    let man_dir = out_dir.join("man");
    std::fs::create_dir_all(&man_dir).ok();
    std::fs::write(man_dir.join("tr300.1"), &buffer).expect("Failed to write man page");

    // Also mirror into the project-root man/ directory for release packaging.
    // Best-effort: a read-only source tree (e.g. a locked-down `cargo install`
    // registry cache, or a sandboxed packaging step) must still build, so a
    // failure here is non-fatal — the OUT_DIR copy above is authoritative.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let project_man = std::path::PathBuf::from(manifest_dir).join("man");
        std::fs::create_dir_all(&project_man).ok();
        std::fs::write(project_man.join("tr300.1"), buffer).ok();
    }
}
