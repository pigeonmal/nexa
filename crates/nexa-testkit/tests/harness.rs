use nexa_testkit::{
    TestProject, TestTier, Toolchain, assert_file_contains, assert_file_not_contains,
    assert_output_failure, assert_output_success, example_path, fixture_path, workspace_root,
};
use std::process::Command;

#[test]
fn test_project_creates_and_manages_files() {
    let project = TestProject::new("testkit-smoke");
    assert!(project.path().is_dir());

    let app = project.write_app("app Smoke { body { Text(\"Hello\") } }");
    assert!(app.is_file());
    assert!(project.exists("App.nx"));
    assert!(project.file_contains("App.nx", "Hello"));

    let config = project.write_config("config { app { displayName: \"Smoke\" } }");
    assert!(config.is_file());
    assert!(project.file_contains("nexa.config.nx", "Smoke"));

    let sub = project.write("nested/deep/file.txt", "deep content");
    assert!(sub.is_file());
    assert!(project.file_contains("nested/deep/file.txt", "deep content"));
    assert!(project.source_tree_contains("txt", "deep content"));
    assert_eq!(
        project.source_text_containing("txt", "deep content"),
        Some("deep content".to_owned())
    );

    let sources = project.collect_sources("nx");
    assert_eq!(sources.len(), 2);
}

#[test]
fn toolchain_detection_caches_results() {
    // Calling discovery methods multiple times should return identical cached pointers
    let swift1 = Toolchain::swiftc();
    let swift2 = Toolchain::swiftc();
    assert_eq!(swift1, swift2);

    let kotlinc1 = Toolchain::kotlinc();
    let kotlinc2 = Toolchain::kotlinc();
    assert_eq!(kotlinc1, kotlinc2);

    let gradle1 = Toolchain::gradle();
    let gradle2 = Toolchain::gradle();
    assert_eq!(gradle1, gradle2);

    let tier = Toolchain::tier();
    assert!(tier == TestTier::Fast || tier == TestTier::Integration || tier == TestTier::E2E);
}

#[test]
fn fixture_helpers_locate_workspace_items() {
    let root = workspace_root();
    assert!(root.join("Cargo.toml").is_file());

    let showcase = example_path("showcase.nx");
    assert!(showcase.is_file());

    let fixture = fixture_path("dev_keyboard_aware.nx");
    assert!(fixture.is_file());
}

#[test]
fn assertion_helpers_work_as_expected() {
    let project = TestProject::new("testkit-assert");
    let file = project.write("test.txt", "alpha beta gamma");
    assert_file_contains(&file, "beta");
    assert_file_not_contains(&file, "delta");

    let success = Command::new("echo").arg("hello").output().unwrap();
    assert_output_success(&success);

    let failure = Command::new("sh")
        .args(["-c", "echo 'expected error message' >&2; exit 1"])
        .output()
        .unwrap();
    assert_output_failure(&failure, "expected error message");
}
