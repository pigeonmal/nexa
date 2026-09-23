use std::collections::{BTreeSet, HashMap, HashSet};

use nexa_ir::walk::{contains_scrollable, walk_actions, walk_expression, walk_ir};
use nexa_ir::{
    ColorValue, Component, Expr, LayoutKind, Module, Node, Permission, State, ViewStyle,
};

pub(crate) fn collect_focus_bindings(nodes: &[Node]) -> BTreeSet<String> {
    let mut bindings = BTreeSet::new();
    walk_ir(
        nodes,
        &mut |node| {
            if let Node::TextInput {
                focused: Some(name),
                ..
            } = node
            {
                bindings.insert(name.clone());
            }
        },
        &mut |_| {},
    );
    bindings
}

#[derive(Default)]
pub(crate) struct Features {
    pub(crate) uses_status_bar: bool,
    pub(crate) uses_bottom_bar: bool,
    pub(crate) uses_bottom_sheet: bool,
    pub(crate) uses_bottom_sheet_partial: bool,
    pub(crate) uses_refresh_control: bool,
    pub(crate) uses_refresh_scroll: bool,
    pub(crate) uses_asset: bool,
    pub(crate) uses_tab_icon: bool,
    pub(crate) uses_tab_badge: bool,
    pub(crate) uses_tab_badge_placeholder: bool,
    pub(crate) uses_remote_image: bool,
    pub(crate) uses_native_library: bool,
    pub(crate) uses_network_api: bool,
    pub(crate) uses_path_api: bool,
    pub(crate) uses_file_api: bool,
    pub(crate) uses_file_async: bool,
    pub(crate) uses_permissions: bool,
    pub(crate) uses_permission_request: bool,
    pub(crate) used_permissions: HashSet<Permission>,
    pub(crate) dynamic_permission: bool,
    pub(crate) uses_placeholder: bool,
    pub(crate) uses_navigation_link: bool,
    pub(crate) uses_navigation_back: bool,
    pub(crate) uses_navigation_uri: bool,
    pub(crate) uses_link: bool,
    pub(crate) app_uses_link: bool,
    pub(crate) uses_accessibility: bool,
    pub(crate) uses_accessibility_role: bool,
    pub(crate) uses_accessibility_heading: bool,
    pub(crate) uses_accessibility_hint: bool,
    pub(crate) uses_list: bool,
    pub(crate) uses_linear_list: bool,
    pub(crate) uses_horizontal_list: bool,
    pub(crate) uses_grid_list: bool,
    pub(crate) uses_list_end_reached: bool,
    pub(crate) uses_linear_list_end_reached: bool,
    pub(crate) uses_grid_end_reached: bool,
    pub(crate) uses_list_scroll_position: bool,
    pub(crate) uses_linear_list_scroll_position: bool,
    pub(crate) uses_grid_scroll_position: bool,
    pub(crate) uses_list_scroll_events: bool,
    pub(crate) uses_linear_list_scroll_events: bool,
    pub(crate) uses_grid_scroll_events: bool,
    pub(crate) uses_sticky_header: bool,
    pub(crate) uses_keyboard_aware: bool,
    pub(crate) uses_keyboard_interactive: bool,
    pub(crate) app_uses_keyboard_interactive: bool,
    pub(crate) uses_adaptive_color: bool,
    pub(crate) uses_font_weight: bool,
    pub(crate) uses_text_sp: bool,
    pub(crate) uses_selectable_text: bool,
    pub(crate) uses_button: bool,
    pub(crate) uses_button_icon: bool,
    pub(crate) uses_button_loading: bool,
    pub(crate) uses_text_node: bool,
    pub(crate) uses_text_input: bool,
    pub(crate) uses_text_input_submit: bool,
    pub(crate) uses_focus: bool,
    pub(crate) uses_secure_text_input: bool,
    pub(crate) uses_capitalization: bool,
    pub(crate) uses_switch: bool,
    pub(crate) uses_pressable: bool,
    pub(crate) uses_haptic: bool,
    pub(crate) app_uses_haptic: bool,
    pub(crate) uses_clickable: bool,
    pub(crate) uses_long_press: bool,
    pub(crate) uses_column: bool,
    pub(crate) uses_row: bool,
    pub(crate) uses_box: bool,
    pub(crate) uses_alignment: bool,
    pub(crate) uses_size_class: bool,
    pub(crate) uses_arrangement: bool,
    pub(crate) uses_modifier: bool,
    pub(crate) uses_background: bool,
    pub(crate) uses_border: bool,
    pub(crate) uses_padding: bool,
    pub(crate) uses_width: bool,
    pub(crate) uses_height: bool,
    pub(crate) uses_width_in: bool,
    pub(crate) uses_height_in: bool,
    pub(crate) uses_corner_radius: bool,
    pub(crate) uses_opacity: bool,
    pub(crate) uses_animation: bool,
    pub(crate) uses_animation_spring: bool,
    pub(crate) uses_animation_ease_in: bool,
    pub(crate) uses_animation_ease_out: bool,
    pub(crate) uses_animation_ease_in_out: bool,
    pub(crate) uses_animation_linear: bool,
    pub(crate) uses_color: bool,
    pub(crate) uses_dp: bool,
    pub(crate) uses_mutable_state: bool,
    pub(crate) uses_native_class_instance: bool,
    pub(crate) uses_mutable_collection: bool,
    pub(crate) uses_mutable_list: bool,
    pub(crate) uses_mutable_set: bool,
    pub(crate) uses_mutable_map: bool,
    pub(crate) uses_mutable_int_state: bool,
    pub(crate) uses_mutable_long_state: bool,
    pub(crate) uses_mutable_float_state: bool,
    pub(crate) uses_mutable_generic_state: bool,
    component_theme: HashSet<String>,
    components_using_navigation: HashSet<String>,
    components_using_link: HashSet<String>,
    components_using_haptic: HashSet<String>,
    components_using_keyboard_interactive: HashSet<String>,
}

