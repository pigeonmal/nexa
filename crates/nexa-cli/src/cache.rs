//! Disk-backed cache for deterministic native source builds.
//!
//! The cache stores only generated source and warning text. Compiler semantics
//! remain the source of truth; a cache hit is valid only when the complete
//! entry/import/plugin-IDL graph has the same content fingerprint.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process,
};

use nexa_syntax::ast::Program;

// Bump when compiler or backend semantics change without a source-graph change.
// This prevents old generated native units from surviving a generator update.
const CACHE_VERSION: &str = "build-v92";

pub(super) fn restore_warnings(entry: &Path, key: &str) -> Result<Option<Vec<String>>, String> {
    let path = cache_directory(entry).join(format!("{key}.warnings"));
    if !path.is_file() {
        return Ok(None);
    }
    let contents =
        fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(Some(contents.lines().map(str::to_owned).collect()))
}

pub(super) fn store_warnings(entry: &Path, key: &str, warnings: &[String]) -> Result<(), String> {
    let directory = cache_directory(entry);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    write_atomic(
        &directory.join(format!("{key}.warnings")),
        warnings.join("\n").as_bytes(),
    )
    .map_err(|error| format!("cache warnings: {error}"))?;
    Ok(())
}

fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension(format!("nexa-cache-{}", process::id()));
    fs::write(&temporary, contents)?;
    fs::rename(temporary, path)
}

fn cache_directory(entry: &Path) -> PathBuf {
    entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".nexa/cache")
}

#[cfg(test)]
fn key(entry: &Path, target: &str) -> Result<String, String> {
    key_with_extra_and_roots(entry, target, &[], &BTreeMap::new())
}

pub(super) fn key_with_extra_and_roots(
    entry: &Path,
    target: &str,
    extra_files: &[&Path],
    plugin_roots: &BTreeMap<String, PathBuf>,
) -> Result<String, String> {
    let mut hasher = Fnv64::default();
    hasher.write(CACHE_VERSION.as_bytes());
    hasher.write(target.as_bytes());
    let mut visited = HashSet::new();
    fingerprint_file(
        entry,
        true,
        target.starts_with("project"),
        &mut visited,
        &mut hasher,
        plugin_roots,
    )?;
    for extra in extra_files {
        hasher.write(extra.to_string_lossy().as_bytes());
        if extra.is_file() {
            fingerprint_file(extra, false, false, &mut visited, &mut hasher, plugin_roots)?;
        } else if extra.is_dir() {
            fingerprint_directory(extra, false, &mut visited, &mut hasher)?;
        } else {
            hasher.write(b"missing");
        }
    }
    for (package_id, root) in plugin_roots {
        hasher.write(package_id.as_bytes());
        fingerprint_directory(root, true, &mut visited, &mut hasher)?;
    }
    Ok(format!("{target}-{:016x}", hasher.finish()))
}

fn fingerprint_file(
    path: &Path,
    is_entry: bool,
    include_plugin_sources: bool,
    visited: &mut HashSet<PathBuf>,
    hasher: &mut Fnv64,
    plugin_roots: &BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("cannot fingerprint {}: {error}", path.display()))?;
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }
    hasher.write(canonical.to_string_lossy().as_bytes());
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("cannot fingerprint {}: {error}", canonical.display()))?;
    hasher.write(&metadata.len().to_le_bytes());
    let is_nexa_source = canonical.extension().and_then(|value| value.to_str()) == Some("nx");
    let source_text = if is_nexa_source {
        let source = fs::read_to_string(&canonical)
            .map_err(|error| format!("cannot fingerprint {}: {error}", canonical.display()))?;
        hasher.write(source.as_bytes());
        Some(source)
    } else {
        let mut file = fs::File::open(&canonical)
            .map_err(|error| format!("cannot fingerprint {}: {error}", canonical.display()))?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|error| format!("cannot fingerprint {}: {error}", canonical.display()))?;
            if count == 0 {
                break;
            }
            hasher.write(&buffer[..count]);
        }
        None
    };
    let Some(source_text) = source_text else {
        return Ok(());
    };
    let Ok(program) = nexa_syntax::parse_program(&source_text) else {
        return Ok(());
    };
    for import in &program.imports {
        let imported = canonical
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&import.path);
        fingerprint_file(
            &imported,
            false,
            include_plugin_sources,
            visited,
            hasher,
            plugin_roots,
        )?;
    }
    if is_entry {
        fingerprint_plugins(
            &canonical,
            &program,
            include_plugin_sources,
            visited,
            hasher,
            plugin_roots,
        )?;
    }
    Ok(())
}

