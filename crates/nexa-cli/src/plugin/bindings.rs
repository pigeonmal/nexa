//! Direct native binding skeletons generated from the plugin IDL.

use nexa_plugin_idl::{
    Event, Interface, InterfaceKind, Method, NamedType, NamedTypeKind, PluginIdl, Property, TypeRef,
};

pub(crate) fn swift(idl: &PluginIdl) -> String {
    let mut out = String::from("import Foundation\n\n");
    for ty in &idl.types {
        out.push_str(&swift_named_type(ty));
        out.push('\n');
    }
    for interface in &idl.interfaces {
        let contract_name = swift_contract_name(interface);
        let inheritance = if matches!(
            interface.kind,
            InterfaceKind::NativeClass | InterfaceKind::NativeComponent
        ) {
            ": AnyObject"
        } else {
            ""
        };
        out.push_str(&format!(
            "public protocol {contract_name}{inheritance} {{\n"
        ));
        for constructor in &interface.constructors {
            out.push_str("    init(");
            out.push_str(&swift_parameters(&constructor.parameters));
            out.push_str(")\n");
        }
        for property in &interface.properties {
            out.push_str("    ");
            out.push_str(&swift_property(property));
            out.push('\n');
        }
        for method in &interface.methods {
            out.push_str("    ");
            out.push_str(&swift_method(method));
            out.push('\n');
        }
        for event in &interface.events {
            out.push_str("    ");
            out.push_str(&swift_event(event));
            out.push('\n');
        }
        out.push_str("}\n\n");
        if interface.kind == InterfaceKind::NativeClass {
            out.push_str(&format!(
                "public typealias {} = {}Impl\n\n",
                interface.name, interface.name
            ));
        }
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
        out.push_str(&format!(
            "public interface {} {{\n",
            kotlin_contract_name(interface)
        ));
        // Kotlin interfaces cannot declare constructors. The generated
        // implementation alias below keeps construction direct while the
        // implementation class owns its concrete constructor.
        for property in &interface.properties {
            out.push_str("    ");
            out.push_str(&kotlin_property(property));
            out.push('\n');
        }
        for method in &interface.methods {
            out.push_str("    ");
            out.push_str(&kotlin_method(method));
            out.push('\n');
        }
        for event in &interface.events {
            out.push_str("    ");
            out.push_str(&kotlin_event(event));
            out.push('\n');
        }
        out.push_str("}\n\n");
        if interface.kind == InterfaceKind::NativeClass {
            out.push_str(&format!(
                "public typealias {} = {}Impl\n\n",
                interface.name, interface.name
            ));
        }
    }
    out
}

