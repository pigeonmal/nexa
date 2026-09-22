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
use nexa_compiler::{Target, compile_file_with_warnings_for_targets};
use nexa_ir::Module;
use nexa_syntax::ast::ConfigValue;

use crate::{cache, config, config::ProjectConfig};

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

    let inferred_name = source_app_name(&input).unwrap_or_else(|| "NexaApp".to_owned());
    let requested_name = name.as_deref().unwrap_or(&inferred_name);
    let app_name = type_name(requested_name);
    if app_name.is_empty() {
        return Err("`--name` must contain at least one letter or digit".to_owned());
    }
    let project_target = match target {
        ProjectTarget::Ios => "project-ios",
        ProjectTarget::Android => "project-android",
        ProjectTarget::All => "project-all",
    };
    let config_path = output.join("nexa.config.nx");
    let plugin_definitions = config::load_plugin_definitions(&input)?;
    let existing_config = config_path.is_file();
    let project_config = if existing_config {
        Some(ProjectConfig::parse_file(
            &config_path,
            &plugin_definitions,
        )?)
    } else {
        None
    };
    let mut cache_key = cache::key_with_extra(&input, project_target, &[config_path.as_path()])
        .map_err(|error| format!("project cache: {error}"))?;
    if project_cache_is_current(&output, &app_name, target, &cache_key) {
        if let Some(warnings) = cache::restore_warnings(&input, &cache_key)
            .map_err(|error| format!("project cache: {error}"))?
        {
            for warning in &warnings {
                eprintln!("{warning}");
            }
            if deny_warnings && !warnings.is_empty() {
                return Err(format!("{} warning(s) treated as errors", warnings.len()));
            }
            println!("generated {} (cache hit)", output.display());
            return Ok(());
        }
    }

    let targets: &[Target] = match target {
        ProjectTarget::Ios => &[Target::Swift],
        ProjectTarget::Android => &[Target::Kotlin],
        ProjectTarget::All => &[Target::Swift, Target::Kotlin],
    };
    let compilations = compile_file_with_warnings_for_targets(&input, targets)
        .map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    let compiled = targets
        .iter()
        .copied()
        .zip(compilations)
        .map(|(target, compilation)| {
            warnings.extend(compilation.warnings);
            (target, compilation.module)
        })
        .collect::<Vec<_>>();
    let warnings = super::deduplicate_warnings(warnings);
    super::report_warnings(&warnings, deny_warnings)?;

    let project_config = match project_config {
        Some(config) => config,
        None => match ProjectConfig::from_defaults(&plugin_definitions) {
            Ok(config) => config,
            Err(error) => {
                write_if_changed(&config_path, &config::render_template(&plugin_definitions))?;
                return Err(format!(
                    "{error}; edit {} and run `nexa generate` again",
                    config_path.display()
                ));
            }
        },
    };
    if !existing_config {
        write_if_changed(&config_path, &project_config.render())?;
        cache_key = cache::key_with_extra(&input, project_target, &[config_path.as_path()])
            .map_err(|error| format!("project cache: {error}"))?;
    }

    fs::create_dir_all(&output).map_err(|error| format!("{}: {error}", output.display()))?;

    let mut generated_targets = Vec::new();
    for (compile_target, mut module) in compiled {
        module.app_name = app_name.clone();
        match compile_target {
            Target::Swift => {
                generate_ios(&output, &app_name, &module, &project_config)?;
                generated_targets.push("ios");
            }
            Target::Kotlin => {
                generate_android(&output, &app_name, &module, &project_config)?;
                generated_targets.push("android");
            }
            Target::All => unreachable!("project generation compiles concrete platform targets"),
        }
    }

    let entry = input.canonicalize().unwrap_or(input.clone());
    let manifest = format!(
        "{{\n  \"format\": 1,\n  \"entry\": \"{}\",\n  \"name\": \"{}\",\n  \"targets\": [{}],\n  \"cacheKey\": \"{}\"\n}}\n",
        json_escape(&entry.display().to_string()),
        json_escape(&app_name),
        generated_targets
            .iter()
            .map(|target| format!("\"{target}\""))
            .collect::<Vec<_>>()
            .join(", "),
        cache_key,
    );
    write_if_changed(&output.join("nexa.project.json"), &manifest)?;
    write_if_changed(
        &output.join("README.md"),
        &root_readme(&app_name, &generated_targets),
    )?;
    let warning_text = warnings.iter().map(ToString::to_string).collect::<Vec<_>>();
    if let Err(error) = cache::store_warnings(&input, &cache_key, &warning_text) {
        eprintln!("warning: could not update project cache: {error}");
    }
    println!(
        "generated {} ({})",
        output.display(),
        generated_targets.join(", ")
    );
    Ok(())
}

