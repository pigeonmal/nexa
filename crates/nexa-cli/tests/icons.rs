#[path = "../src/project/assets.rs"]
#[allow(dead_code)]
mod assets;

#[path = "../src/project/localization.rs"]
#[allow(dead_code)]
mod localization;

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
fn flat_raster_icon_only_generates_legacy_density_resources() {
    let root = temp_root("nexa-assets");
    let source = write_test_icon(&root);
    assets::generate_android_icon(&source, &root).expect("generate legacy icon resources");

    let res = root.join("android/app/src/main/res");
    assert!(res.join("mipmap-mdpi/ic_launcher.png").is_file());
    assert!(res.join("mipmap-xxxhdpi/ic_launcher_round.png").is_file());
    assert!(!res.join("mipmap-anydpi-v26/ic_launcher.xml").exists());
    assert!(!res.join("mipmap-anydpi-v33/ic_launcher.xml").exists());
}

#[test]
fn layered_android_icon_generates_adaptive_and_themed_layers() {
    let root = temp_root("nexa-layered-icon");
    let source = root.path().join("Icon");
    fs::create_dir_all(&source).expect("create Android icon set");
    write_test_icon(&source);
    fs::write(
        source.join("foreground.xml"),
        r##"<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108"><path android:fillColor="#FFFFFFFF" android:pathData="M 20,20 L 88,20 L 88,88 L 20,88 Z" /></vector>"##,
    )
    .expect("write foreground layer");
    fs::write(
        source.join("background.xml"),
        r##"<shape xmlns:android="http://schemas.android.com/apk/res/android" android:shape="rectangle"><solid android:color="#FF336699" /></shape>"##,
    )
    .expect("write background layer");
    fs::write(
        source.join("monochrome.xml"),
        r##"<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108"><path android:fillColor="#FFFFFFFF" android:pathData="M 20,20 L 88,20 L 88,88 L 20,88 Z" /></vector>"##,
    )
    .expect("write monochrome layer");

    assets::generate_android_icon(&source, root.path())
        .expect("generate layered adaptive icon resources");

    let res = root.path().join("android/app/src/main/res");
    let adaptive = fs::read_to_string(res.join("mipmap-anydpi-v26/ic_launcher.xml"))
        .expect("adaptive icon XML");
    let themed = fs::read_to_string(res.join("mipmap-anydpi-v33/ic_launcher.xml"))
        .expect("themed adaptive icon XML");
    assert!(adaptive.contains("@drawable/ic_launcher_foreground"));
    assert!(adaptive.contains("@drawable/ic_launcher_background"));
    assert!(!adaptive.contains("monochrome"));
    assert!(themed.contains("@drawable/ic_launcher_monochrome"));
    assert!(res.join("drawable/ic_launcher_foreground.xml").is_file());
    assert!(res.join("mipmap-xxxhdpi/ic_launcher_round.png").is_file());
}

#[test]
fn android_icon_generation_removes_stale_layered_and_monochrome_outputs() {
    let root = temp_root("nexa-icon-cleanup");
    let icon_set = root.path().join("Icon");
    fs::create_dir_all(&icon_set).expect("create icon set");
    write_test_icon(&icon_set);
    fs::write(
        icon_set.join("foreground.xml"),
        r##"<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108"><path android:fillColor="#FFFFFFFF" android:pathData="M 20,20 L 88,20 L 88,88 L 20,88 Z" /></vector>"##,
    )
    .expect("write foreground layer");
    fs::write(
        icon_set.join("background.xml"),
        r##"<shape xmlns:android="http://schemas.android.com/apk/res/android" android:shape="rectangle"><solid android:color="#FF336699" /></shape>"##,
    )
    .expect("write background layer");
    fs::write(
        icon_set.join("monochrome.xml"),
        r##"<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108"><path android:fillColor="#FFFFFFFF" android:pathData="M 20,20 L 88,20 L 88,88 L 20,88 Z" /></vector>"##,
    )
    .expect("write monochrome layer");

    assets::generate_android_icon_named(&icon_set, root.path(), "ic_launcher")
        .expect("generate layered icon");
    fs::remove_file(icon_set.join("monochrome.xml")).expect("remove monochrome layer");
    assets::generate_android_icon_named(&icon_set, root.path(), "ic_launcher")
        .expect("regenerate without monochrome layer");

    let res = root.path().join("android/app/src/main/res");
    assert!(!res.join("mipmap-anydpi-v33/ic_launcher.xml").exists());
    assert!(!res.join("drawable/ic_launcher_monochrome.xml").exists());

    let flat = write_test_icon(&root.path().join("flat"));
    assets::generate_android_icon_named(&flat, root.path(), "ic_launcher")
        .expect("replace adaptive icon with flat fallback");
    assert!(!res.join("mipmap-anydpi-v26/ic_launcher.xml").exists());
    assert!(!res.join("drawable/ic_launcher_foreground.xml").exists());
    assert!(res.join("mipmap-xxxhdpi/ic_launcher.png").is_file());
}

#[test]
fn android_drawable_xml_is_well_formed_and_has_a_drawable_root() {
    let root = temp_root("nexa-invalid-icon-xml");
    let source = root.path().join("Icon");
    fs::create_dir_all(&source).expect("create Android icon set");
    write_test_icon(&source);
    fs::write(
        source.join("foreground.xml"),
        "<manifest><broken></manifest>",
    )
    .expect("write malformed drawable");
    fs::write(source.join("background.xml"), "<shape />").expect("write background");

    let error = assets::generate_android_icon(&source, root.path())
        .expect_err("malformed drawable XML should fail before writing output");
    assert!(error.contains("is malformed"));
    assert!(
        !root
            .path()
            .join("android/app/src/main/res/mipmap-mdpi/ic_launcher.png")
            .exists()
    );

    fs::write(source.join("foreground.xml"), "<manifest />").expect("write invalid drawable root");
    let error = assets::generate_android_icon(&source, root.path())
        .expect_err("non-drawable XML root should fail");
    assert!(error.contains("must have a drawable root element"));
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
