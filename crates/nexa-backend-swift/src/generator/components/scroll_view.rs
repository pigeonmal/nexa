use nexa_codegen::SourceWriter;
use nexa_ir::{Node, ViewStyle, adaptive_layout::ScrollAxis};

use crate::generator::{
    components::render_children, engine::expressions::expression, utils::indent,
};

use super::RenderScope;

pub(crate) struct ScrollViewOptions<'a> {
    pub axis: ScrollAxis,
    pub hide_horizontal_scrollbar: bool,
    pub hide_vertical_scrollbar: bool,
    pub style: &'a ViewStyle,
    pub component_ref: Option<&'a nexa_ir::Expr>,
}

pub(crate) fn render_scroll_view(
    options: ScrollViewOptions<'_>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let ScrollViewOptions {
        axis,
        hide_horizontal_scrollbar,
        hide_vertical_scrollbar,
        style,
        component_ref,
    } = options;
    let axis = match axis {
        ScrollAxis::Vertical => ".vertical",
        ScrollAxis::Horizontal => ".horizontal",
        // `Both` is used only by two-dimensional collection owners. The
        // public ScrollView API validates to one axis before producing IR.
        ScrollAxis::Both => ".vertical",
    };
    let reference = component_ref
        .map(expression)
        .unwrap_or_else(|| "nil".to_owned());
    indent(out, depth);
    out.push_str(&format!(
        "NexaScrollViewPrimitive(axis: {axis}, hideHorizontalScrollbar: {hide_horizontal_scrollbar}, hideVerticalScrollbar: {hide_vertical_scrollbar}, componentRef: {reference}) {{\n"
    ));
    out.line_at(depth + 1, format_args!("NexaScrollContent {{"));
    match axis {
        ".vertical" => out.line_at(
            depth + 2,
            format_args!(
                "NexaColumnPrimitive(alignment: .leading, textAlignment: .leading, spacing: 0) {{"
            ),
        ),
        _ => out.line_at(
            depth + 2,
            format_args!("NexaRowPrimitive(alignment: .top, spacing: 0) {{"),
        ),
    }
    render_children(children, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}}"));
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
    out.line_at(
        depth,
        format_args!(".frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)"),
    );
    super::layout::append_style(out, depth, style, ".topLeading");
}
