import Foundation
import SwiftUI

/// Route token encapsulating unique entry identity, target screen and navigator for SwiftUI NavigationStack.
struct NexaDevRoute: Hashable, Identifiable {
    let entryId: String
    let screen: String
    let navigatorId: String
    var id: String { entryId }

    var token: String { entryId }
}

struct NexaDevNavigationStackIdentityKey: EnvironmentKey {
    static let defaultValue: String? = nil
}

struct NexaDevNavigationPathKey: EnvironmentKey {
    static let defaultValue: Binding<[NexaDevRoute]>? = nil
}

extension EnvironmentValues {
    var nexaDevNavigationStackIdentity: String? {
        get { self[NexaDevNavigationStackIdentityKey.self] }
        set { self[NexaDevNavigationStackIdentityKey.self] = newValue }
    }

    var nexaDevNavigationPath: Binding<[NexaDevRoute]>? {
        get { self[NexaDevNavigationPathKey.self] }
        set { self[NexaDevNavigationPathKey.self] = newValue }
    }
}

@MainActor
extension NexaDevStateStore {
    func navigationRoute(
        screen: String,
        values: [String: Any],
        signature: String,
        navigatorId: String = "app/root"
    ) -> NexaDevRoute {
        let entryId = "entry_\(UUID().uuidString.prefix(8))"
        routeArguments[entryId] = (screen, values, signature)
        return NexaDevRoute(entryId: entryId, screen: screen, navigatorId: navigatorId)
    }

    func routeDestination(_ route: NexaDevRoute) -> (screen: String, values: [String: Any])? {
        guard let destination = routeArguments[route.entryId] else { return nil }
        return (destination.screen, destination.values)
    }

    func handleDeepLink(
        url: URL,
        screens: [[String: Any]],
        stackIdentity: String
    ) {
        let scheme = url.scheme?.lowercased() ?? ""
        let queryItems = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems ?? []
        var segments = url.pathComponents.dropFirst().map { $0.removingPercentEncoding ?? $0 }
        if scheme != "http", scheme != "https", let host = url.host {
            segments.insert(host, at: 0)
        }
        guard let routeSlug = segments.first?.lowercased() else { return }

        for screen in screens {
            guard let name = screen["name"] as? String else { continue }
            let slug = nexaRouteSlug(name)
            guard slug == routeSlug else { continue }

            let parameters = screen["parameters"] as? [[String: Any]] ?? []
            guard segments.count <= parameters.count + 1 else { return }

            var values: [String: Any] = [:]
            for (index, param) in parameters.enumerated() {
                guard let paramName = param["name"] as? String else { continue }
                var raw = queryItems.first(where: { $0.name == paramName })?.value
                if segments.count > index + 1 {
                    raw = segments[index + 1]
                }
                guard let raw else { return }
                let paramType = param["type"] as? String ?? "String"
                switch paramType {
                case "String":
                    values[paramName] = raw
                case "Bool":
                    guard let b = Bool(raw) else { return }
                    values[paramName] = b
                case "Int32", "Int", "Int64":
                    guard let i = Int(raw) else { return }
                    values[paramName] = i
                case "Float32", "Float64", "Double":
                    guard let d = Double(raw) else { return }
                    values[paramName] = d
                default:
                    values[paramName] = raw
                }
            }

            let route = navigationRoute(
                screen: name,
                values: values,
                signature: UUID().uuidString,
                navigatorId: stackIdentity
            )
            var currentPath = navigationPath(for: stackIdentity)
            currentPath.append(route)
            setNavigationPath(currentPath, for: stackIdentity)
            invalidateScope(stackIdentity)
            return
        }
    }
}

private func nexaRouteSlug(_ name: String) -> String {
    var slug = ""
    for (index, char) in name.enumerated() {
        if char.isUppercase && index > 0 {
            slug.append("-")
        }
        slug.append(contentsOf: char.lowercased())
    }
    return slug
}
