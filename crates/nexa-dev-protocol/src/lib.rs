//! Versioned messages exchanged by Nexa's development server and native
//! development runtimes. This protocol is not linked into release app hosts.

use nexa_dev_ir::DevModulePatch;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 3;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetPlatform {
    Ios,
    Android,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        protocol_version: u16,
        session_token: String,
        target: TargetPlatform,
    },
    Acknowledge {
        revision: String,
    },
    RequestFullModule,
    Disconnect {
        reason: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        protocol_version: u16,
        revision: String,
    },
    FullModule {
        revision: String,
        module: Box<nexa_dev_ir::DevModule>,
    },
    Patch {
        patch: Box<DevModulePatch>,
    },
    Diagnostics {
        diagnostics: Vec<Diagnostic>,
    },
    Reload {
        revision: String,
    },
    Restart {
        reason: String,
    },
    PerformanceOverlay {
        enabled: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
}

/// Encode a client message as one JSON WebSocket text frame.
pub fn encode_client(message: &ClientMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(message)
}

/// Decode one JSON WebSocket text frame from a native development runtime.
pub fn decode_client(frame: &str) -> Result<ClientMessage, serde_json::Error> {
    serde_json::from_str(frame)
}

/// Encode a server message as one JSON WebSocket text frame.
pub fn encode_server(message: &ServerMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(message)
}

/// Decode one JSON WebSocket text frame from the Nexa development server.
pub fn decode_server(frame: &str) -> Result<ServerMessage, serde_json::Error> {
    serde_json::from_str(frame)
}
