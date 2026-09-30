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
    output
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
