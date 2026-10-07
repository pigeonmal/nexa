use nexa_lsp::LineIndex;
use nexa_lsp::protocol::{DiagnosticSeverity, JsonRpcRequest};
use nexa_lsp::server::LspServer;
use serde_json::json;

#[test]
fn lsp_initialization_reports_capabilities() {
    let mut server = LspServer::new();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({})),
    };

    let (response, _) = server.handle_request(request);
    let resp = response.expect("response should be returned");
    assert_eq!(resp.id, Some(json!(1)));
    assert!(resp.error.is_none());

    let result = resp.result.expect("result should exist");
    assert_eq!(result["capabilities"]["textDocumentSync"], 1);
    assert!(result["capabilities"]["completionProvider"].is_object());
    assert_eq!(result["capabilities"]["hoverProvider"], true);
    assert_eq!(result["capabilities"]["documentSymbolProvider"], true);
    assert_eq!(
        result["capabilities"]["semanticTokensProvider"]["full"],
        true
    );
    assert_eq!(result["capabilities"]["documentFormattingProvider"], true);
    assert!(result["capabilities"]["signatureHelpProvider"].is_object());
    assert!(result["capabilities"]["codeActionProvider"].is_object());
}

#[test]
fn lsp_publishes_syntax_error_diagnostics() {
    let mut server = LspServer::new();
    let broken_source =
        "app BrokenApp {\n    body {\n        Text(\"missing closing brace\"\n    \n";

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test.nx",
                "text": broken_source
            }
        })),
    };

    let (_, notifications) = server.handle_request(request);
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].uri, "file:///test.nx");
    assert!(!notifications[0].diagnostics.is_empty());
    assert_eq!(
        notifications[0].diagnostics[0].severity,
        Some(DiagnosticSeverity::Error)
    );
}

#[test]
fn lsp_reports_clean_diagnostics_for_valid_app() {
    let mut server = LspServer::new();
    let valid_source = r#"
app CounterApp {
    state count: Int32 = 0
    body {
        Column(spacing: 12) {
            Text("Count")
            Button("Increment") {
                count = count + 1
            }
        }
    }
}
"#;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///counter.nx",
                "text": valid_source
            }
        })),
    };

    let (_, notifications) = server.handle_request(request);
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].diagnostics.len(), 0);
}

#[test]
fn lsp_provides_rich_completions() {
    let mut server = LspServer::new();
    let source = "app Demo {\n    state count: Int32 = 0\n    body {\n        Text(count)\n        \n    }\n}\n";
    let (open_response, notifications) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.nx", "text": source }
        })),
    });
    assert!(open_response.is_none());
    assert!(
        notifications[0]
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Some(DiagnosticSeverity::Error))
    );
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/completion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.nx" },
            "position": { "line": 4, "character": 8 }
        })),
    };

    let (response, _) = server.handle_request(request);
    let resp = response.expect("response should be returned");
    let result = resp.result.expect("result should exist");
    let completions = result.as_array().expect("completions is an array");

    let labels: Vec<&str> = completions
        .iter()
        .filter_map(|c| c.get("label").and_then(|l| l.as_str()))
        .collect();

    assert!(labels.contains(&"Column"));
    assert!(labels.contains(&"Row"));
    assert!(labels.contains(&"FastList"));
    assert!(labels.contains(&"Text"));
    assert!(labels.contains(&"Button"));
    assert!(labels.contains(&"if"));
    assert!(labels.contains(&"count"));
    assert!(!labels.contains(&"state"));
}

#[test]
fn lsp_provides_hover_documentation() {
    let mut server = LspServer::new();
    let source = "app Demo {\n    body {\n        FastList(items) {}\n    }\n}\n";

    server.open_or_change_document("file:///hover.nx".to_string(), source.to_string());

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(3)),
        method: "textDocument/hover".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///hover.nx" },
            "position": { "line": 2, "character": 10 } // over "FastList"
        })),
    };

    let (response, _) = server.handle_request(request);
    let resp = response.expect("response should be returned");
    let result = resp.result.expect("result should exist");
    assert!(result.is_object());
    let doc = result["contents"]["value"]
        .as_str()
        .expect("hover markdown");
    assert!(doc.contains("FastList"));
    assert!(doc.contains("virtualized"));
}

