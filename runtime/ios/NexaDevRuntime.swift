import Foundation
import CoreFoundation
import QuartzCore
import SwiftUI
import UIKit

struct NexaDevVisualSafeAreaSnapshot: Equatable {
    var top: CGFloat = 0
    var bottom: CGFloat = 0
    var left: CGFloat = 0
    var right: CGFloat = 0

    func logical(_ direction: LayoutDirection) -> EdgeInsets {
        EdgeInsets(
            top: top,
            leading: direction == .rightToLeft ? right : left,
            bottom: bottom,
            trailing: direction == .rightToLeft ? left : right
        )
    }

    func devRemoving(_ edge: Edge, direction: LayoutDirection) -> Self {
        switch edge {
        case .top: return Self(top: 0, bottom: bottom, left: left, right: right)
        case .bottom: return Self(top: top, bottom: 0, left: left, right: right)
        case .leading:
            return direction == .rightToLeft
                ? Self(top: top, bottom: bottom, left: left, right: 0)
                : Self(top: top, bottom: bottom, left: 0, right: right)
        case .trailing:
            return direction == .rightToLeft
                ? Self(top: top, bottom: bottom, left: 0, right: right)
                : Self(top: top, bottom: bottom, left: left, right: 0)
        }
    }

}

struct NexaDevVisualSafeInsetsKey: EnvironmentKey {
    static let defaultValue = NexaDevVisualSafeAreaSnapshot()
}
struct NexaDevViewportSizeKey: EnvironmentKey {
    static let defaultValue = CGSize.zero
}

extension EnvironmentValues {
    var nexaDevVisualSafeInsets: NexaDevVisualSafeAreaSnapshot {
        get { self[NexaDevVisualSafeInsetsKey.self] }
        set { self[NexaDevVisualSafeInsetsKey.self] = newValue }
    }

    var nexaDevViewportSize: CGSize {
        get { self[NexaDevViewportSizeKey.self] }
        set { self[NexaDevViewportSizeKey.self] = newValue }
    }
}

private final class NexaDevSafeAreaInsetsProbeView: UIView {
    var onInsetsChange: ((UIEdgeInsets) -> Void)?
    private var lastReportedInsets: UIEdgeInsets?

    override func didMoveToWindow() {
        super.didMoveToWindow()
        reportInsets()
    }

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        reportInsets()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        reportInsets()
    }

    func reportInsets() {
        guard let window else { return }
        let insets = window.safeAreaInsets
        guard insets != lastReportedInsets else { return }
        lastReportedInsets = insets
        DispatchQueue.main.async { [weak self] in
            guard self?.window != nil else { return }
            self?.onInsetsChange?(insets)
        }
    }
}

private struct NexaDevSafeAreaInsetsProbe: UIViewRepresentable {
    let onChange: (UIEdgeInsets) -> Void

    func makeUIView(context: Context) -> NexaDevSafeAreaInsetsProbeView {
        let view = NexaDevSafeAreaInsetsProbeView()
        view.isUserInteractionEnabled = false
        view.backgroundColor = .clear
        view.onInsetsChange = onChange
        return view
    }

    func updateUIView(_ view: NexaDevSafeAreaInsetsProbeView, context: Context) {
        view.onInsetsChange = onChange
        view.reportInsets()
    }
}


private struct NexaDevSafeAreaInsetsPadding: ViewModifier {
    let insets: EdgeInsets

    func body(content: Content) -> some View {
        content.padding(insets)
    }
}

private struct NexaDevNativeSafeAreaInset<Attached: View>: ViewModifier {
    let edge: Edge
    @ViewBuilder let attached: () -> Attached

    @ViewBuilder
    func body(content: Content) -> some View {
        switch edge {
        case .top:
            content.safeAreaInset(edge: .top, spacing: 0) { attached() }
        case .bottom:
            content.safeAreaInset(edge: .bottom, spacing: 0) { attached() }
        case .leading:
            content.safeAreaInset(edge: .leading, spacing: 0) { attached() }
        case .trailing:
            content.safeAreaInset(edge: .trailing, spacing: 0) { attached() }
        }
    }
}

