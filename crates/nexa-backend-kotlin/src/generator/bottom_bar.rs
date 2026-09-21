use nexa_codegen::names::state_name;
use nexa_ir::{BottomBarTab, Module};

use super::{
    components::render_children,
    features::Features,
    utils::{indent, kotlin_string},
};

pub(super) fn render_app_bottom_bar(
    state: &str,
    tabs: &[BottomBarTab],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("Scaffold(\n");
    indent(out, depth + 1);
    out.push_str("bottomBar = {\n");
    indent(out, depth + 2);
    out.push_str("NavigationBar {\n");
    for tab in tabs {
        indent(out, depth + 3);
        out.push_str(&format!(
            "NavigationBarItem(selected = {} == {}, onClick = {{ {} = {} }}, icon = {{}}, label = {{ Text({}) }})\n",
            state_name(state),
            tab.index,
            state_name(state),
            tab.index,
            kotlin_string(&tab.label)
        ));
    }
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push_str(") { nexaBottomBarPadding ->\n");
    indent(out, depth + 1);
    out.push_str("Column(modifier = Modifier.padding(nexaBottomBarPadding)) {\n");
    indent(out, depth + 2);
    out.push_str("when (");
    out.push_str(&state_name(state));
    out.push_str(") {\n");
    for tab in tabs {
        indent(out, depth + 3);
        out.push_str(&format!("{} -> {{\n", tab.index));
        render_children(&tab.children, module, features, depth + 4, out);
        out.push('\n');
        indent(out, depth + 3);
        out.push_str("}\n");
    }
    indent(out, depth + 3);
    out.push_str("else -> {}\n");
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}
