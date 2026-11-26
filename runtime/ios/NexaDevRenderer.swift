import Foundation
import SwiftUI
import UIKit

private func nexaDevNumberFlowAnimation(_ raw: Any?) -> Animation {
    guard let animation = raw as? [String: Any], let (kind, payload) = animation.first else {
        return .spring(response: 0.35, dampingFraction: 0.82)
    }
    switch kind {
    case "Spring":
        let values = payload as? [String: Any] ?? [:]
        return .spring(
            response: values["response"] as? Double ?? 0.35,
            dampingFraction: values["damping"] as? Double ?? 0.82
        )
    case "EaseIn": return .easeIn(duration: 0.3)
    case "EaseOut": return .easeOut(duration: 0.3)
    case "EaseInOut": return .easeInOut(duration: 0.3)
    case "Linear": return .linear(duration: 0.3)
    case "Tween":
        let values = payload as? [String: Any] ?? [:]
        let duration = values["duration"] as? Double ?? 0.3
        let delay = values["delay"] as? Double ?? 0
        let easingValue = values["easing"]
        let easing = easingValue as? [String: Any] ?? [:]
        let easingName = easingValue as? String ?? easing.keys.first
        let curve: Animation
        if let cubic = easing["CubicBezier"] as? [String: Any] {
            curve = .timingCurve(
                cubic["x1"] as? Double ?? 0.4,
                cubic["y1"] as? Double ?? 0,
                cubic["x2"] as? Double ?? 0.2,
                cubic["y2"] as? Double ?? 1,
                duration: duration
            )
        } else if easingName == "EaseIn" {
            curve = .easeIn(duration: duration)
        } else if easingName == "EaseOut" {
            curve = .easeOut(duration: duration)
        } else if easingName == "Linear" {
            curve = .linear(duration: duration)
        } else {
            curve = .easeInOut(duration: duration)
        }
        return delay > 0 ? curve.delay(delay) : curve
    default: return .spring(response: 0.35, dampingFraction: 0.82)
    }
}

private enum NexaDevDimension: Equatable {
    case fixed(CGFloat)
    case parentFraction(CGFloat)
    case fill
    case screenWidthFraction(CGFloat)
    case screenHeightFraction(CGFloat)
}

private func nexaDevDimension(_ raw: Any?) -> NexaDevDimension? {
    if let value = raw as? NSNumber { return .fixed(CGFloat(value.doubleValue)) }
    if let value = raw as? String, value == "Fill" { return .fill }
    guard let tagged = raw as? [String: Any], let (kind, payload) = tagged.first else { return nil }
    let value = (payload as? NSNumber).map { CGFloat($0.doubleValue) } ?? 0
    switch kind {
    case "Fixed": return .fixed(value)
    case "ParentFraction": return .parentFraction(value)
    case "ScreenWidthFraction": return .screenWidthFraction(value)
    case "ScreenHeightFraction": return .screenHeightFraction(value)
    default: return nil
    }
}

private func nexaDevContainsNativeScrollOwner(_ nodes: [Any]) -> Bool {
    for value in nodes {
        guard let node = value as? [String: Any],
              let kind = node.keys.first,
              let fields = node[kind] as? [String: Any] else {
            continue
        }
        if kind == "ScrollView" || kind == "FastList" || kind == "Form" || kind == "KeyboardAware" || kind == "PagePager" {
            return true
        }
        for key in ["children", "content", "then_body", "else_body", "sidebar", "detail"] {
            if let nested = fields[key] as? [Any], nexaDevContainsNativeScrollOwner(nested) {
                return true
            }
        }
        if let pages = fields["pages"] as? [Any] {
            for page in pages {
                if let nested = page as? [Any], nexaDevContainsNativeScrollOwner(nested) {
                    return true
                }
            }
        }
        if let cases = fields["cases"] as? [[String: Any]],
           cases.contains(where: { nexaDevContainsNativeScrollOwner($0["body"] as? [Any] ?? []) }) {
            return true
        }
    }
    return false
}

private let nexaDevFormRowMinHeight: CGFloat = __NEXA_FORM_ROW_MIN_HEIGHT__
private let nexaDevFormRowHorizontalInset: CGFloat = __NEXA_FORM_ROW_HORIZONTAL_INSET__

private func nexaDevTextBaseFontSize(_ style: [String: Any]) -> CGFloat {
    if let fontSize = style["font_size"] as? Double {
        return CGFloat(fontSize)
    }
    switch style["font_style"] as? String {
__NEXA_SWIFT_TEXT_FONT_STYLE_SIZE_CASES__
    default:
        return CGFloat(__NEXA_DEFAULT_BODY_FONT_SIZE__)
    }
}

private func nexaDevFontFaceName(_ familyName: String, weight: String) -> String {
    switch (familyName.lowercased(), weight) {
__NEXA_FONT_FACE_CASES__
    default: return familyName
    }
}

private struct NexaDevContentSizePreferenceKey: PreferenceKey {
    static var defaultValue: [UUID: CGSize] { [:] }

    static func reduce(value: inout [UUID: CGSize], nextValue: () -> [UUID: CGSize]) {
        value.merge(nextValue()) { _, latest in latest }
    }
}

private struct NexaDevAnimateContentSizeLayout: Layout {
    var animatedSize: CGSize?

    struct Cache {}

    func makeCache(subviews: Subviews) -> Cache { Cache() }

    var animatableData: AnimatablePair<CGFloat, CGFloat> {
        get {
            guard let animatedSize else { return AnimatablePair(-1, -1) }
            return AnimatablePair(animatedSize.width, animatedSize.height)
        }
        set {
            guard newValue.first >= 0, newValue.second >= 0 else {
                animatedSize = nil
                return
            }
            animatedSize = CGSize(width: newValue.first, height: newValue.second)
        }
    }

    func sizeThatFits(
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout Cache
    ) -> CGSize {
        guard let subview = subviews.first else { return .zero }
        guard let animatedSize else { return subview.sizeThatFits(proposal) }
        return animatedSize
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache: inout Cache
    ) {
        guard let subview = subviews.first else { return }
        // Keep measuring against the parent's proposal while the visible
        // wrapper interpolates, so newly inserted text can widen the layout.
        subview.place(at: bounds.origin, anchor: .topLeading, proposal: proposal)
    }
}

@MainActor
private struct NexaDevAnimateContentSizeModifier: ViewModifier {
    let animation: Animation

