use nexa_codegen::names::navigation_case_name;
use nexa_ir::{Module, Node, ScreenId};

use super::render_immutable_state;
use super::{components::render_children, utils::indent};

pub(super) fn render_link(
    destination: ScreenId,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!(
        "NavigationLink(value: NexaNavigationRoute.{}) {{\n",
        navigation_case_name(destination)
    ));
    render_children(children, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

pub(super) fn render_navigation_stack(
    module: &Module,
    root: ScreenId,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("NavigationStack {\n");
    indent(out, depth + 1);
    out.push_str(&format!("{}()\n", screen_view_name(root)));
    indent(out, depth + 2);
    out.push_str(".navigationDestination(for: NexaNavigationRoute.self) { route in\n");
    indent(out, depth + 3);
    out.push_str("switch route {\n");
    for screen in &module.screens {
        indent(out, depth + 3);
        out.push_str(&format!("case .{}:\n", navigation_case_name(screen.id)));
        indent(out, depth + 4);
        out.push_str(&format!("{}()\n", screen_view_name(screen.id)));
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
    out.push_str(&format!(
        "private func {}() -> some View {{\n",
        screen_view_name(screen.id)
    ));
    render_immutable_state(&module.states, depth + 1, out);
    render_children(&screen.body, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("}\n");
}

fn screen_view_name(screen: ScreenId) -> String {
    format!("nexaScreen{}", screen.0)
}
