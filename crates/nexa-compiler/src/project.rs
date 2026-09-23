use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use nexa_diagnostics::{CompileError, Span};
use nexa_plugin_idl::{
    manifest::parse_file as parse_plugin_manifest, parse_file as parse_plugin_idl,
};
use nexa_syntax::ast::{App, ComponentDecl, ImportDecl, StructDecl};

use crate::{Target, semantic};

pub fn compile_file(path: impl AsRef<Path>) -> Result<nexa_ir::Module, CompileError> {
    Ok(compile_file_with_warnings(path)?.module)
}

pub fn compile_file_with_warnings(
    path: impl AsRef<Path>,
) -> Result<crate::Compilation, CompileError> {
    compile_file_with_target(path, Target::All)
}

pub fn compile_file_for_target(
    path: impl AsRef<Path>,
    target: Target,
) -> Result<nexa_ir::Module, CompileError> {
    Ok(compile_file_with_warnings_for_target(path, target)?.module)
}

pub fn compile_file_with_warnings_for_target(
    path: impl AsRef<Path>,
    target: Target,
) -> Result<crate::Compilation, CompileError> {
    compile_file_with_warnings_for_targets(path, &[target])
        .map(|mut compilations| compilations.remove(0))
}

/// Loads a source project once and lowers it for each requested native target.
///
/// `generate --target all` uses this path so imported files are canonicalized,
/// read, lexed, and parsed once instead of once per backend. Semantic lowering
/// still runs independently for each target because compile-time platform
/// blocks and target-specific diagnostics must remain isolated.
pub fn compile_file_with_warnings_for_targets(
    path: impl AsRef<Path>,
    targets: &[Target],
) -> Result<Vec<crate::Compilation>, CompileError> {
    let entry_path = path.as_ref();
    let mut loaded = LoadedProject::default();
    load_file(
        entry_path,
        true,
        None,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut loaded,
    )?;

    let app = loaded.app.ok_or_else(|| {
        CompileError::new(
            file_level_span(),
            "entry file is missing an `app` declaration",
        )
        .with_file(entry_path.display().to_string())
    })?;
    targets
        .iter()
        .map(|&target| {
            let mut app = app.clone();
            app.components = loaded.components.clone();
            app.structs = loaded.structs.clone();
            app.functions.extend(loaded.functions.clone());
            let (module, mut warnings) = semantic::lower_with_warnings(app, target)
                .map_err(|error| error.with_file(entry_path.display().to_string()))?;
            for warning in &mut warnings {
                if warning.file.is_none() {
                    warning.file = Some(entry_path.display().to_string());
                }
            }
            Ok(crate::Compilation { module, warnings })
        })
        .collect()
}

fn compile_file_with_target(
    path: impl AsRef<Path>,
    target: Target,
) -> Result<crate::Compilation, CompileError> {
    compile_file_with_warnings_for_targets(path, &[target])
        .map(|mut compilations| compilations.remove(0))
}

#[derive(Default)]
struct LoadedProject {
    app: Option<App>,
    components: Vec<ComponentDecl>,
    structs: Vec<StructDecl>,
    functions: Vec<nexa_syntax::ast::FunctionDecl>,
}

