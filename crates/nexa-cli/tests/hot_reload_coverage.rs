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

/// Returns documented cases that remain outside DevRuntime's compiled host.
fn dev_boundary(enum_name: &str, variant_name: &str) -> Option<&'static str> {
    match (enum_name, variant_name) {
        // Plugin class values can refer only to native types compiled into
        // the host; hot reload cannot load a new native plugin implementation.
        ("Type", "Plugin") => Some("partial"),
        _ => None,
    }
}

fn is_semantically_probed_plugin_variant(enum_name: &str, variant_name: &str) -> bool {
    matches!(
        (enum_name, variant_name),
        ("Node", "NativeComponentCall")
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
                        matches!(
                            status,
                            Some("pending" | "covered" | "partial" | "native_rebuild" | "aot_only")
                        ),
                        "{enum_name}::{} needs an explicit {platform} coverage status",
                        entry["name"]
                    );
                    let name = entry["name"].as_str().expect("variant name");
                    if let Some(boundary) = dev_boundary(enum_name, name) {
                        assert_eq!(
                            status,
                            Some(boundary),
                            "{enum_name}::{name} has the wrong Dev boundary for {platform}"
                        );
                    } else if status == Some("native_rebuild") || status == Some("aot_only") {
                        panic!(
                            "{enum_name}::{name} has an undocumented Dev boundary for {platform}"
                        );
                    }
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
fn shared_element_image_fields_have_dev_renderers_on_both_platforms() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS dev renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android dev renderer");
    let swift_root = fs::read_to_string(root.join("../../runtime/ios/NexaDevRuntime.swift"))
        .expect("read iOS dev root");
    let kotlin_root = fs::read_to_string(root.join("../../runtime/android/NexaDevRuntime.kt"))
        .expect("read Android dev root");

    assert!(swift.contains("fields[\"shared_element\"]"));
    assert!(swift.contains("matchedGeometryEffect(id: id, in: namespace)"));
    assert!(swift_root.contains(".environment(\\.nexaSharedNamespace, nexaSharedNamespace)"));
    assert!(kotlin.contains("fields.opt(\"shared_element\")"));
    assert!(kotlin.contains("nexaSharedElementModifier("));
    assert!(kotlin_root.contains("NexaSharedTransitionContent {"));
}

