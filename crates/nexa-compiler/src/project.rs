use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use nexa_diagnostics::{CompileError, Span};
use nexa_plugin_compiler_api::{
    ANALYZER_PROTOCOL_VERSION, AnalysisRequest, AnalyzerProcess, SourceFile as AnalyzerSourceFile,
    TargetConfiguration,
};
use nexa_plugin_idl::{
    manifest::parse_file as parse_plugin_manifest, parse_file as parse_plugin_idl,
};
use nexa_syntax::ast::{App, ClassDecl, ComponentDecl, ImportDecl, PluginDecl, StructDecl};

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
    compile_file_with_warnings_for_targets_and_plugin_roots(path, targets, &HashMap::new())
}

/// Loads and compiles a project while resolving plugin package IDs to local
/// package roots supplied by the CLI dependency resolver.
pub fn compile_file_with_warnings_for_targets_and_plugin_roots(
    path: impl AsRef<Path>,
    targets: &[Target],
    plugin_roots: &HashMap<String, PathBuf>,
) -> Result<Vec<crate::Compilation>, CompileError> {
    IncrementalProjectCompiler::default().compile_file_with_warnings_for_targets_and_plugin_roots(
        path,
        targets,
        plugin_roots,
    )
}

/// Compiles source for the hot-reload interpreter. This accepts nullable
/// generic plugin reads because DevRuntime carries decode validity separately
/// from nullable values in its plugin codec results.
pub fn compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
    path: impl AsRef<Path>,
    targets: &[Target],
    plugin_roots: &HashMap<String, PathBuf>,
) -> Result<Vec<crate::Compilation>, CompileError> {
    IncrementalProjectCompiler::default()
        .compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
            path,
            targets,
            plugin_roots,
        )
}

/// Reuses parsed source files across builds in a long-lived development
/// session. Semantic analysis still runs for the assembled project so changes
/// to shared declarations are checked against every dependent declaration.
#[derive(Default)]
pub struct IncrementalProjectCompiler {
    parsed_sources: HashMap<PathBuf, CachedProgram>,
    analyzers: HashMap<String, AnalyzerProcess>,
    last_stats: ProjectCompileStats,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectCompileStats {
    pub parsed_source_files: usize,
    pub reused_source_files: usize,
}

struct CachedProgram {
    source: String,
    program: nexa_syntax::ast::Program,
}

struct PluginWarning {
    target: Option<String>,
    warning: nexa_diagnostics::CompileWarning,
}

struct PluginAnalysis {
    warnings: Vec<PluginWarning>,
}

impl IncrementalProjectCompiler {
    /// Counts source files freshly parsed and reused by the last compilation
    /// attempt. Counts reset for every compile call.
    pub fn last_compile_stats(&self) -> ProjectCompileStats {
        self.last_stats
    }

    /// Incrementally loads and compiles a Nexa source graph for the requested
    /// native targets, reusing parsed ASTs whose source text is unchanged.
    pub fn compile_file_with_warnings_for_targets_and_plugin_roots(
        &mut self,
        path: impl AsRef<Path>,
        targets: &[Target],
        plugin_roots: &HashMap<String, PathBuf>,
    ) -> Result<Vec<crate::Compilation>, CompileError> {
        self.compile_file_with_warnings_for_targets_and_plugin_roots_in_mode(
            path,
            targets,
            plugin_roots,
            false,
            &HashMap::new(),
        )
    }

    /// Compiles an on-disk project while substituting in-memory source text for
    /// any canonical source paths in `source_overrides`. Language servers use
    /// this to analyze open, unsaved buffers together with their import graph.
    pub fn compile_file_with_warnings_for_targets_and_plugin_roots_with_overrides(
        &mut self,
        path: impl AsRef<Path>,
        targets: &[Target],
        plugin_roots: &HashMap<String, PathBuf>,
        source_overrides: &HashMap<PathBuf, String>,
    ) -> Result<Vec<crate::Compilation>, CompileError> {
        self.compile_file_with_warnings_for_targets_and_plugin_roots_in_mode(
            path,
            targets,
            plugin_roots,
            false,
            source_overrides,
        )
    }

