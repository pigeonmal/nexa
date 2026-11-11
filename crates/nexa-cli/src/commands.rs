use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    env, fs,
    io::{self, BufRead, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub(super) fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("create") => create(&args[1..]),
        Some("check") => check_project(&args[1..]),
        Some("audit") => crate::audit::run(&args[1..]),
        Some("fmt") => format_sources(&args[1..]),
        Some("build") => build_command(&args[1..]),
        Some("dev") => native_command("dev", &args[1..]),
        Some("test") => native_command("test", &args[1..]),
        Some("release") => native_command("release", &args[1..]),
        Some("doctor") => doctor(),
        Some("plugin") => crate::plugin_cli::run(&args[1..]),
        Some("--help" | "-h") | None => {
            print_help();
            Ok(())
        }
        Some("--version" | "-V") => {
            println!("nexa {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(command) => Err(format!(
            "unknown command `{command}`\n\nRun `nexa --help` for usage."
        )),
    }
}

fn create(args: &[String]) -> Result<(), String> {
    let mut name = None;
    let mut directory = None;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--directory" | "-d" => {
                cursor += 1;
                directory = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--directory` requires a path")?,
                ));
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if name.is_none() => name = Some(value.to_owned()),
            value => return Err(format!("unexpected argument `{value}`")),
        }
        cursor += 1;
    }
    let name = name.ok_or("usage: nexa create <ProjectName> [--directory <path>]")?;
    let project_name = type_name(&name);
    if project_name.is_empty() {
        return Err("project name must contain at least one letter or digit".to_owned());
    }
    let root = directory.unwrap_or_else(|| PathBuf::from(&name));
    if root.exists() {
        return Err(format!("{} already exists", root.display()));
    }
    fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;
    write(&root.join("App.nx"), &starter_source(&project_name))?;
    write(&root.join("nexa.config.nx"), &starter_config(&project_name))?;
    fs::create_dir_all(root.join(".nexa"))
        .map_err(|error| format!("{}: {error}", root.join(".nexa").display()))?;
    write(
        &root.join(".nexa/signing.properties"),
        "# Local Android release signing. This directory is ignored by Git.\nNEXA_ANDROID_KEYSTORE=\nNEXA_ANDROID_KEY_ALIAS=\nNEXA_ANDROID_STORE_PASSWORD=\nNEXA_ANDROID_KEY_PASSWORD=\n",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.join(".nexa/signing.properties"),
            fs::Permissions::from_mode(0o600),
        )
        .map_err(|error| {
            format!(
                "{}: {error}",
                root.join(".nexa/signing.properties").display()
            )
        })?;
    }
    write(
        &root.join(".gitignore"),
        "build/\n.nexa/\n*.xcuserstate\n.DS_Store\nlocal.properties\n*.keystore\n*.jks\n",
    )?;
    write(
        &root.join("README.md"),
        &format!(
            "# {project_name}\n\nCreated with Nexa. Start the app with `cd {} && nexa dev`.\n",
            root.file_name()
                .and_then(|part| part.to_str())
                .unwrap_or(&name)
        ),
    )?;
    println!("Created {project_name} in {}", root.display());
    println!("Next: cd {} && nexa dev", root.display());
    Ok(())
}

fn format_sources(args: &[String]) -> Result<(), String> {
    let mut check_only = false;
    let mut roots = Vec::new();
    for argument in args {
        match argument.as_str() {
            "--check" => check_only = true,
            "--help" | "-h" => {
                println!("Usage: nexa fmt [<file-or-directory> ...] [--check]");
                return Ok(());
            }
            option if option.starts_with('-') => {
                return Err(format!("unknown `nexa fmt` option `{option}`"));
            }
            path => roots.push(PathBuf::from(path)),
        }
    }
    if roots.is_empty() {
        roots.push(PathBuf::from("."));
    }

    let mut files = Vec::new();
    for root in &roots {
        collect_source_files(root, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err("no `.nx` source files found".to_owned());
    }

    let mut changed = Vec::new();
    for file in files {
        let source =
            fs::read_to_string(&file).map_err(|error| format!("{}: {error}", file.display()))?;
        let formatted = nexa_syntax::format_source(&source)
            .map_err(|error| format!("{}: {}", file.display(), error.message))?;
        if formatted != source {
            changed.push((file, formatted));
        }
    }

    if check_only && !changed.is_empty() {
        let files = changed
            .iter()
            .map(|(file, _)| file.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  ");
        return Err(format!("these files need formatting:\n  {files}"));
    }
    if !check_only {
        for (file, formatted) in changed {
            fs::write(&file, formatted).map_err(|error| format!("{}: {error}", file.display()))?;
        }
    }
    if check_only {
        println!("All Nexa source files are formatted.");
    } else {
        println!("Formatted Nexa source files.");
    }
    Ok(())
}

fn collect_source_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata = fs::metadata(root).map_err(|error| format!("{}: {error}", root.display()))?;
    if metadata.is_file() {
        if root.extension().is_some_and(|extension| extension == "nx") {
            files.push(root.to_owned());
            return Ok(());
        }
        return Err(format!("{} is not a `.nx` source file", root.display()));
    }
    if !metadata.is_dir() {
        return Err(format!("{} is not a file or directory", root.display()));
    }

    let entries = fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))?;
    let mut entries = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    entries.sort();
    for entry in entries {
        let Some(name) = entry.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if matches!(name, ".git" | ".nexa" | "target" | "build" | "node_modules") {
            continue;
        }
        let file_type = fs::symlink_metadata(&entry)
            .map_err(|error| format!("{}: {error}", entry.display()))?
            .file_type();
        if file_type.is_dir() {
            if entry.join(".git").exists() {
                continue;
            }
            collect_source_files(&entry, files)?;
        } else if file_type.is_file()
            && entry.extension().is_some_and(|extension| extension == "nx")
        {
            files.push(entry);
        }
    }
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckDiagnosticOutput {
    severity: &'static str,
    code: Option<String>,
    file: Option<String>,
    span: Option<nexa_compiler::Span>,
    message: String,
}

impl CheckDiagnosticOutput {
    fn from_warning(warning: nexa_compiler::CompileWarning) -> Self {
        Self {
            severity: "warning",
            code: Some(warning.code.as_str().to_owned()),
            file: warning.file,
            span: Some(warning.span),
            message: warning.message,
        }
    }

    fn from_error(error: nexa_compiler::CompileError) -> Self {
        Self {
            severity: "error",
            code: None,
            file: error.file,
            span: Some(error.span),
            message: error.message,
        }
    }

    fn from_message(message: impl Into<String>) -> Self {
        Self {
            severity: "error",
            code: None,
            file: None,
            span: None,
            message: message.into(),
        }
    }

    fn display(&self) -> String {
        match (&self.file, &self.span) {
            (Some(file), Some(span)) if self.severity == "warning" => format!(
                "{file}:{}:{}: warning[{}]: {}",
                span.line,
                span.column,
                self.code.as_deref().unwrap_or("NEXA-W0000"),
                self.message
            ),
            (Some(file), Some(span)) => {
                format!(
                    "{file}:{}:{}: error: {}",
                    span.line, span.column, self.message
                )
            }
            (None, Some(span)) if self.severity == "warning" => format!(
                "{}:{}: warning[{}]: {}",
                span.line,
                span.column,
                self.code.as_deref().unwrap_or("NEXA-W0000"),
                self.message
            ),
            (None, Some(span)) => format!("{}:{}: error: {}", span.line, span.column, self.message),
            (None, None) if self.severity == "warning" => format!("warning: {}", self.message),
            _ => self.message.clone(),
        }
    }
}

struct CheckFailure(Box<CheckDiagnosticOutput>);

impl From<String> for CheckFailure {
    fn from(message: String) -> Self {
        Self(Box::new(CheckDiagnosticOutput::from_message(message)))
    }
}

impl From<&str> for CheckFailure {
    fn from(message: &str) -> Self {
        Self(Box::new(CheckDiagnosticOutput::from_message(message)))
    }
}

impl From<nexa_compiler::CompileError> for CheckFailure {
    fn from(error: nexa_compiler::CompileError) -> Self {
        Self(Box::new(CheckDiagnosticOutput::from_error(error)))
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckReport {
    format_version: u32,
    ok: bool,
    platform: String,
    diagnostics: Vec<CheckDiagnosticOutput>,
    error: Option<String>,
}

fn print_check_report(report: &CheckReport) -> Result<(), String> {
    let json = serde_json::to_string(report)
        .map_err(|error| format!("could not serialize `nexa check --json` report: {error}"))?;
    println!("{json}");
    Ok(())
}

fn check_project(args: &[String]) -> Result<(), String> {
    let json = args.iter().any(|argument| argument == "--json");
    if args
        .iter()
        .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
    {
        println!(
            "Usage: nexa check [--ios | --android] [--deny-warnings] [--audit] [--locked] [--json]"
        );
        return Ok(());
    }
    let args = args
        .iter()
        .filter(|argument| argument.as_str() != "--json")
        .cloned()
        .collect::<Vec<_>>();
    if json && args.iter().any(|argument| argument == "--audit") {
        let report = CheckReport {
            format_version: 1,
            ok: false,
            platform: check_platform_name(&args),
            diagnostics: vec![CheckDiagnosticOutput::from_message(
                "`--audit` cannot be combined with `--json`; run `nexa audit` separately",
            )],
            error: Some("check failed".to_owned()),
        };
        print_check_report(&report)?;
        return Err("check failed; see JSON diagnostics on stdout".to_owned());
    }
    let result = check_project_inner(&args);
    if !json {
        let report = result.map_err(|diagnostic| diagnostic.0.display())?;
        for diagnostic in &report.diagnostics {
            eprintln!("{}", diagnostic.display());
        }
        if let Some(error) = report.error {
            return Err(error);
        }
        println!("check passed ({})", report.platform);
        if args.iter().any(|argument| argument == "--audit") {
            let root = env::current_dir().map_err(|error| error.to_string())?;
            let entry = find_entry(&root)?;
            crate::audit::run(&[
                entry.display().to_string(),
                "--target".to_owned(),
                report.platform,
            ])?;
        }
        return Ok(());
    }

    match result {
        Ok(report) => {
            let failed = !report.ok;
            let failure = report.error.clone();
            print_check_report(&report)?;
            if failed {
                Err(failure.unwrap_or_else(|| "check failed".to_owned()))
            } else {
                Ok(())
            }
        }
        Err(diagnostic) => {
            let report = CheckReport {
                format_version: 1,
                ok: false,
                platform: check_platform_name(&args),
                diagnostics: vec![*diagnostic.0],
                error: Some("check failed".to_owned()),
            };
            print_check_report(&report)?;
            Err("check failed; see JSON diagnostics on stdout".to_owned())
        }
    }
}

fn check_platform_name(args: &[String]) -> String {
    if args.iter().any(|argument| argument == "--ios") {
        "ios".to_owned()
    } else if args.iter().any(|argument| argument == "--android") {
        "android".to_owned()
    } else {
        "all".to_owned()
    }
}

fn check_project_inner(args: &[String]) -> Result<CheckReport, CheckFailure> {
    let mut platform = "all".to_owned();
    let mut platform_set = false;
    let mut deny_warnings = false;
    let mut locked = false;
    for argument in args {
        match argument.as_str() {
            "--ios" => set_platform(&mut platform, &mut platform_set, "ios")?,
            "--android" => set_platform(&mut platform, &mut platform_set, "android")?,
            "--deny-warnings" => deny_warnings = true,
            "--audit" => {}
            "--locked" => locked = true,
            "--help" | "-h" => {}
            option if option.starts_with('-') => {
                return Err(format!("unknown option `{option}`").into());
            }
            value => {
                return Err(
                    format!("unexpected argument `{value}`; run `nexa check --help`").into(),
                );
            }
        }
    }

    let root = env::current_dir().map_err(|error| error.to_string())?;
    let entry = find_entry(&root)?;
    let entry_source =
        fs::read_to_string(&entry).map_err(|error| format!("{}: {error}", entry.display()))?;
    let program = nexa_syntax::parse_program(&entry_source)
        .map_err(|error| error.with_file(entry.display().to_string()))?;
    let app_name = type_name(
        program
            .app
            .as_ref()
            .map(|app| app.name.as_str())
            .or_else(|| entry.file_stem().and_then(|stem| stem.to_str()))
            .unwrap_or("NexaApp"),
    );
    let config_path = root.join("nexa.config.nx");
    let dependencies = crate::config::load_plugin_dependencies(&config_path)?;
    let resolved = crate::dependencies::resolve(&root, &dependencies)?;
    crate::dependencies::sync_lock(&root, !dependencies.is_empty(), &resolved.lock_file, locked)?;
    let definitions = crate::config::load_plugin_definitions(&entry, &resolved.plugin_roots)?;
    let project_config = if config_path.is_file() {
        crate::config::ProjectConfig::parse_file(&config_path, &definitions, &app_name)?
    } else {
        crate::config::ProjectConfig::from_defaults(&[], &app_name)?
    };

    let targets = match platform.as_str() {
        "ios" => vec![nexa_compiler::Target::Swift],
        "android" => vec![nexa_compiler::Target::Kotlin],
        _ => vec![nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin],
    };
    let compilations = nexa_compiler::compile_file_with_warnings_for_targets_and_plugin_roots(
        &entry,
        &targets,
        &resolved.plugin_roots,
    )?;
    let mut warnings = std::collections::BTreeMap::new();
    for (target, compilation) in targets.iter().copied().zip(compilations) {
        let packages = crate::project::plugin_package::packages_for_module(
            &compilation.plugins,
            &compilation.module,
        );
        for warning in crate::project::validate_permission_configuration(
            &compilation.module,
            &packages,
            &project_config,
            &config_path,
            target,
        )? {
            warnings.entry(warning.to_string()).or_insert(warning);
        }
        if target == nexa_compiler::Target::Kotlin {
            let requirements = packages
                .iter()
                .map(|plugin| (plugin.namespace.as_str(), plugin.artifacts.android_min_sdk))
                .collect::<Vec<_>>();
            project_config.validate_android_plugin_minimums(&requirements)?;
            let features =
                nexa_backend_kotlin::KotlinBackend.analyze_project_features(&compilation.module);
            crate::project::validate_android_feature_minimums(features, false, &project_config)?;
        }
        if target == nexa_compiler::Target::Swift {
            crate::project::validate_ios_background_task_minimum(
                &compilation.module,
                &packages,
                &project_config,
            )?;
        }
        for warning in compilation.warnings {
            warnings.entry(warning.to_string()).or_insert(warning);
        }
    }
    let failed = deny_warnings && !warnings.is_empty();
    let diagnostics = warnings
        .into_values()
        .map(CheckDiagnosticOutput::from_warning)
        .collect::<Vec<_>>();
    let error = failed.then(|| format!("{} warning(s) treated as errors", diagnostics.len()));
    Ok(CheckReport {
        format_version: 1,
        ok: !failed,
        platform,
        diagnostics,
        error,
    })
}

fn native_command(command: &str, args: &[String]) -> Result<(), String> {
    let mut platform = "all".to_owned();
    let mut output = PathBuf::from("build");
    let mut output_was_set = false;
    let mut flavor: Option<String> = None;
    let mut arch: Option<String> = None;
    let mut platform_set = false;
    let mut once = false;
    let mut compile_only = false;
    let mut dry_run = false;
    let mut locked = false;
    let mut unit_only = false;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--ios" => {
                select_native_platform(&mut platform, &mut platform_set, "ios");
            }
            "--android" => {
                select_native_platform(&mut platform, &mut platform_set, "android");
            }
            "--all" => {
                select_native_platform(&mut platform, &mut platform_set, "all");
            }
            "--platform" | "-p" => {
                cursor += 1;
                let value = args
                    .get(cursor)
                    .ok_or("`--platform` requires ios, android, or all")?;
                if !matches!(value.as_str(), "ios" | "android" | "all") {
                    return Err(format!(
                        "unknown platform `{value}`; expected ios, android, or all"
                    ));
                }
                select_native_platform(&mut platform, &mut platform_set, value);
            }
            "--out" => {
                cursor += 1;
                output = PathBuf::from(args.get(cursor).ok_or("`--out` requires a directory")?);
                output_was_set = true;
            }
            "--staging" => set_flavor(&mut flavor, "staging")?,
            "--once" if command == "dev" => once = true,
            "--once" => return Err("`--once` is only supported by `nexa dev`".to_owned()),
            "--compile-only" if command == "dev" => compile_only = true,
            "--compile-only" => {
                return Err("`--compile-only` is only supported by `nexa dev`".to_owned());
            }
            "--dry-run" => dry_run = true,
            "--locked" => locked = true,
            "--unit-only" if command == "test" => unit_only = true,
            "--unit-only" => {
                return Err("`--unit-only` is only supported by `nexa test`".to_owned());
            }
            "--flavor" => {
                cursor += 1;
                set_flavor(
                    &mut flavor,
                    args.get(cursor).ok_or("`--flavor` requires a name")?,
                )?;
            }
            "--arch" => {
                cursor += 1;
                if arch.is_some() {
                    return Err("`--arch` may only be specified once".to_owned());
                }
                arch = Some(
                    args.get(cursor)
                        .ok_or("`--arch` requires an architecture")?
                        .clone(),
                );
            }
            "--help" | "-h" => {
                print_command_help(command);
                return Ok(());
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value => {
                return Err(format!(
                    "unexpected argument `{value}`; run `nexa {command} --help`"
                ));
            }
        }
        cursor += 1;
    }

    let root = env::current_dir().map_err(|error| error.to_string())?;
    if !output_was_set && let Some(flavor) = &flavor {
        output.push(flavor);
    }
    let entry = find_entry(&root)?;
    if unit_only && dry_run {
        print_build_plan(command, &platform, once, compile_only, unit_only);
        return Ok(());
    }
    if unit_only {
        return run_in_language_tests(&entry, &platform, locked);
    }
    if !dry_run && command == "release" && matches!(platform.as_str(), "android" | "all") {
        validate_android_release_signing()?;
    }
    let project_name = nexa_syntax::parse_program(
        &fs::read_to_string(&entry).map_err(|error| format!("{}: {error}", entry.display()))?,
    )
    .map_err(|error| format!("{}: {error}", entry.display()))?
    .app
    .map(|app| type_name(&app.name))
    .unwrap_or_else(|| {
        type_name(
            entry
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("NexaApp"),
        )
    });
    let output = if output.is_absolute() {
        output
    } else {
        root.join(output)
    };
    let target = if platform == "all" {
        "all"
    } else {
        platform.as_str()
    };
    if matches!(command, "test" | "release") && matches!(platform.as_str(), "ios" | "all") {
        let configured_ios_arch = root
            .join("nexa.config.nx")
            .is_file()
            .then(|| {
                fs::read_to_string(root.join("nexa.config.nx"))
                    .map_err(|error| format!("nexa.config.nx: {error}"))
            })
            .transpose()?
            .map(|source| {
                nexa_syntax::parse_config(&source)
                    .map_err(|error| format!("nexa.config.nx: {error}"))
            })
            .transpose()?
            .and_then(|config| config.ios.and_then(|ios| ios.arch))
            .filter(|architectures| !(architectures.len() == 1 && architectures[0].is_empty()));
        let invalid_arch = match arch.as_deref() {
            Some(architecture) => architecture != "arm64",
            None => configured_ios_arch.as_ref().is_some_and(|architectures| {
                architectures
                    .iter()
                    .any(|architecture| architecture != "arm64")
            }),
        };
        if invalid_arch {
            return Err("iOS device builds require `arm64`; `x86_64` is supported for `nexa dev` simulator builds".to_owned());
        }
    }
    if once && compile_only {
        return Err("choose only one of `--once` and `--compile-only`".to_owned());
    }
    if dry_run {
        print_build_plan(command, &platform, once, compile_only, unit_only);
        return Ok(());
    }
    preflight_native_toolchains(&platform, command, compile_only)?;
    let mut project_args = vec![
        entry.display().to_string(),
        "--target".to_owned(),
        target.to_owned(),
        "--out".to_owned(),
        output.display().to_string(),
        "--name".to_owned(),
        project_name.clone(),
    ];
    if let Some(flavor) = flavor {
        project_args.push("--flavor".to_owned());
        project_args.push(flavor);
    }
    if let Some(arch) = arch {
        project_args.push("--arch".to_owned());
        project_args.push(arch);
    }
    if locked {
        project_args.push("--locked".to_owned());
        let dependencies = crate::config::load_plugin_dependencies(&root.join("nexa.config.nx"))?;
        let resolved = crate::dependencies::resolve(&root, &dependencies)?;
        crate::dependencies::sync_lock(&root, !dependencies.is_empty(), &resolved.lock_file, true)?;
    }
    if command == "test" {
        project_args.push("--execute-tests".to_owned());
        super::project::run(&project_args)?;
        build_platforms(&output, &project_name, &platform, BuildMode::Test, None)
    } else if command == "dev" {
        if once {
            super::project::run(&project_args)?;
            return build_platforms(&output, &project_name, &platform, BuildMode::Dev, None);
        }
        let _dev_session_lock = (!compile_only)
            .then(|| acquire_dev_session_lock(&root))
            .transpose()?;
        let dependencies = crate::config::load_plugin_dependencies(&root.join("nexa.config.nx"))?;
        let mut plugin_roots = crate::dependencies::resolve(&root, &dependencies)?.plugin_roots;
        let mut dev_compiler = nexa_compiler::IncrementalProjectCompiler::default();
        let modules = super::project::compile_dev_modules_with_compiler(
            &entry,
            &platform,
            &plugin_roots,
            &mut dev_compiler,
        )?;
        let mut background_tasks = background_task_fingerprint(&modules)?;
        let server = nexa_dev_server::DevServer::bind(modules)?;
        if std::env::var_os("NEXA_DEV_FRAME_FENCE").is_some() {
            server.set_frame_fence(true);
        }
        let server_url = format!("ws://{}", server.address());
        project_args.extend([
            "--dev-server-url".to_owned(),
            server_url,
            "--dev-session-token".to_owned(),
            server.session_token().to_owned(),
        ]);
        super::project::run(&project_args)?;
        if compile_only {
            return build_platforms(
                &output,
                &project_name,
                &platform,
                BuildMode::DevCompile,
                Some(server.address().port()),
            );
        }
        let interactive_terminal = io::stdin().is_terminal() && io::stdout().is_terminal();
        if !interactive_terminal {
            print_dev_welcome();
        }
        println!("Nexa dev server listening at ws://{}", server.address());
        build_platforms(
            &output,
            &project_name,
            &platform,
            BuildMode::Dev,
            Some(server.address().port()),
        )?;
        if !interactive_terminal {
            println!("Watching .nx sources. Press Ctrl-C to stop.");
        }
        let watch_context = DevWatchContext {
            root: &root,
            entry: &entry,
            platform: &platform,
            project_name: &project_name,
            output: &output,
            project_args: &project_args,
            server: &server,
        };
        watch_sources(
            &watch_context,
            &mut background_tasks,
            &mut dev_compiler,
            &mut plugin_roots,
        )
    } else {
        super::project::run(&project_args)?;
        build_platforms(&output, &project_name, &platform, BuildMode::Release, None)
    }
}

fn print_build_plan(
    command: &str,
    platform: &str,
    once: bool,
    compile_only: bool,
    unit_only: bool,
) {
    println!(
        "Dry-run build plan only; dependency, lockfile, native build, toolchain, and signing checks are skipped."
    );
    for (index, step) in build_plan_steps(command, platform, once, compile_only, unit_only)
        .iter()
        .enumerate()
    {
        println!("  {}. {step}", index + 1);
    }
}

fn build_plan_steps(
    command: &str,
    platform: &str,
    once: bool,
    compile_only: bool,
    unit_only: bool,
) -> Vec<String> {
    if unit_only {
        return vec!["Compile and run `.nx` unit tests without native hosts.".to_owned()];
    }

    let mut steps = match command {
        "dev" if once => vec!["Compile the app and generate one-shot DevRuntime hosts.".to_owned()],
        "dev" if compile_only => vec![
            "Compile DevRuntime modules, start the local server, and generate debug hosts without launching the app."
                .to_owned(),
        ],
        "dev" => vec![
            "Compile DevRuntime modules and start the local hot-reload server.".to_owned(),
            "Generate debug hosts for the selected platforms.".to_owned(),
        ],
        "test" => vec![
            "Compile the app and its `.nx` test blocks, then generate native test hosts."
                .to_owned(),
        ],
        "release" => vec!["Compile the app and generate native Release projects.".to_owned()],
        _ => vec!["Validate the project and generate native projects.".to_owned()],
    };

    for &target in selected_platforms(platform) {
        let build = match (command, target) {
            ("release", "ios") => "Archive the iOS app and export an IPA with Xcode.",
            ("release", "android") => "Build the signed Android AAB with Gradle.",
            ("test", "ios") => "Build the iOS test host with Xcode.",
            ("test", "android") => "Build the Android test host with Gradle.",
            (_, "ios") => "Build the iOS app with Xcode.",
            (_, "android") => "Build the Android app with Gradle.",
            _ => continue,
        };
        steps.push(build.to_owned());
    }

    if command == "dev" && !compile_only {
        let launch_step = if once {
            "Install and launch the app once on the selected simulator or emulator."
        } else {
            "Install and launch the app on the selected simulator or emulator."
        };
        steps.push(launch_step.to_owned());
        if !once {
            steps.push("Watch `.nx` files and send changes to the running app.".to_owned());
        }
    }
    steps
}

fn selected_platforms(platform: &str) -> &'static [&'static str] {
    match platform {
        "ios" => &["ios"],
        "android" => &["android"],
        _ => &["ios", "android"],
    }
}

fn build_command(args: &[String]) -> Result<(), String> {
    eprintln!("`nexa build` is deprecated; use `nexa release`.");
    if args
        .iter()
        .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
    {
        print_command_help("build");
        return Ok(());
    }
    let release_args = release_build_arguments(args)?;
    native_command("release", &release_args)
}

fn release_build_arguments(args: &[String]) -> Result<Vec<String>, String> {
    let mut release_selected = false;
    let mut artifact: Option<&'static str> = None;
    let mut native_args = Vec::with_capacity(args.len());
    for argument in args {
        match argument.as_str() {
            "--release" => {
                if release_selected {
                    return Err("`--release` may only be specified once".to_owned());
                }
                release_selected = true;
            }
            "--ipa" | "--aab" => {
                let requested = if argument == "--ipa" { "ipa" } else { "aab" };
                if artifact.replace(requested).is_some() {
                    return Err("choose only one release artifact (`--ipa` or `--aab`)".to_owned());
                }
            }
            _ => native_args.push(argument.clone()),
        }
    }
    if !release_selected {
        return Err("`nexa build` requires `--release`; use `nexa release` instead".to_owned());
    }
    if let Some(artifact) = artifact {
        let (platform, platform_flag) = if artifact == "ipa" {
            ("ios", "--ios")
        } else {
            ("android", "--android")
        };
        match explicit_build_platform(&native_args) {
            Some(selected) if selected != platform => {
                return Err(format!(
                    "`--{artifact}` requires `--{platform}`, but `{selected}` was selected"
                ));
            }
            Some(_) => {}
            None => native_args.insert(0, platform_flag.to_owned()),
        }
    }
    Ok(native_args)
}

fn explicit_build_platform(args: &[String]) -> Option<&str> {
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--ios" => return Some("ios"),
            "--android" => return Some("android"),
            "--all" => return Some("all"),
            "--platform" | "-p" => return args.get(cursor + 1).map(String::as_str),
            _ => cursor += 1,
        }
    }
    None
}

fn run_in_language_tests(entry: &Path, platform: &str, locked: bool) -> Result<(), String> {
    let project_root = entry.parent().unwrap_or_else(|| Path::new("."));
    let config_path = project_root.join("nexa.config.nx");
    let dependencies = crate::config::load_plugin_dependencies(&config_path)?;
    let resolved = crate::dependencies::resolve(project_root, &dependencies)?;
    crate::dependencies::sync_lock(
        project_root,
        !dependencies.is_empty(),
        &resolved.lock_file,
        locked,
    )?;
    let targets = match platform {
        "ios" => vec![nexa_compiler::Target::Swift],
        "android" => vec![nexa_compiler::Target::Kotlin],
        "all" => vec![nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin],
        value => return Err(format!("unknown platform `{value}`")),
    };
    let compilations = nexa_compiler::compile_file_with_warnings_for_targets_and_plugin_roots(
        entry,
        &targets,
        &resolved.plugin_roots,
    )
    .map_err(|error| error.to_string())?;
    let mut warnings = std::collections::BTreeSet::new();
    for compilation in &compilations {
        for warning in &compilation.warnings {
            warnings.insert(warning.to_string());
        }
    }
    for warning in warnings {
        eprintln!("{warning}");
    }
    let Some(compilation) = compilations.into_iter().next() else {
        return Err("no compilation target was selected for in-language tests".to_owned());
    };
    report_in_language_tests(&compilation.tests, entry)
}

pub(crate) fn report_in_language_tests(
    suite: &nexa_compiler::testing::TestSuite,
    entry: &Path,
) -> Result<(), String> {
    if suite.tests.is_empty() {
        println!("No in-language `.nx` tests found.");
        return Ok(());
    }

    println!("Running {} in-language `.nx` test(s)", suite.tests.len());
    let report = nexa_compiler::testing::run_tests(suite);
    for name in report.passed_names {
        println!("PASS {name}");
    }
    for failure in &report.failures {
        let location = failure.source_file.as_deref().map_or_else(
            || {
                format!(
                    "{}:{}:{}",
                    entry.display(),
                    failure.span.line,
                    failure.span.column
                )
            },
            |file| format!("{file}:{}:{}", failure.span.line, failure.span.column),
        );
        eprintln!("FAIL {} ({location}): {}", failure.name, failure.message);
    }
    if report.failures.is_empty() {
        println!("{} passed; 0 failed", report.passed);
        Ok(())
    } else {
        Err(format!(
            "in-language tests failed: {} passed; {} failed",
            report.passed,
            report.failures.len()
        ))
    }
}

struct DevWatchContext<'a> {
    root: &'a Path,
    entry: &'a Path,
    platform: &'a str,
    project_name: &'a str,
    output: &'a Path,
    project_args: &'a [String],
    server: &'a nexa_dev_server::DevServer,
}

fn watch_sources(
    context: &DevWatchContext<'_>,
    background_tasks: &mut String,
    compiler: &mut nexa_compiler::IncrementalProjectCompiler,
    plugin_roots: &mut HashMap<String, PathBuf>,
) -> Result<(), String> {
    let mut performance_overlay_enabled = false;
    let mut runtime_trace_enabled = false;
    let mut native_rebuild_pending = false;
    let mut active_device: Option<(String, String)> = None;
    let (device_log_sender, device_log_receiver) = mpsc::channel();
    let mut dashboard = DevDashboard::new(context);
    let mut device_log_streams =
        start_device_log_streams(context, &device_log_sender, &mut dashboard);
    let mut log_deduplicator = DeviceLogDeduplicator::default();
    let (command_sender, console_commands, _raw_terminal) = start_dev_console_input();
    let _source_watcher =
        start_dev_source_watcher(context.root, plugin_roots, command_sender.clone())?;
    let translation_catalog_path = context.root.join("locales/translations.json");
    let mut translation_catalog_snapshot = read_translation_catalog(&translation_catalog_path);
    let mut pending_commands = VecDeque::new();
    loop {
        while let Ok(event) = device_log_receiver.try_recv() {
            match event {
                DeviceLogEvent::Line(line) => {
                    for output in log_deduplicator.push(line, Instant::now()) {
                        dashboard.device_log(output);
                    }
                }
                DeviceLogEvent::Ended {
                    platform,
                    device_id,
                } => dashboard.warning(format!(
                    "[{platform}] device log stream ended ({device_id})"
                )),
                DeviceLogEvent::Error {
                    platform,
                    device_id,
                    message,
                } => dashboard.warning(format!(
                    "[{platform}] device log stream failed ({device_id}): {message}"
                )),
            }
        }
        for output in log_deduplicator.flush_expired(Instant::now()) {
            dashboard.device_log(output);
        }
        let command = pending_commands.pop_front().or_else(|| {
            console_commands
                .recv_timeout(Duration::from_millis(50))
                .ok()
        });
        match command {
            Some(DevConsoleCommand::HotReload) => {
                dashboard.set_status("Reloading Nexa source".to_owned());
                dashboard.info("Hot reloading Nexa source...");
                match dashboard.with_terminal_output(|| {
                    reload_or_rebuild_dev_app(context, background_tasks, compiler, plugin_roots)
                }) {
                    Ok(true) => {
                        translation_catalog_snapshot =
                            read_translation_catalog(&translation_catalog_path);
                        dashboard.info("Native app rebuilt for background task changes.");
                        refresh_device_log_streams(
                            context,
                            &device_log_sender,
                            &mut device_log_streams,
                            &mut dashboard,
                        );
                        dashboard.set_status("Watching .nx sources".to_owned());
                    }
                    Ok(false) => {
                        translation_catalog_snapshot =
                            read_translation_catalog(&translation_catalog_path);
                        dashboard.info("Nexa source reloaded in the running app.");
                        dashboard.set_status("Watching .nx sources".to_owned());
                    }
                    Err(error) => {
                        dashboard.error(error.clone());
                        dashboard.set_status("Reload failed".to_owned());
                        publish_dev_error(context.server, context.entry, context.platform, error);
                    }
                }
            }
            Some(DevConsoleCommand::HotRestart) => {
                dashboard.set_status("Restarting app state".to_owned());
                dashboard.info("Hot restarting Nexa app state...");
                match dashboard.with_terminal_output(|| {
                    reload_or_rebuild_dev_app(context, background_tasks, compiler, plugin_roots)
                }) {
                    Ok(rebuilt) => {
                        translation_catalog_snapshot =
                            read_translation_catalog(&translation_catalog_path);
                        if !rebuilt {
                            context.server.publish_restart("hot restart requested");
                        } else {
                            refresh_device_log_streams(
                                context,
                                &device_log_sender,
                                &mut device_log_streams,
                                &mut dashboard,
                            );
                        }
                        dashboard.info("Nexa app state restarted in the running app.");
                        dashboard.set_status("Watching .nx sources".to_owned());
                    }
                    Err(error) => {
                        dashboard.error(error.clone());
                        dashboard.set_status("Restart failed".to_owned());
                        publish_dev_error(context.server, context.entry, context.platform, error);
                    }
                }
            }
            Some(DevConsoleCommand::Rebuild) => {
                dashboard.set_status("Rebuilding native app".to_owned());
                dashboard.info("Rebuilding native app...");
                let rebuilt = match dashboard
                    .with_terminal_output(|| rebuild_dev_app(context, compiler, plugin_roots))
                {
                    Ok(task_fingerprint) => {
                        *background_tasks = task_fingerprint;
                        dashboard.info("Native app rebuilt and relaunched.");
                        true
                    }
                    Err(error) => {
                        dashboard.error(format!("native dev rebuild failed: {error}"));
                        dashboard.set_status("Native rebuild failed".to_owned());
                        false
                    }
                };
                if rebuilt {
                    translation_catalog_snapshot =
                        read_translation_catalog(&translation_catalog_path);
                    refresh_device_log_streams(
                        context,
                        &device_log_sender,
                        &mut device_log_streams,
                        &mut dashboard,
                    );
                }
                if rebuilt {
                    native_rebuild_pending = false;
                    dashboard.set_status("Watching .nx sources".to_owned());
                }
                dashboard.render();
                continue;
            }
            Some(DevConsoleCommand::NextDevice) => {
                match dashboard.with_terminal_output(|| cycle_dev_device(context, &active_device)) {
                    Ok(device) => {
                        dashboard.info(format!(
                            "Selected [{}] {} ({})",
                            device.platform, device.name, device.id
                        ));
                        dashboard.set_device(&device);
                        active_device = Some((device.platform.clone(), device.id.clone()));
                        let platform = display_device_platform(&device.platform);
                        device_log_streams.retain(|stream| stream.platform != platform);
                        match start_device_log_stream(context, &device, device_log_sender.clone()) {
                            Ok(stream) => device_log_streams.push(stream),
                            Err(error) => dashboard.warning(format!("warning: {error}")),
                        }
                    }
                    Err(error) => dashboard.warning(format!("device selector: {error}")),
                }
            }
            Some(DevConsoleCommand::TogglePerformanceOverlay) => {
                performance_overlay_enabled = !performance_overlay_enabled;
                context
                    .server
                    .set_performance_overlay(performance_overlay_enabled);
                dashboard.set_overlay(performance_overlay_enabled);
                let state = if performance_overlay_enabled {
                    "on"
                } else {
                    "off"
                };
                dashboard.info(format!("Nexa performance overlay {state}."));
            }
            Some(DevConsoleCommand::ToggleRuntimeTrace) => {
                runtime_trace_enabled = !runtime_trace_enabled;
                context.server.set_runtime_trace(runtime_trace_enabled);
                let state = if runtime_trace_enabled { "on" } else { "off" };
                dashboard.info(format!(
                    "Nexa runtime trace {state}. Trace events omit values."
                ));
            }
            Some(DevConsoleCommand::ClearConsole) => {
                dashboard.clear();
            }
            Some(DevConsoleCommand::Stop) => {
                dashboard.finish("Stopping Nexa dev.");
                return Ok(());
            }
            Some(DevConsoleCommand::SourceChanges(paths)) => {
                let changed =
                    coalesce_source_changes(paths, &console_commands, &mut pending_commands);
                let changed = discard_unchanged_translation_catalog_event(
                    changed,
                    &translation_catalog_path,
                    translation_catalog_snapshot.as_deref(),
                );
                if changed.is_empty() {
                    continue;
                }
                let host_changed = changed
                    .iter()
                    .any(|path| requires_native_rebuild(path, plugin_roots));
                if host_changed {
                    if !native_rebuild_pending {
                        dashboard.info(
                            "Native host changes are waiting. Press 'b' to rebuild and relaunch the app.",
                        );
                    }
                    native_rebuild_pending = true;
                    dashboard.set_status("Native host changes waiting for rebuild".to_owned());
                } else if native_rebuild_pending {
                    continue;
                } else {
                    dashboard.set_status("Reloading changed .nx files".to_owned());
                    match dashboard.with_terminal_output(|| {
                        reload_or_rebuild_dev_app(context, background_tasks, compiler, plugin_roots)
                    }) {
                        Ok(true) => {
                            translation_catalog_snapshot =
                                read_translation_catalog(&translation_catalog_path);
                            native_rebuild_pending = false;
                            dashboard.info(
                                "Background task changes rebuilt and relaunched the native app.",
                            );
                            refresh_device_log_streams(
                                context,
                                &device_log_sender,
                                &mut device_log_streams,
                                &mut dashboard,
                            );
                            dashboard.set_status("Watching .nx sources".to_owned());
                        }
                        Ok(false) => {
                            translation_catalog_snapshot =
                                read_translation_catalog(&translation_catalog_path);
                            dashboard.info("Nexa source reloaded in the running app.");
                            dashboard.set_status("Watching .nx sources".to_owned());
                        }
                        Err(error) => {
                            dashboard.error(error.clone());
                            dashboard.set_status("Reload failed".to_owned());
                            publish_dev_error(
                                context.server,
                                context.entry,
                                context.platform,
                                error,
                            );
                        }
                    }
                }
            }
            Some(DevConsoleCommand::WatchError(error)) => {
                dashboard.warning(format!("file watcher: {error}"));
            }
            None => {}
        }
        if let Some(request) = context.server.try_recv_editor_request()
            && let Err(error) = open_editor_location(&request.file, request.line, request.column)
        {
            dashboard.warning(format!(
                "cannot open {}:{}:{} in an editor: {error}",
                request.file, request.line, request.column
            ));
        }
        dashboard.render();
    }
}

#[derive(Clone, Debug)]
struct DeviceLogLine {
    platform: String,
    device_id: String,
    text: String,
}

enum DeviceLogEvent {
    Line(DeviceLogLine),
    Ended {
        platform: String,
        device_id: String,
    },
    Error {
        platform: String,
        device_id: String,
        message: String,
    },
}

struct DashboardLine {
    prefix: String,
    text: String,
    color: Color,
}

struct DevDashboard {
    alternate_screen: bool,
    project_name: String,
    targets: String,
    server_address: String,
    status: String,
    overlay_enabled: bool,
    devices: String,
    selected_device: Option<String>,
    lines: VecDeque<DashboardLine>,
}

impl DevDashboard {
    const MAX_LINES: usize = 400;

    fn new(context: &DevWatchContext<'_>) -> Self {
        let mut dashboard = Self {
            alternate_screen: false,
            project_name: context.project_name.to_owned(),
            targets: match context.platform {
                "ios" => "iOS".to_owned(),
                "android" => "Android".to_owned(),
                _ => "iOS + Android".to_owned(),
            },
            server_address: context.server.address().to_string(),
            status: "Watching .nx sources".to_owned(),
            overlay_enabled: false,
            devices: "Detecting devices".to_owned(),
            selected_device: None,
            lines: VecDeque::new(),
        };
        if io::stdin().is_terminal() && io::stdout().is_terminal() {
            let mut stdout = io::stdout();
            if execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All)).is_ok() {
                dashboard.alternate_screen = true;
            }
        }
        dashboard.render();
        dashboard
    }

    fn info(&mut self, message: impl Into<String>) {
        let message = message.into();
        if self.alternate_screen {
            self.push_line("INFO", message, Color::Cyan);
            self.render();
        } else {
            println!("{message}");
        }
    }

    fn warning(&mut self, message: impl Into<String>) {
        let message = message.into();
        if self.alternate_screen {
            self.push_line("WARN", message, Color::Yellow);
            self.render();
        } else {
            eprintln!("{message}");
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        let message = message.into();
        if self.alternate_screen {
            self.push_line("ERROR", message, Color::Red);
            self.render();
        } else {
            eprintln!("{message}");
        }
    }

    fn device_log(&mut self, line: DeviceLogLine) {
        if !self.alternate_screen {
            print_device_log(&line);
            return;
        }
        let color = if line.platform.contains('+') {
            Color::Magenta
        } else if line.platform == "iOS" {
            Color::Cyan
        } else {
            Color::Green
        };
        self.push_line(format!("[{}]", line.platform), line.text, color);
    }

    fn push_line(&mut self, prefix: impl Into<String>, text: impl Into<String>, color: Color) {
        self.lines.push_back(DashboardLine {
            prefix: prefix.into(),
            text: text.into(),
            color,
        });
        while self.lines.len() > Self::MAX_LINES {
            self.lines.pop_front();
        }
    }

    fn set_status(&mut self, status: String) {
        self.status = status;
        self.render();
    }

    fn set_overlay(&mut self, enabled: bool) {
        self.overlay_enabled = enabled;
        self.render();
    }

    fn set_devices(&mut self, devices: &[DevDevice]) {
        self.devices = if devices.is_empty() {
            "No connected devices".to_owned()
        } else {
            devices
                .iter()
                .map(|device| {
                    format!(
                        "{}: {}",
                        display_device_platform(&device.platform),
                        device.name
                    )
                })
                .collect::<Vec<_>>()
                .join(" · ")
        };
        self.render();
    }

    fn set_device(&mut self, device: &DevDevice) {
        self.selected_device = Some(format!(
            "{}: {}",
            display_device_platform(&device.platform),
            device.name
        ));
        self.render();
    }

    fn clear(&mut self) {
        self.lines.clear();
        if self.alternate_screen {
            self.render();
        } else if io::stdout().is_terminal() {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, Clear(ClearType::All), MoveTo(0, 0));
            print_dev_welcome();
        }
    }

    fn with_terminal_output<T>(&mut self, operation: impl FnOnce() -> T) -> T {
        let resume_dashboard = self.alternate_screen;
        if resume_dashboard {
            self.leave_screen();
        }
        let result = operation();
        if resume_dashboard {
            self.enter_screen();
            self.render();
        }
        result
    }

    fn finish(&mut self, message: &str) {
        self.leave_screen();
        println!("{message}");
    }

    fn enter_screen(&mut self) {
        if self.alternate_screen {
            return;
        }
        let mut stdout = io::stdout();
        if execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All)).is_ok() {
            self.alternate_screen = true;
        }
    }

    fn leave_screen(&mut self) {
        if !self.alternate_screen {
            return;
        }
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, ResetColor, LeaveAlternateScreen);
        self.alternate_screen = false;
    }

    fn render(&self) {
        if !self.alternate_screen {
            return;
        }
        let (width, height) = size().unwrap_or((80, 24));
        let mut stdout = io::stdout();
        let _ = self.render_into(&mut stdout, width, height);
    }

    fn render_into(&self, stdout: &mut impl Write, width: u16, height: u16) -> io::Result<()> {
        let width = width.max(1);
        let height = height.max(1);
        queue!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;
        if height < 6 {
            write_dashboard_row(
                stdout,
                0,
                width,
                "NEXA DEV",
                Color::Cyan,
                "Resize terminal for dashboard",
            )?;
            return stdout.flush();
        }
        write_dashboard_row(
            stdout,
            0,
            width,
            "NEXA DEV",
            Color::Cyan,
            &format!("{} · native mobile preview", self.project_name),
        )?;
        write_dashboard_row(
            stdout,
            1,
            width,
            "TARGETS",
            Color::Blue,
            &format!("{}  |  Devices: {}", self.targets, self.devices),
        )?;
        let selected = self
            .selected_device
            .as_ref()
            .map_or(String::new(), |device| format!("  |  Selected: {device}"));
        let overlay = if self.overlay_enabled { "ON" } else { "OFF" };
        let status_color = if self.status.contains("failed") {
            Color::Red
        } else if self.status.contains("waiting") || self.status.contains("Rebuilding") {
            Color::Yellow
        } else {
            Color::Green
        };
        write_dashboard_row(
            stdout,
            2,
            width,
            "STATUS",
            status_color,
            &format!(
                "{}  |  Performance overlay: {overlay}{selected}  |  ws://{}",
                self.status, self.server_address
            ),
        )?;
        write_dashboard_row(
            stdout,
            3,
            width,
            "",
            Color::DarkGrey,
            &"─".repeat(width as usize),
        )?;

        let log_rows = height.saturating_sub(6) as usize;
        let start = self.lines.len().saturating_sub(log_rows);
        for (index, line) in self.lines.iter().skip(start).enumerate() {
            let row = 4usize.saturating_add(index).min(u16::MAX as usize) as u16;
            write_dashboard_row(stdout, row, width, &line.prefix, line.color, &line.text)?;
        }

        let footer_top = height.saturating_sub(2);
        write_dashboard_row(
            stdout,
            footer_top,
            width,
            "",
            Color::DarkGrey,
            &"─".repeat(width as usize),
        )?;
        write_dashboard_row(
            stdout,
            height.saturating_sub(1),
            width,
            "KEYS",
            Color::Yellow,
            "r reload  R restart  b rebuild  d device  p overlay  l clear  q quit",
        )?;
        stdout.flush()
    }
}

impl Drop for DevDashboard {
    fn drop(&mut self) {
        self.leave_screen();
    }
}

fn write_dashboard_row(
    stdout: &mut impl Write,
    row: u16,
    width: u16,
    prefix: &str,
    color: Color,
    text: &str,
) -> io::Result<()> {
    let prefix = sanitize_terminal_text(prefix);
    let text = sanitize_terminal_text(text);
    let prefix_width = prefix.chars().count().min(width as usize);
    let separator_width = if prefix.is_empty() { 0 } else { 1 };
    let available = width.saturating_sub(prefix_width as u16 + separator_width) as usize;
    let prefix = clip_terminal_text(&prefix, width as usize);
    let text = clip_terminal_text(&text, available);
    queue!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(color),
        Print(prefix),
        ResetColor,
        Print(if text.is_empty() || prefix_width == 0 {
            ""
        } else {
            " "
        }),
        Print(text),
        Clear(ClearType::UntilNewLine)
    )
}

fn clip_terminal_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn sanitize_terminal_text(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && character != '\t' {
                '�'
            } else {
                character
            }
        })
        .collect()
}

struct DeviceLogStream {
    platform: String,
    child: Child,
    readers: Vec<thread::JoinHandle<()>>,
    stopping: Arc<AtomicBool>,
}

impl Drop for DeviceLogStream {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        let _ = self.child.kill();
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

fn start_device_log_streams(
    context: &DevWatchContext<'_>,
    sender: &Sender<DeviceLogEvent>,
    dashboard: &mut DevDashboard,
) -> Vec<DeviceLogStream> {
    let mut devices = Vec::new();
    if matches!(context.platform, "ios" | "all") {
        match available_ios_devices() {
            Ok(available) => {
                if let Some(device) = available
                    .iter()
                    .find(|device| device.booted)
                    .or_else(|| available.first())
                {
                    devices.push(device.clone());
                }
            }
            Err(error) => {
                dashboard.warning(format!("warning: cannot start iOS device logs: {error}"))
            }
        }
    }
    if matches!(context.platform, "android" | "all") {
        match available_android_devices() {
            Ok(available) => {
                if let Some(device) = available.into_iter().next() {
                    devices.push(device);
                }
            }
            Err(error) => dashboard.warning(format!(
                "warning: cannot start Android device logs: {error}"
            )),
        }
    }
    dashboard.set_devices(&devices);

    devices
        .iter()
        .filter_map(
            |device| match start_device_log_stream(context, device, sender.clone()) {
                Ok(stream) => Some(stream),
                Err(error) => {
                    dashboard.warning(format!("warning: {error}"));
                    None
                }
            },
        )
        .collect()
}

fn refresh_device_log_streams(
    context: &DevWatchContext<'_>,
    sender: &Sender<DeviceLogEvent>,
    streams: &mut Vec<DeviceLogStream>,
    dashboard: &mut DevDashboard,
) {
    streams.clear();
    streams.extend(start_device_log_streams(context, sender, dashboard));
}

fn start_device_log_stream(
    context: &DevWatchContext<'_>,
    device: &DevDevice,
    sender: Sender<DeviceLogEvent>,
) -> Result<DeviceLogStream, String> {
    let (mut command, pid_filter) = match device.platform.as_str() {
        "ios" => {
            let predicate = format!(
                "process == \"{}\" OR senderImagePath CONTAINS \"/{}.app/\"",
                context.project_name, context.project_name
            );
            let mut command = Command::new("xcrun");
            command.args([
                "simctl",
                "spawn",
                &device.id,
                "log",
                "stream",
                "--style",
                "compact",
                "--level",
                "info",
                "--predicate",
                &predicate,
            ]);
            (command, None)
        }
        "android" => {
            let package = android_application_id(context.output)?;
            let pid = android_app_process_id(device, &package)?;
            let mut command = Command::new("adb");
            command.args(["-s", &device.id, "logcat", "-v", "brief", "*:V"]);
            (command, Some(pid))
        }
        platform => return Err(format!("unsupported device log platform `{platform}`")),
    };

    let platform = display_device_platform(&device.platform).to_owned();
    let device_id = device.id.clone();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "cannot start {platform} device logs for {} ({device_id}): {error}",
                device.name
            )
        })?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("{platform} device log output is unavailable"));
    };
    let Some(stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("{platform} device log diagnostics are unavailable"));
    };

    let stdout_platform = platform.clone();
    let stdout_device = device_id.clone();
    let stdout_sender = sender.clone();
    let stopping = Arc::new(AtomicBool::new(false));
    let stdout_stopping = Arc::clone(&stopping);
    let reader = thread::Builder::new()
        .name(format!("nexa-{}-logs", device.platform))
        .spawn(move || {
            for line in io::BufReader::new(stdout).lines() {
                match line {
                    Ok(text)
                        if pid_filter
                            .as_deref()
                            .is_none_or(|pid| android_log_line_matches_pid(&text, pid)) =>
                    {
                        if stdout_sender
                            .send(DeviceLogEvent::Line(DeviceLogLine {
                                platform: stdout_platform.clone(),
                                device_id: stdout_device.clone(),
                                text,
                            }))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        let _ = stdout_sender.send(DeviceLogEvent::Error {
                            platform: stdout_platform.clone(),
                            device_id: stdout_device.clone(),
                            message: error.to_string(),
                        });
                        break;
                    }
                }
            }
            if !stdout_stopping.load(Ordering::Acquire) {
                let _ = stdout_sender.send(DeviceLogEvent::Ended {
                    platform: stdout_platform,
                    device_id: stdout_device,
                });
            }
        })
        .map_err(|error| {
            let _ = child.kill();
            let _ = child.wait();
            format!("cannot read {platform} device logs: {error}")
        })?;

    let stderr_platform = platform.clone();
    let stderr_device = device_id.clone();
    let stderr_sender = sender;
    let stderr_reader = match thread::Builder::new()
        .name(format!("nexa-{}-log-errors", device.platform))
        .spawn(move || {
            let mut message = String::new();
            if let Err(error) = io::BufReader::new(stderr).read_to_string(&mut message) {
                let _ = stderr_sender.send(DeviceLogEvent::Error {
                    platform: stderr_platform,
                    device_id: stderr_device,
                    message: error.to_string(),
                });
            } else if !message.trim().is_empty() {
                let _ = stderr_sender.send(DeviceLogEvent::Error {
                    platform: stderr_platform,
                    device_id: stderr_device,
                    message: message.trim().to_owned(),
                });
            }
        }) {
        Ok(reader) => reader,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(format!(
                "cannot read {platform} device log diagnostics: {error}"
            ));
        }
    };

    Ok(DeviceLogStream {
        platform,
        child,
        readers: vec![reader, stderr_reader],
        stopping,
    })
}

fn display_device_platform(platform: &str) -> &str {
    match platform {
        "ios" => "iOS",
        "android" => "Android",
        other => other,
    }
}

fn parse_android_process_id(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .find(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .map(str::to_owned)
}

fn android_app_process_id(device: &DevDevice, package: &str) -> Result<String, String> {
    let mut last_detail = String::new();
    for attempt in 0..10 {
        let output = Command::new("adb")
            .args(["-s", &device.id, "shell", "pidof", package])
            .output()
            .map_err(|error| {
                format!(
                    "cannot find the Android app process on {}: {error}",
                    device.name
                )
            })?;
        if output.status.success()
            && let Some(pid) = parse_android_process_id(&String::from_utf8_lossy(&output.stdout))
        {
            return Ok(pid);
        }
        last_detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if attempt < 9 {
            thread::sleep(Duration::from_millis(100));
        }
    }
    if last_detail.is_empty() {
        Err(format!(
            "Android app process `{package}` is not running on {}",
            device.name
        ))
    } else {
        Err(format!(
            "cannot find the Android app process `{package}` on {}: {last_detail}",
            device.name
        ))
    }
}

fn android_log_line_matches_pid(line: &str, pid: &str) -> bool {
    let Some(open) = line.find('(') else {
        return false;
    };
    let Some(close_offset) = line[open + 1..].find(')') else {
        return false;
    };
    line[open + 1..open + 1 + close_offset].trim() == pid
}

#[derive(Default)]
struct DeviceLogDeduplicator {
    recent: HashMap<String, RecentDeviceLog>,
}

struct RecentDeviceLog {
    message: String,
    platforms: BTreeSet<String>,
    device_ids: BTreeSet<String>,
    last_seen: Instant,
    repeats: usize,
}

impl DeviceLogDeduplicator {
    const WINDOW: Duration = Duration::from_millis(1500);

    fn push(&mut self, line: DeviceLogLine, now: Instant) -> Vec<DeviceLogLine> {
        let message = normalize_device_log(&line);
        if let Some(recent) = self.recent.get_mut(&message)
            && now.duration_since(recent.last_seen) <= Self::WINDOW
        {
            recent.platforms.insert(line.platform);
            recent.device_ids.insert(line.device_id);
            recent.last_seen = now;
            recent.repeats += 1;
            return Vec::new();
        }

        let mut output = self
            .recent
            .remove(&message)
            .and_then(recent_device_log_summary)
            .into_iter()
            .collect::<Vec<_>>();
        self.recent.insert(
            message,
            RecentDeviceLog {
                message: normalize_device_log(&line),
                platforms: BTreeSet::from([line.platform.clone()]),
                device_ids: BTreeSet::from([line.device_id.clone()]),
                last_seen: now,
                repeats: 0,
            },
        );
        output.push(line);
        output
    }

    fn flush_expired(&mut self, now: Instant) -> Vec<DeviceLogLine> {
        let expired = self
            .recent
            .iter()
            .filter(|(_, recent)| now.duration_since(recent.last_seen) > Self::WINDOW)
            .map(|(message, _)| message.clone())
            .collect::<Vec<_>>();
        expired
            .into_iter()
            .filter_map(|message| self.recent.remove(&message))
            .filter_map(recent_device_log_summary)
            .collect()
    }
}

fn recent_device_log_summary(recent: RecentDeviceLog) -> Option<DeviceLogLine> {
    (recent.repeats > 0).then(|| DeviceLogLine {
        platform: recent.platforms.into_iter().collect::<Vec<_>>().join("+"),
        device_id: recent.device_ids.into_iter().collect::<Vec<_>>().join(","),
        text: format!("{} (repeated {} times)", recent.message, recent.repeats + 1),
    })
}

fn normalize_device_log(line: &DeviceLogLine) -> String {
    if line.platform == "Android"
        && let Some((header, message)) = line.text.split_once(": ")
    {
        let tag = header
            .rsplit_once('/')
            .map_or(header, |(_, tag)| tag)
            .split('(')
            .next()
            .unwrap_or_default()
            .trim();
        if !tag.is_empty() {
            return format!("{tag}: {}", message.trim());
        }
    }
    if line.platform == "iOS"
        && let Some(index) = line.text.find("[Nexa]")
    {
        let message = line.text[index + "[Nexa]".len()..].trim_start();
        let message = message
            .strip_prefix('[')
            .and_then(|message| message.split_once(']'))
            .map_or(message, |(_, remainder)| remainder.trim_start());
        return format!("Nexa: {}", message.trim());
    }
    line.text.trim().to_owned()
}

fn print_device_log(line: &DeviceLogLine) {
    let text = line
        .text
        .chars()
        .map(|character| {
            if character.is_control() && character != '\t' {
                '�'
            } else {
                character
            }
        })
        .collect::<String>();
    let mut stdout = io::stdout();
    if stdout.is_terminal() {
        let color = if line.platform.contains('+') {
            Color::Magenta
        } else if line.platform == "iOS" {
            Color::Cyan
        } else {
            Color::Green
        };
        if execute!(
            stdout,
            SetForegroundColor(color),
            Print(format!("[{}]", line.platform)),
            ResetColor,
            Print(" "),
            Print(text.as_str()),
            Print("\r\n")
        )
        .is_ok()
        {
            return;
        }
    }
    println!("[{}] {text}", line.platform);
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DevConsoleCommand {
    HotReload,
    HotRestart,
    Rebuild,
    NextDevice,
    TogglePerformanceOverlay,
    ToggleRuntimeTrace,
    ClearConsole,
    Stop,
    SourceChanges(Vec<PathBuf>),
    WatchError(String),
}

fn print_dev_welcome() {
    println!(
        "\nWelcome to Nexa dev\n  r          Hot reload the current source\n  Shift+R    Hot restart and reset app state\n  b          Rebuild and relaunch the native app\n  d          Cycle to the next connected iOS or Android device\n  p          Toggle the performance overlay (FPS and frame time)\n  t          Toggle redacted runtime trace events\n  l          Clear the console\n  q          Stop the dev session\n  Ctrl-C     Stop the dev session\n\nBuild and device logs will appear below.\n"
    );
}

struct RawTerminalGuard;

impl RawTerminalGuard {
    fn enable() -> Result<Self, String> {
        enable_raw_mode()
            .map_err(|error| format!("cannot enable Nexa dev keyboard shortcuts: {error}"))?;
        Ok(Self)
    }
}

impl Drop for RawTerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn start_dev_console_input() -> (
    Sender<DevConsoleCommand>,
    Receiver<DevConsoleCommand>,
    Option<RawTerminalGuard>,
) {
    let (sender, receiver) = mpsc::channel();
    let input_sender = sender.clone();
    if io::stdin().is_terminal() {
        match RawTerminalGuard::enable() {
            Ok(guard) => {
                thread::spawn(move || {
                    loop {
                        match event::poll(Duration::from_millis(200)) {
                            Ok(false) => continue,
                            Err(error) => {
                                eprintln!("Nexa dev keyboard input stopped: {error}");
                                break;
                            }
                            Ok(true) => {}
                        }
                        let Ok(Event::Key(key)) = event::read() else {
                            continue;
                        };
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        let command = match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                Some(DevConsoleCommand::Stop)
                            }
                            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                                Some(DevConsoleCommand::HotRestart)
                            }
                            KeyCode::Char('R') => Some(DevConsoleCommand::HotRestart),
                            KeyCode::Char('r') => Some(DevConsoleCommand::HotReload),
                            KeyCode::Char('b' | 'B') => Some(DevConsoleCommand::Rebuild),
                            KeyCode::Char('d' | 'D') => Some(DevConsoleCommand::NextDevice),
                            KeyCode::Char('p' | 'P') => {
                                Some(DevConsoleCommand::TogglePerformanceOverlay)
                            }
                            KeyCode::Char('t' | 'T') => Some(DevConsoleCommand::ToggleRuntimeTrace),
                            KeyCode::Char('l' | 'L') => Some(DevConsoleCommand::ClearConsole),
                            KeyCode::Char('q' | 'Q') => Some(DevConsoleCommand::Stop),
                            _ => None,
                        };
                        if let Some(command) = command {
                            let stop = command == DevConsoleCommand::Stop;
                            if input_sender.send(command).is_err() || stop {
                                break;
                            }
                        }
                    }
                });
                return (sender, receiver, Some(guard));
            }
            Err(error) => eprintln!("{error}; using line input instead."),
        }
    }
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let command = match line.trim() {
                "r" => Some(DevConsoleCommand::HotReload),
                "R" => Some(DevConsoleCommand::HotRestart),
                "b" | "B" => Some(DevConsoleCommand::Rebuild),
                "d" | "D" => Some(DevConsoleCommand::NextDevice),
                "p" | "P" => Some(DevConsoleCommand::TogglePerformanceOverlay),
                "t" | "T" => Some(DevConsoleCommand::ToggleRuntimeTrace),
                "l" | "L" => Some(DevConsoleCommand::ClearConsole),
                "q" | "quit" | "exit" => Some(DevConsoleCommand::Stop),
                "" => None,
                value => {
                    eprintln!("unknown Nexa dev shortcut `{value}`; use r, R, b, d, p, t, l, or q");
                    None
                }
            };
            if let Some(command) = command {
                let stop = command == DevConsoleCommand::Stop;
                if input_sender.send(command).is_err() || stop {
                    break;
                }
            }
        }
    });
    (sender, receiver, None)
}

fn start_dev_source_watcher(
    root: &Path,
    plugin_roots: &HashMap<String, PathBuf>,
    sender: Sender<DevConsoleCommand>,
) -> Result<notify::RecommendedWatcher, String> {
    let project_root = fs::canonicalize(root)
        .map_err(|error| format!("cannot watch {}: {error}", root.display()))?;
    let mut canonical_plugin_roots = Vec::new();
    for path in plugin_roots.values() {
        if !path.is_dir() {
            continue;
        }
        let path = fs::canonicalize(path)
            .map_err(|error| format!("cannot watch plugin {}: {error}", path.display()))?;
        if !canonical_plugin_roots.contains(&path) {
            canonical_plugin_roots.push(path);
        }
    }
    let watched_plugin_roots = canonical_plugin_roots.clone();
    let external_plugin_roots = canonical_plugin_roots
        .into_iter()
        .filter(|path| !path.starts_with(&project_root))
        .collect::<Vec<_>>();
    let callback_root = project_root.clone();
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) if !matches!(event.kind, EventKind::Access(_)) => {
                let paths = event
                    .paths
                    .into_iter()
                    .filter(|path| {
                        is_relevant_dev_path(path, &callback_root, &watched_plugin_roots)
                    })
                    .collect::<Vec<_>>();
                if !paths.is_empty() {
                    let _ = sender.send(DevConsoleCommand::SourceChanges(paths));
                }
            }
            Ok(_) => {}
            Err(error) => {
                let _ = sender.send(DevConsoleCommand::WatchError(error.to_string()));
            }
        })
        .map_err(|error| format!("cannot start Nexa file watcher: {error}"))?;

    watcher
        .watch(&project_root, RecursiveMode::Recursive)
        .map_err(|error| format!("cannot watch {}: {error}", project_root.display()))?;
    for path in external_plugin_roots {
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|error| format!("cannot watch plugin {}: {error}", path.display()))?;
    }
    Ok(watcher)
}

fn coalesce_source_changes(
    paths: Vec<PathBuf>,
    receiver: &Receiver<DevConsoleCommand>,
    pending: &mut VecDeque<DevConsoleCommand>,
) -> Vec<PathBuf> {
    // Editors often save one file through multiple writes or a temporary-file
    // rename. Wait for the complete save burst before compiling, even when the
    // first notification names only one path.
    let quiet_window = Duration::from_millis(40);
    let mut changed = paths.into_iter().collect::<BTreeSet<_>>();
    let mut deadline = Instant::now() + quiet_window;
    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        match receiver.recv_timeout(deadline - now) {
            Ok(DevConsoleCommand::SourceChanges(paths)) => {
                changed.extend(paths);
                deadline = Instant::now() + quiet_window;
            }
            Ok(command) => pending.push_back(command),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    changed.into_iter().collect()
}

fn read_translation_catalog(path: &Path) -> Option<Vec<u8>> {
    fs::read(path).ok()
}

fn discard_unchanged_translation_catalog_event(
    paths: Vec<PathBuf>,
    catalog_path: &Path,
    compiled_snapshot: Option<&[u8]>,
) -> Vec<PathBuf> {
    let Some(compiled_snapshot) = compiled_snapshot else {
        return paths;
    };

    paths
        .into_iter()
        .filter(|path| {
            path != catalog_path
                || fs::read(path).map_or(true, |current| current.as_slice() != compiled_snapshot)
        })
        .collect()
}

fn reload_or_rebuild_dev_app(
    context: &DevWatchContext<'_>,
    background_tasks: &mut String,
    compiler: &mut nexa_compiler::IncrementalProjectCompiler,
    plugin_roots: &mut HashMap<String, PathBuf>,
) -> Result<bool, String> {
    let compile_started = Instant::now();
    let modules = super::project::compile_dev_modules_with_compiler(
        context.entry,
        context.platform,
        plugin_roots,
        compiler,
    )?;
    let compile_elapsed = compile_started.elapsed();
    let next_fingerprint = background_task_fingerprint(&modules)?;
    if next_fingerprint != *background_tasks {
        println!(
            "Background task declarations or actions changed; rebuilding the native host because the OS runs compiled handlers."
        );
        *background_tasks = rebuild_dev_app(context, compiler, plugin_roots)?;
        return Ok(true);
    }
    let publish_started = Instant::now();
    for (target, module) in modules {
        context.server.publish_module(target, module)?;
    }
    let publish_elapsed = publish_started.elapsed();
    let stats = compiler.last_compile_stats();
    println!(
        "Nexa dev reload: compile/IR {} ms, publish {} ms ({} parsed, {} reused source files)",
        compile_elapsed.as_millis(),
        publish_elapsed.as_millis(),
        stats.parsed_source_files,
        stats.reused_source_files,
    );
    Ok(false)
}

fn rebuild_dev_app(
    context: &DevWatchContext<'_>,
    compiler: &mut nexa_compiler::IncrementalProjectCompiler,
    plugin_roots: &mut HashMap<String, PathBuf>,
) -> Result<String, String> {
    let dependencies =
        crate::config::load_plugin_dependencies(&context.root.join("nexa.config.nx"))?;
    let refreshed_plugin_roots =
        crate::dependencies::resolve(context.root, &dependencies)?.plugin_roots;
    let modules = super::project::compile_dev_modules_with_compiler(
        context.entry,
        context.platform,
        &refreshed_plugin_roots,
        compiler,
    )?;
    let task_fingerprint = background_task_fingerprint(&modules)?;
    super::project::run(context.project_args)?;
    build_platforms(
        context.output,
        context.project_name,
        context.platform,
        BuildMode::Dev,
        Some(context.server.address().port()),
    )?;
    for (target, module) in modules {
        context.server.publish_module(target, module)?;
    }
    *plugin_roots = refreshed_plugin_roots;
    Ok(task_fingerprint)
}

fn background_task_fingerprint(
    modules: &[(nexa_dev_protocol::TargetPlatform, nexa_dev_ir::DevModule)],
) -> Result<String, String> {
    let task_sets = modules
        .iter()
        .map(|(_, module)| &module.module.background_tasks)
        .collect::<Vec<_>>();
    serde_json::to_string(&task_sets)
        .map_err(|error| format!("could not fingerprint background tasks: {error}"))
}

#[cfg(test)]
fn changed_paths(
    previous: &BTreeMap<PathBuf, (u64, u64)>,
    current: &BTreeMap<PathBuf, (u64, u64)>,
) -> Vec<PathBuf> {
    let mut changed = previous
        .keys()
        .chain(current.keys())
        .filter(|path| previous.get(*path) != current.get(*path))
        .cloned()
        .collect::<Vec<_>>();
    changed.sort();
    changed.dedup();
    changed
}

fn publish_dev_error(
    server: &nexa_dev_server::DevServer,
    entry: &Path,
    platform: &str,
    message: String,
) {
    let diagnostic = parse_dev_diagnostic(entry, &message);
    for target in dev_targets(platform) {
        let _ = server.publish_diagnostics(target, vec![diagnostic.clone()]);
    }
}

fn parse_dev_diagnostic(entry: &Path, message: &str) -> nexa_dev_protocol::Diagnostic {
    let parsed = message.lines().find_map(|line| {
        let (location, detail) = line.split_once(": error: ")?;
        let mut parts = location.rsplitn(3, ':');
        let column = parts.next()?.parse::<u32>().ok()?;
        let line_number = parts.next()?.parse::<u32>().ok()?;
        let file = parts.next()?;
        Some((file, line_number, column, detail))
    });
    let (file, line, column, detail) =
        parsed.unwrap_or((entry.to_str().unwrap_or("<unknown>"), 1, 1, message));
    nexa_dev_protocol::Diagnostic {
        severity: nexa_dev_protocol::Severity::Error,
        file: file.to_owned(),
        line,
        column,
        message: detail.to_owned(),
    }
}

fn open_editor_location(file: &str, line: u32, column: u32) -> Result<(), String> {
    let source_path = PathBuf::from(file);
    let absolute_path = std::fs::canonicalize(&source_path).unwrap_or_else(|_| {
        if source_path.is_absolute() {
            source_path
        } else {
            env::current_dir()
                .map(|directory| directory.join(&source_path))
                .unwrap_or(source_path)
        }
    });
    let path = absolute_path.to_string_lossy().replace('\\', "/");
    let encoded_path = path
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<String>();
    let uri = format!(
        "vscode://file/{}:{line}:{column}",
        encoded_path.trim_start_matches('/')
    );
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(&uri);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", &uri]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(&uri);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not launch the system URL handler for {uri}: {error}"))
}

fn dev_targets(platform: &str) -> Vec<nexa_dev_protocol::TargetPlatform> {
    match platform {
        "ios" => vec![nexa_dev_protocol::TargetPlatform::Ios],
        "android" => vec![nexa_dev_protocol::TargetPlatform::Android],
        _ => vec![
            nexa_dev_protocol::TargetPlatform::Ios,
            nexa_dev_protocol::TargetPlatform::Android,
        ],
    }
}

fn is_relevant_dev_path(path: &Path, project_root: &Path, plugin_roots: &[PathBuf]) -> bool {
    let in_plugin = plugin_roots
        .iter()
        .map(PathBuf::as_path)
        .find(|plugin_root| path.starts_with(plugin_root));
    let Some(base) = in_plugin.or_else(|| path.starts_with(project_root).then_some(project_root))
    else {
        return false;
    };
    if has_excluded_dev_component(path, base) {
        return false;
    }
    if in_plugin.is_some() {
        return true;
    }
    path.file_name()
        .is_some_and(|name| name == "nexa.lock" || name == "nexa.config.nx")
        || path.extension().is_some_and(|extension| extension == "nx")
        || (path
            .file_name()
            .is_some_and(|name| name == "translations.json")
            && path
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == "locales"))
        || is_native_asset(path)
}

fn acquire_dev_session_lock(project_root: &Path) -> Result<fs::File, String> {
    let directory = project_root.join(".nexa");
    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "cannot create Nexa dev session lock directory {}: {error}",
            directory.display()
        )
    })?;
    let path = directory.join("dev-session.lock");
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| {
            format!(
                "cannot open Nexa dev session lock {}: {error}",
                path.display()
            )
        })?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(fs::TryLockError::WouldBlock) => Err(format!(
            "another Nexa dev session is already active for this project ({}); stop it before starting a second host",
            path.display()
        )),
        Err(fs::TryLockError::Error(error)) => Err(format!(
            "cannot acquire Nexa dev session lock {}: {error}",
            path.display()
        )),
    }
}

fn has_excluded_dev_component(path: &Path, base: &Path) -> bool {
    path.strip_prefix(base).is_ok_and(|relative| {
        relative.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".git" | ".nexa" | "build" | "target" | "ios-derived" | ".gradle")
            )
        })
    })
}

fn requires_native_rebuild(path: &Path, plugin_roots: &HashMap<String, PathBuf>) -> bool {
    path.file_name()
        .is_some_and(|name| name == "nexa.lock" || name == "nexa.config.nx")
        || is_native_asset(path)
        || plugin_roots
            .values()
            .any(|plugin_root| path.starts_with(plugin_root))
}

#[cfg(test)]
fn source_fingerprint(
    root: &Path,
    plugin_roots: &[PathBuf],
) -> Result<BTreeMap<PathBuf, (u64, u64)>, String> {
    fn visit(
        directory: &Path,
        include_all_files: bool,
        result: &mut BTreeMap<PathBuf, (u64, u64)>,
    ) -> Result<(), String> {
        for entry in
            fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
            let path = entry.path();
            let name = entry.file_name();
            if matches!(
                name.to_str(),
                Some(".git" | ".nexa" | "build" | "target" | "ios-derived" | ".gradle")
            ) {
                continue;
            }
            if path.is_dir() {
                let bundle_directory = matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("icon" | "xcassets")
                );
                visit(&path, include_all_files || bundle_directory, result)?;
            } else if include_all_files
                || entry.file_name() == "nexa.lock"
                || (entry.file_name() == "translations.json"
                    && path
                        .parent()
                        .and_then(Path::file_name)
                        .is_some_and(|name| name == "locales"))
                || path.extension().is_some_and(|extension| extension == "nx")
                || is_native_asset(&path)
            {
                let metadata = entry
                    .metadata()
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |value| value.as_nanos() as u64);
                result.insert(path, (metadata.len(), modified));
            }
        }
        Ok(())
    }

    let mut result = BTreeMap::new();
    visit(root, false, &mut result)?;
    for plugin_root in plugin_roots {
        if plugin_root.is_dir() {
            visit(plugin_root, true, &mut result)?;
        }
    }
    Ok(result)
}

fn is_native_asset(path: &Path) -> bool {
    if path.ancestors().any(|ancestor| {
        matches!(
            ancestor
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("icon" | "xcassets")
        )
    }) {
        return true;
    }
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "pdf" | "svg")
    )
}

#[derive(Clone, Copy)]
enum BuildMode {
    Dev,
    DevCompile,
    Test,
    Release,
}

fn build_platforms(
    root: &Path,
    app_name: &str,
    platform: &str,
    mode: BuildMode,
    dev_port: Option<u16>,
) -> Result<(), String> {
    if matches!(platform, "ios" | "all") {
        build_ios(root, app_name, mode)?;
    }
    if matches!(platform, "android" | "all") {
        build_android(root, mode, dev_port)?;
    }
    Ok(())
}

fn build_ios(root: &Path, app_name: &str, mode: BuildMode) -> Result<(), String> {
    let archive = root
        .join("artifacts/ios")
        .join(format!("{app_name}.xcarchive"));
    if matches!(mode, BuildMode::Release) {
        remove_generated_path(&archive)?;
    }
    let manual_profile = if matches!(mode, BuildMode::Release) {
        match env::var("NEXA_IOS_PROVISIONING_PROFILE") {
            Ok(profile) if !profile.trim().is_empty() => Some(profile),
            Ok(_) => {
                return Err("NEXA_IOS_PROVISIONING_PROFILE cannot be empty".to_owned());
            }
            Err(env::VarError::NotPresent) => None,
            Err(error) => return Err(format!("invalid NEXA_IOS_PROVISIONING_PROFILE: {error}")),
        }
    } else {
        None
    };
    let project = root.join("ios").join(format!("{app_name}.xcodeproj"));
    let mut command = Command::new("xcodebuild");
    command
        .arg("-project")
        .arg(&project)
        .arg("-scheme")
        .arg(app_name);
    match mode {
        BuildMode::Dev | BuildMode::DevCompile => {
            command.args([
                "-configuration",
                "Debug",
                "-sdk",
                "iphonesimulator",
                "build",
            ]);
            command
                .arg("-derivedDataPath")
                .arg(root.join("ios-derived"));
        }
        BuildMode::Test => {
            command.args([
                "-configuration",
                "Debug",
                "-destination",
                "generic/platform=iOS",
                "CODE_SIGNING_ALLOWED=NO",
                "build",
            ]);
            command
                .arg("-derivedDataPath")
                .arg(root.join("ios-derived"));
        }
        BuildMode::Release => {
            command.args([
                "-configuration",
                "Release",
                "-destination",
                "generic/platform=iOS",
            ]);
            if let Some(profile) = manual_profile.as_deref() {
                command
                    .args([
                        "CODE_SIGN_STYLE=Manual",
                        "CODE_SIGN_IDENTITY=Apple Distribution",
                    ])
                    .arg(format!("PROVISIONING_PROFILE_SPECIFIER={profile}"));
            } else {
                command.arg("CODE_SIGN_STYLE=Automatic");
            }
            if let Ok(team_id) = env::var("NEXA_IOS_TEAM_ID") {
                command.arg(format!("DEVELOPMENT_TEAM={team_id}"));
            }
            command.arg("-archivePath").arg(&archive).arg("archive");
            command.arg("-allowProvisioningUpdates");
        }
    }
    run_command(command, "iOS build")?;
    if matches!(mode, BuildMode::Dev) {
        launch_ios_simulator(root, app_name)?;
    }
    if matches!(mode, BuildMode::Release) {
        let archived_app = archive
            .join("Products/Applications")
            .join(format!("{app_name}.app"));
        if !archived_app.is_dir() {
            return Err(format!(
                "iOS archive command succeeded but did not contain the app product {}",
                archived_app.display()
            ));
        }
        let export_dir = root.join("artifacts/ios/ipa");
        remove_generated_path(&export_dir)?;
        let options_path = root.join("ExportOptions.plist");
        let Some(options_parent) = options_path.parent() else {
            return Err("invalid export options path".to_owned());
        };
        fs::create_dir_all(options_parent).map_err(|error| error.to_string())?;
        let method =
            env::var("NEXA_IOS_EXPORT_METHOD").unwrap_or_else(|_| "app-store-connect".to_owned());
        if !matches!(
            method.as_str(),
            "app-store-connect" | "ad-hoc" | "development" | "enterprise"
        ) {
            return Err(format!("unsupported NEXA_IOS_EXPORT_METHOD `{method}`"));
        }
        let team = env::var("NEXA_IOS_TEAM_ID")
            .ok()
            .map(|team| format!("<key>teamID</key><string>{}</string>", xml_escape(&team)))
            .unwrap_or_default();
        let (signing_style, provisioning_profiles) = if let Some(profile) =
            manual_profile.as_deref()
        {
            let bundle_id = bundle_identifier(root, app_name)?;
            (
                "manual",
                format!(
                    "<key>provisioningProfiles</key><dict><key>{}</key><string>{}</string></dict>",
                    xml_escape(&bundle_id),
                    xml_escape(profile)
                ),
            )
        } else {
            ("automatic", String::new())
        };
        fs::write(&options_path, format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>method</key><string>{method}</string><key>signingStyle</key><string>{signing_style}</string>{team}{provisioning_profiles}<key>stripSwiftSymbols</key><true/><key>manageAppVersionAndBuildNumber</key><false/></dict></plist>\n"))
            .map_err(|error| format!("{}: {error}", options_path.display()))?;
        let mut export = Command::new("xcodebuild");
        export
            .args(["-exportArchive", "-archivePath"])
            .arg(&archive)
            .args(["-exportPath"])
            .arg(&export_dir)
            .args(["-exportOptionsPlist"])
            .arg(&options_path)
            .arg("-allowProvisioningUpdates");
        run_command(export, "iOS IPA export")?;
        let ipa = match fs::read_dir(&export_dir) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| path.extension().is_some_and(|extension| extension == "ipa")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("{}: {error}", export_dir.display())),
        }
        .ok_or_else(|| {
            format!(
                "iOS export command succeeded but did not create an IPA under {}",
                export_dir.display()
            )
        })?;
        if fs::metadata(&ipa)
            .map_err(|error| format!("{}: {error}", ipa.display()))?
            .len()
            == 0
        {
            return Err(format!(
                "iOS export created an empty IPA: {}",
                ipa.display()
            ));
        }
        println!(
            "Created iOS archive at {} and IPA at {}",
            archive.display(),
            ipa.display(),
        );
    }
    Ok(())
}

fn build_android(root: &Path, mode: BuildMode, dev_port: Option<u16>) -> Result<(), String> {
    let gradle = root.join("android/gradlew");
    if !gradle.is_file() {
        return Err(format!(
            "{} is missing; regenerate the Android project with `nexa dev`",
            gradle.display()
        ));
    }
    let task = match mode {
        BuildMode::Dev | BuildMode::DevCompile => ":app:assembleDebug",
        BuildMode::Test => ":app:assembleDebug",
        BuildMode::Release => ":app:bundleRelease",
    };
    let aab = root.join("android/app/build/outputs/bundle/release/app-release.aab");
    if matches!(mode, BuildMode::Release) {
        remove_generated_path(&aab)?;
    }
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(&gradle);
        command
    };
    #[cfg(not(windows))]
    let mut command = Command::new(&gradle);
    // Refresh only the Material 3 family whose explicit version is selected
    // by generated Nexa features. These artifacts share an atomic version.
    command.current_dir(root.join("android")).arg(task).args([
        "--update-locks",
        "androidx.compose.material3:material3,androidx.compose.material3:material3-adaptive-navigation-suite,androidx.compose.material3:material3-ripple",
    ]);
    if matches!(mode, BuildMode::Release) {
        let signing = validate_android_release_signing()?;
        command
            .env("NEXA_ANDROID_KEYSTORE", signing.keystore)
            .env("NEXA_ANDROID_KEY_ALIAS", signing.key_alias)
            .env("NEXA_ANDROID_STORE_PASSWORD", signing.store_password)
            .env("NEXA_ANDROID_KEY_PASSWORD", signing.key_password);
    }
    run_command(command, "Android build")?;
    match mode {
        BuildMode::Dev => launch_android_emulator(root, dev_port)?,
        BuildMode::DevCompile => {}
        BuildMode::Test => println!("Android Kotlin and native project compile passed."),
        BuildMode::Release => {
            let size = fs::metadata(&aab)
                .map_err(|error| {
                    format!(
                        "Android bundle command succeeded but did not create {}: {error}",
                        aab.display()
                    )
                })?
                .len();
            if size == 0 {
                return Err(format!(
                    "Android bundle created an empty AAB: {}",
                    aab.display()
                ));
            }
            verify_android_aab_signature(&aab)?;
            println!("Created signed Android AAB at {}", aab.display());
        }
    }
    Ok(())
}

fn remove_generated_path(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let remove = if metadata.file_type().is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    remove.map_err(|error| {
        format!(
            "cannot remove previous generated release output {}: {error}",
            path.display()
        )
    })
}

fn verify_android_aab_signature(aab: &Path) -> Result<(), String> {
    let jarsigner = env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .map(|java_home| {
            java_home.join("bin").join(if cfg!(windows) {
                "jarsigner.exe"
            } else {
                "jarsigner"
            })
        })
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("jarsigner"));
    let output = Command::new(&jarsigner)
        .args(["-verify", "-verbose:summary"])
        .arg(aab)
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| {
            format!(
                "cannot verify Android AAB signature with {}: {error} (install a full JDK)",
                jarsigner.display()
            )
        })?;
    let report = String::from_utf8_lossy(&output.stdout);
    let normalized_report = report.to_ascii_lowercase();
    if !output.status.success()
        || normalized_report.contains("jar is unsigned.")
        || normalized_report.contains("unsigned entr")
        || !normalized_report.contains("jar verified.")
    {
        let details = String::from_utf8_lossy(&output.stderr);
        let details = if details.trim().is_empty() {
            report.trim()
        } else {
            details.trim()
        };
        return Err(format!(
            "Android AAB signature verification failed for {}: {}",
            aab.display(),
            if details.is_empty() {
                "the bundle is unsigned or has an invalid signature"
            } else {
                details
            }
        ));
    }
    Ok(())
}

struct AndroidReleaseSigning {
    keystore: PathBuf,
    key_alias: String,
    store_password: String,
    key_password: String,
}

fn validate_android_release_signing() -> Result<AndroidReleaseSigning, String> {
    let required = [
        "NEXA_ANDROID_KEYSTORE",
        "NEXA_ANDROID_KEY_ALIAS",
        "NEXA_ANDROID_STORE_PASSWORD",
        "NEXA_ANDROID_KEY_PASSWORD",
    ];
    let local_signing_path = env::current_dir()
        .map_err(|error| error.to_string())?
        .join(".nexa/signing.properties");
    let local_values = if local_signing_path.is_file() {
        parse_android_signing_properties(&local_signing_path)?
    } else {
        BTreeMap::new()
    };
    let values = required
        .iter()
        .map(|name| {
            let value = env::var(name)
                .ok()
                .or_else(|| local_values.get(*name).cloned())
                .unwrap_or_default();
            (*name, value)
        })
        .collect::<BTreeMap<_, _>>();
    let missing = required
        .iter()
        .filter(|key| {
            values
                .get(**key)
                .is_none_or(|value| value.trim().is_empty())
        })
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "Android release signing requires {}. Set them in `.nexa/signing.properties` or the environment",
            missing.join(", ")
        ));
    }

    let value = |name: &str| {
        values
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Android release signing is missing {name}"))
    };
    let keystore = PathBuf::from(value("NEXA_ANDROID_KEYSTORE")?);
    let keystore = if keystore.is_absolute() {
        keystore
    } else {
        env::current_dir()
            .map_err(|error| format!("cannot resolve the Android keystore path: {error}"))?
            .join(keystore)
    };
    let keystore = keystore.canonicalize().map_err(|error| {
        format!(
            "NEXA_ANDROID_KEYSTORE does not point to an existing file ({}): {error}",
            keystore.display()
        )
    })?;
    if !keystore.is_file() {
        return Err(format!(
            "NEXA_ANDROID_KEYSTORE must point to a file: {}",
            keystore.display()
        ));
    }
    Ok(AndroidReleaseSigning {
        keystore,
        key_alias: value("NEXA_ANDROID_KEY_ALIAS")?,
        store_password: value("NEXA_ANDROID_STORE_PASSWORD")?,
        key_password: value("NEXA_ANDROID_KEY_PASSWORD")?,
    })
}

fn parse_android_signing_properties(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let allowed = [
        "NEXA_ANDROID_KEYSTORE",
        "NEXA_ANDROID_KEY_ALIAS",
        "NEXA_ANDROID_STORE_PASSWORD",
        "NEXA_ANDROID_KEY_PASSWORD",
    ];
    let mut values = BTreeMap::new();
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, raw_value) = line
            .split_once('=')
            .ok_or_else(|| format!("{}:{line_number}: expected `KEY=value`", path.display()))?;
        let key = key.trim();
        if !allowed.contains(&key) {
            return Err(format!(
                "{}:{line_number}: unknown Android signing key `{key}`",
                path.display()
            ));
        }
        let raw_value = raw_value.trim();
        let value = if raw_value.len() >= 2
            && ((raw_value.starts_with('"') && raw_value.ends_with('"'))
                || (raw_value.starts_with('\'') && raw_value.ends_with('\'')))
        {
            &raw_value[1..raw_value.len() - 1]
        } else if raw_value.starts_with('"')
            || raw_value.starts_with('\'')
            || raw_value.ends_with('"')
            || raw_value.ends_with('\'')
        {
            return Err(format!(
                "{}:{line_number}: unmatched quote in signing value",
                path.display()
            ));
        } else {
            raw_value
        };
        if values.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!(
                "{}:{line_number}: duplicate Android signing key `{key}`",
                path.display()
            ));
        }
    }
    Ok(values)
}

fn doctor() -> Result<(), String> {
    let mut failed = false;
    for (label, name, args) in [
        ("Rust compiler", "rustc", vec!["--version"]),
        ("Cargo", "cargo", vec!["--version"]),
        ("Xcode", "xcodebuild", vec!["-version"]),
        (
            "iOS Simulator",
            "xcrun",
            vec!["simctl", "list", "devices", "available", "-j"],
        ),
        ("Java", "java", vec!["-version"]),
        ("JDK compiler", "javac", vec!["--version"]),
        ("Android platform tools", "adb", vec!["version"]),
        ("Clang", "clang", vec!["--version"]),
    ] {
        match Command::new(name).args(args).output() {
            Ok(output) if output.status.success() => {
                if name == "xcrun" {
                    if has_available_simulator(&output.stdout) {
                        println!("✓ {label}: at least one available simulator");
                    } else {
                        failed = true;
                        print_doctor_failure(
                            label,
                            "no available simulator device was found",
                            doctor_repair_hint(label),
                        );
                    }
                } else {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let detail = stdout
                        .lines()
                        .chain(stderr.lines())
                        .find(|line| !line.trim().is_empty())
                        .map(str::trim)
                        .unwrap_or("available")
                        .to_owned();
                    if let Some(minimum) = doctor_minimum_version(label) {
                        match parse_version(&detail) {
                            Some(actual) if actual < minimum => {
                                failed = true;
                                print_doctor_failure(
                                    label,
                                    &format!(
                                        "{detail} is older than the required version {}",
                                        format_version(minimum)
                                    ),
                                    doctor_repair_hint(label),
                                );
                                continue;
                            }
                            None => {
                                failed = true;
                                print_doctor_failure(
                                    label,
                                    &format!("could not read a version from `{detail}`"),
                                    doctor_repair_hint(label),
                                );
                                continue;
                            }
                            _ => {}
                        }
                    }
                    println!("✓ {label}: {detail}");
                }
            }
            Ok(output) => {
                failed = true;
                let stderr = String::from_utf8_lossy(&output.stderr);
                let detail = stderr
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .map(str::trim)
                    .unwrap_or("command failed")
                    .to_owned();
                print_doctor_failure(label, &detail, doctor_repair_hint(label));
            }
            Err(error) => {
                failed = true;
                print_doctor_failure(label, &error.to_string(), doctor_repair_hint(label));
            }
        }
    }
    let root =
        env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))?;
    let sdk = android_sdk_path(
        &root,
        env::var_os("ANDROID_HOME").as_deref(),
        env::var_os("ANDROID_SDK_ROOT").as_deref(),
        env::var_os("HOME").as_deref().map(Path::new),
        env::var_os("LOCALAPPDATA").as_deref().map(Path::new),
    );
    if let Some(path) = sdk {
        println!("✓ Android SDK: {}", path.display());
        if android_sdk_has_compile_platform(&path) {
            println!(
                "✓ Android compile SDK: API {}",
                crate::config::ANDROID_COMPILE_SDK
            );
        } else {
            failed = true;
            print_doctor_failure(
                "Android compile SDK",
                &format!(
                    "API {} is not installed",
                    crate::config::ANDROID_COMPILE_SDK
                ),
                doctor_repair_hint("Android compile SDK"),
            );
        }
        let emulator_name = if cfg!(windows) {
            "emulator.exe"
        } else {
            "emulator"
        };
        let avd_output = path.join("emulator").join(emulator_name);
        if avd_output.is_file() {
            match Command::new(&avd_output).arg("-list-avds").output() {
                Ok(output)
                    if output.status.success()
                        && !String::from_utf8_lossy(&output.stdout).trim().is_empty() =>
                {
                    let count = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .count();
                    println!("✓ Android emulator: {count} AVD(s) configured");
                }
                _ => println!(
                    "! Android emulator: no AVD is configured. Create one in Android Studio's Device Manager and install its Android system image."
                ),
            }
        } else {
            println!(
                "! Android emulator: emulator tools are missing. Install Android Emulator in Android Studio's SDK Manager."
            );
        }
    } else {
        failed = true;
        print_doctor_failure(
            "Android SDK",
            "no installed SDK was found",
            doctor_repair_hint("Android SDK"),
        );
    }
    let dependencies = crate::config::load_plugin_dependencies(&root.join("nexa.config.nx"));
    match dependencies {
        Ok(dependencies) => match crate::dependencies::check_offline(&root, &dependencies) {
            Ok(()) if dependencies.is_empty() => {
                println!("✓ Plugin setup: no dependencies configured");
            }
            Ok(()) => println!(
                "✓ Plugin setup: {} dependencies and lockfile are valid",
                dependencies.len()
            ),
            Err(error) => {
                failed = true;
                println!("✗ Plugin setup: {error}");
            }
        },
        Err(error) => {
            failed = true;
            println!("✗ Plugin setup: {error}");
        }
    }
    if failed {
        Err("one or more environment checks failed; see https://nexa.dev/docs/setup".to_owned())
    } else {
        Ok(())
    }
}

fn print_doctor_failure(label: &str, detail: &str, hint: &str) {
    println!("✗ {label}: {detail}\n  Fix: {hint}");
}

fn doctor_repair_hint(label: &str) -> &'static str {
    match label {
        "Rust compiler" | "Cargo" => "Install Rust with rustup, then run `rustup update stable`.",
        "Xcode" => {
            "Install Xcode 15 or newer, select it with `sudo xcode-select -s /Applications/Xcode.app/Contents/Developer`, then accept the license with `sudo xcodebuild -license accept`."
        }
        "iOS Simulator" => {
            "In Xcode, download an iOS Simulator runtime, then create an iPhone or iPad simulator in Devices and Simulators."
        }
        "Java" => "Install JDK 17 or newer and set `JAVA_HOME` to that JDK.",
        "Android platform tools" => {
            "Install Android SDK Platform-Tools in Android Studio's SDK Manager and add `<android-sdk>/platform-tools` to `PATH`."
        }
        "Clang" => {
            "On macOS, run `xcode-select --install`; on Linux, install Clang with your distribution's package manager."
        }
        "Android SDK" => {
            "Install Android SDK in Android Studio, then set `ANDROID_HOME` or `ANDROID_SDK_ROOT` to the SDK directory."
        }
        "Android compile SDK" => {
            "In Android Studio's SDK Manager, install Android SDK Platform for the required API level shown above."
        }
        _ => "Follow the Nexa toolchain setup guide and rerun `nexa doctor`.",
    }
}

fn has_available_simulator(json: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(json)
        .ok()
        .and_then(|value| value.get("devices").cloned())
        .and_then(|value| value.as_object().cloned())
        .is_some_and(|platforms| {
            platforms.values().any(|devices| {
                devices
                    .as_array()
                    .is_some_and(|devices| !devices.is_empty())
            })
        })
}

fn doctor_minimum_version(label: &str) -> Option<(u32, u32, u32)> {
    match label {
        "Rust compiler" | "Cargo" => Some((1, 85, 0)),
        "Xcode" => Some((15, 0, 0)),
        "Java" | "JDK compiler" => Some((17, 0, 0)),
        _ => None,
    }
}

fn android_sdk_has_compile_platform(sdk_root: &Path) -> bool {
    let platform_prefix = format!("android-{}", crate::config::ANDROID_COMPILE_SDK);
    fs::read_dir(sdk_root.join("platforms"))
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| android_platform_name_matches_api(name, &platform_prefix))
                && entry.path().join("android.jar").is_file()
        })
}

fn android_platform_name_matches_api(name: &str, platform_prefix: &str) -> bool {
    let Some(version) = name.strip_prefix(platform_prefix) else {
        return false;
    };
    version.is_empty()
        || version.strip_prefix('.').is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .split('.')
                    .all(|part| part.parse::<u32>().is_ok_and(|value| value == 0))
        })
}

fn parse_version(value: &str) -> Option<(u32, u32, u32)> {
    for token in value
        .split(|character: char| !character.is_ascii_digit() && character != '.')
        .filter(|token| !token.is_empty())
    {
        let mut parts = token.split('.');
        let Ok(major) = parts.next()?.parse() else {
            continue;
        };
        let Ok(minor) = parts.next().unwrap_or("0").parse() else {
            continue;
        };
        let Ok(patch) = parts.next().unwrap_or("0").parse() else {
            continue;
        };
        return Some((major, minor, patch));
    }
    None
}

fn format_version(version: (u32, u32, u32)) -> String {
    format!("{}.{}.{}", version.0, version.1, version.2)
}

fn android_sdk_path(
    project_root: &Path,
    android_home: Option<&std::ffi::OsStr>,
    android_sdk_root: Option<&std::ffi::OsStr>,
    _home: Option<&Path>,
    _local_app_data: Option<&Path>,
) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for properties in [
        project_root.join("build/android/local.properties"),
        project_root.join("android/local.properties"),
    ] {
        if let Ok(contents) = fs::read_to_string(properties) {
            for line in contents.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                if key.trim() == "sdk.dir" {
                    candidates.push(PathBuf::from(
                        value.trim().replace("\\:", ":").replace("\\\\", "\\"),
                    ));
                    break;
                }
            }
        }
    }
    candidates.extend(
        [android_home, android_sdk_root]
            .into_iter()
            .flatten()
            .map(PathBuf::from),
    );
    #[cfg(target_os = "macos")]
    if let Some(home) = _home {
        candidates.push(home.join("Library/Android/sdk"));
    }
    #[cfg(target_os = "linux")]
    if let Some(home) = _home {
        candidates.push(home.join("Android/Sdk"));
    }
    #[cfg(target_os = "windows")]
    if let Some(local_app_data) = _local_app_data {
        candidates.push(local_app_data.join("Android/Sdk"));
    }
    candidates.into_iter().find(|path| path.is_dir())
}

fn find_entry(root: &Path) -> Result<PathBuf, String> {
    for candidate in ["App.nx", "app.nx"] {
        let path = root.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    let mut entries = fs::read_dir(root)
        .map_err(|error| format!("{}: {error}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "nx"))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name != "nexa.config.nx")
        })
        .collect::<Vec<_>>();
    entries.sort();
    match entries.as_slice() {
        [entry] => Ok(entry.clone()),
        [] => Err(format!(
            "no .nx app entry found in {}; run `nexa create MyApp` first",
            root.display()
        )),
        _ => Err(format!(
            "multiple .nx files found in {}; specify one as the project entrypoint in a future Nexa version",
            root.display()
        )),
    }
}

fn launch_ios_simulator(root: &Path, app_name: &str) -> Result<(), String> {
    let booted_devices = booted_ios_devices().unwrap_or_default();
    let device = booted_devices
        .first()
        .cloned()
        .or_else(|| available_ios_devices().ok()?.into_iter().next())
        .ok_or_else(|| {
            "no available iOS Simulator found; install an iOS Simulator runtime in Xcode".to_owned()
        })?;
    launch_ios_simulator_on_device(root, app_name, &device)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DevDevice {
    platform: String,
    name: String,
    id: String,
    booted: bool,
}

fn cycle_dev_device(
    context: &DevWatchContext<'_>,
    current: &Option<(String, String)>,
) -> Result<DevDevice, String> {
    let mut devices = Vec::new();
    let mut unavailable = Vec::new();
    if matches!(context.platform, "ios" | "all") {
        match available_ios_devices() {
            Ok(found) => devices.extend(found),
            Err(error) => unavailable.push(error),
        }
    }
    if matches!(context.platform, "android" | "all") {
        match available_android_devices() {
            Ok(found) => devices.extend(found),
            Err(error) => unavailable.push(error),
        }
    }
    if devices.is_empty() {
        let details = if unavailable.is_empty() {
            "no available iOS simulators or online Android devices were found".to_owned()
        } else {
            unavailable.join("; ")
        };
        return Err(details);
    }
    let next = current
        .as_ref()
        .and_then(|(platform, id)| {
            devices
                .iter()
                .position(|device| &device.platform == platform && &device.id == id)
        })
        .map_or(0, |index| (index + 1) % devices.len());
    let device = devices[next].clone();
    match device.platform.as_str() {
        "ios" => launch_ios_simulator_on_device(context.output, context.project_name, &device)?,
        "android" => launch_android_emulator_on_device(
            context.output,
            Some(context.server.address().port()),
            &device,
        )?,
        _ => return Err(format!("unsupported device platform `{}`", device.platform)),
    }
    Ok(device)
}

fn available_ios_devices() -> Result<Vec<DevDevice>, String> {
    let output = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available"])
        .output()
        .map_err(|error| format!("cannot list iOS simulators with xcrun: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "xcrun simctl could not list available iOS simulators: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(parse_ios_simulator_devices(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn booted_ios_devices() -> Result<Vec<DevDevice>, String> {
    let output = Command::new("xcrun")
        .args(["simctl", "list", "devices", "booted"])
        .output()
        .map_err(|error| format!("cannot list booted iOS simulators with xcrun: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "xcrun simctl could not list booted iOS simulators: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(parse_ios_simulator_devices(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_ios_simulator_devices(listing: &str) -> Vec<DevDevice> {
    let mut devices = listing
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.contains("iPhone") && !trimmed.contains("iPad") {
                return None;
            }
            let id = trimmed
                .split_whitespace()
                .map(|part| {
                    part.trim_matches(|character: char| {
                        !character.is_ascii_hexdigit() && character != '-'
                    })
                })
                .find(|part| is_simulator_udid(part))?;
            let marker = format!("({id})");
            let name = trimmed.split_once(&marker)?.0.trim().to_owned();
            Some(DevDevice {
                platform: "ios".to_owned(),
                name,
                id: id.to_owned(),
                booted: trimmed.contains("(Booted)"),
            })
        })
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    devices
}

fn is_simulator_udid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn available_android_devices() -> Result<Vec<DevDevice>, String> {
    let output = Command::new("adb")
        .args(["devices", "-l"])
        .output()
        .map_err(|error| format!("cannot list Android devices with adb: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "adb could not list Android devices: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(parse_android_devices(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_android_devices(listing: &str) -> Vec<DevDevice> {
    let mut devices = listing
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let id = fields.next()?;
            if fields.next()? != "device" {
                return None;
            }
            let name = line
                .split_whitespace()
                .find_map(|field| field.strip_prefix("model:"))
                .map(|model| model.replace('_', " "))
                .unwrap_or_else(|| id.to_owned());
            Some(DevDevice {
                platform: "android".to_owned(),
                name,
                id: id.to_owned(),
                booted: true,
            })
        })
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    devices
}

fn launch_ios_simulator_on_device(
    root: &Path,
    app_name: &str,
    device: &DevDevice,
) -> Result<(), String> {
    if !device.booted {
        let mut boot = Command::new("xcrun");
        boot.args(["simctl", "boot", &device.id]);
        run_command(boot, "Simulator boot")?;
    }
    let mut bootstatus = Command::new("xcrun");
    bootstatus.args(["simctl", "bootstatus", &device.id, "-b"]);
    run_command(bootstatus, "Simulator boot")?;
    let app = root
        .join("ios-derived/Build/Products/Debug-iphonesimulator")
        .join(format!("{app_name}.app"));
    let bundle_id = bundle_identifier(root, app_name)?;
    let mut install = Command::new("xcrun");
    install.args(["simctl", "install", &device.id]).arg(&app);
    run_command(install, "Simulator install")?;
    let mut launch = Command::new("xcrun");
    launch.args(["simctl", "launch", &device.id, &bundle_id]);
    run_command(launch, "Simulator launch")?;
    Ok(())
}

fn launch_android_emulator(root: &Path, dev_port: Option<u16>) -> Result<(), String> {
    let device = available_android_devices()?
        .into_iter()
        .next()
        .ok_or_else(|| {
            "no Android emulator or device is connected; start one, then rerun `nexa dev --android`"
                .to_owned()
        })?;
    launch_android_emulator_on_device(root, dev_port, &device)
}

fn launch_android_emulator_on_device(
    root: &Path,
    dev_port: Option<u16>,
    device: &DevDevice,
) -> Result<(), String> {
    if let Some(port) = dev_port {
        let local = format!("tcp:{port}");
        let remote = local.clone();
        let mut reverse = Command::new("adb");
        reverse.args(["-s", &device.id, "reverse", local.as_str(), remote.as_str()]);
        run_command(reverse, "Android dev server port forwarding")?;
    }
    let package = android_application_id(root)?;
    let apk = root.join("android/app/build/outputs/apk/debug/app-debug.apk");
    let mut install = Command::new("adb");
    install.args(["-s", &device.id, "install", "-r"]).arg(apk);
    run_command(install, "Android install")?;
    let mut launch = Command::new("adb");
    launch.args(["-s", &device.id, "shell", "monkey", "-p", &package, "1"]);
    if let Err(monkey_error) = run_command(launch, "Android launch") {
        let activity = format!("{package}/.MainActivity");
        let mut direct_launch = Command::new("adb");
        direct_launch.args([
            "-s",
            &device.id,
            "shell",
            "am",
            "start",
            "-n",
            activity.as_str(),
        ]);
        run_command(direct_launch, "Android activity launch").map_err(|activity_error| {
            format!("{monkey_error}; direct activity launch also failed: {activity_error}")
        })?;
    }
    Ok(())
}

fn bundle_identifier(root: &Path, app_name: &str) -> Result<String, String> {
    let project = root
        .join("ios")
        .join(format!("{app_name}.xcodeproj/project.pbxproj"));
    let source =
        fs::read_to_string(&project).map_err(|error| format!("{}: {error}", project.display()))?;
    source
        .split("PRODUCT_BUNDLE_IDENTIFIER = ")
        .nth(1)
        .and_then(|part| part.split(';').next())
        .map(|value| value.trim().trim_matches('"').to_owned())
        .ok_or_else(|| {
            format!(
                "{}: generated project has no bundle identifier",
                project.display()
            )
        })
}

fn android_application_id(root: &Path) -> Result<String, String> {
    let gradle = root.join("android/app/build.gradle.kts");
    let source =
        fs::read_to_string(&gradle).map_err(|error| format!("{}: {error}", gradle.display()))?;
    source
        .split("applicationId = \"")
        .nth(1)
        .and_then(|part| part.split('"').next())
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "{}: generated project has no applicationId",
                gradle.display()
            )
        })
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn set_platform(
    current: &mut String,
    explicitly_set: &mut bool,
    requested: &str,
) -> Result<(), String> {
    if *explicitly_set && current != requested {
        return Err(
            "choose only one platform (`--ios`, `--android`, or `--platform all`)".to_owned(),
        );
    }
    *current = requested.to_owned();
    *explicitly_set = true;
    Ok(())
}

fn select_native_platform(current: &mut String, explicitly_set: &mut bool, requested: &str) {
    if !*explicitly_set {
        *current = requested.to_owned();
        *explicitly_set = true;
    } else if current != requested {
        *current = "all".to_owned();
    }
}

fn starter_source(name: &str) -> String {
    format!(
        "app {name} {{\n    state count: Int32 = 0\n\n    body {{\n        Column(gap: 16) {{\n            Text(\"Welcome to {name}\")\n            Text(\"Count: $count\")\n            Button(\"Add one\") {{\n                count = count + 1\n            }}\n        }}\n    }}\n}}\n"
    )
}

fn starter_config(name: &str) -> String {
    let id = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    format!(
        "config {{\n    app {{ displayName: \"{name}\", version: \"1.0.0\", buildNumber: 1, deepLinks: [] }}\n    flavors {{ staging {{ suffix: \"staging\" }} }}\n    ios {{ minVersion: \"16.0\", bundleIdentifier: \"dev.nexa.{id}\" }}\n    android {{ minSdk: 24, targetSdk: 36, applicationId: \"dev.nexa.{id}\", cronet {{ provider: \"play-services\", diskCacheSizeMb: 64 }} }}\n    permissions {{}}\n}}\n"
    )
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn type_name(value: &str) -> String {
    let mut result = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    if result.starts_with(|character: char| character.is_ascii_digit()) {
        result.insert(0, 'N');
    }
    result
}

fn preflight_native_toolchains(
    platform: &str,
    command: &str,
    compile_only: bool,
) -> Result<(), String> {
    preflight_native_toolchains_with(platform, command, compile_only, check_native_tool)
}

fn preflight_native_toolchains_with(
    platform: &str,
    command: &str,
    compile_only: bool,
    mut check: impl FnMut(&str, &[&str], &str) -> Result<(), String>,
) -> Result<(), String> {
    let launches_device = command == "dev" && !compile_only;
    if matches!(platform, "ios" | "all") {
        check(
            "xcodebuild",
            &["-version"],
            "Install Xcode and select it with xcode-select.",
        )?;
        if launches_device {
            check(
                "xcrun",
                &["--find", "simctl"],
                "Install an iOS Simulator runtime with Xcode.",
            )?;
        }
    }
    if matches!(platform, "android" | "all") {
        check(
            "java",
            &["-version"],
            "Install a JDK supported by the Android Gradle plugin.",
        )?;
        if launches_device {
            check(
                "adb",
                &["version"],
                "Install Android platform tools and make `adb` available on PATH.",
            )?;
        }
    }
    Ok(())
}

fn check_native_tool(program: &str, args: &[&str], hint: &str) -> Result<(), String> {
    let invocation = format!("{program} {}", args.join(" "));
    let output = Command::new(program).args(args).output().map_err(|error| {
        format!("native build preflight could not start `{invocation}`: {error}. {hint}")
    })?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let detail = if detail.is_empty() {
        format!("process exited with {}", output.status)
    } else {
        detail
    };
    Err(format!(
        "native build preflight failed for `{invocation}`: {detail}. {hint}"
    ))
}

fn run_command(mut command: Command, label: &str) -> Result<(), String> {
    let rendered = format!("{command:?}");
    let status = command
        .status()
        .map_err(|error| format!("cannot start {label} ({rendered}): {error}"))?;
    ensure_success(status, label)
}

fn ensure_success(status: ExitStatus, label: &str) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}"))
    }
}

fn print_help() {
    println!(
        "Nexa — native iOS and Android apps from one .nx project\n\nUsage:\n  nexa create <ProjectName>\n  nexa check [--ios | --android] [--json] [--audit] [--locked]\n  nexa audit <source.nx> [--target <ios|android|all>] [--release-sizes] [--optimization-report] [--out <path>]\n  nexa fmt [<file-or-directory> ...] [--check]\n  nexa dev [--ios | --android | --all] [--dry-run] [--locked]\n  nexa test [--ios | --android | --all] [--dry-run] [--locked]\n  nexa test --unit-only [--ios | --android] [--dry-run] [--locked]\n  nexa release [--ios | --android | --all] [--dry-run] [--locked]\n  nexa doctor\n  nexa plugin new | check | generate | doc\n\nRun `nexa <command> --help` for command options."
    );
}

fn print_command_help(command: &str) {
    match command {
        "create" => println!("Usage: nexa create <ProjectName> [--directory <path>]"),
        "check" => {
            println!("Usage: nexa check [--ios | --android] [--deny-warnings] [--audit] [--locked]")
        }
        "audit" => crate::audit::print_help(),
        "fmt" => println!("Usage: nexa fmt [<file-or-directory> ...] [--check]"),
        "build" => println!("`nexa build` is deprecated; use `nexa release`."),
        "dev" => {
            println!(
                "Usage: nexa dev [--ios | --android | --all] [--arch <architecture>] [--once | --compile-only] [--flavor <name>] [--out <directory>] [--dry-run] [--locked]\n`--dry-run` prints the planned steps without running them.\nWhile running: `r` hot reloads, `Shift+R` hot restarts, `b` rebuilds and relaunches, `d` cycles connected devices, `p` toggles the performance overlay, `t` toggles redacted runtime trace events, `l` clears logs, and `q` stops the session."
            )
        }
        "test" => {
            println!(
                "Usage: nexa test [--ios | --android | --all] [--arch <architecture>] [--flavor <name>] [--out <directory>] [--dry-run] [--locked]\n       nexa test --unit-only [--ios | --android] [--dry-run] [--locked]\n`--dry-run` prints the planned steps without running them."
            )
        }
        "release" => println!(
            "Usage: nexa release [--ios | --android | --all] [--arch <architecture>] [--flavor <name>] [--out <directory>] [--dry-run] [--locked]\n`--dry-run` prints the planned steps without running them. Builds an iOS archive or Android AAB. Android signing uses .nexa/signing.properties or NEXA_ANDROID_* environment variables; iOS signing uses Xcode."
        ),
        "plugin" => println!(
            "Usage:\n  nexa plugin new <plugin.id> --out <directory> [--name <TypeName>] [--kind native|pure]\n  nexa plugin check <package-directory|native.nxid>\n  nexa plugin generate <package-directory|native.nxid> --target <swift|kotlin|cpp> [--package <name>] [--out <directory>]\n  nexa plugin doc <package-directory|native.nxid> [--out <file>]\n\n`nexa plugin init` is deprecated; use `nexa plugin new`."
        ),
        _ => print_help(),
    }
}

fn set_flavor(current: &mut Option<String>, requested: &str) -> Result<(), String> {
    if let Some(current) = current
        && current != requested
    {
        return Err("choose only one flavor (`--staging` or `--flavor <name>`)".to_owned());
    }
    *current = Some(requested.to_owned());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        DashboardLine, DevConsoleCommand, DevDashboard, DeviceLogDeduplicator, DeviceLogLine,
        android_log_line_matches_pid, android_platform_name_matches_api,
        android_sdk_has_compile_platform, android_sdk_path, changed_paths, clip_terminal_text,
        coalesce_source_changes, doctor_minimum_version, doctor_repair_hint,
        has_available_simulator, is_relevant_dev_path, parse_android_devices,
        parse_android_process_id, parse_dev_diagnostic, parse_ios_simulator_devices, parse_version,
        preflight_native_toolchains_with, release_build_arguments, requires_native_rebuild, run,
        run_in_language_tests, sanitize_terminal_text, select_native_platform, source_fingerprint,
        write_dashboard_row,
    };
    use crossterm::style::Color;
    use nexa_testkit::{TempDir, TestProject};
    use std::collections::{HashMap, VecDeque};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn dev_watcher_coalesces_a_single_file_save_burst_for_40ms() {
        let (sender, receiver) = mpsc::channel();
        let second_sender = sender.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            second_sender
                .send(DevConsoleCommand::SourceChanges(vec![PathBuf::from(
                    "src/Second.nx",
                )]))
                .expect("the watcher is still coalescing the save");
        });
        let first_paths = vec![PathBuf::from("src/First.nx")];
        let start = Instant::now();
        let changed = coalesce_source_changes(first_paths, &receiver, &mut VecDeque::new());

        assert!(start.elapsed() >= Duration::from_millis(40));
        assert_eq!(
            changed,
            [
                PathBuf::from("src/First.nx"),
                PathBuf::from("src/Second.nx")
            ]
        );
    }

    #[test]
    fn dev_watcher_detects_atomic_replacement_of_an_existing_source_file() {
        let project = TestProject::new("nexa-dev-watcher-atomic-save");
        let app = fs::canonicalize(project.write_app("app Demo { body { Text(\"Initial\") } }\n"))
            .expect("canonicalize the watched source");
        let (sender, receiver) = mpsc::channel();
        let watcher = super::start_dev_source_watcher(project.path(), &HashMap::new(), sender)
            .expect("watch the project directory");

        let replacement = project.join("App.nx.tmp");
        fs::write(&replacement, "app Demo { body { Text(\"Saved once\") } }\n")
            .expect("write the editor temporary file");
        fs::rename(&replacement, &app).expect("atomically replace the source file");

        let mut saw_replacement = false;
        while let Ok(DevConsoleCommand::SourceChanges(paths)) =
            receiver.recv_timeout(Duration::from_secs(2))
        {
            saw_replacement |= paths.contains(&app);
            if saw_replacement {
                break;
            }
        }
        drop(watcher);
        assert!(
            saw_replacement,
            "the renamed .nx source must trigger a reload"
        );
    }

    #[test]
    fn doctor_finds_sdk_from_project_properties_before_environment() {
        let scratch = TempDir::new("nexa-doctor-android-sdk");
        let project = scratch.path().join("project");
        let local_sdk = scratch.path().join("local-sdk");
        let environment_sdk = scratch.path().join("environment-sdk");
        fs::create_dir_all(project.join("android")).expect("create project Android directory");
        fs::create_dir_all(&local_sdk).expect("create local SDK directory");
        fs::create_dir_all(&environment_sdk).expect("create environment SDK directory");
        fs::write(
            project.join("android/local.properties"),
            format!("sdk.dir={}\n", local_sdk.display()),
        )
        .expect("write local Android SDK location");

        let found = android_sdk_path(
            &project,
            Some(environment_sdk.as_os_str()),
            None,
            None,
            None,
        );

        assert_eq!(found, Some(local_sdk));
    }

    #[test]
    fn doctor_checks_minimum_versions_and_points_to_specific_fixes() {
        assert_eq!(
            parse_version("rustc 1.85.2 (abcd 2026-10-01)"),
            Some((1, 85, 2))
        );
        assert_eq!(parse_version("Xcode 27.0"), Some((27, 0, 0)));
        assert_eq!(
            parse_version(r#"openjdk version "17.0.16""#),
            Some((17, 0, 16))
        );
        assert_eq!(doctor_minimum_version("Rust compiler"), Some((1, 85, 0)));
        assert_eq!(doctor_minimum_version("Cargo"), Some((1, 85, 0)));
        assert_eq!(doctor_minimum_version("Xcode"), Some((15, 0, 0)));
        assert_eq!(doctor_minimum_version("Java"), Some((17, 0, 0)));
        assert_eq!(doctor_minimum_version("Clang"), None);

        assert!(doctor_repair_hint("Xcode").contains("xcode-select"));
        assert!(doctor_repair_hint("iOS Simulator").contains("Simulator runtime"));
        assert!(doctor_repair_hint("Java").contains("JDK 17"));
        assert!(doctor_repair_hint("Android platform tools").contains("platform-tools"));
        assert!(doctor_repair_hint("Android SDK").contains("ANDROID_SDK_ROOT"));
    }

    #[test]
    fn doctor_detects_when_xcode_has_no_available_simulator_devices() {
        assert!(has_available_simulator(
            br#"{"devices":{"iOS 27.0":[{"name":"iPhone 17"}]}}"#
        ));
        assert!(!has_available_simulator(br#"{"devices":{"iOS 27.0":[]}}"#));
        assert!(!has_available_simulator(b"not JSON"));
    }

    #[test]
    fn doctor_checks_the_android_platform_used_by_generated_projects() {
        let scratch = TempDir::new("nexa-doctor-android-platform");
        let sdk = scratch.path();
        let platform = sdk
            .join("platforms")
            .join(format!("android-{}", crate::config::ANDROID_COMPILE_SDK));
        fs::create_dir_all(&platform).expect("create Android platform directory");
        let android_jar = platform.join("android.jar");
        fs::write(&android_jar, "").expect("create Android platform stub");

        let prefix = format!("android-{}", crate::config::ANDROID_COMPILE_SDK);
        assert!(android_platform_name_matches_api(&prefix, &prefix));
        assert!(android_platform_name_matches_api(
            &format!("{prefix}.0"),
            &prefix,
        ));
        assert!(!android_platform_name_matches_api(
            &format!("{prefix}0"),
            &prefix,
        ));
        assert!(android_sdk_has_compile_platform(sdk));

        fs::remove_file(android_jar).expect("remove Android platform stub");
        assert!(!android_sdk_has_compile_platform(sdk));
    }

    #[test]
    fn native_preflight_checks_only_tools_needed_by_the_selected_build() {
        let cases = [
            ("android", "dev", true, vec![("java", vec!["-version"])]),
            ("ios", "dev", true, vec![("xcodebuild", vec!["-version"])]),
            (
                "ios",
                "dev",
                false,
                vec![
                    ("xcodebuild", vec!["-version"]),
                    ("xcrun", vec!["--find", "simctl"]),
                ],
            ),
            (
                "all",
                "dev",
                false,
                vec![
                    ("xcodebuild", vec!["-version"]),
                    ("xcrun", vec!["--find", "simctl"]),
                    ("java", vec!["-version"]),
                    ("adb", vec!["version"]),
                ],
            ),
            (
                "android",
                "release",
                false,
                vec![("java", vec!["-version"])],
            ),
        ];

        for (platform, command, compile_only, expected) in cases {
            let mut actual = Vec::new();
            preflight_native_toolchains_with(
                platform,
                command,
                compile_only,
                |program, args, _hint| {
                    actual.push((
                        program.to_owned(),
                        args.iter().map(|argument| (*argument).to_owned()).collect(),
                    ));
                    Ok(())
                },
            )
            .expect("the injected tool checks succeed");
            let expected = expected
                .into_iter()
                .map(|(program, args)| {
                    (
                        program.to_owned(),
                        args.into_iter().map(str::to_owned).collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{platform} {command} preflight");
        }
    }

    #[test]
    fn native_preflight_stops_before_build_when_a_tool_check_fails() {
        let mut calls = Vec::new();
        let error =
            preflight_native_toolchains_with("android", "dev", false, |program, _args, _hint| {
                calls.push(program.to_owned());
                Err(format!("{program} is unavailable"))
            })
            .expect_err("a missing tool should stop the preflight");

        assert_eq!(calls, ["java"]);
        assert_eq!(error, "java is unavailable");
    }

    #[test]
    fn dashboard_rows_clip_text_and_sanitize_terminal_controls() {
        assert_eq!(clip_terminal_text("Nexa dashboard", 4), "Nexa");
        assert_eq!(sanitize_terminal_text("ready\n\u{1b}[31m"), "ready��[31m");

        let mut rendered = Vec::new();
        write_dashboard_row(&mut rendered, 2, 32, "STATUS", Color::Green, "Watching .nx")
            .expect("render a dashboard row");
        let rendered = String::from_utf8(rendered).expect("dashboard output is UTF-8");
        assert!(rendered.contains("STATUS"));
        assert!(rendered.contains("Watching .nx"));
    }

    #[test]
    fn dashboard_frame_shows_project_status_device_logs_and_shortcuts() {
        let dashboard = DevDashboard {
            alternate_screen: false,
            project_name: "Onboarding".to_owned(),
            targets: "iOS + Android".to_owned(),
            server_address: "127.0.0.1:4321".to_owned(),
            status: "Watching .nx sources".to_owned(),
            overlay_enabled: true,
            devices: "iOS: iPhone 17".to_owned(),
            selected_device: Some("iOS: iPhone 17".to_owned()),
            lines: VecDeque::from([DashboardLine {
                prefix: "[iOS]".to_owned(),
                text: "app connected".to_owned(),
                color: Color::Cyan,
            }]),
        };
        let mut rendered = Vec::new();

        dashboard
            .render_into(&mut rendered, 100, 12)
            .expect("render a dashboard frame");
        let rendered = String::from_utf8(rendered).expect("dashboard output is UTF-8");

        for expected in [
            "Onboarding",
            "iOS + Android",
            "Watching .nx sources",
            "Performance overlay: ON",
            "iPhone 17",
            "app connected",
            "r reload",
            "q quit",
        ] {
            assert!(rendered.contains(expected), "missing {expected:?} in frame");
        }
    }

    #[test]
    fn dashboard_frame_handles_short_terminal_height() {
        let dashboard = DevDashboard {
            alternate_screen: false,
            project_name: "Onboarding".to_owned(),
            targets: "iOS + Android".to_owned(),
            server_address: "127.0.0.1:4321".to_owned(),
            status: "Watching .nx sources".to_owned(),
            overlay_enabled: false,
            devices: "No connected devices".to_owned(),
            selected_device: None,
            lines: VecDeque::new(),
        };
        let mut rendered = Vec::new();

        dashboard
            .render_into(&mut rendered, 40, 3)
            .expect("render a compact terminal notice");
        let rendered = String::from_utf8(rendered).expect("dashboard output is UTF-8");
        assert!(rendered.contains("Resize terminal for dashboard"));
        assert!(!rendered.contains("r reload"));
    }

    #[test]
    fn android_device_logs_are_filtered_to_the_app_process() {
        assert_eq!(
            parse_android_process_id("\n 4821  4902\n"),
            Some("4821".to_owned())
        );
        assert_eq!(parse_android_process_id("no process"), None);
        assert!(android_log_line_matches_pid("I/Nexa( 4821): ready", "4821"));
        assert!(!android_log_line_matches_pid(
            "I/Nexa( 4902): ready",
            "4821"
        ));
        assert!(!android_log_line_matches_pid("I/Nexa: ready", "4821"));
    }

    #[test]
    fn dual_platform_device_logs_coalesce_repeated_messages() {
        let now = Instant::now();
        let mut deduplicator = DeviceLogDeduplicator::default();
        let first = DeviceLogLine {
            platform: "iOS".to_owned(),
            device_id: "simulator".to_owned(),
            text: "12:00 Demo[10:20] [dev.nexa.demo] [Nexa][INFO] Saved: https://example.test/item"
                .to_owned(),
        };
        assert_eq!(deduplicator.push(first, now).len(), 1);

        let duplicate = DeviceLogLine {
            platform: "Android".to_owned(),
            device_id: "emulator-5554".to_owned(),
            text: "I/Nexa( 4821): Saved: https://example.test/item".to_owned(),
        };
        assert!(
            deduplicator
                .push(duplicate, now + Duration::from_millis(50))
                .is_empty()
        );

        let summary = deduplicator.flush_expired(now + Duration::from_secs(2));
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].platform, "Android+iOS");
        assert_eq!(
            summary[0].text,
            "Nexa: Saved: https://example.test/item (repeated 2 times)"
        );
    }

    #[test]
    fn dev_diagnostics_preserve_paths_and_source_positions() {
        let entry = std::path::Path::new("fallback/App.nx");
        let diagnostic = parse_dev_diagnostic(
            entry,
            "C:\\work\\My App\\Screen.nx:18:6: error: unknown state `count`",
        );
        assert_eq!(diagnostic.file, "C:\\work\\My App\\Screen.nx");
        assert_eq!(diagnostic.line, 18);
        assert_eq!(diagnostic.column, 6);
        assert_eq!(diagnostic.message, "unknown state `count`");

        let fallback = parse_dev_diagnostic(entry, "native build failed");
        assert_eq!(fallback.file, "fallback/App.nx");
        assert_eq!(fallback.line, 1);
        assert_eq!(fallback.column, 1);
        assert_eq!(fallback.message, "native build failed");
    }

    #[test]
    fn fmt_formats_source_and_check_mode_reports_changes_without_writing() {
        let scratch = TempDir::new("nexa-cli-fmt");
        let source = scratch.path().join("App.nx");
        fs::write(
            &source,
            "fn total(count:Int32)->Int32{return count}\napp Counter{state count:Int32=0\nbody{Text(count)}}\n",
        )
        .expect("write formatter fixture");

        let check_result = run(vec![
            "fmt".to_owned(),
            "--check".to_owned(),
            source.display().to_string(),
        ]);
        assert!(check_result.is_err());
        assert!(
            fs::read_to_string(&source)
                .expect("read unchanged source")
                .contains("app Counter{")
        );

        run(vec!["fmt".to_owned(), source.display().to_string()]).expect("format source file");
        let formatted = fs::read_to_string(&source).expect("read formatted source");
        assert!(formatted.contains("fn total(count: Int32) -> Int32 {"));
        assert!(!formatted.contains("- >"));
        assert!(formatted.contains("app Counter {\n    state count: Int32 = 0"));
        run(vec![
            "fmt".to_owned(),
            "--check".to_owned(),
            source.display().to_string(),
        ])
        .expect("check formatted source");
    }

    #[test]
    fn fmt_validates_all_files_before_writing_changes() {
        let scratch = TempDir::new("nexa-cli-fmt-preflight");
        let valid = scratch.path().join("A.nx");
        let invalid = scratch.path().join("Z.nx");
        let original = "app Counter{body{Text(\"ready\")}}\n";
        fs::write(&valid, original).expect("write valid formatter input");
        fs::write(&invalid, "app Broken { body { Text(\"unterminated) }\n")
            .expect("write invalid formatter input");

        assert!(run(vec!["fmt".to_owned(), scratch.path().display().to_string()]).is_err());
        assert_eq!(
            fs::read_to_string(&valid).expect("read valid source after preflight failure"),
            original,
            "a later invalid file must not leave earlier files reformatted"
        );
    }

    #[test]
    fn native_platform_flags_select_one_or_both_targets() {
        let mut platform = "all".to_owned();
        let mut selected = false;
        select_native_platform(&mut platform, &mut selected, "ios");
        assert_eq!(platform, "ios");
        select_native_platform(&mut platform, &mut selected, "android");
        assert_eq!(platform, "all");

        let mut all = "all".to_owned();
        let mut all_selected = false;
        select_native_platform(&mut all, &mut all_selected, "all");
        select_native_platform(&mut all, &mut all_selected, "ios");
        assert_eq!(all, "all");
    }

    #[test]
    fn build_alias_selects_ipa_or_aab_and_forwards_release_options() {
        assert_eq!(
            release_build_arguments(&[
                "--release".to_owned(),
                "--ipa".to_owned(),
                "--out".to_owned(),
                "ios-release".to_owned(),
            ])
            .expect("infer iOS from --ipa"),
            ["--ios", "--out", "ios-release"]
        );
        assert_eq!(
            release_build_arguments(&[
                "--release".to_owned(),
                "--android".to_owned(),
                "--aab".to_owned(),
                "--locked".to_owned(),
            ])
            .expect("forward Android AAB release"),
            ["--android", "--locked"]
        );
    }

    #[test]
    fn build_alias_requires_release_and_rejects_mismatched_artifacts() {
        assert!(release_build_arguments(&["--ios".to_owned()]).is_err());
        assert!(
            release_build_arguments(&[
                "--release".to_owned(),
                "--android".to_owned(),
                "--ipa".to_owned(),
            ])
            .is_err()
        );
        assert!(
            release_build_arguments(&[
                "--release".to_owned(),
                "--ipa".to_owned(),
                "--aab".to_owned(),
            ])
            .is_err()
        );
    }

    #[test]
    fn device_listing_parsers_select_only_launchable_mobile_targets() {
        let ios = parse_ios_simulator_devices(
            "-- iOS 26.0 --\n    iPhone SE (3rd generation) (11111111-2222-3333-4444-555555555555) (Booted)\n    iPad Pro 13-inch (M5) (aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee) (Shutdown)\n    Apple Watch Series 11 (99999999-2222-3333-4444-555555555555) (Booted)\n",
        );
        assert_eq!(ios.len(), 2);
        assert_eq!(ios[0].name, "iPad Pro 13-inch (M5)");
        assert!(!ios[0].booted);
        assert_eq!(ios[1].name, "iPhone SE (3rd generation)");
        assert!(ios[1].booted);

        let android = parse_android_devices(
            "List of devices attached\nemulator-5554 device product:sdk_gphone64_x86_64 model:Pixel_9_Pro device:generic_x86_64\nR58M12345 unauthorized usb:1-1\n192.0.2.1:5555 offline product:sdk model:Offline_Test\n",
        );
        assert_eq!(android.len(), 1);
        assert_eq!(android[0].id, "emulator-5554");
        assert_eq!(android[0].name, "Pixel 9 Pro");
        assert!(android[0].booted);
    }

    #[test]
    fn dev_watcher_detects_new_imported_screen_and_tab_files() {
        let project = TestProject::new("nexa-dev-watcher-modular-sources");
        let entry = project.write_app("app Demo { body { Text(\"Initial\") } }\n");
        let previous =
            source_fingerprint(project.path(), &[]).expect("fingerprint initial source tree");

        let screen = project.write("screens/Home.nx", "screen Home { Column { HomeTab() } }\n");
        let tab = project.write(
            "tabs/HomeTab.nx",
            "component HomeTab() { body { Text(\"Home tab\") } }\n",
        );
        project.write_app(
            "import \"screens/Home.nx\"\nimport \"tabs/HomeTab.nx\"\napp Demo { body { NavigationStack(root: Home) } }\n",
        );

        let current = source_fingerprint(project.path(), &[])
            .expect("fingerprint source tree with new modules");
        let changed = changed_paths(&previous, &current);

        assert!(changed.contains(&entry));
        assert!(changed.contains(&screen));
        assert!(changed.contains(&tab));
    }

    #[test]
    fn event_watcher_filters_irrelevant_files_and_marks_native_host_changes() {
        let project = TestProject::new("nexa-dev-watcher-event-filter");
        project.write_app("app Demo { body { Text(\"Initial\") } }\n");
        let root = fs::canonicalize(project.path()).expect("canonical project root");
        let source = fs::canonicalize(project.write(
            "screens/Home.nx",
            "screen Home { body { Text(\"Home\") } }\n",
        ))
        .expect("canonical screen path");
        let config = fs::canonicalize(project.write("nexa.config.nx", "config { app: Demo }\n"))
            .expect("canonical config path");
        let lockfile =
            fs::canonicalize(project.write("nexa.lock", "{}\n")).expect("canonical lockfile path");
        let translations = fs::canonicalize(project.write("locales/translations.json", "{}\n"))
            .expect("canonical translations path");
        let asset = fs::canonicalize(project.write("assets/icon.svg", "<svg/>\n"))
            .expect("canonical asset path");
        let generated = root.join("build/generated/Unused.nx");
        let editor_state = root.join(".nexa/signing.properties");
        let plugin_root = project.join("plugins/notes");
        fs::create_dir_all(&plugin_root).expect("create plugin root");
        let plugin_root = fs::canonicalize(plugin_root).expect("canonical plugin root");
        let plugin_source = plugin_root.join("src/Notes.kt");
        let plugin_roots = HashMap::from([("notes".to_owned(), plugin_root.clone())]);
        let watched_plugins = vec![plugin_root.clone()];

        assert!(is_relevant_dev_path(&source, &root, &watched_plugins));
        assert!(!requires_native_rebuild(&source, &plugin_roots));
        assert!(is_relevant_dev_path(&config, &root, &watched_plugins));
        assert!(requires_native_rebuild(&config, &plugin_roots));
        assert!(is_relevant_dev_path(&lockfile, &root, &watched_plugins));
        assert!(requires_native_rebuild(&lockfile, &plugin_roots));
        assert!(is_relevant_dev_path(&translations, &root, &watched_plugins));
        assert!(is_relevant_dev_path(&asset, &root, &watched_plugins));
        assert!(requires_native_rebuild(&asset, &plugin_roots));
        assert!(is_relevant_dev_path(
            &plugin_source,
            &root,
            &watched_plugins
        ));
        assert!(requires_native_rebuild(&plugin_source, &plugin_roots));
        assert!(!is_relevant_dev_path(&generated, &root, &watched_plugins));
        assert!(!is_relevant_dev_path(
            &editor_state,
            &root,
            &watched_plugins
        ));
    }

    #[test]
    fn unit_test_command_loads_imported_test_and_app_function_modules() {
        let project = TestProject::new("nexa-cli-unit-test-command");
        let entry = project.write_app(
            "import \"math.nx\"\nimport \"tests/Cart.nx\"\napp Cart { body { Text(\"Cart\") } }\n",
        );
        project.write(
            "math.nx",
            "fn add(left: Int32, right: Int32) -> Int32 { return left + right }\n",
        );
        project.write(
            "tests/Cart.nx",
            "test \"named test arguments\" { assert(add(right: 2, left: 3) == 5) }\n",
        );

        run_in_language_tests(&entry, "all", false)
            .expect("execute tests from imported source files without native toolchains");
        let output = project.join("build");
        crate::project::run(&[
            entry.display().to_string(),
            "--target".to_owned(),
            "all".to_owned(),
            "--out".to_owned(),
            output.display().to_string(),
            "--name".to_owned(),
            "Cart".to_owned(),
            "--execute-tests".to_owned(),
        ])
        .expect("run in-language tests as part of the native test project pipeline");
    }
}
