import Foundation
import CoreFoundation
import UIKit

/// Reactive state store and expression evaluation engine for the Nexa dev runtime.
struct NexaDevNativeEventSubscription {
    let receiver: AnyObject
    let property: String
    var actions: [Any]
    var parameters: [String]
    let scope: String
    let locals: [String: Any]
}

struct NexaDevNetworkStatusSubscription {
    let id: String
    var parameter: String
    var actions: [Any]
    let scope: String
    let locals: [String: Any]
}

@MainActor
final class NexaDevStateStore: ObservableObject {
    @Published var revision = 0
    @Published private(set) var appLifecycleEpoch = 0
    @Published var navigationPath: [NexaDevRoute] = []
    @Published private(set) var focusedFieldKey: String?
    var values: [String: Any] = [:]
    var typeSignatures: [String: String] = [:]
    var functions: [String: [String: Any]] = [:]
    var structs: [String: [[String: Any]]] = [:]
    var enumCases: [String: [String]] = [:]
    var routeArguments: [String: (screen: String, values: [String: Any], signature: String)] = [:]
    var navigationRoot: String?
    var activeFunctions = Set<String>()
    var focusBindings: [String: (scope: String, state: String?)] = [:]
    var nativeEventSubscriptions: [NexaDevNativeEventSubscription] = []
    var networkStatusSubscriptions: [NexaDevNetworkStatusSubscription] = []
    var foregroundTasks: [String: Task<Void, Never>] = [:]
    var activeScreenParameters: [String: [String: Any]] = [:]
    var pendingPluginFailure: NexaDevPluginFailure?
    var hasInstalledModule = false

    func install(module: [String: Any]) {
        let isHotReplacement = hasInstalledModule
        if isHotReplacement { clearNativeTasks() }
        let screens = module["screens"] as? [[String: Any]] ?? []
        let appStates = module["states"] as? [[String: Any]] ?? []
        functions = Dictionary(
            (module["functions"] as? [[String: Any]] ?? []).compactMap { function in
                guard let name = function["name"] as? String else { return nil }
                return (name, function)
            },
            uniquingKeysWith: { _, newest in newest }
        )
        structs = Dictionary(
            (module["structs"] as? [[String: Any]] ?? []).compactMap { declaration in
                guard let name = declaration["name"] as? String else { return nil }
                return (name, declaration["fields"] as? [[String: Any]] ?? [])
            },
            uniquingKeysWith: { _, newest in newest }
        )
        enumCases = Dictionary(
            (module["enums"] as? [[String: Any]] ?? []).compactMap { declaration in
                guard let name = declaration["name"] as? String else { return nil }
                return (name, declaration["cases"] as? [String] ?? [])
            },
            uniquingKeysWith: { _, newest in newest }
        )
        var nextValues: [String: Any] = [:]
        var nextTypes: [String: String] = [:]
        var initialLocals: [String: Any] = [:]
        let declarations = appStates.map { ("app", $0) } + screens.flatMap { screen in
            guard let name = screen["name"] as? String else { return [(String, [String: Any])]() }
            return (screen["states"] as? [[String: Any]] ?? []).map { ("screen/\(name)", $0) }
        }
        for (scope, state) in declarations {
            guard let name = state["name"] as? String else { continue }
            let identity = "\(scope)/state/\(name)"
            let signature = Self.canonicalJSON(state["ty"])
            nextTypes[identity] = signature
            let resolvedValue: Any
            if typeSignatures[identity] == signature, let value = values[identity] {
                resolvedValue = value
            } else if let initial = state["initial"] {
                resolvedValue = evaluate(initial, locals: initialLocals, scope: scope)
            } else {
                continue
            }
            nextValues[identity] = resolvedValue
            initialLocals[name] = resolvedValue
        }
        values = nextValues
        typeSignatures = nextTypes
        if isHotReplacement {
            let currentScreens = Set(screens.compactMap { $0["name"] as? String })
            activeScreenParameters = activeScreenParameters.filter { scope, _ in
                currentScreens.contains(String(scope.split(separator: "/").last ?? ""))
            }
            refreshNativeEventSubscriptions(module: module)
        }
        var nextFocusBindings: [String: (scope: String, state: String?)] = [:]
        Self.collectFocusBindings(module["body"] as? [Any] ?? [], scope: "app", into: &nextFocusBindings)
        for screen in screens {
            guard let name = screen["name"] as? String else { continue }
            Self.collectFocusBindings(
                screen["body"] as? [Any] ?? [],
                scope: "screen/\(name)",
                into: &nextFocusBindings
            )
        }
        focusBindings = nextFocusBindings
        if let focusedFieldKey, nextFocusBindings[focusedFieldKey] == nil {
            self.focusedFieldKey = nil
        }
        if self.focusedFieldKey == nil {
            self.focusedFieldKey = nextFocusBindings.first { _, binding in
                guard let state = binding.state else { return false }
                let identity = "\(binding.scope)/state/\(state)"
                return values[identity] != nil
            }?.key
        }
        let rootScreen = screens.first?["name"] as? String
        let screensSignature = Self.canonicalJSON(screens.map { $0["name"] ?? NSNull() })
        if navigationRoot != rootScreen || typeSignatures["__screens__"] != screensSignature {
            navigationRoot = rootScreen
            typeSignatures["__screens__"] = screensSignature
            navigationPath = []
        }
        revision += 1
        if !hasInstalledModule {
            hasInstalledModule = true
            appLifecycleEpoch += 1
        }
    }

    func hotRestart(module: [String: Any]) {
        clearNativeTasks()
        clearNativeEventSubscriptions()
        activeScreenParameters.removeAll(keepingCapacity: true)
        let screens = module["screens"] as? [[String: Any]] ?? []
        let appStates = module["states"] as? [[String: Any]] ?? []
        var nextValues: [String: Any] = [:]
        var initialLocals: [String: Any] = [:]
        for state in appStates {
            guard let name = state["name"] as? String, let initial = state["initial"] else { continue }
            let value = evaluate(initial, locals: initialLocals, scope: "app")
            nextValues["app/state/\(name)"] = value
            initialLocals[name] = value
        }
        for screen in screens {
            guard let screenName = screen["name"] as? String else { continue }
            for state in screen["states"] as? [[String: Any]] ?? [] {
                guard let name = state["name"] as? String, let initial = state["initial"] else { continue }
                let value = evaluate(initial, locals: initialLocals, scope: "screen/\(screenName)")
                nextValues["screen/\(screenName)/state/\(name)"] = value
                initialLocals[name] = value
            }
        }
        values = nextValues
        navigationPath = []
        focusedFieldKey = focusBindings.first?.key
        revision += 1
        appLifecycleEpoch += 1
    }

    func value(_ name: String, scope: String = "app") -> Any {
        values["\(scope)/state/\(name)"] ?? values["app/state/\(name)"] ?? NSNull()
    }