    /// Compiles source for hot reload, enabling type shapes interpreted by
    /// DevRuntime that AOT Kotlin cannot represent without additional runtime
    /// wrappers.
    pub fn compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
        &mut self,
        path: impl AsRef<Path>,
        targets: &[Target],
        plugin_roots: &HashMap<String, PathBuf>,
    ) -> Result<Vec<crate::Compilation>, CompileError> {
        self.compile_file_with_warnings_for_targets_and_plugin_roots_in_mode(
            path,
            targets,
            plugin_roots,
            true,
            &HashMap::new(),
        )
    }

    fn compile_file_with_warnings_for_targets_and_plugin_roots_in_mode(
        &mut self,
        path: impl AsRef<Path>,
        targets: &[Target],
        plugin_roots: &HashMap<String, PathBuf>,
        dev_runtime: bool,
        source_overrides: &HashMap<PathBuf, String>,
    ) -> Result<Vec<crate::Compilation>, CompileError> {
        self.last_stats = ProjectCompileStats::default();
        let entry_path = path.as_ref();
        let mut context = ProjectLoadContext {
            active: HashSet::new(),
            loaded_paths: HashSet::new(),
            loaded: LoadedProject::default(),
            plugin_roots,
            source_overrides,
            compiler: self,
        };
        load_file(entry_path, true, None, &mut context)?;
        let ProjectLoadContext {
            loaded,
            loaded_paths,
            ..
        } = context;
        self.parsed_sources
            .retain(|path, _| loaded_paths.contains(path));

        let mut app = loaded.app.ok_or_else(|| {
            CompileError::new(
                file_level_span(),
                "entry file is missing an `app` declaration",
            )
            .with_file(entry_path.display().to_string())
        })?;
        app.plugins = loaded.plugins;
        let plugin_analysis = if app
            .plugins
            .iter()
            .any(|plugin| !plugin.compiler_analyzer.is_empty())
        {
            let (ios_minimum, android_min_sdk) = project_target_minimums(entry_path)?;
            self.run_plugin_analyzers(
                &app.plugins,
                entry_path,
                targets,
                &ios_minimum,
                android_min_sdk,
                &loaded_paths,
            )?
        } else {
            PluginAnalysis {
                warnings: Vec::new(),
            }
        };
        targets
            .iter()
            .map(|&target| {
                let mut app = app.clone();
                app.components = loaded.components.clone();
                app.enums.extend(loaded.enums.clone());
                app.structs = loaded.structs.clone();
                app.classes = loaded.classes.clone();
                app.screens.extend(loaded.screens.clone());
                app.widgets.extend(loaded.widgets.clone());
                app.functions.extend(loaded.functions.clone());
                app.globals.extend(loaded.globals.clone());
                app.tests = loaded
                    .tests
                    .iter()
                    .filter(|test| {
                        !test.source_file.as_deref().is_some_and(|source_file| {
                            let source_file = Path::new(source_file);
                            plugin_roots
                                .values()
                                .any(|plugin_root| source_file.starts_with(plugin_root))
                        })
                    })
                    .cloned()
                    .collect();
                let plugins = app.plugins.clone();
                let lowered = semantic::lower_with_project_targets(app, target, dev_runtime);
                let (module, mut warnings, tests) =
                    lowered.map_err(|error| error.with_file(entry_path.display().to_string()))?;
                warnings.extend(
                    plugin_analysis
                        .warnings
                        .iter()
                        .filter(|warning| {
                            warning.target.as_deref().is_none_or(|warning_target| {
                                target == Target::All || warning_target == target.as_str()
                            })
                        })
                        .map(|warning| warning.warning.clone()),
                );
                for warning in &mut warnings {
                    if warning.file.is_none() {
                        warning.file = Some(entry_path.display().to_string());
                    }
                }
                Ok(crate::Compilation {
                    module,
                    warnings,
                    plugins,
                    tests,
                })
            })
            .collect()
    }

