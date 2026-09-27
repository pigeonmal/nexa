//! The generated ISO 8601 layout, checked against the instants that break a
//! hand-rolled civil-date conversion.
//!
//! `Time.iso8601` and `Time.iso8601ToMillis` are emitted as Swift and Kotlin
//! source rather than computed in Rust, so a type check says nothing about
//! whether the arithmetic is right. A leap day, a date before the epoch, and
//! the end of a century are exactly the cases a civil-date conversion gets
//! wrong, so this test runs the emitted Swift and compares it with instants
//! whose layout is unambiguous.
//!
//! The test is skipped when no Swift toolchain is installed, so a machine
//! without Xcode still runs the suite; the same table is asserted against the
//! emitted Kotlin source textually in the Kotlin backend's own test.

use nexa_backend_swift::SwiftBackend;
use nexa_ir::{Expr, Module, Node, NumericType, TimeMethod, Type};

/// A module whose body reads the clock, so the clock support is emitted.
fn module() -> Module {
    Module {
        app_name: "ClockApp".to_owned(),
        plugins: Vec::new(),
        plugin_assets: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        states: Vec::new(),
        screens: Vec::new(),
        components: Vec::new(),
        body: vec![Node::OnAppear {
            actions: vec![nexa_ir::Action::Expression(Expr::TimeCall {
                method: TimeMethod::Iso8601,
                arguments: vec![Expr::Number {
                    raw: "0".to_owned(),
                    ty: NumericType::Int64,
                }],
                return_type: Type::String,
                is_async: false,
            })],
            asynchronous: false,
        }],
        status_bar: None,
        direction: None,
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
        on_active: None,
        on_inactive: None,
        on_background: None,
    }
}

/// The emitted clock helpers, without the rest of the generated host.
fn clock_support() -> String {
    let sources = SwiftBackend.generate_units(&module());
    let unit = sources
        .units
        .iter()
        .find(|unit| unit.name.contains("time"))
        .expect("a module that reads the clock emits the clock support");
    unit.contents.clone()
}

#[test]
fn the_emitted_iso8601_layout_matches_known_instants() {
    let support = clock_support();
    // A driver that formats each instant, parses it back, and prints one line
    // per case. `nexaIso8601` and `nexaIso8601ToMillis` are internal, so this
    // is compiled as a single file with the emitted support.
    // The unit is written without its import block, because the generated host
    // carries imports per file; this single-file driver has to declare the one
    // the clock uses.
    let driver = format!(
        r#"import Foundation

{support}
let cases: [(Int64, String)] = [
    (0, "1970-01-01T00:00:00.000Z"),
    (-1, "1969-12-31T23:59:59.999Z"),
    (1_700_000_000_000, "2023-11-14T22:13:20.000Z"),
    // 2024 is a leap year, and this is its last day.
    (1_735_689_599_999, "2024-12-31T23:59:59.999Z"),
    (1_709_164_800_123, "2024-02-29T00:00:00.123Z"),
    // 2100 is not a leap year, and 2000 was.
    (4_102_444_800_000, "2100-01-01T00:00:00.000Z"),
    (951_782_400_000, "2000-02-29T00:00:00.000Z"),
]
var failures: [String] = []
for (milliseconds, expected) in cases {{
    let formatted = nexaIso8601(milliseconds)
    if formatted != expected {{
        failures.append("format \(milliseconds): got \(formatted), want \(expected)")
    }}
    guard let parsed = nexaIso8601ToMillis(expected) else {{
        failures.append("parse \(expected): rejected")
        continue
    }}
    if parsed != milliseconds {{
        failures.append("parse \(expected): got \(parsed), want \(milliseconds)")
    }}
}}
// The fractional part is optional on the way in, and a malformed layout is
// rejected rather than guessed at.
if nexaIso8601ToMillis("2023-11-14T22:13:20Z") != 1_700_000_000_000 {{
    failures.append("parse without a fraction")
}}
if nexaIso8601ToMillis("2023-11-14 22:13:20Z") != nil {{
    failures.append("a space instead of `T` was accepted")
}}
if nexaIso8601ToMillis("not a timestamp") != nil {{
    failures.append("nonsense was accepted")
}}
if nexaIso8601ToMillis("2023-13-01T00:00:00Z") != nil {{
    failures.append("month 13 was accepted")
}}
// A day that the month does not have is malformed, not a date in the next one.
for rollover in [
    "2023-02-29T00:00:00Z",
    "2023-04-31T00:00:00Z",
    "2023-06-31T00:00:00Z",
    "2100-02-29T00:00:00Z",
] {{
    if nexaIso8601ToMillis(rollover) != nil {{
        failures.append("\(rollover) was accepted")
    }}
}}
// 2024 is a leap year, so its 29th of February is a real day.
if nexaIso8601ToMillis("2024-02-29T00:00:00Z") != 1_709_164_800_000 {{
    failures.append("2024-02-29 was rejected")
}}
if failures.isEmpty {{
    print("ok")
}} else {{
    print(failures.joined(separator: "\n"))
    exit(1)
}}
"#
    );

    let scratch = nexa_testkit::TempDir::new("nexa-iso8601-layout");
    let file = scratch.path().join("layout.swift");
    std::fs::write(&file, driver).expect("the driver should be written");
    let output = match std::process::Command::new("xcrun")
        .arg("swift")
        .arg(&file)
        .output()
    {
        Ok(output) => output,
        // No Swift toolchain on this machine: the layout is still covered by
        // the compiler and codegen tests, just not executed.
        Err(_) => return,
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    assert!(
        output.status.success(),
        "the emitted ISO 8601 layout is wrong:\n{stdout}\n{stderr}"
    );
    assert_eq!(stdout, "ok");
}
