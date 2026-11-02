import Combine
import Foundation
import QuartzCore
import SwiftUI

/// Test-only render fence.
///
/// An acknowledgement proves that Dev IR was committed, not that SwiftUI has
/// presented a frame that reads it. The store arms this fence with the installed
/// module generation; a probe view reports the next body evaluation at that
/// generation; the first display-link tick after that evaluation is treated as
/// the presented frame. Harnesses wait for this signal before capturing pixels.
@MainActor
final class NexaDevFrameFence {
    private var isEnabled = false
    private var pendingRevision: String?
    private var pendingGeneration = 0
    private var composedGeneration = -1
    private var frameCounter: UInt64 = 0
    private var displayLink: CADisplayLink?
    var onPresented: ((String, UInt64) -> Void)?

    func configure(enabled: Bool) {
        isEnabled = enabled
        if !enabled { reset() }
    }

    func arm(revision: String, generation: Int) {
        guard isEnabled else { return }
        pendingRevision = revision
        pendingGeneration = generation
        composedGeneration = -1
    }

    /// Called by the render probe after SwiftUI evaluated the tree at `generation`.
    func noteComposed(generation: Int) {
        guard isEnabled,
              pendingRevision != nil,
              generation == pendingGeneration
        else { return }
        composedGeneration = generation
        guard displayLink == nil else { return }
        let link = CADisplayLink(target: self, selector: #selector(tick(_:)))
        link.add(to: .main, forMode: .common)
        displayLink = link
    }

    @objc private func tick(_ link: CADisplayLink) {
        guard isEnabled,
              let revision = pendingRevision,
              composedGeneration == pendingGeneration
        else {
            stop()
            return
        }
        let frameId = frameCounter + 1
        frameCounter = frameId
        pendingRevision = nil
        composedGeneration = -1
        stop()
        onPresented?(revision, frameId)
    }

    private func stop() {
        displayLink?.invalidate()
        displayLink = nil
    }

    func reset() {
        stop()
        pendingRevision = nil
        pendingGeneration = 0
        composedGeneration = -1
    }
}

private final class NexaDevPatchTrie {
    var change: (kind: String, value: Any?)?
    var children: [String: NexaDevPatchTrie] = [:]
}

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
    var compactSourceLocation: String {
        "\(URL(fileURLWithPath: file).lastPathComponent):\(line):\(column)"
    }
    var isRuntimeFailure: Bool { file == "<runtime>" }
}

/// WebSocket protocol and session management for the Nexa development runtime.
@MainActor
final class NexaDevRuntime: ObservableObject {
    @Published var module: [String: Any]?
    @Published var diagnostics: [NexaDevDiagnostic] = []
    @Published var performanceOverlayEnabled = false
    let store = NexaDevStateStore()
    let frameFence = NexaDevFrameFence()

    private let serverURL: String
    private let sessionToken: String
    private var socket: URLSessionWebSocketTask?
    private var connectionTask: Task<Void, Never>?
    private var isConnecting = false
    private var currentRevision: String?
    private var storeSubscription: AnyCancellable?

    init(serverURL: String, sessionToken: String) {
        self.serverURL = serverURL
        self.sessionToken = sessionToken
        store.onRuntimeFailure = { [weak self] error, stackTrace in
            self?.reportRuntimeFailure(error, stackTrace: stackTrace)
        }
        frameFence.onPresented = { [weak self] revision, frameId in
            self?.sendFramePresented(revision: revision, frameId: frameId)
        }
        storeSubscription = store.objectWillChange.sink { [weak self] _ in
            self?.objectWillChange.send()
        }
    }

    func reportRuntimeFailure(_ error: Error, stackTrace: String) {
        let located = error as? NexaDevLocatedRuntimeError
        let pluginFailure = error as? NexaDevPluginFailure
        let message = located?.underlying.localizedDescription ?? String(describing: error)
        diagnostics = [NexaDevDiagnostic(
            file: located?.sourceFile ?? pluginFailure?.sourceFile ?? "<runtime>",
            line: located?.line ?? pluginFailure?.sourceLine ?? 1,
            column: located?.column ?? pluginFailure?.sourceColumn ?? 1,
            message: message,
            stackTrace: "\(message)\n\nCall stack:\n\(stackTrace)"
        )]
    }

