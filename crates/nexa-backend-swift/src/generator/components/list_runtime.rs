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

// <nexa:list-runtime-sectioned:begin>
private let nexaFastSectionedCellReuseIdentifier = "NexaFastSectionedCell"
private let nexaFastSectionedHeaderReuseIdentifier = "NexaFastSectionedHeader"

@available(iOS 16.0, *)
private struct NexaFastSectionedList<RowContent: View, HeaderContent: View>: UIViewRepresentable {
    let sectionCount: Int
    let sectionCounts: [Int]
    let rowHeight: CGFloat?
    let rowKey: ((Int, Int) -> AnyHashable)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    let headerContent: ((Int) -> HeaderContent)?
    let rowContent: (Int, Int) -> RowContent

    init(
        sectionCount: Int,
        sectionCounts: [Int],
        rowHeight: CGFloat? = nil,
        rowKey: ((Int, Int) -> AnyHashable)? = nil,
        isRefreshing: Bool = false,
        onRefresh: (() -> Void)? = nil,
        headerContent: ((Int) -> HeaderContent)? = nil,
        @ViewBuilder rowContent: @escaping (Int, Int) -> RowContent
    ) {
        self.sectionCount = max(0, sectionCount)
        self.sectionCounts = sectionCounts.map { max(0, $0) }
        self.rowHeight = rowHeight
        self.rowKey = rowKey
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
            rowKey: rowKey,
            headerContent: headerContent,
            rowContent: rowContent
        )
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = UITableView(frame: .zero, style: .plain)
        tableView.dataSource = context.coordinator
        tableView.delegate = context.coordinator
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
        return tableView
    }

    func updateUIView(_ tableView: UITableView, context: Context) {
        let coordinator = context.coordinator
        let previousSectionCount = coordinator.sectionCount
        let previousSectionCounts = coordinator.sectionCounts
        coordinator.sectionCount = sectionCount
        coordinator.sectionCounts = sectionCounts.map { max(0, $0) }
        coordinator.rowHeight = rowHeight
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
            tableView.reloadData()
            return
        }
        if let visibleRows = tableView.indexPathsForVisibleRows, !visibleRows.isEmpty {
            tableView.reconfigureRows(at: visibleRows)
        }
        if let headerContent {
            for section in 0..<coordinator.sectionCount {
                guard let header = tableView.headerView(forSection: section) else { continue }
                header.contentConfiguration = UIHostingConfiguration {
                    headerContent(section)
                }
                .margins(.all, 0)
            }
        }
    }

    final class Coordinator: NSObject, UITableViewDataSource, UITableViewDelegate {
        var sectionCount: Int
        var sectionCounts: [Int]
        var rowHeight: CGFloat?
        var rowKey: ((Int, Int) -> AnyHashable)?
        var refreshController: NexaFastListRefreshController?
        var headerContent: ((Int) -> HeaderContent)?
        var rowContent: (Int, Int) -> RowContent

        init(
            sectionCount: Int,
            sectionCounts: [Int],
            rowHeight: CGFloat?,
            rowKey: ((Int, Int) -> AnyHashable)?,
            headerContent: ((Int) -> HeaderContent)?,
            rowContent: @escaping (Int, Int) -> RowContent
        ) {
            self.sectionCount = sectionCount
            self.sectionCounts = sectionCounts
            self.rowHeight = rowHeight
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
            cell.contentConfiguration = UIHostingConfiguration {
                if let rowKey {
                    rowContent(indexPath.section, indexPath.row).id(rowKey(indexPath.section, indexPath.row))
                } else {
                    rowContent(indexPath.section, indexPath.row)
                }
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

        func tableView(_ tableView: UITableView, heightForHeaderInSection section: Int) -> CGFloat {
            headerContent == nil ? .leastNormalMagnitude : UITableView.automaticDimension
        }

        func tableView(_ tableView: UITableView, estimatedHeightForHeaderInSection section: Int) -> CGFloat {
            headerContent == nil ? 0 : 44
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

    override func layoutSubviews() {
        super.layoutSubviews()
        guard bounds.size != reportedBoundsSize else { return }
        reportedBoundsSize = bounds.size
        onViewportSizeChanged?(bounds.size)
    }
}

@available(iOS 16.0, *)
private struct NexaFastList<RowContent: View, HeaderContent: View>: UIViewRepresentable {
    let rowCount: Int
    let rowHeight: CGFloat?
    let rowKey: ((Int) -> AnyHashable)?
    let reverseLayout: Bool
    let pageSnap: Bool
    let scrollPosition: Int32?
    let onScrollPositionChanged: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    let onEndReached: (() -> Void)?
    // <nexa:scroll-events-field:begin>
    let onScroll: (() -> Void)?
    // <nexa:scroll-events-field:end>
    // <nexa:sticky-header-field:begin>
    let headerContent: (() -> HeaderContent)?
    // <nexa:sticky-header-field:end>
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        rowHeight: CGFloat? = nil,
        rowKey: ((Int) -> AnyHashable)? = nil,
        reverseLayout: Bool = false,
        pageSnap: Bool = false,
        scrollPosition: Int32? = nil,
        onScrollPositionChanged: ((Int) -> Void)? = nil,
        isRefreshing: Bool = false,
        onRefresh: (() -> Void)? = nil,
        onEndReached: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:begin>
        onScroll: (() -> Void)? = nil,
        // <nexa:scroll-events-init-parameter:end>
        // <nexa:sticky-header-init-parameter:begin>
        headerContent: (() -> HeaderContent)? = nil,
        // <nexa:sticky-header-init-parameter:end>
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.rowHeight = rowHeight
        self.rowKey = rowKey
        self.reverseLayout = reverseLayout
        self.pageSnap = pageSnap
        self.scrollPosition = scrollPosition
        self.onScrollPositionChanged = onScrollPositionChanged
        self.isRefreshing = isRefreshing
        self.onRefresh = onRefresh
        self.onEndReached = onEndReached
        // <nexa:scroll-events-init-assignment:begin>
        self.onScroll = onScroll
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
            rowKey: rowKey,
            reverseLayout: reverseLayout,
            pageSnap: pageSnap,
            scrollPosition: scrollPosition,
            onScrollPositionChanged: onScrollPositionChanged,
            onEndReached: onEndReached,
            // <nexa:scroll-events-coordinator-argument:begin>
            onScroll: onScroll,
            // <nexa:scroll-events-coordinator-argument:end>
            // <nexa:sticky-header-coordinator-argument:begin>
            headerContent: headerContent,
            // <nexa:sticky-header-coordinator-argument:end>
            rowContent: rowContent
        )
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = NexaFastListTableView(frame: .zero, style: .plain)
        let coordinator = context.coordinator
        tableView.onViewportSizeChanged = { [weak tableView, weak coordinator] size in
            DispatchQueue.main.async {
                guard let tableView, let coordinator else { return }
                coordinator.updatePageHeight(size.height, in: tableView)
            }
        }
        tableView.isPagingEnabled = pageSnap
        if pageSnap {
            tableView.contentInsetAdjustmentBehavior = .never
        }
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
        context.coordinator.applyScrollPosition(to: tableView)
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
        let previousContentHeight = tableView.contentSize.height
        let previousContentOffset = tableView.contentOffset
        let wasAtBottom = reverseLayout
            && previousContentOffset.y <= -tableView.adjustedContentInset.top + 1
        coordinator.rowCount = rowCount
        coordinator.rowHeight = rowHeight
        coordinator.rowKey = rowKey
        coordinator.reverseLayout = reverseLayout
        coordinator.pageSnap = pageSnap
        coordinator.scrollPosition = scrollPosition
        coordinator.onScrollPositionChanged = onScrollPositionChanged
        coordinator.onEndReached = onEndReached
        // <nexa:scroll-events-update-assignment:begin>
        coordinator.onScroll = onScroll
        // <nexa:scroll-events-update-assignment:end>
        // <nexa:sticky-header-update-assignment:begin>
        coordinator.headerContent = headerContent
        // <nexa:sticky-header-update-assignment:end>
        coordinator.rowContent = rowContent
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
        tableView.contentInsetAdjustmentBehavior = pageSnap ? .never : .automatic
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
            coordinator.applyScrollPosition(to: tableView)
            return
        }
        if previousReverseLayout != reverseLayout {
            tableView.reloadData()
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
                coordinator.scrollToPage(targetPage, in: tableView)
            } else {
                coordinator.applyScrollPosition(to: tableView)
            }
            return
        } else if previousScrollPosition != scrollPosition {
            coordinator.applyScrollPosition(to: tableView)
        }

        if let visibleRows = tableView.indexPathsForVisibleRows, !visibleRows.isEmpty {
            tableView.reconfigureRows(at: visibleRows)
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
        var rowKey: ((Int) -> AnyHashable)?
        var reverseLayout: Bool
        var pageSnap: Bool
        var pageHeight: CGFloat
        var scrollPosition: Int32?
        var onScrollPositionChanged: ((Int) -> Void)?
        var refreshController: NexaFastListRefreshController?
        var onEndReached: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:begin>
        var onScroll: (() -> Void)?
        // <nexa:scroll-events-coordinator-field:end>
        // <nexa:sticky-header-coordinator-field:begin>
        var headerContent: (() -> HeaderContent)?
        // <nexa:sticky-header-coordinator-field:end>
        var lastEndReachedRowCount: Int?
        var lastReportedScrollPosition: Int?
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            rowHeight: CGFloat?,
            rowKey: ((Int) -> AnyHashable)?,
            reverseLayout: Bool,
            pageSnap: Bool,
            scrollPosition: Int32?,
            onScrollPositionChanged: ((Int) -> Void)?,
            onEndReached: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:begin>
            onScroll: (() -> Void)?,
            // <nexa:scroll-events-coordinator-init-parameter:end>
            // <nexa:sticky-header-coordinator-init-parameter:begin>
            headerContent: (() -> HeaderContent)?,
            // <nexa:sticky-header-coordinator-init-parameter:end>
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.rowHeight = rowHeight
            self.rowKey = rowKey
            self.reverseLayout = reverseLayout
            self.pageSnap = pageSnap
            self.pageHeight = 0
            self.scrollPosition = scrollPosition
            self.onScrollPositionChanged = onScrollPositionChanged
            self.refreshController = nil
            self.onEndReached = onEndReached
            // <nexa:scroll-events-coordinator-init-assignment:begin>
            self.onScroll = onScroll
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
            cell.transform = reverseLayout
                ? CGAffineTransform(scaleX: 1, y: -1)
                : .identity
            let row = reverseLayout ? rowCount - 1 - indexPath.row : indexPath.row
            cell.contentConfiguration = UIHostingConfiguration {
                if let rowKey {
                    rowContent(row).id(rowKey(row))
                } else {
                    rowContent(row)
                }
            }
            .margins(.all, 0)
            return cell
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
            guard !pageSnap else { return }
            let visibleRows = (scrollView as? UITableView)?.indexPathsForVisibleRows ?? []
            let visiblePosition = reverseLayout
                ? visibleRows.map(\.row).max()
                : visibleRows.first?.row
            let logicalPosition = visiblePosition.map { position in
                reverseLayout ? rowCount - 1 - position : position
            }
            // <nexa:scroll-events-condition-active:begin>
            if (onScrollPositionChanged != nil || onScroll != nil),
               let position = logicalPosition,
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-active:end>
            // <nexa:scroll-events-condition-inactive:begin>
            if onScrollPositionChanged != nil,
               let position = logicalPosition,
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-inactive:end>
            {
                lastReportedScrollPosition = position
                onScrollPositionChanged?(position)
                // <nexa:scroll-events-callback:begin>
                onScroll?()
                // <nexa:scroll-events-callback:end>
            }
            guard let onEndReached, rowCount > 0 else { return }
            let reachedEnd = (scrollView as? UITableView)?.indexPathsForVisibleRows?.contains {
                $0.row >= rowCount - 1
            } == true
            guard reachedEnd, lastEndReachedRowCount != rowCount else { return }
            lastEndReachedRowCount = rowCount
            onEndReached()
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
            reportSettledPage(in: scrollView)
        }

        func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
            if !decelerate {
                reportSettledPage(in: scrollView)
            }
        }

        func scrollViewDidEndScrollingAnimation(_ scrollView: UIScrollView) {
            reportSettledPage(in: scrollView)
        }

        private func reportSettledPage(in scrollView: UIScrollView) {
            guard pageSnap, rowCount > 0, let tableView = scrollView as? UITableView else { return }
            let page = currentPage(in: tableView)
            if lastReportedScrollPosition != page {
                lastReportedScrollPosition = page
                onScrollPositionChanged?(page)
                // <nexa:scroll-events-callback:begin>
                onScroll?()
                // <nexa:scroll-events-callback:end>
            }
            guard let onEndReached, page == rowCount - 1, lastEndReachedRowCount != rowCount else {
                return
            }
            lastEndReachedRowCount = rowCount
            onEndReached()
        }

        func applyScrollPosition(to tableView: UITableView) {
            guard let scrollPosition, rowCount > 0 else { return }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)
            let visualTarget = reverseLayout ? rowCount - 1 - target : target
            let visiblePosition = reverseLayout
                ? tableView.indexPathsForVisibleRows?.map(\.row).max()
                : tableView.indexPathsForVisibleRows?.first?.row
            guard (visiblePosition ?? -1) != visualTarget else {
                return
            }
            lastReportedScrollPosition = target
            tableView.scrollToRow(
                at: IndexPath(row: visualTarget, section: 0),
                at: reverseLayout ? .bottom : .top,
                animated: false
            )
        }
    }
}
// <nexa:list-runtime-vertical:end>

