use std::collections::HashSet;

use nexa_ir::{
    Action, BinaryOp, Expr, InterpolatedPart, LayoutKind, ListSource, Module, Node, NumericType,
    ViewStyle,
};

/// Applies small, semantics-preserving optimizations to the typed IR before
/// either native backend sees it. The pass deliberately stays conservative:
/// it folds only pure constant expressions, removes branches whose conditions
/// are statically known, and drops declarations with no reachable use when
/// their initializers are pure.
pub(crate) fn optimize(module: &mut Module) {
    for function in &mut module.functions {
        for local in &mut function.locals {
            local.initial =
                fold_expression(std::mem::replace(&mut local.initial, Expr::Bool(false)));
        }
        function.body = fold_expression(std::mem::replace(&mut function.body, Expr::Bool(false)));
        prune_unused_function_locals(function);
    }
    module.states.iter_mut().for_each(|state| {
        state.initial = fold_expression(std::mem::replace(&mut state.initial, Expr::Bool(false)));
    });
    module.body = optimize_nodes(std::mem::take(&mut module.body));
    if let Some(actions) = &mut module.on_appear {
        *actions = optimize_actions(std::mem::take(actions));
    }
    if let Some(actions) = &mut module.on_disappear {
        *actions = optimize_actions(std::mem::take(actions));
    }
    if let Some(actions) = &mut module.on_active {
        *actions = optimize_actions(std::mem::take(actions));
    }
    if let Some(actions) = &mut module.on_inactive {
        *actions = optimize_actions(std::mem::take(actions));
    }
    if let Some(actions) = &mut module.on_background {
        *actions = optimize_actions(std::mem::take(actions));
    }
    for screen in &mut module.screens {
        for state in &mut screen.states {
            state.initial =
                fold_expression(std::mem::replace(&mut state.initial, Expr::Bool(false)));
        }
        screen.body = optimize_nodes(std::mem::take(&mut screen.body));
        if let Some(actions) = &mut screen.on_appear {
            *actions = optimize_actions(std::mem::take(actions));
        }
        if let Some(actions) = &mut screen.on_disappear {
            *actions = optimize_actions(std::mem::take(actions));
        }
    }
    for component in &mut module.components {
        component.body = optimize_nodes(std::mem::take(&mut component.body));
        for state in &mut component.states {
            state.initial =
                fold_expression(std::mem::replace(&mut state.initial, Expr::Bool(false)));
        }
    }
    prune_unused_functions(module);
    prune_unused_states(module);
    prune_unused_structs(module);
    prune_unused_plugins(module);
}

fn prune_unused_function_locals(function: &mut nexa_ir::Function) {
    let mut referenced = HashSet::new();
    collect_expression_state_names(&function.body, &mut referenced);
    let mut kept = Vec::with_capacity(function.locals.len());
    for local in function.locals.drain(..).rev() {
        if referenced.contains(&local.name) || !is_pure_expression(&local.initial) {
            collect_expression_state_names(&local.initial, &mut referenced);
            kept.push(local);
        }
    }
    kept.reverse();
    function.locals = kept;
}

