use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub(super) struct ProjectImage {
    pub(super) name: String,
    source: PathBuf,
    extension: String,
}

struct ProjectFont {
    resource_name: String,
    source: PathBuf,
    extension: String,
}

pub(super) fn copy_ios_project_images(
    source_root: &Path,
    output_root: &Path,
    app_name: &str,
) -> Result<bool, String> {
    let images = project_images(source_root)?;
    let catalog = output_root
        .join("ios")
        .join(app_name)
        .join("Assets.xcassets");
    let marker = output_root
        .join("ios")
        .join(app_name)
        .join(".nexa-project-images");
    let generated = images
        .iter()
        .flat_map(|image| {
            let extension = ios_asset_extension(image);
            [
                format!("{}.imageset", image.name),
                format!("{}.imageset/{}.{}", image.name, image.name, extension),
            ]
        })
        .collect::<BTreeSet<_>>();
    remove_stale_generated(&catalog, &marker, &generated, true)?;
    for image in &images {
        let asset_set = format!("{}.imageset", image.name);
        let directory = catalog.join(&asset_set);
        ensure_owned_destination(&directory, &marker, &asset_set)?;
        fs::create_dir_all(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        let filename = format!("{}.{}", image.name, ios_asset_extension(image));
        let destination = directory.join(&filename);
        if image.extension == "webp" {
            let decoded = image::open(&image.source)
                .map_err(|error| format!("{}: {error}", image.source.display()))?;
            let mut encoded = std::io::Cursor::new(Vec::new());
            decoded
                .write_to(&mut encoded, image::ImageFormat::Png)
                .map_err(|error| format!("{}: {error}", destination.display()))?;
            super::file_ops::write_bytes_if_changed(&destination, encoded.get_ref())?;
        } else {
            copy_if_changed(&image.source, &destination)?;
        }
        write_if_changed(
            &directory.join("Contents.json"),
            &format!(
                "{{\"images\":[{{\"filename\":\"{filename}\",\"idiom\":\"universal\"}}],\"info\":{{\"author\":\"xcode\",\"version\":1}}}}\n"
            ),
        )?;
    }
    write_marker(&marker, &generated)?;
    Ok(!images.is_empty())
}

fn ios_asset_extension(image: &ProjectImage) -> &str {
    if image.extension == "webp" {
        "png"
    } else {
        &image.extension
    }
}

pub(super) fn copy_android_project_images(
    images: &[ProjectImage],
    output_root: &Path,
) -> Result<(), String> {
    let resources = output_root.join("android/app/src/main/res/drawable-nodpi");
    let marker = output_root.join("android/app/src/main/res/.nexa-project-images");
    let generated = images
        .iter()
        .map(|image| {
            let extension = if image.extension == "jpeg" {
                "jpg"
            } else {
                &image.extension
            };
            format!("{}.{}", image.name, extension)
        })
        .collect::<BTreeSet<_>>();
    remove_stale_generated(&resources, &marker, &generated, false)?;
    for image in images {
        let filename = if image.extension == "jpeg" {
            format!("{}.jpg", image.name)
        } else {
            format!("{}.{}", image.name, image.extension)
        };
        let destination = resources.join(&filename);
        ensure_owned_destination(&destination, &marker, &filename)?;
        fs::create_dir_all(&resources)
            .map_err(|error| format!("{}: {error}", resources.display()))?;
        copy_if_changed(&image.source, &destination)?;
    }
    write_marker(&marker, &generated)
}

pub(super) fn write_android_image_keep_rules(
    output_root: &Path,
    package: &str,
    module: &nexa_ir::Module,
    known_drawables: &BTreeSet<String>,
) -> Result<(), String> {
    let mut references = BTreeSet::new();
    let bodies = std::iter::once(module.body.as_slice())
        .chain(module.screens.iter().map(|screen| screen.body.as_slice()))
        .chain(
            module
                .components
                .iter()
                .map(|component| component.body.as_slice()),
        )
        .chain(module.widgets.iter().map(|widget| widget.body.as_slice()));
    for body in bodies {
        nexa_ir::walk::walk_ir(
            body,
            &mut |node| {
                if let nexa_ir::Node::Image {
                    source,
                    placeholder,
                    ..
                } = node
                {
                    if let nexa_ir::ImageSource::Asset(name) = source
                        && valid_image_asset_name(name)
                        && !known_drawables.contains(name)
                    {
                        references.insert(format!("@drawable/{name}"));
                    }
                    if let Some(name) = placeholder
                        && valid_image_asset_name(name)
                        && !known_drawables.contains(name)
                    {
                        references.insert(format!("@drawable/{name}"));
                    }
                }
            },
            &mut |_| {},
        );
    }
    let path = output_root
        .join("android/app/src/main/res/raw")
        .join(format!(
            "{}_nexa_image_keep.xml",
            package.to_ascii_lowercase().replace('.', "_")
        ));
    if references.is_empty() {
        return match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("{}: {error}", path.display())),
        };
    }
    write_if_changed(
        &path,
        &format!(
            "<resources xmlns:tools=\"http://schemas.android.com/tools\" tools:keep=\"{}\" />\n",
            references.into_iter().collect::<Vec<_>>().join(",")
        ),
    )
}

