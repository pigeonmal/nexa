use std::fs;

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::{
    KeyboardDismissMode, Node,
    adaptive_layout::{LayoutEdge, ScrollHeaderBehavior, ScrollOwnerResolution},
};
use nexa_testkit::TestProject;

fn compile(
    source: &str,
    target: Target,
) -> Result<nexa_compiler::Compilation, nexa_compiler::CompileError> {
    let project = TestProject::new("nexa-adaptive-layout-lowering");
    let entry = project.join("App.nx");
    fs::write(&entry, source).expect("write adaptive-layout probe");
    compile_file_with_warnings_for_target(&entry, target)
}

#[test]
fn unified_padding_margin_and_layout_gap_lower_to_typed_insets() {
    let source = r#"app Insets {
        body {
            Column(gap: 8, padding: 16, margin: Insets(bottom: 12)) {
                Text("Profile")
            }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("unified insets should lower")
            .module;
        let Node::ViewInsets {
            padding,
            margin,
            child,
        } = &module.body[0]
        else {
            panic!("expected typed view insets IR");
        };
        let padding = padding.as_ref().expect("uniform padding");
        assert!(padding.top.is_some() && padding.bottom.is_some());
        assert!(padding.leading.is_some() && padding.trailing.is_some());
        let margin = margin.as_ref().expect("bottom margin");
        assert!(margin.bottom.is_some());
        assert!(margin.top.is_none() && margin.leading.is_none() && margin.trailing.is_none());
        assert!(matches!(child.as_ref(), Node::Layout { gap: 8.0, .. }));

        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert!(
            swift.contains("NexaViewInsetsPrimitive(padding: EdgeInsets("),
            "{swift}"
        );
        assert!(
            swift.contains(
                "margin: EdgeInsets(top: 0, leading: 0, bottom: max(0, CGFloat(Double(12))), trailing: 0)"
            ),
            "{swift}"
        );
        assert!(
            kotlin.contains("NexaViewInsetsPrimitive(paddingTop = "),
            "{kotlin}"
        );
        assert!(kotlin.contains("marginBottom = "), "{kotlin}");
    }
}

#[test]
fn form_padding_and_margin_keep_distinct_inset_contracts_on_both_backends() {
    let source = r#"app FormInsets {
        body {
            Form(padding: Insets(top: SystemInsets.top)) {
                Section(title: "Padded") { Text("Inside the scroll content") }
            }
            Form(margin: Insets(top: SystemInsets.top)) {
                Section(title: "Margined") { Text("Outside the scroll content") }
            }
        }
    }"#;

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("Form padding and margin should both lower")
            .module;
        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);

        assert_eq!(
            swift
                .matches("nexaWindowMetrics.safeInsets.logical(layoutDirection).top")
                .count(),
            2
        );
        assert_eq!(
            kotlin.matches("systemInset(NexaSafeAreaEdge.Top").count(),
            2
        );

        assert!(
            swift.contains("contentPadding: EdgeInsets(top: max(0, CGFloat(Double(nexaWindowMetrics.safeInsets.logical(layoutDirection).top)))"),
            "Form padding must stay in the scroll content: {swift}"
        );
        assert!(
            swift.contains("margin: EdgeInsets(top: max(0, CGFloat(Double(nexaWindowMetrics.safeInsets.logical(layoutDirection).top)))"),
            "Form margin must stay outside the scroll content: {swift}"
        );
        assert!(
            kotlin.contains("contentPadding = androidx.compose.foundation.layout.PaddingValues(start = androidx.compose.ui.unit.Dp(0f), top = androidx.compose.ui.unit.Dp((LocalNexaWindowMetrics.current.systemInset(NexaSafeAreaEdge.Top"),
            "Android Form padding must stay in the scroll content: {kotlin}"
        );
        assert!(
            kotlin.contains("modifier = Modifier.padding(androidx.compose.foundation.layout.PaddingValues(start = androidx.compose.ui.unit.Dp(0f), top = androidx.compose.ui.unit.Dp((LocalNexaWindowMetrics.current.systemInset(NexaSafeAreaEdge.Top"),
            "Android Form margin must stay outside the scroll content: {kotlin}"
        );
    }
}