#[test]
fn hot_reload_interpreter_variants_have_both_native_dispatches() {
    let (root, fixture) = fixture();
    let inventory = fixture["ir_variants"]
        .as_object()
        .expect("IR inventory object");
    let swift_dir = root.join("../../runtime/ios");
    let swift = fs::read_dir(&swift_dir)
        .expect("read iOS runtime directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "swift"))
        .map(|entry| fs::read_to_string(entry.path()).expect("read swift runtime file"))
        .collect::<Vec<_>>()
        .join("\n");
    let kotlin_dir = root.join("../../runtime/android");
    let kotlin = fs::read_dir(&kotlin_dir)
        .expect("read Android runtime directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "kt"))
        .map(|entry| fs::read_to_string(entry.path()).expect("read kotlin runtime file"))
        .collect::<Vec<_>>()
        .join("\n");

    // This is an inventory guard: it checks for dispatch markers, not semantic
    // parity or successful execution of every variant. Device acceptance needs
    // separate runtime tests.
    let dispatched_enums = ["Expr", "Action", "CollectionMutation"];
    for enum_name in dispatched_enums {
        for entry in inventory[enum_name]
            .as_array()
            .unwrap_or_else(|| panic!("missing {enum_name} inventory"))
        {
            let variant = entry["name"].as_str().expect("variant name");
            if dev_boundary(enum_name, variant).is_some()
                || is_semantically_probed_plugin_variant(enum_name, variant)
            {
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
    ];
    for entry in inventory["Node"].as_array().expect("Node inventory") {
        let variant = entry["name"].as_str().expect("variant name");
        if node_metadata.contains(&variant)
            || dev_boundary("Node", variant).is_some()
            || is_semantically_probed_plugin_variant("Node", variant)
        {
            continue;
        }
        assert_runtime_dispatch(&swift, "Node", variant, "iOS");
        assert_runtime_dispatch(&kotlin, "Node", variant, "Android");
    }
}

#[test]
fn module_initializers_resolve_prior_values_during_install_and_hot_restart() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS dev state evaluator");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android dev state evaluator");

    assert!(swift.contains("evaluate(initial, locals: initialLocals, scope: scope)"));
    assert!(swift.contains("evaluate(initial, locals: initialLocals, scope: \"app\")"));
    assert!(kotlin.contains("evaluate(declaration.get(\"initial\"), initialLocals, scope)"));
    assert!(kotlin.contains("evaluate(state.get(\"initial\"), initialLocals, \"app\")"));
}

#[test]
fn async_member_access_handles_tuple_positions_and_collection_size() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS state evaluator");
    let swift_async = swift
        .split_once("func evaluateAsync(")
        .expect("iOS async evaluator")
        .1;
    let swift_member = swift_async
        .split_once("case \"Member\":")
        .expect("iOS async Member evaluator")
        .1
        .split_once("case \"Array\", \"Set\":")
        .expect("iOS async Array evaluator")
        .0;
    for marker in [
        "case \"first\": 0",
        "case \"second\": 1",
        "case \"third\": 2",
        "name == \"count\"",
        "name == \"isEmpty\"",
    ] {
        assert!(
            swift_member.contains(marker),
            "iOS async Member evaluator is missing {marker}"
        );
    }

    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android state evaluator");
    let kotlin_async = kotlin
        .split_once("suspend fun evaluateAsync(")
        .expect("Android async evaluator")
        .1;
    let kotlin_member = kotlin_async
        .split_once("\"Member\" -> {")
        .expect("Android async Member evaluator")
        .1
        .split_once("\"Array\", \"Set\" -> {")
        .expect("Android async Array evaluator")
        .0;
    for marker in [
        "\"first\" -> base.getOrNull(0)",
        "\"second\" -> base.getOrNull(1)",
        "\"third\" -> base.getOrNull(2)",
        "\"count\" ->",
        "\"isEmpty\" ->",
    ] {
        assert!(
            kotlin_member.contains(marker),
            "Android async Member evaluator is missing {marker}"
        );
    }
}

#[test]
fn typed_plugin_failures_reach_sync_and_async_catch_arms() {
    let (root, _) = fixture();
    let swift_actions = fs::read_to_string(root.join("../../runtime/ios/NexaDevActions.swift"))
        .expect("read iOS action evaluator");
    let kotlin_actions = fs::read_to_string(root.join("../../runtime/android/NexaDevActions.kt"))
        .expect("read Android action evaluator");
    let swift_native = fs::read_to_string(root.join("../../runtime/ios/NexaDevNativeApis.swift"))
        .expect("read iOS native adapters");
    let kotlin_native = fs::read_to_string(root.join("../../runtime/android/NexaDevNativeApis.kt"))
        .expect("read Android native adapters");

    for (platform, source) in [
        ("iOS", swift_actions.as_str()),
        ("Android", kotlin_actions.as_str()),
    ] {
        for marker in [
            "performPluginFailureCatch(",
            "performPluginFailureCatchAsync(",
            "failure.namespace",
            "failure.errorType",
            "failure.variant",
            "failure.payload[property]",
        ] {
            assert!(
                source.contains(marker),
                "{platform} DevRuntime typed error routing is missing {marker}"
            );
        }
    }
    assert!(swift_native.contains("pendingPluginFailure = failure"));
    assert!(kotlin_native.contains("pendingPluginFailure = failure"));
}

#[test]
fn generic_plugin_result_values_have_both_runtime_codecs() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevValueCodec.swift"))
        .expect("read iOS generic value codec");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevValueCodec.kt"))
        .expect("read Android generic value codec");

    assert!(swift.contains("case \"Result\":"));
    assert!(swift.contains("writer.writeBool(true)"));
    assert!(swift.contains("return [\"Ok\": success]"));
    assert!(swift.contains("return [\"Err\": failure]"));
    assert!(kotlin.contains("\"Result\" -> {"));
    assert!(kotlin.contains("writer.writeBool(true)"));
    assert!(kotlin.contains("mapOf(\"Ok\" to success)"));
    assert!(kotlin.contains("mapOf(\"Err\" to failure)"));
}

#[test]
fn optional_generic_plugin_arguments_have_both_runtime_writers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevValueCodec.swift"))
        .expect("read iOS generic value codec");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevValueCodec.kt"))
        .expect("read Android generic value codec");

    assert!(swift.contains("if kind == \"Optional\""));
    assert!(swift.contains("writer.writeBool(false)"));
    assert!(swift.contains("static func box(_ raw: Any?) -> NexaDevHashableValue?"));
    assert!(kotlin.contains("if (kind == \"Optional\")"));
    assert!(kotlin.contains("writer.writeBool(present)"));
    assert!(kotlin.contains("raw != JSONObject.NULL"));
}

