//! Generated reference tables derived from plugin package manifests.
//!
//! Two published documents restate facts that already exist in machine-readable
//! form, one copy per plugin:
//!
//! * The plugin index in `README.md`, whose platform floors were wrong for 14
//!   of 16 packages -- a hand-maintained column of version numbers that nobody
//!   rechecked when a manifest moved to a newer deployment target.
//! * The native requirement matrix in `docs/plugins.md`, which previously
//!   documented no plugin's Android permissions, iOS purpose strings,
//!   entitlements, or background modes at all, because no table existed to keep
//!   up.
//!
//! Both are rendered here from `plugin.config.nx` through
//! [`nexa_plugin_idl::manifest`], and both are gated by tests that fail when a
//! package's manifest changes without the documentation following.
//!
//! # What is generated and what is not
//!
//! Every version number, permission, purpose string, and package id in these
//! tables comes from a manifest. The plugin's display name and one-line
//! capability summary do not exist anywhere in the manifest schema, so they live
//! in [`PluginSummary`]. That is deliberate: it is a registry keyed by package
//! id, and [`crate::docs`] tests fail when a package directory is missing from
//! it. The practical effect is that adding a plugin package requires writing its
//! summary, which is the point -- a plugin cannot become undocumented by
//! accident.
//!
//! # Submodule availability
//!
//! `plugins/` is a git submodule. When it is not checked out there is nothing
//! to render from, so the renderers report that fact and the gating tests skip
//! rather than fail. A checkout without submodules therefore does not
//! silently pass an untested claim about plugin platform support; CI checks out
//! submodules recursively, and the `nexa plugin check` job validates the
//! contracts themselves.

use std::path::Path;

use nexa_plugin_idl::manifest::{self, PluginManifest};

use crate::config::ProjectConfig;

/// A plugin's published name and capability summary.
///
/// These two facts are editorial and have no manifest representation. The
/// `id` field must equal the package's manifest `id`; the gating tests enforce
/// that, so a renamed package fails here rather than publishing a stale label.
pub struct PluginSummary {
    /// Package id as declared by `plugin.config.nx`.
    pub id: &'static str,
    /// Display name used in published tables.
    pub display_name: &'static str,
    /// One-line capability summary.
    pub capabilities: &'static str,
}

/// Editorial summaries for every first-party plugin package, ordered by package
/// id so the rendered tables are stable.
///
/// The capabilities column is deliberately terse: it names what the plugin
/// does, not how it is implemented. Native engine names belong to each
/// package's own README.
pub const PLUGIN_SUMMARIES: &[PluginSummary] = &[
    PluginSummary {
        id: "dev.nexa.audio-player",
        display_name: "Audio Player",
        capabilities: "Background audio streaming, lock screen playback controls",
    },
    PluginSummary {
        id: "dev.nexa.biometrics",
        display_name: "Biometrics",
        capabilities: "Face ID, Touch ID, and Android BiometricPrompt",
    },
    PluginSummary {
        id: "dev.nexa.browser",
        display_name: "Browser",
        capabilities: "In-app Safari and Chrome Custom Tabs for OAuth flows",
    },
    PluginSummary {
        id: "dev.nexa.camera",
        display_name: "Camera",
        capabilities: "Native camera preview, photo capture, video recording",
    },
    PluginSummary {
        id: "dev.nexa.data-extractor",
        display_name: "Data Extractor",
        capabilities: "On-device Vision OCR, barcode scanning, text recognition",
    },
    PluginSummary {
        id: "dev.nexa.in-app-purchases",
        display_name: "In-App Purchases",
        capabilities: "StoreKit 2 and Google Play Billing subscriptions",
    },
    PluginSummary {
        id: "dev.nexa.mail-composer",
        display_name: "Mail Composer",
        capabilities: "Native email composition sheets with attachments",
    },
    PluginSummary {
        id: "dev.nexa.maps",
        display_name: "Maps",
        capabilities: "Interactive MapKit and Google Maps views with pins",
    },
    PluginSummary {
        id: "dev.nexa.media-picker",
        display_name: "Media Picker",
        capabilities: "System photo and video picker without privacy permissions",
    },
    PluginSummary {
        id: "dev.nexa.mmkv",
        display_name: "MMKV",
        capabilities: "High-speed memory-mapped key-value storage",
    },
    PluginSummary {
        id: "dev.nexa.notifications",
        display_name: "Notifications",
        capabilities: "Scheduled local notifications, badge counts, actions",
    },
    PluginSummary {
        id: "dev.nexa.sensors",
        display_name: "Sensors",
        capabilities: "Accelerometer, gyroscope, and magnetometer telemetry",
    },
    PluginSummary {
        id: "dev.nexa.sqlite",
        display_name: "SQLite",
        capabilities: "Reactive `observeQuery<T>`, WAL mode, cross-process sync",
    },
    PluginSummary {
        id: "dev.nexa.video-player",
        display_name: "Video Player",
        capabilities: "Hardware-accelerated HLS and MP4 video playback",
    },
    PluginSummary {
        id: "dev.nexa.websocket",
        display_name: "Websocket",
        capabilities: "Low-latency binary and text WebSockets with auto-reconnect",
    },
    PluginSummary {
        id: "dev.nexa.webview",
        display_name: "Webview",
        capabilities: "In-app browser engine with two-way JavaScript bridge",
    },
];

