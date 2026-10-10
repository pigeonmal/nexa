//! Authenticated loopback WebSocket server for Nexa development sessions.

use std::{
    collections::HashMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use nexa_dev_ir::DevModule;
use nexa_dev_protocol::{
    ClientMessage, Diagnostic, EditorRequest, PROTOCOL_VERSION, ServerMessage, TargetPlatform,
    decode_client, encode_server,
};
use tungstenite::{Bytes, Message, Utf8Bytes, WebSocket, accept};

// The blocking WebSocket read also waits on this interval before the loop can
// observe a newly published module. Keep that delivery bound small enough for
// interactive edits while avoiding a busy spin on connected dev clients.
const READ_TIMEOUT: Duration = Duration::from_millis(10);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone)]
struct Client {
    target: TargetPlatform,
    sender: Sender<Arc<ClientUpdate>>,
}

/// One encoded broadcast frame shared by all clients that receive the update.
struct ClientUpdate {
    frame: Utf8Bytes,
    revision: Option<String>,
    is_patch: bool,
}

/// Keep the last serialized IR shape so diffing a reload does not serialize the
/// previous full module again. The JSON and full-frame size are lazy because
/// a disconnected dev session only needs to retain its latest typed module.
struct PublishedModule {
    module: DevModule,
    json: Option<serde_json::Value>,
    full_frame_len: Option<usize>,
}

#[derive(Default)]
struct SharedState {
    modules: HashMap<TargetPlatform, PublishedModule>,
    diagnostics: HashMap<TargetPlatform, Vec<Diagnostic>>,
    clients: HashMap<u64, Client>,
    performance_overlay_enabled: bool,
}

/// Owns a local development server and its ephemeral authorization token.
/// The token must only be handed to the debug app instance launched by Nexa.
pub struct DevServer {
    address: SocketAddr,
    session_token: String,
    state: Arc<Mutex<SharedState>>,
    editor_requests: Receiver<EditorRequest>,
    stopping: Arc<AtomicBool>,
    listener_thread: Option<JoinHandle<()>>,
}

