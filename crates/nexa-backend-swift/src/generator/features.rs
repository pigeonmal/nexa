use std::collections::HashSet;

use nexa_ir::walk::{walk_actions, walk_expression, walk_ir};
use nexa_ir::{ColorValue, Expr, Module, Node, Permission};

#[derive(Default)]
pub(super) struct Features {
    pub(super) uses_fast_list: bool,
    pub(super) uses_vertical_list: bool,
    pub(super) uses_horizontal_list: bool,
    pub(super) uses_grid_list: bool,
    pub(super) uses_sectioned_list: bool,
    pub(super) uses_sticky_header: bool,
    pub(super) uses_scroll_events: bool,
    pub(super) uses_link: bool,
    pub(super) uses_navigation_back: bool,
    pub(super) uses_remote_image: bool,
    pub(super) uses_native_library: bool,
    pub(super) uses_network_api: bool,
    pub(super) uses_path_api: bool,
    pub(super) uses_file_api: bool,
    pub(super) uses_file_async: bool,
    pub(super) uses_permissions: bool,
    pub(super) uses_permission_request: bool,
    pub(super) used_permissions: HashSet<Permission>,
    pub(super) dynamic_permission: bool,
    pub(super) uses_haptic: bool,
    pub(super) app_uses_adaptive_color: bool,
    pub(super) app_uses_size_class: bool,
    components_using_adaptive_color: HashSet<String>,
    components_using_size_class: HashSet<String>,
}

impl Features {
    pub(super) fn analyze(module: &Module) -> Self {
        let mut features = Self::default();
        let capabilities = nexa_ir::capabilities::analyze(module);
        let mut app_uses_size_class = false;
        let mut uses_permissions = false;
        let mut uses_permission_request = false;

        for state in &module.states {
            walk_expression(&state.initial, &mut |expr| {
                app_uses_size_class |= matches!(
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
                uses_permissions |= uses_permissions_call(expr);
                uses_permission_request |= uses_permission_request_call(expr);
            },
        );
        for screen in &module.screens {
            for state in &screen.states {
                walk_expression(&state.initial, &mut |expr| {
                    app_uses_size_class |= matches!(
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
                    uses_permissions |= uses_permissions_call(expr);
                    uses_permission_request |= uses_permission_request_call(expr);
                });
            }
            walk_ir(
                &component.body,
                &mut |node| {
                    features.record_list_usage(node);
                    features.uses_haptic |= node_uses_haptic(node);
                    features.uses_link |= matches!(node, Node::Link { .. });
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
                    uses_permissions |= uses_permissions_call(expr);
                    uses_permission_request |= uses_permission_request_call(expr);
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
        features.uses_remote_image = capabilities.uses_remote_image;
        features.uses_network_api = capabilities.uses_network_api;
        features.uses_path_api = capabilities.uses_path_api;
        features.uses_file_api = capabilities.uses_file_api;
        features.uses_file_async = capabilities.uses_file_async;
        features.uses_native_library =
            features.uses_network_transport() || features.uses_path_api || features.uses_file_api;
        features.uses_permissions = uses_permissions;
        features.uses_permission_request = uses_permission_request;
        collect_permission_usage(module, &mut features);
        features
    }

    pub(super) fn uses_network_transport(&self) -> bool {
        self.uses_network_api || self.uses_remote_image
    }

    pub(super) fn uses_permission(&self, permission: Permission) -> bool {
        self.dynamic_permission || self.used_permissions.contains(&permission)
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
        self.uses_navigation_back |= matches!(node, Node::NavigationBack { .. });
        self.app_uses_adaptive_color |= node_uses_adaptive_color(node);
    }

    fn record_list_usage(&mut self, node: &Node) {
        let Node::FastList {
            axis,
            source,
            sticky_header,
            on_scroll,
            ..
        } = node
        else {
            return;
        };
        self.uses_fast_list = true;
        if matches!(source, nexa_ir::ListSource::Sections { .. }) {
            self.uses_sectioned_list = true;
        } else {
            match axis {
                nexa_ir::ListAxis::Vertical => self.uses_vertical_list = true,
                nexa_ir::ListAxis::Horizontal => self.uses_horizontal_list = true,
                nexa_ir::ListAxis::Grid { .. } => self.uses_grid_list = true,
            }
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
