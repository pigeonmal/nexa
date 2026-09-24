//! Deterministic native host templates used by `nexa generate`.

use std::path::Path;

use nexa_ir::Permission;

use super::ProjectConfig;

pub(super) fn root_readme(app_name: &str, targets: &[&str]) -> String {
    let mut readme = format!(
        "# {app_name}\n\nEdit your `.nx` source and `nexa.config.nx`, then run `nexa dev`. Generated native project files and artifacts live under this directory.\n\n"
    );
    if targets.contains(&"ios") {
        readme.push_str(&format!("## iOS\n\nRun `nexa dev --ios` for simulator development and `nexa release --ios` for an archive and exported IPA.\n\n"));
    }
    if targets.contains(&"android") {
        readme.push_str("## Android\n\nRun `nexa dev --android` for emulator development and `nexa release --android` for a signed AAB.\n\n");
    }
    readme.push_str("Run `nexa test` to compile generated native projects and `nexa doctor` to check the toolchain.\n");
    readme
}

pub(super) fn ios_info_plist_with_dev_runtime(
    app_name: &str,
    config: &ProjectConfig,
    plugins: &[nexa_ir::Plugin],
    dev_runtime: bool,
) -> Result<String, String> {
    let mut usage_descriptions = std::collections::BTreeMap::<String, String>::new();
    for (permission, description) in config.permissions() {
        let keys: &[&str] = match *permission {
            Permission::Camera => &["NSCameraUsageDescription"],
            Permission::Microphone => &["NSMicrophoneUsageDescription"],
            Permission::Photos => &["NSPhotoLibraryUsageDescription"],
            Permission::Location => &["NSLocationWhenInUseUsageDescription"],
            // iOS notification authorization has no Info.plist usage-description key.
            Permission::Notifications => &[],
            Permission::Contacts => &["NSContactsUsageDescription"],
            Permission::Calendar => &["NSCalendarsFullAccessUsageDescription"],
            Permission::Bluetooth => &["NSBluetoothAlwaysUsageDescription"],
        };
        for key in keys {
            usage_descriptions.insert((*key).to_owned(), description.clone());
        }
    }
    for plugin in plugins {
        for (key, message) in &plugin.ios_usage_descriptions {
            if let Some(existing) = usage_descriptions.get(key)
                && existing != message
            {
                return Err(format!(
                    "plugin `{}` declares `{key}` with a different purpose message than another reachable plugin or app permission",
                    plugin.namespace
                ));
            }
            usage_descriptions.insert(key.clone(), message.clone());
        }
    }
    let entries = usage_descriptions
        .iter()
        .map(|(key, description)| {
            format!(
                "<key>{}</key><string>{}</string>",
                xml_escape(key),
                xml_escape(description)
            )
        })
        .collect::<String>();
    let splash = if config.splash_source.is_some() {
        "<key>UILaunchStoryboardName</key><string>LaunchScreen</string>"
    } else {
        // Apps without a custom splash still need a launch-screen declaration.
        // Without one, iOS can use a legacy 320x480 compatibility scene, which
        // letterboxes modern iPhone apps with black bands above and below.
        "<key>UILaunchScreen</key><dict/>"
    };
    let dev_network = if dev_runtime {
        "<key>NSLocalNetworkUsageDescription</key><string>Connect to the local Nexa development server.</string><key>NSAppTransportSecurity</key><dict><key>NSAllowsLocalNetworking</key><true/></dict>"
    } else {
        ""
    };
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleDisplayName</key><string>{}</string><key>CFBundleIdentifier</key><string>{}</string><key>CFBundleExecutable</key><string>{app_name}</string><key>CFBundleName</key><string>{app_name}</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>{}</string><key>CFBundleVersion</key><string>{}</string><key>LSRequiresIPhoneOS</key><true/>{splash}{dev_network}{entries}</dict></plist>\n",
        xml_escape(&config.display_name),
        xml_escape(&config.ios_bundle_identifier),
        xml_escape(&config.version),
        config.build_number
    ))
}

pub(super) fn ios_launch_storyboard() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><document type=\"com.apple.InterfaceBuilder3.CocoaTouch.Storyboard.XIB\" version=\"3.0\" toolsVersion=\"23094\" targetRuntime=\"iOS.CocoaTouch\" useAutolayout=\"YES\" launchScreen=\"YES\" useTraitCollections=\"YES\"><scenes><scene sceneID=\"launch-scene\"><objects><viewController id=\"launch-controller\" sceneMemberID=\"viewController\"><view key=\"view\" contentMode=\"scaleToFill\" id=\"launch-view\"><rect key=\"frame\" x=\"0.0\" y=\"0.0\" width=\"393\" height=\"852\"/><subviews><imageView contentMode=\"scaleAspectFit\" image=\"NexaSplash\" translatesAutoresizingMaskIntoConstraints=\"NO\" id=\"launch-image\"><rect key=\"frame\" x=\"136\" y=\"366\" width=\"120\" height=\"120\"/></imageView></subviews><constraints><constraint firstItem=\"launch-image\" firstAttribute=\"centerX\" secondItem=\"launch-view\" secondAttribute=\"centerX\" id=\"center-x\"/><constraint firstItem=\"launch-image\" firstAttribute=\"centerY\" secondItem=\"launch-view\" secondAttribute=\"centerY\" id=\"center-y\"/><constraint firstItem=\"launch-image\" firstAttribute=\"width\" constant=\"120\" id=\"image-width\"/><constraint firstItem=\"launch-image\" firstAttribute=\"height\" constant=\"120\" id=\"image-height\"/></constraints><color key=\"backgroundColor\" systemColor=\"systemBackgroundColor\"/><viewLayoutGuide key=\"safeArea\" id=\"safe-area\"/></view></viewController><placeholder placeholderIdentifier=\"IBFirstResponder\" id=\"first-responder\" sceneMemberID=\"firstResponder\"/></objects></scene></scenes><resources><image name=\"NexaSplash\"/></resources></document>\n".to_owned()
}

