import Foundation
import UIKit
import Network

final class NexaDevNetworkPathStatus: @unchecked Sendable {
    static let shared = NexaDevNetworkPathStatus()

    private let monitor = NWPathMonitor()
    private let lock = NSLock()
    private var online = false
    private var statusHandlers: [String: (Bool) -> Void] = [:]

    private init() {
        monitor.pathUpdateHandler = { [weak self] path in
            self?.setOnline(path.status == .satisfied)
        }
        monitor.start(queue: DispatchQueue(label: "dev.nexa.dev-network-path-status"))
    }

    var isOnline: Bool {
        lock.lock()
        defer { lock.unlock() }
        return online
    }

    private func setOnline(_ value: Bool) {
        lock.lock()
        let changed = online != value
        online = value
        lock.unlock()
        guard changed else { return }
        DispatchQueue.main.async { [weak self] in
            self?.notifyStatusChange(value)
        }
    }

    func setStatusHandler(id: String, handler: ((Bool) -> Void)?) {
        lock.lock()
        statusHandlers[id] = handler
        lock.unlock()
    }

    private func notifyStatusChange(_ value: Bool) {
        lock.lock()
        let handlers = Array(statusHandlers.values)
        lock.unlock()
        handlers.forEach { $0(value) }
    }
}

private func nexaDevFormatCurrency(_ amount: Double, _ currencyCode: String) -> String {
    let formatter = NumberFormatter()
    formatter.locale = .current
    let code = currencyCode.uppercased()
    if Locale.commonISOCurrencyCodes.contains(code) {
        formatter.currencyCode = code
        formatter.numberStyle = .currency
    } else {
        formatter.numberStyle = .decimal
    }
    return formatter.string(from: NSNumber(value: amount)) ?? String(amount)
}

@MainActor
private func nexaDevPerformHaptics(_ name: String, options: [String: Any]) {
    switch name {
    case "impact":
        let style: UIImpactFeedbackGenerator.FeedbackStyle
        switch options["style"] as? String {
        case "Medium": style = .medium
        case "Heavy": style = .heavy
        default: style = .light
        }
        UIImpactFeedbackGenerator(style: style).impactOccurred()
    case "notification":
        let kind: UINotificationFeedbackGenerator.FeedbackType =
            (options["kind"] as? String) == "Error" ? .error : .success
        UINotificationFeedbackGenerator().notificationOccurred(kind)
    case "selection":
        UISelectionFeedbackGenerator().selectionChanged()
    default:
        break
    }
}

@MainActor
private func nexaDevLockOrientation(_ mode: String) {
    let mask: UIInterfaceOrientationMask
    switch mode {
    case "Portrait":
        mask = .portrait
    case "Landscape":
        mask = .landscape
    default:
        mask = UIDevice.current.userInterfaceIdiom == .pad ? .all : .allButUpsideDown
    }
    guard let scene = UIApplication.shared.connectedScenes
        .compactMap({ $0 as? UIWindowScene })
        .first(where: { $0.activationState == .foregroundActive })
    else {
        return
    }
    scene.requestGeometryUpdate(.iOS(interfaceOrientations: mask)) { _ in }
}

