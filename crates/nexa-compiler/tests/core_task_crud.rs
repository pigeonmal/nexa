//! Task CRUD can be app-authored from generic typed state, collection actions,
//! and reusable controls without task-specific framework code.

use std::{collections::HashMap, fs};

use nexa_compiler::{
    Target, compile_file_with_warnings_for_targets_and_plugin_roots, testing::run_tests,
};
use nexa_testkit::TestProject;

#[test]
fn typed_task_crud_example_compiles_for_both_targets() {
    std::thread::Builder::new()
        .name("nexa-todo-cross-target-compile-test".to_owned())
        // Plugin and multi-screen UI semantic lowering uses larger recursive
        // frames than Cargo's small test worker stack; match a normal compiler
        // process stack rather than weakening the test to a tiny UI fixture.
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            compile_todo_for_both_targets();
        })
        .expect("cross-target compile test thread should start")
        .join()
        .expect("Todo compile test should not panic");
}

fn compile_todo_for_both_targets() {
    let project = TestProject::new("nexa-todo-crud-ios");
    let entry = project.join("App.nx");
    let mut app_source = include_str!("../../../examples/archetypes/todo/App.nx").to_owned();
    app_source.push_str(
        r#"

test "soft-deleted records remain as hidden tombstones" {
    let tasks: Array<TaskItem> = [TaskItem(1, "Task", "", null, false, "None", false, null, false, 0, 0, 0)]
    let foundTask = findTask(tasks, 1)
    assert(foundTask != null)
    assert((foundTask?.title ?? "") == "Task")
    assert(findTask(tasks, 99) == null)
    let deletedTasks = deleteTodo(tasks, 1)
    assert(deletedTasks.count == 1)
    assert(deletedTasks[0].isDeleted)
    assert(!taskMatchesFilter(deletedTasks[0], 0, "", 0))

    let comments: Array<CommentItem> = [CommentItem(2, 1, "Note", "", 1000, false)]
    let deletedComments = deleteComment(comments, 2)
    assert(deletedComments.count == 1)
    assert(deletedComments[0].isDeleted)
    assert(activeTaskComments(deletedComments, 1).isEmpty)

    let reminders: Array<ReminderItem> = [ReminderItem(3, 1, "reminder-3", "", 1000, false)]
    let deletedReminders = deleteReminder(reminders, 3)
    assert(deletedReminders.count == 1)
    assert(deletedReminders[0].isDeleted)
    assert(activeTaskReminders(deletedReminders, 1).isEmpty)
}
"#,
    );
    fs::write(&entry, app_source).expect("test source should be written");
    let components = entry
        .parent()
        .expect("app has a parent directory")
        .join("components");
    fs::create_dir_all(&components).expect("component directory should be created");
    fs::write(
        components.join("CommentPanel.nx"),
        include_str!("../../../examples/archetypes/todo/components/CommentPanel.nx"),
    )
    .expect("comment panel source should be written");
    fs::write(
        components.join("TaskRowContent.nx"),
        include_str!("../../../examples/archetypes/todo/components/TaskRowContent.nx"),
    )
    .expect("task row content source should be written");
    let features = entry
        .parent()
        .expect("app has a parent directory")
        .join("features");
    fs::create_dir_all(&features).expect("feature directory should be created");
    fs::write(
        features.join("TodoOperations.nx"),
        include_str!("../../../examples/archetypes/todo/features/TodoOperations.nx"),
    )
    .expect("Todo operations source should be written");
    let models = entry
        .parent()
        .expect("app has a parent directory")
        .join("models");
    fs::create_dir_all(&models).expect("model directory should be created");
    fs::write(
        models.join("TaskItem.nx"),
        include_str!("../../../examples/archetypes/todo/models/TaskItem.nx"),
    )
    .expect("task model source should be written");
    fs::write(
        models.join("ReminderItem.nx"),
        include_str!("../../../examples/archetypes/todo/models/ReminderItem.nx"),
    )
    .expect("reminder model source should be written");
    let widgets = entry
        .parent()
        .expect("app has a parent directory")
        .join("widgets");
    fs::create_dir_all(&widgets).expect("widget directory should be created");
    fs::write(
        widgets.join("Tasks.nx"),
        include_str!("../../../examples/archetypes/todo/widgets/Tasks.nx"),
    )
    .expect("Todo widget source should be written");
    let storage = entry
        .parent()
        .expect("app has a parent directory")
        .join("storage");
    fs::create_dir_all(&storage).expect("storage directory should be created");
    let persistence_source =
        include_str!("../../../examples/archetypes/todo/storage/TodoPersistence.nx");
    fs::write(storage.join("TodoPersistence.nx"), persistence_source)
        .expect("persistence source should be written");
    let comment_persistence_source =
        include_str!("../../../examples/archetypes/todo/storage/CommentPersistence.nx");
    fs::write(
        storage.join("CommentPersistence.nx"),
        comment_persistence_source,
    )
    .expect("comment persistence source should be written");
    fs::write(
        storage.join("TodoDatabase.nx"),
        include_str!("../../../examples/archetypes/todo/storage/TodoDatabase.nx"),
    )
    .expect("shared database source should be written");
    let screens = entry
        .parent()
        .expect("app has a parent directory")
        .join("screens");
    fs::create_dir_all(&screens).expect("screen directory should be created");
    fs::write(
        screens.join("Completed.nx"),
        include_str!("../../../examples/archetypes/todo/screens/Completed.nx"),
    )
    .expect("completed screen source should be written");
    fs::write(
        screens.join("Main.nx"),
        include_str!("../../../examples/archetypes/todo/screens/Main.nx"),
    )
    .expect("main screen source should be written");
    fs::write(
        screens.join("Theme.nx"),
        include_str!("../../../examples/archetypes/todo/screens/Theme.nx"),
    )
    .expect("theme screen source should be written");

    let sqlite_plugin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/sqlite");
    let media_picker_root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/media-picker");
    let notifications_plugin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/notifications");
    let browser_plugin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/browser");
    let mail_composer_plugin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/mail-composer");
    let data_extractor_plugin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/data-extractor");
    let plugin_roots = HashMap::from([
        ("dev.nexa.sqlite".to_owned(), sqlite_plugin),
        ("dev.nexa.media-picker".to_owned(), media_picker_root),
        ("dev.nexa.notifications".to_owned(), notifications_plugin),
        ("dev.nexa.browser".to_owned(), browser_plugin),
        ("dev.nexa.mail-composer".to_owned(), mail_composer_plugin),
        ("dev.nexa.data-extractor".to_owned(), data_extractor_plugin),
    ]);
    let compiled = compile_file_with_warnings_for_targets_and_plugin_roots(
        &entry,
        &[Target::Swift, Target::Kotlin],
        &plugin_roots,
    )
    .expect("typed Todo flows and native plugins should compile");
    for compilation in &compiled {
        let report = run_tests(&compilation.tests);
        assert_eq!(report.passed, 1, "{:?}", report.failures);
        assert!(report.failures.is_empty());
    }
    let swift_module = &compiled[0].module;
    let kotlin_module = &compiled[1].module;
    for module in [swift_module, kotlin_module] {
        assert!(
            module
                .globals
                .iter()
                .any(|global| global.name == "todoPersistence")
        );
        assert!(module.functions.iter().any(|function| {
            function.receiver.is_some() && function.name.ends_with(".loadTasks")
        }));
        assert!(module.functions.iter().any(|function| {
            function.receiver.is_some() && function.name.ends_with(".saveTask")
        }));
    }
    assert!(
        swift_module
            .structs
            .iter()
            .any(|model| model.name == "TaskItem")
    );
    assert!(
        swift_module
            .structs
            .iter()
            .any(|model| model.name == "CommentItem")
    );
    assert!(swift_module.structs.iter().any(|model| {
        model.name == "CommentItem" && model.fields.iter().any(|field| field.name == "isDeleted")
    }));
    assert!(
        swift_module
            .states
            .iter()
            .any(|state| state.name == "tasks")
    );
    let generated = nexa_backend_swift::SwiftBackend.generate_units(swift_module);
    let generated_source = generated
        .units
        .iter()
        .map(|unit| unit.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(swift_module.widgets.len(), 1);
    let swift_widget = nexa_backend_swift::SwiftBackend
        .generate_widget_units(swift_module)
        .expect("Todo widget should generate a WidgetKit extension");
    let swift_widget_source = std::iter::once(swift_widget.sources.imports.as_str())
        .chain(
            swift_widget
                .sources
                .units
                .iter()
                .map(|unit| unit.contents.as_str()),
        )
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift_widget_source.contains("import WidgetKit"));
    assert!(swift_widget_source.contains("import AppIntents"));
    assert!(swift_widget_source.contains("AppIntentConfiguration("));
    assert!(swift_widget_source.contains("NexaGeneratedWidgetBundle"));
    assert!(swift_widget_source.contains("systemExtraLarge"));
    assert!(swift_widget_source.contains("accessibilityValue("));
    assert!(swift_widget_source.contains("FROM tasks WHERE isCompleted = ?"));
    assert!(!swift_widget_source.contains("widget.tasks"));
    assert!(!swift_widget_source.contains("nexa_todoPersistence.nexa_fn_loadTasks()"));
    assert!(generated_source.contains("final class NexaTodoPersistence"));
    assert!(
        generated_source
            .contains("nexa_todoPersistence: NexaTodoPersistence = NexaTodoPersistence(")
    );
    assert!(generated_source.contains("nexa_todoPersistence.nexa_fn_loadTasks()"));
    let (generated_kotlin_source, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(kotlin_module);
    assert!(
        generated_source
            .contains("UINotificationFeedbackGenerator().notificationOccurred(.success)")
    );
    assert!(generated_source.contains(".spring(response: 0.35, dampingFraction: 0.8)"));
    assert!(generated_kotlin_source.contains("HapticFeedbackConstants.CONFIRM"));
    let kotlin_widget = nexa_backend_kotlin::KotlinBackend
        .generate_widget_units(kotlin_module)
        .expect("Todo widget should generate an Android Glance widget");
    let kotlin_widget_source = kotlin_widget
        .sources
        .units
        .iter()
        .map(|unit| unit.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(kotlin_widget_source.contains("GlanceAppWidgetReceiver"));
    assert!(kotlin_widget_source.contains("SizeMode.Responsive("));
    assert!(kotlin_widget_source.contains("NexaTasksWidgetConfigurationActivity"));
    assert!(kotlin_widget_source.contains("contentDescription = run {"));
    assert!(kotlin_widget_source.contains("internal fun nexaWidgetFamily"));
    assert!(kotlin_widget_source.contains("FROM tasks WHERE isCompleted = ?"));
    assert!(!kotlin_widget_source.contains("widget.tasks"));
    assert!(!kotlin_widget_source.contains("nexa_todoPersistence.nexa_fn_loadTasks()"));
    assert!(
        kotlin_widget
            .resources
            .iter()
            .any(|resource| { resource.name == "drawable/nexa_widget_calendar_circle.xml" })
    );
    assert!(generated_kotlin_source.contains("class NexaTodoPersistence("));
    assert!(generated_kotlin_source.contains("style = MaterialTheme.typography.titleMedium"));
    assert!(generated_kotlin_source.contains("nexa_todoPersistence.nexa_fn_loadTasks()"));
    assert!(generated_source.contains("nexa_tasks.append(nexa_fn_makeTask("));
    assert!(generated_source.contains("nexa_fn_deleteTodo(nexa_tasks,"));
    assert!(generated_source.contains("nexa_field_title:"));
    assert!(generated_source.contains("!nexa_task.nexa_field_isCompleted"));
    assert!(generated_source.contains("nexa_field_taskDescription"));
    assert!(generated_source.contains("DatePicker("));
    assert!(generated_source.contains(".navigationBarTitleDisplayMode(.inline)"));
    assert!(generated_source.contains(".presentationDragIndicator(.visible)"));
    assert!(generated_source.contains("ToolbarItemGroup(placement: .navigationBarLeading)"));
    assert!(generated_source.contains("ToolbarItemGroup(placement: .navigationBarTrailing)"));
    assert!(generated_source.contains("Picker("));
    assert!(generated_source.contains("nexa_source.sorted {"));
    assert!(generated_source.contains("MediaPickerControl"));
    assert!(generated_source.contains("fullScreenCover(isPresented:"));
    assert!(generated_source.contains("TextField(\"Comment\", text: $nexa_editingCommentText)"));
    let localized_entries = nexa_ir::localization::extract(swift_module);
    assert!(localized_entries.contains_key("Comment image"));
    assert!(localized_entries.contains_key("Double tap to view full size"));
    assert!(generated_source.contains("nexa_field_taskID: nexa_taskID"));
    assert!(generated_source.contains("nexa_commentPersistence.nexa_fn_saveComment("));
    assert!(generated_source.contains("nexa_fn_deleteComment(nexa_comments"));
    assert!(generated_source.contains("nexa_fn_deleteReminder(nexa_reminders"));
    assert!(generated_source.contains("onChange(of: nexaScenePhase)"));
    assert!(generated_source.contains("isLocalPending"));
    assert!(generated_source.contains("scheduleLocalAt"));
    assert!(generated_source.contains("cancelLocal"));
    assert!(generated_source.contains("setBadgeCount"));
    assert!(generated_source.contains("settings.auto-reminder-minutes"));
    assert!(generated_source.contains("todo-auto-reminder-"));
    assert!(generated_source.contains("600000"));
    assert!(generated_kotlin_source.contains("AlertDialog("));
    assert!(generated_kotlin_source.contains("TextField("));
    assert!(generated_kotlin_source.contains("nexa_editingCommentText"));
    assert!(generated_kotlin_source.contains("nexa_commentPersistence.nexa_fn_saveComment("));
    assert!(generated_kotlin_source.contains("nexa_fn_deleteComment(nexa_comments"));
    assert!(generated_kotlin_source.contains("nexa_fn_deleteReminder(nexa_reminders"));
    assert!(generated_kotlin_source.contains("ON_RESUME"));
    assert!(
        generated_kotlin_source.contains("Modifier.semantics { contentDescription = \"Time\" }")
    );
    assert!(generated_kotlin_source.contains("isLocalPending"));
    assert!(generated_kotlin_source.contains("scheduleLocalAt"));
    assert!(generated_kotlin_source.contains("cancelLocal"));
    assert!(generated_kotlin_source.contains("setBadgeCount"));
    assert!(generated_kotlin_source.contains("settings.auto-reminder-minutes"));
    assert!(generated_kotlin_source.contains("todo-auto-reminder-"));
    assert!(generated_kotlin_source.contains("600000"));
}
