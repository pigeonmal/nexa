use std::collections::BTreeSet;

use nexa_ir::{LayoutKind, Module, Node, State, ViewStyle, walk::walk_ir};

mod api;
mod components;
mod engine;

pub(super) use api::{network, permissions};
use components::components as component_renderer;
pub(super) use components::{
    accessibility, bottom_bar, controls, custom_components, images, input, keyboard, layout, links,
    list_runtime, lists, navigation, refresh, sheets,
};
pub(super) use engine::{colors, expressions, features, functions, imports, structs, utils};

pub(super) fn generate(module: &Module) -> String {
    let features = features::Features::analyze(module);
    let app_focus_bindings = collect_focus_bindings(&module.body);
    let mut all_focus_bindings = app_focus_bindings.clone();
    for screen in &module.screens {
        all_focus_bindings.extend(collect_focus_bindings(&screen.body));
    }
    let uses_fast_list = features.uses_fast_list;
    let mut out = imports::render(&features);
    if module_has_native_object_state(module) {
        out.push_str(
            "@MainActor\nprivate final class NexaNativeObjectStorage<Value>: ObservableObject {\n    @Published var value: Value\n\n    init(makeValue: () -> Value) {\n        value = makeValue()\n    }\n}\n\n",
        );
    }
    out.push_str("// nexa-unit:types\n");
    if uses_fast_list {
        out.push_str("\n@available(iOS 16.0, *)\n");
    }
    for declaration in &module.enums {
        out.push_str(&format!(
            "private enum {}: String, Error {{\n",
            nexa_codegen::names::enum_name(&declaration.name)
        ));
        for case in &declaration.cases {
            out.push_str(&format!("    case {case}\n"));
        }
        out.push_str("}\n\n");
    }
    structs::render(module, &mut out);
    if !module.screens.is_empty() {
        out.push_str("private enum NexaNavigationRoute: Hashable {\n");
        for screen in &module.screens {
            let case_name = nexa_codegen::names::navigation_case_name(screen.id);
            let mut payload_types = vec!["UUID".to_owned()];
            payload_types.extend(
                screen
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty.swift()),
            );
            out.push_str(&format!(
                "    case {case_name}({})\n",
                payload_types.join(", ")
            ));
        }
        out.push_str("}\n\n");
    }
    out.push_str("\n// nexa-unit:app\n");
    out.push_str(&format!(
        "public struct {}: View {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    if !module.screens.is_empty() {
        out.push_str("    private static let __nexaRootScreenIdentity = UUID()\n");
    }
    for state in &module.states {
        if state.is_native_class_constructor_binding() {
            render_native_object_state(state, 1, &mut out);
            continue;
        }
        if (!state.mutable && !state.is_native_class_instance_binding())
            || all_focus_bindings.contains(&state.name)
        {
            continue;
        }
        let name = nexa_codegen::names::state_name(&state.name);
        out.push_str(&format!(
            "    @State private var {name}: {} = {}\n",
            state.ty.swift(),
            expressions::expression(&state.initial)
        ));
    }
    for binding in &app_focus_bindings {
        out.push_str(&format!(
            "    @FocusState private var {}: Bool\n",
            nexa_codegen::names::state_name(binding),
        ));
    }
    if !module.states.is_empty() || !app_focus_bindings.is_empty() {
        out.push('\n');
    }
    if features.app_uses_adaptive_color {
        out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n");
    }
    if features.app_uses_size_class {
        out.push_str(
            "    @Environment(\\.horizontalSizeClass) private var nexaHorizontalSizeClass\n",
        );
        out.push_str("    @Environment(\\.verticalSizeClass) private var nexaVerticalSizeClass\n");
    }
    if module.on_active.is_some() || module.on_inactive.is_some() || module.on_background.is_some()
    {
        out.push_str("    @Environment(\\.scenePhase) private var nexaScenePhase\n");
    }
    if features.uses_navigation_back {
        out.push_str("    @Environment(\\.dismiss) private var nexaDismiss\n");
    }
    if features.app_uses_adaptive_color
        || features.app_uses_size_class
        || features.uses_navigation_back
        || module.on_active.is_some()
        || module.on_inactive.is_some()
        || module.on_background.is_some()
    {
        out.push('\n');
    }
    out.push_str("    public init() {}\n\n    public var body: some View {\n");
    if module.screens.is_empty() {
        render_immutable_state(&module.states, 2, &mut out);
    }
    if module.body.len() == 1 {
        component_renderer::render_node(&module.body[0], module, 2, &mut out);
    } else {
        layout::render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            &module.body,
            module,
            2,
            &mut out,
        );
    }
    render_direction_modifier(module.direction, 2, &mut out);
    render_on_appear_modifier(
        module.on_appear.as_deref(),
        module.on_appear_async,
        2,
        &mut out,
    );
    render_on_disappear_modifier(module.on_disappear.as_deref(), 2, &mut out);
    render_scene_phase_modifier(module, 2, &mut out);
    render_status_bar_modifiers(module.status_bar, 2, &mut out);
    out.push_str("\n    }\n");
    if !module.screens.is_empty() {
        for screen in &module.screens {
            navigation::render_screen_view(screen, module, &features, &mut out);
        }
    }
    out.push_str("}\n");
    out.push_str("\n// nexa-unit:components\n");
    custom_components::render(module, &features, &mut out);
    if uses_fast_list {
        out.push_str("\n// nexa-unit:list-runtime\n");
        list_runtime::render(
            &mut out,
            features.uses_sticky_header,
            features.uses_scroll_events,
            features.uses_vertical_list,
            features.uses_horizontal_list,
            features.uses_grid_list,
            features.uses_sectioned_list,
        );
    }
    if features.uses_native_library {
        out.push_str("\n// nexa-unit:native-library\n");
        network::render(
            &mut out,
            features.uses_network_api,
            features.uses_remote_image,
            features.uses_path_api,
            features.uses_file_api,
            features.uses_file_async,
        );
    }
    if features.uses_permissions {
        out.push_str("\n// nexa-unit:permissions\n");
        permissions::render(
            &mut out,
            features.uses_permission_request,
            &features.used_permissions,
            features.dynamic_permission,
        );
    }
    out.push_str("\n// nexa-unit:functions\n");
    functions::render(module, &mut out);
    out
}