    func locals(scope: String, parameters: [String: Any]) -> [String: Any] {
        parameters
    }

    func setValue(_ name: String, value: Any, scope: String) {
        let key = "\(scope)/state/\(name)"
        if values[key] != nil || !scope.starts(with: "screen/") {
            values[key] = value
        } else {
            values["app/state/\(name)"] = value
        }
    }

    func focusChanged(to nextIdentity: String?) {
        focusedFieldKey = nextIdentity
    }

    func screenDidAppear(scope: String, parameters: [String: Any]) {
        activeScreenParameters[scope] = parameters
    }

    func screenDidDisappear(scope: String) {
        activeScreenParameters.removeValue(forKey: scope)
        clearNativeEventSubscriptions(scope: scope)
        clearNativeTasks(scope: scope)
    }

    static func collectFocusBindings(
        _ nodes: [Any],
        scope: String,
        into bindings: inout [String: (scope: String, state: String?)]
    ) {
        for node in nodes {
            guard let tagged = node as? [String: Any], let (kind, payload) = tagged.first else { continue }
            let fields = payload as? [String: Any] ?? [:]
            if kind == "TextInput" {
                let stateName = fields["state"] as? String
                let identity = "\(scope)/input/\(stateName ?? UUID().uuidString)"
                bindings[identity] = (scope: scope, state: stateName)
            }
            for childListKey in ["children", "then_body", "else_body"] {
                if let children = fields[childListKey] as? [Any] {
                    collectFocusBindings(children, scope: scope, into: &bindings)
                }
            }
            if let cases = fields["cases"] as? [[String: Any]] {
                for caseBranch in cases {
                    if let children = caseBranch["body"] as? [Any] {
                        collectFocusBindings(children, scope: scope, into: &bindings)
                    }
                }
            }
        }
    }

