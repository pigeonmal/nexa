//! The deterministic data model behind native project generation.
//!
//! Project generation used to interleave three unrelated concerns in one
//! function: deciding what the host project contains, mutating the filesystem,
//! and rendering text. That made the interesting part -- *what* gets generated
//! -- impossible to inspect or test without also writing files, and it left
//! path and name validation scattered across the code that happened to consume
//! each value.
//!
//! A [`ProjectPlan`] separates them. Planning computes every path, name, and
//! file body up front and validates them once; a writer then only materializes
//! what it is handed. Planning is pure with respect to the project tree, so it
//! can be tested directly.
//!
//! ```
//! use nexa_cli::plan::ProjectPlan;
//!
//! let plan = ProjectPlan::ios("Demo")
//!     .with_source_units(vec![nexa_codegen::SourceUnit {
//!         name: "NexaGenerated.swift".to_owned(),
//!         contents: "import SwiftUI\n".to_owned(),
//!     }])
//!     .with_file("ios/Demo/Info.plist", "<plist/>");
//!
//! // One place decides whether the names and paths are sound.
//! plan.validate().expect("distinct paths are valid");
//! assert_eq!(plan.source_unit_paths(), ["ios/NexaGenerated.swift"]);
//! ```

use nexa_codegen::SourceUnit;
use std::path::PathBuf;

/// The native platform a plan targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Ios,
    Android,
}

impl Platform {
    /// The directory under the project root that holds this platform's files.
    pub fn directory(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }

    /// The file extension of the generated compile units.
    pub fn unit_extension(self) -> &'static str {
        match self {
            Self::Ios => "swift",
            Self::Android => "kt",
        }
    }
}

/// A text file the writer should create, with a project-root-relative path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedFile {
    /// Project-root-relative path, using `/` separators.
    pub path: String,
    /// Complete file contents.
    pub contents: String,
}

impl PlannedFile {
    /// Creates a planned file.
    pub fn new(path: impl Into<String>, contents: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
        }
    }
}

/// A file or directory the writer should copy into the project.
///
/// Plugin packages ship sources, resources, and vendored artifacts that the
/// project links but does not generate. Naming them as data keeps path
/// validation in one place and lets the writer perform the copy, instead of
/// each staging function validating and writing on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyAction {
    /// Absolute path of the file or directory to copy.
    pub source: PathBuf,
    /// Project-root-relative destination, using `/` separators.
    pub destination: String,
    /// Whether the source is a directory copied recursively.
    pub directory: bool,
}

impl CopyAction {
    /// Copies a single file to `destination`.
    pub fn file(source: impl Into<PathBuf>, destination: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            directory: false,
        }
    }

    /// Copies a directory recursively to `destination`.
    pub fn directory(source: impl Into<PathBuf>, destination: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            directory: true,
        }
    }
}

/// A binary file the writer should create, with a project-relative path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedBinary {
    /// Project-root-relative path, using `/` separators.
    pub path: String,
    /// Complete file contents.
    pub contents: Vec<u8>,
}

/// Everything a platform writer needs, computed before anything is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPlan {
    platform: Platform,
    app_name: String,
    source_directory: String,
    source_units: Vec<SourceUnit>,
    files: Vec<PlannedFile>,
    binaries: Vec<PlannedBinary>,
    executables: Vec<String>,
    copies: Vec<CopyAction>,
    removals: Vec<String>,
}

impl ProjectPlan {
    /// Starts a plan for an iOS host project.
    pub fn ios(app_name: &str) -> Self {
        Self::for_platform(Platform::Ios, app_name)
    }

    /// Starts a plan for an Android host project.
    pub fn android(app_name: &str) -> Self {
        Self::for_platform(Platform::Android, app_name)
    }

    fn for_platform(platform: Platform, app_name: &str) -> Self {
        Self {
            platform,
            app_name: app_name.to_owned(),
            source_directory: platform.directory().to_owned(),
            source_units: Vec::new(),
            files: Vec::new(),
            binaries: Vec::new(),
            executables: Vec::new(),
            copies: Vec::new(),
            removals: Vec::new(),
        }
    }

    /// The platform this plan targets.
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// The directory generated sources are written to, relative to the project
    /// root and using `/` separators.
    pub fn source_directory(&self) -> &str {
        &self.source_directory
    }

    /// Points generated sources at a different directory.
    pub fn with_source_directory(mut self, directory: impl Into<String>) -> Self {
        self.source_directory = directory.into();
        self
    }

    /// Sets the generated compile units, in write order.
    pub fn with_source_units(mut self, units: Vec<SourceUnit>) -> Self {
        self.source_units = units;
        self
    }

    /// Adds a text file to the plan.
    pub fn with_file(mut self, path: impl Into<String>, contents: impl Into<String>) -> Self {
        self.files.push(PlannedFile::new(path, contents));
        self
    }

