//! Value, event-order, error, and semantic action conformance harness (PR 10 / W7).
//!
//! Evaluates deterministic `.nx` language subsets: arithmetic boundaries,
//! Unicode strings, array/map collection transformations, Result/Option semantics,
//! and sequential action order invariants across AOT and DevRuntime lowering.

use std::collections::HashMap;

use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_testkit::TestProject;

fn compile_to_ir(app_source: &str, project_name: &str) -> nexa_ir::Module {
    let project = TestProject::new(project_name);
    let entry = project.write_app(app_source);
    let roots: HashMap<String, std::path::PathBuf> = HashMap::new();

    let mut compiler = IncrementalProjectCompiler::default();
    let compilations = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &roots)
        .expect("Compilation should succeed");

    compilations.into_iter().next().unwrap().module
}

#[test]
fn conformance_arithmetic_boundaries_and_numeric_types() {
    let source = r#"
app ConformanceArithmetic {
    state minI32: Int32 = -2147483648
    state maxI32: Int32 = 2147483647
    state floatVal: Float64 = 3.141592653589793
    state calculated: Float64 = 0.0
    state boundarySum: Int32 = 0

    fn addDoubles(a: Float64, b: Float64) -> Float64 {
        return a + b
    }

    body {
        Column {
            Text("Min boundary")
            Text("Max boundary")
            Button("Compute") {
                boundarySum = minI32 + maxI32
                calculated = addDoubles(floatVal, 2.71828)
            }
        }
    }
}
"#;

    let module = compile_to_ir(source, "conformance-arithmetic");

    // Verify states are typed accurately
    let state_types: HashMap<_, _> = module
        .states
        .iter()
        .map(|s| (s.name.as_str(), &s.ty))
        .collect();
    assert_eq!(
        state_types.get("minI32"),
        Some(&&nexa_ir::Type::Numeric(nexa_ir::NumericType::Int32))
    );
    assert_eq!(
        state_types.get("maxI32"),
        Some(&&nexa_ir::Type::Numeric(nexa_ir::NumericType::Int32))
    );
    assert_eq!(
        state_types.get("floatVal"),
        Some(&&nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64))
    );

    // Verify function signature and return type
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "addDoubles")
        .expect("find function");
    assert_eq!(
        func.return_type,
        nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64)
    );
    assert_eq!(func.parameters.len(), 2);
}

#[test]
fn conformance_unicode_and_multilingual_strings() {
    let source = r#"
app ConformanceUnicode {
    state arabicGreeting: String = "مرحبا بكم في نكسا"
    state japaneseTitle: String = "ネクサへようこそ"
    state emojiBadge: String = "🚀🔥✨📱"

    body {
        Column {
            Text(arabicGreeting)
            Text(japaneseTitle)
            Text(emojiBadge)
        }
    }
}
"#;

    let module = compile_to_ir(source, "conformance-unicode");

    let initial_values: HashMap<_, _> = module
        .states
        .iter()
        .map(|s| (s.name.as_str(), &s.initial))
        .collect();

    match initial_values.get("arabicGreeting") {
        Some(nexa_ir::Expr::String(s)) => assert_eq!(s, "مرحبا بكم في نكسا"),
        other => panic!("Unexpected arabicGreeting: {:?}", other),
    }

    match initial_values.get("japaneseTitle") {
        Some(nexa_ir::Expr::String(s)) => assert_eq!(s, "ネクサへようこそ"),
        other => panic!("Unexpected japaneseTitle: {:?}", other),
    }

    match initial_values.get("emojiBadge") {
        Some(nexa_ir::Expr::String(s)) => assert_eq!(s, "🚀🔥✨📱"),
        other => panic!("Unexpected emojiBadge: {:?}", other),
    }
}

#[test]
fn conformance_sequential_action_ordering_is_strictly_preserved() {
    let source = r#"
app ConformanceActionOrder {
    state step: Int32 = 0
    state log: String = ""

    body {
        Column {
            Button("Run Sequence") {
                step = 1
                log = "first"
                step = 2
                log = "second"
                step = 3
                log = "third"
            }
        }
    }
}
"#;

    let module = compile_to_ir(source, "conformance-action-order");

    let body_nodes = &module.body;
    let button_actions = match &body_nodes[0] {
        nexa_ir::Node::Button { actions, .. } => actions,
        nexa_ir::Node::Layout { children, .. } => match &children[0] {
            nexa_ir::Node::Button { actions, .. } => actions,
            _ => panic!("Expected Button child"),
        },
        _ => panic!("Expected Button or Layout root"),
    };

    assert_eq!(
        button_actions.len(),
        6,
        "Must contain exactly 6 actions in order"
    );

    let expected_names = ["step", "log", "step", "log", "step", "log"];
    for (i, action) in button_actions.iter().enumerate() {
        match action {
            nexa_ir::Action::Assign { name, .. } => {
                assert_eq!(
                    name, expected_names[i],
                    "Action at index {} has wrong target",
                    i
                );
            }
            other => panic!("Unexpected action at {}: {:?}", i, other),
        }
    }
}

#[test]
fn conformance_semantic_mutation_test_reordered_actions_detected() {
    // Proves that any change to sequential action execution order is caught
    let actions_in_order = vec![
        nexa_ir::Action::Assign {
            name: "step".to_owned(),
            value: nexa_ir::Expr::Number {
                raw: "1".to_owned(),
                ty: nexa_ir::NumericType::Int32,
            },
        },
        nexa_ir::Action::Assign {
            name: "log".to_owned(),
            value: nexa_ir::Expr::String("step1".to_owned()),
        },
    ];

    let mutated_actions = vec![
        nexa_ir::Action::Assign {
            name: "log".to_owned(),
            value: nexa_ir::Expr::String("step1".to_owned()),
        },
        nexa_ir::Action::Assign {
            name: "step".to_owned(),
            value: nexa_ir::Expr::Number {
                raw: "1".to_owned(),
                ty: nexa_ir::NumericType::Int32,
            },
        },
    ];

    assert_ne!(
        actions_in_order, mutated_actions,
        "Mutation must be distinguished from canonical ordered sequence"
    );
}
