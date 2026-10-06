use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, BufRead, IsTerminal},
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

pub(super) fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("create") => create(&args[1..]),
        Some("check") => check_project(&args[1..]),
        Some("audit") => crate::audit::run(&args[1..]),
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

fn check_project(args: &[String]) -> Result<(), String> {
    let mut platform = "all".to_owned();
    let mut platform_set = false;
    let mut deny_warnings = false;
    let mut locked = false;
    let mut audit_enabled = false;
    for argument in args {
        match argument.as_str() {
            "--ios" => set_platform(&mut platform, &mut platform_set, "ios")?,
            "--android" => set_platform(&mut platform, &mut platform_set, "android")?,
            "--deny-warnings" => deny_warnings = true,
            "--audit" => audit_enabled = true,
            "--locked" => locked = true,
            "--help" | "-h" => {
                println!(
                    "Usage: nexa check [--ios | --android] [--deny-warnings] [--audit] [--locked]"
                );
                return Ok(());
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value => {
                return Err(format!(
                    "unexpected argument `{value}`; run `nexa check --help`"
                ));
            }
        }
    }

    let root = env::current_dir().map_err(|error| error.to_string())?;
    let entry = find_entry(&root)?;
    let entry_source =
        fs::read_to_string(&entry).map_err(|error| format!("{}: {error}", entry.display()))?;
    let program = nexa_syntax::parse_program(&entry_source)
        .map_err(|error| format!("{}: {error}", entry.display()))?;
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
    )
    .map_err(|error| error.to_string())?;
    let mut warnings = std::collections::BTreeSet::new();
    for (target, compilation) in targets.iter().copied().zip(compilations) {
        if target == nexa_compiler::Target::Kotlin {
            let packages = crate::project::plugin_package::packages_for_module(
                &compilation.plugins,
                &compilation.module,
            );
            let requirements = packages
                .iter()
                .map(|plugin| (plugin.namespace.as_str(), plugin.artifacts.android_min_sdk))
                .collect::<Vec<_>>();
            project_config.validate_android_plugin_minimums(&requirements)?;
            crate::project::validate_android_background_task_minimum(
                &compilation.module,
                &project_config,
            )?;
        }
        if target == nexa_compiler::Target::Swift {
            let packages = crate::project::plugin_package::packages_for_module(
                &compilation.plugins,
                &compilation.module,
            );
            crate::project::validate_ios_background_task_minimum(
                &compilation.module,
                &packages,
                &project_config,
            )?;
        }
        for warning in compilation.warnings {
            warnings.insert(warning.to_string());
        }
    }
    for warning in &warnings {
        eprintln!("{warning}");
    }
    if deny_warnings && !warnings.is_empty() {
        return Err(format!("{} warning(s) treated as errors", warnings.len()));
    }
    println!("check passed ({platform})");
    if audit_enabled {
        crate::audit::run(&[entry.display().to_string(), "--target".to_owned(), platform])?;
    }
    Ok(())
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
    let mut locked = false;
    let mut unit_only = false;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--ios" => {
                set_platform(&mut platform, &mut platform_set, "ios")?;
            }
            "--android" => {
                set_platform(&mut platform, &mut platform_set, "android")?;
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
                set_platform(&mut platform, &mut platform_set, value)?;
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
    if unit_only {
        return run_in_language_tests(&entry, &platform, locked);
    }
    if command == "release" && matches!(platform.as_str(), "android" | "all") {
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
        let mut dev_compiler = nexa_compiler::IncrementalProjectCompiler::default();
        let modules = super::project::compile_dev_modules_with_compiler(
            &entry,
            &platform,
            &mut dev_compiler,
        )?;
        let mut background_tasks = background_task_fingerprint(&modules)?;
        let server = nexa_dev_server::DevServer::bind(modules)?;
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
        print_dev_welcome();
        println!("Nexa dev server listening at ws://{}", server.address());
        build_platforms(
            &output,
            &project_name,
            &platform,
            BuildMode::Dev,
            Some(server.address().port()),
        )?;
        println!("Watching .nx sources. Press Ctrl-C to stop.");
        let watch_context = DevWatchContext {
            root: &root,
            entry: &entry,
            platform: &platform,
            project_name: &project_name,
            output: &output,
            project_args: &project_args,
            server: &server,
        };
        watch_sources(&watch_context, &mut background_tasks, &mut dev_compiler)
    } else {
        super::project::run(&project_args)?;
        build_platforms(&output, &project_name, &platform, BuildMode::Release, None)
    }
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
) -> Result<(), String> {
    let dependencies =
        crate::config::load_plugin_dependencies(&context.root.join("nexa.config.nx"))?;
    let resolved = crate::dependencies::resolve(context.root, &dependencies)?;
    let mut plugin_roots = resolved.plugin_roots.into_values().collect::<Vec<_>>();
    let mut previous = source_fingerprint(context.root, &plugin_roots)?;
    let mut performance_overlay_enabled = false;
    let mut native_rebuild_pending = false;
    let (console_commands, _raw_terminal) = start_dev_console_input();
    loop {
        match console_commands.try_recv() {
            Ok(DevConsoleCommand::HotReload) => {
                println!("Hot reloading Nexa source...");
                match reload_or_rebuild_dev_app(context, background_tasks, compiler) {
                    Ok(true) => println!("Native app rebuilt for background task changes."),
                    Ok(false) => println!("Nexa source reloaded in the running app."),
                    Err(error) => {
                        eprintln!("{error}");
                        publish_dev_error(context.server, context.entry, context.platform, error);
                    }
                }
            }
            Ok(DevConsoleCommand::HotRestart) => {
                println!("Hot restarting Nexa app state...");
                match reload_or_rebuild_dev_app(context, background_tasks, compiler) {
                    Ok(rebuilt) => {
                        if !rebuilt {
                            context.server.publish_restart("hot restart requested");
                        }
                        println!("Nexa app state restarted in the running app.");
                    }
                    Err(error) => {
                        eprintln!("{error}");
                        publish_dev_error(context.server, context.entry, context.platform, error);
                    }
                }
            }
            Ok(DevConsoleCommand::Rebuild) => {
                println!("Rebuilding native app...");
                let rebuilt = match rebuild_dev_app(context, compiler) {
                    Ok(task_fingerprint) => {
                        *background_tasks = task_fingerprint;
                        println!("Native app rebuilt and relaunched.");
                        true
                    }
                    Err(error) => {
                        eprintln!("native dev rebuild failed: {error}");
                        false
                    }
                };
                if rebuilt {
                    let dependencies = crate::config::load_plugin_dependencies(
                        &context.root.join("nexa.config.nx"),
                    )?;
                    let resolved = crate::dependencies::resolve(context.root, &dependencies)?;
                    plugin_roots = resolved.plugin_roots.into_values().collect();
                }
                previous = source_fingerprint(context.root, &plugin_roots)?;
                if rebuilt {
                    native_rebuild_pending = false;
                }
                continue;
            }
            Ok(DevConsoleCommand::TogglePerformanceOverlay) => {
                performance_overlay_enabled = !performance_overlay_enabled;
                context
                    .server
                    .set_performance_overlay(performance_overlay_enabled);
                let state = if performance_overlay_enabled {
                    "on"
                } else {
                    "off"
                };
                println!("Nexa performance overlay {state}.");
            }
            Ok(DevConsoleCommand::Stop) => {
                println!("Stopping Nexa dev.");
                return Ok(());
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
        thread::sleep(Duration::from_millis(350));
        let current = match source_fingerprint(context.root, &plugin_roots) {
            Ok(current) => current,
            Err(error) => {
                eprintln!("warning: {error}");
                continue;
            }
        };
        if current == previous {
            continue;
        }
        let changed = changed_paths(&previous, &current);
        let host_changed = changed.iter().any(|path| {
            path.file_name()
                .is_some_and(|name| name == "nexa.lock" || name == "nexa.config.nx")
                || is_native_asset(path)
                || plugin_roots
                    .iter()
                    .any(|plugin_root| path.starts_with(plugin_root))
        });
        previous = current;
        if host_changed {
            if !native_rebuild_pending {
                println!(
                    "Native host changes are waiting. Press 'b' to rebuild and relaunch the app."
                );
            }
            native_rebuild_pending = true;
            continue;
        }
        if native_rebuild_pending {
            continue;
        }
        thread::sleep(Duration::from_millis(200));
        match reload_or_rebuild_dev_app(context, background_tasks, compiler) {
            Ok(true) => {
                native_rebuild_pending = false;
                println!("Background task changes rebuilt and relaunched the native app.");
            }
            Ok(false) => println!("Nexa source reloaded in the running app."),
            Err(error) => {
                eprintln!("{error}");
                publish_dev_error(context.server, context.entry, context.platform, error);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DevConsoleCommand {
    HotReload,
    HotRestart,
    Rebuild,
    TogglePerformanceOverlay,
    Stop,
}

fn print_dev_welcome() {
    println!(
        "\nWelcome to Nexa dev\n  r          Hot reload the current source\n  Shift+R    Hot restart and reset app state\n  b          Rebuild and relaunch the native app\n  p          Toggle the performance overlay (FPS and frame time)\n  Ctrl-C     Stop the dev session\n\nBuild and device logs will appear below.\n"
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

fn start_dev_console_input() -> (Receiver<DevConsoleCommand>, Option<RawTerminalGuard>) {
    let (sender, receiver) = mpsc::channel();
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
                            KeyCode::Char('p' | 'P') => {
                                Some(DevConsoleCommand::TogglePerformanceOverlay)
                            }
                            _ => None,
                        };
                        if let Some(command) = command {
                            let stop = command == DevConsoleCommand::Stop;
                            if sender.send(command).is_err() || stop {
                                break;
                            }
                        }
                    }
                });
                return (receiver, Some(guard));
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
                "p" | "P" => Some(DevConsoleCommand::TogglePerformanceOverlay),
                "q" | "quit" | "exit" => Some(DevConsoleCommand::Stop),
                "" => None,
                value => {
                    eprintln!("unknown Nexa dev shortcut `{value}`; use r, R, b, or p");
                    None
                }
            };
            if let Some(command) = command {
                let stop = command == DevConsoleCommand::Stop;
                if sender.send(command).is_err() || stop {
                    break;
                }
            }
        }
    });
    (receiver, None)
}

