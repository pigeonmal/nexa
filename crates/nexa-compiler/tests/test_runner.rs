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

#[test]
fn headless_component_tests_mount_tap_and_assert_state_and_text() {
    let compilation = compile_with_warnings(
        r#"
component Counter() {
    state count: Int32 = 0
    body {
        Text("Count: $count")
        Button("Increment") { count += 1 }
        if count > 0 {
            Text("Ready")
        }
    }
}

app CounterDemo {
    body { Counter() }
}

test "counter increments" for Counter() {
    assert(count == 0)
    assertText("Count: 0")
    assertText("Increment")
    tap("Increment")
    assert(count == 1)
    assertText("Count: 1")
    assertText("Ready")
}
"#,
    )
    .expect("compile a headless component behavior test");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
    assert_eq!(report.passed_names, ["counter increments"]);
}

#[test]
fn headless_component_tests_report_missing_buttons_and_text() {
    let compilation = compile_with_warnings(
        r#"
component Counter() {
    state count: Int32 = 0
    body {
        Text("Count: $count")
        Button("Increment") { count += 1 }
    }
}

app CounterDemo {
    body { Counter() }
}

test "missing button" for Counter() {
    tap("Reset")
    assert(count == 0)
}
"#,
    )
    .expect("compile a headless test that will fail at runtime");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 0);
    assert_eq!(report.failures.len(), 1);
    assert!(report.failures[0].message.contains("no visible Button"));
}

#[test]
fn headless_component_tests_bind_named_component_arguments() {
    let compilation = compile_with_warnings(
        r#"
component Greeting(name: String) {
    body { Text("Hello, $name") }
}

app GreetingDemo {
    body { Greeting(name: "App") }
}

test "greets the mounted value" for Greeting(name: "Nexa") {
    assert(name == "Nexa")
    assertText("Hello, Nexa")
}
"#,
    )
    .expect("compile a headless component test with a named argument");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
}

#[test]
fn headless_component_tests_keep_nested_component_state_across_taps() {
    let compilation = compile_with_warnings(
        r#"
component CounterControl() {
    state count: Int32 = 0
    body {
        Text("Nested count: $count")
        Pressable(disabled: false) { Text("Add one") }.onTap { count += 1 }
    }
}

component ContentFrame() {
    body { Column { Content() } }
}

component CounterScreen() {
    body { ContentFrame() { CounterControl() } }
}

app CounterDemo {
    body { CounterScreen() }
}

test "nested counter pressable" for CounterScreen() {
    assertText("Nested count: 0")
    tap("Add one")
    assertText("Nested count: 1")
}
"#,
    )
    .expect("compile a nested headless component test");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
}