impl Features {
    pub(crate) fn analyze(module: &Module) -> Self {
        let capabilities = nexa_ir::capabilities::analyze(module);
        let mut features = Self {
            uses_status_bar: module.status_bar.is_some()
                || module
                    .screens
                    .iter()
                    .any(|screen| screen.status_bar.is_some()),
            uses_bottom_bar: false,
            uses_bottom_sheet: false,
            uses_refresh_control: false,
            uses_refresh_scroll: false,
            uses_column: module.body.len() != 1
                || module.screens.iter().any(|screen| screen.body.len() > 1)
                || module
                    .components
                    .iter()
                    .any(|component| component.body.len() > 1),
            ..Self::default()
        };
        let mut uses_size_class = false;
        let mut uses_permissions = false;
        let mut uses_permission_request = false;

        for state in module.states.iter().chain(
            module
                .components
                .iter()
                .flat_map(|component| component.states.iter()),
        ) {
            features.record_state(state);
            walk_expression(&state.initial, &mut |expr| {
                uses_size_class |= matches!(
                    expr,
                    Expr::IsRegularWidth
                        | Expr::IsCompactWidth
                        | Expr::IsRegularHeight
                        | Expr::IsCompactHeight
                );
                uses_permissions |= uses_permissions_call(expr);
                uses_permission_request |= uses_permission_request_call(expr);
            });
        }
        for screen in &module.screens {
            for state in &screen.states {
                features.record_state(state);
                walk_expression(&state.initial, &mut |expr| {
                    uses_size_class |= matches!(
                        expr,
                        Expr::IsRegularWidth
                            | Expr::IsCompactWidth
                            | Expr::IsRegularHeight
                            | Expr::IsCompactHeight
                    );
                    uses_permissions |= uses_permissions_call(expr);
                    uses_permission_request |= uses_permission_request_call(expr);
                });
            }
        }
        for function in &module.functions {
            for local in &function.locals {
                walk_expression(&local.initial, &mut |expr| {
                    uses_permissions |= uses_permissions_call(expr);
                    uses_permission_request |= uses_permission_request_call(expr);
                });
            }
            walk_expression(&function.body, &mut |expr| {
                uses_permissions |= uses_permissions_call(expr);
                uses_permission_request |= uses_permission_request_call(expr);
            });
        }

        let mut direct_theme = HashSet::new();
        let mut components_using_link = HashSet::new();
        let mut components_using_navigation = HashSet::new();
        let mut components_using_haptic = HashSet::new();
        let mut components_using_keyboard_interactive = HashSet::new();
        let mut app_uses_link = false;
        let mut app_uses_haptic = false;
        let mut app_uses_keyboard_interactive = false;
        let mut calls = HashMap::<String, Vec<String>>::with_capacity(module.components.len());
        walk_ir(
            &module.body,
            &mut |node| {
                features.record_node(node);
                app_uses_link |= matches!(node, Node::Link { .. });
                app_uses_haptic |= node_uses_haptic(node);
                app_uses_keyboard_interactive |= node_uses_keyboard_interactive(node);
            },
            &mut |expr| {
                uses_size_class |= matches!(
                    expr,
                    Expr::IsRegularWidth
                        | Expr::IsCompactWidth
                        | Expr::IsRegularHeight
                        | Expr::IsCompactHeight
                );
                uses_permissions |= uses_permissions_call(expr);
                uses_permission_request |= uses_permission_request_call(expr);
            },
        );
        for screen in &module.screens {
            walk_ir(
                &screen.body,
                &mut |node| {
                    features.record_node(node);
                    app_uses_link |= matches!(node, Node::Link { .. });
                    app_uses_haptic |= node_uses_haptic(node);
                    app_uses_keyboard_interactive |= node_uses_keyboard_interactive(node);
                },
                &mut |expr| {
                    uses_size_class |= matches!(
                        expr,
                        Expr::IsRegularWidth
                            | Expr::IsCompactWidth
                            | Expr::IsRegularHeight
                            | Expr::IsCompactHeight
                    );
                    uses_permissions |= uses_permissions_call(expr);
                    uses_permission_request |= uses_permission_request_call(expr);
                },
            );
        }
        for actions in module
            .on_appear
            .iter()
            .chain(module.on_disappear.iter())
            .chain(module.on_active.iter())
            .chain(module.on_inactive.iter())
            .chain(module.on_background.iter())
        {
            walk_actions(actions, &mut |expr| {
                uses_permissions |= uses_permissions_call(expr);
                uses_permission_request |= uses_permission_request_call(expr);
            });
        }
        for screen in &module.screens {
            for actions in screen.on_appear.iter().chain(screen.on_disappear.iter()) {
                walk_actions(actions, &mut |expr| {
                    uses_permissions |= uses_permissions_call(expr);
                    uses_permission_request |= uses_permission_request_call(expr);
                });
            }
        }
        for component in &module.components {
            let mut child_calls = Vec::new();
            let mut uses_theme = false;
            let mut uses_link = false;
            let mut uses_navigation = false;
            let mut uses_haptic = false;
            let mut uses_keyboard_interactive = false;
            walk_ir(
                &component.body,
                &mut |node| {
                    features.record_node(node);
                    match node {
                        Node::ComponentCall { name, .. } => child_calls.push(name.clone()),
                        Node::Layout { style, .. } => {
                            uses_theme |= style
                                .background
                                .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
                        }
                        Node::Text { style, .. } => {
                            uses_theme |= style
                                .color
                                .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
                        }
                        Node::Link { .. } => uses_link = true,
                        Node::NavigationLink { .. } => uses_navigation = true,
                        Node::Pressable {
                            haptic: Some(_), ..
                        } => uses_haptic = true,
                        Node::KeyboardAware { dismiss, .. } => {
                            uses_keyboard_interactive =
                                matches!(dismiss, nexa_ir::KeyboardDismissMode::Interactive)
                        }
                        _ => {}
                    }
                },
                &mut |expr| {
                    uses_size_class |= matches!(
                        expr,
                        Expr::IsRegularWidth
                            | Expr::IsCompactWidth
                            | Expr::IsRegularHeight
                            | Expr::IsCompactHeight
                    );
                    uses_permissions |= uses_permissions_call(expr);
                    uses_permission_request |= uses_permission_request_call(expr);
                },
            );
            if uses_theme {
                direct_theme.insert(component.name.clone());
            }
            if uses_link {
                components_using_link.insert(component.name.clone());
            }
            if uses_navigation {
                components_using_navigation.insert(component.name.clone());
            }
            if uses_haptic {
                components_using_haptic.insert(component.name.clone());
            }
            if uses_keyboard_interactive {
                components_using_keyboard_interactive.insert(component.name.clone());
            }
            calls.insert(component.name.clone(), child_calls);
        }

        features.component_theme =
            components_requiring_theme(&module.components, &direct_theme, &calls);
        features.components_using_navigation = components_requiring_navigation(
            &module.components,
            &components_using_navigation,
            &calls,
        );
        features.components_using_link = components_using_link;
        features.components_using_haptic = components_using_haptic;
        features.components_using_keyboard_interactive = components_using_keyboard_interactive;
        features.app_uses_link = app_uses_link;
        features.app_uses_haptic = app_uses_haptic;
        features.app_uses_keyboard_interactive = app_uses_keyboard_interactive;
        features.uses_size_class = uses_size_class;
        features.uses_remote_image = capabilities.uses_remote_image;
        features.uses_network_api = capabilities.uses_network_api;
        features.uses_path_api = capabilities.uses_path_api;
        features.uses_file_api = capabilities.uses_file_api;
        features.uses_file_async = capabilities.uses_file_async;
        features.uses_navigation_uri = module
            .screens
            .iter()
            .flat_map(|screen| &screen.parameters)
            .any(|parameter| matches!(parameter.ty, nexa_ir::Type::String));
        features.uses_native_library =
            features.uses_network_transport() || features.uses_path_api || features.uses_file_api;
        features.uses_permissions = uses_permissions;
        features.uses_permission_request = uses_permission_request;
        collect_permission_usage(module, &mut features);
        features
    }

