use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;
use nexa_codegen::SourceWriter;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_fast_list, "UIKit");
}

pub(crate) fn render(
    out: &mut SourceWriter,
    uses_sticky_header: bool,
    uses_scroll_events: bool,
    uses_vertical_list: bool,
    uses_horizontal_list: bool,
    uses_grid_list: bool,
    uses_sectioned_list: bool,
) {
    let mut runtime = String::from(
        r#"
private struct NexaFastListScrollPositionEchoes {
    private var positions: [Int] = []

    mutating func record(_ position: Int) {
        positions.append(position)
        if positions.count > 64 {
            positions.removeFirst(positions.count - 64)
        }
    }

    mutating func consume(_ position: Int32?) -> Bool {
        guard let position = position.map(Int.init),
              let echoIndex = positions.lastIndex(of: position)
        else {
            return false
        }
        // SwiftUI can deliver an intermediate state update before the view
        // update carrying the scroll-position callback. Keep unmatched echoes
        // until their corresponding state arrives; consume coalesced earlier
        // positions together with the latest matching value.
        positions.removeSubrange(0...echoIndex)
        return true
    }
}

@MainActor
private final class NexaFastListRefreshController: NSObject {
    let control = UIRefreshControl()
    var action: (() -> Void)?

    init(action: @escaping () -> Void) {
        self.action = action
        super.init()
        control.addTarget(self, action: #selector(refresh), for: .valueChanged)
    }

    @objc private func refresh() {
        action?()
    }

    func update(isRefreshing: Bool) {
        if isRefreshing {
            if !control.isRefreshing {
                control.beginRefreshing()
            }
        } else if control.isRefreshing {
            control.endRefreshing()
        }
    }
}

@MainActor
private func nexaUpdateRefreshControl(
    _ scrollView: UIScrollView,
    controller: inout NexaFastListRefreshController?,
    isRefreshing: Bool,
    action: (() -> Void)?
) {
    guard let action else {
        scrollView.refreshControl = nil
        controller = nil
        return
    }
    if controller == nil {
        controller = NexaFastListRefreshController(action: action)
        scrollView.refreshControl = controller?.control
    } else {
        controller?.action = action
    }
    controller?.update(isRefreshing: isRefreshing)
}

@MainActor
private func nexaFastListIsOverscrolling(_ scrollView: UIScrollView, horizontal: Bool) -> Bool {
    let insets = scrollView.adjustedContentInset
    let leadingInset = horizontal ? insets.left : insets.top
    let trailingInset = horizontal ? insets.right : insets.bottom
    let contentExtent = horizontal ? scrollView.contentSize.width : scrollView.contentSize.height
    let viewportExtent = horizontal ? scrollView.bounds.width : scrollView.bounds.height
    let offset = horizontal ? scrollView.contentOffset.x : scrollView.contentOffset.y
    let minimumOffset = -leadingInset
    let maximumOffset = max(minimumOffset, contentExtent - viewportExtent + trailingInset)
    return offset < minimumOffset - 1 || offset > maximumOffset + 1
}

@MainActor
func nexaConfigureEdgeToEdgeScrollView(
    _ scrollView: UIScrollView,
    environment: EnvironmentValues,
    reverseLayout: Bool = false
) {
    scrollView.contentInsetAdjustmentBehavior = .never
    let padding = environment.nexaScrollContentPadding
    let rtl = environment.layoutDirection == .rightToLeft
    let insets = UIEdgeInsets(top: reverseLayout ? padding.bottom : padding.top,
        left: rtl ? padding.trailing : padding.leading,
        bottom: reverseLayout ? padding.top : padding.bottom, right: rtl ? padding.leading : padding.trailing)
    let previous = scrollView.contentInset
    guard previous != insets else { return }
    let atTop = scrollView.contentOffset.y <= -previous.top + 1
    let atLeading = scrollView.contentOffset.x <= -previous.left + 1
    scrollView.contentInset = insets
    if atTop { scrollView.contentOffset.y = -insets.top }
    if atLeading { scrollView.contentOffset.x = -insets.left }
}

// <nexa:list-runtime-sectioned:begin>
private let nexaFastSectionedCellReuseIdentifier = "NexaFastSectionedCell"
private let nexaFastSectionedHeaderReuseIdentifier = "NexaFastSectionedHeader"

@available(iOS 16.0, *)
private struct NexaFastSectionedList<RowContent: View, HeaderContent: View>: UIViewRepresentable {
    let sectionCount: Int
    let sectionCounts: [Int]
    let rowHeight: CGFloat?
    let spacing: CGFloat
    let showDividers: Bool
    let hideHorizontalScrollbar: Bool
    let hideVerticalScrollbar: Bool
    let scrollEnabled: Bool
    let rowKey: ((Int, Int) -> AnyHashable)?
    let componentRef: NexaFastListRef?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    let headerContent: ((Int) -> HeaderContent)?
    let rowContent: (Int, Int) -> RowContent

    init(
        sectionCount: Int,
        sectionCounts: [Int],
        rowHeight: CGFloat? = nil,
        spacing: CGFloat = 0,
        showDividers: Bool = true,
        hideHorizontalScrollbar: Bool = false,
        hideVerticalScrollbar: Bool = false,
        scrollEnabled: Bool = true,
        rowKey: ((Int, Int) -> AnyHashable)? = nil,
        componentRef: NexaFastListRef? = nil,
        isRefreshing: Bool = false,
        onRefresh: (() -> Void)? = nil,
        headerContent: ((Int) -> HeaderContent)? = nil,
        @ViewBuilder rowContent: @escaping (Int, Int) -> RowContent
    ) {
        self.sectionCount = max(0, sectionCount)
        self.sectionCounts = sectionCounts.map { max(0, $0) }
        self.rowHeight = rowHeight
        self.spacing = max(0, spacing)
        self.showDividers = showDividers
        self.hideHorizontalScrollbar = hideHorizontalScrollbar
        self.hideVerticalScrollbar = hideVerticalScrollbar
        self.scrollEnabled = scrollEnabled
        self.rowKey = rowKey
        self.componentRef = componentRef
        self.isRefreshing = isRefreshing
        self.onRefresh = onRefresh
        self.headerContent = headerContent
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(
            sectionCount: sectionCount,
            sectionCounts: sectionCounts,
            rowHeight: rowHeight,
            spacing: spacing,
            rowKey: rowKey,
            headerContent: headerContent,
            rowContent: rowContent
        )
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = UITableView(frame: .zero, style: .plain)
        nexaConfigureEdgeToEdgeScrollView(tableView, environment: context.environment)
        tableView.separatorStyle = showDividers ? .singleLine : .none
        tableView.dataSource = context.coordinator
        tableView.delegate = context.coordinator
        tableView.alwaysBounceVertical = onRefresh != nil
        tableView.isScrollEnabled = scrollEnabled
        tableView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        tableView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        tableView.sectionHeaderTopPadding = 0
        nexaUpdateRefreshControl(
            tableView,
            controller: &context.coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        tableView.register(UITableViewCell.self, forCellReuseIdentifier: nexaFastSectionedCellReuseIdentifier)
        tableView.register(
            UITableViewHeaderFooterView.self,
            forHeaderFooterViewReuseIdentifier: nexaFastSectionedHeaderReuseIdentifier
        )
        if let rowHeight {
            tableView.rowHeight = rowHeight
            tableView.estimatedRowHeight = rowHeight
        } else {
            tableView.rowHeight = UITableView.automaticDimension
            tableView.estimatedRowHeight = 44
        }
        tableView.allowsSelection = false
        tableView.backgroundColor = .clear
        tableView.reloadData()
        nexaAttachFastListRef(componentRef, to: tableView, horizontal: false) { [weak tableView] index, animated, viewPosition in
            guard let tableView else { return }
            var remaining = max(0, Int(index))
            for section in 0..<sectionCounts.count {
                let count = sectionCounts[section]
                if remaining < count {
                    let path = IndexPath(row: remaining, section: section)
                    tableView.layoutIfNeeded()
                    let frame = tableView.rectForRow(at: path)
                    nexaScrollFastList(to: frame, in: tableView, horizontal: false, viewPosition: viewPosition, animated: animated)
                    return
                }
                remaining -= count
            }
        }
        return tableView
    }

    func updateUIView(_ tableView: UITableView, context: Context) {
        nexaConfigureEdgeToEdgeScrollView(tableView, environment: context.environment)
        let coordinator = context.coordinator
        let previousSectionCount = coordinator.sectionCount
        let previousSectionCounts = coordinator.sectionCounts
        coordinator.sectionCount = sectionCount
        // The immutable view initializer already normalizes these counts.
        coordinator.sectionCounts = sectionCounts
        coordinator.rowHeight = rowHeight
        coordinator.spacing = max(0, spacing)
        tableView.isScrollEnabled = scrollEnabled
        tableView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        tableView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        nexaAttachFastListRef(componentRef, to: tableView, horizontal: false) { [weak tableView] index, animated, viewPosition in
            guard let tableView else { return }
            var remaining = max(0, Int(index))
            for section in 0..<sectionCounts.count {
                let count = sectionCounts[section]
                if remaining < count {
                    let path = IndexPath(row: remaining, section: section)
                    tableView.layoutIfNeeded()
                    let frame = tableView.rectForRow(at: path)
                    nexaScrollFastList(to: frame, in: tableView, horizontal: false, viewPosition: viewPosition, animated: animated)
                    return
                }
                remaining -= count
            }
        }
        tableView.separatorStyle = showDividers ? .singleLine : .none
        coordinator.rowKey = rowKey
        coordinator.headerContent = headerContent
        coordinator.rowContent = rowContent
        nexaUpdateRefreshControl(
            tableView,
            controller: &coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        guard previousSectionCount == coordinator.sectionCount,
              previousSectionCounts == coordinator.sectionCounts else {
            coordinator.sectionHeaders.removeAll()
            tableView.reloadData()
            return
        }
        if let visibleRows = tableView.indexPathsForVisibleRows, !visibleRows.isEmpty {
            tableView.reconfigureRows(at: visibleRows)
        }
        if let headerContent {
            coordinator.sectionHeaders.forEach { section, header in
                header.contentConfiguration = UIHostingConfiguration {
                    headerContent(section)
                }
                .margins(.all, 0)
            }
        }
    }

    @MainActor
    final class Coordinator: NSObject, UITableViewDataSource, UITableViewDelegate {
        let sectionHeaders = NexaSectionHeaderRegistry()
        var sectionCount: Int
        var sectionCounts: [Int]
        var rowHeight: CGFloat?
        var spacing: CGFloat
        var rowKey: ((Int, Int) -> AnyHashable)?
        var refreshController: NexaFastListRefreshController?
        var headerContent: ((Int) -> HeaderContent)?
        var rowContent: (Int, Int) -> RowContent

        init(
            sectionCount: Int,
            sectionCounts: [Int],
            rowHeight: CGFloat?,
            spacing: CGFloat,
            rowKey: ((Int, Int) -> AnyHashable)?,
            headerContent: ((Int) -> HeaderContent)?,
            rowContent: @escaping (Int, Int) -> RowContent
        ) {
            self.sectionCount = sectionCount
            self.sectionCounts = sectionCounts
            self.rowHeight = rowHeight
            self.spacing = max(0, spacing)
            self.rowKey = rowKey
            self.refreshController = nil
            self.headerContent = headerContent
            self.rowContent = rowContent
            super.init()
        }

        func numberOfSections(in tableView: UITableView) -> Int {
            min(sectionCount, sectionCounts.count)
        }

        func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
            section < sectionCounts.count ? sectionCounts[section] : 0
        }

        func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
            let cell = tableView.dequeueReusableCell(withIdentifier: nexaFastSectionedCellReuseIdentifier, for: indexPath)
            cell.selectionStyle = .none
            cell.isOpaque = false
            cell.backgroundColor = .clear
            cell.backgroundConfiguration = UIBackgroundConfiguration.clear()
            cell.contentView.isOpaque = false
            cell.contentView.backgroundColor = .clear
            cell.contentConfiguration = UIHostingConfiguration {
                Group {
                    if let rowKey {
                        rowContent(indexPath.section, indexPath.row).id(rowKey(indexPath.section, indexPath.row))
                    } else {
                        rowContent(indexPath.section, indexPath.row)
                    }
                }
                .frame(minHeight: rowHeight ?? CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__), alignment: .center)
                .padding(.top, indexPath.row > 0 ? spacing / 2 : 0)
                .padding(.bottom, indexPath.row + 1 < sectionCounts[indexPath.section] ? spacing / 2 : 0)
            }
            .margins(.all, 0)
            return cell
        }

        func tableView(_ tableView: UITableView, viewForHeaderInSection section: Int) -> UIView? {
            guard let headerContent else { return nil }
            let header = tableView.dequeueReusableHeaderFooterView(
                withIdentifier: nexaFastSectionedHeaderReuseIdentifier
            ) ?? UITableViewHeaderFooterView(reuseIdentifier: nexaFastSectionedHeaderReuseIdentifier)
            header.contentConfiguration = UIHostingConfiguration {
                headerContent(section)
            }
            .margins(.all, 0)
            return header
        }

        func tableView(_ tableView: UITableView, willDisplayHeaderView view: UIView, forSection section: Int) {
            guard let header = view as? UITableViewHeaderFooterView else { return }
            sectionHeaders.register(header, section: section)
        }

        func tableView(_ tableView: UITableView, didEndDisplayingHeaderView view: UIView, forSection section: Int) {
            sectionHeaders.remove(view, section: section)
        }

        func tableView(_ tableView: UITableView, heightForHeaderInSection section: Int) -> CGFloat {
            headerContent == nil ? .leastNormalMagnitude : UITableView.automaticDimension
        }

        func tableView(_ tableView: UITableView, estimatedHeightForHeaderInSection section: Int) -> CGFloat {
            headerContent == nil ? 0 : 44
        }

        func tableView(_ tableView: UITableView, heightForRowAt indexPath: IndexPath) -> CGFloat {
            let count = indexPath.section < sectionCounts.count ? sectionCounts[indexPath.section] : 0
            let gap = (indexPath.row > 0 ? spacing / 2 : 0)
                + (indexPath.row + 1 < count ? spacing / 2 : 0)
            return rowHeight.map { $0 + gap } ?? UITableView.automaticDimension
        }
    }
}
// <nexa:list-runtime-sectioned:end>

// <nexa:list-runtime-vertical-constants:begin>
private let nexaFastListCellReuseIdentifier = "NexaFastListCell"
// <nexa:sticky-header-identifier:begin>
private let nexaFastListHeaderReuseIdentifier = "NexaFastListHeader"
// <nexa:sticky-header-identifier:end>
// <nexa:list-runtime-vertical-constants:end>

// <nexa:list-runtime-vertical:begin>
private final class NexaFastListTableView: UITableView {
    var onViewportSizeChanged: ((CGSize) -> Void)?
    private var reportedBoundsSize = CGSize.zero
    private var didReportAttachedViewport = false

    override func layoutSubviews() {
        super.layoutSubviews()
        let firstAttachedLayout = window != nil && !didReportAttachedViewport
        guard bounds.size != reportedBoundsSize || firstAttachedLayout else { return }
        reportedBoundsSize = bounds.size
        if firstAttachedLayout { didReportAttachedViewport = true }
        onViewportSizeChanged?(bounds.size)
    }
}

// <nexa:sticky-header-generics:begin>
@available(iOS 16.0, *)
private struct NexaFastList<RowContent: View, HeaderContent: View>: UIViewRepresentable {
// <nexa:sticky-header-generics:end>
// <nexa:sticky-header-generics-none:begin>
@available(iOS 16.0, *)
private struct NexaFastList<RowContent: View>: UIViewRepresentable {
// <nexa:sticky-header-generics-none:end>
    let rowCount: Int
    let rowHeight: CGFloat?
    let spacing: CGFloat
    let showDividers: Bool
    let hideHorizontalScrollbar: Bool
    let hideVerticalScrollbar: Bool
    let scrollEnabled: Bool
    let rowKey: ((Int) -> AnyHashable)?
    let reverseLayout: Bool
    let pageSnap: Bool
    let scrollPosition: Int32?
    let componentRef: NexaFastListRef?
    let restoreRevision: Int
    let onScrollPositionChanged: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    let onEndReached: (() -> Void)?
    // <nexa:scroll-events-field:begin>
    let onScroll: (() -> Void)?
    let onScrollBeginDrag: (() -> Void)?
    // <nexa:scroll-events-field:end>
    // <nexa:sticky-header-field:begin>
    let headerContent: (() -> HeaderContent)?
    // <nexa:sticky-header-field:end>
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        rowHeight: CGFloat? = nil,
        spacing: CGFloat = 0,
        showDividers: Bool = true,
        hideHorizontalScrollbar: Bool = false,
        hideVerticalScrollbar: Bool = false,
        scrollEnabled: Bool = true,
        rowKey: ((Int) -> AnyHashable)? = nil,
        reverseLayout: Bool = false,
        pageSnap: Bool = false,
        scrollPosition: Int32? = nil,
        componentRef: NexaFastListRef? = nil,
        restoreRevision: Int = 0,
        onScrollPositionChanged: ((Int) -> Void)? = nil,
        isRefreshing: Bool = false,
        onRefresh: (() -> Void)? = nil,
        onEndReached: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:begin>
        onScroll: (() -> Void)? = nil,
        onScrollBeginDrag: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:end>
        // <nexa:sticky-header-init-parameter:begin>
        headerContent: (() -> HeaderContent)? = nil,
        // <nexa:sticky-header-init-parameter:end>
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.rowHeight = rowHeight
        self.spacing = max(0, spacing)
        self.showDividers = showDividers
        self.hideHorizontalScrollbar = hideHorizontalScrollbar
        self.hideVerticalScrollbar = hideVerticalScrollbar
        self.scrollEnabled = scrollEnabled
        self.rowKey = rowKey
        self.reverseLayout = reverseLayout
        self.pageSnap = pageSnap
        self.scrollPosition = scrollPosition
        self.componentRef = componentRef
        self.restoreRevision = restoreRevision
        self.onScrollPositionChanged = onScrollPositionChanged
        self.isRefreshing = isRefreshing
        self.onRefresh = onRefresh
        self.onEndReached = onEndReached
        // <nexa:scroll-events-init-assignment:begin>
        self.onScroll = onScroll
        self.onScrollBeginDrag = onScrollBeginDrag
        // <nexa:scroll-events-init-assignment:end>
        // <nexa:sticky-header-init-assignment:begin>
        self.headerContent = headerContent
        // <nexa:sticky-header-init-assignment:end>
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(
            rowCount: rowCount,
            rowHeight: rowHeight,
            spacing: spacing,
            rowKey: rowKey,
            reverseLayout: reverseLayout,
            pageSnap: pageSnap,
            scrollPosition: scrollPosition,
            restoreRevision: restoreRevision,
            onScrollPositionChanged: onScrollPositionChanged,
            onEndReached: onEndReached,
            // <nexa:scroll-events-coordinator-argument:begin>
            onScroll: onScroll,
            onScrollBeginDrag: onScrollBeginDrag,
            // <nexa:scroll-events-coordinator-argument:end>
            // <nexa:sticky-header-coordinator-argument:begin>
            headerContent: headerContent,
            // <nexa:sticky-header-coordinator-argument:end>
            rowContent: rowContent
        )
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = NexaFastListTableView(frame: .zero, style: .plain)
        tableView.separatorStyle = showDividers ? .singleLine : .none
        let coordinator = context.coordinator
        tableView.onViewportSizeChanged = { [weak tableView, weak coordinator] size in
            DispatchQueue.main.async {
                guard let tableView, let coordinator else { return }
                coordinator.updatePageHeight(size.height, in: tableView)
                guard tableView.window != nil, !pageSnap else { return }
                guard !tableView.isTracking,
                      !tableView.isDragging,
                      !tableView.isDecelerating
                else {
                    return
                }
                // A page-backed tab can discard this UIKit view while it is
                // offscreen. Restore its controlled position on the first
                // attached layout, when UITableView has a real viewport.
                coordinator.applyScrollPosition(to: tableView)
                coordinator.seedInitialScrollPosition(in: tableView)
            }
        }
        tableView.isPagingEnabled = pageSnap
        tableView.isScrollEnabled = scrollEnabled
        tableView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        tableView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        tableView.alwaysBounceVertical = onRefresh != nil
        nexaConfigureEdgeToEdgeScrollView(tableView, environment: context.environment, reverseLayout: reverseLayout)
        if reverseLayout {
            tableView.transform = CGAffineTransform(scaleX: 1, y: -1)
        }
        tableView.dataSource = context.coordinator
        tableView.delegate = context.coordinator
        nexaUpdateRefreshControl(
            tableView,
            controller: &context.coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        tableView.register(UITableViewCell.self, forCellReuseIdentifier: nexaFastListCellReuseIdentifier)
        // <nexa:sticky-header-register:begin>
        tableView.register(
            UITableViewHeaderFooterView.self,
            forHeaderFooterViewReuseIdentifier: nexaFastListHeaderReuseIdentifier
        )
        // <nexa:sticky-header-register:end>
        if pageSnap {
            let viewportHeight = max(tableView.bounds.height, 1)
            tableView.rowHeight = viewportHeight
            tableView.estimatedRowHeight = viewportHeight
        } else if let rowHeight {
            tableView.rowHeight = rowHeight
            tableView.estimatedRowHeight = rowHeight
        } else {
            tableView.rowHeight = UITableView.automaticDimension
            tableView.estimatedRowHeight = 44
        }
        tableView.allowsSelection = false
        tableView.backgroundColor = .clear
        tableView.reloadData()
        nexaAttachFastListRef(componentRef, to: tableView, horizontal: false, reverseLayout: reverseLayout) { [weak tableView] index, animated, viewPosition in
            guard let tableView, rowCount > 0 else { return }
            let row = min(max(Int(index), 0), rowCount - 1)
            tableView.layoutIfNeeded()
            nexaScrollFastList(to: tableView.rectForRow(at: IndexPath(row: row, section: 0)), in: tableView, horizontal: false, viewPosition: reverseLayout ? 1 - viewPosition : viewPosition, animated: animated)
        }
        context.coordinator.applyScrollPosition(to: tableView)
        if reverseLayout {
            // The table has no final viewport while makeUIView is constructing
            // it. Reapply the logical initial position after SwiftUI attaches
            // and sizes the transformed UITableView, otherwise its first rows
            // can land above the visible list region.
            DispatchQueue.main.async { [weak tableView, weak coordinator] in
                guard let tableView, let coordinator else { return }
                tableView.layoutIfNeeded()
                coordinator.applyScrollPosition(to: tableView, force: true)
            }
        }
        DispatchQueue.main.async { [weak tableView, weak coordinator] in
            guard let tableView, let coordinator else { return }
            tableView.layoutIfNeeded()
            // SwiftUI page containers can create this UITableView before the
            // page has its final size. The eager scrollToRow above is ignored
            // in that case, so restore the controlled position after layout.
            if !pageSnap && !reverseLayout {
                coordinator.applyScrollPosition(to: tableView, force: true)
            }
            coordinator.seedInitialScrollPosition(in: tableView)
        }
        return tableView
    }

    func updateUIView(_ tableView: UITableView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        let previousReverseLayout = coordinator.reverseLayout
        let previousPageSnap = coordinator.pageSnap
        let previousPage = coordinator.pageSnap && coordinator.rowCount > 0
            ? coordinator.currentPage(in: tableView)
            : nil
        let previousScrollPosition = coordinator.scrollPosition
        let previousRestoreRevision = coordinator.restoreRevision
        let isScrollPositionEcho = previousScrollPosition != scrollPosition
            && coordinator.scrollPositionEchoes.consume(scrollPosition)
        if previousScrollPosition != scrollPosition, !isScrollPositionEcho {
            coordinator.scrollCallbackGeneration += 1
        }
        let previousContentHeight = tableView.contentSize.height
        let previousContentOffset = tableView.contentOffset
        let wasAtBottom = reverseLayout
            && previousContentOffset.y <= -tableView.adjustedContentInset.top + 1
        coordinator.rowCount = rowCount
        coordinator.rowHeight = rowHeight
        coordinator.spacing = max(0, spacing)
        tableView.isScrollEnabled = scrollEnabled
        tableView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        tableView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        tableView.separatorStyle = showDividers ? .singleLine : .none
        coordinator.rowKey = rowKey
        coordinator.reverseLayout = reverseLayout
        coordinator.pageSnap = pageSnap
        coordinator.scrollPosition = scrollPosition
        coordinator.restoreRevision = restoreRevision
        coordinator.onScrollPositionChanged = onScrollPositionChanged
        coordinator.onEndReached = onEndReached
        // <nexa:scroll-events-update-assignment:begin>
        coordinator.onScroll = onScroll
        coordinator.onScrollBeginDrag = onScrollBeginDrag
        // <nexa:scroll-events-update-assignment:end>
        // <nexa:sticky-header-update-assignment:begin>
        coordinator.headerContent = headerContent
        // <nexa:sticky-header-update-assignment:end>
        coordinator.rowContent = rowContent
        nexaAttachFastListRef(componentRef, to: tableView, horizontal: false, reverseLayout: reverseLayout) { [weak tableView] index, animated, viewPosition in
            guard let tableView, rowCount > 0 else { return }
            let row = min(max(Int(index), 0), rowCount - 1)
            tableView.layoutIfNeeded()
            nexaScrollFastList(to: tableView.rectForRow(at: IndexPath(row: row, section: 0)), in: tableView, horizontal: false, viewPosition: reverseLayout ? 1 - viewPosition : viewPosition, animated: animated)
        }
        nexaUpdateRefreshControl(
            tableView,
            controller: &coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        tableView.transform = reverseLayout
            ? CGAffineTransform(scaleX: 1, y: -1)
            : .identity
        tableView.isPagingEnabled = pageSnap
        nexaConfigureEdgeToEdgeScrollView(tableView, environment: context.environment, reverseLayout: reverseLayout)
        if pageSnap {
            coordinator.updatePageHeight(tableView.bounds.height, in: tableView)
        } else if previousPageSnap {
            coordinator.pageHeight = 0
            if let rowHeight {
                tableView.rowHeight = rowHeight
                tableView.estimatedRowHeight = rowHeight
            } else {
                tableView.rowHeight = UITableView.automaticDimension
                tableView.estimatedRowHeight = 44
            }
        }
        if previousPageSnap != pageSnap {
            tableView.reloadData()
            tableView.layoutIfNeeded()
            coordinator.applyScrollPosition(to: tableView)
            return
        }
        if previousReverseLayout != reverseLayout {
            tableView.reloadData()
            tableView.layoutIfNeeded()
            coordinator.applyScrollPosition(to: tableView)
            return
        }
        if previousRowCount != rowCount {
            coordinator.lastEndReachedRowCount = nil
            tableView.reloadData()
            tableView.layoutIfNeeded()
            if reverseLayout && scrollPosition == nil && rowCount > previousRowCount {
                let contentHeightGrowth = tableView.contentSize.height - previousContentHeight
                let targetOffsetY = wasAtBottom
                    ? -tableView.adjustedContentInset.top
                    : previousContentOffset.y + contentHeightGrowth
                tableView.setContentOffset(
                    CGPoint(x: previousContentOffset.x, y: targetOffsetY),
                    animated: false
                )
            } else if pageSnap {
                let targetPage = previousScrollPosition != scrollPosition
                    ? Int(scrollPosition ?? 0)
                    : (previousPage ?? Int(scrollPosition ?? 0))
                if !isScrollPositionEcho {
                    coordinator.scrollToPage(targetPage, in: tableView)
                }
            } else if !isScrollPositionEcho {
                coordinator.applyScrollPosition(to: tableView)
            }
            return
        } else if previousScrollPosition != scrollPosition, !isScrollPositionEcho {
            coordinator.applyScrollPosition(to: tableView)
        }

        // A paged container can reset a UIKit-backed list's content offset
        // while keeping its bound Nexa position unchanged. Restore only when
        // the page host explicitly reports re-entry, after its page animation
        // has completed.
        if previousRestoreRevision != restoreRevision {
            coordinator.restoreBoundScrollPositionIfNeeded(
                in: tableView,
                revision: restoreRevision
            )
        }

        // A position echo changes the SwiftUI parent state, but does not mean
        // the row data changed. Reconfiguring visible hosted cells here can
        // make UITableView adjust its offset and feed another position back.
        if !isScrollPositionEcho {
            coordinator.reconfigureVisibleRows(in: tableView, preservingViewport: true)
        }
        // <nexa:sticky-header-update-view:begin>
        if let header = tableView.headerView(forSection: 0), let headerContent {
            header.contentConfiguration = UIHostingConfiguration {
                headerContent()
            }
            .margins(.all, 0)
        }
        // <nexa:sticky-header-update-view:end>
    }

    final class Coordinator: NSObject, UITableViewDataSource, UITableViewDelegate {
        var rowCount: Int
        var rowHeight: CGFloat?
        var spacing: CGFloat
        var rowKey: ((Int) -> AnyHashable)?
        var reverseLayout: Bool
        var pageSnap: Bool
        var pageHeight: CGFloat
        var scrollPosition: Int32?
        var restoreRevision: Int
        var onScrollPositionChanged: ((Int) -> Void)?
        var refreshController: NexaFastListRefreshController?
        var onEndReached: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:begin>
        var onScroll: (() -> Void)?
        var onScrollBeginDrag: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:end>
        // <nexa:sticky-header-coordinator-field:begin>
        var headerContent: (() -> HeaderContent)?
        // <nexa:sticky-header-coordinator-field:end>
        var lastEndReachedRowCount: Int?
        var lastReportedScrollPosition: Int?
        var isApplyingProgrammaticScrollPosition = false
        var isIgnoringProgrammaticScrollCallbacks = false
        var scrollCallbackGeneration = 0
        var isReadyToReportScroll = false
        var needsVisibleRowReconfiguration = false
        var isApplyingVisibleRowReconfiguration = false
        var visibleRowReconfigurationGeneration = 0
        var scrollPositionEchoes = NexaFastListScrollPositionEchoes()
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            rowHeight: CGFloat?,
            spacing: CGFloat,
            rowKey: ((Int) -> AnyHashable)?,
            reverseLayout: Bool,
            pageSnap: Bool,
            scrollPosition: Int32?,
            restoreRevision: Int,
            onScrollPositionChanged: ((Int) -> Void)?,
            onEndReached: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:begin>
            onScroll: (() -> Void)?,
            onScrollBeginDrag: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:end>
            // <nexa:sticky-header-coordinator-init-parameter:begin>
            headerContent: (() -> HeaderContent)?,
            // <nexa:sticky-header-coordinator-init-parameter:end>
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.rowHeight = rowHeight
            self.spacing = max(0, spacing)
            self.rowKey = rowKey
            self.reverseLayout = reverseLayout
            self.pageSnap = pageSnap
            self.pageHeight = 0
            self.scrollPosition = scrollPosition
                self.restoreRevision = restoreRevision
            self.onScrollPositionChanged = onScrollPositionChanged
            self.refreshController = nil
            self.onEndReached = onEndReached
            // <nexa:scroll-events-coordinator-init-assignment:begin>
            self.onScroll = onScroll
            self.onScrollBeginDrag = onScrollBeginDrag
            // <nexa:scroll-events-coordinator-init-assignment:end>
            // <nexa:sticky-header-coordinator-init-assignment:begin>
            self.headerContent = headerContent
            // <nexa:sticky-header-coordinator-init-assignment:end>
            self.lastEndReachedRowCount = nil
            self.lastReportedScrollPosition = nil
            self.rowContent = rowContent
            super.init()
        }

        func updatePageHeight(_ height: CGFloat, in tableView: UITableView) {
            guard pageSnap, height > 0, abs(height - pageHeight) > 0.5 else { return }
            let page = pageHeight > 0
                ? currentPage(in: tableView)
                : min(max(Int(scrollPosition ?? 0), 0), max(0, rowCount - 1))
            pageHeight = height
            tableView.rowHeight = height
            tableView.estimatedRowHeight = height
            tableView.reloadData()
            tableView.layoutIfNeeded()
            scrollToPage(page, in: tableView)
        }

        func currentPage(in tableView: UITableView) -> Int {
            guard pageHeight > 0, rowCount > 0 else { return 0 }
            let position = (tableView.contentOffset.y + tableView.adjustedContentInset.top) / pageHeight
            return min(max(Int(position.rounded()), 0), rowCount - 1)
        }

        func leadingVisibleRowPosition(in tableView: UITableView) -> Int? {
            let visibleRows = tableView.indexPathsForVisibleRows ?? []
            guard !reverseLayout,
                  let pinnedHeader = tableView.headerView(forSection: 0)
            else {
                // Project lazily: scroll callbacks only need one extremum, not a temporary array.
                let positions = visibleRows.lazy.map(\.row)
                return reverseLayout ? positions.max() : positions.min()
            }

            // UITableView can still report a cell visible while a pinned
            // section header is covering it. Compose's LazyColumn starts its
            // visible-item range below the sticky header, so exclude cells
            // whose leading edge is still behind that header on iOS as well.
            let visibleTop = max(
                tableView.bounds.minY + tableView.adjustedContentInset.top,
                pinnedHeader.frame.maxY
            )
            let unobscuredPositions = visibleRows.lazy.compactMap { indexPath in
                tableView.rectForRow(at: indexPath).minY >= visibleTop - 1
                    ? indexPath.row
                    : nil
            }
            return unobscuredPositions.min()
        }

        func seedInitialScrollPosition(in tableView: UITableView) {
            let visiblePosition = leadingVisibleRowPosition(in: tableView)
            if let visiblePosition {
                lastReportedScrollPosition = reverseLayout
                    ? rowCount - 1 - visiblePosition
                    : visiblePosition
            }
            isReadyToReportScroll = true
            if pageSnap {
                reportSettledPage(in: tableView)
            } else {
                reportEndReachedIfVisible(in: tableView)
            }
        }

        private func reportEndReachedIfVisible(in tableView: UITableView) {
            guard let onEndReached,
                  rowCount > 0,
                  lastEndReachedRowCount != rowCount,
                  tableView.indexPathsForVisibleRows?.contains(where: { $0.row >= rowCount - 1 }) == true
            else {
                return
            }
            lastEndReachedRowCount = rowCount
            DispatchQueue.main.async { onEndReached() }
        }

        // __NEXA_RECONFIGURE_TABLE__

        private func seedVisibleScrollPosition(in tableView: UITableView) {
            guard !pageSnap else { return }
            let visiblePosition = leadingVisibleRowPosition(in: tableView)
            if let visiblePosition {
                lastReportedScrollPosition = reverseLayout
                    ? rowCount - 1 - visiblePosition
                    : visiblePosition
            }
        }

        private func flushVisibleRowReconfigurationIfNeeded(in scrollView: UIScrollView) {
            guard needsVisibleRowReconfiguration,
                  let tableView = scrollView as? UITableView
            else {
                return
            }
            reconfigureVisibleRows(in: tableView)
        }

        func scrollToPage(_ requestedPage: Int, in tableView: UITableView) {
            guard pageSnap, pageHeight > 0, rowCount > 0 else { return }
            let page = min(max(requestedPage, 0), rowCount - 1)
            let targetOffsetY = CGFloat(page) * pageHeight - tableView.adjustedContentInset.top
            guard abs(tableView.contentOffset.y - targetOffsetY) > 1 else { return }
            tableView.scrollToRow(at: IndexPath(row: page, section: 0), at: .top, animated: false)
        }

        func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
            rowCount
        }

        func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
            let cell = tableView.dequeueReusableCell(withIdentifier: nexaFastListCellReuseIdentifier, for: indexPath)
            cell.selectionStyle = .none
            cell.isOpaque = false
            cell.backgroundColor = .clear
            cell.backgroundConfiguration = UIBackgroundConfiguration.clear()
            cell.contentView.isOpaque = false
            cell.contentView.backgroundColor = .clear
            cell.transform = reverseLayout
                ? CGAffineTransform(scaleX: 1, y: -1)
                : .identity
            // The table view is flipped for reverseLayout, so its data-source
            // index already provides the reversed visual order.
            let row = indexPath.row
            let rowHeight = self.rowHeight
            cell.contentConfiguration = UIHostingConfiguration {
                Group {
                    if let rowKey {
                        rowContent(row).id(rowKey(row))
                    } else {
                        rowContent(row)
                    }
                }
                .frame(
                    width: pageSnap ? tableView.bounds.width : nil,
                    height: pageSnap ? pageHeight : nil,
                    alignment: .topLeading
                )
                .frame(
                    minHeight: pageSnap ? nil : (rowHeight ?? CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__)),
                    alignment: .center
                )
                .padding(.top, row > 0 ? spacing / 2 : 0)
                .padding(.bottom, row + 1 < rowCount ? spacing / 2 : 0)
            }
            .margins(.all, 0)
            return cell
        }

        func tableView(_ tableView: UITableView, heightForRowAt indexPath: IndexPath) -> CGFloat {
            if pageSnap {
                return pageHeight > 0 ? pageHeight : max(tableView.bounds.height, 1)
            }
            let row = indexPath.row
            let gap = (row > 0 ? spacing / 2 : 0)
                + (row + 1 < rowCount ? spacing / 2 : 0)
            return rowHeight.map { $0 + gap } ?? UITableView.automaticDimension
        }

        func tableView(_ tableView: UITableView, estimatedHeightForRowAt indexPath: IndexPath) -> CGFloat {
            if pageSnap {
                return pageHeight > 0 ? pageHeight : max(tableView.bounds.height, 1)
            }
            guard let rowHeight else { return tableView.estimatedRowHeight }
            let row = indexPath.row
            let gap = (row > 0 ? spacing / 2 : 0)
                + (row + 1 < rowCount ? spacing / 2 : 0)
            return rowHeight + gap
        }

        // <nexa:sticky-header-delegate-methods:begin>
        func tableView(_ tableView: UITableView, viewForHeaderInSection section: Int) -> UIView? {
            guard let headerContent else { return nil }
            let header = tableView.dequeueReusableHeaderFooterView(
                withIdentifier: nexaFastListHeaderReuseIdentifier
            ) ?? UITableViewHeaderFooterView(reuseIdentifier: nexaFastListHeaderReuseIdentifier)
            header.contentConfiguration = UIHostingConfiguration {
                headerContent()
            }
            .margins(.all, 0)
            return header
        }

        func tableView(_ tableView: UITableView, heightForHeaderInSection section: Int) -> CGFloat {
            headerContent == nil ? .leastNormalMagnitude : UITableView.automaticDimension
        }

        func tableView(_ tableView: UITableView, estimatedHeightForHeaderInSection section: Int) -> CGFloat {
            headerContent == nil ? 0 : 44
        }
        // <nexa:sticky-header-delegate-methods:end>

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            if isApplyingProgrammaticScrollPosition { return }
            guard !pageSnap else { return }
            let isOverscrolling = nexaFastListIsOverscrolling(scrollView, horizontal: false)
            if isApplyingVisibleRowReconfiguration {
                if let tableView = scrollView as? UITableView {
                    seedVisibleScrollPosition(in: tableView)
                }
                return
            }
            let visiblePosition = (scrollView as? UITableView)
                .flatMap { leadingVisibleRowPosition(in: $0) }
            let logicalPosition = visiblePosition.map { position in
                reverseLayout ? rowCount - 1 - position : position
            }
            // <nexa:scroll-events-condition-active:begin>
            if !isOverscrolling,
               !isIgnoringProgrammaticScrollCallbacks,
               (onScrollPositionChanged != nil || onScroll != nil),
               let position = logicalPosition,
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-active:end>
            // <nexa:scroll-events-condition-inactive:begin>
            if !isOverscrolling,
               !isIgnoringProgrammaticScrollCallbacks,
               onScrollPositionChanged != nil,
               let position = logicalPosition,
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-inactive:end>
            {
                lastReportedScrollPosition = position
                if isReadyToReportScroll {
                    let positionCallback = onScrollPositionChanged
                    // <nexa:scroll-events-callback:begin>
                    let scrollCallback = onScroll
                    // <nexa:scroll-events-callback:end>
                    let callbackGeneration = scrollCallbackGeneration
                    DispatchQueue.main.async { [weak self] in
                        guard let self else { return }
                        guard self.scrollCallbackGeneration == callbackGeneration else { return }
                        // A newer visible row can arrive before this main-queue
                        // callback runs. Ignore stale notifications entirely so
                        // their onScroll actions cannot amplify layout feedback.
                        guard self.lastReportedScrollPosition == position else { return }
                        if let positionCallback,
                           self.scrollPosition != Int32(position) {
                            self.scrollPositionEchoes.record(position)
                            positionCallback(position)
                        }
                        // <nexa:scroll-events-callback:begin>
                        scrollCallback?()
                        // <nexa:scroll-events-callback:end>
                    }
                }
            }
            if let tableView = scrollView as? UITableView {
                reportEndReachedIfVisible(in: tableView)
            }
        }

        func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
            isIgnoringProgrammaticScrollCallbacks = false
            // <nexa:scroll-events-callback:begin>
            onScrollBeginDrag?()
            // <nexa:scroll-events-callback:end>
            if isApplyingVisibleRowReconfiguration {
                visibleRowReconfigurationGeneration += 1
            }
            isApplyingVisibleRowReconfiguration = false
        }

        func scrollViewWillEndDragging(
            _ scrollView: UIScrollView,
            withVelocity velocity: CGPoint,
            targetContentOffset: UnsafeMutablePointer<CGPoint>
        ) {
            guard pageSnap,
                  rowCount > 0,
                  pageHeight > 0,
                  let tableView = scrollView as? UITableView
            else {
                return
            }

            let current = min(
                max(lastReportedScrollPosition ?? currentPage(in: tableView), 0),
                rowCount - 1
            )
            let proposedOffset = targetContentOffset.pointee.y + tableView.adjustedContentInset.top
            var proposed = Int((proposedOffset / pageHeight).rounded())
            if proposed == current && abs(velocity.y) > 0.2 {
                proposed += velocity.y > 0 ? 1 : -1
            }
            let target = min(max(proposed, current - 1), current + 1)
            let boundedTarget = min(max(target, 0), rowCount - 1)
            targetContentOffset.pointee.y = CGFloat(boundedTarget) * pageHeight
                - tableView.adjustedContentInset.top
        }

        func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
            flushVisibleRowReconfigurationIfNeeded(in: scrollView)
            reportSettledPage(in: scrollView)
        }

        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate {
                flushVisibleRowReconfigurationIfNeeded(in: scrollView)
                reportSettledPage(in: scrollView)
            }
        }

        func scrollViewDidEndScrollingAnimation(_ scrollView: UIScrollView) {
            flushVisibleRowReconfigurationIfNeeded(in: scrollView)
            reportSettledPage(in: scrollView)
        }

        private func reportSettledPage(in scrollView: UIScrollView) {
            guard pageSnap, rowCount > 0, let tableView = scrollView as? UITableView else { return }
            let page = currentPage(in: tableView)
            var reportedPage: Int?
            if lastReportedScrollPosition != page {
                lastReportedScrollPosition = page
                reportedPage = page
            }
            var didReachEnd = false
            if page == rowCount - 1, lastEndReachedRowCount != rowCount {
                lastEndReachedRowCount = rowCount
                didReachEnd = true
            }
            guard reportedPage != nil || didReachEnd else { return }

            let positionCallback = onScrollPositionChanged
            let endReachedCallback = onEndReached
            // <nexa:scroll-events-callback:begin>
            let scrollCallback = onScroll
            // <nexa:scroll-events-callback:end>
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                if let reportedPage {
                    if self.lastReportedScrollPosition == reportedPage,
                       let positionCallback,
                       self.scrollPosition != Int32(reportedPage) {
                        self.scrollPositionEchoes.record(reportedPage)
                        positionCallback(reportedPage)
                    }
                // <nexa:scroll-events-callback:begin>
                    scrollCallback?()
                // <nexa:scroll-events-callback:end>
                }
                if didReachEnd { endReachedCallback?() }
            }
        }

        func applyScrollPosition(to tableView: UITableView, force: Bool = false) {
            guard let scrollPosition, rowCount > 0 else { return }
            guard force || (!tableView.isTracking && !tableView.isDragging && !tableView.isDecelerating) else {
                return
            }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)
            let visualTarget = reverseLayout ? rowCount - 1 - target : target
            let visiblePosition = leadingVisibleRowPosition(in: tableView)
            guard force || (visiblePosition ?? -1) != visualTarget else {
                return
            }
            // Let the scroll delegate report the changed visible row so that
            // programmatic positions use the same callback path as user scrolls.
            // Ignore the delegate's intermediate visible rows during this
            // non-animated scroll; report the settled target exactly once below.
            isApplyingProgrammaticScrollPosition = true
            isIgnoringProgrammaticScrollCallbacks = true
            tableView.scrollToRow(
                at: IndexPath(row: visualTarget, section: 0),
                at: reverseLayout ? .bottom : .top,
                animated: false
            )
            tableView.layoutIfNeeded()
            isApplyingProgrammaticScrollPosition = false
            if let visiblePosition = leadingVisibleRowPosition(in: tableView) {
                let logicalPosition = reverseLayout ? rowCount - 1 - visiblePosition : visiblePosition
                reportProgrammaticScrollPosition(logicalPosition)
            }
            reportEndReachedIfVisible(in: tableView)
        }

        private func reportProgrammaticScrollPosition(_ position: Int) {
            guard lastReportedScrollPosition != position else { return }
            lastReportedScrollPosition = position
            guard isReadyToReportScroll else { return }
            let positionCallback = onScrollPositionChanged
            // <nexa:scroll-events-callback:begin>
            let scrollCallback = onScroll
            // <nexa:scroll-events-callback:end>
            let callbackGeneration = scrollCallbackGeneration
            DispatchQueue.main.async { [weak self] in
                guard let self,
                      self.scrollCallbackGeneration == callbackGeneration,
                      self.lastReportedScrollPosition == position else { return }
                if let positionCallback, self.scrollPosition != Int32(position) {
                    self.scrollPositionEchoes.record(position)
                    positionCallback(position)
                }
                // <nexa:scroll-events-callback:begin>
                scrollCallback?()
                // <nexa:scroll-events-callback:end>
            }
        }

        func restoreBoundScrollPositionIfNeeded(in tableView: UITableView, revision: Int) {
            guard let scrollPosition,
                  rowCount > 0,
                  !tableView.isTracking,
                  !tableView.isDragging,
                  !tableView.isDecelerating
            else {
                return
            }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)

            DispatchQueue.main.asyncAfter(deadline: .now() + 0.45) { [weak self, weak tableView] in
                guard let self,
                      let tableView,
                      tableView.window != nil,
                      self.restoreRevision == revision,
                      self.scrollPosition == scrollPosition,
                      !tableView.isTracking,
                      !tableView.isDragging,
                      !tableView.isDecelerating
                else {
                    return
                }
                tableView.layoutIfNeeded()
                let visualTarget = self.reverseLayout ? self.rowCount - 1 - target : target
                let visiblePosition = self.leadingVisibleRowPosition(in: tableView)
                guard visiblePosition != visualTarget else { return }
                self.applyScrollPosition(to: tableView, force: true)
            }
        }
    }
}
// <nexa:list-runtime-vertical:end>

