use clap::CommandFactory;

include!("src/cli.rs");

fn render_man_page(command: clap::Command, see_also: &str) -> Vec<u8> {
    let man = clap_mangen::Man::new(command);
    let mut buffer = Vec::new();
    man.render(&mut buffer).expect("Failed to render man page");
    let rendered = String::from_utf8(buffer).expect("Generated man page should be UTF-8");
    let mut normalized = rendered
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    normalized = normalized.replace(
        "Save this full table report as Markdown in Downloads",
        "Save this full table report as Markdown in Downloads (aliases: \\-s, \\-\\-save)",
    );
    normalized.push_str("\n.SH SEE ALSO\n");
    normalized.push_str(see_also);
    normalized.push_str("(1)\n");
    normalized.into_bytes()
}

fn main() {
    let out_dir = match std::env::var("OUT_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => return,
    };

    let pages = [
        ("tr300.1", render_man_page(Cli::command(), "report")),
        (
            "report.1",
            render_man_page(Cli::command().name("report"), "tr300"),
        ),
    ];

    // Write to OUT_DIR for packaging (authoritative — OUT_DIR is always writable).
    let man_dir = out_dir.join("man");
    std::fs::create_dir_all(&man_dir).ok();
    for (name, page) in &pages {
        std::fs::write(man_dir.join(name), page).expect("Failed to write man page");
    }

    // Also mirror into the project-root man/ directory for release packaging.
    // Best-effort: a read-only source tree (e.g. a locked-down `cargo install`
    // registry cache, or a sandboxed packaging step) must still build, so a
    // failure here is non-fatal — the OUT_DIR copy above is authoritative.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let project_man = std::path::PathBuf::from(manifest_dir).join("man");
        std::fs::create_dir_all(&project_man).ok();
        for (name, page) in &pages {
            std::fs::write(project_man.join(name), page).ok();
        }
    }
}
