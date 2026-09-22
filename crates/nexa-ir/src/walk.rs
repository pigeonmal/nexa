use crate::{Action, Expr, InterpolatedPart, ListSource, Node};

/// Visits IR nodes and expressions in preorder without building an intermediate tree.
pub fn walk_ir(
    nodes: &[Node],
    visit_node: &mut impl FnMut(&Node),
    visit_expression: &mut impl FnMut(&Expr),
) {
    for node in nodes {
        visit_node(node);
        match node {
            Node::Layout { children, .. }
            | Node::NavigationLink { children, .. }
            | Node::KeyboardAware { children, .. }
            | Node::BottomSheet { children, .. } => walk_ir(children, visit_node, visit_expression),
            Node::Pressable {
                disabled,
                children,
                actions,
                long_press_actions,
                ..
            } => {
                walk_expression(disabled, visit_expression);
                walk_actions(actions, visit_expression);
                walk_actions(long_press_actions, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::Link { url, children } => {
                walk_expression(url, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::Accessibility {
                label, children, ..
            } => {
                walk_expression(label, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::RefreshControl {
                children, actions, ..
            } => {
                walk_actions(actions, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::OnAppear { actions, .. } | Node::OnDisappear { actions } => {
                walk_actions(actions, visit_expression)
            }
            Node::AppBottomBar { tabs, .. } => {
                for tab in tabs {
                    walk_ir(&tab.children, visit_node, visit_expression);
                }
            }
            Node::FastList {
                source,
                key,
                children,
                ..
            } => {
                walk_list_source(source, visit_expression);
                if let Some(key) = key {
                    walk_expression(key, visit_expression);
                }
                walk_ir(children, visit_node, visit_expression);
            }
            Node::If {
                condition,
                then_body,
                else_body,
            } => {
                walk_expression(condition, visit_expression);
                walk_ir(then_body, visit_node, visit_expression);
                if let Some(else_body) = else_body {
                    walk_ir(else_body, visit_node, visit_expression);
                }
            }
            Node::When {
                value,
                cases,
                else_body,
            } => {
                walk_expression(value, visit_expression);
                for case in cases {
                    walk_expression(&case.value, visit_expression);
                    walk_ir(&case.body, visit_node, visit_expression);
                }
                walk_ir(else_body, visit_node, visit_expression);
            }
            Node::Text { value, .. } => walk_expression(value, visit_expression),
            Node::Button {
                label,
                loading,
                disabled,
                actions,
                ..
            } => {
                walk_expression(label, visit_expression);
                if let Some(loading) = loading {
                    walk_expression(loading, visit_expression);
                }
                if let Some(disabled) = disabled {
                    walk_expression(disabled, visit_expression);
                }
                walk_actions(actions, visit_expression);
            }
            Node::ComponentCall {
                arguments,
                children,
                ..
            } => {
                for (_, argument) in arguments {
                    walk_expression(argument, visit_expression);
                }
                if let Some(children) = children {
                    walk_ir(children, visit_node, visit_expression);
                }
            }
            Node::TextInput { actions, .. } => walk_actions(actions, visit_expression),
            Node::Content
            | Node::Switch { .. }
            | Node::StatusBar { .. }
            | Node::Direction { .. }
            | Node::NavigationStack { .. } => {}
            Node::Image { source, .. } => {
                if let crate::ImageSource::RemoteUrl(url) = source {
                    walk_expression(url, visit_expression);
                }
            }
        }
    }
}

/// Visits an expression tree in preorder without allocating.
pub fn walk_expression(expression: &Expr, visit: &mut impl FnMut(&Expr)) {
    visit(expression);
    match expression {
        Expr::Add(left, right, _) | Expr::Binary { left, right, .. } => {
            walk_expression(left, visit);
            walk_expression(right, visit);
        }
        Expr::Contains {
            value, collection, ..
        } => {
            walk_expression(value, visit);
            walk_expression(collection, visit);
        }
        Expr::Not(value) => walk_expression(value, visit),
        Expr::Array(items) | Expr::Set(items) => {
            for item in items {
                walk_expression(item, visit);
            }
        }
        Expr::Map(entries) => {
            for (key, value) in entries {
                walk_expression(key, visit);
                walk_expression(value, visit);
            }
        }
        Expr::Pair(first, second) => {
            walk_expression(first, visit);
            walk_expression(second, visit);
        }
        Expr::Triple(first, second, third) => {
            walk_expression(first, visit);
            walk_expression(second, visit);
            walk_expression(third, visit);
        }
        Expr::Call { arguments, .. } => {
            for argument in arguments {
                walk_expression(argument, visit);
            }
        }
        Expr::CollectionTransform {
            collection,
            initial,
            closure,
            ..
        } => {
            walk_expression(collection, visit);
            if let Some(initial) = initial {
                walk_expression(initial, visit);
            }
            walk_expression(closure, visit);
        }
        Expr::Closure { body, .. } => walk_expression(body, visit),
        Expr::NativeCall { arguments, .. } => {
            for (_, argument) in arguments {
                walk_expression(argument, visit);
            }
        }
        Expr::Index {
            collection, index, ..
        } => {
            walk_expression(collection, visit);
            walk_expression(index, visit);
        }
        Expr::Range {
            start, end, step, ..
        } => {
            walk_expression(start, visit);
            walk_expression(end, visit);
            if let Some(step) = step {
                walk_expression(step, visit);
            }
        }
        Expr::Member { base, .. } => walk_expression(base, visit),
        Expr::Coalesce(left, right) => {
            walk_expression(left, visit);
            walk_expression(right, visit);
        }
        Expr::Await(value) => walk_expression(value, visit),
        Expr::Interpolation(parts) => {
            for part in parts {
                if let InterpolatedPart::Value(value) = part {
                    walk_expression(value, visit);
                }
            }
        }
        Expr::String(_)
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::State(_, _)
        | Expr::EnumValue { .. }
        | Expr::Null(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight => {}
    }
}

/// Returns whether a node tree already contains a native scrolling primitive.
pub fn contains_scrollable(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::FastList { .. } | Node::KeyboardAware { .. } => true,
        Node::Layout { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::Pressable { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. } => contains_scrollable(children),
        Node::AppBottomBar { tabs, .. } => {
            tabs.iter().any(|tab| contains_scrollable(&tab.children))
        }
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            contains_scrollable(then_body) || else_body.as_deref().is_some_and(contains_scrollable)
        }
        Node::When {
            cases, else_body, ..
        } => {
            cases.iter().any(|case| contains_scrollable(&case.body))
                || contains_scrollable(else_body)
        }
        Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. }
        | Node::Content
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. } => false,
    })
}

fn walk_list_source(source: &ListSource, visit: &mut impl FnMut(&Expr)) {
    match source {
        ListSource::Count(count) => walk_expression(count, visit),
        ListSource::Items { collection, .. } => walk_expression(collection, visit),
    }
}

pub fn walk_actions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    for action in actions {
        match action {
            Action::Assign { value, .. } => walk_expression(value, visit),
            Action::CollectionMutation { arguments, .. } => {
                for argument in arguments {
                    walk_expression(argument, visit);
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                walk_expression(condition, visit);
                walk_actions(then_branch, visit);
                if let Some(else_branch) = else_branch {
                    walk_actions(else_branch, visit);
                }
            }
            Action::For { iterable, body, .. } => {
                walk_expression(iterable, visit);
                walk_actions(body, visit);
            }
            Action::ForMap { iterable, body, .. } => {
                walk_expression(iterable, visit);
                walk_actions(body, visit);
            }
            Action::While { condition, body } => {
                walk_expression(condition, visit);
                walk_actions(body, visit);
            }
            Action::Break | Action::Continue => {}
        }
    }
}
