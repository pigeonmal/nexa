use nexa_codegen::names::navigation_case_name;
use nexa_ir::{Expr, Module, Node, ScreenId};

use super::expressions::expression;
use super::expressions::text_expression;
use super::render_immutable_state;
use super::{components::render_children, utils::indent};

pub(super) fn render_link(
    destination: ScreenId,
    arguments: &[Expr],
    guard: Option<&Expr>,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    if matches!(guard, Some(Expr::Bool(false))) {
        render_children(children, module, depth, out);
        return;
    }
    indent(out, depth);
    out.push_str(&format!(
        "NavigationLink(value: {}) {{\n",
        route_value(destination, arguments)
    ));
    render_children(children, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    if let Some(guard) = guard {
        out.push_str(&format!(".disabled(!({}))", expression(guard)));
    }
}

pub(super) fn render_back(label: &nexa_ir::Expr, depth: usize, out: &mut String) {
    indent(out, depth);
    out.push_str(&format!("Button({}) {{\n", text_expression(label)));
    indent(out, depth + 1);
    out.push_str("nexaDismiss()\n");
    indent(out, depth);
    out.push('}');
}

pub(super) fn render_navigation_stack(
    module: &Module,
    root: ScreenId,
    arguments: &[Expr],
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("NavigationStack {\n");
    indent(out, depth + 1);
    out.push_str(&screen_call(root, arguments));
    out.push('\n');
    indent(out, depth + 2);
    out.push_str(".navigationDestination(for: NexaNavigationRoute.self) { route in\n");
    indent(out, depth + 3);
    out.push_str("switch route {\n");
    for screen in &module.screens {
        indent(out, depth + 3);
        if screen.parameters.is_empty() {
            out.push_str(&format!("case .{}:\n", navigation_case_name(screen.id)));
        } else {
            out.push_str(&format!(
                "case let .{}({}):\n",
                navigation_case_name(screen.id),
                screen
                    .parameters
                    .iter()
                    .map(|parameter| nexa_codegen::names::state_name(&parameter.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        indent(out, depth + 4);
        out.push_str(&screen_call(
            screen.id,
            &screen
                .parameters
                .iter()
                .map(|parameter| Expr::State(parameter.name.clone(), parameter.ty.clone()))
                .collect::<Vec<_>>(),
        ));
        out.push('\n');
    }
    indent(out, depth + 3);
    out.push_str("}\n");
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}

pub(super) fn render_screen_function(
    screen: &nexa_ir::Screen,
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    out.push('\n');
    indent(out, depth);
    out.push_str("@ViewBuilder\n");
    indent(out, depth);
    let parameters = screen
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
    out.push_str(&format!(
        "private func {}({}) -> some View {{\n",
        screen_view_name(screen.id),
        parameters.join(", ")
    ));
    render_immutable_state(&module.states, depth + 1, out);
    render_immutable_state(&screen.states, depth + 1, out);
    render_children(&screen.body, module, depth + 1, out);
    super::render_on_appear_modifier(
        screen.on_appear.as_deref(),
        screen.on_appear_async,
        depth + 1,
        out,
    );
    super::render_on_disappear_modifier(screen.on_disappear.as_deref(), depth + 1, out);
    super::render_status_bar_modifiers(screen.status_bar.or(module.status_bar), depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("}\n");
}

fn screen_view_name(screen: ScreenId) -> String {
    format!("nexaScreen{}", screen.0)
}

fn route_value(destination: ScreenId, arguments: &[Expr]) -> String {
    let route = navigation_case_name(destination);
    if arguments.is_empty() {
        format!("NexaNavigationRoute.{route}")
    } else {
        format!(
            "NexaNavigationRoute.{route}({})",
            arguments
                .iter()
                .map(expression)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn screen_call(screen: ScreenId, arguments: &[Expr]) -> String {
    if arguments.is_empty() {
        format!("{}()", screen_view_name(screen))
    } else {
        format!(
            "{}({})",
            screen_view_name(screen),
            arguments
                .iter()
                .map(expression)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