struct NexaDevSafeAreaInsetPrimitive<Content: View, Attached: View>: View {
    let edge: Edge
    let accommodatesEndpoint: Bool
    @ViewBuilder let content: () -> Content
    @ViewBuilder let attachedContent: () -> Attached
    @Environment(\.nexaDevVisualSafeInsets) private var safeAreaSnapshot
    @Environment(\.layoutDirection) private var layoutDirection

    private var safeInsets: EdgeInsets { safeAreaSnapshot.logical(layoutDirection) }

    var body: some View {
        if accommodatesEndpoint {
            content()
                .environment(
                    \.nexaDevVisualSafeInsets,
                    safeAreaSnapshot.devRemoving(edge, direction: layoutDirection)
                )
                .modifier(NexaDevNativeSafeAreaInset(edge: edge) {
                    attachedContent()
                        .modifier(NexaDevSafeAreaInsetsPadding(insets: safeInsets.devPerpendicular(to: edge)))
                        .environment(
                            \.nexaDevVisualSafeInsets,
                            safeAreaSnapshot.devRemoving(edge, direction: layoutDirection)
                        )
                })
        } else {
            GeometryReader { proxy in
                content()
                    .environment(\.nexaSafeAreaRootFillsWidth, true)
                    .environment(
                        \.nexaDevVisualSafeInsets,
                        safeAreaSnapshot.devRemoving(edge, direction: layoutDirection)
                    )
                    .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                    .overlay(alignment: edge.devAlignment) {
                        attachedContent()
                            .modifier(NexaDevSafeAreaInsetsPadding(insets: safeInsets.devPerpendicular(to: edge)))
                            .environment(
                                \.nexaDevVisualSafeInsets,
                                safeAreaSnapshot.devRemoving(edge, direction: layoutDirection)
                            )
                    }
            }
        }
    }
}

private extension EdgeInsets {
    func devPerpendicular(to edge: Edge) -> EdgeInsets {
        switch edge {
        case .top, .bottom:
            return EdgeInsets(top: 0, leading: leading, bottom: 0, trailing: trailing)
        case .leading, .trailing:
            return EdgeInsets(top: top, leading: 0, bottom: bottom, trailing: 0)
        }
    }

}

private extension Edge {
    var devAlignment: Alignment {
        switch self {
        case .top: return .top
        case .bottom: return .bottom
        case .leading: return .leading
        case .trailing: return .trailing
        }
    }
}

private struct NexaDevEdgeToEdgeRoot<Content: View>: View {
    private let content: Content
    @State private var containerSafeInsets = NexaDevVisualSafeAreaSnapshot()

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        GeometryReader { proxy in
            let sharedSafeInsets = NexaVisualSafeAreaSnapshot(
                top: containerSafeInsets.top,
                bottom: containerSafeInsets.bottom,
                left: containerSafeInsets.left,
                right: containerSafeInsets.right
            )
            content
                .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                .environment(\.nexaDevVisualSafeInsets, containerSafeInsets)
                .environment(
                    \.nexaWindowSafeAreaInsets,
                    sharedSafeInsets
                )
                .environment(\.nexaDevViewportSize, proxy.size)
                .environment(
                    \.nexaWindowMetrics,
                    NexaWindowMetricsSnapshot(viewportSize: proxy.size, safeInsets: sharedSafeInsets)
                )
        }
        .background {
            NexaDevSafeAreaInsetsProbe { insets in
                let next = NexaDevVisualSafeAreaSnapshot(
                    top: insets.top,
                    bottom: insets.bottom,
                    left: insets.left,
                    right: insets.right
                )
                guard containerSafeInsets != next else { return }
                containerSafeInsets = next
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
        }
        .ignoresSafeArea(.container, edges: .all)
        .ignoresSafeArea(.keyboard, edges: .bottom)
    }
}

