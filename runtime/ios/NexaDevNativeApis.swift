import Foundation
import UIKit
import Network

private func nexaCurrentLanguageCode() -> String {
    guard let language = Locale.current.identifier
        .split(whereSeparator: { $0 == "-" || $0 == "_" })
        .first,
        language.count > 1
    else {
        return "und"
    }
    return language.lowercased()
}

private func nexaDevResponseHeaderValues(_ response: Any, name: String) -> [String] {
    let fields = response as? [String: Any] ?? [:]
    let headers = fields["headers"] as? [[String: Any]] ?? []
    var values: [String] = []
    for header in headers {
        guard let headerName = header["name"] as? String,
              headerName.caseInsensitiveCompare(name) == .orderedSame,
              let value = header["value"] as? String else { continue }
        values.append(value)
    }
    return values
}

private func nexaDevResponseHeaderValue(_ response: Any, name: String) -> String? {
    let fields = response as? [String: Any] ?? [:]
    let headers = fields["headers"] as? [[String: Any]] ?? []
    for header in headers {
        guard let headerName = header["name"] as? String,
              headerName.caseInsensitiveCompare(name) == .orderedSame,
              let value = header["value"] as? String else { continue }
        return value
    }
    return nil
}

private func nexaDevCookieHeader(_ setCookieValues: [String]) -> String {
    setCookieValues.compactMap { value -> String? in
        guard let first = value
            .split(separator: ";", maxSplits: 1, omittingEmptySubsequences: false)
            .first else { return nil }
        let pair = String(first).trimmingCharacters(in: .whitespaces)
        guard let equals = pair.firstIndex(of: "="), equals != pair.startIndex else {
            return nil
        }
        let name = String(pair[..<equals]).trimmingCharacters(in: .whitespaces)
        let cookieValue = String(pair[pair.index(after: equals)...])
            .trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty else { return nil }
        return "\(name)=\(cookieValue)"
    }.joined(separator: "; ")
}

private func nexaDevNetworkResponseMethod(
    _ response: Any,
    name: String,
    headerName: String
) -> Any {
    if name == "getHeader" {
        return nexaDevResponseHeaderValue(response, name: headerName) ?? NSNull()
    }
    if name == "getCookie" {
        return nexaDevCookieHeader(nexaDevResponseHeaderValues(response, name: "Set-Cookie"))
    }
    return nexaDevResponseHeaderValues(response, name: headerName)
}

private func nexaDevStorageMethod(
    name: String,
    options: [String: Any],
    codecs: [Any],
    enumCases: [String: [String]]
) -> Any {
    let key = options["key"] as? String ?? ""
    switch name {
    case "getString": return (NexaStorage.getString(key) as Any?) ?? NSNull()
    case "setString":
        NexaStorage.setString(key, options["value"] as? String ?? "")
    case "getBool": return (NexaStorage.getBool(key) as Any?) ?? NSNull()
    case "setBool": NexaStorage.setBool(key, options["value"] as? Bool ?? false)
    case "getInt64": return (NexaStorage.getInt64(key) as Any?) ?? NSNull()
    case "setInt64": NexaStorage.setInt64(key, options["value"] as? Int64 ?? 0)
    case "getFloat64": return (NexaStorage.getFloat64(key) as Any?) ?? NSNull()
    case "setFloat64": NexaStorage.setFloat64(key, options["value"] as? Double ?? 0)
    case "get":
        guard let codec = codecs.first, let raw = NexaStorage.getJson(key) else { return NSNull() }
        let parsed = NexaDevValueCodec.parseJSON(raw, type: codec, enumCases: enumCases)
        return parsed["Ok"] ?? NSNull()
    case "set":
        guard let codec = codecs.first else { return NSNull() }
        NexaStorage.setJson(
            key,
            NexaDevValueCodec.stringifyJSON(options["value"], type: codec, enumCases: enumCases)
        )
    case "delete": NexaStorage.delete(key)
    case "clear": NexaStorage.clear()
    default: return NSNull()
    }
    return NSNull()
}

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

private func nexaDevRegexMatch(_ match: NSTextCheckingResult, text: String) -> [String: Any] {
    let value = Range(match.range, in: text).map { String(text[$0]) } ?? ""
    let groups: [Any] = (1..<match.numberOfRanges).map { index in
        let range = match.range(at: index)
        guard range.location != NSNotFound, let swiftRange = Range(range, in: text) else {
            return NSNull()
        }
        return String(text[swiftRange])
    }
    return [
        "value": value,
        "range": [
            "lowerBound": Int64(match.range.location),
            "upperBound": Int64(NSMaxRange(match.range)),
        ],
        "groups": groups,
    ]
}

