use nexa_ir::{LayoutKind, Module, ViewStyle};

mod colors;
mod components;
mod controls;
mod custom_components;
mod expressions;
mod features;
mod images;
mod imports;
mod input;
mod keyboard;
mod layout;
mod lists;
mod navigation;
mod state;
mod utils;

pub(super) fn generate(module: &Module) -> String {
    let features = features::Features::analyze(module);
    let mut out = String::new();
    imports::render(&features, !module.screens.is_empty(), &mut out);
    out.push_str(&format!(
        "@Composable\nfun {}() {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    if features.uses_adaptive_color {
        out.push_str("    val nexaIsDarkTheme = isSystemInDarkTheme()\n");
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
    if !module.states.is_empty() {
        out.push('\n');
    }
    if module.body.len() == 1 {
        components::render_node(&module.body[0], module, &features, 1, &mut out);
    } else {
        layout::render_layout(
            LayoutKind::View,
            0.0,
            &ViewStyle::default(),
            &module.body,
            module,
            &features,
            1,
            &mut out,
        );
    }
    out.push_str("\n}\n");
    custom_components::render(module, &features, &mut out);
    out
}
