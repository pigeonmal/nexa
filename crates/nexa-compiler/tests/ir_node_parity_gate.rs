//! Parity exhaustiveness gate for IR view nodes.
//!
//! The variant inventory is derived from `nexa_ir::Node` itself, not from a
//! hand-maintained list: adding a variant must fail this gate until both
//! DevRuntime renderers dispatch it. Lifecycle and screen-chrome variants are
//! listed explicitly with the reason they never reach node rendering.
//!
//! The gate reads the *dispatch block* of each renderer rather than the whole
//! file, so a variant name mentioned in a comment or an unrelated string cannot
//! satisfy it. The mutation tests below prove that removing one dispatch label
//! fails the gate.

use std::collections::{BTreeSet, HashMap};
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

/// Nodes the compiler resolves before rendering.
///
/// - `OnAppear`/`OnDisappear`/`OnActive`/`OnInactive`/`OnBackground` become
///   screen/app lifecycle actions (`Screen::on_appear`, `Module::on_active`, …).
/// - `StatusBar`/`Direction` become screen chrome configuration applied by the
///   native host (`NexaDevRuntime`), not body nodes.
const RESOLVED_BEFORE_RENDER: &[(&str, &str)] = &[
    ("OnAppear", "lowered to Screen::on_appear"),
    ("OnDisappear", "lowered to Screen::on_disappear"),
    ("OnActive", "lowered to lifecycle actions"),
    ("OnInactive", "lowered to lifecycle actions"),
    ("OnBackground", "lowered to lifecycle actions"),
    ("StatusBar", "lowered to Screen::status_bar chrome"),
    ("Direction", "lowered to Screen::direction chrome"),
];

fn node_variants(ir_source: &str) -> BTreeSet<String> {
    let enum_start = ir_source
        .find("pub enum Node {")
        .expect("nexa-ir defines `pub enum Node`");
    let body = brace_block(&ir_source[enum_start..]).expect("Node enum has a balanced body");
    let mut variants = BTreeSet::new();
    for line in body.lines().skip(1) {
        let trimmed = line.trim_start();
        if !line.starts_with("    ") || trimmed.starts_with("///") || trimmed.starts_with("//") {
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
        let rest = &trimmed[identifier.len()..];
        let rest = rest.trim_start();
        if rest.starts_with('{') || rest.starts_with('(') || rest.starts_with(',') {
            variants.insert(identifier);
        }
    }
    variants
}

/// Returns the source from `{` to its matching `}`, inclusive.
fn brace_block(source: &str) -> Option<&str> {
    let start = source.find('{')?;
    let mut depth = 0i32;
    for (offset, character) in source[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[start..=start + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The body of the main node-dispatch switch/`when` for one renderer.
fn ios_dispatch_block(renderer: &str) -> String {
    let start = renderer
        .find("private func renderNodeContent(")
        .expect("iOS renderer has renderNodeContent");
    let function = brace_block(&renderer[start..]).expect("balanced renderNodeContent body");
    function.to_owned()
}

fn android_dispatch_block(renderer: &str) -> String {
    let start = renderer
        .find("internal fun NexaDevNode(")
        .expect("Android renderer has NexaDevNode");
    let function = brace_block(&renderer[start..]).expect("balanced NexaDevNode body");
    let switch_start = function
        .find("when (kind) {")
        .expect("NexaDevNode dispatches on `kind`");
    brace_block(&function[switch_start..])
        .expect("balanced node when block")
        .to_owned()
}

fn ios_labels(block: &str, schema: &str) -> BTreeSet<String> {
    let constants = schema_constants(schema, "NexaDevNodeKind");
    let mut labels = BTreeSet::new();
    for line in block.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("case ") else {
            continue;
        };
        if let Some(literal) = rest.strip_prefix('"')
            && let Some(end) = literal.find('"')
        {
            labels.insert(literal[..end].to_owned());
        } else if let Some(constant) = rest.strip_prefix("NexaDevNodeKind.") {
            let name: String = constant
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if let Some(value) = constants.get(name.as_str()) {
                labels.insert(value.clone());
            }
        }
    }
    labels
}

fn android_labels(block: &str, schema: &str) -> BTreeSet<String> {
    let constants = schema_constants(schema, "NexaDevNodeKind");
    let mut labels = BTreeSet::new();
    let mut branch_start = 0usize;
    for (arrow, _) in block.match_indices("->") {
        let branch = &block[branch_start..arrow];
        collect_string_literals(branch, &mut labels);
        for (name, value) in &constants {
            if branch.contains(&format!("NexaDevNodeKind.{name}")) {
                labels.insert(value.clone());
            }
        }
        branch_start = arrow + 2;
    }
    labels
}

fn collect_string_literals(text: &str, into: &mut BTreeSet<String>) {
    let mut rest = text;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else { break };
        let literal = &after[..end];
        if !literal.is_empty()
            && literal
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            into.insert(literal.to_owned());
        }
        rest = &after[end + 1..];
    }
}

fn schema_constants(schema: &str, enum_name: &str) -> HashMap<String, String> {
    let start = ["public enum ", "internal object ", "object "]
        .into_iter()
        .find_map(|prefix| schema.find(&format!("{prefix}{enum_name} {{")))
        .unwrap_or_else(|| panic!("schema defines {enum_name}"));
    let body = brace_block(&schema[start..]).expect("balanced schema enum");
    let mut constants = HashMap::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        let rest = if let Some(rest) = trimmed.strip_prefix("public static let ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("const val ") {
            rest
        } else {
            continue;
        };
        let Some((name, value)) = rest.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if let Some(value) = value.strip_prefix('"')
            && let Some(end) = value.find('"')
        {
            constants.insert(name.to_owned(), value[..end].to_owned());
        }
    }
    constants
}

