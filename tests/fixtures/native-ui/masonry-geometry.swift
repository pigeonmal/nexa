func verifyGeometry(
    _ geometry: NexaMasonryGeometry,
    count: Int,
    columns: Int,
    width: CGFloat,
    spacing: CGFloat,
    measurements: [Int: CGFloat]
) {
    var bottoms = Array(repeating: CGFloat(0), count: columns)
    let columnWidth = max(0, (width - spacing * CGFloat(columns - 1)) / CGFloat(columns))
    let frames = (0..<count).map { index -> CGRect in
        let column = bottoms.indices.min { bottoms[$0] < bottoms[$1] } ?? 0
        let height = measurements[index] ?? 120
        let y = bottoms[column] == 0 ? 0 : bottoms[column] + spacing
        bottoms[column] = y + height
        return CGRect(x: CGFloat(column) * (columnWidth + spacing), y: y, width: columnWidth, height: height)
    }
    precondition(geometry.contentSize == CGSize(width: width, height: bottoms.max() ?? 0))
    precondition(geometry.frame(for: -1) == nil)
    precondition(geometry.frame(for: count) == nil)
    for (index, frame) in frames.enumerated() {
        precondition(geometry.frame(for: index) == frame, "incorrect frame at \(index)")
    }
    for rect in [
        CGRect(x: 0, y: 0, width: width, height: 1000),
        CGRect(x: 0, y: 120, width: width, height: 60),
        CGRect(x: columnWidth + spacing, y: 1200, width: columnWidth, height: 400),
        CGRect(x: 0, y: max(0, geometry.contentSize.height - 900), width: width, height: 1000),
    ] {
        var visible: [Int: CGRect] = [:]
        geometry.forEachFrame(in: rect) { index, frame in
            precondition(visible.updateValue(frame, forKey: index) == nil)
        }
        let expected = Dictionary(uniqueKeysWithValues: frames.enumerated().compactMap { index, frame in
            frame.minX < rect.maxX && frame.maxX > rect.minX
                && frame.minY < rect.maxY && frame.maxY > rect.minY ? (index, frame) : nil
        })
        precondition(visible == expected, "visible frames differ at \(rect)")
    }
}

for columns in [1, 2, 3, 5, 11] {
    for spacing: CGFloat in [0, 0.125, 8, 63.75] {
        let width: CGFloat = 600
        var geometry = NexaMasonryGeometry(columns: columns, spacing: spacing)
        var measurements: [Int: CGFloat] = [:]
        var count = 600
        geometry.prepare(width: width, count: count)
        verifyGeometry(geometry, count: count, columns: columns, width: width, spacing: spacing, measurements: measurements)
        for step in 0..<90 {
            let item = (step * 137 + 17) % count
            let height = CGFloat((step * 239 + 103) % 830 + 1) + 0.375
            measurements[item] = height
            geometry.recordHeight(height, for: item)
            if step % 7 == 0 {
                geometry.prepare(width: width, count: count)
                verifyGeometry(geometry, count: count, columns: columns, width: width, spacing: spacing, measurements: measurements)
            }
        }
        count += 130
        geometry.prepare(width: width, count: count)
        verifyGeometry(geometry, count: count, columns: columns, width: width, spacing: spacing, measurements: measurements)
        geometry.prepare(width: width / 2, count: count)
        verifyGeometry(geometry, count: count, columns: columns, width: width / 2, spacing: spacing, measurements: [:])
        geometry.prepare(width: width / 2, count: 2)
        verifyGeometry(geometry, count: 2, columns: columns, width: width / 2, spacing: spacing, measurements: [:])
        geometry.prepare(width: width / 2, count: 0)
        verifyGeometry(geometry, count: 0, columns: columns, width: width / 2, spacing: spacing, measurements: [:])
    }
}

var large = NexaMasonryGeometry(columns: 3, spacing: 8)
large.prepare(width: 316, count: 1_000_000)
let last = large.frame(for: 999_999)
precondition(last == CGRect(x: 0, y: 333_333 * 128, width: 100, height: 120))
var lastItems: [Int] = []
large.forEachFrame(in: CGRect(x: 0, y: 333_333 * 128, width: 316, height: 120)) { item, _ in
    lastItems.append(item)
}
precondition(lastItems == [999_999])
