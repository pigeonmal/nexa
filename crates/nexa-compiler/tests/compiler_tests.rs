use std::{fs, path::Path};

use nexa_ir::{
    AccessibilityRole, Action, ArithmeticOp, AutofillType, CollectionMutation, ColorExpression,
    Expr, ListPlan, MemberKind, Node, NumericType, ReturnKeyType, SystemIcon, Type,
    walk::{walk_actions, walk_ir},
};

use nexa_codegen::Backend;
use nexa_compiler::{
    Target, compile, compile_file_with_warnings_for_target, compile_with_warnings_for_target,
};
use nexa_testkit::TestProject;

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
            let sha256: String = Crypto.sha256(text: "nexa")
            let sha512: String = Crypto.sha512(text: "nexa")
            let signature: String = Crypto.hmacSha256(key: "secret", message: "payload")
            let random: String = Crypto.randomBytes(count: 16)

            body {
                Text(sha256)
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
        ("sha256", &["text"][..]),
        ("sha512", &["text"][..]),
        ("hmacSha256", &["key", "message"][..]),
        ("randomBytes", &["count"][..]),
    ]) {
        assert!(matches!(
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
        ));
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
            .contains("unknown shared system icon")
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
fn bottom_sheet_lowers_shared_native_detents_and_optional_surface_configuration() {
    let default_sheet = compile(
        r#"
        app DefaultSheetHandle {
            state presented: Bool = false
            body { BottomSheet(isPresented: presented) { Text("Default handle") } }
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
            if *content_padding == 8.0
                && detents == &[nexa_ir::BottomSheetDetent::Medium, nexa_ir::BottomSheetDetent::Expanded]
    ));

    let configured_sheet = compile(
        r##"
        app ConfiguredSheet {
            state presented: Bool = false
            body {
                BottomSheet(
                    isPresented: presented,
                    contentPadding: 20,
                    detents: [Expanded],
                    initialDetent: Expanded,
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
            initial_detent: Some(nexa_ir::BottomSheetDetent::Expanded),
            show_drag_indicator: false,
            allows_dismissal: false,
            allows_background_interaction: true,
            should_dismiss_on_back_press: false,
            background_color: Some(_),
            ..
        }] if *content_padding == 20.0
            && detents == &[nexa_ir::BottomSheetDetent::Expanded]
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&configured_sheet);
    assert!(swift.contains("detents: [.large]"));
    assert!(swift.contains("contentPadding: 20"));
    assert!(swift.contains("initialDetent: .large"));
    assert!(swift.contains("showDragIndicator: false"));
    assert!(swift.contains("allowsDismissal: false"));
    assert!(swift.contains("allowsBackgroundInteraction: true"));
    assert!(swift.contains("shouldDismissOnBackPress: false"));
    assert!(swift.contains("backgroundColor: Color(red:"));
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&configured_sheet);
    assert!(kotlin.contains("SheetValue.Expanded"));
    assert!(kotlin.contains("contentPadding = 20f.dp"));
    assert!(kotlin.contains("showDragIndicator = false"));
    assert!(kotlin.contains("allowsDismissal = false"));
    assert!(kotlin.contains("allowsBackgroundInteraction = true"));
    assert!(kotlin.contains("shouldDismissOnBackPress = false"));
    assert!(kotlin.contains("backgroundColor = Color(0xFFF2F4F8)"));
}

#[test]
fn bottom_sheet_rejects_empty_duplicate_and_unconfigured_initial_detents() {
    for source in [
        r#"app Empty { state shown: Bool = false body { BottomSheet(isPresented: shown, detents: []) { Text("x") } } }"#,
        r#"app NegativePadding { state shown: Bool = false body { BottomSheet(isPresented: shown, contentPadding: -1) { Text("x") } } }"#,
        r#"app Duplicate { state shown: Bool = false body { BottomSheet(isPresented: shown, detents: [Medium, Medium]) { Text("x") } } }"#,
        r#"app Missing { state shown: Bool = false body { BottomSheet(isPresented: shown, detents: [Medium], initialDetent: Expanded) { Text("x") } } }"#,
    ] {
        assert!(
            compile(source).is_err(),
            "invalid detent configuration must fail"
        );
    }
}

#[test]
fn bottom_sheet_lowers_custom_snap_points_and_dismiss_actions_for_both_backends() {
    let module = compile(
        r##"
        app CustomSnapSheet {
            state presented: Bool = false
            state dismissed: Bool = false
            body {
                BottomSheet(
                    isPresented: presented,
                    snapPoints: [0.25, 0.5, 0.85],
                    initialSnapPoint: 0.5,
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
            snap_points,
            initial_snap_point: Some(0.5),
            show_drag_indicator: false,
            background_color: Some(_),
            scrim_color: Some(_),
            on_dismiss,
            ..
        }] if detents.is_empty()
            && snap_points == &[0.25, 0.5, 0.85]
            && matches!(on_dismiss.as_slice(), [nexa_ir::Action::Assign { name, value: nexa_ir::Expr::Bool(true) }] if name == "dismissed")
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("snapPoints: [0.25, 0.5, 0.85]"));
    assert!(swift.contains("initialSnapPoint: 0.5"));
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
    assert!(kotlin.contains("snapPoints = listOf(0.25f, 0.5f, 0.85f)"));
    assert!(kotlin.contains("initialSnapPoint = 0.5f"));
    assert!(kotlin.contains("showDragIndicator = false"));
    assert!(kotlin.contains("onDismiss = {"));
    assert!(kotlin.contains("dismissed = true"));
    assert!(kotlin.contains("backgroundColor = Color(0x00000000)"));
    assert!(kotlin.contains("scrimColor = Color(0x00000000)"));
    assert!(kotlin.contains("import kotlinx.coroutines.launch"));
}

#[test]
fn bottom_sheet_actions_drive_presentation_and_bound_snap_selection() {
    let module = compile(
        r#"
        app ProgrammaticSheet {
            state presented: Bool = false
            state selectedPoint: Float64 = 0.5
            body {
                Button("Open") { BottomSheet.present(presented) }
                Button("Close") { BottomSheet.dismiss(presented) }
                Button("Expand") { BottomSheet.snapTo(selectedPoint, 0.85) }
                BottomSheet(
                    isPresented: presented,
                    snapPoints: [0.25, 0.5, 0.85],
                    selectedSnapPoint: selectedPoint
                ) { Text("Programmatic sheet") }
            }
        }
        "#,
    )
    .expect("BottomSheet action helpers and selected snap state should compile");

    assert!(matches!(
        module.body.last(),
        Some(Node::BottomSheet {
            state,
            snap_points,
            selected_snap_point: Some(selected),
            initial_snap_point: None,
            ..
        }) if state == "presented"
            && snap_points == &[0.25, 0.5, 0.85]
            && selected == "selectedPoint"
    ));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("nexa_presented = true"));
    assert!(swift.contains("nexa_presented = false"));
    assert!(swift.contains("nexa_selectedPoint = Double(0.85)"));
    assert!(swift.contains("selectedSnapPoint: $nexa_selectedPoint"));
    assert!(swift.contains(".onChange(of: selectedSnapPoint?.wrappedValue)"));
    assert!(swift.contains("selectedSnapPoint.wrappedValue = value"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("nexa_presented = true"));
    assert!(kotlin.contains("nexa_presented = false"));
    assert!(kotlin.contains("nexa_selectedPoint = 0.85"));
    assert!(kotlin.contains("selectedSnapPoint = nexa_selectedPoint.toFloat()"));
    assert!(
        kotlin.contains("onSnapPointChange = { point -> nexa_selectedPoint = point.toDouble() }")
    );
    assert!(kotlin.contains("LaunchedEffect(isPresented, selectedSnapPoint, sheetState)"));
    assert!(kotlin.contains("if (currentValue == lastReportedSheetValue.floatValue)"));
    assert!(kotlin.contains("currentOnSnapPointChange?.invoke(currentValue)"));

    let binding_only = compile(
        r#"
        app SelectedSnapPointIsLiveState {
            state presented: Bool = false
            state selectedPoint: Float64 = 0.5
            body {
                BottomSheet(
                    isPresented: presented,
                    snapPoints: [0.25, 0.5, 0.85],
                    selectedSnapPoint: selectedPoint
                ) { Text("Bound selection") }
            }
        }
        "#,
    )
    .expect("the selected point must stay live when it has no other references");
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&binding_only);
    assert!(
        kotlin.contains("var nexa_selectedPoint by rememberSaveable { mutableDoubleStateOf(0.5) }")
    );
}

#[test]
fn bottom_sheet_header_save_and_close_actions_compile_for_both_platforms() {
    let module = compile(
        r#"
        app BottomSheetHeaderActions {
            state presented: Bool = false
            state saved: Bool = false
            body {
                BottomSheet(isPresented: presented) {
                    Column {
                        Row {
                            Text("Edit task")
                            Spacer()
                            Button("Save", icon: "save", style: Borderless) { saved = true }
                            Button("Close", icon: "close", style: Plain) {
                                BottomSheet.dismiss(presented)
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
    assert!(swift.contains("Label(\"Close\", systemImage: \"xmark\")"));
    assert!(swift.contains("nexa_saved = true"));
    assert!(swift.contains("nexa_presented = false"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin.contains("icon = Icons.Filled.Save,"));
    assert!(kotlin.contains("icon = Icons.Filled.Close,"));
    assert!(kotlin.contains("nexa_saved = true"));
    assert!(kotlin.contains("nexa_presented = false"));
}

#[test]
fn nested_bottom_sheets_keep_the_parent_presented_on_both_platforms() {
    let module = compile(
        r#"
        app NestedBottomSheets {
            state parentPresented: Bool = false
            state childPresented: Bool = false
            body {
                BottomSheet(isPresented: parentPresented) {
                    Column {
                        Button("Open child") { BottomSheet.present(childPresented) }
                        BottomSheet(isPresented: childPresented) {
                            Button("Close child") { BottomSheet.dismiss(childPresented) }
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
        .find("isPresented: $nexa_parentPresented")
        .expect("emit the parent native SwiftUI sheet");
    let swift_child = swift
        .find("isPresented: $nexa_childPresented")
        .expect("emit the nested native SwiftUI sheet");
    assert!(swift_parent < swift_child);
    assert!(swift.contains(".sheet(isPresented: $isPresented, onDismiss: {"));

    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
    let kotlin_parent = kotlin
        .find("isPresented = nexa_parentPresented,")
        .expect("emit the parent Material 3 sheet");
    let kotlin_child = kotlin
        .find("isPresented = nexa_childPresented,")
        .expect("emit the nested Material 3 sheet");
    assert!(kotlin_parent < kotlin_child);
    assert!(kotlin.contains("ModalBottomSheet("));
}

#[test]
fn bottom_sheet_dynamic_surface_and_dismiss_callback_count_as_state_uses() {
    let compilation = compile_with_warnings_for_target(
        r##"
        app BottomSheetStateUses {
            state presented: Bool = false
            state surface: String = "#00000000"
            state scrim: String = "#00000000"
            state dismissed: Bool = false
            body {
                BottomSheet(isPresented: presented, backgroundColor: surface, scrimColor: scrim) {
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
fn bottom_sheet_rejects_invalid_custom_snap_point_combinations() {
    for source in [
        r#"app Empty { state shown: Bool = false body { BottomSheet(isPresented: shown, snapPoints: []) { Text("x") } } }"#,
        r#"app Duplicate { state shown: Bool = false body { BottomSheet(isPresented: shown, snapPoints: [0.5, 0.5]) { Text("x") } } }"#,
        r#"app Unsorted { state shown: Bool = false body { BottomSheet(isPresented: shown, snapPoints: [0.75, 0.5]) { Text("x") } } }"#,
        r#"app OutOfRange { state shown: Bool = false body { BottomSheet(isPresented: shown, snapPoints: [1.2]) { Text("x") } } }"#,
        r#"app MissingInitial { state shown: Bool = false body { BottomSheet(isPresented: shown, snapPoints: [0.25, 0.75], initialSnapPoint: 0.5) { Text("x") } } }"#,
        r#"app Combined { state shown: Bool = false body { BottomSheet(isPresented: shown, detents: [Medium], snapPoints: [0.5]) { Text("x") } } }"#,
        r#"app RequiresPoints { state shown: Bool = false body { BottomSheet(isPresented: shown, initialSnapPoint: 0.5) { Text("x") } } }"#,
        r#"app MixedInitial { state shown: Bool = false body { BottomSheet(isPresented: shown, snapPoints: [0.5], initialDetent: Medium) { Text("x") } } }"#,
        r#"app SelectedWithoutPoints { state shown: Bool = false state point: Float64 = 0.5 body { BottomSheet(isPresented: shown, selectedSnapPoint: point) { Text("x") } } }"#,
        r#"app SelectedAndInitial { state shown: Bool = false state point: Float64 = 0.5 body { BottomSheet(isPresented: shown, snapPoints: [0.5], initialSnapPoint: 0.5, selectedSnapPoint: point) { Text("x") } } }"#,
        r#"app SelectedNotMutable { state shown: Bool = false let point: Float64 = 0.5 body { BottomSheet(isPresented: shown, snapPoints: [0.5], selectedSnapPoint: point) { Text("x") } } }"#,
        r#"app SnapToUnknownState { body { Button("Snap") { BottomSheet.snapTo(missing, 0.5) } } }"#,
        r#"app PresentWrongType { state point: Float64 = 0.5 body { Button("Open") { BottomSheet.present(point) } } }"#,
    ] {
        assert!(
            compile(source).is_err(),
            "invalid custom snap-point configuration must fail: {source}"
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
                            fields: ["kind": "receipt"]
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
            && arguments.iter().map(|(argument, _)| argument.as_str()).collect::<Vec<_>>() == ["url", "file", "fields"]
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
            if state == "volume" && *min == 0.0 && *max == 1.0 && *step == 0.1
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
        Node::Slider { min, max, .. } if *min == -1.0 && *max == 1.0
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
fn progress_controls_lower_float64_expressions() {
    let module = compile(
        r##"
        app ProgressControls {
            state progress: Float64 = 0.5
            state tint: String = "#2563EB"
            state wave: Bool = true
            body {
                ProgressBar(progress: progress)
                ProgressRing(progress: progress + 0.1, tint: tint, trackColor: "#DBEAFE", size: 48, thickness: 5, android_wave: wave)
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
fn progress_ring_rejects_negative_dimensions() {
    for dimension in ["size: -1", "thickness: -1"] {
        let source = format!(
            "app InvalidProgressRing {{ body {{ ProgressRing(progress: 0.5, {dimension}) }} }}"
        );
        let error = compile(&source).expect_err("negative ring dimensions must be rejected");
        assert!(error.to_string().contains("non-negative"), "{error}");
    }

    let error = compile("app InvalidProgressRingWave { body { ProgressRing(android_wave: 1) } }")
        .expect_err("android_wave must be a Bool");
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
                    android_wave: wave
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
        r#"app InvalidProgressWave { body { ProgressBar(progress: 0.5, android_wave: "yes") } }"#,
    )
    .expect_err("android_wave must be a typed Boolean expression");
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
                        nexa_ir::Action::Expression(Expr::TryAwait(call))
                            if matches!(call.as_ref(), Expr::NativeCall { name, is_throwing: true, .. } if name == "prepare")
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
