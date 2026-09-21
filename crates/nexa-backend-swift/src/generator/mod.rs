use nexa_ir::{LayoutKind, Module, State, ViewStyle};

mod colors;
mod components;
mod controls;
mod expressions;
mod features;
mod images;
mod input;
mod keyboard;
mod layout;
mod list_runtime;
mod lists;
mod navigation;
mod utils;

pub(super) fn generate(module: &Module) -> String {
    let uses_fast_list = features::uses_fast_list(module);
    let uses_adaptive_color = features::uses_adaptive_color(module);
    let mut out = if uses_fast_list {
        String::from("import SwiftUI\nimport UIKit\n\n@available(iOS 16.0, *)\n")
    } else {
        String::from("import SwiftUI\n\n")
    };
    out.push_str(&format!(
        "public struct {}: View {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    for state in &module.states {
        if !state.mutable {
            continue;
        }
        let name = nexa_codegen::names::state_name(&state.name);
        out.push_str(&format!(
            "    @State private var {name}: {} = {}\n",
            state.ty.swift(),
            expressions::expression(&state.initial)
        ));
    }
    if !module.screens.is_empty() {
        out.push_str("\n    private enum NexaNavigationRoute: Hashable {\n");
        for screen in &module.screens {
            out.push_str(&format!(
                "        case {}\n",
                nexa_codegen::names::navigation_case_name(screen.id)
            ));
        }
        out.push_str("    }\n");
    }
    if !module.states.is_empty() {
        out.push('\n');
    }
    if uses_adaptive_color {
        out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n\n");
    }
    out.push_str("    public init() {}\n\n    public var body: some View {\n");
    if module.screens.is_empty() {
        render_immutable_state(&module.states, 2, &mut out);
    }
    if module.body.len() == 1 {
        components::render_node(&module.body[0], module, 2, &mut out);
    } else {
        layout::render_layout(
            LayoutKind::View,
            0.0,
            &ViewStyle::default(),
            &module.body,
            module,
            2,
            &mut out,
        );
    }
    out.push_str("\n    }\n");
    if !module.screens.is_empty() {
        for screen in &module.screens {
            navigation::render_screen_function(screen, module, 1, &mut out);
        }
    }
    out.push_str("}\n");
    if uses_fast_list {
        list_runtime::render(&mut out);
    }
    out
}

fn render_immutable_state(states: &[State], depth: usize, out: &mut String) {
    let immutable = states
        .iter()
        .filter(|state| !state.mutable)
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
    if states.iter().any(|state| !state.mutable) {
        out.push('\n');
    }
}
