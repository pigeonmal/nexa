//! Resolved native plugin packages for project scaffolding.
//!
//! Semantic IR keeps only plugin identity (`nexa_ir::Plugin`: namespace plus
//! IDL contract path). Everything scaffolding needs beyond that — resolved
//! source globs, platform frameworks, resources, permissions, and manifests —
//! lives here, built from the manifest-resolved [`nexa_syntax::ast::PluginDecl`]
//! records the compiler hands over alongside each compilation.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use nexa_plugin_idl::manifest::{EntitlementValue, SwiftPackage};

/// A non-pure plugin package ready for project scaffolding.
#[derive(Clone, Debug)]
pub struct PluginPackage {
    /// Plugin namespace used for generated names (`NexaPlugin{index}`).
    pub namespace: String,
    /// Path to the plugin IDL contract; doubles as the package-root anchor.
    pub idl_path: String,
    /// Resolved platform artifacts declared by `plugin.config.nx`.
    pub artifacts: PluginArtifacts,
}

/// Platform file sets and metadata consumed by project scaffolding.
#[derive(Clone, Debug, Default)]
pub struct PluginArtifacts {
    /// Absolute conventional iOS source roots/globs.
    pub ios_sources: Vec<String>,
    /// Absolute Swift sources declared safe for app-extension targets.
    pub ios_extension_sources: Vec<String>,
    /// Absolute conventional Android source roots/globs.
    pub android_sources: Vec<String>,
    /// Absolute optional C++ implementation source globs.
    pub cpp_sources: Vec<String>,
    /// Absolute optional C++ header globs.
    pub cpp_headers: Vec<String>,
    /// Minimum C++ language standard required by native C++ sources.
    pub cpp_standard: Option<u8>,
    pub ios_min_version: Option<String>,
    pub android_min_sdk: Option<u32>,
    pub ios_frameworks: Vec<String>,
    /// Absolute paths to local XCFramework bundles.
    pub ios_xcframeworks: Vec<String>,
    /// Absolute paths to plugin-owned iOS bundle resources.
    pub ios_resources: Vec<String>,
    /// Absolute path to the plugin's `PrivacyInfo.xcprivacy` manifest.
    pub ios_privacy_manifest: Option<String>,
    pub swift_packages: Vec<SwiftPackage>,
    pub maven_dependencies: Vec<String>,
    /// Absolute paths to local Android AAR artifacts.
    pub android_aars: Vec<String>,
    /// Absolute paths to plugin-owned Android assets.
    pub android_resources: Vec<String>,
    /// Absolute paths to Android R8/ProGuard rules.
    pub android_proguard_rules: Vec<String>,
    pub android_maven_repositories: Vec<String>,
    pub ios_usage_descriptions: Vec<(String, String)>,
    pub ios_entitlements: Vec<(String, EntitlementValue)>,
    pub ios_application_delegate: Option<String>,
    pub ios_background_modes: Vec<String>,
    pub ios_linker_flags: Vec<String>,
    pub android_permissions: Vec<String>,
    pub android_application_metadata: Vec<(String, String)>,
    pub android_firebase_messaging_service: Option<String>,
    pub android_picture_in_picture: bool,
    pub android_media_playback_service: Option<String>,
}

impl PluginPackage {
    /// Build a package from a manifest-resolved plugin declaration.
    pub fn from_decl(decl: &nexa_syntax::ast::PluginDecl) -> Self {
        let artifacts = PluginArtifacts {
            ios_sources: decl.ios_sources.clone(),
            ios_extension_sources: decl.ios_extension_sources.clone(),
            android_sources: decl.android_sources.clone(),
            cpp_sources: decl.cpp_sources.clone(),
            cpp_headers: decl.cpp_headers.clone(),
            cpp_standard: decl.cpp_standard,
            ios_min_version: decl.ios_min_version.clone(),
            android_min_sdk: decl.android_min_sdk,
            ios_frameworks: decl.ios_frameworks.clone(),
            ios_xcframeworks: decl.ios_xcframeworks.clone(),
            ios_resources: decl.ios_resources.clone(),
            ios_privacy_manifest: decl.ios_privacy_manifest.clone(),
            swift_packages: decl.swift_packages.clone(),
            maven_dependencies: decl.maven_dependencies.clone(),
            android_aars: decl.android_aars.clone(),
            android_resources: decl.android_resources.clone(),
            android_proguard_rules: decl.android_proguard_rules.clone(),
            android_maven_repositories: decl.android_maven_repositories.clone(),
            ios_usage_descriptions: decl.ios_usage_descriptions.clone(),
            ios_entitlements: decl
                .ios_entitlements
                .iter()
                .map(|(key, value)| {
                    let value = match value {
                        nexa_syntax::ast::PluginEntitlementValue::String(value) => {
                            EntitlementValue::String(value.clone())
                        }
                        nexa_syntax::ast::PluginEntitlementValue::Bool(value) => {
                            EntitlementValue::Bool(*value)
                        }
                        nexa_syntax::ast::PluginEntitlementValue::Strings(values) => {
                            EntitlementValue::Strings(values.clone())
                        }
                    };
                    (key.clone(), value)
                })
                .collect(),
            ios_application_delegate: decl.ios_application_delegate.clone(),
            ios_background_modes: decl.ios_background_modes.clone(),
            ios_linker_flags: decl.ios_linker_flags.clone(),
            android_permissions: decl.android_permissions.clone(),
            android_application_metadata: decl.android_application_metadata.clone(),
            android_firebase_messaging_service: decl.android_firebase_messaging_service.clone(),
            android_picture_in_picture: decl.android_picture_in_picture,
            android_media_playback_service: decl.android_media_playback_service.clone(),
        };
        Self {
            namespace: decl.namespace.clone(),
            idl_path: decl.path.clone(),
            artifacts,
        }
    }