    pub(crate) fn uses_network_transport(&self) -> bool {
        self.uses_network_api || self.uses_remote_image
    }

    pub(crate) fn component_requires_system_theme(&self, name: &str) -> bool {
        self.component_theme.contains(name)
    }

    pub(crate) fn component_uses_link(&self, name: &str) -> bool {
        self.components_using_link.contains(name)
    }

    pub(crate) fn component_uses_haptic(&self, name: &str) -> bool {
        self.components_using_haptic.contains(name)
    }

    pub(crate) fn component_uses_keyboard_interactive(&self, name: &str) -> bool {
        self.components_using_keyboard_interactive.contains(name)
    }

    pub(crate) fn component_requires_navigation(&self, name: &str) -> bool {
        self.components_using_navigation.contains(name)
    }

    fn record_state(&mut self, state: &State) {
        if state.is_native_class_instance_binding() {
            self.uses_native_class_instance = true;
            return;
        }
        if !state.mutable {
            return;
        }
        match state.ty {
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Int32) => {
                self.uses_mutable_state = true;
                self.uses_mutable_int_state = true;
            }
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Int64) => {
                self.uses_mutable_state = true;
                self.uses_mutable_long_state = true;
            }
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Float32) => {
                self.uses_mutable_state = true;
                self.uses_mutable_float_state = true;
            }
            nexa_ir::Type::Array(_) => {
                self.uses_mutable_collection = true;
                self.uses_mutable_list = true;
            }
            nexa_ir::Type::Set(_) => {
                self.uses_mutable_collection = true;
                self.uses_mutable_set = true;
            }
            nexa_ir::Type::Map(_, _) => {
                self.uses_mutable_collection = true;
                self.uses_mutable_map = true;
            }
            _ => {
                self.uses_mutable_state = true;
                self.uses_mutable_generic_state = true;
            }
        }
    }

    fn record_node(&mut self, node: &Node) {
        match node {
            Node::StatusBar { .. }
            | Node::Direction { .. }
            | Node::OnAppear { .. }
            | Node::OnDisappear { .. }
            | Node::OnActive { .. }
            | Node::OnInactive { .. }
            | Node::OnBackground { .. } => {}
            Node::Layout {
                kind,
                spacing,
                style,
                ..
            } => {
                match kind {
                    LayoutKind::Column => self.uses_column = true,
                    LayoutKind::Row => self.uses_row = true,
                    LayoutKind::Stack => self.uses_box = true,
                }
                if *spacing > 0.0 {
                    self.uses_dp = true;
                    self.uses_arrangement = true;
                }
                self.record_style(style);
            }
            Node::Text { style, .. } => {
                self.uses_text_node = true;
                self.uses_color |= style.color.is_some();
                self.uses_adaptive_color |= style
                    .color
                    .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
                self.uses_font_weight |= style.font_weight.is_some();
                self.uses_text_sp |= style.font_size.is_some()
                    || style.line_height.is_some()
                    || style.letter_spacing.is_some();
                self.uses_selectable_text |= style.selectable;
            }
            Node::Button { icon, loading, .. } => {
                self.uses_button = true;
                self.uses_button_icon |= icon.is_some();
                self.uses_button_loading |= loading.is_some();
            }
            Node::TextInput {
                secure,
                capitalization,
                focused,
                actions,
                ..
            } => {
                self.uses_text_input = true;
                self.uses_text_input_submit |= !actions.is_empty();
                self.uses_focus |= focused.is_some();
                self.uses_modifier |= focused.is_some();
                self.uses_secure_text_input |= *secure;
                self.uses_capitalization |= capitalization.is_some();
            }
            Node::Switch { .. } => {
                self.uses_switch = true;
                self.uses_modifier = true;
            }
            Node::Image {
                source,
                placeholder,
                ..
            } => {
                self.uses_asset |= matches!(source, nexa_ir::ImageSource::Asset(_));
                self.uses_remote_image |= matches!(source, nexa_ir::ImageSource::RemoteUrl(_));
                self.uses_placeholder |= placeholder.is_some();
            }
            Node::Pressable {
                children,
                haptic,
                long_press_actions,
                ..
            } => {
                self.uses_pressable = true;
                self.uses_haptic |= haptic.is_some();
                self.uses_clickable |= long_press_actions.is_empty();
                self.uses_long_press |= !long_press_actions.is_empty();
                self.uses_box = true;
                self.uses_modifier = true;
                self.record_child_layout(children);
            }
            Node::NavigationLink { children, .. } => {
                self.uses_navigation_link = true;
                self.record_child_layout(children);
            }
            Node::NavigationBack { .. } => {
                self.uses_navigation_back = true;
            }
            Node::Link { children, .. } => {
                self.uses_link = true;
                self.uses_box = true;
                self.uses_modifier = true;
                self.record_child_layout(children);
            }
            Node::Accessibility {
                hint,
                role,
                children,
                ..
            } => {
                self.uses_accessibility = true;
                self.uses_accessibility_hint |= hint.is_some();
                self.uses_accessibility_role |= matches!(
                    role,
                    nexa_ir::AccessibilityRole::Button | nexa_ir::AccessibilityRole::Image
                );
                self.uses_accessibility_heading |=
                    matches!(role, nexa_ir::AccessibilityRole::Header);
                self.uses_box = true;
                self.uses_modifier = true;
                self.record_child_layout(children);
            }
            Node::KeyboardAware { dismiss, .. } => {
                self.uses_keyboard_aware = true;
                self.uses_keyboard_interactive |=
                    matches!(dismiss, nexa_ir::KeyboardDismissMode::Interactive);
                self.uses_column = true;
                self.uses_modifier = true;
            }
            Node::BottomSheet {
                partial, children, ..
            } => {
                self.uses_bottom_sheet = true;
                self.uses_bottom_sheet_partial |= *partial;
                self.record_child_layout(children);
            }
            Node::AppBottomBar { tabs, .. } => {
                self.uses_bottom_bar = true;
                self.uses_column = true;
                self.uses_modifier = true;
                self.uses_padding = true;
                for tab in tabs {
                    self.uses_tab_icon |= tab.icon.is_some();
                    self.uses_tab_badge |= tab.badge.is_some();
                    self.uses_tab_badge_placeholder |= tab.badge.is_some() && tab.icon.is_none();
                    self.record_child_layout(&tab.children);
                }
            }
            Node::RefreshControl { children, .. } => {
                self.uses_refresh_control = true;
                if !contains_scrollable(children) {
                    self.uses_refresh_scroll = true;
                    self.uses_column = true;
                    self.uses_modifier = true;
                }
                self.record_child_layout(children);
            }
            Node::FastList {
                axis,
                item_extent,
                on_end_reached,
                on_scroll,
                refresh,
                scroll_position,
                sticky_header,
                section_header,
                children,
                ..
            } => {
                self.uses_sticky_header |= sticky_header.is_some() || section_header.is_some();
                self.uses_refresh_control |= refresh.is_some();
                self.uses_list_scroll_position |= scroll_position.is_some();
                self.uses_linear_list_scroll_position |=
                    scroll_position.is_some() && !matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_grid_scroll_position |=
                    scroll_position.is_some() && matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_list_scroll_events |= on_scroll.is_some();
                self.uses_linear_list_scroll_events |=
                    on_scroll.is_some() && !matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_grid_scroll_events |=
                    on_scroll.is_some() && matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_list |= matches!(axis, nexa_ir::ListAxis::Vertical);
                self.uses_linear_list |= !matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_horizontal_list |= matches!(axis, nexa_ir::ListAxis::Horizontal);
                self.uses_grid_list |= matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_list_end_reached |= on_end_reached.is_some();
                self.uses_linear_list_end_reached |=
                    on_end_reached.is_some() && !matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_grid_end_reached |=
                    on_end_reached.is_some() && matches!(axis, nexa_ir::ListAxis::Grid { .. });
                self.uses_mutable_state |= on_end_reached.is_some();
                self.uses_mutable_int_state |= on_end_reached.is_some();
                self.uses_mutable_state |= on_scroll.is_some();
                self.uses_mutable_int_state |= on_scroll.is_some();
                if item_extent.is_some() {
                    self.uses_box = true;
                    self.uses_modifier = true;
                    self.uses_dp = true;
                    self.uses_height = true;
                    self.uses_width |= matches!(axis, nexa_ir::ListAxis::Horizontal);
                }
                self.record_child_layout(children);
            }
            Node::If {
                then_body,
                else_body,
                ..
            } => {
                self.record_child_layout(then_body);
                if let Some(else_body) = else_body {
                    self.record_child_layout(else_body);
                }
            }
            Node::Content
            | Node::NavigationStack { .. }
            | Node::When { .. }
            | Node::ComponentCall { .. }
            | Node::NativeComponentCall { .. } => {}
        }
    }

    fn record_child_layout(&mut self, children: &[Node]) {
        if children.len() > 1 {
            self.uses_column = true;
        }
    }

    fn record_style(&mut self, style: &ViewStyle) {
        self.uses_alignment |= style.alignment.is_some();
        self.uses_padding |= style.padding.is_some();
        self.uses_width |= style.width.is_some();
        self.uses_height |= style.height.is_some();
        self.uses_width_in |= style.min_width.is_some() || style.max_width.is_some();
        self.uses_height_in |= style.min_height.is_some() || style.max_height.is_some();
        self.uses_background |= style.background.is_some();
        self.uses_border |= style.border_color.is_some();
        self.uses_corner_radius |= style.corner_radius.is_some();
        self.uses_corner_radius |= style.border_color.is_some();
        self.uses_opacity |= style.opacity.is_some();
        if let Some(animation) = style.animation {
            self.uses_animation = true;
            match animation {
                nexa_ir::AnimationSpec::Spring => self.uses_animation_spring = true,
                nexa_ir::AnimationSpec::EaseIn => self.uses_animation_ease_in = true,
                nexa_ir::AnimationSpec::EaseOut => self.uses_animation_ease_out = true,
                nexa_ir::AnimationSpec::EaseInOut => self.uses_animation_ease_in_out = true,
                nexa_ir::AnimationSpec::Linear => self.uses_animation_linear = true,
            }
        }
        self.uses_modifier |= style.has_modifiers();
        self.uses_color |= style.background.is_some() || style.border_color.is_some();
        self.uses_adaptive_color |= style
            .background
            .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
        self.uses_adaptive_color |= style
            .border_color
            .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
        self.uses_dp |= style.padding.is_some()
            || style.width.is_some()
            || style.height.is_some()
            || style.min_width.is_some()
            || style.max_width.is_some()
            || style.min_height.is_some()
            || style.max_height.is_some()
            || style.corner_radius.is_some()
            || style.border_width.is_some();
    }
}

