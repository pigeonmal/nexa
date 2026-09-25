use nexa_ir::{Component, LayoutKind, Module, Node, ViewStyle, walk::walk_ir};

use crate::generator::{
    components::render_node, features::Features, layout, render_immutable_state,
    render_native_object_state,
};

pub(crate) fn render(module: &Module, features: &Features, out: &mut String) {
    for component in &module.components {
        render_component(component, module, features, out);
    }
}

fn render_component(component: &Component, module: &Module, features: &Features, out: &mut String) {
    let name = nexa_codegen::names::component_name(&component.name);
    let focus_bindings = features
        .facts
        .focus_bindings
        .components
        .get(&component.name)
        .cloned()
        .unwrap_or_default();
    let has_content_slot = component_has_content_slot(component);
    let needs_ios16 = component_uses_fast_list(component);
    if needs_ios16 {
        out.push_str("\n@available(iOS 16.0, *)\n");
    } else {
        out.push('\n');
    }
    if has_content_slot {
        out.push_str(&format!(
            "private struct {name}<SlotContent: View>: View {{\n"
        ));
    } else {
        out.push_str(&format!("private struct {name}: View {{\n"));
    }

    for parameter in &component.parameters {
        out.push_str(&format!(
            "    private let {}: {}\n",
            nexa_codegen::names::state_name(&parameter.name),
            parameter.ty.swift()
        ));
    }
    if has_content_slot {
        out.push_str("    private let nexaContent: () -> SlotContent\n");
    }
    for state in &component.states {
        if state.is_native_class_constructor_binding() {
            render_native_object_state(state, 1, out);
            continue;
        }
        if state.mutable && !focus_bindings.contains(&state.name) {
            out.push_str(&format!(
                "    @State private var {}: {} = {}\n",
                nexa_codegen::names::state_name(&state.name),
                state.ty.swift(),
                crate::generator::engine::expressions::expression(&state.initial)
            ));
        }
    }
    for binding in &focus_bindings {
        out.push_str(&format!(
            "    @FocusState private var {}: Bool\n",
            nexa_codegen::names::state_name(binding),
        ));
    }
    let uses_adaptive_color = features.component_uses_adaptive_color(&component.name);
    let uses_size_class = features.component_uses_size_class(&component.name);
    if uses_adaptive_color {
        out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n");
    }
    if uses_size_class {
        out.push_str(
            "    @Environment(\\.horizontalSizeClass) private var nexaHorizontalSizeClass\n",
        );
        out.push_str("    @Environment(\\.verticalSizeClass) private var nexaVerticalSizeClass\n");
    }

    if !component.parameters.is_empty()
        || !component.states.is_empty()
        || has_content_slot
        || uses_adaptive_color
        || uses_size_class
    {
        out.push('\n');
    }
    out.push_str("    init(");
    let mut init_parameters = component
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "_ {}: {}",
                nexa_codegen::names::state_name(&parameter.name),
                parameter.ty.swift()
            )
        })
        .collect::<Vec<_>>();
    if has_content_slot {
        init_parameters.push("@ViewBuilder nexaContent: @escaping () -> SlotContent".to_owned());
    }
    out.push_str(&init_parameters.join(", "));
    if init_parameters.is_empty() {
        out.push_str(") {}\n\n");
    } else {
        out.push_str(") {\n");
        for parameter in &component.parameters {
            let name = nexa_codegen::names::state_name(&parameter.name);
            out.push_str(&format!("        self.{name} = {name}\n"));
        }
        if has_content_slot {
            out.push_str("        self.nexaContent = nexaContent\n");
        }
        out.push_str("    }\n\n");
    }

    out.push_str("    var body: some View {\n");
    render_immutable_state(&component.states, 2, out);
    render_body(&component.body, module, features, 2, out);
    out.push_str("\n    }\n}\n");
}

fn component_has_content_slot(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| found |= matches!(node, Node::Content),
        &mut |_| {},
    );
    found
}

fn component_uses_fast_list(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| found |= matches!(node, Node::FastList { .. }),
        &mut |_| {},
    );
    found
}

fn render_body(
    body: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    match body {
        [node] => render_node(node, module, features, depth, out),
        children => layout::render_layout(
            LayoutKind::Column,
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
