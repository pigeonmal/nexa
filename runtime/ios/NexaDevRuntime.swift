import Foundation
import CoreFoundation
import QuartzCore
import SwiftUI
import UIKit

/// Debug-only root view backed by the authenticated Nexa Dev IR stream.
@MainActor
public struct NexaDevRuntimeRoot: View {
    @StateObject private var runtime: NexaDevRuntime
    @StateObject private var performance = NexaDevPerformanceMonitor()
    @FocusState private var activeInput: String?
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.layoutDirection) private var inheritedLayoutDirection
    @Environment(\.scenePhase) private var scenePhase

    public init(serverURL: String, sessionToken: String) {
        _runtime = StateObject(
            wrappedValue: NexaDevRuntime(serverURL: serverURL, sessionToken: sessionToken)
        )
    }

    private var configuredLayoutDirection: LayoutDirection {
        guard let direction = runtime.module?["direction"] as? [String: Any],
              let style = direction["style"] as? String
        else { return inheritedLayoutDirection }
        return style == "Rtl" ? .rightToLeft : .leftToRight
    }

    public var body: some View {
        let statusBar = runtime.module?["status_bar"] as? [String: Any] ?? [:]
        let statusBarColorScheme: ColorScheme? = switch statusBar["style"] as? String {
        case "Light": .dark
        case "Dark": .light
        default: nil
        }
        Group {
            ZStack(alignment: .topTrailing) {
                VStack(spacing: 0) {
                    if !runtime.diagnostics.isEmpty {
                        Text(runtime.diagnostics.joined(separator: "\n"))
                            .font(.caption.monospaced())
                            .foregroundStyle(.white)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(10)
                            .background(.red)
                    }
                    if let module = runtime.module {
                        NexaDevNodeList(
                            nodes: module["body"] as? [Any] ?? [],
                            module: module,
                            store: runtime.store,
                            focusedField: $activeInput
                        )
                    } else {
                        ProgressView("Connecting to Nexa…")
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                if runtime.performanceOverlayEnabled {
                    NexaDevPerformanceOverlay(fps: performance.fps, frameTimeMs: performance.frameTimeMs)
                        .padding(8)
                }
            }
            .sheet(isPresented: Binding(
                get: {
                    guard let module = runtime.module,
                          let sheet = devBottomSheet(in: module["body"] as? [Any] ?? [])
                    else { return false }
                    return runtime.store.truthy(
                        runtime.store.value(sheet.state, scope: sheet.scope)
                    )
                },
                set: { isPresented in
                    guard !isPresented,
                          let module = runtime.module,
                          let sheet = devBottomSheet(in: module["body"] as? [Any] ?? [])
                    else { return }
                    runtime.store.setValue(sheet.state, value: false, scope: sheet.scope)
                }
            )) {
                if let module = runtime.module,
                   let sheet = devBottomSheet(in: module["body"] as? [Any] ?? []) {
                    NexaDevNodeList(
                        nodes: sheet.children,
                        module: module,
                        store: runtime.store,
                        focusedField: $activeInput,
                        stateScope: sheet.scope
                    )
                    .presentationDetents(sheet.partial ? [.medium, .large] : [.large])
                }
            }
        }
        .onChange(of: runtime.store.appLifecycleEpoch) { _ in
            guard let module = runtime.module else { return }
            let actions = module["on_appear"] as? [Any] ?? []
            if module["on_appear_async"] as? Bool == true {
                Task { @MainActor in
                    try? await runtime.store.performAsync(actions, scope: "app", locals: [:])
                }
            } else {
                runtime.store.perform(actions, scope: "app", locals: [:])
            }
            if scenePhase == .active {
                runtime.store.perform(module["on_active"] as? [Any] ?? [], scope: "app", locals: [:])
            }
        }
        .onChange(of: scenePhase) { phase in
            guard let module = runtime.module else { return }
            switch phase {
            case .active:
                runtime.store.perform(module["on_active"] as? [Any] ?? [], scope: "app", locals: [:])
            case .inactive:
                runtime.store.perform(module["on_inactive"] as? [Any] ?? [], scope: "app", locals: [:])
            case .background:
                runtime.store.perform(module["on_background"] as? [Any] ?? [], scope: "app", locals: [:])
            @unknown default:
                break
            }
        }
        .onDisappear {
            guard let module = runtime.module else { return }
            runtime.store.perform(module["on_disappear"] as? [Any] ?? [], scope: "app", locals: [:])
        }
        .task { runtime.connect() }
        .onChange(of: activeInput) { runtime.store.focusChanged(to: $0) }
        .onChange(of: runtime.store.focusedFieldKey) { activeInput = $0 }
        .onChange(of: runtime.performanceOverlayEnabled) { enabled in
            performance.setEnabled(enabled)
        }
        .environment(\.layoutDirection, configuredLayoutDirection)
        .modifier(NexaDevStatusBarVisibility(hidden: statusBar["hidden"] as? Bool ?? false))
        .preferredColorScheme(statusBarColorScheme)
        .background(alignment: .top) {
            if let color = devStatusBarColor(statusBar["background"], isDark: colorScheme == .dark) {
                GeometryReader { proxy in
                    color.frame(height: proxy.safeAreaInsets.top).ignoresSafeArea(edges: .top)
                }
            }
        }
    }
}

private struct NexaDevStatusBarVisibility: ViewModifier {
    let hidden: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 27.0, *) {
            content.toolbarVisibility(hidden ? .hidden : .visible, for: .statusBar)
        } else {
            content.statusBarHidden(hidden)
        }
    }
}

private struct NexaDevBottomSheet {
    let state: String
    let partial: Bool
    let children: [Any]
    let scope: String
}

