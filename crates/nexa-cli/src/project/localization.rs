use std::{collections::BTreeMap, fs, path::Path, sync::Mutex};

use nexa_ir::{Module, localization::TextEntry};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const FORMAT_VERSION: u32 = 1;
static CATALOG_SYNC_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Catalog {
    format: u32,
    #[serde(rename = "sourceLanguage")]
    pub source_language: String,
    pub strings: BTreeMap<String, CatalogEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CatalogEntry {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argument_types: Vec<String>,
    #[serde(skip)]
    pub apple_key: String,
    #[serde(default)]
    pub translations: BTreeMap<String, Value>,
    #[serde(skip)]
    pub apple_value: String,
    #[serde(skip)]
    pub android_value: String,
}

impl CatalogEntry {
    fn from_text(entry: TextEntry, translations: BTreeMap<String, Value>) -> Self {
        Self {
            source: entry.key,
            comment: entry.comment,
            arguments: entry.arguments,
            argument_types: entry.argument_types,
            apple_key: entry.apple_key,
            translations,
            apple_value: entry.apple_value,
            android_value: entry.android_value,
        }
    }

    fn is_plural(&self) -> bool {
        self.arguments
            .iter()
            .position(|name| name == "count")
            .and_then(|index| self.argument_types.get(index))
            .is_some_and(|ty| ty.starts_with("Int") || ty.starts_with("UInt"))
    }
}

impl Catalog {
    pub(super) fn synchronize(source_root: &Path, modules: &[&Module]) -> Result<Self, String> {
        let path = source_root.join("locales/translations.json");
        let mut extracted = BTreeMap::<String, TextEntry>::new();
        for module in modules {
            for (key, entry) in nexa_ir::localization::extract(module) {
                match extracted.get_mut(&key) {
                    Some(existing) => {
                        if let Some(comment) = entry.comment {
                            match &mut existing.comment {
                                Some(existing_comment)
                                    if !existing_comment.lines().any(|line| line == comment) =>
                                {
                                    existing_comment.push('\n');
                                    existing_comment.push_str(&comment);
                                }
                                None => existing.comment = Some(comment),
                                _ => {}
                            }
                        }
                    }
                    None => {
                        extracted.insert(key, entry);
                    }
                }
            }
        }
        let _guard = CATALOG_SYNC_LOCK
            .lock()
            .map_err(|_| "localization catalog synchronization lock is poisoned".to_owned())?;
        let previous = read_new_catalog(&path)?;
        let source_language = previous
            .as_ref()
            .map(|catalog| catalog.source_language.clone())
            .unwrap_or_else(|| "en".to_owned());
        let strings = extracted
            .into_iter()
            .map(|(key, entry)| {
                let translations = previous
                    .as_ref()
                    .and_then(|catalog| catalog.strings.get(&key))
                    .map(|entry| entry.translations.clone())
                    .unwrap_or_default();
                (key, CatalogEntry::from_text(entry, translations))
            })
            .collect();
        let catalog = Self {
            format: FORMAT_VERSION,
            source_language,
            strings,
        };
        write_catalog(&path, &catalog)?;
        Ok(catalog)
    }

    pub(super) fn android_string_files(&self) -> Result<Vec<(String, String)>, String> {
        let mut files = BTreeMap::<String, Vec<String>>::new();
        for entry in self.strings.values() {
            let resource = nexa_codegen::names::localization_resource_name(&entry.source);
            let comment = android_comment(entry.comment.as_deref());
            if entry.is_plural() {
                let source_forms = BTreeMap::from([
                    ("one".to_owned(), entry.android_value.clone()),
                    ("other".to_owned(), entry.android_value.clone()),
                ]);
                push_android_value(
                    &mut files,
                    "values",
                    comment.as_deref(),
                    &resource,
                    AndroidValue::Plural(source_forms),
                );
            } else {
                push_android_value(
                    &mut files,
                    "values",
                    comment.as_deref(),
                    &resource,
                    AndroidValue::String(entry.android_value.clone()),
                );
            }
            for (language, translation) in &entry.translations {
                let language = canonical_language_tag(language)?;
                let directory = format!("values-b+{}", language.replace('-', "+"));
                let value = android_translation(entry, translation)?;
                push_android_value(&mut files, &directory, comment.as_deref(), &resource, value);
            }
        }
        Ok(files
            .into_iter()
            .map(|(directory, rows)| {
                let mut xml =
                    String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n");
                for row in rows {
                    xml.push_str("    ");
                    xml.push_str(&row);
                    xml.push('\n');
                }
                xml.push_str("</resources>\n");
                (format!("{directory}/strings.xml"), xml)
            })
            .collect())
    }

    pub(super) fn apple_string_catalog(&self) -> Result<String, String> {
        let mut strings = serde_json::Map::new();
        for entry in self.strings.values() {
            let mut localizations = serde_json::Map::new();
            localizations.insert(
                self.source_language.clone(),
                apple_value(entry, &entry.source, &Value::Null, true)?,
            );
            for (language, translation) in &entry.translations {
                let language = canonical_language_tag(language)?;
                localizations.insert(
                    language,
                    apple_value(entry, &entry.source, translation, false)?,
                );
            }
            let mut definition = serde_json::Map::new();
            if let Some(comment) = &entry.comment {
                definition.insert("comment".to_owned(), Value::String(comment.clone()));
            }
            definition.insert("localizations".to_owned(), Value::Object(localizations));
            strings.insert(entry.apple_key.clone(), Value::Object(definition));
        }
        serde_json::to_string_pretty(&json!({
            "sourceLanguage": self.source_language,
            "strings": strings,
            "version": "1.1"
        }))
        .map_err(|error| format!("could not serialize generated Apple string catalog: {error}"))
    }

    pub(super) fn dev_translations(&self) -> BTreeMap<String, BTreeMap<String, Value>> {
        let mut translations = BTreeMap::new();
        for (key, entry) in &self.strings {
            for (language, value) in &entry.translations {
                translations
                    .entry(language.clone())
                    .or_insert_with(BTreeMap::new)
                    .insert(key.clone(), value.clone());
            }
        }
        translations
    }
}

fn read_new_catalog(path: &Path) -> Result<Option<Catalog>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "translation catalog must be a regular file, not a symlink: {}",
            path.display()
        ));
    }
    let contents =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let catalog: Catalog = serde_json::from_str(&contents)
        .map_err(|error| format!("{}: invalid translations.json: {error}", path.display()))?;
    if catalog.format != FORMAT_VERSION {
        return Err(format!(
            "{}: unsupported translations format {}; expected {FORMAT_VERSION}",
            path.display(),
            catalog.format
        ));
    }
    let language = canonical_language_tag(&catalog.source_language)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if language != catalog.source_language {
        return Err(format!(
            "{}: sourceLanguage must use canonical BCP 47 spelling `{language}`",
            path.display()
        ));
    }
    for entry in catalog.strings.values() {
        for language in entry.translations.keys() {
            canonical_language_tag(language)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    Ok(Some(catalog))
}

fn write_catalog(path: &Path, catalog: &Catalog) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid path {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let mut output = serde_json::to_string_pretty(catalog)
        .map_err(|error| format!("could not serialize translations: {error}"))?;
    output.push('\n');
    if fs::read_to_string(path).ok().as_deref() != Some(output.as_str()) {
        fs::write(path, output).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

fn apple_value(
    entry: &CatalogEntry,
    source: &str,
    translation: &Value,
    is_source: bool,
) -> Result<Value, String> {
    if entry.is_plural() {
        let forms = if is_source {
            BTreeMap::from([
                ("one".to_owned(), entry.apple_value.clone()),
                ("other".to_owned(), entry.apple_value.clone()),
            ])
        } else {
            apple_plural_forms(entry, source, translation)?
        };
        let plural = forms
            .into_iter()
            .map(|(category, text)| {
                Ok((
                    category,
                    json!({"stringUnit": {"state": "translated", "value": text}}),
                ))
            })
            .collect::<Result<serde_json::Map<_, _>, String>>()?;
        Ok(json!({"variations": {"plural": plural}}))
    } else {
        let text = if is_source {
            entry.apple_value.clone()
        } else {
            let value = translation
                .as_str()
                .ok_or_else(|| format!("translation for `{source}` must be a string"))?;
            format_translation(entry, value, true)
        };
        Ok(json!({"stringUnit": {"state": "translated", "value": text}}))
    }
}

fn apple_plural_forms(
    entry: &CatalogEntry,
    source: &str,
    value: &Value,
) -> Result<BTreeMap<String, String>, String> {
    let mut forms = BTreeMap::new();
    match value {
        Value::String(value) => {
            let value = format_translation(entry, value, true);
            forms.insert("one".to_owned(), value.clone());
            forms.insert("other".to_owned(), value);
        }
        Value::Object(values) => {
            for (category, value) in values {
                validate_plural_category(source, category)?;
                let value = value.as_str().ok_or_else(|| {
                    format!("plural translation `{source}` category `{category}` must be a string")
                })?;
                forms.insert(category.clone(), format_translation(entry, value, true));
            }
        }
        _ => {
            return Err(format!(
                "plural translation for `{source}` must be a string or plural form object"
            ));
        }
    }
    if !forms.contains_key("other") {
        let fallback = forms
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| entry.apple_value.clone());
        forms.insert("other".to_owned(), fallback);
    }
    Ok(forms)
}

#[derive(Clone)]
enum AndroidValue {
    String(String),
    Plural(BTreeMap<String, String>),
}

fn android_translation(entry: &CatalogEntry, value: &Value) -> Result<AndroidValue, String> {
    if entry.is_plural() {
        let forms = match value {
            Value::String(value) => BTreeMap::from([
                ("one".to_owned(), format_translation(entry, value, false)),
                ("other".to_owned(), format_translation(entry, value, false)),
            ]),
            Value::Object(values) => {
                let mut forms = BTreeMap::new();
                for (category, value) in values {
                    validate_plural_category(&entry.source, category)?;
                    let value = value.as_str().ok_or_else(|| {
                        format!(
                            "plural translation `{}` category `{category}` must be a string",
                            entry.source
                        )
                    })?;
                    forms.insert(category.clone(), format_translation(entry, value, false));
                }
                if !forms.contains_key("other") {
                    let fallback = forms
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| entry.android_value.clone());
                    forms.insert("other".to_owned(), fallback);
                }
                forms
            }
            _ => {
                return Err(format!(
                    "plural translation for `{}` must be a string or plural form object",
                    entry.source
                ));
            }
        };
        Ok(AndroidValue::Plural(forms))
    } else {
        let value = value
            .as_str()
            .ok_or_else(|| format!("translation for `{}` must be a string", entry.source))?;
        Ok(AndroidValue::String(format_translation(
            entry, value, false,
        )))
    }
}

fn push_android_value(
    files: &mut BTreeMap<String, Vec<String>>,
    directory: &str,
    comment: Option<&str>,
    resource: &str,
    value: AndroidValue,
) {
    let rows = files.entry(directory.to_owned()).or_default();
    if let Some(comment) = comment {
        rows.push(format!("<!-- {} -->", xml_escape_comment(comment)));
    }
    match value {
        AndroidValue::String(value) => rows.push(format!(
            "<string name=\"{resource}\" formatted=\"{}\">{}</string>",
            value.contains('%'),
            xml_escape(&value)
        )),
        AndroidValue::Plural(forms) => {
            rows.push(format!("<plurals name=\"{resource}\">"));
            for (category, value) in forms {
                rows.push(format!(
                    "    <item quantity=\"{category}\">{}</item>",
                    xml_escape(&value)
                ));
            }
            rows.push("</plurals>".to_owned());
        }
    }
}

fn format_translation(entry: &CatalogEntry, template: &str, apple: bool) -> String {
    let mut output = template.to_owned();
    for (index, name) in entry.arguments.iter().enumerate() {
        let kind = entry
            .argument_types
            .get(index)
            .map(String::as_str)
            .unwrap_or("String");
        let specifier = if apple {
            match kind {
                "Int8" | "Int16" | "Int32" => "d",
                "Int64" | "UInt64" => "lld",
                "Float32" | "Float64" => "f",
                _ => "@",
            }
        } else if kind.starts_with("Int") || kind.starts_with("UInt") {
            "d"
        } else if kind.starts_with("Float") {
            "f"
        } else {
            "s"
        };
        output = output.replace(
            &format!("{{{name}}}"),
            &format!("%{}${specifier}", index + 1),
        );
    }
    output
}

fn validate_plural_category(key: &str, category: &str) -> Result<(), String> {
    if matches!(category, "zero" | "one" | "two" | "few" | "many" | "other") {
        Ok(())
    } else {
        Err(format!(
            "plural translation `{key}` has unsupported category `{category}`"
        ))
    }
}

fn canonical_language_tag(tag: &str) -> Result<String, String> {
    let invalid = || format!("invalid BCP 47 language tag `{tag}`");
    let mut parts = tag.split('-');
    let language = parts.next().ok_or_else(invalid)?;
    if !(2..=8).contains(&language.len())
        || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(invalid());
    }
    let mut result = vec![language.to_ascii_lowercase()];
    let mut region_seen = false;
    for part in parts {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(invalid());
        }
        if !region_seen
            && ((part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_alphabetic()))
                || (part.len() == 3 && part.bytes().all(|byte| byte.is_ascii_digit())))
        {
            result.push(part.to_ascii_uppercase());
            region_seen = true;
        } else if part.len() == 4 && part.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            let mut script = part.to_ascii_lowercase();
            script
                .get_mut(0..1)
                .ok_or_else(invalid)?
                .make_ascii_uppercase();
            result.push(script);
        } else if (4..=8).contains(&part.len()) {
            result.push(part.to_ascii_lowercase());
        } else {
            return Err(invalid());
        }
    }
    Ok(result.join("-"))
}

