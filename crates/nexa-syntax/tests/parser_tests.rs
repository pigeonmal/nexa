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
fn parses_top_level_async_function_declaration() {
    let program = nexa_syntax::parse_program(
        r#"async fn readReceipt() -> Int32 { return 1 }
        app Demo { body { Text("ready") } }"#,
    )
    .expect("top-level async functions should parse");

    assert_eq!(program.functions.len(), 1);
    assert!(program.functions[0].is_async);
}


#[test]
fn database_declarations_are_not_part_of_core_language_syntax() {
    let error = nexa_syntax::parse(
        r#"database TodoDatabase("todo") { migration 1 { "CREATE TABLE tasks(id INTEGER)" } }
        app Todo { body { Text("ready") } }"#,
    )
    .expect_err("database-specific declarations belong to plugins, not core grammar");
    assert!(error.message.contains("expected an `import`, `plugin`"));
}

#[test]
fn ordinary_strings_keep_escape_processing_with_raw_strings_available() {
    let app = nexa_syntax::parse(r#"app Demo { body { Text("line\\nnext") } }"#)
        .expect("ordinary and raw string forms should parse");

    let nexa_syntax::ast::Node::ComponentInvocation(invocation) = &app.body[0] else {
        panic!("expected Text invocation");
    };
    let nexa_syntax::ast::Expr::String(text, _) = &invocation.positional[0] else {
        panic!("expected ordinary string argument");
    };
    assert_eq!(text, "line\\nnext");
}

#[test]
fn parses_app_scoped_periodic_background_task() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            background task Refresh(identifier: "dev.example.refresh", everyMinutes: 60) {
                Storage.setString(key: "last-run", value: "ok")
            }
            body { Text("ready") }
        }"#,
    )
    .expect("background task declaration should parse");

    assert_eq!(app.background_tasks.len(), 1);
    assert_eq!(app.background_tasks[0].name, "Refresh");
    assert_eq!(app.background_tasks[0].identifier, "dev.example.refresh");
    assert_eq!(app.background_tasks[0].interval_minutes, 60);
    assert_eq!(app.background_tasks[0].body.len(), 1);
}

#[test]
fn rejects_duplicate_background_task_identifiers() {
    let error = nexa_syntax::parse(
        r#"app Demo {
            background task First(identifier: "dev.example.refresh", everyMinutes: 60) {}
            background task Second(identifier: "dev.example.refresh", everyMinutes: 90) {}
            body { Text("ready") }
        }"#,
    )
    .expect_err("two task declarations cannot register one operating-system identifier");

    assert!(error.to_string().contains("declared more than once"));
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
fn parses_returns_inside_if_and_catch_all_function_blocks() {
    let program = nexa_syntax::parse_program(
        r#"
        async fn readValue() -> Bool {
            if true {
                return true
            } else {
                try {
                    await Database.read()
                    return true
                } catch {
                    else { return false }
                }
            }
        }
        app Demo { body { Text("ready") } }
        "#,
    )
    .expect("function returns should be accepted inside nested recovery blocks");

    assert_eq!(program.functions.len(), 1);
    assert!(matches!(
        program.functions[0].body.as_slice(),
        [Stmt::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        }] if matches!(then_branch.as_slice(), [Stmt::Return { .. }])
            && matches!(else_branch.as_slice(), [Stmt::TryCatch {
                catch_body: Some(catch_body), ..
            }] if matches!(catch_body.as_slice(), [Stmt::Return { .. }]))
    ));
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
fn parses_generic_qualified_service_calls_in_action_blocks() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            body {
                Button("Store") {
                    Storage.write<Array<String?>>(values)
                }
            }
        }"#,
    )
    .expect("generic qualified service calls should parse");
    let Node::ComponentInvocation(invocation) = &app.body[0] else {
        panic!("expected a button node");
    };
    let ChildBody::Actions(actions) = &invocation.children else {
        panic!("expected button actions");
    };
    assert!(
        matches!(
            &actions[0],
            Stmt::Expression {
                expression: Expr::QualifiedCall {
                    namespace,
                    name,
                    type_arguments,
                    ..
                },
                ..
            } if namespace == "Storage"
                && name == "write"
                && type_arguments.len() == 1
        ),
        "unexpected generic qualified call AST: {:#?}",
        actions[0]
    );
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
fn parses_closed_transition_modifiers_on_if_and_when_blocks() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            state visible = false
            body {
                if visible { Text("shown") }.transition(.fade)
                when visible {
                    true: { Text("yes") }
                    else: { Text("no") }
                }.transition(.slide(from: .bottom))
            }
        }"#,
    )
    .expect("conditional transition syntax should parse");

    assert!(matches!(
        &app.body[0],
        Node::If {
            transition: Some(nexa_syntax::ast::ViewTransition::Fade),
            ..
        }
    ));
    assert!(matches!(
        &app.body[1],
        Node::When {
            transition: Some(nexa_syntax::ast::ViewTransition::SlideFromBottom),
            ..
        }
    ));
}

