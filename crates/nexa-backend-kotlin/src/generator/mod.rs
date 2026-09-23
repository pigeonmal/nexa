use nexa_ir::{LayoutKind, Module, ViewStyle};

mod api;
mod components;
mod engine;

pub(super) use api::{network, permissions};
use components::components as component_renderer;
pub(super) use components::{
    accessibility, assets, bottom_bar, controls, custom_components, images, input, keyboard,
    layout, links, lists, navigation, refresh, sheets,
};
pub(super) use engine::{colors, expressions, features, functions, runtime, state, structs, utils};

fn project_features_from_analysis(
    module: &Module,
    features: &features::Features,
) -> crate::KotlinProjectFeatures {
    crate::KotlinProjectFeatures {
        uses_network: features.uses_network_transport(),
        uses_remote_image: features.uses_remote_image,
        uses_coroutines: features.uses_network_transport()
            || features.uses_file_async
            || features.uses_permission_request,
        uses_permission_request: features.uses_permission_request,
        uses_navigation: !module.screens.is_empty(),
        uses_compose_graphics: features.uses_color
            || features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder,
        uses_lifecycle_events: module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some(),
    }
}

pub(super) fn generate(module: &Module) -> String {
    let features = features::Features::analyze(module);
    generate_with_analysis(module, &features)
}

pub(super) fn generate_with_project_features(
    module: &Module,
) -> (String, crate::KotlinProjectFeatures) {
    let features = features::Features::analyze(module);
    let project_features = project_features_from_analysis(module, &features);
    let generated = generate_with_analysis(module, &features);
    (generated, project_features)
}

