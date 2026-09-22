pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
private let nexaFastListCellReuseIdentifier = "NexaFastListCell"

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

@available(iOS 16.0, *)
private struct NexaFastList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let rowHeight: CGFloat?
    let rowKey: ((Int) -> AnyHashable)?
    let scrollPosition: Int32?
    let onScrollPositionChanged: ((Int) -> Void)?
    let isRefreshing: Bool
    let onRefresh: (() -> Void)?
    let onEndReached: (() -> Void)?
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        rowHeight: CGFloat? = nil,
        rowKey: ((Int) -> AnyHashable)? = nil,
        scrollPosition: Int32? = nil,
        onScrollPositionChanged: ((Int) -> Void)? = nil,
        isRefreshing: Bool = false,
        onRefresh: (() -> Void)? = nil,
        onEndReached: (() -> Void)? = nil,
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.rowHeight = rowHeight
        self.rowKey = rowKey
        self.scrollPosition = scrollPosition
        self.onScrollPositionChanged = onScrollPositionChanged
        self.isRefreshing = isRefreshing
        self.onRefresh = onRefresh
        self.onEndReached = onEndReached
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(
            rowCount: rowCount,
            rowHeight: rowHeight,
            rowKey: rowKey,
            scrollPosition: scrollPosition,
            onScrollPositionChanged: onScrollPositionChanged,
            onEndReached: onEndReached,
            rowContent: rowContent
        )
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = UITableView(frame: .zero, style: .plain)
        tableView.dataSource = context.coordinator
        tableView.delegate = context.coordinator
        nexaUpdateRefreshControl(
            tableView,
            controller: &context.coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        tableView.register(UITableViewCell.self, forCellReuseIdentifier: nexaFastListCellReuseIdentifier)
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
        context.coordinator.applyScrollPosition(to: tableView)
        return tableView
    }

    func updateUIView(_ tableView: UITableView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        let previousScrollPosition = coordinator.scrollPosition
        coordinator.rowCount = rowCount
        coordinator.rowHeight = rowHeight
        coordinator.rowKey = rowKey
        coordinator.scrollPosition = scrollPosition
        coordinator.onScrollPositionChanged = onScrollPositionChanged
        coordinator.onEndReached = onEndReached
        coordinator.rowContent = rowContent
        nexaUpdateRefreshControl(
            tableView,
            controller: &coordinator.refreshController,
            isRefreshing: isRefreshing,
            action: onRefresh
        )
        if previousRowCount != rowCount {
            coordinator.lastEndReachedRowCount = nil
            coordinator.applyScrollPosition(to: tableView)
        } else if previousScrollPosition != scrollPosition {
            coordinator.applyScrollPosition(to: tableView)
        }

        guard previousRowCount != rowCount else {
            let visibleRows = tableView.indexPathsForVisibleRows ?? []
            if !visibleRows.isEmpty {
                UIView.performWithoutAnimation {
                    tableView.reconfigureRows(at: visibleRows)
                }
            }
            return
        }
        tableView.reloadData()
    }

    final class Coordinator: NSObject, UITableViewDataSource, UITableViewDelegate {
        var rowCount: Int
        var rowHeight: CGFloat?
        var rowKey: ((Int) -> AnyHashable)?
        var scrollPosition: Int32?
        var onScrollPositionChanged: ((Int) -> Void)?
        var refreshController: NexaFastListRefreshController?
        var onEndReached: (() -> Void)?
        var lastEndReachedRowCount: Int?
        var lastReportedScrollPosition: Int?
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            rowHeight: CGFloat?,
            rowKey: ((Int) -> AnyHashable)?,
            scrollPosition: Int32?,
            onScrollPositionChanged: ((Int) -> Void)?,
            onEndReached: (() -> Void)?,
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.rowHeight = rowHeight
            self.rowKey = rowKey
            self.scrollPosition = scrollPosition
            self.onScrollPositionChanged = onScrollPositionChanged
            self.refreshController = nil
            self.onEndReached = onEndReached
            self.lastEndReachedRowCount = nil
            self.lastReportedScrollPosition = nil
            self.rowContent = rowContent
            super.init()
        }

        func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
            rowCount
        }

        func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
            let cell = tableView.dequeueReusableCell(withIdentifier: nexaFastListCellReuseIdentifier, for: indexPath)
            cell.selectionStyle = .none
            cell.contentConfiguration = UIHostingConfiguration {
                if let rowKey {
                    rowContent(indexPath.row).id(rowKey(indexPath.row))
                } else {
                    rowContent(indexPath.row)
                }
            }
            .margins(.all, 0)
            return cell
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            if let onScrollPositionChanged,
               let position = (scrollView as? UITableView)?.indexPathsForVisibleRows?.map(\.row).min(),
               lastReportedScrollPosition != position
            {
                lastReportedScrollPosition = position
                onScrollPositionChanged(position)
            }
            guard let onEndReached, rowCount > 0 else { return }
            let reachedEnd = (scrollView as? UITableView)?.indexPathsForVisibleRows?.contains {
                $0.row >= rowCount - 1
            } == true
            guard reachedEnd, lastEndReachedRowCount != rowCount else { return }
            lastEndReachedRowCount = rowCount
            onEndReached()
        }

        func applyScrollPosition(to tableView: UITableView) {
            guard let scrollPosition, rowCount > 0 else { return }
            let target = min(max(Int(scrollPosition), 0), rowCount - 1)
            guard (tableView.indexPathsForVisibleRows?.map(\.row).min() ?? -1) != target else {
                return
            }
            lastReportedScrollPosition = target
            tableView.scrollToRow(at: IndexPath(row: target, section: 0), at: .top, animated: false)
        }
    }
}

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
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.itemExtent = itemExtent
            self.rowKey = rowKey
            self.scrollPosition = scrollPosition
            self.onScrollPositionChanged = onScrollPositionChanged
            self.refreshController = nil
            self.onEndReached = onEndReached
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
            if let onScrollPositionChanged,
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.map(\.item).min(),
               lastReportedScrollPosition != position
            {
                lastReportedScrollPosition = position
                onScrollPositionChanged(position)
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
            subitem: item,
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
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.itemHeight = itemHeight
            self.rowKey = rowKey
            self.scrollPosition = scrollPosition
            self.onScrollPositionChanged = onScrollPositionChanged
            self.refreshController = nil
            self.onEndReached = onEndReached
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
            if let onScrollPositionChanged,
               let position = (scrollView as? UICollectionView)?.indexPathsForVisibleItems.map(\.item).min(),
               lastReportedScrollPosition != position
            {
                lastReportedScrollPosition = position
                onScrollPositionChanged(position)
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
"#,
    );
}
