use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use nexa_ir::Permission;
use nexa_plugin_idl::{ConfigOption, Literal, PluginIdl, TypeRef};
use nexa_syntax::ast::{ConfigValue, PluginDecl, PluginDependencyConfig};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum AndroidCronetProvider {
    #[default]
    PlayServices,
    Embedded,
}

impl AndroidCronetProvider {
    pub(super) fn config_value(self) -> &'static str {
        match self {
            Self::PlayServices => "play-services",
            Self::Embedded => "embedded",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct PluginDefinition {
    pub(super) namespace: String,
    pub(super) idl: PluginIdl,
}

#[derive(Clone, Debug)]
pub(super) struct PluginConfig {
    pub(super) namespace: String,
    pub(super) options: Vec<ResolvedPluginOption>,
}

#[derive(Clone, Debug)]
pub(super) struct ResolvedPluginOption {
    pub(super) name: String,
    pub(super) ty: TypeRef,
    pub(super) value: ConfigValue,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ProjectConfig {
    pub(super) display_name: String,
    pub(super) orientation: String,
    pub(super) version: String,
    pub(super) build_number: u32,
    pub(super) staging_suffix: String,
    pub(super) flavors: Vec<nexa_syntax::ast::FlavorConfig>,
    pub(super) deep_links: Vec<String>,
    permissions: Vec<(Permission, String)>,
    plugins: Vec<PluginConfig>,
    pub(super) ios_min_version: String,
    pub(super) ios_arch: Option<Vec<String>>,
    pub(super) android_min_sdk: u32,
    pub(super) android_arch: Option<Vec<String>>,
    pub(super) android_cronet_provider: AndroidCronetProvider,
    pub(super) android_cronet_disk_cache_size_mb: u32,
    pub(super) android_target_sdk: u32,
    pub(super) ios_bundle_identifier: String,
    pub(super) ios_app_group_identifier: Option<String>,
    pub(super) android_application_id: String,
    pub(super) ios_icon: Option<PathBuf>,
    pub(super) android_icon: Option<PathBuf>,
    pub(super) ios_alternate_icons: Vec<PathBuf>,
    pub(super) android_alternate_icons: Vec<PathBuf>,
    pub(super) dependencies: Vec<PluginDependencyConfig>,
    pub(super) icon_source: Option<PathBuf>,
    pub(super) splash_source: Option<PathBuf>,
}

impl ProjectConfig {
    pub(super) fn set_arch_override(&mut self, arch: &str, target: &str) -> Result<(), String> {
        if matches!(target, "ios" | "all") {
            validate_ios_arch(arch)?;
            self.ios_arch = Some(vec![arch.to_owned()]);
        }
        if matches!(target, "android" | "all") {
            android_abi_for_arch(arch)?;
            self.android_arch = Some(vec![arch.to_owned()]);
        }
        Ok(())
    }

    pub(super) fn with_flavor(&self, name: &str) -> Result<Self, String> {
        let mut config = self.clone();
        let configured = config.flavors.iter().find(|flavor| flavor.name == name);
        let suffix = match configured {
            Some(flavor) => flavor.suffix.clone().unwrap_or_else(|| name.to_owned()),
            None if name == "staging" => config.staging_suffix.clone(),
            None => {
                return Err(format!(
                    "unknown flavor `{name}`; declare it in `nexa.config.nx`"
                ));
            }
        };
        if !suffix.is_empty() && !valid_application_id_segment(&suffix) {
            return Err(format!(
                "flavor `{name}` suffix must be a valid application ID segment (found `{suffix}`)"
            ));
        }
        if !suffix.is_empty() {
            let normalized = format!(".{suffix}");
            if !config.ios_bundle_identifier.ends_with(&normalized) {
                config.ios_bundle_identifier.push_str(&normalized);
            }
            if !config.android_application_id.ends_with(&normalized) {
                config.android_application_id.push_str(&normalized);
            }
        }
        if !config
            .display_name
            .to_ascii_lowercase()
            .ends_with(&format!(" {name}"))
        {
            config
                .display_name
                .push_str(&format!(" {}", title_case(name)));
        }
        Ok(config)
    }

    pub(super) fn parse_file(
        path: &Path,
        plugin_definitions: &[PluginDefinition],
        fallback_name: &str,
    ) -> Result<Self, String> {
        let source = fs::read_to_string(path)
            .map_err(|error| format!("cannot read config {}: {error}", path.display()))?;
        let config = nexa_syntax::parse_config(&source)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let mut permissions = Vec::with_capacity(config.permissions.len());
        for declaration in config.permissions {
            let permission = parse_permission(&declaration.name).ok_or_else(|| {
                format!(
                    "{}:{}:{}: unknown permission `{}`",
                    path.display(),
                    declaration.span.line,
                    declaration.span.column,
                    declaration.name
                )
            })?;
            if permissions
                .iter()
                .any(|(existing, _): &(Permission, String)| *existing == permission)
            {
                return Err(format!(
                    "{}:{}:{}: permission `{}` is declared more than once",
                    path.display(),
                    declaration.span.line,
                    declaration.span.column,
                    declaration.name
                ));
            }
            permissions.push((permission, declaration.message));
        }
        let plugins = resolve_plugins(path, &config.plugins, plugin_definitions)?;
        let flavors = config.flavors;
        let app = config.app;
        let ios = config.ios;
        let android = config.android;
        let cronet = android.as_ref().and_then(|android| android.cronet.as_ref());
        let android_cronet_provider = match cronet
            .and_then(|cronet| cronet.provider.as_deref())
            .unwrap_or("play-services")
        {
            "play-services" => AndroidCronetProvider::PlayServices,
            "embedded" => AndroidCronetProvider::Embedded,
            provider => {
                return Err(format!(
                    "{}: Android Cronet provider must be `play-services` or `embedded` (found `{provider}`)",
                    path.display()
                ));
            }
        };
        let android_cronet_disk_cache_size_mb = cronet
            .and_then(|cronet| cronet.disk_cache_size_mb)
            .unwrap_or(64);
        let assets = config.assets;
        let display_name = app
            .as_ref()
            .map(|app| app.display_name.clone())
            .unwrap_or_else(|| fallback_name.to_owned());
        let version = app
            .as_ref()
            .map(|app| app.version.clone())
            .unwrap_or_else(|| "1.0.0".to_owned());
        let build_number = app.as_ref().map(|app| app.build_number).unwrap_or(1);
        let staging_suffix = app
            .as_ref()
            .and_then(|app| app.staging_suffix.clone())
            .unwrap_or_else(|| "staging".to_owned());
        let deep_links = app
            .as_ref()
            .map(|app| app.deep_links.clone())
            .unwrap_or_default();
        let orientation = app
            .as_ref()
            .and_then(|app| app.orientation.clone())
            .unwrap_or_else(|| "all".to_owned());
        let ios_bundle_identifier = ios
            .as_ref()
            .and_then(|ios| ios.bundle_identifier.clone())
            .unwrap_or_else(|| format!("com.nexa.{}", fallback_name.to_ascii_lowercase()));
        let ios_app_group_identifier = ios
            .as_ref()
            .and_then(|ios| ios.app_group_identifier.clone())
            .filter(|identifier| !identifier.trim().is_empty());
        let android_application_id = android
            .as_ref()
            .and_then(|android| android.application_id.clone())
            .unwrap_or_else(|| format!("com.nexa.{}", fallback_name.to_ascii_lowercase()));
        let android_target_sdk = android
            .as_ref()
            .and_then(|android| android.target_sdk)
            .unwrap_or(36);
        let ios_min_version = ios
            .as_ref()
            .and_then(|ios| ios.min_version.clone())
            .unwrap_or_else(|| "16.0".to_owned());
        let ios_arch = ios
            .as_ref()
            .and_then(|ios| ios.arch.clone())
            .filter(|architectures| architectures.as_slice() != [""]);
        let android_min_sdk = android
            .as_ref()
            .and_then(|android| android.min_sdk)
            .unwrap_or(23);
        let android_arch = android
            .as_ref()
            .and_then(|android| android.arch.clone())
            .filter(|architectures| architectures.as_slice() != [""]);
        if display_name.trim().is_empty() {
            return Err(format!(
                "{}: app displayName cannot be empty",
                path.display()
            ));
        }
        if build_number == 0 {
            return Err(format!(
                "{}: app buildNumber must be greater than zero",
                path.display()
            ));
        }
        if !valid_application_id_segment(&staging_suffix) {
            return Err(format!(
                "{}: app stagingSuffix must be a valid application ID segment (found `{staging_suffix}`)",
                path.display()
            ));
        }
        if android_target_sdk != 36 {
            return Err(format!(
                "{}: Android targetSdk must remain 36 (found {android_target_sdk})",
                path.display()
            ));
        }
        validate_ios_deployment_version(path, &ios_min_version)?;
        if let Some(architectures) = &ios_arch {
            validate_ios_arches(architectures)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
        if let Some(architectures) = &android_arch {
            android_abis_for_arches(architectures)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
        if android_min_sdk == 0 || android_min_sdk > android_target_sdk {
            return Err(format!(
                "{}: Android minSdk must be between 1 and targetSdk ({android_target_sdk}) (found {android_min_sdk})",
                path.display()
            ));
        }
        validate_bundle_identifier(
            &path.display().to_string(),
            "iOS bundleIdentifier",
            &ios_bundle_identifier,
            true,
        )?;
        if let Some(group_identifier) = &ios_app_group_identifier {
            if !group_identifier.starts_with("group.") {
                return Err(format!(
                    "{}: `iOS appGroupIdentifier` must start with `group.` (found `{group_identifier}`)",
                    path.display()
                ));
            }
            validate_bundle_identifier(
                &path.display().to_string(),
                "iOS appGroupIdentifier",
                group_identifier,
                true,
            )?;
        }
        validate_bundle_identifier(
            &path.display().to_string(),
            "Android applicationId",
            &android_application_id,
            false,
        )?;
        validate_deep_links(path, &deep_links)?;
        let ios_alternate_icons = resolve_config_paths(
            path,
            ios.as_ref()
                .map(|ios| ios.alternate_icons.as_slice())
                .unwrap_or(&[]),
        )?;
        let android_alternate_icons = resolve_config_paths(
            path,
            android
                .as_ref()
                .map(|android| android.alternate_icons.as_slice())
                .unwrap_or(&[]),
        )?;
        let ios_icon = resolve_config_path(path, ios.and_then(|ios| ios.icon))?;
        let android_icon = resolve_config_path(path, android.and_then(|android| android.icon))?;
        let icon_source =
            resolve_config_path(path, assets.as_ref().and_then(|assets| assets.icon.clone()))?;
        validate_alternate_icons(path, "iOS", &ios_alternate_icons, "icon")?;
        validate_alternate_icons(path, "Android", &android_alternate_icons, "png")?;
        if !android_alternate_icons.is_empty() && android_icon.is_none() && icon_source.is_none() {
            return Err(format!(
                "{}: Android `alternateIcons` require a default Android `icon` or shared `assets.icon`",
                path.display()
            ));
        }
        Ok(Self {
            display_name,
            orientation,
            version,
            build_number,
            staging_suffix,
            flavors,
            deep_links,
            permissions,
            plugins,
            ios_min_version,
            ios_arch,
            android_min_sdk,
            android_arch,
            android_cronet_provider,
            android_cronet_disk_cache_size_mb,
            android_target_sdk,
            ios_bundle_identifier,
            ios_app_group_identifier,
            android_application_id,
            ios_alternate_icons,
            android_alternate_icons,
            ios_icon,
            android_icon,
            icon_source,
            splash_source: resolve_config_path(path, assets.and_then(|assets| assets.splash))?,
            dependencies: config.dependencies,
        })
    }

    pub(super) fn from_defaults(
        plugin_definitions: &[PluginDefinition],
        fallback_name: &str,
    ) -> Result<Self, String> {
        let plugins = resolve_plugins(Path::new("nexa.config.nx"), &[], plugin_definitions)?;
        Ok(Self {
            display_name: fallback_name.to_owned(),
            orientation: "all".to_owned(),
            version: "1.0.0".to_owned(),
            build_number: 1,
            staging_suffix: "staging".to_owned(),
            flavors: Vec::new(),
            deep_links: Vec::new(),
            permissions: Vec::new(),
            plugins,
            ios_min_version: "16.0".to_owned(),
            ios_arch: None,
            android_min_sdk: 23,
            android_arch: None,
            android_cronet_provider: AndroidCronetProvider::PlayServices,
            android_cronet_disk_cache_size_mb: 64,
            android_target_sdk: 36,
            ios_bundle_identifier: format!("com.nexa.{}", fallback_name.to_ascii_lowercase()),
            ios_app_group_identifier: None,
            android_application_id: format!("com.nexa.{}", fallback_name.to_ascii_lowercase()),
            ios_icon: None,
            android_icon: None,
            ios_alternate_icons: Vec::new(),
            android_alternate_icons: Vec::new(),
            dependencies: Vec::new(),
            icon_source: None,
            splash_source: None,
        })
    }

    /// The configuration `nexa create` starts from, with no plugins resolved.
    ///
    /// Exposed so `crate::docs` can read the real defaults into the published
    /// configuration reference instead of restating them; a default that changes
    /// here then reaches the documentation on the next regeneration.
    pub(super) fn scaffold_defaults(fallback_name: &str) -> Result<Self, String> {
        Self::from_defaults(&[], fallback_name)
    }

    pub(super) fn permissions(&self) -> impl Iterator<Item = &(Permission, String)> {
        self.permissions.iter()
    }

    pub(super) fn plugins(&self) -> impl Iterator<Item = &PluginConfig> {
        self.plugins.iter()
    }

    pub(super) fn validate_android_plugin_minimums(
        &self,
        requirements: &[(&str, Option<u32>)],
    ) -> Result<(), String> {
        for (namespace, required_min_sdk) in requirements {
            let Some(required_min_sdk) = required_min_sdk else {
                continue;
            };
            if *required_min_sdk > self.android_target_sdk {
                return Err(format!(
                    "Android plugin `{namespace}` requires minSdk {required_min_sdk}, above the app targetSdk {}. Nexa currently requires targetSdk 36, so this plugin cannot be used by this app.",
                    self.android_target_sdk
                ));
            }
            if *required_min_sdk > self.android_min_sdk {
                return Err(format!(
                    "Android plugin `{namespace}` requires minSdk {required_min_sdk}, but the app config sets android.minSdk to {}. Increase `android.minSdk` in `nexa.config.nx` to at least {required_min_sdk}.",
                    self.android_min_sdk
                ));
            }
        }
        Ok(())
    }

    pub(super) fn render(&self) -> String {
        let ios_arch = self
            .ios_arch
            .as_deref()
            .map(|architectures| format!(", arch: {}", render_string_array(architectures)))
            .unwrap_or_default();
        let android_arch = self
            .android_arch
            .as_deref()
            .map(|architectures| format!(", arch: {}", render_string_array(architectures)))
            .unwrap_or_default();
        let ios_app_group_identifier = self
            .ios_app_group_identifier
            .as_deref()
            .map(nexa_config_string)
            .unwrap_or_else(|| "\"\"".to_owned());
        let ios_alternate_icons = render_string_array(
            &self
                .ios_alternate_icons
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
        );
        let android_alternate_icons = render_string_array(
            &self
                .android_alternate_icons
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
        );
        let mut output = format!(
            "config {{\n    app {{ displayName: {}, version: {}, buildNumber: {}, stagingSuffix: {}, deepLinks: {}, orientation: {} }}\n    assets {{ icon: {}, splash: {} }}\n    ios {{ minVersion: {}, bundleIdentifier: {}, appGroupIdentifier: {}, icon: {}, alternateIcons: {}{ios_arch} }}\n    android {{ minSdk: {}, targetSdk: {}, applicationId: {}, icon: {}, alternateIcons: {}{android_arch}, cronet {{ provider: {}, diskCacheSizeMb: {} }} }}\n    permissions {{\n",
            nexa_config_string(&self.display_name),
            nexa_config_string(&self.version),
            self.build_number,
            nexa_config_string(&self.staging_suffix),
            render_string_array(&self.deep_links),
            nexa_config_string(&self.orientation),
            self.icon_source
                .as_ref()
                .map(|path| nexa_config_string(&path.display().to_string()))
                .unwrap_or_else(|| "\"\"".to_owned()),
            self.splash_source
                .as_ref()
                .map(|path| nexa_config_string(&path.display().to_string()))
                .unwrap_or_else(|| "\"\"".to_owned()),
            nexa_config_string(&self.ios_min_version),
            nexa_config_string(&self.ios_bundle_identifier),
            ios_app_group_identifier,
            self.ios_icon
                .as_ref()
                .map(|path| nexa_config_string(&path.display().to_string()))
                .unwrap_or_else(|| "\"\"".to_owned()),
            ios_alternate_icons,
            self.android_min_sdk,
            self.android_target_sdk,
            nexa_config_string(&self.android_application_id),
            self.android_icon
                .as_ref()
                .map(|path| nexa_config_string(&path.display().to_string()))
                .unwrap_or_else(|| "\"\"".to_owned()),
            android_alternate_icons,
            nexa_config_string(self.android_cronet_provider.config_value()),
            self.android_cronet_disk_cache_size_mb,
        );
        for (index, (permission, message)) in self.permissions.iter().enumerate() {
            output.push_str("        ");
            output.push_str(permission_name(*permission));
            output.push_str(": ");
            output.push_str(&nexa_config_string(message));
            if index + 1 < self.permissions.len() {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("    }\n");
        if !self.flavors.is_empty() {
            output.push_str("    flavors {\n");
            for flavor in &self.flavors {
                output.push_str(&format!("        {} {{", flavor.name));
                if let Some(suffix) = &flavor.suffix {
                    output.push_str(&format!(" suffix: {}", nexa_config_string(suffix)));
                }
                output.push_str(" }\n");
            }
            output.push_str("    }\n");
        }
        if !self.dependencies.is_empty() {
            output.push_str("    dependencies {\n");
            for dependency in &self.dependencies {
                output.push_str(&format!(
                    "        {} {{ id: {}{}{}{}{} }}\n",
                    dependency.alias,
                    nexa_config_string(&dependency.package_id),
                    dependency
                        .path
                        .as_ref()
                        .map(|value| format!(", path: {}", nexa_config_string(value)))
                        .unwrap_or_default(),
                    dependency
                        .git
                        .as_ref()
                        .map(|value| format!(", git: {}", nexa_config_string(value)))
                        .unwrap_or_default(),
                    dependency
                        .revision
                        .as_ref()
                        .map(|value| format!(", rev: {}", nexa_config_string(value)))
                        .unwrap_or_default(),
                    dependency
                        .package_path
                        .as_ref()
                        .map(|value| format!(", package: {}", nexa_config_string(value)))
                        .unwrap_or_default(),
                ));
            }
            output.push_str("    }\n");
        }
        if !self.plugins.is_empty() {
            output.push_str("    plugins {\n");
            for plugin in &self.plugins {
                output.push_str("        ");
                output.push_str(&plugin.namespace);
                output.push_str(" {\n");
                for (index, option) in plugin.options.iter().enumerate() {
                    output.push_str("            ");
                    output.push_str(&option.name);
                    output.push_str(": ");
                    output.push_str(&render_config_value(&option.value));
                    if index + 1 < plugin.options.len() {
                        output.push(',');
                    }
                    output.push('\n');
                }
                output.push_str("        }\n");
            }
            output.push_str("    }\n");
        }
        output.push_str("}\n");
        output
    }
}

pub(super) fn validate_ios_arch(arch: &str) -> Result<(), String> {
    match arch {
        "arm64" | "x86_64" => Ok(()),
        _ => Err(format!(
            "unsupported iOS architecture `{arch}`; expected `arm64` or `x86_64`"
        )),
    }
}

pub(super) fn validate_ios_arches(architectures: &[String]) -> Result<(), String> {
    if architectures.is_empty() {
        return Err("iOS `arch` must contain at least one architecture".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for arch in architectures {
        validate_ios_arch(arch)?;
        if !seen.insert(arch) {
            return Err(format!(
                "iOS architecture `{arch}` is listed more than once"
            ));
        }
    }
    Ok(())
}

pub(super) fn android_abi_for_arch(arch: &str) -> Result<&'static str, String> {
    match arch {
        "arm64" | "arm64-v8a" => Ok("arm64-v8a"),
        "armv7" | "armeabi-v7a" => Ok("armeabi-v7a"),
        "x86" => Ok("x86"),
        "x86_64" => Ok("x86_64"),
        _ => Err(format!(
            "unsupported Android architecture `{arch}`; expected `arm64`, `armv7`, `x86`, or `x86_64`"
        )),
    }
}

pub(super) fn android_abis_for_arches(
    architectures: &[String],
) -> Result<Vec<&'static str>, String> {
    if architectures.is_empty() {
        return Err("Android `arch` must contain at least one architecture".to_owned());
    }
    let mut abis = Vec::with_capacity(architectures.len());
    for arch in architectures {
        let abi = android_abi_for_arch(arch)?;
        if abis.contains(&abi) {
            return Err(format!(
                "Android ABI `{abi}` is listed more than once through `arch`"
            ));
        }
        abis.push(abi);
    }
    Ok(abis)
}

fn render_string_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| nexa_config_string(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn validate_deep_links(config_path: &Path, values: &[String]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(format!(
                "{}: app deepLinks contains duplicate URL base `{value}`",
                config_path.display()
            ));
        }
        if let Some(scheme) = value.strip_suffix("://") {
            let valid = scheme
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
                && scheme
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'));
            if valid {
                continue;
            }
        }
        if let Some(host) = value.strip_prefix("https://")
            && !host.is_empty()
            && !host.contains(['/', '?', '#', '@', ':'])
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.as_bytes()[0].is_ascii_alphanumeric()
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    && !label.ends_with('-')
            })
        {
            continue;
        }
        return Err(format!(
            "{}: app deepLinks entries must be a custom URL scheme ending in `://` or an HTTPS origin (found `{value}`)",
            config_path.display()
        ));
    }
    Ok(())
}

fn valid_application_id_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.as_bytes()[0].is_ascii_alphabetic()
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn validate_ios_deployment_version(config_path: &Path, version: &str) -> Result<(), String> {
    let parts = version.split('.').collect::<Vec<_>>();
    let valid = (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        && parts.iter().all(|part| part.parse::<u32>().is_ok());
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{}: iOS minVersion must be a numeric version with two or three components (found `{version}`)",
            config_path.display()
        ))
    }
}

fn title_case(value: &str) -> String {
    let mut characters = value.chars();
    characters
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
        .unwrap_or_default()
}

fn validate_bundle_identifier(
    config_path: &str,
    field: &str,
    id: &str,
    allow_hyphen: bool,
) -> Result<(), String> {
    let valid = id.split('.').count() >= 2
        && id.split('.').all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_alphabetic()
                && part.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || byte == b'_' || (allow_hyphen && byte == b'-')
                })
        });
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{config_path}: `{field}` must be a reverse-DNS identifier with at least two valid segments (found `{id}`)"
        ))
    }
}

