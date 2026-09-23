use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let path =
            env::temp_dir().join(format!("nexa-ios-cpp-maps-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("temporary test directory should be created");
        Self(path)
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn command_available(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn generated_ios_cpp_adapters_typecheck_maps_and_async_methods_when_swift_is_available() {
    if !command_available("swiftc", &["--version"]) {
        return;
    }

    let temp = TempProject::new();
    let plugin = temp.0.join("cpp-plugin");
    fs::create_dir_all(plugin.join("cpp/Sources")).expect("C++ source directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.cpp-map-adapters"
    version: "1.0.0"
    sources { native: "native.nxid" }
    cpp { standard: "c++20" sources: ["cpp/Sources/**"] }
}
"#,
    )
    .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        r#"service Lookup {
    fn ping()
    fn mapBool(values: Map<Bool, Bool>) -> Map<Bool, Bool>
    fn mapInt8(values: Map<Int8, Int8>) -> Map<Int8, Int8>
    fn mapInt16(values: Map<Int16, Int16>) -> Map<Int16, Int16>
    fn mapInt32(values: Map<Int32, Int32>) -> Map<Int32, Int32>
    fn mapInt64(values: Map<Int64, Int64>) -> Map<Int64, Int64>
    fn mapUInt8(values: Map<UInt8, UInt8>) -> Map<UInt8, UInt8>
    fn mapUInt16(values: Map<UInt16, UInt16>) -> Map<UInt16, UInt16>
    fn mapUInt32(values: Map<UInt32, UInt32>) -> Map<UInt32, UInt32>
    fn mapUInt64(values: Map<UInt64, UInt64>) -> Map<UInt64, UInt64>
    fn mapFloat32(values: Map<Bool, Float32>) -> Map<Bool, Float32>
    fn mapFloat64(values: Map<Int8, Float64>) -> Map<Int8, Float64>
    fn mapString(values: Map<Int16, String>) -> Map<Int16, String>
    fn echoPayloadSet(values: Set<Bytes>) -> Set<Bytes>
    fn echoNumbers(values: Map<Int32, String>) -> Map<Int32, String>
    fn echoPayloads(values: Map<Bytes, Bytes>) -> Map<Bytes, Bytes>
    fn echoPayloadSets(values: Map<Int32, Set<Bytes>>) -> Map<Int32, Set<Bytes>>
    fn echoNestedMaps(values: Map<Int32, Map<Int32, Bytes>>) -> Map<Int32, Map<Int32, Bytes>>
    fn echoNestedNumbers(values: Map<Int32, Array<Int32>>) -> Map<Int32, Array<Int32>>
    fn echoNestedLabels(values: Map<Int32, Array<String>>) -> Map<Int32, Array<String>>
    fn echoNestedIntegers(values: Array<Array<Int32>>) -> Array<Array<Int32>>
    fn echoNestedBooleans(values: Array<Array<Bool>>) -> Array<Array<Bool>>
    fn echoNestedStrings(values: Array<Array<String>>) -> Array<Array<String>>
    fn echoNestedPayloads(values: Array<Array<Bytes>>) -> Array<Array<Bytes>>
    async fn echoAsync(value: Int32) -> Int32
    async fn echoOptionalAsync(value: Int32?) -> Int32?
    async fn echoTextAsync(value: String) -> String
    async fn echoPayloadAsync(value: Bytes) -> Bytes
    async fn flushAsync()
}
native class Store {
    init(entries: Map<Int32, String>, payloads: Map<Bytes, Bytes>, history: Array<Array<Int32>>)
    property entries: Map<Int32, String>
    property payloads: Map<Bytes, Bytes>
    property nestedPayloads: Map<Int32, Map<Int32, Bytes>>
    property history: Array<Array<Int32>>
    fn echo(entries: Map<Int32, String>) -> Map<Int32, String>
    fn echoNestedMaps(values: Map<Int32, Map<Int32, Bytes>>) -> Map<Int32, Map<Int32, Bytes>>
    fn echoHistory(values: Array<Array<Int32>>) -> Array<Array<Int32>>
    async fn currentCount() -> Int32
    fn dispose()
}
"#,
    )
    .expect("native IDL should be written");
    fs::write(
        plugin.join("cpp/Sources/Plugin.cpp"),
        "#include \"NexaPluginBindings.hpp\"\n",
    )
    .expect("C++ source should be written");
    let entry = temp.0.join("main.nx");
    fs::write(
        &entry,
        "plugin \"cpp-plugin\" as Lookup\napp MapAdapter { body { Button(\"Ping\") { Lookup.ping() } } }\n",
    )
    .expect("Nexa app should be written");
    let output = temp.0.join("Generated");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "ios", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("CppMapAdapter")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "iOS map adapter project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );

    let ios = output.join("ios/CppMapAdapter");
    let bindings = ios.join("NexaPlugins/NexaPlugin0_Bindings.swift");
    let adapters = ios.join("NexaPlugins/NexaPlugin0_CppBindings.swift");
    let probe = temp.0.join("MapAdapterProbe.swift");
    fs::write(
        &probe,
        r#"import Foundation

@MainActor
func mapAdapterProbe() async {
    _ = await LookupPlugin.shared.echoAsync(value: 42)
    _ = await LookupPlugin.shared.echoOptionalAsync(value: nil)
    _ = await LookupPlugin.shared.echoTextAsync(value: "Nexa 🚀")
    _ = await LookupPlugin.shared.echoPayloadAsync(value: Data([0, 1, 255]))
    await LookupPlugin.shared.flushAsync()
    _ = LookupPlugin.shared.mapBool(values: [false: true, true: false])
    _ = LookupPlugin.shared.mapInt8(values: [-1: 1])
    _ = LookupPlugin.shared.mapInt16(values: [-2: 2])
    _ = LookupPlugin.shared.mapInt32(values: [-3: 3])
    _ = LookupPlugin.shared.mapInt64(values: [-4: 4])
    _ = LookupPlugin.shared.mapUInt8(values: [UInt8.max: 1])
    _ = LookupPlugin.shared.mapUInt16(values: [UInt16.max: 2])
    _ = LookupPlugin.shared.mapUInt32(values: [UInt32.max: 3])
    _ = LookupPlugin.shared.mapUInt64(values: [UInt64.max: 4])
    _ = LookupPlugin.shared.mapFloat32(values: [false: 1.25])
    _ = LookupPlugin.shared.mapFloat64(values: [-1: 2.5])
    _ = LookupPlugin.shared.mapString(values: [-2: "two"])
    _ = LookupPlugin.shared.echoPayloadSet(values: Set([Data([0, 1]), Data()]))
    let numbers: [Int32: String] = [1: "one", 2: "two"]
    let payloads = [Data([0, 1]): Data([255]), Data(): Data()]
    _ = LookupPlugin.shared.echoNumbers(values: numbers)
    _ = LookupPlugin.shared.echoPayloads(values: payloads)
    _ = LookupPlugin.shared.echoPayloadSets(values: [1: Set([Data([0, 1]), Data()])])
    let nestedPayloadMap: [Int32: [Int32: Data]] = [1: [10: Data([0, 1]), 11: Data()], 2: [:]]
    _ = LookupPlugin.shared.echoNestedMaps(values: nestedPayloadMap)
    let nestedNumbers: [Int32: [Int32]] = [1: [Int32.min, 0, Int32.max], 2: []]
    let nestedLabels: [Int32: [String]] = [1: ["Nexa 🚀", ""], 2: []]
    _ = LookupPlugin.shared.echoNestedNumbers(values: nestedNumbers)
    _ = LookupPlugin.shared.echoNestedLabels(values: nestedLabels)
    let nestedIntegers = [[Int32.min, 0, Int32.max], [], [-1, 7]]
    let nestedBooleans = [[false, true], [], [true]]
    let nestedStrings = [["Nexa 🚀", ""], [], ["last"]]
    let nestedPayloads = [[Data([0, 1]), Data()], [], [Data([255])]]
    _ = LookupPlugin.shared.echoNestedIntegers(values: nestedIntegers)
    _ = LookupPlugin.shared.echoNestedBooleans(values: nestedBooleans)
    _ = LookupPlugin.shared.echoNestedStrings(values: nestedStrings)
    _ = LookupPlugin.shared.echoNestedPayloads(values: nestedPayloads)
    let store = StoreImpl(entries: numbers, payloads: payloads, history: nestedIntegers)
    _ = store.entries
    store.entries = numbers
    _ = store.payloads
    store.payloads = payloads
    _ = store.nestedPayloads
    store.nestedPayloads = nestedPayloadMap
    _ = store.history
    store.history = nestedIntegers
    _ = store.echo(entries: numbers)
    _ = store.echoNestedMaps(values: nestedPayloadMap)
    _ = store.echoHistory(values: nestedIntegers)
    _ = await store.currentCount()
    store.dispose()
}
"#,
    )
    .expect("Swift map probe should be written");
    let bridge = ios.join("NexaPluginCpp-Bridging-Header.h");
    let checked = Command::new("swiftc")
        .args([
            "-typecheck",
            "-swift-version",
            "6",
            "-cxx-interoperability-mode=default",
            "-import-objc-header",
        ])
        .arg(&bridge)
        .args(["-Xcc", "-std=c++20", "-Xcc"])
        .arg(format!("-I{}", ios.display()))
        .arg(&bindings)
        .arg(&adapters)
        .arg(&probe)
        .output()
        .expect("swiftc should type-check generated map adapters");
    assert!(
        checked.status.success(),
        "generated iOS Swift/C++ map adapters failed to type-check:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
