#[allow(dead_code)]
#[path = "../src/config.rs"]
mod config;
use config::ProjectConfig;

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
            dependencies.contains(&format!("implementation(\"{dependency}")),
            "dev dependency {dependency} should be preloaded"
        );
    }
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
fn android_network_release_bundles_the_cronet_fallback() {
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
    assert!(dependencies.contains("org.chromium.net:cronet-embedded:143.7445.0"));
}

mod template_generation {
    use std::fs;

    use crate::ProjectConfig;
    use crate::templates;
    use crate::templates::*;

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
        let config = ProjectConfig::from_defaults(&[], app_name)?;
        ios_project_file_with_config(
            app_name,
            has_assets,
            has_plugin_resources,
            generated_sources,
            plugin_sources,
            cpp_sources,
            xcframeworks,
            plugins,
            &config,
        )
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
        ios_info_plist_with_dev_runtime(app_name, config, plugins, false)
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
                from: "2.3.0".to_owned(),
                products: vec!["MediaKit".to_owned(), "MediaUI".to_owned()],
            });
        let mut maps_plugin = plugin("Maps");
        maps_plugin.artifacts.ios_frameworks = vec!["MapKit".to_owned()];
        maps_plugin
            .artifacts
            .swift_packages
            .push(nexa_plugin_idl::manifest::SwiftPackage {
                url: "https://example.com/maps.git".to_owned(),
                from: "1.0.0".to_owned(),
                products: vec!["MapsKit".to_owned()],
            });
        let plugins = [media_plugin, maps_plugin];

        let project = ios_project_file_with_config(
            "Demo",
            true,
            true,
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
        let plist = ios_info_plist_with_dev_runtime("Demo", &config, &[], false).unwrap();
        assert!(plist.contains("<key>UILaunchScreen</key><dict/>"));

        let mut custom_splash = config;
        custom_splash.splash_source = Some(std::path::PathBuf::from("assets/splash.png"));
        let plist = ios_info_plist_with_dev_runtime("Demo", &custom_splash, &[], false).unwrap();
        assert!(plist.contains("<key>UILaunchStoryboardName</key><string>LaunchScreen</string>"));
        assert!(!plist.contains("<key>UILaunchScreen</key>"));
    }

    #[test]
    fn android_host_uses_configurable_minimum_and_fixed_target_sdk_36() {
        let config = ProjectConfig::from_defaults(&[], "Demo").unwrap();
        let gradle = android_app_gradle_with_dev_runtime(
            "dev.nexa.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[],
            &[],
            &config,
            false,
        )
        .expect("Android app Gradle file should render");

        assert!(gradle.contains("minSdk = 24"));
        assert!(gradle.contains("targetSdk = 36"));
    }

    #[test]
    fn android_rejects_plugin_minimum_above_target_sdk() {
        let mut plugin = plugin("NewPlatformApi");
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
        .expect_err("a plugin cannot require an SDK above the app target SDK");

        assert!(error.contains("minSdk to 37"), "{error}");
        assert!(error.contains("targetSdk (36)"), "{error}");
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
                from: "2.3.0".to_owned(),
                products: vec!["MediaKit".to_owned()],
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

        let android = android_app_gradle(
            "com.example.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &plugins,
            &["NexaPlugin0_media.aar".to_owned()],
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

        let manifest = android_manifest("Demo", "com.example.demo", false, &config, &[plugin]);
        assert!(manifest.contains("android.permission.CAMERA"));
        assert!(manifest.contains("android.permission.RECORD_AUDIO"));
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
                bluetooth: "Bluetooth purpose"
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
            &config,
            &[],
        );
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

        let plist = ios_info_plist_with_dev_runtime("DeepLinkDemo", &config, &[], false)
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
            &config,
            &[],
        );
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
