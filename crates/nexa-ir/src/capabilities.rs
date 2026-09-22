//! Platform-neutral capability analysis for generated projects.
//!
//! Backends still decide how a capability is implemented, but they must not
//! independently rediscover whether an application uses a core API. Keeping
//! this analysis in the shared IR makes dependency pruning deterministic for
//! every present and future target.

use std::cell::Cell;

use crate::{Expr, ImageSource, Module, Node};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Capabilities {
    pub uses_remote_image: bool,
    pub uses_network_api: bool,
    pub uses_path_api: bool,
    pub uses_file_api: bool,
    pub uses_file_async: bool,
}

impl Capabilities {
    pub const fn uses_network_transport(self) -> bool {
        self.uses_network_api || self.uses_remote_image
    }
}

/// Collects all core API capabilities from the complete typed module.
///
/// This deliberately runs after semantic reachability pruning. Unused
/// functions/components therefore cannot pull optional native dependencies
/// into a generated project.
pub fn analyze(module: &Module) -> Capabilities {
    let mut capabilities = Capabilities::default();
    fn visit_nodes(nodes: &[Node], capabilities: &mut Capabilities) {
        let mut nested = Capabilities::default();
        let remote_image = Cell::new(false);
        crate::walk::walk_ir(
            nodes,
            &mut |node| {
                if let Node::Image {
                    source: ImageSource::RemoteUrl(_),
                    ..
                } = node
                {
                    remote_image.set(true);
                }
            },
            &mut |expression| visit_expression(&mut nested, expression),
        );
        capabilities.uses_remote_image |= remote_image.get();
        capabilities.uses_network_api |= nested.uses_network_api;
        capabilities.uses_path_api |= nested.uses_path_api;
        capabilities.uses_file_api |= nested.uses_file_api;
        capabilities.uses_file_async |= nested.uses_file_async;
    }

    fn visit_expression(capabilities: &mut Capabilities, expression: &Expr) {
        if let Expr::NativeCall {
            namespace, name, ..
        } = expression
        {
            match namespace.as_str() {
                "Network" => capabilities.uses_network_api = true,
                "Path" => capabilities.uses_path_api = true,
                "File" => {
                    capabilities.uses_file_api = true;
                    capabilities.uses_file_async |= name != "exists";
                }
                _ => {}
            }
        }
    }

    for state in &module.states {
        crate::walk::walk_expression(&state.initial, &mut |expression| {
            visit_expression(&mut capabilities, expression)
        });
    }
    for function in &module.functions {
        for local in &function.locals {
            crate::walk::walk_expression(&local.initial, &mut |expression| {
                visit_expression(&mut capabilities, expression)
            });
        }
        crate::walk::walk_expression(&function.body, &mut |expression| {
            visit_expression(&mut capabilities, expression)
        });
    }
    visit_nodes(&module.body, &mut capabilities);
    for actions in module
        .on_appear
        .iter()
        .chain(module.on_disappear.iter())
        .chain(module.on_active.iter())
        .chain(module.on_inactive.iter())
        .chain(module.on_background.iter())
    {
        crate::walk::walk_actions(actions, &mut |expression| {
            visit_expression(&mut capabilities, expression)
        });
    }
    for screen in &module.screens {
        for state in &screen.states {
            crate::walk::walk_expression(&state.initial, &mut |expression| {
                visit_expression(&mut capabilities, expression)
            });
        }
        visit_nodes(&screen.body, &mut capabilities);
        for actions in screen.on_appear.iter().chain(screen.on_disappear.iter()) {
            crate::walk::walk_actions(actions, &mut |expression| {
                visit_expression(&mut capabilities, expression)
            });
        }
    }
    for component in &module.components {
        for state in &component.states {
            crate::walk::walk_expression(&state.initial, &mut |expression| {
                visit_expression(&mut capabilities, expression)
            });
        }
        visit_nodes(&component.body, &mut capabilities);
    }
    capabilities
}
