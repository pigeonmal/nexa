//! Parity exhaustiveness gate for IR member access kinds.
//!
//! The inventory is derived from `nexa_ir::MemberKind`, so a new member kind
//! fails this gate until both DevRuntime evaluators handle it. Value-level
//! evaluation is covered separately by the conformance suite.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn enum_variants(source: &str, declaration: &str) -> BTreeSet<String> {
    let start = source
        .find(declaration)
        .unwrap_or_else(|| panic!("IR source defines {declaration}"));
    let block_start = source[start..].find('{').expect("enum body") + start;
    let mut depth = 0i32;
    let mut end = block_start;
    for (offset, character) in source[block_start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = block_start + offset;
                    break;
                }
            }
            _ => {}
        }
    }
    let mut variants = BTreeSet::new();
    for line in source[block_start..=end].lines().skip(1) {
        let trimmed = line.trim_start();
        if !line.starts_with("    ") || trimmed.starts_with("//") {
            continue;
        }
        let identifier: String = trimmed
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if identifier.is_empty()
            || !identifier.starts_with(|character: char| character.is_ascii_uppercase())
        {
            continue;
        }
        let rest = trimmed[identifier.len()..].trim_start();
        if rest.starts_with('{') || rest.starts_with('(') || rest.starts_with(',') {
            variants.insert(identifier);
        }
    }
    variants
}

#[test]
fn member_kind_inventory_is_derived_from_the_ir() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let variants = enum_variants(&ir, "pub enum MemberKind {");
    assert!(
        variants.len() >= 14,
        "MemberKind variant parser found only {} variants: {variants:#?}",
        variants.len()
    );
    assert!(variants.contains("ResultValue"));
}

#[test]
fn every_member_kind_has_evaluation_support_in_both_dev_runtimes() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let kinds = enum_variants(&ir, "pub enum MemberKind {");

    let ios_state = fs::read_to_string(root.join("runtime/ios/NexaDevState.swift"))
        .expect("read NexaDevState.swift");
    let android_state = fs::read_to_string(root.join("runtime/android/NexaDevState.kt"))
        .expect("read NexaDevState.kt");

    let mut missing_ios = Vec::new();
    let mut missing_android = Vec::new();
    for kind in &kinds {
        let ios_has = ios_state.contains(&format!("\"{kind}\""))
            || match kind.as_str() {
                "TupleIndex" => {
                    ios_state.contains("pair[position]") || ios_state.contains("\"first\"")
                }
                "StructField" => {
                    ios_state.contains("object[name]") || ios_state.contains("row.fields")
                }
                "ClassField" => ios_state.contains("instance.fields"),
                "CollectionCount" => ios_state.contains("name == \"count\""),
                "CollectionIsEmpty" => ios_state.contains("name == \"isEmpty\""),
                "StringTrimmed" => ios_state.contains("name == \"trimmed\""),
                "SignalValue" => ios_state.contains("\"value\""),
                _ => false,
            };
        if !ios_has {
            missing_ios.push(kind.clone());
        }

        let android_has = android_state.contains(&format!("\"{kind}\""))
            || match kind.as_str() {
                "TupleIndex" => {
                    android_state.contains("getOrNull(0)") || android_state.contains("\"first\"")
                }
                "StructField" => {
                    android_state.contains("base[name]")
                        || android_state.contains("row.fields")
                        || android_state.contains("base.opt(name)")
                }
                "ClassField" => android_state.contains("instance.fields"),
                "CollectionCount" => {
                    android_state.contains("name == \"count\"")
                        || android_state.contains("\"count\" ->")
                }
                "CollectionIsEmpty" => {
                    android_state.contains("name == \"isEmpty\"")
                        || android_state.contains("\"isEmpty\" ->")
                }
                "StringTrimmed" => {
                    android_state.contains("name == \"trimmed\"")
                        || android_state.contains("\"trimmed\" ->")
                }
                "SignalValue" => android_state.contains("\"value\""),
                _ => false,
            };
        if !android_has {
            missing_android.push(kind.clone());
        }
    }

    assert!(
        missing_ios.is_empty(),
        "iOS DevRuntime is missing MemberKind support: {missing_ios:#?}"
    );
    assert!(
        missing_android.is_empty(),
        "Android DevRuntime is missing MemberKind support: {missing_android:#?}"
    );
}
