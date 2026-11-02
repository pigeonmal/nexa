import Foundation
import CoreFoundation
import UIKit
import SwiftUI

/// Type-erased SQL row used only by the debug interpreter. Release builds map
/// the same query directly into the declared application struct.
struct NexaDevDynamicRow: @unchecked Sendable {
    let fields: [String: Any]
}

struct NexaDevLocatedRuntimeError: Error, LocalizedError {
    let sourceFile: String?
    let line: Int?
    let column: Int?
    let underlying: Error

    init(sourceFile: String, line: Int, column: Int, underlying: Error) {
        self.sourceFile = sourceFile
        self.line = line
        self.column = column
        self.underlying = underlying
    }

    var errorDescription: String? { underlying.localizedDescription }
}

private func attachSourceLocation(
    file: String,
    line: Int,
    column: Int,
    to failure: inout NexaDevPluginFailure
) {
    // Located wrappers can nest when one throwing call is an argument to
    // another. Keep the first (innermost) call site instead of replacing it
    // while the same failure unwinds through outer expressions.
    guard failure.sourceFile == nil else { return }
    failure.sourceFile = file
    failure.sourceLine = line
    failure.sourceColumn = column
}

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

struct NexaDevForegroundTask {
    let id: String
    let task: Task<Void, Never>
    let owner: String?
}

/// Stable debug-runtime handle for one reusable `.nx` component instance.
final class NexaDevComponentRef {
    let componentName: String
    let identity: String
    private(set) var attachedScope: String?
    private(set) var attachedParameters: [String: Any] = [:]

    init(componentName: String, identity: String) {
        self.componentName = componentName
        self.identity = identity
    }

    func attach(scope: String, parameters: [String: Any]) {
        attachedScope = scope
        attachedParameters = parameters
    }

    func detach() {
        attachedScope = nil
        attachedParameters = [:]
    }
}

@MainActor
final class NexaDevScopeRevision: ObservableObject {
    @Published private(set) var revision = 0

    func invalidate() {
        revision += 1
    }
}

enum NexaDevPatchInstallResult: Equatable {
    case render
    case functionsOnly
    case screenOnly
    case fullInstallRequired
}

@MainActor
final class NexaDevStateStore: ObservableObject {
    private static let presentationPatchFields: Set<String> = [
        "text", "label", "placeholder", "icon", "icon_spec", "source", "tint",
        "color", "background", "border_color", "border_width", "corner_radius",
        "padding", "padding_horizontal", "padding_vertical", "width", "height",
        "min_width", "max_width", "min_height", "max_height", "opacity", "alignment",
        "horizontal_arrangement", "vertical_arrangement", "font_size", "font_weight",
        "font_family", "font_style", "font_design", "line_limit", "line_height",
        "letter_spacing", "strikethrough", "italic", "underline", "text_alignment",
        "scale", "scale_y", "rotation",
        "offset_y", "accessibility_label", "style", "value",
    ]
    var onRuntimeFailure: ((Error, String) -> Void)?
    @Published var revision = 0
    /// Advances only when a new module revision is installed. The test-only
    /// frame fence uses it so ordinary state mutations cannot masquerade as a
    /// rendered module update.
    @Published private(set) var moduleGeneration = 0
    @Published private(set) var appLifecycleEpoch = 0
    @Published var languageOverride: String? = UserDefaults.standard.string(forKey: "dev.nexa.locale.override")
    @Published private var navigationPaths: [String: [NexaDevRoute]] = [:]
    /// Disabled by default; the dev console can opt into value-free trace events.
    var runtimeTraceEnabled = false
    var currentModule: [String: Any] = [:]
    @Published private(set) var focusedFieldKey: String?
    var values: [String: Any] = [:]
    /// Native component handles survive module replacement independently of
    /// app state values, while their key includes the owning UI scope.
    var componentRefs: [String: Any] = [:]
    /// Process-lifetime immutable module bindings. These are deliberately kept
    /// outside UI state identities so class singletons survive view updates.
    var moduleValues: [String: Any] = [:]
    private var moduleValueSignatures: [String: String] = [:]
    var typeSignatures: [String: String] = [:]
    var functions: [String: [String: Any]] = [:]
    var structs: [String: [[String: Any]]] = [:]
    var enumCases: [String: [String]] = [:]
    var routeArguments: [String: (screen: String, values: [String: Any], signature: String)] = [:]
    var routeInstanceTokens = Set<String>()
    private var navigationRoot: String?
    private var navigationScreensSignature: String?
    var activeFunctions = Set<String>()
    var focusBindings: [String: (scope: String, state: String?)] = [:]
    var nativeEventSubscriptions: [NexaDevNativeEventSubscription] = []
    var networkStatusSubscriptions: [NexaDevNetworkStatusSubscription] = []
    var foregroundTasks: [String: NexaDevForegroundTask] = [:]
    var activeScreenParameters: [String: [String: Any]] = [:]
    var pendingPluginFailure: NexaDevPluginFailure?
    var hasInstalledModule = false
    private var hotTranslations: [String: [String: Any]] = [:]
    private var scopeRevisions: [String: NexaDevScopeRevision] = [:]
    private var stateReaders: [String: Set<String>] = [:]
    private var stateCommandRevisions: [String: Int] = [:]
    private var fastListEndReachedCounts: [String: Int] = [:]
    private var fastListEndReachedKeys = Set<String>()

    func revisionObserver(for scope: String) -> NexaDevScopeRevision {
        if let observer = scopeRevisions[scope] { return observer }
        let observer = NexaDevScopeRevision()
        scopeRevisions[scope] = observer
        return observer
    }

    func invalidateScope(_ scope: String) {
        scopeRevisions[scope]?.invalidate()
    }

    func updateFastListEndReachedCount(identity: String, scope: String, count: Int) {
        let key = "\(scope)|\(identity)"
        guard fastListEndReachedCounts[key] != count else { return }
        fastListEndReachedCounts[key] = count
        fastListEndReachedKeys.remove(key)
    }

    func claimFastListEndReached(identity: String, scope: String, count: Int) -> Bool {
        guard count > 0 else { return false }
        let key = "\(scope)|\(identity)"
        if fastListEndReachedCounts[key] != count {
            fastListEndReachedCounts[key] = count
            fastListEndReachedKeys.remove(key)
        }
        return fastListEndReachedKeys.insert(key).inserted
    }

    func invalidateAllScopes() {
        revision += 1
        for observer in scopeRevisions.values { observer.invalidate() }
    }

    func setLanguageOverride(_ code: String?) {
        let trimmed = code?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if trimmed.isEmpty {
            UserDefaults.standard.removeObject(forKey: "dev.nexa.locale.override")
            languageOverride = nil
        } else {
            let locale = Locale(identifier: trimmed.replacingOccurrences(of: "_", with: "-"))
            let ident = (locale.language.languageCode?.identifier != "und") ? locale.identifier : trimmed
            UserDefaults.standard.set(ident, forKey: "dev.nexa.locale.override")
            languageOverride = ident
        }
        invalidateAllScopes()
        objectWillChange.send()
    }

    private func invalidateReaders(of key: String, sourceScope: String) {
        revision += 1
        var readers = stateReaders[key] ?? []
        readers.insert(sourceScope)
        for scope in readers { invalidateScope(scope) }
    }

    func reportRuntimeFailure(_ error: Error, stackTrace: String? = nil) {
        let capturedStack = stackTrace ?? Thread.callStackSymbols.joined(separator: "\n")
        onRuntimeFailure?(error, capturedStack)
    }

    func installTranslations(_ raw: Any?) {
        hotTranslations = raw as? [String: [String: Any]] ?? [:]
        invalidateAllScopes()
    }

