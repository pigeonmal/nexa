//! Optional plugin scaffolding.
//!
//! This module only creates the stable package shape that a future typed IDL
//! and binding generator will consume. It deliberately does not modify the
//! core Nexa compiler or add plugin dependencies to generated applications.

use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(usage());
    };
    match command {
        "init" => init(&args[1..]),
        _ => Err(format!("unknown plugin command `{command}`\n\n{}", usage())),
    }
}

fn init(args: &[String]) -> Result<(), String> {
    let mut id = None;
    let mut output = None;
    let mut display_name = None;
    let mut version = "0.1.0".to_owned();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--out" | "--output" | "-o" => {
                cursor += 1;
                output = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a directory")?,
                ));
            }
            "--name" | "-n" => {
                cursor += 1;
                display_name = Some(
                    args.get(cursor)
                        .ok_or("`--name` requires a plugin type name")?
                        .clone(),
                );
            }
            "--version" => {
                cursor += 1;
                version = args
                    .get(cursor)
                    .ok_or("`--version` requires a version")?
                    .clone();
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if id.is_none() => id = Some(value.to_owned()),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }

    let id = id.ok_or_else(usage)?;
    validate_id(&id)?;
    validate_version(&version)?;
    let type_name = display_name
        .as_deref()
        .map(type_name)
        .unwrap_or_else(|| type_name(id.rsplit('.').next().unwrap_or(&id)));
    if type_name.is_empty() {
        return Err("`--name` must contain at least one letter or digit".to_owned());
    }
    let output = output.unwrap_or_else(|| PathBuf::from(&id));
    fs::create_dir_all(&output).map_err(|error| format!("{}: {error}", output.display()))?;

    let ios_source = format!("ios/Sources/{type_name}.swift");
    let package = package_name(&id);
    let android_source = format!(
        "android/src/main/kotlin/{}/{}.kt",
        package.replace('.', "/"),
        type_name
    );
    write_if_absent(
        &output.join("nexa.plugin.json"),
        &manifest(&id, &version, &ios_source, &android_source),
    )?;
    write_if_absent(
        &output.join("README.md"),
        &readme(&id, &version, &type_name),
    )?;
    write_if_absent(&output.join(&ios_source), &ios_stub(&type_name))?;
    write_if_absent(
        &output.join(&android_source),
        &android_stub(&package, &type_name),
    )?;
    println!("created plugin scaffold {}", output.display());
    Ok(())
}

fn usage() -> String {
    "usage: nexa plugin init <plugin.id> [--out <directory>] [--name <TypeName>] [--version <version>]".to_owned()
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.split('.').any(|part| {
            part.is_empty()
                || !part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
        })
    {
        return Err(
            "plugin id must contain non-empty dot-separated identifier segments".to_owned(),
        );
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<(), String> {
    if version.is_empty() || version.chars().any(char::is_whitespace) {
        return Err("plugin version must be a non-empty value without whitespace".to_owned());
    }
    Ok(())
}

fn type_name(value: &str) -> String {
    let mut result = String::new();
    let mut uppercase = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if uppercase {
                result.extend(character.to_uppercase());
                uppercase = false;
            } else {
                result.push(character);
            }
        } else {
            uppercase = true;
        }
    }
    if result.starts_with(|character: char| character.is_ascii_digit()) {
        result.insert(0, 'N');
    }
    result
}

fn package_name(id: &str) -> String {
    format!(
        "com.nexa.plugin.{}",
        id.replace(['.', '-'], "").to_ascii_lowercase()
    )
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn manifest(id: &str, version: &str, ios_source: &str, android_source: &str) -> String {
    format!(
        "{{\n  \"format\": 1,\n  \"id\": \"{}\",\n  \"version\": \"{}\",\n  \"interfaces\": [],\n  \"implementations\": {{\n    \"ios\": {{ \"source\": \"{}\" }},\n    \"android\": {{ \"source\": \"{}\" }}\n  }}\n}}\n",
        json_escape(id),
        json_escape(version),
        json_escape(ios_source),
        json_escape(android_source),
    )
}

fn readme(id: &str, version: &str, type_name: &str) -> String {
    format!(
        "# {id}\n\nNexa plugin scaffold, version {version}.\n\n## Structure\n\n- `nexa.plugin.json` declares the package identity and platform source entry points.\n- `ios/Sources/{type_name}.swift` is the iOS implementation boundary.\n- `android/src/main/kotlin/` contains the Android implementation boundary.\n\nThe manifest is a stable scaffold for the upcoming typed plugin IDL and binding generator. Add typed interface declarations only when that IDL is available; this scaffold does not install dependencies or connect a plugin to `.nx` code yet.\n"
    )
}

fn ios_stub(type_name: &str) -> String {
    format!(
        "import Foundation\n\npublic enum {type_name}PluginError: Error {{\n    case unavailable\n}}\n\npublic struct {type_name}Plugin {{\n    public init() {{}}\n}}\n"
    )
}

fn android_stub(package: &str, type_name: &str) -> String {
    format!("package {package}\n\nclass {type_name}Plugin\n")
}

fn write_if_absent(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}