fn collect_focus_bindings(nodes: &[Node]) -> BTreeSet<String> {
    let mut bindings = BTreeSet::new();
    walk_ir(
        nodes,
        &mut |node| {
            if let Node::TextInput {
                focused: Some(name),
                ..
            } = node
            {
                bindings.insert(name.clone());
            }
        },
        &mut |_| {},
    );
    bindings
}

fn render_direction_modifier(
    config: Option<nexa_ir::DirectionConfig>,
    depth: usize,
    out: &mut String,
) {
    let Some(config) = config else {
        return;
    };
    let direction = match config.style {
        nexa_ir::DirectionStyle::Ltr => "leftToRight",
        nexa_ir::DirectionStyle::Rtl => "rightToLeft",
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(&format!(".environment(\\.layoutDirection, .{direction})"));
}

pub(super) fn render_on_appear_modifier(
    actions: Option<&[nexa_ir::Action]>,
    asynchronous: bool,
    depth: usize,
    out: &mut String,
) {
    let Some(actions) = actions else {
        return;
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(if asynchronous {
        ".task {"
    } else {
        ".onAppear {"
    });
    if actions.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push('}');
}

pub(super) fn render_on_disappear_modifier(
    actions: Option<&[nexa_ir::Action]>,
    depth: usize,
    out: &mut String,
) {
    let Some(actions) = actions else {
        return;
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(".onDisappear {");
    if actions.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push('}');
}

fn render_scene_phase_modifier(module: &Module, depth: usize, out: &mut String) {
    if module.on_active.is_none() && module.on_inactive.is_none() && module.on_background.is_none()
    {
        return;
    }
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(".onChange(of: nexaScenePhase) { phase in\n");
    utils::indent(out, depth + 2);
    out.push_str("switch phase {\n");
    for (phase, actions) in [
        ("active", module.on_active.as_deref()),
        ("inactive", module.on_inactive.as_deref()),
        ("background", module.on_background.as_deref()),
    ] {
        utils::indent(out, depth + 3);
        out.push_str(&format!("case .{phase}:\n"));
        if let Some(actions) = actions {
            if actions.is_empty() {
                utils::indent(out, depth + 4);
                out.push_str("break\n");
            } else {
                controls::render_actions(actions, depth + 4, out);
            }
        } else {
            utils::indent(out, depth + 4);
            out.push_str("break\n");
        }
    }
    utils::indent(out, depth + 3);
    out.push_str("@unknown default:\n");
    utils::indent(out, depth + 4);
    out.push_str("break\n");
    utils::indent(out, depth + 2);
    out.push_str("}\n");
    utils::indent(out, depth + 1);
    out.push('}');
}

pub(super) fn render_status_bar_modifiers(
    config: Option<nexa_ir::StatusBarConfig>,
    depth: usize,
    out: &mut String,
) {
    let Some(config) = config else {
        return;
    };
    if config.hidden {
        out.push('\n');
        utils::indent(out, depth);
        out.push_str(".statusBarHidden(true)");
    }
    let scheme = match config.style {
        nexa_ir::StatusBarStyle::Default => None,
        // Light status-bar content uses a dark color scheme so the native
        // status bar selects light foreground content.
        nexa_ir::StatusBarStyle::Light => Some("dark"),
        nexa_ir::StatusBarStyle::Dark => Some("light"),
    };
    if let Some(scheme) = scheme {
        out.push('\n');
        utils::indent(out, depth);
        out.push_str(&format!(".preferredColorScheme(.{scheme})"));
    }
    if let Some(background) = config.background {
        out.push('\n');
        utils::indent(out, depth);
        out.push_str(&format!(
            ".background(alignment: .top) {{ GeometryReader {{ proxy in {}.frame(height: proxy.safeAreaInsets.top) }}.ignoresSafeArea(edges: .top) }}",
            colors::expression(background)
        ));
    }
}

pub(super) fn render_immutable_state(states: &[State], depth: usize, out: &mut String) {
    let immutable = states
        .iter()
        .filter(|state| !state.mutable && !state.is_native_class_instance_binding())
        .collect::<Vec<_>>();
    for state in immutable {
        utils::indent(out, depth);
        out.push_str(&format!(
            "let {}: {} = {}\n",
            nexa_codegen::names::state_name(&state.name),
            state.ty.swift(),
            expressions::expression(&state.initial)
        ));
    }
    if states
        .iter()
        .any(|state| !state.mutable && !state.is_native_class_instance_binding())
    {
        out.push('\n');
    }
}

fn module_has_native_object_state(module: &Module) -> bool {
    module
        .states
        .iter()
        .chain(
            module
                .screens
                .iter()
                .flat_map(|screen| screen.states.iter()),
        )
        .chain(
            module
                .components
                .iter()
                .flat_map(|component| component.states.iter()),
        )
        .any(State::is_native_class_constructor_binding)
}

pub(super) fn render_native_object_state(state: &State, depth: usize, out: &mut String) {
    if !state.is_native_class_constructor_binding() {
        return;
    }
    let name = nexa_codegen::names::state_name(&state.name);
    let storage = format!("__nexaNativeObjectStorage_{name}");
    utils::indent(out, depth);
    out.push_str(&format!(
        "@StateObject private var {storage} = NexaNativeObjectStorage {{ {} }}\n",
        expressions::expression(&state.initial)
    ));
    utils::indent(out, depth);
    out.push_str(&format!("private var {name}: {} {{\n", state.ty.swift()));
    utils::indent(out, depth + 1);
    out.push_str(&format!("get {{ {storage}.value }}\n"));
    if state.mutable {
        utils::indent(out, depth + 1);
        out.push_str(&format!(
            "nonmutating set {{ {storage}.value = newValue }}\n"
        ));
    }
    utils::indent(out, depth);
    out.push_str("}\n");
}

#[cfg(test)]
mod tests {
    use super::generate;
    use nexa_ir::{
        Action, Component, Expr, Function, Module, Node, NumericType, Screen, ScreenId, State,
        TextStyle, Type,
    };

    fn native_instance_state(name: &str) -> State {
        let ty = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        State {
            name: name.to_owned(),
            ty: ty.clone(),
            initial: Expr::Call {
                name: "Video.VideoPlayer".to_owned(),
                arguments: Vec::new(),
                return_type: ty,
                is_async: false,
                is_constructor: true,
            },
            mutable: false,
        }
    }

    #[test]
    fn app_native_class_instances_are_not_recreated_in_the_body() {
        let module = Module {
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: vec![native_instance_state("player")],
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "@StateObject private var __nexaNativeObjectStorage_{state_name} = NexaNativeObjectStorage {{ VideoPlayer() }}"
        )));
        assert!(swift.contains(&format!(
            "private var {state_name}: VideoPlayer {{\n        get {{ __nexaNativeObjectStorage_{state_name}.value }}"
        )));
        assert_eq!(
            swift.matches("VideoPlayer()").count(),
            1,
            "the constructor belongs only in the persistent @State initializer"
        );
    }

    #[test]
    fn mutable_native_class_bindings_write_through_identity_storage() {
        let mut player = native_instance_state("player");
        player.mutable = true;
        let module = Module {
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: vec![player],
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "nonmutating set {{ __nexaNativeObjectStorage_{state_name}.value = newValue }}"
        )));
    }

    #[test]
    fn screen_native_class_instances_use_swiftui_identity_storage() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let module = Module {
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: vec![
                native_instance_state("appPlayer"),
                State {
                    name: "sharedCount".to_owned(),
                    ty: Type::Numeric(NumericType::Int32),
                    initial: Expr::Number {
                        raw: "7".to_owned(),
                        ty: NumericType::Int32,
                    },
                    mutable: true,
                },
            ],
            screens: vec![
                Screen {
                    id: ScreenId(0),
                    name: "PlayerScreen".to_owned(),
                    parameters: Vec::new(),
                    states: vec![
                        State {
                            name: "player".to_owned(),
                            ty: player_type.clone(),
                            initial: Expr::Call {
                                name: "Video.VideoPlayer".to_owned(),
                                arguments: Vec::new(),
                                return_type: player_type.clone(),
                                is_async: false,
                                is_constructor: true,
                            },
                            mutable: false,
                        },
                        State {
                            name: "screenCount".to_owned(),
                            ty: Type::Numeric(NumericType::Int32),
                            initial: Expr::Number {
                                raw: "1".to_owned(),
                                ty: NumericType::Int32,
                            },
                            mutable: true,
                        },
                    ],
                    body: vec![Node::NavigationLink {
                        destination: ScreenId(1),
                        arguments: Vec::new(),
                        guard: None,
                        children: vec![Node::Text {
                            value: Expr::String("Push another instance".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Screen {
                    id: ScreenId(1),
                    name: "Details".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Button {
                        label: Expr::String("Play shared player".to_owned()),
                        icon: None,
                        loading: None,
                        disabled: None,
                        actions: vec![Action::Expression(Expr::NativeCall {
                            receiver: Some(Box::new(Expr::State(
                                "appPlayer".to_owned(),
                                player_type.clone(),
                            ))),
                            namespace: "Video".to_owned(),
                            name: "play".to_owned(),
                            arguments: Vec::new(),
                            return_type: Type::Void,
                            is_async: false,
                            is_throwing: false,
                        })],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        let screen_declaration = swift
            .find("private struct NexaScreen0: View")
            .expect("screen state must be owned by a destination view");
        let app_source = &swift[..screen_declaration];
        let screen_source = &swift[screen_declaration..];
        assert!(
            screen_source.contains(&format!(
                "@StateObject private var __nexaNativeObjectStorage_{state_name}"
            )),
            "screen-scoped native objects must be initialized once in route-owned SwiftUI storage"
        );
        assert!(!app_source.contains("__nexaNativeObjectStorage_nexa_player"));
        assert!(
            app_source
                .contains("@StateObject private var __nexaNativeObjectStorage_nexa_appPlayer")
        );
        assert!(screen_source.contains("private let nexa_appPlayer: VideoPlayer"));
        assert!(!screen_source.contains("@Binding private var nexa_appPlayer"));
        assert_eq!(screen_source.matches("VideoPlayer()").count(), 1);
        assert!(app_source.contains("@State private var nexa_sharedCount: Int32 = 7"));
        assert!(screen_source.contains("@Binding private var nexa_sharedCount: Int32"));
        assert!(screen_source.contains("@State private var nexa_screenCount: Int32 = 1"));
        assert!(swift.contains("case screen0(UUID)"));
        assert!(swift.contains("case screen1(UUID)"));
        assert!(swift.contains("NexaNavigationRoute.screen1(UUID())"));
        assert!(swift.contains("case let .screen0(routeIdentity):"));
        assert!(swift.contains("case let .screen1(routeIdentity):"));
        assert!(swift.contains("nexa_appPlayer.play()"));
        assert!(swift.contains(".id(routeIdentity)"));
    }

    #[test]
    fn component_native_class_instances_use_swiftui_identity_storage() {
        let module = Module {
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: vec![Component {
                name: "PlayerPanel".to_owned(),
                source_file: None,
                parameters: Vec::new(),
                states: vec![native_instance_state("player")],
                body: Vec::new(),
            }],
            body: vec![Node::ComponentCall {
                name: "PlayerPanel".to_owned(),
                arguments: Vec::new(),
                children: None,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "@StateObject private var __nexaNativeObjectStorage_{state_name} = NexaNativeObjectStorage {{ VideoPlayer() }}"
        )));
    }

    #[test]
    fn generates_result_and_try_in_swift() {
        let err_type = Type::Enum("AppError".to_owned());
        let module = Module {
            app_name: "ResultApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: vec![nexa_ir::EnumDecl {
                name: "AppError".to_owned(),
                cases: vec!["NotFound".to_owned(), "Unauthorized".to_owned()],
            }],
            structs: Vec::new(),
            functions: vec![
                Function {
                    name: "fetchCode".to_owned(),
                    is_async: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::ResultOk {
                        value: Box::new(Expr::Number {
                            raw: "42".to_owned(),
                            ty: NumericType::Int32,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
                Function {
                    name: "compute".to_owned(),
                    is_async: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::Try {
                        expr: Box::new(Expr::Call {
                            name: "fetchCode".to_owned(),
                            arguments: Vec::new(),
                            return_type: Type::Result(
                                Box::new(Type::Numeric(NumericType::Int32)),
                                Box::new(err_type.clone()),
                            ),
                            is_async: false,
                            is_constructor: false,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
            ],
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("private enum NexaAppError: String, Error {"));
        assert!(swift.contains("Result<Int32, NexaAppError>"));
        assert!(swift.contains(".success(42)"));
        assert!(swift.contains("try nexa_fn_fetchCode().get()"));
    }
}
