use std::{
    env, fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const AUDIT_APP_NAME: &str = "NexaAudit";

pub(super) fn measure_release_sizes(
    input: &Path,
    measure_ios: bool,
    measure_android: bool,
) -> String {
    let mut ios = if measure_ios {
        match ios_toolchain() {
            Ok(toolchain) => IosResult::available(toolchain),
            Err(reason) => IosResult::unavailable(reason),
        }
    } else {
        IosResult::not_requested()
    };
    let mut android = if measure_android {
        match android_toolchain() {
            Ok(toolchain) => AndroidResult::available(toolchain),
            Err(reason) => AndroidResult::unavailable(reason),
        }
    } else {
        AndroidResult::not_requested()
    };

    let has_available_target = ios.toolchain.is_some() || android.toolchain.is_some();
    if !has_available_target {
        return measurements_json(&ios, &android);
    }

    let temp = match TempDirectory::new() {
        Ok(temp) => temp,
        Err(error) => {
            let reason = format!("could not create temporary audit project: {error}");
            ios.fail_if_available(reason.clone());
            android.fail_if_available(reason);
            return measurements_json(&ios, &android);
        }
    };
    let generated_root = temp.path.join("Generated");
    let generate_target = match (ios.toolchain.is_some(), android.toolchain.is_some()) {
        (true, true) => "all",
        (true, false) => "ios",
        (false, true) => "android",
        (false, false) => unreachable!("checked for available targets"),
    };
    let generation = env::current_exe()
        .map_err(|error| format!("could not locate Nexa executable: {error}"))
        .and_then(|executable| {
            Command::new(executable)
                .arg("generate")
                .arg(input)
                .args(["--target", generate_target, "--out"])
                .arg(&generated_root)
                .args(["--name", AUDIT_APP_NAME])
                .output()
                .map_err(|error| format!("could not start temporary project generation: {error}"))
        });
    let generation_error = match generation {
        Ok(output) if output.status.success() => None,
        Ok(output) => Some(command_failure("temporary project generation", &output)),
        Err(error) => Some(error),
    };

    if let Some(reason) = generation_error {
        ios.fail_if_available(reason.clone());
        android.fail_if_available(reason);
        return measurements_json(&ios, &android);
    }

    if let Some(toolchain) = ios.toolchain.clone() {
        ios.measure(&generated_root, &temp.path, &toolchain);
    }
    if let Some(toolchain) = android.toolchain.clone() {
        android.measure(&generated_root, &toolchain);
    }
    measurements_json(&ios, &android)
}

fn measurements_json(ios: &IosResult, android: &AndroidResult) -> String {
    format!(
        "{{\n      \"ios\": {},\n      \"android\": {}\n    }}",
        ios.json(),
        android.json()
    )
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    /// Claims a scratch directory for one audit run.
    ///
    /// Ownership comes from `create_dir`, which fails when a name is taken,
    /// rather than from a clock reading. `as_nanos()` repeats under concurrency
    /// -- the clock is far coarser than a nanosecond -- and `create_dir_all` on
    /// an existing directory is a no-op rather than an error, so a clock-derived
    /// name lets two concurrent audits silently share one tree, and whichever
    /// finishes first deletes the other's generated sources. The counter keeps
    /// the retry making progress, and the process id keeps concurrent audits
    /// apart. `nexa-testkit::TempDir` is the same primitive for test code.
    fn new() -> std::io::Result<Self> {
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let base = env::temp_dir();
        for _ in 0..1024 {
            let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = base.join(format!(
                "nexa-audit-release-{}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::other(
            "no unused temporary directory name available for the audit scratch directory",
        ))
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone)]
struct IosToolchain {
    xcodebuild: PathBuf,
    version: String,
}

fn ios_toolchain() -> Result<IosToolchain, String> {
    if !cfg!(target_os = "macos") {
        return Err("iOS release measurements require macOS and Xcode".to_owned());
    }
    let xcodebuild = find_command("xcodebuild")
        .ok_or_else(|| "`xcodebuild` is not available on PATH".to_owned())?;
    let xcrun =
        find_command("xcrun").ok_or_else(|| "`xcrun` is not available on PATH".to_owned())?;
    let version = Command::new(&xcodebuild)
        .arg("-version")
        .output()
        .map_err(|error| format!("could not query Xcode: {error}"))?;
    if !version.status.success() {
        return Err(command_failure("xcodebuild -version", &version));
    }
    let sdk = Command::new(xcrun)
        .args(["--sdk", "iphoneos", "--show-sdk-path"])
        .output()
        .map_err(|error| format!("could not query the iOS device SDK: {error}"))?;
    if !sdk.status.success() {
        return Err(command_failure("xcrun iPhoneOS SDK discovery", &sdk));
    }
    Ok(IosToolchain {
        xcodebuild,
        version: version_summary(&version, &["Xcode ", "Build version "]),
    })
}

#[derive(Clone)]
struct AndroidToolchain {
    gradle: PathBuf,
    gradle_version: String,
    sdk: PathBuf,
}

fn android_toolchain() -> Result<AndroidToolchain, String> {
    if !command_succeeds("java", &["-version"]) {
        return Err("Java is not available on PATH".to_owned());
    }
    let sdk = android_sdk().ok_or_else(|| {
        "Android SDK was not found in ANDROID_HOME, ANDROID_SDK_ROOT, or its default location"
            .to_owned()
    })?;
    let gradle = gradle_executable().ok_or_else(|| {
        "Gradle was not found on PATH, in GRADLE, or in the Gradle wrapper cache".to_owned()
    })?;
    let version = Command::new(&gradle)
        .arg("--version")
        .output()
        .map_err(|error| format!("could not query Gradle: {error}"))?;
    if !version.status.success() {
        return Err(command_failure("gradle --version", &version));
    }
    Ok(AndroidToolchain {
        gradle,
        gradle_version: version_summary(&version, &["Gradle "]),
        sdk,
    })
}

fn android_sdk() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for variable in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(value) = env::var_os(variable).filter(|value| !value.is_empty()) {
            candidates.push(PathBuf::from(value));
        }
    }
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join("Library/Android/sdk"));
        candidates.push(home.join("Android/Sdk"));
    }
    candidates
        .into_iter()
        .find(|path| path.join("platforms").is_dir())
}

