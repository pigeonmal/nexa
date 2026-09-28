//! Core path directory helpers lower to direct, synchronous native calls.

use nexa_compiler::compile;
use nexa_ir::{Expr, Node, Type};

#[test]
fn path_directory_helpers_lower_to_native_calls() {
    let module = compile(
        r#"
        app Paths {
            body {
                Text(Path.documents())
                Text(Path.caches())
                Text(Path.temporary())
                Text(Path.appSupport())
            }
        }
        "#,
    )
    .expect("core path directory helpers should compile");

    let expected_names = ["documents", "caches", "temporary", "appSupport"];
    assert_eq!(module.body.len(), expected_names.len());
    for (node, expected_name) in module.body.iter().zip(expected_names) {
        let Node::Text { value, .. } = node else {
            panic!("expected Text node");
        };
        let Expr::NativeCall {
            receiver,
            namespace,
            name,
            arguments,
            return_type,
            is_async,
            is_throwing,
            ..
        } = value
        else {
            panic!("expected a statically typed native path call, found {value:?}");
        };

        assert!(receiver.is_none());
        assert_eq!(namespace, "Path");
        assert_eq!(name, expected_name);
        assert!(arguments.is_empty());
        assert_eq!(return_type, &Type::String);
        assert!(!is_async);
        assert!(!is_throwing);
    }
}
