//! Optional plugin scaffolding, IDL validation, and native binding generation.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

mod bindings;
mod bindings_cpp;
use nexa_plugin_idl::{self as idl, manifest::PluginManifest};

/// Render the platform contract used by generated native projects. Keeping
/// this entry point in the plugin module makes the CLI command and project
/// scaffolder use the exact same IDL code generator.
pub(crate) fn render_swift_bindings(contract: &idl::PluginIdl) -> String {
    bindings::swift(contract)
}

pub(crate) fn render_kotlin_bindings(contract: &idl::PluginIdl, package: &str) -> String {
    bindings::kotlin(contract, package)
}

pub(crate) fn render_cpp_bindings(contract: &idl::PluginIdl, plugin_id: &str) -> String {
    bindings_cpp::render(contract, plugin_id)
}

pub(crate) fn render_cpp_swift_adapters(
    contract: &idl::PluginIdl,
    plugin_id: &str,
) -> Result<String, String> {
    bindings_cpp::render_swift_adapters(contract, plugin_id)
}

pub(crate) fn render_cpp_android_adapters(
    contract: &idl::PluginIdl,
    plugin_id: &str,
    plugin_namespace: &str,
    package: &str,
    plugin_index: usize,
) -> Result<(String, String), String> {
    bindings_cpp::render_android_adapters(
        contract,
        plugin_id,
        plugin_namespace,
        package,
        plugin_index,
    )
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    match command {
        "init" => init(&args[1..]),
        "check" => check(&args[1..]),
        "generate" => generate(&args[1..]),
        _ => Err(format!("unknown plugin command `{command}`\n\n{}", usage())),
    }
}

fn init(args: &[String]) -> Result<(), String> {
    let mut id = None;
    let mut output = None;
    let mut display_name = None;
    let mut version = "0.1.0".to_owned();
    let mut kind = "native";
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--out" | "--output" | "-o" => {
                cursor += 1;
                output = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a directory")?,
                ));
            }
            "--name" | "-n" => {
                cursor += 1;
                display_name = Some(
                    args.get(cursor)
                        .ok_or("`--name` requires a plugin type name")?
                        .clone(),
                );
            }
            "--version" => {
                cursor += 1;
                version = args
                    .get(cursor)
                    .ok_or("`--version` requires a version")?
                    .clone();
            }
            "--kind" => {
                cursor += 1;
                kind = args
                    .get(cursor)
                    .ok_or("`--kind` requires `pure` or `native`")?
                    .as_str();
                if !matches!(kind, "pure" | "native") {
                    return Err(format!(
                        "unknown plugin kind `{kind}`; expected `pure` or `native`"
                    ));
                }
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if id.is_none() => id = Some(value.to_owned()),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }

    let id = id.ok_or_else(usage)?;
    validate_id(&id)?;
    validate_version(&version)?;
    let type_name = display_name
        .as_deref()
        .map(type_name)
        .unwrap_or_else(|| type_name(id.rsplit('.').next().unwrap_or(&id)));
    if type_name.is_empty() {
        return Err("`--name` must contain at least one letter or digit".to_owned());
    }
    let output = output.unwrap_or_else(|| PathBuf::from(&id));
    fs::create_dir_all(&output).map_err(|error| format!("{}: {error}", output.display()))?;

    let ios_source = format!("ios/Sources/{type_name}.swift");
    let package = package_name(&id);
    let android_source = format!(
        "android/src/main/kotlin/{}/{}.kt",
        package.replace('.', "/"),
        type_name
    );
    write_if_absent(
        &output.join("plugin.config.nx"),
        &manifest(&id, &version, kind),
    )?;
    write_if_absent(
        &output.join("README.md"),
        &readme(&id, &version, &type_name, kind),
    )?;
    if kind == "pure" {
        write_if_absent(
            &output.join("plugin.nx"),
            &format!(
                "// Pure Nexa plugins are composed into the app at compile time.\ncomponent {type_name}Label(text: String) {{\n    body {{\n        Text(text)\n    }}\n}}\n"
            ),
        )?;
        fs::create_dir_all(output.join("assets"))
            .map_err(|error| format!("{}: {error}", output.display()))?;
    } else {
        write_if_absent(&output.join(&ios_source), &ios_stub(&type_name))?;
        write_if_absent(
            &output.join(&android_source),
            &android_stub(&package, &type_name),
        )?;
        write_if_absent(
            &output.join("native.nxid"),
            &format!(
                "// Public typed interface declarations for {type_name}.\n// Describe data crossing the native boundary with a complete struct or enum.\n// Add methods here, then implement the matching native methods in both source trees.\n// Add compile-time options in `config`; users set them in generated `nexa.config.nx`.\n\nconfig {{\n    // compiledOption: String\n}}\n\ninterface {type_name} {{\n    // async fn method(input: String) -> String\n}}\n"
            ),
        )?;
    }
    println!("created plugin scaffold {}", output.display());
    Ok(())
}

