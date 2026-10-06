#[allow(dead_code)]
#[path = "../src/config.rs"]
mod config;
use config::ProjectConfig;

struct NativeWidgetSource {
    relative_path: String,
    #[allow(dead_code)]
    contents: String,
}

#[allow(dead_code)]
#[path = "../src/project/plugin_package.rs"]
mod plugin_package;

mod plugins {
    pub(super) fn cpp_header_include_roots(
        _plugin: &crate::plugin_package::PluginPackage,
        _plugin_index: usize,
    ) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    pub(super) fn minimum_cpp_standard(plugins: &[crate::plugin_package::PluginPackage]) -> u8 {
        plugins
            .iter()
            .filter_map(|plugin| plugin.artifacts.cpp_standard)
            .max()
            .unwrap_or(20)
    }
}

#[allow(dead_code)]
#[path = "../src/project/pbxproj.rs"]
mod pbxproj;

#[allow(dead_code)]
#[path = "../src/project/templates.rs"]
mod templates;

#[test]
fn android_wrapper_is_pinned_and_checksum_verified() {
    let properties = templates::android_gradle_wrapper_properties();
    assert!(properties.contains("gradle-9.4.1-bin.zip"));
    assert!(properties.contains("distributionSha256Sum="));
    assert!(templates::ANDROID_GRADLE_WRAPPER_JAR.starts_with(b"PK"));
    assert!(
        templates::ANDROID_GRADLEW.contains("-jar \"$APP_HOME/gradle/wrapper/gradle-wrapper.jar\"")
    );
    assert!(
        templates::ANDROID_GRADLEW_BAT
            .contains("-jar \"%APP_HOME%\\gradle\\wrapper\\gradle-wrapper.jar\"")
    );
}

#[test]
fn ios_localization_catalog_is_a_compiled_bundle_resource() {
    let config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
    let project = templates::ios_project_file_with_localization_config(
        "Demo",
        false,
        false,
        true,
        false,
        &["NexaGenerated.swift".to_owned()],
        &[],
        &[],
        &[],
        &[],
        &config,
    )
    .expect("localized iOS project should render");

    assert!(project.contains("text.json.xcstrings"));
    assert!(project.contains("Localizable.xcstrings"));
    assert!(project.contains("PBXResourcesBuildPhase"));
}

#[test]
fn android_release_signing_is_configured_at_build_time() {
    let config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    let gradle = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures::default(),
        &[],
        &[],
        &config,
        false,
    )
    .unwrap();
    assert!(gradle.contains("create(\"nexaRelease\")"));
    assert!(gradle.contains("System.getenv(\"NEXA_ANDROID_KEYSTORE\")"));
    assert!(gradle.contains("signingConfigs.getByName(\"nexaRelease\")"));
    assert!(!gradle.contains("androidx.work:work-runtime-ktx"));
}

#[test]
fn android_background_tasks_require_workmanager_minimum_sdk() {
    let mut config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    config.android_min_sdk = 22;
    let error = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures {
            uses_background_tasks: true,
            ..Default::default()
        },
        &[],
        &[],
        &config,
        false,
    )
    .expect_err("WorkManager 2.11.2 requires API 23 or later");
    assert!(error.contains("require minSdk 23"));
}

#[test]
fn android_architecture_selects_a_single_ndk_abi() {
    let mut config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    config.android_arch = Some(vec!["arm64".to_owned()]);
    let gradle = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures::default(),
        &[],
        &[],
        &config,
        false,
    )
    .unwrap();
    assert!(gradle.contains("ndk { abiFilters.addAll(listOf(\"arm64-v8a\")) }"));
}

#[test]
fn android_architecture_array_selects_multiple_ndk_abis() {
    let mut config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    config.android_arch = Some(vec!["arm64".to_owned(), "x86_64".to_owned()]);
    let gradle = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures::default(),
        &[],
        &[],
        &config,
        false,
    )
    .unwrap();
    assert!(gradle.contains("ndk { abiFilters.addAll(listOf(\"arm64-v8a\", \"x86_64\")) }"));
}

#[test]
fn android_architecture_names_map_to_supported_abis() {
    assert_eq!(config::android_abi_for_arch("arm64").unwrap(), "arm64-v8a");
    assert_eq!(
        config::android_abi_for_arch("armv7").unwrap(),
        "armeabi-v7a"
    );
    assert!(config::android_abi_for_arch("sparc").is_err());
}

#[test]
fn android_dev_runtime_preloads_compose_and_platform_dependencies() {
    let config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    let dependencies = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures::default(),
        &[],
        &[],
        &config,
        true,
    )
    .unwrap();

    for dependency in [
        "androidx.compose.foundation:foundation",
        "androidx.compose.runtime:runtime",
        "androidx.compose.animation:animation",
        "androidx.navigation:navigation-compose",
        "androidx.lifecycle:lifecycle-runtime-compose",
        "org.jetbrains.kotlinx:kotlinx-coroutines-android",
        "androidx.core:core",
        "io.coil-kt.coil3:coil-compose",
        "io.coil-kt.coil3:coil-network-core",
        "com.google.android.gms:play-services-cronet",
    ] {
        assert!(
            dependencies.contains(&format!("implementation(\"{dependency}")),
            "dev dependency {dependency} should be preloaded"
        );
    }
    assert!(
        dependencies.contains("implementation(\"androidx.navigation:navigation-compose:2.9.8\")")
    );
    assert!(!dependencies.contains("org.chromium.net:cronet-embedded"));
}

#[test]
fn android_aot_dependencies_remain_feature_gated() {
    let config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    let dependencies = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures::default(),
        &[],
        &[],
        &config,
        false,
    )
    .unwrap();

    for dependency in [
        "androidx.compose.foundation:foundation",
        "androidx.compose.runtime:runtime",
        "androidx.compose.animation:animation",
        "androidx.navigation:navigation-compose",
        "androidx.lifecycle:lifecycle-runtime-compose",
        "org.jetbrains.kotlinx:kotlinx-coroutines-android",
        "androidx.core:core",
        "io.coil-kt.coil3:coil-compose",
        "io.coil-kt.coil3:coil-network-core",
        "com.google.android.gms:play-services-cronet",
        "org.chromium.net:cronet-embedded",
    ] {
        assert!(
            !dependencies.contains(dependency),
            "AOT dependency {dependency} should remain feature-gated"
        );
    }
}

#[test]
fn android_compose_animation_dependency_is_feature_gated_for_aot() {
    let config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    let dependencies = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures {
            uses_compose_animation: true,
            ..Default::default()
        },
        &[],
        &[],
        &config,
        false,
    )
    .unwrap();

    assert!(dependencies.contains("implementation(\"androidx.compose.animation:animation\")"));
}

#[test]
fn android_network_release_uses_play_services_cronet_by_default() {
    let config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    let dependencies = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures {
            uses_network: true,
            ..Default::default()
        },
        &[],
        &[],
        &config,
        false,
    )
    .unwrap();

    assert!(dependencies.contains("com.google.android.gms:play-services-cronet"));
    assert!(!dependencies.contains("org.chromium.net:cronet-embedded"));
}

