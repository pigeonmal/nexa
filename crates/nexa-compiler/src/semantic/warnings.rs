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
        app.screens
            .iter()
            .flat_map(|screen| screen.states.iter().map(|state| &state.initial)),
        target,
        None,
        &mut warnings,
    );
    for screen in &app.screens {
        let mut screen_names = app_names.clone();
        screen_names.extend(
            screen
                .states
                .iter()
                .map(|declaration| declaration.name.clone()),
        );
        analyze_scope(
            &screen.states,
            &[],
            &screen_names,
            screen.body.iter(),
            std::iter::empty(),
            std::iter::empty(),
            target,
            None,
            &mut warnings,
        );
    }

    let function_names = app
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect::<HashSet<_>>();
    let mut used_functions = HashSet::new();
    let mut reachability_warnings = Vec::new();
    for state in &app.states {
        walk_expression(&state.initial, &function_names, &mut used_functions);
    }
    for screen in &app.screens {
        for state in &screen.states {
            walk_expression(&state.initial, &function_names, &mut used_functions);
        }
    }
    for node in app
        .body
        .iter()
        .chain(app.screens.iter().flat_map(|screen| screen.body.iter()))
        .chain(
            app.components
                .iter()
                .flat_map(|component| component.body.iter()),
        )
    {
        walk_node(
            node,
            &function_names,
            &mut used_functions,
            target,
            None,
            &mut reachability_warnings,
        );
    }
    let function_by_name = app
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<std::collections::HashMap<_, _>>();
    let mut pending = used_functions.iter().cloned().collect::<Vec<_>>();
    let mut expanded_functions = HashSet::new();
    while let Some(name) = pending.pop() {
        if !expanded_functions.insert(name.clone()) {
            continue;
        }
        let Some(function) = function_by_name.get(name.as_str()) else {
            continue;
        };
        walk_actions(
            &function.body,
            &function_names,
            &mut used_functions,
            target,
            None,
            &mut reachability_warnings,
        );
        pending.extend(
            used_functions
                .iter()
                .filter(|name| !expanded_functions.contains(*name))
                .cloned(),
        );
    }
    for function in &app.functions {
        if !used_functions.contains(&function.name) {
            push_warning(
                &mut warnings,
                function.span,
                format!(
                    "unused function `{}`{}; it will be removed from generated code",
                    function.name,
                    target_suffix(target)
                ),
                None,
            );
        }
    }

    for function in &app.functions {
        let mut names = function
            .parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<HashSet<_>>();
        names.extend(
            function
                .body
                .iter()
                .filter_map(|statement| match statement {
                    ast::Stmt::Let { name, .. } => Some(name.clone()),
                    _ => None,
                }),
        );
        let mut used = HashSet::new();
        walk_actions(
            &function.body,
            &names,
            &mut used,
            target,
            None,
            &mut warnings,
        );
        for statement in &function.body {
            let ast::Stmt::Let { name, span, .. } = statement else {
                continue;
            };
            if !used.contains(name) {
                push_warning(
                    &mut warnings,
                    *span,
                    format!("unused local constant `{name}`{}", target_suffix(target)),
                    None,
                );
            }
        }
        for parameter in &function.parameters {
            if !used.contains(&parameter.name) {
                push_warning(
                    &mut warnings,
                    parameter.span,
                    format!(
                        "unused function parameter `{}`{}",
                        parameter.name,
                        target_suffix(target)
                    ),
                    None,
                );
            }
        }
    }

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
            std::iter::empty(),
            target,
            component.source_file.as_deref(),
            &mut warnings,
        );
    }

    warnings
}

