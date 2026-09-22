use nexa_codegen::names::state_name;
use nexa_ir::{Module, Node};

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
        features.uses_bottom_sheet_partial,
        "androidx.compose.material3.rememberModalBottomSheetState",
    );
}

pub(crate) fn render_bottom_sheet(
    state: &str,
    partial: bool,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!("if ({}) {{\n", state_name(state)));
    indent(out, depth + 1);
    if partial {
        out.push_str("ModalBottomSheet(\n");
        indent(out, depth + 2);
        out.push_str(&format!(
            "onDismissRequest = {{ {} = false }},\n",
            state_name(state)
        ));
        indent(out, depth + 2);
        out.push_str(
            "sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = false),\n",
        );
        indent(out, depth + 1);
        out.push_str(") {\n");
    } else {
        out.push_str(&format!(
            "ModalBottomSheet(onDismissRequest = {{ {} = false }}) {{\n",
            state_name(state)
        ));
    }
    render_children(children, module, features, depth + 2, out);
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}
