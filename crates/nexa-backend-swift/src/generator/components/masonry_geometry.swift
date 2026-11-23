// Estimated cards sharing a height occupy a repeating order of columns. Store
// one band for that order and materialize attributes only for requested cards.
struct NexaMasonryGeometry {
    private struct ColumnStart {
        let column: Int
        let y: CGFloat
    }

    private struct Band {
        let firstItem: Int
        let count: Int
        // Each stored column start belongs to at least one distinct item.
        // The layout's Int32 row-count contract therefore bounds both fields.
        let columnOffset: Int32
        let columnLength: Int32
        let height: CGFloat
        let stride: CGFloat

        var end: Int { firstItem + count }
        var firstColumn: Int { Int(columnOffset) }
        var columnCount: Int { Int(columnLength) }
    }

    private struct Segment {
        let band: Int
        let slot: Int
        let count: Int
        let bottom: CGFloat
    }

    private struct Measurement {
        var item: Int
        let height: CGFloat
    }

    private let columns: Int
    private let spacing: CGFloat
    private let estimatedHeight: CGFloat = 120
    private var width: CGFloat = -1
    private var itemCount = 0
    private var columnWidth: CGFloat = 0
    private var bands: [Band] = []
    private var bandColumns: [ColumnStart] = []
    private var segments: [[Segment]]
    private var measurements: [Measurement] = []
    private var firstDirtyItem: Int?
    private(set) var contentSize: CGSize = .zero

    init(columns: Int, spacing: CGFloat) {
        self.columns = max(1, columns)
        self.spacing = max(0, spacing)
        segments = Array(repeating: [], count: self.columns)
    }

    mutating func recordHeight(_ height: CGFloat, for item: Int) {
        guard item >= 0, item < itemCount, height.isFinite else { return }
        let height = max(1, height)
        let index = Self.partition(measurements) { $0.item < item }
        let measurement = Measurement(item: item, height: height)
        if index < measurements.count, measurements[index].item == item {
            guard measurements[index].height != height else { return }
            measurements[index] = measurement
        } else {
            measurements.insert(measurement, at: index)
        }
        firstDirtyItem = min(firstDirtyItem ?? item, item)
    }

    mutating func moveItem(from source: Int, to destination: Int) {
        guard source >= 0, source < itemCount,
              destination >= 0, destination < itemCount,
              source != destination
        else { return }

        let firstChangedItem = min(source, destination)
        let sourceMeasurement = Self.partition(measurements) { $0.item < source }
        var movedMeasurement: Measurement?
        if sourceMeasurement < measurements.count, measurements[sourceMeasurement].item == source {
            movedMeasurement = measurements.remove(at: sourceMeasurement)
        }

        let lower = Self.partition(measurements) { $0.item < firstChangedItem }
        let upper = Self.partition(measurements) { $0.item <= max(source, destination) }
        let shift = source < destination ? -1 : 1
        for index in lower..<upper {
            measurements[index].item += shift
        }

        if var movedMeasurement {
            movedMeasurement.item = destination
            let insertion = Self.partition(measurements) { $0.item < destination }
            measurements.insert(movedMeasurement, at: insertion)
        }
        firstDirtyItem = min(firstDirtyItem ?? firstChangedItem, firstChangedItem)
    }

    mutating func prepare(width: CGFloat, count: Int) {
        guard firstDirtyItem != nil || self.width != width || itemCount != count else { return }
        let reset = self.width != width || count < itemCount
        var dirtyItem = firstDirtyItem ?? itemCount
        if reset {
            measurements.removeAll(keepingCapacity: count >= itemCount)
            if count < itemCount {
                bands.removeAll()
                bandColumns.removeAll()
                for column in segments.indices {
                    segments[column].removeAll()
                }
            }
            dirtyItem = 0
        } else if count > itemCount, let last = bands.last, last.height == estimatedHeight {
            // Merge incremental appends into the final estimated band.
            dirtyItem = min(dirtyItem, last.firstItem)
        }
        self.width = width
        itemCount = count
        firstDirtyItem = nil
        columnWidth = max(0, (width - spacing * CGFloat(columns - 1)) / CGFloat(columns))

        let firstBand = Self.partition(bands) { $0.end <= dirtyItem }
        var item = dirtyItem
        if firstBand < bands.count {
            item = bands[firstBand].firstItem
            bandColumns.removeSubrange(bands[firstBand].firstColumn...)
            bands.removeSubrange(firstBand...)
        }
        var bottoms: [CGFloat] = []
        bottoms.reserveCapacity(columns)
        for column in 0..<columns {
            let firstSegment = Self.partition(segments[column]) { $0.band < firstBand }
            segments[column].removeSubrange(firstSegment...)
            bottoms.append(segments[column].last?.bottom ?? 0)
        }

        let firstMeasurement = Self.partition(measurements) { $0.item < item }
        for measurementIndex in firstMeasurement..<measurements.count {
            let measurement = measurements[measurementIndex]
            appendEstimates(from: &item, to: measurement.item, bottoms: &bottoms)
            let column = bottoms.indices.min { bottoms[$0] < bottoms[$1] } ?? 0
            let y = bottoms[column] == 0 ? 0 : bottoms[column] + spacing
            appendBand(firstItem: item, count: 1, starts: CollectionOfOne(ColumnStart(column: column, y: y)),
                       height: measurement.height, bottoms: &bottoms)
            item += 1
        }
        appendEstimates(from: &item, to: count, bottoms: &bottoms)
        contentSize = CGSize(width: width, height: bottoms.max() ?? 0)
    }

