use std::{
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};

use nexa_dev_ir::{DevModule, lower};
use nexa_dev_protocol::{
    ClientMessage, Diagnostic, PROTOCOL_VERSION, ServerMessage, Severity, TargetPlatform,
    decode_server, encode_client,
};
use nexa_dev_server::DevServer;
use nexa_ir::Module;
use tungstenite::{ClientRequestBuilder, Message, WebSocket, client};

fn module(revision: &str, label: &str) -> DevModule {
    let module = Module {
        widgets: Vec::new(),
        app_name: "Demo".to_owned(),
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
        body: vec![nexa_ir::Node::Text {
            value: nexa_ir::Expr::String(label.to_owned()),
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
    lower(&module, revision)
}

fn connect(server: &DevServer, token: &str, target: TargetPlatform) -> WebSocket<TcpStream> {
    let stream = TcpStream::connect(server.address()).expect("connect to loopback dev server");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set test client timeout");
    let uri = format!("ws://{}/", server.address())
        .parse()
        .expect("parse websocket uri");
    let request = ClientRequestBuilder::new(uri);
    let (mut websocket, _) = client(request, stream).expect("complete websocket handshake");
    let hello = ClientMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        session_token: token.to_owned(),
        target,
    };
    websocket
        .send(Message::Text(
            encode_client(&hello).expect("encode hello").into(),
        ))
        .expect("send hello");
    websocket
}

fn read_server_message(websocket: &mut WebSocket<TcpStream>) -> ServerMessage {
    match websocket.read().expect("read server frame") {
        Message::Text(text) => decode_server(text.as_str()).expect("decode server frame"),
        message => panic!("unexpected server frame: {message:?}"),
    }
}

#[test]
fn server_binds_only_loopback_and_authenticates_with_ephemeral_token() {
    let server = DevServer::bind([
        (TargetPlatform::Ios, module("ios-1", "first ios")),
        (
            TargetPlatform::Android,
            module("android-1", "first android"),
        ),
    ])
    .expect("bind dev server");
    assert!(server.address().ip().is_loopback());
    assert_eq!(server.session_token().len(), 32);

    for target in [TargetPlatform::Ios, TargetPlatform::Android] {
        let mut client = connect(&server, "wrong-token", target);
        assert!(
            matches!(client.read(), Ok(Message::Close(_)) | Err(_)),
            "unauthorized {target:?} runtime must be disconnected during handshake"
        );

        server
            .publish_module(target, module("unauthorized-update", "must stay private"))
            .expect("publishing to the configured target remains valid");
        client
            .get_mut()
            .set_read_timeout(Some(Duration::from_millis(150)))
            .expect("set timeout");
        thread::sleep(Duration::from_millis(200));
        assert!(
            client.read().is_err(),
            "unauthorized {target:?} runtime must not receive module updates"
        );
    }
}

#[test]
fn authenticated_client_receives_full_module_and_compiler_diagnostics_keep_last_good() {
    let server = DevServer::bind([(TargetPlatform::Android, module("good-1", "last good"))])
        .expect("bind dev server");
    let mut client = connect(&server, server.session_token(), TargetPlatform::Android);

    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::Welcome { revision, .. } if revision == "good-1"
    ));
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::FullModule { revision, .. } if revision == "good-1"
    ));

    server
        .publish_diagnostics(
            TargetPlatform::Android,
            vec![Diagnostic {
                severity: Severity::Error,
                file: "App.nx".to_owned(),
                line: 4,
                column: 8,
                message: "unexpected token".to_owned(),
            }],
        )
        .expect("publish diagnostics");
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::Diagnostics { diagnostics } if diagnostics.len() == 1
    ));

    client
        .send(Message::Text(
            encode_client(&ClientMessage::RequestFullModule)
                .expect("encode request")
                .into(),
        ))
        .expect("request retained module");
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::FullModule { revision, .. } if revision == "good-1"
    ));
}

