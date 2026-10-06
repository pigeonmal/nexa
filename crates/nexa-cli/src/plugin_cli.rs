//! `nexa plugin init | check | generate` — authoring commands for plugin
//! packages.
//!
//! These commands operate on a package directory (or a bare `native.nxid`) and
//! never on the surrounding application project, so plugin work can be
//! validated and committed from the plugin repository alone. Every step is
//! deterministic: the same inputs produce the same files and the same
//! diagnostics, and scaffolding refuses to overwrite a file whose contents
//! differ from the template.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use nexa_codegen::plugin::{bindings, bindings_cpp, bridge_plan::BridgePlan};
use nexa_plugin_idl::{PluginIdl, manifest};

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("init") => init(&args[1..]),
        Some("check") => check(&args[1..]),
        Some("generate") => generate(&args[1..]),
        Some(target) => Err(format!(
            "unknown plugin subcommand `{target}`\n\n{}",
            usage()
        )),
        None => Err(usage()),
    }
}

fn usage() -> String {
    "Usage:\n  nexa plugin init <plugin.id> --out <directory> [--name <TypeName>] [--kind native|pure]\n  nexa plugin check <package-directory|native.nxid>\n  nexa plugin generate <package-directory|native.nxid> --target <swift|kotlin|cpp> [--package <name>] [--out <directory>]"
        .to_owned()
}

fn init(args: &[String]) -> Result<(), String> {
    let mut id = None;
    let mut out = None;
    let mut name = None;
    let mut kind = "native".to_owned();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--out" | "-o" => {
                cursor += 1;
                out = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a directory")?,
                ));
            }
            "--name" => {
                cursor += 1;
                name = Some(
                    args.get(cursor)
                        .ok_or("`--name` requires a type name")?
                        .to_owned(),
                );
            }
            "--kind" => {
                cursor += 1;
                kind = args
                    .get(cursor)
                    .ok_or("`--kind` requires `native` or `pure`")?
                    .to_owned();
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if id.is_none() => id = Some(value.to_owned()),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }
    let id = id.ok_or_else(usage)?;
    let directory =
        out.ok_or_else(|| "`nexa plugin init` requires `--out <directory>`".to_owned())?;
    if !matches!(kind.as_str(), "native" | "pure") {
        return Err(format!(
            "unsupported plugin kind `{kind}`; expected `native` or `pure`"
        ));
    }
    let name = name.unwrap_or_else(|| {
        id.rsplit('.')
            .next()
            .and_then(pascal_case)
            .unwrap_or_else(|| "Plugin".to_owned())
    });
    validate_identifier(&name)?;
    scaffold(&directory, &id, &name, &kind)
}

/// Converts a package id suffix into a type name: `fast-math` becomes
/// `FastMath`, `mmkv` becomes `Mmkv`.
fn pascal_case(value: &str) -> Option<String> {
    let mut out = String::new();
    let mut upper = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if upper {
                out.extend(character.to_uppercase());
                upper = false;
            } else {
                out.push(character);
            }
        } else {
            upper = true;
        }
    }
    (!out.is_empty()).then_some(out)
}

fn validate_identifier(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
    {
        return Err("plugin type name must start with a letter".to_owned());
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric())
    {
        return Err("plugin type name must contain only letters and digits".to_owned());
    }
    Ok(())
}

fn scaffold(directory: &Path, id: &str, name: &str, kind: &str) -> Result<(), String> {
    let files = if kind == "pure" {
        pure_template(id, name)
    } else {
        native_template(id, name)
    };
    for (relative, contents) in files {
        let path = directory.join(relative);
        write_deterministic(&path, contents.as_bytes())?;
    }
    println!("scaffolded plugin `{id}` in {}", directory.display());
    Ok(())
}

