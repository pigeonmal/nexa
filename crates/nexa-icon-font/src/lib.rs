//! Deterministic discovery and build support for project custom icon sets.
//!
//! SVG files are compiled to a static TrueType font at project compile time.
//! Native apps only render Unicode glyphs through their platform text stack.

mod subset;
pub use subset::subset_icon_font;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use webfont_generator::{
    ColorGlyphSelection, FontType, FormatOptions, GenerateWebfontsOptions, TtfFormatOptions,
};

const FIRST_PRIVATE_USE: u32 = 0xE000;
const LAST_PRIVATE_USE: u32 = 0xF8FF;
const FIRST_LAYER_PRIVATE_USE: u32 = 0xF0000;
const LAST_LAYER_PRIVATE_USE: u32 = 0xFFFFD;
const GENERATED_METADATA_VERSION: u32 = 1;

type IconCodepoints = BTreeMap<String, u32>;
type IconLayers = BTreeMap<String, Vec<IconGlyphLayer>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconGlyphLayer {
    pub glyph: u32,
    pub rgba: [u8; 4],
    #[serde(default)]
    pub tintable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconSet {
    pub name: String,
    /// Font face name used by SwiftUI and DevRuntime. Generated SVG sets
    /// include a deterministic content suffix to refresh edited art safely.
    pub font_family: String,
    /// File stem used for static native font resources. This is the SVG
    /// versioned family for generated fonts and the set name for prebuilt fonts.
    pub font_resource: String,
    pub font_path: PathBuf,
    pub icons: BTreeMap<String, u32>,
    /// SVG color layers mapped to extra cmap entries in the same font. Each
    /// layer is a monochrome outline plus its original SVG color, allowing
    /// native text renderers to preserve fills and apply a uniform tint.
    pub layers: BTreeMap<String, Vec<IconGlyphLayer>>,
    pub generated: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IconRegistry {
    pub sets: BTreeMap<String, IconSet>,
}

impl IconRegistry {
    /// Finds `assets/icons/<Set>/*.svg` and prebuilt icon fonts with sidecars.
    /// SVG fonts are written under `generated_root`, one file per icon set.
    pub fn load(project_root: &Path, generated_root: &Path) -> Result<Self, String> {
        validate_generated_root(generated_root)?;
        let icons_root = project_root.join("assets/icons");
        let fonts_root = project_root.join("assets/fonts");
        validate_project_asset_directory(&icons_root)?;
        validate_project_asset_directory(&fonts_root)?;
        let mut registry = Self::default();
        let mut generated_paths = BTreeSet::new();

        if let Ok(entries) = fs::read_dir(&icons_root) {
            let mut directories = entries
                .map(|entry| entry.map(|value| value.path()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("{}: {error}", icons_root.display()))?;
            directories.sort();
            for directory in directories {
                let metadata = fs::symlink_metadata(&directory)
                    .map_err(|error| format!("{}: {error}", directory.display()))?;
                if metadata.file_type().is_symlink() {
                    return Err(format!(
                        "icon set directories cannot be symlinks: {}",
                        directory.display()
                    ));
                }
                if !metadata.is_dir() {
                    return Err(format!(
                        "expected an icon set directory under {}: {}",
                        icons_root.display(),
                        directory.display()
                    ));
                }
                let name = directory
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| format!("invalid icon set name: {}", directory.display()))?
                    .to_owned();
                validate_identifier(&name)
                    .map_err(|message| format!("{}: {message}", directory.display()))?;
                let svg_files = sorted_svg_files(&directory)?;
                if svg_files.is_empty() {
                    return Err(format!(
                        "icon set `{name}` has no SVG files in {}",
                        directory.display()
                    ));
                }
                validate_unique_icon_names(&svg_files)?;
                let font_family = versioned_family_name(&name, &svg_files)?;
                let output_path = generated_root.join(format!("{font_family}.ttf"));
                let (generated_icons, layers) =
                    load_or_generate_svg_font(&font_family, &svg_files, &output_path)?;
                generated_paths.insert(output_path.clone());
                registry.insert(IconSet {
                    name,
                    font_resource: font_family.clone(),
                    font_family,
                    font_path: output_path,
                    icons: generated_icons,
                    layers,
                    generated: true,
                })?;
            }
        } else if icons_root.exists() {
            return Err(format!("cannot read {}", icons_root.display()));
        }

        if let Ok(entries) = fs::read_dir(&fonts_root) {
            let mut files = entries
                .map(|entry| entry.map(|value| value.path()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("{}: {error}", fonts_root.display()))?;
            files.sort();
            for font_path in files {
                let Some(extension) = font_path.extension().and_then(|value| value.to_str()) else {
                    continue;
                };
                if !extension.eq_ignore_ascii_case("ttf") && !extension.eq_ignore_ascii_case("otf")
                {
                    continue;
                }
                let stem = font_path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| format!("invalid icon font filename: {}", font_path.display()))?
                    .to_owned();
                validate_identifier(&stem)
                    .map_err(|message| format!("{}: {message}", font_path.display()))?;
                let sidecar = fonts_root.join(format!("{stem}.icons.json"));
                let sidecar_metadata = match fs::symlink_metadata(&sidecar) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(format!("{}: {error}", sidecar.display())),
                };
                if !sidecar_metadata.file_type().is_file() {
                    return Err(format!(
                        "icon font sidecars must be regular files: {}",
                        sidecar.display()
                    ));
                }
                let metadata = fs::symlink_metadata(&font_path)
                    .map_err(|error| format!("{}: {error}", font_path.display()))?;
                if !metadata.file_type().is_file() {
                    return Err(format!(
                        "icon fonts must be regular files: {}",
                        font_path.display()
                    ));
                }
                let icons = read_icon_map(&sidecar)?;
                let font_family = prebuilt_font_family(&font_path, &icons)?;
                registry.insert(IconSet {
                    name: stem.clone(),
                    font_family,
                    font_resource: stem,
                    font_path,
                    icons,
                    layers: BTreeMap::new(),
                    generated: false,
                })?;
            }
        } else if fonts_root.exists() {
            return Err(format!("cannot read {}", fonts_root.display()));
        }
        if generated_root.is_dir() {
            let marker = generated_root.join(".nexa-icon-fonts");
            let previous = read_generated_manifest(&marker)?;
            let active = generated_paths
                .iter()
                .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();
            for stale in previous.difference(&active) {
                let path = generated_root.join(stale);
                remove_owned_regular_file(&path)?;
                remove_owned_regular_file(&path.with_extension("icons.json"))?;
                let stem = Path::new(stale)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| format!("invalid generated icon font filename `{stale}`"))?;
                remove_owned_source_dir(&generated_root.join(".svg-sources").join(stem))?;
            }
            write_generated_manifest(&marker, &active)?;
        } else if !generated_paths.is_empty() {
            return Err(format!(
                "generated icon font directory is not a directory: {}",
                generated_root.display()
            ));
        }
        Ok(registry)
    }

    fn insert(&mut self, set: IconSet) -> Result<(), String> {
        if self.sets.contains_key(&set.name) {
            return Err(format!(
                "icon set `{}` has more than one source; use either assets/icons/{} or assets/fonts/{}.ttf with its sidecar",
                set.name, set.name, set.name
            ));
        }
        if let Some(existing) = self
            .sets
            .values()
            .find(|existing| existing.font_family == set.font_family)
        {
            return Err(format!(
                "icon sets `{}` and `{}` resolve to the same native font face `{}`; use fonts with distinct PostScript names",
                existing.name, set.name, set.font_family
            ));
        }
        self.sets.insert(set.name.clone(), set);
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct GeneratedIconMetadata {
    version: u32,
    icons: IconCodepoints,
    layers: IconLayers,
}

fn validate_generated_root(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(format!(
            "generated icon font path must be a real directory: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn validate_project_asset_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(format!(
            "custom icon asset path must be a real directory: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn load_or_generate_svg_font(
    set_name: &str,
    files: &[PathBuf],
    output_path: &Path,
) -> Result<(IconCodepoints, IconLayers), String> {
    validate_owned_output_file(output_path)?;
    let metadata_path = output_path.with_extension("icons.json");
    validate_owned_output_file(&metadata_path)?;
    let expected_icons = files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let name = file
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| format!("invalid icon filename: {}", file.display()))?;
            let codepoint = FIRST_PRIVATE_USE + index as u32;
            if codepoint > LAST_PRIVATE_USE {
                return Err(format!(
                    "icon set `{set_name}` exceeds the available private-use codepoint range"
                ));
            }
            Ok((name.to_owned(), codepoint))
        })
        .collect::<Result<IconCodepoints, String>>()?;

    let cached_metadata = output_path
        .is_file()
        .then(|| fs::read_to_string(&metadata_path).ok())
        .flatten()
        .and_then(|contents| serde_json::from_str::<GeneratedIconMetadata>(&contents).ok());
    if let Some(metadata) = cached_metadata {
        let layers_are_valid = metadata.layers.iter().all(|(name, layers)| {
            expected_icons.contains_key(name)
                && layers.iter().all(|layer| {
                    (FIRST_LAYER_PRIVATE_USE..=LAST_LAYER_PRIVATE_USE).contains(&layer.glyph)
                })
        });
        if metadata.version == GENERATED_METADATA_VERSION
            && metadata.icons == expected_icons
            && layers_are_valid
        {
            return Ok((metadata.icons, metadata.layers));
        }
    }

    let (icons, layers) = generate_svg_font(set_name, files, output_path)?;
    let metadata = GeneratedIconMetadata {
        version: GENERATED_METADATA_VERSION,
        icons: icons.clone(),
        layers: layers.clone(),
    };
    let contents = serde_json::to_vec(&metadata)
        .map_err(|error| format!("cannot serialize icon metadata for `{set_name}`: {error}"))?;
    write_generated_file(&metadata_path, &contents)?;
    Ok((icons, layers))
}

fn validate_owned_output_file(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(format!(
            "generated icon font output must be a regular file: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn remove_owned_regular_file(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            fs::remove_file(path).map_err(|error| format!("{}: {error}", path.display()))
        }
        Ok(_) => Err(format!(
            "refusing to remove a non-regular generated icon file: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn remove_owned_source_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        validate_project_asset_directory(parent)?;
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            fs::remove_dir_all(path).map_err(|error| format!("{}: {error}", path.display()))
        }
        Ok(_) => Err(format!(
            "refusing to remove a non-directory generated SVG source: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn write_generated_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    validate_owned_output_file(path)?;
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn read_generated_manifest(path: &Path) -> Result<BTreeSet<String>, String> {
    validate_owned_output_file(path)?;
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    contents
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            if is_generated_icon_font_filename(line) {
                Ok(line.to_owned())
            } else {
                Err(format!(
                    "invalid generated icon font entry `{line}` in {}",
                    path.display()
                ))
            }
        })
        .collect()
}

fn write_generated_manifest(path: &Path, files: &BTreeSet<String>) -> Result<(), String> {
    let mut contents = files.iter().cloned().collect::<Vec<_>>().join("\n");
    if !contents.is_empty() {
        contents.push('\n');
    }
    write_generated_file(path, contents.as_bytes())
}

fn is_generated_icon_font_filename(filename: &str) -> bool {
    let Some(stem) = filename.strip_suffix(".ttf") else {
        return false;
    };
    let Some((set, hash)) = stem.rsplit_once('_') else {
        return false;
    };
    let mut characters = set.bytes();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && characters.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && hash.len() == 16
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && filename.ends_with(".ttf")
}

fn versioned_family_name(set_name: &str, files: &[PathBuf]) -> Result<String, String> {
    // A stable face name changes only when the set artwork changes. This is a
    // font cache key, not a Dev IR revision or project-tree fingerprint.
    let mut hash = 0xcbf29ce484222325u64;
    for file in files {
        let name = file
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid icon filename: {}", file.display()))?;
        for byte in name
            .bytes()
            .chain(fs::read(file).map_err(|error| format!("{}: {error}", file.display()))?)
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("{set_name}_{hash:016x}"))
}

/// Reads only set and icon names for project-aware editor completions. This
/// deliberately avoids parsing SVG artwork or generating fonts while a user is
/// typing.
pub fn discover_icon_names(project_root: &Path) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut sets = BTreeMap::<String, BTreeSet<String>>::new();
    let icons_root = project_root.join("assets/icons");
    if icons_root.is_dir() {
        let mut directories = fs::read_dir(&icons_root)
            .map_err(|error| format!("{}: {error}", icons_root.display()))?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("{}: {error}", icons_root.display()))?;
        directories.sort();
        for directory in directories {
            if !directory.is_dir() {
                continue;
            }
            let Some(name) = directory.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let icons = sorted_svg_files(&directory)?
                .into_iter()
                .filter_map(|file| {
                    file.file_stem()
                        .and_then(|value| value.to_str())
                        .map(str::to_owned)
                })
                .collect::<BTreeSet<_>>();
            if !icons.is_empty() {
                sets.insert(name.to_owned(), icons);
            }
        }
    }

    let fonts_root = project_root.join("assets/fonts");
    if fonts_root.is_dir() {
        let mut sidecars = fs::read_dir(&fonts_root)
            .map_err(|error| format!("{}: {error}", fonts_root.display()))?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("{}: {error}", fonts_root.display()))?;
        sidecars.sort();
        for sidecar in sidecars {
            let Some(name) = sidecar.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(set) = name.strip_suffix(".icons.json") else {
                continue;
            };
            let matching_font = ["ttf", "otf"]
                .iter()
                .any(|extension| fonts_root.join(format!("{set}.{extension}")).is_file());
            if !matching_font {
                continue;
            }
            let names = read_icon_map(&sidecar)?
                .into_keys()
                .collect::<BTreeSet<_>>();
            sets.insert(set.to_owned(), names);
        }
    }

    Ok(sets
        .into_iter()
        .map(|(set, names)| (set, names.into_iter().collect()))
        .collect())
}

fn validate_identifier(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("icon set name cannot be empty".into());
    };
    if !(first == '_' || first.is_ascii_alphabetic())
        || !chars.all(|value| value == '_' || value.is_ascii_alphanumeric())
    {
        return Err("icon set names must be Nexa identifiers (letters, digits, and `_`; start with a letter or `_`)".into());
    }
    Ok(())
}