struct Inventory<'a> {
    variants: BTreeSet<String>,
    resolved: BTreeSet<&'a str>,
}

fn inventory<'a>(ir_source: &str, resolved: &'a [(&'a str, &'a str)]) -> Inventory<'a> {
    let variants = node_variants(ir_source);
    let resolved_names = resolved.iter().map(|(name, _)| *name).collect();
    Inventory {
        variants,
        resolved: resolved_names,
    }
}

impl Inventory<'_> {
    fn required(&self) -> BTreeSet<String> {
        self.variants
            .difference(&self.resolved.iter().map(|s| s.to_string()).collect())
            .cloned()
            .collect()
    }

    fn missing(&self, labels: &BTreeSet<String>) -> Vec<String> {
        self.required()
            .difference(labels)
            .cloned()
            .collect::<Vec<_>>()
    }
}

fn assert_stale_resolved_names(inventory: &Inventory<'_>) {
    let stale = inventory
        .resolved
        .iter()
        .filter(|name| !inventory.variants.contains(**name))
        .copied()
        .collect::<Vec<_>>();
    assert!(
        stale.is_empty(),
        "RESOLVED_BEFORE_RENDER names no longer exist in nexa_ir::Node: {stale:#?}"
    );
}

#[test]
fn node_inventory_is_derived_from_the_ir() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let inventory = inventory(&ir, RESOLVED_BEFORE_RENDER);
    assert!(
        inventory.variants.len() >= 50,
        "Node variant parser found only {} variants: {:#?}",
        inventory.variants.len(),
        inventory.variants
    );
    assert!(inventory.variants.contains("NavigationStack"));
    assert!(inventory.variants.contains("NativeComponentCall"));
    assert_stale_resolved_names(&inventory);
}

#[test]
fn every_ir_node_has_renderer_dispatch_in_both_dev_runtimes() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let inventory = inventory(&ir, RESOLVED_BEFORE_RENDER);
    assert_stale_resolved_names(&inventory);

    let schema =
        fs::read_to_string(root.join("runtime/ios/NexaDevSchema.swift")).expect("read iOS schema");
    let ios_renderer = fs::read_to_string(root.join("runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let ios_block = ios_dispatch_block(&ios_renderer);
    let ios_labels = ios_labels(&ios_block, &schema);
    assert!(
        ios_labels.len() >= 40,
        "iOS dispatch parser found only {} labels",
        ios_labels.len()
    );
    let missing_ios = inventory.missing(&ios_labels);
    assert!(
        missing_ios.is_empty(),
        "iOS DevRuntime renderer is missing dispatch for IR nodes: {missing_ios:#?}"
    );

    let android_schema = fs::read_to_string(root.join("runtime/android/NexaDevSchema.kt"))
        .expect("read Android schema");
    let android_renderer = fs::read_to_string(root.join("runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");
    let android_block = android_dispatch_block(&android_renderer);
    let android_labels = android_labels(&android_block, &android_schema);
    assert!(
        android_labels.len() >= 40,
        "Android dispatch parser found only {} labels",
        android_labels.len()
    );
    let missing_android = inventory.missing(&android_labels);
    assert!(
        missing_android.is_empty(),
        "Android DevRuntime renderer is missing dispatch for IR nodes: {missing_android:#?}"
    );
}

#[test]
fn removing_an_ios_dispatch_label_fails_the_gate() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let inventory = inventory(&ir, RESOLVED_BEFORE_RENDER);
    let schema =
        fs::read_to_string(root.join("runtime/ios/NexaDevSchema.swift")).expect("read iOS schema");
    let renderer = fs::read_to_string(root.join("runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");

    let mutated = renderer.replacen("case \"NavigationStack\":", "case \"__removed__\":", 1);
    assert_ne!(mutated, renderer, "fixture must contain the dispatch case");
    let labels = ios_labels(&ios_dispatch_block(&mutated), &schema);
    assert!(
        inventory
            .missing(&labels)
            .contains(&"NavigationStack".to_owned()),
        "removing the NavigationStack case must make the gate fail"
    );
}

#[test]
fn removing_an_android_dispatch_label_fails_the_gate() {
    let root = workspace_root();
    let ir = fs::read_to_string(root.join("crates/nexa-ir/src/lib.rs")).expect("read IR source");
    let inventory = inventory(&ir, RESOLVED_BEFORE_RENDER);
    let schema = fs::read_to_string(root.join("runtime/android/NexaDevSchema.kt"))
        .expect("read Android schema");

    let renderer = fs::read_to_string(root.join("runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");
    let original = android_dispatch_block(&renderer);
    let mutated = original.replacen("\"NavigationStack\", ", "", 1);
    assert_ne!(mutated, original, "fixture must contain the dispatch label");
    let labels = android_labels(&mutated, &schema);
    assert!(
        inventory
            .missing(&labels)
            .contains(&"NavigationStack".to_owned()),
        "removing the NavigationStack label must make the gate fail"
    );
}
