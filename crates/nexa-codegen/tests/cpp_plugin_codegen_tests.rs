use nexa_codegen::plugin::bindings_cpp::{render, render_android_adapters, render_swift_adapters};
use nexa_codegen::plugin::bridge_plan::BridgePlan;

#[test]
fn cpp_contract_maps_native_values_and_typed_async_errors() {
    let idl = nexa_plugin_idl::parse(
        r#"
            struct PlayerOptions { autoplay: Bool }
            enum PlayerState { idle, playing }
            error PlayerError { invalidUrl, decodingFailed(message: String) }
            service Codec { fn checksum(data: Bytes) -> UInt64 }
            native class VideoPlayer {
                init(options: PlayerOptions)
                readonly property state: PlayerState
            property volume: Float64
                event ended()
                async fn prepare(url: String) throws PlayerError
            }
            "#,
    )
    .expect("native IDL should parse");

    let cpp = render(
        &BridgePlan::validate_contract(&idl).expect("contract should validate"),
        "dev.example.video-player",
    );
    assert!(
        cpp.contains("namespace plugin_dev::plugin_example::plugin_video_dash_player"),
        "unexpected generated namespace:\n{cpp}"
    );
    assert!(cpp.contains("std::vector<std::uint8_t> data"));
    assert!(cpp.contains("std::uint64_t checksum"));
    assert!(cpp.contains("virtual double getVolume() const noexcept = 0;"));
    assert!(cpp.contains("virtual void setVolume(double value) noexcept = 0;"));
    assert!(cpp.contains("virtual std::future<NexaResult<void, PlayerError>> prepare"));
    assert!(cpp.contains("virtual void setOnEnded(std::function<void()> handler) noexcept = 0;"));
    assert!(cpp.contains("std::unique_ptr<VideoPlayerSpec> makeVideoPlayerImpl"));
    assert!(cpp.contains("SWIFT_SHARED_REFERENCE(.nexaRetainForSwift, .nexaReleaseFromSwift)"));
    assert!(cpp.contains("nexaMakeVideoPlayerForSwift"));
    assert!(cpp.contains("using NexaCppByteBuffer = std::vector<std::uint8_t>;"));
}

#[test]
fn swift_cpp_byte_alias_avoids_plugin_type_names() {
    let idl = nexa_plugin_idl::parse(
            "struct NexaCppByteBuffer { value: Int32 } struct NexaCppByteBuffer1 { value: Int32 } service Codec { fn echo(data: Bytes) -> Bytes }",
        )
        .expect("contract names should parse");
    let cpp = render(
        &BridgePlan::validate_contract(&idl).expect("contract should validate"),
        "dev.example.cpp-plugin",
    );
    let swift = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&idl).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("byte adapter should generate");
    assert!(cpp.contains("using NexaCppByteBuffer2 = std::vector<std::uint8_t>;"));
    assert!(
        swift.contains("plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppByteBuffer2(data)")
    );
}

#[test]
fn swift_cpp_nested_boolean_array_converters_are_emitted_inner_first() {
    let idl = nexa_plugin_idl::parse(
            "service Types { fn flat(values: Array<Bool>) -> Array<Bool> fn nested(values: Array<Array<Bool>>) -> Array<Array<Bool>> }",
        )
        .expect("Boolean array IDL should parse");
    let cpp = render(
        &BridgePlan::validate_contract(&idl).expect("contract should validate"),
        "dev.example.cpp-plugin",
    );
    let inner_converter = cpp
        .find("inline std::vector<bool> nexaCppArrayToNativeBool(")
        .expect("flat Boolean converter should be generated");
    let nested_converter = cpp
        .find("inline std::vector<std::vector<bool>> nexaCppArrayToNativeArrayBool(")
        .expect("nested Boolean converter should be generated");

    assert!(
        inner_converter < nested_converter,
        "nested converter must follow the converter it calls"
    );
}