fn check(args: &[String]) -> Result<(), String> {
    let package = plugin_package(args, "check")?;
    validate_package_sources(&package)?;
    let Some(path) = package.native_path.as_ref() else {
        println!(
            "checked {} (pure Nexa package, manifest schema {})",
            package.root.display(),
            package.manifest.schema
        );
        return Ok(());
    };
    let parsed = idl::parse_file(&path)?;
    if let Some(result) = typecheck_swift_implementation(&package, &parsed)? {
        println!("{result}");
    }
    if let Some(result) = typecheck_kotlin_implementation(&package, &parsed)? {
        println!("{result}");
    }
    let methods = parsed
        .interfaces
        .iter()
        .map(|interface| interface.methods.len())
        .sum::<usize>();
    println!(
        "checked {} (manifest schema {}, {} type(s), {} interface(s), {} method(s))",
        path.display(),
        package.manifest.schema,
        parsed.types.len(),
        parsed.interfaces.len(),
        methods
    );
    Ok(())
}

fn typecheck_swift_implementation(
    package: &PluginPackage,
    contract: &idl::PluginIdl,
) -> Result<Option<String>, String> {
    if !package.manifest.ios.swift_packages.is_empty() {
        return Ok(Some(
            "Swift implementation type check skipped: declared Swift packages need Xcode project resolution".to_owned(),
        ));
    }
    if !package.manifest.ios.xcframeworks.is_empty() {
        return Ok(Some(
            "Swift implementation type check skipped: declared XCFramework dependencies need Xcode project integration".to_owned(),
        ));
    }

    let package_root = fs::canonicalize(&package.root)
        .map_err(|error| format!("{}: {error}", package.root.display()))?;
    let sources = crate::project::plugin_platform_sources(&package_root, &package.manifest, true)?;
    if sources.is_empty() {
        return Ok(None);
    }

    let swiftc = Command::new("xcrun")
        .args(["--sdk", "iphonesimulator", "--find", "swiftc"])
        .output();
    let Ok(swiftc) = swiftc else {
        return Ok(Some(
            "Swift implementation type check skipped: Xcode command line tools are unavailable"
                .to_owned(),
        ));
    };
    if !swiftc.status.success() {
        return Ok(Some(
            "Swift implementation type check skipped: iOS simulator SDK is unavailable".to_owned(),
        ));
    }
    let swiftc = String::from_utf8_lossy(&swiftc.stdout).trim().to_owned();
    let sdk = Command::new("xcrun")
        .args(["--sdk", "iphonesimulator", "--show-sdk-path"])
        .output()
        .map_err(|error| format!("could not query the iOS simulator SDK: {error}"))?;
    if !sdk.status.success() {
        return Ok(Some(
            "Swift implementation type check skipped: iOS simulator SDK is unavailable".to_owned(),
        ));
    }
    let sdk = String::from_utf8_lossy(&sdk.stdout).trim().to_owned();

    let temporary = TempCheckDirectory::new()?;
    let binding_path = temporary.0.join("NexaPluginBindings.swift");
    fs::write(&binding_path, bindings::swift(contract))
        .map_err(|error| format!("{}: {error}", binding_path.display()))?;
    let minimum = package
        .manifest
        .ios
        .min_version
        .as_deref()
        .unwrap_or("17.0");
    let target = format!("arm64-apple-ios{minimum}-simulator");
    let output = Command::new(swiftc)
        .arg("-typecheck")
        .arg("-swift-version")
        .arg("6")
        .arg("-target")
        .arg(target)
        .arg("-sdk")
        .arg(sdk)
        .arg(&binding_path)
        .args(&sources)
        .current_dir(&package_root)
        .output()
        .map_err(|error| format!("could not run Swift plugin type check: {error}"))?;
    if !output.status.success() {
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Swift plugin implementation does not conform to its generated contract:\n{}",
            diagnostics.trim()
        ));
    }

    Ok(Some(
        "Swift implementation type check passed against the generated contract".to_owned(),
    ))
}