// <nexa:list-runtime-horizontal:begin>
private let nexaFastHorizontalListCellReuseIdentifier = "NexaFastHorizontalListCell"

@available(iOS 16.0, *)
@MainActor
private func nexaFastHorizontalEstimatedItemSize<RowContent: View>(
    rowCount: Int,
    itemExtent: CGFloat?,
    rowContent: (Int) -> RowContent
) -> CGSize {
    let minimumHeight = CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__)
    guard rowCount > 0 else {
        let height = itemExtent ?? minimumHeight
        return CGSize(width: height, height: height)
    }

    let estimatedSize = UIHostingController(
        rootView: rowContent(0)
            .frame(
                minHeight: itemExtent ?? minimumHeight,
                maxHeight: itemExtent,
                alignment: .center
            )
            .fixedSize(horizontal: true, vertical: false)
    )
    .sizeThatFits(in: CGSize(width: 10_000, height: 10_000))
    return CGSize(
        width: max(1, estimatedSize.width),
        height: max(itemExtent ?? minimumHeight, estimatedSize.height)
    )
}

@available(iOS 16.0, *)
private final class NexaFastHorizontalListCell: UICollectionViewCell {
    var itemExtent: CGFloat?

    override func preferredLayoutAttributesFitting(
        _ layoutAttributes: UICollectionViewLayoutAttributes
    ) -> UICollectionViewLayoutAttributes {
        let attributes = super.preferredLayoutAttributesFitting(layoutAttributes)
        let fittedSize = contentView.systemLayoutSizeFitting(
            CGSize(
                width: UIView.layoutFittingCompressedSize.width,
                height: itemExtent ?? UIView.layoutFittingCompressedSize.height
            ),
            withHorizontalFittingPriority: .fittingSizeLevel,
            verticalFittingPriority: itemExtent == nil ? .fittingSizeLevel : .required
        )
        attributes.size = CGSize(
            width: max(1, fittedSize.width),
            height: itemExtent ?? max(1, fittedSize.height)
        )
        return attributes
    }
}

