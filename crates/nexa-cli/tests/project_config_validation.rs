use nexa_testkit::{TestProject, assert_output_failure, assert_output_success, nexa_command};

fn setup_project() -> TestProject {
    let project = TestProject::new("nexa-config-validation");
    project.write_app("app Demo { body { Text(\"ready\") } }\n");
    project
}

fn check_config(project: &TestProject, config: &str) -> std::process::Output {
    project.write_config(config);
    nexa_command()
        .args(["check"])
        .current_dir(project.path())
        .output()
        .expect("run nexa check")
}

#[test]
fn check_accepts_supported_configured_sdk_minimums() {
    let project = setup_project();
    let valid_configs = [
        "config { ios { minVersion: \"15.1\" } android { minSdk: 24, targetSdk: 36 } }\n",
        "config { ios { minVersion: \"17.0\" } android { minSdk: 26, targetSdk: 36 } }\n",
    ];
    for config in valid_configs {
        let output = check_config(&project, config);
        assert_output_success(&output);
    }
}

#[test]
fn check_rejects_malformed_ios_minimum_versions_table() {
    let project = setup_project();
    let cases = [
        ("15", "iOS minVersion must be a numeric version"),
        ("15.x", "iOS minVersion must be a numeric version"),
        ("15.1.0.2", "iOS minVersion must be a numeric version"),
    ];
    for (version, expected_err) in cases {
        let config = format!("config {{ ios {{ minVersion: \"{version}\" }} }}\n");
        let output = check_config(&project, &config);
        assert_output_failure(&output, expected_err);
    }
}

#[test]
fn check_rejects_android_minimum_outside_target_range_table() {
    let project = setup_project();
    let cases = [
        (0, "Android minSdk must be between 1 and targetSdk (36)"),
        (37, "Android minSdk must be between 1 and targetSdk (36)"),
    ];
    for (min_sdk, expected_err) in cases {
        let config = format!("config {{ android {{ minSdk: {min_sdk}, targetSdk: 36 }} }}\n");
        let output = check_config(&project, &config);
        assert_output_failure(&output, expected_err);
    }
}

#[test]
fn check_validates_custom_and_https_deep_link_bases_table() {
    let project = setup_project();
    let valid =
        check_config(&project, r#"config { app { deepLinks: ["nexa://", "https://links.example.com"] } }"#);
    assert_output_success(&valid);

    let invalid_cases = [
        (r#"["http://links.example.com"]"#, "deepLinks"),
        (r#"["https://links.example.com/path"]"#, "deepLinks"),
        (r#"["nexa://host/path"]"#, "deepLinks"),
        (r#"["nexa://", "nexa://"]"#, "deepLinks"),
    ];
    for (deep_links, expected_err) in invalid_cases {
        let config = format!("config {{ app {{ deepLinks: {deep_links} }} }}");
        let output = check_config(&project, &config);
        assert_output_failure(&output, expected_err);
    }
}