private func nexaDevRegexFindAll(_ pattern: String, _ text: String) -> [[String: Any]] {
    guard let expression = try? NSRegularExpression(pattern: pattern) else { return [] }
    let range = NSRange(text.startIndex..., in: text)
    return expression.matches(in: text, range: range).map { nexaDevRegexMatch($0, text: text) }
}

private enum NexaDevSharedNativeResult {
    case handled(Any)
    case unhandled
}

@MainActor
extension NexaDevStateStore {
    private func invokeSharedNative(
        namespace: String,
        name: String,
        options: [String: Any],
        codecs: [Any]
    ) -> NexaDevSharedNativeResult {
        func stringOption(_ key: String, _ fallback: String = "") -> String {
            options[key] as? String ?? fallback
        }
        func numberOption(_ key: String, _ fallback: Double = 0) -> Double {
            (options[key] as? NSNumber)?.doubleValue ?? fallback
        }

        switch namespace {
        case "Network" where name == "isOnline":
            return .handled(NexaDevNetworkPathStatus.shared.isOnline)
        case "Keyboard" where name == "dismiss":
            UIApplication.shared.sendAction(
                #selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil
            )
            return .handled(NSNull())
        case "Haptics":
            nexaDevPerformHaptics(name, options: options)
            return .handled(NSNull())
        case "Screen" where name == "lockOrientation":
            nexaDevLockOrientation(options["mode"] as? String ?? "All")
            return .handled(NSNull())
        case "Json":
            guard let codec = codecs.first else { return .unhandled }
            switch name {
            case "parse":
                return .handled(NexaDevValueCodec.parseJSON(
                    options["raw"] as? String ?? "", type: codec, enumCases: enumCases
                ))
            case "stringify":
                return .handled(NexaDevValueCodec.stringifyJSON(
                    options["value"], type: codec, enumCases: enumCases
                ))
            default:
                return .unhandled
            }
        case "Clipboard":
            switch name {
            case "setText":
                UIPasteboard.general.string = options["text"] as? String ?? ""
                return .handled(NSNull())
            case "getText":
                return .handled((UIPasteboard.general.string as Any?) ?? NSNull())
            case "hasText":
                return .handled(UIPasteboard.general.hasStrings)
            default:
                return .unhandled
            }
        case "Storage":
            return .handled(nexaDevStorageMethod(
                name: name, options: options, codecs: codecs, enumCases: enumCases
            ))
        case "Number" where name == "formatCurrency":
            return .handled(nexaDevFormatCurrency(
                numberOption("amount"), stringOption("currencyCode")
            ))
        case "Locale":
            switch name {
            case "currentLanguageCode": return .handled(nexaCurrentLanguageCode())
            case "preferredLanguageCodes": return .handled(Locale.preferredLanguages)
            case "displayName":
                return .handled(
                    Locale.current.localizedString(forLanguageCode: stringOption("languageCode"))
                        ?? NSNull() as Any
                )
            default: return .unhandled
            }
        case "Crypto":
            switch name {
            case "sha256": return .handled(nexaCryptoSha256(stringOption("text")))
            case "sha512": return .handled(nexaCryptoSha512(stringOption("text")))
            case "hmacSha256":
                return .handled(nexaCryptoHmacSha256(stringOption("key"), stringOption("message")))
            case "randomBytes":
                return .handled(nexaCryptoRandomBytes(Int32(numberOption("count"))))
            default: return .unhandled
            }
        case "Path":
            switch name {
            case "documents": return .handled(NexaPath.documents())
            case "caches": return .handled(NexaPath.caches())
            case "temporary": return .handled(NexaPath.temporary())
            case "appSupport": return .handled(NexaPath.appSupport())
            case "join":
                return .handled(NexaPath.join(stringOption("path"), stringOption("component")))
            default: return .unhandled
            }
        case "File" where name == "exists":
            return .handled(NexaFile.exists(stringOption("path")))
        default:
            return .unhandled
        }
    }