fn fingerprint_plugins(
    entry: &Path,
    program: &Program,
    include_plugin_sources: bool,
    visited: &mut HashSet<PathBuf>,
    hasher: &mut Fnv64,
    plugin_roots: &BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    for plugin in &program.plugins {
        let declared = plugin_roots.get(&plugin.path).cloned().unwrap_or_else(|| {
            entry
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&plugin.path)
        });
        if declared.is_dir() {
            let manifest_path = declared.join("plugin.config.nx");
            fingerprint_file(
                &manifest_path,
                false,
                include_plugin_sources,
                visited,
                hasher,
                plugin_roots,
            )?;
            let manifest = nexa_plugin_idl::manifest::parse_file(&manifest_path)?;
            if let Some(source) = manifest.nexa {
                fingerprint_file(
                    &declared.join(source),
                    false,
                    include_plugin_sources,
                    visited,
                    hasher,
                    plugin_roots,
                )?;
            }
            if include_plugin_sources {
                for asset in manifest.assets {
                    let asset = asset.strip_suffix("/**").unwrap_or(&asset);
                    fingerprint_directory(
                        &declared.join(asset),
                        include_plugin_sources,
                        visited,
                        hasher,
                    )?;
                }
                for artifact in &manifest.ios.xcframeworks {
                    fingerprint_directory(
                        &declared.join(artifact),
                        include_plugin_sources,
                        visited,
                        hasher,
                    )?;
                }
                for artifact in &manifest.android.aars {
                    fingerprint_file(
                        &declared.join(artifact),
                        false,
                        include_plugin_sources,
                        visited,
                        hasher,
                        plugin_roots,
                    )?;
                }
                for resource in manifest
                    .ios
                    .resources
                    .iter()
                    .chain(manifest.ios.privacy_manifest.iter())
                    .chain(manifest.android.resources.iter())
                    .chain(manifest.android.proguard_rules.iter())
                {
                    fingerprint_file(
                        &declared.join(resource),
                        false,
                        include_plugin_sources,
                        visited,
                        hasher,
                        &BTreeMap::new(),
                    )?;
                }
                for input in manifest.cpp.sources.iter().chain(&manifest.cpp.headers) {
                    fingerprint_declared_pattern(
                        &declared.join(input),
                        include_plugin_sources,
                        visited,
                        hasher,
                    )?;
                }
            }
            if let Some(native) = manifest.native {
                let idl = declared.join(native);
                fingerprint_file(
                    &idl,
                    false,
                    include_plugin_sources,
                    visited,
                    hasher,
                    plugin_roots,
                )?;
                if include_plugin_sources {
                    fingerprint_directory(
                        &declared.join("ios/Sources"),
                        include_plugin_sources,
                        visited,
                        hasher,
                    )?;
                    fingerprint_directory(
                        &declared.join("android/src/main/kotlin"),
                        include_plugin_sources,
                        visited,
                        hasher,
                    )?;
                }
            }
            continue;
        }
        let idl = declared;
        fingerprint_file(
            &idl,
            false,
            include_plugin_sources,
            visited,
            hasher,
            plugin_roots,
        )?;
        if include_plugin_sources {
            let plugin_root = idl
                .parent()
                .ok_or_else(|| format!("invalid plugin IDL path `{}`", idl.display()))?;
            fingerprint_directory(
                &plugin_root.join("ios/Sources"),
                include_plugin_sources,
                visited,
                hasher,
            )?;
            fingerprint_directory(
                &plugin_root.join("android/src/main/kotlin"),
                include_plugin_sources,
                visited,
                hasher,
            )?;
        }
    }
    Ok(())
}