fn generate_ios(
    root: &Path,
    app_name: &str,
    module: &Module,
    config: &ProjectConfig,
) -> Result<(), String> {
    let directory = root.join("ios").join(app_name);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let screen = nexa_codegen::names::screen_name(app_name);
    let source = ios_generated_source(module, config)?;
    write_if_changed(&directory.join("NexaGenerated.swift"), &source)?;
    write_if_changed(
        &directory.join(format!("{app_name}App.swift")),
        &format!(
            "import SwiftUI\n\n@main\nstruct {app_name}App: App {{\n    var body: some Scene {{\n        WindowGroup {{\n            {screen}()\n        }}\n    }}\n}}\n"
        ),
    )?;
    write_if_changed(
        &directory.join("Info.plist"),
        &ios_info_plist(app_name, config),
    )?;
    write_if_changed(
        &root
            .join("ios")
            .join(format!("{app_name}.xcodeproj/project.pbxproj")),
        &ios_project_file(app_name),
    )?;
    write_if_changed(
        &root.join("ios").join(format!(
            "{app_name}.xcodeproj/xcshareddata/xcschemes/{app_name}.xcscheme"
        )),
        &ios_scheme(app_name),
    )?;
    Ok(())
}

fn generate_android(
    root: &Path,
    app_name: &str,
    module: &Module,
    config: &ProjectConfig,
) -> Result<(), String> {
    let package = package_name(app_name);
    let package_path = package.replace('.', "/");
    let source_dir = root.join("android/app/src/main/java").join(&package_path);
    fs::create_dir_all(&source_dir)
        .map_err(|error| format!("{}: {error}", source_dir.display()))?;
    let generated = KotlinBackend.generate(module);
    copy_android_plugin_sources(root, module, &package, config)?;
    let screen = nexa_codegen::names::screen_name(app_name);
    let uses_network = generated.contains("NexaNetwork") || generated.contains("org.chromium.net");
    let uses_remote_image =
        generated.contains("coil3.compose.AsyncImage") || generated.contains("coil3.network");
    let uses_compose_graphics = generated.contains("androidx.compose.ui.graphics.");
    let uses_lifecycle_events = generated.contains("LocalLifecycleOwner");
    let uses_coroutines = generated.contains("kotlinx.coroutines.");
    let cronet_import = if uses_network {
        "import com.google.android.gms.net.CronetProviderInstaller\n"
    } else {
        ""
    };
    let permission_callback = if generated.contains("NexaPermissions.request") {
        "    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<String>, grantResults: IntArray) {\n        super.onRequestPermissionsResult(requestCode, permissions, grantResults)\n        NexaRuntime.dispatchPermissionResult(requestCode, grantResults)\n    }\n"
    } else {
        ""
    };
    let content_setup = if uses_network {
        format!(
            "        CronetProviderInstaller.installProvider(this).addOnCompleteListener {{\n            setContent {{ MaterialTheme {{ {screen}() }} }}\n        }}\n"
        )
    } else {
        format!("        setContent {{ MaterialTheme {{ {screen}() }} }}\n")
    };
    write_if_changed(
        &source_dir.join("NexaGenerated.kt"),
        &format!(
            "package {package}\n\n{generated}{}",
            render_kotlin_plugin_config(module, config)
        ),
    )?;
    write_if_changed(
        &source_dir.join("MainActivity.kt"),
        &format!(
            "package {package}\n\nimport android.os.Bundle\nimport androidx.activity.ComponentActivity\nimport androidx.activity.compose.setContent\nimport androidx.compose.material3.MaterialTheme\n{cronet_import}\nclass MainActivity : ComponentActivity() {{\n    override fun onCreate(savedInstanceState: Bundle?) {{\n        super.onCreate(savedInstanceState)\n{content_setup}    }}\n{permission_callback}}}\n"
        ),
    )?;
    write_if_changed(
        &root.join("android/app/src/main/AndroidManifest.xml"),
        &android_manifest(app_name, &package, uses_network, config),
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
        &android_app_gradle(
            &package,
            uses_network,
            uses_remote_image,
            uses_coroutines,
            generated.contains("NavHost"),
            uses_compose_graphics,
            uses_lifecycle_events,
        ),
    )?;
    write_if_changed(
        &root.join("android/app/proguard-rules.pro"),
        android_proguard_rules(),
    )?;
    Ok(())
}

