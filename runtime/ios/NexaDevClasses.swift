import Foundation

@MainActor
final class NexaDevUserClassInstance {
    let className: String
    let fields: [String: Any]

    init(className: String, fields: [String: Any]) {
        self.className = className
        self.fields = fields
    }
}

@MainActor
extension NexaDevStateStore {
    func makeUserClassInstance(
        _ classType: [String: Any],
        arguments: [Any],
        scope: String
    ) -> Any {
        guard let className = classType["name"] as? String else { return NSNull() }
        let fields = classType["fields"] as? [[Any]] ?? []
        let constructorCount = classType["constructor_parameter_count"] as? Int ?? fields.count
        guard constructorCount == arguments.count, constructorCount <= fields.count else { return NSNull() }
        var instanceFields: [String: Any] = [:]
        for (field, argument) in zip(fields.prefix(constructorCount), arguments) {
            guard let name = field.first as? String else { continue }
            instanceFields[name] = argument
        }
        guard let classFunction = functions.keys.sorted().compactMap({ functions[$0] }).first(where: { function in
            guard let receiver = function["receiver"] as? [String: Any],
                  let owner = receiver["Class"] as? [String: Any]
            else { return false }
            return owner["name"] as? String == className
        }) else { return NexaDevUserClassInstance(className: className, fields: instanceFields) }
        for initializer in classFunction["class_initializers"] as? [[String: Any]] ?? [] {
            guard let name = initializer["name"] as? String,
                  let expression = initializer["initial"] else { continue }
            var initializerLocals = moduleValues
            initializerLocals.merge(instanceFields) { _, instanceValue in instanceValue }
            instanceFields[name] = evaluate(expression, locals: initializerLocals, scope: scope)
        }
        return NexaDevUserClassInstance(className: className, fields: instanceFields)
    }

    /// Executes a synchronous Nexa class method against its reference value.
    func invokeUserClassMethod(
        _ className: String,
        method: String,
        receiver: Any,
        arguments: [Any],
        locals: [String: Any],
        scope: String
    ) -> Any {
        guard let instance = receiver as? NexaDevUserClassInstance,
              instance.className == className
        else { return NSNull() }
        let functionName = "\(className).\(method)"
        guard let function = functions[functionName],
              activeFunctions.insert(functionName).inserted else { return NSNull() }
        defer { activeFunctions.remove(functionName) }
        let parameters = function["parameters"] as? [[String: Any]] ?? []
        guard parameters.count == arguments.count else { return NSNull() }
        var functionScope = locals
        functionScope["this"] = instance
        // Bare immutable class properties lower to State expressions. Put the
        // receiver's fields in the local environment before method locals so
        // normal lexical bindings retain their usual shadowing behavior.
        functionScope.merge(instance.fields) { _, fieldValue in fieldValue }
        for (parameter, argument) in zip(parameters, arguments) {
            guard let name = parameter["name"] as? String else { continue }
            functionScope[name] = argument
        }
        for local in function["locals"] as? [[String: Any]] ?? [] {
            guard let name = local["name"] as? String,
                  let initial = local["initial"] else { continue }
            functionScope[name] = evaluate(initial, locals: functionScope, scope: scope)
        }
        if let actions = function["body_actions"] as? [Any] {
            switch performFunctionActions(actions, scope: scope, locals: functionScope) {
            case .returned(let value): return value
            case .normal, .break, .continue: return NSNull()
            }
        }
        guard let body = function["body"] else { return NSNull() }
        return evaluate(body, locals: functionScope, scope: scope)
    }

    /// Runs async user methods in the same interpreter as async app actions so
    /// edits to class-based persistence logic remain live during hot reload.
    func invokeUserClassMethodAsync(
        _ className: String,
        method: String,
        receiver: Any,
        arguments: [Any],
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
        guard let instance = receiver as? NexaDevUserClassInstance,
              instance.className == className
        else { return NSNull() }
        let functionName = "\(className).\(method)"
        guard let function = functions[functionName],
              activeFunctions.insert(functionName).inserted else { return NSNull() }
        defer { activeFunctions.remove(functionName) }
        let parameters = function["parameters"] as? [[String: Any]] ?? []
        guard parameters.count == arguments.count else { return NSNull() }
        var functionScope = locals
        functionScope["this"] = instance
        functionScope.merge(instance.fields) { _, fieldValue in fieldValue }
        for (parameter, argument) in zip(parameters, arguments) {
            guard let name = parameter["name"] as? String else { continue }
            functionScope[name] = argument
        }
        for local in function["locals"] as? [[String: Any]] ?? [] {
            guard let name = local["name"] as? String,
                  let initial = local["initial"] else { continue }
            functionScope[name] = try await evaluateAsync(initial, locals: functionScope, scope: scope)
        }
        if let actions = function["body_actions"] as? [Any] {
            switch try await performFunctionAsync(actions, scope: scope, locals: functionScope) {
            case .returned(let value): return value
            case .normal, .break, .continue: return NSNull()
            }
        }
        guard let body = function["body"] else { return NSNull() }
        return try await evaluateAsync(body, locals: functionScope, scope: scope)
    }
}

extension String {
    func stripPrefix(_ prefix: String) -> String? {
        hasPrefix(prefix) ? String(dropFirst(prefix.count)) : nil
    }
}
