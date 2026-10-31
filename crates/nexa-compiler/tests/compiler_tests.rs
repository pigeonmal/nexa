use std::{fs, path::Path};

use nexa_ir::{
    AccessibilityRole, Action, ArithmeticOp, AutofillType, CollectionMutation, ColorExpression,
    Expr, ListPlan, MemberKind, Node, NumericType, ReturnKeyType, SystemIcon, Type,
    walk::{walk_actions, walk_ir},
};

use nexa_codegen::Backend;
use nexa_compiler::{
    Target, WarningCode, compile, compile_file_with_warnings_for_target,
    compile_with_warnings_for_target,
};
use nexa_testkit::TestProject;

#[test]
fn binary_file_apis_lower_to_typed_throwing_ir_and_native_calls() {
    let module = compile(
        r#"
        app BinaryFileContract {
            state status: String = "Waiting"

            body {
                OnAppear async {
                    try {
                        let path = Path.join(Path.documents(), "receipt.pdf")
                        let payload = Bytes.fromText(text: "Receipt 4481")
                        let wrote = await File.writeBytes(path: path, contents: payload)
                        let restored: Bytes = await File.readBytes(path: path)
                        let sameLength = Bytes.count(bytes: restored) == Bytes.count(bytes: payload)
                        status = wrote && sameLength ? "Saved" : "Could not verify bytes"
                    } catch {
                        status = "File access failed"
                    }
                }
                Text(status)
            }
        }
        "#,
    )
    .expect("binary file APIs should type-check and preserve I/O failures");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("try await NexaFile.write("), "{swift}");
    assert!(swift.contains("try await NexaFile.read("), "{swift}");
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("NexaFile.write("), "{kotlin}");
    assert!(kotlin.contains("NexaFile.read("), "{kotlin}");

    let invalid = compile(
        r#"
        app InvalidBinaryFileWrite {
            body {
                OnAppear async {
                    try {
                        await File.writeBytes(path: "receipt.pdf", contents: "not bytes")
                    } catch {
                    }
                }
                Text("Receipt")
            }
        }
        "#,
    )
    .expect_err("File.writeBytes must accept Bytes rather than arbitrary text");
    assert!(invalid.to_string().contains("Bytes"), "{invalid}");
}

fn source_text(expression: &Expr) -> Option<&str> {
    match expression {
        Expr::String(value) => Some(value),
        Expr::LocalizedText { value, .. } => source_text(value),
        _ => None,
    }
}

#[test]
fn string_trimmed_is_a_typed_member_property() {
    let module = compile(
        r#"
        app TrimInput {
            state name: String = "  Nexa  "
            body { Text(name.trimmed) }
        }
        "#,
    )
    .expect("String.trimmed should compile as a property");

    assert!(matches!(
        module.body.first(),
        Some(Node::Text {
            value: Expr::Member {
                name,
                kind: MemberKind::StringTrimmed,
                base_type: Type::String,
                field_type: Type::String,
                ..
            },
            ..
        }) if name == "trimmed"
    ));
}

#[test]
fn switch_change_action_is_typed_and_generated_for_both_native_targets() {
    let module = compile(
        r#"
        app PlaybackPreferences {
            state skipSilence: Bool = false
            state savedPreference: Bool = false

            body {
                Switch(value: skipSilence, label: "Skip silence").onChange { enabled ->
                    savedPreference = enabled
                }
            }
        }
        "#,
    )
    .expect("Switch onChange should receive a typed Bool value");

    let Node::Switch {
        on_change: Some(change),
        ..
    } = &module.body[0]
    else {
        panic!("Switch should retain its typed change action");
    };
    assert_eq!(change.parameter, "enabled");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("onChange: { nexa_enabled in"), "{swift}");
    assert!(
        swift.contains("nexa_savedPreference = nexa_enabled"),
        "{swift}"
    );

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("val nexa_enabled = value"), "{kotlin}");
    assert!(
        kotlin.contains("nexa_savedPreference = nexa_enabled"),
        "{kotlin}"
    );

    let invalid = compile(
        r#"
        app InvalidSwitchChange {
            state enabled: Bool = false
            body { Switch(value: enabled, label: "Enabled").onChange { next -> enabled = 1 } }
        }
        "#,
    )
    .expect_err("Switch change values must be Bool");
    assert!(invalid.to_string().contains("Bool"), "{invalid}");
}

#[test]
fn network_request_headers_and_response_helpers_are_typed() {
    let module = compile(
        r##"
        app NetworkHeaderContract {
            state accessToken: String = "session-token"
            state profileJson: String = ""
            state cookieHeader: String = ""
            state contentType: String? = null
            state refreshTask: TaskHandle? = null

            body {
                Button("Refresh profile") {
                    Task.launch(handle: refreshTask, executor: TaskExecutor.Main) {
                        try {
                            let requestHeaders: Map<String, String> = [
                                "Authorization": "Bearer \(accessToken)",
                                "Accept": "application/json"
                            ]
                            let response = await Network.fetch(
                                url: "https://api.example.com/v1/profile",
                                headers: requestHeaders,
                                timeoutSeconds: 12.5
                            )
                            profileJson = response.body
                            cookieHeader = response.getCookie()
                            contentType = response.getHeader(name: "content-type")
                            let setCookies: Array<String> = response.getHeaders(name: "Set-Cookie")
                            let contentTypes: Array<String> = response.getHeaders("Content-Type")
                        } catch {
                        }
                    }
                }
                Text(profileJson)
            }
        }
        "##,
    )
    .expect("request maps and response header helpers should be typed");

    assert!(
        module
            .structs
            .iter()
            .all(|declaration| declaration.name != "HttpHeader"),
        "NetworkResponse should not add a public header wrapper type"
    );
    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains(".getCookie()"));
    assert!(swift.contains(".getHeader(\"content-type\")"));
    assert!(swift.contains("public func getHeader(_ name: String) -> String?"));
    assert!(swift.contains(
        "headers.first(where: { $0.0.caseInsensitiveCompare(name) == .orderedSame })?.1"
    ));
    assert!(swift.contains("public func getHeaders(_ name: String) -> [String]"));
    assert!(swift.contains("public func getCookie() -> String"));
    assert!(swift.contains("timeout: Double(12.5)"));
    assert!(swift.contains("cachePolicy: \"Default\""));
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains(".getCookie()"));
    assert!(kotlin.contains(".getHeader(\"content-type\")"));
    assert!(kotlin.contains("fun getHeader(name: String): String?"));
    assert!(
        kotlin.contains("headers.firstOrNull { it.first.equals(name, ignoreCase = true) }?.second")
    );
    assert!(kotlin.contains("fun getHeaders(name: String): List<String>"));
    assert!(kotlin.contains("fun getCookie(): String"));
    assert!(kotlin.contains("(12.5 * 1000.0).toLong()"));
    assert!(kotlin.contains("\"Default\""));
}

#[test]
fn network_cache_policy_is_explicit_and_validated() {
    let module = compile(include_str!("../../../examples/network_caching.nx"))
        .expect("network cache policies should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("cachePolicy: \"CacheOnly\""));
    assert!(swift.contains("cachePolicy: \"Reload\""));
    assert!(swift.contains("case \"CacheOnly\": return .returnCacheDataDontLoad"));
    assert!(swift.contains("case \"Reload\": return .reloadIgnoringLocalCacheData"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("\"CacheOnly\""));
    assert!(kotlin.contains("\"Reload\""));
    assert!(kotlin.contains("builder.disableCache()"));
    assert!(kotlin.contains("only-if-cached, max-stale"));
}

#[test]
fn network_cache_policy_rejects_unknown_modes_and_the_old_boolean() {
    let invalid_mode = compile(
        r#"
        app InvalidCachePolicy {
            body {
                OnAppear async {
                    try {
                        let response = await Network.fetch(
                            url: "https://example.com",
                            cachePolicy: "AlwaysNetwork"
                        )
                    } catch { }
                }
                Text("Ready")
            }
        }
        "#,
    )
    .expect_err("unknown cache policies must fail at compile time");
    assert!(
        invalid_mode
            .to_string()
            .contains("Network.fetch cachePolicy must be one of: Default, Reload, CacheOnly")
    );

    let old_option = compile(
        r#"
        app OldCacheToggle {
            body {
                OnAppear async {
                    try {
                        let response = await Network.fetch(
                            url: "https://example.com",
                            useCache: false
                        )
                    } catch { }
                }
                Text("Ready")
            }
        }
        "#,
    )
    .expect_err("the unclear useCache Boolean should be removed");
    assert!(old_option.to_string().contains(
        "unknown option `useCache` for `Network.fetch`; use `cachePolicy` with `Default`, `Reload`, or `CacheOnly`"
    ));
}

#[test]
fn typed_storage_apis_keep_optional_reads_and_native_values_typed() {
    let module = compile(include_str!("../../../examples/typed_storage.nx"))
        .expect("typed storage reads and writes should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    for expected in [
        "NexaStorage.getBool(\"reader.notifications\")",
        "NexaStorage.getInt64(\"reader.weekly-goal\")",
        "NexaStorage.getFloat64(\"reader.text-scale\")",
        "NexaStorage.setBool(\"reader.notifications\",",
        "NexaStorage.setInt64(\"reader.weekly-goal\",",
        "NexaStorage.setFloat64(\"reader.text-scale\",",
        "static func getInt64(_ key: String) -> Int64?",
        "static func getFloat64(_ key: String) -> Double?",
    ] {
        assert!(
            swift.contains(expected),
            "Swift output should contain `{expected}`"
        );
    }

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    for expected in [
        "NexaStorage.getBool(NexaRuntime.context(), \"reader.notifications\")",
        "NexaStorage.getInt64(NexaRuntime.context(), \"reader.weekly-goal\")",
        "NexaStorage.getFloat64(NexaRuntime.context(), \"reader.text-scale\")",
        "NexaStorage.setBool(NexaRuntime.context(), \"reader.notifications\",",
        "NexaStorage.setInt64(NexaRuntime.context(), \"reader.weekly-goal\",",
        "NexaStorage.setFloat64(NexaRuntime.context(), \"reader.text-scale\",",
        "fun getInt64(context: Context, key: String): Long?",
        "fun getFloat64(context: Context, key: String): Double?",
    ] {
        assert!(
            kotlin.contains(expected),
            "Kotlin output should contain `{expected}`"
        );
    }
}

#[test]
fn network_timeout_requires_explicit_seconds_name() {
    let error = compile(
        r#"
        app InvalidTimeoutName {
            body {
                OnAppear async {
                    try {
                        let response = await Network.fetch(
                            url: "https://example.com",
                            timeout: 5
                        )
                    } catch { }
                }
                Text("Profile")
            }
        }
        "#,
    )
    .expect_err("the unit-less timeout parameter should be rejected");
    assert!(error.to_string().contains(
        "unknown option `timeout` for `Network.fetch`; use `timeoutSeconds` (measured in seconds)"
    ));
}

#[test]
fn permission_status_preserves_limited_provisional_and_permanent_states() {
    let module = compile(
        r#"
        app PermissionStateScreen {
            state photoStatus: PermissionStatus = PermissionStatus.notDetermined
            state notificationStatus: PermissionStatus = PermissionStatus.notDetermined
            body {
                OnAppear async {
                    photoStatus = await Permissions.status(permission: Permission.Photos)
                    notificationStatus = await Permissions.status(permission: Permission.Notifications)
                }
                Text("Permission settings")
            }
        }
        "#,
    )
    .expect("permission status cases should remain typed across platforms");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("case limited"));
    assert!(swift.contains("case provisional"));
    assert!(swift.contains("case permanentlyDenied"));
    assert!(swift.contains("case .limited: return .limited"));
    assert!(swift.contains("case .provisional: return .provisional"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("limited,"));
    assert!(kotlin.contains("provisional,"));
    assert!(kotlin.contains("permanentlyDenied,"));
    assert!(kotlin.contains("READ_MEDIA_VISUAL_USER_SELECTED"));
    assert!(kotlin.contains("denialCount >= 2"));
    assert!(kotlin.contains("shouldShowRequestPermissionRationale(permission)"));
}

#[test]
fn network_response_get_headers_requires_one_string_name() {
    let error = compile(
        r#"
        app InvalidHeaderLookup {
            body {
                OnAppear async {
                    try {
                        let response = await Network.fetch(url: "https://example.com")
                        let values: Array<String> = response.getHeaders(name: 42)
                    } catch {
                    }
                }
                Text("Headers")
            }
        }
        "#,
    )
    .expect_err("getHeaders must require a String header name");
    assert!(
        error.to_string().contains("String"),
        "unexpected error: {error}"
    );
}

#[test]
fn type_mismatch_diagnostics_show_expected_and_actual_types_with_a_fix_hint() {
    let error = compile(
        r#"
        app ClearTypeError {
            body {
                Button("Read documents path") {
                    let documentPath: Int32 = Path.documents()
                }
            }
        }
        "#,
    )
    .expect_err("a String result cannot initialize an Int32 local");

    assert!(
        error.message.contains("expected Int32, found String"),
        "type diagnostic should include both types: {error}"
    );
    assert!(
        error
            .message
            .contains("change the expression or expected type"),
        "type diagnostic should suggest a correction: {error}"
    );
    assert_eq!(error.span.line, 5, "diagnostic should point at the call");
}

#[test]
fn network_response_raw_headers_property_is_not_part_of_the_language_api() {
    let error = compile(
        r#"
        app RawNetworkHeaders {
            body {
                OnAppear async {
                    try {
                        let response = await Network.fetch(url: "https://example.com")
                        let headers: Array<String> = response.headers
                    } catch {
                    }
                }
                Text("Network")
            }
        }
        "#,
    )
    .expect_err("response headers should be read through the documented helpers");
    assert!(
        error
            .to_string()
            .contains("NetworkResponse` has no member `headers"),
        "unexpected error: {error}"
    );
}

#[test]
fn network_fetch_rejects_nested_header_maps() {
    let error = compile(
        r#"
        app InvalidNetworkHeaders {
            body {
                OnAppear async {
                    try {
                        let headers: Map<String, Array<String>> = [
                            "Accept": ["application/json"]
                        ]
                        await Network.fetch(
                            url: "https://example.com",
                            headers: headers
                        )
                    } catch {
                    }
                }
                Text("Headers")
            }
        }
        "#,
    )
    .expect_err("request headers must use one string value per name");
    assert!(
        error.to_string().contains("Map<String, String>"),
        "unexpected type error: {error}"
    );
}

#[test]
fn network_api_does_not_add_a_public_header_wrapper_type() {
    let module = compile(
        r#"
        app NetworkFetchOnly {
            body {
                OnAppear async {
                    try {
                        await Network.fetch(url: "https://example.com")
                    } catch {
                    }
                }
                Text("Network")
            }
        }
        "#,
    )
    .expect("the fetch-only app should compile");

    assert!(
        module
            .structs
            .iter()
            .all(|declaration| declaration.name != "HttpHeader"),
        "response header storage is internal to the native network adapter"
    );
}

