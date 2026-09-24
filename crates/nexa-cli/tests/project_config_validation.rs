use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static PROJECT_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let sequence = PROJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nexa-config-validation-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temporary project");
        fs::write(
            root.join("App.nx"),
            "app Demo { body { Text(\"ready\") } }\n",
        )
        .expect("write app source");
        Self(root)
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

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
