use std::{collections::BTreeSet, fs, path::PathBuf};

use serde_json::Value;

fn variants(source: &str, enum_name: &str) -> BTreeSet<String> {
    let marker = format!("pub enum {enum_name} {{");
    let body = source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("IR enum {enum_name} was not found"))
        .1
        .split_once("\n}")
        .unwrap_or_else(|| panic!("IR enum {enum_name} has no closing brace"))
        .0;
    body.lines()
        .filter_map(|line| {
            let line = line.strip_prefix("    ")?;
            if line.starts_with(' ') {
                return None;
            }
            let name = line
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

fn public_enums(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("pub enum "))
        .filter_map(|declaration| declaration.split_once(' ').map(|(name, _)| name))
        .map(str::to_owned)
        .collect()
}

/// Plugin values and bindings call AOT host code, so their tested path is `b`.
fn requires_native_rebuild(enum_name: &str, variant_name: &str) -> bool {
    matches!(
        (enum_name, variant_name),
        ("Type", "Plugin")
            | ("Node", "NativeComponentCall")
            | ("Action", "NativePropertyAssign" | "NativeEventSubscribe")
    )
}

fn fixture() -> (PathBuf, Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = root.join("tests/fixtures/hot_reload_coverage.json");
    let fixture: Value = serde_json::from_str(
        &fs::read_to_string(&fixture_path).expect("read hot-reload coverage inventory"),
    )
    .expect("parse hot-reload coverage inventory");
    (root, fixture)
}

fn assert_runtime_dispatch(runtime: &str, enum_name: &str, variant: &str, platform: &str) {
    let marker = format!("\"{variant}\"");
    assert!(
        runtime.contains(&marker),
        "{platform} dev runtime has no dispatch for {enum_name}::{variant}"
    );
}

#[test]
fn inventory_tracks_every_public_ir_node_and_interpreter_variant() {
    let (root, fixture) = fixture();
    let source_path = root.join(fixture["source"].as_str().expect("source path"));
    let source = fs::read_to_string(source_path).expect("read typed IR declarations");
    let inventory = fixture["ir_variants"]
        .as_object()
        .expect("IR inventory object");
    let require_coverage = std::env::var_os("NEXA_REQUIRE_HOT_RELOAD_COVERAGE").is_some();

    let inventoried_enums = inventory.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(inventoried_enums, public_enums(&source));

    for (enum_name, entries) in inventory {
        let expected = variants(&source, enum_name);
        let entries = entries
            .as_array()
            .unwrap_or_else(|| panic!("invalid {enum_name} coverage entries"));
        let actual = entries
            .iter()
            .map(|entry| {
                for platform in ["ios", "android"] {
                    let status = entry[platform].as_str();
                    assert!(
                        matches!(status, Some("pending" | "covered" | "native_rebuild")),
                        "{enum_name}::{} needs an explicit {platform} coverage status",
                        entry["name"]
                    );
                    let name = entry["name"].as_str().expect("variant name");
                    assert_eq!(
                        status == Some("native_rebuild"),
                        requires_native_rebuild(enum_name, name),
                        "{enum_name}::{name} has the wrong hot-reload boundary for {platform}"
                    );
                    if require_coverage {
                        assert_ne!(
                            status,
                            Some("pending"),
                            "{enum_name}::{} has no {platform} coverage evidence",
                            entry["name"]
                        );
                    }
                }
                entry["name"].as_str().expect("variant name").to_owned()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{enum_name} coverage inventory is stale");
    }
}

#[test]
fn hot_reload_interpreter_variants_have_both_native_dispatches() {
    let (root, fixture) = fixture();
    let inventory = fixture["ir_variants"]
        .as_object()
        .expect("IR inventory object");
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRuntime.swift"))
        .expect("read iOS dev runtime");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRuntime.kt"))
        .expect("read Android dev runtime");

    // These IR enums are consumed by the hot-reload interpreters. Metadata-only
    // node variants are handled by the module/store lifecycle; a native plugin
    // component is intentionally on the rebuild path.
    let dispatched_enums = ["Expr", "Action", "CollectionMutation"];
    for enum_name in dispatched_enums {
        for entry in inventory[enum_name]
            .as_array()
            .unwrap_or_else(|| panic!("missing {enum_name} inventory"))
        {
            let variant = entry["name"].as_str().expect("variant name");
            if requires_native_rebuild(enum_name, variant) {
                continue;
            }
            assert_runtime_dispatch(&swift, enum_name, variant, "iOS");
            assert_runtime_dispatch(&kotlin, enum_name, variant, "Android");
        }
    }

    let node_metadata = [
        "StatusBar",
        "Direction",
        "OnAppear",
        "OnDisappear",
        "OnActive",
        "OnInactive",
        "OnBackground",
        "NativeComponentCall",
    ];
    for entry in inventory["Node"].as_array().expect("Node inventory") {
        let variant = entry["name"].as_str().expect("variant name");
        if node_metadata.contains(&variant) {
            continue;
        }
        assert_runtime_dispatch(&swift, "Node", variant, "iOS");
        assert_runtime_dispatch(&kotlin, "Node", variant, "Android");
    }
}

#[test]
fn runtime_acceptance_scenarios_have_explicit_dual_platform_status() {
    let (_, fixture) = fixture();
    let scenarios = fixture["runtime_scenarios"]
        .as_object()
        .expect("runtime scenario inventory");
    let require_coverage = std::env::var_os("NEXA_REQUIRE_HOT_RELOAD_COVERAGE").is_some();
    for (name, status) in scenarios {
        for platform in ["ios", "android"] {
            let value = status[platform].as_str();
            assert!(
                matches!(value, Some("pending" | "covered")),
                "scenario {name} needs an explicit {platform} test status"
            );
            if require_coverage {
                assert_eq!(
                    value,
                    Some("covered"),
                    "scenario {name} is not covered on {platform}"
                );
            }
        }
    }
}

#[test]
fn runtime_feature_gaps_have_explicit_dual_platform_status() {
    let (_, fixture) = fixture();
    let features = fixture["runtime_features"]
        .as_object()
        .expect("runtime feature inventory");
    let require_coverage = std::env::var_os("NEXA_REQUIRE_HOT_RELOAD_COVERAGE").is_some();
    for (name, status) in features {
        for platform in ["ios", "android"] {
            let value = status[platform].as_str();
            assert!(
                matches!(value, Some("pending" | "covered")),
                "runtime feature {name} needs an explicit {platform} test status"
            );
            if require_coverage {
                assert_eq!(
                    value,
                    Some("covered"),
                    "runtime feature {name} is not covered on {platform}"
                );
            }
        }
    }
}