/// Zero-size probe that tells the render fence when SwiftUI evaluated the tree
/// for a newly installed module generation. It is inert in release builds.
private struct NexaDevFrameFenceProbe: View {
    @ObservedObject var store: NexaDevStateStore
    let fence: NexaDevFrameFence

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
            .onChange(of: store.moduleGeneration) { generation in
                fence.noteComposed(generation: generation)
            }
    }
}

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
    @Environment(\.nexaDevVisualSafeInsets) private var visualSafeInsets

    public init(serverURL: String, sessionToken: String) {
        _runtime = StateObject(
            wrappedValue: NexaDevRuntime(serverURL: serverURL, sessionToken: sessionToken)
        )
    }

    private var configuredLayoutDirection: LayoutDirection {
        guard let direction = runtime.module?["direction"] as? [String: Any],
              let style = direction["style"] as? String
        else {
            let override = runtime.store.languageOverride ?? UserDefaults.standard.string(forKey: "dev.nexa.locale.override")
            guard override != nil else {
                return inheritedLayoutDirection
            }
            return Locale.Language(identifier: override ?? nexaCurrentLanguageCode()).characterDirection == .rightToLeft
                ? .rightToLeft
                : .leftToRight
        }
        return style == "Rtl" ? .rightToLeft : .leftToRight
    }

    private var configuredLocale: Locale {
        let override = runtime.store.languageOverride ?? UserDefaults.standard.string(forKey: "dev.nexa.locale.override")
        return Locale(identifier: override ?? Locale.current.identifier)
    }

    public var body: some View {
        let screens = runtime.module?["screens"] as? [[String: Any]] ?? []
        let hasScreenStatusBarConfiguration = screens.contains {
            $0["status_bar"] as? [String: Any] != nil
        }
        let rootStatusBar = hasScreenStatusBarConfiguration
            ? nil
            : runtime.module?["status_bar"] as? [String: Any]
        NexaDevEdgeToEdgeRoot {
        Group {
            if let module = runtime.module {
                NexaDevNodeList(
                    nodes: module["body"] as? [Any] ?? [],
                    module: module,
                    store: runtime.store,
                    focusedField: $activeInput
                )
                // AOT wraps every screen root with ignoresSafeArea. The Dev
                // root is that same screen boundary, so apply it here too;
                // otherwise SwiftUI constrains the interpreted tree to the
                // safe frame before its Nexa safe-area modifiers run.
                .ignoresSafeArea(.container, edges: .all)
            } else {
                ProgressView("Connecting to Nexa…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(Color(uiColor: .systemBackground).ignoresSafeArea())
        .overlay(alignment: .topTrailing) {
            if !runtime.diagnostics.isEmpty {
                NexaDevErrorOverlay(
                    diagnostics: runtime.diagnostics,
                    onOpenInEditor: runtime.openInEditor
                )
                .padding(.horizontal, 12)
                .padding(.top, 8)
            }
        }
        .overlay(alignment: .topTrailing) {
            if runtime.performanceOverlayEnabled {
                NexaDevPerformanceOverlay(fps: performance.fps, frameTimeMs: performance.frameTimeMs)
                    .padding(8)
            }
        }
        .overlay {
            NexaDevFrameFenceProbe(store: runtime.store, fence: runtime.frameFence)
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
        // Own the socket task in the runtime object instead of SwiftUI's view
        // task lifecycle. The interpreted root can switch from a loading view
        // to the app tree while the initial module is being installed.
        .onAppear { runtime.startConnection() }
        .onChange(of: scenePhase) { phase in
            if phase == .active { runtime.startConnection() }
        }
        .onChange(of: activeInput) { runtime.store.focusChanged(to: $0) }
        .onChange(of: runtime.store.focusedFieldKey) { activeInput = $0 }
        .onChange(of: runtime.performanceOverlayEnabled) { enabled in
            performance.setEnabled(enabled)
        }
        .environment(\.locale, configuredLocale)
        .environment(\.layoutDirection, configuredLayoutDirection)
        .environment(\.nexaSharedNamespace, nexaSharedNamespace)
        .modifier(NexaDevStatusBarConfigurationModifier(
            config: rootStatusBar,
            safeInsets: visualSafeInsets,
            isDark: colorScheme == .dark
        ))
        }
    }
}
