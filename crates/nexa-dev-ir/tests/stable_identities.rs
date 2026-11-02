use std::collections::BTreeMap;

use nexa_dev_ir::{IdentityKind, lower};
use nexa_ir::{
    Action, Component, ComponentParameter, DirectionConfig, DirectionStyle, Expr, Module, Node,
    NumericType, State, TextStyle, Type, ViewTransition,
    adaptive_layout::{LayoutEdge, SafeAreaInsetPlan, ScrollOwnerResolution},
};

fn module(body: Vec<Node>, state_type: Type) -> Module {
    Module {
        widgets: Vec::new(),
        app_name: "Demo".to_owned(),
        plugins: Vec::new(),
        plugin_assets: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        background_tasks: Vec::new(),
        globals: Vec::new(),
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
        fill_max_size: true,
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
        context_menu: Vec::new(),
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
        fill_max_size: false,
        children: vec![text("Pinch")],
        actions: Vec::new(),
        double_tap_actions: Vec::new(),
        long_press_duration_ms: Expr::Number {
            raw: "500".to_owned(),
            ty: NumericType::Int32,
        },
        long_press_actions: Vec::new(),
        context_menu: Vec::new(),
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
        fill_max_size: false,
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
        context_menu: Vec::new(),
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
    assert!(payload["body"][0]["Pressable"]["context_menu"].is_array());
    assert_eq!(payload["body"][0]["Pressable"]["fill_max_size"], true);

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
        transition_animation: None,
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
        component_ref: None,
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
        ref_methods: Vec::new(),
        body: vec![text("Component body"), Node::Content],
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
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
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    accessibility_label: None,
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
fn shape_primitive_has_a_stable_dev_identity_and_typed_payload() {
    let dev = lower(
        &module(
            vec![Node::Shape {
                shape: nexa_ir::ButtonShape::Capsule,
                width: 29.25,
                height: 3.51,
                fill: nexa_ir::ColorValue::Static(nexa_ir::Color {
                    red: 255,
                    green: 255,
                    blue: 255,
                    alpha: 255,
                }),
                width_expression: None,
                height_expression: None,
                animation: None,
            }],
            Type::Numeric(NumericType::Int32),
        ),
        "shape",
    );

    assert!(dev.identities.iter().any(|identity| {
        identity.id == "app/body/Shape:0" && identity.kind == IdentityKind::Node
    }));
    let payload = serde_json::to_value(&dev.module).expect("shape payload serializes");
    assert_eq!(payload["body"][0]["Shape"]["shape"], "Capsule");
    assert_eq!(payload["body"][0]["Shape"]["width"], 29.25);
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
    let original_json = serde_json::to_value(&original.module).expect("serialize source module");
    let updated_json = serde_json::to_value(&updated.module).expect("serialize updated module");
    let cached_patch =
        nexa_dev_ir::diff_with_module_values(&original, &original_json, &updated, &updated_json)
            .expect("cached module values should be patchable");

    assert_eq!(cached_patch, patch);
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
fn translation_edits_are_sent_as_patch_data_without_module_changes() {
    let module = module(vec![text("Save")], Type::Numeric(NumericType::Int32));
    let before = nexa_dev_ir::lower_with_translations(
        &module,
        "before",
        BTreeMap::from([(
            "fr".to_owned(),
            BTreeMap::from([("Save".to_owned(), serde_json::json!("Enregistrer"))]),
        )]),
    );
    let after = nexa_dev_ir::lower_with_translations(
        &module,
        "after",
        BTreeMap::from([(
            "fr".to_owned(),
            BTreeMap::from([("Save".to_owned(), serde_json::json!("Sauvegarder"))]),
        )]),
    );

    let patch = nexa_dev_ir::diff(&before, &after).expect("translation edit should be patchable");
    assert!(patch.operations.is_empty());
    assert_eq!(patch.translations, Some(after.translations));
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

#[test]
fn route_host_inset_serializes_as_a_dev_runtime_endpoint_reservation() {
    let route_inset = Node::SafeAreaInset {
        plan: SafeAreaInsetPlan {
            edge: LayoutEdge::Bottom,
            scroll_owner: ScrollOwnerResolution::RouteHost,
        },
        content: vec![text("Tabs")],
        children: vec![text("Navigation host")],
    };
    let dev_module = lower(
        &module(vec![route_inset], Type::Numeric(NumericType::Int32)),
        "route-host",
    );
    let encoded = serde_json::to_value(&dev_module).expect("serialize route host");
    assert_eq!(
        encoded.pointer("/module/body/0/SafeAreaInset/plan/scroll_owner"),
        Some(&serde_json::json!("RouteHost")),
        "{encoded:#?}"
    );
}

#[test]
fn inserting_same_kind_sibling_produces_ordered_distinct_identities() {
    let m1 = module(vec![text("Original")], Type::Numeric(NumericType::Int32));
    let dev1 = lower(&m1, "rev-1");
    let text_ids_1: Vec<_> = dev1
        .identities
        .iter()
        .filter(|id| id.id.contains("Text:"))
        .map(|id| id.id.clone())
        .collect();
    assert_eq!(text_ids_1, vec!["app/body/Text:0"]);

    // Insert a new Text before the original
    let m2 = module(
        vec![text("Inserted Before"), text("Original")],
        Type::Numeric(NumericType::Int32),
    );
    let dev2 = lower(&m2, "rev-2");
    let text_ids_2: Vec<_> = dev2
        .identities
        .iter()
        .filter(|id| id.id.contains("Text:"))
        .map(|id| id.id.clone())
        .collect();
    assert_eq!(text_ids_2, vec!["app/body/Text:0", "app/body/Text:1"]);
}

#[test]
fn screen_and_component_states_have_distinct_namespaced_identities() {
    let mut m = module(vec![text("Root")], Type::Numeric(NumericType::Int32));
    m.screens.push(nexa_ir::Screen {
        id: nexa_ir::ScreenId(1),
        name: "ProfileScreen".to_owned(),
        parameters: vec![],
        states: vec![State {
            name: "profileScore".to_owned(),
            ty: Type::Numeric(NumericType::Int32),
            initial: Expr::Number {
                raw: "100".to_owned(),
                ty: NumericType::Int32,
            },
            mutable: true,
        }],
        body: vec![text("Profile")],
        status_bar: None,
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
    });
    m.components.push(Component {
        name: "CustomBadge".to_owned(),
        source_file: None,
        parameters: vec![],
        states: vec![State {
            name: "badgeColor".to_owned(),
            ty: Type::String,
            initial: Expr::String("blue".to_owned()),
            mutable: false,
        }],
        body: vec![text("Badge")],
        ref_methods: vec![],
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
    });

    let dev = lower(&m, "namespaced-rev");
    let state_ids: Vec<_> = dev
        .identities
        .iter()
        .filter(|id| {
            matches!(
                id.kind,
                IdentityKind::AppState | IdentityKind::ScreenState | IdentityKind::ComponentState
            )
        })
        .map(|id| (id.kind, id.id.clone()))
        .collect();

    assert!(state_ids.contains(&(IdentityKind::AppState, "app/state/count".to_string())));
    assert!(state_ids.contains(&(
        IdentityKind::ScreenState,
        "screen/ProfileScreen/state/profileScore".to_string()
    )));
    assert!(state_ids.contains(&(
        IdentityKind::ComponentState,
        "component/CustomBadge/state/badgeColor".to_string()
    )));
}
