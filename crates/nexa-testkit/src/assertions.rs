use std::path::Path;
use std::process::Output;

/// Asserts that a process output exited successfully (exit code 0), printing stdout and stderr on failure.
pub fn assert_output_success(output: &Output) {
    if !output.status.success() {
        panic!(
            "command failed with status {}:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Asserts that a process output exited with failure and contains `expected_needle` in stderr or stdout.
pub fn assert_output_failure(output: &Output, expected_needle: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded. Expected failure containing `{expected_needle}`.\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    assert!(
        stderr.contains(expected_needle) || stdout.contains(expected_needle),
        "output did not contain expected needle `{expected_needle}`.\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
}

/// Asserts that the file at `path` exists and contains `needle`.
pub fn assert_file_contains(path: &Path, needle: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    assert!(
        content.contains(needle),
        "file {} did not contain `{needle}`",
        path.display()
    );
}

/// Asserts that the file at `path` exists and does NOT contain `needle`.
pub fn assert_file_not_contains(path: &Path, needle: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    assert!(
        !content.contains(needle),
        "file {} unexpectedly contained `{needle}`",
        path.display()
    );
}
