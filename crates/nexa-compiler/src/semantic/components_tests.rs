use std::collections::{BTreeMap, HashMap};

use nexa_diagnostics::Span;
use nexa_ir::{Action, Expr, NativeComponentEventHandler, NumericType, Type};
use nexa_syntax::ast;

use super::{FunctionSignatures, lower_actions_with_aliases, lower_actions_with_depth, lower_node};
use crate::Target;
use crate::semantic::custom_components::{
    ComponentEventSignature, ComponentSignature, ComponentSignatures,
};
use crate::semantic::expressions::{
    FunctionSignature, PluginErrorType, PluginErrorVariant, record_native_alias,
};
use crate::semantic::themes::ThemeSymbols;

fn fixture(mutable: bool) -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
    let player_type = Type::Plugin {
        namespace: "Video".to_owned(),
        name: "VideoPlayer".to_owned(),
    };
    let symbols = HashMap::from([("player".to_owned(), (player_type.clone(), false))]);
    let signature = FunctionSignature {
        parameters: Vec::new(),
        return_type: Type::Numeric(NumericType::Float64),
        is_async: false,
        is_throwing: false,
        receiver: Some(player_type.clone()),
        is_constructor: false,
        is_mutable_property: mutable,
        error_handling_allowed: false,
        error_type: None,
    };
    let functions = HashMap::from([("VideoPlayer.#property.volume".to_owned(), signature)]);
    (symbols, functions)
}

fn assignment() -> ast::Stmt {
    ast::Stmt::NativePropertyAssign {
        receiver: ast::Expr::Name("player".to_owned(), Span::default()),
        property: "volume".to_owned(),
        value: ast::Expr::Number("0.5".to_owned(), Span::default()),
        span: Span::default(),
    }
}

fn assignment_with_value(value: ast::Expr) -> ast::Stmt {
    ast::Stmt::NativePropertyAssign {
        receiver: ast::Expr::Name("player".to_owned(), Span::default()),
        property: "volume".to_owned(),
        value,
        span: Span::default(),
    }
}

fn event_fixture() -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
    let player_type = Type::Plugin {
        namespace: "Video".to_owned(),
        name: "VideoPlayer".to_owned(),
    };
    let symbols = HashMap::from([("player".to_owned(), (player_type.clone(), false))]);
    let functions = HashMap::from([(
        "VideoPlayer.#event.progressChanged".to_owned(),
        FunctionSignature {
            parameters: vec![
                ("position".to_owned(), Type::Numeric(NumericType::Float64)),
                ("duration".to_owned(), Type::Numeric(NumericType::Float64)),
            ],
            return_type: Type::Void,
            is_async: false,
            is_throwing: false,
            receiver: Some(player_type),
            is_constructor: false,
            is_mutable_property: false,
            error_handling_allowed: false,
            error_type: None,
        },
    )]);
    (symbols, functions)
}

fn progress_event(parameters: Vec<String>) -> ast::Stmt {
    ast::Stmt::NativeEventSubscribe {
        receiver: ast::Expr::Name("player".to_owned(), Span::default()),
        event: "progressChanged".to_owned(),
        parameters,
        actions: vec![ast::Stmt::Expression {
            expression: ast::Expr::Name("position".to_owned(), Span::default()),
            span: Span::default(),
        }],
        span: Span::default(),
    }
}

fn native_component_event_node(property: &str, parameters: Vec<String>) -> ast::Node {
    ast::Node::NativeComponentCall {
        namespace: "Video".to_owned(),
        name: "VideoView".to_owned(),
        arguments: BTreeMap::new(),
        children: None,
        event_handlers: vec![ast::NativeComponentEventHandler {
            property: property.to_owned(),
            parameters,
            actions: vec![ast::Stmt::Expression {
                expression: ast::Expr::Name("position".to_owned(), Span::default()),
                span: Span::default(),
            }],
            span: Span::default(),
        }],
        span: Span::default(),
    }
}

