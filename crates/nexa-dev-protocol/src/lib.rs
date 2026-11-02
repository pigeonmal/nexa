//! Versioned messages exchanged by Nexa's development server and native
//! development runtimes. This protocol is not linked into release app hosts.

use nexa_dev_ir::DevModulePatch;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 14;

/// Template token for native DevRuntime protocol schemas.
pub const PROTOCOL_VERSION_TOKEN: &str = "__NEXA_PROTOCOL_VERSION__";

/// Injects the canonical protocol version into a native runtime template.
fn render_protocol_version(source: &str) -> String {
    source.replace(PROTOCOL_VERSION_TOKEN, &PROTOCOL_VERSION.to_string())
}

/// Renders a native DevRuntime schema with the canonical protocol and Dev IR
/// versions used by the Rust server.
pub fn render_runtime_schema(source: &str) -> String {
    let with_format_version = nexa_dev_ir::render_dev_ir_format_version(source);
    render_protocol_version(&with_format_version)
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetPlatform {
    Ios,
    Android,
}

/// Native host capabilities fingerprint exchanged during handshake and rebuild detection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeHostCapabilities {
    pub protocol_version: u16,
    pub target: TargetPlatform,
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub background_tasks: Vec<String>,
    #[serde(default)]
    pub assets_hash: String,
    #[serde(default)]
    pub rebuild_required: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        protocol_version: u16,
        session_token: String,
        target: TargetPlatform,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capabilities: Option<NativeHostCapabilities>,
    },
    Acknowledge {
        revision: String,
    },
    RequestFullModule,
    OpenInEditor {
        file: String,
        line: u32,
        column: u32,
    },
    Disconnect {
        reason: Option<String>,
    },
    /// Test-only render fence: the native host produced a frame for `revision`.
    ///
    /// This is distinct from `Acknowledge`, which only proves that the module
    /// was committed. Harnesses wait for this frame before capturing pixels.
    FramePresented {
        revision: String,
        frame_id: u64,
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
    RebuildRequired {
        reason: String,
    },
    PerformanceOverlay {
        enabled: bool,
    },
    RuntimeTrace {
        enabled: bool,
    },
    /// Opt-in render-fence reporting for UI and screenshot harnesses.
    FrameFence {
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

/// A source location the connected app asks the host editor to open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRequest {
    pub file: String,
    pub line: u32,
    pub column: u32,
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
