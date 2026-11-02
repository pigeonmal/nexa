//! Clean vs cached compiler equivalence tests (PR 07 / F-11).
//!
//! Verifies that compiling incrementally with a warm `IncrementalProjectCompiler`
//! cache produces 100% semantically and structurally identical IR to a clean,
//! cold-start compile across a realistic 50+ edit history.

use std::collections::HashMap;
use std::fs;

use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_testkit::TestProject;

fn normalize_module_ir(module: &nexa_ir::Module) -> serde_json::Value {
    serde_json::to_value(module).expect("serialize module to JSON")
}

#[test]
fn test_50_plus_incremental_edits_match_clean_compilation_identically() {
    let project = TestProject::new("clean-vs-cached-history");
    let entry = project.write_app(
        r#"
app CleanCachedEquivalence {
    state counter: Int32 = 0

    body {
        Column {
            Text("Initial Header")
            Text("Counter active")
        }
    }
}
"#,
    );

    let roots: HashMap<String, std::path::PathBuf> = HashMap::new();
    let target = Target::Swift;

    let mut warm_compiler = IncrementalProjectCompiler::default();

    // Generate 52 sequential, realistic developer edits
    let mut revisions = Vec::new();

    // 1-10: Rapid text and styling edits
    for i in 1..=10 {
        revisions.push(format!(
            r#"
app CleanCachedEquivalence {{
    state counter: Int32 = {i}

    body {{
        Column(gap: {spacing}) {{
            Text("Header revision {i}")
            Text("Running counter")
        }}
    }}
}}
"#,
            spacing = i * 2
        ));
    }

    // 11-20: Adding state variables and buttons
    for i in 11..=20 {
        revisions.push(format!(
            r#"
app CleanCachedEquivalence {{
    state counter: Int32 = {i}
    state flag: Bool = true
    state label: String = "Label {i}"

    body {{
        Column(gap: 12) {{
            Text(label)
            Button("Increment") {{
                counter = counter + 1
            }}
        }}
    }}
}}
"#
        ));
    }

    // 21-30: Adding helper functions and expressions
    for i in 21..=30 {
        revisions.push(format!(
            r#"
app CleanCachedEquivalence {{
    state counter: Int32 = {i}

    fn calculate(base: Int32) -> Int32 {{
        return base * 2 + {i}
    }}

    body {{
        Column {{
            Text("Calculated output")
            Button("Step {i}") {{
                counter = calculate(counter)
            }}
        }}
    }}
}}
"#
        ));
    }

    // 31-40: Adding multiple components and nested layouts
    for i in 31..=40 {
        revisions.push(format!(
            r#"
component CardItem(title: String, score: String) {{
    body {{
        Row(gap: 8) {{
            Text(title)
            Text(score)
        }}
    }}
}}

app CleanCachedEquivalence {{
    state counter: Int32 = {i}

    body {{
        Column {{
            CardItem(title: "Item A", score: "Score A")
            CardItem(title: "Item B", score: "Score B")
            Text("Total revisions: {i}")
        }}
    }}
}}
"#
        ));
    }

    // 41-52: Adding Screens and Navigation
    for i in 41..=52 {
        revisions.push(format!(
            r#"
component CardItem(title: String, score: String) {{
    body {{
        Row(gap: 8) {{
            Text(title)
            Text(score)
        }}
    }}
}}

screen DetailScreen {{
    Column {{
        Text("Detail for item {i}")
        NavigationBack(label: "Back")
    }}
}}

screen HomeScreen {{
    Column {{
        CardItem(title: "Item A", score: "Score A")
        NavigationLink(destination: DetailScreen) {{
            Text("View Details {i}")
        }}
    }}
}}

app CleanCachedEquivalence {{
    state counter: Int32 = {i}

    body {{
        NavigationStack(root: HomeScreen)
    }}
}}
"#
        ));
    }

    assert!(revisions.len() >= 50, "Must test at least 50 revisions");

    for (idx, source) in revisions.iter().enumerate() {
        fs::write(&entry, source).expect("write edited source");

        // 1. Warm incremental compilation
        let warm_compilations = warm_compiler
            .compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
                &entry,
                &[target],
                &roots,
            )
            .unwrap_or_else(|e| panic!("Warm compile failed at revision {}: {:?}", idx + 1, e));

        let warm_ir = normalize_module_ir(&warm_compilations[0].module);

        // 2. Fresh clean compilation
        let mut cold_compiler = IncrementalProjectCompiler::default();
        let cold_compilations = cold_compiler
            .compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
                &entry,
                &[target],
                &roots,
            )
            .unwrap_or_else(|e| panic!("Cold compile failed at revision {}: {:?}", idx + 1, e));

        let cold_ir = normalize_module_ir(&cold_compilations[0].module);

        // Invariant: Warm compiler output == Cold compiler output
        assert_eq!(
            warm_ir,
            cold_ir,
            "Revision {} diverged between incremental cached compile and clean cold compile!",
            idx + 1
        );
    }
}
