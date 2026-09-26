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