fn gradle_executable() -> Option<PathBuf> {
    if let Some(path) = env::var_os("GRADLE").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    if let Some(path) = find_command("gradle") {
        return Some(path);
    }

    let home = env::var_os("GRADLE_USER_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".gradle")))?;
    let distributions = fs::read_dir(home.join("wrapper/dists")).ok()?;
    let mut candidates = Vec::new();
    for distribution in distributions.filter_map(Result::ok) {
        if !distribution.path().is_dir() {
            continue;
        }
        for hash in fs::read_dir(distribution.path())
            .ok()?
            .filter_map(Result::ok)
        {
            if !hash.path().is_dir() {
                continue;
            }
            for version in fs::read_dir(hash.path()).ok()?.filter_map(Result::ok) {
                if !version.path().is_dir() {
                    continue;
                }
                let executable = version.path().join("bin").join(gradle_binary_name());
                if executable.is_file() {
                    candidates.push((
                        parse_version_key(&version.file_name().to_string_lossy()),
                        executable,
                    ));
                }
            }
        }
    }
    candidates.sort_unstable_by(|left, right| right.0.cmp(&left.0));
    candidates.into_iter().next().map(|(_, path)| path)
}

#[cfg(windows)]
fn gradle_binary_name() -> &'static str {
    "gradle.bat"
}

#[cfg(not(windows))]
fn gradle_binary_name() -> &'static str {
    "gradle"
}

fn parse_version_key(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect()
}