fn components_requiring_theme(
    components: &[Component],
    direct_theme: &HashSet<String>,
    calls: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut required = HashSet::with_capacity(direct_theme.len());
    let mut completed = HashSet::with_capacity(components.len());
    for component in components {
        component_requires_theme(
            &component.name,
            direct_theme,
            calls,
            &mut required,
            &mut completed,
        );
    }
    required
}

fn components_requiring_navigation(
    components: &[Component],
    direct_navigation: &HashSet<String>,
    calls: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut required = HashSet::with_capacity(direct_navigation.len());
    let mut completed = HashSet::with_capacity(components.len());
    for component in components {
        component_requires_navigation(
            &component.name,
            direct_navigation,
            calls,
            &mut required,
            &mut completed,
        );
    }
    required
}

fn component_requires_navigation(
    name: &str,
    direct_navigation: &HashSet<String>,
    calls: &HashMap<String, Vec<String>>,
    required: &mut HashSet<String>,
    completed: &mut HashSet<String>,
) -> bool {
    if completed.contains(name) {
        return required.contains(name);
    }
    completed.insert(name.to_owned());
    let needs_navigation = direct_navigation.contains(name)
        || calls.get(name).is_some_and(|children| {
            children.iter().any(|child| {
                component_requires_navigation(child, direct_navigation, calls, required, completed)
            })
        });
    if needs_navigation {
        required.insert(name.to_owned());
    }
    needs_navigation
}

