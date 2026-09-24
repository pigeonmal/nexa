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
    let image = image::open(source).map_err(|error| format!("{}: {error}", source.display()))?;
    let res = root.join("android/app/src/main/res");
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
        icon.save(directory.join("ic_launcher.png"))
            .map_err(|error| error.to_string())?;
        icon.save(directory.join("ic_launcher_round.png"))
            .map_err(|error| error.to_string())?;
    }
    let foreground_dir = res.join("drawable-nodpi");
    fs::create_dir_all(&foreground_dir).map_err(|error| error.to_string())?;
    let mut foreground = image::RgbaImage::new(432, 432);
    let fit = image
        .resize_to_fill(288, 288, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    image::imageops::overlay(&mut foreground, &fit, 72, 72);
    foreground
        .save(foreground_dir.join("ic_launcher_foreground.png"))
        .map_err(|error| error.to_string())?;
    let mut monochrome = foreground.clone();
    for pixel in monochrome.pixels_mut() {
        pixel.0 = [255, 255, 255, pixel.0[3]];
    }
    monochrome
        .save(foreground_dir.join("ic_launcher_monochrome.png"))
        .map_err(|error| error.to_string())?;
    let adaptive_dir = res.join("mipmap-anydpi-v26");
    fs::create_dir_all(&adaptive_dir).map_err(|error| error.to_string())?;
    write_if_changed(
        &adaptive_dir.join("ic_launcher.xml"),
        "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\"><background android:drawable=\"@color/nexa_icon_background\"/><foreground android:drawable=\"@drawable/ic_launcher_foreground\"/></adaptive-icon>\n",
    )?;
    write_if_changed(
        &adaptive_dir.join("ic_launcher_round.xml"),
        "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\"><background android:drawable=\"@color/nexa_icon_background\"/><foreground android:drawable=\"@drawable/ic_launcher_foreground\"/></adaptive-icon>\n",
    )?;
    let themed_dir = res.join("mipmap-anydpi-v33");
    fs::create_dir_all(&themed_dir).map_err(|error| error.to_string())?;
    for name in ["ic_launcher.xml", "ic_launcher_round.xml"] {
        write_if_changed(
            &themed_dir.join(name),
            "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\"><background android:drawable=\"@color/nexa_icon_background\"/><foreground android:drawable=\"@drawable/ic_launcher_foreground\"/><monochrome android:drawable=\"@drawable/ic_launcher_monochrome\"/></adaptive-icon>\n",
        )?;
    }
    let values = res.join("values");
    fs::create_dir_all(&values).map_err(|error| error.to_string())?;
    write_if_changed(
        &values.join("nexa_icon_colors.xml"),
        "<resources><color name=\"nexa_icon_background\">#FFFFFFFF</color></resources>\n",
    )
}

fn write_if_changed(path: &Path, contents: &str) -> Result<(), String> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}