/// One plugin package's manifest, paired with its editorial summary.
pub struct PluginPackage {
    pub summary: &'static PluginSummary,
    pub manifest: PluginManifest,
}

/// Reads every first-party plugin package under `plugins/`.
///
/// Returns `Ok(None)` when the submodule is not checked out, which is the only
/// case where absence is expected rather than an error: a contributor working
/// in this repository alone has no plugin sources to read, and failing there
/// would make the documentation tests unusable without a recursive clone.
pub fn load_plugin_packages(root: &Path) -> Result<Option<Vec<PluginPackage>>, String> {
    let plugins = root.join("plugins");
    if !plugins.is_dir() {
        return Ok(None);
    }
    let mut packages = Vec::new();
    let entries =
        std::fs::read_dir(&plugins).map_err(|error| format!("{}: {error}", plugins.display()))?;
    let mut directories: Vec<std::path::PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    directories.sort();
    for directory in directories {
        let manifest_path = directory.join("plugin.config.nx");
        if !manifest_path.is_file() {
            // `plugins/.github` and similar non-package directories.
            continue;
        }
        let manifest = manifest::parse_file(&manifest_path)?;
        let summary = PLUGIN_SUMMARIES
            .iter()
            .find(|summary| summary.id == manifest.id)
            .ok_or_else(|| {
                format!(
                    "{}: package id `{}` has no entry in PLUGIN_SUMMARIES; \
                     add one so the plugin is documented",
                    manifest_path.display(),
                    manifest.id
                )
            })?;
        packages.push(PluginPackage { summary, manifest });
    }
    packages.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
    Ok(Some(packages))
}

/// Renders the official plugin index published in `README.md`.
///
/// Columns are the manifest package id, the editorial summary, and the two
/// platform floors, so a deployment-target bump in a manifest is reflected by
/// regenerating rather than by remembering to edit a table.
pub fn render_plugin_index(packages: &[PluginPackage]) -> String {
    let mut out = String::from(
        "| Plugin | Package ID | Key Capabilities | Supported Platforms |\n\
         |---|---|---|---|\n",
    );
    for package in packages {
        out.push_str(&format!(
            "| **{}** | `{}` | {} | {} |\n",
            package.summary.display_name,
            package.manifest.id,
            package.summary.capabilities,
            platform_floors(package),
        ));
    }
    out
}

