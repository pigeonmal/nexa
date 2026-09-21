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
            | Node::KeyboardAware { children } => walk_ir(children, visit_node, visit_expression),
            Node::Pressable {
                children, actions, ..
            } => {
                walk_actions(actions, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::FastList {
                source, children, ..
            } => {
                walk_list_source(source, visit_expression);
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
            Node::Text { value, .. } => walk_expression(value, visit_expression),
            Node::Button { label, actions } => {
                walk_expression(label, visit_expression);
                walk_actions(actions, visit_expression);
            }
            Node::ComponentCall { arguments, .. } => {
                for (_, argument) in arguments {
                    walk_expression(argument, visit_expression);
                }
            }
            Node::TextInput { .. }
            | Node::Switch { .. }
            | Node::Image { .. }
            | Node::StatusBar { .. }
            | Node::NavigationStack { .. } => {}
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
        | Expr::IsRegularWidth => {}
    }
}

fn walk_list_source(source: &ListSource, visit: &mut impl FnMut(&Expr)) {
    match source {
        ListSource::Count(count) => walk_expression(count, visit),
        ListSource::Items { collection, .. } => walk_expression(collection, visit),
    }
}

fn walk_actions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    for action in actions {
        match action {
            Action::Assign { value, .. } => walk_expression(value, visit),
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
        }
    }
}
