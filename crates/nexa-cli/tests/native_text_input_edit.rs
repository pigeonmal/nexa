//! Execute the shared native edit policy against one cross-platform corpus.
#[cfg(target_os = "macos")]
#[test]
fn swift_text_input_edit_policy_handles_unicode_and_line_limits() {
    use std::{fs, process::Command};
    let scratch = nexa_testkit::TempDir::new("nexa-text-input-edits");
    let source =
        include_str!("../../nexa-backend-swift/src/generator/components/text_input_edit.swift");
    let cases = include_str!("../../../tests/fixtures/text-input-edits.json");
    let script = format!(
        r##"
import Foundation
{source}
let data = #"""
{cases}
"""#.data(using: .utf8)!
let cases = try JSONSerialization.jsonObject(with: data) as! [[String: Any]]
for item in cases {{
    let actual = nexaTextInputEdit(
        previous: item["previous"] as! String,
        proposed: item["proposed"] as! String,
        maxLength: item["maxLength"] as? Int,
        maxLines: item["maxLines"] as? Int,
        multiline: item["multiline"] as! Bool
    )
    precondition(actual == item["expected"] as! String, "Failed: \(item["name"]!)")
}}
"##
    );
    let path = scratch.path().join("InputEdits.swift");
    fs::write(&path, script).expect("write native input policy regression");
    let result = Command::new("swift")
        .arg(path)
        .output()
        .expect("run Swift edit policy");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
