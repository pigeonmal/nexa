use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static WORKSPACE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Returns the absolute path to the Nexa repository workspace root.
pub fn workspace_root() -> &'static Path {
    WORKSPACE_ROOT.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // nexa-testkit is in crates/nexa-testkit, so root is ../..
        manifest
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or(manifest)
    })
}

/// Resolves a path under the `examples/` directory.
pub fn example_path(name: &str) -> PathBuf {
    workspace_root().join("examples").join(name)
}

/// Resolves a fixture path, looking under `tests/fixtures/` and `crates/nexa-cli/tests/fixtures/`.
pub fn fixture_path(name: &str) -> PathBuf {
    let root = workspace_root();
    let candidates = [
        root.join("tests/fixtures").join(name),
        root.join("crates/nexa-cli/tests/fixtures").join(name),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }
    candidates[0].clone()
}

/// Reads the contents of a fixture file as a UTF-8 string.
pub fn read_fixture(name: &str) -> String {
    let path = fixture_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture {}: {err}", path.display()))
}
