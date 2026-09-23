use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const EXAMPLES: &[&str] = &["counter", "showcase", "todo_app", "virtual_list"];

struct TempRoot(PathBuf);

impl TempRoot {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        Self(env::temp_dir().join(format!(
            "nexa-native-examples-{}-{nonce}",
            std::process::id()
        )))
    }

    fn generate(&self, example: &str, target: &str) -> PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let output = self.0.join(format!("{example}-{target}"));
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
        let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
            .args([
                "generate",
                root.join(format!("{example}.nx")).to_str().unwrap(),
                "--target",
                target,
                "--out",
                output.to_str().unwrap(),
                "--name",
                &app_name,
            ])
            .output()
            .expect("Nexa CLI should start");
        assert!(
            generated.status.success(),
            "failed to generate {example} for {target}:\n{}\n{}",
            String::from_utf8_lossy(&generated.stdout),
            String::from_utf8_lossy(&generated.stderr)
        );
        output
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn android_sdk() -> Option<PathBuf> {
    env::var_os("ANDROID_HOME")
        .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
}

fn gradle() -> Option<PathBuf> {
    if let Some(path) = env::var_os("GRADLE").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    if let Some(path) = find_on_path("gradle") {
        return Some(path);
    }
    let home = env::var_os("GRADLE_USER_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".gradle")))?;
    let mut found = Vec::new();
    collect_gradle(&home.join("wrapper/dists"), &mut found);
    found.sort();
    found.pop()
}

fn collect_gradle(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == "gradle") && path.is_file() {
            found.push(path);
        } else if path.is_dir() {
            collect_gradle(&path, found);
        }
    }
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
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
                .is_some_and(|name| name != "MainActivity.kt")
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
    if !Command::new("xcrun")
        .args(["--sdk", "iphonesimulator", "--show-sdk-path"])
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("skipping iOS example builds: iOS Simulator SDK is unavailable");
        return;
    }

    let temp = TempRoot::new();
    for example in EXAMPLES {
        let project = temp.generate(example, "ios");
        let source_root = project.join("ios");
        let sdk = Command::new("xcrun")
            .args(["--sdk", "iphonesimulator", "--show-sdk-path"])
            .output()
            .expect("xcrun should start");
        let sdk = String::from_utf8_lossy(&sdk.stdout).trim().to_owned();
        let sources = swift_sources(&source_root.join(example_app_dir(example)));
        let mut command = Command::new("xcrun");
        command
            .args([
                "--sdk",
                "iphonesimulator",
                "swiftc",
                "-typecheck",
                "-sdk",
                &sdk,
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
    let (Some(gradle), Some(sdk)) = (gradle(), android_sdk()) else {
        eprintln!("skipping Android example builds: Gradle or Android SDK is unavailable");
        return;
    };
    if !sdk.join("platforms/android-36/android.jar").is_file() {
        eprintln!("skipping Android example builds: Android API 36 is unavailable");
        return;
    }

    let temp = TempRoot::new();
    let project = temp.generate("showcase", "android");
    let java_root = project.join("android/app/src/main/java");
    for example in EXAMPLES
        .iter()
        .copied()
        .filter(|example| *example != "showcase")
    {
        let generated = temp.generate(example, "android");
        let package_root = generated.join("android/app/src/main/java");
        copy_generated_kotlin(&package_root, &java_root);
    }
    let built = Command::new(&gradle)
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