fn collect_expression_state_names(expression: &Expr, names: &mut HashSet<String>) {
    match expression {
        Expr::State(name, _) => {
            names.insert(name.clone());
        }
        Expr::Add(left, right, _) | Expr::Binary { left, right, .. } => {
            collect_expression_state_names(left, names);
            collect_expression_state_names(right, names);
        }
        Expr::Contains {
            value, collection, ..
        } => {
            collect_expression_state_names(value, names);
            collect_expression_state_names(collection, names);
        }
        Expr::Not(value) | Expr::Await(value) => collect_expression_state_names(value, names),
        Expr::Index {
            collection, index, ..
        } => {
            collect_expression_state_names(collection, names);
            collect_expression_state_names(index, names);
        }
        Expr::Range {
            start, end, step, ..
        } => {
            collect_expression_state_names(start, names);
            collect_expression_state_names(end, names);
            if let Some(step) = step {
                collect_expression_state_names(step, names);
            }
        }
        Expr::Member { base, .. } => collect_expression_state_names(base, names),
        Expr::Coalesce(left, right) => {
            collect_expression_state_names(left, names);
            collect_expression_state_names(right, names);
        }
        Expr::Array(items) | Expr::Set(items) => {
            for item in items {
                collect_expression_state_names(item, names);
            }
        }
        Expr::Map(entries) => {
            for (key, value) in entries {
                collect_expression_state_names(key, names);
                collect_expression_state_names(value, names);
            }
        }
        Expr::Pair(first, second) => {
            collect_expression_state_names(first, names);
            collect_expression_state_names(second, names);
        }
        Expr::Triple(first, second, third) => {
            collect_expression_state_names(first, names);
            collect_expression_state_names(second, names);
            collect_expression_state_names(third, names);
        }
        Expr::Call { arguments, .. } => {
            for argument in arguments {
                collect_expression_state_names(argument, names);
            }
        }
        Expr::CollectionTransform {
            collection,
            initial,
            closure,
            ..
        } => {
            collect_expression_state_names(collection, names);
            if let Some(initial) = initial {
                collect_expression_state_names(initial, names);
            }
            collect_expression_state_names(closure, names);
        }
        Expr::Closure { parameters, body } => {
            let mut closure_names = HashSet::new();
            collect_expression_state_names(body, &mut closure_names);
            for name in closure_names {
                if !parameters.iter().any(|parameter| parameter == &name) {
                    names.insert(name);
                }
            }
        }
        Expr::NativeCall {
            receiver,
            arguments,
            ..
        } => {
            if let Some(receiver) = receiver {
                collect_expression_state_names(receiver, names);
            }
            for (_, argument) in arguments {
                collect_expression_state_names(argument, names);
            }
        }
        Expr::Interpolation(parts) => {
            for part in parts {
                if let InterpolatedPart::Value(value) = part {
                    collect_expression_state_names(value, names);
                }
            }
        }
        Expr::String(_)
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::EnumValue { .. }
        | Expr::Null(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight => {}
    }
}

fn is_pure_expression(expression: &Expr) -> bool {
    match expression {
        Expr::Call {
            arguments,
            is_async,
            ..
        } => !is_async && arguments.iter().all(is_pure_expression),
        Expr::CollectionTransform {
            collection,
            initial,
            closure,
            ..
        } => {
            is_pure_expression(collection)
                && initial.as_deref().is_none_or(is_pure_expression)
                && is_pure_expression(closure)
        }
        Expr::Closure { body, .. } => is_pure_expression(body),
        Expr::NativeCall { .. } => false,
        Expr::Await(_) => false,
        Expr::Add(left, right, _) | Expr::Binary { left, right, .. } => {
            is_pure_expression(left) && is_pure_expression(right)
        }
        Expr::Contains {
            value, collection, ..
        } => is_pure_expression(value) && is_pure_expression(collection),
        Expr::Not(value) => is_pure_expression(value),
        Expr::Index {
            collection, index, ..
        } => is_pure_expression(collection) && is_pure_expression(index),
        Expr::Range {
            start, end, step, ..
        } => {
            is_pure_expression(start)
                && is_pure_expression(end)
                && step.as_deref().is_none_or(is_pure_expression)
        }
        Expr::Member { base, .. } => is_pure_expression(base),
        Expr::Coalesce(left, right) => is_pure_expression(left) && is_pure_expression(right),
        Expr::Array(items) | Expr::Set(items) => items.iter().all(is_pure_expression),
        Expr::Map(entries) => entries
            .iter()
            .all(|(key, value)| is_pure_expression(key) && is_pure_expression(value)),
        Expr::Pair(first, second) => is_pure_expression(first) && is_pure_expression(second),
        Expr::Triple(first, second, third) => {
            is_pure_expression(first) && is_pure_expression(second) && is_pure_expression(third)
        }
        Expr::Interpolation(parts) => parts.iter().all(|part| match part {
            InterpolatedPart::Literal(_) => true,
            InterpolatedPart::Value(value) => is_pure_expression(value),
        }),
        Expr::String(_)
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::EnumValue { .. }
        | Expr::State(_, _)
        | Expr::Null(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight => true,
    }
}

fn prune_unused_functions(module: &mut Module) {
    let declared = module
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<HashSet<_>>();
    let mut used = HashSet::new();
    for state in &module.states {
        collect_expression_function_references(&state.initial, &declared, &mut used);
    }
    collect_node_function_references(&module.body, &declared, &mut used);
    if let Some(actions) = &module.on_appear {
        collect_action_function_references(actions, &declared, &mut used);
    }
    if let Some(actions) = &module.on_disappear {
        collect_action_function_references(actions, &declared, &mut used);
    }
    for actions in module
        .on_active
        .iter()
        .chain(module.on_inactive.iter())
        .chain(module.on_background.iter())
    {
        collect_action_function_references(actions, &declared, &mut used);
    }
    for screen in &module.screens {
        for state in &screen.states {
            collect_expression_function_references(&state.initial, &declared, &mut used);
        }
        collect_node_function_references(&screen.body, &declared, &mut used);
        if let Some(actions) = &screen.on_appear {
            collect_action_function_references(actions, &declared, &mut used);
        }
        if let Some(actions) = &screen.on_disappear {
            collect_action_function_references(actions, &declared, &mut used);
        }
    }
    for component in &module.components {
        collect_node_function_references(&component.body, &declared, &mut used);
        for state in &component.states {
            collect_expression_function_references(&state.initial, &declared, &mut used);
        }
    }

    let mut expanded = HashSet::new();
    let mut pending = used.iter().cloned().collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        if !expanded.insert(name.clone()) {
            continue;
        }
        let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name)
        else {
            continue;
        };
        for local in &function.locals {
            collect_expression_function_references(&local.initial, &declared, &mut used);
        }
        collect_expression_function_references(&function.body, &declared, &mut used);
        pending.extend(
            used.iter()
                .filter(|name| !expanded.contains(*name))
                .cloned(),
        );
    }
    module
        .functions
        .retain(|function| used.contains(function.name.as_str()));
}

fn collect_node_function_references(
    nodes: &[Node],
    declared: &HashSet<&str>,
    used: &mut HashSet<String>,
) {
    nexa_ir::walk::walk_ir(nodes, &mut |_| {}, &mut |expression| {
        if let Expr::Call { name, .. } = expression {
            if declared.contains(name.as_str()) {
                used.insert(name.clone());
            }
        }
    });
}

fn collect_expression_function_references(
    expression: &Expr,
    declared: &HashSet<&str>,
    used: &mut HashSet<String>,
) {
    nexa_ir::walk::walk_expression(expression, &mut |expression| {
        if let Expr::Call { name, .. } = expression {
            if declared.contains(name.as_str()) {
                used.insert(name.clone());
            }
        }
    });
}

