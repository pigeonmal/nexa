use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

struct ProjectImage {
    name: String,
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
            image::open(&image.source)
                .map_err(|error| format!("{}: {error}", image.source.display()))?
                .save(&destination)
                .map_err(|error| format!("{}: {error}", destination.display()))?;
        } else {
            fs::copy(&image.source, &destination)
                .map_err(|error| format!("{}: {error}", image.source.display()))?;
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
    source_root: &Path,
    output_root: &Path,
) -> Result<(), String> {
    let images = project_images(source_root)?;
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
    for image in &images {
        let filename = if image.extension == "jpeg" {
            format!("{}.jpg", image.name)
        } else {
            format!("{}.{}", image.name, image.extension)
        };
        let destination = resources.join(&filename);
        ensure_owned_destination(&destination, &marker, &filename)?;
        fs::create_dir_all(&resources)
            .map_err(|error| format!("{}: {error}", resources.display()))?;
        fs::copy(&image.source, &destination)
            .map_err(|error| format!("{}: {error}", image.source.display()))?;
    }
    write_marker(&marker, &generated)
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

fn project_images(root: &Path) -> Result<Vec<ProjectImage>, String> {
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
    if fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{generate_android_icon_named, generate_android_icon_set_named};

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