/// Renders the native requirement matrix published in `docs/plugins.md`.
///
/// This is the table that was missing entirely: platform floors were published
/// in one place, Android permissions nowhere, and iOS purpose strings,
/// entitlements, and background modes nowhere at all, even though the plugin
/// generator requires all of them.
pub fn render_native_requirements(packages: &[PluginPackage]) -> String {
    let mut out = String::from(
        "| Plugin | iOS minimum | Android `minSdk` | Android permissions | iOS usage descriptions |\n\
         |---|---|---|---|---|\n",
    );
    for package in packages {
        out.push_str(&format!(
            "| **{}** | {} | {} | {} | {} |\n",
            package.summary.display_name,
            package
                .manifest
                .ios
                .min_version
                .as_deref()
                .map(|version| format!("`{version}`"))
                .unwrap_or_else(|| "`—`".to_owned()),
            package
                .manifest
                .android
                .min_sdk
                .map(|sdk| format!("`{sdk}` (API {sdk})"))
                .unwrap_or_else(|| "`—`".to_owned()),
            code_list(
                package
                    .manifest
                    .android
                    .permissions
                    .iter()
                    .map(|permission| permission.as_str()),
            ),
            key_list(package.manifest.ios.usage_descriptions.as_slice()),
        ));
    }
    out
}

/// Renders the per-plugin native integration facts that no table covered.
///
/// Entitlements, background modes, service declarations, and dependency
/// coordinates decide whether a plugin links and runs at all on a real device,
/// and all of them live in the manifest.
pub fn render_native_integration(packages: &[PluginPackage]) -> String {
    let mut out = String::from(
        "| Plugin | iOS entitlements | iOS background modes | App delegate | Android services | Android metadata |\n\
         |---|---|---|---|---|---|\n",
    );
    for package in packages {
        let ios = &package.manifest.ios;
        let android = &package.manifest.android;
        let services = android
            .media_playback_service
            .iter()
            .chain(android.firebase_messaging_service.iter())
            .map(|service| format!("`{service}`"))
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "| **{}** | {} | {} | {} | {} | {} |\n",
            package.summary.display_name,
            key_list(
                &ios.entitlements
                    .iter()
                    .map(|(key, value)| (key.clone(), entitlement_text(value)))
                    .collect::<Vec<_>>(),
            ),
            code_list(ios.background_modes.iter().map(|mode| mode.as_str())),
            ios.application_delegate
                .as_deref()
                .map(|name| format!("`{name}`"))
                .unwrap_or_else(|| "`—`".to_owned()),
            if services.is_empty() {
                "`—`".to_owned()
            } else {
                services.join(", ")
            },
            key_list(android.application_metadata.as_slice()),
        ));
    }
    out
}

/// Renders the native dependency coordinates each plugin contributes.
pub fn render_native_dependencies(packages: &[PluginPackage]) -> String {
    let mut out = String::from(
        "| Plugin | Swift packages | Maven dependencies | iOS frameworks | Linker flags |\n\
         |---|---|---|---|---|\n",
    );
    for package in packages {
        let ios = &package.manifest.ios;
        let android = &package.manifest.android;
        let swift_packages = ios
            .swift_packages
            .iter()
            .map(|package| format!("`{}` {}", package.url, swift_requirement(package)))
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "| **{}** | {} | {} | {} | {} |\n",
            package.summary.display_name,
            if swift_packages.is_empty() {
                "`—`".to_owned()
            } else {
                swift_packages.join(", ")
            },
            code_list(android.maven_dependencies.iter().map(|c| c.as_str())),
            code_list(ios.frameworks.iter().map(|f| f.as_str())),
            code_list(ios.linker_flags.iter().map(|f| f.as_str())),
        ));
    }
    out
}

