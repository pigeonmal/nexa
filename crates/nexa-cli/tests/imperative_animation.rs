use nexa_codegen::Backend;

#[test]
fn imperative_float_animation_is_lowered_for_both_native_backends() {
    let module = nexa_compiler::compile(
        r#"app AnimatedProgress {
            state progress: Float64 = 0.0
            body {
                Button("Complete") {
                    withAnimation(Spring(response: 0.35, damping: 0.8)) {
                        progress = 1.0
                    }
                }
                ProgressBar(progress: progress)
            }
        }"#,
    )
    .expect("imperative Float64 animation should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("withAnimation(.interpolatingSpring(mass: 1, stiffness: 816.32654, damping: 45.714287, initialVelocity: 0)) {"));
    assert!(swift.contains(".modifier(NexaAnimationAccessibilityModifier())"));
    assert!(swift.contains("@Environment(\\.accessibilityReduceMotion) private var reduceMotion"));
    assert!(swift.contains("transaction.disablesAnimations = true"));
    assert!(nexa_ir::facts::ModuleFacts::analyze(&module).uses_with_animation);

    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(features.uses_compose_animation);
    assert!(
        kotlin.contains("nexa_progressAnimationSpec = androidx.compose.animation.core.spring(")
    );
    assert!(kotlin.contains("nexa_progressAnimationSpec by remember"));
    assert!(
        kotlin.contains(
            "nexa_progressAnimated by androidx.compose.animation.core.animateFloatAsState("
        )
    );
    assert!(kotlin.contains("androidx.compose.animation.core.animateFloatAsState("));
    assert!(kotlin.contains("nexa_progressAnimated.toDouble()"));
    assert!(kotlin.contains("val nexaAnimatedStateBefore"));
    assert!(kotlin.contains("nexa_progress.toBits() == nexaAnimatedStateBefore"));
    assert!(kotlin.contains("nexa_progressAnimationSpec = androidx.compose.animation.core.snap()"));
}

#[test]
fn tween_animation_is_preserved_in_aot_and_dev_ir() {
    let module = nexa_compiler::compile(
        r#"app TimedProgress {
            state progress: Float64 = 0.2
            body {
                Button("Advance") {
                    withAnimation(Tween(duration: 0.24, easing: EaseOut)) {
                        progress = 0.8
                    }
                }
                ProgressBar(progress: progress)
            }
        }"#,
    )
    .expect("timed state animation should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains(".timingCurve(0.0, 0.0, 0.2, 1.0, duration: 0.24)"));

    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(features.uses_compose_animation);
    assert!(kotlin.contains(
        "androidx.compose.animation.core.tween(durationMillis = 240, easing = androidx.compose.animation.core.LinearOutSlowInEasing)"
    ));

    let dev_module = nexa_dev_ir::lower(&module, "timed-progress");
    let json = serde_json::to_value(dev_module).expect("serialize Tween Dev IR");
    assert_eq!(
        json["module"]["body"][0]["Button"]["actions"][0]["WithAnimation"]["animation"]["Tween"]["duration"],
        0.24
    );
    assert_eq!(
        json["module"]["body"][0]["Button"]["actions"][0]["WithAnimation"]["animation"]["Tween"]["easing"],
        "EaseOut"
    );
    assert_eq!(
        json["module"]["body"][0]["Button"]["actions"][0]["WithAnimation"]["animation"]["Tween"]["delay"],
        0.0
    );
}

#[test]
fn delayed_cubic_bezier_animation_is_shared_by_aot_and_dev_ir() {
    let module = nexa_compiler::compile(
        r#"app CustomMotion {
            state progress: Float64 = 0.0
            body {
                Button("Animate") {
                    withAnimation(Tween(
                        duration: 0.3,
                        delay: 0.08,
                        easing: CubicBezier(0.2, 0.0, 0.2, 1.0)
                    )) {
                        progress = 1.0
                    }
                }
                ProgressBar(progress: progress)
            }
        }"#,
    )
    .expect("custom delayed easing should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains(".timingCurve(0.2, 0, 0.2, 1, duration: 0.3).delay(0.08)"));
    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(features.uses_compose_animation);
    assert!(kotlin.contains(
        "androidx.compose.animation.core.tween(durationMillis = 300, delayMillis = 80, easing = androidx.compose.animation.core.CubicBezierEasing(0.2f, 0f, 0.2f, 1f))"
    ));

    let dev_module = nexa_dev_ir::lower(&module, "custom-motion");
    let json = serde_json::to_value(dev_module).expect("serialize custom motion Dev IR");
    let animation =
        &json["module"]["body"][0]["Button"]["actions"][0]["WithAnimation"]["animation"]["Tween"];
    assert_eq!(animation["duration"], 0.3);
    assert_eq!(animation["delay"], 0.08);
    assert!((animation["easing"]["CubicBezier"]["x1"].as_f64().unwrap() - 0.2).abs() < 1e-6);
    assert!((animation["easing"]["CubicBezier"]["y2"].as_f64().unwrap() - 1.0).abs() < 1e-6);
}

#[test]
fn conditional_transitions_accept_custom_timing_in_aot_and_dev_runtime() {
    let module = nexa_compiler::compile(
        r#"app AnimatedDetails {
            state visible: Bool = false
            body {
                Button("Toggle details") { visible = !visible }
                if visible {
                    Text("Saved preferences")
                }.transition(.slide(from: .bottom), animation: Tween(
                    duration: 0.28,
                    delay: 0.04,
                    easing: EaseOut
                ))
            }
        }"#,
    )
    .expect("conditional transition should accept a custom animation spec");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains(
        ".transition(.move(edge: .bottom)).animation(.timingCurve(0.0, 0.0, 0.2, 1.0, duration: 0.28).delay(0.04), value: nexa_visible)"
    ));

    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(features.uses_compose_animation);
    assert!(kotlin.contains(
        "slideInVertically(animationSpec = androidx.compose.animation.core.tween(durationMillis = 280, delayMillis = 40, easing = androidx.compose.animation.core.LinearOutSlowInEasing)) { height -> height } togetherWith slideOutVertically(animationSpec = androidx.compose.animation.core.tween(durationMillis = 280, delayMillis = 40, easing = androidx.compose.animation.core.LinearOutSlowInEasing)) { height -> height }"
    ));

    let dev_module = nexa_dev_ir::lower(&module, "animated-details");
    let json = serde_json::to_value(dev_module).expect("serialize transition animation Dev IR");
    let animation = &json["module"]["body"][1]["If"]["transition_animation"]["Tween"];
    assert_eq!(animation["duration"], 0.28);
    assert_eq!(animation["delay"], 0.04);
    assert_eq!(animation["easing"], "EaseOut");

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let swift_runtime = std::fs::read_to_string(root.join("runtime/ios/NexaDevRenderer.swift"))
        .expect("read Swift DevRuntime renderer");
    let kotlin_runtime = std::fs::read_to_string(root.join("runtime/android/NexaDevRenderer.kt"))
        .expect("read Compose DevRuntime renderer");
    assert!(
        swift_runtime.contains("nexaDevAnimation(fields[\"transition_animation\"]) ?? .default")
    );
    assert!(
        kotlin_runtime
            .contains("nexaDevContentTransform(transition, floatAnimation, offsetAnimation)")
    );
    assert!(
        kotlin_runtime
            .contains("store.devAnimationSpec<androidx.compose.ui.unit.IntOffset>(rawAnimation)")
    );
    assert!(kotlin_runtime.contains("slideInVertically(animationSpec = spec)"));
}

