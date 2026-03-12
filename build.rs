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

    // Write to OUT_DIR for packaging
    let man_dir = out_dir.join("man");
    std::fs::create_dir_all(&man_dir).ok();
    std::fs::write(man_dir.join("tr300.1"), &buffer).expect("Failed to write man page");

    // Also write to project root man/ directory for inclusion in releases
    let project_man =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("man");
    std::fs::create_dir_all(&project_man).ok();
    std::fs::write(project_man.join("tr300.1"), buffer).expect("Failed to write man page");
}