#[test]
fn android_embedded_cronet_is_an_explicit_provider_choice() {
    let mut config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    config.android_cronet_provider = config::AndroidCronetProvider::Embedded;
    config.android_cronet_disk_cache_size_mb = 0;
    let dependencies = templates::android_app_gradle_with_dev_runtime(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures {
            uses_network: true,
            ..Default::default()
        },
        &[],
        &[],
        &config,
        false,
    )
    .unwrap();

    assert!(dependencies.contains("org.chromium.net:cronet-embedded:143.7445.0"));
    assert!(!dependencies.contains("com.google.android.gms:play-services-cronet"));
}

#[test]
fn normalized_project_config_renders_cronet_defaults_and_round_trips() {
    let config = ProjectConfig::from_defaults(&[], "demo").unwrap();
    assert_eq!(
        config.android_cronet_provider,
        config::AndroidCronetProvider::PlayServices
    );
    assert_eq!(config.android_cronet_disk_cache_size_mb, 64);

    let rendered = config.render();
    let parsed = nexa_syntax::parse_config(&rendered).expect("rendered config should parse");
    let cronet = parsed
        .android
        .expect("rendered Android config")
        .cronet
        .expect("rendered Cronet config");
    assert_eq!(cronet.provider.as_deref(), Some("play-services"));
    assert_eq!(cronet.disk_cache_size_mb, Some(64));
}

mod template_generation {
    use std::fs;

    use crate::ProjectConfig;
    use crate::templates;
    use crate::templates::*;