fn component_requires_theme(
    name: &str,
    direct_theme: &HashSet<String>,
    calls: &HashMap<String, Vec<String>>,
    required: &mut HashSet<String>,
    completed: &mut HashSet<String>,
) -> bool {
    if completed.contains(name) {
        return required.contains(name);
    }
    completed.insert(name.to_owned());
    let needs_theme = direct_theme.contains(name)
        || calls.get(name).is_some_and(|children| {
            children.iter().any(|child| {
                component_requires_theme(child, direct_theme, calls, required, completed)
            })
        });
    if needs_theme {
        required.insert(name.to_owned());
    }
    needs_theme
}

fn node_uses_haptic(node: &Node) -> bool {
    matches!(
        node,
        Node::Pressable {
            haptic: Some(_),
            ..
        }
    )
}

fn node_uses_keyboard_interactive(node: &Node) -> bool {
    matches!(
        node,
        Node::KeyboardAware {
            dismiss: nexa_ir::KeyboardDismissMode::Interactive,
            ..
        }
    )
}

fn uses_permission_request_call(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::NativeCall { namespace, name, .. }
            if namespace == "Permissions" && name == "request"
    )
}

fn uses_permissions_call(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::NativeCall { namespace, name, .. }
            if namespace == "Permissions" && matches!(name.as_str(), "status" | "request")
    )
}

