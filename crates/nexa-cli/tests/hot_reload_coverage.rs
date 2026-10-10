use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_ir::{
    AccessibilityRole, Expr, LayoutKind, ListAxis, ListCommon, ListPlan, Module, Node, NumericType,
    Screen, ScreenId, ViewStyle,
};
use nexa_syntax::catalog;
use serde_json::Value;

fn variants(source: &str, enum_name: &str) -> BTreeSet<String> {
    let marker = format!("pub enum {enum_name} {{");
    let body = source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("IR enum {enum_name} was not found"))
        .1
        .split_once("\n}")
        .unwrap_or_else(|| panic!("IR enum {enum_name} has no closing brace"))
        .0;
    body.lines()
        .filter_map(|line| {
            let line = line.strip_prefix("    ")?;
            if line.starts_with(' ') {
                return None;
            }
            let name = line
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

fn public_enums(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("pub enum "))
        .filter_map(|declaration| declaration.split_once(' ').map(|(name, _)| name))
        .map(str::to_owned)
        .collect()
}

/// Returns documented cases that remain outside DevRuntime's compiled host.
fn dev_boundary(enum_name: &str, variant_name: &str) -> Option<&'static str> {
    match (enum_name, variant_name) {
        // Plugin class values can refer only to native types compiled into
        // the host; hot reload cannot load a new native plugin implementation.
        ("Type", "Plugin") => Some("partial"),
        // Interpreted user classes support constructors, methods and fields,
        // but IR does not yet carry stored-property initializers for classes
        // with no instance methods.
        ("Type", "Class") | ("MemberKind", "ClassField") => Some("partial"),
        _ => None,
    }
}

#[test]
fn user_classes_are_interpreted_by_both_dev_runtimes() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS dev state evaluator");
    let swift_classes = fs::read_to_string(root.join("../../runtime/ios/NexaDevClasses.swift"))
        .expect("read iOS class evaluator");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state evaluator");
    for marker in [
        "is_constructor",
        "__NexaUserClass:",
        "ClassStaticField",
        "moduleValues",
    ] {
        assert!(
            swift.contains(marker) || swift_classes.contains(marker),
            "iOS class runtime missing {marker}"
        );
        assert!(
            kotlin.contains(marker),
            "Android class runtime missing {marker}"
        );
    }
    assert!(swift_classes.contains("functionScope.merge(instance.fields)"));
    assert!(kotlin.contains("callLocals = instance.fields.toMutableMap()"));
}

#[test]
fn android_dev_runtime_unboxes_primitive_state_and_closes_its_event_scope() {
    let (root, _) = fixture();
    let kotlin_state = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state store");
    let kotlin_runtime = fs::read_to_string(root.join("../../runtime/android/NexaDevRuntime.kt"))
        .expect("read Android dev runtime");

    for declaration in [
        "appLifecycleEpoch by mutableIntStateOf(0)",
        "navigationEpoch by mutableIntStateOf(0)",
        "moduleRevision by mutableIntStateOf(0)",
    ] {
        assert!(
            kotlin_state.contains(declaration),
            "missing unboxed state: {declaration}"
        );
    }
    assert!(kotlin_runtime.contains("frameTimeMs by remember { mutableDoubleStateOf(0.0) }"));
    assert!(kotlin_state.contains("fun dispose()"));
    assert!(kotlin_state.contains("eventScope.cancel()"));
    assert!(kotlin_runtime.contains("store.dispose()"));
}

#[test]
fn android_dev_node_dispatch_is_split_into_art_jittable_composable_families() {
    let (root, _) = fixture();
    let renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime renderer");
    let dispatch = renderer
        .split_once("internal fun NexaDevNode(")
        .expect("find Android DevRuntime node dispatcher")
        .1
        .split_once("private fun nexaDevContainsForm(")
        .expect("find Android DevRuntime node dispatcher end")
        .0;

    for family in [
        "NexaDevRenderTextNode",
        "NexaDevRenderControlNode",
        "NexaDevRenderListNode",
        "NexaDevRenderNavigationNode",
    ] {
        assert!(renderer.contains(&format!("private fun {family}(")));
        assert!(dispatch.contains(family), "dispatcher omitted {family}");
    }
    for extracted in [
        "NexaDevRenderPressableNode",
        "NexaDevRenderTextInputNode",
        "NexaDevRenderFastListNode",
        "NexaDevRenderAppBottomBarNode",
    ] {
        assert!(renderer.contains(&format!("private fun {extracted}(")));
        assert!(renderer.contains(&format!(
            "{extracted}(fields, module, store, locals, scope, modifier)"
        )));
    }
}

fn is_semantically_probed_plugin_variant(enum_name: &str, variant_name: &str) -> bool {
    matches!(
        (enum_name, variant_name),
        ("Node", "NativeComponentCall")
            | ("Action", "NativePropertyAssign" | "NativeEventSubscribe")
    )
}

fn fixture() -> (PathBuf, Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = root.join("tests/fixtures/hot_reload_coverage.json");
    let fixture: Value = serde_json::from_str(
        &fs::read_to_string(&fixture_path).expect("read hot-reload coverage inventory"),
    )
    .expect("parse hot-reload coverage inventory");
    (root, fixture)
}

fn collect_parity_components(
    nodes: &[nexa_syntax::ast::Node],
    found: &mut BTreeSet<String>,
    arguments: &mut BTreeMap<String, BTreeSet<String>>,
) {
    use nexa_syntax::ast::{ChildBody, ModifierBody, Node};

    for node in nodes {
        match node {
            Node::Platform { children, .. } => {
                collect_parity_components(children, found, arguments)
            }
            Node::ComponentInvocation(invocation) => {
                found.insert(invocation.name.clone());
                arguments
                    .entry(invocation.name.clone())
                    .or_default()
                    .extend(invocation.arguments.keys().cloned());
                match &invocation.children {
                    ChildBody::Nodes(children) => {
                        collect_parity_components(children, found, arguments)
                    }
                    ChildBody::Tabs(tabs) => {
                        found.insert("Tab".to_owned());
                        for tab in tabs {
                            collect_parity_components(&tab.children, found, arguments);
                        }
                    }
                    ChildBody::SplitPanes { sidebar, detail } => {
                        collect_parity_components(sidebar, found, arguments);
                        collect_parity_components(detail, found, arguments);
                    }
                    ChildBody::Rows(rows) => {
                        collect_parity_components(&rows.children, found, arguments)
                    }
                    ChildBody::None | ChildBody::Actions(_) => {}
                }
                for modifier in &invocation.modifiers {
                    if let ModifierBody::Nodes(children) = &modifier.body {
                        collect_parity_components(children, found, arguments);
                    }
                }
            }
            Node::If {
                then_body,
                else_body,
                ..
            } => {
                collect_parity_components(then_body, found, arguments);
                if let Some(else_body) = else_body {
                    collect_parity_components(else_body, found, arguments);
                }
            }
            Node::When {
                cases, else_body, ..
            } => {
                for case in cases {
                    collect_parity_components(&case.body, found, arguments);
                }
                collect_parity_components(else_body, found, arguments);
            }
            Node::ComponentCall { children, .. } | Node::NativeComponentCall { children, .. } => {
                if let Some(children) = children {
                    collect_parity_components(children, found, arguments);
                }
            }
        }
    }
}

#[test]
fn render_parity_fixture_exercises_every_catalog_component_and_modifier() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("../../tests/fixtures/render_parity.nx"))
        .expect("read render parity fixture");
    let app = nexa_syntax::parse(&source).expect("parse render parity fixture");
    let mut found = BTreeSet::new();
    let mut arguments = BTreeMap::new();
    collect_parity_components(&app.body, &mut found, &mut arguments);
    for screen in &app.screens {
        collect_parity_components(&screen.body, &mut found, &mut arguments);
    }
    for component in &app.components {
        collect_parity_components(&component.body, &mut found, &mut arguments);
    }
    for widget in &app.widgets {
        collect_parity_components(&widget.body, &mut found, &mut arguments);
    }
    for component in catalog::COMPONENTS {
        assert!(
            found.contains(component.name),
            "render parity fixture is missing catalog component {}",
            component.name
        );
    }
    let mut missing_arguments = Vec::new();
    for schema in catalog::COMPONENT_SCHEMAS {
        let covered = arguments.get(schema.name);
        for argument in schema.arguments {
            if !covered.is_some_and(|covered| covered.contains(argument.name)) {
                missing_arguments.push(format!("{}({})", schema.name, argument.name));
            }
        }
    }
    assert!(
        missing_arguments.is_empty(),
        "render parity fixture is missing component parameters: {}",
        missing_arguments.join(", ")
    );
    let accessibility_arguments = catalog::ACCESSIBILITY_ARGUMENTS
        .iter()
        .filter(|argument| {
            !arguments
                .values()
                .any(|covered| covered.contains(**argument))
        })
        .copied()
        .collect::<Vec<_>>();
    assert!(
        accessibility_arguments.is_empty(),
        "render parity fixture is missing shared accessibility parameters: {}",
        accessibility_arguments.join(", ")
    );
    for modifier in catalog::DOT_MODIFIERS {
        assert!(
            source.contains(&format!(".{}(", modifier.name))
                || source.contains(&format!(".{} {{", modifier.name)),
            "render parity fixture is missing catalog behavior modifier .{}",
            modifier.name
        );
    }
}

#[test]
fn android_aot_and_dev_runtime_share_refresh_control_layout() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let refresh_generator = fs::read_to_string(
        root.join("../../crates/nexa-backend-kotlin/src/generator/components/refresh.rs"),
    )
    .expect("read Android refresh-control generator");
    let primitives =
        fs::read_to_string(root.join("../../crates/nexa-backend-kotlin/src/generator/mod.rs"))
            .expect("read Android shared component primitives");
    let dev_renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime renderer");

    assert!(refresh_generator.contains("NexaRefreshControlPrimitive("));
    assert!(primitives.contains("internal fun NexaRefreshControlPrimitive("));
    assert!(dev_renderer.contains("NexaRefreshControlPrimitive("));
    assert!(dev_renderer.contains("scrollContent = !nexaDevContainsScrollable(children)"));

    let column_children = dev_renderer
        .split_once("internal fun ColumnScope.RenderColumnChildren(")
        .expect("find Android DevRuntime column child renderer")
        .1
        .split_once("@Composable\n@OptIn")
        .expect("find end of Android DevRuntime column child renderer")
        .0;
    assert!(column_children.contains("if (child.has(\"Spacer\")) Modifier.weight(1f)"));
    assert!(!column_children.contains("child.has(\"RefreshControl\")"));
    assert!(!column_children.contains("child.has(\"FastList\")"));
}

fn empty_module(body: Vec<Node>) -> Module {
    Module {
        app_name: "RenderParity".to_owned(),
        plugins: Vec::new(),
        plugin_assets: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        background_tasks: Vec::new(),
        states: Vec::new(),
        globals: Vec::new(),
        screens: Vec::new(),
        widgets: Vec::new(),
        components: Vec::new(),
        body,
        status_bar: None,
        direction: None,
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
        on_active: None,
        on_inactive: None,
        on_background: None,
    }
}

fn keyed_fast_list_module(native: bool) -> Module {
    let item_type = nexa_ir::Type::String;
    empty_module(vec![Node::FastList {
        plan: ListPlan::Items {
            collection: Expr::Array(vec![Expr::String("task-a".to_owned())]),
            element_type: item_type.clone(),
            item: "task".to_owned(),
            common: ListCommon {
                axis: ListAxis::Vertical,
                native,
                reverse_layout: false,
                page_snap: false,
                item_extent: None,
                index: "index".to_owned(),
                key: Some(Expr::State("task".to_owned(), item_type.clone())),
                scroll_position: None,
                children: vec![Node::Text {
                    value: Expr::State("task".to_owned(), item_type),
                    style: nexa_ir::TextStyle::default(),
                }],
                on_end_reached: None,
                on_scroll: None,
                on_move: None,
                swipe_actions: None,
                sticky_header: None,
                refresh: None,
            },
        },
    }])
}