fn fingerprint_directory(
    directory: &Path,
    include_plugin_sources: bool,
    visited: &mut HashSet<PathBuf>,
    hasher: &mut Fnv64,
) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    let canonical = fs::canonicalize(directory)
        .map_err(|error| format!("cannot fingerprint {}: {error}", directory.display()))?;
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }
    let mut entries = fs::read_dir(&canonical)
        .map_err(|error| format!("cannot fingerprint {}: {error}", canonical.display()))?
        .filter_map(|entry| match entry {
            Ok(entry)
                if [".git", ".nexa", "build", "target", ".gradle"]
                    .iter()
                    .any(|excluded| entry.file_name() == *excluded)
                    && entry.path().is_dir() =>
            {
                None
            }
            Ok(entry) => Some(Ok(entry.path())),
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot fingerprint {}: {error}", canonical.display()))?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            fingerprint_directory(&path, include_plugin_sources, visited, hasher)?;
        } else {
            fingerprint_file(
                &path,
                false,
                include_plugin_sources,
                visited,
                hasher,
                &BTreeMap::new(),
            )?;
        }
    }
    Ok(())
}

fn fingerprint_declared_pattern(
    pattern: &Path,
    include_plugin_sources: bool,
    visited: &mut HashSet<PathBuf>,
    hasher: &mut Fnv64,
) -> Result<(), String> {
    if pattern.is_file() {
        return fingerprint_file(
            pattern,
            false,
            include_plugin_sources,
            visited,
            hasher,
            &BTreeMap::new(),
        );
    }
    if pattern.is_dir() {
        return fingerprint_directory(pattern, include_plugin_sources, visited, hasher);
    }
    let components = pattern.components().collect::<Vec<_>>();
    let Some(wildcard_index) = components.iter().position(|component| {
        let value = component.as_os_str().to_string_lossy();
        value.contains('*') || value.contains('?')
    }) else {
        return Ok(());
    };
    let root =
        components
            .iter()
            .take(wildcard_index)
            .fold(PathBuf::new(), |mut path, component| {
                path.push(component.as_os_str());
                path
            });
    if root.is_dir() {
        fingerprint_directory(&root, include_plugin_sources, visited, hasher)?;
    }
    Ok(())
}

#[derive(Default)]
struct Fnv64(u64);