/// Writes a scaffold file. Re-running is idempotent: identical contents are
/// left alone, and differing contents are reported instead of overwritten.
fn write_deterministic(path: &Path, contents: &[u8]) -> Result<(), String> {
    if path.exists() {
        let existing = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        if existing == contents {
            return Ok(());
        }
        return Err(format!(
            "{} already exists with different contents",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn native_template(id: &str, name: &str) -> Vec<(String, String)> {
    vec![
        ("plugin.config.nx".to_owned(), manifest_template(id, true)),
        (
            "native.nxid".to_owned(),
            format!(
                "// Typed native contract for `{id}`.\n\
                 //\n\
                 // Structs, enums, and error types become Swift types and Kotlin\n\
                 // classes. `native class` declarations become direct contracts:\n\
                 // implement `{name}Impl` and the compiler aliases `{name}` to it.\n\
                 \n\
                 native class {name} {{\n    \
                     init()\n\
                 \n    \
                     fn dispose()\n\
                 }}\n"
            ),
        ),
        (
            format!("ios/Sources/{name}Impl.swift"),
            format!(
                "import Foundation\n\
                 \n\
                 @MainActor\n\
                 public final class {name}Impl: {name}Spec {{\n    \
                     public init() {{}}\n\
                 \n    \
                     public func dispose() {{}}\n\
                 }}\n"
            ),
        ),
        (
            format!("android/src/main/kotlin/dev/nexa/plugin/{name}Impl.kt"),
            format!(
                "package dev.nexa.plugin\n\
                 \n\
                 public class {name}Impl : {name}Spec {{\n    \
                     override fun dispose() {{\n    \
                     }}\n\
                 }}\n"
            ),
        ),
    ]
}

fn pure_template(id: &str, name: &str) -> Vec<(String, String)> {
    vec![
        ("plugin.config.nx".to_owned(), manifest_template(id, false)),
        (
            "plugin.nx".to_owned(),
            format!(
                "// Reusable Nexa source for `{id}`.\n\
                 //\n\
                 // A pure package declares no `native.nxid`: components and\n\
                 // functions are loaded through the normal `.nx` graph.\n\
                 \n\
                 fn {lower}Greeting(name: String) -> String {{\n    \
                     return name\n\
                 }}\n",
                lower = name.to_lowercase(),
            ),
        ),
    ]
}

fn manifest_template(id: &str, native: bool) -> String {
    let sources = if native {
        "    sources {\n        native: \"native.nxid\"\n    }\n"
    } else {
        "    sources {\n        nexa: \"plugin.nx\"\n    }\n"
    };
    let platforms = if native {
        "    ios {\n        minVersion: \"17.0\"\n        sources: [\"ios/Sources/**/*.swift\"]\n    }\n    android {\n        minSdk: 26\n        sources: [\"android/src/main/kotlin/**/*.kt\"]\n    }\n"
    } else {
        ""
    };
    format!(
        "plugin {{\n    schema: 2\n    id: \"{id}\"\n    version: \"0.1.0\"\n{sources}{platforms}}}\n"
    )
}

fn check(args: &[String]) -> Result<(), String> {
    let target = single_target(args)?;
    let package = resolve_package(&target)?;
    let mut checked = Vec::new();
    for (label, plan) in contracts(&package)? {
        let rendered = render(&label, &plan, &package.manifest, &package.idl)?;
        checked.push(format!("{label} contract ({} bytes)", rendered.len()));
    }
    for note in source_notes(&package)? {
        checked.push(note);
    }
    for note in native_typecheck_notes(&package)? {
        checked.push(note);
    }
    println!("{}: {}", package.root.display(), checked.join("; "));
    Ok(())
}

fn generate(args: &[String]) -> Result<(), String> {
    let mut target = None;
    let mut package_arg: Option<String> = None;
    let mut out = None;
    let mut cursor = 0;
    let positional = args
        .iter()
        .filter(|argument| !argument.starts_with('-'))
        .count();
    let mut positionals = Vec::new();
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--target" => {
                cursor += 1;
                target = Some(
                    args.get(cursor)
                        .ok_or("`--target` requires `swift`, `kotlin`, or `cpp`")?
                        .to_owned(),
                );
            }
            "--package" => {
                cursor += 1;
                package_arg = Some(
                    args.get(cursor)
                        .ok_or("`--package` requires a Kotlin package name")?
                        .to_owned(),
                );
            }
            "--out" | "-o" => {
                cursor += 1;
                out = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a directory")?,
                ));
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value => positionals.push(value.to_owned()),
        }
        cursor += 1;
    }
    let _ = positional;
    if positionals.len() > 1 {
        return Err("`nexa plugin generate` accepts one package path".to_owned());
    }
    let target = target.ok_or("`nexa plugin generate` requires `--target <swift|kotlin|cpp>`")?;
    let target = match target.as_str() {
        value @ ("swift" | "kotlin" | "cpp") => value.to_owned(),
        other => {
            return Err(format!(
                "unsupported generation target `{other}`; expected `swift`, `kotlin`, or `cpp`"
            ));
        }
    };
    let directory = out.ok_or("`nexa plugin generate` requires `--out <directory>`".to_owned())?;
    let source = positionals
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let package = resolve_package(&source)?;
    let kotlin_package = package_arg.unwrap_or_else(|| "dev.nexa.plugin".to_owned());
    let contents = match target.as_str() {
        "swift" => {
            let plan = BridgePlan::validate_swift_contract(&package.idl)?;
            bindings::swift(&plan)
        }
        "kotlin" => {
            let plan = BridgePlan::validate_kotlin_contract(&package.idl)?;
            bindings::kotlin(&plan, &kotlin_package)
        }
        _ => {
            let plan = BridgePlan::validate_contract(&package.idl)?;
            let id = package
                .manifest
                .as_ref()
                .map(|manifest| manifest.id.clone())
                .unwrap_or_else(|| "dev.nexa.plugin".to_owned());
            bindings_cpp::render(&plan, &id)
        }
    };
    let file = match target.as_str() {
        "swift" => format!("{}.swift", package.namespace),
        "kotlin" => format!("{}.kt", package.namespace),
        _ => format!("{}.hpp", package.namespace),
    };
    let path = directory.join(file);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(&path, contents.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn single_target(args: &[String]) -> Result<PathBuf, String> {
    let mut positional = None;
    for argument in args {
        if argument.starts_with('-') {
            return Err(format!("unknown option `{argument}`"));
        }
        if positional.is_some() {
            return Err("expected exactly one package path".to_owned());
        }
        positional = Some(PathBuf::from(argument));
    }
    positional.ok_or_else(|| "expected a plugin package directory or `native.nxid`".to_owned())
}

/// A resolved package plus the namespace generation derives from its id.
struct ResolvedPackage {
    root: PathBuf,
    namespace: String,
    idl: PluginIdl,
    manifest: Option<manifest::PluginManifest>,
}

fn resolve_package(path: &Path) -> Result<ResolvedPackage, String> {
    if path.is_dir() {
        return resolve_directory(path);
    }
    if path.is_file() {
        let idl = nexa_plugin_idl::parse_file(path)?;
        return Ok(ResolvedPackage {
            root: path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(".")),
            namespace: path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Plugin")
                .to_owned(),
            idl,
            manifest: None,
        });
    }
    Err(format!("{} is not a plugin package", path.display()))
}

fn resolve_directory(root: &Path) -> Result<ResolvedPackage, String> {
    let manifest_path = root.join("plugin.config.nx");
    let manifest = manifest::parse_file(&manifest_path)?;
    if let Some(compiler) = &manifest.compiler
        && let Some(program) = compiler.analyzer.first()
        && (program.contains('/') || program.contains('\\') || program.starts_with('.'))
    {
        let package_root =
            fs::canonicalize(root).map_err(|error| format!("{}: {error}", root.display()))?;
        let analyzer_path = package_root.join(program);
        let canonical_analyzer = fs::canonicalize(&analyzer_path)
            .map_err(|error| format!("{}: {error}", analyzer_path.display()))?;
        if !canonical_analyzer.starts_with(&package_root) || !canonical_analyzer.is_file() {
            return Err(format!(
                "{}: compiler analyzer must be a file inside the plugin package",
                analyzer_path.display()
            ));
        }
    }
    let idl_path = root.join(manifest.native.as_deref().unwrap_or("native.nxid"));
    let idl = if manifest.native.is_some() {
        nexa_plugin_idl::parse_file(&idl_path)?
    } else {
        PluginIdl {
            types: Vec::new(),
            interfaces: Vec::new(),
            config: Vec::new(),
        }
    };
    // Declared platform sources must resolve: a manifest that points at a
    // missing tree fails here rather than in a generated project.
    for (platform, sources) in [
        ("ios", &manifest.ios.sources),
        ("android", &manifest.android.sources),
        ("cpp", &manifest.cpp.sources),
    ] {
        if sources.is_empty() {
            continue;
        }
        if sources.iter().all(|source| !pattern_matches(root, source)) {
            return Err(format!(
                "{}: declared {platform} sources do not match any file ({})",
                manifest_path.display(),
                sources.join(", ")
            ));
        }
    }
    for asset in &manifest.assets {
        if !root.join(asset).exists() {
            return Err(format!(
                "{}: declared asset root `{}` does not exist",
                manifest_path.display(),
                asset
            ));
        }
    }
    let suffix = manifest
        .id
        .rsplit('.')
        .next()
        .unwrap_or(manifest.id.as_str());
    let namespace = pascal_case(suffix).unwrap_or_else(|| "Plugin".to_owned());
    Ok(ResolvedPackage {
        root: root.to_path_buf(),
        namespace,
        idl,
        manifest: Some(manifest),
    })
}

/// Minimal glob matcher for the `*`, `?`, and `**` forms the manifest
/// documents. Deterministic, and never touches the filesystem beyond the
/// package root.
fn pattern_matches(root: &Path, pattern: &str) -> bool {
    let normalized = pattern.trim_end_matches('/');
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if glob_matches(normalized, &relative) {
                return true;
            }
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                stack.push(path);
            }
        }
    }
    false
}

fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    segments_match(&pattern, &path)
}

fn segments_match(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.first() {
        None => path.is_empty(),
        Some(&"**") => {
            for split in 0..=path.len() {
                if segments_match(&pattern[1..], &path[split..]) {
                    return true;
                }
            }
            false
        }
        Some(segment) => match path.first() {
            None => false,
            Some(candidate) => {
                segment_matches(segment, candidate) && segments_match(&pattern[1..], &path[1..])
            }
        },
    }
}

fn segment_matches(pattern: &str, value: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let value: Vec<char> = value.chars().collect();
    fn walk(pattern: &[char], value: &[char]) -> bool {
        match pattern.first() {
            None => value.is_empty(),
            Some('*') => {
                for split in 0..=value.len() {
                    if walk(&pattern[1..], &value[split..]) {
                        return true;
                    }
                }
                false
            }
            Some('?') => !value.is_empty() && walk(&pattern[1..], &value[1..]),
            Some(character) => value.first() == Some(character) && walk(&pattern[1..], &value[1..]),
        }
    }
    walk(&pattern, &value)
}

/// Every contract the package must be able to produce, paired with the plan
/// that proves it is renderable.
fn contracts(package: &ResolvedPackage) -> Result<Vec<(String, BridgePlan)>, String> {
    let mut plans = vec![(
        "swift".to_owned(),
        BridgePlan::validate_swift_contract(&package.idl)?,
    )];
    if package
        .manifest
        .as_ref()
        .is_some_and(|manifest| !manifest.android.sources.is_empty())
    {
        plans.push((
            "kotlin".to_owned(),
            BridgePlan::validate_kotlin_contract(&package.idl)?,
        ));
    }
    // The C++ contract is only required when the package declares a C++ block:
    // a plugin with no C++ sources never generates one, and a contract it does
    // not generate must not constrain what it may declare.
    if package
        .manifest
        .as_ref()
        .is_some_and(|manifest| !manifest.cpp.sources.is_empty())
    {
        plans.push((
            "cpp".to_owned(),
            BridgePlan::validate_contract(&package.idl)?,
        ));
    }
    Ok(plans)
}

