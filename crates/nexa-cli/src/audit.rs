//! Static generated-output audit with optional native release measurements.
//!
//! The default command stays fast and reports only generated-source facts.
//! `--release-sizes` opts into isolated Release builds and measures the actual
//! APK or iOS app bundle when the relevant toolchain is installed.

use std::{collections::BTreeSet, fs, path::PathBuf};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_targets_and_plugin_roots};
use nexa_ir::{Module, capabilities::Capabilities};

mod native;

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut target = "all";
    let mut release_sizes = false;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--target" | "-t" => {
                cursor += 1;
                target = args
                    .get(cursor)
                    .ok_or("`--target` requires `ios`, `android`, or `all`")?;
                if !matches!(target, "ios" | "swift" | "android" | "kotlin" | "all") {
                    return Err(format!(
                        "unknown target `{target}`; expected `ios`, `android`, or `all`"
                    ));
                }
            }
            "--out" | "--output" | "-o" => {
                cursor += 1;
                output = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a JSON path")?,
                ));
            }
            "--release-sizes" => release_sizes = true,
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if input.is_none() => input = Some(PathBuf::from(value)),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }
    let input = input.ok_or(
        "usage: nexa audit <source.nx> [--target <ios|android|all>] [--release-sizes] [--out <path>]",
    )?;
    let targets = match target {
        "ios" | "swift" => vec![("ios", Target::Swift)],
        "android" | "kotlin" => vec![("android", Target::Kotlin)],
        _ => vec![("ios", Target::Swift), ("android", Target::Kotlin)],
    };

    let project_root = input
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let config_path = project_root.join("nexa.config.nx");
    let dependencies = crate::config::load_plugin_dependencies(&config_path)?;
    let resolved = crate::dependencies::resolve(project_root, &dependencies)?;
    let compile_targets = targets
        .iter()
        .map(|(_, target)| *target)
        .collect::<Vec<_>>();
    let compilations = compile_file_with_warnings_for_targets_and_plugin_roots(
        &input,
        &compile_targets,
        &resolved.plugin_roots,
    )
    .map_err(|error| error.to_string())?;

    let mut report = String::from("{\n  \"format\": 2,\n  \"tool\": \"nexa audit\",\n");
    report.push_str(&format!(
        "  \"entry\": \"{}\",\n  \"targets\": [\n",
        json_escape(&input.display().to_string())
    ));
    let mut all_warnings = Vec::new();
    for (index, ((name, _), compilation)) in targets.iter().zip(compilations).enumerate() {
        all_warnings.extend(compilation.warnings.clone());
        let native_plugins = crate::project::plugin_package::packages_for_module(
            &compilation.plugins,
            &compilation.module,
        );
        let module = compilation.module;
        let capabilities = nexa_ir::capabilities::analyze(&module);
        let (source, dependencies) = generated_source_and_dependencies(name, &module);
        if index != 0 {
            report.push_str(",\n");
        }
        report.push_str(&target_report(
            name,
            &module,
            capabilities,
            source.len(),
            &dependencies,
            &native_plugins,
        ));
    }
    report.push_str("\n  ],\n  \"warnings\": [");
    let warnings = all_warnings
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    for (index, warning) in warnings.iter().enumerate() {
        if index != 0 {
            report.push_str(", ");
        }
        report.push_str(&format!("\"{}\"", json_escape(&warning.to_string())));
    }
    report.push_str("],\n  \"releaseMeasurements\": ");
    if release_sizes {
        report.push_str(&native::measure_release_sizes(
            &input,
            matches!(target, "ios" | "swift" | "all"),
            matches!(target, "android" | "kotlin" | "all"),
        ));
    } else {
        report.push_str("null");
    }
    report.push_str("\n}\n");

    if let Some(path) = output {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(&path, &report).map_err(|error| format!("{}: {error}", path.display()))?;
        println!("audited {}", path.display());
    } else {
        print!("{report}");
    }
    Ok(())
}

