import Foundation
import SwiftUI

/// Route token encapsulating target screen and arguments for SwiftUI NavigationStack.
struct NexaDevRoute: Hashable {
    let token: String
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
    func navigationRoute(screen: String, values: [String: Any], signature: String) -> NexaDevRoute {
        let token = "\(screen):\(Self.canonicalJSON(values))"
        routeArguments[token] = (screen, values, signature)
        return NexaDevRoute(token: token)
    }

    func routeDestination(_ route: NexaDevRoute) -> (screen: String, values: [String: Any])? {
        guard let destination = routeArguments[route.token] else { return nil }
        return (destination.screen, destination.values)
    }
}
