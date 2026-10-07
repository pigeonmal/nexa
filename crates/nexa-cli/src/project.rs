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
use nexa_compiler::{CompileWarning, Target};
use nexa_ir::{Module, system_icons::SHARED_ICONS};

use crate::{cache, config, config::ProjectConfig};

mod assets;
pub mod plan;
use self::plan::ProjectPlan;
mod localization;
mod pbxproj;
pub(crate) mod plugin_package;
mod plugins;
mod templates;
pub mod writers;

fn report_warnings(warnings: &[CompileWarning], deny_warnings: bool) -> Result<(), String> {
    for warning in warnings {
        eprintln!("{warning}");
    }
    if deny_warnings && !warnings.is_empty() {
        return Err(format!("{} warning(s) treated as errors", warnings.len()));
    }
    Ok(())
}

fn deduplicate_warnings(warnings: Vec<CompileWarning>) -> Vec<CompileWarning> {
    let mut seen = std::collections::HashSet::new();
    warnings
        .into_iter()
        .filter(|warning| seen.insert(warning.to_string()))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProjectTarget {
    Ios,
    Android,
    All,
}

#[derive(Clone, Debug)]
pub(crate) struct DevSessionConfig {
    pub(crate) server_url: String,
    pub(crate) session_token: String,
}

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    run_with_summary(args, true)
}

pub(crate) fn run_quiet(args: &[String]) -> Result<(), String> {
    run_with_summary(args, false)
}

