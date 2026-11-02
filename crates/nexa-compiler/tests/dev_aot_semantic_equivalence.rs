//! Semantic acceptance must not depend on AOT vs DevRuntime lowering.
//!
//! `nexa dev` compiles through `IncrementalProjectCompiler`'s cached dev
//! entry point; `nexa build` uses the normal entry point. If the two disagreed
//! on source acceptance, a green dev session could fail release builds. These
//! tests drive both real entry points for both native targets and compare
//! acceptance, diagnostics and the canonical typed IR.

use std::collections::HashMap;
use std::fs;

use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_testkit::TestProject;

const STORE_MANIFEST: &str = "plugin { schema: 2 id: \"dev.test.store\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n";

const STORE_CONTRACT: &str = "native class Store {\n\
       init(id: String)\n\
       fn setObject<T>(key: String, value: T) -> Bool\n\
       fn getObject<T>(key: String) -> T?\n\
       fn getString(key: String) -> String?\n\
       fn dispose()\n\
     }\n";

struct Outcome {
    accepted: bool,
    diagnostics: Vec<String>,
    canonical_ir: Option<serde_json::Value>,
    warnings: Vec<String>,
}

fn project_with_plugin(name: &str, app_source: &str) -> (TestProject, std::path::PathBuf) {
    let project = TestProject::new(name);
    let plugin = project.join("store");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(plugin.join("plugin.config.nx"), STORE_MANIFEST)
        .expect("plugin manifest should be written");
    fs::write(plugin.join("native.nxid"), STORE_CONTRACT)
        .expect("plugin contract should be written");
    let entry = project.write_app(app_source);
    (project, entry)
}

fn compile_mode(entry: &std::path::Path, target: Target, dev_runtime: bool) -> Outcome {
    let mut compiler = IncrementalProjectCompiler::default();
    let roots: HashMap<String, std::path::PathBuf> = HashMap::new();
    let result = if dev_runtime {
        compiler.compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
            entry,
            &[target],
            &roots,
        )
    } else {
        compiler.compile_file_with_warnings_for_targets_and_plugin_roots(entry, &[target], &roots)
    };
    match result {
        Ok(mut compilations) => {
            let compilation = compilations.remove(0);
            Outcome {
                accepted: true,
                diagnostics: Vec::new(),
                canonical_ir: Some(
                    serde_json::to_value(&compilation.module)
                        .expect("typed IR should serialize canonically"),
                ),
                warnings: compilation
                    .warnings
                    .iter()
                    .map(|warning| format!("{:?}:{}", warning.code, warning.message))
                    .collect(),
            }
        }
        Err(error) => Outcome {
            accepted: false,
            diagnostics: vec![format!(
                "{}:{}:{}: {}",
                error.file.as_deref().unwrap_or("<none>"),
                error.span.line,
                error.span.column,
                error.message
            )],
            canonical_ir: None,
            warnings: Vec::new(),
        },
    }
}

fn assert_mode_equivalence(name: &str, app_source: &str) -> Outcome {
    let (_project, entry) = project_with_plugin(name, app_source);
    let mut reference: Option<Outcome> = None;
    for target in [Target::Swift, Target::Kotlin] {
        let aot = compile_mode(&entry, target, false);
        let dev = compile_mode(&entry, target, true);
        assert_eq!(
            aot.accepted, dev.accepted,
            "{name} ({target:?}) acceptance diverged between AOT and DevRuntime"
        );
        assert_eq!(
            aot.diagnostics, dev.diagnostics,
            "{name} ({target:?}) diagnostics diverged between AOT and DevRuntime"
        );
        assert_eq!(
            aot.warnings, dev.warnings,
            "{name} ({target:?}) warnings diverged between AOT and DevRuntime"
        );
        assert_eq!(
            aot.canonical_ir, dev.canonical_ir,
            "{name} ({target:?}) canonical IR diverged between AOT and DevRuntime"
        );
        if reference.is_none() {
            reference = Some(aot);
        }
    }
    reference.expect("at least one target compiled")
}

#[test]
fn nullable_top_level_generic_plugin_read_is_rejected_in_both_modes() {
    let outcome = assert_mode_equivalence(
        "nexa-dev-aot-nullable-generic",
        "plugin \"store\" as Store\n\
         app Demo {\n\
           let store = Store.Store(\"id\")\n\
           body { Button(\"Read\") { store.getObject<String?>(\"maybe\") } }\n\
         }\n",
    );
    assert!(!outcome.accepted, "the read must be rejected in both modes");
    assert!(
        outcome.diagnostics[0].contains("must not be optional at the top level"),
        "unexpected diagnostic: {:?}",
        outcome.diagnostics
    );
}

#[test]
fn nullable_compound_element_generic_plugin_read_is_accepted_in_both_modes() {
    let outcome = assert_mode_equivalence(
        "nexa-dev-aot-nullable-compound",
        "plugin \"store\" as Store\n\
         app Demo {\n\
           let store = Store.Store(\"id\")\n\
           state values: Array<String?> = [\"one\", null]\n\
           body { Button(\"Read\") { values = store.getObject<Array<String?>>(\"values\") ?? [] } }\n\
         }\n",
    );
    assert!(
        outcome.accepted,
        "compound nullable elements stay supported"
    );
}

#[test]
fn result_with_non_enum_error_is_rejected_in_both_modes() {
    let outcome = assert_mode_equivalence(
        "nexa-dev-aot-result-error",
        "plugin \"store\" as Store\n\
         app Demo {\n\
           let store = Store.Store(\"id\")\n\
           body { Button(\"Read\") { store.getObject<Result<Float64, String>>(\"k\") } }\n\
         }\n",
    );
    assert!(!outcome.accepted);
    assert!(
        outcome.diagnostics[0].contains("must use an enum error type"),
        "unexpected diagnostic: {:?}",
        outcome.diagnostics
    );
}

#[test]
fn screens_components_effects_and_async_calls_agree_across_modes() {
    let outcome = assert_mode_equivalence(
        "nexa-dev-aot-broad-shape",
        r#"plugin "store" as Store

struct Player {
    name: String,
    score: Int32
}

enum LoadError {
    missing,
    corrupt
}

component ScoreRow(player: Player) {
    state expanded: Bool = false
    body {
        Row {
            Text("${player.name}: ${player.score}")
            Button(expanded ? "Hide" : "Show") { expanded = !expanded }
        }
    }
}

app Demo {
    let store = Store.Store("id")
    state players: Array<Player> = [Player(name: "Ada", score: 10)]
    state loadError: LoadError? = null

    screen Home {
        state selected: Bool = true

        OnAppear {
            selected = true
        }

        Column {
            Switch(value: selected, label: "Enabled")
            FastList(players, key: .name) { player, index in
                ScoreRow(player: player)
            }
        }
    }

    screen Settings {
        state enabled: Bool = true

        Text(enabled ? "On" : "Off")
    }

    body {
        OnActive {
            players = store.getObject<Array<Player>>("players") ?? []
        }
        NavigationStack(root: Home)
    }
}
"#,
    );
    assert!(
        outcome.accepted,
        "the broad fixture must compile: {:?}",
        outcome.diagnostics
    );
}
