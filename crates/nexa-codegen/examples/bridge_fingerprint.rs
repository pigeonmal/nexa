//! Byte-identity harness for the C++ bridge renderers.
//!
//! Renders every render path for the shipped example contracts and for
//! shape-exercising synthetic contracts, then prints a stable digest per
//! artifact. Used to prove that modularising the C++ generator is a pure
//! code motion: digests must not change.

use nexa_codegen::plugin::bindings_cpp;
use nexa_codegen::plugin::bridge_plan::BridgePlan;
use nexa_plugin_idl::PluginIdl;

/// Declarations both adapter targets accept: plain structs, enums, and a
/// typed error. Shared so every fixture exercises the named-value, enum, and
/// error rendering paths.
const PRELUDE: &str = r#"
struct Point {
    x: Float64
    y: Float64
}

struct Pairish {
    origin: Point
    label: String
    ratio: Float64
}

enum Channel {
    left
    right
    both
}

enum Grade {
    low
    high
}

error BridgeFault {
    empty
    detail(reason: String, code: Int32)
    retryable(attempts: Int32)
}

service Plain {
    fn ping() -> Int32
    fn echo(name: String) -> String
    fn point(p: Point) -> Point
    fn pair(p: Pairish) -> Point
    fn channel() -> Channel
    fn grade(value: Int32) -> Grade
    fn counts() -> Array<Int32>
    fn flags() -> Set<Int32>
    fn lookup() -> Map<Int32, Int32>
    fn mixed() -> Map<Int32, Array<Int32>>
    fn text() -> Array<String>
    fn blobs() -> Array<Bytes>
    fn matrix() -> Array<Array<Float64>>
    fn nothing() -> Int32?
    fn maybeName(name: String?) -> String?
    async fn delayed(value: String) -> String
    async fn risky(value: Int32) throws BridgeFault
    async fn rated(value: Int32) -> Result<Int32, BridgeFault>
}

native class Recorder {
    init(label: String)
    readonly property label: String
    property gain: Float64
    property samples: Array<Int32>
    property point: Point
    property channel: Channel
    property nothing: Int32?
    fn capture(frame: Array<Int32>)
    async fn captureTyped(frame: Array<Int32>) throws BridgeFault
    async fn flush() throws BridgeFault
    fn dispose()
    event captured(count: Int32)
    event labelled(label: String, flag: Bool)
}
"#;

/// A native component keeps the Android C++ target out of scope (Android
/// adapters reject native-class references), so it lives in its own fixture
/// that still exercises the Swift adapter and contract paths.
const COMPONENT_ONLY: &str = r#"
native component RecorderView {
    content
    prop recorder: Recorder
    prop visible: Bool = true
    event tapped()
}
"#;

/// Shapes the Swift C++ adapter target accepts: a struct that mixes a nested
/// named value with collections, integer-keyed maps, nested arrays, `Bool`
/// leaf arrays (which need their own facade), and arrays of sets.
const SWIFT_ONLY: &str = r#"
struct Optionals {
    maybe: Int32?
    maybeName: String?
    maybePoint: Point?
}

struct Composite {
    origin: Point
    maybe: Int32?
    name: String
    blob: Bytes
    tag: String?
    counts: Array<Int32>
    grid: Array<Array<Int32>>
    flags: Set<Int32>
    weights: Map<Int32, Float64>
    bucketed: Map<Int32, Array<Int32>>
    deep: Map<Int32, Map<Int32, Float64>>
}

service Gallery {
    fn describe(shape: Composite) -> String
    fn optionals(shape: Optionals) -> Int32?
    fn grid() -> Array<Array<Int32>>
    fn masked() -> Array<Bool>
    fn grouped() -> Array<Set<Int32>>
    fn weights() -> Map<Int32, Float64>
    fn bucketed() -> Map<Int32, Array<Int32>>
    fn deep() -> Map<Int32, Map<Int32, Float64>>
    fn raw() -> Bytes
    fn nothing() -> Int32?
    fn maybeName(name: String?) -> String?
    fn point(p: Point) -> Point
    async fn analyse(shape: Composite) throws BridgeFault
    async fn history() -> Array<Int32>
    async fn table() -> Map<Int32, Float64>
    async fn rated(shape: Composite) -> Result<Int32, BridgeFault>
}
"#;

/// Shapes the Android C++ adapter target accepts: maps keyed by `String`,
/// and optional set values.
const ANDROID_ONLY: &str = r#"
service AndroidExtra {
    fn byName() -> Map<String, Int32>
    fn scored() -> Map<String, Float64>
    fn named() -> Map<String, String>
    fn setValues() -> Set<Int64>
    fn flat() -> Map<String, String>
}
"#;

fn digest(label: &str, body: &str) {
    if std::env::var_os("NEXA_FP_DUMP").is_some() {
        println!("===== {label} =====\n{body}\n===== end {label} =====");
        return;
    }
    // FNV-1a over the rendered artifact: stable across runs and platforms.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    println!("{label}\t{hash:016x}\t{}", body.len());
}

fn emit(label: &str, idl: &PluginIdl, plugin_id: &str) {
    let contract = match BridgePlan::validate_contract(idl) {
        Ok(plan) => plan,
        Err(error) => return digest(&format!("{label}/cpp"), &format!("ERR:{error}")),
    };
    digest(
        &format!("{label}/cpp"),
        &bindings_cpp::render(&contract, plugin_id),
    );

    match BridgePlan::validate_swift_cpp(idl) {
        Ok(plan) => match bindings_cpp::render_swift_adapters(&plan, plugin_id) {
            Ok(adapters) => digest(&format!("{label}/swift"), &adapters),
            Err(error) => digest(&format!("{label}/swift"), &format!("ERR:{error}")),
        },
        Err(error) => digest(&format!("{label}/swift"), &format!("ERR:{error}")),
    }

    match BridgePlan::validate_android(idl) {
        Ok(plan) => {
            match bindings_cpp::render_android_adapters(
                &plan,
                plugin_id,
                "Native",
                "dev.example.app",
                0,
            ) {
                Ok((kotlin, jni)) => {
                    digest(&format!("{label}/kotlin"), &kotlin);
                    digest(&format!("{label}/jni"), &jni);
                }
                Err(error) => digest(&format!("{label}/android"), &format!("ERR:{error}")),
            }
        }
        Err(error) => digest(&format!("{label}/android"), &format!("ERR:{error}")),
    }
}

fn main() {
    let android_fixture = format!("{PRELUDE}{ANDROID_ONLY}");
    let swift_fixture = format!("{PRELUDE}{COMPONENT_ONLY}{SWIFT_ONLY}");

    let fixtures: [(&str, &str, &str); 5] = [
        (
            "synthetic-android",
            android_fixture.as_str(),
            "dev.example.kitchen-sink",
        ),
        (
            "synthetic-swift",
            swift_fixture.as_str(),
            "dev.example.kitchen-sink",
        ),
        (
            "fast-math",
            include_str!("../../../examples/plugins/fast-math/native.nxid"),
            "dev.example.fast-math",
        ),
        (
            "video-player",
            include_str!("../../../examples/plugins/video-player/native.nxid"),
            "dev.example.video-player",
        ),
        (
            "fast-math-service",
            include_str!("../../../examples/plugins/fast-math/native.nxid"),
            "dev.example.video-player",
        ),
    ];

    for (label, source, plugin_id) in fixtures {
        let idl = nexa_plugin_idl::parse(source).unwrap_or_else(|error| {
            eprintln!("{label}: parse failed: {error}");
            std::process::exit(1);
        });
        emit(label, &idl, plugin_id);
    }
}