pub(super) fn ios_entitlements(plugins: &[nexa_ir::Plugin]) -> Result<Option<String>, String> {
    let mut values = std::collections::BTreeMap::<String, nexa_ir::PluginEntitlementValue>::new();
    let mut owners = std::collections::HashMap::<String, &str>::new();
    for plugin in plugins {
        for (key, value) in &plugin.ios_entitlements {
            if let Some(existing) = values.get(key)
                && existing != value
            {
                return Err(format!(
                    "plugins `{}` and `{}` declare conflicting values for iOS entitlement `{key}`",
                    owners.get(key).copied().unwrap_or("<unknown>"),
                    plugin.namespace
                ));
            }
            values.entry(key.clone()).or_insert_with(|| value.clone());
            owners.entry(key.clone()).or_insert(&plugin.namespace);
        }
    }
    if values.is_empty() {
        return Ok(None);
    }

    let entries = values
        .iter()
        .map(|(key, value)| {
            let value = match value {
                nexa_ir::PluginEntitlementValue::String(value) => {
                    format!("<string>{}</string>", xml_escape(value))
                }
                nexa_ir::PluginEntitlementValue::Bool(true) => "<true/>".to_owned(),
                nexa_ir::PluginEntitlementValue::Bool(false) => "<false/>".to_owned(),
                nexa_ir::PluginEntitlementValue::Strings(values) => format!(
                    "<array>{}</array>",
                    values
                        .iter()
                        .map(|value| format!("<string>{}</string>", xml_escape(value)))
                        .collect::<String>()
                ),
            };
            format!("<key>{}</key>{value}", xml_escape(key))
        })
        .collect::<String>();
    Ok(Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>{entries}</dict></plist>\n"
    )))
}

fn merge_swift_packages(plugins: &[nexa_ir::Plugin]) -> Result<Vec<nexa_ir::SwiftPackage>, String> {
    let mut packages: Vec<nexa_ir::SwiftPackage> = Vec::new();
    let mut product_owners = std::collections::HashMap::<String, String>::new();
    for plugin in plugins {
        for package in &plugin.swift_packages {
            if let Some(existing) = packages.iter_mut().find(|value| value.url == package.url) {
                if existing.from != package.from {
                    return Err(format!(
                        "Swift package `{}` has conflicting minimum versions `{}` and `{}`",
                        package.url, existing.from, package.from
                    ));
                }
                for product in &package.products {
                    if let Some(owner) = product_owners.get(product)
                        && owner != &package.url
                    {
                        return Err(format!(
                            "Swift package product `{product}` is declared by both `{owner}` and `{}`",
                            package.url
                        ));
                    }
                    product_owners.insert(product.clone(), package.url.clone());
                    if !existing.products.contains(product) {
                        existing.products.push(product.clone());
                    }
                }
            } else {
                for product in &package.products {
                    if let Some(owner) = product_owners.get(product)
                        && owner != &package.url
                    {
                        return Err(format!(
                            "Swift package product `{product}` is declared by both `{owner}` and `{}`",
                            package.url
                        ));
                    }
                    product_owners.insert(product.clone(), package.url.clone());
                }
                packages.push(package.clone());
            }
        }
    }
    Ok(packages)
}

fn render_swift_package_objects(packages: &[nexa_ir::SwiftPackage]) -> String {
    let mut objects = String::new();
    let mut product_index = 0;
    for (package_index, package) in packages.iter().enumerate() {
        objects.push_str(&format!(
            "\n\t\t{} = {{ isa = XCRemoteSwiftPackageReference; repositoryURL = \"{}\"; requirement = {{ kind = upToNextMajorVersion; minimumVersion = \"{}\"; }}; }};",
            pbx_identifier(1000 + package_index),
            package.url,
            package.from
        ));
        for product in &package.products {
            objects.push_str(&format!(
                "\n\t\t{} = {{ isa = XCSwiftPackageProductDependency; package = {}; productName = \"{}\"; }};\n\t\t{} = {{ isa = PBXBuildFile; productRef = {}; }};",
                pbx_identifier(2000 + product_index),
                pbx_identifier(1000 + package_index),
                product,
                pbx_identifier(3000 + product_index),
                pbx_identifier(2000 + product_index)
            ));
            product_index += 1;
        }
    }
    objects
}

pub(super) fn pbx_identifier(index: usize) -> String {
    format!("BB{:022X}", index)
}

fn minimum_ios_version(configured: &str, plugins: &[nexa_ir::Plugin]) -> Result<String, String> {
    let mut minimum = configured
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| format!("invalid iOS minimum version `{configured}`"))?;
    for version in plugins
        .iter()
        .filter_map(|plugin| plugin.ios_min_version.as_deref())
    {
        let parts = version
            .split('.')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| format!("invalid iOS minimum version `{version}`"))?;
        if parts.len() < 2 || parts.len() > 3 {
            return Err(format!("invalid iOS minimum version `{version}`"));
        }
        let width = minimum.len().max(parts.len());
        let greater = (0..width)
            .map(|index| {
                (
                    *parts.get(index).unwrap_or(&0),
                    *minimum.get(index).unwrap_or(&0),
                )
            })
            .find(|(left, right)| left != right)
            .is_some_and(|(left, right)| left > right);
        if greater {
            minimum = parts;
        }
    }
    if plugins.iter().any(|plugin| !plugin.cpp_sources.is_empty()) {
        let cpp_interop_minimum = [16, 4];
        let width = minimum.len().max(cpp_interop_minimum.len());
        let greater = (0..width)
            .map(|index| {
                (
                    *cpp_interop_minimum.get(index).unwrap_or(&0),
                    *minimum.get(index).unwrap_or(&0),
                )
            })
            .find(|(required, current)| required != current)
            .is_some_and(|(required, current)| required > current);
        if greater {
            minimum = cpp_interop_minimum.to_vec();
        }
    }
    let mut formatted = minimum.iter().map(u32::to_string).collect::<Vec<_>>();
    while formatted.len() > 2 && formatted.last().is_some_and(|part| part == "0") {
        formatted.pop();
    }
    Ok(formatted.join("."))
}

fn merge_maven_dependencies(plugins: &[nexa_ir::Plugin]) -> Result<Vec<String>, String> {
    let mut dependencies = Vec::new();
    let mut versions = std::collections::HashMap::<String, String>::new();
    for dependency in plugins
        .iter()
        .flat_map(|plugin| plugin.maven_dependencies.iter())
    {
        let parts = dependency.split(':').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(format!("invalid Android Maven dependency `{dependency}`"));
        }
        let artifact = format!("{}:{}", parts[0], parts[1]);
        if let Some(version) = versions.get(&artifact) {
            if version != parts[2] {
                return Err(format!(
                    "Android Maven artifact `{artifact}` has conflicting versions `{version}` and `{}`",
                    parts[2]
                ));
            }
            continue;
        }
        versions.insert(artifact, parts[2].to_owned());
        dependencies.push(dependency.clone());
    }
    Ok(dependencies)
}