@available(iOS 16.0, *)
private struct NexaFastHorizontalList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let itemExtent: CGFloat?
    let spacing: CGFloat
    let hideHorizontalScrollbar: Bool
    let hideVerticalScrollbar: Bool
    let scrollEnabled: Bool
    let rowKey: ((Int) -> AnyHashable)?
    let scrollPosition: Int32?
    let componentRef: NexaFastListRef?
    let onScrollPositionChanged: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    let onEndReached: (() -> Void)?
    // <nexa:scroll-events-field:begin>
    let onScroll: (() -> Void)?
    // <nexa:scroll-events-field:end>
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        itemExtent: CGFloat? = nil,
        spacing: CGFloat = 0,
        hideHorizontalScrollbar: Bool = false,
        hideVerticalScrollbar: Bool = false,
        scrollEnabled: Bool = true,
        rowKey: ((Int) -> AnyHashable)? = nil,
        scrollPosition: Int32? = nil,
        componentRef: NexaFastListRef? = nil,
        onScrollPositionChanged: ((Int) -> Void)? = nil,
        isRefreshing: Bool = false,
        onRefresh: (() -> Void)? = nil,
        onEndReached: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:begin>
        onScroll: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:end>
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.itemExtent = itemExtent
        self.spacing = max(0, spacing)
        self.hideHorizontalScrollbar = hideHorizontalScrollbar
        self.hideVerticalScrollbar = hideVerticalScrollbar
        self.scrollEnabled = scrollEnabled
        self.rowKey = rowKey
        self.scrollPosition = scrollPosition
        self.componentRef = componentRef
        self.onScrollPositionChanged = onScrollPositionChanged
        self.isRefreshing = isRefreshing
        self.onRefresh = onRefresh
        self.onEndReached = onEndReached
        // <nexa:scroll-events-init-assignment:begin>
        self.onScroll = onScroll
        // <nexa:scroll-events-init-assignment:end>
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(
            rowCount: rowCount,
            itemExtent: itemExtent,
            spacing: spacing,
            rowKey: rowKey,
            scrollPosition: scrollPosition,
            onScrollPositionChanged: onScrollPositionChanged,
            onEndReached: onEndReached,
            // <nexa:scroll-events-coordinator-argument:begin>
            onScroll: onScroll,
            // <nexa:scroll-events-coordinator-argument:end>
            rowContent: rowContent
        )
    }

    func makeUIView(context: Context) -> UICollectionView {
        let layout = UICollectionViewFlowLayout()
        layout.scrollDirection = .horizontal
        layout.estimatedItemSize = nexaFastHorizontalEstimatedItemSize(
            rowCount: rowCount,
            itemExtent: itemExtent,
            rowContent: rowContent
        )
        layout.minimumLineSpacing = spacing
        layout.minimumInteritemSpacing = 0
        let collectionView = UICollectionView(frame: .zero, collectionViewLayout: layout)
        nexaConfigureEdgeToEdgeScrollView(collectionView, environment: context.environment)
        collectionView.dataSource = context.coordinator
        collectionView.delegate = context.coordinator
        nexaUpdateRefreshControl(
            collectionView,
            controller: &context.coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        collectionView.register(
            NexaFastHorizontalListCell.self,
            forCellWithReuseIdentifier: nexaFastHorizontalListCellReuseIdentifier
        )
        collectionView.alwaysBounceHorizontal = onRefresh != nil
        collectionView.isScrollEnabled = scrollEnabled
        collectionView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        collectionView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        collectionView.backgroundColor = .clear
        collectionView.reloadData()
        nexaAttachFastListRef(componentRef, to: collectionView, horizontal: true) { [weak collectionView] index, animated, viewPosition in
            guard let collectionView, rowCount > 0 else { return }
            let item = min(max(Int(index), 0), rowCount - 1)
            collectionView.layoutIfNeeded()
            let path = IndexPath(item: item, section: 0)
            let frame = collectionView.collectionViewLayout.layoutAttributesForItem(at: path)?.frame ?? .zero
            nexaScrollFastList(to: frame, in: collectionView, horizontal: true, viewPosition: viewPosition, animated: animated)
        }
        context.coordinator.applyScrollPosition(to: collectionView)
        DispatchQueue.main.async { [weak collectionView, weak coordinator = context.coordinator] in
            guard let collectionView, let coordinator else { return }
            coordinator.seedInitialScrollPosition(in: collectionView)
        }
        return collectionView
    }

    func sizeThatFits(
        _ proposal: ProposedViewSize,
        uiView: UICollectionView,
        context: Context
    ) -> CGSize? {
        guard rowCount > 0 else {
            return CGSize(width: proposal.width ?? uiView.bounds.width, height: 0)
        }

        let estimatedHeight = itemExtent
            ?? (uiView.collectionViewLayout as? UICollectionViewFlowLayout)?.estimatedItemSize.height
            ?? CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__)
        let layoutHeight = uiView.collectionViewLayout.collectionViewContentSize.height
        let width = proposal.width ?? max(uiView.bounds.width, 320)
        return CGSize(width: width, height: max(estimatedHeight, layoutHeight))
    }

    func updateUIView(_ collectionView: UICollectionView, context: Context) {
        nexaConfigureEdgeToEdgeScrollView(collectionView, environment: context.environment)
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        let previousItemExtent = coordinator.itemExtent
        let previousScrollPosition = coordinator.scrollPosition
        let isScrollPositionEcho = previousScrollPosition != scrollPosition
            && coordinator.scrollPositionEchoes.consume(scrollPosition)
        if previousScrollPosition != scrollPosition, !isScrollPositionEcho {
            coordinator.scrollCallbackGeneration += 1
        }
        coordinator.rowCount = rowCount
        coordinator.itemExtent = itemExtent
        coordinator.spacing = max(0, spacing)
        collectionView.isScrollEnabled = scrollEnabled
        collectionView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        collectionView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        if let layout = collectionView.collectionViewLayout as? UICollectionViewFlowLayout {
            var shouldInvalidateLayout = false
            if layout.minimumLineSpacing != spacing {
                layout.minimumLineSpacing = spacing
                shouldInvalidateLayout = true
            }
            if previousItemExtent != itemExtent {
                layout.estimatedItemSize = nexaFastHorizontalEstimatedItemSize(
                    rowCount: rowCount,
                    itemExtent: itemExtent,
                    rowContent: rowContent
                )
                shouldInvalidateLayout = true
            }
            if shouldInvalidateLayout {
                layout.invalidateLayout()
            }
        }
        coordinator.rowKey = rowKey
        coordinator.scrollPosition = scrollPosition
        nexaAttachFastListRef(componentRef, to: collectionView, horizontal: true) { [weak collectionView] index, animated, viewPosition in
            guard let collectionView, rowCount > 0 else { return }
            let item = min(max(Int(index), 0), rowCount - 1)
            collectionView.layoutIfNeeded()
            let path = IndexPath(item: item, section: 0)
            let frame = collectionView.collectionViewLayout.layoutAttributesForItem(at: path)?.frame ?? .zero
            nexaScrollFastList(to: frame, in: collectionView, horizontal: true, viewPosition: viewPosition, animated: animated)
        }
        coordinator.onScrollPositionChanged = onScrollPositionChanged
        coordinator.onEndReached = onEndReached
        // <nexa:scroll-events-update-assignment:begin>
        coordinator.onScroll = onScroll
        // <nexa:scroll-events-update-assignment:end>
        coordinator.rowContent = rowContent
        nexaUpdateRefreshControl(
            collectionView,
            controller: &coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        if previousRowCount != rowCount {
            coordinator.lastEndReachedRowCount = nil
            if !isScrollPositionEcho { coordinator.applyScrollPosition(to: collectionView) }
            collectionView.reloadData()
            collectionView.layoutIfNeeded()
            return
        } else if previousScrollPosition != scrollPosition, !isScrollPositionEcho {
            coordinator.applyScrollPosition(to: collectionView)
        }
        coordinator.reconfigureVisibleItems(in: collectionView, preservingViewport: true)
        collectionView.layoutIfNeeded()
    }

    final class Coordinator: NSObject, UICollectionViewDataSource, UICollectionViewDelegate {
        var rowCount: Int
        var itemExtent: CGFloat?
        var spacing: CGFloat
        var rowKey: ((Int) -> AnyHashable)?
        var scrollPosition: Int32?
        var onScrollPositionChanged: ((Int) -> Void)?
        var refreshController: NexaFastListRefreshController?
        var onEndReached: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:begin>
        var onScroll: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:end>
        var lastEndReachedRowCount: Int?
        var lastReportedScrollPosition: Int?
        var isApplyingProgrammaticScrollPosition = false
        var isIgnoringProgrammaticScrollCallbacks = false
        var scrollCallbackGeneration = 0
        var isReadyToReportScroll = false
        var needsVisibleItemReconfiguration = false
        var isApplyingVisibleItemReconfiguration = false
        var visibleItemReconfigurationGeneration = 0
        var scrollPositionEchoes = NexaFastListScrollPositionEchoes()
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            itemExtent: CGFloat?,
            spacing: CGFloat,
            rowKey: ((Int) -> AnyHashable)?,
            scrollPosition: Int32?,
            onScrollPositionChanged: ((Int) -> Void)?,
            onEndReached: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:begin>
            onScroll: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:end>
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.itemExtent = itemExtent
            self.spacing = max(0, spacing)
            self.rowKey = rowKey
            self.scrollPosition = scrollPosition
                self.onScrollPositionChanged = onScrollPositionChanged
            self.refreshController = nil
            self.onEndReached = onEndReached
            // <nexa:scroll-events-coordinator-init-assignment:begin>
            self.onScroll = onScroll
            // <nexa:scroll-events-coordinator-init-assignment:end>
            self.lastEndReachedRowCount = nil
            self.lastReportedScrollPosition = nil
            self.rowContent = rowContent
            super.init()
        }

        func collectionView(
            _ collectionView: UICollectionView,
            numberOfItemsInSection section: Int
        ) -> Int {
            rowCount
        }

        func seedInitialScrollPosition(in collectionView: UICollectionView) {
            collectionView.layoutIfNeeded()
            lastReportedScrollPosition = collectionView.indexPathsForVisibleItems.lazy.map(\.item).min()
            isReadyToReportScroll = true
            reportEndReachedIfVisible(in: collectionView)
        }

        // __NEXA_RECONFIGURE_HORIZONTAL__

        private func seedVisibleScrollPosition(in collectionView: UICollectionView) {
            lastReportedScrollPosition = collectionView.indexPathsForVisibleItems.lazy.map(\.item).min()
        }

        private func flushVisibleItemReconfigurationIfNeeded(in scrollView: UIScrollView) {
            guard needsVisibleItemReconfiguration,
                  let collectionView = scrollView as? UICollectionView
            else {
                return
            }
            reconfigureVisibleItems(in: collectionView)
        }

        private func reportEndReachedIfVisible(in collectionView: UICollectionView) {
            guard let onEndReached,
                  rowCount > 0,
                  lastEndReachedRowCount != rowCount,
                  collectionView.indexPathsForVisibleItems.contains(where: { $0.item >= rowCount - 1 })
            else {
                return
            }
            lastEndReachedRowCount = rowCount
            DispatchQueue.main.async { onEndReached() }
        }

        func collectionView(
            _ collectionView: UICollectionView,
            cellForItemAt indexPath: IndexPath
        ) -> UICollectionViewCell {
            let cell = collectionView.dequeueReusableCell(
                withReuseIdentifier: nexaFastHorizontalListCellReuseIdentifier,
                for: indexPath
            )
            (cell as? NexaFastHorizontalListCell)?.itemExtent = itemExtent
            cell.contentConfiguration = UIHostingConfiguration {
                if let rowKey {
                    rowContent(indexPath.item)
                        .id(rowKey(indexPath.item))
                        .frame(
                            minHeight: itemExtent ?? CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__),
                            maxHeight: itemExtent,
                            alignment: .center
                        )
                        .fixedSize(horizontal: true, vertical: false)
                } else {
                    rowContent(indexPath.item)
                        .frame(
                            minHeight: itemExtent ?? CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__),
                            maxHeight: itemExtent,
                            alignment: .center
                        )
                        .fixedSize(horizontal: true, vertical: false)
                }
            }
            .margins(.all, 0)
            return cell
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            if isApplyingProgrammaticScrollPosition { return }
            let isOverscrolling = nexaFastListIsOverscrolling(scrollView, horizontal: true)
            if isApplyingVisibleItemReconfiguration {
                if let collectionView = scrollView as? UICollectionView {
                    seedVisibleScrollPosition(in: collectionView)
                }
                return
            }
            // <nexa:scroll-events-condition-active:begin>
            if !isOverscrolling,
               !isIgnoringProgrammaticScrollCallbacks,
               (onScrollPositionChanged != nil || onScroll != nil),
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.lazy.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-active:end>
            // <nexa:scroll-events-condition-inactive:begin>
            if !isOverscrolling,
               !isIgnoringProgrammaticScrollCallbacks,
               onScrollPositionChanged != nil,
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.lazy.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-inactive:end>
            {
                lastReportedScrollPosition = position
                if isReadyToReportScroll {
                    let positionCallback = onScrollPositionChanged
                    // <nexa:scroll-events-callback:begin>
                    let scrollCallback = onScroll
                    // <nexa:scroll-events-callback:end>
                    let callbackGeneration = scrollCallbackGeneration
                    DispatchQueue.main.async { [weak self] in
                        guard let self else { return }
                        guard self.scrollCallbackGeneration == callbackGeneration else { return }
                        guard self.lastReportedScrollPosition == position else { return }
                        if let positionCallback,
                           self.scrollPosition != Int32(position) {
                            self.scrollPositionEchoes.record(position)
                            positionCallback(position)
                        }
                        // <nexa:scroll-events-callback:begin>
                        scrollCallback?()
                        // <nexa:scroll-events-callback:end>
                    }
                }
            }
            if let collectionView = scrollView as? UICollectionView {
                reportEndReachedIfVisible(in: collectionView)
            }
        }

        func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
            isIgnoringProgrammaticScrollCallbacks = false
            if isApplyingVisibleItemReconfiguration {
                visibleItemReconfigurationGeneration += 1
            }
            isApplyingVisibleItemReconfiguration = false
        }

        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate { flushVisibleItemReconfigurationIfNeeded(in: scrollView) }
        }

        func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
            flushVisibleItemReconfigurationIfNeeded(in: scrollView)
        }

        func scrollViewDidEndScrollingAnimation(_ scrollView: UIScrollView) {
            flushVisibleItemReconfigurationIfNeeded(in: scrollView)
        }

        func applyScrollPosition(to collectionView: UICollectionView) {
            guard let scrollPosition, rowCount > 0 else { return }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)
            guard (collectionView.indexPathsForVisibleItems.lazy.map(\.item).min() ?? -1) != target else {
                return
            }
            // Let scrollViewDidScroll report the new leading item.
            // Ignore the delegate's intermediate visible items during this
            // non-animated scroll; report the settled target exactly once below.
            isApplyingProgrammaticScrollPosition = true
            isIgnoringProgrammaticScrollCallbacks = true
            collectionView.scrollToItem(
                at: IndexPath(item: target, section: 0),
                at: .left,
                animated: false
            )
            collectionView.layoutIfNeeded()
            isApplyingProgrammaticScrollPosition = false
            if let position = collectionView.indexPathsForVisibleItems.lazy.map(\.item).min() {
                reportProgrammaticScrollPosition(position)
            }
            reportEndReachedIfVisible(in: collectionView)
        }

        private func reportProgrammaticScrollPosition(_ position: Int) {
            let boundPositionChanged = onScrollPositionChanged != nil
                && scrollPosition != Int32(position)
            guard lastReportedScrollPosition != position || boundPositionChanged else { return }
            lastReportedScrollPosition = position
            guard isReadyToReportScroll else { return }
            let positionCallback = onScrollPositionChanged
            // <nexa:scroll-events-callback:begin>
            let scrollCallback = onScroll
            // <nexa:scroll-events-callback:end>
            let callbackGeneration = scrollCallbackGeneration
            DispatchQueue.main.async { [weak self] in
                guard let self,
                      self.scrollCallbackGeneration == callbackGeneration,
                      self.lastReportedScrollPosition == position else { return }
                if let positionCallback, self.scrollPosition != Int32(position) {
                    self.scrollPositionEchoes.record(position)
                    positionCallback(position)
                }
                // <nexa:scroll-events-callback:begin>
                scrollCallback?()
                // <nexa:scroll-events-callback:end>
            }
        }
    }
}
// <nexa:list-runtime-horizontal:end>

