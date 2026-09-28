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
            if pendingPluginFailure != nil { return .normal }
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
            } else if let assignment = tagged["NativePropertyAssign"] as? [String: Any],
                      let receiverExpression = assignment["receiver"],
                      let property = assignment["property"] as? String,
                      let valueExpression = assignment["value"] {
                let receiver = evaluate(receiverExpression, locals: locals, scope: scope)
                let value = evaluate(valueExpression, locals: locals, scope: scope)
                if !NexaDevPluginBridge.writeInstanceProperty(receiver: receiver, property: property, value: value) {
                    NSLog("NexaDevRuntime: native property write is unsupported: %@", property)
                }
            } else if let subscription = tagged["NativeEventSubscribe"] as? [String: Any],
                      let receiverExpression = subscription["receiver"],
                      let property = subscription["property"] as? String {
                let receiver = evaluate(receiverExpression, locals: locals, scope: scope)
                if !subscribeNativeEvent(
                    receiver: receiver,
                    property: property,
                    subscription["actions"] as? [Any] ?? [],
                    parameters: subscription["parameters"] as? [String] ?? [],
                    scope: scope,
                    locals: locals
                ) {
                    NSLog("NexaDevRuntime: native event subscription is unsupported: %@", property)
                }
            } else if let mutation = tagged["CollectionMutation"] as? [String: Any],
                      let name = mutation["name"] as? String {
                performCollectionMutation(mutation, name: name, scope: scope, locals: locals)
            } else if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                pendingPluginFailure = nil
                let flow = performActions(tryCatch["body"] as? [Any] ?? [], scope: scope, locals: locals)
                if flow != .normal { return flow }
                if let failure = pendingPluginFailure {
                    pendingPluginFailure = nil
                    if let caught = performPluginFailureCatch(
                        failure,
                        arms: tryCatch["error_catches"] as? [Any] ?? [],
                        scope: scope,
                        locals: locals
                    ) {
                        if caught != .normal { return caught }
                    } else if let catchBody = tryCatch["catch_body"] as? [Any] {
                        let catchFlow = performActions(catchBody, scope: scope, locals: locals)
                        if catchFlow != .normal { return catchFlow }
                    } else {
                        pendingPluginFailure = failure
                    }
                }
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
        applyCollectionMutation(mutation, name: name, scope: scope, arguments: arguments)
    }

    func devNativeEventHandler(
        _ actions: [Any],
        parameters: [String],
        scope: String,
        locals: [String: Any]
    ) -> ([Any]) -> Void {
        { [weak self] arguments in
            Task { @MainActor [weak self] in
                guard let self else { return }
                var eventLocals = locals
                for (name, value) in zip(parameters, arguments) { eventLocals[name] = value }
                do {
                    _ = try await self.performAsync(actions, scope: scope, locals: eventLocals)
                } catch {
                    NSLog("NexaDevRuntime native event action failed: %@", String(describing: error))
                }
            }
        }
    }

    @discardableResult
    func subscribeNativeEvent(
        receiver: Any,
        property: String,
        _ actions: [Any],
        parameters: [String],
        scope: String,
        locals: [String: Any]
    ) -> Bool {
        let nativeReceiver = receiver as AnyObject
        let callback = devNativeEventHandler(actions, parameters: parameters, scope: scope, locals: locals)
        guard NexaDevPluginBridge.subscribeInstanceEvent(
            receiver: receiver,
            property: property,
            handler: callback
        ) else { return false }
        nativeEventSubscriptions.removeAll { $0.receiver === nativeReceiver && $0.property == property }
        nativeEventSubscriptions.append(NexaDevNativeEventSubscription(
            receiver: nativeReceiver,
            property: property,
            actions: actions,
            parameters: parameters,
            scope: scope,
            locals: locals
        ))
        return true
    }

    func refreshNativeEventSubscriptions(module: [String: Any]) {
        guard !nativeEventSubscriptions.isEmpty else { return }
        let previous = nativeEventSubscriptions
        nativeEventSubscriptions.removeAll(keepingCapacity: true)
        for subscription in previous {
            _ = NexaDevPluginBridge.clearInstanceEvent(
                receiver: subscription.receiver,
                property: subscription.property
            )
        }
        var candidates: [[String: Any]] = []
        collectNativeEventSubscriptions(module, into: &candidates)
        for subscription in previous {
            var replacement: [String: Any]?
            for candidate in candidates where candidate["property"] as? String == subscription.property {
                guard let receiverExpression = candidate["receiver"],
                      let resolved = stableNativeEventReceiver(
                        receiverExpression,
                        locals: subscription.locals,
                        scope: subscription.scope
                      ) as? AnyObject,
                      resolved === subscription.receiver
                else { continue }
                replacement = candidate
            }
            guard let replacement else { continue }
            _ = subscribeNativeEvent(
                receiver: subscription.receiver,
                property: subscription.property,
                replacement["actions"] as? [Any] ?? [],
                parameters: replacement["parameters"] as? [String] ?? [],
                scope: subscription.scope,
                locals: subscription.locals
            )
        }
    }

    func clearNativeEventSubscriptions(scope: String? = nil) {
        let removed = nativeEventSubscriptions.filter { scope == nil || $0.scope == scope }
        nativeEventSubscriptions.removeAll { scope == nil || $0.scope == scope }
        for subscription in removed {
            _ = NexaDevPluginBridge.clearInstanceEvent(
                receiver: subscription.receiver,
                property: subscription.property
            )
        }
    }

    private func stableNativeEventReceiver(
        _ expression: Any,
        locals: [String: Any],
        scope: String
    ) -> Any? {
        guard let tagged = expression as? [String: Any],
              let (kind, payload) = tagged.first
        else { return nil }
        switch kind {
        case "State":
            return evaluate(expression, locals: locals, scope: scope)
        case "Member":
            guard let member = payload as? [String: Any],
                  let base = member["base"],
                  stableNativeEventReceiver(base, locals: locals, scope: scope) != nil
            else { return nil }
            return evaluate(expression, locals: locals, scope: scope)
        default:
            return nil
        }
    }

    private func collectNativeEventSubscriptions(_ value: Any, into result: inout [[String: Any]]) {
        if let array = value as? [Any] {
            for item in array { collectNativeEventSubscriptions(item, into: &result) }
        } else if let object = value as? [String: Any] {
            if let subscription = object["NativeEventSubscribe"] as? [String: Any] {
                result.append(subscription)
            }
            for nested in object.values { collectNativeEventSubscriptions(nested, into: &result) }
        }
    }

    private func applyCollectionMutation(
        _ mutation: [String: Any],
        name: String,
        scope: String,
        arguments: [Any]
    ) {
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
        case "Replace":
            // A collection state keeps its identity: only the contents are
            // replaced, which is what the generated code does too.
            if let replacement = arguments.first { setValue(name, value: replacement, scope: scope) }
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

    func performAsync(
        _ actions: [Any],
        scope: String,
        locals: [String: Any]
    ) async throws -> NexaDevActionFlow {
        for action in actions {
            if let flow = action as? String {
                if flow == "Break" { return .break }
                if flow == "Continue" { return .continue }
                continue
            }
            guard let tagged = action as? [String: Any] else { continue }
            if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                do {
                    let flow = try await performAsync(
                        tryCatch["body"] as? [Any] ?? [],
                        scope: scope,
                        locals: locals
                    )
                    if flow != .normal { return flow }
                } catch is CancellationError {
                    throw CancellationError()
                } catch {
                    if let failure = error as? NexaDevPluginFailure,
                       let flow = try await performPluginFailureCatchAsync(
                           failure,
                           arms: tryCatch["error_catches"] as? [Any] ?? [],
                           scope: scope,
                           locals: locals
                       ) {
                        if flow != .normal { return flow }
                    } else if let catchBody = tryCatch["catch_body"] as? [Any] {
                        let flow = try await performAsync(catchBody, scope: scope, locals: locals)
                        if flow != .normal { return flow }
                    } else {
                        NSLog("NexaDevRuntime async dev action failed: %@", String(describing: error))
                        throw error
                    }
                }
            } else if let assignment = tagged["Assign"] as? [String: Any],
               let name = assignment["name"] as? String,
               let expression = assignment["value"] {
                let value = try await evaluateAsync(expression, locals: locals, scope: scope)
                setValue(name, value: value, scope: scope)
                revision += 1
            } else if let assignment = tagged["NativePropertyAssign"] as? [String: Any],
                      let receiverExpression = assignment["receiver"],
                      let property = assignment["property"] as? String,
                      let valueExpression = assignment["value"] {
                let receiver = try await evaluateAsync(receiverExpression, locals: locals, scope: scope)
                let value = try await evaluateAsync(valueExpression, locals: locals, scope: scope)
                if !NexaDevPluginBridge.writeInstanceProperty(receiver: receiver, property: property, value: value) {
                    NSLog("NexaDevRuntime: native property write is unsupported: %@", property)
                }
            } else if let subscription = tagged["NativeEventSubscribe"] as? [String: Any],
                      let receiverExpression = subscription["receiver"],
                      let property = subscription["property"] as? String {
                let receiver = try await evaluateAsync(receiverExpression, locals: locals, scope: scope)
                if !subscribeNativeEvent(
                    receiver: receiver,
                    property: property,
                    subscription["actions"] as? [Any] ?? [],
                    parameters: subscription["parameters"] as? [String] ?? [],
                    scope: scope,
                    locals: locals
                ) {
                    NSLog("NexaDevRuntime: native event subscription is unsupported: %@", property)
                }
            } else if let branch = tagged["If"] as? [String: Any],
                      let condition = branch["condition"] {
                let value = try await evaluateAsync(condition, locals: locals, scope: scope)
                let selected = truthy(value)
                    ? branch["then_branch"] as? [Any]
                    : branch["else_branch"] as? [Any]
                let flow = try await performAsync(selected ?? [], scope: scope, locals: locals)
                if flow != .normal { return flow }
            } else if let expression = tagged["Expression"] {
                _ = try await evaluateAsync(expression, locals: locals, scope: scope)
            } else if let loop = tagged["For"] as? [String: Any],
                      let name = loop["name"] as? String,
                      let iterableExpression = loop["iterable"] {
                let evaluated = try await evaluateAsync(iterableExpression, locals: locals, scope: scope)
                let items: [Any]
                if let array = evaluated as? [Any] {
                    items = array
                } else if let set = evaluated as? Set<String> {
                    items = set.sorted()
                } else {
                    items = []
                }
                iterationLoop: for item in items {
                    var iterationLocals = locals
                    iterationLocals[name] = item
                    switch try await performAsync(
                        loop["body"] as? [Any] ?? [],
                        scope: scope,
                        locals: iterationLocals
                    ) {
                    case .break: break iterationLoop
                    case .continue, .normal: continue
                    }
                }
            } else if let loop = tagged["ForMap"] as? [String: Any],
                      let keyName = loop["key_name"] as? String,
                      let valueName = loop["value_name"] as? String,
                      let iterableExpression = loop["iterable"],
                      let map = try await evaluateAsync(iterableExpression, locals: locals, scope: scope) as? [String: Any] {
                iterationLoop: for key in map.keys.sorted() {
                    var iterationLocals = locals
                    iterationLocals[keyName] = key
                    iterationLocals[valueName] = map[key] ?? NSNull()
                    switch try await performAsync(
                        loop["body"] as? [Any] ?? [],
                        scope: scope,
                        locals: iterationLocals
                    ) {
                    case .break: break iterationLoop
                    case .continue, .normal: continue
                    }
                }
            } else if let loop = tagged["While"] as? [String: Any], let condition = loop["condition"] {
                iterationLoop: for _ in 0..<10_000 {
                    guard truthy(try await evaluateAsync(condition, locals: locals, scope: scope)) else { break }
                    switch try await performAsync(
                        loop["body"] as? [Any] ?? [],
                        scope: scope,
                        locals: locals
                    ) {
                    case .break: break iterationLoop
                    case .continue, .normal: continue
                    }
                }
            } else if let mutation = tagged["CollectionMutation"] as? [String: Any],
                      let name = mutation["name"] as? String {
                var arguments: [Any] = []
                for argument in mutation["arguments"] as? [Any] ?? [] {
                    arguments.append(try await evaluateAsync(argument, locals: locals, scope: scope))
                }
                applyCollectionMutation(mutation, name: name, scope: scope, arguments: arguments)
            } else if tagged["Break"] != nil {
                return .break
            } else if tagged["Continue"] != nil {
                return .continue
            }
        }
        return .normal
    }

    private func performPluginFailureCatch(
        _ failure: NexaDevPluginFailure,
        arms: [Any],
        scope: String,
        locals: [String: Any]
    ) -> NexaDevActionFlow? {
        for rawArm in arms {
            guard let arm = rawArm as? [String: Any],
                  arm["namespace"] as? String == failure.namespace,
                  arm["error_type"] as? String == failure.errorType,
                  arm["variant"] as? String == failure.variant
            else { continue }
            var catchLocals = locals
            for tuple in arm["parameters"] as? [[Any]] ?? [] where tuple.count >= 2 {
                guard let binding = tuple[0] as? String,
                      let property = tuple[1] as? String
                else { continue }
                catchLocals[binding] = failure.payload[property] ?? NSNull()
            }
            return performActions(arm["body"] as? [Any] ?? [], scope: scope, locals: catchLocals)
        }
        return nil
    }

    private func performPluginFailureCatchAsync(
        _ failure: NexaDevPluginFailure,
        arms: [Any],
        scope: String,
        locals: [String: Any]
    ) async throws -> NexaDevActionFlow? {
        for rawArm in arms {
            guard let arm = rawArm as? [String: Any],
                  arm["namespace"] as? String == failure.namespace,
                  arm["error_type"] as? String == failure.errorType,
                  arm["variant"] as? String == failure.variant
            else { continue }
            var catchLocals = locals
            for tuple in arm["parameters"] as? [[Any]] ?? [] where tuple.count >= 2 {
                guard let binding = tuple[0] as? String,
                      let property = tuple[1] as? String
                else { continue }
                catchLocals[binding] = failure.payload[property] ?? NSNull()
            }
            return try await performAsync(arm["body"] as? [Any] ?? [], scope: scope, locals: catchLocals)
        }
        return nil
    }
}
