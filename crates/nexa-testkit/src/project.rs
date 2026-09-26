use std::fs;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::TempDir;

/// An isolated temporary project workspace.
///
/// Wraps a [`TempDir`] and provides convenient helpers for writing `.nx` source,
/// configs, reading generated files, and asserting on file tree contents.
#[derive(Debug)]
pub struct TestProject {
    dir: TempDir,
}

impl TestProject {
    /// Creates a new temporary project with the given name prefix.
    pub fn new(prefix: &str) -> Self {
        Self {
            dir: TempDir::new(prefix),
        }
    }

    /// The root path of the project.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Appends a relative path to the project root.
    pub fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.dir.path().join(path)
    }

    /// Writes `content` to `relative_path` inside the project, creating parent directories if needed.
    pub fn write(&self, relative_path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> PathBuf {
        let dest = self.join(relative_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|err| {
                panic!(
                    "failed to create parent directories for {}: {err}",
                    dest.display()
                )
            });
        }
        fs::write(&dest, content)
            .unwrap_or_else(|err| panic!("failed to write file {}: {err}", dest.display()));
        dest
    }

    /// Writes `App.nx` with the given source string.
    pub fn write_app(&self, source: &str) -> PathBuf {
        self.write("App.nx", source)
    }

    /// Writes `nexa.config.nx` with the given configuration string.
    pub fn write_config(&self, source: &str) -> PathBuf {
        self.write("nexa.config.nx", source)
    }

    /// Reads a file relative to the project root into a string.
    pub fn read_to_string(&self, relative_path: impl AsRef<Path>) -> io::Result<String> {
        fs::read_to_string(self.join(relative_path))
    }

    /// Returns `true` if the relative path exists.
    pub fn exists(&self, relative_path: impl AsRef<Path>) -> bool {
        self.join(relative_path).exists()
    }

    /// Returns `true` if the relative file exists and contains `needle`.
    pub fn file_contains(&self, relative_path: impl AsRef<Path>, needle: &str) -> bool {
        match self.read_to_string(relative_path) {
            Ok(content) => content.contains(needle),
            Err(_) => false,
        }
    }

    /// Recursively checks if any file with `extension` in the project contains `needle`.
    pub fn source_tree_contains(&self, extension: &str, needle: &str) -> bool {
        self.source_text_containing(extension, needle).is_some()
    }

    /// Recursively finds the first file with `extension` containing `needle` and returns its contents.
    pub fn source_text_containing(&self, extension: &str, needle: &str) -> Option<String> {
        for source in self.collect_sources(extension) {
            if let Ok(content) = fs::read_to_string(&source)
                && content.contains(needle)
            {
                return Some(content);
            }
        }
        None
    }

    /// Reads a file relative to the project root, panicking with a clear error message on failure.
    pub fn read(&self, relative_path: impl AsRef<Path>) -> String {
        self.read_to_string(relative_path.as_ref())
            .unwrap_or_else(|err| {
                panic!(
                    "failed to read file {}: {err}",
                    relative_path.as_ref().display()
                )
            })
    }

    /// Recursively collects all files ending with `extension`.
    pub fn collect_sources(&self, extension: &str) -> Vec<PathBuf> {
        Self::collect_sources_in(self.path(), extension)
    }

    /// Recursively collects all files ending with `extension` inside `root`.
    pub fn collect_sources_in(root: &Path, extension: &str) -> Vec<PathBuf> {
        let mut sources = Vec::new();
        collect_by_extension(root, extension, &mut sources);
        sources.sort();
        sources
    }

    /// Recursively reads and concatenates all files ending with `extension` inside `root`.
    pub fn concat_sources_in(root: &Path, extension: &str) -> String {
        Self::collect_sources_in(root, extension)
            .iter()
            .filter_map(|path| fs::read_to_string(path).ok())
            .collect()
    }

    /// Consumes the guard and keeps the project on disk for debugging.
    pub fn keep(self) -> PathBuf {
        self.dir.keep()
    }
}

fn collect_by_extension(dir: &Path, extension: &str, result: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_by_extension(&path, extension, result);
        } else if path.extension().is_some_and(|ext| ext == extension) {
            result.push(path);
        }
    }
}

impl AsRef<Path> for TestProject {
    fn as_ref(&self) -> &Path {
        self.dir.as_ref()
    }
}

impl Deref for TestProject {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.dir.deref()
    }
}
