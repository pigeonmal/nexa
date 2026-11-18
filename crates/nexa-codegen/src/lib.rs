use std::collections::BTreeMap;

use nexa_ir::{FontWeight, Module};

/// A resolved custom font face shared by both native backends.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontFace {
    /// The family name used by `.nx` source.
    pub family: String,
    /// The closest packaged face selected for the requested weight.
    pub weight: FontWeight,
    /// The registered PostScript name used by SwiftUI.
    pub postscript_name: String,
    /// The static Android `R.font` resource name.
    pub android_resource_name: String,
}

/// Build-time mapping from a source font family and weight to native faces.
///
/// The catalog is resolved while generating native code. Apps receive static
/// PostScript names and Android resource IDs rather than performing runtime
/// font discovery.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FontCatalog {
    faces: BTreeMap<(String, u8), FontFace>,
}

impl FontCatalog {
    /// Adds or replaces one family face. Family lookup is case-insensitive.
    pub fn insert(&mut self, face: FontFace) -> Option<FontFace> {
        let key = (
            face.family.to_ascii_lowercase(),
            font_weight_key(face.weight),
        );
        self.faces.insert(key, face)
    }

    /// Returns an exact face when available, otherwise the nearest packaged
    /// weight using the shared deterministic fallback order.
    pub fn resolve(&self, family: &str, weight: FontWeight) -> Option<&FontFace> {
        let requested = family.to_ascii_lowercase();
        let family = if self.faces.keys().any(|(name, _)| name == &requested) {
            requested
        } else {
            self.faces
                .values()
                .find(|face| face.postscript_name.eq_ignore_ascii_case(family))?
                .family
                .to_ascii_lowercase()
        };
        fallback_order(weight).iter().find_map(|candidate| {
            self.faces
                .get(&(family.clone(), font_weight_key(*candidate)))
        })
    }

    /// Returns the exact family face without applying fallback.
    pub fn resolve_exact(&self, family: &str, weight: FontWeight) -> Option<&FontFace> {
        self.faces
            .get(&(family.to_ascii_lowercase(), font_weight_key(weight)))
    }

    /// Iterates faces in stable family and weight order.
    pub fn faces(&self) -> impl Iterator<Item = &FontFace> {
        self.faces.values()
    }

    /// Iterates the distinct source-level family names in stable order.
    pub fn families(&self) -> impl Iterator<Item = &str> {
        let mut seen = std::collections::BTreeSet::new();
        self.faces.values().filter_map(move |face| {
            seen.insert(face.family.to_ascii_lowercase())
                .then_some(face.family.as_str())
        })
    }
}

fn font_weight_key(weight: FontWeight) -> u8 {
    match weight {
        FontWeight::Normal => 0,
        FontWeight::Medium => 1,
        FontWeight::Semibold => 2,
        FontWeight::Bold => 3,
    }
}

fn fallback_order(weight: FontWeight) -> &'static [FontWeight] {
    const NORMAL: &[FontWeight] = &[
        FontWeight::Normal,
        FontWeight::Medium,
        FontWeight::Semibold,
        FontWeight::Bold,
    ];
    const MEDIUM: &[FontWeight] = &[
        FontWeight::Medium,
        FontWeight::Normal,
        FontWeight::Semibold,
        FontWeight::Bold,
    ];
    const SEMIBOLD: &[FontWeight] = &[
        FontWeight::Semibold,
        FontWeight::Bold,
        FontWeight::Medium,
        FontWeight::Normal,
    ];
    const BOLD: &[FontWeight] = &[
        FontWeight::Bold,
        FontWeight::Semibold,
        FontWeight::Medium,
        FontWeight::Normal,
    ];
    match weight {
        FontWeight::Normal => NORMAL,
        FontWeight::Medium => MEDIUM,
        FontWeight::Semibold => SEMIBOLD,
        FontWeight::Bold => BOLD,
    }
}

pub mod names {
    /// Keeps generated declaration names away from platform keywords and APIs.
    pub fn screen_name(name: &str) -> String {
        format!("Nexa{}Screen", pascal_name(name, "App"))
    }

    /// Gives source-defined components stable names that cannot shadow native APIs.
    pub fn component_name(name: &str) -> String {
        format!("Nexa{name}Component")
    }

    fn pascal_name(name: &str, empty_name: &str) -> String {
        let mut pascal = String::new();
        let mut uppercase = true;
        for character in name.chars() {
            if character == '_' {
                uppercase = true;
            } else if uppercase {
                pascal.extend(character.to_uppercase());
                uppercase = false;
            } else {
                pascal.push(character);
            }
        }
        if pascal.is_empty() {
            pascal.push_str(empty_name);
        }
        pascal
    }

