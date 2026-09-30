use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

fn compile(name: &str, source: &str) -> Result<nexa_ir::Module, String> {
    let project = TestProject::new(name);
    let entry = project.join("App.nx");
    fs::write(&entry, source).expect("background-task test source should be written");
    compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .map(|compiled| compiled.module)
        .map_err(|error| error.to_string())
}

#[test]
fn lowers_background_task_actions_and_keeps_their_native_capabilities() {
    let module = compile(
        "nexa-background-task-valid",
        r#"app BackgroundSync {
            fn shouldWriteMarker() -> Bool { return true }

            background task Refresh(
                identifier: "dev.example.backgroundsync.refresh",
                everyMinutes: 60
            ) {
                if shouldWriteMarker() {
                    Storage.setString(key: "background.lastRefresh", value: "done")
                }
            }

            body { Text("Background sync") }
        }"#,
    )
    .expect("valid background task should lower");

    assert_eq!(module.background_tasks.len(), 1);
    assert_eq!(module.background_tasks[0].interval_minutes, 60);
    assert_eq!(
        module.functions.len(),
        1,
        "the task keeps its called helper reachable"
    );
    let facts = nexa_ir::facts::ModuleFacts::analyze(&module);
    assert!(facts.capabilities.uses_storage_api);
}

#[test]
fn rejects_background_task_intervals_below_the_cross_platform_minimum() {
    let error = compile(
        "nexa-background-task-interval",
        r#"app TooFrequent {
            background task Refresh(
                identifier: "dev.example.refresh",
                everyMinutes: 14
            ) {}
            body { Text("refresh") }
        }"#,
    )
    .expect_err("Android periodic work requires at least 15 minutes");

    assert!(error.contains("at least 15 minutes"));
}

#[test]
fn rejects_invalid_background_task_identifiers() {
    let error = compile(
        "nexa-background-task-identifier",
        r#"app BadIdentifier {
            background task Refresh(identifier: "refresh", everyMinutes: 60) {}
            body { Text("refresh") }
        }"#,
    )
    .expect_err("task identifiers must use reverse-domain notation");

    assert!(error.contains("reverse-domain notation"));
}

#[test]
fn background_task_cannot_capture_ui_state() {
    let error = compile(
        "nexa-background-task-state",
        r#"app StateCapture {
            state label: String = "ready"

            background task Refresh(identifier: "dev.example.refresh", everyMinutes: 60) {
                Storage.setString(key: "label", value: label)
            }

            body { Text(label) }
        }"#,
    )
    .expect_err("background work must not capture state owned by a foreground UI");

    assert!(error.contains("label"), "unexpected diagnostic: {error}");
}
