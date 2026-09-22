use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use nexa_diagnostics::{CompileError, Span};
use nexa_syntax::ast::{App, ComponentDecl, ImportDecl, StructDecl};

use crate::{Target, semantic};

pub fn compile_file(path: impl AsRef<Path>) -> Result<nexa_ir::Module, CompileError> {
    Ok(compile_file_with_warnings(path)?.module)
}

pub fn compile_file_with_warnings(
    path: impl AsRef<Path>,
) -> Result<crate::Compilation, CompileError> {
    compile_file_with_target(path, Target::All)
}

pub fn compile_file_for_target(
    path: impl AsRef<Path>,
    target: Target,
) -> Result<nexa_ir::Module, CompileError> {
    Ok(compile_file_with_warnings_for_target(path, target)?.module)
}

pub fn compile_file_with_warnings_for_target(
    path: impl AsRef<Path>,
    target: Target,
) -> Result<crate::Compilation, CompileError> {
    compile_file_with_target(path, target)
}

fn compile_file_with_target(
    path: impl AsRef<Path>,
    target: Target,
) -> Result<crate::Compilation, CompileError> {
    let entry_path = path.as_ref();
    let mut loaded = LoadedProject::default();
    load_file(
        entry_path,
        true,
        None,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut loaded,
    )?;

    let mut app = loaded.app.ok_or_else(|| {
        CompileError::new(
            file_level_span(),
            "entry file is missing an `app` declaration",
        )
        .with_file(entry_path.display().to_string())
    })?;
    app.components = loaded.components;
    app.structs = loaded.structs;
    let (module, mut warnings) = semantic::lower_with_warnings(app, target)
        .map_err(|error| error.with_file(entry_path.display().to_string()))?;
    for warning in &mut warnings {
        if warning.file.is_none() {
            warning.file = Some(entry_path.display().to_string());
        }
    }
    Ok(crate::Compilation { module, warnings })
}

#[derive(Default)]
struct LoadedProject {
    app: Option<App>,
    components: Vec<ComponentDecl>,
    structs: Vec<StructDecl>,
}

fn load_file(
    path: &Path,
    is_entry: bool,
    import_site: Option<(&str, Span)>,
    active: &mut HashSet<PathBuf>,
    loaded_paths: &mut HashSet<PathBuf>,
    loaded: &mut LoadedProject,
) -> Result<(), CompileError> {
    let canonical_path = fs::canonicalize(path).map_err(|error| {
        let (span, file) = import_site
            .map(|(file, span)| (span, file.to_owned()))
            .unwrap_or((file_level_span(), path.display().to_string()));
        CompileError::new(span, format!("cannot resolve source file: {error}")).with_file(file)
    })?;

    if active.contains(&canonical_path) {
        let (span, file) = import_site
            .map(|(file, span)| (span, file.to_owned()))
            .unwrap_or((file_level_span(), canonical_path.display().to_string()));
        return Err(CompileError::new(span, "cyclic source import detected").with_file(file));
    }
    if loaded_paths.contains(&canonical_path) {
        return Ok(());
    }

    let source = fs::read_to_string(&canonical_path).map_err(|error| {
        CompileError::new(
            file_level_span(),
            format!("cannot read source file: {error}"),
        )
        .with_file(canonical_path.display().to_string())
    })?;
    let program = nexa_syntax::parse_program(&source)
        .map_err(|error| error.with_file(canonical_path.display().to_string()))?;

    active.insert(canonical_path.clone());
    for import in &program.imports {
        load_import(import, &canonical_path, active, loaded_paths, loaded)?;
    }

    let source_file = canonical_path.display().to_string();
    loaded
        .components
        .extend(program.components.into_iter().map(|mut component| {
            component.source_file = Some(source_file.clone());
            component
        }));
    loaded
        .structs
        .extend(program.structs.into_iter().map(|mut structure| {
            structure.source_file = Some(source_file.clone());
            structure
        }));

    if let Some(app) = program.app {
        if !is_entry {
            return Err(CompileError::new(
                app.span,
                "an imported file can declare components, but not an `app`",
            )
            .with_file(source_file));
        }
        if loaded.app.replace(app).is_some() {
            return Err(CompileError::new(
                file_level_span(),
                "a project can only declare one `app` in its entry file",
            )
            .with_file(source_file));
        }
    } else if is_entry {
        return Err(CompileError::new(
            file_level_span(),
            "entry file is missing an `app` declaration",
        )
        .with_file(source_file));
    }

    active.remove(&canonical_path);
    loaded_paths.insert(canonical_path);
    Ok(())
}

fn file_level_span() -> Span {
    Span {
        line: 1,
        column: 1,
        ..Span::default()
    }
}

fn load_import(
    import: &ImportDecl,
    importing_file: &Path,
    active: &mut HashSet<PathBuf>,
    loaded_paths: &mut HashSet<PathBuf>,
    loaded: &mut LoadedProject,
) -> Result<(), CompileError> {
    let imported_path = importing_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&import.path);
    let importing_path = importing_file.display().to_string();
    load_file(
        &imported_path,
        false,
        Some((&importing_path, import.span)),
        active,
        loaded_paths,
        loaded,
    )
}
