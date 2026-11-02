use nexa_codegen::{GeneratedSources, SourceUnit, SourceUnits};
use nexa_ir::{LayoutKind, Module, Node, State, ViewStyle};

mod api;
mod components;
mod engine;
mod widget;

// Re-exported through `generator` because `engine` is private and the crate
// root cannot name a path through it.
pub use engine::swift_scalar_types;

const NEXA_TASK_SCOPE_HELPER: &str = r#"@MainActor
final class NexaTaskScope {
    private var tasks: [UUID: Task<Void, Never>] = [:]

    func track(_ task: Task<Void, Never>) -> Task<Void, Never> {
        let id = UUID()
        tasks[id] = task
        Task { @MainActor [weak self] in
            await task.value
            self?.tasks.removeValue(forKey: id)
        }
        return task
    }

    func cancelAll() {
        let runningTasks = Array(tasks.values)
        tasks.removeAll()
        for task in runningTasks {
            task.cancel()
        }
    }
}

"#;

const NEXA_LIST_TASK_SCOPE_VIEW_HELPER: &str = r#"@MainActor
private struct NexaTaskScopeView<Content: View>: View {
    @State private var nexaTaskScope = NexaTaskScope()
    private let content: (NexaTaskScope) -> Content

    init(@ViewBuilder content: @escaping (NexaTaskScope) -> Content) {
        self.content = content
    }

    var body: some View {
        content(nexaTaskScope)
            .onDisappear { nexaTaskScope.cancelAll() }
    }
}

"#;

const DRAG_GESTURE_HELPERS: &str = r#"@MainActor
private final class NexaDragVelocityTracker {
    private var lastTranslation = CGSize.zero
    private var lastTime: Date?

    func update(_ value: DragGesture.Value) -> CGSize {
        defer {
            lastTranslation = value.translation
            lastTime = value.time
        }
        if #available(iOS 18.0, *) {
            return value.velocity
        }
        guard let lastTime else { return .zero }
        let elapsed = value.time.timeIntervalSince(lastTime)
        guard elapsed > 0 else { return .zero }
        return CGSize(
            width: (value.translation.width - lastTranslation.width) / elapsed,
            height: (value.translation.height - lastTranslation.height) / elapsed
        )
    }

    func reset() {
        lastTranslation = .zero
        lastTime = nil
    }
}

@MainActor
private final class NexaMagnificationTracker {
    private var previousMagnification = 1.0

    func consume(_ magnification: Double) -> Double? {
        guard previousMagnification > 0, magnification.isFinite else { return nil }
        let scaleFactor = magnification / previousMagnification
        previousMagnification = magnification
        return scaleFactor.isFinite ? scaleFactor : nil
    }

    func reset() {
        previousMagnification = 1.0
    }
}

@MainActor
private struct NexaDragGestureView<Content: View>: View {
    let enabled: Bool
    let onDrag: ((Double, Double, Double, Double) -> Void)?
    let onPinch: ((Double) -> Void)?
    let content: Content
    @State private var velocityTracker: NexaDragVelocityTracker?
    @State private var magnificationTracker = NexaMagnificationTracker()

    init(
        enabled: Bool,
        trackVelocity: Bool,
        onDrag: ((Double, Double, Double, Double) -> Void)?,
        onPinch: ((Double) -> Void)?,
        @ViewBuilder content: () -> Content
    ) {
        self.enabled = enabled
        self.onDrag = onDrag
        self.onPinch = onPinch
        self.content = content()
        _velocityTracker = State(
            initialValue: trackVelocity ? NexaDragVelocityTracker() : nil
        )
    }

    private var dragGesture: some Gesture {
        DragGesture()
            .onChanged { value in
                guard enabled else { return }
                let velocity = velocityTracker?.update(value) ?? .zero
                onDrag?(
                    Double(value.translation.width),
                    Double(value.translation.height),
                    Double(velocity.width),
                    Double(velocity.height)
                )
            }
            .onEnded { _ in velocityTracker?.reset() }
    }

    private var pinchGesture: some Gesture {
        MagnificationGesture()
            .onChanged { magnification in
                guard enabled else { return }
                guard let scaleFactor = magnificationTracker.consume(Double(magnification)) else {
                    return
                }
                onPinch?(scaleFactor)
            }
            .onEnded { _ in magnificationTracker.reset() }
    }

    @ViewBuilder
    var body: some View {
        if onDrag != nil, onPinch != nil {
            content
                .highPriorityGesture(dragGesture)
                .simultaneousGesture(pinchGesture)
        } else if onDrag != nil {
            content.highPriorityGesture(dragGesture)
        } else if onPinch != nil {
            content.simultaneousGesture(pinchGesture)
        } else {
            content
        }
    }
}

"#;

const NEXA_DYNAMIC_COLOR_HELPER: &str = r##"private func nexaColor(hex: String) -> Color {
    let digits = hex.hasPrefix("#") ? String(hex.dropFirst()) : hex
    guard let value = UInt64(digits, radix: 16) else { return __NEXA_SWIFT_DEFAULT_ACCENT_COLOR__ }
    let red: UInt64
    let green: UInt64
    let blue: UInt64
    let alpha: UInt64
    switch digits.count {
    case 6:
        red = (value >> 16) & 0xFF
        green = (value >> 8) & 0xFF
        blue = value & 0xFF
        alpha = 0xFF
    case 8:
        red = (value >> 24) & 0xFF
        green = (value >> 16) & 0xFF
        blue = (value >> 8) & 0xFF
        alpha = value & 0xFF
    default:
        return __NEXA_SWIFT_DEFAULT_ACCENT_COLOR__
    }
    return Color(
        .sRGB,
        red: Double(red) / 255,
        green: Double(green) / 255,
        blue: Double(blue) / 255,
        opacity: Double(alpha) / 255
    )
}
"##;

const NEXA_GLASS_HELPER: &str = r#"fileprivate enum NexaGlassShape {
    case circle
    case capsule
    case rounded(Double)
}

fileprivate extension View {
    @ViewBuilder
    func nexaGlass(tint: Color? = nil, shape: NexaGlassShape = .circle) -> some View {
        if #available(iOS 26.0, *) {
            switch shape {
            case .circle:
                if let tint {
                    self.glassEffect(.clear.tint(tint).interactive(), in: .circle)
                } else {
                    self.glassEffect(.clear.interactive(), in: .circle)
                }
            case .capsule:
                if let tint {
                    self.glassEffect(.clear.tint(tint).interactive(), in: .capsule)
                } else {
                    self.glassEffect(.clear.interactive(), in: .capsule)
                }
            case .rounded(let r):
                if let tint {
                    self.glassEffect(.clear.tint(tint).interactive(), in: .rect(cornerRadius: CGFloat(r)))
                } else {
                    self.glassEffect(.clear.interactive(), in: .rect(cornerRadius: CGFloat(r)))
                }
            }
        } else {
            switch shape {
            case .circle:
                self.background(.ultraThinMaterial, in: Circle())
            case .capsule:
                self.background(.ultraThinMaterial, in: Capsule())
            case .rounded(let r):
                self.background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: CGFloat(r)))
            }
        }
    }

}
"#;

const NEXA_LOCALE_HELPER: &str = r#"enum NexaLocale {
    private static let overrideKey = "dev.nexa.locale.override"

    private static var overrideCode: String? {
        UserDefaults.standard.string(forKey: overrideKey)
    }

    static var currentLanguageCode: String {
        let identifier = overrideCode ?? Locale.current.identifier
        guard let language = identifier
        .split(whereSeparator: { $0 == "-" || $0 == "_" })
        .first,
              language.count > 1
        else { return "und" }
        return language.lowercased()
    }

    static var preferredLanguageCodes: [String] {
        guard let overrideCode else { return Locale.preferredLanguages }
        return [overrideCode] + Locale.preferredLanguages.filter {
            $0.caseInsensitiveCompare(overrideCode) != .orderedSame
                && $0.split(whereSeparator: { $0 == "-" || $0 == "_" }).first?.lowercased() != currentLanguageCode
        }
    }

    static var formatLocale: Locale {
        Locale(identifier: overrideCode ?? Locale.current.identifier)
    }

    static func formatLocale(override languageCode: String) -> Locale {
        Locale(identifier: languageCode.isEmpty ? Locale.current.identifier : languageCode)
    }

    static var isRightToLeft: Bool {
        Locale.Language(identifier: currentLanguageCode).characterDirection == .rightToLeft
    }

    static func isRightToLeft(override languageCode: String) -> Bool {
        let identifier = languageCode.isEmpty ? Locale.current.identifier : languageCode
        return Locale.Language(identifier: identifier).characterDirection == .rightToLeft
    }

    static func setLanguageOverride(_ languageCode: String?) {
        guard let languageCode else {
            UserDefaults.standard.removeObject(forKey: overrideKey)
            return
        }
        let trimmed = languageCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            UserDefaults.standard.removeObject(forKey: overrideKey)
            return
        }
        let locale = Locale(identifier: trimmed.replacingOccurrences(of: "_", with: "-"))
        guard let language = locale.language.languageCode?.identifier, language != "und" else { return }
        UserDefaults.standard.set(locale.identifier, forKey: overrideKey)
    }

    static func displayName(languageCode: String) -> String? {
        formatLocale.localizedString(forLanguageCode: languageCode)
    }

    static func localizedString(key: String, value: String, comment: String) -> String {
        let bundle = overrideCode
            .flatMap { code in
                let base = code.split(whereSeparator: { $0 == "-" || $0 == "_" }).first.map(String.init)
                for candidate in [code, base].compactMap({ $0 }) {
                    if let path = Bundle.main.path(forResource: candidate, ofType: "lproj"),
                       let bundle = Bundle(path: path) {
                        return bundle
                    }
                }
                return nil
            } ?? .main
        return bundle.localizedString(forKey: key, value: value, table: nil)
    }
}

"#;

const NEXA_LOCALIZATION_HELPER: &str = r#"enum NexaLocale {
    static var formatLocale: Locale { .current }

    static func localizedString(key: String, value: String, comment: String) -> String {
        Bundle.main.localizedString(forKey: key, value: value, table: nil)
    }
}
"#;

const NEXA_COLLECTION_HELPERS: &str = r#"private func nexaGroupByStable<Element, Key: Hashable>(
    _ elements: [Element],
    by keyForElement: (Element) -> Key
) -> [[Element]] {
    var sectionByKey: [Key: Int] = [:]
    var sections: [[Element]] = []
    for element in elements {
        let key = keyForElement(element)
        if let section = sectionByKey[key] {
            sections[section].append(element)
        } else {
            sectionByKey[key] = sections.count
            sections.append([element])
        }
    }
    return sections
}

"#;

const NEXA_SHARED_COMPONENT_PRIMITIVES: &str = concat!(
    r#"// nexa-codegen: NexaEdgeToEdgeRoot
struct NexaSafeAreaRootWidthKey: EnvironmentKey {
    static let defaultValue = false
}

extension EnvironmentValues {
    var nexaSafeAreaRootFillsWidth: Bool {
        get { self[NexaSafeAreaRootWidthKey.self] }
        set { self[NexaSafeAreaRootWidthKey.self] = newValue }
    }
}

struct NexaVisualSafeAreaSnapshot: Equatable {
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

    func removing(_ edge: Edge, direction: LayoutDirection) -> Self {
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

struct NexaWindowMetricsSnapshot: Equatable {
    let viewportSize: CGSize
    let safeInsets: NexaVisualSafeAreaSnapshot
    let keyboardInsets: UIEdgeInsets

    init(
        viewportSize: CGSize = .zero,
        safeInsets: NexaVisualSafeAreaSnapshot = NexaVisualSafeAreaSnapshot(),
        keyboardInsets: UIEdgeInsets = .zero
    ) {
        self.viewportSize = viewportSize
        self.safeInsets = safeInsets
        self.keyboardInsets = keyboardInsets
    }
}

struct NexaWindowMetricsKey: EnvironmentKey {
    static let defaultValue = NexaWindowMetricsSnapshot()
}

struct NexaVisualSafeAreaInsetsKey: EnvironmentKey {
    static let defaultValue = NexaVisualSafeAreaSnapshot()
}

struct NexaWindowSafeAreaInsetsKey: EnvironmentKey {
    static let defaultValue = NexaVisualSafeAreaSnapshot()
}

private struct NexaFormTopPaddingKey: EnvironmentKey {
    static let defaultValue: CGFloat = 0
}

extension EnvironmentValues {
    var nexaWindowMetrics: NexaWindowMetricsSnapshot {
        get { self[NexaWindowMetricsKey.self] }
        set { self[NexaWindowMetricsKey.self] = newValue }
    }

    var nexaVisualSafeAreaInsets: NexaVisualSafeAreaSnapshot {
        get { self[NexaVisualSafeAreaInsetsKey.self] }
        set { self[NexaVisualSafeAreaInsetsKey.self] = newValue }
    }

    var nexaWindowSafeAreaInsets: NexaVisualSafeAreaSnapshot {
        get { self[NexaWindowSafeAreaInsetsKey.self] }
        set { self[NexaWindowSafeAreaInsetsKey.self] = newValue }
    }

    var nexaFormTopPadding: CGFloat {
        get { self[NexaFormTopPaddingKey.self] }
        set { self[NexaFormTopPaddingKey.self] = newValue }
    }

}

struct NexaViewportSizeKey: EnvironmentKey {
    static let defaultValue = CGSize.zero
}

extension EnvironmentValues {
    var nexaViewportSize: CGSize {
        get { self[NexaViewportSizeKey.self] }
        set { self[NexaViewportSizeKey.self] = newValue }
    }
}

private struct NexaAnimatedWhen<Value: Hashable, Content: View>: View {
    let value: Value
    let animation: Animation
    let transition: AnyTransition
    private let content: (Value) -> Content
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var displayedValue: Value

    init(
        value: Value,
        animation: Animation,
        transition: AnyTransition,
        @ViewBuilder content: @escaping (Value) -> Content
    ) {
        self.value = value
        self.animation = animation
        self.transition = transition
        self.content = content
        self._displayedValue = State(initialValue: value)
    }

    var body: some View {
        content(displayedValue)
            .id(displayedValue)
            .transition(reduceMotion ? .identity : transition)
            .onChange(of: value) { nextValue in
                if reduceMotion {
                    displayedValue = nextValue
                } else {
                    withAnimation(animation) {
                        displayedValue = nextValue
                    }
                }
            }
    }
}

private final class NexaSafeAreaInsetsProbeView: UIView {
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

private struct NexaSafeAreaInsetsProbe: UIViewRepresentable {
    let onChange: (UIEdgeInsets) -> Void

    func makeUIView(context: Context) -> NexaSafeAreaInsetsProbeView {
        let view = NexaSafeAreaInsetsProbeView()
        view.isUserInteractionEnabled = false
        view.backgroundColor = .clear
        view.onInsetsChange = onChange
        return view
    }

    func updateUIView(_ view: NexaSafeAreaInsetsProbeView, context: Context) {
        view.onInsetsChange = onChange
        view.reportInsets()
    }
}

private struct NexaEdgeToEdgeRoot<Content: View>: View {
    private let content: (NexaWindowMetricsSnapshot) -> Content
    @State private var containerSafeInsets = NexaVisualSafeAreaSnapshot()

    init(@ViewBuilder content: @escaping (NexaWindowMetricsSnapshot) -> Content) {
        self.content = content
    }

    var body: some View {
        GeometryReader { proxy in
            let metrics = NexaWindowMetricsSnapshot(
                viewportSize: proxy.size,
                safeInsets: containerSafeInsets
            )
            content(metrics)
                .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                .environment(\.nexaWindowMetrics, metrics)
                .environment(\.nexaVisualSafeAreaInsets, containerSafeInsets)
                .environment(\.nexaWindowSafeAreaInsets, containerSafeInsets)
                .environment(\.nexaViewportSize, proxy.size)
        }
        .background {
            NexaSafeAreaInsetsProbe { insets in
                let next = NexaVisualSafeAreaSnapshot(
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

// nexa-codegen: NexaViewInsetsPrimitive
struct NexaViewInsetsPrimitive<Content: View>: View {
    let padding: EdgeInsets
    let margin: EdgeInsets
    var scrollContent: Bool = false
    @ViewBuilder let content: () -> Content

    @ViewBuilder
    var body: some View {
        if scrollContent {
            if margin.top == 0, margin.leading == 0, margin.bottom == 0, margin.trailing == 0 {
                NexaScrollViewport {
                    content().environment(\.nexaScrollContentPadding, padding)
                }
            } else {
                NexaScrollMarginViewport(margin: margin) {
                    content().environment(\.nexaScrollContentPadding, padding)
                }
            }
        } else {
            content().padding(padding).padding(margin)
        }
    }
}

// nexa-codegen: nexaScrollContentPadding
private struct NexaScrollContentPaddingKey: EnvironmentKey {
    static let defaultValue = EdgeInsets()
}

extension EnvironmentValues {
    var nexaScrollContentPadding: EdgeInsets {
        get { self[NexaScrollContentPaddingKey.self] }
        set { self[NexaScrollContentPaddingKey.self] = newValue }
    }
}

// nexa-codegen: NexaScrollContent
struct NexaScrollContent<Content: View>: View {
    @Environment(\.nexaScrollContentPadding) private var padding
    @ViewBuilder let content: () -> Content

    var body: some View {
        content()
            .environment(\.nexaScrollContentPadding, EdgeInsets())
            .padding(padding)
    }
}

// nexa-codegen: NexaRefreshPrimitive
struct NexaRefreshPrimitive<Content: View>: View {
    let hasNativeScrollOwner: Bool
    @ViewBuilder let content: () -> Content

    @ViewBuilder
    var body: some View {
        if hasNativeScrollOwner {
            content()
        } else {
            NexaScrollViewport {
                ScrollView { NexaScrollContent { content() } }
            }
        }
    }
}

// nexa-codegen: NexaParentRelativeDimensionModifier
struct NexaParentRelativeDimensionModifier: ViewModifier {
    var width: CGFloat? = nil
    var height: CGFloat? = nil
    var minWidth: CGFloat? = nil
    var maxWidth: CGFloat? = nil
    var minHeight: CGFloat? = nil
    var maxHeight: CGFloat? = nil

    func body(content: Content) -> some View {
        NexaParentRelativeDimensionLayout(
            width: width,
            height: height,
            minWidth: minWidth,
            maxWidth: maxWidth,
            minHeight: minHeight,
            maxHeight: maxHeight
        ) { content }
    }
}

private struct NexaParentRelativeDimensionLayout: Layout {
    let width: CGFloat?
    let height: CGFloat?
    let minWidth: CGFloat?
    let maxWidth: CGFloat?
    let minHeight: CGFloat?
    let maxHeight: CGFloat?
    struct Cache {}

    func makeCache(subviews: Subviews) -> Cache { Cache() }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) -> CGSize {
        guard let child = subviews.first else { return .zero }
        let boundedWidth = proposal.width.flatMap { $0.isFinite ? $0 : nil }
        let boundedHeight = proposal.height.flatMap { $0.isFinite ? $0 : nil }
        let targetWidth = width.flatMap { fraction in boundedWidth.map { max(0, $0 * fraction) } }
        let targetHeight = height.flatMap { fraction in boundedHeight.map { max(0, $0 * fraction) } }
        let measured = child.sizeThatFits(ProposedViewSize(
            width: targetWidth ?? proposal.width,
            height: targetHeight ?? proposal.height
        ))
        let measuredWidth = targetWidth ?? measured.width
        let measuredHeight = targetHeight ?? measured.height
        let resolvedWidth = min(max(measuredWidth, minWidth.flatMap { fraction in boundedWidth.map { $0 * fraction } } ?? 0), maxWidth.flatMap { fraction in boundedWidth.map { $0 * fraction } } ?? .infinity)
        let resolvedHeight = min(max(measuredHeight, minHeight.flatMap { fraction in boundedHeight.map { $0 * fraction } } ?? 0), maxHeight.flatMap { fraction in boundedHeight.map { $0 * fraction } } ?? .infinity)
        return CGSize(width: resolvedWidth, height: resolvedHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout Cache) {
        subviews.first?.place(
            at: bounds.origin,
            anchor: .topLeading,
            proposal: ProposedViewSize(width: bounds.width, height: bounds.height)
        )
    }
}

// nexa-codegen: NexaSafeAreaInsetPrimitive
private struct NexaSafeAreaInsetsPadding: ViewModifier {
    let insets: EdgeInsets

    func body(content: Content) -> some View {
        content.padding(insets)
    }
}

private struct NexaNativeSafeAreaInset<Attached: View>: ViewModifier {
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

private struct NexaSafeAreaInsetPrimitive<Content: View, Attached: View>: View {
    let edge: Edge
    let accommodatesEndpoint: Bool
    @ViewBuilder let content: () -> Content
    @ViewBuilder let attachedContent: () -> Attached
    @Environment(\.nexaVisualSafeAreaInsets) private var safeAreaSnapshot
    @Environment(\.layoutDirection) private var layoutDirection

    private var safeInsets: EdgeInsets { safeAreaSnapshot.logical(layoutDirection) }

    @ViewBuilder
    var body: some View {
        if accommodatesEndpoint {
            content()
                .environment(
                    \.nexaVisualSafeAreaInsets,
                    safeAreaSnapshot.removing(edge, direction: layoutDirection)
                )
                .modifier(NexaNativeSafeAreaInset(edge: edge) {
                    attachedContent()
                        .modifier(NexaSafeAreaInsetsPadding(insets: safeInsets.perpendicular(to: edge)))
                        .environment(
                            \.nexaVisualSafeAreaInsets,
                            safeAreaSnapshot.removing(edge, direction: layoutDirection)
                        )
                })
        } else {
            GeometryReader { proxy in
                content()
                    .environment(\.nexaSafeAreaRootFillsWidth, true)
                    .environment(
                        \.nexaVisualSafeAreaInsets,
                        safeAreaSnapshot.removing(edge, direction: layoutDirection)
                    )
                    .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
                    .overlay(alignment: edge.nexaAlignment) {
                        attachedContent()
                            .modifier(NexaSafeAreaInsetsPadding(insets: safeInsets.perpendicular(to: edge)))
                            .environment(
                                \.nexaVisualSafeAreaInsets,
                                safeAreaSnapshot.removing(edge, direction: layoutDirection)
                            )
                    }
            }
        }
    }
}

private extension Edge {
    var nexaAlignment: Alignment {
        switch self {
        case .top: return .top
        case .bottom: return .bottom
        case .leading: return .leading
        case .trailing: return .trailing
        }
    }
}

// nexa-codegen: EdgeInsets
private extension EdgeInsets {
    func perpendicular(to edge: Edge) -> EdgeInsets {
        switch edge {
        case .top, .bottom:
            return EdgeInsets(top: 0, leading: leading, bottom: 0, trailing: trailing)
        case .leading, .trailing:
            return EdgeInsets(top: top, leading: 0, bottom: bottom, trailing: 0)
        }
    }

}

// nexa-codegen: NexaTextPrimitive
__NEXA_REDUCE_MOTION_ANIMATION_MODIFIER__

struct NexaTextStyle {
    var alignment: TextAlignment?
    var color: Color?
    var font: Font?
    var weight: Font.Weight?
    var lineLimit: Int?
    var lineHeightFactor: CGFloat?
    var lineHeightFontSize: CGFloat
    var tracking: CGFloat?
    var strikethrough: Bool
    var italic: Bool
    var underline: Bool
    var selectable: Bool

    init() {
        self.alignment = nil
        self.color = nil
        self.font = nil
        self.weight = nil
        self.lineLimit = nil
        self.lineHeightFactor = nil
        self.lineHeightFontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__
        self.tracking = nil
        self.strikethrough = false
        self.italic = false
        self.underline = false
        self.selectable = false
    }

    func withAlignment(_ value: TextAlignment) -> Self {
        var style = self
        style.alignment = value
        return style
    }

    func withColor(_ value: Color) -> Self {
        var style = self
        style.color = value
        return style
    }

    func withFont(_ value: Font) -> Self {
        var style = self
        style.font = value
        return style
    }

    func withWeight(_ value: Font.Weight) -> Self {
        var style = self
        style.weight = value
        return style
    }

    func withLineLimit(_ value: Int) -> Self {
        var style = self
        style.lineLimit = value
        return style
    }

    func withLineHeight(factor: CGFloat, fontSize: CGFloat) -> Self {
        var style = self
        style.lineHeightFactor = factor
        style.lineHeightFontSize = fontSize
        return style
    }

    func withTracking(_ value: CGFloat) -> Self {
        var style = self
        style.tracking = value
        return style
    }

    func withStrikethrough() -> Self {
        var style = self
        style.strikethrough = true
        return style
    }

    func withItalic() -> Self {
        var style = self
        style.italic = true
        return style
    }

    func withUnderline() -> Self {
        var style = self
        style.underline = true
        return style
    }

    func withSelectable() -> Self {
        var style = self
        style.selectable = true
        return style
    }
}

struct NexaTextPrimitive: View {
    let text: Text
    let style: NexaTextStyle

    init(text: Text) {
        self.text = text
        self.style = NexaTextStyle()
    }

    init(text: Text, style: NexaTextStyle) {
        self.text = text
        self.style = style
    }

    @ViewBuilder var body: some View {
        let styledText = text
            .font(style.font)
            .fontWeight(style.weight)
            .foregroundStyle(style.color ?? Color.primary)
            .lineLimit(style.lineLimit)
            .tracking(style.tracking ?? 0)
            .strikethrough(style.strikethrough)
            .italic(style.italic)
            .underline(style.underline)
            .__NEXA_TEXT_LINE_HEIGHT_MODIFIER__
        if style.selectable {
            if let alignment = style.alignment {
                styledText.multilineTextAlignment(alignment).textSelection(.enabled)
            } else {
                styledText.textSelection(.enabled)
            }
        } else {
            if let alignment = style.alignment {
                styledText.multilineTextAlignment(alignment)
            } else {
                styledText
            }
        }
    }
}

// nexa-codegen: NexaSwitchPrimitive
struct NexaSwitchPrimitive: View {
    let label: Text
    @Binding var isOn: Bool
    let onChange: ((Bool) -> Void)?

    private var selection: Binding<Bool> {
        Binding(
            get: { isOn },
            set: { nextValue in
                guard nextValue != isOn else { return }
                isOn = nextValue
                onChange?(nextValue)
            }
        )
    }

    var body: some View {
        HStack(spacing: 12) {
            label
                .foregroundColor(.white)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .onTapGesture { selection.wrappedValue.toggle() }
            Toggle("", isOn: selection)
                .labelsHidden()
                .tint(Color(uiColor: .systemBlue))
                .accessibilityLabel(label)
        }
    }
}

// nexa-codegen: NexaTextInputPrimitive
struct NexaTextInputPrimitive: View {
    @Binding var text: String
    let placeholder: Text
    let secure: Bool
    let multiline: Bool
    let searchable: Bool
    let maxLength: Int?
    let fontSize: CGFloat?
    let fontFamily: String?
    let fontRelativeTo: Font.TextStyle
    let fontDefaultSize: CGFloat
    let textColor: Color?
    let placeholderColor: Color?
    let clearable: Bool
    let keepsFocusOnNext: Bool
    let onSubmitAction: (() -> Void)?
    let onChange: ((String) -> Void)?
    @State private var correctionRevision = 0
    @FocusState private var nextSubmissionFocused: Bool

    private var explicitFont: Font? {
        if let fontFamily {
            return .custom(
                fontFamily,
                size: fontSize ?? fontDefaultSize,
                relativeTo: fontRelativeTo
            )
        }
        return fontSize.map { .system(size: $0) }
    }

    private var styledPlaceholder: Text {
        guard let placeholderColor else { return placeholder }
        return placeholder.foregroundColor(placeholderColor)
    }

    private var inputText: Binding<String> {
        Binding(
            get: { text },
            set: { nextValue in
                let limitedValue = maxLength.map { String(nextValue.prefix(max(0, $0))) } ?? nextValue
                let previousValue = text
                text = limitedValue
                if previousValue != limitedValue {
                    onChange?(limitedValue)
                }
                if limitedValue != nextValue {
                    correctionRevision &+= 1
                }
            }
        )
    }

    @ViewBuilder
    private var field: some View {
        if secure {
            SecureField(text: inputText, prompt: styledPlaceholder) { styledPlaceholder }
        } else if multiline {
            TextField(text: inputText, prompt: styledPlaceholder, axis: .vertical) { styledPlaceholder }
        } else {
            TextField(text: inputText, prompt: styledPlaceholder) { styledPlaceholder }
        }
    }

    @ViewBuilder
    private var focusRetainingField: some View {
        if keepsFocusOnNext {
            field
                .focused($nextSubmissionFocused)
                .font(explicitFont)
                .foregroundColor(textColor)
                .onSubmit {
                    onSubmitAction?()
                    // SwiftUI dismisses a lone field after its .next submit
                    // action unless focus is explicitly restored. Keep the
                    // native keyboard available when there is no focus target
                    // supplied by the caller.
                    DispatchQueue.main.async {
                        nextSubmissionFocused = true
                    }
                }
        } else {
            field
                .font(explicitFont)
                .foregroundColor(textColor)
        }
    }

    @ViewBuilder
    private var clearControl: some View {
        if clearable && !text.isEmpty {
            Button {
                inputText.wrappedValue = ""
            } label: {
                Image(systemName: "xmark.circle.fill")
                    .foregroundStyle(.tertiary)
                    .font(.system(size: 14))
            }
            .buttonStyle(.plain)
        }
    }

    @ViewBuilder
    var body: some View {
        // A rejected insertion can leave SwiftUI's native text control holding
        // the overlong editing buffer when the bound value is already at its
        // limit. Reading this revision makes the field reconcile from the
        // bounded binding after each rejected edit without replacing the
        // focused native control.
        let _ = correctionRevision
        if searchable {
            if clearable {
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(.secondary)
                        .accessibilityHidden(true)
                    leadingField
                    clearControl
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            } else {
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(.secondary)
                        .accessibilityHidden(true)
                    leadingField
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        } else if clearable {
            HStack(spacing: 8) {
                leadingField
                clearControl
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            leadingField
        }
    }

    // SwiftUI TextField otherwise measures to its placeholder's ideal width. Compose fields fill
    // the available parent width, so keep the SwiftUI primitive leading-aligned and equally wide.
    private var leadingField: some View {
        focusRetainingField
            .frame(maxWidth: .infinity, alignment: .leading)
            .multilineTextAlignment(.leading)
    }
}

// nexa-codegen: nexaPickerTint
private extension View {
    @ViewBuilder
    func nexaPickerTint(_ tint: Color?) -> some View {
        if let tint {
            self.tint(tint)
        } else {
            self
        }
    }
}

// nexa-codegen: NexaPickerPrimitive
struct NexaPickerPrimitive<LabelContent: View>: View {
    @Binding var selection: String
    let items: [String]
    let itemIcons: [NexaIconSpec]
    let icon: NexaIconSpec?
    let tint: Color?
    let hasLabel: Bool
    @ViewBuilder let label: () -> LabelContent

    private var resolvedTint: Color {
        tint ?? Color.accentColor
    }

    @ViewBuilder
    private func pickerOptions(selection: Binding<String>) -> some View {
        Picker("", selection: selection) {
            ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                if itemIcons.indices.contains(index) {
                    Label {
                        Text(item)
                    } icon: {
                        NexaIconView(icon: itemIcons[index], description: "", size: 16, tint: .secondary)
                    }
                        .tag(item)
                } else {
                    Text(item)
                        .tag(item)
                }
            }
        }
        .labelsHidden()
    }

    @ViewBuilder
    var body: some View {
        if hasLabel {
            Menu {
                pickerOptions(selection: $selection)
            } label: {
                HStack(spacing: 8) {
                    if let icon {
                        NexaIconView(icon: icon, description: "", size: 16)
                    }
                    label()
                    Spacer(minLength: 12)
                    HStack(spacing: 4) {
                        Text(selection)
                        Image(systemName: "chevron.up.chevron.down")
                            .font(.caption)
                    }
                }
                .foregroundStyle(resolvedTint)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityElement(children: .combine)
            .accessibilityValue(selection)
        } else if let icon {
            Menu {
                pickerOptions(selection: $selection)
            } label: {
                NexaIconView(
                    icon: icon,
                    description: "",
                    size: 16,
                    tint: tint ?? Color.accentColor,
                )
            }
            .accessibilityLabel("Choose an option")
            .accessibilityValue(selection)
        } else {
            pickerOptions(selection: $selection)
                .pickerStyle(.menu)
                .labelsHidden()
                .nexaPickerTint(tint)
        }
    }
}

"#,
    r#"// nexa-codegen: NexaSharedButtonStyleKind
enum NexaSharedButtonStyleKind {
    case borderedProminent
    case bordered
    case borderless
    case plain
}

// nexa-codegen: NexaSharedButtonSizeKind
enum NexaSharedButtonSizeKind {
    case small
    case regular
    case large
}

// nexa-codegen: NexaSharedButtonShapeKind
enum NexaSharedButtonShapeKind {
    case circle
    case capsule
    case rounded(CGFloat)
}

// nexa-codegen: NexaSharedButtonOutline
private struct NexaSharedButtonOutline: Shape {
    let kind: NexaSharedButtonShapeKind

    func path(in rect: CGRect) -> Path {
        switch kind {
        case .circle: Circle().path(in: rect)
        case .capsule: Capsule().path(in: rect)
        case .rounded(let radius): RoundedRectangle(cornerRadius: radius).path(in: rect)
        }
    }
}

// nexa-codegen: NexaSharedBorderedButtonStyle
private struct NexaSharedBorderedButtonStyle: ButtonStyle {
    let tint: Color
    let shape: NexaSharedButtonOutline
    let size: NexaSharedButtonSizeKind?

    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        let padding: (horizontal: CGFloat, vertical: CGFloat) = switch size {
        case .small: (CGFloat(__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_SMALL_VERTICAL_PADDING__))
        case .large: (CGFloat(__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_LARGE_VERTICAL_PADDING__))
        case .regular, nil: (CGFloat(__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__))
        }
        let contentColor = isEnabled ? tint : tint.opacity(0.38)
        let borderColor = isEnabled ? tint : tint.opacity(0.12)
        configuration.label
            .foregroundStyle(contentColor)
            .padding(.horizontal, padding.horizontal)
            .padding(.vertical, padding.vertical)
            .frame(minWidth: CGFloat(__NEXA_BUTTON_MIN_WIDTH__), minHeight: size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__))
            .background(tint.opacity(configuration.isPressed ? 0.12 : 0), in: shape)
            .overlay(shape.stroke(borderColor, lineWidth: 1))
            .contentShape(shape)
    }
}

// nexa-codegen: NexaSharedGlassModifier
private struct NexaSharedGlassModifier: ViewModifier {
    let enabled: Bool
    let tint: Color?
    let shape: NexaSharedButtonShapeKind

    @ViewBuilder
    func body(content: Content) -> some View {
        if enabled {
            if #available(iOS 26.0, *) {
                let effect = tint.map { Glass.clear.tint($0).interactive() } ?? Glass.clear.interactive()
                switch shape {
                case .circle: content.glassEffect(effect, in: Circle())
                case .capsule: content.glassEffect(effect, in: Capsule())
                case .rounded(let radius): content.glassEffect(effect, in: RoundedRectangle(cornerRadius: radius))
                }
            } else {
                switch shape {
                case .circle: content.background(.ultraThinMaterial, in: Circle())
                case .capsule: content.background(.ultraThinMaterial, in: Capsule())
                case .rounded(let radius): content.background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: radius))
                }
            }
        } else {
            content
        }
    }
}

// nexa-codegen: nexaSharedButtonShape
private extension View {
    @ViewBuilder
    func nexaSharedButtonShape(_ shape: NexaSharedButtonShapeKind) -> some View {
        if #available(iOS 17.0, *) {
            switch shape {
            case .circle: self.buttonBorderShape(.circle)
            case .capsule: self.buttonBorderShape(.capsule)
            case .rounded(let radius): self.buttonBorderShape(.roundedRectangle(radius: radius))
            }
        } else {
            switch shape {
            case .circle: self.clipShape(Circle())
            case .capsule: self.clipShape(Capsule())
            case .rounded(let radius): self.clipShape(RoundedRectangle(cornerRadius: radius))
            }
        }
    }
}

private struct NexaSharedProminentButtonStyle: ButtonStyle {
    let tint: Color
    let shape: NexaSharedButtonOutline
    let size: NexaSharedButtonSizeKind?
    let iconOnlyCircle: Bool

    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        let padding: (horizontal: CGFloat, vertical: CGFloat) = switch size {
        case .small: (CGFloat(__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_SMALL_VERTICAL_PADDING__))
        case .large: (CGFloat(__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_LARGE_VERTICAL_PADDING__))
        case .regular, nil: (CGFloat(__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__))
        }
        let contentColor = isEnabled ? Color.white : Color.primary.opacity(0.38)
        let containerColor = isEnabled ? tint : Color.primary.opacity(0.12)
        let minWidth = iconOnlyCircle ? CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__) : CGFloat(__NEXA_BUTTON_MIN_WIDTH__)
        let minHeight = size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__)
        configuration.label
            .foregroundStyle(contentColor)
            .padding(.horizontal, iconOnlyCircle ? 0 : padding.horizontal)
            .padding(.vertical, iconOnlyCircle ? 0 : padding.vertical)
            .frame(minWidth: minWidth, minHeight: minHeight)
            .background(containerColor, in: shape)
            .overlay(shape.fill(Color.white.opacity(configuration.isPressed && isEnabled ? 0.10 : 0)))
            .contentShape(shape)
    }
}

// nexa-codegen: NexaButtonPrimitive
struct NexaButtonPrimitive<Label: View>: View {
    let style: NexaSharedButtonStyleKind
    let size: NexaSharedButtonSizeKind?
    let shape: NexaSharedButtonShapeKind
    let tint: Color?
    let loading: Bool
    let disabled: Bool
    let accessibilityLabel: String?
    let glass: Bool
    let iconOnlyCircle: Bool
    let action: () -> Void
    private let label: Label

    init(
        style: NexaSharedButtonStyleKind,
        size: NexaSharedButtonSizeKind?,
        shape: NexaSharedButtonShapeKind,
        tint: Color?,
        loading: Bool,
        disabled: Bool,
        accessibilityLabel: String?,
        glass: Bool,
        iconOnlyCircle: Bool,
        action: @escaping () -> Void,
        @ViewBuilder label: () -> Label
    ) {
        self.style = style
        self.size = size
        self.shape = shape
        self.tint = tint
        self.loading = loading
        self.disabled = disabled
        self.glass = glass
        self.iconOnlyCircle = iconOnlyCircle
        self.accessibilityLabel = accessibilityLabel
        self.action = action
        self.label = label()
    }

    @ViewBuilder
    private var progressLabelContent: some View {
        ProgressView()
            .frame(
                width: CGFloat(__NEXA_BUTTON_LOADING_INDICATOR_SIZE__),
                height: CGFloat(__NEXA_BUTTON_LOADING_INDICATOR_SIZE__)
            )
    }

    @ViewBuilder
    private var labelContent: some View {
        if loading {
            progressLabelContent
        } else {
            label.font(.system(size: CGFloat(__NEXA_DEFAULT_BODY_FONT_SIZE__)))
        }
    }

    @ViewBuilder
    private var plainLabelContent: some View {
        if loading {
            progressLabelContent.tint(tint.map { ((disabled || loading) ? $0.opacity(0.38) : $0) })
        } else if let tint {
            label
                .font(.system(size: CGFloat(__NEXA_DEFAULT_BODY_FONT_SIZE__)))
                .foregroundStyle((disabled || loading) ? tint.opacity(0.38) : tint)
        } else {
            labelContent
        }
    }

    private func decorated<V: View>(_ view: V) -> some View {
        view
            .nexaSharedButtonShape(shape)
            .tint(tint ?? .blue)
            .modifier(NexaSharedGlassModifier(enabled: glass, tint: tint, shape: shape))
    }

    @ViewBuilder
    private func accessible<V: View>(_ view: V) -> some View {
        if let accessibilityLabel {
            view.accessibilityLabel(Text(accessibilityLabel))
        } else {
            view
        }
    }

    @ViewBuilder
    private func platformSize<V: View>(_ view: V) -> some View {
        switch size {
        case .small: decorated(view.controlSize(.small))
        case .regular: decorated(view.controlSize(.regular))
        case .large: decorated(view.controlSize(.large))
        case nil: decorated(view)
        }
    }

    @ViewBuilder
    var body: some View {
        switch style {
        case .bordered:
            let tint = tint ?? .blue
            let outline = NexaSharedButtonOutline(kind: shape)
            accessible(Button(action: action) { labelContent }
                .disabled(disabled || loading)
                .buttonStyle(NexaSharedBorderedButtonStyle(tint: tint, shape: outline, size: size))
                .modifier(NexaSharedGlassModifier(enabled: glass, tint: self.tint, shape: shape)))
        case .borderedProminent:
            accessible(Button(action: action) { labelContent }
                .disabled(disabled || loading)
                .buttonStyle(NexaSharedProminentButtonStyle(
                    tint: tint ?? Color(uiColor: .systemBlue),
                    shape: NexaSharedButtonOutline(kind: shape),
                    size: size,
                    iconOnlyCircle: iconOnlyCircle
                ))
                .modifier(NexaSharedGlassModifier(enabled: glass, tint: tint, shape: shape)))
        case .borderless:
            platformSize(
                accessible(Button(action: action) { plainLabelContent }
                    .disabled(disabled || loading)
                    .buttonStyle(.borderless)
                    .frame(minWidth: CGFloat(__NEXA_BUTTON_MIN_WIDTH__), minHeight: size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__)))
            )
        case .plain:
            platformSize(
                accessible(Button(action: action) { plainLabelContent }
                    .disabled(disabled || loading)
                    .buttonStyle(.plain)
                    .frame(minWidth: CGFloat(__NEXA_BUTTON_MIN_WIDTH__), minHeight: size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__)))
            )
        }
    }
}

// nexa-codegen: NexaSliderPrimitive
struct NexaSliderPrimitive: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Binding var value: Double
    let minValue: Double
    let maxValue: Double
    let step: Double
    let tint: Color?
    let trackColor: Color?
    let thumbColor: Color?
    let trackHeight: CGFloat?
    let thumbSize: CGFloat?
    let hitSlop: CGFloat?
    let onChange: ((Double) -> Void)?
    let onEditingChanged: ((Bool) -> Void)?
    @State private var isEditing = false

    private var safeThumbSize: CGFloat { max(thumbSize ?? 14, 1) }
    private var safeTrackHeight: CGFloat { max(trackHeight ?? 4, 1) }
    private var lowerBound: Double { minValue.isFinite ? minValue : 0 }
    private var upperBound: Double {
        maxValue.isFinite && maxValue > lowerBound ? maxValue : lowerBound + 1
    }
    private var normalizedValue: CGFloat {
        guard value.isFinite else { return 0 }
        return CGFloat(min(max((value - lowerBound) / (upperBound - lowerBound), 0), 1))
    }

    var body: some View {
        GeometryReader { geometry in
            let thumb = safeThumbSize
            let width = geometry.size.width
            ZStack(alignment: .leading) {
                ZStack(alignment: .leading) {
                    Capsule()
                        .fill(trackColor ?? Color.gray.opacity(0.28))
                    Rectangle()
                        .fill(tint ?? Color.accentColor)
                        .frame(width: width * normalizedValue)
                }
                .frame(width: width, height: safeTrackHeight)
                .clipShape(Capsule())
                Circle()
                    .fill(thumbColor ?? Color.white)
                    .frame(width: thumb, height: thumb)
                    .offset(x: width * normalizedValue - thumb / 2)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
            .contentShape(Rectangle().inset(by: -max(hitSlop ?? 0, 0)))
            .allowsHitTesting(minValue.isFinite && maxValue.isFinite && maxValue > minValue)
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { gesture in
                        if !isEditing && abs(gesture.translation.width) > 3 {
                            isEditing = true
                            onEditingChanged?(true)
                        }
                        if isEditing {
                            update(gesture.location.x, width: width)
                        }
                    }
                    .onEnded { gesture in
                        if isEditing {
                            update(gesture.location.x, width: width)
                            isEditing = false
                            onEditingChanged?(false)
                        } else {
                            update(gesture.location.x, width: width)
                        }
                    }
            )
        }
        .frame(height: safeThumbSize)
        .animation(reduceMotion || isEditing ? nil : .linear(duration: 0.25), value: value)
    }

    private func update(_ locationX: CGFloat, width: CGFloat) {
        let fraction = width > 0
            ? min(max(locationX / width, 0), 1)
            : 0
        let nextValue = lowerBound + Double(fraction) * (upperBound - lowerBound)
        let quantizedValue = step > 0 ? lowerBound + ((nextValue - lowerBound) / step).rounded() * step : nextValue
        value = min(max(quantizedValue, lowerBound), upperBound)
        onChange?(value)
    }
}