    /// Build a native package from a configured dependency before its first
    /// source import. Dev hosts need this metadata so later hot reloads can
    /// import the already-configured plugin without relinking the host.
    pub fn from_package_root(namespace: &str, root: &Path) -> Result<Option<Self>, String> {
        let manifest_path = root.join("plugin.config.nx");
        let manifest = nexa_plugin_idl::manifest::parse_file(&manifest_path)?;
        let Some(native) = manifest.native else {
            return Ok(None);
        };
        let idl_path = fs::canonicalize(root.join(native)).map_err(|error| {
            format!(
                "{}: cannot resolve native plugin contract: {error}",
                manifest_path.display()
            )
        })?;
        let absolute_paths = |paths: &[String]| {
            paths
                .iter()
                .map(|path| root.join(path).display().to_string())
                .collect::<Vec<_>>()
        };
        let artifacts = PluginArtifacts {
            ios_sources: absolute_paths(&manifest.ios.sources),
            ios_extension_sources: absolute_paths(&manifest.ios.extension_sources),
            android_sources: absolute_paths(&manifest.android.sources),
            cpp_sources: absolute_paths(&manifest.cpp.sources),
            cpp_headers: absolute_paths(&manifest.cpp.headers),
            cpp_standard: manifest.cpp.standard,
            ios_min_version: manifest.ios.min_version,
            android_min_sdk: manifest.android.min_sdk,
            ios_frameworks: manifest.ios.frameworks,
            ios_xcframeworks: absolute_paths(&manifest.ios.xcframeworks),
            ios_resources: absolute_paths(&manifest.ios.resources),
            ios_privacy_manifest: manifest
                .ios
                .privacy_manifest
                .map(|path| root.join(path).display().to_string()),
            swift_packages: manifest.ios.swift_packages,
            maven_dependencies: manifest.android.maven_dependencies,
            android_aars: absolute_paths(&manifest.android.aars),
            android_resources: absolute_paths(&manifest.android.resources),
            android_proguard_rules: absolute_paths(&manifest.android.proguard_rules),
            android_maven_repositories: manifest.android.repositories,
            ios_usage_descriptions: manifest.ios.usage_descriptions,
            ios_entitlements: manifest.ios.entitlements,
            ios_application_delegate: manifest.ios.application_delegate,
            ios_background_modes: manifest.ios.background_modes,
            ios_linker_flags: manifest.ios.linker_flags,
            android_permissions: manifest.android.permissions,
            android_application_metadata: manifest.android.application_metadata,
            android_firebase_messaging_service: manifest.android.firebase_messaging_service,
            android_picture_in_picture: manifest.android.picture_in_picture,
            android_media_playback_service: manifest.android.media_playback_service,
        };
        Ok(Some(Self {
            namespace: namespace.to_owned(),
            idl_path: idl_path.display().to_string(),
            artifacts,
        }))
    }
}

/// Build generation packages from resolved declarations: non-pure plugins
/// whose namespaces survived pruning, in declaration order. This mirrors the
/// pruned IR plugin list exactly (same filter, same order), so generated
/// `NexaPlugin{index}` names are unchanged.
pub fn packages_for_module(
    declarations: &[nexa_syntax::ast::PluginDecl],
    module: &nexa_ir::Module,
) -> Vec<PluginPackage> {
    let live: HashSet<&str> = module
        .plugins
        .iter()
        .map(|plugin| plugin.namespace.as_str())
        .collect();
    let packages: Vec<PluginPackage> = declarations
        .iter()
        .filter(|declaration| !declaration.pure && live.contains(declaration.namespace.as_str()))
        .map(PluginPackage::from_decl)
        .collect();
    debug_assert!(
        packages
            .iter()
            .map(|package| package.namespace.as_str())
            .eq(module
                .plugins
                .iter()
                .map(|plugin| plugin.namespace.as_str())),
        "plugin packages must mirror the pruned IR plugin list"
    );
    packages
}

/// Build the native packages a development host must have available for later
/// hot reloads. Unlike an AOT target, the DevRuntime may receive a call to a
/// plugin that the first module did not reference, so every configured native
/// dependency is linked into the host up front.
pub fn packages_for_dev(declarations: &[nexa_syntax::ast::PluginDecl]) -> Vec<PluginPackage> {
    declarations
        .iter()
        .filter(|declaration| !declaration.pure)
        .map(PluginPackage::from_decl)
        .collect()
}