// <nexa:list-runtime-grid:begin>
private let nexaFastGridListCellReuseIdentifier = "NexaFastGridListCell"

@available(iOS 16.0, *)
@MainActor
private func nexaFastGridLayout(
    columns: Int,
    minCellWidth: CGFloat?,
    itemHeight: CGFloat?,
    spacing: CGFloat
) -> UICollectionViewCompositionalLayout {
    let columns = max(1, columns)
    let minCellWidth = minCellWidth.map { max(1, $0) }
    let itemHeight = itemHeight.map { max(1, $0) }
    let spacing = max(0, spacing)
    return UICollectionViewCompositionalLayout { _, environment in
        let availableWidth = max(0, environment.container.effectiveContentSize.width)
        let columnCount = minCellWidth.map {
            max(1, Int((availableWidth + spacing) / ($0 + spacing)))
        } ?? columns
        let cellWidth = max(
            0,
            (availableWidth - spacing * CGFloat(columnCount - 1)) / CGFloat(columnCount)
        )
        let heightDimension: NSCollectionLayoutDimension = itemHeight.map {
            .absolute($0)
        } ?? .estimated(CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__))
        let itemSize = NSCollectionLayoutSize(
            widthDimension: .absolute(cellWidth),
            heightDimension: heightDimension
        )
        let item = NSCollectionLayoutItem(layoutSize: itemSize)
        let groupSize = NSCollectionLayoutSize(
            widthDimension: .fractionalWidth(1.0),
            heightDimension: heightDimension
        )
        let group = NSCollectionLayoutGroup.horizontal(
            layoutSize: groupSize,
            repeatingSubitem: item,
            count: columnCount
        )
        group.interItemSpacing = .fixed(spacing)
        let section = NSCollectionLayoutSection(group: group)
        section.interGroupSpacing = spacing
        return section
    }
}

