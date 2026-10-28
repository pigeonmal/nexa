use std::{fs, path::PathBuf};

#[allow(dead_code)]
#[path = "../src/project/file_ops.rs"]
mod file_ops;

#[allow(dead_code)]
#[path = "../src/project/localization.rs"]
mod localization;

#[allow(dead_code)]
#[path = "../src/project/assets.rs"]
mod assets;

use nexa_testkit::TestProject;

fn add_image(project: &TestProject, name: &str, extension: &str) -> PathBuf {
    let source = project
        .path()
        .join("assets/images")
        .join(format!("{name}.{extension}"));
    if let Some(parent) = source.parent() {
        fs::create_dir_all(parent).expect("create source image directory");
    }
    image::RgbaImage::from_pixel(4, 4, image::Rgba([20, 40, 60, 255]))
        .save(&source)
        .expect("write valid source image");
    source
}

#[test]
fn app_images_are_copied_to_ios_catalog_and_android_drawables_deterministically() {
    let project = TestProject::new("nexa-app-assets");
    let source = add_image(&project, "hero_banner", "png");
    let webp_source = add_image(&project, "webp_banner", "webp");

    let output = project.path().join("generated");
    assert!(
        assets::copy_ios_project_images(project.path(), &output, "Demo").expect("copy iOS image")
    );
    assets::copy_android_project_images(project.path(), &output).expect("copy Android image");

    let ios_image = output.join("ios/Demo/Assets.xcassets/hero_banner.imageset/hero_banner.png");
    let ios_manifest = output.join("ios/Demo/Assets.xcassets/hero_banner.imageset/Contents.json");
    let android_image = output.join("android/app/src/main/res/drawable-nodpi/hero_banner.png");
    let ios_webp_image =
        output.join("ios/Demo/Assets.xcassets/webp_banner.imageset/webp_banner.png");
    let android_webp_image =
        output.join("android/app/src/main/res/drawable-nodpi/webp_banner.webp");
    assert_eq!(fs::read(&source).unwrap(), fs::read(&ios_image).unwrap());
    assert_eq!(
        fs::read(&source).unwrap(),
        fs::read(&android_image).unwrap()
    );
    assert_eq!(
        image::open(&webp_source).unwrap().to_rgba8(),
        image::open(&ios_webp_image).unwrap().to_rgba8()
    );
    assert_eq!(
        fs::read(&webp_source).unwrap(),
        fs::read(&android_webp_image).unwrap()
    );
    let manifest = fs::read_to_string(ios_manifest).unwrap();
    assert!(manifest.contains("hero_banner.png"));

    let ios_marker = output.join("ios/Demo/.nexa-project-images");
    let marker_before = fs::read(&ios_marker).unwrap();
    let unchanged_outputs = [
        ios_image.clone(),
        ios_webp_image.clone(),
        android_image.clone(),
        android_webp_image.clone(),
    ]
    .into_iter()
    .map(|path| {
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        (path, modified)
    })
    .collect::<Vec<_>>();
    std::thread::sleep(std::time::Duration::from_millis(25));
    assets::copy_ios_project_images(project.path(), &output, "Demo")
        .expect("regenerate iOS images");
    assets::copy_android_project_images(project.path(), &output)
        .expect("regenerate Android images");
    assert_eq!(marker_before, fs::read(ios_marker).unwrap());
    for (path, modified) in unchanged_outputs {
        assert_eq!(
            modified,
            fs::metadata(&path).unwrap().modified().unwrap(),
            "unchanged asset should not be rewritten: {}",
            path.display()
        );
    }

    image::RgbaImage::from_pixel(4, 4, image::Rgba([90, 80, 70, 255]))
        .save(&source)
        .expect("modify PNG source");
    image::RgbaImage::from_pixel(4, 4, image::Rgba([70, 80, 90, 255]))
        .save(&webp_source)
        .expect("modify WebP source");
    assets::copy_ios_project_images(project.path(), &output, "Demo")
        .expect("update changed iOS images");
    assets::copy_android_project_images(project.path(), &output)
        .expect("update changed Android images");
    assert_eq!(fs::read(&source).unwrap(), fs::read(&ios_image).unwrap());
    assert_eq!(
        fs::read(&source).unwrap(),
        fs::read(&android_image).unwrap()
    );
    assert_eq!(
        image::open(&webp_source).unwrap().to_rgba8(),
        image::open(&ios_webp_image).unwrap().to_rgba8()
    );
    assert_eq!(
        fs::read(&webp_source).unwrap(),
        fs::read(&android_webp_image).unwrap()
    );

    fs::remove_file(source).expect("remove source image");
    fs::remove_file(webp_source).expect("remove WebP source image");
    assert!(!assets::copy_ios_project_images(project.path(), &output, "Demo").unwrap());
    assets::copy_android_project_images(project.path(), &output).unwrap();
    assert!(!ios_image.exists());
    assert!(!ios_webp_image.exists());
    assert!(!android_image.exists());
    assert!(!android_webp_image.exists());
}

#[test]
fn app_image_names_and_formats_are_validated_before_copying() {
    let project = TestProject::new("nexa-app-assets");
    add_image(&project, "Uppercase", "png");
    let error = assets::copy_ios_project_images(project.path(), project.path(), "Demo")
        .expect_err("Android-incompatible image names should fail");
    assert!(error.contains("lowercase letter"));

    fs::remove_dir_all(project.path().join("assets/images")).unwrap();
    let image_dir = project.path().join("assets/images");
    fs::create_dir_all(&image_dir).unwrap();
    fs::write(image_dir.join("unknown.gif"), b"GIF89a").unwrap();
    let error = assets::copy_android_project_images(project.path(), project.path())
        .expect_err("unsupported source formats should fail");
    assert!(error.contains("unsupported app image asset format"));
}

#[test]
fn app_image_marker_rejects_root_relative_cleanup_entries() {
    let project = TestProject::new("nexa-app-assets");
    let output = project.path().join("generated");
    let catalog = output.join("ios/Demo/Assets.xcassets");
    let preserved = catalog.join("preserved.imageset");
    fs::create_dir_all(&preserved).unwrap();
    let marker = output.join("ios/Demo/.nexa-project-images");
    fs::write(&marker, ".\n").unwrap();

    let error = assets::copy_ios_project_images(project.path(), &output, "Demo")
        .expect_err("root-relative marker entries must not be cleaned up");
    assert!(error.contains("invalid generated asset marker entry"));
    assert!(preserved.is_dir());
}
