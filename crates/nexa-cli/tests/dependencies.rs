#[path = "../src/dependencies.rs"]
mod dependencies;

use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use nexa_syntax::ast::PluginDependencyConfig;

static TEMP_PROJECT_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

struct TempProject(std::path::PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = TEMP_PROJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "nexa-dependencies-{}-{nonce}-{sequence}",
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

fn dependency(alias: &str, id: &str, path: &str) -> PluginDependencyConfig {
    PluginDependencyConfig {
        alias: alias.to_owned(),
        package_id: id.to_owned(),
        path: Some(path.to_owned()),
        git: None,
        revision: None,
        package_path: None,
        span: Default::default(),
    }
}

fn write_package(root: &Path, id: &str) {
    fs::create_dir_all(root).expect("create plugin package");
    fs::write(
        root.join("plugin.config.nx"),
        format!("plugin {{ schema: 2 id: \"{id}\" version: \"1.0.0\" sources {{ native: \"native.nxid\" }} }}\n"),
    )
    .expect("write plugin manifest");
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[test]
fn resolves_multiple_packages_from_one_pinned_git_revision_and_writes_stable_lock() {
    let project = TempProject::new();
    let repository = project.0.join("repository");
    fs::create_dir_all(&repository).expect("create repository");
    git(&repository, &["init", "--quiet"]);
    git(
        &repository,
        &["config", "user.email", "nexa-tests@example.invalid"],
    );
    git(&repository, &["config", "user.name", "Nexa tests"]);
    write_package(
        &repository.join("plugins/fast-math"),
        "dev.example.fast-math",
    );
    write_package(
        &repository.join("plugins/date-time"),
        "dev.example.date-time",
    );
    git(&repository, &["add", "."]);
    git(
        &repository,
        &["commit", "--quiet", "-m", "add plugin packages"],
    );
    let revision = git(&repository, &["rev-parse", "HEAD"]);
    let repository_url = repository.to_string_lossy().into_owned();
    let dependencies = [
        PluginDependencyConfig {
            alias: "FastMath".to_owned(),
            package_id: "dev.example.fast-math".to_owned(),
            path: None,
            git: Some(repository_url.clone()),
            revision: Some(revision.clone()),
            package_path: Some("plugins/fast-math".to_owned()),
            span: Default::default(),
        },
        PluginDependencyConfig {
            alias: "DateTime".to_owned(),
            package_id: "dev.example.date-time".to_owned(),
            path: None,
            git: Some(repository_url),
            revision: Some(revision.clone()),
            package_path: Some("plugins/date-time".to_owned()),
            span: Default::default(),
        },
    ];

    let first = dependencies::resolve(&project.0, &dependencies).expect("resolve pinned plugins");
    let fast_math = &first.plugin_roots["dev.example.fast-math"];
    let date_time = &first.plugin_roots["dev.example.date-time"];
    assert!(fast_math.ends_with("plugins/fast-math"));
    assert!(date_time.ends_with("plugins/date-time"));
    assert_eq!(
        fast_math.parent().unwrap().parent().unwrap(),
        date_time.parent().unwrap().parent().unwrap()
    );

    dependencies::write_lock(&project.0, &first.lock_file).expect("write Nexa lockfile");
    let lock_path = project.0.join("nexa.lock");
    let original_lock = fs::read_to_string(&lock_path).expect("read lockfile");
    assert!(original_lock.contains(&revision));
    assert!(original_lock.contains("dev.example.fast-math"));
    assert!(original_lock.contains("plugins/date-time"));

    let second =
        dependencies::resolve(&project.0, &dependencies).expect("repeat locked resolution");
    dependencies::write_lock(&project.0, &second.lock_file).expect("rewrite identical lockfile");
    assert_eq!(fs::read_to_string(lock_path).unwrap(), original_lock);

    let mut escaping = dependencies[0].clone();
    escaping.package_path = Some("../../outside".to_owned());
    let error = dependencies::resolve(&project.0, &[escaping])
        .expect_err("Git package paths must remain inside their checkout");
    assert!(error.contains("without `..` segments"));
}

#[test]
fn resolves_local_plugin_dependencies_and_rejects_a_mismatched_manifest_id() {
    let project = TempProject::new();
    write_package(&project.0.join("plugins/math"), "dev.example.math");
    let resolved = dependencies::resolve(
        &project.0,
        &[dependency("Math", "dev.example.math", "plugins/math")],
    )
    .expect("resolve local plugin");
    assert!(resolved.plugin_roots["dev.example.math"].ends_with("plugins/math"));
    assert!(resolved.lock_file.contains("path:plugins/math"));

    let error = dependencies::resolve(
        &project.0,
        &[dependency("Math", "dev.example.other", "plugins/math")],
    )
    .expect_err("mismatched manifest id must fail");
    assert!(error.contains("package manifest is `dev.example.math`"));
}

#[test]
fn locked_dependencies_detect_missing_lockfiles_and_local_plugin_changes() {
    let project = TempProject::new();
    let package = project.0.join("plugins/math");
    write_package(&package, "dev.example.math");
    fs::write(
        package.join("native.nxid"),
        "service Math { fn add(a: Int32, b: Int32) -> Int32 }\n",
    )
    .expect("write plugin interface");
    let dependencies = [dependency("Math", "dev.example.math", "plugins/math")];
    let resolved = dependencies::resolve(&project.0, &dependencies).expect("resolve plugin");

    let missing = dependencies::sync_lock(&project.0, true, &resolved.lock_file, true)
        .expect_err("locked resolution requires a lockfile");
    assert!(missing.contains("`nexa.lock` is missing"));

    dependencies::sync_lock(&project.0, true, &resolved.lock_file, false).expect("write lockfile");
    dependencies::sync_lock(&project.0, true, &resolved.lock_file, true)
        .expect("accept matching lockfile");

    fs::write(package.join("native.nxid"), "service Math { fn add(a: Int32, b: Int32) -> Int32; fn sub(a: Int32, b: Int32) -> Int32 }\n")
        .expect("change plugin interface");
    let changed = dependencies::resolve(&project.0, &dependencies).expect("resolve changed plugin");
    let stale = dependencies::sync_lock(&project.0, true, &changed.lock_file, true)
        .expect_err("locked resolution rejects changed local packages");
    assert!(stale.contains("`nexa.lock` is out of date"));
}
