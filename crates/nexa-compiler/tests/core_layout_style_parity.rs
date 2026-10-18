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
            Row(horizontalArrangement: Start, width: 280, spacing: 8) {
                Text("Explicit start")
                Text("Second")
            }
            Row(horizontalArrangement: Center, width: 280, spacing: 8) {
                Text("Centered")
                Text("Second")
            }
            Row(horizontalArrangement: End, width: 280, spacing: 8) {
                Text("End aligned")
                Text("Second")
            }
            Row(horizontalArrangement: Center, spacing: 8) {
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
    assert!(
        swift.contains(
            ".frame(maxWidth: .infinity, alignment: horizontalArrangement.frameAlignment)"
        )
    );

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