// nexa-codegen: NexaProgressBarPrimitive
struct NexaProgressBarPrimitive: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isVisible = false

    let progress: Double?
    let tint: Color?
    let trackColor: Color?
    let thickness: CGFloat?
    let cornerRadius: CGFloat?

    private var normalizedProgress: Double? {
        guard let progress else { return nil }
        guard progress.isFinite else { return 0 }
        return min(max(progress, 0), 1)
    }

    @ViewBuilder var body: some View {
        if let normalizedProgress {
            if trackColor == nil && thickness == nil && cornerRadius == nil {
                if let tint {
                    ProgressView(value: normalizedProgress, total: 1).tint(tint)
                } else {
                    ProgressView(value: normalizedProgress, total: 1)
                }
            } else {
                let barHeight = thickness ?? 4
                let radius = cornerRadius ?? barHeight / 2
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        RoundedRectangle(cornerRadius: radius)
                            .fill(trackColor ?? Color.primary.opacity(0.1))
                        RoundedRectangle(cornerRadius: radius)
                            .fill(tint ?? Color.accentColor)
                            .frame(width: max(geometry.size.width * CGFloat(normalizedProgress), 0))
                    }
                }
                .frame(height: barHeight)
                .animation(
                    cornerRadius == 0 && !reduceMotion ? .linear(duration: 0.25) : nil,
                    value: normalizedProgress,
                )
                .accessibilityValue(Text("\(Int((normalizedProgress * 100).rounded()))%"))
            }
        } else {
            if let tint {
                indeterminateBar(tint: tint)
            } else {
                indeterminateBar(tint: Color.accentColor)
            }
        }
    }

    private func indeterminateBar(tint: Color) -> some View {
        let barHeight = thickness ?? 4
        let radius = cornerRadius ?? barHeight / 2
        return GeometryReader { geometry in
            let segmentWidth = min(max(geometry.size.width * 0.25, barHeight * 2), geometry.size.width)
            let shouldAnimate = isVisible && !reduceMotion
            ZStack(alignment: .leading) {
                RoundedRectangle(cornerRadius: radius)
                    .fill(trackColor ?? Color.primary.opacity(0.1))
                RoundedRectangle(cornerRadius: radius)
                    .fill(tint)
                    .frame(width: segmentWidth)
                    .offset(x: reduceMotion
                        ? (geometry.size.width - segmentWidth) / 2
                        : isVisible ? geometry.size.width : -segmentWidth)
            }
            .clipShape(RoundedRectangle(cornerRadius: radius))
            .animation(
                reduceMotion ? nil : .linear(duration: 1.2).repeatForever(autoreverses: false),
                value: shouldAnimate,
            )
            .onAppear { isVisible = true }
            .onDisappear { isVisible = false }
            .accessibilityRepresentation { ProgressView() }
        }
        .frame(height: barHeight)
    }
}

// nexa-codegen: NexaProgressRingPrimitive
struct NexaProgressRingPrimitive: View {
    let progress: Double?
    let tint: Color?
    let trackColor: Color?
    let size: CGFloat?
    let thickness: CGFloat?

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isAnimating = false

    private var normalizedProgress: Double? {
        guard let progress else { return nil }
        guard progress.isFinite else { return 0 }
        return min(max(progress, 0), 1)
    }

    private var indicatorColor: Color { tint ?? Color.accentColor }
    private var indicatorSize: CGFloat { max(size ?? 40, 0) }
    private var indicatorThickness: CGFloat { min(max(thickness ?? 4, 0), indicatorSize) }
    private var pathSize: CGFloat { max(indicatorSize - indicatorThickness, 0) }

    var body: some View {
        ZStack {
            Circle()
                .stroke(trackColor ?? indicatorColor.opacity(0.16), lineWidth: indicatorThickness)

            if let normalizedProgress {
                if normalizedProgress >= 1 {
                    Circle()
                        .stroke(indicatorColor, lineWidth: indicatorThickness)
                } else if normalizedProgress > 0 {
                    Circle()
                        .trim(from: 0, to: normalizedProgress)
                        .stroke(
                            indicatorColor,
                            style: StrokeStyle(lineWidth: indicatorThickness, lineCap: .round)
                        )
                        .rotationEffect(.degrees(-90))
                }
            } else {
                Circle()
                    .trim(from: 0, to: 0.25)
                    .stroke(
                        indicatorColor,
                        style: StrokeStyle(lineWidth: indicatorThickness, lineCap: .round)
                    )
                    .rotationEffect(.degrees(isAnimating ? 270 : -90))
            }
        }
        .frame(width: pathSize, height: pathSize)
        .frame(width: indicatorSize, height: indicatorSize)
        .animation(.easeInOut(duration: 0.2), value: normalizedProgress)
        .animation(
            reduceMotion ? nil : .linear(duration: 1).repeatForever(autoreverses: false),
            value: isAnimating,
        )
        .onAppear { isAnimating = !reduceMotion }
        .onDisappear { isAnimating = false }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Progress")
        .accessibilityValue(normalizedProgress.map { "\(Int(($0 * 100).rounded()))%" } ?? "In Progress")
    }
}

// nexa-codegen: NexaStatusBarBackgroundPrimitive
struct NexaStatusBarBackgroundPrimitive: View {
    let color: Color
    @Environment(\.nexaVisualSafeAreaInsets) private var safeInsets

    var body: some View {
        color
            .frame(maxWidth: .infinity)
            .frame(height: max(0, safeInsets.top))
            .ignoresSafeArea(edges: .top)
            .allowsHitTesting(false)
    }
}

// nexa-codegen: NexaStatusBarStylePrimitive
struct NexaStatusBarStylePrimitive: ViewModifier {
    let colorScheme: ColorScheme?

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 27.0, *), let colorScheme {
            content.toolbarColorScheme(colorScheme, for: .statusBar)
        } else {
            // Older SwiftUI releases have no status-bar-only color-scheme API.
            // Preserve the app's interface appearance instead of changing the
            // entire presentation to recolor status-bar items.
            content
        }
    }
}

// nexa-codegen: NexaDividerPrimitive
struct NexaDividerPrimitive: View {
    let color: Color
    let thickness: CGFloat

    var body: some View {
        Rectangle()
            .fill(color)
            .frame(maxWidth: .infinity)
            .frame(height: max(0, thickness))
    }
}

// nexa-codegen: NexaSegmentedControlPrimitive
@MainActor
private struct NexaNativeSegmentedControl: UIViewRepresentable {
    let items: [String]
    @Binding var selection: String
    let tint: Color?
    let background: Color?
    let textColor: Color?
    let selectedTextColor: Color?
    let fontSize: CGFloat?
    let fontWeight: Font.Weight?
    let height: CGFloat?
    let cornerRadius: CGFloat?
    let borderColor: Color?
    let borderWidth: CGFloat?

    func makeCoordinator() -> Coordinator {
        Coordinator(selection: $selection, items: items)
    }

    func makeUIView(context: Context) -> UISegmentedControl {
        let control = UISegmentedControl(items: items)
        control.addTarget(
            context.coordinator,
            action: #selector(Coordinator.selectionChanged(_:)),
            for: .valueChanged
        )
        control.setContentHuggingPriority(.defaultLow, for: .horizontal)
        control.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return control
    }

    func updateUIView(_ control: UISegmentedControl, context: Context) {
        context.coordinator.selection = $selection
        context.coordinator.items = items

        let displayedItems = (0..<control.numberOfSegments).compactMap {
            control.titleForSegment(at: $0)
        }
        if displayedItems != items {
            control.removeAllSegments()
            for (index, item) in items.enumerated() {
                control.insertSegment(withTitle: item, at: index, animated: false)
            }
        }

        control.selectedSegmentIndex = items.firstIndex(of: selection) ?? UISegmentedControl.noSegment

        if let background {
            let color = UIColor(background)
            control.backgroundColor = color
            control.layer.backgroundColor = color.cgColor
        } else {
            control.backgroundColor = nil
            control.layer.backgroundColor = nil
        }

        // UIKit owns the selected capsule in every style. Its iOS 26 material may blend an
        // explicit track background, while tint and title attributes remain directly configurable.
        control.selectedSegmentTintColor = tint.map { UIColor($0) }
            ?? (background == nil ? nil : UIColor.systemBackground)

        let font = (fontSize != nil || fontWeight != nil)
            ? UIFont.systemFont(ofSize: fontSize ?? UIFont.systemFontSize, weight: Self.uiFontWeight(fontWeight))
            : nil
        var normalAttributes: [NSAttributedString.Key: Any] = [:]
        if let textColor { normalAttributes[.foregroundColor] = UIColor(textColor) }
        if let font { normalAttributes[.font] = font }
        control.setTitleTextAttributes(normalAttributes.isEmpty ? nil : normalAttributes, for: .normal)

        var selectedAttributes: [NSAttributedString.Key: Any] = [:]
        if let selectedTextColor { selectedAttributes[.foregroundColor] = UIColor(selectedTextColor) }
        if let font { selectedAttributes[.font] = font }
        control.setTitleTextAttributes(selectedAttributes.isEmpty ? nil : selectedAttributes, for: .selected)

        control.layer.cornerRadius = cornerRadius ?? (background == nil ? 0 : max(32, height ?? 32) / 2)
        control.layer.borderColor = borderColor.map { UIColor($0).cgColor }
        control.layer.borderWidth = max(0, borderWidth ?? (borderColor == nil ? 0 : 1))
        control.layer.masksToBounds = background != nil || borderColor != nil || (borderWidth ?? 0) > 0
    }

    private static func uiFontWeight(_ weight: Font.Weight?) -> UIFont.Weight {
        guard let weight else { return .regular }
        if weight == .ultraLight { return .ultraLight }
        if weight == .thin { return .thin }
        if weight == .light { return .light }
        if weight == .medium { return .medium }
        if weight == .semibold { return .semibold }
        if weight == .bold { return .bold }
        if weight == .heavy { return .heavy }
        if weight == .black { return .black }
        return .regular
    }

    @MainActor
    final class Coordinator: NSObject {
        var selection: Binding<String>
        var items: [String]

        init(selection: Binding<String>, items: [String]) {
            self.selection = selection
            self.items = items
        }

        @objc func selectionChanged(_ sender: UISegmentedControl) {
            guard items.indices.contains(sender.selectedSegmentIndex) else { return }
            selection.wrappedValue = items[sender.selectedSegmentIndex]
        }
    }
}

struct NexaSegmentedControlPrimitive: View {
    let items: [String]
    @Binding var selection: String
    var animation: Animation? = nil
    var tint: Color? = nil
    var background: Color? = nil
    var textColor: Color? = nil
    var selectedTextColor: Color? = nil
    var fontSize: CGFloat? = nil
    var fontWeight: Font.Weight? = nil
    var height: CGFloat? = nil
    var cornerRadius: CGFloat? = nil
    var borderColor: Color? = nil
    var borderWidth: CGFloat? = nil
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ViewBuilder
    var body: some View {
        if items.isEmpty {
            EmptyView()
        } else {
            NexaNativeSegmentedControl(
                items: items,
                selection: $selection,
                tint: tint,
                background: background,
                textColor: textColor,
                selectedTextColor: selectedTextColor,
                fontSize: fontSize,
                fontWeight: fontWeight,
                height: height,
                cornerRadius: cornerRadius,
                borderColor: borderColor,
                borderWidth: borderWidth
            )
            .frame(maxWidth: .infinity)
            .frame(height: max(32, height ?? 32))
            .transaction { transaction in
                if reduceMotion {
                    transaction.animation = nil
                    transaction.disablesAnimations = true
                } else if let animation {
                    transaction.animation = animation
                }
            }
        }
    }
}

// nexa-codegen: NexaSystemIconPrimitive
struct NexaSystemIconPrimitive: View {
    let symbol: String
    let description: String
    let size: CGFloat
    let tint: Color

