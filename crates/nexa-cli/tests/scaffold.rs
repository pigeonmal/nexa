use std::{fs, process::Command};

#[test]
fn scaffold_writes_valid_source_and_default_config() {
    // `nexa create` refuses to scaffold into a directory that already exists.
    let root = nexa_testkit::VacantDir::new("nexa-create");
    let output = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["create", "SampleApp", "--directory"])
        .arg(root.path())
        .output()
        .expect("run Nexa scaffold command");
    assert!(output.status.success(), "{output:?}");

    let source = fs::read_to_string(root.join("App.nx")).expect("scaffold source");
    nexa_syntax::parse_program(&source).expect("scaffold app is valid");
    assert!(source.contains("Text(\"Count: $count\")"));
    assert!(!source.contains("${count}"));
    let config = fs::read_to_string(root.join("nexa.config.nx")).expect("scaffold config");
    let config = nexa_syntax::parse_config(&config).expect("scaffold config is valid");
    assert_eq!(config.android.expect("android config").target_sdk, Some(36));
    let signing = fs::read_to_string(root.join(".nexa/signing.properties"))
        .expect("scaffold local signing settings");
    assert!(signing.contains("NEXA_ANDROID_KEYSTORE="));
    assert!(signing.contains("NEXA_ANDROID_KEY_ALIAS="));
    assert!(
        fs::read_to_string(root.join(".gitignore"))
            .expect("scaffold gitignore")
            .lines()
            .any(|line| line == ".nexa/"),
        "local signing settings should be Git-ignored"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.join(".nexa/signing.properties"))
                .expect("signing settings metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600,
            "local signing settings should only be readable by their owner"
        );
    }
}