pub(super) fn ios_project_file_with_config(
    app_name: &str,
    has_assets: bool,
    has_plugin_resources: bool,
    generated_sources: &[String],
    plugin_sources: &[String],
    cpp_sources: &[String],
    xcframeworks: &[String],
    plugins: &[nexa_ir::Plugin],
    config: &ProjectConfig,
) -> Result<String, String> {
    let packages = merge_swift_packages(plugins)?;
    let frameworks = merge_ios_frameworks(plugins);
    let product_count = packages
        .iter()
        .map(|package| package.products.len())
        .sum::<usize>();
    let package_reference_ids = (0..packages.len())
        .map(|index| pbx_identifier(1000 + index))
        .collect::<Vec<_>>();
    let package_product_ids = (0..product_count)
        .map(|index| pbx_identifier(2000 + index))
        .collect::<Vec<_>>();
    let package_build_ids = (0..product_count)
        .map(|index| pbx_identifier(3000 + index))
        .collect::<Vec<_>>();
    let package_objects = render_swift_package_objects(&packages);
    let framework_reference_ids = (0..frameworks.len())
        .map(|index| pbx_identifier(4000 + index))
        .collect::<Vec<_>>();
    let framework_build_ids = (0..frameworks.len())
        .map(|index| pbx_identifier(5000 + index))
        .collect::<Vec<_>>();
    let xcframework_link_ids = (0..xcframeworks.len())
        .map(|index| pbx_identifier(7000 + index))
        .collect::<Vec<_>>();
    let framework_objects = render_ios_framework_objects(&frameworks);
    let embed_phase = render_ios_xcframework_embed_phase(xcframeworks.len());
    let mut project = ios_project_base_file(
        app_name,
        has_assets,
        has_plugin_resources,
        generated_sources,
        plugin_sources,
        xcframeworks,
    );
    let has_icon_composer = config
        .ios_icon
        .as_ref()
        .or(config.icon_source.as_ref())
        .is_some_and(|path| {
            path.extension()
                .is_some_and(|extension| extension == "icon")
        });
    if has_icon_composer {
        let reference_id = pbx_identifier(10012);
        let build_id = pbx_identifier(10013);
        project = project.replace(
            "PBXResourcesBuildPhase; files = (",
            &format!("PBXResourcesBuildPhase; files = ( {build_id},"),
        );
        let objects = format!(
            "\n\t\t{reference_id} = {{ isa = PBXFileReference; lastKnownFileType = folder.iconcomposer.icon; path = {app_name}/AppIcon.icon; sourceTree = SOURCE_ROOT; }};\n\t\t{build_id} = {{ isa = PBXBuildFile; fileRef = {reference_id}; }};\n"
        );
        project = project.replace(
            "AA0000000000000000000005 = { isa = PBXNativeTarget;",
            &format!("{objects}\t\tAA0000000000000000000005 = {{ isa = PBXNativeTarget;"),
        );
    }
    if config.splash_source.is_some() {
        let reference_id = pbx_identifier(10020);
        let build_id = pbx_identifier(10021);
        project = project.replace(
            "PBXResourcesBuildPhase; files = (",
            &format!("PBXResourcesBuildPhase; files = ( {build_id},"),
        );
        let objects = format!(
            "\n\t\t{reference_id} = {{ isa = PBXFileReference; lastKnownFileType = file.storyboard; path = {app_name}/LaunchScreen.storyboard; sourceTree = SOURCE_ROOT; }};\n\t\t{build_id} = {{ isa = PBXBuildFile; fileRef = {reference_id}; }};\n"
        );
        project = project.replace(
            "AA0000000000000000000005 = { isa = PBXNativeTarget;",
            &format!("{objects}\t\tAA0000000000000000000005 = {{ isa = PBXNativeTarget;"),
        );
    }
    add_ios_cpp_objects(&mut project, app_name, cpp_sources, plugins)?;
    if !framework_reference_ids.is_empty() {
        project = project.replace(
            "children = ( AA0000000000000000000014, AA0000000000000000000004 );",
            &format!(
                "children = ( AA0000000000000000000014, AA0000000000000000000004, {} );",
                framework_reference_ids.join(", ")
            ),
        );
    }
    project = project.replace(
        "targets = ( AA0000000000000000000005 );",
        &format!(
            "targets = ( AA0000000000000000000005 ); packageReferences = ( {} );",
            package_reference_ids.join(", ")
        ),
    );
    project = project.replace(
        "productType = \"com.apple.product-type.application\"; };",
        &format!(
            "productType = \"com.apple.product-type.application\"; packageProductDependencies = ( {} ); }};",
            package_product_ids.join(", ")
        ),
    );
    project = project.replace(
        "PBXFrameworksBuildPhase; files = (); };",
        &format!(
            "PBXFrameworksBuildPhase; files = ( {} ); }};",
            package_build_ids
                .iter()
                .chain(&framework_build_ids)
                .chain(&xcframework_link_ids)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    if !package_objects.is_empty() || !framework_objects.is_empty() || !embed_phase.is_empty() {
        let insertion = "\n\t\tAA0000000000000000000005 = { isa = PBXNativeTarget;";
        project = project.replace(
            insertion,
            &format!("{package_objects}{framework_objects}{embed_phase}{insertion}"),
        );
    }
    if !xcframeworks.is_empty() {
        let embed_phase_id = pbx_identifier(9000);
        project = project.replace(
            "buildPhases = ( AA0000000000000000000008, AA0000000000000000000009, AA000000000000000000000A );",
            &format!("buildPhases = ( AA0000000000000000000008, AA0000000000000000000009, AA000000000000000000000A, {embed_phase_id} );"),
        );
    }
    let minimum_version = minimum_ios_version(&config.ios_min_version, plugins)?;
    project = project.replace(
        "IPHONEOS_DEPLOYMENT_TARGET = 17.0",
        &format!("IPHONEOS_DEPLOYMENT_TARGET = {minimum_version}"),
    );
    project = project.replace(
        &format!(
            "PRODUCT_BUNDLE_IDENTIFIER = com.nexa.{};",
            app_name.to_ascii_lowercase()
        ),
        &format!(
            "PRODUCT_BUNDLE_IDENTIFIER = {};",
            config.ios_bundle_identifier
        ),
    );
    project = project.replace(
        &format!(
            "PRODUCT_BUNDLE_IDENTIFIER = com.nexa.{};",
            app_name.to_ascii_lowercase()
        ),
        &format!(
            "PRODUCT_BUNDLE_IDENTIFIER = {};",
            config.ios_bundle_identifier
        ),
    );
    let mut extra_target_settings = String::new();
    let has_icon_composer = config
        .ios_icon
        .as_ref()
        .or(config.icon_source.as_ref())
        .is_some_and(|path| {
            path.extension()
                .is_some_and(|extension| extension == "icon")
        });
    if (config.ios_icon.is_some() || config.icon_source.is_some()) && !has_icon_composer {
        extra_target_settings.push_str(" ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon;");
    }
    if plugins
        .iter()
        .any(|plugin| !plugin.ios_entitlements.is_empty())
    {
        extra_target_settings.push_str(&format!(
            " CODE_SIGN_ENTITLEMENTS = {app_name}/Nexa.entitlements;"
        ));
    }
    let linker_flags = merge_ios_linker_flags(plugins);
    if !linker_flags.is_empty() {
        let flags = linker_flags
            .iter()
            .map(|flag| pbx_quote(flag))
            .collect::<Vec<_>>()
            .join(", ");
        extra_target_settings.push_str(&format!(" OTHER_LDFLAGS = ( \"$(inherited)\", {flags} );"));
    }
    if !extra_target_settings.is_empty() {
        project = project.replace(
            "TARGETED_DEVICE_FAMILY = \"1,2\";",
            &format!("TARGETED_DEVICE_FAMILY = \"1,2\";{extra_target_settings}"),
        );
    }
    Ok(project)
}

fn add_ios_cpp_objects(
    project: &mut String,
    app_name: &str,
    cpp_sources: &[String],
    plugins: &[nexa_ir::Plugin],
) -> Result<(), String> {
    if cpp_sources.is_empty() {
        return Ok(());
    }
    let group_children = cpp_sources
        .iter()
        .enumerate()
        .map(|(index, _)| format!(", {}", pbx_identifier(12000 + index)))
        .collect::<String>();
    *project = project.replacen(
        "AA0000000000000000000012",
        &format!("AA0000000000000000000012{group_children}"),
        1,
    );
    let file_references = cpp_sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            format!(
                "\n\t\t{} = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.cpp.cpp; path = {}; sourceTree = \"<group>\"; }};",
                pbx_identifier(12000 + index),
                pbx_quote(&format!("NexaPluginCpp/{source}"))
            )
        })
        .collect::<String>();
    let build_files = cpp_sources
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "\n\t\t{} = {{ isa = PBXBuildFile; fileRef = {}; }};",
                pbx_identifier(13000 + index),
                pbx_identifier(12000 + index)
            )
        })
        .collect::<String>();
    let build_ids = cpp_sources
        .iter()
        .enumerate()
        .map(|(index, _)| format!(", {}", pbx_identifier(13000 + index)))
        .collect::<String>();
    let group_marker = "AA0000000000000000000004 = { isa = PBXGroup;";
    if let Some(position) = project.find(group_marker) {
        project.insert_str(position, &file_references);
    }
    let build_marker = "AA0000000000000000000005 = { isa = PBXNativeTarget;";
    if let Some(position) = project.find(build_marker) {
        project.insert_str(position, &build_files);
    }
    let sources_marker = "PBXSourcesBuildPhase; files = (";
    if let Some(position) = project.find(sources_marker) {
        let phase_end = project[position..]
            .find(" );")
            .map(|end| position + end)
            .unwrap_or(position + sources_marker.len());
        project.insert_str(phase_end, &build_ids);
    }
    let mut include_paths = cpp_sources
        .iter()
        .filter_map(|source| Path::new(source).components().next())
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<std::collections::BTreeSet<_>>();
    for (index, plugin) in plugins.iter().enumerate() {
        if plugin.cpp_sources.is_empty() {
            continue;
        }
        include_paths.extend(super::plugins::cpp_header_include_roots(plugin, index)?);
    }
    let include_paths = include_paths
        .into_iter()
        .map(|plugin_root| {
            pbx_quote(&format!(
                "$(PROJECT_DIR)/{app_name}/NexaPluginCpp/{plugin_root}"
            ))
        })
        .collect::<Vec<_>>();
    let cpp_standard = super::plugins::minimum_cpp_standard(plugins);
    let settings = format!(
        "CLANG_CXX_LANGUAGE_STANDARD = \"c++{cpp_standard}\"; SWIFT_OBJC_INTEROP_MODE = objcxx; SWIFT_OBJC_BRIDGING_HEADER = {}; HEADER_SEARCH_PATHS = ( \"$(inherited)\", {} );",
        pbx_quote(&format!(
            "$(PROJECT_DIR)/{app_name}/NexaPluginCpp-Bridging-Header.h"
        )),
        include_paths.join(", ")
    );
    *project = project.replace(
        "SWIFT_VERSION = 6.0;",
        &format!("SWIFT_VERSION = 6.0; {settings}"),
    );
    Ok(())
}