/// Copies registered app fonts into both native targets' resource layouts.
/// Font file stems use `<family>_<weight>.ttf` for weighted families, or
/// `<family>.ttf` for a single-face family. Recognized weights are `regular`,
/// `medium`, `semibold`, and `bold`.
pub(super) fn copy_ios_project_fonts(
    source_root: &Path,
    output_root: &Path,
    app_name: &str,
    icon_usage: Option<&IconFontUsage>,
) -> Result<Vec<String>, String> {
    let fonts = packaged_project_fonts(source_root, icon_usage)?;
    let directory = output_root.join("ios").join(app_name).join("Fonts");
    let marker = output_root
        .join("ios")
        .join(app_name)
        .join(".nexa-project-fonts");
    let generated = fonts
        .iter()
        .map(|font| {
            font.source
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| format!("Fonts/{name}"))
                .ok_or_else(|| format!("invalid app font path: {}", font.source.display()))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    remove_stale_generated(
        &output_root.join("ios").join(app_name),
        &marker,
        &generated,
        false,
    )?;
    if fonts.is_empty() {
        write_marker(&marker, &generated)?;
        return Ok(Vec::new());
    }
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    for font in &fonts {
        let filename = font
            .source
            .file_name()
            .ok_or_else(|| format!("invalid font path: {}", font.source.display()))?;
        let destination = directory.join(filename);
        let relative = format!("Fonts/{}", filename.to_string_lossy());
        ensure_owned_destination(&destination, &marker, &relative)?;
        stage_project_font(font, &destination, icon_usage)?;
    }
    write_marker(&marker, &generated)?;
    Ok(generated.into_iter().collect())
}

pub(super) fn copy_android_project_fonts(
    source_root: &Path,
    output_root: &Path,
    icon_usage: Option<&IconFontUsage>,
) -> Result<(), String> {
    let fonts = packaged_project_fonts(source_root, icon_usage)?;
    let resources = output_root.join("android/app/src/main/res/font");
    let marker = output_root.join("android/app/src/main/res/.nexa-project-fonts");
    let mut generated = BTreeSet::new();
    for font in &fonts {
        generated.insert(format!("{}.{}", font.resource_name, font.extension));
    }
    remove_stale_generated(&resources, &marker, &generated, false)?;
    if !fonts.is_empty() {
        fs::create_dir_all(&resources)
            .map_err(|error| format!("{}: {error}", resources.display()))?;
    }
    for font in &fonts {
        let filename = format!("{}.{}", font.resource_name, font.extension);
        let destination = resources.join(&filename);
        ensure_owned_destination(&destination, &marker, &filename)?;
        stage_project_font(font, &destination, icon_usage)?;
    }
    write_marker(&marker, &generated)?;

    // Older DevRuntime hosts staged a second copy under assets/nexa/fonts.
    // Retire only files listed in that generator-owned marker so upgrades
    // stop packaging unused font assets without touching user files.
    let legacy_dev_assets = output_root.join("android/app/src/main/assets/nexa/fonts");
    let legacy_dev_marker = legacy_dev_assets.join(".nexa-project-fonts");
    remove_stale_generated(
        &legacy_dev_assets,
        &legacy_dev_marker,
        &BTreeSet::new(),
        false,
    )?;
    write_marker(&legacy_dev_marker, &BTreeSet::new())?;
    Ok(())
}

pub(super) struct IconFontUsage {
    codepoints: std::collections::BTreeMap<String, Option<BTreeSet<u32>>>,
    text_resources: BTreeSet<String>,
}

impl IconFontUsage {
    pub(super) fn analyze(module: &nexa_ir::Module, catalog: &nexa_codegen::FontCatalog) -> Self {
        let ui = nexa_ir::facts::ModuleFacts::analyze(module).ui;
        let families = ui
            .text_font_families
            .iter()
            .filter_map(|family| {
                catalog
                    .resolve(family, nexa_ir::FontWeight::Normal)
                    .map(|face| face.family.to_ascii_lowercase())
            })
            .collect::<BTreeSet<_>>();
        let text_resources = catalog
            .faces()
            .filter(|face| families.contains(&face.family.to_ascii_lowercase()))
            .map(|face| face.android_resource_name.clone())
            .collect();
        let codepoints = ui
            .icon_font_codepoints
            .into_iter()
            .map(|(name, codepoints)| (nexa_codegen::names::font_resource_name(&name), codepoints))
            .collect();
        Self {
            codepoints,
            text_resources,
        }
    }

    fn is_managed(&self, font: &ProjectFont) -> bool {
        !self.text_resources.contains(&font.resource_name)
            && font.source.with_extension("icons.json").is_file()
    }
}

fn packaged_project_fonts(
    source_root: &Path,
    icon_usage: Option<&IconFontUsage>,
) -> Result<Vec<ProjectFont>, String> {
    let mut fonts = project_fonts(source_root)?;
    if let Some(usage) = icon_usage {
        fonts.retain(|font| {
            !usage.is_managed(font) || usage.codepoints.contains_key(&font.resource_name)
        });
    }
    Ok(fonts)
}

fn stage_project_font(
    font: &ProjectFont,
    destination: &Path,
    icon_usage: Option<&IconFontUsage>,
) -> Result<(), String> {
    if let Some(codepoints) = icon_usage
        .filter(|usage| usage.is_managed(font))
        .and_then(|usage| usage.codepoints.get(&font.resource_name))
        .and_then(Option::as_ref)
    {
        let original = fs::read(&font.source)
            .map_err(|error| format!("{}: {error}", font.source.display()))?;
        let subset = nexa_icon_font::subset_icon_font(&original, codepoints);
        return super::file_ops::write_bytes_if_changed(
            destination,
            subset.as_deref().unwrap_or(&original),
        );
    }
    copy_if_changed(&font.source, destination)
}

pub(super) fn load_font_catalog(
    root: &Path,
    configured_families: &[nexa_syntax::ast::FontFamilyConfig],
) -> Result<nexa_codegen::FontCatalog, String> {
    // Generated icon fonts are registered by the typed icon path, not as text
    // families. Keeping them out of this catalog avoids parsing and resolving
    // a second copy of SVG-derived font metadata for every source font lookup.
    let generated_root = root.join(".nexa/build/icon-fonts");
    let fonts = project_fonts(root)?
        .into_iter()
        .filter(|font| !font.source.starts_with(&generated_root))
        .collect::<Vec<_>>();
    let mut catalog = nexa_codegen::FontCatalog::default();
    let mut metadata_by_path = std::collections::HashMap::new();

    for font in &fonts {
        let face = read_font_face(&font.source)?;
        let resolved = nexa_codegen::FontFace {
            family: face.family.clone(),
            weight: face.weight,
            postscript_name: face.postscript_name.clone(),
            android_resource_name: font.resource_name.clone(),
        };
        if catalog
            .resolve_exact(&resolved.family, resolved.weight)
            .is_some()
        {
            return Err(format!(
                "multiple font files declare family `{}` at weight {:?}; configure an explicit `assets.fonts` mapping",
                resolved.family, resolved.weight
            ));
        }
        catalog.insert(resolved);
        metadata_by_path.insert(
            font.source
                .canonicalize()
                .unwrap_or_else(|_| font.source.clone()),
            face,
        );
    }

    let font_directory = root.join("assets/fonts").canonicalize().ok();
    for configured in configured_families {
        for configured_face in &configured.faces {
            let path = root.join(&configured_face.path);
            let canonical = path
                .canonicalize()
                .map_err(|error| format!("configured font {}: {error}", path.display()))?;
            let Some(font_directory) = &font_directory else {
                return Err(format!(
                    "configured font family `{}` refers to `{}`, but the project has no assets/fonts directory",
                    configured.family, configured_face.path
                ));
            };
            if canonical.parent() != Some(font_directory.as_path()) {
                return Err(format!(
                    "configured font `{}` must be a regular file directly inside assets/fonts",
                    configured_face.path
                ));
            }
            let Some(font) = fonts.iter().find(|font| {
                font.source.canonicalize().ok().as_deref() == Some(canonical.as_path())
            }) else {
                return Err(format!(
                    "configured font `{}` is not a supported file in assets/fonts",
                    configured_face.path
                ));
            };
            let metadata = metadata_by_path.get(&canonical).ok_or_else(|| {
                format!(
                    "configured font metadata was not loaded: {}",
                    canonical.display()
                )
            })?;
            let weight = match configured_face.weight {
                nexa_syntax::ast::FontFaceWeight::Normal => nexa_ir::FontWeight::Normal,
                nexa_syntax::ast::FontFaceWeight::Medium => nexa_ir::FontWeight::Medium,
                nexa_syntax::ast::FontFaceWeight::Semibold => nexa_ir::FontWeight::Semibold,
                nexa_syntax::ast::FontFaceWeight::Bold => nexa_ir::FontWeight::Bold,
            };
            catalog.insert(nexa_codegen::FontFace {
                family: configured.family.clone(),
                weight,
                postscript_name: metadata.postscript_name.clone(),
                android_resource_name: font.resource_name.clone(),
            });
        }
    }

    Ok(catalog)
}

struct ParsedFontFace {
    family: String,
    weight: nexa_ir::FontWeight,
    postscript_name: String,
}

fn read_font_face(path: &Path) -> Result<ParsedFontFace, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let face = ttf_parser::Face::parse(&bytes, 0)
        .map_err(|error| format!("invalid font file {}: {error:?}", path.display()))?;
    let name = |id: u16| {
        face.names()
            .into_iter()
            .filter(|record| record.name_id == id)
            .max_by_key(|record| u8::from(record.language_id == 0x0409))
            .and_then(|record| record.to_string())
    };
    let family = name(16)
        .or_else(|| name(1))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("font has no family name: {}", path.display()))?;
    let postscript_name = name(6)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("font has no PostScript name: {}", path.display()))?;
    let numeric_weight = face.weight().to_number();
    let weight = match numeric_weight {
        0..=450 => nexa_ir::FontWeight::Normal,
        451..=550 => nexa_ir::FontWeight::Medium,
        551..=650 => nexa_ir::FontWeight::Semibold,
        _ => nexa_ir::FontWeight::Bold,
    };
    Ok(ParsedFontFace {
        family,
        weight,
        postscript_name,
    })
}

