use nexa_codegen::{GeneratedSources, SourceUnit, SourceUnits};
use nexa_ir::{LayoutKind, Module, State, ViewStyle};

mod api;
mod components;
mod engine;

pub(super) use api::{network, permissions};
use components::components as component_renderer;
pub(super) use components::{
    accessibility, bottom_bar, controls, custom_components, direction, images, input, keyboard,
    layout, lifecycle, links, list_runtime, lists, navigation, refresh, sheets, status_bar,
};
pub(super) use engine::state::{render_immutable_state, render_native_object_state};
use engine::types::{self, swift_type};
pub(super) use engine::{colors, expressions, features, functions, imports, structs, utils};

/// Generates the release source as one concatenated string.
///
/// Prefer [`generate_units`]: the CLI writes each unit as its own file, and
/// this form exists for callers that only inspect the text.
pub(super) fn generate(module: &Module) -> String {
    join_units(generate_units(module).into_files(&[], ""))
}

/// Generates the release source as separate compile units.
///
/// Returns the pieces rather than finished files so the caller can merge extra
/// imports -- plugin bindings add imports that every file must see -- before
/// the files are assembled.
pub(super) fn generate_units(module: &Module) -> GeneratedSources {
    let features = features::Features::analyze(module);
    generate_with_analysis(module, features)
}

fn generate_with_analysis(module: &Module, features: features::Features) -> GeneratedSources {
    let app_focus_bindings = features.facts.focus_bindings.app.clone();
    let mut all_focus_bindings = app_focus_bindings.clone();
    for bindings in features.facts.focus_bindings.screens.values() {
        all_focus_bindings.extend(bindings.iter().cloned());
    }
    let uses_fast_list = features.uses_fast_list;
    // Each unit becomes its own file, so every file needs the full import
    // block and the shared native-object storage helper.
    let mut preamble = String::new();
    if module_has_native_object_state(module) {
        preamble.push_str(
            "@MainActor\nprivate final class NexaNativeObjectStorage<Value>: ObservableObject {\n    @Published var value: Value\n\n    init(makeValue: () -> Value) {\n        value = makeValue()\n    }\n}\n\n",
        );
    }
    let mut units = SourceUnits::new("swift");
    units.set_imports(&imports::render(&features));
    units.set_preamble(&preamble);
    units.write("types", |out| {
        types::render_enums(module, out);
        structs::render(module, out);
        types::render_navigation_routes(module, out);
    });

    units.write("app", |out| {
        out.push_str(&format!(
            "public struct {}: View {{\n",
            nexa_codegen::names::screen_name(&module.app_name)
        ));
        if !module.screens.is_empty() {
            out.push_str("    private static let __nexaRootScreenIdentity = UUID()\n");
            out.push_str("    @State private var __nexaNavigationPath = NavigationPath()\n");
        }
        for state in &module.states {
            if state.is_native_class_constructor_binding() {
                render_native_object_state(state, 1, out);
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
                swift_type(&state.ty),
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
            out.push_str(
                "    @Environment(\\.verticalSizeClass) private var nexaVerticalSizeClass\n",
            );
        }
        if module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some()
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
            render_immutable_state(&module.states, 2, out);
        }
        if module.body.len() == 1 {
            component_renderer::render_node(&module.body[0], module, &features, 2, out);
        } else {
            layout::render_layout(
                LayoutKind::Column,
                0.0,
                &ViewStyle::default(),
                &module.body,
                module,
                &features,
                2,
                out,
            );
        }
        direction::render(module.direction, 2, out);
        lifecycle::render_on_appear(module.on_appear.as_deref(), module.on_appear_async, 2, out);
        lifecycle::render_on_disappear(module.on_disappear.as_deref(), 2, out);
        lifecycle::render_scene_phase(module, 2, out);
        status_bar::render(module.status_bar, 2, out);
        out.push_str("\n    }\n");
        if !module.screens.is_empty() {
            for screen in &module.screens {
                navigation::render_screen_view(screen, module, &features, out);
            }
        }
        out.push_str("}\n");
    });

    units.write("components", |out| {
        custom_components::render(module, &features, out);
    });
    if uses_fast_list {
        units.write("list-runtime", |out| {
            list_runtime::render(
                out,
                features.uses_sticky_header,
                features.uses_scroll_events,
                features.uses_vertical_list,
                features.uses_horizontal_list,
                features.uses_grid_list,
                features.uses_sectioned_list,
            );
        });
    }
    if features.uses_native_library {
        units.write("native-library", |out| {
            network::render(
                out,
                features.uses_network_api,
                features.uses_remote_image,
                features.uses_path_api,
                features.uses_file_api,
                features.uses_file_async,
            );
        });
    }
    if features.uses_permissions {
        units.write("permissions", |out| {
            permissions::render(
                out,
                features.uses_permission_request,
                &features.used_permissions,
                features.dynamic_permission,
                features.expose_permissions_to_dev_runtime,
            );
        });
    }
    units.write("functions", |out| {
        functions::render(module, out);
    });
    units.finish()
}

/// Concatenates units into a single source string.
fn join_units(units: Vec<SourceUnit>) -> String {
    let mut source = String::new();
    for unit in units {
        source.push_str(&unit.contents);
    }
    source
}

/// Generates the development host source as one concatenated string.
pub(super) fn generate_for_dev(module: &Module) -> String {
    join_units(generate_for_dev_units(module).into_files(&[], ""))
}

/// Generates the development host source as separate compile units.
pub(super) fn generate_for_dev_units(module: &Module) -> GeneratedSources {
    let mut features = features::Features::analyze(module);
    // Calls to Nexa's async native APIs can appear after the dev host has been
    // built. Keep the same URLSession adapter as release output in that host.
    features.uses_network_api = true;
    // File and Path calls can be introduced by a hot reload after the first
    // native build, so keep the same first-party helpers available in dev.
    features.uses_path_api = true;
    features.uses_file_api = true;
    features.uses_file_async = true;
    features.uses_permissions = true;
    features.uses_permission_request = true;
    features.dynamic_permission = true;
    features.used_permissions = [
        nexa_ir::Permission::Camera,
        nexa_ir::Permission::Microphone,
        nexa_ir::Permission::Photos,
        nexa_ir::Permission::Location,
        nexa_ir::Permission::Notifications,
        nexa_ir::Permission::Contacts,
        nexa_ir::Permission::Calendar,
        nexa_ir::Permission::Bluetooth,
    ]
    .into_iter()
    .collect();
    features.expose_permissions_to_dev_runtime = true;
    features.uses_native_library = true;
    generate_with_analysis(module, features)
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
        // Top-level `private` is lowered because each unit compiles as its own
        // file, so the screen declaration is internal here.
        let screen_declaration = swift
            .find("struct NexaScreen0: View")
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
        assert!(swift.contains("enum NexaAppError: String, Error {"));
        assert!(swift.contains("Result<Int32, NexaAppError>"));
        assert!(swift.contains(".success(42)"));
        assert!(swift.contains("try nexa_fn_fetchCode().get()"));
    }
}