fn resolve_config_path(
    config_path: &Path,
    path: Option<String>,
) -> Result<Option<PathBuf>, String> {
    let Some(path) = path.filter(|path| !path.is_empty()) else {
        return Ok(None);
    };
    let path = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(path);
    let canonical =
        fs::canonicalize(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(Some(canonical))
}

fn resolve_config_paths(config_path: &Path, paths: &[String]) -> Result<Vec<PathBuf>, String> {
    paths
        .iter()
        .map(|path| {
            resolve_config_path(config_path, Some(path.clone()))?.ok_or_else(|| {
                format!(
                    "{}: alternate icon path cannot be empty",
                    config_path.display()
                )
            })
        })
        .collect()
}

fn validate_alternate_icons(
    config_path: &Path,
    platform: &str,
    icons: &[PathBuf],
    extension: &str,
) -> Result<(), String> {
    let mut names = HashSet::new();
    for icon in icons {
        let name = icon
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                format!(
                    "{}: invalid {platform} alternate icon path",
                    config_path.display()
                )
            })?;
        let valid_name = name.chars().enumerate().all(|(index, character)| {
            character.is_ascii_alphanumeric() || (index > 0 && character == '_')
        }) && name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
        if !valid_name {
            return Err(format!(
                "{}: {platform} alternate icon name `{name}` must start with a letter and contain only ASCII letters, numbers, or underscores",
                config_path.display()
            ));
        }
        let resource_name = name.to_ascii_lowercase();
        if !names.insert(resource_name.clone()) {
            return Err(format!(
                "{}: duplicate {platform} alternate icon resource name `{name}` (resource names are case-insensitive)",
                config_path.display()
            ));
        }
        if platform == "Android"
            && matches!(
                resource_name.as_str(),
                "default" | "ic_launcher" | "ic_launcher_round"
            )
        {
            return Err(format!(
                "{}: Android alternate icon name `{name}` is reserved for the default launcher icon",
                config_path.display()
            ));
        }
        let valid_kind = if extension == "icon" {
            icon.extension().and_then(|value| value.to_str()) == Some("icon") && icon.is_dir()
        } else {
            // Android alternates must provide both adaptive layers instead of
            // silently degrading to a flat density PNG.
            icon.is_dir()
                && icon.join("icon.png").is_file()
                && has_android_icon_layer(icon, "foreground")
                && has_android_icon_layer(icon, "background")
        };
        if !valid_kind {
            return Err(format!(
                "{}: invalid {platform} alternate icon asset at `{}`",
                config_path.display(),
                icon.display()
            ));
        }
    }
    Ok(())
}

