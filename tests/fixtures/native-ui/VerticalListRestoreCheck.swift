@MainActor
private final class VerticalListRestoreTable: UITableView {
    var scrolledRows: [Int] = []

    override func scrollToRow(at path: IndexPath, at position: UITableView.ScrollPosition, animated: Bool) {
        scrolledRows.append(path.row)
    }
}

@MainActor
private func runVerticalListRestoreChecks() {
    let data = VerticalListRestoreProbe()
    let view = VerticalListRestoreTable(frame: CGRect(x: 0, y: 0, width: 350, height: 300), style: .plain)
    let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 400, height: 800))
    window.addSubview(view)
    view.dataSource = data
    view.reloadData()

    // Let the same queued work used by the emitted UIKit coordinator run.
    func settle() {
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.5))
    }

    data.restoreBoundScrollPositionIfNeeded(in: view, revision: 1)
    view.dataSource = nil
    view.reloadData()
    verify(view.window != nil, "A dismantled table can still belong to the pager's window")
    settle()
    verify(view.scrolledRows.isEmpty, "A deferred scroll must reject a retained table that lost its data source")

    view.dataSource = data
    view.reloadData()
    data.restoreBoundScrollPositionIfNeeded(in: view, revision: 1)
    data.cancelPendingScrollPositionRestore()
    settle()
    verify(view.scrolledRows.isEmpty, "Cancelled scroll work must not touch the table")

    data.restoreBoundScrollPositionIfNeeded(in: view, revision: 1)
    data.scrollPosition = 3
    data.restoreRevision = 2
    data.restoreBoundScrollPositionIfNeeded(in: view, revision: 2)
    data.rowCount = 8
    data.nativeRowCount = 8
    view.reloadData()
    settle()
    verify(view.scrolledRows == [3], "A reverse scroll must use its source row index")

    data.nativeRowCount = 0
    view.reloadData()
    data.applyScrollPosition(to: view, force: true)
    verify(view.scrolledRows == [3], "An empty native table must reject a stale model row")

    data.rowCount = 8
    data.nativeRowCount = 8
    data.scrollPosition = 3
    data.restoreRevision = 3
    view.reloadData()
    data.restoreBoundScrollPositionIfNeeded(in: view, revision: 3)
    data.nativeRowCount = 0
    view.reloadData()
    settle()
    verify(view.scrolledRows == [3], "A delayed restore must reject a row removed after it was queued")
}
