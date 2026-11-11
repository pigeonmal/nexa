//! Compile the real audit app with strict Swift diagnostics, without a device.
use std::path::{Path, PathBuf};
use std::process::Command;

fn swift_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("generated directory should exist") {
        let path = entry.expect("generated entry should be readable").path();
        if path.is_dir() {
            swift_sources(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "swift")
        {
            files.push(path);
        }
    }
}

fn compile_aot_quality(entry: &Path, name: &str) {
    if !cfg!(target_os = "macos") {
        eprintln!("skipping native Swift quality gate: macOS is required");
        return;
    }
    let Ok(sdk) = Command::new("xcrun")
        .args(["--sdk", "iphonesimulator", "--show-sdk-path"])
        .output()
    else {
        eprintln!("skipping native Swift quality gate: Xcode is unavailable");
        return;
    };
    if !sdk.status.success() {
        eprintln!("skipping native Swift quality gate: simulator SDK is unavailable");
        return;
    }
    let scratch = nexa_testkit::TestProject::new("nexa-native-swift-ui-quality");
    nexa_cli::generate_project(entry, "ios", scratch.path(), name)
        .expect("AOT audit app should generate");
    let mut files = Vec::new();
    swift_sources(&scratch.path().join(format!("ios/{name}")), &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "Swift gate must compile real generated files"
    );
    let sdk = String::from_utf8_lossy(&sdk.stdout).trim().to_owned();
    let target = format!("{}-apple-ios16.0-simulator", std::env::consts::ARCH);
    let output = Command::new("xcrun")
        .args([
            "--sdk",
            "iphonesimulator",
            "swiftc",
            "-O",
            "-whole-module-optimization",
            "-warnings-as-errors",
            "-swift-version",
            "6",
            "-parse-as-library",
            "-sdk",
            &sdk,
            "-target",
            &target,
        ])
        .args(&files)
        .arg("-o")
        .arg(scratch.path().join("native-quality"))
        .output()
        .expect("native Swift compiler should execute");
    assert!(
        output.status.success(),
        "native Swift quality gate failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn aot_ui_primitives_compile_in_swift_6_release_mode_without_warnings() {
    let entry =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/performance-audit/App.nx");
    compile_aot_quality(&entry, "PerformanceQuality");
}

#[test]
fn native_image_transport_and_sectioned_swipe_bindings_compile_without_warnings() {
    let entry = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/native-ui/image-network-quality.nx");
    compile_aot_quality(&entry, "ImageNetworkQuality");
}
