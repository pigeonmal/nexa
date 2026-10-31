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
fn safe_area_padding_lowers_selected_edges_and_fixed_spacing_for_both_targets() {
    let source = r#"app Insets {
        body {
            Stack { Text("Profile") }
                .safeAreaPadding(edges: [Top, Leading], spacing: 12)
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("safe-area padding should lower")
            .module;
        let Node::SafeAreaPadding { plan, children } = &module.body[0] else {
            panic!("expected typed safe-area padding IR");
        };
        assert_eq!(plan.edges, vec![LayoutEdge::Top, LayoutEdge::Leading]);
        assert_eq!(plan.spacing, 12.0);
        assert_eq!(children.len(), 1);
        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert!(
            swift.contains("NexaSafeAreaPaddingPrimitive(edges: [.top, .leading], spacing: 12)")
        );
        assert!(swift.contains("safeInsets.removing(edges)"));
        assert!(
            swift.contains(".padding(.top, edges.contains(.top) ? max(0, safeInsets.top) : 0)")
        );
        assert!(swift.contains(".padding(.top, edges.contains(.top) ? spacing : 0)"));
        assert!(
            kotlin.contains("val spacingPadding = androidx.compose.foundation.layout.PaddingValues(")
        );
        assert!(kotlin.contains(".padding(spacingPadding)"));
        assert!(kotlin.contains(".windowInsetsPadding(safeWindowInsets)"));
        assert!(kotlin.contains(
            "NexaSafeAreaPaddingPrimitive(edges = setOf(NexaSafeAreaEdge.Top, NexaSafeAreaEdge.Leading), spacing = 12.dp)"
        ));
        assert!(kotlin.contains("import androidx.compose.foundation.layout.systemBars"));
        assert!(kotlin.contains("import androidx.compose.foundation.layout.displayCutout"));
        assert!(kotlin.contains("import androidx.compose.foundation.layout.waterfall"));
        assert!(!kotlin.contains("import androidx.compose.foundation.layout.safeDrawing"));
        assert!(kotlin.contains("raw.getLeft(density, direction)"));
    }
}

#[test]
fn screen_height_fraction_uses_native_logical_metrics_for_both_targets() {
    let source = r#"app ResponsiveHero {
        body {
            Stack(screenHeightFraction: 0.5) {
                Text("Artist artwork")
            }
        }
    }"#;
    let compilation = compile(source, Target::Swift)
        .expect("screen height fraction should lower to the shared layout style");
    let swift = SwiftBackend.generate(&compilation.module);
    assert!(
        swift.contains("UIScreen.main.bounds.height * 0.5"),
        "{swift}"
    );

    let compilation = compile(source, Target::Kotlin)
        .expect("screen height fraction should lower to the shared layout style");
    let kotlin = KotlinBackend.generate(&compilation.module);
    assert!(kotlin.contains("LocalConfiguration.current.screenHeightDp.toFloat() * 0.5f"));
    assert!(kotlin.contains("import androidx.compose.ui.platform.LocalConfiguration"));
}

#[test]
fn screen_width_fraction_uses_native_logical_metrics_for_both_targets() {
    let source = r#"app ResponsiveDialog {
        body {
            Column(screenWidthFraction: 0.9) {
                Text("Update announcement")
            }
        }
    }"#;
    let compilation = compile(source, Target::Swift)
        .expect("screen width fraction should lower to the shared layout style");
    let swift = SwiftBackend.generate(&compilation.module);
    assert!(
        swift.contains("UIScreen.main.bounds.width * 0.9"),
        "{swift}"
    );

    let compilation = compile(source, Target::Kotlin)
        .expect("screen width fraction should lower to the shared layout style");
    let kotlin = KotlinBackend.generate(&compilation.module);
    assert!(kotlin.contains("LocalConfiguration.current.screenWidthDp.toFloat() * 0.9f"));
    assert!(kotlin.contains("import androidx.compose.ui.platform.LocalConfiguration"));
}

