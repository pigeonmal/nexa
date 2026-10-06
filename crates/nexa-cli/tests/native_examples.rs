use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use nexa_testkit::{TestProject, Toolchain, example_path};

const EXAMPLES: &[&str] = &[
    "counter",
    "crypto",
    "clipboard",
    "haptics",
    "network_status",
    "currency_formatting",
    "keyboard_ergonomics",
    "pressable_double_tap",
    "pressable_pinch",
    "spring_animation",
    "showcase",
    "todo_app",
    "virtual_list",
];

fn generate_example(project: &TestProject, example: &str, target: &str) -> PathBuf {
    let entry = example_path(&format!("{example}.nx"));
    let output = project.join(format!("{example}-{target}"));
    let app_name = format!(
        "Nexa{}",
        example
            .split('_')
            .map(|part| {
                let mut characters = part.chars();
                characters
                    .next()
                    .map(|first| first.to_uppercase().chain(characters).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<String>()
    );
    nexa_cli::generate_project(&entry, target, &output, &app_name)
        .unwrap_or_else(|error| panic!("failed to generate {example} for {target}:\n{error}"));
    if example == "network_status" && target == "android" {
        let manifest = fs::read_to_string(output.join("android/app/src/main/AndroidManifest.xml"))
            .expect("network status example should generate an Android manifest");
        assert!(manifest.contains("android.permission.ACCESS_NETWORK_STATE"));
        let gradle = fs::read_to_string(output.join("android/app/build.gradle.kts"))
            .expect("network status example should generate Android Gradle config");
        assert!(!gradle.to_ascii_lowercase().contains("cronet"));
    }
    output
}

fn build_android_debug_if_available(android_project: &Path, host: &str) {
    if !Toolchain::should_run_native_builds() {
        return;
    }
    let (Some(gradle), Some(sdk)) = (Toolchain::gradle(), Toolchain::android_sdk()) else {
        return;
    };
    if !sdk.join("platforms/android-36/android.jar").is_file() {
        return;
    }
    let built = Command::new(gradle)
        .args([":app:assembleDebug"])
        .current_dir(android_project.join("android"))
        .output()
        .expect("Gradle should start for the Android native host");
    assert!(
        built.status.success(),
        "{host} failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
}

#[test]
fn imperative_animation_aot_hosts_compile_when_native_toolchains_are_available() {
    let temp = TestProject::new("nexa-imperative-animation-aot");
    let entry = temp.join("AnimatedProgress.nx");
    fs::write(
        &entry,
        r#"app AnimatedProgress {
    state progress: Float64 = 0.0
    body {
        Button("Complete") {
            withAnimation(Spring(response: 0.35, damping: 0.8)) {
                progress = 1.0
            }
        }
        ProgressBar(progress: progress)
    }
}"#,
    )
    .expect("animation fixture should be written");

    let ios = temp.join("animated-progress-ios");
    nexa_cli::generate_project(&entry, "ios", &ios, "NexaAnimatedProgress")
        .expect("iOS animation host should generate");
    if let Some(sdk) = Toolchain::ios_simulator_sdk_path() {
        let built = Command::new("xcrun")
            .args([
                "--sdk",
                "iphonesimulator",
                "swiftc",
                "-typecheck",
                "-sdk",
                sdk,
                "-target",
                "arm64-apple-ios16.0-simulator",
            ])
            .args(swift_sources(&ios.join("ios/NexaAnimatedProgress")))
            .output()
            .expect("Swift compiler should start for the animation host");
        assert!(
            built.status.success(),
            "iOS animation host failed to type-check:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }

    let android = temp.join("animated-progress-android");
    nexa_cli::generate_project(&entry, "android", &android, "NexaAnimatedProgress")
        .expect("Android animation host should generate");
    build_android_debug_if_available(&android, "Android animation host");
}

#[test]
fn multipart_upload_generates_native_streaming_calls() {
    let temp = TestProject::new("nexa-multipart-upload-codegen");
    let entry = temp.join("Upload.nx");
    fs::write(
        &entry,
        r#"app Upload {
    state statusCode: Int32 = 0
    body {
        Text("Upload")
        OnAppear async {
            try {
                statusCode = (await Network.upload(
                    url: "https://example.com/upload",
                    file: "/tmp/receipt.pdf",
                    fields: ["kind": "receipt"]
                )).statusCode
            } catch {
                statusCode = -1
            }
        }
    }
}"#,
    )
    .expect("upload fixture should be written");

    let ios = temp.join("upload-ios");
    nexa_cli::generate_project(&entry, "ios", &ios, "NexaUpload")
        .expect("iOS upload project should generate");
    let swift = swift_sources(&ios.join("ios/NexaUpload"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Swift source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift.contains("try await NexaNetwork.upload("));
    assert!(swift.contains("fromFile: uploadURL"));

    let android = temp.join("upload-android");
    nexa_cli::generate_project(&entry, "android", &android, "NexaUpload")
        .expect("Android upload project should generate");
    let kotlin = kotlin_sources(&android.join("android/app/src/main/java"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Kotlin source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(kotlin.contains("NexaNetwork.upload(NexaRuntime.context()"));
    assert!(kotlin.contains("NexaMultipartUploadProvider"));

    if let Some(sdk) = Toolchain::ios_simulator_sdk_path() {
        let built = Command::new("xcrun")
            .args([
                "--sdk",
                "iphonesimulator",
                "swiftc",
                "-typecheck",
                "-sdk",
                sdk,
                "-target",
                "arm64-apple-ios16.0-simulator",
            ])
            .args(swift_sources(&ios.join("ios/NexaUpload")))
            .output()
            .expect("Swift compiler should start for the iOS upload host");
        assert!(
            built.status.success(),
            "iOS upload host failed to type-check:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }

    build_android_debug_if_available(&android, "Android upload host");
}

#[test]
fn background_task_generates_native_schedulers_and_feature_gated_dependencies() {
    let temp = TestProject::new("nexa-background-task-codegen");
    let entry = example_path("background_task.nx");

    let ios = temp.join("background-task-ios");
    nexa_cli::generate_project(&entry, "ios", &ios, "NexaBackgroundSync")
        .expect("iOS background-task project should generate");
    let swift = swift_sources(&ios.join("ios/NexaBackgroundSync"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Swift source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift.contains(".backgroundTask(.appRefresh(\"dev.example.backgroundsync.refresh\"))"));
    assert!(swift.contains("await __nexaBackgroundTask0()"));
    assert!(swift.contains("NexaStorage.setString("));
    let plist = fs::read_to_string(ios.join("ios/NexaBackgroundSync/Info.plist"))
        .expect("read generated iOS background-task metadata");
    assert!(plist.contains("<key>BGTaskSchedulerPermittedIdentifiers</key>"));
    assert!(plist.contains("<string>dev.example.backgroundsync.refresh</string>"));
    assert!(plist.contains("<key>UIBackgroundModes</key><array><string>fetch</string></array>"));
    if let Some(sdk) = Toolchain::ios_simulator_sdk_path() {
        let built = Command::new("xcrun")
            .args([
                "--sdk",
                "iphonesimulator",
                "swiftc",
                "-typecheck",
                "-sdk",
                sdk,
                "-target",
                "arm64-apple-ios16.0-simulator",
            ])
            .args(swift_sources(&ios.join("ios/NexaBackgroundSync")))
            .output()
            .expect("Swift compiler should start for the iOS background-task host");
        assert!(
            built.status.success(),
            "iOS background-task host failed to type-check:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }

    let android = temp.join("background-task-android");
    nexa_cli::generate_project(&entry, "android", &android, "NexaBackgroundSync")
        .expect("Android background-task project should generate");
    let kotlin = kotlin_sources(&android.join("android/app/src/main/java"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Kotlin source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(kotlin.contains("suspend fun __nexaBackgroundTask0()"));
    assert!(kotlin.contains("class NexaBackgroundWorker"));
    assert!(kotlin.contains("ExistingPeriodicWorkPolicy.UPDATE"));
    assert!(kotlin.contains("NexaRuntime.bind(applicationContext)"));
    let gradle = fs::read_to_string(android.join("android/app/build.gradle.kts"))
        .expect("read generated Android dependencies");
    assert!(gradle.contains("androidx.work:work-runtime-ktx:2.11.2"));
    build_android_debug_if_available(&android, "Android background-task host");
}

#[test]
fn imported_websocket_screen_generates_a_hot_reloadable_plugin_bridge() {
    let temp = TestProject::new("nexa-websocket-imported-screen-dev");
    let entry =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/websocket/tests/demo/app/App.nx");
    let ios = temp.join("websocket-dev-ios");
    nexa_cli::generate_dev_project(
        &entry,
        "ios",
        &ios,
        "NexaWebSocketDemo",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("iOS DevRuntime host should include the plugin used by imported Chat.nx");

    let swift = swift_sources(&ios.join("ios/NexaWebSocketDemo"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated iOS DevRuntime source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift.contains("name == \"connect\""));
    assert!(swift.contains("property == \"onMessageReceived\""));
    assert!(swift.contains("nexaDevFailureWebSocketWebSocketError"));
    assert!(!ios.join("ios/NexaWebSocketDemo/Chat.nx").exists());

    if let Some(sdk) = Toolchain::ios_simulator_sdk_path() {
        let built = Command::new("xcrun")
            .args([
                "--sdk",
                "iphonesimulator",
                "swiftc",
                "-typecheck",
                "-sdk",
                sdk,
                "-target",
                "arm64-apple-ios16.0-simulator",
            ])
            .args(swift_sources(&ios.join("ios/NexaWebSocketDemo")))
            .output()
            .expect("Swift compiler should start for the WebSocket DevRuntime host");
        assert!(
            built.status.success(),
            "iOS WebSocket DevRuntime host failed to type-check:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }

    let android = temp.join("websocket-dev-android");
    nexa_cli::generate_dev_project(
        &entry,
        "android",
        &android,
        "NexaWebSocketDemo",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("Android DevRuntime host should include the plugin used by imported Chat.nx");

    let bridge = fs::read_to_string(
        android.join("android/app/src/main/java/dev/nexa/websocket/demo/NexaDevPluginBridge.kt"),
    )
    .expect("Android DevRuntime should generate the WebSocket plugin bridge");
    assert!(bridge.contains("name == \"connect\""));
    assert!(bridge.contains("property == \"onMessageReceived\""));
    assert!(bridge.contains("nexaDevFailureWebSocketWebSocketError"));
    assert!(!android.join("android/app/src/main/assets/Chat.nx").exists());

    if Toolchain::should_run_native_builds()
        && let (Some(gradle), Some(sdk)) = (Toolchain::gradle(), Toolchain::android_sdk())
        && sdk.join("platforms/android-36/android.jar").is_file()
    {
        let built = Command::new(gradle)
            .args([":app:assembleDebug"])
            .current_dir(android.join("android"))
            .output()
            .expect("Gradle should start for the WebSocket DevRuntime host");
        assert!(
            built.status.success(),
            "Android WebSocket DevRuntime host failed to compile:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }
}

fn copy_generated_kotlin(source: &Path, destination: &Path) {
    copy_generated_kotlin_from(source, source, destination);
}

fn copy_generated_kotlin_from(root: &Path, source: &Path, destination: &Path) {
    let Ok(entries) = fs::read_dir(source) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            copy_generated_kotlin_from(root, &path, destination);
        } else if path.extension().is_some_and(|extension| extension == "kt")
            && path
                .file_name()
                .is_some_and(|name| name != "MainActivity.kt" && name != "NexaGenerated.kt")
        {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            let output = destination.join(relative);
            fs::create_dir_all(output.parent().expect("Kotlin file has a parent"))
                .expect("generated Kotlin package directory should be created");
            fs::copy(&path, output).expect("generated Kotlin source should be staged");
        }
    }
}

#[test]
fn generated_ios_example_hosts_build_with_xcode_when_available() {
    let Some(sdk) = Toolchain::ios_simulator_sdk_path() else {
        eprintln!("skipping iOS example builds: iOS Simulator SDK is unavailable");
        return;
    };

    let temp = TestProject::new("nexa-native-examples-ios");
    for example in EXAMPLES {
        let project = generate_example(&temp, example, "ios");
        let source_root = project.join("ios");
        let sources = swift_sources(&source_root.join(example_app_dir(example)));
        let mut command = Command::new("xcrun");
        command
            .args([
                "--sdk",
                "iphonesimulator",
                "swiftc",
                "-typecheck",
                "-sdk",
                sdk,
                "-target",
                "arm64-apple-ios16.0-simulator",
            ])
            .args(&sources);
        let built = command.output().expect("Swift compiler should start");
        assert!(
            built.status.success(),
            "generated iOS sources for `{example}` failed to type-check:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }

    let dev_project = temp.join("spring-animation-dev-ios");
    nexa_cli::generate_dev_project(
        &example_path("spring_animation.nx"),
        "ios",
        &dev_project,
        "NexaSpringAnimation",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate iOS DevRuntime host");
    let sources = swift_sources(&dev_project.join("ios/NexaSpringAnimation"));
    let built = Command::new("xcrun")
        .args([
            "--sdk",
            "iphonesimulator",
            "swiftc",
            "-typecheck",
            "-sdk",
            sdk,
            "-target",
            "arm64-apple-ios16.0-simulator",
        ])
        .args(&sources)
        .output()
        .expect("Swift compiler should start for the iOS DevRuntime host");
    assert!(
        built.status.success(),
        "iOS DevRuntime host failed to type-check:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    let plugin_dev_project = temp.join("video-player-dev-ios");
    nexa_cli::generate_dev_project(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/video-player/tests/demo/app/App.nx"),
        "ios",
        &plugin_dev_project,
        "NexaVideoPlayerDemo",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate iOS DevRuntime host with class, event, and component plugin APIs");
    let sources = swift_sources(&plugin_dev_project.join("ios/NexaVideoPlayerDemo"));
    let built = Command::new("xcrun")
        .args([
            "--sdk",
            "iphonesimulator",
            "swiftc",
            "-typecheck",
            "-sdk",
            sdk,
            "-target",
            "arm64-apple-ios17.0-simulator",
        ])
        .args(&sources)
        .output()
        .expect("Swift compiler should start for the iOS plugin DevRuntime host");
    assert!(
        built.status.success(),
        "iOS plugin DevRuntime host failed to type-check:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
}

#[test]
fn haptics_example_generates_native_calls_and_android_activity_binding() {
    let temp = TestProject::new("nexa-haptics-codegen");
    let ios = generate_example(&temp, "haptics", "ios");
    let swift = swift_sources(&ios.join("ios/NexaHaptics"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Swift source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift.contains("import UIKit"));
    assert!(swift.contains("UIImpactFeedbackGenerator(style: .light).impactOccurred()"));
    assert!(swift.contains("UINotificationFeedbackGenerator().notificationOccurred(.success)"));
    assert!(swift.contains("UISelectionFeedbackGenerator().selectionChanged()"));

    let android = generate_example(&temp, "haptics", "android");
    let kotlin = kotlin_sources(&android.join("android/app/src/main/java"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Kotlin source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(kotlin.contains("import androidx.compose.ui.platform.LocalContext"));
    assert!(kotlin.contains("NexaRuntime.bind(LocalContext.current)"));
    assert!(kotlin.contains(
        "NexaRuntimeCore.performHapticFeedback(android.view.HapticFeedbackConstants.KEYBOARD_TAP)"
    ));
    let core_runtime = fs::read_to_string(
        android.join("android/app/src/main/java/dev/nexa/core/NexaRuntimeCore.kt"),
    )
    .expect("read generated Android core runtime");
    assert!(core_runtime.contains("performHapticFeedback(feedbackConstant: Int)"));
}

#[test]
fn screen_orientation_generates_direct_native_calls_and_runtime_support() {
    let temp = TestProject::new("nexa-screen-orientation-codegen");
    let entry = temp.join("Orientation.nx");
    fs::write(
        &entry,
        r#"app Orientation {
    body {
        Column {
            Button("Portrait") { Screen.lockOrientation(mode: Portrait) }
            Button("Landscape") { Screen.lockOrientation(mode: Landscape) }
            Button("All") { Screen.lockOrientation(mode: All) }
        }
    }
}"#,
    )
    .expect("orientation fixture should be written");
    fs::write(
        temp.join("nexa.config.nx"),
        "config { app { orientation: \"portrait-phones\" } }",
    )
    .expect("write phone-only orientation policy");

    let ios = temp.join("orientation-ios");
    nexa_cli::generate_project(&entry, "ios", &ios, "NexaOrientation")
        .expect("iOS orientation project should generate");
    let swift = swift_sources(&ios.join("ios/NexaOrientation"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Swift source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(swift.contains("import UIKit"));
    assert!(swift.contains("NexaScreen.lockOrientation(\"Portrait\")"));
    assert!(swift.contains("NexaScreen.lockOrientation(\"Landscape\")"));
    assert!(swift.contains("NexaScreen.lockOrientation(\"All\")"));
    assert!(swift.contains("requestGeometryUpdate(.iOS(interfaceOrientations: mask))"));
    let plist = fs::read_to_string(ios.join("ios/NexaOrientation/Info.plist"))
        .expect("read generated iOS Info.plist");
    assert!(plist.contains("UIInterfaceOrientationLandscapeLeft"));
    assert!(plist.contains("UIInterfaceOrientationLandscapeRight"));
    if let Some(sdk) = Toolchain::ios_simulator_sdk_path() {
        let sources = swift_sources(&ios.join("ios/NexaOrientation"));
        let built = Command::new("xcrun")
            .args([
                "--sdk",
                "iphonesimulator",
                "swiftc",
                "-typecheck",
                "-sdk",
                sdk,
                "-target",
                "arm64-apple-ios17.0-simulator",
            ])
            .args(&sources)
            .output()
            .expect("Swift compiler should start for the orientation host");
        assert!(
            built.status.success(),
            "iOS orientation host failed to type-check:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }

    let android = temp.join("orientation-android");
    nexa_cli::generate_project(&entry, "android", &android, "NexaOrientation")
        .expect("Android orientation project should generate");
    let kotlin = kotlin_sources(&android.join("android/app/src/main/java"))
        .iter()
        .map(|path| fs::read_to_string(path).expect("read generated Kotlin source"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(kotlin.contains("NexaRuntimeCore.lockOrientation(\"Portrait\")"));
    assert!(kotlin.contains("NexaRuntimeCore.lockOrientation(\"Landscape\")"));
    assert!(kotlin.contains("NexaRuntimeCore.lockOrientation(\"All\")"));
    assert!(kotlin.contains("import androidx.compose.ui.platform.LocalContext"));
    assert!(kotlin.contains("NexaRuntime.bind(LocalContext.current)"));
    let core_runtime = fs::read_to_string(
        android.join("android/app/src/main/java/dev/nexa/core/NexaRuntimeCore.kt"),
    )
    .expect("read generated Android core runtime");
    assert!(core_runtime.contains("lockOrientation(mode: String)"));
    assert!(core_runtime.contains("SCREEN_ORIENTATION_SENSOR_LANDSCAPE"));
    assert!(core_runtime.contains("applyConfiguredOrientation(activity: android.app.Activity)"));
    let activity = fs::read_to_string(
        android.join("android/app/src/main/java/com/nexa/nexaorientation/MainActivity.kt"),
    )
    .expect("read generated Android activity");
    assert!(activity.contains("applyConfiguredOrientation(this)"));
    assert!(activity.contains("onConfigurationChanged(newConfig: Configuration)"));
}

#[test]
fn generated_android_example_hosts_build_with_gradle_when_available() {
    if !Toolchain::should_run_native_builds() {
        eprintln!(
            "skipping Android example builds in default test tier (opt-in with NEXA_TEST_NATIVE_BUILDS=1 or NEXA_TEST_TIER=e2e)"
        );
        return;
    }

    let (Some(gradle), Some(sdk)) = (Toolchain::gradle(), Toolchain::android_sdk()) else {
        eprintln!("skipping Android example builds: Gradle or Android SDK is unavailable");
        return;
    };
    if !sdk.join("platforms/android-36/android.jar").is_file() {
        eprintln!("skipping Android example builds: Android API 36 is unavailable");
        return;
    }

    let temp = TestProject::new("nexa-native-examples-android");
    let project = generate_example(&temp, "showcase", "android");
    let java_root = project.join("android/app/src/main/java");
    for example in EXAMPLES
        .iter()
        .copied()
        .filter(|example| *example != "showcase")
    {
        let generated = generate_example(&temp, example, "android");
        let package_root = generated.join("android/app/src/main/java");
        copy_generated_kotlin(&package_root, &java_root);
    }
    let built = Command::new(gradle)
        .args([":app:assembleDebug"])
        .current_dir(project.join("android"))
        .output()
        .expect("Gradle should start when the Android SDK is available");
    assert!(
        built.status.success(),
        "generated Android sources for the example set failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    let dev_project = temp.join("spring-animation-dev-android");
    nexa_cli::generate_dev_project(
        &example_path("spring_animation.nx"),
        "android",
        &dev_project,
        "NexaSpringAnimation",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Android DevRuntime host");
    let built = Command::new(gradle)
        .args([":app:assembleDebug"])
        .current_dir(dev_project.join("android"))
        .output()
        .expect("Gradle should start for the Android DevRuntime host");
    assert!(
        built.status.success(),
        "Android DevRuntime host failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    let plugin_dev_project = temp.join("video-player-dev-android");
    nexa_cli::generate_dev_project(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/video-player/tests/demo/app/App.nx"),
        "android",
        &plugin_dev_project,
        "NexaVideoPlayerDemo",
        "ws://127.0.0.1:43210",
        "0123456789abcdef0123456789abcdef",
    )
    .expect("generate Android DevRuntime host with class, event, and component plugin APIs");
    let built = Command::new(gradle)
        .args([":app:assembleDebug"])
        .current_dir(plugin_dev_project.join("android"))
        .output()
        .expect("Gradle should start for the Android plugin DevRuntime host");
    assert!(
        built.status.success(),
        "Android plugin DevRuntime host failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
}

fn example_app_dir(example: &str) -> String {
    let mut name = String::from("Nexa");
    for part in example.split('_') {
        let mut characters = part.chars();
        if let Some(first) = characters.next() {
            name.push_str(&first.to_uppercase().collect::<String>());
            name.extend(characters);
        }
    }
    name
}

fn swift_sources(root: &Path) -> Vec<String> {
    let mut sources = Vec::new();
    collect_swift_sources(root, &mut sources);
    sources.sort();
    sources
}

fn kotlin_sources(root: &Path) -> Vec<String> {
    let mut sources = Vec::new();
    collect_kotlin_sources(root, &mut sources);
    sources.sort();
    sources
}

fn collect_kotlin_sources(root: &Path, sources: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_kotlin_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "kt") {
            sources.push(path.to_string_lossy().into_owned());
        }
    }
}

fn collect_swift_sources(root: &Path, sources: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_swift_sources(&path, sources);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "swift")
        {
            sources.push(path.to_string_lossy().into_owned());
        }
    }
}
