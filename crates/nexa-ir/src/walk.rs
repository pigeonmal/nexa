use crate::{
    Action, BottomBarTab, ErrorCatchArm, Expr, FastListRefresh, InterpolatedPart, ListCommon,
    ListPlan, NativeComponentEventHandler, NetworkRequest, Node, SectionedListCommon, WhenCase,
};

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
            Node::NavigationLink {
                arguments,
                guard,
                children,
                ..
            } => {
                for argument in arguments {
                    walk_expression(argument, visit_expression);
                }
                if let Some(guard) = guard {
                    walk_expression(guard, visit_expression);
                }
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
            Node::OnAppear { actions, .. }
            | Node::OnDisappear { actions }
            | Node::OnActive { actions }
            | Node::OnInactive { actions }
            | Node::OnBackground { actions } => walk_actions(actions, visit_expression),
            Node::AppBottomBar { tabs, .. } => {
                for tab in tabs {
                    walk_ir(&tab.children, visit_node, visit_expression);
                }
            }
            Node::FastList { plan } => {
                walk_list_plan(plan, visit_expression);
                if let Some(key) = plan.key() {
                    walk_expression(key, visit_expression);
                }
                if let Some(actions) = plan.on_end_reached() {
                    walk_actions(actions, visit_expression);
                }
                if let Some(actions) = plan.on_scroll() {
                    walk_actions(actions, visit_expression);
                }
                if let Some(refresh) = plan.refresh() {
                    walk_actions(&refresh.actions, visit_expression);
                }
                if let Some(sticky_header) = plan.sticky_header() {
                    walk_ir(sticky_header, visit_node, visit_expression);
                }
                if let Some(section_header) = plan.section_header() {
                    walk_ir(section_header, visit_node, visit_expression);
                }
                walk_ir(plan.children(), visit_node, visit_expression);
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
            Node::NativeComponentCall {
                arguments,
                children,
                event_handlers,
                ..
            } => {
                for (_, argument) in arguments {
                    walk_expression(argument, visit_expression);
                }
                for handler in event_handlers {
                    walk_actions(&handler.actions, visit_expression);
                }
                if let Some(children) = children {
                    walk_ir(children, visit_node, visit_expression);
                }
            }
            Node::TextInput { actions, .. } => walk_actions(actions, visit_expression),
            Node::Content
            | Node::Switch { .. }
            | Node::StatusBar { .. }
            | Node::Direction { .. } => {}
            Node::NavigationStack { arguments, .. } => {
                for argument in arguments {
                    walk_expression(argument, visit_expression);
                }
            }
            Node::NavigationBack { label } => walk_expression(label, visit_expression),
            Node::Image { source, .. } => {
                if let crate::ImageSource::RemoteUrl(url) = source {
                    walk_expression(url, visit_expression);
                }
            }
        }
    }
}

/// Returns whether any node in the tree satisfies `predicate`.
///
/// Semantic validation uses this shared traversal for placement checks so all
/// nested slots (including list headers, tab bodies, and component content)
/// receive the same coverage as backend feature analysis.
pub fn any_node(nodes: &[Node], mut predicate: impl FnMut(&Node) -> bool) -> bool {
    let mut found = false;
    walk_ir(
        nodes,
        &mut |node| {
            found |= predicate(node);
        },
        &mut |_| {},
    );
    found
}

