import Foundation
import SwiftUI

/// Route token encapsulating target screen and arguments for SwiftUI NavigationStack.
struct NexaDevRoute: Hashable {
    let token: String
}

/// Dynamic model representing an active bottom sheet in the Dev IR tree.
struct NexaDevBottomSheet {
    let state: String
    let partial: Bool
    let children: [Any]
    let scope: String
}

/// Recursively find a BottomSheet node within the given node tree.
func devBottomSheet(in nodes: [Any], scope: String = "app") -> NexaDevBottomSheet? {
    for rawNode in nodes {
        guard let tagged = rawNode as? [String: Any],
              let (kind, rawFields) = tagged.first
        else { continue }
        let fields = rawFields as? [String: Any] ?? [:]
        if kind == "BottomSheet" {
            return NexaDevBottomSheet(
                state: fields["state"] as? String ?? "",
                partial: fields["partial"] as? Bool ?? false,
                children: fields["children"] as? [Any] ?? [],
                scope: scope
            )
        }
        for key in ["children", "then_body", "else_body"] {
            if let nested = fields[key] as? [Any],
               let sheet = devBottomSheet(in: nested, scope: scope) {
                return sheet
            }
        }
        if let cases = fields["cases"] as? [[String: Any]] {
            for item in cases {
                if let nested = item["body"] as? [Any],
                   let sheet = devBottomSheet(in: nested, scope: scope) {
                    return sheet
                }
            }
        }
    }
    return nil
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