    #[allow(clippy::too_many_arguments)]
    fn ios_project_file(
        app_name: &str,
        has_assets: bool,
        has_plugin_resources: bool,
        generated_sources: &[String],
        plugin_sources: &[String],
        cpp_sources: &[String],
        xcframeworks: &[String],
        plugins: &[crate::plugin_package::PluginPackage],
    ) -> Result<String, String> {
        ios_project_file_with_privacy_manifest(
            app_name,
            has_assets,
            has_plugin_resources,
            false,
            generated_sources,
            plugin_sources,
            cpp_sources,
            xcframeworks,
            plugins,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn ios_project_file_with_privacy_manifest(
        app_name: &str,
        has_assets: bool,
        has_plugin_resources: bool,
        has_privacy_manifest: bool,
        generated_sources: &[String],
        plugin_sources: &[String],
        cpp_sources: &[String],
        xcframeworks: &[String],
        plugins: &[crate::plugin_package::PluginPackage],
    ) -> Result<String, String> {
        let config = ProjectConfig::from_defaults(&[], app_name)?;
        ios_project_file_with_config(
            app_name,
            has_assets,
            has_plugin_resources,
            has_privacy_manifest,
            generated_sources,
            plugin_sources,
            cpp_sources,
            xcframeworks,
            plugins,
            &config,
        )
    }

    #[test]
    fn ios_privacy_manifest_is_feature_gated_and_uses_app_only_reasons() {
        let storage = templates::ios_privacy_manifest(nexa_backend_swift::SwiftProjectFeatures {
            requires_user_defaults_reason: true,
            requires_file_timestamp_reason: false,
            uses_screen_orientation_api: false,
        })
        .expect("storage uses a required-reason API");
        assert!(storage.contains("NSPrivacyAccessedAPICategoryUserDefaults"));
        assert!(storage.contains("CA92.1"));
        assert!(!storage.contains("NSPrivacyAccessedAPICategoryFileTimestamp"));
        assert!(
            templates::ios_privacy_manifest(nexa_backend_swift::SwiftProjectFeatures::default())
                .is_none()
        );
    }

    #[test]
    fn ios_project_includes_privacy_manifest_as_a_resource_only_when_needed() {
        let sources = ["NexaGenerated.swift".to_owned()];
        let with_manifest = ios_project_file_with_privacy_manifest(
            "PrivacyDemo",
            false,
            false,
            true,
            &sources,
            &[],
            &[],
            &[],
            &[],
        )
        .expect("privacy-enabled project should render");
        assert!(with_manifest.contains("path = PrivacyInfo.xcprivacy;"));
        let privacy_build_file = pbx_build_file_for_path(&with_manifest, "PrivacyInfo.xcprivacy");
        let resource_phase = with_manifest
            .lines()
            .find(|line| line.contains("isa = PBXResourcesBuildPhase;"))
            .expect("project should contain a resource phase");
        assert!(resource_phase.contains(&privacy_build_file));

        let without_manifest = ios_project_file_with_privacy_manifest(
            "PrivacyDemo",
            false,
            false,
            false,
            &sources,
            &[],
            &[],
            &[],
            &[],
        )
        .expect("privacy-free project should render");
        assert!(!without_manifest.contains("PrivacyInfo.xcprivacy"));
    }

    #[test]
    fn ios_architecture_is_applied_to_debug_and_release() {
        let mut config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        config.ios_arch = Some(vec!["arm64".to_owned(), "x86_64".to_owned()]);
        let project = ios_project_file_with_config(
            "Demo",
            false,
            false,
            false,
            &["NexaGenerated.swift".to_owned()],
            &[],
            &[],
            &[],
            &[],
            &config,
        )
        .unwrap();
        assert_eq!(project.matches("ARCHS = \"arm64 x86_64\";").count(), 2);
    }

    fn android_app_gradle(
        package: &str,
        features: nexa_backend_kotlin::KotlinProjectFeatures,
        plugins: &[crate::plugin_package::PluginPackage],
        local_aars: &[String],
    ) -> Result<String, String> {
        let config = ProjectConfig::from_defaults(&[], package)?;
        android_app_gradle_with_dev_runtime(package, features, plugins, local_aars, &config, false)
    }

    fn ios_info_plist(
        app_name: &str,
        config: &ProjectConfig,
        plugins: &[crate::plugin_package::PluginPackage],
    ) -> Result<String, String> {
        ios_info_plist_with_orientation(app_name, config, plugins, false, false, &[])
    }

    fn plugin(namespace: &str) -> crate::plugin_package::PluginPackage {
        crate::plugin_package::PluginPackage {
            namespace: namespace.to_owned(),
            idl_path: String::new(),
            artifacts: Default::default(),
        }
    }

    /// Collect every `ID = { isa = ...` object key from a rendered pbxproj.
    fn pbx_object_ids(project: &str) -> Vec<String> {
        let mut ids = Vec::new();
        let mut search = project;
        while let Some(end) = search.find(" = { isa = ") {
            let start = search[..end]
                .rfind(['\t', ' ', '\n', '{'])
                .map(|index| index + 1)
                .unwrap_or(0);
            ids.push(search[start..end].to_owned());
            search = &search[end + 1..];
        }
        ids
    }

    fn is_pbx_id(value: &str) -> bool {
        value.len() == 24
            && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            && value.bytes().all(|byte| !byte.is_ascii_lowercase())
    }

    /// Find the `PBXBuildFile` whose `fileRef` points at the file reference
    /// with the given `path = ...` value.
    fn pbx_object_id_on_line(project: &str, position: usize) -> String {
        let line_start = project[..position]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let line = &project[line_start..];
        let id = line
            .split(" = ")
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        assert!(
            is_pbx_id(&id),
            "expected a 24-hex PBX object key on line `{line}`"
        );
        id
    }

    fn pbx_build_file_for_path(project: &str, path: &str) -> String {
        let needle = format!("path = {path};");
        let file_end = project
            .find(&needle)
            .unwrap_or_else(|| panic!("expected a file reference with `{needle}`"));
        let file_id = pbx_object_id_on_line(project, file_end);
        let build_needle = format!("isa = PBXBuildFile; fileRef = {file_id};");
        let build_end = project
            .find(&build_needle)
            .expect("expected a build file for the file reference");
        pbx_object_id_on_line(project, build_end)
    }

    #[test]
    fn pbx_identifiers_stay_unique_at_plugin_scale() {
        // Regression test for the old numeric-range scheme, where plugin file
        // references at 30+index collided with generated file references at
        // 40+index once 11 plugin Swift files existed. 30 plugin files, 10
        // generated units, frameworks, XCFrameworks, SwiftPM packages, C++
        // sources, splash, icons, assets, and plugin resources must all
        // receive distinct 24-hex object identifiers.
        let mut config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        config.splash_source = Some(std::path::PathBuf::from("assets/splash.png"));
        config.ios_icon = Some(std::path::PathBuf::from("assets/AppIcon.icon"));

        let mut generated = vec!["NexaGenerated.swift".to_owned()];
        for index in 0..9 {
            generated.push(format!("NexaGenerated_Section{index}.swift"));
        }
        let plugin_sources: Vec<String> = (0..30)
            .map(|index| format!("NexaPlugin{index}_Bindings.swift"))
            .collect();
        let cpp_sources = vec![
            "Plugin0/cpp/Sources/Decoder.cpp".to_owned(),
            "Plugin1/cpp/Sources/Encoder.cpp".to_owned(),
        ];
        let xcframeworks = vec![
            "Frameworks/NexaPlugin0_VideoSDK.xcframework".to_owned(),
            "Frameworks/NexaPlugin1_AudioSDK.xcframework".to_owned(),
        ];
        let mut media_plugin = plugin("Media");
        media_plugin.artifacts.ios_frameworks =
            vec!["AVFoundation".to_owned(), "CoreMedia".to_owned()];
        media_plugin
            .artifacts
            .swift_packages
            .push(nexa_plugin_idl::manifest::SwiftPackage {
                url: "https://example.com/media.git".to_owned(),
                requirement: nexa_plugin_idl::manifest::SwiftPackageRequirement::From(
                    "2.3.0".to_owned(),
                ),
                products: vec!["MediaKit".to_owned(), "MediaUI".to_owned()],
                extension_products: Vec::new(),
            });
        let mut maps_plugin = plugin("Maps");
        maps_plugin.artifacts.ios_frameworks = vec!["MapKit".to_owned()];
        maps_plugin
            .artifacts
            .swift_packages
            .push(nexa_plugin_idl::manifest::SwiftPackage {
                url: "https://example.com/maps.git".to_owned(),
                requirement: nexa_plugin_idl::manifest::SwiftPackageRequirement::From(
                    "1.0.0".to_owned(),
                ),
                products: vec!["MapsKit".to_owned()],
                extension_products: Vec::new(),
            });
        let plugins = [media_plugin, maps_plugin];

        let project = ios_project_file_with_config(
            "Demo",
            true,
            true,
            false,
            &generated,
            &plugin_sources,
            &cpp_sources,
            &xcframeworks,
            &plugins,
            &config,
        )
        .expect("scaled iOS project should render");

        // Generation must be deterministic: the same inputs produce the same
        // identifiers on every run.
        let again = ios_project_file_with_config(
            "Demo",
            true,
            true,
            false,
            &generated,
            &plugin_sources,
            &cpp_sources,
            &xcframeworks,
            &plugins,
            &config,
        )
        .expect("scaled iOS project should render deterministically");
        assert_eq!(project, again, "project generation must be deterministic");

        let ids = pbx_object_ids(&project);
        assert!(
            ids.len() >= 100,
            "expected a large object graph at this scale, found {} objects",
            ids.len()
        );
        for id in &ids {
            assert!(is_pbx_id(id), "PBX object key should be 24-hex: {id}");
        }
        let unique: std::collections::HashSet<&str> = ids.iter().map(String::as_str).collect();
        assert_eq!(
            ids.len(),
            unique.len(),
            "every PBX object key must be unique at plugin scale"
        );

        // Spot-check that every input file is referenced exactly once.
        for name in generated.iter().skip(1) {
            assert_eq!(
                project.matches(&format!("path = {name};")).count(),
                1,
                "generated unit {name} should have exactly one file reference"
            );
        }
        for name in &plugin_sources {
            assert_eq!(
                project
                    .matches(&format!("path = NexaPlugins/{name};"))
                    .count(),
                1,
                "plugin source {name} should have exactly one file reference"
            );
        }
        for product in ["MediaKit", "MediaUI", "MapsKit"] {
            assert!(
                project.contains(&format!("productName = \"{product}\"")),
                "SwiftPM product {product} should be present"
            );
        }
        assert!(project.contains("Embed Frameworks"));
        assert!(project.contains("path = Assets.xcassets"));
        assert!(project.contains("path = NexaPluginResources"));
        assert!(project.contains("AppIcon.icon"));
        assert!(project.contains("LaunchScreen.storyboard"));

        // The scheme must point at the target emitted into the project.
        let scheme = ios_scheme("Demo");
        let blueprint = scheme
            .split("BlueprintIdentifier=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("scheme should reference the native target");
        assert!(is_pbx_id(blueprint));
        assert!(
            project.contains(&format!("{blueprint} = {{ isa = PBXNativeTarget;")),
            "scheme target should match the generated PBXNativeTarget"
        );

        // On macOS CI, prove the generated project parses with xcodebuild.
        // SwiftPM remotes cannot be cloned in CI, so the xcodebuild leg uses
        // a package-free variant at the same file-count scale; package object
        // syntax is covered by the assertions above.
        #[cfg(target_os = "macos")]
        {
            let no_package_plugins = [plugin("Media"), plugin("Maps")];
            let list_project = ios_project_file_with_config(
                "Demo",
                true,
                true,
                false,
                &generated,
                &plugin_sources,
                &cpp_sources,
                &xcframeworks,
                &no_package_plugins,
                &config,
            )
            .expect("package-free scaled project should render");
            let list_ids = pbx_object_ids(&list_project);
            let list_unique: std::collections::HashSet<&str> =
                list_ids.iter().map(String::as_str).collect();
            assert_eq!(
                list_ids.len(),
                list_unique.len(),
                "every PBX object key must be unique in the xcodebuild variant"
            );
            let root = nexa_testkit::TempDir::new("nexa-pbx-scale");
            let proj_dir = root.join("Demo.xcodeproj");
            std::fs::create_dir_all(proj_dir.join("xcshareddata/xcschemes"))
                .expect("create xcodeproj layout");
            std::fs::write(proj_dir.join("project.pbxproj"), &list_project)
                .expect("write project.pbxproj");
            std::fs::write(
                proj_dir.join("xcshareddata/xcschemes/Demo.xcscheme"),
                &scheme,
            )
            .expect("write scheme");
            let output = std::process::Command::new("xcodebuild")
                .arg("-list")
                .arg("-project")
                .arg(proj_dir.as_os_str())
                .output();
            match output {
                Ok(output) if output.status.success() => {}
                Ok(output) => {
                    panic!(
                        "xcodebuild -list rejected the project: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    eprintln!("skipping xcodebuild check: xcodebuild not installed");
                }
                Err(error) => panic!("failed to run xcodebuild -list: {error}"),
            }
        }
    }

    #[test]
    fn ios_debug_and_release_share_configured_identity_and_minimum_version() {
        let mut config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        config.ios_bundle_identifier = "com.example.demo.staging".to_owned();
        config.ios_min_version = "15.1".to_owned();

        let project = ios_project_file_with_config(
            "Demo",
            false,
            false,
            false,
            &["NexaGenerated.swift".to_owned()],
            &[],
            &[],
            &[],
            &[],
            &config,
        )
        .expect("iOS project should include the configured host settings");

        assert_eq!(
            project
                .matches("PRODUCT_BUNDLE_IDENTIFIER = com.example.demo.staging;")
                .count(),
            2,
            "Debug and Release must use the same configured bundle identifier"
        );
        assert!(project.contains("IPHONEOS_DEPLOYMENT_TARGET = 15.1;"));
        assert!(!project.contains("IPHONEOS_DEPLOYMENT_TARGET = 17.0;"));
    }

    #[test]
    fn ios_hosts_always_declare_a_launch_screen() {
        let config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        let plist =
            ios_info_plist_with_orientation("Demo", &config, &[], false, false, &[]).unwrap();
        assert!(plist.contains("<key>UILaunchScreen</key><dict/>"));
        assert!(!plist.contains("<key>UISupportedInterfaceOrientations</key>"));

        let mut custom_splash = config;
        custom_splash.splash_source = Some(std::path::PathBuf::from("assets/splash.png"));
        let plist = ios_info_plist_with_orientation("Demo", &custom_splash, &[], false, false, &[])
            .unwrap();
        assert!(plist.contains("<key>UILaunchStoryboardName</key><string>LaunchScreen</string>"));
        assert!(!plist.contains("<key>UILaunchScreen</key>"));
    }

    #[test]
    fn ios_hosts_declare_landscape_for_scene_orientation_requests() {
        let config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        let plist =
            ios_info_plist_with_orientation("Demo", &config, &[], false, true, &[]).unwrap();
        assert!(plist.contains("<key>UISupportedInterfaceOrientations</key>"));
        assert!(plist.contains("<string>UIInterfaceOrientationLandscapeLeft</string>"));
        assert!(plist.contains("<string>UIInterfaceOrientationLandscapeRight</string>"));
        assert!(plist.contains("<key>UISupportedInterfaceOrientations~ipad</key>"));
        assert!(plist.contains("<string>UIInterfaceOrientationPortraitUpsideDown</string>"));
    }

    #[test]
    fn orientation_policy_can_lock_every_device_or_only_phone_layouts() {
        let mut config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        config.orientation = "portrait".to_owned();
        let portrait =
            ios_info_plist_with_orientation("Demo", &config, &[], false, false, &[]).unwrap();
        assert!(portrait.contains("UIInterfaceOrientationPortrait</string></array>"));
        assert!(!portrait.contains("UIInterfaceOrientationLandscapeLeft"));
        let manifest =
            android_manifest("Demo", "com.example.demo", false, false, &config, &[]).unwrap();
        assert!(manifest.contains("android:screenOrientation=\"portrait\""));

        config.orientation = "portrait-phones".to_owned();
        let phone_only =
            ios_info_plist_with_orientation("Demo", &config, &[], false, false, &[]).unwrap();
        assert!(phone_only.contains("UISupportedInterfaceOrientations~ipad"));
        assert!(phone_only.contains("UIInterfaceOrientationLandscapeLeft"));
        let manifest =
            android_manifest("Demo", "com.example.demo", false, false, &config, &[]).unwrap();
        assert!(manifest.contains("android:screenOrientation=\"fullUser\""));
        assert!(
            manifest.contains("dev.nexa.orientationPolicy\" android:value=\"portrait-phones\"")
        );
        assert!(manifest.contains(
            "android:configChanges=\"orientation|screenSize|smallestScreenSize|screenLayout\""
        ));
    }

    #[test]
    fn android_host_uses_api_23_by_default_in_dev_and_release() {
        let config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        for dev_runtime in [false, true] {
            let gradle = android_app_gradle_with_dev_runtime(
                "dev.nexa.demo",
                nexa_backend_kotlin::KotlinProjectFeatures::default(),
                &[],
                &[],
                &config,
                dev_runtime,
            )
            .expect("Android app Gradle file should render");

            assert!(gradle.contains("minSdk = 23"));
            assert!(gradle.contains("targetSdk = 36"));
        }

        let mut configured = config;
        configured.android_min_sdk = 26;
        let gradle = android_app_gradle_with_dev_runtime(
            "dev.nexa.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[],
            &[],
            &configured,
            false,
        )
        .expect("configured Android app minSdk should render");
        assert!(gradle.contains("minSdk = 26"));
    }

    #[test]
    fn android_rejects_plugin_minimum_above_app_minimum() {
        let mut plugin = plugin("NewPlatformApi");
        plugin.artifacts.android_min_sdk = Some(24);
        let config = ProjectConfig::from_defaults(&[], "Demo").unwrap();

        let error = android_app_gradle_with_dev_runtime(
            "dev.nexa.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[plugin],
            &[],
            &config,
            false,
        )
        .expect_err("a plugin cannot raise the app minimum implicitly");

        assert!(
            error.contains("plugin `NewPlatformApi` requires minSdk 24"),
            "{error}"
        );
        assert!(error.contains("android.minSdk to 23"), "{error}");
        assert!(error.contains("Increase `android.minSdk`"), "{error}");
    }

    #[test]
    fn android_keeps_the_app_minimum_when_plugin_requires_less() {
        let mut plugin = plugin("OlderPlatformApi");
        plugin.artifacts.android_min_sdk = Some(21);
        let mut config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        config.android_min_sdk = 26;

        let gradle = android_app_gradle_with_dev_runtime(
            "dev.nexa.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[plugin],
            &[],
            &config,
            false,
        )
        .expect("a lower plugin minimum should not change the app minimum");

        assert!(gradle.contains("minSdk = 26"));
    }

    #[test]
    fn android_rejects_plugin_minimum_above_supported_target_sdk() {
        let mut plugin = plugin("FuturePlatformApi");
        plugin.artifacts.android_min_sdk = Some(37);
        let config = ProjectConfig::from_defaults(&[], "Demo").unwrap();

        let error = android_app_gradle_with_dev_runtime(
            "dev.nexa.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[plugin],
            &[],
            &config,
            false,
        )
        .expect_err("a plugin requiring a newer target API cannot be built");

        assert!(error.contains("requires minSdk 37"), "{error}");
        assert!(error.contains("above the app targetSdk 36"), "{error}");
    }

    #[test]
    fn emits_native_dependency_metadata_into_both_projects() {
        let mut media_plugin = plugin("Media");
        media_plugin.artifacts.ios_min_version = Some("18.2".to_owned());
        media_plugin.artifacts.android_min_sdk = Some(29);
        media_plugin.artifacts.ios_frameworks = vec!["AVFoundation".to_owned()];
        media_plugin.artifacts.ios_xcframeworks =
            vec!["/plugins/video/ios/VideoSDK.xcframework".to_owned()];
        media_plugin.artifacts.ios_linker_flags = vec![
            "-ObjC".to_owned(),
            "-force_load".to_owned(),
            "$(PROJECT_DIR)/Vendor SDK/lib.a".to_owned(),
        ];
        media_plugin
            .artifacts
            .swift_packages
            .push(nexa_plugin_idl::manifest::SwiftPackage {
                url: "https://example.com/media.git".to_owned(),
                requirement: nexa_plugin_idl::manifest::SwiftPackageRequirement::From(
                    "2.3.0".to_owned(),
                ),
                products: vec!["MediaKit".to_owned()],
                extension_products: Vec::new(),
            });
        media_plugin
            .artifacts
            .maven_dependencies
            .push("com.example:media:2.3.0".to_owned());
        media_plugin.artifacts.android_aars =
            vec!["/plugins/video/android/libs/media.aar".to_owned()];
        media_plugin.artifacts.android_maven_repositories =
            vec!["https://maven.example.com/releases".to_owned()];
        let mut second_plugin = plugin("Recorder");
        second_plugin.artifacts.ios_frameworks = vec!["AVFoundation".to_owned()];
        second_plugin.artifacts.ios_linker_flags = vec!["-lz".to_owned()];
        second_plugin.artifacts.android_maven_repositories =
            vec!["https://maven.example.com/releases".to_owned()];
        let plugins = [media_plugin, second_plugin];

        let ios = ios_project_file(
            "Demo",
            false,
            false,
            &["NexaGenerated.swift".to_owned()],
            &[],
            &[],
            &["Frameworks/NexaPlugin0_VideoSDK.xcframework".to_owned()],
            &plugins,
        )
        .expect("SwiftPM metadata should render");
        assert!(ios.contains("XCRemoteSwiftPackageReference"));
        assert!(ios.contains("repositoryURL = \"https://example.com/media.git\""));
        assert!(ios.contains("productName = \"MediaKit\""));
        assert!(ios.contains("System/Library/Frameworks/AVFoundation.framework"));
        assert!(ios.contains("PBXFrameworksBuildPhase; files = ("));
        assert!(ios.contains("lastKnownFileType = wrapper.framework"));
        assert!(ios.contains("lastKnownFileType = wrapper.xcframework"));
        assert!(ios.contains("Embed Frameworks"));
        assert!(ios.contains("CodeSignOnCopy, RemoveHeadersOnCopy"));
        assert!(ios.contains("IPHONEOS_DEPLOYMENT_TARGET = 18.2"));
        assert!(ios.contains(
            "OTHER_LDFLAGS = ( \"$(inherited)\", \"-ObjC\", \"-force_load\", \"$(PROJECT_DIR)/Vendor SDK/lib.a\", \"-lz\" );"
        ));

        let resource_project = ios_project_file("Demo", false, true, &[], &[], &[], &[], &plugins)
            .expect("plugin resource bundle should be included in the Xcode project");
        assert!(resource_project.contains("path = NexaPluginResources"));
        let resources_build_id = pbx_build_file_for_path(&resource_project, "NexaPluginResources");
        assert!(
            resource_project.contains(&format!(
                "PBXResourcesBuildPhase; files = ( {resources_build_id} );"
            )),
            "plugin resource build file should be the only resources entry"
        );

        // Object identifiers are hash-derived from stable logical keys, so the
        // previously colliding numeric ranges (plugin files at 30+index vs
        // generated files at 40+index) can no longer overlap.
        let ids = pbx_object_ids(&ios);
        assert!(!ids.is_empty());
        let unique: std::collections::HashSet<&str> = ids.iter().map(String::as_str).collect();
        assert_eq!(
            ids.len(),
            unique.len(),
            "every PBX object key must be unique"
        );

        let settings = android_settings("Demo", &plugins);
        assert!(settings.contains("maven { url = uri(\"https://maven.example.com/releases\") }"));
        assert_eq!(
            settings
                .matches("https://maven.example.com/releases")
                .count(),
            1
        );

        let mut android_config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        android_config.android_min_sdk = 29;
        let android = android_app_gradle_with_dev_runtime(
            "com.example.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &plugins,
            &["NexaPlugin0_media.aar".to_owned()],
            &android_config,
            false,
        )
        .expect("Maven metadata should render");
        assert!(android.contains("minSdk = 29"));
        assert!(android.contains("implementation(\"com.example:media:2.3.0\")"));
        assert!(android.contains("implementation(files(\"libs/NexaPlugin0_media.aar\"))"));
        assert!(android.contains("dependencyLocking {\n    lockAllConfigurations()\n}"));
    }

    #[test]
    fn generated_hosts_compile_declared_cpp_implementation_sources() {
        let cpp_sources = vec!["Plugin0/cpp/Sources/Decoder.cpp".to_owned()];
        let mut cpp_plugin = plugin("Video");
        cpp_plugin.artifacts.cpp_sources = vec!["/plugins/video/cpp/Sources/**".to_owned()];
        cpp_plugin.artifacts.cpp_standard = Some(23);
        let mut lower_standard_plugin = plugin("Audio");
        lower_standard_plugin.artifacts.cpp_sources =
            vec!["/plugins/audio/cpp/Sources/**".to_owned()];
        lower_standard_plugin.artifacts.cpp_standard = Some(17);
        let ios = ios_project_file(
            "Demo",
            false,
            false,
            &[],
            &[],
            &cpp_sources,
            &[],
            &[cpp_plugin.clone(), lower_standard_plugin],
        )
        .expect("iOS project should render C++ implementation sources");
        assert!(ios.contains("lastKnownFileType = sourcecode.cpp.cpp"));
        assert!(ios.contains("NexaPluginCpp/Plugin0/cpp/Sources/Decoder.cpp"));
        assert!(ios.contains("CLANG_CXX_LANGUAGE_STANDARD = \"c++23\""));
        assert!(ios.contains("HEADER_SEARCH_PATHS"));

        let mut default_standard_plugin = plugin("DefaultStandard");
        default_standard_plugin.artifacts.cpp_sources =
            vec!["/plugins/default/cpp/Sources/**".to_owned()];
        let default_ios = ios_project_file(
            "Demo",
            false,
            false,
            &[],
            &[],
            &cpp_sources,
            &[],
            &[default_standard_plugin],
        )
        .expect("iOS C++ project should retain its default language level");
        assert!(default_ios.contains("CLANG_CXX_LANGUAGE_STANDARD = \"c++20\""));

        let android = android_app_gradle(
            "com.example.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[cpp_plugin],
            &[],
        )
        .expect("Android Gradle file should opt into CMake for declared C++");
        assert!(android.contains("externalNativeBuild { cmake"));
        assert!(android.contains("src/main/cpp/CMakeLists.txt"));
    }

    #[test]
    fn rejects_conflicting_versions_across_plugins() {
        let mut first = plugin("First");
        first.artifacts.maven_dependencies = vec!["com.example:media:1.0.0".to_owned()];
        let mut second = plugin("Second");
        second.artifacts.maven_dependencies = vec!["com.example:media:2.0.0".to_owned()];
        let error = android_app_gradle(
            "com.example.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[first, second],
            &[],
        )
        .expect_err("conflicting Maven versions must fail project generation");
        assert!(error.contains("conflicting versions"));
    }

    #[test]
    fn plugin_platform_permissions_reach_generated_manifests() {
        let mut plugin = plugin("Video");
        plugin.artifacts.ios_usage_descriptions.push((
            "NSCameraUsageDescription".to_owned(),
            "Record video clips.".to_owned(),
        ));
        plugin.artifacts.android_permissions = vec![
            "android.permission.CAMERA".to_owned(),
            "android.permission.RECORD_AUDIO".to_owned(),
        ];
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("empty project config");

        let plist = ios_info_plist("Demo", &config, &[plugin.clone()])
            .expect("purpose string should render");
        assert!(
            plist.contains(
                "<key>NSCameraUsageDescription</key><string>Record video clips.</string>"
            )
        );

        let manifest =
            android_manifest("Demo", "com.example.demo", false, false, &config, &[plugin])
                .expect("Android manifest should generate");
        assert!(manifest.contains("android.permission.CAMERA"));
        assert!(manifest.contains("android.permission.RECORD_AUDIO"));
        assert!(manifest.contains("android:theme=\"@style/NexaAppTheme\""));
    }

    #[test]
    fn android_plugin_application_metadata_reaches_manifest_and_gradle() {
        let mut maps = plugin("Maps");
        maps.artifacts.android_application_metadata = vec![
            (
                "com.google.android.geo.API_KEY".to_owned(),
                "${NEXA_MAPS_API_KEY}".to_owned(),
            ),
            (
                "com.example.maps.description".to_owned(),
                "Maps & Places".to_owned(),
            ),
        ];
        maps.artifacts.android_permissions = vec!["android.permission.INTERNET".to_owned()];
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("default project config");

        let manifest = templates::android_manifest(
            "Demo",
            "com.example.demo",
            false,
            false,
            &config,
            &[maps.clone()],
        )
        .expect("Android application metadata should generate");
        assert!(manifest.contains("android.permission.INTERNET"));
        assert!(manifest.contains(
            "<meta-data android:name=\"com.google.android.geo.API_KEY\" android:value=\"${NEXA_MAPS_API_KEY}\" />"
        ));
        assert!(manifest.contains("android:value=\"Maps &amp; Places\""));

        let gradle = templates::android_app_gradle_with_dev_runtime(
            "Demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[maps],
            &[],
            &config,
            false,
        )
        .expect("environment-backed manifest placeholders should generate");
        assert!(gradle.contains(
            "manifestPlaceholders[\"NEXA_MAPS_API_KEY\"] = providers.environmentVariable(\"NEXA_MAPS_API_KEY\").orElse(\"\").get()"
        ));
    }

    #[test]
    fn native_notification_services_reach_both_generated_hosts() {
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("default project config");
        let mut notifications = plugin("Notifications");
        notifications.artifacts.ios_application_delegate =
            Some("NotificationsAppDelegate".to_owned());
        notifications.artifacts.android_firebase_messaging_service =
            Some("dev.nexa.notifications.NotificationsFirebaseMessagingService".to_owned());
        notifications.artifacts.ios_entitlements.push((
            "aps-environment".to_owned(),
            nexa_plugin_idl::manifest::EntitlementValue::String("development".to_owned()),
        ));

        let app_source =
            templates::ios_app_source("Demo", "DemoRoot()", &[], &[notifications.clone()], false)
                .expect("iOS app delegate should integrate");
        assert!(app_source.contains(
            "@UIApplicationDelegateAdaptor(NotificationsAppDelegate.self) private var nexaApplicationDelegate"
        ));

        let manifest = templates::android_manifest(
            "Demo",
            "com.example.demo",
            false,
            false,
            &config,
            &[notifications.clone()],
        )
        .expect("Android messaging service should integrate");
        assert!(manifest.contains(
            "<service android:name=\"dev.nexa.notifications.NotificationsFirebaseMessagingService\" android:exported=\"false\"><intent-filter><action android:name=\"com.google.firebase.MESSAGING_EVENT\" /></intent-filter></service>"
        ));

        let debug = templates::ios_entitlements(&config, &[notifications.clone()])
            .expect("debug entitlements should generate")
            .expect("APNs requires entitlements");
        let release = templates::ios_release_entitlements(&config, &[notifications.clone()])
            .expect("release entitlements should generate")
            .expect("APNs requires entitlements");
        assert!(debug.contains("<key>aps-environment</key><string>development</string>"));
        assert!(release.contains("<key>aps-environment</key><string>production</string>"));

        let xcode_project = templates::ios_project_file_with_config(
            "Demo",
            false,
            false,
            false,
            &[],
            &[],
            &[],
            &[],
            &[notifications],
            &config,
        )
        .expect("APNs should choose configuration-specific entitlements");
        assert!(xcode_project.contains("CODE_SIGN_ENTITLEMENTS = Demo/Nexa-Release.entitlements"));
        assert!(xcode_project.contains("CODE_SIGN_ENTITLEMENTS = Demo/Nexa.entitlements"));
    }

    #[test]
    fn android_firebase_app_options_become_default_sdk_resources() {
        let idl = nexa_plugin_idl::parse(
            r#"config {
                fcmApiKey: String = ""
                fcmApplicationId: String = ""
                fcmProjectId: String = ""
                fcmSenderId: String = ""
            }"#,
        )
        .expect("notification config IDL should parse");
        let definitions = [crate::config::PluginDefinition {
            namespace: "Notifications".to_owned(),
            idl,
        }];
        let directory = nexa_testkit::TempDir::new("nexa-firebase-config");
        let path = directory.path().join("nexa.config.nx");
        fs::write(
            &path,
            r#"config {
                plugins {
                    Notifications {
                        fcmApiKey: "api<&key",
                        fcmApplicationId: "1:123:android:app",
                        fcmProjectId: "project-id",
                        fcmSenderId: "123",
                    }
                }
            }"#,
        )
        .expect("write Firebase app config");
        let config = ProjectConfig::parse_file(&path, &definitions, "Demo")
            .expect("Firebase app config should resolve");
        let mut plugin = plugin("Notifications");
        plugin.artifacts.android_firebase_messaging_service =
            Some("dev.nexa.notifications.NotificationsFirebaseMessagingService".to_owned());

        let resources = templates::android_firebase_resources(&[plugin], &config)
            .expect("Firebase resources should render")
            .expect("configured Firebase options should create resources");
        assert!(resources.contains("name=\"google_app_id\""));
        assert!(resources.contains("1:123:android:app"));
        assert!(resources.contains("api&lt;&amp;key"));
        assert!(resources.contains("name=\"gcm_defaultSenderId\""));
    }

    #[test]
    fn conflicting_android_plugin_application_metadata_is_rejected() {
        let mut first = plugin("First");
        first.artifacts.android_application_metadata.push((
            "com.example.maps.API_KEY".to_owned(),
            "${FIRST_MAPS_KEY}".to_owned(),
        ));
        let mut second = plugin("Second");
        second.artifacts.android_application_metadata.push((
            "com.example.maps.API_KEY".to_owned(),
            "${SECOND_MAPS_KEY}".to_owned(),
        ));
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("default project config");

        let error = templates::android_manifest(
            "Demo",
            "com.example.demo",
            false,
            false,
            &config,
            &[first, second],
        )
        .expect_err("conflicting metadata values must be rejected");
        assert!(error.contains("conflicting values from plugins"));
    }

    #[test]
    fn plugin_picture_in_picture_metadata_reaches_only_opted_in_hosts() {
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("empty project config");
        let without_video =
            android_manifest("Demo", "com.example.demo", false, false, &config, &[])
                .expect("Android manifest should generate");
        assert!(!without_video.contains("android:supportsPictureInPicture"));

        let mut video = plugin("Video");
        video.artifacts.android_picture_in_picture = true;
        let with_video =
            android_manifest("Demo", "com.example.demo", false, false, &config, &[video])
                .expect("Android manifest should generate");
        assert!(with_video.contains("android:supportsPictureInPicture=\"true\""));
        assert!(with_video.contains(
            "android:configChanges=\"screenSize|smallestScreenSize|screenLayout|orientation\""
        ));
    }

    #[test]
    fn android_launcher_uses_the_user_orientation_policy_at_startup() {
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("empty project config");
        let manifest = android_manifest("Demo", "com.example.demo", false, false, &config, &[])
            .expect("Android manifest should generate");

        assert!(manifest.contains("android:screenOrientation=\"fullUser\""));
        assert!(!manifest.contains("android:screenOrientation=\"portrait\""));
    }

    #[test]
    fn connectivity_status_adds_state_permission_without_internet_transport() {
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("empty project config");
        let manifest = android_manifest("Demo", "com.example.demo", false, true, &config, &[])
            .expect("Android manifest should generate");

        assert!(manifest.contains("android.permission.ACCESS_NETWORK_STATE"));
        assert!(!manifest.contains("android.permission.INTERNET"));
    }

    #[test]
    fn audio_plugin_background_metadata_reaches_both_native_hosts() {
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("empty project config");
        let mut audio = plugin("Audio");
        audio.artifacts.ios_background_modes = vec!["audio".to_owned()];
        audio.artifacts.android_media_playback_service =
            Some("dev.nexa.audio.AudioPlaybackService".to_owned());

        let plist =
            ios_info_plist_with_orientation("Demo", &config, &[audio.clone()], false, false, &[])
                .expect("iOS audio metadata should generate");
        assert!(
            plist.contains("<key>UIBackgroundModes</key><array><string>audio</string></array>")
        );

        let manifest =
            android_manifest("Demo", "com.example.demo", false, false, &config, &[audio])
                .expect("Android manifest should generate");
        assert!(manifest.contains("android.permission.FOREGROUND_SERVICE"));
        assert!(manifest.contains("android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK"));
        assert!(manifest.contains(
            "<service android:name=\"dev.nexa.audio.AudioPlaybackService\" android:exported=\"true\" android:foregroundServiceType=\"mediaPlayback\"><intent-filter><action android:name=\"androidx.media3.session.MediaSessionService\" /></intent-filter></service>"
        ));
    }

    #[test]
    fn every_configured_permission_maps_to_ios_and_android_manifests() {
        let home = nexa_testkit::TempDir::new("nexa-all-permissions");
        let path = home.path().join("config.nx");
        fs::write(
            &path,
            r#"config { permissions {
                camera: "Camera purpose",
                microphone: "Microphone purpose",
                photos: "Photos purpose",
                location: "Location purpose",
                notifications: "Notifications purpose",
                contacts: "Contacts purpose",
                calendar: "Calendar purpose",
                bluetooth: "Bluetooth purpose",
                motion: "Motion purpose"
            } }"#,
        )
        .expect("write permission config");
        let config = ProjectConfig::parse_file(&path, &[], "Permissions")
            .expect("parse all configured permissions");
        fs::remove_file(path).expect("remove permission config");

        let plist =
            ios_info_plist("Permissions", &config, &[]).expect("generate iOS permission metadata");
        let manifest = templates::android_manifest(
            "Permissions",
            "com.example.permissions",
            false,
            false,
            &config,
            &[],
        )
        .expect("Android manifest should generate");
        let platform_permissions = [
            (
                "camera",
                Some("NSCameraUsageDescription"),
                &["android.permission.CAMERA"][..],
            ),
            (
                "microphone",
                Some("NSMicrophoneUsageDescription"),
                &["android.permission.RECORD_AUDIO"][..],
            ),
            (
                "photos",
                Some("NSPhotoLibraryUsageDescription"),
                &[
                    "android.permission.READ_MEDIA_IMAGES",
                    "android.permission.READ_EXTERNAL_STORAGE",
                ][..],
            ),
            (
                "location",
                Some("NSLocationWhenInUseUsageDescription"),
                &[
                    "android.permission.ACCESS_COARSE_LOCATION",
                    "android.permission.ACCESS_FINE_LOCATION",
                ][..],
            ),
            (
                "notifications",
                None,
                &["android.permission.POST_NOTIFICATIONS"][..],
            ),
            (
                "contacts",
                Some("NSContactsUsageDescription"),
                &[
                    "android.permission.READ_CONTACTS",
                    "android.permission.WRITE_CONTACTS",
                ][..],
            ),
            (
                "calendar",
                Some("NSCalendarsFullAccessUsageDescription"),
                &[
                    "android.permission.READ_CALENDAR",
                    "android.permission.WRITE_CALENDAR",
                ][..],
            ),
            (
                "bluetooth",
                Some("NSBluetoothAlwaysUsageDescription"),
                &[
                    "android.permission.BLUETOOTH_SCAN",
                    "android.permission.BLUETOOTH_CONNECT",
                ][..],
            ),
            (
                "motion",
                Some("NSMotionUsageDescription"),
                &["android.permission.ACTIVITY_RECOGNITION"][..],
            ),
        ];
        for (name, ios_key, android_permissions) in platform_permissions {
            if let Some(key) = ios_key {
                assert!(
                    plist.contains(&format!("<key>{key}</key>")),
                    "missing iOS {name}"
                );
            }
            for permission in android_permissions {
                assert!(
                    manifest.contains(permission),
                    "missing Android {name} permission {permission}"
                );
            }
        }
        assert!(!plist.contains("NSNotificationsUsageDescription"));
    }

