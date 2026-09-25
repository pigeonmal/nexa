//! Android JNI C++ adapters and the Kotlin JNI binding declarations.
//!
//! Renders the C++ half of the Android bridge (`extern "C"` entry points,
//! array/map conversion helpers, named-value readers and writers) and the Kotlin
//! half that calls them, including native-class ownership and event callback
//! plumbing.
//!
//! Type spelling is separated out into [`values`]: this module decides *where*
//! to emit a boundary function, and `values` decides *how* a value is spelled
//! once it is there.

pub(crate) mod values;

use self::values::{
    android_boxed_primitive, android_cpp_type, android_cpp_value, android_jni_class_descriptor,
    android_jni_map_boxed_primitive, android_jni_type, android_kotlin_type,
    android_primitive_array, android_primitive_descriptor, android_reference_array_element,
};
use super::abi::{
    bridge_strip_optional, bridge_type_name, cpp_factory_name, cpp_getter_name, cpp_identifier,
    cpp_namespace, cpp_setter_name, cpp_type,
};
use super::errors::{
    render_android_error_factory, render_android_jni_error_converters,
    render_jni_typed_error_result,
};
use crate::SourceWriter;
use crate::plugin::bridge_plan::{
    BridgeConstructor, BridgeEvent, BridgeInterface, BridgeMethod, BridgeParameter, BridgePlan,
    BridgeProperty, BridgeScalar, BridgeType, BridgeTypeKind,
};
use crate::plugin::type_visit::collect_value_types;
use nexa_plugin_idl::{self, InterfaceKind};

/// Emits Kotlin wrappers and JNI declarations for the synchronous Android value
/// subset of a C++ plugin contract, including flat primitive arrays. Unsupported
/// IDL shapes fail generation instead of leaving declarations without a safe
/// native implementation.
pub fn render_android_adapters(
    plan: &BridgePlan,
    plugin_id: &str,
    plugin_namespace: &str,
    package: &str,
    plugin_index: usize,
) -> Result<(String, String), String> {
    let mut native_declarations = SourceWriter::new();
    let mut kotlin_implementations = SourceWriter::new();
    let mut kotlin_event_bridges = SourceWriter::new();
    let mut jni = SourceWriter::new();
    let class_name = format!("NexaPlugin{plugin_index}_CppBindings");
    let mut service_interfaces = Vec::new();
    let mut service_methods = std::collections::BTreeSet::new();
    let mut has_adapters = false;

    for interface in &plan.interfaces {
        match interface.kind {
            InterfaceKind::Service => {
                for method in &interface.methods {
                    if !service_methods.insert(method.name.clone()) {
                        return Err(format!(
                            "Android C++ adapters do not support duplicate service method name `{}`",
                            method.name
                        ));
                    }
                    let native_name = format!("service_{}_{}", interface.name, method.name);
                    render_kotlin_native_declaration(
                        &mut native_declarations,
                        method,
                        &native_name,
                    );
                    render_jni_service_method(
                        &mut jni,
                        package,
                        &class_name,
                        &interface.name,
                        method,
                        &native_name,
                    );
                }
                service_interfaces.push(interface.name.clone());
                has_adapters = true;
            }
            InterfaceKind::NativeClass => {
                let constructor = interface.constructors.first();
                for event in &interface.events {
                    let setter = format!(
                        "set_{}_{}",
                        interface.name,
                        nexa_plugin_idl::event_callback_property(&event.name)
                    );
                    render_kotlin_native_event_declaration(&mut native_declarations, &setter);
                    render_jni_event_setter(
                        &mut jni,
                        plugin_id,
                        package,
                        &class_name,
                        interface,
                        event,
                        &setter,
                    );
                    render_kotlin_event_bridge(
                        &mut kotlin_event_bridges,
                        interface,
                        event,
                        plugin_index,
                    );
                }
                // Plan validation proves every native class declares a
                // well-formed `dispose` method; the method loop below
                // renders it by name.
                let create_name = format!("create_{}", interface.name);
                render_kotlin_native_declaration(
                    &mut native_declarations,
                    &BridgeMethod {
                        name: create_name.clone(),
                        parameters: constructor
                            .map(|constructor| constructor.parameters.clone())
                            .unwrap_or_default(),
                        return_type: BridgeType::Scalar(BridgeScalar::Int64),
                        is_async: false,
                        throws: None,
                    },
                    &create_name,
                );
                render_jni_constructor(
                    &mut jni,
                    plugin_id,
                    package,
                    &class_name,
                    interface,
                    &create_name,
                );

                for property in &interface.properties {
                    let getter_name = format!("get_{}_{}", interface.name, property.name);
                    render_kotlin_property_native_declarations(
                        &mut native_declarations,
                        property,
                        &interface.name,
                        &getter_name,
                    );
                    render_jni_property(
                        &mut jni,
                        plugin_id,
                        package,
                        &class_name,
                        interface,
                        property,
                        &getter_name,
                    );
                }
                for method in &interface.methods {
                    if method.name == "dispose" {
                        let native_name = format!("dispose_{}", interface.name);
                        render_kotlin_native_dispose_declaration(
                            &mut native_declarations,
                            &native_name,
                        );
                        render_jni_dispose(
                            &mut jni,
                            plugin_id,
                            package,
                            &class_name,
                            interface,
                            &native_name,
                        );
                    } else {
                        let native_name = format!("call_{}_{}", interface.name, method.name);
                        render_kotlin_class_native_declaration(
                            &mut native_declarations,
                            method,
                            &native_name,
                        );
                        render_jni_class_method(
                            &mut jni,
                            plugin_id,
                            package,
                            &class_name,
                            interface,
                            method,
                            &native_name,
                        );
                    }
                }
                render_kotlin_cpp_class(
                    &mut kotlin_implementations,
                    interface,
                    plugin_index,
                    constructor,
                );
                has_adapters = true;
            }
            InterfaceKind::Interface | InterfaceKind::NativeComponent => {}
        }
    }

    if !has_adapters {
        return Ok((String::new(), String::new()));
    }

    let mut kotlin_output = SourceWriter::new();
    kotlin_output.text(format_args!(
        "\ninternal object {class_name} {{\n    init {{ System.loadLibrary(\"nexa_plugins\") }}\n"
    ));
    kotlin_output.push_str(&native_declarations);
    kotlin_output.push_str("}\n\n");

    render_android_error_factory(&mut kotlin_output, plan, plugin_index);

    if !service_interfaces.is_empty() {
        let contracts = service_interfaces.join(", ");
        kotlin_output.push_str(&format!(
            "public object {plugin_namespace}Plugin : {contracts} {{\n    public val instance: {plugin_namespace}Plugin get() = this\n"
        ));
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                render_kotlin_service_adapter(
                    &mut kotlin_output,
                    method,
                    &format!("service_{}_{}", interface.name, method.name),
                    &class_name,
                );
            }
        }
        kotlin_output.push_str("}\n\n");
    }

    if !kotlin_event_bridges.is_empty() {
        kotlin_output.push_str(&format!(
            "private object NexaPlugin{plugin_index}_CppEventDispatcher {{\n    private val mainHandler = android.os.Handler(android.os.Looper.getMainLooper())\n    fun post(operation: () -> Unit) {{ mainHandler.post(operation) }}\n}}\n\n"
        ));
        kotlin_output.push_str(&kotlin_event_bridges);
    }
    kotlin_output.push_str(&kotlin_implementations);
    let mut jni_output = SourceWriter::new();
    jni_output.push_str(&render_android_jni_prelude(plugin_id));
    render_android_jni_error_converters(&mut jni_output, plan, package, plugin_index);
    render_android_jni_named_value_helpers(&mut jni_output, plan, package);
    jni_output.push_str(&jni);
    Ok((kotlin_output.finish(), jni_output.finish()))
}

fn render_android_jni_prelude(plugin_id: &str) -> String {
    const PRELUDE: &str = r#"#include <jni.h>
#include <algorithm>
#include <array>
#include <bit>
#include <cstdint>
#include <exception>
#include <limits>
#include <memory>
#include <new>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#include "NexaPluginBindings.hpp"

namespace {
using namespace __NEXA_CPP_NAMESPACE__;

void throwIllegalState(JNIEnv* env, const char* message) {
    if (env->ExceptionCheck()) return;
    jclass exception = env->FindClass("java/lang/IllegalStateException");
    if (exception != nullptr) env->ThrowNew(exception, message);
}

void throwRuntime(JNIEnv* env, const char* message) {
    if (env->ExceptionCheck()) return;
    jclass exception = env->FindClass("java/lang/RuntimeException");
    if (exception != nullptr) env->ThrowNew(exception, message);
}

template <class Function>
auto withJniExceptions(JNIEnv* env, Function&& function) noexcept -> decltype(function()) {
    using Return = decltype(function());
    try {
        if constexpr (std::is_void_v<Return>) {
            function();
            return;
        } else {
            return function();
        }
    } catch (const std::exception& error) {
        throwRuntime(env, error.what());
    } catch (...) {
        throwRuntime(env, "native plugin operation failed");
    }
    if constexpr (std::is_void_v<Return>) return;
    else return Return{};
}

void appendUtf8(std::string& output, std::uint32_t codepoint) {
    if (codepoint <= 0x7f) output.push_back(static_cast<char>(codepoint));
    else if (codepoint <= 0x7ff) {
        output.push_back(static_cast<char>(0xc0 | (codepoint >> 6)));
        output.push_back(static_cast<char>(0x80 | (codepoint & 0x3f)));
    } else if (codepoint <= 0xffff) {
        output.push_back(static_cast<char>(0xe0 | (codepoint >> 12)));
        output.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3f)));
        output.push_back(static_cast<char>(0x80 | (codepoint & 0x3f)));
    } else {
        output.push_back(static_cast<char>(0xf0 | (codepoint >> 18)));
        output.push_back(static_cast<char>(0x80 | ((codepoint >> 12) & 0x3f)));
        output.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3f)));
        output.push_back(static_cast<char>(0x80 | (codepoint & 0x3f)));
    }
}

std::string fromJniString(JNIEnv* env, jstring value) {
    if (value == nullptr) {
        throwIllegalState(env, "non-null Nexa string was null at the JNI boundary");
        return {};
    }
    const jsize length = env->GetStringLength(value);
    const jchar* characters = env->GetStringChars(value, nullptr);
    if (characters == nullptr) return {};
    struct ReleaseStringChars {
        JNIEnv* env;
        jstring value;
        const jchar* characters;
        ~ReleaseStringChars() { env->ReleaseStringChars(value, characters); }
    } release{env, value, characters};

    std::string output;
    for (jsize index = 0; index < length; ++index) {
        std::uint32_t codepoint = characters[index];
        if (codepoint >= 0xd800 && codepoint <= 0xdbff) {
            if (index + 1 < length && characters[index + 1] >= 0xdc00 && characters[index + 1] <= 0xdfff) {
                codepoint = 0x10000 + ((codepoint - 0xd800) << 10) + (characters[++index] - 0xdc00);
            } else {
                codepoint = 0xfffd;
            }
        } else if (codepoint >= 0xdc00 && codepoint <= 0xdfff) {
            codepoint = 0xfffd;
        }
        appendUtf8(output, codepoint);
    }
    return output;
}

std::vector<std::uint8_t> fromJniBytes(JNIEnv* env, jbyteArray value) {
    if (value == nullptr) {
        throwIllegalState(env, "non-null Nexa byte array was null at the JNI boundary");
        return {};
    }
    const jsize length = env->GetArrayLength(value);
    std::vector<std::uint8_t> output(static_cast<std::size_t>(length));
    if (length > 0) {
        env->GetByteArrayRegion(value, 0, length, reinterpret_cast<jbyte*>(output.data()));
    }
    return output;
}

jbyteArray toJniBytes(JNIEnv* env, const std::vector<std::uint8_t>& value) {
    if (value.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) {
        throw std::length_error("native plugin byte array is too large for JNI");
    }
    const auto length = static_cast<jsize>(value.size());
    jbyteArray output = env->NewByteArray(length);
    if (output == nullptr || length == 0) return output;
    env->SetByteArrayRegion(output, 0, length, reinterpret_cast<const jbyte*>(value.data()));
    return output;
}

std::uint32_t decodeUtf8(const std::string& value, std::size_t& index) {
    const auto first = static_cast<std::uint8_t>(value[index]);
    if (first < 0x80) {
        ++index;
        return first;
    }
    const std::size_t count = first >= 0xf0 ? 4 : first >= 0xe0 ? 3 : first >= 0xc2 ? 2 : 0;
    if (count == 0 || index + count > value.size()) {
        ++index;
        return 0xfffd;
    }
    std::uint32_t codepoint = first & (count == 2 ? 0x1f : count == 3 ? 0x0f : 0x07);
    for (std::size_t offset = 1; offset < count; ++offset) {
        const auto byte = static_cast<std::uint8_t>(value[index + offset]);
        if ((byte & 0xc0) != 0x80) {
            ++index;
            return 0xfffd;
        }
        codepoint = (codepoint << 6) | (byte & 0x3f);
    }
    const bool overlong = (count == 2 && codepoint < 0x80) ||
        (count == 3 && codepoint < 0x800) ||
        (count == 4 && codepoint < 0x10000);
    if (overlong || codepoint > 0x10ffff || (codepoint >= 0xd800 && codepoint <= 0xdfff)) {
        ++index;
        return 0xfffd;
    }
    index += count;
    return codepoint;
}

