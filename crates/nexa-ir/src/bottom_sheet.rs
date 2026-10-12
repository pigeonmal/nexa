//! Target-neutral transforms for bottom-sheet child trees.

use crate::{Node, ToolbarPlacement, WhenCase};

/// Bottom-sheet body and navigation actions separated while retaining
/// conditional branches in each tree.
#[derive(Clone, Debug, Default)]
pub struct BottomSheetChildren {
    /// Nodes rendered in the sheet's scrollable/body content.
    pub content: Vec<Node>,
    /// Toolbar nodes retained in source order, with layout wrappers removed.
    pub toolbars: Vec<Node>,
}

impl BottomSheetChildren {
    /// Returns toolbar child nodes for one native navigation slot.
    ///
    /// Conditional nodes are preserved so each platform renders the same
    /// active branch. The `Toolbar` wrappers are removed because the caller
    /// owns placement when it installs content into a native slot.
    pub fn toolbar_slot(&self, placement: ToolbarPlacement) -> Vec<Node> {
        toolbar_slot(&self.toolbars, placement)
    }
}

/// Separates toolbar nodes from bottom-sheet body content.
///
/// Layout and keyboard-aware wrappers are transparent to toolbar placement.
/// `If` and `When` wrappers remain around extracted actions and body content
/// so state-dependent sheet controls preserve their source branches.
pub fn extract_bottom_sheet_children(nodes: &[Node]) -> BottomSheetChildren {
    let mut extracted = BottomSheetChildren::default();

    for node in nodes {
        match node {
            Node::Toolbar { .. } => extracted.toolbars.push(node.clone()),
            Node::Layout {
                kind,
                spacing,
                style,
                children,
            } => {
                let children = extract_bottom_sheet_children(children);
                if !children.content.is_empty() || children.toolbars.is_empty() {
                    extracted.content.push(Node::Layout {
                        kind: *kind,
                        spacing: *spacing,
                        style: *style,
                        children: children.content,
                    });
                }
                extracted.toolbars.extend(children.toolbars);
            }
            Node::KeyboardAware { dismiss, children } => {
                let children = extract_bottom_sheet_children(children);
                if !children.content.is_empty() || children.toolbars.is_empty() {
                    extracted.content.push(Node::KeyboardAware {
                        dismiss: *dismiss,
                        children: children.content,
                    });
                }
                extracted.toolbars.extend(children.toolbars);
            }
            Node::If {
                condition,
                then_body,
                else_body,
                transition,
            } => {
                let then_parts = extract_bottom_sheet_children(then_body);
                let else_parts = else_body
                    .as_ref()
                    .map(|body| extract_bottom_sheet_children(body));
                let had_body = !then_body.is_empty()
                    || else_body.as_ref().is_some_and(|body| !body.is_empty());
                let content = !then_parts.content.is_empty()
                    || else_parts
                        .as_ref()
                        .is_some_and(|parts| !parts.content.is_empty());
                let has_toolbars = !then_parts.toolbars.is_empty()
                    || else_parts
                        .as_ref()
                        .is_some_and(|parts| !parts.toolbars.is_empty());
                let then_content = then_parts.content;
                let then_toolbars = then_parts.toolbars;
                let (else_content, else_toolbars) = else_parts
                    .map(|parts| (parts.content, parts.toolbars))
                    .unzip();

                if content || !had_body {
                    extracted.content.push(Node::If {
                        condition: condition.clone(),
                        then_body: then_content,
                        else_body: else_content,
                        transition: *transition,
                    });
                }

                if has_toolbars {
                    extracted.toolbars.push(Node::If {
                        condition: condition.clone(),
                        then_body: then_toolbars,
                        else_body: else_toolbars,
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
                let case_parts = cases
                    .iter()
                    .map(|case| {
                        (
                            case.value.clone(),
                            extract_bottom_sheet_children(&case.body),
                        )
                    })
                    .collect::<Vec<_>>();
                let else_parts = extract_bottom_sheet_children(else_body);
                let had_body =
                    cases.iter().any(|case| !case.body.is_empty()) || !else_body.is_empty();
                let has_content = case_parts
                    .iter()
                    .any(|(_, parts)| !parts.content.is_empty())
                    || !else_parts.content.is_empty();
                let has_toolbars = case_parts
                    .iter()
                    .any(|(_, parts)| !parts.toolbars.is_empty())
                    || !else_parts.toolbars.is_empty();

                let mut content_cases = Vec::with_capacity(case_parts.len());
                let mut toolbar_cases = Vec::with_capacity(case_parts.len());
                for (case_value, parts) in case_parts {
                    content_cases.push(WhenCase {
                        value: case_value.clone(),
                        body: parts.content,
                    });
                    toolbar_cases.push(WhenCase {
                        value: case_value,
                        body: parts.toolbars,
                    });
                }
                let BottomSheetChildren {
                    content: else_content,
                    toolbars: else_toolbars,
                } = else_parts;

                if has_content || !had_body {
                    extracted.content.push(Node::When {
                        value: value.clone(),
                        cases: content_cases,
                        else_body: else_content,
                        transition: *transition,
                    });
                }

                if has_toolbars {
                    extracted.toolbars.push(Node::When {
                        value: value.clone(),
                        cases: toolbar_cases,
                        else_body: else_toolbars,
                        transition: *transition,
                    });
                }
            }
            _ => extracted.content.push(node.clone()),
        }
    }

    extracted
}

fn toolbar_slot(nodes: &[Node], placement: ToolbarPlacement) -> Vec<Node> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            Node::Toolbar {
                placement: candidate,
                children,
            } if *candidate == placement => result.extend(children.iter().cloned()),
            Node::Toolbar { .. } => {}
            Node::If {
                condition,
                then_body,
                else_body,
                transition,
            } => {
                let then_body = toolbar_slot(then_body, placement);
                let else_body = else_body.as_ref().map(|body| toolbar_slot(body, placement));
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
                        body: toolbar_slot(&case.body, placement),
                    })
                    .collect::<Vec<_>>();
                let else_body = toolbar_slot(else_body, placement);
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

#[cfg(test)]
mod tests {
    use super::extract_bottom_sheet_children;
    use crate::{Expr, LayoutKind, Node, ToolbarPlacement, Type, ViewStyle, WhenCase};

    fn text(value: &str) -> Node {
        Node::Text {
            value: Expr::String(value.to_owned()),
            style: Default::default(),
        }
    }

    fn layout(children: Vec<Node>) -> Node {
        Node::Layout {
            kind: LayoutKind::Column,
            spacing: 0.0,
            style: ViewStyle::default(),
            children,
        }
    }

    #[test]
    fn separates_toolbar_slots_and_body_through_layout_wrappers() {
        let children = extract_bottom_sheet_children(&[layout(vec![
            Node::Toolbar {
                placement: ToolbarPlacement::Leading,
                children: vec![text("Cancel")],
            },
            Node::Toolbar {
                placement: ToolbarPlacement::Trailing,
                children: vec![text("Save")],
            },
            text("Sheet body"),
        ])]);

        assert_eq!(children.content.len(), 1);
        assert_eq!(children.toolbar_slot(ToolbarPlacement::Leading).len(), 1);
        assert_eq!(children.toolbar_slot(ToolbarPlacement::Trailing).len(), 1);
        assert!(children.toolbar_slot(ToolbarPlacement::Leading).iter().any(
            |node| matches!(node, Node::Text { value: Expr::String(value), .. } if value == "Cancel")
        ));
        assert!(children.content.iter().any(|node| matches!(
            node,
            Node::Layout { children, .. }
                if children.iter().any(|child| matches!(child, Node::Text { value: Expr::String(value), .. } if value == "Sheet body"))
        )));
    }

    #[test]
    fn keeps_toolbar_and_content_in_their_original_conditional_branches() {
        let children = extract_bottom_sheet_children(&[Node::If {
            condition: Expr::State("showDatePicker".to_owned(), Type::Bool),
            then_body: vec![layout(vec![
                Node::Toolbar {
                    placement: ToolbarPlacement::Leading,
                    children: vec![text("Cancel")],
                },
                text("Date picker"),
            ])],
            else_body: Some(vec![layout(vec![
                Node::Toolbar {
                    placement: ToolbarPlacement::Trailing,
                    children: vec![text("Save")],
                },
                text("Editor"),
            ])]),
            transition: None,
        }]);

        assert!(matches!(
            children.content.first(),
            Some(Node::If { then_body, else_body: Some(else_body), .. })
                if !then_body.is_empty() && !else_body.is_empty()
        ));
        assert!(matches!(
            children.toolbars.first(),
            Some(Node::If { then_body, else_body: Some(else_body), .. })
                if !then_body.is_empty() && !else_body.is_empty()
        ));
        assert_eq!(children.toolbar_slot(ToolbarPlacement::Leading).len(), 1);
        assert_eq!(children.toolbar_slot(ToolbarPlacement::Trailing).len(), 1);
    }

    #[test]
    fn preserves_when_cases_in_extracted_toolbar_slots() {
        let children = extract_bottom_sheet_children(&[Node::When {
            value: Expr::State("sheetMode".to_owned(), Type::String),
            cases: vec![
                WhenCase {
                    value: Expr::String("editing".to_owned()),
                    body: vec![Node::Toolbar {
                        placement: ToolbarPlacement::Trailing,
                        children: vec![text("Save")],
                    }],
                },
                WhenCase {
                    value: Expr::String("reading".to_owned()),
                    body: vec![text("Read only")],
                },
            ],
            else_body: vec![text("Unknown mode")],
            transition: None,
        }]);

        let slot = children.toolbar_slot(ToolbarPlacement::Trailing);
        assert!(matches!(
            slot.first(),
            Some(Node::When { cases, else_body, .. })
                if cases.len() == 2
                    && cases[0].body.len() == 1
                    && cases[1].body.is_empty()
                    && else_body.is_empty()
        ));
        assert!(matches!(
            children.content.first(),
            Some(Node::When { cases, else_body, .. })
                if cases.len() == 2
                    && cases[0].body.is_empty()
                    && cases[1].body.len() == 1
                    && else_body.len() == 1
        ));
    }
}
