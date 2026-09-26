//! Optional plugin source copying and compile-time configuration emission.

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::plan::CopyAction;
use super::plugin_package::PluginPackage;
use nexa_ir::Module;
#[cfg(test)]
use nexa_plugin_idl::manifest::PluginManifest;
use nexa_syntax::ast::ConfigValue;

use super::{ProjectConfig, write_if_changed};

#[cfg(test)]
pub(super) fn validate_manifest_sources(
    package_root: &Path,
    manifest: &PluginManifest,
) -> Result<(), String> {
    plugin_platform_sources(package_root, manifest, true)?;
    plugin_platform_sources(package_root, manifest, false)?;
    declared_cpp_files(
        package_root,
        &manifest.cpp.sources,
        &["cpp", "cc", "cxx"],
        "C++ source",
    )?;
    declared_cpp_files(
        package_root,
        &manifest.cpp.headers,
        &["h", "hh", "hpp", "hxx"],
        "C++ header",
    )?;
    validate_native_artifacts(
        package_root,
        &manifest.ios.xcframeworks,
        "iOS XCFramework",
        true,
    )?;
    validate_native_artifacts(package_root, &manifest.android.aars, "Android AAR", false)?;
    validate_native_artifacts(
        package_root,
        &manifest.ios.resources,
        "iOS platform resource",
        false,
    )?;
    validate_native_artifacts(
        package_root,
        &manifest.android.resources,
        "Android platform resource",
        false,
    )?;
    validate_native_artifacts(
        package_root,
        &manifest.android.proguard_rules,
        "Android ProGuard rule",
        false,
    )?;
    if let Some(privacy_manifest) = &manifest.ios.privacy_manifest {
        validate_native_artifacts(
            package_root,
            std::slice::from_ref(privacy_manifest),
            "iOS privacy manifest",
            false,
        )?;
    }

    let canonical_package_root = fs::canonicalize(package_root)
        .map_err(|error| format!("{}: {error}", package_root.display()))?;
    for asset in &manifest.assets {
        let path = package_root.join(asset);
        let wildcard_index = path.components().position(|component| {
            let value = component.as_os_str().to_string_lossy();
            value.contains('*') || value.contains('?')
        });
        let existing_prefix = if let Some(wildcard_index) = wildcard_index {
            path.components()
                .take(wildcard_index)
                .fold(PathBuf::new(), |mut prefix, component| {
                    prefix.push(component.as_os_str());
                    prefix
                })
        } else {
            path.clone()
        };
        if !existing_prefix.exists() {
            return Err(format!(
                "declared plugin asset path `{asset}` does not exist under {}",
                package_root.display()
            ));
        }
        let canonical_prefix = fs::canonicalize(&existing_prefix)
            .map_err(|error| format!("{}: {error}", existing_prefix.display()))?;
        if !canonical_prefix.starts_with(&canonical_package_root) {
            return Err(format!(
                "declared plugin asset `{asset}` resolves outside the plugin package"
            ));
        }
    }
    Ok(())
}

fn declared_cpp_files(
    package_root: &Path,
    patterns: &[String],
    extensions: &[&str],
    kind: &str,
) -> Result<Vec<PathBuf>, String> {
    let canonical_root = fs::canonicalize(package_root)
        .map_err(|error| format!("{}: {error}", package_root.display()))?;
    let mut files = Vec::new();
    for pattern in patterns {
        let pattern_path = package_root.join(pattern);
        let mut matches = Vec::new();
        for extension in extensions {
            matches.extend(
                expand_source_pattern(&pattern_path, extension, package_root)?
                    .into_iter()
                    .map(|(path, _)| path),
            );
        }
        matches.sort();
        matches.dedup();
        if matches.is_empty() {
            return Err(format!(
                "declared {kind} pattern `{pattern}` matches no supported C++ files"
            ));
        }
        for path in matches {
            let canonical =
                fs::canonicalize(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            if !canonical.starts_with(&canonical_root) {
                return Err(format!(
                    "declared {kind} `{pattern}` resolves outside the plugin package"
                ));
            }
            files.push(canonical);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn plugin_cpp_files(plugin: &PluginPackage, headers: bool) -> Result<Vec<PathBuf>, String> {
    let package_root = Path::new(&plugin.idl_path)
        .parent()
        .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
    let patterns = if headers {
        &plugin.artifacts.cpp_headers
    } else {
        &plugin.artifacts.cpp_sources
    };
    let extensions: &[&str] = if headers {
        &["h", "hh", "hpp", "hxx"]
    } else {
        &["cpp", "cc", "cxx"]
    };
    declared_cpp_files(
        package_root,
        patterns,
        extensions,
        if headers { "C++ header" } else { "C++ source" },
    )
}

#[cfg(test)]
fn validate_native_artifacts(
    package_root: &Path,
    artifacts: &[String],
    kind: &str,
    directory: bool,
) -> Result<(), String> {
    let package_root = fs::canonicalize(package_root)
        .map_err(|error| format!("{}: {error}", package_root.display()))?;
    for artifact in artifacts {
        let source = package_root.join(artifact);
        let canonical = fs::canonicalize(&source)
            .map_err(|error| format!("declared {kind} `{artifact}` does not exist: {error}"))?;
        if !canonical.starts_with(&package_root) {
            return Err(format!(
                "declared {kind} `{artifact}` resolves outside the plugin package"
            ));
        }
        if directory != canonical.is_dir() {
            let expected = if directory { "directory" } else { "file" };
            return Err(format!("declared {kind} `{artifact}` must be a {expected}"));
        }
    }
    Ok(())
}

/// An artifact a plugin contributes to the app bundle, named but not yet
/// copied.
///
/// Staging and copying are separate: staging validates the artifact and
/// decides where it belongs, and the plan's writer performs the copy. The
/// names are what the Xcode project file references.
pub struct StagedArtifact {
    /// Destination path relative to the app directory, as the Xcode project
    /// references it.
    pub name: String,
    /// The copy that materializes it.
    pub copy: CopyAction,
}

/// Stages the XCFrameworks a set of plugins vendors into the app.
///
/// Returns the staged artifacts and the names a previous run staged, so the
/// caller can turn the difference into plan removals instead of deleting
/// directories as a side effect.
pub(super) fn stage_ios_plugin_artifacts(
    root: &Path,
    app_name: &str,
    plugins: &[PluginPackage],
) -> Result<(Vec<StagedArtifact>, Vec<String>), String> {
    let mut artifacts = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        let package_root = plugin_package_root(plugin)?;
        for (artifact_index, artifact) in plugin.artifacts.ios_xcframeworks.iter().enumerate() {
            let source = validate_plugin_artifact(&package_root, artifact, "xcframework", true)?;
            let base_name = source
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("invalid XCFramework path `{}`", source.display()))?;
            let name = format!(
                "NexaPlugin{plugin_index}_{artifact_index}_{}.xcframework",
                sanitize_asset_name(base_name)
            );
            artifacts.push(StagedArtifact {
                name: format!("Frameworks/{name}"),
                copy: CopyAction::directory(source, format!("ios/{app_name}/Frameworks/{name}")),
            });
        }
    }
    artifacts.sort_by(|left, right| left.name.cmp(&right.name));
    let previous = read_staged_names(&root.join("ios").join(app_name).join(MARKER));
    Ok((artifacts, previous))
}

/// Relative marker recording what a staging pass produced, so the next run can
/// tell what to remove.
const MARKER: &str = ".nexa-plugin-frameworks";

/// A resource a plugin contributes to the app bundle, staged but not copied.
pub struct StagedResource {
    /// Destination relative to the resource root, as the manifest records it.
    pub relative: String,
    /// The copy that materializes it.
    pub copy: CopyAction,
}

/// A file generated alongside staged resources, such as a privacy bundle's
/// `Info.plist`.
pub struct StagedGenerated {
    /// Destination relative to the resource root.
    pub relative: String,
    /// Complete file contents.
    pub contents: String,
}

/// Everything one platform needs in order to populate its resource directory.
pub struct StagedResources {
    /// Files to copy in.
    pub copies: Vec<StagedResource>,
    /// Files to generate.
    pub generated: Vec<StagedGenerated>,
    /// Paths a previous build staged, relative to the resource root.
    pub previous: Vec<String>,
    /// Whether the app gains any resource at all.
    pub present: bool,
}

/// Stages the resources and privacy manifests plugins contribute to the iOS
/// app bundle.
///
/// Staging validates every resource and decides where it belongs; the plan
/// carries the copies and the writer performs them. Nothing is written here,
/// so a resource that fails validation leaves the previous build intact.
pub(super) fn stage_ios_plugin_resources(
    root: &Path,
    app_name: &str,
    plugins: &[PluginPackage],
) -> Result<StagedResources, String> {
    let directory = format!("ios/{app_name}/NexaPluginResources");
    let mut copies = Vec::new();
    let mut generated = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        let package_root = plugin_package_root(plugin)?;
        for resource in &plugin.artifacts.ios_resources {
            let source = validate_plugin_file(&package_root, resource, "iOS platform resource")?;
            let relative = plugin_resource_relative(&source, &package_root, plugin_index)?;
            copies.push(StagedResource {
                copy: CopyAction::file(source, format!("{directory}/{relative}")),
                relative,
            });
        }
        if let Some(privacy_manifest) = &plugin.artifacts.ios_privacy_manifest {
            let source =
                validate_plugin_artifact(&package_root, privacy_manifest, "xcprivacy", false)?;
            if source.file_name().and_then(|value| value.to_str()) != Some("PrivacyInfo.xcprivacy")
            {
                return Err(format!(
                    "iOS privacy manifest `{privacy_manifest}` must be named `PrivacyInfo.xcprivacy`"
                ));
            }
            let bundle = format!("NexaPlugin{plugin_index}.bundle");
            copies.push(StagedResource {
                copy: CopyAction::file(
                    source,
                    format!("{directory}/{bundle}/PrivacyInfo.xcprivacy"),
                ),
                relative: format!("{bundle}/PrivacyInfo.xcprivacy"),
            });
            generated.push(StagedGenerated {
                relative: format!("{bundle}/Info.plist"),
                contents: format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>com.nexa.plugin.{}</string><key>CFBundleName</key><string>NexaPlugin{plugin_index}</string><key>CFBundlePackageType</key><string>BNDL</string><key>CFBundleVersion</key><string>1</string></dict></plist>\n",
                    sanitize_asset_name(&plugin.namespace)
                ),
            });
        }
    }
    copies.sort_by(|left, right| left.relative.cmp(&right.relative));
    let present = !copies.is_empty();
    Ok(StagedResources {
        copies,
        generated,
        previous: read_staged_names(&root.join(directory).join(RESOURCE_MARKER)),
        present,
    })
}

