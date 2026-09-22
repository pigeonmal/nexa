//! Native project bundle generation.
//!
//! This module deliberately owns only project scaffolding. The compiler and
//! backends continue to own parsing, semantic analysis, and native source
//! generation. Keeping templates here lets the generated host projects evolve
//! without adding a runtime or coupling platform build files to the IR.

use std::{
    fs,
    path::{Path, PathBuf},
};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Module;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProjectTarget {
    Ios,
    Android,
    All,
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut name = None;
    let mut target = ProjectTarget::All;
    let mut deny_warnings = false;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--deny-warnings" => deny_warnings = true,
            "--target" | "-t" => {
                cursor += 1;
                target = match args.get(cursor).map(String::as_str) {
                    Some("ios") | Some("swift") => ProjectTarget::Ios,
                    Some("android") | Some("kotlin") => ProjectTarget::Android,
                    Some("all") => ProjectTarget::All,
                    Some(value) => {
                        return Err(format!(
                            "unknown target `{value}`; expected `ios`, `android`, or `all`"
                        ));
                    }
                    None => return Err("`--target` requires `ios`, `android`, or `all`".to_owned()),
                };
            }
            "--out" | "--output" | "-o" => {
                cursor += 1;
                output = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a directory")?,
                ));
            }
            "--name" | "-n" => {
                cursor += 1;
                name = Some(
                    args.get(cursor)
                        .ok_or("`--name` requires an app name")?
                        .clone(),
                );
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if input.is_none() => input = Some(PathBuf::from(value)),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }

    let input = input.ok_or("usage: nexa generate <source.nx> [--target <ios|android|all>] [--out <directory>] [--name <AppName>] [--deny-warnings]")?;
    let output = output.unwrap_or_else(|| {
        let stem = input
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("nexa-app");
        input.with_file_name(format!("{stem}-project"))
    });

    let targets: &[Target] = match target {
        ProjectTarget::Ios => &[Target::Swift],
        ProjectTarget::Android => &[Target::Kotlin],
        ProjectTarget::All => &[Target::Swift, Target::Kotlin],
    };
    let mut compiled = Vec::with_capacity(targets.len());
    let mut warnings = Vec::new();
    for &compile_target in targets {
        let compilation = compile_file_with_warnings_for_target(&input, compile_target)
            .map_err(|error| error.to_string())?;
        warnings.extend(compilation.warnings.clone());
        compiled.push((compile_target, compilation.module));
    }
    let warnings = super::deduplicate_warnings(warnings);
    super::report_warnings(&warnings, deny_warnings)?;

    let inferred_name = compiled
        .first()
        .map(|(_, module)| module.app_name.as_str())
        .unwrap_or("NexaApp");
    let app_name = name.as_deref().unwrap_or(inferred_name);
    let app_name = type_name(app_name);
    if app_name.is_empty() {
        return Err("`--name` must contain at least one letter or digit".to_owned());
    }
    fs::create_dir_all(&output).map_err(|error| format!("{}: {error}", output.display()))?;

    let mut generated_targets = Vec::new();
    for (compile_target, mut module) in compiled {
        module.app_name = app_name.clone();
        match compile_target {
            Target::Swift => {
                generate_ios(&output, &app_name, &module)?;
                generated_targets.push("ios");
            }
            Target::Kotlin => {
                generate_android(&output, &app_name, &module)?;
                generated_targets.push("android");
            }
            Target::All => unreachable!("project generation compiles concrete platform targets"),
        }
    }

    let entry = input.canonicalize().unwrap_or(input.clone());
    let manifest = format!(
        "{{\n  \"format\": 1,\n  \"entry\": \"{}\",\n  \"name\": \"{}\",\n  \"targets\": [{}]\n}}\n",
        json_escape(&entry.display().to_string()),
        json_escape(&app_name),
        generated_targets
            .iter()
            .map(|target| format!("\"{target}\""))
            .collect::<Vec<_>>()
            .join(", "),
    );
    write_if_changed(&output.join("nexa.project.json"), &manifest)?;
    write_if_changed(
        &output.join("README.md"),
        &root_readme(&app_name, &generated_targets),
    )?;
    println!(
        "generated {} ({})",
        output.display(),
        generated_targets.join(", ")
    );
    Ok(())
}

fn generate_ios(root: &Path, app_name: &str, module: &Module) -> Result<(), String> {
    let directory = root.join("ios").join(app_name);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let screen = nexa_codegen::names::screen_name(app_name);
    let source = SwiftBackend.generate(module);
    write_if_changed(&directory.join("NexaGenerated.swift"), &source)?;
    write_if_changed(
        &directory.join(format!("{app_name}App.swift")),
        &format!(
            "import SwiftUI\n\n@main\nstruct {app_name}App: App {{\n    var body: some Scene {{\n        WindowGroup {{\n            {screen}()\n        }}\n    }}\n}}\n"
        ),
    )?;
    write_if_changed(
        &directory.join("Info.plist"),
        &ios_info_plist(app_name, &module.permissions),
    )?;
    write_if_changed(
        &root
            .join("ios")
            .join(format!("{app_name}.xcodeproj/project.pbxproj")),
        &ios_project_file(app_name),
    )?;
    Ok(())
}

