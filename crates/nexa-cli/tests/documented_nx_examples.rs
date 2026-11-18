//! Compile complete Nexa applications published in the documentation.
//!
//! Small code fragments in the language guide intentionally depend on the
//! surrounding context explained by the prose. Complete `app` examples are
//! standalone public programs, so compile those directly against both target
//! semantics to prevent their grammar, types, or component APIs from drifting.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

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
    let icon_sets = custom_icon_sets(source);
    if icon_sets.is_empty() {
        return compile_with_warnings_for_target(source, target)
            .map(|_| ())
            .map_err(|error| format!("{location}: {error}"));
    }

    let project = nexa_testkit::TempDir::new("nexa-documented-custom-icons");
    let app_path = project.path().join("App.nx");
    fs::write(&app_path, source).map_err(|error| error.to_string())?;
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#E11D48" d="M12 21 3 12a5 5 0 0 1 9-5 5 5 0 0 1 9 5z"/></svg>"##;
    for (set, names) in icon_sets {
        for name in names {
            let path = project
                .path()
                .join("assets/icons")
                .join(set.as_str())
                .join(format!("{name}.svg"));
            let directory = path
                .parent()
                .ok_or_else(|| "invalid icon path".to_owned())?;
            fs::create_dir_all(directory).map_err(|error| error.to_string())?;
            fs::write(path, svg).map_err(|error| error.to_string())?;
        }
    }

    compile_file_with_warnings_for_target(&app_path, target)
        .map(|_| ())
        .map_err(|error| format!("{location}: {error}"))
}

fn custom_icon_sets(source: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut sets = BTreeMap::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("Icon(") {
        let start = cursor + relative + "Icon(".len();
        let Some(end) = source[start..].find(')') else {
            break;
        };
        let invocation = &source[start..start + end];
        cursor = start + end + 1;
        let Some((set, icon)) = invocation.split_once(':') else {
            continue;
        };
        let set = set.trim();
        if matches!(set, "system" | "sfsymbol" | "materialsymbol") {
            continue;
        }
        let valid_set = set.chars().enumerate().all(|(index, value)| {
            value == '_' || value.is_ascii_alphanumeric() && (index > 0 || !value.is_ascii_digit())
        });
        if !valid_set || set.is_empty() {
            continue;
        }
        let Some(first_quote) = icon.find('"') else {
            continue;
        };
        let icon = &icon[first_quote + 1..];
        let Some(last_quote) = icon.find('"') else {
            continue;
        };
        let name = &icon[..last_quote];
        if name.is_empty()
            || name.chars().any(char::is_whitespace)
            || name.contains('/')
            || name.contains('\\')
        {
            continue;
        }
        sets.entry(set.to_owned())
            .or_insert_with(BTreeSet::new)
            .insert(name.to_owned());
    }
    sets
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
