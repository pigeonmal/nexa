// nexa-codegen: NexaTextRevealPrimitive
struct NexaTextRevealPrimitive<Segment>: View {
    let text: String
    let segments: [Segment]
    let textOf: (Segment) -> String
    let startTimeOf: (Segment) -> Double
    let endTimeOf: (Segment) -> Double
    let hasTrailingSpaceOf: (Segment) -> Bool
    let position: Double
    let active: Bool
    let playing: Bool
    let enabled: Bool
    let seekRevision: Int64
    let dimOpacity: CGFloat
    let leadTime: Double
    let targetOffset: Double
    let smoothingDuration: Double
    let fontName: String?
    let fontSize: CGFloat
    let fontWeight: UIFont.Weight
    let fontDesign: String
    let color: UIColor
    let alignment: String
    let lineHeight: CGFloat
    let letterSpacing: CGFloat
    let maxLines: Int
    let italic: Bool
    let underline: Bool
    let strikethrough: Bool

    var body: some View {
        NexaTextRevealRepresentable(
            text: text,
            segments: segments,
            textOf: textOf,
            startTimeOf: startTimeOf,
            endTimeOf: endTimeOf,
            hasTrailingSpaceOf: hasTrailingSpaceOf,
            position: position,
            active: active,
            playing: playing,
            enabled: enabled,
            seekRevision: seekRevision,
            dimOpacity: dimOpacity,
            leadTime: leadTime,
            targetOffset: targetOffset,
            smoothingDuration: smoothingDuration,
            fontName: fontName,
            fontSize: fontSize,
            fontWeight: fontWeight,
            fontDesign: fontDesign,
            color: color,
            alignment: alignment,
            lineHeight: lineHeight,
            letterSpacing: letterSpacing,
            maxLines: maxLines,
            italic: italic,
            underline: underline,
            strikethrough: strikethrough
        )
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityLabel(text)
    }
}

private struct NexaTextRevealRepresentable<Segment>: UIViewRepresentable {
    let text: String
    let segments: [Segment]
    let textOf: (Segment) -> String
    let startTimeOf: (Segment) -> Double
    let endTimeOf: (Segment) -> Double
    let hasTrailingSpaceOf: (Segment) -> Bool
    let position: Double
    let active: Bool
    let playing: Bool
    let enabled: Bool
    let seekRevision: Int64
    let dimOpacity: CGFloat
    let leadTime: Double
    let targetOffset: Double
    let smoothingDuration: Double
    let fontName: String?
    let fontSize: CGFloat
    let fontWeight: UIFont.Weight
    let fontDesign: String
    let color: UIColor
    let alignment: String
    let lineHeight: CGFloat
    let letterSpacing: CGFloat
    let maxLines: Int
    let italic: Bool
    let underline: Bool
    let strikethrough: Bool

    func makeUIView(context: Context) -> NexaTextRevealView<Segment> {
        NexaTextRevealView()
    }

    func updateUIView(_ view: NexaTextRevealView<Segment>, context: Context) {
        view.update(
            text: text,
            segments: segments,
            textOf: textOf,
            startTimeOf: startTimeOf,
            endTimeOf: endTimeOf,
            hasTrailingSpaceOf: hasTrailingSpaceOf,
            position: position,
            active: active,
            playing: playing,
            enabled: enabled,
            seekRevision: seekRevision,
            dimOpacity: dimOpacity,
            leadTime: leadTime,
            targetOffset: targetOffset,
            smoothingDuration: smoothingDuration,
            fontName: fontName,
            fontSize: fontSize,
            fontWeight: fontWeight,
            fontDesign: fontDesign,
            color: color,
            alignment: alignment,
            lineHeight: lineHeight,
            letterSpacing: letterSpacing,
            maxLines: maxLines,
            italic: italic,
            underline: underline,
            strikethrough: strikethrough
        )
    }

    @available(iOS 16.0, *)
    func sizeThatFits(_ proposal: ProposedViewSize, uiView: NexaTextRevealView<Segment>, context: Context) -> CGSize {
        uiView.measuredSize(width: proposal.width ?? 320)
    }
}

private struct NexaTextRevealWordTiming {
    let startTime: Double
    let endTime: Double
    let startProgress: CGFloat
    let endProgress: CGFloat
}