    @ViewBuilder var body: some View {
        let icon = Image(systemName: symbol)
            .font(.system(size: size))
            .foregroundStyle(tint)
        if description.isEmpty {
            icon.accessibilityHidden(true)
        } else {
            icon.accessibilityLabel(description)
        }
    }
}

// nexa-codegen: NexaIconSpec
struct NexaIconSpec {
    let symbol: String?
    let glyph: Int32?
    let fontFamily: String?

    init(symbol: String) {
        self.symbol = symbol
        self.glyph = nil
        self.fontFamily = nil
    }

    init(glyph: Int32, fontFamily: String) {
        self.symbol = nil
        self.glyph = glyph
        self.fontFamily = fontFamily
    }
}

// nexa-codegen: NexaIconView
struct NexaIconView: View {
    let icon: NexaIconSpec
    let description: String
    let size: CGFloat
    var tint: Color? = nil

    @ViewBuilder var body: some View {
        if let glyph = icon.glyph, let fontFamily = icon.fontFamily {
            let glyphText = UnicodeScalar(Int(glyph)).map { String($0) } ?? "�"
            let text = Text(glyphText).font(.custom(fontFamily, size: size))
            if let tint {
                accessible(text.foregroundStyle(tint))
            } else {
                accessible(text)
            }
        } else if let symbol = icon.symbol {
            let image = Image(systemName: symbol).font(.system(size: size))
            if let tint {
                accessible(image.foregroundStyle(tint))
            } else {
                accessible(image)
            }
        } else {
            EmptyView()
        }
    }

    @ViewBuilder
    private func accessible<Content: View>(_ content: Content) -> some View {
        if description.isEmpty {
            content.accessibilityHidden(true)
        } else {
            content.accessibilityLabel(description)
        }
    }
}

// nexa-codegen: NexaFontIconPrimitive
struct NexaFontIconPrimitive: View {
    let glyph: Int32
    let fontFamily: String
    let description: String
    let size: CGFloat
    let tint: Color

    @ViewBuilder var body: some View {
        let glyphText = UnicodeScalar(Int(glyph)).map { String($0) } ?? "�"
        let icon = Text(glyphText)
            .font(.custom(fontFamily, size: size))
            .foregroundStyle(tint)
        if description.isEmpty {
            icon.accessibilityHidden(true)
        } else {
            icon.accessibilityLabel(description)
        }
    }
}

// nexa-codegen: NexaLinearGradientPrimitive
struct NexaLinearGradientPrimitive: View {
    let colors: [Color]
    let startPoint: UnitPoint
    let endPoint: UnitPoint
    let height: CGFloat

    var body: some View {
        LinearGradient(colors: colors, startPoint: startPoint, endPoint: endPoint)
            .frame(maxWidth: .infinity)
            .frame(height: height)
            .allowsHitTesting(false)
    }
}

// nexa-codegen: NexaContentUnavailablePrimitive
struct NexaContentUnavailablePrimitive: View {
    let title: Text
    let icon: NexaIconSpec
    let description: Text?
    let titleColor: Color?
    let descriptionColor: Color?
    let iconTint: Color?
    let background: Color?

    @ViewBuilder private var iconContent: some View {
        if let glyph = icon.glyph, let fontFamily = icon.fontFamily {
            Text(UnicodeScalar(Int(glyph)).map { String($0) } ?? "�")
                .font(.custom(fontFamily, size: CGFloat(58)))
        } else if let symbol = icon.symbol {
            Image(systemName: symbol)
        } else {
            EmptyView()
        }
    }

    @ViewBuilder var body: some View {
        if #available(iOS 17.0, *) {
            ContentUnavailableView {
                Label {
                    if let titleColor { title.foregroundStyle(titleColor) } else { title }
                } icon: {
                    if let iconTint {
                        iconContent.foregroundStyle(iconTint)
                    } else {
                        iconContent
                    }
                }
            } description: {
                if let description {
                    if let descriptionColor {
                        description.foregroundStyle(descriptionColor)
                    } else {
                        description
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background { if let background { background } }
        } else {
            VStack(spacing: 8) {
                Label {
                    if let titleColor { title.foregroundStyle(titleColor) } else { title }
                } icon: {
                    if let iconTint {
                        iconContent.foregroundStyle(iconTint)
                    } else {
                        iconContent
                    }
                }
                    .font(.title3)
                if let description {
                    if let descriptionColor {
                        description.foregroundStyle(descriptionColor)
                            .multilineTextAlignment(.center)
                    } else {
                        description.foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(32)
            .background { if let background { background } }
        }
    }
}

// nexa-codegen: NexaColumnPrimitive
struct NexaColumnPrimitive<Content: View>: View {
    let alignment: HorizontalAlignment
    let textAlignment: TextAlignment
    let spacing: CGFloat
    private let content: Content
    @Environment(\.nexaSafeAreaRootFillsWidth) private var fillsSafeAreaRootWidth

    init(
        alignment: HorizontalAlignment = .center,
        textAlignment: TextAlignment = .center,
        spacing: CGFloat = 0,
        @ViewBuilder content: () -> Content
    ) {
        self.alignment = alignment
        self.textAlignment = textAlignment
        self.spacing = spacing
        self.content = content()
    }

    var body: some View {
        VStack(alignment: alignment, spacing: spacing) { content }
            .multilineTextAlignment(textAlignment)
            .environment(\.nexaSafeAreaRootFillsWidth, false)
            .frame(maxWidth: fillsSafeAreaRootWidth ? .infinity : nil)
    }
}

// nexa-codegen: NexaRowPrimitive
enum NexaRowArrangement {
    case start
    case center
    case end

    var frameAlignment: Alignment {
        switch self {
        case .start: .leading
        case .center: .center
        case .end: .trailing
        }
    }
}

struct NexaRowPrimitive<Content: View>: View {
    let alignment: VerticalAlignment
    let spacing: CGFloat
    let horizontalArrangement: NexaRowArrangement?
    private let content: Content
    @Environment(\.nexaSafeAreaRootFillsWidth) private var fillsSafeAreaRootWidth

    init(
        alignment: VerticalAlignment = .center,
        spacing: CGFloat = 0,
        horizontalArrangement: NexaRowArrangement? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.alignment = alignment
        self.spacing = spacing
        self.horizontalArrangement = horizontalArrangement
        self.content = content()
    }

    var body: some View {
        if let horizontalArrangement {
            HStack(alignment: alignment, spacing: spacing) { content }
                .frame(maxWidth: .infinity, alignment: horizontalArrangement.frameAlignment)
                .multilineTextAlignment(.leading)
                .environment(\.nexaSafeAreaRootFillsWidth, false)
        } else {
            HStack(alignment: alignment, spacing: spacing) { content }
                .multilineTextAlignment(.leading)
                .environment(\.nexaSafeAreaRootFillsWidth, false)
                .frame(maxWidth: fillsSafeAreaRootWidth ? .infinity : nil, alignment: .leading)
        }
    }
}

// nexa-codegen: NexaStackPrimitive
struct NexaStackPrimitive<Content: View>: View {
    let alignment: Alignment
    private let content: Content
    @Environment(\.nexaSafeAreaRootFillsWidth) private var fillsSafeAreaRootWidth

    init(alignment: Alignment = .center, @ViewBuilder content: () -> Content) {
        self.alignment = alignment
        self.content = content()
    }

    var body: some View {
        ZStack(alignment: alignment) { content }
            .environment(\.nexaSafeAreaRootFillsWidth, false)
            .frame(maxWidth: fillsSafeAreaRootWidth ? .infinity : nil, alignment: alignment)
    }
}

// nexa-codegen: NexaAnimateContentSizeModifier
struct NexaAnimateContentSizeModifier: ViewModifier {
    let animation: Animation

    @State private var contentSize: CGSize?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            // Keep the child at its final intrinsic size. Only the outer frame
            // interpolates, so the Stack reveals/clips content like Compose's
            // animateContentSize instead of moving or fading the child itself.
            .transaction { transaction in
                transaction.animation = nil
            }
            .fixedSize(horizontal: false, vertical: true)
            .background {
                GeometryReader { geometry in
                    Color.clear
                        .onAppear { updateContentSize(geometry.size) }
                        .onChange(of: geometry.size, perform: updateContentSize)
                }
            }
            .frame(width: contentSize?.width, height: contentSize?.height, alignment: .topLeading)
            .clipped()
    }

    private func updateContentSize(_ newSize: CGSize) {
        guard newSize.width.isFinite, newSize.height.isFinite else {
            return
        }

        guard let contentSize else {
            var transaction = Transaction()
            transaction.disablesAnimations = true
            withTransaction(transaction) {
                self.contentSize = newSize
            }
            return
        }

        guard abs(newSize.width - contentSize.width) > 0.5
            || abs(newSize.height - contentSize.height) > 0.5 else {
            return
        }
        if reduceMotion {
            self.contentSize = newSize
        } else {
            withAnimation(animation) {
                self.contentSize = newSize
            }
        }
    }
}

// Keep Stack's source order explicit in both drawing and hit testing. This
// matters when an earlier child hosts UIKit content (for example FastList)
// that otherwise captures touches intended for a later overlay child.
// nexa-codegen: NexaStackLayer
struct NexaStackLayer<Content: View>: View {
    let order: Double
    private let content: Content

    init(order: Int, @ViewBuilder content: () -> Content) {
        self.order = Double(order)
        self.content = content()
    }

    var body: some View {
        content.zIndex(order)
    }
}

// nexa-codegen: NexaNativeListPrimitive
@MainActor
private final class NexaFastListReorderRowState: ObservableObject {
    @Published fileprivate(set) var isDragging = false
    @Published fileprivate(set) var dragOffset: CGFloat = 0
}

@MainActor
private final class NexaFastListReorderSession {
    private(set) var activeIndex: Int?
    private var rowFrames: [Int: CGRect] = [:]
    private var rowStates: [Int: NexaFastListReorderRowState] = [:]
    private var startY: CGFloat = 0
    private var accumulatedLayoutShift: CGFloat = 0
    private var lastInsertionIndex: Int?

    func rowState(at index: Int) -> NexaFastListReorderRowState {
        if let state = rowStates[index] {
            return state
        }
        let state = NexaFastListReorderRowState()
        rowStates[index] = state
        return state
    }

    func updateFrames(_ frames: [Int: CGRect]) {
        rowFrames = frames
        rowStates = rowStates.filter { frames[$0.key] != nil || $0.key == activeIndex }
    }

    func begin(at index: Int, startY: CGFloat) {
        activeIndex = index
        self.startY = startY
        accumulatedLayoutShift = 0
        lastInsertionIndex = nil
        let state = rowState(at: index)
        state.isDragging = true
        state.dragOffset = 0
    }

    func update(to location: CGPoint, perform onMove: (Int, Int) -> Void) {
        guard let source = activeIndex,
              let sourceFrame = rowFrames[source],
              let sourceState = rowStates[source],
              let target = rowFrames.min(by: {
                  abs($0.value.midY - location.y) < abs($1.value.midY - location.y)
              })
        else { return }

        let insertionIndex = target.key + (location.y >= target.value.midY ? 1 : 0)
        guard lastInsertionIndex != insertionIndex else {
            sourceState.dragOffset = location.y - startY - accumulatedLayoutShift
            return
        }
        lastInsertionIndex = insertionIndex

        let destination = insertionIndex > source ? insertionIndex - 1 : insertionIndex
        guard destination != source else {
            sourceState.dragOffset = location.y - startY - accumulatedLayoutShift
            return
        }

        let targetCenter = rowFrames[destination]?.midY ?? target.value.midY
        accumulatedLayoutShift += targetCenter - sourceFrame.midY
        withAnimation(.default) {
            onMove(source, insertionIndex)
        }
        sourceState.isDragging = false
        sourceState.dragOffset = 0
        activeIndex = destination
        let destinationState = rowState(at: destination)
        destinationState.isDragging = true
        destinationState.dragOffset = location.y - startY - accumulatedLayoutShift
    }

    func finish() {
        if let activeIndex, let state = rowStates[activeIndex] {
            state.isDragging = false
            state.dragOffset = 0
        }
        activeIndex = nil
        accumulatedLayoutShift = 0
        lastInsertionIndex = nil
    }
}

private struct NexaFastListRowFramesKey: PreferenceKey {
    static let defaultValue: [Int: CGRect] = [:]

    static func reduce(value: inout [Int: CGRect], nextValue: () -> [Int: CGRect]) {
        value.merge(nextValue(), uniquingKeysWith: { _, next in next })
    }
}

@MainActor
private struct NexaFastListReorderableRow<Content: View>: View {
    let index: Int
    let onReorder: (Int, Int) -> Void
    private let content: Content
    private let session: NexaFastListReorderSession
    @ObservedObject private var state: NexaFastListReorderRowState

    init(
        index: Int,
        session: NexaFastListReorderSession,
        onReorder: @escaping (Int, Int) -> Void,
        @ViewBuilder content: () -> Content
    ) {
        self.index = index
        self.onReorder = onReorder
        self.session = session
        self._state = ObservedObject(wrappedValue: session.rowState(at: index))
        self.content = content()
    }

    var body: some View {
        let isDragging = state.isDragging
        ZStack {
            content.offset(y: isDragging ? state.dragOffset : 0)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background {
            GeometryReader { geometry in
                Color.clear.preference(
                    key: NexaFastListRowFramesKey.self,
                    value: [index: geometry.frame(in: .global)]
                )
            }
        }
        .contentShape(Rectangle())
        .zIndex(isDragging ? 1 : 0)
        .highPriorityGesture(
            LongPressGesture(minimumDuration: 0.35)
                .sequenced(before: DragGesture(minimumDistance: 0, coordinateSpace: .global))
                .onChanged { value in
                    guard case let .second(true, drag?) = value else { return }
                    if session.activeIndex == nil {
                        session.begin(at: index, startY: drag.startLocation.y)
                    }
                    session.update(to: drag.location, perform: onReorder)
                }
                .onEnded { _ in session.finish() }
        )
    }
}

private struct NexaFastListReorderSessionKey: EnvironmentKey {
    static let defaultValue: NexaFastListReorderSession? = nil
}

private extension EnvironmentValues {
    var nexaFastListReorderSession: NexaFastListReorderSession? {
        get { self[NexaFastListReorderSessionKey.self] }
        set { self[NexaFastListReorderSessionKey.self] = newValue }
    }
}

private struct NexaNativeListComponentRefKey: EnvironmentKey {
    static let defaultValue: NexaFastListRef? = nil
}

private extension EnvironmentValues {
    var nexaNativeListComponentRef: NexaFastListRef? {
        get { self[NexaNativeListComponentRefKey.self] }
        set { self[NexaNativeListComponentRefKey.self] = newValue }
    }
}

@MainActor
private struct NexaNativeListRefScrollViewProbe: UIViewRepresentable {
    let reference: NexaFastListRef

    func makeUIView(context: Context) -> NexaNativeListRefProbeView {
        let view = NexaNativeListRefProbeView()
        view.reference = reference
        return view
    }

    func updateUIView(_ view: NexaNativeListRefProbeView, context: Context) {
        view.reference = reference
        view.connectToContainingList()
    }
}

@MainActor
private final class NexaNativeListRefProbeView: UIView {
    weak var reference: NexaFastListRef?
    private var connectionCheckPending = false

    override func didMoveToWindow() {
        super.didMoveToWindow()
        connectToContainingList()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        connectToContainingList()
    }

    func connectToContainingList() {
        guard !connectionCheckPending else { return }
        connectionCheckPending = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.connectionCheckPending = false
            guard self.window != nil else { return }
            var ancestor = self.superview
            while let view = ancestor {
                if let scrollView = view as? UIScrollView {
                    self.reference?.updateNativeScrollView(scrollView)
                    return
                }
                ancestor = view.superview
            }
        }
    }
}

@MainActor
struct NexaNativeListPrimitive<Content: View>: View {
    private let spacing: CGFloat
    private let scrollEnabled: Bool
    private let rowCount: Int
    private let componentRef: NexaFastListRef?
    private let content: Content
    @Environment(\.nexaScrollContentPadding) private var contentPadding
    @State private var reorderSession: NexaFastListReorderSession?

    init(
        spacing: CGFloat = 0,
        scrollEnabled: Bool = true,
        reorderEnabled: Bool = false,
        rowCount: Int = 0,
        componentRef: NexaFastListRef? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.spacing = spacing
        self.scrollEnabled = scrollEnabled
        self.rowCount = rowCount
        self.componentRef = componentRef
        _reorderSession = State(
            initialValue: reorderEnabled ? NexaFastListReorderSession() : nil
        )
        self.content = content()
    }

    var body: some View {
        ScrollViewReader { proxy in
            List {
                if #unavailable(iOS 17.0) {
                    Color.clear.frame(height: CGFloat(__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__) + contentPadding.top)
                        .listRowInsets(EdgeInsets()).listRowSeparator(.hidden)
                        .listRowBackground(Color.clear).accessibilityHidden(true)
                }
                content
                    .environment(\.nexaScrollContentPadding, EdgeInsets())
                    .environment(\.nexaFastListReorderSession, reorderSession)
                    .environment(\.nexaNativeListComponentRef, componentRef)
                if #unavailable(iOS 17.0) {
                    Color.clear.frame(height: CGFloat(__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__) + contentPadding.bottom)
                        .listRowInsets(EdgeInsets()).listRowSeparator(.hidden)
                        .listRowBackground(Color.clear).accessibilityHidden(true)
                }
            }
            .onAppear {
                guard let componentRef else { return }
                componentRef.attach(
                    view: componentRef,
                    scrollToIndex: { index, animated, viewPosition in
                        guard rowCount > 0 else { return }
                        let anchor = UnitPoint(x: 0.5, y: CGFloat(viewPosition))
                        let scroll = {
                            proxy.scrollTo(Int(min(max(index, 0), Int32(rowCount - 1))), anchor: anchor)
                        }
                        if animated { withAnimation { scroll() } } else { scroll() }
                    },
                    scrollToTop: { animated in
                        guard rowCount > 0 else { return }
                        if animated { withAnimation { proxy.scrollTo(0, anchor: .top) } }
                        else { proxy.scrollTo(0, anchor: .top) }
                    },
                    scrollToEnd: { animated in
                        guard rowCount > 0 else { return }
                        if animated { withAnimation { proxy.scrollTo(rowCount - 1, anchor: .bottom) } }
                        else { proxy.scrollTo(rowCount - 1, anchor: .bottom) }
                    },
                    scrollToOffset: { [weak componentRef] offset, animated in
                        guard offset.isFinite,
                              let scrollView = componentRef?.nativeScrollViewObject() as? UIScrollView
                        else { return }
                        let minimum = -scrollView.adjustedContentInset.top
                        let maximum = max(
                            minimum,
                            scrollView.contentSize.height - scrollView.bounds.height
                                + scrollView.adjustedContentInset.bottom
                        )
                        let target = min(max(minimum + CGFloat(max(0, offset)), minimum), maximum)
                        scrollView.setContentOffset(
                            CGPoint(x: scrollView.contentOffset.x, y: target),
                            animated: animated
                        )
                    }
                )
            }
            .onDisappear { componentRef?.detach() }
            .onPreferenceChange(NexaFastListRowFramesKey.self) { frames in
                reorderSession?.updateFrames(frames)
            }
            .listRowSpacing(spacing)
            .scrollDisabled(!scrollEnabled)
            .modifier(NexaNativeListStyleModifier())
        }
    }
}

// nexa-codegen: NexaNativeListRowPrimitive
@MainActor
private struct NexaFastListSwipePagerGestureBridge: UIViewRepresentable {
    func makeUIView(context: Context) -> NexaFastListSwipePagerGestureView {
        NexaFastListSwipePagerGestureView()
    }

    func updateUIView(_ view: NexaFastListSwipePagerGestureView, context: Context) {
        view.connectToContainingPager()
    }
}

@MainActor
private final class NexaFastListSwipePagerGestureView: UIView, UIGestureRecognizerDelegate {
    private weak var connectedRowCell: UIView?
    private weak var connectedPagerScrollView: UIScrollView?
    private weak var connectedListScrollView: UIScrollView?
    private var rowTouchRecognizer: UILongPressGestureRecognizer?
    private var connectionCheckPending = false

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil {
            disconnectFromContainingPager()
            return
        }
        connectToContainingPager()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        connectToContainingPager()
    }

    func connectToContainingPager() {
        guard !connectionCheckPending else { return }
        connectionCheckPending = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.connectionCheckPending = false
            guard self.window != nil else { return }

            var ancestor = self.superview
            var rowCell: UIView?
            var listScrollView: UIScrollView?
            var pagerScrollView: UIScrollView?
            while let view = ancestor {
                if rowCell == nil,
                   view is UITableViewCell || view is UICollectionViewCell {
                    rowCell = view
                }
                if let scrollView = view as? UIScrollView {
                    if listScrollView == nil {
                        listScrollView = scrollView
                    } else if isHorizontalPageContainer(scrollView) {
                        // SwiftUI's page TabView is backed by a horizontal UICollectionView.
                        // On current iOS versions it does not consistently set isPagingEnabled,
                        // even though its collection content spans the pages horizontally.
                        pagerScrollView = scrollView
                        break
                    }
                }
                ancestor = view.superview
            }
            guard let rowCell, let listScrollView, let pagerScrollView else { return }
            guard connectedRowCell !== rowCell
                    || connectedListScrollView !== listScrollView
                    || connectedPagerScrollView !== pagerScrollView
            else { return }
            disconnectFromContainingPager()

            // A zero-duration, non-cancelling recognizer claims the row touch
            // before SwiftUI's ancestor page TabView can start paging. It
            // recognizes simultaneously with the native List gestures, so
            // SwiftUI still owns and renders the row swipe action.
            let rowTouchRecognizer = UILongPressGestureRecognizer(
                target: self,
                action: #selector(rowTouchRecognized(_:))
            )
            rowTouchRecognizer.minimumPressDuration = 0
            rowTouchRecognizer.allowableMovement = .greatestFiniteMagnitude
            rowTouchRecognizer.cancelsTouchesInView = false
            rowTouchRecognizer.delaysTouchesBegan = false
            rowTouchRecognizer.delaysTouchesEnded = false
            rowTouchRecognizer.delegate = self
            rowCell.addGestureRecognizer(rowTouchRecognizer)
            pagerScrollView.panGestureRecognizer.require(toFail: rowTouchRecognizer)

            connectedRowCell = rowCell
            connectedListScrollView = listScrollView
            connectedPagerScrollView = pagerScrollView
            self.rowTouchRecognizer = rowTouchRecognizer
        }
    }

    private func isHorizontalPageContainer(_ scrollView: UIScrollView) -> Bool {
        return scrollView.isPagingEnabled
            || (scrollView.contentSize.width > scrollView.bounds.width + 1
                && scrollView.contentSize.height <= scrollView.bounds.height + 1)
    }

    func gestureRecognizer(
        _ gestureRecognizer: UIGestureRecognizer,
        shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer
    ) -> Bool {
        guard gestureRecognizer === rowTouchRecognizer || otherGestureRecognizer === rowTouchRecognizer else {
            return false
        }
        guard let rowCell = connectedRowCell else { return false }
        let peer = gestureRecognizer === rowTouchRecognizer ? otherGestureRecognizer : gestureRecognizer
        var view = peer.view
        while let current = view {
            if current === rowCell { return true }
            if current === connectedListScrollView { return true }
            view = current.superview
        }
        return false
    }

    @objc private func rowTouchRecognized(_ recognizer: UIGestureRecognizer) {
        // The recognizer exists only to establish the failure dependency with
        // the outer pager. Native List recognizers handle the actual gesture.
        guard recognizer.state == .ended
                || recognizer.state == .cancelled
                || recognizer.state == .failed
        else { return }
    }

    private func disconnectFromContainingPager() {
        if let rowCell = connectedRowCell, let rowTouchRecognizer {
            rowCell.removeGestureRecognizer(rowTouchRecognizer)
        }
        rowTouchRecognizer = nil
        connectedRowCell = nil
        connectedListScrollView = nil
        connectedPagerScrollView = nil
    }
}

struct NexaNativeListRowPrimitive<Content: View>: View {
    let position: Int
    let showDividers: Bool
    let prioritizeSwipeActions: Bool
    let reorderIndex: Int?
    let reorderEnabled: Bool
    let onReorder: ((Int, Int) -> Void)?
    private let content: Content
    @Environment(\.nexaFastListReorderSession) private var reorderSession
    @Environment(\.nexaNativeListComponentRef) private var componentRef

    init(
        position: Int,
        showDividers: Bool = true,
        prioritizeSwipeActions: Bool = false,
        reorderIndex: Int? = nil,
        reorderEnabled: Bool = false,
        onReorder: ((Int, Int) -> Void)? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.position = position
        self.showDividers = showDividers
        self.prioritizeSwipeActions = prioritizeSwipeActions
        self.reorderIndex = reorderIndex
        self.reorderEnabled = reorderEnabled
        self.onReorder = onReorder
        self.content = content()
    }

    var body: some View {
        let row = Group {
            if prioritizeSwipeActions {
                content.background {
                    NexaFastListSwipePagerGestureBridge()
                        .frame(width: 1, height: 1)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }
            } else {
                content
            }
        }
        let observedRow = Group {
            if let componentRef {
                row.background {
                    NexaNativeListRefScrollViewProbe(reference: componentRef)
                        .frame(width: 1, height: 1)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }
            } else {
                row
            }
        }.id(position)
        Group {
            if reorderEnabled, let reorderIndex, let onReorder, let reorderSession {
                NexaFastListReorderableRow(
                    index: reorderIndex,
                    session: reorderSession,
                    onReorder: onReorder
                ) {
                    observedRow
                }
            } else {
                observedRow
            }
        }
            .listRowInsets(EdgeInsets(
                top: CGFloat(__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__),
                leading: CGFloat(__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__),
                bottom: CGFloat(__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__),
                trailing: CGFloat(__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__)
            ))
            .listRowBackground(Color(uiColor: .secondarySystemGroupedBackground))
            .listRowSeparatorTint(Color(uiColor: .separator))
            .listRowSeparator(showDividers ? .visible : .hidden)
    }
}

// nexa-codegen: NexaNativeListStyleModifier
struct NexaNativeListStyleModifier: ViewModifier {

    @Environment(\.nexaScrollContentPadding) private var padding

    func body(content: Content) -> some View {
        let groupedList = NexaScrollViewport { content }
            .listStyle(.insetGrouped)
            .scrollContentBackground(.hidden)
            .background(Color(uiColor: .systemGroupedBackground))
            .ignoresSafeArea(.container, edges: .all)
        if #available(iOS 17.0, *) {
            groupedList.contentMargins(
                .horizontal,
                CGFloat(__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__),
                for: .scrollContent
            )
            .contentMargins(.leading, CGFloat(__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__) + padding.leading, for: .scrollContent)
            .contentMargins(.trailing, CGFloat(__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__) + padding.trailing, for: .scrollContent)
            .contentMargins(
                .vertical,
                EdgeInsets(
                    top: padding.top + CGFloat(__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__),
                    leading: 0,
                    bottom: padding.bottom + CGFloat(__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__),
                    trailing: 0
                ),
                for: .scrollContent
            )
        } else {
            // Before iOS 17, insetGrouped List has no public contentMargins API.
            // Expand only the container so its system inset cancels out; row
            // content keeps the explicit listRowInsets above.
            groupedList
                .padding(
                    .horizontal,
                    -CGFloat(__NEXA_IOS_LEGACY_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__)
                )

        }
    }
}

// nexa-codegen: NexaScrollViewport
private struct NexaScrollViewport<Content: View>: View {
    private let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        GeometryReader { proxy in
            content
                .ignoresSafeArea(.container, edges: .all)
                .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
        }
        .ignoresSafeArea(.container, edges: .all)
    }
}

// nexa-codegen: NexaScrollMarginViewport
private struct NexaScrollMarginViewport<Content: View>: View {
    let margin: EdgeInsets
    private let content: Content
    @Environment(\.self) private var environment

    init(margin: EdgeInsets, @ViewBuilder content: () -> Content) {
        self.margin = margin
        self.content = content()
    }

    var body: some View {
        GeometryReader { proxy in
            NexaScrollMarginHostedContent(
                content: NexaScrollViewport { content.environment(\.self, environment) }
            )
                .frame(
                    width: max(0, proxy.size.width - margin.leading - margin.trailing),
                    height: max(0, proxy.size.height - margin.top - margin.bottom),
                    alignment: .topLeading
                )
                .padding(margin)
                .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
        }
        .ignoresSafeArea(.container, edges: .all)
    }
}

