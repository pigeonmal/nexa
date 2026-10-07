use std::collections::HashMap;

use serde_json::json;

use crate::code_actions::code_actions_with_documents;
use crate::completions::get_document_completions;
use nexa_compiler::IncrementalProjectCompiler;

use crate::diagnostics::{check_document, check_project_document};
use crate::hover::get_hover;
use crate::navigation::{definition, references};
use crate::protocol::{
    Diagnostic, JsonRpcError, JsonRpcRequest, JsonRpcResponse, Position, PublishDiagnosticsParams,
};
use crate::semantic_tokens::semantic_tokens;
use crate::signature_help::signature_help;
use crate::symbols::get_document_symbols;

/// In-memory Language Server instance.
pub struct LspServer {
    documents: HashMap<String, String>,
    project_compiler: IncrementalProjectCompiler,
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
            project_compiler: IncrementalProjectCompiler::default(),
            shutdown: false,
        }
    }

    /// Stores or updates an open document.
    pub fn open_or_change_document(&mut self, uri: String, text: String) -> Vec<Diagnostic> {
        let diagnostics = check_project_document(&text, &uri, &mut self.project_compiler)
            .unwrap_or_else(|| check_document(&text, &uri));
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
                            "triggerCharacters": [".", "(", "<", ":", ",", "\""],
                            "resolveProvider": false
                        },
                        "hoverProvider": true,
                        "documentSymbolProvider": true,
                        "definitionProvider": true,
                        "referencesProvider": true,
                        "documentFormattingProvider": true,
                        "signatureHelpProvider": {
                            "triggerCharacters": ["(", ",", ":"],
                            "retriggerCharacters": [",", ":"]
                        },
                        "codeActionProvider": {
                            "codeActionKinds": ["quickfix"]
                        },
                        "semanticTokensProvider": {
                            "legend": {
                                "tokenTypes": crate::semantic_tokens::token_types(),
                                "tokenModifiers": crate::semantic_tokens::token_modifier_names()
                            },
                            "full": true,
                            "range": false
                        }
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
                    notifications.push(PublishDiagnosticsParams {
                        uri: uri.to_string(),
                        diagnostics: Vec::new(),
                    });
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
                        get_document_completions(text, pos, uri.ends_with("/nexa.config.nx"))
                    } else {
                        get_document_completions("", pos, uri.ends_with("/nexa.config.nx"))
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
            "textDocument/semanticTokens/full" => {
                let tokens = if let Some(params) = &request.params {
                    let uri = params
                        .get("textDocument")
                        .and_then(|document| document.get("uri"))
                        .and_then(|value| value.as_str())
                        .unwrap_or_default();
                    self.get_document(uri)
                        .map(semantic_tokens)
                        .map(|(_, _, data)| json!({ "data": data }))
                        .unwrap_or_else(|| json!({ "data": [] }))
                } else {
                    json!({ "data": [] })
                };
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(tokens),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/definition" => {
                let location = request.params.as_ref().and_then(|params| {
                    let uri = params
                        .get("textDocument")
                        .and_then(|document| document.get("uri"))
                        .and_then(|value| value.as_str())?;
                    let position = serde_json::from_value(params.get("position")?.clone()).ok()?;
                    let (target_uri, span) = definition(&self.documents, uri, position)?;
                    let target_source = self.documents.get(&target_uri)?;
                    Some(json!({
                        "uri": target_uri,
                        "range": crate::line_index::span_to_range(&span, target_source)
                    }))
                });
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(location.unwrap_or(serde_json::Value::Null)),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/references" => {
                let locations = request
                    .params
                    .as_ref()
                    .and_then(|params| {
                        let uri = params
                            .get("textDocument")
                            .and_then(|document| document.get("uri"))
                            .and_then(|value| value.as_str())?;
                        let position = serde_json::from_value(
                            params.get("position")?.clone(),
                        )
                        .ok()?;
                        let include_declaration = params
                            .get("context")
                            .and_then(|context| context.get("includeDeclaration"))
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                        Some(
                            references(&self.documents, uri, position, include_declaration)
                                .into_iter()
                                .filter_map(|(target_uri, span)| {
                                    let target_source = self.documents.get(&target_uri)?;
                                    Some(json!({
                                        "uri": target_uri,
                                        "range": crate::line_index::span_to_range(&span, target_source)
                                    }))
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .unwrap_or_default();
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!(locations)),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/formatting" => {
                let edits = request.params.as_ref().and_then(|params| {
                    let uri = params
                        .get("textDocument")
                        .and_then(|document| document.get("uri"))
                        .and_then(|value| value.as_str())?;
                    let source = self.documents.get(uri)?;
                    let options = params.get("options");
                    let tab_size = options
                        .and_then(|value| value.get("tabSize"))
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|value| usize::try_from(value).ok())
                        .unwrap_or(4);
                    let insert_spaces = options
                        .and_then(|value| value.get("insertSpaces"))
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(true);
                    let formatted =
                        nexa_syntax::format_source_with_options(source, tab_size, insert_spaces)
                            .ok()?;
                    if formatted == *source {
                        return Some(Vec::new());
                    }
                    let end =
                        crate::line_index::LineIndex::new(source).to_position(source, source.len());
                    Some(vec![json!({
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": end
                        },
                        "newText": formatted
                    })])
                });
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!(edits.unwrap_or_default())),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/signatureHelp" => {
                let signature = request.params.as_ref().and_then(|params| {
                    let uri = params
                        .get("textDocument")
                        .and_then(|document| document.get("uri"))
                        .and_then(|value| value.as_str())?;
                    let position = serde_json::from_value(params.get("position")?.clone()).ok()?;
                    signature_help(self.get_document(uri)?, position)
                });
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(signature.unwrap_or(serde_json::Value::Null)),
                        error: None,
                    }),
                    notifications,
                )
            }
            "textDocument/codeAction" => {
                let actions = request
                    .params
                    .as_ref()
                    .and_then(|params| {
                        let uri = params
                            .get("textDocument")
                            .and_then(|document| document.get("uri"))
                            .and_then(|value| value.as_str())?;
                        let source = self.get_document(uri)?;
                        Some(code_actions_with_documents(
                            source,
                            uri,
                            params.get("context").unwrap_or(&serde_json::Value::Null),
                            self.documents.values().map(String::as_str),
                        ))
                    })
                    .unwrap_or_default();
                (
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!(actions)),
                        error: None,
                    }),
                    notifications,
                )
            }
            _ if request.id.is_some() => (
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
            _ => (None, notifications),
        }
    }
}