fn reload_or_rebuild_dev_app(
    context: &DevWatchContext<'_>,
    background_tasks: &mut String,
    compiler: &mut nexa_compiler::IncrementalProjectCompiler,
) -> Result<bool, String> {
    let modules = super::project::compile_dev_modules_with_compiler(
        context.entry,
        context.platform,
        compiler,
    )?;
    let next_fingerprint = background_task_fingerprint(&modules)?;
    if next_fingerprint != *background_tasks {
        println!(
            "Background task declarations or actions changed; rebuilding the native host because the OS runs compiled handlers."
        );
        *background_tasks = rebuild_dev_app(context, compiler)?;
        return Ok(true);
    }
    for (target, module) in modules {
        context.server.publish_module(target, module)?;
    }
    Ok(false)
}

fn rebuild_dev_app(
    context: &DevWatchContext<'_>,
    compiler: &mut nexa_compiler::IncrementalProjectCompiler,
) -> Result<String, String> {
    let modules = super::project::compile_dev_modules_with_compiler(
        context.entry,
        context.platform,
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
    let diagnostic = nexa_dev_protocol::Diagnostic {
        severity: nexa_dev_protocol::Severity::Error,
        file: entry.display().to_string(),
        line: 1,
        column: 1,
        message,
    };
    for target in dev_targets(platform) {
        let _ = server.publish_diagnostics(target, vec![diagnostic.clone()]);
    }
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
    require_command(
        "xcodebuild",
        "Install Xcode and select it with xcode-select.",
    )?;
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
    command.current_dir(root.join("android")).arg(task);
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
    for (name, args) in [
        ("rustc", vec!["--version"]),
        ("cargo", vec!["--version"]),
        ("xcodebuild", vec!["-version"]),
        ("xcrun", vec!["simctl", "list", "devices", "available"]),
        ("java", vec!["-version"]),
        ("adb", vec!["version"]),
    ] {
        match Command::new(name).args(args).output() {
            Ok(output) if output.status.success() => println!("✓ {name}"),
            _ => {
                failed = true;
                println!("✗ {name}");
            }
        }
    }
    if failed {
        Err("one or more platform tools are missing; see https://nexa.dev/docs/setup".to_owned())
    } else {
        Ok(())
    }
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
    let devices = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available"])
        .output()
        .map_err(|error| format!("xcrun simctl: {error}"))?;
    let listing = String::from_utf8_lossy(&devices.stdout);
    let booted = listing.lines().find(|line| line.contains("(Booted)"));
    let candidate = booted.or_else(|| {
        listing
            .lines()
            .find(|line| line.contains("iPhone") && line.contains("("))
    });
    let Some(line) = candidate else {
        return Err(
            "no available iOS Simulator found; install an iOS Simulator runtime in Xcode"
                .to_owned(),
        );
    };
    let udid = line
        .split('(')
        .nth(1)
        .and_then(|part| part.split(')').next())
        .ok_or("could not read Simulator device ID")?;
    if !line.contains("(Booted)") {
        let mut boot = Command::new("xcrun");
        boot.args(["simctl", "boot", udid]);
        run_command(boot, "Simulator boot")?;
    }
    let mut bootstatus = Command::new("xcrun");
    bootstatus.args(["simctl", "bootstatus", udid, "-b"]);
    run_command(bootstatus, "Simulator boot")?;
    let app = root
        .join("ios-derived/Build/Products/Debug-iphonesimulator")
        .join(format!("{app_name}.app"));
    let bundle_id = bundle_identifier(root, app_name)?;
    let mut install = Command::new("xcrun");
    install.args(["simctl", "install", udid]).arg(&app);
    run_command(install, "Simulator install")?;
    let mut launch = Command::new("xcrun");
    launch.args(["simctl", "launch", udid, &bundle_id]);
    run_command(launch, "Simulator launch")?;
    Ok(())
}

fn launch_android_emulator(root: &Path, dev_port: Option<u16>) -> Result<(), String> {
    require_command(
        "adb",
        "Install Android platform-tools and start an emulator.",
    )?;
    let devices = Command::new("adb")
        .args(["devices"])
        .output()
        .map_err(|error| error.to_string())?;
    let listing = String::from_utf8_lossy(&devices.stdout);
    if !listing
        .lines()
        .skip(1)
        .any(|line| line.ends_with("\tdevice"))
    {
        return Err("no Android emulator or device is connected; start one, then rerun `nexa dev --android`".to_owned());
    }
    if let Some(port) = dev_port {
        let local = format!("tcp:{port}");
        let remote = local.clone();
        let mut reverse = Command::new("adb");
        reverse.args(["reverse", local.as_str(), remote.as_str()]);
        run_command(reverse, "Android dev server port forwarding")?;
    }
    let package = android_application_id(root)?;
    let apk = root.join("android/app/build/outputs/apk/debug/app-debug.apk");
    let mut install = Command::new("adb");
    install.arg("install").arg("-r").arg(apk);
    run_command(install, "Android install")?;
    let mut launch = Command::new("adb");
    launch.args(["shell", "monkey", "-p", &package, "1"]);
    run_command(launch, "Android launch")?;
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

fn starter_source(name: &str) -> String {
    format!(
        "app {name} {{\n    state count: Int32 = 0\n\n    body {{\n        Column(spacing: 16) {{\n            Text(\"Welcome to {name}\")\n            Text(\"Count: $count\")\n            Button(\"Add one\") {{\n                count = count + 1\n            }}\n        }}\n    }}\n}}\n"
    )
}

fn starter_config(name: &str) -> String {
    let id = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    format!(
        "config {{\n    app {{ displayName: \"{name}\", version: \"1.0.0\", buildNumber: 1, deepLinks: [] }}\n    flavors {{ staging {{ suffix: \"staging\" }} }}\n    ios {{ minVersion: \"16.0\", bundleIdentifier: \"dev.nexa.{id}\" }}\n    android {{ minSdk: 23, targetSdk: 36, applicationId: \"dev.nexa.{id}\", cronet {{ provider: \"play-services\", diskCacheSizeMb: 64 }} }}\n    permissions {{}}\n}}\n"
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

fn require_command(program: &str, hint: &str) -> Result<(), String> {
    if Command::new(program).arg("--version").output().is_ok() {
        Ok(())
    } else {
        Err(format!("`{program}` is required. {hint}"))
    }
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
        "Nexa — native iOS and Android apps from one .nx project\n\nUsage:\n  nexa create <ProjectName>\n  nexa check [--ios | --android] [--audit] [--locked]\n  nexa audit <source.nx> [--target <ios|android|all>] [--release-sizes] [--out <path>]\n  nexa dev [--ios | --android] [--locked]\n  nexa test [--ios | --android] [--locked]\n  nexa test --unit-only [--ios | --android] [--locked]\n  nexa release [--ios | --android] [--locked]\n  nexa doctor\n  nexa plugin init | check | generate\n\nRun `nexa <command> --help` for command options."
    );
}

fn print_command_help(command: &str) {
    match command {
        "create" => println!("Usage: nexa create <ProjectName> [--directory <path>]"),
        "check" => {
            println!("Usage: nexa check [--ios | --android] [--deny-warnings] [--audit] [--locked]")
        }
        "audit" => crate::audit::print_help(),
        "dev" => {
            println!(
                "Usage: nexa dev [--ios | --android] [--arch <architecture>] [--once | --compile-only] [--flavor <name>] [--out <directory>] [--locked]\nWhile running: `r` hot reloads, `Shift+R` hot restarts, `b` rebuilds and relaunches, and `p` toggles the performance overlay."
            )
        }
        "test" => {
            println!(
                "Usage: nexa test [--ios | --android] [--arch <architecture>] [--flavor <name>] [--out <directory>] [--locked]\n       nexa test --unit-only [--ios | --android] [--locked]"
            )
        }
        "release" => println!(
            "Usage: nexa release [--ios | --android] [--arch <architecture>] [--flavor <name>] [--out <directory>] [--locked]\nBuilds an iOS archive or Android AAB. Android signing uses .nexa/signing.properties or NEXA_ANDROID_* environment variables; iOS signing uses Xcode."
        ),
        "plugin" => println!(
            "Usage:\n  nexa plugin init <plugin.id> --out <directory> [--name <TypeName>] [--kind native|pure]\n  nexa plugin check <package-directory|native.nxid>\n  nexa plugin generate <package-directory|native.nxid> --target <swift|kotlin|cpp> [--package <name>] [--out <directory>]"
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
    use super::{changed_paths, run_in_language_tests, source_fingerprint};
    use nexa_testkit::TestProject;

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