jstring toJniString(JNIEnv* env, const std::string& value) {
    std::vector<jchar> output;
    output.reserve(value.size());
    for (std::size_t index = 0; index < value.size();) {
        const auto codepoint = decodeUtf8(value, index);
        if (codepoint <= 0xffff) {
            output.push_back(static_cast<jchar>(codepoint));
        } else {
            const auto adjusted = codepoint - 0x10000;
            output.push_back(static_cast<jchar>(0xd800 + (adjusted >> 10)));
            output.push_back(static_cast<jchar>(0xdc00 + (adjusted & 0x3ff)));
        }
    }
    if (output.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) {
        throw std::length_error("native plugin string is too large for JNI");
    }
    const jchar empty = 0;
    return env->NewString(output.empty() ? &empty : output.data(), static_cast<jsize>(output.size()));
}

template <class T>
std::unique_ptr<T>* requireHandle(JNIEnv* env, jlong rawHandle) {
    if (rawHandle == 0) {
        throwIllegalState(env, "native plugin object is disposed");
        return nullptr;
    }
    auto* handle = reinterpret_cast<std::unique_ptr<T>*>(static_cast<std::uintptr_t>(rawHandle));
    if (handle == nullptr || !*handle) {
        throwIllegalState(env, "native plugin object handle is invalid");
        return nullptr;
    }
    return handle;
}

template <class T>
class ScopedLocalRef {
public:
    ScopedLocalRef(JNIEnv* env, T value) : env_(env), value_(value) {}
    ScopedLocalRef(const ScopedLocalRef&) = delete;
    ScopedLocalRef& operator=(const ScopedLocalRef&) = delete;
    ~ScopedLocalRef() { if (value_ != nullptr) env_->DeleteLocalRef(value_); }
    T get() const noexcept { return value_; }
    T release() noexcept { T value = value_; value_ = nullptr; return value; }
private:
    JNIEnv* env_;
    T value_;
};

class ScopedJniEnvironment {
public:
    explicit ScopedJniEnvironment(JavaVM* vm) noexcept : vm_(vm) {
        if (vm_ == nullptr) return;
        void* rawEnvironment = nullptr;
        const jint status = vm_->GetEnv(&rawEnvironment, JNI_VERSION_1_6);
        if (status == JNI_OK) {
            environment_ = static_cast<JNIEnv*>(rawEnvironment);
        } else if (status == JNI_EDETACHED && attachCurrentThread() == JNI_OK) {
            attached_ = true;
        }
    }
    ScopedJniEnvironment(const ScopedJniEnvironment&) = delete;
    ScopedJniEnvironment& operator=(const ScopedJniEnvironment&) = delete;
    ~ScopedJniEnvironment() {
        if (attached_) vm_->DetachCurrentThread();
    }
    JNIEnv* get() const noexcept { return environment_; }
private:
    jint attachCurrentThread() noexcept {
#if defined(__ANDROID__)
        return vm_->AttachCurrentThread(&environment_, nullptr);
#else
        return vm_->AttachCurrentThread(reinterpret_cast<void**>(&environment_), nullptr);
#endif
    }
    JavaVM* vm_{};
    JNIEnv* environment_{};
    bool attached_{};
};

class JniGlobalReference {
public:
    JniGlobalReference(JNIEnv* env, jobject value) {
        if (env == nullptr || value == nullptr || env->GetJavaVM(&vm_) != JNI_OK) return;
        value_ = env->NewGlobalRef(value);
    }
    JniGlobalReference(const JniGlobalReference&) = delete;
    JniGlobalReference& operator=(const JniGlobalReference&) = delete;
    ~JniGlobalReference() {
        if (value_ == nullptr) return;
        ScopedJniEnvironment scope(vm_);
        if (auto* env = scope.get()) env->DeleteGlobalRef(value_);
    }
    JavaVM* vm() const noexcept { return vm_; }
    jobject get() const noexcept { return value_; }
private:
    JavaVM* vm_{};
    jobject value_{};
};

class ScopedJniLocalFrame {
public:
    ScopedJniLocalFrame(JNIEnv* env, jint capacity) noexcept
        : env_(env), active_(env_ != nullptr && env_->PushLocalFrame(capacity) == JNI_OK) {}
    ScopedJniLocalFrame(const ScopedJniLocalFrame&) = delete;
    ScopedJniLocalFrame& operator=(const ScopedJniLocalFrame&) = delete;
    ~ScopedJniLocalFrame() { if (active_) env_->PopLocalFrame(nullptr); }
    bool active() const noexcept { return active_; }
private:
    JNIEnv* env_{};
    bool active_{};
};

void requireJniMapOperation(JNIEnv* env, const char* message) {
    if (env->ExceptionCheck()) throw std::runtime_error(message);
}

template <class T>
inline constexpr bool isOptionalMapValue = false;

template <class T>
inline constexpr bool isOptionalMapValue<std::optional<T>> = true;

template <class Map, class KeyConverter, class ValueConverter>
Map fromJniMap(JNIEnv* env, jobject input, KeyConverter&& convertKey, ValueConverter&& convertValue) {
    if (input == nullptr) {
        throwIllegalState(env, "non-null Nexa map was null at the JNI boundary");
        throw std::runtime_error("null JNI map");
    }
    ScopedLocalRef<jclass> mapClass(env, env->FindClass("java/util/Map"));
    requireJniMapOperation(env, "unable to resolve java.util.Map");
    jmethodID entrySetMethod = env->GetMethodID(mapClass.get(), "entrySet", "()Ljava/util/Set;");
    requireJniMapOperation(env, "unable to resolve Map.entrySet");
    ScopedLocalRef<jobject> entries(env, env->CallObjectMethod(input, entrySetMethod));
    requireJniMapOperation(env, "unable to read JNI map entries");
    ScopedLocalRef<jclass> setClass(env, env->FindClass("java/util/Set"));
    requireJniMapOperation(env, "unable to resolve java.util.Set");
    jmethodID iteratorMethod = env->GetMethodID(setClass.get(), "iterator", "()Ljava/util/Iterator;");
    requireJniMapOperation(env, "unable to resolve Set.iterator");
    ScopedLocalRef<jobject> iterator(env, env->CallObjectMethod(entries.get(), iteratorMethod));
    requireJniMapOperation(env, "unable to iterate JNI map entries");
    ScopedLocalRef<jclass> iteratorClass(env, env->FindClass("java/util/Iterator"));
    requireJniMapOperation(env, "unable to resolve java.util.Iterator");
    jmethodID hasNextMethod = env->GetMethodID(iteratorClass.get(), "hasNext", "()Z");
    jmethodID nextMethod = env->GetMethodID(iteratorClass.get(), "next", "()Ljava/lang/Object;");
    requireJniMapOperation(env, "unable to resolve Iterator methods");
    ScopedLocalRef<jclass> entryClass(env, env->FindClass("java/util/Map$Entry"));
    requireJniMapOperation(env, "unable to resolve java.util.Map.Entry");
    jmethodID keyMethod = env->GetMethodID(entryClass.get(), "getKey", "()Ljava/lang/Object;");
    jmethodID valueMethod = env->GetMethodID(entryClass.get(), "getValue", "()Ljava/lang/Object;");
    requireJniMapOperation(env, "unable to resolve Map.Entry methods");

    Map output;
    while (env->CallBooleanMethod(iterator.get(), hasNextMethod) == JNI_TRUE) {
        requireJniMapOperation(env, "unable to advance JNI map iterator");
        ScopedLocalRef<jobject> entry(env, env->CallObjectMethod(iterator.get(), nextMethod));
        requireJniMapOperation(env, "unable to read JNI map entry");
        ScopedLocalRef<jobject> key(env, env->CallObjectMethod(entry.get(), keyMethod));
        requireJniMapOperation(env, "unable to read JNI map key");
        ScopedLocalRef<jobject> value(env, env->CallObjectMethod(entry.get(), valueMethod));
        requireJniMapOperation(env, "unable to read JNI map value");
        if (key.get() == nullptr) {
            throwIllegalState(env, "non-null Nexa map key was null at the JNI boundary");
            throw std::runtime_error("null JNI map key");
        }
        auto convertedKey = convertKey(key.get());
        requireJniMapOperation(env, "unable to convert JNI map key");
        if (value.get() == nullptr) {
            if constexpr (isOptionalMapValue<std::invoke_result_t<ValueConverter, jobject>>) {
                auto convertedValue = convertValue(nullptr);
                requireJniMapOperation(env, "unable to convert nullable JNI map value");
                output.insert_or_assign(std::move(convertedKey), std::move(convertedValue));
                continue;
            } else {
                throwIllegalState(env, "non-null Nexa map value was null at the JNI boundary");
                throw std::runtime_error("null JNI map value");
            }
        }
        auto convertedValue = convertValue(value.get());
        requireJniMapOperation(env, "unable to convert JNI map value");
        output.insert_or_assign(std::move(convertedKey), std::move(convertedValue));
    }
    requireJniMapOperation(env, "unable to finish reading JNI map");
    return output;
}

template <class Map, class KeyConverter, class ValueConverter>
jobject toJniMap(JNIEnv* env, const Map& input, KeyConverter&& convertKey, ValueConverter&& convertValue) {
    const auto maxCapacity = static_cast<std::size_t>(std::numeric_limits<jint>::max());
    if (input.size() > maxCapacity) {
        throw std::length_error("native plugin map is too large for JNI");
    }
    const auto expectedSize = input.size();
    const auto extraCapacity = (expectedSize + 2) / 3;
    const auto initialCapacity = std::min(maxCapacity, expectedSize + extraCapacity);
    ScopedLocalRef<jclass> mapClass(env, env->FindClass("java/util/HashMap"));
    requireJniMapOperation(env, "unable to resolve java.util.HashMap");
    jmethodID constructor = env->GetMethodID(mapClass.get(), "<init>", "(I)V");
    jmethodID putMethod = env->GetMethodID(mapClass.get(), "put", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;");
    requireJniMapOperation(env, "unable to resolve HashMap methods");
    ScopedLocalRef<jobject> output(env, env->NewObject(mapClass.get(), constructor, static_cast<jint>(initialCapacity)));
    requireJniMapOperation(env, "unable to create JNI map");
    for (const auto& item : input) {
        ScopedLocalRef<jobject> key(env, convertKey(item.first));
        requireJniMapOperation(env, "unable to convert native map key");
        ScopedLocalRef<jobject> value(env, convertValue(item.second));
        requireJniMapOperation(env, "unable to convert native map value");
        ScopedLocalRef<jobject> previous(env, env->CallObjectMethod(output.get(), putMethod, key.get(), value.get()));
        requireJniMapOperation(env, "unable to populate JNI map");
    }
    return output.release();
}
} // namespace
"#;
    PRELUDE.replace("__NEXA_CPP_NAMESPACE__", &cpp_namespace(plugin_id))
}

fn render_kotlin_native_declaration(
    out: &mut SourceWriter,
    method: &BridgeMethod,
    native_name: &str,
) {
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                parameter.name,
                android_kotlin_type(&parameter.ty, true)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = android_kotlin_type(method.success_type(), true);
    out.push_str(&format!(
        "    external fun {native_name}({parameters}): {return_type}\n"
    ));
}

fn render_kotlin_class_native_declaration(
    out: &mut SourceWriter,
    method: &BridgeMethod,
    native_name: &str,
) {
    let mut parameters = vec!["handle: Long".to_owned()];
    parameters.extend(method.parameters.iter().map(|parameter| {
        format!(
            "{}: {}",
            parameter.name,
            android_kotlin_type(&parameter.ty, true)
        )
    }));
    let return_type = android_kotlin_type(method.success_type(), true);
    out.push_str(&format!(
        "    external fun {native_name}({}): {return_type}\n",
        parameters.join(", ")
    ));
}

fn render_kotlin_property_native_declarations(
    out: &mut SourceWriter,
    property: &BridgeProperty,
    interface: &str,
    getter_name: &str,
) {
    let value_type = android_kotlin_type(&property.ty, true);
    out.push_str(&format!(
        "    external fun {getter_name}(handle: Long): {value_type}\n"
    ));
    if property.mutable {
        out.push_str(&format!(
            "    external fun set_{interface}_{}(handle: Long, value: {value_type})\n",
            property.name
        ));
    }
}

fn render_kotlin_native_dispose_declaration(out: &mut SourceWriter, native_name: &str) {
    out.push_str(&format!("    external fun {native_name}(handle: Long)\n"));
}