    fn run_plugin_analyzers(
        &mut self,
        plugins: &[PluginDecl],
        entry_path: &Path,
        targets: &[Target],
        ios_minimum: &str,
        android_min_sdk: u32,
        loaded_paths: &HashSet<PathBuf>,
    ) -> Result<PluginAnalysis, CompileError> {
        if !plugins
            .iter()
            .any(|plugin| !plugin.compiler_analyzer.is_empty())
        {
            return Ok(PluginAnalysis {
                warnings: Vec::new(),
            });
        }
        let mut source_files = loaded_paths
            .iter()
            .filter_map(|path| {
                self.parsed_sources
                    .get(path)
                    .map(|source| AnalyzerSourceFile {
                        path: path.display().to_string(),
                        contents: source.source.clone(),
                    })
            })
            .collect::<Vec<_>>();
        source_files.sort_by(|left, right| left.path.cmp(&right.path));
        let target_configurations =
            analyzer_target_configurations(targets, ios_minimum, android_min_sdk);
        let mut warnings = Vec::new();
        for plugin in plugins
            .iter()
            .filter(|plugin| !plugin.compiler_analyzer.is_empty())
        {
            let Some(package_id) = plugin.package_id.as_deref() else {
                return Err(CompileError::new(
                    plugin.span,
                    "a compiler analyzer requires a plugin package ID",
                )
                .with_file(entry_path.display().to_string()));
            };
            let Some(package_root) = plugin.package_root.as_deref() else {
                return Err(CompileError::new(
                    plugin.span,
                    "a compiler analyzer requires a resolved plugin package root",
                )
                .with_file(entry_path.display().to_string()));
            };
            let request = AnalysisRequest::new(
                package_id,
                &plugin.namespace,
                package_root,
                entry_path.display().to_string(),
                target_configurations.clone(),
                source_files.clone(),
            );
            let process_key = format!(
                "{package_id}:{package_root}:{}",
                plugin.compiler_analyzer.join("\0")
            );
            if !self.analyzers.contains_key(&process_key) {
                let process =
                    AnalyzerProcess::spawn(&plugin.compiler_analyzer, Path::new(package_root))
                        .map_err(|message| {
                            CompileError::new(plugin.span, message)
                                .with_file(entry_path.display().to_string())
                        })?;
                self.analyzers.insert(process_key.clone(), process);
            }
            let response = self
                .analyzers
                .get_mut(&process_key)
                .map(|process| process.analyze(&request))
                .ok_or_else(|| {
                    CompileError::new(plugin.span, "plugin analyzer process was not retained")
                        .with_file(entry_path.display().to_string())
                })?;
            let response = match response {
                Ok(response) => response,
                Err(message) => {
                    self.analyzers.remove(&process_key);
                    return Err(CompileError::new(plugin.span, message)
                        .with_file(entry_path.display().to_string()));
                }
            };
            if response.protocol_version != ANALYZER_PROTOCOL_VERSION {
                return Err(CompileError::new(
                    plugin.span,
                    format!(
                        "plugin analyzer protocol mismatch: Nexa supports {}, analyzer returned {}",
                        ANALYZER_PROTOCOL_VERSION, response.protocol_version
                    ),
                )
                .with_file(entry_path.display().to_string()));
            }
            for diagnostic in response.diagnostics {
                if diagnostic
                    .target
                    .as_deref()
                    .is_some_and(|diagnostic_target| {
                        !target_configurations
                            .iter()
                            .any(|target| target.target == diagnostic_target)
                    })
                {
                    continue;
                }
                let span = Span {
                    start: diagnostic.start,
                    end: diagnostic.end,
                    line: diagnostic.line,
                    column: diagnostic.column,
                };
                let file = Some(diagnostic.file);
                match diagnostic.severity {
                    nexa_plugin_compiler_api::DiagnosticSeverity::Error => {
                        return Err(CompileError {
                            span,
                            message: diagnostic.message,
                            file,
                        });
                    }
                    nexa_plugin_compiler_api::DiagnosticSeverity::Warning => {
                        warnings.push(PluginWarning {
                            target: diagnostic.target,
                            warning: nexa_diagnostics::CompileWarning {
                                span,
                                message: diagnostic.message,
                                file,
                            },
                        });
                    }
                }
            }
        }
        Ok(PluginAnalysis { warnings })
    }

    fn parse_source(
        &mut self,
        path: &Path,
        source: String,
    ) -> Result<nexa_syntax::ast::Program, CompileError> {
        if let Some(cached) = self.parsed_sources.get(path)
            && cached.source == source
        {
            self.last_stats.reused_source_files += 1;
            return Ok(cached.program.clone());
        }
        let program = nexa_syntax::parse_program(&source)
            .map_err(|error| error.with_file(path.display().to_string()))?;
        self.last_stats.parsed_source_files += 1;
        self.parsed_sources.insert(
            path.to_owned(),
            CachedProgram {
                source,
                program: program.clone(),
            },
        );
        Ok(program)
    }
}

