use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

static NEXA_BIN: OnceLock<PathBuf> = OnceLock::new();

/// Returns the path to the compiled `nexa` CLI binary.
pub fn nexa_bin() -> &'static Path {
    NEXA_BIN.get_or_init(|| {
        if let Ok(bin) = std::env::var("CARGO_BIN_EXE_nexa") {
            let path = PathBuf::from(bin);
            if path.exists() {
                return path;
            }
        }
        let root = crate::fixtures::workspace_root();
        let debug_bin = root.join("target/debug/nexa");
        if debug_bin.exists() {
            return debug_bin;
        }
        let release_bin = root.join("target/release/nexa");
        if release_bin.exists() {
            return release_bin;
        }
        PathBuf::from("nexa")
    })
}

/// Creates a new `Command` invoking the `nexa` CLI binary.
pub fn nexa_command() -> Command {
    Command::new(nexa_bin())
}

/// Invokes `nexa create <name> --directory <project_dir>`.
pub fn run_nexa_create(project_dir: &Path, name: &str) -> Output {
    nexa_command()
        .args(["create", name, "--directory"])
        .arg(project_dir)
        .output()
        .expect("run nexa create")
}

/// Invokes `nexa check` in `project_dir`.
pub fn run_nexa_check(project_dir: &Path) -> Output {
    nexa_command()
        .arg("check")
        .current_dir(project_dir)
        .output()
        .expect("run nexa check")
}