impl DevServer {
    /// Bind an ephemeral TCP port on IPv4 loopback and start serving the given
    /// platform-specific, last-known-good modules.
    pub fn bind(
        modules: impl IntoIterator<Item = (TargetPlatform, DevModule)>,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .map_err(|error| format!("cannot bind Nexa dev server to loopback: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("cannot configure Nexa dev server: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("cannot read Nexa dev server address: {error}"))?;

        let mut state = SharedState::default();
        for (target, module) in modules {
            if module.protocol_version != nexa_dev_ir::DEV_IR_FORMAT_VERSION {
                return Err(format!(
                    "unsupported Dev IR format {}; this server supports {}",
                    module.protocol_version,
                    nexa_dev_ir::DEV_IR_FORMAT_VERSION
                ));
            }
            let json = serde_json::to_value(&module.module)
                .map_err(|error| format!("cannot cache the {target:?} dev module: {error}"))?;
            let full_frame_len = encode_update(&ServerMessage::FullModule {
                revision: module.revision.clone(),
                module: Box::new(module.clone()),
            })?
            .frame
            .len();
            if state
                .modules
                .insert(
                    target,
                    PublishedModule {
                        module,
                        json: Some(json),
                        full_frame_len: Some(full_frame_len),
                    },
                )
                .is_some()
            {
                return Err(format!("duplicate dev module for {target:?}"));
            }
        }
        if state.modules.is_empty() {
            return Err("Nexa dev server requires at least one platform module".to_owned());
        }

        let state = Arc::new(Mutex::new(state));
        let stopping = Arc::new(AtomicBool::new(false));
        let (editor_request_sender, editor_requests) = mpsc::channel();
        let session_token = uuid::Uuid::new_v4().simple().to_string();
        let listener_state = Arc::clone(&state);
        let listener_stopping = Arc::clone(&stopping);
        let listener_token = session_token.clone();
        let listener_thread = thread::Builder::new()
            .name("nexa-dev-server".to_owned())
            .spawn(move || {
                let next_client_id = AtomicU64::new(1);
                while !listener_stopping.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, peer)) => {
                            if !peer.ip().is_loopback() {
                                continue;
                            }
                            if stream.set_nonblocking(false).is_err() {
                                continue;
                            }
                            let state = Arc::clone(&listener_state);
                            let stopping = Arc::clone(&listener_stopping);
                            let token = listener_token.clone();
                            let editor_requests = editor_request_sender.clone();
                            let client_id = next_client_id.fetch_add(1, Ordering::Relaxed);
                            let _ = thread::Builder::new()
                                .name("nexa-dev-client".to_owned())
                                .spawn(move || {
                                    serve_client(
                                        stream,
                                        state,
                                        stopping,
                                        token,
                                        client_id,
                                        editor_requests,
                                    );
                                });
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(20));
                        }
                        Err(_) => thread::sleep(Duration::from_millis(50)),
                    }
                }
            })
            .map_err(|error| format!("cannot start Nexa dev server thread: {error}"))?;

        Ok(Self {
            address,
            session_token,
            state,
            editor_requests,
            stopping,
            listener_thread: Some(listener_thread),
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn session_token(&self) -> &str {
        &self.session_token
    }

    /// Return the next authenticated editor-open request, if one is waiting.
    pub fn try_recv_editor_request(&self) -> Option<EditorRequest> {
        self.editor_requests.try_recv().ok()
    }

    /// Publish a newly compiled module. The update is routed only to clients
    /// connected to the matching platform; this becomes the last-good module.
    pub fn publish_module(&self, target: TargetPlatform, module: DevModule) -> Result<(), String> {
        if module.protocol_version != nexa_dev_ir::DEV_IR_FORMAT_VERSION {
            return Err(format!(
                "unsupported Dev IR format {}; this server supports {}",
                module.protocol_version,
                nexa_dev_ir::DEV_IR_FORMAT_VERSION
            ));
        }
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "Nexa dev server state is unavailable".to_owned())?;
            if !state.modules.contains_key(&target) {
                return Err(format!("no dev session was started for {target:?}"));
            }
            let has_client = state.clients.values().any(|client| client.target == target);
            if !has_client {
                state.modules.insert(
                    target,
                    PublishedModule {
                        module,
                        json: None,
                        // A disconnected update can change the module size by
                        // an arbitrary amount. Force one full publish after a
                        // runtime reconnect to refresh the size baseline.
                        full_frame_len: None,
                    },
                );
                state.diagnostics.remove(&target);
                return Ok(());
            }
        }

        let module_json = serde_json::to_value(&module.module)
            .map_err(|error| format!("cannot cache the {target:?} dev module: {error}"))?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Nexa dev server state is unavailable".to_owned())?;
        if !state.modules.contains_key(&target) {
            return Err(format!("no dev session was started for {target:?}"));
        }

        let revision = module.revision.clone();
        let patch = state.modules.get(&target).and_then(|previous| {
            previous.json.as_ref().map_or_else(
                || nexa_dev_ir::diff(&previous.module, &module),
                |previous_json| {
                    nexa_dev_ir::diff_with_module_values(
                        &previous.module,
                        previous_json,
                        &module,
                        &module_json,
                    )
                },
            )
        });
        if patch.as_ref().is_some_and(|patch| {
            patch.operations.is_empty()
                && patch.identities.is_none()
                && patch.translations.is_none()
        }) {
            // Filesystem watchers can report a generated localization write
            // after the source edit that caused it. The second compile then
            // produces a new revision with no semantic IR changes. Keep the
            // runtime and cached revision aligned, and avoid a redundant
            // empty patch/recomposition.
            return Ok(());
        }
        let patch_message = patch
            .map(|patch| {
                encode_update(&ServerMessage::Patch {
                    patch: Box::new(patch),
                })
            })
            .transpose()?;
        let previous_full_frame_len = state
            .modules
            .get(&target)
            .and_then(|previous| previous.full_frame_len);
        // Tiny patches are guaranteed to be much smaller than a full module
        // frame at the last known size. Avoid cloning and encoding the full IR
        // on the common source-edit path; fall back to a full module for large
        // structural changes and refresh the cached size then.
        let patch_is_clearly_smaller = patch_message.as_ref().is_some_and(|patch| {
            previous_full_frame_len.is_some_and(|full_len| patch.frame.len() < full_len / 2)
        });
        let (update, full_frame_len) = match (patch_is_clearly_smaller, patch_message) {
            (true, Some(patch)) => (patch, previous_full_frame_len),
            _ => {
                let full_update = encode_update(&ServerMessage::FullModule {
                    revision: revision.clone(),
                    module: Box::new(module.clone()),
                })?;
                let full_frame_len = full_update.frame.len();
                (full_update, Some(full_frame_len))
            }
        };
        state.modules.insert(
            target,
            PublishedModule {
                module,
                json: Some(module_json),
                full_frame_len,
            },
        );
        state.diagnostics.remove(&target);
        broadcast_encoded(&mut state, Some(target), update);
        broadcast(&mut state, Some(target), ServerMessage::Reload { revision })?;
        Ok(())
    }

    /// Send compiler diagnostics without replacing the last-good module.
    pub fn publish_diagnostics(
        &self,
        target: TargetPlatform,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Nexa dev server state is unavailable".to_owned())?;
        if !state.modules.contains_key(&target) {
            return Err(format!("no dev session was started for {target:?}"));
        }
        state.diagnostics.insert(target, diagnostics.clone());
        broadcast(
            &mut state,
            Some(target),
            ServerMessage::Diagnostics { diagnostics },
        )?;
        Ok(())
    }

    /// Ask all connected debug runtimes to relaunch their native host.
    pub fn publish_restart(&self, reason: impl Into<String>) {
        let message = ServerMessage::Restart {
            reason: reason.into(),
        };
        if let Ok(mut state) = self.state.lock() {
            let _ = broadcast(&mut state, None, message);
        }
    }

    /// Show or hide the development performance overlay in connected apps.
    pub fn set_performance_overlay(&self, enabled: bool) {
        let message = ServerMessage::PerformanceOverlay { enabled };
        if let Ok(mut state) = self.state.lock() {
            state.performance_overlay_enabled = enabled;
            let _ = broadcast(&mut state, None, message);
        }
    }
}