fn has_android_icon_layer(icon_set: &Path, name: &str) -> bool {
    ["png", "xml"].into_iter().any(|extension| {
        icon_set.join(format!("{name}.{extension}")).is_file()
            || icon_set.parent().is_some_and(|parent| {
                parent
                    .join("shared")
                    .join(format!("{name}.{extension}"))
                    .is_file()
            })
    })
}

pub(super) fn load_plugin_dependencies(path: &Path) -> Result<Vec<PluginDependencyConfig>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read config {}: {error}", path.display()))?;
    nexa_syntax::parse_config(&source)
        .map(|config| config.dependencies)
        .map_err(|error| format!("{}: {error}", path.display()))
}

pub(super) fn load_plugin_definitions(
    entry: &Path,
    plugin_roots: &std::collections::HashMap<String, PathBuf>,
) -> Result<Vec<PluginDefinition>, String> {
    let base = entry.parent().unwrap_or_else(|| Path::new("."));
    let declarations = collect_plugin_declarations(entry, plugin_roots)?;
    let mut definitions = Vec::with_capacity(declarations.len());
    let mut declared_contracts = HashMap::<String, PathBuf>::new();
    for (plugin, source_file) in declarations {
        let source_base = source_file.parent().unwrap_or_else(|| Path::new("."));
        let declared_path = plugin_roots
            .get(&plugin.path)
            .cloned()
            .unwrap_or_else(|| source_base.join(&plugin.path));
        let idl_path = if declared_path.is_dir() {
            let manifest_path = declared_path.join("plugin.config.nx");
            let manifest =
                nexa_plugin_idl::manifest::parse_file(&manifest_path).map_err(|error| {
                    format!(
                        "{}:{}:{}: {error}",
                        source_file.display(),
                        plugin.span.line,
                        plugin.span.column
                    )
                })?;
            if manifest.nexa.is_some() && manifest.native.is_none() {
                continue;
            }
            let native = manifest.native.ok_or_else(|| {
                format!(
                    "{}:{}:{}: plugin manifest does not declare `sources.native`",
                    source_file.display(),
                    plugin.span.line,
                    plugin.span.column
                )
            })?;
            declared_path.join(native)
        } else {
            declared_path
        };
        let idl_path = fs::canonicalize(&idl_path).map_err(|error| {
            format!(
                "{}:{}:{}: cannot resolve plugin IDL `{}`: {error}",
                source_file.display(),
                plugin.span.line,
                plugin.span.column,
                plugin.path
            )
        })?;
        let idl = nexa_plugin_idl::parse_file(&idl_path).map_err(|error| {
            format!(
                "{}:{}:{}: {error}",
                source_file.display(),
                plugin.span.line,
                plugin.span.column
            )
        })?;
        if let Some(previous_path) = declared_contracts.get(&plugin.namespace) {
            if previous_path != &idl_path {
                return Err(format!(
                    "{}:{}:{}: plugin namespace `{}` refers to more than one package",
                    source_file.display(),
                    plugin.span.line,
                    plugin.span.column,
                    plugin.namespace
                ));
            }
            continue;
        }
        declared_contracts.insert(plugin.namespace.clone(), idl_path);
        definitions.push(PluginDefinition {
            namespace: plugin.namespace,
            idl,
        });
    }

    // Dependencies in nexa.config.nx are compiled into Dev hosts up front so
    // source added by a later hot reload can import them. Load their contracts
    // here as well, using the dependency alias as the prelinked Dev namespace.
    let config_path = base.join("nexa.config.nx");
    for dependency in load_plugin_dependencies(&config_path)? {
        if definitions
            .iter()
            .any(|definition| definition.namespace == dependency.alias)
        {
            continue;
        }
        let Some(package_root) = plugin_roots.get(&dependency.package_id) else {
            return Err(format!(
                "{}: configured plugin `{}` was not resolved to a package root",
                config_path.display(),
                dependency.alias
            ));
        };
        let manifest_path = package_root.join("plugin.config.nx");
        let manifest = nexa_plugin_idl::manifest::parse_file(&manifest_path)?;
        let Some(native) = manifest.native else {
            continue;
        };
        let idl_path = fs::canonicalize(package_root.join(native)).map_err(|error| {
            format!(
                "{}: cannot resolve native plugin contract: {error}",
                manifest_path.display()
            )
        })?;
        let idl = nexa_plugin_idl::parse_file(&idl_path)?;
        definitions.push(PluginDefinition {
            namespace: dependency.alias,
            idl,
        });
    }
    Ok(definitions)
}

