use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

#[test]
fn explicit_layout_spacing_and_padding_are_preserved_once_on_both_targets() {
    let project = TestProject::new("nexa-layout-style-parity");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app LayoutStyleParity {
    body {
        Column(spacing: 7, padding: 13) {
            Text("outer", padding: 5)
            Row(spacing: 11) {
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
    for padding in ["13", "5", "3"] {
        let modifier = format!(".padding({padding})");
        assert_eq!(swift.matches(&modifier).count(), 1, "{modifier}: {swift}");
    }

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("layout style example should compile for Compose")
        .module;
    let (kotlin, _) =
        nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&kotlin_module);

    assert!(kotlin.contains("spacing = 7f"), "{kotlin}");
    assert!(kotlin.contains("spacing = 11f"), "{kotlin}");
    for padding in ["13", "5", "3"] {
        let modifier = format!(".padding({padding}.dp)");
        assert_eq!(kotlin.matches(&modifier).count(), 1, "{modifier}: {kotlin}");
    }
}
