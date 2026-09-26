use std::{collections::HashMap, fs, path::PathBuf};

use nexa_compiler::{Target, compile_file_with_warnings_for_targets_and_plugin_roots};

/// A temporary project whose directory is owned for as long as it is bound.
struct TempProject(nexa_testkit::TempDir);

impl TempProject {
    fn new() -> Self {
        Self(nexa_testkit::TempDir::new("nexa-plugin-roots"))
    }
}

#[test]
fn compiler_resolves_plugin_package_ids_to_cli_resolved_roots() {
    let project = TempProject::new();
    let entry = project.0.join("App.nx");
    fs::write(
        &entry,
        "plugin \"dev.nexa.fast-math\" as FastMath\napp Demo { state result: Int32 = 0 body { Button(\"Compute\") { result = FastMath.add(10, 20) } } }\n",
    )
    .expect("write app source");
    let plugin_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/plugins/fast-math")
        .canonicalize()
        .expect("resolve example plugin package");
    let plugin_roots = HashMap::from([("dev.nexa.fast-math".to_owned(), plugin_root.clone())]);

    let compilations = compile_file_with_warnings_for_targets_and_plugin_roots(
        &entry,
        &[Target::Swift, Target::Kotlin],
        &plugin_roots,
    )
    .expect("compile app using package ID plugin declaration");
    assert_eq!(compilations.len(), 2);
    for compilation in compilations {
        assert_eq!(compilation.module.plugins.len(), 1);
        assert_eq!(
            PathBuf::from(&compilation.module.plugins[0].idl_path),
            plugin_root.join("native.nxid")
        );
    }
}
