use std::fs;

use nexa_compiler::{
    Target, compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots,
    compile_file_with_warnings_for_target,
};
use nexa_ir::{Expr, HorizontalArrangement, LayoutKind, Node};
use nexa_testkit::TestProject;

#[test]
fn explicit_layout_spacing_and_padding_are_preserved_once_on_both_targets() {
    let project = TestProject::new("nexa-layout-style-parity");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app LayoutStyleParity {
    body {
        Column(gap: 7, padding: 13) {
            Text("outer", padding: 5)
            Row(gap: 11) {
                Text("first")
                Text("second", padding: 3)
            }
        }
    }
}

"#,
    )
    .expect("test source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("layout style example should compile for SwiftUI")
        .module;
    let swift = nexa_backend_swift::SwiftBackend
        .generate_units(&swift_module)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(swift.contains("spacing: 7)"), "{swift}");
    assert!(swift.contains("spacing: 11)"), "{swift}");
    assert_eq!(swift.matches("NexaViewInsetsPrimitive(padding:").count(), 3);
    for padding in ["13", "5", "3"] {
        assert!(
            swift.contains(&format!("max(0, CGFloat(Double({padding})))")),
            "{swift}"
        );
    }

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("layout style example should compile for Compose")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);

    assert!(kotlin.contains("spacing = 7f"), "{kotlin}");
    assert!(kotlin.contains("spacing = 11f"), "{kotlin}");
    assert_eq!(
        kotlin.matches("NexaViewInsetsPrimitive(paddingTop").count(),
        3
    );
}

#[test]
fn screen_fraction_layouts_use_the_shared_window_viewport_on_both_targets() {
    let project = TestProject::new("nexa-layout-screen-fraction-viewport");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app ScreenFractionViewport {
    screen ViewportCases {
        Column(width: screenWidthFraction(0.9), height: screenHeightFraction(0.12)) {
            Text("Viewport-sized content")
        }
    }

    body {
        NavigationStack(root: ViewportCases)
    }
}

"#,
    )
    .expect("test source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("screen fraction example should compile for SwiftUI")
        .module;
    let swift = nexa_backend_swift::SwiftBackend
        .generate_units(&swift_module)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        swift.contains("@Environment(\\.nexaWindowMetrics) private var nexaWindowMetrics"),
        "screen view should read the shared window snapshot:\n{swift}"
    );
    assert!(swift.contains("NexaEdgeToEdgeRoot(edgeToEdge: false) { nexaWindowMetrics in"));
    assert!(
        swift.contains("nexaWindowMetrics.viewportSize.width * 0.9"),
        "{swift}"
    );
    assert!(
        swift.contains("nexaWindowMetrics.viewportSize.height * 0.12"),
        "{swift}"
    );
    assert!(!swift.contains("UIScreen.main.bounds"), "{swift}");

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("screen fraction example should compile for Compose")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);
    assert!(kotlin.contains("NexaSafeAreaProvider {"));
    assert!(
        kotlin.contains("LocalNexaWindowMetrics.current.viewportWidthDp"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("LocalNexaWindowMetrics.current.viewportHeightDp"),
        "{kotlin}"
    );
    assert!(
        !kotlin.contains("LocalConfiguration.current.screen"),
        "{kotlin}"
    );
}

#[test]
fn fixed_layout_dimensions_must_satisfy_bounds_on_both_targets() {
    let project = TestProject::new("nexa-layout-dimension-bounds");
    let entry = project.join("App.nx");
    for (arguments, expected) in [
        (
            "width: 100, minWidth: 120",
            "`width` cannot be less than `minWidth`",
        ),
        (
            "width: 300, maxWidth: 240",
            "`width` cannot be greater than `maxWidth`",
        ),
        (
            "height: 40, minHeight: 60",
            "`height` cannot be less than `minHeight`",
        ),
        (
            "height: 100, maxHeight: 80",
            "`height` cannot be greater than `maxHeight`",
        ),
    ] {
        fs::write(
            &entry,
            format!(
                "app LayoutBounds {{ body {{ Column({arguments}) {{ Text(\"content\") }} }} }}"
            ),
        )
        .expect("test source should be written");

        for target in [Target::Swift, Target::Kotlin] {
            let error = match compile_file_with_warnings_for_target(&entry, target) {
                Ok(_) => panic!("contradictory fixed and flexible dimensions must be rejected"),
                Err(error) => error,
            };
            assert_eq!(error.message, expected);
            assert!(
                error.span.start > 0,
                "diagnostic should point to the fixed size"
            );
        }
    }
}

