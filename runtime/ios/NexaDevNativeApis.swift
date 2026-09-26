import Foundation

@MainActor
extension NexaDevStateStore {
    func invokeNativeAsync(
        _ call: [String: Any],
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
        let namespace = call["namespace"] as? String ?? ""
        let name = call["name"] as? String ?? ""
        var options: [String: Any] = [:]
        for argument in call["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
            guard let argumentName = argument[0] as? String else { continue }
            options[argumentName] = try await evaluateAsync(argument[1], locals: locals, scope: scope)
        }
        func stringOption(_ key: String, _ fallback: String = "") -> String {
            options[key] as? String ?? fallback
        }
        func numberOption(_ key: String, _ fallback: Double) -> Double {
            (options[key] as? NSNumber)?.doubleValue ?? fallback
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
        guard namespace == "Network", name == "fetch" || name == "download" else {
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
        var options: [String: Any] = [:]
        for argument in call["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
            guard let argumentName = argument[0] as? String else { continue }
            options[argumentName] = evaluate(argument[1], locals: locals, scope: scope)
        }
        func stringOption(_ key: String) -> String { options[key] as? String ?? "" }
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
