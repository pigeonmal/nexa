use nexa_syntax::ast::{ArithmeticOp, BinaryOp, ChildBody, Expr, Node, Stmt};

#[test]
fn parses_plugin_import_alias_before_app_declaration() {
    let app = nexa_syntax::parse(
        r#"plugin "./video-player" as Video
        app Demo {
            body { Text("ready") }
        }"#,
    )
    .expect("valid plugin import and app syntax");

    assert_eq!(app.plugins.len(), 1);
    assert_eq!(app.plugins[0].path, "./video-player");
    assert_eq!(app.plugins[0].namespace, "Video");
}

#[test]
fn parses_qualified_native_class_component_parameter_types() {
    let app = nexa_syntax::parse(
        r#"
        component PlayerSurface(player: Video.VideoPlayer) {
            body { Text("player") }
        }
        app Demo {
            body { PlayerSurface(player: player) }
        }
        "#,
    )
    .expect("qualified native class types should parse in component parameters");

    assert!(matches!(
        &app.components[0].parameters[0].ty,
        nexa_syntax::ast::TypeSyntax::Named(name, _) if name == "Video.VideoPlayer"
    ));
}

#[test]
fn parses_native_property_assignment_in_action() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            let player = Player()
            body {
                Button("Set volume") {
                    player.volume = 0.5
                }
            }
        }"#,
    )
    .expect("valid app syntax");

    let Node::ComponentInvocation(invocation) = &app.body[0] else {
        panic!("expected a button node");
    };
    assert_eq!(invocation.name, "Button");
    let ChildBody::Actions(actions) = &invocation.children else {
        panic!("expected button actions");
    };
    assert!(matches!(
        &actions[0],
        Stmt::NativePropertyAssign {
            receiver: Expr::Name(receiver, _),
            property,
            value: Expr::Number(raw, _),
            ..
        } if receiver == "player" && property == "volume" && raw == "0.5"
    ));
}

#[test]
fn parses_try_catch_action_blocks() {
    let app = nexa_syntax::parse(
        r#"
        app Demo {
            state failed = false
            body {
                Button("Load") {
                    try {
                        Download.fetch()
                    } catch {
                        failed = true
                    }
                }
            }
        }
        "#,
    )
    .expect("try/catch action blocks should parse");

    let Node::ComponentInvocation(invocation) = &app.body[0] else {
        panic!("expected a button node");
    };
    assert_eq!(invocation.name, "Button");
    let ChildBody::Actions(actions) = &invocation.children else {
        panic!("expected button actions");
    };
    assert!(
        matches!(actions.as_slice(), [Stmt::TryCatch { body, error_catches, catch_body, .. }]
        if matches!(body.as_slice(), [Stmt::Expression { .. }])
            && error_catches.is_empty()
            && matches!(catch_body.as_deref(), Some([Stmt::Assign { name, .. }]) if name == "failed"))
    );
}

#[test]
fn parses_typed_error_catch_variants_and_payload_bindings() {
    let app = nexa_syntax::parse(
        r#"
        app Demo {
            state failed = false
            state message = ""
            body {
                Button("Load") {
                    try {
                        await Video.prepare()
                    } catch {
                        case Video.PlayerError.invalidUrl {
                            failed = true
                        }
                        case Video.PlayerError.decodingFailed(message) {
                            message = message
                        }
                    }
                }
            }
        }
        "#,
    )
    .expect("typed error catch cases should parse");

    let Node::ComponentInvocation(invocation) = &app.body[0] else {
        panic!("expected a button node");
    };
    assert_eq!(invocation.name, "Button");
    let ChildBody::Actions(actions) = &invocation.children else {
        panic!("expected button actions");
    };
    let [
        Stmt::TryCatch {
            error_catches,
            catch_body,
            ..
        },
    ] = actions.as_slice()
    else {
        panic!("expected a typed try/catch action");
    };
    assert_eq!(error_catches.len(), 2);
    assert_eq!(error_catches[0].namespace, "Video");
    assert_eq!(error_catches[0].error_name, "PlayerError");
    assert_eq!(error_catches[0].variant, "invalidUrl");
    assert!(error_catches[0].bindings.is_empty());
    assert_eq!(error_catches[1].variant, "decodingFailed");
    assert_eq!(error_catches[1].bindings, ["message"]);
    assert!(catch_body.is_none());
}