fn analyze_scope<'a, I, J, K>(
    declarations: &[ast::StateDecl],
    parameters: &[ast::ComponentParameter],
    names: &HashSet<String>,
    body: I,
    screens: J,
    initializers: K,
    target: Target,
    file: Option<&str>,
    warnings: &mut Vec<CompileWarning>,
) where
    I: IntoIterator<Item = &'a ast::Node>,
    J: IntoIterator<Item = &'a ast::Node>,
    K: IntoIterator<Item = &'a ast::Expr>,
{
    let mut used = HashSet::new();
    for declaration in declarations {
        walk_expression(&declaration.initial, names, &mut used);
    }
    for initializer in initializers {
        walk_expression(initializer, names, &mut used);
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
        ast::Node::StatusBar { .. }
        | ast::Node::Direction { .. }
        | ast::Node::NavigationBack { .. } => {}
        ast::Node::OnAppear { actions, .. } => {
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::OnDisappear { actions, .. } => {
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::OnActive { actions, .. }
        | ast::Node::OnInactive { actions, .. }
        | ast::Node::OnBackground { actions, .. } => {
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::Button {
            label,
            loading,
            disabled,
            actions,
            ..
        } => {
            walk_expression(label, names, used);
            if let Some(loading) = loading {
                walk_expression(loading, names, used);
            }
            if let Some(disabled) = disabled {
                walk_expression(disabled, names, used);
            }
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::TextInput {
            value,
            focused,
            actions,
            ..
        } => {
            walk_expression(value, names, used);
            if let Some(focused) = focused {
                walk_expression(focused, names, used);
            }
            walk_actions(actions, names, used, target, file, warnings);
        }
        ast::Node::Switch { value, .. } => walk_expression(value, names, used),
        ast::Node::Image { source, .. } => {
            if let ast::ImageSource::Url(url) = source {
                walk_expression(url, names, used);
            }
        }
        ast::Node::Pressable {
            disabled,
            children,
            actions,
            long_press_actions,
            ..
        } => {
            if let Some(disabled) = disabled {
                walk_expression(disabled, names, used);
            }
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
            walk_actions(actions, names, used, target, file, warnings);
            walk_actions(long_press_actions, names, used, target, file, warnings);
        }
        ast::Node::NavigationStack { arguments, .. } => {
            for argument in arguments {
                walk_expression(argument, names, used);
            }
        }
        ast::Node::NavigationLink {
            arguments,
            guard,
            children,
            ..
        } => {
            for argument in arguments {
                walk_expression(argument, names, used);
            }
            if let Some(guard) = guard {
                walk_expression(guard, names, used);
            }
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
        }
        ast::Node::KeyboardAware { children, .. } => {
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
        }
        ast::Node::Link { url, children, .. } => {
            walk_expression(url, names, used);
            for child in children {
                walk_node(child, names, used, target, file, warnings);
            }
        }
        ast::Node::Accessibility {
            label, children, ..
        } => {
            walk_expression(label, names, used);
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
            axis: _,
            section,
            item_extent,
            index,
            item,
            key,
            scroll_position,
            children,
            on_end_reached,
            on_scroll,
            sticky_header,
            section_header,
            span,
        } => {
            match source {
                ast::ListSource::Count(count) | ast::ListSource::Items(count) => {
                    walk_expression(count, names, used)
                }
                ast::ListSource::Sections(sections) => walk_expression(sections, names, used),
            }
            if let Some(section) = section {
                walk_expression(section, names, used);
            }
            if let Some(item_extent) = item_extent {
                walk_expression(item_extent, names, used);
            }
            if let Some(scroll_position) = scroll_position {
                walk_expression(scroll_position, names, used);
            }
            if let Some(actions) = on_end_reached {
                walk_actions(actions, names, used, target, file, warnings);
            }
            if let Some(actions) = on_scroll {
                walk_actions(actions, names, used, target, file, warnings);
            }
            if let Some(sticky_header) = sticky_header {
                for child in sticky_header {
                    walk_node(child, names, used, target, file, warnings);
                }
            }
            let mut row_names = names.clone();
            let is_sections = source_is_sections(source);
            let section_name = section
                .as_ref()
                .and_then(|section| match section {
                    ast::Expr::Name(name, _) => Some(name.as_str()),
                    _ => None,
                })
                .unwrap_or("section");
            if is_sections && section_name != "_" {
                row_names.insert(section_name.to_owned());
            }
            let index_name = index
                .as_ref()
                .and_then(|index| match index {
                    ast::Expr::Name(name, _) => Some(name.as_str()),
                    _ => None,
                })
                .unwrap_or("index");
            if index_name != "_" {
                row_names.insert(index_name.to_owned());
            }
            let item_name = item.as_ref().and_then(|item| match item {
                ast::Expr::Name(name, _) => Some(name.as_str()),
                _ => None,
            });
            if let Some(item_name) = item_name.filter(|name| *name != "_") {
                row_names.insert(item_name.to_owned());
            }
            if index_name != "_" && !nodes_reference_name(children, index_name, target) {
                push_warning(
                    warnings,
                    *span,
                    format!(
                        "unused FastList index binding `{index_name}`{}",
                        target_suffix(target)
                    ),
                    file,
                );
            }
            if let Some(item_name) = item_name
                && key.is_none()
                && !nodes_reference_name(children, item_name, target)
            {
                push_warning(
                    warnings,
                    *span,
                    format!(
                        "unused FastList item binding `{item_name}`{}",
                        target_suffix(target)
                    ),
                    file,
                );
            }
            if section_name != "_"
                && is_sections
                && !nodes_reference_name(children, section_name, target)
                && !section_header
                    .as_deref()
                    .is_some_and(|header| nodes_reference_name(header, section_name, target))
            {
                push_warning(
                    warnings,
                    *span,
                    format!(
                        "unused FastList section binding `{section_name}`{}",
                        target_suffix(target)
                    ),
                    file,
                );
            }
            if let Some(section_header) = section_header {
                for child in section_header {
                    walk_node(child, &row_names, used, target, file, warnings);
                }
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
        ast::Node::When {
            value,
            cases,
            else_body,
            ..
        } => {
            walk_expression(value, names, used);
            for case in cases {
                walk_expression(&case.value, names, used);
                for child in &case.body {
                    walk_node(child, names, used, target, file, warnings);
                }
            }
            for child in else_body {
                walk_node(child, names, used, target, file, warnings);
            }
        }
        ast::Node::ComponentCall {
            arguments,
            children,
            ..
        } => {
            for value in arguments.values() {
                walk_expression(value, names, used);
            }
            if let Some(children) = children {
                for child in children {
                    walk_node(child, names, used, target, file, warnings);
                }
            }
        }
        ast::Node::Content { .. } => {}
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
            ast::Stmt::Let { initial, .. } => walk_expression(initial, names, used),
            ast::Stmt::Assign { name, value, .. } => {
                if names.contains(name) {
                    used.insert(name.clone());
                }
                walk_expression(value, names, used);
            }
            ast::Stmt::CollectionMutation {
                name, arguments, ..
            } => {
                if names.contains(name) {
                    used.insert(name.clone());
                }
                for argument in arguments {
                    walk_expression(argument, names, used);
                }
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
            ast::Stmt::For {
                name,
                iterable,
                body,
                span,
            } => {
                walk_expression(iterable, names, used);
                warn_unused_loop_binding(name, body, *span, file, target, warnings);
                walk_actions(body, names, used, target, file, warnings);
            }
            ast::Stmt::ForMap {
                key_name,
                value_name,
                iterable,
                body,
                span,
            } => {
                walk_expression(iterable, names, used);
                warn_unused_loop_binding(key_name, body, *span, file, target, warnings);
                warn_unused_loop_binding(value_name, body, *span, file, target, warnings);
                walk_actions(body, names, used, target, file, warnings);
            }
            ast::Stmt::While {
                condition, body, ..
            } => {
                walk_expression(condition, names, used);
                walk_actions(body, names, used, target, file, warnings);
            }
            ast::Stmt::Break { .. } | ast::Stmt::Continue { .. } => {}
            ast::Stmt::Return { value, .. } => walk_expression(value, names, used),
        }
    }
}

fn nodes_reference_name(nodes: &[ast::Node], name: &str, target: Target) -> bool {
    let names = std::iter::once(name.to_owned()).collect::<HashSet<_>>();
    let mut used = HashSet::new();
    let mut ignored_warnings = Vec::new();
    for node in nodes {
        walk_node(node, &names, &mut used, target, None, &mut ignored_warnings);
    }
    used.contains(name)
}

fn warn_unused_loop_binding(
    name: &str,
    body: &[ast::Stmt],
    span: Span,
    file: Option<&str>,
    target: Target,
    warnings: &mut Vec<CompileWarning>,
) {
    if !actions_reference_name(body, name) {
        push_warning(
            warnings,
            span,
            format!("unused loop binding `{name}`{}", target_suffix(target)),
            file,
        );
    }
}

fn actions_reference_name(actions: &[ast::Stmt], name: &str) -> bool {
    actions.iter().any(|action| match action {
        ast::Stmt::Let { initial, .. } | ast::Stmt::Return { value: initial, .. } => {
            expression_references_name(initial, name)
        }
        ast::Stmt::Assign { value, .. } => expression_references_name(value, name),
        ast::Stmt::CollectionMutation {
            name: binding,
            arguments,
            ..
        } => {
            binding == name
                || arguments
                    .iter()
                    .any(|argument| expression_references_name(argument, name))
        }
        ast::Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            expression_references_name(condition, name)
                || actions_reference_name(then_branch, name)
                || else_branch
                    .as_deref()
                    .is_some_and(|branch| actions_reference_name(branch, name))
        }
        ast::Stmt::For { iterable, body, .. } | ast::Stmt::ForMap { iterable, body, .. } => {
            expression_references_name(iterable, name) || actions_reference_name(body, name)
        }
        ast::Stmt::While {
            condition, body, ..
        } => expression_references_name(condition, name) || actions_reference_name(body, name),
        ast::Stmt::Break { .. } | ast::Stmt::Continue { .. } => false,
    })
}

fn expression_references_name(expression: &ast::Expr, name: &str) -> bool {
    match expression {
        ast::Expr::Name(candidate, _) => candidate == name,
        ast::Expr::Add(left, right, _)
        | ast::Expr::Binary(left, _, right, _)
        | ast::Expr::Pair(left, right, _) => {
            expression_references_name(left, name) || expression_references_name(right, name)
        }
        ast::Expr::Not(value, _) | ast::Expr::Await(value, _) => {
            expression_references_name(value, name)
        }
        ast::Expr::Array(values, _) => values
            .iter()
            .any(|value| expression_references_name(value, name)),
        ast::Expr::Map(entries, _) => entries.iter().any(|(key, value)| {
            expression_references_name(key, name) || expression_references_name(value, name)
        }),
        ast::Expr::Triple(first, second, third, _) => {
            expression_references_name(first, name)
                || expression_references_name(second, name)
                || expression_references_name(third, name)
        }
        ast::Expr::Call(_, arguments, _) => arguments
            .iter()
            .any(|argument| expression_references_name(argument, name)),
        ast::Expr::MethodCall {
            base, arguments, ..
        } => {
            expression_references_name(base, name)
                || arguments
                    .iter()
                    .any(|argument| expression_references_name(argument, name))
        }
        ast::Expr::Closure { body, .. } => expression_references_name(body, name),
        ast::Expr::QualifiedCall { arguments, .. } => arguments
            .values()
            .any(|argument| expression_references_name(argument, name)),
        ast::Expr::Index {
            collection, index, ..
        } => {
            expression_references_name(collection, name) || expression_references_name(index, name)
        }
        ast::Expr::Member { base, .. } => expression_references_name(base, name),
        ast::Expr::Range {
            start, end, step, ..
        } => {
            expression_references_name(start, name)
                || expression_references_name(end, name)
                || step
                    .as_deref()
                    .is_some_and(|step| expression_references_name(step, name))
        }
        ast::Expr::Coalesce(left, right, _) => {
            expression_references_name(left, name) || expression_references_name(right, name)
        }
        ast::Expr::Interpolation(parts, _) => parts.iter().any(|part| match part {
            ast::StringPart::Name(candidate) => candidate == name,
            ast::StringPart::Expression(expression) => expression_references_name(expression, name),
            ast::StringPart::Literal(_) => false,
        }),
        ast::Expr::String(_, _)
        | ast::Expr::Number(_, _)
        | ast::Expr::Bool(_, _)
        | ast::Expr::EnumCase { .. }
        | ast::Expr::Null(_)
        | ast::Expr::ThemeToken(_, _)
        | ast::Expr::IsRegularWidth(_)
        | ast::Expr::IsCompactWidth(_)
        | ast::Expr::IsRegularHeight(_)
        | ast::Expr::IsCompactHeight(_) => false,
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
        ast::Expr::Call(name, arguments, _) => {
            if names.contains(name) {
                used.insert(name.clone());
            }
            for argument in arguments {
                walk_expression(argument, names, used);
            }
        }
        ast::Expr::MethodCall {
            base, arguments, ..
        } => {
            walk_expression(base, names, used);
            for argument in arguments {
                walk_expression(argument, names, used);
            }
        }
        ast::Expr::Closure { body, .. } => walk_expression(body, names, used),
        ast::Expr::QualifiedCall { arguments, .. } => {
            for argument in arguments.values() {
                walk_expression(argument, names, used);
            }
        }
        ast::Expr::Index {
            collection, index, ..
        } => {
            walk_expression(collection, names, used);
            walk_expression(index, names, used);
        }
        ast::Expr::Range {
            start, end, step, ..
        } => {
            walk_expression(start, names, used);
            walk_expression(end, names, used);
            if let Some(step) = step {
                walk_expression(step, names, used);
            }
        }
        ast::Expr::Member { base, .. } => walk_expression(base, names, used),
        ast::Expr::Coalesce(left, right, _) => {
            walk_expression(left, names, used);
            walk_expression(right, names, used);
        }
        ast::Expr::Await(value, _) => walk_expression(value, names, used),
        ast::Expr::Interpolation(parts, _) => {
            for part in parts {
                match part {
                    ast::StringPart::Name(name) => {
                        if names.contains(name) {
                            used.insert(name.clone());
                        }
                    }
                    ast::StringPart::Expression(expression) => {
                        walk_expression(expression, names, used);
                    }
                    ast::StringPart::Literal(_) => {}
                }
            }
        }
        ast::Expr::String(_, _)
        | ast::Expr::Number(_, _)
        | ast::Expr::Bool(_, _)
        | ast::Expr::EnumCase { .. }
        | ast::Expr::Null(_)
        | ast::Expr::ThemeToken(_, _)
        | ast::Expr::IsRegularWidth(_)
        | ast::Expr::IsCompactWidth(_)
        | ast::Expr::IsRegularHeight(_)
        | ast::Expr::IsCompactHeight(_) => {}
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

fn source_is_sections(source: &ast::ListSource) -> bool {
    matches!(source, ast::ListSource::Sections(_))
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
            ast::BinaryOp::Contains => None,
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
        ast::BinaryOp::And | ast::BinaryOp::Or | ast::BinaryOp::Contains => return None,
    })
}
