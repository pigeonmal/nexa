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
    @Namespace private var nexaSharedNamespace
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
                ZStack(alignment: .topLeading) {
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
                if !runtime.diagnostics.isEmpty {
                    NexaDevErrorOverlay(
                        diagnostics: runtime.diagnostics,
                        onOpenInEditor: runtime.openInEditor
                    )
                    .padding(.horizontal, 12)
                    .padding(.top, 8)
                }
                if runtime.performanceOverlayEnabled {
                    NexaDevPerformanceOverlay(fps: performance.fps, frameTimeMs: performance.frameTimeMs)
                        .padding(8)
                }
            }
        }
        .onChange(of: runtime.store.appLifecycleEpoch) { _ in
            guard let module = runtime.module else { return }
            let actions = module["on_appear"] as? [Any] ?? []
            if module["on_appear_async"] as? Bool == true {
                Task { @MainActor in
                    do {
                        _ = try await runtime.store.performAsync(actions, scope: "app", locals: [:])
                    } catch is CancellationError {
                        return
                    } catch {
                        runtime.store.reportRuntimeFailure(error)
                    }
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
            runtime.store.clearNativeEventSubscriptions(scope: "app")
            runtime.store.clearNativeTasks(scope: "app")
        }
        .task { runtime.connect() }
        .onChange(of: activeInput) { runtime.store.focusChanged(to: $0) }
        .onChange(of: runtime.store.focusedFieldKey) { activeInput = $0 }
        .onChange(of: runtime.performanceOverlayEnabled) { enabled in
            performance.setEnabled(enabled)
        }
        .environment(\.layoutDirection, configuredLayoutDirection)
        .environment(\.nexaSharedNamespace, nexaSharedNamespace)
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
