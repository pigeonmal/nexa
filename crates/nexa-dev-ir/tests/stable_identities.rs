use nexa_dev_ir::{IdentityKind, lower};
use nexa_ir::{
    Action, Component, ComponentParameter, DirectionConfig, DirectionStyle, Expr, Module, Node,
    NumericType, State, TextStyle, Type, ViewTransition,
};

fn module(body: Vec<Node>, state_type: Type) -> Module {
    Module {
        app_name: "Demo".to_owned(),
        plugins: Vec::new(),
        plugin_assets: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        states: vec![State {
            name: "count".to_owned(),
            ty: state_type,
            initial: nexa_ir::Expr::Number {
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

fn text(value: &str) -> Node {
    Node::Text {
        value: nexa_ir::Expr::String(value.to_owned()),
        style: TextStyle::default(),
    }
}

fn double_tap_pressable(value: &str) -> Node {
    Node::Pressable {
        disabled: Expr::Bool(false),
        haptic: None,
        children: vec![text("Double tap")],
        actions: Vec::new(),
        double_tap_actions: vec![Action::Assign {
            name: "count".to_owned(),
            value: Expr::Number {
                raw: value.to_owned(),
                ty: NumericType::Int32,
            },
        }],
        long_press_duration_ms: Expr::Number {
            raw: "500".to_owned(),
            ty: NumericType::Int32,
        },
        long_press_actions: Vec::new(),
        drag_parameters: Vec::new(),
        drag_actions: Vec::new(),
        pinch_parameter: None,
        pinch_actions: Vec::new(),
    }
}

fn pinch_pressable(value: &str) -> Node {
    Node::Pressable {
        disabled: Expr::Bool(false),
        haptic: None,
        children: vec![text("Pinch")],
        actions: Vec::new(),
        double_tap_actions: Vec::new(),
        long_press_duration_ms: Expr::Number {
            raw: "500".to_owned(),
            ty: NumericType::Int32,
        },
        long_press_actions: Vec::new(),
        drag_parameters: Vec::new(),
        drag_actions: Vec::new(),
        pinch_parameter: Some("scaleFactor".to_owned()),
        pinch_actions: vec![Action::Assign {
            name: "count".to_owned(),
            value: Expr::Number {
                raw: value.to_owned(),
                ty: NumericType::Float64,
            },
        }],
    }
}

fn long_pressable(duration_ms: &str) -> Node {
    Node::Pressable {
        disabled: Expr::Bool(false),
        haptic: None,
        children: vec![text("Long press")],
        actions: Vec::new(),
        double_tap_actions: Vec::new(),
        long_press_duration_ms: Expr::Number {
            raw: duration_ms.to_owned(),
            ty: NumericType::Int32,
        },
        long_press_actions: vec![Action::Assign {
            name: "count".to_owned(),
            value: Expr::Number {
                raw: "1".to_owned(),
                ty: NumericType::Int32,
            },
        }],
        drag_parameters: Vec::new(),
        drag_actions: Vec::new(),
        pinch_parameter: None,
        pinch_actions: Vec::new(),
    }
}

#[test]
fn double_tap_action_changes_are_carried_by_a_hot_reload_patch() {
    let original = lower(
        &module(
            vec![double_tap_pressable("1")],
            Type::Numeric(NumericType::Int32),
        ),
        "revision-1",
    );
    let updated = lower(
        &module(
            vec![double_tap_pressable("2")],
            Type::Numeric(NumericType::Int32),
        ),
        "revision-2",
    );

    let payload = serde_json::to_value(&updated.module).expect("Dev IR serializes");
    assert!(payload["body"][0]["Pressable"]["double_tap_actions"].is_array());

    let patch = nexa_dev_ir::diff(&original, &updated).expect("changed action is patchable");
    assert_eq!(patch.operations.len(), 1);
    assert!(patch.operations[0].path.contains("/double_tap_actions/"));
    assert_eq!(patch.operations[0].value, Some(serde_json::json!("2")));
}

#[test]
fn pinch_callback_changes_are_carried_by_a_hot_reload_patch() {
    let original = lower(
        &module(
            vec![pinch_pressable("1.0")],
            Type::Numeric(NumericType::Int32),
        ),
        "revision-1",
    );
    let updated = lower(
        &module(
            vec![pinch_pressable("1.5")],
            Type::Numeric(NumericType::Int32),
        ),
        "revision-2",
    );

    let payload = serde_json::to_value(&updated.module).expect("Dev IR serializes");
    assert_eq!(
        payload["body"][0]["Pressable"]["pinch_parameter"],
        "scaleFactor"
    );
    assert!(payload["body"][0]["Pressable"]["pinch_actions"].is_array());

    let patch = nexa_dev_ir::diff(&original, &updated).expect("pinch callback is patchable");
    assert_eq!(patch.operations.len(), 1);
    assert!(patch.operations[0].path.contains("/pinch_actions/"));
    assert_eq!(patch.operations[0].value, Some(serde_json::json!("1.5")));
}

#[test]
fn long_press_duration_changes_are_carried_by_a_hot_reload_patch() {
    let original = lower(
        &module(
            vec![long_pressable("500")],
            Type::Numeric(NumericType::Int32),
        ),
        "revision-1",
    );
    let updated = lower(
        &module(
            vec![long_pressable("750")],
            Type::Numeric(NumericType::Int32),
        ),
        "revision-2",
    );

    let payload = serde_json::to_value(&updated.module).expect("Dev IR serializes");
    assert_eq!(
        payload["body"][0]["Pressable"]["long_press_duration_ms"]["Number"]["raw"],
        "750"
    );

    let patch = nexa_dev_ir::diff(&original, &updated).expect("duration is patchable");
    assert_eq!(patch.operations.len(), 1);
    assert!(
        patch.operations[0]
            .path
            .contains("/long_press_duration_ms/")
    );
    assert_eq!(patch.operations[0].value, Some(serde_json::json!("750")));
}

#[test]
fn conditional_transition_changes_are_carried_by_a_hot_reload_patch() {
    let conditional = |transition| Node::If {
        condition: Expr::State("visible".to_owned(), Type::Bool),
        then_body: vec![text("Visible")],
        else_body: None,
        transition,
    };
    let original = lower(&module(vec![conditional(None)], Type::Bool), "revision-1");
    let updated = lower(
        &module(vec![conditional(Some(ViewTransition::Fade))], Type::Bool),
        "revision-2",
    );

    let payload = serde_json::to_value(&updated.module).expect("Dev IR serializes");
    assert_eq!(payload["body"][0]["If"]["transition"], "Fade");
    let patch = nexa_dev_ir::diff(&original, &updated).expect("transition change is patchable");
    assert_eq!(patch.operations.len(), 1);
    assert_eq!(patch.operations[0].path, "/body/0/If/transition");
    assert_eq!(patch.operations[0].value, Some(serde_json::json!("Fade")));
}

fn component_call(name: &str) -> Node {
    Node::ComponentCall {
        name: name.to_owned(),
        arguments: vec![("title".to_owned(), Expr::String("Title".to_owned()))],
        children: Some(vec![text("Projected child")]),
    }
}

fn component(name: &str) -> Component {
    Component {
        name: name.to_owned(),
        source_file: None,
        parameters: vec![ComponentParameter {
            name: "title".to_owned(),
            ty: Type::String,
        }],
        states: Vec::new(),
        body: vec![text("Component body"), Node::Content],
    }
}

#[test]
fn state_identity_uses_the_declaration_name_not_its_type() {
    let before = lower(&module(Vec::new(), Type::Numeric(NumericType::Int32)), "a");
    let after = lower(&module(Vec::new(), Type::String), "b");

    let before_state = before
        .identities
        .iter()
        .find(|identity| identity.kind == IdentityKind::AppState)
        .expect("state identity");
    let after_state = after
        .identities
        .iter()
        .find(|identity| identity.kind == IdentityKind::AppState)
        .expect("state identity");

    assert_eq!(before_state.id, "app/state/count");
    assert_eq!(before_state.id, after_state.id);
}

#[test]
fn inserting_a_different_sibling_kind_preserves_existing_node_identity() {
    let original = lower(
        &module(vec![text("Keep")], Type::Numeric(NumericType::Int32)),
        "a",
    );
    let updated = lower(
        &module(
            vec![
                Node::Button {
                    label: nexa_ir::Expr::String("New".to_owned()),
                    icon: None,
                    loading: None,
                    disabled: None,
                    actions: Vec::new(),
                },
                text("Keep"),
            ],
            Type::Numeric(NumericType::Int32),
        ),
        "b",
    );

    let original_text = original
        .identities
        .iter()
        .find(|identity| identity.id == "app/body/Text:0")
        .expect("text identity");
    let updated_text = updated
        .identities
        .iter()
        .find(|identity| identity.id == "app/body/Text:0")
        .expect("text identity");

    assert_eq!(original_text, updated_text);
}

#[test]
fn diff_sends_only_the_changed_expression_path() {
    let original = lower(
        &module(vec![text("Before")], Type::Numeric(NumericType::Int32)),
        "revision-1",
    );
    let updated = lower(
        &module(vec![text("After")], Type::Numeric(NumericType::Int32)),
        "revision-2",
    );

    let patch = nexa_dev_ir::diff(&original, &updated).expect("module change should be patchable");

    assert_eq!(patch.base_revision, "revision-1");
    assert_eq!(patch.revision, "revision-2");
    assert_eq!(patch.operations.len(), 1);
    assert_eq!(patch.operations[0].path, "/body/0/Text/value/String");
    assert_eq!(patch.operations[0].value, Some(serde_json::json!("After")));
}

#[test]
fn diff_replaces_an_array_when_its_shape_changes() {
    let original = lower(
        &module(vec![text("Before")], Type::Numeric(NumericType::Int32)),
        "revision-1",
    );
    let updated = lower(
        &module(
            vec![text("Before"), text("Added")],
            Type::Numeric(NumericType::Int32),
        ),
        "revision-2",
    );

    let patch = nexa_dev_ir::diff(&original, &updated).expect("module change should be patchable");

    assert_eq!(patch.operations.len(), 1);
    assert_eq!(patch.operations[0].path, "/body");
    assert!(patch.operations[0].value.is_some());
}

#[test]
fn diff_hot_reloads_custom_component_renames_and_removals() {
    let mut original_module = module(
        vec![component_call("Card")],
        Type::Numeric(NumericType::Int32),
    );
    original_module.components.push(component("Card"));
    let original = lower(&original_module, "revision-1");

    let mut renamed_module = module(
        vec![component_call("ProductCard")],
        Type::Numeric(NumericType::Int32),
    );
    renamed_module.components.push(component("ProductCard"));
    let renamed = lower(&renamed_module, "revision-2");
    let rename_patch = nexa_dev_ir::diff(&original, &renamed)
        .expect("renaming a custom component should be patchable");

    assert!(rename_patch.operations.iter().any(|operation| {
        operation.path == "/components/0/name"
            && operation.value == Some(serde_json::json!("ProductCard"))
    }));
    assert!(rename_patch.operations.iter().any(|operation| {
        operation.path == "/body/0/ComponentCall/name"
            && operation.value == Some(serde_json::json!("ProductCard"))
    }));

    let mut removed_module = module(
        vec![text("Standalone after component removal")],
        Type::Numeric(NumericType::Int32),
    );
    removed_module.components.clear();
    let removed = lower(&removed_module, "revision-3");
    let remove_patch =
        nexa_dev_ir::diff(&renamed, &removed).expect("removing a component should be patchable");

    assert!(remove_patch.operations.iter().any(|operation| {
        operation.path == "/components" && operation.value == Some(serde_json::json!([]))
    }));
    assert!(remove_patch.operations.iter().any(|operation| {
        operation.path == "/body/0/Text"
            && operation
                .value
                .as_ref()
                .and_then(|node| node.pointer("/value/String"))
                == Some(&serde_json::json!("Standalone after component removal"))
    }));
}

#[test]
fn dev_module_round_trips_as_a_typed_json_payload() {
    let dev_module = lower(
        &module(vec![text("Hello")], Type::Numeric(NumericType::Int32)),
        "r1",
    );
    let encoded = serde_json::to_string(&dev_module).expect("serialize DevModule");
    let decoded: nexa_dev_ir::DevModule =
        serde_json::from_str(&encoded).expect("deserialize DevModule");

    assert_eq!(decoded.revision, "r1");
    assert_eq!(decoded.identities, dev_module.identities);
}