    /// Adds a binary file to the plan.
    ///
    /// Vendored build tooling, such as the Gradle wrapper jar, is not text and
    /// cannot go through the text-file path without corrupting it.
    pub fn with_binary(mut self, path: impl Into<String>, contents: impl Into<Vec<u8>>) -> Self {
        self.binaries.push(PlannedBinary {
            path: path.into(),
            contents: contents.into(),
        });
        self
    }

    /// Marks a path as needing the executable bit after writing.
    pub fn with_executable(mut self, path: impl Into<String>) -> Self {
        self.executables.push(path.into());
        self
    }

    /// Marks a project-relative path for deletion when it is written.
    ///
    /// Used for files the generator stops producing, such as the dev runtime
    /// once a project stops using it, so a rebuild does not leave them behind.
    pub fn with_removal(mut self, path: impl Into<String>) -> Self {
        self.removals.push(path.into());
        self
    }

    /// The generated compile units.
    pub fn source_units(&self) -> &[SourceUnit] {
        &self.source_units
    }

    /// The text files to write.
    pub fn files(&self) -> &[PlannedFile] {
        &self.files
    }

    /// The binary files to write.
    pub fn binaries(&self) -> &[PlannedBinary] {
        &self.binaries
    }

    /// The paths that need the executable bit set after writing.
    pub fn executables(&self) -> &[String] {
        &self.executables
    }

    /// Adds a file or directory to copy into the project.
    pub fn with_copy(mut self, copy: CopyAction) -> Self {
        self.copies.push(copy);
        self
    }

    /// The copies the writer should perform.
    pub fn copies(&self) -> &[CopyAction] {
        &self.copies
    }

    /// The paths to delete before writing.
    pub fn removals(&self) -> &[String] {
        &self.removals
    }

    /// Where each compile unit will be written, relative to the project root.
    pub fn source_unit_paths(&self) -> Vec<String> {
        self.source_units
            .iter()
            .map(|unit| format!("{}/{}", self.source_directory, unit.name))
            .collect()
    }

    /// Every path the plan touches, in the order it touches them: generated
    /// sources first, then the rest of the files.
    pub fn written_paths(&self) -> Vec<String> {
        self.source_unit_paths()
            .into_iter()
            .chain(self.files.iter().map(|file| file.path.clone()))
            .chain(self.binaries.iter().map(|file| file.path.clone()))
            .chain(self.copies.iter().map(|copy| copy.destination.clone()))
            .collect()
    }

    /// Checks that the plan is internally consistent.
    ///
    /// All path and name validation lives here so a planner cannot forget it:
    /// a unit name that escapes its directory, two units claiming the same
    /// file, a file path that leaves the project, or a removal that would
    /// delete something this same plan writes are all rejected.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for unit in &self.source_units {
            let name = unit.name.as_str();
            if name.is_empty() {
                return Err(format!(
                    "app `{}` has a generated source unit with an empty name",
                    self.app_name
                ));
            }
            if name.contains('/') || name.contains('\\') {
                return Err(format!(
                    "generated source unit `{name}` must be a file name, not a path"
                ));
            }
            if name == "." || name == ".." {
                return Err(format!("generated source unit `{name}` is not a file name"));
            }
            if let Some(previous) = seen.iter().find(|seen| **seen == name) {
                return Err(format!(
                    "generated source unit `{name}` collides with an earlier unit `{previous}`"
                ));
            }
            seen.push(name);
        }

        let mut written: Vec<String> = Vec::new();
        for path in self.written_paths() {
            if let Err(error) = validate_relative_path(&path) {
                return Err(error);
            }
            if written.contains(&path) {
                return Err(format!(
                    "`{path}` is written twice, by a source unit and by a planned file"
                ));
            }
            written.push(path);
        }

        for removal in &self.removals {
            validate_relative_path(removal)?;
            if written.iter().any(|path| path == removal) {
                return Err(format!(
                    "`{removal}` is both written and removed by the same plan"
                ));
            }
        }
        Ok(())
    }
}