#[test]
fn unchanged_revisions_skip_empty_patches_and_keep_runtime_base_revision() {
    let server = DevServer::bind([(TargetPlatform::Android, module("android-1", "same"))])
        .expect("bind dev server");
    let mut client = connect(&server, server.session_token(), TargetPlatform::Android);
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::Welcome { revision, .. } if revision == "android-1"
    ));
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::FullModule { revision, .. } if revision == "android-1"
    ));

    client
        .get_mut()
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set no-op timeout");
    server
        .publish_module(TargetPlatform::Android, module("android-2", "same"))
        .expect("publish semantically unchanged revision");
    assert!(
        client.read().is_err(),
        "a new revision with no module, identity, or translation changes must not reach the runtime"
    );

    client
        .get_mut()
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("restore update timeout");
    server
        .publish_module(TargetPlatform::Android, module("android-3", "updated"))
        .expect("publish next semantic change");
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::Patch { patch }
            if patch.base_revision == "android-1"
                && patch.revision == "android-3"
                && !patch.operations.is_empty()
    ));
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::Reload { revision } if revision == "android-3"
    ));
}

#[test]
fn authenticated_runtime_can_request_host_editor_navigation() {
    let server = DevServer::bind([(TargetPlatform::Ios, module("ios-1", "last good"))])
        .expect("bind dev server");
    let mut client = connect(&server, server.session_token(), TargetPlatform::Ios);
    let _ = read_server_message(&mut client);
    let _ = read_server_message(&mut client);

    client
        .send(Message::Text(
            encode_client(&ClientMessage::OpenInEditor {
                file: "/work/nexa/App.nx".to_owned(),
                line: 12,
                column: 7,
            })
            .expect("encode editor request")
            .into(),
        ))
        .expect("send editor request");

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(request) = server.try_recv_editor_request() {
            assert_eq!(request.file, "/work/nexa/App.nx");
            assert_eq!(request.line, 12);
            assert_eq!(request.column, 7);
            break;
        }
        assert!(Instant::now() < deadline, "editor request should arrive");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn valid_recompilation_replaces_only_the_matching_platform_module() {
    let server = DevServer::bind([
        (TargetPlatform::Ios, module("ios-1", "ios")),
        (TargetPlatform::Android, module("android-1", "android")),
    ])
    .expect("bind dev server");
    let mut ios = connect(&server, server.session_token(), TargetPlatform::Ios);
    let mut android = connect(&server, server.session_token(), TargetPlatform::Android);
    for client in [&mut ios, &mut android] {
        let _ = read_server_message(client);
        let _ = read_server_message(client);
    }

    server
        .publish_module(TargetPlatform::Ios, module("ios-2", "updated"))
        .expect("publish iOS module");
    assert!(matches!(
        read_server_message(&mut ios),
        ServerMessage::Patch { patch }
            if patch.base_revision == "ios-1" && patch.revision == "ios-2"
    ));
    assert!(matches!(
        read_server_message(&mut ios),
        ServerMessage::Reload { revision } if revision == "ios-2"
    ));
    ios.send(Message::Text(
        encode_client(&ClientMessage::RequestFullModule)
            .expect("encode full module request")
            .into(),
    ))
    .expect("request full module fallback");
    assert!(matches!(
        read_server_message(&mut ios),
        ServerMessage::FullModule { revision, .. } if revision == "ios-2"
    ));
    android
        .get_mut()
        .set_read_timeout(Some(Duration::from_millis(150)))
        .expect("set timeout");
    thread::sleep(Duration::from_millis(200));
    assert!(android.read().is_err());
}

#[test]
fn large_module_change_falls_back_to_full_update() {
    let server = DevServer::bind([(TargetPlatform::Ios, module("ios-1", "initial"))])
        .expect("bind dev server");
    let mut ios = connect(&server, server.session_token(), TargetPlatform::Ios);
    let _ = read_server_message(&mut ios);
    let _ = read_server_message(&mut ios);

    let large_label = "updated task description ".repeat(64);
    server
        .publish_module(TargetPlatform::Ios, module("ios-2", &large_label))
        .expect("publish large iOS update");

    assert!(matches!(
        read_server_message(&mut ios),
        ServerMessage::FullModule { revision, module }
            if revision == "ios-2" && module.module.body.len() == 1
    ));
    assert!(matches!(
        read_server_message(&mut ios),
        ServerMessage::Reload { revision } if revision == "ios-2"
    ));
}

#[test]
fn one_encoded_module_update_reaches_multiple_same_platform_runtimes() {
    let server = DevServer::bind([(TargetPlatform::Ios, module("ios-1", "initial"))])
        .expect("bind dev server");
    let mut first = connect(&server, server.session_token(), TargetPlatform::Ios);
    let mut second = connect(&server, server.session_token(), TargetPlatform::Ios);
    for client in [&mut first, &mut second] {
        let _ = read_server_message(client);
        let _ = read_server_message(client);
    }

    server
        .publish_module(TargetPlatform::Ios, module("ios-2", "updated"))
        .expect("publish iOS module");

    for client in [&mut first, &mut second] {
        assert!(matches!(
            read_server_message(client),
            ServerMessage::Patch { patch }
                if patch.base_revision == "ios-1" && patch.revision == "ios-2"
        ));
        assert!(matches!(
            read_server_message(client),
            ServerMessage::Reload { revision } if revision == "ios-2"
        ));
    }
}

#[test]
fn publish_without_connected_runtime_skips_patch_and_keeps_latest_module() {
    let server = DevServer::bind([(TargetPlatform::Android, module("android-1", "initial"))])
        .expect("bind dev server");

    server
        .publish_module(TargetPlatform::Android, module("android-2", "latest"))
        .expect("publish without a connected runtime");

    let mut client = connect(&server, server.session_token(), TargetPlatform::Android);
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::Welcome { revision, .. } if revision == "android-2"
    ));
    assert!(matches!(
        read_server_message(&mut client),
        ServerMessage::FullModule { revision, module }
            if revision == "android-2" && module.module.body.len() == 1
    ));
}

