// Appended to the real generated UIKit primitive by native_text_reveal_check.py.
// Everything renders into private in-memory bitmaps; no screen is captured.
private struct Segment {
    let text: String
    let start: Double
    let end: Double
}

extension NexaTextRevealView {
    var testLineCount: Int { lineTimings.count }
    var testHasDisplayLink: Bool { displayLink != nil }
    var testTimingStorage: UInt { lineTimings.withUnsafeBufferPointer { UInt(bitPattern: $0.baseAddress) } }
    var testBaseColor: UIColor? {
        textStorage.length > 0 ? textStorage.attribute(.foregroundColor, at: 0, effectiveRange: nil) as? UIColor : nil
    }

    // Independent reference renderer: the previous per-line clipping behavior.
    // Keep its rectangular clips to detect even one-channel rounding changes.
    func drawReference() {
        rebuildTextIfNeeded()
        rebuildLineTimingsIfNeeded()
        let range = NSRange(location: 0, length: layoutManager.numberOfGlyphs)
        guard range.length > 0 else { return }
        layoutManager.drawBackground(forGlyphRange: range, at: .zero)
        layoutManager.drawGlyphs(forGlyphRange: range, at: .zero)
        guard currentActive, let context = UIGraphicsGetCurrentContext() else { return }
        let full = !currentEnabled || segmentSnapshots.isEmpty || lineTimings.isEmpty
        for timing in lineTimings {
            let progress = full ? CGFloat(1) : nexaTextRevealProgress(timing, position: currentPosition)
            guard progress > 0 else { continue }
            let width = timing.rect.width * progress
            let x = timing.isRightToLeft ? timing.rect.maxX - width : timing.rect.minX
            context.saveGState()
            context.clip(to: CGRect(x: x, y: timing.rect.minY, width: width, height: timing.rect.height))
            highlightLayoutManager.drawGlyphs(forGlyphRange: NSRange(location: 0, length: highlightLayoutManager.numberOfGlyphs), at: .zero)
            context.restoreGState()
        }
    }
}

