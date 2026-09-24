#[allow(dead_code)]
#[path = "../src/config.rs"]
mod config;
use config::ProjectConfig;

mod plugins {
    pub(super) fn cpp_header_include_roots(
        _plugin: &nexa_ir::Plugin,
        _plugin_index: usize,
    ) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    pub(super) fn minimum_cpp_standard(plugins: &[nexa_ir::Plugin]) -> u8 {
        plugins
            .iter()
            .filter_map(|plugin| plugin.cpp_standard)
            .max()
            .unwrap_or(20)
    }
}

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
    let gradle = templates::android_app_gradle_with_config(
        "demo",
        nexa_backend_kotlin::KotlinProjectFeatures::default(),
        &[],
        &[],
        &config,
    )
    .unwrap();
    assert!(gradle.contains("create(\"nexaRelease\")"));
    assert!(gradle.contains("System.getenv(\"NEXA_ANDROID_KEYSTORE\")"));
    assert!(gradle.contains("signingConfigs.getByName(\"nexaRelease\")"));
}

mod template_generation {
    use crate::ProjectConfig;
    use crate::templates::*;

    fn ios_project_file(
        app_name: &str,
        has_assets: bool,
        has_plugin_resources: bool,
        generated_sources: &[String],
        plugin_sources: &[String],
        cpp_sources: &[String],
        xcframeworks: &[String],
        plugins: &[nexa_ir::Plugin],
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
        plugins: &[nexa_ir::Plugin],
        local_aars: &[String],
    ) -> Result<String, String> {
        let config = ProjectConfig::from_defaults(&[], package)?;
        android_app_gradle_with_config(package, features, plugins, local_aars, &config)
    }

    fn ios_info_plist(
        app_name: &str,
        config: &ProjectConfig,
        plugins: &[nexa_ir::Plugin],
    ) -> Result<String, String> {
        ios_info_plist_with_dev_runtime(app_name, config, plugins, false)
    }

