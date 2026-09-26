//! A typed model for the Xcode `project.pbxproj` object graph.
//!
//! PBX files are an old-style plist: a flat map of 96-bit identifiers to
//! untyped bodies. Writing one by string concatenation means a misplaced
//! semicolon or a forgotten quote produces a file Xcode refuses to open, and
//! the failure surfaces as an opaque `PBXParseError` at build time -- long
//! after the mistake. This module makes the graph a value instead: callers
//! construct typed objects, and each one knows how to render itself.
//!
//! Two invariants hold for everything rendered here:
//!
//! * **Deterministic.** Objects are stored in a [`BTreeMap`], and build
//!   settings in a [`BTreeMap`] too, so the same inputs always produce
//!   byte-identical output. Scaffolding must be idempotent (see `AGENTS.md`),
//!   which is impossible if iteration order depends on insertion.
//! * **Rendered once.** Bodies are built when the object is created, never
//!   patched afterwards. There is no "string surgery" phase to fall out of
//!   sync with the object graph.

use std::collections::{BTreeMap, HashMap};

/// A single build-setting value.
///
/// Settings are not all scalars: `OTHER_LDFLAGS` and `HEADER_SEARCH_PATHS`
/// are ordered lists, and Xcode treats a list and a scalar as different
/// plist types. Encoding that distinction here keeps callers from hand-writing
/// the parenthesized form, which is where unbalanced parens creep in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SettingValue {
    /// A bare token, rendered exactly as given. The caller quotes it.
    Scalar(String),
    /// An ordered list, rendered as `( a, b, c )`.
    List(Vec<String>),
}

impl SettingValue {
    /// A scalar that needs no quoting, such as `YES` or `wholemodule`.
    pub(super) fn bare(value: &str) -> Self {
        Self::Scalar(value.to_owned())
    }

    /// A scalar wrapped in double quotes, as Xcode stores almost every string.
    pub(super) fn quoted(value: &str) -> Self {
        Self::Scalar(format!("\"{}\"", escape(value)))
    }

    /// A list whose entries are each quoted.
    pub(super) fn quoted_list<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::List(
            values
                .into_iter()
                .map(|value| Self::quoted(value.as_ref()).render())
                .collect(),
        )
    }

    /// A list whose entries are already valid plist tokens.
    pub(super) fn raw_list<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::List(values.into_iter().map(Into::into).collect())
    }

    fn render(&self) -> String {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::List(values) => format!("( {} )", values.join(", ")),
        }
    }
}

/// Escapes a string for a double-quoted plist value.
fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The `buildSettings` dictionary of one `XCBuildConfiguration`.
#[derive(Debug, Default, Clone)]
pub(super) struct BuildSettings {
    entries: BTreeMap<String, SettingValue>,
}

impl BuildSettings {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Sets a scalar setting, replacing any previous value for the key.
    pub(super) fn set(&mut self, key: &str, value: SettingValue) -> &mut Self {
        self.entries.insert(key.to_owned(), value);
        self
    }

