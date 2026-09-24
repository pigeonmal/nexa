use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

static TEMP_PROJECT_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_project() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "nexa-dev-runtime-command-{}-{nonce}-{}",
        std::process::id(),
        TEMP_PROJECT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["create", "RuntimeSmoke", "--directory"])
        .arg(&root)
        .output()
        .expect("run create");
    assert!(output.status.success(), "{output:?}");
    root
}

#[test]
fn dev_help_describes_debug_runtime_compile_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["dev", "--help"])
        .output()
        .expect("run dev help");

    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--compile-only"), "{help}");
    assert!(help.contains("--once"), "{help}");
    assert!(help.contains("`r` hot reloads"), "{help}");
    assert!(help.contains("`Shift+R` hot restarts"), "{help}");
    assert!(help.contains("`b` rebuilds and relaunches"), "{help}");
    assert!(
        help.contains("`p` toggles the performance overlay"),
        "{help}"
    );
}

#[test]
fn dev_rejects_conflicting_once_and_compile_only_modes() {
    let root = temporary_project();
    let output = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["dev", "--once", "--compile-only"])
        .current_dir(&root)
        .output()
        .expect("run conflicting dev options");

    assert!(!output.status.success(), "conflicting modes should fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("choose only one"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(root).expect("remove temporary project");
}

#[test]
fn development_runtime_is_debug_only_and_removed_by_aot_regeneration() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    let output = root.join("build");

    nexa_cli::generate_project(&entry, "all", &output, "RuntimeSmoke").expect("generate AOT hosts");
    let ios_app = output.join("ios/RuntimeSmoke/RuntimeSmokeApp.swift");
    let android_root =
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/MainActivity.kt");
    assert!(ios_app.is_file());
    assert!(
        !fs::read_to_string(&ios_app)
            .expect("read AOT iOS app entry")
            .contains("NexaDevRuntime")
    );
    assert!(
        !output
            .join("ios/RuntimeSmoke/NexaDevRuntime.swift")
            .exists()
    );
    assert!(
        !output
            .join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevRuntime.kt")
            .exists()
    );

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate dev hosts");
    assert!(
        output
            .join("ios/RuntimeSmoke/NexaDevRuntime.swift")
            .is_file()
    );
    assert!(
        output
            .join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevRuntime.kt")
            .is_file()
    );
    assert!(
        fs::read_to_string(&ios_app)
            .expect("read iOS app entry")
            .contains("NexaDevRuntimeRoot")
    );
    assert!(
        fs::read_to_string(&android_root)
            .expect("read Android activity")
            .contains("NexaDevRuntimeRoot")
    );

    nexa_cli::generate_project(&entry, "all", &output, "RuntimeSmoke")
        .expect("regenerate AOT hosts");
    assert!(
        !output
            .join("ios/RuntimeSmoke/NexaDevRuntime.swift")
            .exists()
    );
    assert!(
        !output
            .join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevRuntime.kt")
            .exists()
    );
    assert!(
        !fs::read_to_string(ios_app)
            .expect("read regenerated iOS app entry")
            .contains("NexaDevRuntime")
    );

    fs::remove_dir_all(root).expect("remove temporary project");
}

#[test]
fn development_navigation_host_keeps_platform_navigation_state_outside_the_tree() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(
        &entry,
        include_str!("../../../examples/navigation.nx").replace("NavigationDemo", "RuntimeSmoke"),
    )
    .expect("write navigation app source");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate navigation dev hosts");

    let swift = fs::read_to_string(output.join("ios/RuntimeSmoke/NexaDevRuntime.swift"))
        .expect("read Swift DevRuntime");
    assert!(swift.contains("NavigationStack(path: $store.navigationPath)"));
    assert!(swift.contains("routeDestination(route)"));
    assert!(swift.contains("screen/\\(name)"));

    let kotlin_path =
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevRuntime.kt");
    let kotlin = fs::read_to_string(kotlin_path).expect("read Kotlin DevRuntime");
    assert!(kotlin.contains("rememberNavController()"));
    assert!(kotlin.contains("NavHost(navController = navController"));
    assert!(kotlin.contains("navigationEpoch"));
    assert!(
        fs::read_to_string(output.join("android/app/build.gradle.kts"))
            .expect("read Android navigation dependencies")
            .contains("androidx.navigation:navigation-compose")
    );

    fs::remove_dir_all(root).expect("remove temporary project");
}