fn swift_named_type(ty: &NamedType) -> String {
    match ty.kind {
        NamedTypeKind::Enum => format!(
            "public enum {} {{\n{}\n}}",
            ty.name,
            ty.cases
                .iter()
                .map(|case_name| format!("    case {case_name}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        NamedTypeKind::Error => {
            let cases = if ty.cases.is_empty() {
                String::new()
            } else {
                ty.cases
                    .iter()
                    .map(|case_name| format!("    case {case_name}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!("public enum {}: Error {{\n{}\n}}", ty.name, cases)
        }
        NamedTypeKind::Struct => {
            let fields = ty
                .fields
                .iter()
                .map(|field| format!("    public let {}: {}", field.name, swift_type(&field.ty)))
                .collect::<Vec<_>>();
            let parameters = ty
                .fields
                .iter()
                .map(|field| format!("{}: {}", field.name, swift_type(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            let assignments = ty
                .fields
                .iter()
                .map(|field| format!("        self.{} = {}", field.name, field.name))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "public struct {} {{\n{}\n\n    public init({}) {{\n{}\n    }}\n}}",
                ty.name,
                fields.join("\n"),
                parameters,
                assignments
            )
        }
        NamedTypeKind::Opaque => {
            if ty.is_error {
                format!("public struct {}: Error {{}}", ty.name)
            } else {
                format!("public struct {} {{}}", ty.name)
            }
        }
    }
}

fn kotlin_named_type(ty: &NamedType) -> String {
    match ty.kind {
        NamedTypeKind::Enum => format!(
            "public enum class {} {{ {} }}",
            ty.name,
            ty.cases.join(", ")
        ),
        NamedTypeKind::Error => format!("public open class {} : Exception()", ty.name),
        NamedTypeKind::Struct => {
            let fields = ty
                .fields
                .iter()
                .map(|field| format!("val {}: {}", field.name, kotlin_type(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("public data class {}({})", ty.name, fields)
        }
        NamedTypeKind::Opaque => {
            if ty.is_error {
                format!("public open class {} : Exception()", ty.name)
            } else {
                format!("public class {}", ty.name)
            }
        }
    }
}

fn swift_method(method: &Method) -> String {
    let parameters = swift_parameters(&method.parameters);
    let (return_type, result_throws) = result_return(&method.return_type);
    format!(
        "func {}({}){}{} -> {}",
        method.name,
        parameters,
        if method.is_async { " async" } else { "" },
        if result_throws || method.throws.is_some() {
            " throws"
        } else {
            ""
        },
        swift_type(return_type)
    )
}

fn kotlin_method(method: &Method) -> String {
    let parameters = kotlin_parameters(&method.parameters);
    let (return_type, _) = result_return(&method.return_type);
    format!(
        "{}fun {}({}): {}",
        if method.is_async { "suspend " } else { "" },
        method.name,
        parameters,
        kotlin_type(return_type)
    )
}

fn swift_contract_name(interface: &Interface) -> String {
    match interface.kind {
        InterfaceKind::NativeClass => format!("{}Spec", interface.name),
        InterfaceKind::NativeComponent => format!("{}ComponentSpec", interface.name),
        InterfaceKind::Interface | InterfaceKind::Service => interface.name.clone(),
    }
}

fn kotlin_contract_name(interface: &Interface) -> String {
    match interface.kind {
        InterfaceKind::NativeClass => format!("{}Spec", interface.name),
        InterfaceKind::NativeComponent => format!("{}ComponentSpec", interface.name),
        InterfaceKind::Interface | InterfaceKind::Service => interface.name.clone(),
    }
}

fn swift_parameters(parameters: &[nexa_plugin_idl::Parameter]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, swift_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn kotlin_parameters(parameters: &[nexa_plugin_idl::Parameter]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, kotlin_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn swift_property(property: &Property) -> String {
    format!(
        "var {}: {} {{ get{} }}",
        property.name,
        swift_type(&property.ty),
        if property.mutable { " set" } else { "" }
    )
}

fn kotlin_property(property: &Property) -> String {
    format!(
        "{} {}: {}",
        if property.mutable { "var" } else { "val" },
        property.name,
        kotlin_type(&property.ty)
    )
}

fn swift_event(event: &Event) -> String {
    let parameters = event
        .parameters
        .iter()
        .map(|parameter| swift_type(&parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let callback = if parameters.is_empty() {
        "() -> Void".to_owned()
    } else {
        format!("({parameters}) -> Void")
    };
    format!(
        "var on{}: ({})? {{ get set }}",
        type_name(&event.name),
        callback
    )
}

fn kotlin_event(event: &Event) -> String {
    let parameters = event
        .parameters
        .iter()
        .map(|parameter| kotlin_type(&parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let callback = if parameters.is_empty() {
        "() -> Unit".to_owned()
    } else {
        format!("({parameters}) -> Unit")
    };
    format!("var on{}: ({})?", type_name(&event.name), callback)
}

fn type_name(value: &str) -> String {
    let mut output = String::new();
    let mut uppercase = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if uppercase {
                output.extend(character.to_uppercase());
                uppercase = false;
            } else {
                output.push(character);
            }
        } else {
            uppercase = true;
        }
    }
    output
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
