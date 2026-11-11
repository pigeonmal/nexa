// Deterministic UIKit inputs exercise the exact emitted coordinator method.
// No window, screenshot or foreground-app interaction is required.
@MainActor
private final class VisibilityTable: UITableView {
    var paths: [IndexPath] = []
    var pinned: UITableViewHeaderFooterView?
    var geometryQueries = 0
    override var indexPathsForVisibleRows: [IndexPath]? { paths.isEmpty ? nil : paths }
    override func headerView(forSection section: Int) -> UITableViewHeaderFooterView? { pinned }
    override func rectForRow(at path: IndexPath) -> CGRect {
        geometryQueries += 1
        return CGRect(x: 0, y: CGFloat(path.row) * 44, width: 320, height: 44)
    }
}

@MainActor
private func referenceLeading(_ table: UITableView, reverse: Bool) -> Int? {
    let paths = table.indexPathsForVisibleRows ?? []
    guard !reverse, let header = table.headerView(forSection: 0) else {
        let positions = paths.map(\.row)
        return reverse ? positions.max() : positions.min()
    }
    let top = max(table.bounds.minY + table.adjustedContentInset.top, header.frame.maxY)
    return paths.compactMap { path in
        table.rectForRow(at: path).minY >= top - 1 ? path.row : nil
    }.min()
}

@MainActor
private func runVisibilityChecks() -> [String: Any] {
    let table = VisibilityTable(frame: CGRect(x: 0, y: 0, width: 320, height: 480), style: .plain)
    table.contentInsetAdjustmentBehavior = .never
    let header = UITableViewHeaderFooterView(reuseIdentifier: "visibility")
    var cases = 0
    for count in [0, 1, 2, 7, 40] {
        // Deliberately unsorted: UIKit collection-view ordering is not promised.
        table.paths = (0..<count).reversed().map { IndexPath(row: $0 * 2 + 1, section: 0) }
        let expected = table.paths.map(\.item).min()
        verify(collectionLeading(table.paths) == expected, "Collection leading index changed")
        for reverse in [false, true] {
            let probe = VisibilityProbe(reverseLayout: reverse)
            for pinned in [false, true] {
                table.pinned = pinned ? header : nil
                for top in [CGFloat(-44), 0, 44, 45, 440, 4400] {
                    header.frame = CGRect(x: 0, y: top - 28, width: 320, height: 28)
                    for inset in [CGFloat(0), 20, 80] {
                        table.contentInset.top = inset
                        for offset in [CGFloat(-80), 0, 66, 440] {
                            table.contentOffset.y = offset
                            table.geometryQueries = 0
                            let before = referenceLeading(table, reverse: reverse)
                            let oldQueries = table.geometryQueries
                            table.geometryQueries = 0
                            let after = probe.leadingVisibleRowPosition(in: table)
                            verify(before == after, "Sticky/reverse leading index changed")
                            verify(table.geometryQueries == oldQueries, "Lazy evaluation must not repeat geometry queries")
                            cases += 1
                        }
                    }
                }
            }
        }
    }
    table.paths = (0..<40).reversed().map { IndexPath(row: $0, section: 0) }
    table.contentOffset = .zero
    table.contentInset = .zero
    let probe = VisibilityProbe(reverseLayout: false)
    var samples: [[String: Any]] = []
    let iterations = 10000
    var checksum = 0
    for pinned in [false, true] {
        table.pinned = pinned ? header : nil
        header.frame = CGRect(x: 0, y: 0, width: 320, height: 88)
        var referenceMs: [Double] = []
        var optimizedMs: [Double] = []
        for pass in 0..<9 {
            // Alternate order to reduce systematic warm-up and thermal bias.
            func measure(_ optimized: Bool) -> Double {
                let start = CACurrentMediaTime()
                for _ in 0..<iterations {
                    let position = optimized ? probe.leadingVisibleRowPosition(in: table) : referenceLeading(table, reverse: false)
                    checksum += position ?? -1
                }
                return (CACurrentMediaTime() - start) * 1000
            }
            if pass % 2 == 0 {
                referenceMs.append(measure(false)); optimizedMs.append(measure(true))
            } else {
                optimizedMs.append(measure(true)); referenceMs.append(measure(false))
            }
        }
        samples.append(["pinnedHeader": pinned, "visibleRows": table.paths.count,
                        "iterationsPerSample": iterations, "referenceMs": referenceMs, "optimizedMs": optimizedMs])
    }
    return ["equivalenceCases": cases, "checksum": checksum, "tableSamples": samples,
            "scope": "Emitted visible-index scan with deterministic UIKit inputs; excludes UIKit snapshot creation and complete scroll frame"]
}
