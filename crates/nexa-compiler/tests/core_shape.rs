use std::fs;

use nexa_compiler::{
    Target, compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots,
    compile_file_with_warnings_for_target,
};
use nexa_ir::{ButtonShape, Node, ShapeMeasurement};
use nexa_testkit::TestProject;

#[test]
fn filled_shapes_lower_to_native_primitives_on_both_targets() {
    let project = TestProject::new("nexa-filled-shape");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r##"app ShapeParity {
    body {
        Row(gap: 4) {
            Shape(shape: Capsule, width: 29.25, height: 3.51, fill: "#FFFFFF")
            Shape(shape: Rounded(2), width: 16, height: 4, fill: "#CA2828")
            Shape(shape: Circle, width: 40, height: 16, fill: "#356EAF")
        }
    }
}
"##,
    )
    .expect("shape source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("filled shapes should compile for SwiftUI")
        .module;
    let shapes = swift_module
        .body
        .iter()
        .flat_map(|node| match node {
            Node::Layout { children, .. } => children.as_slice(),
            _ => &[],
        })
        .filter_map(|node| match node {
            Node::Shape { shape, .. } => Some(*shape),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        shapes,
        [
            ButtonShape::Capsule,
            ButtonShape::Rounded(2.0),
            ButtonShape::Circle
        ]
    );
    let swift = nexa_backend_swift::SwiftBackend
        .generate_units(&swift_module)
        .units
        .into_iter()
        .map(|unit| unit.contents)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        swift.contains("Capsule().fill(Color(")
            && swift.contains("frame(width: 29.25, height: 3.51)"),
        "SwiftUI should draw the active pill as a concrete shape: {swift}"
    );
    assert!(
        swift.contains("RoundedRectangle(cornerRadius: 2).fill(Color("),
        "SwiftUI should preserve the requested rounded radius: {swift}"
    );

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("filled shapes should compile for Compose")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);
    assert!(
        kotlin.contains(
            "Spacer(modifier = Modifier.size(width = 29.25.dp, height = 3.51.dp).background(Color("
        ),
        "Compose should draw the active pill using the shared native shape primitive: {kotlin}"
    );
    assert!(
        kotlin.contains("RoundedCornerShape(2.dp)"),
        "Compose should preserve the requested rounded radius: {kotlin}"
    );
    assert!(kotlin.contains("shape = androidx.compose.foundation.shape.CircleShape"));
    assert!(kotlin.contains("shape = NexaCenteredCircleShape"));
    assert!(kotlin.contains("internal object NexaCenteredCircleShape"));
    assert!(!kotlin.contains("NexaMeasuredShapePrimitive"));

    let kotlin_dev = include_str!("../../../runtime/android/NexaDevRenderer.kt");
    assert!(kotlin_dev.contains("encodedShape == \"Circle\" -> NexaCenteredCircleShape"));

    let swift_dev = include_str!("../../../runtime/ios/NexaDevRenderer.swift");
    let shape = swift_dev
        .split_once("case NexaDevNodeKind.shape:")
        .expect("Swift shape renderer")
        .1
        .split_once("case \"Content\":")
        .expect("end of Swift shape renderer")
        .0;
    assert!(shape.contains("guard let animation else { return AnyView(framed) }"));
    assert_eq!(shape.matches(".animation(animation, value:").count(), 1);
}

#[test]
fn shape_measurement_defers_safe_dimensions_and_keeps_calls_in_composition() {
    let project = TestProject::new("nexa-shape-measurement");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r##"
fn extraHeight() -> Float64 { return 3.0 }

app ShapeMeasurementCases {
    state tileWidth: Float64 = 8.0
    state tileHeight: Float64 = 12.0
    body {
        Column(gap: 2) {
            Shape(shape: Capsule, width: tileWidth, height: 12, fill: "#007AFF")
            Shape(shape: Circle, width: tileWidth + 2.0, height: tileHeight * 2.0, fill: "#007AFF")
            Shape(shape: Rounded(2), width: tileWidth + extraHeight(), height: tileHeight, fill: "#007AFF")
            Shape(shape: Capsule, width: tileWidth, height: tileHeight, fill: "#007AFF", animation: Tween(duration: 0.2, easing: Linear))
            Shape(shape: Capsule, width: 2.0 + 2.0, height: 12, fill: "#007AFF")
        }
    }
}
"##,
    )
    .expect("write the shape measurement fixture");

    let aot = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("compile AOT shape measurement");
    let dev = compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
        &entry,
        &[Target::Kotlin],
        &Default::default(),
    )
    .expect("compile hot reload shape measurement");
    let modes = |nodes: &[Node]| {
        let mut modes = Vec::new();
        nexa_ir::walk::walk_ir(
            nodes,
            &mut |node| {
                if let Node::Shape { measurement, .. } = node {
                    modes.push(*measurement);
                }
            },
            &mut |_| {},
        );
        modes
    };
    let expected = [
        ShapeMeasurement::Width,
        ShapeMeasurement::Both,
        ShapeMeasurement::None,
        ShapeMeasurement::None,
        ShapeMeasurement::None,
    ];
    assert_eq!(modes(&aot.module.body), expected);
    assert_eq!(modes(&dev[0].module.body), expected);

    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&aot.module);
    assert!(kotlin.contains("widthReader = { (nexa_tileWidth).dp.coerceAtLeast(0.dp).value }"));
    assert!(kotlin.lines().any(|line| {
        line.contains("Spacer(modifier = Modifier.size(")
            && line.contains("extraHeight()")
            && line.contains("nexa_tileHeight")
    }));
    assert!(kotlin.contains("animateDpAsState(targetValue = (nexa_tileWidth)"));
    assert!(kotlin.contains("internal fun NexaMeasuredShapePrimitive"));
}