fn sorted_svg_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries =
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("{}: {error}", directory.display()))?
            .path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "icon assets cannot be symlinks: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            return Err(format!(
                "nested icon folders are not supported; put SVG files directly inside {}",
                directory.display()
            ));
        }
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("svg"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn validate_unique_icon_names(files: &[PathBuf]) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for file in files {
        let name = file
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid icon filename: {}", file.display()))?;
        if name.is_empty() || name.chars().any(char::is_whitespace) {
            return Err(format!(
                "icon filename must have a non-empty name without whitespace: {}",
                file.display()
            ));
        }
        if !names.insert(name.to_owned()) {
            return Err(format!(
                "duplicate icon name `{name}` in set folder {}",
                file.parent().unwrap_or_else(|| Path::new(".")).display()
            ));
        }
    }
    Ok(())
}

fn prebuilt_font_family(path: &Path, icons: &BTreeMap<String, u32>) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let face = ttf_parser::Face::parse(&bytes, 0)
        .map_err(|error| format!("{}: invalid TTF/OTF font: {error}", path.display()))?;
    for (name, codepoint) in icons {
        let character = char::from_u32(*codepoint).ok_or_else(|| {
            format!(
                "{}: icon `{name}` has invalid Unicode codepoint U+{codepoint:04X}",
                path.display()
            )
        })?;
        if face.glyph_index(character).is_none() {
            return Err(format!(
                "{}: icon `{name}` maps to U+{codepoint:04X}, which is missing from the font",
                path.display()
            ));
        }
    }
    face.names()
        .into_iter()
        .find(|name| name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME && name.is_unicode())
        .and_then(|name| name.to_string())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "{}: font is missing a Unicode PostScript name required by iOS",
                path.display()
            )
        })
}