    func startConnection() {
        guard connectionTask == nil else { return }
        connectionTask = Task { [weak self] in
            await self?.connect()
            self?.connectionTask = nil
        }
    }

    func connect() async {
        guard !isConnecting, let url = URL(string: serverURL) else { return }
        isConnecting = true
        defer {
            socket = nil
            isConnecting = false
        }

        var retryDelayNanoseconds: UInt64 = 50_000_000
        while !Task.isCancelled {
            let socket = URLSession.shared.webSocketTask(with: url)
            // Initial Dev IR for a real app can exceed URLSession's default
            // receive cap. This setting is debug transport only; AOT apps do
            // not buffer or interpret Dev IR modules.
            socket.maximumMessageSize = 32 * 1024 * 1024
            self.socket = socket
            socket.resume()
            do {
                let hello = try Self.encode([
                    NexaDevKeys.type: NexaDevKeys.msgHello,
                    NexaDevKeys.payload: [
                        NexaDevKeys.protocolVersion: NexaDevSchema.protocolVersion,
                        NexaDevKeys.sessionToken: sessionToken,
                        NexaDevKeys.target: NexaDevSchema.targetPlatform,
                    ],
                ])
                // URLSession queues writes until the WebSocket upgrade is complete.
                try await withTaskCancellationHandler {
                    try await socket.send(.string(hello))
                } onCancel: {
                    socket.cancel(with: .goingAway, reason: nil)
                }
                retryDelayNanoseconds = 50_000_000
                while !Task.isCancelled {
                    let message = try await withTaskCancellationHandler {
                        try await socket.receive()
                    } onCancel: {
                        socket.cancel(with: .goingAway, reason: nil)
                    }
                    guard case let .string(text) = message else { continue }
                    receive(text)
                }
            } catch {
                print("Nexa dev connection ended: \(error); retrying")
            }
            socket.cancel(with: Task.isCancelled ? .goingAway : .normalClosure, reason: nil)
            self.socket = nil
            if !Task.isCancelled {
                try? await Task.sleep(nanoseconds: retryDelayNanoseconds)
                retryDelayNanoseconds = min(retryDelayNanoseconds * 2, 1_000_000_000)
            }
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
            if let revision = currentRevision {
                frameFence.arm(revision: revision, generation: store.moduleGeneration)
                acknowledge(revision)
            }
            print("Nexa iOS DevRuntime applied module \(currentRevision ?? "unknown")")
        case NexaDevKeys.msgPatch:
            let patchStarted = DispatchTime.now().uptimeNanoseconds
            guard let patch = payload[NexaDevKeys.msgPatch] as? [String: Any],
                  let baseRevision = patch[NexaDevKeys.baseRevision] as? String,
                  let revision = patch[NexaDevKeys.revision] as? String,
                  let operations = patch[NexaDevKeys.operations] as? [[String: Any]],
                  module != nil
            else { return }
            var nextModule = store.currentModule
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
            let installResult = store.installPatchedModule(nextModule, operations: operations)
            if installResult == .fullInstallRequired {
                store.install(module: nextModule)
            }
            currentRevision = revision
            diagnostics = []
            if installResult != .functionsOnly && installResult != .screenOnly {
                module = nextModule
            }
            let patchNanos = DispatchTime.now().uptimeNanoseconds - patchStarted
            let patchWorkMs = Double(patchNanos) / 1_000_000.0
            frameFence.arm(revision: revision, generation: store.moduleGeneration)
            acknowledge(revision)
            NSLog("Nexa iOS DevRuntime applied patch %@ (patch work %.2fms / %lluµs)", revision, patchWorkMs, patchNanos / 1000)
            print("Nexa iOS DevRuntime applied patch \(revision) (patch work \(String(format: "%.2f", patchWorkMs))ms / \(patchNanos / 1000)µs)")
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
        case NexaDevKeys.msgRuntimeTrace:
            store.runtimeTraceEnabled = payload[NexaDevKeys.enabled] as? Bool ?? false
        case NexaDevKeys.msgFrameFence:
            frameFence.configure(enabled: payload[NexaDevKeys.enabled] as? Bool ?? false)
        default:
            break
        }
    }

