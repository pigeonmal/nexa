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
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/completion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": "file:///test.nx" },
            "position": { "line": 0, "character": 0 }
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
    assert!(labels.contains(&"Result"));
    assert!(labels.contains(&"state"));
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
        "Spacer",
        "Divider",
        "VStack",
        "HStack",
        "ZStack",
    ] {
        let source = format!("app P {{\n    body {{\n        {name}()\n    }}\n}}\n");
        let diagnostics = nexa_lsp::check_source(&source);
        assert!(!diagnostics.is_empty(), "{name} should not compile cleanly");
    }
    // The supported spelling compiles cleanly.
    let diagnostics = nexa_lsp::check_source(
        "app P {\n    state name: String = \"\"\n    body {\n        TextInput(value: name, placeholder: \"Name\")\n    }\n}\n",
    );
    assert!(diagnostics.is_empty());
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

    for supported in ["Column", "TextInput", "FastList", "Pressable"] {
        assert!(labels.contains(&supported), "{supported} should be offered");
    }
    assert!(
        !labels.contains(&"onPress"),
        "dot modifiers need member position"
    );
    for rejected in [
        "TextField",
        "FastSectionedList",
        "Spacer",
        "Divider",
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
        Text("Done")
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
    assert!(symbol_names.contains(&"TodoRow"));
    assert!(symbol_names.contains(&"Item"));
    assert!(symbol_names.contains(&"Priority"));
    assert!(symbol_names.contains(&"items_count"));
    assert!(symbol_names.contains(&"refresh"));
}
