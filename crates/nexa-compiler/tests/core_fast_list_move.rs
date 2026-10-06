//! Reorder callbacks remain statically typed and generate native list paths.

use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Node;
use nexa_testkit::TestProject;

#[test]
fn native_array_fast_list_move_generates_swiftui_and_compose_reordering() {
    let project = TestProject::new("nexa-fast-list-move");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app ReorderExample {
    state values: Array<Int32> = [1, 2, 3]
    state canMove: Bool = true
    body {
        FastList(values, native: true) { value, index in
            Text(value)
        }.onMove(enabled: canMove) { from, to ->
            values.move(from, to)
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("native FastList move should compile for both targets")
            .module;
        let mut found_reorder = false;
        nexa_ir::walk::walk_ir(
            &module.body,
            &mut |node| {
                found_reorder |=
                    matches!(node, Node::FastList { plan } if plan.on_move().is_some());
            },
            &mut |_| {},
        );
        assert!(
            found_reorder,
            "move callback must remain present in typed IR"
        );

        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        let expected = match target {
            Target::Swift => ".onMove { source, destination in",
            Target::Kotlin => "detectDragGesturesAfterLongPress(",
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains(expected), "{native}");
        let enabled_check = match target {
            Target::Swift => ".moveDisabled(!(nexa_canMove))",
            Target::Kotlin => "if (nexa_canMove)",
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains(enabled_check), "{native}");
    }
}

#[test]
fn filtered_array_subset_move_generates_for_both_native_targets() {
    let project = TestProject::new("nexa-fast-list-move-subset");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
struct Task { id: Int32, done: Bool }
app ReorderExample {
    state tasks: Array<Task> = []
    body {
        FastList(tasks.filter { task -> !task.done }, native: true) { task, index in
            Text(task.id)
        }.onMove { from, to ->
            tasks.moveSubset(from, to, tasks.filter { task -> !task.done })
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("filtered subset reordering should compile for both targets")
            .module;
        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains("nexaOriginalSubset"), "{native}");
        assert!(native.contains("nexaReorderedSubset"), "{native}");
    }
}