fn merge_ios_frameworks(plugins: &[nexa_ir::Plugin]) -> Vec<String> {
    let mut frameworks = Vec::new();
    for framework in plugins
        .iter()
        .flat_map(|plugin| plugin.ios_frameworks.iter())
    {
        if !frameworks.contains(framework) {
            frameworks.push(framework.clone());
        }
    }
    frameworks
}

fn merge_ios_linker_flags(plugins: &[nexa_ir::Plugin]) -> Vec<String> {
    plugins
        .iter()
        .flat_map(|plugin| plugin.ios_linker_flags.iter().cloned())
        .collect()
}

fn pbx_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn render_ios_framework_objects(frameworks: &[String]) -> String {
    let mut objects = String::new();
    for (index, framework) in frameworks.iter().enumerate() {
        let reference_id = pbx_identifier(4000 + index);
        let build_id = pbx_identifier(5000 + index);
        objects.push_str(&format!(
            "\n\t\t{reference_id} = {{ isa = PBXFileReference; lastKnownFileType = wrapper.framework; name = \"{framework}.framework\"; path = System/Library/Frameworks/{framework}.framework; sourceTree = SDKROOT; }};\n\t\t{build_id} = {{ isa = PBXBuildFile; fileRef = {reference_id}; }};"
        ));
    }
    objects
}