#[test]
fn percentages_and_screen_fraction_functions_use_parent_and_window_metrics() {
    let source = r#"app ResponsiveHero {
        body {
            Column(width: 80%, minWidth: 20%, maxWidth: 90%, height: screenHeightFraction(0.5), minHeight: screenHeightFraction(0.25)) {
                Text("Artist artwork")
            }
        }
    }"#;
    let swift_module = compile(source, Target::Swift)
        .expect("typed responsive dimensions should lower")
        .module;
    let swift = SwiftBackend.generate(&swift_module);
    assert!(
        swift.contains(
            "NexaParentRelativeDimensionModifier(width: 0.8, minWidth: 0.2, maxWidth: 0.9)"
        ),
        "{swift}"
    );
    assert!(
        swift.contains("nexaWindowMetrics.viewportSize.height * 0.5"),
        "{swift}"
    );
    assert!(
        swift.contains("nexaWindowMetrics.viewportSize.height * 0.25"),
        "{swift}"
    );
    assert!(
        swift.contains("proposal.width.flatMap { $0.isFinite ? $0 : nil }"),
        "{swift}"
    );
    assert!(
        swift.contains("proposal.height.flatMap { $0.isFinite ? $0 : nil }"),
        "{swift}"
    );

    let kotlin_module = compile(source, Target::Kotlin)
        .expect("typed responsive dimensions should lower")
        .module;
    let kotlin = KotlinBackend.generate(&kotlin_module);
    assert!(
        kotlin.contains(
            "nexaParentRelativeDimension(width = 0.8f, minWidth = 0.2f, maxWidth = 0.9f)"
        ),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("LocalNexaWindowMetrics.current.viewportHeightDp * 0.5f"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("LocalNexaWindowMetrics.current.viewportHeightDp * 0.25f"),
        "{kotlin}"
    );
    assert!(kotlin.contains("constraints.hasBoundedWidth"), "{kotlin}");
    assert!(kotlin.contains("constraints.hasBoundedHeight"), "{kotlin}");
}

#[test]
fn dimension_aliases_and_inset_shorthands_lower_to_shared_typed_values() {
    let source = r#"app UnifiedGeometry {
        body {
            Column(width: Fill, maxWidth: 100%, height: 50%, minHeight: 25%, maxHeight: Fill) {
                Text("Geometry", padding: Insets(horizontal: 20), margin: 4)
                Spacer(height: 24)
            }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("Fill aliases, parent percentages, and inset shorthands should lower")
            .module;
        let Node::Layout {
            style, children, ..
        } = &module.body[0]
        else {
            panic!("expected typed layout node");
        };
        assert_eq!(style.width, Some(nexa_ir::Dimension::Fill));
        assert_eq!(
            style.max_width,
            Some(nexa_ir::Dimension::ParentFraction(1.0))
        );
        assert_eq!(style.height, Some(nexa_ir::Dimension::ParentFraction(0.5)));
        assert_eq!(
            style.min_height,
            Some(nexa_ir::Dimension::ParentFraction(0.25))
        );
        assert_eq!(
            style.max_height,
            Some(nexa_ir::Dimension::ParentFraction(1.0))
        );

        let Node::ViewInsets {
            padding, margin, ..
        } = &children[0]
        else {
            panic!("expected edge padding and outer margin");
        };
        let padding = padding
            .as_ref()
            .expect("horizontal shorthand should add padding");
        assert!(padding.top.is_none() && padding.bottom.is_none());
        assert!(padding.leading.is_some() && padding.trailing.is_some());
        let margin = margin
            .as_ref()
            .expect("uniform margin should add all edges");
        assert!(margin.top.is_some() && margin.bottom.is_some());
        assert!(margin.leading.is_some() && margin.trailing.is_some());

        assert!(matches!(
            &children[1],
            Node::Spacer {
                width: None,
                height: Some(nexa_ir::Dimension::Fixed(24.0))
            }
        ));
    }
}

