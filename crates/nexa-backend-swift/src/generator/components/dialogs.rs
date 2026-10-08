use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Node};

use crate::generator::{
    components::{controls, render_children, render_node},
    utils::indent,
};

use super::RenderScope;

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_dialog(
    state: &str,
    title: &Expr,
    message: &Expr,
    comment: Option<&str>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state_name = state_name(state);
    let source_message = match message {
        Expr::LocalizedText { value, .. } => value.as_ref(),
        message => message,
    };
    let has_message = !matches!(source_message, Expr::String(value) if value.is_empty());
    out.line_at(
        depth,
        format_args!(
            "Color.clear.frame(width: 0, height: 0).alert({}, isPresented: ${state_name}) {{",
            crate::generator::expressions::localized_text_view(title, comment)
        ),
    );
    render_children(children, scope.module, scope.features, depth + 1, out);
    if has_message {
        out.push('\n');
        indent(out, depth);
        out.push_str("} message: {\n");
        out.line_at(
            depth + 1,
            format_args!(
                "{}",
                crate::generator::expressions::localized_text_view(message, comment)
            ),
        );
    }
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_confirmation_dialog(
    state: &str,
    title: &Expr,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state_name = state_name(state);
    out.line_at(
        depth,
        format_args!(
            "Color.clear.frame(width: 0, height: 0).confirmationDialog({}, isPresented: ${state_name}, titleVisibility: .visible) {{",
            crate::generator::expressions::localized_text_view(title, None)
        ),
    );
    for (index, child) in children.iter().enumerate() {
        if matches!(child, Node::Button { .. }) {
            controls::render_confirmation_action_button(child, depth + 1, out);
        } else {
            render_node(child, scope.module, scope.features, depth + 1, out);
        }
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, Module, Node};

    use crate::generator::features::Features;

    use super::{RenderScope, render_confirmation_dialog, render_dialog};

    fn empty_module() -> Module {
        Module {
            app_name: "Dialog".to_owned(),
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
        }
    }

    fn button(label: &str) -> Node {
        Node::Button {
            label: Expr::String(label.to_owned()),
            icon: None,
            loading: None,
            disabled: None,
            style: None,
            size: None,
            shape: None,
            tint: None,
            glass: false,
            actions: Vec::new(),
        }
    }

    #[test]
    fn confirmation_dialog_uses_visible_anchor_and_direct_actions() {
        let module = empty_module();
        let features = Features::default();
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let children = [button("High"), button("Cancel")];
        let mut output = SourceWriter::new();

        render_confirmation_dialog(
            "showingPriorityPicker",
            &Expr::String("Select Priority".to_owned()),
            &children,
            &scope,
            0,
            &mut output,
        );

        assert!(output.contains(
            "Color.clear.frame(width: 0, height: 0).confirmationDialog(Text(\"Select Priority\"), isPresented: $nexa_showingPriorityPicker, titleVisibility: .visible) {"
        ));
        assert_eq!(output.as_str().matches("Button(action:").count(), 2);
        assert!(!output.contains("VStack"));
        assert!(!output.contains(".buttonStyle("));
        assert!(!output.contains(".frame(minWidth:"));
    }

    #[test]
    fn alert_uses_visible_anchor() {
        let module = empty_module();
        let features = Features::default();
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let mut output = SourceWriter::new();

        render_dialog(
            "showingAlert",
            &Expr::String("Title".to_owned()),
            &Expr::String("Message".to_owned()),
            None,
            &[button("OK")],
            &scope,
            0,
            &mut output,
        );

        assert!(output.contains(
            "Color.clear.frame(width: 0, height: 0).alert(Text(\"Title\"), isPresented: $nexa_showingAlert) {"
        ));
    }
}