fn collect_plugin_declarations(
    entry: &Path,
    plugin_roots: &HashMap<String, PathBuf>,
) -> Result<Vec<(PluginDecl, PathBuf)>, String> {
    fn visit(
        path: &Path,
        plugin_roots: &HashMap<String, PathBuf>,
        visited: &mut HashSet<PathBuf>,
        declarations: &mut Vec<(PluginDecl, PathBuf)>,
    ) -> Result<(), String> {
        let canonical = fs::canonicalize(path)
            .map_err(|error| format!("cannot resolve source {}: {error}", path.display()))?;
        if !visited.insert(canonical.clone()) {
            return Ok(());
        }
        let source = fs::read_to_string(&canonical)
            .map_err(|error| format!("cannot read source {}: {error}", canonical.display()))?;
        let program = nexa_syntax::parse_program(&source)
            .map_err(|error| format!("{}: {error}", canonical.display()))?;
        let base = canonical.parent().unwrap_or_else(|| Path::new("."));
        let plugins = program.plugins;
        declarations.extend(
            plugins
                .iter()
                .cloned()
                .map(|plugin| (plugin, canonical.clone())),
        );
        for import in program.imports {
            visit(&base.join(import.path), plugin_roots, visited, declarations)?;
        }

        // A plugin's bundled Nexa source is outside the app's ordinary import
        // list. Traverse it here too, matching the compiler's project loader.
        for plugin in plugins {
            let root = plugin_roots
                .get(&plugin.path)
                .cloned()
                .unwrap_or_else(|| base.join(&plugin.path));
            if !root.is_dir() {
                continue;
            }
            let manifest = nexa_plugin_idl::manifest::parse_file(&root.join("plugin.config.nx"))?;
            if let Some(source) = manifest.nexa {
                visit(&root.join(source), plugin_roots, visited, declarations)?;
            }
        }
        Ok(())
    }

    let mut declarations = Vec::new();
    visit(entry, plugin_roots, &mut HashSet::new(), &mut declarations)?;
    Ok(declarations)
}