fn generate_android(root: &Path, app_name: &str, module: &Module) -> Result<(), String> {
    let package = package_name(app_name);
    let package_path = package.replace('.', "/");
    let source_dir = root.join("android/app/src/main/java").join(&package_path);
    fs::create_dir_all(&source_dir)
        .map_err(|error| format!("{}: {error}", source_dir.display()))?;
    let generated = KotlinBackend.generate(module);
    let screen = nexa_codegen::names::screen_name(app_name);
    let remote = generated.contains("NexaNetwork")
        || generated.contains("coil3.network")
        || generated.contains("org.chromium.net");
    let cronet_import = if remote {
        "import com.google.android.gms.net.CronetProviderInstaller\n"
    } else {
        ""
    };
    let content_setup = if remote {
        format!(
            "        CronetProviderInstaller.installProvider(this).addOnCompleteListener {{\n            setContent {{ MaterialTheme {{ {screen}() }} }}\n        }}\n"
        )
    } else {
        format!("        setContent {{ MaterialTheme {{ {screen}() }} }}\n")
    };
    write_if_changed(
        &source_dir.join("NexaGenerated.kt"),
        &format!("package {package}\n\n{generated}"),
    )?;
    write_if_changed(
        &source_dir.join("MainActivity.kt"),
        &format!(
            "package {package}\n\nimport android.os.Bundle\nimport androidx.activity.ComponentActivity\nimport androidx.activity.compose.setContent\nimport androidx.compose.material3.MaterialTheme\n{cronet_import}\nclass MainActivity : ComponentActivity() {{\n    override fun onCreate(savedInstanceState: Bundle?) {{\n        super.onCreate(savedInstanceState)\n{content_setup}    }}\n}}\n"
        ),
    )?;
    write_if_changed(
        &root.join("android/app/src/main/AndroidManifest.xml"),
        &android_manifest(app_name, &package, remote, &module.permissions),
    )?;
    write_if_changed(
        &root.join("android/settings.gradle.kts"),
        &android_settings(app_name),
    )?;
    write_if_changed(
        &root.join("android/build.gradle.kts"),
        &android_root_gradle(),
    )?;
    write_if_changed(
        &root.join("android/gradle.properties"),
        &android_properties(),
    )?;
    write_if_changed(
        &root.join("android/app/build.gradle.kts"),
        &android_app_gradle(&package, remote, generated.contains("NavHost")),
    )?;
    Ok(())
}

fn write_if_changed(path: &Path, contents: &str) -> Result<(), String> {
    if path.is_file() && fs::read_to_string(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn type_name(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            result.push(character);
        }
    }
    if result.starts_with(|character: char| character.is_ascii_digit()) {
        result.insert(0, 'N');
    }
    result
}

fn package_name(app_name: &str) -> String {
    format!(
        "com.nexa.{}",
        app_name.to_ascii_lowercase().replace('_', "")
    )
}
fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn root_readme(app_name: &str, targets: &[&str]) -> String {
    let mut readme = format!(
        "# {app_name}\n\nGenerated by Nexa from a single `.nx` entry file. Edit the Nexa source and regenerate; generated native files are build outputs.\n\n"
    );
    if targets.contains(&"ios") {
        readme.push_str(&format!("## iOS\n\n`xcodebuild -project ios/{app_name}.xcodeproj -scheme {app_name} -sdk iphonesimulator build`\n\nOpen `ios/{app_name}.xcodeproj` in Xcode to run on a device or simulator.\n\n"));
    }
    if targets.contains(&"android") {
        readme.push_str("## Android\n\n`gradle -p android :app:assembleDebug`\n\nThe generated Gradle project uses Jetpack Compose and Coil 3 with the platform network stack when needed.\n\n");
    }
    readme.push_str("Requirements: Rust/Nexa for regeneration, Xcode 15+ for iOS, and Android SDK/Gradle for Android.\n");
    readme
}

