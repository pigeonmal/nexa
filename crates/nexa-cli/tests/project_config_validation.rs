use std::{
    fs,
    process::{Command, Output},
};

/// A temporary project whose directory is owned for as long as it is bound.
struct TempProject(nexa_testkit::TempDir);

impl TempProject {
    fn new() -> Self {
        let temporary = nexa_testkit::TempDir::new("nexa-config-validation");
        fs::write(
            temporary.join("App.nx"),
            "app Demo { body { Text(\"ready\") } }\n",
        )
        .expect("write app source");
        Self(temporary)
    }

    fn check(&self, config: &str) -> Output {
        fs::write(self.0.join("nexa.config.nx"), config).expect("write project config");
        Command::new(env!("CARGO_BIN_EXE_nexa"))
            .args(["check"])
            .current_dir(&self.0)
            .output()
            .expect("run nexa check")
    }
}

#[test]
fn check_accepts_supported_configured_sdk_minimums() {
    let project = TempProject::new();
    let output = project
        .check("config { ios { minVersion: \"15.1\" } android { minSdk: 24, targetSdk: 36 } }\n");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_rejects_malformed_ios_minimum_versions() {
    for version in ["15", "15.x", "15.1.0.2"] {
        let project = TempProject::new();
        let config = format!("config {{ ios {{ minVersion: \"{version}\" }} }}\n");
        let output = project.check(&config);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "accepted iOS minVersion {version}"
        );
        assert!(
            stderr.contains("iOS minVersion must be a numeric version"),
            "version {version}: {stderr}"
        );
    }
}

#[test]
fn check_rejects_android_minimum_outside_target_range() {
    for min_sdk in [0, 37] {
        let project = TempProject::new();
        let config = format!("config {{ android {{ minSdk: {min_sdk}, targetSdk: 36 }} }}\n");
        let output = project.check(&config);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "accepted Android minSdk {min_sdk}"
        );
        assert!(
            stderr.contains("Android minSdk must be between 1 and targetSdk (36)"),
            "minSdk {min_sdk}: {stderr}"
        );
    }
}

#[test]
fn check_validates_custom_and_https_deep_link_bases() {
    let project = TempProject::new();
    let valid =
        project.check(r#"config { app { deepLinks: ["nexa://", "https://links.example.com"] } }"#);
    assert!(
        valid.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&valid.stdout),
        String::from_utf8_lossy(&valid.stderr)
    );

    for deep_links in [
        r#"["http://links.example.com"]"#,
        r#"["https://links.example.com/path"]"#,
        r#"["nexa://host/path"]"#,
        r#"["nexa://", "nexa://"]"#,
    ] {
        let config = format!("config {{ app {{ deepLinks: {deep_links} }} }}");
        let output = project.check(&config);
        assert!(
            !output.status.success(),
            "accepted invalid deep-link bases {deep_links}"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("deepLinks"));
    }
}
