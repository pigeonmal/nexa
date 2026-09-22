use nexa_codegen::names::navigation_route_name;
use nexa_ir::{Module, Node, ScreenId};

use super::{
    components::render_children,
    features::Features,
    utils::{indent, kotlin_string},
};

pub(super) fn render_link(
    destination: ScreenId,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!(
        "TextButton(onClick = {{ navController.navigate({}) }}) {{\n",
        kotlin_string(&navigation_route_name(destination))
    ));
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

pub(super) fn render_navigation_stack(
    module: &Module,
    root: ScreenId,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("val navController = rememberNavController()\n");
    indent(out, depth);
    out.push_str("NavHost(\n");
    indent(out, depth + 1);
    out.push_str("navController = navController,\n");
    indent(out, depth + 1);
    out.push_str(&format!(
        "startDestination = {},\n",
        kotlin_string(&navigation_route_name(root))
    ));
    indent(out, depth);
    out.push_str(") {\n");
    for screen in &module.screens {
        indent(out, depth + 1);
        out.push_str(&format!(
            "composable(route = {}) {{\n",
            kotlin_string(&navigation_route_name(screen.id))
        ));
        render_children(&screen.body, module, features, depth + 2, out);
        if screen.on_appear.is_some() || screen.on_disappear.is_some() {
            out.push('\n');
        }
        super::render_on_appear_effect(screen.on_appear.as_deref(), depth + 2, out);
        super::render_on_disappear_effect(screen.on_disappear.as_deref(), depth + 2, out);
        if screen.status_bar.or(module.status_bar).is_some() {
            out.push('\n');
        }
        super::render_status_bar(
            screen.status_bar.or(module.status_bar),
            features.uses_status_bar,
            depth + 2,
            out,
        );
        if screen.on_appear.is_none() && screen.on_disappear.is_none() {
            out.push('\n');
        }
        indent(out, depth + 1);
        out.push_str("}\n");
    }
    indent(out, depth);
    out.push('}');
}