#[test]
fn swift_cpp_adapters_bridge_async_scalar_string_and_byte_methods() {
    let idl = nexa_plugin_idl::parse(
        r#"
            service Counter {
                fn echo(value: Int32) -> Int32
                fn echoText(value: String) -> String
                fn echoBytes(value: Bytes) -> Bytes
                async fn echoAsync(value: Int32) -> Int32
                async fn echoOptionalAsync(value: Int32?) -> Int32?
                async fn echoTextAsync(value: String) -> String
                async fn echoBytesAsync(value: Bytes) -> Bytes
                async fn flush()
                fn echoUByte(value: UInt8) -> UInt8
                fn echoUShort(value: UInt16) -> UInt16
                fn echoUInt(value: UInt32) -> UInt32
                fn echoULong(value: UInt64) -> UInt64
            }
            native class Meter {
                init(initial: Int32, label: String)
                property value: Float64
                property label: String
                property payload: Bytes
                fn increment()
                fn echo(value: String) -> String
                fn echoBytes(value: Bytes) -> Bytes
                async fn currentValue() -> Int32
            }
            "#,
    )
    .expect("native IDL should parse");

    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&idl).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("scalar synchronous C++ contracts should generate Swift adapters");
    assert!(adapters.contains("public final class CounterPlugin: Counter, @unchecked Sendable"));
    assert!(
        adapters.contains("plugin_dev.plugin_example.plugin_cpp_dash_plugin.Counter.echo(value)")
    );
    assert!(adapters.contains("public final class MeterImpl: MeterSpec"));
    assert!(adapters.contains("nexaMakeMeterForSwift(initial, std.string(label))"));
    assert!(adapters.contains("nexaCppObject.setValue(newValue)"));
    assert!(adapters.contains("get { String(nexaCppObject.getLabel()) }"));
    assert!(adapters.contains("set { nexaCppObject.setLabel(std.string(newValue)) }"));
    assert!(adapters.contains("payload: Data"));
    assert!(adapters.contains("get { Data(nexaCppObject.getPayload()) }"));
    assert!(adapters.contains(
            "set { nexaCppObject.setPayload(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppByteBuffer(newValue)) }"
        ));
    assert!(adapters.contains(
            "return Data(plugin_dev.plugin_example.plugin_cpp_dash_plugin.Counter.echoBytes(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppByteBuffer(value)))"
        ));
    assert!(adapters.contains(
            "return Data(nexaCppObject.echoBytes(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppByteBuffer(value)))"
        ));
    assert!(adapters.contains(
            "return String(plugin_dev.plugin_example.plugin_cpp_dash_plugin.Counter.echoText(std.string(value)))"
        ));
    assert!(adapters.contains("return String(nexaCppObject.echo(std.string(value)))"));
    assert!(
        adapters.contains("private final class NexaCppFutureWork<Output>: @unchecked Sendable")
    );
    assert!(adapters.contains("public func echoAsync(_ value: Int32) async -> Int32"));
    assert!(adapters.contains("public func echoOptionalAsync(_ value: Int32?) async -> Int32?"));
    assert!(adapters.contains("public func echoTextAsync(_ value: String) async -> String"));
    assert!(adapters.contains("public func echoBytesAsync(_ value: Data) async -> Data"));
    assert!(adapters.contains("public func flush() async -> Void"));
    assert!(adapters.contains("return await withCheckedContinuation"));
    assert!(adapters.contains("public func currentValue() async -> Int32"));

    let synchronous = nexa_plugin_idl::parse("service Clock { fn now() -> Int64 }")
        .expect("synchronous native IDL should parse");
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&synchronous).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("synchronous native contract should generate Swift adapters");
    assert!(!adapters.contains("NexaCppFutureWork"));

    let asynchronous = nexa_plugin_idl::parse("service Clock { async fn now() -> Int64 }")
        .expect("async native IDL should parse");
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&asynchronous).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("async scalar C++ signatures should generate Swift adapters");
    assert!(adapters.contains("public func now() async -> Int64"));

    let throwing = nexa_plugin_idl::parse(
            "error Failure { rejected, malformed(message: String, payload: Bytes) } service Clock { async fn read() throws Failure async fn readValue() -> Result<Int32, Failure> }",
        )
        .expect("throwing native IDL should parse");
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&throwing).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("typed async C++ errors should generate Swift adapters");
    assert!(adapters.contains("public func read() async throws(Failure) -> Void"));
    assert!(adapters.contains("public func readValue() async throws(Failure) -> Int32"));
    assert!(adapters.contains("Failure.malformed(message: String("));
    assert!(adapters.contains("payload: Data("));
    assert!(adapters.contains("return try nexaCppTypedResult.get()"));

    let throwing_collection = nexa_plugin_idl::parse(
            "error Failure { rejected } service Clock { async fn values() -> Result<Array<Int32>, Failure> }",
        )
        .expect("async typed-collection IDL should parse");
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&throwing_collection)
            .expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("typed async array returns should use the existing vector converter");
    assert!(adapters.contains("async throws(Failure) -> [Int32]"));

    let async_collection =
            nexa_plugin_idl::parse("service Clock { async fn history() -> Array<Int32> async fn count(values: Array<Int32>) -> Int32 }")
                .expect("async collection native IDL should parse");
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&async_collection).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("async collection signatures should use the existing vector converters");
    assert!(adapters.contains("public func history() async -> [Int32]"));
    assert!(adapters.contains("public func count(_ values: [Int32]) async -> Int32"));

    let events = nexa_plugin_idl::parse(
        "native class Watcher { init() event changed(value: Int32) fn dispose() }",
    )
    .expect("native class event IDL should parse");
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&events).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("Swift event adapters should preserve instance-scoped callbacks");
    assert!(adapters.contains("public var onChanged: ((Int32) -> Void)?"));
    assert!(adapters.contains("nexaCppEventInvokedev_example_cpp_plugin_Watcher_changed"));
}

#[test]
fn swift_cpp_adapters_bridge_optional_scalars_strings_and_bytes() {
    let idl = nexa_plugin_idl::parse(
        r#"
            struct NexaCppOptionalInt32 { value: Int32 }
            service Lookup {
                fn find(key: Int32?) -> Int32?
                fn maybeText(value: String?) -> String?
                fn maybeBytes(value: Bytes?) -> Bytes?
                fn echoValues(values: Array<Int32>) -> Array<Int32>
                fn echoLabels(values: Array<String>) -> Array<String>
            }
            native class Record {
                init(initial: Int32?)
                property maybeValue: Int32?
                property maybeTitle: String?
                property maybePayload: Bytes?
                fn maybeEcho(value: String?) -> String?
            }
            "#,
    )
    .expect("optional native IDL should parse");

    let cpp = render(
        &BridgePlan::validate_contract(&idl).expect("contract should validate"),
        "dev.example.cpp-plugin",
    );
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&idl).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("optional scalar, string, and byte adapters should generate");
    assert!(cpp.contains("using NexaCppOptionalInt321 = std::optional<std::int32_t>;"));
    assert!(cpp.contains("static NexaCppOptionalInt321 someInt32(std::int32_t value) noexcept"));
    assert!(adapters.contains("public func find(_ key: Int32?) -> Int32?"));
    assert!(adapters.contains(
            "key.map { plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppOptionalBridge.someInt32($0) } ?? plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppOptionalBridge.noneInt32()"
        ));
    assert!(adapters.contains(
        "return Optional(fromCxx: plugin_dev.plugin_example.plugin_cpp_dash_plugin.Lookup.find("
    ));
    assert!(adapters.contains("public func maybeText(_ value: String?) -> String?"));
    assert!(adapters.contains("value.map { plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppOptionalBridge.someString(std.string($0)) }"));
    assert!(
        adapters.contains(
            "Optional(fromCxx: plugin_dev.plugin_example.plugin_cpp_dash_plugin.Lookup.maybeText("
        ) && adapters.contains(".map { String($0) }")
    );
    assert!(adapters.contains("value.map { plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppOptionalBridge.someBytes(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppByteBuffer($0)) }"));
    assert!(adapters.contains(".map { Data($0) }"));
    assert!(adapters.contains("required public init(_ initial: Int32?)"));
    assert!(cpp.contains("using NexaCppArrayInt32 = std::vector<std::int32_t>;"));
    assert!(adapters.contains("public func echoValues(_ values: [Int32]) -> [Int32]"));
    assert!(
        adapters
            .contains("plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppArrayInt32(values)")
    );
    assert!(adapters.contains(
            "return Array(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppArrayInt32(plugin_dev.plugin_example.plugin_cpp_dash_plugin.Lookup.echoValues(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppArrayInt32(values))))"
        ));
    assert!(adapters.contains("values.map { std.string($0) }"));
    assert!(adapters.contains(".map { String($0) }"));
    assert!(adapters.contains("public var maybeTitle: String?"));
    assert!(
        adapters.contains(
            "get { Optional(fromCxx: nexaCppObject.getMaybeTitle()).map { String($0) } }"
        )
    );
    assert!(adapters.contains("set { nexaCppObject.setMaybeTitle(newValue.map { plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppOptionalBridge.someString(std.string($0)) } ?? plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppOptionalBridge.noneString()) }"));
    assert!(adapters.contains("public var maybePayload: Data?"));
    assert!(
        adapters.contains(
            "get { Optional(fromCxx: nexaCppObject.getMaybePayload()).map { Data($0) } }"
        )
    );
}

