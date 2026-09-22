use std::collections::HashSet;

use nexa_ir::walk::{walk_expression, walk_ir};
use nexa_ir::{ColorValue, Expr, Module, Node};

#[derive(Default)]
pub(super) struct Features {
    pub(super) uses_fast_list: bool,
    pub(super) uses_link: bool,
    pub(super) uses_remote_image: bool,
    pub(super) app_uses_adaptive_color: bool,
    pub(super) app_uses_regular_width: bool,
    components_using_adaptive_color: HashSet<String>,
    components_using_regular_width: HashSet<String>,
}

impl Features {
    pub(super) fn analyze(module: &Module) -> Self {
        let mut features = Self::default();
        let mut app_uses_regular_width = false;

        for state in &module.states {
            walk_expression(&state.initial, &mut |expr| {
                app_uses_regular_width |= matches!(expr, Expr::IsRegularWidth);
            });
        }
        walk_ir(
            &module.body,
            &mut |node| features.record_app_node(node),
            &mut |expr| app_uses_regular_width |= matches!(expr, Expr::IsRegularWidth),
        );
        for screen in &module.screens {
            walk_ir(
                &screen.body,
                &mut |node| features.record_app_node(node),
                &mut |expr| app_uses_regular_width |= matches!(expr, Expr::IsRegularWidth),
            );
        }
        features.app_uses_regular_width = app_uses_regular_width;

        for component in &module.components {
            let mut uses_adaptive_color = false;
            let mut uses_regular_width = false;
            for state in &component.states {
                walk_expression(&state.initial, &mut |expr| {
                    uses_regular_width |= matches!(expr, Expr::IsRegularWidth);
                });
            }
            walk_ir(
                &component.body,
                &mut |node| {
                    features.record_list_usage(node);
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
                &mut |expr| uses_regular_width |= matches!(expr, Expr::IsRegularWidth),
            );
            if uses_adaptive_color {
                features
                    .components_using_adaptive_color
                    .insert(component.name.clone());
            }
            if uses_regular_width {
                features
                    .components_using_regular_width
                    .insert(component.name.clone());
            }
        }
        features
    }

    pub(super) fn component_uses_adaptive_color(&self, name: &str) -> bool {
        self.components_using_adaptive_color.contains(name)
    }

    pub(super) fn component_uses_regular_width(&self, name: &str) -> bool {
        self.components_using_regular_width.contains(name)
    }

    fn record_app_node(&mut self, node: &Node) {
        self.record_list_usage(node);
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
        self.uses_fast_list |= matches!(node, Node::FastList { .. });
    }
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