fn find_command(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

struct IosResult {
    status: &'static str,
    reason: Option<String>,
    toolchain: Option<IosToolchain>,
    executable_bytes: Option<u64>,
    bundle_bytes: Option<u64>,
    other_bytes: Option<u64>,
}

impl IosResult {
    fn not_requested() -> Self {
        Self::empty("not_requested", None)
    }

    fn unavailable(reason: String) -> Self {
        Self::empty("unavailable", Some(reason))
    }

    fn available(toolchain: IosToolchain) -> Self {
        Self {
            status: "pending",
            reason: None,
            toolchain: Some(toolchain),
            executable_bytes: None,
            bundle_bytes: None,
            other_bytes: None,
        }
    }

    fn empty(status: &'static str, reason: Option<String>) -> Self {
        Self {
            status,
            reason,
            toolchain: None,
            executable_bytes: None,
            bundle_bytes: None,
            other_bytes: None,
        }
    }

    fn fail_if_available(&mut self, reason: String) {
        if self.toolchain.is_some() {
            self.status = "failed";
            self.reason = Some(reason);
        }
    }

    fn measure(&mut self, generated_root: &Path, temp_root: &Path, toolchain: &IosToolchain) {
        let project = generated_root
            .join("ios")
            .join(format!("{AUDIT_APP_NAME}.xcodeproj"));
        let derived_data = temp_root.join("DerivedData-iOS");
        let output = Command::new(&toolchain.xcodebuild)
            .args(["-quiet", "-project"])
            .arg(&project)
            .args([
                "-scheme",
                AUDIT_APP_NAME,
                "-configuration",
                "Release",
                "-sdk",
                "iphoneos",
                "-destination",
                "generic/platform=iOS",
                "-derivedDataPath",
            ])
            .arg(&derived_data)
            .arg("CODE_SIGNING_ALLOWED=NO")
            .arg("build")
            .output();
        match output {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                self.status = "failed";
                self.reason = Some(command_failure("iOS Release build", &output));
                return;
            }
            Err(error) => {
                self.status = "failed";
                self.reason = Some(format!("could not start iOS Release build: {error}"));
                return;
            }
        }

        let app = derived_data
            .join("Build/Products/Release-iphoneos")
            .join(format!("{AUDIT_APP_NAME}.app"));
        let executable = app.join(AUDIT_APP_NAME);
        let executable_bytes = match fs::metadata(&executable) {
            Ok(metadata) if metadata.is_file() => metadata.len(),
            Ok(_) => {
                self.status = "failed";
                self.reason = Some("iOS app executable is not a file".to_owned());
                return;
            }
            Err(error) => {
                self.status = "failed";
                self.reason = Some(format!("could not inspect iOS app executable: {error}"));
                return;
            }
        };
        let bundle_bytes = match directory_bytes(&app) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = "failed";
                self.reason = Some(format!("could not inspect iOS app bundle: {error}"));
                return;
            }
        };
        self.status = "measured";
        self.executable_bytes = Some(executable_bytes);
        self.bundle_bytes = Some(bundle_bytes);
        self.other_bytes = Some(bundle_bytes.saturating_sub(executable_bytes));
    }

    fn json(&self) -> String {
        let versions = self
            .toolchain
            .as_ref()
            .map_or_else(String::new, |toolchain| {
                format!(", \"xcodeVersion\": {}", quote(&toolchain.version))
            });
        format!(
            "{{\"status\": {}, \"reason\": {}, \"sdk\": \"iphoneos\", \"configuration\": \"Release\"{versions}, \"mainExecutableBytes\": {}, \"appBundleBytes\": {}, \"bundleOtherBytes\": {}}}",
            quote(self.status),
            optional_string(self.reason.as_deref()),
            optional_number(self.executable_bytes),
            optional_number(self.bundle_bytes),
            optional_number(self.other_bytes),
        )
    }
}

