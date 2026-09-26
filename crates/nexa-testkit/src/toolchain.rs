use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Test execution tiers for categorizing test speed and resource requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestTier {
    /// Fast in-memory / unit / IR / diagnostics / parser / codegen tests (<30s).
    Fast,
    /// Integration tests including project scaffolding and template checks.
    Integration,
    /// Heavy native toolchain executions (Xcodebuild, full Gradle builds, jarsigner, simulators).
    E2E,
}

impl TestTier {
    /// Determines the active test tier from environment variables.
    ///
    /// Configured via `NEXA_TEST_TIER=fast|integration|e2e|all`,
    /// or opt-in flags `NEXA_TEST_NATIVE_BUILDS=1` / `NEXA_E2E=1`.
    /// Defaults to `TestTier::Fast`.
    pub fn current() -> Self {
        if let Ok(tier) = env::var("NEXA_TEST_TIER") {
            match tier.to_ascii_lowercase().as_str() {
                "fast" => TestTier::Fast,
                "integration" => TestTier::Integration,
                "e2e" | "all" => TestTier::E2E,
                _ => TestTier::Fast,
            }
        } else if env::var("NEXA_TEST_NATIVE_BUILDS").is_ok_and(|v| v == "1" || v == "true")
            || env::var("NEXA_E2E").is_ok_and(|v| v == "1" || v == "true")
        {
            TestTier::E2E
        } else {
            TestTier::Fast
        }
    }

    /// Whether the current tier includes the requested tier.
    pub fn includes(&self, target: TestTier) -> bool {
        *self >= target
    }
}

/// Cached discovery of host platform native toolchains.
pub struct Toolchain;

impl Toolchain {
    /// Returns the currently active test tier.
    pub fn tier() -> TestTier {
        TestTier::current()
    }

    /// Whether heavy external compilation (Xcode simulator builds, full Gradle builds) should execute.
    ///
    /// In fast/default development mode, heavy native compilation is skipped to keep the suite fast (~30s).
    /// Opt-in with `NEXA_TEST_NATIVE_BUILDS=1` or `NEXA_TEST_TIER=e2e`.
    pub fn should_run_native_builds() -> bool {
        Self::tier().includes(TestTier::E2E)
    }