pub(super) fn print_help() {
    println!(
        "Usage: nexa audit <source.nx> [--target <ios|android|all>] [--release-sizes] [--out <path>]"
    );
    println!("Reports generated native capabilities and optional release package sizes.");
}

fn generated_source_and_dependencies(target: &str, module: &Module) -> (String, Vec<String>) {
    if target == "ios" {
        let source = SwiftBackend.generate(module);
        let mut dependencies = source
            .lines()
            .filter_map(|line| line.strip_prefix("import "))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        dependencies.sort();
        dependencies.dedup();
        (source, dependencies)
    } else {
        let (source, features) = KotlinBackend.generate_with_project_features(module);
        let mut dependencies = vec![
            "androidx.activity:activity-compose".to_owned(),
            "androidx.compose.ui:ui".to_owned(),
            "androidx.compose.material3:material3".to_owned(),
        ];
        if features.uses_network {
            dependencies.push("com.google.android.gms:play-services-cronet".to_owned());
        }
        if features.uses_remote_image {
            dependencies.push("io.coil-kt.coil3:coil-compose".to_owned());
            dependencies.push("io.coil-kt.coil3:coil-network-core".to_owned());
        }
        if features.uses_coroutines {
            dependencies.push("org.jetbrains.kotlinx:kotlinx-coroutines-android".to_owned());
        }
        if features.uses_navigation {
            dependencies.push("androidx.navigation:navigation-compose".to_owned());
        }
        if features.uses_compose_animation {
            dependencies.push("androidx.compose.animation:animation".to_owned());
        }
        if features.uses_lifecycle_events {
            dependencies.push("androidx.lifecycle:lifecycle-runtime-compose".to_owned());
        }
        if features.uses_compose_graphics {
            dependencies.push("androidx.compose.ui:ui-graphics".to_owned());
        }
        dependencies.sort();
        (source, dependencies)
    }
}

fn target_report(
    target: &str,
    module: &Module,
    capabilities: Capabilities,
    generated_bytes: usize,
    dependencies: &[String],
    native_plugins: &[crate::project::plugin_package::PluginPackage],
) -> String {
    let mut output = format!(
        "    {{\n      \"target\": \"{target}\",\n      \"app\": \"{}\",\n      \"generatedSourceBytes\": {generated_bytes},\n      \"capabilities\": {{\n        \"remoteImage\": {},\n        \"network\": {},\n        \"path\": {},\n        \"file\": {},\n        \"fileAsync\": {},\n        \"storage\": {},\n        \"secureStorage\": {}\n      }},\n      \"dependencies\": [",
        json_escape(&module.app_name),
        capabilities.uses_remote_image,
        capabilities.uses_network_api,
        capabilities.uses_path_api,
        capabilities.uses_file_api,
        capabilities.uses_file_async,
        capabilities.uses_storage_api,
        capabilities.uses_secure_storage_api,
    );
    for (index, dependency) in dependencies.iter().enumerate() {
        if index != 0 {
            output.push_str(", ");
        }
        output.push_str(&format!("\"{}\"", json_escape(dependency)));
    }
    output.push_str("],\n      \"nativePlugins\": ");
    output.push_str(&native_plugins_report(target, native_plugins));
    output.push_str("\n    }");
    output
}

fn native_plugins_report(
    target: &str,
    plugins: &[crate::project::plugin_package::PluginPackage],
) -> String {
    let mut output = String::from("[");
    for (index, plugin) in plugins.iter().enumerate() {
        if index != 0 {
            output.push_str(", ");
        }
        output.push_str(&format!(
            "{{\"namespace\": \"{}\", \"dependencies\": {{",
            json_escape(&plugin.namespace)
        ));
        if target == "ios" {
            output.push_str(&format!(
                "\"frameworks\": {}, \"xcframeworks\": {}, \"swiftPackages\": [",
                string_array_json(&plugin.artifacts.ios_frameworks),
                path_name_array_json(&plugin.artifacts.ios_xcframeworks),
            ));
            for (package_index, package) in plugin.artifacts.swift_packages.iter().enumerate() {
                if package_index != 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!(
                    "{{\"url\": \"{}\", \"requirement\": {}, \"products\": {}}}",
                    json_escape(&package.url),
                    swift_package_requirement_json(&package.requirement),
                    string_array_json(&package.products)
                ));
            }
            output.push(']');
        } else {
            output.push_str(&format!(
                "\"mavenDependencies\": {}, \"aars\": {}",
                string_array_json(&plugin.artifacts.maven_dependencies),
                path_name_array_json(&plugin.artifacts.android_aars),
            ));
        }
        output.push_str("}}");
    }
    output.push(']');
    output
}