@available(iOS 16.0, *)
private final class NexaFastMasonryLayout: UICollectionViewLayout {
    var columns: Int
    var spacing: CGFloat
    private var measuredHeights: [Int: CGFloat] = [:]
    private var columnAttributes: [[UICollectionViewLayoutAttributes]] = []
    private var itemAttributes: [UICollectionViewLayoutAttributes] = []
    private var contentSize: CGSize = .zero
    private var cachedWidth: CGFloat = -1
    private var cachedItemCount = -1
    private var isDirty = true

    init(columns: Int, spacing: CGFloat) {
        self.columns = max(1, columns)
        self.spacing = max(0, spacing)
        super.init()
    }

    required init?(coder: NSCoder) {
        nil
    }

    override func prepare() {
        super.prepare()
        guard let collectionView else { return }
        let width = max(0, collectionView.bounds.width - collectionView.adjustedContentInset.left - collectionView.adjustedContentInset.right)
        let itemCount = collectionView.numberOfItems(inSection: 0)
        guard isDirty || cachedWidth != width || cachedItemCount != itemCount else { return }

        if cachedWidth != width || cachedItemCount >= 0 && itemCount < cachedItemCount {
            measuredHeights.removeAll(keepingCapacity: true)
        }

        cachedWidth = width
        cachedItemCount = itemCount
        isDirty = false
        let laneCount = max(1, columns)
        let itemWidth = max(0, (width - spacing * CGFloat(laneCount - 1)) / CGFloat(laneCount))
        var laneBottoms = Array(repeating: CGFloat.zero, count: laneCount)
        columnAttributes = Array(repeating: [], count: laneCount)
        itemAttributes = []
        itemAttributes.reserveCapacity(itemCount)

        for item in 0..<itemCount {
            let lane = laneBottoms.enumerated().min(by: { $0.element < $1.element })?.offset ?? 0
            let y = laneBottoms[lane] == 0 ? 0 : laneBottoms[lane] + spacing
            let indexPath = IndexPath(item: item, section: 0)
            let attributes = UICollectionViewLayoutAttributes(forCellWith: indexPath)
            let height = measuredHeights[item] ?? 120
            attributes.frame = CGRect(x: CGFloat(lane) * (itemWidth + spacing), y: y, width: itemWidth, height: height)
            columnAttributes[lane].append(attributes)
            itemAttributes.append(attributes)
            laneBottoms[lane] = y + height
        }
        contentSize = CGSize(width: width, height: max(0, (laneBottoms.max() ?? 0)))
    }

    override var collectionViewContentSize: CGSize { contentSize }

    override func layoutAttributesForElements(in rect: CGRect) -> [UICollectionViewLayoutAttributes]? {
        var result: [UICollectionViewLayoutAttributes] = []
        for lane in columnAttributes {
            var low = 0
            var high = lane.count
            while low < high {
                let middle = (low + high) / 2
                if lane[middle].frame.maxY <= rect.minY {
                    low = middle + 1
                } else {
                    high = middle
                }
            }
            var index = low
            while index < lane.count, lane[index].frame.minY < rect.maxY {
                result.append(lane[index])
                index += 1
            }
        }
        return result
    }

    override func layoutAttributesForItem(at indexPath: IndexPath) -> UICollectionViewLayoutAttributes? {
        guard indexPath.section == 0, itemAttributes.indices.contains(indexPath.item) else {
            return nil
        }
        return itemAttributes[indexPath.item]
    }

    override func shouldInvalidateLayout(forBoundsChange newBounds: CGRect) -> Bool {
        guard let collectionView else { return false }
        let insets = collectionView.adjustedContentInset
        let width = max(0, newBounds.width - insets.left - insets.right)
        return width != cachedWidth
    }

    override func shouldInvalidateLayout(
        forPreferredLayoutAttributes preferredAttributes: UICollectionViewLayoutAttributes,
        withOriginalAttributes originalAttributes: UICollectionViewLayoutAttributes
    ) -> Bool {
        abs(preferredAttributes.size.height - originalAttributes.size.height) > 0.5
    }

    override func invalidationContext(
        forPreferredLayoutAttributes preferredAttributes: UICollectionViewLayoutAttributes,
        withOriginalAttributes originalAttributes: UICollectionViewLayoutAttributes
    ) -> UICollectionViewLayoutInvalidationContext {
        let context = super.invalidationContext(
            forPreferredLayoutAttributes: preferredAttributes,
            withOriginalAttributes: originalAttributes
        )
        context.invalidateItems(at: itemAttributes.map(\.indexPath))
        measuredHeights[originalAttributes.indexPath.item] = max(1, preferredAttributes.size.height)
        isDirty = true
        return context
    }
}

@available(iOS 16.0, *)
private final class NexaFastMasonryCell: UICollectionViewCell {
    override func preferredLayoutAttributesFitting(
        _ layoutAttributes: UICollectionViewLayoutAttributes
    ) -> UICollectionViewLayoutAttributes {
        let attributes = super.preferredLayoutAttributesFitting(layoutAttributes)
        let fittedSize = contentView.systemLayoutSizeFitting(
            CGSize(width: layoutAttributes.size.width, height: UIView.layoutFittingCompressedSize.height),
            withHorizontalFittingPriority: .required,
            verticalFittingPriority: .fittingSizeLevel
        )
        attributes.size.height = max(1, fittedSize.height)
        return attributes
    }
}