#[test]
fn parses_qualified_service_calls_in_action_blocks() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            body {
                Button("Register") {
                    Push.register()
                }
            }
        }"#,
    )
    .expect("valid qualified service call");
    let Node::ComponentInvocation(invocation) = &app.body[0] else {
        panic!("expected a button node");
    };
    assert_eq!(invocation.name, "Button");
    let ChildBody::Actions(actions) = &invocation.children else {
        panic!("expected button actions");
    };
    assert!(matches!(
        &actions[0],
        Stmt::Expression {
            expression: Expr::QualifiedCall { namespace, name, .. },
            ..
        } if namespace == "Push" && name == "register"
    ));
}

#[test]
fn parses_instance_event_handlers_with_and_without_payload_bindings() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            let player = Player()
            state finished = false
            body {
                OnAppear {
                    player.ended { finished = true }
                    player.progressChanged { position, duration ->
                        finished = position > 0.0 && duration > 0.0
                    }
                }
            }
        }"#,
    )
    .expect("valid event handler syntax");

    let Node::ComponentInvocation(invocation) = &app.body[0] else {
        panic!("expected an OnAppear node");
    };
    assert_eq!(invocation.name, "OnAppear");
    let ChildBody::Actions(actions) = &invocation.children else {
        panic!("expected OnAppear actions");
    };
    assert!(matches!(
        &actions[0],
        Stmt::NativeEventSubscribe {
            receiver: Expr::Name(receiver, _),
            event,
            parameters,
            actions: handler,
            ..
        } if receiver == "player" && event == "ended" && parameters.is_empty()
            && matches!(&handler[0], Stmt::Assign { name, .. } if name == "finished")
    ));
    assert!(matches!(
        &actions[1],
        Stmt::NativeEventSubscribe {
            event,
            parameters,
            actions: handler,
            ..
        } if event == "progressChanged" && parameters == &["position", "duration"]
            && matches!(&handler[0], Stmt::Assign { name, .. } if name == "finished")
    ));
}

#[test]
fn parses_native_component_event_modifiers() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            state tapped = false
            body {
                Video.VideoView(player: player, controls: true).onTapped {
                    tapped = true
                }
            }
        }"#,
    )
    .expect("valid native component event modifier");

    let Node::NativeComponentCall {
        namespace,
        name,
        event_handlers,
        ..
    } = &app.body[0]
    else {
        panic!("expected a native component call");
    };
    assert_eq!(namespace, "Video");
    assert_eq!(name, "VideoView");
    assert_eq!(event_handlers.len(), 1);
    assert_eq!(event_handlers[0].property, "onTapped");
    assert!(event_handlers[0].parameters.is_empty());
    assert!(matches!(
        &event_handlers[0].actions[0],
        Stmt::Assign { name, .. } if name == "tapped"
    ));
}

#[test]
fn parses_postfix_try_propagation_operator() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            fn fetch_user() -> Result<String, String> {
                let user = fetch_data()?
                return Ok(user)
            }
            body {
                Text("OK")
            }
        }"#,
    )
    .expect("valid result and try expression");

    assert_eq!(app.functions.len(), 1);
    let function = &app.functions[0];
    assert_eq!(function.body.len(), 2);
    assert!(matches!(
        &function.body[0],
        Stmt::Let {
            initial: Expr::Try { .. },
            ..
        }
    ));
}

#[test]
fn parses_arithmetic_precedence_unary_negation_and_compound_assignment() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            state value: Int32 = 0
            body {
                Button("Arithmetic") {
                    value += 2 + 3 * 4 - 5 / 1 % 2
                    value -= 1
                    value *= 2
                    value /= 3
                    value %= 4
                    value = -value
                }
            }
        }"#,
    )
    .expect("arithmetic expressions and compound assignments should parse");

    let Node::ComponentInvocation(button) = &app.body[0] else {
        panic!("expected a button node");
    };
    let ChildBody::Actions(actions) = &button.children else {
        panic!("expected button actions");
    };
    assert_eq!(actions.len(), 6);
    let Stmt::Assign {
        value: Expr::Add(left, right, _),
        ..
    } = &actions[0]
    else {
        panic!("expected `+=` to lower to an addition assignment");
    };
    assert!(matches!(left.as_ref(), Expr::Name(name, _) if name == "value"));
    assert!(matches!(
        right.as_ref(),
        Expr::Arithmetic(
            add,
            ArithmeticOp::Subtract,
            modulo,
            _
        ) if matches!(add.as_ref(), Expr::Add(_, product, _) if matches!(product.as_ref(), Expr::Arithmetic(_, ArithmeticOp::Multiply, _, _)))
            && matches!(modulo.as_ref(), Expr::Arithmetic(divide, ArithmeticOp::Remainder, _, _) if matches!(divide.as_ref(), Expr::Arithmetic(_, ArithmeticOp::Divide, _, _)))
    ));
    assert!(matches!(
        &actions[1],
        Stmt::Assign {
            value: Expr::Arithmetic(_, ArithmeticOp::Subtract, _, _),
            ..
        }
    ));
    assert!(matches!(
        &actions[2],
        Stmt::Assign {
            value: Expr::Arithmetic(_, ArithmeticOp::Multiply, _, _),
            ..
        }
    ));
    assert!(matches!(
        &actions[3],
        Stmt::Assign {
            value: Expr::Arithmetic(_, ArithmeticOp::Divide, _, _),
            ..
        }
    ));
    assert!(matches!(
        &actions[4],
        Stmt::Assign {
            value: Expr::Arithmetic(_, ArithmeticOp::Remainder, _, _),
            ..
        }
    ));
    assert!(matches!(
        &actions[5],
        Stmt::Assign { value: Expr::Negate(value, _), .. }
            if matches!(value.as_ref(), Expr::Name(name, _) if name == "value")
    ));
}

