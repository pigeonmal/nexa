//! Direct native binding skeletons generated from the plugin IDL.

use nexa_plugin_idl::{Method, NamedType, PluginIdl, TypeRef, abi::AbiContract};

pub(crate) fn c_header(idl: &PluginIdl, abi: &AbiContract) -> Result<String, String> {
    abi.validate()?;
    let mut out = String::from(
        "#ifndef NEXA_PLUGIN_BINDINGS_H\n#define NEXA_PLUGIN_BINDINGS_H\n\n#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n\n/* Nexa ABI schema 1: call-scoped borrowed inputs, caller-owned returns. */\ntypedef struct { const char *data; size_t length; } NexaBorrowedString;\ntypedef struct { const uint8_t *data; size_t length; } NexaBorrowedBytes;\ntypedef struct { char *data; size_t length; } NexaOwnedString;\ntypedef struct { uint8_t *data; size_t length; } NexaOwnedBytes;\n\n#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n",
    );
    for interface in &idl.interfaces {
        for method in &interface.methods {
            if method.is_async {
                return Err(format!(
                    "C ABI generation does not support async method `{}`; add a native callback contract first",
                    method.name
                ));
            }
            if method.return_type.name == "Result" {
                return Err(format!(
                    "C ABI generation does not support Result method `{}` until error transport is versioned",
                    method.name
                ));
            }
            let return_type = c_type(&method.return_type, true, abi)?;
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| {
                    if parameter.ty.name == "Void" {
                        return Err(format!(
                            "C ABI generation does not support a Void parameter `{}`",
                            parameter.name
                        ));
                    }
                    Ok(format!(
                        "{} {}",
                        c_type(&parameter.ty, false, abi)?,
                        parameter.name
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?
                .join(", ");
            let parameters = if parameters.is_empty() {
                "void".to_owned()
            } else {
                parameters
            };
            out.push_str(&format!(
                "{} {}_{}({});\n",
                return_type, interface.name, method.name, parameters
            ));
        }
    }
    out.push_str("\n#ifdef __cplusplus\n}\n#endif\n\n#endif /* NEXA_PLUGIN_BINDINGS_H */\n");
    Ok(out)
}

fn c_type(ty: &TypeRef, return_position: bool, abi: &AbiContract) -> Result<&'static str, String> {
    if ty.optional {
        return Err(format!(
            "C ABI generation does not support optional `{}` until nullable layout is declared",
            ty.name
        ));
    }
    match ty.name.as_str() {
        "Void" => Ok("void"),
        "Bool" => Ok("bool"),
        "Int8" => Ok("int8_t"),
        "Int16" => Ok("int16_t"),
        "Int32" => Ok("int32_t"),
        "Int64" => Ok("int64_t"),
        "UInt8" => Ok("uint8_t"),
        "UInt16" => Ok("uint16_t"),
        "UInt32" => Ok("uint32_t"),
        "UInt64" => Ok("uint64_t"),
        "Float32" => Ok("float"),
        "Float64" => Ok("double"),
        "String" => {
            if return_position {
                if abi.returned_buffers != nexa_plugin_idl::abi::ReturnOwnership::CallerOwned {
                    return Err(
                        "C ABI generation currently requires caller-owned returned strings"
                            .to_owned(),
                    );
                }
                Ok("NexaOwnedString")
            } else {
                Ok(match abi.strings {
                    nexa_plugin_idl::abi::BufferOwnership::BorrowedReadOnly => "NexaBorrowedString",
                    nexa_plugin_idl::abi::BufferOwnership::Owned => "NexaOwnedString",
                })
            }
        }
        "Bytes" => {
            if return_position {
                if abi.returned_buffers != nexa_plugin_idl::abi::ReturnOwnership::CallerOwned {
                    return Err(
                        "C ABI generation currently requires caller-owned returned bytes"
                            .to_owned(),
                    );
                }
                Ok("NexaOwnedBytes")
            } else {
                Ok(match abi.bytes {
                    nexa_plugin_idl::abi::BufferOwnership::BorrowedReadOnly => "NexaBorrowedBytes",
                    nexa_plugin_idl::abi::BufferOwnership::Owned => "NexaOwnedBytes",
                })
            }
        }
        "Array" | "Set" | "Map" | "Pair" | "Triple" => Err(format!(
            "C ABI generation does not support generic collection `{}` until its native layout is fixed",
            ty.name
        )),
        _ => Err(format!(
            "C ABI generation does not support named type `{}` until its native layout is fixed",
            ty.name
        )),
    }
}

pub(crate) fn swift(idl: &PluginIdl) -> String {
    let mut out = String::from("import Foundation\n\n");
    for ty in &idl.types {
        out.push_str(&swift_named_type(ty));
        out.push('\n');
    }
    for interface in &idl.interfaces {
        out.push_str(&format!("public protocol {} {{\n", interface.name));
        for method in &interface.methods {
            out.push_str("    ");
            out.push_str(&swift_method(method));
            out.push('\n');
        }
        out.push_str("}\n\n");
    }
    out
}

pub(crate) fn kotlin(idl: &PluginIdl, package: &str) -> String {
    let mut out = format!("package {package}\n\n");
    for ty in &idl.types {
        out.push_str(&kotlin_named_type(ty));
        out.push('\n');
    }
    for interface in &idl.interfaces {
        out.push_str(&format!("public interface {} {{\n", interface.name));
        for method in &interface.methods {
            out.push_str("    ");
            out.push_str(&kotlin_method(method));
            out.push('\n');
        }
        out.push_str("}\n\n");
    }
    out
}

fn swift_named_type(ty: &NamedType) -> String {
    if ty.is_error {
        format!("public struct {}: Error {{}}", ty.name)
    } else {
        format!("public struct {} {{}}", ty.name)
    }
}

fn kotlin_named_type(ty: &NamedType) -> String {
    if ty.is_error {
        format!("public class {} : Exception()", ty.name)
    } else {
        format!("public class {}", ty.name)
    }
}

fn swift_method(method: &Method) -> String {
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, swift_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let (return_type, throws) = result_return(&method.return_type);
    format!(
        "func {}({}){}{} -> {}",
        method.name,
        parameters,
        if method.is_async { " async" } else { "" },
        if throws { " throws" } else { "" },
        swift_type(return_type)
    )
}

fn kotlin_method(method: &Method) -> String {
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, kotlin_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let (return_type, _) = result_return(&method.return_type);
    format!(
        "{}fun {}({}): {}",
        if method.is_async { "suspend " } else { "" },
        method.name,
        parameters,
        kotlin_type(return_type)
    )
}

fn result_return(ty: &TypeRef) -> (&TypeRef, bool) {
    if ty.name == "Result" && ty.arguments.len() == 2 {
        (&ty.arguments[0], true)
    } else {
        (ty, false)
    }
}

fn swift_type(ty: &TypeRef) -> String {
    let base = match ty.name.as_str() {
        "Void" => "Void".to_owned(),
        "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32" | "UInt64"
        | "Float32" | "Float64" | "Bool" | "String" => ty.name.clone(),
        "Bytes" => "Data".to_owned(),
        "Array" if ty.arguments.len() == 1 => format!("[{}]", swift_type(&ty.arguments[0])),
        "Set" if ty.arguments.len() == 1 => format!("Set<{}>", swift_type(&ty.arguments[0])),
        "Map" if ty.arguments.len() == 2 => format!(
            "[{}: {}]",
            swift_type(&ty.arguments[0]),
            swift_type(&ty.arguments[1])
        ),
        "Pair" if ty.arguments.len() == 2 => format!(
            "({}, {})",
            swift_type(&ty.arguments[0]),
            swift_type(&ty.arguments[1])
        ),
        "Triple" if ty.arguments.len() == 3 => format!(
            "({}, {}, {})",
            swift_type(&ty.arguments[0]),
            swift_type(&ty.arguments[1]),
            swift_type(&ty.arguments[2])
        ),
        "Result" if ty.arguments.len() == 2 => format!(
            "Result<{}, {}>",
            swift_type(&ty.arguments[0]),
            swift_type(&ty.arguments[1])
        ),
        _ => ty.name.clone(),
    };
    if ty.optional {
        format!("{base}?")
    } else {
        base
    }
}

fn kotlin_type(ty: &TypeRef) -> String {
    let base = match ty.name.as_str() {
        "Void" => "Unit".to_owned(),
        "Int8" => "Byte".to_owned(),
        "Int16" => "Short".to_owned(),
        "Int32" => "Int".to_owned(),
        "Int64" => "Long".to_owned(),
        "UInt8" => "UByte".to_owned(),
        "UInt16" => "UShort".to_owned(),
        "UInt32" => "UInt".to_owned(),
        "UInt64" => "ULong".to_owned(),
        "Float32" => "Float".to_owned(),
        "Float64" => "Double".to_owned(),
        "Bool" => "Boolean".to_owned(),
        "String" => "String".to_owned(),
        "Bytes" => "ByteArray".to_owned(),
        "Array" if ty.arguments.len() == 1 => format!("List<{}>", kotlin_type(&ty.arguments[0])),
        "Set" if ty.arguments.len() == 1 => format!("Set<{}>", kotlin_type(&ty.arguments[0])),
        "Map" if ty.arguments.len() == 2 => format!(
            "Map<{}, {}>",
            kotlin_type(&ty.arguments[0]),
            kotlin_type(&ty.arguments[1])
        ),
        "Pair" if ty.arguments.len() == 2 => format!(
            "Pair<{}, {}>",
            kotlin_type(&ty.arguments[0]),
            kotlin_type(&ty.arguments[1])
        ),
        "Triple" if ty.arguments.len() == 3 => format!(
            "Triple<{}, {}, {}>",
            kotlin_type(&ty.arguments[0]),
            kotlin_type(&ty.arguments[1]),
            kotlin_type(&ty.arguments[2])
        ),
        "Result" if ty.arguments.len() == 2 => format!(
            "Result<{}, {}>",
            kotlin_type(&ty.arguments[0]),
            kotlin_type(&ty.arguments[1])
        ),
        _ => ty.name.clone(),
    };
    if ty.optional {
        format!("{base}?")
    } else {
        base
    }
}
