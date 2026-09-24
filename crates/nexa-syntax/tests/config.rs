use nexa_syntax::parse_config;

#[test]
fn parses_shared_assets_and_platform_icon_overrides() {
    let config = parse_config(
        r#"config {
            assets { icon: "assets/icon.png", splash: "assets/splash.png" }
            ios { icon: "assets/AppIcon.icon" }
            android { minSdk: 24, targetSdk: 36 }
        }"#,
    )
    .expect("valid multiplatform asset configuration");
    assert_eq!(
        config.assets.expect("assets block").icon.as_deref(),
        Some("assets/icon.png")
    );
    assert_eq!(
        config.ios.expect("ios block").icon.as_deref(),
        Some("assets/AppIcon.icon")
    );
    let android = config.android.expect("android block");
    assert_eq!(android.min_sdk, Some(24));
    assert_eq!(android.target_sdk, Some(36));
}

#[test]
fn requires_git_plugin_dependencies_to_pin_a_revision() {
    let error = parse_config(
        r#"config { dependencies { Math { id: "dev.example.math", git: "https://example.dev/math.git" } } }"#,
    )
    .expect_err("unlocked git dependency should be rejected");
    assert!(error.to_string().contains("pin a `rev`"));
}

#[test]
fn parses_local_and_monorepo_git_plugin_dependencies() {
    let config = parse_config(
        r#"config {
            dependencies {
                LocalMath { id: "dev.example.math", path: "../math" }
                FastMath {
                    id: "dev.example.fast-math",
                    git: "https://example.dev/plugins.git",
                    rev: "0123456789abcdef0123456789abcdef01234567",
                    package: "plugins/fast-math"
                }
                DateTime {
                    id: "dev.example.date-time",
                    git: "https://example.dev/plugins.git",
                    rev: "0123456789abcdef0123456789abcdef01234567",
                    package: "plugins/date-time"
                }
            }
        }"#,
    )
    .expect("valid local and Git plugin dependencies");
    assert_eq!(config.dependencies.len(), 3);
    assert_eq!(config.dependencies[0].path.as_deref(), Some("../math"));
    assert_eq!(
        config.dependencies[1].package_path.as_deref(),
        Some("plugins/fast-math")
    );
    assert_eq!(
        config.dependencies[1].revision,
        config.dependencies[2].revision
    );
    assert_eq!(config.dependencies[1].git, config.dependencies[2].git);
}

#[test]
fn parses_app_staging_suffix() {
    let config =
        parse_config(r#"config { app { displayName: "Nexa", stagingSuffix: "internal" } }"#)
            .expect("valid staging config");
    assert_eq!(
        config.app.expect("app block").staging_suffix.as_deref(),
        Some("internal")
    );
}

#[test]
fn parses_named_flavors_with_optional_suffixes() {
    let config = parse_config(
        r#"config { flavors { staging { suffix: "staging" } production { suffix: "" } qa {} } }"#,
    )
    .expect("valid flavor config");
    assert_eq!(config.flavors.len(), 3);
    assert_eq!(config.flavors[0].name, "staging");
    assert_eq!(config.flavors[0].suffix.as_deref(), Some("staging"));
    assert_eq!(config.flavors[1].suffix.as_deref(), Some(""));
    assert_eq!(config.flavors[2].name, "qa");
    assert_eq!(config.flavors[2].suffix, None);
}