    func evaluate(_ expression: Any, locals: [String: Any], scope: String = "app") -> Any {
        if let unit = expression as? String {
            switch unit {
            case "IsRegularWidth": return UIScreen.main.bounds.width >= 600
            case "IsCompactWidth": return UIScreen.main.bounds.width < 600
            case "IsRegularHeight": return UIScreen.main.bounds.height >= 600
            case "IsCompactHeight": return UIScreen.main.bounds.height < 600
            default: return NSNull()
            }
        }
        guard let tagged = expression as? [String: Any], let (kind, payload) = tagged.first else {
            return NSNull()
        }
        switch kind {
        case "String": return payload as? String ?? ""
        case "Bool": return payload as? Bool ?? false
        case "Array":
            return (payload as? [Any] ?? []).map { evaluate($0, locals: locals, scope: scope) }
        case "Set":
            return Set((payload as? [Any] ?? []).map {
                NexaDevHashableValue(evaluate($0, locals: locals, scope: scope))
            })
        case "Map":
            var result: [String: Any] = [:]
            for pair in payload as? [[Any]] ?? [] where pair.count >= 2 {
                result[stringify(evaluate(pair[0], locals: locals, scope: scope))] = evaluate(pair[1], locals: locals, scope: scope)
            }
            return result
        case "Pair":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 2 else { return [NSNull(), NSNull()] }
            return [evaluate(parts[0], locals: locals, scope: scope), evaluate(parts[1], locals: locals, scope: scope)]
        case "Triple":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 3 else { return [NSNull(), NSNull(), NSNull()] }
            return [evaluate(parts[0], locals: locals, scope: scope), evaluate(parts[1], locals: locals, scope: scope), evaluate(parts[2], locals: locals, scope: scope)]
        case "Number":
            let fields = payload as? [String: Any] ?? [:]
            let raw = fields["raw"] as? String ?? "0"
            switch fields["ty"] as? String {
            case "Int8": return Int8(raw) ?? 0
            case "Int16": return Int16(raw) ?? 0
            case "Int32": return Int32(raw) ?? 0
            case "Int64": return Int64(raw) ?? 0
            case "UInt8": return UInt8(raw) ?? 0
            case "UInt16": return UInt16(raw) ?? 0
            case "UInt32": return UInt32(raw) ?? 0
            case "UInt64": return UInt64(raw) ?? 0
            case "Float32": return Float(raw) ?? 0
            case "Float64": return Double(raw) ?? 0
            default: return raw.contains(".") ? (Double(raw) ?? 0) as Any : (Int64(raw) ?? 0) as Any
            }
        case "EnumValue", "PluginEnumValue":
            return (payload as? [String: Any])?["case_name"] as? String ?? ""
        case "State":
            let parts = payload as? [Any] ?? []
            guard let name = parts.first as? String else { return NSNull() }
            return locals[name] ?? value(name, scope: scope)
        case "Interpolation":
            return (payload as? [Any] ?? []).map { part -> String in
                guard let taggedPart = part as? [String: Any], let (partKind, partValue) = taggedPart.first else { return "" }
                if partKind == "Literal" { return partValue as? String ?? "" }
                if partKind == "Value" { return stringify(evaluate(partValue, locals: locals, scope: scope)) }
                return ""
            }.joined()
        case "Add":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 2 else { return 0 }
            let left = evaluate(parts[0], locals: locals, scope: scope)
            let right = evaluate(parts[1], locals: locals, scope: scope)
            if let leftString = left as? String, let rightString = right as? String {
                return leftString + rightString
            }
            let sum = number(left) + number(right)
            return sum.rounded() == sum ? Int64(sum) as Any : sum as Any
        case "Concat":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 2 else { return "" }
            return stringify(evaluate(parts[0], locals: locals, scope: scope))
                + stringify(evaluate(parts[1], locals: locals, scope: scope))
        case "Arithmetic":
            let fields = payload as? [String: Any] ?? [:]
            guard let left = fields["left"], let right = fields["right"] else { return NSNull() }
            return arithmetic(
                fields["op"] as? String ?? "",
                evaluate(left, locals: locals, scope: scope),
                evaluate(right, locals: locals, scope: scope),
                type: fields["ty"] as? String ?? "Int32"
            )
        case "Negate":
            let fields = payload as? [String: Any] ?? [:]
            guard let value = fields["value"] else { return NSNull() }
            let type = fields["ty"] as? String ?? "Int32"
            return arithmetic("Subtract", numericZero(type: type), evaluate(value, locals: locals, scope: scope), type: type)
        case "Not": return !truthy(evaluate(payload, locals: locals, scope: scope))
        case "Binary":
            guard let binary = payload as? [String: Any],
                  let op = binary["op"] as? String,
                  let leftExpr = binary["left"], let rightExpr = binary["right"]
            else { return false }
            return compare(
                op,
                evaluate(leftExpr, locals: locals, scope: scope),
                evaluate(rightExpr, locals: locals, scope: scope)
            )
        case "Conditional":
            let fields = payload as? [String: Any] ?? [:]
            guard let condition = fields["condition"] else { return NSNull() }
            let branch = truthy(evaluate(condition, locals: locals, scope: scope))
                ? fields["then_value"]
                : fields["else_value"]
            return branch.map { evaluate($0, locals: locals, scope: scope) } ?? NSNull()
        case "Contains":
            let fields = payload as? [String: Any] ?? [:]
            guard let value = fields["value"], let collection = fields["collection"] else { return false }
            return contains(evaluate(value, locals: locals, scope: scope), in: evaluate(collection, locals: locals, scope: scope))
        case "CollectionTransform":
            let fields = payload as? [String: Any] ?? [:]
            guard let collectionExpression = fields["collection"],
                  let closure = fields["closure"] as? [String: Any],
                  let closurePayload = closure["Closure"] as? [String: Any]
            else { return [] }
            let collectionValue = evaluate(collectionExpression, locals: locals, scope: scope)
            let setItems = NexaDevValueCodec.asSet(collectionValue)?
                .sorted { $0.stableOrderKey < $1.stableOrderKey }
                .map(\.value) ?? []
            let collection = collectionValue as? [Any] ?? setItems
            let parameters = closurePayload["parameters"] as? [String] ?? []
            let body = closurePayload["body"] ?? NSNull()
            func apply(_ item: Any, _ accumulator: Any? = nil) -> Any {
                var closureLocals = locals
                if parameters.count > 1, let accumulator {
                    closureLocals[parameters[0]] = accumulator
                    closureLocals[parameters[1]] = item
                } else if let parameter = parameters.first {
                    closureLocals[parameter] = item
                }
                return evaluate(body, locals: closureLocals, scope: scope)
            }
            switch fields["operation"] as? String {
            case "Map": return collection.map { apply($0) }
            case "Filter": return collection.filter { truthy(apply($0)) }
            case "Reduce":
                var accumulator = fields["initial"].map { evaluate($0, locals: locals, scope: scope) } ?? 0
                for item in collection { accumulator = apply(item, accumulator) }
                return accumulator
            default: return []
            }
        case "CollectionUtility":
            let fields = payload as? [String: Any] ?? [:]
            guard let collectionExpression = fields["collection"],
                  let values = evaluate(collectionExpression, locals: locals, scope: scope) as? [Any]
            else { return [] }
            switch fields["operation"] as? String {
            case "Random": return values.randomElement().map { $0 as Any } ?? NSNull()
            case "First": return values.first.map { $0 as Any } ?? NSNull()
            case "Last": return values.last.map { $0 as Any } ?? NSNull()
            case "Shuffled": return values.shuffled()
            case "Reverse": return Array(values.reversed())
            case "Slice":
                guard let startExpression = fields["start"], let endExpression = fields["end"] else { return [] }
                let start = Int(number(evaluate(startExpression, locals: locals, scope: scope)))
                let end = Int(number(evaluate(endExpression, locals: locals, scope: scope)))
                let stop = end + ((fields["inclusive"] as? Bool == true) ? 1 : 0)
                return Array(values[start..<stop])
            default: return []
            }
        case "Closure": return ["Closure": payload]
        case "Index":
            let fields = payload as? [String: Any] ?? [:]
            guard let collection = fields["collection"].map({ evaluate($0, locals: locals, scope: scope) }),
                  let indexExpression = fields["index"] else { return NSNull() }
            let index = (evaluate(indexExpression, locals: locals, scope: scope) as? NSNumber)?.intValue ?? -1
            if let values = collection as? [Any], values.indices.contains(index) { return values[index] }
            if let values = collection as? [String: Any] {
                return values[stringify(evaluate(indexExpression, locals: locals, scope: scope))] ?? NSNull()
            }
            return NSNull()
        case "Member":
            let fields = payload as? [String: Any] ?? [:]
            guard let base = fields["base"], let name = fields["name"] as? String else { return NSNull() }
            let value = evaluate(base, locals: locals, scope: scope)
            if let memberKind = fields["kind"] as? [String: Any],
               let property = memberKind["PluginField"] as? String {
                let result = NexaDevPluginBridge.readInstanceProperty(receiver: value, property: property)
                if result.0 { return result.1 }
            }
            if name == "count" {
                if let values = value as? [Any] { return Int32(values.count) }
                if let values = value as? [String: Any] { return Int32(values.count) }
            }
            if name == "isEmpty" {
                if let values = value as? [Any] { return values.isEmpty }
                if let values = value as? [String: Any] { return values.isEmpty }
            }
            if let object = value as? [String: Any] { return object[name] ?? NSNull() }
            if let pair = value as? [Any] {
                let position = switch name {
                case "first": 0
                case "second": 1
                case "third": 2
                default: Int(name) ?? -1
                }
                if pair.indices.contains(position) { return pair[position] }
            }
            return NSNull()
        case "Range":
            let fields = payload as? [String: Any] ?? [:]
            guard let startExpr = fields["start"], let endExpr = fields["end"] else { return [Int]() }
            let start = number(evaluate(startExpr, locals: locals, scope: scope))
            let end = number(evaluate(endExpr, locals: locals, scope: scope))
            let step = max(1, abs(Int(fields["step"].map { number(evaluate($0, locals: locals, scope: scope)) } ?? 1)))
            let inclusive = fields["inclusive"] as? Bool ?? false
            guard start.isFinite, end.isFinite, abs(end - start) < 100_000 else { return [Int]() }
            let boundary = Int(end) + ((inclusive && end >= start) ? 1 : 0)
            return Array(stride(from: Int(start), to: boundary, by: step))
        case "Null": return NSNull()
        // A byte buffer crosses the hot-reload boundary as base64 text, which
        // is the one representation both dev runtimes agree on.
        case "BytesFromText":
            let text = evaluate((payload as? [String: Any])?["text"] ?? NSNull(), locals: locals, scope: scope)
            return Data((text as? String ?? "").utf8).base64EncodedString()
        case "BytesFromArray":
            let values = evaluate((payload as? [String: Any])?["values"] ?? NSNull(), locals: locals, scope: scope)
            let bytes = (values as? [Any] ?? []).map { value in
                UInt8(truncatingIfNeeded: Int64(stringify(value)) ?? 0)
            }
            return Data(bytes).base64EncodedString()
        case "BytesCount":
            let bytes = evaluate((payload as? [String: Any])?["bytes"] ?? NSNull(), locals: locals, scope: scope)
            guard let text = bytes as? String, let data = Data(base64Encoded: text) else { return 0 }
            return data.count
        case "TimeCall":
            return evaluateTimeCall(payload as? [String: Any], locals: locals, scope: scope)
        case "LogCall":
            let logCall = payload as? [String: Any] ?? [:]
            let message = stringify(logCall["message"].map { evaluate($0, locals: locals, scope: scope) } ?? NSNull())
            let severity = logCall["method"] as? String ?? "Info"
            NSLog("[Nexa][%@] %@", severity.uppercased(), message)
            return NSNull()
        case "Coalesce":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 2 else { return NSNull() }
            let value = evaluate(parts[0], locals: locals, scope: scope)
            return value is NSNull ? evaluate(parts[1], locals: locals, scope: scope) : value
        case "ResultOk", "ResultErr":
            let key = kind == "ResultOk" ? "value" : "error"
            let field = payload as? [String: Any] ?? [:]
            return [kind == "ResultOk" ? "Ok" : "Err": field[key].map { evaluate($0, locals: locals, scope: scope) } ?? NSNull()]
        case "Try":
            let field = payload as? [String: Any] ?? [:]
            guard let expr = field["expr"] else { return NSNull() }
            let result = evaluate(expr, locals: locals, scope: scope)
            return (result as? [String: Any])?["Ok"] ?? NSNull()
        case "Call":
            guard let call = payload as? [String: Any],
                  let name = call["name"] as? String else { return NSNull() }
            if call["is_constructor"] as? Bool == true,
               let returnType = call["return_type"] as? [String: Any],
               let pluginType = returnType["Plugin"] as? [String: Any],
               let namespace = pluginType["namespace"] as? String,
               let className = pluginType["name"] as? String {
                let arguments = (call["arguments"] as? [Any] ?? []).map {
                    evaluate($0, locals: locals, scope: scope)
                }
                let result = NexaDevPluginBridge.construct(
                    namespace: namespace,
                    name: className,
                    arguments: arguments
                )
                if result.0 { return result.1 }
            }
            if call["is_constructor"] as? Bool == true, let fields = structs[name] {
                let arguments = call["arguments"] as? [Any] ?? []
                var instance: [String: Any] = [:]
                for (field, argument) in zip(fields, arguments) {
                    guard let fieldName = field["name"] as? String else { continue }
                    instance[fieldName] = evaluate(argument, locals: locals, scope: scope)
                }
                return instance
            }
            guard let function = functions[name], activeFunctions.insert(name).inserted else { return NSNull() }
            defer { activeFunctions.remove(name) }
            let parameters = function["parameters"] as? [[String: Any]] ?? []
            let arguments = call["arguments"] as? [Any] ?? []
            guard parameters.count == arguments.count else { return NSNull() }
            var functionScope = locals
            for (parameter, argument) in zip(parameters, arguments) {
                guard let parameterName = parameter["name"] as? String else { continue }
                functionScope[parameterName] = evaluate(argument, locals: functionScope, scope: scope)
            }
            for local in function["locals"] as? [[String: Any]] ?? [] {
                guard let localName = local["name"] as? String,
                      let initial = local["initial"]
                else { continue }
                functionScope[localName] = evaluate(initial, locals: functionScope, scope: scope)
            }
            guard let body = function["body"] else { return NSNull() }
            return evaluate(body, locals: functionScope, scope: scope)
        case "NativeCall":
            guard let call = payload as? [String: Any] else { return NSNull() }
            return invokeNativeSync(call, locals: locals, scope: scope)
        case "PathJoin":
            guard let join = payload as? [String: Any] else { return NSNull() }
            return invokeNativeSync(
                [
                    "namespace": "Path",
                    "name": "join",
                    "arguments": [
                        ["path", join["path"] ?? NSNull()],
                        ["component", join["component"] ?? NSNull()],
                    ],
                ],
                locals: locals,
                scope: scope
            )
        case "FileExists":
            guard let file = payload as? [String: Any] else { return NSNull() }
            return invokeNativeSync(
                [
                    "namespace": "File",
                    "name": "exists",
                    "arguments": [["path", file["path"] ?? NSNull()]],
                ],
                locals: locals,
                scope: scope
            )
        default: return NSNull()
        }
    }

