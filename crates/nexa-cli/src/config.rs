use std::{
    fs,
    path::{Path, PathBuf},
};

use nexa_ir::Permission;
use nexa_plugin_idl::{ConfigOption, Literal, PluginIdl, TypeRef};
use nexa_syntax::ast::{ConfigValue, PluginDependencyConfig};

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
    pub(super) version: String,
    pub(super) build_number: u32,
    pub(super) staging_suffix: String,
    pub(super) flavors: Vec<nexa_syntax::ast::FlavorConfig>,
    pub(super) deep_links: Vec<String>,
    permissions: Vec<(Permission, String)>,
    plugins: Vec<PluginConfig>,
    pub(super) ios_min_version: String,
    pub(super) android_min_sdk: u32,
    pub(super) android_target_sdk: u32,
    pub(super) ios_bundle_identifier: String,
    pub(super) android_application_id: String,
    pub(super) ios_icon: Option<PathBuf>,
    pub(super) android_icon: Option<PathBuf>,
    pub(super) dependencies: Vec<PluginDependencyConfig>,
    pub(super) icon_source: Option<PathBuf>,
    pub(super) splash_source: Option<PathBuf>,
}

impl ProjectConfig {
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
        let ios_bundle_identifier = ios
            .as_ref()
            .and_then(|ios| ios.bundle_identifier.clone())
            .unwrap_or_else(|| format!("com.nexa.{}", fallback_name.to_ascii_lowercase()));
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
        let android_min_sdk = android
            .as_ref()
            .and_then(|android| android.min_sdk)
            .unwrap_or(24);
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
        validate_bundle_identifier(
            &path.display().to_string(),
            "Android applicationId",
            &android_application_id,
            false,
        )?;
        validate_deep_links(path, &deep_links)?;
        Ok(Self {
            display_name,
            version,
            build_number,
            staging_suffix,
            flavors,
            deep_links,
            permissions,
            plugins,
            ios_min_version,
            android_min_sdk,
            android_target_sdk,
            ios_bundle_identifier,
            android_application_id,
            ios_icon: resolve_config_path(path, ios.and_then(|ios| ios.icon))?,
            android_icon: resolve_config_path(path, android.and_then(|android| android.icon))?,
            icon_source: resolve_config_path(
                path,
                assets.as_ref().and_then(|assets| assets.icon.clone()),
            )?,
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
            version: "1.0.0".to_owned(),
            build_number: 1,
            staging_suffix: "staging".to_owned(),
            flavors: Vec::new(),
            deep_links: Vec::new(),
            permissions: Vec::new(),
            plugins,
            ios_min_version: "16.0".to_owned(),
            android_min_sdk: 24,
            android_target_sdk: 36,
            ios_bundle_identifier: format!("com.nexa.{}", fallback_name.to_ascii_lowercase()),
            android_application_id: format!("com.nexa.{}", fallback_name.to_ascii_lowercase()),
            ios_icon: None,
            android_icon: None,
            dependencies: Vec::new(),
            icon_source: None,
            splash_source: None,
        })
    }

    pub(super) fn permissions(&self) -> impl Iterator<Item = &(Permission, String)> {
        self.permissions.iter()
    }

    pub(super) fn plugins(&self) -> impl Iterator<Item = &PluginConfig> {
        self.plugins.iter()
    }

    pub(super) fn render(&self) -> String {
        let mut output = format!(
            "config {{\n    app {{ displayName: {}, version: {}, buildNumber: {}, stagingSuffix: {}, deepLinks: {} }}\n    assets {{ icon: {}, splash: {} }}\n    ios {{ minVersion: {}, bundleIdentifier: {}, icon: {} }}\n    android {{ minSdk: {}, targetSdk: {}, applicationId: {}, icon: {} }}\n    permissions {{\n",
            nexa_config_string(&self.display_name),
            nexa_config_string(&self.version),
            self.build_number,
            nexa_config_string(&self.staging_suffix),
            render_string_array(&self.deep_links),
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
            self.ios_icon
                .as_ref()
                .map(|path| nexa_config_string(&path.display().to_string()))
                .unwrap_or_else(|| "\"\"".to_owned()),
            self.android_min_sdk,
            self.android_target_sdk,
            nexa_config_string(&self.android_application_id),
            self.android_icon
                .as_ref()
                .map(|path| nexa_config_string(&path.display().to_string()))
                .unwrap_or_else(|| "\"\"".to_owned())
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
    let source = fs::read_to_string(entry)
        .map_err(|error| format!("cannot read entry {}: {error}", entry.display()))?;
    let program = nexa_syntax::parse_program(&source)
        .map_err(|error| format!("{}: {error}", entry.display()))?;
    let base = entry.parent().unwrap_or_else(|| Path::new("."));
    let mut definitions = Vec::with_capacity(program.plugins.len());
    for plugin in program.plugins {
        let declared_path = plugin_roots
            .get(&plugin.path)
            .cloned()
            .unwrap_or_else(|| base.join(&plugin.path));
        if declared_path.is_dir()
            && nexa_plugin_idl::manifest::parse_file(&declared_path.join("plugin.config.nx"))
                .map(|manifest| manifest.nexa.is_some())
                .unwrap_or(false)
        {
            continue;
        }
        let idl_path = if declared_path.is_dir() {
            let manifest_path = declared_path.join("plugin.config.nx");
            let manifest =
                nexa_plugin_idl::manifest::parse_file(&manifest_path).map_err(|error| {
                    format!(
                        "{}:{}:{}: {error}",
                        entry.display(),
                        plugin.span.line,
                        plugin.span.column
                    )
                })?;
            let native = manifest.native.ok_or_else(|| {
                format!(
                    "{}:{}:{}: plugin manifest does not declare `sources.native`",
                    entry.display(),
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
                entry.display(),
                plugin.span.line,
                plugin.span.column,
                plugin.path
            )
        })?;
        let idl = nexa_plugin_idl::parse_file(&idl_path).map_err(|error| {
            format!(
                "{}:{}:{}: {error}",
                entry.display(),
                plugin.span.line,
                plugin.span.column
            )
        })?;
        if definitions
            .iter()
            .any(|definition: &PluginDefinition| definition.namespace == plugin.namespace)
        {
            return Err(format!(
                "{}:{}:{}: plugin namespace `{}` is declared more than once",
                entry.display(),
                plugin.span.line,
                plugin.span.column,
                plugin.namespace
            ));
        }
        definitions.push(PluginDefinition {
            namespace: plugin.namespace,
            idl,
        });
    }
    Ok(definitions)
}

pub(super) fn render_template(plugin_definitions: &[PluginDefinition]) -> String {
    let mut output = String::from(
        "config {\n    app { stagingSuffix: \"staging\" }\n    ios { minVersion: \"16.0\" }\n    android { minSdk: 24, targetSdk: 36 }\n    permissions {}\n",
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
