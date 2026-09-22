use nexa_codegen::names::state_name;
use nexa_ir::{BottomBarTab, Module};

use crate::generator::{
    components::render_children,
    features::Features,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_bottom_bar,
        "androidx.compose.material3.NavigationBar",
    );
    imports.add(
        features.uses_bottom_bar,
        "androidx.compose.material3.NavigationBarItem",
    );
    imports.add(
        features.uses_bottom_bar,
        "androidx.compose.material3.Scaffold",
    );
    imports.add(
        features.uses_tab_icon || features.uses_button_icon,
        "androidx.compose.material3.Icon",
    );
    imports.add(features.uses_tab_badge, "androidx.compose.material3.Badge");
    imports.add(
        features.uses_tab_badge,
        "androidx.compose.material3.BadgedBox",
    );
    imports.add(
        features.uses_tab_badge_placeholder,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        features.uses_tab_badge_placeholder,
        "androidx.compose.foundation.layout.size",
    );
    imports.add(
        features.uses_tab_badge_placeholder,
        "androidx.compose.ui.unit.dp",
    );
}

pub(crate) fn render_app_bottom_bar(
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
            "NavigationBarItem(selected = {} == {}, onClick = {{ {} = {} }}, icon = {{",
            state_name(state),
            tab.index,
            state_name(state),
            tab.index
        ));
        if let Some(badge) = &tab.badge {
            out.push_str(&format!(
                " BadgedBox(badge = {{ Badge {{ Text({}) }} }}) {{ ",
                kotlin_string(badge)
            ));
            if let Some(icon) = &tab.icon {
                out.push_str(&format!(
                    "Icon(painter = nexaDrawablePainter({}), contentDescription = {})",
                    kotlin_string(icon),
                    kotlin_string(&tab.label)
                ));
            } else {
                out.push_str("Box(modifier = Modifier.size(24.dp))");
            }
            out.push_str(" } ");
        } else if let Some(icon) = &tab.icon {
            out.push_str(&format!(
                " Icon(painter = nexaDrawablePainter({}), contentDescription = {}) ",
                kotlin_string(icon),
                kotlin_string(&tab.label)
            ));
        }
        out.push_str(&format!(
            "}}, label = {{ Text({}) }})\n",
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