fn collect_action_function_references(
    actions: &[Action],
    declared: &HashSet<&str>,
    used: &mut HashSet<String>,
) {
    for action in actions {
        match action {
            Action::Expression(expression) => {
                collect_expression_function_references(expression, declared, used)
            }
            Action::Assign { value, .. } => {
                collect_expression_function_references(value, declared, used)
            }
            Action::CollectionMutation { arguments, .. } => {
                for argument in arguments {
                    collect_expression_function_references(argument, declared, used);
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_expression_function_references(condition, declared, used);
                collect_action_function_references(then_branch, declared, used);
                if let Some(else_branch) = else_branch {
                    collect_action_function_references(else_branch, declared, used);
                }
            }
            Action::For { iterable, body, .. } => {
                collect_expression_function_references(iterable, declared, used);
                collect_action_function_references(body, declared, used);
            }
            Action::ForMap { iterable, body, .. } => {
                collect_expression_function_references(iterable, declared, used);
                collect_action_function_references(body, declared, used);
            }
            Action::While { condition, body } => {
                collect_expression_function_references(condition, declared, used);
                collect_action_function_references(body, declared, used);
            }
            Action::Break | Action::Continue => {}
        }
    }
}

fn prune_unused_states(module: &mut Module) {
    let mut used = HashSet::new();
    collect_node_state_references(&module.body, &mut used);
    if let Some(actions) = &module.on_appear {
        collect_action_state_references(actions, &mut used);
    }
    if let Some(actions) = &module.on_disappear {
        collect_action_state_references(actions, &mut used);
    }
    for actions in module
        .on_active
        .iter()
        .chain(module.on_inactive.iter())
        .chain(module.on_background.iter())
    {
        collect_action_state_references(actions, &mut used);
    }
    for screen in &module.screens {
        collect_node_state_references(&screen.body, &mut used);
        if let Some(actions) = &screen.on_appear {
            collect_action_state_references(actions, &mut used);
        }
        if let Some(actions) = &screen.on_disappear {
            collect_action_state_references(actions, &mut used);
        }
    }
    loop {
        let previous_len = used.len();
        for screen in &module.screens {
            for state in &screen.states {
                if used.contains(&state.name) {
                    collect_expression_state_references(&state.initial, &mut used);
                }
            }
        }
        if used.len() == previous_len {
            break;
        }
    }
    retain_referenced_states(&mut module.states, used.clone());
    for screen in &mut module.screens {
        retain_referenced_states(&mut screen.states, used.clone());
    }

    for component in &mut module.components {
        let mut used = HashSet::new();
        collect_node_state_references(&component.body, &mut used);
        retain_referenced_states(&mut component.states, used);
    }
}

fn prune_unused_structs(module: &mut Module) {
    let declarations = module
        .structs
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration))
        .collect::<std::collections::HashMap<_, _>>();
    let mut used = HashSet::new();
    for state in &module.states {
        collect_type_struct_names(&state.ty, &mut used);
        collect_expression_struct_names(&state.initial, &mut used);
    }
    for function in &module.functions {
        for parameter in &function.parameters {
            collect_type_struct_names(&parameter.ty, &mut used);
        }
        collect_type_struct_names(&function.return_type, &mut used);
        for local in &function.locals {
            collect_type_struct_names(&local.ty, &mut used);
            collect_expression_struct_names(&local.initial, &mut used);
        }
        collect_expression_struct_names(&function.body, &mut used);
    }
    for component in &module.components {
        for parameter in &component.parameters {
            collect_type_struct_names(&parameter.ty, &mut used);
        }
        for state in &component.states {
            collect_type_struct_names(&state.ty, &mut used);
            collect_expression_struct_names(&state.initial, &mut used);
        }
        collect_node_struct_names(&component.body, &mut used);
    }
    collect_node_struct_names(&module.body, &mut used);
    for screen in &module.screens {
        for state in &screen.states {
            collect_type_struct_names(&state.ty, &mut used);
            collect_expression_struct_names(&state.initial, &mut used);
        }
        collect_node_struct_names(&screen.body, &mut used);
        if let Some(actions) = &screen.on_appear {
            collect_action_struct_names(actions, &mut used);
        }
        if let Some(actions) = &screen.on_disappear {
            collect_action_struct_names(actions, &mut used);
        }
    }
    if let Some(actions) = &module.on_appear {
        collect_action_struct_names(actions, &mut used);
    }
    if let Some(actions) = &module.on_disappear {
        collect_action_struct_names(actions, &mut used);
    }
    for actions in module
        .on_active
        .iter()
        .chain(module.on_inactive.iter())
        .chain(module.on_background.iter())
    {
        collect_action_struct_names(actions, &mut used);
    }

    let mut pending = used.iter().cloned().collect::<Vec<_>>();
    let mut expanded = HashSet::new();
    while let Some(name) = pending.pop() {
        if !expanded.insert(name.clone()) {
            continue;
        }
        let Some(declaration) = declarations.get(name.as_str()) else {
            continue;
        };
        for field in &declaration.fields {
            collect_type_struct_names(&field.ty, &mut used);
        }
        pending.extend(
            used.iter()
                .filter(|field_name| !expanded.contains(*field_name))
                .cloned(),
        );
    }
    module
        .structs
        .retain(|declaration| used.contains(declaration.name.as_str()));
}

fn prune_unused_plugins(module: &mut Module) {
    let declared = module
        .plugins
        .iter()
        .map(|plugin| plugin.namespace.as_str())
        .collect::<HashSet<_>>();
    if declared.is_empty() {
        return;
    }
    let mut used = HashSet::new();
    let mut used_by_component = HashSet::new();
    let mut collect_node = |node: &nexa_ir::Node| {
        if let nexa_ir::Node::NativeComponentCall { namespace, .. } = node {
            if declared.contains(namespace.as_str()) {
                used_by_component.insert(namespace.clone());
            }
        }
    };
    let mut collect = |expression: &Expr| match expression {
        Expr::NativeCall { namespace, .. } if declared.contains(namespace.as_str()) => {
            used.insert(namespace.clone());
        }
        Expr::Call {
            return_type: nexa_ir::Type::Plugin { namespace, .. },
            ..
        } if declared.contains(namespace.as_str()) => {
            used.insert(namespace.clone());
        }
        _ => {}
    };
    for state in &module.states {
        nexa_ir::walk::walk_expression(&state.initial, &mut collect);
    }
    nexa_ir::walk::walk_ir(&module.body, &mut collect_node, &mut collect);
    if let Some(actions) = &module.on_appear {
        collect_action_plugin_references(actions, &mut collect);
    }
    if let Some(actions) = &module.on_disappear {
        collect_action_plugin_references(actions, &mut collect);
    }
    for actions in module
        .on_active
        .iter()
        .chain(module.on_inactive.iter())
        .chain(module.on_background.iter())
    {
        collect_action_plugin_references(actions, &mut collect);
    }
    for screen in &module.screens {
        for state in &screen.states {
            nexa_ir::walk::walk_expression(&state.initial, &mut collect);
        }
        nexa_ir::walk::walk_ir(&screen.body, &mut collect_node, &mut collect);
        if let Some(actions) = &screen.on_appear {
            collect_action_plugin_references(actions, &mut collect);
        }
        if let Some(actions) = &screen.on_disappear {
            collect_action_plugin_references(actions, &mut collect);
        }
    }
    for component in &module.components {
        nexa_ir::walk::walk_ir(&component.body, &mut collect_node, &mut collect);
        for state in &component.states {
            nexa_ir::walk::walk_expression(&state.initial, &mut collect);
        }
    }
    for function in &module.functions {
        for local in &function.locals {
            nexa_ir::walk::walk_expression(&local.initial, &mut collect);
        }
        nexa_ir::walk::walk_expression(&function.body, &mut collect);
    }
    used.extend(used_by_component);
    module
        .plugins
        .retain(|plugin| used.contains(plugin.namespace.as_str()));
}