#[test]
fn fast_list_reverse_layout_is_typed_for_flat_vertical_lists() {
    let module = compile(
        r##"
        app ReverseList {
            body {
                FastList(count: 3, reverseLayout: true) { index in
                    Text(index)
                }
            }
        }
        "##,
    )
    .expect("a flat vertical FastList may use reverseLayout");

    let Some(Node::FastList { plan }) = module.body.first() else {
        panic!("expected the count-backed FastList plan");
    };
    let ListPlan::Count { common, .. } = plan.as_ref() else {
        panic!("expected the count-backed FastList plan");
    };
    assert!(common.reverse_layout);

    let horizontal = compile(
        r#"
        app InvalidReverseAxis {
            body {
                FastList(count: 3, axis: Horizontal, reverseLayout: true) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect_err("reverseLayout is supported only on vertical lists");
    assert!(
        horizontal
            .to_string()
            .contains("supported only for vertical lists")
    );
}

#[test]
fn refresh_control_wraps_accessible_fast_list_with_native_refresh_behavior() {
    let module = compile(
        r#"
        app RefreshAccessibleFastList {
            state refreshing: Bool = false
            state refreshCount: Int32 = 0
            body {
                RefreshControl(isRefreshing: refreshing) {
                    FastList(count: 8, rowHeight: 56, accessibilityLabel: "Refresh rows") { index in
                        Text(index)
                    }
                }.onRefresh {
                    refreshing = false
                    refreshCount = refreshCount + 1
                }
            }
        }
        "#,
    )
    .expect("RefreshControl around an accessible FastList should compile");

    let Some(Node::Accessibility {
        label, children, ..
    }) = module.body.first()
    else {
        panic!("FastList accessibility options should remain a typed wrapper");
    };
    assert_eq!(source_text(label), Some("Refresh rows"));
    let Some(Node::FastList { plan }) = children.first() else {
        panic!("RefreshControl should specialize its FastList child");
    };
    let refresh = plan
        .refresh()
        .expect("the native FastList should own the RefreshControl action");
    assert_eq!(refresh.state, "refreshing");
    assert_eq!(refresh.actions.len(), 2);

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("isRefreshing: nexa_refreshing"));
    assert!(swift.contains("onRefresh: {"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("PullToRefreshBox("));
    assert!(kotlin.contains("isRefreshing = nexa_refreshing"));
    assert!(kotlin.contains("userScrollEnabled = true"));
}

#[test]
fn fast_list_show_dividers_is_typed_and_reaches_both_native_backends() {
    let module = compile(
        r#"
        struct DividerRow {
            id: String,
        }

        app DividerOptions {
            let rows: Array<DividerRow> = [DividerRow("first"), DividerRow("second")]
            let groups: Array<Array<DividerRow>> = [rows]

            body {
                FastList(count: 2, showDividers: false) { index in Text(index) }
                FastList(rows, key: .id, showDividers: false) { row, index in Text(row.id) }
                FastList(sections: groups, showDividers: false) { row, index, section in Text(row.id) }
            }
        }
        "#,
    )
    .expect("showDividers is valid on flat and sectioned lists");

    let plans = module
        .body
        .iter()
        .filter_map(|node| match node {
            Node::FastList { plan } => Some(plan.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(plans.len(), 3);
    assert!(plans.iter().all(|plan| match plan {
        ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => !common.show_dividers,
        ListPlan::Sections { common, .. } => !common.show_dividers,
    }));

    let swift = nexa_backend_swift::SwiftBackend
        .generate_units(&module)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(swift.matches("NexaFastListPrimitive(rowCount:").count(), 2);
    assert_eq!(
        swift
            .matches(", showDividers: false, scrollEnabled: true")
            .count(),
        2
    );
    assert!(swift.contains(
        "NexaFastListPrimitive(rowCount: max(0, Int(2)), rowHeight: nil, showDividers: false, scrollEnabled: true)"
    ));
    let sectioned_start = swift
        .find("NexaFastSectionedListPrimitive")
        .expect("sectioned FastList uses the shared sectioned primitive");
    let sectioned_parameters = swift[sectioned_start..]
        .split("rowContent:")
        .next()
        .unwrap_or_default();
    assert!(sectioned_parameters.contains("showDividers: false"));

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(!kotlin.contains("NexaFastListDividerPrimitive()"));
    assert_eq!(
        kotlin.matches("NexaNativeListDividerPrimitive()").count(),
        1
    );

    let sectioned = compile(
        r#"
        app SectionedSpacing {
            let groups: Array<Array<String>> = [["First", "Second"], ["Third"]]

            body {
                FastList(sections: groups, rowHeight: 48, spacing: 4, showDividers: true) { item, index, section in
                    Text(item)
                }.sectionHeader {
                    Text("Section")
                }
            }
        }
        "#,
    )
    .expect("sectioned lists may combine row sizing, spacing, and dividers");

    let swift = nexa_backend_swift::SwiftBackend.generate(&sectioned);
    let sectioned_call = swift
        .find("NexaFastSectionedListPrimitive(")
        .map(|start| &swift[start..])
        .expect("Swift should emit the sectioned-list primitive");
    let row_height = sectioned_call
        .find("rowHeight: 48")
        .expect("rowHeight should be emitted");
    let spacing = sectioned_call
        .find("spacing: 4")
        .expect("spacing should be emitted");
    let dividers = sectioned_call
        .find("showDividers: true")
        .expect("showDividers should be emitted");
    assert!(row_height < spacing && spacing < dividers);

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&sectioned);
    assert!(kotlin.contains("Arrangement.spacedBy(4.dp)"));
    assert!(kotlin.contains("Modifier.fillMaxWidth().height(48.dp)"));
    assert!(kotlin.contains("key = { itemPosition -> sectionPosition to itemPosition },"));
    assert!(kotlin.contains("NexaFastListDividerPrimitive()"));
}

#[test]
fn fast_list_page_snap_is_typed_and_requires_viewport_sized_vertical_rows() {
    let module = compile(
        r#"
        app PagedFeed {
            body {
                FastList(count: 3, pageSnap: true) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect("a flat vertical FastList may use pageSnap");

    let Some(Node::FastList { plan }) = module.body.first() else {
        panic!("expected the count-backed FastList plan");
    };
    let ListPlan::Count { common, .. } = plan.as_ref() else {
        panic!("expected the count-backed FastList plan");
    };
    assert!(common.page_snap);
    assert!(
        nexa_ir::facts::ModuleFacts::analyze(&module)
            .ui
            .lists
            .page_snap
    );

    let horizontal = compile(
        r#"
        app InvalidPagedAxis {
            body {
                FastList(count: 3, axis: Horizontal, pageSnap: true) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect_err("pageSnap is restricted to vertical lists");
    assert!(
        horizontal
            .to_string()
            .contains("pageSnap` is supported only for vertical lists")
    );

    let fixed_height = compile(
        r#"
        app InvalidPagedHeight {
            body {
                FastList(count: 3, rowHeight: 80, pageSnap: true) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect_err("pageSnap determines row height from the viewport");
    assert!(fixed_height.to_string().contains("omit `rowHeight`"));

    let spaced_pages = compile(
        r#"
        app InvalidPagedSpacing {
            body {
                FastList(count: 3, pageSnap: true, spacing: 8) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect_err("pageSnap rows fill the viewport and cannot be separated");
    assert!(
        spaced_pages
            .to_string()
            .contains("`pageSnap` fills the viewport")
    );
}

#[test]
fn fast_list_auto_grid_uses_a_positive_minimum_cell_width() {
    let module = compile(
        r#"
        app ResponsiveVideoGrid {
            body {
                FastList(count: 6, axis: AutoGrid(168), rowHeight: 220, spacing: 12) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect("AutoGrid should accept a positive minimum cell width");

    let Some(Node::FastList { plan }) = module.body.first() else {
        panic!("expected the count-backed FastList plan");
    };
    let ListPlan::Count { common, .. } = plan.as_ref() else {
        panic!("expected the count-backed FastList plan");
    };
    assert_eq!(
        common.axis,
        nexa_ir::ListAxis::AutoGrid {
            min_cell_width: 168
        }
    );
    assert_eq!(common.spacing, 12.0);
    assert!(nexa_ir::facts::ModuleFacts::analyze(&module).ui.lists.grid);

    let invalid = compile(
        r#"
        app InvalidResponsiveGrid {
            body {
                FastList(count: 6, axis: AutoGrid(0)) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect_err("AutoGrid must reject a zero minimum cell width");
    assert!(invalid.to_string().contains("must be greater than zero"));

    let negative_spacing = compile(
        r#"
        app InvalidGridSpacing {
            body {
                FastList(count: 6, axis: AutoGrid(168), spacing: -2) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect_err("FastList spacing cannot be negative");
    assert!(
        negative_spacing
            .to_string()
            .contains("must be finite and non-negative")
    );
}

#[test]
fn fast_list_scroll_enabled_is_a_reactive_boolean_option() {
    let module = compile(
        r#"
        app ScrollableFeed {
            state canScroll: Bool = true
            body {
                FastList(count: 20, axis: AutoGrid(168), scrollEnabled: canScroll) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect("scrollEnabled should accept a reactive Bool");

    let Some(Node::FastList { plan }) = module.body.first() else {
        panic!("expected the count-backed FastList plan");
    };
    let ListPlan::Count { common, .. } = plan.as_ref() else {
        panic!("expected the count-backed FastList plan");
    };
    assert!(matches!(
        common.scroll_enabled.as_ref(),
        Some(Expr::State(name, Type::Bool)) if name == "canScroll"
    ));

    let invalid = compile(
        r#"
        app InvalidScrollableFeed {
            state canScroll: Int32 = 1
            body {
                FastList(count: 20, scrollEnabled: canScroll) { index in Text(index) }
            }
        }
        "#,
    )
    .expect_err("scrollEnabled must reject non-Bool expressions");
    assert!(invalid.to_string().contains("Bool"));
}

#[test]
fn fast_list_member_keys_lower_in_the_row_binding_scope() {
    let module = compile(
        r#"
        struct VideoClip {
            id: String,
            title: String,
        }

        app VideoFeed {
            let clips: Array<VideoClip> = [
                VideoClip("first", "First"),
                VideoClip("second", "Second"),
            ]
            state currentPage: Int32 = 0

            body {
                FastList(clips, key: .id, pageSnap: true, scrollPosition: currentPage) { clip, index in
                    Text(clip.title)
                }
            }
        }
        "#,
    )
    .expect("FastList member keys resolve from the row binding type");

    let Some(Node::FastList { plan }) = module.body.first() else {
        panic!("expected the collection-backed FastList plan");
    };
    let ListPlan::Items { common, .. } = plan.as_ref() else {
        panic!("expected the collection-backed FastList plan");
    };
    assert!(matches!(common.key.as_ref(), Some(Expr::Member { .. })));
}

#[test]
fn flat_map_flattens_typed_arrays_in_collection_closures() {
    let module = compile(
        r#"
        struct Record {
            id: String,
            title: String,
        }

        app SearchResults {
            let sections: Array<Array<Record>> = [
                [Record("first", "First")],
                [Record("second", "Second")],
            ]
            let results: Array<Record> = sections.flatMap { rows -> rows }

            body {
                FastList(results, key: .id) { record, index in
                    Text(record.title)
                }
            }
        }
        "#,
    )
    .expect("flatMap should flatten each typed array result");

    let Some(Node::FastList { plan }) = module.body.first() else {
        panic!("expected the flattened results list");
    };
    let ListPlan::Items { element_type, .. } = plan.as_ref() else {
        panic!("expected the flattened results list");
    };
    assert!(matches!(element_type, Type::Struct { name, .. } if name == "Record"));

    let invalid = compile(
        r#"
        app InvalidFlatMap {
            let values: Array<Int32> = [1, 2].flatMap { value -> value }
            body { Text(values.count) }
        }
        "#,
    )
    .expect_err("flatMap closures must return arrays");
    let _ = invalid;
}

#[test]
fn platform_fast_list_behavior_is_selected_automatically_for_row_actions() {
    let module = compile(
        r#"
        struct Record {
            id: String,
            title: String,
        }

        app SwipeActionRecordList {
            let records: Array<Record> = [Record("one", "One")]

            body {
                FastList(records, key: .id) { record, index in
                    Text(record.title)
                }.swipeActions {
                    Button("Delete") { }
                }
            }
        }
        "#,
    )
    .expect("swipe actions select the platform list surface automatically");

    let Some(Node::FastList { plan }) = module.body.first() else {
        panic!("expected the collection-backed FastList plan");
    };
    let ListPlan::Items { .. } = plan.as_ref() else {
        panic!("expected the collection-backed FastList plan");
    };
    assert!(plan.uses_platform_list());
    let list_facts = nexa_ir::facts::ModuleFacts::analyze(&module).ui.lists;
    assert!(list_facts.any);
    assert!(!list_facts.virtualized);

    let sectioned = compile(
        r#"
        struct Record {
            id: String,
            title: String,
        }

        app SectionedRecordList {
            let records: Array<Record> = [Record("one", "One")]
            let sections: Array<Array<Record>> = records.groupedBy { record -> record.title }

            body {
                FastList(sections: sections) { record, index, section in
                    Text(record.title)
                }.sectionHeader {
                    Text(sectionItems[0].title)
                }
            }
        }
        "#,
    )
    .expect("sectioned lists keep the shared virtualized presentation by default");
    let Some(Node::FastList { plan }) = sectioned.body.first() else {
        panic!("expected a sectioned FastList plan");
    };
    let ListPlan::Sections { common, .. } = plan.as_ref() else {
        panic!("expected a sectioned FastList plan");
    };
    assert!(!plan.uses_platform_list());
    assert!(common.section_header.is_some());

    let unsupported = compile(
        r#"
        app RemovedFastListNativeOption {
            body {
                FastList(count: 3, native: true) { index in
                    Text(index)
                }
            }
        }
        "#,
    )
    .expect_err("the removed native parameter must no longer parse");
    assert!(unsupported.to_string().contains("native"));
}

#[test]
fn numeric_coalesce_values_can_be_used_in_collection_mutations() {
    compile(
        r#"
        app IncrementMapValue {
            state counts: Map<String, Int32> = ["first": 1]

            body {
                Button("Increment") {
                    counts.set("first", (counts["first"] ?? 0) + 1)
                }
            }
        }
        "#,
    )
    .expect("a numeric fallback keeps its type through arithmetic and map mutation");
}

#[test]
fn mutable_map_clear_lowers_to_typed_collection_ir() {
    let module = compile(
        r#"
        app ClearCart {
            state quantities: Map<String, Int32> = ["sku-441": 2]

            body {
                Button("Clear cart") {
                    quantities.clear()
                }
            }
        }
        "#,
    )
    .expect("Map.clear should compile for mutable map state");

    let Some(Node::Button { actions, .. }) = module
        .body
        .iter()
        .find(|node| matches!(node, Node::Button { .. }))
    else {
        panic!("expected a clear cart button");
    };
    assert!(matches!(
        actions.as_slice(),
        [Action::CollectionMutation {
            name,
            operation: CollectionMutation::MapClear,
            arguments,
        }] if name == "quantities" && arguments.is_empty()
    ));
}

#[test]
fn in_memory_compile_merges_top_level_screens_into_the_app() {
    let module = compile(
        r#"
        screen Home {
            Text("Home")
        }

        app ModularNavigation {
            body {
                NavigationStack(root: Home)
            }
        }
        "#,
    )
    .expect("top-level screen should be available to the app");

    assert_eq!(module.screens.len(), 1);
    assert_eq!(module.screens[0].name, "Home");
    assert!(matches!(
        module.body.first(),
        Some(Node::NavigationStack {
            root: nexa_ir::ScreenId(0),
            ..
        })
    ));
}

#[test]
fn app_root_navigation_stacks_can_be_selected_by_top_level_control_flow() {
    let module = compile(
        r#"
        app ConditionalNavigationRoot {
            state authenticated: Bool = false
            screen Welcome { Text("Welcome") }
            screen Library { Text("Library") }

            body {
                if authenticated {
                    NavigationStack(root: Library)
                } else {
                    NavigationStack(root: Welcome)
                }
            }
        }
        "#,
    )
    .expect("each top-level app branch should be able to choose its own navigation root");

    let Some(Node::If {
        then_body,
        else_body: Some(else_body),
        ..
    }) = module.body.first()
    else {
        panic!("expected conditional app root");
    };
    assert!(matches!(
        then_body.first(),
        Some(Node::NavigationStack { .. })
    ));
    assert!(matches!(
        else_body.first(),
        Some(Node::NavigationStack { .. })
    ));

    let nested = compile(
        r#"
        app NestedConditionalNavigationRoot {
            state authenticated: Bool = false
            screen Welcome { Text("Welcome") }
            screen Library { Text("Library") }

            body {
                Column {
                    if authenticated {
                        NavigationStack(root: Library)
                    } else {
                        NavigationStack(root: Welcome)
                    }
                }
            }
        }
        "#,
    )
    .expect_err("navigation stacks inside visual layouts must remain invalid");
    assert!(nested.message.contains("app body"), "{nested}");
}

#[test]
fn app_bottom_bar_tabs_can_own_navigation_roots() {
    let module = compile(
        r#"
        app TabNavigationRoots {
            state selectedTab: Int32 = 0

            screen Home { Text("Home") }
            screen Library { Text("Library") }
            screen MainTabs {
                AppBottomBar(selected: selectedTab) {
                    Tab(index: 0, label: "Home", icon: "home") {
                        NavigationStack(root: Home)
                    }
                    Tab(index: 1, label: "Library", icon: "music_library") {
                        NavigationStack(root: Library)
                    }
                }
            }

            body { NavigationStack(root: MainTabs) }
        }
        "#,
    )
    .expect("each native app tab should own an independent navigation root");

    let main_tabs = module
        .screens
        .iter()
        .find(|screen| screen.name == "MainTabs")
        .expect("the app should retain its tab shell screen");
    assert!(matches!(
        main_tabs.body.first(),
        Some(Node::AppBottomBar { tabs, .. })
            if tabs.len() == 2
                && tabs.iter().all(|tab| matches!(
                    tab.children.as_slice(),
                    [Node::NavigationStack { .. }]
                ))
    ));
}

#[test]
fn navigation_push_and_back_lower_to_typed_screen_actions() {
    let module = compile(
        r#"
        screen Home {
            state selectedOrderID: Int32 = 42
            Pressable() { Text("Open order") }
                .onTap { Navigation.push(OrderDetail(selectedOrderID)) }
        }

        screen OrderDetail(orderID: Int32) {
            Pressable() { Text("Back") }
                .onTap { Navigation.back() }
        }

        app NavigationActions {
            body { NavigationStack(root: Home) }
        }
        "#,
    )
    .expect("screen actions should support typed push and back calls");

    let Some(Node::Pressable { actions, .. }) = module.screens[0]
        .body
        .iter()
        .find(|node| matches!(node, Node::Pressable { .. }))
    else {
        panic!("expected the Home screen pressable");
    };
    assert!(matches!(
        actions.as_slice(),
        [Action::NavigationPush {
            destination: nexa_ir::ScreenId(1),
            arguments,
        }] if matches!(arguments.as_slice(), [Expr::State(name, Type::Numeric(NumericType::Int32))] if name == "selectedOrderID")
    ));
    let Some(Node::Pressable { actions, .. }) = module.screens[1]
        .body
        .iter()
        .find(|node| matches!(node, Node::Pressable { .. }))
    else {
        panic!("expected the OrderDetail screen pressable");
    };
    assert!(matches!(actions.as_slice(), [Action::NavigationBack]));
}

#[test]
fn navigation_push_rejects_bad_targets_and_argument_types() {
    let wrong_argument = compile(
        r#"
        app WrongRouteArgument {
            screen Home {
                Pressable() { Text("Open") }
                    .onTap { Navigation.push(OrderDetail("not an id")) }
            }
            screen OrderDetail(orderID: Int32) { Text("Order") }
            body { NavigationStack(root: Home) }
        }
        "#,
    )
    .expect_err("route arguments must match the destination screen signature");
    assert!(wrong_argument.to_string().contains("expected Int32"));

    let unknown_route = compile(
        r#"
        app UnknownRoute {
            screen Home {
                Pressable() { Text("Open") }
                    .onTap { Navigation.push(MissingScreen) }
            }
            body { NavigationStack(root: Home) }
        }
        "#,
    )
    .expect_err("navigation targets must resolve to declared screens");
    assert!(
        unknown_route
            .to_string()
            .contains("unknown screen `MissingScreen`")
    );
}

#[test]
fn component_refs_infer_fast_list_methods_and_attach_to_the_matching_component() {
    let module = compile(
        r#"
        app FastListComponentRef {
            let listRef = ComponentRef<FastList>()

            body {
                Column {
                    FastList(count: 12, ref: listRef) { index in
                        Text(index)
                    }
                    Button("Jump to row") {
                        listRef.scrollToIndex(index: 3)
                    }
                    Button("Top") {
                        listRef.scrollToTop()
                    }
                }
            }
        }
        "#,
    )
    .expect("FastList refs should infer their target and expose typed scroll methods");

    assert!(matches!(
        module.states.as_slice(),
        [state] if state.name == "listRef"
            && state.ty == Type::ComponentRef { namespace: "Nexa".to_owned(), name: "FastList".to_owned() }
    ));
    let mut list_has_ref = false;
    walk_ir(
        &module.body,
        &mut |node| {
            if let Node::FastList { plan } = node {
                list_has_ref |= matches!(
                    plan.as_ref(),
                    ListPlan::Count { common, .. }
                        if matches!(common.component_ref.as_ref(), Some(Expr::State(name, Type::ComponentRef { .. })) if name == "listRef")
                );
            }
        },
        &mut |_| {},
    );
    assert!(
        list_has_ref,
        "expected the FastList node to retain its typed ref"
    );
    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(
        swift.contains("@State private var nexa_listRef: NexaFastListRef = NexaFastListRef()"),
        "SwiftUI must retain a component ref across view recomputations"
    );
    assert!(swift.contains("animated: Bool = true"));
    assert!(swift.contains("viewPosition: Double = 0.0"));
    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(
        kotlin.contains("val nexa_listRef: NexaFastListRef = remember { NexaFastListRef() }"),
        "Compose must retain a component ref across recompositions"
    );
    assert!(kotlin.contains("animated: Boolean = true"));
    assert!(kotlin.contains("viewPosition: Double = 0.0"));
    assert!(kotlin.contains("animateScrollBy(delta)"));
    let mut native_calls = Vec::new();
    walk_ir(&module.body, &mut |_| {}, &mut |expr| {
        if let Expr::NativeCall {
            namespace,
            name,
            return_type,
            ..
        } = expr
            && namespace == "NexaFastListRef"
        {
            native_calls.push((name.clone(), return_type.clone()));
        }
    });
    assert!(native_calls.iter().any(|(name, ty)| {
        name == "scrollToIndex"
            && *ty
                == Type::Result(
                    Box::new(Type::Void),
                    Box::new(Type::ComponentRefError(Box::new(Type::Void))),
                )
    }));
    assert!(native_calls.iter().any(|(name, _)| name == "scrollToTop"));

    let explicitly_typed = compile(
        r#"
        app ExplicitComponentRef {
            let listRef: ComponentRef<FastList> = ComponentRef<FastList>()
            body { FastList(count: 1, ref: listRef) { index in Text(index) } }
        }
        "#,
    )
    .expect("explicit ComponentRef variable annotations should compile");
    assert!(matches!(
        explicitly_typed.states.first(),
        Some(state) if state.ty == Type::ComponentRef { namespace: "Nexa".to_owned(), name: "FastList".to_owned() }
            && matches!(&state.initial, Expr::ComponentRefCreate { identity, .. } if !identity.is_empty())
    ));
}

#[test]
fn fast_list_ref_scroll_options_are_typed_and_defaulted() {
    let module = compile(
        r#"
        app FastListRefScrollOptions {
            let listRef = ComponentRef<FastList>()
            body {
                Column {
                    FastList(count: 30, ref: listRef) { index in Text(index) }
                    Button("Default scroll") { listRef.scrollToIndex(index: 3) }
                    Button("Centered jump") {
                        listRef.scrollToIndex(index: 8, animated: false, viewPosition: 0.5)
                    }
                    Button("Offset jump") { listRef.scrollToOffset(offset: 120.0, animated: false) }
                }
            }
        }
        "#,
    )
    .expect("FastList ref scroll options should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    let swift_ref_calls = swift
        .lines()
        .filter(|line| line.contains("scrollToIndex"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        swift.contains(".scrollToIndex(3, animated: true, viewPosition: Double(0.0))"),
        "{swift_ref_calls}"
    );
    assert!(swift.contains(".scrollToIndex(8, animated: false, viewPosition: Double(0.5))"));
    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(kotlin.contains(".scrollToIndex(3, true, 0.0)"));
    assert!(kotlin.contains(".scrollToIndex(8, false, 0.5)"));

    let mut calls = Vec::new();
    walk_ir(&module.body, &mut |_| {}, &mut |expression| {
        if let Expr::NativeCall {
            namespace,
            name,
            arguments,
            ..
        } = expression
            && namespace == "NexaFastListRef"
        {
            calls.push((name.clone(), arguments.clone()));
        }
    });

    let default_scroll = calls
        .iter()
        .find(|(name, _)| name == "scrollToIndex")
        .expect("expected a scrollToIndex call");
    assert!(matches!(&default_scroll.1[1].1, Expr::Bool(true)));
    assert!(matches!(
        &default_scroll.1[2].1,
        Expr::Number { raw, ty: NumericType::Float64 } if raw == "0.0"
    ));

    let centered_scroll = calls
        .iter()
        .filter(|(name, _)| name == "scrollToIndex")
        .nth(1)
        .expect("expected a second scrollToIndex call");
    assert!(matches!(&centered_scroll.1[1].1, Expr::Bool(false)));
    assert!(matches!(
        &centered_scroll.1[2].1,
        Expr::Number { raw, ty: NumericType::Float64 } if raw == "0.5"
    ));
    let offset_scroll = calls
        .iter()
        .find(|(name, _)| name == "scrollToOffset")
        .expect("expected a scrollToOffset call");
    assert!(matches!(&offset_scroll.1[1].1, Expr::Bool(false)));

    let bad_option = compile(
        r#"
        app BadFastListRefScrollOptions {
            let listRef = ComponentRef<FastList>()
            body {
                FastList(count: 1, ref: listRef) { index in Text(index) }
                Button("Bad option") { listRef.scrollToIndex(index: 0, animated: "yes") }
            }
        }
        "#,
    )
    .expect_err("scroll animation options must be Boolean");
    assert!(bad_option.to_string().contains("Bool"), "{bad_option}");
}

#[test]
fn reusable_components_expose_typed_ref_methods_on_both_backends() {
    let module = compile(
        r#"
        component QuantityStepper(startingQuantity: Int32) {
            state quantity: Int32 = startingQuantity

            fn add(amount: Int32) -> Int32 {
                quantity = quantity + amount
                return quantity + startingQuantity
            }

            body { Text(quantity) }
        }

        app Cart {
            let stepperRef = ComponentRef<QuantityStepper>()
            body {
                Column {
                    QuantityStepper(startingQuantity: 1, ref: stepperRef)
                    Button("Add two") { stepperRef.add(amount: 2) }
                }
            }
        }
        "#,
    )
    .expect("a reusable component can expose a typed ref method");

    let component = module
        .components
        .iter()
        .find(|component| component.name == "QuantityStepper")
        .expect("the referenced reusable component should remain reachable");
    assert_eq!(component.ref_methods.len(), 1);
    assert_eq!(component.ref_methods[0].name, "Nexa.QuantityStepper.add");
    assert_eq!(
        component.ref_methods[0].parameters[0].ty,
        Type::Numeric(NumericType::Int32)
    );
    assert_eq!(
        component.ref_methods[0].return_type,
        Type::Numeric(NumericType::Int32)
    );
    assert!(matches!(
        component.ref_methods[0].body_actions.as_deref(),
        Some([Action::Assign { name, .. }, Action::Return { .. }]) if name == "quantity"
    ));
    let mut method_captures_component_parameter = false;
    nexa_ir::walk::walk_actions(
        component.ref_methods[0]
            .body_actions
            .as_deref()
            .unwrap_or_default(),
        &mut |expression| {
            method_captures_component_parameter |=
                matches!(expression, Expr::State(name, _) if name == "startingQuantity");
        },
    );
    assert!(
        method_captures_component_parameter,
        "the method should retain typed access to its component parameter"
    );
    let mut attached = false;
    let mut typed_call = false;
    walk_ir(
        &module.body,
        &mut |node| {
            if let Node::ComponentCall {
                name,
                component_ref: Some(Expr::State(state, Type::ComponentRef { .. })),
                ..
            } = node
            {
                attached |= name == "QuantityStepper" && state == "stepperRef";
            }
        },
        &mut |expression| {
            if let Expr::NativeCall {
                receiver: Some(receiver),
                namespace,
                name,
                return_type,
                ..
            } = expression
            {
                typed_call |= namespace == "Nexa"
                    && name == "add"
                    && matches!(receiver.as_ref(), Expr::State(state, Type::ComponentRef { name, .. }) if state == "stepperRef" && name == "QuantityStepper")
                    && *return_type
                        == Type::Result(
                            Box::new(Type::Numeric(NumericType::Int32)),
                            Box::new(Type::ComponentRefError(Box::new(Type::Void))),
                        );
            }
        },
    );
    assert!(attached, "the component call should retain its typed ref");
    assert!(
        typed_call,
        "the method call should carry its typed Result return"
    );

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(
        swift.contains("public final class NexaQuantityStepperRef"),
        "{swift}"
    );
    assert!(swift.contains("public func nexa_fn_add(_ nexa_amount: Int32) -> Result<Int32, NexaComponentRefError<Never>>"));
    assert!(swift.contains("nexaComponentRef?.attachNexa_fn_add"));
    assert!(swift.contains("nexaComponentRef: nexa_stepperRef"));

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(kotlin.contains("public class NexaQuantityStepperRef"));
    assert!(kotlin.contains(
        "public fun nexa_fn_add(nexa_amount: Int): NexaResult<Int, NexaComponentRefError<Unit>>"
    ));
    assert!(kotlin.contains("nexaComponentRef?.attachNexa_fn_add"));
    assert!(kotlin.contains("nexa_stepperRef"));
}

#[test]
fn documented_reusable_component_ref_example_compiles() {
    compile(
        r#"
        component QuantityStepper() {
            state quantity: Int32 = 1

            fn add(amount: Int32) -> Void {
                quantity = quantity + amount
            }

            body { Text("Quantity: \(quantity)") }
        }

        app Cart {
            let stepperRef = ComponentRef<QuantityStepper>()
            body {
                Column {
                    QuantityStepper(ref: stepperRef)
                    Button("Add two") { stepperRef.add(amount: 2) }
                }
            }
        }
        "#,
    )
    .expect("the documented reusable component ref example should compile");
}

#[test]
fn reusable_component_refs_can_return_app_defined_typed_failures() {
    let module = compile(
        r#"
        enum ProfileLookupError { missing, offline }

        component ProfileCard() {
            fn loadName() -> Result<String, ProfileLookupError> {
                return Err(ProfileLookupError.missing)
            }

            async fn loadNameAsync() -> Result<String, ProfileLookupError> {
                return Err(ProfileLookupError.offline)
            }

            body { Text("Profile") }
        }

        app ProfileScreen {
            let profileRef = ComponentRef<ProfileCard>()
            body {
                ProfileCard(ref: profileRef)
                Button("Load profile") { profileRef.loadName() }
                OnAppear async {
                    let asyncResult: Result<String, ComponentRefError<ProfileLookupError>> =
                        await profileRef.loadNameAsync()
                }
            }
        }
        "#,
    )
    .expect("a reusable ref method should carry a Nexa enum failure");

    let component = module
        .components
        .iter()
        .find(|component| component.name == "ProfileCard")
        .expect("the referenced component should remain reachable");
    assert_eq!(
        component.ref_methods[0].return_type,
        Type::Result(
            Box::new(Type::String),
            Box::new(Type::Enum("ProfileLookupError".to_owned()))
        )
    );
    let mut ref_failure = None;
    walk_ir(&module.body, &mut |_| {}, &mut |expression| {
        if let Expr::NativeCall {
            namespace,
            name,
            return_type,
            ..
        } = expression
            && namespace == "Nexa"
            && name == "loadName"
        {
            ref_failure = Some(return_type.clone());
        }
    });
    assert_eq!(
        ref_failure,
        Some(Type::Result(
            Box::new(Type::String),
            Box::new(Type::ComponentRefError(Box::new(Type::Enum(
                "ProfileLookupError".to_owned()
            ))))
        ))
    );

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(
        swift.contains(
            "public func nexa_fn_loadName() -> Result<String, NexaComponentRefError<NexaProfileLookupError>>"
        ),
        "{swift}"
    );
    assert!(swift.contains(".failure(.failure(failure))"), "{swift}");
    assert!(
        swift.contains("case .failure(let failure): return .failure(.failure(failure))"),
        "{swift}"
    );
    assert!(
        swift.contains("public func nexa_fn_loadNameAsync() async -> Result<String, NexaComponentRefError<NexaProfileLookupError>>"),
        "{swift}"
    );
    assert!(swift.contains("switch await action()"), "{swift}");

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(
        kotlin.contains(
            "public fun nexa_fn_loadName(): NexaResult<String, NexaComponentRefError<NexaProfileLookupError>>"
        ),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("NexaComponentRefError.Failure(outcome.error as NexaProfileLookupError)"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("public suspend fun nexa_fn_loadNameAsync(): NexaResult<String, NexaComponentRefError<NexaProfileLookupError>>"),
        "{kotlin}"
    );
    assert!(kotlin.contains("when (val outcome = action())"), "{kotlin}");
}

#[test]
fn reusable_component_ref_does_not_mix_throws_with_result_failures() {
    let error = compile(
        r#"
        enum LookupError { missing }
        component ProfileCard() {
            fn loadName() -> Result<String, LookupError> throws Video.PlayerFailure {
                return Err(LookupError.missing)
            }
            body { Text("Profile") }
        }
        app ProfileScreen {
            let profileRef = ComponentRef<ProfileCard>()
            body { ProfileCard(ref: profileRef) }
        }
        "#,
    )
    .expect_err("a ref method must choose one failure style");
    assert!(
        error
            .to_string()
            .contains("cannot use both `throws` and a `Result` return type"),
        "{error}"
    );
}

#[test]
fn reusable_component_refs_require_declared_methods_and_matching_types() {
    let no_methods = compile(
        r#"
        component LabelOnly() { body { Text("Label") } }
        app MissingMethods {
            let labelRef = ComponentRef<LabelOnly>()
            body { LabelOnly(ref: labelRef) }
        }
        "#,
    )
    .expect_err("a component without methods should not expose a ref");
    assert!(
        no_methods
            .to_string()
            .contains("does not declare ref methods")
    );

    let mismatch = compile(
        r#"
        component CounterControl() {
            fn reset() -> Void { }
            body { Text("Counter") }
        }
        app WrongRef {
            let listRef = ComponentRef<FastList>()
            body { CounterControl(ref: listRef) }
        }
        "#,
    )
    .expect_err("component refs must match the declared component type");
    assert!(
        mismatch
            .to_string()
            .contains("expected ComponentRef<Nexa.CounterControl>"),
        "{mismatch}"
    );
}

#[test]
fn component_ref_rejects_bad_targets_methods_and_mismatched_attachments() {
    let unknown_target = compile(
        r#"
        app UnknownComponentRef {
            let ref = ComponentRef<Missing.Widget>()
            body { Text("Missing") }
        }
        "#,
    )
    .expect_err("component ref target types must be declared");
    assert!(
        unknown_target
            .to_string()
            .contains("unknown native component ref target")
    );

    let mismatched_ref = compile(
        r#"
        app MismatchedComponentRef {
            let listRef = ComponentRef<FastList>()
            let otherListRef = ComponentRef<FastList>()
            body {
                FastList(count: 1, ref: listRef) { index in Text(index) }
                Button("Top") { otherListRef.scrollToIndex(index: "bad") }
            }
        }
        "#,
    );
    assert!(mismatched_ref.is_err());

    let unknown_method = compile(
        r#"
        app UnknownComponentRefMethod {
            let listRef = ComponentRef<FastList>()
            body { Button("Top") { listRef.scrollToUnknown() } }
        }
        "#,
    )
    .expect_err("FastList refs should reject methods outside the typed API");
    assert!(
        unknown_method
            .to_string()
            .contains("unknown FastList ref method")
    );
}

#[test]
fn documented_fast_list_component_ref_example_compiles() {
    compile(
        r#"
        struct OrderSummary {
            id: Int32,
            itemName: String,
            status: String,
        }

        app OrderQueue {
            let queueRef = ComponentRef<FastList>()
            state orders: Array<OrderSummary> = [
                OrderSummary(701, "Travel mug", "Packing"),
                OrderSummary(702, "Notebook set", "Ready")
            ]

            body {
                Column(spacing: 8) {
                    Button("Show the last order") {
                        queueRef.scrollToIndex(index: orders.count - 1)
                    }
                    Button("Return to the first order") {
                        queueRef.scrollToTop()
                    }
                    FastList(orders, key: .id, ref: queueRef) { order, index in
                        Row(spacing: 8) {
                            Text("\\(index + 1). \\(order.itemName)")
                            Spacer()
                            Text(order.status)
                        }
                    }
                }
            }
        }
        "#,
    )
    .expect("the published FastList ref example should compile");
}

#[test]
fn plugin_component_refs_validate_and_lower_typed_sync_async_methods() {
    let project = TestProject::new("nexa-plugin-component-ref");
    let plugin = project.join("video");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        "plugin { schema: 2 id: \"dev.example.video\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
    )
    .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        r#"
            error PlayerFailure { unavailable }
            native component VideoView {
                prop source: String
                fn seek(position: Float64) -> String throws PlayerFailure
                async fn currentSource() -> String throws PlayerFailure
            }
            "#,
    )
    .expect("plugin contract should be written");
    let entry = project.join("main.nx");
    fs::write(
        &entry,
        r#"
            plugin "video" as Video
            app ComponentRefPlugin {
                let videoRef = ComponentRef<Video.VideoView>()
                body {
                    OnAppear async {
                        await videoRef.currentSource()
                    }
                    Column {
                        Video.VideoView(source: "file:///lesson.mp4", ref: videoRef)
                        Button("Seek") { videoRef.seek(position: 12.5) }
                    }
                }
            }
            "#,
    )
    .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .unwrap_or_else(|error| {
                panic!("plugin component refs should compile for {target:?}: {error}")
            })
            .module;
        assert!(matches!(
            module.states.iter().find(|state| state.name == "videoRef"),
            Some(state) if state.ty == Type::ComponentRef {
                namespace: "Video".to_owned(),
                name: "VideoView".to_owned(),
            }
        ));

        let mut methods: Vec<(String, Type, bool, bool)> = Vec::new();
        let mut collect_method = |expression: &Expr| {
            if let Expr::NativeCall {
                receiver: Some(receiver),
                namespace,
                name,
                return_type,
                is_async,
                is_throwing,
                ..
            } = expression
                && namespace == "Video"
                && matches!(receiver.as_ref(), Expr::State(state, Type::ComponentRef { .. }) if state == "videoRef")
            {
                methods.push((name.clone(), return_type.clone(), *is_async, *is_throwing));
            }
        };
        walk_ir(&module.body, &mut |_| {}, &mut collect_method);
        if let Some(actions) = module.on_appear.as_deref() {
            walk_actions(actions, &mut collect_method);
        }
        assert!(methods.iter().any(|(name, ty, is_async, is_throwing)| {
            name == "seek"
                && *ty
                    == Type::Result(
                        Box::new(Type::String),
                        Box::new(Type::ComponentRefError(Box::new(Type::Plugin {
                            namespace: "Video".to_owned(),
                            name: "PlayerFailure".to_owned(),
                        }))),
                    )
                && !is_async
                && !is_throwing
        }));
        assert!(methods.iter().any(|(name, ty, is_async, is_throwing)| {
            name == "currentSource"
                && matches!(ty, Type::Result(success, failure)
                if **success == Type::String
                    && **failure == Type::ComponentRefError(Box::new(Type::Plugin {
                        namespace: "Video".to_owned(),
                        name: "PlayerFailure".to_owned(),
                    })))
                && *is_async
                && !is_throwing
        }));
    }
}

#[test]
fn reusable_component_ref_methods_support_async_and_typed_plugin_failures() {
    let project = TestProject::new("nexa-reusable-component-ref-async-throws");
    let plugin = project.join("video");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        "plugin { schema: 2 id: \"dev.example.video\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
    )
    .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        r#"
            error PlayerFailure { unavailable }
            native class Player {
                init(id: String)
                fn seek(position: Float64) -> String throws PlayerFailure
                async fn currentSource() -> String throws PlayerFailure
            }
            "#,
    )
    .expect("plugin contract should be written");
    let entry = project.join("main.nx");
    fs::write(
        &entry,
        r#"
            plugin "video" as Video
            component PlayerControls(player: Video.Player) {
                fn seek(position: Float64) -> String throws Video.PlayerFailure {
                    return player.seek(position)
                }

                async fn currentSource() -> String throws Video.PlayerFailure {
                    return await player.currentSource()
                }

                body { Text("Player controls") }
            }

            app PlayerScreen {
                let player = Video.Player("lesson-player")
                let playerRef = ComponentRef<PlayerControls>()
                body {
                    PlayerControls(player: player, ref: playerRef)
                    OnAppear async { await playerRef.currentSource() }
                    Button("Seek") { playerRef.seek(position: 12.5) }
                }
            }
            "#,
    )
    .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .unwrap_or_else(|error| {
                panic!("async and throwing component refs should compile for {target:?}: {error}")
            })
            .module;
        let component = module
            .components
            .iter()
            .find(|component| component.name == "PlayerControls")
            .expect("the referenced custom component should remain reachable");
        let sync_method = component
            .ref_methods
            .iter()
            .find(|method| method.name.ends_with(".seek"))
            .expect("sync throwing method should be exposed");
        let async_method = component
            .ref_methods
            .iter()
            .find(|method| method.name.ends_with(".currentSource"))
            .expect("async throwing method should be exposed");
        assert!(sync_method.is_throwing);
        assert!(async_method.is_throwing);
        assert!(async_method.is_async);
        assert_eq!(
            async_method.failure_type,
            Some(Type::Plugin {
                namespace: "Video".to_owned(),
                name: "PlayerFailure".to_owned(),
            })
        );
        let expected_ref_result = Type::Result(
            Box::new(Type::String),
            Box::new(Type::ComponentRefError(Box::new(Type::Plugin {
                namespace: "Video".to_owned(),
                name: "PlayerFailure".to_owned(),
            }))),
        );
        let mut typed_ref_call = None;
        let mut inspect_call = |expression: &Expr| {
            if let Expr::NativeCall {
                namespace,
                name,
                return_type,
                ..
            } = expression
                && namespace == "Nexa"
                && name == "currentSource"
            {
                typed_ref_call = Some(return_type.clone());
            }
        };
        walk_ir(&module.body, &mut |_| {}, &mut inspect_call);
        if let Some(actions) = &module.on_appear {
            walk_actions(actions, &mut inspect_call);
        }
        assert_eq!(
            typed_ref_call,
            Some(expected_ref_result),
            "ref calls should return the declared failure type"
        );

        let swift = nexa_backend_swift::SwiftBackend.generate(&module);
        let (kotlin, _) =
            nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
        assert!(
            swift.contains("NexaComponentRefError<PlayerFailure>"),
            "{swift}"
        );
        assert!(
            swift.contains("async -> Result<String, NexaComponentRefError<PlayerFailure>>"),
            "{swift}"
        );
        assert!(swift.contains("async throws -> String"), "{swift}");
        assert!(
            kotlin.contains("NexaComponentRefError<PlayerFailure>"),
            "{kotlin}"
        );
        assert!(
            kotlin.contains("public suspend fun nexa_fn_currentSource"),
            "{kotlin}"
        );
        assert!(
            kotlin.contains("suspend ( ) -> String") || kotlin.contains("suspend () -> String"),
            "{kotlin}"
        );
    }

    fs::write(
        &entry,
        r#"
            plugin "video" as Video
            component PlayerControls(player: Video.Player) {
                fn seek(position: Float64) -> String throws {
                    return player.seek(position)
                }
                body { Text("Player controls") }
            }
            app PlayerScreen {
                let player = Video.Player("lesson-player")
                let controlsRef = ComponentRef<PlayerControls>()
                body { PlayerControls(player: player, ref: controlsRef) }
            }
            "#,
    )
    .expect("untyped failure case should be written");
    let untyped = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .err()
        .expect("throwing reusable methods must declare a concrete failure type");
    assert!(
        untyped
            .to_string()
            .contains("must name their plugin error type")
    );
}

#[test]
fn custom_navigation_headers_can_disable_the_native_navigation_bar_on_both_targets() {
    let module = compile(
        r#"
        app CustomNavigationHeader {
            screen Home { Text("Home") }
            screen Details { Text("Custom header content") }
            body { NavigationStack(root: Home, showNavigationBar: false) }
        }
        "#,
    )
    .expect("custom-header navigation should type-check");
    assert!(matches!(
        module.body.first(),
        Some(Node::NavigationStack {
            show_navigation_bar: false,
            ..
        })
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    let swift_host = swift
        .split("struct NexaScreen0: View")
        .next()
        .expect("navigation host is emitted before screens");
    assert!(swift_host.contains(".toolbar(.hidden, for: .navigationBar)"));
    assert!(!swift_host.contains("NexaNavigationScreenPrimitive(title: \"Details\""));
    assert!(swift_host.contains(".ignoresSafeArea(.container, edges: .all)"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("nexaShowNavigationBar = false"));
    assert!(kotlin.contains("if (nexaIsRoot || !nexaShowNavigationBar)"));
}

#[test]
fn navigation_title_defaults_and_explicit_title_modes_and_alignment_compile() {
    let inline = compile(
        r#"
        app InlineNavigationTitle {
            screen Home { Text("Home") }
            body { NavigationStack(root: Home) }
        }
        "#,
    )
    .expect("the default title mode should be inline");
    assert!(matches!(
        inline.body.first(),
        Some(Node::NavigationStack {
            title_mode: nexa_ir::NavigationTitleMode::Inline,
            title_alignment: nexa_ir::NavigationTitleAlignment::Automatic,
            ..
        })
    ));

    let large = compile(
        r#"
        app LargeNavigationTitle {
            screen Home { Text("Home") }
            body { NavigationStack(root: Home, titleMode: Large) }
        }
        "#,
    )
    .expect("Large should opt into the native expanding title mode");
    assert!(matches!(
        large.body.first(),
        Some(Node::NavigationStack {
            title_mode: nexa_ir::NavigationTitleMode::Large,
            ..
        })
    ));

    let centered = compile(
        r#"
        app CenteredNavigationTitle {
            screen Home { Text("Home") }
            body { NavigationStack(root: Home, titleAlignment: Center) }
        }
        "#,
    )
    .expect("Center should preserve the native centered inline title");
    assert!(matches!(
        centered.body.first(),
        Some(Node::NavigationStack {
            title_mode: nexa_ir::NavigationTitleMode::Inline,
            title_alignment: nexa_ir::NavigationTitleAlignment::Center,
            ..
        })
    ));

    let leading = compile(
        r#"
        app LeadingNavigationTitle {
            screen Home { Text("Home") }
            body { NavigationStack(root: Home, titleAlignment: Leading) }
        }
        "#,
    )
    .expect("Leading should explicitly align the title beside the back control");
    assert!(matches!(
        leading.body.first(),
        Some(Node::NavigationStack {
            title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            ..
        })
    ));

    let invalid_alignment = compile(
        r#"
        app InvalidNavigationTitleAlignment {
            screen Home { Text("Home") }
            body { NavigationStack(root: Home, titleAlignment: Trailing) }
        }
        "#,
    )
    .expect_err("unsupported title alignments should be diagnosed");
    assert!(invalid_alignment.message.contains("titleAlignment"));
}

#[test]
fn arithmetic_lowers_with_numeric_types_and_folds_constants() {
    let module = compile(
        r#"
        app Arithmetic {
            state result: Int32 = 0

            fn computed() -> Int32 {
                return 2 + 3 * 4 - 10 / 2 % 3
            }

            body {
                Button("Calculate") {
                    result = -((result + 1) * 2 - 3) / 2 % 2
                    result *= computed()
                }
            }
        }
        "#,
    )
    .expect("typed arithmetic should compile");

    let [function] = module.functions.as_slice() else {
        panic!("the called helper should remain reachable");
    };
    assert!(matches!(
        &function.body,
        Expr::Number { raw, ty: NumericType::Int32 } if raw == "12"
    ));

    let Node::Button { actions, .. } = &module.body[0] else {
        panic!("expected the arithmetic button");
    };
    let [
        Action::Assign {
            value:
                Expr::Arithmetic {
                    op: ArithmeticOp::Remainder,
                    ty: NumericType::Int32,
                    left,
                    ..
                },
            ..
        },
        Action::Assign {
            value:
                Expr::Arithmetic {
                    op: ArithmeticOp::Multiply,
                    ty: NumericType::Int32,
                    left: compound_left,
                    right: call,
                },
            ..
        },
    ] = actions.as_slice()
    else {
        panic!("arithmetic and compound assignment should lower to typed IR");
    };
    assert!(matches!(
        left.as_ref(),
        Expr::Arithmetic {
            op: ArithmeticOp::Divide,
            ty: NumericType::Int32,
            left,
            ..
        } if matches!(left.as_ref(), Expr::Negate { ty: NumericType::Int32, .. })
    ));
    assert!(matches!(
        compound_left.as_ref(),
        Expr::State(name, Type::Numeric(NumericType::Int32)) if name == "result"
    ));
    assert!(matches!(call.as_ref(), Expr::Call { name, .. } if name == "computed"));
}

#[test]
fn awaited_numeric_calls_lower_inside_arithmetic_expressions() {
    let module = compile(
        r#"
        app NestedAwait {
            async fn first() -> Int32 {
                return 20
            }

            async fn second() -> Int32 {
                return 22
            }

            state total: Int32 = 0

            body {
                OnAppear async {
                    total = (await first()) + (await second())
                }
                Text(total)
            }
        }
        "#,
    )
    .expect("awaited async results should retain their value type in arithmetic");

    let Some(
        [
            Action::Assign {
                value: Expr::Add(left, right, NumericType::Int32),
                ..
            },
        ],
    ) = module.on_appear.as_deref()
    else {
        panic!("expected one typed arithmetic assignment in OnAppear async");
    };
    for (expression, expected_name) in [(left, "first"), (right, "second")] {
        assert!(matches!(
            expression.as_ref(),
            Expr::Await(call)
                if matches!(call.as_ref(), Expr::Call { name, is_async: true, .. } if name == expected_name)
        ));
    }
}

#[test]
fn text_input_keyboard_ergonomics_lower_to_typed_native_options() {
    let module = compile(
        r#"
        app KeyboardErgonomics {
            state email: String = ""
            state password: String = ""
            state focused: Bool = false

            body {
                TextInput(
                    value: email,
                    placeholder: "Email",
                    keyboardType: Email,
                    autofill: Username,
                    returnKeyType: Next,
                    focused: focused,
                ) { }
                TextInput(
                    value: password,
                    placeholder: "Password",
                    isSecure: true,
                    autofill: Password,
                    returnKeyType: Done,
                ) { }
                Button("Dismiss") { Keyboard.dismiss() }
            }
        }
        "#,
    )
    .expect("keyboard ergonomics should compile");

    assert!(matches!(
        &module.body[0],
        Node::TextInput {
            keyboard: nexa_ir::KeyboardType::Email,
            autofill: Some(AutofillType::Username),
            return_key: Some(ReturnKeyType::Next),
            ..
        }
    ));
    assert!(matches!(
        &module.body[1],
        Node::TextInput {
            secure: true,
            autofill: Some(AutofillType::Password),
            return_key: Some(ReturnKeyType::Done),
            ..
        }
    ));
    assert!(matches!(
        &module.body[2],
        Node::Button { actions, .. }
            if matches!(actions.as_slice(), [Action::Expression(Expr::NativeCall {
                namespace,
                name,
                return_type: Type::Void,
                is_async: false,
                ..
            })] if namespace == "Keyboard" && name == "dismiss")
    ));
    assert!(nexa_ir::capabilities::analyze(&module).uses_keyboard_api);
}

#[test]
fn text_input_static_style_lowers_to_typed_native_properties() {
    let module = compile(
        r##"
        app SearchFieldStyle {
            state query: String = ""
            body {
                TextInput(
                    value: query,
                    placeholder: "Search songs",
                    fontSize: 16.38,
                    fontFamily: "Gilroy-Regular",
                    textColor: "#FFFFFF",
                    placeholderColor: "#9F9F9F",
                    clearable: true,
                )
            }
        }
        "##,
    )
    .expect("static text input styles should compile");

    let Node::TextInput {
        font_size,
        font_family,
        text_color,
        placeholder_color,
        clearable,
        ..
    } = &module.body[0]
    else {
        panic!("expected a lowered TextInput");
    };
    assert_eq!(*font_size, Some(16.38));
    assert_eq!(font_family.as_deref(), Some("Gilroy-Regular"));
    assert_eq!(
        *text_color,
        Some(nexa_ir::ColorValue::Static(nexa_ir::Color {
            red: 255,
            green: 255,
            blue: 255,
            alpha: 255,
        }))
    );
    assert_eq!(
        *placeholder_color,
        Some(nexa_ir::ColorValue::Static(nexa_ir::Color {
            red: 159,
            green: 159,
            blue: 159,
            alpha: 255,
        }))
    );
    assert!(*clearable);
}

#[test]
fn text_input_focus_action_lowers_to_a_typed_native_callback() {
    let module = compile(
        r#"
        app SearchFieldFocus {
            state query: String = ""
            state focused: Bool = false
            state didFocus: Bool = false
            body {
                TextInput(value: query, placeholder: "Search", focused: focused)
                    .onFocus { didFocus = true }
            }
        }
        "#,
    )
    .expect("a focus callback with a tracked focus binding should compile");

    assert!(matches!(
        &module.body[0],
        Node::TextInput {
            focused: Some(focused),
            on_focus: Some(actions),
            ..
        } if focused == "focused"
            && matches!(actions.as_slice(), [Action::Assign { name, value: Expr::Bool(true) }] if name == "didFocus")
    ));

    let missing_focus_binding = r#"
        app SearchFieldFocus {
            state query: String = ""
            state didFocus: Bool = false
            body { TextInput(value: query, placeholder: "Search").onFocus { didFocus = true } }
        }
    "#;
    assert!(
        compile(missing_focus_binding)
            .expect_err("focus callbacks need a cross-platform focus binding")
            .message
            .contains("requires a `focused` state binding")
    );
}

#[test]
fn currency_formatting_lowers_to_a_typed_core_call() {
    let module = compile(
        r#"
        app CurrencyFormatting {
            state price: String = Number.formatCurrency(amount: 1234.5, currencyCode: "EUR")

            body {
                Text(price)
            }
        }
        "#,
    )
    .expect("currency formatting should compile");

    assert!(matches!(
        &module.states[0].initial,
        Expr::NativeCall {
            namespace,
            name,
            arguments,
            return_type: Type::String,
            is_async: false,
            ..
        } if namespace == "Number"
            && name == "formatCurrency"
            && matches!(arguments.as_slice(), [
                (amount_name, Expr::Number { ty: NumericType::Float64, .. }),
                (currency_name, Expr::String(code)),
            ] if amount_name == "amount" && currency_name == "currencyCode" && code == "EUR")
    ));
    assert!(nexa_ir::capabilities::analyze(&module).uses_number_formatting);
}

#[test]
fn core_apis_accept_positional_and_mixed_arguments() {
    let module = compile(
        r#"
        app PositionalCoreApis {
            state saved: String? = Storage.getString("theme")
            state price: String = Number.formatCurrency(12.5, "EUR")

            body {
                Text(saved)
                Text(price)
                Button("Save") { Storage.setString("theme", value: "dark") }
                Button("Delete") { Storage.delete("theme") }
            }
        }
        "#,
    )
    .expect("core APIs should accept positional and trailing named arguments");

    assert!(matches!(
        &module.states[0].initial,
        Expr::NativeCall { namespace, name, arguments, .. }
            if namespace == "Storage" && name == "getString"
                && matches!(arguments.as_slice(), [(key, Expr::String(value))]
                    if key == "key" && value == "theme")
    ));
    assert!(matches!(
        &module.states[1].initial,
        Expr::NativeCall { namespace, name, arguments, .. }
            if namespace == "Number" && name == "formatCurrency"
                && matches!(arguments.as_slice(), [
                    (amount, Expr::Number { raw, .. }),
                    (currency, Expr::String(code))
                ] if amount == "amount" && raw == "12.5"
                    && currency == "currencyCode" && code == "EUR")
    ));
}

#[test]
fn storage_reads_and_writes_typed_structs_and_collections() {
    let module = compile(
        r#"
        struct ReaderSettings {
            fontSize: Float64,
            showPageNumbers: Bool,
        }
        app TypedStorage {
            state settings: ReaderSettings = Storage.get<ReaderSettings>(key: "reader.settings") ?? ReaderSettings(18.0, true)
            state recentBooks: Array<String> = Storage.get<Array<String>>(key: "reader.recent-books") ?? []
            state savedSettings: ReaderSettings? = Storage.get(key: "reader.saved-settings")
            body {
                Text("Font size: \(settings.fontSize)")
                Text("Saved settings loaded: \(savedSettings != null)")
                Button("Save") {
                    Storage.set(key: "reader.settings", value: settings)
                    Storage.set(key: "reader.recent-books", value: recentBooks)
                }
            }
        }
        "#,
    )
    .expect("typed storage should support structs and collections");

    assert!(matches!(
        &module.states[0].initial,
        Expr::Coalesce(left, _) if matches!(
            &**left,
            Expr::NativeCall { namespace, name, codecs, return_type, .. }
                if namespace == "Storage" && name == "get"
                    && codecs.len() == 1 && codecs[0].ty == module.states[0].ty
                    && codecs[0].decodes
                    && return_type == &Type::Optional(Box::new(module.states[0].ty.clone()))
        )
    ));
    assert!(matches!(
        &module.states[1].initial,
        Expr::Coalesce(left, _) if matches!(
            &**left,
            Expr::NativeCall { namespace, name, codecs, .. }
                if namespace == "Storage" && name == "get"
                    && codecs.len() == 1 && codecs[0].ty == module.states[1].ty
                    && codecs[0].decodes
        )
    ));
    let Type::Optional(saved_settings_type) = &module.states[2].ty else {
        panic!("savedSettings should remain optional");
    };
    assert!(matches!(
        &module.states[2].initial,
        Expr::NativeCall { namespace, name, codecs, return_type, .. }
            if namespace == "Storage" && name == "get"
                && codecs.len() == 1 && codecs[0].ty == **saved_settings_type
                && codecs[0].decodes
                && return_type == &module.states[2].ty
    ));
    let Node::Button { actions, .. } = &module.body[2] else {
        panic!("expected the Save button");
    };
    assert!(matches!(
        actions.as_slice(),
        [
            Action::Expression(Expr::NativeCall { namespace: first_namespace, name: first_name, codecs: first_codecs, .. }),
            Action::Expression(Expr::NativeCall { namespace: second_namespace, name: second_name, codecs: second_codecs, .. }),
        ] if first_namespace == "Storage" && first_name == "set"
            && first_codecs.len() == 1 && !first_codecs[0].decodes
            && second_namespace == "Storage" && second_name == "set"
            && second_codecs.len() == 1 && !second_codecs[0].decodes
    ));
    let capabilities = nexa_ir::capabilities::analyze(&module);
    assert!(capabilities.uses_storage_api);
    assert!(capabilities.uses_json_api);
}

#[test]
fn typed_storage_rejects_mismatched_and_unsupported_types() {
    let mismatch = compile(
        r#"
        app InvalidStorageWrite {
            body { Button("Save") { Storage.set<Int64>(key: "count", value: "many") } }
        }
        "#,
    )
    .expect_err("typed storage must validate its value type");
    assert!(
        mismatch
            .to_string()
            .contains("Storage.set` type argument is Int64")
    );

    let unsupported = compile(
        r#"
        class Account(val id: Int64) {}
        app InvalidStorageClass {
            body { Button("Save") { Storage.set<Account>(key: "account", value: Account(1)) } }
        }
        "#,
    )
    .expect_err("native classes cannot be serialized as JSON values");
    assert!(unsupported.to_string().contains("JSON"), "{unsupported}");

    let unexpected_argument = compile(
        r#"
        app InvalidStorageReadArguments {
            state value: String? = Storage.get<String>(key: "reader.name", value: "unused")
            body { Text("Reader") }
        }
        "#,
    )
    .expect_err("typed reads must reject arguments that are not part of their signature");
    assert!(
        unexpected_argument
            .to_string()
            .contains("unknown argument `value`")
    );
}

#[test]
fn secure_storage_scopes_isolate_reads_writes_and_clear() {
    let module = compile(
        r#"
        app ScopedVault {
            state task: TaskHandle? = null
            state token: String = ""
            body {
                Button("Save token") {
                    Task.launch(handle: task, executor: TaskExecutor.Main) {
                        try {
                            let saved: String? = await SecureStorage.get(key: "refresh-token", scope: "auth")
                            await SecureStorage.set(key: "refresh-token", value: saved ?? token, scope: "auth")
                            await SecureStorage.delete(key: "refresh-token", scope: "auth")
                            await SecureStorage.clear(scope: "auth")
                            await SecureStorage.delete(key: "legacy-token")
                            await SecureStorage.clear()
                        } catch { }
                    }
                }
            }
        }
        "#,
    )
    .expect("scoped secure storage calls should type-check");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    for expected in [
        "NexaSecureStorage.get(\"refresh-token\", \"auth\")",
        "NexaSecureStorage.set(\"refresh-token\", (nexa_saved ?? nexa_token), \"auth\")",
        "NexaSecureStorage.delete(\"refresh-token\", \"auth\")",
        "NexaSecureStorage.clear(\"auth\")",
        "NexaSecureStorage.delete(\"legacy-token\", nil)",
        "NexaSecureStorage.clear(nil)",
        "scopedServicePrefix",
        "storedService.hasPrefix(scopedServicePrefix)",
    ] {
        assert!(
            swift.contains(expected),
            "Swift output should contain `{expected}`"
        );
    }

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    for expected in [
        "NexaSecureStorage.get(NexaRuntime.context(), \"refresh-token\", \"auth\")",
        "NexaSecureStorage.set(NexaRuntime.context(), \"refresh-token\",",
        "NexaSecureStorage.delete(NexaRuntime.context(), \"refresh-token\", \"auth\")",
        "NexaSecureStorage.clear(NexaRuntime.context(), \"auth\")",
        "NexaSecureStorage.delete(NexaRuntime.context(), \"legacy-token\", null)",
        "NexaSecureStorage.clear(NexaRuntime.context(), null)",
        "storageIdentity(key: String, scope: String?)",
        "scopedDirectory(",
        "deleteRecursively()",
    ] {
        assert!(
            kotlin.contains(expected),
            "Kotlin output should contain `{expected}`"
        );
    }
}

#[test]
fn json_parse_and_stringify_lower_with_concrete_value_codecs() {
    let module = compile(
        r#"
        struct UserProfile {
            name: String,
            age: Int32,
        }

        app JsonExample {
            fn encode(profile: UserProfile) -> String {
                return Json.stringify(value: profile)
            }

            fn decode(raw: String) -> Result<UserProfile, JsonError> {
                return Json.parse<UserProfile>(raw: raw)
            }

            state encoded: String = encode(UserProfile("Ada", 37))
            state status: String = ""

            body {
                Text(encoded)
                Button("Decode") {
                    status = Json.stringify(value: decode(encoded))
                }
            }
        }
        "#,
    )
    .expect("typed JSON calls should compile");

    let encode = module
        .functions
        .iter()
        .find(|function| function.name == "encode")
        .expect("the encode function should remain reachable");
    assert!(matches!(
        &encode.body,
        Expr::NativeCall {
            namespace,
            name,
            codecs,
            return_type: Type::String,
            ..
        } if namespace == "Json"
            && name == "stringify"
            && matches!(codecs.as_slice(), [codec]
                if !codec.decodes && matches!(&codec.ty, Type::Struct { name, .. } if name == "UserProfile"))
    ));

    let decode = module
        .functions
        .iter()
        .find(|function| function.name == "decode")
        .expect("the decode function should remain reachable");
    assert!(matches!(
        &decode.body,
        Expr::NativeCall {
            namespace,
            name,
            codecs,
            return_type: Type::Result(value, error),
            ..
        } if namespace == "Json"
            && name == "parse"
            && matches!(codecs.as_slice(), [codec]
                if codec.decodes && matches!(&codec.ty, Type::Struct { name, .. } if name == "UserProfile"))
            && matches!(value.as_ref(), Type::Struct { name, .. } if name == "UserProfile")
            && matches!(error.as_ref(), Type::Enum(name) if name == "JsonError")
    ));
    assert!(
        module
            .enums
            .iter()
            .any(|declaration| declaration.name == "JsonError")
    );
    assert!(nexa_ir::capabilities::analyze(&module).uses_json_api);
}

#[test]
fn json_apis_accept_positional_values() {
    let module = compile(
        r#"
        struct Profile { name: String }
        app PositionalJson {
            state decoded: Result<Profile, JsonError> = Json.parse<Profile>("{}")

            body {
                Text(Json.stringify(Profile("Ada")))
                Button("Parse") { decoded = Json.parse<Profile>("{}") }
            }
        }
        "#,
    )
    .expect("JSON core APIs should accept simple positional values");

    assert!(matches!(
        &module.states[0].initial,
        Expr::NativeCall { namespace, name, arguments, .. }
            if namespace == "Json" && name == "parse"
                && arguments.len() == 1 && arguments[0].0 == "raw"
    ));
    let Node::Text { value, .. } = &module.body[0] else {
        panic!("expected encoded profile text");
    };
    assert!(matches!(
        value,
        Expr::NativeCall { namespace, name, arguments, .. }
            if namespace == "Json" && name == "stringify"
                && arguments.len() == 1 && arguments[0].0 == "value"
    ));
    let Node::Button { actions, .. } = &module.body[1] else {
        panic!("expected Parse button");
    };
    assert!(matches!(
        actions.as_slice(),
        [Action::Assign { value: Expr::NativeCall { namespace, name, arguments, .. }, .. }]
            if namespace == "Json" && name == "parse"
                && arguments.len() == 1 && arguments[0].0 == "raw"
    ));
}

#[test]
fn json_rejects_non_string_map_keys_and_unknown_types() {
    let non_string_keys = compile(
        r#"
        app InvalidJsonMap {
            let values: Map<Int32, String> = [:]
            let encoded: String = Json.stringify(value: values)
            body { Text(encoded) }
        }
        "#,
    )
    .expect_err("JSON object maps must use string keys");
    assert!(
        non_string_keys
            .to_string()
            .contains("JSON maps require `String` keys")
    );

    let unknown_type = compile(
        r#"
        app InvalidJsonType {
            let decoded: Result<MissingType, JsonError> = Json.parse<MissingType>(raw: "{}")
            body { Text("invalid") }
        }
        "#,
    )
    .expect_err("JSON codecs need a declared concrete type");
    assert!(
        unknown_type.to_string().contains("MissingType"),
        "{unknown_type}"
    );
}

#[test]
fn crypto_calls_lower_to_typed_core_calls() {
    let module = compile(
        r#"
        app CryptoExample {
            let sha1: String = Crypto.sha1(text: "nexa")
            let sha256: String = Crypto.sha256(text: "nexa")
            let sha512: String = Crypto.sha512(text: "nexa")
            let signature: String = Crypto.hmacSha256(key: "secret", message: "payload")
            let random: String = Crypto.randomBytes(count: 16)

            body {
                Text(sha1 + sha256 + sha512 + signature + random)
            }
        }
        "#,
    )
    .expect("core cryptographic calls should compile");

    let calls = module
        .states
        .iter()
        .map(|state| &state.initial)
        .collect::<Vec<_>>();
    for (expression, (expected_name, expected_arguments)) in calls.iter().zip([
        ("sha1", &["text"][..]),
        ("sha256", &["text"][..]),
        ("sha512", &["text"][..]),
        ("hmacSha256", &["key", "message"][..]),
        ("randomBytes", &["count"][..]),
    ]) {
        assert!(
            matches!(
                expression,
                Expr::NativeCall {
                    namespace,
                    name: actual_name,
                    arguments,
                    return_type: Type::String,
                    is_async: false,
                    is_throwing: false,
                    ..
                } if namespace == "Crypto"
                    && actual_name == expected_name
                    && arguments.iter().map(|(argument, _)| argument.as_str()).collect::<Vec<_>>() == expected_arguments
            ),
            "expected Crypto.{expected_name} with {expected_arguments:?}, got {expression:?}"
        );
    }
    assert!(nexa_ir::capabilities::analyze(&module).uses_crypto_api);
}

#[test]
fn clipboard_calls_lower_to_typed_core_calls() {
    let module = compile(
        r#"
        app ClipboardExample {
            state source: String = "copied"
            state pasted: String = ""

            body {
                Button("Copy") { Clipboard.setText(text: source) }
                Button("Paste") { pasted = Clipboard.getText() ?? "" }
                Button("Check") { pasted = if Clipboard.hasText() { "yes" } else { "no" } }
            }
        }
        "#,
    )
    .expect("typed clipboard calls should compile");

    let Node::Button { actions: copy, .. } = &module.body[0] else {
        panic!("expected copy button");
    };
    assert!(matches!(
        copy.as_slice(),
        [Action::Expression(Expr::NativeCall {
            namespace,
            name,
            arguments,
            return_type: Type::Void,
            is_async: false,
            is_throwing: false,
            ..
        })] if namespace == "Clipboard"
            && name == "setText"
            && matches!(arguments.as_slice(), [(argument, Expr::State(source, Type::String))]
                if argument == "text" && source == "source")
    ));

    let Node::Button { actions: paste, .. } = &module.body[1] else {
        panic!("expected paste button");
    };
    assert!(matches!(
        paste.as_slice(),
        [Action::Assign { value, .. }]
            if matches!(value, Expr::Coalesce(left, _)
                if matches!(left.as_ref(), Expr::NativeCall {
                    namespace,
                    name,
                    return_type: Type::Optional(inner),
                    is_async: false,
                    ..
                } if namespace == "Clipboard" && name == "getText" && **inner == Type::String))
    ));

    let Node::Button { actions: check, .. } = &module.body[2] else {
        panic!("expected check button");
    };
    assert!(matches!(
        check.as_slice(),
        [Action::Assign { value, .. }]
            if matches!(value, Expr::Conditional { condition, .. }
                if matches!(condition.as_ref(), Expr::NativeCall {
                    namespace,
                    name,
                    return_type: Type::Bool,
                    is_async: false,
                    ..
                } if namespace == "Clipboard" && name == "hasText"))
    ));
    assert!(nexa_ir::capabilities::analyze(&module).uses_clipboard_api);
}

#[test]
fn app_storage_calls_lower_to_typed_synchronous_core_calls() {
    let module = compile(
        r#"
        app StorageExample {
            state saved: String = Storage.getString(key: "theme") ?? "system"

            body {
                Text(saved)
                Button("Save") { Storage.setString(key: "theme", value: saved) }
                Button("Delete") { Storage.delete(key: "theme") }
                Button("Clear") { Storage.clear() }
            }
        }
        "#,
    )
    .expect("typed app-private storage calls should compile");

    assert!(matches!(
        &module.states[0].initial,
        Expr::Coalesce(left, _)
            if matches!(left.as_ref(), Expr::NativeCall {
                namespace,
                name,
                arguments,
                return_type: Type::Optional(inner),
                is_async: false,
                is_throwing: false,
                ..
            } if namespace == "Storage"
                && name == "getString"
                && **inner == Type::String
                && matches!(arguments.as_slice(), [(key, Expr::String(value))]
                    if key == "key" && value == "theme"))
    ));

    for (index, name, expected_arguments) in [
        (1, "setString", &["key", "value"][..]),
        (2, "delete", &["key"][..]),
        (3, "clear", &[][..]),
    ] {
        let Node::Button { actions, .. } = &module.body[index] else {
            panic!("expected a storage action button");
        };
        assert!(matches!(
            actions.as_slice(),
            [Action::Expression(Expr::NativeCall {
                namespace,
                name: actual_name,
                arguments,
                return_type: Type::Void,
                is_async: false,
                is_throwing: false,
                ..
            })]
                if namespace == "Storage"
                    && actual_name == name
                    && arguments.iter().map(|(argument, _)| argument.as_str()).collect::<Vec<_>>() == expected_arguments
        ));
    }
    assert!(nexa_ir::capabilities::analyze(&module).uses_storage_api);
}

#[test]
fn page_pager_keeps_the_bound_selection_in_ir() {
    let module = compile(
        r#"
        app Onboarding {
            state page: Int32 = 0
            body {
                PagePager(selected: page) {
                    Tab(index: 0) { Text("Welcome") }
                    Tab(index: 1) { Text("Complete") }
                }
            }
        }
        "#,
    )
    .expect("page style is a typed AppBottomBar presentation");

    assert!(matches!(
        module.body.first(),
        Some(Node::PagePager { pages, .. }) if pages.len() == 2
    ));
}

#[test]
fn app_bottom_bar_and_controls_accept_runtime_hex_color_state() {
    let module = compile(
        r##"
        app ThemedTabs {
            state page: Int32 = 0
            state accent: String = "#D63031"
            body {
                AppBottomBar(selected: page, tint: accent) {
                    Tab(index: 0, label: "Home") {
                        Button("Add", tint: accent) { }
                        Icon(system: "add", description: "Add", size: 24, tint: accent)
                    }
                }
            }
        }
        "##,
    )
    .expect("runtime hexadecimal string state should feed native tints");

    let Some(Node::AppBottomBar {
        tint: Some(nexa_ir::ColorExpression::Dynamic(color)),
        tabs,
        ..
    }) = module.body.first()
    else {
        panic!("expected a dynamic AppBottomBar tint");
    };
    assert!(matches!(
        color.as_ref(),
        Expr::State(name, Type::String) if name == "accent"
    ));
    let [
        Node::Button {
            tint: Some(nexa_ir::ColorExpression::Dynamic(button_color)),
            ..
        },
        Node::SystemIcon {
            tint: nexa_ir::ColorExpression::Dynamic(icon_color),
            ..
        },
    ] = tabs[0].children.as_slice()
    else {
        panic!("expected a tinted button and icon in the custom tab bar");
    };
    assert!(matches!(
        button_color.as_ref(),
        Expr::State(name, Type::String) if name == "accent"
    ));
    assert!(matches!(
        icon_color.as_ref(),
        Expr::State(name, Type::String) if name == "accent"
    ));
}

#[test]
fn app_bottom_bar_custom_chrome_is_lowered_separately_from_destinations() {
    let module = compile(
        r#"
        app CustomTabs {
            state selectedTab: Int32 = 0
            body {
                AppBottomBar(selected: selectedTab) {
                    Tab(index: 0, label: "Home", icon: "home") {
                        Text("Home destination")
                    }
                    Tab(index: 1, label: "Profile", icon: "person") {
                        Text("Profile destination")
                    }
                    TabBar {
                        Row {
                            Pressable() { Text("Custom home") }.onTap { selectedTab = 0 }
                        }
                    }
                }
            }
        }
        "#,
    )
    .expect("custom tab chrome should keep native AppBottomBar destinations");

    let Some(Node::AppBottomBar {
        tabs,
        custom_bar: Some(custom_bar),
        ..
    }) = module.body.first()
    else {
        panic!("expected custom bar and native destinations in the typed AppBottomBar");
    };
    assert_eq!(tabs.len(), 2);
    let [Node::Text { value, .. }] = tabs[0].children.as_slice() else {
        panic!("expected the Home destination text to be retained");
    };
    assert_eq!(source_text(value), Some("Home destination"));
    let mut custom_pressable_retained = false;
    walk_ir(
        custom_bar,
        &mut |node| custom_pressable_retained |= matches!(node, Node::Pressable { .. }),
        &mut |_| {},
    );
    assert!(custom_pressable_retained);
}

#[test]
fn button_glass_defaults_off_and_rejects_non_boolean_values() {
    let module = compile(
        r#"
        app DefaultButtonGlass {
            body { Button("Continue") { } }
        }
        "#,
    )
    .expect("Button glass should default to false");

    assert!(matches!(
        module.body.first(),
        Some(Node::Button { glass: false, .. })
    ));

    let invalid = compile(
        r#"
        app InvalidButtonGlass {
            body { Button("Continue", glass: "sometimes") { } }
        }
        "#,
    )
    .expect_err("Button glass must reject non-boolean values");
    assert!(invalid.to_string().contains("Button glass"));
}

#[test]
fn icon_component_supports_shared_and_platform_specific_symbols() {
    let module = compile(
        r##"
        app IconExample {
            body {
                Icon(system: "favorite_filled", description: "Like", size: 24, tint: "#FFFFFF")
                Icon(sfsymbol: "person.crop.circle.fill", description: "Profile", size: 24, tint: "#FFFFFF")
                Icon(materialsymbol: "outlined:account_circle", description: "Profile", size: 24, tint: "#FFFFFF")
            }
        }
        "##,
    )
    .expect("shared and platform-specific system icon selectors should type-check");

    assert!(matches!(
        module.body.as_slice(),
        [
            Node::SystemIcon { icon: nexa_ir::SystemIcon::Shared(name), .. },
            Node::SystemIcon { icon: nexa_ir::SystemIcon::SfSymbol(sf), .. },
            Node::SystemIcon { icon: nexa_ir::SystemIcon::MaterialSymbol(material), .. },
        ] if name == "favorite_filled"
            && sf == "person.crop.circle.fill"
            && material == "outlined:account_circle"
    ));
}

#[test]
fn font_icon_accepts_typed_runtime_size_on_both_native_backends() {
    let module = compile(
        r##"
        app MaziconGlyph {
            state iconSize: Float64 = 20.0
            body {
                Icon(
                    fontFamily: "Mazicon",
                    glyph: 61696,
                    description: "Activity",
                    size: iconSize,
                    tint: "#FFFFFF"
                )
            }
        }
        "##,
    )
    .expect("font glyph size should accept a typed runtime expression");

    assert!(matches!(
        module.body.first(),
        Some(Node::FontIcon {
            size: Expr::State(name, Type::Numeric(NumericType::Float64)),
            ..
        }) if name == "iconSize"
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(
        swift.contains("CGFloat(max(1.0, min(512.0, nexa_iconSize)))"),
        "{swift}"
    );

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(
        kotlin.contains("(nexa_iconSize).toFloat().coerceIn(1f, 512f).dp"),
        "{kotlin}"
    );
}

#[test]
fn content_unavailable_lowers_localized_text_and_a_shared_icon() {
    let module = compile(
        r##"
        app EmptyState {
            state descriptionColor: String = "#666666"
            body {
                ContentUnavailable(
                    title: "Inbox is empty",
                    icon: "inbox",
                    description: "Tasks you add will appear here.",
                    titleColor: "#222222",
                    descriptionColor: descriptionColor,
                    iconTint: "#0088FF",
                    background: "#F4F4F8"
                )
            }
        }
        "##,
    )
    .expect("ContentUnavailable should accept simple localized text and shared icons");

    let Some(Node::ContentUnavailable {
        title,
        icon: nexa_ir::SystemIcon::Shared(icon),
        description: Some(description),
        title_color:
            Some(nexa_ir::ColorExpression::Static(nexa_ir::ColorValue::Static(title_color))),
        description_color: Some(nexa_ir::ColorExpression::Dynamic(dynamic_description_color)),
        icon_tint: Some(nexa_ir::ColorExpression::Static(nexa_ir::ColorValue::Static(icon_tint))),
        background: Some(nexa_ir::ColorExpression::Static(nexa_ir::ColorValue::Static(background))),
    }) = module.body.first()
    else {
        panic!("expected a typed ContentUnavailable node");
    };
    assert_eq!(source_text(title), Some("Inbox is empty"));
    assert_eq!(icon, "inbox");
    assert_eq!(
        source_text(description),
        Some("Tasks you add will appear here.")
    );
    assert_eq!(
        (
            title_color.red,
            title_color.green,
            title_color.blue,
            title_color.alpha
        ),
        (34, 34, 34, 255)
    );
    assert!(matches!(
        dynamic_description_color.as_ref(),
        Expr::State(name, _) if name == "descriptionColor"
    ));
    assert_eq!(
        (
            icon_tint.red,
            icon_tint.green,
            icon_tint.blue,
            icon_tint.alpha
        ),
        (0, 136, 255, 255)
    );
    assert_eq!(
        (
            background.red,
            background.green,
            background.blue,
            background.alpha
        ),
        (244, 244, 248, 255)
    );

    let swift = nexa_backend_swift::SwiftBackend
        .generate_units(&module)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(swift.contains("titleColor: Color(red: 0.133333"));
    assert!(swift.contains("descriptionColor: nexaColor(hex:"));
    assert!(swift.contains("iconTint: Color(red: 0.000000, green: 0.533333, blue: 1.000000"));
    assert!(swift.contains("background: Color(red: 0.956863, green: 0.956863, blue: 0.972549"));
    assert!(kotlin.contains("titleColor = Color(0xFF222222)"));
    assert!(kotlin.contains("descriptionColor = nexaColorFromHex("));
    assert!(kotlin.contains("iconTint = Color(0xFF0088FF)"));
    assert!(kotlin.contains("background = Color(0xFFF4F4F8)"));

    let without_description = compile(
        r#"
        app EmptyStateWithoutDescription {
            body {
                ContentUnavailable(title: "No reminders", icon: "calendar")
            }
        }
        "#,
    )
    .expect("ContentUnavailable description should be optional");
    assert!(matches!(
        without_description.body.first(),
        Some(Node::ContentUnavailable {
            description: None,
            title_color: None,
            description_color: None,
            icon_tint: None,
            background: None,
            ..
        })
    ));

    let invalid_icon = compile(
        r#"
        app InvalidEmptyState {
            body {
                ContentUnavailable(title: "Empty", icon: "not_a_shared_icon", description: "Try again")
            }
        }
        "#,
    )
    .expect_err("ContentUnavailable icons must resolve through the shared icon catalog");
    assert!(
        invalid_icon
            .to_string()
            .contains("unknown icon `not_a_shared_icon`")
    );
}

#[test]
fn appearance_is_a_generic_typed_wrapper_and_rejects_invalid_static_modes() {
    let module = compile(
        r#"
        app AppearanceExample {
            state mode: String = "system"
            body {
                Appearance(mode: mode) {
                    Text("Hello")
                }
            }
        }
        "#,
    )
    .expect("appearance should accept a runtime String mode");

    assert!(matches!(
        module.body.as_slice(),
        [Node::Appearance {
            mode: Expr::State(name, Type::String),
            children,
        }] if name == "mode" && matches!(children.as_slice(), [Node::Text { .. }])
    ));

    let error = compile(
        r#"app InvalidAppearance { body { Appearance(mode: "sepia") { Text("Hello") } } }"#,
    )
    .expect_err("unknown static appearance modes should be rejected");
    assert!(error.message.contains("system`, `light`, or `dark"));
}

#[test]
fn toolbar_lowers_placement_and_native_action_content() {
    let module = compile(
        r#"
        app ToolbarExample {
            body {
                Toolbar(placement: Trailing) {
                    Button("Add", icon: "add") { }
                }
            }
        }
        "#,
    )
    .expect("toolbar actions lower to a typed native toolbar node");

    assert!(matches!(
        module.body.first(),
        Some(Node::Toolbar {
            placement: nexa_ir::ToolbarPlacement::Trailing,
            children,
        }) if matches!(children.first(), Some(Node::Button { .. }))
    ));
}

#[test]
fn button_accessibility_label_lowers_as_an_optional_localized_string() {
    let module = compile(
        r#"
        app AccessibleIconAction {
            body {
                Button("", icon: "add", shape: Circle, accessibilityLabel: "Add") { }
            }
        }
        "#,
    )
    .expect("Button accessibilityLabel should lower through the typed IR");

    let Some(Node::Button {
        accessibility_label: Some(accessibility_label),
        ..
    }) = module.body.first()
    else {
        panic!(
            "expected typed Button accessibility label, got: {:#?}",
            module.body
        );
    };
    assert!(
        matches!(
            accessibility_label.as_ref(),
            Expr::String(value) if value == "Add"
        ) || matches!(
            accessibility_label.as_ref(),
            Expr::LocalizedText { value, .. } if matches!(value.as_ref(), Expr::String(value) if value == "Add")
        )
    );
    assert!(nexa_ir::localization::extract(&module).contains_key("Add"));

    let default_module = compile(r#"app DefaultButton { body { Button("Save") { } } }"#)
        .expect("Button accessibilityLabel should remain optional");
    assert!(matches!(
        default_module.body.as_slice(),
        [Node::Button {
            accessibility_label: None,
            ..
        }]
    ));
}

#[test]
fn bottom_sheet_lowers_shared_native_detents_and_optional_surface_configuration() {
    let default_sheet = compile(
        r#"
        app DefaultSheetHandle {
            let sheetRef = ComponentRef<BottomSheet>()
            body { BottomSheet(ref: sheetRef) { Text("Default handle") } }
        }
        "#,
    )
    .expect("BottomSheet should default to its native drag handle");
    assert!(matches!(
        default_sheet.body.as_slice(),
        [Node::BottomSheet {
            detents,
            content_padding,
            initial_detent: None,
            show_drag_indicator: true,
            allows_dismissal: true,
            allows_background_interaction: false,
            should_dismiss_on_back_press: true,
            ..
        }]
            if *content_padding == 0.0
                && detents == &[0.5, 1.0]
    ));
    let default_swift = nexa_backend_swift::SwiftBackend.generate(&default_sheet);
    assert!(default_swift.contains("contentPadding: 0"));
    let default_kotlin = nexa_backend_kotlin::KotlinBackend.generate(&default_sheet);
    assert!(default_kotlin.contains("contentPadding = 0f.dp"));

    let configured_sheet = compile(
        r##"
        app ConfiguredSheet {
            let sheetRef = ComponentRef<BottomSheet>()
            body {
                BottomSheet(
                    ref: sheetRef,
                    cornerRadius: 29.25,
                    contentPadding: 20,
                    detents: [1.0],
                    initialDetent: 1.0,
                    showDragIndicator: false,
                    allowsDismissal: false,
                    allowsBackgroundInteraction: true,
                    shouldDismissOnBackPress: false,
                    backgroundColor: "#F2F4F8"
                ) { Text("Details") }
            }
        }
        "##,
    )
    .expect("BottomSheet should lower all shared presentation parameters");
    assert!(matches!(
        configured_sheet.body.as_slice(),
        [Node::BottomSheet {
            detents,
            content_padding,
            corner_radius: Some(29.25),
            initial_detent: Some(1.0),
            show_drag_indicator: false,
            allows_dismissal: false,
            allows_background_interaction: true,
            should_dismiss_on_back_press: false,
            background_color: Some(_),
            ..
        }] if *content_padding == 20.0
            && detents == &[1.0]
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&configured_sheet);
    assert!(swift.contains("detents: [.fraction(1)]"));
    assert!(swift.contains("contentPadding: 20"));
    assert!(swift.contains("cornerRadius: 29.25"));
    assert!(swift.contains("initialDetent: .fraction(1)"));
    assert!(swift.contains("showDragIndicator: false"));
    assert!(swift.contains("allowsDismissal: false"));
    assert!(swift.contains("allowsBackgroundInteraction: true"));
    assert!(swift.contains("shouldDismissOnBackPress: false"));
    assert!(swift.contains("backgroundColor: Color(red:"));
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&configured_sheet);
    assert!(kotlin.contains("detents = listOf(1f)"));
    assert!(kotlin.contains("contentPadding = 20f.dp"));
    assert!(kotlin.contains("cornerRadius = 29.25f.dp"));
    assert!(kotlin.contains("showDragIndicator = false"));
    assert!(kotlin.contains("allowsDismissal = false"));
    assert!(kotlin.contains("allowsBackgroundInteraction = true"));
    assert!(kotlin.contains("shouldDismissOnBackPress = false"));
    assert!(kotlin.contains("backgroundColor = Color(0xFFF2F4F8)"));
}

#[test]
fn bottom_sheet_refs_can_be_passed_to_reusable_components() {
    let module = compile(
        r#"
        component PlayerSheetControls(sheetRef: ComponentRef<BottomSheet>) {
            body {
                Button("Close player") { sheetRef.dismiss() }
            }
        }
        app PlayerSheetReference {
            let sheetRef = ComponentRef<BottomSheet>()
            body {
                BottomSheet(ref: sheetRef, detents: [1.0]) { Text("Now playing") }
                PlayerSheetControls(sheetRef: sheetRef)
            }
        }
        "#,
    )
    .expect("a typed BottomSheet reference should be reusable as a component parameter");

    let [
        Node::BottomSheet { .. },
        Node::ComponentCall {
            name, arguments, ..
        },
    ] = module.body.as_slice()
    else {
        panic!(
            "expected the sheet and controls component, got: {:#?}",
            module.body
        );
    };
    assert_eq!(name, "PlayerSheetControls");
    assert!(matches!(
        arguments.as_slice(),
        [(argument, Expr::State(state_name, Type::ComponentRef { name, .. }))]
            if argument == "sheetRef" && state_name == "sheetRef" && name == "BottomSheet"
    ));
}

#[test]
fn bottom_sheet_can_host_a_nested_navigation_stack() {
    let module = compile(
        r#"
        screen Player { Text("Now playing") }
        screen Home {
            let sheetRef = ComponentRef<BottomSheet>()
            Button("Open player") { sheetRef.present() }
            BottomSheet(ref: sheetRef, detents: [1.0]) {
                NavigationStack(root: Player, showNavigationBar: false)
            }
        }
        app PlayerSheetNavigation {
            body { NavigationStack(root: Home, showNavigationBar: false) }
        }
        "#,
    )
    .expect("a modal sheet may own a nested navigation stack");

    assert!(matches!(
        module.body.as_slice(),
        [Node::NavigationStack { .. }]
    ));
}

#[test]
fn bottom_sheet_rejects_empty_duplicate_and_unconfigured_initial_detents() {
    for source in [
        r#"app Empty { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: []) { Text("x") } } }"#,
        r#"app NegativePadding { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, contentPadding: -1) { Text("x") } } }"#,
        r#"app NegativeCorner { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, cornerRadius: -1) { Text("x") } } }"#,
        r#"app Duplicate { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: [0.5, 0.5]) { Text("x") } } }"#,
        r#"app Missing { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: [0.5], initialDetent: 1.0) { Text("x") } } }"#,
        r#"app MixedSizing { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, contentSized: true, detents: [0.5]) { Text("x") } } }"#,
        r#"app MixedInitial { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, contentSized: true, initialDetent: 0.5) { Text("x") } } }"#,
    ] {
        assert!(
            compile(source).is_err(),
            "invalid detent configuration must fail"
        );
    }
}

#[test]
fn bottom_sheet_content_sizing_lowers_to_intrinsic_native_sheets() {
    let module = compile(
        r#"
        app ContentSizedSheet {
            let sheetRef = ComponentRef<BottomSheet>()
            body {
                BottomSheet(ref: sheetRef, contentSized: true) {
                    Text("Actions")
                }
            }
        }
        "#,
    )
    .expect("content-sized BottomSheet should lower without fractional detents");

    assert!(matches!(
        module.body.as_slice(),
        [Node::BottomSheet {
            content_sized: true,
            detents,
            initial_detent: None,
            ..
        }] if detents.is_empty()
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("contentSized: true"));
    assert!(swift.contains(".presentationDetents(\n                        contentSized ? [.height(measuredContentHeight)] : detents"));
    assert!(swift.contains("fixedSize(horizontal: false, vertical: true)"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("contentSized = true"));
    assert!(kotlin.contains("detents = listOf()"));
    assert!(kotlin.contains("if (contentSized || nativeValues.size == detents.size)"));
    assert!(kotlin.contains("Modifier.fillMaxWidth().weight(1f)"));
}

#[test]
fn bottom_sheet_lowers_fractional_detents_and_dismiss_actions_for_both_backends() {
    let module = compile(
        r##"
        app CustomSnapSheet {
            let sheetRef = ComponentRef<BottomSheet>()
            state dismissed: Bool = false
            body {
                BottomSheet(
                    ref: sheetRef,
                    detents: [0.25, 0.5, 0.85],
                    initialDetent: 0.5,
                    showDragIndicator: false,
                    backgroundColor: "#00000000",
                    scrimColor: "#00000000"
                ) { Text("Map details") }.onDismiss {
                    dismissed = true
                }
            }
        }
        "##,
    )
    .expect("BottomSheet should lower fractional snaps and dismissal actions");
    assert!(matches!(
        module.body.as_slice(),
        [Node::BottomSheet {
            detents,
            initial_detent: Some(0.5),
            show_drag_indicator: false,
            background_color: Some(_),
            scrim_color: Some(_),
            on_dismiss,
            ..
        }] if detents == &[0.25, 0.5, 0.85]
            && matches!(on_dismiss.as_slice(), [nexa_ir::Action::Assign { name, value: nexa_ir::Expr::Bool(true) }] if name == "dismissed")
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("detents: [.fraction(0.25), .fraction(0.5), .fraction(0.85)]"));
    assert!(swift.contains("initialDetent: .fraction(0.5)"));
    assert!(swift.contains("showDragIndicator: false"));
    assert!(swift.contains("onDismiss: {"));
    assert!(swift.contains(
        "scrimColor: Color(red: 0.000000, green: 0.000000, blue: 0.000000, opacity: 0.000000)"
    ));
    assert!(swift.contains("dismissed = true"));
    assert!(
        swift.contains("Color(red: 0.000000, green: 0.000000, blue: 0.000000, opacity: 0.000000)")
    );

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("detents = listOf(0.25f, 0.5f, 0.85f)"));
    assert!(kotlin.contains("initialDetent = 0.5f"));
    assert!(kotlin.contains("showDragIndicator = false"));
    assert!(kotlin.contains("onDismiss = {"));
    assert!(kotlin.contains("dismissed = true"));
    assert!(kotlin.contains("backgroundColor = Color(0x00000000)"));
    assert!(kotlin.contains("scrimColor = Color(0x00000000)"));
    assert!(kotlin.contains("import kotlinx.coroutines.launch"));
}

#[test]
fn bottom_sheet_uses_one_detent_list_and_native_drag_selection() {
    let module = compile(
        r#"
        app FractionalSheet {
            let sheetRef = ComponentRef<BottomSheet>()
            body {
                Button("Open") { sheetRef.present() }
                Button("Close") { sheetRef.dismiss() }
                BottomSheet(
                    ref: sheetRef,
                    detents: [0.25, 0.5, 0.85],
                    initialDetent: 0.5
                ) { Text("Drag to resize") }
            }
        }
        "#,
    )
    .expect("fractional detents should be selected by dragging the sheet");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("detents: [.fraction(0.25), .fraction(0.5), .fraction(0.85)]"));
    assert!(swift.contains("initialDetent: .fraction(0.5)"));
    assert!(!swift.contains("selectedSnapPoint"));
    assert!(swift.contains("componentRef: nexa_sheetRef"));
    assert!(swift.contains("nexa_sheetRef.present()"));
    assert!(swift.contains("nexa_sheetRef.dismiss()"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("detents = listOf(0.25f, 0.5f, 0.85f)"));
    assert!(kotlin.contains("initialDetent = 0.5f"));
    assert!(kotlin.contains("NexaCustomBottomSheetPrimitive("));
    assert!(!kotlin.contains("selectedSnapPoint"));
}

#[test]
fn bottom_sheet_header_save_and_close_actions_compile_for_both_platforms() {
    let module = compile(
        r#"
        app BottomSheetHeaderActions {
            let sheetRef = ComponentRef<BottomSheet>()
            state saved: Bool = false
            body {
                BottomSheet(ref: sheetRef) {
                    Column {
                        Row {
                            Text("Edit task")
                            Spacer()
                            Button("Save", icon: "save", style: Borderless) { saved = true }
                            Button("Close", icon: "close", style: Plain) {
                                sheetRef.dismiss()
                            }
                        }
                        Text(saved ? "Changes saved" : "Unsaved changes")
                    }
                }
            }
        }
        "#,
    )
    .expect("buttons in a custom BottomSheet header should compile");
    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("Label(\"Save\", systemImage: \"square.and.arrow.down\")"));
    assert!(swift.contains("Spacer(minLength: 0)"));
    assert!(swift.contains("Label(\"Close\", systemImage: \"xmark\")"));
    assert!(swift.contains("nexa_saved = true"));
    assert!(swift.contains("nexa_sheetRef.dismiss()"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("icon = NexaIconSpec(vector = Icons.Filled.Save),"));
    assert!(kotlin.contains("icon = NexaIconSpec(vector = Icons.Filled.Close),"));
    assert!(kotlin.contains("nexa_saved = true"));
    assert!(kotlin.contains("nexa_sheetRef.dismiss()"));
}

#[test]
fn nested_bottom_sheets_keep_the_parent_presented_on_both_platforms() {
    let module = compile(
        r#"
        app NestedBottomSheets {
            let parentSheetRef = ComponentRef<BottomSheet>()
            let childSheetRef = ComponentRef<BottomSheet>()
            body {
                BottomSheet(ref: parentSheetRef) {
                    Column {
                        Button("Open child") { childSheetRef.present() }
                        BottomSheet(ref: childSheetRef) {
                            Button("Close child") { childSheetRef.dismiss() }
                        }
                    }
                }
            }
        }
        "#,
    )
    .expect("a BottomSheet can be nested in another sheet's content");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    let swift_parent = swift
        .find("componentRef: nexa_parentSheetRef")
        .expect("emit the parent native SwiftUI sheet");
    let swift_child = swift
        .find("componentRef: nexa_childSheetRef")
        .expect("emit the nested native SwiftUI sheet");
    assert!(swift_parent < swift_child);
    assert!(swift.contains(".sheet(isPresented: $isPresented, onDismiss: {"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    let kotlin_parent = kotlin
        .find("componentRef = nexa_parentSheetRef,")
        .expect("emit the parent Compose sheet");
    let kotlin_child = kotlin
        .find("componentRef = nexa_childSheetRef,")
        .expect("emit the nested Material 3 sheet");
    assert!(kotlin_parent < kotlin_child);
    assert!(kotlin.contains("NexaCustomBottomSheetPrimitive("));
}

#[test]
fn bottom_sheet_dynamic_surface_and_dismiss_callback_count_as_state_uses() {
    let compilation = compile_with_warnings_for_target(
        r##"
        app BottomSheetStateUses {
            let sheetRef = ComponentRef<BottomSheet>()
            state surface: String = "#00000000"
            state scrim: String = "#00000000"
            state dismissed: Bool = false
            body {
                BottomSheet(ref: sheetRef, backgroundColor: surface, scrimColor: scrim) {
                    Text("Map details")
                }.onDismiss {
                    dismissed = true
                }
            }
        }
        "##,
        Target::Kotlin,
    )
    .expect("dynamic sheet surface and dismissal callback should compile");

    for state in ["surface", "scrim", "dismissed"] {
        assert!(
            compilation
                .warnings
                .iter()
                .all(|warning| !warning.message.contains(&format!("unused state `{state}`"))),
            "BottomSheet must count `{state}` as used"
        );
    }
}

#[test]
fn compiler_warnings_have_stable_codes_by_warning_kind() {
    let compilation = compile_with_warnings_for_target(
        r#"
        fn unusedReport() -> Int32 {
            let scratch = 3
            return 1
        }

        app WarningCodes {
            state unusedState: Int32 = 0
            body { Text("Ready") }
        }
        "#,
        Target::Swift,
    )
    .expect("warning examples should compile");

    assert!(compilation.warnings.iter().any(|warning| {
        warning.code == WarningCode::UNUSED_FUNCTION
            && warning.message.contains("unused function `unusedReport`")
    }));
    assert!(compilation.warnings.iter().any(|warning| {
        warning.code == WarningCode::UNUSED_LOCAL
            && warning.message.contains("unused local constant `scratch`")
    }));
    assert!(compilation.warnings.iter().any(|warning| {
        warning.code == WarningCode::UNUSED_DECLARATION
            && warning.message.contains("unused state `unusedState`")
    }));
}

#[test]
fn top_level_helpers_called_from_reachable_class_static_functions_are_reachable() {
    let compilation = compile_with_warnings_for_target(
        r#"
        fn formatSearchTitle(value: String) -> String {
            return "Search: " + value
        }

        class SearchText {
            static fn title(value: String) -> String {
                return formatSearchTitle(value)
            }
        }

        app StaticFunctionReachability {
            body { Text(SearchText.title("Songs")) }
        }
        "#,
        Target::Swift,
    )
    .expect("reachable static functions should keep their top-level helpers");

    assert!(
        compilation.warnings.iter().all(|warning| !warning
            .message
            .contains("unused function `formatSearchTitle`")),
        "a top-level helper called by a reachable class static function must not be reported unused"
    );
}

#[test]
fn fast_lists_warn_when_collection_identity_is_only_positional() {
    let unkeyed = compile_with_warnings_for_target(
        r#"
        app UnkeyedRows {
            state messages: Array<String> = ["First", "Second"]
            body {
                FastList(messages) { message, index in
                    Text("\(index): \(message)")
                }
            }
        }
        "#,
        Target::Swift,
    )
    .expect("an unkeyed list remains valid but should warn");
    let warning = unkeyed
        .warnings
        .iter()
        .find(|warning| warning.code == WarningCode::FAST_LIST_MISSING_KEY)
        .expect("collection lists without a key should warn");
    assert!(warning.message.contains("use positional identity"));
    assert!(warning.message.contains("key: .id"));
    assert!(warning.message.contains("key: .self"));

    let keyed = compile_with_warnings_for_target(
        r#"
        app KeyedRows {
            state messages: Array<String> = ["First", "Second"]
            body {
                FastList(messages, key: .self) { message, index in
                    Text("\(index): \(message)")
                }
            }
        }
        "#,
        Target::Swift,
    )
    .expect("a scalar list with a key should compile");
    assert!(
        keyed
            .warnings
            .iter()
            .all(|warning| warning.code != WarningCode::FAST_LIST_MISSING_KEY)
    );
}

#[test]
fn bottom_sheet_rejects_invalid_detents_and_removed_legacy_parameters() {
    for source in [
        r#"app Empty { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: []) { Text("x") } } }"#,
        r#"app Duplicate { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: [0.5, 0.5]) { Text("x") } } }"#,
        r#"app Unsorted { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: [0.75, 0.5]) { Text("x") } } }"#,
        r#"app Zero { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: [0.0]) { Text("x") } } }"#,
        r#"app OutOfRange { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: [1.2]) { Text("x") } } }"#,
        r#"app MissingInitial { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, detents: [0.25, 0.75], initialDetent: 0.5) { Text("x") } } }"#,
        r#"app InitialOutOfRange { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, initialDetent: 1.2) { Text("x") } } }"#,
        r#"app OldSnapPoints { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, snapPoints: [0.5]) { Text("x") } } }"#,
        r#"app OldInitialSnapPoint { let sheetRef = ComponentRef<BottomSheet>() body { BottomSheet(ref: sheetRef, initialSnapPoint: 0.5) { Text("x") } } }"#,
        r#"app OldSelectedSnapPoint { let sheetRef = ComponentRef<BottomSheet>() state point: Float64 = 0.5 body { BottomSheet(ref: sheetRef, selectedSnapPoint: point) { Text("x") } } }"#,
        r#"app RemovedStateBinding { state shown: Bool = false body { BottomSheet(isPresented: shown) { Text("x") } } }"#,
        r#"app RemovedPresentHelper { state shown: Bool = false body { Button("Open") { BottomSheet.present(shown) } } }"#,
        r#"app RemovedDismissHelper { state shown: Bool = true body { Button("Close") { BottomSheet.dismiss(shown) } } }"#,
        r#"app RemovedSnapHelper { state point: Float64 = 0.5 body { Button("Expand") { BottomSheet.snapTo(point, 0.85) } } }"#,
        r#"app SnapToUnknownState { body { Button("Snap") { BottomSheet.snapTo(missing, 0.5) } } }"#,
        r#"app PresentWrongType { state point: Float64 = 0.5 body { Button("Open") { BottomSheet.present(point) } } }"#,
    ] {
        assert!(
            compile(source).is_err(),
            "invalid or removed API must fail: {source}"
        );
    }
}

#[test]
fn network_online_property_lowers_to_a_typed_synchronous_core_call() {
    let module = compile(
        r#"
        app Connectivity {
            state connected: Bool = Network.isOnline
            body { Text(connected ? "Online" : "Offline") }
        }
        "#,
    )
    .expect("network connectivity property should compile");

    assert!(matches!(
        &module.states[0].initial,
        Expr::NativeCall {
            receiver: None,
            namespace,
            name,
            arguments,
            return_type: Type::Bool,
            is_async: false,
            is_throwing: false,
            ..
        } if namespace == "Network" && name == "isOnline" && arguments.is_empty()
    ));
    let capabilities = nexa_ir::capabilities::analyze(&module);
    assert!(capabilities.uses_network_connectivity);
    assert!(!capabilities.uses_network_api);
    assert!(!capabilities.uses_network_transport());
}

#[test]
fn network_status_subscription_lowers_a_typed_boolean_callback() {
    let module = compile(
        r#"
        app Connectivity {
            state connected: Bool = false
            body {
                Button("Watch") {
                    Network.onStatusChange { online -> connected = online }
                }
            }
        }
        "#,
    )
    .expect("network status subscription should compile");

    let Node::Button { actions, .. } = &module.body[0] else {
        panic!("expected a button action");
    };
    let [Action::NetworkStatusSubscribe { parameter, actions }] = actions.as_slice() else {
        panic!("expected a lowered network status subscription");
    };
    assert_eq!(parameter, "online");
    assert!(matches!(
        actions.as_slice(),
        [Action::Assign { name, value: Expr::State(binding, _) }]
            if name == "connected" && binding == "online"
    ));
    let capabilities = nexa_ir::capabilities::analyze(&module);
    assert!(capabilities.uses_network_connectivity);
    assert!(!capabilities.uses_network_transport());
}

#[test]
fn multipart_upload_lowers_to_an_async_typed_network_call() {
    let module = compile(
        r#"
        app MultipartUpload {
            state statusCode: Int32 = 0
            body {
                Text("Ready")
                OnAppear async {
                    try {
                        statusCode = (await Network.upload(
                            url: "https://api.example.com/receipts",
                            file: "receipt.bin",
                            fields: ["kind": "receipt"],
                            headers: ["Authorization": "Bearer test-token"],
                            timeoutSeconds: 45,
                            followRedirects: false,
                            maxResponseBytes: 1048576,
                            certificatePins: ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"]
                        )).statusCode
                    } catch { }
                }
            }
        }
        "#,
    )
    .expect("multipart upload should compile with a typed response");

    let Some([Action::TryCatch { body, .. }]) = module.on_appear.as_deref() else {
        panic!("expected the async upload's error handling action");
    };
    let [Action::Assign { value, .. }] = body.as_slice() else {
        panic!("expected to assign the upload response status");
    };
    let Expr::Member {
        base,
        name,
        field_type: Type::Numeric(NumericType::Int32),
        ..
    } = value
    else {
        panic!("expected typed statusCode access on the upload response");
    };
    assert_eq!(name, "statusCode");
    let Expr::TryAwait(call) = base.as_ref() else {
        panic!("expected the async upload call to propagate errors");
    };
    assert!(matches!(
        call.as_ref(),
        Expr::NativeCall {
            receiver: None,
            namespace,
            name,
            arguments,
            return_type: Type::NetworkResponse,
            is_async: true,
            is_throwing: true,
            ..
        } if namespace == "Network"
            && name == "upload"
            && arguments.iter().map(|(argument, _)| argument.as_str()).collect::<Vec<_>>() == ["url", "file", "fields", "headers", "timeout", "followRedirects", "maxResponseBytes", "certificatePins"]
    ));
    let capabilities = nexa_ir::capabilities::analyze(&module);
    assert!(capabilities.uses_network_api);
    assert!(capabilities.uses_network_transport());
}

#[test]
fn haptics_calls_lower_to_validated_typed_core_calls() {
    let module = compile(
        r#"
        app HapticsExample {
            body {
                Button("Tap") { Haptics.impact(style: Heavy) }
                Button("Done") { Haptics.notification(kind: Success) }
                Button("Select") { Haptics.selection() }
            }
        }
        "#,
    )
    .expect("haptics calls should compile");

    for (node, expected_name, expected_argument, expected_value) in [
        (&module.body[0], "impact", "style", "Heavy"),
        (&module.body[1], "notification", "kind", "Success"),
        (&module.body[2], "selection", "", ""),
    ] {
        let Node::Button { actions, .. } = node else {
            panic!("expected a button");
        };
        assert!(matches!(
            actions.as_slice(),
            [Action::Expression(Expr::NativeCall {
                namespace,
                name,
                arguments,
                return_type: Type::Void,
                is_async: false,
                is_throwing: false,
                ..
            })] if namespace == "Haptics"
                && name == expected_name
                && arguments.is_empty() == expected_argument.is_empty()
                && (expected_argument.is_empty()
                    || matches!(arguments.as_slice(), [(argument, Expr::String(value))]
                        if argument == expected_argument && value == expected_value))
        ));
    }
    assert!(nexa_ir::capabilities::analyze(&module).uses_haptics_api);
}

#[test]
fn haptics_reject_unknown_feedback_style() {
    let error = compile(
        r#"
        app InvalidHaptics {
            body { Button("Tap") { Haptics.impact(style: Soft) } }
        }
        "#,
    )
    .expect_err("unknown haptics feedback styles should fail type checking");

    assert!(error.to_string().contains("Haptics.impact style must be"));
}

#[test]
fn text_input_rejects_unsupported_option_names() {
    let error = compile(
        r#"
        app InvalidKeyboardOptions {
            state text: String = ""
            body {
                TextInput(value: text, placeholder: "Text", keyboard: Email) { }
            }
        }
        "#,
    )
    .expect_err("TextInput options must use canonical names");
    assert!(error.to_string().contains("unknown option `keyboard`"));
}

#[test]
fn string_add_concatenates_and_folds_literals() {
    let module = compile(
        r#"
        app Greeting {
            state title: String = "Nexa" + " " + "1.0"
            state greeting: String = "Nexa"
            state entries: Array<Int32> = [1, 2]

            body {
                Text(title)
                Text(entries.count)
                Text(entries.isEmpty)
                Button("Append") {
                    greeting += " 1.0"
                }
            }
        }
        "#,
    )
    .expect("string addition should compile as concatenation");

    let title = module
        .states
        .iter()
        .find(|state| state.name == "title")
        .expect("the title state should be present");
    assert!(matches!(&title.initial, Expr::String(value) if value == "Nexa 1.0"));
    assert!(module.body.iter().any(|node| matches!(
        node,
        Node::Text {
            value: Expr::Member {
                kind: MemberKind::CollectionCount,
                field_type: Type::Numeric(NumericType::Int32),
                ..
            },
            ..
        }
    )));
    assert!(module.body.iter().any(|node| matches!(
        node,
        Node::Text {
            value: Expr::Member {
                kind: MemberKind::CollectionIsEmpty,
                field_type: Type::Bool,
                ..
            },
            ..
        }
    )));

    let Some(Node::Button { actions, .. }) = module
        .body
        .iter()
        .find(|node| matches!(node, Node::Button { .. }))
    else {
        panic!("expected the append button");
    };
    assert!(matches!(
        actions.first(),
        Some(Action::Assign {
            value: Expr::Concat(left, right),
            ..
        }) if matches!(left.as_ref(), Expr::State(name, Type::String) if name == "greeting")
            && matches!(right.as_ref(), Expr::String(value) if value == " 1.0")
    ));
}

#[test]
fn array_utilities_lower_to_typed_collection_ir() {
    let module = compile(
        r#"
        app ArrayUtilities {
            state values: Array<Int32> = [10, 20, 30, 40]

            body {
                Text(values.random() ?? 0)
                Text(values.first() ?? 0)
                Text(values.last() ?? 0)
                Text(values.shuffled().count)
                Text(values.reverse().count)
                Text(values.slice(1..<3).count)
            }
        }
        "#,
    )
    .expect("array utilities should compile for typed arrays");

    fn find_utility(expression: &Expr) -> Option<&Expr> {
        match expression {
            Expr::CollectionUtility { .. } => Some(expression),
            Expr::Coalesce(left, right) => find_utility(left).or_else(|| find_utility(right)),
            Expr::Member { base, .. } => find_utility(base),
            _ => None,
        }
    }
    let utilities = module
        .body
        .iter()
        .filter_map(|node| match node {
            Node::Text { value, .. } => find_utility(value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(utilities.len(), 6);
    assert!(matches!(
        utilities[0],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Random,
            element_type: Type::Numeric(NumericType::Int32),
            ..
        }
    ));
    assert!(matches!(
        utilities[1],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::First,
            element_type: Type::Numeric(NumericType::Int32),
            ..
        }
    ));
    assert!(matches!(
        utilities[2],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Last,
            element_type: Type::Numeric(NumericType::Int32),
            ..
        }
    ));
    assert!(matches!(
        utilities[3],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Shuffled,
            ..
        }
    ));
    assert!(matches!(
        utilities[4],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Reverse,
            ..
        }
    ));
    assert!(matches!(
        utilities[5],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Slice,
            start: Some(start),
            end: Some(end),
            inclusive: false,
            ..
        } if matches!(start.as_ref(), Expr::Number { raw, ty: NumericType::Int32 } if raw == "1")
            && matches!(end.as_ref(), Expr::Number { raw, ty: NumericType::Int32 } if raw == "3")
    ));
}

#[test]
fn conditional_expressions_infer_shared_and_optional_branch_types() {
    let module = compile(
        r#"
        app ConditionalValues {
            state enabled: Bool = true

            fn selected(enabled: Bool) -> String? {
                return if enabled { "ready" } else { null }
            }

            body {
                Text(if enabled { "ready" } else { "waiting" })
                Text(enabled ? "on" : "off")
                Text(selected(enabled) ?? "unavailable")
            }
        }
        "#,
    )
    .expect("value-producing if expressions should infer branch types");

    let function = module
        .functions
        .iter()
        .find(|function| function.name == "selected")
        .expect("the selected helper should be present");
    assert!(matches!(
        &function.body,
        Expr::Conditional {
            value_type: Type::Optional(inner),
            else_value,
            ..
        } if matches!(inner.as_ref(), Type::String)
            && matches!(else_value.as_ref(), Expr::Null(Type::Optional(_)))
    ));

    assert!(matches!(
        &module.body[0],
        Node::Text {
            value: Expr::Conditional {
                value_type: Type::String,
                ..
            },
            ..
        }
    ));
    assert!(
        matches!(
            &module.body[1],
            Node::Text {
                value: Expr::Conditional {
                    value_type: Type::String,
                    ..
                },
                ..
            }
        ),
        "unexpected lowered ring: {:?}",
        module.body[1]
    );
}

#[test]
fn constant_conditional_expressions_fold_to_the_selected_branch() {
    let module = compile(
        r#"
        app FoldConditional {
            state label: String = if true { "ready" } else { "unreachable" }
            body { Text(label) }
        }
        "#,
    )
    .expect("constant conditional branches should compile");

    let label = module
        .states
        .iter()
        .find(|state| state.name == "label")
        .expect("the label state should be present");
    assert!(matches!(&label.initial, Expr::String(value) if value == "ready"));
}

#[test]
fn text_style_arguments_lower_to_native_text_style_fields() {
    let module = compile(
        r#"
        app TextStyles {
            body {
                Text("Hi", fontSize: 18, fontWeight: Bold, padding: 12, italic: true, underline: true, fontDesign: Rounded)
            }
        }
        "#,
    )
    .expect("text style arguments should lower to native style fields");

    assert!(matches!(
        &module.body[0],
        Node::Text { value, style, .. }
            if source_text(value) == Some("Hi")
                && style.font_size == Some(18.0)
                && style.font_weight == Some(nexa_ir::FontWeight::Bold)
                && style.italic
                && style.underline
                && style.font_design == Some(nexa_ir::TextFontDesign::Rounded)
                && style.padding == Some(12.0)
    ));
}

#[test]
fn text_font_family_uses_theme_default_and_static_overrides() {
    let module = compile(
        r#"
        app TextFontFamilies {
            theme {
                fontFamily Default: "Gilroy-Regular"
                fontFamily Mazicon: "Mazicon"
            }
            body {
                Text("Title")
                Text("Icon", fontFamily: Theme.Mazicon)
                Text("Override", fontFamily: "NotoSans")
            }
        }
        "#,
    )
    .expect("font family theme tokens should lower to static font names");

    assert!(matches!(
        &module.body[0],
        Node::Text { style, .. } if style.font_family.as_deref() == Some("Gilroy-Regular")
    ));
    assert!(matches!(
        &module.body[1],
        Node::Text { style, .. } if style.font_family.as_deref() == Some("Mazicon")
    ));
    assert!(matches!(
        &module.body[2],
        Node::Text { style, .. } if style.font_family.as_deref() == Some("NotoSans")
    ));

    let invalid = compile(
        r#"
        app InvalidFontFamily {
            body { Text("Dynamic", fontFamily: userFont) }
        }
        "#,
    )
    .expect_err("font family names must be statically known");
    assert!(
        invalid
            .to_string()
            .contains("fontFamily must be a static string")
    );
}

#[test]
fn semantic_text_font_roles_lower_and_validate() {
    let module = compile(
        r#"
        app SemanticTextRoles {
            body {
                Text("Task description", fontStyle: Subheadline)
                Text("Due tomorrow", fontStyle: Caption)
                Text("Code", fontDesign: Monospaced)
            }
        }
        "#,
    )
    .expect("semantic text font roles should lower into typed IR");

    assert!(matches!(
        &module.body[0],
        Node::Text { style, .. }
            if style.font_style == Some(nexa_ir::TextFontStyle::Subheadline)
    ));
    assert!(matches!(
        &module.body[1],
        Node::Text { style, .. }
            if style.font_style == Some(nexa_ir::TextFontStyle::Caption)
    ));
    assert!(matches!(
        &module.body[2],
        Node::Text { style, .. }
            if style.font_design == Some(nexa_ir::TextFontDesign::Monospaced)
    ));

    let invalid = compile(
        r#"
        app InvalidTextRole {
            body { Text("Task", fontStyle: Display) }
        }
        "#,
    )
    .expect_err("unsupported semantic text roles should be rejected");
    assert!(invalid.to_string().contains("fontStyle must be"));

    let invalid_design = compile(
        r#"
        app InvalidTextDesign {
            body { Text("Task", fontDesign: Display) }
        }
        "#,
    )
    .expect_err("unsupported system font designs should be rejected");
    assert!(invalid_design.to_string().contains("fontDesign must be"));
}

#[test]
fn visual_style_arguments_lower_to_validated_static_effects() {
    let module = compile(
        r##"
        app VisualEffects {
            body {
                Column(
                    opacity: 0.8,
                    scale: 1.1,
                scaleY: 0.7,
                    rotation: -6,
                    shadow: Shadow(4, 1, -2, "#00000080"),
                    blur: 2,
                    clip: Rounded(12),
                    zIndex: -3
                ) {
                    Text("Card", scale: 0.95, clip: Rounded(5))
                }
            }
        }
        "##,
    )
    .expect("valid static visual style arguments should compile");

    let Node::Layout {
        style, children, ..
    } = &module.body[0]
    else {
        panic!("expected a styled layout");
    };
    assert_eq!(style.opacity, Some(0.8));
    assert_eq!(style.effects.scale, Some(1.1));
    assert_eq!(style.effects.scale_y, Some(0.7));
    assert_eq!(style.effects.rotation, Some(-6.0));
    assert_eq!(style.effects.blur, Some(2.0));
    assert_eq!(style.effects.clip_rounded, Some(12.0));
    assert_eq!(style.effects.z_index, Some(-3));
    let shadow = style
        .effects
        .shadow
        .expect("shadow parameters should lower");
    assert_eq!(shadow.radius, 4.0);
    assert_eq!(shadow.x, 1.0);
    assert_eq!(shadow.y, -2.0);
    assert!(matches!(shadow.color, nexa_ir::ColorValue::Static(color) if color.alpha == 128));

    let Node::Text { style, .. } = &children[0] else {
        panic!("expected a styled text node");
    };
    assert_eq!(style.effects.scale, Some(0.95));
    assert_eq!(style.effects.clip_rounded, Some(5.0));
}

#[test]
fn view_scale_accepts_animated_numeric_state() {
    let module = compile(
        r#"
        app AnimatedScale {
            state zoom: Float64 = 1.0

            body {
                Column(scale: zoom) {
                    Text("Bookmark")
                }
                Button("Pulse") {
                    withAnimation(Tween(duration: 0.07, easing: Linear)) {
                        zoom = 0.5
                    }
                }
            }
        }
        "#,
    )
    .expect("a Float64 state should be accepted as a native scale transform");

    let Node::Layout { style, .. } = &module.body[0] else {
        panic!("expected the animated scale layout");
    };
    assert!(matches!(
        style.effects.scale_expression.as_deref(),
        Some(nexa_ir::Expr::AnimatedState(name, nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64)))
            if name == "zoom"
    ));
    assert_eq!(style.effects.scale, None);
}

#[test]
fn vertical_view_scale_accepts_animated_numeric_state() {
    let module = compile(
        r#"
        app AnimatedVerticalScale {
            state squash: Float64 = 1.0

            body {
                Row(scaleY: squash) {
                    Text("Shuffle", scaleY: squash)
                }
                Button("Squash") {
                    withAnimation(Tween(duration: 0.13, easing: Linear)) {
                        squash = 0.0
                    }
                }
            }
        }
        "#,
    )
    .expect("a Float64 state should be accepted as a vertical transform");

    let Node::Layout {
        style, children, ..
    } = &module.body[0]
    else {
        panic!("expected the animated vertical-scale layout");
    };
    assert!(matches!(
        style.effects.scale_y_expression.as_deref(),
        Some(nexa_ir::Expr::AnimatedState(name, nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64)))
            if name == "squash"
    ));
    let Node::Text { style, .. } = &children[0] else {
        panic!("expected animated vertical-scale text");
    };
    assert!(matches!(
        style.effects.scale_y_expression.as_deref(),
        Some(nexa_ir::Expr::AnimatedState(name, nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64)))
            if name == "squash"
    ));
}

#[test]
fn visual_styles_are_rejected_as_dot_modifiers() {
    for (component, modifier) in [
        ("Text(\"Hi\")", ".fontSize(18)"),
        ("Text(\"Hi\")", ".bold()"),
        ("Text(\"Hi\")", ".padding(12)"),
        ("Column { Text(\"Hi\") }", ".opacity(0.8)"),
        ("Column { Text(\"Hi\") }", ".scale(1.1)"),
        ("Column { Text(\"Hi\") }", ".rotation(15)"),
        (
            "Column { Text(\"Hi\") }",
            ".shadow(radius: 4, x: 0, y: 2, color: \"#00000040\")",
        ),
        ("Column { Text(\"Hi\") }", ".blur(2)"),
        ("Column { Text(\"Hi\") }", ".clip(shape: Rounded(12))"),
        ("Column { Text(\"Hi\") }", ".zIndex(1)"),
    ] {
        let source = format!("app Invalid {{ body {{ {component}{modifier} }} }}");
        let error = compile(&source).expect_err("visual styles must use component arguments");
        assert!(error.to_string().contains("unknown"), "{error}");
    }
}

#[test]
fn spring_animation_lowers_custom_physics_and_rejects_nonpositive_values() {
    let module = compile(
        r#"app SpringAnimation {
            body {
                Column(animation: Spring(response: 0.35, damping: 0.8)) {
                    Text("animated")
                }
            }
        }"#,
    )
    .expect("valid spring options should compile");
    let Node::Layout { style, .. } = &module.body[0] else {
        panic!("expected Column layout");
    };
    assert_eq!(
        style.animation,
        Some(nexa_ir::AnimationSpec::Spring {
            response: 0.35,
            damping: 0.8,
        })
    );

    for animation in [
        "Spring(response: 0, damping: 0.8)",
        "Spring(response: 0.35, damping: -0.1)",
    ] {
        let source = format!(
            "app InvalidSpring {{ body {{ Column(animation: {animation}) {{ Text(\"x\") }} }} }}"
        );
        let error = compile(&source).expect_err("nonpositive spring parameters should fail");
        assert!(error.to_string().contains("must be greater than zero"));
    }
}

#[test]
fn tween_animation_keeps_duration_and_curve_in_both_native_backends() {
    let module = compile(
        r#"app TweenAnimation {
            state progress: Float64 = 0.0
            body {
                Column(animation: Tween(duration: 0.42, easing: EaseOut)) {
                    Button("Animate") {
                        withAnimation(Tween(duration: 0.18, easing: EaseInOut)) {
                            progress = 1.0
                        }
                    }
                    ProgressBar(progress: progress)
                }
            }
        }"#,
    )
    .expect("duration and easing options should compile");

    let Node::Layout {
        style, children, ..
    } = &module.body[0]
    else {
        panic!("expected animated Column");
    };
    assert_eq!(
        style.animation,
        Some(nexa_ir::AnimationSpec::Tween {
            duration: 0.42,
            delay: 0.0,
            easing: nexa_ir::AnimationEasing::EaseOut,
        })
    );
    let Node::Button { actions, .. } = &children[0] else {
        panic!("expected animation button");
    };
    assert!(matches!(
        actions.as_slice(),
        [Action::WithAnimation {
            animation: nexa_ir::AnimationSpec::Tween {
                duration,
                easing,
                delay,
            },
            animated_states,
            ..
        }] if (*duration - 0.18).abs() < f64::EPSILON
            && delay.abs() < f64::EPSILON
            && *easing == nexa_ir::AnimationEasing::EaseInOut
            && animated_states == &["progress"]
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains(".timingCurve(0.0, 0.0, 0.2, 1.0, duration: 0.42)"));
    assert!(swift.contains(".timingCurve(0.4, 0.0, 0.2, 1.0, duration: 0.18)"));
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("tween(durationMillis = 420, easing = LinearOutSlowInEasing)"));
    assert!(kotlin.contains(
        "androidx.compose.animation.core.tween(durationMillis = 180, easing = androidx.compose.animation.core.FastOutSlowInEasing)"
    ));

    for (easing, swift_curve, kotlin_curve) in [
        (
            "EaseIn",
            ".timingCurve(0.4, 0.0, 1.0, 1.0, duration: 0.24)",
            "tween(durationMillis = 240, easing = FastOutLinearInEasing)",
        ),
        (
            "EaseOut",
            ".timingCurve(0.0, 0.0, 0.2, 1.0, duration: 0.24)",
            "tween(durationMillis = 240, easing = LinearOutSlowInEasing)",
        ),
        (
            "EaseInOut",
            ".timingCurve(0.4, 0.0, 0.2, 1.0, duration: 0.24)",
            "tween(durationMillis = 240, easing = FastOutSlowInEasing)",
        ),
        (
            "Linear",
            ".linear(duration: 0.24)",
            "tween(durationMillis = 240, easing = LinearEasing)",
        ),
    ] {
        let source = format!(
            "app TimedCurve {{ body {{ Column(animation: Tween(duration: 0.24, easing: {easing})) {{ Text(\"x\") }} }} }}"
        );
        let module = compile(&source).expect("each supported easing curve should compile");
        assert!(
            nexa_backend_swift::SwiftBackend
                .generate(&module)
                .contains(swift_curve),
            "Swift easing mapping drifted for {easing}"
        );
        assert!(
            nexa_backend_kotlin::KotlinBackend
                .generate(&module)
                .contains(kotlin_curve),
            "Kotlin easing mapping drifted for {easing}"
        );
    }

    let default_tween = compile(
        r#"app DefaultTween {
            body { Column(animation: Tween(duration: 0.24)) { Text("x") } }
        }"#,
    )
    .expect("Tween should default to EaseInOut");
    assert!(
        nexa_backend_swift::SwiftBackend
            .generate(&default_tween)
            .contains(".timingCurve(0.4, 0.0, 0.2, 1.0, duration: 0.24)")
    );
    assert!(
        nexa_backend_kotlin::KotlinBackend
            .generate(&default_tween)
            .contains("tween(durationMillis = 240, easing = FastOutSlowInEasing)")
    );

    for animation in [
        "Tween(duration: 0, easing: EaseOut)",
        "Tween(duration: 60.1, easing: EaseOut)",
        "Tween(duration: 0.3, easing: Bounce)",
        "Tween(duration: 0.3, delay: -0.1, easing: EaseOut)",
        "Tween(duration: 0.3, delay: 60.1, easing: EaseOut)",
        "Tween(duration: 0.3, easing: CubicBezier(-0.1, 0, 0.2, 1))",
        "Tween(duration: 0.3, easing: CubicBezier(0.1, 0, 1.1, 1))",
    ] {
        let source = format!(
            "app InvalidTween {{ body {{ Column(animation: {animation}) {{ Text(\"x\") }} }} }}"
        );
        assert!(
            compile(&source).is_err(),
            "invalid animation compiled: {animation}"
        );
    }
}

#[test]
fn custom_cubic_bezier_and_delay_map_to_native_animation_apis() {
    let module = compile(
        r#"app CustomAnimation {
            state progress: Float64 = 0.0
            body {
                Button("Animate") {
                    withAnimation(Tween(
                        duration: 0.24,
                        delay: 0.1,
                        easing: CubicBezier(x1: 0.2, y1: 0.1, x2: 0.7, y2: 1)
                    )) {
                        progress = 1.0
                    }
                }
                ProgressBar(progress: progress)
            }
        }"#,
    )
    .expect("custom native easing and delay should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(
        swift.contains(".timingCurve(0.2, 0.1, 0.7, 1, duration: 0.24).delay(0.1)"),
        "{swift}"
    );
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains(
        "androidx.compose.animation.core.tween(durationMillis = 240, delayMillis = 100, easing = androidx.compose.animation.core.CubicBezierEasing(0.2f, 0.1f, 0.7f, 1f))"
    ));
}

#[test]
fn imperative_animation_is_typed_scoped_and_marks_presentation_reads() {
    let module = compile(
        r#"app ImperativeAnimation {
            state progress: Float64 = 0.0
            body {
                Button("Animate") {
                    withAnimation(Spring(response: 0.35, damping: 0.8)) {
                        progress = 1.0
                    }
                }
                ProgressBar(progress: progress)
            }
        }"#,
    )
    .expect("float state assignments should support imperative animation");

    let Node::Button { actions, .. } = module
        .body
        .iter()
        .find(|node| matches!(node, Node::Button { .. }))
        .expect("animation button should remain in the app body")
    else {
        panic!("expected animation button");
    };
    assert!(matches!(
        actions.as_slice(),
        [Action::WithAnimation {
            animation: nexa_ir::AnimationSpec::Spring { response, damping },
            animated_states,
            actions: inner,
        }] if *response == 0.35
            && *damping == 0.8
            && animated_states == &["progress"]
            && matches!(inner.as_slice(), [Action::Assign { name, .. }] if name == "progress")
    ));
    assert!(matches!(
        module.body.iter().find(|node| matches!(node, Node::ProgressBar { .. })).expect("progress bar should remain in the app body"),
        Node::ProgressBar {
            progress: Some(Expr::AnimatedState(name, Type::Numeric(NumericType::Float64))),
            ..
        } if name == "progress"
    ));

    let integer = compile(
        r#"app InvalidAnimation {
            state count: Int32 = 0
            body {
                Button("Animate") {
                    withAnimation(EaseIn) { count = 1 }
                }
            }
        }"#,
    )
    .expect_err("integer state animation has no equivalent Compose animation mapping");
    assert!(
        integer
            .to_string()
            .contains("mutable Float32 or Float64 state")
    );

    let collection = compile(
        r#"app AnimatedCollection {
            state tasks: Array<Int32> = []
            body {
                Button("Add") {
                    withAnimation(Spring(response: 0.35, damping: 0.8)) { tasks = [1] }
                }
            }
        }"#,
    )
    .expect_err("collection updates must not silently ignore a custom animation spec");
    assert!(
        collection
            .to_string()
            .contains("cannot animate collection mutations of `tasks`")
    );

    let collection_mutation = compile(
        r#"app AnimatedCollectionMutation {
            state tasks: Array<Int32> = []
            body {
                Button("Add") {
                    withAnimation(Spring(response: 0.35, damping: 0.8)) { tasks.append(1) }
                }
            }
        }"#,
    )
    .expect_err("collection mutations must not silently ignore a custom animation spec");
    assert!(
        collection_mutation
            .to_string()
            .contains("cannot animate collection mutations of `tasks`")
    );

    let unobserved = compile(
        r#"app UnobservedAnimation {
            state progress: Float64 = 0.0
            body {
                Button("Complete") {
                    withAnimation(EaseIn) { progress = 1.0 }
                }
            }
        }"#,
    )
    .expect("unobserved float state updates should still compile");
    let Node::Button { actions, .. } = &unobserved.body[0] else {
        panic!("expected the unobserved animation button");
    };
    assert!(matches!(
        actions.as_slice(),
        [Action::WithAnimation {
            animated_states, ..
        }] if animated_states.is_empty()
    ));

    let empty = compile(
        r#"app EmptyAnimation {
            state progress: Float64 = 0.0
            body {
                Button("Animate") {
                    withAnimation(EaseIn) { }
                }
            }
        }"#,
    )
    .expect_err("empty animation blocks should be rejected");
    assert!(
        empty
            .to_string()
            .contains("must contain at least one mutable state assignment")
    );

    let side_effect = compile(
        r#"app AnimationSideEffect {
            state progress: Float64 = 0.0
            body {
                Button("Animate") {
                    withAnimation(EaseIn) {
                        Log.info(message: "not an animated state update")
                        progress = 1.0
                    }
                }
            }
        }"#,
    )
    .expect_err("unrelated side effects should not be hidden in animation blocks");
    assert!(
        side_effect
            .to_string()
            .contains("only supports mutable Float32 or Float64 state assignments"),
        "unexpected diagnostic: {side_effect}"
    );
}

#[test]
fn conditional_transitions_lower_to_typed_ir() {
    let module = compile(
        r#"app ConditionalTransitions {
            state visible = false
            body {
                if visible { Text("shown") }.transition(.fade, animation: Tween(duration: 0.24, delay: 0.04, easing: EaseOut))
                when visible {
                    true: { Text("yes") }
                    else: { Text("no") }
                }.transition(.scale)
            }
        }"#,
    )
    .expect("conditional transition declarations should compile");

    assert!(matches!(
        &module.body[0],
        Node::If {
            transition: Some(nexa_ir::ViewTransition::Fade),
            transition_animation: Some(nexa_ir::AnimationSpec::Tween {
                duration,
                delay,
                easing: nexa_ir::AnimationEasing::EaseOut,
            }),
            ..
        } if (*duration - 0.24).abs() < 0.001 && (*delay - 0.04).abs() < 0.001
    ));
    assert!(matches!(
        &module.body[1],
        Node::When {
            transition: Some(nexa_ir::ViewTransition::Scale),
            ..
        }
    ));
}

#[test]
fn when_allows_missing_else_only_for_exhaustive_closed_values() {
    let enum_match = compile(
        r#"app ExhaustiveEnumWhen {
            enum DeliveryStatus { Pending, Shipped }
            state status: DeliveryStatus = DeliveryStatus.Pending
            body {
                when status {
                    DeliveryStatus.Pending: { Text("Preparing") }
                    DeliveryStatus.Shipped: { Text("On the way") }
                }
            }
        }"#,
    )
    .expect("all declared enum cases should make the when exhaustive");
    assert!(matches!(
        enum_match.body.first(),
        Some(Node::When {
            else_body: None,
            cases,
            ..
        }) if cases.len() == 2
    ));

    let bool_match = compile(
        r#"app ExhaustiveBoolWhen {
            state isReady = false
            body {
                when isReady {
                    true: { Text("Ready") }
                    false: { Text("Waiting") }
                }
            }
        }"#,
    )
    .expect("true and false should make a boolean when exhaustive");
    assert!(matches!(
        bool_match.body.first(),
        Some(Node::When { else_body: None, cases, .. }) if cases.len() == 2
    ));

    let partial_enum = compile(
        r#"app PartialEnumWhen {
            enum DeliveryStatus { Pending, Shipped }
            state status: DeliveryStatus = DeliveryStatus.Pending
            body {
                when status {
                    DeliveryStatus.Pending: { Text("Preparing") }
                }
            }
        }"#,
    )
    .expect_err("a partial enum match needs an explicit fallback");
    assert!(
        partial_enum
            .to_string()
            .contains("when needs an `else` case unless it covers every Bool or enum case")
    );

    let string_match = compile(
        r#"app OpenStringWhen {
            state status: String = "pending"
            body { when status { "pending": { Text("Preparing") } } }
        }"#,
    )
    .expect_err("strings have an open value set and need a fallback");
    assert!(
        string_match
            .to_string()
            .contains("when needs an `else` case")
    );
}

#[test]
fn pressable_drag_bindings_are_typed_and_validated() {
    let module = compile(
        r#"app Drag {
            state total: Float64 = 0.0
            body {
                Pressable() {
                    Text("Drag")
                }.onTap {
                    total = 0.0
                }.onDrag { translationX, translationY, velocityX, velocityY ->
                    total = velocityX
                }
            }
        }"#,
    )
    .expect("drag bindings should compile as immutable Float64 values");

    let Node::Pressable {
        drag_parameters,
        drag_actions,
        ..
    } = &module.body[0]
    else {
        panic!("expected Pressable");
    };
    assert_eq!(
        drag_parameters,
        &["translationX", "translationY", "velocityX", "velocityY"]
    );
    assert!(matches!(
        &drag_actions[0],
        Action::Assign {
            value: Expr::State(name, Type::Numeric(NumericType::Float64)),
            ..
        } if name == "velocityX"
    ));

    for (parameters, expected) in [
        ("x, y", "requires four bindings"),
        ("x, x, vx, vy", "binds `x` more than once"),
    ] {
        let source = format!(
            "app Invalid {{ body {{ Pressable() {{ Text(\"Drag\") }}.onTap {{ }}.onDrag {{ {parameters} -> }} }} }}"
        );
        let error = compile(&source).expect_err("invalid drag bindings should be rejected");
        assert!(error.to_string().contains(expected), "{error}");
    }

    let shadow_error = compile(
        r#"app Invalid {
            state x: Float64 = 0.0
            body {
                Pressable() { Text("Drag") }.onTap { }.onDrag { x, y, vx, vy -> }
            }
        }"#,
    )
    .expect_err("drag bindings must not shadow app values");
    assert!(
        shadow_error
            .to_string()
            .contains("shadows an existing value")
    );
}

#[test]
fn pressable_context_menu_lowers_native_button_actions() {
    let module = compile(
        r#"app ContextMenu {
            body {
                Pressable() { Text("Row") }
                    .onTap { }
                    .contextMenu {
                        Button("Edit") { }
                        Button("Delete", icon: "delete") { }
                    }
            }
        }"#,
    )
    .expect("context menu buttons should lower as typed native button nodes");

    let Node::Pressable { context_menu, .. } = &module.body[0] else {
        panic!("expected Pressable");
    };
    assert_eq!(context_menu.len(), 2);
    assert!(
        context_menu
            .iter()
            .all(|node| matches!(node, Node::Button { .. }))
    );

    let error = compile(
        r#"app InvalidContextMenu {
            body {
                Pressable() { Text("Row") }
                    .onTap { }
                    .contextMenu { Text("Not an action") }
            }
        }"#,
    )
    .expect_err("context menus should reject non-action nodes");
    assert!(
        error
            .message
            .contains("requires at least one Button action")
    );
}

#[test]
fn pressable_drag_example_compiles_for_both_targets() {
    let source = include_str!("../../../examples/pressable_drag.nx");
    for target in [Target::Swift, Target::Kotlin] {
        let module = nexa_compiler::compile_for_target(source, target)
            .expect("the drag example should compile for each native target");
        assert!(matches!(
            &module.body[0],
            Node::Layout { children, .. }
                if children.iter().any(|node| matches!(node, Node::Pressable { .. }))
        ));
    }
}

#[test]
fn pressable_pinch_bindings_are_typed_and_validated() {
    let module = compile(
        r#"app Pinch {
            state zoom: Float64 = 1.0
            body {
                Pressable() { Text("Pinch") }
                    .onTap { }
                    .onPinch { scaleFactor -> zoom = zoom * scaleFactor }
            }
        }"#,
    )
    .expect("pinch scale delta should be an immutable Float64 binding");

    let Node::Pressable {
        pinch_parameter,
        pinch_actions,
        ..
    } = &module.body[0]
    else {
        panic!("expected Pressable");
    };
    assert_eq!(pinch_parameter.as_deref(), Some("scaleFactor"));
    assert!(matches!(
        &pinch_actions[0],
        Action::Assign {
            value: Expr::Arithmetic {
                right,
                ty: NumericType::Float64,
                ..
            },
            ..
        } if matches!(right.as_ref(), Expr::State(name, Type::Numeric(NumericType::Float64)) if name == "scaleFactor")
    ));

    for (handler, expected) in [
        ("zoom = 1.0", "requires one binding"),
        ("scaleFactor, extra ->", "requires one binding"),
    ] {
        let source = format!(
            "app Invalid {{ state zoom: Float64 = 0.0 body {{ Pressable() {{ Text(\"Pinch\") }}.onTap {{ }}.onPinch {{ {handler} }} }} }}"
        );
        let error = compile(&source).expect_err("invalid pinch bindings should be rejected");
        assert!(error.to_string().contains(expected), "{error}");
    }

    let shadow_error = compile(
        r#"app Invalid {
            state scaleFactor: Float64 = 1.0
            body {
                Pressable() { Text("Pinch") }.onTap { }.onPinch { scaleFactor -> }
            }
        }"#,
    )
    .expect_err("pinch callback bindings must not shadow app values");
    assert!(
        shadow_error
            .to_string()
            .contains("shadows an existing value")
    );
}

#[test]
fn pressable_pinch_example_compiles_for_both_targets() {
    let source = include_str!("../../../examples/pressable_pinch.nx");
    for target in [Target::Swift, Target::Kotlin] {
        let module = nexa_compiler::compile_for_target(source, target)
            .expect("the pinch example should compile for each native target");
        assert!(matches!(
            &module.body[0],
            Node::Layout { children, .. }
                if children.iter().any(|node| matches!(node, Node::Pressable { .. }))
        ));
    }
}

#[test]
fn pressable_long_press_duration_is_typed_and_tap_uses_its_canonical_name() {
    let module = compile(
        r#"app LongPress {
            state holds: Int32 = 0
            state duration: Int32 = 650
            body {
                Pressable() { Text("Hold") }
                    .onTap { holds = holds + 1 }
                    .onLongPress(durationMs: duration) { holds = holds + 1 }
            }
        }"#,
    )
    .expect("typed duration and canonical tap handler should compile");

    let Node::Pressable {
        actions,
        long_press_duration_ms,
        long_press_actions,
        ..
    } = &module.body[0]
    else {
        panic!("expected Pressable");
    };
    assert!(matches!(
        long_press_duration_ms,
        Expr::State(name, Type::Numeric(NumericType::Int32)) if name == "duration"
    ));
    assert_eq!(actions.len(), 1);
    assert_eq!(long_press_actions.len(), 1);

    for duration in ["\"slow\"", "true"] {
        let source = format!(
            "app Invalid {{ body {{ Pressable() {{ Text(\"Hold\") }}.onTap {{ }}.onLongPress(durationMs: {duration}) {{ }} }} }}"
        );
        let error = compile(&source).expect_err("long-press duration must have type Int32");
        assert!(error.to_string().contains("expected Int32"), "{error}");
    }

    let duplicate_tap = compile(
        r#"app Invalid {
            body {
                Pressable() { Text("Tap") }.onTap { }.onTap { }
            }
        }"#,
    )
    .expect_err("a Pressable cannot install the same tap callback twice");
    assert!(
        duplicate_tap
            .to_string()
            .contains("only one `.onTap` modifier")
    );
}

#[test]
fn visual_modifiers_reject_invalid_ranges_and_shapes() {
    for (body, expected) in [
        (
            "Column(opacity: 1.1) { Text(\"x\") }",
            "opacity must be between 0 and 1",
        ),
        ("Column(blur: -1) { Text(\"x\") }", "blur radius"),
        ("Column(clip: Circle()) { Text(\"x\") }", "Rounded(radius)"),
        ("Column(zIndex: 2147483648) { Text(\"x\") }", "Int32 range"),
    ] {
        let source = format!("app Invalid {{ body {{ {body} }} }}");
        let error = compile(&source).expect_err("invalid visual style arguments should fail");
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn accessibility_options_lower_on_builtins_and_custom_components() {
    let module = compile(
        r#"
        component Caption(title: String) {
            body { Text(title) }
        }

        app AccessibilityOptions {
            body {
                Text("Continue", accessibilityLabel: "Continue action", accessibilityHint: "Opens the next screen", accessibilityValue: "Ready", accessibilityRole: Button)
                Image(asset: "brand", description: "Brand", accessibilityLabel: "Brand mark", accessibilityRole: Image)
                Caption(title: "Profile", accessibilityLabel: "Profile heading", accessibilityRole: Header)
            }
        }
        "#,
    )
    .expect("component accessibility options should compile");

    let [
        Node::Accessibility {
            label,
            hint: Some(hint),
            value: Some(accessibility_value),
            role: AccessibilityRole::Button,
            children: text_children,
        },
        Node::Accessibility {
            label: image_label,
            role: AccessibilityRole::Image,
            children: image_children,
            ..
        },
        Node::Accessibility {
            label: custom_label,
            role: AccessibilityRole::Header,
            children: custom_children,
            ..
        },
    ] = module.body.as_slice()
    else {
        panic!("accessibility parameters should become typed annotations on each component");
    };
    assert_eq!(source_text(label), Some("Continue action"));
    assert_eq!(source_text(hint), Some("Opens the next screen"));
    assert_eq!(source_text(accessibility_value), Some("Ready"));
    assert_eq!(source_text(image_label), Some("Brand mark"));
    assert_eq!(source_text(custom_label), Some("Profile heading"));
    assert!(matches!(text_children.as_slice(), [Node::Text { .. }]));
    assert!(matches!(image_children.as_slice(), [Node::Image { .. }]));
    assert!(
        matches!(custom_children.as_slice(), [Node::ComponentCall { name, arguments, .. }]
        if name == "Caption" && arguments.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>() == ["title"])
    );
}

#[test]
fn accessibility_options_require_a_valid_nonempty_label_and_role() {
    for (node, expected) in [
        (
            "Text(\"x\", accessibilityHint: \"hint\")",
            "accessibilityLabel is required",
        ),
        (
            "Text(\"x\", accessibilityLabel: \"\")",
            "Accessibility label cannot be empty",
        ),
        (
            "Text(\"x\", accessibilityLabel: \"x\", accessibilityHint: \"\")",
            "Accessibility hint cannot be empty",
        ),
        (
            "Text(\"x\", accessibilityLabel: \"x\", accessibilityRole: Slider)",
            "Accessibility role must be",
        ),
        (
            "Accessibility(label: \"x\") { Text(\"x\") }",
            "unknown component `Accessibility`",
        ),
    ] {
        let source = format!("app Invalid {{ body {{ {node} }} }}");
        let error = compile(&source).expect_err("invalid accessibility source should fail");
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn image_shared_element_modifier_lowers_a_typed_string_identifier() {
    let module = compile(
        r#"
        app SharedImage {
            body {
                Image(asset: "hero", description: "Hero")
                    .sharedElement(id: "product-hero")
            }
        }
        "#,
    )
    .expect("shared element identifiers should be type checked");

    assert!(matches!(
        module.body.as_slice(),
        [Node::Image {
            shared_element: Some(Expr::String(id)),
            ..
        }] if id == "product-hero"
    ));
}

#[test]
fn image_max_height_lowers_to_a_bounded_native_dimension() {
    let module = compile(
        r#"
        app CommentPhoto {
            body {
                Image(file: "file:///photo.jpg", description: "Comment image", maxHeight: 200)
            }
        }
        "#,
    )
    .expect("a bounded comment image should compile");

    assert!(matches!(
        module.body.as_slice(),
        [Node::Image {
            max_height: Some(height),
            ..
        }] if (*height - 200.0).abs() < f32::EPSILON
    ));
}

#[test]
fn image_fill_max_size_lowers_to_shared_image_sizing() {
    let module = compile(
        r#"
        app FullPagePoster {
            body {
                Image(asset: "video_poster", description: "Video poster", scale: Fill, fillMaxSize: true)
            }
        }
        "#,
    )
    .expect("full-page image sizing should compile");

    assert!(matches!(
        module.body.as_slice(),
        [Node::Image {
            fill_max_size: true,
            scale: nexa_ir::ImageScale::Fill,
            ..
        }]
    ));
}

#[test]
fn image_loading_indicator_is_opt_in_and_remote_or_file_only() {
    let default_module = compile(
        r#"
        app DefaultImageLoading {
            body {
                Image(url: "https://example.test/image.png", description: "Remote image")
            }
        }
        "#,
    )
    .expect("remote images should default to no loading indicator");
    assert!(matches!(
        default_module.body.as_slice(),
        [Node::Image {
            show_loading_indicator: false,
            ..
        }]
    ));

    let enabled_module = compile(
        r#"
        app ExplicitImageLoading {
            body {
                Image(file: "file:///tmp/image.png", description: "Local image", showLoadingIndicator: true)
            }
        }
        "#,
    )
    .expect("local files can opt into the loading indicator");
    assert!(matches!(
        enabled_module.body.as_slice(),
        [Node::Image {
            show_loading_indicator: true,
            ..
        }]
    ));

    let asset_error = compile(
        r#"
        app InvalidAssetLoading {
            body {
                Image(asset: "logo", description: "Logo", showLoadingIndicator: true)
            }
        }
        "#,
    )
    .expect_err("local assets do not load asynchronously");
    assert!(
        asset_error
            .to_string()
            .contains("only be used with a URL or file")
    );
}

#[test]
fn slider_lowers_a_mutable_float64_binding_and_static_range() {
    let module = compile(
        r#"
        app VolumeControl {
            state volume: Float64 = 0.5

            body {
                Slider(value: volume, min: 0.0, max: 1.0, step: 0.1)
            }
        }
        "#,
    )
    .expect("a valid stepped Float64 slider should compile");

    assert!(matches!(
        &module.body[0],
        Node::Slider {
            state, min, max, step, ..
        }
            if state == "volume"
                && matches!(min, Expr::Number { raw, .. } if raw == "0.0")
                && matches!(max, Expr::Number { raw, .. } if raw == "1.0")
                && *step == 0.1
    ));

    let negative_range = compile(
        r#"
        app SignedSlider {
            state balance: Float64 = 0.0
            body {
                Slider(value: balance, min: -1.0, max: 1.0, step: 0.5)
            }
        }
        "#,
    )
    .expect("negative numeric range endpoints should be accepted");
    assert!(matches!(
        &negative_range.body[0],
        Node::Slider { min, max, .. }
            if matches!(min, Expr::Number { raw, .. } if raw == "-1.0")
                && matches!(max, Expr::Number { raw, .. } if raw == "1.0")
    ));

    let invalid = compile(
        r#"
        app InvalidSlider {
            state volume: Float64 = 0.5
            body {
                Slider(value: volume, min: 0.0, max: 1.0, step: 0.3)
            }
        }
        "#,
    );
    assert!(invalid.is_err(), "the range must divide evenly into steps");
}

#[test]
fn slider_accepts_a_dynamic_range_and_typed_user_events() {
    let module = compile(
        r#"
        app PlayerSeekBar {
            state position: Float64 = 12.0
            state duration: Float64 = 185.0
            state isSeeking: Bool = false

            body {
                Slider(value: position, min: 0.0, max: duration, step: 0.0)
                    .onChange { seconds -> position = seconds }
                    .onEditingChanged { editing -> isSeeking = editing }
            }
        }
        "#,
    )
    .expect("a dynamic playback range and both slider events should compile");

    let Node::Slider {
        min,
        max,
        step,
        on_change,
        on_editing_changed,
        ..
    } = &module.body[0]
    else {
        panic!("expected Slider IR");
    };
    assert!(matches!(min, Expr::Number { raw, .. } if raw == "0.0"));
    assert!(matches!(max, Expr::State(name, _) if name == "duration"));
    assert_eq!(*step, 0.0);
    assert_eq!(
        on_change.as_ref().map(|change| change.parameter.as_str()),
        Some("seconds")
    );
    assert_eq!(
        on_editing_changed
            .as_ref()
            .map(|change| change.parameter.as_str()),
        Some("editing")
    );

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(swift.contains("maxValue: nexa_duration"), "{swift}");
    assert!(
        kotlin.contains("maxValue = (nexa_duration).toFloat()"),
        "{kotlin}"
    );
}

#[test]
fn progress_controls_lower_float64_expressions() {
    let module = compile(
        r##"
        app ProgressControls {
            state progress: Float64 = 0.5
            state tint: String = "#2563EB"
            state wave: Bool = true
            body {
                ProgressBar(progress: progress)
                ProgressRing(progress: progress + 0.1, tint: tint, trackColor: "#DBEAFE", size: 48, thickness: 5, androidWave: wave)
            }
        }
        "##,
    )
    .expect("native progress controls should accept Float64 expressions");

    assert!(matches!(
        &module.body[0],
        Node::ProgressBar { progress: Some(Expr::State(name, Type::Numeric(NumericType::Float64))), .. }
            if name == "progress"
    ));
    assert!(matches!(
        &module.body[1],
        Node::ProgressRing {
            progress: Some(Expr::Add(_, _, NumericType::Float64)),
            style: Some(style),
        }
            if matches!(style.tint.as_ref(), Some(ColorExpression::Dynamic(expression)) if matches!(expression.as_ref(), Expr::State(name, Type::String) if name == "tint"))
                && matches!(style.track_color.as_ref(), Some(ColorExpression::Static(_)))
                && style.size == Some(48.0)
                && style.thickness == Some(5.0)
                && matches!(style.android_wave.as_ref(), Some(Expr::State(name, Type::Bool)) if name == "wave")
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("NexaProgressRingPrimitive(progress:"));
    assert!(swift.contains("tint: nexaColor(hex: nexa_tint)"));
    assert!(swift.contains("trackColor: Color(red:"));
    assert!(swift.contains("size: 48, thickness: 5)"));
    assert!(!swift.contains("androidWave"));
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("NexaProgressRingPrimitive(progress ="));
    assert!(kotlin.contains("tint = nexaColorFromHex(nexa_tint)"));
    assert!(kotlin.contains("trackColor = Color(0xFFDBEAFE)"));
    assert!(kotlin.contains("size = 48f, thickness = 5f, androidWave = nexa_wave)"));
    assert!(kotlin.contains("androidWave = nexa_wave"));
}

#[test]
fn progress_controls_reject_the_old_snake_case_option_name() {
    let error = compile("app OldProgressOption { body { ProgressRing(android_wave: true) } }")
        .expect_err("the component catalog has one canonical camelCase option");
    assert!(
        error.to_string().contains("unknown option `android_wave`"),
        "{error}"
    );
}

#[test]
fn progress_ring_rejects_negative_dimensions() {
    for dimension in ["size: -1", "thickness: -1"] {
        let source = format!(
            "app InvalidProgressRing {{ body {{ ProgressRing(progress: 0.5, {dimension}) }} }}"
        );
        let error = compile(&source).expect_err("negative ring dimensions must be rejected");
        assert!(error.to_string().contains("non-negative"), "{error}");
    }

    let error = compile("app InvalidProgressRingWave { body { ProgressRing(androidWave: 1) } }")
        .expect_err("androidWave must be a Bool");
    assert!(error.to_string().contains("Bool"), "{error}");
}

#[test]
fn omitted_progress_compiles_to_native_indeterminate_indicators() {
    let module = compile("app LoadingState { body { ProgressBar() } }")
        .expect("ProgressBar without a value should use indeterminate mode");
    assert!(matches!(
        module.body.first(),
        Some(Node::ProgressBar {
            progress: None,
            style: None
        })
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains(
        "NexaProgressBarPrimitive(progress: nil, tint: nil, trackColor: nil, thickness: nil, cornerRadius: nil)"
    ));
    assert!(swift.contains("indeterminateBar(tint: Color.accentColor)"));
    assert!(swift.contains(".accessibilityRepresentation { ProgressView() }"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains(
        "NexaProgressBarPrimitive(progress = null, tint = null, trackColor = null, thickness = null, cornerRadius = null)"
    ));
    assert!(kotlin.contains("if (normalizedProgress == null) {\n        androidx.compose.material3.LinearProgressIndicator("));

    let ring_module = compile("app LoadingRing { body { ProgressRing() } }")
        .expect("ProgressRing without a value should use indeterminate mode");
    assert!(matches!(
        ring_module.body.first(),
        Some(Node::ProgressRing {
            progress: None,
            style: None
        })
    ));
    let swift = nexa_backend_swift::SwiftBackend.generate(&ring_module);
    assert!(swift.contains(
        "NexaProgressRingPrimitive(progress: nil, tint: nil, trackColor: nil, size: nil, thickness: nil)"
    ));
    assert!(swift.contains(".trim(from: 0, to: 0.25)"));
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&ring_module);
    assert!(kotlin.contains(
        "NexaProgressRingPrimitive(progress = null, tint = null, trackColor = null, size = null, thickness = null)"
    ));
    assert!(kotlin.contains("val normalizedProgress = progress?.let"));
}

#[test]
fn progress_bar_style_is_typed_and_reaches_both_native_backends() {
    let module = compile(
        r##"
        app StyledProgress {
            state progress: Float64 = 0.45
            state fill: String = "#7C3AED"
            state label: String = "Upload progress"
            state hint: String = "Shows task completion"
            state value: String = "45 percent"
            state wave: Bool = true
            body {
                ProgressBar(
                    progress: progress,
                    tint: fill,
                    trackColor: "#EDE9FE",
                    thickness: 12,
                    cornerRadius: 6,
                    accessibilityLabel: label,
                    accessibilityHint: hint,
                    accessibilityValue: value,
                    accessibilityRole: None,
                    androidWave: wave
                )
            }
        }
        "##,
    )
    .expect("typed progress styles should compile");

    let Some(Node::Accessibility {
        label,
        hint: Some(hint),
        value: Some(value),
        role,
        children,
    }) = module.body.first()
    else {
        panic!("ProgressBar accessibility parameters should lower to a typed wrapper");
    };
    assert!(matches!(label, Expr::State(name, Type::String) if name == "label"));
    assert!(matches!(hint, Expr::State(name, Type::String) if name == "hint"));
    assert!(matches!(value, Expr::State(name, Type::String) if name == "value"));
    assert!(matches!(role, AccessibilityRole::None));
    assert!(matches!(
        children.first(),
        Some(Node::ProgressBar {
            style: Some(style),
            ..
        }) if matches!(style.tint.as_ref(), Some(ColorExpression::Dynamic(_)))
            && matches!(style.track_color.as_ref(), Some(ColorExpression::Static(_)))
            && style.thickness == Some(12.0)
            && style.corner_radius == Some(6.0)
            && matches!(style.android_wave.as_ref(), Some(Expr::State(name, Type::Bool)) if name == "wave")
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("NexaProgressBarPrimitive(progress: nexa_progress"));
    assert!(swift.contains("trackColor: Color(red:"));
    assert!(swift.contains("thickness: 12, cornerRadius: 6)"));
    assert!(swift.contains("NexaAccessibilityPrimitive("));
    assert!(swift.contains("hint: Text(nexa_hint)"));
    assert!(swift.contains("value: Text(nexa_value)"));
    assert!(swift.contains("role: \"None\""));
    assert!(!swift.contains("androidWave"));
    assert!(!swift.contains("android_wave"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("NexaProgressBarPrimitive(progress = nexa_progress.toFloat()"));
    assert!(kotlin.contains(
        "trackColor = Color(0xFFEDE9FE), thickness = 12f, cornerRadius = 6f, androidWave = nexa_wave)"
    ));
    assert!(kotlin.contains("import androidx.compose.ui.graphics.Color"));
    assert!(kotlin.contains("NexaAccessibilityPrimitive("));
    assert!(kotlin.contains("hint = nexa_hint"));
    assert!(kotlin.contains("value = nexa_value"));
    assert!(kotlin.contains("role = null"));
    assert!(kotlin.contains("androidWave = nexa_wave"));
    assert!(kotlin.contains("if (androidWave) {"));
    assert!(kotlin.contains("androidx.compose.material3.LinearWavyProgressIndicator("));
    let (_, project_features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(project_features.uses_wavy_progress_bar);

    let invalid_wave = compile(
        r#"app InvalidProgressWave { body { ProgressBar(progress: 0.5, androidWave: "yes") } }"#,
    )
    .expect_err("androidWave must be a typed Boolean expression");
    assert!(invalid_wave.to_string().contains("Bool"), "{invalid_wave}");

    for dimension in ["thickness: -1", "cornerRadius: -1"] {
        let source = format!(
            "app InvalidProgressStyle {{ body {{ ProgressBar(progress: 0.5, {dimension}) }} }}"
        );
        let error = compile(&source).expect_err("negative visual dimensions must be rejected");
        assert!(error.to_string().contains("non-negative"), "{error}");
    }
}

#[test]
fn segmented_control_lowers_a_string_array_and_mutable_string_binding() {
    let module = compile(
        r#"
        app Filters {
            state filters: Array<String> = ["All", "Open", "Closed"]
            state selectedFilter: String = "All"

            body {
                SegmentedControl(items: filters, selected: selectedFilter)
            }
        }
        "#,
    )
    .expect("a string array and mutable string selection should compile");

    assert!(matches!(
        &module.body[0],
        Node::SegmentedControl {
            items: Expr::State(name, Type::Array(element)),
            state,
            animation,
            ..
        } if name == "filters" && **element == Type::String && state == "selectedFilter" && animation.is_none()
    ));

    let custom_animation = compile(
        r#"
        app AnimatedFilters {
            state filters: Array<String> = ["All", "Open"]
            state selectedFilter: String = "All"
            body {
                SegmentedControl(
                    items: filters,
                    selected: selectedFilter,
                    animation: Tween(duration: 0.24, easing: EaseOut)
                )
            }
        }
        "#,
    )
    .expect("a segmented control should accept a typed animation spec");
    assert!(matches!(
        &custom_animation.body[0],
        Node::SegmentedControl {
            animation: Some(nexa_ir::AnimationSpec::Tween {
                duration,
                easing: nexa_ir::AnimationEasing::EaseOut,
                ..
            }),
            ..
        } if (*duration - 0.24).abs() < f64::EPSILON
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&custom_animation);
    assert!(
        swift.contains("selection: $nexa_selectedFilter, animation: .timingCurve(0.0, 0.0, 0.2, 1.0, duration: 0.24)"),
        "{swift}"
    );
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&custom_animation);
    assert!(
        kotlin.contains("animation = androidx.compose.animation.core.tween(durationMillis = 240, easing = androidx.compose.animation.core.LinearOutSlowInEasing)")
    );

    let invalid = compile(
        r#"
        app InvalidFilters {
            let filters: Array<Int32> = [1, 2]
            state selectedFilter: String = "All"
            body {
                SegmentedControl(items: filters, selected: selectedFilter)
            }
        }
        "#,
    );
    assert!(invalid.is_err(), "segmented options must be Array<String>");
}

#[test]
fn segmented_control_visual_parameters_reach_both_native_backends_and_dev_ir() {
    let module = compile(
        r##"
        app StyledSegments {
            state choices: Array<String> = ["Day", "Week", "Month"]
            state selected: String = "Day"
            state selectedFill: String = "#FFFFFF"
            body {
                SegmentedControl(
                    items: choices,
                    selected: selected,
                    tint: selectedFill,
                    background: "#E9EAF0",
                    selectedTextColor: "#17171A",
                    textColor: "#68686F",
                    fontSize: 14,
                    fontWeight: Semibold,
                    height: 36,
                    cornerRadius: 18,
                    borderColor: selectedFill,
                    borderWidth: 1
                )
            }
        }
        "##,
    )
    .expect("visual segmented-control options should typecheck");

    let Node::SegmentedControl {
        tint: Some(nexa_ir::ColorExpression::Dynamic(_)),
        background: Some(nexa_ir::ColorExpression::Static(_)),
        selected_text_color: Some(nexa_ir::ColorExpression::Static(_)),
        text_color: Some(nexa_ir::ColorExpression::Static(_)),
        font_size: Some(font_size),
        font_weight: Some(nexa_ir::FontWeight::Semibold),
        height: Some(height),
        corner_radius: Some(corner_radius),
        border_color: Some(nexa_ir::ColorExpression::Dynamic(_)),
        border_width: Some(border_width),
        ..
    } = &module.body[0]
    else {
        panic!("every visual option should be represented in typed IR");
    };
    assert_eq!(
        (*font_size, *height, *corner_radius, *border_width),
        (14.0, 36.0, 18.0, 1.0)
    );

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(
        swift.contains("tint: nexaColor(hex: nexa_selectedFill)"),
        "{swift}"
    );
    assert!(swift.contains("background: Color(red:"), "{swift}");
    assert!(swift.contains("textColor: Color(red:"), "{swift}");
    assert!(swift.contains("selectedTextColor: Color(red:"), "{swift}");
    assert!(swift.contains("fontSize: 14"), "{swift}");
    assert!(swift.contains("fontWeight: .semibold"), "{swift}");
    assert!(swift.contains("height: 36"), "{swift}");
    assert!(swift.contains("cornerRadius: 18"), "{swift}");
    assert!(
        swift.contains("borderColor: nexaColor(hex: nexa_selectedFill)"),
        "{swift}"
    );
    assert!(swift.contains("borderWidth: 1"), "{swift}");

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(
        kotlin.contains("tint = nexaColorFromHex(nexa_selectedFill)"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("background = Color(0xFFE9EAF0)"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("selectedTextColor = Color(0xFF17171A)"),
        "{kotlin}"
    );
    assert!(kotlin.contains("textColor = Color(0xFF68686F)"), "{kotlin}");
    assert!(kotlin.contains("fontSize = 14f.sp"), "{kotlin}");
    assert!(
        kotlin.contains("fontWeight = androidx.compose.ui.text.font.FontWeight.SemiBold"),
        "{kotlin}"
    );
    assert!(kotlin.contains("height = 36f.dp"), "{kotlin}");
    assert!(kotlin.contains("cornerRadius = 18f.dp"), "{kotlin}");
    assert!(
        kotlin.contains("borderColor = nexaColorFromHex(nexa_selectedFill)"),
        "{kotlin}"
    );
    assert!(kotlin.contains("borderWidth = 1f.dp"), "{kotlin}");
}

#[test]
fn segmented_control_rejects_negative_visual_dimensions() {
    for argument in ["height: -1", "cornerRadius: -1", "borderWidth: -1"] {
        let source = format!(
            "app InvalidSegments {{ state selected: String = \"Day\" body {{ SegmentedControl(items: [\"Day\"], selected: selected, {argument}) }} }}"
        );
        assert!(compile(&source).is_err(), "should reject {argument}");
    }
}

#[test]
fn picker_lowers_a_string_array_and_mutable_string_binding() {
    let module = compile(
        r#"
        app SizePicker {
            state sizes: Array<String> = ["Small", "Medium", "Large"]
            state selectedSize: String = "Medium"

            body {
                Picker(items: sizes, selected: selectedSize)
            }
        }
        "#,
    )
    .expect("a string array and mutable string selection should compile");

    assert!(matches!(
        &module.body[0],
        Node::Picker {
            items: Expr::State(name, Type::Array(element)),
            state,
            label: None,
            ..
        } if name == "sizes" && **element == Type::String && state == "selectedSize"
    ));

    let invalid = compile(
        r#"
        app InvalidPicker {
            let sizes: Array<Int32> = [1, 2]
            state selectedSize: String = "Medium"
            body {
                Picker(items: sizes, selected: selectedSize)
            }
        }
        "#,
    );
    assert!(invalid.is_err(), "picker options must be Array<String>");
}

#[test]
fn picker_tint_lowers_a_runtime_color_expression() {
    let module = compile(
        r##"
        app PriorityPicker {
            state options: Array<String> = ["None", "High"]
            state selected: String = "High"
            state priorityTint: String = "#FF9500"

            body {
                Picker(items: options, selected: selected, icon: "flag_filled", tint: priorityTint)
            }
        }
        "##,
    )
    .expect("Picker should accept a runtime tint color");

    assert!(matches!(
        module.body.first(),
        Some(Node::Picker {
            tint: Some(ColorExpression::Dynamic(color)),
            ..
        }) if matches!(color.as_ref(), Expr::State(name, Type::String) if name == "priorityTint")
    ));
}

#[test]
fn picker_item_icons_are_resolved_to_shared_system_icons() {
    let module = compile(
        r#"
        app PickerRowIcons {
            state options: Array<String> = ["Due date", "Priority", "Title"]
            state selected: String = "Priority"

            body {
                Picker(
                    items: options,
                    selected: selected,
                    itemIcons: ["calendar", "flag_filled", "edit"]
                )
            }
        }
        "#,
    )
    .expect("static per-option shared icon names should compile");

    assert!(matches!(
        module.body.first(),
        Some(Node::Picker { item_icons, .. })
            if item_icons == &vec![
                SystemIcon::Shared("calendar".to_owned()),
                SystemIcon::Shared("flag_filled".to_owned()),
                SystemIcon::Shared("edit".to_owned()),
            ]
    ));

    let invalid = compile(
        r#"
        app InvalidPickerIcons {
            state options: Array<String> = ["First"]
            state selected: String = "First"
            body { Picker(items: options, selected: selected, itemIcons: ["not_a_system_icon"]) }
        }
        "#,
    );
    assert!(
        invalid.is_err(),
        "unknown option icons must fail compilation"
    );
}

#[test]
fn picker_date_parameters_are_typed_and_reach_both_native_backends() {
    let module = compile(
        r##"
        app DateTimeControls {
            state selected: Int64 = 1792153800000
            state rangeStart: Int64 = 1791763200000
            state rangeEnd: Int64 = 1792454400000
            state lowerBound: Int64 = 1791763200000
            state upperBound: Int64 = 1792454400000
            state accent: String = "#2563EB"
            state localeTag: String = "fr-FR"
            state zoneId: String = "Europe/Paris"
            state isEnabled: Bool = true

            body {
                PickerDate(
                    type: Date,
                    timestamp: selected,
                    style: Graphical,
                    minimum: lowerBound,
                    maximum: upperBound,
                    tint: accent,
                    textColor: "#172033",
                    backgroundColor: "#F1F5F9",
                    pickerBackgroundColor: "#FFFFFF",
                    borderColor: "#CBD5E1",
                    borderWidth: 1,
                    cornerRadius: 16,
                    locale: localeTag,
                    timeZone: zoneId,
                    enabled: isEnabled
                )
                PickerDate(
                    type: Date,
                    style: Graphical,
                    selection: Range,
                    startTimestamp: rangeStart,
                    endTimestamp: rangeEnd,
                    minimum: lowerBound,
                    maximum: upperBound
                )
                PickerDate(
                    type: Date,
                    style: Compact,
                    selection: Range,
                    startTimestamp: rangeStart,
                    endTimestamp: rangeEnd,
                    minimum: lowerBound,
                    maximum: upperBound
                )
                PickerDate(type: Time, style: Wheel, timestamp: selected, is24Hour: true)
                PickerDate(type: Date, style: Input, timestamp: selected)
                PickerDate(type: DateTime, style: Compact, timestamp: selected)
            }
        }
        "##,
    )
    .expect("date, time, date-time, range, style, and visual parameters should typecheck");

    let Node::PickerDate { props } = &module.body[0] else {
        panic!(
            "the first picker should lower to its own typed node: {:?}",
            module.body
        );
    };
    assert_eq!(props.picker_type, nexa_ir::PickerDateType::Date);
    assert_eq!(props.style, nexa_ir::PickerDateStyle::Graphical);
    assert_eq!(props.selection, nexa_ir::PickerDateSelection::Single);
    assert_eq!(props.timestamp_state.as_deref(), Some("selected"));
    assert!(props.minimum.is_some() && props.maximum.is_some());
    assert!(matches!(props.tint, Some(ColorExpression::Dynamic(_))));
    assert!(matches!(props.text_color, Some(ColorExpression::Static(_))));
    assert!(matches!(
        props.background_color,
        Some(ColorExpression::Static(_))
    ));
    assert!(matches!(
        props.picker_background_color,
        Some(ColorExpression::Static(_))
    ));
    assert!(matches!(
        props.border_color,
        Some(ColorExpression::Static(_))
    ));
    assert_eq!(
        (props.border_width, props.corner_radius),
        (Some(1.0), Some(16.0))
    );
    assert!(props.locale.is_some() && props.time_zone.is_some() && props.enabled.is_some());

    let Node::PickerDate { props } = &module.body[1] else {
        panic!("the second picker should lower to the range node");
    };
    assert_eq!(props.selection, nexa_ir::PickerDateSelection::Range);
    assert_eq!(props.start_timestamp_state.as_deref(), Some("rangeStart"));
    assert_eq!(props.end_timestamp_state.as_deref(), Some("rangeEnd"));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    for expected in [
        "NexaDatePickerPrimitive(",
        "pickerType: \"Date\"",
        "style: \"Graphical\"",
        "style: \"Input\"",
        "selection: \"Range\"",
        "timestamp:",
        "startTimestamp:",
        "endTimestamp:",
        "minimum:",
        "maximum:",
        "tint:",
        "textColor:",
        "backgroundColor:",
        "pickerBackgroundColor:",
        "borderColor:",
        "borderWidth:",
        "cornerRadius:",
        "localeIdentifier:",
        "timeZoneIdentifier:",
        "is24Hour:",
        "enabled:",
        "accessibilityLabel:",
    ] {
        assert!(
            swift.contains(expected),
            "missing `{expected}` in Swift output:\n{swift}"
        );
    }

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    for expected in [
        "NexaDatePickerPrimitive(",
        "pickerType = \"Date\"",
        "style = \"Graphical\"",
        "style = \"Input\"",
        "selection = \"Range\"",
        "timestamp =",
        "startTimestamp =",
        "endTimestamp =",
        "minimum =",
        "maximum =",
        "tint =",
        "textColor =",
        "backgroundColor =",
        "pickerBackgroundColor =",
        "borderColor =",
        "borderWidth =",
        "cornerRadius =",
        "localeTag =",
        "timeZoneId =",
        "is24Hour =",
        "enabled =",
        "accessibilityLabel =",
    ] {
        assert!(
            kotlin.contains(expected),
            "missing `{expected}` in Kotlin output:\n{kotlin}"
        );
    }
}

#[test]
fn picker_date_rejects_invalid_style_and_range_combinations() {
    for source in [
        r#"app BadStyle { body { PickerDate(type: Date, style: UnknownStyle) } }"#,
        r#"app BadRange { body { PickerDate(type: Time, selection: Range) } }"#,
        r#"app PartialRange { state start: Int64 = 1 body { PickerDate(type: Date, selection: Range, startTimestamp: start) } }"#,
        r#"app ConflictingBindings { state date: Int64 = 1 body { PickerDate(type: Date, selection: Range, timestamp: date) } }"#,
    ] {
        assert!(
            compile(source).is_err(),
            "invalid declaration should fail: {source}"
        );
    }
}

#[test]
fn dialog_lowers_typed_text_and_actions_for_mutable_state() {
    let module = compile(
        r#"
        app DeleteConfirmation {
            state isPresented: Bool = false
            state deleted: Bool = false
            state comment: String = ""

            body {
                Dialog(isPresented: isPresented, title: "Delete item?", message: "") {
                    TextInput(value: comment, placeholder: "Comment")
                    Button("Cancel") { isPresented = false }
                    Button("Delete") {
                        deleted = true
                        isPresented = false
                    }
                }
            }
        }
        "#,
    )
    .expect("dialog text, state binding, and native action buttons should compile");

    assert!(matches!(
        &module.body[0],
        Node::Dialog {
            state,
            title,
            message,
            children,
            ..
        } if state == "isPresented"
            && source_text(title) == Some("Delete item?")
            && source_text(message) == Some("")
            && children.len() == 3
            && matches!(children.first(), Some(Node::TextInput { state, .. }) if state == "comment")
            && children[1..].iter().all(|node| matches!(node, Node::Button { .. }))
    ));

    let invalid = compile(
        r#"
        app InvalidDialogInput {
            state isPresented: Bool = false
            state first: String = ""
            state second: String = ""
            body {
                Dialog(isPresented: isPresented, title: "Edit", message: "") {
                    TextInput(value: first, placeholder: "First")
                    TextInput(value: second, placeholder: "Second")
                    Button("Save") { isPresented = false }
                }
            }
        }
        "#,
    );
    assert!(invalid.is_err(), "dialog supports only one text field");

    let invalid = compile(
        r#"
        app InvalidDialog {
            state isPresented: Bool = false
            body {
                Dialog(isPresented: isPresented, title: 1, message: "Message") { }
            }
        }
        "#,
    );
    assert!(invalid.is_err(), "dialog title must be a String expression");
}

#[test]
fn unused_native_plugins_are_pruned_from_both_target_modules() {
    let project = TestProject::new("nexa-plugin-pruning");
    for (directory, id, service) in [
        ("used", "dev.example.used", "Used"),
        ("unused", "dev.example.unused", "Unused"),
    ] {
        let plugin = project.join(directory);
        fs::create_dir_all(&plugin).expect("plugin directory should be created");
        fs::write(
                plugin.join("plugin.config.nx"),
                format!(
                    "plugin {{ schema: 2 id: \"{id}\" version: \"1.0.0\" sources {{ native: \"native.nxid\" }} }}\n"
                ),
            )
            .expect("plugin manifest should be written");
        fs::write(
            plugin.join("native.nxid"),
            format!("service {service} {{ fn ping() }}\n"),
        )
        .expect("plugin contract should be written");
    }
    let entry = project.join("main.nx");
    fs::write(
            &entry,
            "plugin \"used\" as Used\nplugin \"unused\" as Unused\napp Demo { body { Button(\"Ping\") { Used.ping() } } }\n",
        )
        .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("app with both valid plugin contracts should compile")
            .module;
        assert_eq!(
            module
                .plugins
                .iter()
                .map(|plugin| plugin.namespace.as_str())
                .collect::<Vec<_>>(),
            ["Used"],
            "unused plugin declarations must not reach native project generation"
        );
    }
}

#[test]
fn nested_borrowed_components_keep_the_owner_instance_live() {
    let project = TestProject::new("nexa-nested-components");
    let plugin = project.join("resource");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.example.resource\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
        )
        .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        "native class Resource { init() fn play() fn dispose() }\n",
    )
    .expect("native contract should be written");

    let entry = project.join("main.nx");
    fs::write(
        &entry,
        r#"
            plugin "resource" as ResourcePlugin

            component Outer(resource: ResourcePlugin.Resource) {
                body { Inner(resource: resource) }
            }

            component Inner(resource: ResourcePlugin.Resource) {
                body { Button("Use") { resource.play() } }
            }

            app Demo {
                let resource = ResourcePlugin.Resource()
                body {
                    Button("Dispose") { resource.dispose() }
                    Outer(resource: resource)
                }
            }
            "#,
    )
    .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let result = compile_file_with_warnings_for_target(&entry, target);
        let Err(error) = result else {
            panic!(
                "a parent callback cannot dispose an object held by nested views for {target:?}"
            );
        };
        assert!(
            error
                .to_string()
                .contains("is disposed in one callback and used in another"),
            "unexpected diagnostic: {error}"
        );
    }
}

#[test]
fn plugin_package_calls_lower_to_typed_instances_and_qualified_components() {
    std::thread::Builder::new()
        .name("plugin-package-compiler-test".to_owned())
        .stack_size(4 * 1024 * 1024)
        .spawn(plugin_package_calls_lower_to_typed_instances_and_qualified_components_inner)
        .expect("plugin compiler test thread should start")
        .join()
        .expect("plugin compiler test should not panic");
}

fn plugin_package_calls_lower_to_typed_instances_and_qualified_components_inner() {
    let entry = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/video-player/tests/demo/app/App.nx");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("VideoPlayer plugin test app should lower for each target")
            .module;

        assert_eq!(module.plugins.len(), 1);
        assert_eq!(module.plugins[0].namespace, "VideoPlayer");
        assert_eq!(module.components.len(), 2);
        assert!(module.components.iter().all(|component| {
            component.parameters.iter().any(|parameter| {
                parameter.name == "player"
                    && matches!(
                        &parameter.ty,
                        Type::Plugin { namespace, name }
                            if namespace == "VideoPlayer" && name == "VideoPlayer"
                    )
            })
        }));
        let player_surface = module
            .components
            .iter()
            .find(|component| component.name == "PlayerSurface")
            .expect("the outer player wrapper should remain reachable");
        let [
            Node::ComponentCall {
                name, arguments, ..
            },
        ] = player_surface.body.as_slice()
        else {
            panic!("the outer wrapper should pass its native object to the nested component");
        };
        assert_eq!(name, "PlayerMedia");
        assert!(matches!(
            arguments.first(),
            Some((parameter, Expr::State(value, Type::Plugin { namespace, name })))
                if parameter == "player"
                    && value == "player"
                    && namespace == "VideoPlayer"
                    && name == "VideoPlayer"
        ));
        let player_instances = module
            .states
            .iter()
            .filter(|state| {
                matches!(
                    &state.ty,
                    Type::Plugin { namespace, name }
                        if namespace == "VideoPlayer" && name == "VideoPlayer"
                )
            })
            .count();
        assert_eq!(player_instances, 2);
        assert!(
            module
                .components
                .iter()
                .flat_map(|component| &component.states)
                .any(|state| state.name == "tapped"),
            "native component callback state must survive optimization"
        );
        assert!(
            module
                .states
                .iter()
                .any(|state| state.name == "secondTapped"),
            "app callback state must survive optimization"
        );

        let mut found_components = Vec::new();
        let mut component_event_handlers = Vec::new();
        let mut component_controls = Vec::new();
        let mut component_children = Vec::new();
        let mut generic_pressable_tap_handlers = 0;
        for nodes in std::iter::once(&module.body)
            .chain(module.components.iter().map(|component| &component.body))
        {
            walk_ir(
                nodes,
                &mut |node| {
                    if let nexa_ir::Node::Pressable { actions, .. } = node
                        && !actions.is_empty()
                    {
                        generic_pressable_tap_handlers += 1;
                    }
                    if let nexa_ir::Node::NativeComponentCall {
                        namespace,
                        name,
                        arguments,
                        children,
                        event_handlers,
                        ..
                    } = node
                    {
                        found_components.push((namespace.clone(), name.clone()));
                        component_controls.push(arguments.iter().find_map(|(name, value)| {
                            if name == "controls" {
                                Some(matches!(value, Expr::Bool(true)))
                            } else {
                                None
                            }
                        }));
                        component_children.push(children.is_some());
                        component_event_handlers.push(
                            event_handlers
                                .iter()
                                .map(|handler| {
                                    (
                                        handler.property.clone(),
                                        matches!(
                                            handler.actions.as_slice(),
                                            [nexa_ir::Action::Assign {
                                                value: Expr::Bool(true),
                                                ..
                                            }]
                                        ),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }
                },
                &mut |_| {},
            );
        }
        assert_eq!(
            found_components,
            vec![
                ("VideoPlayer".to_owned(), "VideoView".to_owned()),
                ("VideoPlayer".to_owned(), "VideoView".to_owned()),
            ]
        );
        assert_eq!(component_controls, vec![Some(false), Some(true)]);
        assert_eq!(component_children, vec![true, true]);
        assert_eq!(component_event_handlers.len(), 2);
        assert!(component_event_handlers.iter().all(Vec::is_empty));
        assert_eq!(
            generic_pressable_tap_handlers, 2,
            "tap behavior belongs to reusable Pressable, not the video component"
        );

        let handled_prepares = module
            .on_appear
            .as_ref()
            .expect("native preparation should run when the view appears")
            .iter()
            .find_map(|action| match action {
                nexa_ir::Action::TryCatch {
                    body,
                    error_catches,
                    catch_body,
                } => Some((body, error_catches, catch_body)),
                _ => None,
            })
            .expect("the example explicitly recovers from native preparation errors");
        assert_eq!(
                handled_prepares
                    .0
                    .iter()
                    .filter(|action| matches!(
                        action,
                        nexa_ir::Action::Expression(Expr::Located { expression, .. })
                            if matches!(expression.as_ref(), Expr::TryAwait(call)
                            if matches!(call.as_ref(), Expr::NativeCall { name, is_throwing: true, .. } if name == "prepare")
                            )
                    ))
                    .count(),
                2
            );
        assert!(handled_prepares.2.is_none());
        assert!(matches!(
            handled_prepares.1.as_slice(),
            [
                nexa_ir::ErrorCatchArm { variant, parameters, .. },
                nexa_ir::ErrorCatchArm {
                    variant: payload_variant,
                    parameters: payload_parameters,
                    ..
                }
            ] if variant == "invalidUrl"
                && parameters.is_empty()
                && payload_variant == "decodingFailed"
                && payload_parameters == &[("message".to_owned(), "message".to_owned(), Type::String)]
        ));

        let mut instance_calls = Vec::new();
        for state in &module.states {
            if let Expr::NativeCall {
                receiver: None,
                name,
                ..
            } = &state.initial
            {
                assert_eq!(name, "VideoPlayer");
            }
        }
        walk_ir(&module.body, &mut |_| {}, &mut |expression| {
            if let Expr::NativeCall {
                receiver: Some(_),
                name,
                ..
            } = expression
            {
                instance_calls.push(name.clone());
            }
        });
        if let Some(actions) = &module.on_appear {
            walk_actions(actions, &mut |expression| {
                if let Expr::NativeCall {
                    receiver: Some(_),
                    name,
                    ..
                } = expression
                {
                    instance_calls.push(name.clone());
                }
            });
        }
        assert!(instance_calls.iter().any(|name| name == "play"));
        assert!(instance_calls.iter().any(|name| name == "pause"));
        assert!(instance_calls.iter().any(|name| name == "prepare"));

        let event_subscriptions = module
            .on_appear
            .as_ref()
            .expect("event handlers are registered when the view appears")
            .iter()
            .filter_map(|action| match action {
                nexa_ir::Action::NativeEventSubscribe {
                    receiver: Expr::State(receiver, Type::Plugin { .. }),
                    property,
                    actions,
                    ..
                } => Some((receiver.as_str(), property.as_str(), actions.as_slice())),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(event_subscriptions.len(), 2);
        assert_eq!(event_subscriptions[0].0, "player1");
        assert_eq!(event_subscriptions[1].0, "player2");
        assert!(event_subscriptions.iter().all(|(_, property, handler)| {
            *property == "onEnded"
                && matches!(
                    handler,
                    [nexa_ir::Action::Assign {
                        value: Expr::Bool(true),
                        ..
                    }]
                )
        }));
    }
}

#[test]
fn compiles_result_type_and_try_propagation() {
    let source = r#"
app TestApp {
    enum AppError {
        NotFound,
        Unauthorized
    }

    fn fetch_code() -> Result<Int32, AppError> {
        return Ok(42);
    }

    fn compute() -> Result<Int32, AppError> {
        let code: Int32 = fetch_code()?;
        return Ok(code);
    }

    state status: String = "Ready"
    state res: Result<Int32, AppError> = Ok(0)

    body {
        Button(status) {
            res = compute();
        }
    }
}
"#;
    let module = compile(source).expect("Result and try propagation should compile successfully");
    assert_eq!(module.functions.len(), 2);
    assert!(matches!(
        module.functions[0].return_type,
        Type::Result(_, _)
    ));
    assert!(matches!(
        module.functions[1].return_type,
        Type::Result(_, _)
    ));
}

#[test]
fn result_members_expose_typed_success_and_failure_values() {
    let module = compile(
        r#"
        enum PaymentError { insufficientFunds }
        fn charge(amount: Float64, balance: Float64) -> Result<Float64, PaymentError> {
            if amount > balance { return Err(PaymentError.insufficientFunds) }
            return Ok(balance - amount)
        }
        app CheckoutResult {
            let outcome: Result<Float64, PaymentError> = charge(125.0, 100.0)
            body {
                Column {
                    Text(outcome.isSuccess ? "Paid" : "Needs attention")
                    Text(outcome.value ?? 0.0)
                    Text((outcome.error ?? PaymentError.insufficientFunds) == PaymentError.insufficientFunds ? "Insufficient funds" : "No error")
                }
            }
        }
        "#,
    )
    .expect("Result values should be easy to check and read in Nexa");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("outcome.isSuccess"));
    assert!(swift.contains("outcome.value ?? Double(0.0)"), "{swift}");
    assert!(swift.contains("outcome.error ?? NexaPaymentError.insufficientFunds"));
    assert!(swift.contains("var isSuccess: Bool"));
    assert!(swift.contains("var value: Success?"));
    assert!(swift.contains("var error: Failure?"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("outcome.isSuccess"));
    assert!(kotlin.contains("outcome.getOrNull() ?: 0.0"));
    assert!(kotlin.contains("outcome.errorOrNull() ?: NexaPaymentError.insufficientFunds"));
}