fn project_fonts(root: &Path) -> Result<Vec<ProjectFont>, String> {
    let directory = root.join("assets/fonts");
    let generated_directory = root.join(".nexa/build/icon-fonts");
    let mut fonts = Vec::new();
    let mut names = BTreeSet::new();
    for (directory, generated) in [(&directory, false), (&generated_directory, true)] {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("{}: {error}", directory.display())),
        };
        for entry in entries {
            let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
            let path = entry.path();
            let filename = entry.file_name().to_string_lossy().into_owned();
            if filename.starts_with('.') || filename.ends_with(".icons.json") {
                continue;
            }
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if !metadata.file_type().is_file() {
                return Err(format!(
                    "app font assets must be regular files directly inside {}: {}",
                    directory.display(),
                    path.display()
                ));
            }
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .map(str::to_ascii_lowercase)
                .ok_or_else(|| {
                    format!("app font has no supported extension: {}", path.display())
                })?;
            if !matches!(extension.as_str(), "ttf" | "otf") || (generated && extension != "ttf") {
                return Err(format!(
                    "unsupported app font format `{extension}`: {} (use ttf or otf)",
                    path.display()
                ));
            }
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| format!("invalid app font filename: {}", path.display()))?;
            let resource_name = nexa_codegen::names::font_resource_name(stem);
            if !names.insert(resource_name.clone()) {
                return Err(format!(
                    "app fonts map to duplicate native face name `{resource_name}`"
                ));
            }
            fonts.push(ProjectFont {
                resource_name,
                source: path,
                extension,
            });
        }
    }
    fonts.sort_by(|left, right| left.resource_name.cmp(&right.resource_name));
    Ok(fonts)
}

