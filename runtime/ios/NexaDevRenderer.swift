import Foundation
import SwiftUI
import UIKit

private let nexaDevFormRowMinHeight: CGFloat = __NEXA_FORM_ROW_MIN_HEIGHT__
private let nexaDevFormRowHorizontalInset: CGFloat = __NEXA_FORM_ROW_HORIZONTAL_INSET__

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
    case "SlideFromBottom": .move(edge: .bottom)
    case "SlideFromLeft": .move(edge: .leading)
    case "SlideFromRight": .move(edge: .trailing)
    case "Scale": .scale
    default: nil
    }
}

private func nexaDevSharedIconSymbol(_ name: String) -> String? {
    switch name {
__NEXA_SHARED_ICON_SF_CASES__
    default: return nil
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

extension EnvironmentValues {
    var nexaDevContentSlot: NexaDevContentSlot? {
        get { self[NexaDevContentSlotKey.self] }
        set { self[NexaDevContentSlotKey.self] = newValue }
    }
}

struct NexaDevListPositionPreference: PreferenceKey {
    static let defaultValue: [Int: CGFloat] = [:]

    static func reduce(value: inout [Int: CGFloat], nextValue: () -> [Int: CGFloat]) {
        value.merge(nextValue(), uniquingKeysWith: { _, newest in newest })
    }
}

struct NexaDevViewportSizePreference: PreferenceKey {
    static let defaultValue: CGSize = .zero

    static func reduce(value: inout CGSize, nextValue: () -> CGSize) {
        value = nextValue()
    }
}

@MainActor
enum NexaDevFastListAxis {
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
struct NexaDevFastList<RowContent: View, StickyHeaderContent: View, SectionHeaderContent: View>: View {
    let count: Int
    let axis: NexaDevFastListAxis
    let reverseLayout: Bool
    let pageSnap: Bool
    let rowHeight: Double?
    let scrollPosition: Int?
    let sectionCounts: [Int]?
    let stickyHeader: StickyHeaderContent?
    let sectionHeader: ((Int) -> SectionHeaderContent)?
    let row: (Int, Int, Int) -> RowContent
    let onMove: ((IndexSet, Int) -> Void)?
    let moveEnabled: Bool
    let onScrollPositionChanged: (Int) -> Void
    let onScroll: ((Int) -> Void)?
    let onEndReached: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    @State private var canUpdateScrollPosition = false
    @State private var didReachEnd = false
    @State private var followsBottom = true
    @State private var horizontalViewportSize: CGSize = .zero

    @ViewBuilder
    var body: some View {
        if let onMove {
            List {
                ForEach(0..<count, id: \.self) { index in
                    row(index, 0, index)
                }
                .onMove(perform: onMove)
                .moveDisabled(!moveEnabled)
            }
            .listStyle(.plain)
        } else if pageSnap {
            GeometryReader { _ in
                NexaDevPageSnapList(
                    count: count,
                    scrollPosition: scrollPosition,
                    row: row,
                    onScrollPositionChanged: onScrollPositionChanged,
                    onScroll: onScroll,
                    onEndReached: onEndReached
                )
            }
        } else {
            if case .horizontal = axis {
                ScrollViewReader { proxy in
                    scrollView(proxy: proxy, viewportSize: horizontalViewportSize)
                        .fixedSize(horizontal: false, vertical: true)
                        .background(GeometryReader { viewport in
                            Color.clear.preference(
                                key: NexaDevViewportSizePreference.self,
                                value: viewport.size
                            )
                        })
                        .onPreferenceChange(NexaDevViewportSizePreference.self) { size in
                            horizontalViewportSize = size
                        }
                }
            } else {
                GeometryReader { viewport in
                    ScrollViewReader { proxy in
                        scrollView(proxy: proxy, viewportSize: viewport.size)
                    }
                }
            }
        }
    }

    private func scrollView(proxy: ScrollViewProxy, viewportSize: CGSize) -> some View {
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
            } else if reverseLayout && count > 0 {
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    proxy.scrollTo(count - 1, anchor: .bottom)
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
        .onChange(of: reverseLayout) { isReversed in
            guard isReversed, count > 0 else { return }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                proxy.scrollTo(count - 1, anchor: .bottom)
            }
        }
        .onChange(of: count) { newCount in
            didReachEnd = false
            guard reverseLayout, followsBottom, newCount > 0 else { return }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                proxy.scrollTo(newCount - 1, anchor: .bottom)
            }
        }
        .onPreferenceChange(NexaDevListPositionPreference.self) { offsets in
            let viewportExtent = axis.tracksHorizontalOffset ? viewportSize.width : viewportSize.height
            let visibleRows = offsets.filter {
                $0.value >= 0 && $0.value <= viewportExtent
            }
            guard let visibleIndex = visibleRows
                .min(by: { abs($0.value) < abs($1.value) })?.key
            else {
                followsBottom = false
                return
            }
            if reverseLayout {
                followsBottom = offsets[count - 1].map {
                    $0 >= 0 && $0 <= viewportExtent
                } ?? false
            }
            onScroll?(visibleIndex)
            if canUpdateScrollPosition { onScrollPositionChanged(visibleIndex) }
            if (reverseLayout ? visibleIndex == 0 : visibleIndex >= count - 10), !didReachEnd {
                didReachEnd = true
                onEndReached?(visibleIndex)
            } else if (reverseLayout ? visibleIndex > 0 : visibleIndex < count - 1) {
                didReachEnd = false
            }
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
                            if let sectionHeader {
                                sectionHeader(sectionIndex)
                            } else {
                                EmptyView()
                            }
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
                guard (reverseLayout ? index == 0 : index >= count - 1), !didReachEnd else { return }
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

@MainActor
private final class NexaDevPageSnapTableView: UITableView {
    var onViewportSizeChanged: ((CGSize) -> Void)?
    private var reportedBoundsSize = CGSize.zero

    override func layoutSubviews() {
        super.layoutSubviews()
        guard bounds.size != reportedBoundsSize else { return }
        reportedBoundsSize = bounds.size
        onViewportSizeChanged?(bounds.size)
    }
}

@MainActor
private struct NexaDevPageSnapList<RowContent: View>: UIViewRepresentable {
    let count: Int
    let scrollPosition: Int?
    let row: (Int, Int, Int) -> RowContent
    let onScrollPositionChanged: (Int) -> Void
    let onScroll: ((Int) -> Void)?
    let onEndReached: ((Int) -> Void)?

    func makeCoordinator() -> Coordinator {
        Coordinator(
            count: count,
            scrollPosition: scrollPosition,
            row: row,
            onScrollPositionChanged: onScrollPositionChanged,
            onScroll: onScroll,
            onEndReached: onEndReached
        )
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = NexaDevPageSnapTableView(frame: .zero, style: .plain)
        let coordinator = context.coordinator
        tableView.onViewportSizeChanged = { [weak tableView, weak coordinator] size in
            DispatchQueue.main.async {
                guard let tableView, let coordinator else { return }
                coordinator.updatePageHeight(size.height, in: tableView)
            }
        }
        tableView.dataSource = coordinator
        tableView.delegate = coordinator
        tableView.isPagingEnabled = true
        tableView.contentInsetAdjustmentBehavior = .never
        tableView.separatorStyle = .none
        tableView.allowsSelection = false
        tableView.showsVerticalScrollIndicator = false
        tableView.backgroundColor = .clear
        tableView.rowHeight = 1
        tableView.estimatedRowHeight = 1
        tableView.register(UITableViewCell.self, forCellReuseIdentifier: "NexaDevPageSnapCell")
        tableView.reloadData()
        return tableView
    }

    func updateUIView(_ tableView: UITableView, context: Context) {
        let coordinator = context.coordinator
        let previousCount = coordinator.count
        let previousPosition = coordinator.scrollPosition
        let previousPage = coordinator.currentPage(in: tableView)
        coordinator.count = max(0, count)
        coordinator.scrollPosition = scrollPosition
        coordinator.row = row
        coordinator.onScrollPositionChanged = onScrollPositionChanged
        coordinator.onScroll = onScroll
        coordinator.onEndReached = onEndReached
        if previousCount != coordinator.count {
            coordinator.lastEndReachedCount = nil
            tableView.reloadData()
            tableView.layoutIfNeeded()
            let targetPage = scrollPosition ?? previousPage
            coordinator.scrollToPage(targetPage, in: tableView)
        }
        if let scrollPosition, scrollPosition != previousPosition {
            coordinator.scrollToPage(scrollPosition, in: tableView)
        }
    }

    @MainActor
    final class Coordinator: NSObject, UITableViewDataSource, UITableViewDelegate {
        var count: Int
        var scrollPosition: Int?
        var pageHeight: CGFloat = 0
        var row: (Int, Int, Int) -> RowContent
        var onScrollPositionChanged: (Int) -> Void
        var onScroll: ((Int) -> Void)?
        var onEndReached: ((Int) -> Void)?
        var lastReportedPage: Int?
        var lastEndReachedCount: Int?

        init(
            count: Int,
            scrollPosition: Int?,
            row: @escaping (Int, Int, Int) -> RowContent,
            onScrollPositionChanged: @escaping (Int) -> Void,
            onScroll: ((Int) -> Void)?,
            onEndReached: ((Int) -> Void)?
        ) {
            self.count = max(0, count)
            self.scrollPosition = scrollPosition
            self.row = row
            self.onScrollPositionChanged = onScrollPositionChanged
            self.onScroll = onScroll
            self.onEndReached = onEndReached
            self.lastReportedPage = nil
            self.lastEndReachedCount = nil
        }

        func updatePageHeight(_ height: CGFloat, in tableView: UITableView) {
            guard height > 0, abs(height - pageHeight) > 0.5 else { return }
            let page = pageHeight > 0
                ? currentPage(in: tableView)
                : min(max(scrollPosition ?? 0, 0), max(0, count - 1))
            pageHeight = height
            tableView.rowHeight = height
            tableView.estimatedRowHeight = height
            tableView.reloadData()
            tableView.layoutIfNeeded()
            scrollToPage(page, in: tableView)
        }

        func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
            count
        }

        func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
            let cell = tableView.dequeueReusableCell(
                withIdentifier: "NexaDevPageSnapCell",
                for: indexPath
            )
            cell.selectionStyle = .none
            cell.contentConfiguration = UIHostingConfiguration {
                row(indexPath.row, 0, indexPath.row)
            }
            .margins(.all, 0)
            return cell
        }

        func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
            reportSettledPage(in: scrollView)
        }

        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate { reportSettledPage(in: scrollView) }
        }

        func scrollViewDidEndScrollingAnimation(_ scrollView: UIScrollView) {
            reportSettledPage(in: scrollView)
        }

        func scrollToPage(_ requestedPage: Int, in tableView: UITableView) {
            guard count > 0, pageHeight > 0 else { return }
            let page = min(max(requestedPage, 0), count - 1)
            let targetOffsetY = CGFloat(page) * pageHeight - tableView.adjustedContentInset.top
            guard abs(tableView.contentOffset.y - targetOffsetY) > 1 else { return }
            tableView.scrollToRow(
                at: IndexPath(row: page, section: 0),
                at: .top,
                animated: false
            )
            lastReportedPage = page
        }

        func currentPage(in tableView: UITableView) -> Int {
            guard pageHeight > 0, count > 0 else { return 0 }
            let position = (tableView.contentOffset.y + tableView.adjustedContentInset.top) / pageHeight
            return min(max(Int(position.rounded()), 0), count - 1)
        }

        private func reportSettledPage(in scrollView: UIScrollView) {
            guard let tableView = scrollView as? UITableView, count > 0 else { return }
            let page = currentPage(in: tableView)
            guard page != lastReportedPage else { return }
            lastReportedPage = page
            onScrollPositionChanged(page)
            onScroll?(page)
            if page >= count - 1, lastEndReachedCount != count {
                lastEndReachedCount = count
                onEndReached?(page)
            } else if page < count - 1 {
                lastEndReachedCount = nil
            }
        }
    }
}

