#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

fn executable_in_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

#[test]
fn android_release_validates_signing_and_reports_only_verified_aabs() {
    // The guard removes the scratch tree, including when an assertion panics.
    let scratch = nexa_testkit::TempDir::new("nexa-android-release-artifacts");
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
        "#!/bin/sh\nprintf '%s' \"$NEXA_ANDROID_KEYSTORE\" > \"$NEXA_FAKE_ANDROID_MARKER\"\nif [ \"${NEXA_FAKE_ANDROID_NO_ARTIFACT:-0}\" = 1 ]; then exit 0; fi\nmkdir -p app/build/outputs/bundle/release\nprintf 'signed bundle' > app/build/outputs/bundle/release/app-release.aab\nexit 0\n",
    )
    .expect("write fake Java tool");
    let mut permissions = fs::metadata(&fake_java)
        .expect("read fake tool metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_java, permissions).expect("make fake Java executable");

    let fake_jarsigner = fake_bin.join("jarsigner");
    fs::write(
        &fake_jarsigner,
        "#!/bin/sh\ncase \"$NEXA_FAKE_ANDROID_VERIFY_MODE\" in\n  unsigned) echo 'jar is unsigned.'; exit 0 ;;\n  partial) echo 'jar verified.'; echo 'This jar contains unsigned entries.'; exit 0 ;;\n  invalid) echo 'signature invalid' >&2; exit 1 ;;\n  *) echo 'jar verified.'; exit 0 ;;\nesac\n",
    )
    .expect("write fake jarsigner");
    let mut permissions = fs::metadata(&fake_jarsigner)
        .expect("read fake jarsigner metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_jarsigner, permissions).expect("make fake jarsigner executable");

    let mut search_paths = vec![fake_bin];
    if let Some(existing) = std::env::var_os("PATH") {
        search_paths.extend(std::env::split_paths(&existing));
    }
    let path = std::env::join_paths(search_paths).expect("compose fake tool PATH");
    let keystore = project.join("keys/release.jks");
    fs::create_dir_all(keystore.parent().expect("keystore parent"))
        .expect("create project keystore directory");
    fs::write(&keystore, "test keystore").expect("create fake keystore file");
    let marker = scratch.join("gradle-ran");
    let required = [
        "NEXA_ANDROID_KEYSTORE",
        "NEXA_ANDROID_KEY_ALIAS",
        "NEXA_ANDROID_STORE_PASSWORD",
        "NEXA_ANDROID_KEY_PASSWORD",
    ];
    let credentials = vec![
        (required[0], Some("keys/release.jks".into())),
        (required[1], Some("release".into())),
        (required[2], Some("test-password".into())),
        (required[3], Some("test-password".into())),
    ];
    let release = |signing: &[(&str, Option<std::ffi::OsString>)],
                   without_artifact: bool,
                   verify_mode: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nexa"));
        command
            .args(["release", "--android"])
            .current_dir(&project)
            .env("PATH", &path)
            .env("JAVA_HOME", &fake_jdk)
            .env("NEXA_FAKE_ANDROID_MARKER", &marker)
            .env("NEXA_FAKE_ANDROID_VERIFY_MODE", verify_mode)
            .env(
                "NEXA_FAKE_ANDROID_NO_ARTIFACT",
                if without_artifact { "1" } else { "0" },
            );
        for key in required {
            command.env_remove(key);
        }
        for (key, value) in signing {
            if let Some(value) = value {
                command.env(key, value);
            }
        }
        command.output().expect("run Nexa Android release")
    };

    for missing_key in required {
        let mut signing = credentials.clone();
        signing
            .iter_mut()
            .find(|(key, _)| *key == missing_key)
            .unwrap()
            .1 = None;
        let failure = release(&signing, false, "valid");
        assert!(!failure.status.success(), "{missing_key} must be required");
        assert!(
            String::from_utf8_lossy(&failure.stderr).contains(missing_key),
            "stderr: {}",
            String::from_utf8_lossy(&failure.stderr)
        );
        assert!(
            !marker.exists(),
            "Gradle must not run without {missing_key}"
        );
    }

    for blank_key in required {
        let mut signing = credentials.clone();
        signing
            .iter_mut()
            .find(|(key, _)| *key == blank_key)
            .unwrap()
            .1 = Some("  ".into());
        let failure = release(&signing, false, "valid");
        assert!(!failure.status.success(), "{blank_key} must not be blank");
        assert!(String::from_utf8_lossy(&failure.stderr).contains(blank_key));
        assert!(
            !marker.exists(),
            "Gradle must not run with blank {blank_key}"
        );
    }

    let mut nonexistent_keystore = credentials.clone();
    nonexistent_keystore[0].1 = Some(scratch.join("missing.jks").into_os_string());
    let failure = release(&nonexistent_keystore, false, "valid");
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("existing file"));
    assert!(
        !marker.exists(),
        "Gradle must not run with a missing keystore"
    );

    let mut directory_keystore = credentials.clone();
    directory_keystore[0].1 = Some(scratch.path().to_path_buf().into_os_string());
    let failure = release(&directory_keystore, false, "valid");
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("must point to a file"));
    assert!(
        !marker.exists(),
        "Gradle must not run with a directory keystore"
    );

    let missing = release(&credentials, true, "valid");
    assert!(!missing.status.success(), "missing AAB must fail release");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("did not create"),
        "stderr: {}",
        String::from_utf8_lossy(&missing.stderr)
    );

    fs::remove_file(&marker).expect("remove fake Gradle marker before successful run");
    let successful = release(&credentials, false, "valid");
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
    assert!(
        marker.is_file(),
        "Gradle should run after signing validation"
    );
    assert_eq!(
        fs::read_to_string(&marker).expect("read Gradle keystore path"),
        keystore
            .canonicalize()
            .expect("resolve project-relative keystore")
            .to_string_lossy()
    );

    let unsigned = release(&credentials, false, "unsigned");
    assert!(!unsigned.status.success(), "unsigned AABs must be rejected");
    assert!(String::from_utf8_lossy(&unsigned.stderr).contains("signature verification failed"));

    let partially_signed = release(&credentials, false, "partial");
    assert!(
        !partially_signed.status.success(),
        "AABs with unsigned entries must be rejected"
    );
    assert!(String::from_utf8_lossy(&partially_signed.stderr).contains("unsigned entries"));

    let invalid = release(&credentials, false, "invalid");
    assert!(
        !invalid.status.success(),
        "invalid AAB signatures must be rejected"
    );
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("signature invalid"));

    fs::remove_file(&marker).expect("remove Gradle marker before stale artifact check");
    let stale = release(&credentials, true, "valid");
    assert!(
        !stale.status.success(),
        "a previous AAB must not count as new output"
    );
    assert!(String::from_utf8_lossy(&stale.stderr).contains("did not create"));
    assert!(
        !project
            .join("build/android/app/build/outputs/bundle/release/app-release.aab")
            .exists(),
        "the previous AAB should be removed before release"
    );
}

