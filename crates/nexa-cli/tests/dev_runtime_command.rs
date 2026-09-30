use std::{collections::HashMap, fs, path::Path, process::Command};

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
fn optional_generic_compound_values_generate_aot_and_dev_codecs() {
    let project = nexa_testkit::TestProject::new("nexa-dev-optional-generic-codec");
    project.write(
        "plugins/codec-probe/native.nxid",
        "service CodecProbe {\n    fn store<T>(value: T) -> Bool\n    fn load<T>(key: String) -> T?\n}\n",
    );
    project.write(
        "plugins/codec-probe/plugin.config.nx",
        "plugin { schema: 2 id: \"dev.nexa.codec-probe\" version: \"0.1.0\" sources { native: \"native.nxid\" } ios { sources: [\"ios/CodecProbeImpl.swift\"] } android { sources: [\"android/src/main/kotlin/dev/nexa/codeprobe/CodecProbeImpl.kt\"] } }\n",
    );
    project.write(
        "plugins/codec-probe/ios/CodecProbeImpl.swift",
        "import Foundation\n",
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
        "plugin \"dev.nexa.codec-probe\" as CodecProbe\napp OptionalCodec { enum CodecError { Missing } state maybe: String? = null state values: Array<String?> = [\"one\", null] state outcome: Result<String?, CodecError> = Ok(if true { \"present\" } else { null }) body { Button(\"Save\") { CodecProbe.store<String?>(maybe)\n values = CodecProbe.load<Array<String?>>(\"values\") ?? []\n CodecProbe.store<Result<String?, CodecError>>(outcome)\n CodecProbe.load<Result<String?, CodecError>>(\"outcome\") } } }\n",
    );

    let entry = project.path().join("App.nx");
    let output = project.path().join("build");
    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "OptionalCodec",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate both Dev hosts for a nullable generic plugin argument");

    let swift =
        nexa_testkit::TestProject::collect_sources_in(&output.join("ios/OptionalCodec"), "swift")
            .into_iter()
            .map(|path| fs::read_to_string(path).expect("read generated Swift source"))
            .collect::<Vec<_>>()
            .join("\n");
    let kotlin = nexa_testkit::TestProject::collect_sources_in(
        &output.join("android/app/src/main/java"),
        "kt",
    )
    .into_iter()
    .map(|path| fs::read_to_string(path).expect("read generated Kotlin source"))
    .collect::<Vec<_>>()
    .join("\n");
    let swift_bridge =
        fs::read_to_string(output.join("ios/OptionalCodec/NexaDevPluginBridge.swift"))
            .expect("read generated Swift Dev plugin bridge");
    let kotlin_bridge_path = nexa_testkit::TestProject::collect_sources_in(
        &output.join("android/app/src/main/java"),
        "kt",
    )
    .into_iter()
    .find(|path| {
        path.file_name()
            .is_some_and(|name| name == "NexaDevPluginBridge.kt")
    })
    .expect("find generated Kotlin Dev plugin bridge");
    let kotlin_bridge =
        fs::read_to_string(kotlin_bridge_path).expect("read generated Kotlin Dev plugin bridge");

    assert!(swift.contains("func nexaWriteoptional_string(_ value: String?"));
    assert!(swift.contains("nexaWriteoptional_string(item, into: writer)"));
    assert!(swift.contains("func nexaReadoptional_string(_ reader: NexaValueReader) -> String??"));
    assert!(
        swift.contains(
            "func nexaReadarray_optional_string(_ reader: NexaValueReader) -> [String?]?"
        )
    );
    assert!(kotlin.contains("fun nexaWriteoptional_string(value: String?"));
    assert!(kotlin.contains("nexaWriteoptional_string(item, writer)"));
    assert!(kotlin.contains("dev.nexa.core.NexaValueReadResult<String?>"));
    assert!(
        nexa_codegen::value::kotlin_runtime_source()
            .contains("public sealed class NexaValueReadResult<out T>")
    );
    assert!(kotlin.contains(
        "fun nexaReadarray_optional_string(reader: dev.nexa.core.NexaValueReader): List<String?>?"
    ));
    assert!(kotlin.contains("val payload = nexaDecoded_payload.value as String?"));
    assert!(kotlin.contains("return NexaResult.Success(payload)"));
    assert!(
        swift.contains("guard let payload = nexaReadoptional_string(reader) else { return nil }")
    );
    assert!(swift.contains("return .success(payload)"));
    assert!(swift_bridge.contains("NexaDevValueCodec.write(nexaItem0, type: nexaCodec0"));
    assert!(kotlin_bridge.contains("NexaDevValueCodec.write(nexaItem0, nexaCodec0"));
    assert!(kotlin_bridge.contains(
        "NexaDevValueCodec.readResult<Any?>(NexaDevValueCodec.read(nexaCodec0, nexaReader0, enumCases))"
    ));
    let kotlin_dev_codec = nexa_testkit::TestProject::collect_sources_in(
        &output.join("android/app/src/main/java"),
        "kt",
    )
    .into_iter()
    .find(|path| {
        path.file_name()
            .is_some_and(|name| name == "NexaDevValueCodec.kt")
    })
    .expect("find generated Kotlin Dev value codec");
    let kotlin_dev_codec =
        fs::read_to_string(kotlin_dev_codec).expect("read Kotlin Dev value codec");
    assert!(
        kotlin_dev_codec.contains(
            "fun <Value> readResult(raw: Any?): dev.nexa.core.NexaValueReadResult<Value>"
        )
    );
    assert!(kotlin_dev_codec.contains("JSONObject.NULL -> null"));
}

