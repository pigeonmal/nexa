use std::{fs, path::Path, process::Command};

fn temporary_project() -> nexa_testkit::VacantDir {
    // `nexa create` refuses to scaffold into a directory that already exists,
    // so the path has to be vacant. The name is still claimed, and the test
    // removes the project when it finishes.
    let root = nexa_testkit::VacantDir::new("nexa-dev-runtime-command");
    let output = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["create", "RuntimeSmoke", "--directory"])
        .arg(root.path())
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
}

const IOS_DEV_RUNTIME_MODULES: &[&str] = &[
    "NexaDevValueCodec.swift",
    "NexaDevSchema.swift",
    "NexaDevProtocol.swift",
    "NexaDevState.swift",
    "NexaDevActions.swift",
    "NexaDevNavigation.swift",
    "NexaDevNativeApis.swift",
    "NexaDevOverlay.swift",
    "NexaDevRenderer.swift",
    "NexaDevRuntime.swift",
];

const ANDROID_DEV_RUNTIME_MODULES: &[&str] = &[
    "NexaDevValueCodec.kt",
    "NexaDevSchema.kt",
    "NexaDevProtocol.kt",
    "NexaDevState.kt",
    "NexaDevActions.kt",
    "NexaDevNavigation.kt",
    "NexaDevNativeApis.kt",
    "NexaDevOverlay.kt",
    "NexaDevRenderer.kt",
    "NexaDevRuntime.kt",
];

