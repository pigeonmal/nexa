//! Versioned, plugin-neutral wire contract for compiler analyzers.
//!
//! Plugins may ship a host executable that reads one JSON value per line from
//! stdin and writes one [`AnalysisResponse`] per line to stdout. Nexa passes
//! source files and target configuration; the plugin owns language-specific
//! analysis and returns source-located diagnostics.

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{Action, Expr, Module, Node, Type};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

pub const ANALYZER_PROTOCOL_VERSION: u16 = 1;

/// A typed call into a native plugin, borrowed from the compiler IR.
///
/// The wrapper keeps the call's namespace, receiver, argument names, codecs,
/// and error behavior available without exposing backend-specific codegen.
pub struct NativeCall<'a> {
    /// Call-site range from the source file, when the expression came from source.
    pub source_span: Option<Span>,
    pub receiver: Option<&'a Expr>,
    pub namespace: &'a str,
    pub name: &'a str,
    pub arguments: &'a [(String, Expr)],
    pub codecs: &'a [nexa_ir::PluginCodec],
    pub return_type: &'a Type,
    pub is_async: bool,
    pub is_throwing: bool,
}

impl<'a> NativeCall<'a> {
    /// Borrows the typed plugin call when `expression` represents one.
    pub fn from_expression(expression: &'a Expr) -> Option<Self> {
        let Expr::NativeCall {
            source_span,
            receiver,
            namespace,
            name,
            arguments,
            codecs,
            return_type,
            is_async,
            is_throwing,
        } = expression
        else {
            return None;
        };
        Some(Self {
            source_span: *source_span,
            receiver: receiver.as_deref(),
            namespace,
            name,
            arguments,
            codecs,
            return_type,
            is_async: *is_async,
            is_throwing: *is_throwing,
        })
    }
}

/// Diagnostics and request metadata made available to one compiler extension.
#[derive(Default)]
pub struct ExtensionContext {
    extension_name: String,
    diagnostics: Vec<CompileError>,
}

impl ExtensionContext {
    fn new(extension_name: String) -> Self {
        Self {
            extension_name,
            diagnostics: Vec::new(),
        }
    }

    /// Name supplied by [`PluginCompilerExtension::name`].
    pub fn extension_name(&self) -> &str {
        &self.extension_name
    }

    /// Adds an error at a source location owned by the extension's analysis.
    pub fn report_error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(CompileError::new(span, message));
    }

    /// Adds an error at the native call's source location when available.
    pub fn report_call_error(&mut self, call: &NativeCall<'_>, message: impl Into<String>) {
        self.report_error(call.source_span.unwrap_or_default(), message);
    }

    /// Adds an already constructed compiler diagnostic.
    pub fn push_error(&mut self, error: CompileError) {
        self.diagnostics.push(error);
    }
}

/// In-process typed hooks for plugin-specific compiler validation.
///
/// Extensions inspect compiler IR after type checking, so they receive native
/// calls and component nodes with resolved types rather than source strings.
pub trait PluginCompilerExtension: Send + Sync {
    /// Stable human-readable extension name for diagnostics and debugging.
    fn name(&self) -> &str;

    /// Inspects each node in the module, screen, component, and widget trees.
    fn inspect_node(&self, _node: &Node, _context: &mut ExtensionContext) {}