private struct NexaScrollMarginHostedContent<Content: View>: UIViewControllerRepresentable {
    let content: Content

    func makeUIViewController(context: Context) -> NexaScrollMarginHostingController<Content> {
        NexaScrollMarginHostingController(rootView: content)
    }

    func updateUIViewController(
        _ controller: NexaScrollMarginHostingController<Content>,
        context: Context
    ) {
        controller.rootView = content
    }
}

private final class NexaScrollMarginHostingController<Content: View>: UIHostingController<Content> {
    override func viewDidLoad() {
        super.viewDidLoad()
        if #available(iOS 16.4, *) {
            safeAreaRegions.remove(.container)
        }
        view.clipsToBounds = true
        view.backgroundColor = .clear
    }
}

// nexa-codegen: NexaFormPrimitive
struct NexaFormPrimitive<Content: View>: View {
    let background: Color?
    let contentPadding: EdgeInsets
    let margin: EdgeInsets
    let firstChildIsSection: Bool
    private let content: Content
    @Environment(\.multilineTextAlignment) private var inheritedTextAlignment

    init(
        background: Color? = nil,
        contentPadding: EdgeInsets = .init(),
        margin: EdgeInsets = .init(),
        firstChildIsSection: Bool = false,
        @ViewBuilder content: () -> Content
    ) {
        self.background = background
        self.contentPadding = contentPadding
        self.margin = margin
        self.firstChildIsSection = firstChildIsSection
        self.content = content()
    }

    @Environment(\.nexaScrollContentPadding) private var inheritedPadding

    private var effectivePadding: EdgeInsets {
        EdgeInsets(top: contentPadding.top + inheritedPadding.top,
            leading: contentPadding.leading + inheritedPadding.leading,
            bottom: contentPadding.bottom + inheritedPadding.bottom,
            trailing: contentPadding.trailing + inheritedPadding.trailing)
    }

    var body: some View {
        let form = Form {
            if !firstChildIsSection, effectivePadding.top != 0 {
                Color.clear
                    .frame(height: effectivePadding.top)
                    .listRowInsets(EdgeInsets())
                    .listRowSeparator(.hidden)
                    .listRowBackground(Color.clear)
                    .accessibilityHidden(true)
            }
            content.environment(\.nexaScrollContentPadding, EdgeInsets())
            if effectivePadding.bottom != 0 {
                Color.clear
                    .frame(height: effectivePadding.bottom)
                    .listRowInsets(EdgeInsets())
                    .listRowSeparator(.hidden)
                    .listRowBackground(Color.clear)
                    .accessibilityHidden(true)
            }
        }
            .multilineTextAlignment(inheritedTextAlignment)
            .scrollContentBackground(.hidden)
            .background(background ?? Color.clear)
            .environment(\.nexaFormTopPadding, firstChildIsSection ? effectivePadding.top : 0)

        // Margin constrains and clips the Form viewport. Explicit padding remains
        // inside the full viewport and therefore scrolls with the Form's content.
        if margin.top == 0, margin.leading == 0, margin.bottom == 0, margin.trailing == 0 {
            NexaScrollViewport {
                configuredForm(form)
            }
        } else {
            NexaScrollMarginViewport(margin: margin) {
                configuredForm(form)
            }
        }
    }

    @ViewBuilder
    private func configuredForm<FormContent: View>(_ form: FormContent) -> some View {
        if #available(iOS 17.0, *) {
            form
                .contentMargins(.leading, effectivePadding.leading, for: .scrollContent)
                .contentMargins(.trailing, effectivePadding.trailing, for: .scrollContent)
        } else {
            form.listRowInsets(EdgeInsets(
                top: 0,
                leading: effectivePadding.leading,
                bottom: 0,
                trailing: effectivePadding.trailing
            ))
        }
    }
}

// nexa-codegen: NexaFormSectionPrimitive
struct NexaFormSectionPrimitive<Content: View, Header: View, Footer: View>: View {
    let background: Color?
    let isFirstSection: Bool
    private let content: Content
    private let header: Header
    private let footer: Footer
    @Environment(\.nexaFormTopPadding) private var formTopPadding

    init(
        background: Color? = nil,
        isFirstSection: Bool = false,
        @ViewBuilder content: () -> Content,
        @ViewBuilder header: () -> Header,
        @ViewBuilder footer: () -> Footer
    ) {
        self.background = background
        self.isFirstSection = isFirstSection
        self.content = content()
        self.header = header()
        self.footer = footer()
    }

    private var firstHeaderTopPadding: CGFloat {
        isFirstSection ? formTopPadding : 0
    }

    var body: some View {
        let section = Section {
            content
        } header: {
            header
                .foregroundStyle(.secondary)
                .padding(.top, firstHeaderTopPadding)
                .padding(.leading, -CGFloat(__NEXA_IOS_FORM_SECTION_HEADER_LEADING_CORRECTION__))
        } footer: {
            footer
        }

        section
            .listSectionSeparator(.hidden)
            .listRowBackground(background ?? Color.clear)
    }
}

// nexa-codegen: NexaFormRowPrimitive
struct NexaFormRowPrimitive<Content: View>: View {
    let minHeight: CGFloat
    let horizontalInset: CGFloat
    let showDividers: Bool
    private let content: Content

    init(
        minHeight: CGFloat,
        horizontalInset: CGFloat,
        showDividers: Bool = true,
        @ViewBuilder content: () -> Content
    ) {
        self.minHeight = minHeight
        self.horizontalInset = horizontalInset
        self.showDividers = showDividers
        self.content = content()
    }

    var body: some View {
        content
            .frame(maxWidth: .infinity, minHeight: minHeight, alignment: .leading)
            .listRowInsets(EdgeInsets(top: 0, leading: horizontalInset, bottom: 0, trailing: horizontalInset))
            .listRowSeparator(showDividers ? .visible : .hidden)
    }
}

// nexa-codegen: NexaAccessibilityPrimitive
struct NexaAccessibilityPrimitive<Content: View>: View {
    let label: Text
    let hint: Text?
    let value: Text?
    let role: String
    let omitLabel: Bool
    private let content: Content

    init(
        label: Text,
        hint: Text?,
        value: Text?,
        role: String,
        omitLabel: Bool,
        @ViewBuilder content: () -> Content
    ) {
        self.label = label
        self.hint = hint
        self.value = value
        self.role = role
        self.omitLabel = omitLabel
        self.content = content()
    }

    @ViewBuilder
    private func addingLabel<V: View>(to view: V) -> some View {
        if omitLabel { view } else { view.accessibilityLabel(label) }
    }

    @ViewBuilder
    private func addingHint<V: View>(to view: V) -> some View {
        if let hint { view.accessibilityHint(hint) } else { view }
    }

    @ViewBuilder
    private func addingValue<V: View>(to view: V) -> some View {
        if let value { view.accessibilityValue(value) } else { view }
    }

    @ViewBuilder
    private func addingRole<V: View>(to view: V) -> some View {
        switch role {
        case "Button": view.accessibilityAddTraits(.isButton)
        case "Link": view.accessibilityAddTraits(.isLink)
        case "Header": view.accessibilityAddTraits(.isHeader)
        case "Image": view.accessibilityAddTraits(.isImage)
        default: view
        }
    }

    var body: some View {
        addingRole(to: addingValue(to: addingHint(to: addingLabel(
            to: content.accessibilityElement(children: .combine)
        ))))
    }
}

// nexa-codegen: NexaAppearancePrimitive
struct NexaAppearancePrimitive<Content: View>: View {
    let mode: String
    private let content: Content
    @Environment(\.colorScheme) private var systemColorScheme

    init(mode: String, @ViewBuilder content: () -> Content) {
        self.mode = mode
        self.content = content()
    }

    private var resolvedColorScheme: ColorScheme {
        switch mode {
        case "dark": .dark
        case "light": .light
        default: systemColorScheme
        }
    }

    private var preferredColorScheme: ColorScheme? {
        switch mode {
        case "dark": .dark
        case "light": .light
        default: nil
        }
    }

    var body: some View {
        content
            .environment(\.colorScheme, resolvedColorScheme)
            .preferredColorScheme(preferredColorScheme)
    }
}

// nexa-codegen: NexaLinkPrimitive
struct NexaLinkPrimitive<Content: View>: View {
    let destination: URL?
    private let content: Content

    init(destination: URL?, @ViewBuilder content: () -> Content) {
        self.destination = destination
        self.content = content()
    }

    @ViewBuilder
    var body: some View {
        if let destination {
            Link(destination: destination) { content }
        } else {
            content
        }
    }
}

// nexa-codegen: NexaKeyboardAwarePrimitive
enum NexaKeyboardBehavior {
    case stay
    case avoid
}

struct NexaKeyboardAwarePrimitive<Content: View>: View {
    let behavior: NexaKeyboardBehavior
    let hasNativeScrollOwner: Bool
    let dismissMode: ScrollDismissesKeyboardMode
    private let content: Content

    init(
        behavior: NexaKeyboardBehavior,
        hasNativeScrollOwner: Bool,
        dismissMode: ScrollDismissesKeyboardMode,
        @ViewBuilder content: () -> Content
    ) {
        self.behavior = behavior
        self.hasNativeScrollOwner = hasNativeScrollOwner
        self.dismissMode = dismissMode
        self.content = content()
    }

    @ViewBuilder
    private var scrollOwner: some View {
        if hasNativeScrollOwner {
            content.scrollDismissesKeyboard(dismissMode)
        } else {
            ScrollView(.vertical) { NexaScrollContent { content } }
                .scrollDismissesKeyboard(dismissMode)
        }
    }

    @ViewBuilder
    var body: some View {
        switch behavior {
        case .stay:
            scrollOwner.ignoresSafeArea(.keyboard, edges: .bottom)
        case .avoid:
            scrollOwner
        }
    }
}

// nexa-codegen: NexaAdaptiveScrollHeaderPrimitive
enum NexaAdaptiveHeaderBehavior: Equatable {
    case pinned
    case collapse
    case hideOnScroll
}

private final class NexaAdaptiveScrollHeaderState: ObservableObject {
    @Published var collapseOffset: CGFloat = 0
    @Published var isHidden = false
    @Published var expandedHeight: CGFloat = 0
    private var lastOffset: CGFloat = 0
    private var accumulatedMovement: CGFloat = 0

    func update(offset: CGFloat, behavior: NexaAdaptiveHeaderBehavior) {
        let nextOffset = max(0, offset)
        let movement = nextOffset - lastOffset
        lastOffset = nextOffset

        switch behavior {
        case .pinned:
            return
        case .collapse:
            if abs(collapseOffset - nextOffset) >= 0.25 {
                collapseOffset = nextOffset
            }
        case .hideOnScroll:
            if nextOffset == 0 {
                accumulatedMovement = 0
                isHidden = false
                return
            }
            if accumulatedMovement * movement < 0 {
                accumulatedMovement = movement
            } else {
                accumulatedMovement += movement
            }
            guard abs(accumulatedMovement) >= 8 else { return }
            isHidden = accumulatedMovement > 0
            accumulatedMovement = 0
        }
    }
}

private struct NexaAdaptiveHeaderHeightPreferenceKey: PreferenceKey {
    static let defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

private struct NexaAdaptiveScrollHeaderView<Header: View>: View {
    @ObservedObject var state: NexaAdaptiveScrollHeaderState
    let behavior: NexaAdaptiveHeaderBehavior
    let collapsedHeight: CGFloat
    private let header: Header
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    init(
        state: NexaAdaptiveScrollHeaderState,
        behavior: NexaAdaptiveHeaderBehavior,
        collapsedHeight: CGFloat,
        @ViewBuilder header: () -> Header
    ) {
        self.state = state
        self.behavior = behavior
        self.collapsedHeight = collapsedHeight
        self.header = header()
    }

    private var displayedHeight: CGFloat? {
        switch behavior {
        case .pinned:
            return nil
        case .collapse:
            let expanded = max(state.expandedHeight, collapsedHeight)
            let progress = min(1, state.collapseOffset / 120)
            return expanded - (expanded - collapsedHeight) * progress
        case .hideOnScroll:
            return state.isHidden ? 0 : (state.expandedHeight > 0 ? state.expandedHeight : nil)
        }
    }

    var body: some View {
        header
            .fixedSize(horizontal: false, vertical: true)
            .background {
                GeometryReader { geometry in
                    Color.clear.preference(
                        key: NexaAdaptiveHeaderHeightPreferenceKey.self,
                        value: geometry.size.height
                    )
                }
            }
            .frame(height: displayedHeight, alignment: .top)
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .clipped()
            .animation(
                behavior == .hideOnScroll && !reduceMotion ? .easeInOut(duration: 0.18) : nil,
                value: state.isHidden
            )
            .accessibilityHidden(behavior == .hideOnScroll && state.isHidden)
            .onPreferenceChange(NexaAdaptiveHeaderHeightPreferenceKey.self) { height in
                guard abs(state.expandedHeight - height) >= 0.5 else { return }
                state.expandedHeight = height
            }
    }
}

private struct NexaScrollOffsetProbe: UIViewRepresentable {
    let state: NexaAdaptiveScrollHeaderState
    let behavior: NexaAdaptiveHeaderBehavior

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeUIView(context: Context) -> UIView {
        let view = UIView(frame: .zero)
        view.isUserInteractionEnabled = false
        view.isAccessibilityElement = false
        context.coordinator.connect(from: view, state: state, behavior: behavior)
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {
        context.coordinator.connect(from: view, state: state, behavior: behavior)
    }

    static func dismantleUIView(_ view: UIView, coordinator: Coordinator) {
        coordinator.disconnect()
    }

    @MainActor
    final class Coordinator {
        private weak var state: NexaAdaptiveScrollHeaderState?
        private weak var scrollView: UIScrollView?
        private var behavior: NexaAdaptiveHeaderBehavior = .pinned
        private var observation: NSKeyValueObservation?

        func connect(
            from view: UIView,
            state: NexaAdaptiveScrollHeaderState,
            behavior: NexaAdaptiveHeaderBehavior
        ) {
            self.state = state
            self.behavior = behavior
            DispatchQueue.main.async { [weak self, weak view] in
                guard let self, let view, let target = Self.nearestScrollView(from: view) else { return }
                guard self.scrollView !== target else { return }
                self.observation?.invalidate()
                self.scrollView = target
                self.observation = target.observe(\.contentOffset, options: [.initial, .new]) { [weak self] scroll, _ in
                    guard let self else { return }
                    self.state?.update(
                        offset: scroll.contentOffset.y + scroll.adjustedContentInset.top,
                        behavior: self.behavior
                    )
                }
            }
        }

        func disconnect() {
            observation?.invalidate()
            observation = nil
            scrollView = nil
            state = nil
        }

        private static func nearestScrollView(from view: UIView) -> UIScrollView? {
            var queue = [view]
            var index = 0
            var visited = Set<ObjectIdentifier>()
            while index < queue.count {
                let current = queue[index]
                index += 1
                guard visited.insert(ObjectIdentifier(current)).inserted else { continue }
                if let scrollView = current as? UIScrollView {
                    return scrollView
                }
                if let parent = current.superview {
                    queue.append(parent)
                }
                queue.append(contentsOf: current.subviews)
            }
            return nil
        }
    }
}

struct NexaAdaptiveScrollHeaderPrimitive<Header: View, Content: View>: View {
    let behavior: NexaAdaptiveHeaderBehavior
    let collapsedHeight: CGFloat
    @State private var state = NexaAdaptiveScrollHeaderState()
    private let header: Header
    private let content: Content

    init(
        behavior: NexaAdaptiveHeaderBehavior,
        collapsedHeight: CGFloat,
        @ViewBuilder header: () -> Header,
        @ViewBuilder content: () -> Content
    ) {
        self.behavior = behavior
        self.collapsedHeight = collapsedHeight
        self.header = header()
        self.content = content()
    }

    var body: some View {
        VStack(spacing: 0) {
            NexaAdaptiveScrollHeaderView(
                state: state,
                behavior: behavior,
                collapsedHeight: collapsedHeight,
                header: { header }
            )
            content
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .background(alignment: .topLeading) {
                    if behavior != .pinned {
                        NexaScrollOffsetProbe(state: state, behavior: behavior)
                            .frame(width: 0, height: 0)
                    }
                }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

// nexa-codegen: NexaRefreshBounceModifier
private struct NexaRefreshBounceModifier: ViewModifier {
    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 16.4, *) {
            content.scrollBounceBehavior(.always, axes: .vertical)
        } else {
            content
        }
    }
}

// nexa-codegen: NexaBottomSheetPrimitive
struct NexaBottomSheetPrimitive<Content: View>: View {
    @State private var isPresented = false
    @State private var refGeneration: UInt64?
    let componentRef: NexaBottomSheetRef
    let contentSized: Bool
    let contentPadding: CGFloat
    let cornerRadius: CGFloat?
    let detents: Set<PresentationDetent>
    let initialDetent: PresentationDetent
    let showDragIndicator: Bool
    let allowsDismissal: Bool
    let allowsBackgroundInteraction: Bool
    let shouldDismissOnBackPress: Bool
    let backgroundColor: Color?
    let onDismiss: () -> Void
    @State private var selectedDetent: PresentationDetent
    @State private var measuredContentHeight: CGFloat = 1
    private let content: Content

    init(
        componentRef: NexaBottomSheetRef,
        contentSized: Bool,
        contentPadding: CGFloat,
        cornerRadius: CGFloat?,
        detents: Set<PresentationDetent>,
        initialDetent: PresentationDetent,
        showDragIndicator: Bool,
        allowsDismissal: Bool,
        allowsBackgroundInteraction: Bool,
        shouldDismissOnBackPress: Bool,
        backgroundColor: Color?,
        scrimColor: Color?,
        onDismiss: @escaping () -> Void,
        @ViewBuilder content: () -> Content
    ) {
        self.componentRef = componentRef
        self.contentSized = contentSized
        self.contentPadding = contentPadding
        self.cornerRadius = cornerRadius
        self.detents = detents
        self.initialDetent = initialDetent
        self.showDragIndicator = showDragIndicator
        self.allowsDismissal = allowsDismissal
        self.allowsBackgroundInteraction = allowsBackgroundInteraction
        self.shouldDismissOnBackPress = shouldDismissOnBackPress
        self.backgroundColor = backgroundColor
        self.onDismiss = onDismiss
        _selectedDetent = State(initialValue: contentSized ? .height(1) : initialDetent)
        self.content = content()
    }

    @ViewBuilder var body: some View {
        let presentation = $isPresented
        Color.clear
            .frame(width: 0, height: 0)
            .sheet(isPresented: $isPresented, onDismiss: {
                onDismiss()
            }) {
                NexaBottomSheetContentPrimitive(
                    contentPadding: contentPadding,
                    contentSized: contentSized
                ) {
                    content
                }
                    .onPreferenceChange(NexaBottomSheetContentHeightKey.self) { height in
                        guard contentSized, height.isFinite, height > 0 else { return }
                        guard abs(measuredContentHeight - height) > 0.5 else { return }
                        measuredContentHeight = height
                        selectedDetent = .height(height)
                    }
                    .presentationDetents(
                        contentSized ? [.height(measuredContentHeight)] : detents,
                        selection: $selectedDetent
                    )
                    .modifier(NexaBottomSheetCornerRadius(cornerRadius: cornerRadius))
                    .presentationDragIndicator(showDragIndicator ? .visible : .hidden)
                    .interactiveDismissDisabled(!allowsDismissal)
                    .modifier(NexaBottomSheetBackground(
                        color: backgroundColor ?? Color(uiColor: .systemBackground)
                    ))
                    .modifier(NexaBottomSheetBackgroundInteraction(
                        allowsBackgroundInteraction: allowsBackgroundInteraction
                    ))
            }
            .onChange(of: isPresented) { presented in
                if presented {
                    selectedDetent = contentSized
                        ? .height(measuredContentHeight)
                        : initialDetent
                }
            }
            .onAppear {
                refGeneration = componentRef.attach(
                    present: { presentation.wrappedValue = true },
                    dismiss: { presentation.wrappedValue = false }
                )
            }
            .onDisappear {
                if let refGeneration {
                    componentRef.detach(generation: refGeneration)
                }
            }
    }
}

// nexa-codegen: NexaBottomSheetCornerRadius
private struct NexaBottomSheetCornerRadius: ViewModifier {
    let cornerRadius: CGFloat?

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 16.4, *), let cornerRadius {
            content.presentationCornerRadius(cornerRadius)
        } else if let cornerRadius {
            content.background {
                NexaBottomSheetCornerRadiusBridge(cornerRadius: cornerRadius)
                    .frame(width: 0, height: 0)
            }
        } else {
            content
        }
    }
}

private struct NexaBottomSheetCornerRadiusBridge: UIViewControllerRepresentable {
    let cornerRadius: CGFloat

    func makeUIViewController(context: Context) -> NexaBottomSheetCornerRadiusController {
        NexaBottomSheetCornerRadiusController()
    }

    func updateUIViewController(
        _ controller: NexaBottomSheetCornerRadiusController,
        context: Context
    ) {
        controller.cornerRadius = cornerRadius
        controller.applyCornerRadius()
    }
}

@MainActor
private final class NexaBottomSheetCornerRadiusController: UIViewController {
    var cornerRadius: CGFloat = 0
    private var appliedCornerRadius: CGFloat?

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        applyCornerRadius()
    }

    func applyCornerRadius() {
        guard #unavailable(iOS 16.4), appliedCornerRadius != cornerRadius else { return }
        var candidate: UIViewController? = self
        while let controller = candidate {
            if let sheet = controller.presentationController as? UISheetPresentationController {
                sheet.preferredCornerRadius = cornerRadius
                appliedCornerRadius = cornerRadius
                return
            }
            candidate = controller.parent
        }
    }
}

// nexa-codegen: NexaBottomSheetContentPrimitive
private struct NexaBottomSheetContentPrimitive<Content: View>: View {
    let contentPadding: CGFloat
    let contentSized: Bool
    private let content: Content

    init(
        contentPadding: CGFloat,
        contentSized: Bool,
        @ViewBuilder content: () -> Content
    ) {
        self.contentPadding = contentPadding
        self.contentSized = contentSized
        self.content = content()
    }

    @ViewBuilder
    var body: some View {
        if contentSized {
            NexaColumnPrimitive(spacing: 0) {
                content
            }
            .frame(maxWidth: .infinity, alignment: .top)
            .fixedSize(horizontal: false, vertical: true)
            .padding(contentPadding)
            .background {
                GeometryReader { geometry in
                    Color.clear.preference(
                        key: NexaBottomSheetContentHeightKey.self,
                        value: geometry.size.height
                    )
                }
            }
        } else {
            NexaColumnPrimitive(spacing: 0) {
                content
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .padding(contentPadding)
        }
    }
}

private struct NexaBottomSheetContentHeightKey: PreferenceKey {
    static let defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

// SwiftUI owns a native sheet's dim layer. Background interaction is the
// explicit cross-platform opt-in for an undimmed, interactive backdrop.
// nexa-codegen: NexaBottomSheetBackgroundInteraction
private struct NexaBottomSheetBackgroundInteraction: ViewModifier {
    let allowsBackgroundInteraction: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 16.4, *), allowsBackgroundInteraction {
            content.presentationBackgroundInteraction(.enabled)
        } else {
            content
        }
    }
}

// nexa-codegen: NexaBottomSheetBackground
private struct NexaBottomSheetBackground: ViewModifier {
    let color: Color?

    @ViewBuilder
    func body(content: Content) -> some View {
        if let color {
            if #available(iOS 16.4, *) {
                content.presentationBackground(color)
            } else {
                content.background(color)
            }
        } else {
            content
        }
    }
}

// nexa-codegen: NexaAlertDialogPrimitive
struct NexaAlertDialogPrimitive<Actions: View>: View {
    @Binding var isPresented: Bool
    let title: Text
    let message: Text?
    private let actions: Actions

    init(
        isPresented: Binding<Bool>,
        title: Text,
        message: Text?,
        @ViewBuilder actions: () -> Actions
    ) {
        self._isPresented = isPresented
        self.title = title
        self.message = message
        self.actions = actions()
    }

    @ViewBuilder var body: some View {
        let host = Color.clear.frame(width: 0, height: 0)
        if let message {
            host.alert(title, isPresented: $isPresented) {
                actions
            } message: {
                message
            }
        } else {
            host.alert(title, isPresented: $isPresented) {
                actions
            }
        }
    }
}

// nexa-codegen: NexaConfirmationDialogPrimitive
struct NexaConfirmationDialogPrimitive<Actions: View>: View {
    @Binding var isPresented: Bool
    let title: Text
    private let actions: Actions

    init(
        isPresented: Binding<Bool>,
        title: Text,
        @ViewBuilder actions: () -> Actions
    ) {
        self._isPresented = isPresented
        self.title = title
        self.actions = actions()
    }

    var body: some View {
        Color.clear.frame(width: 0, height: 0)
            .confirmationDialog(title, isPresented: $isPresented, titleVisibility: .visible) {
                actions
            }
    }
}

// nexa-codegen: NexaPageIndicatorPrimitive
struct NexaPageIndicatorPrimitive: View {
    let index: Int
    let selectedIndex: Int
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            Circle()
                .fill(selectedIndex == index ? __NEXA_SWIFT_DEFAULT_ACCENT_COLOR__ : __NEXA_PAGE_INDICATOR_MUTED_COLOR__.opacity(__NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__))
                .frame(
                    width: selectedIndex == index ? __NEXA_PAGE_INDICATOR_SELECTED_SIZE__ : __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__,
                    height: selectedIndex == index ? __NEXA_PAGE_INDICATOR_SELECTED_SIZE__ : __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__
                )
        }
        .buttonStyle(.plain)
        .animation(.default, value: selectedIndex)
        .accessibilityLabel("Page \(index + 1)")
    }
}

// nexa-codegen: NexaPageIndicatorRow
struct NexaPageIndicatorRow: View {
    let pageCount: Int
    let selectedIndex: Int
    let onSelect: (Int) -> Void

    var body: some View {
        HStack(spacing: __NEXA_PAGE_INDICATOR_SPACING__) {
            ForEach(0..<pageCount, id: \.self) { index in
                NexaPageIndicatorPrimitive(index: index, selectedIndex: selectedIndex) {
                    onSelect(index)
                }
            }
        }
        .padding(.bottom, __NEXA_PAGE_INDICATOR_BOTTOM_INSET__)
    }
}

// nexa-codegen: NexaNavigationSplitViewPrimitive
struct NexaNavigationSplitViewPrimitive<Sidebar: View, Detail: View>: View {
    @Binding var detailVisible: Bool
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    private let sidebar: Sidebar
    private let detail: Detail

    init(
        detailVisible: Binding<Bool>,
        @ViewBuilder sidebar: () -> Sidebar,
        @ViewBuilder detail: () -> Detail
    ) {
        _detailVisible = detailVisible
        self.sidebar = sidebar()
        self.detail = detail()
    }

    @ViewBuilder
    var body: some View {
        if horizontalSizeClass == .compact {
            Group {
                if detailVisible {
                    detail
                        .transition(.move(edge: .trailing))
                } else {
                    sidebar
                        .transition(.move(edge: .leading))
                }
            }
            .animation(.default, value: detailVisible)
        } else if #available(iOS 17.0, *) {
            NavigationSplitView(preferredCompactColumn: Binding(
                get: { detailVisible ? .detail : .sidebar },
                set: { detailVisible = ($0 == .detail) }
            )) {
                sidebar
            } detail: {
                detail
            }
        } else if #available(iOS 16.0, *) {
            NavigationSplitView {
                sidebar
            } detail: {
                detail
            }
        } else {
            HStack(spacing: 0) {
                TupleView((sidebar, Divider(), detail))
            }
        }
    }
}

// nexa-codegen: NexaNavigationScreenPrimitive
struct NexaNavigationScreenPrimitive<Content: View>: View {
    let title: String
    let inline: Bool
    let leadingTitle: Bool
    let content: Content

    init(
        title: String,
        inline: Bool,
        leadingTitle: Bool,
        @ViewBuilder content: () -> Content
    ) {
        self.title = title
        self.inline = inline
        self.leadingTitle = leadingTitle
        self.content = content()
    }

