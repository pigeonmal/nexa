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
            Node::FastList {
                source,
                key,
                children,
                on_end_reached,
                on_scroll,
                sticky_header,
                section_header,
                refresh,
                ..
            } => {
                walk_list_source(source, visit_expression);
                if let Some(key) = key {
                    walk_expression(key, visit_expression);
                }
                if let Some(actions) = on_end_reached {
                    walk_actions(actions, visit_expression);
                }
                if let Some(actions) = on_scroll {
                    walk_actions(actions, visit_expression);
                }
                if let Some(refresh) = refresh {
                    walk_actions(&refresh.actions, visit_expression);
                }
                if let Some(sticky_header) = sticky_header {
                    walk_ir(sticky_header, visit_node, visit_expression);
                }
                if let Some(section_header) = section_header {
                    walk_ir(section_header, visit_node, visit_expression);
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
                Node::FastList {
                    on_end_reached,
                    on_scroll,
                    refresh,
                    ..
                } => {
                    if let Some(actions) = on_end_reached {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(actions) = on_scroll {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(refresh) = refresh {
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

fn walk_list_source(source: &ListSource, visit: &mut impl FnMut(&Expr)) {
    match source {
        ListSource::Count(count) => walk_expression(count, visit),
        ListSource::Items { collection, .. } => walk_expression(collection, visit),
        ListSource::Sections { collection, .. } => walk_expression(collection, visit),
    }
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
        walk_callback_expressions(&nodes_button_actions(&nodes[0]), &mut |expression| {
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
}
