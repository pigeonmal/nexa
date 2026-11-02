//! Differential test proving patch application equivalence with full module lowering.
//!
//! For every revision edit, applying `diff(previous, next)` to `previous.module`
//! MUST produce a JSON object equal to `next.module`.

use nexa_dev_ir::{apply_patch, diff, lower};
use nexa_ir::{
    Action, Color, ColorValue, DirectionConfig, DirectionStyle, Expr, LayoutKind, Module, Node,
    NumericType, Screen, ScreenId, State, TextStyle, Type, ViewStyle,
};

fn base_module(body: Vec<Node>) -> Module {
    Module {
        widgets: Vec::new(),
        app_name: "ParityApp".to_owned(),
        plugins: Vec::new(),
        plugin_assets: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        background_tasks: Vec::new(),
        globals: Vec::new(),
        states: vec![State {
            name: "counter".to_owned(),
            ty: Type::Numeric(NumericType::Int32),
            initial: Expr::Number {
                raw: "0".to_owned(),
                ty: NumericType::Int32,
            },
            mutable: true,
        }],
        screens: Vec::new(),
        components: Vec::new(),
        body,
        status_bar: None,
        direction: Some(DirectionConfig {
            style: DirectionStyle::Ltr,
        }),
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
        on_active: None,
        on_inactive: None,
        on_background: None,
    }
}

fn text(val: &str) -> Node {
    Node::Text {
        value: Expr::String(val.to_owned()),
        style: TextStyle::default(),
    }
}

#[test]
fn single_styling_and_text_patch_equals_full_module() {
    let m1 = base_module(vec![text("Hello World")]);
    let mut m2 = base_module(vec![Node::Text {
        value: Expr::String("Updated Nexa".to_owned()),
        style: TextStyle {
            font_size: Some(24.0),
            ..Default::default()
        },
    }]);
    m2.app_name = "ParityAppUpdated".to_owned();

    let dev1 = lower(&m1, "rev:1");
    let dev2 = lower(&m2, "rev:2");

    let patch = diff(&dev1, &dev2).expect("diff should produce patch");
    let mut current_json = serde_json::to_value(&dev1.module).expect("serialize dev1");
    apply_patch(&mut current_json, &patch).expect("apply patch should succeed");

    let expected_json = serde_json::to_value(&dev2.module).expect("serialize dev2");
    assert_eq!(
        current_json, expected_json,
        "Applied patch must match full module lowering exactly"
    );
}

#[test]
fn multi_step_patch_sequence_matches_each_full_module_revision() {
    let current_module = base_module(vec![text("Step 0")]);
    let mut current_dev = lower(&current_module, "rev:0");
    let mut accumulated_json = serde_json::to_value(&current_dev.module).expect("serialize step 0");

    let steps = vec![
        // Step 1: Add a second node
        base_module(vec![text("Step 0"), text("Step 1 Added")]),
        // Step 2: Wrap into Layout
        base_module(vec![Node::Layout {
            kind: LayoutKind::Column,
            gap: 8.0,
            style: ViewStyle::default(),
            children: vec![text("Header"), text("Body")],
        }]),
        // Step 3: Modify State
        {
            let mut m = base_module(vec![text("With State")]);
            m.states.push(State {
                name: "active".to_owned(),
                ty: Type::Bool,
                initial: Expr::Bool(true),
                mutable: true,
            });
            m
        },
        // Step 4: Add Screen
        {
            let mut m = base_module(vec![text("Root Screen")]);
            m.screens.push(Screen {
                id: ScreenId(1),
                name: "Details".to_owned(),
                parameters: vec![],
                states: vec![],
                body: vec![text("Details Screen Content")],
                status_bar: None,
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            });
            m
        },
        // Step 5: Replace with Button
        base_module(vec![Node::Button {
            label: Expr::String("Click Me".to_owned()),
            icon: None,
            loading: None,
            disabled: Some(Expr::Bool(false)),
            style: None,
            size: None,
            shape: None,
            tint: None,
            glass: false,
            accessibility_label: None,
            actions: vec![Action::Assign {
                name: "counter".to_owned(),
                value: Expr::Number {
                    raw: "1".to_owned(),
                    ty: NumericType::Int32,
                },
            }],
        }]),
    ];

    for (i, next_module) in steps.into_iter().enumerate() {
        let rev = format!("rev:{}", i + 1);
        let next_dev = lower(&next_module, &rev);

        let patch = diff(&current_dev, &next_dev)
            .unwrap_or_else(|| panic!("diff should produce patch for step {}", i + 1));
        apply_patch(&mut accumulated_json, &patch)
            .unwrap_or_else(|e| panic!("apply patch failed at step {}: {}", i + 1, e));

        let expected_json = serde_json::to_value(&next_dev.module).expect("serialize next_dev");
        assert_eq!(
            accumulated_json,
            expected_json,
            "Step {} accumulated patch output must match full module lowering",
            i + 1
        );

        current_dev = next_dev;
    }
}

#[test]
fn adding_optional_presentation_property_emits_set_and_matches_full_module() {
    let m1 = base_module(vec![Node::Text {
        value: Expr::String("Title".to_owned()),
        style: TextStyle::default(),
    }]);
    let m2 = base_module(vec![Node::Text {
        value: Expr::String("Title".to_owned()),
        style: TextStyle {
            font_size: Some(18.0),
            ..Default::default()
        },
    }]);

    let dev1 = lower(&m1, "rev:1");
    let dev2 = lower(&m2, "rev:2");

    let patch = diff(&dev1, &dev2).expect("diff should produce patch");
    assert!(!patch.operations.is_empty());
    assert!(
        patch
            .operations
            .iter()
            .any(|op| op.path.ends_with("/font_size"))
    );

    let mut current_json = serde_json::to_value(&dev1.module).expect("serialize dev1");
    apply_patch(&mut current_json, &patch).expect("apply patch should succeed");

    let expected_json = serde_json::to_value(&dev2.module).expect("serialize dev2");
    assert_eq!(
        current_json, expected_json,
        "Applied patch for new optional property must match full module lowering exactly"
    );
}

#[test]
fn static_background_color_patch_uses_nested_color_value_paths() {
    let make_module = |red| {
        base_module(vec![Node::Layout {
            kind: LayoutKind::Column,
            gap: 0.0,
            style: ViewStyle {
                background: Some(ColorValue::Static(Color {
                    red,
                    green: 40,
                    blue: 80,
                    alpha: 255,
                })),
                ..ViewStyle::default()
            },
            children: Vec::new(),
        }])
    };
    let previous = lower(&make_module(20), "color:1");
    let next = lower(&make_module(240), "color:2");
    let patch = diff(&previous, &next).expect("background changes produce a patch");

    assert!(
        patch
            .operations
            .iter()
            .any(|operation| { operation.path == "/body/0/Layout/style/background/Static/red" })
    );

    let mut patched = serde_json::to_value(&previous.module).expect("serialize previous module");
    apply_patch(&mut patched, &patch).expect("apply background patch");
    assert_eq!(
        patched,
        serde_json::to_value(&next.module).expect("serialize next module")
    );
}