impl Target {
    fn as_str(self) -> &'static str {
        match self {
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::All => "all",
        }
    }
}

fn analyzer_target_configurations(
    targets: &[Target],
    ios_minimum: &str,
    android_min_sdk: u32,
) -> Vec<TargetConfiguration> {
    let mut configurations = Vec::with_capacity(2);
    for target in targets {
        match target {
            Target::Swift => configurations.push(TargetConfiguration {
                target: "swift".to_owned(),
                ios_minimum_version: Some(ios_minimum.to_owned()),
                android_min_sdk: None,
            }),
            Target::Kotlin => configurations.push(TargetConfiguration {
                target: "kotlin".to_owned(),
                ios_minimum_version: None,
                android_min_sdk: Some(android_min_sdk),
            }),
            Target::All => {
                configurations.push(TargetConfiguration {
                    target: "swift".to_owned(),
                    ios_minimum_version: Some(ios_minimum.to_owned()),
                    android_min_sdk: None,
                });
                configurations.push(TargetConfiguration {
                    target: "kotlin".to_owned(),
                    ios_minimum_version: None,
                    android_min_sdk: Some(android_min_sdk),
                });
            }
        }
    }
    configurations.sort_by(|left, right| left.target.cmp(&right.target));
    configurations.dedup_by(|left, right| left.target == right.target);
    configurations
}

fn compile_file_with_target(
    path: impl AsRef<Path>,
    target: Target,
) -> Result<crate::Compilation, CompileError> {
    compile_file_with_warnings_for_targets(path, &[target])
        .map(|mut compilations| compilations.remove(0))
}

fn project_target_minimums(entry_path: &Path) -> Result<(String, u32), CompileError> {
    let config_path = entry_path
        .ancestors()
        .map(|directory| directory.join("nexa.config.nx"))
        .find(|candidate| candidate.is_file());
    let Some(config_path) = config_path else {
        // These match the project generator's defaults, so direct project
        // compilation and generated builds report the same compatibility floor.
        return Ok(("16.0".to_owned(), 23));
    };
    let source = fs::read_to_string(&config_path).map_err(|error| {
        CompileError::new(
            file_level_span(),
            format!("cannot read project config: {error}"),
        )
        .with_file(config_path.display().to_string())
    })?;
    let config = nexa_syntax::parse_config(&source)
        .map_err(|error| error.with_file(config_path.display().to_string()))?;
    Ok((
        config
            .ios
            .and_then(|ios| ios.min_version)
            .unwrap_or_else(|| "16.0".to_owned()),
        config
            .android
            .and_then(|android| android.min_sdk)
            .unwrap_or(23),
    ))
}

#[cfg(test)]
mod target_config_tests {
    use super::project_target_minimums;
    use nexa_testkit::TestProject;

    #[test]
    fn reads_platform_minimums_from_the_nearest_project_config() {
        let project = TestProject::new("nexa-plugin-platform-minimums");
        let entry = project.write_app("app Demo { body { Text(\"ready\") } }\n");
        project.write_config("config { ios { minVersion: \"13.2\" } android { minSdk: 27 } }\n");

        assert_eq!(
            project_target_minimums(&entry).expect("valid project config"),
            ("13.2".to_owned(), 27)
        );
    }

    #[test]
    fn uses_the_same_defaults_as_the_project_generator_when_config_is_absent() {
        let project = TestProject::new("nexa-plugin-default-platform-minimums");
        let entry = project.write_app("app Demo { body { Text(\"ready\") } }\n");
        assert_eq!(
            project_target_minimums(&entry).expect("default targets"),
            ("16.0".to_owned(), 23)
        );
    }
}