fn collect_action_plugin_references(actions: &[Action], collect: &mut impl FnMut(&Expr)) {
    for action in actions {
        match action {
            Action::Expression(expression) => nexa_ir::walk::walk_expression(expression, collect),
            Action::Assign { value, .. } => nexa_ir::walk::walk_expression(value, collect),
            Action::CollectionMutation { arguments, .. } => {
                for argument in arguments {
                    nexa_ir::walk::walk_expression(argument, collect);
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                nexa_ir::walk::walk_expression(condition, collect);
                collect_action_plugin_references(then_branch, collect);
                if let Some(else_branch) = else_branch {
                    collect_action_plugin_references(else_branch, collect);
                }
            }
            Action::For { iterable, body, .. }
            | Action::ForMap { iterable, body, .. }
            | Action::While {
                condition: iterable,
                body,
            } => {
                nexa_ir::walk::walk_expression(iterable, collect);
                collect_action_plugin_references(body, collect);
            }
            Action::Break | Action::Continue => {}
        }
    }
}

fn collect_type_struct_names(ty: &nexa_ir::Type, used: &mut HashSet<String>) {
    match ty {
        nexa_ir::Type::Struct { name, fields } => {
            used.insert(name.clone());
            for (_, field) in fields {
                collect_type_struct_names(field, used);
            }
        }
        nexa_ir::Type::Optional(inner)
        | nexa_ir::Type::Array(inner)
        | nexa_ir::Type::Set(inner) => collect_type_struct_names(inner, used),
        nexa_ir::Type::Map(key, value) | nexa_ir::Type::Pair(key, value) => {
            collect_type_struct_names(key, used);
            collect_type_struct_names(value, used);
        }
        nexa_ir::Type::Triple(first, second, third) => {
            collect_type_struct_names(first, used);
            collect_type_struct_names(second, used);
            collect_type_struct_names(third, used);
        }
        nexa_ir::Type::Void
        | nexa_ir::Type::String
        | nexa_ir::Type::Bool
        | nexa_ir::Type::Numeric(_)
        | nexa_ir::Type::Enum(_)
        | nexa_ir::Type::Plugin { .. }
        | nexa_ir::Type::NetworkResponse => {}
    }
}

fn collect_expression_struct_names(expression: &Expr, used: &mut HashSet<String>) {
    nexa_ir::walk::walk_expression(expression, &mut |expression| {
        collect_expression_type_struct_names(expression, used)
    });
}

fn collect_expression_type_struct_names(expression: &Expr, used: &mut HashSet<String>) {
    match expression {
        Expr::State(_, ty) | Expr::Null(ty) => collect_type_struct_names(ty, used),
        Expr::Call { return_type, .. } => collect_type_struct_names(return_type, used),
        Expr::NativeCall { return_type, .. } => collect_type_struct_names(return_type, used),
        Expr::Index {
            collection_type,
            element_type,
            ..
        } => {
            collect_type_struct_names(collection_type, used);
            collect_type_struct_names(element_type, used);
        }
        Expr::Member {
            base_type,
            field_type,
            ..
        } => {
            collect_type_struct_names(base_type, used);
            collect_type_struct_names(field_type, used);
        }
        Expr::Contains {
            collection_type, ..
        } => collect_type_struct_names(collection_type, used),
        Expr::CollectionTransform { .. } | Expr::Closure { .. } => {}
        Expr::String(_)
        | Expr::Interpolation(_)
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::EnumValue { .. }
        | Expr::Add(_, _, _)
        | Expr::Not(_)
        | Expr::Binary { .. }
        | Expr::Array(_)
        | Expr::Set(_)
        | Expr::Map(_)
        | Expr::Pair(_, _)
        | Expr::Triple(_, _, _)
        | Expr::Range { .. }
        | Expr::Coalesce(_, _)
        | Expr::Await(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight => {}
    }
}

fn collect_node_struct_names(nodes: &[Node], used: &mut HashSet<String>) {
    nexa_ir::walk::walk_ir(nodes, &mut |_| {}, &mut |expression| {
        collect_expression_type_struct_names(expression, used)
    });
}

fn collect_action_struct_names(actions: &[Action], used: &mut HashSet<String>) {
    for action in actions {
        match action {
            Action::Expression(expression) => collect_expression_struct_names(expression, used),
            Action::Assign { value, .. } => collect_expression_struct_names(value, used),
            Action::CollectionMutation { arguments, .. } => {
                for argument in arguments {
                    collect_expression_struct_names(argument, used);
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_expression_struct_names(condition, used);
                collect_action_struct_names(then_branch, used);
                if let Some(else_branch) = else_branch {
                    collect_action_struct_names(else_branch, used);
                }
            }
            Action::For { iterable, body, .. }
            | Action::ForMap { iterable, body, .. }
            | Action::While {
                condition: iterable,
                body,
            } => {
                collect_expression_struct_names(iterable, used);
                collect_action_struct_names(body, used);
            }
            Action::Break | Action::Continue => {}
        }
    }
}

fn retain_referenced_states(states: &mut Vec<nexa_ir::State>, mut used: HashSet<String>) {
    loop {
        let previous_len = used.len();
        let initializers = states
            .iter()
            .filter(|state| used.contains(&state.name))
            .map(|state| state.initial.clone())
            .collect::<Vec<_>>();
        for initializer in &initializers {
            collect_expression_state_references(initializer, &mut used);
        }
        if used.len() == previous_len {
            break;
        }
    }
    states.retain(|state| used.contains(&state.name));
}

fn collect_node_state_references(nodes: &[Node], used: &mut HashSet<String>) {
    let mut bindings = Vec::new();
    nexa_ir::walk::walk_ir(
        nodes,
        &mut |node| match node {
            Node::Button { actions, .. } => {
                collect_action_bindings(actions, &mut bindings);
            }
            Node::Pressable {
                actions,
                long_press_actions,
                ..
            } => {
                collect_action_bindings(actions, &mut bindings);
                collect_action_bindings(long_press_actions, &mut bindings);
            }
            Node::RefreshControl { state, actions, .. } => {
                collect_action_bindings(actions, &mut bindings);
                bindings.push(state.clone());
            }
            Node::TextInput {
                state,
                focused,
                actions,
                ..
            } => {
                collect_action_bindings(actions, &mut bindings);
                bindings.push(state.clone());
                if let Some(focused) = focused {
                    bindings.push(focused.clone());
                }
            }
            Node::Switch { state, .. }
            | Node::BottomSheet { state, .. }
            | Node::AppBottomBar { state, .. } => {
                bindings.push(state.clone());
            }
            Node::FastList {
                on_end_reached,
                on_scroll,
                refresh,
                scroll_position,
                ..
            } => {
                if let Some(scroll_position) = scroll_position {
                    bindings.push(scroll_position.clone());
                }
                if let Some(actions) = on_end_reached {
                    collect_action_bindings(actions, &mut bindings);
                }
                if let Some(actions) = on_scroll {
                    collect_action_bindings(actions, &mut bindings);
                }
                if let Some(refresh) = refresh {
                    bindings.push(refresh.state.clone());
                    collect_action_bindings(&refresh.actions, &mut bindings);
                }
            }
            _ => {}
        },
        &mut |expression| {
            collect_expression_state_references(expression, used);
        },
    );
    used.extend(bindings);
}

fn collect_action_bindings(actions: &[Action], used: &mut Vec<String>) {
    for action in actions {
        match action {
            Action::Expression(expression) => {
                nexa_ir::walk::walk_expression(expression, &mut |expression| {
                    if let Expr::State(name, _) = expression {
                        used.push(name.clone());
                    }
                });
            }
            Action::Assign { name, .. } => used.push(name.clone()),
            Action::CollectionMutation { name, .. } => used.push(name.clone()),
            Action::If {
                then_branch,
                else_branch,
                ..
            } => {
                collect_action_bindings(then_branch, used);
                if let Some(else_branch) = else_branch {
                    collect_action_bindings(else_branch, used);
                }
            }
            Action::For { body, .. } | Action::While { body, .. } => {
                collect_action_bindings(body, used);
            }
            Action::ForMap { body, .. } => collect_action_bindings(body, used),
            Action::Break | Action::Continue => {}
        }
    }
}

fn collect_action_state_references(actions: &[Action], used: &mut HashSet<String>) {
    for action in actions {
        match action {
            Action::Expression(expression) => collect_expression_state_references(expression, used),
            Action::Assign { name, value } => {
                used.insert(name.clone());
                collect_expression_state_references(value, used);
            }
            Action::CollectionMutation {
                name, arguments, ..
            } => {
                used.insert(name.clone());
                for argument in arguments {
                    collect_expression_state_references(argument, used);
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_expression_state_references(condition, used);
                collect_action_state_references(then_branch, used);
                if let Some(else_branch) = else_branch {
                    collect_action_state_references(else_branch, used);
                }
            }
            Action::For { iterable, body, .. } => {
                collect_expression_state_references(iterable, used);
                collect_action_state_references(body, used);
            }
            Action::ForMap { iterable, body, .. } => {
                collect_expression_state_references(iterable, used);
                collect_action_state_references(body, used);
            }
            Action::While { condition, body } => {
                collect_expression_state_references(condition, used);
                collect_action_state_references(body, used);
            }
            Action::Break | Action::Continue => {}
        }
    }
}

fn collect_expression_state_references(expression: &Expr, used: &mut HashSet<String>) {
    collect_expression_state_names(expression, used);
}

fn optimize_nodes(nodes: Vec<Node>) -> Vec<Node> {
    let mut optimized = Vec::with_capacity(nodes.len());
    for node in nodes {
        match optimize_node(node) {
            Some(node) => optimized.push(node),
            None => {}
        }
    }
    optimized
}

fn optimize_node(node: Node) -> Option<Node> {
    match node {
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => {
            let children = optimize_nodes(children);
            if children.len() == 1
                && spacing == 0.0
                && style.alignment.is_none()
                && !style.has_modifiers()
            {
                return children.into_iter().next();
            }
            Some(Node::Layout {
                kind,
                spacing,
                style,
                children,
            })
        }
        Node::Text { value, style } => Some(Node::Text {
            value: fold_expression(value),
            style,
        }),
        Node::Button {
            label,
            icon,
            loading,
            disabled,
            actions,
        } => Some(Node::Button {
            label: fold_expression(label),
            icon,
            loading: loading
                .map(fold_expression)
                .filter(|loading| !matches!(loading, Expr::Bool(false))),
            disabled: disabled
                .map(fold_expression)
                .filter(|disabled| !matches!(disabled, Expr::Bool(false))),
            actions: optimize_actions(actions),
        }),
        Node::Pressable {
            disabled,
            haptic,
            children,
            actions,
            long_press_actions,
        } => Some(Node::Pressable {
            disabled: fold_expression(disabled),
            haptic,
            children: optimize_nodes(children),
            actions: optimize_actions(actions),
            long_press_actions: optimize_actions(long_press_actions),
        }),
        Node::FastList {
            source,
            axis,
            item_extent,
            index,
            item,
            key,
            section,
            children,
            on_end_reached,
            on_scroll,
            sticky_header,
            section_header,
            refresh,
            scroll_position,
        } => Some(Node::FastList {
            source: optimize_list_source(source),
            axis,
            item_extent,
            index,
            item,
            key: key.map(fold_expression),
            children: optimize_nodes(children),
            on_end_reached: on_end_reached.map(optimize_actions),
            on_scroll: on_scroll.map(optimize_actions),
            sticky_header: sticky_header.map(optimize_nodes),
            section_header: section_header.map(optimize_nodes),
            scroll_position,
            section,
            refresh: refresh.map(|refresh| nexa_ir::FastListRefresh {
                state: refresh.state,
                actions: optimize_actions(refresh.actions),
            }),
        }),
        Node::If {
            condition,
            then_body,
            else_body,
        } => {
            let condition = fold_expression(condition);
            let then_body = optimize_nodes(then_body);
            let else_body = else_body.map(optimize_nodes);
            match condition {
                Expr::Bool(true) => collapse_nodes(then_body),
                Expr::Bool(false) => collapse_nodes(else_body.unwrap_or_default()),
                condition => Some(Node::If {
                    condition,
                    then_body,
                    else_body,
                }),
            }
        }
        Node::When {
            value,
            cases,
            else_body,
        } => Some(Node::When {
            value: fold_expression(value),
            cases: cases
                .into_iter()
                .map(|case| nexa_ir::WhenCase {
                    value: fold_expression(case.value),
                    body: optimize_nodes(case.body),
                })
                .collect(),
            else_body: optimize_nodes(else_body),
        }),
        Node::ComponentCall {
            name,
            arguments,
            children,
        } => Some(Node::ComponentCall {
            name,
            arguments: arguments
                .into_iter()
                .map(|(name, value)| (name, fold_expression(value)))
                .collect(),
            children: children.map(optimize_nodes),
        }),
        Node::NativeComponentCall {
            namespace,
            name,
            arguments,
            children,
        } => Some(Node::NativeComponentCall {
            namespace,
            name,
            arguments: arguments
                .into_iter()
                .map(|(name, value)| (name, fold_expression(value)))
                .collect(),
            children: children.map(optimize_nodes),
        }),
        Node::TextInput {
            state,
            placeholder,
            keyboard,
            secure,
            multiline,
            autocorrect,
            capitalization,
            focused,
            max_length,
            actions,
        } => Some(Node::TextInput {
            state,
            placeholder,
            keyboard,
            secure,
            multiline,
            autocorrect,
            capitalization,
            focused,
            max_length,
            actions: optimize_actions(actions),
        }),
        Node::Content => Some(Node::Content),
        Node::NavigationStack { root, arguments } => Some(Node::NavigationStack {
            root,
            arguments: arguments.into_iter().map(fold_expression).collect(),
        }),
        node @ (Node::Switch { .. }
        | Node::Image { .. }
        | Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::NavigationBack { .. }) => Some(node),
        Node::OnAppear {
            actions,
            asynchronous,
        } => Some(Node::OnAppear {
            actions: optimize_actions(actions),
            asynchronous,
        }),
        Node::OnDisappear { actions } => Some(Node::OnDisappear {
            actions: optimize_actions(actions),
        }),
        Node::OnActive { actions } => Some(Node::OnActive {
            actions: optimize_actions(actions),
        }),
        Node::OnInactive { actions } => Some(Node::OnInactive {
            actions: optimize_actions(actions),
        }),
        Node::OnBackground { actions } => Some(Node::OnBackground {
            actions: optimize_actions(actions),
        }),
        Node::NavigationLink {
            destination,
            arguments,
            guard,
            children,
        } => Some(Node::NavigationLink {
            destination,
            arguments: arguments.into_iter().map(fold_expression).collect(),
            guard: guard.and_then(|guard| match fold_expression(guard) {
                Expr::Bool(true) => None,
                guard => Some(guard),
            }),
            children: optimize_nodes(children),
        }),
        Node::Link { url, children } => Some(Node::Link {
            url,
            children: optimize_nodes(children),
        }),
        Node::Accessibility {
            label,
            hint,
            role,
            children,
        } => Some(Node::Accessibility {
            label,
            hint: hint.map(fold_expression),
            role,
            children: optimize_nodes(children),
        }),
        Node::KeyboardAware { dismiss, children } => Some(Node::KeyboardAware {
            dismiss,
            children: optimize_nodes(children),
        }),
        Node::BottomSheet {
            state,
            partial,
            children,
        } => Some(Node::BottomSheet {
            state,
            partial,
            children: optimize_nodes(children),
        }),
        Node::RefreshControl {
            state,
            children,
            actions,
        } => Some(Node::RefreshControl {
            state,
            children: optimize_nodes(children),
            actions: optimize_actions(actions),
        }),
        Node::AppBottomBar { state, tabs } => Some(Node::AppBottomBar {
            state,
            tabs: tabs
                .into_iter()
                .map(|tab| nexa_ir::BottomBarTab {
                    index: tab.index,
                    label: tab.label,
                    icon: tab.icon,
                    badge: tab.badge,
                    children: optimize_nodes(tab.children),
                })
                .collect(),
        }),
    }
}

fn collapse_nodes(nodes: Vec<Node>) -> Option<Node> {
    match nodes.len() {
        0 => None,
        1 => Some(nodes.into_iter().next().expect("length checked")),
        _ => Some(Node::Layout {
            kind: LayoutKind::Column,
            spacing: 0.0,
            style: ViewStyle::default(),
            children: nodes,
        }),
    }
}

fn optimize_list_source(source: ListSource) -> ListSource {
    match source {
        ListSource::Count(count) => ListSource::Count(fold_expression(count)),
        ListSource::Items {
            collection,
            element_type,
        } => ListSource::Items {
            collection: fold_expression(collection),
            element_type,
        },
        ListSource::Sections {
            collection,
            element_type,
        } => ListSource::Sections {
            collection: fold_expression(collection),
            element_type,
        },
    }
}

fn optimize_actions(actions: Vec<Action>) -> Vec<Action> {
    let mut optimized = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            Action::Expression(expression) => {
                optimized.push(Action::Expression(fold_expression(expression)))
            }
            Action::Assign { name, value } => optimized.push(Action::Assign {
                name,
                value: fold_expression(value),
            }),
            Action::CollectionMutation {
                name,
                operation,
                arguments,
            } => optimized.push(Action::CollectionMutation {
                name,
                operation,
                arguments: arguments.into_iter().map(fold_expression).collect(),
            }),
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = fold_expression(condition);
                let then_branch = optimize_actions(then_branch);
                let else_branch = else_branch.map(optimize_actions);
                match condition {
                    Expr::Bool(true) => optimized.extend(then_branch),
                    Expr::Bool(false) => optimized.extend(else_branch.unwrap_or_default()),
                    condition => optimized.push(Action::If {
                        condition,
                        then_branch,
                        else_branch,
                    }),
                }
            }
            Action::For {
                name,
                iterable,
                body,
            } => optimized.push(Action::For {
                name,
                iterable: fold_expression(iterable),
                body: optimize_actions(body),
            }),
            Action::ForMap {
                key_name,
                value_name,
                iterable,
                body,
            } => optimized.push(Action::ForMap {
                key_name,
                value_name,
                iterable: fold_expression(iterable),
                body: optimize_actions(body),
            }),
            Action::While { condition, body } => optimized.push(Action::While {
                condition: fold_expression(condition),
                body: optimize_actions(body),
            }),
            Action::Break => optimized.push(Action::Break),
            Action::Continue => optimized.push(Action::Continue),
        }
    }
    optimized
}

fn fold_expression(expression: Expr) -> Expr {
    match expression {
        Expr::Not(value) => match fold_expression(*value) {
            Expr::Bool(value) => Expr::Bool(!value),
            value => Expr::Not(Box::new(value)),
        },
        Expr::Binary { op, left, right } => {
            let left = fold_expression(*left);
            let right = fold_expression(*right);
            match (op, &left, &right) {
                (BinaryOp::And, Expr::Bool(false), _) => Expr::Bool(false),
                (BinaryOp::And, Expr::Bool(true), _) => right,
                (BinaryOp::Or, Expr::Bool(true), _) => Expr::Bool(true),
                (BinaryOp::Or, Expr::Bool(false), _) => right,
                _ => evaluate_binary(op, &left, &right).unwrap_or(Expr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }),
            }
        }
        Expr::Contains {
            value,
            collection,
            collection_type,
        } => {
            let value = fold_expression(*value);
            let collection = fold_expression(*collection);
            evaluate_contains(&value, &collection, &collection_type).unwrap_or_else(|| {
                Expr::Contains {
                    value: Box::new(value),
                    collection: Box::new(collection),
                    collection_type,
                }
            })
        }
        Expr::Add(left, right, ty) => {
            let left = fold_expression(*left);
            let right = fold_expression(*right);
            fold_numeric_add(&left, &right, ty)
                .unwrap_or_else(|| Expr::Add(Box::new(left), Box::new(right), ty))
        }
        Expr::Array(values) => Expr::Array(values.into_iter().map(fold_expression).collect()),
        Expr::Set(values) => Expr::Set(values.into_iter().map(fold_expression).collect()),
        Expr::Map(entries) => Expr::Map(
            entries
                .into_iter()
                .map(|(key, value)| (fold_expression(key), fold_expression(value)))
                .collect(),
        ),
        Expr::Pair(first, second) => Expr::Pair(
            Box::new(fold_expression(*first)),
            Box::new(fold_expression(*second)),
        ),
        Expr::Triple(first, second, third) => Expr::Triple(
            Box::new(fold_expression(*first)),
            Box::new(fold_expression(*second)),
            Box::new(fold_expression(*third)),
        ),
        Expr::Call {
            name,
            arguments,
            return_type,
            is_async,
            is_constructor,
        } => Expr::Call {
            name,
            arguments: arguments.into_iter().map(fold_expression).collect(),
            return_type,
            is_async,
            is_constructor,
        },
        Expr::CollectionTransform {
            operation,
            collection,
            initial,
            closure,
        } => Expr::CollectionTransform {
            operation,
            collection: Box::new(fold_expression(*collection)),
            initial: initial.map(|initial| Box::new(fold_expression(*initial))),
            closure: Box::new(fold_expression(*closure)),
        },
        Expr::Closure { parameters, body } => Expr::Closure {
            parameters,
            body: Box::new(fold_expression(*body)),
        },
        Expr::Index {
            collection,
            index,
            optional,
            collection_type,
            element_type,
        } => Expr::Index {
            collection: Box::new(fold_expression(*collection)),
            index: Box::new(fold_expression(*index)),
            optional,
            collection_type,
            element_type,
        },
        Expr::Range {
            start,
            end,
            inclusive,
            step,
        } => Expr::Range {
            start: Box::new(fold_expression(*start)),
            end: Box::new(fold_expression(*end)),
            inclusive,
            step: step.map(|step| Box::new(fold_expression(*step))),
        },
        Expr::Member {
            base,
            name,
            optional,
            base_type,
            field_type,
        } => Expr::Member {
            base: Box::new(fold_expression(*base)),
            name,
            optional,
            base_type,
            field_type,
        },
        Expr::Null(ty) => Expr::Null(ty),
        Expr::Coalesce(left, right) => {
            let left = fold_expression(*left);
            let right = fold_expression(*right);
            if matches!(left, Expr::Null(_)) {
                right
            } else {
                Expr::Coalesce(Box::new(left), Box::new(right))
            }
        }
        Expr::Await(value) => Expr::Await(Box::new(fold_expression(*value))),
        Expr::Interpolation(parts) => Expr::Interpolation(
            parts
                .into_iter()
                .map(|part| match part {
                    InterpolatedPart::Literal(value) => InterpolatedPart::Literal(value),
                    InterpolatedPart::Value(value) => {
                        InterpolatedPart::Value(Box::new(fold_expression(*value)))
                    }
                })
                .collect(),
        ),
        expression => expression,
    }
}

