#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn android_release_requires_and_reports_the_generated_aab() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let scratch = std::env::temp_dir().join(format!(
        "nexa-android-release-artifacts-{}-{nonce}",
        std::process::id()
    ));
    let project = scratch.join("project");
    let fake_jdk = scratch.join("jdk");
    let fake_bin = fake_jdk.join("bin");
    fs::create_dir_all(&fake_bin).expect("create fake tool directory");

    let create = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["create", "CiAndroidRelease", "--directory"])
        .arg(&project)
        .output()
        .expect("run Nexa create");
    assert!(create.status.success(), "{create:?}");

    let fake_java = fake_bin.join("java");
    fs::write(
        &fake_java,
        "#!/bin/sh\nif [ \"${NEXA_FAKE_ANDROID_NO_ARTIFACT:-0}\" = 1 ]; then exit 0; fi\nmkdir -p app/build/outputs/bundle/release\nprintf 'signed bundle' > app/build/outputs/bundle/release/app-release.aab\nexit 0\n",
    )
    .expect("write fake Java tool");
    let mut permissions = fs::metadata(&fake_java)
        .expect("read fake tool metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_java, permissions).expect("make fake Java executable");

    let mut search_paths = vec![fake_bin];
    if let Some(existing) = std::env::var_os("PATH") {
        search_paths.extend(std::env::split_paths(&existing));
    }
    let path = std::env::join_paths(search_paths).expect("compose fake tool PATH");
    let release = |without_artifact: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nexa"));
        command
            .args(["release", "--android"])
            .current_dir(&project)
            .env("PATH", &path)
            .env("JAVA_HOME", &fake_jdk)
            .env("NEXA_ANDROID_KEYSTORE", scratch.join("release.jks"))
            .env("NEXA_ANDROID_KEY_ALIAS", "release")
            .env("NEXA_ANDROID_STORE_PASSWORD", "test-password")
            .env("NEXA_ANDROID_KEY_PASSWORD", "test-password")
            .env(
                "NEXA_FAKE_ANDROID_NO_ARTIFACT",
                if without_artifact { "1" } else { "0" },
            );
        command.output().expect("run Nexa Android release")
    };

    let missing = release(true);
    assert!(!missing.status.success(), "missing AAB must fail release");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("did not create"),
        "stderr: {}",
        String::from_utf8_lossy(&missing.stderr)
    );

    let successful = release(false);
    assert!(
        successful.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&successful.stdout),
        String::from_utf8_lossy(&successful.stderr)
    );
    assert!(
        project
            .join("build/android/app/build/outputs/bundle/release/app-release.aab")
            .is_file()
    );
    assert!(String::from_utf8_lossy(&successful.stdout).contains("Created signed Android AAB at"));

    fs::remove_dir_all(scratch).expect("clean test scratch files");
}
