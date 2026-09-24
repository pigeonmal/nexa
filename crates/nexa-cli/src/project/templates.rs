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
    let schemes = config
        .deep_links
        .iter()
        .filter_map(|value| value.strip_suffix("://"))
        .map(|scheme| format!("<string>{}</string>", xml_escape(scheme)))
        .collect::<String>();
    let url_types = if schemes.is_empty() {
        String::new()
    } else {
        format!(
            "<key>CFBundleURLTypes</key><array><dict><key>CFBundleURLName</key><string>{}</string><key>CFBundleURLSchemes</key><array>{schemes}</array></dict></array>",
            xml_escape(&config.ios_bundle_identifier)
        )
    };
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleDisplayName</key><string>{}</string><key>CFBundleIdentifier</key><string>{}</string><key>CFBundleExecutable</key><string>{app_name}</string><key>CFBundleName</key><string>{app_name}</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>{}</string><key>CFBundleVersion</key><string>{}</string><key>LSRequiresIPhoneOS</key><true/>{splash}{dev_network}{url_types}{entries}</dict></plist>\n",
        xml_escape(&config.display_name),
        xml_escape(&config.ios_bundle_identifier),
        xml_escape(&config.version),
        config.build_number
    ))
}

pub(super) fn ios_launch_storyboard() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><document type=\"com.apple.InterfaceBuilder3.CocoaTouch.Storyboard.XIB\" version=\"3.0\" toolsVersion=\"23094\" targetRuntime=\"iOS.CocoaTouch\" useAutolayout=\"YES\" launchScreen=\"YES\" useTraitCollections=\"YES\"><scenes><scene sceneID=\"launch-scene\"><objects><viewController id=\"launch-controller\" sceneMemberID=\"viewController\"><view key=\"view\" contentMode=\"scaleToFill\" id=\"launch-view\"><rect key=\"frame\" x=\"0.0\" y=\"0.0\" width=\"393\" height=\"852\"/><subviews><imageView contentMode=\"scaleAspectFit\" image=\"NexaSplash\" translatesAutoresizingMaskIntoConstraints=\"NO\" id=\"launch-image\"><rect key=\"frame\" x=\"136\" y=\"366\" width=\"120\" height=\"120\"/></imageView></subviews><constraints><constraint firstItem=\"launch-image\" firstAttribute=\"centerX\" secondItem=\"launch-view\" secondAttribute=\"centerX\" id=\"center-x\"/><constraint firstItem=\"launch-image\" firstAttribute=\"centerY\" secondItem=\"launch-view\" secondAttribute=\"centerY\" id=\"center-y\"/><constraint firstItem=\"launch-image\" firstAttribute=\"width\" constant=\"120\" id=\"image-width\"/><constraint firstItem=\"launch-image\" firstAttribute=\"height\" constant=\"120\" id=\"image-height\"/></constraints><color key=\"backgroundColor\" systemColor=\"systemBackgroundColor\"/><viewLayoutGuide key=\"safeArea\" id=\"safe-area\"/></view></viewController><placeholder placeholderIdentifier=\"IBFirstResponder\" id=\"first-responder\" sceneMemberID=\"firstResponder\"/></objects></scene></scenes><resources><image name=\"NexaSplash\"/></resources></document>\n".to_owned()
}

