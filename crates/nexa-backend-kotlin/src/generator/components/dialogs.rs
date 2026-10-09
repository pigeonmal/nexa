use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Node};

use crate::generator::{
    components::{controls, render_children},
    engine::{features::Features, imports::ImportSet},
    expressions::expression,
    utils::indent,
};

use super::RenderScope;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_dialog,
        "androidx.compose.material3.AlertDialog",
    );
    imports.add(
        features.uses_dialog,
        "androidx.compose.foundation.layout.Column",
    );
    imports.add(
        features.uses_dialog,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_dialog,
        "androidx.compose.material3.HorizontalDivider",
    );
    imports.add(
        features.uses_dialog,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(features.uses_dialog, "androidx.compose.ui.Alignment");
    imports.add(features.uses_dialog, "androidx.compose.ui.Modifier");
    imports.add(features.uses_dialog, "androidx.compose.ui.unit.dp");
}

pub(crate) fn render_dialog(
    state: &str,
    title: &Expr,
    message: &Expr,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state_name = state_name(state);
    let has_message = !matches!(message, Expr::String(value) if value.is_empty());
    let inputs = children
        .iter()
        .filter(|child| matches!(child, Node::TextInput { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let actions = children
        .iter()
        .filter(|child| matches!(child, Node::Button { .. }))
        .cloned()
        .collect::<Vec<_>>();
    out.line_at(depth, format_args!("if ({state_name}) {{"));
    indent(out, depth + 1);
    out.push_str("AlertDialog(\n");
    out.line_at(
        depth + 2,
        format_args!("onDismissRequest = {{ {state_name} = false }},"),
    );
    out.line_at(
        depth + 2,
        format_args!("title = {{ Text({}) }},", expression(title)),
    );
    if inputs.is_empty() {
        out.line_at(
            depth + 2,
            format_args!("text = {{ Text({}) }},", expression(message)),
        );
    } else {
        out.line_at(depth + 2, format_args!("text = {{ Column {{"));
        if has_message {
            out.line_at(depth + 3, format_args!("Text({})", expression(message)));
        }
        render_children(&inputs, scope.module, scope.features, depth + 3, out);
        out.line_at(depth + 2, format_args!("}} }},"));
    }
    indent(out, depth + 2);
    out.push_str("confirmButton = {\n");
    render_children(&actions, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("},\n");
    indent(out, depth + 1);
    out.push_str(")\n");
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
    let state = state_name(state);
    out.line_at(depth, format_args!("if ({state}) {{"));
    out.line_at(depth + 1, format_args!("AlertDialog("));
    out.line_at(
        depth + 2,
        format_args!("onDismissRequest = {{ {state} = false }},"),
    );
    out.line_at(
        depth + 2,
        format_args!("title = {{ Text({}) }},", expression(title)),
    );
    out.line_at(
        depth + 2,
        format_args!("text = {{ Column(modifier = Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {{"),
    );
    render_confirmation_actions(children, scope, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("}},\n");
    out.line_at(depth + 2, format_args!("confirmButton = {{}},"));
    out.line_at(depth + 1, format_args!(")"));
    out.line_at(depth, format_args!("}}"));
}

fn render_confirmation_actions(
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    for (index, child) in children.iter().enumerate() {
        if let Node::Button {
            label,
            icon,
            loading,
            disabled,
            style,
            size,
            shape,
            tint,
            actions,
            ..
        } = child
        {
            controls::render_button(
                label,
                icon.as_ref(),
                loading.as_ref(),
                disabled.as_ref(),
                Some((*style).unwrap_or(nexa_ir::ButtonStyle::Borderless)),
                *size,
                *shape,
                tint.as_ref(),
                actions,
                true,
                depth,
                out,
            );
        } else {
            render_children(
                std::slice::from_ref(child),
                scope.module,
                scope.features,
                depth,
                out,
            );
        }
        if index + 1 < children.len() {
            out.push('\n');
            out.line_at(
                depth,
                format_args!("HorizontalDivider(thickness = 0.5.dp, color = MaterialTheme.colorScheme.outlineVariant)"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, Module, Node};

    use crate::generator::features::Features;

    use super::{RenderScope, render_confirmation_dialog};

    #[test]
    fn confirmation_dialog_actions_render_as_centered_text_rows() {
        let module = Module {
            app_name: "ConfirmationDialog".to_owned(),
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
        let features = Features::default();
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let button = |label: &str| Node::Button {
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
            "text = { Column(modifier = Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {"
        ));
        assert!(output.contains("NexaButtonPrimitive("));
        assert!(output.contains("style = \"Borderless\""));
        assert!(output.contains("fullWidth = true"));
        assert!(output.contains("HorizontalDivider(thickness = 0.5.dp"));
        assert!(output.contains("confirmButton = {},"));
        assert!(
            !output.contains(
                "modifier = Modifier.defaultMinSize(minWidth = 64.dp, minHeight = 48.dp)"
            )
        );
    }
}