/// Stages the resources plugins contribute to the Android asset directory.
pub(super) fn stage_android_plugin_resources(
    root: &Path,
    plugins: &[PluginPackage],
) -> Result<StagedResources, String> {
    let directory = ANDROID_RESOURCE_DIRECTORY;
    let mut copies = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        let package_root = plugin_package_root(plugin)?;
        for resource in &plugin.artifacts.android_resources {
            let source =
                validate_plugin_file(&package_root, resource, "Android platform resource")?;
            let relative = plugin_resource_relative(&source, &package_root, plugin_index)?;
            copies.push(StagedResource {
                copy: CopyAction::file(source, format!("{directory}/{relative}")),
                relative,
            });
        }
    }
    copies.sort_by(|left, right| left.relative.cmp(&right.relative));
    let present = !copies.is_empty();
    Ok(StagedResources {
        copies,
        generated: Vec::new(),
        previous: read_staged_names(&root.join(directory).join(RESOURCE_MARKER)),
        present,
    })
}

/// Where the Android app keeps plugin resources.
const ANDROID_RESOURCE_DIRECTORY: &str = "android/app/src/main/assets/nexa/plugins";
/// Marker recording what a staging pass produced, per resource root.
const RESOURCE_MARKER: &str = ".nexa-plugin-resources";

/// The iOS resource directory for an app, relative to the project root.
pub(super) fn ios_resource_directory(app_name: &str) -> String {
    format!("ios/{app_name}/NexaPluginResources")
}

/// The Android resource directory, relative to the project root.
pub(super) const ANDROID_RESOURCES: &str = ANDROID_RESOURCE_DIRECTORY;

/// Reads the names a previous run recorded in a staging marker.
fn read_staged_names(marker: &Path) -> Vec<String> {
    std::fs::read_to_string(marker)
        .map(|contents| {
            contents
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn copy_android_plugin_artifacts(
    root: &Path,
    plugins: &[PluginPackage],
) -> Result<Vec<String>, String> {
    let library_root = root.join("android/app/libs");
    let marker = root.join("android/app/.nexa-plugin-aars");
    let mut generated = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        let package_root = plugin_package_root(plugin)?;
        for (artifact_index, artifact) in plugin.artifacts.android_aars.iter().enumerate() {
            let source = validate_plugin_artifact(&package_root, artifact, "aar", false)?;
            let base_name = source
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("invalid AAR path `{}`", source.display()))?;
            let name = format!(
                "NexaPlugin{plugin_index}_{artifact_index}_{}.aar",
                sanitize_asset_name(base_name)
            );
            fs::create_dir_all(&library_root)
                .map_err(|error| format!("{}: {error}", library_root.display()))?;
            fs::copy(&source, library_root.join(&name))
                .map_err(|error| format!("{}: {error}", source.display()))?;
            generated.push(name);
        }
    }
    generated.sort();
    remove_stale_files(&library_root, &marker, &generated)?;
    write_manifest(&marker, &generated)?;
    Ok(generated)
}

pub(super) fn android_plugin_proguard_rules(
    plugins: &[PluginPackage],
    app_package: &str,
) -> Result<String, String> {
    let mut output = String::from(
        "# Nexa generated bindings use direct calls and do not require broad keep rules.\n",
    );
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        let package_root = plugin_package_root(plugin)?;
        if !plugin.artifacts.cpp_sources.is_empty() {
            let package = android_plugin_package(plugin, app_package)?;
            output.push_str(&format!(
                "\n# Keep JNI lookup names for C++ plugin `{}` ({plugin_index}).\n-keep class {package}.NexaPlugin{plugin_index}_CppBindings {{ *; }}\n",
                plugin.namespace
            ));
            let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
            let marshalled_types = android_jni_marshalled_named_types(&contract);
            for ty in contract.types.iter().filter(|ty| {
                marshalled_types.contains(&ty.name)
                    && matches!(
                        ty.kind,
                        nexa_plugin_idl::NamedTypeKind::Struct
                            | nexa_plugin_idl::NamedTypeKind::Enum
                    )
            }) {
                output.push_str(&format!("-keep class {package}.{} {{ *; }}\n", ty.name));
            }
            for interface in contract
                .interfaces
                .iter()
                .filter(|interface| interface.kind == nexa_plugin_idl::InterfaceKind::NativeClass)
            {
                for event in &interface.events {
                    output.push_str(&format!(
                        "-keep class {package}.NexaPlugin{plugin_index}_CppEvent{}_{} {{ *; }}\n",
                        interface.name, event.name
                    ));
                }
            }
            let errors = contract
                .interfaces
                .iter()
                .flat_map(|interface| &interface.methods)
                .filter_map(|method| {
                    method.throws.as_ref().or_else(|| {
                        (method.return_type.name == "Result")
                            .then(|| method.return_type.arguments.get(1))
                            .flatten()
                    })
                })
                .map(|ty| ty.name.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            if !errors.is_empty() {
                output.push_str(&format!(
                    "-keep class {package}.NexaPlugin{plugin_index}_CppErrorFactory {{ *; }}\n"
                ));
                for error in errors {
                    output.push_str(&format!(
                        "-keep class {package}.{error} {{ *; }}\n-keep class {package}.{error}$* {{ *; }}\n"
                    ));
                }
            }
        }
        for rule in &plugin.artifacts.android_proguard_rules {
            let source = validate_plugin_artifact(&package_root, rule, "pro", false)?;
            let contents = fs::read_to_string(&source)
                .map_err(|error| format!("{}: {error}", source.display()))?;
            output.push_str(&format!(
                "\n# ProGuard rules from plugin `{}` ({plugin_index})\n",
                plugin.namespace
            ));
            output.push_str(&contents);
            if !contents.ends_with('\n') {
                output.push('\n');
            }
        }
    }
    Ok(output)
}

fn android_jni_marshalled_named_types(
    contract: &nexa_plugin_idl::PluginIdl,
) -> std::collections::BTreeSet<String> {
    fn enqueue_type(ty: &nexa_plugin_idl::TypeRef, pending: &mut Vec<String>) {
        pending.push(ty.name.clone());
        for argument in &ty.arguments {
            enqueue_type(argument, pending);
        }
    }

    let mut pending = Vec::new();
    for interface in contract.interfaces.iter().filter(|interface| {
        matches!(
            interface.kind,
            nexa_plugin_idl::InterfaceKind::Service | nexa_plugin_idl::InterfaceKind::NativeClass
        )
    }) {
        for constructor in &interface.constructors {
            for parameter in &constructor.parameters {
                enqueue_type(&parameter.ty, &mut pending);
            }
        }
        for property in &interface.properties {
            enqueue_type(&property.ty, &mut pending);
        }
        for event in &interface.events {
            for parameter in &event.parameters {
                enqueue_type(&parameter.ty, &mut pending);
            }
        }
        for method in &interface.methods {
            enqueue_type(&method.return_type, &mut pending);
            for parameter in &method.parameters {
                enqueue_type(&parameter.ty, &mut pending);
            }
        }
    }

    let mut reachable = std::collections::BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(named) = contract
            .types
            .iter()
            .find(|ty| ty.name == name && ty.kind == nexa_plugin_idl::NamedTypeKind::Struct)
        {
            for field in &named.fields {
                enqueue_type(&field.ty, &mut pending);
            }
        }
    }
    reachable
}

/// The path a plugin resource occupies inside the resource root.
///
/// Resources are recorded by their full path relative to that root, not by
/// base name: a resource that moves within its package must still be removed
/// from where it used to be, or the old copy lingers in the app bundle.
fn plugin_resource_relative(
    source: &Path,
    package_root: &Path,
    plugin_index: usize,
) -> Result<String, String> {
    let relative = source.strip_prefix(package_root).map_err(|_| {
        format!(
            "plugin resource is outside its package: {}",
            source.display()
        )
    })?;
    Ok(format!(
        "Plugin{plugin_index}/{}",
        relative.to_string_lossy().replace('\\', "/")
    ))
}

fn plugin_package_root(plugin: &PluginPackage) -> Result<PathBuf, String> {
    let idl_path = Path::new(&plugin.idl_path);
    let package_root = idl_path
        .parent()
        .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
    fs::canonicalize(package_root).map_err(|error| format!("{}: {error}", package_root.display()))
}

fn validate_plugin_artifact(
    package_root: &Path,
    artifact: &str,
    extension: &str,
    directory: bool,
) -> Result<PathBuf, String> {
    validate_plugin_path(
        package_root,
        artifact,
        "plugin artifact",
        Some(extension),
        directory,
    )
}