fn string_array_json(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
}

fn path_name_array_json(paths: &[String]) -> String {
    let names = paths
        .iter()
        .map(|path| {
            PathBuf::from(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
                .to_owned()
        })
        .collect::<Vec<_>>();
    string_array_json(&names)
}

fn swift_package_requirement_json(
    requirement: &nexa_plugin_idl::manifest::SwiftPackageRequirement,
) -> String {
    use nexa_plugin_idl::manifest::SwiftPackageRequirement;

    let (kind, value) = match requirement {
        SwiftPackageRequirement::From(value) => ("from", value),
        SwiftPackageRequirement::Branch(value) => ("branch", value),
        SwiftPackageRequirement::Revision(value) => ("revision", value),
    };
    format!(
        "{{\"kind\": \"{kind}\", \"value\": \"{}\"}}",
        json_escape(value)
    )
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::native_plugins_report;
    use crate::project::plugin_package::{PluginArtifacts, PluginPackage};
    use nexa_plugin_idl::manifest::{SwiftPackage, SwiftPackageRequirement};

    #[test]
    fn native_plugin_report_shows_platform_dependencies_without_machine_paths() {
        let plugins = [PluginPackage {
            namespace: "VideoPlayer".to_owned(),
            idl_path: "/tmp/video-player/native.nxid".to_owned(),
            artifacts: PluginArtifacts {
                ios_frameworks: vec!["AVFoundation".to_owned()],
                ios_xcframeworks: vec!["/tmp/video-player/VideoCore.xcframework".to_owned()],
                swift_packages: vec![SwiftPackage {
                    url: "https://example.com/video-core.git".to_owned(),
                    requirement: SwiftPackageRequirement::From("1.2.0".to_owned()),
                    products: vec!["VideoCore".to_owned()],
                    extension_products: Vec::new(),
                }],
                maven_dependencies: vec!["androidx.media3:media3-exoplayer:1.11.1".to_owned()],
                android_aars: vec!["/tmp/video-player/media-decoder.aar".to_owned()],
                ..PluginArtifacts::default()
            },
        }];

        let ios: serde_json::Value = serde_json::from_str(&native_plugins_report("ios", &plugins))
            .expect("iOS plugin dependency report should be valid JSON");
        assert_eq!(ios[0]["namespace"], "VideoPlayer");
        assert_eq!(ios[0]["dependencies"]["frameworks"][0], "AVFoundation");
        assert_eq!(
            ios[0]["dependencies"]["xcframeworks"][0],
            "VideoCore.xcframework"
        );
        assert_eq!(
            ios[0]["dependencies"]["swiftPackages"][0]["products"][0],
            "VideoCore"
        );
        assert_eq!(
            ios[0]["dependencies"]["swiftPackages"][0]["requirement"]["kind"],
            "from"
        );
        assert_eq!(
            ios[0]["dependencies"]["swiftPackages"][0]["requirement"]["value"],
            "1.2.0"
        );
        assert!(!ios.to_string().contains("/tmp/video-player"));

        let android: serde_json::Value =
            serde_json::from_str(&native_plugins_report("android", &plugins))
                .expect("Android plugin dependency report should be valid JSON");
        assert_eq!(
            android[0]["dependencies"]["mavenDependencies"][0],
            "androidx.media3:media3-exoplayer:1.11.1"
        );
        assert_eq!(android[0]["dependencies"]["aars"][0], "media-decoder.aar");
        assert!(!android.to_string().contains("/tmp/video-player"));
    }
}