    func evaluateAsync(
        _ expression: Any,
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
        guard let tagged = expression as? [String: Any], let (kind, payload) = tagged.first else {
            return NSNull()
        }
        switch kind {
        case "EnumValue", "PluginEnumValue":
            return (payload as? [String: Any])?["case_name"] as? String ?? ""
        case "Await", "TryAwait":
            return try await evaluateAsync(payload, locals: locals, scope: scope)
        case "TimeCall":
            let call = payload as? [String: Any] ?? [:]
            let method = call["method"] as? String ?? ""
            var arguments: [Any] = []
            for argument in call["arguments"] as? [Any] ?? [] {
                arguments.append(try await evaluateAsync(argument, locals: locals, scope: scope))
            }
            let first = arguments.first
            switch method {
            case "Now": return Int64((Date().timeIntervalSince1970 * 1000).rounded())
            case "Monotonic": return Int64(bitPattern: DispatchTime.now().uptimeNanoseconds)
            case "Elapsed":
                let start = (first as? NSNumber)?.int64Value ?? 0
                return Int64(bitPattern: DispatchTime.now().uptimeNanoseconds) - start
            case "Sleep":
                let milliseconds = (first as? NSNumber)?.int64Value ?? 0
                let nanoseconds = UInt64(max(0, milliseconds)) &* 1_000_000
                try await Task.sleep(nanoseconds: nanoseconds)
                return NSNull()
            case "Iso8601":
                guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
                return NexaDevTime.iso8601(timestamp)
            case "Iso8601ToMillis":
                guard let text = first as? String else { return NSNull() }
                if let timestamp = NexaDevTime.iso8601ToMillis(text) { return timestamp }
                return NSNull()
            default: return NSNull()
            }
        case "LogCall":
            let call = payload as? [String: Any] ?? [:]
            let message: String
            if let expression = call["message"] {
                message = stringify(try await evaluateAsync(expression, locals: locals, scope: scope))
            } else {
                message = "null"
            }
            let severity = (call["method"] as? String ?? "Info").uppercased()
            NSLog("[Nexa][%@] %@", severity, message)
            return NSNull()
        case "Call":
            guard let call = payload as? [String: Any] else { return NSNull() }
            return try await invokeFunctionAsync(call, locals: locals, scope: scope)
        case "NativeCall":
            guard let call = payload as? [String: Any] else { return NSNull() }
            return try await invokeNativeAsync(call, locals: locals, scope: scope)
        case "NetworkFetch", "NetworkDownload":
            guard let fields = payload as? [String: Any] else { return NSNull() }
            let request: [String: Any]
            if kind == "NetworkDownload" {
                guard let nested = fields["request"] as? [String: Any] else { return NSNull() }
                request = nested
            } else {
                request = fields
            }
            let name = kind == "NetworkDownload" ? "download" : "fetch"
            var networkArguments: [[Any]] = [
                ["url", request["url"] ?? NSNull()],
                ["method", request["method"] ?? NSNull()],
                ["headers", request["headers"] ?? NSNull()],
                ["timeout", request["timeout"] ?? NSNull()],
                ["useCache", request["use_cache"] ?? NSNull()],
                ["followRedirects", request["follow_redirects"] ?? NSNull()],
                ["maxResponseBytes", request["max_response_bytes"] ?? NSNull()],
                ["certificatePins", request["certificate_pins"] ?? NSNull()],
            ]
            if kind == "NetworkDownload" {
                networkArguments.append(["destinationPath", fields["destination"] ?? NSNull()])
            }
            networkArguments.append(["body", request["body"] ?? NSNull()])
            return try await invokeNativeAsync(
                ["namespace": "Network", "name": name, "arguments": networkArguments],
                locals: locals,
                scope: scope
            )
        case "PathJoin":
            guard let join = payload as? [String: Any] else { return NSNull() }
            return try await invokeNativeAsync(
                [
                    "namespace": "Path",
                    "name": "join",
                    "arguments": [
                        ["path", join["path"] ?? NSNull()],
                        ["component", join["component"] ?? NSNull()],
                    ],
                ],
                locals: locals,
                scope: scope
            )
        case "FileExists", "FileReadText", "FileWriteText", "FileDelete":
            guard let file = payload as? [String: Any] else { return NSNull() }
            let name: String
            switch kind {
            case "FileExists": name = "exists"
            case "FileReadText": name = "readText"
            case "FileWriteText": name = "writeText"
            default: name = "delete"
            }
            var fileArguments: [[Any]] = [["path", file["path"] ?? NSNull()]]
            if kind == "FileWriteText" {
                fileArguments.append(["contents", file["contents"] ?? NSNull()])
            }
            return try await invokeNativeAsync(
                ["namespace": "File", "name": name, "arguments": fileArguments],
                locals: locals,
                scope: scope
            )
        case "PermissionOp":
            guard let operation = payload as? [String: Any],
                  let permission = operation["permission"]
            else { return NSNull() }
            let name: String
            switch operation["op"] as? String {
            case "Request": name = "request"
            case "Status": name = "status"
            default:
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported permission operation"]
                )
            }
            return try await invokeNativeAsync(
                [
                    "namespace": "Permissions",
                    "name": name,
                    "arguments": [["permission", permission]],
                ],
                locals: locals,
                scope: scope
            )
        case "Member":
            guard let member = payload as? [String: Any],
                  let base = member["base"],
                  let name = member["name"] as? String
            else { return NSNull() }
            let value = try await evaluateAsync(base, locals: locals, scope: scope)
            if let memberKind = member["kind"] as? [String: Any],
               let property = memberKind["PluginField"] as? String {
                let result = NexaDevPluginBridge.readInstanceProperty(receiver: value, property: property)
                if result.0 { return result.1 }
            }
            if name == "count" {
                if let values = value as? [Any] { return Int32(values.count) }
                if let values = value as? [String: Any] { return Int32(values.count) }
                if let values = value as? Set<NexaDevHashableValue> { return Int32(values.count) }
            }
            if name == "isEmpty" {
                if let values = value as? [Any] { return values.isEmpty }
                if let values = value as? [String: Any] { return values.isEmpty }
                if let values = value as? Set<NexaDevHashableValue> { return values.isEmpty }
            }
            if let object = value as? [String: Any] { return object[name] ?? NSNull() }
            if let tuple = value as? [Any] {
                let position = switch name {
                case "first": 0
                case "second": 1
                case "third": 2
                default: Int(name) ?? -1
                }
                if tuple.indices.contains(position) { return tuple[position] }
            }
            return NSNull()
        case "Array", "Set":
            let entries = payload as? [Any] ?? []
            var values: [Any] = []
            for entry in entries {
                values.append(try await evaluateAsync(entry, locals: locals, scope: scope))
            }
            if kind == "Set" { return Set(values.map(NexaDevHashableValue.init)) }
            return values
        case "Pair", "Triple":
            let entries = payload as? [Any] ?? []
            var values: [Any] = []
            for entry in entries {
                values.append(try await evaluateAsync(entry, locals: locals, scope: scope))
            }
            let required = kind == "Pair" ? 2 : 3
            guard values.count >= required else { return Array(repeating: NSNull(), count: required) }
            return Array(values.prefix(required))
        case "Map":
            let entries = payload as? [[Any]] ?? []
            var values: [String: Any] = [:]
            for pair in entries where pair.count >= 2 {
                let key = try await evaluateAsync(pair[0], locals: locals, scope: scope)
                let value = try await evaluateAsync(pair[1], locals: locals, scope: scope)
                values[stringify(key)] = value
            }
            return values
        case "Add":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 2 else { return 0 }
            let left = try await evaluateAsync(parts[0], locals: locals, scope: scope)
            let right = try await evaluateAsync(parts[1], locals: locals, scope: scope)
            if let left = left as? String, let right = right as? String { return left + right }
            let sum = number(left) + number(right)
            return sum.rounded() == sum ? Int64(sum) as Any : sum as Any
        case "Concat":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 2 else { return "" }
            return stringify(try await evaluateAsync(parts[0], locals: locals, scope: scope))
                + stringify(try await evaluateAsync(parts[1], locals: locals, scope: scope))
        case "Arithmetic":
            guard let fields = payload as? [String: Any],
                  let leftExpression = fields["left"], let rightExpression = fields["right"]
            else { return NSNull() }
            return arithmetic(
                fields["op"] as? String ?? "",
                try await evaluateAsync(leftExpression, locals: locals, scope: scope),
                try await evaluateAsync(rightExpression, locals: locals, scope: scope),
                type: fields["ty"] as? String ?? "Int32"
            )
        case "Negate":
            guard let fields = payload as? [String: Any],
                  let valueExpression = fields["value"]
            else { return NSNull() }
            let type = fields["ty"] as? String ?? "Int32"
            return arithmetic(
                "Subtract",
                numericZero(type: type),
                try await evaluateAsync(valueExpression, locals: locals, scope: scope),
                type: type
            )
        case "Not":
            return !truthy(try await evaluateAsync(payload, locals: locals, scope: scope))
        case "Binary":
            guard let binary = payload as? [String: Any],
                  let op = binary["op"] as? String,
                  let leftExpression = binary["left"],
                  let rightExpression = binary["right"]
            else { return false }
            let left = try await evaluateAsync(leftExpression, locals: locals, scope: scope)
            if op == "And", !truthy(left) { return false }
            if op == "Or", truthy(left) { return true }
            let right = try await evaluateAsync(rightExpression, locals: locals, scope: scope)
            return compare(op, left, right)
        case "Interpolation":
            var result = ""
            for rawPart in payload as? [Any] ?? [] {
                guard let part = rawPart as? [String: Any], let (partKind, partValue) = part.first else { continue }
                if partKind == "Literal" {
                    result += partValue as? String ?? ""
                } else if partKind == "Value" {
                    result += stringify(try await evaluateAsync(partValue, locals: locals, scope: scope))
                }
            }
            return result
        case "Index":
            guard let fields = payload as? [String: Any],
                  let collectionExpression = fields["collection"],
                  let indexExpression = fields["index"]
            else { return NSNull() }
            let collection = try await evaluateAsync(collectionExpression, locals: locals, scope: scope)
            let indexValue = try await evaluateAsync(indexExpression, locals: locals, scope: scope)
            let index = (indexValue as? NSNumber)?.intValue ?? -1
            if let values = collection as? [Any], values.indices.contains(index) { return values[index] }
            if let values = collection as? [String: Any] { return values[stringify(indexValue)] ?? NSNull() }
            return NSNull()
        case "Range":
            guard let fields = payload as? [String: Any],
                  let startExpression = fields["start"],
                  let endExpression = fields["end"]
            else { return [Int]() }
            let start = number(try await evaluateAsync(startExpression, locals: locals, scope: scope))
            let end = number(try await evaluateAsync(endExpression, locals: locals, scope: scope))
            var step = 1.0
            if let stepExpression = fields["step"] {
                step = number(try await evaluateAsync(stepExpression, locals: locals, scope: scope))
            }
            guard start.isFinite, end.isFinite, step.isFinite, abs(end - start) < 100_000 else { return [Int]() }
            let strideValue = max(1, abs(Int(step)))
            let inclusive = fields["inclusive"] as? Bool ?? false
            let startValue = Int(start)
            let endValue = Int(end)
            let boundary = endValue + ((inclusive && endValue >= startValue) ? 1 : 0)
            guard startValue != boundary else { return [Int]() }
            return Array(stride(from: startValue, to: boundary, by: startValue <= endValue ? strideValue : -strideValue))
        case "Contains":
            guard let fields = payload as? [String: Any],
                  let valueExpression = fields["value"],
                  let collectionExpression = fields["collection"]
            else { return false }
            let value = try await evaluateAsync(valueExpression, locals: locals, scope: scope)
            let collection = try await evaluateAsync(collectionExpression, locals: locals, scope: scope)
            return contains(value, in: collection)
        case "CollectionTransform":
            guard let fields = payload as? [String: Any],
                  let collectionExpression = fields["collection"]
            else { return [Any]() }
            let collectionValue = try await evaluateAsync(collectionExpression, locals: locals, scope: scope)
            let setItems = NexaDevValueCodec.asSet(collectionValue)?
                .sorted { $0.stableOrderKey < $1.stableOrderKey }
                .map(\.value) ?? []
            let items = collectionValue as? [Any] ?? setItems
            let closureObject = (fields["closure"] as? [String: Any])?["Closure"] as? [String: Any] ?? [:]
            let parameters = closureObject["parameters"] as? [String] ?? []
            let body = closureObject["body"] ?? NSNull()
            func apply(_ item: Any, accumulator: Any? = nil) async throws -> Any {
                var closureLocals = locals
                if parameters.count > 1, let accumulator {
                    closureLocals[parameters[0]] = accumulator
                    closureLocals[parameters[1]] = item
                } else if let parameter = parameters.first {
                    closureLocals[parameter] = item
                }
                return try await evaluateAsync(body, locals: closureLocals, scope: scope)
            }
            switch fields["operation"] as? String {
            case "Map":
                var mapped: [Any] = []
                for item in items { mapped.append(try await apply(item)) }
                return mapped
            case "Filter":
                var filtered: [Any] = []
                for item in items {
                    if truthy(try await apply(item)) { filtered.append(item) }
                }
                return filtered
            case "Reduce":
                var accumulator: Any = 0
                if let initial = fields["initial"] {
                    accumulator = try await evaluateAsync(initial, locals: locals, scope: scope)
                }
                for item in items { accumulator = try await apply(item, accumulator: accumulator) }
                return accumulator
            default: return items
            }
        case "CollectionUtility":
            guard let fields = payload as? [String: Any],
                  let collectionExpression = fields["collection"],
                  let values = try await evaluateAsync(collectionExpression, locals: locals, scope: scope) as? [Any]
            else { return [Any]() }
            switch fields["operation"] as? String {
            case "Random": return values.randomElement() ?? NSNull()
            case "First": return values.first ?? NSNull()
            case "Last": return values.last ?? NSNull()
            case "Shuffled": return values.shuffled()
            case "Reverse": return Array(values.reversed())
            case "Slice":
                guard let startExpression = fields["start"], let endExpression = fields["end"] else { return [Any]() }
                let start = Int(number(try await evaluateAsync(startExpression, locals: locals, scope: scope)))
                let end = Int(number(try await evaluateAsync(endExpression, locals: locals, scope: scope)))
                let lower = min(max(start, 0), values.count)
                let stop = end + ((fields["inclusive"] as? Bool == true) ? 1 : 0)
                let upper = min(max(stop, lower), values.count)
                return Array(values[lower..<upper])
            default: return [Any]()
            }
        case "BytesFromText":
            let text = try await evaluateAsync((payload as? [String: Any])?["text"] ?? NSNull(), locals: locals, scope: scope)
            return Data((text as? String ?? "").utf8).base64EncodedString()
        case "BytesFromArray":
            let values = try await evaluateAsync((payload as? [String: Any])?["values"] ?? NSNull(), locals: locals, scope: scope)
            let bytes = (values as? [Any] ?? []).map { UInt8(truncatingIfNeeded: Int64(stringify($0)) ?? 0) }
            return Data(bytes).base64EncodedString()
        case "BytesCount":
            let bytes = try await evaluateAsync((payload as? [String: Any])?["bytes"] ?? NSNull(), locals: locals, scope: scope)
            guard let text = bytes as? String, let data = Data(base64Encoded: text) else { return 0 }
            return data.count
        case "Coalesce":
            let parts = payload as? [Any] ?? []
            guard parts.count >= 2 else { return NSNull() }
            let value = try await evaluateAsync(parts[0], locals: locals, scope: scope)
            if value is NSNull { return try await evaluateAsync(parts[1], locals: locals, scope: scope) }
            return value
        case "ResultOk", "ResultErr":
            let key = kind == "ResultOk" ? "value" : "error"
            let fields = payload as? [String: Any] ?? [:]
            let value = try await evaluateAsync(fields[key] ?? NSNull(), locals: locals, scope: scope)
            return [kind == "ResultOk" ? "Ok" : "Err": value]
        case "Try":
            let fields = payload as? [String: Any] ?? [:]
            let value = try await evaluateAsync(fields["expr"] ?? NSNull(), locals: locals, scope: scope)
            return (value as? [String: Any])?["Ok"] ?? NSNull()
        case "IsRegularWidth", "IsCompactWidth", "IsRegularHeight", "IsCompactHeight":
            return evaluate(expression, locals: locals, scope: scope)
        case "Conditional":
            guard let fields = payload as? [String: Any],
                  let conditionExpression = fields["condition"]
            else { return NSNull() }
            let condition = try await evaluateAsync(conditionExpression, locals: locals, scope: scope)
            let branch = truthy(condition) ? fields["then_value"] : fields["else_value"]
            guard let branch else { return NSNull() }
            return try await evaluateAsync(branch, locals: locals, scope: scope)
        default:
            return evaluate(expression, locals: locals, scope: scope)
        }
    }

    func invokeFunctionAsync(
        _ call: [String: Any],
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
        if call["is_constructor"] as? Bool == true,
           let returnType = call["return_type"] as? [String: Any],
           let pluginType = returnType["Plugin"] as? [String: Any],
           let namespace = pluginType["namespace"] as? String,
           let className = pluginType["name"] as? String {
            var arguments: [Any] = []
            for expression in call["arguments"] as? [Any] ?? [] {
                arguments.append(try await evaluateAsync(expression, locals: locals, scope: scope))
            }
            let result = NexaDevPluginBridge.construct(
                namespace: namespace,
                name: className,
                arguments: arguments
            )
            if result.0 { return result.1 }
        }
        if call["is_constructor"] as? Bool == true,
           let name = call["name"] as? String,
           let fields = structs[name] {
            let arguments = call["arguments"] as? [Any] ?? []
            guard fields.count == arguments.count else { return NSNull() }
            var instance: [String: Any] = [:]
            for (field, argument) in zip(fields, arguments) {
                guard let fieldName = field["name"] as? String else { continue }
                instance[fieldName] = try await evaluateAsync(argument, locals: locals, scope: scope)
            }
            return instance
        }
        guard let name = call["name"] as? String,
              let function = functions[name],
              activeFunctions.insert(name).inserted
        else { return NSNull() }
        defer { activeFunctions.remove(name) }
        let parameters = function["parameters"] as? [[String: Any]] ?? []
        let arguments = call["arguments"] as? [Any] ?? []
        guard parameters.count == arguments.count else { return NSNull() }
        var functionScope = locals
        for (parameter, argument) in zip(parameters, arguments) {
            guard let parameterName = parameter["name"] as? String else { continue }
            functionScope[parameterName] = try await evaluateAsync(argument, locals: functionScope, scope: scope)
        }
        for local in function["locals"] as? [[String: Any]] ?? [] {
            guard let localName = local["name"] as? String,
                  let initial = local["initial"]
            else { continue }
            functionScope[localName] = try await evaluateAsync(initial, locals: functionScope, scope: scope)
        }
        guard let body = function["body"] else { return NSNull() }
        return try await evaluateAsync(body, locals: functionScope, scope: scope)
    }

    func compare(_ op: String, _ lhs: Any, _ rhs: Any) -> Bool {
        switch op {
        case "And": return truthy(lhs) && truthy(rhs)
        case "Or": return truthy(lhs) || truthy(rhs)
        case "Equal": return stringify(lhs) == stringify(rhs)
        case "NotEqual": return stringify(lhs) != stringify(rhs)
        case "Less": return number(lhs) < number(rhs)
        case "LessEqual": return number(lhs) <= number(rhs)
        case "Greater": return number(lhs) > number(rhs)
        case "GreaterEqual": return number(lhs) >= number(rhs)
        case "Contains": return contains(lhs, in: rhs)
        default: return false
        }
    }

    func contains(_ value: Any, in collection: Any) -> Bool {
        if let values = collection as? [Any] { return values.contains { stringify($0) == stringify(value) } }
        if let values = NexaDevValueCodec.asSet(collection) {
            return values.contains(NexaDevHashableValue(value))
        }
        if let values = collection as? [String: Any] { return values[stringify(value)] != nil }
        if let text = collection as? String, let needle = value as? String { return text.contains(needle) }
        return false
    }

    func number(_ value: Any) -> Double {
        if let number = value as? NSNumber { return number.doubleValue }
        if let text = value as? String { return Double(text) ?? 0 }
        return 0
    }

    private func numericZero(type: String) -> Any {
        if type == "Float32" { return Float(0) }
        if type == "Float64" { return Double(0) }
        if type.hasPrefix("UInt") { return UInt64(0) }
        return Int64(0)
    }

    private func arithmetic(_ op: String, _ lhs: Any, _ rhs: Any, type: String) -> Any {
        if type == "Float32" {
            let left = Float(number(lhs))
            let right = Float(number(rhs))
            switch op {
            case "Subtract": return left - right
            case "Multiply": return left * right
            case "Divide": return left / right
            case "Remainder": return left.truncatingRemainder(dividingBy: right)
            default: return Float(0)
            }
        }
        if type == "Float64" {
            let left = number(lhs)
            let right = number(rhs)
            switch op {
            case "Subtract": return left - right
            case "Multiply": return left * right
            case "Divide": return left / right
            case "Remainder": return left.truncatingRemainder(dividingBy: right)
            default: return 0.0
            }
        }
        if type.hasPrefix("UInt") {
            let left = unsignedInteger(lhs)
            let right = unsignedInteger(rhs)
            let result: UInt64
            switch op {
            case "Subtract": result = left &- right
            case "Multiply": result = left &* right
            case "Divide": result = right == 0 ? 0 : left / right
            case "Remainder": result = right == 0 ? 0 : left % right
            default: return UInt64(0)
            }
            switch type {
            case "UInt8": return UInt8(truncatingIfNeeded: result)
            case "UInt16": return UInt16(truncatingIfNeeded: result)
            case "UInt32": return UInt32(truncatingIfNeeded: result)
            default: return result
            }
        }
        let left = signedInteger(lhs)
        let right = signedInteger(rhs)
        let result: Int64
        switch op {
        case "Subtract": result = left &- right
        case "Multiply": result = left &* right
        case "Divide":
            if right == 0 { return Int64(0) }
            if left == Int64.min && right == -1 { result = Int64.min } else { result = left / right }
        case "Remainder":
            if right == 0 { return Int64(0) }
            result = left == Int64.min && right == -1 ? 0 : left % right
        default: return Int64(0)
        }
        switch type {
        case "Int8": return Int8(truncatingIfNeeded: result)
        case "Int16": return Int16(truncatingIfNeeded: result)
        case "Int32": return Int32(truncatingIfNeeded: result)
        default: return result
        }
    }

    private func signedInteger(_ value: Any) -> Int64 {
        if let number = value as? NSNumber { return number.int64Value }
        if let text = value as? String { return Int64(text) ?? 0 }
        return 0
    }

    private func unsignedInteger(_ value: Any) -> UInt64 {
        if let number = value as? NSNumber { return number.uint64Value }
        if let text = value as? String { return UInt64(text) ?? 0 }
        return 0
    }

    func truthy(_ value: Any) -> Bool {
        if let number = value as? NSNumber { return number.boolValue }
        if let text = value as? String { return !text.isEmpty }
        return false
    }

    func stringify(_ value: Any) -> String {
        if let wrapped = value as? NexaDevHashableValue { return stringify(wrapped.value) }
        if value is NSNull { return "null" }
        if let number = value as? NSNumber {
            if CFGetTypeID(number) == CFBooleanGetTypeID() { return number.boolValue ? "true" : "false" }
            let double = number.doubleValue
            return double.rounded() == double ? String(Int64(double)) : String(double)
        }
        return String(describing: value)
    }

    static func canonicalSignature(_ parameters: [[String: Any]]) -> String {
        canonicalJSON(parameters.map { $0["ty"] ?? NSNull() })
    }

    static func canonicalJSON(_ value: Any?) -> String {
        guard let value, JSONSerialization.isValidJSONObject([value]),
              let data = try? JSONSerialization.data(withJSONObject: [value], options: [.sortedKeys])
        else { return "null" }
        return String(data: data, encoding: .utf8) ?? "null"
    }
}

