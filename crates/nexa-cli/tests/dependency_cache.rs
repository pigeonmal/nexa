#[allow(dead_code)]
#[path = "../src/cache.rs"]
mod cache;

use std::{
    collections::BTreeMap,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

struct TempProject(std::path::PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "nexa-dependency-cache-{}-{nonce}",
            std::process::id()
        ));
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
fn project_cache_key_tracks_resolved_plugin_package_contents() {
    let project = TempProject::new();
    let entry = project.0.join("App.nx");
    fs::write(&entry, "app Demo { body { Text(\"ok\") } }\n").expect("write app");
    let plugin_root = project.0.join("plugins/fast-math");
    fs::create_dir_all(&plugin_root).expect("create plugin package");
    let contract = plugin_root.join("native.nxid");
    fs::write(
        &contract,
        "service Math { fn add(a: Int32, b: Int32) -> Int32 }\n",
    )
    .expect("write plugin contract");
    let roots = BTreeMap::from([("dev.example.fast-math".to_owned(), plugin_root)]);

    let first = cache::key_with_extra_and_roots(&entry, "project-all", &[], &roots)
        .expect("first cache key");
    fs::write(
        &contract,
        "service Math { fn add(a: Int64, b: Int64) -> Int64 }\n",
    )
    .expect("change plugin contract");
    let second = cache::key_with_extra_and_roots(&entry, "project-all", &[], &roots)
        .expect("updated cache key");
    assert_ne!(first, second);
}

#[test]
fn dev_cache_key_resolves_package_ids_to_local_plugin_roots() {
    let project = TempProject::new();
    let entry = project.0.join("App.nx");
    fs::write(
        &entry,
        "plugin \"dev.example.fast-math\" as FastMath\napp Demo { body { Text(\"ready\") } }\n",
    )
    .expect("write app with package-ID plugin declaration");
    let plugin_root = project.0.join("plugins/fast-math");
    fs::create_dir_all(&plugin_root).expect("create local plugin package");
    fs::write(
        plugin_root.join("plugin.config.nx"),
        "plugin { schema: 2 id: \"dev.example.fast-math\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
    )
    .expect("write plugin manifest");
    let contract = plugin_root.join("native.nxid");
    fs::write(
        &contract,
        "service Math { fn add(a: Int32, b: Int32) -> Int32 }\n",
    )
    .expect("write plugin contract");
    let roots = BTreeMap::from([("dev.example.fast-math".to_owned(), plugin_root)]);

    let first = cache::key_with_extra_and_roots(&entry, "dev-android", &[], &roots)
        .expect("package ID should resolve to its local dependency root");
    fs::write(
        &contract,
        "service Math { fn add(a: Int64, b: Int64) -> Int64 }\n",
    )
    .expect("change plugin contract");
    let second = cache::key_with_extra_and_roots(&entry, "dev-android", &[], &roots)
        .expect("fingerprint updated local dependency");
    assert_ne!(
        first, second,
        "plugin contract changes invalidate the dev revision"
    );
}