    func localizedText(
        key: String,
        fallback: String,
        sourceExpression: Any? = nil,
        locals: [String: Any] = [:],
        scope: String = "app"
    ) -> String {
        var translated: Any?
        let override = UserDefaults.standard.string(forKey: "dev.nexa.locale.override")
        let preferredLanguages = override.map { code in
            [code] + Locale.preferredLanguages.filter {
                $0.caseInsensitiveCompare(code) != .orderedSame
                    && $0.split(whereSeparator: { $0 == "-" || $0 == "_" }).first?.lowercased() != nexaCurrentLanguageCode()
            }
        } ?? Locale.preferredLanguages
        for preferred in preferredLanguages + [override ?? Locale.current.identifier] {
            let language = preferred.replacingOccurrences(of: "_", with: "-")
            if let value = hotTranslations[language]?[key] {
                translated = value
                break
            }
            let base = String(language.split(separator: "-").first ?? "")
            if let value = hotTranslations[base]?[key] {
                translated = value
                break
            }
        }
        guard let translated else { return fallback }
        let arguments = localizedArguments(sourceExpression, locals: locals, scope: scope)
        let count = arguments.first { $0.name == "count" }.map { Int(number($0.value)) }
        let template: String
        if let value = translated as? String {
            template = value
        } else if let forms = translated as? [String: String] {
            let language = Locale(identifier: override ?? Locale.current.identifier)
                .language.languageCode?.identifier ?? "en"
            let category = Self.pluralCategory(count ?? 0, language: language)
            template = forms[category] ?? forms["other"] ?? forms.values.first ?? fallback
        } else {
            return fallback
        }
        return arguments.reduce(template) { value, argument in
            value.replacingOccurrences(of: "{\(argument.name)}", with: argument.formatted)
        }
    }

    private func localizedInterpolationText(
        _ sourceExpression: Any,
        locals: [String: Any],
        scope: String
    ) -> String {
        guard let tagged = sourceExpression as? [String: Any],
              let parts = tagged["Interpolation"] as? [[String: Any]]
        else { return stringify(evaluate(sourceExpression, locals: locals, scope: scope)) }
        return parts.map { part in
            if let literal = part["Literal"] as? String { return literal }
            guard let raw = part["Value"] else { return "" }
            let value = evaluate(raw, locals: locals, scope: scope)
            return localizedArgumentText(raw, value: value)
        }.joined()
    }

    private func localizedArguments(
        _ sourceExpression: Any?,
        locals: [String: Any],
        scope: String
    ) -> [(name: String, value: Any, formatted: String)] {
        guard let tagged = sourceExpression as? [String: Any],
              let parts = tagged["Interpolation"] as? [[String: Any]]
        else { return [] }
        return parts.enumerated().compactMap { index, part in
            guard let raw = part["Value"] else { return nil }
            let name = Self.localizedArgumentName(raw, index: index)
            let value = evaluate(raw, locals: locals, scope: scope)
            return (name, value, localizedArgumentText(raw, value: value))
        }
    }

    private func localizedArgumentText(_ expression: Any, value: Any) -> String {
        guard let type = Self.localizedNumericType(expression) else { return stringify(value) }
        let numeric = number(value)
        let formatted = type == "Float32" ? Double(Float(numeric)) : numeric
        return String(format: "%f", locale: Locale.current, arguments: [formatted])
    }

    private static func localizedNumericType(_ raw: Any) -> String? {
        guard let tagged = raw as? [String: Any], let (kind, payload) = tagged.first else { return nil }
        let typeName: String?
        switch kind {
        case "Number":
            typeName = (payload as? [String: Any])?["ty"] as? String
        case "State", "AnimatedState":
            let fields = payload as? [Any] ?? []
            typeName = fields.count > 1 ? localizedNumericTypeName(fields[1]) : nil
        case "Member":
            typeName = localizedNumericTypeName((payload as? [String: Any])?["field_type"])
        case "Index":
            typeName = localizedNumericTypeName((payload as? [String: Any])?["element_type"])
        case "Call", "NativeCall", "TimeCall":
            typeName = localizedNumericTypeName((payload as? [String: Any])?["return_type"])
        case "Conditional":
            typeName = localizedNumericTypeName((payload as? [String: Any])?["value_type"])
        case "Add":
            let fields = payload as? [Any] ?? []
            typeName = fields.count > 2 ? fields[2] as? String : nil
        case "Arithmetic", "Negate":
            typeName = (payload as? [String: Any])?["ty"] as? String
        default:
            typeName = nil
        }
        return typeName == "Float32" || typeName == "Float64" ? typeName : nil
    }

    private static func localizedNumericTypeName(_ raw: Any?) -> String? {
        if let raw = raw as? String { return raw }
        return (raw as? [String: Any])?["Numeric"] as? String
    }

    private static func localizedArgumentName(_ raw: Any, index: Int) -> String {
        guard let tagged = raw as? [String: Any], let (kind, payload) = tagged.first else {
            return "value\(index + 1)"
        }
        if kind == "State" || kind == "AnimatedState",
           let values = payload as? [Any], let name = values.first as? String {
            return String(name.split(separator: "::").last ?? Substring(name))
        }
        if kind == "Member", let fields = payload as? [String: Any], let name = fields["name"] as? String {
            return name
        }
        return "value\(index + 1)"
    }

    private static func pluralCategory(_ count: Int, language: String) -> String {
        let n = abs(count)
        switch language {
        case "ar":
            if n == 0 { return "zero" }
            if n == 1 { return "one" }
            if n == 2 { return "two" }
            if (3...10).contains(n % 100) { return "few" }
            if (11...99).contains(n % 100) { return "many" }
            return "other"
        case "ru", "uk", "be":
            if n % 10 == 1 && n % 100 != 11 { return "one" }
            if (2...4).contains(n % 10) && !(12...14).contains(n % 100) { return "few" }
            if n % 10 == 0 || (11...14).contains(n % 100) { return "many" }
            return "other"
        case "pl":
            if n == 1 { return "one" }
            if (2...4).contains(n % 10) && !(12...14).contains(n % 100) { return "few" }
            return "many"
        case "cs", "sk":
            if n == 1 { return "one" }
            if (2...4).contains(n) { return "few" }
            return "other"
        case "fr", "pt": return n <= 1 ? "one" : "other"
        default: return n == 1 ? "one" : "other"
        }
    }

    private func installFunctionRegistry(_ module: [String: Any]) {
        let functionDeclarations = module["functions"] as? [[String: Any]] ?? []
        let componentDeclarations = module["components"] as? [[String: Any]] ?? []
        var installedFunctions: [String: [String: Any]] = [:]
        installedFunctions.reserveCapacity(functionDeclarations.count)
        for function in functionDeclarations {
            guard let name = function["name"] as? String else { continue }
            installedFunctions[name] = function
        }
        for component in componentDeclarations {
            for method in component[NexaDevKeys.refMethods] as? [[String: Any]] ?? [] {
                guard let name = method["name"] as? String else { continue }
                installedFunctions[name] = method
            }
        }
        functions = installedFunctions
    }

    func screen(named name: String) -> [String: Any]? {
        let screens = currentModule["screens"] as? [[String: Any]] ?? []
        return screens.first(where: { $0["name"] as? String == name })
    }

    func installPatchedModule(
        _ module: [String: Any],
        operations: [[String: Any]]
    ) -> NexaDevPatchInstallResult {
        if operations.isEmpty || isPresentationPatch(operations) {
            currentModule = module
            moduleGeneration += 1
            return .render
        }
        if isFunctionBodyPatch(operations) {
            if !installPatchedFunctions(module, operations: operations) {
                installFunctionRegistry(module)
            }
            currentModule = module
            moduleGeneration += 1
            return .functionsOnly
        }
        if let affectedScreens = screenScopedIndices(operations) {
            installPatchedScreens(module, screenIndices: affectedScreens)
            currentModule = module
            moduleGeneration += 1
            return .screenOnly
        }
        return .fullInstallRequired
    }

    /// Update only changed function entries for body-only hot reloads.
    private func installPatchedFunctions(
        _ module: [String: Any],
        operations: [[String: Any]]
    ) -> Bool {
        guard let declarations = module["functions"] as? [[String: Any]] else { return false }
        for operation in operations {
            let segments = pointerSegments(operation["path"] as? String ?? "")
            guard segments.count >= 3,
                  segments[0] == "functions",
                  (segments[2] == "body" || segments[2] == "body_actions"),
                  let index = Int(segments[1]), declarations.indices.contains(index),
                  let name = declarations[index]["name"] as? String
            else { return false }
            functions[name] = declarations[index]
        }
        return true
    }

    private func pointerSegments(_ path: String) -> [String] {
        guard path.hasPrefix("/") else { return [] }
        return path.dropFirst().split(separator: "/", omittingEmptySubsequences: false)
            .map { String($0).replacingOccurrences(of: "~1", with: "/")
                .replacingOccurrences(of: "~0", with: "~") }
    }