fn generate_svg_font(
    set_name: &str,
    files: &[PathBuf],
    output_path: &Path,
) -> Result<(IconCodepoints, IconLayers), String> {
    for file in files {
        validate_svg(file)?;
    }
    let mut codepoints = HashMap::with_capacity(files.len());
    let mut icons = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        let name = file
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid icon filename: {}", file.display()))?
            .to_owned();
        let codepoint = FIRST_PRIVATE_USE + index as u32;
        if codepoint > LAST_PRIVATE_USE {
            return Err(format!(
                "icon set `{set_name}` exceeds the available private-use codepoint range"
            ));
        }
        codepoints.insert(name.clone(), codepoint);
        icons.insert(name, codepoint);
    }
    let source_cache = output_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".svg-sources");
    validate_project_asset_directory(&source_cache)?;
    let source_root = source_cache.join(set_name);
    fs::create_dir_all(&source_root)
        .map_err(|error| format!("{}: {error}", source_root.display()))?;
    let mut current_color_sentinels: BTreeMap<String, [u8; 3]> = BTreeMap::new();
    let normalized_files = files
        .iter()
        .map(|file| {
            let filename = file
                .file_name()
                .ok_or_else(|| format!("invalid icon filename: {}", file.display()))?;
            let stem = file
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| format!("invalid icon filename: {}", file.display()))?
                .to_owned();
            let normalized = source_root.join(filename);
            if let Some(sentinel) = normalize_svg_for_font(file, &normalized)? {
                current_color_sentinels.insert(stem, sentinel);
            }
            Ok(normalized)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let file_list = normalized_files
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let options = GenerateWebfontsOptions {
        color_glyphs: Some(ColorGlyphSelection::All),
        codepoints: Some(codepoints),
        css: Some(false),
        dest: output_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_string_lossy()
            .into_owned(),
        files: file_list,
        format_options: Some(FormatOptions {
            ttf: Some(TtfFormatOptions {
                copyright: None,
                description: Some(format!("Nexa icon set {set_name}")),
                ts: Some(0),
                url: None,
                version: Some("1.0".to_owned()),
            }),
            ..FormatOptions::default()
        }),
        font_name: Some(set_name.to_owned()),
        html: Some(false),
        ligature: Some(false),
        normalize: Some(true),
        start_codepoint: Some(FIRST_PRIVATE_USE),
        types: Some(vec![FontType::Ttf]),
        write_files: Some(false),
        ..GenerateWebfontsOptions::default()
    };
    let generated = webfont_generator::generate_sync(options, None)
        .map_err(|error| format!("cannot generate icon font for set `{set_name}`: {error}"))?;
    let bytes = generated
        .ttf_bytes()
        .ok_or_else(|| format!("font generator produced no TTF for set `{set_name}`"))?;
    let (bytes, layers) =
        add_color_layer_codepoints(bytes.to_vec(), &icons, &current_color_sentinels, set_name)?;
    fs::create_dir_all(output_path.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|error| format!("{}: {error}", output_path.display()))?;
    write_generated_file(output_path, &bytes)?;
    Ok((icons, layers))
}