    @State private var contentSize: CGSize?
    @State private var preferenceID = UUID()
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        NexaDevAnimateContentSizeLayout(animatedSize: contentSize) {
            content
                .transaction { transaction in
                    transaction.animation = nil
                }
                .fixedSize(horizontal: false, vertical: true)
                .background {
                    GeometryReader { geometry in
                        Color.clear.preference(
                            key: NexaDevContentSizePreferenceKey.self,
                            value: [preferenceID: geometry.size]
                        )
                    }
                }
                .onPreferenceChange(NexaDevContentSizePreferenceKey.self) { measurements in
                    guard let newSize = measurements[preferenceID],
                          newSize.width.isFinite,
                          newSize.height.isFinite else {
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
        .clipped()
    }
}

@MainActor
private final class NexaDevDragVelocityTracker {
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
private final class NexaDevMagnificationTracker {
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
private struct NexaDevDragGestureView<Content: View>: View {
    let enabled: Bool
    let content: Content
    let onDrag: ((Double, Double, Double, Double) -> Void)?
    let onPinch: ((Double) -> Void)?
    @State private var velocityTracker = NexaDevDragVelocityTracker()
    @State private var magnificationTracker = NexaDevMagnificationTracker()

    private var dragGesture: some Gesture {
        DragGesture()
            .onChanged { value in
                guard enabled else { return }
                let velocity = velocityTracker.update(value)
                onDrag?(
                    Double(value.translation.width),
                    Double(value.translation.height),
                    Double(velocity.width),
                    Double(velocity.height)
                )
            }
            .onEnded { _ in velocityTracker.reset() }
    }

    private var pinchGesture: some Gesture {
        MagnificationGesture()
            .onChanged { magnification in
                guard enabled,
                    let scaleFactor = magnificationTracker.consume(Double(magnification))
                else { return }
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

private func nexaDevTransition(_ raw: Any?) -> AnyTransition? {
    switch raw as? String {
    case "Fade": .opacity
    case "FadeSlideFromTop": .asymmetric(
        insertion: .opacity.combined(with: .move(edge: .top)),
        removal: .opacity.combined(with: .move(edge: .top))
    )
    case "SlideFromBottom": .move(edge: .bottom)
    case "SlideFromLeft": .move(edge: .leading)
    case "SlideFromRight": .move(edge: .trailing)
    case "Scale": .scale(scale: 0.0, anchor: .center)
    default: nil
    }
}

private func nexaDevUsesIconOnlyButtonLabel(_ rawLabel: Any?, comment: Any?) -> Bool {
    guard comment == nil || comment is NSNull,
          let expression = rawLabel as? [String: Any]
    else { return false }
    if let text = expression["String"] as? String {
        return text.isEmpty
    }
    guard let localized = expression["LocalizedText"] as? [String: Any],
          localized["comment"] == nil || localized["comment"] is NSNull,
          let value = localized["value"] as? [String: Any],
          let text = value["String"] as? String
    else { return false }
    return text.isEmpty
}

private func nexaDevSharedIconSymbol(_ name: String) -> String? {
    switch name {
__NEXA_SHARED_ICON_SF_CASES__
    default: return nil
    }
}

@MainActor
private func nexaDevIconSpec(
    _ rawIcon: Any?,
    store: NexaDevStateStore,
    locals: [String: Any],
    scope: String,
    isDark: Bool
) -> NexaIconSpec? {
    guard let raw = rawIcon, !(raw is NSNull) else { return nil }
    var iconDict = raw as? [String: Any]
    if let dict = iconDict,
       dict["source"] == nil,
       dict["shared"] == nil,
       dict["FontGlyph"] == nil,
       dict["font_glyph"] == nil,
       dict["System"] == nil,
       dict["system"] == nil,
       dict["Icon"] == nil,
       dict["IconWith"] == nil {
        iconDict = store.evaluate(raw, locals: locals, scope: scope) as? [String: Any]
    }
    guard let unwrapped = iconDict else { return nil }
    if let withSpec = unwrapped["IconWith"] as? [String: Any] {
        guard let base = nexaDevIconSpec(withSpec["base"], store: store, locals: locals, scope: scope, isDark: isDark) else {
            return nil
        }
        let description = withSpec["description"] as? String
        let size = (withSpec["size"] as? NSNumber).map { CGFloat($0.doubleValue) }
        let tint: Color? = {
            guard let value = withSpec["color"] as? [String: Any] else { return nil }
            if let dynamic = value["Dynamic"] {
                return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
            }
            return devColor(value, isDark: isDark)
        }()
        let layerColors = nexaDevIconLayerColors(withSpec["layer_colors"], store: store, locals: locals, scope: scope, isDark: isDark)
        return base.withOverrides(description: description, size: size, tint: tint, layerColors: layerColors)
    }
    let icon = (unwrapped["Icon"] as? [String: Any]) ?? unwrapped
    let source = icon["source"] as? [String: Any] ?? icon
    let description = icon["description"] as? String
    let size = (icon["size"] as? NSNumber).map { CGFloat($0.doubleValue) }
    let tint: Color? = {
        guard let value = icon["color"] as? [String: Any] else { return nil }
        if let dynamic = value["Dynamic"] {
            return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
        }
        return devColor(value, isDark: isDark)
    }()
    let layerColors = nexaDevIconLayerColors(icon["layer_colors"], store: store, locals: locals, scope: scope, isDark: isDark)
    if let glyph = (source["FontGlyph"] as? [String: Any] ?? source["font_glyph"] as? [String: Any]),
       let family = glyph["font_family"] as? String,
       let codepoint = glyph["glyph"] as? NSNumber,
       codepoint.int32Value >= 0 {
        return NexaIconSpec(
            glyph: codepoint.int32Value,
            fontFamily: family,
            description: description,
            size: size,
            tint: tint,
            layers: nexaDevIconLayers(glyph["layers"]),
            layerColors: layerColors,
            preservesColor: true
        )
    }
    let system = source["System"] as? [String: Any]
        ?? source["system"] as? [String: Any]
        ?? source
    if let shared = system["shared"] as? String,
       let symbol = nexaDevSharedIconSymbol(shared) {
        return NexaIconSpec(symbol: symbol, description: description, size: size, tint: tint, layerColors: layerColors)
    }
    return nil
}

@MainActor
private func nexaDevIconLayerColors(
    _ raw: Any?,
    store: NexaDevStateStore,
    locals: [String: Any],
    scope: String,
    isDark: Bool
) -> [Color] {
    guard let list = raw as? [[String: Any]] else { return [] }
    return list.compactMap { value in
        if let dynamic = value["Dynamic"] {
            return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
        }
        return devColor(value, isDark: isDark)
    }
}

private func nexaDevIconLayers(_ raw: Any?) -> [NexaIconLayer] {
    guard let layers = raw as? [[String: Any]] else { return [] }
    return layers.compactMap { layer in
        guard let glyph = layer["glyph"] as? NSNumber,
              let rgba = layer["rgba"] as? [NSNumber],
              rgba.count == 4 else { return nil }
        return NexaIconLayer(
            glyph: glyph.int32Value,
            color: Color(
                .sRGB,
                red: Double(rgba[0].intValue) / 255,
                green: Double(rgba[1].intValue) / 255,
                blue: Double(rgba[2].intValue) / 255,
                opacity: Double(rgba[3].intValue) / 255
            ),
            tintable: layer["tintable"] as? Bool ?? false
        )
    }
}

private enum NexaDevGlassShape {
    case circle
    case capsule
    case rounded(Double)
}

private func nexaDevTabContentWithoutToolbars(_ nodes: [Any]) -> [Any] {
    nodes.compactMap { rawNode in
        guard let tagged = rawNode as? [String: Any], let (tag, payload) = tagged.first else {
            return rawNode
        }
        guard tag != "Toolbar" else { return nil }
        guard tag == "Layout", var fields = payload as? [String: Any] else { return rawNode }
        fields["children"] = nexaDevTabContentWithoutToolbars(fields["children"] as? [Any] ?? [])
        return [tag: fields]
    }
}

private func nexaDevTabToolbars(_ nodes: [Any]) -> [[String: Any]] {
    var toolbars: [[String: Any]] = []
    for rawNode in nodes {
        guard let tagged = rawNode as? [String: Any], let (tag, payload) = tagged.first,
              let fields = payload as? [String: Any]
        else { continue }
        if tag == "Toolbar" {
            toolbars.append(fields)
        } else if tag == "Layout" {
            toolbars.append(contentsOf: nexaDevTabToolbars(fields["children"] as? [Any] ?? []))
        }
    }
    return toolbars
}

private extension View {
    @ViewBuilder
    func nexaDevGlass(tint: Color?, shape: NexaDevGlassShape) -> some View {
        if #available(iOS 26.0, *) {
            let effect = tint.map { Glass.clear.tint($0).interactive() } ?? Glass.clear.interactive()
            switch shape {
            case .circle: self.glassEffect(effect, in: Circle())
            case .capsule: self.glassEffect(effect, in: Capsule())
            case .rounded(let radius): self.glassEffect(effect, in: RoundedRectangle(cornerRadius: radius))
            }
        } else {
            switch shape {
            case .circle: self.background(.ultraThinMaterial, in: Circle())
            case .capsule: self.background(.ultraThinMaterial, in: Capsule())
            case .rounded(let radius): self.background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: radius))
            }
        }
    }


}

struct NexaDevContentSlot: @unchecked Sendable {
    let nodes: [Any]
    let scope: String
    let parameters: [String: Any]
}

struct NexaDevContentSlotKey: EnvironmentKey {
    static let defaultValue: NexaDevContentSlot? = nil
}

struct NexaDevPageRestoreRevisionKey: EnvironmentKey {
    static let defaultValue = 0
}

struct NexaDevNodeIdentityKey: EnvironmentKey {
    static let defaultValue = "root"
}

extension EnvironmentValues {
    var nexaDevContentSlot: NexaDevContentSlot? {
        get { self[NexaDevContentSlotKey.self] }
        set { self[NexaDevContentSlotKey.self] = newValue }
    }

    var nexaDevPageRestoreRevision: Int {
        get { self[NexaDevPageRestoreRevisionKey.self] }
        set { self[NexaDevPageRestoreRevisionKey.self] = newValue }
    }

    var nexaDevNodeIdentity: String {
        get { self[NexaDevNodeIdentityKey.self] }
        set { self[NexaDevNodeIdentityKey.self] = newValue }
    }

}

@MainActor
enum NexaDevFastListAxis {
    case vertical
    case horizontal
    case grid(Int)
    case adaptiveGrid(Int)
    case masonry(Int)
}

private struct NexaDevFastListRow: Identifiable {
    let id: AnyHashable
    let index: Int
    let sectionIndex: Int
    let itemIndex: Int
}

@MainActor
private struct NexaDevFastListSwipeActionContent: View {
    let nodes: [Any]
    let store: NexaDevStateStore
    let locals: [String: Any]
    let scope: String
    let isDark: Bool

    @ViewBuilder
    var body: some View {
        ForEach(nodes.indices, id: \.self) { index in
            actionButton(nodes[index])
        }
    }

    @ViewBuilder
    private func actionButton(_ rawNode: Any) -> some View {
        if let variant = rawNode as? [String: Any],
           let fields = variant["Button"] as? [String: Any] {
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let actions = fields["actions"] as? [Any] ?? []
            let loading = fields["loading"].map {
                store.truthy(store.evaluate($0, locals: locals, scope: scope))
            } ?? false
            let disabled = fields["disabled"].map {
                store.truthy(store.evaluate($0, locals: locals, scope: scope))
            } ?? false
            let icon = nexaDevIconSpec(fields["icon"], store: store, locals: locals, scope: scope, isDark: isDark)
            let tint: Color? = {
                guard let tintValue = fields["tint"] as? [String: Any] else { return nil }
                if let dynamic = tintValue["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(tintValue, isDark: isDark)
            }()

            let actionButton = Button(action: { store.perform(actions, scope: scope, locals: locals) }) {
                if loading {
                    ProgressView()
                } else if let icon {
                    if nexaDevUsesIconOnlyButtonLabel(fields["label"], comment: fields["comment"]) {
                        NexaIconView(icon: icon, description: "", size: 17)
                    } else {
                        Label {
                            Text(label)
                        } icon: {
                            NexaIconView(icon: icon, description: "", size: 17)
                        }
                    }
                } else {
                    Text(label)
                }
            }
            let baseButton = actionButton
                .font(.system(size: nexaDevTextBaseFontSize([:])))
                .disabled(disabled || loading)
                .tint(tint)
            // SwiftUI's swipeActions supplies the native row-action styling and
            // gesture recognizer. An explicit ButtonStyle can disable that native
            // swipe recognizer, so only pass through the supported tint override.
            baseButton
        } else {
            EmptyView()
        }
    }
}

@MainActor
struct NexaDevFastList<RowContent: View, StickyHeaderContent: View, SectionHeaderContent: View, SwipeActionContent: View>: View {
    let count: Int
    let usesPlatformList: Bool
    let showDividers: Bool
    let hideHorizontalScrollbar: Bool
    let hideVerticalScrollbar: Bool
    let axis: NexaDevFastListAxis
    let reverseLayout: Bool
    let pageSnap: Bool
    let scrollEnabled: Bool
    let rowHeight: Double?
    let spacing: Double
    let scrollPosition: Int?
    let sectionCounts: [Int]?
    let sectionOffsets: [Int]?
    let componentRef: NexaFastListRef?
    let rowKey: ((Int, Int, Int) -> AnyHashable)?
    let stickyHeader: StickyHeaderContent?
    let sectionHeader: ((Int) -> SectionHeaderContent)?
    let swipeActions: ((Int, Int, Int) -> SwipeActionContent)?
    let swipeEdge: HorizontalEdge
    let swipeAllowsFullSwipe: Bool
    let row: (Int, Int, Int) -> RowContent
    let onMove: ((IndexSet, Int) -> Void)?
    let moveEnabled: Bool
    let onScrollPositionChanged: ((Int) -> Void)?
    let onScroll: (() -> Void)?
    let onScrollBeginDrag: (() -> Void)?
    let onEndReached: (() -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    @Environment(\.nexaDevPageRestoreRevision) private var pageRestoreRevision

    private func identifiedRows(
        count: Int,
        sectionIndex: Int = 0,
        flatOffset: Int = 0
    ) -> some RandomAccessCollection<NexaDevFastListRow> {
        (0..<max(0, count)).lazy.map { itemIndex in
            let index = flatOffset + itemIndex
            return NexaDevFastListRow(
                id: rowKey?(index, sectionIndex, itemIndex) ?? AnyHashable(index),
                index: index,
                sectionIndex: sectionIndex,
                itemIndex: itemIndex
            )
        }
    }

    @ViewBuilder
    private func nativeRow(_ item: NexaDevFastListRow) -> some View {
        let content = NexaNativeListRowPrimitive(
            position: item.index,
            showDividers: showDividers,
            prioritizeSwipeActions: swipeActions != nil
        ) {
            row(item.index, item.sectionIndex, item.itemIndex)
        }
        if let swipeActions {
            content.swipeActions(edge: swipeEdge, allowsFullSwipe: swipeAllowsFullSwipe) {
                swipeActions(item.index, item.sectionIndex, item.itemIndex)
            }
        } else {
            content
        }
    }

    @ViewBuilder
    var body: some View {
        if usesPlatformList {
            NexaNativeListPrimitive(
                spacing: CGFloat(spacing),
                scrollEnabled: scrollEnabled,
                hideHorizontalScrollbar: hideHorizontalScrollbar,
                hideVerticalScrollbar: hideVerticalScrollbar,
                rowCount: count,
                componentRef: componentRef
            ) {
                if let sectionCounts, let sectionOffsets {
                    ForEach(sectionCounts.indices, id: \.self) { sectionIndex in
                        let start = sectionOffsets[sectionIndex]
                        if let sectionHeader {
                            Section {
                                ForEach(identifiedRows(
                                    count: sectionCounts[sectionIndex],
                                    sectionIndex: sectionIndex,
                                    flatOffset: start
                                )) { item in
                                    nativeRow(item)
                                }
                            } header: {
                                sectionHeader(sectionIndex)
                            }
                        } else {
                            ForEach(identifiedRows(
                                count: sectionCounts[sectionIndex],
                                sectionIndex: sectionIndex,
                                flatOffset: start
                            )) { item in
                                nativeRow(item)
                            }
                        }
                    }
                } else if let onMove {
                    ForEach(identifiedRows(count: count)) { item in
                        nativeRow(item)
                    }
                    .onMove(perform: onMove)
                    .moveDisabled(!moveEnabled)
                } else {
                    ForEach(identifiedRows(count: count)) { item in
                        nativeRow(item)
                    }
                }
            }
        } else if case .vertical = axis, sectionCounts == nil {
            if let stickyHeader {
                NexaFastListPrimitive(
                    rowCount: count,
                    rowHeight: rowHeight.map { CGFloat($0) },
                    spacing: CGFloat(spacing),
                    showDividers: showDividers,
                    hideHorizontalScrollbar: hideHorizontalScrollbar,
                    hideVerticalScrollbar: hideVerticalScrollbar,
                    scrollEnabled: scrollEnabled,
                    rowKey: rowKey.map { key in { index in key(index, 0, index) } },
                    reverseLayout: reverseLayout,
                    pageSnap: pageSnap,
                    scrollPosition: scrollPosition.map(Int32.init),
                    componentRef: componentRef,
                    restoreRevision: pageRestoreRevision,
                    onScrollPositionChanged: onScrollPositionChanged,
                    isRefreshing: isRefreshing,
                    onRefresh: onRefresh,
                    onEndReached: onEndReached,
                    onScroll: onScroll,
                    onScrollBeginDrag: onScrollBeginDrag,
                    headerContent: { stickyHeader },
                    rowContent: { index in row(index, 0, index) }
                )
            } else {
                NexaFastListPrimitive(
                    rowCount: count,
                    rowHeight: rowHeight.map { CGFloat($0) },
                    spacing: CGFloat(spacing),
                    showDividers: showDividers,
                    hideHorizontalScrollbar: hideHorizontalScrollbar,
                    hideVerticalScrollbar: hideVerticalScrollbar,
                    scrollEnabled: scrollEnabled,
                    rowKey: rowKey.map { key in { index in key(index, 0, index) } },
                    reverseLayout: reverseLayout,
                    pageSnap: pageSnap,
                    scrollPosition: scrollPosition.map(Int32.init),
                    componentRef: componentRef,
                    restoreRevision: pageRestoreRevision,
                    onScrollPositionChanged: onScrollPositionChanged,
                    isRefreshing: isRefreshing,
                    onRefresh: onRefresh,
                    onEndReached: onEndReached,
                    onScroll: onScroll,
                    onScrollBeginDrag: onScrollBeginDrag,
                    rowContent: { index in row(index, 0, index) }
                )
            }
        } else if case .horizontal = axis, sectionCounts == nil {
            NexaFastHorizontalListPrimitive(
                rowCount: count,
                itemExtent: rowHeight.map { CGFloat($0) },
                spacing: CGFloat(spacing),
                hideHorizontalScrollbar: hideHorizontalScrollbar,
                hideVerticalScrollbar: hideVerticalScrollbar,
                scrollEnabled: scrollEnabled,
                rowKey: rowKey.map { key in { index in key(index, 0, index) } },
                scrollPosition: scrollPosition.map(Int32.init),
                componentRef: componentRef,
                onScrollPositionChanged: onScrollPositionChanged,
                isRefreshing: isRefreshing,
                onRefresh: onRefresh,
                onEndReached: onEndReached,
                onScroll: onScroll,
                rowContent: { index in row(index, 0, index) }
            )
        } else if case let .grid(columns) = axis, sectionCounts == nil {
            NexaFastGridListPrimitive(
                rowCount: count,
                columns: columns,
                itemHeight: rowHeight.map { CGFloat($0) },
                spacing: CGFloat(spacing),
                hideHorizontalScrollbar: hideHorizontalScrollbar,
                hideVerticalScrollbar: hideVerticalScrollbar,
                scrollEnabled: scrollEnabled,
                rowKey: rowKey.map { key in { index in key(index, 0, index) } },
                scrollPosition: scrollPosition.map(Int32.init),
                componentRef: componentRef,
                onScrollPositionChanged: onScrollPositionChanged,
                isRefreshing: isRefreshing,
                onRefresh: onRefresh,
                onEndReached: onEndReached,
                onScroll: onScroll,
                rowContent: { index in row(index, 0, index) }
            )
        } else if case let .adaptiveGrid(minCellWidth) = axis, sectionCounts == nil {
            NexaFastGridListPrimitive(
                rowCount: count,
                columns: 1,
                minCellWidth: CGFloat(minCellWidth),
                itemHeight: rowHeight.map { CGFloat($0) },
                spacing: CGFloat(spacing),
                hideHorizontalScrollbar: hideHorizontalScrollbar,
                hideVerticalScrollbar: hideVerticalScrollbar,
                scrollEnabled: scrollEnabled,
                rowKey: rowKey.map { key in { index in key(index, 0, index) } },
                scrollPosition: scrollPosition.map(Int32.init),
                componentRef: componentRef,
                onScrollPositionChanged: onScrollPositionChanged,
                isRefreshing: isRefreshing,
                onRefresh: onRefresh,
                onEndReached: onEndReached,
                onScroll: onScroll,
                rowContent: { index in row(index, 0, index) }
            )
        } else if case let .masonry(columns) = axis, sectionCounts == nil {
            NexaFastGridListPrimitive(
                rowCount: count,
                columns: columns,
                itemHeight: nil,
                spacing: CGFloat(spacing),
                hideHorizontalScrollbar: hideHorizontalScrollbar,
                hideVerticalScrollbar: hideVerticalScrollbar,
                scrollEnabled: scrollEnabled,
                rowKey: rowKey.map { key in { index in key(index, 0, index) } },
                onMove: onMove.map { move in
                    { source, target in
                        move(IndexSet(integer: source), target > source ? target + 1 : target)
                    }
                },
                moveEnabled: moveEnabled,
                masonry: true,
                scrollPosition: scrollPosition.map(Int32.init),
                componentRef: componentRef,
                onScrollPositionChanged: onScrollPositionChanged,
                isRefreshing: isRefreshing,
                onRefresh: onRefresh,
                onEndReached: onEndReached,
                onScroll: onScroll,
                rowContent: { index in row(index, 0, index) }
            )
        } else if case .vertical = axis, let sectionCounts, let sectionOffsets {
            NexaFastSectionedListPrimitive(
                sectionCount: sectionCounts.count,
                sectionCounts: sectionCounts,
                rowHeight: rowHeight.map { CGFloat($0) },
                spacing: CGFloat(spacing),
                showDividers: showDividers,
                hideHorizontalScrollbar: hideHorizontalScrollbar,
                hideVerticalScrollbar: hideVerticalScrollbar,
                scrollEnabled: scrollEnabled,
                rowKey: rowKey.map { key in
                    { sectionIndex, itemIndex in
                        let start = sectionOffsets[sectionIndex]
                        return key(start + itemIndex, sectionIndex, itemIndex)
                    }
                },
                componentRef: componentRef,
                isRefreshing: isRefreshing,
                onRefresh: onRefresh,
                headerContent: sectionHeader,
                rowContent: { sectionIndex, itemIndex in
                    let start = sectionOffsets[sectionIndex]
                    row(start + itemIndex, sectionIndex, itemIndex)
                }
            )
        }
    }
}

@MainActor
private struct NexaDevScopedRevisionView<Content: View>: View {
    @ObservedObject var revision: NexaDevScopeRevision
    @ViewBuilder var content: () -> Content

    var body: some View {
        let _ = revision.revision
        content()
    }
}

@MainActor
private struct NexaDevToolbarRevisionView<Content: View>: View {
    @ObservedObject var revision: NexaDevScopeRevision
    @ViewBuilder var content: () -> Content

    var body: some View {
        content().id(revision.revision)
    }
}

private func nexaDevNodeContainsFastList(
    _ value: Any,
    module: [String: Any],
    visitedComponents: inout Set<String>
) -> Bool {
    if let tagged = value as? [String: Any], tagged.count == 1,
       let (kind, rawPayload) = tagged.first {
        if kind == "FastList" { return true }
        let fields = rawPayload as? [String: Any] ?? [:]
        if kind == "ComponentCall",
           let name = fields["name"] as? String,
           visitedComponents.insert(name).inserted {
            if let component = (module["components"] as? [[String: Any]])?.first(where: {
                $0["name"] as? String == name
            }), let body = component["body"] as? [Any],
               body.contains(where: {
                   nexaDevNodeContainsFastList($0, module: module, visitedComponents: &visitedComponents)
               }) {
                visitedComponents.remove(name)
                return true
            }
            visitedComponents.remove(name)
        }
        return fields.values.contains {
            nexaDevNodeContainsFastList($0, module: module, visitedComponents: &visitedComponents)
        }
    }
    if let nodes = value as? [Any] {
        return nodes.contains {
            nexaDevNodeContainsFastList($0, module: module, visitedComponents: &visitedComponents)
        }
    }
    if let fields = value as? [String: Any] {
        return fields.values.contains {
            nexaDevNodeContainsFastList($0, module: module, visitedComponents: &visitedComponents)
        }
    }
    return false
}

@MainActor
private struct NexaDevAnimatedWhen<Value: Hashable, Content: View>: View {
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

private func nexaDevTransitionStateName(_ expression: Any?) -> String? {
    guard let tagged = expression as? [String: Any] else { return nil }
    if let located = tagged["Located"] as? [String: Any] {
        return nexaDevTransitionStateName(located["expression"])
    }
    let payload = (tagged["State"] ?? tagged["AnimatedState"]) as? [Any]
    guard let name = payload?.first as? String, !name.isEmpty else { return nil }
    return name
}

private func nexaDevTransitionLocals(
    _ locals: [String: Any],
    stateName: String?,
    target: Any
) -> [String: Any] {
    guard let stateName else { return locals }
    var captured = locals
    captured[stateName] = target
    captured["__nexa_presented_animated:\(stateName)"] = target
    return captured
}

@MainActor
struct NexaDevNodeList: View {
    let nodes: [Any]
    let module: [String: Any]
    let store: NexaDevStateStore
    var parameters: [String: Any] = [:]
    var stateDeclarations: [[String: Any]]?
    var screenName: String?
    var stateScope: String = "app"
    var rendersFormRows = false
    var appliesFormRowMetrics = false
    var formShowDividers = true
    var firstFormSection = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.nexaSharedNamespace) private var nexaSharedNamespace
    @Environment(\.nexaDevContentSlot) private var contentSlot
    @Environment(\.nexaDevNodeIdentity) private var nodeIdentity
    @Environment(\.nexaDevNavigationStackIdentity) private var navigationStackIdentity
    @Environment(\.nexaDevNavigationPath) private var navigationPath
    @Environment(\.nexaVisualSafeAreaInsets) private var visualSafeInsets
    @Environment(\.nexaWindowMetrics) private var windowMetrics
    @Environment(\.layoutDirection) private var layoutDirection

    var body: some View {
        NexaDevScopedRevisionView(revision: store.revisionObserver(for: stateScope)) {
            renderedBody
        }
        .transaction { transaction in
            if reduceMotion {
                transaction.disablesAnimations = true
            }
        }
    }

    private var resolvedLocals: [String: Any] {
        var locals = resolvedStateDeclarations.map {
            store.bindings($0, parameters: parameters, scope: stateScope)
        } ?? store.locals(scope: stateScope, parameters: parameters)
        locals["__nexa_screen_width"] = Double(windowMetrics.viewportSize.width)
        locals["__nexa_screen_height"] = Double(windowMetrics.viewportSize.height)
        let safeInsets = windowMetrics.safeInsets.logical(layoutDirection)
        locals["__nexa_system_inset_top"] = Double(safeInsets.top)
        locals["__nexa_system_inset_bottom"] = Double(safeInsets.bottom)
        locals["__nexa_system_inset_leading"] = Double(safeInsets.leading)
        locals["__nexa_system_inset_trailing"] = Double(safeInsets.trailing)
        if let navigationStackIdentity {
            locals["__nexa_navigation_stack_identity"] = navigationStackIdentity
        }
        if let navigationPath {
            locals["__nexa_navigation_path_binding"] = navigationPath
        }
        return locals
    }

    private var resolvedNodes: [Any] {
        guard let screenName,
              let screen = store.screen(named: screenName),
              let screenNodes = screen["body"] as? [Any]
        else { return nodes }
        return screenNodes
    }

    private var resolvedStateDeclarations: [[String: Any]]? {
        guard let screenName,
              let screen = store.screen(named: screenName),
              let screenStates = screen["states"] as? [[String: Any]]
        else { return stateDeclarations }
        return screenStates
    }

    @ViewBuilder private var renderedBody: some View {
        let locals = resolvedLocals
        let activeNodes = resolvedNodes
        if activeNodes.isEmpty {
            EmptyView()
        } else if rendersFormRows {
            ForEach(activeNodes.indices, id: \.self) { index in
                if appliesFormRowMetrics,
                   (activeNodes[index] as? [String: Any])?.keys.first != "FormSection" {
                    NexaFormRowPrimitive(
                        minHeight: nexaDevFormRowMinHeight,
                        horizontalInset: nexaDevFormRowHorizontalInset,
                        showDividers: formShowDividers
                    ) {
                        renderNode(activeNodes[index], locals: locals, scope: stateScope, identity: "\(nodeIdentity)/\(index)")
                    }
                } else {
                    renderNode(
                        activeNodes[index],
                        locals: locals,
                        scope: stateScope,
                        identity: "\(nodeIdentity)/\(index)",
                        isFirstFormSection: firstFormSection && index == 0
                    )
                }
            }
        } else if activeNodes.count == 1 {
            renderNode(activeNodes[0], locals: locals, scope: stateScope, identity: "\(nodeIdentity)/0")
        } else {
            NexaColumnPrimitive(alignment: .center, textAlignment: .center, spacing: 0) {
                ForEach(activeNodes.indices, id: \.self) { index in
                    renderNode(activeNodes[index], locals: locals, scope: stateScope, identity: "\(nodeIdentity)/\(index)")
                }
            }
        }
    }

    private func applyViewSize(_ view: AnyView, style: [String: Any], frameAlignment: Alignment = .center) -> AnyView {
        var styled = view
        let widthDimension = nexaDevDimension(style["width"])
        let heightDimension = nexaDevDimension(style["height"])
        let minWidthDimension = nexaDevDimension(style["min_width"])
        let maxWidthDimension = nexaDevDimension(style["max_width"])
        let minHeightDimension = nexaDevDimension(style["min_height"])
        let maxHeightDimension = nexaDevDimension(style["max_height"])
        func boundValue(_ raw: NexaDevDimension?, widthAxis: Bool, minimum: Bool) -> CGFloat? {
            guard let raw else { return nil }
            switch raw {
            case .fixed(let value): return value
            case .fill: return minimum ? 0 : .infinity
            case .screenWidthFraction(let fraction) where widthAxis: return windowMetrics.viewportSize.width * fraction
            case .screenHeightFraction(let fraction) where !widthAxis: return windowMetrics.viewportSize.height * fraction
            case .parentFraction, .screenWidthFraction, .screenHeightFraction: return nil
            }
        }
        func fixedSize(_ raw: NexaDevDimension?, widthAxis: Bool) -> CGFloat? {
            guard let raw else { return nil }
            switch raw {
            case .fixed(let value): return value
            case .screenWidthFraction(let fraction) where widthAxis: return windowMetrics.viewportSize.width * fraction
            case .screenHeightFraction(let fraction) where !widthAxis: return windowMetrics.viewportSize.height * fraction
            case .parentFraction, .fill, .screenWidthFraction, .screenHeightFraction: return nil
            }
        }
        let minWidth = boundValue(minWidthDimension, widthAxis: true, minimum: true)
            ?? (widthDimension == .fill ? 0 : nil)
        let maxWidth = boundValue(maxWidthDimension, widthAxis: true, minimum: false)
            ?? (widthDimension == .fill ? .infinity : nil)
        let minHeight = boundValue(minHeightDimension, widthAxis: false, minimum: true)
            ?? (heightDimension == .fill ? 0 : nil)
        let maxHeight = boundValue(maxHeightDimension, widthAxis: false, minimum: false)
            ?? (heightDimension == .fill ? .infinity : nil)
        if minWidth != nil || maxWidth != nil || minHeight != nil || maxHeight != nil {
            styled = AnyView(styled.frame(minWidth: minWidth, maxWidth: maxWidth, minHeight: minHeight, maxHeight: maxHeight, alignment: frameAlignment))
        }
        let width = fixedSize(widthDimension, widthAxis: true)
        let height = fixedSize(heightDimension, widthAxis: false)
        if let width, let height {
            styled = AnyView(styled.frame(width: width, height: height, alignment: frameAlignment))
        } else if let width {
            styled = AnyView(styled.frame(width: width, alignment: frameAlignment))
        } else if let height {
            styled = AnyView(styled.frame(height: height, alignment: frameAlignment))
        }
        func parentFraction(_ raw: NexaDevDimension?) -> CGFloat? {
            guard let raw, case .parentFraction(let value) = raw else { return nil }
            return value
        }
        let parentWidth = parentFraction(widthDimension)
        let parentHeight = parentFraction(heightDimension)
        let parentMinWidth = parentFraction(minWidthDimension)
        let parentMaxWidth = parentFraction(maxWidthDimension)
        let parentMinHeight = parentFraction(minHeightDimension)
        let parentMaxHeight = parentFraction(maxHeightDimension)
        if parentWidth != nil || parentHeight != nil || parentMinWidth != nil || parentMaxWidth != nil || parentMinHeight != nil || parentMaxHeight != nil {
            styled = AnyView(styled.modifier(NexaParentRelativeDimensionModifier(width: parentWidth, height: parentHeight, minWidth: parentMinWidth, maxWidth: parentMaxWidth, minHeight: parentMinHeight, maxHeight: parentMaxHeight)))
        }
        if maxHeight != nil { styled = AnyView(styled.clipped()) }
        return styled
    }

    private func applyViewSurfaceAndEffects(_ view: AnyView, style: [String: Any], locals: [String: Any], scope: String) -> AnyView {
        var styled = view
        if let rawAnimation = style["animation"],
           !(rawAnimation is NSNull),
           let value = nexaDevAnimation(rawAnimation) {
            styled = AnyView(styled.modifier(NexaDevAnimateContentSizeModifier(animation: value)))
        }
        if let background = devColor(style["background"], isDark: colorScheme == .dark) {
            styled = AnyView(styled.background(background))
        }
        let dynamicCornerRadius = style["corner_radius_expression"]
            .flatMap { value -> Double? in
                guard !(value is NSNull) else { return nil }
                return (store.evaluate(value, locals: locals, scope: scope) as? NSNumber)?.doubleValue
            }
        let staticCornerRadius = (style["corner_radius"] as? NSNumber)?.doubleValue
        let bottomCornerRadius = (style["corner_radius_bottom"] as? NSNumber)?.doubleValue
        let cornerRadius = dynamicCornerRadius ?? staticCornerRadius ?? 0
        let hasCornerRadius = dynamicCornerRadius != nil
            || staticCornerRadius != nil
            || bottomCornerRadius != nil
        if hasCornerRadius {
            if let bottomCornerRadius {
                let shape = UnevenRoundedRectangle(cornerRadii: RectangleCornerRadii(
                    topLeading: cornerRadius,
                    bottomLeading: bottomCornerRadius,
                    bottomTrailing: bottomCornerRadius,
                    topTrailing: cornerRadius
                ))
                styled = AnyView(styled.clipShape(shape))
                if let borderWidth = style["border_width"] as? Double,
                   let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                    styled = AnyView(styled.overlay(shape.stroke(border, lineWidth: borderWidth)))
                }
            } else {
                let shape = RoundedRectangle(cornerRadius: cornerRadius)
                styled = AnyView(styled.clipShape(shape))
                if let borderWidth = style["border_width"] as? Double,
                   let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                    styled = AnyView(styled.overlay(shape.stroke(border, lineWidth: borderWidth)))
                }
            }
        } else if let borderWidth = style["border_width"] as? Double,
                  let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
            let shape = RoundedRectangle(cornerRadius: cornerRadius)
            styled = AnyView(styled.overlay(shape.stroke(border, lineWidth: borderWidth)))
        }
        if let opacity = style["opacity"] as? Double, opacity != 1 {
            styled = AnyView(styled.compositingGroup().opacity(opacity))
        }
        let effects = style["effects"] as? [String: Any] ?? [:]
        if let expression = effects["opacity_expression"], !(expression is NSNull),
           let opacity = store.evaluate(expression, locals: locals, scope: scope) as? Double {
            styled = AnyView(styled.compositingGroup().opacity(min(1, max(0, opacity))))
        }
        let scaleXExpression = effects["scale_expression"]
        let scaleYExpression = effects["scale_y_expression"]
        let hasScaleXExpression = scaleXExpression != nil && !(scaleXExpression is NSNull)
        let scaleX: Double
        if let expression = scaleXExpression, !(expression is NSNull) {
            scaleX = store.evaluate(expression, locals: locals, scope: scope) as? Double ?? 1
        } else {
            scaleX = effects["scale"] as? Double ?? 1
        }
        if let expression = scaleYExpression, !(expression is NSNull) {
            let scaleY = (store.evaluate(expression, locals: locals, scope: scope) as? Double)
                ?? (effects["scale_y"] as? Double) ?? scaleX
            styled = AnyView(styled.scaleEffect(x: scaleX, y: scaleY))
        } else if let scaleY = effects["scale_y"] as? Double {
            styled = AnyView(styled.scaleEffect(x: scaleX, y: scaleY))
        } else if hasScaleXExpression || effects["scale"] as? Double != nil {
            styled = AnyView(styled.scaleEffect(scaleX))
        }
        if let rawOffset = effects["offset_y_expression"], !(rawOffset is NSNull),
           let offsetY = store.evaluate(rawOffset, locals: locals, scope: scope) as? Double {
            styled = AnyView(styled.offset(y: CGFloat(offsetY)))
        }
        if let rotation = effects["rotation"] as? Double { styled = AnyView(styled.rotationEffect(.degrees(rotation))) }
        if let shadow = effects["shadow"] as? [String: Any],
           let radius = shadow["radius"] as? Double,
           let color = devColor(shadow["color"], isDark: colorScheme == .dark) {
            styled = AnyView(styled.shadow(
                color: color,
                radius: radius,
                x: shadow["x"] as? Double ?? 0,
                y: shadow["y"] as? Double ?? 0
            ))
        }
        if let blur = effects["blur"] as? Double { styled = AnyView(styled.blur(radius: blur)) }
        if let radius = effects["clip_rounded"] as? Double {
            styled = AnyView(styled.clipShape(RoundedRectangle(cornerRadius: radius)))
        }
        if let zIndex = effects["z_index"] as? Int { styled = AnyView(styled.zIndex(Double(zIndex))) }
        if let glass = effects["glass"] as? [String: Any] {
            let shapeValue = glass["shape"] as? [String: Any] ?? [:]
            let glassShape: NexaDevGlassShape
            if shapeValue["Circle"] != nil { glassShape = .circle }
            else if let radius = shapeValue["Rounded"] as? Double { glassShape = .rounded(radius) }
            else { glassShape = .capsule }
            styled = AnyView(styled.nexaDevGlass(
                tint: devColor(glass["tint"], isDark: colorScheme == .dark),
                shape: glassShape
            ))
        }
        return styled
    }

    private func applyViewStyle(_ view: AnyView, style: [String: Any], frameAlignment: Alignment = .center, locals: [String: Any], scope: String) -> AnyView {
        let sized = applyViewSize(view, style: style, frameAlignment: frameAlignment)
        return applyViewSurfaceAndEffects(sized, style: style, locals: locals, scope: scope)
    }

    private func renderNode(
        _ rawNode: Any,
        locals: [String: Any],
        scope: String,
        identity: String,
        isFirstFormSection: Bool = false
    ) -> AnyView {
        AnyView(
            renderNodeContent(
                rawNode,
                locals: locals,
                scope: scope,
                identity: identity,
                isFirstFormSection: isFirstFormSection
            )
                .environment(\.nexaDevNodeIdentity, identity)
        )
    }

    private func renderToolbarItem(
        _ rawNode: Any,
        locals: [String: Any],
        scope: String,
        identity: String
    ) -> AnyView {
        AnyView(NexaDevToolbarRevisionView(revision: store.revisionObserver(for: scope)) {
            renderNode(rawNode, locals: locals, scope: scope, identity: identity)
        })
    }

    private func renderNodeContent(
        _ rawNode: Any,
        locals: [String: Any],
        scope: String,
        identity: String,
        formContentPadding: EdgeInsets = .init(),
        formMargin: EdgeInsets = .init(),
        isFirstFormSection: Bool = false
    ) -> AnyView {
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
        case "ViewInsets":
            func resolvedInsets(_ raw: Any?) -> EdgeInsets {
                let values = raw as? [String: Any] ?? [:]
                func value(_ edge: String) -> CGFloat {
                    guard let expression = values[edge], !(expression is NSNull) else { return 0 }
                    return max(0, CGFloat((store.evaluate(expression, locals: locals, scope: scope) as? NSNumber)?.doubleValue ?? 0))
                }
                return EdgeInsets(top: value("top"), leading: value("leading"), bottom: value("bottom"), trailing: value("trailing"))
            }
            let child = fields["child"] ?? NSNull()
            let padding = resolvedInsets(fields["padding"])
            let margin = resolvedInsets(fields["margin"])
            if let taggedChild = child as? [String: Any], taggedChild["Form"] != nil {
                return renderNodeContent(
                    child,
                    locals: locals,
                    scope: scope,
                    identity: "\(identity)/form",
                    formContentPadding: padding,
                    formMargin: margin
                )
            }
            return AnyView(NexaViewInsetsPrimitive(
                padding: padding,
                margin: margin,
                scrollContent: (child as? [String: Any]).map { node in
                    node["ScrollView"] != nil || node["FastList"] != nil || node["KeyboardAware"] != nil || node["RefreshControl"] != nil
                } ?? false,
                usesScrollMarginLayout: (child as? [String: Any])?["ScrollView"] != nil
            ) {
                renderNode(child, locals: locals, scope: scope, identity: "\(identity)/insets")
            })
        case "SafeAreaInset":
            let plan = fields["plan"] as? [String: Any] ?? [:]
            let edge: Edge = switch plan["edge"] as? String {
            case "Top": .top
            case "Leading": .leading
            case "Trailing": .trailing
            default: .bottom
            }
            let scrollOwner = plan["scroll_owner"]
            let isUniqueOwner = (scrollOwner as? [String: Any])?["Unique"] != nil
            let accommodatesEndpoint = isUniqueOwner || (scrollOwner as? String) == "RouteHost"
            let children = fields["children"] as? [Any] ?? []
            let attached = fields["content"] as? [Any] ?? []
            return AnyView(NexaDevSafeAreaInsetPrimitive(
                edge: edge,
                accommodatesEndpoint: accommodatesEndpoint
            ) {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            } attachedContent: {
                NexaDevNodeList(nodes: attached, module: module, store: store, parameters: locals, stateScope: "\(scope)/safe-area-inset")
            })
        case "ScrollHeader":
            let plan = fields["plan"] as? [String: Any] ?? [:]
            let behavior: NexaAdaptiveHeaderBehavior = switch plan["behavior"] as? String {
            case "Collapse": .collapse
            case "HideOnScroll": .hideOnScroll
            default: .pinned
            }
            let collapsedHeight = CGFloat(plan["collapsed_height"] as? Double ?? 56)
            let header = fields["header"] as? [Any] ?? []
            let children = fields["children"] as? [Any] ?? []
            return AnyView(NexaAdaptiveScrollHeaderPrimitive(
                behavior: behavior,
                collapsedHeight: collapsedHeight,
                header: {
                    NexaDevNodeList(nodes: header, module: module, store: store, parameters: locals, stateScope: "\(scope)/scroll-header")
                },
                content: {
                    NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
                }
            ))
        case "Appearance":
            let mode = store.stringify(store.evaluate(fields["mode"] ?? "system", locals: locals, scope: scope))
            let children = fields["children"] as? [Any] ?? []
            let content = NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            return AnyView(content.preferredColorScheme(mode == "dark" ? .dark : mode == "light" ? .light : nil))
        case "Form":
            let children = fields["children"] as? [Any] ?? []
            let firstChildIsSection = (children.first as? [String: Any])?["FormSection"] != nil
            let showDividers = fields["show_dividers"] as? Bool ?? true
            let background = devColor(fields["background"], isDark: colorScheme == .dark)
            return AnyView(NexaFormPrimitive(
                background: background,
                hideHorizontalScrollbar: fields[NexaDevKeys.hideHorizontalScrollbar] as? Bool ?? false,
                hideVerticalScrollbar: fields[NexaDevKeys.hideVerticalScrollbar] as? Bool ?? false,
                contentPadding: formContentPadding,
                margin: formMargin,
                firstChildIsSection: firstChildIsSection
            ) {
                NexaDevNodeList(
                    nodes: children,
                    module: module,
                    store: store,
                    parameters: locals,
                    stateScope: scope,
                    rendersFormRows: true,
                    appliesFormRowMetrics: true,
                    formShowDividers: showDividers,
                    firstFormSection: firstChildIsSection
                )
            })
        case "FormSection":
            let children = fields["children"] as? [Any] ?? []
            let showDividers = fields["show_dividers"] as? Bool ?? formShowDividers
            let background = devColor(fields["background"], isDark: colorScheme == .dark)
            let section = NexaFormSectionPrimitive(
                background: background,
                isFirstSection: isFirstFormSection,
                content: {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope, rendersFormRows: true, appliesFormRowMetrics: true, formShowDividers: showDividers)
            }, header: {
                if let rawTitle = fields["title"], !(rawTitle is NSNull) {
                    Text(store.stringify(store.evaluate(rawTitle, locals: locals, scope: scope)))
                } else {
                    EmptyView()
                }
            }, footer: {
                if let rawFooter = fields["footer"], !(rawFooter is NSNull) {
                    Text(store.stringify(store.evaluate(rawFooter, locals: locals, scope: scope)))
                } else {
                    EmptyView()
                }
            })
            return AnyView(section)
        case "ScrollView":
            let axis: Axis.Set = fields[NexaDevKeys.axis] as? String == "Horizontal" ? .horizontal : .vertical
            let children = fields["children"] as? [Any] ?? []
            let componentRef = fields["component_ref"].flatMap { value -> NexaScrollViewRef? in
                guard !(value is NSNull) else { return nil }
                return store.evaluate(value, locals: locals, scope: scope) as? NexaScrollViewRef
            }
            let content = NexaScrollContent {
                if axis == .horizontal {
                    NexaRowPrimitive(alignment: .center, spacing: 0) {
                        ForEach(children.indices, id: \.self) { index in
                            renderNode(children[index], locals: locals, scope: scope, identity: "\(identity)/children/\(index)")
                        }
                    }
                } else {
                    NexaColumnPrimitive(alignment: .leading, textAlignment: .leading, spacing: 0) {
                        ForEach(children.indices, id: \.self) { index in
                            renderNode(children[index], locals: locals, scope: scope, identity: "\(identity)/children/\(index)")
                        }
                    }
                }
            }
            var scrollView = AnyView(NexaScrollViewPrimitive(
                axis: axis,
                hideHorizontalScrollbar: fields[NexaDevKeys.hideHorizontalScrollbar] as? Bool ?? true,
                hideVerticalScrollbar: fields[NexaDevKeys.hideVerticalScrollbar] as? Bool ?? true,
                componentRef: componentRef
            ) { content })
            let style = fields[NexaDevKeys.style] as? [String: Any] ?? [:]
            scrollView = AnyView(scrollView.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading))
            let widthDimension = nexaDevDimension(style["width"])
            let heightDimension = nexaDevDimension(style["height"])
            let minWidthDimension = nexaDevDimension(style["min_width"])
            let maxWidthDimension = nexaDevDimension(style["max_width"])
            let minHeightDimension = nexaDevDimension(style["min_height"])
            let maxHeightDimension = nexaDevDimension(style["max_height"])
            func boundValue(_ raw: NexaDevDimension?, widthAxis: Bool, minimum: Bool) -> CGFloat? {
                guard let raw else { return nil }
                switch raw {
                case .fixed(let value): return value
                case .fill: return minimum ? 0 : .infinity
                case .screenWidthFraction(let fraction) where widthAxis: return windowMetrics.viewportSize.width * fraction
                case .screenHeightFraction(let fraction) where !widthAxis: return windowMetrics.viewportSize.height * fraction
                case .parentFraction, .screenWidthFraction, .screenHeightFraction: return nil
                }
            }
            func fixedSize(_ raw: NexaDevDimension?, widthAxis: Bool) -> CGFloat? {
                guard let raw else { return nil }
                switch raw {
                case .fixed(let value): return value
                case .screenWidthFraction(let fraction) where widthAxis: return windowMetrics.viewportSize.width * fraction
                case .screenHeightFraction(let fraction) where !widthAxis: return windowMetrics.viewportSize.height * fraction
                case .parentFraction, .fill, .screenWidthFraction, .screenHeightFraction: return nil
                }
            }
            let minWidth = boundValue(minWidthDimension, widthAxis: true, minimum: true)
                ?? (widthDimension == .fill ? 0 : nil)
            let maxWidth = boundValue(maxWidthDimension, widthAxis: true, minimum: false)
                ?? (widthDimension == .fill ? .infinity : nil)
            let minHeight = boundValue(minHeightDimension, widthAxis: false, minimum: true)
                ?? (heightDimension == .fill ? 0 : nil)
            let maxHeight = boundValue(maxHeightDimension, widthAxis: false, minimum: false)
                ?? (heightDimension == .fill ? .infinity : nil)
            if minWidth != nil || maxWidth != nil || minHeight != nil || maxHeight != nil {
                scrollView = AnyView(scrollView.frame(
                    minWidth: minWidth,
                    maxWidth: maxWidth,
                    minHeight: minHeight,
                    maxHeight: maxHeight,
                    alignment: .topLeading
                ))
            }
            let width = fixedSize(widthDimension, widthAxis: true)
            let height = fixedSize(heightDimension, widthAxis: false)
            if let width, let height {
                scrollView = AnyView(scrollView.frame(width: width, height: height, alignment: .topLeading))
            } else if let width {
                scrollView = AnyView(scrollView.frame(width: width, alignment: .topLeading))
            } else if let height {
                scrollView = AnyView(scrollView.frame(height: height, alignment: .topLeading))
            }
            func parentFraction(_ raw: NexaDevDimension?) -> CGFloat? {
                guard let raw, case .parentFraction(let value) = raw else { return nil }
                return value
            }
            let parentWidth = parentFraction(widthDimension)
            let parentHeight = parentFraction(heightDimension)
            let parentMinWidth = parentFraction(minWidthDimension)
            let parentMaxWidth = parentFraction(maxWidthDimension)
            let parentMinHeight = parentFraction(minHeightDimension)
            let parentMaxHeight = parentFraction(maxHeightDimension)
            if parentWidth != nil || parentHeight != nil || parentMinWidth != nil || parentMaxWidth != nil || parentMinHeight != nil || parentMaxHeight != nil {
                scrollView = AnyView(scrollView.modifier(NexaParentRelativeDimensionModifier(
                    width: parentWidth,
                    height: parentHeight,
                    minWidth: parentMinWidth,
                    maxWidth: parentMaxWidth,
                    minHeight: parentMinHeight,
                    maxHeight: parentMaxHeight
                )))
            }
            if maxHeight != nil { scrollView = AnyView(scrollView.clipped()) }
            if let rawAnimation = style["animation"],
               !(rawAnimation is NSNull),
               let value = nexaDevAnimation(rawAnimation) {
                scrollView = AnyView(scrollView.modifier(NexaDevAnimateContentSizeModifier(animation: value)))
            }
            if let background = devColor(style["background"], isDark: colorScheme == .dark) {
                scrollView = AnyView(scrollView.background(background))
            }
            let dynamicCornerRadius = style["corner_radius_expression"]
                .flatMap { value -> Double? in
                    guard !(value is NSNull) else { return nil }
                    return (store.evaluate(value, locals: locals, scope: scope) as? NSNumber)?.doubleValue
                }
            let staticCornerRadius = (style["corner_radius"] as? NSNumber)?.doubleValue
            let bottomCornerRadius = (style["corner_radius_bottom"] as? NSNumber)?.doubleValue
            let cornerRadius = dynamicCornerRadius ?? staticCornerRadius ?? 0
            let hasCornerRadius = dynamicCornerRadius != nil
                || staticCornerRadius != nil
                || bottomCornerRadius != nil
            if hasCornerRadius {
                if let bottomCornerRadius {
                    let shape = UnevenRoundedRectangle(cornerRadii: RectangleCornerRadii(
                        topLeading: cornerRadius,
                        bottomLeading: bottomCornerRadius,
                        bottomTrailing: bottomCornerRadius,
                        topTrailing: cornerRadius
                    ))
                    scrollView = AnyView(scrollView.clipShape(shape))
                    if let borderWidth = style["border_width"] as? Double,
                       let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                        scrollView = AnyView(scrollView.overlay(shape.stroke(border, lineWidth: borderWidth)))
                    }
                } else {
                    let shape = RoundedRectangle(cornerRadius: cornerRadius)
                    scrollView = AnyView(scrollView.clipShape(shape))
                    if let borderWidth = style["border_width"] as? Double,
                       let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                        scrollView = AnyView(scrollView.overlay(shape.stroke(border, lineWidth: borderWidth)))
                    }
                }
            } else if let borderWidth = style["border_width"] as? Double,
                      let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                let shape = RoundedRectangle(cornerRadius: cornerRadius)
                scrollView = AnyView(scrollView.overlay(shape.stroke(border, lineWidth: borderWidth)))
            }
            if let opacity = style["opacity"] as? Double, opacity != 1 {
                scrollView = AnyView(scrollView.compositingGroup().opacity(opacity))
            }
            let effects = style["effects"] as? [String: Any] ?? [:]
            if let expression = effects["opacity_expression"], !(expression is NSNull),
               let opacity = store.evaluate(expression, locals: locals, scope: scope) as? Double {
                scrollView = AnyView(scrollView.compositingGroup().opacity(min(1, max(0, opacity))))
            }
            let scaleXExpression = effects["scale_expression"]
            let scaleYExpression = effects["scale_y_expression"]
            let hasScaleXExpression = scaleXExpression != nil && !(scaleXExpression is NSNull)
            let scaleX: Double
            if let expression = scaleXExpression, !(expression is NSNull) {
                scaleX = store.evaluate(expression, locals: locals, scope: scope) as? Double ?? 1
            } else {
                scaleX = effects["scale"] as? Double ?? 1
            }
            if let expression = scaleYExpression, !(expression is NSNull) {
                let scaleY = (store.evaluate(expression, locals: locals, scope: scope) as? Double)
                    ?? (effects["scale_y"] as? Double) ?? scaleX
                scrollView = AnyView(scrollView.scaleEffect(x: scaleX, y: scaleY))
            } else if let scaleY = effects["scale_y"] as? Double {
                scrollView = AnyView(scrollView.scaleEffect(x: scaleX, y: scaleY))
            } else if hasScaleXExpression || effects["scale"] as? Double != nil {
                scrollView = AnyView(scrollView.scaleEffect(scaleX))
            }
            if let rawOffset = effects["offset_y_expression"], !(rawOffset is NSNull),
               let offsetY = store.evaluate(rawOffset, locals: locals, scope: scope) as? Double {
                scrollView = AnyView(scrollView.offset(y: CGFloat(offsetY)))
            }
            if let rotation = effects["rotation"] as? Double {
                scrollView = AnyView(scrollView.rotationEffect(.degrees(rotation)))
            }
            if let shadow = effects["shadow"] as? [String: Any],
               let radius = shadow["radius"] as? Double,
               let color = devColor(shadow["color"], isDark: colorScheme == .dark) {
                scrollView = AnyView(scrollView.shadow(
                    color: color,
                    radius: radius,
                    x: shadow["x"] as? Double ?? 0,
                    y: shadow["y"] as? Double ?? 0
                ))
            }
            if let blur = effects["blur"] as? Double {
                scrollView = AnyView(scrollView.blur(radius: blur))
            }
            if let radius = effects["clip_rounded"] as? Double {
                scrollView = AnyView(scrollView.clipShape(RoundedRectangle(cornerRadius: radius)))
            }
            if let zIndex = effects["z_index"] as? Int {
                scrollView = AnyView(scrollView.zIndex(Double(zIndex)))
            }
            if let glass = effects["glass"] as? [String: Any] {
                let shapeValue = glass["shape"] as? [String: Any] ?? [:]
                let glassShape: NexaDevGlassShape
                if shapeValue["Circle"] != nil { glassShape = .circle }
                else if let radius = shapeValue["Rounded"] as? Double { glassShape = .rounded(radius) }
                else { glassShape = .capsule }
                scrollView = AnyView(scrollView.nexaDevGlass(
                    tint: devColor(glass["tint"], isDark: colorScheme == .dark),
                    shape: glassShape
                ))
            }
            return scrollView
        case "Layout":
            let children = fields["children"] as? [Any] ?? []
            let toolbarNodes = children.compactMap { rawNode -> [String: Any]? in
                guard let node = rawNode as? [String: Any],
                      let payload = node["Toolbar"] as? [String: Any]
                else { return nil }
                return payload
            }
            let bottomSheetNodes = children.filter { rawNode in
                guard let node = rawNode as? [String: Any] else { return false }
                return node.keys.first == "BottomSheet"
            }
            let contentChildren = children.filter { rawNode in
                guard let node = rawNode as? [String: Any] else { return true }
                return node.keys.first != "Toolbar" && node.keys.first != "BottomSheet"
            }
            let spacing = fields["gap"] as? Double ?? 0
            let style = fields[NexaDevKeys.style] as? [String: Any] ?? [:]
            let rawAlignment = fields["alignment"]
            let alignment = rawAlignment as? String
                ?? (rawAlignment as? [String: Any])?.keys.first
                ?? style["alignment"] as? String
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
            let columnTextAlignment: TextAlignment = switch alignment {
            case "Start": .leading
            case "End": .trailing
            default: .center
            }
            let frameAlignment: Alignment = switch fields["kind"] as? String {
            case "Row":
                switch alignment {
                case "Start": .topLeading
                case "End": .bottomLeading
                default: .leading
                }
            case "Stack": stackAlignment
            default:
                switch alignment {
                case "Start": .topLeading
                case "End": .topTrailing
                default: .top
                }
            }
            let nativeSpacing = CGFloat(spacing)
            let horizontalArrangement: NexaRowArrangement? = switch style["horizontal_arrangement"] as? String {
            case "Start": .start
            case "Center": .center
            case "End": .end
            default: nil
            }
            let rowHasFlexibleSpacer = (fields["kind"] as? String == "Row") && contentChildren.contains { child in
                guard let node = child as? [String: Any],
                      let spacer = node["Spacer"] as? [String: Any] else { return false }
                let width = spacer["width"]
                let height = spacer["height"]
                return (width == nil || width is NSNull) && (height == nil || height is NSNull)
            }
            var stackOverlayStart: Int?
            if fields["kind"] as? String == "Stack" {
                var hasFastListBelow = false
                for index in contentChildren.indices {
                    if hasFastListBelow {
                        stackOverlayStart = index
                        break
                    }
                    var visitedComponents = Set<String>()
                    hasFastListBelow = nexaDevNodeContainsFastList(
                        contentChildren[index],
                        module: module,
                        visitedComponents: &visitedComponents
                    )
                }
            }
            let container: AnyView
            switch fields["kind"] as? String {
            case "Row":
                container = AnyView(NexaRowPrimitive(alignment: rowAlignment, spacing: nativeSpacing, horizontalArrangement: horizontalArrangement) {
                    ForEach(contentChildren.indices, id: \.self) { index in
                        let child = contentChildren[index]
                        let rendered = renderNode(child, locals: locals, scope: scope, identity: "\(identity)/children/\(index)")
                        let isFlexibleSpacer: Bool = {
                            guard let node = child as? [String: Any],
                                  let spacer = node["Spacer"] as? [String: Any] else { return false }
                            let width = spacer["width"]
                            let height = spacer["height"]
                            return (width == nil || width is NSNull) && (height == nil || height is NSNull)
                        }()
                        if rowHasFlexibleSpacer && !isFlexibleSpacer {
                            rendered.layoutPriority(1)
                        } else {
                            rendered
                        }
                    }
                })
            case "Stack":
                container = AnyView(NexaStackPrimitive(alignment: stackAlignment) {
                    ForEach(0..<(stackOverlayStart ?? contentChildren.count), id: \.self) { index in
                        NexaStackLayer(order: index) {
                            renderNode(contentChildren[index], locals: locals, scope: scope, identity: "\(identity)/children/\(index)")
                        }
                    }
                })
            default:
                container = AnyView(NexaColumnPrimitive(alignment: columnAlignment, textAlignment: columnTextAlignment, spacing: nativeSpacing) { ForEach(contentChildren.indices, id: \.self) { index in renderNode(contentChildren[index], locals: locals, scope: scope, identity: "\(identity)/children/\(index)") } })
            }
            var styled = container
            if let marquee = style["marquee"] as? [String: Any] {
                let source = container
                    .environment(\.nexaDevContentSlot, contentSlot)
                    .environment(\.nexaDevNodeIdentity, nodeIdentity)
                    .environment(\.nexaSharedNamespace, nexaSharedNamespace)
                styled = AnyView(NexaMarqueePrimitive(
                    horizontal: (fields["kind"] as? String) == "Row",
                    fillsCrossAxis: nexaDevDimension(style[(fields["kind"] as? String) == "Row" ? "height" : "width"]) != nil,
                    speed: marquee["speed"] as? Double ?? 32,
                    gap: marquee["gap"] as? Double ?? 24,
                    delay: marquee["delay"] as? Double ?? 1,
                    edgeFade: marquee["edge_fade"] as? Double ?? 0,
                    playing: store.truthy(store.evaluate(marquee["playing"], locals: locals, scope: scope)),
                    revision: nexaDevMarqueeRevision(children, store: store, locals: locals, scope: scope)
                ) { source })
            }
            if let stackOverlayStart {
                // Keep Stack overlays anchored to the parent's full proposal.
                // Otherwise a UIKit-backed AutoGrid reports an intrinsic height
                // smaller than the screen and the tab bar is placed halfway up.
                styled = AnyView(styled.frame(maxWidth: .infinity, maxHeight: .infinity))
                for index in stackOverlayStart..<contentChildren.count {
                    styled = AnyView(styled.overlay(alignment: stackAlignment) {
                        NexaStackLayer(order: index) {
                            renderNode(contentChildren[index], locals: locals, scope: scope, identity: "\(identity)/children/\(index)")
                        }
                    })
                }
            }
            styled = applyViewStyle(styled, style: style, frameAlignment: frameAlignment, locals: locals, scope: scope)
            if !bottomSheetNodes.isEmpty {
                styled = AnyView(styled.background {
                    ZStack {
                        ForEach(bottomSheetNodes.indices, id: \.self) { index in
                            renderNode(bottomSheetNodes[index], locals: locals, scope: scope, identity: "\(identity)/background/\(index)")
                        }
                    }
                })
            }
            if !toolbarNodes.isEmpty {
                styled = AnyView(styled.toolbar {
                    ForEach(toolbarNodes.indices, id: \.self) { toolbarIndex in
                        let toolbar = toolbarNodes[toolbarIndex]
                        let placement: ToolbarItemPlacement = toolbar["placement"] as? String == "Leading"
                            ? .navigationBarLeading
                            : .navigationBarTrailing
                        let items = toolbar["children"] as? [Any] ?? []
                        ForEach(items.indices, id: \.self) { itemIndex in
                            ToolbarItem(placement: placement) {
                                renderToolbarItem(items[itemIndex], locals: locals, scope: scope, identity: "\(identity)/toolbar/\(toolbarIndex)/\(itemIndex)")
                            }
                        }
                    }
                })
            }
            return styled
        case "ContentUnavailable":
            let title = store.stringify(store.evaluate(fields["title"] ?? "", locals: locals, scope: scope))
            let description: Text?
            if let rawDescription = fields["description"], !(rawDescription is NSNull) {
                let value = store.stringify(store.evaluate(rawDescription, locals: locals, scope: scope))
                description = Text(value)
            } else {
                description = nil
            }
            let icon = nexaDevIconSpec(fields["icon"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
                ?? NexaIconSpec(symbol: "questionmark")
            func color(_ name: String) -> Color? {
                guard let value = fields[name] as? [String: Any] else { return nil }
                if let dynamic = value["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(value, isDark: colorScheme == .dark)
            }
            return AnyView(NexaContentUnavailablePrimitive(
                title: Text(title),
                icon: icon,
                description: description,
                titleColor: color("title_color"),
                descriptionColor: color("description_color"),
                iconTint: color("icon_tint"),
                background: color("background")
            ))
        case "Text":
            let text = store.stringify(store.evaluate(fields["value"] ?? "", locals: locals, scope: scope))
            let style = fields[NexaDevKeys.style] as? [String: Any] ?? [:]
            let alignment: TextAlignment? = switch style["alignment"] as? String {
            case "Leading": .leading
            case "Center": .center
            case "Trailing": .trailing
            default: nil
            }
            let textStyle: Font.TextStyle? = switch style["font_style"] as? String {
            case "LargeTitle": .largeTitle
            case "Title": .title
            case "Title2": .title2
            case "Title3": .title3
            case "Headline": .headline
            case "Subheadline": .subheadline
            case "Body": .body
            case "Callout": .callout
            case "Footnote": .footnote
            case "Caption": .caption
            case "Caption2": .caption2
            default: nil
            }
            let fontDesign: Font.Design? = switch style["font_design"] as? String {
            case "Default": .default
            case "Serif": .serif
            case "Rounded": .rounded
            case "Monospaced": .monospaced
            default: nil
            }
            let lineHeightFontSize = nexaDevTextBaseFontSize(style)
            let lineHeightFactor = (style["line_height"] as? Double).map {
                CGFloat($0) / lineHeightFontSize
            }
            let weight: Font.Weight? = switch style["font_weight"] as? String {
            case "Normal": .regular
            case "Medium": .medium
            case "Semibold": .semibold
            case "Bold": .bold
            default: nil
            }
            let semanticWeight: Font.Weight? = if style["font_size"] != nil,
                                                   style["font_style"] as? String == "Headline" {
                .semibold
            } else {
                nil
            }
            let resolvedFont: Font?
            if let fontFamily = style["font_family"] as? String {
                let faceWeight: String = switch style["font_weight"] as? String {
                case "Normal": "Normal"
                case "Medium": "Medium"
                case "Semibold": "Semibold"
                case "Bold": "Bold"
                default: style["font_style"] as? String == "Headline" ? "Semibold" : "Normal"
                }
                let faceName = nexaDevFontFaceName(fontFamily, weight: faceWeight)
                resolvedFont = Font.custom(faceName, size: lineHeightFontSize, relativeTo: textStyle ?? .body)
            } else if let fontSize = style["font_size"] as? Double {
                resolvedFont = Font.system(size: fontSize, design: fontDesign)
            } else if let textStyle {
                resolvedFont = Font.system(textStyle, design: fontDesign)
            } else if let fontDesign {
                resolvedFont = Font.system(.body, design: fontDesign)
            } else {
                resolvedFont = nil
            }
            var nativeStyle = NexaTextStyle()
            if let alignment {
                nativeStyle = nativeStyle.withAlignment(alignment)
            }
            if let color = devColor(style["color"], isDark: colorScheme == .dark) {
                nativeStyle = nativeStyle.withColor(color)
            }
            if let resolvedFont { nativeStyle = nativeStyle.withFont(resolvedFont) }
            if !(style["font_family"] is String), let weight = weight ?? semanticWeight {
                nativeStyle = nativeStyle.withWeight(weight)
            }
            if let lineLimit = style["line_limit"] as? Int {
                nativeStyle = nativeStyle.withLineLimit(lineLimit)
            }
            if let lineHeightFactor {
                nativeStyle = nativeStyle.withLineHeight(
                    factor: lineHeightFactor,
                    fontSize: lineHeightFontSize
                )
            }
            if let tracking = style["letter_spacing"] as? CGFloat, tracking != 0 {
                nativeStyle = nativeStyle.withTracking(tracking)
            }
            if style["strikethrough"] as? Bool == true {
                nativeStyle = nativeStyle.withStrikethrough()
            }
            if style["italic"] as? Bool == true {
                nativeStyle = nativeStyle.withItalic()
            }
            if style["underline"] as? Bool == true {
                nativeStyle = nativeStyle.withUnderline()
            }
            if style["selectable"] as? Bool == true {
                nativeStyle = nativeStyle.withSelectable()
            }
            let reveal = style["reveal"] as? [String: Any]
            let numberFlow = style["number_flow"] as? [String: Any]
            let styledText: AnyView
            if let numberFlow {
                let value = store.number(store.evaluate(fields["value"] ?? 0, locals: locals, scope: scope))
                styledText = AnyView(NexaNumberFlowPrimitive(
                    value: value,
                    format: numberFlow["format"] as? String ?? "Number",
                    currencyCode: numberFlow["currency_code"] as? String,
                    minimumFractionDigits: numberFlow["minimum_fraction_digits"] as? Int,
                    maximumFractionDigits: numberFlow["maximum_fraction_digits"] as? Int,
                    animation: nexaDevNumberFlowAnimation(numberFlow["animation"]),
                    fontSize: lineHeightFontSize,
                    style: nativeStyle
                ))
            } else if let reveal {
                let segments = store.evaluate(reveal["segments"] ?? NSNull(), locals: locals, scope: scope) as? [Any] ?? []
                let field: (Any, String) -> Any? = { segment, name in
                    if let fields = segment as? [String: Any] { return fields[name] }
                    if let instance = segment as? NexaDevUserClassInstance { return instance.fields[name] }
                    return nil
                }
                let revealWeight: UIFont.Weight = switch style["font_weight"] as? String {
                case "Medium": .medium
                case "Semibold": .semibold
                case "Bold": .bold
                default: .regular
                }
                let revealFontName = (style["font_family"] as? String).map { family in
                    let faceWeight: String = switch style["font_weight"] as? String {
                    case "Medium": "Medium"
                    case "Semibold": "Semibold"
                    case "Bold": "Bold"
                    default: "Normal"
                    }
                    return nexaDevFontFaceName(family, weight: faceWeight)
                }
                styledText = AnyView(NexaTextRevealPrimitive(
                    text: text,
                    segments: segments,
                    textOf: { segment in field(segment, "text") as? String ?? "" },
                    startTimeOf: { segment in store.number(field(segment, "startTimeSeconds") ?? 0) },
                    endTimeOf: { segment in store.number(field(segment, "endTimeSeconds") ?? 0) },
                    hasTrailingSpaceOf: { segment in field(segment, "hasTrailingSpace") as? Bool ?? true },
                    position: store.number(store.evaluate(reveal["position"] ?? NSNull(), locals: locals, scope: scope)),
                    isActive: store.truthy(store.evaluate(reveal["is_active"] ?? NSNull(), locals: locals, scope: scope)),
                    isPlaying: store.truthy(store.evaluate(reveal["is_playing"] ?? NSNull(), locals: locals, scope: scope)),
                    revealEnabled: store.truthy(store.evaluate(reveal["reveal_enabled"] ?? NSNull(), locals: locals, scope: scope)),
                    seekRevision: Int64(store.number(store.evaluate(reveal["seek_revision"] ?? NSNull(), locals: locals, scope: scope))),
                    dimOpacity: reveal["dim_opacity"] as? CGFloat ?? 0.38,
                    leadTime: reveal["lead_time"] as? Double ?? 0.3,
                    targetOffset: reveal["target_offset"] as? Double ?? 0.56,
                    smoothingDuration: reveal["smoothing_duration"] as? Double ?? 0.25,
                    fontName: revealFontName,
                    fontSize: lineHeightFontSize,
                    fontWeight: revealWeight,
                    fontDesign: style["font_design"] as? String ?? "Default",
                    color: UIColor(devColor(style["color"], isDark: colorScheme == .dark) ?? Color.primary),
                    alignment: style["alignment"] as? String ?? "Natural",
                    lineHeight: style["line_height"] as? CGFloat ?? 0,
                    letterSpacing: style["letter_spacing"] as? CGFloat ?? 0,
                    maxLines: style["line_limit"] as? Int ?? 0,
                    italic: style["italic"] as? Bool == true,
                    underline: style["underline"] as? Bool == true,
                    strikethrough: style["strikethrough"] as? Bool == true
                ))
            } else {
                styledText = AnyView(NexaTextPrimitive(text: Text(text), style: nativeStyle))
            }
            var styled = styledText
            if let padding = style["padding"] as? Double { styled = AnyView(styled.padding(padding)) }
            let effects = style["effects"] as? [String: Any] ?? [:]
            let scaleXExpression = effects["scale_expression"]
            let scaleYExpression = effects["scale_y_expression"]
            let hasScaleXExpression = scaleXExpression != nil && !(scaleXExpression is NSNull)
            let hasScaleX = hasScaleXExpression || (effects["scale"] as? Double != nil)
            if hasScaleX {
                styled = AnyView(styled.fixedSize(horizontal: true, vertical: false))
            }
            if let opacity = style["opacity"] as? Double, opacity != 1 {
                styled = AnyView(styled.opacity(opacity))
            }
            if let expression = effects["opacity_expression"], !(expression is NSNull),
               let opacity = store.evaluate(expression, locals: locals, scope: scope) as? Double {
                styled = AnyView(styled.compositingGroup().opacity(min(1, max(0, opacity))))
            }
            let scaleX: Double
            if let expression = scaleXExpression, !(expression is NSNull) {
                scaleX = store.evaluate(expression, locals: locals, scope: scope) as? Double ?? 1
            } else {
                scaleX = effects["scale"] as? Double ?? 1
            }
            if let expression = scaleYExpression, !(expression is NSNull) {
                let scaleY = (store.evaluate(expression, locals: locals, scope: scope) as? Double)
                    ?? (effects["scale_y"] as? Double) ?? scaleX
                styled = AnyView(styled.scaleEffect(x: scaleX, y: scaleY))
            } else if let scaleY = effects["scale_y"] as? Double {
                styled = AnyView(styled.scaleEffect(x: scaleX, y: scaleY))
            } else if hasScaleX {
                styled = AnyView(styled.scaleEffect(scaleX))
            }
            if let rotation = effects["rotation"] as? Double { styled = AnyView(styled.rotationEffect(.degrees(rotation))) }
            if let shadow = effects["shadow"] as? [String: Any],
               let radius = shadow["radius"] as? Double,
               let color = devColor(shadow["color"], isDark: colorScheme == .dark) {
                styled = AnyView(styled.shadow(
                    color: color,
                    radius: radius,
                    x: shadow["x"] as? Double ?? 0,
                    y: shadow["y"] as? Double ?? 0
                ))
            }
            if let blur = effects["blur"] as? Double { styled = AnyView(styled.blur(radius: blur)) }
            if let radius = effects["clip_rounded"] as? Double {
                styled = AnyView(styled.clipShape(RoundedRectangle(cornerRadius: radius)))
            }
            if let zIndex = effects["z_index"] as? Int { styled = AnyView(styled.zIndex(Double(zIndex))) }
            if let glass = effects["glass"] as? [String: Any] {
                let shapeValue = glass["shape"] as? [String: Any] ?? [:]
                let glassShape: NexaDevGlassShape
                if shapeValue["Circle"] != nil { glassShape = .circle }
                else if let radius = shapeValue["Rounded"] as? Double { glassShape = .rounded(radius) }
                else { glassShape = .capsule }
                styled = AnyView(styled.nexaDevGlass(
                    tint: devColor(glass["tint"], isDark: colorScheme == .dark),
                    shape: glassShape
                ))
            }
            return styled
        case "Spacer":
            let widthDimension = nexaDevDimension(fields["width"])
            let heightDimension = nexaDevDimension(fields["height"])
            if widthDimension == nil && heightDimension == nil {
                return AnyView(Spacer(minLength: 0).layoutPriority(-1))
            }
            func fixedSize(_ dimension: NexaDevDimension?, widthAxis: Bool) -> CGFloat? {
                guard let dimension else { return nil }
                switch dimension {
                case .fixed(let value): return value
                case .screenWidthFraction(let fraction) where widthAxis:
                    return windowMetrics.viewportSize.width * fraction
                case .screenHeightFraction(let fraction) where !widthAxis:
                    return windowMetrics.viewportSize.height * fraction
                case .parentFraction, .fill, .screenWidthFraction, .screenHeightFraction:
                    return nil
                }
            }
            let width = fixedSize(widthDimension, widthAxis: true)
            let height = fixedSize(heightDimension, widthAxis: false)
            if let width, let height {
                return AnyView(Color.clear.frame(width: width, height: height))
            }
            var sizedSpacer = AnyView(Color.clear)
            if widthDimension == .fill { sizedSpacer = AnyView(sizedSpacer.frame(maxWidth: .infinity)) }
            if heightDimension == .fill { sizedSpacer = AnyView(sizedSpacer.frame(maxHeight: .infinity)) }
            if let width {
                sizedSpacer = AnyView(sizedSpacer.frame(width: width))
            } else if let height {
                sizedSpacer = AnyView(sizedSpacer.frame(height: height))
            }
            func parentFraction(_ dimension: NexaDevDimension?) -> CGFloat? {
                guard let dimension, case .parentFraction(let fraction) = dimension else { return nil }
                return fraction
            }
            let parentWidth = parentFraction(widthDimension)
            let parentHeight = parentFraction(heightDimension)
            if parentWidth != nil || parentHeight != nil {
                sizedSpacer = AnyView(sizedSpacer.modifier(NexaParentRelativeDimensionModifier(
                    width: parentWidth,
                    height: parentHeight,
                    minWidth: nil,
                    maxWidth: nil,
                    minHeight: nil,
                    maxHeight: nil
                )))
            }
            return sizedSpacer
        case "Divider":
            let thickness = fields["thickness"] as? Double ?? 1
            guard thickness > 0 else { return AnyView(EmptyView()) }
            let color = devColor(fields["color"], isDark: colorScheme == .dark)
            return AnyView(NexaDividerPrimitive(color: color ?? .gray, thickness: CGFloat(thickness)))
        case NexaDevNodeKind.shape:
            let shapeFields = fields["shape"] as? [String: Any] ?? [:]
            let encodedShape = fields["shape"] as? String
            func dimension(_ key: String, fallback: Double) -> CGFloat {
                guard let expression = fields["\(key)_expression"], !(expression is NSNull) else {
                    return CGFloat(fields[key] as? Double ?? fallback)
                }
                let number = (store.evaluate(expression, locals: locals, scope: scope) as? NSNumber)?.doubleValue ?? fallback
                return CGFloat(number.isFinite ? max(0, number) : max(0, fallback))
            }
            let width = dimension("width", fallback: 0)
            let height = dimension("height", fallback: 0)
            let animation = nexaDevAnimation(fields["animation"])
            let fill = devColor(fields["fill"], isDark: colorScheme == .dark) ?? .clear
            func renderShape<S: Shape>(_ shape: S) -> AnyView {
                let framed = shape.fill(fill).frame(width: width, height: height)
                guard let animation else { return AnyView(framed) }
                return AnyView(framed.animation(animation, value: CGSize(width: width, height: height)))
            }
            if shapeFields["Circle"] != nil || encodedShape == "Circle" {
                return renderShape(Circle())
            }
            if let radius = shapeFields["Rounded"] as? Double {
                return renderShape(RoundedRectangle(cornerRadius: CGFloat(radius)))
            }
            return renderShape(Capsule())
        case "Content":
            guard let contentSlot else { return AnyView(EmptyView()) }
            return AnyView(NexaDevNodeList(
                nodes: contentSlot.nodes,
                module: module,
                store: store,
                parameters: contentSlot.parameters,
                stateScope: contentSlot.scope
            ))
        case "ComponentCall":
            let name = fields["name"] as? String ?? ""
            guard let component = (module["components"] as? [[String: Any]])?.first(where: {
                $0["name"] as? String == name
            }) else { return AnyView(EmptyView()) }
            var componentParameters: [String: Any] = [:]
            if let navId = locals["__nexa_navigation_stack_identity"] {
                componentParameters["__nexa_navigation_stack_identity"] = navId
            }
            if let entryId = locals["__nexa_navigation_entry_id"] {
                componentParameters["__nexa_navigation_entry_id"] = entryId
            }
            for argument in fields["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
                guard let parameter = argument[0] as? String else { continue }
                componentParameters[parameter] = store.evaluate(argument[1], locals: locals, scope: scope)
            }
            let slot = NexaDevContentSlot(
                nodes: fields["children"] as? [Any] ?? [],
                scope: scope,
                parameters: locals
            )
            let componentRef = fields[NexaDevKeys.componentRef].map {
                store.evaluate($0, locals: locals, scope: scope)
            } as? NexaDevComponentRef
            let componentIdentity = componentRef.map { "ref/\($0.identity)" } ?? identity
            let componentScope = "component/\(name)/\(componentIdentity)"
            componentRef?.attach(scope: componentScope, parameters: componentParameters)
            let content = NexaDevNodeList(
                nodes: component["body"] as? [Any] ?? [],
                module: module,
                store: store,
                parameters: componentParameters,
                stateDeclarations: component["states"] as? [[String: Any]] ?? [],
                stateScope: componentScope
            ).environment(\.nexaDevContentSlot, slot)
            return AnyView(content.onAppear {
                let bindings = store.bindings(component["states"] as? [[String: Any]] ?? [], parameters: componentParameters, scope: componentScope)
                if component["on_appear_async"] as? Bool != true {
                    store.perform(component["on_appear"] as? [Any] ?? [], scope: componentScope, locals: bindings)
                }
            }.task {
                if component["on_appear_async"] as? Bool == true {
                    let bindings = store.bindings(component["states"] as? [[String: Any]] ?? [], parameters: componentParameters, scope: componentScope)
                    do { _ = try await store.performAsync(component["on_appear"] as? [Any] ?? [], scope: componentScope, locals: bindings) }
                    catch is CancellationError { return }
                    catch { store.reportRuntimeFailure(error) }
                }
            }.onDisappear {
                store.perform(component["on_disappear"] as? [Any] ?? [], scope: componentScope, locals: componentParameters)
                store.clearNativeEventSubscriptions(scope: componentScope)
                store.clearNativeTasks(scope: componentScope)
                componentRef?.detach()
            })
        case "NativeComponentCall":
            let namespace = fields["namespace"] as? String ?? ""
            let name = fields["name"] as? String ?? ""
            var arguments: [String: Any] = [:]
            for argument in fields["arguments"] as? [[Any]] ?? [] where argument.count >= 2 {
                guard let argumentName = argument[0] as? String else { continue }
                arguments[argumentName] = store.evaluate(argument[1], locals: locals, scope: scope)
            }
            var events: [String: ([Any]) -> Void] = [:]
            for handler in fields["event_handlers"] as? [[String: Any]] ?? [] {
                guard let property = handler["property"] as? String else { continue }
                events[property] = store.devNativeEventHandler(
                    handler["actions"] as? [Any] ?? [],
                    parameters: handler["parameters"] as? [String] ?? [],
                    scope: scope,
                    locals: locals
                )
            }
            let children = fields["children"] as? [Any] ?? []
            let componentRef = fields[NexaDevKeys.componentRef].flatMap { value -> Any? in
                guard !(value is NSNull) else { return nil }
                return store.evaluate(value, locals: locals, scope: scope)
            }
            let content = AnyView(NexaDevNodeList(
                nodes: children,
                module: module,
                store: store,
                parameters: locals,
                stateScope: scope
            ))
            if let component = NexaDevPluginBridge.renderComponent(
                namespace: namespace,
                name: name,
                arguments: arguments,
                events: events,
                componentRef: componentRef,
                content: content
            ) {
                return component
            }
            NSLog("NexaDevRuntime: native component is unsupported: %@.%@", namespace, name)
            return AnyView(EmptyView())
        case "Button":
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let actions = fields["actions"] as? [Any] ?? []
            let disabled = fields["disabled"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let loading = fields["loading"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let accessibilityLabel: String
            if let expression = fields["accessibility_label"], !(expression is NSNull) {
                accessibilityLabel = store.stringify(store.evaluate(expression, locals: locals, scope: scope))
            } else {
                accessibilityLabel = label
            }
            let style: NexaSharedButtonStyleKind = switch fields[NexaDevKeys.style] as? String {
            case "Bordered": .bordered
            case "Borderless": .borderless
            case "Plain": .plain
            default: .borderedProminent
            }
            let size: NexaSharedButtonSizeKind? = switch fields["size"] as? String {
            case "Small": .small
            case "Regular": .regular
            case "Large": .large
            default: nil
            }
            let shapeFields = fields["shape"] as? [String: Any] ?? [:]
            let encodedShape = fields["shape"] as? String
            let shape: NexaSharedButtonShapeKind
            if shapeFields["Circle"] != nil || encodedShape == "Circle" { shape = .circle }
            else if let radius = shapeFields["Rounded"] as? Double { shape = .rounded(CGFloat(radius)) }
            else if shapeFields["Capsule"] != nil || encodedShape == "Capsule" { shape = .capsule }
            else { shape = .capsule }
            let icon = nexaDevIconSpec(fields["icon"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
            let iconOnlyCircle: Bool
            if icon != nil, label.isEmpty, case .circle = shape { iconOnlyCircle = true }
            else { iconOnlyCircle = false }
            let tint: Color? = {
                guard let tintValue = fields["tint"] as? [String: Any] else { return nil }
                if let dynamic = tintValue["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(tintValue, isDark: colorScheme == .dark)
            }()
            return AnyView(NexaButtonPrimitive(
                style: style,
                size: size,
                shape: shape,
                tint: tint,
                loading: loading,
                disabled: disabled,
                accessibilityLabel: accessibilityLabel,
                glass: fields["glass"] as? Bool == true,
                iconOnlyCircle: iconOnlyCircle,
                action: { store.perform(actions, scope: scope, locals: locals) },
                label: {
                    if let icon {
                        if nexaDevUsesIconOnlyButtonLabel(fields["label"], comment: fields["comment"]) {
                            NexaIconView(icon: icon, description: "", size: 17)
                        } else {
                            Label {
                                Text(label)
                            } icon: {
                                NexaIconView(icon: icon, description: "", size: 17)
                            }
                        }
                    } else {
                        Text(label)
                    }
                }
            ))
        case "Pressable":
            let children = fields["children"] as? [Any] ?? []
            let contextMenu = fields["context_menu"] as? [Any] ?? []
            let actions = fields["actions"] as? [Any] ?? []
            let doubleTapActions = fields["double_tap_actions"] as? [Any] ?? []
            let longPressActions = fields["long_press_actions"] as? [Any] ?? []
            let dragParameters = fields[NexaDevKeys.dragParameters] as? [String] ?? []
            let dragActions = fields[NexaDevKeys.dragActions] as? [Any] ?? []
            let pinchParameter = fields[NexaDevKeys.pinchParameter] as? String
            let pinchActions = fields[NexaDevKeys.pinchActions] as? [Any] ?? []
            let disabled = fields["disabled"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let fillMaxSize = fields["fill_max_size"] as? Bool == true
            let haptic = fields["haptic"] as? String
            let content = NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            let hitTarget: AnyView = fillMaxSize
                ? AnyView(content.frame(maxWidth: .infinity, maxHeight: .infinity).contentShape(.interaction, Rectangle()))
                : AnyView(content.frame(minHeight: 48, alignment: .topLeading).contentShape(.interaction, Rectangle()))
            let hasDoubleTap = !doubleTapActions.isEmpty
            let hasLongPress = !longPressActions.isEmpty
            let longPressMinimumDuration: Double
            if hasLongPress {
                let durationMs = fields[NexaDevKeys.longPressDurationMs].flatMap {
                    (store.evaluate($0, locals: locals, scope: scope) as? NSNumber)?.intValue
                } ?? 500
                longPressMinimumDuration = Double(max(1, durationMs)) / 1000.0
            } else {
                longPressMinimumDuration = 0
            }
            let performTap: () -> Void = {
                guard !disabled else { return }
                playNexaHaptic(haptic)
                store.perform(actions, scope: scope, locals: locals)
            }
            let performDoubleTap: () -> Void = {
                guard !disabled else { return }
                playNexaHaptic(haptic)
                store.perform(doubleTapActions, scope: scope, locals: locals)
            }
            let performLongPress: () -> Void = {
                guard !disabled else { return }
                playNexaHaptic(haptic)
                store.perform(longPressActions, scope: scope, locals: locals)
            }
            let gesturePressable: AnyView
            if hasLongPress && hasDoubleTap {
                let gesture = LongPressGesture(minimumDuration: longPressMinimumDuration)
                    .onEnded { _ in performLongPress() }
                    .exclusively(before:
                        TapGesture(count: 2).onEnded { performDoubleTap() }
                            .exclusively(before: TapGesture(count: 1).onEnded { performTap() })
                    )
                gesturePressable = AnyView(hitTarget.highPriorityGesture(gesture))
            } else if hasLongPress {
                let gesture = LongPressGesture(minimumDuration: longPressMinimumDuration)
                    .onEnded { _ in performLongPress() }
                    .exclusively(before: TapGesture(count: 1).onEnded { performTap() })
                gesturePressable = AnyView(hitTarget.highPriorityGesture(gesture))
            } else if hasDoubleTap {
                let gesture = TapGesture(count: 2)
                    .onEnded { performDoubleTap() }
                    .exclusively(before: TapGesture(count: 1).onEnded { performTap() })
                gesturePressable = AnyView(hitTarget.highPriorityGesture(gesture))
            } else {
                gesturePressable = AnyView(hitTarget.overlay {
                    GeometryReader { geometry in
                        Color.clear
                            .contentShape(.interaction, Rectangle())
                            .gesture(SpatialTapGesture(coordinateSpace: .local).onEnded { value in
                                guard CGRect(origin: .zero, size: geometry.size).contains(value.location) else { return }
                                performTap()
                            })
                    }
                })
            }
            let pressable = AnyView(gesturePressable
                .accessibilityAction(.default) { performTap() }
                .accessibilityAddTraits(.isButton)
                .disabled(disabled)
                .opacity(disabled ? 0.5 : 1.0)
                .contentShape(.interaction, Rectangle()))
            let pressableWithMenu: AnyView
            if contextMenu.isEmpty {
                pressableWithMenu = pressable
            } else {
                pressableWithMenu = AnyView(pressable.contextMenu {
                    NexaDevNodeList(nodes: contextMenu, module: module, store: store, parameters: locals, stateScope: scope)
                })
            }
            guard dragParameters.count == 4 || pinchParameter != nil else { return pressableWithMenu }
            var dragLocals = locals
            var pinchLocals = locals
            return AnyView(NexaDevDragGestureView(
                enabled: !disabled,
                content: pressableWithMenu,
                onDrag: dragParameters.count == 4 ? { translationX, translationY, velocityX, velocityY in
                    dragLocals[dragParameters[0]] = translationX
                    dragLocals[dragParameters[1]] = translationY
                    dragLocals[dragParameters[2]] = velocityX
                    dragLocals[dragParameters[3]] = velocityY
                    store.perform(dragActions, scope: scope, locals: dragLocals)
                } : nil,
                onPinch: pinchParameter.map { parameter in
                    { scaleFactor in
                        pinchLocals[parameter] = scaleFactor
                        store.perform(pinchActions, scope: scope, locals: pinchLocals)
                    }
                }
            ))
        case "TextInput":
            let name = fields["state"] as? String ?? ""
            let sourcePlaceholder = fields["placeholder"] as? String ?? ""
            let placeholder = store.localizedText(key: sourcePlaceholder, fallback: sourcePlaceholder)
            let maxLength = fields["max_length"] as? Int
            let onChange = fields["on_change"] as? [String: Any]
            let onFocus = fields["on_focus"] as? [Any]
            let initialFocus = store.evaluate(fields["focused"], locals: locals, scope: scope) as? Bool ?? false
            let componentRef = fields["component_ref"].flatMap { value -> NexaTextInputRef? in
                guard !(value is NSNull) else { return nil }
                return store.evaluate(value, locals: locals, scope: scope) as? NexaTextInputRef
            }
            let submitActions = fields["actions"] as? [Any] ?? []
            let keyboard: UIKeyboardType = switch fields["keyboard"] as? String {
            case "Number": .numberPad
            case "Email": .emailAddress
            case "Phone": .phonePad
            case "Url": .URL
            default: .default
            }
            let capitalization: UITextAutocapitalizationType = switch fields["capitalization"] as? String {
            case "None": .none
            case "Words": .words
            case "Characters": .allCharacters
            default: .sentences
            }
            let autofill: UITextContentType? = switch fields["autofill"] as? String {
            case "Username": .username
            case "Password": .password
            case "OneTimeCode": .oneTimeCode
            default: nil
            }
            let returnKey: UIReturnKeyType = switch fields["return_key"] as? String {
            case "Done": .done
            case "Search": .search
            case "Send": .send
            case "Next": .next
            default: .default
            }
            return AnyView(NexaTextInputPrimitive(
                text: Binding(
                    get: { store.stringify(store.value(name, scope: scope)) },
                    set: { store.setValue(name, value: $0, scope: scope) }
                ),
                placeholder: placeholder,
                secure: fields["secure"] as? Bool == true,
                multiline: fields["multiline"] as? Bool == true,
                searchable: fields["searchable"] as? Bool == true,
                maxLength: maxLength,
                fontSize: fields["font_size"] as? CGFloat,
                fontFamily: fields["font_family"] as? String,
                fontRelativeTo: fields["font"] as? String == "Title3" ? .title3 : .body,
                fontDefaultSize: fields["font"] as? String == "Title3" ? 20 : 17,
                textColor: devColor(fields["text_color"], isDark: colorScheme == .dark),
                placeholderColor: devColor(fields["placeholder_color"], isDark: colorScheme == .dark),
                clearable: fields["clearable"] as? Bool == true,
                minLines: fields["min_lines"] as? Int,
                maxLines: fields["max_lines"] as? Int,
                keyboard: keyboard,
                returnKey: returnKey,
                autofill: autofill,
                autocorrect: fields["autocorrect"] as? Bool,
                capitalization: capitalization,
                initialFocus: initialFocus,
                componentRef: componentRef,
                onFocusAction: {
                    if let onFocus { store.perform(onFocus, scope: scope, locals: locals) }
                },
                onSubmitAction: submitActions.isEmpty ? nil : { store.perform(submitActions, scope: scope, locals: locals) },
                onChange: { value in
                    if let onChange, let parameter = onChange["parameter"] as? String {
                        var changeLocals = locals
                        changeLocals[parameter] = value
                        store.perform(onChange["actions"] as? [Any] ?? [], scope: scope, locals: changeLocals)
                    }
                }
            ))
        case "FastList":
            guard let plan = fields["plan"] as? [String: Any],
                  let countPlan = plan["Count"] as? [String: Any]
                    ?? plan["Items"] as? [String: Any]
                    ?? plan["Sections"] as? [String: Any],
                  let options = countPlan["common"] as? [String: Any] else {
                return AnyView(Text("FastList source could not be evaluated."))
            }
            let countSource = plan["Count"] as? [String: Any]
            let itemSource = plan["Items"] as? [String: Any]
            let sectionSource = plan["Sections"] as? [String: Any]
            let countExpression = countSource?["count"]
            let itemExpression = itemSource?["collection"]
            let sourceItems: [Any]?
            if let itemExpression {
                sourceItems = store.evaluate(itemExpression, locals: locals, scope: scope) as? [Any] ?? []
            } else {
                sourceItems = nil
            }
            let sectionExpression = sectionSource?["collection"]
            let sourceSections: [[Any]]?
            if let sectionExpression {
                sourceSections = store.evaluate(sectionExpression, locals: locals, scope: scope) as? [[Any]] ?? []
            } else {
                sourceSections = nil
            }
            guard countExpression != nil || sourceItems != nil || sourceSections != nil else {
                return AnyView(Text("FastList source could not be evaluated."))
            }
            let itemCount = countExpression.map {
                max(0, (store.evaluate($0, locals: locals, scope: scope) as? NSNumber)?.intValue ?? 0)
            } ?? (sourceItems?.count ?? 0)
            let sectionMetadata = sourceSections.map { sections in
                var counts: [Int] = []
                var offsets: [Int] = []
                counts.reserveCapacity(sections.count)
                offsets.reserveCapacity(sections.count)
                var total = 0
                for section in sections {
                    offsets.append(total)
                    counts.append(section.count)
                    total += section.count
                }
                return (counts: counts, offsets: offsets, total: total)
            }
            let count = sectionMetadata?.total ?? itemCount
            let axis: NexaDevFastListAxis
            if options["axis"] as? String == "Horizontal" {
                axis = .horizontal
            } else if let grid = (options["axis"] as? [String: Any])?["Grid"] as? [String: Any],
                      let columns = grid["columns"] as? Int {
                axis = .grid(max(1, columns))
            } else if let grid = (options["axis"] as? [String: Any])?["AutoGrid"] as? [String: Any],
                      let minCellWidth = grid["min_cell_width"] as? Int {
                axis = .adaptiveGrid(max(1, minCellWidth))
            } else if let masonry = (options["axis"] as? [String: Any])?["Masonry"] as? [String: Any],
                      let columns = masonry["columns"] as? Int {
                axis = .masonry(max(1, columns))
            } else {
                axis = .vertical
            }
            let reverseLayout = options[NexaDevKeys.reverseLayout] as? Bool ?? false
            let pageSnap = options[NexaDevKeys.pageSnap] as? Bool ?? false
            let scrollEnabledExpression = options["scroll_enabled"].flatMap {
                $0 is NSNull ? nil : $0
            }
            let scrollEnabled = scrollEnabledExpression.map {
                store.truthy(store.evaluate($0, locals: locals, scope: scope))
            } ?? true
            let showDividers = options["show_dividers"] as? Bool ?? true
            let hideHorizontalScrollbar = options[NexaDevKeys.hideHorizontalScrollbar] as? Bool ?? false
            let hideVerticalScrollbar = options[NexaDevKeys.hideVerticalScrollbar] as? Bool ?? false
            let indexName = options["index"] as? String ?? "index"
            let itemName = countPlan["item"] as? String
            let sectionName = countPlan["section"] as? String ?? "section"
            let children = options["children"] as? [Any] ?? []
            let itemExtent = options["item_extent"] as? Double
            let spacing = options["gap"] as? Double ?? 0
            let scrollPositionName = options["scroll_position"] as? String
            let requestedIndex = scrollPositionName.flatMap { name in
                (store.value(name, scope: scope) as? NSNumber)?.intValue
            }
            let scrollPositionCommandRevision = scrollPositionName.map {
                store.stateCommandRevision($0, scope: scope)
            } ?? 0
            let stickyHeaderNodes = options["sticky_header"] as? [Any]
            let swipeConfig = options["swipe_actions"] as? [String: Any]
            let swipeActionNodes = swipeConfig?["children"] as? [Any]
            let swipeEdge: HorizontalEdge = (swipeConfig?["edge"] as? String) == "Leading" ? .leading : .trailing
            let swipeAllowsFullSwipe = swipeConfig?["allows_full_swipe"] as? Bool ?? false
            let stickyHeader: AnyView? = stickyHeaderNodes.map { nodes in
                AnyView(NexaDevNodeList(nodes: nodes, module: module, store: store, parameters: locals, stateScope: scope)
                    .environment(\.nexaDevNodeIdentity, "\(identity)/sticky-header"))
            }
            let sectionHeaderNodes = options["section_header"] as? [Any]
            let sectionHeader: ((Int) -> AnyView)? = sectionHeaderNodes.map { nodes in
                { sectionIndex in
                    var headerLocals = locals
                    headerLocals[sectionName] = sectionIndex
                    if let sourceSections, sourceSections.indices.contains(sectionIndex) {
                        headerLocals["sectionItems"] = sourceSections[sectionIndex]
                    }
                    return AnyView(NexaDevNodeList(
                        nodes: nodes,
                        module: module,
                        store: store,
                        parameters: headerLocals,
                        stateScope: scope
                    )
                    .environment(\.nexaDevNodeIdentity, "\(identity)/section-header/\(sectionIndex)"))
                }
            }
            let onScrollActions = options["on_scroll"] as? [Any]
            let onScrollBeginDragActions = options["on_scroll_begin_drag"] as? [Any]
            let onEndReachedActions = options["on_end_reached"] as? [Any]
            let componentRef = options["component_ref"].flatMap { value -> NexaFastListRef? in
                guard !(value is NSNull) else { return nil }
                return store.evaluate(value, locals: locals, scope: scope) as? NexaFastListRef
            }
            let onMove = options["on_move"] as? [String: Any]
            let isMasonry: Bool = {
                if case .masonry = axis { return true }
                return false
            }()
            let usesPlatformList = (onMove != nil && !isMasonry) || swipeActionNodes != nil
            let moveEnabled = onMove?["enabled"].map {
                store.truthy(store.evaluate($0, locals: locals, scope: scope))
            } ?? false
            let refresh = options["refresh"] as? [String: Any]
            let refreshState = refresh?["state"] as? String
            let refreshActions = refresh?["actions"] as? [Any] ?? []
            let makeRowLocals: (Int, Int, Int) -> [String: Any] = { index, sectionIndex, itemIndex in
                var rowLocals = locals
                rowLocals[indexName] = itemIndex
                if let sourceSections, let itemName,
                   sourceSections.indices.contains(sectionIndex),
                   sourceSections[sectionIndex].indices.contains(itemIndex) {
                    rowLocals[itemName] = sourceSections[sectionIndex][itemIndex]
                    rowLocals[sectionName] = sectionIndex
                } else if let itemName, let sourceItems, sourceItems.indices.contains(index) {
                    rowLocals[itemName] = sourceItems[index]
                }
                return rowLocals
            }
            let keyExpression = options["key"].flatMap { $0 is NSNull ? nil : $0 }
            let rowKey: ((Int, Int, Int) -> AnyHashable)? = keyExpression.map { expression in
                { index, sectionIndex, itemIndex in
                    let value = store.evaluate(
                        expression,
                        locals: makeRowLocals(index, sectionIndex, itemIndex),
                        scope: scope
                    )
                    return (value as? AnyHashable) ?? AnyHashable(store.stringify(value))
                }
            }
            let taskOwner = onEndReachedActions.map { _ in "\(scope)/fast-list/\(identity)" }
            return AnyView(NexaDevFastList(
                count: count,
                usesPlatformList: usesPlatformList,
                showDividers: showDividers,
                hideHorizontalScrollbar: hideHorizontalScrollbar,
                hideVerticalScrollbar: hideVerticalScrollbar,
                axis: axis,
                reverseLayout: reverseLayout,
                pageSnap: pageSnap,
                scrollEnabled: scrollEnabled,
                rowHeight: itemExtent,
                spacing: spacing,
                scrollPosition: requestedIndex,
                sectionCounts: sectionMetadata?.counts,
                sectionOffsets: sectionMetadata?.offsets,
                componentRef: componentRef,
                rowKey: rowKey,
                stickyHeader: stickyHeader,
                sectionHeader: sectionHeader,
                swipeActions: swipeActionNodes.map { nodes in
                    { index, sectionIndex, itemIndex in
                        let rowLocals = makeRowLocals(index, sectionIndex, itemIndex)
                        return NexaDevFastListSwipeActionContent(
                            nodes: nodes,
                            store: store,
                            locals: rowLocals,
                            scope: scope,
                            isDark: colorScheme == .dark
                        ).environment(\.nexaDevNodeIdentity, "\(identity)/swipe-actions/\(index)")
                    }
                },
                swipeEdge: swipeEdge,
                swipeAllowsFullSwipe: swipeAllowsFullSwipe,
                row: { index, sectionIndex, itemIndex in
                    let rowLocals = makeRowLocals(index, sectionIndex, itemIndex)
                    return AnyView(NexaDevNodeList(
                        nodes: children,
                        module: module,
                        store: store,
                            parameters: rowLocals,
                        stateScope: scope
                    ).environment(\.nexaDevNodeIdentity, "\(identity)/row/\(index)"))
                },
                onMove: onMove.map { callback in
                    { source, destination in
                        guard moveEnabled else { return }
                        guard let from = source.first else { return }
                        let to = destination > from ? destination - 1 : destination
                        guard from != to else { return }
                        let callbackLocals = locals.merging([
                            callback["from"] as? String ?? "from": NSNumber(value: from),
                            callback["to"] as? String ?? "to": NSNumber(value: to),
                        ]) { _, newest in newest }
                        store.perform(callback["actions"] as? [Any] ?? [], scope: scope, locals: callbackLocals)
                    }
                },
                moveEnabled: moveEnabled,
                onScrollPositionChanged: scrollPositionName.map { name in
                    { index in
                        store.setScrollPositionFromNative(
                            name,
                            value: index,
                            scope: scope,
                            expectedCommandRevision: scrollPositionCommandRevision
                        )
                    }
                },
                onScroll: onScrollActions.map { actions in
                    {
                        guard scrollPositionName.map({
                            store.stateCommandRevision($0, scope: scope) == scrollPositionCommandRevision
                        }) ?? true else { return }
                        store.perform(actions, scope: scope, locals: locals)
                    }
                },
                onScrollBeginDrag: onScrollBeginDragActions.map { actions in
                    { store.perform(actions, scope: scope, locals: locals) }
                },
                onEndReached: onEndReachedActions.map { actions in
                    {
                        var callbackLocals = locals
                        if let taskOwner { callbackLocals[nexaDevTaskOwnerLocal] = taskOwner }
                        store.perform(actions, scope: scope, locals: callbackLocals)
                    }
                },
                isRefreshing: refreshState.map { store.truthy(store.value($0, scope: scope)) } ?? false,
                onRefresh: refresh.map { _ in
                    { store.perform(refreshActions, scope: scope, locals: locals) }
                }
            ).onDisappear {
                if let taskOwner { store.clearNativeTasks(owner: taskOwner) }
            })
        case "Switch":
            let name = fields["state"] as? String ?? ""
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let change = fields["on_change"] as? [String: Any]
            let onChange: ((Bool) -> Void)?
            if let parameter = change?["parameter"] as? String,
               let actions = change?["actions"] as? [Any], !actions.isEmpty {
                onChange = { nextValue in
                    var changeLocals = locals
                    changeLocals[parameter] = nextValue
                    store.perform(actions, scope: scope, locals: changeLocals)
                }
            } else {
                onChange = nil
            }
            return AnyView(NexaSwitchPrimitive(label: Text(label), isOn: Binding(
                get: { store.truthy(store.value(name, scope: scope)) },
                set: { store.setValue(name, value: $0, scope: scope) }
            ), onChange: onChange))
        case "Slider":
            let name = fields["state"] as? String ?? ""
            let min = (store.evaluate(fields["min"] ?? 0, locals: locals, scope: scope) as? NSNumber)?.doubleValue ?? 0
            let max = (store.evaluate(fields["max"] ?? 1, locals: locals, scope: scope) as? NSNumber)?.doubleValue ?? 1
            let step = fields["step"] as? Double ?? 0.0
            let showStepIndicators = fields["show_step_indicators"] as? Bool ?? true
            func color(_ key: String) -> Color? {
                guard let value = fields[key] as? [String: Any] else { return nil }
                if let dynamic = value["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(value, isDark: colorScheme == .dark)
            }
            let onChange = fields["on_change"] as? [String: Any]
            let onEditingChanged = fields["on_editing_changed"] as? [String: Any]
            return AnyView(NexaSliderPrimitive(value: Binding(
                get: { (store.value(name, scope: scope) as? NSNumber)?.doubleValue ?? min },
                set: { store.setValue(name, value: $0, scope: scope) }
            ), minValue: min, maxValue: max, step: step,
                showStepIndicators: showStepIndicators,
                tint: color("tint"), trackColor: color("track_color"), thumbColor: color("thumb_color"),
                trackHeight: fields["track_height"] as? CGFloat,
                thumbSize: fields["thumb_size"] as? CGFloat,
                hitSlop: fields["hit_slop"] as? CGFloat,
                onChange: { value in
                    if let onChange, let parameter = onChange["parameter"] as? String {
                        var changeLocals = locals
                        changeLocals[parameter] = value
                        store.perform(onChange["actions"] as? [Any] ?? [], scope: scope, locals: changeLocals)
                    }
                },
                onEditingChanged: { editing in
                    if let onEditingChanged, let parameter = onEditingChanged["parameter"] as? String {
                        var changeLocals = locals
                        changeLocals[parameter] = editing
                        store.perform(onEditingChanged["actions"] as? [Any] ?? [], scope: scope, locals: changeLocals)
                    }
                }))
        case "ProgressBar":
            let progress = (store.evaluate(fields["progress"] ?? NSNull(), locals: locals, scope: scope) as? NSNumber)?.doubleValue
            let style = fields[NexaDevKeys.style] as? [String: Any] ?? [:]
            func color(_ key: String) -> Color? {
                guard let value = style[key] as? [String: Any] else { return nil }
                if let dynamic = value["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(value, isDark: colorScheme == .dark)
            }
            return AnyView(NexaProgressBarPrimitive(
                progress: progress,
                tint: color("tint"),
                trackColor: color("track_color"),
                thickness: style["thickness"] as? CGFloat,
                cornerRadius: style["corner_radius"] as? CGFloat
            ))
        case "ProgressRing":
            let progress = (store.evaluate(fields["progress"] ?? NSNull(), locals: locals, scope: scope) as? NSNumber)?.doubleValue
            let style = fields[NexaDevKeys.style] as? [String: Any] ?? [:]
            func color(_ key: String) -> Color? {
                guard let value = style[key] as? [String: Any] else { return nil }
                if let dynamic = value["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(value, isDark: colorScheme == .dark)
            }
            return AnyView(NexaProgressRingPrimitive(
                progress: progress,
                tint: color("tint"),
                trackColor: color("track_color"),
                size: style["size"] as? CGFloat,
                thickness: style["thickness"] as? CGFloat
            ))
        case "SegmentedControl":
            let name = fields["state"] as? String ?? ""
            let items = store.evaluate(fields["items"] ?? NSNull(), locals: locals, scope: scope) as? [String] ?? []
            func color(_ key: String) -> Color? {
                guard let value = fields[key] as? [String: Any] else { return nil }
                if let dynamic = value["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(value, isDark: colorScheme == .dark)
            }
            let fontSize = fields["font_size"] as? CGFloat
            let fontWeight: Font.Weight?
            switch fields["font_weight"] as? String {
            case "Normal": fontWeight = .regular
            case "Medium": fontWeight = .medium
            case "Semibold": fontWeight = .semibold
            case "Bold": fontWeight = .bold
            default: fontWeight = nil
            }
            return AnyView(NexaSegmentedControlPrimitive(items: items, selection: Binding(
                get: { store.value(name, scope: scope) as? String ?? "" },
                set: { store.setValue(name, value: $0, scope: scope) }
            ), animation: nexaDevAnimation(fields["animation"]), tint: color("tint"), background: color("background"), textColor: color("text_color"), selectedTextColor: color("selected_text_color"), fontSize: fontSize, fontWeight: fontWeight, height: fields["height"] as? CGFloat, cornerRadius: fields["corner_radius"] as? CGFloat, borderColor: color("border_color"), borderWidth: fields["border_width"] as? CGFloat))
        case "Picker":
            let name = fields["state"] as? String ?? ""
            let items = store.evaluate(fields["items"] ?? NSNull(), locals: locals, scope: scope) as? [String] ?? []
            let evaluatedItemIcons = fields["item_icons"].flatMap { value -> [Any]? in
                if value is NSNull { return nil }
                return store.evaluate(value, locals: locals, scope: scope) as? [Any]
            } ?? []
            let itemIcons = evaluatedItemIcons.compactMap { nexaDevIconSpec($0, store: store, locals: locals, scope: scope, isDark: colorScheme == .dark) }
            let icon = nexaDevIconSpec(fields["icon"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
            let label = fields["label"].flatMap { value in
                value is NSNull ? nil : store.stringify(store.evaluate(value, locals: locals, scope: scope))
            }
            let tint: Color? = {
                guard let tintValue = fields["tint"] as? [String: Any] else { return nil }
                if let dynamic = tintValue["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(tintValue, isDark: colorScheme == .dark)
            }()
            return AnyView(NexaPickerPrimitive(selection: Binding(
                get: { store.value(name, scope: scope) as? String ?? "" },
                set: { store.setValue(name, value: $0, scope: scope) }
            ), items: items, itemIcons: itemIcons, icon: icon, tint: tint, hasLabel: label != nil) {
                if let label {
                    Text(label)
                } else {
                    EmptyView()
                }
            })
        case "PickerDate":
            let pickerFields = fields["props"] as? [String: Any] ?? fields
            func evaluated(_ key: String) -> Any? {
                guard let value = pickerFields[key], !(value is NSNull) else { return nil }
                return store.evaluate(value, locals: locals, scope: scope)
            }
            func integer(_ key: String) -> Int64? {
                guard let value = evaluated(key) else { return nil }
                if let number = value as? NSNumber { return number.int64Value }
                return value as? Int64
            }
            func string(_ key: String) -> String? {
                evaluated(key).map { store.stringify($0) }
            }
            func boolean(_ key: String) -> Bool? {
                evaluated(key).map { store.truthy($0) }
            }
            func color(_ key: String) -> Color? {
                guard let value = pickerFields[key] as? [String: Any] else { return nil }
                if let dynamic = value["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(value, isDark: colorScheme == .dark)
            }
            let timestampName = pickerFields["timestamp_state"] as? String
            let startName = pickerFields["start_timestamp_state"] as? String
            let endName = pickerFields["end_timestamp_state"] as? String
            let timestampBinding: Binding<Int64>? = timestampName.map { name in
                Binding(
                    get: {
                        let value = store.value(name, scope: scope)
                        return (value as? NSNumber)?.int64Value ?? (value as? Int64) ?? 0
                    },
                    set: { store.setValue(name, value: $0, scope: scope) }
                )
            }
            let startBinding: Binding<Int64>? = startName.map { name in
                Binding(
                    get: {
                        let value = store.value(name, scope: scope)
                        return (value as? NSNumber)?.int64Value ?? (value as? Int64) ?? 0
                    },
                    set: { store.setValue(name, value: $0, scope: scope) }
                )
            }
            let endBinding: Binding<Int64>? = endName.map { name in
                Binding(
                    get: {
                        let value = store.value(name, scope: scope)
                        return (value as? NSNumber)?.int64Value ?? (value as? Int64) ?? 0
                    },
                    set: { store.setValue(name, value: $0, scope: scope) }
                )
            }
            return AnyView(NexaDatePickerPrimitive(
                timestamp: timestampBinding,
                startTimestamp: startBinding,
                endTimestamp: endBinding,
                pickerType: pickerFields["picker_type"] as? String ?? "Date",
                style: pickerFields["style"] as? String ?? "Automatic",
                selection: pickerFields["selection"] as? String ?? "Single",
                minimum: integer("minimum"),
                maximum: integer("maximum"),
                tint: color("tint"),
                textColor: color("text_color"),
                backgroundColor: color("background_color"),
                pickerBackgroundColor: color("picker_background_color"),
                borderColor: color("border_color"),
                borderWidth: pickerFields["border_width"] as? CGFloat,
                cornerRadius: pickerFields["corner_radius"] as? CGFloat,
                localeIdentifier: string("locale"),
                timeZoneIdentifier: string("time_zone"),
                is24Hour: boolean("is_24_hour"),
                enabled: boolean("enabled"),
                accessibilityLabel: string("accessibility_label")
            ))
        case "Icon":
            let icon = nexaDevIconSpec(fields["icon"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
                ?? NexaIconSpec(symbol: "questionmark")
            return AnyView(NexaIconView(icon: icon, description: nil, size: nil))
        case "SystemIcon":
            let icon = nexaDevIconSpec(fields["icon"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
                ?? NexaIconSpec(symbol: "questionmark")
            let description = fields["description"] as? String
            let size = fields["size"] as? CGFloat
            let tintValue = fields["tint"] as? [String: Any] ?? [:]
            let tint: Color? = if let dynamic = tintValue["Dynamic"] {
                devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
            } else {
                devColor(fields["tint"], isDark: colorScheme == .dark)
            }
            return AnyView(NexaIconView(icon: icon, description: description, size: size, tint: tint))
        case "FontIcon":
            let glyphValue = store.evaluate(fields["glyph"] ?? 0, locals: locals, scope: scope)
            let glyph = (glyphValue as? NSNumber)?.int32Value ?? -1
            let family = fields["font_family"] as? String ?? ""
            let description = fields["description"] as? String
            let size: CGFloat? = {
                guard let rawSize = fields["size"], !(rawSize is NSNull) else { return nil }
                let value = store.evaluate(rawSize, locals: locals, scope: scope)
                guard let number = value as? NSNumber else { return nil }
                return CGFloat(min(512.0, max(1.0, number.doubleValue)))
            }()
            let tintValue = fields["tint"] as? [String: Any] ?? [:]
            let tint: Color? = if let dynamic = tintValue["Dynamic"] {
                devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
            } else {
                devColor(fields["tint"], isDark: colorScheme == .dark)
            }
            return AnyView(NexaFontIconPrimitive(
                glyph: glyph,
                fontFamily: family,
                description: description,
                size: size,
                tint: tint,
                layers: nexaDevIconLayers(fields["layers"]),
                layerColors: nexaDevIconLayerColors(fields["layer_colors"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
            ))
        case "LinearGradient":
            let start = devColor(fields["start_color"], isDark: colorScheme == .dark) ?? .clear
            let end = devColor(fields["end_color"], isDark: colorScheme == .dark) ?? .clear
            let direction = fields["direction"] as? String ?? "TopToBottom"
            let points: (UnitPoint, UnitPoint) = switch direction {
            case "BottomToTop": (.bottom, .top)
            case "LeadingToTrailing": (.leading, .trailing)
            case "TrailingToLeading": (.trailing, .leading)
            default: (.top, .bottom)
            }
            let height = fields["height"] as? Double ?? 180
            return AnyView(NexaLinearGradientPrimitive(colors: [start, end], startPoint: points.0, endPoint: points.1, height: height))
        case "Image":
            let source = fields["source"] as? [String: Any] ?? [:]
            let description = fields["description"] as? String ?? ""
            let fit: NexaImageScale = (fields["fit"] as? String ?? fields["scale"] as? String) == "Fill" ? .fill : .fit
            let placeholder = fields["placeholder"] as? String
            let showLoadingIndicator = fields["show_loading_indicator"] as? Bool ?? false
            let autoReplay = fields["auto_replay"] as? Bool ?? true
            let style = fields[NexaDevKeys.style] as? [String: Any] ?? [:]
            let sharedExpression = fields["shared_element"] as? [String: Any]
            let fitHeight: CGFloat? = {
                let h = nexaDevDimension(style["height"]) ?? nexaDevDimension(style["max_height"])
                let w = nexaDevDimension(style["width"])
                let hv: CGFloat? = if case .fixed(let v) = h { v } else { nil }
                let wv: CGFloat? = if case .fixed(let v) = w { v } else { nil }
                return switch (wv, hv) {
                case (let a?, let b?): max(a, b)
                case (nil, let b?): b
                case (let a?, nil): a
                case (nil, nil): nil
                }
            }()
            var rendered: AnyView
            if let asset = source["Asset"] as? String {
                rendered = AnyView(NexaAssetImagePrimitive(
                    name: asset,
                    description: description,
                    scale: fit,
                    autoReplay: autoReplay,
                    fitHeight: fitHeight
                ))
            } else if let expression = source["RemoteUrl"] ?? source["LocalFile"] {
                let allowsFile = source["LocalFile"] != nil
                let urlText = store.stringify(store.evaluate(expression, locals: locals, scope: scope))
                rendered = AnyView(NexaRemoteImagePrimitive(
                    url: urlText,
                    scale: fit,
                    placeholder: allowsFile ? nil : placeholder,
                    showLoadingIndicator: showLoadingIndicator,
                    autoReplay: autoReplay,
                    allowsFile: allowsFile,
                    description: description,
                    fitHeight: fitHeight,
                    usesSharedElement: sharedExpression != nil
                ))
            } else {
                rendered = AnyView(NexaImagePlaceholderPrimitive(
                    name: placeholder ?? "",
                    scale: fit,
                    autoReplay: autoReplay
                ))
            }
            let hasHeightBound = nexaDevDimension(style["height"]) != nil || nexaDevDimension(style["max_height"]) != nil
            if fit == .fill && hasHeightBound {
                var sizeStyle = style
                if nexaDevDimension(sizeStyle["width"]) == nil && nexaDevDimension(sizeStyle["max_width"]) == nil {
                    sizeStyle["width"] = "Fill"
                }
                if nexaDevDimension(sizeStyle["height"]) == nil {
                    sizeStyle["height"] = sizeStyle["max_height"]
                }
                let sizedBase = applyViewSize(AnyView(Color.clear), style: sizeStyle)
                rendered = AnyView(sizedBase.overlay(rendered).clipped().contentShape(Rectangle()))
                rendered = applyViewSurfaceAndEffects(rendered, style: style, locals: locals, scope: scope)
            } else {
                if fit == .fill && nexaDevDimension(style["width"]) == nil && nexaDevDimension(style["max_width"]) == nil {
                    rendered = AnyView(rendered.frame(maxWidth: .infinity).clipped())
                }
                rendered = applyViewStyle(rendered, style: style, frameAlignment: .center, locals: locals, scope: scope)
            }
            if let sharedExpression, let namespace = nexaSharedNamespace {
                let id = store.evaluate(sharedExpression, locals: locals, scope: scope) as? String ?? ""
                rendered = AnyView(rendered.matchedGeometryEffect(id: id, in: namespace))
            }
            return rendered
        case "RefreshControl":
            let state = fields["state"] as? String ?? ""
            let actions = fields["actions"] as? [Any] ?? []
            let children = fields["children"] as? [Any] ?? []
            return AnyView(NexaRefreshPrimitive(hasNativeScrollOwner: nexaDevContainsNativeScrollOwner(children)) {
                NexaDevNodeList(
                    nodes: children,
                    module: module,
                    store: store,
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
            let stateName = nexaDevTransitionStateName(fields["condition"])
            let renderContent: (Bool, [String: Any]) -> AnyView = { target, targetLocals in
                let children = target
                    ? fields["then_body"] as? [Any] ?? []
                    : fields["else_body"] as? [Any] ?? []
                return AnyView(NexaDevNodeList(
                    nodes: children,
                    module: module,
                    store: store,
                    parameters: targetLocals,
                    stateScope: scope
                ))
            }
            guard let transition = nexaDevTransition(fields["transition"]) else {
                let rendered = renderContent(condition, locals)
                return AnyView(rendered.transition(.identity))
            }
            let animation = nexaDevAnimation(fields["transition_animation"]) ?? .default
            guard stateName != nil else {
                let rendered = renderContent(condition, locals)
                return AnyView(rendered.transition(transition).animation(animation, value: condition))
            }
            return AnyView(NexaDevAnimatedWhen(
                value: condition,
                animation: animation,
                transition: transition
            ) { target in
                let targetLocals = nexaDevTransitionLocals(locals, stateName: stateName, target: target)
                return renderContent(target, targetLocals)
            })
        case "When":
            let rawValue = store.evaluate(fields["value"] ?? NSNull(), locals: locals, scope: scope)
            let value = store.stringify(rawValue)
            let cases = fields["cases"] as? [[String: Any]] ?? []
            let stateName = nexaDevTransitionStateName(fields["value"])
            let renderContent: (String, [String: Any]) -> AnyView = { target, targetLocals in
                let matchingCase = cases.first { item in
                    store.stringify(store.evaluate(item["value"] ?? NSNull(), locals: targetLocals, scope: scope)) == target
                }
                let children = matchingCase?["body"] as? [Any] ?? fields["else_body"] as? [Any] ?? []
                return AnyView(NexaDevNodeList(
                    nodes: children,
                    module: module,
                    store: store,
                    parameters: targetLocals,
                    stateScope: scope
                ))
            }
            guard let transition = nexaDevTransition(fields["transition"]) else {
                let rendered = renderContent(value, locals)
                return AnyView(rendered.transition(.identity))
            }
            let animation = nexaDevAnimation(fields["transition_animation"]) ?? .default
            guard let stableValue = rawValue as? AnyHashable, stateName != nil else {
                let rendered = renderContent(value, locals)
                return AnyView(rendered.transition(transition).animation(animation, value: value))
            }
            return AnyView(NexaDevAnimatedWhen(
                value: stableValue,
                animation: animation,
                transition: transition
            ) { target in
                let targetValue = target.base
                let targetLocals = nexaDevTransitionLocals(locals, stateName: stateName, target: targetValue)
                return renderContent(store.stringify(targetValue), targetLocals)
            })
        case "Link":
            let urlText = store.stringify(store.evaluate(fields["url"] ?? "", locals: locals, scope: scope))
            let children = fields["children"] as? [Any] ?? []
            return AnyView(NexaLinkPrimitive(destination: URL(string: urlText)) {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            })
        case "Accessibility":
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            let hint = fields["hint"].flatMap { $0 is NSNull ? nil : store.stringify(store.evaluate($0, locals: locals, scope: scope)) }
            let value = fields["value"].flatMap { $0 is NSNull ? nil : store.stringify(store.evaluate($0, locals: locals, scope: scope)) }
            let children = fields["children"] as? [Any] ?? []
            let omitLabel = children.count == 1
                && ((children.first as? [String: Any])?["Image"] as? [String: Any])?["description"] as? String == label
            return AnyView(NexaAccessibilityPrimitive(
                label: Text(label),
                hint: hint.map { Text($0) },
                value: value.map { Text($0) },
                role: fields["role"] as? String ?? "None",
                omitLabel: omitLabel
            ) {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            })
        case "KeyboardAware":
            let children = fields["children"] as? [Any] ?? []
            let toolbar = fields[NexaDevKeys.keyboardToolbar] as? [Any]
            let dismissMode = fields["dismiss"] as? String == "Interactive" ? ScrollDismissesKeyboardMode.interactively : .never
            let behavior = fields["behavior"] as? String == "Avoid" ? "avoid" : "stay"
            let hasNativeScrollOwner = nexaDevContainsNativeScrollOwner(children)
            let renderedChildren = NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            let renderedToolbar = toolbar.map {
                NexaDevNodeList(nodes: $0, module: module, store: store, parameters: locals, stateScope: scope)
            }
            let eventHandler: (Any?) -> ((NexaKeyboardEvent) -> Void)? = { raw in
                guard let handler = raw as? [String: Any],
                      let parameter = handler["parameter"] as? String else { return nil }
                let actions = handler["actions"] as? [Any] ?? []
                return { event in
                    var eventLocals = locals
                    eventLocals[parameter] = [
                        "height": event.nexa_field_height,
                        "duration": event.nexa_field_duration,
                    ]
                    store.perform(actions, scope: scope, locals: eventLocals)
                }
            }
            return AnyView(NexaKeyboardAwarePrimitive(
                behavior: behavior == "stay" ? .stay : .avoid,
                hasNativeScrollOwner: hasNativeScrollOwner,
                dismissMode: dismissMode,
                hideHorizontalScrollbar: fields[NexaDevKeys.hideHorizontalScrollbar] as? Bool ?? false,
                hideVerticalScrollbar: fields[NexaDevKeys.hideVerticalScrollbar] as? Bool ?? false,
                focusedInputOffset: CGFloat((fields[NexaDevKeys.focusedInputOffset] as? NSNumber)?.doubleValue ?? 0),
                hasToolbar: toolbar != nil,
                onShow: eventHandler(fields[NexaDevKeys.keyboardOnShow]),
                onHide: eventHandler(fields[NexaDevKeys.keyboardOnHide])
            ) { renderedChildren } toolbar: {
                if let renderedToolbar { renderedToolbar }
            })
        case "AppBottomBar":
            let state = fields["state"] as? String ?? ""
            let tabs = fields["tabs"] as? [[String: Any]] ?? []
            let customBar = fields["custom_bar"] as? [Any]
            let tintValue = fields["tint"] as? [String: Any]
            let tint: Color? = {
                guard let tintValue else { return nil }
                if let dynamic = tintValue["Dynamic"] {
                    return devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                }
                return devColor(tintValue, isDark: colorScheme == .dark)
            }()
            let selection = Binding(
                get: { (store.value(state, scope: scope) as? NSNumber)?.intValue ?? 0 },
                set: {
                    store.setValue(state, value: $0, scope: scope)
                }
            )
            let selectedTab = selection.wrappedValue
            let tabToolbars = tabs.map { tab in
                nexaDevTabToolbars(tab["children"] as? [Any] ?? [])
            }
            let tabContent: (Int) -> AnyView = { index in
                let tab = tabs[index]
                let children = nexaDevTabContentWithoutToolbars(tab["children"] as? [Any] ?? [])
                let content = NexaDevNodeList(
                    nodes: children,
                    module: module,
                    store: store,
                    parameters: locals,
                    stateScope: scope
                ).environment(\.nexaDevNodeIdentity, "\(identity)/tab/\(index)")
                let tabContent: AnyView
                if let searchState = tab["search_state"] as? String {
                    let searchInput: [String: Any] = [
                        "TextInput": [
                            "state": searchState,
                            "placeholder": tab["search_prompt"] as? String ?? "Search",
                            "keyboard": "Text",
                            "secure": false,
                            "multiline": false,
                            "return_key": "Search",
                            "autocorrect": false,
                            "capitalization": "None",
                            "max_lines": 1,
                            "searchable": true,
                        ]
                    ]
                    tabContent = AnyView(VStack(spacing: 0) {
                        renderNode(searchInput, locals: locals, scope: scope, identity: "\(identity)/search")
                            .padding(.horizontal, 16)
                        content
                    })
                } else {
                    tabContent = AnyView(content)
                }
                // AOT tab helpers fill the native TabView page and pin Nexa
                // content to its top-leading corner. Keep the dynamic renderer's
                // proposal identical so tab contents align the same on both hosts.
                var finalView = AnyView(tabContent.frame(
                    maxWidth: .infinity,
                    maxHeight: .infinity,
                    alignment: .topLeading
                ))
                if customBar != nil {
                    if #available(iOS 18.0, *) {
                        finalView = AnyView(
                            finalView
                                .toolbarVisibility(.hidden, for: .tabBar)
                                .toolbar(.hidden, for: .tabBar)
                        )
                    } else {
                        finalView = AnyView(
                            finalView
                                .toolbar(.hidden, for: .tabBar)
                        )
                    }
                }
                return finalView
            }
            var tabView: AnyView
            if #available(iOS 18.0, *) {
                tabView = AnyView(TabView(selection: selection) {
                    ForEach(0..<tabs.count, id: \.self) { index in
                        let tab = tabs[index]
                        let icon = nexaDevIconSpec(tab["icon"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
                        let title = tab["label"] as? String ?? ""
                        let tag = tab["index"] as? Int ?? index
                        let badge = tab["badge"] as? String ?? ""
                        if let icon {
                            if tab["role"] as? String == "search" {
                                if badge.isEmpty {
                                    Tab(value: tag, role: .search) { tabContent(index) } label: {
                                        NexaTabLabelPrimitive(title: Text(title), icon: icon)
                                    }
                                } else {
                                    Tab(value: tag, role: .search) { tabContent(index) } label: {
                                        NexaTabLabelPrimitive(title: Text(title), icon: icon)
                                    }
                                        .badge(badge)
                                }
                            } else {
                                if badge.isEmpty {
                                    Tab(value: tag) { tabContent(index) } label: {
                                        NexaTabLabelPrimitive(title: Text(title), icon: icon)
                                    }
                                } else {
                                    Tab(value: tag) { tabContent(index) } label: {
                                        NexaTabLabelPrimitive(title: Text(title), icon: icon)
                                    }
                                        .badge(badge)
                                }
                            }
                        } else {
                            if badge.isEmpty {
                                Tab(value: tag) { tabContent(index) } label: { Text(title) }
                            } else {
                                Tab(value: tag) { tabContent(index) } label: { Text(title) }
                                    .badge(badge)
                            }
                        }
                    }
                }.modifier(NexaAppBottomBarPrimitive(
                    tint: tint,
                    adaptiveStyle: customBar == nil,
                    activatesSearchTab: tabs.contains { $0["role"] as? String == "search" }
                )))
            } else {
                tabView = AnyView(TabView(selection: selection) {
                    ForEach(tabs.indices, id: \.self) { index in
                        let tab = tabs[index]
                        let tabItem = tabContent(index)
                            .tabItem {
                                let icon = nexaDevIconSpec(tab["icon"], store: store, locals: locals, scope: scope, isDark: colorScheme == .dark)
                                if let icon {
                                    NexaIconView(icon: icon, description: "", size: 20)
                                }
                                Text(tab["label"] as? String ?? "")
                            }
                            .tag(tab["index"] as? Int ?? index)
                        let badge = tab["badge"] as? String ?? ""
                        if badge.isEmpty {
                            tabItem
                        } else {
                            tabItem.badge(badge)
                        }
                    }
                }.modifier(NexaAppBottomBarPrimitive(
                    tint: tint,
                    adaptiveStyle: false,
                    activatesSearchTab: false
                )))
            }
            tabView = AnyView(tabView.onReceive(NotificationCenter.default.publisher(for: NSNotification.Name("NexaSelectTab"))) { note in
                guard let target = (note.object as? String)?.lowercased() else { return }
                for (index, tab) in tabs.enumerated() {
                    let tag = tab["index"] as? Int ?? index
                    let label = (tab["label"] as? String ?? "").lowercased().replacingOccurrences(of: " ", with: "_")
                    let role = (tab["role"] as? String ?? "").lowercased()
                    let matches = [
                        "\(tag)", "tab_\(tag)", "tab\(tag)",
                        "\(index)", "tab_\(index)", "tab\(index)",
                        label, "tab_\(label)", role
                    ].filter { !$0.isEmpty }
                    if matches.contains(target) {
                        selection.wrappedValue = tag
                        break
                    }
                }
            })
            if let customBar {
                var barView: AnyView = tabView
                if #available(iOS 18.0, *) {
                    barView = AnyView(barView.toolbarVisibility(.hidden, for: .tabBar))
                }
                tabView = AnyView(barView
                    .background(NexaDevTabBarHiddenProbe().frame(width: 1, height: 1).allowsHitTesting(false))
                    .modifier(NexaAppBottomBarSafeAreaPrimitive())
                    .toolbar(.hidden, for: .tabBar)
                    .safeAreaInset(edge: .bottom, spacing: 0) {
                        NexaDevNodeList(
                            nodes: customBar,
                            module: module,
                            store: store,
                                    parameters: locals,
                            stateScope: scope
                        )
                        .environment(\.nexaDevNodeIdentity, "\(identity)/custom-bar")
                    })
            }
            let titledTabs = tabs.filter {
                guard let title = $0["navigation_title"] as? String else { return false }
                return !title.isEmpty
            }
            let activeToolbars = tabs.indices.first { tabIndex in
                let tab = tabs[tabIndex]
                let tag = tab["index"] as? Int ?? tabIndex
                return selectedTab == tag
            }.map { tabToolbars[$0] } ?? []
            if !titledTabs.isEmpty || !activeToolbars.isEmpty {
                let activeTab = tabs.first(where: { ($0["index"] as? Int) == selectedTab })
                let activeTitle = activeTab?["navigation_title"] as? String ?? ""
                let inline = activeTab?["large_title"] as? Bool != true
                tabView = AnyView(
                    tabView
                        .navigationTitle(activeTitle)
                        .modifier(NexaNavigationTitleDisplayModeModifier(inline: inline))
                        .toolbar {
                            if activeToolbars.contains(where: { $0["placement"] as? String == "Leading" }) {
                                ToolbarItemGroup(placement: .navigationBarLeading) {
                                    ForEach(activeToolbars.indices, id: \.self) { toolbarIndex in
                                        let toolbar = activeToolbars[toolbarIndex]
                                        if toolbar["placement"] as? String == "Leading" {
                                            let children = toolbar["children"] as? [Any] ?? []
                                            ForEach(children.indices, id: \.self) { itemIndex in
                                                renderToolbarItem(children[itemIndex], locals: locals, scope: scope, identity: "\(identity)/toolbar/\(toolbarIndex)/leading/\(itemIndex)")
                                            }
                                        }
                                    }
                                }
                            }
                            if activeToolbars.contains(where: { $0["placement"] as? String == "Trailing" }) {
                                ToolbarItemGroup(placement: .navigationBarTrailing) {
                                    ForEach(activeToolbars.indices, id: \.self) { toolbarIndex in
                                        let toolbar = activeToolbars[toolbarIndex]
                                        if toolbar["placement"] as? String == "Trailing" {
                                            let children = toolbar["children"] as? [Any] ?? []
                                            ForEach(children.indices, id: \.self) { itemIndex in
                                                renderToolbarItem(children[itemIndex], locals: locals, scope: scope, identity: "\(identity)/toolbar/\(toolbarIndex)/trailing/\(itemIndex)")
                                            }
                                        }
                                    }
                                }
                            }
                        }
                )
            }
            return tabView
        case "PagePager":
            let state = fields["state"] as? String ?? ""
            let pages = fields["pages"] as? [[Any]] ?? []
            let selected = (store.value(state, scope: scope) as? NSNumber)?.intValue ?? 0
            let restoreRevision = store.pagePagerRestoreRevision(forState: state, scope: scope)
            return AnyView(VStack(spacing: 0) {
                TabView(selection: Binding(
                    get: { (store.value(state, scope: scope) as? NSNumber)?.intValue ?? 0 },
                    set: { store.setValue(state, value: $0, scope: scope) }
                )) {
                    ForEach(pages.indices, id: \.self) { index in
                        NexaDevNodeList(
                            nodes: pages[index],
                            module: module,
                            store: store,
                                    parameters: locals,
                            stateScope: scope
                        )
                        .environment(\.nexaDevNodeIdentity, "\(identity)/page/\(index)")
                        .environment(\.nexaDevPageRestoreRevision, restoreRevision)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                        .tag(index)
                    }
                }
                .tabViewStyle(.page(indexDisplayMode: .never))
                .animation(.default, value: selected)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                NexaPageIndicatorRow(pageCount: pages.count, selectedIndex: selected) { index in
                    store.setValue(state, value: index, scope: scope)
                }
            }.frame(maxWidth: .infinity, maxHeight: .infinity))
        case "Toolbar":
            let children = fields["children"] as? [Any] ?? []
            let placement: ToolbarItemPlacement = fields["placement"] as? String == "Leading" ? .navigationBarLeading : .navigationBarTrailing
            return AnyView(EmptyView().toolbar {
                ToolbarItemGroup(placement: placement) {
                    ForEach(children.indices, id: \.self) { index in
                        renderToolbarItem(children[index], locals: locals, scope: scope, identity: "\(identity)/children/\(index)")
                    }
                }
            })
        case "BottomSheet":
            let children = fields["children"] as? [Any] ?? []
            let componentRef = fields["component_ref"]
                .flatMap { value -> NexaBottomSheetRef? in
                    guard !(value is NSNull) else { return nil }
                    return store.evaluate(value, locals: locals, scope: scope) as? NexaBottomSheetRef
                } ?? NexaBottomSheetRef()
            let contentSized = fields["content_sized"] as? Bool ?? false
            let cornerRadius = (fields["corner_radius"] as? NSNumber).map { CGFloat(truncating: $0) }
            let detentFractions = (fields["detents"] as? [NSNumber] ?? [0.5, 1.0])
                .map { CGFloat(truncating: $0) }
            let detents = Set(detentFractions.map { PresentationDetent.fraction($0) })
            let initialFraction = (fields["initial_detent"] as? NSNumber)
                .map { CGFloat(truncating: $0) } ?? detentFractions.first ?? 0.5
            let initialDetent = PresentationDetent.fraction(initialFraction)
            let showDragIndicator = fields["show_drag_indicator"] as? Bool ?? true
            let contentPadding = (fields["content_padding"] as? NSNumber)
                .map { CGFloat(truncating: $0) } ?? 0
            let allowsDismissal = fields["allows_dismissal"] as? Bool ?? true
            let onDismissActions = fields["on_dismiss"] as? [Any] ?? []
            let backgroundColor: Color?
            if let value = fields["background_color"] as? [String: Any] {
                if let dynamic = value["Dynamic"] {
                    backgroundColor = devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                } else {
                    backgroundColor = devColor(value, isDark: colorScheme == .dark)
                }
            } else {
                backgroundColor = nil
            }
            let scrimColor: Color?
            if let value = fields["scrim_color"] as? [String: Any] {
                if let dynamic = value["Dynamic"] {
                    scrimColor = devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope)))
                } else {
                    scrimColor = devColor(value, isDark: colorScheme == .dark)
                }
            } else {
                scrimColor = nil
            }
            return AnyView(NexaBottomSheetPrimitive(
                componentRef: componentRef,
                contentSized: contentSized,
                contentPadding: contentPadding,
                cornerRadius: cornerRadius,
                detents: detents,
                initialDetent: initialDetent,
                showDragIndicator: showDragIndicator,
                allowsDismissal: allowsDismissal,
                allowsBackgroundInteraction: fields["allows_background_interaction"] as? Bool ?? false,
                shouldDismissOnBackPress: fields["should_dismiss_on_back_press"] as? Bool ?? true,
                backgroundColor: backgroundColor,
                scrimColor: scrimColor,
                onDismiss: { store.perform(onDismissActions, scope: scope, locals: locals) }
            ) {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            })
        case "Dialog":
            let state = fields["state"] as? String ?? ""
            let title = store.stringify(store.evaluate(fields["title"] ?? NSNull(), locals: locals, scope: scope))
            let message = store.stringify(store.evaluate(fields["message"] ?? NSNull(), locals: locals, scope: scope))
            let children = fields["children"] as? [Any] ?? []
            return AnyView(NexaAlertDialogPrimitive(isPresented: Binding(
                get: { store.truthy(store.value(state, scope: scope)) },
                set: { store.setValue(state, value: $0, scope: scope) }
            ), title: Text(title), message: message.isEmpty ? nil : Text(message)) {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            })
        case "ConfirmationDialog":
            let state = fields["state"] as? String ?? ""
            let title = store.stringify(store.evaluate(fields["title"] ?? NSNull(), locals: locals, scope: scope))
            let children = fields["children"] as? [Any] ?? []
            return AnyView(NexaConfirmationDialogPrimitive(isPresented: Binding(
                get: { store.truthy(store.value(state, scope: scope)) },
                set: { store.setValue(state, value: $0, scope: scope) }
            ), title: Text(title)) {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            })
        case "NavigationStack":
            return navigationStack(fields, locals: locals, scope: scope, identity: identity)
        case "NavigationSplitView":
            let sidebar = fields["sidebar"] as? [Any] ?? []
            let detail = fields["detail"] as? [Any] ?? []
            let visibleState = fields["detail_visible"] as? String ?? ""
            let sidebarView = NexaDevNodeList(nodes: sidebar, module: module, store: store, parameters: locals, stateScope: scope)
            let detailView = NexaDevNodeList(nodes: detail, module: module, store: store, parameters: locals, stateScope: scope)
            return AnyView(NexaNavigationSplitViewPrimitive(
                detailVisible: Binding(
                    get: { store.value(visibleState, scope: scope) as? Bool ?? false },
                    set: { store.setValue(visibleState, value: $0, scope: scope) }
                ),
                sidebar: {
                    sidebarView
                },
                detail: {
                    detailView
                }
            ))
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
            let staticallyDisabled = (guardExpression as? [String: Any])?["Bool"] as? Bool == false
            guard !staticallyDisabled else {
                return AnyView(NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope))
            }
            let arguments = fields["arguments"] as? [Any] ?? []
            let parameters = screens[screenIndex]["parameters"] as? [[String: Any]] ?? []
            let argumentValues: [String: Any] = Dictionary(uniqueKeysWithValues: zip(parameters, arguments).compactMap { parameter, expression -> (String, Any)? in
                guard let name = parameter["name"] as? String else { return nil }
                return (name, store.evaluate(expression, locals: locals, scope: scope))
            })
            let signature = NexaDevStateStore.canonicalSignature(parameters)
            let navId = (locals["__nexa_navigation_stack_identity"] as? String)
                ?? store.resolveNavigatorId(stackIdentity: nil, locals: locals, scope: scope)
            let route = store.navigationRoute(screen: screenName, values: argumentValues, signature: signature, navigatorId: navId)
            return AnyView(NexaNavigationLinkPrimitive(value: route, enabled: enabled) {
                NexaDevNodeList(nodes: children, module: module, store: store, parameters: locals, stateScope: scope)
            })
        case "NavigationBack":
            let label = store.stringify(store.evaluate(fields["label"] ?? "Back", locals: locals, scope: scope))
            return AnyView(Button(label) {
                store.popNavigation(
                    stackIdentity: locals["__nexa_navigation_stack_identity"] as? String,
                    scope: scope
                )
            })
        default:
            return AnyView(EmptyView())
        }
    }

@MainActor
private struct NexaDevTabBarHiddenProbe: UIViewRepresentable {
    func makeUIView(context: Context) -> ProbeView { ProbeView() }
    func updateUIView(_ view: ProbeView, context: Context) { view.hideTabBar() }

    @MainActor
    final class ProbeView: UIView {
        override func didMoveToWindow() {
            super.didMoveToWindow()
            DispatchQueue.main.async { [weak self] in
                self?.hideTabBar()
            }
        }
        func hideTabBar() {
            var responder: UIResponder? = self
            while let current = responder {
                if let tab = current as? UITabBarController {
                    tab.tabBar.isHidden = true
                    return
                }
                responder = current.next
            }
        }
    }
}

@MainActor
private struct NexaDevNavigationBackSwipeModifier: ViewModifier {
    @Binding var path: [NexaDevRoute]

    func body(content: Content) -> some View {
        content.background(
            NexaDevNavigationBackGestureProbe(path: $path)
                .frame(width: 1, height: 1)
                .allowsHitTesting(false)
        )
    }
}

@MainActor
private struct NexaDevNavigationBackGestureProbe: UIViewRepresentable {
    @Binding var path: [NexaDevRoute]

    func makeCoordinator() -> Coordinator {
        Coordinator(path: $path)
    }

    func makeUIView(context: Context) -> ProbeView {
        let view = ProbeView()
        view.coordinator = context.coordinator
        return view
    }

    func updateUIView(_ view: ProbeView, context: Context) {
        context.coordinator.path = $path
        view.coordinator = context.coordinator
        view.installGesture()
    }

    @MainActor
    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        var path: Binding<[NexaDevRoute]>

        init(path: Binding<[NexaDevRoute]>) {
            self.path = path
        }

        func install(from view: UIView) {
            var responder: UIResponder? = view
            while let current = responder {
                let nav = (current as? UINavigationController) ?? (current as? UIViewController)?.navigationController
                if let navigationController = nav,
                   let popGesture = navigationController.interactivePopGestureRecognizer {
                    popGesture.delegate = self
                    popGesture.isEnabled = true
                    return
                }
                responder = current.next
            }
            if let root = view.window?.rootViewController {
                if let nav = findNavController(in: root) {
                    nav.interactivePopGestureRecognizer?.delegate = self
                    nav.interactivePopGestureRecognizer?.isEnabled = true
                }
            }
        }

        private func findNavController(in root: UIViewController) -> UINavigationController? {
            if let nav = root as? UINavigationController {
                return nav
            }
            for child in root.children {
                if let found = findNavController(in: child) {
                    return found
                }
            }
            return nil
        }

        func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
            guard !path.wrappedValue.isEmpty else { return false }
            guard let pan = gestureRecognizer as? UIPanGestureRecognizer,
                  let view = gestureRecognizer.view else { return true }
            let velocity = pan.velocity(in: view)
            let isLeadingEdgeSwipe = view.effectiveUserInterfaceLayoutDirection == .rightToLeft
                ? velocity.x < 0
                : velocity.x > 0
            return isLeadingEdgeSwipe && abs(velocity.x) > abs(velocity.y)
        }
    }

    @MainActor
    final class ProbeView: UIView {
        weak var coordinator: Coordinator?

        override func didMoveToWindow() {
            super.didMoveToWindow()
            DispatchQueue.main.async { [weak self] in
                self?.installGesture()
            }
        }

        func installGesture() {
            coordinator?.install(from: self)
        }
    }
}

private struct NexaHideTabBarModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 18.0, *) {
            content
                .toolbarVisibility(.hidden, for: .tabBar)
                .toolbar(.hidden, for: .tabBar)
        } else {
            content
                .toolbar(.hidden, for: .tabBar)
        }
    }
}

private struct NexaDevNavigationStackHostView: View {
    @ObservedObject var store: NexaDevStateStore
    let stackIdentity: String
    let showNavigationBar: Bool
    let titleMode: String
    let titleAlignment: String
    let rootContent: AnyView
    let screens: [[String: Any]]
    let renderScreen: ([String: Any], [String: Any], String, String, String) -> AnyView

    var body: some View {
        let leadingTitle: Bool
        if titleAlignment == "Leading" {
            leadingTitle = true
        } else {
            leadingTitle = false
        }
        let scope = stackIdentity
        let path = Binding(
            get: { store.navigationPath(for: stackIdentity) },
            set: {
                store.setNavigationPath($0, for: stackIdentity)
                store.invalidateScope(scope)
            }
        )
        let popAction: () -> Void = {
            store.popNavigation(stackIdentity: stackIdentity, scope: scope)
        }
        let stack = NavigationStack(path: path) {
            rootContent
                .environment(\.nexaDevNavigationPath, path)
                .environment(\.nexaDevNavigationStackIdentity, stackIdentity)
                .navigationDestination(for: NexaDevRoute.self) { route in
                    guard let destination = store.routeDestination(route),
                          let screen = screens.first(where: { $0["name"] as? String == destination.screen })
                    else { return AnyView(EmptyView()) }
                    let scope = "navigator/\(route.navigatorId)/entry/\(route.entryId)/screen/\(destination.screen)"
                    let screenContent = renderScreen(screen, destination.values, scope, route.entryId, route.navigatorId)
                        .environment(\.nexaDevNavigationPath, path)
                        .environment(\.nexaDevNavigationStackIdentity, stackIdentity)

                    if showNavigationBar {
                        return AnyView(
                            NexaNavigationScreenPrimitive(
                                title: destination.screen,
                                inline: titleMode == "Inline",
                                leadingTitle: leadingTitle
                            ) {
                                screenContent
                            }
                        )
                    }
                    return AnyView(
                        screenContent
                            .toolbar(.hidden, for: .navigationBar)
                            .modifier(NexaHideTabBarModifier())
                            .ignoresSafeArea(.container, edges: .all)
                    )
                }
        }
        .environment(\.nexaDevNavigationPath, path)
        .environment(\.nexaDevNavigationStackIdentity, stackIdentity)

        let styledStack = stack
            .onOpenURL { url in
                store.handleDeepLink(url: url, screens: screens, stackIdentity: stackIdentity)
            }
            .modifier(NexaDevNavigationBackSwipeModifier(path: path))

        if !showNavigationBar {
            return AnyView(
                styledStack
                    .toolbar(.hidden, for: .navigationBar)
                    .modifier(NexaHideTabBarModifier())
                    .ignoresSafeArea(.container, edges: .all)
            )
        } else {
            return AnyView(styledStack)
        }
    }
}

    private func navigationStack(
        _ fields: [String: Any],
        locals: [String: Any],
        scope: String,
        identity: String
    ) -> AnyView {
        let screens = module["screens"] as? [[String: Any]] ?? []
        let rootIndex = fields["root"] as? Int ?? -1
        guard screens.indices.contains(rootIndex) else { return AnyView(EmptyView()) }
        let rootScreen = screens[rootIndex]
        let rootArguments = fields["arguments"] as? [Any] ?? []
        let showNavigationBar = fields["show_navigation_bar"] as? Bool ?? true
        let titleMode = fields["title_mode"] as? String ?? "Inline"
        let titleAlignment = fields["title_alignment"] as? String ?? "Automatic"
        let rootScreenName = rootScreen["name"] as? String ?? ""
        let stackIdentity = "\(identity)/root/\(rootScreenName)"
        let rootValues = argumentValues(rootScreen, expressions: rootArguments, locals: locals, scope: scope)
        let content = renderScreen(rootScreen, parameters: rootValues, scope: "screen/\(rootScreenName)", entryId: "root", navigatorId: stackIdentity)
        let hasRootToolbar = !nexaDevTabToolbars(rootScreen["body"] as? [Any] ?? []).isEmpty
        let rootContent: AnyView = if !showNavigationBar {
            AnyView(content.ignoresSafeArea(.container, edges: .all))
        } else if !hasRootToolbar {
            AnyView(
                content
                    .toolbar(.hidden, for: .navigationBar)
                    .ignoresSafeArea(.container, edges: .top)
            )
        } else {
            content
        }
        return AnyView(
            NexaDevNavigationStackHostView(
                store: store,
                stackIdentity: stackIdentity,
                showNavigationBar: showNavigationBar,
                titleMode: titleMode,
                titleAlignment: titleAlignment,
                rootContent: rootContent,
                screens: screens,
                renderScreen: { [self] screen, params, sc, entryId, navId in
                    renderScreen(screen, parameters: params, scope: sc, entryId: entryId, navigatorId: navId)
                }
            )
        )
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

    private func renderScreen(
        _ screen: [String: Any],
        parameters: [String: Any],
        scope: String? = nil,
        entryId: String = "root",
        navigatorId: String = "app/root"
    ) -> AnyView {
        let name = screen["name"] as? String ?? ""
        let currentScreen = store.screen(named: name) ?? screen
        let effectiveScope = scope ?? (entryId == "root" ? "screen/\(name)" : "navigator/\(navigatorId)/entry/\(entryId)/screen/\(name)")
        let stateDeclarations = currentScreen["states"] as? [[String: Any]] ?? []
        var screenParameters = parameters
        screenParameters["__nexa_navigation_stack_identity"] = navigatorId
        screenParameters["__nexa_navigation_entry_id"] = entryId
        let screenStatusBar = (currentScreen["status_bar"] as? [String: Any])
            ?? (module["status_bar"] as? [String: Any])
        return AnyView(NexaDevNodeList(
            nodes: currentScreen["body"] as? [Any] ?? [],
            module: store.currentModule,
            store: store,
            parameters: screenParameters,
            stateDeclarations: stateDeclarations,
            screenName: name,
            stateScope: effectiveScope
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .modifier(NexaDevStatusBarConfigurationModifier(
            config: screenStatusBar,
            safeInsets: visualSafeInsets,
            isDark: colorScheme == .dark
        ))
        .onAppear {
            var screenBindings = store.bindings(
                stateDeclarations,
                parameters: screenParameters,
                scope: effectiveScope
            )
            screenBindings["__nexa_navigation_stack_identity"] = navigatorId
            screenBindings["__nexa_navigation_entry_id"] = entryId
            store.screenDidAppear(scope: effectiveScope, parameters: screenBindings)
            let actions = currentScreen["on_appear"] as? [Any] ?? []
            if currentScreen["on_appear_async"] as? Bool == true {
                Task { @MainActor in
                    do {
                        _ = try await store.performAsync(actions, scope: effectiveScope, locals: screenBindings)
                    } catch is CancellationError {
                        return
                    } catch {
                        store.reportRuntimeFailure(error)
                    }
                }
            } else {
                store.perform(actions, scope: effectiveScope, locals: screenBindings)
            }
        }
        .onDisappear {
            var screenBindings = store.bindings(
                stateDeclarations,
                parameters: screenParameters,
                scope: effectiveScope
            )
            screenBindings["__nexa_navigation_stack_identity"] = navigatorId
            screenBindings["__nexa_navigation_entry_id"] = entryId
            store.perform(currentScreen["on_disappear"] as? [Any] ?? [], scope: effectiveScope, locals: screenBindings)
            store.screenDidDisappear(scope: effectiveScope, navigationStackIdentity: navigatorId, routeIdentity: entryId)
        })
    }
}

@MainActor
func playNexaHaptic(_ style: String?) {
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

@MainActor
private func nexaDevMarqueeRevision(_ nodes: [Any], store: NexaDevStateStore, locals: [String: Any], scope: String) -> [String] {
    var values: [String] = []
    func visit(_ value: Any) {
        if let object = value as? [String: Any] {
            if object["State"] != nil || object["String"] != nil { values.append(store.stringify(store.evaluate(object, locals: locals, scope: scope))); return }
            for key in object.keys.sorted() { if let child = object[key] { visit(child) } }
        } else if let array = value as? [Any] { array.forEach(visit) }
    }
    nodes.forEach(visit)
    return values
}