@available(iOS 16.0, *)
private struct NexaFastGridList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let columns: Int
    let minCellWidth: CGFloat?
    let itemHeight: CGFloat?
    let masonry: Bool
    let spacing: CGFloat
    let hideHorizontalScrollbar: Bool
    let hideVerticalScrollbar: Bool
    let scrollEnabled: Bool
    let rowKey: ((Int) -> AnyHashable)?
    let scrollPosition: Int32?
    let componentRef: NexaFastListRef?
    let onScrollPositionChanged: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    let onEndReached: (() -> Void)?
    // <nexa:scroll-events-field:begin>
    let onScroll: (() -> Void)?
    // <nexa:scroll-events-field:end>
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        columns: Int,
        minCellWidth: CGFloat? = nil,
        itemHeight: CGFloat? = nil,
        masonry: Bool = false,
        spacing: CGFloat = 0,
        hideHorizontalScrollbar: Bool = false,
        hideVerticalScrollbar: Bool = false,
        scrollEnabled: Bool = true,
        rowKey: ((Int) -> AnyHashable)? = nil,
        scrollPosition: Int32? = nil,
        componentRef: NexaFastListRef? = nil,
        onScrollPositionChanged: ((Int) -> Void)? = nil,
        isRefreshing: Bool = false,
        onRefresh: (() -> Void)? = nil,
        onEndReached: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:begin>
        onScroll: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:end>
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.columns = max(1, columns)
        self.minCellWidth = minCellWidth.map { max(1, $0) }
        self.itemHeight = itemHeight
        self.masonry = masonry
        self.spacing = max(0, spacing)
        self.hideHorizontalScrollbar = hideHorizontalScrollbar
        self.hideVerticalScrollbar = hideVerticalScrollbar
        self.scrollEnabled = scrollEnabled
        self.rowKey = rowKey
        self.scrollPosition = scrollPosition
        self.componentRef = componentRef
        self.onScrollPositionChanged = onScrollPositionChanged
        self.isRefreshing = isRefreshing
        self.onRefresh = onRefresh
        self.onEndReached = onEndReached
        // <nexa:scroll-events-init-assignment:begin>
        self.onScroll = onScroll
        // <nexa:scroll-events-init-assignment:end>
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(
            rowCount: rowCount,
            columns: columns,
            minCellWidth: minCellWidth,
            itemHeight: itemHeight,
            masonry: masonry,
            spacing: spacing,
            rowKey: rowKey,
            scrollPosition: scrollPosition,
            onScrollPositionChanged: onScrollPositionChanged,
            onEndReached: onEndReached,
            // <nexa:scroll-events-coordinator-argument:begin>
            onScroll: onScroll,
            // <nexa:scroll-events-coordinator-argument:end>
            rowContent: rowContent
        )
    }

    func makeUIView(context: Context) -> UICollectionView {
        let layout: UICollectionViewLayout = masonry
            ? NexaFastMasonryLayout(columns: columns, spacing: spacing)
            : nexaFastGridLayout(columns: columns, minCellWidth: minCellWidth, itemHeight: itemHeight, spacing: spacing)
        let collectionView = UICollectionView(frame: .zero, collectionViewLayout: layout)
        nexaConfigureEdgeToEdgeScrollView(collectionView, environment: context.environment)
        collectionView.dataSource = context.coordinator
        collectionView.delegate = context.coordinator
        nexaUpdateRefreshControl(
            collectionView,
            controller: &context.coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        collectionView.register(
            masonry ? NexaFastMasonryCell.self : UICollectionViewCell.self,
            forCellWithReuseIdentifier: nexaFastGridListCellReuseIdentifier
        )
        collectionView.alwaysBounceVertical = onRefresh != nil
        collectionView.isScrollEnabled = scrollEnabled
        collectionView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        collectionView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        collectionView.backgroundColor = .clear
        collectionView.reloadData()
        nexaAttachFastListRef(componentRef, to: collectionView, horizontal: false) { [weak collectionView] index, animated, viewPosition in
            guard let collectionView, rowCount > 0 else { return }
            let item = min(max(Int(index), 0), rowCount - 1)
            collectionView.layoutIfNeeded()
            let path = IndexPath(item: item, section: 0)
            let frame = collectionView.collectionViewLayout.layoutAttributesForItem(at: path)?.frame ?? .zero
            nexaScrollFastList(to: frame, in: collectionView, horizontal: false, viewPosition: viewPosition, animated: animated)
        }
        context.coordinator.applyScrollPosition(to: collectionView)
        DispatchQueue.main.async { [weak collectionView, weak coordinator = context.coordinator] in
            guard let collectionView, let coordinator else { return }
            coordinator.seedInitialScrollPosition(in: collectionView)
        }
        return collectionView
    }

    func updateUIView(_ collectionView: UICollectionView, context: Context) {
        nexaConfigureEdgeToEdgeScrollView(collectionView, environment: context.environment)
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        let previousScrollPosition = coordinator.scrollPosition
        let isScrollPositionEcho = previousScrollPosition != scrollPosition
            && coordinator.scrollPositionEchoes.consume(scrollPosition)
        if previousScrollPosition != scrollPosition, !isScrollPositionEcho {
            coordinator.scrollCallbackGeneration += 1
        }
        let layoutChanged = coordinator.columns != columns
            || coordinator.minCellWidth != minCellWidth
            || coordinator.itemHeight != itemHeight
            || coordinator.masonry != masonry
            || coordinator.spacing != max(0, spacing)
        coordinator.rowCount = rowCount
        coordinator.columns = max(1, columns)
        coordinator.minCellWidth = minCellWidth.map { max(1, $0) }
        coordinator.itemHeight = itemHeight
        coordinator.masonry = masonry
        coordinator.spacing = max(0, spacing)
        collectionView.isScrollEnabled = scrollEnabled
        collectionView.showsHorizontalScrollIndicator = !hideHorizontalScrollbar
        collectionView.showsVerticalScrollIndicator = !hideVerticalScrollbar
        coordinator.rowKey = rowKey
        coordinator.scrollPosition = scrollPosition
        nexaAttachFastListRef(componentRef, to: collectionView, horizontal: false) { [weak collectionView] index, animated, viewPosition in
            guard let collectionView, rowCount > 0 else { return }
            let item = min(max(Int(index), 0), rowCount - 1)
            collectionView.layoutIfNeeded()
            let path = IndexPath(item: item, section: 0)
            let frame = collectionView.collectionViewLayout.layoutAttributesForItem(at: path)?.frame ?? .zero
            nexaScrollFastList(to: frame, in: collectionView, horizontal: false, viewPosition: viewPosition, animated: animated)
        }
        coordinator.onScrollPositionChanged = onScrollPositionChanged
        coordinator.onEndReached = onEndReached
        // <nexa:scroll-events-update-assignment:begin>
        coordinator.onScroll = onScroll
        // <nexa:scroll-events-update-assignment:end>
        coordinator.rowContent = rowContent
        if layoutChanged {
            collectionView.setCollectionViewLayout(
                masonry
                    ? NexaFastMasonryLayout(columns: columns, spacing: spacing)
                    : nexaFastGridLayout(columns: columns, minCellWidth: minCellWidth, itemHeight: itemHeight, spacing: spacing),
                animated: false
            )
        }
        nexaUpdateRefreshControl(
            collectionView,
            controller: &coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        if previousRowCount != rowCount {
            coordinator.lastEndReachedRowCount = nil
            if !isScrollPositionEcho { coordinator.applyScrollPosition(to: collectionView) }
            collectionView.reloadData()
            collectionView.layoutIfNeeded()
            return
        } else if previousScrollPosition != scrollPosition, !isScrollPositionEcho {
            coordinator.applyScrollPosition(to: collectionView)
        }
        coordinator.reconfigureVisibleItems(in: collectionView, preservingViewport: true)
        collectionView.layoutIfNeeded()
    }

    final class Coordinator: NSObject, UICollectionViewDataSource, UICollectionViewDelegate {
        var rowCount: Int
        var columns: Int
        var minCellWidth: CGFloat?
        var itemHeight: CGFloat?
        var masonry: Bool
        var spacing: CGFloat
        var rowKey: ((Int) -> AnyHashable)?
        var scrollPosition: Int32?
        var onScrollPositionChanged: ((Int) -> Void)?
        var refreshController: NexaFastListRefreshController?
        var onEndReached: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:begin>
        var onScroll: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:end>
        var lastEndReachedRowCount: Int?
        var lastReportedScrollPosition: Int?
        var isApplyingProgrammaticScrollPosition = false
        var isIgnoringProgrammaticScrollCallbacks = false
        var scrollCallbackGeneration = 0
        var isReadyToReportScroll = false
        var needsVisibleItemReconfiguration = false
        var isApplyingVisibleItemReconfiguration = false
        var visibleItemReconfigurationGeneration = 0
        var scrollPositionEchoes = NexaFastListScrollPositionEchoes()
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            columns: Int,
            minCellWidth: CGFloat?,
            itemHeight: CGFloat?,
            masonry: Bool,
            spacing: CGFloat,
            rowKey: ((Int) -> AnyHashable)?,
            scrollPosition: Int32?,
            onScrollPositionChanged: ((Int) -> Void)?,
            onEndReached: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:begin>
            onScroll: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:end>
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.columns = max(1, columns)
            self.minCellWidth = minCellWidth.map { max(1, $0) }
            self.itemHeight = itemHeight
            self.masonry = masonry
            self.spacing = max(0, spacing)
            self.rowKey = rowKey
            self.scrollPosition = scrollPosition
                self.onScrollPositionChanged = onScrollPositionChanged
            self.refreshController = nil
            self.onEndReached = onEndReached
            // <nexa:scroll-events-coordinator-init-assignment:begin>
            self.onScroll = onScroll
            // <nexa:scroll-events-coordinator-init-assignment:end>
            self.lastEndReachedRowCount = nil
            self.lastReportedScrollPosition = nil
            self.rowContent = rowContent
            super.init()
        }

        func collectionView(
            _ collectionView: UICollectionView,
            numberOfItemsInSection section: Int
        ) -> Int {
            rowCount
        }

        func seedInitialScrollPosition(in collectionView: UICollectionView) {
            collectionView.layoutIfNeeded()
            lastReportedScrollPosition = collectionView.indexPathsForVisibleItems.lazy.map(\.item).min()
            isReadyToReportScroll = true
            reportEndReachedIfVisible(in: collectionView)
        }

        // __NEXA_RECONFIGURE_GRID__

        private func seedVisibleScrollPosition(in collectionView: UICollectionView) {
            lastReportedScrollPosition = collectionView.indexPathsForVisibleItems.lazy.map(\.item).min()
        }

        private func flushVisibleItemReconfigurationIfNeeded(in scrollView: UIScrollView) {
            guard needsVisibleItemReconfiguration,
                  let collectionView = scrollView as? UICollectionView
            else {
                return
            }
            reconfigureVisibleItems(in: collectionView)
        }

        private func reportEndReachedIfVisible(in collectionView: UICollectionView) {
            guard let onEndReached,
                  rowCount > 0,
                  lastEndReachedRowCount != rowCount,
                  collectionView.indexPathsForVisibleItems.contains(where: { $0.item >= rowCount - 1 })
            else {
                return
            }
            lastEndReachedRowCount = rowCount
            DispatchQueue.main.async { onEndReached() }
        }

        func collectionView(
            _ collectionView: UICollectionView,
            cellForItemAt indexPath: IndexPath
        ) -> UICollectionViewCell {
            let cell = collectionView.dequeueReusableCell(
                withReuseIdentifier: nexaFastGridListCellReuseIdentifier,
                for: indexPath
            )
            cell.contentConfiguration = UIHostingConfiguration {
                if let rowKey {
                    rowContent(indexPath.item)
                        .id(rowKey(indexPath.item))
                        .frame(maxWidth: .infinity, maxHeight: masonry ? nil : .infinity)
                        .frame(minHeight: itemHeight ?? CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__), alignment: .center)
                } else {
                    rowContent(indexPath.item)
                        .frame(maxWidth: .infinity, maxHeight: masonry ? nil : .infinity)
                        .frame(minHeight: itemHeight ?? CGFloat(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__), alignment: .center)
                }
            }
            .margins(.all, 0)
            return cell
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            if isApplyingProgrammaticScrollPosition { return }
            let isOverscrolling = nexaFastListIsOverscrolling(scrollView, horizontal: false)
            if isApplyingVisibleItemReconfiguration {
                if let collectionView = scrollView as? UICollectionView {
                    seedVisibleScrollPosition(in: collectionView)
                }
                return
            }
            // <nexa:scroll-events-condition-active:begin>
            if !isOverscrolling,
               !isIgnoringProgrammaticScrollCallbacks,
               (onScrollPositionChanged != nil || onScroll != nil),
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.lazy.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-active:end>
            // <nexa:scroll-events-condition-inactive:begin>
            if !isOverscrolling,
               !isIgnoringProgrammaticScrollCallbacks,
               onScrollPositionChanged != nil,
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.lazy.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-inactive:end>
            {
                lastReportedScrollPosition = position
                if isReadyToReportScroll {
                    let positionCallback = onScrollPositionChanged
                    // <nexa:scroll-events-callback:begin>
                    let scrollCallback = onScroll
                    // <nexa:scroll-events-callback:end>
                    let callbackGeneration = scrollCallbackGeneration
                    DispatchQueue.main.async { [weak self] in
                        guard let self else { return }
                        guard self.scrollCallbackGeneration == callbackGeneration else { return }
                        guard self.lastReportedScrollPosition == position else { return }
                        if let positionCallback,
                           self.scrollPosition != Int32(position) {
                            self.scrollPositionEchoes.record(position)
                            positionCallback(position)
                        }
                        // <nexa:scroll-events-callback:begin>
                        scrollCallback?()
                        // <nexa:scroll-events-callback:end>
                    }
                }
            }
            if let collectionView = scrollView as? UICollectionView {
                reportEndReachedIfVisible(in: collectionView)
            }
        }

        func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
            isIgnoringProgrammaticScrollCallbacks = false
            if isApplyingVisibleItemReconfiguration {
                visibleItemReconfigurationGeneration += 1
            }
            isApplyingVisibleItemReconfiguration = false
        }

        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate { flushVisibleItemReconfigurationIfNeeded(in: scrollView) }
        }

        func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
            flushVisibleItemReconfigurationIfNeeded(in: scrollView)
        }

        func scrollViewDidEndScrollingAnimation(_ scrollView: UIScrollView) {
            flushVisibleItemReconfigurationIfNeeded(in: scrollView)
        }

        func applyScrollPosition(to collectionView: UICollectionView) {
            guard let scrollPosition, rowCount > 0 else { return }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)
            guard (collectionView.indexPathsForVisibleItems.lazy.map(\.item).min() ?? -1) != target else {
                return
            }
            // Let scrollViewDidScroll report the new leading item.
            // Ignore the delegate's intermediate visible items during this
            // non-animated scroll; report the settled target exactly once below.
            isApplyingProgrammaticScrollPosition = true
            isIgnoringProgrammaticScrollCallbacks = true
            collectionView.scrollToItem(
                at: IndexPath(item: target, section: 0),
                at: .top,
                animated: false
            )
            collectionView.layoutIfNeeded()
            isApplyingProgrammaticScrollPosition = false
            if let position = collectionView.indexPathsForVisibleItems.lazy.map(\.item).min() {
                reportProgrammaticScrollPosition(position)
            }
            reportEndReachedIfVisible(in: collectionView)
        }

        private func reportProgrammaticScrollPosition(_ position: Int) {
            let boundPositionChanged = onScrollPositionChanged != nil
                && scrollPosition != Int32(position)
            guard lastReportedScrollPosition != position || boundPositionChanged else { return }
            lastReportedScrollPosition = position
            guard isReadyToReportScroll else { return }
            let positionCallback = onScrollPositionChanged
            // <nexa:scroll-events-callback:begin>
            let scrollCallback = onScroll
            // <nexa:scroll-events-callback:end>
            let callbackGeneration = scrollCallbackGeneration
            DispatchQueue.main.async { [weak self] in
                guard let self,
                      self.scrollCallbackGeneration == callbackGeneration,
                      self.lastReportedScrollPosition == position else { return }
                if let positionCallback, self.scrollPosition != Int32(position) {
                    self.scrollPositionEchoes.record(position)
                    positionCallback(position)
                }
                // <nexa:scroll-events-callback:begin>
                scrollCallback?()
                // <nexa:scroll-events-callback:end>
            }
        }
    }
}
// <nexa:list-runtime-grid:end>
"#,
    );
    if !uses_sticky_header {
        for section in [
            "identifier",
            "field",
            "init-parameter",
            "init-assignment",
            "coordinator-argument",
            "register",
            "update-assignment",
            "update-view",
            "coordinator-field",
            "coordinator-init-parameter",
            "coordinator-init-assignment",
            "delegate-methods",
            "generics",
        ] {
            runtime = remove_marked_section(&mut runtime, "sticky-header", section);
        }
    } else {
        runtime = remove_marked_section(&mut runtime, "sticky-header", "generics-none");
    }
    if !uses_scroll_events {
        for section in [
            "field",
            "init-parameter",
            "init-assignment",
            "coordinator-argument",
            "update-assignment",
            "coordinator-field",
            "coordinator-init-parameter",
            "coordinator-init-assignment",
            "callback",
            "condition-active",
        ] {
            runtime = remove_marked_section(&mut runtime, "scroll-events", section);
        }
    } else {
        runtime = remove_marked_section(&mut runtime, "scroll-events", "condition-inactive");
    }
    for (uses_list, section) in [
        (uses_vertical_list, "vertical"),
        (uses_horizontal_list, "horizontal"),
        (uses_grid_list, "grid"),
    ] {
        if !uses_list {
            runtime = remove_marked_section(&mut runtime, "list-runtime", section);
        }
    }
    if !uses_sectioned_list {
        runtime = remove_marked_section(&mut runtime, "list-runtime", "sectioned");
    }
    if !uses_vertical_list {
        runtime = remove_marked_section(&mut runtime, "list-runtime", "vertical-constants");
    }
    for section in [
        "identifier",
        "field",
        "init-parameter",
        "init-assignment",
        "coordinator-argument",
        "register",
        "update-assignment",
        "update-view",
        "coordinator-field",
        "coordinator-init-parameter",
        "coordinator-init-assignment",
        "delegate-methods",
        "generics",
        "generics-none",
    ] {
        runtime = strip_markers(&mut runtime, "sticky-header", section);
    }
    for section in [
        "field",
        "init-parameter",
        "init-assignment",
        "coordinator-argument",
        "update-assignment",
        "coordinator-field",
        "coordinator-init-parameter",
        "coordinator-init-assignment",
        "callback",
        "condition-active",
        "condition-inactive",
    ] {
        runtime = strip_markers(&mut runtime, "scroll-events", section);
    }
    for section in [
        "sectioned",
        "vertical",
        "horizontal",
        "grid",
        "vertical-constants",
    ] {
        runtime = strip_markers(&mut runtime, "list-runtime", section);
    }
    runtime = runtime.replace(
        "__NEXA_FAST_LIST_ROW_MIN_HEIGHT__",
        &nexa_codegen::design_system::FAST_LIST_ROW_MIN_HEIGHT.to_string(),
    );
    runtime = runtime.replace(
        "        // __NEXA_RECONFIGURE_TABLE__",
        &visible_content_reconfiguration(false, false),
    );
    runtime = runtime.replace(
        "        // __NEXA_RECONFIGURE_HORIZONTAL__",
        &visible_content_reconfiguration(true, true),
    );
    runtime = runtime.replace(
        "        // __NEXA_RECONFIGURE_GRID__",
        &visible_content_reconfiguration(true, false),
    );
    out.push_str(&runtime);
    if uses_vertical_list {
        out.push_str(&fast_list_primitive(uses_sticky_header, uses_scroll_events));
    }
    if uses_horizontal_list {
        out.push_str(&horizontal_list_primitive(uses_scroll_events));
    }
    if uses_grid_list {
        out.push_str(&grid_list_primitive(uses_scroll_events));
    }
    if uses_sectioned_list {
        out.push_str(include_str!("list_headers.swift"));
        out.push_str(sectioned_list_primitive());
    }
}

// Keep viewport preservation and command cancellation identical for every
// UIKit list coordinator, without introducing a runtime abstraction.
fn visible_content_reconfiguration(collection: bool, horizontal: bool) -> String {
    let (method, view, view_type, kind, paths, update) = if collection {
        (
            "Items",
            "collectionView",
            "UICollectionView",
            "Item",
            "collectionView.indexPathsForVisibleItems",
            "reconfigureItems",
        )
    } else {
        (
            "Rows",
            "tableView",
            "UITableView",
            "Row",
            "tableView.indexPathsForVisibleRows ?? []",
            "reconfigureRows",
        )
    };
    let report_completion = if collection {
        "            let expectedContentOffset = collectionView.contentOffset\n            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self, weak collectionView] in\n                guard let self, let collectionView else { return }\n                let currentContentOffset = collectionView.contentOffset\n                guard abs(currentContentOffset.x - expectedContentOffset.x) <= 1,\n                      abs(currentContentOffset.y - expectedContentOffset.y) <= 1\n                else { return }\n                collectionView.layoutIfNeeded()\n                if let position = collectionView.indexPathsForVisibleItems.lazy.map(\\.item).min() {\n                    self.reportProgrammaticScrollPosition(position)\n                }\n                self.reportEndReachedIfVisible(in: collectionView)\n            }"
    } else {
        "            if pageSnap {\n                reportSettledPage(in: tableView)\n            } else if let position = leadingVisibleRowPosition(in: tableView) {\n                reportProgrammaticScrollPosition(reverseLayout ? rowCount - 1 - position : position)\n            }\n            reportEndReachedIfVisible(in: tableView)"
    };
    r#"        func reconfigureVisible__METHOD__(in __VIEW__: __TYPE__, preservingViewport: Bool = true) {
            if !preservingViewport {
                // A ComponentRef scroll supersedes an earlier deferred
                // viewport restore, including one still waiting for layout.
                visible__KIND__ReconfigurationGeneration += 1
                isApplyingVisible__KIND__Reconfiguration = false
            }
            guard !isApplyingVisible__KIND__Reconfiguration else {
                needsVisible__KIND__Reconfiguration = true
                return
            }
            let visiblePaths = __PATHS__
            guard !visiblePaths.isEmpty else {
                needsVisible__KIND__Reconfiguration = false
                return
            }
            guard !__VIEW__.isTracking, !__VIEW__.isDragging, !__VIEW__.isDecelerating else {
                needsVisible__KIND__Reconfiguration = true
                return
            }

            needsVisible__KIND__Reconfiguration = false
            visible__KIND__ReconfigurationGeneration += 1
            let generation = visible__KIND__ReconfigurationGeneration
            let contentOffset = __VIEW__.contentOffset
            isApplyingVisible__KIND__Reconfiguration = true
            __VIEW__.__UPDATE__(at: visiblePaths)
            __VIEW__.layoutIfNeeded()
            // Refresh row content before applying a ref command, but leave its
            // target offset and callbacks under the command's control.
            guard preservingViewport else {
                isApplyingVisible__KIND__Reconfiguration = false
                return
            }
            if __VIEW__.contentOffset != contentOffset {
                __VIEW__.setContentOffset(contentOffset, animated: false)
                __VIEW__.layoutIfNeeded()
            }
            seedVisibleScrollPosition(in: __VIEW__)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self, weak __VIEW__] in
                guard let self,
                      let __VIEW__,
                      self.visible__KIND__ReconfigurationGeneration == generation
                else {
                    return
                }
                guard !__VIEW__.isTracking, !__VIEW__.isDragging, !__VIEW__.isDecelerating else {
                    self.isApplyingVisible__KIND__Reconfiguration = false
                    self.needsVisible__KIND__Reconfiguration = true
                    return
                }
                __VIEW__.layoutIfNeeded()
                if __VIEW__.contentOffset != contentOffset {
                    __VIEW__.setContentOffset(contentOffset, animated: false)
                    __VIEW__.layoutIfNeeded()
                }
                self.seedVisibleScrollPosition(in: __VIEW__)
                self.isApplyingVisible__KIND__Reconfiguration = false
                if self.needsVisible__KIND__Reconfiguration {
                    self.needsVisible__KIND__Reconfiguration = false
                    self.reconfigureVisible__METHOD__(in: __VIEW__)
                }
            }
__REPORT_COMPLETION__
        }


"#
    .replace("__HORIZONTAL__", if horizontal { "true" } else { "false" })
    .replace("__REPORT_COMPLETION__", report_completion)
    .replace("__METHOD__", method)
    .replace("__VIEW__", view)
    .replace("__TYPE__", view_type)
    .replace("__KIND__", kind)
    .replace("__PATHS__", paths)
    .replace("__UPDATE__", update)
}

fn horizontal_list_primitive(uses_scroll_events: bool) -> String {
    let mut out = String::from(
        "\n@available(iOS 16.0, *)\n@MainActor\nfunc NexaFastHorizontalListPrimitive<RowContent: View>(\n",
    );
    out.push_str("    rowCount: Int,\n    itemExtent: CGFloat? = nil,\n");
    out.push_str("    spacing: CGFloat = 0,\n");
    out.push_str("    hideHorizontalScrollbar: Bool = false,\n");
    out.push_str("    hideVerticalScrollbar: Bool = false,\n");
    out.push_str("    scrollEnabled: Bool = true,\n");
    out.push_str("    rowKey: ((Int) -> AnyHashable)? = nil,\n");
    out.push_str("    scrollPosition: Int32? = nil,\n");
    out.push_str("    componentRef: NexaFastListRef? = nil,\n");
    out.push_str("    onScrollPositionChanged: ((Int) -> Void)? = nil,\n");
    out.push_str("    isRefreshing: Bool = false,\n");
    out.push_str("    onRefresh: (() -> Void)? = nil,\n");
    out.push_str("    onEndReached: (() -> Void)? = nil,\n");
    if uses_scroll_events {
        out.push_str("    onScroll: (() -> Void)? = nil,\n");
    }
    out.push_str("    @ViewBuilder rowContent: @escaping (Int) -> RowContent\n");
    out.push_str(") -> some View {\n    NexaFastHorizontalList<RowContent>(\n");
    out.push_str(
        "        rowCount: rowCount,\n        itemExtent: itemExtent,\n        spacing: spacing,\n",
    );
    out.push_str("        hideHorizontalScrollbar: hideHorizontalScrollbar,\n");
    out.push_str("        hideVerticalScrollbar: hideVerticalScrollbar,\n");
    out.push_str("        scrollEnabled: scrollEnabled,\n");
    out.push_str("        rowKey: rowKey,\n        scrollPosition: scrollPosition,\n");
    out.push_str("        componentRef: componentRef,\n");
    out.push_str("        onScrollPositionChanged: onScrollPositionChanged,\n");
    out.push_str("        isRefreshing: isRefreshing,\n        onRefresh: onRefresh,\n");
    out.push_str("        onEndReached: onEndReached,\n");
    if uses_scroll_events {
        out.push_str("        onScroll: onScroll,\n");
    }
    out.push_str("        rowContent: rowContent\n    )\n}\n");
    out
}

