use std::{fs, path::Path, process::Command};

use nexa_testkit::TempDir;

fn write_app(directory: &Path, name: &str, source: &str) -> std::path::PathBuf {
    let path = directory.join(format!("{name}.nx"));
    fs::write(&path, source).expect("write app source");
    path
}

#[test]
fn generated_ios_app_has_only_reachable_privacy_reasons_and_dev_preloads_them() {
    let scratch = TempDir::new("nexa-storage-privacy");
    let source = write_app(
        scratch.path(),
        "StorageApp",
        r#"
        app StorageApp {
            state theme: String = Storage.getString(key: "theme") ?? "system"

            body {
                Button("Save") {
                    Storage.setString(key: "theme", value: theme)
                }
            }
        }
        "#,
    );
    let release = scratch.path().join("release");
    nexa_cli::generate_project(&source, "ios", &release, "StoragePrivacy")
        .expect("generate iOS release project");

    let app_directory = release.join("ios/StoragePrivacy");
    let privacy = fs::read_to_string(app_directory.join("PrivacyInfo.xcprivacy"))
        .expect("storage app should have a privacy manifest");
    assert!(privacy.contains("NSPrivacyAccessedAPICategoryUserDefaults"));
    assert!(privacy.contains("CA92.1"));
    assert!(!privacy.contains("NSPrivacyAccessedAPICategoryFileTimestamp"));
    if let Ok(output) = Command::new("plutil")
        .arg("-lint")
        .arg(app_directory.join("PrivacyInfo.xcprivacy"))
        .output()
    {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(app_directory.join("NexaGenerated.swift").is_file());
    assert!(
        fs::read_to_string(app_directory.join("NexaGenerated.swift"))
            .expect("read generated Swift")
            .contains("NexaStorage.getString(")
    );
    let project = fs::read_to_string(release.join("ios/StoragePrivacy.xcodeproj/project.pbxproj"))
        .expect("read generated Xcode project");
    assert!(project.contains("PrivacyInfo.xcprivacy"));
    assert!(
        fs::read_to_string(release.join("nexa.project.json"))
            .expect("read project metadata")
            .contains("\"iosPrivacyManifest\": true")
    );

    let dev = scratch.path().join("dev");
    nexa_cli::generate_dev_project(
        &source,
        "ios",
        &dev,
        "StoragePrivacyDev",
        "ws://127.0.0.1:49152",
        "privacy-test-session",
    )
    .expect("generate iOS DevRuntime project");
    let dev_privacy = fs::read_to_string(dev.join("ios/StoragePrivacyDev/PrivacyInfo.xcprivacy"))
        .expect("DevRuntime preloads storage and network helpers");
    assert!(dev_privacy.contains("CA92.1"));
    assert!(dev_privacy.contains("NSPrivacyAccessedAPICategoryFileTimestamp"));
    assert!(dev_privacy.contains("C617.1"));
}

#[test]
fn generated_ios_app_without_required_reason_apis_has_no_privacy_manifest() {
    let scratch = TempDir::new("nexa-minimal-privacy");
    let source = write_app(
        scratch.path(),
        "MinimalApp",
        "app MinimalApp { body { Text(\"Hello\") } }\n",
    );
    let output = scratch.path().join("generated");
    nexa_cli::generate_project(&source, "ios", &output, "MinimalPrivacy")
        .expect("generate minimal iOS project");

    assert!(
        !output
            .join("ios/MinimalPrivacy/PrivacyInfo.xcprivacy")
            .exists()
    );
    assert!(
        fs::read_to_string(output.join("nexa.project.json"))
            .expect("read project metadata")
            .contains("\"iosPrivacyManifest\": false")
    );
}
