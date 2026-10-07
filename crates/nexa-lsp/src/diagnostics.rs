use crate::line_index::LineIndex;
use crate::protocol::{Diagnostic, DiagnosticSeverity, Range};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_diagnostics::{CompileError, CompileWarning, Span};
use nexa_syntax::ast::PluginDependencyConfig;

/// Converts a Nexa source span into an LSP 0-based range for `source`.
///
/// Byte offsets are the canonical location: the span is resolved through a
/// [`LineIndex`] into UTF-16 columns, so multi-line spans and non-ASCII text
/// map correctly. The lexer's scalar line/column are display hints only and
/// are not used here.
pub fn span_to_range(span: &Span, source: &str) -> Range {
    LineIndex::new(source).span_range(source, span)
}

/// Converts a Nexa compiler error to an LSP diagnostic for `source`.
pub fn compile_error_to_diagnostic(error: &CompileError, source: &str) -> Diagnostic {
    Diagnostic {
        range: span_to_range(&error.span, source),
        severity: Some(DiagnosticSeverity::Error),
        message: error.message.clone(),
        source: Some("nexa".to_string()),
    }
}

/// Converts a Nexa compiler warning to an LSP diagnostic for `source`.
pub fn compile_warning_to_diagnostic(warning: &CompileWarning, source: &str) -> Diagnostic {
    Diagnostic {
        range: span_to_range(&warning.span, source),
        severity: Some(DiagnosticSeverity::Warning),
        message: warning.message.clone(),
        source: Some("nexa".to_string()),
    }
}

/// Runs full syntax and semantic analysis on the provided source and returns all diagnostics.
pub fn check_source(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // 1. Syntax analysis
    let program = match nexa_syntax::parse_program(source) {
        Ok(program) => program,
        Err(err) => {
            diagnostics.push(compile_error_to_diagnostic(&err, source));
            return diagnostics;
        }
    };

    // Standalone source modules need their importing app's symbol tables for
    // semantic analysis. The CLI compiles the full source graph; this
    // single-document LSP check can still report syntax errors without
    // incorrectly reporting that a module is missing an app declaration.
    if program.app.is_none() || !program.imports.is_empty() {
        return diagnostics;
    }

    // 2. Full semantic checking and warnings
    match nexa_compiler::compile_with_warnings(source) {
        Ok(compilation) => {
            for warning in compilation.warnings {
                diagnostics.push(compile_warning_to_diagnostic(&warning, source));
            }
        }
        Err(err) => {
            diagnostics.push(compile_error_to_diagnostic(&err, source));
        }
    }

    diagnostics
}

/// Checks the active document format using its URI. Config files have their
/// own parser and must not be fed to the app compiler.
pub fn check_document(source: &str, uri: &str) -> Vec<Diagnostic> {
    if uri
        .rsplit('/')
        .next()
        .is_some_and(|name| name == "nexa.config.nx")
    {
        return nexa_syntax::parse_config(source)
            .err()
            .map(|error| vec![compile_error_to_diagnostic(&error, source)])
            .unwrap_or_default();
    }
    check_source(source)
}

/// Checks an open file in the context of its project, including imported
/// sources and plugin-owned compiler analyzers. Returns `None` when the file
/// is not inside a recognizable app project so callers can use single-file
/// diagnostics instead.
pub fn check_project_document(
    source: &str,
    uri: &str,
    compiler: &mut IncrementalProjectCompiler,
) -> Option<Vec<Diagnostic>> {
    if uri
        .rsplit('/')
        .next()
        .is_some_and(|name| name == "nexa.config.nx")
    {
        return None;
    }
    let active_path = file_uri_path(uri)?;
    let (entry_path, project_root) = find_project_entry(&active_path)?;
    let active_path = absolute_file_path(&active_path)?;
    let entry_path = absolute_file_path(&entry_path)?;
    let plugin_roots = resolve_project_plugin_roots(&project_root);
    let mut overrides = HashMap::with_capacity(1);
    overrides.insert(active_path.clone(), source.to_owned());

    let mut diagnostics = Vec::new();
    match compiler.compile_file_with_warnings_for_targets_and_plugin_roots_with_overrides(
        &entry_path,
        &[Target::All],
        &plugin_roots,
        &overrides,
    ) {
        Ok(compilations) => {
            for warning in compilations
                .into_iter()
                .flat_map(|compile| compile.warnings)
            {
                if diagnostic_belongs_to(&warning.file, &active_path)
                    || (warning.file.is_none() && entry_path == active_path)
                {
                    diagnostics.push(compile_warning_to_diagnostic(&warning, source));
                }
            }
        }
        Err(error) => {
            if diagnostic_belongs_to(&error.file, &active_path)
                || (error.file.is_none() && entry_path == active_path)
            {
                diagnostics.push(compile_error_to_diagnostic(&error, source));
            }
        }
    }
    Some(diagnostics)
}