fn render_kotlin_native_event_declaration(out: &mut SourceWriter, native_name: &str) {
    out.push_str(&format!(
        "    external fun {native_name}(handle: Long, callback: Any?)\n"
    ));
}

fn render_kotlin_service_adapter(
    out: &mut SourceWriter,
    method: &BridgeMethod,
    native_name: &str,
    bindings_class: &str,
) {
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                parameter.name,
                android_kotlin_type(&parameter.ty, false)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| kotlin_to_jni_expression(&parameter.name, &parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let success_type = method.success_type();
    let return_type = android_kotlin_type(success_type, false);
    if method.is_async {
        let native_call = format!("{bindings_class}.{native_name}({arguments})");
        let expression = if return_type == "Unit" {
            native_call
        } else {
            kotlin_from_jni_expression(native_call, success_type)
        };
        out.push_str(&format!(
            "    override suspend fun {}({parameters}): {return_type} = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {{ {expression} }}\n",
            method.name
        ));
        return;
    }
    if return_type == "Unit" {
        out.push_str(&format!(
            "    override fun {}({parameters}) {{ {bindings_class}.{native_name}({arguments}) }}\n",
            method.name
        ));
    } else {
        let native_call = format!("{bindings_class}.{native_name}({arguments})");
        let result = kotlin_from_jni_expression(native_call, success_type);
        out.push_str(&format!(
            "    override fun {}({parameters}): {return_type} = {result}\n",
            method.name
        ));
    }
}

fn kotlin_to_jni_expression(value: &str, ty: &BridgeType) -> String {
    // Split the historical optional flag from the shape so the branches
    // below mirror the renderer's original structure exactly.
    let (shape, optional) = match ty {
        BridgeType::Optional(inner) => (inner.as_ref(), true),
        other => (other, false),
    };
    if let BridgeType::Map(key, map_value) = shape {
        let mut expression = if optional {
            "nexaOptionalMap".to_owned()
        } else {
            value.to_owned()
        };
        if let Some(conversion) = android_cpp_value(key).and_then(|scalar| scalar.kotlin_to_jni) {
            expression = format!("{expression}.mapKeys {{ it.key.{conversion}() }}");
        }
        let converted_value = kotlin_to_jni_expression("nexaMapValue", map_value);
        if converted_value != "nexaMapValue" {
            expression =
                format!("{expression}.mapValues {{ (_, nexaMapValue) -> {converted_value} }}");
        }
        if (!optional && expression == value) || (optional && expression == "nexaOptionalMap") {
            return value.to_owned();
        }
        return if optional {
            format!("{value}?.let {{ nexaOptionalMap -> {expression} }}")
        } else {
            expression
        };
    }
    if let BridgeType::Array(element) | BridgeType::Set(element) = shape {
        let is_set = matches!(shape, BridgeType::Set(_));
        if !is_set && matches!(element.as_ref(), BridgeType::Array(_) | BridgeType::Set(_))
            || (!is_set && matches!(element.as_ref(), BridgeType::Map(..)))
            || element.is_optional()
        {
            return format!(
                "{value}.map {{ nexaNestedCollection -> {} }}.toTypedArray()",
                kotlin_to_jni_expression("nexaNestedCollection", element)
            );
        }
        let Some(array) = android_primitive_array(element) else {
            debug_assert!(android_reference_array_element(element).is_some());
            return format!("{value}.toTypedArray()");
        };
        if is_set {
            let Some(scalar) = android_cpp_value(element) else {
                debug_assert!(false, "validated set elements map to an Android value");
                return format!("{value}.toTypedArray()");
            };
            let source = if let Some(conversion) = scalar.kotlin_to_jni {
                format!("{value}.map {{ it.{conversion}() }}")
            } else {
                value.to_owned()
            };
            return format!(
                "{source}.to{}Array()",
                array.kotlin_array.trim_end_matches("Array")
            );
        }
        if let Some(conversion) = android_cpp_value(element).and_then(|scalar| scalar.kotlin_to_jni)
        {
            return format!(
                "{}({value}.size) {{ {value}[it].{conversion}() }}",
                array.kotlin_array
            );
        }
        return format!(
            "{value}.to{}Array()",
            array.kotlin_array.trim_end_matches("Array")
        );
    }
    if android_cpp_value(shape).is_none() {
        return value.to_owned();
    }
    let Some(scalar) = android_cpp_value(shape) else {
        debug_assert!(false, "validated scalar shape should map");
        return value.to_owned();
    };
    if optional && scalar.unsigned {
        // A nullable signed Long carrier preserves every UInt64 bit pattern and null.
        return format!("{value}?.toLong()");
    }
    scalar.kotlin_to_jni.map_or_else(
        || value.to_owned(),
        |conversion| format!("{value}.{conversion}()"),
    )
}

fn kotlin_from_jni_expression(expression: String, ty: &BridgeType) -> String {
    let (shape, optional) = match ty {
        BridgeType::Optional(inner) => (inner.as_ref(), true),
        other => (other, false),
    };
    if let BridgeType::Map(key, map_value) = shape {
        let mut result = if optional {
            "nexaOptionalMap".to_owned()
        } else {
            expression.clone()
        };
        if let Some(conversion) = android_cpp_value(key).and_then(|scalar| scalar.kotlin_from_jni) {
            result = format!("{result}.mapKeys {{ it.key.{conversion}() }}");
        }
        let converted_value = kotlin_from_jni_expression("nexaMapValue".to_owned(), map_value);
        if converted_value != "nexaMapValue" {
            result = format!("{result}.mapValues {{ (_, nexaMapValue) -> {converted_value} }}");
        }
        if (!optional && result == expression) || (optional && result == "nexaOptionalMap") {
            return expression;
        }
        return if optional {
            format!("{expression}?.let {{ nexaOptionalMap -> {result} }}")
        } else {
            result
        };
    }
    if let BridgeType::Array(element) | BridgeType::Set(element) = shape {
        let is_set = matches!(shape, BridgeType::Set(_));
        if !is_set && matches!(element.as_ref(), BridgeType::Array(_) | BridgeType::Set(_))
            || (!is_set && matches!(element.as_ref(), BridgeType::Map(..)))
            || element.is_optional()
        {
            let converted = format!(
                "{expression}.map {{ nexaNestedCollection -> {} }}",
                kotlin_from_jni_expression("nexaNestedCollection".to_owned(), element)
            );
            return if is_set {
                format!("{converted}.toSet()")
            } else {
                converted
            };
        }
        let Some(_array) = android_primitive_array(element) else {
            debug_assert!(android_reference_array_element(element).is_some());
            return if is_set {
                format!("{expression}.toSet()")
            } else {
                format!("{expression}.asList()")
            };
        };
        if is_set {
            if let Some(conversion) =
                android_cpp_value(element).and_then(|scalar| scalar.kotlin_from_jni)
            {
                return format!("{expression}.map {{ it.{conversion}() }}.toSet()");
            }
            return format!("{expression}.toSet()");
        }
        if let Some(conversion) =
            android_cpp_value(element).and_then(|scalar| scalar.kotlin_from_jni)
        {
            return format!("{expression}.map {{ it.{conversion}() }}");
        }
        return format!("{expression}.asList()");
    }
    if android_cpp_value(shape).is_none() {
        return expression;
    }
    let Some(scalar) = android_cpp_value(shape) else {
        debug_assert!(false, "validated scalar shape should map");
        return expression;
    };
    if optional && scalar.unsigned {
        let Some(conversion) = scalar.kotlin_from_jni else {
            debug_assert!(false, "unsigned Kotlin values have a carrier conversion");
            return expression;
        };
        return format!("{expression}?.{conversion}()");
    }
    match scalar.kotlin_from_jni {
        Some(conversion) => format!("{expression}.{conversion}()"),
        None => expression,
    }
}

fn render_kotlin_cpp_class(
    out: &mut SourceWriter,
    interface: &BridgeInterface,
    plugin_index: usize,
    constructor: Option<&BridgeConstructor>,
) {
    let parameters = constructor
        .map(|constructor| {
            constructor
                .parameters
                .iter()
                .map(|parameter| {
                    format!(
                        "{}: {}",
                        parameter.name,
                        android_kotlin_type(&parameter.ty, false)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let arguments = constructor
        .map(|constructor| {
            constructor
                .parameters
                .iter()
                .map(|parameter| kotlin_to_jni_expression(&parameter.name, &parameter.ty))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    out.push_str(&format!("\npublic class {}Impl private constructor(private var nativeHandle: Long) : {}Spec, java.lang.AutoCloseable {{\n    public constructor({parameters}) : this(NexaPlugin{plugin_index}_CppBindings.create_{}({arguments}))\n    private fun requireNativeHandle(): Long = checkNotNull(nativeHandle.takeIf {{ it != 0L }}) {{ \"{} is disposed\" }}\n", interface.name, interface.name, interface.name, interface.name));
    for property in &interface.properties {
        let value_type = android_kotlin_type(&property.ty, false);
        let getter = format!("get_{}_{}", interface.name, property.name);
        let getter_call =
            format!("NexaPlugin{plugin_index}_CppBindings.{getter}(requireNativeHandle())");
        let getter_value = kotlin_from_jni_expression(getter_call, &property.ty);
        out.push_str(&format!(
            "    override {} {}: {value_type}\n        @Synchronized\n        get() = {getter_value}",
            if property.mutable { "var" } else { "val" },
            property.name
        ));
        if property.mutable {
            let value = kotlin_to_jni_expression("value", &property.ty);
            out.push_str(&format!(
                "\n        @Synchronized\n        set(value) = NexaPlugin{plugin_index}_CppBindings.set_{}_{}(requireNativeHandle(), {value})",
                interface.name,
                property.name
            ));
        }
        out.push('\n');
    }
    for event in &interface.events {
        let property = nexa_plugin_idl::event_callback_property(&event.name);
        let bridge = android_cpp_event_bridge_name(plugin_index, interface, event);
        let setter = format!("set_{}_{}", interface.name, property);
        let callback_type = android_kotlin_event_callback_type(event);
        let backing = android_cpp_event_backing_name(interface, event);
        out.push_str(&format!(
            "    private var {backing}: {bridge}? = null\n    override var {property}: {callback_type}?\n        @Synchronized get() = {backing}?.callback\n        @Synchronized set(value) {{\n            val previous = {backing}\n            previous?.deactivate()\n            val next = value?.let {{ {bridge}(it) }}\n            NexaPlugin{plugin_index}_CppBindings.{setter}(requireNativeHandle(), next)\n            {backing} = next\n        }}\n"
        ));
    }
    for method in &interface.methods {
        if method.name == "dispose" {
            continue;
        }
        let parameters = method
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    parameter.name,
                    android_kotlin_type(&parameter.ty, false)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let arguments = std::iter::once("requireNativeHandle()".to_owned())
            .chain(
                method
                    .parameters
                    .iter()
                    .map(|parameter| kotlin_to_jni_expression(&parameter.name, &parameter.ty)),
            )
            .collect::<Vec<_>>()
            .join(", ");
        let success_type = method.success_type();
        let return_type = android_kotlin_type(success_type, false);
        let native_name = format!("call_{}_{}", interface.name, method.name);
        if method.is_async {
            let call = if return_type == "Unit" {
                format!("NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments})")
            } else {
                let native_call =
                    format!("NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments})");
                kotlin_from_jni_expression(native_call, success_type)
            };
            out.push_str(&format!(
                "    override suspend fun {}({parameters}): {return_type} = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {{ synchronized(this) {{ {call} }} }}\n",
                method.name
            ));
        } else if return_type == "Unit" {
            out.push_str("    @Synchronized\n");
            out.push_str(&format!("    override fun {}({parameters}) {{ NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments}) }}\n", method.name));
        } else {
            out.push_str("    @Synchronized\n");
            let native_call =
                format!("NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments})");
            let result = kotlin_from_jni_expression(native_call, success_type);
            out.push_str(&format!(
                "    override fun {}({parameters}): {return_type} = {result}\n",
                method.name
            ));
        }
    }
    out.push_str("    @Synchronized\n    override fun dispose() {\n        val handle = nativeHandle\n        if (handle == 0L) return\n        nativeHandle = 0L\n");
    for event in &interface.events {
        let backing = android_cpp_event_backing_name(interface, event);
        out.push_str(&format!(
            "        {backing}?.deactivate()\n        {backing} = null\n"
        ));
    }
    out.push_str(&format!(
        "        NexaPlugin{plugin_index}_CppBindings.dispose_{}(handle)\n    }}\n    override fun close() = dispose()\n    @Suppress(\"DEPRECATION\")\n    protected fun finalize() {{\n        if (nativeHandle != 0L) dispose()\n    }}\n}}\n",
        interface.name
    ));
}

fn android_cpp_event_backing_name(interface: &BridgeInterface, event: &BridgeEvent) -> String {
    let mut occupied = std::collections::BTreeSet::new();
    occupied.extend(
        interface
            .properties
            .iter()
            .map(|property| property.name.clone()),
    );
    occupied.extend(interface.methods.iter().map(|method| method.name.clone()));
    occupied.extend(
        interface
            .events
            .iter()
            .map(|event| nexa_plugin_idl::event_callback_property(&event.name)),
    );
    let base = format!("nexaCppEvent{}Bridge", cpp_identifier(&event.name));
    let mut candidate = base.clone();
    let mut suffix = 1usize;
    while occupied.contains(&candidate) {
        candidate = format!("{base}{suffix}");
        suffix += 1;
    }
    candidate
}

fn render_kotlin_event_bridge(
    out: &mut SourceWriter,
    interface: &BridgeInterface,
    event: &BridgeEvent,
    plugin_index: usize,
) {
    let name = android_cpp_event_bridge_name(plugin_index, interface, event);
    let callback_type = android_kotlin_event_callback_type(event);
    out.push_str(&format!("private class {name}(val callback: {callback_type}) {{\n    @Volatile private var active = true\n    fun deactivate() {{ active = false }}\n"));
    let parameters = event
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            format!(
                "nexaArg{index}: {}",
                android_kotlin_type(&parameter.ty, true)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let conversions = event
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            format!(
                "        val nexaEventValue{index} = {}\n",
                kotlin_from_jni_expression(format!("nexaArg{index}"), &parameter.ty)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let arguments = (0..event.parameters.len())
        .map(|index| format!("nexaEventValue{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "    @JvmName(\"nexaDispatch\")\n    fun dispatchFromNative({parameters}) {{\n{}        if (!active) return\n        NexaPlugin{plugin_index}_CppEventDispatcher.post {{ if (active) callback({arguments}) }}\n    }}\n}}\n\n",
        conversions
    ));
}

fn android_cpp_event_bridge_name(
    plugin_index: usize,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) -> String {
    format!(
        "NexaPlugin{plugin_index}_CppEvent{}_{}",
        cpp_identifier(&interface.name),
        cpp_identifier(&event.name)
    )
}

fn android_kotlin_event_callback_type(event: &BridgeEvent) -> String {
    let parameters = event
        .parameters
        .iter()
        .map(|parameter| android_kotlin_type(&parameter.ty, false))
        .collect::<Vec<_>>()
        .join(", ");
    format!("(({parameters}) -> Unit)")
}

fn render_jni_service_method(
    out: &mut SourceWriter,
    package: &str,
    class_name: &str,
    interface_name: &str,
    method: &BridgeMethod,
    native_name: &str,
) {
    let symbol = jni_symbol(package, class_name, native_name);
    let parameters = jni_parameter_declarations(&method.parameters);
    let success_type = method.success_type();
    let return_type = android_jni_type(success_type);
    let method_name = cpp_identifier(&method.name);
    let prefix = if parameters.is_empty() {
        String::new()
    } else {
        format!(", {parameters}")
    };
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT {return_type} JNICALL {symbol}(JNIEnv* env, jobject thiz{prefix}) {{\n    (void)thiz;\n",
    ));
    render_jni_boundary_start(out, &return_type);
    let arguments =
        render_jni_argument_conversions(out, &method.parameters, jni_failure_return(&return_type));
    let call = format!(
        "{}::{}({})",
        cpp_identifier(interface_name),
        method_name,
        arguments.join(", ")
    );
    let call = if method.is_async {
        out.push_str(&format!("        auto nexaCppFuture = {call};\n"));
        "nexaCppFuture.get()".to_owned()
    } else {
        call
    };
    if let Some(error_name) = method.error_type() {
        render_jni_typed_error_result(
            out,
            success_type,
            error_name,
            &call,
            jni_failure_return(&return_type),
        );
    } else if success_type.is_void() {
        out.push_str(&format!("        {call};\n"));
    } else {
        render_jni_return(out, success_type, &call);
    }
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_constructor(
    out: &mut SourceWriter,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &BridgeInterface,
    native_name: &str,
) {
    let symbol = jni_symbol(package, class_name, native_name);
    let constructor = interface.constructors.first();
    let parameters = constructor
        .map(|constructor| jni_parameter_declarations(&constructor.parameters))
        .unwrap_or_default();
    let prefix = if parameters.is_empty() {
        String::new()
    } else {
        format!(", {parameters}")
    };
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT jlong JNICALL {symbol}(JNIEnv* env, jobject thiz{prefix}) {{\n    (void)thiz;\n"
    ));
    render_jni_boundary_start(out, "jlong");
    let arguments = constructor
        .map(|constructor| {
            render_jni_argument_conversions(out, &constructor.parameters, "return 0;")
        })
        .unwrap_or_default();
    out.push_str(&format!(
        "        auto instance = {}::{}({});\n        if (!instance) throw std::runtime_error(\"native plugin factory returned null\");\n        auto* handle = new std::unique_ptr<{cpp_type_name}>(std::move(instance));\n        return static_cast<jlong>(reinterpret_cast<std::uintptr_t>(handle));\n",
        cpp_namespace(plugin_id),
        cpp_factory_name(&interface.name),
        arguments.join(", ")
    ));
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_property(
    out: &mut SourceWriter,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &BridgeInterface,
    property: &BridgeProperty,
    getter_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let return_type = android_jni_type(&property.ty);
    let getter_symbol = jni_symbol(package, class_name, getter_name);
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT {return_type} JNICALL {getter_symbol}(JNIEnv* env, jobject thiz, jlong rawHandle) {{\n    (void)thiz;\n",
    ));
    render_jni_boundary_start(out, &return_type);
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) {}\n",
        jni_failure_return(&return_type)
    ));
    render_jni_return(
        out,
        &property.ty,
        &format!("(*handle)->{}()", cpp_getter_name(&property.name)),
    );
    render_jni_boundary_end(out);
    out.push_str("}\n\n");

    if property.mutable {
        let setter_name = format!("set_{}_{}", interface.name, property.name);
        let setter_symbol = jni_symbol(package, class_name, &setter_name);
        out.push_str(&format!(
            "extern \"C\" JNIEXPORT void JNICALL {setter_symbol}(JNIEnv* env, jobject thiz, jlong rawHandle, {return_type} value) {{\n    (void)thiz;\n",
        ));
        render_jni_boundary_start(out, "void");
        out.push_str(&format!(
            "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) return;\n"
        ));
        let value = if matches!(bridge_strip_optional(&property.ty), BridgeType::Map(..)) {
            render_jni_map_argument_conversion(
                out,
                &property.ty,
                "value",
                "return;",
                "nexaJniMapProperty",
            )
        } else if matches!(&property.ty, BridgeType::Array(_) | BridgeType::Set(_)) {
            render_jni_array_argument_conversion(
                out,
                &property.ty,
                "value",
                "return;",
                "nexaJniArrayProperty",
            )
        } else if let Some(scalar) = android_cpp_value(&property.ty) {
            if scalar.optional {
                render_jni_optional_argument_conversion(
                    out,
                    &property.ty,
                    "value",
                    "return;",
                    "nexaJniOptionalProperty",
                )
            } else if return_type == "jstring" {
                out.push_str(
                    "        auto nexaJniStringValue = fromJniString(env, value);\n        if (env->ExceptionCheck()) return;\n",
                );
                "std::move(nexaJniStringValue)".to_owned()
            } else if return_type == "jbyteArray" {
                out.push_str(
                    "        auto nexaJniBytesValue = fromJniBytes(env, value);\n        if (env->ExceptionCheck()) return;\n",
                );
                "std::move(nexaJniBytesValue)".to_owned()
            } else {
                format!("static_cast<{}>(value)", android_cpp_type(&property.ty))
            }
        } else {
            // Declared struct/enum property. Plan validation admits named
            // values here, and they cross JNI as the same `jobject` the getter
            // returns, so the setter reuses the named-value reader the method
            // argument path already relies on.
            let name = cpp_identifier(bridge_type_name(&property.ty));
            out.push_str(&format!(
                "        auto nexaJniNamedProperty = nexaFromJni{name}(env, static_cast<jobject>(value));\n        if (env->ExceptionCheck()) return;\n"
            ));
            "std::move(nexaJniNamedProperty)".to_owned()
        };
        out.push_str(&format!(
            "        (*handle)->{}({value});\n",
            cpp_setter_name(&property.name)
        ));
        render_jni_boundary_end(out);
        out.push_str("}\n\n");
    }
}

fn render_jni_class_method(
    out: &mut SourceWriter,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &BridgeInterface,
    method: &BridgeMethod,
    native_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let success_type = method.success_type();
    let return_type = android_jni_type(success_type);
    let failure_return = jni_failure_return(&return_type);
    let symbol = jni_symbol(package, class_name, native_name);
    let mut parameters = String::from("jlong rawHandle");
    for parameter in &method.parameters {
        parameters.push_str(&format!(
            ", {} {}",
            android_jni_type(&parameter.ty),
            parameter.name
        ));
    }
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT {return_type} JNICALL {symbol}(JNIEnv* env, jobject thiz, {parameters}) {{\n    (void)thiz;\n",
    ));
    render_jni_boundary_start(out, &return_type);
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) {failure_return}\n"
    ));
    let arguments = render_jni_argument_conversions(out, &method.parameters, failure_return);
    let call = format!(
        "(*handle)->{}({})",
        cpp_identifier(&method.name),
        arguments.join(", ")
    );
    let call = if method.is_async {
        out.push_str(&format!("        auto nexaCppFuture = {call};\n"));
        "nexaCppFuture.get()".to_owned()
    } else {
        call
    };
    if let Some(error_name) = method.error_type() {
        render_jni_typed_error_result(out, success_type, error_name, &call, failure_return);
    } else if success_type.is_void() {
        out.push_str(&format!("        {call};\n"));
    } else {
        render_jni_return(out, success_type, &call);
    }
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_dispose(
    out: &mut SourceWriter,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &BridgeInterface,
    native_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let symbol = jni_symbol(package, class_name, native_name);
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT void JNICALL {symbol}(JNIEnv* env, jobject thiz, jlong rawHandle) {{\n    (void)thiz;\n"
    ));
    render_jni_boundary_start(out, "void");
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) return;\n        std::unique_ptr<{cpp_type_name}> instance = std::move(*handle);\n        delete handle;\n"
    ));
    for event in &interface.events {
        out.push_str(&format!(
            "        instance->{}({{}});\n",
            cpp_setter_name(&nexa_plugin_idl::event_callback_property(&event.name))
        ));
    }
    out.push_str("        instance->dispose();\n");
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_event_setter(
    out: &mut SourceWriter,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &BridgeInterface,
    event: &BridgeEvent,
    native_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let symbol = jni_symbol(package, class_name, native_name);
    let descriptor = event
        .parameters
        .iter()
        .map(|parameter| android_jni_callback_descriptor(&parameter.ty, package))
        .collect::<String>();
    let setter = cpp_setter_name(&nexa_plugin_idl::event_callback_property(&event.name));
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT void JNICALL {symbol}(JNIEnv* env, jobject thiz, jlong rawHandle, jobject callback) {{\n    (void)thiz;\n"
    ));
    render_jni_boundary_start(out, "void");
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) return;\n"
    ));
    out.push_str(
        "        jmethodID nexaDispatch = nullptr;\n        std::shared_ptr<JniGlobalReference> nexaCallback;\n",
    );
    out.push_str(
        "        if (callback != nullptr) {\n            ScopedLocalRef<jclass> nexaCallbackClass(env, env->GetObjectClass(callback));\n            if (nexaCallbackClass.get() == nullptr) return;\n",
    );
    out.push_str(&format!(
        "            nexaDispatch = env->GetMethodID(nexaCallbackClass.get(), \"nexaDispatch\", \"({descriptor})V\");\n            if (nexaDispatch == nullptr) return;\n            nexaCallback = std::make_shared<JniGlobalReference>(env, callback);\n            if (nexaCallback->get() == nullptr) return;\n        }}\n        if (callback == nullptr) {{ (*handle)->{setter}({{}}); return; }}\n"
    ));
    let lambda_parameters = event
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| format!("const {}& nexaValue{index}", cpp_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "        (*handle)->{setter}([nexaCallback, nexaDispatch]({lambda_parameters}) noexcept {{\n            try {{\n                ScopedJniEnvironment nexaEnvironment(nexaCallback->vm());\n                JNIEnv* nexaEnv = nexaEnvironment.get();\n                if (nexaEnv == nullptr) return;\n                JNIEnv* env = nexaEnv;\n                ScopedJniLocalFrame nexaLocalFrame(nexaEnv, {});\n                if (!nexaLocalFrame.active()) {{ if (nexaEnv->ExceptionCheck()) nexaEnv->ExceptionClear(); return; }}\n                ScopedLocalRef<jobject> nexaCallbackLocal(nexaEnv, nexaEnv->NewLocalRef(nexaCallback->get()));\n                if (nexaCallbackLocal.get() == nullptr || nexaEnv->ExceptionCheck()) {{ if (nexaEnv->ExceptionCheck()) nexaEnv->ExceptionClear(); return; }}\n                std::array<jvalue, {}> nexaArguments{{}};\n",
        (event.parameters.len() * 4 + 8).max(16),
        event.parameters.len(),
    ));
    for (index, parameter) in event.parameters.iter().enumerate() {
        let jni_type = android_jni_type(&parameter.ty);
        out.push_str(&format!(
            "                auto nexaArgument{index} = [&]() -> {jni_type} {{\n"
        ));
        render_jni_return(out, &parameter.ty, &format!("nexaValue{index}"));
        out.push_str("                }();\n                if (nexaEnv->ExceptionCheck()) { nexaEnv->ExceptionClear(); return; }\n");
        let field = match jni_type.as_str() {
            "jboolean" => "z",
            "jbyte" => "b",
            "jshort" => "s",
            "jint" => "i",
            "jlong" => "j",
            "jfloat" => "f",
            "jdouble" => "d",
            _ => "l",
        };
        let value = if field == "l" {
            format!("static_cast<jobject>(nexaArgument{index})")
        } else {
            format!("nexaArgument{index}")
        };
        out.push_str(&format!(
            "                nexaArguments[{index}].{field} = {value};\n"
        ));
    }
    out.push_str(
        "                nexaEnv->CallVoidMethodA(nexaCallbackLocal.get(), nexaDispatch, nexaArguments.data());\n                if (nexaEnv->ExceptionCheck()) nexaEnv->ExceptionClear();\n            } catch (...) { }\n        });\n",
    );
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn android_jni_callback_descriptor(ty: &BridgeType, package: &str) -> String {
    if matches!(bridge_strip_optional(ty), BridgeType::Map(..)) {
        return "Ljava/util/Map;".to_owned();
    }
    if let BridgeType::Array(element) | BridgeType::Set(element) = bridge_strip_optional(ty) {
        if android_primitive_array(element).is_some() {
            let code = android_primitive_descriptor(element);
            return format!("[{code}");
        }
        return format!("[{}", android_jni_callback_descriptor(element, package));
    }
    let Some(scalar) = android_cpp_value(ty) else {
        // Named declarations (and the historical spelling of any other
        // compound shape) use the package-qualified name descriptor.
        return format!("L{}/{};", package.replace('.', "/"), bridge_type_name(ty));
    };
    if let BridgeType::Optional(inner) = ty {
        return match inner.as_ref() {
            BridgeType::Scalar(BridgeScalar::String) => "Ljava/lang/String;".to_owned(),
            BridgeType::Scalar(BridgeScalar::Bytes) => "[B".to_owned(),
            BridgeType::Scalar(_) => {
                let wrapper = match scalar.jni_kotlin {
                    "Boolean" => "java/lang/Boolean",
                    "Byte" => "java/lang/Byte",
                    "Short" => "java/lang/Short",
                    "Int" => "java/lang/Integer",
                    "Long" => "java/lang/Long",
                    "Float" => "java/lang/Float",
                    "Double" => "java/lang/Double",
                    _ => {
                        debug_assert!(
                            false,
                            "validated optional scalar carrier should have a wrapper"
                        );
                        "java/lang/Object"
                    }
                };
                format!("L{wrapper};")
            }
            _ => {
                debug_assert!(false, "validated optional callback types should be scalars");
                "Ljava/lang/Object;".to_owned()
            }
        };
    }
    match scalar.jni {
        "jboolean" => "Z".to_owned(),
        "jbyte" => "B".to_owned(),
        "jshort" => "S".to_owned(),
        "jint" => "I".to_owned(),
        "jlong" => "J".to_owned(),
        "jfloat" => "F".to_owned(),
        "jdouble" => "D".to_owned(),
        "jstring" => "Ljava/lang/String;".to_owned(),
        "jbyteArray" => "[B".to_owned(),
        // The scalar table only produces these carriers; anything else
        // cannot occur for a validated scalar shape.
        _ => {
            debug_assert!(false, "scalar JNI carriers are covered above");
            "Ljava/lang/Object;".to_owned()
        }
    }
}

fn render_jni_boundary_start(out: &mut SourceWriter, return_type: &str) {
    out.push_str(&format!(
        "    return withJniExceptions(env, [&]() -> {return_type} {{\n"
    ));
}

fn render_jni_boundary_end(out: &mut SourceWriter) {
    out.push_str("    });\n");
}

fn jni_failure_return(return_type: &str) -> &'static str {
    if return_type == "void" {
        "return;"
    } else {
        "return {};"
    }
}

fn render_android_jni_named_value_helpers(
    out: &mut SourceWriter,
    plan: &BridgePlan,
    package: &str,
) {
    let types = collect_value_types(plan);
    if types.is_empty() {
        return;
    }
    out.push_str("\nnamespace {\n");
    for ty in &types {
        let name = cpp_identifier(&ty.name);
        out.push_str(&format!(
            "{name} nexaFromJni{name}(JNIEnv* env, jobject raw);\njobject nexaToJni{name}(JNIEnv* env, const {name}& value);\n"
        ));
    }
    for ty in types {
        let name = cpp_identifier(&ty.name);
        let class_name = format!("{}/{}", package.replace('.', "/"), ty.name);
        match ty.kind {
            BridgeTypeKind::Enum => {
                let values_descriptor = format!("()[L{};", class_name);
                out.push_str(&format!(
                    "{name} nexaFromJni{name}(JNIEnv* env, jobject raw) {{\n    if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa enum was null at the JNI boundary\"); throw std::runtime_error(\"null JNI enum\"); }}\n    ScopedLocalRef<jclass> enumClass(env, env->FindClass(\"{class_name}\"));\n    if (enumClass.get() == nullptr) throw std::runtime_error(\"unable to resolve Kotlin enum class\");\n    jmethodID ordinalMethod = env->GetMethodID(enumClass.get(), \"ordinal\", \"()I\");\n    if (ordinalMethod == nullptr) throw std::runtime_error(\"unable to resolve Kotlin enum ordinal\");\n    const jint ordinal = env->CallIntMethod(raw, ordinalMethod);\n    if (env->ExceptionCheck()) throw std::runtime_error(\"unable to read Kotlin enum ordinal\");\n    if (ordinal < 0 || ordinal >= {}) {{ throwIllegalState(env, \"Kotlin enum ordinal is outside the IDL definition\"); throw std::runtime_error(\"invalid JNI enum ordinal\"); }}\n    return static_cast<{name}>(ordinal);\n}}\n\njobject nexaToJni{name}(JNIEnv* env, const {name}& value) {{\n    const auto ordinal = static_cast<std::uint8_t>(value);\n    if (ordinal >= {}) {{ throwIllegalState(env, \"native enum value is outside the IDL definition\"); return nullptr; }}\n    ScopedLocalRef<jclass> enumClass(env, env->FindClass(\"{class_name}\"));\n    if (enumClass.get() == nullptr) return nullptr;\n    jmethodID valuesMethod = env->GetStaticMethodID(enumClass.get(), \"values\", \"{values_descriptor}\");\n    if (valuesMethod == nullptr) return nullptr;\n    ScopedLocalRef<jobjectArray> values(env, static_cast<jobjectArray>(env->CallStaticObjectMethod(enumClass.get(), valuesMethod)));\n    if (values.get() == nullptr || env->ExceptionCheck()) return nullptr;\n    return env->GetObjectArrayElement(values.get(), static_cast<jsize>(ordinal));\n}}\n\n",
                    ty.cases.len(),
                    ty.cases.len()
                ));
            }
            BridgeTypeKind::Struct => {
                let descriptor = ty
                    .fields
                    .iter()
                    .map(|field| android_jni_callback_descriptor(&field.ty, package))
                    .collect::<String>();
                let mut constructor_descriptor = format!("({descriptor})V");
                if ty.fields.is_empty() {
                    constructor_descriptor = "()V".to_owned();
                }
                out.push_str(&format!(
                    "{name} nexaFromJni{name}(JNIEnv* env, jobject raw) {{\n    if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa struct was null at the JNI boundary\"); throw std::runtime_error(\"null JNI struct\"); }}\n    ScopedLocalRef<jclass> valueClass(env, env->FindClass(\"{class_name}\"));\n    if (valueClass.get() == nullptr) throw std::runtime_error(\"unable to resolve Kotlin data class\");\n    {name} result{{}};\n"
                ));
                for (index, field) in ty.fields.iter().enumerate() {
                    let field_name = cpp_identifier(&field.name);
                    let getter = android_kotlin_getter_name(&field.name);
                    let signature = android_jni_callback_descriptor(&field.ty, package);
                    let jni_type = android_jni_type(&field.ty);
                    let method = android_jni_call_method(&jni_type);
                    out.push_str(&format!(
                        "    jmethodID getter{index} = env->GetMethodID(valueClass.get(), \"{getter}\", \"(){signature}\");\n    if (getter{index} == nullptr) throw std::runtime_error(\"unable to resolve Kotlin data-class getter\");\n"
                    ));
                    if android_jni_type_is_reference(&jni_type) {
                        out.push_str(&format!(
                            "    ScopedLocalRef<jobject> field{index}(env, env->{method}(raw, getter{index}));\n    if (env->ExceptionCheck()) throw std::runtime_error(\"unable to read Kotlin data-class field\");\n"
                        ));
                    } else {
                        out.push_str(&format!(
                            "    const {jni_type} field{index} = env->{method}(raw, getter{index});\n    if (env->ExceptionCheck()) throw std::runtime_error(\"unable to read Kotlin data-class field\");\n"
                        ));
                    }
                    let conversion = android_jni_to_cpp_field(&field.ty, index);
                    out.push_str(&format!("    result.{field_name} = {conversion};\n"));
                }
                out.push_str("    return result;\n}\n\n");
                out.push_str(&format!(
                    "jobject nexaToJni{name}(JNIEnv* env, const {name}& value) {{\n    ScopedLocalRef<jclass> valueClass(env, env->FindClass(\"{class_name}\"));\n    if (valueClass.get() == nullptr) return nullptr;\n    jmethodID constructor = env->GetMethodID(valueClass.get(), \"<init>\", \"{constructor_descriptor}\");\n    if (constructor == nullptr) return nullptr;\n    std::array<jvalue, {}> arguments{{}};\n",
                    ty.fields.len()
                ));
                for (index, field) in ty.fields.iter().enumerate() {
                    let field_name = cpp_identifier(&field.name);
                    let jni_type = android_jni_type(&field.ty);
                    let conversion =
                        android_cpp_to_jni_field(&field.ty, &format!("value.{field_name}"));
                    if android_jni_type_is_reference(&jni_type) {
                        out.push_str(&format!(
                            "    ScopedLocalRef<jobject> argument{index}(env, {conversion});\n    if (env->ExceptionCheck()) return nullptr;\n    arguments[{index}].l = argument{index}.get();\n"
                        ));
                    } else {
                        let field = android_jvalue_field(&jni_type);
                        out.push_str(&format!("    arguments[{index}].{field} = {conversion};\n"));
                    }
                }
                out.push_str("    return env->NewObjectA(valueClass.get(), constructor, arguments.data());\n}\n\n");
            }
            BridgeTypeKind::Error => unreachable!("errors are not Android value types"),
        }
    }
    out.push_str("} // namespace\n");
}

fn android_jni_to_cpp_field(ty: &BridgeType, index: usize) -> String {
    if let Some(value) = android_cpp_value(ty) {
        return match bridge_type_name(ty) {
            "String" => format!("fromJniString(env, static_cast<jstring>(field{index}.get()))"),
            "Bytes" => format!("fromJniBytes(env, static_cast<jbyteArray>(field{index}.get()))"),
            _ if value.unsigned => format!("std::bit_cast<{}>(field{index})", value.cpp),
            _ => format!("static_cast<{}>(field{index})", value.cpp),
        };
    }
    format!(
        "nexaFromJni{}(env, field{index}.get())",
        cpp_identifier(bridge_type_name(ty))
    )
}

fn android_cpp_to_jni_field(ty: &BridgeType, expression: &str) -> String {
    if let Some(value) = android_cpp_value(ty) {
        return match bridge_type_name(ty) {
            "String" => format!("toJniString(env, {expression})"),
            "Bytes" => format!("toJniBytes(env, {expression})"),
            _ if value.unsigned => format!("std::bit_cast<{}>({expression})", value.jni),
            _ => format!("static_cast<{}>({expression})", value.jni),
        };
    }
    format!(
        "nexaToJni{}(env, {expression})",
        cpp_identifier(bridge_type_name(ty))
    )
}

fn android_jni_call_method(jni_type: &str) -> &'static str {
    match jni_type {
        "jboolean" => "CallBooleanMethod",
        "jbyte" => "CallByteMethod",
        "jshort" => "CallShortMethod",
        "jint" => "CallIntMethod",
        "jlong" => "CallLongMethod",
        "jfloat" => "CallFloatMethod",
        "jdouble" => "CallDoubleMethod",
        _ => "CallObjectMethod",
    }
}

fn android_jni_type_is_reference(jni_type: &str) -> bool {
    matches!(jni_type, "jobject" | "jstring" | "jbyteArray")
}

fn android_jvalue_field(jni_type: &str) -> &'static str {
    match jni_type {
        "jboolean" => "z",
        "jbyte" => "b",
        "jshort" => "s",
        "jint" => "i",
        "jlong" => "j",
        "jfloat" => "f",
        "jdouble" => "d",
        _ => "l",
    }
}

