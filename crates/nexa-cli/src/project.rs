//! Native project bundle generation.
//!
//! This module deliberately owns only project scaffolding. The compiler and
//! backends continue to own parsing, semantic analysis, and native source
//! generation. Command orchestration stays here while deterministic templates
//! and optional plugin emission live in child modules, so host projects can
//! evolve without adding a runtime or coupling build files to the IR.

use std::{
    fs,
    path::{Path, PathBuf},
};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_targets};
use nexa_ir::Module;

use crate::{cache, config, config::ProjectConfig};

mod plugins;
mod templates;

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
        "{{\n  \"format\": 1,\n  \"entry\": \"{}\",\n  \"name\": \"{}\",\n  \"targets\": [{}],\n  \"sourceManifest\": \"nexa.sources.json\",\n  \"cacheKey\": \"{}\"\n}}\n",
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
        &output.join("nexa.sources.json"),
        &source_manifest(&output, &app_name, &generated_targets)?,
    )?;
    write_if_changed(
        &output.join("README.md"),
        &templates::root_readme(&app_name, &generated_targets),
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

fn source_manifest(root: &Path, app_name: &str, targets: &[&str]) -> Result<String, String> {
    let mut units = Vec::new();
    for target in targets {
        let directory = if *target == "ios" {
            root.join("ios").join(app_name)
        } else {
            root.join("android").join("app").join("src").join("main")
        };
        let mut files = Vec::new();
        collect_source_units(&directory, root, &mut files)?;
        files.sort();
        for file in files {
            units.push(format!(
                "    {{ \"target\": \"{}\", \"path\": \"{}\" }}",
                target,
                json_escape(&file)
            ));
        }
    }
    Ok(format!(
        "{{\n  \"format\": 1,\n  \"generatedBy\": \"nexa\",\n  \"units\": [\n{}\n  ]\n}}\n",
        units.join(",\n")
    ))
}

fn collect_source_units(
    directory: &Path,
    root: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_source_units(&path, root, files)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| matches!(extension, "swift" | "kt" | "xml" | "plist" | "json"))
            || path
                .components()
                .any(|component| component.as_os_str() == "Assets.xcassets")
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

/// Splits backend output at generator-owned unit markers while preserving one
/// shared import/package header in every native source file. Top-level private
/// declarations become module-internal so a component can call a generated
/// helper from another unit without a runtime indirection; member visibility
/// remains unchanged.
fn split_generated_units(source: &str, extension: &str) -> Vec<(String, String)> {
    let mut header = Vec::new();
    let mut units: Vec<(String, Vec<String>)> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("// nexa-unit:") {
            units.push((name.to_owned(), Vec::new()));
        } else if let Some((_, lines)) = units.last_mut() {
            lines.push(line.to_owned());
        } else {
            header.push(line.to_owned());
        }
    }
    if units.is_empty() {
        return vec![(
            format!("NexaGenerated.{extension}"),
            ensure_internal_top_level(source),
        )];
    }
    let header = header.join("\n");
    let mut result = Vec::new();
    for (name, lines) in units {
        if lines.iter().all(|line| line.trim().is_empty()) {
            continue;
        }
        let mut contents = String::new();
        if !header.trim().is_empty() {
            contents.push_str(&header);
            contents.push_str("\n\n");
        }
        contents.push_str(&ensure_internal_top_level(&lines.join("\n")));
        contents.push('\n');
        let file_name = if name == "app" {
            format!("NexaGenerated.{extension}")
        } else {
            format!("NexaGenerated_{}.{}", name.replace('-', "_"), extension)
        };
        result.push((file_name, contents));
    }
    if let Some(app_index) = result
        .iter()
        .position(|(file_name, _)| file_name == &format!("NexaGenerated.{extension}"))
    {
        let app = result.remove(app_index);
        result.insert(0, app);
    }
    result
}

