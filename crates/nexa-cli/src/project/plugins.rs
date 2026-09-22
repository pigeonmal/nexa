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
    let marker = root.join("android/app/.nexa-plugin-sources");
    let mut generated = Vec::new();
    let has_plugin_config = config.plugins().any(|plugin| {
        !plugin.options.is_empty()
            && module
                .plugins
                .iter()
                .any(|used| used.namespace == plugin.namespace)
    });
    for plugin in &module.plugins {
        for (path, source_root) in native_plugin_sources(plugin, "android/src/main/kotlin", "kt")? {
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
            generated.push(relative.to_string_lossy().into_owned());
        }
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
    Ok(())
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
        let mut files = Vec::new();
        collect_files(source_root, "", &mut files)?;
        for source in files {
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
    module: &Module,
) -> Result<Vec<String>, String> {
    let destination = root.join("ios").join(app_name).join("NexaPlugins");
    let marker = root.join("ios").join(app_name).join(".nexa-plugin-sources");
    let mut names = Vec::new();
    for (plugin_index, plugin) in module.plugins.iter().enumerate() {
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
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let plugin_root = Path::new(&plugin.idl_path)
        .parent()
        .ok_or_else(|| format!("invalid plugin IDL path `{}`", plugin.idl_path))?;
    let patterns = if relative_root.starts_with("ios/") {
        &plugin.ios_sources
    } else {
        &plugin.android_sources
    };
    let fallback = plugin_root.join(relative_root);
    let mut roots = if patterns.is_empty() {
        vec![fallback]
    } else {
        patterns.iter().map(PathBuf::from).collect::<Vec<_>>()
    };
    roots.sort();
    roots.dedup();

    let mut files = Vec::new();
    for pattern in roots {
        let (root, explicit_file) = if pattern.is_file() {
            (
                pattern
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| plugin_root.to_path_buf()),
                Some(pattern),
            )
        } else {
            let pattern_string = pattern.to_string_lossy();
            let root = pattern_string
                .strip_suffix("/**")
                .map(PathBuf::from)
                .unwrap_or_else(|| pattern.clone());
            (root, None)
        };

        if let Some(file) = explicit_file {
            if file
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value == extension)
            {
                files.push((file, root));
            }
        } else if root.is_dir() {
            let mut matching = Vec::new();
            collect_files(&root, extension, &mut matching)?;
            files.extend(matching.into_iter().map(|file| (file, root.clone())));
        }
    }

    files.sort_by(|left, right| left.0.cmp(&right.0));
    files.dedup_by(|left, right| left.0 == right.0);
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
        } else if extension.is_empty()
            || path.extension().and_then(|value| value.to_str()) == Some(extension)
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
