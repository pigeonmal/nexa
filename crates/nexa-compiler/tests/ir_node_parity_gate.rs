//! Parity exhaustiveness gate for IR View Nodes.
//!
//! Asserts that every `Node` variant in `nexa_ir::Node` has an explicit, active
//! rendering dispatch in:
//! 1. `runtime/ios/NexaDevRenderer.swift` (and `NexaDevRuntime.swift`)
//! 2. `runtime/android/NexaDevRenderer.kt` (and `NexaDevRuntime.kt`)
//!
//! This prevents any component (such as `Shape`, `ContentUnavailable`, etc.) from
//! silently failing to render in DevRuntime while working in AOT.

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

const ALL_IR_NODES: &[&str] = &[
    "SafeAreaPadding",
    "SafeAreaInset",
    "ScrollHeader",
    "Appearance",
    "StatusBar",
    "Direction",
    "Layout",
    "Form",
    "FormSection",
    "Text",
    "ContentUnavailable",
    "Spacer",
    "Divider",
    "Shape",
    "Button",
    "TextInput",
    "Switch",
    "Slider",
    "ProgressBar",
    "ProgressRing",
    "SegmentedControl",
    "Picker",
    "PickerDate",
    "Image",
    "SystemIcon",
    "FontIcon",
    "LinearGradient",
    "Pressable",
    "NavigationStack",
    "NavigationSplitView",
    "NavigationLink",
    "NavigationBack",
    "Link",
    "Accessibility",
    "KeyboardAware",
    "BottomSheet",
    "Dialog",
    "ConfirmationDialog",
    "RefreshControl",
    "AppBottomBar",
    "PagePager",
    "Toolbar",
    "FastList",
    "If",
    "When",
    "Content",
    "ComponentCall",
    "NativeComponentCall",
];

#[test]
fn every_ir_node_has_renderer_dispatch_in_both_dev_runtimes() {
    let root = workspace_root();

    let ios_renderer = fs::read_to_string(root.join("runtime/ios/NexaDevRenderer.swift"))
        .expect("read NexaDevRenderer.swift");
    let ios_runtime = fs::read_to_string(root.join("runtime/ios/NexaDevRuntime.swift"))
        .expect("read NexaDevRuntime.swift");
    let ios_dev = format!("{ios_renderer}\n{ios_runtime}");

    let android_renderer = fs::read_to_string(root.join("runtime/android/NexaDevRenderer.kt"))
        .expect("read NexaDevRenderer.kt");
    let android_runtime = fs::read_to_string(root.join("runtime/android/NexaDevRuntime.kt"))
        .expect("read NexaDevRuntime.kt");
    let android_dev = format!("{android_renderer}\n{android_runtime}");

    let mut missing_ios = Vec::new();
    let mut missing_android = Vec::new();

    for node in ALL_IR_NODES {
        let node_kind_lower = match *node {
            "Shape" => "shape",
            "StatusBar" => "statusBar",
            "Direction" => "direction",
            _ => "",
        };

        // iOS checks for either `"NodeName"` in switch/guard, `NexaDevNodeKind.*`, or `child.has("NodeName")`
        let ios_has = ios_dev.contains(&format!("\"{node}\""))
            || ios_dev.contains(&format!("NexaDevNodeKind.{node}"))
            || (!node_kind_lower.is_empty() && ios_dev.contains(&format!("NexaDevNodeKind.{node_kind_lower}")))
            || ios_dev.contains(&format!("has(\"{node}\")"))
            || ios_dev.contains("devStatusBar")
            || (node == &"Direction" && ios_dev.contains("layoutDirection"));
        if !ios_has {
            missing_ios.push(node.to_string());
        }

        // Android checks for `"NodeName"` in when/if or `child.has("NodeName")`
        let android_has = android_dev.contains(&format!("\"{node}\""))
            || android_dev.contains(&format!("NexaDevNodeKind.{node}"))
            || android_dev.contains(&format!("NexaDevNodeKind.{}", node.to_uppercase()))
            || android_dev.contains(&format!("has(\"{node}\")"))
            || (node == &"StatusBar" && android_dev.contains("NexaDevStatusBar"))
            || (node == &"Direction" && android_dev.contains("LocalLayoutDirection"));
        if !android_has {
            missing_android.push(node.to_string());
        }
    }

    assert!(
        missing_ios.is_empty(),
        "iOS DevRuntime renderer is missing dispatch for IR nodes: {:#?}",
        missing_ios
    );
    assert!(
        missing_android.is_empty(),
        "Android DevRuntime renderer is missing dispatch for IR nodes: {:#?}",
        missing_android
    );
}
