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

#[test]
fn lowers_foreground_tasks_and_cancellation_to_typed_ir() {
    let module = compile(
        "nexa-foreground-task-valid",
        r#"app ForegroundTask {
            state refreshTask: TaskHandle? = null
            state finished: Bool = false

            body {
                Button("Start") {
                    Task.launch(handle: refreshTask, executor: TaskExecutor.Main) {
                        finished = true
                    }
                    Task.cancel(handle: refreshTask)
                }
            }
        }"#,
    )
    .expect("typed task handles should lower");

    assert!(
        module
            .states
            .iter()
            .any(|state| state.name == "refreshTask")
    );
    let nexa_ir::Node::Button { actions, .. } = &module.body[0] else {
        panic!("task actions should remain in the button callback");
    };
    assert!(matches!(
        actions.as_slice(),
        [
            nexa_ir::Action::TaskLaunch {
                handle,
                executor: nexa_ir::TaskExecutor::Main,
                actions: body,
            },
            nexa_ir::Action::TaskCancel { handle: canceled },
        ] if handle.as_deref() == Some("refreshTask")
            && canceled == "refreshTask"
            && matches!(body.as_slice(), [nexa_ir::Action::Assign { name, value: nexa_ir::Expr::Bool(true) }] if name == "finished")
    ));
}

#[test]
fn lowers_handleless_foreground_tasks_without_replacing_prior_work() {
    let module = compile(
        "nexa-fire-and-forget-task",
        r#"app FireAndForget {
            body {
                Button("Start") {
                    Task.launch(executor: TaskExecutor.Main) { Storage.setString(key: "done", value: "yes") }
                }
            }
        }"#,
    )
    .expect("handle-less task launch should lower");
    let nexa_ir::Node::Button { actions, .. } = &module.body[0] else {
        panic!("task action should remain in button callback");
    };
    assert!(matches!(
        actions.as_slice(),
        [nexa_ir::Action::TaskLaunch {
            handle: None,
            executor: nexa_ir::TaskExecutor::Main,
            ..
        }]
    ));
}

#[test]
fn foreground_background_task_cannot_capture_ui_state() {
    let error = compile(
        "nexa-background-executor-state",
        r#"app StateCapture {
            state workerTask: TaskHandle? = null
            state label: String = "ready"
            body {
                Button("Start") {
                    Task.launch(handle: workerTask, executor: TaskExecutor.Background) {
                        label = "working"
                    }
                }
            }
        }"#,
    )
    .expect_err("background execution must not capture UI-owned state");

    assert!(error.contains("cannot read or mutate UI state"));
}
