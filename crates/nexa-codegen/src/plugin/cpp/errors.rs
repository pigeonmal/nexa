//! Typed errors and the `NexaResult` carrier shared by every C++ target.
//!
//! `throws` clauses and `Result` returns resolve to a single error type per
//! method. This module owns that policy: the result carrier itself, the Swift
//! error bridges and converters, the Android error factory, and the JNI
//! converters that turn an error case into a thrown Kotlin exception.

use super::abi::{
    bridge_strip_optional, cpp_case_type_name, cpp_identifier, cpp_type, unique_cpp_type_name,
};
use super::collections::swift_cpp_result_expression;
use super::jni::values::{android_kotlin_type, android_scalar_value};
use super::jni::{android_jni_signature, render_jni_return};
use super::swift::SwiftAdapterContext;
use crate::plugin::bridge_plan::{BridgeMethod, BridgePlan, BridgeScalar, BridgeType};
use crate::plugin::type_visit::referenced_errors;

pub(crate) fn render_jni_typed_error_result(
    out: &mut String,
    success_type: &BridgeType,
    error_name: &str,
    expression: &str,
    failure_return: &str,
) {
    let error_name = cpp_identifier(error_name);
    out.push_str(&format!(
        "        auto nexaCppTypedResult = {expression};\n        if (!nexaCppTypedResult.has_value()) {{\n            nexaCppThrowError{error_name}(env, nexaCppTypedResult.error());\n            {failure_return}\n        }}\n"
    ));
    if success_type.is_void() {
        return;
    }
    render_jni_return(out, success_type, "nexaCppTypedResult.value()");
}

