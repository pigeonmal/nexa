use std::collections::HashSet;

use nexa_ir::walk::{IrFolder, IrVisitor, fold_expr_children, fold_node_children};
use nexa_ir::{Action, BinaryOp, Expr, LayoutKind, Module, Node, NumericType, ViewStyle};

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
    // Both backends wrap a multi-node app body in an implicit
    // `Column { spacing: 0 }` before rendering it, so two adjacent
    // zero-spaced plain columns at the top level merge into exactly that
    // wrapper and the wrapper disappears with them.
    module.body = hoist_nested_layouts(
        optimize_nodes(std::mem::take(&mut module.body)),
        LayoutKind::Column,
        0.0,
        &ViewStyle::default(),
    );
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

struct StateNameCollector<'a> {
    names: &'a mut HashSet<String>,
}

impl IrVisitor for StateNameCollector<'_> {
    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::State(name, _) => {
                self.names.insert(name.clone());
            }
            Expr::Closure { parameters, body } => {
                let mut closure_names = HashSet::new();
                let mut inner = StateNameCollector {
                    names: &mut closure_names,
                };
                inner.visit_expr(body);
                for name in closure_names {
                    if !parameters.iter().any(|parameter| parameter == &name) {
                        self.names.insert(name);
                    }
                }
                return;
            }
            _ => {}
        }
        nexa_ir::walk::walk_expr_children(expr, self);
    }
}

fn collect_expression_state_names(expression: &Expr, names: &mut HashSet<String>) {
    StateNameCollector { names }.visit_expr(expression);
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
        Expr::NativeCall { .. }
        | Expr::NetworkFetch(_)
        | Expr::NetworkDownload { .. }
        | Expr::PathJoin { .. }
        | Expr::FileExists { .. }
        | Expr::FileReadText { .. }
        | Expr::FileWriteText { .. }
        | Expr::FileDelete { .. }
        | Expr::PermissionOp { .. } => false,
        Expr::Await(_) | Expr::TryAwait(_) | Expr::Try { .. } => false,
        Expr::Add(left, right, _) | Expr::Binary { left, right, .. } => {
            is_pure_expression(left) && is_pure_expression(right)
        }
        Expr::Contains {
            value, collection, ..
        } => is_pure_expression(value) && is_pure_expression(collection),
        Expr::Not(value) => is_pure_expression(value),
        Expr::ResultOk { value, .. } => is_pure_expression(value),
        Expr::ResultErr { error, .. } => is_pure_expression(error),
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
            nexa_ir::InterpolatedPart::Literal(_) => true,
            nexa_ir::InterpolatedPart::Value(value) => is_pure_expression(value),
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

struct FunctionRefCollector<'a> {
    declared: &'a HashSet<&'a str>,
    used: &'a mut HashSet<String>,
}

impl IrVisitor for FunctionRefCollector<'_> {
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Call { name, .. } = expr
            && self.declared.contains(name.as_str())
        {
            self.used.insert(name.clone());
        }
        nexa_ir::walk::walk_expr_children(expr, self);
    }
}

fn collect_node_function_references(
    nodes: &[Node],
    declared: &HashSet<&str>,
    used: &mut HashSet<String>,
) {
    FunctionRefCollector { declared, used }.visit_nodes(nodes);
}

fn collect_expression_function_references(
    expression: &Expr,
    declared: &HashSet<&str>,
    used: &mut HashSet<String>,
) {
    FunctionRefCollector { declared, used }.visit_expr(expression);
}