    private func isPresentationPatch(_ operations: [[String: Any]]) -> Bool {
        for operation in operations {
            guard let path = operation["path"] as? String else { return false }
            let segments = pointerSegments(path)
            let bodyIndex: Int
            switch segments.first {
            case "body": bodyIndex = 0
            case "screens" where segments.count > 2 && segments[2] == "body": bodyIndex = 2
            case "components" where segments.count > 2 && segments[2] == "body": bodyIndex = 2
            default: return false
            }
            let nodePropertiesStart = bodyIndex + 3
            guard segments.count >= bodyIndex + 4,
                  segments.dropFirst(nodePropertiesStart).contains(where: {
                      Self.presentationPatchFields.contains($0)
                  })
            else { return false }
        }
        return true
    }

    private func isFunctionBodyPatch(_ operations: [[String: Any]]) -> Bool {
        guard !operations.isEmpty else { return false }
        return operations.allSatisfy { operation in
            guard let path = operation["path"] as? String else { return false }
            let segments = pointerSegments(path)
            return segments.count >= 3 && segments[0] == "functions"
                && (segments[2] == "body" || segments[2] == "body_actions")
        }
    }

    private func screenScopedIndices(_ operations: [[String: Any]]) -> Set<Int>? {
        guard !operations.isEmpty else { return nil }
        var indices = Set<Int>()
        for operation in operations {
            guard let path = operation["path"] as? String else { return nil }
            let segments = pointerSegments(path)
            guard segments.count >= 2, segments[0] == "screens",
                  let index = Int(segments[1])
            else { return nil }
            indices.insert(index)
        }
        return indices
    }

    private func installPatchedScreens(
        _ module: [String: Any],
        screenIndices: Set<Int>
    ) {
        let screens = module["screens"] as? [[String: Any]] ?? []
        for index in screenIndices {
            guard screens.indices.contains(index) else { continue }
            let screen = screens[index]
            guard let name = screen["name"] as? String else { continue }
            let scope = "screen/\(name)"
            var activeKeys = Set<String>()
            for state in screen["states"] as? [[String: Any]] ?? [] {
                guard let stateName = state["name"] as? String else { continue }
                let identity = "\(scope)/state/\(stateName)"
                activeKeys.insert(identity)
                let signature = Self.stateShapeSignature(state["ty"], enumCases: enumCases)
                if typeSignatures[identity] != signature || values[identity] == nil {
                    typeSignatures[identity] = signature
                    if let initial = state["initial"] {
                        let evaluated = evaluate(initial, locals: [:], scope: scope)
                        values[identity] = evaluated
                    }
                }
            }
            for key in values.keys where key.hasPrefix("\(scope)/state/") && !activeKeys.contains(key) {
                values.removeValue(forKey: key)
                typeSignatures.removeValue(forKey: key)
            }
            invalidateScreenScopes(named: name)
        }
    }

    private func invalidateScreenScopes(named name: String) {
        let rootScope = "screen/\(name)"
        for (scope, observer) in scopeRevisions
        where scope == rootScope || scope.hasSuffix("/\(rootScope)") {
            observer.invalidate()
        }
    }

    func install(module: [String: Any]) {
        currentModule = module
        let isHotReplacement = hasInstalledModule
        fastListEndReachedCounts.removeAll(keepingCapacity: true)
        fastListEndReachedKeys.removeAll(keepingCapacity: true)
        if isHotReplacement { clearNativeTasks() }
        let screens = module["screens"] as? [[String: Any]] ?? []
        let appStates = module["states"] as? [[String: Any]] ?? []
        let componentDeclarations = module["components"] as? [[String: Any]] ?? []
        installFunctionRegistry(module)
        let structDeclarations = module["structs"] as? [[String: Any]] ?? []
        var installedStructs: [String: [[String: Any]]] = [:]
        installedStructs.reserveCapacity(structDeclarations.count)
        for declaration in structDeclarations {
            guard let name = declaration["name"] as? String else { continue }
            installedStructs[name] = declaration["fields"] as? [[String: Any]] ?? []
        }
        structs = installedStructs
        let enumDeclarations = module["enums"] as? [[String: Any]] ?? []
        var installedEnumCases: [String: [String]] = [:]
        installedEnumCases.reserveCapacity(enumDeclarations.count)
        for declaration in enumDeclarations {
            guard let name = declaration["name"] as? String else { continue }
            installedEnumCases[name] = declaration["cases"] as? [String] ?? []
        }
        enumCases = installedEnumCases
        var nextModuleValues: [String: Any] = [:]
        var nextModuleSignatures: [String: String] = [:]
        for global in module["globals"] as? [[String: Any]] ?? [] {
            guard let name = global["name"] as? String else { continue }
            let signature = Self.stateShapeSignature(global["ty"], enumCases: installedEnumCases)
            nextModuleSignatures[name] = signature
            if moduleValueSignatures[name] == signature, let existing = moduleValues[name] {
                nextModuleValues[name] = existing
                moduleValues[name] = existing
            } else if let initial = global["initial"] {
                let initialized = evaluate(initial, locals: nextModuleValues, scope: "app")
                nextModuleValues[name] = initialized
                moduleValues[name] = initialized
            }
        }
        moduleValues = nextModuleValues
        moduleValueSignatures = nextModuleSignatures
        var nextValues: [String: Any] = [:]
        var nextTypes: [String: String] = [:]
        var initialLocals: [String: Any] = [:]
        func installState(_ state: [String: Any], scope: String) {
            guard let name = state["name"] as? String else { return }
            let identity = "\(scope)/state/\(name)"
            let signature = Self.stateShapeSignature(state["ty"], enumCases: installedEnumCases)
            nextTypes[identity] = signature
            let resolvedValue: Any
            if typeSignatures[identity] == signature, let value = values[identity] {
                resolvedValue = value
            } else if let initial = state["initial"] {
                resolvedValue = evaluate(initial, locals: initialLocals, scope: scope)
            } else {
                return
            }
            nextValues[identity] = resolvedValue
            initialLocals[name] = resolvedValue
        }
        for state in appStates {
            installState(state, scope: "app")
        }
        for screen in screens {
            guard let name = screen["name"] as? String else { continue }
            for state in screen["states"] as? [[String: Any]] ?? [] {
                installState(state, scope: "screen/\(name)")
            }
        }
        if isHotReplacement {
            for (identity, value) in values where identity.hasPrefix("component/") {
                let parts = identity.components(separatedBy: "/state/")
                guard parts.count == 2,
                      let componentName = parts[0]
                        .dropFirst("component/".count)
                        .split(separator: "/")
                        .first
                        .map(String.init),
                      let component = componentDeclarations.first(where: {
                          $0["name"] as? String == componentName
                      }),
                      let declaration = (component["states"] as? [[String: Any]])?.first(where: {
                          $0["name"] as? String == parts[1]
                      })
                else { continue }
                let signature = Self.stateShapeSignature(declaration["ty"], enumCases: installedEnumCases)
                guard typeSignatures[identity] == signature else { continue }
                nextValues[identity] = value
                nextTypes[identity] = signature
            }
            for (identity, value) in values where identity.hasPrefix("navigator/") || identity.hasPrefix("entry/") {
                let parts = identity.components(separatedBy: "/state/")
                guard parts.count == 2 else { continue }
                let stateName = parts[1]
                guard let screenRange = parts[0].range(of: "/screen/") else { continue }
                let screenName = String(parts[0][screenRange.upperBound...])
                guard let screen = screens.first(where: { $0["name"] as? String == screenName }),
                      let declaration = (screen["states"] as? [[String: Any]])?.first(where: {
                          $0["name"] as? String == stateName
                      })
                else { continue }
                let signature = Self.stateShapeSignature(declaration["ty"], enumCases: installedEnumCases)
                guard typeSignatures[identity] == signature else { continue }
                nextValues[identity] = value
                nextTypes[identity] = signature
            }
        }
        values = nextValues
        stateReaders.removeAll(keepingCapacity: true)
        typeSignatures = nextTypes
        if isHotReplacement {
            var currentScreens = Set<String>()
            currentScreens.reserveCapacity(screens.count)
            for screen in screens {
                if let name = screen["name"] as? String {
                    currentScreens.insert(name)
                }
            }
            activeScreenParameters = activeScreenParameters.filter { scope, _ in
                let components = scope.split(separator: "/")
                guard components.count > 1, components[0] == "screen" else { return false }
                return currentScreens.contains(String(components[1]))
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
                return values[identity] as? Bool == true
            }?.key
        }
        let screensSignature = Self.canonicalJSON(screens.map { $0["name"] ?? NSNull() })
        let rootScreen = screens.first?["name"] as? String
        if navigationRoot != rootScreen || navigationScreensSignature != screensSignature {
            navigationRoot = rootScreen
            navigationScreensSignature = screensSignature
            navigationPaths.removeAll(keepingCapacity: true)
            routeArguments.removeAll(keepingCapacity: true)
            routeInstanceTokens.removeAll(keepingCapacity: true)
        }
        invalidateAllScopes()
        if !hasInstalledModule {
            hasInstalledModule = true
            appLifecycleEpoch += 1
        }
    }