impl Fnv64 {
    fn write(&mut self, bytes: &[u8]) {
        if self.0 == 0 {
            self.0 = 0xcbf29ce484222325;
        }
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::key;

    /// A temporary project whose directory is owned for as long as it is bound.
    struct TempProject(nexa_testkit::TempDir);

    impl TempProject {
        fn new() -> Self {
            Self(nexa_testkit::TempDir::new("nexa-cache-test"))
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    #[test]
    fn project_cache_key_tracks_native_plugin_contract_and_implementation_sources() {
        let project = TempProject::new();
        let plugin = project.path().join("video");
        let swift_sources = plugin.join("ios/Sources");
        fs::create_dir_all(&swift_sources).expect("plugin source directory should be created");
        let kotlin_sources = plugin.join("android/src/main/kotlin/dev/example/video");
        fs::create_dir_all(&kotlin_sources)
            .expect("Kotlin plugin source directory should be created");
        let xcframework = plugin.join("ios/VideoSDK.xcframework");
        fs::create_dir_all(&xcframework).expect("XCFramework directory should be created");
        fs::write(xcframework.join("Info.plist"), "initial framework metadata")
            .expect("XCFramework metadata should be written");
        let aar = plugin.join("android/libs/video-sdk.aar");
        fs::create_dir_all(aar.parent().expect("AAR has a parent"))
            .expect("AAR directory should be created");
        fs::write(&aar, b"initial AAR content").expect("AAR should be written");
        let ios_resource = plugin.join("ios/Resources/model.dat");
        let privacy_manifest = plugin.join("ios/PrivacyInfo.xcprivacy");
        let android_resource = plugin.join("android/resources/model.dat");
        let proguard_rules = plugin.join("android/rules/vendor.pro");
        for file in [
            &ios_resource,
            &privacy_manifest,
            &android_resource,
            &proguard_rules,
        ] {
            fs::create_dir_all(file.parent().expect("plugin input has a parent"))
                .expect("plugin input directory should be created");
            fs::write(file, b"initial plugin input").expect("plugin input should be written");
        }
        fs::write(
            project.path().join("main.nx"),
            "plugin \"video\" as Video\napp Demo { body { Text(\"ok\") } }\n",
        )
        .expect("entry source should be written");
        fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.nexa.video\" version: \"1.0.0\" sources { native: \"native.nxid\" } ios { xcframeworks: [\"ios/VideoSDK.xcframework\"] privacyManifest: \"ios/PrivacyInfo.xcprivacy\" resources: [\"ios/Resources/model.dat\"] } android { aars: [\"android/libs/video-sdk.aar\"] resources: [\"android/resources/model.dat\"] proguardRules: [\"android/rules/vendor.pro\"] } }\n",
        )
        .expect("plugin manifest should be written");
        fs::write(
            plugin.join("native.nxid"),
            "service VideoCatalog { fn count() -> Int32 }\n",
        )
        .expect("plugin contract should be written");
        let swift = swift_sources.join("VideoCatalog.swift");
        fs::write(&swift, "struct VideoCatalogImpl { let count = 0 }\n")
            .expect("native implementation should be written");
        let kotlin = kotlin_sources.join("VideoCatalogImpl.kt");
        fs::write(
            &kotlin,
            "package dev.example.video\nclass VideoCatalogImpl\n",
        )
        .expect("Kotlin implementation should be written");

        let entry = project.path().join("main.nx");
        let initial = key(&entry, "project-ios").expect("initial cache key should resolve");

        fs::write(xcframework.join("Info.plist"), "updated framework metadata")
            .expect("changed XCFramework metadata should be written");
        let after_xcframework =
            key(&entry, "project-ios").expect("updated XCFramework cache key should resolve");
        assert_ne!(
            initial, after_xcframework,
            "changing an XCFramework input must invalidate generated output"
        );
        let android_before_aar =
            key(&entry, "project-android").expect("Android cache key should resolve");
        fs::write(&aar, b"updated AAR content").expect("updated AAR should be written");
        let after_aar =
            key(&entry, "project-android").expect("updated AAR cache key should resolve");
        assert_ne!(
            android_before_aar, after_aar,
            "changing an AAR input must invalidate generated output"
        );
        fs::write(&privacy_manifest, b"updated privacy manifest")
            .expect("privacy manifest should be updated");
        let after_privacy =
            key(&entry, "project-ios").expect("privacy manifest cache key should resolve");
        assert_ne!(
            after_xcframework, after_privacy,
            "changing a privacy manifest must invalidate iOS output"
        );
        fs::write(&proguard_rules, b"updated shrinker rules")
            .expect("ProGuard rules should be updated");
        let after_proguard =
            key(&entry, "project-android").expect("ProGuard cache key should resolve");
        assert_ne!(
            after_aar, after_proguard,
            "changing plugin ProGuard rules must invalidate Android output"
        );

        let before_idl = key(&entry, "project-ios").expect("pre-IDL cache key should resolve");
        fs::write(
            plugin.join("native.nxid"),
            "service VideoCatalog { fn count() -> Int32\n fn refresh() }\n",
        )
        .expect("updated plugin contract should be written");
        let after_idl = key(&entry, "project-ios").expect("updated IDL cache key should resolve");
        assert_ne!(
            before_idl, after_idl,
            "changing native.nxid must invalidate output"
        );

        fs::write(&swift, "struct VideoCatalogImpl { let count = 1 }\n")
            .expect("updated implementation should be written");
        let after_source = key(&entry, "project-ios").expect("updated source key should resolve");
        assert_ne!(
            after_idl, after_source,
            "changing Swift input must invalidate output"
        );

        let android_initial =
            key(&entry, "project-android").expect("Android cache key should resolve");
        fs::write(
            &kotlin,
            "package dev.example.video\nclass VideoCatalogImpl(val count: Int)\n",
        )
        .expect("updated Kotlin implementation should be written");
        let after_kotlin =
            key(&entry, "project-android").expect("updated Android cache key should resolve");
        assert_ne!(
            android_initial, after_kotlin,
            "changing Kotlin input must invalidate Android output"
        );

        fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.nexa.video\" version: \"1.0.0\" sources { native: \"native.nxid\" } ios { frameworks: [\"AVFoundation\"] xcframeworks: [\"ios/VideoSDK.xcframework\"] privacyManifest: \"ios/PrivacyInfo.xcprivacy\" resources: [\"ios/Resources/model.dat\"] entitlements { \"aps-environment\": \"development\" } } android { aars: [\"android/libs/video-sdk.aar\"] resources: [\"android/resources/model.dat\"] proguardRules: [\"android/rules/vendor.pro\"] repositories: [\"https://maven.example.com/releases\"] } }\n",
        )
        .expect("updated plugin metadata should be written");
        let after_manifest =
            key(&entry, "project-ios").expect("updated manifest cache key should resolve");
        assert_ne!(
            after_source, after_manifest,
            "changing platform metadata must invalidate generated project output"
        );

        fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.nexa.video\" version: \"1.0.0\" sources { native: \"native.nxid\" } ios { frameworks: [\"AVFoundation\"] xcframeworks: [\"ios/VideoSDK.xcframework\"] privacyManifest: \"ios/PrivacyInfo.xcprivacy\" resources: [\"ios/Resources/model.dat\"] entitlements { \"aps-environment\": \"production\" } } android { aars: [\"android/libs/video-sdk.aar\"] resources: [\"android/resources/model.dat\"] proguardRules: [\"android/rules/vendor.pro\"] repositories: [\"https://maven.example.com/releases\"] } }\n",
        )
        .expect("changed entitlement metadata should be written");
        let after_entitlement =
            key(&entry, "project-ios").expect("updated entitlement key should resolve");
        assert_ne!(
            after_manifest, after_entitlement,
            "changing an entitlement value must invalidate generated output"
        );
    }