fn generate_with_analysis(module: &Module, features: &features::Features) -> String {
    let focus_bindings = features::collect_focus_bindings(&module.body);
    let mut out = String::new();
    out.push_str(&engine::imports::render(engine::imports::ImportContext {
        features: &features,
        has_navigation: !module.screens.is_empty(),
        has_direction: module.direction.is_some(),
        has_on_appear: module.on_appear.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.on_appear.is_some()),
        has_on_disappear: module.on_disappear.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.on_disappear.is_some()),
        has_lifecycle_events: module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some(),
    }));
    out.push_str("// nexa-unit:types\n");
    for declaration in &module.enums {
        out.push_str(&format!(
            "private enum class {} {{ {} }}\n\n",
            nexa_codegen::names::enum_name(&declaration.name),
            declaration.cases.join(", ")
        ));
    }
    structs::render(module, &mut out);
    if features.uses_bottom_sheet {
        out.push_str("@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)\n");
    }
    if features.app_uses_keyboard_interactive {
        out.push_str("@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)\n");
    }
    if features.uses_sticky_header {
        out.push_str("@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)\n");
    }
    out.push_str("// nexa-unit:app\n");
    out.push_str(&format!(
        "@Composable\nfun {}() {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    if features.uses_adaptive_color {
        out.push_str("    val nexaIsDarkTheme = isSystemInDarkTheme()\n");
    }
    components::status_bar::render(module.status_bar, features.uses_status_bar, 1, &mut out);
    if features.app_uses_link {
        out.push_str("    val nexaLinkContext = LocalContext.current\n");
    }
    if features.uses_permission_request {
        out.push_str(
            "    val nexaPermissionLauncher = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { result ->\n        NexaRuntime.dispatchPermissionResult(result)\n    }\n    NexaRuntime.bindPermissionLauncher(nexaPermissionLauncher)\n",
        );
    }
    if features.uses_network_api || features.uses_path_api || features.uses_permissions {
        out.push_str("    NexaRuntime.bind(LocalContext.current)\n");
    }
    if features.app_uses_haptic {
        out.push_str("    val nexaHapticView = LocalView.current\n");
    }
    for state in &module.states {
        let name = nexa_codegen::names::state_name(&state.name);
        if state.mutable {
            if state::is_mutable_collection(state) {
                out.push_str(&format!(
                    "    val {name} = remember {{ {} }}\n",
                    state::kotlin_state_initializer(state)
                ));
            } else {
                out.push_str(&format!(
                    "    var {name} by remember {{ {} }}\n",
                    state::kotlin_state_initializer(state)
                ));
            }
        } else if state.is_native_class_instance_binding() {
            out.push_str(&format!(
                "    val {name}: {} = remember {{ {} }}\n",
                state.ty.kotlin(),
                expressions::expression(&state.initial)
            ));
        } else {
            out.push_str(&format!(
                "    val {name}: {} = {}\n",
                state.ty.kotlin(),
                expressions::expression(&state.initial)
            ));
        }
    }
    if !focus_bindings.is_empty() {
        for binding in &focus_bindings {
            out.push_str(&format!(
                "    val {} = remember {{ FocusRequester() }}\n",
                input::focus_requester_name(binding)
            ));
        }
        for binding in &focus_bindings {
            let state_name = nexa_codegen::names::state_name(binding);
            let requester_name = input::focus_requester_name(binding);
            out.push_str(&format!(
                "    LaunchedEffect({state_name}) {{\n        if ({state_name}) {requester_name}.requestFocus() else {requester_name}.freeFocus()\n    }}\n"
            ));
        }
    }
    if !module.states.is_empty() {
        out.push('\n');
    }
    let body_depth = components::direction::start(module.direction, &mut out);
    components::lifecycle::render_on_appear(module.on_appear.as_deref(), body_depth, &mut out);
    components::lifecycle::render_on_disappear(
        module.on_disappear.as_deref(),
        body_depth,
        &mut out,
    );
    components::lifecycle::render_app(module, body_depth, &mut out);
    if module.body.len() == 1 {
        component_renderer::render_node(&module.body[0], module, &features, body_depth, &mut out);
    } else {
        layout::render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            &module.body,
            module,
            &features,
            body_depth,
            &mut out,
        );
    }
    components::direction::end(module.direction, &mut out);
    out.push_str("\n}\n");
    out.push_str("// nexa-unit:components\n");
    custom_components::render(module, &features, &mut out);
    if features.uses_network_api || features.uses_path_api || features.uses_permissions {
        out.push_str("// nexa-unit:runtime\n");
        runtime::render(&mut out, features.uses_permission_request);
    }
    if features.uses_native_library {
        out.push_str("// nexa-unit:native-library\n");
        network::render(
            &mut out,
            features.uses_network_api,
            features.uses_remote_image,
            features.uses_path_api,
            features.uses_file_api,
            features.uses_file_async,
        );
    }
    if features.uses_asset
        || features.uses_tab_icon
        || features.uses_button_icon
        || features.uses_placeholder
    {
        out.push_str("// nexa-unit:assets\n");
        assets::render(&mut out);
    }
    if features.uses_permissions {
        out.push_str("// nexa-unit:permissions\n");
        permissions::render(
            &mut out,
            features.uses_permission_request,
            &features.used_permissions,
            features.dynamic_permission,
        );
    }
    out.push_str("// nexa-unit:functions\n");
    functions::render(module, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::generate;
    use nexa_ir::{
        Action, Component, Expr, Module, Node, NumericType, Screen, ScreenId, State, TextStyle,
        Type,
    };

    #[test]
    fn screen_native_class_instances_are_remembered_across_recomposition() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let native_instance_state = |name: &str| State {
            name: name.to_owned(),
            ty: player_type.clone(),
            initial: Expr::Call {
                name: "Video.VideoPlayer".to_owned(),
                arguments: Vec::new(),
                return_type: player_type.clone(),
                is_async: false,
                is_constructor: true,
            },
            mutable: false,
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
                        native_instance_state("player"),
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

        let kotlin = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        let app_state_name = nexa_codegen::names::state_name("appPlayer");
        let route_start = kotlin
            .find("composable(route = \"nexa_screen_0\")")
            .expect("screen body must be emitted inside its navigation entry");
        let screen_player = kotlin
            .find(&format!(
                "val {state_name}: VideoPlayer = remember {{ VideoPlayer() }}"
            ))
            .expect("screen native object must be remembered");
        let screen_count = kotlin
            .find("var nexa_screenCount by remember { mutableIntStateOf(1) }")
            .expect("screen-local mutable state must be remembered");
        let app_state = kotlin
            .find("var nexa_sharedCount by remember { mutableIntStateOf(7) }")
            .expect("app state must remain shared above the navigation host");
        let nav_host = kotlin.find("NavHost(").expect("navigation host exists");
        assert!(route_start < screen_player && screen_player < screen_count);
        assert!(screen_count > nav_host);
        assert!(app_state < nav_host);
        assert!(!kotlin[..route_start].contains("nexa_screenCount"));
        assert!(kotlin.contains(&format!(
            "val {app_state_name}: VideoPlayer = remember {{ VideoPlayer() }}"
        )));
        assert!(kotlin.contains("composable(route = \"nexa_screen_1\")"));
        assert!(kotlin.contains("nexa_appPlayer.play()"));
    }

    #[test]
    fn component_native_class_instances_are_remembered_across_recomposition() {
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
            states: Vec::new(),
            screens: Vec::new(),
            components: vec![Component {
                name: "PlayerPanel".to_owned(),
                source_file: None,
                parameters: Vec::new(),
                states: vec![State {
                    name: "player".to_owned(),
                    ty: player_type.clone(),
                    initial: Expr::Call {
                        name: "Video.VideoPlayer".to_owned(),
                        arguments: Vec::new(),
                        return_type: player_type,
                        is_async: false,
                        is_constructor: true,
                    },
                    mutable: false,
                }],
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

        let kotlin = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(kotlin.contains(&format!(
            "val {state_name}: VideoPlayer = remember {{ VideoPlayer() }}"
        )));
    }
}
