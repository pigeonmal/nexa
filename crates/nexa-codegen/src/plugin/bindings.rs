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
    if plan
        .interfaces
        .iter()
        .flat_map(|interface| &interface.methods)
        .any(|method| !method.row_type_parameters.is_empty())
    {
        out.push_str(
            "public struct NexaRowReader<Cell: Sendable, Failure: Error> {\n    public let value: (Int32) -> Cell\n    public let boolean: (Int32, String) throws(Failure) -> Bool\n    public let optionalBoolean: (Int32, String) throws(Failure) -> Bool?\n    public let integer: (Int32, String) throws(Failure) -> Int64\n    public let optionalInteger: (Int32, String) throws(Failure) -> Int64?\n    public let decimal: (Int32, String) throws(Failure) -> Double\n    public let optionalDecimal: (Int32, String) throws(Failure) -> Double?\n    public let text: (Int32, String) throws(Failure) -> String\n    public let optionalText: (Int32, String) throws(Failure) -> String?\n    public let bytes: (Int32, String) throws(Failure) -> Data\n    public let optionalBytes: (Int32, String) throws(Failure) -> Data?\n\n    public init(\n        value: @escaping (Int32) -> Cell,\n        boolean: @escaping (Int32, String) throws(Failure) -> Bool,\n        optionalBoolean: @escaping (Int32, String) throws(Failure) -> Bool?,\n        integer: @escaping (Int32, String) throws(Failure) -> Int64,\n        optionalInteger: @escaping (Int32, String) throws(Failure) -> Int64?,\n        decimal: @escaping (Int32, String) throws(Failure) -> Double,\n        optionalDecimal: @escaping (Int32, String) throws(Failure) -> Double?,\n        text: @escaping (Int32, String) throws(Failure) -> String,\n        optionalText: @escaping (Int32, String) throws(Failure) -> String?,\n        bytes: @escaping (Int32, String) throws(Failure) -> Data,\n        optionalBytes: @escaping (Int32, String) throws(Failure) -> Data?\n    ) {\n        self.value = value\n        self.boolean = boolean\n        self.optionalBoolean = optionalBoolean\n        self.integer = integer\n        self.optionalInteger = optionalInteger\n        self.decimal = decimal\n        self.optionalDecimal = optionalDecimal\n        self.text = text\n        self.optionalText = optionalText\n        self.bytes = bytes\n        self.optionalBytes = optionalBytes\n    }\n}\npublic typealias NexaRowMapper<Cell: Sendable, Row: Sendable, Failure: Error> = @Sendable ([String], @Sendable (String) -> Failure) throws(Failure) -> (@Sendable (NexaRowReader<Cell, Failure>) throws(Failure) -> Row)\n\n",
        );
    }
    let has_signals = plan.interfaces.iter().any(|interface| {
        interface
            .methods
            .iter()
            .any(|m| matches!(&m.return_type, BridgeType::Signal(_)))
            || interface
                .properties
                .iter()
                .any(|p| matches!(&p.ty, BridgeType::Signal(_)))
    });
    if has_signals {
        out.push_str(
            "@MainActor\npublic final class NexaSignal<Value: Sendable>: ObservableObject {\n    @Published public private(set) var value: Value\n    private var cancelSubscription: (@Sendable () -> Void)?\n    private let subscribeClosure: @MainActor (@escaping @Sendable (Value) -> Void) -> (@Sendable () -> Void)\n\n    public init(\n        initial: Value,\n        subscribe: @escaping @MainActor (@escaping @Sendable (Value) -> Void) -> (@Sendable () -> Void)\n    ) {\n        self.value = initial\n        self.subscribeClosure = subscribe\n        self.cancelSubscription = subscribe { [weak self] newValue in\n            Task { @MainActor in\n                self?.value = newValue\n            }\n        }\n    }\n\n    public var values: AsyncStream<Value> {\n        AsyncStream { continuation in\n            continuation.yield(self.value)\n            let cancel = self.subscribeClosure { newValue in\n                continuation.yield(newValue)\n            }\n            continuation.onTermination = { _ in cancel() }\n        }\n    }\n\n    public func dispose() {\n        cancelSubscription?()\n        cancelSubscription = nil\n    }\n\n    deinit {\n        cancelSubscription?()\n    }\n}\n\nextension NexaSignal: @preconcurrency AsyncSequence {\n    public typealias Element = Value\n    public typealias AsyncIterator = AsyncStream<Value>.Iterator\n\n    public func makeAsyncIterator() -> AsyncStream<Value>.Iterator {\n        values.makeAsyncIterator()\n    }\n}\n\n",
        );
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
    if plan.interfaces.iter().any(|interface| {
        interface.kind == InterfaceKind::NativeComponent && !interface.methods.is_empty()
    }) {
        out.push_str("import androidx.compose.runtime.DisposableEffect\n\n");
    }
    for ty in &plan.types {
        out.push_str(&kotlin_named_type(ty));
        out.push('\n');
    }
    if plan
        .interfaces
        .iter()
        .flat_map(|interface| &interface.methods)
        .any(|method| !method.row_type_parameters.is_empty())
    {
        out.push_str(
            "public class NexaRowReader<Cell>(\n    public val value: (Int) -> Cell,\n    public val boolean: (Int, String) -> Boolean,\n    public val optionalBoolean: (Int, String) -> Boolean?,\n    public val integer: (Int, String) -> Long,\n    public val optionalInteger: (Int, String) -> Long?,\n    public val decimal: (Int, String) -> Double,\n    public val optionalDecimal: (Int, String) -> Double?,\n    public val text: (Int, String) -> String,\n    public val optionalText: (Int, String) -> String?,\n    public val bytes: (Int, String) -> ByteArray,\n    public val optionalBytes: (Int, String) -> ByteArray?\n)\npublic typealias NexaRowMapper<Cell, Row, Failure> = (List<String>, (String) -> Failure) -> (NexaRowReader<Cell>) -> Row\n\n",
        );
    }
    let has_signals = plan.interfaces.iter().any(|interface| {
        interface
            .methods
            .iter()
            .any(|m| matches!(&m.return_type, BridgeType::Signal(_)))
            || interface
                .properties
                .iter()
                .any(|p| matches!(&p.ty, BridgeType::Signal(_)))
    });
    if has_signals {
        out.push_str(
            "public class NexaSignal<T>(\n    initial: T,\n    private val subscribe: ((T) -> Unit) -> (() -> Unit)\n) {\n    private val _flow = kotlinx.coroutines.flow.MutableStateFlow(initial)\n    public val flow: kotlinx.coroutines.flow.StateFlow<T> = _flow\n    public val value: T get() = _flow.value\n    private var cancelSubscription: (() -> Unit)?\n\n    init {\n        cancelSubscription = subscribe { newValue ->\n            _flow.value = newValue\n        }\n    }\n\n    public fun asStateFlow(): kotlinx.coroutines.flow.StateFlow<T> = flow\n\n    public fun dispose() {\n        cancelSubscription?.invoke()\n        cancelSubscription = null\n    }\n}\n\n",
        );
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
    let mut out = if interface.methods.is_empty() {
        String::new()
    } else {
        swift_component_ref(interface)
    };
    let generic = if interface.has_content_slot {
        "<Content: View>"
    } else {
        ""
    };
    out.push_str(&format!(
        "public struct {}{generic}: View {{\n",
        interface.name
    ));
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
    if !interface.methods.is_empty() {
        out.push_str(&format!("    public let ref: {}Ref?\n", interface.name));
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
    if !interface.methods.is_empty() {
        parameters.push(format!("ref: {}Ref? = nil", interface.name));
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
    if !interface.methods.is_empty() {
        out.push_str("        self.ref = ref\n");
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
    if !interface.methods.is_empty() {
        arguments.push("ref: ref".to_owned());
    }
    out.push_str(&arguments.join(", "));
    out.push_str(")");
    if !interface.methods.is_empty() {
        out.push_str(".onDisappear { ref?.detach() }");
    }
    out.push_str("\n    }\n}");
    out
}

fn kotlin_native_component(interface: &BridgeInterface) -> String {
    let mut prefix = if interface.methods.is_empty() {
        String::new()
    } else {
        kotlin_component_ref(interface)
    };
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
    if !interface.methods.is_empty() {
        parameters.push(format!("ref: {}Ref? = null", interface.name));
        arguments.push("ref = ref".to_owned());
    }
    prefix.push_str(&format!(
        "@Composable\npublic fun {}({}) {{\n",
        interface.name,
        parameters.join(", ")
    ));
    if !interface.methods.is_empty() {
        prefix.push_str(
            "    if (ref != null) DisposableEffect(ref) { onDispose { ref.detach() } }\n",
        );
    }
    prefix.push_str(&format!(
        "    {}Impl({})\n}}",
        interface.name,
        arguments.join(", ")
    ));
    prefix
}

fn swift_component_ref(interface: &BridgeInterface) -> String {
    let mut out = format!("@MainActor\npublic final class {}Ref {{\n", interface.name);
    for method in &interface.methods {
        let action = format!("{}Action", method.name);
        let argument_types = swift_ref_argument_types(method);
        let success = swift_type(method.success_type());
        let async_keyword = if method.is_async { " async" } else { "" };
        let throws_keyword = method
            .error_type()
            .map(|error| format!(" throws({error})"))
            .unwrap_or_default();
        out.push_str(&format!(
            "    private var {action}: (({argument_types}){async_keyword}{throws_keyword} -> {success})?\n"
        ));
    }
    out.push_str("\n    public init() {}\n\n");
    for method in &interface.methods {
        let action = format!("{}Action", method.name);
        let parameters = swift_ref_parameters(method);
        let argument_names = swift_ref_argument_names(method);
        let argument_types = swift_ref_argument_types(method);
        let success = swift_type(method.success_type());
        let failure = method.error_type().unwrap_or("Never");
        let async_keyword = if method.is_async { " async" } else { "" };
        let throws_keyword = method
            .error_type()
            .map(|error| format!(" throws({error})"))
            .unwrap_or_default();
        let attach = format!("attach{}", capitalize(&method.name));
        out.push_str(&format!(
            "    public func {attach}(_ action: @escaping ({argument_types}){async_keyword}{throws_keyword} -> {success}) {{ self.{action} = action }}\n"
        ));
        out.push_str(&format!(
            "    public func {}({parameters}){async_keyword} -> Result<{success}, NexaComponentRefError<{failure}>> {{\n",
            method.name
        ));
        out.push_str(&format!(
            "        guard let action = {action} else {{ return .failure(.notAttached) }}\n"
        ));
        let call = if argument_names.is_empty() {
            "action()".to_owned()
        } else {
            format!("action({})", argument_names.join(", "))
        };
        if method.error_type().is_some() {
            out.push_str("        do {\n");
            out.push_str(&format!(
                "            return .success(try {}{call})\n",
                if method.is_async { "await " } else { "" }
            ));
            out.push_str("        } catch let error { return .failure(.failure(error)) }\n");
        } else {
            out.push_str(&format!(
                "        return .success({}{call})\n",
                if method.is_async { "await " } else { "" },
            ));
        }
        out.push_str("    }\n\n");
    }
    out.push_str("    public func detach() {\n");
    for method in &interface.methods {
        out.push_str(&format!("        {}Action = nil\n", method.name));
    }
    out.push_str("    }\n}\n\n");
    out
}

fn kotlin_component_ref(interface: &BridgeInterface) -> String {
    let mut out = format!("public class {}Ref {{\n", interface.name);
    for method in &interface.methods {
        let action = format!("{}Action", method.name);
        let params = kotlin_ref_argument_types(method);
        let success = kotlin_type(method.success_type());
        let suspend = if method.is_async { "suspend " } else { "" };
        out.push_str(&format!(
            "    private var {action}: ({suspend}({params}) -> {success})? = null\n"
        ));
    }
    out.push_str("\n");
    for method in &interface.methods {
        let action = format!("{}Action", method.name);
        let params = kotlin_ref_parameters(method);
        let argument_types = kotlin_ref_argument_types(method);
        let argument_names = kotlin_ref_argument_names(method);
        let success = kotlin_type(method.success_type());
        let failure = method.error_type().unwrap_or("kotlin.Unit");
        let suspend = if method.is_async { "suspend " } else { "" };
        let attach = format!("attach{}", capitalize(&method.name));
        out.push_str(&format!(
            "    public fun {attach}(action: {suspend}({argument_types}) -> {success}) {{ this.{action} = action }}\n"
        ));
        out.push_str(&format!(
            "    public {suspend}fun {}({params}): dev.nexa.core.NexaResult<{success}, dev.nexa.core.NexaComponentRefError<{failure}>> {{\n",
            method.name
        ));
        out.push_str(&format!(
            "        val action = {action} ?: return dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)\n"
        ));
        let call = if argument_names.is_empty() {
            "action()".to_owned()
        } else {
            format!("action({})", argument_names.join(", "))
        };
        if let Some(failure_type) = method.error_type() {
            out.push_str(&format!(
                "        return try {{ dev.nexa.core.NexaResult.Success({call}) }} catch (failure: {failure_type}) {{ dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.Failure(failure)) }}\n"
            ));
        } else {
            out.push_str(&format!(
                "        return dev.nexa.core.NexaResult.Success({call})\n"
            ));
        }
        out.push_str("    }\n\n");
    }
    out.push_str("    public fun detach() {\n");
    for method in &interface.methods {
        out.push_str(&format!("        {}Action = null\n", method.name));
    }
    out.push_str("    }\n}\n\n");
    out
}

fn swift_ref_parameters(method: &BridgeMethod) -> String {
    method
        .parameters
        .iter()
        .map(|parameter| format!("_ {}: {}", parameter.name, swift_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn swift_ref_argument_types(method: &BridgeMethod) -> String {
    method
        .parameters
        .iter()
        .map(|parameter| swift_type(&parameter.ty))
        .collect::<Vec<_>>()
        .join(", ")
}

fn swift_ref_argument_names(method: &BridgeMethod) -> Vec<String> {
    method
        .parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect()
}

fn kotlin_ref_parameters(method: &BridgeMethod) -> String {
    method
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, kotlin_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn kotlin_ref_argument_types(method: &BridgeMethod) -> String {
    method
        .parameters
        .iter()
        .map(|parameter| kotlin_type(&parameter.ty))
        .collect::<Vec<_>>()
        .join(", ")
}

fn kotlin_ref_argument_names(method: &BridgeMethod) -> Vec<String> {
    method
        .parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect()
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
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
        BridgeTypeKind::Enum => {
            let cases = ty
                .cases
                .iter()
                .map(|variant| {
                    if variant.parameters.is_empty() {
                        format!("    case {}", variant.name)
                    } else {
                        let payload = variant
                            .parameters
                            .iter()
                            .map(|parameter| {
                                format!("{}: {}", parameter.name, swift_type(&parameter.ty))
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("    case {}({payload})", variant.name)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("public enum {}: Sendable {{\n{}\n}}", ty.name, cases)
        }
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
            let mut definition = format!(
                "public struct {}: Sendable {{\n{}\n\n    public init({}) {{\n{}\n    }}",
                ty.name,
                fields.join("\n"),
                parameters,
                assignments
            );
            if !ty.fields.is_empty() {
                let positional_parameters = ty
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| format!("_ nexaArg{index}: {}", swift_type(&field.ty)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let positional_assignments = ty
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| format!("        self.{} = nexaArg{index}", field.name))
                    .collect::<Vec<_>>()
                    .join("\n");
                definition.push_str(&format!(
                    "\n\n    public init({positional_parameters}) {{\n{positional_assignments}\n    }}"
                ));
            }
            definition.push_str("\n}");
            definition
        }
    }
}

fn kotlin_named_type(ty: &BridgeNamedType) -> String {
    match ty.kind {
        BridgeTypeKind::Enum
            if ty
                .cases
                .iter()
                .any(|variant| !variant.parameters.is_empty()) =>
        {
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
                                format!("val {}: {}", parameter.name, kotlin_type(&parameter.ty))
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!(
                            "    public data class {}({parameters}) : {}()",
                            variant.name, ty.name
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("public sealed class {} {{\n{}\n}}", ty.name, cases)
        }
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
    let mut parameters = swift_parameters(&method.parameters);
    parameters.push_str(&swift_codec_parameters(method));
    parameters.push_str(&swift_row_mapper_parameters(method));
    let return_type = method.success_type();
    let throws = method
        .error_type()
        .map(|error| format!(" throws({error})"))
        .unwrap_or_default();
    let generics = swift_generics(method);
    format!(
        "func {}{generics}({}){}{} -> {}",
        method.name,
        parameters,
        if method.is_async { " async" } else { "" },
        throws,
        swift_type(return_type)
    )
}

/// A generic method's value codecs, rendered as trailing closure parameters.
/// One closure per parameter that mentions a type parameter, plus one for the
/// return value, so a plugin implements the method once for every value type
/// the caller binds.
fn swift_codec_parameters(method: &BridgeMethod) -> String {
    let mut rendered = String::new();
    for (ty, decodes) in codec_types(method) {
        if decodes {
            rendered.push_str(&format!(
                ", _ decode: (NexaValueReader) -> {}?",
                swift_type(ty)
            ));
        } else {
            rendered.push_str(&format!(
                ", _ encode: ({}, NexaValueWriter) -> Void",
                swift_type(ty)
            ));
        }
    }
    rendered
}

/// Codec types in parameter order, then the return value. Each entry is the
/// concrete value type the closure handles and whether it decodes.
fn codec_types(method: &BridgeMethod) -> Vec<(&BridgeType, bool)> {
    let mut codecs: Vec<(&BridgeType, bool)> = method
        .parameters
        .iter()
        .map(|parameter| &parameter.ty)
        .filter(|ty| super::bridge_plan::contains_type_parameter(ty))
        .map(|ty| (ty, false))
        .collect();
    let success = method.success_type();
    if method.row_type_parameters.is_empty() && super::bridge_plan::contains_type_parameter(success)
    {
        codecs.push((
            match success {
                BridgeType::Optional(inner) => inner.as_ref(),
                other => other,
            },
            true,
        ));
    }
    codecs
}

fn swift_row_mapper_parameters(method: &BridgeMethod) -> String {
    let Some(row_value_type) = method.row_value_type.as_ref() else {
        return String::new();
    };
    let Some(row_parameter) = method.row_type_parameters.first() else {
        return String::new();
    };
    let error = method.error_type().or(method.row_error_type.as_deref());
    let Some(error) = error else {
        return String::new();
    };
    format!(
        ", _ mapRow: @escaping NexaRowMapper<{}, {}, {}>",
        swift_type(row_value_type),
        row_parameter,
        error
    )
}

fn swift_generics(method: &BridgeMethod) -> String {
    if method.type_parameters.is_empty() {
        return String::new();
    }
    let mut hashable = std::collections::HashSet::new();
    for parameter in &method.parameters {
        collect_hashable_type_parameters(&parameter.ty, &mut hashable);
    }
    collect_hashable_type_parameters(method.success_type(), &mut hashable);
    let parameters = method
        .type_parameters
        .iter()
        .map(|parameter| {
            if method.row_type_parameters.contains(parameter) {
                format!("{parameter}: Copyable & Sendable")
            } else if hashable.contains(parameter) {
                format!("{parameter}: Copyable & Hashable")
            } else {
                format!("{parameter}: Copyable")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{parameters}>")
}

fn collect_hashable_type_parameters(
    ty: &BridgeType,
    hashable: &mut std::collections::HashSet<String>,
) {
    match ty {
        BridgeType::Set(inner) => collect_all_type_parameters(inner, hashable),
        BridgeType::Map(key, value) => {
            collect_all_type_parameters(key, hashable);
            collect_hashable_type_parameters(value, hashable);
        }
        BridgeType::Array(inner) | BridgeType::Signal(inner) | BridgeType::Optional(inner) => {
            collect_hashable_type_parameters(inner, hashable)
        }
        BridgeType::Pair(first, second) => {
            collect_hashable_type_parameters(first, hashable);
            collect_hashable_type_parameters(second, hashable);
        }
        BridgeType::Triple(first, second, third) => {
            collect_hashable_type_parameters(first, hashable);
            collect_hashable_type_parameters(second, hashable);
            collect_hashable_type_parameters(third, hashable);
        }
        BridgeType::Result { success, .. } => {
            collect_hashable_type_parameters(success, hashable);
        }
        BridgeType::TypeParameter(_) | BridgeType::Scalar(_) | BridgeType::Named { .. } => {}
    }
}

fn collect_all_type_parameters(
    ty: &BridgeType,
    parameters: &mut std::collections::HashSet<String>,
) {
    match ty {
        BridgeType::TypeParameter(name) => {
            parameters.insert(name.clone());
        }
        BridgeType::Array(inner)
        | BridgeType::Set(inner)
        | BridgeType::Signal(inner)
        | BridgeType::Optional(inner) => {
            collect_all_type_parameters(inner, parameters);
        }
        BridgeType::Map(key, value) | BridgeType::Pair(key, value) => {
            collect_all_type_parameters(key, parameters);
            collect_all_type_parameters(value, parameters);
        }
        BridgeType::Triple(first, second, third) => {
            collect_all_type_parameters(first, parameters);
            collect_all_type_parameters(second, parameters);
            collect_all_type_parameters(third, parameters);
        }
        BridgeType::Result { success, .. } => {
            collect_all_type_parameters(success, parameters);
        }
        BridgeType::Scalar(_) | BridgeType::Named { .. } => {}
    }
}

fn kotlin_method(method: &BridgeMethod) -> String {
    let mut parameters = kotlin_parameters(&method.parameters);
    parameters.push_str(&kotlin_codec_parameters(method));
    parameters.push_str(&kotlin_row_mapper_parameters(method));
    let return_type = method.success_type();
    let annotation = method
        .error_type()
        .map(|error| format!("@Throws({error}::class)\n"))
        .unwrap_or_default();
    let generics = if method.type_parameters.is_empty() {
        String::new()
    } else {
        format!("<{}> ", method.type_parameters.join(", "))
    };
    format!(
        "{}{}fun {generics}{}({}): {}",
        annotation,
        if method.is_async { "suspend " } else { "" },
        method.name,
        parameters,
        kotlin_type(return_type)
    )
}

fn kotlin_row_mapper_parameters(method: &BridgeMethod) -> String {
    let Some(row_value_type) = method.row_value_type.as_ref() else {
        return String::new();
    };
    let Some(row_parameter) = method.row_type_parameters.first() else {
        return String::new();
    };
    let error = method
        .error_type()
        .or(method.row_error_type.as_deref())
        .unwrap_or("Throwable");
    format!(
        ", mapRow: NexaRowMapper<{}, {}, {}>",
        kotlin_type(row_value_type),
        row_parameter,
        error
    )
}

/// The Kotlin mirror of [`swift_codec_parameters`]: a writer takes the value
/// and the writer, a reader takes only the reader and returns the value.
fn kotlin_codec_parameters(method: &BridgeMethod) -> String {
    // A contract is generated into the *plugin's* package, so the codec types
    // are named through the fixed core package every generated app carries.
    let reader = format!("{}.NexaValueReader", crate::value::KOTLIN_CORE_PACKAGE);
    let writer = format!("{}.NexaValueWriter", crate::value::KOTLIN_CORE_PACKAGE);
    let read_result = format!("{}.NexaValueReadResult", crate::value::KOTLIN_CORE_PACKAGE);
    let mut rendered = String::new();
    for (index, (ty, decodes)) in codec_types(method).into_iter().enumerate() {
        let parameter = if decodes { "decode" } else { "encode" };
        if decodes {
            rendered.push_str(&format!(
                ", {parameter}{index}: ({reader}) -> {read_result}<{}>",
                kotlin_type(ty)
            ));
        } else {
            rendered.push_str(&format!(
                ", {parameter}{index}: ({}, {writer}) -> Unit",
                kotlin_type(ty)
            ));
        }
    }
    rendered
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

    fn validate_swift_and_kotlin(source: &str) -> BridgePlan {
        let idl = nexa_plugin_idl::parse(source).expect("IDL should parse");
        BridgePlan::validate_swift_contract(&idl)
            .expect("Swift and Kotlin contract should validate")
    }

    /// A generic method's contract carries one value codec per parameter or
    /// return that mentions a type parameter, so a plugin implements the method
    /// once instead of once per value type.
    #[test]
    fn generic_methods_carry_a_value_codec_per_bound_type() {
        let plan = validate_swift_and_kotlin(
            r#"
            native class Store {
                init()
                fn setObject<T>(key: String, value: T) -> Bool
                fn getObject<T>(key: String) -> T?
                fn setSet<T>(key: String, values: Set<T>) -> Bool
                fn getSet<T>(key: String) -> Set<T>?
                fn setMap<K, V>(key: String, values: Map<K, V>) -> Bool
                fn getMap<K, V>(key: String) -> Map<K, V>?
                fn dispose()
            }
            "#,
        );
        let swift = swift(&plan);
        assert!(swift.contains(
            "func setObject<T: Copyable>(_ key: String, _ value: T, _ encode: (T, NexaValueWriter) -> Void) -> Bool"
        ));
        assert!(swift.contains(
            "func getObject<T: Copyable>(_ key: String, _ decode: (NexaValueReader) -> T?) -> T?"
        ));
        assert!(swift.contains(
            "func getSet<T: Copyable & Hashable>(_ key: String, _ decode: (NexaValueReader) -> Set<T>?) -> Set<T>?"
        ));
        // The map codec covers the whole map, not its two type parameters.
        assert!(
            swift.contains("func getMap<K: Copyable & Hashable, V: Copyable>(_ key: String, _ decode: (NexaValueReader) -> [K: V]?) -> [K: V]?")
        );

        let kotlin = kotlin(&plan, "dev.example.store");
        assert!(kotlin.contains("fun <T> setObject(key: String, value: T, encode0: (T, dev.nexa.core.NexaValueWriter) -> Unit): Boolean"));
        assert!(kotlin.contains(
            "fun <T> getObject(key: String, decode0: (dev.nexa.core.NexaValueReader) -> dev.nexa.core.NexaValueReadResult<T>): T?"
        ));
        assert!(kotlin.contains(
            "fun <K, V> getMap(key: String, decode0: (dev.nexa.core.NexaValueReader) -> dev.nexa.core.NexaValueReadResult<Map<K, V>>): Map<K, V>?"
        ));
    }

    #[test]
    fn typed_plugin_row_reader_keeps_direct_getters_and_a_dynamic_value_fallback() {
        let plan = validate_swift_and_kotlin(
            r#"
            enum Value { nullValue }
            error Failure { invalidValue(message: String) }
            native class Database {
                init()
                async fn query<T: Row>(sql: String, parameters: Array<Value>) -> Array<T> throws Failure rowFailure invalidValue
            }
            "#,
        );

        let swift = swift(&plan);
        assert!(swift.contains("public let value: (Int32) -> Cell"));
        assert!(
            swift.contains("public let optionalInteger: (Int32, String) throws(Failure) -> Int64?")
        );
        assert!(swift.contains("NexaRowMapper<Value, T, Failure>"));
        assert!(swift.contains("_ mapRow: @escaping NexaRowMapper<Value, T, Failure>"));
        assert!(swift.contains("NexaRowReader<Cell, Failure>"));

        let kotlin = kotlin(&plan, "dev.nexa.sqlite");
        assert!(kotlin.contains("public val value: (Int) -> Cell"));
        assert!(kotlin.contains("public val optionalInteger: (Int, String) -> Long?"));
        assert!(kotlin.contains("NexaRowMapper<Value, T, Failure>"));
    }

    #[test]
    fn signal_return_type_generates_nexa_signal_contract_and_wrapper() {
        let plan = validate_swift_and_kotlin(
            r#"
            enum Value { nullValue }
            error Failure { queryFailed(message: String) }
            native class Database {
                init()
                fn observeQuery<T: Row>(sql: String, parameters: Array<Value>) -> Signal<Array<T>> throws Failure rowFailure queryFailed
            }
            "#,
        );

        let swift = swift(&plan);
        assert!(swift.contains("public final class NexaSignal<Value: Sendable>: ObservableObject"));
        assert!(swift.contains("func observeQuery<T: Copyable & Sendable>(_ sql: String, _ parameters: [Value], _ mapRow: @escaping NexaRowMapper<Value, T, Failure>) throws(Failure) -> NexaSignal<[T]>"));

        let kotlin = kotlin(&plan, "dev.nexa.sqlite");
        assert!(kotlin.contains("public class NexaSignal<T>("));
        assert!(kotlin.contains("fun <T> observeQuery(sql: String, parameters: List<Value>, mapRow: NexaRowMapper<Value, T, Failure>): NexaSignal<List<T>>"));
    }

    #[test]
    fn native_component_methods_generate_direct_typed_refs() {
        let plan = validate_swift_and_kotlin(
            r#"
            error PlayerFailure { unavailable }
            native component VideoView {
                prop source: String
                fn seek(position: Float64) -> String throws PlayerFailure
                async fn currentSource() -> String throws PlayerFailure
            }
            "#,
        );

        let swift = swift(&plan);
        assert!(swift.contains("public final class VideoViewRef"));
        assert!(swift.contains(
            "public func attachSeek(_ action: @escaping (Double) throws(PlayerFailure) -> String)"
        ));
        assert!(swift.contains("public func seek(_ position: Double) -> Result<String, NexaComponentRefError<PlayerFailure>>"));
        assert!(swift.contains("public func attachCurrentSource(_ action: @escaping () async throws(PlayerFailure) -> String)"));
        assert!(swift.contains("public func currentSource() async -> Result<String, NexaComponentRefError<PlayerFailure>>"));
        assert!(swift.contains("VideoViewImpl(source: source, ref: ref)"));
        assert!(swift.contains(".onDisappear { ref?.detach() }"));

        let kotlin = kotlin(&plan, "dev.example.video");
        assert!(kotlin.contains("public class VideoViewRef"));
        assert!(kotlin.contains("public fun attachSeek(action: (Double) -> String)"));
        assert!(kotlin.contains("public fun seek(position: Double): dev.nexa.core.NexaResult<String, dev.nexa.core.NexaComponentRefError<PlayerFailure>>"));
        assert!(kotlin.contains("public suspend fun currentSource(): dev.nexa.core.NexaResult<String, dev.nexa.core.NexaComponentRefError<PlayerFailure>>"));
        assert!(kotlin.contains("DisposableEffect(ref) { onDispose { ref.detach() } }"));
        assert!(kotlin.contains("VideoViewImpl(source = source, ref = ref)"));
    }

    /// The C++ bridge has no spelling for a per-call-site value type, so a
    /// plugin that reaches for it on that surface gets a clear error instead of
    /// a contract that cannot be implemented.
    #[test]
    fn the_cpp_bridge_rejects_value_type_parameters() {
        let idl = nexa_plugin_idl::parse(
            r#"
            native class Store {
                init()
                fn setObject<T>(key: String, value: T) -> Bool
                fn dispose()
            }
            "#,
        )
        .expect("IDL should parse");
        let error =
            BridgePlan::validate_contract(&idl).expect_err("generics are Swift and Kotlin only");
        assert!(
            error.contains("does not support value type parameters"),
            "unexpected diagnostic: {error}"
        );
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
        assert!(swift.contains("public init(_ nexaArg0: [String], _ nexaArg1: Int32?)"));
        assert!(swift.contains("self.titles = nexaArg0"));
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
    ty.swift_type()
}

fn kotlin_type(ty: &BridgeType) -> String {
    ty.kotlin_type()
}
