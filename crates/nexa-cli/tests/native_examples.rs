use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use nexa_testkit::{TestProject, Toolchain, example_path};

const EXAMPLES: &[&str] = &["counter", "showcase", "todo_app", "virtual_list"];

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
}

#[test]
fn generated_android_example_hosts_build_with_gradle_when_available() {
    if !Toolchain::should_run_native_builds() {
        eprintln!("skipping Android example builds in default test tier (opt-in with NEXA_TEST_NATIVE_BUILDS=1 or NEXA_TEST_TIER=e2e)");
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
