use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::bottom_sheet::extract_bottom_sheet_children;
use nexa_ir::{Expr, Module, Node, ToolbarPlacement};

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
    show_drag_handle: bool,
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
        format_args!("showDragHandle = {show_drag_handle},"),
    );
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
        let extracted = extract_bottom_sheet_children(children);
        let leading = extracted.toolbar_slot(ToolbarPlacement::Leading);
        let trailing = extracted.toolbar_slot(ToolbarPlacement::Trailing);
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
        out.line_at(depth + 1, format_args!("navigationIcon = {{"));
        render_sheet_toolbar_children(&leading, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
        out.line_at(depth + 1, format_args!("actions = {{"));
        render_sheet_toolbar_children(&trailing, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
        out.line_at(depth + 1, format_args!("content = {{"));
        render_children(&extracted.content, module, features, depth + 2, out);
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
            item_icons,
            label,
            tint,
        } = node
        {
            controls::render_picker(
                items,
                item_icons,
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
        assert!(output.contains("showDragHandle = false"));
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
            transition_animation: None,
        }];
        let mut output = SourceWriter::new();

        render_bottom_sheet(
            "showingReminders",
            true,
            false,
            true,
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
            transition_animation: None,
        }];
        let mut output = SourceWriter::new();

        render_bottom_sheet(
            "showingTaskEditor",
            true,
            false,
            true,
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