    func navigationPath(for stackIdentity: String) -> [NexaDevRoute] {
        navigationPaths[stackIdentity] ?? []
    }

    func setNavigationPath(_ path: [NexaDevRoute], for stackIdentity: String) {
        let previous = navigationPaths[stackIdentity] ?? []
        navigationPaths[stackIdentity] = path
        for oldRoute in previous {
            if !path.contains(where: { $0.entryId == oldRoute.entryId }) {
                routeArguments.removeValue(forKey: oldRoute.entryId)
                clearScope(scope: "navigator/\(stackIdentity)/entry/\(oldRoute.entryId)/screen/\(oldRoute.screen)")
            }
        }
    }

    func resolveNavigatorId(stackIdentity: String? = nil, locals: [String: Any]? = nil, scope: String) -> String {
        if let stackIdentity {
            return navigationPaths.keys.contains(stackIdentity) ? stackIdentity : ""
        }
        if let localId = locals?["__nexa_navigation_stack_identity"] as? String {
            return navigationPaths.keys.contains(localId) ? localId : ""
        }
        if scope.hasPrefix("navigator/") {
            let afterPrefix = scope.dropFirst("navigator/".count)
            if let entryRange = afterPrefix.range(of: "/entry/") {
                let navId = String(afterPrefix[..<entryRange.lowerBound])
                return navigationPaths.keys.contains(navId) ? navId : ""
            }
            let navId = String(afterPrefix)
            return navigationPaths.keys.contains(navId) ? navId : ""
        }
        if navigationPaths.keys.contains(scope) {
            return scope
        }
        if scope == "app" && navigationPaths.keys.contains("app") {
            return "app"
        }
        return ""
    }

    func pushNavigation(destination: Int, expressions: [Any], locals: [String: Any], scope: String) {
        let screens = currentModule["screens"] as? [[String: Any]] ?? []
        guard screens.indices.contains(destination),
              let screenName = screens[destination]["name"] as? String
        else {
            NSLog("NexaDevRuntime pushNavigation FAILED: destination=%d screensCount=%d", destination, screens.count)
            return
        }
        let parameters = screens[destination]["parameters"] as? [[String: Any]] ?? []
        var values: [String: Any] = [:]
        for (parameter, expression) in zip(parameters, expressions) {
            guard let name = parameter["name"] as? String else { continue }
            values[name] = evaluate(expression, locals: locals, scope: scope)
        }
        let signature = Self.canonicalSignature(parameters)
        let stackIdentity = resolveNavigatorId(
            stackIdentity: locals["__nexa_navigation_stack_identity"] as? String,
            locals: locals,
            scope: scope
        )
        let route = navigationRoute(screen: screenName, values: values, signature: signature, navigatorId: stackIdentity)
        if let binding = locals["__nexa_navigation_path_binding"] as? Binding<[NexaDevRoute]> {
            binding.wrappedValue.append(route)
            invalidateScope(scope)
            invalidateScope(stackIdentity)
            objectWillChange.send()
            NSLog("NexaDevRuntime pushNavigation via binding to: %@", screenName)
            return
        }
        NSLog("NexaDevRuntime pushNavigation destination=%@ stackIdentity=%@ availableKeys=%@", screenName, stackIdentity, Array(navigationPaths.keys))
        var path = navigationPath(for: stackIdentity)
        path.append(route)
        setNavigationPath(path, for: stackIdentity)
        invalidateScope(scope)
        invalidateScope(stackIdentity)
        objectWillChange.send()
    }

    func pushNavigation(screenName: String, expressions: [Any], locals: [String: Any], scope: String, stackIdentity: String? = nil) {
        let screens = currentModule["screens"] as? [[String: Any]] ?? []
        guard let screen = screens.first(where: { $0["name"] as? String == screenName }) else { return }
        let parameters = screen["parameters"] as? [[String: Any]] ?? []
        var values: [String: Any] = [:]
        for (parameter, expression) in zip(parameters, expressions) {
            guard let name = parameter["name"] as? String else { continue }
            values[name] = evaluate(expression, locals: locals, scope: scope)
        }
        let signature = Self.canonicalSignature(parameters)
        let resolvedStack = stackIdentity ?? resolveNavigatorId(
            stackIdentity: locals["__nexa_navigation_stack_identity"] as? String,
            locals: locals,
            scope: scope
        )
        let route = navigationRoute(screen: screenName, values: values, signature: signature, navigatorId: resolvedStack)
        if let binding = locals["__nexa_navigation_path_binding"] as? Binding<[NexaDevRoute]> {
            binding.wrappedValue.append(route)
            invalidateScope(scope)
            invalidateScope(resolvedStack)
            objectWillChange.send()
            return
        }
        var path = navigationPath(for: resolvedStack)
        path.append(route)
        setNavigationPath(path, for: resolvedStack)
        invalidateScope(scope)
        invalidateScope(resolvedStack)
        objectWillChange.send()
    }

    func resetNavigation(screenName: String, expressions: [Any], locals: [String: Any], scope: String, stackIdentity: String? = nil) {
        let resolvedStack = stackIdentity ?? resolveNavigatorId(
            stackIdentity: locals["__nexa_navigation_stack_identity"] as? String,
            locals: locals,
            scope: scope
        )
        setNavigationPath([], for: resolvedStack)
        pushNavigation(screenName: screenName, expressions: expressions, locals: locals, scope: scope, stackIdentity: resolvedStack)
    }

    func popNavigation(stackIdentity: String?, scope: String, locals: [String: Any]? = nil) {
        let effectiveStack = resolveNavigatorId(stackIdentity: stackIdentity, locals: locals, scope: scope)

        if let binding = locals?["__nexa_navigation_path_binding"] as? Binding<[NexaDevRoute]> {
            if !binding.wrappedValue.isEmpty {
                let popped = binding.wrappedValue.removeLast()
                clearScope(scope: "navigator/\(effectiveStack)/entry/\(popped.entryId)/screen/\(popped.screen)")
                routeArguments.removeValue(forKey: popped.entryId)
            }
            invalidateScope(scope)
            invalidateScope(effectiveStack)
            objectWillChange.send()
            return
        }

        var path = navigationPath(for: effectiveStack)
        guard !path.isEmpty else { return }
        let popped = path.removeLast()
        clearScope(scope: "navigator/\(effectiveStack)/entry/\(popped.entryId)/screen/\(popped.screen)")
        routeArguments.removeValue(forKey: popped.entryId)
        setNavigationPath(path, for: effectiveStack)
        invalidateScope(scope)
        invalidateScope(effectiveStack)
        objectWillChange.send()
    }

