use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn scaffold_writes_valid_source_and_default_config() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nexa-create-{}-{nonce}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["create", "SampleApp", "--directory"])
        .arg(&root)
        .output()
        .expect("run Nexa scaffold command");
    assert!(output.status.success(), "{output:?}");

    let source = fs::read_to_string(root.join("App.nx")).expect("scaffold source");
    nexa_syntax::parse_program(&source).expect("scaffold app is valid");
    assert!(source.contains("Text(\"Count: $count\")"));
    assert!(!source.contains("${count}"));
    let config = fs::read_to_string(root.join("nexa.config.nx")).expect("scaffold config");
    let config = nexa_syntax::parse_config(&config).expect("scaffold config is valid");
    assert_eq!(config.android.expect("android config").target_sdk, Some(36));
    fs::remove_dir_all(root).expect("remove scaffold");
}