#[test]
fn screen_width_fraction_rejects_conflicting_width_options() {
    for source in [
        r#"app InvalidWidth { body { Column(width: 200, screenWidthFraction: 0.9) { Text("Invalid") } } }"#,
        r#"app InvalidWidth { body { Column(fillMaxWidth: true, screenWidthFraction: 0.9) { Text("Invalid") } } }"#,
        r#"app InvalidWidth { body { Column(screenWidthFraction: 1.1) { Text("Invalid") } } }"#,
    ] {
        let error = match compile(source, Target::Swift) {
            Ok(_) => panic!("invalid screen-width fraction combinations should fail"),
            Err(error) => error,
        };
        assert!(
            error.message.contains("screenWidthFraction") || error.message.contains("width"),
            "{error}"
        );
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
fn nested_safe_area_padding_passes_the_remaining_dynamic_edge_once() {
    let source = r#"app NestedInsets {
        body {
            Stack {
                Text("Details").safeAreaPadding(edges: [Bottom], spacing: 3)
            }
            .safeAreaPadding(edges: [Bottom], spacing: 5)
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target)
            .expect("nested safe-area modifiers should lower")
            .module;
        let Node::SafeAreaPadding { plan, children } = &module.body[0] else {
            panic!("expected outer safe-area padding IR");
        };
        assert_eq!(plan.edges, vec![LayoutEdge::Bottom]);
        assert_eq!(plan.spacing, 5.0);
        assert!(matches!(
            children.first(),
            Some(Node::SafeAreaPadding { plan, .. })
                if plan.edges == vec![LayoutEdge::Bottom] && plan.spacing == 3.0
        ));

        let swift = SwiftBackend.generate(&module);
        let kotlin = KotlinBackend.generate(&module);
        assert_eq!(
            swift
                .matches("NexaSafeAreaPaddingPrimitive(edges: [.bottom]")
                .count(),
            2
        );
        assert_eq!(
            kotlin
                .matches("NexaSafeAreaPaddingPrimitive(edges = setOf(NexaSafeAreaEdge.Bottom)")
                .count(),
            2
        );
        assert!(swift.contains("safeInsets.removing(edges)"));
        assert!(kotlin.contains("nexaSafeAreaDynamicInsets()"));
        assert!(kotlin.contains("parent.copy(consumed = parent.consumed + selectedSafe)"));
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
                .safeAreaInset(edge: Bottom, spacing: 8) { Button("Close") { } }
        }
    }"#;
    let module = compile(no_owner, Target::Kotlin)
        .expect("edge attachment works without scrolling")
        .module;
    let Node::SafeAreaInset { plan, content, .. } = &module.body[0] else {
        panic!("expected typed safe-area inset IR");
    };
    assert_eq!(plan.edge, LayoutEdge::Bottom);
    assert_eq!(plan.spacing, 8.0);
    assert!(matches!(plan.scroll_owner, ScrollOwnerResolution::None));
    assert!(matches!(content.first(), Some(Node::Button { .. })));
    let kotlin = KotlinBackend.generate(&module);
    let swift = SwiftBackend.generate(&module);
    assert!(swift.contains(".preferredColorScheme(.dark)"));
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
    assert!(swift.contains(".modifier(NexaSafeAreaEdgePadding(edge: edge, amount: safeInsets.value(at: edge)))"));
    assert!(swift.contains(".modifier(NexaSafeAreaEdgePadding(edge: edge, amount: spacing))"));
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
    assert!(swift.contains(
        "NexaSafeAreaInsetPrimitive(edge: .bottom, spacing: 0, accommodatesEndpoint: true)"
    ));
    assert!(kotlin.contains("accommodatesEndpoint = true"));
    assert!(kotlin.contains("consumed = parent.consumed.offset(edge, safe)"));
    assert!(kotlin.contains(
        "contentPadding = nexaSafeAreaContentPadding(\"Vertical\", includeVisualSafeInsets = true)"
    ));
    assert!(kotlin.contains(
        "modifier = nexaSafeAreaScrollViewportModifier(\"Vertical\").then(Modifier.fillMaxWidth())"
    ));
    assert!(kotlin.contains("if (accommodatesEndpoint) occupied else 0.dp"));
    assert!(kotlin.contains("internal fun nexaSafeAreaScrollViewportModifier("));
    assert!(kotlin.contains("bottom = if (vertical) endpoint.bottom else 0.dp"));
    assert!(kotlin.contains("val safeContextForAttachment = parent.copy("));
    assert!(kotlin.contains("val attachedSpacing = if (accommodatesEndpoint) 0.dp else spacing"));
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
                .safeAreaInset(edge: Bottom, spacing: 0) { Button("Library") { } }
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
    assert!(kotlin.contains(
        "contentPadding = nexaSafeAreaContentPadding(\"Vertical\", includeVisualSafeInsets = false)"
    ));
    assert!(kotlin.contains("NexaSafeAreaInsetPrimitive("));
}

#[test]
fn fast_list_content_respects_remaining_visual_safe_insets_by_default() {
    let source = r#"app Feed {
        body {
            FastList(count: 4) { index in Text("Article \(index + 1)") }
        }
    }"#;
    let module = compile(source, Target::Kotlin)
        .expect("a standard fast list should compile")
        .module;
    let kotlin = KotlinBackend.generate(&module);
    assert!(kotlin.contains(
        "contentPadding = nexaSafeAreaContentPadding(\"Vertical\", includeVisualSafeInsets = true)"
    ));
    assert!(kotlin.contains(
        "val remaining = if (includeVisualSafeInsets) nexaSafeAreaDynamicInsets() else NexaSafeAreaInsets()"
    ));
    assert!(kotlin.contains("top = if (vertical) remaining.top else 0.dp"));
}

#[test]
fn invalid_safe_area_edges_and_spacing_are_rejected_during_semantic_lowering() {
    for source in [
        r#"app BadEdge { body { Text("X").safeAreaInset(edge: Center) { Text("Y") } } }"#,
        r#"app BadSpacing { body { Text("X").safeAreaPadding(spacing: -4) } }"#,
    ] {
        let error = match compile(source, Target::Swift) {
            Ok(_) => panic!("invalid layout values should fail"),
            Err(error) => error,
        };
        assert!(
            error.message.contains("safe-area edge") || error.message.contains("spacing"),
            "unexpected error: {}",
            error.message
        );
    }
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