#[test]
fn development_runtime_generates_video_plugin_class_event_and_component_bridges() {
    let project = nexa_testkit::TestProject::new("nexa-dev-video-plugin-bridge");
    let entry = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/video-player/tests/demo/app/App.nx");
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
fn hot_reload_can_add_a_module_and_use_a_preconfigured_plugin_after_host_build() {
    let project = nexa_testkit::TestProject::new("nexa-dev-new-fast-math-module");
    let plugin = nexa_testkit::example_path("plugins/fast-math")
        .canonicalize()
        .expect("resolve installed FastMath package");
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
        "app PluginBridge { body { Text(\"Before plugin use\") } }\n",
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
    .expect("build Dev host before the app uses its installed FastMath plugin");

    let ios_bridge_path = output.join("ios/PluginBridge/NexaDevPluginBridge.swift");
    let android_bridge_path =
        output.join("android/app/src/main/java/com/nexa/pluginbridge/NexaDevPluginBridge.kt");
    let ios_bridge = fs::read_to_string(&ios_bridge_path).expect("read prebuilt Swift bridge");
    let android_bridge =
        fs::read_to_string(&android_bridge_path).expect("read prebuilt Kotlin bridge");
    assert!(
        ios_bridge.contains("FastMathPlugin.shared.add(nexaArg0, nexaArg1)"),
        "prebuilt iOS bridge lacks FastMath.add"
    );
    assert!(
        android_bridge.contains("FastMathPlugin.instance.add(nexaArg0, nexaArg1)"),
        "prebuilt Android bridge lacks FastMath.add"
    );

    let roots = HashMap::from([("dev.nexa.fast-math".to_owned(), plugin)]);
    let mut compiler = nexa_compiler::IncrementalProjectCompiler::default();
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin],
            &roots,
        )
        .expect("compile initial app before adding its FastMath component");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 1);

    project.write(
        "AddedMath.nx",
        "component AddedMath() { state sum: Int32 = 0 body { Button(\"Compute\") { sum = FastMath.add(10, 20) } } }\n",
    );
    project.write(
        "App.nx",
        "plugin \"dev.nexa.fast-math\" as FastMath\nimport \"AddedMath.nx\"\napp PluginBridge { body { AddedMath() } }\n",
    );

    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin],
            &roots,
        )
        .expect("compile app after adding and importing its FastMath component");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 2);
    assert_eq!(compiler.last_compile_stats().reused_source_files, 0);
    let compilations = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin],
            &roots,
        )
        .expect("recompile unchanged imported plugin component");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 0);
    assert_eq!(compiler.last_compile_stats().reused_source_files, 2);

    for compilation in compilations {
        let dev_module = nexa_dev_ir::lower(&compilation.module, "new-plugin-module");
        let json = serde_json::to_string(&dev_module).expect("serialize reloaded Dev IR");
        for marker in ["NativeCall", "AddedMath", "FastMath", "add"] {
            assert!(json.contains(marker), "reloaded Dev IR lacks {marker}");
        }
    }

    assert_eq!(
        fs::read_to_string(&ios_bridge_path).expect("read retained Swift bridge"),
        ios_bridge,
        "hot reload must use the bridge already linked into the host"
    );
    assert_eq!(
        fs::read_to_string(&android_bridge_path).expect("read retained Kotlin bridge"),
        android_bridge,
        "hot reload must use the bridge already linked into the host"
    );
}