fn read_ios_dev_runtime(output: &Path) -> String {
    let base = output.join("ios/RuntimeSmoke");
    IOS_DEV_RUNTIME_MODULES
        .iter()
        .map(|name| {
            let path = base.join(name);
            fs::read_to_string(&path).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_android_dev_runtime(output: &Path) -> String {
    let base = output.join("android/app/src/main/java/dev/nexa/runtimesmoke");
    ANDROID_DEV_RUNTIME_MODULES
        .iter()
        .map(|name| {
            let path = base.join(name);
            fs::read_to_string(&path).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn development_runtime_is_debug_only_and_removed_by_aot_regeneration() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    let output = root.join("build");

    nexa_cli::generate_project(&entry, "all", &output, "RuntimeSmoke").expect("generate AOT hosts");
    let ios_app = output.join("ios/RuntimeSmoke/RuntimeSmokeApp.swift");
    let ios_plugin_bridge = output.join("ios/RuntimeSmoke/NexaDevPluginBridge.swift");
    let android_root =
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/MainActivity.kt");
    let android_plugin_bridge =
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevPluginBridge.kt");
    assert!(ios_app.is_file());
    assert!(
        !fs::read_to_string(&ios_app)
            .expect("read AOT iOS app entry")
            .contains("NexaDevRuntime")
    );
    for file in IOS_DEV_RUNTIME_MODULES {
        assert!(
            !output.join("ios/RuntimeSmoke").join(file).exists(),
            "AOT build should not have {file}"
        );
    }
    for file in ANDROID_DEV_RUNTIME_MODULES {
        assert!(
            !output
                .join("android/app/src/main/java/dev/nexa/runtimesmoke")
                .join(file)
                .exists(),
            "AOT build should not have {file}"
        );
    }

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate dev hosts");
    for file in IOS_DEV_RUNTIME_MODULES {
        assert!(
            output.join("ios/RuntimeSmoke").join(file).is_file(),
            "Dev build should produce {file}"
        );
    }
    for file in ANDROID_DEV_RUNTIME_MODULES {
        assert!(
            output
                .join("android/app/src/main/java/dev/nexa/runtimesmoke")
                .join(file)
                .is_file(),
            "Dev build should produce {file}"
        );
    }
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
    assert!(
        ios_plugin_bridge.is_file(),
        "Dev build should generate its iOS plugin bridge"
    );
    assert!(
        android_plugin_bridge.is_file(),
        "Dev build should generate its Android plugin bridge"
    );

    nexa_cli::generate_project(&entry, "all", &output, "RuntimeSmoke")
        .expect("regenerate AOT hosts");
    for file in IOS_DEV_RUNTIME_MODULES {
        assert!(
            !output.join("ios/RuntimeSmoke").join(file).exists(),
            "Regenerated AOT build should remove {file}"
        );
    }
    for file in ANDROID_DEV_RUNTIME_MODULES {
        assert!(
            !output
                .join("android/app/src/main/java/dev/nexa/runtimesmoke")
                .join(file)
                .exists(),
            "Regenerated AOT build should remove {file}"
        );
    }
    assert!(
        !fs::read_to_string(ios_app)
            .expect("read regenerated iOS app entry")
            .contains("NexaDevRuntime")
    );
    assert!(
        !ios_plugin_bridge.exists(),
        "AOT regeneration should remove the iOS Dev plugin bridge"
    );
    assert!(
        !android_plugin_bridge.exists(),
        "AOT regeneration should remove the Android Dev plugin bridge"
    );
}

#[test]
fn development_runtime_generates_typed_bridges_for_declared_plugin_services() {
    let project = nexa_testkit::TestProject::new("nexa-dev-plugin-bridge");
    let plugin = nexa_testkit::example_path("plugins/fast-math");
    let plugin_path = plugin
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    project.write(
        "nexa.config.nx",
        format!(
            "config {{ dependencies {{ FastMath {{ id: \"dev.nexa.fast-math\", path: \"{plugin_path}\" }} }} permissions {{}} }}\n"
        ),
    );
    project.write(
        "App.nx",
        "plugin \"dev.nexa.fast-math\" as FastMath\napp PluginBridge { state result: Int64 = 0 state sum: Int32 = 0 body { OnAppear async { result = await FastMath.heavyCalculation(7) } Button(\"Compute\") { sum = FastMath.add(10, 20) } } }\n",
    );

    let entry = project.path().join("App.nx");
    let output = project.path().join("build");
    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "PluginBridge",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts with a local service plugin");

    let ios_bridge = fs::read_to_string(output.join("ios/PluginBridge/NexaDevPluginBridge.swift"))
        .expect("read generated Swift service bridge");
    let android_bridge = fs::read_to_string(
        output.join("android/app/src/main/java/com/nexa/pluginbridge/NexaDevPluginBridge.kt"),
    )
    .expect("read generated Kotlin service bridge");
    let ios_native_apis =
        fs::read_to_string(output.join("ios/PluginBridge/NexaDevNativeApis.swift"))
            .expect("read iOS native API adapter");
    let android_native_apis = fs::read_to_string(
        output.join("android/app/src/main/java/com/nexa/pluginbridge/NexaDevNativeApis.kt"),
    )
    .expect("read Android native API adapter");
    assert!(ios_bridge.contains("FastMathPlugin.shared.add(nexaArg0, nexaArg1)"));
    assert!(ios_bridge.contains("await FastMathPlugin.shared.heavyCalculation(nexaArg0)"));
    assert!(android_bridge.contains("FastMathPlugin.instance.add(nexaArg0, nexaArg1)"));
    assert!(android_bridge.contains("FastMathPlugin.instance.heavyCalculation(nexaArg0)"));
    assert!(ios_native_apis.contains("NexaDevPluginBridge.invokeSync"));
    assert!(android_native_apis.contains("NexaDevPluginBridge.invokeSync"));
}

#[test]
fn development_runtime_generates_video_plugin_class_event_and_component_bridges() {
    let project = nexa_testkit::TestProject::new("nexa-dev-video-plugin-bridge");
    let entry = nexa_testkit::example_path("plugins/video-player-demo.nx");
    let output = project.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "VideoPlayerDemo",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts with the configured native video plugin");

    let ios_bridge =
        fs::read_to_string(output.join("ios/VideoPlayerDemo/NexaDevPluginBridge.swift"))
            .expect("read generated Swift plugin bridge");
    let android_bridge_path = nexa_testkit::TestProject::collect_sources_in(
        &output.join("android/app/src/main/java"),
        "kt",
    )
    .into_iter()
    .find(|path| {
        path.file_name()
            .is_some_and(|name| name == "NexaDevPluginBridge.kt")
    })
    .expect("generated Kotlin plugin bridge path");
    let android_bridge =
        fs::read_to_string(android_bridge_path).expect("read generated Kotlin plugin bridge");

    for marker in [
        "VideoPlayer()",
        "nexaReceiver.prepare(nexaArg0)",
        "nexaReceiver.volume = nexaValue",
        "static func subscribeInstanceEvent",
        "nexaReceiver.onEnded =",
        "case (\"VideoPlayer\", \"VideoView\")",
        "VideoView(player: nexaArg_player, controls: nexaArg_controls",
        "events[\"onTapped\"]",
    ] {
        assert!(
            ios_bridge.contains(marker),
            "Swift Dev bridge is missing {marker}"
        );
    }
    for marker in [
        "VideoPlayer()",
        "nexaReceiver.prepare(nexaArg0)",
        "receiver.volume = nexaValue",
        "fun subscribeInstanceEvent",
        "receiver.onEnded =",
        "\"VideoPlayer.VideoView\" ->",
        "VideoView(player = nexaArg_player, controls = nexaArg_controls",
        "events[\"onTapped\"]",
    ] {
        assert!(
            android_bridge.contains(marker),
            "Kotlin Dev bridge is missing {marker}"
        );
    }
}

#[test]
fn android_cronet_config_selects_provider_and_cache_for_dev_and_aot_hosts() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/dev_network_fetch.nx"),
        &entry,
    )
    .expect("write network app source");
    fs::write(
        root.join("nexa.config.nx"),
        r#"config { android { cronet { provider: "embedded", diskCacheSizeMb: 32 } } }"#,
    )
    .expect("write Android Cronet config");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "android",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate embedded Cronet Dev host");

    let source_directory = output.join("android/app/src/main/java/com/nexa/runtimesmoke");
    let activity = fs::read_to_string(source_directory.join("MainActivity.kt"))
        .expect("read embedded Cronet Dev activity");
    assert!(!activity.contains("CronetProviderInstaller"));
    let gradle = fs::read_to_string(output.join("android/app/build.gradle.kts"))
        .expect("read embedded Cronet Dev dependencies");
    assert!(gradle.contains("org.chromium.net:cronet-embedded:143.7445.0"));
    assert!(!gradle.contains("com.google.android.gms:play-services-cronet"));
    let cronet_config = fs::read_to_string(source_directory.join("NexaCronetConfig.kt"))
        .expect("read generated Cronet Dev config");
    assert!(cronet_config.contains("DISK_CACHE_SIZE_BYTES: Long = 32L * 1024L * 1024L"));
    let native_library =
        fs::read_to_string(source_directory.join("NexaGenerated_native_library.kt"))
            .expect("read generated Cronet Dev adapter");
    assert!(native_library.contains("NexaCronetConfig.DISK_CACHE_SIZE_BYTES"));
    assert!(native_library.contains("CronetEngine.Builder.HTTP_CACHE_DISABLED"));

    nexa_cli::generate_project(&entry, "android", &output, "RuntimeSmoke")
        .expect("generate embedded Cronet AOT host");
    let aot_gradle = fs::read_to_string(output.join("android/app/build.gradle.kts"))
        .expect("read embedded Cronet AOT dependencies");
    assert!(aot_gradle.contains("org.chromium.net:cronet-embedded:143.7445.0"));
    assert!(!aot_gradle.contains("com.google.android.gms:play-services-cronet"));
    let aot_config = fs::read_to_string(source_directory.join("NexaCronetConfig.kt"))
        .expect("read generated Cronet AOT config");
    assert!(aot_config.contains("DISK_CACHE_SIZE_BYTES: Long = 32L * 1024L * 1024L"));
}

#[test]
fn aot_plugin_hosts_include_value_runtime_when_unused_generic_methods_exist() {
    let project = nexa_testkit::TestProject::new("nexa-aot-plugin-value-runtime");
    project.write(
        "plugins/codec-probe/native.nxid",
        "service CodecProbe {\n    fn echo<T>(value: T) -> T\n    fn ready() -> Bool\n}\n",
    );
    project.write(
        "plugins/codec-probe/plugin.config.nx",
        "plugin { schema: 2 id: \"dev.nexa.codec-probe\" version: \"0.1.0\" sources { native: \"native.nxid\" } android { sources: [\"android/src/main/kotlin/**/*.kt\"] } }\n",
    );
    project.write(
        "plugins/codec-probe/android/src/main/kotlin/dev/nexa/codeprobe/CodecProbeImpl.kt",
        "package dev.nexa.codeprobe\n",
    );
    let plugin_path = project
        .path()
        .join("plugins/codec-probe")
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    project.write(
        "nexa.config.nx",
        format!(
            "config {{ dependencies {{ CodecProbe {{ id: \"dev.nexa.codec-probe\", path: \"{plugin_path}\" }} }} permissions {{}} }}\n"
        ),
    );
    project.write(
        "App.nx",
        "plugin \"dev.nexa.codec-probe\" as CodecProbe\napp CodecRuntimeSmoke { state ready: Bool = false body { OnAppear { ready = CodecProbe.ready() } Text(\"ready\") } }\n",
    );

    let entry = project.path().join("App.nx");
    let output = project.path().join("build");
    nexa_cli::generate_project(&entry, "all", &output, "CodecRuntimeSmoke")
        .expect("generate AOT hosts with a generic plugin method");

    let ios_types =
        fs::read_to_string(output.join("ios/CodecRuntimeSmoke/NexaGenerated_types.swift"))
            .expect("read generated Swift type unit");
    assert!(ios_types.contains("public final class NexaValueWriter"));
    assert!(ios_types.contains("public final class NexaValueReader"));
    assert!(
        output
            .join("android/app/src/main/java/dev/nexa/core/NexaValue.kt")
            .is_file(),
        "AOT Android plugin hosts need the codec runtime even before a generic method is called"
    );
}

#[test]
fn development_remote_images_use_the_release_coil_and_cronet_pipeline() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Android dev host");

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("AsyncImage("));
    assert!(kotlin.contains("imageLoader = nexaImageLoader()"));
    assert!(!kotlin.contains("URL(url).openConnection()"));
    assert!(kotlin.contains("NexaFile.readText(stringOption(\"path\"))"));
    assert!(kotlin.contains("NexaPath.temporary(context)"));
    assert!(kotlin.contains("invokeNativeSync("));
    assert!(kotlin.contains("NexaPermissions.status(context, permission).name"));
    assert!(kotlin.contains("val currentContext = LocalContext.current"));
    assert!(kotlin.contains("NexaRuntime.bind(currentContext)"));
    assert!(kotlin.contains("NexaRuntime.bindPermissionLauncher(permissionLauncher)"));

    let generated_android =
        fs::read_to_string(output.join(
            "android/app/src/main/java/dev/nexa/runtimesmoke/NexaGenerated_native_library.kt",
        ))
        .expect("read generated Android dev APIs");
    assert!(generated_android.contains("public object NexaNetwork"));
    assert!(generated_android.contains("object NexaCronetRuntime"));
    assert!(generated_android.contains(".setStoragePath(cacheDirectory.absolutePath)"));
    assert!(generated_android.contains("public object NexaPath"));
    assert!(generated_android.contains("public object NexaFile"));
    assert!(generated_android.contains("public suspend fun readText(path: String)"));
    let generated_android_permissions = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaGenerated_permissions.kt"),
    )
    .expect("read generated Android permission APIs");
    assert!(generated_android_permissions.contains("public enum class NexaPermission"));
    assert!(generated_android_permissions.contains("public object NexaPermissions"));

    let generated_ios =
        fs::read_to_string(output.join("ios/RuntimeSmoke/NexaGenerated_native_library.swift"))
            .expect("read generated iOS dev APIs");
    assert!(generated_ios.contains("public enum NexaNetwork"));
    assert!(generated_ios.contains("enum NexaURLSessionSupport"));
    assert!(generated_ios.contains("public enum NexaPath"));
    assert!(generated_ios.contains("public enum NexaFile"));
    assert!(
        generated_ios
            .contains("public static func readText(_ path: String) async throws -> String")
    );
    let swift_runtime = read_ios_dev_runtime(&output);
    assert!(swift_runtime.contains("try await NexaFile.readText(stringOption(\"path\"))"));
    assert!(swift_runtime.contains("func invokeNativeSync("));
    assert!(swift_runtime.contains("NexaPermissions.status(permission)"));
    let generated_ios_permissions =
        fs::read_to_string(output.join("ios/RuntimeSmoke/NexaGenerated_permissions.swift"))
            .expect("read generated iOS permission APIs");
    assert!(generated_ios_permissions.contains("public enum NexaPermission"));
    assert!(generated_ios_permissions.contains("public enum NexaPermissions"));

    let activity = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/MainActivity.kt"),
    )
    .expect("read Android dev activity");
    assert!(activity.contains("CronetProviderInstaller.installProvider(this)"));
    assert!(
        activity.contains("Play Services Cronet provider is unavailable; network calls may fail")
    );
    assert!(activity.contains("setContent { MaterialTheme { NexaDevRuntimeRoot"));
    assert!(!activity.contains("Network provider unavailable"));

    let gradle = fs::read_to_string(output.join("android/app/build.gradle.kts"))
        .expect("read Android dev dependencies");
    assert!(gradle.contains("io.coil-kt.coil3:coil-compose"));
    assert!(gradle.contains("io.coil-kt.coil3:coil-network-core"));
    assert!(gradle.contains("com.google.android.gms:play-services-cronet"));
    assert!(!gradle.contains("org.chromium.net:cronet-embedded"));
    let cronet_config = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaCronetConfig.kt"),
    )
    .expect("read Android Cronet host configuration");
    assert!(cronet_config.contains("DISK_CACHE_SIZE_BYTES: Long = 64L * 1024L * 1024L"));

    nexa_cli::generate_project(&entry, "all", &output, "RuntimeSmoke")
        .expect("regenerate the AOT host");
    let release_gradle = fs::read_to_string(output.join("android/app/build.gradle.kts"))
        .expect("read regenerated AOT dependencies");
    assert!(!release_gradle.contains("io.coil-kt.coil3:coil-compose"));
    assert!(!release_gradle.contains("com.google.android.gms:play-services-cronet"));
    assert!(!release_gradle.contains("org.chromium.net:cronet-embedded"));
    assert!(
        !output
            .join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaGenerated_native_library.kt")
            .exists()
    );
    assert!(
        !output
            .join("ios/RuntimeSmoke/NexaGenerated_native_library.swift")
            .exists()
    );
}