/// Rejects paths that would escape the project root or are not usable.
fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("a planned path is empty".to_owned());
    }
    if path.starts_with('/') {
        return Err(format!("`{path}` must be relative to the project root"));
    }
    if path.contains('\\') {
        return Err(format!("`{path}` must use `/` separators"));
    }
    if path.split('/').any(|segment| segment == "..") {
        return Err(format!("`{path}` must not leave the project root"));
    }
    if path.ends_with('/') {
        return Err(format!("`{path}` must name a file, not a directory"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CopyAction, ProjectPlan};
    use nexa_codegen::SourceUnit;

    fn unit(name: &str) -> SourceUnit {
        SourceUnit {
            name: name.to_owned(),
            contents: "// generated\n".to_owned(),
        }
    }

    fn plan() -> ProjectPlan {
        ProjectPlan::ios("Demo").with_source_units(vec![unit("NexaGenerated.swift")])
    }

    #[test]
    fn each_platform_owns_its_directory_and_unit_extension() {
        let ios = ProjectPlan::ios("Demo");
        assert_eq!(ios.platform(), super::Platform::Ios);
        assert_eq!(ios.platform().directory(), "ios");
        assert_eq!(ios.platform().unit_extension(), "swift");

        let android = ProjectPlan::android("Demo");
        assert_eq!(android.platform(), super::Platform::Android);
        assert_eq!(android.platform().directory(), "android");
        assert_eq!(android.platform().unit_extension(), "kt");
    }

    #[test]
    fn a_minimal_plan_is_valid() {
        assert!(plan().validate().is_ok());
    }

    #[test]
    fn source_units_live_under_the_source_directory() {
        let plan = ProjectPlan::android("Demo")
            .with_source_directory("android/app/src/main/java/dev/nexa/demo")
            .with_source_units(vec![unit("NexaGenerated.kt")]);
        assert_eq!(
            plan.source_unit_paths(),
            ["android/app/src/main/java/dev/nexa/demo/NexaGenerated.kt"]
        );
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn duplicate_unit_names_are_rejected() {
        let plan = plan().with_source_units(vec![unit("A.swift"), unit("A.swift")]);
        let error = plan.validate().expect_err("duplicate unit names must fail");
        assert!(error.contains("collides"), "{error}");
    }

    #[test]
    fn a_unit_name_may_not_be_a_path() {
        let plan = plan().with_source_units(vec![unit("nested/A.swift")]);
        let error = plan
            .validate()
            .expect_err("a path-like unit name must fail");
        assert!(error.contains("must be a file name"), "{error}");
    }

    #[test]
    fn a_file_may_not_collide_with_a_source_unit() {
        let plan = plan().with_file("ios/NexaGenerated.swift", "// also a unit\n");
        let error = plan.validate().expect_err("a colliding path must fail");
        assert!(error.contains("written twice"), "{error}");
    }

    #[test]
    fn paths_may_not_escape_the_project_root() {
        for path in ["/etc/passwd", "ios/../../outside", "ios/.."] {
            let plan = plan().with_file(path, "x");
            let error = plan.validate().expect_err("an escaping path must fail");
            assert!(!error.is_empty());
        }
    }

    #[test]
    fn absolute_and_backslash_paths_are_rejected() {
        assert!(plan().with_file("/abs", "x").validate().is_err());
        assert!(plan().with_file("ios\\Demo", "x").validate().is_err());
    }

    #[test]
    fn a_plan_may_not_remove_what_it_writes() {
        let plan = plan().with_removal("ios/NexaGenerated.swift");
        let error = plan.validate().expect_err("self-removal must fail");
        assert!(error.contains("both written and removed"), "{error}");
    }

    #[test]
    fn removing_a_stale_file_is_allowed() {
        let plan = plan().with_removal("ios/Demo/NexaDevRuntime.swift");
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn copy_destinations_are_validated_like_every_other_path() {
        let valid = plan().with_copy(CopyAction::file(
            "/pkg/ios/Sources/A.swift",
            "ios/Demo/A.swift",
        ));
        assert!(valid.validate().is_ok());

        let escaping = plan().with_copy(CopyAction::directory("/pkg", "../outside"));
        let error = escaping.validate().expect_err("an escaping copy must fail");
        assert!(error.contains("must not leave the project root"), "{error}");
    }

    #[test]
    fn a_copy_may_not_collide_with_a_planned_file() {
        let colliding = plan()
            .with_file("ios/Demo/Info.plist", "<plist/>")
            .with_copy(CopyAction::file("/pkg/a", "ios/Demo/Info.plist"));
        let error = colliding
            .validate()
            .expect_err("a copy must not overwrite a file");
        assert!(error.contains("written twice"), "{error}");
    }

    #[test]
    fn duplicate_planned_files_are_rejected() {
        let plan = plan()
            .with_file("ios/Demo/Info.plist", "a")
            .with_file("ios/Demo/Info.plist", "b");
        assert!(plan.validate().is_err());
    }

    #[test]
    fn a_planned_file_may_not_name_a_directory() {
        assert!(plan().with_file("ios/Demo/", "x").validate().is_err());
    }

    #[test]
    fn written_paths_lists_sources_before_files() {
        let plan = plan()
            .with_source_units(vec![unit("A.swift"), unit("B.swift")])
            .with_file("ios/Demo/Info.plist", "x");
        assert_eq!(
            plan.written_paths(),
            ["ios/A.swift", "ios/B.swift", "ios/Demo/Info.plist"]
        );
    }
}