fn render(
    label: &str,
    plan: &BridgePlan,
    manifest: &Option<manifest::PluginManifest>,
    idl: &PluginIdl,
) -> Result<String, String> {
    let _ = idl;
    match label {
        "swift" => Ok(bindings::swift(plan)),
        "kotlin" => Ok(bindings::kotlin(plan, "dev.nexa.plugin")),
        _ => {
            let id = manifest
                .as_ref()
                .map(|manifest| manifest.id.clone())
                .unwrap_or_else(|| "dev.nexa.plugin".to_owned());
            Ok(bindings_cpp::render(plan, &id))
        }
    }
}

/// Counted evidence that the declared sources exist, so `check` fails on an
/// empty or missing platform tree instead of reporting success.
fn source_notes(package: &ResolvedPackage) -> Result<Vec<String>, String> {
    let Some(manifest) = package.manifest.as_ref() else {
        return Ok(Vec::new());
    };
    let mut notes = Vec::new();
    for (label, sources) in [
        ("ios", &manifest.ios.sources),
        ("ios extension", &manifest.ios.extension_sources),
        ("android", &manifest.android.sources),
    ] {
        if sources.is_empty() {
            continue;
        }
        let count = sources
            .iter()
            .filter(|source| pattern_matches(&package.root, source))
            .count();
        notes.push(format!("{label} sources matched ({count} pattern(s))"));
    }
    Ok(notes)
}

