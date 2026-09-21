use std::collections::{HashMap, HashSet};

use nexa_ir::walk::{contains_scrollable, walk_expression, walk_ir};
use nexa_ir::{ColorValue, Component, Expr, LayoutKind, Module, Node, State, ViewStyle};

#[derive(Default)]
pub(super) struct Features {
    pub(super) uses_status_bar: bool,
    pub(super) uses_bottom_sheet: bool,
    pub(super) uses_refresh_control: bool,
    pub(super) uses_refresh_scroll: bool,
    pub(super) uses_image: bool,
    pub(super) uses_remote_image: bool,
    pub(super) uses_placeholder: bool,
    pub(super) uses_navigation_link: bool,
    pub(super) uses_list: bool,
    pub(super) uses_keyboard_aware: bool,
    pub(super) uses_adaptive_color: bool,
    pub(super) uses_font_size: bool,
    pub(super) uses_button: bool,
    pub(super) uses_text: bool,
    pub(super) uses_text_input: bool,
    pub(super) uses_secure_text_input: bool,
    pub(super) uses_capitalization: bool,
    pub(super) uses_switch: bool,
    pub(super) uses_pressable: bool,
    pub(super) uses_column: bool,
    pub(super) uses_row: bool,
    pub(super) uses_box: bool,
    pub(super) uses_alignment: bool,
    pub(super) uses_regular_width: bool,
    pub(super) uses_arrangement: bool,
    pub(super) uses_modifier: bool,
    pub(super) uses_background: bool,
    pub(super) uses_padding: bool,
    pub(super) uses_width: bool,
    pub(super) uses_height: bool,
    pub(super) uses_corner_radius: bool,
    pub(super) uses_opacity: bool,
    pub(super) uses_color: bool,
    pub(super) uses_dp: bool,
    pub(super) uses_mutable_state: bool,
    pub(super) uses_mutable_int_state: bool,
    pub(super) uses_mutable_long_state: bool,
    pub(super) uses_mutable_float_state: bool,
    pub(super) uses_mutable_generic_state: bool,
    component_theme: HashSet<String>,
}

impl Features {
    pub(super) fn analyze(module: &Module) -> Self {
        let mut features = Self {
            uses_status_bar: module.status_bar.is_some(),
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

        for state in module.states.iter().chain(
            module
                .components
                .iter()
                .flat_map(|component| component.states.iter()),
        ) {
            features.record_state(state);
            walk_expression(&state.initial, &mut |expr| {
                uses_regular_width |= matches!(expr, Expr::IsRegularWidth);
            });
        }

        let mut direct_theme = HashSet::new();
        let mut calls = HashMap::<String, Vec<String>>::with_capacity(module.components.len());
        walk_ir(
            &module.body,
            &mut |node| features.record_node(node),
            &mut |expr| uses_regular_width |= matches!(expr, Expr::IsRegularWidth),
        );
        for screen in &module.screens {
            walk_ir(
                &screen.body,
                &mut |node| features.record_node(node),
                &mut |expr| uses_regular_width |= matches!(expr, Expr::IsRegularWidth),
            );
        }
        for component in &module.components {
            let mut child_calls = Vec::new();
            let mut uses_theme = false;
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
                        _ => {}
                    }
                },
                &mut |expr| uses_regular_width |= matches!(expr, Expr::IsRegularWidth),
            );
            if uses_theme {
                direct_theme.insert(component.name.clone());
            }
            calls.insert(component.name.clone(), child_calls);
        }

        features.component_theme =
            components_requiring_theme(&module.components, &direct_theme, &calls);
        features.uses_regular_width = uses_regular_width;
        features
    }

    pub(super) fn component_requires_system_theme(&self, name: &str) -> bool {
        self.component_theme.contains(name)
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
            Node::StatusBar { .. } => {}
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
                if style.font_size.is_some() {
                    self.uses_font_size = true;
                }
            }
            Node::Button { .. } => {
                self.uses_button = true;
                self.uses_text = true;
            }
            Node::TextInput {
                secure,
                capitalization,
                ..
            } => {
                self.uses_text_input = true;
                self.uses_text = true;
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
            Node::Pressable { children, .. } => {
                self.uses_pressable = true;
                self.uses_box = true;
                self.uses_modifier = true;
                self.record_child_layout(children);
            }
            Node::NavigationLink { children, .. } => {
                self.uses_navigation_link = true;
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
            Node::NavigationStack { .. } | Node::ComponentCall { .. } => {}
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
        self.uses_corner_radius |= style.corner_radius.is_some();
        self.uses_opacity |= style.opacity.is_some();
        self.uses_modifier |= style.has_modifiers();
        self.uses_color |= style.background.is_some();
        self.uses_adaptive_color |= style
            .background
            .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
        self.uses_dp |= style.padding.is_some()
            || style.width.is_some()
            || style.height.is_some()
            || style.corner_radius.is_some();
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
