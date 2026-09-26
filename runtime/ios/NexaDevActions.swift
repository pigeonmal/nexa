import Foundation

/// Flow control signal for synchronous Nexa dev action execution loops.
enum NexaDevActionFlow: Equatable {
    case normal
    case `break`
    case `continue`
}

@MainActor
extension NexaDevStateStore {
    func perform(_ actions: [Any], scope: String, locals: [String: Any]) {
        _ = performActions(actions, scope: scope, locals: locals)
    }

    @discardableResult
    func performActions(_ actions: [Any], scope: String, locals: [String: Any]) -> NexaDevActionFlow {
        for action in actions {
            if let unitVariant = action as? String {
                if unitVariant == "Break" { return .break }
                if unitVariant == "Continue" { return .continue }
                continue
            }
            guard let tagged = action as? [String: Any] else { continue }
            if let assignment = tagged["Assign"] as? [String: Any],
               let name = assignment["name"] as? String,
               let expression = assignment["value"] {
                setValue(name, value: evaluate(expression, locals: locals, scope: scope), scope: scope)
                revision += 1
            } else if let expression = tagged["Expression"] {
                _ = evaluate(expression, locals: locals, scope: scope)
            } else if let branch = tagged["If"] as? [String: Any],
                      let condition = branch["condition"] {
                let selected = truthy(evaluate(condition, locals: locals, scope: scope))
                    ? branch["then_branch"] as? [Any]
                    : branch["else_branch"] as? [Any]
                let flow = performActions(selected ?? [], scope: scope, locals: locals)
                if flow != .normal { return flow }
            } else if let loop = tagged["For"] as? [String: Any],
                      let name = loop["name"] as? String,
                      let iterableExpression = loop["iterable"] {
                let values = evaluate(iterableExpression, locals: locals, scope: scope)
                let items: [Any]
                if let array = values as? [Any] { items = array }
                else if let range = values as? Range<Int> { items = Array(range) }
                else if let range = values as? ClosedRange<Int> { items = Array(range) }
                else { items = [] }
                iterationLoop: for item in items {
                    var iterationLocals = locals
                    iterationLocals[name] = item
                    switch performActions(loop["body"] as? [Any] ?? [], scope: scope, locals: iterationLocals) {
                    case .break: break iterationLoop
                    case .normal, .continue: continue
                    }
                }
            } else if let loop = tagged["ForMap"] as? [String: Any],
                      let keyName = loop["key_name"] as? String,
                      let valueName = loop["value_name"] as? String,
                      let iterableExpression = loop["iterable"],
                      let map = evaluate(iterableExpression, locals: locals, scope: scope) as? [String: Any] {
                iterationLoop: for key in map.keys.sorted() {
                    var iterationLocals = locals
                    iterationLocals[keyName] = key
                    iterationLocals[valueName] = map[key] ?? NSNull()
                    switch performActions(loop["body"] as? [Any] ?? [], scope: scope, locals: iterationLocals) {
                    case .break: break iterationLoop
                    case .normal, .continue: continue
                    }
                }
            } else if let loop = tagged["While"] as? [String: Any], let condition = loop["condition"] {
                iterationLoop: for _ in 0..<10_000 {
                    guard truthy(evaluate(condition, locals: locals, scope: scope)) else { break }
                    switch performActions(loop["body"] as? [Any] ?? [], scope: scope, locals: locals) {
                    case .break: break iterationLoop
                    case .normal, .continue: continue
                    }
                }
            } else if let mutation = tagged["CollectionMutation"] as? [String: Any],
                      let name = mutation["name"] as? String {
                performCollectionMutation(mutation, name: name, scope: scope, locals: locals)
            } else if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                let flow = performActions(tryCatch["body"] as? [Any] ?? [], scope: scope, locals: locals)
                if flow != .normal { return flow }
            } else if tagged["Break"] != nil {
                return .break
            } else if tagged["Continue"] != nil {
                return .continue
            }
        }
        return .normal
    }

    func performCollectionMutation(
        _ mutation: [String: Any],
        name: String,
        scope: String,
        locals: [String: Any]
    ) {
        let arguments = (mutation["arguments"] as? [Any] ?? []).map { evaluate($0, locals: locals, scope: scope) }
        switch mutation["operation"] as? String {
        case "ArrayAppend":
            var array = value(name, scope: scope) as? [Any] ?? []
            if let item = arguments.first { array.append(item) }
            setValue(name, value: array, scope: scope)
        case "ArrayRemoveAt":
            var array = value(name, scope: scope) as? [Any] ?? []
            if let index = (arguments.first as? NSNumber)?.intValue, array.indices.contains(index) { array.remove(at: index) }
            setValue(name, value: array, scope: scope)
        case "SetInsert", "SetRemove":
            var set = value(name, scope: scope) as? Set<String> ?? []
            if let item = arguments.first.map(stringify) {
                if mutation["operation"] as? String == "SetInsert" { set.insert(item) } else { set.remove(item) }
            }
            setValue(name, value: set, scope: scope)
        case "MapSet", "MapRemove":
            var map = value(name, scope: scope) as? [String: Any] ?? [:]
            if let key = arguments.first.map(stringify) {
                if mutation["operation"] as? String == "MapSet", arguments.count > 1 { map[key] = arguments[1] }
                else { map.removeValue(forKey: key) }
            }
            setValue(name, value: map, scope: scope)
        default: break
        }
    }

    func performAsync(_ actions: [Any], scope: String, locals: [String: Any]) async throws {
        for action in actions {
            guard let tagged = action as? [String: Any] else { continue }
            if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                do {
                    try await performAsync(
                        tryCatch["body"] as? [Any] ?? [],
                        scope: scope,
                        locals: locals
                    )
                } catch is CancellationError {
                    throw CancellationError()
                } catch {
                    NSLog("NexaDevRuntime async dev action failed: %@", String(describing: error))
                    if let catchBody = tryCatch["catch_body"] as? [Any] {
                        try await performAsync(catchBody, scope: scope, locals: locals)
                    } else {
                        throw error
                    }
                }
            } else if let assignment = tagged["Assign"] as? [String: Any],
               let name = assignment["name"] as? String,
               let expression = assignment["value"] {
                let value = try await evaluateAsync(expression, locals: locals, scope: scope)
                setValue(name, value: value, scope: scope)
                revision += 1
            } else if let branch = tagged["If"] as? [String: Any],
                      let condition = branch["condition"] {
                let value = try await evaluateAsync(condition, locals: locals, scope: scope)
                let selected = truthy(value)
                    ? branch["then_branch"] as? [Any]
                    : branch["else_branch"] as? [Any]
                try await performAsync(selected ?? [], scope: scope, locals: locals)
            } else if let expression = tagged["Expression"] {
                _ = try await evaluateAsync(expression, locals: locals, scope: scope)
            }
        }
    }
}
