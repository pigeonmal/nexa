use std::{fs, path::Path};

use nexa_testkit::TestProject;

#[test]
fn deep_link_config_generates_native_registration_and_typed_screen_routes() {
    let project = TestProject::new("nexa-deep-links");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/deep_links.nx");
    let entry = project.join("App.nx");
    fs::copy(source, &entry).expect("copy deep-link app fixture");
    project.write(
        "nexa.config.nx",
        r#"config {
            app { displayName: "Deep Links", deepLinks: ["nexa://", "https://links.example.com"] }
        }
        "#,
    );

    for target in [nexa_compiler::Target::Swift, nexa_compiler::Target::Kotlin] {
        nexa_compiler::compile_file_for_target(&entry, target)
            .expect("deep-link screen parameters should compile for both platforms");
    }

    let output = project.join("build");
    nexa_cli::generate_project(&entry, "all", &output, "DeepLinkDemo")
        .expect("generate deep-link native hosts");

    let ios_root = output.join("ios/DeepLinkDemo");
    let plist = fs::read_to_string(ios_root.join("Info.plist")).expect("read iOS Info.plist");
    assert!(plist.contains("<string>nexa</string>"));
    assert!(plist.contains("<key>CFBundleURLTypes</key>"));
    assert!(ios_root.join("Nexa.entitlements").is_file());
    let entitlements = fs::read_to_string(ios_root.join("Nexa.entitlements"))
        .expect("read iOS associated-domain entitlement");
    assert!(entitlements.contains("applinks:links.example.com"));
    let swift = TestProject::concat_sources_in(&ios_root, "swift");
    assert!(swift.contains("NavigationStack(path: $__nexaNavigationPath)"));
    assert!(swift.contains("case \"product-details\":"));
    assert!(swift.contains("guard segments.count == 3"));
    assert!(swift.contains("Int32(segments[2])"));
    assert!(swift.contains(".onOpenURL { url in"));

    let android_root = output.join("android/app/src/main");
    let manifest = fs::read_to_string(android_root.join("AndroidManifest.xml"))
        .expect("read Android manifest");
    assert!(manifest.contains("android:scheme=\"nexa\""));
    assert!(manifest.contains("android:autoVerify=\"true\""));
    assert!(manifest.contains("android:host=\"links.example.com\""));
    let kotlin = TestProject::concat_sources_in(&android_root.join("java"), "kt");
    assert!(kotlin.contains("LaunchedEffect(nexaDeepLink)"));
    assert!(kotlin.contains("product-details"));
    assert!(kotlin.contains("segments.size == 3"));
    assert!(kotlin.contains("segments[2].toIntOrNull()"));
    assert!(kotlin.contains("navController.navigate("));
}
