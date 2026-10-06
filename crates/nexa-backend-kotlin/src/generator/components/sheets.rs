use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Module, Node};

use crate::generator::engine::expressions::text_expression;
use crate::generator::{
    components::render_children, engine::features::Features, engine::utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_bottom_sheet,
        "androidx.compose.material3.ModalBottomSheet",
    );
    imports.add(
        features.uses_bottom_sheet_full_screen,
        "androidx.compose.ui.window.Dialog",
    );
    imports.add(
        features.uses_bottom_sheet_full_screen,
        "androidx.compose.ui.window.DialogProperties",
    );
    imports.add(
        features.uses_bottom_sheet_full_screen,
        "androidx.compose.material3.Surface",
    );
    imports.add(
        features.uses_bottom_sheet_full_screen,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.uses_bottom_sheet_full_screen,
        "androidx.compose.ui.graphics.RectangleShape",
    );
    imports.add(
        features.uses_bottom_sheet_partial,
        "androidx.compose.material3.rememberModalBottomSheetState",
    );
}

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
    if !partial {
        out.line_at(depth, format_args!("if ({}) {{", state_name(state)));
        out.line_at(
            depth + 1,
            format_args!(
                "Dialog(onDismissRequest = {{ {} = false }}, properties = DialogProperties(usePlatformDefaultWidth = false)) {{",
                state_name(state)
            ),
        );
        out.line_at(
            depth + 2,
            format_args!("Surface(modifier = Modifier.fillMaxSize(), shape = RectangleShape) {{"),
        );
        render_children(children, module, features, depth + 3, out);
        out.push('\n');
        out.line_at(depth + 2, format_args!("}}"));
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(depth, format_args!("}}"));
        return;
    }
    out.line_at(depth, format_args!("if ({}) {{", state_name(state)));
    indent(out, depth + 1);
    if partial {
        out.push_str("ModalBottomSheet(\n");
        out.line_at(
            depth + 2,
            format_args!("onDismissRequest = {{ {} = false }},", state_name(state)),
        );
        indent(out, depth + 2);
        out.push_str(if large_only {
            "sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),\n"
        } else {
            "sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = false),\n"
        });
        indent(out, depth + 1);
        out.push_str(") {\n");
    }
    if let Some(title) = title {
        out.line_at(
            depth + 2,
            format_args!("Column(modifier = Modifier.fillMaxWidth()) {{"),
        );
        out.line_at(
            depth + 3,
            format_args!(
                "Text({}, style = MaterialTheme.typography.titleMedium)",
                text_expression(title)
            ),
        );
        render_children(children, module, features, depth + 3, out);
        out.line_at(depth + 2, format_args!("}}"));
    } else {
        render_children(children, module, features, depth + 2, out);
    }
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}
