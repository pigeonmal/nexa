use nexa_testkit::{TestProject, assert_output_failure, assert_output_success, nexa_command};

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

fn setup_android_min_sdk_plugin(
    project: &TestProject,
    plugin_min_sdk: u32,
    app_min_sdk: Option<u32>,
) {
    let plugin_path = project.path().join("platform-api");
    project.write(
        "platform-api/plugin.config.nx",
        format!(
            "plugin {{ schema: 2 id: \"dev.nexa.platform-api\" version: \"0.1.0\" sources {{ native: \"native.nxid\" }} android {{ minSdk: {plugin_min_sdk} }} }}\n"
        ),
    );
    project.write(
        "platform-api/native.nxid",
        "service PlatformApi { fn value() -> Int32 }\n",
    );
    let escaped_plugin_path = plugin_path
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let android_config = app_min_sdk
        .map(|min_sdk| format!("android {{ minSdk: {min_sdk}, targetSdk: 36 }}"))
        .unwrap_or_default();
    project.write(
        "nexa.config.nx",
        format!(
            "config {{ dependencies {{ PlatformApi {{ id: \"dev.nexa.platform-api\", path: \"{escaped_plugin_path}\" }} }} {android_config} }}\n"
        ),
    );
    project.write(
        "App.nx",
        "plugin \"dev.nexa.platform-api\" as PlatformApi\napp Demo { state value: Int32 = 0 body { Button(\"Read\") { value = PlatformApi.value() } } }\n",
    );
}

#[test]
fn check_rejects_plugin_android_minimum_above_the_app_default() {
    let project = TestProject::new("nexa-check-plugin-min-sdk");
    setup_android_min_sdk_plugin(&project, 24, None);

    let android = nexa_command()
        .args(["check", "--android"])
        .current_dir(project.path())
        .output()
        .expect("run Android Nexa checker");
    assert_output_failure(
        &android,
        "Android plugin `PlatformApi` requires minSdk 24, but the app config sets android.minSdk to 23",
    );

    let ios = nexa_command()
        .args(["check", "--ios"])
        .current_dir(project.path())
        .output()
        .expect("run iOS Nexa checker");
    assert_output_success(&ios);
}

#[test]
fn check_accepts_plugin_minimum_below_the_configured_app_minimum() {
    let project = TestProject::new("nexa-check-plugin-min-sdk-compatible");
    setup_android_min_sdk_plugin(&project, 24, Some(26));

    let output = nexa_command()
        .args(["check", "--android"])
        .current_dir(project.path())
        .output()
        .expect("run Android Nexa checker");
    assert_output_success(&output);
}