/// Visits each event/action callback in a node tree without building an
/// intermediate collection. Nested native-event subscriptions are reported as
/// their own callback in addition to the enclosing action callback.
pub fn walk_callback_actions(nodes: &[Node], visit: &mut impl FnMut(&[Action])) {
    walk_ir(
        nodes,
        &mut |node| {
            let actions = match node {
                Node::OnAppear { actions, .. }
                | Node::OnDisappear { actions }
                | Node::OnActive { actions }
                | Node::OnInactive { actions }
                | Node::OnBackground { actions }
                | Node::Button { actions, .. }
                | Node::TextInput { actions, .. } => Some(actions.as_slice()),
                Node::Pressable {
                    actions,
                    long_press_actions,
                    ..
                } => {
                    visit(actions);
                    visit(long_press_actions);
                    walk_nested_callback_actions(actions, visit);
                    walk_nested_callback_actions(long_press_actions, visit);
                    None
                }
                Node::RefreshControl { actions, .. } => Some(actions.as_slice()),
                Node::FastList { plan } => {
                    if let Some(actions) = plan.on_end_reached() {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(actions) = plan.on_scroll() {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(refresh) = plan.refresh() {
                        visit(&refresh.actions);
                        walk_nested_callback_actions(&refresh.actions, visit);
                    }
                    None
                }
                Node::NativeComponentCall { event_handlers, .. } => {
                    for handler in event_handlers {
                        visit(&handler.actions);
                        walk_nested_callback_actions(&handler.actions, visit);
                    }
                    None
                }
                _ => None,
            };
            if let Some(actions) = actions {
                visit(actions);
                walk_nested_callback_actions(actions, visit);
            }
        },
        &mut |_| {},
    );
}

fn walk_nested_callback_actions(actions: &[Action], visit: &mut impl FnMut(&[Action])) {
    for action in actions {
        match action {
            Action::NativeEventSubscribe { actions, .. } => {
                visit(actions);
                walk_nested_callback_actions(actions, visit);
            }
            Action::If {
                then_branch,
                else_branch,
                ..
            } => {
                walk_nested_callback_actions(then_branch, visit);
                if let Some(else_branch) = else_branch {
                    walk_nested_callback_actions(else_branch, visit);
                }
            }
            Action::For { body, .. } | Action::ForMap { body, .. } | Action::While { body, .. } => {
                walk_nested_callback_actions(body, visit)
            }
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                walk_nested_callback_actions(body, visit);
                for arm in error_catches {
                    walk_nested_callback_actions(&arm.body, visit);
                }
                if let Some(catch_body) = catch_body {
                    walk_nested_callback_actions(catch_body, visit);
                }
            }
            Action::Expression(_)
            | Action::Assign { .. }
            | Action::NativePropertyAssign { .. }
            | Action::CollectionMutation { .. }
            | Action::Break
            | Action::Continue => {}
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
        Expr::NativeCall {
            receiver,
            arguments,
            ..
        } => {
            if let Some(receiver) = receiver {
                walk_expression(receiver, visit);
            }
            for (_, argument) in arguments {
                walk_expression(argument, visit);
            }
        }
        Expr::NetworkFetch(request) => walk_network_request(request, visit),
        Expr::NetworkDownload {
            destination,
            request,
        } => {
            walk_expression(destination, visit);
            walk_network_request(request, visit);
        }
        Expr::PathJoin { path, component } => {
            walk_expression(path, visit);
            walk_expression(component, visit);
        }
        Expr::FileExists { path } | Expr::FileReadText { path } | Expr::FileDelete { path } => {
            walk_expression(path, visit);
        }
        Expr::FileWriteText { path, contents } => {
            walk_expression(path, visit);
            walk_expression(contents, visit);
        }
        Expr::PermissionOp { permission, .. } => {
            walk_expression(permission, visit);
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
        Expr::Await(value) | Expr::TryAwait(value) => walk_expression(value, visit),
        Expr::ResultOk { value, .. } => walk_expression(value, visit),
        Expr::ResultErr { error, .. } => walk_expression(error, visit),
        Expr::Try { expr, .. } => walk_expression(expr, visit),
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
        | Node::NavigationBack { .. }
        | Node::ComponentCall { .. }
        | Node::NativeComponentCall { .. }
        | Node::Content
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. } => false,
    })
}

fn walk_network_request_children(request: &NetworkRequest, visit: &mut impl FnMut(&Expr)) {
    visit(&request.url);
    visit(&request.method);
    visit(&request.headers);
    visit(&request.timeout);
    visit(&request.use_cache);
    visit(&request.follow_redirects);
    visit(&request.max_response_bytes);
    visit(&request.certificate_pins);
    if let Some(body) = &request.body {
        visit(body);
    }
}

fn walk_network_request(request: &NetworkRequest, visit: &mut impl FnMut(&Expr)) {
    walk_network_request_children(request, &mut |expr| walk_expression(expr, visit));
}

fn walk_list_plan_children(plan: &ListPlan, visit: &mut impl FnMut(&Expr)) {
    match plan {
        ListPlan::Count { count, .. } => visit(count),
        ListPlan::Items { collection, .. } | ListPlan::Sections { collection, .. } => {
            visit(collection);
        }
    }
}

fn walk_list_plan(plan: &ListPlan, visit: &mut impl FnMut(&Expr)) {
    walk_list_plan_children(plan, &mut |expr| walk_expression(expr, visit));
}

pub fn walk_actions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    for action in actions {
        match action {
            Action::Expression(expression) => walk_expression(expression, visit),
            Action::Assign { value, .. } => walk_expression(value, visit),
            Action::NativePropertyAssign {
                receiver, value, ..
            } => {
                walk_expression(receiver, visit);
                walk_expression(value, visit);
            }
            Action::NativeEventSubscribe {
                receiver, actions, ..
            } => {
                walk_expression(receiver, visit);
                walk_actions(actions, visit);
            }
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
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                walk_actions(body, visit);
                for arm in error_catches {
                    walk_actions(&arm.body, visit);
                }
                if let Some(catch_body) = catch_body {
                    walk_actions(catch_body, visit);
                }
            }
            Action::Break | Action::Continue => {}
        }
    }
}

/// Visits expressions executed by one action callback. The receiver of a
/// nested native-event subscription is included, but its handler body is a
/// separate callback and is not traversed here.
pub fn walk_callback_expressions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    for action in actions {
        match action {
            Action::Expression(expression) => walk_expression(expression, visit),
            Action::Assign { value, .. } => walk_expression(value, visit),
            Action::NativePropertyAssign {
                receiver, value, ..
            } => {
                walk_expression(receiver, visit);
                walk_expression(value, visit);
            }
            Action::NativeEventSubscribe { receiver, .. } => walk_expression(receiver, visit),
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
                walk_callback_expressions(then_branch, visit);
                if let Some(else_branch) = else_branch {
                    walk_callback_expressions(else_branch, visit);
                }
            }
            Action::For { iterable, body, .. } => {
                walk_expression(iterable, visit);
                walk_callback_expressions(body, visit);
            }
            Action::ForMap { iterable, body, .. } => {
                walk_expression(iterable, visit);
                walk_callback_expressions(body, visit);
            }
            Action::While { condition, body } => {
                walk_expression(condition, visit);
                walk_callback_expressions(body, visit);
            }
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                walk_callback_expressions(body, visit);
                for arm in error_catches {
                    walk_callback_expressions(&arm.body, visit);
                }
                if let Some(catch_body) = catch_body {
                    walk_callback_expressions(catch_body, visit);
                }
            }
            Action::Break | Action::Continue => {}
        }
    }
}

/// Trait for immutable traversal over IR trees.
pub trait IrVisitor: Sized {
    fn visit_node(&mut self, node: &Node) {
        walk_node_children(node, self);
    }