#[test]
fn system_insets_are_reactive_values_in_explicit_padding_on_both_targets() {
    let source = r#"app ProtectedSettings {
        body {
            Form(
                padding: Insets(top: 16 + SystemInsets.top, horizontal: 20),
                margin: Insets(bottom: 8)
            ) {
                Section(title: "Account") { Text("elias@example.com") }
            }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("SystemInsets should lower as a typed expression")
            .module;
        let Node::ViewInsets {
            padding: Some(padding),
            margin: Some(margin),
            ..
        } = &module.body[0]
        else {
            panic!("expected explicit padding wrapper around Form");
        };
        assert!(matches!(
            padding.top.as_deref(),
            Some(nexa_ir::Expr::Add(..))
        ));
        assert!(margin.bottom.is_some());
        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert!(
            swift.contains("nexaWindowMetrics.safeInsets.logical(layoutDirection).top"),
            "{swift}"
        );
        assert!(
            kotlin.contains("LocalNexaWindowMetrics.current.systemInset(NexaSafeAreaEdge.Top"),
            "{kotlin}"
        );
        assert!(
            swift.contains("NexaFormPrimitive(background: nil, hideHorizontalScrollbar: false, hideVerticalScrollbar: false, contentPadding: EdgeInsets(top:"),
            "{swift}"
        );
        assert!(swift.contains("isFirstSection: true"), "{swift}");
        assert!(
            kotlin.contains("NexaFormPrimitive(background = null, contentPadding = androidx.compose.foundation.layout.PaddingValues("),
            "{kotlin}"
        );
        assert!(swift.contains(
            "margin: EdgeInsets(top: 0, leading: 0, bottom: max(0, CGFloat(Double(8))), trailing: 0)"
        ));
        assert!(kotlin.contains(
            "modifier = Modifier.padding(androidx.compose.foundation.layout.PaddingValues("
        ));
        assert_eq!(swift.matches("NexaViewInsetsPrimitive(padding:").count(), 0);
        assert_eq!(
            kotlin.matches("NexaViewInsetsPrimitive(paddingTop").count(),
            0
        );
    }
}

#[test]
fn invalid_dimension_functions_and_duplicate_widths_are_rejected() {
    for source in [
        r#"app InvalidWidth { body { Column(width: 200, width: screenWidthFraction(0.9)) { Text("Invalid") } } }"#,
        r#"app InvalidWidth { body { Column(width: screenHeightFraction(0.9)) { Text("Invalid") } } }"#,
        r#"app InvalidWidth { body { Column(width: screenWidthFraction(1.1)) { Text("Invalid") } } }"#,
        r#"app InvalidWidth { body { Column(width: 120%) { Text("Invalid") } } }"#,
    ] {
        let error = match compile(source, Target::Swift) {
            Ok(_) => panic!("invalid dimension values should fail"),
            Err(error) => error,
        };
        assert!(
            error.message.contains("width") || error.message.contains("screenWidthFraction"),
            "{error}"
        );
    }
}

#[test]
fn removed_legacy_layout_parameters_are_rejected() {
    for source in [
        r#"app RemovedFillWidth { body { Column(fillMaxWidth: true) { Text("X") } } }"#,
        r#"app RemovedFillHeight { body { Column(fillMaxHeight: true) { Text("X") } } }"#,
        r#"app RemovedWidthFraction { body { Column(screenWidthFraction: 0.5) { Text("X") } } }"#,
        r#"app RemovedHeightFraction { body { Column(screenHeightFraction: 0.5) { Text("X") } } }"#,
        r#"app RemovedOuterSpacing { body { Column(spacing: 8) { Text("X") } } }"#,
    ] {
        let error = match compile(source, Target::Swift) {
            Ok(_) => panic!("removed legacy layout parameters should be rejected"),
            Err(error) => error,
        };
        assert!(error.message.contains("unknown option"), "{error}");
    }
}