private struct NexaTextRevealLineTiming {
    let rect: CGRect
    let isRightToLeft: Bool
    let startTime: Double
    let endTime: Double
    let words: [NexaTextRevealWordTiming]
}

private final class NexaTextRevealView<Segment>: UIView {
    private let textStorage = NSTextStorage()
    private let textContainer = NSTextContainer(size: .zero)
    private let layoutManager = NSLayoutManager()
    private let highlightStorage = NSTextStorage()
    private let highlightContainer = NSTextContainer(size: .zero)
    private let highlightLayoutManager = NSLayoutManager()

    private var text = ""
    private var segments: [Segment] = []
    private var textOf: ((Segment) -> String)?
    private var startTimeOf: ((Segment) -> Double)?
    private var endTimeOf: ((Segment) -> Double)?
    private var hasTrailingSpaceOf: ((Segment) -> Bool)?
    private var lineTimings: [NexaTextRevealLineTiming] = []
    private var displayLink: CADisplayLink?
    private var animationStartTime: CFTimeInterval = 0
    private var animationStartPosition = 0.0
    private var animationTargetPosition = 0.0
    private var animationDuration = 0.0
    private var currentPosition = 0.0
    private var sampledPosition = 0.0
    private var lastSeekRevision: Int64?
    private var currentFontSize: CGFloat = 17
    private var currentLineHeight: CGFloat = 0
    private var currentLetterSpacing: CGFloat = 0
    private var currentMaxLines = 0
    private var currentDimOpacity: CGFloat = 0.38
    private var currentColor = UIColor.white
    private var currentActive = false
    private var currentPlaying = false
    private var currentEnabled = false
    private var currentLeadTime = 0.3
    private var currentTargetOffset = 0.56
    private var currentSmoothingDuration = 0.25
    private var currentFontName: String?
    private var currentFontWeight = UIFont.Weight.regular
    private var currentFontDesign = "Default"
    private var currentAlignment = "Natural"
    private var currentItalic = false
    private var currentUnderline = false
    private var currentStrikethrough = false
    private var needsTextLayout = true

    override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        isAccessibilityElement = true
        textContainer.lineFragmentPadding = 0
        highlightContainer.lineFragmentPadding = 0
        textStorage.addLayoutManager(layoutManager)
        layoutManager.addTextContainer(textContainer)
        highlightStorage.addLayoutManager(highlightLayoutManager)
        highlightLayoutManager.addTextContainer(highlightContainer)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override var intrinsicContentSize: CGSize {
        guard bounds.width > 0 else {
            return CGSize(width: UIView.noIntrinsicMetric, height: UIFont.systemFont(ofSize: currentFontSize).lineHeight)
        }
        return measuredSize(width: bounds.width)
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil { stopDisplayLink() }
        else { resumeAnimationIfNeeded() }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        if textContainer.size.width != bounds.width {
            textContainer.size = CGSize(width: bounds.width, height: .greatestFiniteMagnitude)
            highlightContainer.size = textContainer.size
            needsTextLayout = true
            invalidateIntrinsicContentSize()
        }
        rebuildLineTimingsIfNeeded()
        setNeedsDisplay()
    }

    func measuredSize(width: CGFloat) -> CGSize {
        textContainer.size = CGSize(width: max(width, 1), height: .greatestFiniteMagnitude)
        highlightContainer.size = textContainer.size
        rebuildTextIfNeeded()
        layoutManager.ensureLayout(for: textContainer)
        let used = layoutManager.usedRect(for: textContainer)
        let height = ceil(max(used.height, UIFont.systemFont(ofSize: currentFontSize).lineHeight))
        return CGSize(width: width, height: height)
    }

