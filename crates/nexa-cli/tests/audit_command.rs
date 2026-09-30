use nexa_testkit::{TestProject, assert_output_success, nexa_command};

fn project_with_fast_math() -> TestProject {
    let project = TestProject::new("nexa-audit-plugin");
    let plugin = nexa_testkit::example_path("plugins/fast-math");
    let escaped_plugin = plugin
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    project.write_config(&format!(
        "config {{ dependencies {{ FastMath {{ id: \"dev.nexa.fast-math\", path: \"{escaped_plugin}\" }} }} permissions {{}} }}\n"
    ));
    project.write_app(
        "plugin \"dev.nexa.fast-math\" as FastMath\napp Demo { state result: Int32 = 0 body { Button(\"Compute\") { result = FastMath.add(10, 20) } } }\n",
    );
    project
}

#[test]
fn audit_command_reports_native_output_for_local_plugin_projects() {
    let project = project_with_fast_math();
    let output = nexa_command()
        .args(["audit", "App.nx", "--target", "ios", "--out", "audit.json"])
        .current_dir(project.path())
        .output()
        .expect("run native output audit");
    assert_output_success(&output);

    let report: serde_json::Value =
        serde_json::from_str(&project.read("audit.json")).expect("parse audit report");
    assert_eq!(report["tool"], "nexa audit");
    assert_eq!(report["targets"][0]["target"], "ios");
    assert!(
        report["targets"][0]["generatedSourceBytes"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(report["releaseMeasurements"], serde_json::Value::Null);
}

#[test]
fn check_audit_flag_prints_the_capability_report() {
    let project = TestProject::new("nexa-check-audit");
    project.write_app("app Demo { body { Text(\"Audit\") } }\n");

    let output = nexa_command()
        .args(["check", "--ios", "--audit"])
        .current_dir(project.path())
        .output()
        .expect("run check with audit");
    assert_output_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("check passed (ios)"), "{stdout}");
    assert!(stdout.contains("\"tool\": \"nexa audit\""), "{stdout}");
}

#[test]
fn audit_command_reports_reachable_storage_capability() {
    let project = TestProject::new("nexa-audit-storage");
    project.write_app(
        "app Demo { state theme: String = Storage.getString(key: \"theme\") ?? \"system\" body { Text(theme) } }\n",
    );

    let output = nexa_command()
        .args(["audit", "App.nx", "--target", "ios", "--out", "audit.json"])
        .current_dir(project.path())
        .output()
        .expect("run storage capability audit");
    assert_output_success(&output);

    let report: serde_json::Value =
        serde_json::from_str(&project.read("audit.json")).expect("parse audit report");
    assert_eq!(report["targets"][0]["capabilities"]["storage"], true);
}

#[test]
fn audit_help_is_available_from_the_root_command() {
    let output = nexa_command()
        .args(["audit", "--help"])
        .output()
        .expect("run audit help");
    assert_output_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nexa audit <source.nx>"), "{stdout}");
    assert!(stdout.contains("--release-sizes"), "{stdout}");
}