#[test]
fn lsp_rejects_unsupported_component_names() {
    // Names with no parser production parse as custom component calls and
    // must fail full compilation. Completions and hover never offer them
    // (see the catalog tests); this proves offering them would be an error.
    for name in [
        "TextField",
        "FastSectionedList",
        "VStack",
        "HStack",
        "ZStack",
    ] {
        let source = format!("app P {{\n    body {{\n        {name}()\n    }}\n}}\n");
        let diagnostics = nexa_lsp::check_source(&source);
        assert!(!diagnostics.is_empty(), "{name} should not compile cleanly");
    }
    for component in [
        "Spacer()",
        "Divider(color: \"#808080\", thickness: 1)",
        "Slider(value: amount, min: 0.0, max: 1.0, step: 0.1)",
        "ProgressBar(progress: progress)",
        "ProgressRing(progress: progress)",
        "Dialog(isPresented: show, title: \"Title\", message: \"Message\") { Button(\"OK\") { show = false } }",
        "SegmentedControl(items: filters, selected: selectedFilter)",
        "Picker(items: sizes, selected: selectedSize)",
        "TextInput(value: name, placeholder: \"Name\")",
    ] {
        let state = match component {
            value if value.contains("value: name") => "    state name: String = \"\"\n",
            value if value.contains("value: amount") => "    state amount: Float64 = 0.5\n",
            value if value.contains("progress: progress") => "    state progress: Float64 = 0.5\n",
            value if value.contains("isPresented: show") => "    state show: Bool = false\n",
            value if value.contains("items: filters") => {
                "    state filters: Array<String> = [\"All\", \"Open\"]\n    state selectedFilter: String = \"All\"\n"
            }
            value if value.contains("items: sizes") => {
                "    state sizes: Array<String> = [\"Small\", \"Medium\"]\n    state selectedSize: String = \"Small\"\n"
            }
            _ => "",
        };
        let source = format!("app P {{\n{state}    body {{\n        {component}\n    }}\n}}\n");
        let diagnostics = nexa_lsp::check_source(&source);
        assert!(
            diagnostics.is_empty(),
            "{component} should compile cleanly: {diagnostics:?}"
        );
    }
}

#[test]
fn lsp_completion_response_matches_the_catalog() {
    let mut server = LspServer::new();
    server.open_or_change_document(
        "file:///catalog.nx".to_string(),
        "app P {\n    body {\n        \n    }\n}\n".to_string(),
    );
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(5)),
        method: "textDocument/completion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///catalog.nx" },
            "position": { "line": 2, "character": 8 }
        })),
    };

    let (response, _) = server.handle_request(request);
    let result = response
        .expect("response should be returned")
        .result
        .expect("result");
    let completions = result.as_array().expect("completions is an array");
    let labels: Vec<&str> = completions
        .iter()
        .filter_map(|c| c.get("label").and_then(|l| l.as_str()))
        .collect();

    for supported in [
        "Column",
        "TextInput",
        "FastList",
        "Pressable",
        "Spacer",
        "Divider",
        "Slider",
        "ProgressBar",
        "ProgressRing",
        "Dialog",
        "SegmentedControl",
        "Picker",
    ] {
        assert!(labels.contains(&supported), "{supported} should be offered");
    }
    assert!(
        !labels.contains(&"onPress"),
        "dot modifiers need member position"
    );
    for rejected in [
        "TextField",
        "FastSectionedList",
        "VStack",
        "HStack",
        "ZStack",
    ] {
        assert!(
            !labels.contains(&rejected),
            "{rejected} must not be offered"
        );
    }
}

