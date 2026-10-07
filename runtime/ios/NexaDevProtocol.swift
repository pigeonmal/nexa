import Foundation
import SwiftUI

struct NexaDevDiagnostic: Identifiable, Equatable {
    let id = UUID()
    let file: String
    let line: Int
    let column: Int
    let message: String
    let stackTrace: String?

    init(file: String, line: Int, column: Int, message: String, stackTrace: String? = nil) {
        self.file = file
        self.line = line
        self.column = column
        self.message = message
        self.stackTrace = stackTrace
    }

    var sourceLocation: String { "\(file):\(line):\(column)" }
    var isRuntimeFailure: Bool { file == "<runtime>" }
}

/// WebSocket protocol and session management for the Nexa development runtime.
@MainActor
final class NexaDevRuntime: ObservableObject {
    @Published var module: [String: Any]?
    @Published var diagnostics: [NexaDevDiagnostic] = []
    @Published var performanceOverlayEnabled = false
    let store = NexaDevStateStore()

    private let serverURL: String
    private let sessionToken: String
    private var socket: URLSessionWebSocketTask?
    private var connectionTask: Task<Void, Never>?
    private var currentRevision: String?

    init(serverURL: String, sessionToken: String) {
        self.serverURL = serverURL
        self.sessionToken = sessionToken
        store.onRuntimeFailure = { [weak self] error, stackTrace in
            self?.reportRuntimeFailure(error, stackTrace: stackTrace)
        }
    }

    func reportRuntimeFailure(_ error: Error, stackTrace: String) {
        diagnostics = [NexaDevDiagnostic(
            file: "<runtime>",
            line: 1,
            column: 1,
            message: String(describing: error),
            stackTrace: "\(String(describing: error))\n\nCall stack:\n\(stackTrace)"
        )]
    }

    func connect() {
        guard connectionTask == nil, let url = URL(string: serverURL) else { return }
        connectionTask = Task { [weak self] in
            guard let self else { return }
            while !Task.isCancelled {
                let socket = URLSession.shared.webSocketTask(with: url)
                self.socket = socket
                socket.resume()
                do {
                    try await socket.send(.string(try Self.encode([
                        NexaDevKeys.type: NexaDevKeys.msgHello,
                        NexaDevKeys.payload: [
                            NexaDevKeys.protocolVersion: NexaDevSchema.protocolVersion,
                            NexaDevKeys.sessionToken: sessionToken,
                            NexaDevKeys.target: NexaDevSchema.targetPlatform,
                        ],
                    ])))
                    while !Task.isCancelled {
                        let message = try await socket.receive()
                        guard case let .string(text) = message else { continue }
                        self.receive(text)
                    }
                } catch {
                    guard !Task.isCancelled else { break }
                    print("Nexa dev connection ended: \(error); retrying")
                }
                self.socket = nil
                if !Task.isCancelled {
                    try? await Task.sleep(for: .seconds(1))
                }
            }
            self.connectionTask = nil
        }
    }

    private func receive(_ text: String) {
        guard let data = text.data(using: .utf8),
              let envelope = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let kind = envelope[NexaDevKeys.type] as? String,
              let payload = envelope[NexaDevKeys.payload] as? [String: Any]
        else { return }

        switch kind {
        case NexaDevKeys.msgFullModule:
            guard let devModule = payload[NexaDevKeys.module] as? [String: Any],
                  let nextModule = devModule[NexaDevKeys.module] as? [String: Any]
            else { return }
            store.installTranslations(devModule["translations"])
            store.install(module: nextModule)
            currentRevision = devModule[NexaDevKeys.revision] as? String
            diagnostics = []
            module = nextModule
            if let currentRevision { acknowledge(currentRevision) }
        case NexaDevKeys.msgPatch:
            guard let patch = payload[NexaDevKeys.msgPatch] as? [String: Any],
                  let baseRevision = patch[NexaDevKeys.baseRevision] as? String,
                  let revision = patch[NexaDevKeys.revision] as? String,
                  let operations = patch[NexaDevKeys.operations] as? [[String: Any]],
                  var nextModule = module
            else { return }
            guard currentRevision == baseRevision else {
                requestFullModule()
                return
            }
            guard Self.apply(operations, to: &nextModule) else {
                requestFullModule()
                return
            }
            if let translations = patch["translations"] {
                store.installTranslations(translations)
            }
            store.install(module: nextModule)
            currentRevision = revision
            diagnostics = []
            module = nextModule
            acknowledge(revision)
        case NexaDevKeys.msgDiagnostics:
            diagnostics = (payload[NexaDevKeys.diagnostics] as? [[String: Any]] ?? []).map { diagnostic in
                let file = diagnostic[NexaDevKeys.file] as? String ?? "<unknown>"
                let line = max(diagnostic[NexaDevKeys.line] as? Int ?? 1, 1)
                let column = max(diagnostic[NexaDevKeys.column] as? Int ?? 1, 1)
                let message = diagnostic[NexaDevKeys.message] as? String ?? "compile error"
                let value = NexaDevDiagnostic(file: file, line: line, column: column, message: message)
                print("\(value.sourceLocation): \(message)")
                return value
            }
        case NexaDevKeys.msgRestart:
            if let module { store.hotRestart(module: module) }
            print("Nexa dev hot restart: \(payload[NexaDevKeys.reason] ?? "requested")")
        case NexaDevKeys.msgPerformanceOverlay:
            performanceOverlayEnabled = payload[NexaDevKeys.enabled] as? Bool ?? false
        default:
            break
        }
    }