fn ensure_internal_top_level(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            if line.starts_with("private ") {
                line.replacen("private ", "", 1)
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_stale_generated_units(
    directory: &Path,
    current: &[String],
    extension: &str,
) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with("NexaGenerated_")
            && path.extension().and_then(|value| value.to_str()) == Some(extension)
            && !current.iter().any(|value| value == name)
        {
            fs::remove_file(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
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
    let source_units = split_generated_units(&source, "swift");
    let has_assets = plugins::copy_plugin_assets(root, app_name, module)?;
    let plugin_sources = plugins::copy_ios_plugin_sources(root, app_name, module)?;
    let generated_names = source_units
        .iter()
        .map(|(name, contents)| {
            write_if_changed(&directory.join(name), contents)?;
            Ok(name.clone())
        })
        .collect::<Result<Vec<_>, String>>()?;
    remove_stale_generated_units(&directory, &generated_names, "swift")?;
    write_if_changed(
        &directory.join(format!("{app_name}App.swift")),
        &format!(
            "import SwiftUI\n\n@main\nstruct {app_name}App: App {{\n    var body: some Scene {{\n        WindowGroup {{\n            {screen}()\n        }}\n    }}\n}}\n"
        ),
    )?;
    write_if_changed(
        &directory.join("Info.plist"),
        &templates::ios_info_plist(app_name, config),
    )?;
    write_if_changed(
        &root
            .join("ios")
            .join(format!("{app_name}.xcodeproj/project.pbxproj")),
        &templates::ios_project_file(app_name, has_assets, &generated_names, &plugin_sources),
    )?;
    write_if_changed(
        &root.join("ios").join(format!(
            "{app_name}.xcodeproj/xcshareddata/xcschemes/{app_name}.xcscheme"
        )),
        &templates::ios_scheme(app_name),
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
    let (generated, project_features) = KotlinBackend.generate_with_project_features(module);
    plugins::copy_android_plugin_sources(root, module, &package, config)?;
    plugins::copy_plugin_assets(root, app_name, module)?;
    let screen = nexa_codegen::names::screen_name(app_name);
    let cronet_import = if project_features.uses_network {
        "import com.google.android.gms.net.CronetProviderInstaller\n"
    } else {
        ""
    };
    let permission_callback = "";
    let content_setup = if project_features.uses_network {
        format!(
            "        CronetProviderInstaller.installProvider(this).addOnCompleteListener {{ result ->\n            if (result.isSuccessful) {{\n                setContent {{ MaterialTheme {{ {screen}() }} }}\n            }} else {{\n                setContent {{ MaterialTheme {{ androidx.compose.material3.Text(\"Network provider unavailable\") }} }}\n            }}\n        }}\n"
        )
    } else {
        format!("        setContent {{ MaterialTheme {{ {screen}() }} }}\n")
    };
    let generated_source = format!(
        "package {package}\n\n{generated}{}",
        plugins::render_kotlin_plugin_config(module, config)
    );
    let generated_units = split_generated_units(&generated_source, "kt");
    let generated_names = generated_units
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    for (name, contents) in generated_units {
        write_if_changed(&source_dir.join(name), &contents)?;
    }
    remove_stale_generated_units(&source_dir, &generated_names, "kt")?;
    write_if_changed(
        &source_dir.join("MainActivity.kt"),
        &format!(
            "package {package}\n\nimport android.os.Bundle\nimport androidx.activity.ComponentActivity\nimport androidx.activity.compose.setContent\nimport androidx.compose.material3.MaterialTheme\n{cronet_import}\nclass MainActivity : ComponentActivity() {{\n    override fun onCreate(savedInstanceState: Bundle?) {{\n        super.onCreate(savedInstanceState)\n{content_setup}    }}\n{permission_callback}}}\n"
        ),
    )?;
    write_if_changed(
        &root.join("android/app/src/main/AndroidManifest.xml"),
        &templates::android_manifest(app_name, &package, project_features.uses_network, config),
    )?;
    write_if_changed(
        &root.join("android/settings.gradle.kts"),
        &templates::android_settings(app_name),
    )?;
    write_if_changed(
        &root.join("android/build.gradle.kts"),
        &templates::android_root_gradle(),
    )?;
    write_if_changed(
        &root.join("android/gradle.properties"),
        &templates::android_properties(),
    )?;
    write_if_changed(
        &root.join("android/app/build.gradle.kts"),
        &templates::android_app_gradle(&package, project_features),
    )?;
    write_if_changed(
        &root.join("android/app/proguard-rules.pro"),
        templates::android_proguard_rules(),
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
    let plugin_config = plugins::render_swift_plugin_config(module, config);
    if !plugin_config.is_empty() {
        declarations.push(plugin_config);
    }
    imports.sort();
    imports.dedup();
    let mut source = imports.join("\n");
    source.push_str("\n\n");
    source.push_str(&declarations.join("\n"));
    source.push('\n');
    Ok(source)
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
    if !output.join("nexa.sources.json").is_file() {
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