fn fold_numeric_add(left: &Expr, right: &Expr, ty: NumericType) -> Option<Expr> {
    let Expr::Number {
        raw: left_raw,
        ty: left_ty,
    } = left
    else {
        return None;
    };
    let Expr::Number {
        raw: right_raw,
        ty: right_ty,
    } = right
    else {
        return None;
    };
    if *left_ty != ty || *right_ty != ty {
        return None;
    }
    let raw = match ty {
        NumericType::Int8 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            8,
        )
        .to_string(),
        NumericType::Int16 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            16,
        )
        .to_string(),
        NumericType::Int32 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            32,
        )
        .to_string(),
        NumericType::Int64 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            64,
        )
        .to_string(),
        NumericType::UInt8 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            8,
        )
        .to_string(),
        NumericType::UInt16 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            16,
        )
        .to_string(),
        NumericType::UInt32 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            32,
        )
        .to_string(),
        NumericType::UInt64 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            64,
        )
        .to_string(),
        NumericType::Float32 => {
            let value = left_raw.parse::<f32>().ok()? + right_raw.parse::<f32>().ok()?;
            if !value.is_finite() {
                return None;
            }
            value.to_string()
        }
        NumericType::Float64 => {
            let value = left_raw.parse::<f64>().ok()? + right_raw.parse::<f64>().ok()?;
            if !value.is_finite() {
                return None;
            }
            value.to_string()
        }
    };
    if raw.contains('e') || raw.contains('E') {
        return None;
    }
    Some(Expr::Number { raw, ty })
}

