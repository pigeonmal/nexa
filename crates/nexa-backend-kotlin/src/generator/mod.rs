use nexa_ir::{LayoutKind, Module, ViewStyle};

mod accessibility;
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
mod refresh;
mod sheets;
mod state;
mod structs;
mod utils;

pub(super) fn generate(module: &Module) -> String {
    let features = features::Features::analyze(module);
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
    out.push_str(&format!(
        "@Composable\nfun {}() {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    if features.uses_adaptive_color {
        out.push_str("    val nexaIsDarkTheme = isSystemInDarkTheme()\n");
    }
    render_status_bar(module.status_bar, features.uses_status_bar, &mut out);
    if features.app_uses_link {
        out.push_str("    val nexaLinkContext = LocalContext.current\n");
    }
    if features.uses_native_library {
        out.push_str("    NexaRuntime.bind(LocalContext.current.applicationContext)\n");
    }
    for state in &module.states {
        let name = nexa_codegen::names::state_name(&state.name);
        if state.mutable {
            out.push_str(&format!(
                "    var {name} by remember {{ {} }}\n",
                state::kotlin_state_initializer(state)
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
    if features.uses_native_library {
        network::render(&mut out);
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

fn render_status_bar(config: Option<nexa_ir::StatusBarConfig>, enabled: bool, out: &mut String) {
    if !enabled {
        return;
    }
    let Some(config) = config else {
        return;
    };
    out.push_str("    val nexaStatusBarView = LocalView.current\n");
    out.push_str("    SideEffect {\n");
    out.push_str("        val nexaWindow = (nexaStatusBarView.context as? Activity)?.window\n");
    out.push_str(
        "        nexaWindow?.decorView?.let { nexaDecorView ->\n            var nexaFlags = nexaDecorView.systemUiVisibility\n",
    );
    if config.hidden {
        out.push_str("            nexaFlags = nexaFlags or View.SYSTEM_UI_FLAG_FULLSCREEN\n");
    } else {
        out.push_str(
            "            nexaFlags = nexaFlags and View.SYSTEM_UI_FLAG_FULLSCREEN.inv()\n",
        );
    }
    match config.style {
        nexa_ir::StatusBarStyle::Light => out.push_str(
            "            nexaFlags = nexaFlags and View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR.inv()\n",
        ),
        nexa_ir::StatusBarStyle::Dark => out.push_str(
            "            nexaFlags = nexaFlags or View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR\n",
        ),
        nexa_ir::StatusBarStyle::Default => {}
    }
    out.push_str("            nexaDecorView.systemUiVisibility = nexaFlags\n        }\n    }\n\n");
}