    private func requestFullModule() {
        guard let socket else { return }
        let payload = "{\"\(NexaDevKeys.type)\":\"\(NexaDevKeys.msgRequestFullModule)\"}"
        socket.send(.string(payload)) { _ in }
    }

    private func acknowledge(_ revision: String) {
        guard let socket else { return }
        let payload = "{\"\(NexaDevKeys.type)\":\"\(NexaDevKeys.msgAcknowledge)\",\"\(NexaDevKeys.payload)\":{\"\(NexaDevKeys.revision)\":\"\(revision)\"}}"
        socket.send(.string(payload)) { _ in }
    }

    private func sendFramePresented(revision: String, frameId: UInt64) {
        guard let socket,
              let encoded = try? Self.encode([
                  NexaDevKeys.type: NexaDevKeys.msgFramePresented,
                  NexaDevKeys.payload: [
                      NexaDevKeys.revision: revision,
                      NexaDevKeys.frameId: frameId,
                  ],
              ])
        else { return }
        socket.send(.string(encoded)) { _ in }
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
        let root = NexaDevPatchTrie()
        for operation in operations {
            guard let kind = operation[NexaDevKeys.opSet] as? String ?? operation["op"] as? String,
                  let path = operation[NexaDevKeys.opPath] as? String
            else { return false }
            guard kind == NexaDevKeys.opSet || kind == NexaDevKeys.opRemove else { return false }
            let tokens = pointerSegments(path)
            guard !tokens.isEmpty else { return false }
            var node = root
            for token in tokens.dropLast() {
                if let child = node.children[token] {
                    node = child
                } else {
                    let child = NexaDevPatchTrie()
                    node.children[token] = child
                    node = child
                }
            }
            let leaf = tokens[tokens.count - 1]
            let change = node.children[leaf] ?? NexaDevPatchTrie()
            change.change = (kind, operation[NexaDevKeys.opValue])
            node.children[leaf] = change
        }

        guard let updated = applying(root, to: module) as? [String: Any] else { return false }
        module = updated
        return true
    }

    private static func pointerSegments(_ path: String) -> [String] {
        guard path.hasPrefix("/") else { return [] }
        return path.dropFirst().split(separator: "/", omittingEmptySubsequences: false)
            .map { String($0).replacingOccurrences(of: "~1", with: "/")
                .replacingOccurrences(of: "~0", with: "~") }
    }

    private static func applying(_ node: NexaDevPatchTrie, to current: Any) -> Any? {
        if let objectValue = current as? [String: Any] {
            var object = objectValue
            for (key, childNode) in node.children {
                if let change = childNode.change {
                    switch change.kind {
                    case NexaDevKeys.opSet:
                        guard let value = change.value else { return nil }
                        object[key] = value
                    case NexaDevKeys.opRemove:
                        object.removeValue(forKey: key)
                    default:
                        return nil
                    }
                } else {
                    guard let child = object[key],
                          let updated = applying(childNode, to: child)
                    else { return nil }
                    object[key] = updated
                }
            }
            return object
        }
        if let arrayValue = current as? [Any] {
            var array = arrayValue
            for (key, childNode) in node.children {
                guard let index = Int(key), array.indices.contains(index) else { return nil }
                if let change = childNode.change {
                    guard change.kind == NexaDevKeys.opSet, let value = change.value else { return nil }
                    array[index] = value
                } else {
                    guard let updated = applying(childNode, to: array[index]) else { return nil }
                    array[index] = updated
                }
            }
            return array
        }
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