#[test]
fn parses_ranges_as_collection_method_arguments() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            state values: Array<Int32> = [1, 2, 3]
            body {
                Text(values.slice(1..3))
            }
        }"#,
    )
    .expect("slice range arguments should parse");

    let Node::ComponentInvocation(text) = &app.body[0] else {
        panic!("expected a text node");
    };
    let [Expr::MethodCall { arguments, .. }] = text.positional.as_slice() else {
        panic!("expected a collection method call");
    };
    assert!(matches!(
        arguments.as_slice(),
        [Expr::Range {
            start,
            end,
            inclusive: true,
            step: None,
            ..
        }] if matches!(start.as_ref(), Expr::Number(value, _) if value == "1")
            && matches!(end.as_ref(), Expr::Number(value, _) if value == "3")
    ));
}

#[test]
fn parses_if_expressions_as_typed_value_branches() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            body {
                Text(if true { "enabled" } else { "disabled" })
            }
        }"#,
    )
    .expect("value-producing if expressions should parse");

    let Node::ComponentInvocation(text) = &app.body[0] else {
        panic!("expected a text component");
    };
    assert!(matches!(
        text.positional.as_slice(),
        [Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        }] if matches!(condition.as_ref(), Expr::Bool(true, _))
            && matches!(then_value.as_ref(), Expr::String(value, _) if value == "enabled")
            && matches!(else_value.as_ref(), Expr::String(value, _) if value == "disabled")
    ));
}

#[test]
fn parses_ternary_expressions_with_comparison_precedence() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            state count: Int32 = 1
            state enabled: Bool? = null
            body {
                Text(count > 0 ? "positive" : "zero")
                Text(enabled ?? false ? "enabled" : "disabled")
            }
        }"#,
    )
    .expect("ternary value expressions should parse");

    let Node::ComponentInvocation(text) = &app.body[0] else {
        panic!("expected a text component");
    };
    assert!(matches!(
        text.positional.as_slice(),
        [Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        }] if matches!(condition.as_ref(), Expr::Binary(_, BinaryOp::Greater, _, _))
            && matches!(then_value.as_ref(), Expr::String(value, _) if value == "positive")
            && matches!(else_value.as_ref(), Expr::String(value, _) if value == "zero")
    ));

    let Node::ComponentInvocation(text) = &app.body[1] else {
        panic!("expected a second text component");
    };
    assert!(matches!(
        text.positional.as_slice(),
        [Expr::Conditional {
            condition,
            ..
        }] if matches!(condition.as_ref(), Expr::Coalesce(_, _, _))
    ));
}

#[test]
fn parses_component_style_chains_into_typed_options() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            body {
                Text("Hi").fontSize(18).bold().padding(12)
            }
        }"#,
    )
    .expect("component style chains should parse");

    let Node::ComponentInvocation(text) = &app.body[0] else {
        panic!("expected a text component");
    };
    assert_eq!(text.arguments.len(), 3);
    assert!(matches!(
        text.arguments.get("fontSize"),
        Some(Expr::Number(value, _)) if value == "18"
    ));
    assert!(matches!(
        text.arguments.get("fontWeight"),
        Some(Expr::Name(value, _)) if value == "Bold"
    ));
    assert!(matches!(
        text.arguments.get("padding"),
        Some(Expr::Number(value, _)) if value == "12"
    ));
}