    func invokeNativeAsync(
        _ call: [String: Any],
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
        let namespace = call["namespace"] as? String ?? ""
        let name = call["name"] as? String ?? ""
        runtimeTrace("native_call \(namespace).\(name)")
        let codecs = (call["codecs"] as? [Any]) ?? []
        var options: [String: Any] = [:]
        for argument in call["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
            guard let argumentName = argument[0] as? String else { continue }
            options[argumentName] = try await evaluateAsync(argument[1], locals: locals, scope: scope)
        }
        func stringOption(_ key: String, _ fallback: String = "") -> String {
            options[key] as? String ?? fallback
        }
        func numberOption(_ key: String, _ fallback: Double = 0) -> Double {
            (options[key] as? NSNumber)?.doubleValue ?? fallback
        }
        if namespace == "NexaFastListRef",
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull),
           let reference = try await evaluateAsync(rawReceiver, locals: locals, scope: scope) as? NexaFastListRef {
            return nexaDevInvokeFastListRef(reference, name: name, options: options)
        }
        if namespace == "NexaBottomSheetRef",
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull),
           let reference = try await evaluateAsync(rawReceiver, locals: locals, scope: scope) as? NexaBottomSheetRef {
            return nexaDevInvokeBottomSheetRef(reference, name: name)
        }
        if namespace == "Nexa",
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull) {
            guard let reference = try await evaluateAsync(rawReceiver, locals: locals, scope: scope) as? NexaDevComponentRef,
                  let componentScope = reference.attachedScope else {
                return ["Err": ["NotAttached": NSNull()]]
            }
            let functionName = "Nexa.\(reference.componentName).\(name)"
            guard let declaration = functions[functionName] else { return NSNull() }
            let parameters = declaration["parameters"] as? [[String: Any]] ?? []
            let arguments = parameters.map { parameter in
                options[parameter["name"] as? String ?? ""] ?? NSNull()
            }
            let functionCall: [String: Any] = ["name": functionName, "arguments": arguments]
            do {
                let result: Any
                if declaration["is_async"] as? Bool == true {
                    result = try await invokeFunctionAsync(
                        functionCall,
                        locals: reference.attachedParameters,
                        scope: componentScope
                    )
                } else {
                    result = evaluate(
                        ["Call": functionCall],
                        locals: reference.attachedParameters,
                        scope: componentScope
                    )
                }
                if let failure = pendingPluginFailure {
                    pendingPluginFailure = nil
                    return nexaDevComponentRefFailure(failure)
                }
                let returnsResult = (declaration["return_type"] as? [String: Any])?["Result"] != nil
                return nexaDevComponentRefResult(result, returnsResult: returnsResult)
            } catch let failure as NexaDevPluginFailure {
                pendingPluginFailure = nil
                return nexaDevComponentRefFailure(failure)
            } catch {
                throw error
            }
        }
        if namespace == "__NexaNetworkResponse",
           (name == "getHeader" || name == "getHeaders" || name == "getCookie"),
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull) {
            let response = try await evaluateAsync(rawReceiver, locals: locals, scope: scope)
            return nexaDevNetworkResponseMethod(
                response,
                name: name,
                headerName: options["name"] as? String ?? ""
            )
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
        if case let .handled(value) = invokeSharedNative(
            namespace: namespace, name: name, options: options, codecs: codecs
        ) {
            return value
        }
        if namespace == "AppIcon", name == "set" {
            let iconName = options["name"] as? String
            guard UIApplication.shared.supportsAlternateIcons else { return false }
            return await withCheckedContinuation { continuation in
                UIApplication.shared.setAlternateIconName(iconName) { error in
                    continuation.resume(returning: error == nil)
                }
            }
        }
        if namespace == "File" {
            switch name {
            case "readBytes": return try await NexaFile.read(stringOption("path")).base64EncodedString()
            case "readText": return try await NexaFile.readText(stringOption("path"))
            case "writeBytes":
                guard let data = Data(base64Encoded: stringOption("contents")) else {
                    throw NSError(domain: "NexaDevRuntime", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid byte buffer"])
                }
                try await NexaFile.write(data, to: stringOption("path"))
                return true
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
                let value = try await NexaSecureStorage.get(stringOption("key"), options["scope"] as? String)
                return (value as Any?) ?? NSNull()
            case "set":
                try await NexaSecureStorage.set(stringOption("key"), stringOption("value"), options["scope"] as? String)
                return NSNull()
            case "delete":
                try await NexaSecureStorage.delete(stringOption("key"), options["scope"] as? String)
                return NSNull()
            case "clear":
                try await NexaSecureStorage.clear(options["scope"] as? String)
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
        let cachePolicy = options["cachePolicy"] as? String ?? "Default"
        let followRedirects = options["followRedirects"] as? Bool ?? true
        let maxResponseBytes = Int(numberOption("maxResponseBytes", 67_108_864))
        if name == "upload" {
            let rawFields = options["fields"] as? [String: String] ?? [:]
            let response = try await NexaNetwork.upload(
                url: url,
                file: stringOption("file"),
                fields: rawFields,
                headers: headers,
                timeout: timeout,
                followRedirects: followRedirects,
                maxResponseBytes: maxResponseBytes,
                certificatePins: pins
            )
            return [
                "statusCode": response.statusCode,
                "headers": response.headers.map { ["name": $0.0, "value": $0.1] },
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
                cachePolicy: cachePolicy,
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
            cachePolicy: cachePolicy,
            followRedirects: followRedirects,
            maxResponseBytes: maxResponseBytes,
            certificatePins: pins
        )
        return [
            "statusCode": response.statusCode,
            "headers": response.headers.map { ["name": $0.0, "value": $0.1] },
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
        runtimeTrace("native_call \(namespace).\(name)")
        let codecs = (call["codecs"] as? [Any]) ?? []
        var options: [String: Any] = [:]
        for argument in call["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
            guard let argumentName = argument[0] as? String else { continue }
            options[argumentName] = evaluate(argument[1], locals: locals, scope: scope)
        }
        if namespace == "NexaFastListRef",
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull),
           let reference = evaluate(rawReceiver, locals: locals, scope: scope) as? NexaFastListRef {
            return nexaDevInvokeFastListRef(reference, name: name, options: options)
        }
        if namespace == "NexaBottomSheetRef",
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull),
           let reference = evaluate(rawReceiver, locals: locals, scope: scope) as? NexaBottomSheetRef {
            return nexaDevInvokeBottomSheetRef(reference, name: name)
        }
        if namespace == "Nexa",
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull) {
            guard let reference = evaluate(rawReceiver, locals: locals, scope: scope) as? NexaDevComponentRef,
                  let componentScope = reference.attachedScope else {
                return ["Err": ["NotAttached": NSNull()]]
            }
            let functionName = "Nexa.\(reference.componentName).\(name)"
            guard let declaration = functions[functionName] else { return NSNull() }
            let parameters = declaration["parameters"] as? [[String: Any]] ?? []
            let arguments = parameters.map { parameter in
                options[parameter["name"] as? String ?? ""] ?? NSNull()
            }
            let result = evaluate(
                ["Call": ["name": functionName, "arguments": arguments]],
                locals: reference.attachedParameters,
                scope: componentScope
            )
            if let failure = pendingPluginFailure {
                pendingPluginFailure = nil
                return nexaDevComponentRefFailure(failure)
            }
            let returnsResult = (declaration["return_type"] as? [String: Any])?["Result"] != nil
            return nexaDevComponentRefResult(result, returnsResult: returnsResult)
        }
        if namespace == "__NexaNetworkResponse",
           (name == "getHeader" || name == "getHeaders" || name == "getCookie"),
           let rawReceiver = call["receiver"],
           !(rawReceiver is NSNull) {
            let response = evaluate(rawReceiver, locals: locals, scope: scope)
            return nexaDevNetworkResponseMethod(
                response,
                name: name,
                headerName: options["name"] as? String ?? ""
            )
        }
        if namespace == "Regex" {
            let receiverExpression = call["receiver"]
            if let receiverExpression, !(receiverExpression is NSNull) {
                let pattern = evaluate(receiverExpression, locals: locals, scope: scope) as? String ?? ""
                let text = options["text"] as? String ?? ""
                switch name {
                case "matches":
                    guard let expression = try? NSRegularExpression(pattern: pattern) else { return false }
                    let range = NSRange(text.startIndex..., in: text)
                    guard let match = expression.firstMatch(in: text, range: range) else { return false }
                    return match.range.location == range.location && match.range.length == range.length
                case "find":
                    if let match = nexaDevRegexFindAll(pattern, text).first {
                        return match
                    }
                    return NSNull()
                case "findAll": return nexaDevRegexFindAll(pattern, text)
                case "replace":
                    guard let expression = try? NSRegularExpression(pattern: pattern) else { return text }
                    let range = NSRange(text.startIndex..., in: text)
                    return expression.stringByReplacingMatches(
                        in: text,
                        range: range,
                        withTemplate: options["with"] as? String ?? ""
                    )
                default: return NSNull()
                }
            }
            let pattern = options["pattern"] as? String ?? ""
            let text = options["text"] as? String ?? ""
            switch name {
            case "new": return pattern
            case "isMatch":
                guard let expression = try? NSRegularExpression(pattern: pattern) else { return false }
                let range = NSRange(text.startIndex..., in: text)
                return expression.firstMatch(in: text, range: range) != nil
            case "matches": return nexaDevRegexFindAll(pattern, text).map { $0["value"] as? String ?? "" }
            case "replace":
                guard let expression = try? NSRegularExpression(pattern: pattern) else { return text }
                let range = NSRange(text.startIndex..., in: text)
                return expression.stringByReplacingMatches(
                    in: text,
                    range: range,
                    withTemplate: options["replacement"] as? String ?? ""
                )
            default: return NSNull()
            }
        }
        if namespace == "__NexaMap",
           let receiverExpression = call["receiver"],
           !(receiverExpression is NSNull),
           let receiver = evaluate(receiverExpression, locals: locals, scope: scope) as? [String: Any] {
            let key = stringify(options["key"] ?? NSNull())
            switch name {
            case "get": return receiver[key] ?? NSNull()
            case "contains": return receiver[key] != nil
            case "keys": return Array(receiver.keys)
            case "values": return Array(receiver.values)
            default: return NSNull()
            }
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
        if case let .handled(value) = invokeSharedNative(
            namespace: namespace, name: name, options: options, codecs: codecs
        ) {
            return value
        }
        return NSNull()
    }
}

private func nexaDevComponentRefFailure(_ failure: NexaDevPluginFailure) -> [String: Any] {
    ["Err": ["Failure": [
        "namespace": failure.namespace,
        "errorType": failure.errorType,
        "variant": failure.variant,
        "payload": failure.payload,
    ]]]
}

private func nexaDevComponentRefResult(_ result: Any, returnsResult: Bool) -> [String: Any] {
    guard returnsResult, let outcome = result as? [String: Any] else { return ["Ok": result] }
    if let value = outcome["Ok"] { return ["Ok": value] }
    if let failure = outcome["Err"] { return ["Err": ["Failure": failure]] }
    return ["Ok": result]
}

@MainActor
private func nexaDevInvokeFastListRef(
    _ reference: NexaFastListRef,
    name: String,
    options: [String: Any]
) -> Any {
    let result: Result<Void, NexaComponentRefError<Never>>
    switch name {
    case "scrollToIndex":
        result = reference.scrollToIndex(
            (options["index"] as? NSNumber)?.int32Value ?? 0,
            animated: (options["animated"] as? Bool) ?? true,
            viewPosition: (options["viewPosition"] as? NSNumber)?.doubleValue ?? 0
        )
    case "scrollToTop": result = reference.scrollToTop(animated: (options["animated"] as? Bool) ?? true)
    case "scrollToEnd": result = reference.scrollToEnd(animated: (options["animated"] as? Bool) ?? true)
    case "scrollToOffset":
        result = reference.scrollToOffset(
            (options["offset"] as? NSNumber)?.doubleValue ?? 0,
            animated: (options["animated"] as? Bool) ?? true
        )
    default: return NSNull()
    }
    switch result {
    case .success: return ["Ok": NSNull()]
    case .failure(.notAttached): return ["Err": ["NotAttached": NSNull()]]
    case .failure(.failure(_)): return ["Err": ["NotAttached": NSNull()]]
    }
}

@MainActor
private func nexaDevInvokeBottomSheetRef(_ reference: NexaBottomSheetRef, name: String) -> Any {
    let result: Result<Void, NexaComponentRefError<Never>>
    switch name {
    case "present": result = reference.present()
    case "dismiss": result = reference.dismiss()
    default: return NSNull()
    }
    switch result {
    case .success: return ["Ok": NSNull()]
    case .failure(.notAttached): return ["Err": ["NotAttached": NSNull()]]
    case .failure(.failure(_)): return ["Err": ["NotAttached": NSNull()]]
    }
}