pub(super) fn render_template(plugin_definitions: &[PluginDefinition]) -> String {
    let mut output = String::from(
        "config {\n    app { stagingSuffix: \"staging\", orientation: \"all\" }\n    ios { minVersion: \"16.0\" }\n    android { minSdk: 23, targetSdk: 36, cronet { provider: \"play-services\", diskCacheSizeMb: 64 } }\n    permissions {}\n",
    );
    let configured_plugins = plugin_definitions
        .iter()
        .filter(|definition| !definition.idl.config.is_empty())
        .collect::<Vec<_>>();
    if !configured_plugins.is_empty() {
        output.push_str("    plugins {\n");
    }
    for definition in configured_plugins {
        output.push_str("        ");
        output.push_str(&definition.namespace);
        output.push_str(" {\n");
        for option in &definition.idl.config {
            if let Some(default) = &option.default {
                output.push_str("            ");
                output.push_str(&option.name);
                output.push_str(": ");
                output.push_str(&render_config_value(&literal_to_config_value(default)));
                output.push('\n');
            } else if option.ty.optional {
                output.push_str("            ");
                output.push_str(&option.name);
                output.push_str(": null\n");
            } else {
                output.push_str("            // ");
                output.push_str(&option.name);
                output.push_str(": <required ");
                output.push_str(&option.ty.name);
                output.push_str(">\n");
            }
        }
        output.push_str("        }\n");
    }
    if !plugin_definitions
        .iter()
        .all(|definition| definition.idl.config.is_empty())
    {
        output.push_str("    }\n");
    }
    output.push_str("}\n");
    output
}