pub(super) fn copy_android_localizations(
    source_root: &Path,
    output_root: &Path,
    module: &nexa_ir::Module,
) -> Result<(), String> {
    let catalog = super::localization::synchronize(source_root, module)?;
    let resources = output_root.join("android/app/src/main/res");
    let marker = resources.join(".nexa-localization-resources");
    let files = catalog.android_string_files()?;
    let generated = files
        .iter()
        .map(|(path, _)| path.clone())
        .collect::<BTreeSet<_>>();
    remove_stale_generated(&resources, &marker, &generated, false)?;
    for (relative, contents) in files {
        let destination = resources.join(&relative);
        ensure_owned_destination(&destination, &marker, &relative)?;
        let parent = destination
            .parent()
            .ok_or_else(|| format!("invalid localization output path {}", destination.display()))?;
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        write_if_changed(&destination, &contents)?;
    }
    write_marker(&marker, &generated)
}

pub(super) fn project_images(root: &Path) -> Result<Vec<ProjectImage>, String> {
    let directory = root.join("assets/images");
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", directory.display())),
    };
    let mut images = Vec::new();
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "app image assets must be regular files directly inside {}: {}",
                directory.display(),
                path.display()
            ));
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| {
                format!(
                    "app image asset has no supported extension: {}",
                    path.display()
                )
            })?;
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            return Err(format!(
                "unsupported app image asset format `{extension}`: {} (use png, jpg, jpeg, or webp)",
                path.display()
            ));
        }
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                format!(
                    "app image asset name is not valid UTF-8: {}",
                    path.display()
                )
            })?
            .to_owned();
        if !valid_image_asset_name(&name) {
            return Err(format!(
                "app image asset names must start with a lowercase letter and contain only lowercase letters, digits, or underscores: `{name}`"
            ));
        }
        if !names.insert(name.clone()) {
            return Err(format!(
                "app image asset name `{name}` is duplicated in {}",
                directory.display()
            ));
        }
        images.push(ProjectImage {
            name,
            source: path,
            extension,
        });
    }
    images.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(images)
}

fn valid_image_asset_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn ensure_owned_destination(destination: &Path, marker: &Path, name: &str) -> Result<(), String> {
    if destination.exists() && !read_marker(marker)?.contains(name) {
        return Err(format!(
            "app image asset would overwrite an existing native resource: {}",
            destination.display()
        ));
    }
    Ok(())
}

fn copy_if_changed(source: &Path, destination: &Path) -> Result<(), String> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "refusing to overwrite generated asset symlink: {}",
                destination.display()
            ));
        }
        Ok(metadata)
            if metadata.is_file() && super::file_ops::files_equal(source, destination)? =>
        {
            return Ok(());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}: {error}", destination.display())),
    }
    fs::copy(source, destination).map_err(|error| format!("{}: {error}", source.display()))?;
    Ok(())
}