#[test]
fn bottom_corner_radius_uses_matching_native_shapes_for_both_targets() {
    let source = r#"app ArtistArtwork {
        body {
            Stack(cornerRadiusBottom: 46.8) {
                Image(asset: "artist", description: "Artist artwork")
            }
        }
    }"#;
    let compilation = compile(source, Target::Swift)
        .expect("bottom corner radius should lower to the shared view style");
    let swift = SwiftBackend.generate(&compilation.module);
    assert!(swift.contains("UnevenRoundedRectangle(cornerRadii: RectangleCornerRadii(topLeading: 0, bottomLeading: 46.8, bottomTrailing: 46.8, topTrailing: 0))"), "{swift}");

    let compilation = compile(source, Target::Kotlin)
        .expect("bottom corner radius should lower to the shared view style");
    let kotlin = KotlinBackend.generate(&compilation.module);
    assert!(kotlin.contains("RoundedCornerShape(topStart = 0.dp, topEnd = 0.dp, bottomEnd = 46.8.dp, bottomStart = 46.8.dp)"), "{kotlin}");
}

#[test]
fn component_margin_stays_outside_while_parent_gap_separates_children() {
    let source = r#"app NestedSpacing {
        body {
            Column(gap: 6, margin: Insets(bottom: 5)) {
                Row(gap: 3, margin: Insets(top: 2)) { Text("Details") }
            }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("nested component margin should lower")
            .module;
        let Node::ViewInsets { child, .. } = &module.body[0] else {
            panic!("outer margin wrapper expected")
        };
        assert!(matches!(child.as_ref(), Node::Layout { gap: 6.0, .. }));
        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert_eq!(swift.matches("NexaViewInsetsPrimitive(padding:").count(), 2);
        assert_eq!(
            kotlin.matches("NexaViewInsetsPrimitive(paddingTop").count(),
            2
        );
        assert!(swift.contains(
            "margin: EdgeInsets(top: 0, leading: 0, bottom: max(0, CGFloat(Double(5))), trailing: 0)"
        ));
        assert!(kotlin.contains("marginTop = "));
    }
}

#[test]
fn safe_area_inset_composes_with_explicit_form_padding_without_consuming_system_insets() {
    let source = r#"app ProtectedSettings {
        body {
            Form(padding: Insets(top: SystemInsets.top, leading: SystemInsets.leading)) {
                Section(title: "Account") { Text("elias@example.com") }
            }
            .safeAreaInset(edge: Bottom) { Button("Save") { } }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("safe-area inset should compose with explicit padding")
            .module;
        let Node::SafeAreaInset { plan, children, .. } = &module.body[0] else {
            panic!("expected safe-area inset around Form");
        };
        assert!(
            matches!(plan.scroll_owner, ScrollOwnerResolution::Unique(owner)
            if owner.kind == nexa_ir::adaptive_layout::ScrollOwnerKind::Form)
        );
        assert!(
            matches!(children.first(), Some(Node::ViewInsets { child, .. }) if matches!(child.as_ref(), Node::Form { .. }))
        );

        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert!(
            swift.contains("NexaSafeAreaInsetPrimitive(edge: .bottom, accommodatesEndpoint: true)")
        );
        assert!(kotlin.contains(
            "NexaSafeAreaInsetPrimitive(edge = NexaSafeAreaEdge.Bottom, accommodatesEndpoint = true"
        ));
        assert!(swift.contains("nexaWindowMetrics.safeInsets.logical(layoutDirection).top"));
        assert!(
            kotlin.contains("LocalNexaWindowMetrics.current.systemInset(NexaSafeAreaEdge.Leading")
        );
    }
}