#[test]
fn hot_restart_reaches_each_authenticated_runtime() {
    let server = DevServer::bind([
        (TargetPlatform::Ios, module("ios-1", "ios")),
        (TargetPlatform::Android, module("android-1", "android")),
    ])
    .expect("bind dev server");
    let mut ios = connect(&server, server.session_token(), TargetPlatform::Ios);
    let mut android = connect(&server, server.session_token(), TargetPlatform::Android);
    for client in [&mut ios, &mut android] {
        let _ = read_server_message(client);
        let _ = read_server_message(client);
    }

    server.publish_restart("keyboard shortcut");

    assert!(matches!(
        read_server_message(&mut ios),
        ServerMessage::Restart { reason } if reason == "keyboard shortcut"
    ));
    assert!(matches!(
        read_server_message(&mut android),
        ServerMessage::Restart { reason } if reason == "keyboard shortcut"
    ));
}

#[test]
fn performance_overlay_toggle_reaches_each_authenticated_runtime() {
    let server = DevServer::bind([
        (TargetPlatform::Ios, module("ios-1", "ios")),
        (TargetPlatform::Android, module("android-1", "android")),
    ])
    .expect("bind dev server");
    let mut ios = connect(&server, server.session_token(), TargetPlatform::Ios);
    let mut android = connect(&server, server.session_token(), TargetPlatform::Android);
    for client in [&mut ios, &mut android] {
        let _ = read_server_message(client);
        let _ = read_server_message(client);
    }

    server.set_performance_overlay(true);

    assert!(matches!(
        read_server_message(&mut ios),
        ServerMessage::PerformanceOverlay { enabled: true }
    ));
    assert!(matches!(
        read_server_message(&mut android),
        ServerMessage::PerformanceOverlay { enabled: true }
    ));
}