private func devBottomSheet(in nodes: [Any], scope: String = "app") -> NexaDevBottomSheet? {
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

private func devStatusBarColor(_ value: Any?, isDark: Bool) -> Color? {
    guard let tagged = value as? [String: Any] else { return nil }
    let payload: [String: Any]
    if let fixed = tagged["Static"] as? [String: Any] {
        payload = fixed
    } else if let adaptive = tagged["Adaptive"] as? [String: Any],
              let selected = adaptive[isDark ? "dark" : "light"] as? [String: Any] {
        payload = selected
    } else {
        return nil
    }
    guard let red = payload["red"] as? Double,
          let green = payload["green"] as? Double,
          let blue = payload["blue"] as? Double,
          let alpha = payload["alpha"] as? Double
    else { return nil }
    return Color(.sRGB, red: red / 255, green: green / 255, blue: blue / 255, opacity: alpha / 255)
}

private func devColor(_ value: Any?, isDark: Bool) -> Color? {
    devStatusBarColor(value, isDark: isDark)
}

@MainActor
private final class NexaDevRuntime: ObservableObject {
    @Published var module: [String: Any]?
    @Published var diagnostics: [String] = []
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
                        "type": "hello",
                        "payload": [
                            "protocol_version": 3,
                            "session_token": sessionToken,
                            "target": "ios",
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
              let kind = envelope["type"] as? String,
              let payload = envelope["payload"] as? [String: Any]
        else { return }

        switch kind {
        case "full_module":
            guard let devModule = payload["module"] as? [String: Any],
                  let nextModule = devModule["module"] as? [String: Any]
            else { return }
            store.install(module: nextModule)
            currentRevision = devModule["revision"] as? String
            diagnostics = []
            module = nextModule
            if let currentRevision { acknowledge(currentRevision) }
        case "patch":
            guard let patch = payload["patch"] as? [String: Any],
                  let baseRevision = patch["base_revision"] as? String,
                  let revision = patch["revision"] as? String,
                  let operations = patch["operations"] as? [[String: Any]],
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
            store.install(module: nextModule)
            currentRevision = revision
            diagnostics = []
            module = nextModule
            acknowledge(revision)
        case "diagnostics":
            diagnostics = (payload["diagnostics"] as? [[String: Any]] ?? []).map { diagnostic in
                let file = diagnostic["file"] as? String ?? "<unknown>"
                let line = diagnostic["line"] as? Int ?? 1
                let column = diagnostic["column"] as? Int ?? 1
                let message = diagnostic["message"] as? String ?? "compile error"
                let formatted = "\(file):\(line):\(column): \(message)"
                print(formatted)
                return formatted
            }
        case "restart":
            if let module { store.hotRestart(module: module) }
            print("Nexa dev hot restart: \(payload["reason"] ?? "requested")")
        case "performance_overlay":
            performanceOverlayEnabled = payload["enabled"] as? Bool ?? false
        default:
            break
        }
    }

    private func requestFullModule() {
        guard let socket else { return }
        Task {
            try? await socket.send(.string(try Self.encode(["type": "request_full_module"])))
        }
    }

    private func acknowledge(_ revision: String) {
        guard let socket else { return }
        Task {
            try? await socket.send(.string(try Self.encode([
                "type": "acknowledge",
                "payload": ["revision": revision],
            ])))
        }
    }

    private static func apply(
        _ operations: [[String: Any]],
        to module: inout [String: Any]
    ) -> Bool {
        for operation in operations {
            guard let kind = operation["op"] as? String,
                  let path = operation["path"] as? String
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
                value: operation["value"],
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
                case "set":
                    guard let value else { valid = false; return nil }
                    object[key] = value
                case "remove": object.removeValue(forKey: key)
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
                guard kind == "set", let value else { valid = false; return nil }
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

@MainActor
private final class NexaDevPerformanceMonitor: NSObject, ObservableObject {
    @Published private(set) var fps = 0
    @Published private(set) var frameTimeMs = 0.0

    private var displayLink: CADisplayLink?
    private var frameCount = 0
    private var lastSampleTime = 0.0
    private var previousTimestamp = 0.0

    func setEnabled(_ enabled: Bool) {
        guard enabled, displayLink == nil else {
            if !enabled {
                displayLink?.invalidate()
                displayLink = nil
                fps = 0
                frameTimeMs = 0
                frameCount = 0
                lastSampleTime = 0
                previousTimestamp = 0
            }
            return
        }
        let link = CADisplayLink(target: self, selector: #selector(frame(_:)))
        link.add(to: .main, forMode: .common)
        displayLink = link
    }

    @objc private func frame(_ link: CADisplayLink) {
        let timestamp = link.timestamp
        if previousTimestamp > 0 {
            frameCount += 1
            frameTimeMs = (timestamp - previousTimestamp) * 1_000
        }
        previousTimestamp = timestamp
        if lastSampleTime == 0 {
            lastSampleTime = timestamp
            return
        }
        let elapsed = timestamp - lastSampleTime
        guard elapsed >= 1 else { return }
        fps = Int((Double(frameCount) / elapsed).rounded())
        frameTimeMs = frameCount > 0 ? elapsed * 1_000 / Double(frameCount) : 0
        frameCount = 0
        lastSampleTime = timestamp
    }
}

private struct NexaDevPerformanceOverlay: View {
    let fps: Int
    let frameTimeMs: Double

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text("Nexa Performance").font(.caption.bold())
            Text("\(fps) FPS").font(.caption.monospaced())
            Text(String(format: "Frame %.1f ms", frameTimeMs)).font(.caption.monospaced())
        }
        .foregroundStyle(.white)
        .padding(10)
        .background(.black.opacity(0.78), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
    }
}

private struct NexaDevListPositionPreference: PreferenceKey {
    static let defaultValue: [Int: CGFloat] = [:]

    static func reduce(value: inout [Int: CGFloat], nextValue: () -> [Int: CGFloat]) {
        value.merge(nextValue(), uniquingKeysWith: { _, newest in newest })
    }
}

@MainActor
private enum NexaDevFastListAxis {
    case vertical
    case horizontal
    case grid(Int)

    var scrollAxis: Axis.Set {
        if case .horizontal = self { return .horizontal }
        return .vertical
    }

    var anchor: UnitPoint {
        switch self {
        case .vertical: .top
        case .horizontal: .leading
        case .grid: .topLeading
        }
    }

    var tracksHorizontalOffset: Bool {
        if case .horizontal = self { return true }
        return false
    }
}

@MainActor
private struct NexaDevFastList: View {
    let count: Int
    let axis: NexaDevFastListAxis
    let rowHeight: Double?
    let scrollPosition: Int?
    let sectionCounts: [Int]?
    let stickyHeader: AnyView?
    let sectionHeader: ((Int) -> AnyView)?
    let row: (Int, Int, Int) -> AnyView
    let onScrollPositionChanged: (Int) -> Void
    let onScroll: ((Int) -> Void)?
    let onEndReached: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    @State private var canUpdateScrollPosition = false
    @State private var didReachEnd = false

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView(axis.scrollAxis) {
                listContent
            }
            .refreshable {
                onRefresh?()
            }
            .coordinateSpace(name: "nexa-dev-fast-list")
            .onAppear {
                if let scrollPosition, (0..<count).contains(scrollPosition) {
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                        proxy.scrollTo(scrollPosition, anchor: axis.anchor)
                    }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                        canUpdateScrollPosition = true
                    }
                } else {
                    canUpdateScrollPosition = true
                }
            }
            .onChange(of: scrollPosition) { index in
                guard let index, (0..<count).contains(index) else { return }
                canUpdateScrollPosition = false
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    proxy.scrollTo(index, anchor: axis.anchor)
                }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                    canUpdateScrollPosition = true
                }
            }
            .onPreferenceChange(NexaDevListPositionPreference.self) { offsets in
                let visibleRows = offsets.filter { $0.value >= 0 }
                guard let visibleIndex = visibleRows
                    .min(by: { abs($0.value) < abs($1.value) })?.key
                else { return }
                onScroll?(visibleIndex)
                if canUpdateScrollPosition { onScrollPositionChanged(visibleIndex) }
                if visibleIndex >= count - 10, !didReachEnd {
                    didReachEnd = true
                    onEndReached?(visibleIndex)
                } else if visibleIndex < count - 1 {
                    didReachEnd = false
                }
            }
            .onChange(of: count) { _ in didReachEnd = false }
        }
    }

    @ViewBuilder
    private var listContent: some View {
        switch axis {
        case .vertical:
            LazyVStack(alignment: .leading, spacing: 0, pinnedViews: stickyHeader != nil || sectionCounts != nil ? [.sectionHeaders] : []) {
                if let sectionCounts {
                    ForEach(sectionCounts.indices, id: \.self) { sectionIndex in
                        let start = sectionCounts.prefix(sectionIndex).reduce(0, +)
                        Section {
                            ForEach(0..<sectionCounts[sectionIndex], id: \.self) { itemIndex in
                                listItem(start + itemIndex, section: sectionIndex, item: itemIndex, horizontal: false)
                            }
                        } header: {
                            sectionHeader?(sectionIndex) ?? AnyView(EmptyView())
                        }
                    }
                } else {
                    if let stickyHeader {
                        Section { ForEach(0..<count, id: \.self) { index in listItem(index, section: 0, item: index, horizontal: false) } }
                        header: { stickyHeader }
                    } else {
                        ForEach(0..<count, id: \.self) { index in listItem(index, section: 0, item: index, horizontal: false) }
                    }
                }
            }
        case .horizontal:
            LazyHStack(alignment: .top, spacing: 0) {
                ForEach(0..<count, id: \.self) { index in
                    listItem(index, section: 0, item: index, horizontal: true)
                }
            }
        case let .grid(columns):
            LazyVGrid(
                columns: Array(repeating: GridItem(.flexible(), spacing: 0), count: max(1, columns)),
                alignment: .leading,
                spacing: 0
            ) {
                ForEach(0..<count, id: \.self) { index in
                    listItem(index, section: 0, item: index, horizontal: false)
                }
            }
        }
    }

    private func listItem(_ index: Int, section: Int, item: Int, horizontal: Bool) -> some View {
        row(index, section, item)
            .frame(minWidth: horizontal ? rowHeight.map { CGFloat($0) } : nil)
            .frame(minHeight: horizontal ? nil : rowHeight.map { CGFloat($0) })
            .frame(maxWidth: horizontal ? nil : .infinity, alignment: .leading)
            .id(index)
            .onAppear {
                guard index >= count - 1, !didReachEnd else { return }
                didReachEnd = true
                onEndReached?(index)
            }
            .background(GeometryReader { geometry in
                let frame = geometry.frame(in: .named("nexa-dev-fast-list"))
                Color.clear.preference(
                    key: NexaDevListPositionPreference.self,
                    value: [index: axis.tracksHorizontalOffset ? frame.minX : frame.minY]
                )
            })
    }
}

