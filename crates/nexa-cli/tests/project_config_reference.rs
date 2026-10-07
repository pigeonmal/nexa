//! The published configuration and CLI reference must match the parser.
//!
//! `docs/getting-started.md` carried hand-written tables that had drifted from
//! the code they described: `stagingSuffix` was documented as defaulting to
//! "Unset" while `ProjectConfig::from_defaults` used `"staging"`, and the CLI
//! matrix omitted `--platform`, `--out`, `--staging`, `--locked`, and `--arch`
//! even though `native_command` accepts all of them on `dev`, `test`, and
//! `release`.
//!
//! The tables are generated. This suite also checks the generated flag lists
//! against the parsers themselves, because a generated table is only as honest
//! as the data fed into it.

use nexa_cli::docs::{commands, render_command_matrix, render_config_options};

fn getting_started() -> std::path::PathBuf {
    nexa_testkit::workspace_root().join("docs/getting-started.md")
}

#[test]
fn config_options_reference_matches_resolved_defaults() {
    nexa_testkit::assert_region(
        &getting_started(),
        "config-options",
        &render_config_options().expect("resolve scaffold defaults"),
        "docs/getting-started.md#config-options",
    );
}

#[test]
fn cli_command_matrix_matches_published_commands() {
    nexa_testkit::assert_region(
        &getting_started(),
        "cli-commands",
        &render_command_matrix(),
        "docs/getting-started.md#cli-commands",
    );
}

/// The defaults that were wrong in the published table.
///
/// The region test catches any drift; naming them keeps a regression legible.
#[test]
fn published_defaults_state_the_resolved_values() {
    let rendered = render_config_options().expect("resolve scaffold defaults");
    for expected in [
        "| `app` | `stagingSuffix` | `String` | `\"staging\"` |",
        "| `app` | `orientation` | `String` | `\"all\"` |",
        "| `app` | `version` | `String` | `\"1.0.0\"` |",
        "| `app` | `buildNumber` | `Int32` | `1` |",
        "| `ios` | `minVersion` | `String` | `\"16.0\"` |",
        "| `android` | `minSdk` | `Int32` | `23` |",
        "| `android` | `targetSdk` | `Int32` | `36` |",
        "| `android.cronet` | `diskCacheSizeMb` | `Int32` | `64` |",
        "| `android.cronet` | `provider` | `String` | `\"play-services\"` |",
    ] {
        assert!(
            rendered.contains(expected),
            "configuration reference must state `{expected}`"
        );
    }
}

/// Native build commands publish the flags they share.
///
/// `native_command` parses one argument loop for `dev`, `test`, and `release`;
/// `build` routes into the same release pipeline. All four accept the shared
/// platform, architecture, flavor, output, and lock flags.
///
/// This asserts publication, not parser acceptance: `native_command` is private
/// to the binary and reaching it would mean running a real build. The flag
/// spelling is checked against the parser's own match arms in
/// `flags_match_parser_spelling`.
#[test]
fn native_build_commands_publish_shared_flags() {
    let matrix = render_command_matrix();
    for flag in [
        "--ios",
        "--android",
        "--platform",
        "--arch",
        "--flavor",
        "--staging",
        "--out",
        "--locked",
    ] {
        let occurrences = matrix
            .lines()
            .filter(|line| {
                line.starts_with("| `nexa dev")
                    || line.starts_with("| `nexa test")
                    || line.starts_with("| `nexa release")
                    || line.starts_with("| `nexa build")
            })
            .filter(|line| line.contains(&format!("`{flag}")))
            .count();
        assert_eq!(
            occurrences, 4,
            "`{flag}` is accepted by dev, test, release, and build, so all four rows must publish it"
        );
    }
}

/// Flag spellings match the parser's match arms.
///
/// The parsers are hand-written `match` statements over `&str`, so a rename
/// there is invisible to a generated table. This reads the parser source and
/// checks each published flag against the spellings it accepts, which is what
/// turns the published list into a checked claim rather than a restatement.
#[test]
fn flags_match_parser_spelling() {
    // `nexa audit` parses in its own module, so both are searched: a flag is
    // valid if any parser in the CLI accepts it.
    let mut parsers = String::new();
    for module in ["commands.rs", "audit.rs", "plugin_cli.rs"] {
        parsers.push_str(
            &std::fs::read_to_string(
                nexa_testkit::workspace_root()
                    .join("crates/nexa-cli/src")
                    .join(module),
            )
            .unwrap_or_else(|error| panic!("read {module}: {error}")),
        );
    }
    for entry in commands() {
        for flag in entry.flags {
            let spelling = flag.trim_matches('`');
            let name = spelling
                .split([' ', '<', '|'])
                .next()
                .expect("a flag has a name");
            assert!(
                parsers.contains(&format!("\"{name}\"")),
                "`{name}` is published for {} but no CLI parser accepts it",
                entry.command
            );
        }
    }
}

/// Command-specific flags are only published for the commands that accept them.
#[test]
fn dev_only_flags_are_not_published_for_other_commands() {
    let matrix = render_command_matrix();
    let row = |command: &str| {
        matrix
            .lines()
            .find(|line| line.starts_with(&format!("| `{command}")))
            .unwrap_or_else(|| panic!("the matrix must publish `{command}`"))
            .to_owned()
    };
    assert!(row("nexa dev").contains("`--once`"));
    assert!(row("nexa dev").contains("`--compile-only`"));
    assert!(!row("nexa test").contains("`--once`"));
    assert!(!row("nexa release").contains("`--compile-only`"));
    assert!(!row("nexa build").contains("`--compile-only`"));
    assert!(row("nexa test").contains("`--unit-only`"));
    assert!(!row("nexa dev").contains("`--unit-only`"));
}

/// The published command set matches what the CLI routes.
#[test]
fn published_commands_cover_every_routed_command() {
    let published: Vec<&str> = commands().map(|entry| entry.command).collect();
    for command in [
        "`nexa create <name>`",
        "`nexa check`",
        "`nexa fmt`",
        "`nexa dev`",
        "`nexa test`",
        "`nexa build`",
        "`nexa release`",
        "`nexa doctor`",
        "`nexa audit <file>`",
        "`nexa plugin <subcommand>`",
    ] {
        assert!(
            published.contains(&command),
            "the CLI matrix must publish `{command}`"
        );
    }
}
