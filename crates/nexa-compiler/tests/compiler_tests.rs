use std::{fs, path::Path};

use nexa_ir::{
    Action, ArithmeticOp, Expr, MemberKind, Node, NumericType, Type,
    walk::{walk_actions, walk_ir},
};

use nexa_compiler::{Target, compile, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

#[test]
fn arithmetic_lowers_with_numeric_types_and_folds_constants() {
    let module = compile(
        r#"
        app Arithmetic {
            state result: Int32 = 0

            fn computed() -> Int32 {
                return 2 + 3 * 4 - 10 / 2 % 3
            }

            body {
                Button("Calculate") {
                    result = -((result + 1) * 2 - 3) / 2 % 2
                    result *= computed()
                }
            }
        }
        "#,
    )
    .expect("typed arithmetic should compile");

    let [function] = module.functions.as_slice() else {
        panic!("the called helper should remain reachable");
    };
    assert!(matches!(
        &function.body,
        Expr::Number { raw, ty: NumericType::Int32 } if raw == "12"
    ));

    let Node::Button { actions, .. } = &module.body[0] else {
        panic!("expected the arithmetic button");
    };
    let [
        Action::Assign {
            value:
                Expr::Arithmetic {
                    op: ArithmeticOp::Remainder,
                    ty: NumericType::Int32,
                    left,
                    ..
                },
            ..
        },
        Action::Assign {
            value:
                Expr::Arithmetic {
                    op: ArithmeticOp::Multiply,
                    ty: NumericType::Int32,
                    left: compound_left,
                    right: call,
                },
            ..
        },
    ] = actions.as_slice()
    else {
        panic!("arithmetic and compound assignment should lower to typed IR");
    };
    assert!(matches!(
        left.as_ref(),
        Expr::Arithmetic {
            op: ArithmeticOp::Divide,
            ty: NumericType::Int32,
            left,
            ..
        } if matches!(left.as_ref(), Expr::Negate { ty: NumericType::Int32, .. })
    ));
    assert!(matches!(
        compound_left.as_ref(),
        Expr::State(name, Type::Numeric(NumericType::Int32)) if name == "result"
    ));
    assert!(matches!(call.as_ref(), Expr::Call { name, .. } if name == "computed"));
}

#[test]
fn string_add_concatenates_and_folds_literals() {
    let module = compile(
        r#"
        app Greeting {
            state title: String = "Nexa" + " " + "1.0"
            state greeting: String = "Nexa"
            state entries: Array<Int32> = [1, 2]

            body {
                Text(title)
                Text(entries.count)
                Text(entries.isEmpty)
                Button("Append") {
                    greeting += " 1.0"
                }
            }
        }
        "#,
    )
    .expect("string addition should compile as concatenation");

    let title = module
        .states
        .iter()
        .find(|state| state.name == "title")
        .expect("the title state should be present");
    assert!(matches!(&title.initial, Expr::String(value) if value == "Nexa 1.0"));
    assert!(module.body.iter().any(|node| matches!(
        node,
        Node::Text {
            value: Expr::Member {
                kind: MemberKind::CollectionCount,
                field_type: Type::Numeric(NumericType::Int32),
                ..
            },
            ..
        }
    )));
    assert!(module.body.iter().any(|node| matches!(
        node,
        Node::Text {
            value: Expr::Member {
                kind: MemberKind::CollectionIsEmpty,
                field_type: Type::Bool,
                ..
            },
            ..
        }
    )));

    let Some(Node::Button { actions, .. }) = module
        .body
        .iter()
        .find(|node| matches!(node, Node::Button { .. }))
    else {
        panic!("expected the append button");
    };
    assert!(matches!(
        actions.first(),
        Some(Action::Assign {
            value: Expr::Concat(left, right),
            ..
        }) if matches!(left.as_ref(), Expr::State(name, Type::String) if name == "greeting")
            && matches!(right.as_ref(), Expr::String(value) if value == " 1.0")
    ));
}

#[test]
fn array_utilities_lower_to_typed_collection_ir() {
    let module = compile(
        r#"
        app ArrayUtilities {
            state values: Array<Int32> = [10, 20, 30, 40]

            body {
                Text(values.random() ?? 0)
                Text(values.shuffled().count)
                Text(values.reverse().count)
                Text(values.slice(1..<3).count)
            }
        }
        "#,
    )
    .expect("array utilities should compile for typed arrays");

    fn find_utility(expression: &Expr) -> Option<&Expr> {
        match expression {
            Expr::CollectionUtility { .. } => Some(expression),
            Expr::Coalesce(left, right) => find_utility(left).or_else(|| find_utility(right)),
            Expr::Member { base, .. } => find_utility(base),
            _ => None,
        }
    }
    let utilities = module
        .body
        .iter()
        .filter_map(|node| match node {
            Node::Text { value, .. } => find_utility(value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(utilities.len(), 4);
    assert!(matches!(
        utilities[0],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Random,
            element_type: Type::Numeric(NumericType::Int32),
            ..
        }
    ));
    assert!(matches!(
        utilities[1],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Shuffled,
            ..
        }
    ));
    assert!(matches!(
        utilities[2],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Reverse,
            ..
        }
    ));
    assert!(matches!(
        utilities[3],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Slice,
            start: Some(start),
            end: Some(end),
            inclusive: false,
            ..
        } if matches!(start.as_ref(), Expr::Number { raw, ty: NumericType::Int32 } if raw == "1")
            && matches!(end.as_ref(), Expr::Number { raw, ty: NumericType::Int32 } if raw == "3")
    ));
}

