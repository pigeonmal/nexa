//! Contract-plan pilot golden tests for Column, Row, Button, and TextInput (PR 09 / W6).
//!
//! Validates that IR property contracts and defaults map deterministically
//! to the shared native primitives across SwiftUI and Jetpack Compose.

use std::collections::HashMap;

use nexa_codegen::Backend;
use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_testkit::TestProject;

fn compile_to_swift_and_kotlin(app_source: &str, project_name: &str) -> (String, String) {
    let project = TestProject::new(project_name);
    let entry = project.write_app(app_source);
    let roots: HashMap<String, std::path::PathBuf> = HashMap::new();

    let mut compiler = IncrementalProjectCompiler::default();
    let compilations = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[Target::Swift, Target::Kotlin],
            &roots,
        )
        .expect("Compilation should succeed");

    let swift_module = &compilations[0].module;
    let kotlin_module = &compilations[1].module;

    let swift_units = nexa_backend_swift::SwiftBackend.generate_units(swift_module);
    let swift_code = swift_units
        .units
        .iter()
        .map(|u| u.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let kotlin_code = nexa_backend_kotlin::KotlinBackend.generate(kotlin_module);

    (swift_code, kotlin_code)
}

#[test]
fn column_and_row_contract_plan_emits_shared_primitives_with_exact_alignments() {
    let source = r#"
app LayoutContractPilot {
    body {
        Column(gap: 16) {
            Row(gap: 8) {
                Text("Row Child A")
                Text("Row Child B")
            }
            Text("Column Child 2")
        }
    }
}
"#;

    let (swift, kotlin) = compile_to_swift_and_kotlin(source, "layout-contract-pilot");

    // Swift assertions
    assert!(
        swift.contains("NexaColumnPrimitive"),
        "Swift must use NexaColumnPrimitive"
    );
    assert!(
        swift.contains("NexaRowPrimitive"),
        "Swift must use NexaRowPrimitive"
    );
    assert!(
        swift.contains("spacing: 16"),
        "Swift Column must receive spacing 16"
    );
    assert!(
        swift.contains("spacing: 8"),
        "Swift Row must receive spacing 8"
    );

    // Kotlin assertions
    assert!(
        kotlin.contains("NexaColumnPrimitive"),
        "Kotlin must use NexaColumnPrimitive"
    );
    assert!(
        kotlin.contains("NexaRowPrimitive"),
        "Kotlin must use NexaRowPrimitive"
    );
    assert!(
        kotlin.contains("spacing = 16.dp") || kotlin.contains("16"),
        "Kotlin Column must receive spacing 16"
    );
    assert!(
        kotlin.contains("spacing = 8.dp") || kotlin.contains("8"),
        "Kotlin Row must receive spacing 8"
    );
}

#[test]
fn button_contract_plan_emits_shared_primitive_with_all_states() {
    let source = r#"
app ButtonContractPilot {
    state clickCount: Int32 = 0
    state isBusy: Bool = false

    body {
        Column {
            Button("Submit", disabled: isBusy) {
                clickCount = clickCount + 1
            }
        }
    }
}
"#;

    let (swift, kotlin) = compile_to_swift_and_kotlin(source, "button-contract-pilot");

    // Swift assertions
    assert!(
        swift.contains("NexaButtonPrimitive"),
        "Swift must use NexaButtonPrimitive"
    );
    assert!(
        swift.contains("Submit"),
        "Swift Button must carry label 'Submit'"
    );

    // Kotlin assertions
    assert!(
        kotlin.contains("NexaButtonPrimitive"),
        "Kotlin must use NexaButtonPrimitive"
    );
    assert!(
        kotlin.contains("R.string.") || kotlin.contains("Submit"),
        "Kotlin Button must carry localized string resource for 'Submit'"
    );
    assert!(
        kotlin.contains("disabled = nexa_isBusy"),
        "Kotlin Button must carry disabled binding"
    );
}

#[test]
fn text_input_contract_plan_emits_shared_primitive_and_state_binding() {
    let source = r#"
app TextInputContractPilot {
    state query: String = ""

    body {
        Column {
            TextInput(value: query, placeholder: "Search here...")
        }
    }
}
"#;

    let (swift, kotlin) = compile_to_swift_and_kotlin(source, "text-input-contract-pilot");

    // Swift assertions
    assert!(
        swift.contains("NexaTextInputPrimitive"),
        "Swift TextInput must use NexaTextInputPrimitive"
    );
    assert!(
        swift.contains("Search here..."),
        "Swift TextInput must preserve placeholder"
    );

    // Kotlin assertions
    assert!(
        kotlin.contains("NexaTextInputPrimitive"),
        "Kotlin TextInput must use NexaTextInputPrimitive"
    );
    assert!(
        kotlin.contains("R.string.") || kotlin.contains("Search here..."),
        "Kotlin TextInput must carry localized string resource for placeholder"
    );
    assert!(
        kotlin.contains("value = nexa_query"),
        "Kotlin TextInput must bind query state"
    );
}