#[test]
fn hot_reload_can_add_preconfigured_plugin_classes_events_properties_and_components() {
    let project = nexa_testkit::TestProject::new("nexa-dev-new-video-plugin-module");
    let plugin = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/video-player")
        .canonicalize()
        .expect("resolve installed video-player package");
    let plugin_path = plugin
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    project.write(
        "nexa.config.nx",
        format!(
            "config {{ dependencies {{ VideoPlayer {{ id: \"dev.nexa.video-player\", path: \"{plugin_path}\" }} }} permissions {{}} }}\n"
        ),
    );
    project.write(
        "App.nx",
        "app VideoPluginBridge { body { Text(\"Before plugin use\") } }\n",
    );

    let entry = project.path().join("App.nx");
    let output = project.path().join("build");
    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "VideoPluginBridge",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("build Dev host before the app imports its configured native plugin");

    let ios_bridge_path = output.join("ios/VideoPluginBridge/NexaDevPluginBridge.swift");
    let android_bridge_path = nexa_testkit::TestProject::collect_sources_in(
        &output.join("android/app/src/main/java"),
        "kt",
    )
    .into_iter()
    .find(|path| {
        path.file_name()
            .is_some_and(|name| name == "NexaDevPluginBridge.kt")
    })
    .expect("find generated Kotlin Dev plugin bridge");
    let ios_bridge = fs::read_to_string(&ios_bridge_path).expect("read prebuilt Swift bridge");
    let android_bridge =
        fs::read_to_string(&android_bridge_path).expect("read prebuilt Kotlin bridge");
    for marker in [
        "receiver as? any VideoPlayerSpec",
        "nexaReceiver.play()",
        "nexaReceiver.seek(nexaArg0)",
        "return (true, VideoPlayer())",
        "property == \"volume\"",
        "property == \"onEnded\"",
        "case (\"VideoPlayer\", \"VideoView\"):",
    ] {
        assert!(
            ios_bridge.contains(marker),
            "prebuilt iOS bridge lacks {marker}"
        );
    }
    for marker in [
        "receiver is VideoPlayer",
        "nexaReceiver.play()",
        "nexaReceiver.seek(nexaArg0)",
        "true to VideoPlayer()",
        "property == \"volume\"",
        "property == \"onEnded\"",
        "\"VideoPlayer.VideoView\" ->",
    ] {
        assert!(
            android_bridge.contains(marker),
            "prebuilt Android bridge lacks {marker}"
        );
    }

    let roots = HashMap::from([("dev.nexa.video-player".to_owned(), plugin)]);
    let mut compiler = nexa_compiler::IncrementalProjectCompiler::default();
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin],
            &roots,
        )
        .expect("compile initial app before importing the plugin");

    project.write(
        "AddedPlayer.nx",
        "plugin \"dev.nexa.video-player\" as VideoPlayer\ncomponent AddedPlayer(player: VideoPlayer.VideoPlayer) { body { VideoPlayer.VideoView(player: player) { Text(\"Loaded\") }.onTapped { player.volume = 0.5 } } }\n",
    );
    project.write(
        "App.nx",
        "import \"AddedPlayer.nx\"\napp VideoPluginBridge { let player = VideoPlayer() state ended = false body { OnAppear { player.ended { ended = true } } AddedPlayer(player: player) Button(\"Play\") { player.play() } if ended { Text(\"Ended\") } } }\n",
    );

    let compilations = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin],
            &roots,
        )
        .expect("compile the new module with plugin class, property, event, and component uses");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 2);
    for compilation in compilations {
        let dev_module = nexa_dev_ir::lower(&compilation.module, "new-video-plugin-module");
        let json = serde_json::to_string(&dev_module).expect("serialize reloaded Dev IR");
        for marker in [
            "NativeCall",
            "NativeComponentCall",
            "NativePropertyAssign",
            "NativeEventSubscribe",
        ] {
            assert!(json.contains(marker), "reloaded Dev IR lacks {marker}");
        }
    }

    assert_eq!(
        fs::read_to_string(&ios_bridge_path).expect("read retained Swift bridge"),
        ios_bridge,
        "hot reload must use the bridge already linked into the host"
    );
    assert_eq!(
        fs::read_to_string(&android_bridge_path).expect("read retained Kotlin bridge"),
        android_bridge,
        "hot reload must use the bridge already linked into the host"
    );
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
fn development_runtime_handles_locale_currency_formatting_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(
        &entry,
        r#"app CurrencyFormatting {
            state price: String = Number.formatCurrency(amount: 1234.5, currencyCode: "EUR")
            body { Text(price) }
        }
        "#,
    )
    .expect("write currency formatting app source");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate both Dev hosts");

    let ios = read_ios_dev_runtime(&output);
    assert!(ios.contains("nexaDevFormatCurrency"));
    assert!(ios.contains("namespace == \"Number\", name == \"formatCurrency\""));
    let android = read_android_dev_runtime(&output);
    assert!(android.contains("nexaDevFormatCurrency"));
    assert!(android.contains("namespace == \"Number\" && name == \"formatCurrency\""));
}

