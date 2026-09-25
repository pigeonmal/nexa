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
use nexa_compiler::{CompileWarning, Target};
use nexa_ir::Module;

use crate::{cache, config, config::ProjectConfig};

mod assets;
mod plugin_package;
mod plugins;
mod templates;

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
    source_root: &Path,
    app_name: &str,
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
    dev_session: Option<&DevSessionConfig>,
) -> Result<(), String> {
    let directory = root.join("ios").join(app_name);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let screen = nexa_codegen::names::screen_name(&module.app_name);
    let source = ios_generated_source(module, plugins, config, dev_session.is_some())?;
    let source_units = split_generated_units(&source, "swift");
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
    let xcframeworks = plugins::copy_ios_plugin_artifacts(root, app_name, plugins)?;
    let has_plugin_resources = plugins::copy_ios_plugin_resources(root, app_name, plugins)?;
    let mut generated_names = source_units
        .iter()
        .map(|(name, contents)| {
            write_if_changed(&directory.join(name), contents)?;
            Ok(name.clone())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if dev_session.is_some() {
        write_if_changed(
            &directory.join("NexaDevRuntime.swift"),
            include_str!("../../../runtime/ios/NexaDevRuntime.swift"),
        )?;
        generated_names.push("NexaDevRuntime.swift".to_owned());
    } else {
        let runtime = directory.join("NexaDevRuntime.swift");
        if runtime.is_file() {
            fs::remove_file(&runtime).map_err(|error| format!("{}: {error}", runtime.display()))?;
        }
    }
    remove_stale_generated_units(&directory, &generated_names, "swift")?;
    let app_root = if let Some(dev_session) = dev_session {
        format!(
            "NexaDevRuntimeRoot(serverURL: \"{}\", sessionToken: \"{}\")",
            swift_escape(&dev_session.server_url),
            swift_escape(&dev_session.session_token)
        )
    } else {
        format!("{screen}()")
    };
    write_if_changed(
        &directory.join(format!("{app_name}App.swift")),
        &format!(
            "import SwiftUI\n\n@main\nstruct {app_name}App: App {{\n    var body: some Scene {{\n        WindowGroup {{\n            {app_root}\n        }}\n    }}\n}}\n"
        ),
    )?;
    if config.splash_source.is_some() {
        write_if_changed(
            &directory.join("LaunchScreen.storyboard"),
            &templates::ios_launch_storyboard(),
        )?;
    }
    write_if_changed(
        &directory.join("Info.plist"),
        &templates::ios_info_plist_with_dev_runtime(
            app_name,
            config,
            plugins,
            dev_session.is_some(),
        )?,
    )?;
    let entitlements_path = directory.join("Nexa.entitlements");
    if let Some(entitlements) = templates::ios_entitlements(config, plugins)? {
        write_if_changed(&entitlements_path, &entitlements)?;
    } else if entitlements_path.is_file() {
        fs::remove_file(&entitlements_path)
            .map_err(|error| format!("{}: {error}", entitlements_path.display()))?;
    }
    write_if_changed(
        &root
            .join("ios")
            .join(format!("{app_name}.xcodeproj/project.pbxproj")),
        &templates::ios_project_file_with_config(
            app_name,
            has_assets,
            has_plugin_resources,
            &generated_names,
            &plugin_sources,
            &cpp_sources,
            &xcframeworks,
            plugins,
            config,
        )?,
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
    let (generated, mut project_features) = if dev_session.is_some() {
        KotlinBackend.generate_for_dev_with_project_features(module)
    } else {
        KotlinBackend.generate_with_project_features(module)
    };
    // The debug runtime contains a generic Navigation Compose host even when
    // the app's own tree does not currently declare navigation.
    project_features.uses_navigation |= dev_session.is_some();
    let (plugin_packages, plugin_uses_coroutines) =
        plugins::copy_android_plugin_sources(root, plugins, &package, config)?;
    project_features.uses_coroutines |= plugin_uses_coroutines;
    plugins::copy_android_plugin_cpp_sources(root, plugins, &package)?;
    let local_aars = plugins::copy_android_plugin_artifacts(root, plugins)?;
    plugins::copy_android_plugin_resources(root, plugins)?;
    copy_config_icons(root, app_name, config)?;
    plugins::copy_plugin_assets(root, app_name, module)?;
    assets::copy_android_project_images(source_root, root)?;
    let screen = nexa_codegen::names::screen_name(&module.app_name);
    let cronet_import = if project_features.uses_network {
        "import com.google.android.gms.net.CronetProviderInstaller\n"
    } else {
        ""
    };
    let permission_callback = "";
    let splash_install = if config.splash_source.is_some() {
        "        installSplashScreen()\n"
    } else {
        ""
    };
    let splash_import = if config.splash_source.is_some() {
        "import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen\n"
    } else {
        ""
    };
    let plugin_imports = plugin_packages
        .iter()
        .map(|plugin_package| format!("import {plugin_package}.*"))
        .collect::<Vec<_>>();
    let imports = if plugin_imports.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", plugin_imports.join("\n"))
    };
    let generated_source = format!(
        "package {package}\n\n{imports}{generated}{}",
        plugins::render_kotlin_plugin_config(plugins, config)
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
    if dev_session.is_some() {
        let runtime = include_str!("../../../runtime/android/NexaDevRuntime.kt")
            .replace("__NEXA_PACKAGE__", &package);
        write_if_changed(&source_dir.join("NexaDevRuntime.kt"), &runtime)?;
    } else {
        let runtime = source_dir.join("NexaDevRuntime.kt");
        if runtime.is_file() {
            fs::remove_file(&runtime).map_err(|error| format!("{}: {error}", runtime.display()))?;
        }
    }
    let compose_root = if let Some(dev_session) = dev_session {
        format!(
            "NexaDevRuntimeRoot(serverURL = \"{}\", sessionToken = \"{}\")",
            kotlin_escape(&dev_session.server_url),
            kotlin_escape(&dev_session.session_token)
        )
    } else {
        format!("{screen}()")
    };
    let activity_content = if project_features.uses_network {
        format!(
            "        CronetProviderInstaller.installProvider(this).addOnCompleteListener {{ result ->\n            if (!result.isSuccessful) android.util.Log.w(\"Nexa\", \"Play Services Cronet unavailable; using bundled Cronet when available\", result.exception)\n            setContent {{ MaterialTheme {{ {compose_root} }} }}\n        }}\n"
        )
    } else {
        format!("        setContent {{ MaterialTheme {{ {compose_root} }} }}\n")
    };
    write_if_changed(
        &source_dir.join("MainActivity.kt"),
        &format!(
            "package {package}\n\nimport android.os.Bundle\nimport androidx.activity.ComponentActivity\nimport androidx.activity.compose.setContent\nimport androidx.compose.material3.MaterialTheme\n{splash_import}{cronet_import}\nclass MainActivity : ComponentActivity() {{\n    override fun onCreate(savedInstanceState: Bundle?) {{\n{splash_install}        super.onCreate(savedInstanceState)\n{activity_content}    }}\n{permission_callback}}}\n"
        ),
    )?;
    write_if_changed(
        &root.join("android/app/src/main/AndroidManifest.xml"),
        &templates::android_manifest(
            app_name,
            &package,
            project_features.uses_network,
            config,
            plugins,
        ),
    )?;
    let debug_manifest = root.join("android/app/src/debug/AndroidManifest.xml");
    if dev_session.is_some() {
        write_if_changed(
            &debug_manifest,
            "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"><uses-permission android:name=\"android.permission.INTERNET\"/><application android:usesCleartextTraffic=\"true\"/></manifest>\n",
        )?;
    } else if debug_manifest.is_file() {
        fs::remove_file(&debug_manifest)
            .map_err(|error| format!("{}: {error}", debug_manifest.display()))?;
    }
    write_if_changed(
        &root.join("android/settings.gradle.kts"),
        &templates::android_settings(app_name, plugins),
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
        &root.join("android/gradle/wrapper/gradle-wrapper.properties"),
        &templates::android_gradle_wrapper_properties(),
    )?;
    let wrapper_jar = root.join("android/gradle/wrapper/gradle-wrapper.jar");
    write_bytes_if_changed(&wrapper_jar, templates::ANDROID_GRADLE_WRAPPER_JAR)?;
    let gradlew = root.join("android/gradlew");
    write_if_changed(&gradlew, templates::ANDROID_GRADLEW)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&gradlew, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("{}: {error}", gradlew.display()))?;
    }
    write_if_changed(
        &root.join("android/gradlew.bat"),
        templates::ANDROID_GRADLEW_BAT,
    )?;
    write_if_changed(
        &root.join("android/app/build.gradle.kts"),
        &templates::android_app_gradle_with_dev_runtime(
            &package,
            project_features,
            plugins,
            &local_aars,
            config,
            dev_session.is_some(),
        )?,
    )?;
    write_if_changed(
        &root.join("android/app/proguard-rules.pro"),
        &plugins::android_plugin_proguard_rules(plugins, &package)?,
    )?;
    Ok(())
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

fn ios_generated_source(
    module: &Module,
    plugins: &[plugin_package::PluginPackage],
    config: &ProjectConfig,
    dev_runtime: bool,
) -> Result<String, String> {
    let generated = if dev_runtime {
        SwiftBackend.generate_for_dev(module)
    } else {
        SwiftBackend.generate(module)
    };
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
    let plugin_config = plugins::render_swift_plugin_config(plugins, config);
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

fn write_bytes_if_changed(path: &Path, contents: &[u8]) -> Result<(), String> {
    if path.is_file() && fs::read(path).ok().as_deref() == Some(contents) {
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

    use super::{KotlinBackend, SwiftBackend};

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