#[test]
fn swift_cpp_adapters_bridge_compatible_sets_through_vector_facades() {
    let idl = nexa_plugin_idl::parse(
        r#"
            service Lookup {
                fn integers(values: Set<Int32>) -> Set<Int32>
                fn payloads(values: Set<Bytes>) -> Set<Bytes>
            }
            native class Basket {
                init(values: Set<Int32>)
                property values: Set<Int32>
                fn transform(values: Set<Int32>) -> Set<Int32>
            }
            "#,
    )
    .expect("compatible native Set IDL should parse");

    let cpp = render(
        &BridgePlan::validate_contract(&idl).expect("contract should validate"),
        "dev.example.cpp-plugin",
    );
    let adapters = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&idl).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("compatible native Set values should generate Swift adapters");

    assert!(cpp.contains("using NexaCppArrayInt32 = std::vector<std::int32_t>;"));
    assert!(cpp.contains("using NexaCppArrayBytes = std::vector<std::vector<std::uint8_t>>;"));
    assert!(
        cpp.contains("std::set<std::int32_t> integers(std::set<std::int32_t> values) noexcept;")
    );
    assert!(cpp.contains("inline NexaCppArrayInt32 nexaSwiftAdapter_Lookup_integers(NexaCppArrayInt32 values) noexcept"));
    assert!(cpp.contains("std::set<std::int32_t>(values.begin(), values.end())"));
    assert!(cpp.contains("NexaCppArrayInt32(value.begin(), value.end())"));
    assert!(cpp.contains(
        "NexaCppArrayInt32 nexaSwiftAdapter_transform(NexaCppArrayInt32 values) noexcept"
    ));
    assert!(cpp.contains("nexaSwiftAdapterGetvalues() const noexcept"));
    assert!(cpp.contains("nexaSwiftAdapterSetvalues(NexaCppArrayInt32 value) noexcept"));
    assert!(cpp.contains("nexaMakeBasketForSwift(NexaCppArrayInt32 values) noexcept"));

    assert!(adapters.contains("public func integers(_ values: Set<Int32>) -> Set<Int32>"));
    assert!(adapters.contains(
        "plugin_dev.plugin_example.plugin_cpp_dash_plugin.nexaSwiftAdapter_Lookup_integers("
    ));
    assert!(adapters.contains("NexaCppArrayInt32(Array(values))"));
    assert!(adapters.contains(
        "return Set(Array(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppArrayInt32("
    ));
    assert!(adapters.contains("public func payloads(_ values: Set<Data>) -> Set<Data>"));
    assert!(adapters.contains(
        "values.map { plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppByteBuffer($0) }"
    ));
    assert!(
        adapters.contains(
            "Set(Array(plugin_dev.plugin_example.plugin_cpp_dash_plugin.NexaCppArrayBytes("
        )
    );
    assert!(adapters.contains(".map { Data($0) }"));
    assert!(adapters.contains("required public init(_ values: Set<Int32>)"));
    assert!(adapters.contains("nexaCppObject.nexaSwiftAdapterGetvalues()"));
    assert!(adapters.contains("nexaCppObject.nexaSwiftAdapterSetvalues("));
    assert!(adapters.contains("public func transform(_ values: Set<Int32>) -> Set<Int32>"));

    for unsupported in ["Set<String>", "Set<Float64>", "Set<Int32?>"] {
        let unsupported_idl = nexa_plugin_idl::parse(&format!(
            "service Lookup {{ fn values(items: {unsupported}) -> Int32 }}"
        ))
        .expect("unsupported Set shape should still be valid IDL");
        let error = BridgePlan::validate_swift_cpp(&unsupported_idl)
            .expect_err("Set semantics that differ across Swift and C++ must be rejected");
        assert!(error.contains("compatible `Set` values"), "{error}");
    }

    let collision_idl = nexa_plugin_idl::parse(
        r#"
            native class Collision {
                fn transform(values: Set<Int32>) -> Set<Int32>
                fn nexaSwiftAdapter_transform() -> Int32
            }
            "#,
    )
    .expect("adapter name collision fixture should parse");
    let collision_cpp = render(
        &BridgePlan::validate_contract(&collision_idl).expect("contract should validate"),
        "dev.example.cpp-plugin",
    );
    let collision_swift = render_swift_adapters(
        &BridgePlan::validate_swift_cpp(&collision_idl).expect("swift contract should validate"),
        "dev.example.cpp-plugin",
    )
    .expect("valid colliding user method should still generate");
    assert!(
        collision_cpp.contains("nexaSwiftAdapter_transform1(NexaCppArrayInt32 values) noexcept")
    );
    assert!(collision_swift.contains("nexaCppObject.nexaSwiftAdapter_transform1("));
}

