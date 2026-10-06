//! Versioned, plugin-neutral wire contract for compiler analyzers.
//!
//! Plugins may ship a host executable that reads one JSON value per line from
//! stdin and writes one [`AnalysisResponse`] per line to stdout. Nexa passes
//! source files and target configuration; the plugin owns language-specific
//! analysis and returns source-located diagnostics.

use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

pub const ANALYZER_PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisRequest {
    pub protocol_version: u16,
    pub plugin_id: String,
    pub namespace: String,
    pub package_root: String,
    pub entry_file: String,
    pub targets: Vec<TargetConfiguration>,
    pub source_files: Vec<SourceFile>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetConfiguration {
    /// Stable compiler target identifier, such as `swift` or `kotlin`.
    pub target: String,
    pub ios_minimum_version: Option<String>,
    pub android_min_sdk: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: String,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisResponse {
    pub protocol_version: u16,
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    /// Absolute or project-relative source path from the request.
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    /// Optional stable target identifier when a diagnostic applies to only
    /// one generated platform.
    pub target: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// A persistent JSONL analyzer process. One request and response are exchanged
/// per line, allowing LSP and watch builds to reuse the same plugin process.
pub struct AnalyzerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl AnalyzerProcess {
    pub fn spawn(command: &[String], package_root: &Path) -> Result<Self, String> {
        let Some(program) = command.first() else {
            return Err("plugin analyzer command is empty".to_owned());
        };
        let program_path = Path::new(program);
        let executable = if program_path.components().count() > 1 || program.starts_with('.') {
            let resolved = package_root.join(program_path);
            let canonical_root = package_root
                .canonicalize()
                .map_err(|error| format!("{}: {error}", package_root.display()))?;
            let canonical_executable = resolved
                .canonicalize()
                .map_err(|error| format!("{}: {error}", resolved.display()))?;
            if !canonical_executable.starts_with(&canonical_root) || !canonical_executable.is_file()
            {
                return Err(format!(
                    "plugin analyzer executable `{}` must be a file inside `{}`",
                    canonical_executable.display(),
                    canonical_root.display()
                ));
            }
            canonical_executable
        } else {
            PathBuf::from(program)
        };
        let mut child = Command::new(executable)
            .args(&command[1..])
            .current_dir(package_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("unable to start plugin analyzer `{program}`: {error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "plugin analyzer stdin is unavailable".to_owned())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "plugin analyzer stdout is unavailable".to_owned())?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub fn analyze(&mut self, request: &AnalysisRequest) -> Result<AnalysisResponse, String> {
        if let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| format!("unable to check plugin analyzer process: {error}"))?
        {
            return Err(format!("plugin analyzer exited with status {status}"));
        }
        serde_json::to_writer(&mut self.stdin, request)
            .map_err(|error| format!("unable to encode plugin analyzer request: {error}"))?;
        self.stdin
            .write_all(b"\n")
            .and_then(|()| self.stdin.flush())
            .map_err(|error| format!("unable to send plugin analyzer request: {error}"))?;
        let mut line = String::new();
        let bytes = self
            .stdout
            .read_line(&mut line)
            .map_err(|error| format!("unable to read plugin analyzer response: {error}"))?;
        if bytes == 0 {
            let status = self
                .child
                .try_wait()
                .map_err(|error| format!("unable to check plugin analyzer process: {error}"))?;
            return Err(format!(
                "plugin analyzer closed stdout without a response{}",
                status.map_or_else(String::new, |status| format!(" (exit status {status})"))
            ));
        }
        serde_json::from_str(&line)
            .map_err(|error| format!("invalid plugin analyzer response: {error}"))
    }
}

impl Drop for AnalyzerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl AnalysisRequest {
    pub fn new(
        plugin_id: impl Into<String>,
        namespace: impl Into<String>,
        package_root: impl Into<String>,
        entry_file: impl Into<String>,
        targets: Vec<TargetConfiguration>,
        source_files: Vec<SourceFile>,
    ) -> Self {
        Self {
            protocol_version: ANALYZER_PROTOCOL_VERSION,
            plugin_id: plugin_id.into(),
            namespace: namespace.into(),
            package_root: package_root.into(),
            entry_file: entry_file.into(),
            targets,
            source_files,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{
        ANALYZER_PROTOCOL_VERSION, AnalysisDiagnostic, AnalysisRequest, AnalysisResponse,
        DiagnosticSeverity, SourceFile, TargetConfiguration,
    };

    #[test]
    fn request_and_response_share_a_versioned_json_line_contract() {
        let request = AnalysisRequest::new(
            "dev.example.analyzer",
            "Analyzer",
            "/plugins/analyzer",
            "/project/App.nx",
            vec![TargetConfiguration {
                target: "swift".to_owned(),
                ios_minimum_version: Some("16.0".to_owned()),
                android_min_sdk: None,
            }],
            vec![SourceFile {
                path: "/project/App.nx".to_owned(),
                contents: "app Demo {}".to_owned(),
            }],
        );
        let encoded = serde_json::to_string(&request).expect("request serializes");
        let decoded: AnalysisRequest =
            serde_json::from_str(&encoded).expect("request deserializes");
        assert_eq!(decoded, request);
        assert_eq!(decoded.protocol_version, ANALYZER_PROTOCOL_VERSION);

        let response = AnalysisResponse {
            protocol_version: ANALYZER_PROTOCOL_VERSION,
            diagnostics: vec![AnalysisDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: "plugin-owned warning".to_owned(),
                file: "/project/App.nx".to_owned(),
                start: 4,
                end: 8,
                line: 1,
                column: 5,
                target: Some("swift".to_owned()),
            }],
        };
        let encoded = serde_json::to_string(&response).expect("response serializes");
        let decoded: AnalysisResponse =
            serde_json::from_str(&encoded).expect("response deserializes");
        assert_eq!(decoded, response);
    }

    #[cfg(unix)]
    #[test]
    fn analyzer_process_reuses_one_child_for_multiple_json_lines() {
        use std::{fs, os::unix::fs::PermissionsExt, path::Path};

        use super::AnalyzerProcess;
        use nexa_testkit::TempDir;

        let package = TempDir::new("nexa-plugin-analyzer");
        let executable = package.path().join("analyzer.sh");
        fs::write(
            &executable,
            "#!/bin/sh\nwhile IFS= read -r _request; do printf '%s\\n' '{\"protocol_version\":1,\"diagnostics\":[]}'; done\n",
        )
        .expect("script is written");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
            .expect("script is executable");

        let command = vec!["./analyzer.sh".to_owned()];
        let mut process =
            AnalyzerProcess::spawn(&command, Path::new(package.path())).expect("analyzer starts");
        let request = AnalysisRequest::new(
            "dev.example.analyzer",
            "Analyzer",
            package.path().display().to_string(),
            "/project/App.nx",
            Vec::new(),
            Vec::new(),
        );
        assert!(
            process
                .analyze(&request)
                .expect("first response")
                .diagnostics
                .is_empty()
        );
        assert!(
            process
                .analyze(&request)
                .expect("second response")
                .diagnostics
                .is_empty()
        );
    }
}
