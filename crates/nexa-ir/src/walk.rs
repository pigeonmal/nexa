use crate::adaptive_layout::{
    LayoutEdge, ScrollAxis, ScrollOwnerDescriptor, ScrollOwnerId, ScrollOwnerKind,
    ScrollOwnerResolution,
};
use crate::{
    Action, BottomBarTab, ErrorCatchArm, Expr, FastListMove, FastListRefresh, InterpolatedPart,
    ListCommon, ListPlan, NativeComponentEventHandler, NetworkRequest, Node, SectionedListCommon,
    WhenCase,
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
            Node::ViewInsets {
                padding,
                margin,
                child,
            } => {
                for insets in [padding.as_ref(), margin.as_ref()].into_iter().flatten() {
                    for expression in [
                        &insets.top,
                        &insets.bottom,
                        &insets.leading,
                        &insets.trailing,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        walk_expression(expression, visit_expression);
                    }
                }
                walk_ir(
                    std::slice::from_ref(child.as_ref()),
                    visit_node,
                    visit_expression,
                );
            }
            Node::Form { children, .. }
            | Node::Toolbar { children, .. }
            | Node::KeyboardAware { children, .. } => {
                walk_ir(children, visit_node, visit_expression)
            }
            Node::Layout {
                style, children, ..
            }
            | Node::ScrollView {
                style, children, ..
            } => {
                if let Some(marquee) = &style.marquee {
                    walk_expression(&marquee.playing, visit_expression);
                }
                if let Some(radius) = &style.corner_radius_expression {
                    walk_expression(radius, visit_expression);
                }
                if let Some(scale) = &style.effects.scale_expression {
                    walk_expression(scale, visit_expression);
                }
                if let Some(scale_y) = &style.effects.scale_y_expression {
                    walk_expression(scale_y, visit_expression);
                }
                if let Some(opacity) = &style.effects.opacity_expression {
                    walk_expression(opacity, visit_expression);
                }
                if let Some(offset_y) = &style.effects.offset_y_expression {
                    walk_expression(offset_y, visit_expression);
                }
                if let Node::ScrollView {
                    component_ref: Some(component_ref),
                    ..
                } = node
                {
                    walk_expression(component_ref, visit_expression);
                }
                walk_ir(children, visit_node, visit_expression);
            }
            Node::SafeAreaInset {
                content, children, ..
            } => {
                walk_ir(content, visit_node, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::ScrollHeader {
                header, children, ..
            } => {
                walk_ir(header, visit_node, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::Dialog {
                title,
                message,
                children,
                ..
            } => {
                walk_expression(title, visit_expression);
                walk_expression(message, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::ConfirmationDialog {
                title, children, ..
            } => {
                walk_expression(title, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::BottomSheet {
                component_ref,
                background_color,
                scrim_color,
                on_dismiss,
                children,
                ..
            } => {
                walk_expression(component_ref, visit_expression);
                if let Some(crate::ColorExpression::Dynamic(color)) = background_color {
                    walk_expression(color, visit_expression);
                }
                if let Some(crate::ColorExpression::Dynamic(color)) = scrim_color {
                    walk_expression(color, visit_expression);
                }
                walk_actions(on_dismiss, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::FormSection {
                title,
                footer,
                children,
                ..
            } => {
                if let Some(title) = title {
                    walk_expression(title, visit_expression);
                }
                if let Some(footer) = footer {
                    walk_expression(footer, visit_expression);
                }
                walk_ir(children, visit_node, visit_expression);
            }
            Node::Pressable {
                disabled,
                long_press_duration_ms,
                children,
                context_menu,
                actions,
                double_tap_actions,
                long_press_actions,
                drag_actions,
                pinch_actions,
                ..
            } => {
                walk_expression(disabled, visit_expression);
                walk_expression(long_press_duration_ms, visit_expression);
                walk_actions(actions, visit_expression);
                walk_actions(double_tap_actions, visit_expression);
                walk_actions(long_press_actions, visit_expression);
                walk_actions(drag_actions, visit_expression);
                walk_actions(pinch_actions, visit_expression);
                walk_ir(children, visit_node, visit_expression);
                walk_ir(context_menu, visit_node, visit_expression);
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
            Node::NavigationSplitView {
                sidebar, detail, ..
            } => {
                walk_ir(sidebar, visit_node, visit_expression);
                walk_ir(detail, visit_node, visit_expression);
            }
            Node::Accessibility {
                label,
                hint,
                value,
                children,
                ..
            } => {
                walk_expression(label, visit_expression);
                if let Some(hint) = hint {
                    walk_expression(hint, visit_expression);
                }
                if let Some(value) = value {
                    walk_expression(value, visit_expression);
                }
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
            Node::AppBottomBar {
                tint,
                tabs,
                custom_bar,
                ..
            } => {
                if let Some(crate::ColorExpression::Dynamic(color)) = tint {
                    walk_expression(color, visit_expression);
                }
                for tab in tabs {
                    walk_ir(&tab.children, visit_node, visit_expression);
                }
                if let Some(custom_bar) = custom_bar {
                    walk_ir(custom_bar, visit_node, visit_expression);
                }
            }
            Node::PagePager { pages, .. } => {
                for page in pages {
                    walk_ir(page, visit_node, visit_expression);
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
                if let Some(actions) = plan.on_scroll_begin_drag() {
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
                if let Some(swipe_actions) = plan.swipe_actions() {
                    walk_ir(swipe_actions, visit_node, visit_expression);
                }
                walk_ir(plan.children(), visit_node, visit_expression);
            }
            Node::If {
                condition,
                then_body,
                else_body,
                ..
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
                ..
            } => {
                walk_expression(value, visit_expression);
                for case in cases {
                    walk_expression(&case.value, visit_expression);
                    walk_ir(&case.body, visit_node, visit_expression);
                }
                if let Some(else_body) = else_body {
                    walk_ir(else_body, visit_node, visit_expression);
                }
            }
            Node::Text { value, style } => {
                walk_expression(value, visit_expression);
                if let Some(reveal) = &style.reveal {
                    for expression in [
                        &reveal.segments,
                        &reveal.position,
                        &reveal.is_active,
                        &reveal.is_playing,
                        &reveal.reveal_enabled,
                        &reveal.seek_revision,
                    ] {
                        walk_expression(expression, visit_expression);
                    }
                }
                if let Some(scale) = &style.effects.scale_expression {
                    walk_expression(scale, visit_expression);
                }
                if let Some(scale_y) = &style.effects.scale_y_expression {
                    walk_expression(scale_y, visit_expression);
                }
                if let Some(opacity) = &style.effects.opacity_expression {
                    walk_expression(opacity, visit_expression);
                }
                if let Some(offset_y) = &style.effects.offset_y_expression {
                    walk_expression(offset_y, visit_expression);
                }
            }
            Node::ContentUnavailable {
                title,
                description,
                title_color,
                description_color,
                icon_tint,
                background,
                ..
            } => {
                walk_expression(title, visit_expression);
                if let Some(description) = description {
                    walk_expression(description, visit_expression);
                }
                for color in [
                    title_color.as_ref(),
                    description_color.as_ref(),
                    icon_tint.as_ref(),
                    background.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    if let crate::ColorExpression::Dynamic(expression) = color {
                        walk_expression(expression, visit_expression);
                    }
                }
            }
            Node::ProgressBar {
                progress, style, ..
            } => {
                if let Some(progress) = progress {
                    walk_expression(progress, visit_expression);
                }
                for color in style
                    .as_ref()
                    .into_iter()
                    .flat_map(|style| [style.tint.as_ref(), style.track_color.as_ref()])
                    .flatten()
                {
                    if let crate::ColorExpression::Dynamic(expression) = color {
                        walk_expression(expression, visit_expression);
                    }
                }
                if let Some(android_wave) =
                    style.as_ref().and_then(|style| style.android_wave.as_ref())
                {
                    walk_expression(android_wave, visit_expression);
                }
            }
            Node::ProgressRing { progress, style } => {
                if let Some(progress) = progress {
                    walk_expression(progress, visit_expression);
                }
                for color in style
                    .as_ref()
                    .into_iter()
                    .flat_map(|style| [style.tint.as_ref(), style.track_color.as_ref()])
                    .flatten()
                {
                    if let crate::ColorExpression::Dynamic(expression) = color {
                        walk_expression(expression, visit_expression);
                    }
                }
                if let Some(android_wave) =
                    style.as_ref().and_then(|style| style.android_wave.as_ref())
                {
                    walk_expression(android_wave, visit_expression);
                }
            }
            Node::SegmentedControl {
                items,
                tint,
                background,
                text_color,
                selected_text_color,
                border_color,
                ..
            } => {
                walk_expression(items, visit_expression);
                for color in [
                    tint.as_ref(),
                    background.as_ref(),
                    text_color.as_ref(),
                    selected_text_color.as_ref(),
                    border_color.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    if let crate::ColorExpression::Dynamic(expression) = color {
                        walk_expression(expression, visit_expression);
                    }
                }
            }
            Node::Picker {
                items, label, tint, ..
            } => {
                walk_expression(items, visit_expression);
                if let Some(label) = label {
                    walk_expression(label, visit_expression);
                }
                if let Some(crate::ColorExpression::Dynamic(color)) = tint {
                    walk_expression(color, visit_expression);
                }
            }
            Node::PickerDate { props } => {
                let crate::PickerDateProps {
                    minimum,
                    maximum,
                    tint,
                    text_color,
                    background_color,
                    picker_background_color,
                    border_color,
                    locale,
                    time_zone,
                    is_24_hour,
                    enabled,
                    accessibility_label,
                    ..
                } = props.as_ref();
                for expression in [
                    minimum.as_ref(),
                    maximum.as_ref(),
                    locale.as_ref(),
                    time_zone.as_ref(),
                    is_24_hour.as_ref(),
                    enabled.as_ref(),
                    accessibility_label.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    walk_expression(expression, visit_expression);
                }
                for color in [
                    tint.as_ref(),
                    text_color.as_ref(),
                    background_color.as_ref(),
                    picker_background_color.as_ref(),
                    border_color.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    if let crate::ColorExpression::Dynamic(expression) = color {
                        walk_expression(expression, visit_expression);
                    }
                }
            }
            Node::Button {
                label,
                loading,
                disabled,
                tint,
                accessibility_label,
                actions,
                ..
            } => {
                walk_expression(label, visit_expression);
                if let Some(accessibility_label) = accessibility_label {
                    walk_expression(accessibility_label, visit_expression);
                }
                if let Some(loading) = loading {
                    walk_expression(loading, visit_expression);
                }
                if let Some(disabled) = disabled {
                    walk_expression(disabled, visit_expression);
                }
                if let Some(crate::ColorExpression::Dynamic(color)) = tint {
                    walk_expression(color, visit_expression);
                }
                walk_actions(actions, visit_expression);
            }
            Node::ComponentCall {
                arguments,
                component_ref,
                children,
                ..
            } => {
                for (_, argument) in arguments {
                    walk_expression(argument, visit_expression);
                }
                if let Some(component_ref) = component_ref {
                    walk_expression(component_ref, visit_expression);
                }
                if let Some(children) = children {
                    walk_ir(children, visit_node, visit_expression);
                }
            }
            Node::NativeComponentCall {
                arguments,
                component_ref,
                children,
                event_handlers,
                ..
            } => {
                for (_, argument) in arguments {
                    walk_expression(argument, visit_expression);
                }
                if let Some(component_ref) = component_ref {
                    walk_expression(component_ref, visit_expression);
                }
                for handler in event_handlers {
                    walk_actions(&handler.actions, visit_expression);
                }
                if let Some(children) = children {
                    walk_ir(children, visit_node, visit_expression);
                }
            }
            Node::TextInput {
                focused,
                component_ref,
                actions,
                on_focus,
                on_change,
                ..
            } => {
                if let Some(focused) = focused {
                    walk_expression(focused, visit_expression);
                }
                if let Some(component_ref) = component_ref {
                    walk_expression(component_ref, visit_expression);
                }
                walk_actions(actions, visit_expression);
                if let Some(actions) = on_focus {
                    walk_actions(actions, visit_expression);
                }
                if let Some(change) = on_change {
                    walk_actions(&change.actions, visit_expression);
                }
            }
            Node::Switch {
                label, on_change, ..
            } => {
                walk_expression(label, visit_expression);
                if let Some(change) = on_change {
                    walk_actions(&change.actions, visit_expression);
                }
            }
            Node::Slider {
                min,
                max,
                tint,
                track_color,
                thumb_color,
                on_change,
                on_editing_changed,
                ..
            } => {
                walk_expression(min, visit_expression);
                walk_expression(max, visit_expression);
                for color in [tint, track_color, thumb_color].into_iter().flatten() {
                    if let crate::ColorExpression::Dynamic(expression) = color {
                        walk_expression(expression, visit_expression);
                    }
                }
                if let Some(change) = on_change {
                    walk_actions(&change.actions, visit_expression);
                }
                if let Some(change) = on_editing_changed {
                    walk_actions(&change.actions, visit_expression);
                }
            }
            Node::Appearance { mode, children } => {
                walk_expression(mode, visit_expression);
                walk_ir(children, visit_node, visit_expression);
            }
            Node::Shape {
                width_expression,
                height_expression,
                ..
            } => {
                for expression in [width_expression.as_ref(), height_expression.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    walk_expression(expression, visit_expression);
                }
            }
            Node::Content
            | Node::Spacer { .. }
            | Node::Divider { .. }
            | Node::LinearGradient { .. }
            | Node::StatusBar { .. }
            | Node::Direction { .. } => {}
            Node::SystemIcon { tint, .. } => {
                if let crate::ColorExpression::Dynamic(color) = tint {
                    walk_expression(color, visit_expression);
                }
            }
            Node::FontIcon {
                glyph, size, tint, ..
            } => {
                walk_expression(glyph, visit_expression);
                walk_expression(size, visit_expression);
                if let crate::ColorExpression::Dynamic(color) = tint {
                    walk_expression(color, visit_expression);
                }
            }
            Node::NavigationStack { arguments, .. } => {
                for argument in arguments {
                    walk_expression(argument, visit_expression);
                }
            }
            Node::NavigationBack { label, .. } => walk_expression(label, visit_expression),
            Node::Image {
                source,
                shared_element,
                ..
            } => {
                if let crate::ImageSource::RemoteUrl(url) | crate::ImageSource::LocalFile(url) =
                    source
                {
                    walk_expression(url, visit_expression);
                }
                if let Some(id) = shared_element {
                    walk_expression(id, visit_expression);
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
                | Node::Button { actions, .. } => Some(actions.as_slice()),
                Node::TextInput {
                    actions,
                    on_focus,
                    on_change,
                    ..
                } => {
                    if let Some(actions) = on_focus {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(change) = on_change {
                        visit(&change.actions);
                        walk_nested_callback_actions(&change.actions, visit);
                    }
                    Some(actions.as_slice())
                }
                Node::Pressable {
                    actions,
                    double_tap_actions,
                    long_press_actions,
                    drag_actions,
                    pinch_actions,
                    ..
                } => {
                    visit(actions);
                    visit(double_tap_actions);
                    visit(long_press_actions);
                    visit(drag_actions);
                    visit(pinch_actions);
                    walk_nested_callback_actions(actions, visit);
                    walk_nested_callback_actions(double_tap_actions, visit);
                    walk_nested_callback_actions(long_press_actions, visit);
                    walk_nested_callback_actions(drag_actions, visit);
                    walk_nested_callback_actions(pinch_actions, visit);
                    None
                }
                Node::RefreshControl { actions, .. } => Some(actions.as_slice()),
                Node::BottomSheet { on_dismiss, .. } => Some(on_dismiss.as_slice()),
                Node::FastList { plan } => {
                    if let Some(actions) = plan.on_end_reached() {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(actions) = plan.on_scroll() {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(actions) = plan.on_scroll_begin_drag() {
                        visit(actions);
                        walk_nested_callback_actions(actions, visit);
                    }
                    if let Some(on_move) = plan.on_move() {
                        visit(&on_move.actions);
                        walk_nested_callback_actions(&on_move.actions, visit);
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
            Action::NativeEventSubscribe { actions, .. }
            | Action::NetworkStatusSubscribe { actions, .. }
            | Action::TaskLaunch { actions, .. }
            | Action::WithAnimation { actions, .. } => {
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
            | Action::NavigationPush { .. }
            | Action::NavigationBack
            | Action::NavigationCommand(_)
            | Action::Let { .. }
            | Action::Return { .. }
            | Action::Assign { .. }
            | Action::NativePropertyAssign { .. }
            | Action::CollectionMutation { .. }
            | Action::TaskCancel { .. }
            | Action::Break
            | Action::Continue => {}
        }
    }
}

/// Visits an expression tree in preorder without allocating.
pub fn walk_expression(expression: &Expr, visit: &mut impl FnMut(&Expr)) {
    visit(expression);
    match expression {
        Expr::Add(left, right, _)
        | Expr::Concat(left, right)
        | Expr::Arithmetic { left, right, .. }
        | Expr::Binary { left, right, .. } => {
            walk_expression(left, visit);
            walk_expression(right, visit);
        }
        Expr::Contains {
            value, collection, ..
        } => {
            walk_expression(value, visit);
            walk_expression(collection, visit);
        }
        Expr::Not(value) | Expr::Negate { value, .. } => walk_expression(value, visit),
        Expr::LocalizedText { value, .. }
        | Expr::Located {
            expression: value, ..
        } => walk_expression(value, visit),
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
        Expr::PluginEnumConstructor { arguments, .. } => {
            for argument in arguments {
                walk_expression(argument, visit);
            }
        }
        Expr::PluginEnumOptionalConstructor { value, .. } => walk_expression(value, visit),
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
        Expr::CollectionUtility {
            collection,
            start,
            end,
            ..
        } => {
            walk_expression(collection, visit);
            if let Some(start) = start {
                walk_expression(start, visit);
            }
            if let Some(end) = end {
                walk_expression(end, visit);
            }
        }
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        } => {
            walk_expression(condition, visit);
            walk_expression(then_value, visit);
            walk_expression(else_value, visit);
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
        Expr::FileExists { path }
        | Expr::FileReadText { path }
        | Expr::FileReadBytes { path }
        | Expr::FileDelete { path } => {
            walk_expression(path, visit);
        }
        Expr::BytesFromText { text } => walk_expression(text, visit),
        Expr::BytesFromArray { values } => walk_expression(values, visit),
        Expr::BytesCount { bytes } => walk_expression(bytes, visit),
        Expr::TimeCall { arguments, .. } => {
            for argument in arguments {
                walk_expression(argument, visit);
            }
        }
        Expr::LogCall { message, .. } => walk_expression(message, visit),
        Expr::FileWriteText { path, contents } | Expr::FileWriteBytes { path, contents } => {
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
        | Expr::ComponentRefCreate { .. }
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::State(_, _)
        | Expr::AnimatedState(_, _)
        | Expr::EnumValue { .. }
        | Expr::PluginEnumValue { .. }
        | Expr::Null(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight
        | Expr::ScreenWidth
        | Expr::ScreenHeight
        | Expr::SystemInset(_)
        | Expr::This(_) => {}
    }
}

/// Returns whether a node tree already contains a native scrolling primitive.
pub fn contains_scrollable(nodes: &[Node]) -> bool {
    scroll_owners(nodes)
        .iter()
        .any(|owner| owner.kind != ScrollOwnerKind::PagePager)
}

/// Finds native scroll owners in deterministic depth-first order. IDs are
/// local to the supplied subtree and can be used for compile-time selection.
pub fn scroll_owners(nodes: &[Node]) -> Vec<ScrollOwnerDescriptor> {
    let mut owners = Vec::new();
    walk_ir(
        nodes,
        &mut |node| {
            let (kind, axes) = match node {
                Node::ScrollView { axis, .. } => (ScrollOwnerKind::ScrollView, *axis),
                Node::FastList { plan } => {
                    let axes = match plan.axis() {
                        crate::ListAxis::Vertical => ScrollAxis::Vertical,
                        crate::ListAxis::Horizontal => ScrollAxis::Horizontal,
                        crate::ListAxis::Grid { .. }
                        | crate::ListAxis::AutoGrid { .. }
                        | crate::ListAxis::Masonry { .. } => ScrollAxis::Both,
                    };
                    (ScrollOwnerKind::FastList, axes)
                }
                Node::Form { .. } => (ScrollOwnerKind::Form, ScrollAxis::Vertical),
                Node::KeyboardAware { children, .. } => {
                    if !scroll_owners(children).is_empty() {
                        return;
                    }
                    (ScrollOwnerKind::KeyboardAware, ScrollAxis::Vertical)
                }
                Node::PagePager { .. } => (ScrollOwnerKind::PagePager, ScrollAxis::Horizontal),
                _ => return,
            };
            owners.push(ScrollOwnerDescriptor {
                id: ScrollOwnerId(owners.len() as u32),
                kind,
                axes,
            });
        },
        &mut |_| {},
    );
    owners
}

/// Resolves the scroll owner relevant to an edge. An explicit ID is validated
/// against owners that can scroll along that edge; inference succeeds only for
/// one candidate and never picks an arbitrary nested or sibling owner.
pub fn resolve_scroll_owner(
    nodes: &[Node],
    edge: LayoutEdge,
    explicit: Option<ScrollOwnerId>,
) -> ScrollOwnerResolution {
    let candidates = scroll_owners(nodes)
        .into_iter()
        .filter(|owner| owner.axes.supports_edge(edge))
        .collect::<Vec<_>>();
    if let Some(id) = explicit {
        return candidates
            .into_iter()
            .find(|owner| owner.id == id)
            .map(ScrollOwnerResolution::Unique)
            .unwrap_or(ScrollOwnerResolution::InvalidReference(id));
    }
    match candidates.as_slice() {
        [] => ScrollOwnerResolution::None,
        [owner] => ScrollOwnerResolution::Unique(*owner),
        _ => ScrollOwnerResolution::Ambiguous(candidates),
    }
}

fn walk_network_request_children(request: &NetworkRequest, visit: &mut impl FnMut(&Expr)) {
    visit(&request.url);
    visit(&request.method);
    visit(&request.headers);
    visit(&request.timeout);
    visit(&request.cache_policy);
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
        ListPlan::Count { count, common } => {
            visit(count);
            if let Some(component_ref) = &common.component_ref {
                visit(component_ref);
            }
        }
        ListPlan::Items {
            collection, common, ..
        } => {
            visit(collection);
            if let Some(component_ref) = &common.component_ref {
                visit(component_ref);
            }
        }
        ListPlan::Sections {
            collection, common, ..
        } => {
            visit(collection);
            if let Some(component_ref) = &common.component_ref {
                visit(component_ref);
            }
        }
    }
    if let Some(enabled) = plan.scroll_enabled() {
        visit(enabled);
    }
}

fn walk_list_plan(plan: &ListPlan, visit: &mut impl FnMut(&Expr)) {
    walk_list_plan_children(plan, &mut |expr| walk_expression(expr, visit));
}

pub fn walk_actions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    walk_action_expressions(actions, visit, true);
}

/// Traverses action expressions once while optionally treating subscription
/// handlers as separate callback boundaries.
fn walk_action_expressions(
    actions: &[Action],
    visit: &mut impl FnMut(&Expr),
    include_subscription_handlers: bool,
) {
    for action in actions {
        match action {
            Action::Expression(expression) => walk_expression(expression, visit),
            Action::NavigationPush { arguments, .. } => {
                for argument in arguments {
                    walk_expression(argument, visit);
                }
            }
            Action::NavigationBack => {}
            Action::NavigationCommand(cmd) => match cmd.as_ref() {
                crate::NavigationCommand::Push { arguments, .. }
                | crate::NavigationCommand::ReplaceTop { arguments, .. }
                | crate::NavigationCommand::Reset { arguments, .. } => {
                    for argument in arguments {
                        walk_expression(argument, visit);
                    }
                }
                _ => {}
            },
            Action::Let { value, .. } | Action::Return { value } => walk_expression(value, visit),
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
                if include_subscription_handlers {
                    walk_action_expressions(actions, visit, include_subscription_handlers);
                }
            }
            Action::NetworkStatusSubscribe { actions, .. } => {
                if include_subscription_handlers {
                    walk_action_expressions(actions, visit, include_subscription_handlers);
                }
            }
            Action::CollectionMutation { arguments, .. } => {
                for argument in arguments {
                    walk_expression(argument, visit);
                }
            }
            Action::TaskLaunch { actions, .. } | Action::WithAnimation { actions, .. } => {
                walk_action_expressions(actions, visit, include_subscription_handlers)
            }
            Action::TaskCancel { .. } => {}
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                walk_expression(condition, visit);
                walk_action_expressions(then_branch, visit, include_subscription_handlers);
                if let Some(else_branch) = else_branch {
                    walk_action_expressions(else_branch, visit, include_subscription_handlers);
                }
            }
            Action::For { iterable, body, .. } => {
                walk_expression(iterable, visit);
                walk_action_expressions(body, visit, include_subscription_handlers);
            }
            Action::ForMap { iterable, body, .. } => {
                walk_expression(iterable, visit);
                walk_action_expressions(body, visit, include_subscription_handlers);
            }
            Action::While { condition, body } => {
                walk_expression(condition, visit);
                walk_action_expressions(body, visit, include_subscription_handlers);
            }
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                walk_action_expressions(body, visit, include_subscription_handlers);
                for arm in error_catches {
                    walk_action_expressions(&arm.body, visit, include_subscription_handlers);
                }
                if let Some(catch_body) = catch_body {
                    walk_action_expressions(catch_body, visit, include_subscription_handlers);
                }
            }
            Action::Break | Action::Continue => {}
        }
    }
}

/// Visits every action in source order, including nested branches, callbacks,
/// and native task bodies. This is for action-level analyses; expression-only
/// consumers should prefer `walk_actions`.
pub fn walk_action_tree(actions: &[Action], visit: &mut impl FnMut(&Action)) {
    for action in actions {
        visit(action);
        match action {
            Action::NativeEventSubscribe { actions, .. }
            | Action::NetworkStatusSubscribe { actions, .. }
            | Action::TaskLaunch { actions, .. }
            | Action::WithAnimation { actions, .. } => walk_action_tree(actions, visit),
            Action::If {
                then_branch,
                else_branch,
                ..
            } => {
                walk_action_tree(then_branch, visit);
                if let Some(else_branch) = else_branch {
                    walk_action_tree(else_branch, visit);
                }
            }
            Action::For { body, .. } | Action::ForMap { body, .. } | Action::While { body, .. } => {
                walk_action_tree(body, visit);
            }
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                walk_action_tree(body, visit);
                for arm in error_catches {
                    walk_action_tree(&arm.body, visit);
                }
                if let Some(catch_body) = catch_body {
                    walk_action_tree(catch_body, visit);
                }
            }
            Action::Expression(_)
            | Action::NavigationPush { .. }
            | Action::NavigationBack
            | Action::NavigationCommand(_)
            | Action::Let { .. }
            | Action::Return { .. }
            | Action::Assign { .. }
            | Action::NativePropertyAssign { .. }
            | Action::CollectionMutation { .. }
            | Action::TaskCancel { .. }
            | Action::Break
            | Action::Continue => {}
        }
    }
}

/// Returns whether any action in a callback tree launches a task.
pub fn actions_contain_task_launch(actions: &[Action]) -> bool {
    let mut found = false;
    walk_action_tree(actions, &mut |action| {
        found |= matches!(action, Action::TaskLaunch { .. });
    });
    found
}

/// Returns whether a callback tree launches a task without retaining a handle.
pub fn actions_contain_untracked_task_launch(actions: &[Action]) -> bool {
    let mut found = false;
    walk_action_tree(actions, &mut |action| {
        found |= matches!(action, Action::TaskLaunch { handle: None, .. });
    });
    found
}

/// Returns whether any action expression reads one of the named state/local
/// bindings. The shared expression walk also covers nested actions and
/// callback bodies.
pub fn actions_reference_state(actions: &[Action], names: &[&str]) -> bool {
    let mut found = false;
    walk_actions(actions, &mut |expression| {
        if let Expr::State(name, _) = expression
            && names.contains(&name.as_str())
        {
            found = true;
        }
    });
    found
}

/// Visits expressions executed by one action callback. The receiver of a
/// nested native-event subscription is included, but its handler body is a
/// separate callback and is not traversed here.
pub fn walk_callback_expressions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    walk_action_expressions(actions, visit, false);
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
        Node::ViewInsets {
            padding,
            margin,
            child,
        } => {
            for insets in [padding.as_ref(), margin.as_ref()].into_iter().flatten() {
                for expression in [
                    &insets.top,
                    &insets.bottom,
                    &insets.leading,
                    &insets.trailing,
                ]
                .into_iter()
                .flatten()
                {
                    visitor.visit_expr(expression);
                }
            }
            visitor.visit_nodes(std::slice::from_ref(child.as_ref()));
        }
        Node::Form { children, .. }
        | Node::Toolbar { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::Dialog { children, .. } => visitor.visit_nodes(children),
        Node::Layout {
            style, children, ..
        }
        | Node::ScrollView {
            style, children, ..
        } => {
            if let Some(marquee) = &style.marquee {
                visitor.visit_expr(&marquee.playing);
            }
            if let Some(radius) = &style.corner_radius_expression {
                visitor.visit_expr(radius);
            }
            if let Some(scale) = &style.effects.scale_expression {
                visitor.visit_expr(scale);
            }
            if let Some(scale_y) = &style.effects.scale_y_expression {
                visitor.visit_expr(scale_y);
            }
            if let Some(opacity) = &style.effects.opacity_expression {
                visitor.visit_expr(opacity);
            }
            if let Some(offset_y) = &style.effects.offset_y_expression {
                visitor.visit_expr(offset_y);
            }
            if let Node::ScrollView {
                component_ref: Some(component_ref),
                ..
            } = node
            {
                visitor.visit_expr(component_ref);
            }
            visitor.visit_nodes(children);
        }
        Node::SafeAreaInset {
            content, children, ..
        } => {
            visitor.visit_nodes(content);
            visitor.visit_nodes(children);
        }
        Node::ScrollHeader {
            header, children, ..
        } => {
            visitor.visit_nodes(header);
            visitor.visit_nodes(children);
        }
        Node::BottomSheet {
            component_ref,
            background_color,
            scrim_color,
            on_dismiss,
            children,
            ..
        } => {
            visitor.visit_expr(component_ref);
            if let Some(crate::ColorExpression::Dynamic(color)) = background_color {
                visitor.visit_expr(color);
            }
            if let Some(crate::ColorExpression::Dynamic(color)) = scrim_color {
                visitor.visit_expr(color);
            }
            visitor.visit_actions(on_dismiss);
            visitor.visit_nodes(children);
        }
        Node::ConfirmationDialog { children, .. } => visitor.visit_nodes(children),
        Node::Pressable {
            disabled,
            children,
            actions,
            double_tap_actions,
            long_press_duration_ms,
            long_press_actions,
            context_menu,
            drag_actions,
            pinch_actions,
            ..
        } => {
            visitor.visit_expr(disabled);
            visitor.visit_expr(long_press_duration_ms);
            visitor.visit_actions(actions);
            visitor.visit_actions(double_tap_actions);
            visitor.visit_actions(long_press_actions);
            visitor.visit_nodes(context_menu);
            visitor.visit_actions(drag_actions);
            visitor.visit_actions(pinch_actions);
            visitor.visit_nodes(children);
        }
        Node::FormSection {
            title,
            footer,
            children,
            ..
        } => {
            if let Some(title) = title {
                visitor.visit_expr(title);
            }
            if let Some(footer) = footer {
                visitor.visit_expr(footer);
            }
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
        Node::NavigationSplitView {
            sidebar, detail, ..
        } => {
            visitor.visit_nodes(sidebar);
            visitor.visit_nodes(detail);
        }
        Node::Accessibility {
            label,
            hint,
            value,
            children,
            ..
        } => {
            visitor.visit_expr(label);
            if let Some(hint) = hint {
                visitor.visit_expr(hint);
            }
            if let Some(value) = value {
                visitor.visit_expr(value);
            }
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
        Node::AppBottomBar {
            tint,
            tabs,
            custom_bar,
            ..
        } => {
            if let Some(crate::ColorExpression::Dynamic(color)) = tint {
                visitor.visit_expr(color);
            }
            for tab in tabs {
                visitor.visit_nodes(&tab.children);
            }
            if let Some(custom_bar) = custom_bar {
                visitor.visit_nodes(custom_bar);
            }
        }
        Node::PagePager { pages, .. } => {
            for page in pages {
                visitor.visit_nodes(page);
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
            if let Some(actions) = plan.on_scroll_begin_drag() {
                visitor.visit_actions(actions);
            }
            if let Some(on_move) = plan.on_move() {
                visitor.visit_expr(&on_move.enabled);
                visitor.visit_actions(&on_move.actions);
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
            if let Some(swipe_actions) = plan.swipe_actions() {
                visitor.visit_nodes(swipe_actions);
            }
            visitor.visit_nodes(plan.children());
        }
        Node::If {
            condition,
            then_body,
            else_body,
            ..
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
            ..
        } => {
            visitor.visit_expr(value);
            for case in cases {
                visitor.visit_expr(&case.value);
                visitor.visit_nodes(&case.body);
            }
            if let Some(else_body) = else_body {
                visitor.visit_nodes(else_body);
            }
        }
        Node::Text { value, style } => {
            visitor.visit_expr(value);
            if let Some(reveal) = &style.reveal {
                visitor.visit_expr(&reveal.segments);
                visitor.visit_expr(&reveal.position);
                visitor.visit_expr(&reveal.is_active);
                visitor.visit_expr(&reveal.is_playing);
                visitor.visit_expr(&reveal.reveal_enabled);
                visitor.visit_expr(&reveal.seek_revision);
            }
            if let Some(scale) = &style.effects.scale_expression {
                visitor.visit_expr(scale);
            }
            if let Some(scale_y) = &style.effects.scale_y_expression {
                visitor.visit_expr(scale_y);
            }
            if let Some(opacity) = &style.effects.opacity_expression {
                visitor.visit_expr(opacity);
            }
            if let Some(offset_y) = &style.effects.offset_y_expression {
                visitor.visit_expr(offset_y);
            }
        }
        Node::ContentUnavailable {
            title,
            description,
            title_color,
            description_color,
            icon_tint,
            background,
            ..
        } => {
            visitor.visit_expr(title);
            if let Some(description) = description {
                visitor.visit_expr(description);
            }
            for color in [
                title_color.as_ref(),
                description_color.as_ref(),
                icon_tint.as_ref(),
                background.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                if let crate::ColorExpression::Dynamic(expression) = color {
                    visitor.visit_expr(expression);
                }
            }
        }
        Node::ProgressBar {
            progress, style, ..
        } => {
            if let Some(progress) = progress {
                visitor.visit_expr(progress);
            }
            for color in style
                .as_ref()
                .into_iter()
                .flat_map(|style| [style.tint.as_ref(), style.track_color.as_ref()])
                .flatten()
            {
                if let crate::ColorExpression::Dynamic(expression) = color {
                    visitor.visit_expr(expression);
                }
            }
            if let Some(android_wave) = style.as_ref().and_then(|style| style.android_wave.as_ref())
            {
                visitor.visit_expr(android_wave);
            }
        }
        Node::ProgressRing { progress, style } => {
            if let Some(progress) = progress {
                visitor.visit_expr(progress);
            }
            for color in style
                .as_ref()
                .into_iter()
                .flat_map(|style| [style.tint.as_ref(), style.track_color.as_ref()])
                .flatten()
            {
                if let crate::ColorExpression::Dynamic(expression) = color {
                    visitor.visit_expr(expression);
                }
            }
            if let Some(android_wave) = style.as_ref().and_then(|style| style.android_wave.as_ref())
            {
                visitor.visit_expr(android_wave);
            }
        }
        Node::SegmentedControl {
            items,
            tint,
            background,
            text_color,
            selected_text_color,
            border_color,
            ..
        } => {
            visitor.visit_expr(items);
            for color in [
                tint.as_ref(),
                background.as_ref(),
                text_color.as_ref(),
                selected_text_color.as_ref(),
                border_color.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                if let crate::ColorExpression::Dynamic(expression) = color {
                    visitor.visit_expr(expression);
                }
            }
        }
        Node::Picker {
            items, label, tint, ..
        } => {
            visitor.visit_expr(items);
            if let Some(label) = label {
                visitor.visit_expr(label);
            }
            if let Some(crate::ColorExpression::Dynamic(color)) = tint {
                visitor.visit_expr(color);
            }
        }
        Node::PickerDate { props } => {
            let crate::PickerDateProps {
                minimum,
                maximum,
                tint,
                text_color,
                background_color,
                picker_background_color,
                border_color,
                locale,
                time_zone,
                is_24_hour,
                enabled,
                accessibility_label,
                ..
            } = props.as_ref();
            for expression in [
                minimum.as_ref(),
                maximum.as_ref(),
                locale.as_ref(),
                time_zone.as_ref(),
                is_24_hour.as_ref(),
                enabled.as_ref(),
                accessibility_label.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                visitor.visit_expr(expression);
            }
            for color in [
                tint.as_ref(),
                text_color.as_ref(),
                background_color.as_ref(),
                picker_background_color.as_ref(),
                border_color.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                if let crate::ColorExpression::Dynamic(expression) = color {
                    visitor.visit_expr(expression);
                }
            }
        }
        Node::Button {
            label,
            loading,
            disabled,
            tint,
            accessibility_label,
            actions,
            ..
        } => {
            visitor.visit_expr(label);
            if let Some(accessibility_label) = accessibility_label {
                visitor.visit_expr(accessibility_label);
            }
            if let Some(loading) = loading {
                visitor.visit_expr(loading);
            }
            if let Some(disabled) = disabled {
                visitor.visit_expr(disabled);
            }
            if let Some(crate::ColorExpression::Dynamic(color)) = tint {
                visitor.visit_expr(color);
            }
            visitor.visit_actions(actions);
        }
        Node::SystemIcon { tint, .. } => {
            if let crate::ColorExpression::Dynamic(color) = tint {
                visitor.visit_expr(color);
            }
        }
        Node::FontIcon {
            glyph, size, tint, ..
        } => {
            visitor.visit_expr(glyph);
            visitor.visit_expr(size);
            if let crate::ColorExpression::Dynamic(color) = tint {
                visitor.visit_expr(color);
            }
        }
        Node::ComponentCall {
            arguments,
            component_ref,
            children,
            ..
        } => {
            for (_, argument) in arguments {
                visitor.visit_expr(argument);
            }
            if let Some(component_ref) = component_ref {
                visitor.visit_expr(component_ref);
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
        Node::TextInput {
            focused,
            component_ref,
            actions,
            on_focus,
            on_change,
            ..
        } => {
            if let Some(focused) = focused {
                visitor.visit_expr(focused);
            }
            if let Some(component_ref) = component_ref {
                visitor.visit_expr(component_ref);
            }
            visitor.visit_actions(actions);
            if let Some(actions) = on_focus {
                visitor.visit_actions(actions);
            }
            if let Some(change) = on_change {
                visitor.visit_actions(&change.actions);
            }
        }
        Node::Switch {
            label, on_change, ..
        } => {
            visitor.visit_expr(label);
            if let Some(change) = on_change {
                visitor.visit_actions(&change.actions);
            }
        }
        Node::Slider {
            min,
            max,
            tint,
            track_color,
            thumb_color,
            on_change,
            on_editing_changed,
            ..
        } => {
            visitor.visit_expr(min);
            visitor.visit_expr(max);
            for color in [tint, track_color, thumb_color].into_iter().flatten() {
                if let crate::ColorExpression::Dynamic(expression) = color {
                    visitor.visit_expr(expression);
                }
            }
            if let Some(change) = on_change {
                visitor.visit_actions(&change.actions);
            }
            if let Some(change) = on_editing_changed {
                visitor.visit_actions(&change.actions);
            }
        }
        Node::Appearance { mode, children } => {
            visitor.visit_expr(mode);
            visitor.visit_nodes(children);
        }
        Node::Shape {
            width_expression,
            height_expression,
            ..
        } => {
            for expression in [width_expression.as_ref(), height_expression.as_ref()]
                .into_iter()
                .flatten()
            {
                visitor.visit_expr(expression);
            }
        }
        Node::Content
        | Node::Spacer { .. }
        | Node::Divider { .. }
        | Node::LinearGradient { .. }
        | Node::StatusBar { .. }
        | Node::Direction { .. } => {}
        Node::NavigationStack { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Node::NavigationBack { label, .. } => visitor.visit_expr(label),
        Node::Image {
            source,
            shared_element,
            ..
        } => {
            if let crate::ImageSource::RemoteUrl(url) | crate::ImageSource::LocalFile(url) = source
            {
                visitor.visit_expr(url);
            }
            if let Some(id) = shared_element {
                visitor.visit_expr(id);
            }
        }
    }
}

pub fn walk_expr_children<V: IrVisitor>(expr: &Expr, visitor: &mut V) {
    match expr {
        Expr::Add(left, right, _)
        | Expr::Concat(left, right)
        | Expr::Arithmetic { left, right, .. }
        | Expr::Binary { left, right, .. } => {
            visitor.visit_expr(left);
            visitor.visit_expr(right);
        }
        Expr::Contains {
            value, collection, ..
        } => {
            visitor.visit_expr(value);
            visitor.visit_expr(collection);
        }
        Expr::Not(value) | Expr::Negate { value, .. } => visitor.visit_expr(value),
        Expr::LocalizedText { value, .. } => visitor.visit_expr(value),
        Expr::Located { expression, .. } => visitor.visit_expr(expression),
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
        Expr::PluginEnumConstructor { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Expr::PluginEnumOptionalConstructor { value, .. } => visitor.visit_expr(value),
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
        Expr::CollectionUtility {
            collection,
            start,
            end,
            ..
        } => {
            visitor.visit_expr(collection);
            if let Some(start) = start {
                visitor.visit_expr(start);
            }
            if let Some(end) = end {
                visitor.visit_expr(end);
            }
        }
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        } => {
            visitor.visit_expr(condition);
            visitor.visit_expr(then_value);
            visitor.visit_expr(else_value);
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
        Expr::FileExists { path }
        | Expr::FileReadText { path }
        | Expr::FileReadBytes { path }
        | Expr::FileDelete { path } => {
            visitor.visit_expr(path);
        }
        Expr::BytesFromText { text } => visitor.visit_expr(text),
        Expr::BytesFromArray { values } => visitor.visit_expr(values),
        Expr::BytesCount { bytes } => visitor.visit_expr(bytes),
        Expr::TimeCall { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Expr::LogCall { message, .. } => visitor.visit_expr(message),
        Expr::FileWriteText { path, contents } | Expr::FileWriteBytes { path, contents } => {
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
        | Expr::ComponentRefCreate { .. }
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::State(_, _)
        | Expr::AnimatedState(_, _)
        | Expr::EnumValue { .. }
        | Expr::PluginEnumValue { .. }
        | Expr::Null(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight
        | Expr::ScreenWidth
        | Expr::ScreenHeight
        | Expr::SystemInset(_)
        | Expr::This(_) => {}
    }
}

pub fn walk_action_children<V: IrVisitor>(action: &Action, visitor: &mut V) {
    match action {
        Action::Expression(expr) => visitor.visit_expr(expr),
        Action::NavigationPush { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Action::NavigationBack => {}
        Action::NavigationCommand(cmd) => match cmd.as_ref() {
            crate::NavigationCommand::Push { arguments, .. }
            | crate::NavigationCommand::ReplaceTop { arguments, .. }
            | crate::NavigationCommand::Reset { arguments, .. } => {
                for argument in arguments {
                    visitor.visit_expr(argument);
                }
            }
            _ => {}
        },
        Action::Let { value, .. } | Action::Return { value } => visitor.visit_expr(value),
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
        Action::NetworkStatusSubscribe { actions, .. } => visitor.visit_actions(actions),
        Action::CollectionMutation { arguments, .. } => {
            for argument in arguments {
                visitor.visit_expr(argument);
            }
        }
        Action::TaskLaunch { actions, .. } | Action::WithAnimation { actions, .. } => {
            visitor.visit_actions(actions)
        }
        Action::TaskCancel { .. } => {}
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
        Node::ViewInsets {
            mut padding,
            mut margin,
            child,
        } => {
            for insets in [&mut padding, &mut margin].into_iter().flatten() {
                for edge in [
                    &mut insets.top,
                    &mut insets.bottom,
                    &mut insets.leading,
                    &mut insets.trailing,
                ] {
                    if let Some(expression) = edge.take() {
                        *edge = Some(Box::new(folder.fold_expr(*expression)));
                    }
                }
            }
            let mut children = folder.fold_nodes(vec![*child]);
            let child = children.pop()?;
            Some(Node::ViewInsets {
                padding,
                margin,
                child: Box::new(child),
            })
        }
        Node::SafeAreaInset {
            plan,
            content,
            children,
        } => Some(Node::SafeAreaInset {
            plan,
            content: folder.fold_nodes(content),
            children: folder.fold_nodes(children),
        }),
        Node::ScrollHeader {
            plan,
            header,
            children,
        } => Some(Node::ScrollHeader {
            plan,
            header: folder.fold_nodes(header),
            children: folder.fold_nodes(children),
        }),
        Node::Form {
            show_dividers,
            hide_horizontal_scrollbar,
            hide_vertical_scrollbar,
            background,
            children,
        } => Some(Node::Form {
            show_dividers,
            hide_horizontal_scrollbar,
            hide_vertical_scrollbar,
            background,
            children: folder.fold_nodes(children),
        }),
        Node::Switch {
            state,
            label,
            on_change,
        } => Some(Node::Switch {
            state,
            label: folder.fold_expr(label),
            on_change: on_change.map(|mut change| {
                change.actions = folder.fold_actions(change.actions);
                change
            }),
        }),
        Node::Slider {
            state,
            animated,
            min,
            max,
            step,
            show_step_indicators,
            tint,
            track_color,
            thumb_color,
            track_height,
            thumb_size,
            hit_slop,
            on_change,
            on_editing_changed,
        } => Some(Node::Slider {
            state,
            animated,
            min: folder.fold_expr(min),
            max: folder.fold_expr(max),
            step,
            show_step_indicators,
            tint: tint.map(|color| fold_color_expression(color, folder)),
            track_color: track_color.map(|color| fold_color_expression(color, folder)),
            thumb_color: thumb_color.map(|color| fold_color_expression(color, folder)),
            track_height,
            thumb_size,
            hit_slop,
            on_change: on_change.map(|mut change| {
                change.actions = folder.fold_actions(change.actions);
                change
            }),
            on_editing_changed: on_editing_changed.map(|mut change| {
                change.actions = folder.fold_actions(change.actions);
                change
            }),
        }),
        Node::FormSection {
            title,
            footer,
            show_dividers,
            background,
            children,
        } => Some(Node::FormSection {
            title: title.map(|title| folder.fold_expr(title)),
            footer: footer.map(|footer| folder.fold_expr(footer)),
            show_dividers,
            background,
            children: folder.fold_nodes(children),
        }),
        Node::Layout {
            kind,
            gap,
            mut style,
            children,
        } => {
            style.corner_radius_expression = style
                .corner_radius_expression
                .map(|radius| Box::new(folder.fold_expr(*radius)));
            style.effects.scale_expression = style
                .effects
                .scale_expression
                .map(|scale| Box::new(folder.fold_expr(*scale)));
            style.effects.scale_y_expression = style
                .effects
                .scale_y_expression
                .map(|scale_y| Box::new(folder.fold_expr(*scale_y)));
            style.effects.opacity_expression = style
                .effects
                .opacity_expression
                .map(|opacity| Box::new(folder.fold_expr(*opacity)));
            style.effects.offset_y_expression = style
                .effects
                .offset_y_expression
                .map(|offset_y| Box::new(folder.fold_expr(*offset_y)));
            if let Some(marquee) = &mut style.marquee {
                let playing = std::mem::replace(marquee.playing.as_mut(), Expr::Bool(true));
                *marquee.playing = folder.fold_expr(playing);
            }
            Some(Node::Layout {
                kind,
                gap,
                style,
                children: folder.fold_nodes(children),
            })
        }
        Node::ScrollView {
            axis,
            hide_horizontal_scrollbar,
            hide_vertical_scrollbar,
            mut style,
            component_ref,
            children,
        } => {
            style.corner_radius_expression = style
                .corner_radius_expression
                .map(|radius| Box::new(folder.fold_expr(*radius)));
            style.effects.scale_expression = style
                .effects
                .scale_expression
                .map(|scale| Box::new(folder.fold_expr(*scale)));
            style.effects.scale_y_expression = style
                .effects
                .scale_y_expression
                .map(|scale_y| Box::new(folder.fold_expr(*scale_y)));
            style.effects.opacity_expression = style
                .effects
                .opacity_expression
                .map(|opacity| Box::new(folder.fold_expr(*opacity)));
            style.effects.offset_y_expression = style
                .effects
                .offset_y_expression
                .map(|offset_y| Box::new(folder.fold_expr(*offset_y)));
            if let Some(marquee) = &mut style.marquee {
                let playing = std::mem::replace(marquee.playing.as_mut(), Expr::Bool(true));
                *marquee.playing = folder.fold_expr(playing);
            }
            Some(Node::ScrollView {
                axis,
                hide_horizontal_scrollbar,
                hide_vertical_scrollbar,
                style,
                component_ref: component_ref.map(|reference| folder.fold_expr(reference)),
                children: folder.fold_nodes(children),
            })
        }
        Node::Toolbar {
            placement,
            children,
        } => Some(Node::Toolbar {
            placement,
            children: folder.fold_nodes(children),
        }),
        Node::KeyboardAware {
            dismiss,
            behavior,
            hide_horizontal_scrollbar,
            hide_vertical_scrollbar,
            children,
        } => Some(Node::KeyboardAware {
            dismiss,
            behavior,
            hide_horizontal_scrollbar,
            hide_vertical_scrollbar,
            children: folder.fold_nodes(children),
        }),
        Node::BottomSheet {
            component_ref,
            content_sized,
            content_padding,
            corner_radius,
            detents,
            initial_detent,
            show_drag_indicator,
            allows_dismissal,
            allows_background_interaction,
            should_dismiss_on_back_press,
            background_color,
            scrim_color,
            on_dismiss,
            children,
        } => Some(Node::BottomSheet {
            component_ref: folder.fold_expr(component_ref),
            content_sized,
            content_padding,
            corner_radius,
            detents,
            initial_detent,
            show_drag_indicator,
            allows_dismissal,
            allows_background_interaction,
            should_dismiss_on_back_press,
            background_color: background_color.map(|color| fold_color_expression(color, folder)),
            scrim_color: scrim_color.map(|color| fold_color_expression(color, folder)),
            on_dismiss: folder.fold_actions(on_dismiss),
            children: folder.fold_nodes(children),
        }),
        Node::Dialog {
            state,
            title,
            message,
            children,
        } => Some(Node::Dialog {
            state,
            title: folder.fold_expr(title),
            message: folder.fold_expr(message),
            children: folder.fold_nodes(children),
        }),
        Node::ConfirmationDialog {
            state,
            title,
            children,
        } => Some(Node::ConfirmationDialog {
            state,
            title: folder.fold_expr(title),
            children: folder.fold_nodes(children),
        }),
        Node::Pressable {
            disabled,
            haptic,
            fill_max_size,
            children,
            actions,
            double_tap_actions,
            long_press_duration_ms,
            long_press_actions,
            context_menu,
            drag_parameters,
            drag_actions,
            pinch_parameter,
            pinch_actions,
        } => Some(Node::Pressable {
            disabled: folder.fold_expr(disabled),
            haptic,
            fill_max_size,
            children: folder.fold_nodes(children),
            actions: folder.fold_actions(actions),
            double_tap_actions: folder.fold_actions(double_tap_actions),
            long_press_duration_ms: folder.fold_expr(long_press_duration_ms),
            long_press_actions: folder.fold_actions(long_press_actions),
            context_menu: folder.fold_nodes(context_menu),
            drag_parameters,
            drag_actions: folder.fold_actions(drag_actions),
            pinch_parameter,
            pinch_actions: folder.fold_actions(pinch_actions),
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
        Node::NavigationSplitView {
            detail_visible,
            sidebar,
            detail,
        } => Some(Node::NavigationSplitView {
            detail_visible,
            sidebar: folder.fold_nodes(sidebar),
            detail: folder.fold_nodes(detail),
        }),
        Node::Accessibility {
            label,
            hint,
            value,
            role,
            children,
        } => Some(Node::Accessibility {
            label: folder.fold_expr(label),
            hint: hint.map(|h| folder.fold_expr(h)),
            value: value.map(|value| folder.fold_expr(value)),
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
        Node::AppBottomBar {
            state,
            tint,
            tabs,
            custom_bar,
        } => Some(Node::AppBottomBar {
            state,
            tint: tint.map(|tint| match tint {
                crate::ColorExpression::Dynamic(color) => {
                    crate::ColorExpression::Dynamic(Box::new(folder.fold_expr(*color)))
                }
                other => other,
            }),
            tabs: tabs
                .into_iter()
                .map(|tab| BottomBarTab {
                    index: tab.index,
                    label: tab.label,
                    comment: tab.comment,
                    icon: tab.icon,
                    badge: tab.badge,
                    role: tab.role,
                    navigation_title: tab.navigation_title,
                    large_title: tab.large_title,
                    search_state: tab.search_state,
                    search_prompt: tab.search_prompt,
                    children: folder.fold_nodes(tab.children),
                })
                .collect(),
            custom_bar: custom_bar.map(|nodes| folder.fold_nodes(nodes)),
        }),
        Node::PagePager { state, pages } => Some(Node::PagePager {
            state,
            pages: pages
                .into_iter()
                .map(|page| folder.fold_nodes(page))
                .collect(),
        }),
        Node::FastList { plan } => Some(Node::FastList {
            plan: Box::new(fold_list_plan(*plan, folder)),
        }),
        Node::If {
            condition,
            then_body,
            else_body,
            transition,
            transition_animation,
        } => Some(Node::If {
            condition: folder.fold_expr(condition),
            then_body: folder.fold_nodes(then_body),
            else_body: else_body.map(|b| folder.fold_nodes(b)),
            transition,
            transition_animation,
        }),
        Node::When {
            value,
            cases,
            else_body,
            transition,
            transition_animation,
        } => Some(Node::When {
            value: folder.fold_expr(value),
            cases: cases
                .into_iter()
                .map(|case| WhenCase {
                    value: folder.fold_expr(case.value),
                    body: folder.fold_nodes(case.body),
                })
                .collect(),
            else_body: else_body.map(|body| folder.fold_nodes(body)),
            transition,
            transition_animation,
        }),
        Node::Text { value, mut style } => {
            style.reveal = style.reveal.map(|mut reveal| {
                reveal.segments = folder.fold_expr(reveal.segments);
                reveal.position = folder.fold_expr(reveal.position);
                reveal.is_active = folder.fold_expr(reveal.is_active);
                reveal.is_playing = folder.fold_expr(reveal.is_playing);
                reveal.reveal_enabled = folder.fold_expr(reveal.reveal_enabled);
                reveal.seek_revision = folder.fold_expr(reveal.seek_revision);
                reveal
            });
            style.effects.scale_expression = style
                .effects
                .scale_expression
                .map(|scale| Box::new(folder.fold_expr(*scale)));
            style.effects.scale_y_expression = style
                .effects
                .scale_y_expression
                .map(|scale_y| Box::new(folder.fold_expr(*scale_y)));
            style.effects.opacity_expression = style
                .effects
                .opacity_expression
                .map(|opacity| Box::new(folder.fold_expr(*opacity)));
            style.effects.offset_y_expression = style
                .effects
                .offset_y_expression
                .map(|offset_y| Box::new(folder.fold_expr(*offset_y)));
            Some(Node::Text {
                value: folder.fold_expr(value),
                style,
            })
        }
        Node::ContentUnavailable {
            title,
            icon,
            description,
            title_color,
            description_color,
            icon_tint,
            background,
        } => Some(Node::ContentUnavailable {
            title: folder.fold_expr(title),
            icon,
            description: description.map(|description| folder.fold_expr(description)),
            title_color: title_color.map(|color| fold_color_expression(color, folder)),
            description_color: description_color.map(|color| fold_color_expression(color, folder)),
            icon_tint: icon_tint.map(|color| fold_color_expression(color, folder)),
            background: background.map(|color| fold_color_expression(color, folder)),
        }),
        Node::ProgressBar { progress, style } => Some(Node::ProgressBar {
            progress: progress.map(|progress| folder.fold_expr(progress)),
            style: style.map(|style| {
                Box::new(crate::ProgressBarStyle {
                    tint: style.tint.map(|color| fold_color_expression(color, folder)),
                    track_color: style
                        .track_color
                        .map(|color| fold_color_expression(color, folder)),
                    thickness: style.thickness,
                    corner_radius: style.corner_radius,
                    android_wave: style
                        .android_wave
                        .map(|expression| folder.fold_expr(expression)),
                })
            }),
        }),
        Node::ProgressRing { progress, style } => Some(Node::ProgressRing {
            progress: progress.map(|progress| folder.fold_expr(progress)),
            style: style.map(|style| {
                Box::new(crate::ProgressRingStyle {
                    tint: style.tint.map(|color| fold_color_expression(color, folder)),
                    track_color: style
                        .track_color
                        .map(|color| fold_color_expression(color, folder)),
                    size: style.size,
                    thickness: style.thickness,
                    android_wave: style
                        .android_wave
                        .map(|expression| folder.fold_expr(expression)),
                })
            }),
        }),
        Node::SegmentedControl {
            items,
            state,
            animation,
            tint,
            background,
            text_color,
            selected_text_color,
            font_size,
            font_weight,
            height,
            corner_radius,
            border_color,
            border_width,
        } => Some(Node::SegmentedControl {
            items: folder.fold_expr(items),
            state,
            animation,
            tint: tint.map(|color| fold_color_expression(color, folder)),
            background: background.map(|color| fold_color_expression(color, folder)),
            text_color: text_color.map(|color| fold_color_expression(color, folder)),
            selected_text_color: selected_text_color
                .map(|color| fold_color_expression(color, folder)),
            font_size,
            font_weight,
            height,
            corner_radius,
            border_color: border_color.map(|color| fold_color_expression(color, folder)),
            border_width,
        }),
        Node::Picker {
            items,
            state,
            icon,
            item_icons,
            label,
            tint,
        } => Some(Node::Picker {
            items: folder.fold_expr(items),
            state,
            icon,
            item_icons,
            label: label.map(|label| folder.fold_expr(label)),
            tint: tint.map(|tint| match tint {
                crate::ColorExpression::Dynamic(color) => {
                    crate::ColorExpression::Dynamic(Box::new(folder.fold_expr(*color)))
                }
                other => other,
            }),
        }),
        Node::PickerDate { props } => {
            let crate::PickerDateProps {
                picker_type,
                style,
                selection,
                timestamp_state,
                start_timestamp_state,
                end_timestamp_state,
                minimum,
                maximum,
                tint,
                text_color,
                background_color,
                picker_background_color,
                border_color,
                border_width,
                corner_radius,
                locale,
                time_zone,
                is_24_hour,
                enabled,
                accessibility_label,
            } = *props;
            Some(Node::PickerDate {
                props: Box::new(crate::PickerDateProps {
                    picker_type,
                    style,
                    selection,
                    timestamp_state,
                    start_timestamp_state,
                    end_timestamp_state,
                    minimum: minimum.map(|value| folder.fold_expr(value)),
                    maximum: maximum.map(|value| folder.fold_expr(value)),
                    tint: tint.map(|color| fold_color_expression(color, folder)),
                    text_color: text_color.map(|color| fold_color_expression(color, folder)),
                    background_color: background_color
                        .map(|color| fold_color_expression(color, folder)),
                    picker_background_color: picker_background_color
                        .map(|color| fold_color_expression(color, folder)),
                    border_color: border_color.map(|color| fold_color_expression(color, folder)),
                    border_width,
                    corner_radius,
                    locale: locale.map(|value| folder.fold_expr(value)),
                    time_zone: time_zone.map(|value| folder.fold_expr(value)),
                    is_24_hour: is_24_hour.map(|value| folder.fold_expr(value)),
                    enabled: enabled.map(|value| folder.fold_expr(value)),
                    accessibility_label: accessibility_label.map(|value| folder.fold_expr(value)),
                }),
            })
        }
        Node::Button {
            label,
            icon,
            loading,
            disabled,
            style,
            size,
            shape,
            tint,
            glass,
            accessibility_label,
            actions,
        } => Some(Node::Button {
            label: folder.fold_expr(label),
            icon,
            loading: loading.map(|l| folder.fold_expr(l)),
            disabled: disabled.map(|d| folder.fold_expr(d)),
            style,
            size,
            shape,
            tint: tint.map(|tint| match tint {
                crate::ColorExpression::Dynamic(color) => {
                    crate::ColorExpression::Dynamic(Box::new(folder.fold_expr(*color)))
                }
                other => other,
            }),
            glass,
            accessibility_label: accessibility_label
                .map(|value| Box::new(folder.fold_expr(*value))),
            actions: folder.fold_actions(actions),
        }),
        Node::TextInput {
            state,
            placeholder,
            keyboard,
            secure,
            multiline,
            autofill,
            return_key,
            autocorrect,
            capitalization,
            focused,
            component_ref,
            max_length,
            font,
            font_size,
            font_family,
            text_color,
            placeholder_color,
            clearable,
            min_lines,
            max_lines,
            searchable,
            actions,
            on_focus,
            on_change,
            comment,
        } => Some(Node::TextInput {
            state,
            placeholder,
            comment,
            keyboard,
            secure,
            multiline,
            autofill,
            return_key,
            autocorrect,
            capitalization,
            focused: focused.map(|value| folder.fold_expr(value)),
            component_ref: component_ref.map(|value| folder.fold_expr(value)),
            max_length,
            font,
            font_size,
            font_family,
            text_color,
            placeholder_color,
            clearable,
            min_lines,
            max_lines,
            searchable,
            actions: folder.fold_actions(actions),
            on_focus: on_focus.map(|actions| folder.fold_actions(actions)),
            on_change: on_change.map(|mut change| {
                change.actions = folder.fold_actions(change.actions);
                change
            }),
        }),
        Node::ComponentCall {
            name,
            arguments,
            component_ref,
            children,
        } => Some(Node::ComponentCall {
            name,
            arguments: arguments
                .into_iter()
                .map(|(name, arg)| (name, folder.fold_expr(arg)))
                .collect(),
            component_ref: component_ref.map(|reference| folder.fold_expr(reference)),
            children: children.map(|c| folder.fold_nodes(c)),
        }),
        Node::NativeComponentCall {
            namespace,
            name,
            arguments,
            component_ref,
            children,
            event_handlers,
        } => Some(Node::NativeComponentCall {
            namespace,
            name,
            arguments: arguments
                .into_iter()
                .map(|(name, arg)| (name, folder.fold_expr(arg)))
                .collect(),
            component_ref: component_ref.map(|reference| folder.fold_expr(reference)),
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
        Node::Appearance { mode, children } => Some(Node::Appearance {
            mode: folder.fold_expr(mode),
            children: folder.fold_nodes(children),
        }),
        Node::NavigationStack {
            root,
            arguments,
            show_navigation_bar,
            title_mode,
            title_alignment,
        } => Some(Node::NavigationStack {
            root,
            arguments: arguments
                .into_iter()
                .map(|arg| folder.fold_expr(arg))
                .collect(),
            show_navigation_bar,
            title_mode,
            title_alignment,
        }),
        Node::NavigationBack { label } => Some(Node::NavigationBack {
            label: folder.fold_expr(label),
        }),
        Node::Image {
            source,
            description,
            scale,
            placeholder,
            show_loading_indicator,
            max_height,
            fill_max_size,
            shared_element,
        } => Some(Node::Image {
            source: match source {
                crate::ImageSource::RemoteUrl(url) => {
                    crate::ImageSource::RemoteUrl(folder.fold_expr(url))
                }
                crate::ImageSource::LocalFile(url) => {
                    crate::ImageSource::LocalFile(folder.fold_expr(url))
                }
                other => other,
            },
            description,
            scale,
            placeholder,
            show_loading_indicator,
            max_height,
            fill_max_size,
            shared_element: shared_element.map(|id| folder.fold_expr(id)),
        }),
        Node::Shape {
            shape,
            width,
            height,
            fill,
            width_expression,
            height_expression,
            animation,
        } => Some(Node::Shape {
            shape,
            width,
            height,
            fill,
            width_expression: width_expression.map(|expression| folder.fold_expr(expression)),
            height_expression: height_expression.map(|expression| folder.fold_expr(expression)),
            animation,
        }),
        Node::Content
        | Node::Spacer { .. }
        | Node::Divider { .. }
        | Node::LinearGradient { .. }
        | Node::StatusBar { .. }
        | Node::Direction { .. } => Some(node),
        Node::SystemIcon {
            icon,
            description,
            size,
            tint,
        } => Some(Node::SystemIcon {
            icon,
            description,
            size,
            tint: match tint {
                crate::ColorExpression::Dynamic(color) => {
                    crate::ColorExpression::Dynamic(Box::new(folder.fold_expr(*color)))
                }
                other => other,
            },
        }),
        Node::FontIcon {
            font_family,
            glyph,
            description,
            size,
            tint,
        } => Some(Node::FontIcon {
            font_family,
            glyph: folder.fold_expr(glyph),
            description,
            size: folder.fold_expr(size),
            tint: match tint {
                crate::ColorExpression::Dynamic(color) => {
                    crate::ColorExpression::Dynamic(Box::new(folder.fold_expr(*color)))
                }
                other => other,
            },
        }),
    }
}

fn fold_color_expression<F: IrFolder>(
    color: crate::ColorExpression,
    folder: &mut F,
) -> crate::ColorExpression {
    match color {
        crate::ColorExpression::Dynamic(expression) => {
            crate::ColorExpression::Dynamic(Box::new(folder.fold_expr(*expression)))
        }
        other => other,
    }
}

pub fn fold_list_plan<F: IrFolder>(plan: ListPlan, folder: &mut F) -> ListPlan {
    match plan {
        ListPlan::Count { count, common } => ListPlan::Count {
            count: folder.fold_expr(count),
            common: ListCommon {
                key: common.key.map(|k| folder.fold_expr(k)),
                component_ref: common
                    .component_ref
                    .map(|reference| folder.fold_expr(reference)),
                scroll_enabled: common
                    .scroll_enabled
                    .map(|enabled| folder.fold_expr(enabled)),
                children: folder.fold_nodes(common.children),
                on_end_reached: common.on_end_reached.map(|a| folder.fold_actions(a)),
                on_scroll: common.on_scroll.map(|a| folder.fold_actions(a)),
                on_move: common.on_move.map(|callback| FastListMove {
                    from: callback.from,
                    to: callback.to,
                    enabled: folder.fold_expr(callback.enabled),
                    actions: folder.fold_actions(callback.actions),
                }),
                swipe_actions: common.swipe_actions.map(|nodes| folder.fold_nodes(nodes)),
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
                component_ref: common
                    .component_ref
                    .map(|reference| folder.fold_expr(reference)),
                scroll_enabled: common
                    .scroll_enabled
                    .map(|enabled| folder.fold_expr(enabled)),
                children: folder.fold_nodes(common.children),
                on_end_reached: common.on_end_reached.map(|a| folder.fold_actions(a)),
                on_scroll: common.on_scroll.map(|a| folder.fold_actions(a)),
                on_move: common.on_move.map(|callback| FastListMove {
                    from: callback.from,
                    to: callback.to,
                    enabled: folder.fold_expr(callback.enabled),
                    actions: folder.fold_actions(callback.actions),
                }),
                swipe_actions: common.swipe_actions.map(|nodes| folder.fold_nodes(nodes)),
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
                component_ref: common
                    .component_ref
                    .map(|reference| folder.fold_expr(reference)),
                scroll_enabled: common
                    .scroll_enabled
                    .map(|enabled| folder.fold_expr(enabled)),
                children: folder.fold_nodes(common.children),
                section_header: common.section_header.map(|h| folder.fold_nodes(h)),
                swipe_actions: common.swipe_actions.map(|nodes| folder.fold_nodes(nodes)),
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
        Expr::LocalizedText {
            key,
            value,
            comment,
        } => Expr::LocalizedText {
            key,
            value: Box::new(folder.fold_expr(*value)),
            comment,
        },
        Expr::Located {
            source_file,
            source_span,
            expression,
        } => Expr::Located {
            source_file,
            source_span,
            expression: Box::new(folder.fold_expr(*expression)),
        },
        Expr::Add(left, right, ty) => Expr::Add(
            Box::new(folder.fold_expr(*left)),
            Box::new(folder.fold_expr(*right)),
            ty,
        ),
        Expr::Concat(left, right) => Expr::Concat(
            Box::new(folder.fold_expr(*left)),
            Box::new(folder.fold_expr(*right)),
        ),
        Expr::Arithmetic {
            op,
            left,
            right,
            ty,
        } => Expr::Arithmetic {
            op,
            left: Box::new(folder.fold_expr(*left)),
            right: Box::new(folder.fold_expr(*right)),
            ty,
        },
        Expr::Negate { value, ty } => Expr::Negate {
            value: Box::new(folder.fold_expr(*value)),
            ty,
        },
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
            is_throwing,
            is_constructor,
        } => Expr::Call {
            name,
            arguments: arguments
                .into_iter()
                .map(|arg| folder.fold_expr(arg))
                .collect(),
            return_type,
            is_async,
            is_throwing,
            is_constructor,
        },
        Expr::PluginEnumConstructor {
            namespace,
            enum_name,
            case_name,
            payload_names,
            arguments,
            return_type,
        } => Expr::PluginEnumConstructor {
            namespace,
            enum_name,
            case_name,
            payload_names,
            arguments: arguments
                .into_iter()
                .map(|argument| folder.fold_expr(argument))
                .collect(),
            return_type,
        },
        Expr::PluginEnumOptionalConstructor {
            namespace,
            enum_name,
            case_name,
            null_case_name,
            payload_name,
            value,
            return_type,
        } => Expr::PluginEnumOptionalConstructor {
            namespace,
            enum_name,
            case_name,
            null_case_name,
            payload_name,
            value: Box::new(folder.fold_expr(*value)),
            return_type,
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
        Expr::CollectionUtility {
            operation,
            collection,
            start,
            end,
            inclusive,
            element_type,
        } => Expr::CollectionUtility {
            operation,
            collection: Box::new(folder.fold_expr(*collection)),
            start: start.map(|start| Box::new(folder.fold_expr(*start))),
            end: end.map(|end| Box::new(folder.fold_expr(*end))),
            inclusive,
            element_type,
        },
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            value_type,
        } => Expr::Conditional {
            condition: Box::new(folder.fold_expr(*condition)),
            then_value: Box::new(folder.fold_expr(*then_value)),
            else_value: Box::new(folder.fold_expr(*else_value)),
            value_type,
        },
        Expr::Closure { parameters, body } => Expr::Closure {
            parameters,
            body: Box::new(folder.fold_expr(*body)),
        },
        Expr::NativeCall {
            source_span,
            namespace,
            name,
            receiver,
            arguments,
            codecs,
            return_type,
            is_async,
            is_throwing,
        } => Expr::NativeCall {
            source_span,
            namespace,
            name,
            receiver: receiver.map(|recv| Box::new(folder.fold_expr(*recv))),
            arguments: arguments
                .into_iter()
                .map(|(param, arg)| (param, folder.fold_expr(arg)))
                .collect(),
            codecs,
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
        Expr::FileReadBytes { path } => Expr::FileReadBytes {
            path: Box::new(folder.fold_expr(*path)),
        },
        Expr::FileWriteText { path, contents } => Expr::FileWriteText {
            path: Box::new(folder.fold_expr(*path)),
            contents: Box::new(folder.fold_expr(*contents)),
        },
        Expr::FileWriteBytes { path, contents } => Expr::FileWriteBytes {
            path: Box::new(folder.fold_expr(*path)),
            contents: Box::new(folder.fold_expr(*contents)),
        },
        Expr::FileDelete { path } => Expr::FileDelete {
            path: Box::new(folder.fold_expr(*path)),
        },
        Expr::BytesFromText { text } => Expr::BytesFromText {
            text: Box::new(folder.fold_expr(*text)),
        },
        Expr::BytesFromArray { values } => Expr::BytesFromArray {
            values: Box::new(folder.fold_expr(*values)),
        },
        Expr::TimeCall {
            method,
            arguments,
            return_type,
            is_async,
        } => Expr::TimeCall {
            method,
            arguments: arguments
                .into_iter()
                .map(|argument| folder.fold_expr(argument))
                .collect(),
            return_type,
            is_async,
        },
        Expr::LogCall { method, message } => Expr::LogCall {
            method,
            message: Box::new(folder.fold_expr(*message)),
        },
        Expr::BytesCount { bytes } => Expr::BytesCount {
            bytes: Box::new(folder.fold_expr(*bytes)),
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
        | Expr::ComponentRefCreate { .. }
        | Expr::Bool(_)
        | Expr::Number { .. }
        | Expr::State(_, _)
        | Expr::AnimatedState(_, _)
        | Expr::EnumValue { .. }
        | Expr::PluginEnumValue { .. }
        | Expr::Null(_)
        | Expr::IsRegularWidth
        | Expr::IsCompactWidth
        | Expr::IsRegularHeight
        | Expr::IsCompactHeight
        | Expr::ScreenWidth
        | Expr::ScreenHeight
        | Expr::SystemInset(_)
        | Expr::This(_) => expr,
    }
}

pub fn fold_action_children<F: IrFolder>(action: Action, folder: &mut F) -> Option<Action> {
    match action {
        Action::Expression(expr) => Some(Action::Expression(folder.fold_expr(expr))),
        Action::NavigationPush {
            destination,
            arguments,
        } => Some(Action::NavigationPush {
            destination,
            arguments: arguments
                .into_iter()
                .map(|argument| folder.fold_expr(argument))
                .collect(),
        }),
        Action::NavigationBack => Some(Action::NavigationBack),
        Action::NavigationCommand(cmd) => match *cmd {
            crate::NavigationCommand::Push {
                navigator,
                route,
                arguments,
            } => Some(Action::NavigationCommand(Box::new(
                crate::NavigationCommand::Push {
                    navigator,
                    route,
                    arguments: arguments
                        .into_iter()
                        .map(|arg| folder.fold_expr(arg))
                        .collect(),
                },
            ))),
            crate::NavigationCommand::ReplaceTop {
                navigator,
                route,
                arguments,
            } => Some(Action::NavigationCommand(Box::new(
                crate::NavigationCommand::ReplaceTop {
                    navigator,
                    route,
                    arguments: arguments
                        .into_iter()
                        .map(|arg| folder.fold_expr(arg))
                        .collect(),
                },
            ))),
            crate::NavigationCommand::Reset {
                navigator,
                route,
                arguments,
            } => Some(Action::NavigationCommand(Box::new(
                crate::NavigationCommand::Reset {
                    navigator,
                    route,
                    arguments: arguments
                        .into_iter()
                        .map(|arg| folder.fold_expr(arg))
                        .collect(),
                },
            ))),
            other => Some(Action::NavigationCommand(Box::new(other))),
        },
        Action::Let { name, ty, value } => Some(Action::Let {
            name,
            ty,
            value: folder.fold_expr(value),
        }),
        Action::Return { value } => Some(Action::Return {
            value: folder.fold_expr(value),
        }),
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
        Action::NetworkStatusSubscribe { parameter, actions } => {
            Some(Action::NetworkStatusSubscribe {
                parameter,
                actions: folder.fold_actions(actions),
            })
        }
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
        Action::TaskLaunch {
            handle,
            executor,
            actions,
        } => Some(Action::TaskLaunch {
            handle,
            executor,
            actions: folder.fold_actions(actions),
        }),
        Action::WithAnimation {
            animation,
            animated_states,
            actions,
        } => Some(Action::WithAnimation {
            animation,
            animated_states,
            actions: folder.fold_actions(actions),
        }),
        Action::TaskCancel { .. } => Some(action),
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
    request.cache_policy = Box::new(folder.fold_expr(*request.cache_policy));
    request.follow_redirects = Box::new(folder.fold_expr(*request.follow_redirects));
    request.max_response_bytes = Box::new(folder.fold_expr(*request.max_response_bytes));
    request.certificate_pins = Box::new(folder.fold_expr(*request.certificate_pins));
    request.body = request.body.map(|body| Box::new(folder.fold_expr(*body)));
    request
}

#[cfg(test)]
mod tests {
    use super::{
        contains_scrollable, resolve_scroll_owner, scroll_owners, walk_actions,
        walk_callback_actions, walk_callback_expressions,
    };
    use crate::adaptive_layout::{
        LayoutEdge, ScrollAxis, ScrollOwnerId, ScrollOwnerKind, ScrollOwnerResolution,
    };
    use crate::{Action, Expr, NativeComponentEventHandler, Node, NumericType, Type};

    fn form() -> Node {
        Node::Form {
            show_dividers: true,
            hide_horizontal_scrollbar: false,
            hide_vertical_scrollbar: false,
            background: None,
            children: Vec::new(),
        }
    }

    fn fast_list(axis: crate::ListAxis) -> Node {
        Node::FastList {
            plan: Box::new(crate::ListPlan::Count {
                count: Expr::Number {
                    raw: "0".to_owned(),
                    ty: NumericType::Int32,
                },
                common: crate::ListCommon {
                    axis,
                    show_dividers: true,
                    hide_horizontal_scrollbar: false,
                    hide_vertical_scrollbar: false,
                    reverse_layout: false,
                    page_snap: false,
                    item_extent: None,
                    gap: 0.0,
                    scroll_enabled: None,
                    index: "index".to_owned(),
                    key: None,
                    scroll_position: None,
                    component_ref: None,
                    children: Vec::new(),
                    on_end_reached: None,
                    on_scroll: None,
                    on_scroll_begin_drag: None,
                    on_move: None,
                    swipe_actions: None,
                    sticky_header: None,
                    refresh: None,
                },
            }),
        }
    }

    fn scroll_view(axis: ScrollAxis) -> Node {
        Node::ScrollView {
            axis,
            hide_horizontal_scrollbar: true,
            hide_vertical_scrollbar: true,
            style: crate::ViewStyle::default(),
            component_ref: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn fast_list_scroll_owner_reports_its_declared_axis() {
        let owners = scroll_owners(&[
            fast_list(crate::ListAxis::Vertical),
            fast_list(crate::ListAxis::Horizontal),
            fast_list(crate::ListAxis::Grid { columns: 2 }),
        ]);
        assert_eq!(owners.len(), 3);
        assert_eq!(owners[0].axes, ScrollAxis::Vertical);
        assert_eq!(owners[1].axes, ScrollAxis::Horizontal);
        assert_eq!(owners[2].axes, ScrollAxis::Both);
        assert!(contains_scrollable(&[fast_list(
            crate::ListAxis::Horizontal
        )]));
    }

    #[test]
    fn scroll_view_owner_reports_its_single_axis_for_safe_area_edges() {
        let vertical = scroll_view(ScrollAxis::Vertical);
        let horizontal = scroll_view(ScrollAxis::Horizontal);
        let owners = scroll_owners(&[vertical.clone(), horizontal.clone()]);
        assert_eq!(owners.len(), 2);
        assert_eq!(owners[0].kind, ScrollOwnerKind::ScrollView);
        assert_eq!(owners[0].axes, ScrollAxis::Vertical);
        assert_eq!(owners[1].kind, ScrollOwnerKind::ScrollView);
        assert_eq!(owners[1].axes, ScrollAxis::Horizontal);
        assert!(matches!(
            resolve_scroll_owner(std::slice::from_ref(&vertical), LayoutEdge::Top, None),
            ScrollOwnerResolution::Unique(owner) if owner.kind == ScrollOwnerKind::ScrollView
        ));
        assert_eq!(
            resolve_scroll_owner(std::slice::from_ref(&horizontal), LayoutEdge::Top, None),
            ScrollOwnerResolution::None
        );
        assert!(matches!(
            resolve_scroll_owner(std::slice::from_ref(&horizontal), LayoutEdge::Leading, None),
            ScrollOwnerResolution::Unique(owner) if owner.axes == ScrollAxis::Horizontal
        ));
        assert!(contains_scrollable(std::slice::from_ref(&vertical)));
    }

    #[test]
    fn scroll_owner_analysis_tracks_kind_axis_and_stable_preorder_ids() {
        let nodes = [
            form(),
            Node::PagePager {
                state: "page".to_owned(),
                pages: Vec::new(),
            },
            Node::KeyboardAware {
                dismiss: crate::KeyboardDismissMode::Never,
                behavior: crate::adaptive_layout::KeyboardBehavior::Stay,
                hide_horizontal_scrollbar: false,
                hide_vertical_scrollbar: false,
                children: vec![form()],
            },
            Node::KeyboardAware {
                dismiss: crate::KeyboardDismissMode::Never,
                behavior: crate::adaptive_layout::KeyboardBehavior::Stay,
                hide_horizontal_scrollbar: false,
                hide_vertical_scrollbar: false,
                children: Vec::new(),
            },
        ];
        let owners = scroll_owners(&nodes);
        assert_eq!(owners.len(), 4);
        assert_eq!(owners[0].id, ScrollOwnerId(0));
        assert_eq!(owners[0].kind, ScrollOwnerKind::Form);
        assert_eq!(owners[0].axes, ScrollAxis::Vertical);
        assert_eq!(owners[1].kind, ScrollOwnerKind::PagePager);
        assert_eq!(owners[1].axes, ScrollAxis::Horizontal);
        assert_eq!(owners[2].kind, ScrollOwnerKind::Form);
        assert_eq!(owners[2].id, ScrollOwnerId(2));
        assert_eq!(owners[3].kind, ScrollOwnerKind::KeyboardAware);
        assert_eq!(owners[3].axes, ScrollAxis::Vertical);
        assert_eq!(owners[3].id, ScrollOwnerId(3));
    }

    #[test]
    fn scroll_owner_resolution_is_edge_aware_and_never_guesses() {
        let vertical = form();
        let horizontal = Node::PagePager {
            state: "page".to_owned(),
            pages: Vec::new(),
        };
        let unique_nodes = [vertical.clone(), horizontal.clone()];
        assert!(matches!(
            resolve_scroll_owner(&unique_nodes, LayoutEdge::Top, None),
            ScrollOwnerResolution::Unique(owner) if owner.kind == ScrollOwnerKind::Form
        ));
        assert!(matches!(
            resolve_scroll_owner(&unique_nodes, LayoutEdge::Leading, None),
            ScrollOwnerResolution::Unique(owner) if owner.kind == ScrollOwnerKind::PagePager
        ));
        assert_eq!(
            resolve_scroll_owner(std::slice::from_ref(&horizontal), LayoutEdge::Top, None),
            ScrollOwnerResolution::None
        );
        assert!(matches!(
            resolve_scroll_owner(&[vertical.clone(), form()], LayoutEdge::Bottom, None),
            ScrollOwnerResolution::Ambiguous(owners) if owners.len() == 2
        ));
        assert!(matches!(
            resolve_scroll_owner(&unique_nodes, LayoutEdge::Top, Some(ScrollOwnerId(1))),
            ScrollOwnerResolution::InvalidReference(ScrollOwnerId(1))
        ));
        assert!(matches!(
            resolve_scroll_owner(&[vertical], LayoutEdge::Bottom, Some(ScrollOwnerId(0))),
            ScrollOwnerResolution::Unique(owner) if owner.id == ScrollOwnerId(0)
        ));
    }

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
            actions: vec![Action::Expression(Expr::State(
                "nested_handler_state".to_owned(),
                Type::String,
            ))],
        }];
        let nodes = [
            Node::Button {
                label: Expr::String("play".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                style: None,
                size: None,
                shape: None,
                tint: None,
                glass: false,
                accessibility_label: None,
                actions: nested,
            },
            Node::NativeComponentCall {
                namespace: "Video".to_owned(),
                name: "VideoView".to_owned(),
                arguments: Vec::new(),
                component_ref: None,
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
                Some(Action::Expression(Expr::State(name, _))) => name.as_str(),
                _ => "other",
            };
            callback_kinds.push(kind.to_owned());
        });

        assert_eq!(
            callback_kinds,
            ["subscription", "nested_handler_state", "component"]
        );

        let mut outer_states = Vec::new();
        walk_callback_expressions(nodes_button_actions(&nodes[0]), &mut |expression| {
            if let Expr::State(name, _) = expression {
                outer_states.push(name.clone());
            }
        });
        assert_eq!(outer_states, ["player"]);

        let mut all_action_states = Vec::new();
        walk_actions(nodes_button_actions(&nodes[0]), &mut |expression| {
            if let Expr::State(name, _) = expression {
                all_action_states.push(name.clone());
            }
        });
        assert_eq!(all_action_states, ["player", "nested_handler_state"]);
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
            gap: 0.0,
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
                        style: None,
                        size: None,
                        shape: None,
                        tint: None,
                        glass: false,
                        accessibility_label: None,
                        actions: vec![Action::Assign {
                            name: "count".to_owned(),
                            value: Expr::State(
                                "step".to_owned(),
                                Type::Numeric(NumericType::Int64),
                            ),
                        }],
                    }],
                    else_body: None,
                    transition: None,
                    transition_animation: None,
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
            gap: 0.0,
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
