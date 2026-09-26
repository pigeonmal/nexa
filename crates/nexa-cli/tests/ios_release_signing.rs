#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn ios_release_uses_an_imported_manual_provisioning_profile() {
    // The guard removes the scratch tree, including when an assertion panics.
    let scratch = nexa_testkit::TempDir::new("nexa-ios-release-signing");
    let project = scratch.join("project");
    let fake_bin = scratch.join("bin");
    fs::create_dir_all(&fake_bin).expect("create fake tool directory");

    let log = scratch.join("xcodebuild.log");
    let fake_xcodebuild = fake_bin.join("xcodebuild");
    fs::write(
        &fake_xcodebuild,
        r###"#!/bin/sh
printf '%s\n' "$*" >> "$NEXA_FAKE_XCODEBUILD_LOG"
archive=""
export_path=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -archivePath) archive="$2"; shift 2 ;;
        -exportPath) export_path="$2"; shift 2 ;;
        *) shift ;;
    esac
done
if [ -n "$archive" ] && [ "${NEXA_FAKE_XCODEBUILD_NO_ARCHIVE:-0}" != 1 ]; then
    app_name=$(basename "$archive" .xcarchive)
    mkdir -p "$archive/Products/Applications/$app_name.app"
    printf 'archive' > "$archive/Info.plist"
fi
if [ -n "$export_path" ] && [ "${NEXA_FAKE_XCODEBUILD_NO_IPA:-0}" != 1 ]; then
    mkdir -p "$export_path"
    app_name=$(basename "$archive" .xcarchive)
    printf 'ipa' > "$export_path/$app_name.ipa"
fi
exit 0
"###,
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
    let release = |without_archive: bool, without_ipa: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nexa"));
        command
            .args(["release", "--ios"])
            .current_dir(&project)
            .env("PATH", &path)
            .env("NEXA_FAKE_XCODEBUILD_LOG", &log)
            .env("NEXA_IOS_TEAM_ID", "TEAM123")
            .env("NEXA_IOS_PROVISIONING_PROFILE", "PROFILE-UUID-123")
            .env("NEXA_IOS_EXPORT_METHOD", "app-store-connect")
            .env(
                "NEXA_FAKE_XCODEBUILD_NO_ARCHIVE",
                if without_archive { "1" } else { "0" },
            )
            .env(
                "NEXA_FAKE_XCODEBUILD_NO_IPA",
                if without_ipa { "1" } else { "0" },
            );
        command.output().expect("run Nexa iOS release")
    };
    let missing_archive = release(true, false);
    assert!(
        !missing_archive.status.success(),
        "missing archive app product must fail release"
    );
    assert!(
        String::from_utf8_lossy(&missing_archive.stderr)
            .contains("did not contain the app product"),
        "stderr: {}",
        String::from_utf8_lossy(&missing_archive.stderr)
    );

    let missing = release(false, true);
    assert!(!missing.status.success(), "missing IPA must fail release");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("did not create an IPA"),
        "stderr: {}",
        String::from_utf8_lossy(&missing.stderr)
    );

    let successful = release(false, false);
    assert!(
        successful.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&successful.stdout),
        String::from_utf8_lossy(&successful.stderr)
    );

    let options = fs::read_to_string(project.join("build/ExportOptions.plist"))
        .expect("release export options");
    assert!(options.contains("<key>signingStyle</key><string>manual</string>"));
    assert!(options.contains("<key>dev.nexa.cisigning</key><string>PROFILE-UUID-123</string>"));
    let invocations = fs::read_to_string(&log).expect("captured xcodebuild invocations");
    assert!(invocations.contains("CODE_SIGN_STYLE=Manual"));
    assert!(invocations.contains("CODE_SIGN_IDENTITY=Apple Distribution"));
    assert!(invocations.contains("PROVISIONING_PROFILE_SPECIFIER=PROFILE-UUID-123"));
    assert!(
        project
            .join("build/artifacts/ios/CiSigning.xcarchive/Products/Applications/CiSigning.app")
            .is_dir()
    );
    assert!(
        project
            .join("build/artifacts/ios/ipa/CiSigning.ipa")
            .is_file()
    );

    let stale_archive = release(true, false);
    assert!(
        !stale_archive.status.success(),
        "a previous archive must not count as new output"
    );
    assert!(
        String::from_utf8_lossy(&stale_archive.stderr).contains("did not contain the app product")
    );
    assert!(
        !project
            .join("build/artifacts/ios/CiSigning.xcarchive")
            .exists(),
        "the previous archive should be removed before release"
    );

    let stale_ipa = release(false, true);
    assert!(
        !stale_ipa.status.success(),
        "a previous IPA must not count as new output"
    );
    assert!(String::from_utf8_lossy(&stale_ipa.stderr).contains("did not create an IPA"));
    assert!(
        !project
            .join("build/artifacts/ios/ipa/CiSigning.ipa")
            .exists(),
        "the previous IPA should be removed before export"
    );
}