    func frame(for item: Int) -> CGRect? {
        guard item >= 0, item < itemCount else { return nil }
        let band = bands[Self.partition(bands) { $0.end <= item }]
        let offset = item - band.firstItem
        return frame(in: band, slot: offset % band.columnCount, row: offset / band.columnCount)
    }

    func forEachFrame(in rect: CGRect, _ emit: (Int, CGRect) -> Void) {
        guard !rect.isEmpty else { return }
        for column in 0..<columns {
            let x = CGFloat(column) * (columnWidth + spacing)
            guard x < rect.maxX, x + columnWidth > rect.minX else { continue }
            let lane = segments[column]
            var index = Self.partition(lane) { $0.bottom <= rect.minY }
            while index < lane.count {
                let segment = lane[index]
                let band = bands[segment.band]
                let y = bandColumns[band.firstColumn + segment.slot].y
                guard y < rect.maxY else { break }
                let firstRow = max(0, min(CGFloat(segment.count),
                    ((rect.minY - y - band.height) / band.stride).rounded(.down) + 1))
                var row = Int(firstRow)
                while row < segment.count {
                    let frame = frame(in: band, slot: segment.slot, row: row)
                    guard frame.minY < rect.maxY else { break }
                    if frame.maxY > rect.minY {
                        emit(band.firstItem + segment.slot + row * band.columnCount, frame)
                    }
                    row += 1
                }
                index += 1
            }
        }
    }

    private func frame(in band: Band, slot: Int, row: Int) -> CGRect {
        let start = bandColumns[band.firstColumn + slot]
        return CGRect(x: CGFloat(start.column) * (columnWidth + spacing),
                      y: start.y + CGFloat(row) * band.stride,
                      width: columnWidth, height: band.height)
    }

    private mutating func appendEstimates(from item: inout Int, to limit: Int, bottoms: inout [CGFloat]) {
        guard item < limit else { return }
        let stride = estimatedHeight + spacing
        let empty = bottoms.indices.filter { bottoms[$0] == 0 }
        let starts = empty.prefix(limit - item).map { ColumnStart(column: $0, y: 0) }
        if !starts.isEmpty {
            appendBand(firstItem: item, count: starts.count, starts: starts,
                       height: estimatedHeight, bottoms: &bottoms)
            item += starts.count
        }

        while item < limit {
            let order = bottoms.indices.sorted {
                bottoms[$0] == bottoms[$1] ? $0 < $1 : bottoms[$0] < bottoms[$1]
            }
            let leadingBottom = bottoms[order[0]]
            let active = order.prefix { bottoms[$0] - leadingBottom < stride }
            let remaining = limit - item
            var cycles = remaining / active.count
            if active.count < columns, cycles > 0 {
                // Place complete cycles until another column can become shortest.
                let nextBottom = bottoms[order[active.count]]
                let currentBottom = bottoms[order[active.count - 1]]
                let beforeNext = max(1, ((nextBottom - currentBottom) / stride).rounded(.up))
                cycles = Int(min(CGFloat(cycles), beforeNext))
            }
            let count = cycles > 0 ? cycles * active.count : remaining
            let starts = active.prefix(count).map { ColumnStart(column: $0, y: bottoms[$0] + spacing) }
            appendBand(firstItem: item, count: count, starts: starts,
                       height: estimatedHeight, bottoms: &bottoms)
            item += count
        }
    }

    private mutating func appendBand<Starts: Collection>(
        firstItem: Int, count: Int, starts: Starts, height: CGFloat, bottoms: inout [CGFloat]
    ) where Starts.Element == ColumnStart {
        let stride = height + spacing
        let bandIndex = bands.count
        let columnCount = starts.count
        bands.append(Band(firstItem: firstItem, count: count,
                          columnOffset: Int32(bandColumns.count), columnLength: Int32(columnCount),
                          height: height, stride: stride))
        bandColumns.append(contentsOf: starts)
        for (slot, start) in starts.enumerated() {
            let rows = (count - 1 - slot) / columnCount + 1
            let bottom = start.y + CGFloat(rows - 1) * stride + height
            bottoms[start.column] = bottom
            segments[start.column].append(Segment(band: bandIndex, slot: slot, count: rows, bottom: bottom))
        }
    }

    private static func partition<T>(_ elements: [T], before: (T) -> Bool) -> Int {
        var low = 0
        var high = elements.count
        while low < high {
            let middle = low + (high - low) / 2
            if before(elements[middle]) {
                low = middle + 1
            } else {
                high = middle
            }
        }
        return low
    }
}
