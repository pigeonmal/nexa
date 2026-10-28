use nexa_testkit::{TestProject, nexa_command};
use serde_json::Value;
use std::process::Output;

fn check(project: &TestProject, source: &str, args: &[&str]) -> Output {
    project.write_app(source);
    nexa_command()
        .args(["check"])
        .args(args)
        .current_dir(project.path())
        .output()
        .expect("run nexa check")
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "check should emit one JSON report: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn check_json_emits_typed_warning_fields_and_success_status() {
    let project = TestProject::new("nexa-check-json-warning");
    let output = check(
        &project,
        "app JsonWarning { state unusedCount: Int32 = 0 body { Text(\"Ready\") } }\n",
        &["--json"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = report(&output);
    assert_eq!(report["formatVersion"], 1);
    assert_eq!(report["ok"], true);
    assert_eq!(report["platform"], "all");
    let warning = &report["diagnostics"][0];
    assert_eq!(warning["severity"], "warning");
    assert_eq!(warning["code"], "NEXA-W1005");
    assert!(
        warning["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("App.nx"))
    );
    assert!(warning["span"]["line"].as_u64().is_some());
    assert!(warning["message"].as_str().is_some());
}

#[test]
fn check_json_emits_source_location_for_compile_errors() {
    let project = TestProject::new("nexa-check-json-error");
    let output = check(
        &project,
        "app JsonError { body { Text(missingValue) } }\n",
        &["--json", "--ios"],
    );
    assert!(!output.status.success());
    let report = report(&output);
    assert_eq!(report["ok"], false);
    assert_eq!(report["platform"], "ios");
    let error = &report["diagnostics"][0];
    assert_eq!(error["severity"], "error");
    assert!(
        error["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("App.nx"))
    );
    assert_eq!(error["span"]["line"], 1);
    assert!(error["message"].as_str().is_some());
}

#[test]
fn check_json_returns_nonzero_when_warnings_are_denied() {
    let project = TestProject::new("nexa-check-json-denied-warning");
    let output = check(
        &project,
        "app JsonWarning { state unusedCount: Int32 = 0 body { Text(\"Ready\") } }\n",
        &["--json", "--deny-warnings"],
    );
    assert!(!output.status.success());
    let report = report(&output);
    assert_eq!(report["ok"], false);
    assert!(
        report["error"]
            .as_str()
            .is_some_and(|error| error.contains("warning(s) treated as errors"))
    );
    assert_eq!(report["diagnostics"][0]["severity"], "warning");
}

#[test]
fn check_json_rejects_combining_text_audit_output() {
    let project = TestProject::new("nexa-check-json-audit");
    let output = check(
        &project,
        "app JsonAudit { body { Text(\"Ready\") } }\n",
        &["--json", "--audit"],
    );
    assert!(!output.status.success());
    let report = report(&output);
    assert_eq!(report["ok"], false);
    assert!(
        report["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("run `nexa audit` separately"))
    );
}