    fn visit_nodes(&mut self, nodes: &[Node]) {
        for node in nodes {
            self.visit_node(node);
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr_children(expr, self);
    }

    fn visit_action(&mut self, action: &Action) {
        walk_action_children(action, self);
    }

    fn visit_actions(&mut self, actions: &[Action]) {
        for action in actions {
            self.visit_action(action);
        }
    }
}

/// Trait for transforming / folding IR trees.
pub trait IrFolder: Sized {
    fn fold_node(&mut self, node: Node) -> Option<Node> {
        fold_node_children(node, self)
    }

    fn fold_nodes(&mut self, nodes: Vec<Node>) -> Vec<Node> {
        nodes
            .into_iter()
            .filter_map(|node| self.fold_node(node))
            .collect()
    }

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        fold_expr_children(expr, self)
    }

    fn fold_action(&mut self, action: Action) -> Option<Action> {
        fold_action_children(action, self)
    }

    fn fold_actions(&mut self, actions: Vec<Action>) -> Vec<Action> {
        actions
            .into_iter()
            .filter_map(|action| self.fold_action(action))
            .collect()
    }
}

pub fn walk_node_children<V: IrVisitor>(node: &Node, visitor: &mut V) {
    match node {
        Node::Layout { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. } => visitor.visit_nodes(children),
        Node::Pressable {
            disabled,
            children,
            actions,
            long_press_actions,
            ..
        } => {
            visitor.visit_expr(disabled);
            visitor.visit_actions(actions);
            visitor.visit_actions(long_press_actions);
            visitor.visit_nodes(children);
        }
        Node::Link { url, children } => {
            visitor.visit_expr(url);
            visitor.visit_nodes(children);
        }
        Node::NavigationLink {
            arguments,
            guard,
            children,
            ..
        } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
            if let Some(guard) = guard {
                visitor.visit_expr(guard);
            }
            visitor.visit_nodes(children);
        }
        Node::Accessibility {
            label, children, ..
        } => {
            visitor.visit_expr(label);
            visitor.visit_nodes(children);
        }
        Node::RefreshControl {
            children, actions, ..
        } => {
            visitor.visit_actions(actions);
            visitor.visit_nodes(children);
        }
        Node::OnAppear { actions, .. }
        | Node::OnDisappear { actions }
        | Node::OnActive { actions }
        | Node::OnInactive { actions }
        | Node::OnBackground { actions } => visitor.visit_actions(actions),
        Node::AppBottomBar { tabs, .. } => {
            for tab in tabs {
                visitor.visit_nodes(&tab.children);
            }
        }
        Node::FastList { plan } => {
            walk_list_plan_children(plan, &mut |e| visitor.visit_expr(e));
            if let Some(key) = plan.key() {
                visitor.visit_expr(key);
            }
            if let Some(actions) = plan.on_end_reached() {
                visitor.visit_actions(actions);
            }
            if let Some(actions) = plan.on_scroll() {
                visitor.visit_actions(actions);
            }
            if let Some(refresh) = plan.refresh() {
                visitor.visit_actions(&refresh.actions);
            }
            if let Some(sticky_header) = plan.sticky_header() {
                visitor.visit_nodes(sticky_header);
            }
            if let Some(section_header) = plan.section_header() {
                visitor.visit_nodes(section_header);
            }
            visitor.visit_nodes(plan.children());
        }
        Node::If {
            condition,
            then_body,
            else_body,
        } => {
            visitor.visit_expr(condition);
            visitor.visit_nodes(then_body);
            if let Some(else_body) = else_body {
                visitor.visit_nodes(else_body);
            }
        }
        Node::When {
            value,
            cases,
            else_body,
        } => {
            visitor.visit_expr(value);
            for case in cases {
                visitor.visit_expr(&case.value);
                visitor.visit_nodes(&case.body);
            }
            visitor.visit_nodes(else_body);
        }
        Node::Text { value, .. } => visitor.visit_expr(value),
        Node::Button {
            label,
            loading,
            disabled,
            actions,
            ..
        } => {
            visitor.visit_expr(label);
            if let Some(loading) = loading {
                visitor.visit_expr(loading);
            }
            if let Some(disabled) = disabled {
                visitor.visit_expr(disabled);
            }
            visitor.visit_actions(actions);
        }
        Node::ComponentCall {
            arguments,
            children,
            ..
        } => {
            for (_, argument) in arguments {
                visitor.visit_expr(argument);
            }
            if let Some(children) = children {
                visitor.visit_nodes(children);
            }
        }
        Node::NativeComponentCall {
            arguments,
            children,
            event_handlers,
            ..
        } => {
            for (_, argument) in arguments {
                visitor.visit_expr(argument);
            }
            for handler in event_handlers {
                visitor.visit_actions(&handler.actions);
            }
            if let Some(children) = children {
                visitor.visit_nodes(children);
            }
        }
        Node::TextInput { actions, .. } => visitor.visit_actions(actions),
        Node::Content | Node::Switch { .. } | Node::StatusBar { .. } | Node::Direction { .. } => {}
        Node::NavigationStack { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Node::NavigationBack { label } => visitor.visit_expr(label),
        Node::Image { source, .. } => {
            if let crate::ImageSource::RemoteUrl(url) = source {
                visitor.visit_expr(url);
            }
        }
    }
}

