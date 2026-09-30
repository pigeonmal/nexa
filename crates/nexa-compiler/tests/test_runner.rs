use nexa_compiler::{compile_with_warnings, testing::run_tests};

#[test]
fn evaluates_named_nexa_test_blocks_through_typed_functions() {
    let compilation = compile_with_warnings(
        r#"
fn calculateTotal(price: Float64, qty: Float64) -> Float64 {
    return price * qty
}

app Cart {
    body { Text("Cart") }
}

test "cart item calculation" {
    let result = calculateTotal(price: 50.0, qty: 2)
    assert(result == 100.0)
}
"#,
    )
    .expect("compile app and typed test blocks");

    assert!(compilation.module.functions.is_empty());
    assert_eq!(compilation.tests.functions.len(), 1);
    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1);
    assert!(report.failures.is_empty());
}

#[test]
fn reports_assertion_messages_and_source_locations() {
    let compilation = compile_with_warnings(
        r#"
app Cart {
    body { Text("Cart") }
}

test "wrong total" {
    let actual: Int32 = 12
    assert(actual == 10, "expected ten")
}
"#,
    )
    .expect("compile failing test");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 0);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].message, "expected ten");
    assert_eq!(report.failures[0].name, "wrong total");
    assert_eq!(report.failures[0].span.line, 8);
}

#[test]
fn test_assertions_are_statically_type_checked() {
    let error = compile_with_warnings(
        r#"
app Cart {
    body { Text("Cart") }
}

test "invalid assertion" {
    assert("not a Boolean")
}
"#,
    )
    .err()
    .expect("assertion must be Boolean");

    assert!(error.message.contains("expected Bool"));
}