#[test]
fn every_view_node_has_aot_and_dev_dispatch_on_both_platforms() {
    let (root, inventory) = fixture();
    let ir_source =
        fs::read_to_string(root.join("../nexa-ir/src/lib.rs")).expect("read IR declarations");
    let swift_dispatch = fs::read_to_string(
        root.join("../nexa-backend-swift/src/generator/components/components.rs"),
    )
    .expect("read Swift AOT dispatcher");
    let kotlin_dispatch = fs::read_to_string(
        root.join("../nexa-backend-kotlin/src/generator/components/components.rs"),
    )
    .expect("read Kotlin AOT dispatcher");
    let swift_runtime = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS DevRuntime renderer");
    let kotlin_runtime = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime renderer");
    let expected = variants(&ir_source, "Node");
    let actual = inventory["ir_variants"]["Node"]
        .as_array()
        .expect("Node coverage inventory")
        .iter()
        .map(|entry| entry["name"].as_str().expect("node variant").to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "Node coverage inventory is stale");

    // These IR entries are module metadata handled by the DevRuntime roots,
    // rather than components passed through the node renderer.
    let root_metadata = [
        "StatusBar",
        "Direction",
        "OnAppear",
        "OnDisappear",
        "OnActive",
        "OnInactive",
        "OnBackground",
    ];
    for variant in expected {
        if root_metadata.contains(&variant.as_str()) {
            continue;
        }
        assert!(
            swift_dispatch.contains(&format!("Node::{variant}")),
            "Swift AOT has no Node::{variant} mapping"
        );
        assert!(
            kotlin_dispatch.contains(&format!("Node::{variant}")),
            "Kotlin AOT has no Node::{variant} mapping"
        );
        assert!(
            swift_runtime.contains(&format!("\"{variant}\"")),
            "iOS DevRuntime has no {variant} mapping"
        );
        assert!(
            kotlin_runtime.contains(&format!("\"{variant}\"")),
            "Android DevRuntime has no {variant} mapping"
        );
    }
}

#[test]
fn android_dev_runtime_treats_a_null_transition_as_no_animation() {
    let (root, _) = fixture();
    let renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime renderer");

    assert!(renderer.contains("private fun JSONObject.nexaDevTransition(): String"));
    assert!(renderer.contains("null, JSONObject.NULL -> \"\""));
    assert!(renderer.contains("conditional.nexaDevTransition().isEmpty()"));
    assert!(renderer.contains("val transition = fields.nexaDevTransition()"));
    assert!(
        !renderer.contains("optString(\"transition\")"),
        "nullable transition fields must never use optString, which turns JSON null into \"null\""
    );
}

#[test]
fn android_dev_runtime_treats_null_optional_strings_as_absent() {
    let (root, _) = fixture();
    let renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime renderer");

    assert!(
        renderer.contains("private fun JSONObject.nexaDevOptionalString(name: String): String?")
    );
    assert!(renderer.contains("null, JSONObject.NULL -> null"));
    for field in [
        "NexaDevKeys.PINCH_PARAMETER",
        "\"navigation_title\"",
        "\"search_prompt\"",
        "\"scroll_position\"",
    ] {
        assert!(
            renderer.contains(&format!("nexaDevOptionalString({field})")),
            "nullable field {field} must use the null-safe string reader"
        );
    }
    assert!(
        !renderer.contains("optString(NexaDevKeys.PINCH_PARAMETER)"),
        "JSON null must not enable pinch handling as the literal parameter name \"null\""
    );
}

