use nexa_codegen::names::state_name;
use nexa_ir::{BottomBarTab, Module};

use super::{
    components::render_children,
    utils::{indent, swift_string},
};

pub(super) fn render_app_bottom_bar(
    state: &str,
    tabs: &[BottomBarTab],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!("TabView(selection: ${}) {{\n", state_name(state)));
    for (position, tab) in tabs.iter().enumerate() {
        render_children(&tab.children, module, depth + 1, out);
        out.push('\n');
        indent(out, depth + 1);
        out.push_str(".tabItem {\n");
        indent(out, depth + 2);
        if let Some(icon) = &tab.icon {
            out.push_str(&format!(
                "Label({}, systemImage: {})\n",
                swift_string(&tab.label),
                swift_string(icon)
            ));
        } else {
            out.push_str(&format!("Text({})\n", swift_string(&tab.label)));
        }
        indent(out, depth + 1);
        out.push('}');
        out.push_str(&format!(
            "\n{}.tag(Int32({}))",
            "    ".repeat(depth + 1),
            tab.index
        ));
        if let Some(badge) = &tab.badge {
            out.push_str(&format!(
                "\n{}.badge({})",
                "    ".repeat(depth + 1),
                swift_string(badge)
            ));
        }
        if position + 1 < tabs.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
