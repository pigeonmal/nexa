use std::{collections::HashSet, env, fs, path::PathBuf, process};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::{CompileWarning, Target, compile_file_with_warnings_for_target};

mod plugin;
mod project;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("build") => build(&args[1..]),
        Some("generate") => project::run(&args[1..]),
        Some("plugin") => plugin::run(&args[1..]),
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

fn check(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut deny_warnings = false;
    for argument in args {
        match argument.as_str() {
            "--deny-warnings" => deny_warnings = true,
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            value if path.is_none() => path = Some(PathBuf::from(value)),
            value => return Err(format!("unexpected argument `{value}`")),
        }
    }
    let path = path.ok_or("usage: nexa check <source.nx> [--deny-warnings]")?;
    let mut warnings = Vec::new();
    for target in [Target::Swift, Target::Kotlin] {
        let compilation = compile_file_with_warnings_for_target(&path, target)
            .map_err(|error| error.to_string())?;
        warnings.extend(compilation.warnings);
    }
    let warnings = deduplicate_warnings(warnings);
    report_warnings(&warnings, deny_warnings)?;
    println!("checked {}", path.display());
    Ok(())
}

fn build(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut deny_warnings = false;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--deny-warnings" => deny_warnings = true,
            "--target" | "-t" => {
                cursor += 1;
                target = Some(
                    args.get(cursor)
                        .ok_or("`--target` requires `swift` or `kotlin`")?
                        .as_str(),
                );
            }
            "--out" | "--output" | "-o" => {
                cursor += 1;
                output = Some(PathBuf::from(
                    args.get(cursor).ok_or("`--out` requires a path")?,
                ));
            }
            option if option.starts_with('-') => return Err(format!("unknown option `{option}`")),
            path if input.is_none() => input = Some(PathBuf::from(path)),
            path => return Err(format!("unexpected argument `{path}`")),
        }
        cursor += 1;
    }

    let input =
        input.ok_or("usage: nexa build <source.nx> --target <swift|kotlin> [--out <path>]")?;
    let target = target.ok_or("`nexa build` requires `--target swift` or `--target kotlin`")?;
    let (backend, compile_target): (&dyn Backend, Target) = match target {
        "swift" => (&SwiftBackend, Target::Swift),
        "kotlin" => (&KotlinBackend, Target::Kotlin),
        _ => {
            return Err(format!(
                "unknown target `{target}`; expected `swift` or `kotlin`"
            ));
        }
    };
    let compilation = compile_file_with_warnings_for_target(&input, compile_target)
        .map_err(|error| error.to_string())?;
    report_warnings(&compilation.warnings, deny_warnings)?;
    let output = output.unwrap_or_else(|| input.with_extension(backend.file_extension()));
    fs::write(&output, backend.generate(&compilation.module))
        .map_err(|error| format!("{}: {error}", output.display()))?;
    println!("generated {} ({})", output.display(), backend.name());
    Ok(())
}

fn report_warnings(warnings: &[CompileWarning], deny_warnings: bool) -> Result<(), String> {
    for warning in warnings {
        eprintln!("{warning}");
    }
    if deny_warnings && !warnings.is_empty() {
        return Err(format!("{} warning(s) treated as errors", warnings.len()));
    }
    Ok(())
}

fn deduplicate_warnings(warnings: Vec<CompileWarning>) -> Vec<CompileWarning> {
    let mut seen = HashSet::new();
    warnings
        .into_iter()
        .filter(|warning| seen.insert(warning.to_string()))
        .collect()
}

fn print_help() {
    println!(
        "Nexa — ahead-of-time compiler for native iOS and Android UI\n\n\
Usage:\n  nexa check <source.nx> [--deny-warnings]\n  nexa build <source.nx> --target <swift|kotlin> [--out <path>] [--deny-warnings]\n  nexa generate <source.nx> [--target <ios|android|all>] [--out <directory>] [--name <AppName>] [--deny-warnings]\n  nexa plugin init <plugin.id> [--out <directory>] [--name <TypeName>] [--version <version>]\n  nexa plugin check <plugin-directory|interfaces.nxid>\n  nexa plugin generate <plugin-directory|interfaces.nxid> --target <swift|kotlin> [--out <file>]\n\n\
Targets emit native SwiftUI or Jetpack Compose source. `generate` creates a self-contained native project bundle. `plugin init` creates an isolated optional-plugin scaffold; `plugin check` validates its typed IDL and `plugin generate` emits direct native binding skeletons."
    );
}
