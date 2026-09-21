use std::{env, fs, path::PathBuf, process};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::compile_file;

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
    if args.len() != 1 {
        return Err("usage: nexa check <source.nx>".into());
    }
    let path = PathBuf::from(&args[0]);
    compile_file(&path).map_err(|error| error.to_string())?;
    println!("checked {}", path.display());
    Ok(())
}

fn build(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
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
    let backend: &dyn Backend = match target {
        "swift" => &SwiftBackend,
        "kotlin" => &KotlinBackend,
        _ => {
            return Err(format!(
                "unknown target `{target}`; expected `swift` or `kotlin`"
            ));
        }
    };
    let module = compile_file(&input).map_err(|error| error.to_string())?;
    let output = output.unwrap_or_else(|| input.with_extension(backend.file_extension()));
    fs::write(&output, backend.generate(&module))
        .map_err(|error| format!("{}: {error}", output.display()))?;
    println!("generated {} ({})", output.display(), backend.name());
    Ok(())
}

fn print_help() {
    println!(
        "Nexa — ahead-of-time compiler for native iOS and Android UI\n\n\
Usage:\n  nexa check <source.nx>\n  nexa build <source.nx> --target <swift|kotlin> [--out <path>]\n\n\
Targets emit native SwiftUI or Jetpack Compose source."
    );
}
