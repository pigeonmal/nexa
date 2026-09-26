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
use nexa_ir::Module;

use crate::{cache, config, config::ProjectConfig};

mod assets;
pub mod plan;
use self::plan::ProjectPlan;
mod plugin_package;
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
    let mut input = None;
    let mut output = None;
    let mut name = None;
    let mut target = ProjectTarget::All;
    let mut deny_warnings = false;
    let mut locked = false;
    let mut flavor = None;
    let mut dev_server_url = None;
    let mut dev_session_token = None;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--deny-warnings" => deny_warnings = true,
            "--locked" => locked = true,
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
        &[config_path.as_path(), app_images_path.as_path()],
        &sorted_plugin_roots,
    )
    .map_err(|error| format!("project cache: {error}"))?;
    if dev_session.is_none()
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
            println!("generated {} (cache hit)", output.display());
            return Ok(());
        }
    }

    let targets: &[Target] = match target {
        ProjectTarget::Ios => &[Target::Swift],
        ProjectTarget::Android => &[Target::Kotlin],
        ProjectTarget::All => &[Target::Swift, Target::Kotlin],
    };
    let compilations = nexa_compiler::compile_file_with_warnings_for_targets_and_plugin_roots(
        &input,
        targets,
        plugin_roots,
    )
    .map_err(|error| error.to_string())?;
    let mut warnings = Vec::new();
    let compiled = targets
        .iter()
        .copied()
        .zip(compilations)
        .map(|(target, compilation)| {
            warnings.extend(compilation.warnings);
            let packages =
                plugin_package::packages_for_module(&compilation.plugins, &compilation.module);
            (target, compilation.module, packages)
        })
        .collect::<Vec<_>>();
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
            &[config_path.as_path(), app_images_path.as_path()],
            &sorted_plugin_roots,
        )
        .map_err(|error| format!("project cache: {error}"))?;
    }
    if let Some(flavor) = &flavor {
        project_config = project_config.with_flavor(flavor)?;
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
    let dependencies = config::load_plugin_dependencies(&config_path)?;
    let resolved = crate::dependencies::resolve(project_root, &dependencies)?;
    let compilations = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            entry,
            targets,
            &resolved.plugin_roots,
        )
        .map_err(|error| error.to_string())?;
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
                &[config_path.as_path(), app_images_path.as_path()],
                &sorted_roots,
            )
            .map_err(|error| format!("dev revision: {error}"))?;
            Ok((platform, nexa_dev_ir::lower(&compilation.module, revision)))
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
            .is_some_and(|extension| matches!(extension, "swift" | "kt" | "xml" | "plist" | "json"))
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

/// Every generated file name in an iOS source directory, including the dev
/// runtime when this build uses it. Used to clear units a previous run left.
fn generated_unit_names(plan: &ProjectPlan, dev_runtime: bool) -> Vec<String> {
    let mut names: Vec<String> = plan
        .source_units()
        .iter()
        .map(|unit| unit.name.clone())
        .collect();
    if dev_runtime {
        names.push("NexaDevRuntime.swift".to_owned());
    }
    names
}