    var body: some View {
        content
            .navigationTitle(inline && leadingTitle ? "" : title)
            .modifier(NexaNavigationTitleDisplayModeModifier(inline: inline))
            .toolbar {
                if inline && leadingTitle {
                    ToolbarItem(placement: .topBarLeading) {
                        leadingNavigationTitle
                    }
                }
            }
            .toolbar(.visible, for: .navigationBar)
            .toolbar(.hidden, for: .tabBar)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private var leadingNavigationTitle: some View {
        Text(title)
            .font(.headline)
            .lineLimit(1)
            .truncationMode(.tail)
            .layoutPriority(1)
            .frame(minWidth: 160, alignment: .leading)
            .offset(x: -13)
            .accessibilityLabel(title)
            .accessibilityAddTraits(.isHeader)
    }
}

// nexa-codegen: NexaNavigationTitleDisplayModeModifier
struct NexaNavigationTitleDisplayModeModifier: ViewModifier {
    let inline: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 17.0, *) {
            content.toolbarTitleDisplayMode(inline ? .inline : .large)
        } else {
            content.navigationBarTitleDisplayMode(inline ? .inline : .large)
        }
    }
}

// nexa-codegen: NexaNavigationLinkPrimitive
struct NexaNavigationLinkPrimitive<Value: Hashable, Label: View>: View {
    let value: Value
    let enabled: Bool
    let label: Label

    init(value: Value, enabled: Bool, @ViewBuilder label: () -> Label) {
        self.value = value
        self.enabled = enabled
        self.label = label()
    }

    var body: some View {
        NavigationLink(value: value) {
            label
        }
        .disabled(!enabled)
    }
}
"#
);

/// Owns tab appearance behavior for both generated release views and dynamic
/// DevRuntime tabs while leaving SwiftUI's typed TabContent builders intact.
const NEXA_APP_BOTTOM_BAR_PRIMITIVE: &str = r#"struct NexaAppBottomBarPrimitive: ViewModifier {
    private let tint: Color?
    private let adaptiveStyle: Bool
    private let activatesSearchTab: Bool

    init(
        tint: Color?,
        adaptiveStyle: Bool,
        activatesSearchTab: Bool
    ) {
        self.tint = tint
        self.adaptiveStyle = adaptiveStyle
        self.activatesSearchTab = activatesSearchTab
    }

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 18.0, *), adaptiveStyle {
            if #available(iOS 26.0, *), activatesSearchTab {
                applyTint(content.tabViewStyle(.sidebarAdaptable)
                    .tabViewSearchActivation(.searchTabSelection))
            } else {
                applyTint(content.tabViewStyle(.sidebarAdaptable))
            }
        } else {
            applyTint(content)
        }
    }

    @ViewBuilder
    private func applyTint<Content: View>(_ content: Content) -> some View {
        if let tint {
            content.tint(tint)
        } else {
            content
        }
    }
}

struct NexaAppBottomBarSafeAreaPrimitive: ViewModifier {
    @Environment(\.nexaVisualSafeAreaInsets) private var safeAreaSnapshot

    func body(content: Content) -> some View {
        content.environment(
            \.nexaVisualSafeAreaInsets,
            safeAreaSnapshot.removing(Edge.bottom, direction: .leftToRight)
        )
    }
}

struct NexaTabLabelPrimitive: View {
    let title: Text
    let icon: NexaIconSpec

    var body: some View {
        Label { title } icon: {
            if let symbol = icon.symbol {
                Image(systemName: symbol).accessibilityHidden(true)
            } else {
                NexaIconView(icon: icon, description: "", size: 20)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(title)
        .accessibilityIdentifier("")
    }
}

"#;

#[cfg(test)]
fn shared_component_primitives(line_height: bool) -> String {
    shared_component_primitives_with_animation(line_height, false)
}

fn shared_component_primitives_with_animation(
    line_height: bool,
    reduce_motion_animation: bool,
) -> String {
    let tokens = [
        (
            "__NEXA_BUTTON_LOADING_INDICATOR_SIZE__",
            nexa_codegen::design_system::BUTTON_LOADING_INDICATOR_SIZE.to_string(),
        ),
        (
            "__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_SMALL_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_SMALL_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_SMALL_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_LARGE_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_LARGE_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_DEFAULT_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_DEFAULT_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_MIN_WIDTH__",
            nexa_codegen::design_system::BUTTON_MIN_WIDTH.to_string(),
        ),
        (
            "__NEXA_BUTTON_MIN_TAP_TARGET__",
            nexa_codegen::design_system::BUTTON_MIN_TAP_TARGET.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_MIN_HEIGHT__",
            nexa_codegen::design_system::BUTTON_LARGE_MIN_HEIGHT.to_string(),
        ),
        (
            "__NEXA_DEFAULT_BODY_FONT_SIZE__",
            nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE.to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_HEIGHT__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .header_height
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_COLLAPSE_DISTANCE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .collapse_distance
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_EXPANDED_FONT_SIZE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .expanded_font_size
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_COLLAPSED_FONT_SIZE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .collapsed_font_size
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_HORIZONTAL_INSET__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .horizontal_inset
                .to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_ROW_VERTICAL_INSET.to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_ROW_HORIZONTAL_INSET.to_string(),
        ),
        (
            "__NEXA_IOS_FORM_SECTION_HEADER_LEADING_CORRECTION__",
            nexa_codegen::design_system::IOS_FORM_SECTION_HEADER_LEADING_CORRECTION.to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_CONTAINER_HORIZONTAL_INSET.to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_CONTENT_VERTICAL_INSET.to_string(),
        ),
        (
            "__NEXA_IOS_LEGACY_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__",
            nexa_codegen::design_system::IOS_LEGACY_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET
                .to_string(),
        ),
        (
            "__NEXA_SWIFT_DEFAULT_ACCENT_COLOR__",
            nexa_codegen::design_system::SWIFT_DEFAULT_ACCENT_COLOR.to_owned(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_MUTED_COLOR__",
            crate::generator::engine::colors::expression_from_argb(
                nexa_codegen::design_system::MUTED_TEXT_ARGB,
            ),
        ),
        (
            "__NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__",
            nexa_codegen::design_system::PAGE_INDICATOR_INACTIVE_OPACITY.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_SELECTED_SIZE__",
            nexa_codegen::design_system::PAGE_INDICATOR_SELECTED_SIZE.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__",
            nexa_codegen::design_system::PAGE_INDICATOR_UNSELECTED_SIZE.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_SPACING__",
            nexa_codegen::design_system::PAGE_INDICATOR_SPACING.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_BOTTOM_INSET__",
            nexa_codegen::design_system::PAGE_INDICATOR_BOTTOM_INSET.to_string(),
        ),
    ];
    let mut source = tokens.iter().fold(
        NEXA_SHARED_COMPONENT_PRIMITIVES.to_owned(),
        |source, (token, value)| source.replace(token, value),
    );
    let line_height_modifier = if line_height {
        r#"modifier(NexaTextLineHeightModifier(
                factor: style.lineHeightFactor,
                baseFontSize: style.lineHeightFontSize
            ))"#
    } else {
        "lineSpacing(0)"
    };
    source = source.replace("__NEXA_TEXT_LINE_HEIGHT_MODIFIER__", line_height_modifier);
    let reduce_motion_modifier = if reduce_motion_animation {
        r#"// nexa-codegen: NexaAnimationAccessibilityModifier
private struct NexaAnimationAccessibilityModifier: ViewModifier {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content.transaction { transaction in
            if reduceMotion {
                transaction.disablesAnimations = true
            }
        }
    }
}"#
    } else {
        ""
    };
    source = source.replace(
        "__NEXA_REDUCE_MOTION_ANIMATION_MODIFIER__",
        reduce_motion_modifier,
    );
    if line_height {
        source.push_str(
            r#"

// nexa-codegen: NexaTextLineHeightModifier
private struct NexaTextLineHeightModifier: ViewModifier {
    let factor: CGFloat?
    let baseFontSize: CGFloat

    @ViewBuilder func body(content: Content) -> some View {
        if let factor {
            if #available(iOS 26.0, *) {
                content.lineHeight(.multiple(factor: factor))
            } else {
                content.lineSpacing(
                    baseFontSize * factor - UIFont.systemFont(ofSize: baseFontSize).lineHeight
                )
            }
        } else {
            content.lineSpacing(0)
        }
    }
}
"#,
        );
    }
    source.push_str(include_str!("components/date_picker.swift"));
    source.push_str(include_str!("components/text_reveal.swift"));
    source
}

const NEXA_REGEX_HELPERS: &str = r#"struct NexaRegexMatch {
    let value: String
    let range: NexaRegexRange
    let groups: [String?]
}

struct NexaRegexRange {
    let lowerBound: Int64
    let upperBound: Int64
}

struct NexaRegex {
    private let expression: NSRegularExpression?

    init(pattern: String) {
        expression = try? NSRegularExpression(pattern: pattern)
    }

    func matches(_ text: String) -> Bool {
        guard let expression else { return false }
        let range = NSRange(text.startIndex..., in: text)
        guard let match = expression.firstMatch(in: text, range: range) else { return false }
        return match.range.location == range.location && match.range.length == range.length
    }

    func find(_ text: String) -> NexaRegexMatch? {
        guard let expression else { return nil }
        let range = NSRange(text.startIndex..., in: text)
        guard let match = expression.firstMatch(in: text, range: range) else { return nil }
        return makeMatch(match, in: text)
    }

    func findAll(_ text: String) -> [NexaRegexMatch] {
        guard let expression else { return [] }
        let range = NSRange(text.startIndex..., in: text)
        return expression.matches(in: text, range: range).map { makeMatch($0, in: text) }
    }

    func replace(_ text: String, with replacement: String) -> String {
        guard let expression else { return text }
        let range = NSRange(text.startIndex..., in: text)
        return expression.stringByReplacingMatches(in: text, range: range, withTemplate: replacement)
    }

    private func makeMatch(_ match: NSTextCheckingResult, in text: String) -> NexaRegexMatch {
        let value = Range(match.range, in: text).map { String(text[$0]) } ?? ""
        let groups = (1..<match.numberOfRanges).map { index -> String? in
            let range = match.range(at: index)
            guard range.location != NSNotFound, let swiftRange = Range(range, in: text) else {
                return nil
            }
            return String(text[swiftRange])
        }
        return NexaRegexMatch(
            value: value,
            range: NexaRegexRange(
                lowerBound: Int64(match.range.location),
                upperBound: Int64(NSMaxRange(match.range))
            ),
            groups: groups
        )
    }
}

private func nexaRegexIsMatch(_ pattern: String, in text: String) -> Bool {
    guard let expression = try? NSRegularExpression(pattern: pattern) else { return false }
    let range = NSRange(text.startIndex..., in: text)
    return expression.firstMatch(in: text, range: range) != nil
}

private func nexaRegexMatches(_ pattern: String, in text: String) -> [String] {
    guard let expression = try? NSRegularExpression(pattern: pattern) else { return [] }
    let range = NSRange(text.startIndex..., in: text)
    return expression.matches(in: text, range: range).compactMap { match in
        Range(match.range, in: text).map { String(text[$0]) }
    }
}

private func nexaRegexReplace(_ pattern: String, in text: String, with replacement: String) -> String {
    guard let expression = try? NSRegularExpression(pattern: pattern) else { return text }
    let range = NSRange(text.startIndex..., in: text)
    return expression.stringByReplacingMatches(in: text, range: range, withTemplate: replacement)
}

"#;

pub(super) use api::{crypto, network, number, permissions, time};
use components::node_renderer as component_renderer;
pub(super) use components::{
    accessibility, bottom_bar, bottom_sheet, controls, custom_components, direction, images, input,
    keyboard, layout, lifecycle, links, list_runtime, lists, navigation, refresh, status_bar,
    system_icons,
};
pub(super) use engine::state::{
    render_component_ref_state, render_immutable_state, render_native_object_state,
    render_native_object_state_uninitialized, render_state_initializers_in_init,
};
use engine::types::{self, swift_type};
pub(super) use engine::{colors, expressions, features, functions, imports, structs, utils, value};

/// Generates the release source as one concatenated string.
///
/// Prefer [`generate_units`]: the CLI writes each unit as its own file, and
/// this form exists for callers that only inspect the text.
pub(super) fn generate(module: &Module) -> String {
    join_units(generate_units(module).into_files(&[], ""))
}

/// Generates the release source as separate compile units.
///
/// Returns the pieces rather than finished files so the caller can merge extra
/// imports -- plugin bindings add imports that every file must see -- before
/// the files are assembled.
pub(super) fn generate_units(module: &Module) -> GeneratedSources {
    let features = features::Features::analyze(module);
    generate_with_analysis(module, features)
}

pub(super) fn generate_widget_units(
    module: &Module,
) -> Result<crate::WidgetGeneratedSources, crate::WidgetGenerationError> {
    widget::generate(module)
}

pub(super) fn generate_units_with_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    let features = features::Features::analyze(module);
    let project_features = project_features_from_analysis(&features);
    (generate_with_analysis(module, features), project_features)
}

pub(super) fn generate_units_with_network_and_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    generate_units_with_network_requirements(module, false)
}

pub(super) fn generate_units_with_plugin_value_runtime_and_network_and_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    generate_units_with_network_requirements(module, true)
}

fn generate_units_with_network_requirements(
    module: &Module,
    plugin_value_runtime: bool,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    let mut features = features::Features::analyze(module);
    features.uses_network_api = true;
    features.uses_native_library = true;
    let project_features = project_features_from_analysis(&features);
    let generated = generate_with_analysis_mode(module, features, false, plugin_value_runtime);
    (generated, project_features)
}

fn project_features_from_analysis(features: &features::Features) -> crate::SwiftProjectFeatures {
    crate::SwiftProjectFeatures {
        requires_user_defaults_reason: features.facts.capabilities.uses_storage_api
            || features.facts.capabilities.uses_locale_api,
        // The feature-gated NexaNetwork helper contains its download operation,
        // which inspects the downloaded file's size with FileManager.
        requires_file_timestamp_reason: features.uses_network_api,
        uses_screen_orientation_api: features.facts.capabilities.uses_screen_orientation_api,
    }
}

/// Generates release units with the generic codec runtime needed by native
/// plugin contracts, including when the app only calls a non-generic method.
pub(super) fn generate_units_with_plugin_value_runtime(module: &Module) -> GeneratedSources {
    let features = features::Features::analyze(module);
    generate_with_analysis_mode(module, features, false, true)
}

pub(super) fn generate_units_with_plugin_value_runtime_and_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    let features = features::Features::analyze(module);
    let project_features = project_features_from_analysis(&features);
    (
        generate_with_analysis_mode(module, features, false, true),
        project_features,
    )
}

fn generate_with_analysis(module: &Module, features: features::Features) -> GeneratedSources {
    generate_with_analysis_mode(module, features, false, false)
}

fn generate_with_analysis_mode(
    module: &Module,
    features: features::Features,
    dev_runtime: bool,
    plugin_value_runtime: bool,
) -> GeneratedSources {
    components::controls::register_known_screens(&module.screens);
    let app_focus_bindings = app_focus_bindings(module, &features);
    let mut all_focus_bindings = app_focus_bindings.clone();
    for screen in &module.screens {
        all_focus_bindings.extend(screen_focus_bindings(screen, &features));
    }
    let uses_fast_list = features.uses_fast_list;
    // Each unit becomes its own file, so every file needs the full import
    // block and the shared native-object storage helper.
    let mut preamble = String::new();
    if module_has_native_object_state(module) {
        preamble.push_str(
            "@MainActor\nprivate final class NexaNativeObjectStorage<Value>: ObservableObject {\n    @Published var value: Value\n\n    init(makeValue: () -> Value) {\n        value = makeValue()\n    }\n}\n\n",
        );
    }
    if features.uses_drag || features.uses_pinch {
        preamble.push_str(DRAG_GESTURE_HELPERS);
    }
    if features.facts.ui.style.dynamic_color {
        preamble.push_str(&NEXA_DYNAMIC_COLOR_HELPER.replace(
            "__NEXA_SWIFT_DEFAULT_ACCENT_COLOR__",
            nexa_codegen::design_system::SWIFT_DEFAULT_ACCENT_COLOR,
        ));
    }
    if features.facts.ui.style.glass {
        preamble.push_str(NEXA_GLASS_HELPER);
    }
    preamble.push_str(NEXA_COLLECTION_HELPERS);
    let mut units = SourceUnits::new("swift");
    units.set_imports(&imports::render_with_dev_runtime(&features, dev_runtime));
    units.set_preamble(&preamble);
    // Value codecs live in the types file: app structs and enums are
    // file-private, and a codec for one has to construct and read it.
    let value_codecs = nexa_codegen::value::collect(module);
    let json_types = nexa_codegen::value::collect_json_types(module);
    units.write("types", |out| {
        if dev_runtime || features.facts.ui.bottom_bar.present {
            out.push_str(NEXA_APP_BOTTOM_BAR_PRIMITIVE);
        }
        if features.facts.capabilities.uses_locale_api {
            out.push_str(NEXA_LOCALE_HELPER);
        } else if features.facts.capabilities.uses_localized_strings {
            out.push_str(NEXA_LOCALIZATION_HELPER);
        }
        // Regex declarations are shared by every generated unit. Keep them in
        // the single types file instead of the repeated per-file preamble.
        if features.facts.capabilities.uses_regex_api {
            out.push_str(NEXA_REGEX_HELPERS);
        }
        types::render_enums(module, out);
        types::render_component_ref_types(out);
        structs::render(module, out);
        types::render_navigation_routes(module, out);
        if dev_runtime {
            value::render_for_dev(&value_codecs, out);
        } else if plugin_value_runtime {
            value::render_with_runtime(&value_codecs, out);
        } else {
            value::render(&value_codecs, out);
        }
    });

    if features.facts.ui.image.asset || features.uses_remote_image {
        units.write("image-primitives", |out| {
            components::images::render_primitives(
                features.facts.ui.image.asset,
                features.uses_remote_image,
                out,
            );
        });
    }

    if features.facts.capabilities.uses_json_api {
        units.write("json", |out| {
            api::json::render(
                &json_types,
                &module.enums,
                features.facts.capabilities.uses_storage_api,
                out,
            );
        });
    }

    units.write("app", |out| {
        if features.uses_fast_list {
            out.push_str(
                "private struct NexaIdentifiedListRow: Identifiable {\n    let id: AnyHashable\n    let position: Int\n}\n\n",
            );
        }
        let untracked_task_scopes = &features.facts.untracked_task_launch_scopes;
        let has_task_scope = untracked_task_scopes.app
            || !untracked_task_scopes.screens.is_empty()
            || !untracked_task_scopes.components.is_empty()
            || features.facts.ui.lists.end_reached_task_launch;
        if has_task_scope {
            out.push_str(NEXA_TASK_SCOPE_HELPER);
        }
        if features.facts.ui.lists.end_reached_task_launch {
            out.push_str(NEXA_LIST_TASK_SCOPE_VIEW_HELPER);
        }
        out.push_str(&format!(
            "public struct {}: View {{\n",
            nexa_codegen::names::screen_name(&module.app_name)
        ));
        if features.facts.capabilities.uses_locale_api {
            out.push_str(
                "    @AppStorage(\"dev.nexa.locale.override\") private var nexaLocaleOverride = \"\"\n",
            );
        }
        if untracked_task_scopes.app {
            out.push_str("    @State private var nexaTaskScope = NexaTaskScope()\n");
        }
        if features.uses_shared_elements {
            out.push_str("    @Namespace private var nexaSharedNamespace\n");
        }
        if features.facts.ui.app.window_metrics {
            out.push_str("    @Environment(\\.layoutDirection) private var layoutDirection\n");
        }
        let has_signals = module
            .states
            .iter()
            .any(|state| matches!(state.ty, nexa_ir::Type::Signal(_)));
        for state in &module.states {
            if render_component_ref_state(state, 1, out) {
                continue;
            }
            if state.is_native_class_constructor_binding() {
                if has_signals {
                    render_native_object_state_uninitialized(state, 1, out);
                } else {
                    render_native_object_state(state, 1, out);
                }
                continue;
            }
            if matches!(state.ty, nexa_ir::Type::Signal(_)) {
                let name = nexa_codegen::names::state_name(&state.name);
                out.push_str(&format!(
                    "    @StateObject private var {name}: {}\n",
                    swift_type(&state.ty),
                ));
                continue;
            }
            if (!state.mutable && !state.is_native_class_instance_binding())
                || all_focus_bindings.contains(&state.name)
            {
                continue;
            }
            let name = nexa_codegen::names::state_name(&state.name);
            out.push_str(&format!(
                "    @State private var {name}: {} = {}\n",
                swift_type(&state.ty),
                expressions::expression(&state.initial)
            ));
        }
        render_immutable_state(&module.states, 1, out);
        for binding in &app_focus_bindings {
            out.push_str(&format!(
                "    @FocusState private var {}: Bool\n",
                nexa_codegen::names::state_name(binding),
            ));
        }
        if !module.states.is_empty() || !app_focus_bindings.is_empty() {
            out.push('\n');
        }
        if features.app_uses_adaptive_color {
            out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n");
        }
        if features.app_uses_size_class {
            out.push_str(
                "    @Environment(\\.horizontalSizeClass) private var nexaHorizontalSizeClass\n",
            );
            out.push_str(
                "    @Environment(\\.verticalSizeClass) private var nexaVerticalSizeClass\n",
            );
        }
        if module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some()
            || features.uses_widgets
        {
            out.push_str("    @Environment(\\.scenePhase) private var nexaScenePhase\n");
        }
        if features.uses_navigation_back {
            out.push_str("    @Environment(\\.dismiss) private var nexaDismiss\n");
        }
        if features.app_uses_adaptive_color
            || features.app_uses_size_class
            || features.uses_navigation_back
            || module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some()
            || features.uses_widgets
        {
            out.push('\n');
        }
        if has_signals {
            out.push_str("    public init() {\n");
            let mut writer = nexa_codegen::SourceWriter::new();
            render_state_initializers_in_init(&module.states, 2, &mut writer);
            out.push_str(&writer.finish());
            out.push_str("    }\n\n    public var body: some View {\n");
        } else {
            out.push_str("    public init() {}\n\n    public var body: some View {\n");
        }
        // A top-level conditional is already emitted as a concrete `Group` by
        // the node renderer, so body modifiers (including task cleanup) can be
        // applied to it directly without adding a redundant second wrapper.
        let needs_group = features.uses_shared_elements;
        out.push_str("        NexaEdgeToEdgeRoot { nexaWindowMetrics in\n");

        if needs_group {
            out.push_str("            Group {\n");
            if module.body.len() == 1 {
                component_renderer::render_node(&module.body[0], module, &features, 4, out);
            } else {
                layout::render_layout(
                    LayoutKind::Column,
                    0.0,
                    &ViewStyle::default(),
                    &module.body,
                    &components::RenderScope {
                        module,
                        features: &features,
                    },
                    4,
                    out,
                );
            }
            out.push_str("\n            }");
        } else if module.body.len() == 1 {
            component_renderer::render_node(&module.body[0], module, &features, 3, out);
        } else {
            layout::render_layout(
                LayoutKind::Column,
                0.0,
                &ViewStyle::default(),
                &module.body,
                &components::RenderScope {
                    module,
                    features: &features,
                },
                3,
                out,
            );
        }
        direction::render(module.direction, 3, out);
        lifecycle::render_on_appear(module.on_appear.as_deref(), module.on_appear_async, 3, out);
        lifecycle::render_on_disappear(
            module.on_disappear.as_deref(),
            &lifecycle::task_handles(&module.states),
            untracked_task_scopes.app,
            3,
            out,
        );
        lifecycle::render_scene_phase(module, 3, out);
        let app_root_status_bar = if module
            .screens
            .iter()
            .any(|screen| screen.status_bar.is_some())
        {
            None
        } else {
            module.status_bar
        };
        status_bar::render(app_root_status_bar, 3, out);
        if features.uses_shared_elements {
            out.push_str("\n            .environment(\\.nexaSharedNamespace, nexaSharedNamespace)");
        }
        if features.facts.uses_with_animation {
            out.push_str("\n            .modifier(NexaAnimationAccessibilityModifier())");
        }
        if features.facts.capabilities.uses_locale_api {
            out.push_str("\n            .environment(\\.locale, NexaLocale.formatLocale(override: nexaLocaleOverride))");
            out.push_str("\n            .environment(\\.layoutDirection, NexaLocale.isRightToLeft(override: nexaLocaleOverride) ? .rightToLeft : .leftToRight)");
        }
        out.push_str("\n        }\n    }\n");
        components::bottom_bar::render_bottom_bar_helpers(&module.body, module, &features, out);
        if !module.screens.is_empty() {
            for screen in &module.screens {
                navigation::render_screen_view(screen, module, &features, out);
            }
        }
        out.push_str("}\n");
    });

    if features.uses_shared_elements {
        units.write("shared-elements", components::shared_elements::render);
    }

    units.write("components", |out| {
        custom_components::render(module, &features, out);
    });
    if uses_fast_list {
        units.write("list-runtime", |out| {
            list_runtime::render(
                out,
                features.uses_sticky_header,
                features.uses_scroll_events,
                features.uses_vertical_list,
                features.uses_horizontal_list,
                features.uses_grid_list,
                features.uses_sectioned_list,
            );
        });
    }
    if features.uses_native_library {
        units.write("native-library", |out| {
            network::render(
                out,
                features.uses_network_api,
                features.uses_remote_image,
                features.uses_path_api,
                features.uses_file_api,
                features.uses_file_async,
                features.uses_network_connectivity,
            );
        });
    }
    if features.uses_permissions {
        units.write("permissions", |out| {
            permissions::render(
                out,
                features.uses_permission_request,
                &features.used_permissions,
                features.dynamic_permission,
                features.expose_permissions_to_dev_runtime,
            );
        });
    }
    if features.facts.capabilities.uses_time {
        units.write("time", |out| {
            time::render(out);
        });
    }
    if features.facts.capabilities.uses_app_icon_api {
        units.write("app-icon", api::app_icon::render);
    }
    if features.facts.capabilities.uses_number_formatting {
        units.write("number", |out| {
            number::render(out);
        });
    }
    if features.facts.capabilities.uses_crypto_api {
        units.write("crypto", |out| {
            crypto::render(out);
        });
    }
    if features.facts.capabilities.uses_secure_storage_api {
        units.write("secure-storage", |out| {
            api::secure_storage::render(out);
        });
    }
    if features.facts.capabilities.uses_storage_api {
        units.write("storage", api::storage::render);
    }
    if features.facts.capabilities.uses_clipboard_api {
        units.write("clipboard", api::clipboard::render);
    }
    if features.facts.capabilities.uses_screen_orientation_api {
        units.write("screen-orientation", api::screen_orientation::render);
    }
    units.write("functions", |out| {
        functions::render(module, out);
    });
    let mut generated = units.finish();
    let references = generated
        .units
        .iter()
        .map(|unit| unit.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let primitive_source = shared_component_primitives_with_animation(
        dev_runtime || features.facts.ui.text.line_height,
        features.facts.uses_with_animation,
    );
    let primitives = if dev_runtime {
        nexa_codegen::source_sections::all(&primitive_source)
    } else {
        nexa_codegen::source_sections::reachable(&primitive_source, &references)
    };
    generated.prepend_to_unit("NexaGenerated_types.swift", &primitives);
    generated
}

pub(super) fn app_focus_bindings(
    module: &Module,
    features: &features::Features,
) -> std::collections::BTreeSet<String> {
    let mut bindings = features.facts.focus_bindings.app.clone();
    for screen in &module.screens {
        for binding in screen_focus_bindings(screen, features) {
            if module.states.iter().any(|state| state.name == binding) {
                bindings.insert(binding);
            }
        }
    }
    bindings
}

pub(super) fn screen_focus_bindings(
    screen: &nexa_ir::Screen,
    features: &features::Features,
) -> std::collections::BTreeSet<String> {
    let mut bindings = features
        .facts
        .focus_bindings
        .screens
        .get(&screen.name)
        .cloned()
        .unwrap_or_default();
    nexa_ir::walk::walk_ir(
        &screen.body,
        &mut |node| {
            if let Node::TextInput {
                focused: Some(name),
                ..
            } = node
            {
                bindings.insert(name.clone());
            }
        },
        &mut |_| {},
    );
    bindings
}

/// Concatenates units into a single source string.
fn join_units(units: Vec<SourceUnit>) -> String {
    let mut source = String::new();
    for unit in units {
        source.push_str(&unit.contents);
    }
    source
}

/// Generates the development host source as one concatenated string.
pub(super) fn generate_for_dev(module: &Module) -> String {
    join_units(generate_for_dev_units(module).into_files(&[], ""))
}

/// Generates the development host source as separate compile units.
pub(super) fn generate_for_dev_units(module: &Module) -> GeneratedSources {
    generate_for_dev_units_with_project_features(module).0
}

pub(super) fn generate_for_dev_units_with_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    let mut features = features::Features::analyze(module);
    // Hot reload can introduce any FastList axis after the native host has
    // been built. Keep the same UIKit list primitives used by AOT apps ready
    // in the DevRuntime host.
    features.uses_fast_list = true;
    features.uses_vertical_list = true;
    features.uses_horizontal_list = true;
    features.uses_grid_list = true;
    features.uses_sectioned_list = true;
    features.uses_sticky_header = true;
    features.uses_scroll_events = true;
    // A shared element may appear after the development host is built, so
    // keep its namespace bridge available for hot-reloaded modules.
    features.uses_shared_elements = true;
    // Image nodes can be introduced by a hot reload after the development
    // host is built, so preload both local and network image primitives.
    features.facts.ui.image.asset = true;
    features.uses_remote_image = true;
    // Calls to Nexa's async native APIs can appear after the dev host has been
    // built. Keep the same URLSession adapter as release output in that host.
    features.uses_network_api = true;
    // File and Path calls can be introduced by a hot reload after the first
    // native build, so keep the same first-party helpers available in dev.
    features.uses_path_api = true;
    features.uses_file_api = true;
    features.uses_file_async = true;
    features.facts.capabilities.uses_crypto_api = true;
    // JSON is interpreted by NexaDevValueCodec using type descriptors from
    // every hot-reloaded module. Do not emit static JSON helpers for a module
    // that does not declare JsonError or concrete JSON value codecs yet.
    // SecureStorage calls can be added after the dev host is built, so the
    // Keychain adapter must already be part of every development host.
    features.facts.capabilities.uses_secure_storage_api = true;
    features.facts.capabilities.uses_storage_api = true;
    features.facts.capabilities.uses_regex_api = true;
    features.uses_permissions = true;
    features.uses_permission_request = true;
    features.dynamic_permission = true;
    features.used_permissions = [
        nexa_ir::Permission::Camera,
        nexa_ir::Permission::Microphone,
        nexa_ir::Permission::Photos,
        nexa_ir::Permission::Location,
        nexa_ir::Permission::Notifications,
        nexa_ir::Permission::Contacts,
        nexa_ir::Permission::Calendar,
        nexa_ir::Permission::Bluetooth,
        nexa_ir::Permission::Motion,
    ]
    .into_iter()
    .collect();
    features.expose_permissions_to_dev_runtime = true;
    features.uses_native_library = true;
    let project_features = project_features_from_analysis(&features);
    (
        generate_with_analysis_mode(module, features, true, false),
        project_features,
    )
}

