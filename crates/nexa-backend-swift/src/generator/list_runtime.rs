pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
private let nexaFastListCellReuseIdentifier = "NexaFastListCell"

@available(iOS 16.0, *)
private struct NexaFastList<RowContent: View>: UIViewRepresentable {
    let rowCount: Int
    let rowKey: ((Int) -> AnyHashable)?
    let rowContent: (Int) -> RowContent

    init(
        rowCount: Int,
        rowKey: ((Int) -> AnyHashable)? = nil,
        @ViewBuilder rowContent: @escaping (Int) -> RowContent
    ) {
        self.rowCount = max(0, rowCount)
        self.rowKey = rowKey
        self.rowContent = rowContent
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(rowCount: rowCount, rowKey: rowKey, rowContent: rowContent)
    }

    func makeUIView(context: Context) -> UITableView {
        let tableView = UITableView(frame: .zero, style: .plain)
        tableView.dataSource = context.coordinator
        tableView.register(UITableViewCell.self, forCellReuseIdentifier: nexaFastListCellReuseIdentifier)
        tableView.rowHeight = UITableView.automaticDimension
        tableView.estimatedRowHeight = 44
        tableView.allowsSelection = false
        tableView.backgroundColor = .clear
        tableView.reloadData()
        return tableView
    }

    func updateUIView(_ tableView: UITableView, context: Context) {
        let coordinator = context.coordinator
        let previousRowCount = coordinator.rowCount
        coordinator.rowCount = rowCount
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
        var rowKey: ((Int) -> AnyHashable)?
        var rowContent: (Int) -> RowContent

        init(
            rowCount: Int,
            rowKey: ((Int) -> AnyHashable)?,
            rowContent: @escaping (Int) -> RowContent
        ) {
            self.rowCount = rowCount
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
"#,
    );
}
