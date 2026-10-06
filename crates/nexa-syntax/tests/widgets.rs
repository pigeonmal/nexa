use nexa_syntax::ast::{Expr, WidgetDecl};

#[test]
fn parses_universal_widget_provider_families_and_refresh() {
    let source = r#"
        widget Tasks(
            entry: TaskEntry.current(),
            families: [Small, Medium, Large, ExtraLarge],
            refreshSeconds: 1800
        ) {
            Text(entry.title)
        }
    "#;
    let program = nexa_syntax::parse_program(source).expect("widget declaration should parse");
    let widget: &WidgetDecl = program.widgets.first().expect("widget is retained");
    assert_eq!(widget.name, "Tasks");
    assert!(!matches!(&widget.entry_provider, Expr::String(_, _)));
    assert_eq!(
        widget.families,
        vec!["Small", "Medium", "Large", "ExtraLarge"]
    );
    assert!(matches!(&widget.refresh_seconds, Expr::Number(value, _) if value == "1800"));
    assert_eq!(widget.body.len(), 1);
}

#[test]
fn widget_requires_provider_families_and_refresh_seconds() {
    let error = nexa_syntax::parse_program("widget Tasks() { Text(\"Hello\") }")
        .expect_err("missing required parameters should be rejected");
    assert!(error.message.contains("requires `entry:"));
}

#[test]
fn parses_widget_gallery_and_configuration_source_text() {
    let source = r#"
        widget Tasks(
            displayName: "Tasks",
            description: "A quick view of your tasks",
            configurationTitle: "Task Filter",
            configurationDescription: "Choose which tasks to display",
            configuration: TaskConfiguration(Filter.today),
            entry: TaskData.load(configuration),
            families: [Small],
            refreshSeconds: 1800
        ) {
            Text(entry.title)
        }
    "#;
    let program = nexa_syntax::parse_program(source).expect("widget declaration should parse");
    let widget = program.widgets.first().expect("widget is retained");
    assert_eq!(widget.display_name.as_deref(), Some("Tasks"));
    assert_eq!(
        widget.description.as_deref(),
        Some("A quick view of your tasks")
    );
    assert_eq!(widget.configuration_title.as_deref(), Some("Task Filter"));
    assert_eq!(
        widget.configuration_description.as_deref(),
        Some("Choose which tasks to display")
    );
}

#[test]
fn parses_a_synchronous_placeholder_for_async_widget_entries() {
    let source = r#"
        struct TaskEntry { title: String }

        widget Tasks(
            placeholder: TaskEntry("Loading"),
            entry: await TaskData.load(),
            families: [Small],
            refreshSeconds: 1800
        ) {
            Text(entry.title)
        }
    "#;
    let program = nexa_syntax::parse_program(source).expect("widget should parse");
    assert!(matches!(
        &program.widgets[0].placeholder_provider,
        Some(Expr::Call(name, _, _, _)) if name == "TaskEntry"
    ));
    assert!(matches!(
        &program.widgets[0].entry_provider,
        Expr::Await(_, _)
    ));
}

#[test]
fn widget_localization_metadata_requires_nonempty_source_text() {
    let error = nexa_syntax::parse_program(
        r#"widget Tasks(displayName: "", entry: TaskData.load(), families: [Small], refreshSeconds: 1800) { Text("Tasks") }"#,
    )
    .expect_err("empty widget source text should be rejected");
    assert!(error.message.contains("text cannot be empty"));
}

#[test]
fn standalone_modules_can_declare_enums_for_widget_configuration() {
    let program = nexa_syntax::parse_program(
        r#"
        enum TaskFilter {
            today,
            upcoming
        }

        struct TaskConfiguration {
            filter: TaskFilter
        }

        widget Tasks(
            configuration: TaskConfiguration(TaskFilter.today),
            entry: TaskData.load(configuration),
            families: [Small],
            refreshSeconds: 1800
        ) {
            Text(entry.title)
        }
        "#,
    )
    .expect("standalone widget modules should allow enum declarations");
    assert_eq!(program.enums.len(), 1);
    assert_eq!(program.enums[0].name, "TaskFilter");
}