fn ios_info_plist(app_name: &str, permissions: &[nexa_ir::Permission]) -> String {
    let mut entries = String::new();
    for permission in permissions {
        let (key, description) = match permission {
            nexa_ir::Permission::Camera => {
                ("NSCameraUsageDescription", "Nexa needs camera access.")
            }
            nexa_ir::Permission::Microphone => (
                "NSMicrophoneUsageDescription",
                "Nexa needs microphone access.",
            ),
            nexa_ir::Permission::Photos => (
                "NSPhotoLibraryUsageDescription",
                "Nexa needs photo library access.",
            ),
            nexa_ir::Permission::Location => (
                "NSLocationWhenInUseUsageDescription",
                "Nexa needs location access while in use.",
            ),
            // iOS notification authorization has no Info.plist usage-description key.
            nexa_ir::Permission::Notifications => continue,
            nexa_ir::Permission::Contacts => {
                ("NSContactsUsageDescription", "Nexa needs contacts access.")
            }
            nexa_ir::Permission::Calendar => {
                ("NSCalendarsUsageDescription", "Nexa needs calendar access.")
            }
            nexa_ir::Permission::Bluetooth => (
                "NSBluetoothAlwaysUsageDescription",
                "Nexa needs Bluetooth access.",
            ),
        };
        entries.push_str(&format!("<key>{key}</key><string>{description}</string>"));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleDisplayName</key><string>{app_name}</string><key>CFBundleIdentifier</key><string>com.nexa.{}</string><key>CFBundleName</key><string>{app_name}</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>1.0</string><key>CFBundleVersion</key><string>1</string><key>LSRequiresIPhoneOS</key><true/>{entries}</dict></plist>\n",
        app_name.to_ascii_lowercase()
    )
}