#[test]
fn swift_cpp_adapters_reject_map_shapes_without_safe_cross_platform_keys() {
    for unsupported in [
        "Map<String, Int32>",
        "Map<Float64, Int32>",
        "Map<Int32?, String>",
        "Map<Int32, String?>",
        "Map<Int32, Map<String, String>>",
        "Map<Int32, String>?",
    ] {
        let idl = nexa_plugin_idl::parse(&format!(
            "service Lookup {{ fn values(items: {unsupported}) -> Int32 }}"
        ))
        .expect("unsupported map shape should remain valid IDL");
        let error = BridgePlan::validate_swift_cpp(&idl)
            .expect_err("unsupported C++ map shapes must fail generation");
        assert!(error.contains("supported-key `Map`"), "{error}");
    }

    {
        let unsupported = "Array<Int32?>";
        let idl = nexa_plugin_idl::parse(&format!(
            "service Lookup {{ fn values(items: {unsupported}) -> Int32 }}"
        ))
        .expect("unsupported array shape should remain valid IDL");
        let error = BridgePlan::validate_swift_cpp(&idl)
            .expect_err("unsupported nested array shapes must fail generation");
        assert!(error.contains("`Array` values"), "{error}");
    }
}

#[test]
fn android_cpp_adapters_generate_typed_kotlin_jni_and_owned_native_classes() {
    let idl = nexa_plugin_idl::parse(
        r#"
            service Counter {
                fn echo(value: Int32) -> Int32
                async fn echoAsync(value: Int32) -> Int32
                async fn echoTextAsync(value: String) -> String
                async fn echoBytesAsync(value: Bytes) -> Bytes
                async fn flushAsync()
                fn echoText(value: String) -> String
                fn echoBytes(value: Bytes) -> Bytes
                fn echoUByte(value: UInt8) -> UInt8
                fn echoUShort(value: UInt16) -> UInt16
                fn echoUInt(value: UInt32) -> UInt32
                fn echoULong(value: UInt64) -> UInt64
                fn maybeInt(value: Int32?) -> Int32?
                fn maybeText(value: String?) -> String?
                fn maybeBytes(value: Bytes?) -> Bytes?
            }
            native class Meter {
                init(initial: Float64, label: String)
                property value: Float64
                property label: String
                property payload: Bytes
                property unsignedValue: UInt32
                fn add(amount: Float64) -> Float64
                fn echo(value: String) -> String
                fn echoBytes(value: Bytes) -> Bytes
                fn echoUnsigned(value: UInt64) -> UInt64
                fn maybeNumber(value: Int64?) -> Int64?
                async fn currentValue() -> Int32
                fn dispose()
            }
            "#,
    )
    .expect("native IDL should parse");

    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&idl).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Counter",
        "dev.example.app",
        2,
    )
    .expect("synchronous Android C++ contracts should generate JNI adapters");

    assert!(kotlin.contains("internal object NexaPlugin2_CppBindings"));
    assert!(kotlin.contains("external fun service_Counter_echo(value: Int): Int"));
    assert!(kotlin.contains("public object CounterPlugin : Counter"));
    assert!(kotlin.contains("NexaPlugin2_CppBindings.service_Counter_echo(value)"));
    assert!(kotlin.contains(
            "override suspend fun echoAsync(value: Int): Int = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) { NexaPlugin2_CppBindings.service_Counter_echoAsync(value) }"
        ));
    assert!(kotlin.contains("override suspend fun echoTextAsync(value: String): String"));
    assert!(kotlin.contains("override suspend fun echoBytesAsync(value: ByteArray): ByteArray"));
    assert!(kotlin.contains("override suspend fun flushAsync(): Unit"));
    assert!(jni.contains("auto nexaCppFuture = Counter::echoAsync"));
    assert!(jni.contains("nexaCppFuture.get()"));
    assert!(kotlin.contains("public class MeterImpl"));
    assert!(kotlin.contains("java.lang.AutoCloseable"));
    assert!(kotlin.contains("private var nativeHandle: Long"));
    assert!(kotlin.contains("value: Double"));
    assert!(kotlin.contains("override fun dispose()"));
    assert!(kotlin.contains("override fun close() = dispose()"));
    assert!(jni.contains("Java_dev_example_app_NexaPlugin2_1CppBindings_service_1Counter_1echo"));
    assert!(jni.contains(
        "std::unique_ptr<plugin_dev::plugin_example::plugin_cpp_dash_plugin::MeterSpec>"
    ));
    assert!(jni.contains("instance->dispose();"));
    assert!(jni.contains("static_cast<jdouble>((*handle)->add"));
    assert!(kotlin.contains("external fun service_Counter_echoText(value: String): String"));
    assert!(kotlin.contains("external fun service_Counter_echoBytes(value: ByteArray): ByteArray"));
    assert!(
        kotlin.contains("external fun service_Counter_echoUByte(value: Byte): Byte"),
        "unexpected Kotlin JNI declaration:\n{kotlin}"
    );
    assert!(kotlin.contains("external fun service_Counter_echoUShort(value: Short): Short"));
    assert!(kotlin.contains("external fun service_Counter_echoUInt(value: Int): Int"));
    assert!(kotlin.contains("external fun service_Counter_echoULong(value: Long): Long"));
    assert!(kotlin.contains("label: String"));
    assert!(kotlin.contains("override var label: String"));
    assert!(kotlin.contains("override fun echo(value: String): String"));
    assert!(kotlin.contains("payload: ByteArray"));
    assert!(kotlin.contains("override var payload: ByteArray"));
    assert!(kotlin.contains("override var unsignedValue: UInt"));
    assert!(kotlin.contains(
        "get() = NexaPlugin2_CppBindings.get_Meter_unsignedValue(requireNativeHandle()).toUInt()"
    ));
    assert!(kotlin.contains(
            "set(value) = NexaPlugin2_CppBindings.set_Meter_unsignedValue(requireNativeHandle(), value.toInt())"
        ));
    assert!(kotlin.contains("override fun echoBytes(value: ByteArray): ByteArray"));
    assert!(kotlin.contains(
            "override suspend fun currentValue(): Int = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) { synchronized(this) { NexaPlugin2_CppBindings.call_Meter_currentValue(requireNativeHandle()) } }"
        ));
    assert!(kotlin.contains("external fun service_Counter_maybeInt(value: Int?): Int?"));
    assert!(kotlin.contains("external fun service_Counter_maybeText(value: String?): String?"));
    assert!(kotlin.contains("override fun maybeInt(value: Int?): Int?"));
    assert!(kotlin.contains("override fun maybeText(value: String?): String?"));
    assert!(jni.contains("jobject value)"));
    assert!(jni.contains("std::optional<std::int32_t> nexaJniOptionalArgument0"));
    assert!(jni.contains("java/lang/Integer"));
    assert!(jni.contains("fromJniString(env, value)"));
    assert!(jni.contains("return nullptr;"));
    assert!(kotlin.contains(
            "override fun echoUnsigned(value: ULong): ULong = NexaPlugin2_CppBindings.call_Meter_echoUnsigned(requireNativeHandle(), value.toLong()).toULong()"
        ));
    assert!(jni.contains("fromJniString(env, value)"));
    assert!(jni.contains("return toJniString(env, Counter::echoText("));
    assert!(jni.contains("return toJniString(env, (*handle)->echo("));
    assert!(jni.contains("ReleaseStringChars(value, characters)"));
    assert!(jni.contains("fromJniBytes(env, value)"));
    assert!(jni.contains("return toJniBytes(env, Counter::echoBytes("));
    assert!(jni.contains("return toJniBytes(env, (*handle)->echoBytes("));
    assert!(jni.contains("GetByteArrayRegion"));
    assert!(jni.contains("SetByteArrayRegion"));
    assert!(jni.contains(
        "return std::bit_cast<jlong>(static_cast<std::uint64_t>((*handle)->echoUnsigned"
    ));
}

