#[path = "../src/config.rs"]
mod config;

use config::ProjectConfig;

#[test]
fn staging_identity_uses_configured_suffix_and_keeps_base_identity() {
    let mut base = ProjectConfig::from_defaults(&[], "WeatherApp").unwrap();
    base.staging_suffix = "qa".to_owned();
    let staging = base.with_flavor("staging").unwrap();

    assert_eq!(base.ios_bundle_identifier, "com.nexa.weatherapp");
    assert_eq!(staging.ios_bundle_identifier, "com.nexa.weatherapp.qa");
    assert_eq!(staging.android_application_id, "com.nexa.weatherapp.qa");
    assert_eq!(staging.display_name, "WeatherApp Staging");
    assert_eq!(staging.android_target_sdk, 36);
}

#[test]
fn staging_identity_does_not_append_suffix_twice() {
    let mut base = ProjectConfig::from_defaults(&[], "WeatherApp").unwrap();
    base.ios_bundle_identifier.push_str(".staging");
    base.android_application_id.push_str(".staging");
    base.display_name.push_str(" Staging");

    let staging = base.with_flavor("staging").unwrap();
    assert_eq!(staging.ios_bundle_identifier, "com.nexa.weatherapp.staging");
    assert_eq!(
        staging.android_application_id,
        "com.nexa.weatherapp.staging"
    );
    assert_eq!(staging.display_name, "WeatherApp Staging");
}

#[test]
fn each_flavor_can_override_or_disable_its_application_id_suffix() {
    let mut base = ProjectConfig::from_defaults(&[], "WeatherApp").unwrap();
    base.flavors = vec![
        nexa_syntax::ast::FlavorConfig {
            name: "qa".to_owned(),
            suffix: Some("internal".to_owned()),
        },
        nexa_syntax::ast::FlavorConfig {
            name: "production".to_owned(),
            suffix: Some(String::new()),
        },
    ];

    let qa = base.with_flavor("qa").unwrap();
    let production = base.with_flavor("production").unwrap();
    assert_eq!(qa.android_application_id, "com.nexa.weatherapp.internal");
    assert_eq!(qa.ios_bundle_identifier, "com.nexa.weatherapp.internal");
    assert_eq!(production.android_application_id, "com.nexa.weatherapp");
    assert_eq!(production.ios_bundle_identifier, "com.nexa.weatherapp");
    assert_eq!(production.display_name, "WeatherApp Production");
}
