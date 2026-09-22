//! Static generated-output audit.
//!
//! `nexa audit` deliberately reports facts available before native toolchains
//! run. It never invents APK or app-binary sizes; those fields are only
//! populated by a future native release integration.

use std::{fs, path::PathBuf};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::{Module, capabilities::Capabilities};

use crate::deduplicate_warnings;

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut target = "all";
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
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
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if input.is_none() => input = Some(PathBuf::from(value)),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }
    let input =
        input.ok_or("usage: nexa audit <source.nx> [--target <ios|android|all>] [--out <path>]")?;
    let targets = match target {
        "ios" | "swift" => vec![("ios", Target::Swift)],
        "android" | "kotlin" => vec![("android", Target::Kotlin)],
        _ => vec![("ios", Target::Swift), ("android", Target::Kotlin)],
    };

    let mut report = String::from("{\n  \"format\": 1,\n  \"tool\": \"nexa audit\",\n");
    report.push_str(&format!(
        "  \"entry\": \"{}\",\n  \"targets\": [\n",
        json_escape(&input.display().to_string())
    ));
    let mut all_warnings = Vec::new();
    for (index, (name, compile_target)) in targets.iter().enumerate() {
        let compilation = compile_file_with_warnings_for_target(&input, *compile_target)
            .map_err(|error| error.to_string())?;
        all_warnings.extend(compilation.warnings.clone());
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
        ));
    }
    report.push_str("\n  ],\n  \"warnings\": [");
    let warnings = deduplicate_warnings(all_warnings);
    for (index, warning) in warnings.iter().enumerate() {
        if index != 0 {
            report.push_str(", ");
        }
        report.push_str(&format!("\"{}\"", json_escape(&warning.to_string())));
    }
    report.push_str("],\n  \"nativeBinarySize\": null\n}\n");

    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(&path, &report).map_err(|error| format!("{}: {error}", path.display()))?;
        println!("audited {}", path.display());
    } else {
        print!("{report}");
    }
    Ok(())
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
) -> String {
    let mut output = format!(
        "    {{\n      \"target\": \"{target}\",\n      \"app\": \"{}\",\n      \"generatedSourceBytes\": {generated_bytes},\n      \"capabilities\": {{\n        \"remoteImage\": {},\n        \"network\": {},\n        \"path\": {},\n        \"file\": {},\n        \"fileAsync\": {}\n      }},\n      \"dependencies\": [",
        json_escape(&module.app_name),
        capabilities.uses_remote_image,
        capabilities.uses_network_api,
        capabilities.uses_path_api,
        capabilities.uses_file_api,
        capabilities.uses_file_async,
    );
    for (index, dependency) in dependencies.iter().enumerate() {
        if index != 0 {
            output.push_str(", ");
        }
        output.push_str(&format!("\"{}\"", json_escape(dependency)));
    }
    output.push_str("],\n      \"nativeBinaryBytes\": null\n    }");
    output
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