/// One published `nexa.config.nx` key: where it lives, its type, its default,
/// and what it does.
///
/// The `default` field reads the value from a real
/// [`crate::config::ProjectConfig`] rather than restating it, so a default that
/// changes in `from_defaults` reaches the documentation on regeneration. The
/// published table had drifted here -- it claimed `stagingSuffix` defaulted to
/// "Unset" while the compiler used `"staging"`.
struct ConfigOptionDoc {
    pub section: &'static str,
    pub key: &'static str,
    pub ty: &'static str,
    pub summary: &'static str,
    /// Reads this key's default from the resolved project configuration.
    pub default: fn(&ProjectConfig) -> String,
}

/// Every `nexa.config.nx` key published in the configuration reference.
///
/// Keys whose value has no meaningful default (`permissions`, `flavors`,
/// `dependencies`, `plugins`) report `—`, which is what the table meant by
/// "Unset" before.
const CONFIG_OPTIONS: &[ConfigOptionDoc] = &[
    ConfigOptionDoc {
        section: "app",
        key: "displayName",
        ty: "`String`",
        summary: "Human-readable app name shown on the device launcher.",
        default: |config| quote(&config.display_name),
    },
    ConfigOptionDoc {
        section: "app",
        key: "version",
        ty: "`String`",
        summary: "Semantic version string (`MAJOR.MINOR.PATCH`).",
        default: |config| quote(&config.version),
    },
    ConfigOptionDoc {
        section: "app",
        key: "buildNumber",
        ty: "`Int32`",
        summary: "Monotonically increasing build integer.",
        default: |config| cell(&config.build_number.to_string()),
    },
    ConfigOptionDoc {
        section: "app",
        key: "orientation",
        ty: "`String`",
        summary: "`\"all\"`, `\"portrait\"`, or `\"portrait-phones\"`.",
        default: |config| quote(&config.orientation),
    },
    ConfigOptionDoc {
        section: "app",
        key: "deepLinks",
        ty: "`Array<String>`",
        summary: "App URL schemes and domains used by native project metadata. Incoming Nexa screen routing is not implemented.",
        default: |config| array(&config.deep_links),
    },
    ConfigOptionDoc {
        section: "app",
        key: "stagingSuffix",
        ty: "`String`",
        summary: "Application ID suffix applied by the implicit `staging` flavor.",
        default: |config| quote(&config.staging_suffix),
    },
    ConfigOptionDoc {
        section: "assets",
        key: "icon",
        ty: "`String`",
        summary: "Path to the app icon asset.",
        default: |config| optional_path(config.icon_source.as_deref()),
    },
    ConfigOptionDoc {
        section: "assets",
        key: "splash",
        ty: "`String`",
        summary: "Path to the splash image asset.",
        default: |config| optional_path(config.splash_source.as_deref()),
    },
    ConfigOptionDoc {
        section: "flavors.<name>",
        key: "suffix",
        ty: "`String`",
        summary: "Application ID suffix for that named flavor. Defaults to the flavor name when omitted.",
        default: |_| unset(),
    },
    ConfigOptionDoc {
        section: "ios",
        key: "minVersion",
        ty: "`String`",
        summary: "Minimum supported iOS deployment target.",
        default: |config| quote(&config.ios_min_version),
    },
    ConfigOptionDoc {
        section: "ios",
        key: "bundleIdentifier",
        ty: "`String`",
        summary: "Reverse-DNS iOS app ID, derived from the project name by default.",
        default: |config| quote(&config.ios_bundle_identifier),
    },
    ConfigOptionDoc {
        section: "ios",
        key: "appGroupIdentifier",
        ty: "`String`",
        summary: "App Group identifier for plugins and widgets that share storage.",
        default: |config| match &config.ios_app_group_identifier {
            Some(identifier) => quote(identifier),
            None => unset(),
        },
    },
    ConfigOptionDoc {
        section: "ios",
        key: "icon",
        ty: "`String` or `Array<String>`",
        summary: "Primary and alternate app icon asset names.",
        default: |config| paths_or_unset(config.ios_icon.as_deref(), &config.ios_alternate_icons),
    },
    ConfigOptionDoc {
        section: "ios",
        key: "arch",
        ty: "`String` or `Array<String>`",
        summary: "Requested iOS architectures. Device builds require `arm64`.",
        default: |config| optional_strings(config.ios_arch.as_deref()),
    },
    ConfigOptionDoc {
        section: "android",
        key: "minSdk",
        ty: "`Int32`",
        summary: "Minimum supported Android API level. Must be at least every used plugin's `minSdk`.",
        default: |config| cell(&config.android_min_sdk.to_string()),
    },
    ConfigOptionDoc {
        section: "android",
        key: "targetSdk",
        ty: "`Int32`",
        summary: "Target Android API level. Nexa currently requires `36`.",
        default: |config| cell(&config.android_target_sdk.to_string()),
    },
    ConfigOptionDoc {
        section: "android",
        key: "applicationId",
        ty: "`String`",
        summary: "Android package name, derived from the project name by default.",
        default: |config| quote(&config.android_application_id),
    },
    ConfigOptionDoc {
        section: "android",
        key: "icon",
        ty: "`String` or `Array<String>`",
        summary: "Primary and alternate app icon asset names.",
        default: |config| {
            paths_or_unset(
                config.android_icon.as_deref(),
                &config.android_alternate_icons,
            )
        },
    },
    ConfigOptionDoc {
        section: "android",
        key: "arch",
        ty: "`String` or `Array<String>`",
        summary: "Requested Android ABIs.",
        default: |config| optional_strings(config.android_arch.as_deref()),
    },
    ConfigOptionDoc {
        section: "android.cronet",
        key: "provider",
        ty: "`String`",
        summary: "`\"play-services\"` or `\"embedded\"` network provider.",
        default: |config| quote(config.android_cronet_provider.config_value()),
    },
    ConfigOptionDoc {
        section: "android.cronet",
        key: "diskCacheSizeMb",
        ty: "`Int32`",
        summary: "Cronet disk cache size in MiB.",
        default: |config| cell(&config.android_cronet_disk_cache_size_mb.to_string()),
    },
    ConfigOptionDoc {
        section: "permissions",
        key: "<permission>",
        ty: "`String`",
        summary: "User-facing purpose message for a native permission used by the app.",
        default: |_| unset(),
    },
    ConfigOptionDoc {
        section: "dependencies.<alias>",
        key: "`id`, `path`",
        ty: "strings",
        summary: "Local plugin package identity and path.",
        default: |_| unset(),
    },
    ConfigOptionDoc {
        section: "dependencies.<alias>",
        key: "`id`, `git`, `rev`, `package`",
        ty: "strings",
        summary: "Pinned Git plugin source and optional package subdirectory. `rev` is required with `git`.",
        default: |_| unset(),
    },
    ConfigOptionDoc {
        section: "plugins.<alias>",
        key: "scalar option values",
        ty: "string, boolean, or string array",
        summary: "Compile-time plugin options validated against the plugin contract.",
        default: |_| unset(),
    },
];