fn native_component_signatures() -> ComponentSignatures {
    ComponentSignatures::from([(
        "Video.VideoView".to_owned(),
        ComponentSignature {
            parameters: Vec::new(),
            defaults: HashMap::new(),
            events: vec![ComponentEventSignature {
                property: "onProgressChanged".to_owned(),
                parameters: vec![
                    ("position".to_owned(), Type::Numeric(NumericType::Float64)),
                    ("duration".to_owned(), Type::Numeric(NumericType::Float64)),
                ],
            }],
            has_content_slot: false,
            native: true,
        },
    )])
}

fn lower_native_component_event(
    property: &str,
    parameters: Vec<String>,
) -> Result<nexa_ir::Node, nexa_diagnostics::CompileError> {
    lower_node(
        native_component_event_node(property, parameters),
        &HashMap::new(),
        &HashMap::new(),
        &ThemeSymbols::default(),
        &native_component_signatures(),
        &HashMap::new(),
        &HashMap::new(),
        false,
        false,
        Target::Swift,
    )
}

fn disposal_fixture() -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
    let player_type = Type::Plugin {
        namespace: "Video".to_owned(),
        name: "VideoPlayer".to_owned(),
    };
    let symbols = HashMap::from([("player".to_owned(), (player_type.clone(), false))]);
    let method = |return_type| FunctionSignature {
        parameters: Vec::new(),
        return_type,
        is_async: false,
        is_throwing: false,
        receiver: Some(player_type.clone()),
        is_constructor: false,
        is_mutable_property: false,
        error_handling_allowed: false,
        error_type: None,
    };
    let functions = HashMap::from([
        ("VideoPlayer.dispose".to_owned(), method(Type::Void)),
        ("VideoPlayer.play".to_owned(), method(Type::Void)),
    ]);
    (symbols, functions)
}

fn native_method_call(method: &str) -> ast::Stmt {
    native_method_call_on("player", method)
}

fn native_method_call_on(name: &str, method: &str) -> ast::Stmt {
    ast::Stmt::CollectionMutation {
        name: name.to_owned(),
        method: method.to_owned(),
        arguments: Vec::new(),
        span: Span::default(),
    }
}

fn throwing_prepare() -> ast::Expr {
    ast::Expr::Await(
        Box::new(ast::Expr::MethodCall {
            base: Box::new(ast::Expr::Name("player".to_owned(), Span::default())),
            name: "prepare".to_owned(),
            arguments: Vec::new(),
            named_arguments: BTreeMap::new(),
            span: Span::default(),
        }),
        Span::default(),
    )
}

fn throwing_file_read() -> ast::Expr {
    let span = Span::default();
    ast::Expr::Await(
        Box::new(ast::Expr::QualifiedCall {
            namespace: "File".to_owned(),
            name: "readText".to_owned(),
            arguments: Vec::new(),
            named_arguments: BTreeMap::from([(
                "path".to_owned(),
                ast::Expr::String("notes.txt".to_owned(), span),
            )]),
            span,
        }),
        span,
    )
}

fn throwing_fixture() -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
    let player_type = Type::Plugin {
        namespace: "Video".to_owned(),
        name: "VideoPlayer".to_owned(),
    };
    let error_type = PluginErrorType {
        namespace: "Video".to_owned(),
        name: "PlayerError".to_owned(),
        variants: vec![
            PluginErrorVariant {
                name: "invalidUrl".to_owned(),
                parameters: Vec::new(),
            },
            PluginErrorVariant {
                name: "decodingFailed".to_owned(),
                parameters: vec![("message".to_owned(), Type::String)],
            },
        ],
    };
    let signature = |receiver| FunctionSignature {
        parameters: Vec::new(),
        return_type: Type::Void,
        is_async: true,
        is_throwing: true,
        receiver,
        is_constructor: false,
        is_mutable_property: false,
        error_handling_allowed: false,
        error_type: Some(error_type.clone()),
    };
    (
        HashMap::from([
            ("player".to_owned(), (player_type.clone(), false)),
            ("failed".to_owned(), (Type::Bool, true)),
        ]),
        HashMap::from([(
            "VideoPlayer.prepare".to_owned(),
            signature(Some(player_type)),
        )]),
    )
}