    private func requestFullModule() {
        guard let socket else { return }
        Task {
            try? await socket.send(.string(try Self.encode([NexaDevKeys.type: NexaDevKeys.msgRequestFullModule])))
        }
    }

    private func acknowledge(_ revision: String) {
        guard let socket else { return }
        Task {
            try? await socket.send(.string(try Self.encode([
                NexaDevKeys.type: NexaDevKeys.msgAcknowledge,
                NexaDevKeys.payload: [NexaDevKeys.revision: revision],
            ])))
        }
    }

    func openInEditor(_ diagnostic: NexaDevDiagnostic) {
        guard let socket else { return }
        Task<Void, Never> {
            guard let encoded = try? Self.encode([
                NexaDevKeys.type: NexaDevKeys.msgOpenInEditor,
                NexaDevKeys.payload: [
                    NexaDevKeys.file: diagnostic.file,
                    NexaDevKeys.line: diagnostic.line,
                    NexaDevKeys.column: diagnostic.column,
                ],
            ]) else { return }
            try? await socket.send(.string(encoded))
        }
    }

    private static func apply(
        _ operations: [[String: Any]],
        to module: inout [String: Any]
    ) -> Bool {
        for operation in operations {
            guard let kind = operation[NexaDevKeys.opSet] as? String ?? operation["op"] as? String,
                  let path = operation[NexaDevKeys.opPath] as? String
            else { return false }
            let tokens = path.split(separator: "/").map {
                $0.replacingOccurrences(of: "~1", with: "/")
                    .replacingOccurrences(of: "~0", with: "~")
            }
            guard !tokens.isEmpty else { return false }
            var valid = true
            guard let updated = update(
                module,
                tokens: tokens,
                index: 0,
                kind: kind,
                value: operation[NexaDevKeys.opValue],
                valid: &valid
            ) as? [String: Any], valid else { return false }
            module = updated
        }
        return true
    }

    private static func update(
        _ current: Any,
        tokens: [String],
        index: Int,
        kind: String,
        value: Any?,
        valid: inout Bool
    ) -> Any? {
        guard index < tokens.count else {
            valid = false
            return nil
        }
        let key = tokens[index]
        let isLeaf = index == tokens.count - 1
        if var object = current as? [String: Any] {
            if isLeaf {
                switch kind {
                case NexaDevKeys.opSet:
                    guard let value else { valid = false; return nil }
                    object[key] = value
                case NexaDevKeys.opRemove: object.removeValue(forKey: key)
                default: valid = false; return nil
                }
            } else {
                guard let child = object[key],
                      let updated = update(
                        child,
                        tokens: tokens,
                        index: index + 1,
                        kind: kind,
                        value: value,
                        valid: &valid
                      ), valid
                else { valid = false; return nil }
                object[key] = updated
            }
            return object
        }
        if var array = current as? [Any], let arrayIndex = Int(key), array.indices.contains(arrayIndex) {
            if isLeaf {
                guard kind == NexaDevKeys.opSet, let value else { valid = false; return nil }
                array[arrayIndex] = value
            } else {
                guard let updated = update(
                    array[arrayIndex],
                    tokens: tokens,
                    index: index + 1,
                    kind: kind,
                    value: value,
                    valid: &valid
                ), valid else { valid = false; return nil }
                array[arrayIndex] = updated
            }
            return array
        }
        valid = false
        return nil
    }

    private static func encode(_ object: [String: Any]) throws -> String {
        let data = try JSONSerialization.data(withJSONObject: object)
        guard let value = String(data: data, encoding: .utf8) else {
            throw NSError(domain: "NexaDevRuntime", code: 1)
        }
        return value
    }
}