fn ios_generated_source(module: &Module, config: &ProjectConfig) -> Result<String, String> {
    let generated = SwiftBackend.generate(module);
    if module.plugins.is_empty() {
        return Ok(generated);
    }
    let mut imports = vec!["import Foundation".to_owned()];
    let mut declarations = Vec::new();
    for line in generated.lines() {
        if line.trim_start().starts_with("import ") {
            imports.push(line.to_owned());
        } else {
            declarations.push(line.to_owned());
        }
    }
    let plugin_config = render_swift_plugin_config(module, config);
    if !plugin_config.is_empty() {
        declarations.push(plugin_config);
    }
    for plugin in &module.plugins {
        for path in native_plugin_sources(plugin, "ios/Sources", "swift")? {
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            for line in contents.lines() {
                if line.trim_start().starts_with("import ") {
                    imports.push(line.to_owned());
                } else {
                    declarations.push(line.to_owned());
                }
            }
            declarations.push(format!("// Nexa plugin: {}", plugin.namespace));
        }
    }
    imports.sort();
    imports.dedup();
    let mut source = imports.join("\n");
    source.push_str("\n\n");
    source.push_str(&declarations.join("\n"));
    source.push('\n');
    Ok(source)
}

fn copy_android_plugin_sources(
    root: &Path,
    module: &Module,
    app_package: &str,
    config: &ProjectConfig,
) -> Result<(), String> {
    let destination = root.join("android/app/src/main/java");
    let has_plugin_config = config.plugins().any(|plugin| {
        !plugin.options.is_empty()
            && module
                .plugins
                .iter()
                .any(|used| used.namespace == plugin.namespace)
    });
    for plugin in &module.plugins {
        for path in native_plugin_sources(plugin, "android/src/main/kotlin", "kt")? {
            let plugin_root = Path::new(&plugin.idl_path)
                .parent()
                .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
            let source_root = plugin_root.join("android/src/main/kotlin");
            let relative = path.strip_prefix(&source_root).map_err(|_| {
                format!(
                    "plugin source is outside its Kotlin source root: {}",
                    path.display()
                )
            })?;
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            let contents = if has_plugin_config {
                kotlin_plugin_config_import(&contents, app_package)
            } else {
                contents
            };
            write_if_changed(&destination.join(relative), &contents)?;
        }
    }
    Ok(())
}