#[test]
fn development_runtime_restores_text_input_focus_after_compatible_module_replacement() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(&entry, include_str!("fixtures/dev_focus.nx")).expect("write focus smoke app");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate text input dev hosts");

    let swift = fs::read_to_string(output.join("ios/RuntimeSmoke/NexaDevRuntime.swift"))
        .expect("read Swift DevRuntime");
    assert!(swift.contains("@FocusState private var activeInput: String?"));
    assert!(swift.contains(".focused(focusedField, equals: identity)"));
    assert!(swift.contains("focusedFieldKey"));
    assert!(swift.contains("store.hotRestart(module: module)"));
    assert!(swift.contains("func hotRestart(module: [String: Any])"));

    let kotlin_path =
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevRuntime.kt");
    let kotlin = fs::read_to_string(kotlin_path).expect("read Kotlin DevRuntime");
    assert!(kotlin.contains("FocusRequester()"));
    assert!(kotlin.contains("store.focusedFieldKey == focusKey"));
    assert!(kotlin.contains("store.focusChanged(focusKey)"));
    assert!(kotlin.contains("store.hotRestart()"));
    assert!(kotlin.contains("fun hotRestart()"));
    assert!(
        fs::read_to_string(output.join("android/app/build.gradle.kts"))
            .expect("read Android dev dependencies")
            .contains("androidx.navigation:navigation-compose"),
        "debug runtime navigation dependency should be available even when the app has no navigation"
    );

    fs::remove_dir_all(root).expect("remove temporary project");
}

#[test]
fn development_runtime_renders_bottom_sheets_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dev_bottom_sheet.nx"),
        &entry,
    )
    .expect("write bottom-sheet smoke app");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate bottom-sheet dev hosts");

    let swift = fs::read_to_string(output.join("ios/RuntimeSmoke/NexaDevRuntime.swift"))
        .expect("read Swift DevRuntime");
    assert!(swift.contains("case \"BottomSheet\":"));
    assert!(swift.contains(".sheet(isPresented: Binding("));
    assert!(swift.contains("presentationDetents(sheet.partial ? [.medium, .large] : [.large])"));

    let kotlin_path =
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevRuntime.kt");
    let kotlin = fs::read_to_string(kotlin_path).expect("read Kotlin DevRuntime");
    assert!(kotlin.contains("\"BottomSheet\" ->"));
    assert!(kotlin.contains("ModalBottomSheet(onDismissRequest"));

    fs::remove_dir_all(root).expect("remove temporary project");
}

#[test]
fn development_runtime_renders_bottom_tabs_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dev_bottom_bar.nx"),
        &entry,
    )
    .expect("write bottom-bar smoke app");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate bottom-bar dev hosts");

    let swift = fs::read_to_string(output.join("ios/RuntimeSmoke/NexaDevRuntime.swift"))
        .expect("read Swift DevRuntime");
    assert!(swift.contains("case \"AppBottomBar\":"));
    assert!(swift.contains("TabView(selection: Binding("));

    let kotlin_path =
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevRuntime.kt");
    let kotlin = fs::read_to_string(kotlin_path).expect("read Kotlin DevRuntime");
    assert!(kotlin.contains("\"AppBottomBar\" ->"));
    assert!(kotlin.contains("NavigationBarItem("));

    fs::remove_dir_all(root).expect("remove temporary project");
}
