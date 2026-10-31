//! Parity exhaustiveness gate for IR Actions.
//!
//! Asserts that every `Action` variant in `nexa_ir::Action` has explicit handling
//! in:
//! 1. `runtime/ios/NexaDevActions.swift`
//! 2. `runtime/android/NexaDevActions.kt`
//!
//! This prevents control flow, async tasks, navigation, or mutations from silently
//! failing during DevRuntime execution while working in AOT.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const ALL_IR_ACTIONS: &[&str] = &[
    "Expression",
    "NavigationPush",
    "NavigationBack",
    "Let",
    "Return",
    "Assign",
    "NativePropertyAssign",
    "NativeEventSubscribe",
    "NetworkStatusSubscribe",
    "CollectionMutation",
    "TaskLaunch",
    "TaskCancel",
    "WithAnimation",
    "If",
    "For",
    "ForMap",
    "While",
    "TryCatch",
    "Break",
    "Continue",
];

#[test]
fn every_ir_action_has_execution_support_in_both_dev_runtimes() {
    let root = workspace_root();

    let ios_actions = fs::read_to_string(root.join("runtime/ios/NexaDevActions.swift"))
        .expect("read NexaDevActions.swift");
    let android_actions = fs::read_to_string(root.join("runtime/android/NexaDevActions.kt"))
        .expect("read NexaDevActions.kt");

    let mut missing_ios = Vec::new();
    let mut missing_android = Vec::new();

    for action in ALL_IR_ACTIONS {
        let action_needle = format!("\"{action}\"");

        let ios_supported = ios_actions.contains(&action_needle)
            || (action == &"Return" && ios_actions.contains("returnFlow"));
        if !ios_supported {
            missing_ios.push(action.to_string());
        }

        let android_supported = android_actions.contains(&action_needle)
            || (action == &"Return" && android_actions.contains("NexaDevFunctionFlow.Return"));
        if !android_supported {
            missing_android.push(action.to_string());
        }
    }

    assert!(
        missing_ios.is_empty(),
        "iOS DevRuntime is missing handling for IR Actions: {:#?}",
        missing_ios
    );
    assert!(
        missing_android.is_empty(),
        "Android DevRuntime is missing handling for IR Actions: {:#?}",
        missing_android
    );
}
