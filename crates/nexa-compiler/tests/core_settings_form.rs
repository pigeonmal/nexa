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
        matches!(swift_module.body.first(), Some(Node::Form { show_dividers: false, background: Some(_), children, .. }) if matches!(children.first(), Some(Node::FormSection { title: Some(_), footer: Some(_), show_dividers: Some(true), background: Some(_), .. })) && matches!(children.get(1), Some(Node::FormSection { show_dividers: Some(false), .. })))
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
    assert!(swift.contains("icon: NexaIconSpec(symbol: \"star.fill\")"));
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
    assert!(kotlin.contains("NexaFormRowPrimitive(minHeight = 50f, horizontalInset = 16f) {"));
    assert_eq!(kotlin.matches("NexaFormDividerPrimitive()").count(), 2);
    let section_title = nexa_codegen::names::localization_resource_name("General");
    assert!(kotlin.contains(&format!(
        "title = NexaLocale.string(R.string.{section_title})"
    )));
    assert!(kotlin.contains("NexaPickerPrimitive(items ="));
    let picker_label = nexa_codegen::names::localization_resource_name("Start Page");
    assert!(
        kotlin.contains(&format!("NexaLocale.string(R.string.{picker_label})")),
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
            assert!(swift.contains("NexaFormPrimitive(background: nil, hideHorizontalScrollbar: false, hideVerticalScrollbar: false) {"));
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

#[test]
fn form_padding_and_margin_keep_their_authored_values_on_both_targets() {
    let project = TestProject::new("nexa-form-explicit-insets");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app FormInsets {
    body {
        Form(padding: Insets(top: SystemInsets.top), margin: Insets(top: 4)) {
            Section(title: "Featured") {
                Text("First setting")
            }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("form insets should compile for iOS")
        .module;
    let swift = nexa_backend_swift::SwiftBackend.generate(&swift_module);
    assert!(
        swift.contains(
            "contentPadding: EdgeInsets(top: max(0, CGFloat(Double(nexaWindowMetrics.safeInsets.logical(layoutDirection).top)))"
        ),
        "{swift}"
    );
    assert!(
        swift.contains("margin: EdgeInsets(top: max(0, CGFloat(Double(4)))"),
        "{swift}"
    );
    assert!(swift.contains("firstChildIsSection: true"), "{swift}");
    assert!(
        swift.contains(
            ".environment(\\.nexaFormTopPadding, firstChildIsSection ? effectivePadding.top : 0)"
        ),
        "{swift}"
    );
    assert!(
        swift.contains(".padding(.top, firstHeaderTopPadding)"),
        "{swift}"
    );
    assert!(swift.contains("NexaFormPrimitive("), "{swift}");
    assert!(!swift.contains("NexaFormScrollInsetsProbe"));
    assert!(
        swift.contains(".frame(height: effectivePadding.bottom)"),
        "{swift}"
    );
    assert!(!swift.contains("contentPadding.top - max(0"), "{swift}");
    assert!(!swift.contains("listSectionMargins(.top"), "{swift}");
    assert!(!swift.contains("formSectionTopPadding"), "{swift}");

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("form insets should compile for Android")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);
    assert!(
        kotlin.contains("contentPadding = androidx.compose.foundation.layout.PaddingValues("),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("LocalNexaWindowMetrics.current.systemInset(NexaSafeAreaEdge.Top"),
        "{kotlin}"
    );
    assert!(kotlin.contains("modifier = Modifier.padding("), "{kotlin}");
    assert!(
        kotlin.contains(".verticalScroll(scrollState, enabled = scrollState.maxValue > 0)\n                .padding(effectiveContentPadding)"),
        "{kotlin}"
    );
}

#[test]
fn scroll_insets_reach_native_content_instead_of_outer_padding() {
    let project = TestProject::new("nexa-scroll-content-insets");
    let entry = project.join("App.nx");
    fs::write(&entry, r#"app ScrollInsets {
    body {
        FastList(count: 20, rowHeight: 56, padding: Insets(top: SystemInsets.top, bottom: 12), margin: Insets(top: 4)) {
            index in
            Text("Library song \(index)")
        }
        Column(padding: 16) { Text("Library controls") }
    }
}"#).expect("write scroll inset fixture");
    let module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("compile scroll inset fixture")
        .module;
    assert!(
        matches!(&module.body[0], Node::ViewInsets { child, .. } if matches!(child.as_ref(), Node::FastList { .. }))
    );
    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("scrollContent: true, usesScrollMarginLayout: false) {"));
    assert!(swift.contains("scrollContent: false, usesScrollMarginLayout: false) {"));
    assert!(swift.contains("scrollView.contentInset = insets"));
    assert!(swift.contains("environment: context.environment, reverseLayout: reverseLayout"));
    assert!(swift.contains("content().environment(\\.nexaScrollContentPadding, padding)"));

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("compile scroll inset fixture for Android")
        .module;
    assert!(
        matches!(&kotlin_module.body[0], Node::ViewInsets { child, .. } if matches!(child.as_ref(), Node::FastList { .. }))
    );
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);
    assert!(kotlin.contains("scrollContent = true"), "{kotlin}");
    assert!(
        kotlin.contains("contentPadding = LocalNexaScrollContentPadding.current"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("NexaScrollContentResetPrimitive {"),
        "{kotlin}"
    );
}

#[test]
fn android_dev_runtime_native_fast_list_preserves_outer_modifier() {
    let renderer = include_str!("../../../runtime/android/NexaDevRenderer.kt");

    // ViewInsets passes component margin through this modifier. The platform
    // list branch must keep it outside the scroll viewport just like the
    // ordinary LazyColumn branch does.
    assert!(
        renderer.contains(
            "modifier.then(scrollViewportModifier).then(NexaNativeListContainerModifier())"
        )
    );
    assert!(!renderer.contains("scrollViewportModifier.then(NexaNativeListContainerModifier())"));
}
