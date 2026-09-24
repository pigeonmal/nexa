use nexa_dev_protocol::{
    ClientMessage, Diagnostic, PROTOCOL_VERSION, ServerMessage, Severity, TargetPlatform,
    decode_client, decode_server, encode_client, encode_server,
};

#[test]
fn client_hello_round_trips_with_the_session_token() {
    let message = ClientMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        session_token: "ephemeral-session-secret".to_owned(),
        target: TargetPlatform::Ios,
    };

    let frame = encode_client(&message).expect("client message should serialize");
    let decoded = decode_client(&frame).expect("client message should deserialize");

    assert_eq!(decoded, message);
}

#[test]
fn server_full_module_round_trips_json_payloads() {
    let typed_module = nexa_ir::Module {
        app_name: "Demo".to_owned(),
        plugins: Vec::new(),
        plugin_assets: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        states: Vec::new(),
        screens: Vec::new(),
        components: Vec::new(),
        body: vec![nexa_ir::Node::Text {
            value: nexa_ir::Expr::String("Hello".to_owned()),
            style: nexa_ir::TextStyle::default(),
        }],
        status_bar: None,
        direction: None,
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
        on_active: None,
        on_inactive: None,
        on_background: None,
    };
    let message = ServerMessage::FullModule {
        revision: "sha256:abc123".to_owned(),
        module: Box::new(nexa_dev_ir::lower(&typed_module, "sha256:abc123")),
    };

    let frame = encode_server(&message).expect("server message should serialize");
    let decoded = decode_server(&frame).expect("server message should deserialize");

    assert_eq!(encode_server(&decoded).expect("re-encode message"), frame);
}

#[test]
fn server_patch_round_trips_path_level_operations() {
    let module = nexa_ir::Module {
        app_name: "Demo".to_owned(),
        plugins: Vec::new(),
        plugin_assets: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        states: Vec::new(),
        screens: Vec::new(),
        components: Vec::new(),
        body: vec![nexa_ir::Node::Text {
            value: nexa_ir::Expr::String("Before".to_owned()),
            style: nexa_ir::TextStyle::default(),
        }],
        status_bar: None,
        direction: None,
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
        on_active: None,
        on_inactive: None,
        on_background: None,
    };
    let previous = nexa_dev_ir::lower(&module, "r1");
    let mut next_module = module;
    next_module.body = vec![nexa_ir::Node::Text {
        value: nexa_ir::Expr::String("After".to_owned()),
        style: nexa_ir::TextStyle::default(),
    }];
    let patch = nexa_dev_ir::diff(&previous, &nexa_dev_ir::lower(&next_module, "r2"))
        .expect("changed module should produce a patch");
    let message = ServerMessage::Patch {
        patch: Box::new(patch),
    };

    let frame = encode_server(&message).expect("patch should serialize");
    let decoded = decode_server(&frame).expect("patch should deserialize");

    assert_eq!(encode_server(&decoded).expect("re-encode patch"), frame);
}

#[test]
fn diagnostics_preserve_file_position_and_severity() {
    let message = ServerMessage::Diagnostics {
        diagnostics: vec![Diagnostic {
            severity: Severity::Error,
            file: "App.nx".to_owned(),
            line: 12,
            column: 7,
            message: "unknown state `count`".to_owned(),
        }],
    };

    let frame = encode_server(&message).expect("diagnostics should serialize");
    let decoded = decode_server(&frame).expect("diagnostics should deserialize");

    assert_eq!(encode_server(&decoded).expect("re-encode message"), frame);
}

#[test]
fn performance_overlay_toggle_round_trips() {
    let message = ServerMessage::PerformanceOverlay { enabled: true };
    let frame = encode_server(&message).expect("performance overlay should serialize");
    let decoded = decode_server(&frame).expect("performance overlay should deserialize");
    assert_eq!(
        encode_server(&decoded).expect("re-encode performance overlay"),
        frame
    );
}

#[test]
fn unknown_or_malformed_messages_are_rejected() {
    assert!(decode_client(r#"{"type":"patch","payload":{}}"#).is_err());
    assert!(decode_server(r#"{"type":"full_module","payload":{"revision":1}}"#).is_err());
}