    func update(
        text: String,
        segments: [Segment],
        textOf: @escaping (Segment) -> String,
        startTimeOf: @escaping (Segment) -> Double,
        endTimeOf: @escaping (Segment) -> Double,
        hasTrailingSpaceOf: @escaping (Segment) -> Bool,
        position: Double,
        active: Bool,
        playing: Bool,
        enabled: Bool,
        seekRevision: Int64,
        dimOpacity: CGFloat,
        leadTime: Double,
        targetOffset: Double,
        smoothingDuration: Double,
        fontName: String?,
        fontSize: CGFloat,
        fontWeight: UIFont.Weight,
        fontDesign: String,
        color: UIColor,
        alignment: String,
        lineHeight: CGFloat,
        letterSpacing: CGFloat,
        maxLines: Int,
        italic: Bool,
        underline: Bool,
        strikethrough: Bool
    ) {
        let textChanged = self.text != text
            || currentFontSize != fontSize
            || currentLineHeight != lineHeight
            || currentLetterSpacing != letterSpacing
            || currentMaxLines != maxLines
            || currentFontName != fontName
            || currentFontWeight != fontWeight
            || currentFontDesign != fontDesign
            || currentAlignment != alignment
            || currentItalic != italic
            || currentUnderline != underline
            || currentStrikethrough != strikethrough
        self.text = text
        self.segments = segments
        self.textOf = textOf
        self.startTimeOf = startTimeOf
        self.endTimeOf = endTimeOf
        self.hasTrailingSpaceOf = hasTrailingSpaceOf
        self.currentDimOpacity = dimOpacity
        self.currentLeadTime = leadTime
        self.currentTargetOffset = targetOffset
        self.currentSmoothingDuration = smoothingDuration
        self.currentFontName = fontName
        self.currentFontSize = fontSize
        self.currentFontWeight = fontWeight
        self.currentFontDesign = fontDesign
        self.currentColor = color
        self.currentAlignment = alignment
        self.currentLineHeight = lineHeight
        self.currentLetterSpacing = letterSpacing
        self.currentMaxLines = maxLines
        self.currentItalic = italic
        self.currentUnderline = underline
        self.currentStrikethrough = strikethrough
        accessibilityLabel = text
        textContainer.maximumNumberOfLines = maxLines
        highlightContainer.maximumNumberOfLines = maxLines
        if textChanged { needsTextLayout = true }
        rebuildTextIfNeeded()
        rebuildLineTimingsIfNeeded()

        let seekChanged = lastSeekRevision.map { $0 != seekRevision } ?? false
        let becameActive = active && (!currentActive || (enabled && !currentEnabled))
        let playbackStateChanged = playing != currentPlaying || enabled != currentEnabled
        currentActive = active
        currentPlaying = playing
        currentEnabled = enabled

        if seekChanged {
            stopDisplayLink()
            let firstStart = segments.first.flatMap { startTimeOf($0) } ?? position
            currentPosition = min(position, firstStart)
            sampledPosition = currentPosition
            lastSeekRevision = seekRevision
        } else if !active || !playing || !enabled {
            stopDisplayLink()
            currentPosition = position
            sampledPosition = position
            lastSeekRevision = seekRevision
        } else if becameActive || playbackStateChanged || sampledPosition != position {
            let from = becameActive ? position + leadTime : currentPosition
            animatePosition(from: from, to: position + targetOffset, duration: smoothingDuration)
            sampledPosition = position
            lastSeekRevision = seekRevision
        }
        if textChanged { invalidateIntrinsicContentSize() }
        setNeedsDisplay()
    }

    override func draw(_ rect: CGRect) {
        super.draw(rect)
        rebuildTextIfNeeded()
        rebuildLineTimingsIfNeeded()
        let allGlyphs = NSRange(location: 0, length: layoutManager.numberOfGlyphs)
        guard allGlyphs.length > 0 else { return }
        let context = UIGraphicsGetCurrentContext()
        layoutManager.drawBackground(forGlyphRange: allGlyphs, at: .zero)
        layoutManager.drawGlyphs(forGlyphRange: allGlyphs, at: .zero)

        guard currentActive else { return }
        let fullReveal = !currentEnabled || segments.isEmpty || lineTimings.isEmpty
        for timing in lineTimings {
            let progress = fullReveal ? CGFloat(1) : nexaTextRevealProgress(timing, position: currentPosition)
            guard progress > 0 else { continue }
            let width = timing.rect.width * progress
            let x = timing.isRightToLeft ? timing.rect.maxX - width : timing.rect.minX
            let clip = CGRect(x: x, y: timing.rect.minY, width: width, height: timing.rect.height)
            context?.saveGState()
            context?.clip(to: clip)
            highlightLayoutManager.drawGlyphs(
                forGlyphRange: NSRange(location: 0, length: highlightLayoutManager.numberOfGlyphs),
                at: .zero
            )
            context?.restoreGState()
        }
    }

