use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Node;
use nexa_testkit::TestProject;

#[test]
fn settings_form_and_labeled_picker_lower_natively_for_both_targets() {
    let project = TestProject::new("nexa-settings-form");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r##"app SettingsFormExample {
    let pages: Array<String> = ["Inbox", "Today", "Upcoming"]
    state startPage: String = "Today"
    state openLinksInApp: Bool = false
    state switchLabel: String = "Open Links In App"

    body {
        Form(showDividers: false, background: "#FAFAFA") {
            Section(title: "General", footer: "Version 1.0", showDividers: true, background: "#FFFFFF") {
                Picker(items: pages, selected: startPage, label: "Start Page", icon: "star")
                Switch(value: openLinksInApp, label: switchLabel)
            }
            Section(title: "Quiet", showDividers: false) {
                Text("Updates are off")
                Text("You can turn them back on later")
            }
        }
    }
}
"##,
    )
    .expect("test source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("settings form should lower for iOS")
        .module;
    assert!(
        matches!(swift_module.body.first(), Some(Node::Form { show_dividers: false, background: Some(_), children }) if matches!(children.first(), Some(Node::FormSection { title: Some(_), footer: Some(_), show_dividers: Some(true), background: Some(_), .. })) && matches!(children.get(1), Some(Node::FormSection { show_dividers: Some(false), .. })))
    );
    let swift = nexa_backend_swift::SwiftBackend
        .generate_units(&swift_module)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift.contains("Form {"));
    assert!(swift.contains("Section {"));
    assert!(swift.contains("NexaFormSectionPrimitive("));
    assert!(swift.contains("NexaFormPrimitive(background: Color(red:"));
    assert!(swift.contains("background: Color(red:"));
    assert!(
        swift.contains(
            "NexaFormRowPrimitive(minHeight: 56, horizontalInset: 16, showDividers: true)"
        )
    );
    assert!(
        swift.contains(
            "NexaFormRowPrimitive(minHeight: 56, horizontalInset: 16, showDividers: false)"
        )
    );
    assert!(swift.contains("header: {"));
    assert!(swift.contains("Text(\"General\")"));
    assert!(swift.contains("footer: {"));
    assert!(swift.contains("Text(\"Version 1.0\")"));
    assert!(swift.contains("NexaPickerPrimitive(selection: $nexa_startPage"));
    assert!(swift.contains("hasLabel: true) {"));
    assert!(swift.contains("Text(\"Start Page\")"));
    assert!(swift.contains("icon: \"star.fill\""));
    assert!(swift.contains("Text(nexa_switchLabel)"));
    assert!(swift.contains(".scrollContentBackground(.hidden)"));
    assert!(swift.contains(".listRowBackground(background ?? Color.clear)"));

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("settings form should lower for Android")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);
    assert!(kotlin.contains("NexaFormPrimitive(background = Color(0xFFFAFAFA)) {"));
    assert!(kotlin.contains("background = Color(0xFFFFFFFF),"));
    assert!(kotlin.contains("NexaFormRowPrimitive(minHeight = 56f, horizontalInset = 0f) {"));
    assert_eq!(kotlin.matches("NexaFormDividerPrimitive()").count(), 2);
    let section_title = nexa_codegen::names::localization_resource_name("General");
    assert!(kotlin.contains(&format!("title = stringResource(R.string.{section_title})")));
    assert!(kotlin.contains("NexaPickerPrimitive(items ="));
    let picker_label = nexa_codegen::names::localization_resource_name("Start Page");
    assert!(
        kotlin.contains(&format!(
            "NexaRuntime.context().getString(R.string.{picker_label})"
        )),
        "{kotlin}"
    );
    assert!(kotlin.contains("nexa_switchLabel"));
}

#[test]
fn form_dividers_can_be_disabled_and_section_can_override() {
    let project = TestProject::new("nexa-form-divider-options");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app FormDividerOptions {
    body {
        Form(showDividers: false) {
            Text("Unsectioned row one")
            Text("Unsectioned row two")
            Section(title: "Visible separators", showDividers: true) {
                Text("First setting")
                Text("Second setting")
            }
            Section(title: "Inherited hidden separators") {
                Text("First quiet row")
                Text("Second quiet row")
            }
        }
        Form {
            Text("Default row one")
            Text("Default row two")
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("Form and Section divider options should type-check")
            .module;
        assert!(matches!(
            module.body.first(),
            Some(Node::Form { show_dividers: false, background: None, children, .. })
                if matches!(children.get(2), Some(Node::FormSection { show_dividers: Some(true), .. }))
                    && matches!(children.get(3), Some(Node::FormSection { show_dividers: None, .. }))
        ));
        assert!(matches!(
            module.body.get(1),
            Some(Node::Form { show_dividers: true, background: None, children, .. }) if children.len() == 2
        ));

        if target == Target::Swift {
            let swift = nexa_backend_swift::SwiftBackend.generate(&module);
            assert!(swift.contains("NexaFormPrimitive(background: nil) {"));
            assert!(swift.contains("background: nil,"));
            assert!(swift.contains("showDividers: false"));
            assert!(swift.contains("showDividers: true"));
        } else {
            let (kotlin, _) =
                nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
            assert!(kotlin.contains("NexaFormPrimitive(background = null) {"));
            assert!(kotlin.contains("background = null,"));
            // One occurrence is the primitive definition; only the explicit
            // enabled section emits a divider call.
            assert_eq!(kotlin.matches("NexaFormDividerPrimitive()").count(), 3);
        }
    }
}