// <nexa:list-runtime-horizontal:begin>
private let nexaFastHorizontalListCellReuseIdentifier = "NexaFastHorizontalListCell"

@available(iOS 16.0, *)
private struct NexaFastHorizontalList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let itemExtent: CGFloat?
    let rowKey: ((Int) -> AnyHashable)?
    let scrollPosition: Int32?
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
        rowKey: ((Int) -> AnyHashable)? = nil,
        scrollPosition: Int32? = nil,
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
        self.rowKey = rowKey
        self.scrollPosition = scrollPosition
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
        layout.estimatedItemSize = UICollectionViewFlowLayout.automaticSize
        if let itemExtent {
            layout.itemSize = CGSize(width: itemExtent, height: itemExtent)
        }
        layout.minimumLineSpacing = 0
        layout.minimumInteritemSpacing = 0
        let collectionView = UICollectionView(frame: .zero, collectionViewLayout: layout)
        collectionView.dataSource = context.coordinator
        collectionView.delegate = context.coordinator
        nexaUpdateRefreshControl(
            collectionView,
            controller: &context.coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        collectionView.register(
            UICollectionViewCell.self,
            forCellWithReuseIdentifier: nexaFastHorizontalListCellReuseIdentifier
        )
        collectionView.alwaysBounceHorizontal = true
        collectionView.showsHorizontalScrollIndicator = true
        collectionView.backgroundColor = .clear
        collectionView.reloadData()
        context.coordinator.applyScrollPosition(to: collectionView)
        return collectionView
    }

    func updateUIView(_ collectionView: UICollectionView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        let previousScrollPosition = coordinator.scrollPosition
        coordinator.rowCount = rowCount
        coordinator.itemExtent = itemExtent
        coordinator.rowKey = rowKey
        coordinator.scrollPosition = scrollPosition
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
            coordinator.applyScrollPosition(to: collectionView)
        } else if previousScrollPosition != scrollPosition {
            coordinator.applyScrollPosition(to: collectionView)
        }

        guard previousRowCount != rowCount else {
            let visibleItems = collectionView.indexPathsForVisibleItems
            if !visibleItems.isEmpty {
                collectionView.reloadItems(at: visibleItems)
            }
            return
        }
        collectionView.reloadData()
    }

    final class Coordinator: NSObject, UICollectionViewDataSource, UICollectionViewDelegate {
        var rowCount: Int
        var itemExtent: CGFloat?
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
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            itemExtent: CGFloat?,
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

        func collectionView(
            _ collectionView: UICollectionView,
            cellForItemAt indexPath: IndexPath
        ) -> UICollectionViewCell {
            let cell = collectionView.dequeueReusableCell(
                withReuseIdentifier: nexaFastHorizontalListCellReuseIdentifier,
                for: indexPath
            )
            cell.contentConfiguration = UIHostingConfiguration {
                if let rowKey {
                    rowContent(indexPath.item).id(rowKey(indexPath.item))
                } else {
                    rowContent(indexPath.item)
                }
            }
            .margins(.all, 0)
            return cell
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            // <nexa:scroll-events-condition-active:begin>
            if (onScrollPositionChanged != nil || onScroll != nil),
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-active:end>
            // <nexa:scroll-events-condition-inactive:begin>
            if onScrollPositionChanged != nil,
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-inactive:end>
            {
                lastReportedScrollPosition = position
                onScrollPositionChanged?(position)
                // <nexa:scroll-events-callback:begin>
                onScroll?()
                // <nexa:scroll-events-callback:end>
            }
            guard let onEndReached, rowCount > 0 else { return }
            let reachedEnd = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.contains {
                $0.item >= rowCount - 1
            } == true
            guard reachedEnd, lastEndReachedRowCount != rowCount else { return }
            lastEndReachedRowCount = rowCount
            onEndReached()
        }

        func applyScrollPosition(to collectionView: UICollectionView) {
            guard let scrollPosition, rowCount > 0 else { return }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)
            guard (collectionView.indexPathsForVisibleItems.map(\.item).min() ?? -1) != target else {
                return
            }
            lastReportedScrollPosition = target
            collectionView.scrollToItem(
                at: IndexPath(item: target, section: 0),
                at: .left,
                animated: false
            )
        }
    }
}
// <nexa:list-runtime-horizontal:end>

