use nexa_ir::{LayoutKind, Module, ViewStyle};

mod accessibility;
mod assets;
mod bottom_bar;
mod colors;
mod components;
mod controls;
mod custom_components;
mod expressions;
mod features;
mod functions;
mod images;
mod imports;
mod input;
mod keyboard;
mod layout;
mod links;
mod lists;
mod navigation;
mod network;
mod permissions;
mod refresh;
mod runtime;
mod sheets;
mod state;
mod structs;
mod utils;

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
    let mut focus_bindings = features::collect_focus_bindings(&module.body);
    for screen in &module.screens {
        focus_bindings.extend(features::collect_focus_bindings(&screen.body));
    }
    let mut out = String::new();
    imports::render(
        &features,
        !module.screens.is_empty(),
        module.direction.is_some(),
        module.on_appear.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.on_appear.is_some()),
        module.on_disappear.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.on_disappear.is_some()),
        module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some(),
        &mut out,
    );
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
    out.push_str(&format!(
        "@Composable\nfun {}() {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    if features.uses_adaptive_color {
        out.push_str("    val nexaIsDarkTheme = isSystemInDarkTheme()\n");
    }
    render_status_bar(module.status_bar, features.uses_status_bar, 1, &mut out);
    if features.app_uses_link {
        out.push_str("    val nexaLinkContext = LocalContext.current\n");
    }
    if features.uses_network_api || features.uses_path_api || features.uses_permissions {
        out.push_str("    NexaRuntime.bind(LocalContext.current)\n");
        if features.uses_permission_request {
            out.push_str("    NexaRuntime.bindActivity(LocalContext.current)\n");
        }
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
        } else {
            out.push_str(&format!(
                "    val {name}: {} = {}\n",
                state.ty.kotlin(),
                expressions::expression(&state.initial)
            ));
        }
    }
    for screen in &module.screens {
        for state in &screen.states {
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
            } else {
                out.push_str(&format!(
                    "    val {name}: {} = {}\n",
                    state.ty.kotlin(),
                    expressions::expression(&state.initial)
                ));
            }
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
            let requester_name = crate::generator::input::focus_requester_name(binding);
            out.push_str(&format!(
                "    LaunchedEffect({state_name}) {{\n        if ({state_name}) {requester_name}.requestFocus() else {requester_name}.freeFocus()\n    }}\n"
            ));
        }
    }
    if !module.states.is_empty() {
        out.push('\n');
    }
    let body_depth = if let Some(direction) = module.direction {
        let direction = match direction.style {
            nexa_ir::DirectionStyle::Ltr => "Ltr",
            nexa_ir::DirectionStyle::Rtl => "Rtl",
        };
        out.push_str("    CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.");
        out.push_str(direction);
        out.push_str(") {\n");
        2
    } else {
        1
    };
    render_on_appear_effect(module.on_appear.as_deref(), body_depth, &mut out);
    render_on_disappear_effect(module.on_disappear.as_deref(), body_depth, &mut out);
    render_lifecycle_effect(module, body_depth, &mut out);
    if module.body.len() == 1 {
        components::render_node(&module.body[0], module, &features, body_depth, &mut out);
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
    if module.direction.is_some() {
        out.push_str("\n    }");
    }
    out.push_str("\n}\n");
    custom_components::render(module, &features, &mut out);
    if features.uses_network_api || features.uses_path_api || features.uses_permissions {
        runtime::render(&mut out, features.uses_permission_request);
    }
    if features.uses_native_library {
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
        assets::render(&mut out);
    }
    if features.uses_permissions {
        permissions::render(
            &mut out,
            features.uses_permission_request,
            &features.used_permissions,
            features.dynamic_permission,
        );
    }
    functions::render(module, &mut out);
    out
}

pub(super) fn render_on_appear_effect(
    actions: Option<&[nexa_ir::Action]>,
    depth: usize,
    out: &mut String,
) {
    let Some(actions) = actions else {
        return;
    };
    utils::indent(out, depth);
    out.push_str("LaunchedEffect(Unit) {");
    if actions.is_empty() {
        out.push_str("}\n");
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 1, out);
    utils::indent(out, depth);
    out.push_str("}\n");
}

pub(super) fn render_on_disappear_effect(
    actions: Option<&[nexa_ir::Action]>,
    depth: usize,
    out: &mut String,
) {
    let Some(actions) = actions else {
        return;
    };
    utils::indent(out, depth);
    out.push_str("DisposableEffect(Unit) {");
    if actions.is_empty() {
        out.push('\n');
        utils::indent(out, depth + 1);
        out.push_str("onDispose {}\n");
        utils::indent(out, depth);
        out.push_str("}\n");
        return;
    }
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str("onDispose {\n");
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push_str("}\n");
    utils::indent(out, depth);
    out.push_str("}\n");
}

