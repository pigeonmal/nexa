pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
private let nexaFastListCellReuseIdentifier = "NexaFastListCell"

@available(iOS 16.0, *)
private struct NexaFastList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let rowHeight: CGFloat?
    let rowKey: ((Int) -> AnyHashable)?
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        rowHeight: CGFloat? = nil,
        rowKey: ((Int) -> AnyHashable)? = nil,
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.rowHeight = rowHeight
        self.rowKey = rowKey
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(rowCount: rowCount, rowHeight: rowHeight, rowKey: rowKey, rowContent: rowContent)
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = UITableView(frame: .zero, style: .plain)
        tableView.dataSource = context.coordinator
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
        return tableView
    }

    func updateUIView(_ tableView: UITableView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        coordinator.rowCount = rowCount
        coordinator.rowHeight = rowHeight
        coordinator.rowKey = rowKey
        coordinator.rowContent = rowContent

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

    final class Coordinator: NSObject, UITableViewDataSource {
        var rowCount: Int
        var rowHeight: CGFloat?
        var rowKey: ((Int) -> AnyHashable)?
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            rowHeight: CGFloat?,
            rowKey: ((Int) -> AnyHashable)?,
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.rowHeight = rowHeight
            self.rowKey = rowKey
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
    }
}

private let nexaFastHorizontalListCellReuseIdentifier = "NexaFastHorizontalListCell"

@available(iOS 16.0, *)
private struct NexaFastHorizontalList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let itemExtent: CGFloat?
    let rowKey: ((Int) -> AnyHashable)?
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        itemExtent: CGFloat? = nil,
        rowKey: ((Int) -> AnyHashable)? = nil,
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.itemExtent = itemExtent
        self.rowKey = rowKey
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(rowCount: rowCount, itemExtent: itemExtent, rowKey: rowKey, rowContent: rowContent)
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
        collectionView.register(
            UICollectionViewCell.self,
            forCellWithReuseIdentifier: nexaFastHorizontalListCellReuseIdentifier
        )
        collectionView.alwaysBounceHorizontal = true
        collectionView.showsHorizontalScrollIndicator = true
        collectionView.backgroundColor = .clear
        collectionView.reloadData()
        return collectionView
    }

    func updateUIView(_ collectionView: UICollectionView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        coordinator.rowCount = rowCount
        coordinator.itemExtent = itemExtent
        coordinator.rowKey = rowKey
        coordinator.rowContent = rowContent

        guard previousRowCount != rowCount else {
            let visibleItems = collectionView.indexPathsForVisibleItems
            if !visibleItems.isEmpty {
                collectionView.reloadItems(at: visibleItems)
            }
            return
        }
        collectionView.reloadData()
    }

    final class Coordinator: NSObject, UICollectionViewDataSource {
        var rowCount: Int
        var itemExtent: CGFloat?
        var rowKey: ((Int) -> AnyHashable)?
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            itemExtent: CGFloat?,
            rowKey: ((Int) -> AnyHashable)?,
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.itemExtent = itemExtent
            self.rowKey = rowKey
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
    }
}

private let nexaFastGridListCellReuseIdentifier = "NexaFastGridListCell"

@available(iOS 16.0, *)
private struct NexaFastGridList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let columns: Int
    let itemHeight: CGFloat?
    let rowKey: ((Int) -> AnyHashable)?
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        columns: Int,
        itemHeight: CGFloat? = nil,
        rowKey: ((Int) -> AnyHashable)? = nil,
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.columns = max(1, columns)
        self.itemHeight = itemHeight
        self.rowKey = rowKey
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(rowCount: rowCount, itemHeight: itemHeight, rowKey: rowKey, rowContent: rowContent)
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
        collectionView.register(
            UICollectionViewCell.self,
            forCellWithReuseIdentifier: nexaFastGridListCellReuseIdentifier
        )
        collectionView.alwaysBounceVertical = true
        collectionView.showsVerticalScrollIndicator = true
        collectionView.backgroundColor = .clear
        collectionView.reloadData()
        return collectionView
    }

    func updateUIView(_ collectionView: UICollectionView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        coordinator.rowCount = rowCount
        coordinator.itemHeight = itemHeight
        coordinator.rowKey = rowKey
        coordinator.rowContent = rowContent

        guard previousRowCount != rowCount else {
            let visibleItems = collectionView.indexPathsForVisibleItems
            if !visibleItems.isEmpty {
                collectionView.reloadItems(at: visibleItems)
            }
            return
        }
        collectionView.reloadData()
    }

    final class Coordinator: NSObject, UICollectionViewDataSource {
        var rowCount: Int
        var itemHeight: CGFloat?
        var rowKey: ((Int) -> AnyHashable)?
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            itemHeight: CGFloat?,
            rowKey: ((Int) -> AnyHashable)?,
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
            self.itemHeight = itemHeight
            self.rowKey = rowKey
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
    }
}
"#,
    );
}
