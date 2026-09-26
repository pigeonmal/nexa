use std::collections::HashMap;

use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_testkit::TestProject;

#[test]
fn unchanged_project_reuses_parsed_imports_and_reparses_only_changed_source() {
    let project = TestProject::new("nexa-incremental-project");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"Card.nx\"\napp Demo { body { Card() } }\n",
    );
    project.write("Card.nx", "component Card() { body { Text(\"First\") } }\n");
    let mut compiler = IncrementalProjectCompiler::default();
    let plugins = HashMap::new();

    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile initial project");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 2,
            reused_source_files: 0,
        }
    );

    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile unchanged project");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 0,
            reused_source_files: 2,
        }
    );

    project.write(
        "Card.nx",
        "component Card() { body { Text(\"Updated\") } }\n",
    );
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile changed imported component");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 1,
            reused_source_files: 1,
        }
    );
}

#[test]
fn removed_imports_are_pruned_from_incremental_source_cache() {
    let project = TestProject::new("nexa-incremental-project-prune");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"Card.nx\"\napp Demo { body { Card() } }\n",
    );
    project.write("Card.nx", "component Card() { body { Text(\"Card\") } }\n");
    let mut compiler = IncrementalProjectCompiler::default();
    let plugins = HashMap::new();
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile imported project");

    project.write("App.nx", "app Demo { body { Text(\"No import\") } }\n");
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile project after removing import");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 1);

    project.write(
        "App.nx",
        "import \"Card.nx\"\napp Demo { body { Card() } }\n",
    );
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile project after restoring import");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 2,
            reused_source_files: 0,
        }
    );
}