/// A core clock call, evaluated the same way the generated code renders it: a
/// system clock, a monotonic counter, or the ISO 8601 helpers the app target
/// carries.
private extension NexaDevStateStore {
func evaluateTimeCall(_ call: [String: Any]?, locals: [String: Any], scope: String) -> Any {
    guard let call,
        let method = call["method"] as? String
    else {
        return NSNull()
    }
    let arguments = (call["arguments"] as? [Any] ?? []).map { evaluate($0, locals: locals, scope: scope) }
    let first = arguments.first
    switch method {
    case "Now":
        return Int64((Date().timeIntervalSince1970 * 1000).rounded())
    case "Monotonic":
        return Int64(bitPattern: DispatchTime.now().uptimeNanoseconds)
    case "Elapsed":
        let start = (first as? NSNumber)?.int64Value ?? 0
        return Int64(bitPattern: DispatchTime.now().uptimeNanoseconds) - start
    case "Sleep":
        // Sleep suspends, and a synchronous evaluator cannot suspend, so a
        // hot reload reports the request and moves on rather than blocking the
        // UI thread it is being evaluated on.
        return NSNull()
    case "Iso8601":
        guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
        return NexaDevTime.iso8601(timestamp)
    case "Iso8601ToMillis":
        guard let text = first as? String else { return NSNull() }
        guard let value = NexaDevTime.iso8601ToMillis(text) else { return NSNull() }
        return value
    default:
        return NSNull()
    }
}
}