fn wrap_signed(value: i128, bits: u32) -> i128 {
    let modulus = 1_i128 << bits;
    let half = 1_i128 << (bits - 1);
    let wrapped = value.rem_euclid(modulus);
    if wrapped >= half {
        wrapped - modulus
    } else {
        wrapped
    }
}

fn wrap_unsigned(value: u128, bits: u32) -> u128 {
    value % (1_u128 << bits)
}

fn evaluate_binary(op: BinaryOp, left: &Expr, right: &Expr) -> Option<Expr> {
    match (op, left, right) {
        (BinaryOp::Equal, Expr::String(left), Expr::String(right)) => {
            Some(Expr::Bool(left == right))
        }
        (BinaryOp::NotEqual, Expr::String(left), Expr::String(right)) => {
            Some(Expr::Bool(left != right))
        }
        (BinaryOp::Equal, Expr::Bool(left), Expr::Bool(right)) => Some(Expr::Bool(left == right)),
        (BinaryOp::NotEqual, Expr::Bool(left), Expr::Bool(right)) => {
            Some(Expr::Bool(left != right))
        }
        (op @ (BinaryOp::Equal | BinaryOp::NotEqual), left, right)
        | (
            op
            @ (BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual),
            left,
            right,
        ) => {
            let (left, right) = numeric_constants(left, right)?;
            let value = match op {
                BinaryOp::Equal => left == right,
                BinaryOp::NotEqual => left != right,
                BinaryOp::Less => left < right,
                BinaryOp::LessEqual => left <= right,
                BinaryOp::Greater => left > right,
                BinaryOp::GreaterEqual => left >= right,
                BinaryOp::And | BinaryOp::Or | BinaryOp::Contains => return None,
            };
            Some(Expr::Bool(value))
        }
        _ => None,
    }
}

