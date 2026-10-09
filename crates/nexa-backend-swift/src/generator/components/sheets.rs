use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Module, Node};

use crate::generator::components::render_children;
use crate::generator::features::Features;

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_bottom_sheet(
    state: &str,
    partial: bool,
    large_only: bool,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaBottomSheetPrimitive(isPresented: ${}, partial: {partial}, largeOnly: {large_only}) {{",
            nexa_codegen::names::state_name(state)
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("{}()", bottom_sheet_content_name(state)),
    );
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
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

#[cfg(test)]
mod tests {
    use super::{bottom_sheet_content_name, render_bottom_sheet};
    use nexa_codegen::SourceWriter;

    #[test]
    fn full_screen_sheet_uses_its_typed_content_helper_once() {
        let mut output = SourceWriter::new();

        render_bottom_sheet("editorVisible", false, false, 0, &mut output);

        assert_eq!(
            output
                .as_str()
                .matches(&format!("{}()", bottom_sheet_content_name("editorVisible")))
                .count(),
            1
        );
        assert!(output.contains(
            "NexaBottomSheetPrimitive(isPresented: $nexa_editorVisible, partial: false, largeOnly: false) {"
        ));
        assert!(!output.contains("AnyView"));
    }

    #[test]
    fn partial_sheet_keeps_its_native_detents_and_typed_content() {
        let mut output = SourceWriter::new();

        render_bottom_sheet("editorVisible", true, false, 0, &mut output);

        assert!(output.contains(
            "NexaBottomSheetPrimitive(isPresented: $nexa_editorVisible, partial: true, largeOnly: false) {"
        ));
        assert!(!output.contains("AnyView"));
    }
}
