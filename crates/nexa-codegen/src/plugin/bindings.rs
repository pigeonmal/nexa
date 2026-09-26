//! Direct native binding skeletons generated from the plugin IDL.
#![allow(clippy::all)]

use nexa_plugin_idl::{InterfaceKind, Literal};

use super::bridge_plan::{
    BridgeEvent, BridgeInterface, BridgeMethod, BridgeNamedType, BridgeParameter, BridgePlan,
    BridgeProperty, BridgeScalar, BridgeType, BridgeTypeKind,
};

pub fn swift(plan: &BridgePlan) -> String {
    let has_components = plan
        .interfaces
        .iter()
        .any(|interface| interface.kind == InterfaceKind::NativeComponent);
    let mut out = String::from("import Foundation\n");
    if has_components {
        out.push_str("import SwiftUI\n");
    }
    out.push('\n');
    for ty in &plan.types {
        out.push_str(&swift_named_type(ty));
        out.push('\n');
    }
    for interface in &plan.interfaces {
        if interface.kind == InterfaceKind::NativeComponent {
            out.push_str(&swift_native_component(interface));
            out.push_str("\n\n");
            continue;
        }
        let contract_name = swift_contract_name(interface);
        let inheritance = if matches!(
            interface.kind,
            InterfaceKind::NativeClass | InterfaceKind::NativeComponent
        ) {
            ": AnyObject"
        } else {
            ""
        };
        if interface.kind == InterfaceKind::NativeClass {
            out.push_str("@MainActor\n");
        }
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
                "public typealias {} = {}Impl\n\n@MainActor private func _nexaCheck{}Implementation(_ value: {}Impl) -> any {} {{ value }}\n\n",
                interface.name,
                interface.name,
                interface.name,
                interface.name,
                swift_contract_name(interface)
            ));
        }
    }
    out
}

pub fn kotlin(plan: &BridgePlan, package: &str) -> String {
    let has_components = plan
        .interfaces
        .iter()
        .any(|interface| interface.kind == InterfaceKind::NativeComponent);
    let mut out = format!("package {package}\n\n");
    if has_components {
        out.push_str("import androidx.compose.runtime.Composable\n\n");
    }
    for ty in &plan.types {
        out.push_str(&kotlin_named_type(ty));
        out.push('\n');
    }
    for interface in &plan.interfaces {
        if interface.kind == InterfaceKind::NativeComponent {
            out.push_str(&kotlin_native_component(interface));
            out.push_str("\n\n");
            continue;
        }
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
            let constructor = interface.constructors.first();
            let (constructor_parameters, constructor_arguments) = constructor
                .map(|constructor| {
                    let parameters = constructor
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(index, parameter)| {
                            (
                                format!("nexaArg{index}: {}", kotlin_type(&parameter.ty)),
                                format!("nexaArg{index}"),
                            )
                        })
                        .collect::<Vec<_>>();
                    parameters.into_iter().unzip()
                })
                .unwrap_or_else(|| (Vec::new(), Vec::new()));
            out.push_str(&format!(
                "public typealias {} = {}Impl\n\nprivate fun _nexaCheck{}Implementation(value: {}Impl): {}Spec = value\nprivate fun _nexaConstruct{}({}): {}Spec = {}Impl({})\n\n",
                interface.name,
                interface.name,
                interface.name,
                interface.name,
                interface.name,
                interface.name,
                constructor_parameters.join(", "),
                interface.name,
                interface.name,
                constructor_arguments.join(", ")
            ));
        }
    }
    out
}