fn remove_stale_generated(
    destination_root: &Path,
    marker: &Path,
    generated: &BTreeSet<String>,
    directories: bool,
) -> Result<(), String> {
    for name in read_marker(marker)? {
        if generated.contains(&name) {
            continue;
        }
        let stale = destination_root.join(name);
        if directories && stale.is_dir() {
            fs::remove_dir_all(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
        } else if stale.is_file() {
            fs::remove_file(&stale).map_err(|error| format!("{}: {error}", stale.display()))?;
        }
    }
    Ok(())
}

fn read_marker(marker: &Path) -> Result<BTreeSet<String>, String> {
    match fs::read_to_string(marker) {
        Ok(contents) => {
            let mut entries = BTreeSet::new();
            for entry in contents.lines() {
                let path = Path::new(entry);
                if entry.is_empty()
                    || path.is_absolute()
                    || !path
                        .components()
                        .all(|component| matches!(component, std::path::Component::Normal(_)))
                    || path.components().count() == 0
                    || path.components().count() > 2
                {
                    return Err(format!("invalid generated asset marker entry `{entry}`"));
                }
                entries.insert(entry.to_owned());
            }
            Ok(entries)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(error) => Err(format!("{}: {error}", marker.display())),
    }
}

fn write_marker(marker: &Path, generated: &BTreeSet<String>) -> Result<(), String> {
    if generated.is_empty() {
        if marker.is_file() {
            fs::remove_file(marker).map_err(|error| format!("{}: {error}", marker.display()))?;
        }
        return Ok(());
    }
    let mut contents = generated.iter().cloned().collect::<Vec<_>>().join("\n");
    contents.push('\n');
    write_if_changed(marker, &contents)
}

pub(super) fn generate_ios_icon(source: &Path, root: &Path, app_name: &str) -> Result<(), String> {
    let image = image::open(source).map_err(|error| format!("{}: {error}", source.display()))?;
    let icon = image.resize_to_fill(1024, 1024, image::imageops::FilterType::Lanczos3);
    let directory = root
        .join("ios")
        .join(app_name)
        .join("Assets.xcassets/AppIcon.appiconset");
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    icon.save(directory.join("AppIcon.png"))
        .map_err(|error| error.to_string())?;
    write_if_changed(
        &directory.join("Contents.json"),
        "{\n  \"images\" : [\n    { \"filename\" : \"AppIcon.png\", \"idiom\" : \"universal\", \"platform\" : \"ios\", \"size\" : \"1024x1024\" }\n  ],\n  \"info\" : { \"author\" : \"xcode\", \"version\" : 1 }\n}\n",
    )
}

pub(super) fn copy_icon_composer(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(source).map_err(|error| format!("{}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Icon Composer assets cannot be symlinks: {}",
            source.display()
        ));
    }
    if !metadata.is_dir() && !metadata.is_file() {
        return Err(format!(
            "Icon Composer asset must be a file or package directory: {}",
            source.display()
        ));
    }

    match fs::symlink_metadata(destination) {
        Ok(existing) if existing.file_type().is_dir() => {
            fs::remove_dir_all(destination)
                .map_err(|error| format!("{}: {error}", destination.display()))?;
        }
        Ok(_) => {
            fs::remove_file(destination)
                .map_err(|error| format!("{}: {error}", destination.display()))?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}: {error}", destination.display())),
    }

    if metadata.is_dir() {
        fs::create_dir_all(destination)
            .map_err(|error| format!("{}: {error}", destination.display()))?;
        for entry in
            fs::read_dir(source).map_err(|error| format!("{}: {error}", source.display()))?
        {
            let entry = entry.map_err(|error| format!("{}: {error}", source.display()))?;
            copy_icon_composer_entry(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else {
        let parent = destination.parent().ok_or_else(|| {
            format!(
                "invalid Icon Composer destination {}",
                destination.display()
            )
        })?;
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        fs::copy(source, destination)
            .map_err(|error| format!("{}: {error}", destination.display()))?;
    }
    Ok(())
}

fn copy_icon_composer_entry(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(source).map_err(|error| format!("{}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Icon Composer packages cannot contain symlinks: {}",
            source.display()
        ));
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination)
            .map_err(|error| format!("{}: {error}", destination.display()))?;
        for entry in
            fs::read_dir(source).map_err(|error| format!("{}: {error}", source.display()))?
        {
            let entry = entry.map_err(|error| format!("{}: {error}", source.display()))?;
            copy_icon_composer_entry(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        let parent = destination
            .parent()
            .ok_or_else(|| format!("invalid Icon Composer path {}", destination.display()))?;
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        fs::copy(source, destination)
            .map_err(|error| format!("{}: {error}", destination.display()))?;
    } else {
        return Err(format!(
            "Icon Composer packages contain an unsupported file type: {}",
            source.display()
        ));
    }
    Ok(())
}

pub(super) fn generate_android_icon(source: &Path, root: &Path) -> Result<(), String> {
    generate_android_icon_named(source, root, "ic_launcher")
}

pub(super) fn generate_android_icon_named(
    source: &Path,
    root: &Path,
    resource_name: &str,
) -> Result<(), String> {
    validate_android_icon_resource_name(resource_name)?;
    if source.is_dir() {
        return generate_android_icon_set_named(source, root, resource_name);
    }

    let image = image::open(source).map_err(|error| format!("{}: {error}", source.display()))?;
    let res = root.join("android/app/src/main/res");
    remove_previous_android_icon_layers(&res, resource_name)?;
    for (density, size) in [
        ("mdpi", 48),
        ("hdpi", 72),
        ("xhdpi", 96),
        ("xxhdpi", 144),
        ("xxxhdpi", 192),
    ] {
        let directory = res.join(format!("mipmap-{density}"));
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let icon = image.resize_to_fill(size, size, image::imageops::FilterType::Lanczos3);
        icon.save(directory.join(format!("{resource_name}.png")))
            .map_err(|error| error.to_string())?;
        icon.save(directory.join(format!("{resource_name}_round.png")))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Generates density-aware legacy icons and a native adaptive icon from a
/// developer-supplied set of separate layers. A single flattened bitmap cannot
/// provide safe-zone-aware motion or a correct monochrome themed icon, so it
/// intentionally stays a legacy launcher image unless a layer set is given.
pub(super) fn generate_android_icon_set_named(
    source: &Path,
    root: &Path,
    resource_name: &str,
) -> Result<(), String> {
    validate_android_icon_resource_name(resource_name)?;
    let fallback = source.join("icon.png");
    let image =
        image::open(&fallback).map_err(|error| format!("{}: {error}", fallback.display()))?;
    let foreground = required_icon_layer(source, "foreground")?;
    let background = required_icon_layer(source, "background")?;
    let monochrome = optional_icon_layer(source, "monochrome");
    validate_android_icon_layer(&foreground)?;
    validate_android_icon_layer(&background)?;
    if let Some(monochrome) = &monochrome {
        validate_android_icon_layer(monochrome)?;
    }

    let res = root.join("android/app/src/main/res");
    remove_previous_android_icon_layers(&res, resource_name)?;
    for (density, size) in [
        ("mdpi", 48),
        ("hdpi", 72),
        ("xhdpi", 96),
        ("xxhdpi", 144),
        ("xxxhdpi", 192),
    ] {
        let directory = res.join(format!("mipmap-{density}"));
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let icon = image.resize_to_fill(size, size, image::imageops::FilterType::Lanczos3);
        icon.save(directory.join(format!("{resource_name}.png")))
            .map_err(|error| error.to_string())?;
        icon.save(directory.join(format!("{resource_name}_round.png")))
            .map_err(|error| error.to_string())?;
    }

    install_android_icon_layer(&foreground, &res, resource_name, "foreground")?;
    install_android_icon_layer(&background, &res, resource_name, "background")?;
    if let Some(monochrome) = &monochrome {
        install_android_icon_layer(monochrome, &res, resource_name, "monochrome")?;
    }

    let adaptive_dir = res.join("mipmap-anydpi-v26");
    fs::create_dir_all(&adaptive_dir).map_err(|error| error.to_string())?;
    let adaptive_xml = adaptive_icon_xml(resource_name, false);
    for name in [
        format!("{resource_name}.xml"),
        format!("{resource_name}_round.xml"),
    ] {
        write_if_changed(&adaptive_dir.join(name), &adaptive_xml)?;
    }

    if monochrome.is_some() {
        let themed_dir = res.join("mipmap-anydpi-v33");
        fs::create_dir_all(&themed_dir).map_err(|error| error.to_string())?;
        let themed_xml = adaptive_icon_xml(resource_name, true);
        for name in [
            format!("{resource_name}.xml"),
            format!("{resource_name}_round.xml"),
        ] {
            write_if_changed(&themed_dir.join(name), &themed_xml)?;
        }
    }
    Ok(())
}

fn validate_android_icon_resource_name(resource_name: &str) -> Result<(), String> {
    let valid = resource_name
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_lowercase)
        && resource_name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid Android icon resource name `{resource_name}`; expected lowercase ASCII letters, digits, and underscores, starting with a letter"
        ))
    }
}

fn remove_previous_android_icon_layers(res: &Path, resource_name: &str) -> Result<(), String> {
    let mut stale = vec![
        res.join("mipmap-anydpi-v26")
            .join(format!("{resource_name}.xml")),
        res.join("mipmap-anydpi-v26")
            .join(format!("{resource_name}_round.xml")),
        res.join("mipmap-anydpi-v33")
            .join(format!("{resource_name}.xml")),
        res.join("mipmap-anydpi-v33")
            .join(format!("{resource_name}_round.xml")),
    ];
    for layer in ["foreground", "background", "monochrome"] {
        stale.push(
            res.join("drawable")
                .join(format!("{resource_name}_{layer}.xml")),
        );
        for density in ["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"] {
            stale.push(
                res.join(format!("drawable-{density}"))
                    .join(format!("{resource_name}_{layer}.png")),
            );
        }
    }
    for path in stale {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    Ok(())
}

fn required_icon_layer(source: &Path, name: &str) -> Result<PathBuf, String> {
    if let Some(layer) = find_icon_layer(source, name) {
        return Ok(layer);
    }
    Err(format!(
        "Android icon set {} needs `{name}.png` or `{name}.xml`",
        source.display()
    ))
}

fn optional_icon_layer(source: &Path, name: &str) -> Option<PathBuf> {
    find_icon_layer(source, name)
}

fn find_icon_layer(source: &Path, name: &str) -> Option<PathBuf> {
    ["png", "xml"]
        .into_iter()
        .flat_map(|extension| {
            [
                source.join(format!("{name}.{extension}")),
                source
                    .parent()
                    .unwrap_or(source)
                    .join("shared")
                    .join(format!("{name}.{extension}")),
            ]
        })
        .find(|path| path.is_file())
}

fn install_android_icon_layer(
    source: &Path,
    res: &Path,
    resource_name: &str,
    layer_name: &str,
) -> Result<(), String> {
    match source.extension().and_then(|extension| extension.to_str()) {
        Some("xml") => {
            let drawable_dir = res.join("drawable");
            fs::create_dir_all(&drawable_dir).map_err(|error| error.to_string())?;
            let contents = fs::read_to_string(source)
                .map_err(|error| format!("{}: {error}", source.display()))?;
            write_if_changed(
                &drawable_dir.join(format!("{resource_name}_{layer_name}.xml")),
                &contents,
            )
        }
        Some("png") => {
            let image =
                image::open(source).map_err(|error| format!("{}: {error}", source.display()))?;
            for (density, scale) in [
                ("mdpi", 1.0),
                ("hdpi", 1.5),
                ("xhdpi", 2.0),
                ("xxhdpi", 3.0),
                ("xxxhdpi", 4.0),
            ] {
                let directory = res.join(format!("drawable-{density}"));
                fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
                let size = (108.0 * scale) as u32;
                image
                    .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                    .save(directory.join(format!("{resource_name}_{layer_name}.png")))
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
        _ => Err(format!(
            "Android adaptive icon layer {} must be PNG or Android drawable XML",
            source.display()
        )),
    }
}

fn validate_android_icon_layer(source: &Path) -> Result<(), String> {
    match source.extension().and_then(|extension| extension.to_str()) {
        Some("png") => {
            let image =
                image::open(source).map_err(|error| format!("{}: {error}", source.display()))?;
            if image.width() != image.height() {
                return Err(format!(
                    "Android adaptive icon layer {} must be square",
                    source.display()
                ));
            }
            Ok(())
        }
        Some("xml") => {
            const MAX_ICON_XML_BYTES: u64 = 1_048_576;
            let metadata =
                fs::metadata(source).map_err(|error| format!("{}: {error}", source.display()))?;
            if metadata.len() > MAX_ICON_XML_BYTES {
                return Err(format!(
                    "Android drawable XML {} exceeds the 1 MiB limit",
                    source.display()
                ));
            }
            let contents = fs::read_to_string(source)
                .map_err(|error| format!("{}: {error}", source.display()))?;
            let document = roxmltree::Document::parse(&contents).map_err(|error| {
                format!(
                    "Android drawable XML {} is malformed: {error}",
                    source.display()
                )
            })?;
            let root = document.root_element().tag_name().name();
            if !matches!(
                root,
                "vector"
                    | "shape"
                    | "inset"
                    | "layer-list"
                    | "selector"
                    | "bitmap"
                    | "nine-patch"
                    | "scale"
                    | "rotate"
                    | "clip"
                    | "level-list"
                    | "transition"
                    | "animated-rotate"
                    | "animated-selector"
                    | "animated-vector"
                    | "ripple"
                    | "color"
            ) {
                return Err(format!(
                    "Android drawable XML {} must have a drawable root element (found `<{root}>`)",
                    source.display()
                ));
            }
            Ok(())
        }
        _ => Err(format!(
            "Android adaptive icon layer {} must be PNG or Android drawable XML",
            source.display()
        )),
    }
}

fn adaptive_icon_xml(resource_name: &str, include_monochrome: bool) -> String {
    let monochrome = if include_monochrome {
        format!("<monochrome android:drawable=\"@drawable/{resource_name}_monochrome\"/>")
    } else {
        String::new()
    };
    format!(
        "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\"><background android:drawable=\"@drawable/{resource_name}_background\"/><foreground android:drawable=\"@drawable/{resource_name}_foreground\"/>{monochrome}</adaptive-icon>\n"
    )
}

fn write_if_changed(path: &Path, contents: &str) -> Result<(), String> {
    super::file_ops::write_bytes_if_changed(path, contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        IconFontUsage, copy_android_project_fonts, copy_ios_project_fonts,
        generate_android_icon_named, generate_android_icon_set_named, load_font_catalog,
    };

    #[test]
    fn release_font_staging_subsets_icon_only_faces_and_preserves_a_face_used_by_text() {
        let project = nexa_testkit::TestProject::new("nexa-release-icon-fonts");
        let font = include_bytes!("../../../../examples/component-parity/assets/fonts/mazicon.ttf");
        std::fs::create_dir_all(project.join("assets/fonts")).expect("create fonts directory");
        std::fs::write(project.join("assets/fonts/mazicon.ttf"), font).expect("write font");
        std::fs::write(
            project.join("assets/fonts/mazicon.icons.json"),
            r#"{"play": 61696}"#,
        )
        .expect("write sidecar");
        let catalog = load_font_catalog(project.path(), &[]).expect("discover native font faces");
        let out = project.join("out");
        let android = out.join("android/app/src/main/res/font/mazicon.ttf");
        let ios = out.join("ios/IconProbe/Fonts/mazicon.ttf");

        for (contents, expect_subset) in [
            (
                r#"app IconProbe { body { Button("Play", icon: Icons.mazicon.play) {} } }"#,
                true,
            ),
            (
                r#"app IconProbe { body { Text("Play", fontFamily: "Mazicon") } }"#,
                false,
            ),
        ] {
            let entry = project.write_app(contents);
            let module = nexa_compiler::compile_file(&entry).expect("compile font usage");
            let usage = IconFontUsage::analyze(&module, &catalog);
            copy_android_project_fonts(project.path(), &out, Some(&usage)).expect("stage Android");
            copy_ios_project_fonts(project.path(), &out, "IconProbe", Some(&usage))
                .expect("stage iOS");
            let bytes = std::fs::read(&android).expect("read Android font");
            assert_eq!(bytes, std::fs::read(&ios).expect("read iOS font"));
            if expect_subset {
                assert!(bytes.len() < font.len() / 4);
            } else {
                assert_eq!(bytes, font);
            }
        }

        let entry = project.write_app(r#"app IconProbe { body { Text("Ready") } }"#);
        let module = nexa_compiler::compile_file(&entry).expect("compile without custom fonts");
        let usage = IconFontUsage::analyze(&module, &catalog);
        copy_android_project_fonts(project.path(), &out, Some(&usage)).expect("stage Android");
        copy_ios_project_fonts(project.path(), &out, "IconProbe", Some(&usage)).expect("stage iOS");
        assert!(!android.exists());
        assert!(!ios.exists());
    }

    #[test]
    fn project_font_catalog_discovers_family_weight_and_postscript_metadata() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/component-parity");
        let catalog = load_font_catalog(&project, &[]).expect("read packaged font metadata");
        let regular = catalog
            .resolve("Gilroy", nexa_ir::FontWeight::Normal)
            .expect("discover Gilroy regular face");
        let semibold = catalog
            .resolve("Gilroy", nexa_ir::FontWeight::Semibold)
            .expect("discover Gilroy semibold face");
        let mazicon = catalog
            .resolve("Mazicon", nexa_ir::FontWeight::Normal)
            .expect("discover single-face font");

        assert_eq!(regular.postscript_name, "Gilroy-Regular");
        assert_eq!(regular.android_resource_name, "gilroy_regular");
        assert_eq!(semibold.postscript_name, "Gilroy-Semibold");
        assert_eq!(semibold.android_resource_name, "gilroy_semibold");
        assert_eq!(mazicon.postscript_name, "Mazicon");
        assert_eq!(mazicon.android_resource_name, "mazicon");
    }

    #[test]
    fn project_font_config_can_alias_a_face_with_an_explicit_weight() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/component-parity");
        let config = nexa_syntax::parse_config(
            r#"config { assets { fonts { "Brand Display" {
                semibold: "assets/fonts/gilroy_semibold.ttf"
            } } } }"#,
        )
        .expect("font alias config parses");
        let aliases = config.assets.expect("assets").fonts;
        let catalog = load_font_catalog(&project, &aliases).expect("load configured font alias");
        let face = catalog
            .resolve("Brand Display", nexa_ir::FontWeight::Semibold)
            .expect("resolve alias");

        assert_eq!(face.postscript_name, "Gilroy-Semibold");
        assert_eq!(face.android_resource_name, "gilroy_semibold");
    }

    #[test]
    fn project_fonts_register_in_ios_and_android_resources() {
        let temp = nexa_testkit::TempDir::new("nexa-project-fonts");
        let source = temp.path().join("source");
        let output = temp.path().join("project");
        let font_directory = source.join("assets/fonts");
        std::fs::create_dir_all(&font_directory).expect("create font source directory");
        std::fs::write(font_directory.join("gilroy_regular.ttf"), b"regular font")
            .expect("write regular font");
        std::fs::write(font_directory.join("gilroy_medium.ttf"), b"medium font")
            .expect("write medium font");
        std::fs::write(font_directory.join("Mazicon.ttf"), b"icon font").expect("write icon font");
        let legacy_dev_fonts = output.join("android/app/src/main/assets/nexa/fonts");
        std::fs::create_dir_all(&legacy_dev_fonts).expect("create previous DevRuntime fonts");
        std::fs::write(
            legacy_dev_fonts.join("gilroy_regular.ttf"),
            b"old copied font",
        )
        .expect("write previous DevRuntime font");
        std::fs::write(
            legacy_dev_fonts.join(".nexa-project-fonts"),
            "gilroy_regular.ttf\n",
        )
        .expect("write previous DevRuntime font marker");

        let ios_fonts = copy_ios_project_fonts(&source, &output, "Demo", None)
            .expect("copy iOS font resources");
        copy_android_project_fonts(&source, &output, None).expect("copy Android font resources");

        assert_eq!(
            ios_fonts,
            [
                "Fonts/Mazicon.ttf".to_owned(),
                "Fonts/gilroy_medium.ttf".to_owned(),
                "Fonts/gilroy_regular.ttf".to_owned(),
            ]
        );
        assert!(output.join("ios/Demo/Fonts/gilroy_regular.ttf").is_file());
        assert!(output.join("ios/Demo/Fonts/gilroy_medium.ttf").is_file());

        let resources = output.join("android/app/src/main/res/font");
        assert!(resources.join("gilroy_regular.ttf").is_file());
        assert!(resources.join("gilroy_medium.ttf").is_file());
        assert!(resources.join("mazicon.ttf").is_file());
        assert!(
            !output
                .join("android/app/src/main/assets/nexa/fonts/gilroy_regular.ttf")
                .exists()
        );
        assert!(
            !output
                .join("android/app/src/main/assets/nexa/fonts/.nexa-project-fonts")
                .exists()
        );
    }

    #[test]
    fn adaptive_icon_set_generates_fallback_and_native_layered_resources() {
        let temp = nexa_testkit::TempDir::new("nexa-adaptive-icon");
        let assets = temp.path().join("assets");
        let shared = temp.path().join("shared");
        let output = temp.path().join("project");
        std::fs::create_dir_all(&assets).expect("create icon set");
        std::fs::create_dir_all(&shared).expect("create icon set");
        image::RgbaImage::from_pixel(64, 64, image::Rgba([25, 45, 65, 255]))
            .save(assets.join("icon.png"))
            .expect("write fallback icon");
        std::fs::write(assets.join("background.xml"), "<shape />").expect("write background layer");
        std::fs::write(shared.join("foreground.xml"), "<vector />")
            .expect("write shared foreground layer");
        std::fs::write(shared.join("monochrome.xml"), "<vector />")
            .expect("write shared monochrome layer");

        generate_android_icon_set_named(&assets, &output, "iconblue")
            .expect("generate adaptive icon set");

        let resources = output.join("android/app/src/main/res");
        let adaptive = std::fs::read_to_string(resources.join("mipmap-anydpi-v26/iconblue.xml"))
            .expect("read adaptive definition");
        let themed = std::fs::read_to_string(resources.join("mipmap-anydpi-v33/iconblue.xml"))
            .expect("read themed adaptive definition");
        assert!(adaptive.contains("@drawable/iconblue_background"));
        assert!(adaptive.contains("@drawable/iconblue_foreground"));
        assert!(!adaptive.contains("monochrome"));
        assert!(themed.contains("@drawable/iconblue_monochrome"));
        assert!(resources.join("drawable/iconblue_foreground.xml").is_file());
        assert!(resources.join("mipmap-xxxhdpi/iconblue.png").is_file());
        assert!(
            resources
                .join("mipmap-xxxhdpi/iconblue_round.png")
                .is_file()
        );
    }

    #[test]
    fn one_flat_bitmap_generates_legacy_icons_without_fake_adaptive_layers() {
        let temp = nexa_testkit::TempDir::new("nexa-flat-icon");
        let source = temp.path().join("icon.png");
        let output = temp.path().join("project");
        image::RgbaImage::from_pixel(64, 64, image::Rgba([25, 45, 65, 255]))
            .save(&source)
            .expect("write flat icon");

        generate_android_icon_named(&source, &output, "ic_launcher").expect("generate legacy icon");

        let resources = output.join("android/app/src/main/res");
        assert!(resources.join("mipmap-xxxhdpi/ic_launcher.png").is_file());
        assert!(!resources.join("mipmap-anydpi-v26/ic_launcher.xml").exists());
    }
}