#[test]
fn async_expression_evaluator_recurses_through_nested_values() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevState.swift"))
        .expect("read iOS state evaluator");
    let swift_async = swift
        .split_once("func evaluateAsync(")
        .expect("iOS async evaluator")
        .1;
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevState.kt"))
        .expect("read Android state evaluator");
    let kotlin_async = kotlin
        .split_once("suspend fun evaluateAsync(")
        .expect("Android async evaluator")
        .1;

    for (platform, source, markers) in [
        (
            "iOS",
            swift_async,
            [
                "case \"ResultOk\", \"ResultErr\":",
                "try await evaluateAsync(fields[key]",
                "case \"Array\", \"Set\":",
                "case \"Map\":",
                "case \"Conditional\":",
                "if op == \"And\"",
            ],
        ),
        (
            "Android",
            kotlin_async,
            [
                "\"ResultOk\", \"ResultErr\" ->",
                "evaluateAsync(fields.opt(key)",
                "\"Array\", \"Set\" ->",
                "\"Map\" ->",
                "\"Conditional\" ->",
                "if (op == \"And\"",
            ],
        ),
    ] {
        for marker in markers {
            assert!(
                source.contains(marker),
                "{platform} async expression evaluator is missing {marker}"
            );
        }
    }
}

#[test]
fn async_expression_coverage_requires_log_only_semantic_device_evidence() {
    let (root, fixture) = fixture();
    let feature = &fixture["runtime_features"]["async_expression_evaluation"];
    let script =
        fs::read_to_string(root.join("../../scripts/test-dev-runtime-plugin-hot-reload.sh"))
            .expect("read log-only DevRuntime probe");
    let app = fs::read_to_string(root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.nx"))
        .expect("read initial probe app");
    let template = fs::read_to_string(
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.after.template"),
    )
    .expect("read hot-reload probe app");

    for platform in ["ios", "android"] {
        assert_eq!(
            feature[platform].as_str(),
            Some("covered"),
            "async expression evaluation lacks {platform} device evidence"
        );
    }
    for marker in feature["semantic_probe_markers"]
        .as_array()
        .expect("async expression semantic markers")
    {
        let marker = marker.as_str().expect("string semantic marker");
        assert!(
            script.contains(marker),
            "device probe does not wait for {marker}"
        );
        assert!(app.contains(marker), "initial app does not emit {marker}");
        assert!(
            template.contains(marker),
            "hot-reload app does not emit {marker}"
        );
    }
    assert!(script.contains("DEVRT_ASYNC_NESTED_EXPRESSION_FAIL"));
    assert!(app.contains("DEVRT_ASYNC_NESTED_EXPRESSION_FAIL"));
    assert!(template.contains("DEVRT_ASYNC_NESTED_EXPRESSION_FAIL"));
}

#[test]
fn pressable_double_tap_actions_are_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("fields[\"double_tap_actions\"]"));
    assert!(swift.contains("TapGesture(count: 2)"));
    assert!(swift.contains("store.perform(doubleTapActions"));
    assert!(kotlin.contains("fields.optJSONArray(\"double_tap_actions\")"));
    assert!(kotlin.contains("onDoubleClick = {"));
    assert!(kotlin.contains("store.perform(doubleTapActions"));
}

#[test]
fn pressable_long_press_duration_is_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("fields[NexaDevKeys.longPressDurationMs]"));
    assert!(swift.contains("LongPressGesture(minimumDuration: longPressMinimumDuration)"));
    assert!(kotlin.contains("NexaDevKeys.LONG_PRESS_DURATION_MS"));
    assert!(kotlin.contains("override val longPressTimeoutMillis: Long = longPressDurationMs"));
}

#[test]
fn pressable_pinch_actions_are_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("fields[NexaDevKeys.pinchParameter]"));
    assert!(swift.contains("MagnificationGesture()"));
    assert!(swift.contains("store.perform(pinchActions"));
    assert!(kotlin.contains("fields.optString(NexaDevKeys.PINCH_PARAMETER)"));
    assert!(kotlin.contains("detectTransformGestures"));
    assert!(kotlin.contains("store.perform(pinchActions"));
}

#[test]
/// Checks the DevRuntime conditional rendering paths and their transition table.
/// Device animation timing still requires a native runtime test.
fn conditional_view_transitions_are_consumed_by_both_dev_renderers() {
    let (root, _) = fixture();
    let swift = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");

    assert!(swift.contains("private func nexaDevTransition(_ raw: Any?) -> AnyTransition?"));
    assert!(
        swift.contains("rendered.transition(transition).animation(.default, value: condition)")
    );
    assert!(swift.contains("rendered.transition(transition).animation(.default, value: value)"));
    for transition in ["Fade", "SlideFromBottom", "Scale"] {
        assert!(kotlin.contains(&format!("\"{transition}\" ->")));
    }
    assert!(kotlin.contains("AnimatedContent(\n                    targetState = condition"));
    assert!(kotlin.contains("AnimatedContent(\n                    targetState = value"));
}

