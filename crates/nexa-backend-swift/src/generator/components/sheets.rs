use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Module, Node};

use crate::generator::features::Features;
use crate::generator::{components::render_children, utils::indent};

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_bottom_sheet(
    state: &str,
    partial: bool,
    large_only: bool,
    title: Option<&Expr>,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let presentation = if partial { "sheet" } else { "fullScreenCover" };
    out.line_at(
        depth,
        format_args!(
            "Color.clear.frame(width: 0, height: 0).{presentation}(isPresented: ${}) {{",
            state_name(state)
        ),
    );
    if partial {
        if let Some(title) = title {
            out.line_at(depth + 1, format_args!("NavigationStack {{"));
            out.line_at(depth + 2, format_args!("Group {{"));
            render_children(children, module, features, depth + 3, out);
            out.line_at(depth + 2, format_args!("}}"));
            out.line_at(
                depth + 2,
                format_args!(
                    ".navigationTitle({})",
                    crate::generator::engine::expressions::expression(title)
                ),
            );
            out.line_at(
                depth + 2,
                format_args!(".navigationBarTitleDisplayMode(.inline)"),
            );
            out.line_at(depth + 1, format_args!("}}"));
        } else {
            out.line_at(depth + 1, format_args!("Group {{"));
            render_children(children, module, features, depth + 2, out);
            out.line_at(depth + 1, format_args!("}}"));
        }
        out.push('\n');
        indent(out, depth + 1);
        let detents = if large_only {
            "[.large]"
        } else {
            "[.medium, .large]"
        };
        out.push_str(&format!(
            ".presentationDetents({detents}).presentationDragIndicator(.visible)"
        ));
    } else {
        render_children(children, module, features, depth + 1, out);
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