    #[test]
    fn project_cache_key_tracks_declared_cpp_sources_and_headers() {
        let project = TempProject::new();
        let plugin = project.path().join("signal");
        let cpp_source = plugin.join("native/Sources/Signal.cpp");
        let cpp_header = plugin.join("native/include/Signal.hpp");
        fs::create_dir_all(cpp_source.parent().expect("C++ source has a parent"))
            .expect("C++ source directory should be created");
        fs::create_dir_all(cpp_header.parent().expect("C++ header has a parent"))
            .expect("C++ header directory should be created");
        fs::write(&cpp_source, "int signal() { return 1; }\n")
            .expect("C++ source should be written");
        fs::write(&cpp_header, "#pragma once\nint signal();\n")
            .expect("C++ header should be written");
        fs::write(
            project.path().join("main.nx"),
            "plugin \"signal\" as Signal\napp Demo { body { Text(\"ok\") } }\n",
        )
        .expect("entry source should be written");
        fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.nexa.signal\" version: \"1.0.0\" sources { native: \"native.nxid\" } cpp { sources: [\"native/Sources/**\"] headers: [\"native/include/**\"] } }\n",
        )
        .expect("plugin manifest should be written");
        fs::write(
            plugin.join("native.nxid"),
            "service Signal { fn value() -> Int32 }\n",
        )
        .expect("native contract should be written");

        let entry = project.path().join("main.nx");
        let initial = key(&entry, "project-ios").expect("initial cache key should resolve");
        fs::write(&cpp_source, "int signal() { return 2; }\n")
            .expect("updated C++ source should be written");
        let after_source = key(&entry, "project-ios").expect("updated source key should resolve");
        assert_ne!(
            initial, after_source,
            "C++ sources must invalidate iOS project output"
        );

        let before_header =
            key(&entry, "project-android").expect("pre-header Android key should resolve");
        fs::write(&cpp_header, "#pragma once\nint signal(int);\n")
            .expect("updated C++ header should be written");
        let after_header =
            key(&entry, "project-android").expect("updated C++ header key should resolve");
        assert_ne!(
            before_header, after_header,
            "C++ headers must invalidate Android project output"
        );

        let before_standard = key(&entry, "project-ios").expect("baseline standard cache key");
        fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.nexa.signal\" version: \"1.0.0\" sources { native: \"native.nxid\" } cpp { standard: \"c++23\" sources: [\"native/Sources/**\"] headers: [\"native/include/**\"] } }\n",
        )
        .expect("C++ standard should be written to the manifest");
        let after_standard = key(&entry, "project-ios").expect("standard cache key should resolve");
        assert_ne!(
            before_standard, after_standard,
            "C++ language standard changes must invalidate generated projects"
        );
    }
}
