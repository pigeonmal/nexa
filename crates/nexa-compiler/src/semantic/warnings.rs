use std::collections::HashSet;

use nexa_diagnostics::{CompileWarning, Span};
use nexa_syntax::ast;

use crate::Target;

/// Collects source-level diagnostics that are useful even when the program is
/// valid. The analysis intentionally stays independent from native backends.
pub(super) fn analyze(app: &ast::App, target: Target) -> Vec<CompileWarning> {
    let mut warnings = Vec::new();
    let app_names = app
        .states
        .iter()
        .map(|declaration| declaration.name.clone())
        .collect::<HashSet<_>>();
    analyze_scope(
        &app.states,
        &[],
        &app_names,
        app.body.iter(),
        app.screens.iter().flat_map(|screen| screen.body.iter()),
        target,
        None,
        &mut warnings,
    );

    for component in &app.components {
        let mut names = component
            .parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<HashSet<_>>();
        names.extend(
            component
                .states
                .iter()
                .map(|declaration| declaration.name.clone()),
        );
        analyze_scope(
            &component.states,
            &component.parameters,
            &names,
            component.body.iter(),
            std::iter::empty(),
            target,
            component.source_file.as_deref(),
            &mut warnings,
        );
    }

    warnings
}

fn analyze_scope<'a, I, J>(
    declarations: &[ast::StateDecl],
    parameters: &[ast::ComponentParameter],
    names: &HashSet<String>,
    body: I,
    screens: J,
    target: Target,
    file: Option<&str>,
    warnings: &mut Vec<CompileWarning>,
) where
    I: IntoIterator<Item = &'a ast::Node>,
    J: IntoIterator<Item = &'a ast::Node>,
{
    let mut used = HashSet::new();
    for declaration in declarations {
        walk_expression(&declaration.initial, names, &mut used);
    }
    for node in body.into_iter().chain(screens) {
        walk_node(node, names, &mut used, target, file, warnings);
    }

    for declaration in declarations {
        if !used.contains(&declaration.name) {
            let kind = if declaration.mutable {
                "state"
            } else {
                "constant"
            };
            push_warning(
                warnings,
                declaration.span,
                format!(
                    "unused {kind} `{}`{}; it will be removed from generated code",
                    declaration.name,
                    target_suffix(target)
                ),
                file,
            );
        }
    }
    for parameter in parameters {
        if !used.contains(&parameter.name) {
            push_warning(
                warnings,
                parameter.span,
                format!(
                    "unused component parameter `{}`{}",
                    parameter.name,
                    target_suffix(target)
                ),
                file,
            );
        }
    }
}