#[test]
fn segmented_control_selection_uses_native_motion_in_aot_and_dev_runtime() {
    let module = nexa_compiler::compile(
        r#"app AnimatedSegments {
            state choices: Array<String> = ["Day", "Week", "Month"]
            state selected: String = "Day"
            body {
                SegmentedControl(
                    items: choices,
                    selected: selected,
                    animation: Tween(duration: 0.24, easing: EaseOut)
                )
            }
        }"#,
    )
    .expect("segmented control should accept the shared animation spec");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains(
        "NexaSegmentedControlPrimitive(items: nexa_choices, selection: $nexa_selected, animation: .timingCurve(0.0, 0.0, 0.2, 1.0, duration: 0.24), tint: nil, background: nil, textColor: nil, selectedTextColor: nil, fontSize: nil, fontWeight: nil, height: nil, cornerRadius: nil, borderColor: nil, borderWidth: nil)"
    ));
    assert!(swift.contains("struct NexaNativeSegmentedControl: UIViewRepresentable"));
    assert!(swift.contains("@Environment(\\.accessibilityReduceMotion) private var reduceMotion"));
    assert!(swift.contains("if reduceMotion {"));
    assert!(swift.contains("transaction.animation = nil"));
    assert!(swift.contains("transaction.disablesAnimations = true"));
    assert!(swift.contains("else if let animation {"));

    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(features.uses_compose_animation);
    assert!(features.uses_expressive_motion);
    assert!(kotlin.contains(
        "NexaSegmentedControlPrimitive(items = nexa_choices, selected = nexa_selected, animation = androidx.compose.animation.core.tween(durationMillis = 240, easing = androidx.compose.animation.core.LinearOutSlowInEasing), tint = null, background = null, textColor = null, selectedTextColor = null, fontSize = null, fontWeight = null, height = null, cornerRadius = null, borderColor = null, borderWidth = null, onSelected = { nexa_selected = it })"
    ));
    assert!(kotlin.contains(
        "androidx.compose.material3.MotionScheme.expressive()\n                .defaultSpatialSpec<androidx.compose.ui.unit.Dp>()"
    ));
    assert!(kotlin.contains(".offset {"));

    let dev_module = nexa_dev_ir::lower(&module, "animated-segments");
    let json = serde_json::to_value(dev_module).expect("serialize segmented-control Dev IR");
    assert_eq!(
        json["module"]["body"][0]["SegmentedControl"]["animation"]["Tween"]["duration"],
        0.24
    );
    assert_eq!(
        json["module"]["body"][0]["SegmentedControl"]["animation"]["Tween"]["easing"],
        "EaseOut"
    );
}