private struct NexaDevRoute: Hashable {
    let token: String
}

private enum NexaDevActionFlow: Equatable {
    case normal
    case `break`
    case `continue`
}

@MainActor
private final class NexaDevStateStore: ObservableObject {
    @Published private(set) var revision = 0
    @Published private(set) var appLifecycleEpoch = 0
    @Published var navigationPath: [NexaDevRoute] = []
    @Published private(set) var focusedFieldKey: String?
    private var values: [String: Any] = [:]
    private var typeSignatures: [String: String] = [:]
    private var functions: [String: [String: Any]] = [:]
    private var structs: [String: [[String: Any]]] = [:]
    private var routeArguments: [String: (screen: String, values: [String: Any], signature: String)] = [:]
    private var navigationRoot: String?
    private var activeFunctions = Set<String>()
    private var focusBindings: [String: (scope: String, state: String?)] = [:]
    private var hasInstalledModule = false

    func install(module: [String: Any]) {
        let screens = module["screens"] as? [[String: Any]] ?? []
        let appStates = module["states"] as? [[String: Any]] ?? []
        functions = Dictionary(
            (module["functions"] as? [[String: Any]] ?? []).compactMap { function in
                guard let name = function["name"] as? String else { return nil }
                return (name, function)
            },
            uniquingKeysWith: { _, newest in newest }
        )
        structs = Dictionary(
            (module["structs"] as? [[String: Any]] ?? []).compactMap { declaration in
                guard let name = declaration["name"] as? String else { return nil }
                return (name, declaration["fields"] as? [[String: Any]] ?? [])
            },
            uniquingKeysWith: { _, newest in newest }
        )
        var nextValues: [String: Any] = [:]
        var nextTypes: [String: String] = [:]
        let declarations = appStates.map { ("app", $0) } + screens.flatMap { screen in
            guard let name = screen["name"] as? String else { return [(String, [String: Any])]() }
            return (screen["states"] as? [[String: Any]] ?? []).map { ("screen/\(name)", $0) }
        }
        for (scope, state) in declarations {
            guard let name = state["name"] as? String else { continue }
            let identity = "\(scope)/state/\(name)"
            let signature = Self.canonicalJSON(state["ty"])
            nextTypes[identity] = signature
            if typeSignatures[identity] == signature, let value = values[identity] {
                nextValues[identity] = value
            } else if let initial = state["initial"] {
                nextValues[identity] = evaluate(initial, locals: [:], scope: scope)
            }
        }
        values = nextValues
        typeSignatures = nextTypes
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
                return (nextValues["\(binding.scope)/state/\(state)"] as? Bool) == true
            }?.key
        }
        let rootNode = (module["body"] as? [[String: Any]] ?? []).first { $0["NavigationStack"] != nil }
        let rootIndex = (rootNode?["NavigationStack"] as? [String: Any])?["root"] as? Int
        let nextRoot = rootIndex.flatMap { screens.indices.contains($0) ? screens[$0]["name"] as? String : nil }
        if navigationRoot != nextRoot {
            navigationPath.removeAll()
            navigationRoot = nextRoot
        }
        let screenSignatures: [String: String] = Dictionary(uniqueKeysWithValues: screens.compactMap { screen -> (String, String)? in
            guard let name = screen["name"] as? String else { return nil }
            let parameters = screen["parameters"] as? [[String: Any]] ?? []
            return (name, Self.canonicalJSON(parameters))
        })
        navigationPath.removeAll {
            guard let destination = routeArguments[$0.token] else { return true }
            return screenSignatures[destination.screen] != destination.signature
        }
        routeArguments = routeArguments.filter {
            screenSignatures[$0.value.screen] == $0.value.signature
        }
        if !hasInstalledModule {
            hasInstalledModule = true
            appLifecycleEpoch += 1
        }
        revision += 1
    }

    func hotRestart(module: [String: Any]) {
        values.removeAll()
        typeSignatures.removeAll()
        routeArguments.removeAll()
        navigationPath.removeAll()
        navigationRoot = nil
        appLifecycleEpoch += 1
        focusedFieldKey = nil
        activeFunctions.removeAll()
        install(module: module)
    }

    func value(_ name: String, scope: String = "app") -> Any {
        values["\(scope)/state/\(name)"] ?? values["app/state/\(name)"] ?? ""
    }

    func locals(scope: String, parameters: [String: Any]) -> [String: Any] {
        parameters
    }

    func setValue(_ name: String, value: Any, scope: String) {
        let scopedIdentity = "\(scope)/state/\(name)"
        let identity = typeSignatures[scopedIdentity] != nil || scope == "app"
            ? scopedIdentity
            : "app/state/\(name)"
        values[identity] = value
        revision += 1
    }

    func focusChanged(to nextIdentity: String?) {
        guard focusedFieldKey != nextIdentity else { return }
        let previousIdentity = focusedFieldKey
        focusedFieldKey = nextIdentity
        for identity in Set([previousIdentity, nextIdentity].compactMap { $0 }) {
            guard let binding = focusBindings[identity], let state = binding.state else { continue }
            setValue(state, value: identity == nextIdentity, scope: binding.scope)
        }
    }

    private static func collectFocusBindings(
        _ value: Any,
        scope: String,
        into bindings: inout [String: (scope: String, state: String?)]
    ) {
        if let nodes = value as? [Any] {
            for node in nodes { collectFocusBindings(node, scope: scope, into: &bindings) }
        } else if let node = value as? [String: Any] {
            if let fields = node["TextInput"] as? [String: Any],
               let state = fields["state"] as? String {
                bindings["\(scope)/input/\(state)"] = (
                    scope,
                    fields["focused"] as? String
                )
            }
            for child in node.values {
                collectFocusBindings(child, scope: scope, into: &bindings)
            }
        }
    }

    func perform(_ actions: [Any], scope: String, locals: [String: Any]) {
        _ = performActions(actions, scope: scope, locals: locals)
    }

    @discardableResult
    private func performActions(_ actions: [Any], scope: String, locals: [String: Any]) -> NexaDevActionFlow {
        for action in actions {
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
            } else if let mutation = tagged["CollectionMutation"] as? [String: Any],
                      let name = mutation["name"] as? String {
                performCollectionMutation(mutation, name: name, scope: scope, locals: locals)
            } else if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                let flow = performActions(tryCatch["body"] as? [Any] ?? [], scope: scope, locals: locals)
                if flow != .normal { return flow }
            } else if tagged["Break"] != nil {
                return .break
            } else if tagged["Continue"] != nil {
                return .continue
            }
        }
        return .normal
    }

    private func performCollectionMutation(
        _ mutation: [String: Any],
        name: String,
        scope: String,
        locals: [String: Any]
    ) {
        let arguments = (mutation["arguments"] as? [Any] ?? []).map { evaluate($0, locals: locals, scope: scope) }
        switch mutation["operation"] as? String {
        case "ArrayAppend":
            var array = value(name, scope: scope) as? [Any] ?? []
            if let item = arguments.first { array.append(item) }
            setValue(name, value: array, scope: scope)
        case "ArrayRemoveAt":
            var array = value(name, scope: scope) as? [Any] ?? []
            if let index = (arguments.first as? NSNumber)?.intValue, array.indices.contains(index) { array.remove(at: index) }
            setValue(name, value: array, scope: scope)
        case "SetInsert", "SetRemove":
            var set = value(name, scope: scope) as? Set<String> ?? []
            if let item = arguments.first.map(stringify) {
                if mutation["operation"] as? String == "SetInsert" { set.insert(item) } else { set.remove(item) }
            }
            setValue(name, value: set, scope: scope)
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

    func performAsync(_ actions: [Any], scope: String, locals: [String: Any]) async throws {
        for action in actions {
            guard let tagged = action as? [String: Any] else { continue }
            if let tryCatch = tagged["TryCatch"] as? [String: Any] {
                do {
                    try await performAsync(
                        tryCatch["body"] as? [Any] ?? [],
                        scope: scope,
                        locals: locals
                    )
                } catch is CancellationError {
                    throw CancellationError()
                } catch {
                    NSLog("NexaDevRuntime async dev action failed: %@", String(describing: error))
                    if let catchBody = tryCatch["catch_body"] as? [Any] {
                        try await performAsync(catchBody, scope: scope, locals: locals)
                    } else {
                        throw error
                    }
                }
            } else if let assignment = tagged["Assign"] as? [String: Any],
               let name = assignment["name"] as? String,
               let expression = assignment["value"] {
                let value = try await evaluateAsync(expression, locals: locals, scope: scope)
                setValue(name, value: value, scope: scope)
                revision += 1
            } else if let branch = tagged["If"] as? [String: Any],
                      let condition = branch["condition"] {
                let value = try await evaluateAsync(condition, locals: locals, scope: scope)
                let selected = truthy(value)
                    ? branch["then_branch"] as? [Any]
                    : branch["else_branch"] as? [Any]
                try await performAsync(selected ?? [], scope: scope, locals: locals)
            } else if let expression = tagged["Expression"] {
                _ = try await evaluateAsync(expression, locals: locals, scope: scope)
            }
        }
    }

    private func evaluateAsync(
        _ expression: Any,
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
        guard let tagged = expression as? [String: Any], let (kind, payload) = tagged.first else {
            return NSNull()
        }
        switch kind {
        case "Await", "TryAwait":
            return try await evaluateAsync(payload, locals: locals, scope: scope)
        case "Call":
            guard let call = payload as? [String: Any] else { return NSNull() }
            return try await invokeFunctionAsync(call, locals: locals, scope: scope)
        case "NativeCall":
            guard let call = payload as? [String: Any] else { return NSNull() }
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
                ["useCache", request["use_cache"] ?? NSNull()],
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
        case "FileExists", "FileReadText", "FileWriteText", "FileDelete":
            guard let file = payload as? [String: Any] else { return NSNull() }
            let name: String
            switch kind {
            case "FileExists": name = "exists"
            case "FileReadText": name = "readText"
            case "FileWriteText": name = "writeText"
            default: name = "delete"
            }
            var fileArguments: [[Any]] = [["path", file["path"] ?? NSNull()]]
            if kind == "FileWriteText" {
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
            return (value as? [String: Any])?[name] ?? NSNull()
        case "Array", "Set":
            let entries = payload as? [Any] ?? []
            var values: [Any] = []
            for entry in entries {
                values.append(try await evaluateAsync(entry, locals: locals, scope: scope))
            }
            if kind == "Set" { return Set(values.compactMap { $0 as? String }) }
            return values
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
        case "Not":
            return !truthy(try await evaluateAsync(payload, locals: locals, scope: scope))
        case "Binary":
            guard let binary = payload as? [String: Any],
                  let op = binary["op"] as? String,
                  let leftExpression = binary["left"],
                  let rightExpression = binary["right"]
            else { return false }
            let left = try await evaluateAsync(leftExpression, locals: locals, scope: scope)
            let right = try await evaluateAsync(rightExpression, locals: locals, scope: scope)
            return compare(op, left, right)
        default:
            return evaluate(expression, locals: locals, scope: scope)
        }
    }

    private func invokeFunctionAsync(
        _ call: [String: Any],
        locals: [String: Any],
        scope: String
    ) async throws -> Any {
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
        guard let body = function["body"] else { return NSNull() }
        return try await evaluateAsync(body, locals: functionScope, scope: scope)
    }

    private func invokeNativeAsync(
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
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported native call \\(namespace).\\(name)"]
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
                    userInfo: [NSLocalizedDescriptionKey: "Unsupported async native call \\(namespace).\\(name)"]
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
                userInfo: [NSLocalizedDescriptionKey: "Unsupported async native call \\(namespace).\\(name)"]
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

    func navigationRoute(screen: String, values: [String: Any], signature: String) -> NexaDevRoute {
        let token = "\(screen):\(Self.canonicalJSON(values))"
        routeArguments[token] = (screen, values, signature)
        return NexaDevRoute(token: token)
    }

    func routeDestination(_ route: NexaDevRoute) -> (screen: String, values: [String: Any])? {
        guard let destination = routeArguments[route.token] else { return nil }
        return (destination.screen, destination.values)
    }

    func evaluate(_ expression: Any, locals: [String: Any], scope: String = "app") -> Any {
        if let unit = expression as? String {
            switch unit {
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
        case "String": return payload as? String ?? ""
        case "Bool": return payload as? Bool ?? false
        case "Array":
            return (payload as? [Any] ?? []).map { evaluate($0, locals: locals, scope: scope) }
        case "Set":
            return Set((payload as? [Any] ?? []).map { stringify(evaluate($0, locals: locals, scope: scope)) })
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
            let number = (payload as? [String: Any])?["raw"] as? String ?? "0"
            return number.contains(".") ? (Double(number) ?? 0) as Any : (Int64(number) ?? 0) as Any
        case "EnumValue": return (payload as? [String: Any])?["case_name"] as? String ?? ""
        case "State":
            let parts = payload as? [Any] ?? []
            guard let name = parts.first as? String else { return NSNull() }
            return locals[name] ?? value(name, scope: scope)
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
        case "Contains":
            let fields = payload as? [String: Any] ?? [:]
            guard let value = fields["value"], let collection = fields["collection"] else { return false }
            return contains(evaluate(value, locals: locals, scope: scope), in: evaluate(collection, locals: locals, scope: scope))
        case "CollectionTransform":
            let fields = payload as? [String: Any] ?? [:]
            guard let collectionExpression = fields["collection"],
                  let collection = evaluate(collectionExpression, locals: locals, scope: scope) as? [Any],
                  let closure = fields["closure"] as? [String: Any],
                  let closurePayload = closure["Closure"] as? [String: Any]
            else { return [] }
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
            case "Filter": return collection.filter { truthy(apply($0)) }
            case "Reduce":
                var accumulator = fields["initial"].map { evaluate($0, locals: locals, scope: scope) } ?? 0
                for item in collection { accumulator = apply(item, accumulator) }
                return accumulator
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
            if call["is_constructor"] as? Bool == true, let fields = structs[name] {
                let arguments = call["arguments"] as? [Any] ?? []
                var instance: [String: Any] = [:]
                for (field, argument) in zip(fields, arguments) {
                    guard let fieldName = field["name"] as? String else { continue }
                    instance[fieldName] = evaluate(argument, locals: locals, scope: scope)
                }
                return instance
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
            guard let body = function["body"] else { return NSNull() }
            return evaluate(body, locals: functionScope, scope: scope)
        case "NativeCall":
            guard let call = payload as? [String: Any] else { return NSNull() }
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

    private func invokeNativeSync(
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

    private func compare(_ op: String, _ lhs: Any, _ rhs: Any) -> Bool {
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

    private func contains(_ value: Any, in collection: Any) -> Bool {
        if let values = collection as? [Any] { return values.contains { stringify($0) == stringify(value) } }
        if let values = collection as? Set<String> { return values.contains(stringify(value)) }
        if let values = collection as? [String: Any] { return values[stringify(value)] != nil }
        if let text = collection as? String, let needle = value as? String { return text.contains(needle) }
        return false
    }

    private func number(_ value: Any) -> Double {
        if let number = value as? NSNumber { return number.doubleValue }
        if let text = value as? String { return Double(text) ?? 0 }
        return 0
    }

    fileprivate func truthy(_ value: Any) -> Bool {
        if let number = value as? NSNumber { return number.boolValue }
        if let text = value as? String { return !text.isEmpty }
        return false
    }

    fileprivate func stringify(_ value: Any) -> String {
        if value is NSNull { return "null" }
        if let number = value as? NSNumber {
            if CFGetTypeID(number) == CFBooleanGetTypeID() { return number.boolValue ? "true" : "false" }
            let double = number.doubleValue
            return double.rounded() == double ? String(Int64(double)) : String(double)
        }
        return String(describing: value)
    }

    fileprivate static func canonicalSignature(_ parameters: [[String: Any]]) -> String {
        canonicalJSON(parameters.map { $0["ty"] ?? NSNull() })
    }

    private static func canonicalJSON(_ value: Any?) -> String {
        guard let value, JSONSerialization.isValidJSONObject([value]),
              let data = try? JSONSerialization.data(withJSONObject: [value], options: [.sortedKeys])
        else { return "null" }
        return String(data: data, encoding: .utf8) ?? "null"
    }
}

private struct NexaDevContentSlot: @unchecked Sendable {
    let nodes: [Any]
    let scope: String
    let parameters: [String: Any]
}

private struct NexaDevContentSlotKey: EnvironmentKey {
    static let defaultValue: NexaDevContentSlot? = nil
}

private extension EnvironmentValues {
    var nexaDevContentSlot: NexaDevContentSlot? {
        get { self[NexaDevContentSlotKey.self] }
        set { self[NexaDevContentSlotKey.self] = newValue }
    }
}

@MainActor
private struct NexaDevNodeList: View {
    let nodes: [Any]
    let module: [String: Any]
    @ObservedObject var store: NexaDevStateStore
    var focusedField: FocusState<String?>.Binding
    var parameters: [String: Any] = [:]
    var stateScope: String = "app"
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.nexaDevContentSlot) private var contentSlot

    var body: some View {
        let _ = store.revision
        let locals = store.locals(scope: stateScope, parameters: parameters)
        VStack(spacing: 0) {
            ForEach(nodes.indices, id: \.self) { index in
                renderNode(nodes[index], locals: locals, scope: stateScope)
            }
        }
    }

    private func renderNode(_ rawNode: Any, locals: [String: Any], scope: String) -> AnyView {
        let kind: String
        let fields: [String: Any]
        if let unitVariant = rawNode as? String {
            kind = unitVariant
            fields = [:]
        } else if let tagged = rawNode as? [String: Any], let (tag, payload) = tagged.first {
            kind = tag
            fields = payload as? [String: Any] ?? [:]
        } else {
            return AnyView(EmptyView())
        }
        switch kind {
        case "Layout":
            let children = fields["children"] as? [Any] ?? []
            let spacing = fields["spacing"] as? Double ?? 0
            let style = fields["style"] as? [String: Any] ?? [:]
            let alignment = style["alignment"] as? String
            let rowAlignment: VerticalAlignment = switch alignment {
            case "Start": .top
            case "End": .bottom
            default: .center
            }
            let stackAlignment: Alignment = switch alignment {
            case "Start": .topLeading
            case "End": .bottomTrailing
            default: .center
            }
            let columnAlignment: HorizontalAlignment = switch alignment {
            case "Start": .leading
            case "End": .trailing
            default: .center
            }
            let container: AnyView
            switch fields["kind"] as? String {
            case "Row":
                container = AnyView(HStack(alignment: rowAlignment, spacing: spacing) { ForEach(children.indices, id: \.self) { renderNode(children[$0], locals: locals, scope: scope) } })
            case "Stack":
                container = AnyView(ZStack(alignment: stackAlignment) { ForEach(children.indices, id: \.self) { renderNode(children[$0], locals: locals, scope: scope) } })
            default:
                container = AnyView(VStack(alignment: columnAlignment, spacing: spacing) { ForEach(children.indices, id: \.self) { renderNode(children[$0], locals: locals, scope: scope) } })
            }
            var styled = container
            if let padding = style["padding"] as? Double { styled = AnyView(styled.padding(padding)) }
            let width = style["width"] as? Double
            let height = style["height"] as? Double
            let minWidth = style["min_width"] as? Double
            let maxWidth = style["max_width"] as? Double
            let minHeight = style["min_height"] as? Double
            let maxHeight = style["max_height"] as? Double
            if width != nil || height != nil || minWidth != nil || maxWidth != nil || minHeight != nil || maxHeight != nil {
                styled = AnyView(styled.frame(
                    minWidth: minWidth.map { CGFloat($0) },
                    idealWidth: width.map { CGFloat($0) },
                    maxWidth: maxWidth.map { CGFloat($0) },
                    minHeight: minHeight.map { CGFloat($0) },
                    idealHeight: height.map { CGFloat($0) },
                    maxHeight: maxHeight.map { CGFloat($0) }
                ))
            }
            if let background = devColor(style["background"], isDark: colorScheme == .dark) {
                styled = AnyView(styled.background(background))
            }
            if let radius = style["corner_radius"] as? Double {
                styled = AnyView(styled.clipShape(RoundedRectangle(cornerRadius: radius)))
            }
            if let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                let width = style["border_width"] as? Double ?? 1
                styled = AnyView(styled.overlay(RoundedRectangle(cornerRadius: style["corner_radius"] as? Double ?? 0).stroke(border, lineWidth: width)))
            }
            if let opacity = style["opacity"] as? Double { styled = AnyView(styled.opacity(opacity)) }
            if let animation = style["animation"] as? String {
                let value: Animation
                switch animation {
                case "Spring": value = .spring()
                case "EaseIn": value = .easeIn
                case "EaseOut": value = .easeOut
                case "EaseInOut": value = .easeInOut
                default: value = .linear
                }
                styled = AnyView(styled.animation(value, value: store.revision))
            }
            return styled
        case "Text":
            let text = store.stringify(store.evaluate(fields["value"] ?? "", locals: locals, scope: scope))
            let style = fields["style"] as? [String: Any] ?? [:]
            var styled = AnyView(Text(text))
            if let size = style["font_size"] as? Double {
                styled = AnyView(styled.font(.system(size: size)))
            }
            let weight: Font.Weight? = switch style["font_weight"] as? String {
            case "Normal": .regular
            case "Medium": .medium
            case "Semibold": .semibold
            case "Bold": .bold
            default: nil
            }
            if let weight { styled = AnyView(styled.fontWeight(weight)) }
            if let color = devColor(style["color"], isDark: colorScheme == .dark) { styled = AnyView(styled.foregroundStyle(color)) }
            if let lineLimit = style["line_limit"] as? Int { styled = AnyView(styled.lineLimit(lineLimit)) }
            if let lineHeight = style["line_height"] as? Double { styled = AnyView(styled.lineSpacing(max(0, lineHeight - (style["font_size"] as? Double ?? lineHeight)))) }
            if let letterSpacing = style["letter_spacing"] as? Double { styled = AnyView(styled.tracking(letterSpacing)) }
            if style["selectable"] as? Bool == true { styled = AnyView(styled.textSelection(.enabled)) }
            return styled
        case "Content":
            guard let contentSlot else { return AnyView(EmptyView()) }
            return AnyView(NexaDevNodeList(
                nodes: contentSlot.nodes,
                module: module,
                store: store,
                focusedField: focusedField,
                parameters: contentSlot.parameters,
                stateScope: contentSlot.scope
            ))
        case "ComponentCall":
            let name = fields["name"] as? String ?? ""
            guard let component = (module["components"] as? [[String: Any]])?.first(where: {
                $0["name"] as? String == name
            }) else { return AnyView(EmptyView()) }
            var componentParameters: [String: Any] = [:]
            for argument in fields["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
                guard let parameter = argument[0] as? String else { continue }
                componentParameters[parameter] = store.evaluate(argument[1], locals: locals, scope: scope)
            }
            let slot = NexaDevContentSlot(
                nodes: fields["children"] as? [Any] ?? [],
                scope: scope,
                parameters: locals
            )
            return AnyView(NexaDevNodeList(
                nodes: component["body"] as? [Any] ?? [],
                module: module,
                store: store,
                focusedField: focusedField,
                parameters: componentParameters,
                stateScope: "component/\(name)"
            ).environment(\.nexaDevContentSlot, slot))
        case "Button":
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let actions = fields["actions"] as? [Any] ?? []
            let disabled = fields["disabled"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            return AnyView(Button(label) { store.perform(actions, scope: scope, locals: locals) }.disabled(disabled))
        case "Pressable":
            let children = fields["children"] as? [Any] ?? []
            let actions = fields["actions"] as? [Any] ?? []
            let longPressActions = fields["long_press_actions"] as? [Any] ?? []
            let disabled = fields["disabled"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let haptic = fields["haptic"] as? String
            let content = NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            let button = Button {
                playNexaHaptic(haptic)
                store.perform(actions, scope: scope, locals: locals)
            } label: {
                content
            }
            .buttonStyle(.plain)
            .disabled(disabled)
            if longPressActions.isEmpty {
                return AnyView(button)
            }
            return AnyView(button.simultaneousGesture(LongPressGesture().onEnded { _ in
                store.perform(longPressActions, scope: scope, locals: locals)
            }))
        case "TextInput":
            let name = fields["state"] as? String ?? ""
            let placeholder = fields["placeholder"] as? String ?? ""
            let identity = "\(scope)/input/\(name)"
            let maxLength = fields["max_length"] as? Int
            let binding = Binding(
                get: { store.stringify(store.value(name, scope: scope)) },
                set: { nextValue in
                    let value = maxLength.map { String(nextValue.prefix(max(0, $0))) } ?? nextValue
                    store.setValue(name, value: value, scope: scope)
                }
            )
            var input = AnyView(TextField(placeholder, text: binding, axis: fields["multiline"] as? Bool == true ? .vertical : .horizontal))
            switch fields["keyboard"] as? String {
            case "Number": input = AnyView(input.keyboardType(.numberPad))
            case "Email": input = AnyView(input.keyboardType(.emailAddress))
            case "Phone": input = AnyView(input.keyboardType(.phonePad))
            case "Url": input = AnyView(input.keyboardType(.URL))
            default: input = AnyView(input.keyboardType(.default))
            }
            let capitalization: TextInputAutocapitalization = switch fields["capitalization"] as? String {
            case "None": .never
            case "Words": .words
            case "Characters": .characters
            default: .sentences
            }
            input = AnyView(input.textInputAutocapitalization(capitalization))
            if fields["autocorrect"] as? Bool == false || fields["capitalization"] as? String == "None" {
                input = AnyView(input.autocorrectionDisabled())
            } else if fields["autocorrect"] as? Bool == true {
                input = AnyView(input.autocorrectionDisabled(false))
            }
            input = AnyView(input.focused(focusedField, equals: identity))
            if fields["secure"] as? Bool == true {
                return AnyView(SecureField(placeholder, text: binding).focused(focusedField, equals: identity))
            }
            return input
        case "FastList":
            guard let source = fields["source"] as? [String: Any] else {
                return AnyView(Text("FastList source could not be evaluated."))
            }
            let countExpression = source["Count"]
            let sourceItems: [Any]?
            if let itemSource = source["Items"] as? [String: Any],
               let itemExpression = itemSource["collection"] {
                sourceItems = store.evaluate(itemExpression, locals: locals, scope: scope) as? [Any] ?? []
            } else {
                sourceItems = nil
            }
            let sourceSections: [[Any]]? = (source["Sections"] as? [String: Any])
                .flatMap { $0["collection"] }
                .flatMap { store.evaluate($0, locals: locals, scope: scope) as? [[Any]] }
            guard countExpression != nil || sourceItems != nil || sourceSections != nil else {
                return AnyView(Text("FastList source could not be evaluated."))
            }
            let itemCount = countExpression.map {
                max(0, (store.evaluate($0, locals: locals, scope: scope) as? NSNumber)?.intValue ?? 0)
            } ?? (sourceItems?.count ?? 0)
            let count = sourceSections?.reduce(0) { $0 + $1.count } ?? itemCount
            let axis: NexaDevFastListAxis
            if fields["axis"] as? String == "Horizontal" {
                axis = .horizontal
            } else if let grid = (fields["axis"] as? [String: Any])?["Grid"] as? [String: Any],
                      let columns = grid["columns"] as? Int {
                axis = .grid(max(1, columns))
            } else {
                axis = .vertical
            }
            let indexName = fields["index"] as? String ?? "index"
            let itemName = fields["item"] as? String
            let sectionName = fields["section"] as? String ?? "section"
            let children = fields["children"] as? [Any] ?? []
            let itemExtent = fields["item_extent"] as? Double
            let scrollPositionName = fields["scroll_position"] as? String
            let requestedIndex = scrollPositionName.flatMap { name in
                (store.value(name, scope: scope) as? NSNumber)?.intValue
            }
            let stickyHeaderNodes = fields["sticky_header"] as? [Any]
            let stickyHeader = stickyHeaderNodes.map { nodes in
                AnyView(NexaDevNodeList(nodes: nodes, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            }
            let sectionHeaderNodes = fields["section_header"] as? [Any]
            let sectionHeader: ((Int) -> AnyView)? = sectionHeaderNodes.map { nodes in
                { sectionIndex in
                    AnyView(NexaDevNodeList(
                        nodes: nodes,
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: locals.merging([sectionName: sectionIndex]) { _, newest in newest },
                        stateScope: scope
                    ))
                }
            }
            let onScrollActions = fields["on_scroll"] as? [Any]
            let onEndReachedActions = fields["on_end_reached"] as? [Any]
            let refresh = fields["refresh"] as? [String: Any]
            let refreshState = refresh?["state"] as? String
            let refreshActions = refresh?["actions"] as? [Any] ?? []
            return AnyView(NexaDevFastList(
                count: count,
                axis: axis,
                rowHeight: itemExtent,
                scrollPosition: requestedIndex,
                sectionCounts: sourceSections?.map(\.count),
                stickyHeader: stickyHeader,
                sectionHeader: sectionHeader,
                row: { index, sectionIndex, itemIndex in
                    var rowLocals = locals.merging([indexName: itemIndex]) { _, newest in newest }
                    if let sourceSections, let itemName,
                       sourceSections.indices.contains(sectionIndex),
                       sourceSections[sectionIndex].indices.contains(itemIndex) {
                        rowLocals[itemName] = sourceSections[sectionIndex][itemIndex]
                        rowLocals[sectionName] = sectionIndex
                    } else if let itemName, let sourceItems, sourceItems.indices.contains(index) {
                        rowLocals[itemName] = sourceItems[index]
                    }
                    return AnyView(NexaDevNodeList(
                        nodes: children,
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: rowLocals,
                        stateScope: scope
                    ))
                },
                onScrollPositionChanged: { index in
                    guard let scrollPositionName,
                          (store.value(scrollPositionName, scope: scope) as? NSNumber)?.intValue != index
                    else { return }
                    store.setValue(scrollPositionName, value: index, scope: scope)
                },
                onScroll: onScrollActions.map { actions in
                    { index in store.perform(actions, scope: scope, locals: locals.merging([indexName: index]) { _, newest in newest }) }
                },
                onEndReached: onEndReachedActions.map { actions in
                    { index in store.perform(actions, scope: scope, locals: locals.merging([indexName: index]) { _, newest in newest }) }
                },
                isRefreshing: refreshState.map { store.truthy(store.value($0, scope: scope)) } ?? false,
                onRefresh: refresh.map { _ in
                    { store.perform(refreshActions, scope: scope, locals: locals) }
                }
            ))
        case "Switch":
            let name = fields["state"] as? String ?? ""
            let label = fields["label"] as? String ?? ""
            return AnyView(Toggle(label, isOn: Binding(
                get: { store.truthy(store.value(name, scope: scope)) },
                set: { store.setValue(name, value: $0, scope: scope) }
            )))
        case "Image":
            let source = fields["source"] as? [String: Any] ?? [:]
            let description = fields["description"] as? String ?? ""
            let mode: ContentMode = fields["scale"] as? String == "Fill" ? .fill : .fit
            let placeholder = fields["placeholder"] as? String
            let rendered: AnyView
            if let asset = source["Asset"] as? String {
                rendered = AnyView(Image(asset).resizable().aspectRatio(contentMode: mode))
            } else if let expression = source["RemoteUrl"] {
                let urlText = store.stringify(store.evaluate(expression, locals: locals, scope: scope))
                rendered = AnyView(AsyncImage(url: URL(string: urlText)) { phase in
                    if let image = phase.image {
                        image.resizable().aspectRatio(contentMode: mode)
                    } else if let placeholder {
                        Image(placeholder).resizable().aspectRatio(contentMode: mode)
                    } else if phase.error != nil {
                        Image(systemName: "photo")
                    } else {
                        ProgressView()
                    }
                })
            } else {
                if let placeholder {
                    rendered = AnyView(Image(placeholder).resizable().aspectRatio(contentMode: mode))
                } else {
                    rendered = AnyView(Image(systemName: "photo"))
                }
            }
            return AnyView(rendered.accessibilityLabel(description).accessibilityHidden(description.isEmpty))
        case "RefreshControl":
            let state = fields["state"] as? String ?? ""
            let actions = fields["actions"] as? [Any] ?? []
            let children = fields["children"] as? [Any] ?? []
            return AnyView(ScrollView(.vertical) {
                NexaDevNodeList(
                    nodes: children,
                    module: module,
                    store: store,
                    focusedField: focusedField,
                    parameters: locals,
                    stateScope: scope
                )
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .refreshable {
                store.perform(actions, scope: scope, locals: locals)
                while store.truthy(store.value(state, scope: scope)) {
                    try? await Task.sleep(nanoseconds: 100_000_000)
                }
            })
        case "If":
            let condition = fields["condition"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let children = condition
                ? fields["then_body"] as? [Any] ?? []
                : fields["else_body"] as? [Any] ?? []
            return AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
        case "When":
            let value = store.stringify(store.evaluate(fields["value"] ?? NSNull(), locals: locals, scope: scope))
            let cases = fields["cases"] as? [[String: Any]] ?? []
            let matchingCase = cases.first { item in
                store.stringify(store.evaluate(item["value"] ?? NSNull(), locals: locals, scope: scope)) == value
            }
            let children = matchingCase?["body"] as? [Any] ?? fields["else_body"] as? [Any] ?? []
            return AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
        case "Link":
            let urlText = store.stringify(store.evaluate(fields["url"] ?? "", locals: locals, scope: scope))
            guard let destination = URL(string: urlText) else {
                return AnyView(NexaDevNodeList(nodes: fields["children"] as? [Any] ?? [], module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            }
            let children = fields["children"] as? [Any] ?? []
            return AnyView(Link(destination: destination) {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            })
        case "Accessibility":
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let hint = fields["hint"].flatMap { $0 is NSNull ? nil : store.stringify(store.evaluate($0, locals: locals, scope: scope)) }
            let children = fields["children"] as? [Any] ?? []
            let content = NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            let role = fields["role"] as? String
            switch role {
            case "Button": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isButton))
            case "Link": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isLink))
            case "Header": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isHeader))
            case "Image": return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? "").accessibilityAddTraits(.isImage))
            default: return AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? ""))
            }
        case "KeyboardAware":
            let children = fields["children"] as? [Any] ?? []
            let dismissMode = fields["dismiss"] as? String == "Interactive" ? ScrollDismissesKeyboardMode.interactively : .never
            return AnyView(ScrollView(.vertical) {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            }.scrollDismissesKeyboard(dismissMode))
        case "AppBottomBar":
            let state = fields["state"] as? String ?? ""
            let tabs = fields["tabs"] as? [[String: Any]] ?? []
            return AnyView(TabView(selection: Binding(
                get: { (store.value(state, scope: scope) as? NSNumber)?.intValue ?? 0 },
                set: { store.setValue(state, value: $0, scope: scope) }
            )) {
                ForEach(tabs.indices, id: \.self) { index in
                    let tab = tabs[index]
                    NexaDevNodeList(
                        nodes: tab["children"] as? [Any] ?? [],
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: locals,
                        stateScope: scope
                    )
                    .tabItem {
                        if let icon = tab["icon"] as? String, !icon.isEmpty {
                            Image(systemName: icon)
                        }
                        Text(tab["label"] as? String ?? "")
                    }
                    .tag(tab["index"] as? Int ?? index)
                    .badge(tab["badge"] as? String ?? "")
                }
            })
        case "BottomSheet":
            return AnyView(EmptyView())
        case "NavigationStack":
            return navigationStack(fields, locals: locals, scope: scope)
        case "NavigationLink":
            let screenIndex = fields["destination"] as? Int ?? -1
            let screens = module["screens"] as? [[String: Any]] ?? []
            guard screens.indices.contains(screenIndex),
                  let screenName = screens[screenIndex]["name"] as? String
            else { return AnyView(EmptyView()) }
            let children = fields["children"] as? [Any] ?? []
            let guardExpression = fields["guard"]
            let enabled = guardExpression.map { expression in
                expression is NSNull || store.truthy(store.evaluate(expression, locals: locals, scope: scope))
            } ?? true
            guard enabled else {
                return AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            }
            let arguments = fields["arguments"] as? [Any] ?? []
            let parameters = screens[screenIndex]["parameters"] as? [[String: Any]] ?? []
            let argumentValues: [String: Any] = Dictionary(uniqueKeysWithValues: zip(parameters, arguments).compactMap { parameter, expression -> (String, Any)? in
                guard let name = parameter["name"] as? String else { return nil }
                return (name, store.evaluate(expression, locals: locals, scope: scope))
            })
            let signature = NexaDevStateStore.canonicalSignature(parameters)
            let route = store.navigationRoute(screen: screenName, values: argumentValues, signature: signature)
            return AnyView(NavigationLink(value: route) {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            })
        case "NavigationBack":
            let label = store.stringify(store.evaluate(fields["label"] ?? "Back", locals: locals, scope: scope))
            return AnyView(Button(label) {
                if !store.navigationPath.isEmpty { store.navigationPath.removeLast() }
            })
        default:
            return AnyView(EmptyView())
        }
    }

    private func navigationStack(
        _ fields: [String: Any],
        locals: [String: Any],
        scope: String
    ) -> AnyView {
        let screens = module["screens"] as? [[String: Any]] ?? []
        let rootIndex = fields["root"] as? Int ?? -1
        guard screens.indices.contains(rootIndex) else { return AnyView(EmptyView()) }
        let rootScreen = screens[rootIndex]
        let rootArguments = fields["arguments"] as? [Any] ?? []
        let rootValues = argumentValues(rootScreen, expressions: rootArguments, locals: locals, scope: scope)
        let content = renderScreen(rootScreen, parameters: rootValues)
        return AnyView(NavigationStack(path: $store.navigationPath) {
            content.navigationDestination(for: NexaDevRoute.self) { route in
                guard let destination = store.routeDestination(route),
                      let screen = screens.first(where: { $0["name"] as? String == destination.screen })
                else { return AnyView(EmptyView()) }
                return renderScreen(screen, parameters: destination.values)
            }
        })
    }

    private func argumentValues(
        _ screen: [String: Any],
        expressions: [Any],
        locals: [String: Any],
        scope: String
    ) -> [String: Any] {
        let parameters = screen["parameters"] as? [[String: Any]] ?? []
        return Dictionary(uniqueKeysWithValues: zip(parameters, expressions).compactMap { parameter, expression in
            guard let name = parameter["name"] as? String else { return nil }
            return (name, store.evaluate(expression, locals: locals, scope: scope))
        })
    }

    private func renderScreen(_ screen: [String: Any], parameters: [String: Any]) -> AnyView {
        let name = screen["name"] as? String ?? ""
        let scope = "screen/\(name)"
        return AnyView(NexaDevNodeList(
            nodes: screen["body"] as? [Any] ?? [],
            module: module,
            store: store,
            focusedField: focusedField,
            parameters: parameters,
            stateScope: scope
        )
        .onAppear {
            let actions = screen["on_appear"] as? [Any] ?? []
            if screen["on_appear_async"] as? Bool == true {
                Task { @MainActor in
                    try? await store.performAsync(actions, scope: scope, locals: parameters)
                }
            } else {
                store.perform(actions, scope: scope, locals: parameters)
            }
        }
        .onDisappear {
            store.perform(screen["on_disappear"] as? [Any] ?? [], scope: scope, locals: parameters)
        })
    }
}

@MainActor
private func playNexaHaptic(_ style: String?) {
    let impactStyle: UIImpactFeedbackGenerator.FeedbackStyle
    switch style {
    case "Light": impactStyle = .light
    case "Medium": impactStyle = .medium
    case "Heavy": impactStyle = .heavy
    default: return
    }
    let generator = UIImpactFeedbackGenerator(style: impactStyle)
    generator.prepare()
    generator.impactOccurred()
}
