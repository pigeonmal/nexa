use nexa_codegen::Backend;
use nexa_compiler::compile;

fn generated(container: &str, row: &str, header: &str, swipe: &str) -> String {
    let source = format!(
        r#"app LibraryBindings {{
            state groups: Array<Array<String>> = [["Saved tracks"]]
            state selected: String = ""
            body {{
                {container} {{
                    FastList(sections: groups, key: .self) {{ item, index, section in
                        {row}
                    }}.sectionHeader {{ {header} }}{swipe}
                }}
            }}
        }}"#
    );
    let module = compile(&source).expect("section bindings should compile");
    nexa_backend_swift::SwiftBackend.generate(&module)
}

#[test]
fn unused_section_bindings_are_not_materialized_in_either_list_presentation() {
    for platform in [false, true] {
        let container = "Column";
        let swipe = if platform {
            r#".swipeActions { Button("Archive") { selected = "Archived" } }"#
        } else {
            ""
        };
        let swift = generated(
            container,
            r#"Text("Archived library")"#,
            r#"Text("Saved music")"#,
            swipe,
        );
        for binding in ["item", "index", "section"] {
            assert!(
                !swift.contains(&format!("let nexa_{binding}:")),
                "unused {binding} in {container}"
            );
        }
        let section_arrays = swift.matches("let nexa_sectionItems:").count();
        assert_eq!(
            section_arrays,
            usize::from(platform),
            "only the platform row loop needs its section array"
        );
    }
}

#[test]
fn used_header_and_row_bindings_remain_typed_without_duplicate_section_arrays() {
    for platform in [false, true] {
        let container = "Column";
        let swipe = if platform {
            r#".swipeActions { Button("Archive") { selected = "Archived" } }"#
        } else {
            ""
        };
        let swift = generated(
            container,
            r#"Text("$item · $index · $section")"#,
            r#"Text(sectionItems[0]) Text("Library group $section")"#,
            swipe,
        );
        assert!(swift.contains("let nexa_item: String"));
        assert!(swift.contains("let nexa_index: Int32"));
        assert_eq!(swift.matches("let nexa_section: Int32").count(), 2);
        assert_eq!(swift.matches("let nexa_sectionItems:").count(), 1);
    }
}

#[test]
fn swipe_only_item_capture_is_preserved_when_row_content_is_static() {
    let swift = generated(
        "Column",
        r#"Text("Saved track")"#,
        r#"Text("Library")"#,
        r#".swipeActions { Button("Select") { selected = item } }"#,
    );
    assert!(swift.contains("let nexa_item: String"));
    assert!(!swift.contains("let nexa_index: Int32"));
    assert!(!swift.contains("let nexa_section: Int32"));
    assert!(swift.contains("nexa_selected = nexa_item"));
}
