mod audit;
mod cache;
mod commands;
mod config;
mod dependencies;

/// Renderers for reference tables that restate a machine-readable source of
/// truth, gated by the tests in `tests/plugin_reference.rs`.
pub mod docs;
mod plugin;
mod plugin_cli;
mod project;

/// The deterministic model behind native project generation, and the writers
/// that materialize it.
pub mod plan {
    pub use crate::project::plan::{PlannedBinary, PlannedFile, ProjectPlan};
}

/// The generated-source cache schema version.
///
/// Bumped whenever compiler or backend semantics change in a way that makes
/// previously generated native units stale. Published as the newest row of
/// `docs/architecture-audit.md`; the `cache_audit_documents_current_version`
/// test fails when the constant and the audit disagree.
#[doc(hidden)]
pub fn cache_version() -> &'static str {
    cache::CACHE_VERSION
}

/// Run the Nexa command-line interface.
pub fn run() -> Result<(), String> {
    commands::run(std::env::args().skip(1).collect())
}

/// Generate a native host project through Nexa's project pipeline.
///
/// This entry point is used by Nexa's integration tests so they can validate
/// generated hosts without keeping an obsolete `nexa generate` CLI command.
#[doc(hidden)]
pub fn generate_project(
    input: &std::path::Path,
    target: &str,
    output: &std::path::Path,
    name: &str,
) -> Result<(), String> {
    if !matches!(target, "ios" | "android" | "all") {
        return Err(format!("unsupported generation target `{target}`"));
    }
    let arguments = vec![
        input.display().to_string(),
        "--target".to_owned(),
        target.to_owned(),
        "--out".to_owned(),
        output.display().to_string(),
        "--name".to_owned(),
        name.to_owned(),
    ];
    project::run(&arguments)
}

/// Generate the debug host used by the authenticated Nexa dev session.
#[doc(hidden)]
pub fn generate_dev_project(
    input: &std::path::Path,
    target: &str,
    output: &std::path::Path,
    name: &str,
    server_url: &str,
    session_token: &str,
) -> Result<(), String> {
    if !matches!(target, "ios" | "android" | "all") {
        return Err(format!("unsupported generation target `{target}`"));
    }
    let mut arguments = vec![
        input.display().to_string(),
        "--target".to_owned(),
        target.to_owned(),
        "--out".to_owned(),
        output.display().to_string(),
        "--name".to_owned(),
        name.to_owned(),
        "--dev-server-url".to_owned(),
        server_url.to_owned(),
        "--dev-session-token".to_owned(),
        session_token.to_owned(),
    ];
    if !output.is_absolute() {
        let current_dir = std::env::current_dir().map_err(|error| error.to_string())?;
        arguments[4] = current_dir.join(output).display().to_string();
    }
    project::run(&arguments)
}