fn run_with_summary(args: &[String], print_summary: bool) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut name = None;
    let mut target = ProjectTarget::All;
    let mut deny_warnings = false;
    let mut locked = false;
    let mut execute_tests = false;
    let mut flavor = None;
    let mut arch = None;
    let mut dev_server_url = None;
    let mut dev_session_token = None;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--deny-warnings" => deny_warnings = true,
            "--locked" => locked = true,
            "--execute-tests" => execute_tests = true,
            "--dev-server-url" => {
                cursor += 1;
                dev_server_url = Some(
                    args.get(cursor)
                        .ok_or("`--dev-server-url` requires a URL")?
                        .clone(),
                );
            }
            "--dev-session-token" => {
                cursor += 1;
                dev_session_token = Some(
                    args.get(cursor)
                        .ok_or("`--dev-session-token` requires a token")?
                        .clone(),
                );
            }
            "--staging" => flavor = Some("staging".to_owned()),
            "--flavor" => {
                cursor += 1;
                flavor = Some(
                    args.get(cursor)
                        .ok_or("`--flavor` requires a name")?
                        .clone(),
                );
            }
            "--arch" => {
                cursor += 1;
                arch = Some(
                    args.get(cursor)
                        .ok_or("`--arch` requires an architecture")?
                        .clone(),
                );
            }
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
    let dev_session = match (dev_server_url, dev_session_token) {
        (Some(server_url), Some(session_token))
            if server_url.starts_with("ws://127.0.0.1:") && !session_token.is_empty() =>
        {
            Some(DevSessionConfig {
                server_url,
                session_token,
            })
        }
        (None, None) => None,
        (Some(_), Some(_)) => {
            return Err("dev runtime requires a loopback `ws://127.0.0.1:<port>` URL and a non-empty session token".to_owned());
        }
        _ => {
            return Err(
                "`--dev-server-url` and `--dev-session-token` must be supplied together".to_owned(),
            );
        }
    };
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
    let project_target = if let Some(flavor) = &flavor {
        format!("{project_target}-flavor-{flavor}")
    } else {
        project_target.to_owned()
    };
    let project_target = if let Some(arch) = &arch {
        format!("{project_target}-arch-{arch}")
    } else {
        project_target
    };
    let project_target = if dev_session.is_some() {
        format!("{project_target}-dev-runtime")
    } else {
        project_target
    };
    let project_root = input.parent().unwrap_or_else(|| Path::new("."));
    let config_path = input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("nexa.config.nx");
    let app_images_path = project_root.join("assets/images");
    let translations_path = project_root.join("locales/translations.json");
    let existing_config = config_path.is_file();
    let dependencies = config::load_plugin_dependencies(&config_path)?;
    let resolved_dependencies = crate::dependencies::resolve(project_root, &dependencies)?;
    crate::dependencies::sync_lock(
        project_root,
        !dependencies.is_empty(),
        &resolved_dependencies.lock_file,
        locked,
    )?;
    let plugin_roots = &resolved_dependencies.plugin_roots;
    let plugin_definitions = config::load_plugin_definitions(&input, plugin_roots)?;
    let project_config = if existing_config {
        Some(ProjectConfig::parse_file(
            &config_path,
            &plugin_definitions,
            &app_name,
        )?)
    } else {
        None
    };
    let sorted_plugin_roots = plugin_roots
        .iter()
        .map(|(package_id, path)| (package_id.clone(), path.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut cache_key = cache::key_with_extra_and_roots(
        &input,
        &project_target,
        &[
            config_path.as_path(),
            app_images_path.as_path(),
            translations_path.as_path(),
        ],
        &sorted_plugin_roots,
    )
    .map_err(|error| format!("project cache: {error}"))?;
    if !execute_tests
        && dev_session.is_none()
        && project_config.as_ref().is_some_and(|config| {
            let config = match &flavor {
                Some(flavor) => match config.with_flavor(flavor) {
                    Ok(config) => config,
                    Err(_) => return false,
                },
                None => config.clone(),
            };
            project_cache_is_current(&output, &app_name, target, &cache_key, &config)
        })
    {
        let cached_warnings = cache::restore_warnings(&input, &cache_key)
            .map_err(|error| format!("project cache: {error}"))?;
        if let Some(warnings) = cached_warnings {
            for warning in &warnings {
                eprintln!("{warning}");
            }
            if deny_warnings && !warnings.is_empty() {
                return Err(format!("{} warning(s) treated as errors", warnings.len()));
            }
            if print_summary {
                println!("generated {} (cache hit)", output.display());
            }
            return Ok(());
        }
    }

    let targets: &[Target] = match target {
        ProjectTarget::Ios => &[Target::Swift],
        ProjectTarget::Android => &[Target::Kotlin],
        ProjectTarget::All => &[Target::Swift, Target::Kotlin],
    };
    let compilations = if dev_session.is_some() {
        nexa_compiler::compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
            &input,
            targets,
            plugin_roots,
        )
    } else {
        nexa_compiler::compile_file_with_warnings_for_targets_and_plugin_roots(
            &input,
            targets,
            plugin_roots,
        )
    }
    .map_err(|error| error.to_string())?;
    if execute_tests {
        let Some(compilation) = compilations.first() else {
            return Err("no compilation target was selected for in-language tests".to_owned());
        };
        crate::commands::report_in_language_tests(&compilation.tests, &input)?;
    }
    let mut warnings = Vec::new();
    let compiled = targets
        .iter()
        .copied()
        .zip(compilations)
        .map(|(target, compilation)| -> Result<_, String> {
            warnings.extend(compilation.warnings);
            let packages = if dev_session.is_some() {
                plugin_package::packages_for_dev_with_dependencies(
                    &compilation.plugins,
                    &dependencies,
                    plugin_roots,
                )?
            } else {
                plugin_package::packages_for_module(&compilation.plugins, &compilation.module)
            };
            Ok((target, compilation.module, packages))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let warnings = deduplicate_warnings(warnings);
    report_warnings(&warnings, deny_warnings)?;

    let mut project_config = match project_config {
        Some(config) => config,
        None => match ProjectConfig::from_defaults(&plugin_definitions, &app_name) {
            Ok(config) => config,
            Err(error) => {
                write_if_changed(&config_path, &config::render_template(&plugin_definitions))?;
                return Err(format!(
                    "{error}; edit {} and run `nexa dev` again",
                    config_path.display()
                ));
            }
        },
    };
    if !existing_config {
        write_if_changed(&config_path, &project_config.render())?;
        cache_key = cache::key_with_extra_and_roots(
            &input,
            &project_target,
            &[
                config_path.as_path(),
                app_images_path.as_path(),
                translations_path.as_path(),
            ],
            &sorted_plugin_roots,
        )
        .map_err(|error| format!("project cache: {error}"))?;
    }
    if let Some(flavor) = &flavor {
        project_config = project_config.with_flavor(flavor)?;
    }
    if let Some(arch) = &arch {
        let target = match target {
            ProjectTarget::Ios => "ios",
            ProjectTarget::Android => "android",
            ProjectTarget::All => "all",
        };
        project_config.set_arch_override(arch, target)?;
    }

    for (compile_target, _, packages) in &compiled {
        if *compile_target == Target::Kotlin {
            let requirements = packages
                .iter()
                .map(|plugin| (plugin.namespace.as_str(), plugin.artifacts.android_min_sdk))
                .collect::<Vec<_>>();
            project_config.validate_android_plugin_minimums(&requirements)?;
        }
    }

    fs::create_dir_all(&output).map_err(|error| format!("{}: {error}", output.display()))?;

    let mut generated_targets = Vec::new();
    for (compile_target, module, packages) in compiled {
        match compile_target {
            Target::Swift => {
                generate_ios(
                    &output,
                    project_root,
                    &app_name,
                    &module,
                    &packages,
                    &project_config,
                    dev_session.as_ref(),
                )?;
                generated_targets.push("ios");
            }
            Target::Kotlin => {
                generate_android(
                    &output,
                    project_root,
                    &app_name,
                    &module,
                    &packages,
                    &project_config,
                    dev_session.as_ref(),
                )?;
                generated_targets.push("android");
            }
            Target::All => unreachable!("project generation compiles concrete platform targets"),
        }
    }

    let entry = input.canonicalize().unwrap_or(input.clone());
    let has_ios_privacy_manifest = generated_targets.contains(&"ios")
        && output
            .join("ios")
            .join(&app_name)
            .join("PrivacyInfo.xcprivacy")
            .is_file();
    let manifest = format!(
        "{{\n  \"format\": 1,\n  \"entry\": \"{}\",\n  \"name\": \"{}\",\n  \"targets\": [{}],\n  \"sourceManifest\": \"nexa.sources.json\",\n  \"iosPrivacyManifest\": {},\n  \"cacheKey\": \"{}\"\n}}\n",
        json_escape(&entry.display().to_string()),
        json_escape(&app_name),
        generated_targets
            .iter()
            .map(|target| format!("\"{target}\""))
            .collect::<Vec<_>>()
            .join(", "),
        has_ios_privacy_manifest,
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
    if print_summary {
        println!(
            "generated {} ({})",
            output.display(),
            generated_targets.join(", ")
        );
    }
    Ok(())
}

/// Compile the current project into the serializable representation consumed
/// by Nexa's debug development server. Release generation never calls this.
pub(crate) fn compile_dev_modules_with_compiler(
    entry: &Path,
    platform: &str,
    compiler: &mut nexa_compiler::IncrementalProjectCompiler,
) -> Result<Vec<(nexa_dev_protocol::TargetPlatform, nexa_dev_ir::DevModule)>, String> {
    let targets: &[Target] = match platform {
        "ios" => &[Target::Swift],
        "android" => &[Target::Kotlin],
        "all" => &[Target::Swift, Target::Kotlin],
        value => return Err(format!("unknown platform `{value}`")),
    };
    let project_root = entry.parent().unwrap_or_else(|| Path::new("."));
    let config_path = project_root.join("nexa.config.nx");
    let app_images_path = project_root.join("assets/images");
    let translations_path = project_root.join("locales/translations.json");
    let dependencies = config::load_plugin_dependencies(&config_path)?;
    let resolved = crate::dependencies::resolve(project_root, &dependencies)?;
    let compilations = compiler
        .compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
            entry,
            targets,
            &resolved.plugin_roots,
        )
        .map_err(|error| error.to_string())?;
    let translation_modules = compilations
        .iter()
        .map(|compilation| &compilation.module)
        .collect::<Vec<_>>();
    let translations = localization::synchronize_modules(project_root, &translation_modules)?;
    let dev_translations = translations.dev_translations();
    let sorted_roots = resolved
        .plugin_roots
        .iter()
        .map(|(id, path)| (id.clone(), path.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    targets
        .iter()
        .copied()
        .zip(compilations)
        .map(|(target, compilation)| {
            let platform = match target {
                Target::Swift => nexa_dev_protocol::TargetPlatform::Ios,
                Target::Kotlin => nexa_dev_protocol::TargetPlatform::Android,
                Target::All => unreachable!("dev compiler targets are concrete"),
            };
            for warning in &compilation.warnings {
                eprintln!("{warning}");
            }
            let cache_target = match target {
                Target::Swift => "dev-ios",
                Target::Kotlin => "dev-android",
                Target::All => unreachable!("dev compiler targets are concrete"),
            };
            let revision = cache::key_with_extra_and_roots(
                entry,
                cache_target,
                &[
                    config_path.as_path(),
                    app_images_path.as_path(),
                    translations_path.as_path(),
                ],
                &sorted_roots,
            )
            .map_err(|error| format!("dev revision: {error}"))?;
            Ok((
                platform,
                nexa_dev_ir::lower_with_translations(
                    &compilation.module,
                    revision,
                    dev_translations.clone(),
                ),
            ))
        })
        .collect()
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
        if *target == "android" {
            collect_source_units(&root.join("android/app/src/debug"), root, &mut files)?;
        }
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
            .is_some_and(|extension| {
                matches!(
                    extension,
                    "swift" | "kt" | "xml" | "plist" | "json" | "xcprivacy"
                )
            })
            || path
                .components()
                .any(|component| component.as_os_str() == "Assets.xcassets")
            || path
                .components()
                .any(|component| component.as_os_str() == "NexaPluginResources")
            || (path
                .components()
                .any(|component| component.as_os_str() == "assets")
                && path
                    .components()
                    .any(|component| component.as_os_str() == "plugins"))
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

/// Generates the iOS host project.
///
/// The work is split in three. `prepare_ios` performs the filesystem copies
/// whose results the Xcode project has to reference. `ios_plan` then computes
/// every path and file body from those facts without touching disk, and the
/// writer only materializes the validated plan.
fn generate_ios(
    root: &Path,
    source_root: &Path,
    app_name: &str,
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
    dev_session: Option<&DevSessionConfig>,
) -> Result<(), String> {
    validate_ios_background_task_minimum(module, plugins, config)?;
    let dev_runtime = dev_session.is_some();
    let prepared = prepare_ios(
        root,
        source_root,
        app_name,
        module,
        plugins,
        config,
        dev_runtime,
    )?;
    let plan = ios_plan(app_name, module, plugins, config, dev_session, &prepared)?;
    writers::write_plan(root, &plan)?;
    writers::remove_stale_units(
        &root.join("ios").join(app_name),
        &generated_unit_names(&plan, dev_runtime),
        plan.platform().unit_extension(),
    )
}

const IOS_DEV_RUNTIME_FILES: &[(&str, &str)] = &[
    (
        "NexaDevValueCodec.swift",
        include_str!("../../../runtime/ios/NexaDevValueCodec.swift"),
    ),
    (
        "NexaDevSchema.swift",
        include_str!("../../../runtime/ios/NexaDevSchema.swift"),
    ),
    (
        "NexaDevProtocol.swift",
        include_str!("../../../runtime/ios/NexaDevProtocol.swift"),
    ),
    (
        "NexaDevState.swift",
        include_str!("../../../runtime/ios/NexaDevState.swift"),
    ),
    (
        "NexaDevClasses.swift",
        include_str!("../../../runtime/ios/NexaDevClasses.swift"),
    ),
    (
        "NexaDevActions.swift",
        include_str!("../../../runtime/ios/NexaDevActions.swift"),
    ),
    (
        "NexaDevNavigation.swift",
        include_str!("../../../runtime/ios/NexaDevNavigation.swift"),
    ),
    (
        "NexaDevNativeApis.swift",
        include_str!("../../../runtime/ios/NexaDevNativeApis.swift"),
    ),
    (
        "NexaDevOverlay.swift",
        include_str!("../../../runtime/ios/NexaDevOverlay.swift"),
    ),
    (
        "NexaDevRenderer.swift",
        include_str!("../../../runtime/ios/NexaDevRenderer.swift"),
    ),
    (
        "NexaDevRuntime.swift",
        include_str!("../../../runtime/ios/NexaDevRuntime.swift"),
    ),
];

const ANDROID_DEV_RUNTIME_FILES: &[(&str, &str)] = &[
    (
        "NexaDevValueCodec.kt",
        include_str!("../../../runtime/android/NexaDevValueCodec.kt"),
    ),
    (
        "NexaDevSchema.kt",
        include_str!("../../../runtime/android/NexaDevSchema.kt"),
    ),
    (
        "NexaDevProtocol.kt",
        include_str!("../../../runtime/android/NexaDevProtocol.kt"),
    ),
    (
        "NexaDevState.kt",
        include_str!("../../../runtime/android/NexaDevState.kt"),
    ),
    (
        "NexaDevActions.kt",
        include_str!("../../../runtime/android/NexaDevActions.kt"),
    ),
    (
        "NexaDevNavigation.kt",
        include_str!("../../../runtime/android/NexaDevNavigation.kt"),
    ),
    (
        "NexaDevNativeApis.kt",
        include_str!("../../../runtime/android/NexaDevNativeApis.kt"),
    ),
    (
        "NexaDevOverlay.kt",
        include_str!("../../../runtime/android/NexaDevOverlay.kt"),
    ),
    (
        "NexaDevRenderer.kt",
        include_str!("../../../runtime/android/NexaDevRenderer.kt"),
    ),
    (
        "NexaDevRuntime.kt",
        include_str!("../../../runtime/android/NexaDevRuntime.kt"),
    ),
];

fn dev_runtime_source(filename: &str, template: &str, package: &str, module: &Module) -> String {
    let mut source = template.replace("__NEXA_PACKAGE__", package);
    match filename {
        "NexaDevRenderer.swift" => {
            let cases = SHARED_ICONS
                .iter()
                .map(|icon| format!("    case \"{}\": return \"{}\"", icon.name, icon.sf_symbol))
                .collect::<Vec<_>>()
                .join("\n");
            source = source.replace("__NEXA_SHARED_ICON_SF_CASES__", &cases);
        }
        "NexaDevRenderer.kt" => {
            let material_cases = SHARED_ICONS
                .iter()
                .map(|icon| {
                    format!(
                        "    \"{}\" -> Icons.{}.{}",
                        icon.name, icon.material_namespace, icon.material_name
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let sf_alias_cases = SHARED_ICONS
                .iter()
                .flat_map(|icon| {
                    icon.aliases.iter().map(move |alias| {
                        format!(
                            "    \"{alias}\" -> Icons.{}.{}",
                            icon.material_namespace, icon.material_name
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join("\n");
            let auto_mirrored_imports = SHARED_ICONS
                .iter()
                .filter(|icon| icon.material_namespace.starts_with("AutoMirrored."))
                .map(|icon| {
                    format!(
                        "import androidx.compose.material.icons.{}.{}",
                        icon.material_namespace.to_lowercase(),
                        icon.material_name
                    )
                })
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join("\n");
            let used_icons = nexa_ir::facts::ModuleFacts::analyze(module).ui.system_icons;
            let material_symbol_icons = used_icons
                .iter()
                .filter_map(|icon| match icon {
                    nexa_ir::SystemIcon::MaterialSymbol(name) => Some((
                        name.clone(),
                        icon.material_reference(),
                        icon.material_import(),
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let material_symbol_cases = material_symbol_icons
                .iter()
                .map(|(name, reference, _)| format!("    \"{name}\" -> {reference}"))
                .collect::<Vec<_>>()
                .join("\n");
            let specific_material_imports = material_symbol_icons
                .iter()
                .map(|(_, _, import)| import.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .map(|import| format!("import {import}"))
                .collect::<Vec<_>>()
                .join("\n");
            source = source
                .replace("__NEXA_SHARED_ICON_MATERIAL_CASES__", &material_cases)
                .replace("__NEXA_SHARED_ICON_SF_ALIAS_CASES__", &sf_alias_cases)
                .replace(
                    "__NEXA_SHARED_ICON_AUTO_MIRRORED_IMPORTS__",
                    &auto_mirrored_imports,
                )
                .replace(
                    "__NEXA_SPECIFIC_MATERIAL_ICON_CASES__",
                    &material_symbol_cases,
                )
                .replace(
                    "__NEXA_SPECIFIC_MATERIAL_ICON_IMPORTS__",
                    &specific_material_imports,
                );
        }
        _ => {}
    }
    if matches!(filename, "NexaDevRenderer.swift" | "NexaDevRenderer.kt") {
        use nexa_codegen::design_system as design;

        let muted_red = (design::MUTED_TEXT_ARGB >> 16) & 0xFF;
        let muted_green = (design::MUTED_TEXT_ARGB >> 8) & 0xFF;
        let muted_blue = design::MUTED_TEXT_ARGB & 0xFF;
        let replacements = [
            (
                "__NEXA_DEFAULT_BODY_FONT_SIZE__",
                design::DEFAULT_BODY_FONT_SIZE.to_string(),
            ),
            (
                "__NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__",
                design::DEFAULT_LINE_HEIGHT_MULTIPLIER.to_string(),
            ),
            (
                "__NEXA_BUTTON_MIN_WIDTH__",
                design::BUTTON_MIN_WIDTH.to_string(),
            ),
            (
                "__NEXA_BUTTON_MIN_TAP_TARGET__",
                design::BUTTON_MIN_TAP_TARGET.to_string(),
            ),
            (
                "__NEXA_BUTTON_LARGE_MIN_HEIGHT__",
                design::BUTTON_LARGE_MIN_HEIGHT.to_string(),
            ),
            (
                "__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__",
                design::BUTTON_SMALL_HORIZONTAL_PADDING.to_string(),
            ),
            (
                "__NEXA_BUTTON_SMALL_VERTICAL_PADDING__",
                design::BUTTON_SMALL_VERTICAL_PADDING.to_string(),
            ),
            (
                "__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__",
                design::BUTTON_LARGE_HORIZONTAL_PADDING.to_string(),
            ),
            (
                "__NEXA_BUTTON_LARGE_VERTICAL_PADDING__",
                design::BUTTON_LARGE_VERTICAL_PADDING.to_string(),
            ),
            (
                "__NEXA_PAGE_INDICATOR_SELECTED_SIZE__",
                design::PAGE_INDICATOR_SELECTED_SIZE.to_string(),
            ),
            (
                "__NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__",
                design::PAGE_INDICATOR_UNSELECTED_SIZE.to_string(),
            ),
            (
                "__NEXA_PAGE_INDICATOR_SPACING__",
                design::PAGE_INDICATOR_SPACING.to_string(),
            ),
            (
                "__NEXA_PAGE_INDICATOR_BOTTOM_INSET__",
                design::PAGE_INDICATOR_BOTTOM_INSET.to_string(),
            ),
            (
                "__NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__",
                design::PAGE_INDICATOR_INACTIVE_OPACITY.to_string(),
            ),
            (
                "__NEXA_DEFAULT_ACCENT_ARGB__",
                format!("0x{:08X}", design::DEFAULT_ACCENT_ARGB),
            ),
            (
                "__NEXA_SWIFT_DEFAULT_ACCENT_COLOR__",
                design::SWIFT_DEFAULT_ACCENT_COLOR.to_owned(),
            ),
            (
                "__NEXA_DARK_ACCENT_ARGB__",
                format!("0x{:08X}", design::DARK_ACCENT_ARGB),
            ),
            (
                "__NEXA_LIGHT_PRIMARY_CONTAINER_ARGB__",
                format!("0x{:08X}", design::LIGHT_PRIMARY_CONTAINER_ARGB),
            ),
            (
                "__NEXA_DARK_PRIMARY_CONTAINER_ARGB__",
                format!("0x{:08X}", design::DARK_PRIMARY_CONTAINER_ARGB),
            ),
            (
                "__NEXA_LIGHT_ON_PRIMARY_CONTAINER_ARGB__",
                format!("0x{:08X}", design::LIGHT_ON_PRIMARY_CONTAINER_ARGB),
            ),
            (
                "__NEXA_DARK_ON_PRIMARY_CONTAINER_ARGB__",
                format!("0x{:08X}", design::DARK_ON_PRIMARY_CONTAINER_ARGB),
            ),
            (
                "__NEXA_LIGHT_BACKGROUND_ARGB__",
                format!("0x{:08X}", design::LIGHT_BACKGROUND_ARGB),
            ),
            (
                "__NEXA_DARK_BACKGROUND_ARGB__",
                format!("0x{:08X}", design::DARK_BACKGROUND_ARGB),
            ),
            (
                "__NEXA_LIGHT_SURFACE_ARGB__",
                format!("0x{:08X}", design::LIGHT_SURFACE_ARGB),
            ),
            (
                "__NEXA_DARK_SURFACE_ARGB__",
                format!("0x{:08X}", design::DARK_SURFACE_ARGB),
            ),
            (
                "__NEXA_LIGHT_SURFACE_VARIANT_ARGB__",
                format!("0x{:08X}", design::LIGHT_SURFACE_VARIANT_ARGB),
            ),
            (
                "__NEXA_DARK_SURFACE_VARIANT_ARGB__",
                format!("0x{:08X}", design::DARK_SURFACE_VARIANT_ARGB),
            ),
            (
                "__NEXA_LIGHT_OUTLINE_ARGB__",
                format!("0x{:08X}", design::LIGHT_OUTLINE_ARGB),
            ),
            (
                "__NEXA_DARK_OUTLINE_ARGB__",
                format!("0x{:08X}", design::DARK_OUTLINE_ARGB),
            ),
            (
                "__NEXA_LIGHT_ON_SURFACE_ARGB__",
                format!("0x{:08X}", design::LIGHT_ON_SURFACE_ARGB),
            ),
            (
                "__NEXA_DARK_ON_SURFACE_ARGB__",
                format!("0x{:08X}", design::DARK_ON_SURFACE_ARGB),
            ),
            (
                "__NEXA_MUTED_TEXT_ARGB__",
                format!("0x{:08X}", design::MUTED_TEXT_ARGB),
            ),
            (
                "__NEXA_DEFAULT_ERROR_ARGB__",
                format!("0x{:08X}", design::DEFAULT_ERROR_ARGB),
            ),
            (
                "__NEXA_MUTED_TEXT_RED__",
                format!("{:.6}", f64::from(muted_red) / 255.0),
            ),
            (
                "__NEXA_MUTED_TEXT_GREEN__",
                format!("{:.6}", f64::from(muted_green) / 255.0),
            ),
            (
                "__NEXA_MUTED_TEXT_BLUE__",
                format!("{:.6}", f64::from(muted_blue) / 255.0),
            ),
        ];
        for (placeholder, value) in replacements {
            source = source.replace(placeholder, &value);
        }
    }
    source
}

/// Every generated file name in an iOS source directory, including the dev
/// runtime when this build uses it. Used to clear units a previous run left.
fn generated_unit_names(plan: &ProjectPlan, dev_runtime: bool) -> Vec<String> {
    let mut names: Vec<String> = plan
        .source_units()
        .iter()
        .map(|unit| unit.name.clone())
        .collect();
    if dev_runtime {
        for (filename, _) in IOS_DEV_RUNTIME_FILES {
            names.push((*filename).to_owned());
        }
    }
    names
}

/// Facts gathered by the filesystem work, which the Xcode project references.
struct PreparedIos {
    source_units: Vec<nexa_codegen::SourceUnit>,
    privacy_manifest: Option<String>,
    supports_screen_orientation: bool,
    /// Whether the app bundle carries images, icons, or a splash screen.
    has_assets: bool,
    /// Swift files staged from plugin packages, relative to the app directory.
    plugin_sources: Vec<String>,
    /// C++ sources staged for the bridging header.
    cpp_sources: Vec<String>,
    /// XCFrameworks vendored into the app.
    xcframeworks: Vec<String>,
    /// The XCFrameworks to copy, and the ones a previous run staged that this
    /// build no longer produces.
    artifacts: Vec<plan::CopyAction>,
    removed_artifacts: Vec<String>,
    /// Plugin resources staged for the app bundle.
    resources: plugins::StagedResources,
    /// Apple string catalog compiled as a main-bundle localization resource.
    localization_catalog: Option<String>,
    /// Backend-generated WidgetKit units and explicitly extension-safe plugin
    /// sources compiled only by the widget extension target.
    widget_sources: Vec<NativeWidgetSource>,
    previous_widget_sources: Vec<String>,
}

/// A generated native widget source, addressed relative to its target-owned
/// output directory.
#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeWidgetSource {
    relative_path: String,
    contents: String,
}

/// Performs the iOS filesystem copies and gathers the facts they produce.
fn prepare_ios(
    root: &Path,
    source_root: &Path,
    app_name: &str,
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
    dev_runtime: bool,
) -> Result<PreparedIos, String> {
    let directory = root.join("ios").join(app_name);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let (source_units, project_features) = ios_source_units(module, plugins, config, dev_runtime)?;
    let widget_sources = generated_ios_widget_sources(module, plugins, config)?;
    let mut previous_widget_sources =
        read_generated_marker(&directory.join(".nexa-widget-sources"))?;
    let widget_directory = directory.join("NexaWidgets");
    if widget_directory.is_dir() {
        previous_widget_sources.extend(
            fs::read_dir(&widget_directory)
                .map_err(|error| format!("{}: {error}", widget_directory.display()))?
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect::<Vec<_>>(),
        );
    }
    previous_widget_sources.sort();
    previous_widget_sources.dedup();
    let privacy_manifest = templates::ios_privacy_manifest(project_features);
    let supports_screen_orientation = project_features.uses_screen_orientation_api;
    copy_config_icons(root, app_name, config)?;
    let ios_icon = config.ios_icon.as_ref().or(config.icon_source.as_ref());
    let has_project_images = assets::copy_ios_project_images(source_root, root, app_name)?;
    let has_assets = plugins::copy_plugin_assets(root, app_name, module)?
        || has_project_images
        || ios_icon
            .is_some_and(|path| path.is_dir() || path.extension().is_some_and(|ext| ext != "icon"))
        || config.splash_source.is_some();
    let plugin_sources = plugins::copy_ios_plugin_sources(root, app_name, plugins)?;
    let cpp_sources = plugins::copy_ios_plugin_cpp_sources(root, app_name, plugins)?;
    let (staged_artifacts, previous_artifacts) =
        plugins::stage_ios_plugin_artifacts(root, app_name, plugins)?;
    let xcframeworks = staged_artifacts
        .iter()
        .map(|artifact| artifact.name.clone())
        .collect::<Vec<_>>();
    let resources = plugins::stage_ios_plugin_resources(root, app_name, plugins)?;
    let translations = localization::synchronize(source_root, module)?;
    let localization_catalog = if translations.strings.is_empty() {
        None
    } else {
        Some(translations.apple_string_catalog()?)
    };
    Ok(PreparedIos {
        source_units,
        privacy_manifest,
        supports_screen_orientation,
        has_assets,
        plugin_sources,
        cpp_sources,
        xcframeworks: xcframeworks.clone(),
        artifacts: staged_artifacts
            .iter()
            .map(|artifact| artifact.copy.clone())
            .collect(),
        removed_artifacts: previous_artifacts
            .iter()
            .filter(|name| !xcframeworks.iter().any(|current| current == *name))
            .map(|name| format!("ios/{app_name}/{name}"))
            .collect(),
        resources,
        localization_catalog,
        widget_sources,
        previous_widget_sources,
    })
}

/// Computes every iOS file and path from prepared facts, without touching disk.
fn ios_plan(
    app_name: &str,
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
    dev_session: Option<&DevSessionConfig>,
    prepared: &PreparedIos,
) -> Result<ProjectPlan, String> {
    let screen = nexa_codegen::names::screen_name(&module.app_name);
    let directory = format!("ios/{app_name}");
    let dev_runtime = dev_session.is_some();
    let app_root = match dev_session {
        Some(dev_session) => format!(
            "NexaDevRuntimeRoot(serverURL: \"{}\", sessionToken: \"{}\")",
            swift_escape(&dev_session.server_url),
            swift_escape(&dev_session.session_token)
        ),
        None => format!("{screen}()"),
    };
    let generated_names = generated_unit_names(
        &ProjectPlan::ios(app_name).with_source_units(prepared.source_units.clone()),
        dev_runtime,
    );
    let mut plan = ProjectPlan::ios(app_name)
        .with_source_directory(directory.clone())
        .with_source_units(prepared.source_units.clone());
    match &prepared.privacy_manifest {
        Some(contents) => {
            plan = plan.with_file(
                format!("{directory}/PrivacyInfo.xcprivacy"),
                contents.clone(),
            );
        }
        None => {
            plan = plan.with_removal(format!("{directory}/PrivacyInfo.xcprivacy"));
        }
    }
    for copy in &prepared.artifacts {
        plan = plan.with_copy(copy.clone());
    }
    for stale in &prepared.removed_artifacts {
        plan = plan.with_removal(stale.clone());
    }
    // Plugin resources are staged relative to the resource root so the marker
    // survives the app being renamed; the plan prefixes them on the way in.
    let resource_directory = plugins::ios_resource_directory(app_name);
    for resource in &prepared.resources.copies {
        plan = plan.with_copy(resource.copy.clone());
    }
    for file in &prepared.resources.generated {
        plan = plan.with_file(
            format!("{resource_directory}/{}", file.relative),
            file.contents.clone(),
        );
    }
    let staged_now: Vec<&str> = prepared
        .resources
        .copies
        .iter()
        .map(|resource| resource.relative.as_str())
        .chain(
            prepared
                .resources
                .generated
                .iter()
                .map(|file| file.relative.as_str()),
        )
        .collect();
    for stale in &prepared.resources.previous {
        if !staged_now.contains(&stale.as_str()) {
            plan = plan.with_removal(format!("{resource_directory}/{stale}"));
        }
    }
    if let Some(catalog) = &prepared.localization_catalog {
        plan = plan.with_file(
            format!("{directory}/Localizable.xcstrings"),
            catalog.clone(),
        );
    } else {
        plan = plan.with_removal(format!("{directory}/Localizable.xcstrings"));
    }
    if prepared.resources.present {
        plan = plan.with_file(
            format!("{resource_directory}/.nexa-plugin-resources"),
            format!("{}\n", staged_now.join("\n")),
        );
    } else {
        plan = plan.with_removal(format!("{resource_directory}/.nexa-plugin-resources"));
    }
    // The marker records what this build staged, so the next run can tell what
    // to remove. It is a planned file like any other, which keeps staging free
    // of writes and stale removal inside the plan.
    if prepared.xcframeworks.is_empty() {
        plan = plan.with_removal(format!("{directory}/.nexa-plugin-frameworks"));
    } else {
        plan = plan.with_file(
            format!("{directory}/.nexa-plugin-frameworks"),
            format!("{}\n", prepared.xcframeworks.join("\n")),
        );
    }
    plan = plan
        .with_file(
            format!("{directory}/{app_name}App.swift"),
            templates::ios_app_source(
                app_name,
                &app_root,
                &module.background_tasks,
                plugins,
                !prepared.widget_sources.is_empty(),
            )?,
        )
        .with_file(
            format!("ios/{app_name}.xcodeproj/xcshareddata/xcschemes/{app_name}.xcscheme"),
            templates::ios_scheme(app_name),
        )
        .with_file(
            format!("ios/{app_name}.xcodeproj/project.pbxproj"),
            templates::ios_project_file_with_localization_config_and_widgets(
                app_name,
                prepared.has_assets,
                prepared.resources.present,
                prepared.localization_catalog.is_some(),
                prepared.privacy_manifest.is_some(),
                &generated_names,
                &prepared.plugin_sources,
                &prepared.cpp_sources,
                &prepared.xcframeworks,
                plugins,
                config,
                &prepared.widget_sources,
                module
                    .widgets
                    .iter()
                    .any(|widget| widget.configuration.is_some()),
            )?,
        )
        .with_file(
            format!("{directory}/Info.plist"),
            templates::ios_info_plist_with_orientation(
                app_name,
                config,
                plugins,
                dev_runtime,
                dev_runtime || prepared.supports_screen_orientation,
                &module
                    .background_tasks
                    .iter()
                    .map(|task| task.identifier.clone())
                    .collect::<Vec<_>>(),
            )?,
        );
    if !prepared.widget_sources.is_empty() {
        plan = plan.with_file(
            format!("{directory}/NexaWidgets-Info.plist"),
            templates::ios_widget_info_plist(app_name, config),
        );
        for source in &prepared.widget_sources {
            plan = plan.with_file(
                format!("{directory}/NexaWidgets/{}", source.relative_path),
                source.contents.clone(),
            );
        }
        plan = plan.with_file(
            format!("{directory}/NexaWidgets.entitlements"),
            templates::ios_widget_entitlements(config),
        );
        let current = prepared
            .widget_sources
            .iter()
            .map(|source| source.relative_path.as_str())
            .collect::<std::collections::HashSet<_>>();
        for stale in &prepared.previous_widget_sources {
            if !current.contains(stale.as_str()) {
                plan = plan.with_removal(format!("{directory}/NexaWidgets/{stale}"));
            }
        }
        plan = plan.with_removal(format!("{directory}/.nexa-widget-sources"));
    } else {
        plan = plan.with_removal(format!("{directory}/NexaWidgets-Info.plist"));
        plan = plan.with_removal(format!("{directory}/NexaWidgets.entitlements"));
        plan = plan.with_removal(format!("{directory}/.nexa-widget-sources"));
        for stale in &prepared.previous_widget_sources {
            plan = plan.with_removal(format!("{directory}/NexaWidgets/{stale}"));
        }
    }
    if config.splash_source.is_some() {
        plan = plan.with_file(
            format!("{directory}/LaunchScreen.storyboard"),
            templates::ios_launch_storyboard(),
        );
    }
    if dev_runtime {
        for (filename, content) in IOS_DEV_RUNTIME_FILES {
            plan = plan.with_file(
                format!("{directory}/{filename}"),
                dev_runtime_source(filename, content, "", module),
            );
        }
    } else {
        for (filename, _) in IOS_DEV_RUNTIME_FILES {
            plan = plan.with_removal(format!("{directory}/{filename}"));
        }
        plan = plan.with_removal(format!("{directory}/NexaDevPluginBridge.swift"));
    }
    match templates::ios_entitlements(config, plugins)? {
        Some(entitlements) => {
            plan = plan.with_file(format!("{directory}/Nexa.entitlements"), entitlements);
        }
        None => {
            plan = plan.with_removal(format!("{directory}/Nexa.entitlements"));
        }
    }
    let has_aps_environment = plugins.iter().any(|plugin| {
        plugin
            .artifacts
            .ios_entitlements
            .iter()
            .any(|(key, _)| key == "aps-environment")
    });
    if has_aps_environment {
        match templates::ios_release_entitlements(config, plugins)? {
            Some(entitlements) => {
                plan = plan.with_file(
                    format!("{directory}/Nexa-Release.entitlements"),
                    entitlements,
                );
            }
            None => {
                plan = plan.with_removal(format!("{directory}/Nexa-Release.entitlements"));
            }
        }
    } else {
        plan = plan.with_removal(format!("{directory}/Nexa-Release.entitlements"));
    }
    plan.validate()?;
    Ok(plan)
}

fn generate_android(
    root: &Path,
    source_root: &Path,
    app_name: &str,
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
    dev_session: Option<&DevSessionConfig>,
) -> Result<(), String> {
    validate_android_background_task_minimum(module, config)?;
    let widget_generated = KotlinBackend
        .generate_widget_units(module)
        .map_err(|error| error.to_string())?;
    let widget_units = widget_generated.sources;
    let widget_resources = widget_generated.resources;
    validate_android_widget_resources(&widget_resources, module)?;
    let package = config.android_application_id.clone();
    let package_path = package.replace('.', "/");
    let source_dir = root.join("android/app/src/main/java").join(&package_path);
    let stale_widget_sources = read_generated_marker(&source_dir.join(".nexa-widget-sources"))?;
    let stale_widget_resources =
        read_generated_marker(&root.join("android/app/src/main/res/.nexa-widget-resources"))?;
    let widget_xml_directory = root.join("android/app/src/main/res/xml");
    let stale_widget_xml = if widget_xml_directory.is_dir() {
        fs::read_dir(&widget_xml_directory)
            .map_err(|error| format!("{}: {error}", widget_xml_directory.display()))?
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.strip_prefix("nexa_widget_info_")
                    .and_then(|value| value.strip_suffix(".xml"))
                    .and_then(|index| index.parse::<usize>().ok())
                    .is_some_and(|index| index >= module.widgets.len())
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    fs::create_dir_all(&source_dir)
        .map_err(|error| format!("{}: {error}", source_dir.display()))?;
    let (mut sources, mut project_features) = if dev_session.is_some() {
        KotlinBackend.generate_for_dev_units_with_project_features(module)
    } else {
        KotlinBackend.generate_units_with_project_features(module)
    };
    if !module.widgets.is_empty() {
        // The normal app generation already emits application types and
        // functions. Widget-specific files reference those declarations, so
        // only merge the per-widget units and their import set here.
        let imports = sources
            .imports
            .lines()
            .chain(widget_units.imports.lines())
            .filter(|line| line.starts_with("import "))
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>();
        sources.imports = if imports.is_empty() {
            String::new()
        } else {
            imports.iter().map(|line| format!("{line}\n")).collect()
        };
        let mut merged_unit_names = sources
            .units
            .iter()
            .map(|unit| unit.name.clone())
            .collect::<std::collections::HashSet<_>>();
        sources
            .units
            .extend(widget_units.units.into_iter().filter(|unit| {
                (unit.name.starts_with("widget-")
                    || unit.name == "NexaWidgetFamily.kt"
                    || unit.contents.contains("WidgetConfigurationActivity"))
                    && merged_unit_names.insert(unit.name.clone())
            }));
    }
    if dev_session.is_some() || project_features.uses_network {
        sources.units.push(nexa_codegen::SourceUnit {
            name: "NexaCronetConfig.kt".to_owned(),
            contents: format!(
                "internal object NexaCronetConfig {{\n    const val DISK_CACHE_SIZE_BYTES: Long = {}L * 1024L * 1024L\n}}\n",
                config.android_cronet_disk_cache_size_mb
            ),
        });
    }
    if dev_session.is_some() {
        let contracts = dev_plugin_contracts(plugins)?;
        sources.units.push(nexa_codegen::SourceUnit {
            name: "NexaDevPluginBridge.kt".to_owned(),
            contents: nexa_codegen::plugin::render_dev_bridge_kotlin(&contracts)?,
        });
    }
    // The debug runtime contains a generic Navigation Compose host even when
    // the app's own tree does not currently declare navigation.
    project_features.uses_navigation |= dev_session.is_some();
    let (plugin_packages, plugin_uses_coroutines) =
        plugins::copy_android_plugin_sources(root, plugins, &package, config)?;
    project_features.uses_coroutines |= plugin_uses_coroutines;
    plugins::copy_android_plugin_cpp_sources(root, plugins, &package)?;
    let local_aars = plugins::copy_android_plugin_artifacts(root, plugins)?;
    let resources = plugins::stage_android_plugin_resources(root, plugins)?;
    copy_config_icons(root, app_name, config)?;
    plugins::copy_plugin_assets(root, app_name, module)?;
    assets::copy_android_project_images(source_root, root)?;
    assets::copy_android_localizations(source_root, root, module)?;
    // Every generated file needs the package declaration, and plugin bindings
    // add an import between the package line and the generator's own imports.
    // The header is assembled here rather than by rewriting a concatenated
    // string, so the unit files are written exactly as computed.
    let plugin_imports = plugin_packages
        .iter()
        .map(|plugin_package| format!("import {plugin_package}.*"))
        .collect::<Vec<_>>();
    let mut header_lines = vec![format!("package {package}"), String::new()];
    if !plugin_imports.is_empty() {
        header_lines.extend(plugin_imports.iter().cloned());
        header_lines.push(String::new());
    }
    // `header_with` keeps the generator's own import block spacing, including
    // the blank line it ends with.
    header_lines.push(sources.header_with(&[]));
    let header = header_lines.join("\n");
    let generated_units = sources.into_files_with_header(
        &header,
        &plugins::render_kotlin_plugin_config(plugins, config),
    );
    let generated_names = generated_units
        .iter()
        .map(|unit| unit.name.clone())
        .collect::<Vec<_>>();

    // The generated core runtime is written outside the app's package: a
    // plugin lives in its own package and still has to reach the application
    // context, and a generic plugin contract names the value codec. The codec
    // is emitted when app calls or included plugin declarations need it.
    let core_package_path = nexa_codegen::value::KOTLIN_CORE_PACKAGE.replace('.', "/");
    let mut value_codec_files = vec![(
        format!("{core_package_path}/NexaRuntimeCore.kt"),
        nexa_codegen::value::kotlin_core_runtime_source(),
    )];
    let plugin_value_runtime =
        dev_session.is_none() && plugin_packages_require_value_runtime(plugins)?;
    if !nexa_codegen::value::collect(module).is_empty()
        || dev_session.is_some()
        || plugin_value_runtime
    {
        value_codec_files.push((
            format!("{core_package_path}/NexaValue.kt"),
            nexa_codegen::value::kotlin_runtime_source(),
        ));
    }

    let mut plan = android_plan(
        app_name,
        module,
        nexa_codegen::names::screen_name(&module.app_name),
        &package,
        &package_path,
        &generated_units,
        config,
        plugins,
        dev_session,
        project_features,
        &module.background_tasks,
        &local_aars,
        &resources,
        &widget_resources,
    )?;
    for filename in stale_widget_xml {
        plan = plan.with_removal(format!("android/app/src/main/res/xml/{filename}"));
    }
    for filename in stale_widget_sources {
        plan = plan.with_removal(format!(
            "android/app/src/main/java/{package_path}/{filename}"
        ));
    }
    let mut current_widget_resources = module
        .widgets
        .iter()
        .enumerate()
        .map(|(index, _)| format!("xml/{}.xml", templates::android_widget_resource_name(index)))
        .chain(
            widget_resources
                .iter()
                .map(|resource| resource.name.clone()),
        )
        .collect::<std::collections::HashSet<_>>();
    current_widget_resources.insert("values/nexa_widget_strings.xml".to_owned());
    current_widget_resources.insert("xml/nexa_widget_info.xml".to_owned());
    for relative in stale_widget_resources {
        if !current_widget_resources.contains(&relative) {
            plan = plan.with_removal(format!("android/app/src/main/res/{relative}"));
        }
    }
    let plan = plan.with_files(
        value_codec_files
            .iter()
            .map(|(path, contents)| {
                (
                    format!("android/app/src/main/java/{path}"),
                    contents.clone(),
                )
            })
            .collect::<Vec<_>>(),
    );
    writers::write_plan(root, &plan)?;
    let mut keep = generated_names;
    if dev_session.is_some() {
        for (filename, _) in ANDROID_DEV_RUNTIME_FILES {
            keep.push((*filename).to_owned());
        }
    }
    // The core runtime lives outside the app's source directory, so the unit
    // sweep never sees it. An app that stopped using generic plugin calls must
    // not keep compiling a value codec no app or plugin contract references.
    if value_codec_files.len() == 1 {
        let _ = fs::remove_file(
            root.join("android/app/src/main/java")
                .join(&core_package_path)
                .join("NexaValue.kt"),
        );
    }
    writers::remove_stale_units(&source_dir, &keep, plan.platform().unit_extension())
}

pub(crate) fn validate_ios_background_task_minimum(
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
) -> Result<(), String> {
    if module.background_tasks.is_empty() {
        return Ok(());
    }
    let minimum = templates::minimum_ios_version(&config.ios_min_version, plugins)?;
    let parts = minimum
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| format!("invalid iOS minimum version `{minimum}`"))?;
    let major = *parts.first().unwrap_or(&0);
    let minor = *parts.get(1).unwrap_or(&0);
    if (major, minor) < (16, 0) {
        return Err(format!(
            "iOS background tasks require iOS 16.0 or later, but the resolved deployment target is {minimum}. Increase `ios.minVersion` in `nexa.config.nx`."
        ));
    }
    Ok(())
}

pub(crate) fn validate_android_background_task_minimum(
    module: &Module,
    config: &ProjectConfig,
) -> Result<(), String> {
    if !module.background_tasks.is_empty() && config.android_min_sdk < 23 {
        return Err(format!(
            "Android background tasks require minSdk 23 because WorkManager 2.11.2 does not support API levels below 23; increase `android.minSdk` from {} in `nexa.config.nx`.",
            config.android_min_sdk
        ));
    }
    Ok(())
}

/// Computes every Android file and path, without touching the filesystem.
#[allow(clippy::too_many_arguments)]
fn android_plan(
    app_name: &str,
    module: &Module,
    screen: String,
    package: &str,
    package_path: &str,
    source_units: &[nexa_codegen::SourceUnit],
    config: &ProjectConfig,
    plugins: &[plugin_package::PluginPackage],
    dev_session: Option<&DevSessionConfig>,
    project_features: nexa_backend_kotlin::KotlinProjectFeatures,
    background_tasks: &[nexa_ir::BackgroundTask],
    local_aars: &[String],
    resources: &plugins::StagedResources,
    widget_resources: &[nexa_codegen::SourceUnit],
) -> Result<ProjectPlan, String> {
    let dev_runtime = dev_session.is_some();
    let source_directory = format!("android/app/src/main/java/{package_path}");
    let compose_root = match dev_session {
        Some(dev_session) => format!(
            "NexaDevRuntimeRoot(serverURL = \"{}\", sessionToken = \"{}\")",
            kotlin_escape(&dev_session.server_url),
            kotlin_escape(&dev_session.session_token)
        ),
        None => format!("{screen}()"),
    };
    let install_play_services_cronet = project_features.uses_network
        && config.android_cronet_provider == config::AndroidCronetProvider::PlayServices;
    let cronet_import = if install_play_services_cronet {
        "import com.google.android.gms.net.CronetProviderInstaller\n"
    } else {
        ""
    };
    let (splash_install, splash_import) = if config.splash_source.is_some() {
        (
            "        installSplashScreen()\n",
            "import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen\n",
        )
    } else {
        ("", "")
    };
    let (orientation_imports, orientation_apply, orientation_callbacks) = if config.orientation
        == "portrait-phones"
    {
        (
            "import android.content.res.Configuration\n",
            "        dev.nexa.core.NexaRuntimeCore.bind(this)\n        dev.nexa.core.NexaRuntimeCore.applyConfiguredOrientation(this)\n",
            "\n    override fun onConfigurationChanged(newConfig: Configuration) {\n        super.onConfigurationChanged(newConfig)\n        dev.nexa.core.NexaRuntimeCore.applyConfiguredOrientation(this)\n    }\n",
        )
    } else if config.orientation == "portrait" {
        (
            "",
            "        dev.nexa.core.NexaRuntimeCore.bind(this)\n        dev.nexa.core.NexaRuntimeCore.applyConfiguredOrientation(this)\n",
            "",
        )
    } else {
        ("", "", "")
    };
    let background_schedule = if background_tasks.is_empty() {
        String::new()
    } else {
        "        NexaBackgroundWorker.scheduleAll(applicationContext)\n".to_owned()
    };
    let uses_firebase_messaging = plugins.iter().any(|plugin| {
        plugin
            .artifacts
            .android_firebase_messaging_service
            .is_some()
    });
    let uses_notifications_plugin = plugins.iter().any(|plugin| {
        Path::new(&plugin.idl_path)
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "notifications")
    });
    let remote_notification_imports = if uses_firebase_messaging || uses_notifications_plugin {
        "import android.content.Intent\nimport dev.nexa.notifications.NotificationsRemoteHub\n"
    } else {
        ""
    };
    let remote_notification_dispatch = if uses_firebase_messaging || uses_notifications_plugin {
        let mut dispatch = String::new();
        if uses_firebase_messaging {
            dispatch.push_str("        NotificationsRemoteHub.dispatchOpened(intent)\n");
        }
        if uses_notifications_plugin {
            dispatch.push_str("        NotificationsRemoteHub.dispatchLocalOpened(intent)\n");
        }
        dispatch
    } else {
        String::new()
    };
    let remote_notification_new_intent = if uses_firebase_messaging || uses_notifications_plugin {
        let mut dispatch = String::from(
            "\n    override fun onNewIntent(intent: Intent) {\n        super.onNewIntent(intent)\n        setIntent(intent)\n",
        );
        if uses_firebase_messaging {
            dispatch.push_str("        NotificationsRemoteHub.dispatchOpened(intent)\n");
        }
        if uses_notifications_plugin {
            dispatch.push_str("        NotificationsRemoteHub.dispatchLocalOpened(intent)\n");
        }
        dispatch.push_str("    }\n");
        dispatch
    } else {
        String::new()
    };
    let has_widgets = !module.widgets.is_empty();
    let widget_refresh_imports = if has_widgets {
        "import androidx.glance.appwidget.updateAll\nimport androidx.lifecycle.lifecycleScope\nimport kotlinx.coroutines.launch\n"
    } else {
        ""
    };
    let widget_refresh_lifecycle = if has_widgets {
        let refreshes = module
            .widgets
            .iter()
            .map(|widget| {
                format!(
                    "            {}().updateAll(applicationContext)\n",
                    nexa_codegen::names::widget_name(&widget.name)
                )
            })
            .collect::<String>();
        format!(
            "\n    override fun onStop() {{\n        super.onStop()\n        lifecycleScope.launch {{\n{refreshes}        }}\n    }}\n"
        )
    } else {
        String::new()
    };
    let dark_colors = nexa_codegen::design_system::kotlin_color_scheme(true);
    let light_colors = nexa_codegen::design_system::kotlin_color_scheme(false);
    let default_colors = format!("if (isSystemInDarkTheme()) {dark_colors} else {light_colors}");
    let activity_content = if install_play_services_cronet {
        format!(
            "        CronetProviderInstaller.installProvider(this).addOnCompleteListener {{ result ->\n            if (!result.isSuccessful) android.util.Log.w(\"Nexa\", \"Play Services Cronet provider is unavailable; network calls may fail\", result.exception)\n            setContent {{\n                MaterialTheme(\n                    colorScheme = {default_colors},\n                ) {{\n                    Surface(modifier = Modifier.fillMaxSize()) {{\n                        Box(modifier = Modifier.fillMaxSize().safeDrawingPadding()) {{\n                            {compose_root}\n                        }}\n                    }}\n                }}\n            }}\n        }}\n"
        )
    } else {
        format!(
            "        setContent {{\n            MaterialTheme(\n                colorScheme = {default_colors},\n            ) {{\n                Surface(modifier = Modifier.fillMaxSize()) {{\n                    Box(modifier = Modifier.fillMaxSize().safeDrawingPadding()) {{\n                        {compose_root}\n                    }}\n                }}\n            }}\n        }}\n"
        )
    };
    let mut plan = ProjectPlan::android(app_name)
        .with_source_directory(source_directory.clone())
        .with_source_units(source_units.to_vec())
        .with_file(
            format!("{source_directory}/MainActivity.kt"),
            format!(
                "package {package}\n\nimport android.os.Bundle\nimport androidx.activity.ComponentActivity\nimport androidx.activity.enableEdgeToEdge\nimport androidx.activity.compose.setContent\nimport androidx.compose.foundation.isSystemInDarkTheme\nimport androidx.compose.foundation.layout.Box\nimport androidx.compose.foundation.layout.fillMaxSize\nimport androidx.compose.foundation.layout.safeDrawingPadding\nimport androidx.compose.material3.MaterialTheme\nimport androidx.compose.material3.Surface\nimport androidx.compose.material3.darkColorScheme\nimport androidx.compose.material3.lightColorScheme\nimport androidx.compose.ui.graphics.Color\nimport androidx.compose.ui.Modifier\n{splash_import}{cronet_import}{remote_notification_imports}{widget_refresh_imports}{orientation_imports}\nclass MainActivity : ComponentActivity() {{\n    override fun onCreate(savedInstanceState: Bundle?) {{\n{splash_install}        super.onCreate(savedInstanceState)\n        enableEdgeToEdge()\n{orientation_apply}{remote_notification_dispatch}{background_schedule}{activity_content}    }}{orientation_callbacks}{remote_notification_new_intent}{widget_refresh_lifecycle}}}\n"
            ),
        )
        .with_file(
            "android/app/src/main/res/values/nexa_theme.xml",
            "<resources><style name=\"NexaAppTheme\" parent=\"@android:style/Theme.Material.Light.NoActionBar\"><item name=\"android:windowLightStatusBar\">true</item></style></resources>\n",
        )
        .with_file(
            "android/app/src/main/res/values-night/nexa_theme.xml",
            "<resources><style name=\"NexaAppTheme\" parent=\"@android:style/Theme.Material.NoActionBar\"><item name=\"android:windowLightStatusBar\">false</item></style></resources>\n",
        )
        .with_file(
            "android/app/src/main/AndroidManifest.xml",
            templates::android_manifest_with_widgets(
                app_name,
                package,
                project_features.uses_network,
                dev_session.is_some() || project_features.uses_network_connectivity,
                config,
                plugins,
                &module.widgets,
            )?,
        )
        .with_file(
            "android/settings.gradle.kts",
            templates::android_settings(app_name, plugins),
        )
        .with_file("android/build.gradle.kts", templates::android_root_gradle())
        .with_file("android/gradle.properties", templates::android_properties())
        .with_file(
            "android/gradle/wrapper/gradle-wrapper.properties",
            templates::android_gradle_wrapper_properties(),
        )
        .with_file("android/gradlew", templates::ANDROID_GRADLEW)
        .with_executable("android/gradlew")
        .with_file("android/gradlew.bat", templates::ANDROID_GRADLEW_BAT)
        .with_binary(
            "android/gradle/wrapper/gradle-wrapper.jar",
            templates::ANDROID_GRADLE_WRAPPER_JAR,
        )
        .with_file(
            "android/app/build.gradle.kts",
            templates::android_app_gradle_with_widgets_and_dev_runtime(
                package,
                project_features,
                plugins,
                local_aars,
                config,
                dev_runtime,
                has_widgets,
            )?,
        )
        .with_file(
            "android/app/proguard-rules.pro",
            plugins::android_plugin_proguard_rules(plugins, package)?,
        );
    plan = plan
        .with_removal(format!("{source_directory}/.nexa-widget-sources"))
        .with_removal("android/app/src/main/res/xml/nexa_widget_info.xml")
        .with_removal("android/app/src/main/res/values/nexa_widget_strings.xml");
    for (index, widget) in module.widgets.iter().enumerate() {
        plan = plan.with_file(
            format!(
                "android/app/src/main/res/xml/{}.xml",
                templates::android_widget_resource_name(index)
            ),
            templates::android_widget_provider_info(widget, package),
        );
    }
    for resource in widget_resources {
        plan = plan.with_file(
            format!("android/app/src/main/res/{}", resource.name),
            resource.contents.clone(),
        );
    }
    if widget_resources.is_empty() {
        plan = plan.with_removal("android/app/src/main/res/.nexa-widget-resources");
    } else {
        let mut resource_paths = widget_resources
            .iter()
            .map(|resource| resource.name.as_str())
            .collect::<Vec<_>>();
        resource_paths.sort_unstable();
        plan = plan.with_file(
            "android/app/src/main/res/.nexa-widget-resources",
            format!("{}\n", resource_paths.join("\n")),
        );
    }
    // Legacy marker and resource names are removed as part of the transition
    // to declaration-indexed generated metadata.
    let background_worker_path = format!("{source_directory}/NexaBackgroundWorker.kt");
    if background_tasks.is_empty() {
        plan = plan.with_removal(background_worker_path);
    } else {
        plan = plan.with_file(
            background_worker_path,
            android_background_worker_source(package, background_tasks),
        );
    }
    // Plugin resources are staged relative to the asset directory so the
    // marker survives the application id changing.
    let resource_directory = plugins::ANDROID_RESOURCES;
    for resource in &resources.copies {
        plan = plan.with_copy(resource.copy.clone());
    }
    for file in &resources.generated {
        plan = plan.with_file(
            format!("{resource_directory}/{}", file.relative),
            file.contents.clone(),
        );
    }
    let staged_now: Vec<&str> = resources
        .copies
        .iter()
        .map(|resource| resource.relative.as_str())
        .collect();
    for stale in &resources.previous {
        if !staged_now.contains(&stale.as_str()) {
            plan = plan.with_removal(format!("{resource_directory}/{stale}"));
        }
    }
    if resources.present {
        plan = plan.with_file(
            format!("{resource_directory}/.nexa-plugin-resources"),
            format!("{}\n", staged_now.join("\n")),
        );
    } else {
        plan = plan.with_removal(format!("{resource_directory}/.nexa-plugin-resources"));
    }
    match templates::android_firebase_resources(plugins, config)? {
        Some(resources) => {
            plan = plan.with_file(
                "android/app/src/main/res/values/nexa_firebase.xml",
                resources,
            );
        }
        None => {
            plan = plan.with_removal("android/app/src/main/res/values/nexa_firebase.xml");
        }
    }
    if dev_runtime {
        for (filename, content) in ANDROID_DEV_RUNTIME_FILES {
            plan = plan.with_file(
                format!("{source_directory}/{filename}"),
                dev_runtime_source(filename, content, package, module),
            );
        }
        plan = plan.with_file(
            "android/app/src/debug/AndroidManifest.xml",
            "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><uses-permission android:name=\"android.permission.INTERNET\"/><application android:usesCleartextTraffic=\"true\"/></manifest>\n",
        );
    } else {
        for (filename, _) in ANDROID_DEV_RUNTIME_FILES {
            plan = plan.with_removal(format!("{source_directory}/{filename}"));
        }
        plan = plan.with_removal(format!("{source_directory}/NexaDevPluginBridge.kt"));
        plan = plan.with_removal("android/app/src/debug/AndroidManifest.xml");
    }
    plan.validate()?;
    Ok(plan)
}

fn android_background_worker_source(package: &str, tasks: &[nexa_ir::BackgroundTask]) -> String {
    let mut schedules = String::new();
    for (index, task) in tasks.iter().enumerate() {
        schedules.push_str(&format!(
            "        workManager.enqueueUniquePeriodicWork(\n            \"{}\",\n            ExistingPeriodicWorkPolicy.UPDATE,\n            PeriodicWorkRequestBuilder<NexaBackgroundWorker>({}, TimeUnit.MINUTES)\n                .setInputData(workDataOf(\"nexaTaskIndex\" to {index}))\n                .build(),\n        )\n",
            task.identifier, task.interval_minutes
        ));
    }
    format!(
        "package {package}\n\nimport android.content.Context\nimport androidx.work.CoroutineWorker\nimport androidx.work.ExistingPeriodicWorkPolicy\nimport androidx.work.PeriodicWorkRequestBuilder\nimport androidx.work.WorkManager\nimport androidx.work.WorkerParameters\nimport androidx.work.workDataOf\nimport kotlinx.coroutines.CancellationException\nimport java.util.concurrent.TimeUnit\n\nclass NexaBackgroundWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {{\n    override suspend fun doWork(): Result {{\n        NexaRuntime.bind(applicationContext)\n        return try {{\n            __nexaRunBackgroundTask(inputData.getInt(\"nexaTaskIndex\", -1))\n            Result.success()\n        }} catch (cancelled: CancellationException) {{\n            throw cancelled\n        }} catch (_: Exception) {{\n            Result.retry()\n        }}\n    }}\n\n    companion object {{\n        fun scheduleAll(context: Context) {{\n            val workManager = WorkManager.getInstance(context.applicationContext)\n{schedules}        }}\n    }}\n}}\n"
    )
}

fn copy_config_icons(root: &Path, app_name: &str, config: &ProjectConfig) -> Result<(), String> {
    let ios_source = config.ios_icon.as_ref().or(config.icon_source.as_ref());
    if let Some(source) = ios_source {
        if source.is_dir() {
            copy_directory_contents(
                source,
                &root
                    .join("ios")
                    .join(app_name)
                    .join("Assets.xcassets/AppIcon.appiconset"),
            )?;
        } else if source
            .extension()
            .is_some_and(|extension| extension == "icon")
        {
            let destination = root.join("ios").join(app_name).join("AppIcon.icon");
            assets::copy_icon_composer(source, &destination)?;
        } else {
            assets::generate_ios_icon(source, root, app_name)?;
        }
    }
    for source in &config.ios_alternate_icons {
        let name = source
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("invalid alternate iOS icon path: {}", source.display()))?;
        assets::copy_icon_composer(
            source,
            &root.join("ios").join(app_name).join(format!("{name}.icon")),
        )?;
    }
    let android_source = config.android_icon.as_ref().or(config.icon_source.as_ref());
    if let Some(source) = android_source {
        assets::generate_android_icon(source, root)?;
    }
    for source in &config.android_alternate_icons {
        let name = source
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("invalid alternate Android icon path: {}", source.display()))?
            .to_ascii_lowercase();
        assets::generate_android_icon_named(source, root, &name)?;
    }
    if let Some(splash) = &config.splash_source {
        generate_splash(splash, root, app_name)?;
    }
    Ok(())
}

fn generate_splash(source: &Path, root: &Path, app_name: &str) -> Result<(), String> {
    let image = image::open(source).map_err(|error| format!("{}: {error}", source.display()))?;
    let ios = root
        .join("ios")
        .join(app_name)
        .join("Assets.xcassets/NexaSplash.imageset");
    fs::create_dir_all(&ios).map_err(|error| error.to_string())?;
    image
        .save(ios.join("NexaSplash.png"))
        .map_err(|error| error.to_string())?;
    write_if_changed(
        &ios.join("Contents.json"),
        "{\"images\":[{\"filename\":\"NexaSplash.png\",\"idiom\":\"universal\"}],\"info\":{\"author\":\"xcode\",\"version\":1}}\n",
    )?;
    let android = root.join("android/app/src/main/res/drawable-nodpi");
    fs::create_dir_all(&android).map_err(|error| error.to_string())?;
    image
        .save(android.join("nexa_splash.png"))
        .map_err(|error| error.to_string())?;
    write_if_changed(
        &root.join("android/app/src/main/res/values/nexa_splash_theme.xml"),
        "<resources><style name=\"NexaSplashTheme\" parent=\"Theme.SplashScreen\"><item name=\"windowSplashScreenBackground\">#FFFFFFFF</item><item name=\"windowSplashScreenAnimatedIcon\">@drawable/nexa_splash</item><item name=\"postSplashScreenTheme\">@style/NexaAppTheme</item></style></resources>\n",
    )?;
    write_if_changed(
        &root.join("android/app/src/main/res/values-night/nexa_splash_theme.xml"),
        "<resources><style name=\"NexaSplashTheme\" parent=\"Theme.SplashScreen\"><item name=\"windowSplashScreenBackground\">#FF121212</item><item name=\"windowSplashScreenAnimatedIcon\">@drawable/nexa_splash</item><item name=\"postSplashScreenTheme\">@style/NexaAppTheme</item></style></resources>\n",
    )?;
    Ok(())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("{}: {error}", destination.display()))?;
    for entry in fs::read_dir(source).map_err(|error| format!("{}: {error}", source.display()))? {
        let entry = entry.map_err(|error| format!("{}: {error}", source.display()))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)
            .map_err(|error| format!("{}: {error}", source_path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "icon resources cannot contain symlinks: {}",
                source_path.display()
            ));
        }
        if metadata.is_dir() {
            copy_directory_contents(&source_path, &destination_path)?;
        } else if metadata.is_file()
            && (!destination_path.is_file()
                || fs::read(&source_path)
                    .map_err(|error| format!("{}: {error}", source_path.display()))?
                    != fs::read(&destination_path).unwrap_or_default())
        {
            fs::copy(&source_path, &destination_path)
                .map_err(|error| format!("{}: {error}", destination_path.display()))?;
        }
    }
    Ok(())
}

/// Renders the iOS compile units for a host project.
///
/// Plugin bindings add an import that every generated file must see, and a
/// configuration enum that belongs at the end of the output. Both are applied
/// to the unit list directly, rather than by rewriting a concatenated string
/// and splitting it again.
fn ios_source_units(
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
    dev_runtime: bool,
) -> Result<
    (
        Vec<nexa_codegen::SourceUnit>,
        nexa_backend_swift::SwiftProjectFeatures,
    ),
    String,
> {
    let plugin_value_runtime = !dev_runtime && plugin_packages_require_value_runtime(plugins)?;
    let (mut sources, project_features) = if dev_runtime {
        SwiftBackend.generate_for_dev_units_with_project_features(module)
    } else if plugin_value_runtime {
        SwiftBackend.generate_units_with_plugin_value_runtime_and_project_features(module)
    } else {
        SwiftBackend.generate_units_with_project_features(module)
    };
    if dev_runtime {
        let contracts = dev_plugin_contracts(plugins)?;
        sources.units.push(nexa_codegen::SourceUnit {
            name: "NexaDevPluginBridge.swift".to_owned(),
            contents: nexa_codegen::plugin::render_dev_bridge_swift(&contracts)?,
        });
    }
    if module.plugins.is_empty() && (!dev_runtime || plugins.is_empty()) {
        return Ok((sources.into_files(&[], ""), project_features));
    }
    let plugin_config = plugins::render_swift_plugin_config(plugins, config);
    // Plugin bindings add an import every generated file must see.
    Ok((
        sources.into_files(&["import Foundation"], &plugin_config),
        project_features,
    ))
}

/// Produces the complete source set owned by the WidgetKit extension. Widget
/// declarations and their reachable helpers come from the Swift backend; a
/// plugin contributes extension code only when its manifest explicitly opts
/// into the extension target.
fn generated_ios_widget_sources(
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
) -> Result<Vec<NativeWidgetSource>, String> {
    if module.widgets.is_empty() {
        return Ok(Vec::new());
    }
    let generated = SwiftBackend
        .generate_widget_units(module)
        .map_err(|error| error.to_string())?;
    let mut extension_plugins = Vec::new();
    let mut selected_extension_sources = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        let extension_sources = plugins::native_plugin_extension_sources(plugin)?;
        if !extension_sources.is_empty() {
            extension_plugins.push(plugin.clone());
            selected_extension_sources.push((plugin_index, plugin, extension_sources));
        }
    }
    let plugin_config = plugins::render_swift_plugin_config(&extension_plugins, config);
    let files = generated
        .sources
        .into_files(&["import Foundation"], &plugin_config)
        .into_iter()
        .map(|unit| NativeWidgetSource {
            relative_path: unit.name,
            contents: unit.contents,
        })
        .collect::<Vec<_>>();
    let mut output = files;
    for (plugin_index, plugin, extension_sources) in selected_extension_sources {
        let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
        output.push(NativeWidgetSource {
            relative_path: format!("NexaPluginExtension{plugin_index}_Bindings.swift"),
            contents: crate::plugin::render_swift_bindings(&contract)?,
        });
        for (source_index, (source, _)) in extension_sources.into_iter().enumerate() {
            let stem = source
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("PluginSource")
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || character == '_' {
                        character
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            output.push(NativeWidgetSource {
                relative_path: format!(
                    "NexaPluginExtension{plugin_index}_{source_index}_{stem}.swift"
                ),
                contents: fs::read_to_string(&source)
                    .map_err(|error| format!("{}: {error}", source.display()))?,
            });
        }
    }
    output.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(output)
}

/// Reads a previous generated-file marker only for deleting outputs from the
/// retired hand-authored widget scaffold. Invalid or escaping entries are
/// ignored so stale marker contents cannot name paths outside generated roots.
fn read_generated_marker(marker: &Path) -> Result<Vec<String>, String> {
    if !marker.is_file() {
        return Ok(Vec::new());
    }
    let contents =
        fs::read_to_string(marker).map_err(|error| format!("{}: {error}", marker.display()))?;
    Ok(contents
        .lines()
        .filter(|line| {
            !line.is_empty()
                && Path::new(line)
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)))
        })
        .map(str::to_owned)
        .collect())
}

fn validate_android_widget_resources(
    resources: &[nexa_codegen::SourceUnit],
    module: &Module,
) -> Result<(), String> {
    let mut paths = std::collections::HashSet::new();
    paths.insert("values/nexa_widget_strings.xml".to_owned());
    paths.insert("xml/nexa_widget_info.xml".to_owned());
    for index in 0..module.widgets.len() {
        paths.insert(format!(
            "xml/{}.xml",
            templates::android_widget_resource_name(index)
        ));
    }
    for resource in resources {
        let path = Path::new(&resource.name);
        if path.as_os_str().is_empty()
            || path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(format!(
                "Android widget resource path `{}` must be relative to `src/main/res`",
                resource.name
            ));
        }
        if !paths.insert(resource.name.clone()) {
            return Err(format!(
                "Android widget resource `{}` conflicts with another generated widget resource",
                resource.name
            ));
        }
    }
    Ok(())
}

fn dev_plugin_contracts(
    plugins: &[plugin_package::PluginPackage],
) -> Result<Vec<(String, nexa_plugin_idl::PluginIdl)>, String> {
    plugins
        .iter()
        .map(|plugin| {
            let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
            Ok((plugin.namespace.clone(), contract))
        })
        .collect()
}

fn plugin_packages_require_value_runtime(
    plugins: &[plugin_package::PluginPackage],
) -> Result<bool, String> {
    for plugin in plugins {
        let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
        if contract.interfaces.iter().any(|interface| {
            interface
                .methods
                .iter()
                .any(|method| !method.type_parameters.is_empty())
        }) {
            return Ok(true);
        }
    }
    Ok(false)
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

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn swift_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn kotlin_escape(value: &str) -> String {
    swift_escape(value)
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
    config: &ProjectConfig,
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
        if manifest.contains("\"iosPrivacyManifest\": true")
            && !output
                .join("ios")
                .join(app_name)
                .join("PrivacyInfo.xcprivacy")
                .is_file()
        {
            return false;
        }
    }
    if needs_android {
        let package = &config.android_application_id;
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
            output.join("android/gradlew"),
            output.join("android/gradlew.bat"),
            output.join("android/gradle/wrapper/gradle-wrapper.jar"),
            output.join("android/gradle/wrapper/gradle-wrapper.properties"),
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nexa_codegen::Backend;
    use nexa_compiler::{Target, compile_file_with_warnings_for_targets};

    use super::{
        KotlinBackend, ProjectPlan, SHARED_ICONS, SwiftBackend, dev_runtime_source,
        generate_splash, plugin_package, plugins, writers,
    };

    #[test]
    fn dev_runtime_ui_defaults_are_injected_from_the_shared_design_system() {
        let module =
            nexa_compiler::compile(r##"app DesignSystemProbe { body { Text("Parity") } }"##)
                .expect("compile the runtime design-system fixture");
        let swift = dev_runtime_source(
            "NexaDevRenderer.swift",
            include_str!("../../../runtime/ios/NexaDevRenderer.swift"),
            "",
            &module,
        );
        let kotlin = dev_runtime_source(
            "NexaDevRenderer.kt",
            include_str!("../../../runtime/android/NexaDevRenderer.kt"),
            "dev.nexa",
            &module,
        );

        for (platform, source) in [("iOS", swift.as_str()), ("Android", kotlin.as_str())] {
            assert!(
                !source.contains("__NEXA_"),
                "{platform} DevRuntime retained an unresolved design-system token"
            );
        }
        assert!(swift.contains("Text(label).font(.system(size: 17))"));
        assert!(swift.contains("minWidth: CGFloat(64)"));
        assert!(
            swift.contains(
                "minHeight: CGFloat((fields[\"size\"] as? String) == \"Large\" ? 50 : 48)"
            )
        );
        assert!(swift.contains("HStack(spacing: 8)"));
        assert!(swift.contains(".padding(.bottom, 24)"));
        assert!(swift.contains("button.tint(Color(uiColor: .systemBlue))"));
        assert!(swift.contains("index == selected ? Color(uiColor: .systemBlue)"));
        assert!(swift.contains("Color(red: 0.556863, green: 0.556863, blue: 0.576471)"));

        assert!(kotlin.contains("private const val nexaDevDefaultBodyFontSize = 17"));
        assert!(kotlin.contains("private const val nexaDevDefaultLineHeightMultiplier = 1.2f"));
        assert!(kotlin.contains("private const val nexaDevButtonMinWidth = 64"));
        assert!(kotlin.contains("private const val nexaDevButtonMinTapTarget = 48"));
        assert!(kotlin.contains("private const val nexaDevButtonLargeMinHeight = 50"));
        assert!(kotlin.contains("private const val nexaDevButtonLargeHorizontalPadding = 20"));
        assert!(kotlin.contains("primary = Color(0xFF007AFF)"));
        assert!(kotlin.contains(
            "lineHeight = nexaDevDefaultBodyFontSize.sp * nexaDevDefaultLineHeightMultiplier"
        ));
        assert!(kotlin.contains("contentAlignment = Alignment.Center"));
        assert!(kotlin.contains("propagateMinConstraints = true"));
        assert!(kotlin.contains("Modifier.fillMaxWidth()"));
        assert!(kotlin.contains(
            "nexaDevPageIndicatorSpacing.dp,\n                        Alignment.CenterHorizontally,"
        ));
        assert!(kotlin.contains("verticalAlignment = Alignment.CenterVertically"));
        assert!(kotlin.contains("Modifier.fillMaxWidth()\n                        .padding(bottom = nexaDevPageIndicatorBottomInset.dp)"));
    }

    #[test]
    fn generated_dev_icon_mappings_cover_the_shared_catalog() {
        let module = nexa_compiler::compile(
            r##"app IconMap { body { Icon(materialsymbol: "rounded:account_circle", description: "Profile", size: 24, tint: "#FFFFFF") } }"##,
        )
        .expect("compile icon mapping fixture");
        let swift = dev_runtime_source(
            "NexaDevRenderer.swift",
            include_str!("../../../runtime/ios/NexaDevRenderer.swift"),
            "",
            &module,
        );
        let kotlin = dev_runtime_source(
            "NexaDevRenderer.kt",
            include_str!("../../../runtime/android/NexaDevRenderer.kt"),
            "dev.nexa",
            &module,
        );
        for icon in SHARED_ICONS {
            assert!(
                swift.contains(&format!(
                    "case \"{}\": return \"{}\"",
                    icon.name, icon.sf_symbol
                )),
                "iOS DevRuntime is missing shared icon {}",
                icon.name
            );
            assert!(
                kotlin.contains(&format!(
                    "\"{}\" -> Icons.{}.{}",
                    icon.name, icon.material_namespace, icon.material_name
                )),
                "Android DevRuntime is missing shared icon {}",
                icon.name
            );
            for alias in icon.aliases {
                assert!(
                    kotlin.contains(&format!(
                        "\"{alias}\" -> Icons.{}.{}",
                        icon.material_namespace, icon.material_name
                    )),
                    "Android DevRuntime is missing SF alias {alias} for {}",
                    icon.name
                );
            }
        }
        assert!(kotlin.contains("import androidx.compose.material.icons.rounded.AccountCircle"));
        assert!(kotlin.contains("\"rounded:account_circle\" -> Icons.Rounded.AccountCircle"));
    }

    /// Builds a plugin package that vendors one XCFramework.
    fn package_with_xcframework(root: &Path, name: &str) -> plugin_package::PluginPackage {
        let xcframework = root.join("ios").join(format!("{name}.xcframework"));
        std::fs::create_dir_all(&xcframework).expect("xcframework directory");
        std::fs::write(xcframework.join("Info.plist"), "metadata").expect("xcframework metadata");
        plugin_package::PluginPackage {
            namespace: "Vendor".to_owned(),
            idl_path: root.join("native.nxid").display().to_string(),
            artifacts: plugin_package::PluginArtifacts {
                ios_xcframeworks: vec![xcframework.display().to_string()],
                ..Default::default()
            },
        }
    }

    /// Claims a temporary project root that stays alive for the whole test.
    fn temp_root(tag: &str) -> nexa_testkit::TempDir {
        nexa_testkit::TempDir::new(&format!("nexa-artifact-staging-{tag}"))
    }

    #[test]
    fn android_splash_resources_follow_day_and_night_appearance() {
        let root = temp_root("appearance-splash");
        let source = root.path().join("splash.png");
        let output = root.path().join("generated");
        std::fs::create_dir_all(&output).expect("create generated project root");
        image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
            .save(&source)
            .expect("write splash image");

        generate_splash(&source, &output, "Demo").expect("generate platform splash assets");

        let day = std::fs::read_to_string(
            output.join("android/app/src/main/res/values/nexa_splash_theme.xml"),
        )
        .expect("read day splash style");
        let night = std::fs::read_to_string(
            output.join("android/app/src/main/res/values-night/nexa_splash_theme.xml"),
        )
        .expect("read night splash style");
        assert!(day.contains("#FFFFFFFF"));
        assert!(night.contains("#FF121212"));
        assert!(day.contains("@style/NexaAppTheme"));
        assert!(night.contains("@style/NexaAppTheme"));
    }

    #[test]
    fn xcframework_staging_is_planned_and_replaced_across_builds() {
        let root = temp_root("xcframework");
        let package_root = root.join("pkg");
        std::fs::create_dir_all(&package_root).expect("package root");
        let plugins = [package_with_xcframework(&package_root, "Vendor")];

        // First build: the framework is copied and the marker records it.
        let (staged, previous) =
            plugins::stage_ios_plugin_artifacts(&root, "Demo", &plugins).expect("staging");
        assert!(previous.is_empty(), "nothing staged before the first build");
        assert_eq!(staged.len(), 1);
        assert_eq!(
            staged[0].name,
            "Frameworks/NexaPlugin0_0_vendor.xcframework"
        );
        assert_eq!(
            staged[0].copy.destination,
            "ios/Demo/Frameworks/NexaPlugin0_0_vendor.xcframework"
        );
        let plan = ProjectPlan::ios("Demo")
            .with_copy(staged[0].copy.clone())
            .with_file(
                "ios/Demo/.nexa-plugin-frameworks",
                format!("{}\n", staged[0].name),
            );
        writers::write_plan(&root, &plan).expect("first build writes");
        assert!(
            root.join("ios/Demo/Frameworks/NexaPlugin0_0_vendor.xcframework/Info.plist")
                .is_file()
        );

        // Dropping the plugin must plan a removal of what the marker recorded.
        let (_, previous) =
            plugins::stage_ios_plugin_artifacts(&root, "Demo", &[]).expect("staging");
        assert_eq!(
            previous,
            vec!["Frameworks/NexaPlugin0_0_vendor.xcframework"]
        );
        let plan = ProjectPlan::ios("Demo")
            .with_removal("ios/Demo/Frameworks/NexaPlugin0_0_vendor.xcframework")
            .with_removal("ios/Demo/.nexa-plugin-frameworks");
        writers::write_plan(&root, &plan).expect("second build writes");
        assert!(
            !root
                .join("ios/Demo/Frameworks/NexaPlugin0_0_vendor.xcframework")
                .exists(),
            "a framework the app no longer uses must not be left behind"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn video_player_demo_generates_two_direct_native_instances_on_both_targets() {
        let entry = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/video-player/tests/demo/app/App.nx");
        let compilations =
            compile_file_with_warnings_for_targets(&entry, &[Target::Swift, Target::Kotlin])
                .expect("the plugin example should compile for both native targets");

        assert_eq!(compilations.len(), 2);
        assert_eq!(compilations[0].module.plugins.len(), 1);
        assert_eq!(compilations[1].module.plugins.len(), 1);

        let swift = SwiftBackend.generate(&compilations[0].module);
        assert_eq!(
            swift
                .matches("= NexaNativeObjectStorage { VideoPlayer() }")
                .count(),
            2
        );
        assert!(swift.contains(
            "private var nexa_player1: VideoPlayer {\n        get { __nexaNativeObjectStorage_nexa_player1.value }"
        ));
        assert!(swift.contains(
            "private var nexa_player2: VideoPlayer {\n        get { __nexaNativeObjectStorage_nexa_player2.value }"
        ));
        for fragment in [
            "NexaPlayerSurfaceComponent(nexa_player1, \"First player\")",
            "NexaPlayerMediaComponent(nexa_player, nexa_title)",
            "VideoView(player: nexa_player,",
            "VideoView(player: nexa_player2",
            "nexa_tapped = true",
            "nexa_secondTapped = true",
            "await nexa_player1.prepare(\"",
            "await nexa_player2.prepare(\"",
            "nexa_player1.volume = Double(0.5)",
            "nexa_player2.volume = Double(0.25)",
            "nexa_player1.dispose()",
            "nexa_player2.dispose()",
            "nexa_player1.onEnded = {",
            "nexa_player2.onEnded = {",
            "nexa_firstEnded = true",
            "nexa_secondEnded = true",
            "First player ended",
            "Second player ended",
            "Second view tapped",
        ] {
            assert!(swift.contains(fragment), "missing Swift output: {fragment}");
        }
        assert!(!swift.contains("onTapped"));
        assert!(swift.contains("do {"));
        assert!(swift.contains("try await nexa_player1.prepare(\""));
        assert!(swift.contains("} catch let error as PlayerError {"));
        assert!(swift.contains("case .invalidUrl:"));
        assert!(swift.contains("case let .decodingFailed(nexa_message):"));
        assert!(swift.contains("nexa_loadError = nexa_message"));
        assert!(!swift.contains("try?"));

        let kotlin = KotlinBackend.generate(&compilations[1].module);
        assert_eq!(kotlin.matches("remember { VideoPlayer() }").count(), 2);
        assert!(kotlin.contains("val nexa_player1: VideoPlayer = remember { VideoPlayer() }"));
        assert!(kotlin.contains("val nexa_player2: VideoPlayer = remember { VideoPlayer() }"));
        for fragment in [
            "NexaPlayerSurfaceComponent(nexa_player1, \"First player\")",
            "NexaPlayerMediaComponent(nexa_player, nexa_title)",
            "VideoView(player = nexa_player,",
            "VideoView(player = nexa_player2",
            "nexa_tapped = true",
            "nexa_secondTapped = true",
            "nexa_player1.prepare(",
            "nexa_player2.prepare(",
            "nexa_player1.volume = 0.5",
            "nexa_player2.volume = 0.25",
            "nexa_player1.dispose()",
            "nexa_player2.dispose()",
            "nexa_player1.onEnded = {",
            "nexa_player2.onEnded = {",
            "nexa_firstEnded = true",
            "nexa_secondEnded = true",
        ] {
            assert!(
                kotlin.contains(fragment),
                "missing Kotlin output: {fragment}"
            );
        }
        for source_text in [
            "First player ended",
            "Second player ended",
            "Second view tapped",
        ] {
            let resource = nexa_codegen::names::localization_resource_name(source_text);
            assert!(
                kotlin.contains(&format!("stringResource(R.string.{resource})")),
                "missing localized Kotlin output for {source_text}"
            );
        }
        assert!(!kotlin.contains("onTapped"));
        assert!(kotlin.contains("when (error) {"));
        assert!(kotlin.contains("PlayerError.invalidUrl ->"));
        assert!(kotlin.contains("is PlayerError.decodingFailed ->"));
        assert!(kotlin.contains("nexa_loadError = nexa_message"));
        assert!(kotlin.contains("try {"));
        assert!(kotlin.contains("nexa_player1.prepare("));
        assert!(kotlin.contains("} catch (error: Exception) {"));
    }

    #[test]
    fn app_owned_native_resources_are_shared_across_route_lifetimes() {
        let entry = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/video-player/tests/demo/RouteSharing.nx");
        let compilations =
            compile_file_with_warnings_for_targets(&entry, &[Target::Swift, Target::Kotlin])
                .expect("route-sharing plugin test app should compile for both native targets");

        let swift = SwiftBackend.generate(&compilations[0].module);
        assert_eq!(
            swift
                .matches("= NexaNativeObjectStorage { VideoPlayer() }")
                .count(),
            1
        );
        assert!(swift.contains("NexaNavigationRoute.screen1(UUID())"));
        assert!(swift.contains("nexa_sharedPlayer.play()"));
        assert!(swift.contains("nexa_sharedPlayer.pause()"));
        assert!(swift.contains("nexa_sharedPlayer.dispose()"));

        let kotlin = KotlinBackend.generate(&compilations[1].module);
        assert_eq!(kotlin.matches("remember { VideoPlayer() }").count(), 1);
        assert!(kotlin.contains("nexa_sharedPlayer.play()"));
        assert!(kotlin.contains("nexa_sharedPlayer.pause()"));
        assert!(kotlin.contains("nexa_sharedPlayer.dispose()"));
        assert!(kotlin.find("val nexa_sharedPlayer").unwrap() < kotlin.find("NavHost(").unwrap());
    }
}