fn ios_project_file(app_name: &str) -> String {
    let app_file = format!("{app_name}App.swift");
    format!(
        "// !$*UTF8*$!\n{{\n\tarchiveVersion = 1;\n\tclasses = {{}};\n\tobjectVersion = 56;\n\tobjects = {{\n\t\tAA0000000000000000000001 = {{ isa = PBXProject; buildConfigurationList = AA0000000000000000000002; compatibilityVersion = \"Xcode 14.0\"; mainGroup = AA0000000000000000000003; productRefGroup = AA0000000000000000000004; targets = ( AA0000000000000000000005 ); }};\n\t\tAA0000000000000000000003 = {{ isa = PBXGroup; children = ( AA0000000000000000000014, AA0000000000000000000004 ); sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000014 = {{ isa = PBXGroup; children = ( AA0000000000000000000010, AA0000000000000000000011, AA0000000000000000000012 ); path = {app_name}; sourceTree = \"<group>\"; }};
        AA0000000000000000000004 = {{ isa = PBXGroup; children = ( AA0000000000000000000013 ); name = Products; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000010 = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {app_file}; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000011 = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = NexaGenerated.swift; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000012 = {{ isa = PBXFileReference; lastKnownFileType = text.plist.xml; path = Info.plist; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000013 = {{ isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = {app_name}.app; sourceTree = BUILT_PRODUCTS_DIR; }};\n\t\tAA0000000000000000000020 = {{ isa = PBXBuildFile; fileRef = AA0000000000000000000010; }};\n\t\tAA0000000000000000000021 = {{ isa = PBXBuildFile; fileRef = AA0000000000000000000011; }};\n\t\tAA0000000000000000000005 = {{ isa = PBXNativeTarget; buildConfigurationList = AA0000000000000000000007; buildPhases = ( AA0000000000000000000008, AA0000000000000000000009, AA000000000000000000000A ); name = {app_name}; productName = {app_name}; productReference = AA0000000000000000000013; productType = \"com.apple.product-type.application\"; }};\n\t\tAA0000000000000000000008 = {{ isa = PBXSourcesBuildPhase; files = ( AA0000000000000000000020, AA0000000000000000000021 ); }};\n\t\tAA0000000000000000000009 = {{ isa = PBXFrameworksBuildPhase; files = (); }};\n\t\tAA000000000000000000000A = {{ isa = PBXResourcesBuildPhase; files = (); }};\n\t\tAA0000000000000000000002 = {{ isa = XCConfigurationList; buildConfigurations = ( AA0000000000000000000022 ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; }};\n\t\tAA0000000000000000000007 = {{ isa = XCConfigurationList; buildConfigurations = ( AA0000000000000000000023 ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; }};\n\t\tAA0000000000000000000022 = {{ isa = XCBuildConfiguration; buildSettings = {{ SWIFT_VERSION = 5.0; IPHONEOS_DEPLOYMENT_TARGET = 16.0; }}; name = Release; }};\n\t\tAA0000000000000000000023 = {{ isa = XCBuildConfiguration; buildSettings = {{ PRODUCT_BUNDLE_IDENTIFIER = com.nexa.{}; PRODUCT_NAME = {app_name}; INFOPLIST_FILE = {app_name}/Info.plist; SWIFT_VERSION = 5.0; IPHONEOS_DEPLOYMENT_TARGET = 16.0; TARGETED_DEVICE_FAMILY = \"1,2\"; }}; name = Release; }};\n\t}};\n\trootObject = AA0000000000000000000001;\n}}\n",
        app_name.to_ascii_lowercase()
    )
}

fn android_settings(app_name: &str) -> String {
    format!(
        "pluginManagement {{ repositories {{ google(); mavenCentral(); gradlePluginPortal() }} }}\ndependencyResolutionManagement {{ repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories {{ google(); mavenCentral() }} }}\nrootProject.name = \"{app_name}\"\ninclude(\":app\")\n"
    )
}
fn android_root_gradle() -> String {
    "plugins {\n    id(\"com.android.application\") version \"8.5.2\" apply false\n    id(\"org.jetbrains.kotlin.android\") version \"2.0.21\" apply false\n    id(\"org.jetbrains.kotlin.plugin.compose\") version \"2.0.21\" apply false\n}\n".to_owned()
}
fn android_properties() -> String {
    "android.useAndroidX=true\nandroid.enableJetifier=true\nkotlin.code.style=official\norg.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n".to_owned()
}
fn android_manifest(
    app_name: &str,
    package: &str,
    remote: bool,
    permissions: &[nexa_ir::Permission],
) -> String {
    let mut declared = String::new();
    if remote {
        declared.push_str("    <uses-permission android:name=\"android.permission.INTERNET\" />\n");
    }
    for permission in permissions {
        let names: &[&str] = match permission {
            nexa_ir::Permission::Camera => &["android.permission.CAMERA"],
            nexa_ir::Permission::Microphone => &["android.permission.RECORD_AUDIO"],
            nexa_ir::Permission::Photos => &[
                "android.permission.READ_MEDIA_IMAGES",
                "android.permission.READ_EXTERNAL_STORAGE",
            ],
            nexa_ir::Permission::Location => &[
                "android.permission.ACCESS_COARSE_LOCATION",
                "android.permission.ACCESS_FINE_LOCATION",
            ],
            nexa_ir::Permission::Notifications => &["android.permission.POST_NOTIFICATIONS"],
            nexa_ir::Permission::Contacts => &[
                "android.permission.READ_CONTACTS",
                "android.permission.WRITE_CONTACTS",
            ],
            nexa_ir::Permission::Calendar => &[
                "android.permission.READ_CALENDAR",
                "android.permission.WRITE_CALENDAR",
            ],
            nexa_ir::Permission::Bluetooth => &[
                "android.permission.BLUETOOTH_SCAN",
                "android.permission.BLUETOOTH_CONNECT",
            ],
        };
        for name in names {
            declared.push_str(&format!(
                "    <uses-permission android:name=\"{name}\" />\n"
            ));
        }
    }
    format!(
        "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">\n{declared}    <application android:label=\"{app_name}\" android:theme=\"@android:style/Theme.Material.Light.NoActionBar\">\n        <activity android:name=\"{package}.MainActivity\" android:exported=\"true\">\n            <intent-filter><action android:name=\"android.intent.action.MAIN\"/><category android:name=\"android.intent.category.LAUNCHER\"/></intent-filter>\n        </activity>\n    </application>\n</manifest>\n"
    )
}
fn android_app_gradle(package: &str, remote: bool, navigation: bool) -> String {
    let mut dependencies = String::from(
        "    implementation(platform(\"androidx.compose:compose-bom:2024.12.01\"))\n    implementation(\"androidx.activity:activity-compose:1.10.0\")\n    implementation(\"androidx.compose.ui:ui\")\n    implementation(\"androidx.compose.ui:ui-graphics\")\n    implementation(\"androidx.compose.ui:ui-tooling-preview\")\n    implementation(\"androidx.compose.material3:material3\")\n    debugImplementation(\"androidx.compose.ui:ui-tooling\")\n",
    );
    if navigation {
        dependencies
            .push_str("    implementation(\"androidx.navigation:navigation-compose:2.8.5\")\n");
    }
    if remote {
        dependencies.push_str("    implementation(\"io.coil-kt.coil3:coil-compose:3.6.3\")\n    implementation(\"io.coil-kt.coil3:coil-network-core:3.6.3\")\n    implementation(\"com.google.android.gms:play-services-cronet:18.0.1\")\n    implementation(\"org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0\")\n");
    }
    format!(
        "plugins {{\n    id(\"com.android.application\")\n    id(\"org.jetbrains.kotlin.android\")\n    id(\"org.jetbrains.kotlin.plugin.compose\")\n}}\n\nandroid {{\n    namespace = \"{package}\"\n    compileSdk = 35\n    defaultConfig {{ applicationId = \"{package}\"; minSdk = 26; targetSdk = 35; versionCode = 1; versionName = \"1.0\" }}\n    buildFeatures {{ compose = true }}\n    compileOptions {{ sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }}\n    kotlinOptions {{ jvmTarget = \"17\" }}\n}}\n\ndependencies {{\n{dependencies}}}\n"
    )
}