#[test]
fn conditional_expressions_infer_shared_and_optional_branch_types() {
    let module = compile(
        r#"
        app ConditionalValues {
            state enabled: Bool = true

            fn selected(enabled: Bool) -> String? {
                return if enabled { "ready" } else { null }
            }

            body {
                Text(if enabled { "ready" } else { "waiting" })
                Text(enabled ? "on" : "off")
                Text(selected(enabled) ?? "unavailable")
            }
        }
        "#,
    )
    .expect("value-producing if expressions should infer branch types");

    let function = module
        .functions
        .iter()
        .find(|function| function.name == "selected")
        .expect("the selected helper should be present");
    assert!(matches!(
        &function.body,
        Expr::Conditional {
            value_type: Type::Optional(inner),
            else_value,
            ..
        } if matches!(inner.as_ref(), Type::String)
            && matches!(else_value.as_ref(), Expr::Null(Type::Optional(_)))
    ));

    assert!(matches!(
        &module.body[0],
        Node::Text {
            value: Expr::Conditional {
                value_type: Type::String,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        &module.body[1],
        Node::Text {
            value: Expr::Conditional {
                value_type: Type::String,
                ..
            },
            ..
        }
    ));
}

#[test]
fn constant_conditional_expressions_fold_to_the_selected_branch() {
    let module = compile(
        r#"
        app FoldConditional {
            state label: String = if true { "ready" } else { "unreachable" }
            body { Text(label) }
        }
        "#,
    )
    .expect("constant conditional branches should compile");

    let label = module
        .states
        .iter()
        .find(|state| state.name == "label")
        .expect("the label state should be present");
    assert!(matches!(&label.initial, Expr::String(value) if value == "ready"));
}

#[test]
fn chained_text_styles_lower_to_native_text_style_fields() {
    let module = compile(
        r#"
        app ChainedTextStyles {
            body {
                Text("Hi").fontSize(18).bold().padding(12)
            }
        }
        "#,
    )
    .expect("text style chains should lower to native style fields");

    assert!(matches!(
        &module.body[0],
        Node::Text { value: Expr::String(value), style }
            if value == "Hi"
                && style.font_size == Some(18.0)
                && style.font_weight == Some(nexa_ir::FontWeight::Bold)
                && style.padding == Some(12.0)
    ));
}

#[test]
fn unused_native_plugins_are_pruned_from_both_target_modules() {
    let project = TestProject::new("nexa-plugin-pruning");
    for (directory, id, service) in [
        ("used", "dev.example.used", "Used"),
        ("unused", "dev.example.unused", "Unused"),
    ] {
        let plugin = project.join(directory);
        fs::create_dir_all(&plugin).expect("plugin directory should be created");
        fs::write(
                plugin.join("plugin.config.nx"),
                format!(
                    "plugin {{ schema: 2 id: \"{id}\" version: \"1.0.0\" sources {{ native: \"native.nxid\" }} }}\n"
                ),
            )
            .expect("plugin manifest should be written");
        fs::write(
            plugin.join("native.nxid"),
            format!("service {service} {{ fn ping() }}\n"),
        )
        .expect("plugin contract should be written");
    }
    let entry = project.join("main.nx");
    fs::write(
            &entry,
            "plugin \"used\" as Used\nplugin \"unused\" as Unused\napp Demo { body { Button(\"Ping\") { Used.ping() } } }\n",
        )
        .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("app with both valid plugin contracts should compile")
            .module;
        assert_eq!(
            module
                .plugins
                .iter()
                .map(|plugin| plugin.namespace.as_str())
                .collect::<Vec<_>>(),
            ["Used"],
            "unused plugin declarations must not reach native project generation"
        );
    }
}

#[test]
fn nested_borrowed_components_keep_the_owner_instance_live() {
    let project = TestProject::new("nexa-nested-components");
    let plugin = project.join("resource");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.example.resource\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
        )
        .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        "native class Resource { init() fn play() fn dispose() }\n",
    )
    .expect("native contract should be written");

    let entry = project.join("main.nx");
    fs::write(
        &entry,
        r#"
            plugin "resource" as ResourcePlugin

            component Outer(resource: ResourcePlugin.Resource) {
                body { Inner(resource: resource) }
            }

            component Inner(resource: ResourcePlugin.Resource) {
                body { Button("Use") { resource.play() } }
            }

            app Demo {
                let resource = ResourcePlugin.Resource()
                body {
                    Button("Dispose") { resource.dispose() }
                    Outer(resource: resource)
                }
            }
            "#,
    )
    .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let result = compile_file_with_warnings_for_target(&entry, target);
        let Err(error) = result else {
            panic!(
                "a parent callback cannot dispose an object held by nested views for {target:?}"
            );
        };
        assert!(
            error
                .to_string()
                .contains("is disposed in one callback and used in another"),
            "unexpected diagnostic: {error}"
        );
    }
}