fn module_has_native_object_state(module: &Module) -> bool {
    module
        .states
        .iter()
        .chain(
            module
                .screens
                .iter()
                .flat_map(|screen| screen.states.iter()),
        )
        .chain(
            module
                .components
                .iter()
                .flat_map(|component| component.states.iter()),
        )
        .any(State::is_native_class_constructor_binding)
}

#[cfg(test)]
mod tests {
    use super::{
        NEXA_APP_BOTTOM_BAR_PRIMITIVE, NEXA_SHARED_COMPONENT_PRIMITIVES, generate,
        generate_units_with_network_and_project_features, shared_component_primitives,
    };
    use nexa_ir::{
        Action, AnimationSpec, Component, Expr, FastListMove, Function, ImageScale, ImageSource,
        LayoutKind, ListAxis, ListCommon, ListPlan, Module, Node, NumericType, Screen, ScreenId,
        SectionedListCommon, State, SystemIcon, TaskExecutor, TextStyle, Type, ViewStyle,
        ViewTransition, WhenCase,
    };

    #[test]
    fn custom_app_bottom_bar_removes_its_reserved_bottom_safe_area_from_tab_content() {
        assert!(NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("struct NexaAppBottomBarSafeAreaPrimitive"));
        assert!(
            NEXA_APP_BOTTOM_BAR_PRIMITIVE
                .contains("safeAreaSnapshot.removing(Edge.bottom, direction: .leftToRight)")
        );
    }