/// Type-checks the plugin's own Swift or Kotlin sources against the generated
/// contract when the toolchain is available and no external dependency would
/// make the check meaningless.
fn native_typecheck_notes(package: &ResolvedPackage) -> Result<Vec<String>, String> {
    let Some(manifest) = package.manifest.as_ref() else {
        return Ok(Vec::new());
    };
    let mut notes = Vec::new();
    if !manifest.ios.sources.is_empty() && manifest.ios.swift_packages.is_empty() {
        match typecheck_swift(package) {
            Some(note) => notes.push(note),
            None => notes.push("ios type-check skipped (no Swift toolchain)".to_owned()),
        }
    } else if !manifest.ios.sources.is_empty() {
        notes.push(
            "ios type-check skipped (declared Swift packages resolve in the generated Xcode project)"
                .to_owned(),
        );
    }
    if !manifest.android.sources.is_empty() && manifest.android.maven_dependencies.is_empty() {
        match typecheck_kotlin(package) {
            Some(note) => notes.push(note),
            None => notes.push("android type-check skipped (no kotlinc)".to_owned()),
        }
    } else if !manifest.android.sources.is_empty() {
        notes.push(
            "android type-check skipped (declared Maven dependencies resolve in the generated Gradle project)"
                .to_owned(),
        );
    }
    Ok(notes)
}

