//! Optional plugin scaffolding, IDL validation, and native binding generation.

use std::{
    fs,
    path::{Path, PathBuf},
};

mod bindings;
use nexa_plugin_idl::{self as idl, manifest::PluginManifest};

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
                "// Public typed interface declarations for {type_name}.\n// Use `type Name` for value models and `type Error: Error` for typed failures.\n// Add methods here, then implement the matching native methods in both source trees.\n// Add compile-time options in `config`; users set them in generated `nexa.config.nx`.\n\nconfig {{\n    // compiledOption: String\n}}\n\ninterface {type_name} {{\n    // async fn method(input: String) -> String\n}}\n"
            ),
        )?;
    }
    println!("created plugin scaffold {}", output.display());
    Ok(())
}

fn check(args: &[String]) -> Result<(), String> {
    let package = plugin_package(args, "check")?;
    let Some(path) = package.native_path else {
        println!(
            "checked {} (pure Nexa package, manifest schema {})",
            package.root.display(),
            package.manifest.schema
        );
        return Ok(());
    };
    let parsed = idl::parse_file(&path)?;
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

fn generate(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut target = None;
    let mut output = None;
    let mut kotlin_package = "com.nexa.plugin.generated".to_owned();
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
        target.ok_or("`nexa plugin generate` requires `--target swift` or `--target kotlin`")?;
    let parsed = idl::parse_file(&path)?;
    let source = match target {
        "swift" => bindings::swift(&parsed),
        "kotlin" => bindings::kotlin(&parsed, &kotlin_package),
        _ => {
            return Err(format!(
                "unknown target `{target}`; expected `swift` or `kotlin`"
            ));
        }
    };
    let output = output.unwrap_or_else(|| {
        path.with_file_name(match target {
            "swift" => "NexaPluginBindings.swift",
            "kotlin" => "NexaPluginBindings.kt",
            _ => "NexaPluginBindings.swift",
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
        "generate" => "usage: nexa plugin generate <plugin-directory|native.nxid> --target <swift|kotlin> [--package <kotlin.package>] [--out <file>]".to_owned(),
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
        "# {id}\n\nNexa plugin scaffold, version {version}.\n\n## Structure\n\n- `plugin.config.nx` is the package manifest and source of truth for identity and platform source roots.\n- `native.nxid` contains typed native contracts and compile-time config options.\n- `ios/Sources/{type_name}.swift` is the iOS implementation boundary.\n- `android/src/main/kotlin/` contains the Android implementation boundary.\n\nValidate the manifest and native contract with `nexa plugin check .`. Generate direct native contract skeletons with `nexa plugin generate . --target swift` or `--target kotlin`. A local `.nx` app can declare `plugin \"path\" as Namespace`; project generation then includes these platform source trees. Package installation, dependency resolution, and generated implementation methods are not included yet.\n"
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

fn write_if_absent(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}
