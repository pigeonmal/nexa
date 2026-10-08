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
    _title: Option<&Expr>,
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
    out.line_at(
        depth + 1,
        format_args!("{}()", bottom_sheet_content_name(state)),
    );
    if partial {
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
        out.line_at(depth + 1, format_args!("AnyView(Group {{"));
        render_children(children, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}})"));
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_bottom_sheet_helpers(
    nodes: &[Node],
    module: &Module,
    features: &Features,
    out: &mut SourceWriter,
) {
    let mut sheets = Vec::new();
    nexa_ir::walk::walk_ir(
        nodes,
        &mut |node| {
            if let Node::BottomSheet {
                state,
                partial,
                title,
                children,
                ..
            } = node
            {
                sheets.push((state.clone(), *partial, title.clone(), children.clone()));
            }
        },
        &mut |_| {},
    );

    let mut emitted = std::collections::BTreeSet::new();
    for (state, partial, title, children) in sheets {
        if !emitted.insert(state.clone()) {
            continue;
        }
        out.push('\n');
        out.line_at(
            1,
            format_args!(
                "@ViewBuilder private func {}() -> some View {{",
                bottom_sheet_content_name(&state)
            ),
        );
        if partial && let Some(title) = title.as_ref() {
            out.line_at(2, format_args!("NavigationStack {{"));
            out.line_at(3, format_args!("Group {{"));
            render_children(&children, module, features, 4, out);
            out.line_at(3, format_args!("}}"));
            out.line_at(
                3,
                format_args!(
                    ".navigationTitle({})",
                    crate::generator::engine::expressions::expression(title)
                ),
            );
            out.line_at(3, format_args!(".navigationBarTitleDisplayMode(.inline)"));
            out.line_at(2, format_args!("}}"));
        } else {
            out.line_at(2, format_args!("Group {{"));
            render_children(&children, module, features, 3, out);
            out.line_at(2, format_args!("}}"));
        }
        out.line_at(1, format_args!("}}"));
    }
}

fn bottom_sheet_content_name(state: &str) -> String {
    format!(
        "nexa_bottom_sheet_{}",
        state_name(state).trim_start_matches("nexa_")
    )
}
