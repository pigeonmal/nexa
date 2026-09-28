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
fn parses_platform_architecture_options() {
    let config = parse_config(
        r#"config {
            ios { arch: "arm64" }
            android { arch: "arm64" }
        }"#,
    )
    .expect("valid platform architecture configuration");
    assert_eq!(
        config.ios.expect("iOS config").arch.as_deref(),
        Some(["arm64".to_owned()].as_slice())
    );
    assert_eq!(
        config.android.expect("Android config").arch.as_deref(),
        Some(["arm64".to_owned()].as_slice())
    );
}

#[test]
fn parses_multiple_platform_architectures() {
    let config = parse_config(
        r#"config {
            ios { arch: ["arm64", "x86_64"] }
            android { arch: ["arm64", "x86_64"] }
        }"#,
    )
    .expect("valid architecture arrays");
    assert_eq!(
        config.ios.expect("iOS config").arch.unwrap(),
        ["arm64", "x86_64"]
    );
    assert_eq!(
        config.android.expect("Android config").arch.unwrap(),
        ["arm64", "x86_64"]
    );
}

#[test]
fn parses_android_cronet_provider_and_disk_cache_options() {
    let config = parse_config(
        r#"config {
            android {
                minSdk: 24,
                cronet {
                    provider: "embedded",
                    diskCacheSizeMb: 128
                }
            }
        }"#,
    )
    .expect("valid Android Cronet configuration");
    let cronet = config
        .android
        .expect("Android config")
        .cronet
        .expect("Cronet config");
    assert_eq!(cronet.provider.as_deref(), Some("embedded"));
    assert_eq!(cronet.disk_cache_size_mb, Some(128));
}

#[test]
fn rejects_duplicate_android_cronet_blocks_and_fields() {
    for (source, expected) in [
        (
            r#"config { android { cronet {}, cronet {} } }"#,
            "Android cronet config is declared more than once",
        ),
        (
            r#"config { android { cronet { provider: "embedded", provider: "play-services" } } }"#,
            "Android Cronet provider is declared more than once",
        ),
    ] {
        let error = parse_config(source).expect_err("duplicate Cronet config should fail");
        assert!(error.to_string().contains(expected), "{error}");
    }
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
fn parses_app_deep_link_bases() {
    let config =
        parse_config(r#"config { app { deepLinks: ["nexa://", "https://links.example.com"] } }"#)
            .expect("valid deep-link bases");
    assert_eq!(
        config.app.expect("app block").deep_links,
        ["nexa://", "https://links.example.com"]
    );
}

#[test]
fn rejects_non_string_deep_link_entries() {
    let error = parse_config(r#"config { app { deepLinks: [true] } }"#)
        .expect_err("deep-link bases must be strings");
    assert!(error.to_string().contains("entries must be quoted strings"));
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
