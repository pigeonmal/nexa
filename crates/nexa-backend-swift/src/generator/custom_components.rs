use nexa_ir::{Component, LayoutKind, Module, Node, ViewStyle};

use super::{components::render_node, features::Features, layout, render_immutable_state};

pub(super) fn render(module: &Module, features: &Features, out: &mut String) {
    let needs_ios16 = features.uses_fast_list;
    for component in &module.components {
        render_component(component, module, features, needs_ios16, out);
    }
}

fn render_component(
    component: &Component,
    module: &Module,
    features: &Features,
    needs_ios16: bool,
    out: &mut String,
) {
    let name = nexa_codegen::names::component_name(&component.name);
    if needs_ios16 {
        out.push_str("\n@available(iOS 16.0, *)\n");
    } else {
        out.push('\n');
    }
    out.push_str(&format!("private struct {name}: View {{\n"));

    for parameter in &component.parameters {
        out.push_str(&format!(
            "    private let {}: {}\n",
            nexa_codegen::names::state_name(&parameter.name),
            parameter.ty.swift()
        ));
    }
    for state in &component.states {
        if state.mutable {
            out.push_str(&format!(
                "    @State private var {}: {} = {}\n",
                nexa_codegen::names::state_name(&state.name),
                state.ty.swift(),
                super::expressions::expression(&state.initial)
            ));
        }
    }
    if features.component_uses_adaptive_color(&component.name) {
        out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n");
    }

    if !component.parameters.is_empty() || !component.states.is_empty() {
        out.push('\n');
    }
    out.push_str("    init(");
    out.push_str(
        &component
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "_ {}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    parameter.ty.swift()
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    if component.parameters.is_empty() {
        out.push_str(") {}\n\n");
    } else {
        out.push_str(") {\n");
        for parameter in &component.parameters {
            let name = nexa_codegen::names::state_name(&parameter.name);
            out.push_str(&format!("        self.{name} = {name}\n"));
        }
        out.push_str("    }\n\n");
    }

    out.push_str("    var body: some View {\n");
    render_immutable_state(&component.states, 2, out);
    render_body(&component.body, module, 2, out);
    out.push_str("\n    }\n}\n");
}

fn render_body(body: &[Node], module: &Module, depth: usize, out: &mut String) {
    match body {
        [node] => render_node(node, module, depth, out),
        children => layout::render_layout(
            LayoutKind::View,
            0.0,
            &ViewStyle::default(),
            children,
            module,
            depth,
            out,
        ),
    }
}
