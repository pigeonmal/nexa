//! Materializing a [`ProjectPlan`] onto disk.
//!
//! Writers do no decisions. Everything a writer needs is in the plan, so the
//! same plan always produces the same tree and the interesting logic -- what to
//! generate, and whether the names and paths are sound -- lives in planning and
//! in [`ProjectPlan::validate`].
//!
//! Writes are content-addressed: a file whose contents already match on disk is
//! left alone, so a rebuild does not churn timestamps or trigger downstream
//! incremental builds.

use std::path::Path;

use super::plan::ProjectPlan;

/// Writes every generated source unit under the plan's source directory.
pub fn write_source_units(root: &Path, plan: &ProjectPlan) -> Result<Vec<String>, String> {
    let directory = root.join(plan.source_directory());
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    let mut written = Vec::with_capacity(plan.source_units().len());
    for unit in plan.source_units() {
        write_if_changed(&directory.join(&unit.name), &unit.contents)?;
        written.push(unit.name.clone());
    }
    Ok(written)
}

/// Writes the plan's text files at their project-relative paths.
pub fn write_files(root: &Path, plan: &ProjectPlan) -> Result<(), String> {
    for file in plan.files() {
        write_if_changed(&root.join(&file.path), &file.contents)?;
    }
    Ok(())
}

/// Deletes the plan's removal paths when they exist.
pub fn apply_removals(root: &Path, plan: &ProjectPlan) -> Result<(), String> {
    for path in plan.removals() {
        let path = root.join(path);
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    Ok(())
}

/// Writes the plan's binary files.
pub fn write_binaries(root: &Path, plan: &ProjectPlan) -> Result<(), String> {
    for file in plan.binaries() {
        let path = root.join(&file.path);
        if path.is_file() && std::fs::read(&path).ok().as_deref() == Some(file.contents.as_slice())
        {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        std::fs::write(&path, &file.contents)
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

/// Sets the executable bit on the paths the plan marks executable.
#[cfg(unix)]
pub fn mark_executable(root: &Path, plan: &ProjectPlan) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    for path in plan.executables() {
        let path = root.join(path);
        if !path.is_file() {
            continue;
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

/// Materializes a whole plan: sources, then files, then removals.
pub fn write_plan(root: &Path, plan: &ProjectPlan) -> Result<(), String> {
    plan.validate()?;
    write_source_units(root, plan)?;
    write_files(root, plan)?;
    write_binaries(root, plan)?;
    #[cfg(unix)]
    mark_executable(root, plan)?;
    apply_removals(root, plan)
}

/// Deletes generated units that the current run did not produce.
///
/// A plan lists what exists now; this clears what a previous run left behind so
/// removing a component from the app does not strand its generated file in the
/// project. Only files this generator owns are considered.
pub fn remove_stale_units(
    directory: &Path,
    keep: &[String],
    extension: &str,
) -> Result<(), String> {
    let prefix = "NexaGenerated_";
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{}: {error}", directory.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with(&prefix)
            && path.extension().and_then(|value| value.to_str()) == Some(extension)
            && !keep.iter().any(|current| current == name)
        {
            std::fs::remove_file(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    Ok(())
}

/// Writes `contents` to `path` only when they differ from what is on disk.
pub fn write_if_changed(path: &Path, contents: &str) -> Result<(), String> {
    if path.is_file() && std::fs::read_to_string(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{remove_stale_units, write_if_changed, write_plan};
    use crate::project::plan::ProjectPlan;
    use nexa_codegen::SourceUnit;
    use std::path::{Path, PathBuf};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "nexa-writers-{tag}-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("temp dir");
            Self(path)
        }

        fn read(&self, relative: &str) -> String {
            std::fs::read_to_string(self.0.join(relative)).expect("file was written")
        }

        fn exists(&self, relative: &str) -> bool {
            self.0.join(relative).exists()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn unit(name: &str) -> SourceUnit {
        SourceUnit {
            name: name.to_owned(),
            contents: format!("// {name}\n"),
        }
    }

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("create");
        std::fs::write(path, contents).expect("seed");
    }

    #[test]
    fn write_plan_materializes_sources_files_and_removals() {
        let temp = TempDir::new("materialize");
        write(&temp.0.join("ios/Demo/NexaDevRuntime.swift"), "// stale\n");

        let plan = ProjectPlan::ios("Demo")
            .with_source_units(vec![
                unit("NexaGenerated.swift"),
                unit("NexaGenerated_types.swift"),
            ])
            .with_file("ios/Demo/Info.plist", "<plist/>")
            .with_removal("ios/Demo/NexaDevRuntime.swift");

        write_plan(&temp.0, &plan).expect("plan writes");

        assert_eq!(
            temp.read("ios/NexaGenerated.swift"),
            "// NexaGenerated.swift\n"
        );
        assert_eq!(
            temp.read("ios/NexaGenerated_types.swift"),
            "// NexaGenerated_types.swift\n"
        );
        assert_eq!(temp.read("ios/Demo/Info.plist"), "<plist/>");
        assert!(!temp.exists("ios/Demo/NexaDevRuntime.swift"));
    }

    #[test]
    fn an_invalid_plan_is_refused_before_touching_disk() {
        let temp = TempDir::new("invalid");
        let plan = ProjectPlan::ios("Demo")
            .with_source_units(vec![unit("A.swift")])
            .with_file("ios/A.swift", "collides with the unit");
        let error = write_plan(&temp.0, &plan).expect_err("plan must be refused");
        assert!(error.contains("written twice"), "{error}");
        assert!(!temp.exists("ios/A.swift"));
    }

    #[test]
    fn writing_the_same_contents_twice_leaves_one_file() {
        let temp = TempDir::new("idempotent");
        let path = temp.0.join("ios/Demo/Info.plist");
        write_if_changed(&path, "<plist/>").expect("first write");
        let first = std::fs::metadata(&path)
            .expect("metadata")
            .modified()
            .expect("mtime");
        write_if_changed(&path, "<plist/>").expect("second write");
        let second = std::fs::metadata(&path)
            .expect("metadata")
            .modified()
            .expect("mtime");
        assert_eq!(
            first, second,
            "unchanged contents must not rewrite the file"
        );
    }

    #[test]
    fn stale_units_are_removed_but_current_ones_are_kept() {
        let temp = TempDir::new("stale");
        write(&temp.0.join("ios/NexaGenerated_old.swift"), "// old\n");
        write(&temp.0.join("ios/NexaGenerated_keep.swift"), "// keep\n");
        write(&temp.0.join("ios/HandWritten.swift"), "// not ours\n");

        remove_stale_units(
            &temp.0.join("ios"),
            &["NexaGenerated_keep.swift".to_owned()],
            "swift",
        )
        .expect("cleanup runs");

        assert!(!temp.exists("ios/NexaGenerated_old.swift"));
        assert!(temp.exists("ios/NexaGenerated_keep.swift"));
        assert!(
            temp.exists("ios/HandWritten.swift"),
            "hand-written files are untouched"
        );
    }

    #[test]
    fn removing_units_from_a_missing_directory_is_a_no_op() {
        let temp = TempDir::new("missing");
        remove_stale_units(&temp.0.join("nope"), &[], "swift").expect("no error");
    }
}