    fn plugin(namespace: &str) -> nexa_ir::Plugin {
        nexa_ir::Plugin {
            namespace: namespace.to_owned(),
            idl_path: String::new(),
            ios_sources: Vec::new(),
            android_sources: Vec::new(),
            cpp_sources: Vec::new(),
            cpp_headers: Vec::new(),
            cpp_standard: None,
            ios_min_version: None,
            android_min_sdk: None,
            ios_frameworks: Vec::new(),
            ios_xcframeworks: Vec::new(),
            ios_resources: Vec::new(),
            ios_privacy_manifest: None,
            swift_packages: Vec::new(),
            maven_dependencies: Vec::new(),
            android_aars: Vec::new(),
            android_resources: Vec::new(),
            android_proguard_rules: Vec::new(),
            android_maven_repositories: Vec::new(),
            ios_usage_descriptions: Vec::new(),
            ios_entitlements: Vec::new(),
            ios_linker_flags: Vec::new(),
            android_permissions: Vec::new(),
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
        let gradle = android_app_gradle_with_config(
            "dev.nexa.demo",
            nexa_backend_kotlin::KotlinProjectFeatures::default(),
            &[],
            &[],
            &config,
        )
        .expect("Android app Gradle file should render");

        assert!(gradle.contains("minSdk = 24"));
        assert!(gradle.contains("targetSdk = 36"));
    }

    #[test]
    fn emits_native_dependency_metadata_into_both_projects() {
        let mut media_plugin = plugin("Media");
        media_plugin.ios_min_version = Some("18.2".to_owned());
        media_plugin.android_min_sdk = Some(29);
        media_plugin.ios_frameworks = vec!["AVFoundation".to_owned()];
        media_plugin.ios_xcframeworks = vec!["/plugins/video/ios/VideoSDK.xcframework".to_owned()];
        media_plugin.ios_linker_flags = vec![
            "-ObjC".to_owned(),
            "-force_load".to_owned(),
            "$(PROJECT_DIR)/Vendor SDK/lib.a".to_owned(),
        ];
        media_plugin.swift_packages.push(nexa_ir::SwiftPackage {
            url: "https://example.com/media.git".to_owned(),
            from: "2.3.0".to_owned(),
            products: vec!["MediaKit".to_owned()],
        });
        media_plugin
            .maven_dependencies
            .push("com.example:media:2.3.0".to_owned());
        media_plugin.android_aars = vec!["/plugins/video/android/libs/media.aar".to_owned()];
        media_plugin.android_maven_repositories =
            vec!["https://maven.example.com/releases".to_owned()];
        let mut second_plugin = plugin("Recorder");
        second_plugin.ios_frameworks = vec!["AVFoundation".to_owned()];
        second_plugin.ios_linker_flags = vec!["-lz".to_owned()];
        second_plugin.android_maven_repositories =
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
        assert!(resource_project.contains(&format!(
            "PBXResourcesBuildPhase; files = ( {} );",
            pbx_identifier(10001)
        )));

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
        cpp_plugin.cpp_sources = vec!["/plugins/video/cpp/Sources/**".to_owned()];
        cpp_plugin.cpp_standard = Some(23);
        let mut lower_standard_plugin = plugin("Audio");
        lower_standard_plugin.cpp_sources = vec!["/plugins/audio/cpp/Sources/**".to_owned()];
        lower_standard_plugin.cpp_standard = Some(17);
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
        default_standard_plugin.cpp_sources = vec!["/plugins/default/cpp/Sources/**".to_owned()];
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
        first.maven_dependencies = vec!["com.example:media:1.0.0".to_owned()];
        let mut second = plugin("Second");
        second.maven_dependencies = vec!["com.example:media:2.0.0".to_owned()];
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
        plugin.ios_usage_descriptions.push((
            "NSCameraUsageDescription".to_owned(),
            "Record video clips.".to_owned(),
        ));
        plugin.android_permissions = vec![
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
    fn plugin_entitlements_merge_into_plist_and_code_signing_settings() {
        let mut plugin = plugin("SecureStorage");
        plugin.ios_entitlements = vec![
            (
                "aps-environment".to_owned(),
                nexa_ir::PluginEntitlementValue::String("development".to_owned()),
            ),
            (
                "com.apple.developer.associated-domains".to_owned(),
                nexa_ir::PluginEntitlementValue::Strings(vec![
                    "applinks:example.com".to_owned(),
                    "webcredentials:example.com".to_owned(),
                ]),
            ),
            (
                "com.apple.developer.networking.wifi-info".to_owned(),
                nexa_ir::PluginEntitlementValue::Bool(true),
            ),
        ];

        let entitlements = ios_entitlements(&[plugin.clone()])
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
    fn rejects_conflicting_plugin_entitlements() {
        let mut first = plugin("First");
        first.ios_entitlements.push((
            "aps-environment".to_owned(),
            nexa_ir::PluginEntitlementValue::String("development".to_owned()),
        ));
        let mut second = plugin("Second");
        second.ios_entitlements.push((
            "aps-environment".to_owned(),
            nexa_ir::PluginEntitlementValue::String("production".to_owned()),
        ));

        let error = ios_entitlements(&[first, second])
            .expect_err("conflicting values for one entitlement must fail");
        assert!(error.contains("conflicting values"));
        assert!(error.contains("First"));
        assert!(error.contains("Second"));
    }

    #[test]
    fn rejects_conflicting_ios_plugin_purpose_strings() {
        let mut first = plugin("First");
        first.ios_usage_descriptions.push((
            "NSCameraUsageDescription".to_owned(),
            "Record video.".to_owned(),
        ));
        let mut second = plugin("Second");
        second.ios_usage_descriptions.push((
            "NSCameraUsageDescription".to_owned(),
            "Scan documents.".to_owned(),
        ));
        let config = ProjectConfig::from_defaults(&[], "Demo").expect("empty project config");
        let error = ios_info_plist("Demo", &config, &[first, second])
            .expect_err("ambiguous usage-description text must fail generation");
        assert!(error.contains("different purpose message"));
    }
}
