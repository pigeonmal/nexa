use nexa_testkit::{TestProject, assert_output_success, nexa_command};

#[test]
fn test_command_runs_headless_custom_component_behavior_without_a_native_host() {
    let project = TestProject::new("nexa-headless-component-test");
    project.write_app(
        r#"
component Counter() {
    state count: Int32 = 0
    body {
        Text("Count: $count")
        Button("Increment") { count += 1 }
        if count > 0 {
            Text("Updated")
        }
    }
}

app CounterDemo {
    body { Counter() }
}

test "increments the counter" for Counter() {
    assert(count == 0)
    assertText("Count: 0")
    tap("Increment")
    assert(count == 1)
    assertText("Count: 1")
    assertText("Updated")
}
"#,
    );

    let output = nexa_command()
        .args(["test", "--unit-only"])
        .current_dir(project.path())
        .output()
        .expect("run Nexa headless component test");
    assert_output_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS increments the counter"), "{stdout}");
    assert!(stdout.contains("1 passed; 0 failed"), "{stdout}");
}
