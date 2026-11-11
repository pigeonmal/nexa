@MainActor
private func verify(_ condition: @autoclosure () -> Bool, _ message: String = "Native header regression") {
    guard condition() else {
        FileHandle.standardError.write(Data((message + "\n").utf8))
        exit(1)
    }
}

extension NexaSectionHeaderRegistry {
    var testTrackedCount: Int { entries.count }
}

@MainActor
private final class TableData: NSObject, UITableViewDataSource, UITableViewDelegate {
    let sections: Int
    var revision = 0
    let sectionHeaders = NexaSectionHeaderRegistry()
    init(sections: Int) { self.sections = sections }
    func numberOfSections(in tableView: UITableView) -> Int { sections }
    func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int { section % 4 == 0 ? 0 : 1 }
    func tableView(_ tableView: UITableView, heightForHeaderInSection section: Int) -> CGFloat { section % 8 == 0 ? 0 : 28 }
    func tableView(_ tableView: UITableView, heightForFooterInSection section: Int) -> CGFloat { .leastNormalMagnitude }
    func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
        tableView.dequeueReusableCell(withIdentifier: "row", for: indexPath)
    }
    func tableView(_ tableView: UITableView, willDisplayHeaderView view: UIView, forSection section: Int) {
        guard let header = view as? UITableViewHeaderFooterView else { return }
        sectionHeaders.register(header, section: section)
    }
    func tableView(_ tableView: UITableView, didEndDisplayingHeaderView view: UIView, forSection section: Int) {
        sectionHeaders.remove(view, section: section)
    }
    func title(_ section: Int) -> String { "Library group \(section), revision \(revision)" }
    func tableView(_ tableView: UITableView, viewForHeaderInSection section: Int) -> UIView? {
        guard section % 8 != 0 else { return nil }
        let header = tableView.dequeueReusableHeaderFooterView(withIdentifier: "header")
        header?.textLabel?.text = title(section)
        return header
    }
}

@MainActor
private func checkTable(style: UITableView.Style, count: Int, benchmark: Bool) -> [String: Any] {
    let data = TableData(sections: count)
    let table = UITableView(frame: CGRect(x: 0, y: 0, width: 320, height: 480), style: style)
    table.register(UITableViewCell.self, forCellReuseIdentifier: "row")
    table.register(UITableViewHeaderFooterView.self, forHeaderFooterViewReuseIdentifier: "header")
    table.rowHeight = 44
    table.estimatedRowHeight = 0
    table.estimatedSectionHeaderHeight = 0
    table.estimatedSectionFooterHeight = 0
    table.sectionHeaderTopPadding = 0
    table.contentInset = UIEdgeInsets(top: 20, left: 0, bottom: 35, right: 0)
    table.contentInsetAdjustmentBehavior = .never
    table.dataSource = data
    table.delegate = data
    let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 440, height: 956))
    window.addSubview(table)
    defer { withExtendedLifetime(window) {} }
    table.reloadData()
    table.layoutIfNeeded()
    var headerQueries = 0
    func refresh(_ optimized: Bool) -> [Int] {
        var updated: [Int] = []
        func visit(_ section: Int) {
            headerQueries += 1
            guard let header = table.headerView(forSection: section) else { return }
            updated.append(section)
            header.textLabel?.text = data.title(section)
        }
        if optimized {
            data.sectionHeaders.forEach { section, header in
                updated.append(section)
                header.textLabel?.text = data.title(section)
            }
        } else {
            for section in 0..<table.numberOfSections { visit(section) }
        }
        return updated
    }
    for offset in [-80, 0, 79, 120, Int(table.contentSize.height / 2), max(0, Int(table.contentSize.height - 480))] {
        table.setContentOffset(CGPoint(x: 0, y: offset), animated: false)
        table.layoutIfNeeded()
        let expected = refresh(false)
        let actual = refresh(true)
        verify(data.sectionHeaders.testTrackedCount == actual.count, "Displayed-header storage must not accumulate historical sections: entries=\(data.sectionHeaders.testTrackedCount), live=\(actual.count), offset=\(offset)")
        verify(actual == expected, "Visible header mismatch: style=\(style), offset=\(offset), expected=\(expected), actual=\(actual)")
        data.revision += 1
        _ = refresh(true)
        for section in actual {
            verify(table.headerView(forSection: section)?.textLabel?.text == data.title(section))
        }
    }
    data.sectionHeaders.removeAll()
    data.revision += 1
    table.reloadData()
    table.layoutIfNeeded()
    verify(refresh(true) == refresh(false), "Reload must repopulate the displayed-header registry")

    // Returning to a previously offscreen section must use the latest content.
    table.setContentOffset(CGPoint(x: 0, y: -20), animated: false)
    table.layoutIfNeeded()
    for section in 0..<table.numberOfSections {
        guard let header = table.headerView(forSection: section), header.frame.intersects(table.bounds) else { continue }
        verify(header.textLabel?.text == data.title(section), "Returning headers must receive current content through the delegate")
    }
    var referenceSamples: [Double] = []
    var optimizedSamples: [Double] = []
    if benchmark {
        for _ in 0..<3 { _ = refresh(false); _ = refresh(true) }
        for _ in 0..<7 {
            var start = CACurrentMediaTime()
            for _ in 0..<10 { _ = refresh(false) }
            referenceSamples.append((CACurrentMediaTime() - start) * 1000)
            start = CACurrentMediaTime()
            for _ in 0..<10 { _ = refresh(true) }
            optimizedSamples.append((CACurrentMediaTime() - start) * 1000)
        }
    }
    headerQueries = 0
    _ = refresh(false)
    let oldQueries = headerQueries
    headerQueries = 0
    let visible = refresh(true)
    let result: [String: Any] = ["sections": count, "referenceHeaderQueries": oldQueries,
        "optimizedHeaderQueries": headerQueries, "registeredHeaders": visible.count,
        "referenceMs": referenceSamples, "optimizedMs": optimizedSamples, "refreshesPerSample": 10]
    table.removeFromSuperview()
    return result
}