fn android_kotlin_getter_name(field_name: &str) -> String {
    let mut characters = field_name.chars();
    let is_property_getter =
        field_name.starts_with("is") && field_name.chars().nth(2).is_some_and(char::is_uppercase);
    match characters.next() {
        Some(_) if is_property_getter => field_name.to_owned(),
        Some(first) => format!("get{}{}", first.to_uppercase(), characters.as_str()),
        None => "get".to_owned(),
    }
}

pub(crate) fn android_jni_signature(ty: &BridgeType) -> String {
    match android_jni_type(ty).as_str() {
        "jboolean" => "Z".to_owned(),
        "jbyte" => "B".to_owned(),
        "jshort" => "S".to_owned(),
        "jint" => "I".to_owned(),
        "jlong" => "J".to_owned(),
        "jfloat" => "F".to_owned(),
        "jdouble" => "D".to_owned(),
        "jstring" => "Ljava/lang/String;".to_owned(),
        "jbyteArray" => "[B".to_owned(),
        // Error payloads are validated to primitive, `String`, and `Bytes`
        // shapes, all of which map above. Anything else cannot reach this
        // signature.
        _ => {
            debug_assert!(
                false,
                "validated Android typed error payload should have a JNI type"
            );
            "Ljava/lang/Object;".to_owned()
        }
    }
}