fn grid_list_primitive(uses_scroll_events: bool) -> String {
    let mut out = String::from(
        "\n@available(iOS 16.0, *)\n@MainActor\nfunc NexaFastGridListPrimitive<RowContent: View>(\n",
    );
    out.push_str(
        "    rowCount: Int,\n    columns: Int,\n    minCellWidth: CGFloat? = nil,\n    itemHeight: CGFloat? = nil,\n    spacing: CGFloat = 0,\n",
    );
    out.push_str("    hideHorizontalScrollbar: Bool = false,\n");
    out.push_str("    hideVerticalScrollbar: Bool = false,\n");
    out.push_str("    scrollEnabled: Bool = true,\n");
    out.push_str("    rowKey: ((Int) -> AnyHashable)? = nil,\n");
    out.push_str("    masonry: Bool = false,\n");
    out.push_str("    scrollPosition: Int32? = nil,\n");
    out.push_str("    componentRef: NexaFastListRef? = nil,\n");
    out.push_str("    onScrollPositionChanged: ((Int) -> Void)? = nil,\n");
    out.push_str("    isRefreshing: Bool = false,\n");
    out.push_str("    onRefresh: (() -> Void)? = nil,\n");
    out.push_str("    onEndReached: (() -> Void)? = nil,\n");
    if uses_scroll_events {
        out.push_str("    onScroll: (() -> Void)? = nil,\n");
    }
    out.push_str("    @ViewBuilder rowContent: @escaping (Int) -> RowContent\n");
    out.push_str(") -> some View {\n    NexaFastGridList<RowContent>(\n");
    out.push_str(
        "        rowCount: rowCount,\n        columns: columns,\n        minCellWidth: minCellWidth,\n        itemHeight: itemHeight,\n        masonry: masonry,\n",
    );
    out.push_str("        spacing: spacing,\n");
    out.push_str("        hideHorizontalScrollbar: hideHorizontalScrollbar,\n");
    out.push_str("        hideVerticalScrollbar: hideVerticalScrollbar,\n");
    out.push_str("        scrollEnabled: scrollEnabled,\n");
    out.push_str("        rowKey: rowKey,\n        scrollPosition: scrollPosition,\n");
    out.push_str("        componentRef: componentRef,\n");
    out.push_str("        onScrollPositionChanged: onScrollPositionChanged,\n");
    out.push_str("        isRefreshing: isRefreshing,\n        onRefresh: onRefresh,\n");
    out.push_str("        onEndReached: onEndReached,\n");
    if uses_scroll_events {
        out.push_str("        onScroll: onScroll,\n");
    }
    out.push_str("        rowContent: rowContent\n    )\n    .frame(maxHeight: .infinity)\n}\n");
    out
}

fn sectioned_list_primitive() -> &'static str {
    r#"
@available(iOS 16.0, *)
@MainActor
func NexaFastSectionedListPrimitive<RowContent: View, HeaderContent: View>(
    sectionCount: Int,
    sectionCounts: [Int],
    rowHeight: CGFloat? = nil,
    spacing: CGFloat = 0,
    showDividers: Bool = true,
    hideHorizontalScrollbar: Bool = false,
    hideVerticalScrollbar: Bool = false,
    scrollEnabled: Bool = true,
    rowKey: ((Int, Int) -> AnyHashable)? = nil,
    componentRef: NexaFastListRef? = nil,
    isRefreshing: Bool = false,
    onRefresh: (() -> Void)? = nil,
    headerContent: ((Int) -> HeaderContent)? = nil,
    @ViewBuilder rowContent: @escaping (Int, Int) -> RowContent
) -> some View {
    NexaFastSectionedList<RowContent, HeaderContent>(
        sectionCount: sectionCount,
        sectionCounts: sectionCounts,
        rowHeight: rowHeight,
        spacing: spacing,
        showDividers: showDividers,
        hideHorizontalScrollbar: hideHorizontalScrollbar,
        hideVerticalScrollbar: hideVerticalScrollbar,
        scrollEnabled: scrollEnabled,
        rowKey: rowKey,
        componentRef: componentRef,
        isRefreshing: isRefreshing,
        onRefresh: onRefresh,
        headerContent: headerContent,
        rowContent: rowContent
    )
    .frame(maxHeight: .infinity)
}

"#
}

fn fast_list_primitive(uses_sticky_header: bool, uses_scroll_events: bool) -> String {
    let mut out = swiftui_page_snap_list().to_owned();
    out.push_str(
        r#"
@MainActor
private func nexaAttachFastListRef(
    _ reference: NexaFastListRef?,
    to view: UIScrollView,
    horizontal: Bool,
    reverseLayout: Bool = false,
    scrollToIndex: @escaping (Int32, Bool, Double) -> Void
) {
    guard let reference else { return }
    reference.attach(
        view: view,
        scrollToIndex: scrollToIndex,
        scrollToTop: { [weak view] animated in
            guard let view else { return }
            let minimum = horizontal ? -view.adjustedContentInset.left : -view.adjustedContentInset.top
            let maximum = horizontal
                ? max(minimum, view.contentSize.width - view.bounds.width + view.adjustedContentInset.right)
                : max(minimum, view.contentSize.height - view.bounds.height + view.adjustedContentInset.bottom)
            let leading = reverseLayout ? maximum : minimum
            let target = horizontal ? CGPoint(x: leading, y: view.contentOffset.y) : CGPoint(x: view.contentOffset.x, y: leading)
            view.setContentOffset(target, animated: animated)
        },
        scrollToEnd: { [weak view] animated in
            guard let view else { return }
            let minimum = horizontal ? -view.adjustedContentInset.left : -view.adjustedContentInset.top
            let maximum = horizontal
                ? max(minimum, view.contentSize.width - view.bounds.width + view.adjustedContentInset.right)
                : max(minimum, view.contentSize.height - view.bounds.height + view.adjustedContentInset.bottom)
            let trailing = reverseLayout ? minimum : maximum
            let target = horizontal ? CGPoint(x: trailing, y: view.contentOffset.y) : CGPoint(x: view.contentOffset.x, y: trailing)
            view.setContentOffset(target, animated: animated)
        },
        scrollToOffset: { [weak view] offset, animated in
            guard let view else { return }
            let minimum = horizontal ? -view.adjustedContentInset.left : -view.adjustedContentInset.top
            let maximum = horizontal
                ? max(minimum, view.contentSize.width - view.bounds.width + view.adjustedContentInset.right)
                : max(minimum, view.contentSize.height - view.bounds.height + view.adjustedContentInset.bottom)
            let logicalOffset = CGFloat(max(0, offset))
            let physicalOffset = reverseLayout ? maximum - logicalOffset : minimum + logicalOffset
            let clamped = min(max(physicalOffset, minimum), maximum)
            let target = horizontal ? CGPoint(x: clamped, y: view.contentOffset.y) : CGPoint(x: view.contentOffset.x, y: clamped)
            view.setContentOffset(target, animated: animated)
        }
    )
}

@MainActor
private func nexaScrollFastList(
    to frame: CGRect,
    in view: UIScrollView,
    horizontal: Bool,
    viewPosition: Double,
    animated: Bool
) {
    let position = CGFloat(viewPosition.isFinite ? min(max(viewPosition, 0), 1) : 0)
    let minimum = horizontal ? -view.adjustedContentInset.left : -view.adjustedContentInset.top
    let maximum = horizontal
        ? max(minimum, view.contentSize.width - view.bounds.width + view.adjustedContentInset.right)
        : max(minimum, view.contentSize.height - view.bounds.height + view.adjustedContentInset.bottom)
    let viewportLength = horizontal
        ? view.bounds.width - view.adjustedContentInset.left - view.adjustedContentInset.right
        : view.bounds.height - view.adjustedContentInset.top - view.adjustedContentInset.bottom
    let itemStart = horizontal ? frame.minX : frame.minY
    let itemLength = horizontal ? frame.width : frame.height
    let leadingInset = horizontal ? view.adjustedContentInset.left : view.adjustedContentInset.top
    let target = itemStart - leadingInset - max(0, viewportLength - itemLength) * position
    let clamped = min(max(target, minimum), maximum)
    let offset = horizontal
        ? CGPoint(x: clamped, y: view.contentOffset.y)
        : CGPoint(x: view.contentOffset.x, y: clamped)
    view.setContentOffset(offset, animated: animated)
}
"#,
    );
    append_fast_list_primitive(&mut out, false, uses_sticky_header, uses_scroll_events);
    if uses_sticky_header {
        append_fast_list_primitive(&mut out, true, true, uses_scroll_events);
    }
    out
}

fn append_fast_list_primitive(
    out: &mut String,
    has_header: bool,
    uses_sticky_header: bool,
    uses_scroll_events: bool,
) {
    out.push_str(
        "\n@available(iOS 16.0, *)\n@MainActor\n@ViewBuilder\nfunc NexaFastListPrimitive<",
    );
    out.push_str("RowContent: View");
    if has_header {
        out.push_str(", HeaderContent: View");
    }
    out.push_str(">(\n");
    out.push_str("    rowCount: Int,\n");
    out.push_str("    rowHeight: CGFloat? = nil,\n");
    out.push_str("    spacing: CGFloat = 0,\n");
    out.push_str("    showDividers: Bool = true,\n");
    out.push_str("    hideHorizontalScrollbar: Bool = false,\n");
    out.push_str("    hideVerticalScrollbar: Bool = false,\n");
    out.push_str("    scrollEnabled: Bool = true,\n");
    out.push_str("    rowKey: ((Int) -> AnyHashable)? = nil,\n");
    out.push_str("    reverseLayout: Bool = false,\n");
    out.push_str("    pageSnap: Bool = false,\n");
    out.push_str("    scrollPosition: Int32? = nil,\n");
    out.push_str("    componentRef: NexaFastListRef? = nil,\n");
    out.push_str("    restoreRevision: Int = 0,\n");
    out.push_str("    onScrollPositionChanged: ((Int) -> Void)? = nil,\n");
    out.push_str("    isRefreshing: Bool = false,\n");
    out.push_str("    onRefresh: (() -> Void)? = nil,\n");
    out.push_str("    onEndReached: (() -> Void)? = nil,\n");
    if uses_scroll_events {
        out.push_str("    onScroll: (() -> Void)? = nil,\n");
        out.push_str("    onScrollBeginDrag: (() -> Void)? = nil,\n");
    }
    if has_header {
        out.push_str("    headerContent: (() -> HeaderContent)?,\n");
    }
    out.push_str("    @ViewBuilder rowContent: @escaping (Int) -> RowContent\n");
    out.push_str(") -> some View {\n");
    if !has_header {
        out.push_str(
            "    if pageSnap && !reverseLayout && componentRef == nil {\n        if #available(iOS 17.0, *) {\n            NexaSwiftUIPageSnapList(\n                rowCount: rowCount,\n                scrollPosition: scrollPosition.map(Int.init),\n                scrollEnabled: scrollEnabled,\n                hideHorizontalScrollbar: hideHorizontalScrollbar,\n                hideVerticalScrollbar: hideVerticalScrollbar,\n                onScrollPositionChanged: onScrollPositionChanged,\n                onRefresh: onRefresh,\n",
        );
        if uses_scroll_events {
            out.push_str(
                "                onScroll: onScroll.map { callback in { _ in callback() } },\n",
            );
        } else {
            out.push_str("                onScroll: nil,\n");
        }
        out.push_str("                onEndReached: onEndReached.map { callback in { _ in callback() } },\n                rowContent: rowContent\n            )\n        } else {\n");
        append_native_fast_list_call(
            out,
            has_header,
            uses_sticky_header,
            uses_scroll_events,
            true,
        );
        out.push_str("        }\n    } else {\n");
    }
    append_native_fast_list_call(
        out,
        has_header,
        uses_sticky_header,
        uses_scroll_events,
        false,
    );
    if !has_header {
        out.push_str("    }\n");
    }
    out.push_str("}\n");
}

fn append_native_fast_list_call(
    out: &mut String,
    has_header: bool,
    uses_sticky_header: bool,
    uses_scroll_events: bool,
    page_snap_fallback: bool,
) {
    out.push_str("        NexaFastList<");
    out.push_str("RowContent");
    if has_header {
        out.push_str(", HeaderContent");
    } else if uses_sticky_header {
        out.push_str(", EmptyView");
    }
    out.push_str(">(\n");
    out.push_str("            rowCount: rowCount,\n");
    out.push_str("            rowHeight: rowHeight,\n");
    out.push_str("            spacing: spacing,\n");
    out.push_str(if page_snap_fallback {
        "            showDividers: false,\n"
    } else {
        "            showDividers: showDividers,\n"
    });
    out.push_str("            hideHorizontalScrollbar: hideHorizontalScrollbar,\n");
    out.push_str("            hideVerticalScrollbar: hideVerticalScrollbar,\n");
    out.push_str("            scrollEnabled: scrollEnabled,\n");
    out.push_str("            rowKey: rowKey,\n");
    out.push_str("            reverseLayout: reverseLayout,\n");
    out.push_str("            pageSnap: pageSnap,\n");
    out.push_str("            scrollPosition: scrollPosition,\n");
    out.push_str("            componentRef: componentRef,\n");
    out.push_str("            restoreRevision: restoreRevision,\n");
    out.push_str("            onScrollPositionChanged: onScrollPositionChanged,\n");
    out.push_str("            isRefreshing: isRefreshing,\n");
    out.push_str("            onRefresh: onRefresh,\n");
    out.push_str("            onEndReached: onEndReached,\n");
    if uses_scroll_events {
        out.push_str("            onScroll: onScroll,\n");
        out.push_str("            onScrollBeginDrag: onScrollBeginDrag,\n");
    }
    if has_header {
        out.push_str("            headerContent: headerContent,\n");
    } else if uses_sticky_header {
        out.push_str("            headerContent: nil,\n");
    }
    out.push_str(
        "            rowContent: rowContent\n        )\n        .frame(maxWidth: pageSnap ? .infinity : nil, maxHeight: .infinity)\n",
    );
}

fn swiftui_page_snap_list() -> &'static str {
    r#"
@available(iOS 17.0, *)
private struct NexaPageSnapRefreshModifier: ViewModifier {
    let action: (() -> Void)?

    @ViewBuilder
    func body(content: Content) -> some View {
        if let action {
            content.refreshable { action() }
        } else {
            content
        }
    }
}

@available(iOS 17.0, *)
@MainActor
private final class NexaSwiftUIPageSnapCallbackState {
    var lastReportedPage: Int?
    var lastEndReachedCount: Int?

    init(initialPage: Int?) {
        self.lastReportedPage = initialPage
        self.lastEndReachedCount = nil
    }
}

@available(iOS 17.0, *)
@MainActor
private struct NexaSwiftUIPageSnapList<RowContent: View>: View {
    @Environment(\.nexaScrollContentPadding) private var contentPadding
    let rowCount: Int
    let scrollPosition: Int?
    let scrollEnabled: Bool
    let hideHorizontalScrollbar: Bool
    let hideVerticalScrollbar: Bool
    let onScrollPositionChanged: ((Int) -> Void)?
    let onRefresh: (() -> Void)?
    let onScroll: ((Int) -> Void)?
    let onEndReached: ((Int) -> Void)?
    let rowContent: (Int) -> RowContent
    @State private var selectedPage: Int?
    @State private var callbackState: NexaSwiftUIPageSnapCallbackState