#[test]
fn android_cpp_adapters_bridge_named_values_and_reject_unimplemented_shapes() {
    let idl = nexa_plugin_idl::parse(
        r#"
            enum MediaMode { idle playing }
            struct MediaStats {
                frameCount: Int32
                active: Bool
                title: String
                payload: Bytes
                mode: MediaMode
            }
            service Media {
                fn echoMode(value: MediaMode) -> MediaMode
                fn echoStats(value: MediaStats) -> MediaStats
            }
            "#,
    )
    .expect("named-value Android contract should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&idl).expect("android contract should validate"),
        "dev.example.media",
        "Media",
        "dev.example.app",
        0,
    )
    .expect("Android should bridge supported enum and struct values");
    assert!(kotlin.contains("external fun service_Media_echoMode(value: MediaMode): MediaMode"));
    assert!(kotlin.contains("external fun service_Media_echoStats(value: MediaStats): MediaStats"));
    assert!(jni.contains("nexaFromJniMediaMode(env, value)"));
    assert!(jni.contains("nexaToJniMediaStats(env, Media::echoStats"));
    assert!(jni.contains("getFrameCount"));
    assert!(jni.contains("getPayload"));
    assert!(jni.contains("ordinal"));

    for unsupported in ["MediaMode?", "Array<MediaMode>"] {
        let unsupported_idl = nexa_plugin_idl::parse(&format!(
                "enum MediaMode {{ idle playing }} service Media {{ fn echo(value: {unsupported}) -> {unsupported} }}"
            ))
            .expect("unsupported named-value shape should parse as IDL");
        let error = BridgePlan::validate_android(&unsupported_idl)
            .expect_err("unimplemented named-value shapes must be rejected");
        assert!(error.contains("unsupported type"));
    }

    let optional_field = nexa_plugin_idl::parse(
            "enum MediaMode { idle playing } struct MediaStats { mode: MediaMode? } service Media { fn echo(value: MediaStats) -> MediaStats }",
        )
        .expect("optional named struct field should parse as IDL");
    let error = BridgePlan::validate_android(&optional_field)
        .expect_err("unsupported optional named fields must be rejected");
    assert!(error.contains("unsupported type"));
}