@MainActor
struct NexaDevNodeList: View {
    let nodes: [Any]
    let module: [String: Any]
    @ObservedObject var store: NexaDevStateStore
    var focusedField: FocusState<String?>.Binding
    var parameters: [String: Any] = [:]
    var stateScope: String = "app"
    var rendersFormRows = false
    var appliesFormRowMetrics = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.nexaSharedNamespace) private var nexaSharedNamespace
    @Environment(\.nexaDevContentSlot) private var contentSlot

    @ViewBuilder var body: some View {
        let _ = store.revision
        let locals = store.locals(scope: stateScope, parameters: parameters)
        if nodes.isEmpty {
            EmptyView()
        } else if rendersFormRows {
            ForEach(nodes.indices, id: \.self) { index in
                if appliesFormRowMetrics,
                   (nodes[index] as? [String: Any])?.keys.first != "FormSection" {
                    NexaFormRowPrimitive(
                        minHeight: nexaDevFormRowMinHeight,
                        horizontalInset: nexaDevFormRowHorizontalInset
                    ) {
                        renderNode(nodes[index], locals: locals, scope: stateScope)
                    }
                } else {
                    renderNode(nodes[index], locals: locals, scope: stateScope)
                }
            }
        } else if nodes.count == 1 {
            renderNode(nodes[0], locals: locals, scope: stateScope)
        } else {
            VStack(alignment: .center, spacing: 0) {
            ForEach(nodes.indices, id: \.self) { index in
                renderNode(nodes[index], locals: locals, scope: stateScope)
            }
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
        case "Appearance":
            let mode = store.stringify(store.evaluate(fields["mode"] ?? "system", locals: locals, scope: scope))
            let children = fields["children"] as? [Any] ?? []
            let content = NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            return AnyView(content.preferredColorScheme(mode == "dark" ? .dark : mode == "light" ? .light : nil))
        case "Form":
            let children = fields["children"] as? [Any] ?? []
            return AnyView(Form {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope, rendersFormRows: true, appliesFormRowMetrics: true)
            })
        case "FormSection":
            let children = fields["children"] as? [Any] ?? []
            let section = Section {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope, rendersFormRows: true, appliesFormRowMetrics: true)
            } header: {
                if let rawTitle = fields["title"], !(rawTitle is NSNull) {
                    Text(store.stringify(store.evaluate(rawTitle, locals: locals, scope: scope)))
                } else {
                    EmptyView()
                }
            } footer: {
                if let rawFooter = fields["footer"], !(rawFooter is NSNull) {
                    Text(store.stringify(store.evaluate(rawFooter, locals: locals, scope: scope)))
                } else {
                    EmptyView()
                }
            }
            return AnyView(section)
        case "Layout":
            let children = fields["children"] as? [Any] ?? []
            let toolbarNodes = children.compactMap { rawNode -> [String: Any]? in
                guard let node = rawNode as? [String: Any],
                      let payload = node["Toolbar"] as? [String: Any]
                else { return nil }
                return payload
            }
            let contentChildren = children.filter { rawNode in
                guard let node = rawNode as? [String: Any] else { return true }
                return node.keys.first != "Toolbar"
            }
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
            let nativeSpacing = CGFloat(spacing)
            let container: AnyView
            switch fields["kind"] as? String {
            case "Row":
                container = AnyView(NexaRowPrimitive(alignment: rowAlignment, spacing: nativeSpacing) { ForEach(contentChildren.indices, id: \.self) { renderNode(contentChildren[$0], locals: locals, scope: scope) } })
            case "Stack":
                container = AnyView(NexaStackPrimitive(alignment: stackAlignment) { ForEach(contentChildren.indices, id: \.self) { renderNode(contentChildren[$0], locals: locals, scope: scope) } })
            default:
                container = AnyView(NexaColumnPrimitive(alignment: columnAlignment, spacing: nativeSpacing) { ForEach(contentChildren.indices, id: \.self) { renderNode(contentChildren[$0], locals: locals, scope: scope) } })
            }
            var styled = container
            let width = style["width"] as? Double
            let height = style["height"] as? Double
            let minWidth = style["min_width"] as? Double
            let maxWidth = style["max_width"] as? Double
            let minHeight = style["min_height"] as? Double
            let maxHeight = style["max_height"] as? Double
            if width != nil || height != nil || minWidth != nil || maxWidth != nil || minHeight != nil || maxHeight != nil {
                if let width, let height { styled = AnyView(styled.frame(width: width, height: height)) }
                else if let width { styled = AnyView(styled.frame(width: width)) }
                else if let height { styled = AnyView(styled.frame(height: height)) }
                if minWidth != nil || maxWidth != nil || minHeight != nil || maxHeight != nil {
                    styled = AnyView(styled.frame(
                        minWidth: minWidth.map { CGFloat($0) }, maxWidth: maxWidth.map { CGFloat($0) },
                        minHeight: minHeight.map { CGFloat($0) }, maxHeight: maxHeight.map { CGFloat($0) }
                    ))
                }
            }
            if let padding = style["padding"] as? Double { styled = AnyView(styled.padding(padding)) }
            if let background = devColor(style["background"], isDark: colorScheme == .dark) {
                styled = AnyView(styled.background(background))
            }
            if let radius = style["corner_radius"] as? Double {
                styled = AnyView(styled.clipShape(RoundedRectangle(cornerRadius: radius)))
            }
            if let borderWidth = style["border_width"] as? Double,
               let border = devColor(style["border_color"], isDark: colorScheme == .dark) {
                let width = borderWidth
                styled = AnyView(styled.overlay(RoundedRectangle(cornerRadius: style["corner_radius"] as? Double ?? 0).stroke(border, lineWidth: width)))
            }
            if let opacity = style["opacity"] as? Double { styled = AnyView(styled.opacity(opacity)) }
            let effects = style["effects"] as? [String: Any] ?? [:]
            if let scale = effects["scale"] as? Double { styled = AnyView(styled.scaleEffect(scale)) }
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
            if let spring = (style["animation"] as? [String: Any])?["Spring"] as? [String: Any],
               let response = spring["response"] as? Double,
               let damping = spring["damping"] as? Double {
                let value = Animation.spring(response: response, dampingFraction: damping)
                styled = AnyView(styled.animation(value, value: store.revision))
            } else if let animation = style["animation"] as? String {
                let value: Animation
                switch animation {
                case "EaseIn": value = .easeIn
                case "EaseOut": value = .easeOut
                case "EaseInOut": value = .easeInOut
                default: value = .linear
                }
                styled = AnyView(styled.animation(value, value: store.revision))
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
                                renderNode(items[itemIndex], locals: locals, scope: scope)
                            }
                        }
                    }
                })
            }
            return styled
        case "ContentUnavailable":
            let title = store.stringify(store.evaluate(fields["title"] ?? "", locals: locals, scope: scope))
            let description = store.stringify(store.evaluate(fields["description"] ?? "", locals: locals, scope: scope))
            let icon = fields["icon"] as? [String: String] ?? [:]
            let symbol = nexaDevSharedIconSymbol(icon["shared"] ?? "") ?? "questionmark"
            return AnyView(NexaContentUnavailablePrimitive(title: Text(title), symbol: symbol, description: Text(description)))
        case "Text":
            let text = store.stringify(store.evaluate(fields["value"] ?? "", locals: locals, scope: scope))
            let style = fields["style"] as? [String: Any] ?? [:]
            let alignment: TextAlignment? = switch style["alignment"] as? String {
            case "Leading": .leading
            case "Center": .center
            case "Trailing": .trailing
            default: nil
            }
            let font: Font? = switch style["font_style"] as? String {
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
            let resolvedFont = (style["font_size"] as? Double).map { Font.system(size: $0) } ?? font
            let weight: Font.Weight? = switch style["font_weight"] as? String {
            case "Normal": .regular
            case "Medium": .medium
            case "Semibold": .semibold
            case "Bold": .bold
            default: nil
            }
            var styled = AnyView(NexaTextPrimitive(
                text: Text(text),
                alignment: alignment,
                color: devColor(style["color"], isDark: colorScheme == .dark),
                font: resolvedFont,
                weight: weight,
                lineLimit: style["line_limit"] as? Int,
                lineSpacing: style["line_height"] as? CGFloat,
                tracking: style["letter_spacing"] as? CGFloat,
                strikethrough: style["strikethrough"] as? Bool == true,
                selectable: style["selectable"] as? Bool == true
            ))
            if let padding = style["padding"] as? Double { styled = AnyView(styled.padding(padding)) }
            if let opacity = style["opacity"] as? Double { styled = AnyView(styled.opacity(opacity)) }
            let effects = style["effects"] as? [String: Any] ?? [:]
            if let scale = effects["scale"] as? Double { styled = AnyView(styled.scaleEffect(scale)) }
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
            return AnyView(Spacer())
        case "Divider":
            let color = devColor(fields["color"], isDark: colorScheme == .dark)
            let thickness = fields["thickness"] as? Double ?? 1
            return AnyView(NexaDividerPrimitive(color: color ?? .gray, thickness: CGFloat(thickness)))
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
            let content = AnyView(NexaDevNodeList(
                nodes: children,
                module: module,
                store: store,
                focusedField: focusedField,
                parameters: locals,
                stateScope: scope
            ))
            if let component = NexaDevPluginBridge.renderComponent(
                namespace: namespace,
                name: name,
                arguments: arguments,
                events: events,
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
            let style: NexaSharedButtonStyleKind = switch fields["style"] as? String {
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
            let icon = fields["icon"] as? [String: String] ?? [:]
            let symbol = icon["sf_symbol"] ?? icon["shared"].flatMap(nexaDevSharedIconSymbol)
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
                glass: fields["glass"] as? Bool == true,
                action: { store.perform(actions, scope: scope, locals: locals) },
                label: {
                    if let symbol {
                        Label(label, systemImage: symbol)
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
            let longPressDurationMs = max(1, (store.evaluate(fields[NexaDevKeys.longPressDurationMs] ?? ["Number": ["raw": "500", "ty": "Int32"]], locals: locals, scope: scope) as? NSNumber)?.intValue ?? 500)
            let longPressMinimumDuration = Double(longPressDurationMs) / 1000.0
            let dragParameters = fields[NexaDevKeys.dragParameters] as? [String] ?? []
            let dragActions = fields[NexaDevKeys.dragActions] as? [Any] ?? []
            let pinchParameter = fields[NexaDevKeys.pinchParameter] as? String
            let pinchActions = fields[NexaDevKeys.pinchActions] as? [Any] ?? []
            let disabled = fields["disabled"].map { store.truthy(store.evaluate($0, locals: locals, scope: scope)) } ?? false
            let fillMaxSize = fields["fill_max_size"] as? Bool == true
            let haptic = fields["haptic"] as? String
            let content = NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            let button = Button {
                if doubleTapActions.isEmpty {
                    playNexaHaptic(haptic)
                    store.perform(actions, scope: scope, locals: locals)
                }
            } label: {
                content
            }
            .buttonStyle(.plain)
            .disabled(disabled)
            let pressable: AnyView
            if !doubleTapActions.isEmpty {
                let tapped = button
                    .highPriorityGesture(
                        TapGesture(count: 2)
                            .onEnded {
                                guard !disabled else { return }
                                playNexaHaptic(haptic)
                                store.perform(doubleTapActions, scope: scope, locals: locals)
                            }
                            .exclusively(before: TapGesture(count: 1).onEnded {
                                guard !disabled else { return }
                                playNexaHaptic(haptic)
                                store.perform(actions, scope: scope, locals: locals)
                            })
                    )
                    .accessibilityAction(.default) {
                        playNexaHaptic(haptic)
                        store.perform(actions, scope: scope, locals: locals)
                    }
                    .accessibilityAddTraits(.isButton)
                if longPressActions.isEmpty {
                    pressable = AnyView(tapped)
                } else {
                    pressable = AnyView(tapped.simultaneousGesture(LongPressGesture(minimumDuration: longPressMinimumDuration).onEnded { _ in
                        guard !disabled else { return }
                        playNexaHaptic(haptic)
                        store.perform(longPressActions, scope: scope, locals: locals)
                    }))
                }
            } else if longPressActions.isEmpty {
                pressable = AnyView(button)
            } else {
                pressable = AnyView(button.simultaneousGesture(LongPressGesture(minimumDuration: longPressMinimumDuration).onEnded { _ in
                    guard !disabled else { return }
                    playNexaHaptic(haptic)
                    store.perform(longPressActions, scope: scope, locals: locals)
                }))
            }
            let expandedPressable: AnyView = fillMaxSize
                ? AnyView(pressable.frame(maxWidth: .infinity, maxHeight: .infinity).contentShape(Rectangle()))
                : pressable
            let pressableWithMenu: AnyView
            if contextMenu.isEmpty {
                pressableWithMenu = expandedPressable
            } else {
                pressableWithMenu = AnyView(expandedPressable.contextMenu {
                    NexaDevNodeList(nodes: contextMenu, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
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
            let identity = "\(scope)/input/\(name)"
            let maxLength = fields["max_length"] as? Int
            let onChange = fields["on_change"] as? [String: Any]
            let binding = Binding(
                get: { store.stringify(store.value(name, scope: scope)) },
                set: { nextValue in
                    let value = maxLength.map { String(nextValue.prefix(max(0, $0))) } ?? nextValue
                    store.setValue(name, value: value, scope: scope)
                    if let onChange,
                       let parameter = onChange["parameter"] as? String {
                        var changeLocals = locals
                        changeLocals[parameter] = value
                        store.perform(onChange["actions"] as? [Any] ?? [], scope: scope, locals: changeLocals)
                    }
                }
            )
            var input = AnyView(NexaTextInputPrimitive(
                text: binding,
                placeholder: Text(placeholder),
                secure: fields["secure"] as? Bool == true,
                multiline: fields["multiline"] as? Bool == true,
                searchable: fields["searchable"] as? Bool == true
            ))
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
            switch fields["font"] as? String {
            case "Body": input = AnyView(input.font(.body))
            case "Title3": input = AnyView(input.font(.title3))
            default: break
            }
            let minLines = fields["min_lines"] as? Int
            let maxLines = fields["max_lines"] as? Int
            switch (minLines, maxLines) {
            case let (.some(minimum), .some(maximum)):
                input = AnyView(input.lineLimit(minimum...maximum))
            case let (.some(minimum), .none):
                input = AnyView(input.lineLimit(minimum...))
            case let (.none, .some(maximum)):
                input = AnyView(input.lineLimit(...maximum))
            case (.none, .none):
                break
            }
            let autofill: UITextContentType? = switch fields["autofill"] as? String {
            case "Username": .username
            case "Password": .password
            case "OneTimeCode": .oneTimeCode
            default: nil
            }
            if let autofill {
                input = AnyView(input.textContentType(autofill))
            }
            let returnKey: SubmitLabel? = switch fields["return_key"] as? String {
            case "Done": .done
            case "Search": .search
            case "Send": .send
            case "Next": .next
            default: nil
            }
            if let returnKey {
                input = AnyView(input.submitLabel(returnKey))
            }
            if fields["autocorrect"] as? Bool == false || fields["capitalization"] as? String == "None" {
                input = AnyView(input.autocorrectionDisabled())
            } else if fields["autocorrect"] as? Bool == true {
                input = AnyView(input.autocorrectionDisabled(false))
            }
            input = AnyView(input.focused(focusedField, equals: identity))
            let submitActions = fields["actions"] as? [Any] ?? []
            if !submitActions.isEmpty {
                input = AnyView(input.onSubmit {
                    store.perform(submitActions, scope: scope, locals: locals)
                })
            }
            return input
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
            var listFields = options
            for (key, value) in countPlan where key != "common" {
                listFields[key] = value
            }
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
            let count = sourceSections?.reduce(0) { $0 + $1.count } ?? itemCount
            let axis: NexaDevFastListAxis
            if listFields["axis"] as? String == "Horizontal" {
                axis = .horizontal
            } else if let grid = (listFields["axis"] as? [String: Any])?["Grid"] as? [String: Any],
                      let columns = grid["columns"] as? Int {
                axis = .grid(max(1, columns))
            } else {
                axis = .vertical
            }
            let reverseLayout = listFields[NexaDevKeys.reverseLayout] as? Bool ?? false
            let pageSnap = listFields[NexaDevKeys.pageSnap] as? Bool ?? false
            let indexName = listFields["index"] as? String ?? "index"
            let itemName = listFields["item"] as? String
            let sectionName = listFields["section"] as? String ?? "section"
            let children = listFields["children"] as? [Any] ?? []
            let itemExtent = listFields["item_extent"] as? Double
            let scrollPositionName = listFields["scroll_position"] as? String
            let requestedIndex = scrollPositionName.flatMap { name in
                (store.value(name, scope: scope) as? NSNumber)?.intValue
            }
            let stickyHeaderNodes = listFields["sticky_header"] as? [Any]
            let stickyHeader: NexaDevNodeList? = stickyHeaderNodes.map { nodes in
                NexaDevNodeList(nodes: nodes, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            }
            let sectionHeaderNodes = listFields["section_header"] as? [Any]
            let sectionHeader: ((Int) -> NexaDevNodeList)? = sectionHeaderNodes.map { nodes in
                { sectionIndex in
                    NexaDevNodeList(
                        nodes: nodes,
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: locals.merging([sectionName: sectionIndex]) { _, newest in newest },
                        stateScope: scope
                    )
                }
            }
            let onScrollActions = listFields["on_scroll"] as? [Any]
            let onEndReachedActions = listFields["on_end_reached"] as? [Any]
            let onMove = listFields["on_move"] as? [String: Any]
            let moveEnabled = onMove?["enabled"].map {
                store.truthy(store.evaluate($0, locals: locals, scope: scope))
            } ?? false
            let refresh = listFields["refresh"] as? [String: Any]
            let refreshState = refresh?["state"] as? String
            let refreshActions = refresh?["actions"] as? [Any] ?? []
            return AnyView(NexaDevFastList(
                count: count,
                axis: axis,
                reverseLayout: reverseLayout,
                pageSnap: pageSnap,
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
                    return NexaDevNodeList(
                        nodes: children,
                        module: module,
                        store: store,
                        focusedField: focusedField,
                        parameters: rowLocals,
                        stateScope: scope
                    )
                },
                onMove: onMove.map { callback in
                    { source, destination in
                        guard moveEnabled else { return }
                        guard let from = source.first else { return }
                        let to = destination > from ? destination - 1 : destination
                        let callbackLocals = locals.merging([
                            callback["from"] as? String ?? "from": NSNumber(value: from),
                            callback["to"] as? String ?? "to": NSNumber(value: to),
                        ]) { _, newest in newest }
                        store.perform(callback["actions"] as? [Any] ?? [], scope: scope, locals: callbackLocals)
                    }
                },
                moveEnabled: moveEnabled,
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
            let label = store.stringify(store.evaluate(fields["label"] ?? "", locals: locals, scope: scope))
            return AnyView(NexaSwitchPrimitive(label: Text(label), isOn: Binding(
                get: { store.truthy(store.value(name, scope: scope)) },
                set: { store.setValue(name, value: $0, scope: scope) }
            )))
        case "Slider":
            let name = fields["state"] as? String ?? ""
            let min = fields["min"] as? Double ?? 0
            let max = fields["max"] as? Double ?? 1
            let step = fields["step"] as? Double ?? 0.1
            return AnyView(NexaSliderPrimitive(value: Binding(
                get: { (store.value(name, scope: scope) as? NSNumber)?.doubleValue ?? min },
                set: { store.setValue(name, value: $0, scope: scope) }
            ), range: min...max, step: step))
        case "ProgressBar":
            let progress = (store.evaluate(fields["progress"] ?? NSNull(), locals: locals, scope: scope) as? NSNumber)?.doubleValue ?? 0
            return AnyView(NexaProgressBarPrimitive(progress: progress))
        case "ProgressRing":
            let progress = (store.evaluate(fields["progress"] ?? NSNull(), locals: locals, scope: scope) as? NSNumber)?.doubleValue ?? 0
            return AnyView(NexaProgressRingPrimitive(progress: progress))
        case "SegmentedControl":
            let name = fields["state"] as? String ?? ""
            let items = store.evaluate(fields["items"] ?? NSNull(), locals: locals, scope: scope) as? [String] ?? []
            return AnyView(NexaSegmentedControlPrimitive(items: items, selection: Binding(
                get: { store.value(name, scope: scope) as? String ?? "" },
                set: { store.setValue(name, value: $0, scope: scope) }
            )))
        case "Picker":
            let name = fields["state"] as? String ?? ""
            let items = store.evaluate(fields["items"] ?? NSNull(), locals: locals, scope: scope) as? [String] ?? []
            let iconSelection = fields["icon"] as? [String: String] ?? [:]
            let icon = iconSelection["sf_symbol"] ?? iconSelection["shared"].flatMap(nexaDevSharedIconSymbol)
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
            ), items: items, icon: icon, tint: tint, hasLabel: label != nil) {
                if let label {
                    Text(label)
                } else {
                    EmptyView()
                }
            })
        case "DatePicker":
            let timestampName = fields["timestamp_state"] as? String ?? ""
            let hasTimeName = fields["has_time_state"] as? String ?? ""
            return AnyView(NexaDatePickerPrimitive(timestamp: Binding(
                get: { (store.value(timestampName, scope: scope) as? NSNumber)?.int64Value ?? 0 },
                set: { store.setValue(timestampName, value: $0, scope: scope) }
            ), includesTime: Binding(
                get: { (store.value(hasTimeName, scope: scope) as? Bool) ?? false },
                set: { store.setValue(hasTimeName, value: $0, scope: scope) }
            )))
        case "SystemIcon":
            let icon = fields["icon"] as? [String: String] ?? [:]
            let symbol: String
            if let shared = icon["shared"] {
                symbol = nexaDevSharedIconSymbol(shared) ?? "questionmark"
            } else if let sfSymbol = icon["sf_symbol"] {
                symbol = sfSymbol
            } else if icon["material_symbol"] != nil {
                symbol = "questionmark"
            } else {
                symbol = "questionmark"
            }
            let description = fields["description"] as? String ?? ""
            let size = fields["size"] as? Double ?? 24
            let tintValue = fields["tint"] as? [String: Any] ?? [:]
            let tint: Color
            if let dynamic = tintValue["Dynamic"] {
                tint = devHexColor(store.stringify(store.evaluate(dynamic, locals: locals, scope: scope))) ?? .white
            } else {
                tint = devColor(fields["tint"], isDark: colorScheme == .dark) ?? .white
            }
            return AnyView(NexaSystemIconPrimitive(symbol: symbol, description: description, size: size, tint: tint))
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
            let mode: ContentMode = fields["scale"] as? String == "Fill" ? .fill : .fit
            let placeholder = fields["placeholder"] as? String
            var rendered: AnyView
            if let asset = source["Asset"] as? String {
                rendered = AnyView(Image(asset).resizable().aspectRatio(contentMode: mode))
            } else if let expression = source["RemoteUrl"] {
                let urlText = store.stringify(store.evaluate(expression, locals: locals, scope: scope))
                let remoteURL = URL(string: urlText).flatMap { $0.scheme?.lowercased() == "https" ? $0 : nil }
                rendered = AnyView(AsyncImage(url: remoteURL) { phase in
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
            } else if let expression = source["LocalFile"] {
                let urlText = store.stringify(store.evaluate(expression, locals: locals, scope: scope))
                let fileURL = URL(string: urlText).flatMap { $0.isFileURL ? $0 : nil }
                rendered = AnyView(AsyncImage(url: fileURL) { phase in
                    if let image = phase.image {
                        image.resizable().aspectRatio(contentMode: mode)
                    } else {
                        Image(systemName: "photo")
                    }
                })
            } else {
                if let placeholder {
                    rendered = AnyView(Image(placeholder).resizable().aspectRatio(contentMode: mode))
                } else {
                    rendered = AnyView(Image(systemName: "photo"))
                }
            }
            if let maxHeight = fields["max_height"] as? Double {
                rendered = AnyView(rendered.frame(maxHeight: maxHeight))
            }
            if let sharedExpression = fields["shared_element"] as? [String: Any],
               let namespace = nexaSharedNamespace {
                let id = store.evaluate(sharedExpression, locals: locals, scope: scope) as? String ?? ""
                rendered = AnyView(rendered.matchedGeometryEffect(id: id, in: namespace))
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
            let rendered = AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            guard let transition = nexaDevTransition(fields["transition"]) else { return rendered }
            return AnyView(rendered.transition(transition).animation(.default, value: condition))
        case "When":
            let value = store.stringify(store.evaluate(fields["value"] ?? NSNull(), locals: locals, scope: scope))
            let cases = fields["cases"] as? [[String: Any]] ?? []
            let matchingCase = cases.first { item in
                store.stringify(store.evaluate(item["value"] ?? NSNull(), locals: locals, scope: scope)) == value
            }
            let children = matchingCase?["body"] as? [Any] ?? fields["else_body"] as? [Any] ?? []
            let rendered = AnyView(NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope))
            guard let transition = nexaDevTransition(fields["transition"]) else { return rendered }
            return AnyView(rendered.transition(transition).animation(.default, value: value))
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
            let value = fields["value"].flatMap { $0 is NSNull ? nil : store.stringify(store.evaluate($0, locals: locals, scope: scope)) }
            let children = fields["children"] as? [Any] ?? []
            let content = NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            var accessible = AnyView(content.accessibilityElement(children: .combine).accessibilityLabel(label).accessibilityHint(hint ?? ""))
            if let value { accessible = AnyView(accessible.accessibilityValue(value)) }
            let role = fields["role"] as? String
            switch role {
            case "Button": return AnyView(accessible.accessibilityAddTraits(.isButton))
            case "Link": return AnyView(accessible.accessibilityAddTraits(.isLink))
            case "Header": return AnyView(accessible.accessibilityAddTraits(.isHeader))
            case "Image": return AnyView(accessible.accessibilityAddTraits(.isImage))
            default: return accessible
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
                    store.revision += 1
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
                    focusedField: focusedField,
                    parameters: locals,
                    stateScope: scope
                )
                guard let searchState = tab["search_state"] as? String else {
                    return AnyView(content)
                }
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
                return AnyView(VStack(spacing: 0) {
                    renderNode(searchInput, locals: locals, scope: scope)
                        .padding(.horizontal, 16)
                    content
                })
            }
            var tabView: AnyView
            if #available(iOS 18.0, *) {
                tabView = AnyView(TabView(selection: selection) {
                    ForEach(0..<tabs.count, id: \.self) { index in
                        let tab = tabs[index]
                        let icon = tab["icon"] as? [String: String] ?? [:]
                        let title = tab["label"] as? String ?? ""
                        let tag = tab["index"] as? Int ?? index
                        let badge = tab["badge"] as? String ?? ""
                        if let name = icon["sf_symbol"] ?? icon["shared"].flatMap(nexaDevSharedIconSymbol) {
                            if tab["role"] as? String == "search" {
                                if badge.isEmpty {
                                    Tab(title, systemImage: name, value: tag, role: .search) { tabContent(index) }
                                } else {
                                    Tab(title, systemImage: name, value: tag, role: .search) { tabContent(index) }
                                        .badge(badge)
                                }
                            } else {
                                if badge.isEmpty {
                                    Tab(title, systemImage: name, value: tag) { tabContent(index) }
                                } else {
                                    Tab(title, systemImage: name, value: tag) { tabContent(index) }
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
                }.tabViewStyle(.sidebarAdaptable))
                if tabs.contains(where: { $0["role"] as? String == "search" }), #available(iOS 26.0, *) {
                    tabView = AnyView(tabView.tabViewSearchActivation(.searchTabSelection))
                }
            } else {
                tabView = AnyView(TabView(selection: selection) {
                    ForEach(tabs.indices, id: \.self) { index in
                        let tab = tabs[index]
                        let tabItem = tabContent(index)
                            .tabItem {
                                let icon = tab["icon"] as? [String: String] ?? [:]
                                if let name = icon["sf_symbol"] ?? icon["shared"].flatMap(nexaDevSharedIconSymbol) {
                                    Image(systemName: name)
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
                })
            }
            if let tint { tabView = AnyView(tabView.tint(tint)) }
            let titledTabs = tabs.filter {
                guard let title = $0["navigation_title"] as? String else { return false }
                return !title.isEmpty
            }
            if !titledTabs.isEmpty {
                let activeTitle = tabs.first(where: { ($0["index"] as? Int) == selectedTab })?["navigation_title"] as? String ?? ""
                tabView = AnyView(tabView.navigationTitle(activeTitle))
                if titledTabs.allSatisfy({ $0["large_title"] as? Bool == true }) {
                    if #available(iOS 26.0, *) {
                        tabView = AnyView(tabView.toolbarTitleDisplayMode(.inlineLarge))
                    } else {
                        tabView = AnyView(tabView.navigationBarTitleDisplayMode(.large))
                    }
                } else {
                    tabView = AnyView(tabView.navigationBarTitleDisplayMode(.inline))
                }
            }
            let activeToolbars = tabs.indices.first { tabIndex in
                let tab = tabs[tabIndex]
                let tag = tab["index"] as? Int ?? tabIndex
                return selectedTab == tag
            }.map { tabToolbars[$0] } ?? []
            if !activeToolbars.isEmpty {
                tabView = AnyView(tabView.toolbar {
                    ForEach(activeToolbars.indices, id: \.self) { toolbarIndex in
                        let toolbar = activeToolbars[toolbarIndex]
                        let placement: ToolbarItemPlacement = toolbar["placement"] as? String == "Leading"
                            ? .navigationBarLeading
                            : .navigationBarTrailing
                        let children = toolbar["children"] as? [Any] ?? []
                        ForEach(children.indices, id: \.self) { itemIndex in
                            ToolbarItem(placement: placement) {
                                renderNode(children[itemIndex], locals: locals, scope: scope)
                            }
                        }
                    }
                })
            }
            return tabView
        case "PagePager":
            let state = fields["state"] as? String ?? ""
            let pages = fields["pages"] as? [[Any]] ?? []
            let selected = (store.value(state, scope: scope) as? NSNumber)?.intValue ?? 0
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
                            focusedField: focusedField,
                            parameters: locals,
                            stateScope: scope
                        )
                        .tag(index)
                    }
                }
                .tabViewStyle(.page(indexDisplayMode: .never))
                .animation(.default, value: selected)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                HStack(spacing: __NEXA_PAGE_INDICATOR_SPACING__) {
                    ForEach(pages.indices, id: \.self) { index in
                        Button {
                            store.setValue(state, value: index, scope: scope)
                        } label: {
                            Circle()
                                .fill(index == selected ? __NEXA_SWIFT_DEFAULT_ACCENT_COLOR__ : Color(red: __NEXA_MUTED_TEXT_RED__, green: __NEXA_MUTED_TEXT_GREEN__, blue: __NEXA_MUTED_TEXT_BLUE__).opacity(__NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__))
                                .frame(width: index == selected ? __NEXA_PAGE_INDICATOR_SELECTED_SIZE__ : __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__, height: index == selected ? __NEXA_PAGE_INDICATOR_SELECTED_SIZE__ : __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Page \(index + 1)")
                            .animation(.default, value: selected)
                    }
                }
                .padding(.bottom, __NEXA_PAGE_INDICATOR_BOTTOM_INSET__)
            }.frame(maxWidth: .infinity, maxHeight: .infinity))
        case "Toolbar":
            let children = fields["children"] as? [Any] ?? []
            let placement: ToolbarItemPlacement = fields["placement"] as? String == "Leading" ? .navigationBarLeading : .navigationBarTrailing
            return AnyView(EmptyView().toolbar {
                ToolbarItemGroup(placement: placement) {
                    ForEach(children.indices, id: \.self) { index in
                        renderNode(children[index], locals: locals, scope: scope)
                    }
                }
            })
        case "BottomSheet":
            let state = fields["state"] as? String ?? ""
            let children = fields["children"] as? [Any] ?? []
            let binding = Binding<Bool>(
                get: { store.truthy(store.value(state, scope: scope)) },
                set: { store.setValue(state, value: $0, scope: scope) }
            )
            let isPartial = fields["partial"] as? Bool == true
            let largeOnly = fields["large_only"] as? Bool == true
            let sheetContent: AnyView
            if isPartial,
               let rawTitle = fields["title"],
               !(rawTitle is NSNull) {
                let title = store.stringify(store.evaluate(rawTitle, locals: locals, scope: scope))
                sheetContent = AnyView(NavigationStack {
                    Group {
                        NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
                    }
                    .navigationTitle(title)
                    .navigationBarTitleDisplayMode(.inline)
                })
            } else {
                sheetContent = AnyView(
                    NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
                )
            }
            return AnyView(NexaBottomSheetPrimitive(
                isPresented: binding,
                partial: isPartial,
                largeOnly: largeOnly
            ) {
                sheetContent
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
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            })
        case "ConfirmationDialog":
            let state = fields["state"] as? String ?? ""
            let title = store.stringify(store.evaluate(fields["title"] ?? NSNull(), locals: locals, scope: scope))
            let children = fields["children"] as? [Any] ?? []
            return AnyView(NexaConfirmationDialogPrimitive(isPresented: Binding(
                get: { store.truthy(store.value(state, scope: scope)) },
                set: { store.setValue(state, value: $0, scope: scope) }
            ), title: Text(title)) {
                NexaDevNodeList(nodes: children, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            })
        case "NavigationStack":
            return navigationStack(fields, locals: locals, scope: scope)
        case "NavigationSplitView":
            let sidebar = fields["sidebar"] as? [Any] ?? []
            let detail = fields["detail"] as? [Any] ?? []
            let visibleState = fields["detail_visible"] as? String ?? ""
            let sidebarView = NexaDevNodeList(nodes: sidebar, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            let detailView = NexaDevNodeList(nodes: detail, module: module, store: store, focusedField: focusedField, parameters: locals, stateScope: scope)
            if #available(iOS 17.0, *) {
                return AnyView(NavigationSplitView(preferredCompactColumn: Binding(
                    get: { (store.value(visibleState, scope: scope) as? Bool ?? false) ? .detail : .sidebar },
                    set: { store.setValue(visibleState, value: $0 == .detail, scope: scope) }
                )) {
                    sidebarView
                } detail: {
                    detailView
                })
            }
            if #available(iOS 16.0, *) {
                return AnyView(NavigationSplitView {
                    sidebarView
                } detail: {
                    detailView
                })
            }
            return AnyView(HStack(spacing: 0) { sidebarView; Divider(); detailView })
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
                return AnyView(
                    renderScreen(screen, parameters: destination.values)
                        .navigationTitle(destination.screen)
                        .navigationBarTitleDisplayMode(.large)
                        .toolbar(.hidden, for: .tabBar)
                )
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
            store.screenDidAppear(scope: scope, parameters: parameters)
            let actions = screen["on_appear"] as? [Any] ?? []
            if screen["on_appear_async"] as? Bool == true {
                Task { @MainActor in
                    do {
                        _ = try await store.performAsync(actions, scope: scope, locals: parameters)
                    } catch is CancellationError {
                        return
                    } catch {
                        store.reportRuntimeFailure(error)
                    }
                }
            } else {
                store.perform(actions, scope: scope, locals: parameters)
            }
        }
        .onDisappear {
            store.perform(screen["on_disappear"] as? [Any] ?? [], scope: scope, locals: parameters)
            store.screenDidDisappear(scope: scope)
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
