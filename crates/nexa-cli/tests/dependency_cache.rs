#[allow(dead_code)]
#[path = "../src/cache.rs"]
mod cache;

use std::{collections::BTreeMap, fs};

/// A temporary project whose directory is owned for as long as it is bound.
struct TempProject(nexa_testkit::TempDir);

impl TempProject {
    fn new() -> Self {
        Self(nexa_testkit::TempDir::new("nexa-dependency-cache"))
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

#[test]
fn app_image_assets_invalidate_project_and_dev_cache_keys() {
    let project = TempProject::new();
    let entry = project.0.join("App.nx");
    fs::write(&entry, "app Demo { body { Text(\"ok\") } }\n").expect("write app");
    let images = project.0.join("assets/images");
    let extras = [images.as_path()];
    let roots = BTreeMap::new();

    let missing = cache::key_with_extra_and_roots(&entry, "project-all", &extras, &roots)
        .expect("cache key without optional images");
    fs::create_dir_all(&images).expect("create image assets");
    let image = images.join("logo.png");
    fs::write(&image, b"first image contents").expect("add image asset");
    let added = cache::key_with_extra_and_roots(&entry, "project-all", &extras, &roots)
        .expect("cache key with image");
    assert_ne!(
        missing, added,
        "adding an image invalidates project generation"
    );

    fs::write(&image, b"updated image contents").expect("modify image asset");
    let modified = cache::key_with_extra_and_roots(&entry, "dev-android", &extras, &roots)
        .expect("dev revision with updated image");
    fs::write(&image, b"first image contents").expect("restore image asset");
    let restored = cache::key_with_extra_and_roots(&entry, "dev-android", &extras, &roots)
        .expect("dev revision with restored image");
    assert_ne!(
        modified, restored,
        "image bytes participate in dev revision"
    );

    fs::remove_file(image).expect("remove image asset");
    let removed = cache::key_with_extra_and_roots(&entry, "project-all", &extras, &roots)
        .expect("cache key without removed image");
    assert_ne!(
        added, removed,
        "removing an image invalidates project generation"
    );
}
