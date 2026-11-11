use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::{Expr, Node};
use nexa_testkit::TestProject;

fn compile(source: &str, target: Target) -> Result<nexa_ir::Module, String> {
    let project = TestProject::new("marquee");
    let file = project.join("App.nx");
    std::fs::write(&file, source).expect("write test app");
    compile_file_with_warnings_for_target(&file, target)
        .map(|c| c.module)
        .map_err(|e| e.to_string())
}

#[test]
fn marquee_infers_direction_and_preserves_reactive_playback_through_codegen() {
    let source = r#"app News {
        state playing: Bool = true
        state title: String = "Latest music news"
        body {
            Row(width: 200, animation: Marquee(speed: 32, gap: 24, delay: 1, edgeFade: 16.5, playing: playing)) { Text(title) }
            Column(height: 64, animation: Marquee()) { Stack { Text("Daily announcements") } }
        }
    }"#;
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(source, target).expect("marquee app compiles");
        let Node::Layout { style, .. } = &module.body[0] else {
            panic!("layout")
        };
        let spec = style.marquee.as_ref().expect("typed marquee");
        assert_eq!((spec.speed, spec.gap, spec.delay), (32.0, 24.0, 1.0));
        assert_eq!(spec.edge_fade, 16.5);
        let Node::Layout {
            style: default_style,
            ..
        } = &module.body[1]
        else {
            panic!("column")
        };
        assert_eq!(
            default_style
                .marquee
                .as_ref()
                .expect("default marquee")
                .edge_fade,
            0.0
        );
        assert!(matches!(spec.playing.as_ref(), Expr::State(name, _) if name == "playing"));
        let mut reads = Vec::new();
        nexa_ir::walk::walk_ir(&module.body, &mut |_| {}, &mut |expr| {
            if let Expr::State(name, _) = expr {
                reads.push(name.clone())
            }
        });
        assert!(reads.contains(&"playing".to_owned()));
        assert!(module.states.iter().any(|s| s.name == "playing"));
        let encoded = serde_json::to_value(&module.body[0]).expect("Dev IR");
        assert_eq!(encoded["Layout"]["style"]["marquee"]["speed"], 32.0);
        assert_eq!(encoded["Layout"]["style"]["marquee"]["edge_fade"], 16.5);
        let decoded: Node = serde_json::from_value(encoded).expect("Dev IR round-trip");
        let Node::Layout {
            style: decoded_style,
            ..
        } = decoded
        else {
            panic!("layout")
        };
        assert_eq!(
            decoded_style
                .marquee
                .as_ref()
                .expect("decoded marquee")
                .edge_fade,
            16.5
        );
        match target {
            Target::Swift => {
                let code = nexa_backend_swift::SwiftBackend
                    .generate_units(&module)
                    .units
                    .into_iter()
                    .map(|u| u.contents)
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(code.contains("NexaMarqueePrimitive(horizontal: true"));
                assert!(code.contains("NexaMarqueePrimitive(horizontal: false"));
                assert!(code.contains("CAReplicatorLayer"));
                assert!(code.contains("edgeFade: 16.5"));
                assert!(!code.contains("AnyView"));
            }
            Target::All => unreachable!("test selects individual targets"),
            Target::Kotlin => {
                let (code, features) =
                    nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
                assert!(features.uses_compose_animation);
                assert!(features.uses_lifecycle_events);
                assert!(features.uses_coroutines);
                assert!(code.contains("NexaMarqueePrimitive(horizontal = true"));
                assert!(code.contains("NexaMarqueePrimitive(horizontal = false"));
                assert!(code.contains("NexaMarqueeGeometry"));
                assert!(code.contains("edgeFade = 16.5"));
                assert!(code.contains("drawWithContent"));
                assert!(code.contains("revision = listOf((nexa_title).toString())"));
                assert!(!code.contains("contentInvalidated"));
            }
        }
    }
}

#[test]
fn marquee_reports_invalid_configuration_and_unsupported_contexts() {
    for (body, expected) in [
        (
            "Row(animation: Marquee(axis: Horizontal)) { Text(\"News\") }",
            "no axis",
        ),
        (
            "Row(animation: Marquee(speed: 0)) { Text(\"News\") }",
            "positive",
        ),
        (
            "Row(animation: Marquee(gap: -1)) { Text(\"News\") }",
            "nonnegative",
        ),
        (
            "Row(animation: Marquee(edgeFade: -1)) { Text(\"News\") }",
            "edgeFade must be a finite nonnegative",
        ),
        (
            "Row(animation: Marquee(edgeFade: true)) { Text(\"News\") }",
            "edgeFade must be a finite nonnegative",
        ),
        (
            "Row(animation: Marquee(edgeFade: SystemInsets.top)) { Text(\"News\") }",
            "numeric literal",
        ),
        (
            "Row(animation: Marquee(playing: 1)) { Text(\"News\") }",
            "Bool",
        ),
        (
            "Stack(animation: Marquee()) { Text(\"News\") }",
            "wrap Stack",
        ),
        (
            "Row(animation: Marquee()) { Button(\"Action\") {} }",
            "display-only",
        ),
        (
            "Row(animation: Marquee()) { Text(\"News\", selectable: true) }",
            "display-only",
        ),
        (
            "Row(animation: Marquee()) { Column(animation: Marquee()) { Text(\"News\") } }",
            "display-only",
        ),
        (
            "Button(\"Start\") { withAnimation(Marquee()) {} }",
            "timing curve",
        ),
        (
            "NumberFlow(value: 10, animation: Marquee())",
            "timing curve",
        ),
    ] {
        let source = format!("app News {{ body {{ {body} }} }}");
        let error = compile(&source, Target::Swift).expect_err(body);
        assert!(error.contains(expected), "{body}: {error}");
    }
}

#[test]
fn marquee_checks_custom_component_bodies_and_slots() {
    let source = r#"component ActionCard() { body { Button("Action") {} } }
        app News { body { Row(animation: Marquee()) { ActionCard() } } }"#;
    let error = compile(source, Target::Swift).expect_err("interactive custom component");
    assert!(error.contains("display-only"), "{error}");
    let source = r#"component Card() { body { Stack { Content() } } }
        app News { body { Row(animation: Marquee()) { Card() { Text("News") } } } }"#;
    compile(source, Target::Swift).expect("display slot component");
}

#[test]
fn image_transport_does_not_request_an_absent_full_network_bridge() {
    let module = compile(
        r#"app Gallery { body { Image(url: "https://example.com/cover.jpg", description: "Album artwork") } }"#,
        Target::Kotlin,
    )
    .expect("image app");
    let features = nexa_backend_kotlin::KotlinBackend.analyze_project_features(&module);
    assert!(features.uses_network);
    assert!(!features.uses_network_api);
}