fn resolve_plugins(
    path: &Path,
    declarations: &[nexa_syntax::ast::PluginConfigDecl],
    definitions: &[PluginDefinition],
) -> Result<Vec<PluginConfig>, String> {
    let mut resolved = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let declaration = declarations
            .iter()
            .find(|declaration| declaration.name == definition.namespace);
        let options = resolve_options(path, definition, declaration)?;
        resolved.push(PluginConfig {
            namespace: definition.namespace.clone(),
            options,
        });
    }
    for declaration in declarations {
        if !definitions
            .iter()
            .any(|definition| definition.namespace == declaration.name)
        {
            return Err(format!(
                "{}:{}:{}: config references undeclared plugin `{}`",
                path.display(),
                declaration.span.line,
                declaration.span.column,
                declaration.name
            ));
        }
    }
    Ok(resolved)
}

fn resolve_options(
    path: &Path,
    definition: &PluginDefinition,
    declaration: Option<&nexa_syntax::ast::PluginConfigDecl>,
) -> Result<Vec<ResolvedPluginOption>, String> {
    let declarations = declaration.map(|declaration| declaration.options.as_slice());
    let mut options = Vec::with_capacity(definition.idl.config.len());
    for schema in &definition.idl.config {
        let provided = declarations
            .and_then(|options| options.iter().find(|option| option.name == schema.name));
        let value = match provided {
            Some(option) => {
                validate_value(
                    path,
                    &definition.namespace,
                    schema,
                    &option.value,
                    option.span.line,
                    option.span.column,
                )?;
                option.value.clone()
            }
            None => schema
                .default
                .as_ref()
                .map(literal_to_config_value)
                .or_else(|| schema.ty.optional.then_some(ConfigValue::Null))
                .ok_or_else(|| {
                    format!(
                        "{}: plugin `{}` requires config option `{}`",
                        path.display(),
                        definition.namespace,
                        schema.name
                    )
                })?,
        };
        options.push(ResolvedPluginOption {
            name: schema.name.clone(),
            ty: schema.ty.clone(),
            value,
        });
    }
    if let Some(declarations) = declarations {
        for option in declarations {
            if !definition
                .idl
                .config
                .iter()
                .any(|schema| schema.name == option.name)
            {
                return Err(format!(
                    "{}:{}:{}: plugin `{}` does not declare config option `{}`",
                    path.display(),
                    option.span.line,
                    option.span.column,
                    definition.namespace,
                    option.name
                ));
            }
        }
    }
    Ok(options)
}