    #[test]
    fn project_config_parses_and_validates_architectures() {
        let home = nexa_testkit::TempDir::new("nexa-project-architecture");
        let path = home.path().join("nexa.config.nx");
        fs::write(
            &path,
            r#"config {
                ios { arch: ["arm64", "x86_64"] }
                android { arch: ["arm64", "x86_64"] }
            }"#,
        )
        .expect("write architecture config");
        let config = ProjectConfig::parse_file(&path, &[], "Architecture")
            .expect("parse architecture settings");
        assert_eq!(
            config.ios_arch.as_deref(),
            Some(["arm64".to_owned(), "x86_64".to_owned()].as_slice())
        );
        assert_eq!(
            config.android_arch.as_deref(),
            Some(["arm64".to_owned(), "x86_64".to_owned()].as_slice())
        );

        fs::write(&path, r#"config { ios { arch: "x86" } }"#)
            .expect("write invalid architecture config");
        let error = ProjectConfig::parse_file(&path, &[], "Architecture")
            .expect_err("unsupported iOS architecture should fail");
        assert!(error.contains("unsupported iOS architecture"));
    }

    fn create_android_icon_set(root: &std::path::Path, name: &str) {
        let directory = root.join(name);
        fs::create_dir_all(&directory).expect("create Android icon set");
        fs::write(directory.join("icon.png"), b"icon").expect("write legacy icon placeholder");
        fs::write(directory.join("foreground.xml"), "<vector />")
            .expect("write foreground placeholder");
        fs::write(directory.join("background.xml"), "<shape />")
            .expect("write background placeholder");
    }

