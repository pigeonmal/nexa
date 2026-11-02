//! Parity exhaustiveness gate for IR Actions.
//!
//! The inventory is derived from `nexa_ir::Action`, so a new action variant
//! fails this gate until both DevRuntimes handle it. This is intentionally a
//! *coverage smoke test*: value-level behavior is covered by the conformance
//! tests in `tests/dev_runtime_conformance.rs`.

use std::collections::BTreeSet;
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

fn enum_variants(source: &str, declaration: &str) -> BTreeSet<String> {
    let start = source
        .find(declaration)
        .unwrap_or_else(|| panic!("IR source defines {declaration}"));
    let block_start = source[start..].find('{').expect("enum body") + start;
    let mut depth = 0i32;
    let mut end = block_start;
    for (offset, character) in source[block_start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = block_start + offset;
                    break;
                }
            }
            _ => {}
        }
    }
    let mut variants = BTreeSet::new();
    for line in source[block_start..=end].lines().skip(1) {
        let trimmed = line.trim_start();
        if !line.starts_with("    ") || trimmed.starts_with("//") {
            continue;
        }
        let identifier: String = trimmed
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if identifier.is_empty()
            || !identifier.starts_with(|character: char| character.is_ascii_uppercase())
        {
            continue;
        }
        let rest = trimmed[identifier.len()..].trim_start();
        if rest.starts_with('{') || rest.starts_with('(') || rest.starts_with(',') {
            variants.insert(identifier);
        }
    }
    variants
}

#[test]
fn action_inventory_is_derived_from_the_ir() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let variants = enum_variants(&ir, "pub enum Action {");
    assert!(
        variants.len() >= 18,
        "Action variant parser found only {} variants: {variants:#?}",
        variants.len()
    );
    for expected in ["Assign", "NavigationPush", "For", "TryCatch"] {
        assert!(variants.contains(expected), "missing {expected}");
    }
}

#[test]
fn every_ir_action_has_execution_support_in_both_dev_runtimes() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let actions = enum_variants(&ir, "pub enum Action {");

    let ios_actions = fs::read_to_string(root.join("runtime/ios/NexaDevActions.swift"))
        .expect("read NexaDevActions.swift");
    let android_actions = fs::read_to_string(root.join("runtime/android/NexaDevActions.kt"))
        .expect("read NexaDevActions.kt");

    let mut missing_ios = Vec::new();
    let mut missing_android = Vec::new();
    for action in &actions {
        let ios_supported = ios_actions.contains(&format!("\"{action}\""))
            || (action == "Return" && ios_actions.contains("returnFlow"));
        if !ios_supported {
            missing_ios.push(action.clone());
        }
        let android_supported = android_actions.contains(&format!("\"{action}\""))
            || (action == "Return" && android_actions.contains("NexaDevFunctionFlow.Return"));
        if !android_supported {
            missing_android.push(action.clone());
        }
    }

    assert!(
        missing_ios.is_empty(),
        "iOS DevRuntime is missing handling for IR Actions: {missing_ios:#?}"
    );
    assert!(
        missing_android.is_empty(),
        "Android DevRuntime is missing handling for IR Actions: {missing_android:#?}"
    );
}

#[test]
fn every_navigation_command_variant_has_execution_support_in_both_dev_runtimes_and_both_aot_backends()
 {
    let root = workspace_root();
    let nav_ir = fs::read_to_string(root.join("crates/nexa-ir/src/navigation.rs"))
        .expect("read navigation IR source");
    let nav_commands = enum_variants(&nav_ir, "pub enum NavigationCommand {");
    assert!(
        !nav_commands.is_empty(),
        "NavigationCommand variants should not be empty"
    );

    let ios_actions = fs::read_to_string(root.join("runtime/ios/NexaDevActions.swift"))
        .expect("read NexaDevActions.swift");
    let android_actions = fs::read_to_string(root.join("runtime/android/NexaDevActions.kt"))
        .expect("read NexaDevActions.kt");
    let swift_aot = fs::read_to_string(
        root.join("crates/nexa-backend-swift/src/generator/components/controls.rs"),
    )
    .expect("read Swift controls.rs");
    let kotlin_aot = fs::read_to_string(
        root.join("crates/nexa-backend-kotlin/src/generator/components/controls.rs"),
    )
    .expect("read Kotlin controls.rs");

    for cmd in &nav_commands {
        assert!(
            ios_actions.contains(&format!("command[\"{cmd}\"]"))
                || ios_actions.contains(&format!("\"{cmd}\"")),
            "iOS DevRuntime is missing explicit handling for NavigationCommand::{cmd}"
        );
        assert!(
            android_actions.contains(&format!("navCmd.optJSONObject(\"{cmd}\")"))
                || android_actions.contains(&format!("navCmd.has(\"{cmd}\")")),
            "Android DevRuntime is missing explicit handling for NavigationCommand::{cmd}"
        );
        assert!(
            swift_aot.contains(&format!("NavigationCommand::{cmd}")),
            "Swift AOT generator is missing explicit match branch for NavigationCommand::{cmd}"
        );
        assert!(
            kotlin_aot.contains(&format!("NavigationCommand::{cmd}")),
            "Kotlin AOT generator is missing explicit match branch for NavigationCommand::{cmd}"
        );
    }

    // Verify non-empty functional execution for each command:
    // 1. Android Dev: SelectTab must invoke selectTab and OpenExternalRoute must invoke startActivity
    assert!(
        android_actions.contains("selectTab(target)"),
        "Android DevRuntime SelectTab must execute selectTab"
    );
    assert!(
        android_actions.contains("context.startActivity(intent)"),
        "Android DevRuntime OpenExternalRoute must execute startActivity"
    );

    // 2. Kotlin AOT: SelectTab emits NexaRuntime.selectTab and OpenExternalRoute emits startActivity
    assert!(
        kotlin_aot.contains("NexaRuntime.selectTab"),
        "Kotlin AOT SelectTab must emit NexaRuntime.selectTab"
    );
    assert!(
        kotlin_aot.contains("navController.context.startActivity"),
        "Kotlin AOT OpenExternalRoute must emit startActivity"
    );

    // 3. Swift AOT: SelectTab is observed in bottom_bar and OpenExternalRoute calls UIApplication.shared.open
    let swift_bottom_bar = fs::read_to_string(
        root.join("crates/nexa-backend-swift/src/generator/components/bottom_bar.rs"),
    )
    .expect("read Swift bottom_bar.rs");
    assert!(
        swift_bottom_bar.contains("NotificationCenter.default.publisher(for: NSNotification.Name")
            && swift_bottom_bar.contains("NexaSelectTab"),
        "Swift AOT TabView must observe NexaSelectTab notification"
    );
    assert!(
        swift_aot.contains("UIApplication.shared.open"),
        "Swift AOT OpenExternalRoute must call UIApplication.shared.open"
    );

    // 4. Swift Route Identity: NexaNavigationRoute exposes var identity: UUID
    let swift_types =
        fs::read_to_string(root.join("crates/nexa-backend-swift/src/generator/engine/types.rs"))
            .expect("read Swift types.rs");
    assert!(
        swift_types.contains("var identity: UUID"),
        "Swift AOT NexaNavigationRoute must declare var identity: UUID"
    );
}