fn evaluate_contains(
    value: &Expr,
    collection: &Expr,
    collection_type: &nexa_ir::Type,
) -> Option<Expr> {
    let matches = match (collection_type, collection) {
        (
            nexa_ir::Type::Array(_) | nexa_ir::Type::Set(_),
            Expr::Array(items) | Expr::Set(items),
        ) => items.iter().any(|item| scalar_equal(value, item)),
        (nexa_ir::Type::Map(_, _), Expr::Map(entries)) => {
            entries.iter().any(|(key, _)| scalar_equal(value, key))
        }
        _ => return None,
    };
    Some(Expr::Bool(matches))
}

fn scalar_equal(left: &Expr, right: &Expr) -> bool {
    match (left, right) {
        (Expr::String(left), Expr::String(right)) => left == right,
        (Expr::Bool(left), Expr::Bool(right)) => left == right,
        (
            Expr::Number {
                raw: left_raw,
                ty: left_ty,
            },
            Expr::Number {
                raw: right_raw,
                ty: right_ty,
            },
        ) if left_ty == right_ty => match left_ty {
            NumericType::Int8 | NumericType::Int16 | NumericType::Int32 | NumericType::Int64 => {
                left_raw.parse::<i128>().ok() == right_raw.parse::<i128>().ok()
            }
            NumericType::UInt8
            | NumericType::UInt16
            | NumericType::UInt32
            | NumericType::UInt64 => {
                left_raw.parse::<u128>().ok() == right_raw.parse::<u128>().ok()
            }
            NumericType::Float32 | NumericType::Float64 => {
                left_raw.parse::<f64>().ok() == right_raw.parse::<f64>().ok()
            }
        },
        _ => false,
    }
}

