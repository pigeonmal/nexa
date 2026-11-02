//! Parity exhaustiveness gate for Shared Native Component Primitives.
//!
//! Asserts that every core visual primitive:
//! - NexaButtonPrimitive
//! - NexaTextInputPrimitive
//! - NexaFastList
//! - NexaSwitchPrimitive
//! - NexaSliderPrimitive
//! - NexaProgressBarPrimitive
//! - NexaProgressRingPrimitive
//! - NexaSegmentedControlPrimitive
//! - NexaContentUnavailablePrimitive
//! - NexaColumnPrimitive
//! - NexaRowPrimitive
//! - NexaStackPrimitive
//!
//! is actively invoked by:
//! 1. Swift AOT backend (`crates/nexa-backend-swift`)
//! 2. Kotlin AOT backend (`crates/nexa-backend-kotlin`)
//! 3. iOS DevRuntime renderer (`runtime/ios/NexaDevRenderer.swift`)
//! 4. Android DevRuntime renderer (`runtime/android/NexaDevRenderer.kt`)
//!
//! This ensures that release AOT and debug DevRuntime share identical layout,
//! interaction geometry, touch targets, accessibility semantics, and styling.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn collect_rs_files(dir: &Path, out: &mut String) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && let Ok(content) = fs::read_to_string(&path)
            {
                out.push_str(&content);
                out.push('\n');
            }
        }
    }
}

const SHARED_PRIMITIVES: &[&str] = &[
    "NexaButtonPrimitive",
    "NexaTextInputPrimitive",
    "NexaFastList",
    "NexaSwitchPrimitive",
    "NexaSliderPrimitive",
    "NexaProgressBarPrimitive",
    "NexaProgressRingPrimitive",
    "NexaSegmentedControlPrimitive",
    "NexaContentUnavailablePrimitive",
    "NexaColumnPrimitive",
    "NexaRowPrimitive",
    "NexaStackPrimitive",
];

#[test]
fn every_component_primitive_is_shared_across_aot_and_dev_runtime() {
    let root = workspace_root();

    // Read Swift AOT
    let mut swift_aot = String::new();
    collect_rs_files(&root.join("crates/nexa-backend-swift/src"), &mut swift_aot);

    // Read Kotlin AOT
    let mut kotlin_aot = String::new();
    collect_rs_files(
        &root.join("crates/nexa-backend-kotlin/src"),
        &mut kotlin_aot,
    );

    // Read DevRuntimes
    let ios_renderer = fs::read_to_string(root.join("runtime/ios/NexaDevRenderer.swift"))
        .expect("read NexaDevRenderer.swift");
    let android_renderer = fs::read_to_string(root.join("runtime/android/NexaDevRenderer.kt"))
        .expect("read NexaDevRenderer.kt");

    let mut missing_swift_aot = Vec::new();
    let mut missing_kotlin_aot = Vec::new();
    let mut missing_ios_dev = Vec::new();
    let mut missing_android_dev = Vec::new();

    for primitive in SHARED_PRIMITIVES {
        if !swift_aot.contains(primitive) {
            missing_swift_aot.push(primitive.to_string());
        }
        if !kotlin_aot.contains(primitive) {
            missing_kotlin_aot.push(primitive.to_string());
        }
        if !ios_renderer.contains(primitive) {
            missing_ios_dev.push(primitive.to_string());
        }
        if !android_renderer.contains(primitive) {
            missing_android_dev.push(primitive.to_string());
        }
    }

    assert!(
        missing_swift_aot.is_empty(),
        "Swift AOT backend is missing invocation for primitives: {:#?}",
        missing_swift_aot
    );
    assert!(
        missing_kotlin_aot.is_empty(),
        "Kotlin AOT backend is missing invocation for primitives: {:#?}",
        missing_kotlin_aot
    );
    assert!(
        missing_ios_dev.is_empty(),
        "iOS DevRuntime is missing invocation for primitives: {:#?}",
        missing_ios_dev
    );
    assert!(
        missing_android_dev.is_empty(),
        "Android DevRuntime is missing invocation for primitives: {:#?}",
        missing_android_dev
    );
}