fn validate_value(
    path: &Path,
    namespace: &str,
    schema: &ConfigOption,
    value: &ConfigValue,
    line: usize,
    column: usize,
) -> Result<(), String> {
    let valid = match (value, schema.ty.name.as_str()) {
        (ConfigValue::String(_), "String") => true,
        (ConfigValue::Bool(_), "Bool") => true,
        (ConfigValue::Number(raw), name) if integer_type(name) => integer_value_fits(raw, name),
        (ConfigValue::Number(raw), name) if float_type(name) => match name {
            "Float32" => raw.parse::<f32>().is_ok(),
            "Float64" => raw.parse::<f64>().is_ok(),
            _ => false,
        },
        (ConfigValue::Null, _) => schema.ty.optional,
        _ => false,
    };
    if valid {
        return Ok(());
    }
    Err(format!(
        "{}:{}:{}: value for plugin `{}` option `{}` does not match `{}`",
        path.display(),
        line,
        column,
        namespace,
        schema.name,
        schema.ty.name
    ))
}

fn literal_to_config_value(literal: &Literal) -> ConfigValue {
    match literal {
        Literal::String(value) => ConfigValue::String(value.clone()),
        Literal::Number(value) => ConfigValue::Number(value.clone()),
        Literal::Bool(value) => ConfigValue::Bool(*value),
        Literal::Null => ConfigValue::Null,
    }
}