/// Renders the configuration options reference published in
/// `docs/getting-started.md`.
///
/// The defaults are resolved from
/// [`crate::config::ProjectConfig::scaffold_defaults`] for a throwaway project
/// name, so the column states what the compiler actually substitutes for an
/// omitted key rather than a remembered approximation of it.
pub fn render_config_options() -> Result<String, String> {
    let config = ProjectConfig::scaffold_defaults("NexaApp")?;
    let mut out =
        String::from("| Section | Key | Type | Default | Description |\n|---|---|---|---|---|\n");
    for option in CONFIG_OPTIONS {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            cell(option.section),
            cell(option.key),
            option.ty,
            (option.default)(&config),
            option.summary,
        ));
    }
    Ok(out)
}

/// A CLI command's published flags.
///
/// These restate the parser's accepted options rather than a prose summary, so
/// the published matrix cannot omit `--platform`, `--out`, `--staging`,
/// `--locked`, or `--arch` the way the hand-written table did. They are not
/// derived automatically -- the parsers are hand-written match statements -- so
/// `command_matrix_matches_parsers` in `tests/project_config_reference.rs` checks
/// each entry against the parser that accepts it.
struct CommandDoc {
    command: &'static str,
    flags: &'static [&'static str],
    summary: &'static str,
}