#[test]
fn core_crypto_helpers_are_ready_for_hot_reload_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(&entry, "app CryptoHotReload { body { Text(\"ready\") } }\n")
        .expect("write app without crypto calls");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "CryptoHotReload",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts before Crypto is used by app source");

    let ios_crypto =
        fs::read_to_string(output.join("ios/CryptoHotReload/NexaGenerated_crypto.swift"))
            .expect("Dev host should include crypto helpers for future hot reloads");
    assert!(ios_crypto.contains("func nexaCryptoSha256"));
    assert!(ios_crypto.contains("func nexaCryptoHmacSha256"));
    assert!(ios_crypto.contains("func nexaCryptoRandomBytes"));
    let ios_runtime =
        fs::read_to_string(output.join("ios/CryptoHotReload/NexaDevNativeApis.swift"))
            .expect("read iOS native API dispatcher");
    assert!(ios_runtime.contains("case \"sha256\": return nexaCryptoSha256"));
    assert!(ios_runtime.contains("case \"randomBytes\": return nexaCryptoRandomBytes"));

    let android_crypto = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaGenerated_crypto.kt"),
    )
    .expect("Android Dev host should include crypto helpers for future hot reloads");
    assert!(android_crypto.contains("internal fun nexaCryptoSha256"));
    assert!(android_crypto.contains("internal fun nexaCryptoHmacSha256"));
    assert!(android_crypto.contains("internal fun nexaCryptoRandomBytes"));
    let android_runtime = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaDevNativeApis.kt"),
    )
    .expect("read Android native API dispatcher");
    assert!(android_runtime.contains("\"sha256\" -> nexaCryptoSha256"));
    assert!(android_runtime.contains("\"randomBytes\" -> nexaCryptoRandomBytes"));
}

#[test]
fn core_json_codecs_are_ready_for_hot_reload_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(&entry, "app JsonHotReload { body { Text(\"ready\") } }\n")
        .expect("write app without JSON calls");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "JsonHotReload",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts before Json is used by app source");

    let ios_codecs = fs::read_to_string(output.join("ios/JsonHotReload/NexaDevValueCodec.swift"))
        .expect("Dev host should contain JSON codecs for later hot reloads");
    assert!(ios_codecs.contains("static func parseJSON("));
    assert!(ios_codecs.contains("private static func decodeJSON("));
    let ios_native = fs::read_to_string(output.join("ios/JsonHotReload/NexaDevNativeApis.swift"))
        .expect("read iOS native API dispatcher");
    assert!(ios_native.contains("NexaDevValueCodec.parseJSON("));
    assert!(ios_native.contains("NexaDevValueCodec.stringifyJSON("));

    let android_package = output.join("android/app/src/main/java/dev/nexa/runtimesmoke");
    let android_codecs = fs::read_to_string(android_package.join("NexaDevValueCodec.kt"))
        .expect("Dev host should contain JSON codecs for later hot reloads");
    assert!(android_codecs.contains("fun parseJson("));
    assert!(android_codecs.contains("private fun readJson("));
    let android_native = fs::read_to_string(android_package.join("NexaDevNativeApis.kt"))
        .expect("read Android native API dispatcher");
    assert!(android_native.contains("NexaDevValueCodec.parseJson("));
    assert!(android_native.contains("NexaDevValueCodec.stringifyJson("));
}