/// The ISO 8601 layout the generated code implements, kept in one place so the
/// two cannot drift.
enum NexaDevTime {
    private static let epochDays: Int64 = 719_468
    private static let daysIn400Years: Int64 = 146_097

    static func iso8601(_ milliseconds: Int64) -> String {
        var days = milliseconds / 86_400_000
        var remainder = milliseconds % 86_400_000
        if remainder < 0 {
            remainder += 86_400_000
            days -= 1
        }
        let (year, month, day) = civilDate(days)
        let hour = remainder / 3_600_000
        remainder %= 3_600_000
        let minute = remainder / 60_000
        remainder %= 60_000
        let second = remainder / 1000
        let millisecond = remainder % 1000
        return String(
            format: "%04d-%02d-%02dT%02d:%02d:%02d.%03dZ",
            year, month, day, hour, minute, second, millisecond
        )
    }

    static func iso8601ToMillis(_ text: String) -> Int64? {
        let scalars = Array(text.utf8)
        guard scalars.count == 20 || scalars.count == 24,
            scalars[4] == UInt8(ascii: "-"),
            scalars[7] == UInt8(ascii: "-"),
            scalars[10] == UInt8(ascii: "T"),
            scalars[13] == UInt8(ascii: ":"),
            scalars[16] == UInt8(ascii: ":"),
            scalars[scalars.count - 1] == UInt8(ascii: "Z")
        else {
            return nil
        }
        if scalars.count == 24 {
            guard scalars[19] == UInt8(ascii: ".") else { return nil }
        }
        // Digit runs in layout order. A run is read left to right, so its first
        // digit is the most significant one.
        let runs = [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)]
        var values: [Int64] = []
        for (start, end) in runs {
            guard let value = nexaDevDigits(scalars[start..<end]) else { return nil }
            values.append(value)
        }
        if scalars.count == 24 {
            guard let millisecond = nexaDevDigits(scalars[20..<23]) else { return nil }
            values.append(millisecond)
        }
        let (year, month, day) = (values[0], values[1], values[2])
        let (hour, minute, second) = (values[3], values[4], values[5])
        guard (1...12).contains(month), day >= 1 else { return nil }
        guard day <= nexaDevMonthLength(year: year, month: month) else { return nil }
        guard hour < 24, minute < 60, second <= 60 else { return nil }
        let millisecond = values.count > 6 ? values[6] : 0
        guard millisecond < 1000 else { return nil }
        let days = daysFromCivil(year: year, month: month, day: day)
        return (((days * 24 + hour) * 60 + minute) * 60 + second) * 1000 + millisecond
    }

    private static func civilDate(_ daysSinceEpoch: Int64) -> (Int64, Int64, Int64) {
        let shifted = daysSinceEpoch + epochDays
        let era = (shifted >= 0 ? shifted : shifted - (daysIn400Years - 1)) / daysIn400Years
        let dayOfEra = shifted - era * daysIn400Years
        let yearOfEra = (dayOfEra - dayOfEra / 1460 + dayOfEra / 36_524 - dayOfEra / 146_096) / 365
        let year = yearOfEra + era * 400
        let dayOfYear = dayOfEra - (365 * yearOfEra + yearOfEra / 4 - yearOfEra / 100)
        let monthIndex = (5 * dayOfYear + 2) / 153
        let day = dayOfYear - (153 * monthIndex + 2) / 5 + 1
        let month = monthIndex + (monthIndex < 10 ? 3 : -9)
        return (month <= 2 ? year + 1 : year, month, day)
    }

    private static func daysFromCivil(year: Int64, month: Int64, day: Int64) -> Int64 {
        let adjustedYear = year - (month <= 2 ? 1 : 0)
        let era = (adjustedYear >= 0 ? adjustedYear : adjustedYear - 399) / 400
        let yearOfEra = adjustedYear - era * 400
        let monthIndex = month > 2 ? month - 3 : month + 9
        let dayOfYear = (153 * monthIndex + 2) / 5 + day - 1
        let dayOfEra = yearOfEra * 365 + yearOfEra / 4 - yearOfEra / 100 + dayOfYear
        return era * daysIn400Years + dayOfEra - epochDays
    }
}

/// A run of ASCII digits as a number, or nil when anything else is in it.
private func nexaDevDigits(_ digits: ArraySlice<UInt8>) -> Int64? {
    var value: Int64 = 0
    for digit in digits {
        guard digit >= UInt8(ascii: "0"), digit <= UInt8(ascii: "9") else { return nil }
        value = value * 10 + Int64(digit - UInt8(ascii: "0"))
    }
    return value
}

/// The number of days in a proleptic Gregorian month. February is the only one
/// that depends on the year, and 31 February has to be rejected rather than
/// rolled forward into March.
func nexaDevMonthLength(year: Int64, month: Int64) -> Int64 {
    if month == 2 {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
        return leap ? 29 : 28
    }
    if month == 4 || month == 6 || month == 9 || month == 11 {
        return 30
    }
    return 31
}