/// Facts gathered by the filesystem work, which the Xcode project references.
struct PreparedIos {
    source_units: Vec<nexa_codegen::SourceUnit>,
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
    let source_units = ios_source_units(module, plugins, config, dev_runtime)?;
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
    Ok(PreparedIos {
        source_units,
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
            format!(
                "import SwiftUI\n\n@main\nstruct {app_name}App: App {{\n    var body: some Scene {{\n        WindowGroup {{\n            {app_root}\n        }}\n    }}\n}}\n"
            ),
        )
        .with_file(
            format!("ios/{app_name}.xcodeproj/xcshareddata/xcschemes/{app_name}.xcscheme"),
            templates::ios_scheme(app_name),
        )
        .with_file(
            format!("ios/{app_name}.xcodeproj/project.pbxproj"),
            templates::ios_project_file_with_config(
                app_name,
                prepared.has_assets,
                prepared.resources.present,
                &generated_names,
                &prepared.plugin_sources,
                &prepared.cpp_sources,
                &prepared.xcframeworks,
                plugins,
                config,
            )?,
        )
        .with_file(
            format!("{directory}/Info.plist"),
            templates::ios_info_plist_with_dev_runtime(app_name, config, plugins, dev_runtime)?,
        );
    if config.splash_source.is_some() {
        plan = plan.with_file(
            format!("{directory}/LaunchScreen.storyboard"),
            templates::ios_launch_storyboard(),
        );
    }
    if dev_runtime {
        plan = plan.with_file(
            format!("{directory}/NexaDevRuntime.swift"),
            include_str!("../../../runtime/ios/NexaDevRuntime.swift"),
        );
    } else {
        plan = plan.with_removal(format!("{directory}/NexaDevRuntime.swift"));
    }
    match templates::ios_entitlements(config, plugins)? {
        Some(entitlements) => {
            plan = plan.with_file(format!("{directory}/Nexa.entitlements"), entitlements);
        }
        None => {
            plan = plan.with_removal(format!("{directory}/Nexa.entitlements"));
        }
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
    let package = config.android_application_id.clone();
    let package_path = package.replace('.', "/");
    let source_dir = root.join("android/app/src/main/java").join(&package_path);
    fs::create_dir_all(&source_dir)
        .map_err(|error| format!("{}: {error}", source_dir.display()))?;
    let (sources, mut project_features) = if dev_session.is_some() {
        KotlinBackend.generate_for_dev_units_with_project_features(module)
    } else {
        KotlinBackend.generate_units_with_project_features(module)
    };
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

    let plan = android_plan(
        app_name,
        nexa_codegen::names::screen_name(&module.app_name),
        &package,
        &package_path,
        &generated_units,
        config,
        plugins,
        dev_session,
        project_features,
        &local_aars,
        &resources,
    )?;
    writers::write_plan(root, &plan)?;
    let mut keep = generated_names;
    if dev_session.is_some() {
        keep.push("NexaDevRuntime.kt".to_owned());
    }
    writers::remove_stale_units(&source_dir, &keep, plan.platform().unit_extension())
}

/// Computes every Android file and path, without touching the filesystem.
#[allow(clippy::too_many_arguments)]
fn android_plan(
    app_name: &str,
    screen: String,
    package: &str,
    package_path: &str,
    source_units: &[nexa_codegen::SourceUnit],
    config: &ProjectConfig,
    plugins: &[plugin_package::PluginPackage],
    dev_session: Option<&DevSessionConfig>,
    project_features: nexa_backend_kotlin::KotlinProjectFeatures,
    local_aars: &[String],
    resources: &plugins::StagedResources,
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
    let cronet_import = if project_features.uses_network {
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
    let activity_content = if project_features.uses_network {
        format!(
            "        CronetProviderInstaller.installProvider(this).addOnCompleteListener {{ result ->\n            if (!result.isSuccessful) android.util.Log.w(\"Nexa\", \"Play Services Cronet unavailable; using bundled Cronet when available\", result.exception)\n            setContent {{ MaterialTheme {{ {compose_root} }} }}\n        }}\n"
        )
    } else {
        format!("        setContent {{ MaterialTheme {{ {compose_root} }} }}\n")
    };
    let mut plan = ProjectPlan::android(app_name)
        .with_source_directory(source_directory.clone())
        .with_source_units(source_units.to_vec())
        .with_file(
            format!("{source_directory}/MainActivity.kt"),
            format!(
                "package {package}\n\nimport android.os.Bundle\nimport androidx.activity.ComponentActivity\nimport androidx.activity.compose.setContent\nimport androidx.compose.material3.MaterialTheme\n{splash_import}{cronet_import}\nclass MainActivity : ComponentActivity() {{\n    override fun onCreate(savedInstanceState: Bundle?) {{\n{splash_install}        super.onCreate(savedInstanceState)\n{activity_content}    }}\n}}\n"
            ),
        )
        .with_file(
            "android/app/src/main/AndroidManifest.xml",
            templates::android_manifest(app_name, package, project_features.uses_network, config, plugins),
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
            templates::android_app_gradle_with_dev_runtime(
                package,
                project_features,
                plugins,
                local_aars,
                config,
                dev_runtime,
            )?,
        )
        .with_file(
            "android/app/proguard-rules.pro",
            plugins::android_plugin_proguard_rules(plugins, package)?,
        );
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
    if dev_runtime {
        plan = plan.with_file(
            format!("{source_directory}/NexaDevRuntime.kt"),
            include_str!("../../../runtime/android/NexaDevRuntime.kt")
                .replace("__NEXA_PACKAGE__", package),
        );
        plan = plan.with_file(
            "android/app/src/debug/AndroidManifest.xml",
            "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><uses-permission android:name=\"android.permission.INTERNET\"/><application android:usesCleartextTraffic=\"true\"/></manifest>\n",
        );
    } else {
        plan = plan
            .with_removal(format!("{source_directory}/NexaDevRuntime.kt"))
            .with_removal("android/app/src/debug/AndroidManifest.xml");
    }
    plan.validate()?;
    Ok(plan)
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
    let android_source = config.android_icon.as_ref().or(config.icon_source.as_ref());
    if let Some(source) = android_source {
        if source.is_dir() {
            copy_directory_contents(source, &root.join("android/app/src/main/res"))?;
        } else {
            assets::generate_android_icon(source, root)?;
        }
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
        "<resources><style name=\"NexaSplashTheme\" parent=\"Theme.SplashScreen\"><item name=\"windowSplashScreenBackground\">#FFFFFFFF</item><item name=\"windowSplashScreenAnimatedIcon\">@drawable/nexa_splash</item><item name=\"postSplashScreenTheme\">@android:style/Theme.Material.Light.NoActionBar</item></style></resources>\n",
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
        } else if metadata.is_file() {
            if !destination_path.is_file()
                || fs::read(&source_path)
                    .map_err(|error| format!("{}: {error}", source_path.display()))?
                    != fs::read(&destination_path).unwrap_or_default()
            {
                fs::copy(&source_path, &destination_path)
                    .map_err(|error| format!("{}: {error}", destination_path.display()))?;
            }
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
) -> Result<Vec<nexa_codegen::SourceUnit>, String> {
    let sources = if dev_runtime {
        SwiftBackend.generate_for_dev_units(module)
    } else {
        SwiftBackend.generate_units(module)
    };
    if module.plugins.is_empty() {
        return Ok(sources.into_files(&[], ""));
    }
    let plugin_config = plugins::render_swift_plugin_config(plugins, config);
    // Plugin bindings add an import every generated file must see.
    Ok(sources.into_files(&["import Foundation"], &plugin_config))
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

    use super::{KotlinBackend, ProjectPlan, SwiftBackend, plugin_package, plugins, writers};

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
            .join("../../examples/plugins/video-player-demo.nx");
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
            "onTapped: {",
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
            "onTapped = {",
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
            "First player ended",
            "Second player ended",
            "Second view tapped",
        ] {
            assert!(
                kotlin.contains(fragment),
                "missing Kotlin output: {fragment}"
            );
        }
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
            .join("../../examples/plugins/video-player-route-sharing.nx");
        let compilations =
            compile_file_with_warnings_for_targets(&entry, &[Target::Swift, Target::Kotlin])
                .expect("route-sharing example should compile for both native targets");

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