#[test]
fn row_horizontal_arrangement_is_typed_and_maps_to_both_backends() {
    let project = TestProject::new("nexa-row-horizontal-arrangement");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r##"app RowArrangementParity {
    body {
        Column {
            Row(height: 40, background: "#F2F2F7") { Text("Default start") }
            Row(horizontalArrangement: Start, width: 280, gap: 8) {
                Text("Explicit start")
                Text("Second")
            }
            Row(horizontalArrangement: Center, width: 280, gap: 8) {
                Text("Centered")
                Text("Second")
            }
            Row(horizontalArrangement: End, width: 280, gap: 8) {
                Text("End aligned")
                Text("Second")
            }
            Row(horizontalArrangement: Center, gap: 8) {
                Text("Fills available width")
            }
        }
    }
}
"##,
    )
    .expect("row arrangement example should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("row arrangement should compile for SwiftUI")
        .module;
    let swift = nexa_backend_swift::SwiftBackend
        .generate_units(&swift_module)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        swift.contains("NexaRowPrimitive(alignment: .center, spacing: 0) {"),
        "an omitted arrangement must keep the default start behavior: {swift}"
    );
    for arrangement in [".start", ".center", ".end"] {
        assert!(
            swift.contains(&format!("horizontalArrangement: {arrangement}")),
            "SwiftUI should emit horizontal arrangement {arrangement}: {swift}"
        );
    }
    assert!(swift.contains("horizontalArrangement?.frameAlignment"));
    assert!(swift.contains("?? (fillsSafeAreaRootWidth ? .leading : nil)"));
    assert!(swift.contains("if let frameAlignment {"));
    assert!(swift.contains(".frame(maxWidth: .infinity, alignment: frameAlignment)"));

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("row arrangement should compile for Compose")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);
    for arrangement in ["Start", "Center", "End"] {
        assert!(
            kotlin.contains(&format!(
                "horizontalArrangement = androidx.compose.foundation.layout.Arrangement.{arrangement}"
            )),
            "Compose should emit horizontal arrangement {arrangement}: {kotlin}"
        );
    }
    assert!(kotlin.contains(
        "horizontalArrangement: androidx.compose.foundation.layout.Arrangement.Horizontal? = null"
    ));
    assert!(kotlin.contains("modifier.fillMaxWidth()"));
    assert!(kotlin.contains("Arrangement.spacedBy("));
}

#[test]
fn row_horizontal_arrangement_rejects_unknown_values() {
    let project = TestProject::new("nexa-row-horizontal-arrangement-invalid");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app InvalidRowArrangement {
    body {
        Row(horizontalArrangement: SpaceAround) {
            Text("Invalid arrangement")
        }
    }
}
"#,
    )
    .expect("invalid row arrangement source should be written");

    let error = match compile_file_with_warnings_for_target(&entry, Target::Swift) {
        Ok(_) => panic!("unsupported row arrangements should be rejected"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("horizontalArrangement must be `Start`, `Center`, or `End`"),
        "unexpected compiler diagnostic: {error}"
    );
}

#[test]
fn style_lowering_preserves_native_layer_topology_for_aot_and_dev() {
    let project = TestProject::new("nexa-identity-effects");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app StylePlan {
    state scaleFactor: Float64 = 2.0
    body {
        Column(width: 200, opacity: 0.5 + 0.5, scale: 1, rotation: 0, offsetY: 0, blur: 0) {
            Text("Identity", opacity: 1, scale: 1, scaleY: 1)
            Text("Transform", scale: scaleFactor, scaleY: 1, opacity: 0.5)
            Row(horizontalArrangement: Center) { Text("Centered") }
        }
        ScrollView(opacity: 1, rotation: 0) { Text("Scrollable") }
    }
}"#,
    )
    .expect("write layout fixture");

    let targets = [Target::Swift, Target::Kotlin];
    let dev = compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
        &entry,
        &targets,
        &Default::default(),
    )
    .expect("compile DevRuntime style plan");
    for (target, dev) in targets.into_iter().zip(dev) {
        let aot =
            compile_file_with_warnings_for_target(&entry, target).expect("compile AOT style plan");
        assert_eq!(aot.module.body, dev.module.body);
        let swift = target == Target::Swift;
        let mut checked_nodes = 0;
        nexa_ir::walk::walk_ir(
            &aot.module.body,
            &mut |node| {
                match node {
                    Node::Layout { style, .. } if style.width.is_some() => {
                        assert_eq!(style.opacity, None);
                        let effects = &style.effects;
                        if swift {
                            assert!(!effects.has_modifiers());
                        } else {
                            assert!(matches!(
                                effects.opacity_expression.as_deref(),
                                Some(Expr::Number { raw, .. }) if raw.parse::<f64>() == Ok(1.0)
                            ));
                            assert_eq!(effects.scale, Some(1.0));
                            assert_eq!(effects.rotation, Some(0.0));
                            assert_eq!(effects.blur, Some(0.0));
                            assert!(effects.offset_y_expression.is_some());
                        }
                    }
                    Node::Layout {
                        kind: LayoutKind::Row,
                        style,
                        ..
                    } => {
                        assert_eq!(
                            style.horizontal_arrangement,
                            Some(HorizontalArrangement::Center)
                        );
                    }
                    Node::ScrollView { style, .. } => {
                        assert_eq!(style.opacity, Some(1.0));
                        assert_eq!(style.effects.rotation, (!swift).then_some(0.0));
                    }
                    Node::Text {
                        value: Expr::LocalizedText { key, .. },
                        style,
                    } if key == "Identity" => {
                        assert_eq!(style.opacity, Some(1.0));
                        assert_eq!(style.effects.scale, (!swift).then_some(1.0));
                        assert_eq!(style.effects.scale_y, (!swift).then_some(1.0));
                    }
                    Node::Text {
                        value: Expr::LocalizedText { key, .. },
                        style,
                    } if key == "Transform" => {
                        assert_eq!(style.opacity, Some(0.5));
                        assert!(style.effects.scale_expression.is_some());
                        assert_eq!(style.effects.scale_y, Some(1.0));
                    }
                    _ => return,
                }
                checked_nodes += 1;
            },
            &mut |_| {},
        );
        assert_eq!(checked_nodes, 5);
    }
}