#[test]
fn conditional_transitions_reject_unknown_edges() {
    let error = nexa_syntax::parse(
        r#"app Demo {
            state visible = false
            body { if visible { Text("shown") }.transition(.slide(from: .top)) }
        }"#,
    )
    .expect_err("unsupported transition edges should be rejected");

    assert!(
        error
            .message
            .contains("support `.bottom`, `.left`, and `.right`")
    );
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

#[test]
fn parses_visual_modifier_chains_as_typed_style_arguments() {
    let app = nexa_syntax::parse(
        r##"app Demo {
            body {
                Column { Text("Card") }
                    .opacity(0.8)
                    .scale(1.1)
                    .rotation(-6)
                    .shadow(radius: 4, x: 1, y: -2, color: "#00000080")
                    .blur(2)
                    .clip(shape: Rounded(12))
                    .zIndex(-3)
            }
        }"##,
    )
    .expect("visual modifier chains should parse");

    let Node::ComponentInvocation(layout) = &app.body[0] else {
        panic!("expected a layout component");
    };
    assert_eq!(layout.arguments.len(), 7);
    assert!(
        matches!(layout.arguments.get("opacity"), Some(Expr::Number(value, _)) if value == "0.8")
    );
    assert!(
        matches!(layout.arguments.get("scale"), Some(Expr::Number(value, _)) if value == "1.1")
    );
    assert!(
        matches!(layout.arguments.get("rotation"), Some(Expr::Number(value, _)) if value == "-6")
    );
    assert!(
        matches!(layout.arguments.get("shadow"), Some(Expr::Call(name, _, arguments, _)) if name == "Shadow" && arguments.len() == 4)
    );
    assert!(matches!(layout.arguments.get("blur"), Some(Expr::Number(value, _)) if value == "2"));
    assert!(
        matches!(layout.arguments.get("clip"), Some(Expr::Call(name, _, arguments, _)) if name == "Rounded" && arguments.len() == 1)
    );
    assert!(
        matches!(layout.arguments.get("zIndex"), Some(Expr::Number(value, _)) if value == "-3")
    );
}

#[test]
fn parses_named_spring_response_and_damping_options() {
    let app = nexa_syntax::parse(
        r#"app Demo {
            body { Column(animation: Spring(response: 0.35, damping: 0.8)) { Text("animated") } }
        }"#,
    )
    .expect("named spring parameters should parse");
    let nexa_syntax::ast::Node::ComponentInvocation(column) = &app.body[0] else {
        panic!("expected Column invocation");
    };
    let Some(nexa_syntax::ast::Expr::Call(name, type_arguments, arguments, _)) =
        column.arguments.get("animation")
    else {
        panic!("expected Spring animation expression");
    };
    assert_eq!(name, "Spring");
    assert!(type_arguments.is_empty());
    assert!(matches!(&arguments[0], nexa_syntax::ast::Expr::Number(raw, _) if raw == "0.35"));
    assert!(matches!(&arguments[1], nexa_syntax::ast::Expr::Number(raw, _) if raw == "0.8"));
}
