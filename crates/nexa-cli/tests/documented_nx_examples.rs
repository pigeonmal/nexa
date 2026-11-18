//! Compile complete Nexa applications published in the documentation.
//!
//! Small code fragments in the language guide intentionally depend on the
//! surrounding context explained by the prose. Complete `app` examples are
//! standalone public programs, so compile those directly against both target
//! semantics to prevent their grammar, types, or component APIs from drifting.

use std::{fs, path::Path};

use nexa_compiler::{
    Target, compile_file_with_warnings_for_target, compile_with_warnings_for_target,
};

fn documented_app_examples() -> Vec<(String, String)> {
    documented_fenced_examples("nx")
        .into_iter()
        .filter(|(_, source)| source.trim_start().starts_with("app "))
        .collect()
}

fn documented_plugin_idl_examples() -> Vec<(String, String)> {
    documented_fenced_examples("nxid")
}

fn compile_documented_example(location: &str, source: &str, target: Target) -> Result<(), String> {
    if location.starts_with("docs/custom-icons.md:") && source.contains("trail_") {
        return compile_custom_icon_example(location, source, target);
    }

    compile_with_warnings_for_target(source, target)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn compile_custom_icon_example(location: &str, source: &str, target: Target) -> Result<(), String> {
    let config = documented_fenced_examples("nexa")
        .into_iter()
        .find(|(config_location, config)| {
            config_location.starts_with("docs/custom-icons.md:")
                && config.trim_start().starts_with("config {")
        })
        .map(|(_, config)| config)
        .ok_or_else(|| "custom icon guide is missing its project config example".to_owned())?;

    let project = nexa_testkit::TempDir::new("nexa-documented-custom-icons");
    let app_path = project.path().join("App.nx");
    let config_path = project.path().join("nexa.config.nx");
    let font_directory = project.path().join("assets/fonts");
    fs::create_dir_all(&font_directory).map_err(|error| error.to_string())?;
    fs::write(&app_path, source).map_err(|error| error.to_string())?;
    fs::write(&config_path, config).map_err(|error| error.to_string())?;
    // Project compilation validates the font resource path and extension; it
    // does not inspect font tables, which are outside this compiler example.
    fs::write(font_directory.join("TrailIcons.ttf"), []).map_err(|error| error.to_string())?;

    compile_file_with_warnings_for_target(&app_path, target)
        .map(|_| ())
        .map_err(|error| format!("{location}: {error}"))
}

fn documented_fenced_examples(language: &str) -> Vec<(String, String)> {
    let docs = nexa_testkit::workspace_root().join("docs");
    let mut files = Vec::new();
    collect_markdown_files(&docs, &mut files).unwrap_or_else(|error| {
        panic!(
            "read public documentation under {}: {error}",
            docs.display()
        )
    });
    files.sort();

    let mut examples = Vec::new();
    for path in files {
        let markdown = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let relative = path
            .strip_prefix(nexa_testkit::workspace_root())
            .expect("documentation file is inside the workspace")
            .display()
            .to_string();

        let mut fence: Option<(usize, String)> = None;
        for (line_index, line) in markdown.lines().enumerate() {
            if let Some((start_line, source)) = fence.as_mut() {
                if line.trim() == "```" {
                    examples.push((
                        format!("{relative}:{}", *start_line + 1),
                        source.trim().to_owned(),
                    ));
                    fence = None;
                } else {
                    source.push_str(line);
                    source.push('\n');
                }
            } else if line.trim() == format!("```{language}") {
                fence = Some((line_index, String::new()));
            }
        }
    }
    examples
}

fn collect_markdown_files(
    directory: &Path,
    files: &mut Vec<std::path::PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
    Ok(())
}

#[test]
fn every_complete_nexa_app_example_in_docs_compiles_for_both_targets() {
    let examples = documented_app_examples();
    assert!(
        examples.len() >= 20,
        "the docs gate should cover the published application examples"
    );

    std::thread::Builder::new()
        .name("documented-nx-examples".to_owned())
        // A complex component example can exceed the default 2 MiB test
        // worker stack even though the compiler terminates normally.
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            for (location, source) in examples {
                for target in [Target::Swift, Target::Kotlin] {
                    if let Err(error) = compile_documented_example(&location, &source, target) {
                        panic!(
                            "documented Nexa example at {location} does not compile for {target:?}:\n{error}"
                        );
                    }
                }
            }
        })
        .expect("start docs compiler thread")
        .join()
        .expect("documented Nexa examples compile without panicking");
}

#[test]
fn every_plugin_idl_example_in_docs_parses_and_validates() {
    let examples = documented_plugin_idl_examples();
    assert!(
        examples.len() >= 4,
        "the docs gate should cover every published plugin contract example"
    );

    for (location, source) in examples {
        nexa_plugin_idl::parse(&source).unwrap_or_else(|error| {
            panic!("documented plugin contract at {location} is invalid:\n{error}")
        });
    }
}
