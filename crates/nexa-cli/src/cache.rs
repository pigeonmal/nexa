//! Disk-backed cache for deterministic native source builds.
//!
//! The cache stores only generated source and warning text. Compiler semantics
//! remain the source of truth; a cache hit is valid only when the complete
//! entry/import/plugin-IDL graph has the same content fingerprint.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use nexa_syntax::ast::Program;

const CACHE_VERSION: &str = "build-v1";

pub(super) struct CachedBuild {
    pub(super) warnings: Vec<String>,
}

pub(super) fn restore(
    entry: &Path,
    key: &str,
    output: &Path,
) -> Result<Option<CachedBuild>, String> {
    let directory = cache_directory(entry);
    let artifact = directory.join(format!("{key}.source"));
    if !artifact.is_file() {
        return Ok(None);
    }
    let warnings_path = directory.join(format!("{key}.warnings"));
    let warnings = fs::read_to_string(&warnings_path)
        .ok()
        .map(|contents| contents.lines().map(str::to_owned).collect())
        .unwrap_or_default();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::copy(&artifact, output).map_err(|error| format!("{}: {error}", output.display()))?;
    Ok(Some(CachedBuild { warnings }))
}

pub(super) fn store(
    entry: &Path,
    key: &str,
    source: &str,
    warnings: &[String],
) -> Result<(), String> {
    let directory = cache_directory(entry);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    fs::write(directory.join(format!("{key}.source")), source)
        .map_err(|error| format!("cache source: {error}"))?;
    fs::write(
        directory.join(format!("{key}.warnings")),
        warnings.join("\n"),
    )
    .map_err(|error| format!("cache warnings: {error}"))?;
    Ok(())
}

fn cache_directory(entry: &Path) -> PathBuf {
    entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".nexa/cache")
}

pub(super) fn key(entry: &Path, target: &str) -> Result<String, String> {
    let mut hasher = Fnv64::default();
    hasher.write(CACHE_VERSION.as_bytes());
    hasher.write(target.as_bytes());
    let mut visited = HashSet::new();
    fingerprint_file(entry, true, &mut visited, &mut hasher)?;
    Ok(format!("{target}-{:016x}", hasher.finish()))
}

fn fingerprint_file(
    path: &Path,
    is_entry: bool,
    visited: &mut HashSet<PathBuf>,
    hasher: &mut Fnv64,
) -> Result<(), String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("cannot fingerprint {}: {error}", path.display()))?;
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }
    let source = fs::read(&canonical)
        .map_err(|error| format!("cannot fingerprint {}: {error}", canonical.display()))?;
    hasher.write(canonical.to_string_lossy().as_bytes());
    hasher.write(&(source.len() as u64).to_le_bytes());
    hasher.write(&source);

    let Ok(source_text) = std::str::from_utf8(&source) else {
        return Ok(());
    };
    let Ok(program) = nexa_syntax::parse_program(source_text) else {
        return Ok(());
    };
    for import in &program.imports {
        let imported = canonical
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&import.path);
        fingerprint_file(&imported, false, visited, hasher)?;
    }
    if is_entry {
        fingerprint_plugins(&canonical, &program, visited, hasher)?;
    }
    Ok(())
}

fn fingerprint_plugins(
    entry: &Path,
    program: &Program,
    visited: &mut HashSet<PathBuf>,
    hasher: &mut Fnv64,
) -> Result<(), String> {
    for plugin in &program.plugins {
        let declared = entry
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&plugin.path);
        let idl = if declared.is_dir() {
            declared.join("interfaces.nxid")
        } else {
            declared
        };
        fingerprint_file(&idl, false, visited, hasher)?;
    }
    Ok(())
}

#[derive(Default)]
struct Fnv64(u64);

impl Fnv64 {
    fn write(&mut self, bytes: &[u8]) {
        if self.0 == 0 {
            self.0 = 0xcbf29ce484222325;
        }
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