fn walk_node(
    node: &ast::Node,
    names: &HashSet<String>,
    used: &mut HashSet<String>,
    target: Target,
    file: Option<&str>,
    warnings: &mut Vec<CompileWarning>,
) {
    match node {
        ast::Node::Layout {
            spacing, children, ..
        } => {
            if let Some(spacing) = spacing {
                walk_expression(spacing, names, used);
            }
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
        }
        ast::Node::Platform {
            target: platform,
            children,
            ..
        } => {
            if target == Target::All || platform_matches(*platform, target) {
                for child in children {
                    walk_node(child, names, used, target, file, warnings);
                }
            }
        }
        ast::Node::Text { value, .. } => walk_expression(value, names, used),
        ast::Node::StatusBar { .. } => {}
        ast::Node::Button { label, actions, .. } => {
            walk_expression(label, names, used);
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::TextInput { value, .. } | ast::Node::Switch { value, .. } => {
            walk_expression(value, names, used)
        }
        ast::Node::Image { .. } => {}
        ast::Node::Pressable {
            disabled,
            children,
            actions,
            ..
        } => {
            if let Some(disabled) = disabled {
                walk_expression(disabled, names, used);
            }
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::NavigationStack { .. } => {}
        ast::Node::NavigationLink { children, .. } | ast::Node::KeyboardAware { children, .. } => {
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
        }
        ast::Node::BottomSheet {
            is_presented,
            children,
            ..
        } => {
            walk_expression(is_presented, names, used);
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
        }
        ast::Node::RefreshControl {
            is_refreshing,
            children,
            actions,
            ..
        } => {
            walk_expression(is_refreshing, names, used);
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::AppBottomBar { selected, tabs, .. } => {
            walk_expression(selected, names, used);
            for tab in tabs {
                for child in &tab.children {
                    walk_node(child, names, used, target, file, warnings);
                }
            }
        }
        ast::Node::FastList {
            source,
            index,
            item,
            children,
            ..
        } => {
            match source {
                ast::ListSource::Count(count) | ast::ListSource::Items(count) => {
                    walk_expression(count, names, used)
                }
            }
            let mut row_names = names.clone();
            if let Some(ast::Expr::Name(name, _)) = index {
                row_names.insert(name.clone());
            }
            if let Some(ast::Expr::Name(name, _)) = item {
                row_names.insert(name.clone());
            }
            for child in children {
                walk_node(child, &row_names, used, target, file, warnings);
            }
        }
        ast::Node::If {
            condition,
            then_body,
            else_body,
            span,
        } => {
            warn_constant_condition(condition, *span, file, warnings);
            walk_expression(condition, names, used);
            for child in then_body {
                walk_node(child, names, used, target, file, warnings);
            }
            if let Some(else_body) = else_body {
                for child in else_body {
                    walk_node(child, names, used, target, file, warnings);
                }
            }
        }
        ast::Node::ComponentCall { arguments, .. } => {
            for value in arguments.values() {
                walk_expression(value, names, used);
            }
        }
    }
}

fn walk_actions(
    actions: &[ast::Stmt],
    names: &HashSet<String>,
    used: &mut HashSet<String>,
    target: Target,
    file: Option<&str>,
    warnings: &mut Vec<CompileWarning>,
) {
    for action in actions {
        match action {
            ast::Stmt::Assign { name, value, .. } => {
                if names.contains(name) {
                    used.insert(name.clone());
                }
                walk_expression(value, names, used);
            }
            ast::Stmt::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                warn_constant_condition(condition, *span, file, warnings);
                walk_expression(condition, names, used);
                walk_actions(then_branch, names, used, target, file, warnings);
                if let Some(else_branch) = else_branch {
                    walk_actions(else_branch, names, used, target, file, warnings);
                }
            }
        }
    }
}

fn platform_matches(platform: ast::PlatformTarget, target: Target) -> bool {
    matches!(
        (platform, target),
        (ast::PlatformTarget::Ios, Target::Swift) | (ast::PlatformTarget::Android, Target::Kotlin)
    )
}

fn target_suffix(target: Target) -> &'static str {
    match target {
        Target::Swift => " for Swift",
        Target::Kotlin => " for Kotlin",
        Target::All => "",
    }
}

fn walk_expression(expr: &ast::Expr, names: &HashSet<String>, used: &mut HashSet<String>) {
    match expr {
        ast::Expr::Name(name, _) => {
            if names.contains(name) {
                used.insert(name.clone());
            }
        }
        ast::Expr::Add(left, right, _)
        | ast::Expr::Binary(left, _, right, _)
        | ast::Expr::Pair(left, right, _) => {
            walk_expression(left, names, used);
            walk_expression(right, names, used);
        }
        ast::Expr::Not(value, _) => walk_expression(value, names, used),
        ast::Expr::Array(values, _) => {
            for value in values {
                walk_expression(value, names, used);
            }
        }
        ast::Expr::Map(entries, _) => {
            for (key, value) in entries {
                walk_expression(key, names, used);
                walk_expression(value, names, used);
            }
        }
        ast::Expr::Triple(first, second, third, _) => {
            walk_expression(first, names, used);
            walk_expression(second, names, used);
            walk_expression(third, names, used);
        }
        ast::Expr::Interpolation(parts, _) => {
            for part in parts {
                if let ast::StringPart::Name(name) = part {
                    if names.contains(name) {
                        used.insert(name.clone());
                    }
                }
            }
        }
        ast::Expr::String(_, _)
        | ast::Expr::Number(_, _)
        | ast::Expr::Bool(_, _)
        | ast::Expr::ThemeToken(_, _)
        | ast::Expr::IsRegularWidth(_) => {}
    }
}

fn warn_constant_condition(
    expression: &ast::Expr,
    span: Span,
    file: Option<&str>,
    warnings: &mut Vec<CompileWarning>,
) {
    if let Some(value) = constant_bool(expression) {
        push_warning(
            warnings,
            span,
            format!(
                "condition is always {}; the compiler will remove the unreachable branch",
                if value { "true" } else { "false" }
            ),
            file,
        );
    }
}

fn push_warning(
    warnings: &mut Vec<CompileWarning>,
    span: Span,
    message: String,
    file: Option<&str>,
) {
    let warning = CompileWarning::new(span, message);
    warnings.push(match file {
        Some(file) => warning.with_file(file),
        None => warning,
    });
}

#[derive(Clone, Copy)]
enum Constant<'a> {
    Bool(bool),
    String(&'a str),
    Number(f64),
}

fn constant_bool(expression: &ast::Expr) -> Option<bool> {
    match expression {
        ast::Expr::Bool(value, _) => Some(*value),
        ast::Expr::Not(value, _) => constant_bool(value).map(|value| !value),
        ast::Expr::Binary(left, operator, right, _) => match operator {
            ast::BinaryOp::And => match constant_bool(left) {
                Some(false) => Some(false),
                Some(true) => constant_bool(right),
                None => None,
            },
            ast::BinaryOp::Or => match constant_bool(left) {
                Some(true) => Some(true),
                Some(false) => constant_bool(right),
                None => None,
            },
            ast::BinaryOp::Equal
            | ast::BinaryOp::NotEqual
            | ast::BinaryOp::Less
            | ast::BinaryOp::LessEqual
            | ast::BinaryOp::Greater
            | ast::BinaryOp::GreaterEqual => {
                let left = constant_value(left)?;
                let right = constant_value(right)?;
                compare_constants(left, *operator, right)
            }
        },
        _ => None,
    }
}

fn constant_value(expression: &ast::Expr) -> Option<Constant<'_>> {
    match expression {
        ast::Expr::String(value, _) => Some(Constant::String(value)),
        ast::Expr::Number(value, _) => Some(Constant::Number(value.parse().ok()?)),
        ast::Expr::Bool(value, _) => Some(Constant::Bool(*value)),
        ast::Expr::Not(value, _) => Some(Constant::Bool(!constant_bool(value)?)),
        _ => None,
    }
}

fn compare_constants(
    left: Constant<'_>,
    operator: ast::BinaryOp,
    right: Constant<'_>,
) -> Option<bool> {
    let comparison = match (left, right) {
        (Constant::Bool(left), Constant::Bool(right)) => left.partial_cmp(&right),
        (Constant::String(left), Constant::String(right)) => left.partial_cmp(right),
        (Constant::Number(left), Constant::Number(right)) => left.partial_cmp(&right),
        _ => return None,
    }?;
    Some(match operator {
        ast::BinaryOp::Equal => comparison.is_eq(),
        ast::BinaryOp::NotEqual => !comparison.is_eq(),
        ast::BinaryOp::Less => comparison.is_lt(),
        ast::BinaryOp::LessEqual => comparison.is_le(),
        ast::BinaryOp::Greater => comparison.is_gt(),
        ast::BinaryOp::GreaterEqual => comparison.is_ge(),
        ast::BinaryOp::And | ast::BinaryOp::Or => return None,
    })
}