struct AndroidResult {
    status: &'static str,
    reason: Option<String>,
    toolchain: Option<AndroidToolchain>,
    apk_bytes: Option<u64>,
    apk_entries: Option<usize>,
    dex_compressed: Option<u64>,
    dex_uncompressed: Option<u64>,
    resources_compressed: Option<u64>,
    resources_uncompressed: Option<u64>,
    assets_compressed: Option<u64>,
    assets_uncompressed: Option<u64>,
    native_compressed: Option<u64>,
    native_uncompressed: Option<u64>,
    other_compressed: Option<u64>,
    mapping_bytes: Option<u64>,
    usage_bytes: Option<u64>,
    resource_report_bytes: Option<u64>,
    minification_enabled: Option<bool>,
    resource_shrinking_enabled: Option<bool>,
}

impl AndroidResult {
    fn not_requested() -> Self {
        Self::empty("not_requested", None)
    }

    fn unavailable(reason: String) -> Self {
        Self::empty("unavailable", Some(reason))
    }

    fn available(toolchain: AndroidToolchain) -> Self {
        let mut result = Self::empty("pending", None);
        result.toolchain = Some(toolchain);
        result
    }

    fn empty(status: &'static str, reason: Option<String>) -> Self {
        Self {
            status,
            reason,
            toolchain: None,
            apk_bytes: None,
            apk_entries: None,
            dex_compressed: None,
            dex_uncompressed: None,
            resources_compressed: None,
            resources_uncompressed: None,
            assets_compressed: None,
            assets_uncompressed: None,
            native_compressed: None,
            native_uncompressed: None,
            other_compressed: None,
            mapping_bytes: None,
            usage_bytes: None,
            resource_report_bytes: None,
            minification_enabled: None,
            resource_shrinking_enabled: None,
        }
    }

    fn fail_if_available(&mut self, reason: String) {
        if self.toolchain.is_some() {
            self.status = "failed";
            self.reason = Some(reason);
        }
    }

    fn measure(&mut self, generated_root: &Path, toolchain: &AndroidToolchain) {
        let project = generated_root.join("android");
        let output = Command::new(&toolchain.gradle)
            .arg("--no-daemon")
            .arg("--console=plain")
            .arg("--project-dir")
            .arg(&project)
            .arg(":app:assembleRelease")
            .env("ANDROID_HOME", &toolchain.sdk)
            .env("ANDROID_SDK_ROOT", &toolchain.sdk)
            .output();
        match output {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                self.status = "failed";
                self.reason = Some(command_failure("Android Release build", &output));
                return;
            }
            Err(error) => {
                self.status = "failed";
                self.reason = Some(format!("could not start Android Release build: {error}"));
                return;
            }
        }