// <nexa:list-runtime-grid:begin>
private let nexaFastGridListCellReuseIdentifier = "NexaFastGridListCell"

@available(iOS 16.0, *)
private struct NexaFastGridList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let columns: Int
    let itemHeight: CGFloat?
    let rowKey: ((Int) -> AnyHashable)?
    let scrollPosition: Int32?
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
        itemHeight: CGFloat? = nil,
        rowKey: ((Int) -> AnyHashable)? = nil,
        scrollPosition: Int32? = nil,
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
        self.itemHeight = itemHeight
        self.rowKey = rowKey
        self.scrollPosition = scrollPosition
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
            itemHeight: itemHeight,
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
        let heightDimension: NSCollectionLayoutDimension = itemHeight.map { .absolute($0) } ?? .estimated(44)
        let itemSize = NSCollectionLayoutSize(
            widthDimension: .fractionalWidth(1.0 / CGFloat(columns)),
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
            count: columns
        )
        let layout = UICollectionViewCompositionalLayout(
            section: NSCollectionLayoutSection(group: group)
        )
        let collectionView = UICollectionView(frame: .zero, collectionViewLayout: layout)
        collectionView.dataSource = context.coordinator
        collectionView.delegate = context.coordinator
        nexaUpdateRefreshControl(
            collectionView,
            controller: &context.coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        collectionView.register(
            UICollectionViewCell.self,
            forCellWithReuseIdentifier: nexaFastGridListCellReuseIdentifier
        )
        collectionView.alwaysBounceVertical = true
        collectionView.showsVerticalScrollIndicator = true
        collectionView.backgroundColor = .clear
        collectionView.reloadData()
        context.coordinator.applyScrollPosition(to: collectionView)
        return collectionView
    }

    func updateUIView(_ collectionView: UICollectionView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        let previousScrollPosition = coordinator.scrollPosition
        coordinator.rowCount = rowCount
        coordinator.itemHeight = itemHeight
        coordinator.rowKey = rowKey
        coordinator.scrollPosition = scrollPosition
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
            coordinator.applyScrollPosition(to: collectionView)
        } else if previousScrollPosition != scrollPosition {
            coordinator.applyScrollPosition(to: collectionView)
        }

        guard previousRowCount != rowCount else {
            let visibleItems = collectionView.indexPathsForVisibleItems
            if !visibleItems.isEmpty {
                collectionView.reloadItems(at: visibleItems)
            }
            return
        }
        collectionView.reloadData()
    }

    final class Coordinator: NSObject, UICollectionViewDataSource, UICollectionViewDelegate {
        var rowCount: Int
        var itemHeight: CGFloat?
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
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            itemHeight: CGFloat?,
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
            self.itemHeight = itemHeight
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
                    rowContent(indexPath.item).id(rowKey(indexPath.item))
                } else {
                    rowContent(indexPath.item)
                }
            }
            .margins(.all, 0)
            return cell
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            // <nexa:scroll-events-condition-active:begin>
            if (onScrollPositionChanged != nil || onScroll != nil),
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-active:end>
            // <nexa:scroll-events-condition-inactive:begin>
            if onScrollPositionChanged != nil,
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.map(\.item).min(),
               lastReportedScrollPosition != position
            // <nexa:scroll-events-condition-inactive:end>
            {
                lastReportedScrollPosition = position
                onScrollPositionChanged?(position)
                // <nexa:scroll-events-callback:begin>
                onScroll?()
                // <nexa:scroll-events-callback:end>
            }
            guard let onEndReached, rowCount > 0 else { return }
            let reachedEnd = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.contains {
                $0.item >= rowCount - 1
            } == true
            guard reachedEnd, lastEndReachedRowCount != rowCount else { return }
            lastEndReachedRowCount = rowCount
            onEndReached()
        }

        func applyScrollPosition(to collectionView: UICollectionView) {
            guard let scrollPosition, rowCount > 0 else { return }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)
            guard (collectionView.indexPathsForVisibleItems.map(\.item).min() ?? -1) != target else {
                return
            }
            lastReportedScrollPosition = target
            collectionView.scrollToItem(
                at: IndexPath(item: target, section: 0),
                at: .top,
                animated: false
            )
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
        ] {
            runtime = remove_marked_section(&mut runtime, "sticky-header", section);
        }
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
    out.push_str(&runtime);
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