#[test]
/// Scenario labels are maintainer inventory metadata; this test does not run
/// each listed scenario on a simulator or emulator.
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
/// Feature labels describe current boundaries and are not runtime test results.
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
                matches!(
                    value,
                    Some("pending" | "covered" | "partial" | "native_rebuild" | "aot_only")
                ),
                "runtime feature {name} needs an explicit {platform} test status"
            );
            if require_coverage {
                assert_ne!(
                    value,
                    Some("pending"),
                    "runtime feature {name} has no audited boundary for {platform}"
                );
            }
        }
    }
}

#[test]
fn plugin_hot_reload_coverage_uses_log_only_semantic_device_evidence() {
    let (root, fixture) = fixture();
    let script =
        fs::read_to_string(root.join("../../scripts/test-dev-runtime-plugin-hot-reload.sh"))
            .expect("read log-only plugin hot-reload probe");
    let probe_sources = [
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/App.nx"),
        root.join("tests/fixtures/dev_runtime_plugin_probe_app/ProbePanel.nx"),
    ]
    .map(|path| fs::read_to_string(path).expect("read plugin probe source"))
    .join("\n");
    let features = fixture["runtime_features"]
        .as_object()
        .expect("runtime feature inventory");
    let expected = [
        "plugin_service_call_shapes",
        "typed_plugin_error_catches",
        "plugin_native_class_scalar_constructors_and_methods",
        "native_plugin_properties_events_and_components",
    ];

    assert!(script.contains("logcat"));
    assert!(script.contains("log stream"));
    assert!(script.contains("cp \"$app_fixture/ProbePanel.nx\" \"$tmp_root/ProbePanel.nx\""));
    assert!(script.contains("wait_for_marker \"$native_log\" DEVRT_IMPORTED_COMPONENT_PASS"));
    assert!(script.contains("Rebuilding native app"));

    for feature_name in expected {
        let feature = &features[feature_name];
        for platform in ["ios", "android"] {
            assert_eq!(
                feature[platform].as_str(),
                Some("covered"),
                "{feature_name} requires successful semantic device evidence on {platform}"
            );
        }
        let markers = feature["semantic_probe_markers"]
            .as_array()
            .unwrap_or_else(|| panic!("{feature_name} has no semantic probe markers"));
        assert!(
            !markers.is_empty(),
            "{feature_name} has no probe assertions"
        );
        for marker in markers {
            let marker = marker.as_str().expect("probe marker string");
            assert!(script.contains(marker), "probe does not wait for {marker}");
            assert!(
                probe_sources.contains(marker),
                "plugin probe source does not assert {marker}"
            );
        }
    }
}

#[test]
fn keyboard_ergonomics_are_consumed_by_dev_runtime() {
    let (root, _) = fixture();
    let swift_renderer = fs::read_to_string(root.join("../../runtime/ios/NexaDevRenderer.swift"))
        .expect("read iOS renderer");
    let kotlin_renderer = fs::read_to_string(root.join("../../runtime/android/NexaDevRenderer.kt"))
        .expect("read Android renderer");
    let swift_native = fs::read_to_string(root.join("../../runtime/ios/NexaDevNativeApis.swift"))
        .expect("read iOS native API adapter");
    let kotlin_native = fs::read_to_string(root.join("../../runtime/android/NexaDevNativeApis.kt"))
        .expect("read Android native API adapter");
    let kotlin_core = nexa_codegen::value::kotlin_core_runtime_source();

    for (platform, source, markers) in [
        (
            "iOS",
            swift_renderer.as_str(),
            [
                "case \"Username\": .username",
                "case \"Password\": .password",
                "case \"OneTimeCode\": .oneTimeCode",
                "case \"Search\": .search",
                "case \"Send\": .send",
                "case \"Next\": .next",
                "store.perform(submitActions, scope: scope, locals: locals)",
            ],
        ),
        (
            "Android",
            kotlin_renderer.as_str(),
            [
                "ContentType.Username",
                "ContentType.Password",
                "ContentType.SmsOtpCode",
                "ImeAction.Search",
                "ImeAction.Send",
                "ImeAction.Next",
                "store.perform(submitActions, scope, locals)",
            ],
        ),
    ] {
        for marker in markers {
            assert!(
                source.contains(marker),
                "{platform} DevRuntime keyboard handling is missing {marker}"
            );
        }
    }
    assert!(swift_native.contains("namespace == \"Keyboard\", name == \"dismiss\""));
    assert!(kotlin_native.contains("namespace == \"Keyboard\" && name == \"dismiss\""));
    assert!(kotlin_core.contains("foregroundActivity"));
    assert!(kotlin_core.contains("hideSoftInputFromWindow"));
}