pub(crate) fn render_android_error_factory(
    out: &mut String,
    plan: &BridgePlan,
    plugin_index: usize,
) {
    let errors = referenced_errors(plan);
    if errors.is_empty() {
        return;
    }
    out.push_str(&format!(
        "internal object NexaPlugin{plugin_index}_CppErrorFactory {{\n"
    ));
    for error in errors {
        for (case_index, case) in error.cases.iter().enumerate() {
            let parameters = case
                .parameters
                .iter()
                .map(|parameter| {
                    format!(
                        "{}: {}",
                        cpp_identifier(&parameter.name),
                        android_kotlin_type(&parameter.ty, true)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let arguments = case
                .parameters
                .iter()
                .map(|parameter| {
                    let name = cpp_identifier(&parameter.name);
                    match bridge_strip_optional(&parameter.ty) {
                        BridgeType::Scalar(BridgeScalar::UInt8) => format!("{name}.toUByte()"),
                        BridgeType::Scalar(BridgeScalar::UInt16) => format!("{name}.toUShort()"),
                        BridgeType::Scalar(BridgeScalar::UInt32) => format!("{name}.toUInt()"),
                        BridgeType::Scalar(BridgeScalar::UInt64) => format!("{name}.toULong()"),
                        _ => name,
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let method = android_cpp_error_factory_method(&error.name, case_index);
            let case_name = cpp_identifier(&case.name);
            let value = if case.parameters.is_empty() {
                format!("{}.{}", error.name, case_name)
            } else {
                format!("{}.{}({arguments})", error.name, case_name)
            };
            out.push_str(&format!(
                "    @JvmStatic public fun {method}({parameters}): {} = {value}\n",
                error.name
            ));
        }
    }
    out.push_str("}\n\n");
}

pub(crate) fn render_android_jni_error_converters(
    out: &mut String,
    plan: &BridgePlan,
    package: &str,
    plugin_index: usize,
) {
    let errors = referenced_errors(plan);
    if errors.is_empty() {
        return;
    }
    let factory_class = format!(
        "{}/NexaPlugin{plugin_index}_CppErrorFactory",
        package.replace('.', "/")
    );
    out.push_str("\nnamespace {\n");
    for error in errors {
        let error_name = cpp_identifier(&error.name);
        let return_signature = format!("L{}/{};", package.replace('.', "/"), error.name);
        out.push_str(&format!(
            "void nexaCppThrowError{error_name}(JNIEnv* env, const {error_name}& error) {{\n    ScopedLocalRef<jclass> factoryClass(env, env->FindClass(\"{factory_class}\"));\n    if (factoryClass.get() == nullptr) return;\n    switch (error.value.index()) {{\n"
        ));
        for (case_index, case) in error.cases.iter().enumerate() {
            let method_name = android_cpp_error_factory_method(&error.name, case_index);
            let parameters_signature = case
                .parameters
                .iter()
                .map(|parameter| android_jni_signature(&parameter.ty))
                .collect::<String>();
            let method_signature = format!("({parameters_signature}){return_signature}");
            out.push_str(&format!(
                "    case {case_index}: {{\n        jmethodID factoryMethod = env->GetStaticMethodID(factoryClass.get(), \"{method_name}\", \"{method_signature}\");\n        if (factoryMethod == nullptr) return;\n        std::array<jvalue, {}> arguments{{}};\n",
                case.parameters.len().max(1)
            ));
            for (parameter_index, parameter) in case.parameters.iter().enumerate() {
                let case_type = cpp_case_type_name(&case.name);
                let value = format!(
                    "std::get<{error_name}::{case_type}>(error.value).{}",
                    cpp_identifier(&parameter.name)
                );
                // Error payloads are validated to non-optional scalars, so
                // this dispatches on the scalar spelling directly.
                match &parameter.ty {
                    BridgeType::Scalar(BridgeScalar::String)
                    | BridgeType::Scalar(BridgeScalar::Bytes) => {
                        let converter =
                            if matches!(parameter.ty, BridgeType::Scalar(BridgeScalar::String)) {
                                "toJniString"
                            } else {
                                "toJniBytes"
                            };
                        out.push_str(&format!(
                            "        ScopedLocalRef<jobject> argument{parameter_index}(env, {converter}(env, {value}));\n        if (env->ExceptionCheck()) return;\n        arguments[{parameter_index}].l = argument{parameter_index}.get();\n"
                        ));
                    }
                    _ => {
                        let BridgeType::Scalar(scalar) = &parameter.ty else {
                            debug_assert!(false, "validated error payloads are scalars");
                            continue;
                        };
                        let carrier = match scalar {
                            BridgeScalar::UInt8 => {
                                format!("std::bit_cast<jbyte>(static_cast<std::uint8_t>({value}))")
                            }
                            BridgeScalar::UInt16 => format!(
                                "std::bit_cast<jshort>(static_cast<std::uint16_t>({value}))"
                            ),
                            BridgeScalar::UInt32 => {
                                format!("std::bit_cast<jint>(static_cast<std::uint32_t>({value}))")
                            }
                            BridgeScalar::UInt64 => {
                                format!("std::bit_cast<jlong>(static_cast<std::uint64_t>({value}))")
                            }
                            _ => format!(
                                "static_cast<{}>({value})",
                                android_scalar_value(*scalar, false).jni
                            ),
                        };
                        let field = match scalar {
                            BridgeScalar::Bool => "z",
                            BridgeScalar::Int8 | BridgeScalar::UInt8 => "b",
                            BridgeScalar::Int16 | BridgeScalar::UInt16 => "s",
                            BridgeScalar::Int32 | BridgeScalar::UInt32 => "i",
                            BridgeScalar::Int64 | BridgeScalar::UInt64 => "j",
                            BridgeScalar::Float32 => "f",
                            BridgeScalar::Float64 => "d",
                            BridgeScalar::String | BridgeScalar::Bytes | BridgeScalar::Void => {
                                debug_assert!(
                                    false,
                                    "string, bytes, and void payloads render above"
                                );
                                continue;
                            }
                        };
                        out.push_str(&format!(
                            "        arguments[{parameter_index}].{field} = {carrier};\n"
                        ));
                    }
                }
            }
            out.push_str(
                "        ScopedLocalRef<jobject> typedError(env, env->CallStaticObjectMethodA(factoryClass.get(), factoryMethod, arguments.data()));\n        if (env->ExceptionCheck()) return;\n        if (typedError.get() == nullptr) { throwIllegalState(env, \"Kotlin typed-error factory returned null\"); return; }\n        env->Throw(static_cast<jthrowable>(typedError.get()));\n        return;\n    }\n",
            );
        }
        out.push_str(&format!(
            "    default: throwIllegalState(env, \"native plugin returned an unknown {error_name} variant\"); return;\n    }}\n}}\n\n"
        ));
    }
    out.push_str("} // namespace\n");
}

fn android_cpp_error_factory_method(error_name: &str, case_index: usize) -> String {
    format!("create{}Case{case_index}", cpp_identifier(error_name))
}

/// The call-site facts a `throws` body needs, resolved once by the caller.
///
/// The method, its success type, the Swift spelling of that type, the error
/// name, and the call expression all come from the same plan, so they travel
/// together instead of as a positional argument list.
pub(crate) struct TypedErrorCall<'a> {
    pub method: &'a BridgeMethod,
    pub success_type: &'a BridgeType,
    pub return_type: &'a str,
    pub error_name: &'a str,
    pub call: &'a str,
    pub call_on_worker: bool,
}

/// Renders the `try`/`await` body for a `throws` method.
pub(crate) fn render_swift_cpp_typed_error_call(
    out: &mut String,
    call_site: &TypedErrorCall<'_>,
    context: &SwiftAdapterContext<'_>,
    depth: usize,
) {
    let TypedErrorCall {
        method,
        success_type,
        return_type,
        error_name,
        call,
        call_on_worker,
    } = *call_site;
    let indent = "    ".repeat(depth);
    let converter = swift_cpp_error_converter_name(error_name);
    if method.is_async {
        let result_type = format!("Result<{return_type}, {error_name}>");
        if !call_on_worker {
            out.push_str(&format!("{indent}    var nexaCppFuture = {call}\n"));
        }
        let work_body = if return_type == "Void" {
            format!(
                "{}var nexaCppTypedResult = nexaCppFuture.get(); if nexaCppTypedResult.has_value() {{ return .success(()) }}; return .failure({converter}(nexaCppTypedResult.nexaSwiftError()))",
                if call_on_worker {
                    format!("var nexaCppFuture = {call}; ")
                } else {
                    String::new()
                }
            )
        } else {
            let converted = swift_cpp_result_expression(
                success_type,
                "nexaCppTypedResult.nexaSwiftValue()",
                context,
            );
            format!(
                "{}var nexaCppTypedResult = nexaCppFuture.get(); if nexaCppTypedResult.has_value() {{ return .success({converted}) }}; return .failure({converter}(nexaCppTypedResult.nexaSwiftError()))",
                if call_on_worker {
                    format!("var nexaCppFuture = {call}; ")
                } else {
                    String::new()
                }
            )
        };
        out.push_str(&format!(
            "{indent}    let nexaCppFutureWork = NexaCppFutureWork<{result_type}> {{ {work_body} }}\n{indent}    let nexaCppTypedResult = await withCheckedContinuation {{ (continuation: CheckedContinuation<{result_type}, Never>) in\n{indent}        DispatchQueue.global(qos: .userInitiated).async {{\n{indent}            continuation.resume(returning: nexaCppFutureWork.run())\n{indent}        }}\n{indent}    }}\n{indent}    return try nexaCppTypedResult.get()\n"
        ));
        return;
    }

    out.push_str(&format!("{indent}    var nexaCppTypedResult = {call}\n"));
    out.push_str(&format!(
        "{indent}    guard nexaCppTypedResult.has_value() else {{ throw {converter}(nexaCppTypedResult.nexaSwiftError()) }}\n"
    ));
    if return_type == "Void" {
        return;
    }
    let converted =
        swift_cpp_result_expression(success_type, "nexaCppTypedResult.nexaSwiftValue()", context);
    out.push_str(&format!("{indent}    return {converted}\n"));
}

pub(crate) fn render_result_type(out: &mut String) {
    out.push_str(
        "template <class Success, class Failure>\nclass NexaResult {\npublic:\n    static NexaResult success(Success value) {\n        return NexaResult(Storage{std::in_place_index<0>, std::move(value)});\n    }\n    static NexaResult failure(Failure error) {\n        return NexaResult(Storage{std::in_place_index<1>, std::move(error)});\n    }\n    bool has_value() const noexcept { return storage_.index() == 0; }\n    Success& value() & { return std::get<0>(storage_); }\n    const Success& value() const & { return std::get<0>(storage_); }\n    Failure& error() & { return std::get<1>(storage_); }\n    const Failure& error() const & { return std::get<1>(storage_); }\n    Success nexaSwiftValue() { return std::move(std::get<0>(storage_)); }\n    Failure nexaSwiftError() { return std::move(std::get<1>(storage_)); }\nprivate:\n    using Storage = std::variant<Success, Failure>;\n    explicit NexaResult(Storage storage) : storage_(std::move(storage)) {}\n    Storage storage_;\n};\n\ntemplate <class Failure>\nclass NexaResult<void, Failure> {\npublic:\n    static NexaResult success() { return NexaResult(std::nullopt); }\n    static NexaResult failure(Failure error) {\n        return NexaResult(std::optional<Failure>(std::move(error)));\n    }\n    bool has_value() const noexcept { return !error_.has_value(); }\n    Failure& error() & { return *error_; }\n    const Failure& error() const & { return *error_; }\n    Failure nexaSwiftError() { return std::move(*error_); }\nprivate:\n    explicit NexaResult(std::optional<Failure> error) : error_(std::move(error)) {}\n    std::optional<Failure> error_;\n};\n\n",
    );
}

pub(crate) fn render_swift_cpp_error_bridges(out: &mut String, plan: &BridgePlan) {
    for error in referenced_errors(plan) {
        let error_name = error.name.as_str();
        let bridge = cpp_swift_error_bridge_name(plan, error_name);
        let name = cpp_identifier(error_name);
        out.push_str(&format!("struct {bridge} {{\n    static std::uint8_t caseIndex(const {name}& error) noexcept {{ return static_cast<std::uint8_t>(error.value.index()); }}\n"));
        for (case_index, case) in error.cases.iter().enumerate() {
            let case_type = cpp_case_type_name(&case.name);
            for (parameter_index, parameter) in case.parameters.iter().enumerate() {
                out.push_str(&format!(
                    "    static {} payload_{case_index}_{parameter_index}(const {name}& error) {{ return std::get<{name}::{case_type}>(error.value).{}; }}\n",
                    cpp_type(&parameter.ty),
                    cpp_identifier(&parameter.name)
                ));
            }
        }
        out.push_str("};\n\n");
    }
}

pub(crate) fn render_swift_cpp_error_converters(
    out: &mut String,
    context: &SwiftAdapterContext<'_>,
) {
    let SwiftAdapterContext {
        plan, namespace, ..
    } = context;
    for error in referenced_errors(plan) {
        let error_name = error.name.as_str();
        let bridge = format!(
            "{namespace}.{}",
            cpp_swift_error_bridge_name(plan, error_name)
        );
        out.push_str(&format!(
            "private func {}(_ error: {namespace}.{error_name}) -> {error_name} {{\n    switch {bridge}.caseIndex(error) {{\n",
            swift_cpp_error_converter_name(error_name)
        ));
        for (case_index, case) in error.cases.iter().enumerate() {
            if case.parameters.is_empty() {
                out.push_str(&format!(
                    "    case {case_index}: return {error_name}.{}\n",
                    case.name
                ));
                continue;
            }
            let payload = case
                .parameters
                .iter()
                .enumerate()
                .map(|(parameter_index, parameter)| {
                    let bridge_call =
                        format!("{bridge}.payload_{case_index}_{parameter_index}(error)");
                    let value = swift_cpp_result_expression(&parameter.ty, &bridge_call, context);
                    format!("{}: {value}", parameter.name)
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "    case {case_index}: return {error_name}.{}({payload})\n",
                case.name
            ));
        }
        out.push_str(
            "    default: preconditionFailure(\"C++ error variant is outside the declared contract\")\n    }\n}\n\n",
        );
    }
}

fn cpp_swift_error_bridge_name(plan: &BridgePlan, error_name: &str) -> String {
    unique_cpp_type_name(
        plan,
        &format!("NexaCppErrorBridge{}", cpp_identifier(error_name)),
    )
}

fn swift_cpp_error_converter_name(error_name: &str) -> String {
    format!("nexaCppErrorTo{}", cpp_identifier(error_name))
}
