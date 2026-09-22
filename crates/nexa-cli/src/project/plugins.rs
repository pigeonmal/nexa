//! Optional plugin source copying and compile-time configuration emission.

use std::{
    fs,
    path::{Path, PathBuf},
};

use nexa_ir::Module;
use nexa_syntax::ast::ConfigValue;

use super::{ProjectConfig, write_if_changed};

pub(super) fn copy_android_plugin_sources(
    root: &Path,
    module: &Module,
    app_package: &str,
    config: &ProjectConfig,
) -> Result<(), String> {
    let destination = root.join("android/app/src/main/java");
    let has_plugin_config = config.plugins().any(|plugin| {
        !plugin.options.is_empty()
            && module
                .plugins
                .iter()
                .any(|used| used.namespace == plugin.namespace)
    });
    for plugin in &module.plugins {
        for path in native_plugin_sources(plugin, "android/src/main/kotlin", "kt")? {
            let plugin_root = Path::new(&plugin.idl_path)
                .parent()
                .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
            let source_root = plugin_root.join("android/src/main/kotlin");
            let relative = path.strip_prefix(&source_root).map_err(|_| {
                format!(
                    "plugin source is outside its Kotlin source root: {}",
                    path.display()
                )
            })?;
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            let contents = if has_plugin_config {
                kotlin_plugin_config_import(&contents, app_package)
            } else {
                contents
            };
            write_if_changed(&destination.join(relative), &contents)?;
        }
    }
    Ok(())
}

pub(super) fn render_swift_plugin_config(module: &Module, config: &ProjectConfig) -> String {
    let plugins = config
        .plugins()
        .filter(|plugin| {
            !plugin.options.is_empty()
                && module
                    .plugins
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

pub(super) fn render_kotlin_plugin_config(module: &Module, config: &ProjectConfig) -> String {
    let plugins = config
        .plugins()
        .filter(|plugin| {
            !plugin.options.is_empty()
                && module
                    .plugins
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
    plugin: &nexa_ir::Plugin,
    relative_root: &str,
    extension: &str,
) -> Result<Vec<PathBuf>, String> {
    let plugin_root = Path::new(&plugin.idl_path)
        .parent()
        .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
    let source_root = plugin_root.join(relative_root);
    if !source_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files(&source_root, extension, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(
    directory: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, extension, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            files.push(path);
        }
    }
    Ok(())
}
