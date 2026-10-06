use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Expr;
use nexa_testkit::TestProject;

#[test]
fn text_literals_are_translation_keys_and_keep_translator_comments() {
    let project = TestProject::new("nexa-zero-key-localization");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app LocaleExample {
    state name: String = "Ada"
    state count: Int32 = 3
    state enabled: Bool = false
    state showingDialog: Bool = false
    body {
        Column {
            Text("Save", comment: "Button to persist user profile")
            Text("Hello, \(name)!", comment: "Greeting shown at the top of the screen")
            Text("\(count) task", comment: "A task count; translators should use the appropriate plural form")
            TextInput(value: name, placeholder: "Your name", comment: "Prompt for the person's name")
            Form {
                Section(title: "Profile", footer: "Account details", comment: "Profile settings section") {
                    Switch(value: enabled, label: "Enable sync", comment: "Sync preference")
                }
            }
            Button("Continue", comment: "Moves to the next onboarding step") { showingDialog = true }
            Dialog(isPresented: showingDialog, title: "Saved", message: "Your profile was saved", comment: "Save confirmation") {
                Button("Done") { showingDialog = false }
            }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let swift = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("source literals should compile for SwiftUI")
        .module;
    let entries = nexa_ir::localization::extract(&swift);
    assert_eq!(
        entries["Save"].comment.as_deref(),
        Some("Button to persist user profile")
    );
    assert_eq!(entries["Hello, {name}!"].arguments, ["name"]);
    assert_eq!(entries["{count} task"].arguments, ["count"]);
    assert_eq!(
        entries["Profile"].comment.as_deref(),
        Some("Profile settings section")
    );
    assert_eq!(
        entries["Account details"].comment.as_deref(),
        Some("Profile settings section")
    );
    assert_eq!(
        entries["Enable sync"].comment.as_deref(),
        Some("Sync preference")
    );
    assert_eq!(
        entries["Continue"].comment.as_deref(),
        Some("Moves to the next onboarding step")
    );
    assert_eq!(
        entries["Saved"].comment.as_deref(),
        Some("Save confirmation")
    );
    assert_eq!(
        entries["Your profile was saved"].comment.as_deref(),
        Some("Save confirmation")
    );
    assert_eq!(
        entries["Your name"].comment.as_deref(),
        Some("Prompt for the person's name")
    );

    let swift_source = nexa_backend_swift::SwiftBackend
        .generate_units(&swift)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift_source.contains("Text(\"Save\", comment: \"Button to persist user profile\")"));
    assert!(swift_source.contains(
        "Text(\"Hello, \\(nexa_name)!\", comment: \"Greeting shown at the top of the screen\")"
    ));
    assert!(
        swift_source.contains("Text(\"Your name\", comment: \"Prompt for the person's name\")")
    );
    assert!(
        swift_source.contains("Text(\"Continue\", comment: \"Moves to the next onboarding step\")")
    );
    assert!(swift_source.contains("Text(\"Enable sync\", comment: \"Sync preference\")"));
    assert!(swift_source.contains("Text(\"Profile\", comment: \"Profile settings section\")"));

    let kotlin = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("source literals should compile for Compose")
        .module;
    let kotlin_source = nexa_backend_kotlin::KotlinBackend
        .generate_with_project_features(&kotlin)
        .0;
    assert!(
        kotlin_source.contains("stringResource(R.string.nexa_save_"),
        "{kotlin_source}"
    );
    assert!(
        kotlin_source.contains("pluralStringResource(R.plurals."),
        "{kotlin_source}"
    );

    let mut saw_localized_expr = false;
    nexa_ir::walk::walk_ir(&swift.body, &mut |_| {}, &mut |expression| {
        saw_localized_expr |= matches!(expression, Expr::LocalizedText { .. })
    });
    assert!(saw_localized_expr);
}

#[test]
fn locale_localized_accepts_source_text_for_native_api_arguments() {
    let project = TestProject::new("nexa-locale-localized-source-text");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app LocaleExample {
    state name: String = "Ada"
    body {
        Column {
            Text(Locale.localized("Reminder"))
            Text(Locale.localized("Welcome, \(name)!"))
            Button("Save") {
                Storage.setString("reminder.title", Locale.localized("Reminder"))
            }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let swift = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("source-text locale calls should compile for SwiftUI")
        .module;
    let entries = nexa_ir::localization::extract(&swift);
    assert!(entries.contains_key("Reminder"));
    assert_eq!(entries["Welcome, {name}!"].arguments, ["name"]);
    let swift_source = nexa_backend_swift::SwiftBackend
        .generate_units(&swift)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift_source.contains("NSLocalizedString(\"Reminder\""));

    let kotlin = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("source-text locale calls should compile for Compose")
        .module;
    let kotlin_source = nexa_backend_kotlin::KotlinBackend
        .generate_with_project_features(&kotlin)
        .0;
    assert!(kotlin_source.contains("NexaRuntime.context().getString(R.string."));
}

#[test]
fn locale_localized_from_top_level_functions_is_extracted() {
    let project = TestProject::new("nexa-locale-localized-function-source-text");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"fn reminderTitle() -> String {
    return Locale.localized("Task reminder")
}

app LocaleExample {
    body {
        Column {
            Text(reminderTitle())
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("localized source text in a function should compile")
        .module;
    let entries = nexa_ir::localization::extract(&module);
    assert!(entries.contains_key("Task reminder"));
}