#[test]
fn safe_area_inset_places_content_without_a_scroll_owner_and_records_unique_owners() {
    let no_owner = r#"app Attachment {
        body {
            StatusBar(style: Light, hidden: false)
            Stack {
                Text("Viewer")
                Text("Cover artwork")
            }
                .safeAreaInset(edge: Bottom) { Button("Close") { } }
        }
    }"#;
    let module = compile(no_owner, Target::Kotlin)
        .expect("edge attachment works without scrolling")
        .module;
    let Node::SafeAreaInset { plan, content, .. } = &module.body[0] else {
        panic!("expected typed safe-area inset IR");
    };
    assert_eq!(plan.edge, LayoutEdge::Bottom);
    assert!(matches!(plan.scroll_owner, ScrollOwnerResolution::None));
    assert!(matches!(content.first(), Some(Node::Button { .. })));
    let kotlin = KotlinBackend.generate(&module);
    let swift = SwiftBackend.generate(&module);
    assert!(swift.contains(".modifier(NexaStatusBarStylePrimitive(colorScheme: .dark))"));
    assert!(kotlin.contains("nexaController.isAppearanceLightStatusBars = false"));
    assert!(kotlin.contains("accommodatesEndpoint = false"));
    assert!(swift.contains("accommodatesEndpoint: false"));
    assert!(swift.contains(".overlay(alignment: edge.nexaAlignment)"));
    assert!(swift.contains("struct NexaStackLayer<Content: View>: View"));
    assert!(swift.contains(".environment(\\.nexaSafeAreaRootFillsWidth, true)"));
    assert!(swift.contains(
        ".frame(maxWidth: fillsSafeAreaRootWidth ? .infinity : nil, alignment: alignment)"
    ));
    assert!(
        swift.contains("GeometryReader { proxy in")
            && swift.contains(
                ".frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)"
            )
    );
    assert!(swift.contains(".ignoresSafeArea(.container, edges: .all)"));
    assert!(kotlin.contains("SubcomposeLayout"));
    assert!(!swift.contains("NexaSafeAreaEdgePadding"));
    assert!(swift.contains("content.safeAreaInset(edge: .bottom, spacing: 0)"));
    assert!(kotlin.contains("val safe = safeInsets.get(edge)"));

    let with_owner = r#"app Feed {
        body {
            Column {
                FastList(count: 24) { index in Text("Article \(index)") }
            }
            .safeAreaInset(edge: Bottom) { Button("Compose") { } }
        }
    }"#;
    let module = compile(with_owner, Target::Swift)
        .expect("a unique compatible scroll owner should be inferred")
        .module;
    let Node::SafeAreaInset { plan, .. } = &module.body[0] else {
        panic!("expected typed safe-area inset IR");
    };
    assert!(matches!(
        plan.scroll_owner,
        ScrollOwnerResolution::Unique(_)
    ));
    let swift = SwiftBackend.generate(&module);
    let kotlin = KotlinBackend.generate(&module);
    assert!(
        swift.contains("NexaSafeAreaInsetPrimitive(edge: .bottom, accommodatesEndpoint: true)")
    );
    assert!(kotlin.contains("accommodatesEndpoint = true"));
    assert!(kotlin.contains("consumed = parent.consumed.offset(edge, safe)"));
    assert!(!kotlin.contains("contentPadding = nexaSafeAreaContentPadding"));
    assert!(kotlin.contains(
        "modifier = nexaSafeAreaScrollViewportModifier(\"Vertical\").then(Modifier.fillMaxWidth())"
    ));
    assert!(kotlin.contains("if (accommodatesEndpoint) occupied else 0.dp"));
    assert!(kotlin.contains("internal fun nexaSafeAreaScrollViewportModifier("));
    assert!(kotlin.contains("bottom = if (vertical) endpoint.bottom else 0.dp"));
    assert!(kotlin.contains("val safeContextForAttachment = parent.copy("));
    assert!(kotlin.contains("val occupied = safe + attachmentSize"));
}

#[test]
fn safe_area_inset_around_navigation_stack_reserves_route_local_lists() {
    // Mazica keeps a custom tab bar alongside its root navigation container.
    // A corresponding Nexa composition wraps the navigation root, while the
    // actual scrolling content belongs to a destination screen.
    let source = r#"app MusicShell {
        screen Home {
            FastList(count: 8) { index in Text("Track \\(index + 1)") }
        }
        screen Library {
            FastList(count: 5) { index in Text("Saved \\(index + 1)") }
        }
        body {
            NavigationStack(root: Home)
                .safeAreaInset(edge: Bottom) { Button("Library") { } }
        }
    }"#;
    let module = compile(source, Target::Kotlin)
        .expect("navigation root with an attached tab bar should compile")
        .module;
    let Node::SafeAreaInset { plan, children, .. } = &module.body[0] else {
        panic!("expected root safe-area inset");
    };
    assert!(matches!(
        plan.scroll_owner,
        ScrollOwnerResolution::RouteHost
    ));
    assert!(matches!(
        children.first(),
        Some(Node::NavigationStack { .. })
    ));
    assert_eq!(module.screens.len(), 2);
    assert!(
        module
            .screens
            .iter()
            .all(|screen| matches!(screen.body.first(), Some(Node::FastList { .. })))
    );

    // A route host is one active destination at a time, so endpoint reservation
    // flows through the navigation composition to each route-local list.
    let kotlin = KotlinBackend.generate(&module);
    let swift = SwiftBackend.generate(&module);
    assert!(kotlin.contains("accommodatesEndpoint = true"));
    assert!(swift.contains("accommodatesEndpoint: true"));
    assert!(kotlin.contains("modifier = nexaSafeAreaScrollViewportModifier(\"Vertical\")"));
}

