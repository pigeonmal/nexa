#[test]
fn parses_async_instance_and_static_class_methods() {
    let program = nexa_syntax::parse_program(
        r#"
class Repository {
    async fn load() -> Int32 { return 1 }
    static async fn count() -> Int32 { return 2 }
}
app Demo { body { Text("ready") } }
"#,
    )
    .expect("async class methods should parse");

    assert_eq!(program.classes[0].methods.len(), 1);
    assert!(program.classes[0].methods[0].is_async);
    assert_eq!(program.classes[0].static_methods.len(), 1);
    assert!(program.classes[0].static_methods[0].is_async);
}