    #[test]
    fn android_alternate_icons_require_a_default_icon_and_icon_sets() {
        let home = nexa_testkit::TempDir::new("nexa-android-alternate-icons");
        let assets = home.path().join("icons");
        fs::create_dir_all(&assets).expect("create icon assets directory");
        create_android_icon_set(&assets, "Blue");
        let path = home.path().join("nexa.config.nx");

        fs::write(
            &path,
            r#"config { android { alternateIcons: ["icons/Blue"] } }"#,
        )
        .expect("write missing-default config");
        let error = ProjectConfig::parse_file(&path, &[], "Icons")
            .expect_err("alternate icons require a default launcher icon");
        assert!(error.contains("require a default Android `icon` or shared `assets.icon`"));

        create_android_icon_set(&assets, "Default");
        fs::write(
            &path,
            r#"config { android { icon: "icons/Default", alternateIcons: ["icons/Blue"] } }"#,
        )
        .expect("write valid icon-set config");
        ProjectConfig::parse_file(&path, &[], "Icons")
            .expect("default and alternate icon sets should parse");

        fs::write(assets.join("Flat.png"), b"icon").expect("write flat icon placeholder");
        fs::write(
            &path,
            r#"config { android { icon: "icons/Default", alternateIcons: ["icons/Flat.png"] } }"#,
        )
        .expect("write flat alternate config");
        let error = ProjectConfig::parse_file(&path, &[], "Icons")
            .expect_err("Android alternate icons must be adaptive icon sets");
        assert!(error.contains("invalid Android alternate icon asset"));
    }