fn render_jni_argument_conversions(
    out: &mut SourceWriter,
    parameters: &[BridgeParameter],
    failure_return: &str,
) -> Vec<String> {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            if matches!(
                bridge_strip_optional(&parameter.ty),
                BridgeType::Map(..)
            ) {
                let mut local = format!("nexaJniMapArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                return render_jni_map_argument_conversion(
                    out,
                    &parameter.ty,
                    &parameter.name,
                    failure_return,
                    &local,
                );
            }
            if matches!(
                bridge_strip_optional(&parameter.ty),
                BridgeType::Array(_) | BridgeType::Set(_)
            ) {
                let mut local = format!("nexaJniArrayArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                return render_jni_array_argument_conversion(
                    out,
                    &parameter.ty,
                    &parameter.name,
                    failure_return,
                    &local,
                );
            }
            let Some(scalar) = android_cpp_value(&parameter.ty) else {
                let name = cpp_identifier(bridge_type_name(&parameter.ty));
                let local = format!("nexaJniNamedArgument{index}");
                out.push_str(&format!(
                    "        auto {local} = nexaFromJni{name}(env, {});\n        if (env->ExceptionCheck()) {failure_return}\n",
                    parameter.name
                ));
                return format!("std::move({local})");
            };
            if scalar.optional {
                render_jni_optional_argument_conversion(
                    out,
                    &parameter.ty,
                    &parameter.name,
                    failure_return,
                    &format!("nexaJniOptionalArgument{index}"),
                )
            } else if scalar.jni == "jstring" {
                let mut local = format!("nexaJniStringArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                out.push_str(&format!(
                    "        auto {local} = fromJniString(env, {});\n        if (env->ExceptionCheck()) {failure_return}\n",
                    parameter.name
                ));
                format!("std::move({local})")
            } else if scalar.jni == "jbyteArray" {
                let mut local = format!("nexaJniBytesArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                out.push_str(&format!(
                    "        auto {local} = fromJniBytes(env, {});\n        if (env->ExceptionCheck()) {failure_return}\n",
                    parameter.name
                ));
                format!("std::move({local})")
            } else {
                format!("static_cast<{}>({})", scalar.cpp, parameter.name)
            }
        })
        .collect()
}