fn add_color_layer_codepoints(
    bytes: Vec<u8>,
    icons: &BTreeMap<String, u32>,
    current_color_sentinels: &BTreeMap<String, [u8; 3]>,
    set_name: &str,
) -> Result<(Vec<u8>, IconLayers), String> {
    use ttf_parser::colr::{Paint, Painter};

    struct LayerCollector {
        current_glyph: Option<u16>,
        layers: Vec<(u16, [u8; 4])>,
        unsupported_paint: bool,
    }

    impl Painter<'_> for LayerCollector {
        fn outline_glyph(&mut self, glyph_id: ttf_parser::GlyphId) {
            self.current_glyph = Some(glyph_id.0);
        }

        fn paint(&mut self, paint: Paint<'_>) {
            match (self.current_glyph.take(), paint) {
                (Some(glyph), Paint::Solid(color)) => self
                    .layers
                    .push((glyph, [color.red, color.green, color.blue, color.alpha])),
                (
                    _,
                    Paint::LinearGradient(_) | Paint::RadialGradient(_) | Paint::SweepGradient(_),
                ) => {
                    self.unsupported_paint = true;
                }
                (None, _) => self.unsupported_paint = true,
            }
        }

        fn push_clip(&mut self) {}
        fn push_clip_box(&mut self, _clipbox: ttf_parser::colr::ClipBox) {}
        fn pop_clip(&mut self) {}
        fn push_layer(&mut self, _mode: ttf_parser::colr::CompositeMode) {}
        fn pop_layer(&mut self) {}
        fn push_transform(&mut self, _transform: ttf_parser::Transform) {}
        fn pop_transform(&mut self) {}
    }

    let (mappings, metric_overrides, layers_by_name) = {
        let face = ttf_parser::Face::parse(&bytes, 0)
            .map_err(|error| format!("generated icon font for `{set_name}` is invalid: {error}"))?;
        let mut mappings = Vec::new();
        let mut metric_overrides = Vec::new();
        let mut layers_by_name = BTreeMap::new();
        let mut next_layer_codepoint = FIRST_LAYER_PRIVATE_USE;
        for (name, codepoint) in icons {
            let character = char::from_u32(*codepoint).ok_or_else(|| {
                format!(
                    "generated icon `{set_name}:{name}` has invalid codepoint U+{codepoint:04X}"
                )
            })?;
            let Some(glyph) = face.glyph_index(character) else {
                return Err(format!(
                    "generated icon font for `{set_name}` is missing `{name}` at U+{codepoint:04X}"
                ));
            };
            mappings.push((*codepoint, glyph.0));
            let base_advance = face.glyph_hor_advance(glyph).unwrap_or(0);
            let mut collector = LayerCollector {
                current_glyph: None,
                layers: Vec::new(),
                unsupported_paint: false,
            };
            let _ = face.paint_color_glyph(
                glyph,
                0,
                ttf_parser::RgbaColor::new(0, 0, 0, 255),
                &mut collector,
            );
            if collector.unsupported_paint {
                return Err(format!(
                    "generated icon font for `{set_name}` contains unsupported COLR paint in `{name}`"
                ));
            }
            if collector.layers.is_empty() {
                continue;
            }
            let sentinel = current_color_sentinels.get(name).copied();
            let mut icon_layers = Vec::with_capacity(collector.layers.len());
            for (layer_glyph, rgba) in collector.layers {
                if next_layer_codepoint > LAST_LAYER_PRIVATE_USE {
                    return Err(format!(
                        "icon set `{set_name}` exceeds the available color-layer codepoint range"
                    ));
                }
                mappings.push((next_layer_codepoint, layer_glyph));
                metric_overrides.push((layer_glyph, base_advance));
                let tintable = sentinel.is_some_and(|rgb| rgba[..3] == rgb);
                icon_layers.push(IconGlyphLayer {
                    glyph: next_layer_codepoint,
                    rgba: if tintable { [0, 0, 0, rgba[3]] } else { rgba },
                    tintable,
                });
                next_layer_codepoint += 1;
            }
            layers_by_name.insert(name.clone(), icon_layers);
        }
        (mappings, metric_overrides, layers_by_name)
    };

    Ok((
        rewrite_font_cmap(bytes, &mappings, &metric_overrides, set_name)?,
        layers_by_name,
    ))
}