fn render_config_value(value: &ConfigValue) -> String {
    match value {
        ConfigValue::String(value) => nexa_config_string(value),
        ConfigValue::Number(value) => value.clone(),
        ConfigValue::Bool(value) => value.to_string(),
        ConfigValue::Null => "null".to_owned(),
        ConfigValue::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(render_config_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn integer_type(name: &str) -> bool {
    matches!(
        name,
        "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32" | "UInt64"
    )
}

fn float_type(name: &str) -> bool {
    matches!(name, "Float32" | "Float64")
}

fn integer_value_fits(raw: &str, name: &str) -> bool {
    if raw.contains('.') {
        return false;
    }
    match name {
        "Int8" => raw.parse::<i8>().is_ok(),
        "Int16" => raw.parse::<i16>().is_ok(),
        "Int32" => raw.parse::<i32>().is_ok(),
        "Int64" => raw.parse::<i64>().is_ok(),
        "UInt8" => raw.parse::<u8>().is_ok(),
        "UInt16" => raw.parse::<u16>().is_ok(),
        "UInt32" => raw.parse::<u32>().is_ok(),
        "UInt64" => raw.parse::<u64>().is_ok(),
        _ => false,
    }
}

pub(super) fn parse_permission(name: &str) -> Option<Permission> {
    match name.to_ascii_lowercase().as_str() {
        "camera" => Some(Permission::Camera),
        "microphone" => Some(Permission::Microphone),
        "photos" => Some(Permission::Photos),
        "location" => Some(Permission::Location),
        "notifications" => Some(Permission::Notifications),
        "contacts" => Some(Permission::Contacts),
        "calendar" => Some(Permission::Calendar),
        "bluetooth" => Some(Permission::Bluetooth),
        "motion" => Some(Permission::Motion),
        _ => None,
    }
}

fn permission_name(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => "camera",
        Permission::Microphone => "microphone",
        Permission::Photos => "photos",
        Permission::Location => "location",
        Permission::Notifications => "notifications",
        Permission::Contacts => "contacts",
        Permission::Calendar => "calendar",
        Permission::Bluetooth => "bluetooth",
        Permission::Motion => "motion",
    }
}

fn nexa_config_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character => output.push(character),
        }
    }
    output.push('"');
    output
}
