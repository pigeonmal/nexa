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
        &templates::ios_info_plist(app_name, config),
    )?;
    write_if_changed(
        &root
            .join("ios")
            .join(format!("{app_name}.xcodeproj/project.pbxproj")),
        &templates::ios_project_file(app_name),
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
    let screen = nexa_codegen::names::screen_name(app_name);
    let cronet_import = if project_features.uses_network {
        "import com.google.android.gms.net.CronetProviderInstaller\n"
    } else {
        ""
    };
    let permission_callback = if project_features.uses_permission_request {
        "    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<String>, grantResults: IntArray) {\n        super.onRequestPermissionsResult(requestCode, permissions, grantResults)\n        NexaRuntime.dispatchPermissionResult(requestCode, grantResults)\n    }\n"
    } else {
        ""
    };
    let content_setup = if project_features.uses_network {
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
            plugins::render_kotlin_plugin_config(module, config)
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
    for plugin in &module.plugins {
        for path in plugins::native_plugin_sources(plugin, "ios/Sources", "swift")? {
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