fn rewrite_font_cmap(
    bytes: Vec<u8>,
    mappings: &[(u32, u16)],
    metric_overrides: &[(u16, u16)],
    set_name: &str,
) -> Result<Vec<u8>, String> {
    #[derive(Clone)]
    struct Table {
        tag: [u8; 4],
        data: Vec<u8>,
    }

    fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
        Some(u16::from_be_bytes(
            bytes.get(offset..offset + 2)?.try_into().ok()?,
        ))
    }

    fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
        Some(u32::from_be_bytes(
            bytes.get(offset..offset + 4)?.try_into().ok()?,
        ))
    }

    fn checksum(bytes: &[u8]) -> u32 {
        bytes.chunks(4).fold(0u32, |sum, word| {
            let mut padded = [0u8; 4];
            padded[..word.len()].copy_from_slice(word);
            sum.wrapping_add(u32::from_be_bytes(padded))
        })
    }

    fn put_u16(target: &mut Vec<u8>, value: u16) {
        target.extend_from_slice(&value.to_be_bytes());
    }

    fn put_u32(target: &mut Vec<u8>, value: u32) {
        target.extend_from_slice(&value.to_be_bytes());
    }

    let error =
        || format!("generated icon font for `{set_name}` has an invalid SFNT table directory");
    if bytes.len() < 12 {
        return Err(error());
    }
    let num_tables = usize::from(read_u16(&bytes, 4).ok_or_else(error)?);
    let directory_end = 12usize
        .checked_add(num_tables.checked_mul(16).ok_or_else(error)?)
        .ok_or_else(error)?;
    if directory_end > bytes.len() {
        return Err(error());
    }
    let mut tables = Vec::with_capacity(num_tables);
    for index in 0..num_tables {
        let entry = 12 + index * 16;
        let tag: [u8; 4] = bytes[entry..entry + 4].try_into().map_err(|_| error())?;
        let offset = read_u32(&bytes, entry + 8).ok_or_else(error)? as usize;
        let length = read_u32(&bytes, entry + 12).ok_or_else(error)? as usize;
        let end = offset.checked_add(length).ok_or_else(error)?;
        let data = bytes.get(offset..end).ok_or_else(error)?.to_vec();
        tables.push(Table { tag, data });
    }
    let cmap = tables
        .iter_mut()
        .find(|table| &table.tag == b"cmap")
        .ok_or_else(error)?;
    let mut character_glyphs = mappings.to_vec();
    character_glyphs.sort_unstable_by_key(|(codepoint, _)| *codepoint);
    if character_glyphs
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0)
    {
        return Err(format!(
            "generated icon font for `{set_name}` has duplicate cmap codepoints"
        ));
    }
    let groups = character_glyphs.into_iter().fold(
        Vec::<(u32, u32, u32)>::new(),
        |mut groups, (codepoint, glyph)| {
            if let Some(last) = groups.last_mut()
                && last.1.checked_add(1) == Some(codepoint)
                && last.2.checked_add(1) == Some(u32::from(glyph))
            {
                last.1 = codepoint;
            } else {
                groups.push((codepoint, codepoint, u32::from(glyph)));
            }
            groups
        },
    );
    let group_count = u32::try_from(groups.len())
        .map_err(|_| format!("icon set `{set_name}` has too many cmap ranges"))?;
    let subtable_length = 16usize
        .checked_add(groups.len().checked_mul(12).ok_or_else(error)?)
        .and_then(|length| u32::try_from(length).ok())
        .ok_or_else(|| format!("icon set `{set_name}` cmap is too large"))?;
    let mut subtable = Vec::with_capacity(subtable_length as usize);
    put_u16(&mut subtable, 12);
    put_u16(&mut subtable, 0);
    put_u32(&mut subtable, subtable_length);
    put_u32(&mut subtable, 0);
    put_u32(&mut subtable, group_count);
    for (start, end, start_glyph) in groups {
        put_u32(&mut subtable, start);
        put_u32(&mut subtable, end);
        put_u32(&mut subtable, start_glyph);
    }
    let mut cmap_data = Vec::with_capacity(12 + subtable.len());
    put_u16(&mut cmap_data, 0);
    put_u16(&mut cmap_data, 1);
    put_u16(&mut cmap_data, 3);
    put_u16(&mut cmap_data, 10);
    put_u32(&mut cmap_data, 12);
    cmap_data.extend_from_slice(&subtable);
    cmap.data = cmap_data;

    let glyph_count = tables
        .iter()
        .find(|table| &table.tag == b"maxp")
        .and_then(|table| read_u16(&table.data, 4))
        .ok_or_else(error)?;
    let old_metric_count = tables
        .iter()
        .find(|table| &table.tag == b"hhea")
        .and_then(|table| read_u16(&table.data, 34))
        .ok_or_else(error)?;
    if old_metric_count == 0 || old_metric_count > glyph_count {
        return Err(error());
    }
    let hmtx = tables
        .iter_mut()
        .find(|table| &table.tag == b"hmtx")
        .ok_or_else(error)?;
    let long_metrics_bytes = usize::from(old_metric_count) * 4;
    let trailing_metrics_bytes = usize::from(glyph_count - old_metric_count) * 2;
    if hmtx.data.len() < long_metrics_bytes + trailing_metrics_bytes {
        return Err(error());
    }
    let shared_advance = read_u16(&hmtx.data, long_metrics_bytes - 4).ok_or_else(error)?;
    let mut metrics = Vec::with_capacity(usize::from(glyph_count));
    for glyph_id in 0..glyph_count {
        let (advance, bearing) = if glyph_id < old_metric_count {
            let offset = usize::from(glyph_id) * 4;
            (
                read_u16(&hmtx.data, offset).ok_or_else(error)?,
                read_u16(&hmtx.data, offset + 2).ok_or_else(error)?,
            )
        } else {
            let bearing_offset = long_metrics_bytes + usize::from(glyph_id - old_metric_count) * 2;
            (
                shared_advance,
                read_u16(&hmtx.data, bearing_offset).ok_or_else(error)?,
            )
        };
        metrics.push((advance, bearing));
    }
    for (glyph_id, advance) in metric_overrides {
        let Some(metric) = metrics.get_mut(usize::from(*glyph_id)) else {
            return Err(error());
        };
        metric.0 = *advance;
    }
    let mut hmtx_data = Vec::with_capacity(metrics.len() * 4);
    for (advance, bearing) in metrics {
        put_u16(&mut hmtx_data, advance);
        put_u16(&mut hmtx_data, bearing);
    }
    hmtx.data = hmtx_data;
    let hhea = tables
        .iter_mut()
        .find(|table| &table.tag == b"hhea")
        .ok_or_else(error)?;
    hhea.data[34..36].copy_from_slice(&glyph_count.to_be_bytes());

    let head = tables
        .iter_mut()
        .find(|table| &table.tag == b"head")
        .ok_or_else(error)?;
    if head.data.len() < 12 {
        return Err(error());
    }
    head.data[8..12].fill(0);

    let mut output = Vec::with_capacity(bytes.len() + 1024);
    output.extend_from_slice(&bytes[..12]);
    let directory_start = output.len();
    output.resize(directory_start + num_tables * 16, 0);
    let mut head_offset = None;
    for (index, table) in tables.iter().enumerate() {
        while output.len() % 4 != 0 {
            output.push(0);
        }
        let offset = u32::try_from(output.len()).map_err(|_| error())?;
        let length = u32::try_from(table.data.len()).map_err(|_| error())?;
        let entry = directory_start + index * 16;
        output[entry..entry + 4].copy_from_slice(&table.tag);
        output[entry + 4..entry + 8].copy_from_slice(&checksum(&table.data).to_be_bytes());
        output[entry + 8..entry + 12].copy_from_slice(&offset.to_be_bytes());
        output[entry + 12..entry + 16].copy_from_slice(&length.to_be_bytes());
        if &table.tag == b"head" {
            head_offset = Some(offset as usize);
        }
        output.extend_from_slice(&table.data);
    }
    while output.len() % 4 != 0 {
        output.push(0);
    }
    let head_offset = head_offset.ok_or_else(error)?;
    let adjustment = 0xB1B0AFBAu32.wrapping_sub(checksum(&output));
    output[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
    ttf_parser::Face::parse(&output, 0).map_err(|parse_error| {
        format!("rewritten icon font for `{set_name}` is invalid: {parse_error}")
    })?;
    Ok(output)
}