    init(
        rowCount: Int,
        scrollPosition: Int?,
        scrollEnabled: Bool,
        hideHorizontalScrollbar: Bool,
        hideVerticalScrollbar: Bool,
        onScrollPositionChanged: ((Int) -> Void)?,
        onRefresh: (() -> Void)?,
        onScroll: ((Int) -> Void)?,
        onEndReached: ((Int) -> Void)?,
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.scrollPosition = scrollPosition
        self.scrollEnabled = scrollEnabled
        self.hideHorizontalScrollbar = hideHorizontalScrollbar
        self.hideVerticalScrollbar = hideVerticalScrollbar
        self.onScrollPositionChanged = onScrollPositionChanged
        self.onRefresh = onRefresh
        self.onScroll = onScroll
        self.onEndReached = onEndReached
        self.rowContent = rowContent
        let initialPage = self.rowCount > 0
            ? min(max(scrollPosition ?? 0, 0), self.rowCount - 1)
            : nil
        _selectedPage = State(initialValue: initialPage)
        _callbackState = State(initialValue: NexaSwiftUIPageSnapCallbackState(initialPage: initialPage))
    }

    @ViewBuilder
    var body: some View {
        if #available(iOS 18.0, *) {
            GeometryReader { geometry in
                pagedScrollView(size: geometry.size)
                    .onAppear { reportSettledPage(selectedPage) }
                    .onScrollPhaseChange { _, phase in
                        if phase == .idle { reportSettledPage(selectedPage) }
                    }
            }
        } else {
            GeometryReader { geometry in
                pagedScrollView(size: geometry.size)
                    .onAppear { reportSettledPage(selectedPage) }
                    .onChange(of: selectedPage) { _, page in
                        guard let page else { return }
                        Task { @MainActor in
                            try? await Task.sleep(for: .milliseconds(180))
                            if selectedPage == page { reportSettledPage(page) }
                        }
                    }
            }
        }
    }

    private func pagedScrollView(size: CGSize) -> some View {
        ScrollView(.vertical) {
            LazyVStack(spacing: 0) {
                ForEach(0..<rowCount, id: \.self) { index in
                    rowContent(index)
                        .frame(width: size.width, height: size.height, alignment: .topLeading)
                        .environment(\.nexaScrollContentPadding, EdgeInsets())
                        .id(index)
                }
            }
            .scrollTargetLayout()
        }
        .contentMargins(.all, contentPadding, for: .scrollContent)
        .scrollTargetBehavior(.paging)
        .scrollPosition(id: $selectedPage, anchor: .top)
        .scrollDisabled(!scrollEnabled)
        .modifier(NexaScrollIndicatorVisibility(
            hideHorizontal: hideHorizontalScrollbar,
            hideVertical: hideVerticalScrollbar
        ))
        .modifier(NexaPageSnapRefreshModifier(action: onRefresh))
        .onChange(of: scrollPosition) { _, requestedPage in
            guard rowCount > 0, let requestedPage else { return }
            let page = min(max(requestedPage, 0), rowCount - 1)
            if selectedPage != page { selectedPage = page }
        }
        .onChange(of: rowCount) { _, newCount in
            if newCount <= 0 {
                selectedPage = nil
                callbackState.lastReportedPage = nil
                callbackState.lastEndReachedCount = nil
            } else if (selectedPage ?? 0) >= newCount {
                selectedPage = newCount - 1
            }
            if callbackState.lastEndReachedCount != newCount {
                callbackState.lastEndReachedCount = nil
            }
        }
    }

    private func reportSettledPage(_ page: Int?) {
        guard let page, rowCount > 0, (0..<rowCount).contains(page) else { return }
        let pageChanged = page != callbackState.lastReportedPage
        let didReachEnd = page == rowCount - 1 && callbackState.lastEndReachedCount != rowCount
        guard pageChanged || didReachEnd else { return }

        if pageChanged { callbackState.lastReportedPage = page }
        if didReachEnd { callbackState.lastEndReachedCount = rowCount }

        let positionCallback = onScrollPositionChanged
        let scrollCallback = onScroll
        let endReachedCallback = onEndReached
        DispatchQueue.main.async {
            if pageChanged {
                positionCallback?(page)
                scrollCallback?(page)
            }
            if didReachEnd { endReachedCallback?(page) }
        }
    }
}
"#
}

fn remove_marked_section(source: &mut String, prefix: &str, section: &str) -> String {
    let start = format!("// <nexa:{prefix}-{section}:begin>");
    let end = format!("// <nexa:{prefix}-{section}:end>");
    let mut source = std::mem::take(source);
    loop {
        let Some(start_position) = source.find(&start) else {
            return source;
        };
        let start_line = source[..start_position]
            .rfind('\n')
            .map_or(0, |position| position + 1);
        let end_position = start_position + start.len();
        let Some(end_relative_position) = source[end_position..].find(&end) else {
            return source;
        };
        let end_position = end_position + end_relative_position + end.len();
        let suffix_start = source[end_position..]
            .find('\n')
            .map_or(source.len(), |position| end_position + position + 1);
        source = format!("{}{}", &source[..start_line], &source[suffix_start..]);
    }
}

fn strip_markers(source: &mut String, prefix: &str, section: &str) -> String {
    let start = format!("// <nexa:{prefix}-{section}:begin>");
    let end = format!("// <nexa:{prefix}-{section}:end>");
    let mut source = remove_marker_line(source, &start);
    remove_marker_line(&mut source, &end)
}

fn remove_marker_line(source: &mut String, marker: &str) -> String {
    let Some(marker_position) = source.find(marker) else {
        return std::mem::take(source);
    };
    let line_start = source[..marker_position]
        .rfind('\n')
        .map_or(0, |position| position + 1);
    let line_end = source[marker_position..]
        .find('\n')
        .map_or(source.len(), |position| marker_position + position + 1);
    source.replace_range(line_start..line_end, "");
    remove_marker_line(source, marker)
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;

    use super::render;

    #[test]
    fn section_header_registry_is_only_emitted_for_sectioned_lists() {
        let mut sectioned = SourceWriter::new();
        render(&mut sectioned, false, false, false, false, false, true);
        let source = sectioned.as_str();
        assert!(source.contains("private final class NexaSectionHeaderRegistry"));
        assert!(source.contains("coordinator.sectionHeaders.forEach"));
        assert!(source.contains("willDisplayHeaderView view: UIView, forSection section: Int"));
        assert!(
            source.contains("didEndDisplayingHeaderView view: UIView, forSection section: Int")
        );
        assert!(!source.contains("for section in 0..<coordinator.sectionCount"));

        let mut plain = SourceWriter::new();
        render(&mut plain, false, false, true, true, true, false);
        assert!(!plain.as_str().contains("NexaSectionHeaderRegistry"));
    }

    #[test]
    fn scroll_observation_does_not_materialize_index_arrays() {
        let mut generated = SourceWriter::new();
        render(&mut generated, true, true, true, true, true, false);
        let source = generated.as_str();
        assert!(source.contains("visibleRows.lazy.map(\\.row)"));
        assert!(source.contains("visibleRows.lazy.compactMap"));
        assert!(!source.contains("visibleRows.map("));
        assert!(!source.contains("visibleRows.compactMap"));
        assert!(source.contains("indexPathsForVisibleItems.lazy.map(\\.item).min()"));
        assert!(!source.contains("indexPathsForVisibleItems.map("));
    }

    #[test]
    fn short_lists_bounce_only_when_refresh_is_enabled() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, false, true, true, true, true);

        assert!(
            generated
                .as_str()
                .contains("collectionView.alwaysBounceHorizontal = onRefresh != nil")
        );
        assert_eq!(
            generated
                .as_str()
                .matches("alwaysBounceVertical = onRefresh != nil")
                .count(),
            3,
            "vertical, grid, and sectioned lists should share refresh-aware bounce behavior"
        );
        assert!(!generated.as_str().contains("alwaysBounceHorizontal = true"));
        assert!(!generated.as_str().contains("alwaysBounceVertical = true"));
    }

    #[test]
    fn vertical_lists_expand_into_their_available_viewport() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, false, true, false, false, false);

        assert!(
            generated
                .as_str()
                .contains(".frame(maxWidth: pageSnap ? .infinity : nil, maxHeight: .infinity)")
        );
    }

    #[test]
    fn content_sized_fast_list_rows_share_the_52_point_minimum() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, false, true, false, false, true);

        assert_eq!(nexa_codegen::design_system::FAST_LIST_ROW_MIN_HEIGHT, 52);
        assert!(generated.as_str().contains(
            ".frame(\n                    minHeight: pageSnap ? nil : (rowHeight ?? CGFloat(52)),"
        ));
        assert!(
            generated
                .as_str()
                .contains(".frame(minHeight: rowHeight ?? CGFloat(52), alignment: .center)")
        );
        assert!(
            !generated
                .as_str()
                .contains("__NEXA_FAST_LIST_ROW_MIN_HEIGHT__")
        );

        let mut all_axes = SourceWriter::new();
        render(&mut all_axes, false, false, true, true, true, true);
        assert!(all_axes.as_str().contains("} ?? .estimated(CGFloat(52))"));
        assert!(all_axes.as_str().contains(
            "minHeight: itemExtent ?? CGFloat(52),\n                            maxHeight: itemExtent"
        ));
        assert!(
            all_axes
                .as_str()
                .contains(".frame(minHeight: itemHeight ?? CGFloat(52), alignment: .center)")
        );
    }

    #[test]
    fn horizontal_item_extent_sets_height_without_constraining_content_width() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, false, false, true, false, false);
        let source = generated.as_str();

        assert!(source.contains("layout.estimatedItemSize = nexaFastHorizontalEstimatedItemSize("));
        assert!(source.contains("maxHeight: itemExtent"));
        assert!(source.contains(".fixedSize(horizontal: true, vertical: false)"));
        assert!(source.contains("width: max(1, estimatedSize.width)"));
        assert!(source.contains("contentView.systemLayoutSizeFitting("));
        assert!(source.contains(
            "verticalFittingPriority: itemExtent == nil ? .fittingSizeLevel : .required"
        ));
        assert!(
            !source.contains("layout.itemSize = CGSize(width: itemExtent, height: itemExtent)"),
            "rowHeight must not become the horizontal cell width"
        );
    }

    #[test]
    fn grid_and_sectioned_lists_expand_into_their_available_viewport() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, false, false, false, true, true);

        assert_eq!(
            generated
                .as_str()
                .matches(".frame(maxHeight: .infinity)")
                .count(),
            2,
            "grid and sectioned list primitives should both claim the available height"
        );
    }

    #[test]
    fn masonry_height_changes_invalidate_all_repositioned_items() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, false, true, true, true, true);
        let source = generated.as_str();

        assert!(source.contains("context.invalidateItems(at: itemAttributes.map(\\.indexPath))"));
        assert!(source.contains("measuredHeights[originalAttributes.indexPath.item]"));
        assert!(source.contains("isDirty = true"));
        assert!(!source.contains("context.invalidateEverything = true"));
    }

    #[test]
    fn reverse_layout_flips_placement_without_reversing_row_data_again() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, false, true, false, false, false);

        assert!(
            generated
                .as_str()
                .contains("tableView.transform = reverseLayout")
        );
        assert!(generated.as_str().contains("let row = indexPath.row"));
        assert!(
            !generated
                .as_str()
                .contains("let row = reverseLayout ? rowCount - 1 - indexPath.row : indexPath.row")
        );
    }

    #[test]
    fn scroll_updates_preserve_the_viewport_and_defer_row_layout_until_idle() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, true, true, true, true, false);
        let source = generated.as_str();

        assert!(source.contains("var needsVisibleRowReconfiguration = false"));
        assert!(source.contains("var isApplyingVisibleRowReconfiguration = false"));
        assert!(source.contains("func reconfigureVisibleRows(in tableView: UITableView, preservingViewport: Bool = true)"));
        assert!(source.contains(
            "guard !tableView.isTracking, !tableView.isDragging, !tableView.isDecelerating else {"
        ));
        assert!(source.contains("needsVisibleRowReconfiguration = true"));
        assert!(source.contains("let contentOffset = tableView.contentOffset"));
        assert!(source.contains("tableView.setContentOffset(contentOffset, animated: false)"));
        assert!(source.contains("DispatchQueue.main.asyncAfter(deadline: .now() + 0.05)"));
        assert!(source.contains("flushVisibleRowReconfigurationIfNeeded(in: scrollView)"));
        assert!(source.contains("if isApplyingVisibleRowReconfiguration {"));
        assert_eq!(
            source.matches("preservingViewport: true").count(),
            3,
            "typed ref scrolls must supersede delayed viewport restoration on every axis"
        );
        assert_eq!(source.matches("guard preservingViewport else {").count(), 3);
        assert_eq!(
            source
                .matches("let expectedContentOffset = collectionView.contentOffset")
                .count(),
            2,
            "collection position callbacks must report the settled viewport"
        );
        assert_eq!(
            source
                .matches("let boundPositionChanged = onScrollPositionChanged != nil")
                .count(),
            2,
            "horizontal and grid commands must report when the bound index is stale"
        );
        assert_eq!(
            source
                .matches("self.isApplyingVisibleItemReconfiguration = false\n                if self.needsVisibleItemReconfiguration")
                .count(),
            2,
            "collection refreshes must release their guard before draining queued updates"
        );
        assert!(
            !source.contains("NEXA_FASTLIST_"),
            "temporary scroll diagnostics must not be emitted in generated apps"
        );
    }

    #[test]
    fn initial_visible_final_row_uses_the_same_end_reached_rule_on_every_axis() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, true, true, true, true, false);
        let source = generated.as_str();

        assert!(source.contains("reportEndReachedIfVisible(in: tableView)"));
        assert_eq!(
            source
                .matches("private func reportEndReachedIfVisible(in collectionView:")
                .count(),
            2,
            "horizontal and grid lists should each define one shared end-reached check"
        );
        assert_eq!(
            source
                .matches("reportEndReachedIfVisible(in: collectionView)")
                .count(),
            8,
            "horizontal and grid lists should check visibility on initialization, scrolling, indexed updates, and typed ref scrolls"
        );
        assert!(source.contains("lastEndReachedRowCount != rowCount"));
        assert!(source.contains("lastEndReachedRowCount = rowCount"));
    }

    #[test]
    fn programmatic_list_positions_report_through_the_scroll_delegate() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, true, true, true, true, false);
        let source = generated.as_str();

        assert!(source.contains("tableView.scrollToRow("));
        assert!(
            source.matches("collectionView.scrollToItem(").count() >= 2,
            "horizontal and grid lists must both route programmatic positions through native scrolling"
        );
        assert!(!source.contains("lastReportedScrollPosition = target"));
        assert!(
            source.contains("programmatic positions use the same callback path as user scrolls")
        );
    }

    #[test]
    fn controlled_scroll_positions_do_not_interrupt_an_active_drag() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, true, true, false, false, false);
        let source = generated.as_str();

        assert!(source.contains(
            "guard force || (!tableView.isTracking && !tableView.isDragging && !tableView.isDecelerating) else {"
        ));
        assert!(source.contains("coordinator.scrollPositionEchoes.consume(scrollPosition)"));
        assert!(!source.contains("NEXA_TRACE"));

        let viewport_resize = source
            .split("tableView.onViewportSizeChanged =")
            .nth(1)
            .expect("viewport resize handler")
            .split("tableView.isPagingEnabled")
            .next()
            .expect("viewport resize handler body");
        assert!(viewport_resize.contains("!tableView.isTracking"));
        assert!(viewport_resize.contains("!tableView.isDragging"));
        assert!(viewport_resize.contains("!tableView.isDecelerating"));
        assert!(viewport_resize.contains("coordinator.applyScrollPosition(to: tableView)"));
        assert!(!viewport_resize.contains("force: true"));
    }

    #[test]
    fn scroll_callbacks_ignore_ios_rubber_band_positions() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, true, true, true, true, false);
        let source = generated.as_str();

        assert!(source.contains("private func nexaFastListIsOverscrolling"));
        assert!(!source.contains("NEXA_TRACE"));
        assert_eq!(
            source.matches("if !isOverscrolling,").count(),
            3,
            "each scroll callback path should ignore temporary overscroll positions"
        );
    }

    #[test]
    fn sticky_headers_do_not_report_rows_hidden_behind_the_header() {
        let mut generated = SourceWriter::new();
        render(&mut generated, false, true, true, false, false, false);
        let source = generated.as_str();

        assert!(
            source.contains("func leadingVisibleRowPosition(in tableView: UITableView) -> Int?")
        );
        assert!(source.contains("tableView.headerView(forSection: 0)"));
        assert!(source.contains("pinnedHeader.frame.maxY"));
        assert!(source.contains("tableView.rectForRow(at: indexPath).minY >= visibleTop - 1"));
        assert!(source.contains("let visiblePosition = leadingVisibleRowPosition(in: tableView)"));
    }
}