#[test]
fn android_cpp_adapters_reject_unsafe_shapes_and_bridge_flat_primitive_arrays() {
    let no_dispose =
        nexa_plugin_idl::parse("native class Meter { init() }").expect("native IDL should parse");
    let error = BridgePlan::validate_android(&no_dispose)
        .expect_err("Android C++ native classes must have explicit deterministic disposal");
    assert!(error.contains("must declare `fn dispose()`"));

    let asynchronous = nexa_plugin_idl::parse("service Clock { async fn now() -> Int64 }")
        .expect("async native IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&asynchronous).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Clock",
        "dev.example.app",
        0,
    )
    .expect("async scalar C++ signatures should generate Kotlin and JNI adapters");
    assert!(kotlin.contains("override suspend fun now(): Long"));
    assert!(kotlin.contains("kotlinx.coroutines.Dispatchers.IO"));
    assert!(jni.contains("nexaCppFuture.get()"));

    let async_collection =
        nexa_plugin_idl::parse("service Clock { async fn values() -> Array<Int32> }")
            .expect("async collection native IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&async_collection).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Clock",
        "dev.example.app",
        0,
    )
    .expect("async collection return should use the existing JNI converters");
    assert!(kotlin.contains("external fun service_Clock_values(): IntArray"));
    assert!(kotlin.contains("override suspend fun values(): List<Int>"));
    assert!(jni.contains("Clock::values()"));
    assert!(jni.contains("nexaCppFuture.get()"));

    let async_collection_parameter =
        nexa_plugin_idl::parse("service Clock { async fn count(values: Array<Int32>) -> Int32 }")
            .expect("async collection parameter IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&async_collection_parameter)
            .expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Clock",
        "dev.example.app",
        0,
    )
    .expect("async collection parameter should use the existing JNI converters");
    assert!(kotlin.contains("override suspend fun count(values: List<Int>): Int"));
    assert!(kotlin.contains("values.toIntArray()"));
    assert!(jni.contains("nexaJniArrayArgument0"));
    assert!(jni.contains("GetIntArrayRegion"));

    let throwing = nexa_plugin_idl::parse(
        r#"error Failure { rejected, malformed(code: UInt32, message: String, payload: Bytes) }
            service Clock {
                async fn read() throws Failure
                async fn readValue() -> Result<Int32, Failure>
            }"#,
    )
    .expect("throwing native IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&throwing).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Clock",
        "dev.example.app",
        0,
    )
    .expect("Android should bridge typed C++ errors into Kotlin exceptions");
    assert!(
        kotlin.contains("@JvmStatic public fun createFailureCase0(): Failure = Failure.rejected")
    );
    assert!(kotlin.contains("createFailureCase1(code: Int, message: String, payload: ByteArray): Failure = Failure.malformed(code.toUInt(), message, payload)"));
    assert!(kotlin.contains("override suspend fun read(): Unit"));
    assert!(kotlin.contains("override suspend fun readValue(): Int"));
    assert!(jni.contains("NexaPlugin0_CppErrorFactory"));
    assert!(
        jni.contains("createFailureCase1\", \"(ILjava/lang/String;[B)Ldev/example/app/Failure;\"")
    );
    assert!(jni.contains("nexaCppThrowErrorFailure(env, nexaCppTypedResult.error())"));

    let bad_error_payload = nexa_plugin_idl::parse(
            "error Failure { rejected(payload: Array<Int32>) } service Clock { async fn read() throws Failure }",
        )
        .expect("typed collection error payload should parse");
    let error = BridgePlan::validate_android(&bad_error_payload)
        .expect_err("unsupported typed-error payloads must fail generation");
    assert!(error.contains("typed errors support"));

    let events = nexa_plugin_idl::parse(
        "native class Watcher { init() event changed(value: Int32) fn dispose() }",
    )
    .expect("native class event IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&events).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Watcher",
        "dev.example.app",
        0,
    )
    .expect("Android event adapters should preserve instance-scoped callbacks");
    assert!(kotlin.contains("override var onChanged: ((Int) -> Unit)?"));
    assert!(jni.contains("GetMethodID(nexaCallbackClass.get(), \"nexaDispatch\", \"(I)V\")"));
    assert!(jni.contains("instance->setOnChanged({})"));

    let optional = nexa_plugin_idl::parse("service Lookup { fn maybe(value: UInt64?) -> UInt64? }")
        .expect("optional native IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&optional).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("nullable unsigned adapters should use the Long JNI carrier");
    assert!(kotlin.contains("maybe(value: ULong?): ULong?"));
    assert!(kotlin.contains("value?.toLong()"));
    assert!(kotlin.contains(".toULong()"));
    assert!(jni.contains("std::optional<std::uint64_t>"));
    assert!(jni.contains("std::bit_cast<jlong>(static_cast<std::uint64_t>(*nexaOptionalValue))"));

    let collection =
        nexa_plugin_idl::parse("service Lookup { fn find(ids: Array<UInt32>) -> Array<UInt32> }")
            .expect("collection native IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&collection).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("flat primitive arrays should generate typed JNI adapters");
    assert!(kotlin.contains("external fun service_Lookup_find(ids: IntArray): IntArray"));
    assert!(kotlin.contains("override fun find(ids: List<UInt>): List<UInt>"));
    assert!(
        kotlin.contains("IntArray(ids.size) { ids[it].toInt() }"),
        "expected unsigned list conversion in generated Kotlin:\n{kotlin}"
    );
    assert!(kotlin.contains(".map { it.toUInt() }"));
    assert!(jni.contains("jintArray ids"));
    assert!(jni.contains("GetIntArrayRegion"));
    assert!(jni.contains("SetIntArrayRegion"));

    let nested = nexa_plugin_idl::parse(
        "service Lookup { fn find(ids: Array<Array<Int32>>) -> Array<Array<Int32>> }",
    )
    .expect("nested collection native IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&nested).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("nested arrays should generate recursive JNI adapters");
    assert!(
        kotlin.contains("external fun service_Lookup_find(ids: Array<IntArray>): Array<IntArray>")
    );
    assert!(kotlin.contains("override fun find(ids: List<List<Int>>): List<List<Int>>"));
    assert!(kotlin.contains(
        "ids.map { nexaNestedCollection -> nexaNestedCollection.toIntArray() }.toTypedArray()"
    ));
    assert!(
        kotlin.contains(").map { nexaNestedCollection -> nexaNestedCollection.asList() }"),
        "nested Kotlin return conversion is missing:\n{kotlin}"
    );
    assert!(jni.contains("std::vector<std::vector<std::int32_t>> nexaJniArrayArgument0"));
    assert!(jni.contains("GetObjectArrayElement(static_cast<jobjectArray>(ids)"));
    assert!(jni.contains("FindClass(\"[I\")"));
    assert!(jni.contains("NewObjectArray(nexaNestedArrayLength"));
    assert!(jni.contains("GetIntArrayRegion"));
    assert!(jni.contains("SetIntArrayRegion"));

    let primitive_arrays = nexa_plugin_idl::parse(
        r#"service Arrays {
                fn booleans(values: Array<Bool>) -> Array<Bool>
                fn int8s(values: Array<Int8>) -> Array<Int8>
                fn uint8s(values: Array<UInt8>) -> Array<UInt8>
                fn int16s(values: Array<Int16>) -> Array<Int16>
                fn uint16s(values: Array<UInt16>) -> Array<UInt16>
                fn int32s(values: Array<Int32>) -> Array<Int32>
                fn uint32s(values: Array<UInt32>) -> Array<UInt32>
                fn int64s(values: Array<Int64>) -> Array<Int64>
                fn uint64s(values: Array<UInt64>) -> Array<UInt64>
                fn float32s(values: Array<Float32>) -> Array<Float32>
                fn float64s(values: Array<Float64>) -> Array<Float64>
            }"#,
    )
    .expect("primitive array IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&primitive_arrays).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Arrays",
        "dev.example.app",
        0,
    )
    .expect("all primitive array carriers should be supported");
    for declaration in [
        "booleans(values: BooleanArray): BooleanArray",
        "int8s(values: ByteArray): ByteArray",
        "uint8s(values: ByteArray): ByteArray",
        "int16s(values: ShortArray): ShortArray",
        "uint16s(values: ShortArray): ShortArray",
        "int32s(values: IntArray): IntArray",
        "uint32s(values: IntArray): IntArray",
        "int64s(values: LongArray): LongArray",
        "uint64s(values: LongArray): LongArray",
        "float32s(values: FloatArray): FloatArray",
        "float64s(values: DoubleArray): DoubleArray",
    ] {
        assert!(
            kotlin.contains(declaration),
            "missing generated primitive array declaration `{declaration}`"
        );
    }
    for conversion in [
        "GetBooleanArrayRegion",
        "SetBooleanArrayRegion",
        "GetByteArrayRegion",
        "SetByteArrayRegion",
        "GetShortArrayRegion",
        "SetShortArrayRegion",
        "GetIntArrayRegion",
        "SetIntArrayRegion",
        "GetLongArrayRegion",
        "SetLongArrayRegion",
        "GetFloatArrayRegion",
        "SetFloatArrayRegion",
        "GetDoubleArrayRegion",
        "SetDoubleArrayRegion",
    ] {
        assert!(
            jni.contains(conversion),
            "missing JNI conversion `{conversion}`"
        );
    }

    let reference_arrays = nexa_plugin_idl::parse(
            "service Lookup { fn strings(values: Array<String>) -> Array<String> fn payloads(values: Array<Bytes>) -> Array<Bytes> }",
        )
        .expect("string and byte-array IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&reference_arrays).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("string and byte-array arrays should generate safe JNI adapters");
    assert!(
        kotlin
            .contains("external fun service_Lookup_strings(values: Array<String>): Array<String>")
    );
    assert!(kotlin.contains(
        "external fun service_Lookup_payloads(values: Array<ByteArray>): Array<ByteArray>"
    ));
    assert!(kotlin.contains("override fun strings(values: List<String>): List<String>"));
    assert!(kotlin.contains("values.toTypedArray()"));
    assert!(kotlin.contains("service_Lookup_strings(values.toTypedArray()).asList()"));
    assert!(kotlin.contains("service_Lookup_payloads(values.toTypedArray()).asList()"));
    assert!(jni.contains("jobjectArray values"));
    assert!(jni.contains("FindClass(\"java/lang/String\")"));
    assert!(jni.contains("FindClass(\"[B\")"));
    assert!(jni.contains("fromJniString(env, nexaJniArrayArgument0Element)"));
    assert!(jni.contains("fromJniBytes(env, nexaJniArrayArgument0Element)"));
    assert!(jni.contains("SetObjectArrayElement"));

    let sets = nexa_plugin_idl::parse(
            "service Lookup { fn integers(values: Set<UInt32>) -> Set<UInt32> fn labels(values: Set<String>) -> Set<String> }",
        )
        .expect("set IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&sets).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("primitive and string sets should adapt through JNI arrays");
    assert!(kotlin.contains("external fun service_Lookup_integers(values: IntArray): IntArray"));
    assert!(kotlin.contains("override fun integers(values: Set<UInt>): Set<UInt>"));
    assert!(kotlin.contains("override fun labels(values: Set<String>): Set<String>"));
    assert!(kotlin.contains("values.map { it.toInt() }.toIntArray()"));
    assert!(kotlin.contains(".map { it.toUInt() }.toSet()"));
    let signed_set =
        nexa_plugin_idl::parse("service Lookup { fn signed(values: Set<Int32>) -> Set<Int32> }")
            .expect("signed set IDL should parse");
    let (kotlin, _) = render_android_adapters(
        &BridgePlan::validate_android(&signed_set).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("signed integer sets should use direct collection conversion");
    assert!(kotlin.contains("values.toIntArray()"));
    assert!(jni.contains("std::set<std::uint32_t> nexaJniArrayArgument0"));
    assert!(jni.contains("nexaJniArrayArgument0.insert("));
    assert!(jni.contains("std::set<std::string> nexaJniArrayArgument0"));

    for unsupported in ["Set<Bytes>", "Set<Float64>"] {
        let idl = nexa_plugin_idl::parse(&format!(
            "service Lookup {{ fn values(items: {unsupported}) -> {unsupported} }}"
        ))
        .expect("set IDL should parse");
        let error = BridgePlan::validate_android(&idl)
            .expect_err("Android set adapters must preserve target-native Set equality");
        assert!(error.contains("unsupported type `Set`"));
    }

    let maps = nexa_plugin_idl::parse(
            "service Lookup { fn values(items: Map<String, Int32>) -> Map<String, Int32> fn unsigned(items: Map<UInt32, UInt64>) -> Map<UInt32, UInt64> }",
        )
        .expect("map IDL should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&maps).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("flat scalar and string maps should adapt through JNI");
    assert!(
        kotlin.contains(
            "external fun service_Lookup_values(items: Map<String, Int>): Map<String, Int>"
        )
    );
    assert!(kotlin.contains("override fun values(items: Map<String, Int>): Map<String, Int>"));
    assert!(jni.contains("fromJniMap<std::map<std::string, std::int32_t>>"));
    assert!(jni.contains("toJniMap(env, Lookup::values"));
    assert!(
        kotlin.contains(
            "external fun service_Lookup_unsigned(items: Map<Int, Long>): Map<Int, Long>"
        )
    );
    assert!(kotlin.contains(
        "items.mapKeys { it.key.toInt() }.mapValues { (_, nexaMapValue) -> nexaMapValue.toLong() }"
    ));
    assert!(kotlin.contains(
        ".mapKeys { it.key.toUInt() }.mapValues { (_, nexaMapValue) -> nexaMapValue.toULong() }"
    ));
    assert!(jni.contains("std::map<std::uint32_t, std::uint64_t>"));
    assert!(jni.contains("java/lang/Integer"));
    assert!(jni.contains("java/lang/Long"));

    let nested_maps = nexa_plugin_idl::parse(
            "service Lookup { fn arrays(items: Map<String, Array<Int32>>) -> Map<String, Array<Int32>> fn sets(items: Map<String, Set<UInt32>>) -> Map<String, Set<UInt32>> }",
        )
        .expect("maps with collection values should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&nested_maps).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("maps with primitive array and set values should adapt recursively");
    assert!(kotlin.contains(
        "external fun service_Lookup_arrays(items: Map<String, IntArray>): Map<String, IntArray>"
    ));
    assert!(
        kotlin
            .contains("override fun arrays(items: Map<String, List<Int>>): Map<String, List<Int>>")
    );
    assert!(kotlin.contains("items.mapValues { (_, nexaMapValue) -> nexaMapValue.toIntArray() }"));
    assert!(kotlin.contains("nexaMapValue.asList()"));
    assert!(
        kotlin.contains("override fun sets(items: Map<String, Set<UInt>>): Map<String, Set<UInt>>")
    );
    assert!(kotlin.contains("nexaMapValue.map { it.toInt() }.toIntArray()"));
    assert!(kotlin.contains("nexaMapValue.map { it.toUInt() }.toSet()"));
    assert!(jni.contains("std::map<std::string, std::vector<std::int32_t>>"));
    assert!(jni.contains("std::map<std::string, std::set<std::uint32_t>>"));
    assert!(jni.contains("GetIntArrayRegion"));
    assert!(jni.contains("SetIntArrayRegion"));

    let reference_maps = nexa_plugin_idl::parse(
            "service Lookup { fn strings(items: Map<Int32, Array<String>>) -> Map<Int32, Array<String>> fn bytes(items: Map<Int32, Array<Bytes>>) -> Map<Int32, Array<Bytes>> fn byteValues(items: Map<Int32, Bytes>) -> Map<Int32, Bytes> fn optional(items: Map<UInt32, Bytes>?) -> Map<UInt32, Bytes>? fn nested(items: Map<Int32, Map<String, Bytes>>) -> Map<Int32, Map<String, Bytes>> fn labels(items: Map<Int32, Set<String>>) -> Map<Int32, Set<String>> }",
        )
        .expect("maps with reference collection values should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&reference_maps).expect("android contract should validate"),
        "dev.example.cpp-plugin",
        "Lookup",
        "dev.example.app",
        0,
    )
    .expect("string and byte arrays and string sets should adapt as map values");
    assert!(kotlin.contains("Map<Int, Array<String>>"));
    assert!(kotlin.contains("Map<Int, Array<ByteArray>>"));
    assert!(kotlin.contains("Map<Int, ByteArray>"));
    assert!(kotlin.contains("Map<Int, ByteArray>?"));
    assert!(kotlin.contains("Map<UInt, ByteArray>?"));
    assert!(
        kotlin.contains(
            "items?.let { nexaOptionalMap -> nexaOptionalMap.mapKeys { it.key.toInt() } }"
        )
    );
    assert!(kotlin.contains("nexaOptionalMap.mapKeys { it.key.toUInt() }"));
    assert!(kotlin.contains("Map<Int, Map<String, ByteArray>>"));
    assert!(kotlin.contains("Map<Int, Set<String>>"));
    assert!(jni.contains("std::map<std::int32_t, std::vector<std::string>>"));
    assert!(jni.contains("std::map<std::int32_t, std::vector<std::vector<std::uint8_t>>>"));
    assert!(jni.contains("std::map<std::int32_t, std::vector<std::uint8_t>>"));
    assert!(jni.contains(
        "std::optional<std::map<std::uint32_t, std::vector<std::uint8_t>>> nexaJniMapArgument0"
    ));
    assert!(jni.contains("nexaJniOptionalMapReturn.has_value()"));
    assert!(
        jni.contains("std::map<std::int32_t, std::map<std::string, std::vector<std::uint8_t>>>")
    );
    assert!(jni.contains("toJniBytes(env, value)"));
    assert!(jni.contains("std::map<std::int32_t, std::set<std::string>>"));
    assert!(jni.contains("fromJniString(env, static_cast<jstring>(element.get()))"));
    assert!(jni.contains("fromJniBytes(env, static_cast<jbyteArray>(element.get()))"));
    assert!(jni.contains("NewObjectArray(length, elementClass.get(), nullptr)"));
    assert!(jni.contains("fromJniMap<std::map<std::string, std::vector<std::uint8_t>>>"));

    for unsupported in [
        "Map<Float64, Int32>",
        "Map<Float32, Int32>",
        "Map<Bytes, Int32>",
        "Map<String?, Int32>",
    ] {
        let idl = nexa_plugin_idl::parse(&format!(
            "service Lookup {{ fn values(items: {unsupported}) -> Int32 }}"
        ))
        .expect("unsupported map shape should remain valid IDL");
        let error = BridgePlan::validate_android(&idl)
            .expect_err("unsupported C++ map shapes must fail generation");
        assert!(error.contains("unsupported type `Map`"), "{error}");
    }
}

#[test]
fn android_cpp_adapters_bridge_mutable_named_properties() {
    // A mutable native-class property of a declared struct or enum passes
    // Android validation, so the JNI setter must render the same named-value
    // reader the method-argument path uses instead of panicking on a missing
    // scalar spelling.
    let idl = nexa_plugin_idl::parse(
        r#"
            enum MediaMode { idle playing }
            struct MediaStats {
                frameCount: Int32
                mode: MediaMode
            }
            native class Recorder {
                init()
                property stats: MediaStats
                property mode: MediaMode
                property gain: Float64
                fn dispose()
            }
            "#,
    )
    .expect("mutable named-property Android contract should parse");
    let (kotlin, jni) = render_android_adapters(
        &BridgePlan::validate_android(&idl).expect("android contract should validate"),
        "dev.example.media",
        "Recorder",
        "dev.example.app",
        0,
    )
    .expect("Android should bridge mutable named properties");

    assert!(kotlin.contains("external fun set_Recorder_stats(handle: Long, value: MediaStats)"));
    assert!(kotlin.contains("external fun set_Recorder_mode(handle: Long, value: MediaMode)"));
    // Named setters read the incoming `jobject` through the shared helper.
    assert!(jni.contains("nexaFromJniMediaStats(env, static_cast<jobject>(value))"));
    assert!(jni.contains("nexaFromJniMediaMode(env, static_cast<jobject>(value))"));
    // Scalar properties keep their existing spelling.
    assert!(jni.contains("static_cast<double>(value)"));
}