#[test]
fn native_clipboard_calls_are_ready_for_hot_reload_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(
        &entry,
        "app ClipboardHotReload { body { Text(\"ready\") } }\n",
    )
    .expect("write app without clipboard calls");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts before Clipboard is used by app source");

    let ios = read_ios_dev_runtime(&output);
    assert!(ios.contains("if namespace == \"Clipboard\" {"));
    assert!(ios.contains("case \"setText\":"));
    assert!(ios.contains("UIPasteboard.general.hasStrings"));

    let android = read_android_dev_runtime(&output);
    assert!(android.contains("if (namespace == \"Clipboard\") {"));
    assert!(android.contains("\"setText\" ->"));
    assert!(android.contains("primaryClipDescription"));
}

#[test]
fn native_haptics_calls_are_ready_for_hot_reload_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(
        &entry,
        "app HapticsHotReload { body { Text(\"ready\") } }\n",
    )
    .expect("write app without haptics calls");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts before Haptics is used by app source");

    let ios = read_ios_dev_runtime(&output);
    assert!(ios.contains("private func nexaDevPerformHaptics"));
    assert!(ios.contains("if namespace == \"Haptics\""));
    assert!(ios.contains("UIImpactFeedbackGenerator(style: style).impactOccurred()"));
    assert!(ios.contains("UISelectionFeedbackGenerator().selectionChanged()"));

    let android = read_android_dev_runtime(&output);
    assert!(android.contains("private fun nexaDevPerformHaptics"));
    assert!(android.contains("if (namespace == \"Haptics\")"));
    assert!(android.contains("NexaRuntimeCore.performHapticFeedback(constant)"));
    let core_runtime = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/core/NexaRuntimeCore.kt"),
    )
    .expect("Android Dev host should include its haptics runtime helper");
    assert!(core_runtime.contains("performHapticFeedback(feedbackConstant: Int)"));
}

#[test]
fn native_storage_calls_are_ready_for_hot_reload_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(
        &entry,
        "app StorageHotReload { body { Text(\"ready\") } }\n",
    )
    .expect("write app without storage calls");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts before Storage is used by app source");

    let ios = read_ios_dev_runtime(&output);
    assert!(ios.contains("if namespace == \"Storage\" {"));
    assert!(ios.contains("case \"getString\":"));
    assert!(ios.contains("NexaStorage.setString("));
    let ios_storage =
        fs::read_to_string(output.join("ios/RuntimeSmoke/NexaGenerated_storage.swift"))
            .expect("iOS Dev host should preload the storage helper");
    assert!(ios_storage.contains("enum NexaStorage"));

    let android = read_android_dev_runtime(&output);
    assert!(android.contains("if (namespace == \"Storage\") {"));
    assert!(android.contains("\"getString\" ->"));
    assert!(android.contains("NexaStorage.setString("));
    let android_storage = fs::read_to_string(
        output.join("android/app/src/main/java/dev/nexa/runtimesmoke/NexaGenerated_storage.kt"),
    )
    .expect("Android Dev host should preload the storage helper");
    assert!(android_storage.contains("internal object NexaStorage"));
}

#[test]
fn network_status_calls_are_ready_for_hot_reload_on_both_platforms() {
    let root = temporary_project();
    let entry = root.join("App.nx");
    fs::write(
        &entry,
        "app NetworkStatusHotReload { body { Text(\"ready\") } }\n",
    )
    .expect("write app without network status calls");
    let output = root.join("build");

    nexa_cli::generate_dev_project(
        &entry,
        "all",
        &output,
        "RuntimeSmoke",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Dev hosts before Network.isOnline is used by app source");

    let ios = read_ios_dev_runtime(&output);
    assert!(ios.contains("private final class NexaDevNetworkPathStatus"));
    assert!(ios.contains("if namespace == \"Network\", name == \"isOnline\""));
    assert!(ios.contains("NWPathMonitor()"));

    let android = read_android_dev_runtime(&output);
    assert!(android.contains("namespace == \"Network\" && name == \"isOnline\""));
    assert!(android.contains("android.net.ConnectivityManager"));
    assert!(android.contains("NET_CAPABILITY_INTERNET"));
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
