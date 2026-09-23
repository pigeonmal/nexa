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

#[cfg(test)]
mod tests {
    use super::analyze;
    use crate::{Action, Expr, ImageScale, ImageSource, LayoutKind, Module, Node, ViewStyle};

    fn empty_module(body: Vec<Node>) -> Module {
        Module {
            app_name: "CapabilitiesTest".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body,
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        }
    }

    #[test]
    fn detects_core_calls_inside_nested_ui_actions() {
        let call = |namespace: &str, name: &str| Expr::NativeCall {
            receiver: None,
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            arguments: Vec::new(),
            return_type: crate::Type::String,
            is_async: true,
            is_throwing: false,
        };
        let module = empty_module(vec![Node::Layout {
            kind: LayoutKind::Column,
            spacing: 0.0,
            style: ViewStyle::default(),
            children: vec![Node::Button {
                label: Expr::String("Load".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                actions: vec![
                    Action::Expression(call("Network", "fetch")),
                    Action::Expression(call("Path", "documents")),
                    Action::Expression(call("File", "readText")),
                ],
            }],
        }]);

        let capabilities = analyze(&module);
        assert!(capabilities.uses_network_api);
        assert!(capabilities.uses_path_api);
        assert!(capabilities.uses_file_api);
        assert!(capabilities.uses_file_async);
    }

    #[test]
    fn remote_images_request_transport_without_counting_as_network_api_calls() {
        let module = empty_module(vec![Node::Image {
            source: ImageSource::RemoteUrl(Expr::String(
                "https://example.test/image.png".to_owned(),
            )),
            description: "Example image".to_owned(),
            scale: ImageScale::Fit,
            placeholder: None,
        }]);

        let capabilities = analyze(&module);
        assert!(capabilities.uses_remote_image);
        assert!(capabilities.uses_network_transport());
        assert!(!capabilities.uses_network_api);
        assert!(!capabilities.uses_file_api);
    }

    #[test]
    fn synchronous_file_existence_checks_do_not_enable_async_support() {
        let module = Module {
            on_appear: Some(vec![Action::Expression(Expr::NativeCall {
                receiver: None,
                namespace: "File".to_owned(),
                name: "exists".to_owned(),
                arguments: Vec::new(),
                return_type: crate::Type::Bool,
                is_async: false,
                is_throwing: false,
            })]),
            ..empty_module(Vec::new())
        };

        let capabilities = analyze(&module);
        assert!(capabilities.uses_file_api);
        assert!(!capabilities.uses_file_async);
    }

    #[test]
    fn async_file_calls_in_function_bodies_enable_async_support() {
        let mut module = empty_module(Vec::new());
        module.functions.push(crate::Function {
            name: "load_text".to_owned(),
            is_async: true,
            parameters: vec![crate::FunctionParameter {
                name: "path".to_owned(),
                ty: crate::Type::String,
            }],
            locals: Vec::new(),
            return_type: crate::Type::String,
            body: Expr::Await(Box::new(Expr::NativeCall {
                receiver: None,
                namespace: "File".to_owned(),
                name: "readText".to_owned(),
                arguments: vec![(
                    "path".to_owned(),
                    Expr::State("path".to_owned(), crate::Type::String),
                )],
                return_type: crate::Type::String,
                is_async: true,
                is_throwing: false,
            })),
        });

        let capabilities = analyze(&module);
        assert!(capabilities.uses_file_api);
        assert!(capabilities.uses_file_async);
    }

    #[test]
    fn nested_native_component_arguments_are_included_in_capability_analysis() {
        let module = empty_module(vec![Node::NativeComponentCall {
            namespace: "Video".to_owned(),
            name: "VideoView".to_owned(),
            arguments: vec![(
                "caption".to_owned(),
                Expr::NativeCall {
                    receiver: None,
                    namespace: "File".to_owned(),
                    name: "readText".to_owned(),
                    arguments: vec![("path".to_owned(), Expr::String("caption.txt".to_owned()))],
                    return_type: crate::Type::String,
                    is_async: true,
                    is_throwing: false,
                },
            )],
            children: None,
            event_handlers: Vec::new(),
        }]);

        let capabilities = analyze(&module);
        assert!(capabilities.uses_file_api);
        assert!(capabilities.uses_file_async);
    }
}
