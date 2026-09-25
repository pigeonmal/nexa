//! Platform-neutral capability analysis for generated projects.
//!
//! Backends still decide how a capability is implemented, but they must not
//! independently rediscover whether an application uses a core API. Keeping
//! this analysis in the shared IR makes dependency pruning deterministic for
//! every present and future target.

use std::cell::Cell;

use crate::{Expr, ImageSource, Module, Node, Type};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Capabilities {
    pub uses_remote_image: bool,
    pub uses_network_api: bool,
    pub uses_path_api: bool,
    pub uses_file_api: bool,
    pub uses_file_async: bool,
    pub uses_result: bool,
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
        capabilities.uses_result |= nested.uses_result;
    }

    fn visit_expression(capabilities: &mut Capabilities, expression: &Expr) {
        if matches!(
            expression,
            Expr::ResultOk { .. } | Expr::ResultErr { .. }
        ) {
            capabilities.uses_result = true;
        }
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

    /// Whether a declared type contains `Result` at any depth, including
    /// nested collections and nested struct fields.
    fn type_uses_result(ty: &Type) -> bool {
        match ty {
            Type::Result(_, _) => true,
            Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) => {
                type_uses_result(inner)
            }
            Type::Map(key, value) | Type::Pair(key, value) => {
                type_uses_result(key) || type_uses_result(value)
            }
            Type::Triple(first, second, third) => {
                type_uses_result(first) || type_uses_result(second) || type_uses_result(third)
            }
            Type::Struct { fields, .. } => {
                fields.iter().any(|(_, field)| type_uses_result(field))
            }
            _ => false,
        }
    }

    /// Visit every declared type position in the module exactly once. All
    /// reachable expressions are already visited exactly once by the
    /// expression walks above; result constructors are detected there.
    fn visit_result_types(module: &Module, capabilities: &mut Capabilities) {
        for state in &module.states {
            capabilities.uses_result |= type_uses_result(&state.ty);
        }
        for declaration in &module.structs {
            capabilities.uses_result |= declaration
                .fields
                .iter()
                .any(|field| type_uses_result(&field.ty));
        }
        for function in &module.functions {
            capabilities.uses_result |= type_uses_result(&function.return_type);
            capabilities.uses_result |= function
                .parameters
                .iter()
                .any(|parameter| type_uses_result(&parameter.ty));
            capabilities.uses_result |= function
                .locals
                .iter()
                .any(|local| type_uses_result(&local.ty));
        }
        for screen in &module.screens {
            capabilities.uses_result |= screen
                .parameters
                .iter()
                .any(|parameter| type_uses_result(&parameter.ty));
            capabilities.uses_result |= screen
                .states
                .iter()
                .any(|state| type_uses_result(&state.ty));
        }
        for component in &module.components {
            capabilities.uses_result |= component
                .parameters
                .iter()
                .any(|parameter| type_uses_result(&parameter.ty));
            capabilities.uses_result |= component
                .states
                .iter()
                .any(|state| type_uses_result(&state.ty));
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
    visit_result_types(module, &mut capabilities);
    capabilities
}

#[cfg(test)]
mod tests {
    use super::analyze;
    use crate::{Action, Expr, ImageScale, ImageSource, LayoutKind, Module, Node, Type, ViewStyle};

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

    fn result_type() -> Type {
        Type::Result(
            Box::new(Type::Numeric(crate::NumericType::Int32)),
            Box::new(Type::String),
        )
    }

    fn blank_result_module() -> Module {
        empty_module(Vec::new())
    }

    fn int_state(name: &str, ty: Type) -> crate::State {
        crate::State {
            name: name.to_owned(),
            ty,
            initial: Expr::Number {
                raw: "0".to_owned(),
                ty: crate::NumericType::Int32,
            },
            mutable: true,
        }
    }

    fn blank_screen(name: &str) -> crate::Screen {
        crate::Screen {
            id: crate::ScreenId(0),
            name: name.to_owned(),
            parameters: Vec::new(),
            states: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
        }
    }

    fn blank_component(name: &str) -> crate::Component {
        crate::Component {
            name: name.to_owned(),
            source_file: None,
            parameters: Vec::new(),
            states: Vec::new(),
            body: Vec::new(),
        }
    }

    fn blank_function(name: &str) -> crate::Function {
        crate::Function {
            name: name.to_owned(),
            is_async: false,
            parameters: Vec::new(),
            locals: Vec::new(),
            return_type: Type::Void,
            body: Expr::Bool(true),
        }
    }

    #[test]
    fn modules_without_result_stay_result_free() {
        let mut module = blank_result_module();
        module.states.push(int_state(
            "count",
            Type::Numeric(crate::NumericType::Int32),
        ));
        let capabilities = analyze(&module);
        assert!(!capabilities.uses_result);
    }

    #[test]
    fn result_in_app_state_enables_result_capability() {
        let mut module = blank_result_module();
        module.states.push(int_state("outcome", result_type()));
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_screen_state_enables_result_capability() {
        let mut module = blank_result_module();
        let mut screen = blank_screen("Main");
        screen.states.push(int_state("outcome", result_type()));
        module.screens.push(screen);
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_component_state_enables_result_capability() {
        let mut module = blank_result_module();
        let mut component = blank_component("Panel");
        component.states.push(int_state("outcome", result_type()));
        module.components.push(component);
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_function_local_enables_result_capability() {
        let mut module = blank_result_module();
        let mut function = blank_function("compute");
        function.locals.push(crate::FunctionLocal {
            name: "outcome".to_owned(),
            ty: result_type(),
            initial: Expr::Bool(true),
        });
        module.functions.push(function);
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_function_parameter_enables_result_capability() {
        let mut module = blank_result_module();
        let mut function = blank_function("compute");
        function.parameters.push(crate::FunctionParameter {
            name: "outcome".to_owned(),
            ty: result_type(),
        });
        module.functions.push(function);
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_function_return_enables_result_capability() {
        let mut module = blank_result_module();
        let mut function = blank_function("compute");
        function.return_type = result_type();
        module.functions.push(function);
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_screen_and_component_parameters_enables_result_capability() {
        let mut module = blank_result_module();
        let mut screen = blank_screen("Main");
        screen.parameters.push(crate::FunctionParameter {
            name: "outcome".to_owned(),
            ty: result_type(),
        });
        module.screens.push(screen);
        assert!(analyze(&module).uses_result);

        let mut module = blank_result_module();
        let mut component = blank_component("Panel");
        component.parameters.push(crate::ComponentParameter {
            name: "outcome".to_owned(),
            ty: result_type(),
        });
        module.components.push(component);
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_struct_field_enables_result_capability() {
        let mut module = blank_result_module();
        module.structs.push(crate::StructDecl {
            name: "Outcome".to_owned(),
            fields: vec![crate::StructField {
                name: "result".to_owned(),
                ty: result_type(),
            }],
        });
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_nested_struct_field_enables_result_capability() {
        let mut module = blank_result_module();
        module.states.push(int_state(
            "wrapper",
            Type::Struct {
                name: "Wrapper".to_owned(),
                fields: vec![(
                    "inner".to_owned(),
                    Type::Struct {
                        name: "Inner".to_owned(),
                        fields: vec![("result".to_owned(), result_type())],
                    },
                )],
            },
        ));
        assert!(analyze(&module).uses_result);
    }

    #[test]
    fn result_in_nested_collections_enables_result_capability() {
        for ty in [
            Type::Array(Box::new(result_type())),
            Type::Optional(Box::new(result_type())),
            Type::Set(Box::new(result_type())),
            Type::Map(Box::new(Type::String), Box::new(result_type())),
            Type::Pair(Box::new(result_type()), Box::new(Type::Bool)),
            Type::Triple(
                Box::new(Type::Bool),
                Box::new(result_type()),
                Box::new(Type::Bool),
            ),
        ] {
            let mut module = blank_result_module();
            module.states.push(int_state("outcome", ty));
            assert!(analyze(&module).uses_result);
        }
    }

    #[test]
    fn result_constructors_enable_result_capability_without_declared_types() {
        let ok = Expr::ResultOk {
            value: Box::new(Expr::Number {
                raw: "1".to_owned(),
                ty: crate::NumericType::Int32,
            }),
            value_type: Type::Numeric(crate::NumericType::Int32),
            error_type: Type::String,
        };
        let mut module = blank_result_module();
        module.on_appear = Some(vec![crate::Action::Expression(ok)]);
        assert!(analyze(&module).uses_result);

        let err = Expr::ResultErr {
            error: Box::new(Expr::String("boom".to_owned())),
            value_type: Type::Numeric(crate::NumericType::Int32),
            error_type: Type::String,
        };
        let mut function = blank_function("fail");
        function.body = err;
        let mut module = blank_result_module();
        module.functions.push(function);
        assert!(analyze(&module).uses_result);
    }
}