@main struct Check {
    @MainActor static func main() {
        let registry = NexaSectionHeaderRegistry()
        let views = (0..<4).map { _ in UITableViewHeaderFooterView(reuseIdentifier: "probe") }
        defer { withExtendedLifetime(views) {} }
        for (section, view) in zip([9, 1, 7, 2], views) { registry.register(view, section: section) }
        var order: [Int] = []
        registry.forEach { section, _ in order.append(section) }
        verify(order == [1, 2, 7, 9], "Refresh order must be stable")
        registry.remove(views[2], section: 7)
        let replacement = UITableViewHeaderFooterView(reuseIdentifier: "replacement")
        replacement.frame = .zero
        registry.register(replacement, section: 1)
        registry.remove(views[1], section: 1)
        var replacements = 0
        registry.forEach { section, header in
            if section == 1 { verify(header === replacement); replacements += 1 }
        }
        verify(replacements == 1, "Late removal must not detach a replacement or skip a collapsed header")
        weak var probe: UITableViewHeaderFooterView?
        autoreleasepool {
            let temporary = UITableViewHeaderFooterView(reuseIdentifier: "temporary")
            probe = temporary
            registry.register(temporary, section: 30)
        }
        verify(probe == nil, "Tracking must not retain header views")
        registry.register(replacement, section: 1)
        verify(registry.testTrackedCount == 3, "Registration should prune expired weak entries")
        registry.removeAll()
        registry.forEach { _, _ in preconditionFailure("Cleared headers must not be visited") }
        let results = [checkTable(style: .plain, count: 20000, benchmark: true), checkTable(style: .grouped, count: 1000, benchmark: false)]
        let output: [String: Any] = ["tables": results, "registryChecks": "ordering, replacement identity, collapsed views, weak ownership, removal", "visibility": runVisibilityChecks()]
        print(String(data: try! JSONSerialization.data(withJSONObject: output, options: [.sortedKeys]), encoding: .utf8)!)
    }
}