fn render_ios_xcframework_objects(xcframeworks: &[String]) -> String {
    let mut objects = String::new();
    for (index, path) in xcframeworks.iter().enumerate() {
        let reference_id = pbx_identifier(6000 + index);
        let link_id = pbx_identifier(7000 + index);
        let embed_id = pbx_identifier(8000 + index);
        let path = pbx_quote(path);
        objects.push_str(&format!(
            "\n\t\t{reference_id} = {{ isa = PBXFileReference; lastKnownFileType = wrapper.xcframework; path = {path}; sourceTree = \"<group>\"; }};\n\t\t{link_id} = {{ isa = PBXBuildFile; fileRef = {reference_id}; }};\n\t\t{embed_id} = {{ isa = PBXBuildFile; fileRef = {reference_id}; settings = {{ ATTRIBUTES = (CodeSignOnCopy, RemoveHeadersOnCopy); }}; }};"
        ));
    }
    objects
}

fn render_ios_xcframework_embed_phase(count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    let files = (0..count)
        .map(|index| pbx_identifier(8000 + index))
        .collect::<Vec<_>>()
        .join(", ");
    let phase_id = pbx_identifier(9000);
    format!(
        "\n\t\t{phase_id} = {{ isa = PBXCopyFilesBuildPhase; buildActionMask = 2147483647; dstPath = \"\"; dstSubfolderSpec = 10; files = ( {files} ); name = \"Embed Frameworks\"; runOnlyForDeploymentPostprocessing = 0; }};"
    )
}