    pub fn state_name(name: &str) -> String {
        format!("nexa_{name}")
    }

    /// Gives source-defined pure functions stable top-level native names.
    pub fn function_name(name: &str) -> String {
        format!("nexa_fn_{name}")
    }

    /// Gives source-defined struct fields a stable name that cannot shadow a native member.
    pub fn struct_field_name(name: &str) -> String {
        format!("nexa_field_{name}")
    }

    /// Maps a shared font family name to its deterministic Android resource ID.
    pub fn font_resource_name(name: &str) -> String {
        let mut slug = String::new();
        let mut separator = false;
        for character in name.chars() {
            if character.is_ascii_alphanumeric() {
                if separator && !slug.is_empty() {
                    slug.push('_');
                }
                slug.push(character.to_ascii_lowercase());
                separator = false;
            } else {
                separator = true;
            }
        }
        while slug.ends_with('_') {
            slug.pop();
        }
        if slug.is_empty() {
            slug.push_str("font");
        } else if slug.as_bytes()[0].is_ascii_digit() {
            slug.insert_str(0, "font_");
        }
        slug
    }

    /// Resolves a source family name to the conventional face name used by
    /// font files when no project font catalog is available.
    pub fn font_face_name(name: &str, weight: nexa_ir::FontWeight) -> String {
        let Some((family, separator)) = name
            .rsplit_once('-')
            .filter(|(_, style)| style.eq_ignore_ascii_case("regular"))
            .map(|(family, _)| (family, "-"))
            .or_else(|| {
                name.rsplit_once('_')
                    .filter(|(_, style)| style.eq_ignore_ascii_case("regular"))
                    .map(|(family, _)| (family, "_"))
            })
        else {
            return name.to_owned();
        };
        let face = match weight {
            nexa_ir::FontWeight::Normal => "Regular",
            nexa_ir::FontWeight::Medium => "Medium",
            nexa_ir::FontWeight::Semibold => "Semibold",
            nexa_ir::FontWeight::Bold => "Bold",
        };
        format!("{family}{separator}{face}")
    }

    /// Gives source-defined enum declarations stable native type names.
    pub fn enum_name(name: &str) -> String {
        format!("Nexa{}", pascal_name(name, "Enum"))
    }

    /// Gives source-defined value structs stable native type names.
    pub fn struct_name(name: &str) -> String {
        format!("Nexa{}", pascal_name(name, "Struct"))
    }

    /// Gives generated widget declarations a stable framework-owned name.
    pub fn widget_name(name: &str) -> String {
        format!("Nexa{}Widget", pascal_name(name, "Widget"))
    }

    pub fn navigation_case_name(screen: nexa_ir::ScreenId) -> String {
        format!("screen{}", screen.0)
    }

    pub fn navigation_route_name(screen: nexa_ir::ScreenId) -> String {
        format!("nexa_screen_{}", screen.0)
    }

    /// Creates a deterministic, collision-resistant Android `R.string` name
    /// for source text without runtime resource lookup.
    pub fn localization_resource_name(source_text: &str) -> String {
        let mut slug = String::new();
        let mut separator = false;
        for character in source_text.chars() {
            if character.is_ascii_alphanumeric() {
                if separator && !slug.is_empty() {
                    slug.push('_');
                }
                slug.push(character.to_ascii_lowercase());
                separator = false;
            } else {
                separator = true;
            }
            if slug.len() >= 40 {
                break;
            }
        }
        while slug.ends_with('_') {
            slug.pop();
        }
        if slug.is_empty() {
            slug.push_str("key");
        }
        let hash = source_text
            .bytes()
            .fold(0xcbf29ce484222325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            });
        format!("nexa_{slug}_{hash:016x}")
    }
}

pub mod animation;
pub mod design_system;
pub mod plugin;
pub mod source_sections;
pub mod source_units;
pub mod source_writer;
pub mod value;

pub use source_units::{GeneratedSources, SourceUnit, SourceUnits};
pub use source_writer::SourceWriter;

/// Supported compilation target platforms for Nexa code generation and scaffolding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TargetPlatform {
    Ios,
    Android,
    MacOs,
    Windows,
    Web,
}

impl TargetPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Web => "web",
        }
    }
}

