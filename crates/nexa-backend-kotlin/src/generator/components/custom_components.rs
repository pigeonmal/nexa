use nexa_ir::walk::walk_ir;
use nexa_ir::{Component, LayoutKind, Module, Node, ViewStyle};

use crate::generator::{
    components::render_node,
    features::{self, Features},
    layout, state,
    utils::indent,
};

pub(crate) fn render(module: &Module, features: &Features, out: &mut String) {
    for component in &module.components {
        render_component(component, module, features, out);
    }
}

fn render_component(component: &Component, module: &Module, features: &Features, out: &mut String) {
    if component_uses_bottom_sheet(component) {
        out.push_str("\n@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)");
    }
    if features.component_uses_keyboard_interactive(&component.name) {
        out.push_str("\n@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)");
    }
    if component_uses_sticky_header(component) {
        out.push_str("\n@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)");
    }
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
    let needs_navigation = features.component_requires_navigation(&component.name);
    if needs_system_theme && !component.parameters.is_empty() {
        out.push_str(", ");
    }
    if needs_system_theme {
        out.push_str("nexaIsDarkTheme: Boolean");
    }
    if needs_navigation {
        if needs_system_theme || !component.parameters.is_empty() {
            out.push_str(", ");
        }
        out.push_str("navController: NavHostController");
    }
    if component_has_content_slot(component) {
        if needs_system_theme || needs_navigation || !component.parameters.is_empty() {
            out.push_str(", ");
        }
        out.push_str("nexaContent: @Composable () -> Unit");
    }
    out.push_str(") {\n");

    if features.component_uses_link(&component.name) {
        out.push_str("    val nexaLinkContext = LocalContext.current\n");
    }
    if features.component_uses_haptic(&component.name) {
        out.push_str("    val nexaHapticView = LocalView.current\n");
    }
    render_component_states(&component.states, 1, out);
    let focus_bindings = features::collect_focus_bindings(&component.body);
    for binding in &focus_bindings {
        out.push_str(&format!(
            "    val {} = remember {{ FocusRequester() }}\n",
            crate::generator::components::input::focus_requester_name(binding)
        ));
    }
    for binding in &focus_bindings {
        let state_name = nexa_codegen::names::state_name(binding);
        let requester_name = crate::generator::components::input::focus_requester_name(binding);
        out.push_str(&format!(
            "    LaunchedEffect({state_name}) {{\n        if ({state_name}) {requester_name}.requestFocus() else {requester_name}.freeFocus()\n    }}\n"
        ));
    }
    if !focus_bindings.is_empty() {
        out.push('\n');
    }
    render_body(&component.body, module, features, 1, out);
    out.push_str("\n}\n");
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

fn component_uses_bottom_sheet(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| found |= matches!(node, Node::BottomSheet { .. }),
        &mut |_| {},
    );
    found
}

fn component_uses_sticky_header(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| {
            found |= matches!(
                node,
                Node::FastList {
                    sticky_header: Some(_),
                    ..
                }
            )
        },
        &mut |_| {},
    );
    found
}

fn render_component_states(states: &[nexa_ir::State], depth: usize, out: &mut String) {
    for state in states {
        indent(out, depth);
        let name = nexa_codegen::names::state_name(&state.name);
        if state.mutable {
            if state::is_mutable_collection(state) {
                out.push_str(&format!(
                    "val {name} = remember {{ {} }}\n",
                    state::kotlin_state_initializer(state)
                ));
            } else {
                out.push_str(&format!(
                    "var {name} by remember {{ {} }}\n",
                    state::kotlin_state_initializer(state)
                ));
            }
        } else {
            out.push_str(&format!(
                "val {name}: {} = {}\n",
                state.ty.kotlin(),
                crate::generator::engine::expressions::expression(&state.initial)
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
