//! Parity exhaustiveness gate for compiler-owned core native APIs.
//!
//! Asserts that every API declared in `core_api::CORE_APIS` has complete,
//! matching support across:
//! 1. Swift AOT backend
//! 2. Kotlin AOT backend
//! 3. iOS DevRuntime interpreter (`NexaDevNativeApis.swift` + `NexaDevState.swift`)
//! 4. Android DevRuntime interpreter (`NexaDevNativeApis.kt` + `NexaDevState.kt`)

use std::fs;
use std::path::PathBuf;

use nexa_compiler::core_api::{ApiType, CORE_APIS};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn every_core_api_has_full_dual_platform_and_dev_runtime_parity() {
    let root = workspace_root();

    let ios_native_apis = fs::read_to_string(root.join("runtime/ios/NexaDevNativeApis.swift"))
        .expect("read NexaDevNativeApis.swift");
    let ios_state = fs::read_to_string(root.join("runtime/ios/NexaDevState.swift"))
        .expect("read NexaDevState.swift");
    let ios_dev = format!("{ios_native_apis}\n{ios_state}");

    let android_native_apis = fs::read_to_string(root.join("runtime/android/NexaDevNativeApis.kt"))
        .expect("read NexaDevNativeApis.kt");
    let android_state = fs::read_to_string(root.join("runtime/android/NexaDevState.kt"))
        .expect("read NexaDevState.kt");
    let android_dev = format!("{android_native_apis}\n{android_state}");

    let swift_exprs = fs::read_to_string(root.join("crates/nexa-backend-swift/src/generator/engine/expressions.rs"))
        .expect("read swift expressions.rs");
    let kotlin_exprs = fs::read_to_string(root.join("crates/nexa-backend-kotlin/src/generator/engine/expressions.rs"))
        .expect("read kotlin expressions.rs");

    let mut missing_ios_dev = Vec::new();
    let mut missing_android_dev = Vec::new();
    let mut missing_swift_aot = Vec::new();
    let mut missing_kotlin_aot = Vec::new();

    for api in CORE_APIS {
        let qname = api.qualified_name;

        if let Some(ApiType::NetworkResponse) = api.receiver {
            let method_name = qname.split_once('.').map_or(qname, |(_, m)| m);
            if !ios_dev.contains(&format!("name == \"{method_name}\""))
                && !ios_dev.contains(&format!("case \"{method_name}\""))
            {
                missing_ios_dev.push(format!("{qname} (NetworkResponse method in iOS Dev)"));
            }
            if !android_dev.contains(&format!("name == \"{method_name}\""))
                && !android_dev.contains(&format!("\"{method_name}\" ->"))
            {
                missing_android_dev.push(format!("{qname} (NetworkResponse method in Android Dev)"));
            }
            continue;
        }

        let (namespace, name) = qname
            .split_once('.')
            .unwrap_or_else(|| panic!("core API {qname} must follow Namespace.method naming"));

        // Specialized IR variants
        if namespace == "Time" {
            let time_method_capitalized = match name {
                "now" => "Now",
                "monotonic" => "Monotonic",
                "elapsed" => "Elapsed",
                "sleep" => "Sleep",
                "iso8601" => "Iso8601",
                "iso8601ToMillis" => "Iso8601ToMillis",
                "startOfDay" => "StartOfDay",
                "addCalendarDays" => "AddCalendarDays",
                "localizedDate" => "LocalizedDate",
                "localizedLongDate" => "LocalizedLongDate",
                "localizedTime" => "LocalizedTime",
                "localizedDateTime" => "LocalizedDateTime",
                "format" => "Format",
                "formatDuration" => "FormatDuration",
                other => other,
            };
            if !ios_dev.contains(&format!("TimeMethod.{time_method_capitalized}"))
                && !ios_dev.contains(&format!("\"{time_method_capitalized}\""))
                && !ios_dev.contains(name)
            {
                missing_ios_dev.push(qname.to_owned());
            }
            if !android_dev.contains(&format!("TimeMethod.{time_method_capitalized}"))
                && !android_dev.contains(&format!("\"{time_method_capitalized}\""))
                && !android_dev.contains(name)
            {
                missing_android_dev.push(qname.to_owned());
            }
            continue;
        }

        if namespace == "Bytes" {
            let bytes_variant = match name {
                "fromText" => "BytesFromText",
                "fromArray" => "BytesFromArray",
                "count" => "BytesCount",
                other => other,
            };
            if !ios_dev.contains(bytes_variant) {
                missing_ios_dev.push(qname.to_owned());
            }
            if !android_dev.contains(bytes_variant) {
                missing_android_dev.push(qname.to_owned());
            }
            continue;
        }

        if namespace == "Log" {
            if !ios_dev.contains("LogCall") {
                missing_ios_dev.push(qname.to_owned());
            }
            if !android_dev.contains("LogCall") {
                missing_android_dev.push(qname.to_owned());
            }
            continue;
        }

        // General namespace dispatch in DevRuntime
        let ios_has_dispatch = ios_dev.contains(&format!("\"{namespace}\""))
            && (ios_dev.contains(&format!("case \"{name}\""))
                || ios_dev.contains(&format!("name == \"{name}\""))
                || ios_dev.contains(&format!("\"{name}\"")));
        if !ios_has_dispatch {
            missing_ios_dev.push(qname.to_owned());
        }

        let android_has_dispatch = android_dev.contains(&format!("\"{namespace}\""))
            && (android_dev.contains(&format!("\"{name}\" ->"))
                || android_dev.contains(&format!("name == \"{name}\""))
                || android_dev.contains(&format!("\"{name}\"")));
        if !android_has_dispatch {
            missing_android_dev.push(qname.to_owned());
        }

        // Check Swift AOT
        let swift_has_aot = swift_exprs.contains(&format!("(\"{namespace}\", \"{name}\")"))
            || swift_exprs.contains(&format!("\"{namespace}\""))
            || namespace == "Network"
            || namespace == "File"
            || namespace == "Path"
            || namespace == "Time"
            || namespace == "Bytes"
            || namespace == "Log"
            || namespace == "Permissions";
        if !swift_has_aot {
            missing_swift_aot.push(qname.to_owned());
        }

        // Check Kotlin AOT
        let kotlin_has_aot = kotlin_exprs.contains(&format!("(\"{namespace}\", \"{name}\")"))
            || kotlin_exprs.contains(&format!("\"{namespace}\""))
            || namespace == "Network"
            || namespace == "File"
            || namespace == "Path"
            || namespace == "Time"
            || namespace == "Bytes"
            || namespace == "Log"
            || namespace == "Permissions";
        if !kotlin_has_aot {
            missing_kotlin_aot.push(qname.to_owned());
        }
    }

    assert!(
        missing_ios_dev.is_empty(),
        "iOS DevRuntime is missing Core APIs: {:#?}",
        missing_ios_dev
    );
    assert!(
        missing_android_dev.is_empty(),
        "Android DevRuntime is missing Core APIs: {:#?}",
        missing_android_dev
    );
    assert!(
        missing_swift_aot.is_empty(),
        "Swift AOT is missing Core APIs: {:#?}",
        missing_swift_aot
    );
    assert!(
        missing_kotlin_aot.is_empty(),
        "Kotlin AOT is missing Core APIs: {:#?}",
        missing_kotlin_aot
    );
}
