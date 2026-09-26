use nexa_testkit::{assert_output_success, nexa_command, TestProject};

#[test]
fn check_resolves_local_plugin_dependencies_without_native_toolchains() {
    let project = TestProject::new("nexa-check");
    let plugin = nexa_testkit::example_path("plugins/fast-math");
    let escaped_plugin = plugin
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    project.write(
        "nexa.config.nx",
        format!(
            "config {{ dependencies {{ FastMath {{ id: \"dev.nexa.fast-math\", path: \"{escaped_plugin}\" }} }} permissions {{}} }}\n"
        ),
    );
    project.write(
        "App.nx",
        "plugin \"dev.nexa.fast-math\" as FastMath\napp Demo { state result: Int32 = 0 body { Button(\"Compute\") { result = FastMath.add(10, 20) } } }\n",
    );

    let output = nexa_command()
        .args(["check", "--deny-warnings"])
        .current_dir(project.path())
        .output()
        .expect("run Nexa checker");
    assert_output_success(&output);
    let lock = project.read("nexa.lock");
    assert!(lock.contains("dev.nexa.fast-math"));
    assert!(lock.contains("path:"));
    assert!(!project.exists("build"));

    let locked = nexa_command()
        .args(["check", "--deny-warnings", "--locked"])
        .current_dir(project.path())
        .output()
        .expect("run Nexa checker with the lockfile enforced");
    assert_output_success(&locked);
}