fn ios_project_base_file(
    app_name: &str,
    has_assets: bool,
    has_plugin_resources: bool,
    generated_sources: &[String],
    plugin_sources: &[String],
    xcframeworks: &[String],
) -> String {
    let app_file = format!("{app_name}App.swift");
    let asset_reference_id = pbx_identifier(10010);
    let asset_build_id = pbx_identifier(10011);
    let asset_group = if has_assets {
        format!(", {asset_reference_id}")
    } else {
        String::new()
    };
    let asset_reference = if has_assets {
        format!(
            "\n\t\t{asset_reference_id} = {{ isa = PBXFileReference; lastKnownFileType = folder.assetcatalog; path = Assets.xcassets; sourceTree = \"<group>\"; }};"
        )
    } else {
        String::new()
    };
    let asset_build_file = if has_assets {
        format!(
            "\n\t\t{asset_build_id} = {{ isa = PBXBuildFile; fileRef = {asset_reference_id}; }};"
        )
    } else {
        String::new()
    };
    let resource_files = if has_assets {
        asset_build_id.clone()
    } else {
        String::new()
    };
    let plugin_group_children = plugin_sources
        .iter()
        .enumerate()
        .map(|(index, _)| format!(", AA00000000000000000000{:02}", 30 + index))
        .collect::<String>();
    let xcframework_group_children = (0..xcframeworks.len())
        .map(|index| format!(", {}", pbx_identifier(6000 + index)))
        .collect::<String>();
    let xcframework_objects = render_ios_xcframework_objects(xcframeworks);
    let plugin_file_references = plugin_sources
        .iter()
        .enumerate()
        .map(|(index, name)| {
            format!(
                "\n\t\tAA00000000000000000000{:02} = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = NexaPlugins/{name}; sourceTree = \"<group>\"; }};",
                30 + index
            )
        })
        .collect::<String>();
    let plugin_build_files = plugin_sources
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "\n\t\tAA00000000000000000000{:02} = {{ isa = PBXBuildFile; fileRef = AA00000000000000000000{:02}; }};",
                60 + index,
                30 + index
            )
        })
        .collect::<String>();
    let plugin_build_ids = plugin_sources
        .iter()
        .enumerate()
        .map(|(index, _)| format!(", AA00000000000000000000{:02}", 60 + index))
        .collect::<String>();
    let generated_group_children = generated_sources
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, _)| format!(", AA00000000000000000000{:02}", 40 + index))
        .collect::<String>();
    let generated_file_references = generated_sources
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, name)| {
            format!(
                "\n\t\tAA00000000000000000000{:02} = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {name}; sourceTree = \"<group>\"; }};",
                40 + index
            )
        })
        .collect::<String>();
    let generated_build_files = generated_sources
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, _)| {
            format!(
                "\n\t\tAA00000000000000000000{:02} = {{ isa = PBXBuildFile; fileRef = AA00000000000000000000{:02}; }};",
                90 + index,
                40 + index
            )
        })
        .collect::<String>();
    let generated_build_ids = generated_sources
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, _)| format!(", AA00000000000000000000{:02}", 90 + index))
        .collect::<String>();
    let mut project = format!(
        "// !$*UTF8*$!\n{{\n\tarchiveVersion = 1;\n\tclasses = {{}};\n\tobjectVersion = 77;\n\tobjects = {{\n\t\tAA0000000000000000000001 = {{ isa = PBXProject; buildConfigurationList = AA0000000000000000000002; compatibilityVersion = \"Xcode 16.0\"; mainGroup = AA0000000000000000000003; productRefGroup = AA0000000000000000000004; targets = ( AA0000000000000000000005 ); }};\n\t\tAA0000000000000000000003 = {{ isa = PBXGroup; children = ( AA0000000000000000000014, AA0000000000000000000004 ); sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000014 = {{ isa = PBXGroup; children = ( AA0000000000000000000010, AA0000000000000000000011, AA0000000000000000000012{asset_group}{generated_group_children}{plugin_group_children}{xcframework_group_children} ); path = {app_name}; sourceTree = \"<group>\"; }};{asset_reference}{generated_file_references}{plugin_file_references}{xcframework_objects}\n\t\tAA0000000000000000000004 = {{ isa = PBXGroup; children = ( AA0000000000000000000013 ); name = Products; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000010 = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {app_file}; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000011 = {{ isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = NexaGenerated.swift; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000012 = {{ isa = PBXFileReference; lastKnownFileType = text.plist.xml; path = Info.plist; sourceTree = \"<group>\"; }};\n\t\tAA0000000000000000000013 = {{ isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = {app_name}.app; sourceTree = BUILT_PRODUCTS_DIR; }};\n\t\tAA0000000000000000000020 = {{ isa = PBXBuildFile; fileRef = AA0000000000000000000010; }};\n\t\tAA0000000000000000000021 = {{ isa = PBXBuildFile; fileRef = AA0000000000000000000011; }};{generated_build_files}{asset_build_file}{plugin_build_files}\n\t\tAA0000000000000000000005 = {{ isa = PBXNativeTarget; buildConfigurationList = AA0000000000000000000007; buildPhases = ( AA0000000000000000000008, AA0000000000000000000009, AA000000000000000000000A ); name = {app_name}; productName = {app_name}; productReference = AA0000000000000000000013; productType = \"com.apple.product-type.application\"; }};\n\t\tAA0000000000000000000008 = {{ isa = PBXSourcesBuildPhase; files = ( AA0000000000000000000020, AA0000000000000000000021{generated_build_ids}{plugin_build_ids} ); }};\n\t\tAA0000000000000000000009 = {{ isa = PBXFrameworksBuildPhase; files = (); }};\n\t\tAA000000000000000000000A = {{ isa = PBXResourcesBuildPhase; files = ( {resource_files} ); }};\n\t\tAA0000000000000000000002 = {{ isa = XCConfigurationList; buildConfigurations = ( AA0000000000000000000022 ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; }};\n\t\tAA0000000000000000000007 = {{ isa = XCConfigurationList; buildConfigurations = ( AA0000000000000000000023 ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; }};\n\t\tAA0000000000000000000022 = {{ isa = XCBuildConfiguration; buildSettings = {{ ALWAYS_SEARCH_USER_PATHS = NO; SWIFT_VERSION = 5.0; SWIFT_OPTIMIZATION_LEVEL = \"-O\"; SWIFT_COMPILATION_MODE = wholemodule; GCC_OPTIMIZATION_LEVEL = s; DEAD_CODE_STRIPPING = YES; IPHONEOS_DEPLOYMENT_TARGET = 16.0; }}; name = Release; }};\n\t\tAA0000000000000000000023 = {{ isa = XCBuildConfiguration; buildSettings = {{ ALWAYS_SEARCH_USER_PATHS = NO; PRODUCT_BUNDLE_IDENTIFIER = com.nexa.{}; PRODUCT_NAME = {app_name}; INFOPLIST_FILE = {app_name}/Info.plist; SUPPORTED_PLATFORMS = \"iphoneos iphonesimulator\"; SWIFT_VERSION = 5.0; SWIFT_OPTIMIZATION_LEVEL = \"-O\"; SWIFT_COMPILATION_MODE = wholemodule; GCC_OPTIMIZATION_LEVEL = s; DEAD_CODE_STRIPPING = YES; IPHONEOS_DEPLOYMENT_TARGET = 16.0; TARGETED_DEVICE_FAMILY = \"1,2\"; }}; name = Release; }};\n\t}};\n\trootObject = AA0000000000000000000001;\n}}\n",
        app_name.to_ascii_lowercase()
    )
    .replace("compatibilityVersion = \"Xcode 16.0\"", "compatibilityVersion = \"Xcode 27.0\"")
    .replace("SWIFT_VERSION = 5.0", "SWIFT_VERSION = 6.0")
    .replace("IPHONEOS_DEPLOYMENT_TARGET = 16.0", "IPHONEOS_DEPLOYMENT_TARGET = 17.0");
    project = project
        .replace(
            "buildConfigurations = ( AA0000000000000000000022 );",
            "buildConfigurations = ( AA0000000000000000000022, AA0000000000000000000024 );",
        )
        .replace(
            "buildConfigurations = ( AA0000000000000000000023 );",
            "buildConfigurations = ( AA0000000000000000000023, AA0000000000000000000025 );",
        );
    let debug_configurations = "\n\t\tAA0000000000000000000024 = { isa = XCBuildConfiguration; buildSettings = { ALWAYS_SEARCH_USER_PATHS = NO; SWIFT_VERSION = 6.0; SWIFT_OPTIMIZATION_LEVEL = \"-Onone\"; IPHONEOS_DEPLOYMENT_TARGET = 17.0; }; name = Debug; };\n\t\tAA0000000000000000000025 = { isa = XCBuildConfiguration; buildSettings = { ALWAYS_SEARCH_USER_PATHS = NO; PRODUCT_BUNDLE_IDENTIFIER = com.nexa.APP_ID; PRODUCT_NAME = APP_NAME; INFOPLIST_FILE = APP_NAME/Info.plist; SUPPORTED_PLATFORMS = \"iphoneos iphonesimulator\"; SWIFT_VERSION = 6.0; SWIFT_OPTIMIZATION_LEVEL = \"-Onone\"; IPHONEOS_DEPLOYMENT_TARGET = 17.0; TARGETED_DEVICE_FAMILY = \"1,2\"; }; name = Debug; };\n";
    let debug_configurations = debug_configurations
        .replace("APP_ID", &app_name.to_ascii_lowercase())
        .replace("APP_NAME", app_name);
    project = project.replace(
        "\n\t};\n\trootObject = AA0000000000000000000001;",
        &format!("{debug_configurations}\n\t}};\n\trootObject = AA0000000000000000000001;"),
    );
    if has_plugin_resources {
        let reference_id = pbx_identifier(10000);
        let build_id = pbx_identifier(10001);
        project = project.replacen(
            "AA0000000000000000000012",
            &format!("AA0000000000000000000012, {reference_id}"),
            1,
        );
        let old_resource_phase = format!("PBXResourcesBuildPhase; files = ( {resource_files} );");
        let mut resource_ids = Vec::new();
        if !resource_files.is_empty() {
            resource_ids.push(resource_files.as_str());
        }
        resource_ids.push(&build_id);
        project = project.replace(
            &old_resource_phase,
            &format!(
                "PBXResourcesBuildPhase; files = ( {} );",
                resource_ids.join(", ")
            ),
        );
        let insertion = "\n\t\tAA0000000000000000000005 = { isa = PBXNativeTarget;";
        project = project.replace(
            insertion,
            &format!(
                "\n\t\t{reference_id} = {{ isa = PBXFileReference; lastKnownFileType = folder; path = NexaPluginResources; sourceTree = \"<group>\"; }};\n\t\t{build_id} = {{ isa = PBXBuildFile; fileRef = {reference_id}; }};{insertion}"
            ),
        );
    }
    project
}