impl Drop for DevServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(thread) = self.listener_thread.take() {
            let _ = thread.join();
        }
    }
}

fn encode_update(message: &ServerMessage) -> Result<Arc<ClientUpdate>, String> {
    let (revision, is_patch) = match message {
        ServerMessage::FullModule { revision, .. } => (Some(revision.clone()), false),
        ServerMessage::Patch { patch } => (Some(patch.revision.clone()), true),
        _ => (None, false),
    };
    let frame = encode_server(message)
        .map_err(|error| format!("cannot encode Nexa dev update: {error}"))?;
    let frame = Utf8Bytes::try_from(Bytes::from(frame))
        .map_err(|error| format!("cannot encode Nexa dev update as UTF-8: {error}"))?;
    Ok(Arc::new(ClientUpdate {
        frame,
        revision,
        is_patch,
    }))
}

fn broadcast(
    state: &mut SharedState,
    target: Option<TargetPlatform>,
    message: ServerMessage,
) -> Result<(), String> {
    let update = encode_update(&message)?;
    broadcast_encoded(state, target, update);
    Ok(())
}

fn broadcast_encoded(
    state: &mut SharedState,
    target: Option<TargetPlatform>,
    update: Arc<ClientUpdate>,
) {
    state.clients.retain(|_, client| {
        if target.is_some_and(|target| target != client.target) {
            return true;
        }
        client.sender.send(Arc::clone(&update)).is_ok()
    });
}

