use std::fs;

use nexa_compiler::{
    Target, compile_file_with_warnings_for_target, compile_with_warnings, testing::run_tests,
};
use nexa_testkit::TestProject;

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
fn evaluates_typed_collection_expressions_in_headless_tests() {
    let compilation = compile_with_warnings(
        r#"
app CollectionExpressions {
    body { Text("Collections") }
}

test "typed collection expressions" {
    let values: Array<Int32> = [1, 2, 3]
    let doubled: Array<Int32> = values.map { value -> value * 2 }
    let evens: Array<Int32> = doubled.filter { value -> value % 2 == 0 }
    let total: Int32 = evens.reduce(0) { sum, value -> sum + value }
    let ordered: Array<Int32> = values.sortedBy { value -> 0 - value }
    assert(values.count == 3)
    assert(2 in values)
    assert(doubled[1] == 4)
    assert(total == 12)
    assert(ordered[0] == 3)
    assert(ordered[2] == 1)
}
"#,
    )
    .expect("compile a headless collection expression test");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
}

#[test]
fn evaluates_case_insensitive_string_membership_in_headless_tests() {
    let compilation = compile_with_warnings(
        r#"
app Search {
    body { Text("Search") }
}

test "substring membership ignores case" {
    assert("meeting" in "Team Meeting")
    assert("MEETING" in "Team Meeting")
    assert(!("dentist" in "Team Meeting"))
}
"#,
    )
    .expect("compile case-insensitive substring assertions");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
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
fn headless_component_tests_clear_map_state() {
    let compilation = compile_with_warnings(
        r#"
component Cart() {
    state quantities: Map<String, Int32> = ["sku-441": 2, "sku-807": 1]
    body {
        Text(quantities.count)
        Button("Clear cart") { quantities.clear() }
    }
}

app CartDemo {
    body { Cart() }
}

test "clear map" for Cart() {
    assert(quantities.count == 2)
    assert(quantities.keys.count == 2)
    assert(quantities.values.count == 2)
    assert((quantities.get("sku-441") ?? 0) == 2)
    assert(quantities.contains("sku-807"))
    tap("Clear cart")
    assert(quantities.isEmpty)
}
"#,
    )
    .expect("compile a headless map clear behavior test");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
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

#[test]
fn headless_component_tests_cover_forms_and_platform_backed_controls() {
    let compilation = compile_with_warnings(
        r#"
component Preferences() {
    state name: String = ""
    state submitted: String = ""
    state enabled: Bool = false
    state volume: Float64 = 0.0
    state choice: String = "First"
    body {
        TextInput(value: name, placeholder: "Name") {
            submitted = name
        }
        Switch(value: enabled, label: "Enabled")
        Slider(value: volume, min: 0.0, max: 1.0, step: 0.25)
        SegmentedControl(items: ["First", "Second"], selected: choice)
        Text("Name: $name")
        Text("Submitted: $submitted")
        Text(enabled ? "Enabled" : "Disabled")
        Text("Volume: $volume")
        Text("Choice: $choice")
    }
}

app PreferencesDemo {
    body { Preferences() }
}

test "form and control interactions" for Preferences() {
    typeText("Name", "Ada")
    submit("Name")
    toggle("Enabled")
    slide("volume", 0.74)
    select("choice", "Second")
    assert(name == "Ada")
    assert(submitted == "Ada")
    assert(enabled)
    assert(volume == 0.75)
    assert(choice == "Second")
    assertText("Name: Ada")
    assertText("Submitted: Ada")
    assertText("Enabled")
    assertText("Volume: 0.75")
    assertText("Choice: Second")
}
"#,
    )
    .expect("compile headless form and control interactions");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
}

#[test]
fn headless_component_tests_execute_loops_and_collection_mutations() {
    let compilation = compile_with_warnings(
        r#"
component CollectionEditor() {
    state values: Array<Int32> = [1, 2, 3]
    state total: Int32 = 0
    state steps: Int32 = 0
    body {
        Text("Total: $total")
        Text("Steps: $steps")
        Text("Items: $values")
        Button("Process") {
            for value in values {
                total += value
            }
            values.append(4)
            values.remove(0)
            values.move(2, 0)
            values.move(99, 0)
            values.move(-1, 0)
            while steps < 2 {
                steps += 1
            }
        }
    }
}

app CollectionDemo {
    body { CollectionEditor() }
}

test "collection actions" for CollectionEditor() {
    tap("Process")
    assert(total == 6)
    assert(steps == 2)
    assert(values.count == 3)
    assert(values[0] == 4)
    assertText("Total: 6")
    assertText("Steps: 2")
    assertText("Items: [4, 2, 3]")
}
"#,
    )
    .expect("compile headless collection actions");

    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
}

#[test]
fn headless_component_tests_mount_and_emit_native_plugin_events() {
    let project = TestProject::new("nexa-test-native-component");
    let plugin = project.join("widget");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        "plugin { schema: 2 id: \"dev.test.widget\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
    )
    .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        "native component Widget { event completed() }\n",
    )
    .expect("native component contract should be written");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
plugin "widget" as Widget

component EventDemo() {
    state completed: Bool = false
    body {
        Widget.Widget().onCompleted {
            completed = true
        }
        if completed {
            Text("Completed")
        }
    }
}

app Demo {
    body { EventDemo() }
}

test "native component event" for EventDemo() {
    assertComponent("Widget.Widget")
    emit("Widget.Widget", "onCompleted")
    assert(completed)
    assertText("Completed")
}
"#,
    )
    .expect("app source should be written");

    let compilation = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("compile headless native component test");
    let report = run_tests(&compilation.tests);
    assert_eq!(report.passed, 1, "{:?}", report.failures);
    assert!(report.failures.is_empty());
}