#[test]
fn lowers_mutable_native_property_assignment() {
    let (symbols, functions) = fixture(true);
    let actions = lower_actions_with_depth(vec![assignment()], &symbols, &functions, false, 0)
        .expect("mutable plugin property should be assignable");

    assert!(matches!(
        &actions[0],
        Action::NativePropertyAssign {
            receiver: Expr::State(name, Type::Plugin { name: class, .. }),
            property,
            value: Expr::Number { raw, ty: NumericType::Float64 },
        } if name == "player" && class == "VideoPlayer" && property == "volume" && raw == "0.5"
    ));
}

#[test]
fn rejects_assignment_to_readonly_native_property() {
    let (symbols, functions) = fixture(false);
    let error = lower_actions_with_depth(vec![assignment()], &symbols, &functions, false, 0)
        .expect_err("readonly plugin property must not be assignable");
    assert!(error.to_string().contains("read-only"));
}

#[test]
fn rejects_native_property_assignment_with_the_wrong_type() {
    let (symbols, functions) = fixture(true);
    let statement = assignment_with_value(ast::Expr::Bool(true, Span::default()));
    let error = lower_actions_with_depth(vec![statement], &symbols, &functions, false, 0)
        .expect_err("a Bool must not be assigned to a Float64 property");
    assert!(error.to_string().contains("Bool"));
    assert!(error.to_string().contains("Float64"));
}

#[test]
fn lowers_instance_event_with_typed_payload_bindings() {
    let (symbols, functions) = event_fixture();
    let actions = lower_actions_with_depth(
        vec![progress_event(vec![
            "position".to_owned(),
            "duration".to_owned(),
        ])],
        &symbols,
        &functions,
        false,
        0,
    )
    .expect("declared native event should lower");

    assert!(matches!(
        &actions[0],
        Action::NativeEventSubscribe {
            receiver: Expr::State(receiver, Type::Plugin { name: class, .. }),
            property,
            parameters,
            actions: handler,
        } if receiver == "player" && class == "VideoPlayer"
            && property == "onProgressChanged"
            && parameters == &["position", "duration"]
            && matches!(
                &handler[0],
                Action::Expression(Expr::State(name, Type::Numeric(NumericType::Float64)))
                    if name == "position"
            )
    ));
}

#[test]
fn rejects_event_handlers_with_the_wrong_payload_arity() {
    let (symbols, functions) = event_fixture();
    let error = lower_actions_with_depth(
        vec![progress_event(vec!["position".to_owned()])],
        &symbols,
        &functions,
        false,
        0,
    )
    .expect_err("a two-value event must not accept one handler binding");
    assert!(error.to_string().contains("provides 2 value(s)"));
}

#[test]
fn rejects_event_payload_bindings_that_shadow_existing_values() {
    let (mut symbols, functions) = event_fixture();
    symbols.insert("position".to_owned(), (Type::String, false));
    let error = lower_actions_with_depth(
        vec![progress_event(vec![
            "position".to_owned(),
            "duration".to_owned(),
        ])],
        &symbols,
        &functions,
        false,
        0,
    )
    .expect_err("event bindings must not capture an existing value ambiguously");
    assert!(error.to_string().contains("shadows an existing value"));
}

#[test]
fn lowers_native_component_event_with_typed_payload_bindings() {
    let node = lower_native_component_event(
        "onProgressChanged",
        vec!["position".to_owned(), "duration".to_owned()],
    )
    .expect("declared native component event should lower");

    assert!(matches!(
        node,
        nexa_ir::Node::NativeComponentCall { event_handlers, .. }
            if matches!(
                event_handlers.as_slice(),
                [NativeComponentEventHandler {
                    property,
                    parameters,
                    actions,
                }] if property == "onProgressChanged"
                    && parameters == &["position", "duration"]
                    && matches!(
                        actions.as_slice(),
                        [Action::Expression(Expr::State(name, Type::Numeric(NumericType::Float64)))]
                            if name == "position"
                    )
            )
    ));
}

