#[path = "../src/project/assets.rs"]
#[allow(dead_code)]
mod assets;

use std::fs;

/// Claims a temporary asset root that is removed when the guard is dropped.
fn temp_root(prefix: &str) -> nexa_testkit::TempDir {
    nexa_testkit::TempDir::new(prefix)
}

fn write_test_icon(root: &std::path::Path) -> std::path::PathBuf {
    fs::create_dir_all(root).expect("temporary asset directory");
    let source = root.join("icon.png");
    image::RgbaImage::from_pixel(64, 64, image::Rgba([10, 20, 30, 255]))
        .save(&source)
        .expect("write test icon");
    source
}

#[test]
fn shared_raster_icon_generates_android_adaptive_round_and_themed_layers() {
    let root = temp_root("nexa-assets");
    let source = write_test_icon(&root);
    assets::generate_android_icon(&source, &root).expect("generate adaptive icon resources");

    let res = root.join("android/app/src/main/res");
    assert!(res.join("mipmap-mdpi/ic_launcher.png").is_file());
    assert!(res.join("mipmap-xxxhdpi/ic_launcher_round.png").is_file());
    let adaptive = fs::read_to_string(res.join("mipmap-anydpi-v26/ic_launcher.xml"))
        .expect("adaptive icon XML");
    assert!(adaptive.contains("ic_launcher_foreground"));
    let themed = fs::read_to_string(res.join("mipmap-anydpi-v33/ic_launcher.xml"))
        .expect("themed adaptive icon XML");
    assert!(themed.contains("<monochrome"));
}

#[test]
fn shared_raster_icon_generates_ios_asset_catalog_icon() {
    let root = temp_root("nexa-ios-icon");
    let source = write_test_icon(&root);
    assets::generate_ios_icon(&source, &root, "Demo").expect("generate iOS icon");

    let icon_dir = root.join("ios/Demo/Assets.xcassets/AppIcon.appiconset");
    let contents =
        fs::read_to_string(icon_dir.join("Contents.json")).expect("AppIcon asset catalog metadata");
    assert!(contents.contains("1024x1024"));
    assert!(icon_dir.join("AppIcon.png").is_file());
}

#[test]
fn ios_icon_composer_overrides_copy_packages_and_single_files() {
    let root = temp_root("nexa-icon-composer");
    let package = root.join("source/Brand.icon");
    fs::create_dir_all(package.join("Resources/Nested")).expect("create Icon Composer package");
    fs::write(package.join("document.json"), "{\"formatVersion\":1}")
        .expect("write Icon Composer document");
    fs::write(package.join("Resources/Nested/layer.bin"), [1, 2, 3])
        .expect("write nested Icon Composer resource");

    let destination = root.join("ios/Demo/AppIcon.icon");
    assets::copy_icon_composer(&package, &destination)
        .expect("copy Icon Composer package directory");
    assert_eq!(
        fs::read(destination.join("document.json")).expect("copied document"),
        b"{\"formatVersion\":1}"
    );
    assert_eq!(
        fs::read(destination.join("Resources/Nested/layer.bin")).expect("copied nested layer"),
        [1, 2, 3]
    );

    let file = root.join("source/Flat.icon");
    fs::write(&file, "icon file").expect("write file-form Icon Composer asset");
    assets::copy_icon_composer(&file, &destination)
        .expect("replace package with file-form Icon Composer asset");
    assert_eq!(
        fs::read(&destination).expect("copied icon file"),
        b"icon file"
    );
}
