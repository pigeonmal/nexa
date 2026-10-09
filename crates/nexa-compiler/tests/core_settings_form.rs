use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Node;
use nexa_testkit::TestProject;

#[test]
fn settings_form_and_labeled_picker_lower_natively_for_both_targets() {
    let project = TestProject::new("nexa-settings-form");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app SettingsFormExample {
    let pages: Array<String> = ["Inbox", "Today", "Upcoming"]
    state startPage: String = "Today"
    state openLinksInApp: Bool = false
    state switchLabel: String = "Open Links In App"

    body {
        Form {
            Section(title: "General", footer: "Version 1.0") {
                Picker(items: pages, selected: startPage, label: "Start Page", icon: "star")
                Switch(value: openLinksInApp, label: switchLabel)
            }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("settings form should lower for iOS")
        .module;
    assert!(
        matches!(swift_module.body.first(), Some(Node::Form { children }) if matches!(children.first(), Some(Node::FormSection { title: Some(_), footer: Some(_), .. })))
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
    assert!(swift.contains("header: { Text(\"General\") }"));
    assert!(swift.contains("footer: { Text(\"Version 1.0\") }"));
    assert!(swift.contains("NexaPickerPrimitive(selection: $nexa_startPage"));
    assert!(swift.contains("hasLabel: true) {"));
    assert!(swift.contains("Text(\"Start Page\")"));
    assert!(swift.contains("icon: \"star.fill\""));
    assert!(swift.contains("Text(nexa_switchLabel)"));

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("settings form should lower for Android")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);
    assert!(kotlin.contains("verticalScroll(rememberScrollState())"));
    let section_title = nexa_codegen::names::localization_resource_name("General");
    assert!(kotlin.contains(&format!("Text(stringResource(R.string.{section_title})")));
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