#[test]
fn rejects_native_component_event_with_unknown_callback() {
    let error = lower_native_component_event("onMissing", Vec::new())
        .expect_err("unknown native component events must not be ignored");
    assert!(
        error
            .to_string()
            .contains("has no event callback `onMissing`")
    );
}

#[test]
fn rejects_native_component_event_with_wrong_payload_arity() {
    let error = lower_native_component_event("onProgressChanged", vec!["position".to_owned()])
        .expect_err("component event handler must bind every payload value");
    assert!(error.to_string().contains("provides 2 value(s)"));
}

#[test]
fn throwing_native_calls_need_and_use_an_explicit_recovery_block() {
    let (symbols, functions) = throwing_fixture();
    let actions = lower_actions_with_depth(
        vec![ast::Stmt::TryCatch {
            body: vec![ast::Stmt::Expression {
                expression: throwing_prepare(),
                span: Span::default(),
            }],
            error_catches: Vec::new(),
            catch_body: Some(vec![ast::Stmt::Assign {
                name: "failed".to_owned(),
                value: ast::Expr::Bool(true, Span::default()),
                span: Span::default(),
            }]),
            span: Span::default(),
        }],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect("a try block should lower a throwing native call");

    assert!(matches!(
        actions.as_slice(),
        [Action::TryCatch {
            body,
            error_catches,
            catch_body,
        }] if matches!(
            body.as_slice(),
            [Action::Expression(Expr::TryAwait(call))]
                if matches!(call.as_ref(), Expr::NativeCall { name, is_throwing: true, .. } if name == "prepare")
        ) && error_catches.is_empty()
            && matches!(catch_body.as_deref(), Some([Action::Assign { name, .. }]) if name == "failed")
    ));

    let unhandled = lower_actions_with_depth(
        vec![ast::Stmt::Expression {
            expression: throwing_prepare(),
            span: Span::default(),
        }],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect_err("a throwing call outside try/catch must remain a semantic error");
    assert!(unhandled.to_string().contains("may throw"));
}

#[test]
fn typed_catch_cases_bind_declared_error_payloads_with_native_types() {
    let (mut symbols, functions) = throwing_fixture();
    symbols.insert("messageOut".to_owned(), (Type::String, true));
    let actions = lower_actions_with_depth(
        vec![ast::Stmt::TryCatch {
            body: vec![ast::Stmt::Expression {
                expression: throwing_prepare(),
                span: Span::default(),
            }],
            error_catches: vec![
                ast::ErrorCatchArm {
                    namespace: "Video".to_owned(),
                    error_name: "PlayerError".to_owned(),
                    variant: "invalidUrl".to_owned(),
                    bindings: Vec::new(),
                    body: vec![ast::Stmt::Assign {
                        name: "failed".to_owned(),
                        value: ast::Expr::Bool(true, Span::default()),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                },
                ast::ErrorCatchArm {
                    namespace: "Video".to_owned(),
                    error_name: "PlayerError".to_owned(),
                    variant: "decodingFailed".to_owned(),
                    bindings: vec!["message".to_owned()],
                    body: vec![ast::Stmt::Assign {
                        name: "messageOut".to_owned(),
                        value: ast::Expr::Name("message".to_owned(), Span::default()),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                },
            ],
            catch_body: None,
            span: Span::default(),
        }],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect("declared plugin error cases and payloads should lower");

    assert!(matches!(
        actions.as_slice(),
        [Action::TryCatch {
            error_catches,
            catch_body: None,
            ..
        }] if matches!(error_catches.as_slice(), [
            nexa_ir::ErrorCatchArm { variant, parameters, .. },
            nexa_ir::ErrorCatchArm { variant: payload_variant, parameters: payload_parameters, body, .. }
        ] if variant == "invalidUrl"
            && parameters.is_empty()
            && payload_variant == "decodingFailed"
            && payload_parameters == &[("message".to_owned(), "message".to_owned(), Type::String)]
            && matches!(body.as_slice(), [Action::Assign {
                name,
                value: Expr::State(payload, Type::String),
            }] if name == "messageOut" && payload == "message"))
    ));
}

#[test]
fn typed_catch_cases_must_be_exhaustive_without_an_else_branch() {
    let (symbols, functions) = throwing_fixture();
    let error = lower_actions_with_depth(
        vec![ast::Stmt::TryCatch {
            body: vec![ast::Stmt::Expression {
                expression: throwing_prepare(),
                span: Span::default(),
            }],
            error_catches: vec![ast::ErrorCatchArm {
                namespace: "Video".to_owned(),
                error_name: "PlayerError".to_owned(),
                variant: "invalidUrl".to_owned(),
                bindings: Vec::new(),
                body: Vec::new(),
                span: Span::default(),
            }],
            catch_body: None,
            span: Span::default(),
        }],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect_err("unmatched typed error variants need an explicit fallback");

    assert!(
        error
            .to_string()
            .contains("catch every variant of `Video.PlayerError`")
    );
}

#[test]
fn typed_catch_cases_require_else_for_untyped_failures() {
    let (symbols, functions) = throwing_fixture();
    let error = lower_actions_with_depth(
        vec![ast::Stmt::TryCatch {
            body: vec![
                ast::Stmt::Expression {
                    expression: throwing_prepare(),
                    span: Span::default(),
                },
                ast::Stmt::Expression {
                    expression: throwing_file_read(),
                    span: Span::default(),
                },
            ],
            error_catches: vec![
                ast::ErrorCatchArm {
                    namespace: "Video".to_owned(),
                    error_name: "PlayerError".to_owned(),
                    variant: "invalidUrl".to_owned(),
                    bindings: Vec::new(),
                    body: Vec::new(),
                    span: Span::default(),
                },
                ast::ErrorCatchArm {
                    namespace: "Video".to_owned(),
                    error_name: "PlayerError".to_owned(),
                    variant: "decodingFailed".to_owned(),
                    bindings: vec!["message".to_owned()],
                    body: Vec::new(),
                    span: Span::default(),
                },
            ],
            catch_body: None,
            span: Span::default(),
        }],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect_err("typed cases cannot handle a built-in untyped error");

    assert!(
        error
            .to_string()
            .contains("catch-all `else` block is required")
    );
}

#[test]
fn typed_catch_cases_reject_unknown_variants_and_wrong_payload_arity() {
    let (symbols, functions) = throwing_fixture();
    let catch = |variant: &str, bindings: Vec<String>| ast::Stmt::TryCatch {
        body: vec![ast::Stmt::Expression {
            expression: throwing_prepare(),
            span: Span::default(),
        }],
        error_catches: vec![ast::ErrorCatchArm {
            namespace: "Video".to_owned(),
            error_name: "PlayerError".to_owned(),
            variant: variant.to_owned(),
            bindings,
            body: Vec::new(),
            span: Span::default(),
        }],
        catch_body: None,
        span: Span::default(),
    };

    let unknown = lower_actions_with_depth(
        vec![catch("notAPlayerError", Vec::new())],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect_err("error patterns must reference a declared error variant");
    assert!(unknown.to_string().contains("unknown plugin error variant"));

    let missing_payload = lower_actions_with_depth(
        vec![catch("decodingFailed", Vec::new())],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect_err("payload variants must bind their declared values");
    assert!(
        missing_payload
            .to_string()
            .contains("provides 1 payload value(s)")
    );
}

#[test]
fn throwing_file_calls_preserve_failures_only_inside_recovery_blocks() {
    let symbols = HashMap::new();
    let functions = FunctionSignatures::new();
    let actions = lower_actions_with_depth(
        vec![ast::Stmt::TryCatch {
            body: vec![ast::Stmt::Expression {
                expression: throwing_file_read(),
                span: Span::default(),
            }],
            error_catches: Vec::new(),
            catch_body: Some(Vec::new()),
            span: Span::default(),
        }],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect("File.readText should preserve its native error inside try/catch");

    assert!(matches!(
        actions.as_slice(),
        [Action::TryCatch { body, .. }]
            if matches!(
                body.as_slice(),
                [Action::Expression(Expr::TryAwait(call))]
                    if matches!(call.as_ref(), Expr::FileReadText { .. })
            )
    ));

    let unhandled = lower_actions_with_depth(
        vec![ast::Stmt::Expression {
            expression: throwing_file_read(),
            span: Span::default(),
        }],
        &symbols,
        &functions,
        true,
        0,
    )
    .expect_err("File.readText must not silently discard native failures");
    assert!(unhandled.to_string().contains("may throw"));
}

#[test]
fn try_catch_does_not_handle_errors_from_later_native_callbacks() {
    let (symbols, mut functions) = throwing_fixture();
    let player_type = symbols["player"].0.clone();
    functions.insert(
        "VideoPlayer.#event.ended".to_owned(),
        FunctionSignature {
            parameters: Vec::new(),
            return_type: Type::Void,
            is_async: false,
            is_throwing: false,
            receiver: Some(player_type),
            is_constructor: false,
            is_mutable_property: false,
            error_handling_allowed: false,
            error_type: None,
        },
    );
    let actions = vec![ast::Stmt::TryCatch {
        body: vec![ast::Stmt::NativeEventSubscribe {
            receiver: ast::Expr::Name("player".to_owned(), Span::default()),
            event: "ended".to_owned(),
            parameters: Vec::new(),
            actions: vec![ast::Stmt::Expression {
                expression: throwing_prepare(),
                span: Span::default(),
            }],
            span: Span::default(),
        }],
        error_catches: Vec::new(),
        catch_body: Some(Vec::new()),
        span: Span::default(),
    }];
    let error = lower_actions_with_depth(actions, &symbols, &functions, true, 0)
        .expect_err("an outer try block cannot catch a later callback failure");
    assert!(
        error
            .to_string()
            .contains("`await` is only allowed in an async function")
    );
}

#[test]
fn native_component_content_blocks_must_match_the_declared_slot() {
    let mut signatures = native_component_signatures();
    signatures
        .get_mut("Video.VideoView")
        .expect("component fixture should exist")
        .has_content_slot = true;
    let error = lower_node(
        native_component_event_node(
            "onProgressChanged",
            vec!["position".to_owned(), "duration".to_owned()],
        ),
        &HashMap::new(),
        &HashMap::new(),
        &ThemeSymbols::default(),
        &signatures,
        &HashMap::new(),
        &HashMap::new(),
        false,
        false,
        Target::Swift,
    )
    .expect_err("a declared content slot requires a child block");
    assert!(error.to_string().contains("requires a child block"));

    let mut node = native_component_event_node(
        "onProgressChanged",
        vec!["position".to_owned(), "duration".to_owned()],
    );
    if let ast::Node::NativeComponentCall { children, .. } = &mut node {
        *children = Some(Vec::new());
    }
    let error = lower_node(
        node,
        &HashMap::new(),
        &HashMap::new(),
        &ThemeSymbols::default(),
        &native_component_signatures(),
        &HashMap::new(),
        &HashMap::new(),
        false,
        false,
        Target::Swift,
    )
    .expect_err("a child block without a declared slot must be rejected");
    assert!(
        error
            .to_string()
            .contains("does not declare a content slot")
    );
}

#[test]
fn rejects_native_instance_use_after_disposal() {
    let (symbols, functions) = disposal_fixture();
    let error = lower_actions_with_depth(
        vec![native_method_call("dispose"), native_method_call("play")],
        &symbols,
        &functions,
        false,
        0,
    )
    .expect_err("native methods must not be called after dispose");

    assert!(error.to_string().contains("used after disposal"));
}

#[test]
fn disposal_analysis_tracks_aliases_to_the_same_native_instance() {
    let (mut symbols, functions) = disposal_fixture();
    let player_type = symbols["player"].0.clone();
    symbols.insert("playerAlias".to_owned(), (player_type, false));
    let mut aliases = HashMap::new();
    let player = Expr::State("player".to_owned(), symbols["player"].0.clone());
    record_native_alias("playerAlias", &symbols["player"].0, &player, &mut aliases);
    let alias = Expr::State("playerAlias".to_owned(), symbols["playerAlias"].0.clone());
    record_native_alias(
        "playerAlias2",
        &symbols["playerAlias"].0,
        &alias,
        &mut aliases,
    );
    symbols.insert(
        "playerAlias2".to_owned(),
        (symbols["player"].0.clone(), false),
    );
    assert_eq!(
        aliases.get("playerAlias2").map(String::as_str),
        Some("player")
    );

    let use_after_dispose = lower_actions_with_aliases(
        vec![
            native_method_call("dispose"),
            native_method_call_on("playerAlias", "play"),
        ],
        &symbols,
        &functions,
        false,
        &aliases,
    )
    .expect_err("disposing an object must invalidate every binding alias");
    assert!(
        use_after_dispose
            .to_string()
            .contains("native class instance `player` may be used after disposal")
    );

    let double_dispose = lower_actions_with_aliases(
        vec![
            native_method_call("dispose"),
            native_method_call_on("playerAlias2", "dispose"),
        ],
        &symbols,
        &functions,
        false,
        &aliases,
    )
    .expect_err("disposing through an alias must count as a second disposal");
    assert!(
        double_dispose
            .to_string()
            .contains("native class instance `player` may be disposed more than once")
    );
}

#[test]
fn native_binding_with_alias_cannot_be_replaced_after_disposal() {
    let (mut symbols, mut functions) = disposal_fixture();
    symbols.get_mut("player").expect("fixture state").1 = true;
    let player_type = symbols["player"].0.clone();
    symbols.insert("playerAlias".to_owned(), (player_type.clone(), false));
    let mut aliases = HashMap::new();
    let player = Expr::State("player".to_owned(), player_type.clone());
    record_native_alias("playerAlias", &player_type, &player, &mut aliases);
    functions.insert(
        "Video.VideoPlayer".to_owned(),
        FunctionSignature {
            parameters: Vec::new(),
            return_type: player_type,
            is_async: false,
            is_throwing: false,
            receiver: None,
            is_constructor: true,
            is_mutable_property: false,
            error_handling_allowed: false,
            error_type: None,
        },
    );
    let fresh_player = ast::Expr::QualifiedCall {
        namespace: "Video".to_owned(),
        name: "VideoPlayer".to_owned(),
        arguments: Vec::new(),
        named_arguments: BTreeMap::new(),
        span: Span::default(),
    };
    let reset = ast::Stmt::Assign {
        name: "player".to_owned(),
        value: fresh_player,
        span: Span::default(),
    };
    let error = lower_actions_with_aliases(
        vec![native_method_call("dispose"), reset],
        &symbols,
        &functions,
        false,
        &aliases,
    )
    .expect_err("replacing the source binding would leave its alias stale");
    assert!(error.to_string().contains("cannot be replaced while alias"));
}

#[test]
fn rejects_disposing_the_same_native_instance_twice() {
    let (symbols, functions) = disposal_fixture();
    let error = lower_actions_with_depth(
        vec![native_method_call("dispose"), native_method_call("dispose")],
        &symbols,
        &functions,
        false,
        0,
    )
    .expect_err("native instances must not be disposed twice");

    assert!(error.to_string().contains("disposed more than once"));
}

#[test]
fn propagates_possible_disposal_from_conditional_branches() {
    let (symbols, functions) = disposal_fixture();
    let conditional_dispose = ast::Stmt::If {
        condition: ast::Expr::Bool(true, Span::default()),
        then_branch: vec![native_method_call("dispose")],
        else_branch: None,
        span: Span::default(),
    };
    let error = lower_actions_with_depth(
        vec![conditional_dispose, native_method_call("play")],
        &symbols,
        &functions,
        false,
        0,
    )
    .expect_err("use after a conditional dispose must be rejected");

    assert!(error.to_string().contains("used after disposal"));
}