fn render_jni_array_argument_conversion(
    out: &mut SourceWriter,
    ty: &BridgeType,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let element = match ty {
        BridgeType::Array(element) | BridgeType::Set(element) => element,
        _ => {
            debug_assert!(
                false,
                "array argument conversion requires a validated array or set type"
            );
            return String::new();
        }
    };
    let is_set = matches!(ty, BridgeType::Set(_));
    if (!is_set
        && matches!(
            bridge_strip_optional(element),
            BridgeType::Array(_) | BridgeType::Set(_) | BridgeType::Map(..)
        ))
        || element.is_optional()
    {
        return render_jni_nested_array_argument_conversion(out, ty, value, failure_return, local);
    }
    render_jni_array_argument_conversion_flat(out, ty, value, failure_return, local)
}

fn render_jni_nested_array_argument_conversion(
    out: &mut SourceWriter,
    ty: &BridgeType,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    // Optional collections are rejected by plan validation; the dispatch
    // above only routes non-optional arrays and sets here.
    let element = match ty {
        BridgeType::Array(element) | BridgeType::Set(element) => element,
        _ => {
            debug_assert!(
                false,
                "nested array conversion requires a validated array or set type"
            );
            return String::new();
        }
    };
    let cpp_type = android_cpp_type(ty);
    let is_set = matches!(ty, BridgeType::Set(_));
    let reserve = if is_set {
        String::new()
    } else {
        format!("{local}.reserve(static_cast<std::size_t>({local}Length));\n        ")
    };
    let null_element_check = if element.is_optional() {
        String::new()
    } else {
        format!(
            "            if ({local}Element == nullptr) {{\n                throwIllegalState(env, \"non-null nested Nexa collection contained null\");\n                {failure_return}\n            }}\n"
        )
    };
    out.push_str(&format!(
        "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa collection was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<jobjectArray>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        {cpp_type} {local};\n        {reserve}for (jsize {local}Index = 0; {local}Index < {local}Length; ++{local}Index) {{\n            auto {local}Element = env->GetObjectArrayElement(static_cast<jobjectArray>({value}), {local}Index);\n            if (env->ExceptionCheck()) {failure_return}\n{null_element_check}"
    ));
    let element_local = format!("{local}ElementValue");
    let element_value = if matches!(bridge_strip_optional(element), BridgeType::Map(..)) {
        render_jni_map_argument_conversion(
            out,
            element,
            &format!("{local}Element"),
            failure_return,
            &element_local,
        )
    } else if matches!(
        bridge_strip_optional(element),
        BridgeType::Array(_) | BridgeType::Set(_)
    ) {
        render_jni_array_argument_conversion(
            out,
            element,
            &format!("{local}Element"),
            failure_return,
            &element_local,
        )
    } else {
        render_jni_optional_argument_conversion(
            out,
            element,
            &format!("{local}Element"),
            failure_return,
            &element_local,
        )
    };
    let append = if is_set {
        format!("{local}.insert(std::move({element_value}));")
    } else {
        format!("{local}.push_back(std::move({element_value}));")
    };
    out.push_str(&format!(
        "            {append}\n            if ({local}Element != nullptr) env->DeleteLocalRef({local}Element);\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n"
    ));
    format!("std::move({local})")
}

fn render_jni_array_argument_conversion_flat(
    out: &mut SourceWriter,
    ty: &BridgeType,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let element = match ty {
        BridgeType::Array(element) | BridgeType::Set(element) => element,
        _ => {
            debug_assert!(
                false,
                "flat array conversion requires a validated array or set type"
            );
            return String::new();
        }
    };
    let is_set = matches!(ty, BridgeType::Set(_));
    if android_primitive_array(element).is_none() {
        let Some(scalar) = android_reference_array_element(element) else {
            debug_assert!(
                false,
                "validated reference array elements map to an Android value"
            );
            return String::new();
        };
        let (jni_type, conversion) =
            if matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::String)) {
                ("jstring", "fromJniString")
            } else {
                ("jbyteArray", "fromJniBytes")
            };
        if is_set {
            out.push_str(&format!(
                "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa set was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<jobjectArray>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::set<{}> {local};\n        for (jsize index = 0; index < {local}Length; ++index) {{\n            auto {local}Element = static_cast<{jni_type}>(env->GetObjectArrayElement(static_cast<jobjectArray>({value}), index));\n            if (env->ExceptionCheck()) {failure_return}\n            auto {local}Value = {conversion}(env, {local}Element);\n            if ({local}Element != nullptr) env->DeleteLocalRef({local}Element);\n            if (env->ExceptionCheck()) {failure_return}\n            {local}.insert(std::move({local}Value));\n        }}\n",
                scalar.cpp,
            ));
        } else {
            out.push_str(&format!(
                "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa array was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<jobjectArray>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::vector<{}> {local};\n        {local}.reserve(static_cast<std::size_t>({local}Length));\n        for (jsize index = 0; index < {local}Length; ++index) {{\n            auto {local}Element = static_cast<{jni_type}>(env->GetObjectArrayElement(static_cast<jobjectArray>({value}), index));\n            if (env->ExceptionCheck()) {failure_return}\n            auto {local}Value = {conversion}(env, {local}Element);\n            if ({local}Element != nullptr) env->DeleteLocalRef({local}Element);\n            if (env->ExceptionCheck()) {failure_return}\n            {local}.push_back(std::move({local}Value));\n        }}\n",
                scalar.cpp,
            ));
        }
        return format!("std::move({local})");
    }
    let array = android_primitive_array(element).expect("validated primitive array");
    let cpp_type = android_cpp_value(element)
        .expect("validated array element")
        .cpp;
    if is_set {
        out.push_str(&format!(
            "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa set was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<{}>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::set<{cpp_type}> {local};\n",
            array.jni_array,
        ));
        out.push_str(&format!(
            "        if ({local}Length > 0) {{\n            std::vector<{}> {local}Carrier({local}Length);\n            env->{}(static_cast<{}>({value}), 0, {local}Length, {local}Carrier.data());\n            if (env->ExceptionCheck()) {failure_return}\n            for (jsize index = 0; index < {local}Length; ++index) {local}.insert(static_cast<{cpp_type}>({local}Carrier[index]));\n        }}\n",
            array.element_jni,
            array.get_region,
            array.jni_array,
        ));
    } else {
        out.push_str(&format!(
            "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa array was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<{}>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::vector<{cpp_type}> {local}({local}Length);\n",
            array.jni_array,
        ));
        out.push_str(&format!(
            "        if ({local}Length > 0) {{\n            std::vector<{}> {local}Carrier({local}Length);\n            env->{}(static_cast<{}>({value}), 0, {local}Length, {local}Carrier.data());\n            if (env->ExceptionCheck()) {failure_return}\n            for (jsize index = 0; index < {local}Length; ++index) {local}[index] = static_cast<{cpp_type}>({local}Carrier[index]);\n        }}\n",
            array.element_jni,
            array.get_region,
            array.jni_array,
        ));
    }
    format!("std::move({local})")
}