fn render_swift_plugin_config(module: &Module, config: &ProjectConfig) -> String {
    let plugins = config
        .plugins()
        .filter(|plugin| {
            !plugin.options.is_empty()
                && module
                    .plugins
                    .iter()
                    .any(|used| used.namespace == plugin.namespace)
        })
        .collect::<Vec<_>>();
    if plugins.is_empty() {
        return String::new();
    }
    let mut output = String::from("internal enum NexaPluginConfig {\n");
    for plugin in plugins {
        output.push_str(&format!("    internal enum {} {{\n", plugin.namespace));
        for option in &plugin.options {
            output.push_str(&format!(
                "        internal static let {}: {} = {}\n",
                option.name,
                swift_config_type(&option.ty),
                swift_config_value(&option.value)
            ));
        }
        output.push_str("    }\n");
    }
    output.push_str("}\n");
    output
}

fn render_kotlin_plugin_config(module: &Module, config: &ProjectConfig) -> String {
    let plugins = config
        .plugins()
        .filter(|plugin| {
            !plugin.options.is_empty()
                && module
                    .plugins
                    .iter()
                    .any(|used| used.namespace == plugin.namespace)
        })
        .collect::<Vec<_>>();
    if plugins.is_empty() {
        return String::new();
    }
    let mut output = String::from("\ninternal object NexaPluginConfig {\n");
    for plugin in plugins {
        output.push_str(&format!("    internal object {} {{\n", plugin.namespace));
        for option in &plugin.options {
            let modifier = if option.ty.optional {
                "val"
            } else {
                "const val"
            };
            output.push_str(&format!(
                "        internal {modifier} {}: {} = {}\n",
                option.name,
                kotlin_config_type(&option.ty),
                kotlin_config_value(&option.value, &option.ty)
            ));
        }
        output.push_str("    }\n");
    }
    output.push_str("}\n");
    output
}

fn swift_config_type(ty: &nexa_plugin_idl::TypeRef) -> String {
    let name = match ty.name.as_str() {
        "String" => "String",
        "Bool" => "Bool",
        "Int8" => "Int8",
        "Int16" => "Int16",
        "Int32" => "Int32",
        "Int64" => "Int64",
        "UInt8" => "UInt8",
        "UInt16" => "UInt16",
        "UInt32" => "UInt32",
        "UInt64" => "UInt64",
        "Float32" => "Float",
        "Float64" => "Double",
        _ => "String",
    };
    if ty.optional {
        format!("{name}?")
    } else {
        name.to_owned()
    }
}

fn kotlin_config_type(ty: &nexa_plugin_idl::TypeRef) -> String {
    let name = match ty.name.as_str() {
        "String" => "String",
        "Bool" => "Boolean",
        "Int8" => "Byte",
        "Int16" => "Short",
        "Int32" => "Int",
        "Int64" => "Long",
        "UInt8" => "UByte",
        "UInt16" => "UShort",
        "UInt32" => "UInt",
        "UInt64" => "ULong",
        "Float32" => "Float",
        "Float64" => "Double",
        _ => "String",
    };
    if ty.optional {
        format!("{name}?")
    } else {
        name.to_owned()
    }
}

fn swift_config_value(value: &ConfigValue) -> String {
    match value {
        ConfigValue::String(value) => format!("\"{}\"", swift_string_escape(value)),
        ConfigValue::Number(value) => value.clone(),
        ConfigValue::Bool(value) => value.to_string(),
        ConfigValue::Null => "nil".to_owned(),
        ConfigValue::Array(_) => "[]".to_owned(),
    }
}

fn kotlin_config_value(value: &ConfigValue, ty: &nexa_plugin_idl::TypeRef) -> String {
    match value {
        ConfigValue::String(value) => format!("\"{}\"", kotlin_string_escape(value)),
        ConfigValue::Number(value) => match ty.name.as_str() {
            "Float32" if !value.ends_with('f') && !value.ends_with('F') => {
                format!("{value}f")
            }
            "UInt8" | "UInt16" | "UInt32" if !value.ends_with('u') && !value.ends_with('U') => {
                format!("{value}u")
            }
            "UInt64" if !value.ends_with("uL") && !value.ends_with("UL") => {
                format!("{value}uL")
            }
            _ => value.clone(),
        },
        ConfigValue::Bool(value) => value.to_string(),
        ConfigValue::Null => "null".to_owned(),
        ConfigValue::Array(_) => "emptyList()".to_owned(),
    }
}

