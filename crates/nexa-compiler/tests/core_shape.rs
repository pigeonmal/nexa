use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::{ButtonShape, Node};
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
    assert_eq!(shapes, [ButtonShape::Capsule, ButtonShape::Rounded(2.0)]);
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
            "Spacer(modifier = Modifier.width(29.25.dp).height(3.51.dp).background(Color("
        ),
        "Compose should draw the active pill using the shared native shape primitive: {kotlin}"
    );
    assert!(
        kotlin.contains("RoundedCornerShape(2.dp)"),
        "Compose should preserve the requested rounded radius: {kotlin}"
    );
}