#[test]
fn aot_and_dev_renderers_call_the_same_native_component_primitives() {
    let module = empty_module(vec![
        Node::Image {
            source: nexa_ir::ImageSource::Asset("task_avatar".to_owned()),
            description: "Task owner".to_owned(),
            scale: nexa_ir::ImageScale::Fit,
            placeholder: None,
            max_height: Some(96.0),
            shared_element: None,
        },
        Node::Image {
            source: nexa_ir::ImageSource::RemoteUrl(Expr::String(
                "https://example.test/task.png".to_owned(),
            )),
            description: "Task attachment".to_owned(),
            scale: nexa_ir::ImageScale::Fill,
            placeholder: Some("task_placeholder".to_owned()),
            max_height: Some(144.0),
            shared_element: None,
        },
        Node::Image {
            source: nexa_ir::ImageSource::LocalFile(Expr::String(
                "file:///tmp/task.png".to_owned(),
            )),
            description: "Local task photo".to_owned(),
            scale: nexa_ir::ImageScale::Fit,
            placeholder: None,
            max_height: None,
            shared_element: None,
        },
        Node::PagePager {
            state: "page".to_owned(),
            pages: vec![
                vec![Node::Text {
                    value: Expr::String("First page".to_owned()),
                    style: nexa_ir::TextStyle::default(),
                }],
                vec![Node::Text {
                    value: Expr::String("Second page".to_owned()),
                    style: nexa_ir::TextStyle::default(),
                }],
            ],
        },
        Node::NavigationSplitView {
            detail_visible: "detailVisible".to_owned(),
            sidebar: vec![Node::Text {
                value: Expr::String("Task categories".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
            detail: vec![Node::Text {
                value: Expr::String("Selected task".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
        },
        Node::AppBottomBar {
            state: "selectedTab".to_owned(),
            tint: None,
            tabs: vec![nexa_ir::BottomBarTab {
                index: 0,
                label: "Today".to_owned(),
                comment: None,
                icon: None,
                badge: None,
                role: None,
                navigation_title: Some("Settings".to_owned()),
                large_title: true,
                search_state: None,
                search_prompt: None,
                children: Vec::new(),
            }],
        },
        Node::Layout {
            kind: LayoutKind::Column,
            spacing: 8.0,
            style: ViewStyle::default(),
            children: vec![Node::Text {
                value: Expr::String("Shared layout".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
        },
        Node::Button {
            label: Expr::String(String::new()),
            icon: Some(nexa_ir::SystemIcon::Shared("add".to_owned())),
            loading: None,
            disabled: None,
            style: Some(nexa_ir::ButtonStyle::BorderedProminent),
            size: None,
            shape: Some(nexa_ir::ButtonShape::Circle),
            tint: None,
            glass: true,
            actions: Vec::new(),
        },
        Node::Form {
            children: vec![
                Node::FormSection {
                    title: Some(Expr::String("General".to_owned())),
                    footer: Some(Expr::String("Task preferences".to_owned())),
                    children: vec![
                        Node::Text {
                            value: Expr::String("Shared form row".to_owned()),
                            style: nexa_ir::TextStyle::default(),
                        },
                        Node::Text {
                            value: Expr::String("Second shared form row".to_owned()),
                            style: nexa_ir::TextStyle::default(),
                        },
                    ],
                },
                Node::Text {
                    value: Expr::String("Unsectioned shared form row".to_owned()),
                    style: nexa_ir::TextStyle::default(),
                },
            ],
        },
        Node::BottomSheet {
            state: "showEditor".to_owned(),
            partial: true,
            large_only: false,
            title: Some(Expr::String("Edit task".to_owned())),
            children: vec![Node::Text {
                value: Expr::String("Task details".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
        },
        Node::Dialog {
            state: "showAlert".to_owned(),
            title: Expr::String("Saved".to_owned()),
            message: Expr::String("Task updated".to_owned()),
            children: vec![Node::Button {
                label: Expr::String("OK".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                style: None,
                size: None,
                shape: None,
                tint: None,
                glass: false,
                actions: Vec::new(),
            }],
        },
        Node::ConfirmationDialog {
            state: "showPriority".to_owned(),
            title: Expr::String("Choose priority".to_owned()),
            children: Vec::new(),
        },
        Node::Switch {
            state: "enabled".to_owned(),
            label: Expr::String("Shared switch".to_owned()),
        },
        Node::TextInput {
            state: "query".to_owned(),
            placeholder: "Search tasks".to_owned(),
            comment: None,
            keyboard: nexa_ir::KeyboardType::Text,
            secure: false,
            multiline: false,
            autofill: None,
            return_key: None,
            autocorrect: None,
            capitalization: None,
            focused: None,
            max_length: None,
            font: None,
            min_lines: None,
            max_lines: None,
            searchable: true,
            actions: Vec::new(),
            on_change: None,
        },
        Node::Slider {
            state: "amount".to_owned(),
            animated: false,
            min: 0.0,
            max: 1.0,
            step: 0.1,
        },
        Node::ProgressBar {
            progress: Expr::Number {
                raw: "0.5".to_owned(),
                ty: nexa_ir::NumericType::Float64,
            },
        },
        Node::ProgressRing {
            progress: Expr::Number {
                raw: "0.5".to_owned(),
                ty: nexa_ir::NumericType::Float64,
            },
        },
        Node::Divider {
            color: nexa_ir::ColorValue::Static(nexa_ir::Color {
                red: 128,
                green: 128,
                blue: 128,
                alpha: 255,
            }),
            thickness: 1.0,
        },
        Node::SegmentedControl {
            items: Expr::Array(vec![
                Expr::String("Today".to_owned()),
                Expr::String("Upcoming".to_owned()),
            ]),
            state: "selectedTab".to_owned(),
        },
        Node::Picker {
            items: Expr::Array(vec![Expr::String("High".to_owned())]),
            state: "priority".to_owned(),
            icon: Some(nexa_ir::SystemIcon::shared("flag").expect("catalog icon")),
            label: Some(Expr::String("Priority".to_owned())),
            tint: None,
        },
        Node::DatePicker {
            timestamp_state: "dueAt".to_owned(),
            has_time_state: "includeTime".to_owned(),
        },
        Node::ContentUnavailable {
            title: Expr::String("No tasks".to_owned()),
            icon: nexa_ir::SystemIcon::shared("inbox").expect("catalog icon"),
            description: Expr::String("Add a task to get started.".to_owned()),
        },
        Node::SystemIcon {
            icon: nexa_ir::SystemIcon::shared("star").expect("catalog icon"),
            description: "Priority".to_owned(),
            size: 20.0,
            tint: nexa_ir::ColorExpression::Static(nexa_ir::ColorValue::Static(nexa_ir::Color {
                red: 255,
                green: 0,
                blue: 0,
                alpha: 255,
            })),
        },
        Node::LinearGradient {
            start_color: nexa_ir::ColorValue::Static(nexa_ir::Color {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            }),
            end_color: nexa_ir::ColorValue::Static(nexa_ir::Color {
                red: 255,
                green: 255,
                blue: 255,
                alpha: 255,
            }),
            direction: nexa_ir::GradientDirection::LeadingToTrailing,
            height: 100.0,
        },
        Node::Accessibility {
            label: Expr::String("Task priority".to_owned()),
            hint: Some(Expr::String("Double tap to change".to_owned())),
            value: Some(Expr::String("High".to_owned())),
            role: AccessibilityRole::Button,
            children: vec![Node::Text {
                value: Expr::String("High priority".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
        },
        Node::Link {
            url: Expr::String("https://example.test/task".to_owned()),
            children: vec![Node::Text {
                value: Expr::String("Open task details".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
        },
    ]);
    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    let (root, _) = fixture();
    let swift_runtime = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS DevRuntime renderer");
    let kotlin_runtime = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime renderer");

    assert!(swift.contains("struct NexaColumnPrimitive<Content: View>"));
    assert!(swift.contains("NexaColumnPrimitive(alignment: .center, spacing: 8)"));
    assert!(swift.contains("struct NexaTextPrimitive: View"));
    assert!(swift.contains("struct NexaButtonPrimitive<Label: View>"));
    assert!(swift.contains("NexaTextPrimitive("));
    assert!(swift.contains("Image(systemName: \"plus\").font(.system(size: 17))"));
    assert!(swift.contains("NexaFormPrimitive {"));
    assert!(swift_runtime.contains("NexaFormPrimitive {"));
    assert!(swift.contains("NexaAppBottomBarPrimitive("));
    assert!(swift_runtime.contains("NexaAppBottomBarPrimitive("));
    assert!(swift_runtime.contains("nexaDevUsesIconOnlyButtonLabel"));
    assert!(swift_runtime.contains("Image(systemName: symbol)"));
    for primitive in ["NexaAssetImagePrimitive(", "NexaRemoteImagePrimitive("] {
        assert!(swift.contains(primitive), "iOS AOT omitted {primitive}");
        assert!(
            swift_runtime.contains(primitive),
            "iOS DevRuntime omitted {primitive}"
        );
    }
    assert!(swift.contains("NexaPageIndicatorRow("));
    assert!(swift_runtime.contains("NexaPageIndicatorRow("));
    for output in [&swift, &swift_runtime] {
        assert!(
            output.contains("NexaNavigationSplitViewPrimitive("),
            "iOS AOT and DevRuntime must share the split-view primitive"
        );
    }
    for primitive in [
        "NexaButtonPrimitive(",
        "NexaFormSectionPrimitive(",
        "NexaFormRowPrimitive(",
        "NexaSwitchPrimitive(label:",
        "NexaTextInputPrimitive(",
        "NexaSliderPrimitive(value:",
        "NexaProgressBarPrimitive(progress:",
        "NexaProgressRingPrimitive(progress:",
        "NexaDividerPrimitive(color:",
        "NexaSegmentedControlPrimitive(items:",
        "NexaPickerPrimitive(selection:",
        "NexaDatePickerPrimitive(timestamp:",
        "NexaContentUnavailablePrimitive(title:",
        "NexaSystemIconPrimitive(symbol:",
        "NexaLinearGradientPrimitive(colors:",
        "NexaAccessibilityPrimitive(",
        "NexaLinkPrimitive(destination:",
    ] {
        assert!(swift.contains(primitive), "iOS AOT omitted {primitive}");
        assert!(
            swift_runtime.contains(primitive),
            "iOS DevRuntime omitted {primitive}"
        );
    }
    assert!(
        kotlin.contains("}) {"),
        "Android AOT link action must close before the trailing content lambda"
    );
    assert!(swift_runtime.contains("NexaColumnPrimitive(alignment: columnAlignment"));
    assert!(swift_runtime.contains("NexaTextPrimitive("));
    assert!(swift_runtime.contains("NexaTextInputPrimitive("));
    for primitive in [
        "NexaBottomSheetPrimitive(isPresented: $nexa_showEditor, partial: true, largeOnly: false)",
        "NexaAlertDialogPrimitive(isPresented: $nexa_showAlert, title: Text(\"Saved\"), message: Text(\"Task updated\"))",
        "NexaConfirmationDialogPrimitive(isPresented: $nexa_showPriority, title: Text(\"Choose priority\"))",
    ] {
        assert!(swift.contains(primitive), "iOS AOT omitted {primitive}");
        let primitive_name = primitive.split('(').next().expect("primitive name");
        assert!(
            swift_runtime.contains(primitive_name),
            "iOS DevRuntime omitted {primitive}"
        );
        assert!(
            kotlin.contains(primitive_name),
            "Android AOT omitted {primitive}"
        );
        assert!(
            kotlin_runtime.contains(primitive_name),
            "Android DevRuntime omitted {primitive}"
        );
    }
    assert!(kotlin.contains("internal fun NexaColumnPrimitive("));
    assert!(kotlin.contains("NexaColumnPrimitive("));
    assert!(kotlin.contains("internal fun NexaTextPrimitive("));
    assert!(kotlin.contains("internal fun NexaButtonPrimitive("));
    assert!(kotlin.contains("internal fun NexaLargeTitlePrimitive("));
    assert!(kotlin.contains("NexaLargeTitlePrimitive("));
    assert!(kotlin.contains("NexaAppBottomBarPrimitive("));
    assert!(kotlin.contains("internal fun NexaAppBottomBarPrimitive("));
    assert!(kotlin_runtime.contains("NexaLargeTitlePrimitive("));
    assert!(kotlin_runtime.contains("NexaAppBottomBarPrimitive("));
    assert!(kotlin.contains("NexaTextPrimitive(text ="));
    for primitive in ["NexaAssetImagePrimitive(", "NexaRemoteImagePrimitive("] {
        assert!(
            kotlin.contains(primitive),
            "Android AOT omitted {primitive}"
        );
        assert!(
            kotlin_runtime.contains(primitive),
            "Android DevRuntime omitted {primitive}"
        );
    }
    assert!(kotlin.contains("NexaPageIndicatorRow("));
    assert!(kotlin_runtime.contains("NexaPageIndicatorRow("));
    for output in [&kotlin, &kotlin_runtime] {
        assert!(
            output.contains("NexaNavigationSplitViewPrimitive("),
            "Android AOT and DevRuntime must share the split-view primitive"
        );
    }
    for primitive in [
        "NexaFormPrimitive",
        "NexaFormSectionPrimitive(",
        "NexaButtonPrimitive(",
        "NexaFormRowPrimitive(",
        "NexaSwitchPrimitive(",
        "NexaTextInputPrimitive(",
        "NexaSliderPrimitive(value =",
        "NexaProgressBarPrimitive(progress =",
        "NexaProgressRingPrimitive(progress =",
        "NexaDividerPrimitive(color =",
        "NexaSegmentedControlPrimitive(items =",
        "NexaPickerPrimitive(items =",
        "NexaDatePickerPrimitive(timestamp =",
        "NexaContentUnavailablePrimitive(title =",
        "NexaSystemIconPrimitive(image =",
        "NexaLinearGradientPrimitive(startColor =",
        "NexaAccessibilityPrimitive(",
        "NexaLinkPrimitive(onClick =",
    ] {
        assert!(
            kotlin.contains(primitive),
            "Android AOT omitted {primitive}"
        );
    }
    for primitive in [
        "NexaFormPrimitive",
        "NexaFormSectionPrimitive(",
        "NexaFormRowPrimitive(",
        "NexaSwitchPrimitive(",
        "NexaSliderPrimitive(",
        "NexaProgressBarPrimitive(",
        "NexaProgressRingPrimitive(",
        "NexaDividerPrimitive(",
        "NexaSegmentedControlPrimitive(",
        "NexaPickerPrimitive(",
        "NexaDatePickerPrimitive(",
        "NexaContentUnavailablePrimitive(",
        "NexaSystemIconPrimitive(",
        "NexaLinearGradientPrimitive(",
    ] {
        assert!(
            kotlin_runtime.contains(primitive),
            "Android DevRuntime omitted {primitive}"
        );
    }
    assert!(kotlin_runtime.contains("NexaColumnPrimitive("));
    assert!(kotlin_runtime.contains("NexaTextPrimitive("));
    assert!(kotlin_runtime.contains("NexaTextInputPrimitive("));
    assert!(kotlin.contains("NexaFormDividerPrimitive()"));
    assert!(kotlin_runtime.contains("NexaFormDividerPrimitive()"));
}

#[test]
fn release_and_dev_hosts_preload_the_same_shared_component_catalog() {
    let module = empty_module(Vec::new());
    let swift_release = SwiftBackend.generate(&module);
    let swift_dev = SwiftBackend.generate_for_dev(&module);
    let (kotlin_release, _) = KotlinBackend.generate_with_project_features(&module);
    let (kotlin_dev, _) = KotlinBackend.generate_for_dev_with_project_features(&module);
    let (root, _) = fixture();
    let swift_runtime = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS DevRuntime renderer");
    let kotlin_runtime = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime renderer");

    // This is the native primitive catalog used by AOT output and dynamic
    // DevRuntime dispatch. Definitions must be present in empty-app hosts so
    // hot reload can introduce any catalogued component without rebuilding
    // the native development host.
    let catalog = [
        (
            "struct NexaColumnPrimitive<Content: View>",
            "internal fun NexaColumnPrimitive(",
            "NexaColumnPrimitive(",
        ),
        (
            "struct NexaRowPrimitive<Content: View>",
            "internal fun NexaRowPrimitive(",
            "NexaRowPrimitive(",
        ),
        (
            "struct NexaStackPrimitive<Content: View>",
            "internal fun NexaStackPrimitive(",
            "NexaStackPrimitive(",
        ),
        (
            "struct NexaFormPrimitive<Content: View>",
            "internal fun NexaFormPrimitive(",
            "NexaFormPrimitive",
        ),
        (
            "struct NexaFormSectionPrimitive<Content: View, Header: View, Footer: View>",
            "internal fun NexaFormSectionPrimitive(",
            "NexaFormSectionPrimitive(",
        ),
        (
            "struct NexaFormRowPrimitive<Content: View>",
            "internal fun NexaFormRowPrimitive(",
            "NexaFormRowPrimitive(",
        ),
        (
            "struct NexaAccessibilityPrimitive<Content: View>",
            "internal fun NexaAccessibilityPrimitive(",
            "NexaAccessibilityPrimitive(",
        ),
        (
            "struct NexaLinkPrimitive<Content: View>",
            "internal fun NexaLinkPrimitive(",
            "NexaLinkPrimitive(",
        ),
        (
            "struct NexaTextPrimitive: View",
            "internal fun NexaTextPrimitive(",
            "NexaTextPrimitive(",
        ),
        (
            "struct NexaButtonPrimitive<Label: View>",
            "internal fun NexaButtonPrimitive(",
            "NexaButtonPrimitive(",
        ),
        (
            "struct NexaTextInputPrimitive: View",
            "internal fun NexaTextInputPrimitive(",
            "NexaTextInputPrimitive(",
        ),
        (
            "struct NexaSwitchPrimitive: View",
            "internal fun NexaSwitchPrimitive(",
            "NexaSwitchPrimitive(",
        ),
        (
            "struct NexaSliderPrimitive: View",
            "internal fun NexaSliderPrimitive(",
            "NexaSliderPrimitive(",
        ),
        (
            "struct NexaProgressBarPrimitive: View",
            "internal fun NexaProgressBarPrimitive(",
            "NexaProgressBarPrimitive(",
        ),
        (
            "struct NexaProgressRingPrimitive: View",
            "internal fun NexaProgressRingPrimitive(",
            "NexaProgressRingPrimitive(",
        ),
        (
            "struct NexaDividerPrimitive: View",
            "internal fun NexaDividerPrimitive(",
            "NexaDividerPrimitive(",
        ),
        (
            "struct NexaSegmentedControlPrimitive: View",
            "internal fun NexaSegmentedControlPrimitive(",
            "NexaSegmentedControlPrimitive(",
        ),
        (
            "struct NexaContentUnavailablePrimitive: View",
            "internal fun NexaContentUnavailablePrimitive(",
            "NexaContentUnavailablePrimitive(",
        ),
        (
            "struct NexaSystemIconPrimitive: View",
            "internal fun NexaSystemIconPrimitive(",
            "NexaSystemIconPrimitive(",
        ),
        (
            "struct NexaLinearGradientPrimitive: View",
            "internal fun NexaLinearGradientPrimitive(",
            "NexaLinearGradientPrimitive(",
        ),
        (
            "struct NexaPickerPrimitive<LabelContent: View>",
            "internal fun NexaPickerPrimitive(",
            "NexaPickerPrimitive(",
        ),
        (
            "struct NexaDatePickerPrimitive: View",
            "internal fun NexaDatePickerPrimitive(",
            "NexaDatePickerPrimitive(",
        ),
        (
            "struct NexaBottomSheetPrimitive<Content: View>",
            "internal fun NexaBottomSheetPrimitive(",
            "NexaBottomSheetPrimitive(",
        ),
        (
            "struct NexaAlertDialogPrimitive<Actions: View>",
            "internal fun NexaAlertDialogPrimitive(",
            "NexaAlertDialogPrimitive(",
        ),
        (
            "struct NexaConfirmationDialogPrimitive<Actions: View>",
            "internal fun NexaConfirmationDialogPrimitive(",
            "NexaConfirmationDialogPrimitive(",
        ),
        (
            "struct NexaPageIndicatorRow: View",
            "internal fun NexaPageIndicatorRow(",
            "NexaPageIndicatorRow(",
        ),
        (
            "struct NexaNavigationSplitViewPrimitive<Sidebar: View, Detail: View>",
            "internal fun NexaNavigationSplitViewPrimitive(",
            "NexaNavigationSplitViewPrimitive(",
        ),
    ];
    assert!(swift_dev.contains("struct NexaAssetImagePrimitive"));
    assert!(swift_dev.contains("struct NexaRemoteImagePrimitive"));
    assert!(kotlin_dev.contains("internal fun NexaAssetImagePrimitive("));
    assert!(kotlin_dev.contains("internal fun NexaRemoteImagePrimitive("));
    assert!(swift_runtime.contains("NexaAssetImagePrimitive("));
    assert!(swift_runtime.contains("NexaRemoteImagePrimitive("));
    assert!(kotlin_runtime.contains("NexaAssetImagePrimitive("));
    assert!(kotlin_runtime.contains("NexaRemoteImagePrimitive("));

    for (swift_declaration, kotlin_declaration, call) in catalog {
        assert!(
            swift_release.contains(swift_declaration),
            "iOS release host omitted {swift_declaration}"
        );
        assert!(
            swift_dev.contains(swift_declaration),
            "iOS DevRuntime host omitted {swift_declaration}"
        );
        assert!(
            kotlin_release.contains(kotlin_declaration),
            "Android release host omitted {kotlin_declaration}"
        );
        assert!(
            kotlin_dev.contains(kotlin_declaration),
            "Android DevRuntime host omitted {kotlin_declaration}"
        );
        assert!(
            swift_runtime.contains(call),
            "iOS DevRuntime does not dispatch through {call}"
        );
        assert!(
            kotlin_runtime.contains(call),
            "Android DevRuntime does not dispatch through {call}"
        );
    }

    assert!(kotlin_release.contains("internal fun NexaFormDividerPrimitive()"));
    assert!(kotlin_dev.contains("internal fun NexaFormDividerPrimitive()"));
    assert!(kotlin_runtime.contains("NexaFormDividerPrimitive()"));
    assert!(!swift_release.contains("struct NexaAppBottomBarPrimitive: ViewModifier"));
    assert!(swift_dev.contains("struct NexaAppBottomBarPrimitive: ViewModifier"));
    assert!(swift_runtime.contains("NexaAppBottomBarPrimitive("));

    // Dev hosts may add a tab bar in a hot-reloaded module even when the
    // starting app is empty. Release apps include this primitive only when
    // their optimized IR actually uses AppBottomBar.
    assert!(!kotlin_release.contains("internal fun NexaAppBottomBarPrimitive("));
    assert!(kotlin_dev.contains("internal fun NexaAppBottomBarPrimitive("));
    assert!(kotlin_runtime.contains("NexaAppBottomBarPrimitive("));

    for (declaration, call) in [
        (
            "internal fun NexaLargeTitlePrimitive(",
            "NexaLargeTitlePrimitive(",
        ),
        (
            "internal fun NexaBottomSheetPrimitive(",
            "NexaBottomSheetPrimitive(",
        ),
        (
            "internal fun NexaAlertDialogPrimitive(",
            "NexaAlertDialogPrimitive(",
        ),
        (
            "internal fun NexaConfirmationDialogPrimitive(",
            "NexaConfirmationDialogPrimitive(",
        ),
        (
            "internal fun NexaDialogActionDividerPrimitive()",
            "NexaDialogActionDividerPrimitive()",
        ),
    ] {
        assert!(
            kotlin_release.contains(declaration),
            "Android AOT host omitted {declaration}"
        );
        assert!(
            kotlin_dev.contains(declaration),
            "Android DevRuntime host omitted {declaration}"
        );
        assert!(
            kotlin_runtime.contains(call),
            "Android DevRuntime does not dispatch through {call}"
        );
    }
}

fn assert_runtime_dispatch(runtime: &str, enum_name: &str, variant: &str, platform: &str) {
    let marker = format!("\"{variant}\"");
    assert!(
        runtime.contains(&marker),
        "{platform} dev runtime has no dispatch for {enum_name}::{variant}"
    );
}

fn section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap_or_else(|| panic!("runtime section `{start}` was not found"))
        .1
        .split_once(end)
        .unwrap_or_else(|| panic!("runtime section `{end}` was not found"))
        .0
}

#[test]
fn inventory_tracks_every_public_ir_node_and_interpreter_variant() {
    let (root, fixture) = fixture();
    let source_path = root.join(fixture["source"].as_str().expect("source path"));
    let source = fs::read_to_string(source_path).expect("read typed IR declarations");
    let inventory = fixture["ir_variants"]
        .as_object()
        .expect("IR inventory object");
    let require_coverage = std::env::var_os("NEXA_REQUIRE_HOT_RELOAD_COVERAGE").is_some();

    let inventoried_enums = inventory.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(inventoried_enums, public_enums(&source));

    for (enum_name, entries) in inventory {
        let expected = variants(&source, enum_name);
        let entries = entries
            .as_array()
            .unwrap_or_else(|| panic!("invalid {enum_name} coverage entries"));
        let actual = entries
            .iter()
            .map(|entry| {
                for platform in ["ios", "android"] {
                    let status = entry[platform].as_str();
                    assert!(
                        matches!(
                            status,
                            Some("pending" | "covered" | "partial" | "native_rebuild" | "aot_only")
                        ),
                        "{enum_name}::{} needs an explicit {platform} coverage status",
                        entry["name"]
                    );
                    let name = entry["name"].as_str().expect("variant name");
                    if let Some(boundary) = dev_boundary(enum_name, name) {
                        assert_eq!(
                            status,
                            Some(boundary),
                            "{enum_name}::{name} has the wrong Dev boundary for {platform}"
                        );
                    } else if status == Some("native_rebuild") || status == Some("aot_only") {
                        panic!(
                            "{enum_name}::{name} has an undocumented Dev boundary for {platform}"
                        );
                    }
                    if require_coverage {
                        assert_ne!(
                            status,
                            Some("pending"),
                            "{enum_name}::{} has no {platform} coverage evidence",
                            entry["name"]
                        );
                    }
                }
                entry["name"].as_str().expect("variant name").to_owned()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{enum_name} coverage inventory is stale");
    }
}

#[test]
fn shared_element_image_fields_have_dev_renderers_on_both_platforms() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS dev renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android dev renderer");
    let swift_root = fs::read_to_string(root.join("../../runtime/ios/NexaDevRuntime.swift"))
        .expect("read iOS dev root");
    let kotlin_root = fs::read_to_string(root.join("../../runtime/android/NexaDevRuntime.kt"))
        .expect("read Android dev root");

    assert!(swift.contains("fields[\"shared_element\"]"));
    assert!(swift.contains("matchedGeometryEffect(id: id, in: namespace)"));
    assert!(swift_root.contains(".environment(\\.nexaSharedNamespace, nexaSharedNamespace)"));
    assert!(kotlin.contains("fields.opt(\"shared_element\")"));
    assert!(kotlin.contains("nexaSharedElementModifier("));
    assert!(kotlin_root.contains("NexaSharedTransitionContent {"));
}

#[test]
fn dev_runtime_roots_preserve_aot_layout_background_and_binding_updates() {
    let (root, _) = fixture();
    let swift_root = fs::read_to_string(root.join("../../runtime/ios/NexaDevRuntime.swift"))
        .expect("read iOS dev root");
    let swift_state = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS dev state store");
    let kotlin_root = fs::read_to_string(root.join("../../runtime/android/NexaDevRuntime.kt"))
        .expect("read Android dev root");

    assert!(swift_root.contains("if let module = runtime.module {"));
    assert!(swift_root.contains(".overlay(alignment: .topTrailing)"));
    assert!(!swift_root.contains("alignment: .topLeading"));
    assert!(swift_state.contains("func setValue(_ name: String, value: Any, scope: String)"));
    assert!(swift_state.contains("current.isEqual(next)"));
    assert!(swift_state.contains("invalidateReaders(of: storageKey, sourceScope: scope)"));
    assert!(kotlin_root.contains("Surface(Modifier.fillMaxSize()) {"));
}

#[test]
fn hot_reload_interpreter_variants_have_both_native_dispatches() {
    let (root, fixture) = fixture();
    let inventory = fixture["ir_variants"]
        .as_object()
        .expect("IR inventory object");
    let swift_dir = root.join("../../runtime/ios");
    let swift = fs::read_dir(&swift_dir)
        .expect("read iOS runtime directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "swift"))
        .map(|entry| fs::read_to_string(entry.path()).expect("read swift runtime file"))
        .collect::<Vec<_>>()
        .join("\n");
    let kotlin_dir = root.join("../../runtime/android");
    let kotlin = fs::read_dir(&kotlin_dir)
        .expect("read Android runtime directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "kt"))
        .map(|entry| fs::read_to_string(entry.path()).expect("read kotlin runtime file"))
        .collect::<Vec<_>>()
        .join("\n");

    // This is an inventory guard: it checks for dispatch markers, not semantic
    // parity or successful execution of every variant. Device acceptance needs
    // separate runtime tests.
    let dispatched_enums = ["Expr", "Action", "CollectionMutation"];
    for enum_name in dispatched_enums {
        for entry in inventory[enum_name]
            .as_array()
            .unwrap_or_else(|| panic!("missing {enum_name} inventory"))
        {
            let variant = entry["name"].as_str().expect("variant name");
            if dev_boundary(enum_name, variant).is_some()
                || is_semantically_probed_plugin_variant(enum_name, variant)
            {
                continue;
            }
            assert_runtime_dispatch(&swift, enum_name, variant, "iOS");
            assert_runtime_dispatch(&kotlin, enum_name, variant, "Android");
        }
    }

    let node_metadata = [
        "StatusBar",
        "Direction",
        "OnAppear",
        "OnDisappear",
        "OnActive",
        "OnInactive",
        "OnBackground",
    ];
    for entry in inventory["Node"].as_array().expect("Node inventory") {
        let variant = entry["name"].as_str().expect("variant name");
        if node_metadata.contains(&variant)
            || dev_boundary("Node", variant).is_some()
            || is_semantically_probed_plugin_variant("Node", variant)
        {
            continue;
        }
        assert_runtime_dispatch(&swift, "Node", variant, "iOS");
        assert_runtime_dispatch(&kotlin, "Node", variant, "Android");
    }
}

#[test]
fn system_icons_and_gradient_directions_have_both_native_renderers() {
    let (root, fixture) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS dev renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android dev renderer");
    let swift_icons = section(&swift, "case \"SystemIcon\":", "case \"LinearGradient\":");
    let kotlin_icons = section(&kotlin, "\"SystemIcon\" -> {", "\"LinearGradient\" -> {");
    for (kind, swift_key, kotlin_key) in [
        ("shared", "icon[\"shared\"]", "selection.has(\"shared\")"),
        (
            "SF Symbol",
            "icon[\"sf_symbol\"]",
            "selection.has(\"sf_symbol\")",
        ),
        (
            "Material Symbol",
            "icon[\"material_symbol\"]",
            "selection.has(\"material_symbol\")",
        ),
    ] {
        assert!(
            swift_icons.contains(swift_key),
            "iOS dev renderer has no {kind} system icon mapping"
        );
        assert!(
            kotlin_icons.contains(kotlin_key),
            "Android dev renderer has no {kind} system icon mapping"
        );
    }

    let swift_gradients = section(&swift, "case \"LinearGradient\":", "case \"Image\":");
    let kotlin_gradients = section(&kotlin, "\"LinearGradient\" -> {", "\"Button\" ->");
    for entry in fixture["ir_variants"]["GradientDirection"]
        .as_array()
        .expect("GradientDirection inventory")
    {
        let name = entry["name"].as_str().expect("gradient direction variant");
        assert!(
            swift_gradients.contains(&format!("\"{name}\"")),
            "iOS dev renderer has no GradientDirection::{name} handling"
        );
        assert!(
            kotlin_gradients.contains(&format!("\"{name}\"")),
            "Android dev renderer has no GradientDirection::{name} handling"
        );
    }
}

#[test]
fn semantic_text_font_roles_match_native_metrics_in_both_dev_runtimes() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS dev renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android dev renderer");
    let swift_text = section(&swift, "case \"Text\":", "case \"Spacer\":");
    let kotlin_text = section(&kotlin, "\"Text\" -> {", "\"Spacer\" -> Spacer");

    for (role, swift_style, kotlin_size, kotlin_weight) in [
        ("LargeTitle", ".largeTitle", 34, "Normal"),
        ("Title", ".title", 28, "Normal"),
        ("Title2", ".title2", 22, "Normal"),
        ("Title3", ".title3", 20, "Normal"),
        ("Headline", ".headline", 17, "SemiBold"),
        ("Subheadline", ".subheadline", 15, "Normal"),
        ("Body", ".body", 17, "Normal"),
        ("Callout", ".callout", 16, "Normal"),
        ("Footnote", ".footnote", 13, "Normal"),
        ("Caption", ".caption", 12, "Normal"),
        ("Caption2", ".caption2", 11, "Normal"),
    ] {
        assert!(
            swift_text.contains(&format!("case \"{role}\": {swift_style}")),
            "iOS DevRuntime has no {role} text style mapping"
        );
        assert!(
            kotlin_text.contains(&format!(
                "\"{role}\" -> {kotlin_size}.sp to FontWeight.{kotlin_weight}"
            )),
            "Android DevRuntime has no matching {role} font metrics"
        );
    }
    assert!(swift_text.contains("style[\"font_size\"]"));
    assert!(swift_text.contains("NexaTextPrimitive("));
    assert!(kotlin_text.contains("NexaTextPrimitive("));
    assert!(kotlin_text.contains("fontSize * nexaDevDefaultLineHeightMultiplier"));
}

#[test]
fn calendar_day_arithmetic_has_native_and_dev_runtime_mappings() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS dev state evaluator");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state evaluator");
    assert!(swift.contains("case \"AddCalendarDays\":"));
    assert!(swift.contains("nexaDevAddCalendarDays(timestamp.int64Value, days.int32Value)"));
    assert!(kotlin.contains("\"AddCalendarDays\" ->"));
    assert!(kotlin.contains("nexaDevAddCalendarDays(timestamp, days)"));
}

#[test]
fn localized_date_time_formatting_has_both_dev_runtime_mappings() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS dev state evaluator");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state evaluator");

    for method in ["LocalizedDate", "LocalizedTime", "LocalizedDateTime"] {
        assert!(
            swift.contains(&format!("case \"{method}\":")),
            "iOS DevRuntime has no {method} dispatch"
        );
        assert!(
            kotlin.contains(&format!("\"{method}\" ->")),
            "Android DevRuntime has no {method} dispatch"
        );
    }
    assert!(swift.contains("DateFormatter.localizedString(from: date"));
    assert!(kotlin.contains("java.text.DateFormat.getDateTimeInstance("));
}

#[test]
fn module_initializers_resolve_prior_values_during_install_and_hot_restart() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS dev state evaluator");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state evaluator");

    assert!(swift.contains("evaluate(initial, locals: initialLocals, scope: scope)"));
    assert!(swift.contains("evaluate(initial, locals: initialLocals, scope: \"app\")"));
    assert!(kotlin.contains("evaluate(declaration.get(\"initial\"), initialLocals, scope)"));
    assert!(kotlin.contains("evaluate(state.get(\"initial\"), initialLocals, \"app\")"));
}

#[test]
fn async_member_access_handles_tuple_positions_and_collection_size() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS state evaluator");
    let swift_async = swift
        .split_once("func evaluateAsync(")
        .expect("iOS async evaluator")
        .1;
    let swift_member = swift_async
        .split_once("case \"Member\":")
        .expect("iOS async Member evaluator")
        .1
        .split_once("case \"Array\", \"Set\":")
        .expect("iOS async Array evaluator")
        .0;
    for marker in [
        "case \"first\": 0",
        "case \"second\": 1",
        "case \"third\": 2",
        "name == \"trimmed\"",
        "name == \"count\"",
        "name == \"isEmpty\"",
    ] {
        assert!(
            swift_member.contains(marker),
            "iOS async Member evaluator is missing {marker}"
        );
    }

    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android state evaluator");
    let kotlin_async = kotlin
        .split_once("suspend fun evaluateAsync(")
        .expect("Android async evaluator")
        .1;
    let kotlin_member = kotlin_async
        .split_once("\"Member\" -> {")
        .expect("Android async Member evaluator")
        .1
        .split_once("\"Array\", \"Set\" -> {")
        .expect("Android async Array evaluator")
        .0;
    for marker in [
        "\"first\" -> base.getOrNull(0)",
        "\"second\" -> base.getOrNull(1)",
        "\"third\" -> base.getOrNull(2)",
        "\"trimmed\" -> (base as? String)?.trim()",
        "\"count\" ->",
        "\"isEmpty\" ->",
    ] {
        assert!(
            kotlin_member.contains(marker),
            "Android async Member evaluator is missing {marker}"
        );
    }
}

#[test]
fn typed_plugin_failures_reach_sync_and_async_catch_arms() {
    let (root, _) = fixture();
    let swift_actions = fs::read_to_string(root.join("../../runtime/ios/NexaDevActions.swift"))
        .expect("read iOS action evaluator");
    let kotlin_actions = fs::read_to_string(root.join("../../runtime/android/NexaDevActions.kt"))
        .expect("read Android action evaluator");
    let swift_native = fs::read_to_string(root.join("../../runtime/ios/NexaDevNativeApis.swift"))
        .expect("read iOS native adapters");
    let kotlin_native = fs::read_to_string(root.join("../../runtime/android/NexaDevNativeApis.kt"))
        .expect("read Android native adapters");

    for (platform, source) in [
        ("iOS", swift_actions.as_str()),
        ("Android", kotlin_actions.as_str()),
    ] {
        for marker in [
            "performPluginFailureCatch(",
            "performPluginFailureCatchAsync(",
            "failure.namespace",
            "failure.errorType",
            "failure.variant",
            "failure.payload[property]",
        ] {
            assert!(
                source.contains(marker),
                "{platform} DevRuntime typed error routing is missing {marker}"
            );
        }
    }
    assert!(swift_native.contains("pendingPluginFailure = failure"));
    assert!(kotlin_native.contains("pendingPluginFailure = failure"));
}

#[test]
fn generic_plugin_result_values_have_both_runtime_codecs() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevValueCodec.swift"))
        .expect("read iOS generic value codec");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevValueCodec.kt"))
        .expect("read Android generic value codec");

    assert!(swift.contains("case \"Result\":"));
    assert!(swift.contains("writer.writeBool(true)"));
    assert!(swift.contains("return [\"Ok\": success]"));
    assert!(swift.contains("return [\"Err\": failure]"));
    assert!(kotlin.contains("\"Result\" -> {"));
    assert!(kotlin.contains("writer.writeBool(true)"));
    assert!(kotlin.contains("mapOf(\"Ok\" to success)"));
    assert!(kotlin.contains("mapOf(\"Err\" to failure)"));
}

#[test]
fn optional_generic_plugin_arguments_have_both_runtime_writers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevValueCodec.swift"))
        .expect("read iOS generic value codec");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevValueCodec.kt"))
        .expect("read Android generic value codec");

    assert!(swift.contains("if kind == \"Optional\""));
    assert!(swift.contains("writer.writeBool(false)"));
    assert!(swift.contains("static func box(_ raw: Any?) -> NexaDevHashableValue?"));
    assert!(kotlin.contains("if (kind == \"Optional\")"));
    assert!(kotlin.contains("writer.writeBool(present)"));
    assert!(kotlin.contains("raw != JSONObject.NULL"));
}

#[test]
fn async_expression_evaluator_recurses_through_nested_values() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS state evaluator");
    let swift_async = swift
        .split_once("func evaluateAsync(")
        .expect("iOS async evaluator")
        .1;
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android state evaluator");
    let kotlin_async = kotlin
        .split_once("suspend fun evaluateAsync(")
        .expect("Android async evaluator")
        .1;

    for (platform, source, markers) in [
        (
            "iOS",
            swift_async,
            [
                "case \"ResultOk\", \"ResultErr\":",
                "try await evaluateAsync(fields[key]",
                "case \"Array\", \"Set\":",
                "case \"Map\":",
                "case \"Conditional\":",
                "if op == \"And\"",
            ],
        ),
        (
            "Android",
            kotlin_async,
            [
                "\"ResultOk\", \"ResultErr\" ->",
                "evaluateAsync(fields.opt(key)",
                "\"Array\", \"Set\" ->",
                "\"Map\" ->",
                "\"Conditional\" ->",
                "if (op == \"And\"",
            ],
        ),
    ] {
        for marker in markers {
            assert!(
                source.contains(marker),
                "{platform} async expression evaluator is missing {marker}"
            );
        }
    }
}

#[test]
fn async_expression_coverage_requires_log_only_semantic_device_evidence() {
    let (root, fixture) = fixture();
    let feature = &fixture["runtime_features"]["async_expression_evaluation"];
    let script =
        fs::read_to_string(root.join("../../scripts/test-dev-runtime-plugin-hot-reload.sh"))
            .expect("read log-only DevRuntime probe");
    let app = fs::read_to_string(root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.nx"))
        .expect("read initial probe app");
    let template = fs::read_to_string(
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.after.template"),
    )
    .expect("read hot-reload probe app");

    for platform in ["ios", "android"] {
        assert_eq!(
            feature[platform].as_str(),
            Some("covered"),
            "async expression evaluation lacks {platform} device evidence"
        );
    }
    for marker in feature["semantic_probe_markers"]
        .as_array()
        .expect("async expression semantic markers")
    {
        let marker = marker.as_str().expect("string semantic marker");
        assert!(
            script.contains(marker),
            "device probe does not wait for {marker}"
        );
        assert!(app.contains(marker), "initial app does not emit {marker}");
        assert!(
            template.contains(marker),
            "hot-reload app does not emit {marker}"
        );
    }
    assert!(script.contains("DEVRT_ASYNC_NESTED_EXPRESSION_FAIL"));
    assert!(app.contains("DEVRT_ASYNC_NESTED_EXPRESSION_FAIL"));
    assert!(template.contains("DEVRT_ASYNC_NESTED_EXPRESSION_FAIL"));
}

#[test]
fn foreground_task_hot_reload_has_log_only_semantic_device_evidence() {
    let (root, fixture) = fixture();
    let feature = &fixture["runtime_features"]["foreground_tasks"];
    let script =
        fs::read_to_string(root.join("../../scripts/test-dev-runtime-plugin-hot-reload.sh"))
            .expect("read log-only DevRuntime probe");
    let app = fs::read_to_string(root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.nx"))
        .expect("read foreground task probe app");
    let template = fs::read_to_string(
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.after.template"),
    )
    .expect("read foreground task hot-reload app");
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevActions.swift"))
        .expect("read iOS action evaluator");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevActions.kt"))
        .expect("read Android action evaluator");

    for platform in ["ios", "android"] {
        assert_eq!(
            feature[platform].as_str(),
            Some("covered"),
            "foreground tasks lack {platform} device-log evidence"
        );
    }
    for marker in feature["semantic_probe_markers"]
        .as_array()
        .expect("foreground task semantic markers")
    {
        let marker = marker.as_str().expect("string semantic marker");
        assert!(
            script.contains(marker),
            "device probe does not wait for {marker}"
        );
        assert!(app.contains(marker), "initial app does not emit {marker}");
        assert!(
            template.contains(marker),
            "hot-reload app does not emit {marker}"
        );
    }
    for (platform, source) in [("iOS", swift.as_str()), ("Android", kotlin.as_str())] {
        for marker in [
            "TaskLaunch",
            "TaskCancel",
            "launchNativeTask",
            "clearNativeTasks",
        ] {
            assert!(
                source.contains(marker),
                "{platform} task runtime is missing {marker}"
            );
        }
    }
}

#[test]
fn scoped_imperative_animation_has_log_only_device_evidence() {
    let (root, fixture) = fixture();
    let feature = &fixture["runtime_features"]["scoped_imperative_animation"];
    let script =
        fs::read_to_string(root.join("../../scripts/test-dev-runtime-plugin-hot-reload.sh"))
            .expect("read log-only DevRuntime probe");
    let app = fs::read_to_string(root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.nx"))
        .expect("read initial animation probe app");
    let template = fs::read_to_string(
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.after.template"),
    )
    .expect("read hot-reload animation probe app");

    for platform in ["ios", "android"] {
        assert_eq!(
            feature[platform].as_str(),
            Some("covered"),
            "scoped imperative animation lacks {platform} device-log evidence"
        );
    }
    for marker in feature["semantic_probe_markers"]
        .as_array()
        .expect("animation semantic markers")
    {
        let marker = marker.as_str().expect("string semantic marker");
        assert!(
            script.contains(marker),
            "device probe does not wait for {marker}"
        );
        assert!(app.contains(marker), "initial app does not emit {marker}");
        assert!(
            template.contains(marker),
            "hot-reload app does not emit {marker}"
        );
    }
}

#[test]
fn pressable_double_tap_actions_are_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("fields[\"double_tap_actions\"]"));
    assert!(swift.contains("TapGesture(count: 2)"));
    assert!(swift.contains("store.perform(doubleTapActions"));
    assert!(kotlin.contains("fields.optJSONArray(\"double_tap_actions\")"));
    assert!(kotlin.contains("onDoubleClick = {"));
    assert!(kotlin.contains("store.perform(doubleTapActions"));
}

#[test]
fn pressable_long_press_duration_is_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("fields[NexaDevKeys.longPressDurationMs]"));
    assert!(swift.contains("LongPressGesture(minimumDuration: longPressMinimumDuration)"));
    assert!(kotlin.contains("NexaDevKeys.LONG_PRESS_DURATION_MS"));
    assert!(kotlin.contains("override val longPressTimeoutMillis: Long = longPressDurationMs"));
}

#[test]
fn pressable_pinch_actions_are_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("fields[NexaDevKeys.pinchParameter]"));
    assert!(swift.contains("MagnificationGesture()"));
    assert!(swift.contains("store.perform(pinchActions"));
    assert!(kotlin.contains("fields.nexaDevOptionalString(NexaDevKeys.PINCH_PARAMETER)"));
    assert!(
        !kotlin.contains("fields.optString(NexaDevKeys.PINCH_PARAMETER)"),
        "JSON null must not make a non-pinch Pressable install a transform gesture"
    );
    assert!(kotlin.contains("detectTransformGestures"));
    assert!(kotlin.contains("store.perform(pinchActions"));
}

#[test]
/// Checks the DevRuntime conditional rendering paths and their transition table.
/// Device animation timing still requires a native runtime test.
fn conditional_view_transitions_are_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("private func nexaDevTransition(_ raw: Any?) -> AnyTransition?"));
    assert!(
        swift.contains("rendered.transition(transition).animation(.default, value: condition)")
    );
    assert!(swift.contains("rendered.transition(transition).animation(.default, value: value)"));
    for transition in [
        "Fade",
        "SlideFromBottom",
        "SlideFromLeft",
        "SlideFromRight",
        "Scale",
    ] {
        assert!(kotlin.contains(&format!("\"{transition}\" ->")));
    }
    assert!(kotlin.contains("AnimatedContent(\n                    targetState = condition"));
    assert!(kotlin.contains("AnimatedContent(\n                    targetState = value"));
}

#[test]
/// Scenario labels are maintainer inventory metadata; this test does not run
/// each listed scenario on a simulator or emulator.
fn runtime_acceptance_scenarios_have_explicit_dual_platform_status() {
    let (_, fixture) = fixture();
    let scenarios = fixture["runtime_scenarios"]
        .as_object()
        .expect("runtime scenario inventory");
    let require_coverage = std::env::var_os("NEXA_REQUIRE_HOT_RELOAD_COVERAGE").is_some();
    for (name, status) in scenarios {
        for platform in ["ios", "android"] {
            let value = status[platform].as_str();
            assert!(
                matches!(value, Some("pending" | "covered")),
                "scenario {name} needs an explicit {platform} test status"
            );
            if require_coverage {
                assert_eq!(
                    value,
                    Some("covered"),
                    "scenario {name} is not covered on {platform}"
                );
            }
        }
    }
}

#[test]
/// Feature labels describe current boundaries and are not runtime test results.
fn runtime_feature_gaps_have_explicit_dual_platform_status() {
    let (_, fixture) = fixture();
    let features = fixture["runtime_features"]
        .as_object()
        .expect("runtime feature inventory");
    let require_coverage = std::env::var_os("NEXA_REQUIRE_HOT_RELOAD_COVERAGE").is_some();
    for (name, status) in features {
        for platform in ["ios", "android"] {
            let value = status[platform].as_str();
            assert!(
                matches!(
                    value,
                    Some("pending" | "covered" | "partial" | "native_rebuild" | "aot_only")
                ),
                "runtime feature {name} needs an explicit {platform} test status"
            );
            if require_coverage {
                assert_ne!(
                    value,
                    Some("pending"),
                    "runtime feature {name} has no audited boundary for {platform}"
                );
            }
        }
    }
}

#[test]
fn interactive_diagnostics_route_source_locations_from_both_dev_runtimes() {
    let (root, _) = fixture();
    let ios_protocol = fs::read_to_string(root.join("../../runtime/ios/NexaDevProtocol.swift"))
        .expect("read iOS dev protocol");
    let ios_overlay = fs::read_to_string(root.join("../../runtime/ios/NexaDevOverlay.swift"))
        .expect("read iOS dev overlay");
    let ios_state = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS dev state");
    let ios_actions = fs::read_to_string(root.join("../../runtime/ios/NexaDevActions.swift"))
        .expect("read iOS dev actions");
    let android_protocol =
        fs::read_to_string(root.join("../../runtime/android/NexaDevProtocol.kt"))
            .expect("read Android dev protocol");
    let android_overlay = fs::read_to_string(root.join("../../runtime/android/NexaDevOverlay.kt"))
        .expect("read Android dev overlay");
    let android_state = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state");
    let android_actions = fs::read_to_string(root.join("../../runtime/android/NexaDevActions.kt"))
        .expect("read Android dev actions");
    let server = fs::read_to_string(root.join("../../crates/nexa-dev-server/src/lib.rs"))
        .expect("read dev server");
    let cli = fs::read_to_string(root.join("../../crates/nexa-cli/src/commands.rs"))
        .expect("read CLI commands");

    assert!(ios_protocol.contains("NexaDevKeys.msgOpenInEditor"));
    assert!(ios_protocol.contains("NexaDevDiagnostic(file: file, line: line, column: column"));
    assert!(ios_overlay.contains("Development diagnostics"));
    assert!(ios_overlay.contains("onOpenInEditor(item)"));
    assert!(ios_overlay.contains("item.stackTrace"));
    assert!(ios_overlay.contains("source mapping unavailable"));
    assert!(ios_state.contains("onRuntimeFailure?(error, capturedStack)"));
    assert!(ios_actions.contains("self.reportRuntimeFailure(error)"));
    assert!(android_protocol.contains("NexaDevKeys.MSG_OPEN_IN_EDITOR"));
    assert!(android_protocol.contains("NexaDevDiagnostic("));
    assert!(android_overlay.contains("Development diagnostics"));
    assert!(android_overlay.contains("onOpenInEditor(diagnostic)"));
    assert!(android_overlay.contains("diagnostic.stackTrace"));
    assert!(android_overlay.contains("source mapping unavailable"));
    assert!(android_state.contains("error.stackTraceToString()"));
    assert!(android_actions.contains("store.reportRuntimeFailure(error)"));
    assert!(server.contains("ClientMessage::OpenInEditor { file, line, column }"));
    assert!(cli.contains("vscode://file/"));
}

#[test]
fn appearance_wrapper_is_rendered_by_both_dev_runtimes() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");
    let kotlin_navigation =
        fs::read_to_string(root.join("../../runtime/android/NexaDevNavigation.kt"))
            .expect("read Android navigation renderer");
    let (kotlin_dev_host, _) =
        KotlinBackend.generate_for_dev_with_project_features(&empty_module(Vec::new()));
    assert!(swift.contains("case \"Appearance\":"));
    assert!(swift.contains("preferredColorScheme(mode == \"dark\""));
    assert!(kotlin.contains("\"Appearance\" -> {"));
    assert!(kotlin.contains("NexaAppearancePrimitive(mode = mode)"));
    assert!(kotlin_navigation.contains("NexaAppearancePrimitive(mode = appearanceMode)"));
    assert!(kotlin_dev_host.contains("internal fun NexaAppearancePrimitive("));
}

#[test]
fn plugin_hot_reload_coverage_uses_log_only_semantic_device_evidence() {
    let (root, fixture) = fixture();
    let script =
        fs::read_to_string(root.join("../../scripts/test-dev-runtime-plugin-hot-reload.sh"))
            .expect("read log-only plugin hot-reload probe");
    let probe_sources = [
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.nx"),
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/ProbePanel.nx"),
    ]
    .map(|path| fs::read_to_string(path).expect("read plugin probe source"))
    .join("\n");
    let features = fixture["runtime_features"]
        .as_object()
        .expect("runtime feature inventory");
    let expected = [
        "plugin_service_call_shapes",
        "typed_plugin_error_catches",
        "plugin_native_class_scalar_constructors_and_methods",
        "native_plugin_properties_events_and_components",
    ];

    assert!(script.contains("logcat"));
    assert!(script.contains("log stream"));
    assert!(script.contains("cp \"$app_fixture/ProbePanel.nx\" \"$tmp_root/ProbePanel.nx\""));
    assert!(script.contains("wait_for_marker \"$native_log\" DEVRT_IMPORTED_COMPONENT_PASS"));
    assert!(script.contains("Rebuilding native app"));

    for feature_name in expected {
        let feature = &features[feature_name];
        for platform in ["ios", "android"] {
            assert_eq!(
                feature[platform].as_str(),
                Some("covered"),
                "{feature_name} requires successful semantic device evidence on {platform}"
            );
        }
        let markers = feature["semantic_probe_markers"]
            .as_array()
            .unwrap_or_else(|| panic!("{feature_name} has no semantic probe markers"));
        assert!(
            !markers.is_empty(),
            "{feature_name} has no probe assertions"
        );
        for marker in markers {
            let marker = marker.as_str().expect("probe marker string");
            assert!(script.contains(marker), "probe does not wait for {marker}");
            assert!(
                probe_sources.contains(marker),
                "plugin probe source does not assert {marker}"
            );
        }
    }
}

#[test]
fn hot_reload_latency_gate_checks_end_to_end_apply_on_both_platforms() {
    let (root, _) = fixture();
    let script = fs::read_to_string(root.join("../../tests/hot-reload-latency.sh"))
        .expect("read end-to-end hot-reload latency gate");
    let workflow =
        fs::read_to_string(root.join("../../.github/workflows/ci.yml")).expect("read CI workflow");

    for marker in [
        "time.monotonic_ns()",
        "os.replace(temporary, path)",
        "hr_get_patch_count",
        "Nexa %s hot reload %s applied in %s ms (budget: 1000 ms)",
        "hr_wait_for_visible_text \"Reload 3\"",
    ] {
        assert!(script.contains(marker), "latency gate omitted {marker}");
    }
    assert!(
        script.contains("for revision in 1 2 3"),
        "latency gate must sample 3 edits"
    );
    assert_eq!(
        workflow.matches("tests/hot-reload-latency.sh").count(),
        2,
        "the iOS and Android CI jobs must both run the latency gate"
    );
    assert!(workflow.contains("Run the iOS hot-reload latency gate"));
    assert!(workflow.contains("Run the Android hot-reload latency gate"));
}

#[test]
fn hot_reload_component_and_fast_list_gates_run_on_both_platforms() {
    let (root, _) = fixture();
    let workflow =
        fs::read_to_string(root.join("../../.github/workflows/ci.yml")).expect("read CI workflow");
    let components = fs::read_to_string(root.join("../../tests/hot-reload-components.sh"))
        .expect("read cross-platform component smoke gate");

    for (script, label) in [
        ("tests/hot-reload-components.sh", "component and state"),
        ("tests/hot-reload-fast-list.sh", "FastList"),
    ] {
        assert_eq!(
            workflow.matches(script).count(),
            2,
            "iOS and Android CI jobs must both run {script}"
        );
        for platform in ["iOS", "Android"] {
            assert!(
                workflow.contains(&format!("Run the {platform} hot-reload {label} gate")),
                "{platform} CI job omitted the {label} gate"
            );
        }
    }
    assert!(components.contains("Nexa app state restarted in the running app."));
    assert!(components.contains("Android Button action did not update state"));
    assert!(components.contains("Android If branch did not update after Button interaction"));
    assert!(components.contains("text=\"Size classes: "));
    assert!(
        !components.contains("input swipe 360"),
        "Android smoke interactions must use device-derived coordinates"
    );
}

#[test]
fn aot_and_devruntime_accessibility_and_screenshot_parity_runs_on_both_platforms() {
    let (root, _) = fixture();
    let workflow =
        fs::read_to_string(root.join("../../.github/workflows/ci.yml")).expect("read CI workflow");
    let script = fs::read_to_string(root.join("../../tests/hot-reload-aot-parity.sh"))
        .expect("read paired AOT and DevRuntime gate");
    let comparator = fs::read_to_string(root.join("../../tests/compare-render-parity.py"))
        .expect("read rendering parity comparator");
    let ios_ui_test = fs::read_to_string(root.join("../../tests/ios-render-parity-ui.swift"))
        .expect("read iOS rendering parity UI test");
    let fixture = fs::read_to_string(root.join("../../tests/fixtures/render_parity.nx"))
        .expect("read rendering parity fixture");

    assert_eq!(
        workflow.matches("tests/hot-reload-aot-parity.sh").count(),
        2
    );
    assert!(workflow.contains("Compare iOS AOT and DevRuntime rendering"));
    assert!(workflow.contains("Compare Android AOT and DevRuntime rendering"));
    for marker in [
        "Nexa iOS dev runtime applied module",
        "Nexa Android dev runtime applied module",
        "nexa-render-parity-$phase.png",
        "compare-render-parity.py",
        "uiautomator dump",
        "xcodebuild test",
        "adb shell pm clear \"$app_id\" >/dev/null",
    ] {
        assert!(script.contains(marker), "paired gate omitted {marker}");
    }
    for marker in [
        "initial",
        "toolbar",
        "button",
        "switch",
        "text-entry",
        "controls-scroll-3",
        "controls-scroll-5",
        "navigation",
        "lists",
        "lists-refresh",
        "lists-scroll-1",
        "lists-scroll-3",
        "list-parameters",
        "list-parameters-next",
        "workspace",
        "pages",
        "pages-next",
        "style-layout",
        "style-typography",
        "style-button",
        "style-button-interaction",
        "style-input",
        "style-images",
        "style-pressable",
        "large-sheet",
        "bottom-sheet",
        "dialog",
        "confirmation",
    ] {
        assert!(
            script.contains(marker),
            "paired gate omitted {marker} phase"
        );
        assert!(
            ios_ui_test.contains(marker),
            "iOS interaction test omitted {marker} phase"
        );
    }
    for marker in [
        "roles, labels, actions, and frames",
        "screenshot matches",
        "status and navigation",
    ] {
        assert!(comparator.contains(marker), "comparator omitted {marker}");
    }
    for marker in [
        "Button(\"Increment:",
        "Button(\"Toolbar action\"",
        "Switch(value:",
        "TextInput(value:",
        "ParityCard(",
        "\"Style parameter coverage\",",
        "fontSize: 14,",
        "screen StyleParameters {",
        "Typography style parameters",
        "alignment: Center,",
        "fontStyle: Subheadline,",
        "lineHeight: 20,",
        "Button(\n                    \"Configured button\"",
        "searchable: true",
        "materialsymbol: \"star\"",
        "reverseLayout: true",
        "pageSnap: true",
        "screen ListParameters {",
    ] {
        assert!(fixture.contains(marker), "parity fixture omitted {marker}");
    }
    let main_and_detail = fixture.split("screen TextEntry").next().unwrap_or_default();
    assert!(
        !main_and_detail.contains("KeyboardAware"),
        "KeyboardAware must not be nested in the vertically scrolling Form"
    );
    assert!(fixture.contains("screen TextEntry {\n        KeyboardAware(dismiss: Never)"));
    for style_modifier in [
        ".fontSize(",
        ".bold()",
        ".padding(",
        ".opacity(",
        ".scale(",
        ".rotation(",
        ".shadow(",
        ".blur(",
        ".clip(",
        ".zIndex(",
    ] {
        assert!(
            !fixture.contains(style_modifier),
            "style coverage must pass styling as parameters, not dot modifiers: {style_modifier}"
        );
    }
    assert!(ios_ui_test.contains("enabled.value as? String, \"1\""));
    assert!(ios_ui_test.contains("app.screenshot()"));
}

#[test]
fn keyboard_ergonomics_are_consumed_by_dev_runtime() {
    let (root, _) = fixture();
    let swift_renderer = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin_renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");
    let swift_native = fs::read_to_string(root.join("../../runtime/ios/NexaDevNativeApis.swift"))
        .expect("read iOS native API adapter");
    let kotlin_native = fs::read_to_string(root.join("../../runtime/android/NexaDevNativeApis.kt"))
        .expect("read Android native API adapter");
    let kotlin_core = nexa_codegen::value::kotlin_core_runtime_source();

    for (platform, source, markers) in [
        (
            "iOS",
            swift_renderer.as_str(),
            [
                "case \"Username\": .username",
                "case \"Password\": .password",
                "case \"OneTimeCode\": .oneTimeCode",
                "case \"Search\": .search",
                "case \"Send\": .send",
                "case \"Next\": .next",
                "store.perform(submitActions, scope: scope, locals: locals)",
            ],
        ),
        (
            "Android",
            kotlin_renderer.as_str(),
            [
                "ContentType.Username",
                "ContentType.Password",
                "ContentType.SmsOtpCode",
                "ImeAction.Search",
                "ImeAction.Send",
                "ImeAction.Next",
                "store.perform(submitActions, scope, locals)",
            ],
        ),
    ] {
        for marker in markers {
            assert!(
                source.contains(marker),
                "{platform} DevRuntime keyboard handling is missing {marker}"
            );
        }
    }
    assert!(swift_native.contains("namespace == \"Keyboard\", name == \"dismiss\""));
    assert!(kotlin_native.contains("namespace == \"Keyboard\" && name == \"dismiss\""));
    assert!(kotlin_core.contains("foregroundActivity"));
    assert!(kotlin_core.contains("hideSoftInputFromWindow"));
}

#[test]
fn dev_renderers_keep_release_native_structure_for_common_controls() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");
    let swift_primitives =
        fs::read_to_string(root.join("../nexa-backend-swift/src/generator/mod.rs"))
            .expect("read shared Swift component primitives");

    for marker in [
        "else if nodes.count == 1",
        "spacing: nativeSpacing",
        "nexaDevGlass(tint:",
        "NexaNavigationTitleDisplayModePrimitive(",
        "NexaTabLabelPrimitive(title: Text(title), systemImage: name)",
    ] {
        assert!(swift.contains(marker), "iOS DevRuntime is missing {marker}");
    }
    assert!(
        swift_primitives
            .contains("content.toolbarTitleDisplayMode(large ? .inlineLarge : .inline)")
    );
    assert!(
        swift_primitives
            .contains("content.navigationBarTitleDisplayMode(large ? .large : .inline)")
    );
    assert!(swift_primitives.contains(".tabViewStyle(.sidebarAdaptable)"));
    assert!(swift_primitives.contains(".tabViewSearchActivation(.searchTabSelection)"));
    assert!(swift_primitives.contains("struct NexaTabLabelPrimitive: View"));
    assert!(swift_primitives.contains("Image(systemName: systemImage).accessibilityHidden(true)"));
    assert!(swift_primitives.contains(".accessibilityIdentifier(\"\")"));
    assert!(swift.contains("NexaRemoteImagePrimitive("));
    for marker in [
        "1 -> NexaDevNode(nexaDevNodeObject(nodes.opt(0))",
        "NexaTextInputPrimitive(",
        "strikethrough = style.optBoolean(\"strikethrough\")",
        "nexaDevSpecificMaterialIcon",
    ] {
        assert!(
            kotlin.contains(marker),
            "Android DevRuntime is missing {marker}"
        );
    }
    let kotlin_primitives =
        fs::read_to_string(root.join("../nexa-backend-kotlin/src/generator/mod.rs"))
            .expect("read shared Kotlin component primitives");
    assert!(kotlin_primitives.contains("val enabled = !loading && !disabled"));
    assert!(kotlin_primitives.contains("DialogProperties(usePlatformDefaultWidth = false)"));
}

#[test]
fn hot_reload_patch_application_avoids_full_module_clones_and_long_retry_waits() {
    let (root, _) = fixture();
    let android_state = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state store");
    let android_protocol =
        fs::read_to_string(root.join("../../runtime/android/NexaDevProtocol.kt"))
            .expect("read Android dev protocol");
    let ios_protocol = fs::read_to_string(root.join("../../runtime/ios/NexaDevProtocol.swift"))
        .expect("read iOS dev protocol");

    assert!(android_state.contains("val updated = shallowCopy(root)"));
    assert!(android_state.contains("private fun shallowCopy(value: Any): Any?"));
    assert!(
        !android_state.contains("JSONObject(root.toString())"),
        "an ordinary Android patch must not serialize and reparse the full Dev IR module"
    );
    assert!(android_protocol.contains("var retryDelayMs = 50L"));
    assert!(android_protocol.contains("Thread.sleep(retryDelayMs)"));
    assert!(!android_protocol.contains("Thread.sleep(1_000)"));
    assert!(ios_protocol.contains("var retryDelayNanoseconds: UInt64 = 50_000_000"));
    assert!(ios_protocol.contains("Task.sleep(nanoseconds: retryDelayNanoseconds)"));
    assert!(!ios_protocol.contains("Task.sleep(for: .seconds(1))"));
}

#[test]
fn android_fast_list_rows_use_one_native_primitive_in_aot_and_devruntime() {
    let count = Expr::Number {
        raw: "2".to_owned(),
        ty: NumericType::Int32,
    };
    let row = |value: &str| Node::Text {
        value: Expr::String(value.to_owned()),
        style: nexa_ir::TextStyle::default(),
    };
    let module = empty_module(vec![Node::FastList {
        plan: ListPlan::Count {
            count,
            common: ListCommon {
                axis: ListAxis::Vertical,
                native: false,
                reverse_layout: false,
                page_snap: false,
                item_extent: Some(72.0),
                index: "index".to_owned(),
                key: None,
                scroll_position: None,
                children: vec![row("Task title"), row("Task details")],
                on_end_reached: None,
                on_scroll: None,
                on_move: None,
                swipe_actions: Some(vec![Node::Button {
                    label: Expr::String("Delete".to_owned()),
                    icon: Some(nexa_ir::SystemIcon::Shared("delete".to_owned())),
                    loading: None,
                    disabled: None,
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    actions: Vec::new(),
                }]),
                sticky_header: None,
                refresh: None,
            },
        },
    }]);
    let aot = KotlinBackend.generate(&module);
    let keyed_aot = KotlinBackend.generate(&keyed_fast_list_module(false));
    let (dev_host, _) =
        KotlinBackend.generate_for_dev_with_project_features(&empty_module(Vec::new()));
    let (root, _) = fixture();
    let renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime list renderer");

    assert!(aot.contains("NexaFastListRowPrimitive("));
    assert!(aot.contains("NexaFastListDividerPrimitive()"));
    assert!(aot.contains("Modifier.fillMaxWidth().height(72.dp)"));
    assert!(aot.contains("horizontalAlignment = Alignment.CenterHorizontally"));
    assert!(aot.contains("NexaFastListSwipePrimitive("));
    assert!(keyed_aot.contains("key = { itemPosition ->"));
    assert!(dev_host.contains("internal fun NexaFastListRowPrimitive("));
    assert!(dev_host.contains("internal fun NexaFastListDividerPrimitive()"));
    assert!(dev_host.contains("internal fun NexaFastListSwipePrimitive("));
    assert!(renderer.contains("NexaFastListRowPrimitive("));
    assert!(renderer.contains("NexaFastListDividerPrimitive()"));
    assert!(renderer.contains("horizontalAlignment = if (rowNodes.length() > 1)"));
    assert!(renderer.contains("NexaFastListSwipePrimitive("));
    assert!(renderer.contains("val keyExpression = options?.opt(\"key\")"));
    assert!(
        renderer
            .contains("store.evaluate(expression, listRowLocals(itemIndex, sectionIndex), scope)")
    );
    assert!(renderer.contains("stableRowKey(itemIndex)"));
    assert!(renderer.contains("store.perform(swipeButtonActions, scope, rowLocals)"));

    let empty_release = KotlinBackend.generate(&empty_module(Vec::new()));
    assert!(!empty_release.contains("internal fun NexaFastListRowPrimitive("));
}

#[test]
fn ios_fast_list_uses_the_same_uikit_primitive_in_aot_and_devruntime() {
    let module = empty_module(vec![Node::FastList {
        plan: ListPlan::Count {
            count: Expr::Number {
                raw: "3".to_owned(),
                ty: NumericType::Int32,
            },
            common: ListCommon {
                axis: ListAxis::Vertical,
                native: false,
                reverse_layout: false,
                page_snap: false,
                item_extent: Some(64.0),
                index: "index".to_owned(),
                key: None,
                scroll_position: None,
                children: vec![Node::Text {
                    value: Expr::String("AOT and DevRuntime row".to_owned()),
                    style: nexa_ir::TextStyle::default(),
                }],
                on_end_reached: None,
                on_scroll: None,
                on_move: None,
                swipe_actions: None,
                sticky_header: None,
                refresh: None,
            },
        },
    }]);
    let aot = SwiftBackend.generate(&module);
    let keyed_native_aot = SwiftBackend.generate(&keyed_fast_list_module(true));
    let dev_host = SwiftBackend.generate_for_dev(&empty_module(Vec::new()));
    let (root, _) = fixture();
    let renderer = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS DevRuntime list renderer");

    assert!(aot.contains("NexaFastListPrimitive(rowCount:"));
    assert!(keyed_native_aot.contains("NexaIdentifiedListRow(id: AnyHashable("));
    assert!(
        dev_host.contains("func NexaFastListPrimitive<RowContent: View, HeaderContent: View>(")
    );
    assert!(renderer.contains("NexaFastListPrimitive("));
    assert!(renderer.contains("sectionCounts == nil, onMove == nil"));
    assert!(renderer.contains("let rowKey: ((Int, Int, Int) -> AnyHashable)?"));
    assert!(renderer.contains("ForEach(identifiedRows(count: count))"));
    assert!(renderer.contains("rowKey: rowKey.map"));
    for declaration in [
        "func NexaFastHorizontalListPrimitive<RowContent: View>(",
        "func NexaFastGridListPrimitive<RowContent: View>(",
        "func NexaFastSectionedListPrimitive<RowContent: View, HeaderContent: View>(",
        "struct NexaNativeListPrimitive<Content: View>",
    ] {
        assert!(
            dev_host.contains(declaration),
            "Dev host omitted {declaration}"
        );
    }
    for call in [
        "NexaFastHorizontalListPrimitive(",
        "NexaFastGridListPrimitive(",
        "NexaFastSectionedListPrimitive(",
        "NexaNativeListPrimitive {",
    ] {
        assert!(renderer.contains(call), "DevRuntime omitted {call}");
    }
    assert!(renderer.contains("listFields[\"swipe_actions\"]"));
    assert!(renderer.contains("row.swipeActions(edge: .trailing)"));

    let empty_release = SwiftBackend.generate(&empty_module(Vec::new()));
    assert!(!empty_release.contains("func NexaFastListPrimitive<"));

    let swipe_actions_aot = SwiftBackend.generate(&empty_module(vec![Node::FastList {
        plan: ListPlan::Count {
            count: Expr::Number {
                raw: "1".to_owned(),
                ty: NumericType::Int32,
            },
            common: ListCommon {
                axis: ListAxis::Vertical,
                native: true,
                reverse_layout: false,
                page_snap: false,
                item_extent: None,
                index: "index".to_owned(),
                key: None,
                scroll_position: None,
                children: vec![Node::Text {
                    value: Expr::String("Task".to_owned()),
                    style: nexa_ir::TextStyle::default(),
                }],
                on_end_reached: None,
                on_scroll: None,
                on_move: None,
                swipe_actions: Some(vec![Node::Button {
                    label: Expr::String("Delete".to_owned()),
                    icon: Some(nexa_ir::SystemIcon::Shared("delete".to_owned())),
                    loading: None,
                    disabled: None,
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    actions: Vec::new(),
                }]),
                sticky_header: None,
                refresh: None,
            },
        },
    }]));
    assert!(swipe_actions_aot.contains("NexaNativeListPrimitive {"));
    assert!(swipe_actions_aot.contains(".swipeActions(edge: .trailing)"));
}

#[test]
fn development_button_shape_defaults_match_on_both_platforms() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("return AnyView(NexaButtonPrimitive("));
    assert!(swift.contains("shape: shape,"));
    assert!(swift.contains("else { shape = .capsule }"));
    assert!(kotlin.contains("shape = buttonShape,"));
    let kotlin_primitives =
        fs::read_to_string(root.join("../../crates/nexa-backend-kotlin/src/generator/mod.rs"))
            .expect("read shared Kotlin button primitive");
    assert!(kotlin_primitives.contains(
        "val resolvedShape = shape ?: androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)"
    ));
}

#[test]
fn navigation_destinations_use_the_same_native_screen_shell_in_aot_and_devruntime() {
    let mut module = empty_module(vec![Node::NavigationStack {
        root: ScreenId(0),
        arguments: Vec::new(),
    }]);
    module.screens = vec![
        Screen {
            id: ScreenId(0),
            name: "Home".to_owned(),
            parameters: Vec::new(),
            states: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Home screen".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
            status_bar: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
        },
        Screen {
            id: ScreenId(1),
            name: "Task Details".to_owned(),
            parameters: Vec::new(),
            states: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Task details".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
            status_bar: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
        },
    ];
    module.states.push(nexa_ir::State {
        name: "canOpenDetails".to_owned(),
        ty: nexa_ir::Type::Bool,
        initial: Expr::Bool(true),
        mutable: true,
    });
    module.screens[0].body.push(Node::NavigationLink {
        destination: ScreenId(1),
        arguments: Vec::new(),
        guard: Some(Expr::State(
            "canOpenDetails".to_owned(),
            nexa_ir::Type::Bool,
        )),
        children: vec![Node::Text {
            value: Expr::String("Open details".to_owned()),
            style: nexa_ir::TextStyle::default(),
        }],
    });

    let swift = SwiftBackend.generate(&module);
    let kotlin = KotlinBackend.generate(&module);
    let (root, _) = fixture();
    let swift_runtime = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS DevRuntime navigation renderer");
    let kotlin_runtime =
        fs::read_to_string(root.join("../../runtime/android/NexaDevNavigation.kt"))
            .expect("read Android DevRuntime navigation renderer");
    let kotlin_renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android DevRuntime node renderer");

    assert!(swift.contains("struct NexaNavigationScreenPrimitive<Content: View>"));
    assert!(swift.contains("struct NexaNavigationTitleDisplayModePrimitive: ViewModifier"));
    assert!(swift.contains("NexaNavigationScreenPrimitive(title: \"Task Details\")"));
    assert!(swift.contains(".modifier(NexaNavigationTitleDisplayModePrimitive(large: true))"));
    assert!(swift.contains("struct NexaNavigationLinkPrimitive<Value: Hashable, Label: View>"));
    assert!(swift.contains("NexaNavigationLinkPrimitive(value:"));
    assert!(swift_runtime.contains("NexaNavigationScreenPrimitive(title: destination.screen)"));
    assert!(swift_runtime.contains("NexaNavigationTitleDisplayModePrimitive("));
    assert!(swift_runtime.contains("NexaNavigationLinkPrimitive(value: route, enabled: enabled)"));
    assert!(swift_runtime.contains("staticallyDisabled"));

    assert!(kotlin.contains("internal fun NexaNavigationScreenPrimitive("));
    assert!(kotlin.contains(
        "@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)\ninternal fun NexaNavigationScreenPrimitive("
    ));
    assert!(kotlin.contains("contentDescription = \"Back\""));
    assert!(kotlin.contains("NexaNavigationScreenPrimitive("));
    assert!(kotlin.contains("title = \"Task Details\""));
    assert!(kotlin.contains("internal fun NexaNavigationLinkPrimitive("));
    assert!(kotlin.contains("NexaNavigationLinkPrimitive("));
    assert!(kotlin_runtime.contains("NexaNavigationScreenPrimitive("));
    assert!(kotlin_runtime.contains("title = screen.optString(\"name\")"));
    assert!(kotlin_renderer.contains("NexaNavigationLinkPrimitive("));
    assert!(kotlin_renderer.contains("staticallyDisabled"));
}