fn swift_string_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn kotlin_string_escape(value: &str) -> String {
    swift_string_escape(value)
}

fn kotlin_plugin_config_import(contents: &str, app_package: &str) -> String {
    let import = format!("import {app_package}.NexaPluginConfig");
    if contents.lines().any(|line| line.trim() == import) {
        return contents.to_owned();
    }
    let mut lines = contents.lines();
    let Some(package_line) = lines.next() else {
        return contents.to_owned();
    };
    if !package_line.trim_start().starts_with("package ") {
        return contents.to_owned();
    }
    let remainder = lines.collect::<Vec<_>>().join("\n");
    if remainder.is_empty() {
        format!("{package_line}\n\n{import}\n")
    } else {
        format!("{package_line}\n\n{import}\n{remainder}\n")
    }
}

fn native_plugin_sources(
    plugin: &nexa_ir::Plugin,
    relative_root: &str,
    extension: &str,
) -> Result<Vec<PathBuf>, String> {
    let plugin_root = Path::new(&plugin.idl_path)
        .parent()
        .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
    let source_root = plugin_root.join(relative_root);
    if !source_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files(&source_root, extension, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(
    directory: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, extension, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            files.push(path);
        }
    }
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

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn source_app_name(path: &Path) -> Option<String> {
    let source = fs::read_to_string(path).ok()?;
    nexa_syntax::parse_program(&source)
        .ok()?
        .app
        .map(|app| app.name)
}

fn project_cache_is_current(
    output: &Path,
    app_name: &str,
    target: ProjectTarget,
    cache_key: &str,
) -> bool {
    let Ok(manifest) = fs::read_to_string(output.join("nexa.project.json")) else {
        return false;
    };
    if !manifest.contains(&format!("\"cacheKey\": \"{cache_key}\"")) {
        return false;
    }
    if !output.join("README.md").is_file() {
        return false;
    }
    if !output.join("nexa.config.nx").is_file() {
        return false;
    }
    let needs_ios = matches!(target, ProjectTarget::Ios | ProjectTarget::All);
    let needs_android = matches!(target, ProjectTarget::Android | ProjectTarget::All);
    if needs_ios {
        for path in [
            output
                .join("ios")
                .join(app_name)
                .join("NexaGenerated.swift"),
            output
                .join("ios")
                .join(app_name)
                .join(format!("{app_name}App.swift")),
            output.join("ios").join(app_name).join("Info.plist"),
            output
                .join("ios")
                .join(format!("{app_name}.xcodeproj/project.pbxproj")),
        ] {
            if !path.is_file() {
                return false;
            }
        }
    }
    if needs_android {
        let package = package_name(app_name);
        let package_path = package.replace('.', "/");
        for path in [
            output
                .join("android/app/src/main/java")
                .join(&package_path)
                .join("NexaGenerated.kt"),
            output
                .join("android/app/src/main/java")
                .join(&package_path)
                .join("MainActivity.kt"),
            output.join("android/app/build.gradle.kts"),
            output.join("android/build.gradle.kts"),
            output.join("android/settings.gradle.kts"),
            output.join("android/gradle.properties"),
            output.join("android/app/proguard-rules.pro"),
            output.join("android/app/src/main/AndroidManifest.xml"),
        ] {
            if !path.is_file() {
                return false;
            }
        }
    }
    true
}

fn root_readme(app_name: &str, targets: &[&str]) -> String {
    let mut readme = format!(
        "# {app_name}\n\nGenerated by Nexa from a single `.nx` entry file. Edit the Nexa source and regenerate; generated native files are build outputs. Edit `nexa.config.nx` to customize compile-time permissions, iOS purpose messages, and declared plugin options.\n\n"
    );
    if targets.contains(&"ios") {
        readme.push_str(&format!("## iOS\n\n`xcodebuild -project ios/{app_name}.xcodeproj -scheme {app_name} -sdk iphonesimulator build`\n\nOpen `ios/{app_name}.xcodeproj` in Xcode to run on a device or simulator.\n\n"));
    }
    if targets.contains(&"android") {
        readme.push_str("## Android\n\n`gradle -p android :app:assembleDebug`\n\nThe generated Gradle project uses Jetpack Compose and Coil 3 with the platform network stack when needed. Release builds enable R8 shrinking, resource shrinking, and the optimized Android ruleset.\n\n");
    }
    readme.push_str("Requirements: Rust/Nexa for regeneration, Xcode 16+ for iOS, and Android SDK/Gradle for Android.\n");
    readme
}

fn ios_info_plist(app_name: &str, config: &ProjectConfig) -> String {
    let mut entries = String::new();
    for (permission, description) in config.permissions() {
        let keys: &[&str] = match *permission {
            nexa_ir::Permission::Camera => &["NSCameraUsageDescription"],
            nexa_ir::Permission::Microphone => &["NSMicrophoneUsageDescription"],
            nexa_ir::Permission::Photos => &["NSPhotoLibraryUsageDescription"],
            nexa_ir::Permission::Location => &["NSLocationWhenInUseUsageDescription"],
            // iOS notification authorization has no Info.plist usage-description key.
            nexa_ir::Permission::Notifications => &[],
            nexa_ir::Permission::Contacts => &["NSContactsUsageDescription"],
            nexa_ir::Permission::Calendar => &["NSCalendarsFullAccessUsageDescription"],
            nexa_ir::Permission::Bluetooth => &["NSBluetoothAlwaysUsageDescription"],
        };
        for key in keys {
            entries.push_str(&format!(
                "<key>{key}</key><string>{}</string>",
                xml_escape(description)
            ));
        }
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleDisplayName</key><string>{app_name}</string><key>CFBundleIdentifier</key><string>com.nexa.{}</string><key>CFBundleName</key><string>{app_name}</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>1.0</string><key>CFBundleVersion</key><string>1</string><key>LSRequiresIPhoneOS</key><true/>{entries}</dict></plist>\n",
        app_name.to_ascii_lowercase()
    )
}

fn ios_project_file(app_name: &str) -> String {
    let app_file = format!("{app_name}App.swift");
    format!(
        "// !$*UTF8*$!\n{{\n\tarchiveVersion = 1;\n\tclasses = {{}};\n\tobjectVersion = 77;\n\tobjects = {{\n\t\tAA0000000000000000000001 = {{ isa = PBXProject; buildConfigurationList = AA0000000000000000000002; compatibilityVersion = \"Xcode 16.0\"; mainGroup = AA0000000000000000000003; productRefGroup = AA0000000000000000000004; targets = ( AA0000000000000000000005 ); }};\n\t\tAA0000000000000000000003 = {{ isa = PBXGroup; children = ( AA0000000000000000000014, AA0000000000000000000004 ); sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000014 = {{ isa = PBXGroup; children = ( AA0000000000000000000010, AA0000000000000000000011, AA0000000000000000000012 ); path = {app_name}; sourceTree = \"<group>\"; }};
        AA0000000000000000000004 = {{ isa = PBXGroup; children = ( AA0000000000000000000013 ); name = Products; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000010 = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {app_file}; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000011 = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = NexaGenerated.swift; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000012 = {{ isa = PBXFileReference; lastKnownFileType = text.plist.xml; path = Info.plist; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000013 = {{ isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = {app_name}.app; sourceTree = BUILT_PRODUCTS_DIR; }};\n\t\tAA0000000000000000000020 = {{ isa = PBXBuildFile; fileRef = AA0000000000000000000010; }};\n\t\tAA0000000000000000000021 = {{ isa = PBXBuildFile; fileRef = AA0000000000000000000011; }};\n\t\tAA0000000000000000000005 = {{ isa = PBXNativeTarget; buildConfigurationList = AA0000000000000000000007; buildPhases = ( AA0000000000000000000008, AA0000000000000000000009, AA000000000000000000000A ); name = {app_name}; productName = {app_name}; productReference = AA0000000000000000000013; productType = \"com.apple.product-type.application\"; }};\n\t\tAA0000000000000000000008 = {{ isa = PBXSourcesBuildPhase; files = ( AA0000000000000000000020, AA0000000000000000000021 ); }};\n\t\tAA0000000000000000000009 = {{ isa = PBXFrameworksBuildPhase; files = (); }};\n\t\tAA000000000000000000000A = {{ isa = PBXResourcesBuildPhase; files = (); }};\n\t\tAA0000000000000000000002 = {{ isa = XCConfigurationList; buildConfigurations = ( AA0000000000000000000022 ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; }};\n\t\tAA0000000000000000000007 = {{ isa = XCConfigurationList; buildConfigurations = ( AA0000000000000000000023 ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; }};\n\t\tAA0000000000000000000022 = {{ isa = XCBuildConfiguration; buildSettings = {{ ALWAYS_SEARCH_USER_PATHS = NO; SWIFT_VERSION = 5.0; SWIFT_OPTIMIZATION_LEVEL = \"-O\"; SWIFT_COMPILATION_MODE = wholemodule; GCC_OPTIMIZATION_LEVEL = s; DEAD_CODE_STRIPPING = YES; IPHONEOS_DEPLOYMENT_TARGET = 16.0; }}; name = Release; }};\n\t\tAA0000000000000000000023 = {{ isa = XCBuildConfiguration; buildSettings = {{ ALWAYS_SEARCH_USER_PATHS = NO; PRODUCT_BUNDLE_IDENTIFIER = com.nexa.{}; PRODUCT_NAME = {app_name}; INFOPLIST_FILE = {app_name}/Info.plist; SUPPORTED_PLATFORMS = \"iphoneos iphonesimulator\"; SWIFT_VERSION = 5.0; SWIFT_OPTIMIZATION_LEVEL = \"-O\"; SWIFT_COMPILATION_MODE = wholemodule; GCC_OPTIMIZATION_LEVEL = s; DEAD_CODE_STRIPPING = YES; IPHONEOS_DEPLOYMENT_TARGET = 16.0; TARGETED_DEVICE_FAMILY = \"1,2\"; }}; name = Release; }};\n\t}};\n\trootObject = AA0000000000000000000001;\n}}\n",
        app_name.to_ascii_lowercase()
    )
}

fn ios_scheme(app_name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Scheme LastUpgradeVersion="2700" version="1.7">
   <BuildAction parallelizeBuildables="YES" buildImplicitDependencies="YES">
      <BuildActionEntries>
         <BuildActionEntry buildForTesting="YES" buildForRunning="YES" buildForProfiling="YES" buildForArchiving="YES" buildForAnalyzing="YES">
            <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="AA0000000000000000000005" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
         </BuildActionEntry>
      </BuildActionEntries>
   </BuildAction>
   <TestAction buildConfiguration="Release" shouldUseLaunchSchemeArgsEnv="YES"/>
   <LaunchAction buildConfiguration="Release" useCustomWorkingDirectory="NO" ignoresPersistentStateOnLaunch="NO" debugDocumentVersioning="YES" debugServiceExtension="internal" allowLocationSimulation="YES">
      <BuildableProductRunnable runnableDebuggingMode="0">
         <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="AA0000000000000000000005" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
      </BuildableProductRunnable>
   </LaunchAction>
   <ProfileAction buildConfiguration="Release" shouldUseLaunchSchemeArgsEnv="YES" savedToolIdentifier="" useCustomWorkingDirectory="NO" debugDocumentVersioning="YES">
      <BuildableProductRunnable runnableDebuggingMode="0">
         <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="AA0000000000000000000005" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
      </BuildableProductRunnable>
   </ProfileAction>
   <AnalyzeAction buildConfiguration="Release"/>
   <ArchiveAction buildConfiguration="Release" revealArchiveInOrganizer="YES"/>
</Scheme>
"#
    )
}

fn android_settings(app_name: &str) -> String {
    format!(
        "pluginManagement {{ repositories {{ google(); mavenCentral(); gradlePluginPortal() }} }}\ndependencyResolutionManagement {{ repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories {{ google(); mavenCentral() }} }}\nrootProject.name = \"{app_name}\"\ninclude(\":app\")\n"
    )
}
fn android_root_gradle() -> String {
    "plugins {\n    id(\"com.android.application\") version \"9.2.1\" apply false\n    id(\"org.jetbrains.kotlin.plugin.compose\") version \"2.4.10\" apply false\n}\n".to_owned()
}
fn android_properties() -> String {
    "android.useAndroidX=true\nkotlin.code.style=official\norg.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n".to_owned()
}
fn android_manifest(app_name: &str, package: &str, remote: bool, config: &ProjectConfig) -> String {
    let mut declared = String::new();
    if remote {
        declared.push_str("    <uses-permission android:name=\"android.permission.INTERNET\" />\n");
    }
    for (permission, _) in config.permissions() {
        let names: &[&str] = match *permission {
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

fn android_app_gradle(
    package: &str,
    uses_network: bool,
    uses_remote_image: bool,
    uses_coroutines: bool,
    navigation: bool,
    uses_compose_graphics: bool,
    uses_lifecycle_events: bool,
) -> String {
    let mut dependencies = String::from(
        "    implementation(platform(\"androidx.compose:compose-bom:2026.09.00\"))\n    implementation(\"androidx.activity:activity-compose:1.13.0\")\n    implementation(\"androidx.compose.ui:ui\")\n    implementation(\"androidx.compose.material3:material3\")\n",
    );
    if uses_compose_graphics {
        dependencies.push_str("    implementation(\"androidx.compose.ui:ui-graphics\")\n");
    }
    if navigation {
        dependencies
            .push_str("    implementation(\"androidx.navigation:navigation-compose:2.8.5\")\n");
    }
    if uses_lifecycle_events {
        dependencies.push_str(
            "    implementation(\"androidx.lifecycle:lifecycle-runtime-compose:2.11.0\")\n",
        );
    }
    if uses_remote_image {
        dependencies.push_str(
            "    implementation(\"io.coil-kt.coil3:coil-compose:3.6.3\")\n    implementation(\"io.coil-kt.coil3:coil-network-core:3.6.3\")\n",
        );
    }
    if uses_coroutines {
        dependencies.push_str(
            "    implementation(\"org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0\")\n",
        );
    }
    if uses_network {
        dependencies.push_str(
            "    implementation(\"com.google.android.gms:play-services-cronet:18.0.1\")\n",
        );
    }
    format!(
        "plugins {{\n    id(\"com.android.application\")\n    id(\"org.jetbrains.kotlin.plugin.compose\")\n}}\n\nandroid {{\n    namespace = \"{package}\"\n    compileSdk = 37\n    defaultConfig {{ applicationId = \"{package}\"; minSdk = 26; targetSdk = 37; versionCode = 1; versionName = \"1.0\" }}\n    buildFeatures {{ compose = true }}\n    compileOptions {{ sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }}\n    buildTypes {{\n        release {{\n            isMinifyEnabled = true\n            isShrinkResources = true\n            proguardFiles(\n                getDefaultProguardFile(\"proguard-android-optimize.txt\"),\n                \"proguard-rules.pro\"\n            )\n        }}\n    }}\n}}\n\nkotlin {{\n    compilerOptions {{\n        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)\n    }}\n}}\n\ndependencies {{\n{dependencies}}}\n"
    )
}

fn android_proguard_rules() -> &'static str {
    "# Nexa generated bindings use direct calls and do not require broad keep rules.\n"
}