/// The commands the published CLI matrix covers, with their parsers' flags.
pub fn commands() -> impl Iterator<Item = CommandDocRef<'static>> {
    COMMANDS.iter().map(|entry| CommandDocRef {
        command: entry.command,
        flags: entry.flags,
        summary: entry.summary,
    })
}

/// A borrowed view of one [`CommandDoc`], so the table can be read from tests
/// without exposing the private struct.
pub struct CommandDocRef<'a> {
    pub command: &'a str,
    pub flags: &'a [&'a str],
    pub summary: &'a str,
}

/// Commands published in the CLI matrix.
const COMMANDS: &[CommandDoc] = &[
    CommandDoc {
        command: "`nexa create <name>`",
        flags: &["`--directory <path>`"],
        summary: "Creates the app source, configuration, and local signing template.",
    },
    CommandDoc {
        command: "`nexa check`",
        flags: &[
            "`--ios`",
            "`--android`",
            "`--locked`",
            "`--deny-warnings`",
            "`--audit`",
        ],
        summary: "Validates source, plugin dependencies, types, and platform constraints.",
    },
    CommandDoc {
        command: "`nexa fmt`",
        flags: &["`--check`"],
        summary: "Formats Nexa source files or checks whether they need formatting.",
    },
    CommandDoc {
        command: "`nexa dev`",
        flags: &[
            "`--ios`",
            "`--android`",
            "`--all`",
            "`--platform <ios\\|android\\|all>`",
            "`--arch <arch>`",
            "`--once`",
            "`--compile-only`",
            "`--flavor <name>`",
            "`--staging`",
            "`--out <directory>`",
            "`--locked`",
        ],
        summary: "Builds and launches a native app with DevRuntime hot reload.",
    },
    CommandDoc {
        command: "`nexa test`",
        flags: &[
            "`--unit-only`",
            "`--ios`",
            "`--android`",
            "`--all`",
            "`--platform <ios\\|android\\|all>`",
            "`--arch <arch>`",
            "`--flavor <name>`",
            "`--staging`",
            "`--out <directory>`",
            "`--locked`",
        ],
        summary: "Runs app tests; `--unit-only` skips native test hosts.",
    },
    CommandDoc {
        command: "`nexa build`",
        flags: &[
            "`--release`",
            "`--ipa`",
            "`--aab`",
            "`--ios`",
            "`--android`",
            "`--all`",
            "`--platform <ios\\|android\\|all>`",
            "`--arch <arch>`",
            "`--flavor <name>`",
            "`--staging`",
            "`--out <directory>`",
            "`--locked`",
        ],
        summary: "Builds a signed iOS IPA or Android AAB.",
    },
    CommandDoc {
        command: "`nexa release`",
        flags: &[
            "`--ios`",
            "`--android`",
            "`--all`",
            "`--platform <ios\\|android\\|all>`",
            "`--arch <arch>`",
            "`--flavor <name>`",
            "`--staging`",
            "`--out <directory>`",
            "`--locked`",
        ],
        summary: "Builds an iOS archive or Android AAB using platform signing credentials.",
    },
    CommandDoc {
        command: "`nexa doctor`",
        flags: &[],
        summary: "Verifies toolchain installation and environment health.",
    },
    CommandDoc {
        command: "`nexa audit <file>`",
        flags: &[
            "`--target <ios\\|android\\|all>`",
            "`--release-sizes`",
            "`--out <path>`",
        ],
        summary: "Reports reachable features, generated dependencies, and optional release-size data.",
    },
    CommandDoc {
        command: "`nexa plugin <subcommand>`",
        flags: &[
            "`init <plugin.id> --out <directory>`",
            "`check <package-directory\\|native.nxid>`",
            "`generate <package-directory\\|native.nxid> --target <swift\\|kotlin\\|cpp>`",
        ],
        summary: "Scaffolds, validates, and generates bindings for a plugin package.",
    },
];