#[test]
fn segmented_control_visual_parameters_survive_dev_ir_serialization() {
    let module = nexa_compiler::compile(
        r##"app StyledSegments {
            state selected: String = "Day"
            state segmentTint: String = "#FFFFFF"
            body {
                SegmentedControl(
                    items: ["Day", "Week", "Month"],
                    selected: selected,
                    tint: segmentTint,
                    background: "#E9EAF0",
                    selectedTextColor: "#17171A",
                    textColor: "#68686F",
                    fontSize: 14,
                    fontWeight: Bold,
                    height: 36,
                    cornerRadius: 18,
                    borderColor: "#D2D3DA",
                    borderWidth: 1
                )
            }
        }"##,
    )
    .expect("style options should compile for hot reload");
    let dev_module = nexa_dev_ir::lower(&module, "styled-segments");
    let json = serde_json::to_value(dev_module).expect("serialize style options into Dev IR");
    let node = &json["module"]["body"][0]["SegmentedControl"];
    assert!(node["tint"]["Dynamic"].is_object());
    assert_eq!(node["background"]["Static"]["Static"]["red"], 233);
    assert_eq!(node["selected_text_color"]["Static"]["Static"]["red"], 23);
    assert_eq!(node["text_color"]["Static"]["Static"]["red"], 104);
    assert_eq!(node["font_size"], 14.0);
    assert_eq!(node["font_weight"], "Bold");
    assert_eq!(node["height"], 36.0);
    assert_eq!(node["corner_radius"], 18.0);
    assert_eq!(node["border_color"]["Static"]["Static"]["red"], 210);
    assert_eq!(node["border_width"], 1.0);
}

#[test]
fn segmented_control_routes_track_background_to_the_native_bridge() {
    let module = nexa_compiler::compile(
        r##"app NativeStyledSegments {
            state selected: String = "Day"
            body {
                SegmentedControl(
                    items: ["Day", "Week", "Month"],
                    selected: selected,
                    background: "#E8EDF5"
                )
            }
        }"##,
    )
    .expect("native picker background should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("struct NexaNativeSegmentedControl: UIViewRepresentable"));
    assert!(swift.contains("background: background,"));
    assert!(swift.contains("control.backgroundColor = color"));
}