    /// Copies entries from `other`, letting this set win on conflict.
    ///
    /// A target's Release and Debug configurations must agree about the app
    /// icon, entitlements, and linker flags; building them from one shared
    /// base is what keeps them from drifting.
    pub(super) fn merge(&mut self, other: &Self) -> &mut Self {
        for (key, value) in &other.entries {
            self.entries
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
        self
    }

    /// Applies the same settings to several configurations at once.
    ///
    /// C++ interop needs its settings in all four configurations; forgetting
    /// one produces a Debug build that fails to find the bridging header.
    pub(super) fn apply_to(settings: &mut [&mut Self], key: &str, value: SettingValue) {
        for target in settings {
            target.set(key, value.clone());
        }
    }

    fn render(&self) -> String {
        self.entries
            .iter()
            .map(|(key, value)| format!("{key} = {};", value.render()))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A whole `project.pbxproj` file: objects plus the root reference.
#[derive(Default)]
pub(super) struct PbxProject {
    objects: BTreeMap<String, String>,
    root_id: String,
}

impl PbxProject {
    pub(super) fn new(root_id: String) -> Self {
        Self {
            objects: BTreeMap::new(),
            root_id,
        }
    }

    /// Adds an object under `id`.
    ///
    /// Two different bodies claiming one identifier is the one corruption
    /// that silently produces a project Xcode will open but build wrongly, so
    /// it is an error rather than a last-writer-wins overwrite.
    pub(super) fn insert(&mut self, id: String, body: String) -> Result<(), String> {
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

    pub(super) fn render(&self) -> String {
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

/// Single source of truth for every PBX object identifier in a project.
///
/// Identifiers are 24 uppercase hex characters (96 bits) derived from a
/// stable hash of a logical key such as `file:plugin:<path>` or
/// `package:<url>:<product>`. No numeric ranges are reserved, so any object
/// kind can grow without colliding with another kind. The astronomically
/// unlikely hash collision is detected through the reverse map and resolved
/// with a salted rehash, erroring only if the table cannot be resolved.
///
/// The same key always yields the same identifier, which is what makes
/// re-scaffolding an existing project idempotent instead of churning every
/// object in the file.
#[derive(Default)]
pub(super) struct PbxIdAllocator {
    key_to_id: HashMap<String, String>,
    id_to_key: HashMap<String, String>,
}

impl PbxIdAllocator {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn id(&mut self, key: &str) -> Result<String, String> {
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
                Some(_) => {
                    attempt += 1;
                    if attempt > 1024 {
                        return Err(format!("PBX object ID collision while allocating `{key}`"));
                    }
                }
            }
        }
    }

    /// Pure stable 96-bit identifier for a logical key, with no allocator
    /// state. Callers outside the allocation flow -- the widget-extension
    /// scheme generator, for instance -- use this to name the same target
    /// without sharing state.
    pub(super) fn stable_id(key: &str) -> String {
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

// ---------------------------------------------------------------------------
// Object bodies
//
// Each constructor below owns one `isa`. Callers no longer spell an `isa =`
// literal, so adding a required field to an object kind is a one-line change
// here instead of a search across every call site.
// ---------------------------------------------------------------------------

/// Renders a parenthesized object list, or nothing when empty.
fn list(ids: &[String]) -> String {
    format!("( {} )", ids.join(", "))
}

pub(super) fn project_object(
    configuration_list: &str,
    main_group: &str,
    products_group: &str,
    target: &str,
    package_references: &[String],
) -> String {
    let mut body = format!(
        "isa = PBXProject; buildConfigurationList = {configuration_list}; compatibilityVersion = \"Xcode 27.0\"; mainGroup = {main_group}; productRefGroup = {products_group}; targets = ( {target} );"
    );
    if !package_references.is_empty() {
        body.push_str(&format!(
            " packageReferences = ( {} );",
            package_references.join(", ")
        ));
    }
    body
}

/// A group in the project navigator.
///
/// `name` is set independently of `path` so a virtual Products group can
/// carry a display name with no filesystem location.
pub(super) fn group_object(children: &[String], path: Option<&str>, name: Option<&str>) -> String {
    let mut body = format!("isa = PBXGroup; children = {};", list(children));
    if let Some(name) = name {
        body.push_str(&format!(" name = {name};"));
    }
    if let Some(path) = path {
        body.push_str(&format!(" path = {path};"));
    }
    body.push_str(" sourceTree = \"<group>\";");
    body
}

/// A file reference.
///
/// `path` is a ready-to-emit plist token, not a raw string. Xcode accepts a
/// bare token for a path with no spaces and a quoted string otherwise, and
/// the generated project relies on both forms, so the decision belongs to the
/// caller. `source_tree` is likewise a token: `<group>`, `SOURCE_ROOT`,
/// `SDKROOT`, or `BUILT_PRODUCTS_DIR`.
///
/// `name` is the display name, which is set separately when the visible name
/// differs from the on-disk path -- a system framework, for instance, is
/// shown as `Foo.framework` but resolved from the SDK root.
pub(super) fn file_reference_object(
    file_type: &str,
    path: &str,
    source_tree: &str,
    name: Option<&str>,
) -> String {
    let mut body = format!("isa = PBXFileReference; lastKnownFileType = {file_type};");
    if let Some(name) = name {
        body.push_str(&format!(" name = \"{name}\";"));
    }
    body.push_str(&format!(" path = {path}; sourceTree = {source_tree};"));
    body
}

/// The application product, which uses `explicitFileType` rather than
/// `lastKnownFileType` and is excluded from the index.
pub(super) fn product_reference_object(path: &str) -> String {
    format!(
        "isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = {path}; sourceTree = BUILT_PRODUCTS_DIR;"
    )
}

/// A build file that compiles or copies `file_ref`.
pub(super) fn build_file_object(file_ref: &str) -> String {
    format!("isa = PBXBuildFile; fileRef = {file_ref};")
}

/// A build file that embeds an XCFramework.
///
/// The copy attributes are what keep a signed binary usable: `CodeSignOnCopy`
/// so the embedded framework carries the app's signature, and
/// `RemoveHeadersOnCopy` so the headers do not ship in the app bundle.
pub(super) fn embed_build_file_object(file_ref: &str) -> String {
    format!(
        "isa = PBXBuildFile; fileRef = {file_ref}; settings = {{ ATTRIBUTES = (CodeSignOnCopy, RemoveHeadersOnCopy); }};"
    )
}

/// A build file that links a SwiftPM product rather than a file reference.
pub(super) fn package_build_file_object(product_ref: &str) -> String {
    format!("isa = PBXBuildFile; productRef = {product_ref};")
}

pub(super) fn sources_phase_object(files: &[String]) -> String {
    format!("isa = PBXSourcesBuildPhase; files = {};", list(files))
}

pub(super) fn frameworks_phase_object(files: &[String]) -> String {
    format!("isa = PBXFrameworksBuildPhase; files = {};", list(files))
}

pub(super) fn resources_phase_object(files: &[String]) -> String {
    format!("isa = PBXResourcesBuildPhase; files = {};", list(files))
}

/// The "Embed Frameworks" copy phase.
///
/// `dstSubfolderSpec = 10` is the Frameworks directory, and
/// `runOnlyForDeploymentPostprocessing = 0` means the embed happens on every
/// build rather than only when archiving.
pub(super) fn embed_frameworks_phase_object(files: &[String]) -> String {
    format!(
        "isa = PBXCopyFilesBuildPhase; buildActionMask = 2147483647; dstPath = \"\"; dstSubfolderSpec = 10; files = {}; name = \"Embed Frameworks\"; runOnlyForDeploymentPostprocessing = 0;",
        list(files)
    )
}

pub(super) fn native_target_object(
    name: &str,
    configuration_list: &str,
    phases: &[String],
    product_reference: &str,
    package_products: &[String],
) -> String {
    let mut body = format!(
        "isa = PBXNativeTarget; buildConfigurationList = {configuration_list}; buildPhases = {}; name = {name}; productName = {name}; productReference = {product_reference}; productType = \"com.apple.product-type.application\";",
        list(phases)
    );
    if !package_products.is_empty() {
        body.push_str(&format!(
            " packageProductDependencies = ( {} );",
            package_products.join(", ")
        ));
    }
    body
}

pub(super) fn remote_package_reference_object(url: &str, minimum_version: &str) -> String {
    format!(
        "isa = XCRemoteSwiftPackageReference; repositoryURL = \"{url}\"; requirement = {{ kind = upToNextMajorVersion; minimumVersion = \"{minimum_version}\"; }};"
    )
}

pub(super) fn package_product_dependency_object(package: &str, product: &str) -> String {
    format!(
        "isa = XCSwiftPackageProductDependency; package = {package}; productName = \"{product}\";"
    )
}

/// The two configurations every list carries, in the order Xcode shows them.
pub(super) fn configuration_list_object(release: &str, debug: &str) -> String {
    format!(
        "isa = XCConfigurationList; buildConfigurations = ( {release}, {debug} ); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release;"
    )
}

pub(super) fn build_configuration_object(name: &str, settings: &BuildSettings) -> String {
    format!(
        "isa = XCBuildConfiguration; buildSettings = {{ {} }}; name = {name};",
        settings.render()
    )
}