fn render_android_jni_map_converter(
    out: &mut SourceWriter,
    ty: &BridgeType,
    name: &str,
    to_java: bool,
    failure_return: &str,
) -> String {
    if let BridgeType::Map(key, value) = bridge_strip_optional(ty) {
        let cpp_type = android_cpp_type(ty);
        let key_converter = render_android_jni_map_converter(
            out,
            key,
            &format!("{name}Key"),
            to_java,
            failure_return,
        );
        let value_converter = render_android_jni_map_converter(
            out,
            value,
            &format!("{name}Value"),
            to_java,
            failure_return,
        );
        return match (to_java, ty.is_optional()) {
            (true, true) => format!(
                "[&](const {cpp_type}& value) -> jobject {{ if (!value.has_value()) return nullptr; return toJniMap(env, *value, {key_converter}, {value_converter}); }}"
            ),
            (false, true) => {
                let map_type = format!(
                    "std::map<{}, {}>",
                    android_cpp_type(key),
                    android_cpp_type(value)
                );
                format!(
                    "[&](jobject raw) -> {cpp_type} {{ if (raw == nullptr) return std::nullopt; return fromJniMap<{map_type}>(env, raw, {key_converter}, {value_converter}); }}"
                )
            }
            (true, false) => format!(
                "[&](const {cpp_type}& value) -> jobject {{ return toJniMap(env, value, {key_converter}, {value_converter}); }}"
            ),
            (false, false) => format!(
                "[&](jobject raw) -> {cpp_type} {{ return fromJniMap<{cpp_type}>(env, raw, {key_converter}, {value_converter}); }}"
            ),
        };
    }
    if let BridgeType::Array(element) | BridgeType::Set(element) = ty {
        let is_set = matches!(ty, BridgeType::Set(_));
        let collection_type = android_cpp_type(ty);
        if matches!(
            bridge_strip_optional(element),
            BridgeType::Array(_) | BridgeType::Set(_) | BridgeType::Map(..)
        ) || element.is_optional()
        {
            let Some(element_class) = android_jni_class_descriptor(element) else {
                debug_assert!(
                    false,
                    "validated nested Android collections have JNI descriptors"
                );
                return String::new();
            };
            let converter = render_android_jni_map_converter(
                out,
                element,
                &format!("{name}Element"),
                to_java,
                failure_return,
            );
            let set_reserve = !is_set;
            let reserve = if set_reserve {
                "            output.reserve(static_cast<std::size_t>(length));\n"
            } else {
                ""
            };
            let null_check = if element.is_optional() {
                ""
            } else {
                "                if (element.get() == nullptr) { throwIllegalState(env, \"non-null Nexa nested collection contained null\"); throw std::runtime_error(\"null JNI nested collection element\"); }\n"
            };
            let append = if is_set {
                "output.insert(std::move(value));"
            } else {
                "output.push_back(std::move(value));"
            };
            return if to_java {
                format!(
                    "[&](const {collection_type}& values) -> jobject {{\n            if (values.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin collection is too large for JNI\");\n            const auto length = static_cast<jsize>(values.size());\n            ScopedLocalRef<jclass> elementClass(env, env->FindClass(\"{element_class}\"));\n            requireJniMapOperation(env, \"unable to resolve JNI nested collection element type\");\n            ScopedLocalRef<jobjectArray> output(env, env->NewObjectArray(length, elementClass.get(), nullptr));\n            requireJniMapOperation(env, \"unable to allocate JNI nested collection\");\n            jsize index = 0;\n            for (const auto& value : values) {{\n                ScopedLocalRef<jobject> element(env, {converter}(value));\n                requireJniMapOperation(env, \"unable to convert native nested collection element\");\n                env->SetObjectArrayElement(output.get(), index++, element.get());\n                requireJniMapOperation(env, \"unable to populate JNI nested collection\");\n            }}\n            return output.release();\n        }}"
                )
            } else {
                format!(
                    "[&](jobject raw) -> {collection_type} {{\n            if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa nested collection was null at the JNI boundary\"); throw std::runtime_error(\"null JNI nested collection\"); }}\n            auto input = static_cast<jobjectArray>(raw);\n            const jsize length = env->GetArrayLength(input);\n            requireJniMapOperation(env, \"unable to read JNI nested collection length\");\n            {collection_type} output;\n{reserve}            for (jsize index = 0; index < length; ++index) {{\n                ScopedLocalRef<jobject> element(env, env->GetObjectArrayElement(input, index));\n                requireJniMapOperation(env, \"unable to read JNI nested collection element\");\n{null_check}                auto value = {converter}(element.get());\n                requireJniMapOperation(env, \"unable to convert JNI nested collection element\");\n                {append}\n            }}\n            return output;\n        }}"
                )
            };
        }
        let Some(array) = android_primitive_array(element) else {
            let Some(_) = android_reference_array_element(element) else {
                debug_assert!(
                    false,
                    "validated Android map collection has a reference element"
                );
                return String::new();
            };
            let element_class =
                if matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::String)) {
                    "java/lang/String"
                } else {
                    "[B"
                };
            if to_java {
                let to_jni = if matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::String))
                {
                    "toJniString"
                } else {
                    "toJniBytes"
                };
                return format!(
                    "[&](const {collection_type}& values) -> jobject {{\n            if (values.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin collection is too large for JNI\");\n            const auto length = static_cast<jsize>(values.size());\n            ScopedLocalRef<jclass> elementClass(env, env->FindClass(\"{element_class}\"));\n            requireJniMapOperation(env, \"unable to resolve JNI map collection element type\");\n            ScopedLocalRef<jobjectArray> output(env, env->NewObjectArray(length, elementClass.get(), nullptr));\n            requireJniMapOperation(env, \"unable to allocate JNI map collection\");\n            jsize index = 0;\n            for (const auto& value : values) {{\n                ScopedLocalRef<jobject> element(env, {to_jni}(env, value));\n                requireJniMapOperation(env, \"unable to convert native map collection element\");\n                env->SetObjectArrayElement(output.get(), index++, element.get());\n                requireJniMapOperation(env, \"unable to populate JNI map collection\");\n            }}\n            return output.release();\n        }}"
                );
            }
            let from_jni = if matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::String)) {
                "fromJniString"
            } else {
                "fromJniBytes"
            };
            let reference_type =
                if matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::String)) {
                    "jstring"
                } else {
                    "jbyteArray"
                };
            let append = if matches!(ty, BridgeType::Set(_)) {
                "output.insert(std::move(value));"
            } else {
                "output.push_back(std::move(value));"
            };
            return format!(
                "[&](jobject raw) -> {collection_type} {{\n            if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa map collection was null at the JNI boundary\"); throw std::runtime_error(\"null JNI map collection\"); }}\n            auto input = static_cast<jobjectArray>(raw);\n            const jsize length = env->GetArrayLength(input);\n            requireJniMapOperation(env, \"unable to read JNI map collection length\");\n            {collection_type} output;\n            for (jsize index = 0; index < length; ++index) {{\n                ScopedLocalRef<jobject> element(env, env->GetObjectArrayElement(input, index));\n                requireJniMapOperation(env, \"unable to read JNI map collection element\");\n                if (element.get() == nullptr) {{ throwIllegalState(env, \"non-null Nexa map collection contained null\"); throw std::runtime_error(\"null JNI map collection element\"); }}\n                auto value = {from_jni}(env, static_cast<{reference_type}>(element.get()));\n                requireJniMapOperation(env, \"unable to convert JNI map collection element\");\n                {append}\n            }}\n            return output;\n        }}"
            );
        };
        let Some(scalar) = android_cpp_value(element) else {
            debug_assert!(
                false,
                "validated map collection elements map to an Android value"
            );
            return String::new();
        };
        if to_java {
            let carrier = if scalar.unsigned {
                format!(
                    "std::bit_cast<{}>(static_cast<{}>(element))",
                    array.element_jni, scalar.cpp
                )
            } else {
                format!("static_cast<{}>(element)", array.element_jni)
            };
            return format!(
                "[&](const {collection_type}& values) -> jobject {{\n            if (values.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin collection is too large for JNI\");\n            const auto length = static_cast<jsize>(values.size());\n            auto output = env->New{}Array(length);\n            if (output == nullptr) return nullptr;\n            std::vector<{}> carrier;\n            carrier.reserve(values.size());\n            for (const auto& element : values) carrier.push_back({carrier});\n            if (length > 0) env->{}(output, 0, length, carrier.data());\n            if (env->ExceptionCheck()) return nullptr;\n            return output;\n        }}",
                array.kotlin_array.trim_end_matches("Array"),
                array.element_jni,
                array.set_region,
            );
        }

        let converted = if scalar.unsigned {
            format!("std::bit_cast<{}>(carrier[index])", scalar.cpp)
        } else {
            format!("static_cast<{}>(carrier[index])", scalar.cpp)
        };
        let insertion = if matches!(ty, BridgeType::Set(_)) {
            format!("output.insert({converted});")
        } else {
            format!("output.push_back({converted});")
        };
        return format!(
            "[&](jobject raw) -> {collection_type} {{\n            if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa map collection was null at the JNI boundary\"); throw std::runtime_error(\"null JNI map collection\"); }}\n            auto input = static_cast<{}>(raw);\n            const jsize length = env->GetArrayLength(input);\n            requireJniMapOperation(env, \"unable to read JNI map collection length\");\n            std::vector<{}> carrier(static_cast<std::size_t>(length));\n            if (length > 0) env->{}(input, 0, length, carrier.data());\n            requireJniMapOperation(env, \"unable to read JNI map collection values\");\n            {collection_type} output;\n            for (jsize index = 0; index < length; ++index) {{ {insertion} }}\n            return output;\n        }}",
            array.jni_array, array.element_jni, array.get_region,
        );
    }
    let Some(scalar) = android_cpp_value(ty) else {
        debug_assert!(false, "validated Android map scalar should map");
        return String::new();
    };
    if ty.is_optional() {
        let cpp_type = android_cpp_type(ty);
        if matches!(
            bridge_strip_optional(ty),
            BridgeType::Scalar(BridgeScalar::String | BridgeScalar::Bytes)
        ) {
            let (class, converter) = if matches!(
                bridge_strip_optional(ty),
                BridgeType::Scalar(BridgeScalar::String)
            ) {
                ("jstring", "String")
            } else {
                ("jbyteArray", "Bytes")
            };
            return if to_java {
                format!(
                    "[&](const {cpp_type}& value) -> jobject {{ if (!value) return nullptr; return toJni{converter}(env, *value); }}"
                )
            } else {
                format!(
                    "[&](jobject raw) -> {cpp_type} {{ if (raw == nullptr) return std::nullopt; return fromJni{converter}(env, static_cast<{class}>(raw)); }}"
                )
            };
        }
        let Some(boxed) = android_boxed_primitive(bridge_type_name(ty)) else {
            debug_assert!(
                false,
                "validated nullable map primitives have a boxed JNI type"
            );
            return String::new();
        };
        let class_name = format!("{name}Class");
        let method_name = format!("{name}BridgeMethod");
        let (lookup, lambda) = if to_java {
            let carrier = if scalar.unsigned
                && matches!(
                    bridge_strip_optional(ty),
                    BridgeType::Scalar(BridgeScalar::UInt64)
                ) {
                "std::bit_cast<jlong>(static_cast<std::uint64_t>(*value))".to_owned()
            } else if scalar.unsigned {
                "static_cast<jlong>(*value)".to_owned()
            } else {
                format!("static_cast<{}>(*value)", boxed.jni_primitive)
            };
            (
                format!(
                    "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetStaticMethodID({class_name}, \"valueOf\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                    boxed.class_name, boxed.value_of_signature
                ),
                format!(
                    "[&](const {cpp_type}& value) -> jobject {{ if (!value) return nullptr; return env->CallStaticObjectMethod({class_name}, {method_name}, {carrier}); }}"
                ),
            )
        } else {
            (
                format!(
                    "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetMethodID({class_name}, \"{}\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                    boxed.class_name, boxed.unbox_method, boxed.unbox_signature
                ),
                format!(
                    "[&](jobject raw) -> {cpp_type} {{ if (raw == nullptr) return std::nullopt; auto carrier = env->{call}(raw, {method_name}); return static_cast<{cpp}>(carrier); }}",
                    call = boxed.unbox_call,
                    cpp = scalar.cpp
                ),
            )
        };
        out.push_str(&lookup);
        return lambda;
    }
    if matches!(
        bridge_strip_optional(ty),
        BridgeType::Scalar(BridgeScalar::String)
    ) {
        return if to_java {
            "[&](const std::string& value) -> jobject { return toJniString(env, value); }"
                .to_owned()
        } else {
            "[&](jobject raw) -> std::string { return fromJniString(env, static_cast<jstring>(raw)); }".to_owned()
        };
    }
    if matches!(
        bridge_strip_optional(ty),
        BridgeType::Scalar(BridgeScalar::Bytes)
    ) {
        return if to_java {
            "[&](const std::vector<std::uint8_t>& value) -> jobject { return toJniBytes(env, value); }"
                .to_owned()
        } else {
            "[&](jobject raw) -> std::vector<std::uint8_t> { return fromJniBytes(env, static_cast<jbyteArray>(raw)); }"
                .to_owned()
        };
    }
    let boxed = android_jni_map_boxed_primitive(ty).expect("validated boxed map scalar");
    let class_name = format!("{name}Class");
    let method_name = format!("{name}BridgeMethod");
    let (lookup, lambda) = if to_java {
        let carrier = if scalar.unsigned {
            format!(
                "std::bit_cast<{}>(static_cast<{}>(value))",
                boxed.jni_primitive, scalar.cpp
            )
        } else {
            format!("static_cast<{}>(value)", boxed.jni_primitive)
        };
        (
            format!(
                "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetStaticMethodID({class_name}, \"valueOf\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                boxed.class_name, boxed.value_of_signature
            ),
            format!(
                "[&](const {cpp}& value) -> jobject {{ return env->CallStaticObjectMethod({class_name}, {method_name}, {carrier}); }}",
                cpp = scalar.cpp
            ),
        )
    } else {
        (
            format!(
                "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetMethodID({class_name}, \"{}\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                boxed.class_name, boxed.unbox_method, boxed.unbox_signature
            ),
            format!(
                "[&](jobject raw) -> {cpp} {{ auto carrier = env->{call}(raw, {method_name}); return static_cast<{cpp}>(carrier); }}",
                cpp = scalar.cpp,
                call = boxed.unbox_call
            ),
        )
    };
    out.push_str(&lookup);
    lambda
}