/// Renders the primitive scalar type table published in
/// `docs/language-guide.md`.
///
/// The Nexa, Swift, and Kotlin columns come from the two backends' own type
/// functions rather than a restatement of them, which is what keeps the guide
/// honest: `Float32` reads as `Float` on Swift and `Float` on Kotlin because the
/// backend says so, not because a table said so once. `Size / Representation`
/// and `Example Literal` have no code to read, so they are
/// [`SCALAR_TYPE_NOTES`], keyed by Nexa type name.
///
/// Fails when a listed type has no spelling in one of the backends, which means
/// the list and the backends have diverged and publishing either would be wrong.
pub fn render_scalar_types() -> Result<String, String> {
    let swift: std::collections::HashMap<&str, String> = nexa_backend_swift::swift_scalar_types()
        .into_iter()
        .collect();
    let kotlin: std::collections::HashMap<&str, String> =
        nexa_backend_kotlin::kotlin_scalar_types()
            .into_iter()
            .collect();
    let mut out = String::from(
        "| Nexa Type | Swift Target | Kotlin Target | Size / Representation | Example Literal |\n\
         |---|---|---|---|---|\n",
    );
    for (name, representation, literal) in SCALAR_TYPE_NOTES {
        let Some(swift_spelling) = swift.get(name) else {
            return Err(format!(
                "`{name}` has no Swift spelling; the published type table is stale"
            ));
        };
        let Some(kotlin_spelling) = kotlin.get(name) else {
            return Err(format!(
                "`{name}` has no Kotlin spelling; the published type table is stale"
            ));
        };
        out.push_str(&format!(
            "| `{name}` | `{swift_spelling}` | `{kotlin_spelling}` | {representation} | {literal} |\n"
        ));
    }
    Ok(out)
}

/// A scalar type's representation and example literal, in published order.
///
/// The order is the published order, and it is also the completeness check: the
/// generated table has a row per entry here, so a scalar the backends support
/// but this list omits shows up as a missing row rather than a silent gap.
const SCALAR_TYPE_NOTES: &[(&str, &str, &str)] = &[
    ("String", "UTF-8 encoded text", "`\"Hello, Nexa!\"`"),
    ("Bool", "1-byte logical boolean", "`true`, `false`"),
    ("Int8", "8-bit signed integer", "`127`"),
    ("Int16", "16-bit signed integer", "`32767`"),
    (
        "Int32",
        "32-bit signed integer (default integer literal)",
        "`42`",
    ),
    ("Int64", "64-bit signed integer", "`10000000000`"),
    ("UInt8", "8-bit unsigned integer", "`255`"),
    ("UInt16", "16-bit unsigned integer", "`65535`"),
    ("UInt32", "32-bit unsigned integer", "`100000`"),
    (
        "UInt64",
        "64-bit unsigned integer",
        "`18446744073709551615`",
    ),
    ("Float32", "32-bit IEEE 754 floating point", "`3.14`"),
    (
        "Float64",
        "64-bit IEEE 754 (default decimal literal)",
        "`0.000001`",
    ),
    (
        "Bytes",
        "Owned contiguous byte buffer",
        "`Bytes.fromText(text: \"receipt\")`",
    ),
    ("Void", "Absence of value", "Return type only"),
];

