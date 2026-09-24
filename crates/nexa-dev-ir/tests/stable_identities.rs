use nexa_dev_ir::{IdentityKind, lower};
use nexa_ir::{DirectionConfig, DirectionStyle, Module, Node, NumericType, State, TextStyle, Type};

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
