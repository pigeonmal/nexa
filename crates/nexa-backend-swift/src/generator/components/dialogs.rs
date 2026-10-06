use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Node};

use crate::generator::{components::render_children, utils::indent};

use super::RenderScope;

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_dialog(
    state: &str,
    title: &Expr,
    message: &Expr,
    comment: Option<&str>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state_name = state_name(state);
    let source_message = match message {
        Expr::LocalizedText { value, .. } => value.as_ref(),
        message => message,
    };
    let has_message = !matches!(source_message, Expr::String(value) if value.is_empty());
    out.line_at(
        depth,
        format_args!(
            "EmptyView().alert({}, isPresented: ${state_name}) {{",
            crate::generator::expressions::localized_text_view(title, comment)
        ),
    );
    render_children(children, scope.module, scope.features, depth + 1, out);
    if has_message {
        out.push('\n');
        indent(out, depth);
        out.push_str("} message: {\n");
        out.line_at(
            depth + 1,
            format_args!(
                "{}",
                crate::generator::expressions::localized_text_view(message, comment)
            ),
        );
    }
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_confirmation_dialog(
    state: &str,
    title: &Expr,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state_name = state_name(state);
    out.line_at(
        depth,
        format_args!(
            "EmptyView().confirmationDialog({}, isPresented: ${state_name}, titleVisibility: .visible) {{",
            crate::generator::expressions::localized_text_view(title, None)
        ),
    );
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