fn typecheck_kotlin_implementation(
    package: &PluginPackage,
    contract: &idl::PluginIdl,
) -> Result<Option<String>, String> {
    let package_root = fs::canonicalize(&package.root)
        .map_err(|error| format!("{}: {error}", package.root.display()))?;
    let sources = crate::project::plugin_platform_sources(&package_root, &package.manifest, false)?;
    if sources.is_empty() {
        return Ok(None);
    }
    if !package.manifest.android.maven_dependencies.is_empty()
        || !package.manifest.android.aars.is_empty()
        || contract
            .interfaces
            .iter()
            .any(|interface| interface.kind == idl::InterfaceKind::NativeComponent)
    {
        return Ok(Some(
            "Kotlin implementation type check skipped: declared Android dependencies or native components require the generated Gradle project".to_owned(),
        ));
    }

    let compiler = match Command::new("kotlinc").arg("-version").output() {
        Ok(output) if output.status.success() => "kotlinc",
        _ => {
            return Ok(Some(
                "Kotlin implementation type check skipped: kotlinc is unavailable".to_owned(),
            ));
        }
    };

    let mut package_name = None;
    for source in &sources {
        let contents =
            fs::read_to_string(source).map_err(|error| format!("{}: {error}", source.display()))?;
        for line in contents.lines().map(str::trim) {
            if let Some(import) = line.strip_prefix("import ") {
                let import = import.split_whitespace().next().unwrap_or_default();
                if !import.starts_with("java.")
                    && !import.starts_with("javax.")
                    && !import.starts_with("kotlin.")
                {
                    return Ok(Some(format!(
                        "Kotlin implementation type check skipped: external import `{import}` requires the generated Gradle project"
                    )));
                }
            }
            if package_name.is_none()
                && let Some(declared) = line.strip_prefix("package ")
            {
                let declared = declared.trim();
                if !declared.is_empty() {
                    package_name = Some(declared.to_owned());
                }
            }
        }
    }
    let Some(package_name) = package_name else {
        return Err("Kotlin plugin sources must declare an implementation package".to_owned());
    };

    let temporary = TempCheckDirectory::new()?;
    let binding_path = temporary.0.join("NexaPluginBindings.kt");
    fs::write(&binding_path, bindings::kotlin(contract, &package_name))
        .map_err(|error| format!("{}: {error}", binding_path.display()))?;
    let output_path = temporary.0.join("plugin.jar");
    let output = Command::new(compiler)
        .arg(&binding_path)
        .args(&sources)
        .arg("-d")
        .arg(&output_path)
        .current_dir(&package_root)
        .output()
        .map_err(|error| format!("could not run Kotlin plugin type check: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Kotlin plugin implementation does not conform to its generated contract:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(Some(
        "Kotlin implementation type check passed against the generated contract".to_owned(),
    ))
}

struct TempCheckDirectory(PathBuf);

impl TempCheckDirectory {
    fn new() -> Result<Self, String> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "nexa-plugin-swift-check-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for TempCheckDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn validate_package_sources(package: &PluginPackage) -> Result<(), String> {
    crate::project::validate_plugin_manifest_sources(&package.root, &package.manifest)?;
    if let Some(source) = &package.manifest.nexa {
        let path = package.root.join(source);
        validate_nexa_source_tree(
            &path,
            &mut std::collections::HashSet::new(),
            &mut std::collections::HashSet::new(),
        )?;
    }
    Ok(())
}