#[test]
fn development_async_network_calls_use_release_native_adapters() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/dev_network_fetch.nx"),
        &entry,
    )
    .expect("write async network smoke app");
    let output = root.join("build");

    for target in [nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin] {
        nexa_compiler::compile_file_for_target(&entry, target)
            .expect("async network smoke app should compile");
    }

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate cross-platform network dev hosts");

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("case \"NativeCall\":"));
    assert!(swift.contains("NexaNetwork.fetch("));
    assert!(swift.contains("tagged[\"TryCatch\"] as? [String: Any]"));
    let generated_swift =
        fs::read_to_string(output.join("ios/RuntimeSmoke/NexaGenerated_native_library.swift"))
            .expect("read generated Swift networking adapter");
    assert!(generated_swift.contains("public enum NexaNetwork"));

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("\"NativeCall\" -> invokeNativeAsync"));
    assert!(kotlin.contains("NexaNetwork.fetch("));
    assert!(kotlin.contains("action.optJSONObject(\"TryCatch\")"));
    let generated_kotlin =
        fs::read_to_string(output.join(
            "android/app/src/main/java/dev/nexa/runtimesmoke/NexaGenerated_native_library.kt",
        ))
        .expect("read generated Kotlin networking adapter");
    assert!(generated_kotlin.contains("public object NexaNetwork"));
    let android_screen = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaGenerated.kt"),
    )
    .expect("read generated Android screen");
    assert!(android_screen.contains("import androidx.compose.ui.platform.LocalContext"));
}

