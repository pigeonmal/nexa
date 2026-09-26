import Foundation
import CoreFoundation
import UIKit

/// Reactive state store and expression evaluation engine for the Nexa dev runtime.
@MainActor
final class NexaDevStateStore: ObservableObject {
    @Published private(set) var revision = 0
    @Published private(set) var appLifecycleEpoch = 0
    @Published var navigationPath: [NexaDevRoute] = []
    @Published private(set) var focusedFieldKey: String?
    var values: [String: Any] = [:]
    var typeSignatures: [String: String] = [:]
    var functions: [String: [String: Any]] = [:]
    var structs: [String: [[String: Any]]] = [:]
    var routeArguments: [String: (screen: String, values: [String: Any], signature: String)] = [:]
    var navigationRoot: String?
    var activeFunctions = Set<String>()
    var focusBindings: [String: (scope: String, state: String?)] = [:]
    var hasInstalledModule = false

    func install(module: [String: Any]) {
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
        var nextValues: [String: Any] = [:]
        var nextTypes: [String: String] = [:]
        let declarations = appStates.map { ("app", $0) } + screens.flatMap { screen in
            guard let name = screen["name"] as? String else { return [(String, [String: Any])]() }
            return (screen["states"] as? [[String: Any]] ?? []).map { ("screen/\(name)", $0) }
        }
        for (scope, state) in declarations {
            guard let name = state["name"] as? String else { continue }
            let identity = "\(scope)/state/\(name)"
            let signature = Self.canonicalJSON(state["ty"])
            nextTypes[identity] = signature
            if typeSignatures[identity] == signature, let value = values[identity] {
                nextValues[identity] = value
            } else if let initial = state["initial"] {
                nextValues[identity] = evaluate(initial, locals: [:], scope: scope)
            }
        }
        values = nextValues
        typeSignatures = nextTypes
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
        let screens = module["screens"] as? [[String: Any]] ?? []
        let appStates = module["states"] as? [[String: Any]] ?? []
        var nextValues: [String: Any] = [:]
        for state in appStates {
            guard let name = state["name"] as? String, let initial = state["initial"] else { continue }
            nextValues["app/state/\(name)"] = evaluate(initial, locals: [:], scope: "app")
        }
        for screen in screens {
            guard let screenName = screen["name"] as? String else { continue }
            for state in screen["states"] as? [[String: Any]] ?? [] {
                guard let name = state["name"] as? String, let initial = state["initial"] else { continue }
                nextValues["screen/\(screenName)/state/\(name)"] = evaluate(initial, locals: [:], scope: "screen/\(screenName)")
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
            return Set((payload as? [Any] ?? []).map { stringify(evaluate($0, locals: locals, scope: scope)) })
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
            let number = (payload as? [String: Any])?["raw"] as? String ?? "0"
            return number.contains(".") ? (Double(number) ?? 0) as Any : (Int64(number) ?? 0) as Any
        case "EnumValue": return (payload as? [String: Any])?["case_name"] as? String ?? ""
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
        case "Contains":
            let fields = payload as? [String: Any] ?? [:]
            guard let value = fields["value"], let collection = fields["collection"] else { return false }
            return contains(evaluate(value, locals: locals, scope: scope), in: evaluate(collection, locals: locals, scope: scope))
        case "CollectionTransform":
            let fields = payload as? [String: Any] ?? [:]
            guard let collectionExpression = fields["collection"],
                  let collection = evaluate(collectionExpression, locals: locals, scope: scope) as? [Any],
                  let closure = fields["closure"] as? [String: Any],
                  let closurePayload = closure["Closure"] as? [String: Any]
            else { return [] }
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
        case "Await", "TryAwait":
            return try await evaluateAsync(payload, locals: locals, scope: scope)
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
            return (value as? [String: Any])?[name] ?? NSNull()
        case "Array", "Set":
            let entries = payload as? [Any] ?? []
            var values: [Any] = []
            for entry in entries {
                values.append(try await evaluateAsync(entry, locals: locals, scope: scope))
            }
            if kind == "Set" { return Set(values.compactMap { $0 as? String }) }
            return values
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
        case "Not":
            return !truthy(try await evaluateAsync(payload, locals: locals, scope: scope))
        case "Binary":
            guard let binary = payload as? [String: Any],
                  let op = binary["op"] as? String,
                  let leftExpression = binary["left"],
                  let rightExpression = binary["right"]
            else { return false }
            let left = try await evaluateAsync(leftExpression, locals: locals, scope: scope)
            let right = try await evaluateAsync(rightExpression, locals: locals, scope: scope)
            return compare(op, left, right)
        default:
            return evaluate(expression, locals: locals, scope: scope)
        }
    }

    func invokeFunctionAsync(
        _ call: [String: Any],
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
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
        if let values = collection as? Set<String> { return values.contains(stringify(value)) }
        if let values = collection as? [String: Any] { return values[stringify(value)] != nil }
        if let text = collection as? String, let needle = value as? String { return text.contains(needle) }
        return false
    }

    func number(_ value: Any) -> Double {
        if let number = value as? NSNumber { return number.doubleValue }
        if let text = value as? String { return Double(text) ?? 0 }
        return 0
    }

    func truthy(_ value: Any) -> Bool {
        if let number = value as? NSNumber { return number.boolValue }
        if let text = value as? String { return !text.isEmpty }
        return false
    }

    func stringify(_ value: Any) -> String {
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