    func popToNavigation(targetEntryId: String, inclusive: Bool, stackIdentity: String? = nil, scope: String = "app", locals: [String: Any]? = nil) {
        let effectiveStack = resolveNavigatorId(stackIdentity: stackIdentity, locals: locals, scope: scope)
        guard !effectiveStack.isEmpty else { return }

        if let binding = locals?["__nexa_navigation_path_binding"] as? Binding<[NexaDevRoute]> {
            var current = binding.wrappedValue
            if let index = current.firstIndex(where: { $0.entryId == targetEntryId }) {
                let cutoff = inclusive ? index : index + 1
                if cutoff < current.count {
                    for entry in current[cutoff...] {
                        clearScope(scope: "navigator/\(effectiveStack)/entry/\(entry.entryId)/screen/\(entry.screen)")
                        routeArguments.removeValue(forKey: entry.entryId)
                    }
                    current.removeSubrange(cutoff...)
                    binding.wrappedValue = current
                    invalidateScope(scope)
                    invalidateScope(effectiveStack)
                    objectWillChange.send()
                }
            }
            return
        }

        var path = navigationPath(for: effectiveStack)
        if let index = path.firstIndex(where: { $0.entryId == targetEntryId }) {
            let cutoff = inclusive ? index : index + 1
            if cutoff < path.count {
                for entry in path[cutoff...] {
                    clearScope(scope: "navigator/\(effectiveStack)/entry/\(entry.entryId)/screen/\(entry.screen)")
                    routeArguments.removeValue(forKey: entry.entryId)
                }
                path.removeSubrange(cutoff...)
                setNavigationPath(path, for: effectiveStack)
                invalidateScope(scope)
                invalidateScope(effectiveStack)
                objectWillChange.send()
            }
        }
    }

    func hotRestart(module: [String: Any]) {
        currentModule = module
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
        navigationPaths.removeAll(keepingCapacity: true)
        routeArguments.removeAll(keepingCapacity: true)
        routeInstanceTokens.removeAll(keepingCapacity: true)
        focusedFieldKey = focusBindings.first { _, binding in
            guard let state = binding.state else { return false }
            return values["\(binding.scope)/state/\(state)"] as? Bool == true
        }?.key
        stateReaders.removeAll(keepingCapacity: true)
        invalidateAllScopes()
        appLifecycleEpoch += 1
    }

    func value(_ name: String, scope: String = "app") -> Any {
        let scopedKey = "\(scope)/state/\(name)"
        if let value = values[scopedKey] {
            stateReaders[scopedKey, default: []].insert(scope)
            return value
        }
        let appKey = "app/state/\(name)"
        if let value = values[appKey] {
            stateReaders[appKey, default: []].insert(scope)
            return value
        }
        return NSNull()
    }

    private func stateStorageKey(_ name: String, scope: String) -> String {
        let scopedKey = "\(scope)/state/\(name)"
        return values[scopedKey] != nil || !scope.starts(with: "screen/")
            ? scopedKey
            : "app/state/\(name)"
    }

    func stateCommandRevision(_ name: String, scope: String) -> Int {
        stateCommandRevisions[stateStorageKey(name, scope: scope), default: 0]
    }

    func pagePagerRestoreRevision(forState name: String, scope: String) -> Int {
        stateCommandRevision(name, scope: scope)
    }

    func locals(scope: String, parameters: [String: Any]) -> [String: Any] {
        parameters
    }

    func bindings(
        _ declarations: [[String: Any]]?,
        parameters: [String: Any],
        scope: String
    ) -> [String: Any] {
        var resolved = parameters
        for declaration in declarations ?? [] {
            guard let name = declaration["name"] as? String,
                  let initial = declaration["initial"] else { continue }
            let type = declaration["ty"] as? [String: Any] ?? [:]
            let stableNativeValue = type["Class"] != nil || type["Plugin"] != nil || type["Signal"] != nil
            if declaration["mutable"] as? Bool == true || stableNativeValue {
                let identity = "\(scope)/state/\(name)"
                typeSignatures[identity] = Self.stateShapeSignature(declaration["ty"], enumCases: enumCases)
                if values[identity] == nil {
                    values[identity] = evaluate(initial, locals: resolved, scope: scope)
                }
                // Mutable state must be read through `value(_:scope:)` so the
                // active scope is registered as a reader and invalidated when
                // the state changes. Capturing its current value in `locals`
                // makes SwiftUI keep rendering the initial snapshot forever.
                if declaration["mutable"] as? Bool != true {
                    resolved[name] = values[identity] ?? NSNull()
                }
            } else {
                resolved[name] = evaluate(initial, locals: resolved, scope: scope)
            }
        }
        return resolved
    }

    func setValue(_ name: String, value: Any, scope: String) {
        let storageKey = stateStorageKey(name, scope: scope)
        if let current = values[storageKey] as? NSObject,
           let next = value as? NSObject,
           type(of: current) == type(of: next),
           current.isEqual(next) {
            return
        }
        values[storageKey] = value
        runtimeTrace("state_update \(runtimeTraceScope(scope)):\(name)")
        stateCommandRevisions[storageKey, default: 0] += 1
        if let focused = value as? Bool,
           let focusIdentity = focusBindings.first(where: { _, binding in
               binding.scope == scope && binding.state == name
           })?.key {
            if focused {
                focusedFieldKey = focusIdentity
            } else if focusedFieldKey == focusIdentity {
                focusedFieldKey = nil
            }
        }
        invalidateReaders(of: storageKey, sourceScope: scope)
    }

    func runtimeTrace(_ event: String) {
        guard runtimeTraceEnabled else { return }
        NSLog("[Nexa][TRACE] %@", event)
    }

    func runtimeTraceScope(_ scope: String) -> String {
        if scope == "app" { return "app" }
        if scope.hasPrefix("screen/") { return "screen" }
        if scope.hasPrefix("component/") { return "component" }
        return "other"
    }

    func setScrollPositionFromNative(
        _ name: String,
        value: Int,
        scope: String,
        expectedCommandRevision: Int
    ) {
        let storageKey = stateStorageKey(name, scope: scope)
        guard stateCommandRevisions[storageKey, default: 0] == expectedCommandRevision else { return }
        if let current = values[storageKey] as? NSNumber, current.intValue == value { return }
        values[storageKey] = value
        invalidateReaders(of: storageKey, sourceScope: scope)
    }

    func focusChanged(to nextIdentity: String?) {
        let previousIdentity = focusedFieldKey
        focusedFieldKey = nextIdentity
        guard previousIdentity != nextIdentity else { return }
        if let previousIdentity, let binding = focusBindings[previousIdentity], let state = binding.state {
            setValue(state, value: false, scope: binding.scope)
        }
        if let nextIdentity, let binding = focusBindings[nextIdentity], let state = binding.state {
            setValue(state, value: true, scope: binding.scope)
        }
    }

    func screenDidAppear(scope: String, parameters: [String: Any]) {
        activeScreenParameters[scope] = parameters
        invalidateScope(scope)
    }

    func clearScope(scope: String) {
        activeScreenParameters.removeValue(forKey: scope)
        for key in Array(stateReaders.keys) {
            stateReaders[key]?.remove(scope)
            if stateReaders[key]?.isEmpty == true { stateReaders.removeValue(forKey: key) }
        }
        clearNativeEventSubscriptions(scope: scope)
        clearNativeTasks(scope: scope)
        let statePrefix = "\(scope)/"
        for key in Array(values.keys) where key.hasPrefix(statePrefix) {
            values.removeValue(forKey: key)
        }
        for key in Array(typeSignatures.keys) where key.hasPrefix(statePrefix) {
            typeSignatures.removeValue(forKey: key)
        }
        for key in Array(stateCommandRevisions.keys) where key.hasPrefix(statePrefix) {
            stateCommandRevisions.removeValue(forKey: key)
        }
        let listPrefix = "\(scope)|"
        for key in Array(fastListEndReachedCounts.keys) where key.hasPrefix(listPrefix) {
            fastListEndReachedCounts.removeValue(forKey: key)
            fastListEndReachedKeys.remove(key)
        }
        scopeRevisions.removeValue(forKey: scope)
        let removedFocusKeys = focusBindings.compactMap { entry in
            entry.value.scope == scope ? entry.key : nil
        }
        for key in removedFocusKeys {
            focusBindings.removeValue(forKey: key)
        }
        if let focusedFieldKey, removedFocusKeys.contains(focusedFieldKey) {
            self.focusedFieldKey = nil
        }
    }

    func screenDidDisappear(
        scope: String,
        navigationStackIdentity: String = "app",
        routeIdentity: String = "root"
    ) {
        activeScreenParameters.removeValue(forKey: scope)
        for key in Array(stateReaders.keys) {
            stateReaders[key]?.remove(scope)
            if stateReaders[key]?.isEmpty == true { stateReaders.removeValue(forKey: key) }
        }
        clearNativeEventSubscriptions(scope: scope)
        clearNativeTasks(scope: scope)

        guard routeIdentity != "root",
              !navigationPath(for: navigationStackIdentity).contains(where: { $0.entryId == routeIdentity })
        else { return }

        clearScope(scope: scope)
        routeArguments.removeValue(forKey: routeIdentity)
    }