#[test]
fn lsp_extracts_document_symbols_hierarchy() {
    let mut server = LspServer::new();
    let source = r#"
screen SharedScreen {
    Text("Shared")
}

component TodoRow(title: String) {
    body { Text(title) }
}

struct Item {
    title: String
}

app TodoApp {
    enum Priority {
        low,
        high
    }

    state items_count: Int32 = 0

    fn refresh() -> Void {
        items_count = 0
    }

    body {
        NavigationStack(root: SharedScreen)
    }
}
"#;

    server.open_or_change_document("file:///todo.nx".to_string(), source.to_string());

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(4)),
        method: "textDocument/documentSymbol".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///todo.nx" }
        })),
    };

    let (response, _) = server.handle_request(request);
    let resp = response.expect("response should be returned");
    let result = resp.result.expect("result should exist");
    let symbols = result.as_array().expect("symbols is an array");

    fn assert_document_symbol_shape(symbols: &[serde_json::Value]) {
        for symbol in symbols {
            assert!(
                symbol
                    .get("kind")
                    .and_then(serde_json::Value::as_u64)
                    .is_some()
            );
            assert!(
                symbol
                    .get("range")
                    .and_then(serde_json::Value::as_object)
                    .is_some()
            );
            assert!(
                symbol
                    .get("selectionRange")
                    .and_then(serde_json::Value::as_object)
                    .is_some()
            );
            if let Some(children) = symbol.get("children").and_then(serde_json::Value::as_array) {
                assert_document_symbol_shape(children);
            }
        }
    }

    assert_document_symbol_shape(symbols);

    fn collect_names<'a>(vals: &'a [serde_json::Value], acc: &mut Vec<&'a str>) {
        for v in vals {
            if let Some(name) = v.get("name").and_then(|n| n.as_str()) {
                acc.push(name);
            }
            if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
                collect_names(children, acc);
            }
        }
    }

    let mut symbol_names = Vec::new();
    collect_names(symbols, &mut symbol_names);

    assert!(symbol_names.contains(&"TodoApp"));
    assert!(symbol_names.contains(&"SharedScreen"));
    assert!(symbol_names.contains(&"TodoRow"));
    assert!(symbol_names.contains(&"Item"));
    assert!(symbol_names.contains(&"Priority"));
    assert!(symbol_names.contains(&"items_count"));
    assert!(symbol_names.contains(&"refresh"));
}

#[test]
fn lsp_returns_semantic_tokens_for_open_documents() {
    let mut server = LspServer::new();
    server.open_or_change_document(
        "file:///counter.nx".to_string(),
        "app Counter {\n state count: Int32 = 0\n body { Text(count) }\n}\n".to_string(),
    );
    let (response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(7)),
        method: "textDocument/semanticTokens/full".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///counter.nx" }
        })),
    });
    let result = response
        .expect("semantic tokens request returns a response")
        .result
        .expect("semantic token result");
    let encoded = result["data"].as_array().expect("semantic token data");
    assert_eq!(encoded.len() % 5, 0);
    assert!(!encoded.is_empty());
}

#[test]
fn lsp_resolves_definitions_and_references_across_open_documents() {
    let mut server = LspServer::new();
    server.open_or_change_document(
        "file:///app.nx".to_string(),
        "import \"Row.nx\"\napp Tasks { body { TodoRow() } }\n".to_string(),
    );
    server.open_or_change_document(
        "file:///Row.nx".to_string(),
        "component TodoRow() { body { Text(\"TodoRow\") } }\n".to_string(),
    );

    let (definition_response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(8)),
        method: "textDocument/definition".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///app.nx" },
            "position": { "line": 1, "character": 25 }
        })),
    });
    let location = definition_response
        .expect("definition response")
        .result
        .expect("definition location");
    assert_eq!(location["uri"], "file:///Row.nx");

    let (references_response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(9)),
        method: "textDocument/references".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///app.nx" },
            "position": { "line": 1, "character": 25 },
            "context": { "includeDeclaration": true }
        })),
    });
    let locations = references_response
        .expect("references response")
        .result
        .expect("references list");
    assert_eq!(locations.as_array().expect("location array").len(), 2);
}

#[test]
fn lsp_formats_an_open_document_with_one_full_range_edit() {
    let mut server = LspServer::new();
    server.open_or_change_document(
        "file:///counter.nx".to_string(),
        "app Counter{state count:Int32=0\nbody{Text(count)}}\n".to_string(),
    );
    let (response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(10)),
        method: "textDocument/formatting".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///counter.nx" },
            "options": { "tabSize": 4, "insertSpaces": true }
        })),
    });
    let edits = response
        .expect("formatting response")
        .result
        .expect("formatting edits");
    assert_eq!(edits.as_array().expect("edit array").len(), 1);
    assert_eq!(
        edits[0]["newText"],
        "app Counter {\n    state count: Int32 = 0\n    body {\n        Text(count)\n    }\n}\n"
    );
}

#[test]
fn lsp_provides_signature_help_for_user_functions() {
    let mut server = LspServer::new();
    server.open_or_change_document(
        "file:///app.nx".to_string(),
        "fn loadUser(id: String, includePrivate: Bool) -> String { return id }\napp A { body { Text(loadUser(\"u1\", true)) } }\n"
            .to_string(),
    );
    let (response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(11)),
        method: "textDocument/signatureHelp".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///app.nx" },
            "position": { "line": 1, "character": 37 }
        })),
    });
    let help = response
        .expect("signature help response")
        .result
        .expect("signature result");
    assert_eq!(help["activeParameter"], 1);
    assert!(
        help["signatures"][0]["label"]
            .as_str()
            .expect("signature label")
            .contains("loadUser(id: String, includePrivate: Bool)")
    );
}

