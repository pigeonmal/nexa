use std::{collections::HashMap, path::PathBuf};

use nexa_compiler::{Target, compile_file_with_warnings_for_targets_and_plugin_roots};
use nexa_testkit::TestProject;

#[test]
fn compiler_resolves_plugin_package_ids_to_cli_resolved_roots() {
    let project = TestProject::new("nexa-plugin-roots");
    let entry = project.write_app(
        "plugin \"dev.nexa.fast-math\" as FastMath\napp Demo { state result: Int32 = 0 body { Button(\"Compute\") { result = FastMath.add(10, 20) } } }\n",
    );
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
