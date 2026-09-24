#[path = "../src/project/assets.rs"]
#[allow(dead_code)]
mod assets;

use std::{
    fs,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_ROOT_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn temp_root(prefix: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{nonce}-{sequence}",
        std::process::id()
    ))
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
    fs::remove_dir_all(root).expect("remove temporary asset directory");
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
    fs::remove_dir_all(root).expect("remove temporary asset directory");
}