fn collect_permission_usage(module: &Module, features: &mut Features) {
    let mut visit = |expr: &Expr| record_permission_usage(expr, features);
    for state in &module.states {
        walk_expression(&state.initial, &mut visit);
    }
    for function in &module.functions {
        for local in &function.locals {
            walk_expression(&local.initial, &mut visit);
        }
        walk_expression(&function.body, &mut visit);
    }
    walk_ir(&module.body, &mut |_| {}, &mut visit);
    for actions in module
        .on_appear
        .iter()
        .chain(module.on_disappear.iter())
        .chain(module.on_active.iter())
        .chain(module.on_inactive.iter())
        .chain(module.on_background.iter())
    {
        walk_actions(actions, &mut visit);
    }
    for screen in &module.screens {
        for state in &screen.states {
            walk_expression(&state.initial, &mut visit);
        }
        walk_ir(&screen.body, &mut |_| {}, &mut visit);
        for actions in screen.on_appear.iter().chain(screen.on_disappear.iter()) {
            walk_actions(actions, &mut visit);
        }
    }
    for component in &module.components {
        for state in &component.states {
            walk_expression(&state.initial, &mut visit);
        }
        walk_ir(&component.body, &mut |_| {}, &mut visit);
    }
}

fn record_permission_usage(expr: &Expr, features: &mut Features) {
    let Expr::NativeCall {
        namespace,
        name,
        arguments,
        ..
    } = expr
    else {
        return;
    };
    if namespace != "Permissions" || !matches!(name.as_str(), "status" | "request") {
        return;
    }
    let Some((_, permission)) = arguments.iter().find(|(name, _)| name == "permission") else {
        features.dynamic_permission = true;
        return;
    };
    match permission {
        Expr::EnumValue {
            enum_name,
            case_name,
            ..
        } if enum_name == "Permission" => match case_name.as_str() {
            "Camera" => {
                features.used_permissions.insert(Permission::Camera);
            }
            "Microphone" => {
                features.used_permissions.insert(Permission::Microphone);
            }
            "Photos" => {
                features.used_permissions.insert(Permission::Photos);
            }
            "Location" => {
                features.used_permissions.insert(Permission::Location);
            }
            "Notifications" => {
                features.used_permissions.insert(Permission::Notifications);
            }
            "Contacts" => {
                features.used_permissions.insert(Permission::Contacts);
            }
            "Calendar" => {
                features.used_permissions.insert(Permission::Calendar);
            }
            "Bluetooth" => {
                features.used_permissions.insert(Permission::Bluetooth);
            }
            _ => features.dynamic_permission = true,
        },
        _ => features.dynamic_permission = true,
    }
}
