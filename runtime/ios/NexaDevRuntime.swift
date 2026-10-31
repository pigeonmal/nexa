import Foundation
import CoreFoundation
import QuartzCore
import SwiftUI
import UIKit

struct NexaDevVisualSafeInsetsKey: EnvironmentKey {
    static let defaultValue = EdgeInsets()
}

struct NexaDevViewportSizeKey: EnvironmentKey {
    static let defaultValue = CGSize.zero
}

extension EnvironmentValues {
    var nexaDevVisualSafeInsets: EdgeInsets {
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


struct NexaDevSafeAreaPaddingPrimitive<Content: View>: View {
    let edges: Edge.Set
    let spacing: CGFloat
    @ViewBuilder let content: () -> Content
    @Environment(\.nexaDevVisualSafeInsets) private var safeInsets

    var body: some View {
        content()
            .padding(.top, edges.contains(.top) ? max(0, safeInsets.top) : 0)
            .padding(.bottom, edges.contains(.bottom) ? max(0, safeInsets.bottom) : 0)
            .padding(.leading, edges.contains(.leading) ? max(0, safeInsets.leading) : 0)
            .padding(.trailing, edges.contains(.trailing) ? max(0, safeInsets.trailing) : 0)
            .padding(.top, edges.contains(.top) ? spacing : 0)
            .padding(.bottom, edges.contains(.bottom) ? spacing : 0)
            .padding(.leading, edges.contains(.leading) ? spacing : 0)
            .padding(.trailing, edges.contains(.trailing) ? spacing : 0)
            .environment(\.nexaDevVisualSafeInsets, safeInsets.devRemoving(edges))
    }
}

private struct NexaDevSafeAreaEdgePadding: ViewModifier {
    let edge: Edge
    let amount: CGFloat

    @ViewBuilder
    func body(content: Content) -> some View {
        switch edge {
        case .top: content.padding(.top, amount)
        case .bottom: content.padding(.bottom, amount)
        case .leading: content.padding(.leading, amount)
        case .trailing: content.padding(.trailing, amount)
        }
    }
}

private struct NexaDevSafeAreaInsetsPadding: ViewModifier {
    let insets: EdgeInsets

    func body(content: Content) -> some View {
        content
            .padding(.top, insets.top)
            .padding(.leading, insets.leading)
            .padding(.bottom, insets.bottom)
            .padding(.trailing, insets.trailing)
    }
}

private struct NexaDevNativeSafeAreaInset<Attached: View>: ViewModifier {
    let edge: Edge
    let spacing: CGFloat
    @ViewBuilder let attached: () -> Attached

    @ViewBuilder
    func body(content: Content) -> some View {
        switch edge {
        case .top:
            content.safeAreaInset(edge: .top, spacing: spacing) { attached() }
        case .bottom:
            content.safeAreaInset(edge: .bottom, spacing: spacing) { attached() }
        case .leading:
            content.safeAreaInset(edge: .leading, spacing: spacing) { attached() }
        case .trailing:
            content.safeAreaInset(edge: .trailing, spacing: spacing) { attached() }
        }
    }
}

struct NexaDevSafeAreaInsetPrimitive<Content: View, Attached: View>: View {
    let edge: Edge
    let spacing: CGFloat
    let accommodatesEndpoint: Bool
    @ViewBuilder let content: () -> Content
    @ViewBuilder let attachedContent: () -> Attached
    @Environment(\.nexaDevVisualSafeInsets) private var safeInsets

    var body: some View {
        if accommodatesEndpoint {
            content()
                .environment(\.nexaDevVisualSafeInsets, safeInsets.devRemoving(edge))
                .modifier(NexaDevNativeSafeAreaInset(edge: edge, spacing: spacing) {
                    attachedContent()
                        .modifier(NexaDevSafeAreaInsetsPadding(insets: safeInsets.devPerpendicular(to: edge)))
                        .modifier(NexaDevSafeAreaEdgePadding(edge: edge, amount: safeInsets.devValue(at: edge)))
                        .environment(\.nexaDevVisualSafeInsets, safeInsets.devRemoving(edge))
                })
        } else {
            GeometryReader { proxy in
                content()
                    .environment(\.nexaSafeAreaRootFillsWidth, true)
                    .environment(\.nexaDevVisualSafeInsets, safeInsets.devRemoving(edge))
                    .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                    .overlay(alignment: edge.devAlignment) {
                        attachedContent()
                            .modifier(NexaDevSafeAreaInsetsPadding(insets: safeInsets.devPerpendicular(to: edge)))
                            .modifier(NexaDevSafeAreaEdgePadding(edge: edge, amount: safeInsets.devValue(at: edge)))
                            .modifier(NexaDevSafeAreaEdgePadding(edge: edge, amount: spacing))
                            .environment(\.nexaDevVisualSafeInsets, safeInsets.devRemoving(edge))
                    }
            }
        }
    }
}

private extension EdgeInsets {
    func devValue(at edge: Edge) -> CGFloat {
        switch edge {
        case .top: return top
        case .bottom: return bottom
        case .leading: return leading
        case .trailing: return trailing
        }
    }

    func devPerpendicular(to edge: Edge) -> EdgeInsets {
        switch edge {
        case .top, .bottom:
            return EdgeInsets(top: 0, leading: leading, bottom: 0, trailing: trailing)
        case .leading, .trailing:
            return EdgeInsets(top: top, leading: 0, bottom: bottom, trailing: 0)
        }
    }

    func devRemoving(_ edge: Edge) -> EdgeInsets {
        EdgeInsets(
            top: edge == .top ? 0 : top,
            leading: edge == .leading ? 0 : leading,
            bottom: edge == .bottom ? 0 : bottom,
            trailing: edge == .trailing ? 0 : trailing
        )
    }

    func devRemoving(_ edges: Edge.Set) -> EdgeInsets {
        EdgeInsets(
            top: edges.contains(.top) ? 0 : top,
            leading: edges.contains(.leading) ? 0 : leading,
            bottom: edges.contains(.bottom) ? 0 : bottom,
            trailing: edges.contains(.trailing) ? 0 : trailing
        )
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
    @State private var containerSafeInsets = EdgeInsets()

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        GeometryReader { proxy in
            content
                .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                .environment(\.nexaDevVisualSafeInsets, containerSafeInsets)
                .environment(\.nexaDevViewportSize, proxy.size)
        }
        .background {
            NexaDevSafeAreaInsetsProbe { insets in
                let next = EdgeInsets(
                    top: insets.top,
                    leading: insets.left,
                    bottom: insets.bottom,
                    trailing: insets.right
                )
                guard containerSafeInsets.top != next.top
                    || containerSafeInsets.leading != next.leading
                    || containerSafeInsets.bottom != next.bottom
                    || containerSafeInsets.trailing != next.trailing
                else { return }
                containerSafeInsets = next
            }
            .frame(width: 0, height: 0)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
        }
        .ignoresSafeArea(.container, edges: .all)
        .ignoresSafeArea(.keyboard, edges: .bottom)
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
        let statusBar = runtime.module?["status_bar"] as? [String: Any] ?? [:]
        let statusBarColorScheme: ColorScheme? = switch statusBar["style"] as? String {
        case "Light": .dark
        case "Dark": .light
        default: .dark
        }
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
        .background(Color.black.ignoresSafeArea())
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
        .modifier(NexaDevStatusBarVisibility(hidden: statusBar["hidden"] as? Bool ?? false))
        .modifier(NexaDevOptionalPreferredColorScheme(colorScheme: statusBarColorScheme))
        .overlay(alignment: .top) {
            if let color = devStatusBarColor(statusBar["background"], isDark: colorScheme == .dark) {
                color
                    .frame(maxWidth: .infinity)
                    .frame(height: max(0, visualSafeInsets.top))
                    .ignoresSafeArea(edges: .top)
                .allowsHitTesting(false)
            }
        }
        }
    }
}

private struct NexaDevOptionalPreferredColorScheme: ViewModifier {
    let colorScheme: ColorScheme?

    @ViewBuilder
    func body(content: Content) -> some View {
        if let colorScheme {
            content.preferredColorScheme(colorScheme)
        } else {
            content
        }
    }
}