fn typecheck_swift(package: &ResolvedPackage) -> Option<String> {
    let scratch = std::env::temp_dir().join(format!("nexa-plugin-swift-{}", std::process::id()));
    fs::create_dir_all(&scratch).ok()?;
    let contract = scratch.join("Contract.swift");
    fs::write(
        &contract,
        render(
            "swift",
            &contracts(package)
                .ok()?
                .into_iter()
                .find(|(label, _)| label == "swift")?
                .1,
            &package.manifest,
            &package.idl,
        )
        .ok()?,
    )
    .ok()?;
    let mut command = Command::new("xcrun");
    command
        .arg("swiftc")
        .arg("-typecheck")
        .arg("-swift-version")
        .arg("6");
    if let Some(sdk) = sdk_path() {
        // The iOS SDK alone is not enough: without a matching target triple
        // `swiftc` falls back to the host and cannot find a standard library,
        // and the deployment target decides whether concurrency and SwiftUI
        // are available at all. The package's own minimum version is the same
        // one the generated Xcode project will use.
        let minimum = package
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.ios.min_version.clone())
            .unwrap_or_else(|| "13.0".to_owned());
        command
            .arg("-sdk")
            .arg(&sdk)
            .arg("-target")
            .arg(format!("arm64-apple-ios{minimum}"));
    }
    command.arg(&contract);
    let mut files = Vec::new();
    for source in &package.manifest.as_ref()?.ios.sources {
        collect_files(&package.root, source, &mut files);
    }
    for file in &files {
        command.arg(file);
    }
    let output = command.output().ok()?;
    let _ = fs::remove_dir_all(&scratch);
    if output.status.success() {
        Some(format!("ios type-check passed ({} file(s))", files.len()))
    } else {
        Some(format!(
            "ios type-check failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn typecheck_kotlin(package: &ResolvedPackage) -> Option<String> {
    static NEXT_SCRATCH_ID: AtomicU64 = AtomicU64::new(0);
    let scratch = loop {
        let candidate = env::temp_dir().join(format!(
            "nexa-plugin-kotlin-{}-{}",
            std::process::id(),
            NEXT_SCRATCH_ID.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => break candidate,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    };
    let contract = scratch.join("Contract.kt");
    let plan = contracts(package)
        .ok()?
        .into_iter()
        .find(|(label, _)| label == "kotlin")?
        .1;
    let mut source_files = Vec::new();
    for source in &package.manifest.as_ref()?.android.sources {
        collect_files(&package.root, source, &mut source_files);
    }
    let implementation_package = source_files
        .iter()
        .filter_map(|file| fs::read_to_string(file).ok())
        .find_map(|contents| kotlin_package(&contents))
        .unwrap_or_else(|| "dev.nexa.plugin".to_owned());
    fs::write(&contract, bindings::kotlin(&plan, &implementation_package)).ok()?;
    let mut files = vec![contract];
    // A generic method's contract names the value-codec runtime, so the same
    // runtime the app will generate has to be part of this compile too.
    let declares_value_types = plan.interfaces.iter().any(|interface| {
        interface
            .methods
            .iter()
            .any(|method| !method.type_parameters.is_empty())
    });
    if declares_value_types {
        let runtime = scratch.join("NexaValue.kt");
        fs::write(&runtime, nexa_codegen::value::kotlin_runtime_source()).ok()?;
        files.push(runtime);
    }
    let sdk_jar = match android_platform_jar() {
        Some(jar) => jar,
        None => {
            let _ = fs::remove_dir_all(&scratch);
            return Some(
                "android type-check skipped (Android SDK platform android.jar not found)"
                    .to_owned(),
            );
        }
    };
    let needs_coroutines = source_files
        .iter()
        .filter_map(|file| fs::read_to_string(file).ok())
        .any(|contents| contents.contains("kotlinx.coroutines"));
    let coroutines = if needs_coroutines {
        match coroutines_jar() {
            Some(jar) => Some(jar),
            None => {
                let _ = fs::remove_dir_all(&scratch);
                return Some("android type-check skipped (kotlinx-coroutines dependency not found in Gradle cache)".to_owned());
            }
        }
    } else {
        None
    };
    let needs_runtime_stub = source_files
        .iter()
        .filter_map(|file| fs::read_to_string(file).ok())
        .any(|contents| contents.contains("dev.nexa.core.NexaRuntimeCore"));
    if needs_runtime_stub {
        let runtime_source = scratch.join("NexaRuntimeCore.kt");
        fs::write(
            &runtime_source,
            nexa_codegen::value::kotlin_core_runtime_source(),
        )
        .ok()?;
        files.push(runtime_source);
    }
    files.extend(source_files);

    let Some(kotlinc) = kotlinc_path() else {
        let _ = fs::remove_dir_all(&scratch);
        return None;
    };
    let mut classpath = vec![sdk_jar];
    if let Some(coroutines) = coroutines {
        classpath.push(coroutines);
    }
    let classpath = env::join_paths(classpath).ok()?;
    let output = Command::new(kotlinc)
        .arg("-nowarn")
        .arg("-classpath")
        .arg(classpath)
        .args(&files)
        .arg("-d")
        .arg(scratch.join("out.jar"))
        .output()
        .ok()?;
    let _ = fs::remove_dir_all(&scratch);
    if output.status.success() {
        Some("android type-check passed".to_owned())
    } else {
        Some(format!(
            "android type-check failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn kotlin_package(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let package = line.trim().strip_prefix("package ")?.trim();
        (!package.is_empty()).then(|| package.to_owned())
    })
}

fn kotlinc_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("KOTLINC")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Some(path);
    }
    if command_available("kotlinc", &["-version"])
        && let Some(path) = find_on_path("kotlinc")
    {
        return Some(path);
    }
    if let Some(kotlin_home) = env::var_os("KOTLIN_HOME").map(PathBuf::from) {
        let candidate = kotlin_home.join("bin/kotlinc");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let android_studio = env::var_os("ANDROID_STUDIO_HOME")
        .map(PathBuf::from)
        .map(|path| path.join("plugins/Kotlin/kotlinc/bin/kotlinc"));
    android_studio.filter(|path| path.is_file()).or_else(|| {
        let path = PathBuf::from(
            "/Applications/Android Studio.app/Contents/plugins/Kotlin/kotlinc/bin/kotlinc",
        );
        path.is_file().then_some(path)
    })
}

fn find_on_path(executable: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
}

fn command_available(executable: &str, args: &[&str]) -> bool {
    Command::new(executable)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn android_platform_jar() -> Option<PathBuf> {
    let mut sdk_roots = Vec::new();
    for variable in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(path) = env::var_os(variable).map(PathBuf::from) {
            sdk_roots.push(path);
        }
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        sdk_roots.push(home.join("Library/Android/sdk"));
        sdk_roots.push(home.join("Android/Sdk"));
    }
    let mut platforms = sdk_roots
        .into_iter()
        .flat_map(|root| {
            fs::read_dir(root.join("platforms"))
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("android.jar"))
                .filter(|path| path.is_file())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    platforms.sort_by_key(|path| android_api_level(path));
    platforms.pop()
}

fn android_api_level(path: &Path) -> (u32, u32) {
    let platform = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let version = platform.strip_prefix("android-").unwrap_or("");
    let mut parts = version
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok());
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

fn coroutines_jar() -> Option<PathBuf> {
    let gradle_home = env::var_os("GRADLE_USER_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".gradle")))?;
    let module = gradle_home
        .join("caches/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-core-jvm");
    let mut jars = fs::read_dir(module)
        .ok()?
        .filter_map(Result::ok)
        .flat_map(|version| {
            fs::read_dir(version.path())
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
        })
        .flat_map(|hash| {
            fs::read_dir(hash.path())
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
        })
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "jar"))
        .collect::<Vec<_>>();
    jars.sort();
    jars.pop()
}

fn sdk_path() -> Option<String> {
    let output = Command::new("xcrun")
        .args(["--sdk", "iphoneos", "--show-sdk-path"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!path.is_empty()).then_some(path)
}

fn collect_files(root: &Path, pattern: &str, out: &mut Vec<PathBuf>) {
    let normalized = pattern.trim_end_matches('/');
    let mut stack = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        paths.sort();
        for path in paths {
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if glob_matches(normalized, &relative) {
                found.push(path.clone());
            }
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    out.extend(found);
}