fn serve_client(
    stream: TcpStream,
    state: Arc<Mutex<SharedState>>,
    stopping: Arc<AtomicBool>,
    session_token: String,
    client_id: u64,
    editor_requests: Sender<EditorRequest>,
) {
    if stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT)).is_err() {
        return;
    }
    let Ok(mut websocket) = accept(stream) else {
        return;
    };

    let target = match websocket.read() {
        Ok(Message::Text(text)) => match decode_client(text.as_str()) {
            Ok(ClientMessage::Hello {
                protocol_version,
                session_token: supplied_token,
                target,
            }) => {
                let version_ok = protocol_version == PROTOCOL_VERSION;
                let token_ok = token_matches(&session_token, &supplied_token);
                if version_ok && token_ok {
                    target
                } else {
                    let _ = websocket.close(None);
                    return;
                }
            }
            _ => {
                let _ = websocket.close(None);
                return;
            }
        },
        _ => {
            let _ = websocket.close(None);
            return;
        }
    };

    let (updates_tx, updates_rx) = mpsc::channel::<Arc<ClientUpdate>>();
    let (module, diagnostics, performance_overlay_enabled) = {
        let Ok(mut shared) = state.lock() else {
            let _ = websocket.close(None);
            return;
        };
        let Some(module) = shared
            .modules
            .get(&target)
            .map(|published| published.module.clone())
        else {
            let _ = websocket.close(None);
            return;
        };
        shared.clients.insert(
            client_id,
            Client {
                target,
                sender: updates_tx,
            },
        );
        (
            module,
            shared.diagnostics.get(&target).cloned().unwrap_or_default(),
            shared.performance_overlay_enabled,
        )
    };

    let revision = module.revision.clone();
    let initial = [
        ServerMessage::Welcome {
            protocol_version: PROTOCOL_VERSION,
            revision: revision.clone(),
        },
        ServerMessage::FullModule {
            revision,
            module: Box::new(module),
        },
    ];
    for message in initial {
        if send(&mut websocket, &message).is_err() {
            unregister(&state, client_id);
            return;
        }
    }
    if performance_overlay_enabled
        && send(
            &mut websocket,
            &ServerMessage::PerformanceOverlay { enabled: true },
        )
        .is_err()
    {
        unregister(&state, client_id);
        return;
    }
    if !diagnostics.is_empty()
        && send(&mut websocket, &ServerMessage::Diagnostics { diagnostics }).is_err()
    {
        unregister(&state, client_id);
        return;
    }
    println!("Nexa {} dev runtime connected.", target_name(target));
    let _ = websocket.get_mut().set_read_timeout(Some(READ_TIMEOUT));

    let mut pending_patch_revisions = std::collections::HashSet::new();
    let mut pending_apply_times = HashMap::new();
    while !stopping.load(Ordering::Acquire) {
        match updates_rx.recv_timeout(READ_TIMEOUT) {
            Ok(update) => {
                let sent_at = std::time::Instant::now();
                if websocket.send(Message::Text(update.frame.clone())).is_err() {
                    break;
                }
                if let Some(revision) = update.revision.as_ref() {
                    if update.is_patch {
                        pending_patch_revisions.insert(revision.clone());
                    } else {
                        pending_patch_revisions.remove(revision);
                    }
                    pending_apply_times.insert(revision.clone(), sent_at);
                    println!(
                        "Nexa {} dev runtime received module {revision}.",
                        target_name(target)
                    );
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        match websocket.read() {
            Ok(Message::Text(text)) => match decode_client(text.as_str()) {
                Ok(ClientMessage::Acknowledge { revision }) => {
                    let elapsed = pending_apply_times
                        .remove(&revision)
                        .map(|sent_at| sent_at.elapsed().as_millis());
                    if pending_patch_revisions.remove(&revision) {
                        println!(
                            "Nexa {} dev runtime applied patch {revision}{}.",
                            target_name(target),
                            elapsed.map_or_else(String::new, |millis| format!(" in {millis} ms"))
                        );
                    } else {
                        println!(
                            "Nexa {} dev runtime applied module {revision}{}.",
                            target_name(target),
                            elapsed.map_or_else(String::new, |millis| format!(" in {millis} ms"))
                        );
                    }
                }
                Ok(ClientMessage::RequestFullModule) => {
                    let module = state.lock().ok().and_then(|shared| {
                        shared
                            .modules
                            .get(&target)
                            .map(|published| published.module.clone())
                    });
                    let Some(module) = module else {
                        break;
                    };
                    let revision = module.revision.clone();
                    let message = ServerMessage::FullModule {
                        revision: revision.clone(),
                        module: Box::new(module),
                    };
                    pending_patch_revisions.remove(&revision);
                    let sent_at = std::time::Instant::now();
                    if send(&mut websocket, &message).is_err() {
                        break;
                    }
                    pending_apply_times.insert(revision, sent_at);
                }
                Ok(ClientMessage::OpenInEditor { file, line, column }) => {
                    if !file.is_empty() && line > 0 && column > 0 {
                        let _ = editor_requests.send(EditorRequest { file, line, column });
                    }
                }
                Ok(ClientMessage::Disconnect { .. }) => break,
                Ok(ClientMessage::Hello { .. }) => {
                    eprintln!(
                        "Nexa {} dev runtime sent a second handshake; closing session.",
                        target_name(target)
                    );
                    break;
                }
                Err(error) => {
                    eprintln!(
                        "Nexa {} dev runtime sent an invalid message: {error}",
                        target_name(target)
                    );
                    break;
                }
            },
            Ok(Message::Ping(payload)) => {
                if websocket.send(Message::Pong(payload)).is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) | Err(tungstenite::Error::ConnectionClosed) => break,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Ok(Message::Binary(_) | Message::Pong(_) | Message::Frame(_))
            | Err(tungstenite::Error::AlreadyClosed) => {
                break;
            }
            Err(_) => break,
        }
    }
    unregister(&state, client_id);
}

fn target_name(target: TargetPlatform) -> &'static str {
    match target {
        TargetPlatform::Ios => "iOS",
        TargetPlatform::Android => "Android",
    }
}

fn send<S: io::Read + io::Write>(
    websocket: &mut WebSocket<S>,
    message: &ServerMessage,
) -> Result<(), tungstenite::Error> {
    let frame = encode_server(message).map_err(|error| {
        tungstenite::Error::Io(io::Error::new(io::ErrorKind::InvalidData, error))
    })?;
    websocket.send(Message::Text(frame.into()))
}

fn unregister(state: &Arc<Mutex<SharedState>>, client_id: u64) {
    if let Ok(mut shared) = state.lock() {
        shared.clients.remove(&client_id);
    }
}

fn token_matches(expected: &str, supplied: &str) -> bool {
    if expected.len() != supplied.len() {
        return false;
    }
    expected
        .bytes()
        .zip(supplied.bytes())
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}