pub(super) fn ios_scheme(app_name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Scheme LastUpgradeVersion="2700" version="1.7">
   <BuildAction parallelizeBuildables="YES" buildImplicitDependencies="YES">
      <BuildActionEntries>
         <BuildActionEntry buildForTesting="YES" buildForRunning="YES" buildForProfiling="YES" buildForArchiving="YES" buildForAnalyzing="YES">
            <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="AA0000000000000000000005" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
         </BuildActionEntry>
      </BuildActionEntries>
   </BuildAction>
   <TestAction buildConfiguration="Debug" shouldUseLaunchSchemeArgsEnv="YES"/>
   <LaunchAction buildConfiguration="Debug" useCustomWorkingDirectory="NO" ignoresPersistentStateOnLaunch="NO" debugDocumentVersioning="YES" debugServiceExtension="internal" allowLocationSimulation="YES">
      <BuildableProductRunnable runnableDebuggingMode="0">
         <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="AA0000000000000000000005" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
      </BuildableProductRunnable>
   </LaunchAction>
   <ProfileAction buildConfiguration="Release" shouldUseLaunchSchemeArgsEnv="YES" savedToolIdentifier="" useCustomWorkingDirectory="NO" debugDocumentVersioning="YES">
      <BuildableProductRunnable runnableDebuggingMode="0">
         <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="AA0000000000000000000005" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
      </BuildableProductRunnable>
   </ProfileAction>
   <AnalyzeAction buildConfiguration="Release"/>
   <ArchiveAction buildConfiguration="Release" revealArchiveInOrganizer="YES"/>
</Scheme>
"#
    )
}

pub(super) fn android_settings(app_name: &str, plugins: &[nexa_ir::Plugin]) -> String {
    let mut repositories: Vec<&str> = Vec::new();
    for repository in plugins
        .iter()
        .flat_map(|plugin| plugin.android_maven_repositories.iter())
    {
        if !repositories.contains(&repository.as_str()) {
            repositories.push(repository.as_str());
        }
    }
    let custom_repositories = repositories
        .iter()
        .map(|repository| format!(" maven {{ url = uri(\"{repository}\") }}"))
        .collect::<String>();
    format!(
        "pluginManagement {{ repositories {{ google(); mavenCentral(); gradlePluginPortal() }} }}\ndependencyResolutionManagement {{ repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories {{ google(); mavenCentral();{custom_repositories} }} }}\nrootProject.name = \"{app_name}\"\ninclude(\":app\")\n"
    )
}
pub(super) fn android_root_gradle() -> String {
    "plugins {\n    id(\"com.android.application\") version \"9.2.1\" apply false\n    id(\"org.jetbrains.kotlin.plugin.compose\") version \"2.4.20\" apply false\n}\n".to_owned()
}
pub(super) fn android_properties() -> String {
    "android.useAndroidX=true\nkotlin.code.style=official\norg.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n".to_owned()
}

pub(super) fn android_gradle_wrapper_properties() -> String {
    "distributionBase=GRADLE_USER_HOME\ndistributionPath=wrapper/dists\ndistributionUrl=https\\://services.gradle.org/distributions/gradle-9.4.1-bin.zip\ndistributionSha256Sum=2ab2958f2a1e51120c326cad6f385153bb11ee93b3c216c5fccebfdfbb7ec6cb\nnetworkTimeout=10000\nvalidateDistributionUrl=true\nzipStoreBase=GRADLE_USER_HOME\nzipStorePath=wrapper/dists\n".to_owned()
}

pub(super) const ANDROID_GRADLE_WRAPPER_JAR: &[u8] =
    include_bytes!("../../resources/gradle-wrapper/gradle-wrapper.jar");
pub(super) const ANDROID_GRADLEW: &str = include_str!("../../resources/gradle-wrapper/gradlew");
pub(super) const ANDROID_GRADLEW_BAT: &str =
    include_str!("../../resources/gradle-wrapper/gradlew.bat");

pub(super) fn android_manifest(
    _app_name: &str,
    package: &str,
    remote: bool,
    config: &ProjectConfig,
    plugins: &[nexa_ir::Plugin],
) -> String {
    let mut permissions = std::collections::BTreeSet::new();
    if remote {
        permissions.insert("android.permission.INTERNET".to_owned());
    }
    for (permission, _) in config.permissions() {
        let names: &[&str] = match *permission {
            Permission::Camera => &["android.permission.CAMERA"],
            Permission::Microphone => &["android.permission.RECORD_AUDIO"],
            Permission::Photos => &[
                "android.permission.READ_MEDIA_IMAGES",
                "android.permission.READ_EXTERNAL_STORAGE",
            ],
            Permission::Location => &[
                "android.permission.ACCESS_COARSE_LOCATION",
                "android.permission.ACCESS_FINE_LOCATION",
            ],
            Permission::Notifications => &["android.permission.POST_NOTIFICATIONS"],
            Permission::Contacts => &[
                "android.permission.READ_CONTACTS",
                "android.permission.WRITE_CONTACTS",
            ],
            Permission::Calendar => &[
                "android.permission.READ_CALENDAR",
                "android.permission.WRITE_CALENDAR",
            ],
            Permission::Bluetooth => &[
                "android.permission.BLUETOOTH_SCAN",
                "android.permission.BLUETOOTH_CONNECT",
            ],
        };
        for name in names {
            permissions.insert((*name).to_owned());
        }
    }
    permissions.extend(
        plugins
            .iter()
            .flat_map(|plugin| plugin.android_permissions.iter().cloned()),
    );
    let declared = permissions
        .iter()
        .map(|name| {
            format!(
                "    <uses-permission android:name=\"{}\" />\n",
                xml_escape(name)
            )
        })
        .collect::<String>();
    let icon_attribute = if config.android_icon.is_some() || config.icon_source.is_some() {
        " android:icon=\"@mipmap/ic_launcher\" android:roundIcon=\"@mipmap/ic_launcher_round\""
    } else {
        ""
    };
    let app_theme = if config.splash_source.is_some() {
        "@style/NexaSplashTheme"
    } else {
        "@android:style/Theme.Material.Light.NoActionBar"
    };
    format!(
        "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">\n{declared}    <application android:label=\"{}\"{icon_attribute} android:theme=\"{app_theme}\" android:enableOnBackInvokedCallback=\"true\">\n        <activity android:name=\"{package}.MainActivity\" android:exported=\"true\">\n            <intent-filter><action android:name=\"android.intent.action.MAIN\"/><category android:name=\"android.intent.category.LAUNCHER\"/></intent-filter>\n        </activity>\n    </application>\n</manifest>\n",
        xml_escape(&config.display_name),
    )
}

