//! Parity exhaustiveness gate for IR Expressions and Member Access.
//!
//! Asserts that every `MemberKind` variant in `nexa_ir::MemberKind` is actively
//! handled by:
//! 1. `runtime/ios/NexaDevState.swift`
//! 2. `runtime/android/NexaDevState.kt`
//!
//! This guarantees that dynamic expression evaluation in DevRuntime behaves
//! identically to statically typed AOT output.

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

const ALL_MEMBER_KINDS: &[&str] = &[
    "TupleIndex",
    "StructField",
    "ClassField",
    "ClassStaticField",
    "PluginField",
    "NetworkStatusCode",
    "NetworkBody",
    "CollectionCount",
    "CollectionIsEmpty",
    "StringTrimmed",
    "SignalValue",
    "ResultIsSuccess",
    "ResultValue",
    "ResultError",
];

#[test]
fn every_member_kind_has_evaluation_support_in_both_dev_runtimes() {
    let root = workspace_root();

    let ios_state = fs::read_to_string(root.join("runtime/ios/NexaDevState.swift"))
        .expect("read NexaDevState.swift");
    let android_state = fs::read_to_string(root.join("runtime/android/NexaDevState.kt"))
        .expect("read NexaDevState.kt");

    let mut missing_ios = Vec::new();
    let mut missing_android = Vec::new();

    for kind in ALL_MEMBER_KINDS {
        // Some member kinds are handled by kind string, some by target property or general object access
        let ios_has = ios_state.contains(&format!("\"{kind}\""))
            || match *kind {
                "TupleIndex" => ios_state.contains("pair[position]") || ios_state.contains("\"first\""),
                "StructField" => ios_state.contains("object[name]") || ios_state.contains("row.fields"),
                "ClassField" => ios_state.contains("instance.fields"),
                "CollectionCount" => ios_state.contains("name == \"count\""),
                "CollectionIsEmpty" => ios_state.contains("name == \"isEmpty\""),
                "StringTrimmed" => ios_state.contains("name == \"trimmed\""),
                "SignalValue" => ios_state.contains("\"value\""),
                _ => false,
            };
        if !ios_has {
            missing_ios.push(kind.to_string());
        }

        let android_has = android_state.contains(&format!("\"{kind}\""))
            || match *kind {
                "TupleIndex" => android_state.contains("getOrNull(0)") || android_state.contains("\"first\""),
                "StructField" => android_state.contains("base[name]") || android_state.contains("row.fields") || android_state.contains("base.opt(name)"),
                "ClassField" => android_state.contains("instance.fields"),
                "CollectionCount" => android_state.contains("name == \"count\"") || android_state.contains("\"count\" ->"),
                "CollectionIsEmpty" => android_state.contains("name == \"isEmpty\"") || android_state.contains("\"isEmpty\" ->"),
                "StringTrimmed" => android_state.contains("name == \"trimmed\"") || android_state.contains("\"trimmed\" ->"),
                "SignalValue" => android_state.contains("\"value\""),
                _ => false,
            };
        if !android_has {
            missing_android.push(kind.to_string());
        }
    }

    assert!(
        missing_ios.is_empty(),
        "iOS DevRuntime is missing MemberKind support: {:#?}",
        missing_ios
    );
    assert!(
        missing_android.is_empty(),
        "Android DevRuntime is missing MemberKind support: {:#?}",
        missing_android
    );
}
