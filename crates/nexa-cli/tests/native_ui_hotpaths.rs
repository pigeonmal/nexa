use std::process::Command;

fn run_native_check(script: &str) {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(script);
    let output = Command::new("python3")
        .arg(script)
        .output()
        .expect("Python should execute the native regression harness");
    assert!(
        output.status.success(),
        "native checks failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn native_ui_hotpaths_execute_when_toolchains_are_available() {
    run_native_check("native_ui_hotpaths.py");
}

#[test]
fn text_reveal_preserves_pixels_and_disposes_clock_when_simulator_is_available() {
    run_native_check("native_text_reveal_check.py");
}

#[test]
fn marquee_metrics_preserve_behavior_on_android_art_when_available() {
    run_native_check("native_marquee_check.py");
}

#[test]
fn section_headers_follow_uikit_lifecycles_when_simulator_is_available() {
    run_native_check("native_section_headers_check.py");
}

#[test]
fn marquee_disposal_is_terminal_when_simulator_is_available() {
    run_native_check("native_marquee_lifecycle_check.py");
}

#[test]
fn response_limits_cancel_native_transfers_before_buffering_oversized_bodies() {
    run_native_check("native_bounded_response_check.py");
}

#[test]
fn image_thumbnails_decode_off_main_actor_without_pixel_regressions() {
    run_native_check("native_image_decode_check.py");
}
