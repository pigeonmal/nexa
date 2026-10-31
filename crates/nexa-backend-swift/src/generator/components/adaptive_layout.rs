use nexa_codegen::SourceWriter;
use nexa_ir::{Node, adaptive_layout::ScrollHeaderBehavior};

use crate::generator::{
    components::render_children,
    engine::features::Features,
    utils::{indent, number},
};

pub(crate) fn render_scroll_header(
    plan: nexa_ir::adaptive_layout::ScrollHeaderPlan,
    header: &[Node],
    children: &[Node],
    module: &nexa_ir::Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let behavior = match plan.behavior {
        ScrollHeaderBehavior::Pinned => "pinned",
        ScrollHeaderBehavior::Collapse => "collapse",
        ScrollHeaderBehavior::HideOnScroll => "hideOnScroll",
    };
    let collapsed_height = plan.collapsed_height.unwrap_or(56.0);
    indent(out, depth);
    out.push_str(&format!(
        "NexaAdaptiveScrollHeaderPrimitive(behavior: .{behavior}, collapsedHeight: {}, header: {{\n",
        number(collapsed_height)
    ));
    render_children(header, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("}, content: {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("})");
}
