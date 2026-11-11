// nexa-codegen: NexaNumberFlowPrimitive
private struct NexaNumberFlowToken: Identifiable {
    let id: String
    let character: Character
    let digitPosition: Int?
}

// Non-observable memoization. One instance per mounted component; cache writes
// never invalidate the SwiftUI tree or schedule another animation transaction.
@MainActor
private final class NexaNumberFlowCache: ObservableObject {
    struct Key: Equatable {
        let value: Double
        let format: String
        let currencyCode: String?
        let minimumFractionDigits: Int?
        let maximumFractionDigits: Int?
        let locale: Locale
    }

    struct Presentation {
        let formatted: String
        let tokens: [NexaNumberFlowToken]
    }

    private var key: Key?
    private var cachedPresentation: Presentation?
    private var glyphLocale: Locale?
    private var cachedDigitGlyphs: [String] = []
    private var previousValue: Double?
    private var increasing = true

    func presentation(for key: Key) -> Presentation {
        if self.key == key, let cachedPresentation { return cachedPresentation }
        let formatted = formattedValue(key)
        if let cachedPresentation, cachedPresentation.formatted == formatted {
            self.key = key
            return cachedPresentation
        }
        let presentation = Presentation(formatted: formatted, tokens: tokens(for: formatted))
        self.key = key
        cachedPresentation = presentation
        return presentation
    }

    func digitGlyphs(locale: Locale) -> [String] {
        if glyphLocale != locale {
            let formatter = NumberFormatter()
            formatter.locale = locale
            formatter.numberStyle = .decimal
            formatter.usesGroupingSeparator = false
            formatter.maximumFractionDigits = 0
            // Every wheel must have ten entries, even if Foundation fails to format.
            cachedDigitGlyphs = (0...9).map { formatter.string(from: NSNumber(value: $0)) ?? String($0) }
            glyphLocale = locale
        }
        return cachedDigitGlyphs
    }

    func direction(for value: Double) -> Bool {
        guard let previousValue else { return true }
        return value == previousValue ? increasing : value > previousValue
    }

    func commit(_ value: Double) {
        increasing = direction(for: value)
        previousValue = value
    }

    private func precisionRange(_ key: Key) -> ClosedRange<Int>? {
        guard key.minimumFractionDigits != nil || key.maximumFractionDigits != nil else { return nil }
        return (key.minimumFractionDigits ?? 0)...(key.maximumFractionDigits ?? 20)
    }

    private func formattedValue(_ key: Key) -> String {
        let value = key.value
        let locale = key.locale
        let precisionRange = precisionRange(key)
        switch key.format {
        case "Currency":
            let style = FloatingPointFormatStyle<Double>.Currency(code: key.currencyCode ?? "USD").locale(locale)
            if let precisionRange { return value.formatted(style.precision(.fractionLength(precisionRange))) }
            return value.formatted(style)
        case "Percent":
            let style = FloatingPointFormatStyle<Double>.Percent().locale(locale)
            if let precisionRange { return value.formatted(style.precision(.fractionLength(precisionRange))) }
            return value.formatted(style)
        case "Compact":
            let style = FloatingPointFormatStyle<Double>.number.notation(.compactName).locale(locale)
            if let precisionRange { return value.formatted(style.precision(.fractionLength(precisionRange))) }
            return value.formatted(style)
        default:
            let style = FloatingPointFormatStyle<Double>.number.locale(locale)
            if let precisionRange { return value.formatted(style.precision(.fractionLength(precisionRange))) }
            return value.formatted(style)
        }
    }

    private func tokens(for formatted: String) -> [NexaNumberFlowToken] {
        let characters = Array(formatted)
        var result: [NexaNumberFlowToken] = []
        result.reserveCapacity(characters.count)
        var position = 0
        for (index, character) in characters.enumerated().reversed() {
            if character.wholeNumberValue != nil {
                result.append(NexaNumberFlowToken(id: "digit:\(position)", character: character, digitPosition: position))
                position += 1
            } else {
                result.append(NexaNumberFlowToken(id: "literal:\(index):\(character)", character: character, digitPosition: nil))
            }
        }
        result.reverse()
        return result
    }
}

struct NexaNumberFlowPrimitive: View {
    let value: Double
    let format: String
    let currencyCode: String?
    let minimumFractionDigits: Int?
    let maximumFractionDigits: Int?
    let animation: Animation
    let fontSize: CGFloat
    let style: NexaTextStyle
    @StateObject private var cache = NexaNumberFlowCache()
    @Environment(\.locale) private var locale
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let presentation = cache.presentation(for: .init(
            value: value, format: format, currencyCode: currencyCode,
            minimumFractionDigits: minimumFractionDigits,
            maximumFractionDigits: maximumFractionDigits, locale: locale
        ))
        let formatted = presentation.formatted
        let glyphs = cache.digitGlyphs(locale: locale)
        let tokens = presentation.tokens
        let isIncreasing = cache.direction(for: value)
        HStack(spacing: 0) {
            ForEach(tokens) { token in
                if let digit = token.character.wholeNumberValue, let digitPosition = token.digitPosition {
                    NexaNumberFlowDigit(
                        digit: digit,
                        id: digitPosition,
                        glyphs: glyphs,
                        fontSize: fontSize,
                        animation: animation,
                        reduceMotion: reduceMotion,
                        isIncreasing: isIncreasing,
                        style: style
                    )
                    .transition(.asymmetric(insertion: .move(edge: .top).combined(with: .opacity), removal: .move(edge: .bottom).combined(with: .opacity)))
                } else {
                    NexaTextPrimitive(text: Text(String(token.character)), style: style)
                        .accessibilityHidden(true)
                        .transition(.opacity)
                }
            }
        }
        .animation(reduceMotion ? nil : animation, value: formatted)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(formatted)
        .onAppear { cache.commit(value) }
        .onChange(of: value) { newValue in cache.commit(newValue) }
    }
}

private struct NexaNumberFlowDigit: View {
    let digit: Int
    let id: Int
    let glyphs: [String]
    let fontSize: CGFloat
    let animation: Animation
    let reduceMotion: Bool
    let isIncreasing: Bool
    let style: NexaTextStyle


    private var lineHeight: CGFloat {
        max(style.lineHeightFactor.map { style.lineHeightFontSize * $0 } ?? fontSize * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__, 1)
    }

    var body: some View {
        NexaTextPrimitive(text: Text(glyphs[digit]), style: style)
            .id(digit)
        .frame(height: lineHeight)
        .clipped()
        .transition(.asymmetric(
            insertion: .move(edge: isIncreasing ? .bottom : .top).combined(with: .opacity),
            removal: .move(edge: isIncreasing ? .top : .bottom).combined(with: .opacity)
        ))
        .animation(reduceMotion ? nil : animation, value: digit)
        .accessibilityHidden(true)
        .id(id)
    }
}
