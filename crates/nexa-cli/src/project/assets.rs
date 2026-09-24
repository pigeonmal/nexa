use std::{fs, path::Path};

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