pub fn walk_expr_children<V: IrVisitor>(expr: &Expr, visitor: &mut V) {
    match expr {
        Expr::Add(left, right, _) | Expr::Binary { left, right, .. } => {
            visitor.visit_expr(left);
            visitor.visit_expr(right);
        }
        Expr::Contains {
            value, collection, ..
        } => {
            visitor.visit_expr(value);
            visitor.visit_expr(collection);
        }
        Expr::Not(value) => visitor.visit_expr(value),
        Expr::Array(items) | Expr::Set(items) => {
            for item in items {
                visitor.visit_expr(item);
            }
        }
        Expr::Map(entries) => {
            for (key, value) in entries {
                visitor.visit_expr(key);
                visitor.visit_expr(value);
            }
        }
        Expr::Pair(first, second) => {
            visitor.visit_expr(first);
            visitor.visit_expr(second);
        }
        Expr::Triple(first, second, third) => {
            visitor.visit_expr(first);
            visitor.visit_expr(second);
            visitor.visit_expr(third);
        }
        Expr::Call { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Expr::CollectionTransform {
            collection,
            initial,
            closure,
            ..
        } => {
            visitor.visit_expr(collection);
            if let Some(initial) = initial {
                visitor.visit_expr(initial);
            }
            visitor.visit_expr(closure);
        }
        Expr::Closure { body, .. } => visitor.visit_expr(body),
        Expr::NativeCall {
            receiver,
            arguments,
            ..
        } => {
            if let Some(receiver) = receiver {
                visitor.visit_expr(receiver);
            }
            for (_, argument) in arguments {
                visitor.visit_expr(argument);
            }
        }
        Expr::NetworkFetch(request) => {
            walk_network_request_children(request, &mut |e| visitor.visit_expr(e));
        }
        Expr::NetworkDownload {
            destination,
            request,
        } => {
            visitor.visit_expr(destination);
            walk_network_request_children(request, &mut |e| visitor.visit_expr(e));
        }
        Expr::PathJoin { path, component } => {
            visitor.visit_expr(path);
            visitor.visit_expr(component);
        }
        Expr::FileExists { path } | Expr::FileReadText { path } | Expr::FileDelete { path } => {
            visitor.visit_expr(path);
        }
        Expr::FileWriteText { path, contents } => {
            visitor.visit_expr(path);
            visitor.visit_expr(contents);
        }
        Expr::PermissionOp { permission, .. } => {
            visitor.visit_expr(permission);
        }
        Expr::Index {
            collection, index, ..
        } => {
            visitor.visit_expr(collection);
            visitor.visit_expr(index);
        }
        Expr::Range {
            start, end, step, ..
        } => {
            visitor.visit_expr(start);
            visitor.visit_expr(end);
            if let Some(step) = step {
                visitor.visit_expr(step);
            }
        }
        Expr::Member { base, .. } => visitor.visit_expr(base),
        Expr::Coalesce(left, right) => {
            visitor.visit_expr(left);
            visitor.visit_expr(right);
        }
        Expr::Await(value) | Expr::TryAwait(value) => visitor.visit_expr(value),
        Expr::ResultOk { value, .. } => visitor.visit_expr(value),
        Expr::ResultErr { error, .. } => visitor.visit_expr(error),
        Expr::Try { expr, .. } => visitor.visit_expr(expr),
        Expr::Interpolation(parts) => {
            for part in parts {
                if let InterpolatedPart::Value(value) = part {
                    visitor.visit_expr(value);
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

pub fn walk_action_children<V: IrVisitor>(action: &Action, visitor: &mut V) {
    match action {
        Action::Expression(expr) => visitor.visit_expr(expr),
        Action::Assign { value, .. } => visitor.visit_expr(value),
        Action::NativePropertyAssign {
            receiver, value, ..
        } => {
            visitor.visit_expr(receiver);
            visitor.visit_expr(value);
        }
        Action::NativeEventSubscribe {
            receiver, actions, ..
        } => {
            visitor.visit_expr(receiver);
            visitor.visit_actions(actions);
        }
        Action::CollectionMutation { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Action::If {
            condition,
            then_branch,
            else_branch,
        } => {
            visitor.visit_expr(condition);
            visitor.visit_actions(then_branch);
            if let Some(else_branch) = else_branch {
                visitor.visit_actions(else_branch);
            }
        }
        Action::For { iterable, body, .. } => {
            visitor.visit_expr(iterable);
            visitor.visit_actions(body);
        }
        Action::ForMap { iterable, body, .. } => {
            visitor.visit_expr(iterable);
            visitor.visit_actions(body);
        }
        Action::While { condition, body } => {
            visitor.visit_expr(condition);
            visitor.visit_actions(body);
        }
        Action::TryCatch {
            body,
            error_catches,
            catch_body,
        } => {
            visitor.visit_actions(body);
            for arm in error_catches {
                visitor.visit_actions(&arm.body);
            }
            if let Some(catch_body) = catch_body {
                visitor.visit_actions(catch_body);
            }
        }
        Action::Break | Action::Continue => {}
    }
}

pub fn fold_node_children<F: IrFolder>(node: Node, folder: &mut F) -> Option<Node> {
    match node {
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => Some(Node::Layout {
            kind,
            spacing,
            style,
            children: folder.fold_nodes(children),
        }),
        Node::KeyboardAware { dismiss, children } => Some(Node::KeyboardAware {
            dismiss,
            children: folder.fold_nodes(children),
        }),
        Node::BottomSheet {
            state,
            partial,
            children,
        } => Some(Node::BottomSheet {
            state,
            partial,
            children: folder.fold_nodes(children),
        }),
        Node::Pressable {
            disabled,
            haptic,
            children,
            actions,
            long_press_actions,
        } => Some(Node::Pressable {
            disabled: folder.fold_expr(disabled),
            haptic,
            children: folder.fold_nodes(children),
            actions: folder.fold_actions(actions),
            long_press_actions: folder.fold_actions(long_press_actions),
        }),
        Node::Link { url, children } => Some(Node::Link {
            url: folder.fold_expr(url),
            children: folder.fold_nodes(children),
        }),
        Node::NavigationLink {
            destination,
            arguments,
            guard,
            children,
        } => Some(Node::NavigationLink {
            destination,
            arguments: arguments
                .into_iter()
                .map(|arg| folder.fold_expr(arg))
                .collect(),
            guard: guard.map(|g| folder.fold_expr(g)),
            children: folder.fold_nodes(children),
        }),
        Node::Accessibility {
            label,
            hint,
            role,
            children,
        } => Some(Node::Accessibility {
            label: folder.fold_expr(label),
            hint: hint.map(|h| folder.fold_expr(h)),
            role,
            children: folder.fold_nodes(children),
        }),
        Node::RefreshControl {
            state,
            children,
            actions,
        } => Some(Node::RefreshControl {
            state,
            children: folder.fold_nodes(children),
            actions: folder.fold_actions(actions),
        }),
        Node::OnAppear {
            actions,
            asynchronous,
        } => Some(Node::OnAppear {
            actions: folder.fold_actions(actions),
            asynchronous,
        }),
        Node::OnDisappear { actions } => Some(Node::OnDisappear {
            actions: folder.fold_actions(actions),
        }),
        Node::OnActive { actions } => Some(Node::OnActive {
            actions: folder.fold_actions(actions),
        }),
        Node::OnInactive { actions } => Some(Node::OnInactive {
            actions: folder.fold_actions(actions),
        }),
        Node::OnBackground { actions } => Some(Node::OnBackground {
            actions: folder.fold_actions(actions),
        }),
        Node::AppBottomBar { state, tabs } => Some(Node::AppBottomBar {
            state,
            tabs: tabs
                .into_iter()
                .map(|tab| BottomBarTab {
                    index: tab.index,
                    label: tab.label,
                    icon: tab.icon,
                    badge: tab.badge,
                    children: folder.fold_nodes(tab.children),
                })
                .collect(),
        }),
        Node::FastList { plan } => Some(Node::FastList {
            plan: fold_list_plan(plan, folder),
        }),
        Node::If {
            condition,
            then_body,
            else_body,
        } => Some(Node::If {
            condition: folder.fold_expr(condition),
            then_body: folder.fold_nodes(then_body),
            else_body: else_body.map(|b| folder.fold_nodes(b)),
        }),
        Node::When {
            value,
            cases,
            else_body,
        } => Some(Node::When {
            value: folder.fold_expr(value),
            cases: cases
                .into_iter()
                .map(|case| WhenCase {
                    value: folder.fold_expr(case.value),
                    body: folder.fold_nodes(case.body),
                })
                .collect(),
            else_body: folder.fold_nodes(else_body),
        }),
        Node::Text { value, style } => Some(Node::Text {
            value: folder.fold_expr(value),
            style,
        }),
        Node::Button {
            label,
            icon,
            loading,
            disabled,
            actions,
        } => Some(Node::Button {
            label: folder.fold_expr(label),
            icon,
            loading: loading.map(|l| folder.fold_expr(l)),
            disabled: disabled.map(|d| folder.fold_expr(d)),
            actions: folder.fold_actions(actions),
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
            actions: folder.fold_actions(actions),
        }),
        Node::ComponentCall {
            name,
            arguments,
            children,
        } => Some(Node::ComponentCall {
            name,
            arguments: arguments
                .into_iter()
                .map(|(name, arg)| (name, folder.fold_expr(arg)))
                .collect(),
            children: children.map(|c| folder.fold_nodes(c)),
        }),
        Node::NativeComponentCall {
            namespace,
            name,
            arguments,
            children,
            event_handlers,
        } => Some(Node::NativeComponentCall {
            namespace,
            name,
            arguments: arguments
                .into_iter()
                .map(|(name, arg)| (name, folder.fold_expr(arg)))
                .collect(),
            children: children.map(|c| folder.fold_nodes(c)),
            event_handlers: event_handlers
                .into_iter()
                .map(|handler| NativeComponentEventHandler {
                    property: handler.property,
                    parameters: handler.parameters,
                    actions: folder.fold_actions(handler.actions),
                })
                .collect(),
        }),
        Node::NavigationStack { root, arguments } => Some(Node::NavigationStack {
            root,
            arguments: arguments
                .into_iter()
                .map(|arg| folder.fold_expr(arg))
                .collect(),
        }),
        Node::NavigationBack { label } => Some(Node::NavigationBack {
            label: folder.fold_expr(label),
        }),
        Node::Image {
            source,
            description,
            scale,
            placeholder,
        } => Some(Node::Image {
            source: match source {
                crate::ImageSource::RemoteUrl(url) => {
                    crate::ImageSource::RemoteUrl(folder.fold_expr(url))
                }
                other => other,
            },
            description,
            scale,
            placeholder,
        }),
        Node::Content | Node::Switch { .. } | Node::StatusBar { .. } | Node::Direction { .. } => {
            Some(node)
        }
    }
}

pub fn fold_list_plan<F: IrFolder>(plan: ListPlan, folder: &mut F) -> ListPlan {
    match plan {
        ListPlan::Count { count, common } => ListPlan::Count {
            count: folder.fold_expr(count),
            common: ListCommon {
                key: common.key.map(|k| folder.fold_expr(k)),
                children: folder.fold_nodes(common.children),
                on_end_reached: common.on_end_reached.map(|a| folder.fold_actions(a)),
                on_scroll: common.on_scroll.map(|a| folder.fold_actions(a)),
                sticky_header: common.sticky_header.map(|h| folder.fold_nodes(h)),
                refresh: common.refresh.map(|r| FastListRefresh {
                    state: r.state,
                    actions: folder.fold_actions(r.actions),
                }),
                ..common
            },
        },
        ListPlan::Items {
            collection,
            element_type,
            item,
            common,
        } => ListPlan::Items {
            collection: folder.fold_expr(collection),
            element_type,
            item,
            common: ListCommon {
                key: common.key.map(|k| folder.fold_expr(k)),
                children: folder.fold_nodes(common.children),
                on_end_reached: common.on_end_reached.map(|a| folder.fold_actions(a)),
                on_scroll: common.on_scroll.map(|a| folder.fold_actions(a)),
                sticky_header: common.sticky_header.map(|h| folder.fold_nodes(h)),
                refresh: common.refresh.map(|r| FastListRefresh {
                    state: r.state,
                    actions: folder.fold_actions(r.actions),
                }),
                ..common
            },
        },
        ListPlan::Sections {
            collection,
            element_type,
            section,
            item,
            common,
        } => ListPlan::Sections {
            collection: folder.fold_expr(collection),
            element_type,
            section,
            item,
            common: SectionedListCommon {
                key: common.key.map(|k| folder.fold_expr(k)),
                children: folder.fold_nodes(common.children),
                section_header: common.section_header.map(|h| folder.fold_nodes(h)),
                refresh: common.refresh.map(|r| FastListRefresh {
                    state: r.state,
                    actions: folder.fold_actions(r.actions),
                }),
                ..common
            },
        },
    }
}

pub fn fold_expr_children<F: IrFolder>(expr: Expr, folder: &mut F) -> Expr {
    match expr {
        Expr::Add(left, right, ty) => Expr::Add(
            Box::new(folder.fold_expr(*left)),
            Box::new(folder.fold_expr(*right)),
            ty,
        ),
        Expr::Binary { op, left, right } => Expr::Binary {
            op,
            left: Box::new(folder.fold_expr(*left)),
            right: Box::new(folder.fold_expr(*right)),
        },
        Expr::Contains {
            value,
            collection,
            collection_type,
        } => Expr::Contains {
            value: Box::new(folder.fold_expr(*value)),
            collection: Box::new(folder.fold_expr(*collection)),
            collection_type,
        },
        Expr::Not(value) => Expr::Not(Box::new(folder.fold_expr(*value))),
        Expr::Array(items) => Expr::Array(
            items
                .into_iter()
                .map(|item| folder.fold_expr(item))
                .collect(),
        ),
        Expr::Set(items) => Expr::Set(
            items
                .into_iter()
                .map(|item| folder.fold_expr(item))
                .collect(),
        ),
        Expr::Map(entries) => Expr::Map(
            entries
                .into_iter()
                .map(|(key, value)| (folder.fold_expr(key), folder.fold_expr(value)))
                .collect(),
        ),
        Expr::Pair(first, second) => Expr::Pair(
            Box::new(folder.fold_expr(*first)),
            Box::new(folder.fold_expr(*second)),
        ),
        Expr::Triple(first, second, third) => Expr::Triple(
            Box::new(folder.fold_expr(*first)),
            Box::new(folder.fold_expr(*second)),
            Box::new(folder.fold_expr(*third)),
        ),
        Expr::Call {
            name,
            arguments,
            return_type,
            is_async,
            is_constructor,
        } => Expr::Call {
            name,
            arguments: arguments
                .into_iter()
                .map(|arg| folder.fold_expr(arg))
                .collect(),
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
            collection: Box::new(folder.fold_expr(*collection)),
            initial: initial.map(|init| Box::new(folder.fold_expr(*init))),
            closure: Box::new(folder.fold_expr(*closure)),
        },
        Expr::Closure { parameters, body } => Expr::Closure {
            parameters,
            body: Box::new(folder.fold_expr(*body)),
        },
        Expr::NativeCall {
            namespace,
            name,
            receiver,
            arguments,
            return_type,
            is_async,
            is_throwing,
        } => Expr::NativeCall {
            namespace,
            name,
            receiver: receiver.map(|recv| Box::new(folder.fold_expr(*recv))),
            arguments: arguments
                .into_iter()
                .map(|(param, arg)| (param, folder.fold_expr(arg)))
                .collect(),
            return_type,
            is_async,
            is_throwing,
        },
        Expr::NetworkFetch(request) => {
            Expr::NetworkFetch(Box::new(fold_network_request(*request, folder)))
        }
        Expr::NetworkDownload {
            destination,
            request,
        } => Expr::NetworkDownload {
            destination: Box::new(folder.fold_expr(*destination)),
            request: Box::new(fold_network_request(*request, folder)),
        },
        Expr::PathJoin { path, component } => Expr::PathJoin {
            path: Box::new(folder.fold_expr(*path)),
            component: Box::new(folder.fold_expr(*component)),
        },
        Expr::FileExists { path } => Expr::FileExists {
            path: Box::new(folder.fold_expr(*path)),
        },
        Expr::FileReadText { path } => Expr::FileReadText {
            path: Box::new(folder.fold_expr(*path)),
        },
        Expr::FileWriteText { path, contents } => Expr::FileWriteText {
            path: Box::new(folder.fold_expr(*path)),
            contents: Box::new(folder.fold_expr(*contents)),
        },
        Expr::FileDelete { path } => Expr::FileDelete {
            path: Box::new(folder.fold_expr(*path)),
        },
        Expr::PermissionOp { op, permission } => Expr::PermissionOp {
            op,
            permission: Box::new(folder.fold_expr(*permission)),
        },
        Expr::Index {
            collection,
            index,
            optional,
            collection_type,
            element_type,
        } => Expr::Index {
            collection: Box::new(folder.fold_expr(*collection)),
            index: Box::new(folder.fold_expr(*index)),
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
            start: Box::new(folder.fold_expr(*start)),
            end: Box::new(folder.fold_expr(*end)),
            inclusive,
            step: step.map(|s| Box::new(folder.fold_expr(*s))),
        },
        Expr::Member {
            base,
            name,
            optional,
            base_type,
            field_type,
            kind,
        } => Expr::Member {
            base: Box::new(folder.fold_expr(*base)),
            name,
            optional,
            base_type,
            field_type,
            kind,
        },
        Expr::Coalesce(left, right) => Expr::Coalesce(
            Box::new(folder.fold_expr(*left)),
            Box::new(folder.fold_expr(*right)),
        ),
        Expr::Await(value) => Expr::Await(Box::new(folder.fold_expr(*value))),
        Expr::TryAwait(value) => Expr::TryAwait(Box::new(folder.fold_expr(*value))),
        Expr::ResultOk {
            value,
            value_type,
            error_type,
        } => Expr::ResultOk {
            value: Box::new(folder.fold_expr(*value)),
            value_type,
            error_type,
        },
        Expr::ResultErr {
            error,
            value_type,
            error_type,
        } => Expr::ResultErr {
            error: Box::new(folder.fold_expr(*error)),
            value_type,
            error_type,
        },
        Expr::Try {
            expr,
            value_type,
            error_type,
        } => Expr::Try {
            expr: Box::new(folder.fold_expr(*expr)),
            value_type,
            error_type,
        },
        Expr::Interpolation(parts) => Expr::Interpolation(
            parts
                .into_iter()
                .map(|part| match part {
                    InterpolatedPart::Literal(s) => InterpolatedPart::Literal(s),
                    InterpolatedPart::Value(val) => {
                        InterpolatedPart::Value(Box::new(folder.fold_expr(*val)))
                    }
                })
                .collect(),
        ),
        Expr::String(_)
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::State(_, _)
        | Expr::EnumValue { .. }
        | Expr::Null(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight => expr,
    }
}

pub fn fold_action_children<F: IrFolder>(action: Action, folder: &mut F) -> Option<Action> {
    match action {
        Action::Expression(expr) => Some(Action::Expression(folder.fold_expr(expr))),
        Action::Assign { name, value } => Some(Action::Assign {
            name,
            value: folder.fold_expr(value),
        }),
        Action::NativePropertyAssign {
            receiver,
            property,
            value,
        } => Some(Action::NativePropertyAssign {
            receiver: folder.fold_expr(receiver),
            property,
            value: folder.fold_expr(value),
        }),
        Action::NativeEventSubscribe {
            receiver,
            property,
            parameters,
            actions,
        } => Some(Action::NativeEventSubscribe {
            receiver: folder.fold_expr(receiver),
            property,
            parameters,
            actions: folder.fold_actions(actions),
        }),
        Action::CollectionMutation {
            name,
            operation,
            arguments,
        } => Some(Action::CollectionMutation {
            name,
            operation,
            arguments: arguments
                .into_iter()
                .map(|arg| folder.fold_expr(arg))
                .collect(),
        }),
        Action::If {
            condition,
            then_branch,
            else_branch,
        } => Some(Action::If {
            condition: folder.fold_expr(condition),
            then_branch: folder.fold_actions(then_branch),
            else_branch: else_branch.map(|eb| folder.fold_actions(eb)),
        }),
        Action::For {
            name,
            iterable,
            body,
        } => Some(Action::For {
            name,
            iterable: folder.fold_expr(iterable),
            body: folder.fold_actions(body),
        }),
        Action::ForMap {
            key_name,
            value_name,
            iterable,
            body,
        } => Some(Action::ForMap {
            key_name,
            value_name,
            iterable: folder.fold_expr(iterable),
            body: folder.fold_actions(body),
        }),
        Action::While { condition, body } => Some(Action::While {
            condition: folder.fold_expr(condition),
            body: folder.fold_actions(body),
        }),
        Action::TryCatch {
            body,
            error_catches,
            catch_body,
        } => Some(Action::TryCatch {
            body: folder.fold_actions(body),
            error_catches: error_catches
                .into_iter()
                .map(|catch| ErrorCatchArm {
                    namespace: catch.namespace,
                    error_type: catch.error_type,
                    variant: catch.variant,
                    parameters: catch.parameters,
                    body: folder.fold_actions(catch.body),
                })
                .collect(),
            catch_body: catch_body.map(|cb| folder.fold_actions(cb)),
        }),
        Action::Break | Action::Continue => Some(action),
    }
}

pub fn fold_network_request<F: IrFolder>(
    mut request: NetworkRequest,
    folder: &mut F,
) -> NetworkRequest {
    request.url = Box::new(folder.fold_expr(*request.url));
    request.method = Box::new(folder.fold_expr(*request.method));
    request.headers = Box::new(folder.fold_expr(*request.headers));
    request.timeout = Box::new(folder.fold_expr(*request.timeout));
    request.use_cache = Box::new(folder.fold_expr(*request.use_cache));
    request.follow_redirects = Box::new(folder.fold_expr(*request.follow_redirects));
    request.max_response_bytes = Box::new(folder.fold_expr(*request.max_response_bytes));
    request.certificate_pins = Box::new(folder.fold_expr(*request.certificate_pins));
    request.body = request.body.map(|body| Box::new(folder.fold_expr(*body)));
    request
}

#[cfg(test)]
mod tests {
    use super::{walk_actions, walk_callback_actions, walk_callback_expressions};
    use crate::{Action, Expr, NativeComponentEventHandler, Node, NumericType, Type};

    #[test]
    fn native_property_action_walks_receiver_and_assigned_value() {
        let actions = [Action::NativePropertyAssign {
            receiver: Expr::State(
                "player".to_owned(),
                Type::Plugin {
                    namespace: "Video".to_owned(),
                    name: "VideoPlayer".to_owned(),
                },
            ),
            property: "volume".to_owned(),
            value: Expr::State(
                "target_volume".to_owned(),
                Type::Numeric(NumericType::Float64),
            ),
        }];
        let mut visited_states = Vec::new();

        walk_actions(&actions, &mut |expression| {
            if let Expr::State(name, _) = expression {
                visited_states.push(name.clone());
            }
        });

        assert_eq!(visited_states, ["player", "target_volume"]);
    }

    #[test]
    fn callback_walker_keeps_ui_and_nested_native_callbacks_separate() {
        let nested = vec![Action::NativeEventSubscribe {
            receiver: Expr::State("player".to_owned(), Type::Void),
            property: "onEnded".to_owned(),
            parameters: Vec::new(),
            actions: vec![Action::Expression(Expr::String("nested".to_owned()))],
        }];
        let nodes = [
            Node::Button {
                label: Expr::String("play".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                actions: nested,
            },
            Node::NativeComponentCall {
                namespace: "Video".to_owned(),
                name: "VideoView".to_owned(),
                arguments: Vec::new(),
                children: None,
                event_handlers: vec![NativeComponentEventHandler {
                    property: "onTapped".to_owned(),
                    parameters: Vec::new(),
                    actions: vec![Action::Expression(Expr::String("component".to_owned()))],
                }],
            },
        ];
        let mut callback_kinds = Vec::new();
        walk_callback_actions(&nodes, &mut |actions| {
            let kind = match actions.first() {
                Some(Action::NativeEventSubscribe { .. }) => "subscription",
                Some(Action::Expression(Expr::String(value))) => value.as_str(),
                _ => "other",
            };
            callback_kinds.push(kind.to_owned());
        });

        assert_eq!(callback_kinds, ["subscription", "nested", "component"]);

        let mut outer_states = Vec::new();
        walk_callback_expressions(nodes_button_actions(&nodes[0]), &mut |expression| {
            if let Expr::State(name, _) = expression {
                outer_states.push(name.clone());
            }
        });
        assert_eq!(outer_states, ["player"]);
    }

    fn nodes_button_actions(node: &Node) -> &[Action] {
        let Node::Button { actions, .. } = node else {
            panic!("fixture should contain a button");
        };
        actions
    }

    #[test]
    fn ir_visitor_traverses_nested_nodes_and_expressions() {
        use super::IrVisitor;

        struct StateCollector(Vec<String>);
        impl IrVisitor for StateCollector {
            fn visit_expr(&mut self, expr: &Expr) {
                if let Expr::State(name, _) = expr {
                    self.0.push(name.clone());
                }
                super::walk_expr_children(expr, self);
            }
        }

        let node = Node::Layout {
            kind: crate::LayoutKind::Column,
            spacing: 0.0,
            style: crate::ViewStyle::default(),
            children: vec![
                Node::Text {
                    value: Expr::State("title".to_owned(), Type::String),
                    style: crate::TextStyle::default(),
                },
                Node::If {
                    condition: Expr::Binary {
                        op: crate::BinaryOp::Equal,
                        left: Box::new(Expr::State(
                            "count".to_owned(),
                            Type::Numeric(NumericType::Int64),
                        )),
                        right: Box::new(Expr::Number {
                            raw: "0".to_owned(),
                            ty: NumericType::Int64,
                        }),
                    },
                    then_body: vec![Node::Button {
                        label: Expr::String("Click".to_owned()),
                        icon: None,
                        loading: None,
                        disabled: None,
                        actions: vec![Action::Assign {
                            name: "count".to_owned(),
                            value: Expr::State(
                                "step".to_owned(),
                                Type::Numeric(NumericType::Int64),
                            ),
                        }],
                    }],
                    else_body: None,
                },
            ],
        };

        let mut collector = StateCollector(Vec::new());
        collector.visit_node(&node);
        assert_eq!(collector.0, vec!["title", "count", "step"]);
    }

    #[test]
    fn ir_folder_rewrites_nested_expressions_and_prunes_nodes() {
        use super::IrFolder;

        struct StateRenamer;
        impl IrFolder for StateRenamer {
            fn fold_expr(&mut self, expr: Expr) -> Expr {
                let expr = super::fold_expr_children(expr, self);
                match expr {
                    Expr::State(name, ty) if name == "old_name" => {
                        Expr::State("new_name".to_owned(), ty)
                    }
                    other => other,
                }
            }

            fn fold_node(&mut self, node: Node) -> Option<Node> {
                // Filter out Text nodes whose value is "drop_me"
                if let Node::Text {
                    value: Expr::String(ref s),
                    ..
                } = node
                    && s == "drop_me"
                {
                    return None;
                }
                super::fold_node_children(node, self)
            }
        }

        let node = Node::Layout {
            kind: crate::LayoutKind::Column,
            spacing: 0.0,
            style: crate::ViewStyle::default(),
            children: vec![
                Node::Text {
                    value: Expr::String("drop_me".to_owned()),
                    style: crate::TextStyle::default(),
                },
                Node::Text {
                    value: Expr::State("old_name".to_owned(), Type::String),
                    style: crate::TextStyle::default(),
                },
            ],
        };

        let mut renamer = StateRenamer;
        let folded = renamer.fold_node(node).expect("layout should survive");
        if let Node::Layout { children, .. } = folded {
            assert_eq!(children.len(), 1);
            match &children[0] {
                Node::Text {
                    value: Expr::State(name, _),
                    ..
                } => {
                    assert_eq!(name, "new_name");
                }
                other => panic!("expected rewritten Text node, got {other:?}"),
            }
        } else {
            panic!("expected Layout");
        }
    }
}