    #[test]
    fn android_alternate_icon_resource_names_reject_collisions_and_reserved_names() {
        let home = nexa_testkit::TempDir::new("nexa-android-icon-resource-names");
        let assets = home.path().join("icons");
        fs::create_dir_all(&assets).expect("create icon assets directory");
        create_android_icon_set(&assets, "Default");
        create_android_icon_set(&assets, "Blue");
        create_android_icon_set(&assets, "blue");
        create_android_icon_set(&assets, "ic_launcher");
        let path = home.path().join("nexa.config.nx");

        fs::write(
            &path,
            r#"config { android { icon: "icons/Default", alternateIcons: ["icons/Blue", "icons/blue"] } }"#,
        )
        .expect("write colliding icon config");
        let error = ProjectConfig::parse_file(&path, &[], "Icons")
            .expect_err("case-insensitive Android resource names must be unique");
        assert!(error.contains("resource names are case-insensitive"));

        fs::write(
            &path,
            r#"config { android { icon: "icons/Default", alternateIcons: ["icons/ic_launcher"] } }"#,
        )
        .expect("write reserved icon config");
        let error = ProjectConfig::parse_file(&path, &[], "Icons")
            .expect_err("default launcher resource names are reserved");
        assert!(error.contains("reserved for the default launcher icon"));
    }