#[test]
fn plugin_package_calls_lower_to_typed_instances_and_qualified_components() {
    let entry =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/video-player-demo.nx");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("VideoPlayer example should lower for each target")
            .module;

        assert_eq!(module.plugins.len(), 1);
        assert_eq!(module.plugins[0].namespace, "VideoPlayer");
        assert_eq!(module.components.len(), 2);
        assert!(module.components.iter().all(|component| {
            component.parameters.iter().any(|parameter| {
                parameter.name == "player"
                    && matches!(
                        &parameter.ty,
                        Type::Plugin { namespace, name }
                            if namespace == "VideoPlayer" && name == "VideoPlayer"
                    )
            })
        }));
        let player_surface = module
            .components
            .iter()
            .find(|component| component.name == "PlayerSurface")
            .expect("the outer player wrapper should remain reachable");
        let [
            Node::ComponentCall {
                name, arguments, ..
            },
        ] = player_surface.body.as_slice()
        else {
            panic!("the outer wrapper should pass its native object to the nested component");
        };
        assert_eq!(name, "PlayerMedia");
        assert!(matches!(
            arguments.first(),
            Some((parameter, Expr::State(value, Type::Plugin { namespace, name })))
                if parameter == "player"
                    && value == "player"
                    && namespace == "VideoPlayer"
                    && name == "VideoPlayer"
        ));
        let player_instances = module
            .states
            .iter()
            .filter(|state| {
                matches!(
                    &state.ty,
                    Type::Plugin { namespace, name }
                        if namespace == "VideoPlayer" && name == "VideoPlayer"
                )
            })
            .count();
        assert_eq!(player_instances, 2);
        assert!(
            module
                .components
                .iter()
                .flat_map(|component| &component.states)
                .any(|state| state.name == "tapped"),
            "native component callback state must survive optimization"
        );
        assert!(
            module
                .states
                .iter()
                .any(|state| state.name == "secondTapped"),
            "app callback state must survive optimization"
        );

        let mut found_components = Vec::new();
        let mut component_event_handlers = Vec::new();
        let mut component_controls = Vec::new();
        let mut component_children = Vec::new();
        for nodes in std::iter::once(&module.body)
            .chain(module.components.iter().map(|component| &component.body))
        {
            walk_ir(
                nodes,
                &mut |node| {
                    if let nexa_ir::Node::NativeComponentCall {
                        namespace,
                        name,
                        arguments,
                        children,
                        event_handlers,
                        ..
                    } = node
                    {
                        found_components.push((namespace.clone(), name.clone()));
                        component_controls.push(arguments.iter().find_map(|(name, value)| {
                            if name == "controls" {
                                Some(matches!(value, Expr::Bool(true)))
                            } else {
                                None
                            }
                        }));
                        component_children.push(children.is_some());
                        component_event_handlers.push(
                            event_handlers
                                .iter()
                                .map(|handler| {
                                    (
                                        handler.property.clone(),
                                        matches!(
                                            handler.actions.as_slice(),
                                            [nexa_ir::Action::Assign {
                                                value: Expr::Bool(true),
                                                ..
                                            }]
                                        ),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }
                },
                &mut |_| {},
            );
        }
        assert_eq!(
            found_components,
            vec![
                ("VideoPlayer".to_owned(), "VideoView".to_owned()),
                ("VideoPlayer".to_owned(), "VideoView".to_owned()),
            ]
        );
        assert_eq!(component_controls, vec![Some(false), Some(true)]);
        assert_eq!(component_children, vec![true, true]);
        assert_eq!(component_event_handlers.len(), 2);
        assert!(component_event_handlers.iter().all(|handlers| {
            handlers.len() == 1 && handlers[0].0 == "onTapped" && handlers[0].1
        }));

        let handled_prepares = module
            .on_appear
            .as_ref()
            .expect("native preparation should run when the view appears")
            .iter()
            .find_map(|action| match action {
                nexa_ir::Action::TryCatch {
                    body,
                    error_catches,
                    catch_body,
                } => Some((body, error_catches, catch_body)),
                _ => None,
            })
            .expect("the example explicitly recovers from native preparation errors");
        assert_eq!(
                handled_prepares
                    .0
                    .iter()
                    .filter(|action| matches!(
                        action,
                        nexa_ir::Action::Expression(Expr::TryAwait(call))
                            if matches!(call.as_ref(), Expr::NativeCall { name, is_throwing: true, .. } if name == "prepare")
                    ))
                    .count(),
                2
            );
        assert!(handled_prepares.2.is_none());
        assert!(matches!(
            handled_prepares.1.as_slice(),
            [
                nexa_ir::ErrorCatchArm { variant, parameters, .. },
                nexa_ir::ErrorCatchArm {
                    variant: payload_variant,
                    parameters: payload_parameters,
                    ..
                }
            ] if variant == "invalidUrl"
                && parameters.is_empty()
                && payload_variant == "decodingFailed"
                && payload_parameters == &[("message".to_owned(), "message".to_owned(), Type::String)]
        ));

        let mut instance_calls = Vec::new();
        for state in &module.states {
            if let Expr::NativeCall {
                receiver: None,
                name,
                ..
            } = &state.initial
            {
                assert_eq!(name, "VideoPlayer");
            }
        }
        walk_ir(&module.body, &mut |_| {}, &mut |expression| {
            if let Expr::NativeCall {
                receiver: Some(_),
                name,
                ..
            } = expression
            {
                instance_calls.push(name.clone());
            }
        });
        if let Some(actions) = &module.on_appear {
            walk_actions(actions, &mut |expression| {
                if let Expr::NativeCall {
                    receiver: Some(_),
                    name,
                    ..
                } = expression
                {
                    instance_calls.push(name.clone());
                }
            });
        }
        assert!(instance_calls.iter().any(|name| name == "play"));
        assert!(instance_calls.iter().any(|name| name == "pause"));
        assert!(instance_calls.iter().any(|name| name == "prepare"));

        let event_subscriptions = module
            .on_appear
            .as_ref()
            .expect("event handlers are registered when the view appears")
            .iter()
            .filter_map(|action| match action {
                nexa_ir::Action::NativeEventSubscribe {
                    receiver: Expr::State(receiver, Type::Plugin { .. }),
                    property,
                    actions,
                    ..
                } => Some((receiver.as_str(), property.as_str(), actions.as_slice())),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(event_subscriptions.len(), 2);
        assert_eq!(event_subscriptions[0].0, "player1");
        assert_eq!(event_subscriptions[1].0, "player2");
        assert!(event_subscriptions.iter().all(|(_, property, handler)| {
            *property == "onEnded"
                && matches!(
                    handler,
                    [nexa_ir::Action::Assign {
                        value: Expr::Bool(true),
                        ..
                    }]
                )
        }));
    }
}

#[test]
fn compiles_result_type_and_try_propagation() {
    let source = r#"
app TestApp {
    enum AppError {
        NotFound,
        Unauthorized
    }

    fn fetch_code() -> Result<Int32, AppError> {
        return Ok(42);
    }

    fn compute() -> Result<Int32, AppError> {
        let code: Int32 = fetch_code()?;
        return Ok(code);
    }

    state status: String = "Ready"
    state res: Result<Int32, AppError> = Ok(0)

    body {
        Button(status) {
            res = compute();
        }
    }
}
"#;
    let module = compile(source).expect("Result and try propagation should compile successfully");
    assert_eq!(module.functions.len(), 2);
    assert!(matches!(
        module.functions[0].return_type,
        Type::Result(_, _)
    ));
    assert!(matches!(
        module.functions[1].return_type,
        Type::Result(_, _)
    ));
}