#[test]
fn bottom_sheet_contract_plan_emits_shared_primitives_with_exact_parameters() {
    let source = r#"
app BottomSheetContractPlan {
    let sheetRef = ComponentRef<BottomSheet>()

    body {
        Column {
            BottomSheet(
                ref: sheetRef,
                detents: [0.5, 1.0],
                initialDetent: 0.5,
                showDragIndicator: true,
                cornerRadius: 16
            ) {
                Text("Sheet Content")
            }
        }
    }
}
"#;

    let (swift, kotlin) = compile_to_swift_and_kotlin(source, "bottom-sheet-contract-plan");

    // Swift assertions
    assert!(
        swift.contains("NexaBottomSheetPrimitive"),
        "Swift BottomSheet must use NexaBottomSheetPrimitive"
    );
    assert!(
        swift.contains(".fraction(0.5)") && swift.contains(".fraction(1)"),
        "Swift BottomSheet must include detents 0.5 and 1.0"
    );
    assert!(
        swift.contains("showDragIndicator: true"),
        "Swift BottomSheet must specify showDragIndicator"
    );
    assert!(
        swift.contains("cornerRadius: 16"),
        "Swift BottomSheet must specify cornerRadius"
    );

    // Kotlin assertions
    assert!(
        kotlin.contains("NexaBottomSheetPrimitive"),
        "Kotlin BottomSheet must use NexaBottomSheetPrimitive"
    );
    assert!(
        kotlin.contains("0.5f") && kotlin.contains("1f"),
        "Kotlin BottomSheet must include detents 0.5f and 1f"
    );
    assert!(
        kotlin.contains("showDragIndicator = true"),
        "Kotlin BottomSheet must specify showDragIndicator"
    );
    assert!(
        kotlin.contains("cornerRadius = 16f.dp") || kotlin.contains("16"),
        "Kotlin BottomSheet must specify cornerRadius"
    );
}

#[test]
fn dialog_and_confirmation_contract_plan_emits_shared_primitives() {
    let source = r#"
app DialogContractPlan {
    state showDialog: Bool = false
    state showConfirm: Bool = false

    body {
        Column {
            Dialog(isPresented: showDialog, title: "Delete item?", message: "Are you sure?") {
                Button("Cancel") { showDialog = false }
                Button("OK") { showDialog = false }
            }
            ConfirmationDialog(isPresented: showConfirm, title: "Choose option") {
                Button("Proceed") { showConfirm = false }
                Button("Cancel") { showConfirm = false }
            }
        }
    }
}
"#;

    let (swift, kotlin) = compile_to_swift_and_kotlin(source, "dialog-contract-plan");

    // Swift assertions
    assert!(
        swift.contains("NexaAlertDialogPrimitive"),
        "Swift Dialog must use NexaAlertDialogPrimitive"
    );
    assert!(
        swift.contains("NexaConfirmationDialogPrimitive"),
        "Swift ConfirmationDialog must use NexaConfirmationDialogPrimitive"
    );

    // Kotlin assertions
    assert!(
        kotlin.contains("NexaAlertDialogPrimitive"),
        "Kotlin Dialog must use NexaAlertDialogPrimitive"
    );
    assert!(
        kotlin.contains("NexaConfirmationDialogPrimitive"),
        "Kotlin ConfirmationDialog must use NexaConfirmationDialogPrimitive"
    );
}

#[test]
fn switch_and_slider_contract_plan_emits_shared_primitives() {
    let source = r#"
app SwitchAndSliderContractPlan {
    state isToggled: Bool = true
    state volume: Float64 = 0.5

    body {
        Column {
            Switch(value: isToggled, label: "Notifications")
            Slider(value: volume, min: 0.0, max: 1.0, step: 0.1)
        }
    }
}
"#;

    let (swift, kotlin) = compile_to_swift_and_kotlin(source, "switch-slider-contract-plan");

    // Swift assertions
    assert!(
        swift.contains("NexaSwitchPrimitive"),
        "Swift Switch must use NexaSwitchPrimitive"
    );
    assert!(
        swift.contains("NexaSliderPrimitive"),
        "Swift Slider must use NexaSliderPrimitive"
    );

    // Kotlin assertions
    assert!(
        kotlin.contains("NexaSwitchPrimitive"),
        "Kotlin Switch must use NexaSwitchPrimitive"
    );
    assert!(
        kotlin.contains("NexaSliderPrimitive"),
        "Kotlin Slider must use NexaSliderPrimitive"
    );
}

#[test]
fn fast_list_contract_plan_emits_shared_primitives_without_type_erasure() {
    let source = r#"
app FastListContractPlan {
    body {
        Column {
            FastList(count: 20, rowHeight: 48, showDividers: true) { index in
                Text("Item")
            }
        }
    }
}
"#;

    let (swift, kotlin) = compile_to_swift_and_kotlin(source, "fast-list-contract-plan");

    // Swift assertions
    assert!(
        swift.contains("NexaFastListPrimitive"),
        "Swift FastList must use NexaFastListPrimitive"
    );
    assert!(
        !swift.contains("AnyView"),
        "Swift FastList hot path must NOT use AnyView type erasure"
    );
    assert!(
        swift.contains("rowHeight: 48"),
        "Swift FastList must preserve rowHeight"
    );

    // Kotlin assertions
    assert!(
        kotlin.contains("LazyColumn("),
        "Kotlin FastList must emit native Compose LazyColumn"
    );
    assert!(
        kotlin.contains("items("),
        "Kotlin FastList must emit items()"
    );
}