        let apk_directory = project.join("app/build/outputs/apk/release");
        let apk = match find_release_apk(&apk_directory) {
            Ok(Some(path)) => path,
            Ok(None) => {
                self.status = "failed";
                self.reason = Some("Android Release build produced no APK".to_owned());
                return;
            }
            Err(error) => {
                self.status = "failed";
                self.reason = Some(format!("could not locate Android Release APK: {error}"));
                return;
            }
        };
        let stats = match apk_inventory(&apk) {
            Ok(stats) => stats,
            Err(error) => {
                self.status = "failed";
                self.reason = Some(format!("could not inspect Android Release APK: {error}"));
                return;
            }
        };
        let apk_bytes = match fs::metadata(&apk) {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                self.status = "failed";
                self.reason = Some(format!(
                    "could not inspect Android Release APK size: {error}"
                ));
                return;
            }
        };
        let app_dir = project.join("app/build/outputs/mapping/release");
        self.status = "measured";
        self.apk_bytes = Some(apk_bytes);
        self.apk_entries = Some(stats.entry_count);
        self.dex_compressed = Some(stats.dex.compressed);
        self.dex_uncompressed = Some(stats.dex.uncompressed);
        self.resources_compressed = Some(stats.resources.compressed);
        self.resources_uncompressed = Some(stats.resources.uncompressed);
        self.assets_compressed = Some(stats.assets.compressed);
        self.assets_uncompressed = Some(stats.assets.uncompressed);
        self.native_compressed = Some(stats.native.compressed);
        self.native_uncompressed = Some(stats.native.uncompressed);
        self.other_compressed = Some(stats.other.compressed);
        self.mapping_bytes = file_bytes_if_present(&app_dir.join("mapping.txt"));
        self.usage_bytes = file_bytes_if_present(&app_dir.join("usage.txt"));
        self.resource_report_bytes = file_bytes_if_present(&app_dir.join("resources.txt"));
        let gradle_file = project.join("app/build.gradle.kts");
        let config = fs::read_to_string(gradle_file).unwrap_or_default();
        self.minification_enabled = Some(config.contains("isMinifyEnabled = true"));
        self.resource_shrinking_enabled = Some(config.contains("isShrinkResources = true"));
    }

    fn json(&self) -> String {
        let version = self
            .toolchain
            .as_ref()
            .map_or_else(String::new, |toolchain| {
                format!(", \"gradleVersion\": {}", quote(&toolchain.gradle_version))
            });
        format!(
            "{{\"status\": {}, \"reason\": {}, \"configuration\": \"Release\"{version}, \"minificationEnabled\": {}, \"resourceShrinkingEnabled\": {}, \"apkBytes\": {}, \"apkEntryCount\": {}, \"dexCompressedBytes\": {}, \"dexUncompressedBytes\": {}, \"resourcesCompressedBytes\": {}, \"resourcesUncompressedBytes\": {}, \"assetsCompressedBytes\": {}, \"assetsUncompressedBytes\": {}, \"nativeLibrariesCompressedBytes\": {}, \"nativeLibrariesUncompressedBytes\": {}, \"otherCompressedBytes\": {}, \"r8MappingBytes\": {}, \"r8UsageReportBytes\": {}, \"resourceShrinkerReportBytes\": {}}}",
            quote(self.status),
            optional_string(self.reason.as_deref()),
            optional_bool(self.minification_enabled),
            optional_bool(self.resource_shrinking_enabled),
            optional_number(self.apk_bytes),
            optional_usize(self.apk_entries),
            optional_number(self.dex_compressed),
            optional_number(self.dex_uncompressed),
            optional_number(self.resources_compressed),
            optional_number(self.resources_uncompressed),
            optional_number(self.assets_compressed),
            optional_number(self.assets_uncompressed),
            optional_number(self.native_compressed),
            optional_number(self.native_uncompressed),
            optional_number(self.other_compressed),
            optional_number(self.mapping_bytes),
            optional_number(self.usage_bytes),
            optional_number(self.resource_report_bytes),
        )
    }
}

fn find_release_apk(directory: &Path) -> std::io::Result<Option<PathBuf>> {
    let mut apks = Vec::new();
    if !directory.is_dir() {
        return Ok(None);
    }
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "apk") {
                apks.push(path);
            }
        }
    }
    apks.sort();
    Ok(apks
        .iter()
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name == "app-release.apk")
        })
        .cloned()
        .or_else(|| apks.into_iter().next()))
}

#[derive(Default, Clone, Copy)]
struct ApkCategory {
    compressed: u64,
    uncompressed: u64,
}

#[derive(Default)]
struct ApkInventory {
    entry_count: usize,
    dex: ApkCategory,
    resources: ApkCategory,
    assets: ApkCategory,
    native: ApkCategory,
    other: ApkCategory,
}

