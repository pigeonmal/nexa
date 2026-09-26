use std::{fs, process::Command};

/// A temporary project whose directory is owned for as long as it is bound.
struct TempProject(nexa_testkit::TempDir);

impl TempProject {
    fn new() -> Self {
        Self(nexa_testkit::TempDir::new("nexa-check"))
    }
}

#[test]
fn check_resolves_local_plugin_dependencies_without_native_toolchains() {
    let project = TempProject::new();
    let plugin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/plugins/fast-math")
        .canonicalize()
        .expect("resolve FastMath fixture");
    let escaped_plugin = plugin
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    fs::write(
        project.0.join("nexa.config.nx"),
        format!(
            "config {{ dependencies {{ FastMath {{ id: \"dev.nexa.fast-math\", path: \"{escaped_plugin}\" }} }} permissions {{}} }}\n"
        ),
    )
    .expect("write Nexa config");
    fs::write(
        project.0.join("App.nx"),
        "plugin \"dev.nexa.fast-math\" as FastMath\napp Demo { state result: Int32 = 0 body { Button(\"Compute\") { result = FastMath.add(10, 20) } } }\n",
    )
    .expect("write app entry");

    let output = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["check", "--deny-warnings"])
        .current_dir(&project.0)
        .output()
        .expect("run Nexa checker");
    assert!(output.status.success(), "{output:?}");
    let lock = fs::read_to_string(project.0.join("nexa.lock")).expect("generated lockfile");
    assert!(lock.contains("dev.nexa.fast-math"));
    assert!(lock.contains("path:"));
    assert!(!project.0.join("build").exists());

    let locked = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["check", "--deny-warnings", "--locked"])
        .current_dir(&project.0)
        .output()
        .expect("run Nexa checker with the lockfile enforced");
    assert!(locked.status.success(), "{locked:?}");
}