#[test]
fn segmented_control_uses_one_native_control_with_default_and_custom_styles() {
    let module = nexa_compiler::compile(
        r#"app NativeSegments {
            state choices: Array<String> = ["Day", "Week", "Month"]
            state selected: String = "Day"
            body { SegmentedControl(items: choices, selected: selected) }
        }"#,
    )
    .expect("default segmented control should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("struct NexaNativeSegmentedControl: UIViewRepresentable"));
    assert!(swift.contains("NexaNativeSegmentedControl(\n                items: items,"));
    assert!(!swift.contains(".pickerStyle(.segmented)"));
    assert!(!swift.contains("private var customizedControl"));
    assert!(swift.contains("NexaSegmentedControlPrimitive(items: nexa_choices, selection: $nexa_selected, animation: nil, tint: nil, background: nil"));

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(kotlin.contains("val selectionIndicatorInset = androidx.compose.ui.unit.Dp(3f)"));
    assert!(!kotlin.contains("androidx.compose.material3.SingleChoiceSegmentedButtonRow("));
}

#[test]
fn segmented_control_uses_native_selected_fill_and_custom_colors() {
    let module = nexa_compiler::compile(
        r##"app TintedSegments {
            state selected: String = "Day"
            body {
                SegmentedControl(
                    items: ["Day", "Week", "Month"],
                    selected: selected,
                    tint: "#2563EB"
                )
            }
        }"##,
    )
    .expect("native picker tint should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("struct NexaNativeSegmentedControl: UIViewRepresentable"));
    assert!(swift.contains("control.selectedSegmentTintColor = tint.map { UIColor($0) }"));
    assert!(swift.contains("control.setTitleTextAttributes"));
    assert!(!swift.contains("selectedPillImage"));
    assert!(swift.contains("NexaNativeSegmentedControl(\n                items: items,"));
    assert!(!swift.contains("private var customizedControl"));
}

#[test]
fn segmented_control_applies_track_background_to_the_native_control() {
    let module = nexa_compiler::compile(
        r##"app GlassSegments {
            state selected: String = "Day"
            body {
                SegmentedControl(
                    items: ["Day", "Week", "Month"],
                    selected: selected,
                    background: "#E8EDF5"
                )
            }
        }"##,
    )
    .expect("custom segmented-control glass should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("control.backgroundColor = color"));
    assert!(swift.contains("control.layer.backgroundColor = color.cgColor"));
    assert!(swift.contains("control.backgroundColor = color"));
    assert!(swift.contains("control.selectedSegmentTintColor = tint.map { UIColor($0) }"));
    assert!(!swift.contains("selectedPillImage"));
    assert!(!swift.contains("solidTrackImage"));
    assert!(!swift.contains(".glassEffect("));
    assert!(!swift.contains(".pickerStyle(.segmented)"));
}

#[test]
fn segmented_control_routes_all_visual_overrides_to_the_native_bridge() {
    let visual_overrides = [
        ("background", "background", "\"#E8EDF5\""),
        ("textColor", "textColor", "\"#475569\""),
        ("selectedTextColor", "selectedTextColor", "\"#FFFFFF\""),
        ("fontSize", "fontSize", "13"),
        ("fontWeight", "fontWeight", "Semibold"),
        ("height", "height", "36"),
        ("cornerRadius", "cornerRadius", "18"),
        ("borderColor", "borderColor", "\"#CBD5E1\""),
        ("borderWidth", "borderWidth", "1"),
    ];

    for (parameter, swift_property, value) in visual_overrides {
        let source = format!(
            "app StyledSegments {{ state selected: String = \"Day\" body {{ SegmentedControl(items: [\"Day\", \"Week\"], selected: selected, {parameter}: {value}) }} }}"
        );
        let module = nexa_compiler::compile(&source)
            .unwrap_or_else(|error| panic!("{parameter} should compile: {error}"));
        let swift = nexa_backend_swift::SwiftBackend.generate(&module);
        let invocation = swift
            .lines()
            .find(|line| line.contains("NexaSegmentedControlPrimitive(items:"))
            .expect("the SwiftUI segmented primitive should be generated");
        assert!(
            !invocation.contains(&format!("{swift_property}: nil")),
            "{parameter} must reach the SwiftUI primitive"
        );

        assert!(swift.contains("struct NexaNativeSegmentedControl: UIViewRepresentable"));
    }

    let tint_module = nexa_compiler::compile(
        r##"app TintedSegments {
            state selected: String = "Day"
            body { SegmentedControl(items: ["Day", "Week"], selected: selected, tint: "#2563EB") }
        }"##,
    )
    .expect("tint should compile");
    let tint_swift = nexa_backend_swift::SwiftBackend.generate(&tint_module);
    assert!(tint_swift.contains("control.selectedSegmentTintColor = tint.map { UIColor($0) }"));
    assert!(tint_swift.contains("NexaNativeSegmentedControl(\n                items: items,"));
}

