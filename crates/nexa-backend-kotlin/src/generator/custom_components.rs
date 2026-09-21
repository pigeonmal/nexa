use nexa_ir::{Component, LayoutKind, Module, Node, ViewStyle};

use super::{components::render_node, features::Features, layout, state, utils::indent};

pub(super) fn render(module: &Module, features: &Features, out: &mut String) {
    for component in &module.components {
        render_component(component, module, features, out);
    }
}

fn render_component(component: &Component, module: &Module, features: &Features, out: &mut String) {
    out.push_str(&format!(
        "\n@Composable\nprivate fun {}(",
        nexa_codegen::names::component_name(&component.name)
    ));
    out.push_str(
        &component
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    parameter.ty.kotlin()
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    let needs_system_theme = features.component_requires_system_theme(&component.name);
    if needs_system_theme && !component.parameters.is_empty() {
        out.push_str(", ");
    }
    if needs_system_theme {
        out.push_str("nexaIsDarkTheme: Boolean");
    }
    out.push_str(") {\n");

    render_component_states(&component.states, 1, out);
    render_body(&component.body, module, features, 1, out);
    out.push_str("\n}\n");
}

fn render_component_states(states: &[nexa_ir::State], depth: usize, out: &mut String) {
    for state in states {
        indent(out, depth);
        let name = nexa_codegen::names::state_name(&state.name);
        if state.mutable {
            out.push_str(&format!(
                "var {name} by remember {{ {} }}\n",
                state::kotlin_state_initializer(state)
            ));
        } else {
            out.push_str(&format!(
                "val {name}: {} = {}\n",
                state.ty.kotlin(),
                super::expressions::expression(&state.initial)
            ));
        }
    }
    if !states.is_empty() {
        out.push('\n');
    }
}

fn render_body(
    body: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    match body {
        [] => {}
        [node] => render_node(node, module, features, depth, out),
        children => layout::render_layout(
            LayoutKind::View,
            0.0,
            &ViewStyle::default(),
            children,
            module,
            features,
            depth,
            out,
        ),
    }
}