#[test]
fn ambiguous_scroll_owners_preserve_placement_without_arbitrary_endpoint_selection() {
    let source = r#"app TwoFeeds {
        body {
            Column {
                FastList(count: 10) { index in Text("First \(index)") }
                FastList(count: 10) { index in Text("Second \(index)") }
            }
            .safeAreaInset(edge: Bottom) { Button("Compose") { } }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("ambiguous placement remains usable without choosing a scroll owner")
            .module;
        let Node::SafeAreaInset { plan, .. } = &module.body[0] else {
            panic!("expected typed safe-area inset IR");
        };
        assert!(matches!(
            plan.scroll_owner,
            ScrollOwnerResolution::Ambiguous(_)
        ));
        assert!(
            SwiftBackend
                .generate(&module)
                .contains("accommodatesEndpoint: false")
        );
        assert!(
            KotlinBackend
                .generate(&module)
                .contains("accommodatesEndpoint = false")
        );
    }
}

#[test]
fn page_snapping_fast_list_uses_a_reserved_scroll_viewport() {
    let source = r#"app SnappingFeed {
        body {
            FastList(count: 6, pageSnap: true) { index in
                Stack { Text("Article \(index)") }
            }
            .safeAreaInset(edge: Bottom) { Button("Compose") { } }
        }
    }"#;
    let module = compile(source, Target::Kotlin)
        .expect("page-snapping list should receive a typed scroll owner")
        .module;
    let kotlin = KotlinBackend.generate(&module);
    assert!(kotlin.contains("VerticalPager("));
    assert!(
        kotlin
            .contains("modifier = nexaSafeAreaScrollViewportModifier(\"Vertical\").fillMaxSize()")
    );
    assert!(!kotlin.contains("contentPadding = nexaSafeAreaContentPadding"));
    assert!(kotlin.contains("NexaSafeAreaProvider {"));
    assert!(kotlin.contains("NexaSafeAreaInsetPrimitive("));
}

#[test]
fn fast_list_content_is_edge_to_edge_without_explicit_safe_area_padding() {
    let source = r#"app Feed {
        body {
            FastList(count: 4) { index in Text("Article \(index + 1)") }
        }
    }"#;
    let module = compile(source, Target::Kotlin)
        .expect("a standard fast list should compile")
        .module;
    let kotlin = KotlinBackend.generate(&module);
    assert!(!kotlin.contains("nexaSafeAreaContentPadding"));
    assert!(kotlin.contains("NexaSafeAreaProvider {"));
    assert!(kotlin.contains("bottom = if (vertical) endpoint.bottom else 0.dp"));
}

#[test]
fn invalid_safe_area_edges_and_spacing_are_rejected_during_semantic_lowering() {
    let error = match compile(
        r#"app BadEdge { body { Text("X").safeAreaInset(edge: Center) { Text("Y") } } }"#,
        Target::Swift,
    ) {
        Ok(_) => panic!("invalid safe-area edges should be rejected"),
        Err(error) => error,
    };
    assert!(error.message.contains("safe-area edge"), "{error}");

    let spacing_error = match compile(
        r#"app RemovedInsetSpacing { body { Text("X").safeAreaInset(edge: Bottom, spacing: 8) { Text("Y") } } }"#,
        Target::Swift,
    ) {
        Ok(_) => panic!("safe-area inset spacing should no longer be accepted"),
        Err(error) => error,
    };
    assert!(spacing_error.message.contains("spacing"), "{spacing_error}");
}