fn numeric_constants(left: &Expr, right: &Expr) -> Option<(ConstantNumber, ConstantNumber)> {
    let Expr::Number {
        raw: left_raw,
        ty: left_ty,
    } = left
    else {
        return None;
    };
    let Expr::Number {
        raw: right_raw,
        ty: right_ty,
    } = right
    else {
        return None;
    };
    if left_ty != right_ty {
        return None;
    }
    Some((
        ConstantNumber::parse(left_raw, *left_ty)?,
        ConstantNumber::parse(right_raw, *right_ty)?,
    ))
}

#[derive(PartialEq, PartialOrd)]
enum ConstantNumber {
    Signed(i128),
    Unsigned(u128),
    Float(f64),
}

impl ConstantNumber {
    fn parse(raw: &str, ty: NumericType) -> Option<Self> {
        Some(match ty {
            NumericType::Int8 | NumericType::Int16 | NumericType::Int32 | NumericType::Int64 => {
                Self::Signed(raw.parse().ok()?)
            }
            NumericType::UInt8
            | NumericType::UInt16
            | NumericType::UInt32
            | NumericType::UInt64 => Self::Unsigned(raw.parse().ok()?),
            NumericType::Float32 => Self::Float(raw.parse::<f32>().ok()? as f64),
            NumericType::Float64 => Self::Float(raw.parse().ok()?),
        })
    }
}