fn find_project_entry(active_path: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut directory = active_path.parent()?;
    loop {
        let entry = directory.join("App.nx");
        if entry.is_file() && directory.join("nexa.config.nx").is_file() {
            return Some((entry, directory.to_owned()));
        }
        directory = directory.parent()?;
    }
}

/// Reads already-declared project dependencies for safe LSP import fixes.
/// This stays offline and returns `None` for files outside a Nexa app project.
pub(crate) fn project_plugin_dependencies(uri: &str) -> Option<Vec<PluginDependencyConfig>> {
    let active_path = file_uri_path(uri)?;
    let (_, project_root) = find_project_entry(&active_path)?;
    let config_source = fs::read_to_string(project_root.join("nexa.config.nx")).ok()?;
    nexa_syntax::parse_config(&config_source)
        .ok()
        .map(|config| config.dependencies)
}

fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'%' {
            let high = hex_value(*bytes.get(cursor + 1)?)?;
            let low = hex_value(*bytes.get(cursor + 2)?)?;
            decoded.push((high << 4) | low);
            cursor += 3;
        } else {
            decoded.push(bytes[cursor]);
            cursor += 1;
        }
    }
    Some(PathBuf::from(String::from_utf8(decoded).ok()?))
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn absolute_file_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        if path.exists() {
            fs::canonicalize(path).ok()
        } else {
            Some(path.parent()?.canonicalize().ok()?.join(path.file_name()?))
        }
    } else {
        fs::canonicalize(path).ok()
    }
}

fn resolve_project_plugin_roots(project_root: &Path) -> HashMap<String, PathBuf> {
    let mut roots = HashMap::new();
    let config_path = project_root.join("nexa.config.nx");
    let Ok(config_source) = fs::read_to_string(&config_path) else {
        return roots;
    };
    let Ok(config) = nexa_syntax::parse_config(&config_source) else {
        return roots;
    };
    for dependency in config.dependencies {
        let Some(path) = dependency.path else {
            continue;
        };
        roots.insert(dependency.package_id, project_root.join(path));
    }

    // Git package locations are recorded in the lockfile. Reuse an existing
    // checkout without doing network work from the language server.
    let lock_path = project_root.join("nexa.lock");
    let Ok(lock_source) = fs::read_to_string(lock_path) else {
        return roots;
    };
    let Ok(lock) = serde_json::from_str::<serde_json::Value>(&lock_source) else {
        return roots;
    };
    let Some(entries) = lock.get("plugins").and_then(serde_json::Value::as_array) else {
        return roots;
    };
    for entry in entries {
        let (Some(package_id), Some(source), Some(revision)) = (
            entry.get("id").and_then(serde_json::Value::as_str),
            entry.get("source").and_then(serde_json::Value::as_str),
            entry.get("revision").and_then(serde_json::Value::as_str),
        ) else {
            continue;
        };
        let Some(git_url) = source.strip_prefix("git:") else {
            continue;
        };
        let repository = project_root.join(".nexa/plugins").join(format!(
            "{:016x}-{}",
            stable_plugin_url_hash(git_url.as_bytes()),
            revision.to_ascii_lowercase()
        ));
        let package_root = entry
            .get("package")
            .and_then(serde_json::Value::as_str)
            .map_or_else(|| repository.clone(), |package| repository.join(package));
        if package_root.is_dir() {
            roots.insert(package_id.to_owned(), package_root);
        }
    }
    roots
}

