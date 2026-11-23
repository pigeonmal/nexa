@MainActor
private final class FixedGridData: NSObject, UICollectionViewDataSource {
    var count = 0

    func collectionView(_ view: UICollectionView, numberOfItemsInSection section: Int) -> Int { count }

    func collectionView(_ view: UICollectionView, cellForItemAt path: IndexPath) -> UICollectionViewCell {
        view.dequeueReusableCell(withReuseIdentifier: "cell", for: path)
    }
}

@MainActor
private final class FixedGridInsetView: UICollectionView {
    override var safeAreaInsets: UIEdgeInsets { UIEdgeInsets(top: 0, left: 10, bottom: 0, right: 20) }
}

@MainActor
private func runFixedGridChecks() -> [String: Int] {
    let data = FixedGridData()
    let layout = NexaFastFixedGridLayout(columns: 1, minCellWidth: 100, itemHeight: 40, spacing: 10)
    let view = FixedGridInsetView(frame: CGRect(x: 0, y: 0, width: 350, height: 200), collectionViewLayout: layout)
    view.contentInsetAdjustmentBehavior = .never
    view.contentInset = UIEdgeInsets(top: 5, left: 2, bottom: 5, right: 3)
    view.dataSource = data
    view.register(UICollectionViewCell.self, forCellWithReuseIdentifier: "cell")
    defer { withExtendedLifetime(data) {} }

    func update(count: Int) {
        data.count = count
        view.reloadData()
        view.layoutIfNeeded()
        layout.prepare()
    }

    update(count: 0)
    verify(layout.collectionViewContentSize.height == 0, "Empty grid must have no trailing gap")
    update(count: 8)
    verify(layout.collectionViewContentSize == CGSize(width: 350, height: 140), "Grid must preserve full scroll width and omit trailing spacing")
    verify(layout.layoutAttributesForItem(at: IndexPath(item: 7, section: 0))?.frame ==
        CGRect(x: 120, y: 100, width: 100, height: 40), "Last partial grid line must honor safe area and spacing")
    let rect = CGRect(x: 125, y: 51, width: 30, height: 12)
    verify(layout.layoutAttributesForElements(in: rect)?.map(\.indexPath.item) == [4], "Query should only visit intersecting columns and rows")
    let gap = CGRect(x: 0, y: 42, width: 320, height: 6)
    verify(layout.layoutAttributesForElements(in: gap)?.isEmpty == true, "Item spacing must not return neighboring cells")

    view.frame.size.width = 240
    view.layoutIfNeeded()
    layout.prepare()
    verify(layout.collectionViewContentSize == CGSize(width: 240, height: 190), "Adaptive grid should recompute columns on resize")

    update(count: 1_000_000)
    let farRect = CGRect(x: 0, y: 20_000_001, width: 210, height: 30)
    let farItems = layout.layoutAttributesForElements(in: farRect)?.map(\.indexPath.item)
    verify(farItems == [800_000, 800_001], "Grid should resolve distant windows without constructing earlier lines")
    update(count: 1)
    verify(layout.layoutAttributesForElements(in: farRect)?.isEmpty == true, "Shrinking a grid must discard obsolete geometry")
    verify(layout.layoutAttributesForItem(at: IndexPath(item: 1, section: 0)) == nil, "Out-of-range grid lookup must return nil")
    return ["largeGridItems": 1_000_000, "distantWindowAttributes": farItems?.count ?? 0]
}
