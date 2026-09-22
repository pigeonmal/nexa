use std::collections::{BTreeSet, HashMap, HashSet};

use nexa_ir::walk::{contains_scrollable, walk_actions, walk_expression, walk_ir};
use nexa_ir::{ColorValue, Component, Expr, LayoutKind, Module, Node, State, ViewStyle};

pub(super) fn collect_focus_bindings(nodes: &[Node]) -> BTreeSet<String> {
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
pub(super) struct Features {
    pub(super) uses_status_bar: bool,
    pub(super) uses_bottom_bar: bool,
    pub(super) uses_bottom_sheet: bool,
    pub(super) uses_refresh_control: bool,
    pub(super) uses_refresh_scroll: bool,
    pub(super) uses_image: bool,
    pub(super) uses_remote_image: bool,
    pub(super) uses_native_library: bool,
    pub(super) uses_permissions: bool,
    pub(super) uses_placeholder: bool,
    pub(super) uses_navigation_link: bool,
    pub(super) uses_link: bool,
    pub(super) app_uses_link: bool,
    pub(super) uses_accessibility: bool,
    pub(super) uses_accessibility_role: bool,
    pub(super) uses_accessibility_heading: bool,
    pub(super) uses_list: bool,
    pub(super) uses_keyboard_aware: bool,
    pub(super) uses_adaptive_color: bool,
    pub(super) uses_font_weight: bool,
    pub(super) uses_text_sp: bool,
    pub(super) uses_selectable_text: bool,
    pub(super) uses_button: bool,
    pub(super) uses_button_loading: bool,
    pub(super) uses_text: bool,
    pub(super) uses_text_input: bool,
    pub(super) uses_text_input_submit: bool,
    pub(super) uses_focus: bool,
    pub(super) uses_secure_text_input: bool,
    pub(super) uses_capitalization: bool,
    pub(super) uses_switch: bool,
    pub(super) uses_pressable: bool,
    pub(super) uses_clickable: bool,
    pub(super) uses_long_press: bool,
    pub(super) uses_column: bool,
    pub(super) uses_row: bool,
    pub(super) uses_box: bool,
    pub(super) uses_alignment: bool,
    pub(super) uses_regular_width: bool,
    pub(super) uses_arrangement: bool,
    pub(super) uses_modifier: bool,
    pub(super) uses_background: bool,
    pub(super) uses_border: bool,
    pub(super) uses_padding: bool,
    pub(super) uses_width: bool,
    pub(super) uses_height: bool,
    pub(super) uses_corner_radius: bool,
    pub(super) uses_opacity: bool,
    pub(super) uses_animation: bool,
    pub(super) uses_animation_spring: bool,
    pub(super) uses_animation_ease_in: bool,
    pub(super) uses_animation_ease_out: bool,
    pub(super) uses_animation_ease_in_out: bool,
    pub(super) uses_animation_linear: bool,
    pub(super) uses_color: bool,
    pub(super) uses_dp: bool,
    pub(super) uses_mutable_state: bool,
    pub(super) uses_mutable_int_state: bool,
    pub(super) uses_mutable_long_state: bool,
    pub(super) uses_mutable_float_state: bool,
    pub(super) uses_mutable_generic_state: bool,
    component_theme: HashSet<String>,
    components_using_link: HashSet<String>,
}

impl Features {
    pub(super) fn analyze(module: &Module) -> Self {
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
        let mut uses_regular_width = false;
        let mut uses_native_library = false;
        let mut uses_permissions = false;

        for state in module.states.iter().chain(
            module
                .components
                .iter()
                .flat_map(|component| component.states.iter()),
        ) {
            features.record_state(state);
            walk_expression(&state.initial, &mut |expr| {
                uses_regular_width |= matches!(expr, Expr::IsRegularWidth);
                uses_native_library |= uses_core_native_library(expr);
                uses_permissions |= uses_permissions_call(expr);
            });
        }
        for function in &module.functions {
            for local in &function.locals {
                walk_expression(&local.initial, &mut |expr| {
                    uses_native_library |= uses_core_native_library(expr);
                    uses_permissions |= uses_permissions_call(expr);
                });
            }
            walk_expression(&function.body, &mut |expr| {
                uses_native_library |= uses_core_native_library(expr);
                uses_permissions |= uses_permissions_call(expr);
            });
        }

        let mut direct_theme = HashSet::new();
        let mut components_using_link = HashSet::new();
        let mut app_uses_link = false;
        let mut calls = HashMap::<String, Vec<String>>::with_capacity(module.components.len());
        walk_ir(
            &module.body,
            &mut |node| {
                features.record_node(node);
                app_uses_link |= matches!(node, Node::Link { .. });
            },
            &mut |expr| {
                uses_regular_width |= matches!(expr, Expr::IsRegularWidth);
                uses_native_library |= uses_core_native_library(expr);
                uses_permissions |= uses_permissions_call(expr);
            },
        );
        for screen in &module.screens {
            walk_ir(
                &screen.body,
                &mut |node| {
                    features.record_node(node);
                    app_uses_link |= matches!(node, Node::Link { .. });
                },
                &mut |expr| {
                    uses_regular_width |= matches!(expr, Expr::IsRegularWidth);
                    uses_native_library |= uses_core_native_library(expr);
                    uses_permissions |= uses_permissions_call(expr);
                },
            );
        }
        for actions in module.on_appear.iter().chain(module.on_disappear.iter()) {
            walk_actions(actions, &mut |expr| {
                uses_native_library |= uses_core_native_library(expr);
                uses_permissions |= uses_permissions_call(expr);
            });
        }
        for screen in &module.screens {
            for actions in screen.on_appear.iter().chain(screen.on_disappear.iter()) {
                walk_actions(actions, &mut |expr| {
                    uses_native_library |= uses_core_native_library(expr);
                    uses_permissions |= uses_permissions_call(expr);
                });
            }
        }
        for component in &module.components {
            let mut child_calls = Vec::new();
            let mut uses_theme = false;
            let mut uses_link = false;
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
                        _ => {}
                    }
                },
                &mut |expr| {
                    uses_regular_width |= matches!(expr, Expr::IsRegularWidth);
                    uses_native_library |= uses_core_native_library(expr);
                    uses_permissions |= uses_permissions_call(expr);
                },
            );
            if uses_theme {
                direct_theme.insert(component.name.clone());
            }
            if uses_link {
                components_using_link.insert(component.name.clone());
            }
            calls.insert(component.name.clone(), child_calls);
        }

        features.component_theme =
            components_requiring_theme(&module.components, &direct_theme, &calls);
        features.components_using_link = components_using_link;
        features.app_uses_link = app_uses_link;
        features.uses_regular_width = uses_regular_width;
        features.uses_native_library = uses_native_library || features.uses_remote_image;
        features.uses_permissions = uses_permissions;
        features
    }

    pub(super) fn component_requires_system_theme(&self, name: &str) -> bool {
        self.component_theme.contains(name)
    }

    pub(super) fn component_uses_link(&self, name: &str) -> bool {
        self.components_using_link.contains(name)
    }

    fn record_state(&mut self, state: &State) {
        if !state.mutable {
            return;
        }
        self.uses_mutable_state = true;
        match state.ty {
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Int32) => {
                self.uses_mutable_int_state = true;
            }
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Int64) => {
                self.uses_mutable_long_state = true;
            }
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Float32) => {
                self.uses_mutable_float_state = true;
            }
            _ => self.uses_mutable_generic_state = true,
        }
    }

    fn record_node(&mut self, node: &Node) {
        match node {
            Node::StatusBar { .. }
            | Node::Direction { .. }
            | Node::OnAppear { .. }
            | Node::OnDisappear { .. } => {}
            Node::Layout {
                kind,
                spacing,
                style,
                ..
            } => {
                match kind {
                    LayoutKind::Column => self.uses_column = true,
                    LayoutKind::Row => self.uses_row = true,
                }
                if *spacing > 0.0 {
                    self.uses_dp = true;
                    self.uses_arrangement = true;
                }
                self.record_style(style);
            }
            Node::Text { style, .. } => {
                self.uses_text = true;
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
            Node::Button { loading, .. } => {
                self.uses_button = true;
                self.uses_text = true;
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
                self.uses_text = true;
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
            Node::Image { placeholder, .. } => {
                self.uses_image = true;
                self.uses_remote_image |= matches!(
                    node,
                    Node::Image {
                        source: nexa_ir::ImageSource::RemoteUrl(_),
                        ..
                    }
                );
                self.uses_placeholder |= placeholder.is_some();
            }
            Node::Pressable {
                children,
                long_press_actions,
                ..
            } => {
                self.uses_pressable = true;
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
            Node::Link { children, .. } => {
                self.uses_link = true;
                self.uses_box = true;
                self.uses_modifier = true;
                self.record_child_layout(children);
            }
            Node::Accessibility { role, children, .. } => {
                self.uses_accessibility = true;
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
            Node::KeyboardAware { .. } => {
                self.uses_keyboard_aware = true;
                self.uses_column = true;
                self.uses_modifier = true;
            }
            Node::BottomSheet { children, .. } => {
                self.uses_bottom_sheet = true;
                self.record_child_layout(children);
            }
            Node::AppBottomBar { tabs, .. } => {
                self.uses_bottom_bar = true;
                self.uses_column = true;
                self.uses_text = true;
                self.uses_modifier = true;
                self.uses_padding = true;
                for tab in tabs {
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
            Node::FastList { children, .. } => {
                self.uses_list = true;
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
            Node::NavigationStack { .. } | Node::When { .. } | Node::ComponentCall { .. } => {}
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

fn uses_core_native_library(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::NativeCall { namespace, .. }
            if matches!(namespace.as_str(), "Network" | "Path" | "File")
    )
}

fn uses_permissions_call(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::NativeCall { namespace, name, .. }
            if namespace == "Permissions" && name == "status"
    )
}