    #[test]
    fn plugin_entitlements_merge_into_plist_and_code_signing_settings() {
        let mut plugin = plugin("SecureStorage");
        plugin.artifacts.ios_entitlements = vec![
            (
                "aps-environment".to_owned(),
                nexa_plugin_idl::manifest::EntitlementValue::String("development".to_owned()),
            ),
            (
                "com.apple.developer.associated-domains".to_owned(),
                nexa_plugin_idl::manifest::EntitlementValue::Strings(vec![
                    "applinks:example.com".to_owned(),
                    "webcredentials:example.com".to_owned(),
                ]),
            ),
            (
                "com.apple.developer.networking.wifi-info".to_owned(),
                nexa_plugin_idl::manifest::EntitlementValue::Bool(true),
            ),
        ];

        let config = ProjectConfig::from_defaults(&[], "Demo").expect("default app config");
        let entitlements = ios_entitlements(&config, &[plugin.clone()])
            .expect("valid entitlements should render")
            .expect("the app should receive an entitlements file");
        assert!(entitlements.contains("<key>aps-environment</key><string>development</string>"));
        assert!(entitlements.contains(
            "<key>com.apple.developer.associated-domains</key><array><string>applinks:example.com</string><string>webcredentials:example.com</string></array>"
        ));
        assert!(
            entitlements.contains("<key>com.apple.developer.networking.wifi-info</key><true/>")
        );

        let project = ios_project_file("Demo", false, false, &[], &[], &[], &[], &[plugin])
            .expect("entitlements should integrate with Xcode project settings");
        assert!(project.contains("CODE_SIGN_ENTITLEMENTS = Demo/Nexa.entitlements"));
    }

