use nexa_testkit::{TestProject, assert_output_failure, assert_output_success, nexa_command};

fn check(project: &TestProject, args: &[&str]) -> std::process::Output {
    nexa_command()
        .args(args)
        .current_dir(project.path())
        .output()
        .expect("run nexa check")
}

fn camera_request_app() -> &'static str {
    r#"
app CameraPrompt {
    state status: PermissionStatus = PermissionStatus.notDetermined
    body {
        OnAppear async {
            status = await Permissions.request(permission: Permission.Camera)
        }
        Text("Camera")
    }
}
"#
}

#[test]
fn check_requires_permission_metadata_for_builtin_requests() {
    let project = TestProject::new("nexa-check-permission-request");
    project.write_app(camera_request_app());
    project.write_config("config { permissions {} }\n");

    let missing = check(&project, &["check", "--ios"]);
    assert_output_failure(&missing, "permission `camera` is used");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("NSCameraUsageDescription"),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );

    project.write_config("config { permissions { camera: \"Scan a shipping label.\" } }\n");
    let configured = check(&project, &["check", "--deny-warnings"]);
    assert_output_success(&configured);
}

#[test]
fn native_project_generation_rejects_missing_permission_metadata() {
    let project = TestProject::new("nexa-generate-missing-permission");
    project.write_app(camera_request_app());
    project.write_config("config { permissions {} }\n");
    let output = project.join("generated");

    let error = nexa_cli::generate_project(&project.join("App.nx"), "ios", &output, "CameraPrompt")
        .expect_err("generation must validate native permission declarations");

    assert!(error.contains("NSCameraUsageDescription"), "{error}");
    assert!(!output.exists(), "invalid projects must not be generated");
}

#[test]
fn check_warns_when_configured_permission_is_unused() {
    let project = TestProject::new("nexa-check-unused-permission");
    project.write_app("app Demo { body { Text(\"Ready\") } }\n");
    project.write_config("config { permissions { camera: \"Scan a shipping label.\" } }\n");

    let output = check(&project, &["check", "--android"]);
    assert_output_success(&output);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("NEXA-W1011"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let denied = check(&project, &["check", "--android", "--deny-warnings"]);
    assert_output_failure(&denied, "warning(s) treated as errors");
}

#[test]
fn check_warns_when_permission_is_selected_dynamically() {
    let project = TestProject::new("nexa-check-dynamic-permission");
    project.write_app(
        r#"
app DynamicPermissionPrompt {
    state status: PermissionStatus = PermissionStatus.notDetermined
    body {
        OnAppear async {
            status = await Permissions.request(permission: if status == PermissionStatus.granted {
                Permission.Camera
            } else {
                Permission.Photos
            })
        }
        Text("Permission status: \(status)")
    }
}
"#,
    );
    project.write_config("config { permissions {} }\n");

    let output = check(&project, &["check"]);
    assert_output_success(&output);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("NEXA-W1012"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let denied = check(&project, &["check", "--deny-warnings"]);
    assert_output_failure(&denied, "warning(s) treated as errors");
}

#[test]
fn reachable_plugin_permission_metadata_satisfies_the_native_requirement() {
    let project = TestProject::new("nexa-check-plugin-permission");
    project.write(
        "camera/plugin.config.nx",
        r#"plugin {
            schema: 2
            id: "dev.nexa.test-camera"
            version: "0.1.0"
            sources { native: "native.nxid" }
            ios { usageDescriptions { NSCameraUsageDescription: "Use the camera to scan a label." } }
            android { permissions: ["android.permission.CAMERA"] }
        }
        "#,
    );
    project.write(
        "camera/native.nxid",
        "service TestCamera { fn currentZoom() -> Float32 }\n",
    );
    project.write(
        "nexa.config.nx",
        "config { dependencies { TestCamera { id: \"dev.nexa.test-camera\", path: \"camera\" } } permissions {} }\n",
    );
    project.write_app(
        r#"
plugin "dev.nexa.test-camera" as TestCamera
app CameraPrompt {
    state status: PermissionStatus = PermissionStatus.notDetermined
    state zoom: Float32 = 1.0
    body {
        OnAppear async {
            status = await Permissions.request(permission: Permission.Camera)
        }
        Button("Read camera zoom") {
            zoom = TestCamera.currentZoom()
        }
    }
}
"#,
    );

    let output = check(&project, &["check", "--deny-warnings"]);
    assert_output_success(&output);
}