fn validate_svg(file: &Path) -> Result<(), String> {
    let source =
        fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))?;
    let document = roxmltree::Document::parse(&source)
        .map_err(|error| format!("{}: invalid SVG XML: {error}", file.display()))?;
    let supported = [
        "svg", "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "title",
        "desc", "defs", "metadata",
    ];
    for node in document.descendants().filter(roxmltree::Node::is_element) {
        let name = node.tag_name().name();
        if !supported.contains(&name) {
            return Err(format!(
                "{}: unsupported SVG element <{name}>; custom icons support paths, basic shapes, groups, transforms, solid fills, and strokes",
                file.display()
            ));
        }
        for attribute in node.attributes() {
            if matches!(
                attribute.name(),
                "filter"
                    | "mask"
                    | "pattern"
                    | "clip-path"
                    | "marker-start"
                    | "marker-mid"
                    | "marker-end"
                    | "paint-order"
            ) {
                return Err(format!(
                    "{}: unsupported SVG attribute `{}` on <{name}>",
                    file.display(),
                    attribute.name()
                ));
            }
            if matches!(attribute.name(), "fill" | "stroke")
                && (attribute.value().starts_with("url(")
                    || attribute.value().starts_with("context-"))
            {
                return Err(format!(
                    "{}: unsupported non-solid `{}` paint on <{name}>",
                    file.display(),
                    attribute.name()
                ));
            }
        }
    }
    usvg::Tree::from_str(&source, &usvg::Options::default())
        .map_err(|error| format!("{}: invalid or unsupported SVG: {error}", file.display()))?;
    Ok(())
}

fn normalize_svg_for_font(
    source_path: &Path,
    output_path: &Path,
) -> Result<Option<[u8; 3]>, String> {
    use std::fmt::Write as _;

    fn paint_color(paint: &usvg::Paint, file: &Path) -> Result<usvg::Color, String> {
        match paint {
            usvg::Paint::Color(color) => Ok(*color),
            _ => Err(format!(
                "{}: unsupported non-solid SVG paint; custom icons support solid fills and strokes",
                file.display()
            )),
        }
    }

    fn append_path(
        data: &usvg::tiny_skia_path::Path,
        transform: usvg::Transform,
        color: usvg::Color,
        opacity: f32,
        even_odd: bool,
        file: &Path,
        output: &mut String,
    ) -> Result<(), String> {
        let transformed = data.clone().transform(transform).ok_or_else(|| {
            format!(
                "{}: cannot apply SVG transform while normalizing icon paths",
                file.display()
            )
        })?;
        let mut path_data = String::new();
        for segment in transformed.segments() {
            match segment {
                usvg::tiny_skia_path::PathSegment::MoveTo(point) => {
                    let _ = write!(path_data, "M{} {} ", point.x, point.y);
                }
                usvg::tiny_skia_path::PathSegment::LineTo(point) => {
                    let _ = write!(path_data, "L{} {} ", point.x, point.y);
                }
                usvg::tiny_skia_path::PathSegment::QuadTo(control, end) => {
                    let _ = write!(
                        path_data,
                        "Q{} {} {} {} ",
                        control.x, control.y, end.x, end.y
                    );
                }
                usvg::tiny_skia_path::PathSegment::CubicTo(first, second, end) => {
                    let _ = write!(
                        path_data,
                        "C{} {} {} {} {} {} ",
                        first.x, first.y, second.x, second.y, end.x, end.y
                    );
                }
                usvg::tiny_skia_path::PathSegment::Close => path_data.push_str("Z "),
            }
        }
        let path_data = path_data.trim_end();
        if path_data.is_empty() {
            return Ok(());
        }
        let _ = writeln!(
            output,
            "<path d=\"{path_data}\" fill=\"#{:02X}{:02X}{:02X}\" fill-opacity=\"{opacity:.6}\"{} />",
            color.red,
            color.green,
            color.blue,
            if even_odd {
                " fill-rule=\"evenodd\""
            } else {
                ""
            },
        );
        Ok(())
    }

    fn append_group(
        group: &usvg::Group,
        inherited_opacity: f32,
        file: &Path,
        output: &mut String,
    ) -> Result<(), String> {
        let opacity = inherited_opacity * group.opacity().get();
        for node in group.children() {
            match node {
                usvg::Node::Group(child) => append_group(child, opacity, file, output)?,
                usvg::Node::Path(path) if path.is_visible() => {
                    if let Some(fill) = path.fill() {
                        append_path(
                            path.data(),
                            path.abs_transform(),
                            paint_color(fill.paint(), file)?,
                            fill.opacity().get() * opacity,
                            fill.rule() == usvg::FillRule::EvenOdd,
                            file,
                            output,
                        )?;
                    }
                    if let Some(stroke) = path.stroke() {
                        let style = usvg::tiny_skia_path::Stroke {
                            width: stroke.width().get(),
                            miter_limit: stroke.miterlimit().get(),
                            line_cap: match stroke.linecap() {
                                usvg::LineCap::Butt => usvg::tiny_skia_path::LineCap::Butt,
                                usvg::LineCap::Round => usvg::tiny_skia_path::LineCap::Round,
                                usvg::LineCap::Square => usvg::tiny_skia_path::LineCap::Square,
                            },
                            line_join: match stroke.linejoin() {
                                usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => {
                                    usvg::tiny_skia_path::LineJoin::Miter
                                }
                                usvg::LineJoin::Round => usvg::tiny_skia_path::LineJoin::Round,
                                usvg::LineJoin::Bevel => usvg::tiny_skia_path::LineJoin::Bevel,
                            },
                            dash: stroke.dasharray().and_then(|array| {
                                usvg::tiny_skia_path::StrokeDash::new(
                                    array.to_vec(),
                                    stroke.dashoffset(),
                                )
                            }),
                        };
                        let outline = path.data().stroke(&style, 1.0).ok_or_else(|| {
                            format!(
                                "{}: cannot convert stroke on <path> to a font outline",
                                file.display()
                            )
                        })?;
                        append_path(
                            &outline,
                            path.abs_transform(),
                            paint_color(stroke.paint(), file)?,
                            stroke.opacity().get() * opacity,
                            false,
                            file,
                            output,
                        )?;
                    }
                }
                usvg::Node::Path(_) => {}
                other => {
                    return Err(format!(
                        "{}: unsupported normalized SVG node {:?}",
                        file.display(),
                        other
                    ));
                }
            }
        }
        Ok(())
    }

    let raw_source = fs::read_to_string(source_path)
        .map_err(|error| format!("{}: {error}", source_path.display()))?;
    let (source, current_color_sentinel) = replace_current_color_with_sentinel(&raw_source);
    let tree = usvg::Tree::from_str(&source, &usvg::Options::default()).map_err(|error| {
        format!(
            "{}: invalid or unsupported SVG: {error}",
            source_path.display()
        )
    })?;
    let size = tree.size();
    let mut paths = String::new();
    append_group(tree.root(), 1.0, source_path, &mut paths)?;
    let normalized = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n{paths}</svg>\n",
        size.width(),
        size.height(),
        size.width(),
        size.height(),
    );
    fs::write(output_path, normalized)
        .map_err(|error| format!("{}: {error}", output_path.display()))?;
    Ok(current_color_sentinel)
}