#[cfg(all(test, unix))]
mod plugin_analyzer_tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    use super::IncrementalProjectCompiler;
    use crate::Target;
    use nexa_testkit::TestProject;

    #[test]
    fn invokes_plugin_analyzers_once_and_routes_targeted_diagnostics() {
        let project = TestProject::new("nexa-plugin-analyzer-project");
        let source = "plugin \"./analyzer\" as Analyzer\napp Demo {\n    state value = 1\n    body { Text(\"ready\") }\n}\n";
        let entry = project.write_app(source);
        let package = project.join("analyzer");
        fs::create_dir_all(&package).expect("plugin package directory");
        fs::write(
            package.join("plugin.config.nx"),
            r#"plugin {
                schema: 2
                id: "dev.example.analyzer"
                version: "1.0.0"
                sources { native: "native.nxid" }
                compiler { analyzer: ["./analyzer.sh"] }
            }"#,
        )
        .expect("plugin manifest");
        fs::write(
            package.join("native.nxid"),
            "native class Marker { init() }\n",
        )
        .expect("plugin contract");
        let analyzer = package.join("analyzer.sh");
        let analyzer_source = "#!/bin/sh\nwhile IFS= read -r _request; do printf '%s\\n' '{\"protocol_version\":1,\"diagnostics\":[{\"severity\":\"warning\",\"message\":\"analyzer warning\",\"file\":\"App.nx\",\"start\":0,\"end\":1,\"line\":1,\"column\":1,\"target\":\"swift\"}]}'; done\n";
        fs::write(&analyzer, analyzer_source).expect("analyzer executable source");
        fs::set_permissions(&analyzer, fs::Permissions::from_mode(0o700))
            .expect("analyzer executable permissions");

        let mut compiler = IncrementalProjectCompiler::default();
        let compiled = compiler
            .compile_file_with_warnings_for_targets_and_plugin_roots(
                &entry,
                &[Target::Swift, Target::Kotlin],
                &std::collections::HashMap::new(),
            )
            .expect("project compiles");
        assert_eq!(compiled.len(), 2);
        assert!(
            compiled[0]
                .warnings
                .iter()
                .any(|warning| warning.message == "analyzer warning")
        );
        assert!(
            !compiled[1]
                .warnings
                .iter()
                .any(|warning| warning.message == "analyzer warning")
        );

        compiler
            .compile_file_with_warnings_for_targets_and_plugin_roots(
                &entry,
                &[Target::Swift],
                &std::collections::HashMap::new(),
            )
            .expect("the retained analyzer accepts a second request");
    }
}

#[derive(Default)]
struct LoadedProject {
    app: Option<App>,
    enums: Vec<nexa_syntax::ast::EnumDecl>,
    components: Vec<ComponentDecl>,
    structs: Vec<StructDecl>,
    classes: Vec<ClassDecl>,
    functions: Vec<nexa_syntax::ast::FunctionDecl>,
    globals: Vec<nexa_syntax::ast::StateDecl>,
    screens: Vec<nexa_syntax::ast::ScreenDecl>,
    widgets: Vec<nexa_syntax::ast::WidgetDecl>,
    tests: Vec<nexa_syntax::ast::TestDecl>,
    plugins: Vec<PluginDecl>,
}

struct ProjectLoadContext<'a> {
    active: HashSet<PathBuf>,
    loaded_paths: HashSet<PathBuf>,
    loaded: LoadedProject,
    plugin_roots: &'a HashMap<String, PathBuf>,
    source_overrides: &'a HashMap<PathBuf, String>,
    compiler: &'a mut IncrementalProjectCompiler,
}