fn validate_plugin_file(package_root: &Path, path: &str, kind: &str) -> Result<PathBuf, String> {
    validate_plugin_path(package_root, path, kind, None, false)
}

fn validate_plugin_path(
    package_root: &Path,
    path: &str,
    kind: &str,
    extension: Option<&str>,
    directory: bool,
) -> Result<PathBuf, String> {
    // A manifest declares its artifacts relative to the package, so a relative
    // path resolves against the package root. Resolving it against the working
    // directory instead would make the same package build or fail depending on
    // where `nexa` was invoked from.
    let declared = Path::new(path);
    let source = if declared.is_absolute() {
        declared.to_path_buf()
    } else {
        package_root.join(declared)
    };
    let canonical = fs::canonicalize(&source)
        .map_err(|error| format!("declared {kind} `{path}` does not exist: {error}"))?;
    if !canonical.starts_with(package_root) {
        return Err(format!(
            "declared {kind} `{path}` resolves outside the plugin package"
        ));
    }
    if let Some(extension) = extension
        && canonical.extension().and_then(|value| value.to_str()) != Some(extension)
    {
        return Err(format!(
            "declared {kind} `{path}` must have the `.{extension}` extension"
        ));
    }
    if directory != canonical.is_dir() {
        let expected = if directory { "directory" } else { "file" };
        return Err(format!("declared {kind} `{path}` must be a {expected}"));
    }
    Ok(canonical)
}