fn replace_current_color_with_sentinel(source: &str) -> (String, Option<[u8; 3]>) {
    let lower = source.to_ascii_lowercase();
    if !lower.contains("currentcolor") {
        return (source.to_owned(), None);
    }
    let candidates: [([u8; 3], &str); 4] = [
        ([0x01, 0x02, 0x03], "#010203"),
        ([0xFE, 0x02, 0x03], "#fe0203"),
        ([0x01, 0xFE, 0x03], "#01fe03"),
        ([0x01, 0x02, 0xFE], "#0102fe"),
    ];
    let Some((rgb, hex)) = candidates
        .into_iter()
        .find(|(_, hex)| !lower.contains(hex))
    else {
        return (source.to_owned(), None);
    };
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0;
    while let Some(pos) = lower[cursor..].find("currentcolor") {
        let abs = cursor + pos;
        out.push_str(&source[cursor..abs]);
        out.push_str(hex);
        cursor = abs + "currentcolor".len();
    }
    out.push_str(&source[cursor..]);
    (out, Some(rgb))
}

fn read_icon_map(path: &Path) -> Result<BTreeMap<String, u32>, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let values: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&source)
        .map_err(|error| {
            format!(
                "{}: expected a JSON object mapping icon names to Unicode codepoints: {error}",
                path.display()
            )
        })?;
    if values.is_empty() {
        return Err(format!(
            "{}: icon map must contain at least one icon",
            path.display()
        ));
    }
    let mut icons = BTreeMap::new();
    for (name, value) in values {
        if name.trim().is_empty() || name.chars().any(char::is_whitespace) {
            return Err(format!(
                "{}: invalid empty or whitespace icon name `{name}`",
                path.display()
            ));
        }
        let codepoint = match value {
            serde_json::Value::Number(number) => {
                number.as_u64().and_then(|value| u32::try_from(value).ok())
            }
            serde_json::Value::String(value) => parse_codepoint(&value),
            _ => None,
        }
        .ok_or_else(|| {
            format!(
                "{}: icon `{name}` must map to an integer or hexadecimal Unicode codepoint string",
                path.display()
            )
        })?;
        if char::from_u32(codepoint).is_none() || (0xD800..=0xDFFF).contains(&codepoint) {
            return Err(format!(
                "{}: icon `{name}` has invalid Unicode codepoint U+{codepoint:04X}",
                path.display()
            ));
        }
        icons.insert(name, codepoint);
    }
    Ok(icons)
}