    #[test]
    fn edge_to_edge_root_uses_full_window_geometry_and_separate_safe_insets() {
        let root = NEXA_SHARED_COMPONENT_PRIMITIVES
            .split("private struct NexaEdgeToEdgeRoot<Content: View>")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(root.contains("GeometryReader { proxy in"));
        assert!(root.contains("let metrics = NexaWindowMetricsSnapshot("));
        assert!(root.contains("content(metrics)"));
        assert!(root.contains(".environment(\\.nexaWindowMetrics, metrics)"));
        assert!(root.contains(".frame(width: proxy.size.width, height: proxy.size.height"));
        assert!(root.contains("NexaSafeAreaInsetsProbe { insets in"));
        assert!(root.contains(".frame(maxWidth: .infinity, maxHeight: .infinity)"));
        assert!(!root.contains(".frame(width: 0, height: 0)"));
        assert!(root.contains(".environment(\\.nexaVisualSafeAreaInsets, containerSafeInsets)"));
        assert!(root.contains(".environment(\\.nexaWindowSafeAreaInsets, containerSafeInsets)"));
        assert!(root.contains(".environment(\\.nexaViewportSize, proxy.size)"));
        assert!(root.contains(".ignoresSafeArea(.container, edges: .all)"));
        assert!(root.contains("left: insets.left"));
        assert!(root.contains("right: insets.right"));
        assert!(!root.contains("layoutDirection == .rightToLeft"));
        assert!(!root.contains("proxy.safeAreaInsets"));
        assert!(!root.contains("proxy.size.height + safeInsets.top + safeInsets.bottom"));
        assert!(!NEXA_SHARED_COMPONENT_PRIMITIVES.contains("NexaSafeAreaInsetsReader"));
        assert!(!root.contains("@State private var safeInsets"));

        assert!(!NEXA_SHARED_COMPONENT_PRIMITIVES.contains("NexaSafeAreaPaddingPrimitive"));
        assert!(NEXA_SHARED_COMPONENT_PRIMITIVES.contains("NexaViewInsetsPrimitive"));

        let safe_area_inset = NEXA_SHARED_COMPONENT_PRIMITIVES
            .split("private struct NexaSafeAreaInsetPrimitive<Content: View, Attached: View>")
            .nth(1)
            .and_then(|source| source.split("private extension Edge {").next())
            .unwrap_or_default();
        assert!(safe_area_inset.contains("safeAreaSnapshot.logical(layoutDirection)"));
        assert!(
            safe_area_inset.contains("safeAreaSnapshot.removing(edge, direction: layoutDirection)")
        );
        let native_safe_area_inset = safe_area_inset
            .split("if accommodatesEndpoint {")
            .nth(1)
            .and_then(|source| source.split("} else {").next())
            .unwrap_or_default();
        assert!(native_safe_area_inset.contains("NexaNativeSafeAreaInset(edge: edge)"));
        assert!(
            native_safe_area_inset
                .contains("NexaSafeAreaInsetsPadding(insets: safeInsets.perpendicular(to: edge))")
        );
        assert!(
            NEXA_SHARED_COMPONENT_PRIMITIVES
                .contains("content.safeAreaInset(edge: .bottom, spacing: 0)")
        );
        assert!(!safe_area_inset.contains("NexaSafeAreaEdgePadding"));
        assert!(!safe_area_inset.contains("let spacing"));

        let probe = NEXA_SHARED_COMPONENT_PRIMITIVES
            .split("private final class NexaSafeAreaInsetsProbeView: UIView {")
            .nth(1)
            .and_then(|source| {
                source
                    .split("private struct NexaSafeAreaInsetsProbe:")
                    .next()
            })
            .unwrap_or_default();
        assert!(probe.contains("window.safeAreaInsets"));
        assert!(probe.contains("safeAreaInsetsDidChange()"));
        assert!(probe.contains("layoutSubviews()"));
        assert!(probe.contains("guard insets != lastReportedInsets else { return }"));

        let dev_runtime = include_str!("../../../../runtime/ios/NexaDevRuntime.swift");
        assert!(dev_runtime.contains("NexaDevEdgeToEdgeRoot {"));
        let dev_root = dev_runtime
            .split("private struct NexaDevEdgeToEdgeRoot<Content: View>")
            .nth(1)
            .and_then(|source| source.split("/// Zero-size probe").next())
            .unwrap_or_default();
        assert!(dev_root.contains(".frame(maxWidth: .infinity, maxHeight: .infinity)"));
        assert!(!dev_root.contains(".frame(width: 0, height: 0)"));
        assert!(!dev_runtime.contains("NexaDevSafeAreaEdgePadding"));
        assert!(
            dev_runtime.contains(".environment(\\.nexaDevVisualSafeInsets, containerSafeInsets)")
        );
        assert!(dev_runtime.contains("\\.nexaWindowSafeAreaInsets,"));
        assert!(dev_runtime.contains("left: insets.left"));
        assert!(dev_runtime.contains("right: insets.right"));
        assert!(!dev_runtime.contains("NexaDevSafeAreaPaddingPrimitive"));
        let dev_renderer = include_str!("../../../../runtime/ios/NexaDevRenderer.swift");
        assert!(dev_renderer.contains("case \"ViewInsets\":"));

        let dev_safe_area_inset = dev_runtime
            .split("struct NexaDevSafeAreaInsetPrimitive<Content: View, Attached: View>: View {")
            .nth(1)
            .and_then(|source| source.split("private extension EdgeInsets {").next())
            .unwrap_or_default();
        assert!(dev_safe_area_inset.contains("safeAreaSnapshot.logical(layoutDirection)"));
        assert!(
            dev_safe_area_inset
                .contains("safeAreaSnapshot.devRemoving(edge, direction: layoutDirection)")
        );
        let dev_native_safe_area_inset = dev_safe_area_inset
            .split("if accommodatesEndpoint {")
            .nth(1)
            .and_then(|source| source.split("} else {").next())
            .unwrap_or_default();
        assert!(dev_native_safe_area_inset.contains("NexaDevNativeSafeAreaInset(edge: edge)"));
        assert!(dev_runtime.contains("content.safeAreaInset(edge: .bottom, spacing: 0)"));
        assert!(dev_native_safe_area_inset.contains(
            "NexaDevSafeAreaInsetsPadding(insets: safeInsets.devPerpendicular(to: edge))"
        ));
        assert!(!dev_safe_area_inset.contains("NexaDevSafeAreaEdgePadding"));

        let primitives = shared_component_primitives(false);
        let form = primitives
            .split("struct NexaFormPrimitive<Content: View>: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        let native_list = primitives
            .split("struct NexaNativeListStyleModifier: ViewModifier {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        let form_margin_viewport = primitives
            .split("private struct NexaScrollMarginViewport<Content: View>: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        let form_padding_viewport = primitives
            .split("private struct NexaScrollViewport<Content: View>: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        assert!(form.contains("NexaScrollMarginViewport(margin: margin)"));
        assert!(form.contains("if margin.top == 0, margin.leading == 0"));
        assert!(form.contains("NexaScrollViewport {"));
        assert!(!form.contains("NexaFormScrollInsetsProbe"));
        assert!(!form.contains(".padding(margin)"));
        assert!(form_margin_viewport.contains(".ignoresSafeArea(.container, edges: .all)"));
        assert!(
            form_margin_viewport
                .contains("width: max(0, proxy.size.width - margin.leading - margin.trailing)")
        );
        assert!(
            form_margin_viewport
                .contains("height: max(0, proxy.size.height - margin.top - margin.bottom)")
        );
        assert!(form_margin_viewport.contains(".padding(margin)"));
        assert!(form_margin_viewport.contains("NexaScrollMarginHostedContent("));
        assert!(
            form_margin_viewport.contains(
                r"content: NexaScrollViewport { content.environment(\.self, environment) }"
            )
        );
        assert!(form_margin_viewport.contains("view.clipsToBounds = true"));
        assert!(form_padding_viewport.contains(".ignoresSafeArea(.container, edges: .all)"));
        assert!(
            form_padding_viewport
                .contains("frame(width: proxy.size.width, height: proxy.size.height")
        );
        assert!(
            form_margin_viewport.find("height: max(0").unwrap()
                < form_margin_viewport.find(".padding(margin)").unwrap()
        );
        assert!(native_list.contains(".ignoresSafeArea(.container, edges: .all)"));
        assert!(
            form.contains(
                ".contentMargins(.leading, effectivePadding.leading, for: .scrollContent)"
            )
        );
        assert!(form.contains(
            ".contentMargins(.trailing, effectivePadding.trailing, for: .scrollContent)"
        ));
        assert!(form.contains(
            ".environment(\\.nexaFormTopPadding, firstChildIsSection ? effectivePadding.top : 0)"
        ));
        assert!(!form.contains("marginSafeAreaCompensation"));
        assert!(!form.contains("nexaFormTopMarginCompensation"));
        assert!(!form.contains("logicalWindowSafeAreaInsets"));
        assert!(!form.contains("NexaScrollContentSafeAreaModifier("));
        assert!(!form.contains("form.contentMargins(.vertical, 0, for: .scrollContent)"));
        assert!(form.contains("if #available(iOS 17.0, *)"));
        assert!(form.contains("if !firstChildIsSection, effectivePadding.top != 0"));
        assert!(form.contains(".frame(height: effectivePadding.top)"));
        assert!(form.contains(".frame(height: effectivePadding.bottom)"));
        let form_section = primitives
            .split("struct NexaFormSectionPrimitive<Content: View, Header: View, Footer: View>: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        assert!(form_section.contains(".padding(.top, firstHeaderTopPadding)"));
        assert!(form_section.contains("isFirstSection ? formTopPadding : 0"));
        assert!(!form_section.contains("listSectionMargins(.top"));
        assert!(!form_section.contains("formSectionTopPadding"));
        assert!(!primitives.contains("nexaFormNativeTopContentMargin"));
        assert!(native_list.contains("NexaScrollViewport { content }"));
        assert!(!native_list.contains("windowSafeAreaInsets"));
        assert!(root.contains(".ignoresSafeArea(.container, edges: .all)"));

        let list_runtime = include_str!("components/list_runtime.rs");
        assert!(list_runtime.contains("func nexaConfigureEdgeToEdgeScrollView("));
        assert!(list_runtime.contains(
            "nexaConfigureEdgeToEdgeScrollView(tableView, environment: context.environment, reverseLayout: reverseLayout)"
        ));
        assert!(list_runtime.contains(
            "nexaConfigureEdgeToEdgeScrollView(collectionView, environment: context.environment)"
        ));
        assert!(list_runtime.contains("scrollView.contentInsetAdjustmentBehavior = .never"));
        assert!(!list_runtime.contains(
            "contentInsetAdjustmentBehavior = pageSnap || reverseLayout ? .never : .automatic"
        ));
        let dev_renderer = include_str!("../../../../runtime/ios/NexaDevRenderer.swift");
        assert!(!dev_renderer.contains(
            ".scrollDisabled(!scrollEnabled)\n        .ignoresSafeArea(.container, edges: .all)"
        ));
        let form_insets_dispatch = dev_renderer
            .split("case \"ViewInsets\":")
            .nth(1)
            .and_then(|source| source.split("case \"SafeAreaInset\":").next())
            .unwrap_or_default();
        assert!(form_insets_dispatch.contains("taggedChild[\"Form\"] != nil"));
        assert!(form_insets_dispatch.contains("formContentPadding: padding"));
        assert!(form_insets_dispatch.contains("formMargin: margin"));
        assert!(dev_renderer.contains("firstFormSection: firstChildIsSection"));
        assert!(dev_renderer.contains("firstFormSection && index == 0"));
        assert!(dev_renderer.contains("isFirstSection: isFirstFormSection"));
        assert!(dev_renderer.contains("contentPadding: formContentPadding"));
        assert!(dev_renderer.contains("margin: formMargin"));
        assert!(dev_runtime.contains(".ignoresSafeArea(.container, edges: .all)"));
    }

    fn braced_block(source: &str, start: usize) -> &str {
        let open = source[start..]
            .find('{')
            .map(|offset| start + offset)
            .expect("generated block has an opening brace");
        let mut depth = 0usize;
        for (offset, character) in source[open..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &source[start..open + offset + character.len_utf8()];
                    }
                }
                _ => {}
            }
        }
        panic!("generated block is closed");
    }

    #[test]
    fn native_plugin_network_requirement_emits_shared_urlsession_runtime() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PluginNetworkApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let (sources, features) = generate_units_with_network_and_project_features(&module);
        let network = sources
            .units
            .iter()
            .find(|unit| unit.name.contains("native_library"))
            .expect("plugin host requirement should emit the network runtime");
        assert!(features.requires_file_timestamp_reason);
        assert!(network.contents.contains("public enum NexaNetwork"));
        assert!(network.contents.contains("sharedSession"));
    }

    #[test]
    fn segmented_control_reaches_the_unified_native_bridge() {
        let primitives = nexa_codegen::source_sections::reachable(
            NEXA_SHARED_COMPONENT_PRIMITIVES,
            "NexaSegmentedControlPrimitive()",
        );

        assert!(
            primitives.contains("private struct NexaNativeSegmentedControl: UIViewRepresentable")
        );
        assert!(primitives.contains("control.setTitleTextAttributes"));
        assert!(primitives.contains("control.layer.backgroundColor = color.cgColor"));
        assert!(primitives.contains("control.selectedSegmentTintColor = tint.map { UIColor($0) }"));
    }

    #[test]
    fn reusable_component_slots_keep_concrete_generic_view_content() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ReusableComponentApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: vec![Component {
                name: "ReusableCard".to_owned(),
                source_file: None,
                parameters: Vec::new(),
                states: Vec::new(),
                body: vec![Node::Content],
                ref_methods: Vec::new(),
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            body: vec![Node::ComponentCall {
                name: "ReusableCard".to_owned(),
                arguments: Vec::new(),
                component_ref: None,
                children: Some(vec![Node::Text {
                    value: Expr::String("Card body".to_owned()),
                    style: TextStyle::default(),
                }]),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("struct NexaReusableCardComponent<SlotContent: View>: View"));
        assert!(swift.contains("private let nexaContent: () -> SlotContent"));
        assert!(swift.contains("@ViewBuilder nexaContent: @escaping () -> SlotContent"));
        assert!(swift.contains("NexaReusableCardComponent() {"));
        assert!(
            !swift.contains("AnyView"),
            "reusable component slots must keep their concrete view type"
        );
    }

    #[test]
    fn shared_button_primitive_embeds_design_system_geometry() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains("struct NexaButtonPrimitive<Label: View>"));
        assert!(primitives.contains("let accessibilityLabel: String?"));
        assert!(primitives.contains("view.accessibilityLabel(Text(accessibilityLabel))"));
        assert!(primitives.contains("private struct NexaSharedProminentButtonStyle: ButtonStyle"));
        assert!(primitives.contains("NexaSharedProminentButtonStyle("));
        assert!(primitives.contains("case .regular, nil: (CGFloat(0), CGFloat(0))"));
        assert!(primitives.contains("private var plainLabelContent: some View"));
        assert!(
            primitives
                .contains(".foregroundStyle((disabled || loading) ? tint.opacity(0.38) : tint)")
        );
        assert!(primitives.contains("width: CGFloat(20),\n                height: CGFloat(20)"));
        assert!(!primitives.contains("CGFloat(8)"));
        assert!(primitives.contains("struct NexaFormPrimitive<Content: View>"));
        assert!(primitives.contains(
            "@Environment(\\.multilineTextAlignment) private var inheritedTextAlignment"
        ));
        assert!(primitives.contains(".multilineTextAlignment(inheritedTextAlignment)"));
        assert!(primitives.contains(
            "struct NexaFormSectionPrimitive<Content: View, Header: View, Footer: View>"
        ));
        assert!(primitives.contains(".listSectionSeparator(.hidden)"));
        assert!(primitives.contains("CGFloat(64)"));
        assert!(primitives.contains("CGFloat(48)"));
        assert!(primitives.contains("CGFloat(50)"));
        assert!(primitives.contains("CGFloat(12)"));
        assert!(primitives.contains("CGFloat(20)"));
        assert!(
            !primitives.contains("__NEXA_"),
            "unresolved design token in helper source"
        );
    }

    #[test]
    fn date_picker_uses_native_controls_and_a_committable_range_sheet() {
        let primitives = shared_component_primitives(false);
        let date_picker = primitives
            .split("struct NexaDatePickerPrimitive: View {")
            .nth(1)
            .unwrap_or_default();

        assert!(date_picker.contains("DatePicker(\"\", selection: selectedDate"));
        assert!(date_picker.contains(".labelsHidden()"));
        assert!(
            date_picker.contains(
                "MultiDatePicker(\"\", selection: rangeDraftBinding, in: rangeDateRange)"
            )
        );
        assert!(
            date_picker
                .contains("MultiDatePicker(\"\", selection: selectedRange, in: rangeDateRange)")
        );
        assert!(date_picker.contains("private var rangeDateRange: Range<Date>"));
        assert!(
            date_picker
                .contains("pickerCalendar.date(byAdding: .day, value: 1, to: pickerCalendar.startOfDay(for: date(for: value)))")
        );
        assert!(date_picker.contains("let selectedDays = Set("));
        assert!(date_picker.contains("selectedDays.symmetricDifference(previousDays)"));
        assert!(date_picker.contains("selectedDays.contains(changed)"));
        assert!(date_picker.contains("@State private var rangeAnchor: Date?"));
        assert!(!date_picker.contains("selection.contains(changed)"));
        assert!(date_picker.contains(".sheet(isPresented: $isRangePresented)"));
        assert!(date_picker.contains("Button(\"Cancel\")"));
        assert!(date_picker.contains("Button(\"Done\")"));
        assert!(date_picker.contains(".accessibilityValue(rangeSummary)"));
        assert!(date_picker.contains(".disabled(rangeDraft.count < 2)"));
        assert!(date_picker.contains("private func commitRangeSelection()"));
        assert!(
            date_picker.contains("private func updateRangeDraft(_ selection: Set<DateComponents>)")
        );
        assert!(date_picker.contains(".presentationDetents([.medium, .large])"));
        assert!(date_picker.contains(".padding(.top, 24)"));
        assert!(!date_picker.contains("Text(rangeDraftSummary)"));
        assert!(date_picker.contains("Capsule()"));
        assert!(date_picker.contains(".padding(.horizontal, 16)"));
        assert!(
            date_picker.contains(".frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)")
        );
        assert!(date_picker.contains("Color.secondary.opacity(0.38)"));
        assert!(!date_picker.contains("LazyVGrid("));
        assert!(date_picker.contains(".datePickerStyle(.graphical)"));
        assert!(date_picker.contains(".datePickerStyle(.wheel)"));
        assert!(date_picker.contains("case \"Compact\", \"Input\":"));
        assert!(date_picker.contains(".datePickerStyle(.compact)"));
        assert!(date_picker.contains(".datePickerStyle(.automatic)"));
        assert!(!date_picker.contains(".popover("));
        assert!(!date_picker.contains(".accessibilityLabel(accessibilityLabel ?? \"\")"));
        assert!(!date_picker.contains("DateFormatter()"));
    }

    #[test]
    fn refresh_bounce_modifier_allows_short_content_to_trigger_pull_to_refresh() {
        let primitives = super::NEXA_SHARED_COMPONENT_PRIMITIVES;

        assert!(primitives.contains("private struct NexaRefreshBounceModifier: ViewModifier"));
        assert!(primitives.contains("if #available(iOS 16.4, *)"));
        assert!(primitives.contains(".scrollBounceBehavior(.always, axes: .vertical)"));
    }

    #[test]
    fn shared_text_input_fills_parent_and_keeps_text_leading_aligned() {
        let primitives = shared_component_primitives(false);
        let input = primitives
            .split("struct NexaTextInputPrimitive: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(input.contains(".frame(maxWidth: .infinity, alignment: .leading)"));
        assert!(input.contains(".multilineTextAlignment(.leading)"));
        assert!(input.contains("let onSubmitAction: (() -> Void)?"));
        assert!(input.contains("onSubmitAction?()"));
        assert!(input.contains("leadingField"));
        assert!(input.contains("clearable && !text.isEmpty"));
        assert!(input.contains("xmark.circle.fill"));
        assert!(input.contains(".font(explicitFont)"));
        assert!(input.contains(".foregroundColor(textColor)"));
    }

    #[test]
    fn status_bar_background_uses_the_root_geometry_inset() {
        let primitives = shared_component_primitives(false);
        let status_bar = primitives
            .split("struct NexaStatusBarBackgroundPrimitive: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(
            status_bar.contains("@Environment(\\.nexaVisualSafeAreaInsets) private var safeInsets")
        );
        assert!(status_bar.contains(".frame(height: max(0, safeInsets.top))"));
        assert!(!primitives.contains("UIApplication.shared.connectedScenes"));
        assert!(status_bar.contains(".ignoresSafeArea(edges: .top)"));
    }

    #[test]
    fn status_bar_style_uses_a_status_bar_only_appearance_api() {
        let primitives = shared_component_primitives(false);
        let status_bar_style = primitives
            .split("struct NexaStatusBarStylePrimitive: ViewModifier {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(status_bar_style.contains("if #available(iOS 27.0, *)"));
        assert!(status_bar_style.contains(".toolbarColorScheme(colorScheme, for: .statusBar)"));
        assert!(status_bar_style.contains("content\n        }"));
        assert!(!status_bar_style.contains(".preferredColorScheme("));
    }

    #[test]
    fn split_view_uses_visible_single_column_on_compact_ios() {
        let primitives = shared_component_primitives(false);
        let split_view = primitives
            .split("struct NexaNavigationSplitViewPrimitive<Sidebar: View, Detail: View>: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(
            split_view
                .contains("@Environment(\\.horizontalSizeClass) private var horizontalSizeClass")
        );
        assert!(split_view.contains("if horizontalSizeClass == .compact"));
        assert!(split_view.contains("if detailVisible {"));
        assert!(
            split_view
                .contains("detail\n                        .transition(.move(edge: .trailing))")
        );
        assert!(
            split_view
                .contains("sidebar\n                        .transition(.move(edge: .leading))")
        );
        assert!(split_view.contains(".animation(.default, value: detailVisible)"));
        assert!(split_view.contains("NavigationSplitView(preferredCompactColumn:"));
    }

    #[test]
    fn form_section_heading_uses_the_secondary_label_style() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains(".foregroundStyle(.secondary)"));
        let section = primitives
            .split("struct NexaFormSectionPrimitive<Content: View, Header: View, Footer: View>")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        assert!(section.contains(".listSectionSeparator(.hidden)"));
        assert!(section.contains(".listRowBackground(background ?? Color.clear)"));
        assert!(!section.contains("listSectionMargins(.top"));
        assert!(!section.contains("formSectionTopPadding"));
    }

    #[test]
    fn text_alignment_inherits_column_default_when_not_explicit() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains("if let alignment = style.alignment"));
        assert!(primitives.contains("styledText.multilineTextAlignment(alignment)"));
        assert!(!primitives.contains("style.alignment ?? .leading"));
        assert!(primitives.contains(".multilineTextAlignment(textAlignment)"));
    }

    #[test]
    fn shared_picker_primitive_owns_all_native_picker_variants() {
        let primitives = shared_component_primitives(false);
        assert!(primitives.contains("struct NexaPickerPrimitive<LabelContent: View>"));
        assert!(primitives.contains("func nexaPickerTint(_ tint: Color?)"));
        assert!(primitives.contains(".nexaPickerTint(tint)"));
        assert!(primitives.contains("if hasLabel {"));
        assert!(primitives.contains("} else if let icon {"));
        let labeled_picker = primitives
            .split("} else if let icon {")
            .next()
            .unwrap_or_default();
        assert!(
            labeled_picker.contains("Menu {\n                pickerOptions(selection: $selection)")
        );
        assert!(!labeled_picker.contains(".pickerStyle(.inline)"));
        assert!(!labeled_picker.contains(".popover("));
        assert!(labeled_picker.contains(".labelsHidden()"));
        assert!(!labeled_picker.contains("labelsVisibility"));
        assert!(!labeled_picker.contains("labeledMenuOptions"));
        assert!(labeled_picker.contains(
            "NexaIconView(icon: itemIcons[index], description: \"\", size: 16, tint: .secondary)"
        ));
        assert!(labeled_picker.contains(".foregroundStyle(resolvedTint)"));
        assert!(labeled_picker.contains(".contentShape(Rectangle())"));
        assert!(labeled_picker.contains(".frame(maxWidth: .infinity, alignment: .leading)"));
        assert!(labeled_picker.contains("Text(selection)"));
        assert!(labeled_picker.contains("Spacer(minLength: 12)"));
        assert!(primitives.contains(".pickerStyle(.menu)"));
        assert!(primitives.contains(".accessibilityValue(selection)"));
    }

    #[test]
    fn shared_switch_uses_the_cross_platform_system_blue_accent() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains(".tint(Color(uiColor: .systemBlue))"));
    }

    #[test]
    fn shared_progress_ring_renders_the_determinate_value() {
        let primitives = shared_component_primitives(false);
        let ring = primitives
            .split("// nexa-codegen: NexaProgressRingPrimitive")
            .nth(1)
            .unwrap_or_default();

        assert!(ring.contains("private var normalizedProgress: Double?"));
        assert!(ring.contains("guard let progress else { return nil }"));
        assert!(ring.contains("guard progress.isFinite else { return 0 }"));
        assert!(ring.contains(".trim(from: 0, to: normalizedProgress)"));
        assert!(ring.contains("if normalizedProgress >= 1"));
        assert!(ring.contains("private var indicatorSize: CGFloat { max(size ?? 40, 0) }"));
        assert!(ring.contains(
            "private var indicatorThickness: CGFloat { min(max(thickness ?? 4, 0), indicatorSize) }"
        ));
        assert!(ring.contains(
            "private var pathSize: CGFloat { max(indicatorSize - indicatorThickness, 0) }"
        ));
        assert!(ring.contains(".trim(from: 0, to: 0.25)"));
        assert!(ring.contains(".frame(width: pathSize, height: pathSize)"));
        assert!(ring.contains(".frame(width: indicatorSize, height: indicatorSize)"));
        assert!(ring.contains("trackColor ?? indicatorColor.opacity(0.16)"));
        assert!(ring.contains("let tint: Color?"));
        assert!(ring.contains("let trackColor: Color?"));
        assert!(ring.contains(".accessibilityLabel(\"Progress\")"));
        assert!(ring.contains(".accessibilityValue(normalizedProgress.map"));
        assert!(!ring.contains(".progressViewStyle(.circular)"));
    }

    #[test]
    fn shared_progress_bar_uses_native_determinate_and_indeterminate_styles() {
        let primitives = shared_component_primitives(false);
        let bar = primitives
            .split("// nexa-codegen: NexaProgressBarPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaProgressRingPrimitive")
            .next()
            .unwrap_or_default();

        assert!(bar.contains("private var normalizedProgress: Double?"));
        assert!(bar.contains("guard let progress else { return nil }"));
        assert!(bar.contains("guard progress.isFinite else { return 0 }"));
        assert!(bar.contains("ProgressView(value: normalizedProgress, total: 1)"));
        assert!(bar.contains("private func indeterminateBar(tint: Color) -> some View"));
        assert!(bar.contains("let shouldAnimate = isVisible && !reduceMotion"));
        assert!(bar.contains(".accessibilityRepresentation { ProgressView() }"));
        assert!(bar.contains("if let normalizedProgress {"));
        assert!(bar.contains("if trackColor == nil && thickness == nil && cornerRadius == nil"));
        assert!(bar.contains("ProgressView(value: normalizedProgress, total: 1).tint(tint)"));
        assert!(bar.contains("trackColor ?? Color.primary.opacity(0.1)"));
        assert!(bar.contains("tint ?? Color.accentColor"));
        assert!(bar.contains("cornerRadius == 0 && !reduceMotion ? .linear(duration: 0.25) : nil"));
    }

    #[test]
    fn shared_presentation_primitives_use_native_swiftui_sheets() {
        let primitives = shared_component_primitives(false);
        let bottom_sheet = primitives
            .split("// nexa-codegen: NexaBottomSheetPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaAlertDialogPrimitive")
            .next()
            .unwrap_or_default();
        assert!(bottom_sheet.contains("struct NexaBottomSheetPrimitive<Content: View>"));
        assert!(bottom_sheet.contains("NexaBottomSheetCornerRadius(cornerRadius: cornerRadius)"));
        assert!(bottom_sheet.contains("content.presentationCornerRadius(cornerRadius)"));
        assert!(bottom_sheet.contains("sheet.preferredCornerRadius = cornerRadius"));
        assert!(bottom_sheet.contains(
            "contentPadding: contentPadding,\n                    contentSized: contentSized"
        ));
        assert!(bottom_sheet.contains("backgroundColor ?? Color(uiColor: .systemBackground)"));
        assert!(
            bottom_sheet.contains("private struct NexaBottomSheetContentPrimitive<Content: View>")
        );
        assert!(bottom_sheet.contains("NexaColumnPrimitive(spacing: 0)"));
        assert!(bottom_sheet.contains(".padding(contentPadding)"));
        assert!(!bottom_sheet.contains(".padding(.top"));
        assert!(
            bottom_sheet
                .contains(".frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)")
        );
        assert!(bottom_sheet.contains(".sheet(isPresented: $isPresented, onDismiss: {"));
        assert!(bottom_sheet.contains(
            "Color.clear\n            .frame(width: 0, height: 0)\n            .sheet(isPresented:"
        ));
        assert!(bottom_sheet.contains("onDismiss()"));
        assert!(
            bottom_sheet.contains(
                "NexaBottomSheetBackgroundInteraction(\n                        allowsBackgroundInteraction: allowsBackgroundInteraction\n                    )"
            )
        );
        assert!(bottom_sheet.contains("content.presentationBackgroundInteraction(.enabled)"));
        assert!(bottom_sheet.contains("if #available(iOS 16.4, *), allowsBackgroundInteraction"));
        assert!(!bottom_sheet.contains("scrimColor == .clear"));
        assert!(bottom_sheet.contains("let detents: Set<PresentationDetent>"));
        assert!(bottom_sheet.contains(
            ".presentationDetents(\n                        contentSized ? [.height(measuredContentHeight)] : detents"
        ));
        assert!(bottom_sheet.contains("@State private var measuredContentHeight: CGFloat = 1"));
        assert!(bottom_sheet.contains("NexaBottomSheetContentHeightKey.self"));
        assert!(bottom_sheet.contains(".fixedSize(horizontal: false, vertical: true)"));
        assert!(
            bottom_sheet
                .contains(".presentationDragIndicator(showDragIndicator ? .visible : .hidden)")
        );
        assert!(bottom_sheet.contains(".interactiveDismissDisabled(!allowsDismissal)"));
        assert!(bottom_sheet.contains("content.presentationBackground(color)"));
        assert!(bottom_sheet.contains("selectedDetent = contentSized"));
        assert!(!bottom_sheet.contains("fullScreenCover"));
        assert!(!bottom_sheet.contains("keyboardWillShowNotification"));
        assert!(!bottom_sheet.contains("detentBeforeKeyboard"));

        assert!(primitives.contains("host.alert(title, isPresented: $isPresented)"));
        assert!(primitives.contains(".confirmationDialog(title, isPresented: $isPresented"));
    }

    #[test]
    fn shared_page_indicator_owns_dot_geometry_color_and_accessibility() {
        let primitives = shared_component_primitives(false);
        assert!(primitives.contains("struct NexaPageIndicatorPrimitive: View"));
        assert!(primitives.contains("Color(uiColor: .systemBlue)"));
        assert!(primitives.contains("width: selectedIndex == index ? 8 : 6"));
        assert!(primitives.contains(".accessibilityLabel(\"Page \\(index + 1)\")"));
        assert!(primitives.contains("struct NexaPageIndicatorRow: View"));
        assert!(primitives.contains("HStack(spacing: 8)"));
        assert!(primitives.contains(".padding(.bottom, 24)"));
    }

    #[test]
    fn divider_primitive_renders_the_requested_thickness() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains("Rectangle()"));
        assert!(primitives.contains(".fill(color)"));
        assert!(primitives.contains(".frame(height: max(0, thickness))"));
    }

    #[test]
    fn text_line_height_uses_total_height_semantics_with_a_legacy_fallback() {
        let primitives = shared_component_primitives(true);

        assert!(primitives.contains("content.lineHeight(.multiple(factor: factor))"));
        assert!(primitives.contains("UIFont.systemFont(ofSize: baseFontSize).lineHeight"));
        assert!(!primitives.contains("__NEXA_TEXT_LINE_HEIGHT_MODIFIER__"));
    }

    fn regex_module(enabled: bool) -> Module {
        Module {
            widgets: Vec::new(),
            app_name: "RegexUsage".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: if enabled {
                vec![Function {
                    name: "hasDigits".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    failure_type: None,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    return_type: Type::Bool,
                    body: Expr::NativeCall {
                        receiver: None,
                        namespace: "Regex".to_owned(),
                        name: "isMatch".to_owned(),
                        arguments: Vec::new(),
                        codecs: Vec::new(),
                        return_type: Type::Bool,
                        source_span: None,
                        is_async: false,
                        is_throwing: false,
                    },
                    body_actions: None,
                }]
            } else {
                Vec::new()
            },
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Regex capability probe".to_owned()),
                style: TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        }
    }

    #[test]
    fn regex_runtime_helpers_are_emitted_only_when_the_api_is_used() {
        let without_regex = generate(&regex_module(false));
        let with_regex = generate(&regex_module(true));
        let with_regex_units = super::generate_units(&regex_module(true));

        assert!(!without_regex.contains("nexaRegexIsMatch"));
        assert!(with_regex.contains("nexaRegexIsMatch"));
        assert_eq!(
            with_regex_units
                .units
                .iter()
                .map(|unit| unit.contents.matches("struct NexaRegex {").count())
                .sum::<usize>(),
            1,
            "shared Regex types must be emitted once across separate Swift files"
        );
    }

    #[test]
    fn task_cleanup_modifier_wraps_a_top_level_conditional_in_group() {
        let task_handle_type = Type::Optional(Box::new(Type::TaskHandle));
        let module = Module {
            widgets: Vec::new(),
            app_name: "ConditionalTaskCleanup".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "refreshTask".to_owned(),
                ty: task_handle_type.clone(),
                initial: Expr::Null(task_handle_type),
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::If {
                condition: Expr::Bool(true),
                then_body: vec![Node::Text {
                    value: Expr::String("Ready".to_owned()),
                    style: TextStyle::default(),
                }],
                else_body: None,
                transition: None,
                transition_animation: None,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(
            swift.contains("            Group {\n                if true {"),
            "the conditional body needs a concrete view type before shared modifiers are applied"
        );
        assert!(
            swift.contains(".onDisappear {\n                    nexa_refreshTask?.cancel()"),
            "task scope cleanup must apply to the full conditional view"
        );
    }

    #[test]
    fn pressable_context_menu_emits_native_swiftui_actions() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ContextMenuApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Row".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: vec![Node::Button {
                    label: Expr::LocalizedText {
                        key: "Edit".to_owned(),
                        value: Box::new(Expr::String("Edit".to_owned())),
                        comment: Some("Context menu action".to_owned()),
                    },
                    icon: Some(SystemIcon::Shared("edit".to_owned())),
                    loading: None,
                    disabled: None,
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    accessibility_label: None,
                    actions: Vec::new(),
                }],
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains(".contextMenu {"));
        assert!(swift.contains(
            "Label { Text(\"Edit\", comment: \"Context menu action\") } icon: { Image(systemName: \"pencil\") }"
        ));
    }

    #[test]
    fn appearance_wraps_content_in_native_swiftui_color_scheme() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "AppearanceApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Appearance {
                mode: Expr::String("dark".to_owned()),
                children: vec![Node::Text {
                    value: Expr::String("Hello".to_owned()),
                    style: TextStyle::default(),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let swift = generate(&module);
        assert!(swift.contains("NexaAppearancePrimitive(mode: \"dark\")"));
        assert!(swift.contains(".environment(\\.colorScheme, resolvedColorScheme)"));
        assert!(swift.contains(".preferredColorScheme(preferredColorScheme)"));
        assert!(swift.contains("private var preferredColorScheme: ColorScheme?"));
        assert!(swift.contains("Text(\"Hello\")"));
        // AOT output includes only primitives reachable from the app body.
        assert!(!swift.contains("struct NexaButtonPrimitive<Label: View>"));
        let dev_sources = super::generate_for_dev_units(&module);
        let swift_dev = dev_sources
            .units
            .iter()
            .map(|unit| unit.contents.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(swift_dev.contains("struct NexaButtonPrimitive<Label: View>"));
        assert!(!swift.contains("nexaColor(hex:"));
        assert!(!swift.contains("nexaLargeTitleDisplayMode()"));
    }

    #[test]
    fn semantic_text_font_roles_emit_scalable_swiftui_styles() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "SemanticTextStyle".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Task description".to_owned()),
                style: TextStyle {
                    font_style: Some(nexa_ir::TextFontStyle::Subheadline),
                    ..TextStyle::default()
                },
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        assert!(generate(&module).contains(".withFont(.subheadline)"));
    }

    #[test]
    fn dynamic_color_fallback_uses_the_shared_default_accent() {
        let helper = super::NEXA_DYNAMIC_COLOR_HELPER.replace(
            "__NEXA_SWIFT_DEFAULT_ACCENT_COLOR__",
            nexa_codegen::design_system::SWIFT_DEFAULT_ACCENT_COLOR,
        );

        assert!(!helper.contains("__NEXA_"));
        assert!(helper.contains("return Color(uiColor: .systemBlue)"));
    }

    #[test]
    fn page_snap_lists_use_viewport_rows_and_report_the_settled_page() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PageSnap".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "currentPage".to_owned(),
                ty: Type::Numeric(NumericType::Int32),
                initial: Expr::Number {
                    raw: "0".to_owned(),
                    ty: NumericType::Int32,
                },
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::FastList {
                plan: Box::new(ListPlan::Count {
                    count: Expr::Number {
                        raw: "3".to_owned(),
                        ty: NumericType::Int32,
                    },
                    common: ListCommon {
                        axis: ListAxis::Vertical,
                        show_dividers: true,
                        reverse_layout: false,
                        page_snap: true,
                        item_extent: None,
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: None,
                        component_ref: None,
                        children: vec![Node::Text {
                            value: Expr::String("Feed page".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_scroll_begin_drag: None,
                        on_move: None,
                        swipe_actions: None,
                        sticky_header: None,
                        refresh: None,
                        gap: 0.0,
                        scroll_enabled: None,
                    },
                }),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("pageSnap: true"));
        assert!(swift.contains("onScroll: nil,"));
        let page_snap_call = swift
            .split("NexaSwiftUIPageSnapList(")
            .nth(1)
            .expect("iOS 17 page-snapping list should be emitted");
        let scroll_enabled = page_snap_call
            .find("scrollEnabled: scrollEnabled")
            .expect("page-snapping list should forward scrollEnabled");
        let scroll_position_changed = page_snap_call
            .find("onScrollPositionChanged: onScrollPositionChanged")
            .expect("page-snapping list should forward its position callback");
        let refresh = page_snap_call
            .find("onRefresh: onRefresh")
            .expect("page-snapping list should forward its refresh callback");
        assert!(
            scroll_enabled < scroll_position_changed && scroll_position_changed < refresh,
            "the generated initializer call must follow the Swift parameter declaration order"
        );
        assert!(swift.contains("tableView.isPagingEnabled = pageSnap"));
        assert!(swift.contains("tableView.rowHeight = height"));
        assert!(swift.contains("height: pageSnap ? pageHeight : nil"));
        assert!(
            swift.contains("return pageHeight > 0 ? pageHeight : max(tableView.bounds.height, 1)")
        );
        assert!(swift.contains("func scrollViewDidEndDecelerating(_ scrollView: UIScrollView)"));
        assert!(swift.contains("guard !pageSnap else { return }"));
        assert!(!swift.contains("onScroll?()"));

        let mut reverse_module = module.clone();
        let Node::FastList { plan } = &mut reverse_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_mut() else {
            panic!("expected the count-backed FastList plan");
        };
        common.page_snap = false;
        common.reverse_layout = true;
        let reverse_swift = generate(&reverse_module);
        assert!(reverse_swift.contains(
            "nexaConfigureEdgeToEdgeScrollView(tableView, environment: context.environment, reverseLayout: reverseLayout)"
        ));
        assert!(!reverse_swift.contains(
            "contentInsetAdjustmentBehavior = pageSnap || reverseLayout ? .never : .automatic"
        ));
        assert!(reverse_swift.contains("let row = indexPath.row"));
        assert!(
            !reverse_swift
                .contains("let row = reverseLayout ? rowCount - 1 - indexPath.row : indexPath.row")
        );
        assert!(
            reverse_swift.contains("coordinator.applyScrollPosition(to: tableView, force: true)")
        );
        assert!(
            reverse_swift.contains(
                "func applyScrollPosition(to tableView: UITableView, force: Bool = false)"
            )
        );
        assert!(reverse_swift.contains("func tableView(_ tableView: UITableView, estimatedHeightForRowAt indexPath: IndexPath) -> CGFloat"));
        assert!(reverse_swift.contains(
            "nexaConfigureEdgeToEdgeScrollView(tableView, environment: context.environment, reverseLayout: reverseLayout)"
        ));
        assert!(!reverse_swift.contains(
            "contentInsetAdjustmentBehavior = pageSnap || reverseLayout ? .never : .automatic"
        ));

        let mut paginated_module = module.clone();
        let Node::FastList { plan } = &mut paginated_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_mut() else {
            panic!("expected the count-backed FastList plan");
        };
        common.on_end_reached = Some(vec![Action::TaskLaunch {
            handle: Some("loadTask".to_owned()),
            executor: TaskExecutor::Main,
            actions: vec![Action::Expression(Expr::Bool(true))],
        }]);
        paginated_module.states.push(State {
            name: "loadTask".to_owned(),
            ty: Type::Optional(Box::new(Type::TaskHandle)),
            initial: Expr::Null(Type::Optional(Box::new(Type::TaskHandle))),
            mutable: true,
        });
        let paginated_swift = generate(&paginated_module);
        assert!(paginated_swift.contains("NexaTaskScopeView { nexaTaskScope in"));
        assert!(
            paginated_swift.contains("nexa_loadTask = nexaTaskScope.track(Task { @MainActor in")
        );
        assert!(paginated_swift.contains(
            "content(nexaTaskScope)\n            .onDisappear { nexaTaskScope.cancelAll() }"
        ));
    }

    #[test]
    fn swipe_action_lists_select_swiftui_list_without_the_custom_uikit_runtime() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "NativeList".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "moveEnabled".to_owned(),
                ty: Type::Bool,
                initial: Expr::Bool(true),
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::FastList {
                plan: Box::new(ListPlan::Items {
                    collection: Expr::State(
                        "tasks".to_owned(),
                        Type::Array(Box::new(Type::String)),
                    ),
                    element_type: Type::String,
                    item: "task".to_owned(),
                    common: ListCommon {
                        axis: ListAxis::Vertical,
                        show_dividers: true,
                        reverse_layout: false,
                        page_snap: false,
                        item_extent: None,
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: None,
                        component_ref: None,
                        children: vec![Node::Text {
                            value: Expr::String("Task".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_scroll_begin_drag: None,
                        on_move: Some(FastListMove {
                            from: "from".to_owned(),
                            to: "to".to_owned(),
                            enabled: Expr::State("moveEnabled".to_owned(), Type::Bool),
                            actions: Vec::new(),
                        }),
                        swipe_actions: Some(vec![Node::Button {
                            label: Expr::String("Delete".to_owned()),
                            icon: None,
                            loading: None,
                            disabled: None,
                            style: None,
                            size: None,
                            shape: None,
                            tint: None,
                            glass: false,
                            accessibility_label: None,
                            actions: Vec::new(),
                        }]),
                        sticky_header: None,
                        refresh: None,
                        gap: 0.0,
                        scroll_enabled: None,
                    },
                }),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("List {"));
        assert!(swift.contains(
            ".contentMargins(\n                .horizontal,\n                CGFloat(0),\n                for: .scrollContent\n            )"
        ));
        assert!(swift.contains(
            ".contentMargins(\n                .vertical,\n                EdgeInsets(\n                    top: padding.top + CGFloat(16),\n                    leading: 0,\n                    bottom: padding.bottom + CGFloat(16),\n                    trailing: 0\n                ),\n                for: .scrollContent\n            )"
        ));
        let shared_primitives = shared_component_primitives(false);
        assert!(shared_primitives.contains(
            "groupedList\n                .padding(\n                    .horizontal,\n                    -CGFloat(16)\n                )"
        ));
        assert!(swift.contains("leading: CGFloat(16)"));
        assert!(swift.contains("trailing: CGFloat(16)"));
        assert!(!swift.contains("NexaFastList("));
        assert!(swift.contains(
            "private func isHorizontalPageContainer(_ scrollView: UIScrollView) -> Bool"
        ));
        assert!(swift.contains("scrollView.contentSize.width > scrollView.bounds.width + 1"));
        assert!(swift.contains(
            "NexaNativeListRowPrimitive(position: listPosition, showDividers: true, prioritizeSwipeActions: true, reorderIndex: listPosition, reorderEnabled: nexa_moveEnabled, onReorder: { source, destination in"
        ));
        assert!(swift.contains("LongPressGesture(minimumDuration: 0.35)"));
        assert!(swift.contains(
            ".sequenced(before: DragGesture(minimumDistance: 0, coordinateSpace: .global))"
        ));
        assert!(swift.contains("NexaFastListRowFramesKey.self"));
        assert!(swift.contains("proxy.scrollTo(rowCount - 1, anchor: .bottom)"));
        assert!(swift.contains("updateNativeScrollView(scrollView)"));
        assert!(swift.contains("rowFrames.min(by:"));
        assert!(swift.contains("session.update(to: drag.location, perform: onReorder)"));
        assert!(swift.contains("@Published fileprivate(set) var dragOffset: CGFloat = 0"));
        assert!(swift.contains("@ObservedObject private var state: NexaFastListReorderRowState"));
        assert!(swift.contains("rowStates = rowStates.filter"));
        assert!(!swift.contains("@Observable"));
        assert!(swift.contains(".frame(maxWidth: .infinity, alignment: .leading)"));
        assert!(swift.contains(".zIndex(isDragging ? 1 : 0)"));
        assert!(!swift.contains(".onDrag {"));
        assert!(!swift.contains(".onDrop("));
        assert!(!swift.contains("NexaFastListReorderDropDelegate"));
        assert!(!swift.contains("UniformTypeIdentifiers"));
        assert!(swift.contains(
            "NexaNativeListPrimitive(scrollEnabled: true, reorderEnabled: true, rowCount:"
        ));
        assert!(swift.contains("let nexa_from: Int32 = Int32(source)"));
        assert!(!swift.contains("isReorderable"));
        assert!(!swift.contains(".onMove {"));
        assert!(!swift.contains("\\.editMode"));
        // Swipe actions can still require UIKit-backed control styling; the
        // native List assertion above is the reliable renderer check.

        let mut disabled_module = module.clone();
        let Node::FastList { plan } = &mut disabled_module.body[0] else {
            panic!("expected the items-backed FastList plan");
        };
        let ListPlan::Items { common, .. } = plan.as_mut() else {
            panic!("expected the items-backed FastList plan");
        };
        common.scroll_enabled = Some(Expr::Bool(false));
        let disabled_swift = generate(&disabled_module);
        assert!(disabled_swift.contains(
            "NexaNativeListPrimitive(scrollEnabled: false, reorderEnabled: true, rowCount:"
        ));
        assert!(disabled_swift.contains(".scrollDisabled(!scrollEnabled)"));

        let mut referenced_module = module.clone();
        let Node::FastList { plan } = &mut referenced_module.body[0] else {
            panic!("expected the items-backed FastList plan");
        };
        let ListPlan::Items { common, .. } = plan.as_mut() else {
            panic!("expected the items-backed FastList plan");
        };
        common.component_ref = Some(Expr::ComponentRefCreate {
            namespace: "Nexa".to_owned(),
            name: "FastList".to_owned(),
            identity: "test-fast-list-ref".to_owned(),
        });
        let referenced_swift = generate(&referenced_module);
        assert!(
            referenced_swift.contains("componentRef: NexaFastListRef()"),
            "generated list ref argument was missing:\n{referenced_swift}"
        );
        assert!(referenced_swift.contains("position: listPosition"));

        let mut sectioned_module = module;
        let Node::FastList { plan } = &mut sectioned_module.body[0] else {
            panic!("expected the items-backed FastList plan");
        };
        let ListPlan::Items { common, .. } = plan.as_ref() else {
            panic!("expected the items-backed FastList plan");
        };
        let sectioned_common = SectionedListCommon {
            show_dividers: common.show_dividers,
            item_extent: common.item_extent,
            gap: common.gap,
            scroll_enabled: common.scroll_enabled.clone(),
            index: common.index.clone(),
            component_ref: common.component_ref.clone(),
            key: common.key.clone(),
            children: common.children.clone(),
            section_header: None,
            swipe_actions: common.swipe_actions.clone(),
            refresh: common.refresh.clone(),
        };
        **plan = ListPlan::Sections {
            collection: Expr::State(
                "taskSections".to_owned(),
                Type::Array(Box::new(Type::Array(Box::new(Type::String)))),
            ),
            element_type: Type::String,
            section: "section".to_owned(),
            item: "task".to_owned(),
            common: sectioned_common,
        };
        let sectioned_swift = generate(&sectioned_module);
        assert!(sectioned_swift.contains("starts.reserveCapacity(nexa_list_sections_"));
        assert!(sectioned_swift.contains("position: nexa_list_section_starts_"));
        assert!(!sectioned_swift.contains(".prefix(sectionPosition).reduce(0) { $0 + $1.count }"));
    }

    #[test]
    fn navigation_screens_fill_content_and_share_image_namespace() {
        let image = Node::Image {
            source: ImageSource::Asset("hero".to_owned()),
            description: "Hero".to_owned(),
            scale: ImageScale::Fit,
            placeholder: None,
            show_loading_indicator: false,
            max_height: None,
            fill_max_size: false,
            shared_element: Some(Expr::String("hero-image".to_owned())),
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "SharedHero".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: vec![
                Screen {
                    id: ScreenId(0),
                    name: "Home".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Appearance {
                        mode: Expr::String("dark".to_owned()),
                        children: vec![image],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Screen {
                    id: ScreenId(1),
                    name: "Details".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Text {
                        value: Expr::String("Detail screen".to_owned()),
                        style: TextStyle::default(),
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                show_navigation_bar: true,
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let app_body = swift
            .split("struct NexaScreen0: View")
            .next()
            .expect("navigation host is emitted before screen declarations");
        let appearance_position = app_body
            .find("NexaAppearancePrimitive(mode: \"dark\")")
            .expect("root appearance is hoisted to the navigation host");
        let navigation_position = app_body
            .find("NavigationStack(path: __nexaNavigationPath)")
            .expect("navigation stack is generated");
        assert_eq!(
            app_body
                .matches(
                    "NexaNavigationPathHost { __nexaNavigationPath, __nexaRootScreenIdentity in"
                )
                .count(),
            1,
        );
        assert!(!app_body.contains("@State private var __nexaNavigationPath"));
        let mut multiple_stack_module = module.clone();
        multiple_stack_module.body.push(Node::NavigationStack {
            root: ScreenId(1),
            arguments: Vec::new(),
            show_navigation_bar: false,
            title_mode: nexa_ir::NavigationTitleMode::Inline,
            title_alignment: nexa_ir::NavigationTitleAlignment::Automatic,
        });
        let multiple_stack_swift = generate(&multiple_stack_module);
        let multiple_stack_app = multiple_stack_swift
            .split("struct NexaScreen0: View")
            .next()
            .expect("navigation host is emitted before screen declarations");
        assert_eq!(
            multiple_stack_app
                .matches(
                    "NexaNavigationPathHost { __nexaNavigationPath, __nexaRootScreenIdentity in"
                )
                .count(),
            2,
            "each stack occurrence owns an independent path host",
        );
        assert!(appearance_position < navigation_position);
        assert!(swift.contains("static let defaultValue: Binding<[NexaNavigationRoute]>"));
        assert!(swift.contains("@State private var path: [NexaNavigationRoute] = []"));
        assert!(!swift.contains("Binding<NavigationPath>"));
        assert!(swift.contains("struct NexaNavigationBackSwipeModifier: ViewModifier"));
        assert!(swift.contains("struct NexaNavigationBackGestureProbe: UIViewRepresentable"));
        assert!(
            !app_body
                .contains(".modifier(NexaNavigationBackSwipeModifier(path: __nexaNavigationPath))")
        );
        assert!(
            multiple_stack_app
                .contains(".modifier(NexaNavigationBackSwipeModifier(path: __nexaNavigationPath))")
        );
        assert!(swift.contains("navigationController.interactivePopGestureRecognizer"));
        assert!(swift.contains("guard !path.wrappedValue.isEmpty else { return false }"));
        assert!(swift.contains("view.effectiveUserInterfaceLayoutDirection == .rightToLeft"));
        assert!(swift.contains("return isLeadingEdgeSwipe && abs(velocity.x) > abs(velocity.y)"));
        assert!(!swift.contains("DragGesture(minimumDistance: 18"));
        assert!(app_body.contains("NexaNavigationScreenPrimitive(title: \"Details\""));
        assert!(app_body.contains(".ignoresSafeArea(.container, edges: .top)"));
        assert!(app_body.contains(".toolbar(.hidden, for: .navigationBar)"));
        let navigation_stack_environment = app_body
            .find("}.environment(\\.nexaNavigationPath, __nexaNavigationPath)")
            .expect("navigation stack environment is emitted");
        let edge_to_edge_modifier = app_body
            .find(".ignoresSafeArea(.container, edges: .top)")
            .expect("toolbar-free navigation is edge-to-edge");
        assert!(edge_to_edge_modifier > navigation_stack_environment);
        assert!(multiple_stack_app.contains(".ignoresSafeArea(.container, edges: .all)"));
        assert!(swift.contains("@Namespace private var nexaSharedNamespace"));
        assert!(
            swift.contains(".frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)")
        );
        assert!(!swift.contains(".modifier(NexaNavigationChromeModifier("));
        assert!(swift.contains("struct NexaNavigationScreenPrimitive<Content: View>: View"));
        assert!(swift.contains(".toolbar(.visible, for: .navigationBar)"));
        assert!(swift.contains(".navigationTitle(inline && leadingTitle ? \"\" : title)"));
        assert!(swift.contains("NexaNavigationTitleDisplayModeModifier(inline: inline)"));
        assert!(swift.contains("content.toolbarTitleDisplayMode(inline ? .inline : .large)"));
        assert!(swift.contains("content.navigationBarTitleDisplayMode(inline ? .inline : .large)"));
        assert!(!swift.contains(".navigationBarBackButtonHidden(true)"));
        assert!(!swift.contains("NexaNavigationEdgeBackGesture"));
        assert!(swift.contains("ToolbarItem(placement: .topBarLeading)"));
        assert!(!swift.contains("Image(systemName: \"chevron.backward\")"));
        assert!(swift.contains(".accessibilityLabel(title)"));
        assert!(swift.contains(".frame(minWidth: 160, alignment: .leading)"));
        assert!(swift.contains(".offset(x: -13)"));
        assert!(swift.contains("Text(title)"));
        assert!(swift.contains("if inline && leadingTitle"));
        assert!(swift.contains("inline: true"));
        assert!(swift.contains("leadingTitle: true"));
        assert!(!swift.contains("NexaNavigationChromeProbe"));
        assert!(!swift.contains("NexaNavigationChromeState"));
        assert!(!swift.contains("NexaTitleBarModifier"));
        assert!(!swift.contains("NexaTitleBarScrollCoordinator"));
        assert!(!swift.contains("NexaTitleHeaderPrimitive"));
        assert!(swift.contains(".nexaSharedElement(id: \"hero-image\")"));
        assert!(swift.contains(".environment(\\.nexaSharedNamespace, nexaSharedNamespace)"));
        assert!(swift.contains("matchedGeometryEffect(id: id, in: namespace)"));

        let mut large_module = module.clone();
        let Node::NavigationStack { title_mode, .. } = &mut large_module.body[0] else {
            panic!("test module should have a navigation stack");
        };
        *title_mode = nexa_ir::NavigationTitleMode::Large;
        assert!(generate(&large_module).contains("inline: false"));

        let mut centered_module = module.clone();
        let Node::NavigationStack {
            title_alignment, ..
        } = &mut centered_module.body[0]
        else {
            panic!("test module should have a navigation stack");
        };
        *title_alignment = nexa_ir::NavigationTitleAlignment::Center;
        assert!(generate(&centered_module).contains("leadingTitle: false"));

        let mut automatic_without_toolbar = module.clone();
        let Node::NavigationStack {
            title_alignment, ..
        } = &mut automatic_without_toolbar.body[0]
        else {
            panic!("test module should have a navigation stack");
        };
        *title_alignment = nexa_ir::NavigationTitleAlignment::Automatic;
        assert!(generate(&automatic_without_toolbar).contains("leadingTitle: false"));

        automatic_without_toolbar.screens[0]
            .body
            .push(Node::Toolbar {
                placement: nexa_ir::ToolbarPlacement::Trailing,
                children: vec![Node::Text {
                    value: Expr::String("Save".to_owned()),
                    style: TextStyle::default(),
                }],
            });
        let with_root_toolbar = generate(&automatic_without_toolbar);
        let navigation_host = with_root_toolbar
            .split("struct NexaScreen0: View")
            .next()
            .expect("navigation host is emitted before screen declarations");
        assert!(with_root_toolbar.contains("leadingTitle: false"));
        assert!(!navigation_host.contains(".ignoresSafeArea(.container, edges: .top)"));
        assert!(!navigation_host.contains(".toolbar(.hidden, for: .navigationBar)"));
    }

    #[test]
    fn configured_spring_uses_native_response_and_damping_values() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "SpringAnimation".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Layout {
                kind: LayoutKind::Column,
                gap: 0.0,
                style: ViewStyle {
                    animation: Some(AnimationSpec::Spring {
                        response: 0.35,
                        damping: 0.8,
                    }),
                    ..ViewStyle::default()
                },
                children: vec![Node::Text {
                    value: Expr::String("Animated".to_owned()),
                    style: TextStyle::default(),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains(
            ".modifier(NexaAnimateContentSizeModifier(animation: .interpolatingSpring(mass: 1, stiffness: 816.32654, damping: 45.714287, initialVelocity: 0)))"
        ));
        let animated_size_modifier = swift
            .split("struct NexaAnimateContentSizeModifier: ViewModifier {")
            .nth(1)
            .expect("emit the layout animation modifier");
        assert!(animated_size_modifier.contains("transaction.animation = nil"));
        let frame = animated_size_modifier
            .find(".frame(width: contentSize?.width, height: contentSize?.height, alignment: .topLeading)")
            .expect("animate the measured frame");
        assert!(animated_size_modifier.contains("withAnimation(animation)"));
        assert!(animated_size_modifier.contains("if reduceMotion {"));
        assert!(animated_size_modifier.contains("self.contentSize = newSize"));
        assert!(!animated_size_modifier.contains(".animation("));
        let measurement = animated_size_modifier
            .find(".onChange(of: geometry.size, perform: updateContentSize)")
            .expect("geometry changes update the measured content size");
        assert!(
            measurement < frame,
            "the measured size drives the outer frame animation"
        );
        let animated_update = animated_size_modifier
            .split("if reduceMotion {")
            .nth(1)
            .expect("respect Reduce Motion around size updates");
        let animated_branch = animated_update
            .split("else {")
            .nth(1)
            .expect("animate size updates when Reduce Motion is off");
        assert!(
            animated_branch
                .find("withAnimation(animation)")
                .expect("animate the measured size update")
                < animated_branch
                    .find("self.contentSize = newSize")
                    .expect("update the measured size")
        );
        assert!(!animated_size_modifier.contains("PreferenceKey"));
        assert!(animated_size_modifier.contains(".clipped()"));
    }

    #[test]
    fn currency_formatting_emits_only_the_reachable_native_helper() {
        let call = Expr::NativeCall {
            receiver: None,
            namespace: "Number".to_owned(),
            name: "formatCurrency".to_owned(),
            arguments: vec![
                (
                    "amount".to_owned(),
                    Expr::Number {
                        raw: "1234.5".to_owned(),
                        ty: NumericType::Float64,
                    },
                ),
                ("currencyCode".to_owned(), Expr::String("EUR".to_owned())),
            ],
            codecs: Vec::new(),
            return_type: Type::String,
            source_span: None,
            is_async: false,
            is_throwing: false,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "CurrencyFormatting".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "price".to_owned(),
                ty: Type::String,
                initial: call,
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::State("price".to_owned(), Type::String),
                style: TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("nexaFormatCurrency(Double(1234.5), \"EUR\")"));
        assert!(swift.contains("func nexaFormatCurrency"));
        assert!(swift.contains("import Foundation\n"));
    }

    #[test]
    fn conditional_view_transitions_use_native_swiftui_modifiers() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ConditionalTransitions".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![
                Node::If {
                    condition: Expr::State("visible".to_owned(), Type::Bool),
                    then_body: vec![Node::Text {
                        value: Expr::String("shown".to_owned()),
                        style: TextStyle::default(),
                    }],
                    else_body: None,
                    transition: Some(ViewTransition::Fade),
                    transition_animation: Some(nexa_ir::AnimationSpec::Tween {
                        duration: 0.24,
                        delay: 0.04,
                        easing: nexa_ir::AnimationEasing::EaseOut,
                    }),
                },
                Node::When {
                    value: Expr::State("visible".to_owned(), Type::Bool),
                    cases: vec![WhenCase {
                        value: Expr::Bool(true),
                        body: vec![Node::Text {
                            value: Expr::String("yes".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    else_body: Some(vec![Node::Text {
                        value: Expr::String("no".to_owned()),
                        style: TextStyle::default(),
                    }]),
                    transition: Some(ViewTransition::SlideFromBottom),
                    transition_animation: Some(nexa_ir::AnimationSpec::Spring {
                        response: 0.35,
                        damping: 0.8,
                    }),
                },
                Node::If {
                    condition: Expr::State("visible".to_owned(), Type::Bool),
                    then_body: vec![Node::Text {
                        value: Expr::String("no default fade".to_owned()),
                        style: TextStyle::default(),
                    }],
                    else_body: None,
                    transition: None,
                    transition_animation: None,
                },
                Node::When {
                    value: Expr::State("visible".to_owned(), Type::Bool),
                    cases: vec![
                        WhenCase {
                            value: Expr::Bool(true),
                            body: vec![Node::Text {
                                value: Expr::String("exhaustive true".to_owned()),
                                style: TextStyle::default(),
                            }],
                        },
                        WhenCase {
                            value: Expr::Bool(false),
                            body: vec![Node::Text {
                                value: Expr::String("exhaustive false".to_owned()),
                                style: TextStyle::default(),
                            }],
                        },
                    ],
                    else_body: None,
                    transition: None,
                    transition_animation: None,
                },
            ],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("Group {\n"));
        assert!(swift.contains("NexaAnimatedWhen(value: nexa_visible"));
        assert!(swift.contains("transition: .opacity"));
        assert!(swift.contains("transition: .move(edge: .bottom)"));
        assert!(swift.contains("animation: "));
        assert_eq!(swift.matches(".transition(.identity)").count(), 2);
        let false_case = swift
            .rfind("case false:")
            .expect("the exhaustive boolean when emits its false case");
        let exhaustive_start = swift[..false_case]
            .rfind("switch ")
            .expect("the exhaustive boolean when is emitted as a Swift switch");
        let exhaustive_switch = braced_block(&swift, exhaustive_start);
        assert!(!exhaustive_switch.contains("default:"));
    }

    #[test]
    fn double_tap_pressable_emits_exclusive_double_and_single_tap_gestures() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "DoubleTapApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: true,
                children: vec![Node::Text {
                    value: Expr::String("Tap twice".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: vec![Action::Assign {
                    name: "taps".to_owned(),
                    value: Expr::Number {
                        raw: "2".to_owned(),
                        ty: NumericType::Int32,
                    },
                }],
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("TapGesture(count: 2)"));
        assert!(swift.contains(
            ".frame(maxWidth: .infinity, maxHeight: .infinity).contentShape(Rectangle())"
        ));
        assert!(swift.contains(".exclusively(before:"));
        assert!(swift.contains("TapGesture(count: 1)"));
        assert!(swift.contains("nexa_taps = 2"));
        assert!(!swift.contains("onLongPressGesture"));
        assert!(!swift.contains("NexaDragGestureView"));

        let mut long_press_module = module;
        let Node::Pressable {
            actions,
            double_tap_actions,
            long_press_actions,
            ..
        } = &mut long_press_module.body[0]
        else {
            panic!("expected Pressable");
        };
        *actions = vec![Action::Assign {
            name: "taps".to_owned(),
            value: Expr::Number {
                raw: "1".to_owned(),
                ty: NumericType::Int32,
            },
        }];
        double_tap_actions.clear();
        *long_press_actions = vec![Action::Assign {
            name: "longPresses".to_owned(),
            value: Expr::Number {
                raw: "1".to_owned(),
                ty: NumericType::Int32,
            },
        }];

        let swift = generate(&long_press_module);

        assert!(swift.contains("Button(action: {}) {"));
        assert!(swift.contains("LongPressGesture(minimumDuration:"));
        assert!(swift.contains(".exclusively(before:"));
        assert!(swift.contains("TapGesture(count: 1)"));
        assert!(swift.contains("nexa_longPresses = 1"));
        assert!(swift.contains("nexa_taps = 1"));
        assert!(!swift.contains(".onLongPressGesture("));
        assert!(
            swift.find("LongPressGesture(").unwrap() < swift.find("TapGesture(count: 1)").unwrap()
        );

        let mut combined_module = long_press_module.clone();
        let Node::Pressable {
            double_tap_actions, ..
        } = &mut combined_module.body[0]
        else {
            panic!("expected Pressable");
        };
        *double_tap_actions = vec![Action::Assign {
            name: "doubleTaps".to_owned(),
            value: Expr::Number {
                raw: "2".to_owned(),
                ty: NumericType::Int32,
            },
        }];

        let swift = generate(&combined_module);
        let long_press_index = swift.find("LongPressGesture(").unwrap();
        let double_tap_index = swift.find("TapGesture(count: 2)").unwrap();
        let single_tap_index = swift.find("TapGesture(count: 1)").unwrap();
        assert!(long_press_index < double_tap_index && double_tap_index < single_tap_index);
        assert!(swift.contains("nexa_doubleTaps = 2"));
    }

    #[test]
    fn drag_pressable_emits_native_gesture_and_typed_callback() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "DragApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "distance".to_owned(),
                ty: Type::Numeric(NumericType::Float64),
                initial: Expr::Number {
                    raw: "0.0".to_owned(),
                    ty: NumericType::Float64,
                },
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Drag me".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: vec![
                    "translationX".to_owned(),
                    "translationY".to_owned(),
                    "velocityX".to_owned(),
                    "velocityY".to_owned(),
                ],
                drag_actions: vec![Action::Assign {
                    name: "distance".to_owned(),
                    value: Expr::State("velocityX".to_owned(), Type::Numeric(NumericType::Float64)),
                }],
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("private struct NexaDragGestureView<Content: View>"));
        assert!(swift.contains("content.highPriorityGesture(dragGesture)"));
        assert!(swift.contains(
            "onDrag: { nexa_translationX, nexa_translationY, nexa_velocityX, nexa_velocityY in"
        ));
        assert!(swift.contains("trackVelocity: true"));
        assert!(swift.contains("nexa_distance = nexa_velocityX"));

        let mut translation_only_module = module;
        let Node::Pressable { drag_actions, .. } = &mut translation_only_module.body[0] else {
            panic!("expected Pressable");
        };
        drag_actions[0] = Action::Assign {
            name: "distance".to_owned(),
            value: Expr::State(
                "translationX".to_owned(),
                Type::Numeric(NumericType::Float64),
            ),
        };
        let swift = generate(&translation_only_module);
        assert!(swift.contains("trackVelocity: false"));
    }

    #[test]
    fn pinch_pressable_emits_native_scale_delta_callback() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PinchApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "zoom".to_owned(),
                ty: Type::Numeric(NumericType::Float64),
                initial: Expr::Number {
                    raw: "1.0".to_owned(),
                    ty: NumericType::Float64,
                },
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Pinch to zoom".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: Some("scaleFactor".to_owned()),
                pinch_actions: vec![Action::Assign {
                    name: "zoom".to_owned(),
                    value: Expr::State(
                        "scaleFactor".to_owned(),
                        Type::Numeric(NumericType::Float64),
                    ),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("MagnificationGesture()"));
        assert!(swift.contains("NexaMagnificationTracker"));
        assert!(swift.contains("onPinch: { nexa_scaleFactor in"));
        assert!(swift.contains("nexa_zoom = nexa_scaleFactor"));
        assert!(!swift.contains("onDrag: {"));
    }

    fn native_instance_state(name: &str) -> State {
        let ty = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        State {
            name: name.to_owned(),
            ty: ty.clone(),
            initial: Expr::Call {
                name: "Video.VideoPlayer".to_owned(),
                arguments: Vec::new(),
                return_type: ty,
                is_async: false,
                is_throwing: false,
                is_constructor: true,
            },
            mutable: false,
        }
    }

    #[test]
    fn app_native_class_instances_are_not_recreated_in_the_body() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![native_instance_state("player")],
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "@StateObject private var __nexaNativeObjectStorage_{state_name} = NexaNativeObjectStorage {{ VideoPlayer() }}"
        )));
        assert!(swift.contains(&format!(
            "private var {state_name}: VideoPlayer {{\n        get {{ __nexaNativeObjectStorage_{state_name}.value }}"
        )));
        assert_eq!(
            swift.matches("VideoPlayer()").count(),
            1,
            "the constructor belongs only in the persistent @State initializer"
        );
    }

    #[test]
    fn immutable_app_values_are_view_properties_for_generated_helpers() {
        let mut module = regex_module(false);
        module.states.push(State {
            name: "screenTitle".to_owned(),
            ty: Type::String,
            initial: Expr::String("Reading list".to_owned()),
            mutable: false,
        });
        module.body = vec![Node::Text {
            value: Expr::State("screenTitle".to_owned(), Type::String),
            style: TextStyle::default(),
        }];

        let swift = generate(&module);
        let property = "private var nexa_screenTitle: String { \"Reading list\" }";
        let body = swift.find("public var body: some View").unwrap();
        let property = swift.find(property).unwrap();

        assert!(
            property < body,
            "immutable app values must be view properties"
        );
        assert!(swift.contains("Text(nexa_screenTitle)"));
    }

    #[test]
    fn mutable_native_class_bindings_write_through_identity_storage() {
        let mut player = native_instance_state("player");
        player.mutable = true;
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![player],
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "nonmutating set {{ __nexaNativeObjectStorage_{state_name}.value = newValue }}"
        )));
    }

    #[test]
    fn screen_native_class_instances_use_swiftui_identity_storage() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![
                native_instance_state("appPlayer"),
                State {
                    name: "sharedCount".to_owned(),
                    ty: Type::Numeric(NumericType::Int32),
                    initial: Expr::Number {
                        raw: "7".to_owned(),
                        ty: NumericType::Int32,
                    },
                    mutable: true,
                },
            ],
            screens: vec![
                Screen {
                    id: ScreenId(0),
                    name: "PlayerScreen".to_owned(),
                    parameters: Vec::new(),
                    states: vec![
                        State {
                            name: "player".to_owned(),
                            ty: player_type.clone(),
                            initial: Expr::Call {
                                name: "Video.VideoPlayer".to_owned(),
                                arguments: Vec::new(),
                                return_type: player_type.clone(),
                                is_async: false,
                                is_throwing: false,
                                is_constructor: true,
                            },
                            mutable: false,
                        },
                        State {
                            name: "screenCount".to_owned(),
                            ty: Type::Numeric(NumericType::Int32),
                            initial: Expr::Number {
                                raw: "1".to_owned(),
                                ty: NumericType::Int32,
                            },
                            mutable: true,
                        },
                    ],
                    body: vec![Node::NavigationLink {
                        destination: ScreenId(1),
                        arguments: Vec::new(),
                        guard: None,
                        children: vec![Node::Text {
                            value: Expr::String("Push another instance".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Screen {
                    id: ScreenId(1),
                    name: "Details".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Button {
                        label: Expr::String("Play shared player".to_owned()),
                        icon: None,
                        loading: None,
                        disabled: None,
                        style: None,
                        size: None,
                        shape: None,
                        tint: None,
                        glass: false,
                        accessibility_label: None,
                        actions: vec![Action::Expression(Expr::NativeCall {
                            receiver: Some(Box::new(Expr::State(
                                "appPlayer".to_owned(),
                                player_type.clone(),
                            ))),
                            namespace: "Video".to_owned(),
                            name: "play".to_owned(),
                            arguments: Vec::new(),
                            codecs: Vec::new(),
                            return_type: Type::Void,
                            source_span: None,
                            is_async: false,
                            is_throwing: false,
                        })],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                show_navigation_bar: true,
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        // Top-level `private` is lowered because each unit compiles as its own
        // file, so the screen declaration is internal here.
        let screen_declaration = swift
            .find("struct NexaScreen0: View")
            .expect("screen state must be owned by a destination view");
        let app_source = &swift[..screen_declaration];
        let screen_source = &swift[screen_declaration..];
        assert!(
            screen_source.contains(&format!(
                "@StateObject private var __nexaNativeObjectStorage_{state_name}"
            )),
            "screen-scoped native objects must be initialized once in route-owned SwiftUI storage"
        );
        assert!(!app_source.contains("__nexaNativeObjectStorage_nexa_player"));
        assert!(
            app_source
                .contains("@StateObject private var __nexaNativeObjectStorage_nexa_appPlayer")
        );
        assert!(screen_source.contains("private let nexa_appPlayer: VideoPlayer"));
        assert!(!screen_source.contains("@Binding private var nexa_appPlayer"));
        assert_eq!(screen_source.matches("VideoPlayer()").count(), 1);
        assert!(app_source.contains("@State private var nexa_sharedCount: Int32 = 7"));
        assert!(screen_source.contains("@Binding private var nexa_sharedCount: Int32"));
        assert!(screen_source.contains("@State private var nexa_screenCount: Int32 = 1"));
        assert!(swift.contains("case screen0(UUID)"));
        assert!(swift.contains("case screen1(UUID)"));
        assert!(swift.contains("NexaNavigationRoute.screen1(UUID())"));
        assert!(swift.contains("case let .screen0(routeIdentity):"));
        assert!(swift.contains("case let .screen1(routeIdentity):"));
        assert!(swift.contains("nexa_appPlayer.play()"));
        assert!(swift.contains(".id(routeIdentity)"));
    }

    #[test]
    fn component_native_class_instances_use_swiftui_identity_storage() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: vec![Component {
                name: "PlayerPanel".to_owned(),
                source_file: None,
                parameters: Vec::new(),
                states: vec![native_instance_state("player")],
                body: Vec::new(),
                ref_methods: Vec::new(),
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            body: vec![Node::ComponentCall {
                name: "PlayerPanel".to_owned(),
                arguments: Vec::new(),
                component_ref: None,
                children: None,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "@StateObject private var __nexaNativeObjectStorage_{state_name} = NexaNativeObjectStorage {{ VideoPlayer() }}"
        )));
    }

    #[test]
    fn handleless_tasks_are_tracked_and_cancelled_with_their_app_view() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ScopedTaskApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Button {
                label: Expr::String("Start".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                style: None,
                size: None,
                shape: None,
                tint: None,
                glass: false,
                accessibility_label: None,
                actions: vec![Action::TaskLaunch {
                    handle: None,
                    executor: nexa_ir::TaskExecutor::Main,
                    actions: vec![Action::Expression(Expr::Bool(true))],
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("final class NexaTaskScope"));
        assert!(swift.contains("@State private var nexaTaskScope = NexaTaskScope()"));
        assert!(swift.contains("nexaTaskScope.track(Task { @MainActor in"));
        assert!(swift.contains("nexaTaskScope.cancelAll()"));
    }

    #[test]
    fn handleless_component_tasks_are_cancelled_with_the_component_instance() {
        let task_button = Node::Button {
            label: Expr::String("Refresh panel".to_owned()),
            icon: None,
            loading: None,
            disabled: None,
            style: None,
            size: None,
            shape: None,
            tint: None,
            glass: false,
            accessibility_label: None,
            actions: vec![Action::TaskLaunch {
                handle: None,
                executor: nexa_ir::TaskExecutor::Main,
                actions: vec![Action::Expression(Expr::Bool(true))],
            }],
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "ComponentTaskApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: vec![Component {
                name: "TaskPanel".to_owned(),
                source_file: None,
                parameters: Vec::new(),
                states: Vec::new(),
                body: vec![task_button],
                ref_methods: Vec::new(),
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            body: vec![Node::ComponentCall {
                name: "TaskPanel".to_owned(),
                arguments: Vec::new(),
                component_ref: None,
                children: None,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("struct NexaTaskPanelComponent: View {"));
        assert!(swift.contains("@State private var nexaTaskScope = NexaTaskScope()"));
        assert!(swift.contains("nexaTaskScope.track(Task { @MainActor in"));
        assert!(swift.contains("nexaTaskScope.cancelAll()"));
        assert_eq!(swift.matches("@State private var nexaTaskScope").count(), 1);
    }

    #[test]
    fn generates_result_and_try_in_swift() {
        let err_type = Type::Enum("AppError".to_owned());
        let module = Module {
            widgets: Vec::new(),
            app_name: "ResultApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            background_tasks: Vec::new(),
            enums: vec![nexa_ir::EnumDecl {
                name: "AppError".to_owned(),
                cases: vec!["NotFound".to_owned(), "Unauthorized".to_owned()],
            }],
            structs: Vec::new(),
            functions: vec![
                Function {
                    name: "fetchCode".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    failure_type: None,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    body_actions: None,
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::ResultOk {
                        value: Box::new(Expr::Number {
                            raw: "42".to_owned(),
                            ty: NumericType::Int32,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
                Function {
                    name: "compute".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    failure_type: None,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    body_actions: None,
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::Try {
                        expr: Box::new(Expr::Call {
                            name: "fetchCode".to_owned(),
                            arguments: Vec::new(),
                            return_type: Type::Result(
                                Box::new(Type::Numeric(NumericType::Int32)),
                                Box::new(err_type.clone()),
                            ),
                            is_async: false,
                            is_throwing: false,
                            is_constructor: false,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
            ],
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("enum NexaAppError: Int, Error {"));
        assert!(swift.contains("Result<Int32, NexaAppError>"));
        assert!(swift.contains(".success(42)"));
        assert!(swift.contains("try nexa_fn_fetchCode().get()"));
    }

    #[test]
    fn app_user_class_instances_use_native_object_storage() {
        let class_type = Type::Class {
            name: "NoteStore".to_owned(),
            fields: vec![(
                "notes".to_owned(),
                Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
            )],
            constructor_parameter_count: 0,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "Demo".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![
                State {
                    name: "store".to_owned(),
                    ty: class_type.clone(),
                    initial: Expr::Call {
                        name: "NoteStore".to_owned(),
                        arguments: Vec::new(),
                        return_type: class_type.clone(),
                        is_async: false,
                        is_throwing: false,
                        is_constructor: true,
                    },
                    mutable: false,
                },
                State {
                    name: "notes".to_owned(),
                    ty: Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
                    initial: Expr::Member {
                        base: Box::new(Expr::State("store".to_owned(), class_type.clone())),
                        name: "notes".to_owned(),
                        optional: false,
                        base_type: Type::Class {
                            name: "NoteStore".to_owned(),
                            fields: Vec::new(),
                            constructor_parameter_count: 0,
                        },
                        field_type: Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
                        kind: nexa_ir::MemberKind::ClassField("notes".to_owned()),
                    },
                    mutable: false,
                },
            ],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Hello".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("__nexaNativeObjectStorage_nexa_store"));
        assert!(swift.contains("let nexa_store = __storage_nexa_store.value"));
        assert!(swift.contains("_nexa_notes = StateObject(wrappedValue: nexa_store.nexa_notes)"));
    }
}