    /// Checks if a program is executable and exits successfully with given args.
    pub fn is_command_available(program: &str, args: &[&str]) -> bool {
        Command::new(program)
            .args(args)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// Finds an executable directly on `PATH`.
    pub fn find_on_path(program: &str) -> Option<PathBuf> {
        let path = env::var_os("PATH")?;
        env::split_paths(&path)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    }

    /// Discovers `swiftc`, cached per process.
    pub fn swiftc() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if let Some(path) = env::var_os("SWIFTC").map(PathBuf::from)
                    && path.is_file()
                {
                    return Some(path);
                }
                if cfg!(target_os = "macos")
                    && Self::is_command_available("xcrun", &["--find", "swiftc"])
                    && let Ok(output) = Command::new("xcrun").args(["--find", "swiftc"]).output()
                    && output.status.success()
                {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    if !text.is_empty() {
                        return Some(PathBuf::from(text));
                    }
                }
                if Self::is_command_available("swiftc", &["--version"]) {
                    return Self::find_on_path("swiftc");
                }
                None
            })
            .as_deref()
    }

    /// Discovers `xcodebuild`, cached per process.
    pub fn xcodebuild() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if !cfg!(target_os = "macos") {
                    return None;
                }
                if let Some(path) = env::var_os("XCODEBUILD").map(PathBuf::from)
                    && path.is_file()
                {
                    return Some(path);
                }
                if Self::is_command_available("xcodebuild", &["-version"]) {
                    return Self::find_on_path("xcodebuild");
                }
                None
            })
            .as_deref()
    }

    /// Discovers `xcrun`, cached per process.
    pub fn xcrun() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if !cfg!(target_os = "macos") {
                    return None;
                }
                if Self::is_command_available("xcrun", &["--version"]) {
                    return Self::find_on_path("xcrun");
                }
                None
            })
            .as_deref()
    }

    /// Checks if the iOS Simulator SDK is installed and available, cached per process.
    pub fn has_ios_simulator_sdk() -> bool {
        Self::ios_simulator_sdk_path().is_some()
    }

    /// Returns the iOS Simulator SDK path, cached per process.
    pub fn ios_simulator_sdk_path() -> Option<&'static str> {
        static CACHE: OnceLock<Option<String>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if !cfg!(target_os = "macos") {
                    return None;
                }
                let output = Command::new("xcrun")
                    .args(["--sdk", "iphonesimulator", "--show-sdk-path"])
                    .output()
                    .ok()?;
                if output.status.success() {
                    let sdk = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    if !sdk.is_empty() {
                        return Some(sdk);
                    }
                }
                None
            })
            .as_deref()
    }

    /// Discovers `kotlinc`, cached per process.
    pub fn kotlinc() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if let Some(path) = env::var_os("KOTLINC").map(PathBuf::from)
                    && path.is_file()
                {
                    return Some(path);
                }
                if Self::is_command_available("kotlinc", &["-version"]) {
                    return Self::find_on_path("kotlinc");
                }
                let android_studio_kotlinc = PathBuf::from(
                    "/Applications/Android Studio.app/Contents/plugins/Kotlin/kotlinc/bin/kotlinc",
                );
                if android_studio_kotlinc.is_file() {
                    return Some(android_studio_kotlinc);
                }
                None
            })
            .as_deref()
    }

    /// Discovers Gradle executable, cached per process.
    pub fn gradle() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if let Some(path) = env::var_os("GRADLE").map(PathBuf::from)
                    && path.is_file()
                {
                    return Some(path);
                }
                if Self::is_command_available("gradle", &["--version"]) {
                    return Self::find_on_path("gradle");
                }
                let home = env::var_os("GRADLE_USER_HOME")
                    .map(PathBuf::from)
                    .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".gradle")))?;
                let mut candidates = Vec::new();
                collect_executables(&home.join("wrapper/dists"), "gradle", &mut candidates);
                candidates.sort();
                candidates.pop()
            })
            .as_deref()
    }

    /// Discovers `clang++`, cached per process.
    pub fn clang_plus_plus() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if let Some(path) = env::var_os("CLANGXX").map(PathBuf::from)
                    && path.is_file()
                {
                    return Some(path);
                }
                if Self::is_command_available("clang++", &["--version"]) {
                    return Self::find_on_path("clang++");
                }
                None
            })
            .as_deref()
    }

    /// Discovers Java executable (`java`), cached per process.
    pub fn java() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from) {
                    let candidate = java_home.join("bin/java");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
                if Self::is_command_available("java", &["-version"]) {
                    return Self::find_on_path("java");
                }
                None
            })
            .as_deref()
    }

    /// Discovers `jarsigner`, cached per process.
    pub fn jarsigner() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from) {
                    let candidate = java_home.join("bin/jarsigner");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
                Self::find_on_path("jarsigner")
            })
            .as_deref()
    }

    /// Discovers `keytool`, cached per process.
    pub fn keytool() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                if let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from) {
                    let candidate = java_home.join("bin/keytool");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
                Self::find_on_path("keytool")
            })
            .as_deref()
    }

    /// Discovers Android SDK directory, cached per process.
    pub fn android_sdk() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                env::var_os("ANDROID_HOME")
                    .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
                    .map(PathBuf::from)
                    .filter(|p| p.is_dir())
            })
            .as_deref()
    }

    /// Discovers Android NDK C++ compiler (`clang++`), cached per process.
    pub fn android_ndk_compiler() -> Option<&'static Path> {
        static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
        CACHE
            .get_or_init(|| {
                let sdk = Self::android_sdk()?;
                let mut ndks = fs::read_dir(sdk.join("ndk"))
                    .ok()?
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect::<Vec<_>>();
                ndks.sort();
                ndks.into_iter().rev().find_map(|ndk| {
                    let compiler = ndk.join("toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android26-clang++");
                    if compiler.is_file() {
                        Some(compiler)
                    } else {
                        // Check for other host platforms if on Linux/etc
                        let linux_compiler = ndk.join("toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android26-clang++");
                        linux_compiler.is_file().then_some(linux_compiler)
                    }
                })
            })
            .as_deref()
    }
}

fn collect_executables(directory: &Path, name: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().is_some_and(|n| n == name) {
            found.push(path);
        } else if path.is_dir() {
            collect_executables(&path, name, found);
        }
    }
}
