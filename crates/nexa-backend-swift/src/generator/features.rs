use std::collections::HashSet;

use nexa_ir::walk::walk_nodes;
use nexa_ir::{ColorValue, Module, Node};

#[derive(Default)]
pub(super) struct Features {
    pub(super) uses_fast_list: bool,
    pub(super) app_uses_adaptive_color: bool,
    components_using_adaptive_color: HashSet<String>,
}

impl Features {
    pub(super) fn analyze(module: &Module) -> Self {
        let mut features = Self::default();
        walk_nodes(&module.body, &mut |node| features.record_app_node(node));
        for screen in &module.screens {
            walk_nodes(&screen.body, &mut |node| features.record_app_node(node));
        }
        for component in &module.components {
            let mut uses_adaptive_color = false;
            walk_nodes(&component.body, &mut |node| {
                features.record_list_usage(node);
                uses_adaptive_color |= node_uses_adaptive_color(node);
            });
            if uses_adaptive_color {
                features
                    .components_using_adaptive_color
                    .insert(component.name.clone());
            }
        }
        features
    }

    pub(super) fn component_uses_adaptive_color(&self, name: &str) -> bool {
        self.components_using_adaptive_color.contains(name)
    }

    fn record_app_node(&mut self, node: &Node) {
        self.record_list_usage(node);
        self.app_uses_adaptive_color |= node_uses_adaptive_color(node);
    }

    fn record_list_usage(&mut self, node: &Node) {
        self.uses_fast_list |= matches!(node, Node::FastList { .. });
    }
}

fn node_uses_adaptive_color(node: &Node) -> bool {
    let color = match node {
        Node::Layout { style, .. } => style.background,
        Node::Text { style, .. } => style.color,
        _ => None,
    };
    color.is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }))
}