/// Renders the CLI command matrix published in `docs/getting-started.md`.
pub fn render_command_matrix() -> String {
    let mut out = String::from("| Command | Key Flags | Description |\n|---|---|---|\n");
    for entry in COMMANDS {
        let flags = if entry.flags.is_empty() {
            "`—`".to_owned()
        } else {
            entry.flags.join(", ")
        };
        out.push_str(&format!(
            "| {} | {flags} | {} |\n",
            entry.command, entry.summary
        ));
    }
    out
}

/// Wraps a table cell in code formatting unless it already is.
///
/// Keys like `` `id`, `path` `` carry their own backticks per member, and
/// double-wrapping those produces the ````id`, `path```` rendering that Markdown
/// shows as literal backticks.
fn cell(value: &str) -> String {
    if value.starts_with('`') && value.ends_with('`') {
        value.to_owned()
    } else {
        format!("`{value}`")
    }
}

fn unset() -> String {
    "`—`".to_owned()
}

fn quote(value: &str) -> String {
    format!("`\"{value}\"`")
}

fn array(values: &[String]) -> String {
    if values.is_empty() {
        return "`[]`".to_owned();
    }
    let rendered: Vec<String> = values.iter().map(|value| quote(value)).collect();
    format!("`[{}]`", rendered.join(", "))
}

fn optional_strings(values: Option<&[String]>) -> String {
    match values {
        Some(values) => array(values),
        None => unset(),
    }
}

fn optional_path(value: Option<&std::path::Path>) -> String {
    match value {
        Some(path) => quote(&path.display().to_string()),
        None => unset(),
    }
}

fn paths_or_unset(primary: Option<&std::path::Path>, alternates: &[std::path::PathBuf]) -> String {
    match primary {
        Some(path) => {
            let mut rendered = vec![quote(&path.display().to_string())];
            rendered.extend(
                alternates
                    .iter()
                    .map(|alternate| quote(&alternate.display().to_string())),
            );
            format!("`{}`", rendered.join(", "))
        }
        None if alternates.is_empty() => unset(),
        None => {
            let rendered: Vec<String> = alternates
                .iter()
                .map(|alternate| quote(&alternate.display().to_string()))
                .collect();
            format!("`[{}]`", rendered.join(", "))
        }
    }
}

fn platform_floors(package: &PluginPackage) -> String {
    let ios = package
        .manifest
        .ios
        .min_version
        .as_deref()
        .map(|version| format!("iOS {version}+"))
        .unwrap_or_else(|| "iOS `—`".to_owned());
    let android = package
        .manifest
        .android
        .min_sdk
        .map(|sdk| format!("Android {sdk}+"))
        .unwrap_or_else(|| "Android `—`".to_owned());
    format!("{ios} \\| {android}")
}

fn code_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let rendered: Vec<String> = values.map(|value| format!("`{value}`")).collect();
    if rendered.is_empty() {
        "`—`".to_owned()
    } else {
        rendered.join(", ")
    }
}

fn key_list(entries: &[(String, String)]) -> String {
    if entries.is_empty() {
        return "`—`".to_owned();
    }
    entries
        .iter()
        .map(|(key, value)| format!("`{key}`: {value}"))
        .collect::<Vec<_>>()
        .join("<br>")
}

fn entitlement_text(value: &manifest::EntitlementValue) -> String {
    match value {
        manifest::EntitlementValue::String(text) => text.clone(),
        manifest::EntitlementValue::Bool(flag) => flag.to_string(),
        manifest::EntitlementValue::Strings(values) => values.join(", "),
    }
}

/// Renders a SwiftPM version requirement, which every package declares because
/// an unpinned dependency is not reproducible.
fn swift_requirement(package: &manifest::SwiftPackage) -> String {
    match &package.requirement {
        manifest::SwiftPackageRequirement::From(version) => format!("(from `{version}`)"),
        manifest::SwiftPackageRequirement::Branch(branch) => format!("(branch `{branch}`)"),
        manifest::SwiftPackageRequirement::Revision(revision) => {
            format!("(revision `{revision}`)")
        }
    }
}
