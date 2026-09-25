//! How a `BridgeType` is spelled on Android.
//!
//! The JNI boundary needs three different spellings of the same value: the C++
//! type inside the plugin, the JNI carrier that crosses the boundary, and the
//! Kotlin type the app sees. This module is the single table behind all three,
//! plus the array, reference, and boxed-primitive shapes they need.
//!
//! Every function here is total over plan-validated types: validation has
//! already proven each occurrence is mappable, so no mapping returns an error
//! and none can fail on user input.

use super::super::abi::{
    bridge_strip_optional, bridge_type_is_generic, bridge_type_name, cpp_type,
};
use crate::plugin::bridge_plan::{BridgeScalar, BridgeType};

pub(crate) fn android_cpp_value(ty: &BridgeType) -> Option<AndroidValue> {
    // Scalar table over resolved types. Non-scalar shapes have dedicated
    // total spellings elsewhere; validation guarantees this is only called
    // where a scalar (or optional scalar) is legal.
    let (scalar, optional) = match ty {
        BridgeType::Scalar(scalar) => (*scalar, false),
        BridgeType::Optional(inner) => match inner.as_ref() {
            BridgeType::Scalar(scalar) => (*scalar, true),
            _ => return None,
        },
        _ => return None,
    };
    let (kotlin, jni_kotlin, jni, cpp, kotlin_to_jni, kotlin_from_jni, unsigned) = match scalar {
        BridgeScalar::Void => ("Unit", "Unit", "void", "void", None, None, false),
        BridgeScalar::Bool => ("Boolean", "Boolean", "jboolean", "bool", None, None, false),
        BridgeScalar::Int8 => ("Byte", "Byte", "jbyte", "std::int8_t", None, None, false),
        BridgeScalar::Int16 => (
            "Short",
            "Short",
            "jshort",
            "std::int16_t",
            None,
            None,
            false,
        ),
        BridgeScalar::Int32 => ("Int", "Int", "jint", "std::int32_t", None, None, false),
        BridgeScalar::Int64 => ("Long", "Long", "jlong", "std::int64_t", None, None, false),
        BridgeScalar::UInt8 => (
            "UByte",
            "Byte",
            "jbyte",
            "std::uint8_t",
            Some("toByte"),
            Some("toUByte"),
            true,
        ),
        BridgeScalar::UInt16 => (
            "UShort",
            "Short",
            "jshort",
            "std::uint16_t",
            Some("toShort"),
            Some("toUShort"),
            true,
        ),
        BridgeScalar::UInt32 => (
            "UInt",
            "Int",
            "jint",
            "std::uint32_t",
            Some("toInt"),
            Some("toUInt"),
            true,
        ),
        BridgeScalar::UInt64 => (
            "ULong",
            "Long",
            "jlong",
            "std::uint64_t",
            Some("toLong"),
            Some("toULong"),
            true,
        ),
        BridgeScalar::Float32 => ("Float", "Float", "jfloat", "float", None, None, false),
        BridgeScalar::Float64 => ("Double", "Double", "jdouble", "double", None, None, false),
        BridgeScalar::String => (
            "String",
            "String",
            "jstring",
            "std::string",
            None,
            None,
            false,
        ),
        BridgeScalar::Bytes => (
            "ByteArray",
            "ByteArray",
            "jbyteArray",
            "std::vector<std::uint8_t>",
            None,
            None,
            false,
        ),
    };
    if optional && kotlin == "Unit" {
        return None;
    }
    let jni_kotlin = if optional && unsigned {
        "Long"
    } else {
        jni_kotlin
    };
    Some(AndroidValue {
        kotlin,
        jni_kotlin,
        jni: if optional { "jobject" } else { jni },
        cpp,
        kotlin_to_jni,
        kotlin_from_jni,
        unsigned,
        optional,
    })
}
pub(crate) fn android_jni_class_descriptor(ty: &BridgeType) -> Option<String> {
    if ty.is_optional()
        && let Some(boxed) = android_boxed_primitive(bridge_type_name(ty))
    {
        return Some(format!("L{};", boxed.class_name));
    }
    match ty {
        BridgeType::Scalar(scalar) => Some(
            match scalar {
                BridgeScalar::Bool => "Z",
                BridgeScalar::Int8 | BridgeScalar::UInt8 => "B",
                BridgeScalar::Int16 | BridgeScalar::UInt16 => "S",
                BridgeScalar::Int32 | BridgeScalar::UInt32 => "I",
                BridgeScalar::Int64 | BridgeScalar::UInt64 => "J",
                BridgeScalar::Float32 => "F",
                BridgeScalar::Float64 => "D",
                BridgeScalar::String => "Ljava/lang/String;",
                BridgeScalar::Bytes => "[B",
                BridgeScalar::Void => return None,
            }
            .to_owned(),
        ),
        BridgeType::Named { .. } => None,
        BridgeType::Optional(inner) => {
            if let Some(boxed) = android_boxed_primitive(bridge_type_name(inner)) {
                return Some(format!("L{};", boxed.class_name));
            }
            android_jni_class_descriptor(inner)
        }
        BridgeType::Map(..) => Some("Ljava/util/Map;".to_owned()),
        BridgeType::Array(element) => {
            if android_primitive_array(element).is_some() {
                let descriptor = match element.as_ref() {
                    BridgeType::Scalar(BridgeScalar::Bool) => "Z",
                    BridgeType::Scalar(BridgeScalar::Int8 | BridgeScalar::UInt8) => "B",
                    BridgeType::Scalar(BridgeScalar::Int16 | BridgeScalar::UInt16) => "S",
                    BridgeType::Scalar(BridgeScalar::Int32 | BridgeScalar::UInt32) => "I",
                    BridgeType::Scalar(BridgeScalar::Int64 | BridgeScalar::UInt64) => "J",
                    BridgeType::Scalar(BridgeScalar::Float32) => "F",
                    BridgeType::Scalar(BridgeScalar::Float64) => "D",
                    _ => return None,
                };
                Some(format!("[{descriptor}"))
            } else {
                Some(format!("[{}", android_jni_class_descriptor(element)?))
            }
        }
        BridgeType::Set(element) => {
            if android_primitive_array(element).is_some() {
                Some(format!("[{}", android_jni_class_descriptor(element)?))
            } else if android_reference_array_element(element).is_some() {
                Some(format!("[{}", android_jni_class_descriptor(element)?))
            } else {
                None
            }
        }
        BridgeType::Pair(..) | BridgeType::Triple(..) | BridgeType::Result { .. } => None,
    }
}
pub(crate) fn android_primitive_descriptor(ty: &BridgeType) -> &'static str {
    match ty {
        BridgeType::Scalar(BridgeScalar::Bool) => "Z",
        BridgeType::Scalar(BridgeScalar::Int8 | BridgeScalar::UInt8) => "B",
        BridgeType::Scalar(BridgeScalar::Int16 | BridgeScalar::UInt16) => "S",
        BridgeType::Scalar(BridgeScalar::Int32 | BridgeScalar::UInt32) => "I",
        BridgeType::Scalar(BridgeScalar::Int64 | BridgeScalar::UInt64) => "J",
        BridgeType::Scalar(BridgeScalar::Float32) => "F",
        BridgeType::Scalar(BridgeScalar::Float64) => "D",
        BridgeType::Scalar(BridgeScalar::String) => "Ljava/lang/String;",
        BridgeType::Scalar(BridgeScalar::Bytes) => "[B",
        _ => {
            debug_assert!(false, "primitive descriptors cover validated scalar shapes");
            "V"
        }
    }
}
pub(crate) struct AndroidBoxedPrimitive {
    pub(crate) class_name: &'static str,
    pub(crate) unbox_method: &'static str,
    pub(crate) unbox_signature: &'static str,
    pub(crate) unbox_call: &'static str,
    pub(crate) value_of_signature: &'static str,
    pub(crate) jni_primitive: &'static str,
}
pub(crate) fn android_cpp_type(ty: &BridgeType) -> String {
    // Total C++ spelling over validated types. Plan validation proves every
    // occurrence is mappable, so support checks are unnecessary here.
    match ty {
        BridgeType::Map(key, value) => {
            format!(
                "std::map<{}, {}>",
                android_cpp_type(key),
                android_cpp_type(value)
            )
        }
        BridgeType::Array(element) => format!("std::vector<{}>", android_cpp_type(element)),
        BridgeType::Set(element) => format!("std::set<{}>", android_cpp_type(element)),
        BridgeType::Optional(inner) => format!("std::optional<{}>", android_cpp_type(inner)),
        BridgeType::Scalar(_) | BridgeType::Named { .. } => cpp_type(ty),
        BridgeType::Pair(first, second) => format!(
            "std::pair<{}, {}>",
            android_cpp_type(first),
            android_cpp_type(second)
        ),
        BridgeType::Triple(first, second, third) => format!(
            "std::tuple<{}, {}, {}>",
            android_cpp_type(first),
            android_cpp_type(second),
            android_cpp_type(third)
        ),
        BridgeType::Result { success, failure } => {
            debug_assert!(
                false,
                "validated Android values never nest `Result`; success types are unwrapped before mapping"
            );
            format!("NexaResult<{}, {failure}>", android_cpp_type(success))
        }
    }
}
pub(crate) fn android_primitive_array(ty: &BridgeType) -> Option<AndroidPrimitiveArray> {
    // Primitive backing stores require a non-optional scalar element.
    // Strings and bytes use the reference-array path instead.
    let BridgeType::Scalar(scalar) = ty else {
        return None;
    };
    let value = android_scalar_value(*scalar, false);
    let (kotlin_array, jni_array, get_region, set_region) = match scalar {
        BridgeScalar::Bool => (
            "BooleanArray",
            "jbooleanArray",
            "GetBooleanArrayRegion",
            "SetBooleanArrayRegion",
        ),
        BridgeScalar::Int8 | BridgeScalar::UInt8 => (
            "ByteArray",
            "jbyteArray",
            "GetByteArrayRegion",
            "SetByteArrayRegion",
        ),
        BridgeScalar::Int16 | BridgeScalar::UInt16 => (
            "ShortArray",
            "jshortArray",
            "GetShortArrayRegion",
            "SetShortArrayRegion",
        ),
        BridgeScalar::Int32 | BridgeScalar::UInt32 => (
            "IntArray",
            "jintArray",
            "GetIntArrayRegion",
            "SetIntArrayRegion",
        ),
        BridgeScalar::Int64 | BridgeScalar::UInt64 => (
            "LongArray",
            "jlongArray",
            "GetLongArrayRegion",
            "SetLongArrayRegion",
        ),
        BridgeScalar::Float32 => (
            "FloatArray",
            "jfloatArray",
            "GetFloatArrayRegion",
            "SetFloatArrayRegion",
        ),
        BridgeScalar::Float64 => (
            "DoubleArray",
            "jdoubleArray",
            "GetDoubleArrayRegion",
            "SetDoubleArrayRegion",
        ),
        _ => return None,
    };
    Some(AndroidPrimitiveArray {
        kotlin_array,
        jni_array,
        element_jni: value.jni,
        get_region,
        set_region,
    })
}
#[derive(Clone, Copy)]
pub(crate) struct AndroidValue {
    pub(crate) kotlin: &'static str,
    pub(crate) jni_kotlin: &'static str,
    pub(crate) jni: &'static str,
    pub(crate) cpp: &'static str,
    pub(crate) kotlin_to_jni: Option<&'static str>,
    pub(crate) kotlin_from_jni: Option<&'static str>,
    pub(crate) unsigned: bool,
    pub(crate) optional: bool,
}
pub(crate) fn android_boxed_primitive(name: &str) -> Option<AndroidBoxedPrimitive> {
    let (class_name, unbox_method, unbox_signature, unbox_call, value_of_signature, jni_primitive) =
        match name {
            "Bool" => (
                "java/lang/Boolean",
                "booleanValue",
                "()Z",
                "CallBooleanMethod",
                "(Z)Ljava/lang/Boolean;",
                "jboolean",
            ),
            "Int8" => (
                "java/lang/Byte",
                "byteValue",
                "()B",
                "CallByteMethod",
                "(B)Ljava/lang/Byte;",
                "jbyte",
            ),
            "Int16" => (
                "java/lang/Short",
                "shortValue",
                "()S",
                "CallShortMethod",
                "(S)Ljava/lang/Short;",
                "jshort",
            ),
            "Int32" => (
                "java/lang/Integer",
                "intValue",
                "()I",
                "CallIntMethod",
                "(I)Ljava/lang/Integer;",
                "jint",
            ),
            "Int64" => (
                "java/lang/Long",
                "longValue",
                "()J",
                "CallLongMethod",
                "(J)Ljava/lang/Long;",
                "jlong",
            ),
            "UInt8" | "UInt16" | "UInt32" | "UInt64" => (
                "java/lang/Long",
                "longValue",
                "()J",
                "CallLongMethod",
                "(J)Ljava/lang/Long;",
                "jlong",
            ),
            "Float32" => (
                "java/lang/Float",
                "floatValue",
                "()F",
                "CallFloatMethod",
                "(F)Ljava/lang/Float;",
                "jfloat",
            ),
            "Float64" => (
                "java/lang/Double",
                "doubleValue",
                "()D",
                "CallDoubleMethod",
                "(D)Ljava/lang/Double;",
                "jdouble",
            ),
            _ => return None,
        };
    Some(AndroidBoxedPrimitive {
        class_name,
        unbox_method,
        unbox_signature,
        unbox_call,
        value_of_signature,
        jni_primitive,
    })
}
pub(crate) fn android_jni_reference_class_available(ty: &BridgeType) -> bool {
    if android_boxed_primitive(bridge_type_name(ty)).is_some() && ty.is_optional() {
        return true;
    }
    matches!(
        bridge_type_name(ty),
        "Bool"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float32"
            | "Float64"
            | "String"
            | "Bytes"
            | "Map"
            | "Array"
            | "Set"
    ) || (!bridge_type_is_generic(ty) && android_cpp_value(ty).is_none())
}
pub(crate) fn android_jni_map_boxed_primitive(ty: &BridgeType) -> Option<AndroidBoxedPrimitive> {
    let carrier_name = match bridge_strip_optional(ty) {
        BridgeType::Scalar(BridgeScalar::UInt8) => "Int8",
        BridgeType::Scalar(BridgeScalar::UInt16) => "Int16",
        BridgeType::Scalar(BridgeScalar::UInt32) => "Int32",
        BridgeType::Scalar(BridgeScalar::UInt64) => "Int64",
        _ => return android_boxed_primitive(bridge_type_name(ty)),
    };
    android_boxed_primitive(carrier_name)
}
pub(crate) fn android_jni_type(ty: &BridgeType) -> String {
    // Total JNI spelling over validated types. Plan validation proves every
    // occurrence is mappable; unsupported shapes fall back to the universal
    // `jobject` reference exactly where the old mapper produced it.
    match ty {
        BridgeType::Map(..) => "jobject".to_owned(),
        BridgeType::Array(element) | BridgeType::Set(element) => {
            if ty.is_optional() {
                debug_assert!(false, "plan validation rejects optional collections");
                return "jobject".to_owned();
            }
            if let Some(array) = android_primitive_array(element) {
                return array.jni_array.to_owned();
            }
            if !android_jni_reference_class_available(element) {
                debug_assert!(
                    false,
                    "plan validation proves array elements have reference classes"
                );
                return "jobject".to_owned();
            }
            "jobjectArray".to_owned()
        }
        BridgeType::Optional(inner) => {
            // Optional carriers are always references; the historical
            // mapper produced `jobject` for every supported optional shape.
            if matches!(inner.as_ref(), BridgeType::Scalar(BridgeScalar::Void)) {
                debug_assert!(false, "plan validation rejects optional `Void`");
            }
            "jobject".to_owned()
        }
        BridgeType::Scalar(scalar) => android_scalar_value(*scalar, false).jni.to_owned(),
        BridgeType::Named { .. } => "jobject".to_owned(),
        BridgeType::Pair(..) | BridgeType::Triple(..) | BridgeType::Result { .. } => {
            debug_assert!(
                false,
                "plan validation excludes compound value shapes from JNI positions"
            );
            "jobject".to_owned()
        }
    }
}
pub(crate) struct AndroidPrimitiveArray {
    pub(crate) kotlin_array: &'static str,
    pub(crate) jni_array: &'static str,
    pub(crate) element_jni: &'static str,
    pub(crate) get_region: &'static str,
    pub(crate) set_region: &'static str,
}
pub(crate) fn android_kotlin_type(ty: &BridgeType, jni_carrier: bool) -> String {
    match ty {
        BridgeType::Map(key, value) => format!(
            "Map<{}, {}>",
            android_kotlin_type(key, jni_carrier),
            android_kotlin_type(value, jni_carrier)
        ),
        BridgeType::Array(element) => {
            if jni_carrier {
                if let Some(array) = android_primitive_array(element) {
                    return array.kotlin_array.to_owned();
                }
                if android_jni_reference_class_available(element) {
                    return format!("Array<{}>", android_kotlin_type(element, true));
                }
            }
            format!("List<{}>", android_kotlin_type(element, false))
        }
        BridgeType::Set(element) => {
            if jni_carrier {
                if let Some(array) = android_primitive_array(element) {
                    return array.kotlin_array.to_owned();
                }
                if android_jni_reference_class_available(element) {
                    return format!("Array<{}>", android_kotlin_type(element, true));
                }
            }
            format!("Set<{}>", android_kotlin_type(element, false))
        }
        BridgeType::Optional(inner) => {
            // Nullable unsigned integers cross JNI as a boxed signed Long
            // carrier (every bit pattern plus null survives); the scalar
            // table encodes that adjustment through the optional flag.
            if jni_carrier && matches!(inner.as_ref(), BridgeType::Scalar(_)) {
                let scalar = match inner.as_ref() {
                    BridgeType::Scalar(scalar) => *scalar,
                    _ => {
                        debug_assert!(false, "optional scalar carrier should map");
                        return format!("{}?", android_kotlin_type(inner, jni_carrier));
                    }
                };
                // Optional `Void` never validates, but the historical
                // spelling fell back to the source name; preserve it.
                if scalar == BridgeScalar::Void {
                    return "Void?".to_owned();
                }
                return format!("{}?", android_scalar_value(scalar, true).jni_kotlin);
            }
            format!("{}?", android_kotlin_type(inner, jni_carrier))
        }
        BridgeType::Scalar(scalar) => {
            let value = android_scalar_value(*scalar, false);
            if jni_carrier {
                value.jni_kotlin.to_owned()
            } else {
                value.kotlin.to_owned()
            }
        }
        BridgeType::Named { name, .. } => name.clone(),
        BridgeType::Pair(first, second) => format!(
            "Pair<{}, {}>",
            android_kotlin_type(first, jni_carrier),
            android_kotlin_type(second, jni_carrier)
        ),
        BridgeType::Triple(first, second, third) => format!(
            "Triple<{}, {}, {}>",
            android_kotlin_type(first, jni_carrier),
            android_kotlin_type(second, jni_carrier),
            android_kotlin_type(third, jni_carrier)
        ),
        BridgeType::Result { .. } => {
            debug_assert!(
                false,
                "validated Kotlin types never nest `Result`; success types are unwrapped before mapping"
            );
            "Any".to_owned()
        }
    }
}
pub(crate) fn android_reference_array_element(ty: &BridgeType) -> Option<AndroidValue> {
    if !matches!(
        ty,
        BridgeType::Scalar(BridgeScalar::String | BridgeScalar::Bytes)
    ) {
        return None;
    }
    android_cpp_value(ty)
}
/// Total scalar table behind [`android_cpp_value`]. The optional-`Void`
/// combination is rejected by plan validation; the arm below is unreachable
/// in practice and mirrors the historical adjustment.
pub(crate) fn android_scalar_value(scalar: BridgeScalar, optional: bool) -> AndroidValue {
    let (kotlin, jni_kotlin, jni, cpp, kotlin_to_jni, kotlin_from_jni, unsigned) = match scalar {
        BridgeScalar::Void => ("Unit", "Unit", "void", "void", None, None, false),
        BridgeScalar::Bool => ("Boolean", "Boolean", "jboolean", "bool", None, None, false),
        BridgeScalar::Int8 => ("Byte", "Byte", "jbyte", "std::int8_t", None, None, false),
        BridgeScalar::Int16 => (
            "Short",
            "Short",
            "jshort",
            "std::int16_t",
            None,
            None,
            false,
        ),
        BridgeScalar::Int32 => ("Int", "Int", "jint", "std::int32_t", None, None, false),
        BridgeScalar::Int64 => ("Long", "Long", "jlong", "std::int64_t", None, None, false),
        BridgeScalar::UInt8 => (
            "UByte",
            "Byte",
            "jbyte",
            "std::uint8_t",
            Some("toByte"),
            Some("toUByte"),
            true,
        ),
        BridgeScalar::UInt16 => (
            "UShort",
            "Short",
            "jshort",
            "std::uint16_t",
            Some("toShort"),
            Some("toUShort"),
            true,
        ),
        BridgeScalar::UInt32 => (
            "UInt",
            "Int",
            "jint",
            "std::uint32_t",
            Some("toInt"),
            Some("toUInt"),
            true,
        ),
        BridgeScalar::UInt64 => (
            "ULong",
            "Long",
            "jlong",
            "std::uint64_t",
            Some("toLong"),
            Some("toULong"),
            true,
        ),
        BridgeScalar::Float32 => ("Float", "Float", "jfloat", "float", None, None, false),
        BridgeScalar::Float64 => ("Double", "Double", "jdouble", "double", None, None, false),
        BridgeScalar::String => (
            "String",
            "String",
            "jstring",
            "std::string",
            None,
            None,
            false,
        ),
        BridgeScalar::Bytes => (
            "ByteArray",
            "ByteArray",
            "jbyteArray",
            "std::vector<std::uint8_t>",
            None,
            None,
            false,
        ),
    };
    debug_assert!(
        !(optional && matches!(scalar, BridgeScalar::Void)),
        "plan validation rejects optional `Void`"
    );
    let jni_kotlin = if optional && unsigned {
        "Long"
    } else {
        jni_kotlin
    };
    AndroidValue {
        kotlin,
        jni_kotlin,
        jni: if optional { "jobject" } else { jni },
        cpp,
        kotlin_to_jni,
        kotlin_from_jni,
        unsigned,
        optional,
    }
}