#[test]
fn lsp_offers_a_quick_fix_for_unhandled_throwing_action_calls() {
    let mut server = LspServer::new();
    server.open_or_change_document(
        "file:///app.nx".to_string(),
        "app Demo {\n    body {\n        Button(\"Save\") {\n            await storage.save()\n        }\n    }\n}\n".to_string(),
    );
    let (response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(12)),
        method: "textDocument/codeAction".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///app.nx" },
            "range": {
                "start": { "line": 3, "character": 12 },
                "end": { "line": 3, "character": 31 }
            },
            "context": {
                "diagnostics": [{
                    "message": "plugin API `Storage.save` may throw; wrap the call in a `try { ... } catch { ... }` action to handle its failure",
                    "range": {
                        "start": { "line": 3, "character": 18 },
                        "end": { "line": 3, "character": 30 }
                    }
                }]
            }
        })),
    });
    let actions = response
        .expect("code action response")
        .result
        .expect("code actions");
    let action = actions
        .as_array()
        .and_then(|actions| actions.first())
        .expect("try/catch quick fix");
    assert_eq!(action["title"], "Wrap in try/catch");
    assert!(
        action["edit"]["changes"]["file:///app.nx"][0]["newText"]
            .as_str()
            .expect("replacement source")
            .contains("} catch {")
    );
}

#[test]
fn lsp_adds_missing_arguments_from_an_open_component_document() {
    let mut server = LspServer::new();
    server.open_or_change_document(
        "file:///Card.nx".to_string(),
        "component Card(title: String, count: Int32) { body { Text(title) } }\n".to_string(),
    );
    let source = "import \"Card.nx\"\napp Demo { body { Card(title: \"Inbox\") } }\n";
    server.open_or_change_document("file:///app.nx".to_string(), source.to_string());
    let offset = source.find("Card(title").expect("component call");
    let position = LineIndex::new(source).to_position(source, offset);
    let (response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(13)),
        method: "textDocument/codeAction".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///app.nx" },
            "range": { "start": position, "end": position },
            "context": {
                "diagnostics": [{
                    "message": "component `Card` expects 2 named argument(s), got 1",
                    "range": { "start": position, "end": position }
                }]
            }
        })),
    });
    let actions = response
        .expect("code action response")
        .result
        .expect("code actions");
    let action = actions
        .as_array()
        .and_then(|actions| actions.first())
        .expect("missing component argument quick fix");
    assert_eq!(action["title"], "Add missing required component arguments");
    assert_eq!(
        action["edit"]["changes"]["file:///app.nx"][0]["newText"],
        ", count: 0"
    );
}

#[test]
fn lsp_imports_a_configured_plugin_for_an_unknown_native_api() {
    let project = nexa_testkit::TestProject::new("nexa-lsp-plugin-import-integration");
    let source = "app Demo { body { SQLite.open() } }\n";
    let app = project.write_app(source);
    project.write_config(
        r#"config {
            dependencies {
                SQLite { id: "dev.nexa.sqlite", path: "plugins/sqlite" }
            }
        }"#,
    );
    let uri = format!("file://{}", app.display());
    let mut server = LspServer::new();
    let diagnostics = server.open_or_change_document(uri.clone(), source.to_owned());
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "unknown native component `SQLite.open`"),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let offset = source.find("SQLite.open").expect("unknown plugin API");
    let position = LineIndex::new(source).to_position(source, offset);
    let diagnostic_context = serde_json::to_value(&diagnostics).expect("serialize diagnostics");
    let (response, _) = server.handle_request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(14)),
        method: "textDocument/codeAction".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "range": { "start": position, "end": position },
            "context": { "diagnostics": diagnostic_context }
        })),
    });
    let actions = response
        .expect("code action response")
        .result
        .expect("code actions");
    let action = actions
        .as_array()
        .and_then(|actions| actions.first())
        .expect("plugin import quick fix");
    assert_eq!(action["title"], "Import plugin SQLite as SQLite");
    assert_eq!(
        action["edit"]["changes"][uri][0]["newText"],
        "plugin \"dev.nexa.sqlite\" as SQLite\n"
    );
}