    private func animatePosition(from: Double, to: Double, duration: Double) {
        currentPosition = from
        animationStartPosition = from
        animationTargetPosition = to
        animationStartTime = CACurrentMediaTime()
        animationDuration = max(duration, 0)
        if animationDuration == 0 {
            currentPosition = to
            stopDisplayLink()
            setNeedsDisplay()
            return
        }
        resumeAnimationIfNeeded()
    }

    private func resumeAnimationIfNeeded() {
        guard window != nil, currentActive, currentPlaying, currentEnabled,
              animationDuration > 0, currentPosition != animationTargetPosition else { return }
        guard displayLink == nil else { return }
        let link = CADisplayLink(target: self, selector: #selector(updateTimeline(_:)))
        link.add(to: .main, forMode: .common)
        displayLink = link
    }

    @objc private func updateTimeline(_ link: CADisplayLink) {
        guard currentActive, currentPlaying, currentEnabled else {
            stopDisplayLink()
            return
        }
        let fraction = min(max((link.timestamp - animationStartTime) / animationDuration, 0), 1)
        currentPosition = animationStartPosition + (animationTargetPosition - animationStartPosition) * fraction
        setNeedsDisplay()
        if fraction >= 1 { stopDisplayLink() }
    }

    private func stopDisplayLink() {
        displayLink?.invalidate()
        displayLink = nil
    }

    private func rebuildTextIfNeeded() {
        guard needsTextLayout else { return }
        let baseColor = currentColor.withAlphaComponent(currentColor.cgColor.alpha * currentDimOpacity)
        let font = resolvedFont()
        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = resolvedAlignment()
        paragraph.lineBreakMode = .byWordWrapping
        if currentLineHeight > 0 {
            paragraph.minimumLineHeight = currentLineHeight
            paragraph.maximumLineHeight = currentLineHeight
        }
        var attributes: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: baseColor,
            .paragraphStyle: paragraph,
            .kern: currentLetterSpacing,
        ]
        if currentUnderline { attributes[.underlineStyle] = NSUnderlineStyle.single.rawValue }
        if currentStrikethrough { attributes[.strikethroughStyle] = NSUnderlineStyle.single.rawValue }
        if currentItalic, currentFontName == nil {
            attributes[.obliqueness] = 0.2
        }
        textStorage.setAttributedString(NSAttributedString(string: text, attributes: attributes))
        var highlightAttributes = attributes
        highlightAttributes[.foregroundColor] = currentColor
        highlightStorage.setAttributedString(NSAttributedString(string: text, attributes: highlightAttributes))
        layoutManager.ensureLayout(for: textContainer)
        highlightLayoutManager.ensureLayout(for: highlightContainer)
        needsTextLayout = false
        lineTimings = []
        rebuildLineTimingsIfNeeded()
    }

