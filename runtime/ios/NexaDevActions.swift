import Foundation
import SwiftUI

/// Flow control signal for synchronous Nexa dev action execution loops.
enum NexaDevActionFlow: Equatable {
    case normal
    case `break`
    case `continue`
}

/// Return flow is kept separate from UI actions so interpreted class methods
/// can propagate a value through nested branches and try/catch blocks.
enum NexaDevFunctionFlow {
    case normal
    case returned(Any)
    case `break`
    case `continue`
}

private struct NexaDevTaskPayload: @unchecked Sendable {
    let actions: [Any]
    let scope: String
    let locals: [String: Any]
}

@MainActor
extension NexaDevStateStore {
    func perform(_ actions: [Any], scope: String, locals: [String: Any]) {
        _ = performActions(actions, scope: scope, locals: locals)
    }

    func launchNativeTask(
        handle: String?,
        executor: String,
        actions: [Any],
        scope: String,
        locals: [String: Any]
    ) {
        let key: String
        if let handle, !handle.isEmpty {
            key = "\(scope)/task/\(handle)"
            foregroundTasks.removeValue(forKey: key)?.cancel()
        } else {
            key = "\(scope)/task/fire-and-forget/\(UUID().uuidString)"
        }
        let payload = NexaDevTaskPayload(actions: actions, scope: scope, locals: locals)
        let task: Task<Void, Never>
        if executor == "Background" {
            task = Task.detached { [weak self, payload] in
                guard let self else { return }
                do {
                    _ = try await self.performAsync(
                        payload.actions,
                        scope: payload.scope,
                        locals: payload.locals
                    )
                } catch is CancellationError {
                    return
                } catch {
                    NSLog("NexaDevRuntime background task failed: %@", String(describing: error))
                }
            }
        } else {
            task = Task { @MainActor [weak self, payload] in
                guard let self else { return }
                do {
                    _ = try await self.performAsync(
                        payload.actions,
                        scope: payload.scope,
                        locals: payload.locals
                    )
                } catch is CancellationError {
                    return
                } catch {
                    NSLog("NexaDevRuntime foreground task failed: %@", String(describing: error))
                }
            }
        }
        foregroundTasks[key] = task
    }

    func cancelNativeTask(handle: String, scope: String) {
        guard !handle.isEmpty else { return }
        foregroundTasks.removeValue(forKey: "\(scope)/task/\(handle)")?.cancel()
    }