pub(super) fn ios_entitlements(
    config: &ProjectConfig,
    plugins: &[nexa_ir::Plugin],
) -> Result<Option<String>, String> {
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
    let associated_domains = config
        .deep_links
        .iter()
        .filter_map(|value| value.strip_prefix("https://"))
        .map(|host| format!("applinks:{host}"))
        .collect::<Vec<_>>();
    if !associated_domains.is_empty() {
        let key = "com.apple.developer.associated-domains";
        match values.get_mut(key) {
            Some(nexa_ir::PluginEntitlementValue::Strings(existing)) => {
                for domain in associated_domains {
                    if !existing.contains(&domain) {
                        existing.push(domain);
                    }
                }
            }
            Some(_) => {
                return Err(format!(
                    "app deepLinks conflict with plugin entitlement `{key}`"
                ));
            }
            None => {
                values.insert(
                    key.to_owned(),
                    nexa_ir::PluginEntitlementValue::Strings(associated_domains),
                );
            }
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

/// Single source of truth for every PBX object identifier in the generated
/// Xcode project.
///
/// Identifiers are 24 uppercase hex characters (96 bits) derived from a stable
/// hash of a logical key such as `file:plugin:<path>` or
/// `package:<url>:<product>`. No numeric ranges are reserved: any object kind
/// can grow without colliding with another kind. The astronomically unlikely
/// hash collision is detected through the reverse map and resolved with a
/// salted rehash (erroring only if the table cannot be resolved).
#[derive(Default)]
struct PbxIdAllocator {
    key_to_id: std::collections::HashMap<String, String>,
    id_to_key: std::collections::HashMap<String, String>,
}

impl PbxIdAllocator {
    fn new() -> Self {
        Self::default()
    }

    fn id(&mut self, key: &str) -> Result<String, String> {
        if let Some(existing) = self.key_to_id.get(key) {
            return Ok(existing.clone());
        }
        let mut attempt = 0_u32;
        loop {
            let candidate = if attempt == 0 {
                Self::stable_id(key)
            } else {
                Self::stable_id(&format!("{key}\u{0}#{attempt}"))
            };
            match self.id_to_key.get(&candidate) {
                None => {
                    self.key_to_id.insert(key.to_owned(), candidate.clone());
                    self.id_to_key.insert(candidate.clone(), key.to_owned());
                    return Ok(candidate);
                }
                Some(owner) if owner == key => return Ok(candidate),
                Some(owner) => {
                    let owner = owner.clone();
                    let _ = owner;
                    attempt += 1;
                    if attempt > 1024 {
                        return Err(format!(
                            "PBX object ID collision while allocating `{key}`"
                        ));
                    }
                }
            }
        }
    }

    /// Pure stable 96-bit identifier for a logical key. Public within the
    /// module so the scheme generator can reference the same target ID as
    /// the project without sharing allocator state.
    fn stable_id(key: &str) -> String {
        let high = fnv1a64(key.as_bytes(), 0xcbf2_9ce4_8422_2325);
        let low = fnv1a64(key.as_bytes(), 0x8422_2325_cbf2_9ce4);
        format!("{high:016X}{:08X}", low & 0xffff_ffff)
    }
}

fn fnv1a64(bytes: &[u8], basis: u64) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = basis;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    // Avalanche the hash so the low 32 bits used for the tail carry entropy.
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 32;
    hash
}

/// Typed Xcode project model. Objects are collected in a map and rendered
/// exactly once; no string patching after rendering.
struct PbxProject {
    objects: std::collections::BTreeMap<String, String>,
    root_id: String,
}

impl PbxProject {
    fn new(root_id: String) -> Self {
        Self {
            objects: std::collections::BTreeMap::new(),
            root_id,
        }
    }

    fn insert(&mut self, id: String, body: String) -> Result<(), String> {
        match self.objects.get(&id) {
            Some(existing) if existing != &body => {
                return Err(format!("PBX object ID collision for object `{id}`"));
            }
            Some(_) => return Ok(()),
            None => {}
        }
        self.objects.insert(id, body);
        Ok(())
    }

    fn render(&self) -> String {
        let mut out = String::from(
            "// !$*UTF8*$!\n{\n\tarchiveVersion = 1;\n\tclasses = {};\n\tobjectVersion = 77;\n\tobjects = {",
        );
        for (id, body) in &self.objects {
            out.push_str(&format!("\n\t\t{id} = {{ {body} }};"));
        }
        out.push_str(&format!("\n\t}};\n\trootObject = {};\n}}", self.root_id));
        out
    }
}

fn render_settings(settings: &std::collections::BTreeMap<String, String>) -> String {
    settings
        .iter()
        .map(|(key, value)| format!("{key} = {value};"))
        .collect::<Vec<_>>()
        .join(" ")
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

/// Renders `project.pbxproj` in a single pass from typed objects. All PBX
/// identifiers come from one [`PbxIdAllocator`]; see its docs for the key
/// scheme.
#[allow(clippy::too_many_arguments)]
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
    let linker_flags = merge_ios_linker_flags(plugins);
    let minimum_version = minimum_ios_version(&config.ios_min_version, plugins)?;

    let mut alloc = PbxIdAllocator::new();
    // Core object identifiers, each derived from a stable logical key.
    let project_id = alloc.id("project")?;
    let main_group_id = alloc.id("group:main")?;
    let app_group_id = alloc.id("group:app")?;
    let products_group_id = alloc.id("group:products")?;
    let app_file_id = alloc.id("file:app")?;
    let generated_root_id = alloc.id("file:generated:NexaGenerated.swift")?;
    let info_id = alloc.id("file:info")?;
    let product_id = alloc.id("file:product")?;
    let app_build_id = alloc.id("build:app")?;
    let generated_root_build_id = alloc.id("build:generated:NexaGenerated.swift")?;
    let target_id = alloc.id("target:main")?;
    let sources_phase_id = alloc.id("phase:sources")?;
    let frameworks_phase_id = alloc.id("phase:frameworks")?;
    let resources_phase_id = alloc.id("phase:resources")?;
    let project_list_id = alloc.id("configList:project")?;
    let target_list_id = alloc.id("configList:target")?;
    let project_release_id = alloc.id("config:project:release")?;
    let project_debug_id = alloc.id("config:project:debug")?;
    let target_release_id = alloc.id("config:target:release")?;
    let target_debug_id = alloc.id("config:target:debug")?;

    // Per-file identifiers keyed by stable logical names. Unlike the previous
    // numeric-range scheme (plugin files at 30+index colliding with generated
    // files at 40+index once 11 plugin files existed), hash-derived keys
    // cannot collide across kinds.
    let mut generated_file_ids = Vec::new();
    let mut generated_build_ids = Vec::new();
    for name in generated_sources.iter().skip(1) {
        generated_file_ids.push(alloc.id(&format!("file:generated:{name}"))?);
        generated_build_ids.push(alloc.id(&format!("build:generated:{name}"))?);
    }
    let mut plugin_file_ids = Vec::new();
    let mut plugin_build_ids = Vec::new();
    for name in plugin_sources {
        plugin_file_ids.push(alloc.id(&format!("file:plugin:{name}"))?);
        plugin_build_ids.push(alloc.id(&format!("build:plugin:{name}"))?);
    }
    let mut cpp_file_ids = Vec::new();
    let mut cpp_build_ids = Vec::new();
    for source in cpp_sources {
        cpp_file_ids.push(alloc.id(&format!("file:cpp:{source}"))?);
        cpp_build_ids.push(alloc.id(&format!("build:cpp:{source}"))?);
    }
    let mut package_reference_ids = Vec::new();
    let mut package_product_ids = Vec::new();
    let mut package_build_ids = Vec::new();
    for package in &packages {
        package_reference_ids.push(alloc.id(&format!("package:{}", package.url))?);
        for product in &package.products {
            package_product_ids.push(alloc.id(&format!("package:{}:{product}", package.url))?);
            package_build_ids.push(alloc.id(&format!("build:product:{}:{product}", package.url))?);
        }
    }
    let mut framework_reference_ids = Vec::new();
    let mut framework_build_ids = Vec::new();
    for framework in &frameworks {
        framework_reference_ids.push(alloc.id(&format!("file:framework:{framework}"))?);
        framework_build_ids.push(alloc.id(&format!("build:framework:{framework}"))?);
    }
    let mut xcframework_reference_ids = Vec::new();
    let mut xcframework_link_ids = Vec::new();
    let mut xcframework_embed_ids = Vec::new();
    for path in xcframeworks {
        xcframework_reference_ids.push(alloc.id(&format!("file:xcframework:{path}"))?);
        xcframework_link_ids.push(alloc.id(&format!("link:xcframework:{path}"))?);
        xcframework_embed_ids.push(alloc.id(&format!("embed:xcframework:{path}"))?);
    }
    let embed_phase_id = if xcframeworks.is_empty() {
        None
    } else {
        Some(alloc.id("phase:embed")?)
    };
    let asset_reference_id = alloc.id("file:assets")?;
    let asset_build_id = alloc.id("build:assets")?;
    let resources_reference_id = alloc.id("file:plugin-resources")?;
    let resources_build_id = alloc.id("build:plugin-resources")?;
    let icon_reference_id = alloc.id("file:icon")?;
    let icon_build_id = alloc.id("build:icon")?;
    let splash_reference_id = alloc.id("file:splash")?;
    let splash_build_id = alloc.id("build:splash")?;

    let has_icon_composer = config
        .ios_icon
        .as_ref()
        .or(config.icon_source.as_ref())
        .is_some_and(|path| {
            path.extension()
                .is_some_and(|extension| extension == "icon")
        });

    let mut pbx = PbxProject::new(project_id.clone());
    let app_file = format!("{app_name}App.swift");

    // App group children, assembled once in a deterministic order.
    let mut app_children = vec![
        app_file_id.clone(),
        generated_root_id.clone(),
        info_id.clone(),
    ];
    if has_assets {
        app_children.push(asset_reference_id.clone());
    }
    app_children.extend(generated_file_ids.clone());
    app_children.extend(plugin_file_ids.clone());
    app_children.extend(xcframework_reference_ids.clone());
    app_children.extend(cpp_file_ids.clone());
    if has_plugin_resources {
        app_children.push(resources_reference_id.clone());
    }
    let mut main_children = vec![app_group_id.clone(), products_group_id.clone()];
    main_children.extend(framework_reference_ids.clone());

    // Sources phase.
    let mut source_files = vec![app_build_id.clone(), generated_root_build_id.clone()];
    source_files.extend(generated_build_ids.clone());
    source_files.extend(plugin_build_ids.clone());
    source_files.extend(cpp_build_ids.clone());

    // Frameworks phase: SwiftPM products, system frameworks, XCFramework links.
    let mut frameworks_files = package_build_ids.clone();
    frameworks_files.extend(framework_build_ids.clone());
    frameworks_files.extend(xcframework_link_ids.clone());

    // Resources phase.
    let mut resource_files = Vec::new();
    if has_assets {
        resource_files.push(asset_build_id.clone());
    }
    if has_icon_composer {
        resource_files.push(icon_build_id.clone());
    }
    if config.splash_source.is_some() {
        resource_files.push(splash_build_id.clone());
    }
    if has_plugin_resources {
        resource_files.push(resources_build_id.clone());
    }

    // Target build phases.
    let mut target_phases = vec![
        sources_phase_id.clone(),
        frameworks_phase_id.clone(),
        resources_phase_id.clone(),
    ];
    if let Some(embed_id) = &embed_phase_id {
        target_phases.push(embed_id.clone());
    }

    // Build settings, typed as maps and rendered once.
    let mut project_release = std::collections::BTreeMap::new();
    project_release.insert(
        "ALWAYS_SEARCH_USER_PATHS".to_owned(),
        "NO".to_owned(),
    );
    project_release.insert("SWIFT_VERSION".to_owned(), "6.0".to_owned());
    project_release.insert(
        "SWIFT_OPTIMIZATION_LEVEL".to_owned(),
        "\"-O\"".to_owned(),
    );
    project_release.insert(
        "SWIFT_COMPILATION_MODE".to_owned(),
        "wholemodule".to_owned(),
    );
    project_release.insert("GCC_OPTIMIZATION_LEVEL".to_owned(), "s".to_owned());
    project_release.insert("DEAD_CODE_STRIPPING".to_owned(), "YES".to_owned());
    project_release.insert(
        "IPHONEOS_DEPLOYMENT_TARGET".to_owned(),
        minimum_version.clone(),
    );
    let mut project_debug = std::collections::BTreeMap::new();
    project_debug.insert(
        "ALWAYS_SEARCH_USER_PATHS".to_owned(),
        "NO".to_owned(),
    );
    project_debug.insert("SWIFT_VERSION".to_owned(), "6.0".to_owned());
    project_debug.insert(
        "SWIFT_OPTIMIZATION_LEVEL".to_owned(),
        "\"-Onone\"".to_owned(),
    );
    project_debug.insert(
        "IPHONEOS_DEPLOYMENT_TARGET".to_owned(),
        minimum_version.clone(),
    );

    let mut target_extra = String::new();
    if (config.ios_icon.is_some() || config.icon_source.is_some()) && !has_icon_composer {
        target_extra.push_str(" ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon;");
    }
    if plugins
        .iter()
        .any(|plugin| !plugin.ios_entitlements.is_empty())
        || config
            .deep_links
            .iter()
            .any(|value| value.starts_with("https://"))
    {
        target_extra.push_str(&format!(
            " CODE_SIGN_ENTITLEMENTS = {app_name}/Nexa.entitlements;"
        ));
    }
    if !linker_flags.is_empty() {
        let flags = linker_flags
            .iter()
            .map(|flag| pbx_quote(flag))
            .collect::<Vec<_>>()
            .join(", ");
        target_extra.push_str(&format!(" OTHER_LDFLAGS = ( \"$(inherited)\", {flags} );"));
    }
    let mut target_release = std::collections::BTreeMap::new();
    target_release.insert(
        "ALWAYS_SEARCH_USER_PATHS".to_owned(),
        "NO".to_owned(),
    );
    target_release.insert(
        "PRODUCT_BUNDLE_IDENTIFIER".to_owned(),
        config.ios_bundle_identifier.clone(),
    );
    target_release.insert("PRODUCT_NAME".to_owned(), app_name.to_owned());
    target_release.insert(
        "INFOPLIST_FILE".to_owned(),
        format!("{app_name}/Info.plist"),
    );
    target_release.insert(
        "SUPPORTED_PLATFORMS".to_owned(),
        "\"iphoneos iphonesimulator\"".to_owned(),
    );
    target_release.insert("SWIFT_VERSION".to_owned(), "6.0".to_owned());
    target_release.insert(
        "SWIFT_OPTIMIZATION_LEVEL".to_owned(),
        "\"-O\"".to_owned(),
    );
    target_release.insert(
        "SWIFT_COMPILATION_MODE".to_owned(),
        "wholemodule".to_owned(),
    );
    target_release.insert("GCC_OPTIMIZATION_LEVEL".to_owned(), "s".to_owned());
    target_release.insert("DEAD_CODE_STRIPPING".to_owned(), "YES".to_owned());
    target_release.insert(
        "IPHONEOS_DEPLOYMENT_TARGET".to_owned(),
        minimum_version.clone(),
    );
    target_release.insert(
        "TARGETED_DEVICE_FAMILY".to_owned(),
        "\"1,2\"".to_owned(),
    );
    let mut target_debug = std::collections::BTreeMap::new();
    target_debug.insert(
        "ALWAYS_SEARCH_USER_PATHS".to_owned(),
        "NO".to_owned(),
    );
    target_debug.insert(
        "PRODUCT_BUNDLE_IDENTIFIER".to_owned(),
        config.ios_bundle_identifier.clone(),
    );
    target_debug.insert("PRODUCT_NAME".to_owned(), app_name.to_owned());
    target_debug.insert(
        "INFOPLIST_FILE".to_owned(),
        format!("{app_name}/Info.plist"),
    );
    target_debug.insert(
        "SUPPORTED_PLATFORMS".to_owned(),
        "\"iphoneos iphonesimulator\"".to_owned(),
    );
    target_debug.insert("SWIFT_VERSION".to_owned(), "6.0".to_owned());
    target_debug.insert(
        "SWIFT_OPTIMIZATION_LEVEL".to_owned(),
        "\"-Onone\"".to_owned(),
    );
    target_debug.insert(
        "IPHONEOS_DEPLOYMENT_TARGET".to_owned(),
        minimum_version.clone(),
    );
    target_debug.insert(
        "TARGETED_DEVICE_FAMILY".to_owned(),
        "\"1,2\"".to_owned(),
    );

    if !cpp_sources.is_empty() {
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
        let bridging = pbx_quote(&format!(
            "$(PROJECT_DIR)/{app_name}/NexaPluginCpp-Bridging-Header.h"
        ));
        // Render C++ interop settings as explicit map entries instead of a
        // string patch so every configuration (Debug and Release, project
        // and target) receives them exactly once.
        for settings in [
            &mut project_release,
            &mut project_debug,
            &mut target_release,
            &mut target_debug,
        ] {
            settings.insert(
                "CLANG_CXX_LANGUAGE_STANDARD".to_owned(),
                format!("\"c++{cpp_standard}\""),
            );
            settings.insert(
                "SWIFT_OBJC_INTEROP_MODE".to_owned(),
                "objcxx".to_owned(),
            );
            settings.insert("SWIFT_OBJC_BRIDGING_HEADER".to_owned(), bridging.clone());
            settings.insert(
                "HEADER_SEARCH_PATHS".to_owned(),
                format!("( \"$(inherited)\", {} )", include_paths.join(", ")),
            );
        }
    }

    // Core objects.
    let mut project_body = format!(
        "isa = PBXProject; buildConfigurationList = {project_list_id}; compatibilityVersion = \"Xcode 27.0\"; mainGroup = {main_group_id}; productRefGroup = {products_group_id}; targets = ( {target_id} );"
    );
    if !package_reference_ids.is_empty() {
        project_body.push_str(&format!(
            " packageReferences = ( {} );",
            package_reference_ids.join(", ")
        ));
    }
    pbx.insert(project_id, project_body)?;
    pbx.insert(
        main_group_id,
        format!(
            "isa = PBXGroup; children = ( {} ); sourceTree = \"<group>\";",
            main_children.join(", ")
        ),
    )?;
    pbx.insert(
        app_group_id,
        format!(
            "isa = PBXGroup; children = ( {} ); path = {app_name}; sourceTree = \"<group>\";",
            app_children.join(", ")
        ),
    )?;
    pbx.insert(
        products_group_id,
        format!("isa = PBXGroup; children = ( {product_id} ); name = Products; sourceTree = \"<group>\";"),
    )?;
    pbx.insert(
        app_file_id.clone(),
        format!("isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {app_file}; sourceTree = \"<group>\";"),
    )?;
    pbx.insert(
        generated_root_id.clone(),
        "isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = NexaGenerated.swift; sourceTree = \"<group>\";".to_owned(),
    )?;
    pbx.insert(
        info_id.clone(),
        "isa = PBXFileReference; lastKnownFileType = text.plist.xml; path = Info.plist; sourceTree = \"<group>\";".to_owned(),
    )?;
    pbx.insert(
        product_id.clone(),
        format!("isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = {app_name}.app; sourceTree = BUILT_PRODUCTS_DIR;"),
    )?;
    pbx.insert(
        app_build_id,
        format!("isa = PBXBuildFile; fileRef = {app_file_id};"),
    )?;
    pbx.insert(
        generated_root_build_id,
        format!("isa = PBXBuildFile; fileRef = {generated_root_id};"),
    )?;
    if has_assets {
        pbx.insert(
            asset_reference_id.clone(),
            "isa = PBXFileReference; lastKnownFileType = folder.assetcatalog; path = Assets.xcassets; sourceTree = \"<group>\";".to_owned(),
        )?;
        pbx.insert(
            asset_build_id,
            format!("isa = PBXBuildFile; fileRef = {asset_reference_id};"),
        )?;
    }
    if has_plugin_resources {
        pbx.insert(
            resources_reference_id.clone(),
            "isa = PBXFileReference; lastKnownFileType = folder; path = NexaPluginResources; sourceTree = \"<group>\";".to_owned(),
        )?;
        pbx.insert(
            resources_build_id,
            format!("isa = PBXBuildFile; fileRef = {resources_reference_id};"),
        )?;
    }
    if has_icon_composer {
        pbx.insert(
            icon_reference_id.clone(),
            format!("isa = PBXFileReference; lastKnownFileType = folder.iconcomposer.icon; path = {app_name}/AppIcon.icon; sourceTree = SOURCE_ROOT;"),
        )?;
        pbx.insert(
            icon_build_id,
            format!("isa = PBXBuildFile; fileRef = {icon_reference_id};"),
        )?;
    }
    if config.splash_source.is_some() {
        pbx.insert(
            splash_reference_id.clone(),
            format!("isa = PBXFileReference; lastKnownFileType = file.storyboard; path = {app_name}/LaunchScreen.storyboard; sourceTree = SOURCE_ROOT;"),
        )?;
        pbx.insert(
            splash_build_id,
            format!("isa = PBXBuildFile; fileRef = {splash_reference_id};"),
        )?;
    }

    // Generated unit file references and build files.
    for (name, file_id, build_id) in generated_sources
        .iter()
        .skip(1)
        .zip(generated_file_ids.iter())
        .zip(generated_build_ids.iter())
        .map(|((name, file_id), build_id)| (name, file_id, build_id))
    {
        pbx.insert(
            file_id.clone(),
            format!("isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = {name}; sourceTree = \"<group>\";"),
        )?;
        pbx.insert(
            build_id.clone(),
            format!("isa = PBXBuildFile; fileRef = {file_id};"),
        )?;
    }
    // Plugin file references and build files.
    for (name, file_id, build_id) in plugin_sources
        .iter()
        .zip(plugin_file_ids.iter())
        .zip(plugin_build_ids.iter())
        .map(|((name, file_id), build_id)| (name, file_id, build_id))
    {
        pbx.insert(
            file_id.clone(),
            format!("isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = NexaPlugins/{name}; sourceTree = \"<group>\";"),
        )?;
        pbx.insert(
            build_id.clone(),
            format!("isa = PBXBuildFile; fileRef = {file_id};"),
        )?;
    }
    // C++ file references and build files.
    for (source, file_id, build_id) in cpp_sources
        .iter()
        .zip(cpp_file_ids.iter())
        .zip(cpp_build_ids.iter())
        .map(|((source, file_id), build_id)| (source, file_id, build_id))
    {
        pbx.insert(
            file_id.clone(),
            format!(
                "isa = PBXFileReference; lastKnownFileType = sourcecode.cpp.cpp; path = {}; sourceTree = \"<group>\";",
                pbx_quote(&format!("NexaPluginCpp/{source}"))
            ),
        )?;
        pbx.insert(
            build_id.clone(),
            format!("isa = PBXBuildFile; fileRef = {file_id};"),
        )?;
    }
    // SwiftPM packages.
    for (package, reference_id) in packages.iter().zip(package_reference_ids.iter()) {
        pbx.insert(
            reference_id.clone(),
            format!(
                "isa = XCRemoteSwiftPackageReference; repositoryURL = \"{}\"; requirement = {{ kind = upToNextMajorVersion; minimumVersion = \"{}\"; }};",
                package.url, package.from
            ),
        )?;
    }
    {
        let mut product_ids = package_product_ids.iter();
        let mut build_ids = package_build_ids.iter();
        for (package, reference_id) in packages.iter().zip(package_reference_ids.iter()) {
            for product in &package.products {
                let product_id = product_ids
                    .next()
                    .cloned()
                    .unwrap_or_else(|| PbxIdAllocator::stable_id(&format!("package:{}:{product}", package.url)));
                let build_id = build_ids
                    .next()
                    .cloned()
                    .unwrap_or_else(|| PbxIdAllocator::stable_id(&format!("build:product:{}:{product}", package.url)));
                pbx.insert(
                    product_id.clone(),
                    format!("isa = XCSwiftPackageProductDependency; package = {reference_id}; productName = \"{product}\";"),
                )?;
                pbx.insert(
                    build_id.clone(),
                    format!("isa = PBXBuildFile; productRef = {product_id};"),
                )?;
            }
        }
    }
    // System frameworks.
    for (framework, file_id, build_id) in frameworks
        .iter()
        .zip(framework_reference_ids.iter())
        .zip(framework_build_ids.iter())
        .map(|((framework, file_id), build_id)| (framework, file_id, build_id))
    {
        pbx.insert(
            file_id.clone(),
            format!("isa = PBXFileReference; lastKnownFileType = wrapper.framework; name = \"{framework}.framework\"; path = System/Library/Frameworks/{framework}.framework; sourceTree = SDKROOT;"),
        )?;
        pbx.insert(
            build_id.clone(),
            format!("isa = PBXBuildFile; fileRef = {file_id};"),
        )?;
    }
    // XCFrameworks with link and embed build files.
    for (path, file_id, link_id, embed_id) in xcframeworks
        .iter()
        .zip(xcframework_reference_ids.iter())
        .zip(xcframework_link_ids.iter())
        .zip(xcframework_embed_ids.iter())
        .map(|(((path, file_id), link_id), embed_id)| (path, file_id, link_id, embed_id))
    {
        let quoted = pbx_quote(path);
        pbx.insert(
            file_id.clone(),
            format!("isa = PBXFileReference; lastKnownFileType = wrapper.xcframework; path = {quoted}; sourceTree = \"<group>\";"),
        )?;
        pbx.insert(
            link_id.clone(),
            format!("isa = PBXBuildFile; fileRef = {file_id};"),
        )?;
        pbx.insert(
            embed_id.clone(),
            format!("isa = PBXBuildFile; fileRef = {file_id}; settings = {{ ATTRIBUTES = (CodeSignOnCopy, RemoveHeadersOnCopy); }};"),
        )?;
    }
    if let Some(embed_id) = &embed_phase_id {
        pbx.insert(
            embed_id.clone(),
            format!(
                "isa = PBXCopyFilesBuildPhase; buildActionMask = 2147483647; dstPath = \"\"; dstSubfolderSpec = 10; files = ( {} ); name = \"Embed Frameworks\"; runOnlyForDeploymentPostprocessing = 0;",
                xcframework_embed_ids.join(", ")
            ),
        )?;
    }

    // Target and phases.
    let mut target_body = format!(
        "isa = PBXNativeTarget; buildConfigurationList = {target_list_id}; buildPhases = ( {} ); name = {app_name}; productName = {app_name}; productReference = {product_id}; productType = \"com.apple.product-type.application\";",
        target_phases.join(", ")
    );
    if !package_product_ids.is_empty() {
        target_body.push_str(&format!(
            " packageProductDependencies = ( {} );",
            package_product_ids.join(", ")
        ));
    }
    pbx.insert(target_id, target_body)?;
    pbx.insert(
        sources_phase_id,
        format!(
            "isa = PBXSourcesBuildPhase; files = ( {} );",
            source_files.join(", ")
        ),
    )?;
    pbx.insert(
        frameworks_phase_id,
        format!(
            "isa = PBXFrameworksBuildPhase; files = ( {} );",
            frameworks_files.join(", ")
        ),
    )?;
    pbx.insert(
        resources_phase_id,
        format!(
            "isa = PBXResourcesBuildPhase; files = ( {} );",
            resource_files.join(", ")
        ),
    )?;
    pbx.insert(
        project_list_id.clone(),
        format!(
            "isa = XCConfigurationList; buildConfigurations = ( {project_release_id}, {project_debug_id} ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release;"
        ),
    )?;
    pbx.insert(
        target_list_id.clone(),
        format!(
            "isa = XCConfigurationList; buildConfigurations = ( {target_release_id}, {target_debug_id} ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release;"
        ),
    )?;
    pbx.insert(
        project_release_id,
        format!(
            "isa = XCBuildConfiguration; buildSettings = {{ {} }}; name = Release;",
            render_settings(&project_release)
        ),
    )?;
    pbx.insert(
        project_debug_id,
        format!(
            "isa = XCBuildConfiguration; buildSettings = {{ {} }}; name = Debug;",
            render_settings(&project_debug)
        ),
    )?;
    let mut target_release_body = format!(
        "isa = XCBuildConfiguration; buildSettings = {{ {}",
        render_settings(&target_release)
    );
    if !target_extra.is_empty() {
        target_release_body.push_str(&target_extra);
    }
    target_release_body.push_str(" }; name = Release;");
    pbx.insert(target_release_id, target_release_body)?;
    let mut target_debug_body = format!(
        "isa = XCBuildConfiguration; buildSettings = {{ {}",
        render_settings(&target_debug)
    );
    if !target_extra.is_empty() {
        target_debug_body.push_str(&target_extra);
    }
    target_debug_body.push_str(" }; name = Debug;");
    pbx.insert(target_debug_id, target_debug_body)?;

    Ok(pbx.render())
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

pub(super) fn ios_scheme(app_name: &str) -> String {
    // The scheme must reference the same typed target identifier emitted into
    // project.pbxproj by `ios_project_file_with_config`.
    let target_id = PbxIdAllocator::stable_id("target:main");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Scheme LastUpgradeVersion="2700" version="1.7">
   <BuildAction parallelizeBuildables="YES" buildImplicitDependencies="YES">
      <BuildActionEntries>
         <BuildActionEntry buildForTesting="YES" buildForRunning="YES" buildForProfiling="YES" buildForArchiving="YES" buildForAnalyzing="YES">
            <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="{target_id}" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
         </BuildActionEntry>
      </BuildActionEntries>
   </BuildAction>
   <TestAction buildConfiguration="Debug" shouldUseLaunchSchemeArgsEnv="YES"/>
   <LaunchAction buildConfiguration="Debug" useCustomWorkingDirectory="NO" ignoresPersistentStateOnLaunch="NO" debugDocumentVersioning="YES" debugServiceExtension="internal" allowLocationSimulation="YES">
      <BuildableProductRunnable runnableDebuggingMode="0">
         <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="{target_id}" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
      </BuildableProductRunnable>
   </LaunchAction>
   <ProfileAction buildConfiguration="Release" shouldUseLaunchSchemeArgsEnv="YES" savedToolIdentifier="" useCustomWorkingDirectory="NO" debugDocumentVersioning="YES">
      <BuildableProductRunnable runnableDebuggingMode="0">
         <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="{target_id}" BuildableName="{app_name}.app" BlueprintName="{app_name}" ReferencedContainer="container:{app_name}.xcodeproj"/>
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
    let deep_link_filters = config
        .deep_links
        .iter()
        .map(|value| {
            let (auto_verify, data) = if let Some(host) = value.strip_prefix("https://") {
                (
                    " android:autoVerify=\"true\"",
                    format!("<data android:scheme=\"https\" android:host=\"{}\" />", xml_escape(host)),
                )
            } else {
                let scheme = value.trim_end_matches("://");
                (
                    "",
                    format!("<data android:scheme=\"{}\" />", xml_escape(scheme)),
                )
            };
            format!(
                "            <intent-filter{auto_verify}><action android:name=\"android.intent.action.VIEW\"/><category android:name=\"android.intent.category.DEFAULT\"/><category android:name=\"android.intent.category.BROWSABLE\"/>{data}</intent-filter>\n"
            )
        })
        .collect::<String>();
    format!(
        "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">\n{declared}    <application android:label=\"{}\"{icon_attribute} android:theme=\"{app_theme}\" android:enableOnBackInvokedCallback=\"true\">\n        <activity android:name=\"{package}.MainActivity\" android:exported=\"true\">\n            <intent-filter><action android:name=\"android.intent.action.MAIN\"/><category android:name=\"android.intent.category.LAUNCHER\"/></intent-filter>\n{deep_link_filters}        </activity>\n    </application>\n</manifest>\n",
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
    // Keep the network API usable when Play Services Cronet cannot install
    // its provider (for example on non-GMS devices). Dev builds preload this
    // path, while AOT builds include it only when the app uses networking.
    // CronetEngine.Builder selects the bundled provider as a fallback while
    // retaining the same NexaNetwork implementation.
    if dev_runtime || features.uses_network {
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