fn swift_native_component(interface: &BridgeInterface) -> String {
    let generic = if interface.has_content_slot {
        "<Content: View>"
    } else {
        ""
    };
    let mut out = format!("public struct {}{generic}: View {{\n", interface.name);
    for property in &interface.properties {
        out.push_str(&format!(
            "    public let {}: {}\n",
            property.name,
            swift_type(&property.ty)
        ));
    }
    for event in &interface.events {
        out.push_str(&format!(
            "    public let {}: {}?\n",
            nexa_plugin_idl::event_callback_property(&event.name),
            event_callback_type(event, true)
        ));
    }
    if interface.has_content_slot {
        out.push_str("    public let content: Content\n");
    }
    out.push_str("\n    public init(");
    let mut parameters = interface
        .properties
        .iter()
        .map(|property| {
            format!(
                "{}: {}{}",
                property.name,
                swift_type(&property.ty),
                property
                    .default
                    .as_ref()
                    .map(|value| format!(" = {}", swift_literal(value)))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    parameters.extend(interface.events.iter().map(|event| {
        format!(
            "{}: {}? = nil",
            nexa_plugin_idl::event_callback_property(&event.name),
            event_callback_type(event, true)
        )
    }));
    if interface.has_content_slot {
        parameters.push("@ViewBuilder content: () -> Content".to_owned());
    }
    out.push_str(&parameters.join(", "));
    out.push_str(") {\n");
    for property in &interface.properties {
        out.push_str(&format!("        self.{0} = {0}\n", property.name));
    }
    for event in &interface.events {
        let name = nexa_plugin_idl::event_callback_property(&event.name);
        out.push_str(&format!("        self.{name} = {name}\n"));
    }
    if interface.has_content_slot {
        out.push_str("        self.content = content()\n");
    }
    out.push_str("    }\n\n    public var body: some View {\n        ");
    out.push_str(&format!("{}Impl(", interface.name));
    let mut arguments = interface
        .properties
        .iter()
        .map(|property| format!("{}: {}", property.name, property.name))
        .collect::<Vec<_>>();
    arguments.extend(interface.events.iter().map(|event| {
        let name = nexa_plugin_idl::event_callback_property(&event.name);
        format!("{name}: {name}")
    }));
    if interface.has_content_slot {
        arguments.push("content: content".to_owned());
    }
    out.push_str(&arguments.join(", "));
    out.push_str(")\n    }\n}");
    out
}

fn kotlin_native_component(interface: &BridgeInterface) -> String {
    let mut parameters = interface
        .properties
        .iter()
        .map(|property| {
            format!(
                "{}: {}{}",
                property.name,
                kotlin_type(&property.ty),
                property
                    .default
                    .as_ref()
                    .map(|value| format!(" = {}", kotlin_literal(value)))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    parameters.extend(interface.events.iter().map(|event| {
        format!(
            "{}: {}? = null",
            nexa_plugin_idl::event_callback_property(&event.name),
            event_callback_type(event, false)
        )
    }));
    let mut arguments = interface
        .properties
        .iter()
        .map(|property| format!("{} = {}", property.name, property.name))
        .collect::<Vec<_>>();
    arguments.extend(interface.events.iter().map(|event| {
        let name = nexa_plugin_idl::event_callback_property(&event.name);
        format!("{name} = {name}")
    }));
    if interface.has_content_slot {
        parameters.push("content: @Composable () -> Unit".to_owned());
        arguments.push("content = content".to_owned());
    }
    format!(
        "@Composable\npublic fun {}({}) {{\n    {}Impl({})\n}}",
        interface.name,
        parameters.join(", "),
        interface.name,
        arguments.join(", ")
    )
}

fn swift_literal(value: &Literal) -> String {
    match value {
        Literal::String(value) => format!("\"{}\"", escape_string(value, false)),
        Literal::Number(value) => value.clone(),
        Literal::Bool(value) => value.to_string(),
        Literal::Null => "nil".to_owned(),
    }
}

fn kotlin_literal(value: &Literal) -> String {
    match value {
        Literal::String(value) => format!("\"{}\"", escape_string(value, true)),
        Literal::Number(value) => value.clone(),
        Literal::Bool(value) => value.to_string(),
        Literal::Null => "null".to_owned(),
    }
}

fn escape_string(value: &str, escape_dollar: bool) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '$' if escape_dollar => escaped.push_str("\\$"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn event_callback_type(event: &BridgeEvent, swift: bool) -> String {
    let parameters = if swift {
        event
            .parameters
            .iter()
            .map(|parameter| swift_type(&parameter.ty))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        event
            .parameters
            .iter()
            .map(|parameter| kotlin_type(&parameter.ty))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let callback = if swift {
        if parameters.is_empty() {
            "() -> Void".to_owned()
        } else {
            format!("({parameters}) -> Void")
        }
    } else if parameters.is_empty() {
        "() -> Unit".to_owned()
    } else {
        format!("({parameters}) -> Unit")
    };
    format!("({callback})")
}

fn swift_named_type(ty: &BridgeNamedType) -> String {
    match ty.kind {
        BridgeTypeKind::Enum => format!(
            "public enum {} {{\n{}\n}}",
            ty.name,
            ty.cases
                .iter()
                .map(|variant| format!("    case {}", variant.name))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        BridgeTypeKind::Error => {
            let cases = if ty.cases.is_empty() {
                String::new()
            } else {
                ty.cases
                    .iter()
                    .map(|variant| {
                        let payload = if variant.parameters.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "({})",
                                variant
                                    .parameters
                                    .iter()
                                    .map(|parameter| format!(
                                        "{}: {}",
                                        parameter.name,
                                        swift_type(&parameter.ty)
                                    ))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        };
                        format!("    case {}{payload}", variant.name)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!("public enum {}: Error {{\n{}\n}}", ty.name, cases)
        }
        BridgeTypeKind::Struct => {
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
    }
}

fn kotlin_named_type(ty: &BridgeNamedType) -> String {
    match ty.kind {
        BridgeTypeKind::Enum => format!(
            "public enum class {} {{ {} }}",
            ty.name,
            ty.cases
                .iter()
                .map(|variant| variant.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        BridgeTypeKind::Error => {
            let cases = ty
                .cases
                .iter()
                .map(|variant| {
                    if variant.parameters.is_empty() {
                        format!("    public object {} : {}()", variant.name, ty.name)
                    } else {
                        let parameters = variant
                            .parameters
                            .iter()
                            .map(|parameter| {
                                let override_modifier = if parameter.name == "message"
                                    && matches!(
                                        match &parameter.ty {
                                            BridgeType::Optional(inner) => inner.as_ref(),
                                            other => other,
                                        },
                                        BridgeType::Scalar(BridgeScalar::String)
                                    ) {
                                    "override "
                                } else {
                                    ""
                                };
                                format!(
                                    "{override_modifier}val {}: {}",
                                    parameter.name,
                                    kotlin_type(&parameter.ty)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!(
                            "    public data class {}({}) : {}()",
                            variant.name, parameters, ty.name
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "public sealed class {} : Exception() {{\n{}\n}}",
                ty.name, cases
            )
        }
        BridgeTypeKind::Struct => {
            let fields = ty
                .fields
                .iter()
                .map(|field| format!("val {}: {}", field.name, kotlin_type(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("public data class {}({})", ty.name, fields)
        }
    }
}

fn swift_method(method: &BridgeMethod) -> String {
    let parameters = swift_parameters(&method.parameters);
    let return_type = method.success_type();
    let throws = method
        .error_type()
        .map(|error| format!(" throws({error})"))
        .unwrap_or_default();
    format!(
        "func {}({}){}{} -> {}",
        method.name,
        parameters,
        if method.is_async { " async" } else { "" },
        throws,
        swift_type(return_type)
    )
}

fn kotlin_method(method: &BridgeMethod) -> String {
    let parameters = kotlin_parameters(&method.parameters);
    let return_type = method.success_type();
    let annotation = method
        .error_type()
        .map(|error| format!("@Throws({error}::class)\n"))
        .unwrap_or_default();
    format!(
        "{}{}fun {}({}): {}",
        annotation,
        if method.is_async { "suspend " } else { "" },
        method.name,
        parameters,
        kotlin_type(return_type)
    )
}

fn swift_contract_name(interface: &BridgeInterface) -> String {
    match interface.kind {
        InterfaceKind::NativeClass => format!("{}Spec", interface.name),
        InterfaceKind::NativeComponent => format!("{}ComponentSpec", interface.name),
        InterfaceKind::Interface | InterfaceKind::Service => interface.name.clone(),
    }
}

fn kotlin_contract_name(interface: &BridgeInterface) -> String {
    match interface.kind {
        InterfaceKind::NativeClass => format!("{}Spec", interface.name),
        InterfaceKind::NativeComponent => format!("{}ComponentSpec", interface.name),
        InterfaceKind::Interface | InterfaceKind::Service => interface.name.clone(),
    }
}

fn swift_parameters(parameters: &[BridgeParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("_ {}: {}", parameter.name, swift_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn kotlin_parameters(parameters: &[BridgeParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, kotlin_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn swift_property(property: &BridgeProperty) -> String {
    format!(
        "var {}: {} {{ get{} }}",
        property.name,
        swift_type(&property.ty),
        if property.mutable { " set" } else { "" }
    )
}

fn kotlin_property(property: &BridgeProperty) -> String {
    format!(
        "{} {}: {}",
        if property.mutable { "var" } else { "val" },
        property.name,
        kotlin_type(&property.ty)
    )
}

fn swift_event(event: &BridgeEvent) -> String {
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
        "var {}: ({})? {{ get set }}",
        nexa_plugin_idl::event_callback_property(&event.name),
        callback
    )
}

fn kotlin_event(event: &BridgeEvent) -> String {
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
    format!(
        "var {}: ({})?",
        nexa_plugin_idl::event_callback_property(&event.name),
        callback
    )
}

#[cfg(test)]
mod tests {
    use super::{kotlin, swift};
    use crate::plugin::bridge_plan::BridgePlan;

    fn validate(source: &str) -> BridgePlan {
        let idl = nexa_plugin_idl::parse(source).expect("IDL should parse");
        BridgePlan::validate_contract(&idl).expect("contract should validate")
    }

    #[test]
    fn generated_native_classes_are_checked_against_their_contracts() {
        let plan = validate(
            r#"
            native class VideoPlayer {
                init()
                readonly property state: String
                fn play()
            }
            "#,
        );
        let swift = swift(&plan);
        assert!(swift.contains("@MainActor\npublic protocol VideoPlayerSpec: AnyObject"));
        assert!(swift.contains(
            "@MainActor private func _nexaCheckVideoPlayerImplementation(_ value: VideoPlayerImpl) -> any VideoPlayerSpec { value }"
        ));
        let kotlin = kotlin(&plan, "dev.example.video");
        assert!(kotlin.contains(
            "private fun _nexaCheckVideoPlayerImplementation(value: VideoPlayerImpl): VideoPlayerSpec = value"
        ));
        assert!(kotlin.contains(
            "private fun _nexaConstructVideoPlayer(): VideoPlayerSpec = VideoPlayerImpl()"
        ));
    }

    #[test]
    fn kotlin_native_class_factory_probe_checks_typed_constructor_arguments() {
        let plan = validate(
            r#"
            struct PlayerOptions { autoplay: Bool }
            native class VideoPlayer {
                init(options: PlayerOptions)
            }
            "#,
        );
        let kotlin = kotlin(&plan, "dev.example.video");
        assert!(kotlin.contains(
            "private fun _nexaConstructVideoPlayer(nexaArg0: PlayerOptions): VideoPlayerSpec = VideoPlayerImpl(nexaArg0)"
        ));
    }

    #[test]
    fn native_class_event_contracts_use_the_compiler_callback_property_name() {
        let plan = validate(
            r#"
            native class VideoPlayer {
                event progress_changed(position: Float64, duration: Float64)
            }
            "#,
        );
        let swift = swift(&plan);
        assert!(swift.contains("var onProgressChanged: ((Double, Double) -> Void)? { get set }"));
        let kotlin = kotlin(&plan, "dev.example.video");
        assert!(kotlin.contains("var onProgressChanged: ((Double, Double) -> Unit)?"));
    }

    #[test]
    fn native_component_defaults_generate_platform_safe_literals() {
        let plan = validate(
            r#"
            native component VideoView {
                prop title: String = "cost $5\n\"today\""
                prop controls: Bool = true
                prop subtitle: String? = null
            }
            "#,
        );
        let swift = swift(&plan);
        assert!(swift.contains(r#"title: String = "cost $5\n\"today\""#));
        assert!(swift.contains("controls: Bool = true"));
        assert!(swift.contains("subtitle: String? = nil"));

        let kotlin = kotlin(&plan, "dev.example.video");
        assert!(kotlin.contains(r#"title: String = "cost \$5\n\"today\""#));
        assert!(kotlin.contains("controls: Boolean = true"));
        assert!(kotlin.contains("subtitle: String? = null"));
    }

    #[test]
    fn native_component_content_slots_generate_native_builder_parameters() {
        let plan = validate(
            r#"
            native component Container {
                content
                prop title: String
            }
            "#,
        );
        let swift = swift(&plan);
        assert!(swift.contains("public struct Container<Content: View>: View"));
        assert!(swift.contains("@ViewBuilder content: () -> Content"));
        assert!(swift.contains("content: content"));

        let kotlin = kotlin(&plan, "dev.example.components");
        assert!(kotlin.contains("content: @Composable () -> Unit"));
        assert!(kotlin.contains("content = content"));
    }

    #[test]
    fn typed_error_variants_and_throwing_contracts_survive_native_generation() {
        let plan = validate(
            r#"
            error PlayerError {
                invalidUrl,
                decodingFailed(message: String),
                maybe(message: String?)
            }
            native class VideoPlayer {
                init()
                async fn prepare(url: String, playbackRate: Float64) throws PlayerError
                async fn currentSource() -> Result<String, PlayerError>
            }
            "#,
        );
        let swift = swift(&plan);
        assert!(swift.contains("public enum PlayerError: Error"));
        assert!(swift.contains("case invalidUrl"));
        assert!(swift.contains("case decodingFailed(message: String)"));
        assert!(swift.contains(
            "func prepare(_ url: String, _ playbackRate: Double) async throws(PlayerError) -> Void"
        ));
        assert!(swift.contains("func currentSource() async throws(PlayerError) -> String"));

        let kotlin = kotlin(&plan, "dev.example.video");
        assert!(kotlin.contains("public sealed class PlayerError : Exception()"));
        assert!(kotlin.contains("public object invalidUrl : PlayerError()"));
        assert!(kotlin.contains(
            "public data class decodingFailed(override val message: String) : PlayerError()"
        ));
        assert!(
            kotlin
                .contains("public data class maybe(override val message: String?) : PlayerError()")
        );
        assert!(
            kotlin.contains(
                "@Throws(PlayerError::class)\nsuspend fun prepare(url: String, playbackRate: Double): Unit"
            )
        );
        assert!(
            kotlin.contains("@Throws(PlayerError::class)\nsuspend fun currentSource(): String")
        );
    }

    #[test]
    fn value_contracts_preserve_collection_optionality_and_mutability() {
        let plan = validate(
            r#"
            struct Playlist {
                titles: Array<String>,
                selected: Int32?
            }
            service Queue {
                readonly property current: Playlist?
                property volume: Float64
            }
            "#,
        );
        let swift = swift(&plan);
        assert!(swift.contains("public struct Playlist"));
        assert!(swift.contains("public let titles: [String]"));
        assert!(swift.contains("public let selected: Int32?"));
        assert!(swift.contains("var current: Playlist? { get }"));
        assert!(swift.contains("var volume: Double { get set }"));

        let kotlin = kotlin(&plan, "dev.example.queue");
        assert!(kotlin.contains("public data class Playlist"));
        assert!(kotlin.contains("val titles: List<String>"));
        assert!(kotlin.contains("val selected: Int?"));
        assert!(kotlin.contains("val current: Playlist?"));
        assert!(kotlin.contains("var volume: Double"));
    }
}

fn swift_type(ty: &BridgeType) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::Void) => "Void".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bool) => "Bool".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int8) => "Int8".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int16) => "Int16".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int32) => "Int32".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int64) => "Int64".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt8) => "UInt8".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt16) => "UInt16".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt32) => "UInt32".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt64) => "UInt64".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float32) => "Float".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float64) => "Double".to_owned(),
        BridgeType::Scalar(BridgeScalar::String) => "String".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bytes) => "Data".to_owned(),
        BridgeType::Named { name, .. } => name.clone(),
        BridgeType::Array(element) => format!("[{}]", swift_type(element)),
        BridgeType::Set(element) => format!("Set<{}>", swift_type(element)),
        BridgeType::Map(key, value) => {
            format!("[{}: {}]", swift_type(key), swift_type(value))
        }
        BridgeType::Pair(first, second) => {
            format!("({}, {})", swift_type(first), swift_type(second))
        }
        BridgeType::Triple(first, second, third) => format!(
            "({}, {}, {})",
            swift_type(first),
            swift_type(second),
            swift_type(third)
        ),
        BridgeType::Optional(inner) => format!("{}?", swift_type(inner)),
        BridgeType::Result { success, failure } => {
            format!("Result<{}, {failure}>", swift_type(success))
        }
    }
}

fn kotlin_type(ty: &BridgeType) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::Void) => "Unit".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int8) => "Byte".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int16) => "Short".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int32) => "Int".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int64) => "Long".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt8) => "UByte".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt16) => "UShort".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt32) => "UInt".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt64) => "ULong".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float32) => "Float".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float64) => "Double".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bool) => "Boolean".to_owned(),
        BridgeType::Scalar(BridgeScalar::String) => "String".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bytes) => "ByteArray".to_owned(),
        BridgeType::Named { name, .. } => name.clone(),
        BridgeType::Array(element) => format!("List<{}>", kotlin_type(element)),
        BridgeType::Set(element) => format!("Set<{}>", kotlin_type(element)),
        BridgeType::Map(key, value) => {
            format!("Map<{}, {}>", kotlin_type(key), kotlin_type(value))
        }
        BridgeType::Pair(first, second) => {
            format!("Pair<{}, {}>", kotlin_type(first), kotlin_type(second))
        }
        BridgeType::Triple(first, second, third) => format!(
            "Triple<{}, {}, {}>",
            kotlin_type(first),
            kotlin_type(second),
            kotlin_type(third)
        ),
        BridgeType::Optional(inner) => format!("{}?", kotlin_type(inner)),
        BridgeType::Result { success, failure } => {
            format!("Result<{}, {failure}>", kotlin_type(success))
        }
    }
}