fn render_lifecycle_effect(module: &Module, depth: usize, out: &mut String) {
    if module.on_active.is_none() && module.on_inactive.is_none() && module.on_background.is_none()
    {
        return;
    }
    let indent = "    ".repeat(depth);
    let nested = "    ".repeat(depth + 1);
    let deep = "    ".repeat(depth + 2);
    out.push_str(&format!(
        "{indent}val nexaLifecycleOwner = LocalLifecycleOwner.current\n"
    ));
    out.push_str(&format!(
        "{indent}DisposableEffect(nexaLifecycleOwner) {{\n"
    ));
    out.push_str(&format!(
        "{nested}val nexaLifecycleObserver = LifecycleEventObserver {{ _, event ->\n"
    ));
    out.push_str(&format!("{deep}when (event) {{\n"));
    render_lifecycle_case("ON_RESUME", module.on_active.as_deref(), depth + 3, out);
    render_lifecycle_case("ON_PAUSE", module.on_inactive.as_deref(), depth + 3, out);
    render_lifecycle_case("ON_STOP", module.on_background.as_deref(), depth + 3, out);
    out.push_str(&format!("{}else -> Unit\n", "    ".repeat(depth + 3)));
    out.push_str(&format!("{deep}}}\n"));
    out.push_str(&format!("{nested}}}\n"));
    out.push_str(&format!(
        "{nested}nexaLifecycleOwner.lifecycle.addObserver(nexaLifecycleObserver)\n"
    ));
    out.push_str(&format!(
        "{nested}onDispose {{ nexaLifecycleOwner.lifecycle.removeObserver(nexaLifecycleObserver) }}\n"
    ));
    out.push_str(&format!("{indent}}}\n"));
}

fn render_lifecycle_case(
    event: &str,
    actions: Option<&[nexa_ir::Action]>,
    depth: usize,
    out: &mut String,
) {
    let indent = "    ".repeat(depth);
    out.push_str(&format!("{indent}Lifecycle.Event.{event} -> {{\n"));
    if let Some(actions) = actions {
        if actions.is_empty() {
            out.push_str(&format!("{}Unit\n", "    ".repeat(depth + 1)));
        } else {
            controls::render_actions(actions, depth + 1, out);
        }
    } else {
        out.push_str(&format!("{}Unit\n", "    ".repeat(depth + 1)));
    }
    out.push_str(&format!("{indent}}}\n"));
}

pub(super) fn render_status_bar(
    config: Option<nexa_ir::StatusBarConfig>,
    enabled: bool,
    depth: usize,
    out: &mut String,
) {
    if !enabled {
        return;
    }
    let Some(config) = config else {
        return;
    };
    let indent = "    ".repeat(depth);
    let nested_indent = "    ".repeat(depth + 1);
    let deeply_nested_indent = "    ".repeat(depth + 2);
    out.push_str(&format!(
        "{indent}val nexaStatusBarView = LocalView.current\n"
    ));
    out.push_str(&format!("{indent}SideEffect {{\n"));
    out.push_str(&format!(
        "{nested_indent}val nexaWindow = (nexaStatusBarView.context as? Activity)?.window\n"
    ));
    out.push_str(&format!(
        "{nested_indent}nexaWindow?.let {{ nexaWindow ->\n{deeply_nested_indent}val nexaController = WindowCompat.getInsetsController(nexaWindow, nexaStatusBarView)\n"
    ));
    if config.hidden {
        out.push_str(&format!(
            "{deeply_nested_indent}nexaController.hide(WindowInsetsCompat.Type.statusBars())\n"
        ));
    } else {
        out.push_str(&format!(
            "{deeply_nested_indent}nexaController.show(WindowInsetsCompat.Type.statusBars())\n"
        ));
    }
    match config.style {
        nexa_ir::StatusBarStyle::Light => out.push_str(&format!(
            "{deeply_nested_indent}nexaController.isAppearanceLightStatusBars = false\n"
        )),
        nexa_ir::StatusBarStyle::Dark => out.push_str(&format!(
            "{deeply_nested_indent}nexaController.isAppearanceLightStatusBars = true\n"
        )),
        nexa_ir::StatusBarStyle::Default => {}
    }
    if let Some(nexa_ir::ColorValue::Static(color)) = config.background {
        let argb = (u32::from(color.alpha) << 24)
            | (u32::from(color.red) << 16)
            | (u32::from(color.green) << 8)
            | u32::from(color.blue);
        out.push_str(&format!(
            "{deeply_nested_indent}nexaWindow.statusBarColor = android.graphics.Color.parseColor(\"#{argb:08X}\")\n"
        ));
    }
    out.push_str(&format!("{nested_indent}}}\n{indent}}}\n"));
}