    /// Inspects each typed native-plugin call in expressions and actions.
    fn inspect_call(&self, _call: &NativeCall<'_>, _context: &mut ExtensionContext) {}

    /// Runs once after all module nodes and calls have been inspected.
    fn finalize(&self, _context: &mut ExtensionContext) -> Result<(), Vec<CompileError>> {
        Ok(())
    }
}

/// Runs typed compiler extensions over all IR reachable from a module.
///
/// Errors reported directly to each [`ExtensionContext`] and errors returned
/// from `finalize` are combined. An empty vector means validation succeeded.
pub fn inspect_module(
    module: &Module,
    extensions: &[&dyn PluginCompilerExtension],
) -> Result<(), Vec<CompileError>> {
    let mut diagnostics = Vec::new();
    for extension in extensions {
        let context = RefCell::new(ExtensionContext::new(extension.name().to_owned()));
        let mut visit_node = |node: &Node| {
            extension.inspect_node(node, &mut context.borrow_mut());
        };
        let mut visit_expression = |expression: &Expr| {
            if let Some(call) = NativeCall::from_expression(expression) {
                extension.inspect_call(&call, &mut context.borrow_mut());
            }
        };

        nexa_ir::walk::walk_ir(&module.body, &mut visit_node, &mut visit_expression);
        for screen in &module.screens {
            nexa_ir::walk::walk_ir(&screen.body, &mut visit_node, &mut visit_expression);
            for state in &screen.states {
                nexa_ir::walk::walk_expression(&state.initial, &mut visit_expression);
            }
            visit_actions(screen.on_appear.as_deref(), &mut visit_expression);
            visit_actions(screen.on_disappear.as_deref(), &mut visit_expression);
        }
        for component in &module.components {
            nexa_ir::walk::walk_ir(&component.body, &mut visit_node, &mut visit_expression);
            for state in &component.states {
                nexa_ir::walk::walk_expression(&state.initial, &mut visit_expression);
            }
            visit_actions(component.on_appear.as_deref(), &mut visit_expression);
            visit_actions(component.on_disappear.as_deref(), &mut visit_expression);
        }
        for widget in &module.widgets {
            nexa_ir::walk::walk_ir(&widget.body, &mut visit_node, &mut visit_expression);
        }
        for state in module.states.iter().chain(&module.globals) {
            nexa_ir::walk::walk_expression(&state.initial, &mut visit_expression);
        }
        for function in &module.functions {
            for local in &function.class_initializers {
                nexa_ir::walk::walk_expression(&local.initial, &mut visit_expression);
            }
            for local in &function.locals {
                nexa_ir::walk::walk_expression(&local.initial, &mut visit_expression);
            }
            nexa_ir::walk::walk_expression(&function.body, &mut visit_expression);
            visit_actions(function.body_actions.as_deref(), &mut visit_expression);
        }
        for task in &module.background_tasks {
            nexa_ir::walk::walk_actions(&task.actions, &mut visit_expression);
        }
        visit_actions(module.on_appear.as_deref(), &mut visit_expression);
        visit_actions(module.on_disappear.as_deref(), &mut visit_expression);
        visit_actions(module.on_active.as_deref(), &mut visit_expression);
        visit_actions(module.on_inactive.as_deref(), &mut visit_expression);
        visit_actions(module.on_background.as_deref(), &mut visit_expression);

        let mut context = context.into_inner();
        if let Err(errors) = extension.finalize(&mut context) {
            diagnostics.extend(errors);
        }
        diagnostics.extend(context.diagnostics);
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn visit_actions(actions: Option<&[Action]>, visit: &mut impl FnMut(&Expr)) {
    if let Some(actions) = actions {
        nexa_ir::walk::walk_actions(actions, visit);
    }
}

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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use nexa_diagnostics::{CompileError, Span};
    use nexa_ir::{Expr, Function, Module, Node, Type};

    use super::{
        ANALYZER_PROTOCOL_VERSION, AnalysisDiagnostic, AnalysisRequest, AnalysisResponse,
        DiagnosticSeverity, ExtensionContext, NativeCall, PluginCompilerExtension, SourceFile,
        TargetConfiguration, inspect_module,
    };

    struct ProbeExtension {
        nodes: AtomicUsize,
        calls: AtomicUsize,
    }

    impl PluginCompilerExtension for ProbeExtension {
        fn name(&self) -> &str {
            "probe"
        }

        fn inspect_node(&self, node: &Node, context: &mut ExtensionContext) {
            if matches!(node, Node::Spacer) {
                self.nodes.fetch_add(1, Ordering::Relaxed);
                assert_eq!(context.extension_name(), "probe");
            }
        }

        fn inspect_call(&self, call: &NativeCall<'_>, context: &mut ExtensionContext) {
            assert_eq!(call.namespace, "SQLite");
            assert_eq!(call.name, "query");
            assert_eq!(call.source_span, Some(test_call_span()));
            assert!(call.is_async);
            assert!(call.is_throwing);
            self.calls.fetch_add(1, Ordering::Relaxed);
            context.report_call_error(call, "query rejected by test extension");
        }

        fn finalize(&self, _context: &mut ExtensionContext) -> Result<(), Vec<CompileError>> {
            Ok(())
        }
    }

    fn test_call_span() -> Span {
        Span {
            start: 12,
            end: 28,
            line: 3,
            column: 5,
        }
    }

    fn module_with_native_call() -> Module {
        Module {
            app_name: "PluginExtensionProbe".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: vec![Function {
                name: "loadRows".to_owned(),
                receiver: None,
                class_initializers: Vec::new(),
                is_async: true,
                is_throwing: true,
                parameters: Vec::new(),
                locals: Vec::new(),
                return_type: Type::String,
                body: Expr::NativeCall {
                    receiver: None,
                    namespace: "SQLite".to_owned(),
                    name: "query".to_owned(),
                    arguments: Vec::new(),
                    codecs: Vec::new(),
                    return_type: Type::String,
                    source_span: Some(test_call_span()),
                    is_async: true,
                    is_throwing: true,
                },
                body_actions: None,
            }],
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Spacer],
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

    #[test]
    fn typed_extensions_inspect_nodes_and_native_calls_across_the_module() {
        let module = module_with_native_call();
        let extension = ProbeExtension {
            nodes: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
        };

        let errors = inspect_module(&module, &[&extension])
            .expect_err("the extension reports the query as unsupported");

        assert_eq!(extension.nodes.load(Ordering::Relaxed), 1);
        assert_eq!(extension.calls.load(Ordering::Relaxed), 1);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "query rejected by test extension");
        assert_eq!(errors[0].span, test_call_span());
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