fn android_comment(comment: Option<&str>) -> Option<String> {
    comment
        .map(str::trim)
        .filter(|comment| !comment.is_empty())
        .map(str::to_owned)
}

fn xml_escape_comment(value: &str) -> String {
    value.replace("--", "- -").trim_end_matches('-').to_owned()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
}

pub(super) fn synchronize(source_root: &Path, module: &Module) -> Result<Catalog, String> {
    Catalog::synchronize(source_root, &[module])
}

pub(super) fn synchronize_modules(
    source_root: &Path,
    modules: &[&Module],
) -> Result<Catalog, String> {
    Catalog::synchronize(source_root, modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_values_render_to_platform_resources_with_context_and_plurals() {
        let entry = CatalogEntry {
            source: "{count} file for {name}".to_owned(),
            comment: Some("File summary".to_owned()),
            arguments: vec!["count".to_owned(), "name".to_owned()],
            argument_types: vec!["Int32".to_owned(), "String".to_owned()],
            apple_key: "%d file for %@".to_owned(),
            translations: BTreeMap::from([(
                "fr".to_owned(),
                json!({"one": "{count} fichier pour {name}", "other": "{count} fichiers pour {name}"}),
            )]),
            apple_value: "%1$d file for %2$@".to_owned(),
            android_value: "%1$d file for %2$s".to_owned(),
        };
        let catalog = Catalog {
            format: FORMAT_VERSION,
            source_language: "en".to_owned(),
            strings: BTreeMap::from([(entry.source.clone(), entry)]),
        };
        let android = catalog.android_string_files().expect("Android resources");
        assert!(
            android
                .iter()
                .any(|(path, value)| path == "values/strings.xml"
                    && value.contains("<!-- File summary -->")
                    && value.contains("<plurals")
                    && value.contains("%1$d file for %2$s"))
        );
        assert!(
            android
                .iter()
                .any(|(path, value)| path == "values-b+fr/strings.xml"
                    && value.contains("%1$d fichiers pour %2$s"))
        );
        let apple = catalog.apple_string_catalog().expect("Apple catalog");
        assert!(apple.contains("File summary"));
        assert!(apple.contains("%1$d fichier pour %2$@"));
        assert!(apple.contains("%1$d fichiers pour %2$@"));
    }
}