fn load_file(
    path: &Path,
    is_entry: bool,
    import_site: Option<(&str, Span)>,
    context: &mut ProjectLoadContext<'_>,
) -> Result<(), CompileError> {
    let canonical_path = fs::canonicalize(path).map_err(|error| {
        let (span, file) = import_site
            .map(|(file, span)| (span, file.to_owned()))
            .unwrap_or((file_level_span(), path.display().to_string()));
        CompileError::new(span, format!("cannot resolve source file: {error}")).with_file(file)
    })?;

    if context.active.contains(&canonical_path) {
        let (span, file) = import_site
            .map(|(file, span)| (span, file.to_owned()))
            .unwrap_or((file_level_span(), canonical_path.display().to_string()));
        return Err(CompileError::new(span, "cyclic source import detected").with_file(file));
    }
    if context.loaded_paths.contains(&canonical_path) {
        return Ok(());
    }

    let source = match context.source_overrides.get(&canonical_path) {
        Some(source) => source.clone(),
        None => fs::read_to_string(&canonical_path).map_err(|error| {
            CompileError::new(
                file_level_span(),
                format!("cannot read source file: {error}"),
            )
            .with_file(canonical_path.display().to_string())
        })?,
    };
    let mut program = context.compiler.parse_source(&canonical_path, source)?;

    if !program.plugins.is_empty() {
        for plugin in &mut program.plugins {
            let declared_path = context
                .plugin_roots
                .get(&plugin.path)
                .cloned()
                .unwrap_or_else(|| {
                    canonical_path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(&plugin.path)
                });
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
                plugin.package_id = Some(manifest.id.clone());
                plugin.package_root = fs::canonicalize(&declared_path)
                    .ok()
                    .map(|path| path.display().to_string());
                plugin.compiler_analyzer = manifest
                    .compiler
                    .as_ref()
                    .map(|compiler| compiler.analyzer.clone())
                    .unwrap_or_default();
                plugin.ios_sources =
                    resolve_manifest_sources(&declared_path, &manifest.ios.sources);
                plugin.ios_extension_sources =
                    resolve_manifest_sources(&declared_path, &manifest.ios.extension_sources);
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
                plugin.android_application_metadata = manifest.android.application_metadata.clone();
                plugin.ios_application_delegate = manifest.ios.application_delegate.clone();
                plugin.android_firebase_messaging_service =
                    manifest.android.firebase_messaging_service.clone();
                plugin.android_picture_in_picture = manifest.android.picture_in_picture;
                plugin.ios_background_modes = manifest.ios.background_modes.clone();
                plugin.android_media_playback_service =
                    manifest.android.media_playback_service.clone();
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
                    context,
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
        for plugin in program.plugins.drain(..) {
            if let Some(previous) = context
                .loaded
                .plugins
                .iter()
                .find(|previous| previous.namespace == plugin.namespace)
            {
                if previous.path != plugin.path {
                    return Err(CompileError::new(
                        plugin.span,
                        format!(
                            "plugin namespace `{}` refers to more than one package",
                            plugin.namespace
                        ),
                    )
                    .with_file(canonical_path.display().to_string()));
                }
                continue;
            }
            context.loaded.plugins.push(plugin);
        }
    }

    context.active.insert(canonical_path.clone());
    for import in &program.imports {
        load_import(import, &canonical_path, context)?;
    }

    let source_file = canonical_path.display().to_string();
    context
        .loaded
        .components
        .extend(program.components.into_iter().map(|mut component| {
            component.source_file = Some(source_file.clone());
            component
        }));
    context.loaded.enums.extend(program.enums);
    context
        .loaded
        .structs
        .extend(program.structs.into_iter().map(|mut structure| {
            structure.source_file = Some(source_file.clone());
            structure
        }));
    context
        .loaded
        .classes
        .extend(program.classes.into_iter().map(|mut class| {
            class.source_file = Some(source_file.clone());
            class
        }));
    context
        .loaded
        .functions
        .extend(program.functions.into_iter().map(|mut function| {
            function.source_file = Some(source_file.clone());
            function
        }));
    context
        .loaded
        .globals
        .extend(program.globals.into_iter().map(|mut state| {
            state.source_file = Some(source_file.clone());
            state
        }));
    context
        .loaded
        .screens
        .extend(program.screens.into_iter().map(|mut screen| {
            screen.source_file = Some(source_file.clone());
            screen
        }));
    context
        .loaded
        .widgets
        .extend(program.widgets.into_iter().map(|mut widget| {
            widget.source_file = Some(source_file.clone());
            widget
        }));
    context
        .loaded
        .tests
        .extend(program.tests.into_iter().map(|mut test| {
            test.source_file = Some(source_file.clone());
            test
        }));

    if let Some(mut app) = program.app {
        for state in app.states.iter_mut().chain(app.globals.iter_mut()) {
            state.source_file = Some(source_file.clone());
        }
        for function in &mut app.functions {
            function.source_file = Some(source_file.clone());
        }
        for screen in &mut app.screens {
            screen.source_file = Some(source_file.clone());
            for state in &mut screen.states {
                state.source_file = Some(source_file.clone());
            }
        }
        for component in &mut app.components {
            component.source_file = Some(source_file.clone());
            for state in &mut component.states {
                state.source_file = Some(source_file.clone());
            }
        }
        for widget in &mut app.widgets {
            widget.source_file = Some(source_file.clone());
        }
        if !is_entry {
            return Err(
                CompileError::new(app.span, "an imported file cannot declare an `app`")
                    .with_file(source_file),
            );
        }
        if context.loaded.app.replace(app).is_some() {
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

    context.active.remove(&canonical_path);
    context.loaded_paths.insert(canonical_path);
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
    context: &mut ProjectLoadContext<'_>,
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
        context,
    )
}