@MainActor
private func checkCaption(repetitions: Int, width: Int, rtl: Bool, lineHeight: CGFloat,
                          alignment: String, italic: Bool, decorations: Bool,
                          alpha: CGFloat, benchmark: Bool, scale: Int = 1,
                          phraseOverride: String? = nil) -> [String: Any] {
    FileHandle.standardError.write(Data("Checking count=\(repetitions) width=\(width) rtl=\(rtl) height=\(lineHeight) alignment=\(alignment) alpha=\(alpha)\n".utf8))
    let phrase = phraseOverride ?? (rtl ? "استمع إلى الموسيقى واكتشف الفنانين المفضلين لديك" : "Discover independent artists and save the music you love")
    let caption = Array(repeating: phrase, count: repetitions).joined(separator: "\n")
    let segments = caption.split(whereSeparator: { $0.isWhitespace }).enumerated().map {
        Segment(text: String($0.element), start: Double($0.offset), end: Double($0.offset + 1))
    }
    let view = NexaTextRevealView<Segment>()
    func update(_ position: Double, color: UIColor = UIColor.blue.withAlphaComponent(alpha),
                opacity: CGFloat = 0.38, playing: Bool = false) {
        view.update(text: caption, segments: segments, textOf: { $0.text }, startTimeOf: { $0.start }, endTimeOf: { $0.end }, hasTrailingSpaceOf: { _ in true }, position: position,
            isActive: true, isPlaying: playing, revealEnabled: true, seekRevision: 0,
            dimOpacity: opacity, leadTime: 0.3, targetOffset: 0.56, smoothingDuration: 0.25,
            fontName: nil, fontSize: 17, fontWeight: .regular, fontDesign: "Default", color: color,
            alignment: alignment, lineHeight: lineHeight, letterSpacing: 0, maxLines: 0,
            italic: italic, underline: decorations, strikethrough: decorations)
    }
    view.frame = CGRect(x: 0, y: 0, width: width, height: 10000)
    update(10000)
    let measured = view.measuredSize(width: CGFloat(width))
    view.frame.size = measured
    view.layoutIfNeeded()
    let pixelWidth = width * scale
    let pixelHeight = max(Int(ceil(measured.height * CGFloat(scale))), 1)
    let count = pixelWidth * pixelHeight * 4
    let memory = UnsafeMutableRawPointer.allocate(byteCount: count, alignment: 16)
    defer { memory.deallocate() }
    memory.initializeMemory(as: UInt8.self, repeating: 0, count: count)
    guard let context = CGContext(data: memory, width: pixelWidth, height: pixelHeight, bitsPerComponent: 8,
        bytesPerRow: pixelWidth * 4, space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { fatalError("bitmap context") }
    context.translateBy(x: 0, y: CGFloat(pixelHeight))
    context.scaleBy(x: CGFloat(scale), y: -CGFloat(scale))
    UIGraphicsPushContext(context)
    defer { UIGraphicsPopContext() }
    func draw(_ reference: Bool) {
        context.clear(view.bounds)
        if reference { view.drawReference() } else { view.draw(view.bounds) }
    }
    func digest() -> SHA256.Digest {
        SHA256.hash(data: Data(bytesNoCopy: memory, count: count, deallocator: .none))
    }
    var checked = 0
    for position in [0, 1.25, Double(segments.count) / 2 + 0.25, Double(segments.count) - 0.5, 10000] {
        update(position)
        draw(true)
        let reference = digest()
        draw(false)
        guard digest() == reference else {
            FileHandle.standardError.write(Data("Reveal pixels changed at position=\(position), lines=\(view.testLineCount)\n".utf8))
            exit(1)
        }
        checked += 1
    }
    let timings = view.testTimingStorage
    update(10000, color: .red, opacity: 0.7)
    precondition(view.testBaseColor?.isEqual(UIColor.red.withAlphaComponent(0.7)) == true, "Color-only updates must refresh paint")
    precondition(view.testTimingStorage == timings, "Paint updates must retain timing geometry")
    // The optimized renderer must paint correctly before a reference draw has
    // a chance to generate any invalidated highlight glyphs.
    draw(false)
    let recolored = digest()
    draw(true)
    precondition(digest() == recolored, "First draw after a paint update must include the highlight")
    let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 440, height: 956))
    window.addSubview(view)
    update(0.2, playing: true)
    precondition(view.testHasDisplayLink, "Native clock should start")
    view.dispose()
    precondition(!view.testHasDisplayLink, "Disposal must invalidate the native clock")
    view.layoutSubviews()
    precondition(!view.testHasDisplayLink, "Layout after disposal must not restart the clock")
    view.removeFromSuperview()
    var referenceSamples: [Double] = []
    var optimizedSamples: [Double] = []
    if benchmark {
        update(10000)
        for _ in 0..<5 { draw(true); draw(false) }
        for _ in 0..<7 {
            var start = CACurrentMediaTime()
            for _ in 0..<20 { draw(true) }
            referenceSamples.append((CACurrentMediaTime() - start) * 1000)
            start = CACurrentMediaTime()
            for _ in 0..<20 { draw(false) }
            optimizedSamples.append((CACurrentMediaTime() - start) * 1000)
        }
    }
    return ["lines": view.testLineCount, "pixelCases": checked, "displayScale": scale,
            "referenceMs": referenceSamples, "optimizedMs": optimizedSamples, "drawsPerSample": 20]
}

@main struct Check {
    @MainActor static func main() {
        var results: [[String: Any]] = []
        for count in [1, 3, 30] {
            results.append(checkCaption(repetitions: count, width: 320, rtl: false, lineHeight: 22,
                alignment: "Leading", italic: false, decorations: false, alpha: 1, benchmark: true))
        }
        for rtl in [false, true] {
            for height: CGFloat in [0, 9, 22, 23.5] {
                for alignment in ["Leading", "Center", "Trailing"] {
                    results.append(checkCaption(repetitions: 3, width: 240, rtl: rtl, lineHeight: height,
                        alignment: alignment, italic: true, decorations: true, alpha: 0.6, benchmark: false))
                }
            }
        }
        for scale in [2, 3] {
            for rtl in [false, true] {
                results.append(checkCaption(repetitions: 3, width: 240, rtl: rtl, lineHeight: 23.5,
                    alignment: "Leading", italic: true, decorations: true, alpha: 0.6, benchmark: false, scale: scale))
            }
        }
        for phrase in ["新しい音楽とお気に入りのアーティストを見つけよう 🎵", "Find your next artist 👩🏽‍🎤 office affinity", "Discover מוזיקה חדשה tonight"] {
            results.append(checkCaption(repetitions: 3, width: 240, rtl: false, lineHeight: 0,
                alignment: "Leading", italic: false, decorations: false, alpha: 0.6,
                benchmark: false, scale: 3, phraseOverride: phrase))
        }
        let json = try! JSONSerialization.data(withJSONObject: ["cases": results], options: [.sortedKeys])
        print(String(data: json, encoding: .utf8)!)
    }
}