pub(super) fn android_app_gradle_with_dev_runtime(
    package: &str,
    features: nexa_backend_kotlin::KotlinProjectFeatures,
    plugins: &[nexa_ir::Plugin],
    local_aars: &[String],
    config: &ProjectConfig,
    dev_runtime: bool,
) -> Result<String, String> {
    let maven_dependencies = merge_maven_dependencies(plugins)?;
    let minimum_sdk = plugins
        .iter()
        .filter_map(|plugin| plugin.android_min_sdk)
        .max()
        .unwrap_or(config.android_min_sdk)
        .max(config.android_min_sdk);
    if minimum_sdk == 0 || minimum_sdk > config.android_target_sdk {
        return Err(format!(
            "Android plugin requirements raise minSdk to {minimum_sdk}, which must be between 1 and targetSdk ({})",
            config.android_target_sdk
        ));
    }
    let cpp_native_build = if plugins.iter().any(|plugin| !plugin.cpp_sources.is_empty()) {
        "\n    externalNativeBuild { cmake { path = file(\"src/main/cpp/CMakeLists.txt\"); version = \"3.22.1\" } }"
    } else {
        ""
    };
    let mut dependencies = String::from(
        "    implementation(platform(\"androidx.compose:compose-bom:2026.09.00\"))\n    implementation(\"androidx.activity:activity-compose:1.13.0\")\n    implementation(\"androidx.compose.ui:ui\")\n    implementation(\"androidx.compose.material3:material3\")\n",
    );
    if dev_runtime {
        dependencies.push_str(
            "    implementation(\"androidx.compose.foundation:foundation\")\n    implementation(\"androidx.compose.runtime:runtime\")\n    implementation(\"androidx.core:core\")\n",
        );
    }
    if dev_runtime || features.uses_compose_graphics {
        dependencies.push_str("    implementation(\"androidx.compose.ui:ui-graphics\")\n");
    }
    if dev_runtime || features.uses_navigation {
        dependencies
            .push_str("    implementation(\"androidx.navigation:navigation-compose:2.10.1\")\n");
    }
    if dev_runtime || features.uses_lifecycle_events {
        dependencies.push_str(
            "    implementation(\"androidx.lifecycle:lifecycle-runtime-compose:2.11.0\")\n",
        );
    }
    if dev_runtime || features.uses_remote_image {
        dependencies.push_str(
            "    implementation(\"io.coil-kt.coil3:coil-compose:3.6.3\")\n    implementation(\"io.coil-kt.coil3:coil-network-core:3.6.3\")\n",
        );
    }
    if dev_runtime || features.uses_coroutines {
        dependencies.push_str(
            "    implementation(\"org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0\")\n",
        );
    }
    if config.splash_source.is_some() {
        dependencies.push_str("    implementation(\"androidx.core:core-splashscreen:1.0.1\")\n");
    }
    if dev_runtime || features.uses_network {
        dependencies.push_str(
            "    implementation(\"com.google.android.gms:play-services-cronet:18.0.1\")\n",
        );
    }
    // Keep the debug runtime's network API usable when Play Services Cronet
    // cannot install its provider (common on emulator images and non-GMS
    // devices). CronetEngine.Builder selects this bundled provider as a
    // fallback while retaining the same NexaNetwork implementation.
    if dev_runtime {
        dependencies
            .push_str("    implementation(\"org.chromium.net:cronet-embedded:143.7445.0\")\n");
    }
    for dependency in maven_dependencies {
        dependencies.push_str(&format!("    implementation(\"{dependency}\")\n"));
    }
    for aar in local_aars {
        dependencies.push_str(&format!("    implementation(files(\"libs/{aar}\"))\n"));
    }
    Ok(format!(
        "plugins {{\n    id(\"com.android.application\")\n    id(\"org.jetbrains.kotlin.plugin.compose\")\n}}\n\nandroid {{\n    namespace = \"{package}\"\n    compileSdk = 37\n    defaultConfig {{ applicationId = \"{}\"; minSdk = {minimum_sdk}; targetSdk = {}; versionCode = {}; versionName = \"{}\" }}\n    buildFeatures {{ compose = true }}{cpp_native_build}\n    signingConfigs {{\n        create(\"nexaRelease\") {{\n            val keystorePath = System.getenv(\"NEXA_ANDROID_KEYSTORE\")\n            if (!keystorePath.isNullOrBlank()) {{\n                storeFile = file(keystorePath)\n                storePassword = System.getenv(\"NEXA_ANDROID_STORE_PASSWORD\")\n                keyAlias = System.getenv(\"NEXA_ANDROID_KEY_ALIAS\")\n                keyPassword = System.getenv(\"NEXA_ANDROID_KEY_PASSWORD\")\n            }}\n        }}\n    }}\n    compileOptions {{ sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }}\n    buildTypes {{\n        release {{\n            signingConfig = signingConfigs.getByName(\"nexaRelease\")\n            isMinifyEnabled = true\n            isShrinkResources = true\n            proguardFiles(\n                getDefaultProguardFile(\"proguard-android-optimize.txt\"),\n                \"proguard-rules.pro\"\n            )\n        }}\n    }}\n}}\n\nkotlin {{\n    compilerOptions {{\n        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)\n    }}\n}}\n\ndependencyLocking {{\n    lockAllConfigurations()\n}}\n\ndependencies {{\n{dependencies}}}\n",
        config.android_application_id,
        config.android_target_sdk,
        config.build_number,
        config.version,
    ))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