    private func rebuildLineTimingsIfNeeded() {
        guard !needsTextLayout,
              let textOf,
              let startTimeOf,
              let endTimeOf,
              let hasTrailingSpaceOf else { return }
        layoutManager.ensureLayout(for: textContainer)
        var fragments: [(rect: CGRect, characterRange: NSRange)] = []
        let glyphCount = layoutManager.numberOfGlyphs
        var glyphIndex = 0
        while glyphIndex < glyphCount {
            var glyphRange = NSRange(location: 0, length: 0)
            let usedRect = layoutManager.lineFragmentUsedRect(forGlyphAt: glyphIndex, effectiveRange: &glyphRange)
            let chars = layoutManager.characterRange(forGlyphRange: glyphRange, actualGlyphRange: nil)
            fragments.append((usedRect, chars))
            glyphIndex = NSMaxRange(glyphRange)
        }
        let widths = fragments.map { Double($0.rect.width) }
        let totalWidth = widths.reduce(0, +)
        let wordUnits = segments.map { max(textOf($0).utf16.count, 1) + (hasTrailingSpaceOf($0) ? 1 : 0) }
        var cumulative = [Int](repeating: 0, count: segments.count + 1)
        for index in segments.indices { cumulative[index + 1] = cumulative[index] + wordUnits[index] }
        let totalUnits = max(cumulative.last ?? 0, 1)
        var startWord = 0
        var consumedWidth = 0.0
        lineTimings = fragments.enumerated().map { lineIndex, fragment in
            let lineString = (text as NSString).substring(with: fragment.characterRange)
            let rtl = nexaTextRevealIsRTL(lineString)
            guard startWord < segments.count else {
                let previousEnd = startWord > 0 ? endTimeOf(segments[startWord - 1]) : segments.first.map(startTimeOf) ?? 0
                consumedWidth += widths[lineIndex]
                return NexaTextRevealLineTiming(rect: fragment.rect, isRightToLeft: rtl, startTime: previousEnd, endTime: previousEnd + 0.01, words: [])
            }
            let remainingLines = fragments.count - lineIndex - 1
            let maxEndWord = max(startWord + 1, segments.count - remainingLines)
            let targetUnits: Double = totalWidth > 0
                ? Double(totalUnits) * (consumedWidth + widths[lineIndex]) / totalWidth
                : Double(totalUnits) * Double(lineIndex + 1) / Double(fragments.count)
            var endWord = min(startWord + 1, maxEndWord)
            while endWord < maxEndWord && Double(cumulative[endWord]) < targetUnits { endWord += 1 }
            let startUnits = cumulative[startWord]
            let endUnits = max(cumulative[endWord], startUnits + 1)
            let unitSpan = CGFloat(endUnits - startUnits)
            let words = (startWord..<endWord).map { index in
                NexaTextRevealWordTiming(
                    startTime: startTimeOf(segments[index]),
                    endTime: endTimeOf(segments[index]),
                    startProgress: CGFloat(cumulative[index] - startUnits) / unitSpan,
                    endProgress: CGFloat(cumulative[index + 1] - startUnits) / unitSpan
                )
            }
            let startTime = words.first?.startTime ?? (startWord > 0 ? endTimeOf(segments[startWord - 1]) : 0)
            let endTime = max(words.last?.endTime ?? (startTime + 0.01), startTime + 0.01)
            startWord = endWord
            consumedWidth += widths[lineIndex]
            return NexaTextRevealLineTiming(rect: fragment.rect, isRightToLeft: rtl, startTime: startTime, endTime: endTime, words: words)
        }
    }

    private func resolvedFont() -> UIFont {
        if let currentFontName, let font = UIFont(name: currentFontName, size: currentFontSize) { return font }
        let base = UIFont.systemFont(ofSize: currentFontSize, weight: currentFontWeight)
        let design: UIFontDescriptor.SystemDesign? = switch currentFontDesign {
        case "Serif": .serif
        case "Rounded": .rounded
        case "Monospaced": .monospaced
        default: nil
        }
        if let design, let descriptor = base.fontDescriptor.withDesign(design) {
            return UIFont(descriptor: descriptor, size: currentFontSize)
        }
        return base
    }

    private func resolvedAlignment() -> NSTextAlignment {
        let rtl = nexaTextRevealIsRTL(text)
        return switch currentAlignment {
        case "Center": NSTextAlignment.center
        case "Leading": rtl ? NSTextAlignment.right : NSTextAlignment.left
        case "Trailing": rtl ? NSTextAlignment.left : NSTextAlignment.right
        default: NSTextAlignment.natural
        }
    }
}

private func nexaTextRevealIsRTL(_ text: String) -> Bool {
    text.unicodeScalars.contains { scalar in
        let value = scalar.value
        return (0x0591...0x08ff).contains(value)
            || (0xfb1d...0xfdff).contains(value)
            || (0xfe70...0xfefc).contains(value)
    }
}

private func nexaTextRevealProgress(_ timing: NexaTextRevealLineTiming, position: Double) -> CGFloat {
    guard !timing.words.isEmpty else {
        return CGFloat(min(max((position - timing.startTime) / max(timing.endTime - timing.startTime, 0.01), 0), 1))
    }
    if position < timing.words[0].startTime { return 0 }
    if position >= timing.words[timing.words.count - 1].endTime { return 1 }
    var low = 0
    var high = timing.words.count - 1
    var activeWord = -1
    while low <= high {
        let middle = (low + high) >> 1
        if timing.words[middle].startTime <= position {
            activeWord = middle
            low = middle + 1
        } else {
            high = middle - 1
        }
    }
    guard activeWord >= 0 else { return 0 }
    let word = timing.words[activeWord]
    if position >= word.endTime { return min(max(word.endProgress, 0), 1) }
    let duration = max(word.endTime - word.startTime, 0.01)
    let wordProgress = min(max((position - word.startTime) / duration, 0), 1)
    return min(max(word.startProgress + (word.endProgress - word.startProgress) * CGFloat(wordProgress), 0), 1)
}
