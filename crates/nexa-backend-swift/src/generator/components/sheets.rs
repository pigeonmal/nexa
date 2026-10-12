use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Module, Node, WhenCase};

use crate::generator::features::Features;
use crate::generator::{bottom_bar, components::render_children};

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
            let toolbar_nodes = sheet_toolbar_nodes(&children);
            let content = remove_sheet_toolbars(&children);
            out.line_at(2, format_args!("NavigationStack {{"));
            out.line_at(3, format_args!("Group {{"));
            render_children(&content, module, features, 4, out);
            out.line_at(3, format_args!("}}"));
            out.line_at(
                3,
                format_args!(
                    ".navigationTitle({})",
                    crate::generator::engine::expressions::expression(title)
                ),
            );
            out.line_at(3, format_args!(".navigationBarTitleDisplayMode(.inline)"));
            if !toolbar_nodes.is_empty() {
                out.line_at(3, format_args!(".toolbar {{"));
                render_sheet_toolbar_items(&toolbar_nodes, module, features, 4, out);
                out.line_at(3, format_args!("}}"));
            }
            out.line_at(2, format_args!("}}"));
        } else {
            out.line_at(2, format_args!("Group {{"));
            render_children(&children, module, features, 3, out);
            out.line_at(2, format_args!("}}"));
        }
        out.line_at(1, format_args!("}}"));
    }
}

fn sheet_toolbar_nodes(nodes: &[Node]) -> Vec<Node> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            Node::Toolbar { .. } => result.push(node.clone()),
            Node::KeyboardAware { children, .. } | Node::Layout { children, .. } => {
                result.extend(sheet_toolbar_nodes(children));
            }
            Node::If {
                condition,
                then_body,
                else_body,
                transition,
            } => {
                let then_body = sheet_toolbar_nodes(then_body);
                let else_body = else_body.as_ref().map(|body| sheet_toolbar_nodes(body));
                if !then_body.is_empty() || else_body.as_ref().is_some_and(|body| !body.is_empty())
                {
                    result.push(Node::If {
                        condition: condition.clone(),
                        then_body,
                        else_body,
                        transition: *transition,
                    });
                }
            }
            Node::When {
                value,
                cases,
                else_body,
                transition,
            } => {
                let cases = cases
                    .iter()
                    .map(|case| WhenCase {
                        value: case.value.clone(),
                        body: sheet_toolbar_nodes(&case.body),
                    })
                    .collect::<Vec<_>>();
                let else_body = sheet_toolbar_nodes(else_body);
                if cases.iter().any(|case| !case.body.is_empty()) || !else_body.is_empty() {
                    result.push(Node::When {
                        value: value.clone(),
                        cases,
                        else_body,
                        transition: *transition,
                    });
                }
            }
            _ => {}
        }
    }
    result
}

fn render_sheet_toolbar_items(
    nodes: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match node {
            Node::Toolbar { .. } => {
                bottom_bar::render_navigation_toolbar_items(node, module, features, depth, out);
            }
            Node::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                out.line_at(
                    depth,
                    format_args!(
                        "if {} {{",
                        crate::generator::engine::expressions::expression(condition)
                    ),
                );
                render_sheet_toolbar_items(then_body, module, features, depth + 1, out);
                if let Some(else_body) = else_body {
                    out.push('\n');
                    out.line_at(depth, format_args!("}} else {{"));
                    render_sheet_toolbar_items(else_body, module, features, depth + 1, out);
                }
                out.push('\n');
                out.line_at(depth, format_args!("}}"));
            }
            Node::When {
                value,
                cases,
                else_body,
                ..
            } => {
                out.line_at(
                    depth,
                    format_args!(
                        "switch {} {{",
                        crate::generator::engine::expressions::expression(value)
                    ),
                );
                for case in cases {
                    out.line_at(
                        depth + 1,
                        format_args!(
                            "case {}:",
                            crate::generator::engine::expressions::expression(&case.value)
                        ),
                    );
                    render_sheet_toolbar_items(&case.body, module, features, depth + 2, out);
                }
                out.line_at(depth + 1, format_args!("default:"));
                render_sheet_toolbar_items(else_body, module, features, depth + 2, out);
                out.push('\n');
                out.line_at(depth, format_args!("}}"));
            }
            _ => {}
        }
    }
}