fn render_jni_map_argument_conversion(
    out: &mut SourceWriter,
    ty: &BridgeType,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let BridgeType::Map(key, map_value) = bridge_strip_optional(ty) else {
        debug_assert!(
            false,
            "map argument conversion requires a validated map type"
        );
        return String::new();
    };
    let map_cpp_type = format!(
        "std::map<{}, {}>",
        android_cpp_type(key),
        android_cpp_type(map_value)
    );
    let key_converter =
        render_android_jni_map_converter(out, key, &format!("{local}Key"), false, failure_return);
    let value_converter = render_android_jni_map_converter(
        out,
        map_value,
        &format!("{local}Value"),
        false,
        failure_return,
    );
    if ty.is_optional() {
        out.push_str(&format!(
            "        std::optional<{map_cpp_type}> {local};\n        if ({value} != nullptr) {{\n            {local}.emplace(fromJniMap<{map_cpp_type}>(env, {value}, {key_converter}, {value_converter}));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n",
        ));
    } else {
        out.push_str(&format!(
            "        auto {local} = fromJniMap<{map_cpp_type}>(env, {value}, {key_converter}, {value_converter});\n        if (env->ExceptionCheck()) {failure_return}\n",
        ));
    }
    format!("std::move({local})")
}

fn render_jni_optional_argument_conversion(
    out: &mut SourceWriter,
    ty: &BridgeType,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let Some(scalar) = android_cpp_value(ty) else {
        debug_assert!(
            false,
            "validated optional JNI values map to an Android value"
        );
        return String::new();
    };
    out.push_str(&format!("        std::optional<{}> {local};\n", scalar.cpp));
    match bridge_strip_optional(ty) {
        BridgeType::Scalar(BridgeScalar::String) => out.push_str(&format!(
            "        if ({value} != nullptr) {{\n            {local} = fromJniString(env, static_cast<jstring>({value}));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n"
        )),
        BridgeType::Scalar(BridgeScalar::Bytes) => out.push_str(&format!(
            "        if ({value} != nullptr) {{\n            {local} = fromJniBytes(env, static_cast<jbyteArray>({value}));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n"
        )),
        _ => {
            let Some(boxed) = android_boxed_primitive(bridge_type_name(ty)) else {
                debug_assert!(
                    false,
                    "validated nullable map primitives have a boxed JNI type"
                );
                return String::new();
            };
            out.push_str(&format!(
                "        if ({value} != nullptr) {{\n            jclass {local}Class = env->FindClass(\"{}\");\n            if ({local}Class == nullptr) {failure_return}\n            jmethodID {local}Unbox = env->GetMethodID({local}Class, \"{}\", \"{}\");\n            if ({local}Unbox == nullptr) {failure_return}\n            {local} = static_cast<{}>(env->{}({value}, {local}Unbox));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n",
                boxed.class_name,
                boxed.unbox_method,
                boxed.unbox_signature,
                scalar.cpp,
                boxed.unbox_call,
            ));
        }
    }
    format!("std::move({local})")
}

pub(crate) fn render_jni_return(out: &mut SourceWriter, ty: &BridgeType, expression: &str) {
    // Optional wrappers are transparent to shape dispatch, mirroring how
    // the historical renderer matched on the inner type name.
    let shape = bridge_strip_optional(ty);
    let optional = ty.is_optional();
    if let BridgeType::Map(key, map_value) = shape {
        out.push_str("        return [&]() -> jobject {\n");
        let key_converter = render_android_jni_map_converter(
            out,
            key,
            "nexaJniMapReturnKey",
            true,
            "return nullptr;",
        );
        let value_converter = render_android_jni_map_converter(
            out,
            map_value,
            "nexaJniMapReturnValue",
            true,
            "return nullptr;",
        );
        if optional {
            out.push_str(&format!(
                "            auto nexaJniOptionalMapReturn = {expression};\n            if (!nexaJniOptionalMapReturn.has_value()) return nullptr;\n            return toJniMap(env, *nexaJniOptionalMapReturn, {key_converter}, {value_converter});\n        }}();\n",
            ));
        } else {
            out.push_str(&format!(
                "            return toJniMap(env, {expression}, {key_converter}, {value_converter});\n        }}();\n",
            ));
        }
        return;
    }
    if let BridgeType::Array(element) | BridgeType::Set(element) = shape {
        let is_set = matches!(ty, BridgeType::Set(_));
        if (!is_set
            && matches!(
                bridge_strip_optional(element),
                BridgeType::Array(_) | BridgeType::Set(_) | BridgeType::Map(..)
            ))
            || element.is_optional()
        {
            render_jni_nested_array_return(out, ty, expression);
            return;
        }
        let cpp_element = android_cpp_type(element);
        if matches!(shape, BridgeType::Set(_)) {
            out.push_str(&format!(
                "        auto nexaArraySetValues = {expression};\n        std::vector<{cpp_element}> nexaArrayValues(nexaArraySetValues.begin(), nexaArraySetValues.end());\n"
            ));
        } else {
            out.push_str(&format!("        auto nexaArrayValues = {expression};\n"));
        }
        if android_primitive_array(element).is_none() {
            let Some(_) = android_reference_array_element(element) else {
                debug_assert!(
                    false,
                    "validated reference array elements map to an Android value"
                );
                return;
            };
            let (element_class, conversion) =
                if matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::String)) {
                    ("java/lang/String", "toJniString")
                } else {
                    ("[B", "toJniBytes")
                };
            out.push_str(&format!(
                "        if (nexaArrayValues.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin array is too large for JNI\");\n        const auto nexaArrayLength = static_cast<jsize>(nexaArrayValues.size());\n        jclass nexaArrayElementClass = env->FindClass(\"{element_class}\");\n        if (nexaArrayElementClass == nullptr) return nullptr;\n        auto nexaArrayOutput = env->NewObjectArray(nexaArrayLength, nexaArrayElementClass, nullptr);\n        if (nexaArrayOutput == nullptr) return nullptr;\n        for (jsize index = 0; index < nexaArrayLength; ++index) {{\n            auto nexaArrayElement = {conversion}(env, nexaArrayValues[index]);\n            if (env->ExceptionCheck()) return nullptr;\n            env->SetObjectArrayElement(nexaArrayOutput, index, nexaArrayElement);\n            if (nexaArrayElement != nullptr) env->DeleteLocalRef(nexaArrayElement);\n            if (env->ExceptionCheck()) return nullptr;\n        }}\n        env->DeleteLocalRef(nexaArrayElementClass);\n        return nexaArrayOutput;\n"
            ));
            return;
        }
        let Some(array) = android_primitive_array(element) else {
            debug_assert!(
                false,
                "validated primitive array elements have backing stores"
            );
            return;
        };
        out.push_str(&format!(
            "        if (nexaArrayValues.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin array is too large for JNI\");\n        const auto nexaArrayLength = static_cast<jsize>(nexaArrayValues.size());\n        auto nexaArrayOutput = env->New{}Array(nexaArrayLength);\n        if (nexaArrayOutput == nullptr) return nullptr;\n",
            array.kotlin_array.trim_end_matches("Array"),
        ));
        out.push_str(&format!(
            "        if (nexaArrayLength > 0) {{\n            std::vector<{}> nexaArrayCarrier(nexaArrayValues.size());\n            for (jsize index = 0; index < nexaArrayLength; ++index) nexaArrayCarrier[index] = static_cast<{}>(nexaArrayValues[index]);\n            env->{}(nexaArrayOutput, 0, nexaArrayLength, nexaArrayCarrier.data());\n            if (env->ExceptionCheck()) return nullptr;\n        }}\n        return nexaArrayOutput;\n",
            array.element_jni,
            array.element_jni,
            array.set_region,
        ));
        return;
    }
    let Some(scalar) = android_cpp_value(ty) else {
        let name = cpp_identifier(bridge_type_name(ty));
        out.push_str(&format!(
            "        return nexaToJni{name}(env, {expression});\n"
        ));
        return;
    };
    if scalar.optional {
        out.push_str("        return [&]() -> jobject {\n");
        out.push_str(&format!(
            "            auto nexaOptionalValue = {expression};\n"
        ));
        out.push_str("            if (!nexaOptionalValue) return nullptr;\n");
        match bridge_type_name(ty) {
            "String" => out.push_str("            return toJniString(env, *nexaOptionalValue);\n"),
            "Bytes" => out.push_str("            return toJniBytes(env, *nexaOptionalValue);\n"),
            _ => {
                let Some(boxed) = android_boxed_primitive(bridge_type_name(ty)) else {
                    debug_assert!(
                        false,
                        "validated optional boxed primitive has a boxed JNI type"
                    );
                    return;
                };
                let value_expression = if scalar.unsigned && bridge_type_name(ty) == "UInt64" {
                    "std::bit_cast<jlong>(static_cast<std::uint64_t>(*nexaOptionalValue))"
                        .to_owned()
                } else if scalar.unsigned {
                    "static_cast<jlong>(*nexaOptionalValue)".to_owned()
                } else {
                    format!("static_cast<{}>(*nexaOptionalValue)", boxed.jni_primitive)
                };
                out.push_str(&format!(
                    "            jclass nexaBoxClass = env->FindClass(\"{}\");\n            if (nexaBoxClass == nullptr) return nullptr;\n            jmethodID nexaValueOf = env->GetStaticMethodID(nexaBoxClass, \"valueOf\", \"{}\");\n            if (nexaValueOf == nullptr) return nullptr;\n            return env->CallStaticObjectMethod(nexaBoxClass, nexaValueOf, {value_expression});\n",
                    boxed.class_name,
                    boxed.value_of_signature,
                ));
            }
        }
        out.push_str("        }();\n");
    } else if scalar.jni == "jstring" {
        out.push_str(&format!("        return toJniString(env, {expression});\n"));
    } else if scalar.jni == "jbyteArray" {
        out.push_str(&format!("        return toJniBytes(env, {expression});\n"));
    } else if scalar.unsigned {
        out.push_str(&format!(
            "        return std::bit_cast<{}>(static_cast<{}>({expression}));\n",
            scalar.jni, scalar.cpp
        ));
    } else {
        out.push_str(&format!(
            "        return static_cast<{}>({expression});\n",
            scalar.jni
        ));
    }
}

fn render_jni_nested_array_return(out: &mut SourceWriter, ty: &BridgeType, expression: &str) {
    let element: &BridgeType = match ty {
        BridgeType::Array(element) | BridgeType::Set(element) => element,
        _ => {
            debug_assert!(false, "nested array return requires an array or set type");
            return;
        }
    };
    let Some(element_class) = android_jni_class_descriptor(element) else {
        debug_assert!(
            false,
            "validated nested array element has a JNI array class"
        );
        return;
    };
    if matches!(ty, BridgeType::Set(_)) {
        let element_type = android_cpp_type(element);
        out.push_str(&format!(
            "        auto nexaNestedArraySetValues = {expression};\n        std::vector<{element_type}> nexaNestedArrayValues(nexaNestedArraySetValues.begin(), nexaNestedArraySetValues.end());\n"
        ));
    } else {
        out.push_str(&format!(
            "        auto nexaNestedArrayValues = {expression};\n"
        ));
    }
    out.push_str(&format!(
        "        if (nexaNestedArrayValues.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin array is too large for JNI\");\n        const auto nexaNestedArrayLength = static_cast<jsize>(nexaNestedArrayValues.size());\n        jclass nexaNestedArrayElementClass = env->FindClass(\"{element_class}\");\n        if (nexaNestedArrayElementClass == nullptr) return nullptr;\n        auto nexaNestedArrayOutput = env->NewObjectArray(nexaNestedArrayLength, nexaNestedArrayElementClass, nullptr);\n        env->DeleteLocalRef(nexaNestedArrayElementClass);\n        if (nexaNestedArrayOutput == nullptr) return nullptr;\n        for (jsize nexaNestedArrayIndex = 0; nexaNestedArrayIndex < nexaNestedArrayLength; ++nexaNestedArrayIndex) {{\n            auto nexaNestedArrayElement = [&]() -> jobject {{\n"
    ));
    render_jni_return(
        out,
        element,
        "nexaNestedArrayValues[static_cast<std::size_t>(nexaNestedArrayIndex)]",
    );
    out.push_str(
        "            }();\n            if (env->ExceptionCheck()) return nullptr;\n            env->SetObjectArrayElement(nexaNestedArrayOutput, nexaNestedArrayIndex, nexaNestedArrayElement);\n            if (nexaNestedArrayElement != nullptr) env->DeleteLocalRef(nexaNestedArrayElement);\n            if (env->ExceptionCheck()) return nullptr;\n        }\n        return nexaNestedArrayOutput;\n",
    );
}

fn jni_parameter_declarations(parameters: &[BridgeParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("{} {}", android_jni_type(&parameter.ty), parameter.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn jni_symbol(package: &str, class_name: &str, method_name: &str) -> String {
    let package = package.replace('.', "/");
    format!(
        "Java_{}_{}_{}",
        jni_mangle(&package),
        jni_mangle(class_name),
        jni_mangle(method_name)
    )
}

fn jni_mangle(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '/' => out.push('_'),
            '_' => out.push_str("_1"),
            ';' => out.push_str("_2"),
            '[' => out.push_str("_3"),
            character if character.is_ascii_alphanumeric() => out.push(character),
            character => out.push_str(&format!("_0{:04x}", character as u32)),
        }
    }
    out
}