    func clearNativeTasks(scope: String? = nil) {
        let prefix = scope.map { "\($0)/task/" }
        let selected = foregroundTasks.keys.filter { key in
            guard let prefix else { return true }
            return key.hasPrefix(prefix)
        }
        for key in selected {
            foregroundTasks.removeValue(forKey: key)?.cancel()
        }
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
                else if let set = NexaDevValueCodec.asSet(values) {
                    items = set.sorted { $0.stableOrderKey < $1.stableOrderKey }.map(\.value)
                }
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
            } else if let subscription = tagged["NetworkStatusSubscribe"] as? [String: Any],
                      let parameter = subscription["parameter"] as? String {
                subscribeNetworkStatus(
                    parameter: parameter,
                    actions: subscription["actions"] as? [Any] ?? [],
                    scope: scope,
                    locals: locals
                )
            } else if let task = tagged["TaskLaunch"] as? [String: Any] {
                launchNativeTask(
                    handle: task["handle"] as? String,
                    executor: task["executor"] as? String ?? "Main",
                    actions: task["actions"] as? [Any] ?? [],
                    scope: scope,
                    locals: locals
                )
            } else if let task = tagged["TaskCancel"] as? [String: Any],
                      let handle = task["handle"] as? String {
                cancelNativeTask(handle: handle, scope: scope)
            } else if let animated = tagged["WithAnimation"] as? [String: Any] {
                let actions = animated["actions"] as? [Any] ?? []
                let targets = animated["animated_states"] as? [String] ?? []
                let flow: NexaDevActionFlow
                if targets.isEmpty {
                    flow = performActions(actions, scope: scope, locals: locals)
                } else {
                    flow = withAnimation(nexaDevAnimation(animated["animation"])) {
                        performActions(actions, scope: scope, locals: locals)
                    }
                }
                if flow != .normal { return flow }
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

    func subscribeNetworkStatus(
        parameter: String,
        actions: [Any],
        scope: String,
        locals: [String: Any]
    ) {
        let id = "\(scope)/\(parameter)"
        let callback = devNativeEventHandler(actions, parameters: [parameter], scope: scope, locals: locals)
        NexaDevNetworkPathStatus.shared.setStatusHandler(id: id) { online in
            callback([online])
        }
        networkStatusSubscriptions.removeAll { $0.id == id }
        networkStatusSubscriptions.append(NexaDevNetworkStatusSubscription(
            id: id,
            parameter: parameter,
            actions: actions,
            scope: scope,
            locals: locals
        ))
    }

    func refreshNativeEventSubscriptions(module: [String: Any]) {
        let previousNetworkSubscriptions = networkStatusSubscriptions
        networkStatusSubscriptions.removeAll(keepingCapacity: true)
        for subscription in previousNetworkSubscriptions {
            NexaDevNetworkPathStatus.shared.setStatusHandler(id: subscription.id, handler: nil)
            if let replacement = networkStatusAction(
                in: module,
                scope: subscription.scope,
                parameter: subscription.parameter
            ) {
                subscribeNetworkStatus(
                    parameter: subscription.parameter,
                    actions: replacement["actions"] as? [Any] ?? [],
                    scope: subscription.scope,
                    locals: subscription.locals
                )
            }
        }
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

        if let actions = module["on_appear"] as? [Any] {
            installLifecycleNativeEventSubscriptions(actions, scope: "app", locals: [:])
        }
        let screens = module["screens"] as? [[String: Any]] ?? []
        for (scope, parameters) in activeScreenParameters {
            guard let screenName = scope.split(separator: "/").last.map(String.init),
                  let screen = screens.first(where: { $0["name"] as? String == screenName }),
                  let actions = screen["on_appear"] as? [Any]
            else { continue }
            installLifecycleNativeEventSubscriptions(actions, scope: scope, locals: parameters)
        }
    }

    private func installLifecycleNativeEventSubscriptions(
        _ actions: [Any],
        scope: String,
        locals: [String: Any]
    ) {
        for raw in actions {
            guard let tagged = raw as? [String: Any], let (kind, payload) = tagged.first else { continue }
            let fields = payload as? [String: Any] ?? [:]
            if kind == "NativeEventSubscribe",
               let receiverExpression = fields["receiver"],
               let property = fields["property"] as? String,
               let receiver = stableNativeEventReceiver(receiverExpression, locals: locals, scope: scope) {
                let alreadyInstalled = nativeEventSubscriptions.contains {
                    $0.receiver === (receiver as AnyObject) && $0.property == property
                }
                if !alreadyInstalled {
                    _ = subscribeNativeEvent(
                        receiver: receiver,
                        property: property,
                        fields["actions"] as? [Any] ?? [],
                        parameters: fields["parameters"] as? [String] ?? [],
                        scope: scope,
                        locals: locals
                    )
                }
                continue
            }
            if kind == "NetworkStatusSubscribe",
               let parameter = fields["parameter"] as? String {
                let id = "\(scope)/\(parameter)"
                if !networkStatusSubscriptions.contains(where: { $0.id == id }) {
                    subscribeNetworkStatus(
                        parameter: parameter,
                        actions: fields["actions"] as? [Any] ?? [],
                        scope: scope,
                        locals: locals
                    )
                }
                continue
            }
            if kind == "If", let condition = fields["condition"] {
                let selected = truthy(evaluate(condition, locals: locals, scope: scope))
                    ? fields["then_branch"] as? [Any]
                    : fields["else_branch"] as? [Any]
                if let selected {
                    installLifecycleNativeEventSubscriptions(selected, scope: scope, locals: locals)
                }
            } else if kind == "For",
                      let name = fields["name"] as? String,
                      let iterable = fields["iterable"] {
                let values = evaluate(iterable, locals: locals, scope: scope)
                let items = (values as? [Any]) ?? (values as? Set<NexaDevHashableValue>)?.map(\.value) ?? []
                for item in items {
                    var iterationLocals = locals
                    iterationLocals[name] = item
                    installLifecycleNativeEventSubscriptions(
                        fields["body"] as? [Any] ?? [], scope: scope, locals: iterationLocals
                    )
                }
            } else if kind == "ForMap",
                      let keyName = fields["key_name"] as? String,
                      let valueName = fields["value_name"] as? String,
                      let iterable = fields["iterable"] {
                let values = evaluate(iterable, locals: locals, scope: scope) as? [String: Any] ?? [:]
                for key in values.keys.sorted() {
                    var iterationLocals = locals
                    iterationLocals[keyName] = key
                    iterationLocals[valueName] = values[key] ?? NSNull()
                    installLifecycleNativeEventSubscriptions(
                        fields["body"] as? [Any] ?? [], scope: scope, locals: iterationLocals
                    )
                }
            } else if kind == "While", let condition = fields["condition"] {
                for _ in 0..<10_000 {
                    guard truthy(evaluate(condition, locals: locals, scope: scope)) else { break }
                    installLifecycleNativeEventSubscriptions(
                        fields["body"] as? [Any] ?? [], scope: scope, locals: locals
                    )
                }
            }
        }
    }

    func clearNativeEventSubscriptions(scope: String? = nil) {
        let removedNetwork = networkStatusSubscriptions.filter { scope == nil || $0.scope == scope }
        networkStatusSubscriptions.removeAll { scope == nil || $0.scope == scope }
        for subscription in removedNetwork {
            NexaDevNetworkPathStatus.shared.setStatusHandler(id: subscription.id, handler: nil)
        }
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

    private func networkStatusAction(
        in module: [String: Any],
        scope: String,
        parameter: String
    ) -> [String: Any]? {
        let roots: [Any]
        if scope == "app" {
            roots = ["body", "on_appear", "on_active", "on_inactive", "on_background"]
                .compactMap { module[$0] }
        } else if scope.hasPrefix("screen/"),
                  let screenName = scope.split(separator: "/").last.map(String.init),
                  let screen = (module["screens"] as? [[String: Any]] ?? [])
                    .first(where: { $0["name"] as? String == screenName }) {
            roots = ["body", "on_appear", "on_disappear"].compactMap { screen[$0] }
        } else if scope.hasPrefix("component/"),
                  let componentName = scope.split(separator: "/").last.map(String.init),
                  let component = (module["components"] as? [[String: Any]] ?? [])
                    .first(where: { $0["name"] as? String == componentName }) {
            roots = [component["body"]].compactMap { $0 }
        } else {
            roots = [module]
        }
        for root in roots {
            if let action = findNetworkStatusAction(in: root, parameter: parameter) {
                return action
            }
        }
        return nil
    }

    private func findNetworkStatusAction(in value: Any, parameter: String) -> [String: Any]? {
        if let array = value as? [Any] {
            for item in array {
                if let match = findNetworkStatusAction(in: item, parameter: parameter) { return match }
            }
        } else if let object = value as? [String: Any] {
            if let subscription = object["NetworkStatusSubscribe"] as? [String: Any],
               subscription["parameter"] as? String == parameter {
                return subscription
            }
            for nested in object.values {
                if let match = findNetworkStatusAction(in: nested, parameter: parameter) { return match }
            }
        }
        return nil
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
        case "ArrayMove":
            var array = value(name, scope: scope) as? [Any] ?? []
            if arguments.count >= 2,
               let from = (arguments[0] as? NSNumber)?.intValue,
               let to = (arguments[1] as? NSNumber)?.intValue,
               array.indices.contains(from), array.indices.contains(to), from != to {
                let item = array.remove(at: from)
                array.insert(item, at: to)
            }
            setValue(name, value: array, scope: scope)
        case "ArrayMoveSubset":
            var array = value(name, scope: scope) as? [Any] ?? []
            guard arguments.count >= 3,
                  let from = (arguments[0] as? NSNumber)?.intValue,
                  let to = (arguments[1] as? NSNumber)?.intValue,
                  let subset = arguments[2] as? [Any],
                  subset.indices.contains(from), subset.indices.contains(to), from != to else { return }
            var reordered = subset
            let moved = reordered.remove(at: from)
            reordered.insert(moved, at: to)
            var cursor = 0
            for index in array.indices where cursor < subset.count {
                if NexaDevHashableValue(array[index]) == NexaDevHashableValue(subset[cursor]) {
                    array[index] = reordered[cursor]
                    cursor += 1
                }
            }
            if cursor == subset.count { setValue(name, value: array, scope: scope) }
        case "SetInsert", "SetRemove":
            var set = NexaDevValueCodec.asSet(value(name, scope: scope)) ?? []
            if let item = arguments.first.map(NexaDevHashableValue.init) {
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
            } else if let subscription = tagged["NetworkStatusSubscribe"] as? [String: Any],
                      let parameter = subscription["parameter"] as? String {
                subscribeNetworkStatus(
                    parameter: parameter,
                    actions: subscription["actions"] as? [Any] ?? [],
                    scope: scope,
                    locals: locals
                )
            } else if let task = tagged["TaskLaunch"] as? [String: Any] {
                launchNativeTask(
                    handle: task["handle"] as? String,
                    executor: task["executor"] as? String ?? "Main",
                    actions: task["actions"] as? [Any] ?? [],
                    scope: scope,
                    locals: locals
                )
            } else if let task = tagged["TaskCancel"] as? [String: Any],
                      let handle = task["handle"] as? String {
                cancelNativeTask(handle: handle, scope: scope)
            } else if let animated = tagged["WithAnimation"] as? [String: Any] {
                let actions = animated["actions"] as? [Any] ?? []
                let targets = animated["animated_states"] as? [String] ?? []
                let flow: NexaDevActionFlow
                if targets.isEmpty {
                    flow = performActions(actions, scope: scope, locals: locals)
                } else {
                    flow = withAnimation(nexaDevAnimation(animated["animation"])) {
                        performActions(actions, scope: scope, locals: locals)
                    }
                }
                if flow != .normal { return flow }
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
                } else if let set = NexaDevValueCodec.asSet(evaluated) {
                    items = set.sorted { $0.stableOrderKey < $1.stableOrderKey }.map(\.value)
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

    func performFunctionActions(
        _ actions: [Any],
        scope: String,
        locals: [String: Any]
    ) -> NexaDevFunctionFlow {
        var functionLocals = locals
        for action in actions {
            guard let tagged = action as? [String: Any] else { continue }
            if let binding = tagged["Let"] as? [String: Any],
               let name = binding["name"] as? String,
               let value = binding["value"] {
                functionLocals[name] = evaluate(value, locals: functionLocals, scope: scope)
            } else if let returned = tagged["Return"] as? [String: Any],
                      let value = returned["value"] {
                return .returned(evaluate(value, locals: functionLocals, scope: scope))
            } else if let expression = tagged["Expression"] {
                _ = evaluate(expression, locals: functionLocals, scope: scope)
            } else if let branch = tagged["If"] as? [String: Any],
                      let condition = branch["condition"] {
                let selected = truthy(evaluate(condition, locals: functionLocals, scope: scope))
                    ? branch["then_branch"] as? [Any]
                    : branch["else_branch"] as? [Any]
                let flow = performFunctionActions(selected ?? [], scope: scope, locals: functionLocals)
                switch flow {
                case .normal: break
                case .returned, .break, .continue: return flow
                }
            } else if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                pendingPluginFailure = nil
                let flow = performFunctionActions(
                    tryCatch["body"] as? [Any] ?? [],
                    scope: scope,
                    locals: functionLocals
                )
                let failure = pendingPluginFailure
                pendingPluginFailure = nil
                if let failure {
                    if let caught = performFunctionPluginFailureCatch(
                        failure,
                        arms: tryCatch["error_catches"] as? [Any] ?? [],
                        scope: scope,
                        locals: functionLocals
                    ) {
                        switch caught {
                        case .normal: break
                        case .returned, .break, .continue: return caught
                        }
                    } else if let catchBody = tryCatch["catch_body"] as? [Any] {
                        let catchFlow = performFunctionActions(catchBody, scope: scope, locals: functionLocals)
                        switch catchFlow {
                        case .normal: break
                        case .returned, .break, .continue: return catchFlow
                        }
                    } else {
                        pendingPluginFailure = failure
                    }
                } else {
                    switch flow {
                    case .normal: break
                    case .returned, .break, .continue: return flow
                    }
                }
            } else if let loop = tagged["For"] as? [String: Any],
                      let name = loop["name"] as? String,
                      let iterable = loop["iterable"] {
                let evaluated = evaluate(iterable, locals: functionLocals, scope: scope)
                let items: [Any]
                if let array = evaluated as? [Any] { items = array }
                else if let set = NexaDevValueCodec.asSet(evaluated) {
                    items = set.sorted { $0.stableOrderKey < $1.stableOrderKey }.map(\.value)
                } else { items = [] }
                iteration: for item in items {
                    var iterationLocals = functionLocals
                    iterationLocals[name] = item
                    switch performFunctionActions(loop["body"] as? [Any] ?? [], scope: scope, locals: iterationLocals) {
                    case .normal, .continue: continue
                    case .break: break iteration
                    case .returned(let value): return .returned(value)
                    }
                }
            } else if tagged["Break"] != nil {
                return .break
            } else if tagged["Continue"] != nil {
                return .continue
            }
            if pendingPluginFailure != nil { return .normal }
        }
        return .normal
    }

    private func performFunctionPluginFailureCatch(
        _ failure: NexaDevPluginFailure,
        arms: [Any],
        scope: String,
        locals: [String: Any]
    ) -> NexaDevFunctionFlow? {
        for rawArm in arms {
            guard let arm = rawArm as? [String: Any],
                  arm["namespace"] as? String == failure.namespace,
                  arm["error_type"] as? String == failure.errorType,
                  arm["variant"] as? String == failure.variant
            else { continue }
            var catchLocals = locals
            for tuple in arm["parameters"] as? [[Any]] ?? [] where tuple.count >= 2 {
                guard let name = tuple[0] as? String, let property = tuple[1] as? String else { continue }
                catchLocals[name] = failure.payload[property] ?? NSNull()
            }
            return performFunctionActions(arm["body"] as? [Any] ?? [], scope: scope, locals: catchLocals)
        }
        return nil
    }

    func performFunctionAsync(
        _ actions: [Any],
        scope: String,
        locals: [String: Any]
    ) async throws -> NexaDevFunctionFlow {
        var functionLocals = locals
        for action in actions {
            guard let tagged = action as? [String: Any] else { continue }
            if let binding = tagged["Let"] as? [String: Any],
               let name = binding["name"] as? String,
               let value = binding["value"] {
                functionLocals[name] = try await evaluateAsync(value, locals: functionLocals, scope: scope)
            } else if let returned = tagged["Return"] as? [String: Any],
                      let value = returned["value"] {
                return .returned(try await evaluateAsync(value, locals: functionLocals, scope: scope))
            } else if let branch = tagged["If"] as? [String: Any],
                      let condition = branch["condition"] {
                let value = try await evaluateAsync(condition, locals: functionLocals, scope: scope)
                let selected = truthy(value)
                    ? branch["then_branch"] as? [Any]
                    : branch["else_branch"] as? [Any]
                let flow = try await performFunctionAsync(selected ?? [], scope: scope, locals: functionLocals)
                switch flow {
                case .normal: break
                case .returned, .break, .continue: return flow
                }
            } else if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                do {
                    let flow = try await performFunctionAsync(
                        tryCatch["body"] as? [Any] ?? [],
                        scope: scope,
                        locals: functionLocals
                    )
                    switch flow {
                    case .normal: break
                    case .returned, .break, .continue: return flow
                    }
                } catch is CancellationError {
                    throw CancellationError()
                } catch let failure as NexaDevPluginFailure {
                    if let flow = try await performFunctionPluginFailureCatchAsync(
                        failure,
                        arms: tryCatch["error_catches"] as? [Any] ?? [],
                        scope: scope,
                        locals: functionLocals
                    ) {
                        switch flow {
                        case .normal: break
                        case .returned, .break, .continue: return flow
                        }
                    } else if let catchBody = tryCatch["catch_body"] as? [Any] {
                        let flow = try await performFunctionAsync(catchBody, scope: scope, locals: functionLocals)
                        switch flow {
                        case .normal: break
                        case .returned, .break, .continue: return flow
                        }
                    } else {
                        throw failure
                    }
                } catch {
                    if let catchBody = tryCatch["catch_body"] as? [Any] {
                        let flow = try await performFunctionAsync(catchBody, scope: scope, locals: functionLocals)
                        switch flow {
                        case .normal: break
                        case .returned, .break, .continue: return flow
                        }
                    } else {
                        throw error
                    }
                }
            } else if let expression = tagged["Expression"] {
                _ = try await evaluateAsync(expression, locals: functionLocals, scope: scope)
            } else if let loop = tagged["For"] as? [String: Any],
                      let name = loop["name"] as? String,
                      let iterable = loop["iterable"] {
                let value = try await evaluateAsync(iterable, locals: functionLocals, scope: scope)
                let items: [Any]
                if let array = value as? [Any] {
                    items = array
                } else if let set = NexaDevValueCodec.asSet(value) {
                    items = set.sorted { $0.stableOrderKey < $1.stableOrderKey }.map(\.value)
                } else {
                    items = []
                }
                iteration: for item in items {
                    var iterationLocals = functionLocals
                    iterationLocals[name] = item
                    switch try await performFunctionAsync(loop["body"] as? [Any] ?? [], scope: scope, locals: iterationLocals) {
                    case .normal, .continue: continue
                    case .break: break iteration
                    case .returned(let value): return .returned(value)
                    }
                }
            } else if let loop = tagged["ForMap"] as? [String: Any],
                      let keyName = loop["key_name"] as? String,
                      let valueName = loop["value_name"] as? String,
                      let iterable = loop["iterable"],
                      let map = try await evaluateAsync(iterable, locals: functionLocals, scope: scope) as? [String: Any] {
                iteration: for key in map.keys.sorted() {
                    var iterationLocals = functionLocals
                    iterationLocals[keyName] = key
                    iterationLocals[valueName] = map[key] ?? NSNull()
                    switch try await performFunctionAsync(loop["body"] as? [Any] ?? [], scope: scope, locals: iterationLocals) {
                    case .normal, .continue: continue
                    case .break: break iteration
                    case .returned(let value): return .returned(value)
                    }
                }
            } else if let loop = tagged["While"] as? [String: Any],
                      let condition = loop["condition"] {
                iteration: for _ in 0..<10_000 {
                    guard truthy(try await evaluateAsync(condition, locals: functionLocals, scope: scope)) else { break }
                    switch try await performFunctionAsync(loop["body"] as? [Any] ?? [], scope: scope, locals: functionLocals) {
                    case .normal, .continue: continue
                    case .break: break iteration
                    case .returned(let value): return .returned(value)
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

    private func performFunctionPluginFailureCatchAsync(
        _ failure: NexaDevPluginFailure,
        arms: [Any],
        scope: String,
        locals: [String: Any]
    ) async throws -> NexaDevFunctionFlow? {
        for rawArm in arms {
            guard let arm = rawArm as? [String: Any],
                  arm["namespace"] as? String == failure.namespace,
                  arm["error_type"] as? String == failure.errorType,
                  arm["variant"] as? String == failure.variant
            else { continue }
            var catchLocals = locals
            for tuple in arm["parameters"] as? [[Any]] ?? [] where tuple.count >= 2 {
                guard let name = tuple[0] as? String, let property = tuple[1] as? String else { continue }
                catchLocals[name] = failure.payload[property] ?? NSNull()
            }
            return try await performFunctionAsync(arm["body"] as? [Any] ?? [], scope: scope, locals: catchLocals)
        }
        return nil
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

private func nexaDevAnimation(_ value: Any?) -> Animation? {
    if let kind = value as? String {
        switch kind {
        case "EaseIn": return .easeIn
        case "EaseOut": return .easeOut
        case "EaseInOut": return .easeInOut
        case "Linear": return .linear
        default: return .default
        }
    }
    guard let tagged = value as? [String: Any], let (kind, payload) = tagged.first else {
        return .default
    }
    if kind == "Spring", let fields = payload as? [String: Any] {
        let response = fields["response"] as? Double ?? 0.5
        let damping = fields["damping"] as? Double ?? 0.825
        return .spring(response: response, dampingFraction: damping)
    }
    return nexaDevAnimation(kind)
}
