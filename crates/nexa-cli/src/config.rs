use std::{fs, path::Path};

use nexa_ir::Permission;
use nexa_plugin_idl::{ConfigOption, Literal, PluginIdl, TypeRef};
use nexa_syntax::ast::ConfigValue;

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
    permissions: Vec<(Permission, String)>,
    plugins: Vec<PluginConfig>,
}

impl ProjectConfig {
    pub(super) fn parse_file(
        path: &Path,
        plugin_definitions: &[PluginDefinition],
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
        Ok(Self {
            permissions,
            plugins,
        })
    }

    pub(super) fn from_defaults(plugin_definitions: &[PluginDefinition]) -> Result<Self, String> {
        let plugins = resolve_plugins(Path::new("nexa.config.nx"), &[], plugin_definitions)?;
        Ok(Self {
            permissions: Vec::new(),
            plugins,
        })
    }

    pub(super) fn permissions(&self) -> impl Iterator<Item = &(Permission, String)> {
        self.permissions.iter()
    }

    pub(super) fn plugins(&self) -> impl Iterator<Item = &PluginConfig> {
        self.plugins.iter()
    }

    pub(super) fn render(&self) -> String {
        let mut output = String::from("config {\n    permissions {\n");
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

pub(super) fn load_plugin_definitions(entry: &Path) -> Result<Vec<PluginDefinition>, String> {
    let source = fs::read_to_string(entry)
        .map_err(|error| format!("cannot read entry {}: {error}", entry.display()))?;
    let program = nexa_syntax::parse_program(&source)
        .map_err(|error| format!("{}: {error}", entry.display()))?;
    let base = entry.parent().unwrap_or_else(|| Path::new("."));
    let mut definitions = Vec::with_capacity(program.plugins.len());
    for plugin in program.plugins {
        let declared_path = base.join(&plugin.path);
        let idl_path = if declared_path.is_dir() {
            declared_path.join("interfaces.nxid")
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
    let mut output = String::from("config {\n    permissions {}\n");
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
