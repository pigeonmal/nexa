use std::fs;
use std::path::Path;

#[test]
fn dev_ir_schema_fixture_matches_rust_protocol_and_native_runtimes() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../..");

    let schema_path = root.join("tests/fixtures/dev_ir_schema.json");
    assert!(schema_path.exists(), "dev_ir_schema.json must exist");

    let schema_text = fs::read_to_string(&schema_path).expect("read dev_ir_schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&schema_text).expect("parse dev_ir_schema.json");

    // 1. Verify Rust protocol constants match schema
    let protocol_version = schema["protocol_version"]
        .as_u64()
        .expect("protocol_version in schema") as u16;
    assert_eq!(
        protocol_version,
        nexa_dev_protocol::PROTOCOL_VERSION,
        "schema protocol_version matches Rust PROTOCOL_VERSION"
    );

    let format_version = schema["dev_ir_format_version"]
        .as_u64()
        .expect("dev_ir_format_version in schema") as u16;
    assert_eq!(
        format_version,
        nexa_dev_ir::DEV_IR_FORMAT_VERSION,
        "schema dev_ir_format_version matches Rust DEV_IR_FORMAT_VERSION"
    );

    // 2. Verify platforms, message kinds, and patch operations
    let platforms = schema["platforms"]
        .as_array()
        .expect("platforms array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(platforms.contains(&"ios".to_string()));
    assert!(platforms.contains(&"android".to_string()));

    let client_messages = schema["client_messages"]
        .as_array()
        .expect("client_messages array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(client_messages.contains(&"hello".to_string()));
    assert!(client_messages.contains(&"acknowledge".to_string()));
    assert!(client_messages.contains(&"request_full_module".to_string()));
    assert!(client_messages.contains(&"disconnect".to_string()));

    let server_messages = schema["server_messages"]
        .as_array()
        .expect("server_messages array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(server_messages.contains(&"welcome".to_string()));
    assert!(server_messages.contains(&"full_module".to_string()));
    assert!(server_messages.contains(&"patch".to_string()));
    assert!(server_messages.contains(&"diagnostics".to_string()));
    assert!(server_messages.contains(&"reload".to_string()));
    assert!(server_messages.contains(&"restart".to_string()));
    assert!(server_messages.contains(&"performance_overlay".to_string()));

    // 3. Verify Swift Dev Runtime Schema matches
    let swift_schema_path = root.join("runtime/ios/NexaDevSchema.swift");
    let swift_code = fs::read_to_string(&swift_schema_path).expect("read NexaDevSchema.swift");

    assert!(
        swift_code.contains(&format!("protocolVersion: Int = {protocol_version}")),
        "Swift schema defines protocolVersion {protocol_version}"
    );
    assert!(
        swift_code.contains(&format!("devIRFormatVersion: Int = {format_version}")),
        "Swift schema defines devIRFormatVersion {format_version}"
    );

    // Check message kinds in Swift
    for msg in &client_messages {
        assert!(
            swift_code.contains(msg),
            "Swift schema contains client message {msg}"
        );
    }
    for msg in &server_messages {
        assert!(
            swift_code.contains(msg),
            "Swift schema contains server message {msg}"
        );
    }

    // 4. Verify Kotlin Dev Runtime Schema matches
    let kotlin_schema_path = root.join("runtime/android/NexaDevSchema.kt");
    let kotlin_code = fs::read_to_string(&kotlin_schema_path).expect("read NexaDevSchema.kt");

    assert!(
        kotlin_code.contains(&format!("PROTOCOL_VERSION: Int = {protocol_version}")),
        "Kotlin schema defines PROTOCOL_VERSION {protocol_version}"
    );
    assert!(
        kotlin_code.contains(&format!("DEV_IR_FORMAT_VERSION: Int = {format_version}")),
        "Kotlin schema defines DEV_IR_FORMAT_VERSION {format_version}"
    );

    // Check message kinds in Kotlin
    for msg in &client_messages {
        assert!(
            kotlin_code.contains(msg),
            "Kotlin schema contains client message {msg}"
        );
    }
    for msg in &server_messages {
        assert!(
            kotlin_code.contains(msg),
            "Kotlin schema contains server message {msg}"
        );
    }

    // 5. Verify standard keys and node kinds are represented in both
    let node_kinds = schema["node_kinds"]
        .as_array()
        .expect("node_kinds array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    for kind in &node_kinds {
        assert!(
            swift_code.contains(kind),
            "Swift schema defines node kind {kind}"
        );
        assert!(
            kotlin_code.contains(kind),
            "Kotlin schema defines node kind {kind}"
        );
    }

    let action_kinds = schema["action_kinds"]
        .as_array()
        .expect("action_kinds array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    for kind in &action_kinds {
        assert!(
            swift_code.contains(kind),
            "Swift schema defines action kind {kind}"
        );
        assert!(
            kotlin_code.contains(kind),
            "Kotlin schema defines action kind {kind}"
        );
    }

    let mutation_kinds = schema["collection_mutation_kinds"]
        .as_array()
        .expect("collection_mutation_kinds array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    for kind in &mutation_kinds {
        assert!(
            swift_code.contains(kind),
            "Swift schema defines mutation kind {kind}"
        );
        assert!(
            kotlin_code.contains(kind),
            "Kotlin schema defines mutation kind {kind}"
        );
    }

    let namespaces = schema["native_namespaces"]
        .as_array()
        .expect("native_namespaces array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    for ns in &namespaces {
        assert!(
            swift_code.contains(ns),
            "Swift schema defines native namespace {ns}"
        );
        assert!(
            kotlin_code.contains(ns),
            "Kotlin schema defines native namespace {ns}"
        );
    }

    // Verify key categories
    let keys_obj = schema["keys"].as_object().expect("keys object");
    for (category, group) in keys_obj {
        let group_obj = group
            .as_object()
            .unwrap_or_else(|| panic!("expected object for group {category}"));
        for (_k, v) in group_obj {
            let key_str = v.as_str().unwrap();
            assert!(
                swift_code.contains(&format!("\"{key_str}\"")),
                "Swift schema defines key \"{key_str}\""
            );
            assert!(
                kotlin_code.contains(&format!("\"{key_str}\"")),
                "Kotlin schema defines key \"{key_str}\""
            );
        }
    }
}
