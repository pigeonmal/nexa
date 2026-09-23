use std::collections::HashMap;

use serde_json::json;

use crate::completions::get_completions;
use crate::diagnostics::check_source;
use crate::hover::get_hover;
use crate::protocol::{
    Diagnostic, JsonRpcError, JsonRpcRequest, JsonRpcResponse, Position,
    PublishDiagnosticsParams,
};
use crate::symbols::get_document_symbols;

/// In-memory Language Server instance.
pub struct LspServer {
    documents: HashMap<String, String>,
    shutdown: bool,
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            shutdown: false,
        }
    }

    /// Stores or updates an open document.
    pub fn open_or_change_document(&mut self, uri: String, text: String) -> Vec<Diagnostic> {
        let diagnostics = check_source(&text);
        self.documents.insert(uri, text);
        diagnostics
    }

    /// Closes and removes a document.
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Returns the contents of an open document.
    pub fn get_document(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|s| s.as_str())
    }

    /// Processes an incoming JSON-RPC request and returns an optional JSON-RPC response.
    /// Also returns any notification (such as publishDiagnostics) that should be sent.
    pub fn handle_request(
        &mut self,
        request: JsonRpcRequest,
    ) -> (Option<JsonRpcResponse>, Vec<PublishDiagnosticsParams>) {
        let mut notifications = Vec::new();

        match request.method.as_str() {
            "initialize" => {
                let result = json!({
                    "capabilities": {
                        "textDocumentSync": 1, // Full sync
                        "completionProvider": {
                            "triggerCharacters": [".", "(", "<"],
                            "resolveProvider": false
                        },
                        "hoverProvider": true,
                        "documentSymbolProvider": true
                    },
                    "serverInfo": {
                        "name": "nexa-lsp",
                        "version": "0.1.0"
                    }
                });
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(result),
                        error: None,
                    }),
                    notifications,
                )
            }
            "initialized" => (None, notifications),
            "shutdown" => {
                self.shutdown = true;
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(serde_json::Value::Null),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/didOpen" => {
                let params = request.params.as_ref();
                let doc = params.and_then(|p| p.get("textDocument"));
                let uri = doc.and_then(|d| d.get("uri")).and_then(|u| u.as_str());
                let text = doc.and_then(|d| d.get("text")).and_then(|t| t.as_str());

                if let (Some(uri), Some(text)) = (uri, text) {
                    let diags = self.open_or_change_document(uri.to_string(), text.to_string());
                    notifications.push(PublishDiagnosticsParams {
                        uri: uri.to_string(),
                        diagnostics: diags,
                    });
                }
                (None, notifications)
            }
            "textDocument/didChange" => {
                let params = request.params.as_ref();
                let doc = params.and_then(|p| p.get("textDocument"));
                let uri = doc.and_then(|d| d.get("uri")).and_then(|u| u.as_str());
                let text = params
                    .and_then(|p| p.get("contentChanges"))
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|fc| fc.get("text"))
                    .and_then(|t| t.as_str());

                if let (Some(uri), Some(text)) = (uri, text) {
                    let diags = self.open_or_change_document(uri.to_string(), text.to_string());
                    notifications.push(PublishDiagnosticsParams {
                        uri: uri.to_string(),
                        diagnostics: diags,
                    });
                }
                (None, notifications)
            }
            "textDocument/didClose" => {
                let uri = request
                    .params
                    .as_ref()
                    .and_then(|p| p.get("textDocument"))
                    .and_then(|d| d.get("uri"))
                    .and_then(|u| u.as_str());

                if let Some(uri) = uri {
                    self.close_document(uri);
                }
                (None, notifications)
            }
            "textDocument/completion" => {
                let completions = if let Some(params) = &request.params {
                    let uri = params
                        .get("textDocument")
                        .and_then(|d| d.get("uri"))
                        .and_then(|u| u.as_str())
                        .unwrap_or_default();
                    let pos: Position = params
                        .get("position")
                        .and_then(|p| serde_json::from_value(p.clone()).ok())
                        .unwrap_or_default();

                    if let Some(text) = self.get_document(uri) {
                        get_completions(text, pos)
                    } else {
                        get_completions("", pos)
                    }
                } else {
                    Vec::new()
                };

                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!(completions)),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/hover" => {
                let hover = if let Some(params) = &request.params {
                    let uri = params
                        .get("textDocument")
                        .and_then(|d| d.get("uri"))
                        .and_then(|u| u.as_str())
                        .unwrap_or_default();
                    let pos: Position = params
                        .get("position")
                        .and_then(|p| serde_json::from_value(p.clone()).ok())
                        .unwrap_or_default();

                    if let Some(text) = self.get_document(uri) {
                        get_hover(text, pos)
                    } else {
                        None
                    }
                } else {
                    None
                };

                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!(hover)),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/documentSymbol" => {
                let symbols = if let Some(params) = &request.params {
                    let uri = params
                        .get("textDocument")
                        .and_then(|d| d.get("uri"))
                        .and_then(|u| u.as_str())
                        .unwrap_or_default();

                    if let Some(text) = self.get_document(uri) {
                        get_document_symbols(text)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!(symbols)),
                        error: None,
                    }),
                    notifications,
                )
            }
            _ => (
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                }),
                notifications,
            ),
        }
    }
}
