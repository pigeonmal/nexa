#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn ios_release_uses_an_imported_manual_provisioning_profile() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let scratch = std::env::temp_dir().join(format!(
        "nexa-ios-release-signing-{}-{nonce}",
        std::process::id()
    ));
    let project = scratch.join("project");
    let fake_bin = scratch.join("bin");
    fs::create_dir_all(&fake_bin).expect("create fake tool directory");

    let log = scratch.join("xcodebuild.log");
    let fake_xcodebuild = fake_bin.join("xcodebuild");
    fs::write(
        &fake_xcodebuild,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$NEXA_FAKE_XCODEBUILD_LOG\"\nexit 0\n",
    )
    .expect("write fake xcodebuild");
    let mut permissions = fs::metadata(&fake_xcodebuild)
        .expect("read fake xcodebuild metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_xcodebuild, permissions).expect("make fake xcodebuild executable");

    let create = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["create", "CiSigning", "--directory"])
        .arg(&project)
        .output()
        .expect("run Nexa create");
    assert!(create.status.success(), "{create:?}");

    let mut search_paths = vec![fake_bin];
    if let Some(existing) = std::env::var_os("PATH") {
        search_paths.extend(std::env::split_paths(&existing));
    }
    let path = std::env::join_paths(search_paths).expect("compose fake tool PATH");
    let release = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["release", "--ios"])
        .current_dir(&project)
        .env("PATH", path)
        .env("NEXA_FAKE_XCODEBUILD_LOG", &log)
        .env("NEXA_IOS_TEAM_ID", "TEAM123")
        .env("NEXA_IOS_PROVISIONING_PROFILE", "PROFILE-UUID-123")
        .env("NEXA_IOS_EXPORT_METHOD", "app-store-connect")
        .output()
        .expect("run Nexa iOS release");
    assert!(
        release.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&release.stdout),
        String::from_utf8_lossy(&release.stderr)
    );

    let options = fs::read_to_string(project.join("build/ExportOptions.plist"))
        .expect("release export options");
    assert!(options.contains("<key>signingStyle</key><string>manual</string>"));
    assert!(options.contains("<key>dev.nexa.cisigning</key><string>PROFILE-UUID-123</string>"));
    let invocations = fs::read_to_string(log).expect("captured xcodebuild invocations");
    assert!(invocations.contains("CODE_SIGN_STYLE=Manual"));
    assert!(invocations.contains("CODE_SIGN_IDENTITY=Apple Distribution"));
    assert!(invocations.contains("PROVISIONING_PROFILE_SPECIFIER=PROFILE-UUID-123"));

    fs::remove_dir_all(scratch).expect("clean test scratch files");
}
