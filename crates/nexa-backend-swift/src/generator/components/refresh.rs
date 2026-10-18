use nexa_codegen::SourceWriter;
use nexa_ir::walk::contains_scrollable;
use nexa_ir::{Action, Module, Node};

use crate::generator::features::Features;
use crate::generator::{components::render_children, controls::render_actions, utils::indent};

pub(crate) fn render_refresh_control(
    _state: &str,
    children: &[Node],
    actions: &[Action],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    if !contains_scrollable(children) {
        out.push_str("ScrollView {\n");
    } else {
        out.push_str("VStack {\n");
    }
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    out.push_str("\n");
    indent(out, depth);
    out.push_str(".modifier(NexaRefreshBounceModifier())\n");
    indent(out, depth);
    out.push_str(".refreshable {\n");
    render_actions(actions, depth + 1, out);
    indent(out, depth);
    out.push('}');
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, Module, Node, TextStyle};

    use crate::generator::engine::features::Features;

    use super::render_refresh_control;

    #[test]
    fn refresh_control_installs_bounce_modifier_before_refresh_action() {
        let module = Module {
            app_name: "RefreshControlTest".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let children = [Node::Text {
            value: Expr::String("Short refresh content".to_owned()),
            style: TextStyle::default(),
        }];
        let mut output = SourceWriter::new();

        render_refresh_control(
            "refreshing",
            &children,
            &[],
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        let output = output.as_str();
        let bounce_modifier = output
            .find(".modifier(NexaRefreshBounceModifier())")
            .expect("RefreshControl must force bounce for short content on supported iOS");
        let refresh_action = output
            .find(".refreshable {")
            .expect("RefreshControl must retain the native async refresh action");
        assert!(bounce_modifier < refresh_action);
    }
}
