// UIKit's display callbacks own this registry. Track loaded headers directly,
// including pinned and zero-height headers, instead of scanning every section.
@MainActor
private final class NexaSectionHeaderRegistry {
    private struct Entry {
        let section: Int
        weak var view: UITableViewHeaderFooterView?
    }
    private var entries: [Entry] = []

    private func insertionIndex(for section: Int) -> Int {
        var low = 0
        var high = entries.count
        while low < high {
            let middle = low + (high - low) / 2
            if entries[middle].section < section { low = middle + 1 }
            else { high = middle }
        }
        return low
    }

    func register(_ view: UITableViewHeaderFooterView, section: Int) {
        entries.removeAll { $0.view == nil }
        let index = insertionIndex(for: section)
        let entry = Entry(section: section, view: view)
        if index < entries.count, entries[index].section == section { entries[index] = entry }
        else { entries.insert(entry, at: index) }
    }

    func remove(_ view: UIView, section: Int) {
        let index = insertionIndex(for: section)
        guard index < entries.count, entries[index].section == section,
              entries[index].view === view else { return }
        entries.remove(at: index)
    }

    func forEach(_ update: (Int, UITableViewHeaderFooterView) -> Void) {
        // The array preserves section order without sorting or allocating a
        // collection on each refresh. Weak references do not retain UIKit views.
        for entry in entries {
            if let view = entry.view { update(entry.section, view) }
        }
    }

    func removeAll() { entries.removeAll(keepingCapacity: true) }
}