fn stable_plugin_url_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn diagnostic_belongs_to(file: &Option<String>, active_path: &Path) -> bool {
    file.as_deref()
        .and_then(|file| absolute_file_path(Path::new(file)))
        .is_some_and(|file| file == active_path)
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    use nexa_compiler::IncrementalProjectCompiler;
    use nexa_testkit::TempDir;

    use super::{check_document, check_project_document, check_source, span_to_range};
    use nexa_diagnostics::Span;

    #[test]
    fn ranges_use_utf16_columns_for_non_ascii_lines() {
        // Line 0 is `state café = 0`: `café` starts at byte 6, UTF-16 column 6.
        let source = "state café = 0\n";
        let error_at = source.find("café").expect("probe");
        let range = span_to_range(
            &Span {
                start: error_at,
                end: error_at + "café".len(),
                line: 1,
                column: 7,
            },
            source,
        );
        assert_eq!((range.start.line, range.start.character), (0, 6));
        assert_eq!((range.end.line, range.end.character), (0, 10));
    }

    #[test]
    fn ranges_span_multiple_lines_from_byte_offsets() {
        let source = "app P {\n    body {\n        Text(\"x\")\n    }\n}\n";
        let start = source.find("body").expect("probe");
        let end = source.find('}').expect("probe") + 1;
        let range = span_to_range(
            &Span {
                start,
                end,
                line: 2,
                column: 5,
            },
            source,
        );
        assert_eq!((range.start.line, range.start.character), (1, 4));
        // The first `}` closes the Text-adjacent block on line 3.
        assert_eq!((range.end.line, range.end.character), (3, 5));
    }

    #[test]
    fn zero_width_spans_cover_one_character() {
        let source = "ab\n";
        let range = span_to_range(
            &Span {
                start: 1,
                end: 1,
                line: 1,
                column: 2,
            },
            source,
        );
        assert_eq!((range.start.line, range.start.character), (0, 1));
        assert_eq!((range.end.line, range.end.character), (0, 2));
    }

    #[test]
    fn diagnostics_point_at_non_ascii_errors() {
        // `ü` cannot start an identifier, so the lexer reports a zero-width
        // span on it. The range must cover exactly that character in UTF-16
        // columns instead of collapsing or overshooting by byte length.
        let source = "app P {\n    body {\n        ünknown()\n    }\n}\n";
        let diagnostics = check_source(source);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].message.contains('ü'));
        let range = diagnostics[0].range;
        assert_eq!((range.start.line, range.start.character), (2, 8));
        assert_eq!((range.end.line, range.end.character), (2, 9));
    }

    #[test]
    fn imported_screen_modules_do_not_report_a_missing_app() {
        let diagnostics = check_source("screen Home { Text(\"Home\") }\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn config_documents_use_the_config_parser() {
        let valid = "config { app { displayName: \"Todo\" } }";
        assert!(check_document(valid, "file:///project/nexa.config.nx").is_empty());
        let invalid = "config { unknownOption: true }";
        assert!(!check_document(invalid, "file:///project/nexa.config.nx").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn project_diagnostics_run_plugin_analyzers_on_the_unsaved_buffer() {
        let project = TempDir::new("nexa-lsp-plugin-analysis");
        let package = project.path().join("analyzer");
        fs::create_dir_all(&package).expect("plugin directory should be created");
        fs::write(
            package.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.test.analyzer\" version: \"1.0.0\" sources { native: \"native.nxid\" } compiler { analyzer: [\"./analyzer.sh\"] } }\n",
        )
        .expect("plugin manifest should be written");
        fs::write(
            package.join("native.nxid"),
            "service Marker { fn check() -> Bool }\n",
        )
        .expect("plugin contract should be written");

        let active_file = project.path().join("App.nx");
        let source = "plugin \"./analyzer\" as Analyzer\napp Demo { body { Text(\"ready\") } }\n";
        fs::write(&active_file, "app Demo { body { Text(\"disk\") } }\n")
            .expect("on-disk source should be written");
        fs::write(
            project.path().join("nexa.config.nx"),
            "config { ios { minVersion: \"16.0\" } android { minSdk: 28 } }\n",
        )
        .expect("project config should be written");

        let marker = source.find("Text").expect("diagnostic probe");
        let response = serde_json::json!({
            "protocol_version": 1,
            "diagnostics": [{
                "severity": "error",
                "message": "plugin-owned diagnostic from the editor",
                "file": active_file.display().to_string(),
                "start": marker,
                "end": marker + 4,
                "line": 2,
                "column": 19,
                "target": null
            }]
        });
        let script = format!(
            "#!/bin/sh\nwhile IFS= read -r _request; do printf '%s\\n' '{}'; done\n",
            response
        );
        let analyzer = package.join("analyzer.sh");
        fs::write(&analyzer, script).expect("analyzer script should be written");
        fs::set_permissions(&analyzer, fs::Permissions::from_mode(0o700))
            .expect("analyzer should be executable");

        let uri = format!("file://{}", active_file.display());
        let diagnostics =
            check_project_document(source, &uri, &mut IncrementalProjectCompiler::default())
                .expect("the project should be recognized");
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .message
                .contains("plugin-owned diagnostic from the editor")
        );
        let line_start = source[..marker]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        assert_eq!(diagnostics[0].range.start.line, 1);
        assert_eq!(
            diagnostics[0].range.start.character,
            (marker - line_start) as u32
        );
    }
}