fn apk_inventory(path: &Path) -> Result<ApkInventory, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let file_length = file.metadata().map_err(|error| error.to_string())?.len();
    let tail_length = file_length.min(65_557) as usize;
    let mut tail = vec![0; tail_length];
    file.seek(SeekFrom::End(-(tail_length as i64)))
        .and_then(|_| file.read_exact(&mut tail))
        .map_err(|error| format!("could not read ZIP directory footer: {error}"))?;
    let end = tail
        .windows(4)
        .rposition(|bytes| bytes == [0x50, 0x4b, 0x05, 0x06])
        .ok_or_else(|| "APK has no ZIP end-of-central-directory record".to_owned())?;
    let eocd = &tail[end..];
    if eocd.len() < 22 {
        return Err("APK ZIP footer is truncated".to_owned());
    }
    let entry_count = read_u16(eocd, 10)? as usize;
    let central_size = read_u32(eocd, 12)? as u64;
    let central_offset = read_u32(eocd, 16)? as u64;
    if entry_count == u16::MAX as usize || central_size == u32::MAX as u64 {
        return Err("ZIP64 APK inventories are not supported".to_owned());
    }

    file.seek(SeekFrom::Start(central_offset))
        .map_err(|error| format!("could not seek to APK central directory: {error}"))?;
    let mut inventory = ApkInventory::default();
    for _ in 0..entry_count {
        let mut header = [0; 46];
        file.read_exact(&mut header)
            .map_err(|error| format!("truncated APK central directory: {error}"))?;
        if header[..4] != [0x50, 0x4b, 0x01, 0x02] {
            return Err("APK central directory entry has an invalid signature".to_owned());
        }
        let compressed = read_u32(&header, 20)? as u64;
        let uncompressed = read_u32(&header, 24)? as u64;
        if compressed == u32::MAX as u64 || uncompressed == u32::MAX as u64 {
            return Err("ZIP64 APK entry sizes are not supported".to_owned());
        }
        let name_length = read_u16(&header, 28)? as usize;
        let extra_length = read_u16(&header, 30)? as usize;
        let comment_length = read_u16(&header, 32)? as usize;
        let mut name = vec![0; name_length];
        file.read_exact(&mut name)
            .map_err(|error| format!("truncated APK entry name: {error}"))?;
        file.seek(SeekFrom::Current((extra_length + comment_length) as i64))
            .map_err(|error| format!("could not skip APK entry metadata: {error}"))?;
        inventory.entry_count += 1;
        let name = String::from_utf8_lossy(&name);
        let category = if name.ends_with(".dex")
            && name
                .rsplit('/')
                .next()
                .is_some_and(|part| part.starts_with("classes"))
        {
            &mut inventory.dex
        } else if name == "resources.arsc" || name.starts_with("res/") {
            &mut inventory.resources
        } else if name.starts_with("assets/") {
            &mut inventory.assets
        } else if name.starts_with("lib/") {
            &mut inventory.native
        } else {
            &mut inventory.other
        };
        category.compressed = category
            .compressed
            .checked_add(compressed)
            .ok_or_else(|| "APK compressed category size overflow".to_owned())?;
        category.uncompressed = category
            .uncompressed
            .checked_add(uncompressed)
            .ok_or_else(|| "APK uncompressed category size overflow".to_owned())?;
    }
    Ok(inventory)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let bytes = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "truncated ZIP metadata".to_owned())?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "truncated ZIP metadata".to_owned())?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn directory_bytes(directory: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let bytes = if metadata.is_dir() {
            directory_bytes(&path)?
        } else if metadata.is_file() {
            metadata.len()
        } else {
            0
        };
        total = total
            .checked_add(bytes)
            .ok_or_else(|| std::io::Error::other("bundle byte count overflow"))?;
    }
    Ok(total)
}

fn file_bytes_if_present(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
}

fn command_failure(label: &str, output: &Output) -> String {
    let details = output_text(output);
    let details = details.chars().take(4_000).collect::<String>();
    if details.is_empty() {
        format!("{label} exited with {}", output.status)
    } else {
        format!("{label} exited with {}: {details}", output.status)
    }
}

fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!(
        "{}{}{}",
        stdout.trim(),
        if stdout.is_empty() || stderr.is_empty() {
            ""
        } else {
            "\n"
        },
        stderr.trim()
    )
}

