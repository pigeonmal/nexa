use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use nexa_compiler::{Target, compile_file_with_warnings_for_targets_and_plugin_roots};

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("nexa-plugin-roots-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("create temporary project");
        Self(path)
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
