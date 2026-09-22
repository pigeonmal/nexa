use std::collections::HashSet;

use nexa_ir::walk::{walk_actions, walk_expression, walk_ir};
use nexa_ir::{ColorValue, Expr, Module, Node};

#[derive(Default)]
pub(super) struct Features {
    pub(super) uses_fast_list: bool,
    pub(super) uses_vertical_list: bool,
    pub(super) uses_horizontal_list: bool,
    pub(super) uses_grid_list: bool,
    pub(super) uses_sticky_header: bool,
    pub(super) uses_scroll_events: bool,
    pub(super) uses_link: bool,
    pub(super) uses_remote_image: bool,
    pub(super) uses_native_library: bool,
    pub(super) uses_permissions: bool,
    pub(super) uses_haptic: bool,
    pub(super) app_uses_adaptive_color: bool,
    pub(super) app_uses_size_class: bool,
    components_using_adaptive_color: HashSet<String>,
    components_using_size_class: HashSet<String>,
}

impl Features {
    pub(super) fn analyze(module: &Module) -> Self {
        let mut features = Self::default();
        let mut app_uses_size_class = false;
        let mut uses_native_library = false;
        let mut uses_permissions = false;

        for state in &module.states {
            walk_expression(&state.initial, &mut |expr| {
                app_uses_size_class |= matches!(
                    expr,
                    Expr::IsRegularWidth
                        | Expr::IsCompactWidth
                        | Expr::IsRegularHeight
                        | Expr::IsCompactHeight
                );
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
        walk_ir(
            &module.body,
            &mut |node| features.record_app_node(node),
            &mut |expr| {
                app_uses_size_class |= matches!(
                    expr,
                    Expr::IsRegularWidth
                        | Expr::IsCompactWidth
                        | Expr::IsRegularHeight
                        | Expr::IsCompactHeight
                );
                uses_native_library |= uses_core_native_library(expr);
                uses_permissions |= uses_permissions_call(expr);
            },
        );
        for screen in &module.screens {
            walk_ir(
                &screen.body,
                &mut |node| features.record_app_node(node),
                &mut |expr| {
                    app_uses_size_class |= matches!(
                        expr,
                        Expr::IsRegularWidth
                            | Expr::IsCompactWidth
                            | Expr::IsRegularHeight
                            | Expr::IsCompactHeight
                    );
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
        features.app_uses_size_class = app_uses_size_class;

        for component in &module.components {
            let mut uses_adaptive_color = false;
            let mut uses_size_class = false;
            for state in &component.states {
                walk_expression(&state.initial, &mut |expr| {
                    uses_size_class |= matches!(
                        expr,
                        Expr::IsRegularWidth
                            | Expr::IsCompactWidth
                            | Expr::IsRegularHeight
                            | Expr::IsCompactHeight
                    );
                    uses_native_library |= uses_core_native_library(expr);
                    uses_permissions |= uses_permissions_call(expr);
                });
            }
            walk_ir(
                &component.body,
                &mut |node| {
                    features.record_list_usage(node);
                    features.uses_haptic |= node_uses_haptic(node);
                    features.uses_link |= matches!(node, Node::Link { .. });
                    features.uses_remote_image |= matches!(
                        node,
                        Node::Image {
                            source: nexa_ir::ImageSource::RemoteUrl(_),
                            ..
                        }
                    );
                    uses_adaptive_color |= node_uses_adaptive_color(node);
                },
                &mut |expr| {
                    uses_size_class |= matches!(
                        expr,
                        Expr::IsRegularWidth
                            | Expr::IsCompactWidth
                            | Expr::IsRegularHeight
                            | Expr::IsCompactHeight
                    );
                    uses_native_library |= uses_core_native_library(expr);
                    uses_permissions |= uses_permissions_call(expr);
                },
            );
            if uses_adaptive_color {
                features
                    .components_using_adaptive_color
                    .insert(component.name.clone());
            }
            if uses_size_class {
                features
                    .components_using_size_class
                    .insert(component.name.clone());
            }
        }
        features.uses_native_library = uses_native_library || features.uses_remote_image;
        features.uses_permissions = uses_permissions;
        features
    }

    pub(super) fn component_uses_adaptive_color(&self, name: &str) -> bool {
        self.components_using_adaptive_color.contains(name)
    }

    pub(super) fn component_uses_size_class(&self, name: &str) -> bool {
        self.components_using_size_class.contains(name)
    }

    fn record_app_node(&mut self, node: &Node) {
        self.record_list_usage(node);
        self.uses_haptic |= node_uses_haptic(node);
        self.uses_link |= matches!(node, Node::Link { .. });
        self.uses_remote_image |= matches!(
            node,
            Node::Image {
                source: nexa_ir::ImageSource::RemoteUrl(_),
                ..
            }
        );
        self.app_uses_adaptive_color |= node_uses_adaptive_color(node);
    }

    fn record_list_usage(&mut self, node: &Node) {
        let Node::FastList {
            axis,
            sticky_header,
            on_scroll,
            ..
        } = node
        else {
            return;
        };
        self.uses_fast_list = true;
        match axis {
            nexa_ir::ListAxis::Vertical => self.uses_vertical_list = true,
            nexa_ir::ListAxis::Horizontal => self.uses_horizontal_list = true,
            nexa_ir::ListAxis::Grid { .. } => self.uses_grid_list = true,
        }
        self.uses_sticky_header |= sticky_header.is_some();
        self.uses_scroll_events |= on_scroll.is_some();
    }
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

fn node_uses_adaptive_color(node: &Node) -> bool {
    match node {
        Node::Layout { style, .. } => [style.background, style.border_color]
            .into_iter()
            .flatten()
            .any(|color| matches!(color, ColorValue::Adaptive { .. })),
        Node::Text { style, .. } => style
            .color
            .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. })),
        _ => false,
    }
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
            if namespace == "Permissions" && matches!(name.as_str(), "status" | "request")
    )
}