#[test]
fn development_module_preserves_app_lifecycle_callbacks_for_the_native_runtime() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/dev_lifecycle.nx");
    let module = nexa_compiler::compile_file_for_target(&fixture, nexa_compiler::Target::Kotlin)
        .expect("compile lifecycle fixture");
    assert!(module.on_appear.is_some());
    assert!(module.on_disappear.is_none());
    assert!(module.on_active.is_some());
    assert!(module.on_inactive.is_some());
    assert!(module.on_background.is_some());

    let payload = serde_json::to_value(nexa_dev_ir::lower(&module, "lifecycle-test"))
        .expect("serialize lifecycle dev module");
    let module = &payload["module"];
    for callback in ["on_appear", "on_active", "on_inactive", "on_background"] {
        assert_eq!(
            module[callback].as_array().map(Vec::len),
            Some(1),
            "serialized dev module should carry {callback} actions"
        );
    }
    let detail = module["screens"]
        .as_array()
        .expect("serialized screens")
        .iter()
        .find(|screen| screen["name"] == "Detail")
        .expect("Detail screen");
    for callback in ["on_appear", "on_disappear"] {
        assert_eq!(
            detail[callback].as_array().map(Vec::len),
            Some(1),
            "serialized Detail screen should carry {callback} actions"
        );
    }
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

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("NavigationStack(path: $store.navigationPath)"));
    assert!(swift.contains("routeDestination(route)"));
    assert!(swift.contains("screen/\\(name)"));

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("rememberNavController()"));
    assert!(kotlin.contains("NavHost(navController = navController"));
    assert!(kotlin.contains("navigationEpoch"));
    assert!(
        fs::read_to_string(output.join("android/app/build.gradle.kts"))
            .expect("read Android navigation dependencies")
            .contains("androidx.navigation:navigation-compose")
    );
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

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("@FocusState private var activeInput: String?"));
    assert!(swift.contains(".focused(focusedField, equals: identity)"));
    assert!(swift.contains("focusedFieldKey"));
    assert!(swift.contains("store.hotRestart(module: module)"));
    assert!(swift.contains("func hotRestart(module: [String: Any])"));

    let kotlin = read_android_dev_runtime(&output);
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

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("case \"BottomSheet\":"));
    assert!(swift.contains(".sheet(isPresented: Binding("));
    assert!(swift.contains("presentationDetents(sheet.partial ? [.medium, .large] : [.large])"));

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("\"BottomSheet\" ->"));
    assert!(kotlin.contains("ModalBottomSheet(onDismissRequest"));
}

