//! Published plugin tables are views of `plugin.config.nx`.
//!
//! The plugin index in `README.md` carried hand-copied platform floors that had
//! drifted for 14 of the 16 first-party packages -- camera advertised iOS 14
//! against a manifest minimum of 17.0, biometrics advertised Android 23 against
//! `minSdk: 28`. Nothing failed, because a Markdown table has no compiler.
//!
//! These tests render the tables from the manifests and compare them to the
//! checked-in regions, so a manifest change without a documentation change is a
//! test failure rather than a wrong README.

use nexa_cli::docs::{
    PLUGIN_SUMMARIES, load_plugin_packages, render_native_dependencies, render_native_integration,
    render_native_requirements, render_plugin_index,
};

/// Loads the plugin packages, or reports why the suite cannot check them.
///
/// `plugins/` is a submodule. Absence is not a documentation defect, so the
/// tests skip; CI checks out submodules recursively and therefore always runs
/// them.
fn packages() -> Option<Vec<nexa_cli::docs::PluginPackage>> {
    match load_plugin_packages(nexa_testkit::workspace_root()) {
        Ok(Some(packages)) => Some(packages),
        Ok(None) => {
            eprintln!("skipping: plugins/ submodule is not checked out");
            None
        }
        Err(error) => panic!("{error}"),
    }
}

#[test]
fn readme_plugin_index_matches_manifests() {
    let Some(packages) = packages() else {
        return;
    };
    nexa_testkit::assert_region(
        &nexa_testkit::workspace_root().join("README.md"),
        "plugin-index",
        &render_plugin_index(&packages),
        "README.md#plugin-index",
    );
}

#[test]
fn plugin_guide_native_requirements_match_manifests() {
    let Some(packages) = packages() else {
        return;
    };
    nexa_testkit::assert_region(
        &nexa_testkit::workspace_root().join("docs/plugins.md"),
        "native-requirements",
        &render_native_requirements(&packages),
        "docs/plugins.md#native-requirements",
    );
}

#[test]
fn plugin_guide_native_integration_matches_manifests() {
    let Some(packages) = packages() else {
        return;
    };
    nexa_testkit::assert_region(
        &nexa_testkit::workspace_root().join("docs/plugins.md"),
        "native-integration",
        &render_native_integration(&packages),
        "docs/plugins.md#native-integration",
    );
}

#[test]
fn plugin_guide_native_dependencies_match_manifests() {
    let Some(packages) = packages() else {
        return;
    };
    nexa_testkit::assert_region(
        &nexa_testkit::workspace_root().join("docs/plugins.md"),
        "native-dependencies",
        &render_native_dependencies(&packages),
        "docs/plugins.md#native-dependencies",
    );
}

/// Every published plugin appears exactly once in the summary registry.
///
/// Without this, a package could be dropped from a table by an edit to the
/// renderer and the region tests would pass against the smaller table.
#[test]
fn every_package_has_exactly_one_summary() {
    let Some(packages) = packages() else {
        return;
    };
    for package in &packages {
        let matches = PLUGIN_SUMMARIES
            .iter()
            .filter(|summary| summary.id == package.manifest.id)
            .count();
        assert_eq!(
            matches, 1,
            "package `{}` must have exactly one PLUGIN_SUMMARIES entry",
            package.manifest.id
        );
    }
    let published = packages.len();
    assert_eq!(
        PLUGIN_SUMMARIES.len(),
        published,
        "PLUGIN_SUMMARIES lists {} packages but {} are published; \
         remove the stale entry or document the new package",
        PLUGIN_SUMMARIES.len(),
        published,
    );
}

/// The floors this table used to get wrong, asserted against the manifests
/// directly.
///
/// The region tests cover drift in general; this pins the specific floors that
/// were wrong so a regression names the plugin rather than a table cell.
#[test]
fn published_platform_floors_match_each_manifest() {
    let Some(packages) = packages() else {
        return;
    };
    let index = render_plugin_index(&packages);
    for package in &packages {
        let Some(ios) = package.manifest.ios.min_version.as_deref() else {
            continue;
        };
        let Some(android) = package.manifest.android.min_sdk else {
            continue;
        };
        let expected = format!("| iOS {ios}+ \\| Android {android}+ |");
        assert!(
            index.contains(&expected),
            "plugin index must state `{expected}` for `{}`, from its manifest",
            package.manifest.id
        );
    }
}
