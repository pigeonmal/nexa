//! Resolved native plugin packages for project scaffolding.
//!
//! Semantic IR keeps only plugin identity (`nexa_ir::Plugin`: namespace plus
//! IDL contract path). Everything scaffolding needs beyond that — resolved
//! source globs, platform frameworks, resources, permissions, and manifests —
//! lives here, built from the manifest-resolved [`nexa_syntax::ast::PluginDecl`]
//! records the compiler hands over alongside each compilation.

use std::collections::HashSet;

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
    pub ios_linker_flags: Vec<String>,
    pub android_permissions: Vec<String>,
}

impl PluginPackage {
    /// Build a package from a manifest-resolved plugin declaration.
    pub fn from_decl(decl: &nexa_syntax::ast::PluginDecl) -> Self {
        let artifacts = PluginArtifacts {
            ios_sources: decl.ios_sources.clone(),
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
            ios_linker_flags: decl.ios_linker_flags.clone(),
            android_permissions: decl.android_permissions.clone(),
        };
        Self {
            namespace: decl.namespace.clone(),
            idl_path: decl.path.clone(),
            artifacts,
        }
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

#[cfg(test)]
mod tests {
    use super::{PluginPackage, packages_for_module};
    use nexa_compiler::Span;

    fn declaration(namespace: &str, pure: bool) -> nexa_syntax::ast::PluginDecl {
        nexa_syntax::ast::PluginDecl {
            path: format!("/{namespace}/native.nxid"),
            namespace: namespace.to_owned(),
            span: Span::default(),
            idl: None,
            pure,
            assets_path: None,
            ios_sources: vec![format!("/{namespace}/ios")],
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
            ios_linker_flags: Vec::new(),
            android_permissions: Vec::new(),
        }
    }

    fn module_with(namespaces: &[&str]) -> nexa_ir::Module {
        nexa_ir::Module {
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
        let package = PluginPackage::from_decl(&declaration("Video", false));
        assert_eq!(package.namespace, "Video");
        assert_eq!(package.idl_path, "/Video/native.nxid");
        assert_eq!(package.artifacts.ios_sources, vec!["/Video/ios"]);
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
}