#[test]
fn development_runtime_renders_dialogs_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dev_dialog.nx"),
        &entry,
    )
    .expect("write dialog smoke app");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate dialog dev hosts");

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("case \"Dialog\":"));
    assert!(swift.contains(".alert(Text(title), isPresented: Binding("));

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("\"Dialog\" ->"));
    assert!(kotlin.contains("AlertDialog("));
    assert!(kotlin.contains("onDismissRequest = { store.setState(state, false, scope) }"));
}

#[test]
fn development_runtime_renders_segmented_controls_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dev_segmented_control.nx"),
        &entry,
    )
    .expect("write segmented-control smoke app");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate segmented-control dev hosts");

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("case \"SegmentedControl\":"));
    assert!(swift.contains(".pickerStyle(.segmented)"));

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("\"SegmentedControl\" ->"));
    assert!(kotlin.contains("SingleChoiceSegmentedButtonRow"));
    assert!(kotlin.contains("SegmentedButtonDefaults.itemShape"));
}

#[test]
fn development_runtime_renders_pickers_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dev_picker.nx"),
        &entry,
    )
    .expect("write picker smoke app");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate picker dev hosts");

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("case \"Picker\":"));
    assert!(swift.contains(".pickerStyle(.menu)"));

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("\"Picker\" ->"));
    assert!(kotlin.contains("DropdownMenuItem("));
    assert!(kotlin.contains("store.setState(state, item, scope)"));
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

    let swift = read_ios_dev_runtime(&output);
    assert!(swift.contains("case \"AppBottomBar\":"));
    assert!(swift.contains("TabView(selection: Binding("));

    let kotlin = read_android_dev_runtime(&output);
    assert!(kotlin.contains("\"AppBottomBar\" ->"));
    assert!(kotlin.contains("NavigationBarItem("));
}