    static func collectFocusBindings(
        _ nodes: [Any],
        scope: String,
        into bindings: inout [String: (scope: String, state: String?)]
    ) {
        for node in nodes {
            guard let tagged = node as? [String: Any], let (kind, payload) = tagged.first else { continue }
            let fields = payload as? [String: Any] ?? [:]
            if kind == "TextInput", let focusedState = fields["focused"] as? String {
                let stateName = fields["state"] as? String ?? ""
                let identity = "\(scope)/input/\(stateName)"
                bindings[identity] = (scope: scope, state: focusedState)
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
            case "ScreenWidth": return locals["__nexa_screen_width"] ?? Double(UIScreen.main.bounds.width)
            case "ScreenHeight": return locals["__nexa_screen_height"] ?? Double(UIScreen.main.bounds.height)
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
        case "SystemInset":
            switch payload as? String {
            case "Top": return locals["__nexa_system_inset_top"] ?? 0.0
            case "Bottom": return locals["__nexa_system_inset_bottom"] ?? 0.0
            case "Leading": return locals["__nexa_system_inset_leading"] ?? 0.0
            case "Trailing": return locals["__nexa_system_inset_trailing"] ?? 0.0
            default: return 0.0
            }
        case "ComponentRefCreate":
            let fields = payload as? [String: Any] ?? [:]
            let namespace = fields["namespace"] as? String ?? "Nexa"
            let name = fields["name"] as? String ?? ""
            let identity = fields["identity"] as? String ?? ""
            let key = "\(scope)/component-ref/\(namespace).\(name)/\(identity)"
            if let existing = componentRefs[key] { return existing }
            let reference: Any
            if namespace == "Nexa", name == "BottomSheet" {
                reference = NexaBottomSheetRef()
            } else if namespace == "Nexa", name == "FastList" {
                reference = NexaFastListRef()
            } else if namespace == "Nexa" {
                reference = NexaDevComponentRef(componentName: name, identity: key)
            } else if let created = NexaDevPluginBridge.constructComponentRef(namespace: namespace, name: name) {
                reference = created
            } else {
                return NSNull()
            }
            componentRefs[key] = reference
            return reference
        case "String": return payload as? String ?? ""
        case "LocalizedText":
            let fields = payload as? [String: Any] ?? [:]
            let key = fields["key"] as? String ?? ""
            let source = fields["value"]
            let fallback = source.map { localizedInterpolationText($0, locals: locals, scope: scope) } ?? key
            return localizedText(
                key: key,
                fallback: fallback,
                sourceExpression: source,
                locals: locals,
                scope: scope
            )
        case "Located":
            let fields = payload as? [String: Any] ?? [:]
            guard let sourceFile = fields["source_file"] as? String,
                  let sourceSpan = fields["source_span"] as? [String: Any],
                  let line = (sourceSpan["line"] as? NSNumber)?.intValue,
                  let column = (sourceSpan["column"] as? NSNumber)?.intValue,
                  let expression = fields["expression"]
            else { return NSNull() }
            let value = evaluate(expression, locals: locals, scope: scope)
            if var failure = pendingPluginFailure {
                attachSourceLocation(file: sourceFile, line: line, column: column, to: &failure)
                pendingPluginFailure = failure
            }
            return value
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
        case "PluginEnumConstructor":
            let fields = payload as? [String: Any] ?? [:]
            let arguments = (fields["arguments"] as? [Any] ?? []).map {
                evaluate($0, locals: locals, scope: scope)
            }
            return [
                "__nexaEnum": "\(fields["namespace"] as? String ?? "").\(fields["enum_name"] as? String ?? "")",
                "case_name": fields["case_name"] as? String ?? "",
                "arguments": arguments,
            ]
        case "PluginEnumOptionalConstructor":
            let fields = payload as? [String: Any] ?? [:]
            let value = evaluate(fields["value"] ?? NSNull(), locals: locals, scope: scope)
            let enumName = "\(fields["namespace"] as? String ?? "").\(fields["enum_name"] as? String ?? "")"
            let isNull = value is NSNull
            let caseName = fields[isNull ? "null_case_name" : "case_name"] as? String ?? ""
            let arguments: [Any] = isNull ? [] : [value]
            return ["__nexaEnum": enumName, "case_name": caseName, "arguments": arguments]
        case "State", "AnimatedState":
            let parts = payload as? [Any] ?? []
            guard let name = parts.first as? String else { return NSNull() }
            return locals[name] ?? moduleValues[name] ?? value(name, scope: scope)
        case "This":
            return locals["this"] ?? NSNull()
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
            case "FlatMap": return collection.flatMap { apply($0) as? [Any] ?? [] }
            case "Filter": return collection.filter { truthy(apply($0)) }
            case "Reduce":
                var accumulator = fields["initial"].map { evaluate($0, locals: locals, scope: scope) } ?? 0
                for item in collection { accumulator = apply(item, accumulator) }
                return accumulator
            case "SortedBy":
                let keyed = collection.enumerated().map { index, item in
                    (index: index, item: item, key: apply(item))
                }
                return keyed.sorted {
                    if compareSortable($0.key, $1.key) { return true }
                    if compareSortable($1.key, $0.key) { return false }
                    return $0.index < $1.index
                }.map(\.item)
            case "GroupedBy":
                var groupIndexes: [NexaDevHashableValue: Int] = [:]
                var groups: [[Any]] = []
                for item in collection {
                    let key = NexaDevHashableValue(apply(item))
                    if let index = groupIndexes[key] {
                        groups[index].append(item)
                    } else {
                        groupIndexes[key] = groups.count
                        groups.append([item])
                    }
                }
                return groups
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
            if fields["optional"] as? Bool == true, value is NSNull { return NSNull() }
            if let memberKind = fields["kind"] as? String {
                let result = value as? [String: Any] ?? [:]
                switch memberKind {
                case "ResultIsSuccess": return result["Ok"] != nil
                case "ResultValue": return result["Ok"] ?? NSNull()
                case "ResultError": return result["Err"] ?? NSNull()
                case "NetworkBody": return result["body"] ?? (result["text"] as? String) ?? NSNull()
                case "NetworkStatusCode": return (result["statusCode"] as? NSNumber)?.int32Value ?? 0
                case "StringTrimmed": return (value as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                case "CollectionCount":
                    if let values = value as? [Any] { return Int32(values.count) }
                    if let values = value as? [String: Any] { return Int32(values.count) }
                    if let values = value as? Set<NexaDevHashableValue> { return Int32(values.count) }
                    return 0
                case "CollectionIsEmpty":
                    if let values = value as? [Any] { return values.isEmpty }
                    if let values = value as? [String: Any] { return values.isEmpty }
                    if let values = value as? Set<NexaDevHashableValue> { return values.isEmpty }
                    return true
                case "SignalValue": return value
                default: break
                }
            }
            if let memberKind = fields["kind"] as? [String: Any],
               memberKind["ClassStaticField"] != nil,
               let classType = fields["base_type"] as? [String: Any],
               let classPayload = classType["Class"] as? [String: Any],
               let className = classPayload["name"] as? String {
                return moduleValues["\(className)::\(name)"] ?? NSNull()
            }
            if let instance = value as? NexaDevUserClassInstance {
                return instance.fields[name] ?? NSNull()
            }
            if let row = value as? NexaDevDynamicRow {
                return row.fields[name] ?? NSNull()
            }
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
            if name == "trimmed", let text = value as? String {
                return text.trimmingCharacters(in: .whitespacesAndNewlines)
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
            if call["is_constructor"] as? Bool == true,
               let returnType = call["return_type"] as? [String: Any],
               let classType = returnType["Class"] as? [String: Any],
               classType["name"] as? String != nil {
                let arguments = (call["arguments"] as? [Any] ?? []).map {
                    evaluate($0, locals: locals, scope: scope)
                }
                return makeUserClassInstance(classType, arguments: arguments, scope: scope)
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
            if let actions = function["body_actions"] as? [Any] {
                switch performFunctionActions(actions, scope: scope, locals: functionScope) {
                case .returned(let value): return value
                case .normal, .break, .continue: return NSNull()
                }
            }
            guard let body = function["body"] else { return NSNull() }
            return evaluate(body, locals: functionScope, scope: scope)
        case "NativeCall":
            guard let call = payload as? [String: Any] else { return NSNull() }
            if let namespace = call["namespace"] as? String,
               let className = namespace.stripPrefix("__NexaUserClass:") {
                let receiver = call["receiver"].map { evaluate($0, locals: locals, scope: scope) } ?? NSNull()
                let arguments = (call["arguments"] as? [[Any]] ?? []).compactMap { $0.last }.map {
                    evaluate($0, locals: locals, scope: scope)
                }
                return invokeUserClassMethod(className, method: call["name"] as? String ?? "", receiver: receiver, arguments: arguments, locals: locals, scope: scope)
            }
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
        if let unit = expression as? String {
            return evaluate(unit, locals: locals, scope: scope)
        }
        guard let tagged = expression as? [String: Any], let (kind, payload) = tagged.first else {
            return NSNull()
        }
        switch kind {
        case "EnumValue", "PluginEnumValue":
            return (payload as? [String: Any])?["case_name"] as? String ?? ""
        case "PluginEnumConstructor":
            let fields = payload as? [String: Any] ?? [:]
            var arguments: [Any] = []
            for argument in fields["arguments"] as? [Any] ?? [] {
                arguments.append(try await evaluateAsync(argument, locals: locals, scope: scope))
            }
            return [
                "__nexaEnum": "\(fields["namespace"] as? String ?? "").\(fields["enum_name"] as? String ?? "")",
                "case_name": fields["case_name"] as? String ?? "",
                "arguments": arguments,
            ]
        case "PluginEnumOptionalConstructor":
            let fields = payload as? [String: Any] ?? [:]
            let value = try await evaluateAsync(fields["value"] ?? NSNull(), locals: locals, scope: scope)
            let enumName = "\(fields["namespace"] as? String ?? "").\(fields["enum_name"] as? String ?? "")"
            let isNull = value is NSNull
            let caseName = fields[isNull ? "null_case_name" : "case_name"] as? String ?? ""
            let arguments: [Any] = isNull ? [] : [value]
            return ["__nexaEnum": enumName, "case_name": caseName, "arguments": arguments]
        case "This": return locals["this"] ?? NSNull()
        case "State", "AnimatedState":
            let parts = payload as? [Any] ?? []
            guard let name = parts.first as? String else { return NSNull() }
            return locals[name] ?? moduleValues[name] ?? value(name, scope: scope)
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
            case "StartOfDay":
                guard let value = (first as? NSNumber)?.int64Value else { return NSNull() }
                return nexaDevStartOfDay(value)
            case "AddCalendarDays":
                guard arguments.count > 1,
                    let timestamp = arguments[0] as? NSNumber,
                    let days = arguments[1] as? NSNumber
                else { return NSNull() }
                return nexaDevAddCalendarDays(timestamp.int64Value, days.int32Value)
            case "LocalizedDate":
                guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
                return nexaDevLocalizedDate(timestamp)
            case "LocalizedLongDate":
                guard arguments.count > 1,
                    let timestamp = arguments[0] as? NSNumber,
                    let locale = arguments[1] as? String
                else { return NSNull() }
                return nexaDevLocalizedLongDate(timestamp.int64Value, locale)
            case "LocalizedTime":
                guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
                return nexaDevLocalizedTime(timestamp)
            case "LocalizedDateTime":
                guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
                return nexaDevLocalizedDateTime(timestamp)
            case "Format":
                guard arguments.count > 1,
                    let timestamp = arguments[0] as? NSNumber,
                    let pattern = arguments[1] as? String
                else { return NSNull() }
                return nexaDevFormatDate(timestamp.int64Value, pattern)
            case "FormatDuration":
                guard let seconds = (first as? NSNumber)?.doubleValue else { return NSNull() }
                return nexaDevFormatDuration(seconds)
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
        case "Located":
            let fields = payload as? [String: Any] ?? [:]
            guard let sourceFile = fields["source_file"] as? String,
                  let sourceSpan = fields["source_span"] as? [String: Any],
                  let line = (sourceSpan["line"] as? NSNumber)?.intValue,
                  let column = (sourceSpan["column"] as? NSNumber)?.intValue,
                  let expression = fields["expression"]
            else { return NSNull() }
            do {
                let value = try await evaluateAsync(expression, locals: locals, scope: scope)
                if var failure = pendingPluginFailure {
                    attachSourceLocation(file: sourceFile, line: line, column: column, to: &failure)
                    pendingPluginFailure = failure
                }
                return value
            } catch var failure as NexaDevPluginFailure {
                attachSourceLocation(file: sourceFile, line: line, column: column, to: &failure)
                throw failure
            } catch let located as NexaDevLocatedRuntimeError {
                throw located
            } catch let cancellation as CancellationError {
                throw cancellation
            } catch {
                throw NexaDevLocatedRuntimeError(
                    sourceFile: sourceFile,
                    line: line,
                    column: column,
                    underlying: error
                )
            }
        case "NativeCall":
            guard let call = payload as? [String: Any] else { return NSNull() }
            if let namespace = call["namespace"] as? String,
               let className = namespace.stripPrefix("__NexaUserClass:") {
                    let receiver: Any
                    if let expression = call["receiver"] {
                        receiver = try await evaluateAsync(expression, locals: locals, scope: scope)
                    } else {
                        receiver = NSNull()
                    }
                    var arguments: [Any] = []
                    for pair in call["arguments"] as? [[Any]] ?? [] {
                        if let argument = pair.last {
                            arguments.append(try await evaluateAsync(argument, locals: locals, scope: scope))
                        }
                    }
                    if call["is_async"] as? Bool == true {
                        return try await invokeUserClassMethodAsync(
                            className,
                            method: call["name"] as? String ?? "",
                            receiver: receiver,
                            arguments: arguments,
                            locals: locals,
                            scope: scope
                        )
                    }
                    return invokeUserClassMethod(className, method: call["name"] as? String ?? "", receiver: receiver, arguments: arguments, locals: locals, scope: scope)
            }
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
                ["cachePolicy", request["cache_policy"] ?? NSNull()],
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
        case "FileExists", "FileReadText", "FileReadBytes", "FileWriteText", "FileWriteBytes", "FileDelete":
            guard let file = payload as? [String: Any] else { return NSNull() }
            let name: String
            switch kind {
            case "FileExists": name = "exists"
            case "FileReadText": name = "readText"
            case "FileReadBytes": name = "readBytes"
            case "FileWriteText": name = "writeText"
            case "FileWriteBytes": name = "writeBytes"
            default: name = "delete"
            }
            var fileArguments: [[Any]] = [["path", file["path"] ?? NSNull()]]
            if kind == "FileWriteText" || kind == "FileWriteBytes" {
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
            if member["optional"] as? Bool == true, value is NSNull { return NSNull() }
            if let memberKind = member["kind"] as? String {
                let result = value as? [String: Any] ?? [:]
                switch memberKind {
                case "ResultIsSuccess": return result["Ok"] != nil
                case "ResultValue": return result["Ok"] ?? NSNull()
                case "ResultError": return result["Err"] ?? NSNull()
                case "NetworkBody": return result["body"] ?? (result["text"] as? String) ?? NSNull()
                case "NetworkStatusCode": return (result["statusCode"] as? NSNumber)?.int32Value ?? 0
                case "StringTrimmed": return (value as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                case "CollectionCount":
                    if let values = value as? [Any] { return Int32(values.count) }
                    if let values = value as? [String: Any] { return Int32(values.count) }
                    if let values = value as? Set<NexaDevHashableValue> { return Int32(values.count) }
                    return 0
                case "CollectionIsEmpty":
                    if let values = value as? [Any] { return values.isEmpty }
                    if let values = value as? [String: Any] { return values.isEmpty }
                    if let values = value as? Set<NexaDevHashableValue> { return values.isEmpty }
                    return true
                case "SignalValue": return value
                default: break
                }
            }
            if let memberKind = member["kind"] as? [String: Any],
               memberKind["ClassStaticField"] != nil,
               let classType = member["base_type"] as? [String: Any],
               let classPayload = classType["Class"] as? [String: Any],
               let className = classPayload["name"] as? String {
                return moduleValues["\(className)::\(name)"] ?? NSNull()
            }
            if let instance = value as? NexaDevUserClassInstance {
                return instance.fields[name] ?? NSNull()
            }
            if let row = value as? NexaDevDynamicRow {
                return row.fields[name] ?? NSNull()
            }
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
            if name == "trimmed", let text = value as? String {
                return text.trimmingCharacters(in: .whitespacesAndNewlines)
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
            case "FlatMap":
                var flattened: [Any] = []
                for item in items {
                    if let mapped = try await apply(item) as? [Any] { flattened.append(contentsOf: mapped) }
                }
                return flattened
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
            case "SortedBy":
                var keyed: [(index: Int, item: Any, key: Any)] = []
                keyed.reserveCapacity(items.count)
                for (index, item) in items.enumerated() {
                    keyed.append((index, item, try await apply(item)))
                }
                return keyed.sorted {
                    if compareSortable($0.key, $1.key) { return true }
                    if compareSortable($1.key, $0.key) { return false }
                    return $0.index < $1.index
                }.map(\.item)
            case "GroupedBy":
                var groupIndexes: [NexaDevHashableValue: Int] = [:]
                var groups: [[Any]] = []
                for item in items {
                    let key = NexaDevHashableValue(try await apply(item))
                    if let index = groupIndexes[key] {
                        groups[index].append(item)
                    } else {
                        groupIndexes[key] = groups.count
                        groups.append([item])
                    }
                }
                return groups
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
        if call["is_constructor"] as? Bool == true,
           let returnType = call["return_type"] as? [String: Any],
           let classType = returnType["Class"] as? [String: Any] {
            var arguments: [Any] = []
            for argument in call["arguments"] as? [Any] ?? [] {
                arguments.append(try await evaluateAsync(argument, locals: locals, scope: scope))
            }
            return makeUserClassInstance(classType, arguments: arguments, scope: scope)
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
        if let actions = function["body_actions"] as? [Any] {
            switch try await performFunctionAsync(actions, scope: scope, locals: functionScope) {
            case .returned(let value): return value
            case .normal, .break, .continue: return NSNull()
            }
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

    func compareSortable(_ lhs: Any, _ rhs: Any) -> Bool {
        if let left = lhs as? String, let right = rhs as? String { return left < right }
        return number(lhs) < number(rhs)
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

    static func stateShapeSignature(_ type: Any?, enumCases: [String: [String]]) -> String {
        var referencedEnums = Set<String>()
        func collectEnumTypes(in value: Any) {
            if let object = value as? [String: Any] {
                if let name = object["Enum"] as? String { referencedEnums.insert(name) }
                for child in object.values { collectEnumTypes(in: child) }
            } else if let values = value as? [Any] {
                for child in values { collectEnumTypes(in: child) }
            }
        }
        if let type { collectEnumTypes(in: type) }
        var referencedCases: [String: [String]] = [:]
        for name in referencedEnums.sorted() {
            referencedCases[name] = enumCases[name] ?? []
        }
        return canonicalJSON([
            "type": type ?? NSNull(),
            "enumCases": referencedCases,
        ])
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
    case "StartOfDay":
        guard let value = (first as? NSNumber)?.int64Value else { return NSNull() }
        return nexaDevStartOfDay(value)
    case "AddCalendarDays":
        guard arguments.count > 1,
            let timestamp = arguments[0] as? NSNumber,
            let days = arguments[1] as? NSNumber
        else { return NSNull() }
        return nexaDevAddCalendarDays(timestamp.int64Value, days.int32Value)
    case "LocalizedDate":
        guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
        return nexaDevLocalizedDate(timestamp)
    case "LocalizedLongDate":
        guard arguments.count > 1,
            let timestamp = arguments[0] as? NSNumber,
            let locale = arguments[1] as? String
        else { return NSNull() }
        return nexaDevLocalizedLongDate(timestamp.int64Value, locale)
    case "LocalizedTime":
        guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
        return nexaDevLocalizedTime(timestamp)
    case "LocalizedDateTime":
        guard let timestamp = (first as? NSNumber)?.int64Value else { return NSNull() }
        return nexaDevLocalizedDateTime(timestamp)
    case "Format":
        guard arguments.count > 1,
            let timestamp = arguments[0] as? NSNumber,
            let pattern = arguments[1] as? String
        else { return NSNull() }
        return nexaDevFormatDate(timestamp.int64Value, pattern)
    case "FormatDuration":
        guard let seconds = (first as? NSNumber)?.doubleValue else { return NSNull() }
        return nexaDevFormatDuration(seconds)
    default:
        return NSNull()
    }
}
}

private func nexaDevStartOfDay(_ milliseconds: Int64) -> Int64 {
    let date = Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000)
    return Int64(Calendar.current.startOfDay(for: date).timeIntervalSince1970 * 1000)
}

private func nexaDevAddCalendarDays(_ milliseconds: Int64, _ days: Int32) -> Int64 {
    let date = Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000)
    guard let advanced = Calendar.current.date(byAdding: .day, value: Int(days), to: date) else {
        return milliseconds
    }
    return Int64((advanced.timeIntervalSince1970 * 1000).rounded())
}

private func nexaDevLocalizedDate(_ milliseconds: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000)
    return DateFormatter.localizedString(from: date, dateStyle: .short, timeStyle: .none)
}

private func nexaDevLocalizedLongDate(_ milliseconds: Int64, _ languageCode: String) -> String {
    let locale = Locale(identifier: languageCode.replacingOccurrences(of: "_", with: "-"))
    let calendar = locale.calendar
    let timeZone = TimeZone.current
    let key = "nexa.dev.long-date.\(locale.identifier).\(String(describing: calendar.identifier)).\(timeZone.identifier)"
    let cache = Thread.current.threadDictionary
    let formatter: DateFormatter
    if let cached = cache[key] as? DateFormatter {
        formatter = cached
    } else {
        let cachedKeys = cache.allKeys.compactMap { key -> String? in
            guard let key = key as? String, key.hasPrefix("nexa.dev.long-date.") else { return nil }
            return key
        }
        if cachedKeys.count >= 16 {
            cachedKeys.forEach { cache.removeObject(forKey: $0) }
        }
        formatter = DateFormatter()
        formatter.locale = locale
        formatter.calendar = calendar
        formatter.timeZone = timeZone
        formatter.dateStyle = .long
        formatter.timeStyle = .none
        cache[key] = formatter
    }
    return formatter.string(from: Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000))
}

private func nexaDevLocalizedTime(_ milliseconds: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000)
    return DateFormatter.localizedString(from: date, dateStyle: .none, timeStyle: .short)
}

private func nexaDevLocalizedDateTime(_ milliseconds: Int64) -> String {
    let date = Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000)
    return DateFormatter.localizedString(from: date, dateStyle: .short, timeStyle: .short)
}

private func nexaDevFormatDuration(_ seconds: Double) -> String {
    guard seconds.isFinite, seconds > 0 else { return "00:00" }
    let wholeSeconds = Int(seconds.rounded(.down))
    let minutes = wholeSeconds / 60
    let remainder = wholeSeconds % 60
    return "\(minutes < 10 ? "0" : "")\(minutes):\(remainder < 10 ? "0" : "")\(remainder)"
}

/// Mirrors generated `Time.format` while reusing formatters between hot-reload renders.
private func nexaDevFormatDate(_ milliseconds: Int64, _ pattern: String) -> String {
    let locale = Locale.current
    let calendar = Calendar.current
    let timeZone = TimeZone.current
    let key = "nexa.dev.date.\(locale.identifier).\(String(describing: calendar.identifier)).\(timeZone.identifier).\(pattern)"
    let cache = Thread.current.threadDictionary
    let formatter: DateFormatter
    if let cached = cache[key] as? DateFormatter {
        formatter = cached
    } else {
        let cachedKeys = cache.allKeys.compactMap { cachedKey -> String? in
            guard let key = cachedKey as? String, key.hasPrefix("nexa.dev.date.") else { return nil }
            return key
        }
        if cachedKeys.count >= 32 {
            cachedKeys.forEach { cache.removeObject(forKey: $0) }
        }
        formatter = DateFormatter()
        formatter.locale = locale
        formatter.calendar = calendar
        formatter.timeZone = timeZone
        formatter.dateFormat = pattern
        cache[key] = formatter
    }
    return formatter.string(from: Date(timeIntervalSince1970: TimeInterval(milliseconds) / 1000))
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
