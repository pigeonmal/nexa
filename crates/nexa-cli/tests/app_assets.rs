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
fn android_project_images_use_resource_ids_and_only_dynamic_names_need_keep_rules() {
    let project = TestProject::new("nexa-image-shrinking");
    let entry = project.path().join("App.nx");
    let output = project.path().join("generated");
    let main = output.join("android/app/src/main");
    let keep_file = main.join("res/raw/com_nexa_imageaudit_nexa_image_keep.xml");
    add_image(&project, "hero_banner", "png");
    add_image(&project, "when", "png");
    add_image(&project, "unused_banner", "png");
    fs::write(
        &entry,
        r#"app ImageAudit { body {
    Image(asset: "hero_banner.png", description: "Hero")
    Image(asset: "native_banner.png", description: "Native hero")
    Image(url: "https://example.test/hero.png", placeholder: "when.png", description: "Remote hero")
} }"#,
    )
    .unwrap();
    let generated = || {
        fs::read_dir(main.join("java/com/nexa/imageaudit"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "kt"))
            .map(|path| fs::read_to_string(path).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    };
    nexa_cli::generate_project(&entry, "android", &output, "ImageAudit").unwrap();

    let kotlin = generated();
    assert!(kotlin.contains("resourceId = R.drawable.`hero_banner`"));
    assert!(kotlin.contains("placeholderResourceId = R.drawable.`when`"));
    assert!(kotlin.contains(r#"resourceId = nexaDrawableId("native_banner")"#));
    assert!(kotlin.contains("context.resources.getIdentifier("));
    let keep = fs::read_to_string(&keep_file).unwrap();
    assert!(keep.contains(r#"tools:keep="@drawable/native_banner""#));

    let staged_native = add_image(&project, "native_banner", "png");
    nexa_cli::generate_project(&entry, "android", &output, "ImageAudit").unwrap();
    let kotlin = generated();
    assert!(kotlin.contains("resourceId = R.drawable.`native_banner`"));
    assert!(!kotlin.contains("context.resources.getIdentifier("));
    assert!(!keep_file.exists());

    fs::remove_file(staged_native).unwrap();
    nexa_cli::generate_project(&entry, "android", &output, "ImageAudit").unwrap();
    assert!(generated().contains(r#"resourceId = nexaDrawableId("native_banner")"#));
    assert!(keep_file.exists());
    assert!(!main.join("res/drawable-nodpi/native_banner.png").exists());

    fs::write(&entry, r#"app ImageAudit { body { Text("Catalog") } }"#).unwrap();
    nexa_cli::generate_project(&entry, "android", &output, "ImageAudit").unwrap();
    assert!(!keep_file.exists());
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
    assets::copy_android_project_images(&assets::project_images(project.path()).unwrap(), &output)
        .expect("copy Android image");

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
    assets::copy_android_project_images(&assets::project_images(project.path()).unwrap(), &output)
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
    assets::copy_android_project_images(&assets::project_images(project.path()).unwrap(), &output)
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
    assets::copy_android_project_images(&assets::project_images(project.path()).unwrap(), &output)
        .unwrap();
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
    fs::write(image_dir.join("unknown.bmp"), b"BM").unwrap();
    let error = assets::project_images(project.path())
        .err()
        .expect("unsupported source formats should fail");
    assert!(error.contains("unsupported app image asset format"));

    fs::remove_file(image_dir.join("unknown.bmp")).unwrap();
    fs::write(image_dir.join("spinner.gif"), b"GIF89a").unwrap();
    let images = assets::project_images(project.path()).expect("gif assets should be discovered");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].name, "spinner");
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