fn remove_stale_files(directory: &Path, marker: &Path, current: &[String]) -> Result<(), String> {
    if let Ok(previous) = fs::read_to_string(marker) {
        for name in previous
            .lines()
            .filter(|name| !current.iter().any(|value| value == name))
        {
            let stale = directory.join(name);
            if stale.is_file() {
                fs::remove_file(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
            }
        }
    }
    Ok(())
}

fn write_manifest(path: &Path, values: &[String]) -> Result<(), String> {
    if values.is_empty() {
        if path.is_file() {
            fs::remove_file(path).map_err(|error| format!("{}: {error}", path.display()))?;
        }
    } else {
        fs::write(path, values.join("\n") + "\n")
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
fn plugin_platform_sources(
    package_root: &Path,
    manifest: &PluginManifest,
    ios: bool,
) -> Result<Vec<PathBuf>, String> {
    let canonical_root = fs::canonicalize(package_root)
        .map_err(|error| format!("{}: {error}", package_root.display()))?;
    let (platform, sources, fallback, extension) = if ios {
        ("iOS", &manifest.ios.sources, "ios/Sources", "swift")
    } else {
        (
            "Android",
            &manifest.android.sources,
            "android/src/main/kotlin",
            "kt",
        )
    };
    let declared = !sources.is_empty();
    let patterns = if declared {
        sources
            .iter()
            .map(|source| (source.clone(), package_root.join(source)))
            .collect::<Vec<_>>()
    } else {
        vec![(fallback.to_owned(), package_root.join(fallback))]
    };
    let mut files = Vec::new();
    for (source, pattern) in patterns {
        let matches = expand_source_pattern(&pattern, extension, package_root)?;
        if declared && matches.is_empty() {
            return Err(format!(
                "declared {platform} plugin source pattern `{source}` matches no .{extension} files"
            ));
        }
        for (file, _) in matches {
            let canonical =
                fs::canonicalize(&file).map_err(|error| format!("{}: {error}", file.display()))?;
            if !canonical.starts_with(&canonical_root) {
                return Err(format!(
                    "declared {platform} plugin source `{source}` resolves outside the plugin package"
                ));
            }
            files.push(file);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

pub(super) fn copy_android_plugin_sources(
    root: &Path,
    plugins: &[PluginPackage],
    app_package: &str,
    config: &ProjectConfig,
) -> Result<(Vec<String>, bool), String> {
    let destination = root.join("android/app/src/main/java");
    let marker = root.join("android/app/.nexa-plugin-sources");
    let mut generated = Vec::new();
    let mut packages = std::collections::BTreeSet::new();
    let mut uses_coroutines = false;
    let has_plugin_config = config.plugins().any(|plugin| {
        !plugin.options.is_empty()
            && plugins
                .iter()
                .any(|used| used.namespace == plugin.namespace)
    });
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        let mut plugin_packages = std::collections::BTreeSet::new();
        let mut has_kotlin_sources = false;
        for (path, source_root) in native_plugin_sources(plugin, "android/src/main/kotlin", "kt")? {
            has_kotlin_sources = true;
            let relative = path.strip_prefix(&source_root).map_err(|_| {
                format!(
                    "plugin source is outside its Kotlin source root: {}",
                    path.display()
                )
            })?;
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if let Some(package) = kotlin_package(&contents) {
                plugin_packages.insert(package);
            }
            let contents = if has_plugin_config {
                kotlin_plugin_config_import(&contents, app_package)
            } else {
                contents
            };
            write_if_changed(&destination.join(relative), &contents)?;
            generated.push(relative.to_string_lossy().into_owned());
        }
        let package = match plugin_packages.iter().next().cloned() {
            Some(package) => package,
            None if !has_kotlin_sources && !plugin.artifacts.cpp_sources.is_empty() => {
                app_package.to_owned()
            }
            None => {
                return Err(format!(
                    "native Kotlin plugin `{}` must declare a package in its source files",
                    plugin.namespace
                ));
            }
        };
        if plugin_packages.len() > 1 {
            return Err(format!(
                "native Kotlin plugin `{}` uses multiple packages; declare one implementation package before project generation",
                plugin.namespace
            ));
        }
        let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
        uses_coroutines |= !plugin.artifacts.cpp_sources.is_empty()
            && contract
                .interfaces
                .iter()
                .flat_map(|interface| &interface.methods)
                .any(|method| method.is_async);
        let binding_name = format!("NexaPlugin{plugin_index}_Bindings.kt");
        let binding_path = destination
            .join(package.replace('.', "/"))
            .join(&binding_name);
        let mut bindings = crate::plugin::render_kotlin_bindings(&contract, &package)?;
        if !plugin.artifacts.cpp_sources.is_empty() {
            let package_root = Path::new(&plugin.idl_path)
                .parent()
                .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
            let manifest =
                nexa_plugin_idl::manifest::parse_file(&package_root.join("plugin.config.nx"))?;
            let (adapters, _) = crate::plugin::render_cpp_android_adapters(
                &contract,
                &manifest.id,
                &plugin.namespace,
                &package,
                plugin_index,
            )?;
            bindings.push_str(&adapters);
        }
        write_if_changed(&binding_path, &bindings)?;
        generated.push(
            binding_path
                .strip_prefix(&destination)
                .map_err(|_| format!("generated binding is outside {}", destination.display()))?
                .to_string_lossy()
                .into_owned(),
        );
        packages.insert(package);
    }
    generated.sort();
    if let Ok(previous) = fs::read_to_string(&marker) {
        for relative in previous
            .lines()
            .filter(|path| !generated.iter().any(|current| current == path))
        {
            let stale = destination.join(relative);
            if stale.is_file() {
                fs::remove_file(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
            }
        }
    }
    if generated.is_empty() {
        if marker.is_file() {
            fs::remove_file(&marker).map_err(|error| format!("{}: {error}", marker.display()))?;
        }
    } else {
        fs::write(&marker, generated.join("\n") + "\n")
            .map_err(|error| format!("{}: {error}", marker.display()))?;
    }
    Ok((packages.into_iter().collect(), uses_coroutines))
}

/// Copies plugin-owned assets into native resource roots and returns whether
/// the iOS asset catalog must be added to the Xcode project.
pub(super) fn copy_plugin_assets(
    root: &Path,
    app_name: &str,
    module: &Module,
) -> Result<bool, String> {
    let android_root = root.join("android/app/src/main/res/drawable-nodpi");
    let ios_root = root
        .join("ios")
        .join(app_name)
        .join("Assets.xcassets/NexaPlugins");
    let android_marker = root.join("android/app/src/main/res/.nexa-plugin-assets");
    let ios_marker = root.join("ios").join(app_name).join(".nexa-plugin-assets");
    let mut has_assets = false;
    let mut generated_android = Vec::new();
    let mut generated_ios = Vec::new();
    for (plugin_index, assets) in module.plugin_assets.iter().enumerate() {
        let source_root = Path::new(&assets.root);
        if !source_root.is_dir() {
            continue;
        }
        let canonical_source_root = fs::canonicalize(source_root)
            .map_err(|error| format!("{}: {error}", source_root.display()))?;
        if let Some(package_root) = &assets.package_root {
            let canonical_package_root = fs::canonicalize(package_root)
                .map_err(|error| format!("{}: {error}", package_root))?;
            if !canonical_source_root.starts_with(&canonical_package_root) {
                return Err(format!(
                    "plugin asset root `{}` resolves outside the plugin package `{}`",
                    source_root.display(),
                    package_root
                ));
            }
        }
        let mut files = Vec::new();
        collect_files(source_root, "", &mut files)?;
        for source in files {
            let canonical_source = fs::canonicalize(&source)
                .map_err(|error| format!("{}: {error}", source.display()))?;
            if !canonical_source.starts_with(&canonical_source_root) {
                return Err(format!(
                    "plugin asset `{}` resolves outside its asset root `{}`",
                    source.display(),
                    source_root.display()
                ));
            }
            let relative = source.strip_prefix(source_root).map_err(|_| {
                format!(
                    "plugin asset is outside its asset root: {}",
                    source.display()
                )
            })?;
            let stem = sanitize_asset_name(&format!(
                "nexa_plugin_{plugin_index}_{}",
                relative.display()
            ));
            let extension = source
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("bin")
                .to_ascii_lowercase();
            let file_name = format!("{stem}.{extension}");

            fs::create_dir_all(&android_root)
                .map_err(|error| format!("{}: {error}", android_root.display()))?;
            let android_destination = android_root.join(&file_name);
            fs::copy(&source, &android_destination)
                .map_err(|error| format!("{}: {error}", source.display()))?;
            generated_android.push(file_name.clone());

            let image_set = ios_root.join(format!("{stem}.imageset"));
            fs::create_dir_all(&image_set)
                .map_err(|error| format!("{}: {error}", image_set.display()))?;
            let ios_file = image_set.join(&file_name);
            fs::copy(&source, &ios_file)
                .map_err(|error| format!("{}: {error}", source.display()))?;
            fs::write(
                image_set.join("Contents.json"),
                format!(
                    "{{\n  \"images\": [{{\"idiom\": \"universal\", \"filename\": \"{file_name}\"}}],\n  \"info\": {{\"author\": \"nexa\", \"version\": 1}}\n}}\n"
                ),
            )
            .map_err(|error| format!("{}: {error}", image_set.display()))?;
            generated_ios.push(format!("{stem}.imageset"));
            has_assets = true;
        }
    }
    if let Ok(previous) = fs::read_to_string(&android_marker) {
        for name in previous
            .lines()
            .filter(|name| !generated_android.iter().any(|current| current == name))
        {
            let stale = android_root.join(name);
            if stale.is_file() {
                fs::remove_file(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
            }
        }
    }
    if !generated_android.is_empty() {
        fs::write(&android_marker, generated_android.join("\n") + "\n")
            .map_err(|error| format!("android plugin asset manifest: {error}"))?;
    } else if android_marker.is_file() {
        fs::remove_file(&android_marker)
            .map_err(|error| format!("{}: {error}", android_marker.display()))?;
    }
    if let Ok(previous) = fs::read_to_string(&ios_marker) {
        for name in previous
            .lines()
            .filter(|name| !generated_ios.iter().any(|current| current == name))
        {
            let stale = ios_root.join(name);
            if stale.is_dir() {
                fs::remove_dir_all(&stale)
                    .map_err(|error| format!("{}: {error}", stale.display()))?;
            }
        }
    }
    if !generated_ios.is_empty() {
        generated_ios.sort();
        fs::write(&ios_marker, generated_ios.join("\n") + "\n")
            .map_err(|error| format!("{}: {error}", ios_marker.display()))?;
    } else if ios_marker.is_file() {
        fs::remove_file(&ios_marker)
            .map_err(|error| format!("{}: {error}", ios_marker.display()))?;
    }
    Ok(has_assets)
}

pub(super) fn copy_ios_plugin_sources(
    root: &Path,
    app_name: &str,
    plugins: &[PluginPackage],
) -> Result<Vec<String>, String> {
    let destination = root.join("ios").join(app_name).join("NexaPlugins");
    let marker = root.join("ios").join(app_name).join(".nexa-plugin-sources");
    let mut names = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        for (source, _) in native_plugin_sources(plugin, "ios/Sources", "swift")? {
            let name = source
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Plugin.swift");
            let output_name = format!(
                "NexaPlugin{plugin_index}_{}",
                sanitize_asset_name(name.trim_end_matches(".swift"))
            );
            let output_name = format!("{output_name}.swift");
            let contents = fs::read_to_string(&source)
                .map_err(|error| format!("{}: {error}", source.display()))?;
            write_if_changed(&destination.join(&output_name), &contents)?;
            names.push(output_name);
        }
        let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
        let binding_name = format!("NexaPlugin{plugin_index}_Bindings.swift");
        write_if_changed(
            &destination.join(&binding_name),
            &crate::plugin::render_swift_bindings(&contract)?,
        )?;
        names.push(binding_name);
        if !plugin.artifacts.cpp_sources.is_empty() {
            let package_root = Path::new(&plugin.idl_path)
                .parent()
                .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
            let manifest =
                nexa_plugin_idl::manifest::parse_file(&package_root.join("plugin.config.nx"))?;
            let adapters = crate::plugin::render_cpp_swift_adapters(&contract, &manifest.id)?;
            if !adapters.is_empty() {
                let adapter_name = format!("NexaPlugin{plugin_index}_CppBindings.swift");
                write_if_changed(&destination.join(&adapter_name), &adapters)?;
                names.push(adapter_name);
            }
        }
    }
    names.sort();
    if let Ok(previous) = fs::read_to_string(&marker) {
        for name in previous
            .lines()
            .filter(|name| !names.iter().any(|current| current == name))
        {
            let stale = destination.join(name);
            if stale.is_file() {
                fs::remove_file(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
            }
        }
    }
    if names.is_empty() {
        if marker.is_file() {
            fs::remove_file(&marker).map_err(|error| format!("{}: {error}", marker.display()))?;
        }
    } else {
        fs::write(&marker, names.join("\n") + "\n")
            .map_err(|error| format!("{}: {error}", marker.display()))?;
    }
    Ok(names)
}

/// Copies opt-in C++ plugin inputs and their generated contracts into the
/// iOS host tree. Paths stay package-relative so sibling headers retain their
/// normal include layout.
pub(super) fn copy_ios_plugin_cpp_sources(
    root: &Path,
    app_name: &str,
    plugins: &[PluginPackage],
) -> Result<Vec<String>, String> {
    let app_directory = root.join("ios").join(app_name);
    let copied = copy_plugin_cpp_sources(
        app_directory.join("NexaPluginCpp"),
        root.join("ios").join(app_name).join(".nexa-plugin-cpp"),
        plugins,
    )?;

    let includes = plugins
        .iter()
        .enumerate()
        .filter(|(_, plugin)| !plugin.artifacts.cpp_sources.is_empty())
        .map(|(plugin_index, _)| {
            format!("#include \"NexaPluginCpp/Plugin{plugin_index}/NexaPluginBindings.hpp\"\n")
        })
        .collect::<String>();
    let bridging_header = app_directory.join("NexaPluginCpp-Bridging-Header.h");
    if includes.is_empty() {
        if bridging_header.is_file() {
            fs::remove_file(&bridging_header)
                .map_err(|error| format!("{}: {error}", bridging_header.display()))?;
        }
    } else {
        write_if_changed(&bridging_header, &includes)?;
    }

    Ok(copied)
}

/// Copies opt-in C++ plugin inputs into Android's CMake source tree and emits
/// a deterministic library target when at least one implementation is used.
pub(super) fn copy_android_plugin_cpp_sources(
    root: &Path,
    plugins: &[PluginPackage],
    app_package: &str,
) -> Result<Vec<String>, String> {
    let destination = root.join("android/app/src/main/cpp");
    let marker = root.join("android/app/.nexa-plugin-cpp");
    let mut sources = copy_plugin_cpp_sources(destination.clone(), marker, plugins)?;
    let jni_marker = root.join("android/app/.nexa-plugin-cpp-jni");
    let mut generated_jni = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        if plugin.artifacts.cpp_sources.is_empty() {
            continue;
        }
        let package_root = Path::new(&plugin.idl_path)
            .parent()
            .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
        let manifest =
            nexa_plugin_idl::manifest::parse_file(&package_root.join("plugin.config.nx"))?;
        let package = android_plugin_package(plugin, app_package)?;
        let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
        let (_, jni) = crate::plugin::render_cpp_android_adapters(
            &contract,
            &manifest.id,
            &plugin.namespace,
            &package,
            plugin_index,
        )?;
        if jni.is_empty() {
            continue;
        }
        let relative = format!("Plugin{plugin_index}/NexaPluginJni.cpp");
        write_if_changed(&destination.join(&relative), &jni)?;
        generated_jni.push(relative.clone());
        sources.push(relative);
    }
    generated_jni.sort();
    if let Ok(previous) = fs::read_to_string(&jni_marker) {
        for relative in previous
            .lines()
            .filter(|path| !generated_jni.iter().any(|current| current == path))
        {
            let stale = destination.join(relative);
            if stale.is_file() {
                fs::remove_file(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
            }
        }
    }
    if generated_jni.is_empty() {
        if jni_marker.is_file() {
            fs::remove_file(&jni_marker)
                .map_err(|error| format!("{}: {error}", jni_marker.display()))?;
        }
    } else {
        fs::write(&jni_marker, generated_jni.join("\n") + "\n")
            .map_err(|error| format!("{}: {error}", jni_marker.display()))?;
    }
    sources.sort();
    let cmake = destination.join("CMakeLists.txt");
    if sources.is_empty() {
        if cmake.is_file() {
            fs::remove_file(&cmake).map_err(|error| format!("{}: {error}", cmake.display()))?;
        }
        return Ok(sources);
    }

    let cpp_standard = minimum_cpp_standard(plugins);
    let mut output = format!(
        "cmake_minimum_required(VERSION 3.22.1)\nproject(nexa_plugins LANGUAGES CXX)\n\nset(CMAKE_CXX_STANDARD {cpp_standard})\nset(CMAKE_CXX_STANDARD_REQUIRED ON)\nset(CMAKE_CXX_EXTENSIONS OFF)\n\nadd_library(nexa_plugins SHARED\n"
    );
    for source in &sources {
        output.push_str(&format!("    \"${{CMAKE_CURRENT_SOURCE_DIR}}/{source}\"\n"));
    }
    output.push_str(")\n\nfind_library(android_log log)\ntarget_link_libraries(nexa_plugins PRIVATE ${android_log})\n");
    for (index, plugin) in plugins.iter().enumerate() {
        if plugin.artifacts.cpp_sources.is_empty() {
            continue;
        }
        let plugin_root = format!("${{CMAKE_CURRENT_SOURCE_DIR}}/Plugin{index}");
        output.push_str(&format!(
            "\ntarget_include_directories(nexa_plugins PRIVATE \"{plugin_root}\""
        ));
        for include in cpp_header_include_roots(plugin, index)? {
            output.push_str(&format!(" \"${{CMAKE_CURRENT_SOURCE_DIR}}/{include}\""));
        }
        output.push_str(")\n");
    }
    write_if_changed(&cmake, &output)?;
    Ok(sources)
}

fn android_plugin_package(plugin: &PluginPackage, default_package: &str) -> Result<String, String> {
    let mut packages = std::collections::BTreeSet::new();
    let mut has_sources = false;
    for (path, _) in native_plugin_sources(plugin, "android/src/main/kotlin", "kt")? {
        has_sources = true;
        let contents =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        if let Some(package) = kotlin_package(&contents) {
            packages.insert(package);
        }
    }
    if packages.len() > 1 {
        return Err(format!(
            "native Kotlin plugin `{}` uses multiple packages; declare one implementation package before project generation",
            plugin.namespace
        ));
    }
    match packages.into_iter().next() {
        Some(package) => Ok(package),
        None if !has_sources => Ok(default_package.to_owned()),
        None => Err(format!(
            "native Kotlin plugin `{}` must declare a package in its source files",
            plugin.namespace
        )),
    }
}

pub(super) fn minimum_cpp_standard(plugins: &[PluginPackage]) -> u8 {
    plugins
        .iter()
        .filter(|plugin| !plugin.artifacts.cpp_sources.is_empty())
        .filter_map(|plugin| plugin.artifacts.cpp_standard)
        .max()
        .unwrap_or(20)
}

fn copy_plugin_cpp_sources(
    destination: PathBuf,
    marker: PathBuf,
    plugins: &[PluginPackage],
) -> Result<Vec<String>, String> {
    let mut generated = Vec::new();
    let mut sources = Vec::new();
    for (plugin_index, plugin) in plugins.iter().enumerate() {
        if plugin.artifacts.cpp_sources.is_empty() {
            continue;
        }
        let plugin_destination = destination.join(format!("Plugin{plugin_index}"));
        for path in plugin_cpp_files(plugin, false)? {
            let relative = cpp_package_relative_path(plugin, &path)?;
            let output_path = plugin_destination.join(&relative);
            copy_if_changed(&path, &output_path)?;
            let project_relative = Path::new(&format!("Plugin{plugin_index}"))
                .join(&relative)
                .to_string_lossy()
                .into_owned();
            generated.push(project_relative.clone());
            if is_cpp_source(&path) {
                sources.push(project_relative);
            }
        }
        for path in plugin_cpp_files(plugin, true)? {
            let relative = cpp_package_relative_path(plugin, &path)?;
            let output_path = plugin_destination.join(&relative);
            copy_if_changed(&path, &output_path)?;
            generated.push(
                Path::new(&format!("Plugin{plugin_index}"))
                    .join(relative)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        let contract = nexa_plugin_idl::parse_file(Path::new(&plugin.idl_path))?;
        let manifest_path = Path::new(&plugin.idl_path)
            .parent()
            .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?
            .join("plugin.config.nx");
        let manifest = nexa_plugin_idl::manifest::parse_file(&manifest_path)?;
        let bindings_path = plugin_destination.join("NexaPluginBindings.hpp");
        write_if_changed(
            &bindings_path,
            &crate::plugin::render_cpp_bindings(&contract, &manifest.id)?,
        )?;
        generated.push(format!("Plugin{plugin_index}/NexaPluginBindings.hpp"));
    }
    generated.sort();
    sources.sort();
    if let Ok(previous) = fs::read_to_string(&marker) {
        for relative in previous
            .lines()
            .filter(|path| !generated.iter().any(|current| current == path))
        {
            let stale = destination.join(relative);
            if stale.is_file() {
                fs::remove_file(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
            }
        }
    }
    if generated.is_empty() {
        if marker.is_file() {
            fs::remove_file(&marker).map_err(|error| format!("{}: {error}", marker.display()))?;
        }
    } else {
        fs::write(&marker, generated.join("\n") + "\n")
            .map_err(|error| format!("{}: {error}", marker.display()))?;
    }
    Ok(sources)
}

fn copy_if_changed(source: &Path, destination: &Path) -> Result<(), String> {
    let contents = fs::read(source).map_err(|error| format!("{}: {error}", source.display()))?;
    if fs::read(destination).ok().as_deref() == Some(contents.as_slice()) {
        return Ok(());
    }
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid plugin C++ path `{}`", destination.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    fs::write(destination, contents).map_err(|error| format!("{}: {error}", destination.display()))
}

fn cpp_package_relative_path(plugin: &PluginPackage, source: &Path) -> Result<PathBuf, String> {
    let package_root = Path::new(&plugin.idl_path)
        .parent()
        .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
    let package_root = fs::canonicalize(package_root)
        .map_err(|error| format!("{}: {error}", package_root.display()))?;
    source
        .strip_prefix(&package_root)
        .map(Path::to_path_buf)
        .map_err(|_| {
            format!(
                "C++ plugin source is outside its package: {}",
                source.display()
            )
        })
}

fn is_cpp_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("cpp" | "cc" | "cxx")
    )
}

pub(super) fn cpp_header_include_roots(
    plugin: &PluginPackage,
    plugin_index: usize,
) -> Result<Vec<String>, String> {
    let mut roots = std::collections::BTreeSet::new();
    for pattern in &plugin.artifacts.cpp_headers {
        let path = Path::new(pattern);
        let components = path.components().collect::<Vec<_>>();
        let wildcard = components.iter().position(|component| {
            let value = component.as_os_str().to_string_lossy();
            value.contains('*') || value.contains('?')
        });
        let include_root = if let Some(index) = wildcard {
            components
                .iter()
                .take(index)
                .fold(PathBuf::new(), |mut result, component| {
                    result.push(component.as_os_str());
                    result
                })
        } else if path.is_file() {
            path.parent().unwrap_or(path).to_path_buf()
        } else {
            path.to_path_buf()
        };
        let package_root = Path::new(&plugin.idl_path)
            .parent()
            .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
        let package_root = fs::canonicalize(package_root)
            .map_err(|error| format!("{}: {error}", package_root.display()))?;
        let include_root = fs::canonicalize(&include_root)
            .map_err(|error| format!("{}: {error}", include_root.display()))?;
        let relative = include_root.strip_prefix(&package_root).map_err(|_| {
            format!(
                "C++ header root is outside its package: {}",
                include_root.display()
            )
        })?;
        roots.insert(
            Path::new(&format!("Plugin{plugin_index}"))
                .join(relative)
                .to_string_lossy()
                .into_owned(),
        );
    }
    Ok(roots.into_iter().collect())
}

fn kotlin_package(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let package = line.trim().strip_prefix("package ")?.trim();
        (!package.is_empty()).then(|| package.to_owned())
    })
}

pub(super) fn render_swift_plugin_config(
    plugins: &[PluginPackage],
    config: &ProjectConfig,
) -> String {
    let plugins = config
        .plugins()
        .filter(|plugin| {
            !plugin.options.is_empty()
                && plugins
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

pub(super) fn render_kotlin_plugin_config(
    plugins: &[PluginPackage],
    config: &ProjectConfig,
) -> String {
    let plugins = config
        .plugins()
        .filter(|plugin| {
            !plugin.options.is_empty()
                && plugins
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

pub(super) fn native_plugin_sources(
    plugin: &PluginPackage,
    relative_root: &str,
    extension: &str,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let plugin_root = Path::new(&plugin.idl_path)
        .parent()
        .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
    let canonical_plugin_root = fs::canonicalize(plugin_root)
        .map_err(|error| format!("{}: {error}", plugin_root.display()))?;
    let patterns = if relative_root.starts_with("ios/") {
        &plugin.artifacts.ios_sources
    } else {
        &plugin.artifacts.android_sources
    };
    let fallback = plugin_root.join(relative_root);
    let patterns = if patterns.is_empty() {
        vec![fallback]
    } else {
        patterns.iter().map(PathBuf::from).collect::<Vec<_>>()
    };
    let mut files = Vec::new();
    for pattern in patterns {
        files.extend(expand_source_pattern(&pattern, extension, plugin_root)?);
    }

    for (file, _) in &files {
        let canonical =
            fs::canonicalize(file).map_err(|error| format!("{}: {error}", file.display()))?;
        if !canonical.starts_with(&canonical_plugin_root) {
            return Err(format!(
                "plugin source `{}` resolves outside the plugin package `{}`",
                file.display(),
                plugin_root.display()
            ));
        }
    }

    files.sort_by(|left, right| left.0.cmp(&right.0));
    files.dedup_by(|left, right| left.0 == right.0);
    Ok(files)
}

fn expand_source_pattern(
    pattern: &Path,
    extension: &str,
    fallback_root: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    if let Ok(metadata) = fs::symlink_metadata(pattern) {
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "plugin traversal rejected symlink `{}`",
                pattern.display()
            ));
        }
        if metadata.is_file() {
            let matches = pattern
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value == extension);
            return Ok(matches
                .then(|| {
                    (
                        pattern.to_path_buf(),
                        pattern.parent().unwrap_or(fallback_root).to_path_buf(),
                    )
                })
                .into_iter()
                .collect());
        }
        if metadata.is_dir() {
            let mut files = Vec::new();
            collect_files(pattern, extension, &mut files)?;
            return Ok(files
                .into_iter()
                .map(|file| (file, pattern.to_path_buf()))
                .collect());
        }
    }

    let components = pattern
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let Some(wildcard_index) = components
        .iter()
        .position(|component| component.contains('*') || component.contains('?'))
    else {
        return Ok(Vec::new());
    };
    let mut root = PathBuf::new();
    for component in &components[..wildcard_index] {
        root.push(component);
    }
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_matching_pattern(
        &root,
        &components[wildcard_index..],
        extension,
        &root,
        &mut files,
    )?;
    Ok(files)
}

fn collect_matching_pattern(
    current: &Path,
    components: &[String],
    extension: &str,
    source_root: &Path,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
    if fs::symlink_metadata(current)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(format!(
            "plugin traversal rejected symlink `{}`",
            current.display()
        ));
    }
    let Some(component) = components.first() else {
        if current.is_file()
            && current
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value == extension)
        {
            files.push((current.to_path_buf(), source_root.to_path_buf()));
        }
        return Ok(());
    };
    if component == "**" {
        collect_matching_pattern(current, &components[1..], extension, source_root, files)?;
        if current.is_dir() {
            for entry in
                fs::read_dir(current).map_err(|error| format!("{}: {error}", current.display()))?
            {
                let entry = entry.map_err(|error| format!("{}: {error}", current.display()))?;
                let file_type = entry
                    .file_type()
                    .map_err(|error| format!("{}: {error}", current.display()))?;
                if file_type.is_symlink() {
                    return Err(format!(
                        "plugin traversal rejected symlink `{}`",
                        entry.path().display()
                    ));
                }
                collect_matching_pattern(&entry.path(), components, extension, source_root, files)?;
            }
        }
        return Ok(());
    }
    if component.contains('*') || component.contains('?') {
        if current.is_dir() {
            for entry in
                fs::read_dir(current).map_err(|error| format!("{}: {error}", current.display()))?
            {
                let entry = entry.map_err(|error| format!("{}: {error}", current.display()))?;
                let file_type = entry
                    .file_type()
                    .map_err(|error| format!("{}: {error}", current.display()))?;
                if file_type.is_symlink() {
                    return Err(format!(
                        "plugin traversal rejected symlink `{}`",
                        entry.path().display()
                    ));
                }
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if wildcard_matches(component, name) {
                    collect_matching_pattern(
                        &entry.path(),
                        &components[1..],
                        extension,
                        source_root,
                        files,
                    )?;
                }
            }
        } else if current
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| wildcard_matches(component, name))
        {
            collect_matching_pattern(current, &components[1..], extension, source_root, files)?;
        }
        return Ok(());
    }
    if current.is_dir() {
        collect_matching_pattern(
            &current.join(component),
            &components[1..],
            extension,
            source_root,
            files,
        )
    } else if current
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == component)
    {
        collect_matching_pattern(current, &components[1..], extension, source_root, files)
    } else {
        Ok(())
    }
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let (mut pattern_index, mut value_index) = (0, 0);
    let mut last_star = None;
    let mut next_value_after_star = 0;
    while value_index < value.len() {
        match pattern.get(pattern_index) {
            Some(b'*') => {
                last_star = Some(pattern_index);
                pattern_index += 1;
                next_value_after_star = value_index;
            }
            Some(b'?') => {
                pattern_index += 1;
                value_index += 1;
            }
            Some(character) if *character == value[value_index] => {
                pattern_index += 1;
                value_index += 1;
            }
            _ => {
                let Some(star_index) = last_star else {
                    return false;
                };
                next_value_after_star += 1;
                value_index = next_value_after_star;
                pattern_index = star_index + 1;
            }
        }
    }
    while pattern.get(pattern_index) == Some(&b'*') {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn collect_files(
    directory: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "plugin traversal rejected symlink `{}`",
            directory.display()
        ));
    }
    for entry in
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "plugin traversal rejected symlink `{}`",
                path.display()
            ));
        }
        if file_type.is_dir() {
            collect_files(&path, extension, files)?;
        } else if file_type.is_file()
            && (extension.is_empty()
                || path.extension().and_then(|value| value.to_str()) == Some(extension))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn sanitize_asset_name(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
        } else {
            result.push('_');
        }
    }
    if result.is_empty() {
        "asset".to_owned()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        android_plugin_proguard_rules, copy_android_plugin_artifacts,
        copy_android_plugin_cpp_sources, copy_ios_plugin_cpp_sources, copy_plugin_assets,
        native_plugin_sources, stage_android_plugin_resources, stage_ios_plugin_artifacts,
        stage_ios_plugin_resources, validate_manifest_sources, wildcard_matches,
    };
    use crate::project::plugin_package::{PluginArtifacts, PluginPackage};

    /// Writes a fixture file, reporting the path if the write fails.
    fn write_fixture(path: &std::path::Path, contents: &[u8]) {
        fs::write(path, contents).unwrap_or_else(|error| {
            panic!("fixture {} should be writable: {error}", path.display())
        });
    }

    /// A temporary project whose directory is owned for as long as it is bound.
    struct TempProject(nexa_testkit::TempDir);

    impl TempProject {
        fn new() -> Self {
            Self(nexa_testkit::TempDir::new("nexa-plugin-artifact-test"))
        }
    }

    #[test]
    fn wildcard_matching_handles_stars_questions_and_backtracking() {
        assert!(wildcard_matches("*.swift", "VideoPlayer.swift"));
        assert!(!wildcard_matches("*.swift", "VideoPlayer.kt"));
        assert!(wildcard_matches("Source?.swift", "Source1.swift"));
        assert!(!wildcard_matches("Source?.swift", "Source12.swift"));
        assert!(wildcard_matches("a*b*c", "axbyc"));
        assert!(wildcard_matches("a**b", "axxb"));
        assert!(wildcard_matches("*", ""));
    }

    #[test]
    fn native_binary_artifacts_are_validated_and_copied_into_host_projects() {
        let temporary = TempProject::new();
        let package = temporary.0.join("video-plugin");
        let xcframework = package.join("ios/VideoSDK.xcframework");
        let aar = package.join("android/libs/vendor.aar");
        let second_aar = package.join("android/alternate/vendor.aar");
        let ios_resource = package.join("ios/Resources/model.dat");
        let privacy_manifest = package.join("ios/PrivacyInfo.xcprivacy");
        let android_resource = package.join("android/resources/model.dat");
        let proguard_rules = package.join("android/rules/vendor.pro");
        let cpp_source = package.join("cpp/Sources/Decoder.cpp");
        let cpp_header = package.join("cpp/include/vendor/Decoder.hpp");
        fs::create_dir_all(&xcframework).expect("XCFramework directory should be created");
        fs::create_dir_all(aar.parent().expect("AAR has a parent"))
            .expect("AAR directory should be created");
        fs::create_dir_all(second_aar.parent().expect("second AAR has a parent"))
            .expect("second AAR directory should be created");
        fs::create_dir_all(ios_resource.parent().expect("resource has a parent"))
            .expect("iOS resource directory should be created");
        fs::create_dir_all(android_resource.parent().expect("resource has a parent"))
            .expect("Android resource directory should be created");
        fs::create_dir_all(proguard_rules.parent().expect("rules have a parent"))
            .expect("ProGuard directory should be created");
        fs::create_dir_all(cpp_source.parent().expect("C++ source has a parent"))
            .expect("C++ source directory should be created");
        fs::create_dir_all(cpp_header.parent().expect("C++ header has a parent"))
            .expect("C++ include directory should be created");
        // Seed paths appear in the failure messages. This test has failed
        // rarely and only under load, with `NotFound` on a write that followed
        // a successful `create_dir_all`; naming the path is what makes such an
        // occurrence diagnosable instead of a bare assertion.
        write_fixture(&xcframework.join("Info.plist"), b"framework metadata");
        write_fixture(&aar, b"aar payload");
        write_fixture(&second_aar, b"second AAR payload");
        write_fixture(&ios_resource, b"iOS resource payload");
        write_fixture(&privacy_manifest, b"<plist/>\n");
        write_fixture(&android_resource, b"Android resource payload");
        write_fixture(&proguard_rules, b"-keep class com.example.sdk.** { *; }\n");
        fs::write(
            &cpp_source,
            "#include \"NexaPluginBindings.hpp\"\n#include \"vendor/Decoder.hpp\"\nint nexa_decoder_version() { return 1; }\n",
        )
        .expect("C++ source should be written");
        fs::write(&cpp_header, "#pragma once\nstruct VendorDecoder {};\n")
            .expect("C++ header should be written");
        let idl = package.join("native.nxid");
        fs::write(&idl, "service VideoCatalog { fn count() -> Int32 }\n")
            .expect("plugin contract should be written");
        fs::write(
            package.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.nexa.video\" version: \"1.0.0\" sources { native: \"native.nxid\" } cpp { sources: [\"cpp/Sources/**\"] headers: [\"cpp/include/**\"] } }\n",
        )
        .expect("plugin package metadata should be written");

        let mut manifest = nexa_plugin_idl::manifest::PluginManifest::default();
        manifest.ios.xcframeworks = vec!["ios/VideoSDK.xcframework".to_owned()];
        manifest.android.aars = vec![
            "android/libs/vendor.aar".to_owned(),
            "android/alternate/vendor.aar".to_owned(),
        ];
        manifest.ios.resources = vec!["ios/Resources/model.dat".to_owned()];
        manifest.ios.privacy_manifest = Some("ios/PrivacyInfo.xcprivacy".to_owned());
        manifest.android.resources = vec!["android/resources/model.dat".to_owned()];
        manifest.android.proguard_rules = vec!["android/rules/vendor.pro".to_owned()];
        manifest.cpp.sources = vec!["cpp/Sources/**".to_owned()];
        manifest.cpp.headers = vec!["cpp/include/**".to_owned()];
        validate_manifest_sources(&package, &manifest)
            .expect("declared local artifacts should be valid");

        let idl = fs::canonicalize(idl).expect("plugin IDL path should canonicalize");
        let _module = nexa_ir::Module {
            app_name: "Demo".to_owned(),
            plugins: vec![nexa_ir::Plugin {
                namespace: "Video".to_owned(),
                idl_path: idl.display().to_string(),
            }],
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let mut packages = vec![PluginPackage {
            namespace: "Video".to_owned(),
            idl_path: idl.display().to_string(),
            artifacts: PluginArtifacts {
                ios_xcframeworks: vec![
                    fs::canonicalize(&xcframework)
                        .expect("XCFramework path should canonicalize")
                        .display()
                        .to_string(),
                ],
                ios_privacy_manifest: Some(
                    fs::canonicalize(&privacy_manifest)
                        .expect("privacy manifest path should canonicalize")
                        .display()
                        .to_string(),
                ),
                android_aars: [&aar, &second_aar]
                    .into_iter()
                    .map(|path| {
                        fs::canonicalize(path)
                            .expect("AAR path should canonicalize")
                            .display()
                            .to_string()
                    })
                    .collect(),
                android_resources: vec![
                    fs::canonicalize(&android_resource)
                        .expect("Android resource path should canonicalize")
                        .display()
                        .to_string(),
                ],
                android_proguard_rules: vec![
                    fs::canonicalize(&proguard_rules)
                        .expect("ProGuard rules path should canonicalize")
                        .display()
                        .to_string(),
                ],
                ..Default::default()
            },
        }];

        // XCFrameworks are staged as data and copied by the plan's writer.
        let (staged, previous) = stage_ios_plugin_artifacts(&temporary.0, "Demo", &packages)
            .expect("XCFramework should be staged for the iOS host");
        assert!(previous.is_empty());
        let ios = staged
            .iter()
            .map(|artifact| artifact.name.clone())
            .collect::<Vec<_>>();
        assert_eq!(ios, vec!["Frameworks/NexaPlugin0_0_videosdk.xcframework"]);
        let plan = crate::project::plan::ProjectPlan::ios("Demo").with_copy(staged[0].copy.clone());
        crate::project::writers::write_plan(&temporary.0, &plan).expect("plan writes the copies");
        assert_eq!(
            fs::read_to_string(
                temporary
                    .0
                    .join("ios/Demo/Frameworks/NexaPlugin0_0_videosdk.xcframework/Info.plist")
            )
            .expect("copied XCFramework metadata should be readable"),
            "framework metadata"
        );

        let android = copy_android_plugin_artifacts(&temporary.0, &packages)
            .expect("AAR should be copied to the Android host");
        assert_eq!(
            android,
            vec!["NexaPlugin0_0_vendor.aar", "NexaPlugin0_1_vendor.aar"]
        );
        let cpp_source_pattern = package.join("cpp/Sources/**").display().to_string();
        let cpp_header_pattern = package.join("cpp/include/**").display().to_string();
        packages[0].artifacts.cpp_sources = vec![cpp_source_pattern];
        packages[0].artifacts.cpp_headers = vec![cpp_header_pattern];
        packages[0].artifacts.cpp_standard = Some(23);
        let ios_cpp = copy_ios_plugin_cpp_sources(&temporary.0, "Demo", &packages)
            .expect("declared C++ sources and generated contract should be staged for iOS");
        assert_eq!(ios_cpp, vec!["Plugin0/cpp/Sources/Decoder.cpp"]);
        assert!(
            temporary
                .0
                .join("ios/Demo/NexaPluginCpp/Plugin0/NexaPluginBindings.hpp")
                .is_file()
        );
        assert!(
            temporary
                .0
                .join("ios/Demo/NexaPluginCpp/Plugin0/cpp/include/vendor/Decoder.hpp")
                .is_file()
        );
        assert_eq!(
            fs::read_to_string(temporary.0.join("ios/Demo/NexaPluginCpp-Bridging-Header.h"))
                .expect("generated Swift bridging header should be readable"),
            "#include \"NexaPluginCpp/Plugin0/NexaPluginBindings.hpp\"\n"
        );
        let android_cpp = copy_android_plugin_cpp_sources(&temporary.0, &packages, "dev.nexa.demo")
            .expect("C++ sources should be staged with an Android CMake target");
        assert_eq!(
            android_cpp,
            vec![
                "Plugin0/NexaPluginJni.cpp",
                "Plugin0/cpp/Sources/Decoder.cpp"
            ]
        );
        let cmake = fs::read_to_string(temporary.0.join("android/app/src/main/cpp/CMakeLists.txt"))
            .expect("CMake project should be generated");
        assert!(cmake.contains("Plugin0/cpp/Sources/Decoder.cpp"));
        assert!(cmake.contains("Plugin0/NexaPluginJni.cpp"));
        assert!(cmake.contains("Plugin0/cpp/include"));
        assert!(cmake.contains("set(CMAKE_CXX_STANDARD 23)"));
        assert_eq!(
            fs::read(
                temporary
                    .0
                    .join("android/app/libs/NexaPlugin0_0_vendor.aar")
            )
            .expect("copied AAR should be readable"),
            b"aar payload"
        );
        assert_eq!(
            fs::read(
                temporary
                    .0
                    .join("android/app/libs/NexaPlugin0_1_vendor.aar")
            )
            .expect("second copied AAR should be readable"),
            b"second AAR payload"
        );

        packages[0].artifacts.ios_resources = vec![
            fs::canonicalize(&ios_resource)
                .expect("iOS resource path should canonicalize")
                .display()
                .to_string(),
        ];
        // Resources are staged as data and written by the plan.
        let staged = stage_ios_plugin_resources(&temporary.0, "Demo", &packages)
            .expect("iOS resources should be staged");
        assert!(staged.present);
        assert_eq!(
            staged
                .copies
                .iter()
                .map(|resource| resource.relative.as_str())
                .collect::<Vec<_>>(),
            vec![
                "NexaPlugin0.bundle/PrivacyInfo.xcprivacy",
                "Plugin0/ios/Resources/model.dat",
            ]
        );
        assert_eq!(staged.generated.len(), 1);
        assert_eq!(
            staged.generated[0].relative,
            "NexaPlugin0.bundle/Info.plist"
        );
        let mut ios_plan = crate::project::plan::ProjectPlan::ios("Demo");
        for resource in &staged.copies {
            ios_plan = ios_plan.with_copy(resource.copy.clone());
        }
        for file in &staged.generated {
            ios_plan = ios_plan.with_file(
                format!("ios/Demo/NexaPluginResources/{}", file.relative),
                file.contents.clone(),
            );
        }
        crate::project::writers::write_plan(&temporary.0, &ios_plan)
            .expect("staged iOS resources should be written");
        assert_eq!(
            fs::read(
                temporary
                    .0
                    .join("ios/Demo/NexaPluginResources/Plugin0/ios/Resources/model.dat")
            )
            .expect("copied iOS resource should be readable"),
            b"iOS resource payload"
        );
        assert!(
            temporary
                .0
                .join("ios/Demo/NexaPluginResources/NexaPlugin0.bundle/PrivacyInfo.xcprivacy")
                .is_file()
        );

        let android = stage_android_plugin_resources(&temporary.0, &packages)
            .expect("Android resources should be staged");
        let mut android_plan = crate::project::plan::ProjectPlan::android("Demo");
        for resource in &android.copies {
            android_plan = android_plan.with_copy(resource.copy.clone());
        }
        crate::project::writers::write_plan(&temporary.0, &android_plan)
            .expect("staged Android resources should be written");
        assert_eq!(
            fs::read(temporary.0.join(
                "android/app/src/main/assets/nexa/plugins/Plugin0/android/resources/model.dat"
            ))
            .expect("copied Android resource should be readable"),
            b"Android resource payload"
        );
        assert!(
            android_plugin_proguard_rules(&packages, "com.example.host")
                .expect("ProGuard rules should be assembled")
                .contains("-keep class com.example.sdk.** { *; }")
        );
    }

    /// Builds a plugin whose declared resources are the given package-relative
    /// paths.
    fn resource_package(root: &std::path::Path, resources: &[&str]) -> PluginPackage {
        std::fs::create_dir_all(root).expect("package root");
        for resource in resources {
            let path = root.join(resource);
            std::fs::create_dir_all(path.parent().expect("resource has a parent"))
                .expect("resource directory");
            write_fixture(&path, b"payload");
        }
        PluginPackage {
            namespace: "Vendor".to_owned(),
            idl_path: root.join("native.nxid").display().to_string(),
            artifacts: PluginArtifacts {
                ios_resources: resources.iter().map(|r| (*r).to_owned()).collect(),
                android_resources: resources.iter().map(|r| (*r).to_owned()).collect(),
                ..Default::default()
            },
        }
    }

    /// Writes a staged set out the way the iOS plan does.
    fn materialize_ios(root: &std::path::Path, staged: &super::StagedResources) {
        let mut plan = crate::project::plan::ProjectPlan::ios("Demo");
        for resource in &staged.copies {
            plan = plan.with_copy(resource.copy.clone());
        }
        let directory = "ios/Demo/NexaPluginResources";
        let now: Vec<&str> = staged
            .copies
            .iter()
            .map(|resource| resource.relative.as_str())
            .collect();
        for stale in &staged.previous {
            if !now.contains(&stale.as_str()) {
                plan = plan.with_removal(format!("{directory}/{stale}"));
            }
        }
        if staged.present {
            plan = plan.with_file(
                format!("{directory}/.nexa-plugin-resources"),
                format!("{}\n", now.join("\n")),
            );
        }
        crate::project::writers::write_plan(root, &plan).expect("staged resources are written");
    }

    #[test]
    fn a_failed_resource_staging_leaves_the_previous_build_intact() {
        // The old implementation deleted the whole resource directory before
        // validating anything, so one malformed resource destroyed a working app.
        let temporary = TempProject::new();
        let good = resource_package(&temporary.0.join("good"), &["ios/Resources/model.dat"]);
        let staged = stage_ios_plugin_resources(&temporary.0, "Demo", std::slice::from_ref(&good))
            .expect("the valid resource stages");
        assert_eq!(
            staged.copies[0].relative, "Plugin0/ios/Resources/model.dat",
            "a resource is recorded by its full path, not its base name"
        );
        materialize_ios(&temporary.0, &staged);
        let resource = temporary
            .0
            .join("ios/Demo/NexaPluginResources/Plugin0/ios/Resources/model.dat");
        assert!(resource.is_file(), "the first build wrote the resource");

        // A plugin whose resource escapes its package must fail, and must not
        // take the previous build's resources with it.
        let outside = temporary.0.join("outside.dat");
        write_fixture(&outside, b"secret");
        let broken = PluginPackage {
            namespace: "Broken".to_owned(),
            idl_path: temporary.0.join("broken/native.nxid").display().to_string(),
            artifacts: PluginArtifacts {
                ios_resources: vec!["../outside.dat".to_owned()],
                ..Default::default()
            },
        };
        let result = stage_ios_plugin_resources(&temporary.0, "Demo", &[good, broken]);
        assert!(result.is_err(), "a resource outside the package must fail");
        assert!(
            resource.is_file(),
            "a failed staging must not delete what the previous build wrote"
        );
        assert_eq!(
            std::fs::read_to_string(
                temporary
                    .0
                    .join("ios/Demo/NexaPluginResources/.nexa-plugin-resources")
            )
            .expect("the staging manifest survives"),
            "Plugin0/ios/Resources/model.dat\n"
        );
    }

    #[test]
    fn a_resource_that_moves_within_its_package_is_removed_from_its_old_path() {
        let temporary = TempProject::new();
        let before = resource_package(&temporary.0.join("pkg"), &["ios/Resources/old/model.dat"]);
        let staged = stage_ios_plugin_resources(&temporary.0, "Demo", &[before]).expect("staging");
        materialize_ios(&temporary.0, &staged);
        let old_path = temporary
            .0
            .join("ios/Demo/NexaPluginResources/Plugin0/ios/Resources/old/model.dat");
        assert!(old_path.is_file());

        // The same plugin now declares the resource at a new path.
        let mut after =
            resource_package(&temporary.0.join("pkg"), &["ios/Resources/new/model.dat"]);
        after.artifacts.ios_resources = vec!["ios/Resources/new/model.dat".to_owned()];
        let staged = stage_ios_plugin_resources(&temporary.0, "Demo", &[after]).expect("staging");
        assert_eq!(staged.previous, vec!["Plugin0/ios/Resources/old/model.dat"]);
        materialize_ios(&temporary.0, &staged);

        assert!(
            !old_path.exists(),
            "a resource that moved must not linger at its old path"
        );
        assert!(
            temporary
                .0
                .join("ios/Demo/NexaPluginResources/Plugin0/ios/Resources/new/model.dat")
                .is_file()
        );
    }

    #[test]
    fn android_resources_stage_under_the_asset_directory() {
        let temporary = TempProject::new();
        let plugin = resource_package(&temporary.0.join("pkg"), &["android/res/model.dat"]);
        let staged = stage_android_plugin_resources(&temporary.0, &[plugin]).expect("staging");
        assert_eq!(staged.copies.len(), 1);
        assert_eq!(
            staged.copies[0].copy.destination,
            "android/app/src/main/assets/nexa/plugins/Plugin0/android/res/model.dat"
        );
        assert!(staged.present);
    }

    #[test]
    #[cfg(unix)]
    fn source_discovery_rejects_traversal_symlinks() {
        let temporary = TempProject::new();
        let package = temporary.0.join("plugin-symlink");
        let sources = package.join("ios/Sources");
        fs::create_dir_all(&sources).expect("sources directory created");
        let external = temporary.0.join("external-secret");
        fs::create_dir_all(&external).expect("external directory created");
        fs::write(external.join("Secret.swift"), "// secret").expect("secret file written");
        std::os::unix::fs::symlink(&external, sources.join("SymlinkDir")).expect("symlink created");

        let plugin = PluginPackage {
            namespace: "demo".to_string(),
            idl_path: package.join("native.nxid").display().to_string(),
            artifacts: PluginArtifacts {
                ios_sources: vec![],
                android_sources: vec![],
                cpp_sources: vec![],
                cpp_headers: vec![],
                cpp_standard: None,
                ios_min_version: None,
                android_min_sdk: None,
                ios_frameworks: vec![],
                ios_xcframeworks: vec![],
                ios_resources: vec![],
                ios_privacy_manifest: None,
                swift_packages: vec![],
                maven_dependencies: vec![],
                android_aars: vec![],
                android_resources: vec![],
                android_proguard_rules: vec![],
                android_maven_repositories: vec![],
                ios_usage_descriptions: vec![],
                ios_entitlements: vec![],
                ios_linker_flags: vec![],
                android_permissions: vec![],
            },
        };
        fs::write(package.join("native.nxid"), "namespace demo {}").expect("idl written");

        let result = native_plugin_sources(&plugin, "ios/Sources", "swift");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("plugin traversal rejected symlink")
        );
    }

    #[test]
    fn source_discovery_rejects_files_escaping_package_bounds() {
        let temporary = TempProject::new();
        let package = temporary.0.join("plugin-bounds");
        fs::create_dir_all(&package).expect("package directory created");
        let outside = temporary.0.join("Outside.swift");
        fs::write(&outside, "// outside").expect("outside file written");
        fs::write(package.join("native.nxid"), "namespace demo {}").expect("idl written");

        let plugin = PluginPackage {
            namespace: "demo".to_string(),
            idl_path: package.join("native.nxid").display().to_string(),
            artifacts: PluginArtifacts {
                ios_sources: vec![outside.display().to_string()],
                android_sources: vec![],
                cpp_sources: vec![],
                cpp_headers: vec![],
                cpp_standard: None,
                ios_min_version: None,
                android_min_sdk: None,
                ios_frameworks: vec![],
                ios_xcframeworks: vec![],
                ios_resources: vec![],
                ios_privacy_manifest: None,
                swift_packages: vec![],
                maven_dependencies: vec![],
                android_aars: vec![],
                android_resources: vec![],
                android_proguard_rules: vec![],
                android_maven_repositories: vec![],
                ios_usage_descriptions: vec![],
                ios_entitlements: vec![],
                ios_linker_flags: vec![],
                android_permissions: vec![],
            },
        };

        let result = native_plugin_sources(&plugin, "ios/Sources", "swift");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("resolves outside the plugin package")
        );
    }

    #[test]
    #[cfg(unix)]
    fn asset_discovery_rejects_asset_root_linked_outside_package() {
        let temporary = TempProject::new();
        let package = temporary.0.join("plugin-assets");
        fs::create_dir_all(&package).expect("package directory created");
        let external_assets = temporary.0.join("external-assets");
        fs::create_dir_all(&external_assets).expect("external assets created");
        fs::write(external_assets.join("logo.png"), b"png").expect("image written");
        let linked_root = package.join("linked-assets");
        std::os::unix::fs::symlink(&external_assets, &linked_root).expect("symlink created");

        let module = nexa_ir::Module {
            app_name: "DemoApp".to_string(),
            plugins: vec![],
            plugin_assets: vec![nexa_ir::PluginAsset {
                root: linked_root.display().to_string(),
                package_root: Some(package.display().to_string()),
            }],
            enums: vec![],
            structs: vec![],
            functions: vec![],
            states: vec![],
            screens: vec![],
            components: vec![],
            body: vec![],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let result = copy_plugin_assets(&temporary.0, "DemoApp", &module);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("resolves outside the plugin package")
        );
    }

    #[test]
    #[cfg(unix)]
    fn asset_discovery_rejects_traversal_symlinks() {
        let temporary = TempProject::new();
        let package = temporary.0.join("plugin-asset-symlink");
        let assets = package.join("assets");
        fs::create_dir_all(&assets).expect("assets directory created");
        let outside = temporary.0.join("secret-file.txt");
        fs::write(&outside, "secret").expect("outside file written");
        std::os::unix::fs::symlink(&outside, assets.join("symlink.txt")).expect("symlink created");

        let module = nexa_ir::Module {
            app_name: "DemoApp".to_string(),
            plugins: vec![],
            plugin_assets: vec![nexa_ir::PluginAsset {
                root: assets.display().to_string(),
                package_root: Some(package.display().to_string()),
            }],
            enums: vec![],
            structs: vec![],
            functions: vec![],
            states: vec![],
            screens: vec![],
            components: vec![],
            body: vec![],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let result = copy_plugin_assets(&temporary.0, "DemoApp", &module);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("plugin traversal rejected symlink")
        );
    }
}
