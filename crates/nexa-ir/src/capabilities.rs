//! Platform-neutral capability analysis for generated projects.
//!
//! Backends still decide how a capability is implemented, but they must not
//! independently rediscover whether an application uses a core API. Keeping
//! this analysis in the shared IR makes dependency pruning deterministic for
//! every present and future target.

use crate::Module;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Capabilities {
    pub uses_remote_image: bool,
    pub uses_network_api: bool,
    pub uses_network_connectivity: bool,
    pub uses_path_api: bool,
    pub uses_file_api: bool,
    pub uses_file_async: bool,
    pub uses_result: bool,
    pub uses_time: bool,
    pub uses_number_formatting: bool,
    pub uses_locale_api: bool,
    pub uses_app_icon_api: bool,
    pub uses_localized_strings: bool,
    pub uses_json_api: bool,
    pub uses_keyboard_api: bool,
    pub uses_crypto_api: bool,
    pub uses_secure_storage_api: bool,
    pub uses_storage_api: bool,
    pub uses_clipboard_api: bool,
    pub uses_haptics_api: bool,
    pub uses_screen_orientation_api: bool,
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
///
/// The rules live in [`crate::facts`]; this delegates to the single analysis
/// pass so every consumer observes identical capabilities.
pub fn analyze(module: &Module) -> Capabilities {
    crate::facts::ModuleFacts::analyze(module).capabilities
}

#[cfg(test)]
mod tests {
    use super::analyze;
    use crate::{Action, Expr, ImageScale, ImageSource, LayoutKind, Module, Node, Type, ViewStyle};

    fn empty_module(body: Vec<Node>) -> Module {
        Module {
            widgets: Vec::new(),
            app_name: "CapabilitiesTest".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
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
            codecs: Vec::new(),
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
                style: None,
                size: None,
                shape: None,
                tint: None,
                glass: false,
                actions: vec![
                    Action::Expression(call("Network", "fetch")),
                    Action::Expression(call("Path", "documents")),
                    Action::Expression(call("File", "readText")),
                    Action::Expression(call("Number", "formatCurrency")),
                    Action::Expression(call("Crypto", "sha256")),
                    Action::Expression(call("Clipboard", "hasText")),
                    Action::Expression(call("Haptics", "selection")),
                    Action::Expression(call("Screen", "lockOrientation")),
                ],
            }],
        }]);

        let capabilities = analyze(&module);
        assert!(capabilities.uses_network_api);
        assert!(capabilities.uses_path_api);
        assert!(capabilities.uses_file_api);
        assert!(capabilities.uses_file_async);
        assert!(capabilities.uses_number_formatting);
        assert!(capabilities.uses_crypto_api);
        assert!(capabilities.uses_clipboard_api);
        assert!(capabilities.uses_haptics_api);
        assert!(capabilities.uses_screen_orientation_api);
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
            max_height: None,
            shared_element: None,
        }]);

        let capabilities = analyze(&module);
        assert!(capabilities.uses_remote_image);
        assert!(capabilities.uses_network_transport());
        assert!(!capabilities.uses_network_api);
        assert!(!capabilities.uses_file_api);
    }

    #[test]
    fn network_status_does_not_enable_request_transport() {
        let module = Module {
            widgets: Vec::new(),
            on_appear: Some(vec![Action::Expression(Expr::NativeCall {
                receiver: None,
                namespace: "Network".to_owned(),
                name: "isOnline".to_owned(),
                arguments: Vec::new(),
                codecs: Vec::new(),
                return_type: crate::Type::Bool,
                is_async: false,
                is_throwing: false,
            })]),
            ..empty_module(Vec::new())
        };

        let capabilities = analyze(&module);
        assert!(capabilities.uses_network_connectivity);
        assert!(!capabilities.uses_network_api);
        assert!(!capabilities.uses_network_transport());
    }

    #[test]
    fn synchronous_file_existence_checks_do_not_enable_async_support() {
        let module = Module {
            widgets: Vec::new(),
            on_appear: Some(vec![Action::Expression(Expr::NativeCall {
                receiver: None,
                namespace: "File".to_owned(),
                name: "exists".to_owned(),
                arguments: Vec::new(),
                codecs: Vec::new(),
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
            receiver: None,
            class_initializers: Vec::new(),
            is_async: true,
            is_throwing: false,
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
                codecs: Vec::new(),
                return_type: crate::Type::String,
                is_async: true,
                is_throwing: false,
            })),
            body_actions: None,
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
                    codecs: Vec::new(),
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
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
        }
    }

    fn blank_function(name: &str) -> crate::Function {
        crate::Function {
            name: name.to_owned(),
            receiver: None,
            class_initializers: Vec::new(),
            is_async: false,
            is_throwing: false,
            parameters: Vec::new(),
            locals: Vec::new(),
            return_type: Type::Void,
            body: Expr::Bool(true),
            body_actions: None,
        }
    }

    #[test]
    fn modules_without_result_stay_result_free() {
        let mut module = blank_result_module();
        module
            .states
            .push(int_state("count", Type::Numeric(crate::NumericType::Int32)));
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
    fn generic_plugin_result_return_and_codec_enable_result_capability() {
        let result = Type::Result(
            Box::new(Type::Numeric(crate::NumericType::Int32)),
            Box::new(Type::Enum("StoreError".to_owned())),
        );
        for (return_type, codecs) in [
            (Type::Optional(Box::new(result.clone())), Vec::new()),
            (
                Type::Bool,
                vec![crate::PluginCodec {
                    ty: result,
                    decodes: false,
                    row_mapper: false,
                }],
            ),
        ] {
            let mut module = blank_result_module();
            module.on_appear = Some(vec![Action::Expression(Expr::NativeCall {
                receiver: None,
                namespace: "Store".to_owned(),
                name: "getObject".to_owned(),
                arguments: Vec::new(),
                codecs,
                return_type,
                is_async: false,
                is_throwing: false,
            })]);
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