fn remove_sheet_toolbars(nodes: &[Node]) -> Vec<Node> {
    nodes
        .iter()
        .filter_map(|node| match node {
            Node::Toolbar { .. } => None,
            Node::KeyboardAware { dismiss, children } => {
                let content = remove_sheet_toolbars(children);
                if content.is_empty() && !children.is_empty() {
                    None
                } else {
                    Some(Node::KeyboardAware {
                        dismiss: *dismiss,
                        children: content,
                    })
                }
            }
            Node::Layout {
                kind,
                spacing,
                style,
                children,
            } => {
                let content = remove_sheet_toolbars(children);
                if content.is_empty() && !children.is_empty() {
                    None
                } else {
                    Some(Node::Layout {
                        kind: *kind,
                        spacing: *spacing,
                        style: *style,
                        children: content,
                    })
                }
            }
            Node::If {
                condition,
                then_body,
                else_body,
                transition,
            } => {
                let then_content = remove_sheet_toolbars(then_body);
                let else_content = else_body.as_ref().map(|body| remove_sheet_toolbars(body));
                let has_content = !then_content.is_empty()
                    || else_content.as_ref().is_some_and(|body| !body.is_empty());
                let had_content = !then_body.is_empty()
                    || else_body.as_ref().is_some_and(|body| !body.is_empty());
                if !has_content && had_content {
                    None
                } else {
                    Some(Node::If {
                        condition: condition.clone(),
                        then_body: then_content,
                        else_body: else_content,
                        transition: *transition,
                    })
                }
            }
            Node::When {
                value,
                cases,
                else_body,
                transition,
            } => {
                let had_content =
                    cases.iter().any(|case| !case.body.is_empty()) || !else_body.is_empty();
                let cases = cases
                    .iter()
                    .map(|case| WhenCase {
                        value: case.value.clone(),
                        body: remove_sheet_toolbars(&case.body),
                    })
                    .collect::<Vec<_>>();
                let else_content = remove_sheet_toolbars(else_body);
                let has_content =
                    cases.iter().any(|case| !case.body.is_empty()) || !else_content.is_empty();
                if !has_content && had_content {
                    None
                } else {
                    Some(Node::When {
                        value: value.clone(),
                        cases,
                        else_body: else_content,
                        transition: *transition,
                    })
                }
            }
            _ => Some(node.clone()),
        })
        .collect()
}

fn bottom_sheet_content_name(state: &str) -> String {
    format!(
        "nexa_bottom_sheet_{}",
        state_name(state).trim_start_matches("nexa_")
    )
}

#[cfg(test)]
mod tests {
    use super::{bottom_sheet_content_name, render_bottom_sheet, render_bottom_sheet_helpers};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, LayoutKind, Module, Node, ToolbarPlacement, Type, ViewStyle};

    use crate::generator::features::Features;

    fn empty_module(body: Vec<Node>) -> Module {
        Module {
            app_name: "BottomSheetParity".to_owned(),
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
            body,
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

    fn text(value: &str) -> Node {
        Node::Text {
            value: Expr::String(value.to_owned()),
            style: Default::default(),
        }
    }

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

    #[test]
    fn titled_sheet_extracts_toolbar_items_onto_its_navigation_stack() {
        let sheet = Node::BottomSheet {
            state: "sheetPresented".to_owned(),
            partial: true,
            large_only: false,
            title: Some(Expr::String("Parity sheet".to_owned())),
            children: vec![Node::Layout {
                kind: LayoutKind::Column,
                spacing: 0.0,
                style: ViewStyle::default(),
                children: vec![
                    Node::Toolbar {
                        placement: ToolbarPlacement::Leading,
                        children: vec![text("Cancel")],
                    },
                    Node::Toolbar {
                        placement: ToolbarPlacement::Trailing,
                        children: vec![text("Save"), text("Prominent")],
                    },
                    text("Sheet body"),
                ],
            }],
        };
        let module = empty_module(vec![sheet]);
        let mut output = SourceWriter::new();

        render_bottom_sheet_helpers(&module.body, &module, &Features::default(), &mut output);

        let generated = output.as_str();
        assert!(generated.contains(".navigationTitle(\"Parity sheet\")"));
        assert!(generated.contains(".toolbar {"));
        assert!(generated.contains("ToolbarItem(placement: .topBarLeading)"));
        assert_eq!(
            generated
                .matches("ToolbarItem(placement: .topBarTrailing)")
                .count(),
            2
        );
        for label in ["Cancel", "Save", "Prominent", "Sheet body"] {
            assert_eq!(generated.matches(&format!("Text(\"{label}\")")).count(), 1);
        }
        assert!(!generated.contains("EmptyView()\n.toolbar"));
    }

    #[test]
    fn titled_sheet_keeps_conditional_toolbar_items_in_their_active_branch() {
        let sheet = Node::BottomSheet {
            state: "sheetPresented".to_owned(),
            partial: true,
            large_only: false,
            title: Some(Expr::String("Editor".to_owned())),
            children: vec![Node::If {
                condition: Expr::State("showingDatePicker".to_owned(), Type::Bool),
                then_body: vec![Node::Layout {
                    kind: LayoutKind::Column,
                    spacing: 0.0,
                    style: ViewStyle::default(),
                    children: vec![
                        Node::Toolbar {
                            placement: ToolbarPlacement::Leading,
                            children: vec![text("Cancel")],
                        },
                        text("Date picker content"),
                    ],
                }],
                else_body: Some(vec![Node::Layout {
                    kind: LayoutKind::Column,
                    spacing: 0.0,
                    style: ViewStyle::default(),
                    children: vec![
                        Node::Toolbar {
                            placement: ToolbarPlacement::Trailing,
                            children: vec![text("Save")],
                        },
                        text("Editor content"),
                    ],
                }]),
                transition: None,
            }],
        };
        let module = empty_module(vec![sheet]);
        let mut output = SourceWriter::new();

        render_bottom_sheet_helpers(&module.body, &module, &Features::default(), &mut output);

        let generated = output.as_str();
        assert!(generated.contains(".toolbar {"));
        assert!(generated.contains("if nexa_showingDatePicker {"));
        assert!(generated.contains("ToolbarItem(placement: .topBarLeading)"));
        assert!(generated.contains("ToolbarItem(placement: .topBarTrailing)"));
        assert!(generated.contains("Text(\"Date picker content\")"));
        assert!(generated.contains("Text(\"Editor content\")"));
        assert_eq!(generated.matches("Text(\"Cancel\")").count(), 1);
        assert_eq!(generated.matches("Text(\"Save\")").count(), 1);
    }
}