#[test]
fn removed_safe_area_modifier_is_not_accepted() {
    let error = match compile(
        r#"app RemovedSafeAreaApi { body { Text("X").safeAreaPadding(edges: [Top]) } }"#,
        Target::Swift,
    ) {
        Ok(_) => panic!("removed safe-area modifiers should be rejected"),
        Err(error) => error,
    };
    assert!(!error.message.contains("SystemInsets"), "{error}");
}

#[test]
fn keyboard_aware_defaults_to_stay_and_avoid_is_explicit_on_both_targets() {
    let default_module = compile(
        r#"app KeyboardDefault { body { KeyboardAware { Text("Window anchored") } } }"#,
        Target::Swift,
    )
    .expect("default KeyboardAware should compile")
    .module;
    let Node::KeyboardAware {
        dismiss, behavior, ..
    } = &default_module.body[0]
    else {
        panic!("expected KeyboardAware IR");
    };
    assert_eq!(*dismiss, KeyboardDismissMode::Interactive);
    assert_eq!(*behavior, nexa_ir::adaptive_layout::KeyboardBehavior::Stay);

    let source = r#"app KeyboardAvoid { body { KeyboardAware(behavior: Avoid) { Form { Text("Editor") } } } }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("explicit IME avoidance should compile")
            .module;
        let Node::KeyboardAware { behavior, .. } = &module.body[0] else {
            panic!("expected KeyboardAware IR");
        };
        assert_eq!(*behavior, nexa_ir::adaptive_layout::KeyboardBehavior::Avoid);
        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert!(
            swift.contains(
                "NexaKeyboardAwarePrimitive(behavior: .avoid, hasNativeScrollOwner: true"
            )
        );
        assert!(kotlin.contains(
            "NexaKeyboardAwareOwnerPrimitive(interactiveDismiss = true, avoidIme = true)"
        ));
    }
}

#[test]
fn invalid_keyboard_aware_behavior_is_a_semantic_error() {
    let error = match compile(
        r#"app InvalidKeyboard { body { KeyboardAware(behavior: Move) { Text("X") } } }"#,
        Target::Swift,
    ) {
        Ok(_) => panic!("unknown keyboard behavior should fail compilation"),
        Err(error) => error,
    };
    assert!(error.message.contains("must be `Stay` or `Avoid`"));
}

#[test]
fn scroll_header_lowers_to_one_reusable_native_owner_capability() {
    let source = r#"app HeaderScreen {
        body {
            Form { Text("Profile settings") }
                .scrollHeader(behavior: Collapse, collapsedHeight: 64) {
                    Text("Settings", fontSize: 28, fontWeight: Bold)
                }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("scroll header should lower on either backend")
            .module;
        let Node::ScrollHeader {
            plan,
            header,
            children,
        } = &module.body[0]
        else {
            panic!("expected typed ScrollHeader IR");
        };
        assert_eq!(plan.behavior, ScrollHeaderBehavior::Collapse);
        assert_eq!(plan.collapsed_height, Some(64.0));
        assert_eq!(header.len(), 1);
        assert!(matches!(children.first(), Some(Node::Form { .. })));

        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert!(swift.contains(
            "NexaAdaptiveScrollHeaderPrimitive(behavior: .collapse, collapsedHeight: 64"
        ));
        assert!(swift.contains("NexaScrollOffsetProbe"));
        assert!(kotlin.contains(
            "NexaAdaptiveScrollHeaderPrimitive(behavior = \"Collapse\", collapsedHeight = 64.dp"
        ));
        assert!(kotlin.contains("NestedScrollConnection"));
    }
}