#[test]
fn segmented_control_visual_track_is_centered_inside_matching_touch_bounds() {
    let module = nexa_compiler::compile(
        r##"app StyledSegments {
            state selected: String = "Day"
            body {
                SegmentedControl(
                    items: ["Day", "Week", "Month"],
                    selected: selected,
                    background: "#E9EAF0",
                    height: 36,
                    cornerRadius: 18,
                    borderColor: "#D2D3DA",
                    borderWidth: 1
                )
            }
        }"##,
    )
    .expect("custom visual sizing should compile");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("height: height,"));
    assert!(swift.contains(".frame(height: max(32, height ?? 32))"));
    assert!(swift.contains("control.layer.cornerRadius = cornerRadius ?? (background == nil ? 0 : max(32, height ?? 32) / 2)"));
    assert!(swift.contains("control.layer.borderColor = borderColor.map"));
    assert!(swift.contains("control.layer.borderWidth = max(0, borderWidth"));

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(
        kotlin.contains(
            "val outerHeight = maxOf(androidx.compose.ui.unit.Dp(48f), segmentTrackHeight)"
        )
    );
    assert!(kotlin.contains("val selectionIndicatorInset = androidx.compose.ui.unit.Dp(3f)"));
    assert!(kotlin.contains(
        "val selectionIndicatorHeight = (segmentTrackHeight - androidx.compose.ui.unit.Dp(6f))"
    ));
    assert!(kotlin.contains("width = trackBorderWidth"));
    assert!(kotlin.contains(".height(outerHeight)"));
    assert!(kotlin.contains(".height(segmentTrackHeight)"));
    assert!(kotlin.contains(".fillMaxHeight()"));
}

#[test]
fn segmented_control_uses_expressive_native_motion_by_default() {
    let module = nexa_compiler::compile(
        r#"app DefaultAnimatedSegments {
            state choices: Array<String> = ["Day", "Week", "Month"]
            state selected: String = "Day"
            body { SegmentedControl(items: choices, selected: selected) }
        }"#,
    )
    .expect("segmented control should compile without animation configuration");

    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("struct NexaNativeSegmentedControl: UIViewRepresentable"));
    assert!(features.uses_compose_animation);
    assert!(features.uses_expressive_motion);
    assert!(kotlin.contains(
        "NexaSegmentedControlPrimitive(items = nexa_choices, selected = nexa_selected, animation = null, tint = null, background = null, textColor = null, selectedTextColor = null, fontSize = null, fontWeight = null, height = null, cornerRadius = null, borderColor = null, borderWidth = null, onSelected = { nexa_selected = it })"
    ));
    assert!(kotlin.contains(
        "val expressiveSpatialMotion = androidx.compose.runtime.remember {\n            androidx.compose.material3.MotionScheme.expressive()\n                .defaultSpatialSpec<androidx.compose.ui.unit.Dp>()\n        }"
    ));
    assert!(kotlin.contains("animation\n                ?: expressiveSpatialMotion"));
}

