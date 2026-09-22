//! Direct native binding skeletons generated from the plugin IDL.

use super::idl::{Method, NamedType, PluginIdl, TypeRef};

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