#[test]
fn android_release_verifies_real_jarsigner_outputs_when_a_jdk_is_available() {
    let Some(jarsigner) = executable_in_path("jarsigner") else {
        eprintln!("skipping real AAB signature integration: jarsigner is unavailable");
        return;
    };
    let Ok(jarsigner) = jarsigner.canonicalize() else {
        eprintln!("skipping real AAB signature integration: cannot resolve jarsigner");
        return;
    };
    let Some(jdk_home) = jarsigner.parent().and_then(Path::parent) else {
        eprintln!("skipping real AAB signature integration: cannot resolve JDK home");
        return;
    };
    let keytool = jdk_home.join("bin/keytool");
    if !keytool.is_file() {
        eprintln!("skipping real AAB signature integration: matching keytool is unavailable");
        return;
    }

    let scratch = nexa_testkit::TempDir::new("nexa-android-real-aab-signature");
    let project = scratch.join("project");
    let create = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["create", "SignedAabSmoke", "--directory"])
        .arg(&project)
        .output()
        .expect("run Nexa create");
    assert!(create.status.success(), "{create:?}");

    let keystore = scratch.join("release.jks");
    let generated_key = Command::new(&keytool)
        .args(["-genkeypair", "-noprompt", "-alias", "release", "-keystore"])
        .arg(&keystore)
        .args([
            "-storepass",
            "test-password",
            "-keypass",
            "test-password",
            "-dname",
            "CN=Nexa Test",
            "-keyalg",
            "RSA",
            "-keysize",
            "2048",
            "-validity",
            "2",
        ])
        .output()
        .expect("generate temporary Android signing key");
    assert!(
        generated_key.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&generated_key.stdout),
        String::from_utf8_lossy(&generated_key.stderr)
    );

    let signed = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["release", "--android"])
        .current_dir(&project)
        .env("JAVA_HOME", jdk_home)
        .env("NEXA_ANDROID_KEYSTORE", &keystore)
        .env("NEXA_ANDROID_KEY_ALIAS", "release")
        .env("NEXA_ANDROID_STORE_PASSWORD", "test-password")
        .env("NEXA_ANDROID_KEY_PASSWORD", "test-password")
        .output()
        .expect("run Android release with a real temporary key");
    assert!(
        signed.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&signed.stdout),
        String::from_utf8_lossy(&signed.stderr)
    );
    assert!(String::from_utf8_lossy(&signed.stdout).contains("Created signed Android AAB at"));
    let aab = project.join("build/android/app/build/outputs/bundle/release/app-release.aab");
    assert!(aab.is_file(), "release should create an Android App Bundle");
    let verified = Command::new(&jarsigner)
        .args(["-verify", "-verbose:summary"])
        .arg(&aab)
        .env("LC_ALL", "C")
        .output()
        .expect("verify the released AAB with the real jarsigner");
    assert!(
        verified.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&verified.stdout),
        String::from_utf8_lossy(&verified.stderr)
    );
    assert!(
        String::from_utf8_lossy(&verified.stdout).contains("jar verified."),
        "jarsigner did not confirm the bundle signature: {}",
        String::from_utf8_lossy(&verified.stdout)
    );
}