fn validate_nexa_source_tree(
    path: &Path,
    active: &mut std::collections::HashSet<PathBuf>,
    visited: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve Nexa source {}: {error}", path.display()))?;
    if visited.contains(&canonical) {
        return Ok(());
    }
    if !active.insert(canonical.clone()) {
        return Err(format!(
            "cyclic Nexa source import involving {}",
            canonical.display()
        ));
    }

    let result = (|| {
        let source = fs::read_to_string(&canonical)
            .map_err(|error| format!("{}: {error}", canonical.display()))?;
        let program = nexa_syntax::parse_program(&source)
            .map_err(|error| format!("{}: {error}", canonical.display()))?;
        for import in program.imports {
            let imported = canonical
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(import.path);
            validate_nexa_source_tree(&imported, active, visited)?;
        }
        Ok(())
    })();

    active.remove(&canonical);
    if result.is_ok() {
        visited.insert(canonical);
    }
    result
}

fn generate(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut target = None;
    let mut output = None;
    let mut kotlin_package = "com.nexa.plugin.generated".to_owned();
    let mut package_was_provided = false;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--target" | "-t" => {
                cursor += 1;
                target = Some(
                    args.get(cursor)
                        .ok_or("`--target` requires `swift` or `kotlin`")?
                        .as_str(),
                );
            }
            "--out" | "--output" | "-o" => {
                cursor += 1;
                output = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a file path")?,
                ));
            }
            "--package" => {
                cursor += 1;
                package_was_provided = true;
                kotlin_package = args
                    .get(cursor)
                    .ok_or("`--package` requires a Kotlin package name")?
                    .clone();
                validate_package(&kotlin_package)?;
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if path.is_none() => path = Some(PathBuf::from(value)),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }
    let package = plugin_package_from(path.ok_or_else(|| usage_for("generate"))?)?;
    let path = package.native_path.clone().ok_or_else(|| {
        format!(
            "plugin package `{}` has no native.nxid contract",
            package.root.display()
        )
    })?;
    let target =
        target.ok_or("`nexa plugin generate` requires `--target swift`, `kotlin`, or `cpp`")?;
    if target == "cpp" && package_was_provided {
        return Err("`--package` is supported only with `--target kotlin`".to_owned());
    }
    let parsed = idl::parse_file(&path)?;
    let source = match target {
        "swift" => bindings::swift(&parsed),
        "kotlin" => bindings::kotlin(&parsed, &kotlin_package),
        "cpp" => bindings_cpp::render(&parsed, &package.manifest.id),
        _ => {
            return Err(format!(
                "unknown target `{target}`; expected `swift`, `kotlin`, or `cpp`"
            ));
        }
    };
    let output = output.unwrap_or_else(|| {
        path.with_file_name(match target {
            "swift" => "NexaPluginBindings.swift",
            "kotlin" => "NexaPluginBindings.kt",
            "cpp" => "NexaPluginBindings.hpp",
            _ => unreachable!("target was validated before choosing the default output"),
        })
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(&output, source).map_err(|error| format!("{}: {error}", output.display()))?;
    println!("generated {} ({target})", output.display());
    Ok(())
}

fn plugin_package(args: &[String], command: &str) -> Result<PluginPackage, String> {
    let mut path = None;
    for argument in args {
        if argument.starts_with('-') {
            return Err(format!("unknown option `{argument}`"));
        }
        if path.is_some() {
            return Err(format!("unexpected argument `{argument}`"));
        }
        path = Some(PathBuf::from(argument));
    }
    plugin_package_from(path.ok_or_else(|| usage_for(command))?)
}

struct PluginPackage {
    root: PathBuf,
    manifest: PluginManifest,
    native_path: Option<PathBuf>,
}

fn plugin_package_from(path: PathBuf) -> Result<PluginPackage, String> {
    let root = if path.is_dir() {
        path
    } else {
        path.parent()
            .ok_or_else(|| format!("plugin path has no package directory: {}", path.display()))?
            .to_path_buf()
    };
    let manifest_path = root.join("plugin.config.nx");
    let manifest = idl::manifest::parse_file(&manifest_path)?;
    let native_path = manifest.native.as_ref().map(|relative| root.join(relative));
    if let Some(native_path) = &native_path {
        if !native_path.is_file() {
            return Err(format!(
                "plugin native contract does not exist: {}",
                native_path.display()
            ));
        }
    }
    Ok(PluginPackage {
        root,
        manifest,
        native_path,
    })
}

fn usage() -> String {
    format!(
        "{}\n{}\n{}",
        usage_for("init"),
        usage_for("check"),
        usage_for("generate")
    )
}

fn usage_for(command: &str) -> String {
    match command {
        "init" => "usage: nexa plugin init <plugin.id> [--kind <pure|native>] [--out <directory>] [--name <TypeName>] [--version <version>]".to_owned(),
        "check" => "usage: nexa plugin check <plugin-directory|native.nxid>".to_owned(),
        "generate" => "usage: nexa plugin generate <plugin-directory|native.nxid> --target <swift|kotlin|cpp> [--package <kotlin.package>] [--out <file>]".to_owned(),
        _ => "usage: nexa plugin <init|check|generate> ...".to_owned(),
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.split('.').any(|part| {
            part.is_empty()
                || !part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
        })
    {
        return Err(
            "plugin id must contain non-empty dot-separated identifier segments".to_owned(),
        );
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<(), String> {
    if version.is_empty() || version.chars().any(char::is_whitespace) {
        return Err("plugin version must be a non-empty value without whitespace".to_owned());
    }
    Ok(())
}

fn validate_package(package: &str) -> Result<(), String> {
    if package.is_empty()
        || package.split('.').any(|part| {
            part.is_empty()
                || !part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
                || part.starts_with(|character: char| character.is_ascii_digit())
        })
    {
        return Err(format!("invalid Kotlin package `{package}`"));
    }
    Ok(())
}

fn type_name(value: &str) -> String {
    let mut result = String::new();
    let mut uppercase = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if uppercase {
                result.extend(character.to_uppercase());
                uppercase = false;
            } else {
                result.push(character);
            }
        } else {
            uppercase = true;
        }
    }
    if result.starts_with(|character: char| character.is_ascii_digit()) {
        result.insert(0, 'N');
    }
    result
}

fn package_name(id: &str) -> String {
    format!(
        "com.nexa.plugin.{}",
        id.replace(['.', '-'], "").to_ascii_lowercase()
    )
}

fn manifest(id: &str, version: &str, kind: &str) -> String {
    if kind == "pure" {
        return format!(
            "plugin {{\n    schema: 2\n    id: \"{id}\"\n    version: \"{version}\"\n    sources {{\n        nexa: \"plugin.nx\"\n    }}\n    assets: [\"assets/**\"]\n}}\n"
        );
    }
    format!(
        "plugin {{\n    schema: 2\n    id: \"{id}\"\n    version: \"{version}\"\n    sources {{\n        native: \"native.nxid\"\n    }}\n    ios {{\n        minVersion: \"17.0\"\n        sources: [\"ios/Sources/**\"]\n    }}\n    android {{\n        minSdk: 26\n        sources: [\"android/src/main/kotlin/**\"]\n    }}\n}}\n",
    )
}

fn readme(id: &str, version: &str, type_name: &str, kind: &str) -> String {
    if kind == "pure" {
        return format!(
            "# {id}\n\nPure Nexa plugin, version {version}.\n\n`plugin.nx` is loaded into the app's compile-time source graph. Add reusable components, logic, and imports there; place referenced platform assets in `assets/`. Reachability pruning removes unused plugin components from native output.\n"
        );
    }
    format!(
        "# {id}\n\nNexa plugin scaffold, version {version}.\n\n## Structure\n\n- `plugin.config.nx` is the package manifest and source of truth for identity and platform source roots.\n- `native.nxid` contains typed native contracts and compile-time config options.\n- `ios/Sources/{type_name}.swift` is the iOS implementation boundary.\n- `android/src/main/kotlin/` contains the Android implementation boundary.\n\nValidate the manifest and native contract with `nexa plugin check .`. Generate native contract skeletons with `nexa plugin generate . --target swift` or `--target kotlin`; `--target cpp` emits the optional C++ contract header. C++ host interop must be explicitly configured and is not enabled by this scaffold yet. A local `.nx` app can declare `plugin \"path\" as Namespace`; project generation then includes these platform source trees. Package installation and dependency resolution are not included yet.\n"
    )
}

fn ios_stub(type_name: &str) -> String {
    format!(
        "import Foundation\n\npublic enum {type_name}PluginError: Error {{\n    case unavailable\n}}\n\npublic final class {type_name}Plugin: @unchecked Sendable {{\n    public static let shared = {type_name}Plugin()\n    public init() {{}}\n}}\n"
    )
}

fn android_stub(package: &str, type_name: &str) -> String {
    format!(
        "package {package}\n\nobject {type_name}Plugin {{\n    val instance: {type_name}Plugin = this\n}}\n"
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        PluginPackage, idl, typecheck_kotlin_implementation, typecheck_swift_implementation,
        validate_package_sources,
    };

    struct TempPackage(PathBuf);

    static NEXT_TEMP_PACKAGE: AtomicU64 = AtomicU64::new(0);

    impl TempPackage {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after the epoch")
                .as_nanos();
            let sequence = NEXT_TEMP_PACKAGE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "nexa-plugin-check-{}-{unique}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("temporary package should be created");
            Self(root)
        }
    }

    impl Drop for TempPackage {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn plugin_check_parses_declared_nexa_source_with_package_context() {
        let package_root = TempPackage::new();
        fs::create_dir(package_root.0.join("assets")).expect("asset directory should be created");
        fs::write(package_root.0.join("plugin.nx"), "fn invalid(")
            .expect("invalid plugin source should be written");
        let manifest = idl::manifest::parse(
            r#"plugin {
                schema: 2
                id: "dev.nexa.pure"
                version: "1.0.0"
                sources { nexa: "plugin.nx" }
                assets: ["assets/**"]
            }"#,
        )
        .expect("valid pure-plugin manifest");
        let package = PluginPackage {
            root: package_root.0.clone(),
            manifest,
            native_path: None,
        };

        let error = validate_package_sources(&package)
            .expect_err("malformed declared plugin source should fail check");
        assert!(error.contains("plugin.nx:"));
    }

    #[test]
    fn plugin_check_rejects_declared_native_source_globs_without_matches() {
        let package_root = TempPackage::new();
        fs::create_dir_all(package_root.0.join("ios/Sources"))
            .expect("iOS source root should be created");
        let manifest = idl::manifest::parse(
            r#"plugin {
                schema: 2
                id: "dev.nexa.native"
                version: "1.0.0"
                sources { native: "native.nxid" }
                ios { sources: ["ios/Sources/**/*.swift"] }
            }"#,
        )
        .expect("valid native-plugin manifest");
        let package = PluginPackage {
            root: package_root.0.clone(),
            manifest,
            native_path: Some(package_root.0.join("native.nxid")),
        };

        let error = validate_package_sources(&package)
            .expect_err("declared implementation glob should resolve to source files");
        assert!(error.contains("matches no .swift files"));
    }

    #[test]
    fn kotlin_plugin_check_defers_declared_android_dependencies_to_gradle() {
        let package_root = TempPackage::new();
        let source = package_root
            .0
            .join("android/src/main/kotlin/VideoPlayerImpl.kt");
        fs::create_dir_all(source.parent().expect("Kotlin source has a parent"))
            .expect("Kotlin source directory should be created");
        fs::write(&source, "package dev.example.video\nclass VideoPlayerImpl")
            .expect("Kotlin source should be written");
        let aar = package_root.0.join("android/libs/player.aar");
        fs::create_dir_all(aar.parent().expect("AAR has a parent"))
            .expect("AAR directory should be created");
        fs::write(&aar, b"AAR fixture").expect("AAR fixture should be written");
        let manifest = idl::manifest::parse(
            r#"plugin {
                schema: 2
                id: "dev.example.video"
                version: "1.0.0"
                sources { native: "native.nxid" }
                android {
                    sources: ["android/src/main/kotlin/**"]
                    aars: ["android/libs/player.aar"]
                }
            }"#,
        )
        .expect("valid Android plugin manifest");
        let package = PluginPackage {
            root: package_root.0.clone(),
            manifest,
            native_path: None,
        };
        let contract = idl::parse("native class VideoPlayer { init() fn play() }")
            .expect("valid native class contract");

        validate_package_sources(&package).expect("declared AAR should resolve in the package");
        let result = typecheck_kotlin_implementation(&package, &contract)
            .expect("declared AAR should produce a skip, not a failure");
        assert_eq!(
            result.as_deref(),
            Some(
                "Kotlin implementation type check skipped: declared Android dependencies or native components require the generated Gradle project"
            )
        );
    }

    #[test]
    fn swift_plugin_check_defers_declared_xcframeworks_to_xcode() {
        let package_root = TempPackage::new();
        let framework = package_root.0.join("ios/Vendor.xcframework");
        fs::create_dir_all(&framework).expect("XCFramework directory should be created");
        let manifest = idl::manifest::parse(
            r#"plugin {
                schema: 2
                id: "dev.example.vendor"
                version: "1.0.0"
                sources { native: "native.nxid" }
                ios { xcframeworks: ["ios/Vendor.xcframework"] }
            }"#,
        )
        .expect("valid iOS plugin manifest");
        let package = PluginPackage {
            root: package_root.0.clone(),
            manifest,
            native_path: None,
        };
        validate_package_sources(&package)
            .expect("declared XCFramework should resolve within the package");
        let contract = idl::parse("service Vendor { fn version() -> String }")
            .expect("valid service contract");

        let result = typecheck_swift_implementation(&package, &contract)
            .expect("declared XCFramework should produce a skip, not a failure");
        assert_eq!(
            result.as_deref(),
            Some(
                "Swift implementation type check skipped: declared XCFramework dependencies need Xcode project integration"
            )
        );
    }

    #[test]
    fn recursive_native_source_globs_include_matching_files() {
        let package_root = TempPackage::new();
        let source = package_root.0.join("ios/Sources/nested/Push.swift");
        fs::create_dir_all(source.parent().expect("Swift source has a parent"))
            .expect("nested Swift source directory should be created");
        fs::write(&source, "public struct Push {}\n").expect("Swift source should be written");
        let manifest = idl::manifest::parse(
            r#"plugin {
                schema: 2
                id: "dev.nexa.native"
                version: "1.0.0"
                sources { native: "native.nxid" }
                ios { sources: ["ios/Sources/**/*.swift"] }
            }"#,
        )
        .expect("valid native-plugin manifest");
        let package = PluginPackage {
            root: package_root.0.clone(),
            manifest: manifest.clone(),
            native_path: Some(package_root.0.join("native.nxid")),
        };

        validate_package_sources(&package)
            .expect("recursive source patterns should resolve nested files");
        let resolved = crate::project::plugin_platform_sources(&package_root.0, &manifest, true)
            .expect("project source resolution should succeed");
        assert_eq!(resolved, vec![source]);
    }

    #[test]
    fn plugin_check_resolves_imports_from_the_declared_nexa_source() {
        let package_root = TempPackage::new();
        fs::write(package_root.0.join("plugin.nx"), "import \"missing.nx\"\n")
            .expect("plugin source should be written");
        let manifest = idl::manifest::parse(
            r#"plugin {
                schema: 2
                id: "dev.nexa.pure"
                version: "1.0.0"
                sources { nexa: "plugin.nx" }
            }"#,
        )
        .expect("valid pure-plugin manifest");
        let package = PluginPackage {
            root: package_root.0.clone(),
            manifest,
            native_path: None,
        };

        let error = validate_package_sources(&package)
            .expect_err("missing imported source should fail package check");
        assert!(
            error.contains("missing.nx"),
            "unexpected diagnostic: {error}"
        );
    }
}

fn write_if_absent(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}
