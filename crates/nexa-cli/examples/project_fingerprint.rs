//! Byte-identity harness for whole native host project generation.
//!
//! `app_fingerprint` covers the Swift/Kotlin source units; `bridge_fingerprint`
//! covers the plugin bridge. Neither covers what the CLI actually writes to
//! disk: the Xcode project, Gradle scripts, manifests, plists, entitlements,
//! copied plugin artifacts, and the project manifest itself.
//!
//! This drives the real `generate_project` entry point for every shipped
//! example, then prints a digest per generated file plus a combined digest.
//! Refactors of `project.rs` / `project/plugins.rs` / `project/templates.rs`
//! are only safe with something like this in place, because those files mix
//! decisions, filesystem mutation, and text generation.
//!
//! Generated projects embed absolute paths (the output directory, plugin
//! package roots, resolved source globs), so those are normalized to stable
//! placeholders before hashing. Without that, every run in a fresh temporary
//! directory would produce different digests.
//!
//! ```text
//! cargo run -p nexa-cli --example project_fingerprint            # digests
//! NEXA_FP_DUMP=1 cargo run -p nexa-cli --example project_fingerprint  # contents
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Fixtures relative to the workspace `examples/` directory.
const FIXTURES: [(&str, &str); 7] = [
    ("counter.nx", "Counter"),
    ("navigation.nx", "NavigationDemo"),
    ("showcase.nx", "ComponentShowcase"),
    ("todo_app.nx", "TodoApp"),
    ("virtual_list.nx", "VirtualListApp"),
    ("plugins/video-player-demo.nx", "NexaPluginBuildTest"),
    ("plugins/video-player-route-sharing.nx", "NexaPluginBuildTest"),
];

/// Fixtures also generated through the dev host entry point, which selects a
/// different backend path (`generate_for_dev`), adds the dev runtime unit, and
/// writes an Android debug manifest. Fixed values keep the output stable.
const DEV_FIXTURES: [(&str, &str); 2] =
    [("counter.nx", "Counter"), ("virtual_list.nx", "VirtualListApp")];

const DEV_SERVER_URL: &str = "ws://127.0.0.1:5173";
const DEV_SESSION_TOKEN: &str = "nexa-fingerprint-session";

fn digest(label: &str, body: &str) {
    if std::env::var_os("NEXA_FP_DUMP").is_some() {
        println!("===== {label} =====\n{body}\n===== end {label} =====");
        return;
    }
    // FNV-1a over the rendered artifact: stable across runs and platforms.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    println!("{label}\t{hash:016x}\t{}", body.len());
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temporary_root(tag: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after the epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "nexa-project-fingerprint-{tag}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("temporary project root should be created");
    root
}

/// Collects every file under `root`, keyed by its path relative to `root`.
fn collect(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            // A missing directory is an empty subtree, not a failure.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("cannot read {}: {error}", directory.display()),
        };
        for entry in entries {
            let entry = entry.expect("directory entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("collected path should be under the root")
                    .to_string_lossy()
                    .into_owned();
                let contents = std::fs::read(&path)
                    .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
                files.insert(relative, contents);
            }
        }
    }
    files
}

/// Replaces machine-specific absolute paths so digests are reproducible.
fn normalize(bytes: &[u8], output: &Path, workspace: &Path) -> String {
    String::from_utf8_lossy(bytes)
        .replace(&output.to_string_lossy().to_string(), "<OUT>")
        .replace(&workspace.to_string_lossy().to_string(), "<WS>")
}

fn main() {
    let workspace = workspace_root().canonicalize().expect("workspace should exist");
    let examples = workspace.join("examples");

    for (entry, app_name) in FIXTURES {
        report(&examples, entry, app_name, false);
    }
    for (entry, app_name) in DEV_FIXTURES {
        report(&examples, entry, app_name, true);
    }
}

fn report(examples: &Path, entry: &str, app_name: &str, dev: bool) {
    let input = examples.join(entry);
    if !input.is_file() {
        eprintln!("skipping {}: not a file", input.display());
        return;
    }
    let base = entry.trim_end_matches(".nx").replace('/', "-");
    let label = if dev { format!("dev-{base}") } else { base };
    let output = temporary_root(&label).join("Generated");

    let outcome = if dev {
        nexa_cli::generate_dev_project(
            &input,
            "all",
            &output,
            app_name,
            DEV_SERVER_URL,
            DEV_SESSION_TOKEN,
        )
    } else {
        nexa_cli::generate_project(&input, "all", &output, app_name)
    };
    if let Err(error) = outcome {
        digest(&format!("{label}/ERROR"), &format!("ERR:{error}"));
        return;
    }

    let files = collect(&output);
    // One digest per file: a changed path is as much a regression as changed
    // bytes, because the Xcode project references files by name.
    for (path, bytes) in &files {
        let normalized = normalize(bytes, &output, examples.parent().expect("examples has a parent"));
        digest(&format!("{label}/{path}"), &normalized);
    }

    // One combined digest so a whole-fixture change is obvious at a glance.
    let mut combined = String::new();
    for (path, bytes) in &files {
        combined.push_str(path);
        combined.push('\0');
        combined.push_str(&normalize(
            bytes,
            &output,
            examples.parent().expect("examples has a parent"),
        ));
        combined.push('\0');
    }
    digest(&format!("{label}/TOTAL"), &combined);
    println!("# {label}: {} files", files.len());
}

