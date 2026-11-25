import QuartzCore
import UIKit

// Injected into the benchmark host only. A display-link interval measures
// callback cadence, which can reveal missed updates during a scroll.
@MainActor
final class IconFastListFrameProbe: NSObject {
    static let shared = IconFastListFrameProbe()
    private var link: CADisplayLink?
    private var origin: CFTimeInterval?
    private var timestamps: [Double] = []

    func start() {
        guard link == nil, origin == nil else { return }
        let link = CADisplayLink(target: self, selector: #selector(tick(_:)))
        self.link = link
        link.add(to: .main, forMode: .common)
    }

    @objc private func tick(_ link: CADisplayLink) {
        let origin = origin ?? link.timestamp
        self.origin = origin
        timestamps.append(link.timestamp)
        if link.timestamp - origin >= 16 {
            let clockOffset = Date().timeIntervalSince1970 - CACurrentMediaTime()
            let samples = timestamps.map { ($0 + clockOffset) * 1000 }
            link.invalidate()
            self.link = nil
            let file = URL(fileURLWithPath: NSTemporaryDirectory())
                .appendingPathComponent("icon-fastlist-frames.json")
            do {
                let data = try JSONEncoder().encode(samples)
                try data.write(to: file, options: .atomic)
            } catch {
                NSLog("IconFastList probe could not write frame timings: %@", String(describing: error))
            }
        }
    }
}