    #[test]
    fn deep_link_bases_generate_ios_schemes_associated_domains_and_android_intents() {
        let mut config =
            ProjectConfig::from_defaults(&[], "DeepLinkDemo").expect("default app config");
        config.deep_links = vec!["nexa://".to_owned(), "https://links.example.com".to_owned()];

        let plist =
            ios_info_plist_with_orientation("DeepLinkDemo", &config, &[], false, false, &[])
                .expect("render iOS URL scheme metadata");
        assert!(
            plist.contains("<key>CFBundleURLSchemes</key><array><string>nexa</string></array>")
        );

        let entitlements = ios_entitlements(&config, &[])
            .expect("render iOS associated-domain entitlement")
            .expect("universal links require an entitlement file");
        assert!(entitlements.contains(
            "<key>com.apple.developer.associated-domains</key><array><string>applinks:links.example.com</string></array>"
        ));
        let xcode_project = ios_project_file_with_config(
            "DeepLinkDemo",
            false,
            false,
            false,
            &[],
            &[],
            &[],
            &[],
            &[],
            &config,
        )
        .expect("enable iOS associated-domain entitlements");
        assert!(xcode_project.contains("CODE_SIGN_ENTITLEMENTS = DeepLinkDemo/Nexa.entitlements"));

        let manifest = android_manifest(
            "DeepLinkDemo",
            "dev.example.deep_link_demo",
            false,
            false,
            &config,
            &[],
        )
        .expect("Android manifest should generate");
        assert!(manifest.contains("android:scheme=\"nexa\""));
        assert!(manifest.contains("android:autoVerify=\"true\""));
        assert!(manifest.contains("android:host=\"links.example.com\""));
        assert_eq!(manifest.matches("android.intent.action.VIEW").count(), 2);
    }

    #[test]
    fn rejects_conflicting_plugin_entitlements() {
        let mut first = plugin("First");
        first.artifacts.ios_entitlements.push((
            "aps-environment".to_owned(),
            nexa_plugin_idl::manifest::EntitlementValue::String("development".to_owned()),
        ));
        let mut second = plugin("Second");
        second.artifacts.ios_entitlements.push((
            "aps-environment".to_owned(),
            nexa_plugin_idl::manifest::EntitlementValue::String("production".to_owned()),
        ));

        let config = ProjectConfig::from_defaults(&[], "Demo").expect("default app config");
        let error = ios_entitlements(&config, &[first, second])
            .expect_err("conflicting values for one entitlement must fail");
        assert!(error.contains("conflicting values"));
        assert!(error.contains("First"));
        assert!(error.contains("Second"));
    }

    #[test]
    fn rejects_conflicting_ios_plugin_purpose_strings() {
        let mut first = plugin("First");
        first.artifacts.ios_usage_descriptions.push((
            "NSCameraUsageDescription".to_owned(),
            "Record video.".to_owned(),
        ));
        let mut second = plugin("Second");
        second.artifacts.ios_usage_descriptions.push((
            "NSCameraUsageDescription".to_owned(),
            "Scan documents.".to_owned(),
        ));
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("empty project config");
        let error = ios_info_plist("Demo", &config, &[first, second])
            .expect_err("ambiguous usage-description text must fail generation");
        assert!(error.contains("different purpose message"));
    }
}