fn collect_action_function_references(
    actions: &[Action],
    declared: &HashSet<&str>,
    used: &mut HashSet<String>,
) {
    FunctionRefCollector { declared, used }.visit_actions(actions);
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
                .filter(|name| !expanded.contains(*name))
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
    let mut used = HashSet::new();
    let mut used_by_component = HashSet::new();
    let mut collect_node = |node: &nexa_ir::Node| {
        if let nexa_ir::Node::NativeComponentCall { namespace, .. } = node
            && declared.contains(namespace.as_str())
        {
            used_by_component.insert(namespace.clone());
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
        nexa_ir::walk::walk_actions(actions, &mut collect);
    }
    if let Some(actions) = &module.on_disappear {
        nexa_ir::walk::walk_actions(actions, &mut collect);
    }
    for actions in module
        .on_active
        .iter()
        .chain(module.on_inactive.iter())
        .chain(module.on_background.iter())
    {
        nexa_ir::walk::walk_actions(actions, &mut collect);
    }
    for screen in &module.screens {
        for state in &screen.states {
            nexa_ir::walk::walk_expression(&state.initial, &mut collect);
        }
        nexa_ir::walk::walk_ir(&screen.body, &mut collect_node, &mut collect);
        if let Some(actions) = &screen.on_appear {
            nexa_ir::walk::walk_actions(actions, &mut collect);
        }
        if let Some(actions) = &screen.on_disappear {
            nexa_ir::walk::walk_actions(actions, &mut collect);
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
        nexa_ir::Type::Map(key, value)
        | nexa_ir::Type::Pair(key, value)
        | nexa_ir::Type::Result(key, value) => {
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

struct StructNameCollector<'a> {
    used: &'a mut HashSet<String>,
}

impl IrVisitor for StructNameCollector<'_> {
    fn visit_expr(&mut self, expr: &Expr) {
        collect_expression_type_struct_names(expr, self.used);
        nexa_ir::walk::walk_expr_children(expr, self);
    }
}

fn collect_expression_struct_names(expression: &Expr, used: &mut HashSet<String>) {
    StructNameCollector { used }.visit_expr(expression);
}

fn collect_expression_type_struct_names(expression: &Expr, used: &mut HashSet<String>) {
    match expression {
        Expr::State(_, ty) | Expr::Null(ty) => collect_type_struct_names(ty, used),
        Expr::Call { return_type, .. } => collect_type_struct_names(return_type, used),
        Expr::NativeCall { return_type, .. } => collect_type_struct_names(return_type, used),
        Expr::NetworkFetch(_)
        | Expr::NetworkDownload { .. }
        | Expr::PathJoin { .. }
        | Expr::FileExists { .. }
        | Expr::FileReadText { .. }
        | Expr::FileWriteText { .. }
        | Expr::FileDelete { .. }
        | Expr::PermissionOp { .. } => {}
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
        | Expr::TryAwait(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight => {}
        Expr::ResultOk {
            value_type,
            error_type,
            ..
        }
        | Expr::ResultErr {
            value_type,
            error_type,
            ..
        }
        | Expr::Try {
            value_type,
            error_type,
            ..
        } => {
            collect_type_struct_names(value_type, used);
            collect_type_struct_names(error_type, used);
        }
    }
}

fn collect_node_struct_names(nodes: &[Node], used: &mut HashSet<String>) {
    StructNameCollector { used }.visit_nodes(nodes);
}

fn collect_action_struct_names(actions: &[Action], used: &mut HashSet<String>) {
    StructNameCollector { used }.visit_actions(actions);
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
            Node::FastList { plan } => {
                if let Some(scroll_position) = plan.scroll_position() {
                    bindings.push(scroll_position.to_owned());
                }
                if let Some(actions) = plan.on_end_reached() {
                    collect_action_bindings(actions, &mut bindings);
                }
                if let Some(actions) = plan.on_scroll() {
                    collect_action_bindings(actions, &mut bindings);
                }
                if let Some(refresh) = plan.refresh() {
                    bindings.push(refresh.state.clone());
                    collect_action_bindings(&refresh.actions, &mut bindings);
                }
            }
            Node::NativeComponentCall { event_handlers, .. } => {
                for handler in event_handlers {
                    collect_action_bindings(&handler.actions, &mut bindings);
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

struct ActionBindingCollector<'a> {
    used: &'a mut Vec<String>,
}

impl IrVisitor for ActionBindingCollector<'_> {
    fn visit_action(&mut self, action: &Action) {
        match action {
            Action::Assign { name, .. } | Action::CollectionMutation { name, .. } => {
                self.used.push(name.clone());
            }
            _ => {}
        }
        nexa_ir::walk::walk_action_children(action, self);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::State(name, _) = expr {
            self.used.push(name.clone());
        }
        nexa_ir::walk::walk_expr_children(expr, self);
    }
}

fn collect_action_bindings(actions: &[Action], used: &mut Vec<String>) {
    ActionBindingCollector { used }.visit_actions(actions);
}

struct ActionStateRefCollector<'a> {
    used: &'a mut HashSet<String>,
}

impl IrVisitor for ActionStateRefCollector<'_> {
    fn visit_action(&mut self, action: &Action) {
        match action {
            Action::Assign { name, .. } | Action::CollectionMutation { name, .. } => {
                self.used.insert(name.clone());
            }
            _ => {}
        }
        nexa_ir::walk::walk_action_children(action, self);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        collect_expression_state_names(expr, self.used);
    }
}

fn collect_action_state_references(actions: &[Action], used: &mut HashSet<String>) {
    ActionStateRefCollector { used }.visit_actions(actions);
}

fn collect_expression_state_references(expression: &Expr, used: &mut HashSet<String>) {
    collect_expression_state_names(expression, used);
}

struct IrOptimizer;

impl IrFolder for IrOptimizer {
    fn fold_node(&mut self, node: Node) -> Option<Node> {
        match node {
            Node::Layout {
                kind,
                spacing,
                style,
                children,
            } => {
                let children =
                    hoist_nested_layouts(self.fold_nodes(children), kind, spacing, &style);
                if children.len() == 1 && spacing == 0.0 && is_plain_style(&style) {
                    return children.into_iter().next();
                }
                Some(Node::Layout {
                    kind,
                    spacing,
                    style,
                    children,
                })
            }
            Node::Button {
                label,
                icon,
                loading,
                disabled,
                actions,
            } => Some(Node::Button {
                label: self.fold_expr(label),
                icon,
                loading: loading
                    .map(|loading| self.fold_expr(loading))
                    .filter(|loading| !matches!(loading, Expr::Bool(false))),
                disabled: disabled
                    .map(|disabled| self.fold_expr(disabled))
                    .filter(|disabled| !matches!(disabled, Expr::Bool(false))),
                actions: self.fold_actions(actions),
            }),
            Node::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.fold_expr(condition);
                let then_body = self.fold_nodes(then_body);
                let else_body = else_body.map(|body| self.fold_nodes(body));
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
            Node::NavigationLink {
                destination,
                arguments,
                guard,
                children,
            } => Some(Node::NavigationLink {
                destination,
                arguments: arguments
                    .into_iter()
                    .map(|arg| self.fold_expr(arg))
                    .collect(),
                guard: guard.and_then(|guard| match self.fold_expr(guard) {
                    Expr::Bool(true) => None,
                    guard => Some(guard),
                }),
                children: self.fold_nodes(children),
            }),
            other => fold_node_children(other, self),
        }
    }

    fn fold_actions(&mut self, actions: Vec<Action>) -> Vec<Action> {
        let mut optimized = Vec::with_capacity(actions.len());
        for action in actions {
            match action {
                Action::If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    let condition = self.fold_expr(condition);
                    let then_branch = self.fold_actions(then_branch);
                    let else_branch = else_branch.map(|branch| self.fold_actions(branch));
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
                other => {
                    if let Some(action) = self.fold_action(other) {
                        optimized.push(action);
                    }
                }
            }
        }
        optimized
    }

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        let folded = fold_expr_children(expr, self);
        fold_expression_rules(folded)
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

/// Whether a layout style contributes nothing of its own.
///
/// A style with an alignment positions its children on the cross axis, and a
/// style with modifiers draws something the layout is responsible for. Folding
/// a layout whose style is neither moves the grandchildren up one level, which
/// is only invisible when the layout they are moving out of was itself
/// invisible.
fn is_plain_style(style: &ViewStyle) -> bool {
    style.alignment.is_none() && !style.has_modifiers()
}

/// Hoists any child layout that `parent` exactly subsumes.
///
/// `Column { spacing: s, A, Column { spacing: s, B, C }, D }` places B and C
/// at exactly the offsets `Column { spacing: s, A, B, C, D }` does, so the
/// inner column is a view both layout engines measure for nothing. Requiring
/// equal spacings is what makes that identity hold: an inner layout with a
/// different spacing produces non-uniform gaps that no single spacing value
/// reproduces, so those stay nested.
///
/// Both layouts must also be plain. The inner one because its alignment would
/// be lost, and the outer one because its alignment would otherwise start
/// applying to the grandchildren instead of to the layout they are leaving.
fn hoist_nested_layouts(
    children: Vec<Node>,
    kind: LayoutKind,
    spacing: f32,
    style: &ViewStyle,
) -> Vec<Node> {
    if !is_plain_style(style) {
        return children;
    }
    let mut hoisted: Vec<Node> = Vec::with_capacity(children.len());
    for child in children {
        match subsumed_layout_children(&child, kind, spacing) {
            Some(inner) => hoisted.extend(inner.iter().cloned()),
            None => hoisted.push(child),
        }
    }
    hoisted
}

/// The grandchildren a parent layout takes over, when it takes any.
fn subsumed_layout_children(node: &Node, kind: LayoutKind, spacing: f32) -> Option<&[Node]> {
    let Node::Layout {
        kind: child_kind,
        spacing: child_spacing,
        style,
        children,
    } = node
    else {
        return None;
    };
    if *child_kind != kind || *child_spacing != spacing || !is_plain_style(style) {
        return None;
    }
    Some(children)
}

fn fold_expression(expression: Expr) -> Expr {
    IrOptimizer.fold_expr(expression)
}

fn optimize_nodes(nodes: Vec<Node>) -> Vec<Node> {
    IrOptimizer.fold_nodes(nodes)
}

fn optimize_actions(actions: Vec<Action>) -> Vec<Action> {
    IrOptimizer.fold_actions(actions)
}

fn fold_expression_rules(expression: Expr) -> Expr {
    match expression {
        Expr::Not(value) => match *value {
            Expr::Bool(value) => Expr::Bool(!value),
            value => Expr::Not(Box::new(value)),
        },
        Expr::Binary { op, left, right } => match (op, &*left, &*right) {
            (BinaryOp::And, Expr::Bool(false), _) => Expr::Bool(false),
            (BinaryOp::And, Expr::Bool(true), _) => *right,
            (BinaryOp::Or, Expr::Bool(true), _) => Expr::Bool(true),
            (BinaryOp::Or, Expr::Bool(false), _) => *right,
            _ => evaluate_binary(op, &left, &right).unwrap_or(Expr::Binary { op, left, right }),
        },
        Expr::Contains {
            value,
            collection,
            collection_type,
        } => evaluate_contains(&value, &collection, &collection_type).unwrap_or({
            Expr::Contains {
                value,
                collection,
                collection_type,
            }
        }),
        Expr::Add(left, right, ty) => {
            fold_numeric_add(&left, &right, ty).unwrap_or(Expr::Add(left, right, ty))
        }
        Expr::Coalesce(left, right) => {
            if matches!(*left, Expr::Null(_)) {
                *right
            } else {
                Expr::Coalesce(left, right)
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use nexa_ir::Alignment;

    /// Every layout in a body, depth first, as `(kind, spacing)`.
    ///
    /// A flattening bug is a change in the *shape* of the tree, so these tests
    /// compare shapes rather than whole nodes. The IR deliberately has no
    /// `PartialEq`: structural equality over a whole module is not a question
    /// anything needs to ask, and deriving it would push the requirement down
    /// through every expression and action type.
    fn layout_shape(nodes: &[Node]) -> Vec<(LayoutKind, f32)> {
        let mut shapes = Vec::new();
        for node in nodes {
            if let Node::Layout {
                kind,
                spacing,
                children,
                ..
            } = node
            {
                shapes.push((*kind, *spacing));
                shapes.extend(layout_shape(children));
            }
        }
        shapes
    }

    /// The literal text in a body, in traversal order.
    fn text_under(nodes: &[Node]) -> Vec<String> {
        let mut texts = Vec::new();
        for node in nodes {
            match node {
                Node::Text {
                    value: Expr::String(value),
                    ..
                } => texts.push(value.clone()),
                Node::Layout { children, .. } => texts.extend(text_under(children)),
                _ => {}
            }
        }
        texts
    }

    fn text(value: &str) -> Node {
        Node::Text {
            value: Expr::String(value.to_owned()),
            style: Default::default(),
        }
    }

    fn column(spacing: f32, children: Vec<Node>) -> Node {
        Node::Layout {
            kind: LayoutKind::Column,
            spacing,
            style: ViewStyle::default(),
            children,
        }
    }

    /// A column that draws something of its own.
    fn padded_column(spacing: f32, padding: f32, children: Vec<Node>) -> Node {
        Node::Layout {
            kind: LayoutKind::Column,
            spacing,
            style: ViewStyle {
                padding: Some(padding),
                ..ViewStyle::default()
            },
            children,
        }
    }

    fn module_with_body(body: Vec<Node>) -> Module {
        Module {
            app_name: "FlattenApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
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

    fn optimized_body(body: Vec<Node>) -> Vec<Node> {
        let mut module = module_with_body(body);
        optimize(&mut module);
        module.body
    }

    #[test]
    fn nested_columns_with_equal_spacing_collapse_into_one_layout() {
        let body = optimized_body(vec![column(
            8.0,
            vec![
                text("a"),
                column(8.0, vec![text("b"), text("c")]),
                text("d"),
            ],
        )]);

        assert_eq!(
            layout_shape(&body),
            vec![(LayoutKind::Column, 8.0)],
            "the inner 8-spaced column is gone"
        );
        assert_eq!(text_under(&body), ["a", "b", "c", "d"]);
    }

    #[test]
    fn a_run_of_three_matching_columns_collapses_in_one_pass() {
        let body = optimized_body(vec![column(
            0.0,
            vec![
                column(0.0, vec![text("a"), text("b")]),
                column(0.0, vec![text("c"), text("d")]),
                column(0.0, vec![text("e"), text("f")]),
            ],
        )]);

        assert!(
            layout_shape(&body).is_empty(),
            "the zero-spaced column the backends would supply replaces all three: {:?}",
            layout_shape(&body)
        );
        assert_eq!(text_under(&body), ["a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn differing_spacings_keep_the_nested_layout() {
        // An inner spacing of 4 inside an outer spacing of 8 produces
        // non-uniform gaps that no single spacing value can reproduce.
        let body = optimized_body(vec![column(
            8.0,
            vec![text("a"), column(4.0, vec![text("b"), text("c")])],
        )]);

        assert_eq!(
            layout_shape(&body),
            vec![(LayoutKind::Column, 8.0), (LayoutKind::Column, 4.0)],
            "collapsing these would change the gap between a and b"
        );
    }

    #[test]
    fn a_styled_inner_layout_is_not_merged() {
        // Its padding is drawn by the layout itself, so hoisting its children
        // up would pad each of them separately.
        let body = optimized_body(vec![column(
            0.0,
            vec![
                text("a"),
                padded_column(0.0, 12.0, vec![text("b"), text("c")]),
            ],
        )]);

        assert_eq!(
            layout_shape(&body),
            vec![(LayoutKind::Column, 0.0)],
            "only the padded layout survives"
        );
    }

    #[test]
    fn an_aligned_parent_does_not_take_over_its_childs_children() {
        // The parent's alignment would otherwise start applying to the texts,
        // which the inner layout was previously positioning.
        let mut module = module_with_body(vec![Node::Layout {
            kind: LayoutKind::Column,
            spacing: 0.0,
            style: ViewStyle {
                alignment: Some(Alignment::Center),
                ..ViewStyle::default()
            },
            children: vec![text("a"), column(0.0, vec![text("b"), text("c")])],
        }]);
        optimize(&mut module);

        assert_eq!(
            layout_shape(&module.body),
            vec![(LayoutKind::Column, 0.0), (LayoutKind::Column, 0.0)],
            "the inner column keeps positioning its own children"
        );
    }

    #[test]
    fn an_aligned_inner_layout_does_not_lose_its_alignment() {
        let body = optimized_body(vec![column(
            0.0,
            vec![
                text("a"),
                Node::Layout {
                    kind: LayoutKind::Column,
                    spacing: 0.0,
                    style: ViewStyle {
                        alignment: Some(Alignment::End),
                        ..ViewStyle::default()
                    },
                    children: vec![text("b"), text("c")],
                },
            ],
        )]);

        assert_eq!(
            layout_shape(&body),
            vec![(LayoutKind::Column, 0.0)],
            "the trailing-aligned column must stay, or b and c move"
        );
    }

    #[test]
    fn a_row_never_absorbs_a_column() {
        let body = optimized_body(vec![Node::Layout {
            kind: LayoutKind::Row,
            spacing: 0.0,
            style: ViewStyle::default(),
            children: vec![
                column(0.0, vec![text("a"), text("b")]),
                column(0.0, vec![text("c"), text("d")]),
            ],
        }]);

        assert_eq!(
            layout_shape(&body).len(),
            3,
            "a column positions its children along the other axis"
        );
    }

    #[test]
    fn adjacent_top_level_columns_collapse_into_the_implicit_wrapper() {
        // Both backends wrap a multi-node body in a zero-spaced column, so two
        // top-level zero-spaced columns merge into exactly that wrapper.
        let body = optimized_body(vec![
            column(0.0, vec![text("a"), text("b")]),
            column(0.0, vec![text("c"), text("d")]),
        ]);

        assert!(layout_shape(&body).is_empty(), "{:?}", layout_shape(&body));
        assert_eq!(text_under(&body), ["a", "b", "c", "d"]);
    }
}