fn parse_codepoint(value: &str) -> Option<u32> {
    let value = value.trim();
    let digits = value
        .strip_prefix("U+")
        .or_else(|| value.strip_prefix("u+"))
        .or_else(|| value.strip_prefix("0x"))
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    u32::from_str_radix(digits, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::IconRegistry;
    use nexa_testkit::TempDir;
    use std::{fs, path::Path};

    fn svg(path: &Path, data: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("create svg directory");
        fs::write(path, data).expect("write svg");
    }

    #[test]
    fn discovers_named_sets_and_generates_a_deterministic_font() {
        let project = TempDir::new("nexa-icon-font-project");
        let output = TempDir::new("nexa-icon-font-output");
        let root = project.path();
        svg(
            &root.join("assets/icons/Brand/heart.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#e11d48" d="M12 21 3 12a5 5 0 0 1 9-5 5 5 0 0 1 9 5z"/></svg>"##,
        );
        svg(
            &root.join("assets/icons/Brand/like.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/></svg>"#,
        );
        let registry = IconRegistry::load(root, output.path()).expect("load icon set");
        let set = registry.sets.get("Brand").expect("Brand set");
        assert_eq!(set.icons["heart"], 0xE000);
        assert_eq!(set.icons["like"], 0xE001);
        let first = fs::read(&set.font_path).expect("first font");
        let normalized_sources = output.path().join(".svg-sources").join(&set.font_family);
        fs::remove_dir_all(&normalized_sources).expect("remove normalized SVG cache");
        IconRegistry::load(root, output.path()).expect("repeat generation");
        assert_eq!(first, fs::read(&set.font_path).expect("second font"));
        assert!(
            !normalized_sources.exists(),
            "cached icon fonts should avoid reparsing and normalizing their SVG sources"
        );
    }

    #[test]
    fn cleanup_removes_only_previously_managed_generated_font_files() {
        let project = TempDir::new("nexa-icon-font-cleanup-project");
        let output = TempDir::new("nexa-icon-font-cleanup-output");
        let stale_name = "Brand_0123456789abcdef.ttf";
        fs::write(output.path().join(stale_name), b"generated font").expect("stale font");
        fs::write(
            output.path().join("Brand_0123456789abcdef.icons.json"),
            b"generated metadata",
        )
        .expect("stale metadata");
        fs::write(
            output.path().join(".nexa-icon-fonts"),
            format!("{stale_name}\n"),
        )
        .expect("generated file manifest");
        fs::write(output.path().join("user.ttf"), b"user file").expect("unmanaged font");
        let stale_source = output
            .path()
            .join(".svg-sources/Brand_0123456789abcdef/heart.svg");
        fs::create_dir_all(stale_source.parent().expect("SVG cache directory"))
            .expect("create generated SVG cache");
        fs::write(&stale_source, "generated source").expect("generated SVG source");

        IconRegistry::load(project.path(), output.path()).expect("remove stale generated font");

        assert!(!output.path().join(stale_name).exists());
        assert!(
            !output
                .path()
                .join("Brand_0123456789abcdef.icons.json")
                .exists()
        );
        assert_eq!(
            fs::read(output.path().join("user.ttf")).expect("read unmanaged font"),
            b"user file"
        );
        assert!(!stale_source.parent().expect("SVG cache directory").exists());
    }

    #[test]
    fn prebuilt_font_uses_json_name_map_and_allows_duplicate_icon_names_in_other_sets() {
        let project = TempDir::new("nexa-icon-font-sidecar");
        let output = TempDir::new("nexa-icon-font-sidecar-output");
        let font_source = TempDir::new("nexa-icon-font-sidecar-source");
        let font_output = TempDir::new("nexa-icon-font-sidecar-source-output");
        let root = project.path();
        fs::create_dir_all(root.join("assets/fonts")).expect("font dir");
        svg(
            &font_source.path().join("assets/icons/Brand/heart.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>"#,
        );
        let source_registry = IconRegistry::load(font_source.path(), font_output.path())
            .expect("generate a real prebuilt-font fixture");
        fs::copy(
            &source_registry.sets["Brand"].font_path,
            root.join("assets/fonts/Brand.ttf"),
        )
        .expect("copy font fixture");
        fs::write(
            root.join("assets/fonts/Brand.icons.json"),
            r#"{"heart":"0xE000"}"#,
        )
        .expect("map");
        svg(
            &root.join("assets/icons/Other/heart.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#,
        );
        let registry = IconRegistry::load(root, output.path()).expect("load sets");
        assert_eq!(registry.sets["Brand"].icons["heart"], 0xE000);
        assert_eq!(registry.sets["Brand"].font_resource, "Brand");
        assert_ne!(registry.sets["Brand"].font_family, "Brand");
        assert!(registry.sets["Other"].icons.contains_key("heart"));
    }

    #[test]
    fn prebuilt_sidecar_rejects_codepoints_missing_from_the_font() {
        let project = TempDir::new("nexa-icon-font-sidecar-missing-glyph");
        let output = TempDir::new("nexa-icon-font-sidecar-missing-glyph-output");
        let font_source = TempDir::new("nexa-icon-font-sidecar-missing-glyph-source");
        let font_output = TempDir::new("nexa-icon-font-sidecar-missing-glyph-source-output");
        svg(
            &font_source.path().join("assets/icons/Brand/heart.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>"#,
        );
        let source_registry = IconRegistry::load(font_source.path(), font_output.path())
            .expect("generate a real prebuilt-font fixture");
        fs::create_dir_all(project.path().join("assets/fonts")).expect("font dir");
        fs::copy(
            &source_registry.sets["Brand"].font_path,
            project.path().join("assets/fonts/Brand.ttf"),
        )
        .expect("copy font fixture");
        fs::write(
            project.path().join("assets/fonts/Brand.icons.json"),
            r#"{"heart":"0xF100"}"#,
        )
        .expect("write mismatched sidecar");

        let error = IconRegistry::load(project.path(), output.path())
            .expect_err("missing glyphs must not silently render as fallback characters");
        assert!(error.contains("heart"));
        assert!(error.contains("U+F100"));
        assert!(error.contains("missing from the font"));
    }

    #[test]
    fn reports_unsupported_artwork_with_file_and_element() {
        let project = TempDir::new("nexa-icon-font-unsupported");
        let output = TempDir::new("nexa-icon-font-unsupported-output");
        let root = project.path();
        svg(
            &root.join("assets/icons/Brand/heart.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg"><text>heart</text></svg>"#,
        );
        let error = IconRegistry::load(root, output.path()).expect_err("text should be rejected");
        assert!(error.contains("heart.svg"));
        assert!(error.contains("<text>"));
    }

    #[test]
    fn preserves_multicolor_fills_and_converts_strokes_into_color_glyph_layers() {
        let project = TempDir::new("nexa-icon-font-color-layers");
        let output = TempDir::new("nexa-icon-font-color-layers-output");
        let root = project.path();
        svg(
            &root.join("assets/icons/Brand/heart.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
                <g transform="translate(1 1)">
                  <path fill="#e11d48" d="M11 20 2 11a5 5 0 0 1 9-5 5 5 0 0 1 9 5z"/>
                  <path fill="none" stroke="#111827" stroke-width="1.5" stroke-linecap="round" d="M5 12h12"/>
                </g>
            </svg>"##,
        );
        let registry = IconRegistry::load(root, output.path()).expect("supported color SVG");
        let font = fs::read(&registry.sets["Brand"].font_path).expect("generated font");
        assert!(font.windows(4).any(|table| table == b"COLR"));
        assert!(font.windows(4).any(|table| table == b"CPAL"));
        let layers = registry.sets["Brand"]
            .layers
            .get("heart")
            .expect("multicolor glyph layers");
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].rgba, [225, 29, 72, 255]);
        assert_eq!(layers[1].rgba, [17, 24, 39, 255]);
        let face = ttf_parser::Face::parse(&font, 0).expect("rewritten font cmap");
        let base_glyph = face.glyph_index('\u{E000}').expect("base icon cmap");
        let base_advance = face
            .glyph_hor_advance(base_glyph)
            .expect("base icon advance");
        for layer in layers {
            let alias = face
                .glyph_index(char::from_u32(layer.glyph).expect("color layer codepoint"))
                .expect("color layer cmap");
            assert_eq!(face.glyph_hor_advance(alias), Some(base_advance));
        }
    }

    #[test]
    fn subsets_a_generated_font_with_color_layers_and_non_bmp_aliases() {
        let project = TempDir::new("nexa-icon-font-color-subset");
        let output = TempDir::new("nexa-icon-font-color-subset-output");
        let files =
            include_str!("../../../tests/fixtures/icon-fastlist/assets/icons/Brand/badge.svg");
        let unused =
            include_str!("../../../tests/fixtures/icon-fastlist/assets/icons/Brand/unused.svg");
        svg(&project.path().join("assets/icons/Brand/badge.svg"), files);
        svg(
            &project.path().join("assets/icons/Brand/unused.svg"),
            unused,
        );
        let registry = IconRegistry::load(project.path(), output.path()).expect("generate font");
        let set = &registry.sets["Brand"];
        let input = fs::read(&set.font_path).expect("open generated font");
        let used = std::iter::once(set.icons["badge"])
            .chain(set.layers["badge"].iter().map(|layer| layer.glyph))
            .collect();
        let bytes = super::subset_icon_font(&input, &used).expect("subset generated font");
        let face = ttf_parser::Face::parse(&bytes, 0).expect("parse subset");
        for glyph in used {
            assert!(
                face.glyph_index(char::from_u32(glyph).expect("codepoint"))
                    .is_some()
            );
        }
        assert!(
            face.glyph_index(char::from_u32(set.icons["unused"]).expect("codepoint"))
                .is_none()
        );
    }
}