fn version_summary(output: &Output, prefixes: &[&str]) -> String {
    let text = output_text(output);
    let lines = text
        .lines()
        .filter(|line| prefixes.iter().any(|prefix| line.starts_with(prefix)))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        text.lines().next().unwrap_or("unknown").to_owned()
    } else {
        lines.join("; ")
    }
}

fn quote(value: &str) -> String {
    format!("\"{}\"", super::json_escape(value))
}

fn optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), quote)
}

fn optional_number(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn optional_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn optional_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{apk_inventory, directory_bytes, parse_version_key};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    /// A temporary directory owned for as long as it is bound.
    struct TempDirectory(nexa_testkit::TempDir);

    impl TempDirectory {
        fn new() -> Self {
            Self(nexa_testkit::TempDir::new("nexa-audit-native-unit"))
        }
    }

    #[test]
    fn gradle_versions_sort_by_numeric_components() {
        assert!(parse_version_key("gradle-8.10") > parse_version_key("gradle-8.9"));
    }

    #[test]
    fn ios_bundle_size_counts_regular_files_recursively_and_ignores_symlinks() {
        let temp = TempDirectory::new();
        fs::create_dir(temp.0.join("Assets")).expect("asset directory should be created");
        fs::write(temp.0.join("NexaAudit"), [0; 7]).expect("app binary should be written");
        fs::write(temp.0.join("Assets/Assets.car"), [0; 13])
            .expect("asset catalog should be written");
        assert_eq!(
            directory_bytes(&temp.0).expect("bundle should be measurable"),
            20
        );
    }

    #[test]
    fn apk_inventory_separates_dex_resources_assets_and_native_libraries() {
        let temp = TempDirectory::new();
        let apk = temp.0.join("app-release.apk");
        fs::write(&apk, minimal_apk()).expect("synthetic APK should be written");
        let inventory = apk_inventory(&apk).expect("APK central directory should be parsed");
        assert_eq!(inventory.entry_count, 5);
        assert_eq!(inventory.dex.compressed, 2);
        assert_eq!(inventory.dex.uncompressed, 8);
        assert_eq!(inventory.resources.compressed, 3);
        assert_eq!(inventory.resources.uncompressed, 12);
        assert_eq!(inventory.assets.compressed, 4);
        assert_eq!(inventory.assets.uncompressed, 16);
        assert_eq!(inventory.native.compressed, 5);
        assert_eq!(inventory.native.uncompressed, 20);
        assert_eq!(inventory.other.compressed, 6);
        assert_eq!(inventory.other.uncompressed, 24);
    }

    fn minimal_apk() -> Vec<u8> {
        let entries = [
            ("classes.dex", 2u32, 8u32),
            ("resources.arsc", 3, 12),
            ("assets/payload.bin", 4, 16),
            ("lib/arm64-v8a/libnexa.so", 5, 20),
            ("META-INF/cert", 6, 24),
        ];
        let mut bytes = Vec::new();
        let mut central = Vec::new();
        for (name, compressed, uncompressed) in entries {
            let name = name.as_bytes();
            bytes.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            bytes.extend_from_slice(&[20, 0]);
            bytes.extend_from_slice(&[0; 2]);
            bytes.extend_from_slice(&[0; 2]);
            bytes.extend_from_slice(&[0; 4]);
            bytes.extend_from_slice(&[0; 4]);
            bytes.extend_from_slice(&compressed.to_le_bytes());
            bytes.extend_from_slice(&uncompressed.to_le_bytes());
            bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(&vec![0; compressed as usize]);

            central.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0]);
            central.extend_from_slice(&[0; 8]);
            central.extend_from_slice(&compressed.to_le_bytes());
            central.extend_from_slice(&uncompressed.to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&[0; 6]);
            central.extend_from_slice(&[0; 10]);
            central.extend_from_slice(name);
        }
        let central_offset = bytes.len() as u32;
        bytes.extend_from_slice(&central);
        bytes.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(central.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&central_offset.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes
    }
}