#[test]
fn scroll_header_defaults_to_pinned_and_rejects_missing_or_ambiguous_owners() {
    let default = compile(
        r#"app PinnedHeader { body { FastList(count: 4) { i in Text("Row (i)") }.scrollHeader { Text("Feed") } } }"#,
        Target::Swift,
    )
    .expect("a unique owner and omitted behavior should default to pinned")
    .module;
    assert!(matches!(
        &default.body[0],
        Node::ScrollHeader { plan, .. } if plan.behavior == ScrollHeaderBehavior::Pinned
    ));

    for (source, message) in [
        (
            r#"app NoOwner { body { Text("Viewer").scrollHeader { Text("Title") } } }"#,
            "requires a vertical scroll owner",
        ),
        (
            r#"app MultipleOwners { body { Column { Form { Text("One") } Form { Text("Two") } }.scrollHeader { Text("Title") } } }"#,
            "ambiguous with multiple vertical scroll owners",
        ),
        (
            r#"app InvalidHeader { body { Form { Text("One") }.scrollHeader(behavior: Float) { Text("Title") } } }"#,
            "must be `Pinned`, `Collapse`, or `HideOnScroll`",
        ),
    ] {
        let error = match compile(source, Target::Swift) {
            Ok(_) => panic!("invalid scroll-header relationship should fail compilation"),
            Err(error) => error,
        };
        assert!(
            error.message.contains(message),
            "unexpected error: {}",
            error.message
        );
    }
}

#[test]
fn scroll_view_participates_in_insets_headers_and_keyboard_ownership() {
    let bottom_inset = compile(
        r#"app ScrollInsetOwner {
            body {
                ScrollView(axis: Vertical, height: Fill) { Text("Article") }
                    .safeAreaInset(edge: Bottom) { Button("Continue") { } }
            }
        }"#,
        Target::Swift,
    )
    .expect("ScrollView should be a safe-area inset owner")
    .module;
    let Node::SafeAreaInset { plan, children, .. } = &bottom_inset.body[0] else {
        panic!("expected SafeAreaInset");
    };
    assert!(matches!(
        plan.scroll_owner,
        nexa_ir::adaptive_layout::ScrollOwnerResolution::Unique(owner)
            if owner.kind == nexa_ir::adaptive_layout::ScrollOwnerKind::ScrollView
                && owner.axes == nexa_ir::adaptive_layout::ScrollAxis::Vertical
    ));
    assert!(matches!(children.first(), Some(Node::ScrollView { .. })));

    let horizontal_inset = compile(
        r#"app HorizontalScrollInsetOwner {
            body {
                ScrollView(axis: Horizontal, width: Fill) { Row { Text("A") Text("B") } }
                    .safeAreaInset(edge: Trailing) { Text("Attached") }
            }
        }"#,
        Target::Swift,
    )
    .expect("horizontal ScrollView should own horizontal insets")
    .module;
    assert!(matches!(
        &horizontal_inset.body[0],
        Node::SafeAreaInset { plan, .. }
            if matches!(plan.scroll_owner, nexa_ir::adaptive_layout::ScrollOwnerResolution::Unique(owner)
                if owner.axes == nexa_ir::adaptive_layout::ScrollAxis::Horizontal)
    ));

    let header = compile(
        r#"app ScrollViewHeader {
            body {
                ScrollView(height: Fill) { Column { Text("Article") } }
                    .scrollHeader { Text("Reader") }
            }
        }"#,
        Target::Swift,
    )
    .expect("vertical ScrollView should support scrollHeader")
    .module;
    assert!(matches!(
        &header.body[0],
        Node::ScrollHeader { children, .. } if matches!(children.first(), Some(Node::ScrollView { .. }))
    ));
    let horizontal_header = compile(
        r#"app InvalidHorizontalHeader {
            body {
                ScrollView(axis: Horizontal, width: Fill) { Row { Text("A") } }
                    .scrollHeader { Text("Reader") }
            }
        }"#,
        Target::Swift,
    )
    .err()
    .expect("a horizontal-only owner cannot drive vertical header behavior");
    assert!(
        horizontal_header
            .to_string()
            .contains("requires a vertical scroll owner")
    );

    let keyboard = compile(
        r#"app KeyboardScrollView {
            body {
                KeyboardAware(behavior: Avoid) {
                    ScrollView(height: Fill) { Column { Text("Editor") } }
                }
            }
        }"#,
        Target::Swift,
    )
    .expect("KeyboardAware should reuse ScrollView")
    .module;
    let swift = SwiftBackend.generate(&keyboard);
    let kotlin = KotlinBackend.generate(&keyboard);
    assert!(swift.contains("hasNativeScrollOwner: true"));
    assert!(
        kotlin.contains(
            "NexaKeyboardAwareOwnerPrimitive(interactiveDismiss = true, avoidIme = true)"
        )
    );
}