@MainActor
extension NexaDevStateStore {
    func invokeNativeAsync(
        _ call: [String: Any],
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
        let namespace = call["namespace"] as? String ?? ""
        let name = call["name"] as? String ?? ""
        let codecs = (call["codecs"] as? [Any]) ?? []
        var options: [String: Any] = [:]
        for argument in call["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
            guard let argumentName = argument[0] as? String else { continue }
            options[argumentName] = try await evaluateAsync(argument[1], locals: locals, scope: scope)
        }
        if let receiverExpression = call["receiver"], !(receiverExpression is NSNull) {
            let receiver = try await evaluateAsync(receiverExpression, locals: locals, scope: scope)
            let pluginResult = try await NexaDevPluginBridge.invokeInstanceAsync(
                receiver: receiver,
                namespace: namespace,
                name: name,
                options: options,
                codecs: codecs,
                enumCases: enumCases
            )
            if pluginResult.0 {
                if let failure = pluginResult.1 as? NexaDevPluginFailure {
                    throw failure
                }
                return pluginResult.1
            }
        } else {
            let pluginResult = try await NexaDevPluginBridge.invokeAsync(
                namespace: namespace,
                name: name,
                options: options,
                codecs: codecs,
                enumCases: enumCases
            )
            if pluginResult.0 {
                if let failure = pluginResult.1 as? NexaDevPluginFailure {
                    throw failure
                }
                return pluginResult.1
            }
        }
        if namespace == "Keyboard", name == "dismiss" {
            UIApplication.shared.sendAction(
                #selector(UIResponder.resignFirstResponder),
                to: nil,
                from: nil,
                for: nil
            )
            return NSNull()
        }
        if namespace == "Clipboard" {
            switch name {
            case "setText":
                UIPasteboard.general.string = options["text"] as? String ?? ""
                return NSNull()
            case "getText": return (UIPasteboard.general.string as Any?) ?? NSNull()
            case "hasText": return UIPasteboard.general.hasStrings
            default:
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported clipboard call \\(name)"]
                )
            }
        }
        if namespace == "Storage" {
            switch name {
            case "getString": return (NexaStorage.getString(options["key"] as? String ?? "") as Any?) ?? NSNull()
            case "setString":
                NexaStorage.setString(options["key"] as? String ?? "", options["value"] as? String ?? "")
                return NSNull()
            case "delete":
                NexaStorage.delete(options["key"] as? String ?? "")
                return NSNull()
            case "clear":
                NexaStorage.clear()
                return NSNull()
            default:
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported storage call \\(name)"]
                )
            }
        }
        if namespace == "Haptics" {
            nexaDevPerformHaptics(name, options: options)
            return NSNull()
        }
        if namespace == "Screen", name == "lockOrientation" {
            nexaDevLockOrientation(options["mode"] as? String ?? "All")
            return NSNull()
        }
        func stringOption(_ key: String, _ fallback: String = "") -> String {
            options[key] as? String ?? fallback
        }
        func numberOption(_ key: String, _ fallback: Double) -> Double {
            (options[key] as? NSNumber)?.doubleValue ?? fallback
        }
        if namespace == "Number", name == "formatCurrency" {
            return nexaDevFormatCurrency(numberOption("amount", 0), stringOption("currencyCode"))
        }
        if namespace == "Crypto" {
            switch name {
            case "sha256": return nexaCryptoSha256(stringOption("text"))
            case "sha512": return nexaCryptoSha512(stringOption("text"))
            case "hmacSha256":
                return nexaCryptoHmacSha256(stringOption("key"), stringOption("message"))
            case "randomBytes": return nexaCryptoRandomBytes(Int32(numberOption("count", 0)))
            default: break
            }
        }
        if namespace == "Path" {
            switch name {
            case "documents": return NexaPath.documents()
            case "caches": return NexaPath.caches()
            case "temporary": return NexaPath.temporary()
            case "appSupport": return NexaPath.appSupport()
            case "join": return NexaPath.join(stringOption("path"), stringOption("component"))
            default:
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported native call \(namespace).\(name)"]
                )
            }
        }
        if namespace == "File", name == "exists" {
            return NexaFile.exists(stringOption("path"))
        }
        if namespace == "File" {
            switch name {
            case "readText": return try await NexaFile.readText(stringOption("path"))
            case "writeText": return try await NexaFile.writeText(
                stringOption("contents"), to: stringOption("path")
            )
            case "delete": return try await NexaFile.delete(stringOption("path"))
            default:
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported async native call \(namespace).\(name)"]
                )
            }
        }
        if namespace == "SecureStorage" {
            switch name {
            case "get":
                let value = try await NexaSecureStorage.get(stringOption("key"))
                return (value as Any?) ?? NSNull()
            case "set":
                try await NexaSecureStorage.set(stringOption("key"), stringOption("value"))
                return NSNull()
            case "delete":
                try await NexaSecureStorage.delete(stringOption("key"))
                return NSNull()
            case "clear":
                try await NexaSecureStorage.clear()
                return NSNull()
            default:
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported secure storage call \(name)"]
                )
            }
        }
        if namespace == "Permissions" {
            let permissionName = stringOption("permission")
            let permission: NexaPermission
            switch permissionName {
            case "Camera": permission = .Camera
            case "Microphone": permission = .Microphone
            case "Photos": permission = .Photos
            case "Location": permission = .Location
            case "Notifications": permission = .Notifications
            case "Contacts": permission = .Contacts
            case "Calendar": permission = .Calendar
            case "Bluetooth": permission = .Bluetooth
            default:
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported permission \(permissionName)"]
                )
            }
            let status: NexaPermissionStatus
            if name == "request" {
                status = await NexaPermissions.request(permission)
            } else if name == "status" {
                status = await NexaPermissions.status(permission)
            } else {
                throw NSError(
                    domain: "NexaDevRuntime",
                    code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported permission call \(name)"]
                )
            }
            return String(describing: status)
        }
        guard namespace == "Network", name == "fetch" || name == "download" || name == "upload" else {
            throw NSError(
                domain: "NexaDevRuntime",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Unsupported async native call \(namespace).\(name)"]
            )
        }
        let headers = options["headers"] as? [String: String] ?? [:]
        let pins = Set(options["certificatePins"] as? Set<String> ?? [])
        let body = (options["body"] as? String).map { Data($0.utf8) }
        let url = stringOption("url")
        let method = stringOption("method", "GET")
        let timeout = numberOption("timeout", 30)
        let useCache = options["useCache"] as? Bool ?? true
        let followRedirects = options["followRedirects"] as? Bool ?? true
        let maxResponseBytes = Int(numberOption("maxResponseBytes", 67_108_864))
        if name == "upload" {
            let rawFields = options["fields"] as? [String: String] ?? [:]
            let response = try await NexaNetwork.upload(
                url: url,
                file: stringOption("file"),
                fields: rawFields
            )
            return [
                "statusCode": response.statusCode,
                "headers": response.headers,
                "body": response.text
            ]
        }
        if name == "download" {
            return try await NexaNetwork.download(
                url: url,
                destinationPath: stringOption("destinationPath"),
                method: method,
                body: body,
                headers: headers,
                timeout: timeout,
                useCache: useCache,
                followRedirects: followRedirects,
                maxResponseBytes: maxResponseBytes,
                certificatePins: pins
            )
        }
        let response = try await NexaNetwork.fetch(
            url: url,
            method: method,
            body: body,
            headers: headers,
            timeout: timeout,
            useCache: useCache,
            followRedirects: followRedirects,
            maxResponseBytes: maxResponseBytes,
            certificatePins: pins
        )
        return [
            "statusCode": response.statusCode,
            "headers": response.headers,
            "body": response.text
        ]
    }

    func invokeNativeSync(
        _ call: [String: Any],
        locals: [String: Any],
        scope: String
    ) -> Any {
        let namespace = call["namespace"] as? String ?? ""
        let name = call["name"] as? String ?? ""
        let codecs = (call["codecs"] as? [Any]) ?? []
        var options: [String: Any] = [:]
        for argument in call["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
            guard let argumentName = argument[0] as? String else { continue }
            options[argumentName] = evaluate(argument[1], locals: locals, scope: scope)
        }
        if let receiverExpression = call["receiver"], !(receiverExpression is NSNull) {
            let receiver = evaluate(receiverExpression, locals: locals, scope: scope)
            let pluginResult = NexaDevPluginBridge.invokeInstanceSync(
                receiver: receiver,
                namespace: namespace,
                name: name,
                options: options,
                codecs: codecs,
                enumCases: enumCases
            )
            if pluginResult.0 {
                if let failure = pluginResult.1 as? NexaDevPluginFailure {
                    pendingPluginFailure = failure
                    return NSNull()
                }
                return pluginResult.1
            }
        } else {
            let pluginResult = NexaDevPluginBridge.invokeSync(
                namespace: namespace,
                name: name,
                options: options,
                codecs: codecs,
                enumCases: enumCases
            )
            if pluginResult.0 {
                if let failure = pluginResult.1 as? NexaDevPluginFailure {
                    pendingPluginFailure = failure
                    return NSNull()
                }
                return pluginResult.1
            }
        }
        if namespace == "Keyboard", name == "dismiss" {
            UIApplication.shared.sendAction(
                #selector(UIResponder.resignFirstResponder),
                to: nil,
                from: nil,
                for: nil
            )
            return NSNull()
        }
        if namespace == "Network", name == "isOnline" {
            return NexaDevNetworkPathStatus.shared.isOnline
        }
        if namespace == "Json", let codec = codecs.first {
            switch name {
            case "parse":
                return NexaDevValueCodec.parseJSON(
                    options["raw"] as? String ?? "",
                    type: codec,
                    enumCases: enumCases
                )
            case "stringify":
                return NexaDevValueCodec.stringifyJSON(
                    options["value"],
                    type: codec,
                    enumCases: enumCases
                )
            default:
                return NSNull()
            }
        }
        if namespace == "Clipboard" {
            switch name {
            case "setText":
                UIPasteboard.general.string = options["text"] as? String ?? ""
                return NSNull()
            case "getText": return (UIPasteboard.general.string as Any?) ?? NSNull()
            case "hasText": return UIPasteboard.general.hasStrings
            default: return NSNull()
            }
        }
        if namespace == "Storage" {
            switch name {
            case "getString": return (NexaStorage.getString(options["key"] as? String ?? "") as Any?) ?? NSNull()
            case "setString":
                NexaStorage.setString(options["key"] as? String ?? "", options["value"] as? String ?? "")
                return NSNull()
            case "delete":
                NexaStorage.delete(options["key"] as? String ?? "")
                return NSNull()
            case "clear":
                NexaStorage.clear()
                return NSNull()
            default: return NSNull()
            }
        }
        if namespace == "Haptics" {
            nexaDevPerformHaptics(name, options: options)
            return NSNull()
        }
        if namespace == "Screen", name == "lockOrientation" {
            nexaDevLockOrientation(options["mode"] as? String ?? "All")
            return NSNull()
        }
        func stringOption(_ key: String) -> String { options[key] as? String ?? "" }
        if namespace == "Number", name == "formatCurrency" {
            let amount = (options["amount"] as? NSNumber)?.doubleValue ?? 0
            return nexaDevFormatCurrency(amount, stringOption("currencyCode"))
        }
        if namespace == "Crypto" {
            switch name {
            case "sha256": return nexaCryptoSha256(stringOption("text"))
            case "sha512": return nexaCryptoSha512(stringOption("text"))
            case "hmacSha256":
                return nexaCryptoHmacSha256(stringOption("key"), stringOption("message"))
            case "randomBytes":
                let count = (options["count"] as? NSNumber)?.int32Value ?? 0
                return nexaCryptoRandomBytes(count)
            default: break
            }
        }
        switch namespace {
        case "Path":
            switch name {
            case "documents": return NexaPath.documents()
            case "caches": return NexaPath.caches()
            case "temporary": return NexaPath.temporary()
            case "appSupport": return NexaPath.appSupport()
            case "join": return NexaPath.join(stringOption("path"), stringOption("component"))
            default: return NSNull()
            }
        case "File":
            return name == "exists" ? NexaFile.exists(stringOption("path")) : NSNull()
        default:
            return NSNull()
        }
    }
}