fn load_file(
    path: &Path,
    is_entry: bool,
    import_site: Option<(&str, Span)>,
    active: &mut HashSet<PathBuf>,
    loaded_paths: &mut HashSet<PathBuf>,
    loaded: &mut LoadedProject,
) -> Result<(), CompileError> {
    let canonical_path = fs::canonicalize(path).map_err(|error| {
        let (span, file) = import_site
            .map(|(file, span)| (span, file.to_owned()))
            .unwrap_or((file_level_span(), path.display().to_string()));
        CompileError::new(span, format!("cannot resolve source file: {error}")).with_file(file)
    })?;

    if active.contains(&canonical_path) {
        let (span, file) = import_site
            .map(|(file, span)| (span, file.to_owned()))
            .unwrap_or((file_level_span(), canonical_path.display().to_string()));
        return Err(CompileError::new(span, "cyclic source import detected").with_file(file));
    }
    if loaded_paths.contains(&canonical_path) {
        return Ok(());
    }

    let source = fs::read_to_string(&canonical_path).map_err(|error| {
        CompileError::new(
            file_level_span(),
            format!("cannot read source file: {error}"),
        )
        .with_file(canonical_path.display().to_string())
    })?;
    let mut program = nexa_syntax::parse_program(&source)
        .map_err(|error| error.with_file(canonical_path.display().to_string()))?;

    if !program.plugins.is_empty() {
        if !is_entry {
            let plugin = &program.plugins[0];
            return Err(CompileError::new(
                plugin.span,
                "plugin declarations are only allowed in the entry file",
            )
            .with_file(canonical_path.display().to_string()));
        }
        for plugin in &mut program.plugins {
            let declared_path = canonical_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&plugin.path);
            let manifest_path = if declared_path.is_dir() {
                Some(declared_path.join("plugin.config.nx"))
            } else {
                None
            };
            let manifest = manifest_path
                .as_ref()
                .map(|path| {
                    parse_plugin_manifest(path).map_err(|error| {
                        CompileError::new(plugin.span, error)
                            .with_file(canonical_path.display().to_string())
                    })
                })
                .transpose()?;
            if let Some(manifest) = manifest.as_ref() {
                plugin.ios_sources =
                    resolve_manifest_sources(&declared_path, &manifest.ios.sources);
                plugin.android_sources =
                    resolve_manifest_sources(&declared_path, &manifest.android.sources);
                plugin.cpp_sources =
                    resolve_manifest_sources(&declared_path, &manifest.cpp.sources);
                plugin.cpp_headers =
                    resolve_manifest_sources(&declared_path, &manifest.cpp.headers);
                plugin.cpp_standard = manifest.cpp.standard;
                plugin.ios_min_version = manifest.ios.min_version.clone();
                plugin.android_min_sdk = manifest.android.min_sdk;
                plugin.ios_frameworks = manifest.ios.frameworks.clone();
                plugin.ios_xcframeworks =
                    resolve_manifest_sources(&declared_path, &manifest.ios.xcframeworks);
                plugin.ios_resources =
                    resolve_manifest_sources(&declared_path, &manifest.ios.resources);
                plugin.ios_privacy_manifest = manifest
                    .ios
                    .privacy_manifest
                    .as_ref()
                    .map(|path| declared_path.join(path).display().to_string());
                plugin.swift_packages = manifest.ios.swift_packages.clone();
                plugin.maven_dependencies = manifest.android.maven_dependencies.clone();
                plugin.android_aars =
                    resolve_manifest_sources(&declared_path, &manifest.android.aars);
                plugin.android_resources =
                    resolve_manifest_sources(&declared_path, &manifest.android.resources);
                plugin.android_proguard_rules =
                    resolve_manifest_sources(&declared_path, &manifest.android.proguard_rules);
                plugin.android_maven_repositories = manifest.android.repositories.clone();
                plugin.ios_usage_descriptions = manifest.ios.usage_descriptions.clone();
                plugin.ios_entitlements = manifest
                    .ios
                    .entitlements
                    .iter()
                    .map(|(key, value)| {
                        let value = match value {
                            nexa_plugin_idl::manifest::EntitlementValue::String(value) => {
                                nexa_syntax::ast::PluginEntitlementValue::String(value.clone())
                            }
                            nexa_plugin_idl::manifest::EntitlementValue::Bool(value) => {
                                nexa_syntax::ast::PluginEntitlementValue::Bool(*value)
                            }
                            nexa_plugin_idl::manifest::EntitlementValue::Strings(values) => {
                                nexa_syntax::ast::PluginEntitlementValue::Strings(values.clone())
                            }
                        };
                        (key.clone(), value)
                    })
                    .collect();
                plugin.ios_linker_flags = manifest.ios.linker_flags.clone();
                plugin.android_permissions = manifest.android.permissions.clone();
                plugin.assets_path = manifest
                    .assets
                    .first()
                    .and_then(|asset| asset.strip_suffix("/**"))
                    .map(|asset| declared_path.join(asset).display().to_string());
            }
            let pure_source = manifest
                .as_ref()
                .and_then(|manifest| manifest.nexa.as_ref())
                .map(|source| declared_path.join(source))
                .filter(|source| source.is_file());
            if let Some(source) = pure_source.as_ref() {
                // Pure Nexa plugins are ordinary source modules. Loading them
                // through the existing project graph keeps component/function
                // reachability and diagnostics identical to local imports.
                load_file(
                    source,
                    false,
                    Some((&source.display().to_string(), plugin.span)),
                    active,
                    loaded_paths,
                    loaded,
                )?;
                if plugin.assets_path.is_none() {
                    plugin.assets_path = source
                        .parent()
                        .map(|parent| parent.join("assets"))
                        .filter(|path| path.is_dir())
                        .map(|path| path.display().to_string());
                }
            }
            if let Some(source) = &pure_source
                && manifest
                    .as_ref()
                    .is_some_and(|manifest| manifest.native.is_none())
            {
                plugin.pure = true;
                plugin.path = source.display().to_string();
                continue;
            }
            let idl_path = if declared_path.is_dir() {
                let manifest = manifest.ok_or_else(|| {
                    CompileError::new(
                        plugin.span,
                        "plugin directory is missing `plugin.config.nx`",
                    )
                    .with_file(canonical_path.display().to_string())
                })?;
                let native = manifest.native.as_ref().ok_or_else(|| {
                    CompileError::new(
                        plugin.span,
                        "plugin package must declare `sources.native` or `sources.nexa`",
                    )
                    .with_file(canonical_path.display().to_string())
                })?;
                declared_path.join(native)
            } else {
                declared_path
            };
            let idl_path = fs::canonicalize(&idl_path).map_err(|error| {
                CompileError::new(
                    plugin.span,
                    format!("cannot resolve plugin IDL `{}`: {error}", plugin.path),
                )
                .with_file(canonical_path.display().to_string())
            })?;
            plugin.idl = Some(parse_plugin_idl(&idl_path).map_err(|error| {
                CompileError::new(plugin.span, error)
                    .with_file(canonical_path.display().to_string())
            })?);
            plugin.path = idl_path.display().to_string();
        }
        if let Some(app) = program.app.as_mut() {
            app.plugins = program.plugins.clone();
        }
    }

    active.insert(canonical_path.clone());
    for import in &program.imports {
        load_import(import, &canonical_path, active, loaded_paths, loaded)?;
    }

    let source_file = canonical_path.display().to_string();
    loaded
        .components
        .extend(program.components.into_iter().map(|mut component| {
            component.source_file = Some(source_file.clone());
            component
        }));
    loaded
        .structs
        .extend(program.structs.into_iter().map(|mut structure| {
            structure.source_file = Some(source_file.clone());
            structure
        }));
    loaded.functions.extend(program.functions);

    if let Some(app) = program.app {
        if !is_entry {
            return Err(CompileError::new(
                app.span,
                "an imported file can declare components, but not an `app`",
            )
            .with_file(source_file));
        }
        if loaded.app.replace(app).is_some() {
            return Err(CompileError::new(
                file_level_span(),
                "a project can only declare one `app` in its entry file",
            )
            .with_file(source_file));
        }
    } else if is_entry {
        return Err(CompileError::new(
            file_level_span(),
            "entry file is missing an `app` declaration",
        )
        .with_file(source_file));
    }

    active.remove(&canonical_path);
    loaded_paths.insert(canonical_path);
    Ok(())
}

fn resolve_manifest_sources(root: &Path, patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .map(|pattern| root.join(pattern).display().to_string())
        .collect()
}

fn file_level_span() -> Span {
    Span {
        line: 1,
        column: 1,
        ..Span::default()
    }
}

fn load_import(
    import: &ImportDecl,
    importing_file: &Path,
    active: &mut HashSet<PathBuf>,
    loaded_paths: &mut HashSet<PathBuf>,
    loaded: &mut LoadedProject,
) -> Result<(), CompileError> {
    let imported_path = importing_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&import.path);
    let importing_path = importing_file.display().to_string();
    load_file(
        &imported_path,
        false,
        Some((&importing_path, import.span)),
        active,
        loaded_paths,
        loaded,
    )
}
