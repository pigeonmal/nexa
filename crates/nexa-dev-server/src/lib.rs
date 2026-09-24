//! Authenticated loopback WebSocket server for Nexa development sessions.

use std::{
    collections::HashMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use nexa_dev_ir::DevModule;
use nexa_dev_protocol::{
    ClientMessage, Diagnostic, PROTOCOL_VERSION, ServerMessage, TargetPlatform, decode_client,
    encode_server,
};
use tungstenite::{Message, WebSocket, accept};

const READ_TIMEOUT: Duration = Duration::from_millis(100);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone)]
struct Client {
    target: TargetPlatform,
    sender: Sender<ServerMessage>,
}

#[derive(Default)]
struct SharedState {
    modules: HashMap<TargetPlatform, DevModule>,
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
            if state.modules.insert(target, module).is_some() {
                return Err(format!("duplicate dev module for {target:?}"));
            }
        }
        if state.modules.is_empty() {
            return Err("Nexa dev server requires at least one platform module".to_owned());
        }

        let state = Arc::new(Mutex::new(state));
        let stopping = Arc::new(AtomicBool::new(false));
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
                            let client_id = next_client_id.fetch_add(1, Ordering::Relaxed);
                            let _ = thread::Builder::new()
                                .name("nexa-dev-client".to_owned())
                                .spawn(move || {
                                    serve_client(stream, state, stopping, token, client_id);
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
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Nexa dev server state is unavailable".to_owned())?;
        if !state.modules.contains_key(&target) {
            return Err(format!("no dev session was started for {target:?}"));
        }
        let revision = module.revision.clone();
        let reload_revision = revision.clone();
        let previous = state.modules.insert(target, module.clone());
        state.diagnostics.remove(&target);
        let full_message = ServerMessage::FullModule {
            revision,
            module: Box::new(module.clone()),
        };
        let patch_message = previous
            .as_ref()
            .and_then(|previous| nexa_dev_ir::diff(previous, &module))
            .map(|patch| ServerMessage::Patch {
                patch: Box::new(patch),
            })
            .filter(|patch| {
                let patch_size = serde_json::to_vec(patch).map_or(usize::MAX, |data| data.len());
                let full_size =
                    serde_json::to_vec(&full_message).map_or(usize::MAX, |data| data.len());
                patch_size < full_size
            });
        let message = patch_message.unwrap_or(full_message);
        broadcast(&mut state, target, message);
        broadcast(
            &mut state,
            target,
            ServerMessage::Reload {
                revision: reload_revision,
            },
        );
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
            target,
            ServerMessage::Diagnostics { diagnostics },
        );
        Ok(())
    }

    /// Ask all connected debug runtimes to relaunch their native host.
    pub fn publish_restart(&self, reason: impl Into<String>) {
        let message = ServerMessage::Restart {
            reason: reason.into(),
        };
        if let Ok(state) = self.state.lock() {
            let clients = state.clients.values().map(|client| client.sender.clone());
            for client in clients {
                let _ = client.send(message.clone());
            }
        }
    }

    /// Show or hide the development performance overlay in connected apps.
    pub fn set_performance_overlay(&self, enabled: bool) {
        let message = ServerMessage::PerformanceOverlay { enabled };
        if let Ok(mut state) = self.state.lock() {
            state.performance_overlay_enabled = enabled;
            for client in state.clients.values() {
                let _ = client.sender.send(message.clone());
            }
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

fn broadcast(state: &mut SharedState, target: TargetPlatform, message: ServerMessage) {
    state
        .clients
        .retain(|_, client| client.target != target || client.sender.send(message.clone()).is_ok());
}

fn serve_client(
    stream: TcpStream,
    state: Arc<Mutex<SharedState>>,
    stopping: Arc<AtomicBool>,
    session_token: String,
    client_id: u64,
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

    let (updates_tx, updates_rx) = mpsc::channel();
    let (module, diagnostics, performance_overlay_enabled) = {
        let Ok(mut shared) = state.lock() else {
            let _ = websocket.close(None);
            return;
        };
        let Some(module) = shared.modules.get(&target).cloned() else {
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
    while !stopping.load(Ordering::Acquire) {
        match updates_rx.recv_timeout(READ_TIMEOUT) {
            Ok(message) => {
                if send(&mut websocket, &message).is_err() {
                    break;
                }
                let revision = match &message {
                    ServerMessage::FullModule { revision, .. } => Some(revision.as_str()),
                    ServerMessage::Patch { patch } => Some(patch.revision.as_str()),
                    _ => None,
                };
                match &message {
                    ServerMessage::Patch { patch } => {
                        pending_patch_revisions.insert(patch.revision.clone());
                    }
                    ServerMessage::FullModule { revision, .. } => {
                        pending_patch_revisions.remove(revision);
                    }
                    _ => {}
                }
                if let Some(revision) = revision {
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
                    if pending_patch_revisions.remove(&revision) {
                        println!(
                            "Nexa {} dev runtime applied patch {revision}.",
                            target_name(target)
                        );
                    } else {
                        println!(
                            "Nexa {} dev runtime applied module {revision}.",
                            target_name(target)
                        );
                    }
                }
                Ok(ClientMessage::RequestFullModule) => {
                    let module = state
                        .lock()
                        .ok()
                        .and_then(|shared| shared.modules.get(&target).cloned());
                    let Some(module) = module else {
                        break;
                    };
                    let revision = module.revision.clone();
                    let message = ServerMessage::FullModule {
                        revision: revision.clone(),
                        module: Box::new(module),
                    };
                    pending_patch_revisions.remove(&revision);
                    if send(&mut websocket, &message).is_err() {
                        break;
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