#[test]
fn segmented_control_custom_curve_and_delay_survive_aot_and_dev_runtime_lowering() {
    let module = nexa_compiler::compile(
        r#"app DelayedSegments {
            state choices: Array<String> = ["Day", "Week", "Month"]
            state selected: String = "Day"
            body {
                SegmentedControl(
                    items: choices,
                    selected: selected,
                    animation: Tween(
                        duration: 0.32,
                        delay: 0.04,
                        easing: CubicBezier(0.2, 0.0, 0.2, 1.0)
                    )
                )
            }
        }"#,
    )
    .expect("segmented control should accept the full shared tween options");

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("animation: .timingCurve(0.2, 0, 0.2, 1, duration: 0.32).delay(0.04)"));
    let (kotlin, features) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(features.uses_compose_animation);
    assert!(kotlin.contains(
        "animation = androidx.compose.animation.core.tween(durationMillis = 320, delayMillis = 40, easing = androidx.compose.animation.core.CubicBezierEasing(0.2f, 0f, 0.2f, 1f)"
    ));

    let dev_module = nexa_dev_ir::lower(&module, "delayed-segments");
    let json = serde_json::to_value(dev_module).expect("serialize segmented-control animation");
    let animation = &json["module"]["body"][0]["SegmentedControl"]["animation"]["Tween"];
    assert_eq!(animation["duration"], 0.32);
    assert_eq!(animation["delay"], 0.04);
    assert_eq!(animation["easing"]["CubicBezier"]["x1"], 0.2f32);
}

#[test]
fn empty_segmented_control_does_not_render_a_placeholder_track() {
    let module = nexa_compiler::compile(
        r#"app EmptySegments {
            state choices: Array<String> = []
            state selected: String = "Missing"
            body { SegmentedControl(items: choices, selected: selected) }
        }"#,
    )
    .expect("empty segmented-control item lists should compile");

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(kotlin.contains("if (items.isEmpty()) return"));

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("NexaSegmentedControlPrimitive(items: nexa_choices, selection: $nexa_selected, animation: nil, tint: nil, background: nil, textColor: nil, selectedTextColor: nil, fontSize: nil, fontWeight: nil, height: nil, cornerRadius: nil, borderColor: nil, borderWidth: nil)"));
    assert!(swift.contains("if items.isEmpty {"));
    assert!(swift.contains("struct NexaNativeSegmentedControl: UIViewRepresentable"));
    assert!(!swift.contains(".pickerStyle(.segmented)"));
}

#[test]
fn segmented_control_comment_is_attached_to_inline_choice_translations() {
    let module = nexa_compiler::compile(
        r#"app LocalizedSegments {
            state selected: String = "Day"
            body {
                SegmentedControl(
                    items: ["Day", "Week", "Month"],
                    selected: selected,
                    comment: "Date range options"
                )
            }
        }"#,
    )
    .expect("localized segmented choices should compile");

    let entries = nexa_ir::localization::extract(&module);
    for choice in ["Day", "Week", "Month"] {
        assert_eq!(
            entries[choice].comment.as_deref(),
            Some("Date range options"),
            "translator note should be attached to the {choice} option"
        );
    }

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    for choice in ["Day", "Week", "Month"] {
        let resource = nexa_codegen::names::localization_resource_name(choice);
        assert!(kotlin.contains(&format!("R.string.{resource}")));
    }
    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("NSLocalizedString("));
    assert!(swift.contains("comment: \"Date range options\""));

    let dev_module = nexa_dev_ir::lower(&module, "localized-segments");
    let json =
        serde_json::to_value(dev_module).expect("serialize localized segmented-control Dev IR");
    let items = &json["module"]["body"][0]["SegmentedControl"]["items"]["Array"];
    for (index, choice) in ["Day", "Week", "Month"].into_iter().enumerate() {
        assert_eq!(items[index]["LocalizedText"]["key"], choice);
        assert_eq!(
            items[index]["LocalizedText"]["comment"],
            "Date range options"
        );
    }
}

#[test]
fn state_defined_segmented_choices_can_be_localized_at_their_declaration() {
    let module = nexa_compiler::compile(
        r#"app LocalizedStateSegments {
            state choices: Array<String> = [
                Locale.localized("Morning"),
                Locale.localized("Evening")
            ]
            state selected: String = "Morning"
            body {
                SegmentedControl(items: choices, selected: selected)
            }
        }"#,
    )
    .expect("state-defined localized choices should compile");

    let entries = nexa_ir::localization::extract(&module);
    assert!(entries.contains_key("Morning"));
    assert!(entries.contains_key("Evening"));

    let (kotlin, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    for choice in ["Morning", "Evening"] {
        let resource = nexa_codegen::names::localization_resource_name(choice);
        assert!(kotlin.contains(&format!("R.string.{resource}")));
    }

    let swift = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift.contains("NSLocalizedString(\"Morning\""));
    assert!(swift.contains("NSLocalizedString(\"Evening\""));
}
