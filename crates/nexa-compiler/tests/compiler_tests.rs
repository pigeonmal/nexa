use std::{fs, path::Path};

use nexa_ir::{
    AccessibilityRole, Action, ArithmeticOp, Expr, MemberKind, Node, NumericType, Type,
    walk::{walk_actions, walk_ir},
};

use nexa_compiler::{Target, compile, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

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
    assert_eq!(utilities.len(), 4);
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
            operation: nexa_ir::CollectionUtilityKind::Shuffled,
            ..
        }
    ));
    assert!(matches!(
        utilities[2],
        Expr::CollectionUtility {
            operation: nexa_ir::CollectionUtilityKind::Reverse,
            ..
        }
    ));
    assert!(matches!(
        utilities[3],
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
fn chained_text_styles_lower_to_native_text_style_fields() {
    let module = compile(
        r#"
        app ChainedTextStyles {
            body {
                Text("Hi").fontSize(18).bold().padding(12)
            }
        }
        "#,
    )
    .expect("text style chains should lower to native style fields");

    assert!(matches!(
        &module.body[0],
        Node::Text { value: Expr::String(value), style }
            if value == "Hi"
                && style.font_size == Some(18.0)
                && style.font_weight == Some(nexa_ir::FontWeight::Bold)
                && style.padding == Some(12.0)
    ));
}

#[test]
fn visual_modifiers_lower_to_validated_static_effects() {
    let module = compile(
        r##"
        app VisualEffects {
            body {
                Column {
                    Text("Card").scale(0.95).clip(shape: Rounded(5))
                }
                .opacity(0.8)
                .scale(1.1)
                .rotation(-6)
                .shadow(radius: 4, x: 1, y: -2, color: "#00000080")
                .blur(2)
                .clip(shape: Rounded(12))
                .zIndex(-3)
            }
        }
        "##,
    )
    .expect("valid static visual effects should compile");

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
fn conditional_transitions_lower_to_typed_ir() {
    let module = compile(
        r#"app ConditionalTransitions {
            state visible = false
            body {
                if visible { Text("shown") }.transition(.fade)
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
            ..
        }
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
fn visual_modifiers_reject_invalid_ranges_and_shapes() {
    for (body, expected) in [
        (
            "Column { Text(\"x\") }.opacity(1.1)",
            "opacity must be between 0 and 1",
        ),
        ("Column { Text(\"x\") }.blur(-1)", "blur radius"),
        (
            "Column { Text(\"x\") }.clip(shape: Circle())",
            "Rounded(radius)",
        ),
        ("Column { Text(\"x\") }.zIndex(2147483648)", "Int32 range"),
    ] {
        let source = format!("app Invalid {{ body {{ {body} }} }}");
        let error = compile(&source).expect_err("invalid visual modifiers should fail");
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
                Text("Continue", accessibilityLabel: "Continue action", accessibilityHint: "Opens the next screen", accessibilityRole: Button)
                Image(asset: "brand", description: "Brand", accessibilityLabel: "Brand mark", accessibilityRole: Image)
                Caption(title: "Profile", accessibilityLabel: "Profile heading", accessibilityRole: Header)
            }
        }
        "#,
    )
    .expect("component accessibility options should compile");

    let [
        Node::Accessibility {
            label: Expr::String(label),
            hint: Some(Expr::String(hint)),
            role: AccessibilityRole::Button,
            children: text_children,
        },
        Node::Accessibility {
            label: Expr::String(image_label),
            role: AccessibilityRole::Image,
            children: image_children,
            ..
        },
        Node::Accessibility {
            label: Expr::String(custom_label),
            role: AccessibilityRole::Header,
            children: custom_children,
            ..
        },
    ] = module.body.as_slice()
    else {
        panic!("accessibility parameters should become typed annotations on each component");
    };
    assert_eq!(label, "Continue action");
    assert_eq!(hint, "Opens the next screen");
    assert_eq!(image_label, "Brand mark");
    assert_eq!(custom_label, "Profile heading");
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
        Node::Slider { state, min, max, step }
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
        r#"
        app ProgressControls {
            state progress: Float64 = 0.5
            body {
                ProgressBar(progress: progress)
                ProgressRing(progress: progress + 0.1)
            }
        }
        "#,
    )
    .expect("native progress controls should accept Float64 expressions");

    assert!(matches!(
        &module.body[0],
        Node::ProgressBar { progress: Expr::State(name, Type::Numeric(NumericType::Float64)) }
            if name == "progress"
    ));
    assert!(matches!(
        &module.body[1],
        Node::ProgressRing {
            progress: Expr::Add(_, _, NumericType::Float64)
        }
    ));
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
        } if name == "filters" && **element == Type::String && state == "selectedFilter"
    ));

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
fn dialog_lowers_typed_text_and_actions_for_mutable_state() {
    let module = compile(
        r#"
        app DeleteConfirmation {
            state isPresented: Bool = false
            state deleted: Bool = false

            body {
                Dialog(isPresented: isPresented, title: "Delete item?", message: "This cannot be undone.") {
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
            title: Expr::String(title),
            message: Expr::String(message),
            children,
        } if state == "isPresented"
            && title == "Delete item?"
            && message == "This cannot be undone."
            && children.len() == 2
            && children.iter().all(|node| matches!(node, Node::Button { .. }))
    ));

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
    let entry =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/video-player-demo.nx");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("VideoPlayer example should lower for each target")
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
        for nodes in std::iter::once(&module.body)
            .chain(module.components.iter().map(|component| &component.body))
        {
            walk_ir(
                nodes,
                &mut |node| {
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
        assert!(component_event_handlers.iter().all(|handlers| {
            handlers.len() == 1 && handlers[0].0 == "onTapped" && handlers[0].1
        }));

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
