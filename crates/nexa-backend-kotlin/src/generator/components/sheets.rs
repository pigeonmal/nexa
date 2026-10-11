use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Module, Node, ToolbarPlacement, WhenCase};

use crate::generator::{components::render_children, controls, engine::features::Features};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let _ = (features, imports);
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
    let state = state_name(state);
    let sheet_title = title.filter(|_| partial);
    out.line_at(depth, format_args!("NexaBottomSheetPrimitive("));
    out.line_at(depth + 1, format_args!("isPresented = {state},"));
    out.line_at(depth + 1, format_args!("partial = {partial},"));
    out.line_at(depth + 1, format_args!("largeOnly = {large_only},"));
    out.line_at(
        depth + 1,
        format_args!(
            "title = {},",
            sheet_title.map_or_else(
                || "null".to_owned(),
                crate::generator::expressions::expression
            )
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("onDismissRequest = {{ {state} = false }},"),
    );

    if sheet_title.is_some() {
        let leading = sheet_toolbar_nodes(children, ToolbarPlacement::Leading);
        let trailing = sheet_toolbar_nodes(children, ToolbarPlacement::Trailing);
        out.line_at(
            depth + 1,
            format_args!(
                "hasNavigationIcon = {},",
                sheet_toolbar_visibility(children, ToolbarPlacement::Leading)
                    .unwrap_or_else(|| "false".to_owned())
            ),
        );
        out.line_at(
            depth + 1,
            format_args!(
                "hasActions = {},",
                sheet_toolbar_visibility(children, ToolbarPlacement::Trailing)
                    .unwrap_or_else(|| "false".to_owned())
            ),
        );
        let content = remove_sheet_toolbars(children);
        out.line_at(depth + 1, format_args!("navigationIcon = {{"));
        render_sheet_toolbar_children(&leading, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
        out.line_at(depth + 1, format_args!("actions = {{"));
        render_sheet_toolbar_children(&trailing, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
        out.line_at(depth + 1, format_args!("content = {{"));
        render_children(&content, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
    } else {
        out.line_at(depth + 1, format_args!("hasNavigationIcon = false,"));
        out.line_at(depth + 1, format_args!("hasActions = false,"));
        out.line_at(depth + 1, format_args!("navigationIcon = {{ }},"));
        out.line_at(depth + 1, format_args!("actions = {{ }},"));
        out.line_at(depth + 1, format_args!("content = {{"));
        render_children(children, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
    }
    out.line_at(depth, format_args!(")"));
}

fn render_sheet_toolbar_children(
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
        if let Node::Picker {
            items,
            state,
            icon,
            label,
            tint,
        } = node
        {
            controls::render_picker(
                items,
                state,
                icon.as_ref(),
                label.as_ref(),
                tint.as_ref(),
                true,
                depth,
                out,
            );
        } else {
            render_children(std::slice::from_ref(node), module, features, depth, out);
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

fn sheet_toolbar_nodes(nodes: &[Node], placement: ToolbarPlacement) -> Vec<Node> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            Node::Toolbar {
                placement: candidate,
                children,
            } if *candidate == placement => result.extend(children.iter().cloned()),
            Node::Toolbar { .. } => {}
            Node::KeyboardAware { children, .. } | Node::Layout { children, .. } => {
                result.extend(sheet_toolbar_nodes(children, placement));
            }
            Node::If {
                condition,
                then_body,
                else_body,
                transition,
            } => {
                let then_body = sheet_toolbar_nodes(then_body, placement);
                let else_body = else_body
                    .as_ref()
                    .map(|body| sheet_toolbar_nodes(body, placement));
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
                        body: sheet_toolbar_nodes(&case.body, placement),
                    })
                    .collect::<Vec<_>>();
                let else_body = sheet_toolbar_nodes(else_body, placement);
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

/// Generates the runtime visibility predicate for an extracted toolbar slot.
/// The slot can be structurally present in the IR while its active `If` / `When`
/// branch has no toolbar, so a static boolean would leave an empty native slot
/// mounted when a sheet changes modes.
fn sheet_toolbar_visibility(nodes: &[Node], placement: ToolbarPlacement) -> Option<String> {
    let mut visibility = None;
    for node in nodes {
        let node_visibility = match node {
            Node::Toolbar {
                placement: candidate,
                ..
            } if *candidate == placement => Some("true".to_owned()),
            Node::KeyboardAware { children, .. } | Node::Layout { children, .. } => {
                sheet_toolbar_visibility(children, placement)
            }
            Node::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                let then_visibility = sheet_toolbar_visibility(then_body, placement);
                let else_visibility = else_body
                    .as_ref()
                    .and_then(|body| sheet_toolbar_visibility(body, placement));
                match (then_visibility, else_visibility) {
                    (None, None) => None,
                    (Some(then_value), Some(else_value)) if then_value == else_value => {
                        Some(then_value)
                    }
                    (then_value, else_value) => Some(format!(
                        "(if ({}) {} else {})",
                        crate::generator::expressions::expression(condition),
                        then_value.unwrap_or_else(|| "false".to_owned()),
                        else_value.unwrap_or_else(|| "false".to_owned()),
                    )),
                }
            }
            Node::When {
                value,
                cases,
                else_body,
                ..
            } => {
                let case_visibility = cases
                    .iter()
                    .map(|case| {
                        (
                            case.value.clone(),
                            sheet_toolbar_visibility(&case.body, placement),
                        )
                    })
                    .collect::<Vec<_>>();
                let else_visibility = sheet_toolbar_visibility(else_body, placement);
                if case_visibility.iter().all(|(_, visible)| visible.is_none())
                    && else_visibility.is_none()
                {
                    None
                } else if case_visibility
                    .iter()
                    .all(|(_, visible)| visible.as_deref() == Some("true"))
                    && else_visibility.as_deref() == Some("true")
                {
                    Some("true".to_owned())
                } else {
                    let mut arms = case_visibility
                        .into_iter()
                        .map(|(case_value, visible)| {
                            format!(
                                "{} -> {}",
                                crate::generator::expressions::expression(&case_value),
                                visible.unwrap_or_else(|| "false".to_owned())
                            )
                        })
                        .collect::<Vec<_>>();
                    arms.push(format!(
                        "else -> {}",
                        else_visibility.unwrap_or_else(|| "false".to_owned())
                    ));
                    Some(format!(
                        "(when ({}) {{ {} }})",
                        crate::generator::expressions::expression(value),
                        arms.join("; ")
                    ))
                }
            }
            _ => None,
        };
        visibility = match (visibility, node_visibility) {
            (None, next) | (next, None) => next,
            (Some(left), Some(right)) if left == "true" || right == "true" => {
                Some("true".to_owned())
            }
            (Some(left), Some(right)) => Some(format!("({left} || {right})")),
        };
    }
    visibility
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{
        Expr, KeyboardDismissMode, LayoutKind, Module, Node, TextStyle, ToolbarPlacement, Type,
        ViewStyle,
    };

    use crate::generator::engine::features::Features;

    use super::render_bottom_sheet;

    fn empty_module() -> Module {
        Module {
            app_name: "SheetParity".to_owned(),
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

    #[test]
    fn titled_sheet_places_nested_toolbars_with_the_title_and_removes_them_from_content() {
        let text = |value: &str| Node::Text {
            value: Expr::String(value.to_owned()),
            style: TextStyle::default(),
        };
        let module = empty_module();
        let features = Features::default();
        let children = [Node::KeyboardAware {
            dismiss: KeyboardDismissMode::Never,
            children: vec![Node::Layout {
                kind: LayoutKind::Column,
                spacing: 0.0,
                style: ViewStyle::default(),
                children: vec![
                    Node::Toolbar {
                        placement: ToolbarPlacement::Leading,
                        children: vec![text("Back")],
                    },
                    Node::Toolbar {
                        placement: ToolbarPlacement::Trailing,
                        children: vec![text("Done")],
                    },
                    text("Body"),
                ],
            }],
        }];
        let mut output = SourceWriter::new();

        render_bottom_sheet(
            "commentsPresented",
            true,
            false,
            Some(&Expr::String("Comments".to_owned())),
            &children,
            &module,
            &features,
            0,
            &mut output,
        );

        assert!(output.contains("NexaBottomSheetPrimitive("));
        assert!(output.contains("partial = true"));
        assert!(output.contains("title = \"Comments\","));
        assert!(output.contains("navigationIcon = {"));
        assert!(output.contains("actions = {"));
        assert!(output.contains("content = {"));
        assert!(output.contains("NexaTextPrimitive(text = \"Back\")"));
        assert!(output.contains("NexaTextPrimitive(text = \"Done\")"));
        assert_eq!(
            output
                .as_str()
                .matches("NexaTextPrimitive(text = \"Back\")")
                .count(),
            1
        );
        assert_eq!(
            output
                .as_str()
                .matches("NexaTextPrimitive(text = \"Done\")")
                .count(),
            1
        );
        assert!(output.contains("NexaTextPrimitive(text = \"Body\")"));
    }

    #[test]
    fn conditional_sheet_toolbars_stay_in_their_leading_and_trailing_slots() {
        let text = |value: &str| Node::Text {
            value: Expr::String(value.to_owned()),
            style: TextStyle::default(),
        };
        let toolbar = |placement, value: &str| Node::Toolbar {
            placement,
            children: vec![text(value)],
        };
        let column = |children| Node::Layout {
            kind: LayoutKind::Column,
            spacing: 0.0,
            style: ViewStyle::default(),
            children,
        };
        let module = empty_module();
        let features = Features::default();
        let children = [Node::If {
            condition: Expr::State("addingReminder".to_owned(), Type::Bool),
            then_body: vec![column(vec![
                toolbar(ToolbarPlacement::Leading, "Cancel"),
                toolbar(ToolbarPlacement::Trailing, "Save"),
                text("Reminder form"),
            ])],
            else_body: Some(vec![column(vec![
                toolbar(ToolbarPlacement::Leading, "Back"),
                toolbar(ToolbarPlacement::Trailing, "Add"),
                text("Reminder list"),
            ])]),
            transition: None,
        }];
        let mut output = SourceWriter::new();

        render_bottom_sheet(
            "showingReminders",
            true,
            false,
            Some(&Expr::String("Reminders".to_owned())),
            &children,
            &module,
            &features,
            0,
            &mut output,
        );

        assert!(output.contains("hasNavigationIcon = true,"));
        assert!(output.contains("hasActions = true,"));
        assert!(output.contains("navigationIcon = {"));
        assert!(output.contains("actions = {"));
        assert!(output.contains("if (nexa_addingReminder) {"));
        for label in ["Cancel", "Back", "Save", "Add"] {
            assert_eq!(
                output
                    .as_str()
                    .matches(&format!("NexaTextPrimitive(text = \"{label}\")"))
                    .count(),
                1
            );
        }
        assert!(output.contains("NexaTextPrimitive(text = \"Reminder form\")"));
        assert!(output.contains("NexaTextPrimitive(text = \"Reminder list\")"));
    }

    #[test]
    fn conditional_sheet_toolbars_hide_slots_when_the_active_branch_has_no_toolbar() {
        let text = |value: &str| Node::Text {
            value: Expr::String(value.to_owned()),
            style: TextStyle::default(),
        };
        let toolbar = |placement, value: &str| Node::Toolbar {
            placement,
            children: vec![text(value)],
        };
        let module = empty_module();
        let features = Features::default();
        let children = [Node::If {
            condition: Expr::State("showingDatePicker".to_owned(), Type::Bool),
            then_body: vec![text("Date picker content")],
            else_body: Some(vec![Node::Layout {
                kind: LayoutKind::Column,
                spacing: 0.0,
                style: ViewStyle::default(),
                children: vec![
                    toolbar(ToolbarPlacement::Leading, "Priority"),
                    toolbar(ToolbarPlacement::Trailing, "Save"),
                    text("Task editor content"),
                ],
            }]),
            transition: None,
        }];
        let mut output = SourceWriter::new();

        render_bottom_sheet(
            "showingTaskEditor",
            true,
            false,
            Some(&Expr::Conditional {
                condition: Box::new(Expr::State("showingDatePicker".to_owned(), Type::Bool)),
                then_value: Box::new(Expr::String("Pick Date".to_owned())),
                else_value: Box::new(Expr::String("New Task".to_owned())),
                value_type: Type::String,
            }),
            &children,
            &module,
            &features,
            0,
            &mut output,
        );

        assert!(
            output.contains("hasNavigationIcon = (if (nexa_showingDatePicker) false else true),")
        );
        assert!(output.contains("hasActions = (if (nexa_showingDatePicker) false else true),"));
        assert!(output.contains("if (nexa_showingDatePicker) {"));
    }
}