/// Include configured native dependencies in the debug host before source
/// code references them, while avoiding duplicate copies for plugins already
/// declared by the entry module.
pub fn packages_for_dev_with_dependencies(
    declarations: &[nexa_syntax::ast::PluginDecl],
    dependencies: &[nexa_syntax::ast::PluginDependencyConfig],
    plugin_roots: &HashMap<String, PathBuf>,
) -> Result<Vec<PluginPackage>, String> {
    let mut packages = packages_for_dev(declarations);
    for dependency in dependencies {
        let root = plugin_roots.get(&dependency.package_id).ok_or_else(|| {
            format!(
                "configured plugin `{}` was not resolved to a package root",
                dependency.alias
            )
        })?;
        let Some(package) = PluginPackage::from_package_root(&dependency.alias, root)? else {
            continue;
        };
        if !packages
            .iter()
            .any(|existing| existing.idl_path == package.idl_path)
        {
            packages.push(package);
        }
    }
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::{PluginPackage, packages_for_dev, packages_for_module};
    use nexa_compiler::Span;

    fn declaration(namespace: &str, pure: bool) -> nexa_syntax::ast::PluginDecl {
        nexa_syntax::ast::PluginDecl {
            path: format!("/{namespace}/native.nxid"),
            package_id: None,
            package_root: None,
            compiler_analyzer: Vec::new(),
            namespace: namespace.to_owned(),
            span: Span::default(),
            idl: None,
            pure,
            assets_path: None,
            ios_sources: vec![format!("/{namespace}/ios")],
            ios_extension_sources: vec![format!("/{namespace}/ios/WidgetSafe.swift")],
            android_sources: Vec::new(),
            cpp_sources: vec![format!("/{namespace}/cpp/**")],
            cpp_headers: Vec::new(),
            cpp_standard: Some(20),
            ios_min_version: None,
            android_min_sdk: None,
            ios_frameworks: Vec::new(),
            ios_xcframeworks: Vec::new(),
            ios_resources: Vec::new(),
            ios_privacy_manifest: None,
            swift_packages: Vec::new(),
            maven_dependencies: Vec::new(),
            android_aars: Vec::new(),
            android_resources: Vec::new(),
            android_proguard_rules: Vec::new(),
            android_maven_repositories: Vec::new(),
            ios_usage_descriptions: Vec::new(),
            ios_entitlements: Vec::new(),
            ios_application_delegate: None,
            ios_background_modes: Vec::new(),
            ios_linker_flags: Vec::new(),
            android_permissions: Vec::new(),
            android_application_metadata: Vec::new(),
            android_firebase_messaging_service: None,
            android_picture_in_picture: false,
            android_media_playback_service: None,
        }
    }

    fn module_with(namespaces: &[&str]) -> nexa_ir::Module {
        nexa_ir::Module {
            widgets: Vec::new(),
            app_name: "PackagesTest".to_owned(),
            plugins: namespaces
                .iter()
                .map(|namespace| nexa_ir::Plugin {
                    namespace: (*namespace).to_owned(),
                    idl_path: format!("/{namespace}/native.nxid"),
                })
                .collect(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
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
        }
    }

    #[test]
    fn from_decl_carries_identity_and_artifacts() {
        let mut declaration = declaration("Video", false);
        declaration.android_picture_in_picture = true;
        let package = PluginPackage::from_decl(&declaration);
        assert_eq!(package.namespace, "Video");
        assert_eq!(package.idl_path, "/Video/native.nxid");
        assert!(package.artifacts.android_picture_in_picture);
        assert_eq!(package.artifacts.ios_sources, vec!["/Video/ios"]);
        assert_eq!(
            package.artifacts.ios_extension_sources,
            vec!["/Video/ios/WidgetSafe.swift"]
        );
        assert_eq!(package.artifacts.cpp_sources, vec!["/Video/cpp/**"]);
        assert_eq!(package.artifacts.cpp_standard, Some(20));
    }

    #[test]
    fn packages_mirror_the_pruned_plugin_list() {
        let declarations = vec![
            declaration("Dropped", false),
            declaration("Video", false),
            declaration("Pure", true),
            declaration("Audio", false),
        ];
        // The optimizer pruned `Dropped`; pure plugins never reach the IR.
        let packages = packages_for_module(&declarations, &module_with(&["Video", "Audio"]));
        let namespaces: Vec<&str> = packages
            .iter()
            .map(|package| package.namespace.as_str())
            .collect();
        assert_eq!(namespaces, ["Video", "Audio"]);
    }

    #[test]
    fn dev_packages_include_configured_native_plugins_before_first_use() {
        let declarations = [
            declaration("dev.nexa.used", false),
            declaration("dev.nexa.pure", true),
        ];
        let packages = packages_for_dev(&declarations);
        let namespaces: Vec<&str> = packages
            .iter()
            .map(|package| package.namespace.as_str())
            .collect();
        assert_eq!(namespaces, ["dev.nexa.used"]);
    }
}