/// Generated source and asset bundle produced by a target backend.
#[derive(Clone, Debug, Default)]
pub struct TargetOutput {
    /// Primary compiled source code (e.g. Swift view file or Kotlin Compose file).
    pub primary_source: String,
    /// Destination relative file path for the primary source.
    pub source_filename: String,
    /// Additional auxiliary source files generated for the target (path, content).
    pub auxiliary_files: Vec<(String, String)>,
}

/// Converts typed, platform-independent IR into a native source file.
pub trait Backend {
    fn name(&self) -> &'static str;
    fn file_extension(&self) -> &'static str;
    fn generate(&self, module: &Module) -> String;
}

/// Extended backend target contract defining full platform generation capabilities.
pub trait BackendTarget: Backend {
    fn platform(&self) -> TargetPlatform;
    fn generate_target(&self, module: &Module) -> TargetOutput {
        TargetOutput {
            primary_source: self.generate(module),
            source_filename: format!("App.{}", self.file_extension()),
            auxiliary_files: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FontCatalog, FontFace, names};
    use nexa_ir::FontWeight;

    fn font_face(weight: FontWeight, postscript_name: &str, resource: &str) -> FontFace {
        FontFace {
            family: "Inter".to_owned(),
            weight,
            postscript_name: postscript_name.to_owned(),
            android_resource_name: resource.to_owned(),
        }
    }

    #[test]
    fn font_catalog_resolves_family_weights_and_postscript_aliases() {
        let mut catalog = FontCatalog::default();
        catalog.insert(font_face(
            FontWeight::Normal,
            "Inter-Regular",
            "inter_regular",
        ));
        catalog.insert(font_face(
            FontWeight::Semibold,
            "Inter-SemiBold",
            "inter_semibold",
        ));

        assert_eq!(
            catalog.resolve("inter", FontWeight::Normal),
            Some(&font_face(
                FontWeight::Normal,
                "Inter-Regular",
                "inter_regular"
            ))
        );
        assert_eq!(
            catalog.resolve("Inter-Regular", FontWeight::Semibold),
            Some(&font_face(
                FontWeight::Semibold,
                "Inter-SemiBold",
                "inter_semibold"
            ))
        );
        assert_eq!(
            catalog.resolve("Inter", FontWeight::Bold),
            Some(&font_face(
                FontWeight::Semibold,
                "Inter-SemiBold",
                "inter_semibold"
            ))
        );
        assert_eq!(catalog.families().collect::<Vec<_>>(), ["Inter"]);
    }

    #[test]
    fn font_resource_names_are_valid_android_identifiers() {
        assert_eq!(
            names::font_resource_name("Inter SemiBold"),
            "inter_semibold"
        );
        assert_eq!(names::font_resource_name("123 Icons"), "font_123_icons");
        assert_eq!(names::font_resource_name("🎨"), "font");
    }

    #[test]
    fn legacy_font_face_names_keep_single_face_postscript_names() {
        assert_eq!(
            names::font_face_name("Mazicon", FontWeight::Normal),
            "Mazicon"
        );
        assert_eq!(
            names::font_face_name("Gilroy-Regular", FontWeight::Semibold),
            "Gilroy-Semibold"
        );
    }

    #[test]
    fn declaration_names_are_stable_and_avoid_platform_collisions() {
        assert_eq!(
            names::screen_name("account_settings"),
            "NexaAccountSettingsScreen"
        );
        assert_eq!(names::enum_name("http_status"), "NexaHttpStatus");
        assert_eq!(names::struct_name("player_options"), "NexaPlayerOptions");
        assert_eq!(names::component_name("Text"), "NexaTextComponent");
        assert_eq!(names::function_name("Button"), "nexa_fn_Button");
        assert_eq!(names::struct_field_name("id"), "nexa_field_id");
    }

    #[test]
    fn empty_or_separator_only_type_names_use_a_valid_fallback() {
        assert_eq!(names::enum_name(""), "NexaEnum");
        assert_eq!(names::struct_name("___"), "NexaStruct");
        assert_eq!(names::screen_name("___"), "NexaAppScreen");
    }

    #[test]
    fn localization_resource_ids_are_stable_and_keep_distinct_keys_separate() {
        let label = names::localization_resource_name("settings.save");
        assert!(label.starts_with("nexa_settings_save_"));
        assert_eq!(label, names::localization_resource_name("settings.save"));
        assert_ne!(label, names::localization_resource_name("settings-save"));
        assert!(names::localization_resource_name("保存").starts_with("nexa_key_"));
    }
}
