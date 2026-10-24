use nexa_codegen::{GeneratedSources, SourceUnit, SourceUnits};
use nexa_ir::{LayoutKind, Module, Node, State, ViewStyle};

mod api;
mod components;
mod engine;
mod widget;

// Re-exported through `generator` because `engine` is private and the crate
// root cannot name a path through it.
pub use engine::swift_scalar_types;

const DRAG_GESTURE_HELPERS: &str = r#"@MainActor
private final class NexaDragVelocityTracker {
    private var lastTranslation = CGSize.zero
    private var lastTime: Date?

    func update(_ value: DragGesture.Value) -> CGSize {
        defer {
            lastTranslation = value.translation
            lastTime = value.time
        }
        if #available(iOS 18.0, *) {
            return value.velocity
        }
        guard let lastTime else { return .zero }
        let elapsed = value.time.timeIntervalSince(lastTime)
        guard elapsed > 0 else { return .zero }
        return CGSize(
            width: (value.translation.width - lastTranslation.width) / elapsed,
            height: (value.translation.height - lastTranslation.height) / elapsed
        )
    }

    func reset() {
        lastTranslation = .zero
        lastTime = nil
    }
}

@MainActor
private final class NexaMagnificationTracker {
    private var previousMagnification = 1.0

    func consume(_ magnification: Double) -> Double? {
        guard previousMagnification > 0, magnification.isFinite else { return nil }
        let scaleFactor = magnification / previousMagnification
        previousMagnification = magnification
        return scaleFactor.isFinite ? scaleFactor : nil
    }

    func reset() {
        previousMagnification = 1.0
    }
}

@MainActor
private struct NexaDragGestureView<Content: View>: View {
    let enabled: Bool
    let onDrag: ((Double, Double, Double, Double) -> Void)?
    let onPinch: ((Double) -> Void)?
    let content: Content
    @State private var velocityTracker: NexaDragVelocityTracker?
    @State private var magnificationTracker = NexaMagnificationTracker()

    init(
        enabled: Bool,
        trackVelocity: Bool,
        onDrag: ((Double, Double, Double, Double) -> Void)?,
        onPinch: ((Double) -> Void)?,
        @ViewBuilder content: () -> Content
    ) {
        self.enabled = enabled
        self.onDrag = onDrag
        self.onPinch = onPinch
        self.content = content()
        _velocityTracker = State(
            initialValue: trackVelocity ? NexaDragVelocityTracker() : nil
        )
    }

    private var dragGesture: some Gesture {
        DragGesture()
            .onChanged { value in
                guard enabled else { return }
                let velocity = velocityTracker?.update(value) ?? .zero
                onDrag?(
                    Double(value.translation.width),
                    Double(value.translation.height),
                    Double(velocity.width),
                    Double(velocity.height)
                )
            }
            .onEnded { _ in velocityTracker?.reset() }
    }

    private var pinchGesture: some Gesture {
        MagnificationGesture()
            .onChanged { magnification in
                guard enabled else { return }
                guard let scaleFactor = magnificationTracker.consume(Double(magnification)) else {
                    return
                }
                onPinch?(scaleFactor)
            }
            .onEnded { _ in magnificationTracker.reset() }
    }

    @ViewBuilder
    var body: some View {
        if onDrag != nil, onPinch != nil {
            content
                .highPriorityGesture(dragGesture)
                .simultaneousGesture(pinchGesture)
        } else if onDrag != nil {
            content.highPriorityGesture(dragGesture)
        } else if onPinch != nil {
            content.simultaneousGesture(pinchGesture)
        } else {
            content
        }
    }
}

"#;

const NEXA_DYNAMIC_COLOR_HELPER: &str = r##"private func nexaColor(hex: String) -> Color {
    let digits = hex.hasPrefix("#") ? String(hex.dropFirst()) : hex
    guard let value = UInt64(digits, radix: 16) else { return __NEXA_SWIFT_DEFAULT_ACCENT_COLOR__ }
    let red: UInt64
    let green: UInt64
    let blue: UInt64
    let alpha: UInt64
    switch digits.count {
    case 6:
        red = (value >> 16) & 0xFF
        green = (value >> 8) & 0xFF
        blue = value & 0xFF
        alpha = 0xFF
    case 8:
        red = (value >> 24) & 0xFF
        green = (value >> 16) & 0xFF
        blue = (value >> 8) & 0xFF
        alpha = value & 0xFF
    default:
        return __NEXA_SWIFT_DEFAULT_ACCENT_COLOR__
    }
    return Color(
        .sRGB,
        red: Double(red) / 255,
        green: Double(green) / 255,
        blue: Double(blue) / 255,
        opacity: Double(alpha) / 255
    )
}
"##;

const NEXA_GLASS_HELPER: &str = r#"fileprivate enum NexaGlassShape {
    case circle
    case capsule
    case rounded(Double)
}

fileprivate extension View {
    @ViewBuilder
    func nexaGlass(tint: Color? = nil, shape: NexaGlassShape = .circle) -> some View {
        if #available(iOS 26.0, *) {
            switch shape {
            case .circle:
                if let tint {
                    self.glassEffect(.clear.tint(tint).interactive(), in: .circle)
                } else {
                    self.glassEffect(.clear.interactive(), in: .circle)
                }
            case .capsule:
                if let tint {
                    self.glassEffect(.clear.tint(tint).interactive(), in: .capsule)
                } else {
                    self.glassEffect(.clear.interactive(), in: .capsule)
                }
            case .rounded(let r):
                if let tint {
                    self.glassEffect(.clear.tint(tint).interactive(), in: .rect(cornerRadius: CGFloat(r)))
                } else {
                    self.glassEffect(.clear.interactive(), in: .rect(cornerRadius: CGFloat(r)))
                }
            }
        } else {
            switch shape {
            case .circle:
                self.background(.ultraThinMaterial, in: Circle())
            case .capsule:
                self.background(.ultraThinMaterial, in: Capsule())
            case .rounded(let r):
                self.background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: CGFloat(r)))
            }
        }
    }

}
"#;

const NEXA_LOCALE_HELPER: &str = r#"private func nexaCurrentLanguageCode() -> String {
    guard let language = Locale.current.identifier
        .split(whereSeparator: { $0 == "-" || $0 == "_" })
        .first,
        language.count > 1
    else {
        return "und"
    }
    return language.lowercased()
}

"#;

const NEXA_COLLECTION_HELPERS: &str = r#"private func nexaGroupByStable<Element, Key: Hashable>(
    _ elements: [Element],
    by keyForElement: (Element) -> Key
) -> [[Element]] {
    var sectionByKey: [Key: Int] = [:]
    var sections: [[Element]] = []
    for element in elements {
        let key = keyForElement(element)
        if let section = sectionByKey[key] {
            sections[section].append(element)
        } else {
            sectionByKey[key] = sections.count
            sections.append([element])
        }
    }
    return sections
}

"#;

const NEXA_SHARED_COMPONENT_PRIMITIVES: &str = concat!(
    r#"// nexa-codegen: NexaTextPrimitive
__NEXA_REDUCE_MOTION_ANIMATION_MODIFIER__

struct NexaTextStyle {
    var alignment: TextAlignment?
    var color: Color?
    var font: Font?
    var weight: Font.Weight?
    var lineLimit: Int?
    var lineHeightFactor: CGFloat?
    var lineHeightFontSize: CGFloat
    var tracking: CGFloat?
    var strikethrough: Bool
    var italic: Bool
    var underline: Bool
    var selectable: Bool

    init() {
        self.alignment = nil
        self.color = nil
        self.font = nil
        self.weight = nil
        self.lineLimit = nil
        self.lineHeightFactor = nil
        self.lineHeightFontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__
        self.tracking = nil
        self.strikethrough = false
        self.italic = false
        self.underline = false
        self.selectable = false
    }

    func withAlignment(_ value: TextAlignment) -> Self {
        var style = self
        style.alignment = value
        return style
    }

    func withColor(_ value: Color) -> Self {
        var style = self
        style.color = value
        return style
    }

    func withFont(_ value: Font) -> Self {
        var style = self
        style.font = value
        return style
    }

    func withWeight(_ value: Font.Weight) -> Self {
        var style = self
        style.weight = value
        return style
    }

    func withLineLimit(_ value: Int) -> Self {
        var style = self
        style.lineLimit = value
        return style
    }

    func withLineHeight(factor: CGFloat, fontSize: CGFloat) -> Self {
        var style = self
        style.lineHeightFactor = factor
        style.lineHeightFontSize = fontSize
        return style
    }

    func withTracking(_ value: CGFloat) -> Self {
        var style = self
        style.tracking = value
        return style
    }

    func withStrikethrough() -> Self {
        var style = self
        style.strikethrough = true
        return style
    }

    func withItalic() -> Self {
        var style = self
        style.italic = true
        return style
    }

    func withUnderline() -> Self {
        var style = self
        style.underline = true
        return style
    }

    func withSelectable() -> Self {
        var style = self
        style.selectable = true
        return style
    }
}

struct NexaTextPrimitive: View {
    let text: Text
    let style: NexaTextStyle

    init(text: Text) {
        self.text = text
        self.style = NexaTextStyle()
    }

    init(text: Text, style: NexaTextStyle) {
        self.text = text
        self.style = style
    }

    @ViewBuilder var body: some View {
        let styledText = text
            .font(style.font)
            .fontWeight(style.weight)
            .foregroundStyle(style.color ?? Color.primary)
            .lineLimit(style.lineLimit)
            .tracking(style.tracking ?? 0)
            .strikethrough(style.strikethrough)
            .italic(style.italic)
            .underline(style.underline)
            .__NEXA_TEXT_LINE_HEIGHT_MODIFIER__
        if style.selectable {
            if let alignment = style.alignment {
                styledText.multilineTextAlignment(alignment).textSelection(.enabled)
            } else {
                styledText.textSelection(.enabled)
            }
        } else {
            if let alignment = style.alignment {
                styledText.multilineTextAlignment(alignment)
            } else {
                styledText
            }
        }
    }
}

// nexa-codegen: NexaSwitchPrimitive
struct NexaSwitchPrimitive: View {
    let label: Text
    @Binding var isOn: Bool

    var body: some View {
        HStack(spacing: 12) {
            label
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .onTapGesture { isOn.toggle() }
            Toggle("", isOn: $isOn)
                .labelsHidden()
                .tint(Color(uiColor: .systemBlue))
                .accessibilityLabel(label)
        }
    }
}

// nexa-codegen: NexaTextInputPrimitive
struct NexaTextInputPrimitive: View {
    @Binding var text: String
    let placeholder: Text
    let secure: Bool
    let multiline: Bool
    let searchable: Bool
    let maxLength: Int?
    let keepsFocusOnNext: Bool
    let onSubmitAction: (() -> Void)?
    let onChange: ((String) -> Void)?
    @State private var correctionRevision = 0
    @FocusState private var nextSubmissionFocused: Bool

    private var inputText: Binding<String> {
        Binding(
            get: { text },
            set: { nextValue in
                let limitedValue = maxLength.map { String(nextValue.prefix(max(0, $0))) } ?? nextValue
                let previousValue = text
                text = limitedValue
                if previousValue != limitedValue {
                    onChange?(limitedValue)
                }
                if limitedValue != nextValue {
                    correctionRevision &+= 1
                }
            }
        )
    }

    @ViewBuilder
    private var field: some View {
        if secure {
            SecureField(text: inputText, prompt: placeholder) { placeholder }
        } else if multiline {
            TextField(text: inputText, prompt: placeholder, axis: .vertical) { placeholder }
        } else {
            TextField(text: inputText, prompt: placeholder) { placeholder }
        }
    }

    @ViewBuilder
    private var focusRetainingField: some View {
        if keepsFocusOnNext {
            field
                .focused($nextSubmissionFocused)
                .onSubmit {
                    onSubmitAction?()
                    // SwiftUI dismisses a lone field after its .next submit
                    // action unless focus is explicitly restored. Keep the
                    // native keyboard available when there is no focus target
                    // supplied by the caller.
                    DispatchQueue.main.async {
                        nextSubmissionFocused = true
                    }
                }
        } else {
            field
        }
    }

    @ViewBuilder
    var body: some View {
        // A rejected insertion can leave SwiftUI's native text control holding
        // the overlong editing buffer when the bound value is already at its
        // limit. Reading this revision makes the field reconcile from the
        // bounded binding after each rejected edit without replacing the
        // focused native control.
        let _ = correctionRevision
        if searchable {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                    .accessibilityHidden(true)
                leadingField
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            leadingField
        }
    }

    // SwiftUI TextField otherwise measures to its placeholder's ideal width. Compose fields fill
    // the available parent width, so keep the SwiftUI primitive leading-aligned and equally wide.
    private var leadingField: some View {
        focusRetainingField
            .frame(maxWidth: .infinity, alignment: .leading)
            .multilineTextAlignment(.leading)
    }
}

// nexa-codegen: nexaPickerTint
private extension View {
    @ViewBuilder
    func nexaPickerTint(_ tint: Color?) -> some View {
        if let tint {
            self.tint(tint)
        } else {
            self
        }
    }
}

// nexa-codegen: NexaPickerPrimitive
struct NexaPickerPrimitive<LabelContent: View>: View {
    @Binding var selection: String
    let items: [String]
    let itemIcons: [String]
    let icon: String?
    let tint: Color?
    let hasLabel: Bool
    @ViewBuilder let label: () -> LabelContent

    private var resolvedTint: Color {
        tint ?? Color.accentColor
    }

    @ViewBuilder
    private func pickerOptions(selection: Binding<String>) -> some View {
        Picker("", selection: selection) {
            ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                if itemIcons.indices.contains(index) {
                    Label {
                        Text(item)
                    } icon: {
                        Image(systemName: itemIcons[index])
                            .foregroundStyle(.secondary)
                    }
                        .tag(item)
                } else {
                    Text(item)
                        .tag(item)
                }
            }
        }
        .labelsHidden()
    }

    @ViewBuilder
    var body: some View {
        if hasLabel {
            Menu {
                pickerOptions(selection: $selection)
            } label: {
                HStack(spacing: 8) {
                    if let icon {
                        Image(systemName: icon)
                    }
                    label()
                    Spacer(minLength: 12)
                    HStack(spacing: 4) {
                        Text(selection)
                        Image(systemName: "chevron.up.chevron.down")
                            .font(.caption)
                    }
                }
                .foregroundStyle(resolvedTint)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityElement(children: .combine)
            .accessibilityValue(selection)
        } else if let icon {
            Menu {
                pickerOptions(selection: $selection)
            } label: {
                Image(systemName: icon)
                    .foregroundStyle(tint ?? Color.accentColor)
            }
            .accessibilityLabel("Choose an option")
            .accessibilityValue(selection)
        } else {
            pickerOptions(selection: $selection)
                .pickerStyle(.menu)
                .labelsHidden()
                .nexaPickerTint(tint)
        }
    }
}

"#,
    r#"// nexa-codegen: NexaSharedButtonStyleKind
enum NexaSharedButtonStyleKind {
    case borderedProminent
    case bordered
    case borderless
    case plain
}

// nexa-codegen: NexaSharedButtonSizeKind
enum NexaSharedButtonSizeKind {
    case small
    case regular
    case large
}

// nexa-codegen: NexaSharedButtonShapeKind
enum NexaSharedButtonShapeKind {
    case circle
    case capsule
    case rounded(CGFloat)
}

// nexa-codegen: NexaSharedButtonOutline
private struct NexaSharedButtonOutline: Shape {
    let kind: NexaSharedButtonShapeKind

    func path(in rect: CGRect) -> Path {
        switch kind {
        case .circle: Circle().path(in: rect)
        case .capsule: Capsule().path(in: rect)
        case .rounded(let radius): RoundedRectangle(cornerRadius: radius).path(in: rect)
        }
    }
}

// nexa-codegen: NexaSharedBorderedButtonStyle
private struct NexaSharedBorderedButtonStyle: ButtonStyle {
    let tint: Color
    let shape: NexaSharedButtonOutline
    let size: NexaSharedButtonSizeKind?

    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        let padding: (horizontal: CGFloat, vertical: CGFloat) = switch size {
        case .small: (CGFloat(__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_SMALL_VERTICAL_PADDING__))
        case .large: (CGFloat(__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_LARGE_VERTICAL_PADDING__))
        case .regular, nil: (CGFloat(__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__))
        }
        let contentColor = isEnabled ? tint : tint.opacity(0.38)
        let borderColor = isEnabled ? tint : tint.opacity(0.12)
        configuration.label
            .foregroundStyle(contentColor)
            .padding(.horizontal, padding.horizontal)
            .padding(.vertical, padding.vertical)
            .frame(minWidth: CGFloat(__NEXA_BUTTON_MIN_WIDTH__), minHeight: size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__))
            .background(tint.opacity(configuration.isPressed ? 0.12 : 0), in: shape)
            .overlay(shape.stroke(borderColor, lineWidth: 1))
            .contentShape(shape)
    }
}

// nexa-codegen: NexaSharedGlassModifier
private struct NexaSharedGlassModifier: ViewModifier {
    let enabled: Bool
    let tint: Color?
    let shape: NexaSharedButtonShapeKind

    @ViewBuilder
    func body(content: Content) -> some View {
        if enabled {
            if #available(iOS 26.0, *) {
                let effect = tint.map { Glass.clear.tint($0).interactive() } ?? Glass.clear.interactive()
                switch shape {
                case .circle: content.glassEffect(effect, in: Circle())
                case .capsule: content.glassEffect(effect, in: Capsule())
                case .rounded(let radius): content.glassEffect(effect, in: RoundedRectangle(cornerRadius: radius))
                }
            } else {
                switch shape {
                case .circle: content.background(.ultraThinMaterial, in: Circle())
                case .capsule: content.background(.ultraThinMaterial, in: Capsule())
                case .rounded(let radius): content.background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: radius))
                }
            }
        } else {
            content
        }
    }
}

// nexa-codegen: nexaSharedButtonShape
private extension View {
    @ViewBuilder
    func nexaSharedButtonShape(_ shape: NexaSharedButtonShapeKind) -> some View {
        if #available(iOS 17.0, *) {
            switch shape {
            case .circle: self.buttonBorderShape(.circle)
            case .capsule: self.buttonBorderShape(.capsule)
            case .rounded(let radius): self.buttonBorderShape(.roundedRectangle(radius: radius))
            }
        } else {
            switch shape {
            case .circle: self.clipShape(Circle())
            case .capsule: self.clipShape(Capsule())
            case .rounded(let radius): self.clipShape(RoundedRectangle(cornerRadius: radius))
            }
        }
    }
}

private struct NexaSharedProminentButtonStyle: ButtonStyle {
    let tint: Color
    let shape: NexaSharedButtonOutline
    let size: NexaSharedButtonSizeKind?
    let iconOnlyCircle: Bool

    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        let padding: (horizontal: CGFloat, vertical: CGFloat) = switch size {
        case .small: (CGFloat(__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_SMALL_VERTICAL_PADDING__))
        case .large: (CGFloat(__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_LARGE_VERTICAL_PADDING__))
        case .regular, nil: (CGFloat(__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__), CGFloat(__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__))
        }
        let contentColor = isEnabled ? Color.white : Color.primary.opacity(0.38)
        let containerColor = isEnabled ? tint : Color.primary.opacity(0.12)
        let minWidth = iconOnlyCircle ? CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__) : CGFloat(__NEXA_BUTTON_MIN_WIDTH__)
        let minHeight = size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__)
        configuration.label
            .foregroundStyle(contentColor)
            .padding(.horizontal, iconOnlyCircle ? 0 : padding.horizontal)
            .padding(.vertical, iconOnlyCircle ? 0 : padding.vertical)
            .frame(minWidth: minWidth, minHeight: minHeight)
            .background(containerColor, in: shape)
            .overlay(shape.fill(Color.white.opacity(configuration.isPressed && isEnabled ? 0.10 : 0)))
            .contentShape(shape)
    }
}

// nexa-codegen: NexaButtonPrimitive
struct NexaButtonPrimitive<Label: View>: View {
    let style: NexaSharedButtonStyleKind
    let size: NexaSharedButtonSizeKind?
    let shape: NexaSharedButtonShapeKind
    let tint: Color?
    let loading: Bool
    let disabled: Bool
    let glass: Bool
    let iconOnlyCircle: Bool
    let action: () -> Void
    private let label: Label

    init(
        style: NexaSharedButtonStyleKind,
        size: NexaSharedButtonSizeKind?,
        shape: NexaSharedButtonShapeKind,
        tint: Color?,
        loading: Bool,
        disabled: Bool,
        glass: Bool,
        iconOnlyCircle: Bool,
        action: @escaping () -> Void,
        @ViewBuilder label: () -> Label
    ) {
        self.style = style
        self.size = size
        self.shape = shape
        self.tint = tint
        self.loading = loading
        self.disabled = disabled
        self.glass = glass
        self.iconOnlyCircle = iconOnlyCircle
        self.action = action
        self.label = label()
    }

    @ViewBuilder
    private var progressLabelContent: some View {
        ProgressView()
            .frame(
                width: CGFloat(__NEXA_BUTTON_LOADING_INDICATOR_SIZE__),
                height: CGFloat(__NEXA_BUTTON_LOADING_INDICATOR_SIZE__)
            )
    }

    @ViewBuilder
    private var labelContent: some View {
        if loading {
            progressLabelContent
        } else {
            label.font(.system(size: CGFloat(__NEXA_DEFAULT_BODY_FONT_SIZE__)))
        }
    }

    @ViewBuilder
    private var plainLabelContent: some View {
        if loading {
            progressLabelContent.tint(tint.map { ((disabled || loading) ? $0.opacity(0.38) : $0) })
        } else if let tint {
            label
                .font(.system(size: CGFloat(__NEXA_DEFAULT_BODY_FONT_SIZE__)))
                .foregroundStyle((disabled || loading) ? tint.opacity(0.38) : tint)
        } else {
            labelContent
        }
    }

    private func decorated<V: View>(_ view: V) -> some View {
        view
            .nexaSharedButtonShape(shape)
            .tint(tint ?? .blue)
            .modifier(NexaSharedGlassModifier(enabled: glass, tint: tint, shape: shape))
    }

    @ViewBuilder
    private func platformSize<V: View>(_ view: V) -> some View {
        switch size {
        case .small: decorated(view.controlSize(.small))
        case .regular: decorated(view.controlSize(.regular))
        case .large: decorated(view.controlSize(.large))
        case nil: decorated(view)
        }
    }

    @ViewBuilder
    var body: some View {
        switch style {
        case .bordered:
            let tint = tint ?? .blue
            let outline = NexaSharedButtonOutline(kind: shape)
            Button(action: action) { labelContent }
                .disabled(disabled || loading)
                .buttonStyle(NexaSharedBorderedButtonStyle(tint: tint, shape: outline, size: size))
                .modifier(NexaSharedGlassModifier(enabled: glass, tint: self.tint, shape: shape))
        case .borderedProminent:
            Button(action: action) { labelContent }
                .disabled(disabled || loading)
                .buttonStyle(NexaSharedProminentButtonStyle(
                    tint: tint ?? Color(uiColor: .systemBlue),
                    shape: NexaSharedButtonOutline(kind: shape),
                    size: size,
                    iconOnlyCircle: iconOnlyCircle
                ))
                .modifier(NexaSharedGlassModifier(enabled: glass, tint: tint, shape: shape))
        case .borderless:
            platformSize(
                Button(action: action) { plainLabelContent }
                    .disabled(disabled || loading)
                    .buttonStyle(.borderless)
                    .frame(minWidth: CGFloat(__NEXA_BUTTON_MIN_WIDTH__), minHeight: size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__))
            )
        case .plain:
            platformSize(
                Button(action: action) { plainLabelContent }
                    .disabled(disabled || loading)
                    .buttonStyle(.plain)
                    .frame(minWidth: CGFloat(__NEXA_BUTTON_MIN_WIDTH__), minHeight: size == .large ? CGFloat(__NEXA_BUTTON_LARGE_MIN_HEIGHT__) : CGFloat(__NEXA_BUTTON_MIN_TAP_TARGET__))
            )
        }
    }
}

// nexa-codegen: NexaSliderPrimitive
struct NexaSliderPrimitive: View {
    @Binding var value: Double
    let range: ClosedRange<Double>
    let step: Double

    @ViewBuilder var body: some View {
        if step > 0 {
            Slider(value: $value, in: range, step: step)
        } else {
            Slider(value: $value, in: range)
        }
    }
}

// nexa-codegen: NexaProgressBarPrimitive
struct NexaProgressBarPrimitive: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isVisible = false

    let progress: Double?
    let tint: Color?
    let trackColor: Color?
    let thickness: CGFloat?
    let cornerRadius: CGFloat?

    private var normalizedProgress: Double? {
        guard let progress else { return nil }
        guard progress.isFinite else { return 0 }
        return min(max(progress, 0), 1)
    }

    @ViewBuilder var body: some View {
        if let normalizedProgress {
            if trackColor == nil && thickness == nil && cornerRadius == nil {
                if let tint {
                    ProgressView(value: normalizedProgress, total: 1).tint(tint)
                } else {
                    ProgressView(value: normalizedProgress, total: 1)
                }
            } else {
                let barHeight = thickness ?? 4
                let radius = cornerRadius ?? barHeight / 2
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        RoundedRectangle(cornerRadius: radius)
                            .fill(trackColor ?? Color.primary.opacity(0.1))
                        RoundedRectangle(cornerRadius: radius)
                            .fill(tint ?? Color.accentColor)
                            .frame(width: max(geometry.size.width * CGFloat(normalizedProgress), 0))
                    }
                }
                .frame(height: barHeight)
                .accessibilityValue(Text("\(Int((normalizedProgress * 100).rounded()))%"))
            }
        } else {
            if let tint {
                indeterminateBar(tint: tint)
            } else {
                indeterminateBar(tint: Color.accentColor)
            }
        }
    }

    private func indeterminateBar(tint: Color) -> some View {
        let barHeight = thickness ?? 4
        let radius = cornerRadius ?? barHeight / 2
        return GeometryReader { geometry in
            let segmentWidth = min(max(geometry.size.width * 0.25, barHeight * 2), geometry.size.width)
            let shouldAnimate = isVisible && !reduceMotion
            ZStack(alignment: .leading) {
                RoundedRectangle(cornerRadius: radius)
                    .fill(trackColor ?? Color.primary.opacity(0.1))
                RoundedRectangle(cornerRadius: radius)
                    .fill(tint)
                    .frame(width: segmentWidth)
                    .offset(x: reduceMotion
                        ? (geometry.size.width - segmentWidth) / 2
                        : isVisible ? geometry.size.width : -segmentWidth)
            }
            .clipShape(RoundedRectangle(cornerRadius: radius))
            .animation(
                reduceMotion ? nil : .linear(duration: 1.2).repeatForever(autoreverses: false),
                value: shouldAnimate,
            )
            .onAppear { isVisible = true }
            .onDisappear { isVisible = false }
            .accessibilityRepresentation { ProgressView() }
        }
        .frame(height: barHeight)
    }
}

// nexa-codegen: NexaProgressRingPrimitive
struct NexaProgressRingPrimitive: View {
    let progress: Double?
    let tint: Color?
    let trackColor: Color?
    let size: CGFloat?
    let thickness: CGFloat?

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isAnimating = false

    private var normalizedProgress: Double? {
        guard let progress else { return nil }
        guard progress.isFinite else { return 0 }
        return min(max(progress, 0), 1)
    }

    private var indicatorColor: Color { tint ?? Color.accentColor }
    private var indicatorSize: CGFloat { max(size ?? 40, 0) }
    private var indicatorThickness: CGFloat { min(max(thickness ?? 4, 0), indicatorSize) }
    private var pathSize: CGFloat { max(indicatorSize - indicatorThickness, 0) }

    var body: some View {
        ZStack {
            Circle()
                .stroke(trackColor ?? indicatorColor.opacity(0.16), lineWidth: indicatorThickness)

            if let normalizedProgress {
                if normalizedProgress >= 1 {
                    Circle()
                        .stroke(indicatorColor, lineWidth: indicatorThickness)
                } else if normalizedProgress > 0 {
                    Circle()
                        .trim(from: 0, to: normalizedProgress)
                        .stroke(
                            indicatorColor,
                            style: StrokeStyle(lineWidth: indicatorThickness, lineCap: .round)
                        )
                        .rotationEffect(.degrees(-90))
                }
            } else {
                Circle()
                    .trim(from: 0, to: 0.25)
                    .stroke(
                        indicatorColor,
                        style: StrokeStyle(lineWidth: indicatorThickness, lineCap: .round)
                    )
                    .rotationEffect(.degrees(isAnimating ? 270 : -90))
            }
        }
        .frame(width: pathSize, height: pathSize)
        .frame(width: indicatorSize, height: indicatorSize)
        .animation(.easeInOut(duration: 0.2), value: normalizedProgress)
        .animation(
            reduceMotion ? nil : .linear(duration: 1).repeatForever(autoreverses: false),
            value: isAnimating,
        )
        .onAppear { isAnimating = !reduceMotion }
        .onDisappear { isAnimating = false }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Progress")
        .accessibilityValue(normalizedProgress.map { "\(Int(($0 * 100).rounded()))%" } ?? "In Progress")
    }
}

// nexa-codegen: NexaStatusBarBackgroundPrimitive
@MainActor
private func nexaStatusBarSafeAreaHeight() -> CGFloat {
    UIApplication.shared.connectedScenes
        .compactMap { $0 as? UIWindowScene }
        .first { $0.activationState == .foregroundActive }?
        .windows
        .first(where: \.isKeyWindow)?
        .safeAreaInsets.top ?? 0
}

struct NexaStatusBarBackgroundPrimitive: View {
    let color: Color

    var body: some View {
        color
            .frame(maxWidth: .infinity)
            .frame(height: nexaStatusBarSafeAreaHeight())
            .ignoresSafeArea(edges: .top)
            .allowsHitTesting(false)
    }
}

// nexa-codegen: NexaDividerPrimitive
struct NexaDividerPrimitive: View {
    let color: Color
    let thickness: CGFloat

    var body: some View {
        Rectangle()
            .fill(color)
            .frame(maxWidth: .infinity)
            .frame(height: max(0, thickness))
    }
}

// nexa-codegen: NexaSegmentedControlPrimitive
@MainActor
private struct NexaNativeSegmentedControl: UIViewRepresentable {
    let items: [String]
    @Binding var selection: String
    let tint: Color?
    let background: Color?
    let textColor: Color?
    let selectedTextColor: Color?
    let fontSize: CGFloat?
    let fontWeight: Font.Weight?
    let height: CGFloat?
    let cornerRadius: CGFloat?
    let borderColor: Color?
    let borderWidth: CGFloat?

    func makeCoordinator() -> Coordinator {
        Coordinator(selection: $selection, items: items)
    }

    func makeUIView(context: Context) -> UISegmentedControl {
        let control = UISegmentedControl(items: items)
        control.addTarget(
            context.coordinator,
            action: #selector(Coordinator.selectionChanged(_:)),
            for: .valueChanged
        )
        control.setContentHuggingPriority(.defaultLow, for: .horizontal)
        control.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return control
    }

    func updateUIView(_ control: UISegmentedControl, context: Context) {
        context.coordinator.selection = $selection
        context.coordinator.items = items

        let displayedItems = (0..<control.numberOfSegments).compactMap {
            control.titleForSegment(at: $0)
        }
        if displayedItems != items {
            control.removeAllSegments()
            for (index, item) in items.enumerated() {
                control.insertSegment(withTitle: item, at: index, animated: false)
            }
        }

        control.selectedSegmentIndex = items.firstIndex(of: selection) ?? UISegmentedControl.noSegment

        if let background {
            let color = UIColor(background)
            control.backgroundColor = color
            control.layer.backgroundColor = color.cgColor
        } else {
            control.backgroundColor = nil
            control.layer.backgroundColor = nil
        }

        // UIKit owns the selected capsule in every style. Its iOS 26 material may blend an
        // explicit track background, while tint and title attributes remain directly configurable.
        control.selectedSegmentTintColor = tint.map { UIColor($0) }
            ?? (background == nil ? nil : UIColor.systemBackground)

        let font = (fontSize != nil || fontWeight != nil)
            ? UIFont.systemFont(ofSize: fontSize ?? UIFont.systemFontSize, weight: Self.uiFontWeight(fontWeight))
            : nil
        var normalAttributes: [NSAttributedString.Key: Any] = [:]
        if let textColor { normalAttributes[.foregroundColor] = UIColor(textColor) }
        if let font { normalAttributes[.font] = font }
        control.setTitleTextAttributes(normalAttributes.isEmpty ? nil : normalAttributes, for: .normal)

        var selectedAttributes: [NSAttributedString.Key: Any] = [:]
        if let selectedTextColor { selectedAttributes[.foregroundColor] = UIColor(selectedTextColor) }
        if let font { selectedAttributes[.font] = font }
        control.setTitleTextAttributes(selectedAttributes.isEmpty ? nil : selectedAttributes, for: .selected)

        control.layer.cornerRadius = cornerRadius ?? (background == nil ? 0 : max(32, height ?? 32) / 2)
        control.layer.borderColor = borderColor.map { UIColor($0).cgColor }
        control.layer.borderWidth = max(0, borderWidth ?? (borderColor == nil ? 0 : 1))
        control.layer.masksToBounds = background != nil || borderColor != nil || (borderWidth ?? 0) > 0
    }

    private static func uiFontWeight(_ weight: Font.Weight?) -> UIFont.Weight {
        guard let weight else { return .regular }
        if weight == .ultraLight { return .ultraLight }
        if weight == .thin { return .thin }
        if weight == .light { return .light }
        if weight == .medium { return .medium }
        if weight == .semibold { return .semibold }
        if weight == .bold { return .bold }
        if weight == .heavy { return .heavy }
        if weight == .black { return .black }
        return .regular
    }

    @MainActor
    final class Coordinator: NSObject {
        var selection: Binding<String>
        var items: [String]

        init(selection: Binding<String>, items: [String]) {
            self.selection = selection
            self.items = items
        }

        @objc func selectionChanged(_ sender: UISegmentedControl) {
            guard items.indices.contains(sender.selectedSegmentIndex) else { return }
            selection.wrappedValue = items[sender.selectedSegmentIndex]
        }
    }
}

struct NexaSegmentedControlPrimitive: View {
    let items: [String]
    @Binding var selection: String
    var animation: Animation? = nil
    var tint: Color? = nil
    var background: Color? = nil
    var textColor: Color? = nil
    var selectedTextColor: Color? = nil
    var fontSize: CGFloat? = nil
    var fontWeight: Font.Weight? = nil
    var height: CGFloat? = nil
    var cornerRadius: CGFloat? = nil
    var borderColor: Color? = nil
    var borderWidth: CGFloat? = nil
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ViewBuilder
    var body: some View {
        if items.isEmpty {
            EmptyView()
        } else {
            NexaNativeSegmentedControl(
                items: items,
                selection: $selection,
                tint: tint,
                background: background,
                textColor: textColor,
                selectedTextColor: selectedTextColor,
                fontSize: fontSize,
                fontWeight: fontWeight,
                height: height,
                cornerRadius: cornerRadius,
                borderColor: borderColor,
                borderWidth: borderWidth
            )
            .frame(maxWidth: .infinity)
            .frame(height: max(32, height ?? 32))
            .transaction { transaction in
                if reduceMotion {
                    transaction.animation = nil
                    transaction.disablesAnimations = true
                } else if let animation {
                    transaction.animation = animation
                }
            }
        }
    }
}

// nexa-codegen: NexaSystemIconPrimitive
struct NexaSystemIconPrimitive: View {
    let symbol: String
    let description: String
    let size: CGFloat
    let tint: Color

    @ViewBuilder var body: some View {
        let icon = Image(systemName: symbol)
            .font(.system(size: size))
            .foregroundStyle(tint)
        if description.isEmpty {
            icon.accessibilityHidden(true)
        } else {
            icon.accessibilityLabel(description)
        }
    }
}

// nexa-codegen: NexaLinearGradientPrimitive
struct NexaLinearGradientPrimitive: View {
    let colors: [Color]
    let startPoint: UnitPoint
    let endPoint: UnitPoint
    let height: CGFloat

    var body: some View {
        LinearGradient(colors: colors, startPoint: startPoint, endPoint: endPoint)
            .frame(maxWidth: .infinity)
            .frame(height: height)
            .allowsHitTesting(false)
    }
}

// nexa-codegen: NexaContentUnavailablePrimitive
struct NexaContentUnavailablePrimitive: View {
    let title: Text
    let symbol: String
    let description: Text?
    let titleColor: Color?
    let descriptionColor: Color?
    let iconTint: Color?
    let background: Color?

    @ViewBuilder var body: some View {
        if #available(iOS 17.0, *) {
            ContentUnavailableView {
                Label {
                    if let titleColor { title.foregroundStyle(titleColor) } else { title }
                } icon: {
                    if let iconTint {
                        Image(systemName: symbol).foregroundStyle(iconTint)
                    } else {
                        Image(systemName: symbol)
                    }
                }
            } description: {
                if let description {
                    if let descriptionColor {
                        description.foregroundStyle(descriptionColor)
                    } else {
                        description
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background { if let background { background } }
        } else {
            VStack(spacing: 8) {
                Label {
                    if let titleColor { title.foregroundStyle(titleColor) } else { title }
                } icon: {
                    if let iconTint {
                        Image(systemName: symbol).foregroundStyle(iconTint)
                    } else {
                        Image(systemName: symbol)
                    }
                }
                    .font(.title3)
                if let description {
                    if let descriptionColor {
                        description.foregroundStyle(descriptionColor)
                            .multilineTextAlignment(.center)
                    } else {
                        description.foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(32)
            .background { if let background { background } }
        }
    }
}

// nexa-codegen: NexaColumnPrimitive
struct NexaColumnPrimitive<Content: View>: View {
    let alignment: HorizontalAlignment
    let textAlignment: TextAlignment
    let spacing: CGFloat
    private let content: Content

    init(
        alignment: HorizontalAlignment = .center,
        textAlignment: TextAlignment = .center,
        spacing: CGFloat = 0,
        @ViewBuilder content: () -> Content
    ) {
        self.alignment = alignment
        self.textAlignment = textAlignment
        self.spacing = spacing
        self.content = content()
    }

    var body: some View {
        VStack(alignment: alignment, spacing: spacing) { content }
            .multilineTextAlignment(textAlignment)
    }
}

// nexa-codegen: NexaRowPrimitive
enum NexaRowArrangement {
    case start
    case center
    case end

    var frameAlignment: Alignment {
        switch self {
        case .start: .leading
        case .center: .center
        case .end: .trailing
        }
    }
}

struct NexaRowPrimitive<Content: View>: View {
    let alignment: VerticalAlignment
    let spacing: CGFloat
    let horizontalArrangement: NexaRowArrangement?
    private let content: Content

    init(
        alignment: VerticalAlignment = .center,
        spacing: CGFloat = 0,
        horizontalArrangement: NexaRowArrangement? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.alignment = alignment
        self.spacing = spacing
        self.horizontalArrangement = horizontalArrangement
        self.content = content()
    }

    var body: some View {
        if let horizontalArrangement {
            HStack(alignment: alignment, spacing: spacing) { content }
                .frame(maxWidth: .infinity, alignment: horizontalArrangement.frameAlignment)
                .multilineTextAlignment(.leading)
        } else {
            HStack(alignment: alignment, spacing: spacing) { content }
                .multilineTextAlignment(.leading)
        }
    }
}

// nexa-codegen: NexaStackPrimitive
struct NexaStackPrimitive<Content: View>: View {
    let alignment: Alignment
    private let content: Content

    init(alignment: Alignment = .center, @ViewBuilder content: () -> Content) {
        self.alignment = alignment
        self.content = content()
    }

    var body: some View {
        ZStack(alignment: alignment) { content }
    }
}

// nexa-codegen: NexaContentSizePreferenceKey
private struct NexaContentSizePreferenceKey: PreferenceKey {
    static var defaultValue: [UUID: CGSize] { [:] }

    static func reduce(value: inout [UUID: CGSize], nextValue: () -> [UUID: CGSize]) {
        value.merge(nextValue()) { _, latest in latest }
    }
}

// nexa-codegen: NexaAnimateContentSizeModifier
struct NexaAnimateContentSizeModifier: ViewModifier {
    let animation: Animation

    @State private var contentSize: CGSize?
    @State private var preferenceID = UUID()
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            // Keep the child at its final intrinsic size. Only the outer frame
            // interpolates, so the Stack reveals/clips content like Compose's
            // animateContentSize instead of moving or fading the child itself.
            .transaction { transaction in
                transaction.animation = nil
            }
            .fixedSize(horizontal: false, vertical: true)
            .background {
                GeometryReader { geometry in
                    Color.clear.preference(
                        key: NexaContentSizePreferenceKey.self,
                        value: [preferenceID: geometry.size]
                    )
                }
            }
            .onPreferenceChange(NexaContentSizePreferenceKey.self) { measurements in
                guard let newSize = measurements[preferenceID],
                      newSize.width.isFinite,
                      newSize.height.isFinite else {
                    return
                }

                guard let contentSize else {
                    var transaction = Transaction()
                    transaction.disablesAnimations = true
                    withTransaction(transaction) {
                        self.contentSize = newSize
                    }
                    return
                }

                guard abs(newSize.width - contentSize.width) > 0.5
                    || abs(newSize.height - contentSize.height) > 0.5 else {
                    return
                }
                if reduceMotion {
                    self.contentSize = newSize
                } else {
                    withAnimation(animation) {
                        self.contentSize = newSize
                    }
                }
            }
            .frame(width: contentSize?.width, height: contentSize?.height, alignment: .topLeading)
            .clipped()
    }
}

// Keep Stack's source order explicit in both drawing and hit testing. This
// matters when an earlier child hosts UIKit content (for example FastList)
// that otherwise captures touches intended for a later overlay child.
struct NexaStackLayer<Content: View>: View {
    let order: Double
    private let content: Content

    init(order: Int, @ViewBuilder content: () -> Content) {
        self.order = Double(order)
        self.content = content()
    }

    var body: some View {
        content.zIndex(order)
    }
}

// nexa-codegen: NexaNativeListPrimitive
@Observable
@MainActor
private final class NexaFastListReorderSession {
    private(set) var activeIndex: Int?
    private(set) var dragOffset: CGFloat = 0
    @ObservationIgnored private var rowFrames: [Int: CGRect] = [:]
    @ObservationIgnored private var startY: CGFloat = 0
    @ObservationIgnored private var accumulatedLayoutShift: CGFloat = 0
    @ObservationIgnored private var lastInsertionIndex: Int?

    func updateFrames(_ frames: [Int: CGRect]) {
        rowFrames = frames
    }

    func begin(at index: Int, startY: CGFloat) {
        activeIndex = index
        self.startY = startY
        accumulatedLayoutShift = 0
        dragOffset = 0
        lastInsertionIndex = nil
    }

    func update(to location: CGPoint, perform onMove: (Int, Int) -> Void) {
        guard let source = activeIndex,
              let sourceFrame = rowFrames[source],
              let target = rowFrames.min(by: {
                  abs($0.value.midY - location.y) < abs($1.value.midY - location.y)
              })
        else { return }

        let insertionIndex = target.key + (location.y >= target.value.midY ? 1 : 0)
        guard lastInsertionIndex != insertionIndex else {
            dragOffset = location.y - startY - accumulatedLayoutShift
            return
        }
        lastInsertionIndex = insertionIndex

        let destination = insertionIndex > source ? insertionIndex - 1 : insertionIndex
        guard destination != source else {
            dragOffset = location.y - startY - accumulatedLayoutShift
            return
        }

        let targetCenter = rowFrames[destination]?.midY ?? target.value.midY
        accumulatedLayoutShift += targetCenter - sourceFrame.midY
        withAnimation(.default) {
            onMove(source, insertionIndex)
        }
        activeIndex = destination
        dragOffset = location.y - startY - accumulatedLayoutShift
    }

    func finish() {
        activeIndex = nil
        dragOffset = 0
        accumulatedLayoutShift = 0
        lastInsertionIndex = nil
    }
}

private struct NexaFastListRowFramesKey: PreferenceKey {
    static let defaultValue: [Int: CGRect] = [:]

    static func reduce(value: inout [Int: CGRect], nextValue: () -> [Int: CGRect]) {
        value.merge(nextValue(), uniquingKeysWith: { _, next in next })
    }
}

@MainActor
private struct NexaFastListReorderableRow<Content: View>: View {
    let index: Int
    let onReorder: (Int, Int) -> Void
    private let content: Content
    @Bindable private var session: NexaFastListReorderSession

    init(
        index: Int,
        session: NexaFastListReorderSession,
        onReorder: @escaping (Int, Int) -> Void,
        @ViewBuilder content: () -> Content
    ) {
        self.index = index
        self.onReorder = onReorder
        self._session = Bindable(wrappedValue: session)
        self.content = content()
    }

    var body: some View {
        let isDragging = session.activeIndex == index
        ZStack {
            content.offset(y: isDragging ? session.dragOffset : 0)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background {
            GeometryReader { geometry in
                Color.clear.preference(
                    key: NexaFastListRowFramesKey.self,
                    value: [index: geometry.frame(in: .global)]
                )
            }
        }
        .contentShape(Rectangle())
        .zIndex(isDragging ? 1 : 0)
        .highPriorityGesture(
            LongPressGesture(minimumDuration: 0.35)
                .sequenced(before: DragGesture(minimumDistance: 0, coordinateSpace: .global))
                .onChanged { value in
                    guard case let .second(true, drag?) = value else { return }
                    if session.activeIndex == nil {
                        session.begin(at: index, startY: drag.startLocation.y)
                    }
                    session.update(to: drag.location, perform: onReorder)
                }
                .onEnded { _ in session.finish() }
        )
    }
}

private struct NexaFastListReorderSessionKey: EnvironmentKey {
    static let defaultValue: NexaFastListReorderSession? = nil
}

private extension EnvironmentValues {
    var nexaFastListReorderSession: NexaFastListReorderSession? {
        get { self[NexaFastListReorderSessionKey.self] }
        set { self[NexaFastListReorderSessionKey.self] = newValue }
    }
}

struct NexaNativeListPrimitive<Content: View>: View {
    private let spacing: CGFloat
    private let scrollEnabled: Bool
    private let content: Content
    @State private var reorderSession: NexaFastListReorderSession?

    init(
        spacing: CGFloat = 0,
        scrollEnabled: Bool = true,
        reorderEnabled: Bool = false,
        @ViewBuilder content: () -> Content
    ) {
        self.spacing = spacing
        self.scrollEnabled = scrollEnabled
        _reorderSession = State(
            initialValue: reorderEnabled ? NexaFastListReorderSession() : nil
        )
        self.content = content()
    }

    var body: some View {
        List {
            content.environment(\.nexaFastListReorderSession, reorderSession)
        }
            .onPreferenceChange(NexaFastListRowFramesKey.self) { frames in
                reorderSession?.updateFrames(frames)
            }
            .listRowSpacing(spacing)
            .scrollDisabled(!scrollEnabled)
            .modifier(NexaNativeListStyleModifier())
    }
}

// nexa-codegen: NexaNativeListRowPrimitive
@MainActor
private struct NexaFastListSwipePagerGestureBridge: UIViewRepresentable {
    func makeUIView(context: Context) -> NexaFastListSwipePagerGestureView {
        NexaFastListSwipePagerGestureView()
    }

    func updateUIView(_ view: NexaFastListSwipePagerGestureView, context: Context) {
        view.connectToContainingPager()
    }
}

@MainActor
private final class NexaFastListSwipePagerGestureView: UIView, UIGestureRecognizerDelegate {
    private weak var connectedRowCell: UIView?
    private weak var connectedPagerScrollView: UIScrollView?
    private weak var connectedListScrollView: UIScrollView?
    private var rowTouchRecognizer: UILongPressGestureRecognizer?
    private var connectionCheckPending = false

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil {
            disconnectFromContainingPager()
            return
        }
        connectToContainingPager()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        connectToContainingPager()
    }

    func connectToContainingPager() {
        guard !connectionCheckPending else { return }
        connectionCheckPending = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.connectionCheckPending = false
            guard self.window != nil else { return }

            var ancestor = self.superview
            var rowCell: UIView?
            var listScrollView: UIScrollView?
            var pagerScrollView: UIScrollView?
            while let view = ancestor {
                if rowCell == nil,
                   view is UITableViewCell || view is UICollectionViewCell {
                    rowCell = view
                }
                if let scrollView = view as? UIScrollView {
                    if listScrollView == nil {
                        listScrollView = scrollView
                    } else if isHorizontalPageContainer(scrollView) {
                        // SwiftUI's page TabView is backed by a horizontal UICollectionView.
                        // On current iOS versions it does not consistently set isPagingEnabled,
                        // even though its collection content spans the pages horizontally.
                        pagerScrollView = scrollView
                        break
                    }
                }
                ancestor = view.superview
            }
            guard let rowCell, let listScrollView, let pagerScrollView else { return }
            guard connectedRowCell !== rowCell
                    || connectedListScrollView !== listScrollView
                    || connectedPagerScrollView !== pagerScrollView
            else { return }
            disconnectFromContainingPager()

            // A zero-duration, non-cancelling recognizer claims the row touch
            // before SwiftUI's ancestor page TabView can start paging. It
            // recognizes simultaneously with the native List gestures, so
            // SwiftUI still owns and renders the row swipe action.
            let rowTouchRecognizer = UILongPressGestureRecognizer(
                target: self,
                action: #selector(rowTouchRecognized(_:))
            )
            rowTouchRecognizer.minimumPressDuration = 0
            rowTouchRecognizer.allowableMovement = .greatestFiniteMagnitude
            rowTouchRecognizer.cancelsTouchesInView = false
            rowTouchRecognizer.delaysTouchesBegan = false
            rowTouchRecognizer.delaysTouchesEnded = false
            rowTouchRecognizer.delegate = self
            rowCell.addGestureRecognizer(rowTouchRecognizer)
            pagerScrollView.panGestureRecognizer.require(toFail: rowTouchRecognizer)

            connectedRowCell = rowCell
            connectedListScrollView = listScrollView
            connectedPagerScrollView = pagerScrollView
            self.rowTouchRecognizer = rowTouchRecognizer
        }
    }

    private func isHorizontalPageContainer(_ scrollView: UIScrollView) -> Bool {
        guard let collectionView = scrollView as? UICollectionView else {
            return scrollView.isPagingEnabled
        }
        return scrollView.isPagingEnabled
            || (collectionView.contentSize.width > collectionView.bounds.width + 1
                && collectionView.contentSize.height <= collectionView.bounds.height + 1)
    }

    func gestureRecognizer(
        _ gestureRecognizer: UIGestureRecognizer,
        shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer
    ) -> Bool {
        guard gestureRecognizer === rowTouchRecognizer || otherGestureRecognizer === rowTouchRecognizer else {
            return false
        }
        guard let rowCell = connectedRowCell else { return false }
        let peer = gestureRecognizer === rowTouchRecognizer ? otherGestureRecognizer : gestureRecognizer
        var view = peer.view
        while let current = view {
            if current === rowCell { return true }
            if current === connectedListScrollView { return true }
            view = current.superview
        }
        return false
    }

    @objc private func rowTouchRecognized(_ recognizer: UIGestureRecognizer) {
        // The recognizer exists only to establish the failure dependency with
        // the outer pager. Native List recognizers handle the actual gesture.
        guard recognizer.state == .ended
                || recognizer.state == .cancelled
                || recognizer.state == .failed
        else { return }
    }

    private func disconnectFromContainingPager() {
        if let rowCell = connectedRowCell, let rowTouchRecognizer {
            rowCell.removeGestureRecognizer(rowTouchRecognizer)
        }
        rowTouchRecognizer = nil
        connectedRowCell = nil
        connectedListScrollView = nil
        connectedPagerScrollView = nil
    }
}

struct NexaNativeListRowPrimitive<Content: View>: View {
    let showDividers: Bool
    let prioritizeSwipeActions: Bool
    let reorderIndex: Int?
    let reorderEnabled: Bool
    let onReorder: ((Int, Int) -> Void)?
    private let content: Content
    @Environment(\.nexaFastListReorderSession) private var reorderSession

    init(
        showDividers: Bool = true,
        prioritizeSwipeActions: Bool = false,
        reorderIndex: Int? = nil,
        reorderEnabled: Bool = false,
        onReorder: ((Int, Int) -> Void)? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.showDividers = showDividers
        self.prioritizeSwipeActions = prioritizeSwipeActions
        self.reorderIndex = reorderIndex
        self.reorderEnabled = reorderEnabled
        self.onReorder = onReorder
        self.content = content()
    }

    var body: some View {
        let row = Group {
            if prioritizeSwipeActions {
                content.background {
                    NexaFastListSwipePagerGestureBridge()
                        .frame(width: 1, height: 1)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }
            } else {
                content
            }
        }
        Group {
            if reorderEnabled, let reorderIndex, let onReorder, let reorderSession {
                NexaFastListReorderableRow(
                    index: reorderIndex,
                    session: reorderSession,
                    onReorder: onReorder
                ) {
                    row
                    }
            } else {
                row
            }
        }
            .listRowInsets(EdgeInsets(
                top: CGFloat(__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__),
                leading: CGFloat(__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__),
                bottom: CGFloat(__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__),
                trailing: CGFloat(__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__)
            ))
            .listRowBackground(Color(uiColor: .secondarySystemGroupedBackground))
            .listRowSeparatorTint(Color(uiColor: .separator))
            .listRowSeparator(showDividers ? .visible : .hidden)
    }
}

// nexa-codegen: NexaNativeListStyleModifier
struct NexaNativeListStyleModifier: ViewModifier {
    func body(content: Content) -> some View {
        let groupedList = content
            .listStyle(.insetGrouped)
            .scrollContentBackground(.hidden)
            .background(Color(uiColor: .systemGroupedBackground))
        if #available(iOS 17.0, *) {
            groupedList.contentMargins(
                .horizontal,
                CGFloat(__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__),
                for: .scrollContent
            )
            .contentMargins(
                .vertical,
                CGFloat(__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__),
                for: .scrollContent
            )
        } else {
            // Before iOS 17, insetGrouped List has no public contentMargins API.
            // Expand only the container so its system inset cancels out; row
            // content keeps the explicit listRowInsets above.
            groupedList.padding(
                .horizontal,
                -CGFloat(__NEXA_IOS_LEGACY_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__)
            )
        }
    }
}

// nexa-codegen: NexaFormPrimitive
struct NexaFormPrimitive<Content: View>: View {
    let background: Color?
    private let content: Content
    @Environment(\.multilineTextAlignment) private var inheritedTextAlignment

    init(background: Color? = nil, @ViewBuilder content: () -> Content) {
        self.background = background
        self.content = content()
    }

    var body: some View {
        Form { content }
            .multilineTextAlignment(inheritedTextAlignment)
            .scrollContentBackground(.hidden)
            .background(background ?? Color.clear)
    }
}

// nexa-codegen: NexaFormSectionPrimitive
struct NexaFormSectionPrimitive<Content: View, Header: View, Footer: View>: View {
    let background: Color?
    private let content: Content
    private let header: Header
    private let footer: Footer

    init(
        background: Color? = nil,
        @ViewBuilder content: () -> Content,
        @ViewBuilder header: () -> Header,
        @ViewBuilder footer: () -> Footer
    ) {
        self.background = background
        self.content = content()
        self.header = header()
        self.footer = footer()
    }

    var body: some View {
        Section {
            content
        } header: {
            header.foregroundStyle(.secondary)
        } footer: {
            footer
        }
        // Compose form sections draw row dividers but no section-edge rules.
        // Hide SwiftUI's automatic section separators to keep the shared layout aligned.
        .listSectionSeparator(.hidden)
        .listRowBackground(background ?? Color.clear)
    }
}

// nexa-codegen: NexaFormRowPrimitive
struct NexaFormRowPrimitive<Content: View>: View {
    let minHeight: CGFloat
    let horizontalInset: CGFloat
    let showDividers: Bool
    private let content: Content

    init(
        minHeight: CGFloat,
        horizontalInset: CGFloat,
        showDividers: Bool = true,
        @ViewBuilder content: () -> Content
    ) {
        self.minHeight = minHeight
        self.horizontalInset = horizontalInset
        self.showDividers = showDividers
        self.content = content()
    }

    var body: some View {
        content
            .frame(maxWidth: .infinity, minHeight: minHeight, alignment: .leading)
            .listRowInsets(EdgeInsets(top: 0, leading: horizontalInset, bottom: 0, trailing: horizontalInset))
            .listRowSeparator(showDividers ? .visible : .hidden)
    }
}

// nexa-codegen: NexaAccessibilityPrimitive
struct NexaAccessibilityPrimitive<Content: View>: View {
    let label: Text
    let hint: Text?
    let value: Text?
    let role: String
    let omitLabel: Bool
    private let content: Content

    init(
        label: Text,
        hint: Text?,
        value: Text?,
        role: String,
        omitLabel: Bool,
        @ViewBuilder content: () -> Content
    ) {
        self.label = label
        self.hint = hint
        self.value = value
        self.role = role
        self.omitLabel = omitLabel
        self.content = content()
    }

    @ViewBuilder
    private func addingLabel<V: View>(to view: V) -> some View {
        if omitLabel { view } else { view.accessibilityLabel(label) }
    }

    @ViewBuilder
    private func addingHint<V: View>(to view: V) -> some View {
        if let hint { view.accessibilityHint(hint) } else { view }
    }

    @ViewBuilder
    private func addingValue<V: View>(to view: V) -> some View {
        if let value { view.accessibilityValue(value) } else { view }
    }

    @ViewBuilder
    private func addingRole<V: View>(to view: V) -> some View {
        switch role {
        case "Button": view.accessibilityAddTraits(.isButton)
        case "Link": view.accessibilityAddTraits(.isLink)
        case "Header": view.accessibilityAddTraits(.isHeader)
        case "Image": view.accessibilityAddTraits(.isImage)
        default: view
        }
    }

    var body: some View {
        addingRole(to: addingValue(to: addingHint(to: addingLabel(
            to: content.accessibilityElement(children: .combine)
        ))))
    }
}

// nexa-codegen: NexaAppearancePrimitive
struct NexaAppearancePrimitive<Content: View>: View {
    let mode: String
    private let content: Content
    @Environment(\.colorScheme) private var systemColorScheme

    init(mode: String, @ViewBuilder content: () -> Content) {
        self.mode = mode
        self.content = content()
    }

    private var resolvedColorScheme: ColorScheme {
        switch mode {
        case "dark": .dark
        case "light": .light
        default: systemColorScheme
        }
    }

    private var preferredColorScheme: ColorScheme? {
        switch mode {
        case "dark": .dark
        case "light": .light
        default: nil
        }
    }

    var body: some View {
        content
            .environment(\.colorScheme, resolvedColorScheme)
            .preferredColorScheme(preferredColorScheme)
    }
}

// nexa-codegen: NexaLinkPrimitive
struct NexaLinkPrimitive<Content: View>: View {
    let destination: URL?
    private let content: Content

    init(destination: URL?, @ViewBuilder content: () -> Content) {
        self.destination = destination
        self.content = content()
    }

    @ViewBuilder
    var body: some View {
        if let destination {
            Link(destination: destination) { content }
        } else {
            content
        }
    }
}

// nexa-codegen: NexaKeyboardAwarePrimitive
struct NexaKeyboardAwarePrimitive<Content: View>: View {
    let dismissMode: ScrollDismissesKeyboardMode
    private let content: Content

    init(dismissMode: ScrollDismissesKeyboardMode, @ViewBuilder content: () -> Content) {
        self.dismissMode = dismissMode
        self.content = content()
    }

    var body: some View {
        ScrollView(.vertical) { content }
            .scrollDismissesKeyboard(dismissMode)
    }
}

// nexa-codegen: NexaRefreshPrimitive
struct NexaRefreshPrimitive<Content: View>: View {
    private let onRefresh: @Sendable () async -> Void
    private let content: Content

    init(onRefresh: @escaping @Sendable () async -> Void, @ViewBuilder content: () -> Content) {
        self.onRefresh = onRefresh
        self.content = content()
    }

    var body: some View {
        ScrollView(.vertical) { content }
            .refreshable(action: onRefresh)
    }
}

// nexa-codegen: NexaRefreshBounceModifier
private struct NexaRefreshBounceModifier: ViewModifier {
    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 16.4, *) {
            content.scrollBounceBehavior(.always, axes: .vertical)
        } else {
            content
        }
    }
}

// nexa-codegen: NexaBottomSheetPrimitive
struct NexaBottomSheetPrimitive<Content: View>: View {
    @Binding var isPresented: Bool
    let contentPadding: CGFloat
    let detents: Set<PresentationDetent>
    let snapPoints: [CGFloat]
    let initialSnapPoint: CGFloat?
    let selectedSnapPoint: Binding<Double>?
    let initialDetent: PresentationDetent
    let showDragIndicator: Bool
    let allowsDismissal: Bool
    let allowsBackgroundInteraction: Bool
    let shouldDismissOnBackPress: Bool
    let backgroundColor: Color?
    let onDismissRequest: () -> Void
    let onDismiss: () -> Void
    @State private var selectedDetent: PresentationDetent
    private let content: Content

    init(
        isPresented: Binding<Bool>,
        contentPadding: CGFloat,
        detents: Set<PresentationDetent>,
        snapPoints: [CGFloat],
        initialSnapPoint: CGFloat?,
        selectedSnapPoint: Binding<Double>?,
        initialDetent: PresentationDetent,
        showDragIndicator: Bool,
        allowsDismissal: Bool,
        allowsBackgroundInteraction: Bool,
        shouldDismissOnBackPress: Bool,
        backgroundColor: Color?,
        scrimColor: Color?,
        onDismissRequest: @escaping () -> Void,
        onDismiss: @escaping () -> Void,
        @ViewBuilder content: () -> Content
    ) {
        self._isPresented = isPresented
        self.contentPadding = contentPadding
        self.detents = detents
        self.snapPoints = snapPoints
        self.initialSnapPoint = initialSnapPoint
        self.selectedSnapPoint = selectedSnapPoint
        self.initialDetent = initialDetent
        self.showDragIndicator = showDragIndicator
        self.allowsDismissal = allowsDismissal
        self.allowsBackgroundInteraction = allowsBackgroundInteraction
        self.shouldDismissOnBackPress = shouldDismissOnBackPress
        self.backgroundColor = backgroundColor
        self.onDismissRequest = onDismissRequest
        self.onDismiss = onDismiss
        let selectedInitialPoint = selectedSnapPoint.flatMap { binding in
            snapPoints.min {
                abs(Double($0) - binding.wrappedValue)
                    < abs(Double($1) - binding.wrappedValue)
            }
        }
        _selectedDetent = State(
            initialValue: selectedInitialPoint.map { .fraction($0) }
                ?? initialSnapPoint.map { .fraction($0) }
                ?? initialDetent
        )
        self.content = content()
    }

    private var configuredDetents: Set<PresentationDetent> {
        snapPoints.isEmpty ? detents : Set(snapPoints.map { .fraction($0) })
    }

    private var configuredInitialDetent: PresentationDetent {
        if let selectedSnapPoint,
           let point = closestSnapPoint(to: selectedSnapPoint.wrappedValue) {
            return .fraction(point)
        }
        return initialSnapPoint.map { .fraction($0) } ?? initialDetent
    }

    private func closestSnapPoint(to requested: Double) -> CGFloat? {
        snapPoints.min { abs(Double($0) - requested) < abs(Double($1) - requested) }
    }

    @ViewBuilder var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .sheet(isPresented: $isPresented, onDismiss: {
                onDismissRequest()
                onDismiss()
            }) {
                NexaBottomSheetContentPrimitive(contentPadding: contentPadding) {
                    content
                }
                    .presentationDetents(configuredDetents, selection: $selectedDetent)
                    .presentationDragIndicator(showDragIndicator ? .visible : .hidden)
                    .interactiveDismissDisabled(!allowsDismissal)
                    .modifier(NexaBottomSheetBackground(
                        color: backgroundColor ?? Color(uiColor: .systemBackground)
                    ))
                    .modifier(NexaBottomSheetBackgroundInteraction(
                        allowsBackgroundInteraction: allowsBackgroundInteraction
                    ))
            }
            .onChange(of: isPresented) { presented in
                if presented {
                    selectedDetent = configuredInitialDetent
                }
            }
            .onChange(of: selectedSnapPoint?.wrappedValue) { value in
                guard let value, let point = closestSnapPoint(to: value) else { return }
                selectedDetent = .fraction(point)
                let snappedValue = Double(point)
                if snappedValue != value {
                    selectedSnapPoint?.wrappedValue = snappedValue
                }
            }
            .onChange(of: selectedDetent) { detent in
                guard let selectedSnapPoint,
                      let point = snapPoints.first(where: { detent == .fraction($0) })
                else { return }
                let value = Double(point)
                if selectedSnapPoint.wrappedValue != value {
                    selectedSnapPoint.wrappedValue = value
                }
            }
    }
}

// nexa-codegen: NexaBottomSheetContentPrimitive
private struct NexaBottomSheetContentPrimitive<Content: View>: View {
    let contentPadding: CGFloat
    private let content: Content

    init(contentPadding: CGFloat, @ViewBuilder content: () -> Content) {
        self.contentPadding = contentPadding
        self.content = content()
    }

    var body: some View {
        NexaColumnPrimitive(
            spacing: 0
        ) {
            content
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .padding(contentPadding)
    }
}

// SwiftUI owns a native sheet's dim layer. Background interaction is the
// explicit cross-platform opt-in for an undimmed, interactive backdrop.
// nexa-codegen: NexaBottomSheetBackgroundInteraction
private struct NexaBottomSheetBackgroundInteraction: ViewModifier {
    let allowsBackgroundInteraction: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 16.4, *), allowsBackgroundInteraction {
            content.presentationBackgroundInteraction(.enabled)
        } else {
            content
        }
    }
}

// nexa-codegen: NexaBottomSheetBackground
private struct NexaBottomSheetBackground: ViewModifier {
    let color: Color?

    @ViewBuilder
    func body(content: Content) -> some View {
        if let color {
            if #available(iOS 16.4, *) {
                content.presentationBackground(color)
            } else {
                content.background(color)
            }
        } else {
            content
        }
    }
}

// nexa-codegen: NexaAlertDialogPrimitive
struct NexaAlertDialogPrimitive<Actions: View>: View {
    @Binding var isPresented: Bool
    let title: Text
    let message: Text?
    private let actions: Actions

    init(
        isPresented: Binding<Bool>,
        title: Text,
        message: Text?,
        @ViewBuilder actions: () -> Actions
    ) {
        self._isPresented = isPresented
        self.title = title
        self.message = message
        self.actions = actions()
    }

    @ViewBuilder var body: some View {
        let host = Color.clear.frame(width: 0, height: 0)
        if let message {
            host.alert(title, isPresented: $isPresented) {
                actions
            } message: {
                message
            }
        } else {
            host.alert(title, isPresented: $isPresented) {
                actions
            }
        }
    }
}

// nexa-codegen: NexaConfirmationDialogPrimitive
struct NexaConfirmationDialogPrimitive<Actions: View>: View {
    @Binding var isPresented: Bool
    let title: Text
    private let actions: Actions

    init(
        isPresented: Binding<Bool>,
        title: Text,
        @ViewBuilder actions: () -> Actions
    ) {
        self._isPresented = isPresented
        self.title = title
        self.actions = actions()
    }

    var body: some View {
        Color.clear.frame(width: 0, height: 0)
            .confirmationDialog(title, isPresented: $isPresented, titleVisibility: .visible) {
                actions
            }
    }
}

// nexa-codegen: NexaPageIndicatorPrimitive
struct NexaPageIndicatorPrimitive: View {
    let index: Int
    let selectedIndex: Int
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            Circle()
                .fill(selectedIndex == index ? __NEXA_SWIFT_DEFAULT_ACCENT_COLOR__ : __NEXA_PAGE_INDICATOR_MUTED_COLOR__.opacity(__NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__))
                .frame(
                    width: selectedIndex == index ? __NEXA_PAGE_INDICATOR_SELECTED_SIZE__ : __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__,
                    height: selectedIndex == index ? __NEXA_PAGE_INDICATOR_SELECTED_SIZE__ : __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__
                )
        }
        .buttonStyle(.plain)
        .animation(.default, value: selectedIndex)
        .accessibilityLabel("Page \(index + 1)")
    }
}

// nexa-codegen: NexaPageIndicatorRow
struct NexaPageIndicatorRow: View {
    let pageCount: Int
    let selectedIndex: Int
    let onSelect: (Int) -> Void

    var body: some View {
        HStack(spacing: __NEXA_PAGE_INDICATOR_SPACING__) {
            ForEach(0..<pageCount, id: \.self) { index in
                NexaPageIndicatorPrimitive(index: index, selectedIndex: selectedIndex) {
                    onSelect(index)
                }
            }
        }
        .padding(.bottom, __NEXA_PAGE_INDICATOR_BOTTOM_INSET__)
    }
}

// nexa-codegen: NexaNavigationSplitViewPrimitive
struct NexaNavigationSplitViewPrimitive<Sidebar: View, Detail: View>: View {
    @Binding var detailVisible: Bool
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    private let sidebar: Sidebar
    private let detail: Detail

    init(
        detailVisible: Binding<Bool>,
        @ViewBuilder sidebar: () -> Sidebar,
        @ViewBuilder detail: () -> Detail
    ) {
        _detailVisible = detailVisible
        self.sidebar = sidebar()
        self.detail = detail()
    }

    @ViewBuilder
    var body: some View {
        if horizontalSizeClass == .compact {
            Group {
                if detailVisible {
                    detail
                        .transition(.move(edge: .trailing))
                } else {
                    sidebar
                        .transition(.move(edge: .leading))
                }
            }
            .animation(.default, value: detailVisible)
        } else if #available(iOS 17.0, *) {
            NavigationSplitView(preferredCompactColumn: Binding(
                get: { detailVisible ? .detail : .sidebar },
                set: { detailVisible = ($0 == .detail) }
            )) {
                sidebar
            } detail: {
                detail
            }
        } else if #available(iOS 16.0, *) {
            NavigationSplitView {
                sidebar
            } detail: {
                detail
            }
        } else {
            HStack(spacing: 0) {
                TupleView((sidebar, Divider(), detail))
            }
        }
    }
}

// nexa-codegen: NexaNavigationScreenPrimitive
struct NexaNavigationScreenPrimitive<Content: View>: View {
    let title: String
    let inline: Bool
    let leadingTitle: Bool
    let content: Content

    init(
        title: String,
        inline: Bool,
        leadingTitle: Bool,
        @ViewBuilder content: () -> Content
    ) {
        self.title = title
        self.inline = inline
        self.leadingTitle = leadingTitle
        self.content = content()
    }

    var body: some View {
        content
            .navigationTitle(inline && leadingTitle ? "" : title)
            .modifier(NexaNavigationTitleDisplayModeModifier(inline: inline))
            .toolbar {
                if inline && leadingTitle {
                    ToolbarItem(placement: .topBarLeading) {
                        leadingNavigationTitle
                    }
                }
            }
            .toolbar(.hidden, for: .tabBar)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private var leadingNavigationTitle: some View {
        Text(title)
            .font(.headline)
            .lineLimit(1)
            .truncationMode(.tail)
            .layoutPriority(1)
            .frame(minWidth: 160, alignment: .leading)
            .offset(x: -13)
            .accessibilityLabel(title)
            .accessibilityAddTraits(.isHeader)
    }
}

// nexa-codegen: NexaNavigationTitleDisplayModeModifier
struct NexaNavigationTitleDisplayModeModifier: ViewModifier {
    let inline: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 17.0, *) {
            content.toolbarTitleDisplayMode(inline ? .inline : .large)
        } else {
            content.navigationBarTitleDisplayMode(inline ? .inline : .large)
        }
    }
}

// nexa-codegen: NexaNavigationLinkPrimitive
struct NexaNavigationLinkPrimitive<Value: Hashable, Label: View>: View {
    let value: Value
    let enabled: Bool
    let label: Label

    init(value: Value, enabled: Bool, @ViewBuilder label: () -> Label) {
        self.value = value
        self.enabled = enabled
        self.label = label()
    }

    var body: some View {
        NavigationLink(value: value) {
            label
        }
        .disabled(!enabled)
    }
}
"#
);

/// Owns tab appearance behavior for both generated release views and dynamic
/// DevRuntime tabs while leaving SwiftUI's typed TabContent builders intact.
const NEXA_APP_BOTTOM_BAR_PRIMITIVE: &str = r#"struct NexaAppBottomBarPrimitive: ViewModifier {
    private let tint: Color?
    private let adaptiveStyle: Bool
    private let activatesSearchTab: Bool

    init(
        tint: Color?,
        adaptiveStyle: Bool,
        activatesSearchTab: Bool
    ) {
        self.tint = tint
        self.adaptiveStyle = adaptiveStyle
        self.activatesSearchTab = activatesSearchTab
    }

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 18.0, *), adaptiveStyle {
            if #available(iOS 26.0, *), activatesSearchTab {
                applyTint(content.tabViewStyle(.sidebarAdaptable)
                    .tabViewSearchActivation(.searchTabSelection))
            } else {
                applyTint(content.tabViewStyle(.sidebarAdaptable))
            }
        } else {
            applyTint(content)
        }
    }

    @ViewBuilder
    private func applyTint<Content: View>(_ content: Content) -> some View {
        if let tint {
            content.tint(tint)
        } else {
            content
        }
    }
}

struct NexaTabLabelPrimitive: View {
    let title: Text
    let systemImage: String

    var body: some View {
        Label { title } icon: {
            Image(systemName: systemImage).accessibilityHidden(true)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(title)
        .accessibilityIdentifier("")
    }
}

"#;

#[cfg(test)]
fn shared_component_primitives(line_height: bool) -> String {
    shared_component_primitives_with_animation(line_height, false)
}

fn shared_component_primitives_with_animation(
    line_height: bool,
    reduce_motion_animation: bool,
) -> String {
    let tokens = [
        (
            "__NEXA_BUTTON_LOADING_INDICATOR_SIZE__",
            nexa_codegen::design_system::BUTTON_LOADING_INDICATOR_SIZE.to_string(),
        ),
        (
            "__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_SMALL_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_SMALL_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_SMALL_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_LARGE_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_LARGE_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_DEFAULT_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_DEFAULT_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_MIN_WIDTH__",
            nexa_codegen::design_system::BUTTON_MIN_WIDTH.to_string(),
        ),
        (
            "__NEXA_BUTTON_MIN_TAP_TARGET__",
            nexa_codegen::design_system::BUTTON_MIN_TAP_TARGET.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_MIN_HEIGHT__",
            nexa_codegen::design_system::BUTTON_LARGE_MIN_HEIGHT.to_string(),
        ),
        (
            "__NEXA_DEFAULT_BODY_FONT_SIZE__",
            nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE.to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_HEIGHT__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .header_height
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_COLLAPSE_DISTANCE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .collapse_distance
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_EXPANDED_FONT_SIZE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .expanded_font_size
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_COLLAPSED_FONT_SIZE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .collapsed_font_size
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_HORIZONTAL_INSET__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .horizontal_inset
                .to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_ROW_VERTICAL_INSET.to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_ROW_HORIZONTAL_INSET.to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_CONTAINER_HORIZONTAL_INSET.to_string(),
        ),
        (
            "__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__",
            nexa_codegen::design_system::NATIVE_LIST_CONTENT_VERTICAL_INSET.to_string(),
        ),
        (
            "__NEXA_IOS_LEGACY_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__",
            nexa_codegen::design_system::IOS_LEGACY_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET
                .to_string(),
        ),
        (
            "__NEXA_SWIFT_DEFAULT_ACCENT_COLOR__",
            nexa_codegen::design_system::SWIFT_DEFAULT_ACCENT_COLOR.to_owned(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_MUTED_COLOR__",
            crate::generator::engine::colors::expression_from_argb(
                nexa_codegen::design_system::MUTED_TEXT_ARGB,
            ),
        ),
        (
            "__NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__",
            nexa_codegen::design_system::PAGE_INDICATOR_INACTIVE_OPACITY.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_SELECTED_SIZE__",
            nexa_codegen::design_system::PAGE_INDICATOR_SELECTED_SIZE.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__",
            nexa_codegen::design_system::PAGE_INDICATOR_UNSELECTED_SIZE.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_SPACING__",
            nexa_codegen::design_system::PAGE_INDICATOR_SPACING.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_BOTTOM_INSET__",
            nexa_codegen::design_system::PAGE_INDICATOR_BOTTOM_INSET.to_string(),
        ),
    ];
    let mut source = tokens.iter().fold(
        NEXA_SHARED_COMPONENT_PRIMITIVES.to_owned(),
        |source, (token, value)| source.replace(token, value),
    );
    let line_height_modifier = if line_height {
        r#"modifier(NexaTextLineHeightModifier(
                factor: style.lineHeightFactor,
                baseFontSize: style.lineHeightFontSize
            ))"#
    } else {
        "lineSpacing(0)"
    };
    source = source.replace("__NEXA_TEXT_LINE_HEIGHT_MODIFIER__", line_height_modifier);
    let reduce_motion_modifier = if reduce_motion_animation {
        r#"// nexa-codegen: NexaAnimationAccessibilityModifier
private struct NexaAnimationAccessibilityModifier: ViewModifier {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content.transaction { transaction in
            if reduceMotion {
                transaction.disablesAnimations = true
            }
        }
    }
}"#
    } else {
        ""
    };
    source = source.replace(
        "__NEXA_REDUCE_MOTION_ANIMATION_MODIFIER__",
        reduce_motion_modifier,
    );
    if line_height {
        source.push_str(
            r#"

// nexa-codegen: NexaTextLineHeightModifier
private struct NexaTextLineHeightModifier: ViewModifier {
    let factor: CGFloat?
    let baseFontSize: CGFloat

    @ViewBuilder func body(content: Content) -> some View {
        if let factor {
            if #available(iOS 26.0, *) {
                content.lineHeight(.multiple(factor: factor))
            } else {
                content.lineSpacing(
                    baseFontSize * factor - UIFont.systemFont(ofSize: baseFontSize).lineHeight
                )
            }
        } else {
            content.lineSpacing(0)
        }
    }
}
"#,
        );
    }
    source.push_str(include_str!("components/date_picker.swift"));
    source
}

const NEXA_REGEX_HELPERS: &str = r#"struct NexaRegexMatch {
    let value: String
    let range: NexaRegexRange
    let groups: [String?]
}

struct NexaRegexRange {
    let lowerBound: Int64
    let upperBound: Int64
}

struct NexaRegex {
    private let expression: NSRegularExpression?

    init(pattern: String) {
        expression = try? NSRegularExpression(pattern: pattern)
    }

    func matches(_ text: String) -> Bool {
        guard let expression else { return false }
        let range = NSRange(text.startIndex..., in: text)
        guard let match = expression.firstMatch(in: text, range: range) else { return false }
        return match.range.location == range.location && match.range.length == range.length
    }

    func find(_ text: String) -> NexaRegexMatch? {
        guard let expression else { return nil }
        let range = NSRange(text.startIndex..., in: text)
        guard let match = expression.firstMatch(in: text, range: range) else { return nil }
        return makeMatch(match, in: text)
    }

    func findAll(_ text: String) -> [NexaRegexMatch] {
        guard let expression else { return [] }
        let range = NSRange(text.startIndex..., in: text)
        return expression.matches(in: text, range: range).map { makeMatch($0, in: text) }
    }

    func replace(_ text: String, with replacement: String) -> String {
        guard let expression else { return text }
        let range = NSRange(text.startIndex..., in: text)
        return expression.stringByReplacingMatches(in: text, range: range, withTemplate: replacement)
    }

    private func makeMatch(_ match: NSTextCheckingResult, in text: String) -> NexaRegexMatch {
        let value = Range(match.range, in: text).map { String(text[$0]) } ?? ""
        let groups = (1..<match.numberOfRanges).map { index -> String? in
            let range = match.range(at: index)
            guard range.location != NSNotFound, let swiftRange = Range(range, in: text) else {
                return nil
            }
            return String(text[swiftRange])
        }
        return NexaRegexMatch(
            value: value,
            range: NexaRegexRange(
                lowerBound: Int64(match.range.location),
                upperBound: Int64(NSMaxRange(match.range))
            ),
            groups: groups
        )
    }
}

private func nexaRegexIsMatch(_ pattern: String, in text: String) -> Bool {
    guard let expression = try? NSRegularExpression(pattern: pattern) else { return false }
    let range = NSRange(text.startIndex..., in: text)
    return expression.firstMatch(in: text, range: range) != nil
}

private func nexaRegexMatches(_ pattern: String, in text: String) -> [String] {
    guard let expression = try? NSRegularExpression(pattern: pattern) else { return [] }
    let range = NSRange(text.startIndex..., in: text)
    return expression.matches(in: text, range: range).compactMap { match in
        Range(match.range, in: text).map { String(text[$0]) }
    }
}

private func nexaRegexReplace(_ pattern: String, in text: String, with replacement: String) -> String {
    guard let expression = try? NSRegularExpression(pattern: pattern) else { return text }
    let range = NSRange(text.startIndex..., in: text)
    return expression.stringByReplacingMatches(in: text, range: range, withTemplate: replacement)
}

"#;

pub(super) use api::{crypto, network, number, permissions, time};
use components::node_renderer as component_renderer;
pub(super) use components::{
    accessibility, bottom_bar, bottom_sheet, controls, custom_components, direction, images, input,
    keyboard, layout, lifecycle, links, list_runtime, lists, navigation, refresh, status_bar,
    system_icons,
};
pub(super) use engine::state::{
    render_immutable_state, render_native_object_state, render_native_object_state_uninitialized,
    render_state_initializers_in_init,
};
use engine::types::{self, swift_type};
pub(super) use engine::{colors, expressions, features, functions, imports, structs, utils, value};

/// Generates the release source as one concatenated string.
///
/// Prefer [`generate_units`]: the CLI writes each unit as its own file, and
/// this form exists for callers that only inspect the text.
pub(super) fn generate(module: &Module) -> String {
    join_units(generate_units(module).into_files(&[], ""))
}

/// Generates the release source as separate compile units.
///
/// Returns the pieces rather than finished files so the caller can merge extra
/// imports -- plugin bindings add imports that every file must see -- before
/// the files are assembled.
pub(super) fn generate_units(module: &Module) -> GeneratedSources {
    let features = features::Features::analyze(module);
    generate_with_analysis(module, features)
}

pub(super) fn generate_widget_units(
    module: &Module,
) -> Result<crate::WidgetGeneratedSources, crate::WidgetGenerationError> {
    widget::generate(module)
}

pub(super) fn generate_units_with_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    let features = features::Features::analyze(module);
    let project_features = project_features_from_analysis(&features);
    (generate_with_analysis(module, features), project_features)
}

fn project_features_from_analysis(features: &features::Features) -> crate::SwiftProjectFeatures {
    crate::SwiftProjectFeatures {
        requires_user_defaults_reason: features.facts.capabilities.uses_storage_api,
        // The feature-gated NexaNetwork helper contains its download operation,
        // which inspects the downloaded file's size with FileManager.
        requires_file_timestamp_reason: features.uses_network_api,
        uses_screen_orientation_api: features.facts.capabilities.uses_screen_orientation_api,
    }
}

/// Generates release units with the generic codec runtime needed by native
/// plugin contracts, including when the app only calls a non-generic method.
pub(super) fn generate_units_with_plugin_value_runtime(module: &Module) -> GeneratedSources {
    let features = features::Features::analyze(module);
    generate_with_analysis_mode(module, features, false, true)
}

pub(super) fn generate_units_with_plugin_value_runtime_and_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    let features = features::Features::analyze(module);
    let project_features = project_features_from_analysis(&features);
    (
        generate_with_analysis_mode(module, features, false, true),
        project_features,
    )
}

fn generate_with_analysis(module: &Module, features: features::Features) -> GeneratedSources {
    generate_with_analysis_mode(module, features, false, false)
}

fn generate_with_analysis_mode(
    module: &Module,
    features: features::Features,
    dev_runtime: bool,
    plugin_value_runtime: bool,
) -> GeneratedSources {
    let app_focus_bindings = app_focus_bindings(module, &features);
    let mut all_focus_bindings = app_focus_bindings.clone();
    for screen in &module.screens {
        all_focus_bindings.extend(screen_focus_bindings(screen, &features));
    }
    let uses_fast_list = features.uses_fast_list;
    // Each unit becomes its own file, so every file needs the full import
    // block and the shared native-object storage helper.
    let mut preamble = String::new();
    if module_has_native_object_state(module) {
        preamble.push_str(
            "@MainActor\nprivate final class NexaNativeObjectStorage<Value>: ObservableObject {\n    @Published var value: Value\n\n    init(makeValue: () -> Value) {\n        value = makeValue()\n    }\n}\n\n",
        );
    }
    if features.uses_drag || features.uses_pinch {
        preamble.push_str(DRAG_GESTURE_HELPERS);
    }
    if features.facts.ui.style.dynamic_color {
        preamble.push_str(&NEXA_DYNAMIC_COLOR_HELPER.replace(
            "__NEXA_SWIFT_DEFAULT_ACCENT_COLOR__",
            nexa_codegen::design_system::SWIFT_DEFAULT_ACCENT_COLOR,
        ));
    }
    if features.facts.ui.style.glass {
        preamble.push_str(NEXA_GLASS_HELPER);
    }
    if features.facts.capabilities.uses_locale_api {
        preamble.push_str(NEXA_LOCALE_HELPER);
    }
    preamble.push_str(NEXA_COLLECTION_HELPERS);
    let mut units = SourceUnits::new("swift");
    units.set_imports(&imports::render_with_dev_runtime(&features, dev_runtime));
    units.set_preamble(&preamble);
    // Value codecs live in the types file: app structs and enums are
    // file-private, and a codec for one has to construct and read it.
    let value_codecs = nexa_codegen::value::collect(module);
    let json_types = nexa_codegen::value::collect_json_types(module);
    units.write("types", |out| {
        if dev_runtime || features.facts.ui.bottom_bar.present {
            out.push_str(NEXA_APP_BOTTOM_BAR_PRIMITIVE);
        }
        // Regex declarations are shared by every generated unit. Keep them in
        // the single types file instead of the repeated per-file preamble.
        if features.facts.capabilities.uses_regex_api {
            out.push_str(NEXA_REGEX_HELPERS);
        }
        types::render_enums(module, out);
        structs::render(module, out);
        types::render_navigation_routes(module, out);
        if dev_runtime {
            value::render_for_dev(&value_codecs, out);
        } else if plugin_value_runtime {
            value::render_with_runtime(&value_codecs, out);
        } else {
            value::render(&value_codecs, out);
        }
    });

    if features.facts.ui.image.asset || features.uses_remote_image {
        units.write("image-primitives", |out| {
            components::images::render_primitives(
                features.facts.ui.image.asset,
                features.uses_remote_image,
                out,
            );
        });
    }

    if features.facts.capabilities.uses_json_api {
        units.write("json", |out| {
            api::json::render(&json_types, &module.enums, out);
        });
    }

    units.write("app", |out| {
        if features.uses_fast_list {
            out.push_str(
                "private struct NexaIdentifiedListRow: Identifiable {\n    let id: AnyHashable\n    let position: Int\n}\n\n",
            );
        }
        out.push_str(&format!(
            "public struct {}: View {{\n",
            nexa_codegen::names::screen_name(&module.app_name)
        ));
        if !module.screens.is_empty() {
            out.push_str("    private static let __nexaRootScreenIdentity = UUID()\n");
            out.push_str("    @State private var __nexaNavigationPath = NavigationPath()\n");
        }
        if features.uses_shared_elements {
            out.push_str("    @Namespace private var nexaSharedNamespace\n");
        }
        let has_signals = module
            .states
            .iter()
            .any(|state| matches!(state.ty, nexa_ir::Type::Signal(_)));
        for state in &module.states {
            if state.is_native_class_constructor_binding() {
                if has_signals {
                    render_native_object_state_uninitialized(state, 1, out);
                } else {
                    render_native_object_state(state, 1, out);
                }
                continue;
            }
            if matches!(state.ty, nexa_ir::Type::Signal(_)) {
                let name = nexa_codegen::names::state_name(&state.name);
                out.push_str(&format!(
                    "    @StateObject private var {name}: {}\n",
                    swift_type(&state.ty),
                ));
                continue;
            }
            if (!state.mutable && !state.is_native_class_instance_binding())
                || all_focus_bindings.contains(&state.name)
            {
                continue;
            }
            let name = nexa_codegen::names::state_name(&state.name);
            out.push_str(&format!(
                "    @State private var {name}: {} = {}\n",
                swift_type(&state.ty),
                expressions::expression(&state.initial)
            ));
        }
        render_immutable_state(&module.states, 1, out);
        for binding in &app_focus_bindings {
            out.push_str(&format!(
                "    @FocusState private var {}: Bool\n",
                nexa_codegen::names::state_name(binding),
            ));
        }
        if !module.states.is_empty() || !app_focus_bindings.is_empty() {
            out.push('\n');
        }
        if features.app_uses_adaptive_color {
            out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n");
        }
        if features.app_uses_size_class {
            out.push_str(
                "    @Environment(\\.horizontalSizeClass) private var nexaHorizontalSizeClass\n",
            );
            out.push_str(
                "    @Environment(\\.verticalSizeClass) private var nexaVerticalSizeClass\n",
            );
        }
        if module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some()
            || features.uses_widgets
        {
            out.push_str("    @Environment(\\.scenePhase) private var nexaScenePhase\n");
        }
        if features.uses_navigation_back {
            out.push_str("    @Environment(\\.dismiss) private var nexaDismiss\n");
        }
        if features.app_uses_adaptive_color
            || features.app_uses_size_class
            || features.uses_navigation_back
            || module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some()
            || features.uses_widgets
        {
            out.push('\n');
        }
        if has_signals {
            out.push_str("    public init() {\n");
            let mut writer = nexa_codegen::SourceWriter::new();
            render_state_initializers_in_init(&module.states, 2, &mut writer);
            out.push_str(&writer.finish());
            out.push_str("    }\n\n    public var body: some View {\n");
        } else {
            out.push_str("    public init() {}\n\n    public var body: some View {\n");
        }
        let task_handles = lifecycle::task_handles(&module.states);
        // A top-level conditional is already emitted as a concrete `Group` by
        // the node renderer, so body modifiers (including task cleanup) can be
        // applied to it directly without adding a redundant second wrapper.
        let needs_group = features.uses_shared_elements;

        if needs_group {
            out.push_str("        Group {\n");
            if module.body.len() == 1 {
                component_renderer::render_node(&module.body[0], module, &features, 3, out);
            } else {
                layout::render_layout(
                    LayoutKind::Column,
                    0.0,
                    &ViewStyle::default(),
                    &module.body,
                    &components::RenderScope {
                        module,
                        features: &features,
                    },
                    3,
                    out,
                );
            }
            out.push_str("\n        }");
        } else if module.body.len() == 1 {
            component_renderer::render_node(&module.body[0], module, &features, 2, out);
        } else {
            layout::render_layout(
                LayoutKind::Column,
                0.0,
                &ViewStyle::default(),
                &module.body,
                &components::RenderScope {
                    module,
                    features: &features,
                },
                2,
                out,
            );
        }
        direction::render(module.direction, 2, out);
        lifecycle::render_on_appear(module.on_appear.as_deref(), module.on_appear_async, 2, out);
        lifecycle::render_on_disappear(module.on_disappear.as_deref(), &task_handles, 2, out);
        lifecycle::render_scene_phase(module, 2, out);
        status_bar::render(module.status_bar, 2, out);
        if features.uses_shared_elements {
            out.push_str("\n        .environment(\\.nexaSharedNamespace, nexaSharedNamespace)");
        }
        if features.facts.uses_with_animation {
            out.push_str("\n        .modifier(NexaAnimationAccessibilityModifier())");
        }
        out.push_str("\n    }\n");
        components::bottom_bar::render_bottom_bar_helpers(&module.body, module, &features, out);
        if !module.screens.is_empty() {
            for screen in &module.screens {
                navigation::render_screen_view(screen, module, &features, out);
            }
        }
        out.push_str("}\n");
    });

    if features.uses_shared_elements {
        units.write("shared-elements", components::shared_elements::render);
    }

    units.write("components", |out| {
        custom_components::render(module, &features, out);
    });
    if uses_fast_list {
        units.write("list-runtime", |out| {
            list_runtime::render(
                out,
                features.uses_sticky_header,
                features.uses_scroll_events,
                features.uses_vertical_list,
                features.uses_horizontal_list,
                features.uses_grid_list,
                features.uses_sectioned_list,
            );
        });
    }
    if features.uses_native_library {
        units.write("native-library", |out| {
            network::render(
                out,
                features.uses_network_api,
                features.uses_remote_image,
                features.uses_path_api,
                features.uses_file_api,
                features.uses_file_async,
                features.uses_network_connectivity,
            );
        });
    }
    if features.uses_permissions {
        units.write("permissions", |out| {
            permissions::render(
                out,
                features.uses_permission_request,
                &features.used_permissions,
                features.dynamic_permission,
                features.expose_permissions_to_dev_runtime,
            );
        });
    }
    if features.facts.capabilities.uses_time {
        units.write("time", |out| {
            time::render(out);
        });
    }
    if features.facts.capabilities.uses_app_icon_api {
        units.write("app-icon", api::app_icon::render);
    }
    if features.facts.capabilities.uses_number_formatting {
        units.write("number", |out| {
            number::render(out);
        });
    }
    if features.facts.capabilities.uses_crypto_api {
        units.write("crypto", |out| {
            crypto::render(out);
        });
    }
    if features.facts.capabilities.uses_secure_storage_api {
        units.write("secure-storage", |out| {
            api::secure_storage::render(out);
        });
    }
    if features.facts.capabilities.uses_storage_api {
        units.write("storage", api::storage::render);
    }
    if features.facts.capabilities.uses_clipboard_api {
        units.write("clipboard", api::clipboard::render);
    }
    if features.facts.capabilities.uses_screen_orientation_api {
        units.write("screen-orientation", api::screen_orientation::render);
    }
    units.write("functions", |out| {
        functions::render(module, out);
    });
    let mut generated = units.finish();
    let references = generated
        .units
        .iter()
        .map(|unit| unit.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let primitive_source = shared_component_primitives_with_animation(
        dev_runtime || features.facts.ui.text.line_height,
        features.facts.uses_with_animation,
    );
    let primitives = if dev_runtime {
        nexa_codegen::source_sections::all(&primitive_source)
    } else {
        nexa_codegen::source_sections::reachable(&primitive_source, &references)
    };
    generated.prepend_to_unit("NexaGenerated_types.swift", &primitives);
    generated
}

pub(super) fn app_focus_bindings(
    module: &Module,
    features: &features::Features,
) -> std::collections::BTreeSet<String> {
    let mut bindings = features.facts.focus_bindings.app.clone();
    for screen in &module.screens {
        for binding in screen_focus_bindings(screen, features) {
            if module.states.iter().any(|state| state.name == binding) {
                bindings.insert(binding);
            }
        }
    }
    bindings
}

pub(super) fn screen_focus_bindings(
    screen: &nexa_ir::Screen,
    features: &features::Features,
) -> std::collections::BTreeSet<String> {
    let mut bindings = features
        .facts
        .focus_bindings
        .screens
        .get(&screen.name)
        .cloned()
        .unwrap_or_default();
    nexa_ir::walk::walk_ir(
        &screen.body,
        &mut |node| {
            if let Node::TextInput {
                focused: Some(name),
                ..
            } = node
            {
                bindings.insert(name.clone());
            }
        },
        &mut |_| {},
    );
    bindings
}

/// Concatenates units into a single source string.
fn join_units(units: Vec<SourceUnit>) -> String {
    let mut source = String::new();
    for unit in units {
        source.push_str(&unit.contents);
    }
    source
}

/// Generates the development host source as one concatenated string.
pub(super) fn generate_for_dev(module: &Module) -> String {
    join_units(generate_for_dev_units(module).into_files(&[], ""))
}

/// Generates the development host source as separate compile units.
pub(super) fn generate_for_dev_units(module: &Module) -> GeneratedSources {
    generate_for_dev_units_with_project_features(module).0
}

pub(super) fn generate_for_dev_units_with_project_features(
    module: &Module,
) -> (GeneratedSources, crate::SwiftProjectFeatures) {
    let mut features = features::Features::analyze(module);
    // Hot reload can introduce any FastList axis after the native host has
    // been built. Keep the same UIKit list primitives used by AOT apps ready
    // in the DevRuntime host.
    features.uses_fast_list = true;
    features.uses_vertical_list = true;
    features.uses_horizontal_list = true;
    features.uses_grid_list = true;
    features.uses_sectioned_list = true;
    features.uses_sticky_header = true;
    features.uses_scroll_events = true;
    // A shared element may appear after the development host is built, so
    // keep its namespace bridge available for hot-reloaded modules.
    features.uses_shared_elements = true;
    // Image nodes can be introduced by a hot reload after the development
    // host is built, so preload both local and network image primitives.
    features.facts.ui.image.asset = true;
    features.uses_remote_image = true;
    // Calls to Nexa's async native APIs can appear after the dev host has been
    // built. Keep the same URLSession adapter as release output in that host.
    features.uses_network_api = true;
    // File and Path calls can be introduced by a hot reload after the first
    // native build, so keep the same first-party helpers available in dev.
    features.uses_path_api = true;
    features.uses_file_api = true;
    features.uses_file_async = true;
    features.facts.capabilities.uses_crypto_api = true;
    // JSON is interpreted by NexaDevValueCodec using type descriptors from
    // every hot-reloaded module. Do not emit static JSON helpers for a module
    // that does not declare JsonError or concrete JSON value codecs yet.
    // SecureStorage calls can be added after the dev host is built, so the
    // Keychain adapter must already be part of every development host.
    features.facts.capabilities.uses_secure_storage_api = true;
    features.facts.capabilities.uses_storage_api = true;
    features.facts.capabilities.uses_regex_api = true;
    features.uses_permissions = true;
    features.uses_permission_request = true;
    features.dynamic_permission = true;
    features.used_permissions = [
        nexa_ir::Permission::Camera,
        nexa_ir::Permission::Microphone,
        nexa_ir::Permission::Photos,
        nexa_ir::Permission::Location,
        nexa_ir::Permission::Notifications,
        nexa_ir::Permission::Contacts,
        nexa_ir::Permission::Calendar,
        nexa_ir::Permission::Bluetooth,
        nexa_ir::Permission::Motion,
    ]
    .into_iter()
    .collect();
    features.expose_permissions_to_dev_runtime = true;
    features.uses_native_library = true;
    let project_features = project_features_from_analysis(&features);
    (
        generate_with_analysis_mode(module, features, true, false),
        project_features,
    )
}

fn module_has_native_object_state(module: &Module) -> bool {
    module
        .states
        .iter()
        .chain(
            module
                .screens
                .iter()
                .flat_map(|screen| screen.states.iter()),
        )
        .chain(
            module
                .components
                .iter()
                .flat_map(|component| component.states.iter()),
        )
        .any(State::is_native_class_constructor_binding)
}

#[cfg(test)]
mod tests {
    use super::{NEXA_SHARED_COMPONENT_PRIMITIVES, generate, shared_component_primitives};
    use nexa_ir::{
        Action, AnimationSpec, Component, Expr, FastListMove, Function, ImageScale, ImageSource,
        LayoutKind, ListAxis, ListCommon, ListPlan, Module, Node, NumericType, Screen, ScreenId,
        State, SystemIcon, TextStyle, Type, ViewStyle, ViewTransition, WhenCase,
    };

    #[test]
    fn segmented_control_reaches_the_unified_native_bridge() {
        let primitives = nexa_codegen::source_sections::reachable(
            NEXA_SHARED_COMPONENT_PRIMITIVES,
            "NexaSegmentedControlPrimitive()",
        );

        assert!(
            primitives.contains("private struct NexaNativeSegmentedControl: UIViewRepresentable")
        );
        assert!(primitives.contains("control.setTitleTextAttributes"));
        assert!(primitives.contains("control.layer.backgroundColor = color.cgColor"));
        assert!(primitives.contains("control.selectedSegmentTintColor = tint.map { UIColor($0) }"));
    }

    #[test]
    fn shared_button_primitive_embeds_design_system_geometry() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains("struct NexaButtonPrimitive<Label: View>"));
        assert!(primitives.contains("private struct NexaSharedProminentButtonStyle: ButtonStyle"));
        assert!(primitives.contains("NexaSharedProminentButtonStyle("));
        assert!(primitives.contains("private var plainLabelContent: some View"));
        assert!(
            primitives
                .contains(".foregroundStyle((disabled || loading) ? tint.opacity(0.38) : tint)")
        );
        assert!(primitives.contains("width: CGFloat(20),\n                height: CGFloat(20)"));
        assert!(primitives.contains("CGFloat(8)"));
        assert!(primitives.contains("struct NexaFormPrimitive<Content: View>"));
        assert!(primitives.contains(
            "@Environment(\\.multilineTextAlignment) private var inheritedTextAlignment"
        ));
        assert!(primitives.contains(".multilineTextAlignment(inheritedTextAlignment)"));
        assert!(primitives.contains(
            "struct NexaFormSectionPrimitive<Content: View, Header: View, Footer: View>"
        ));
        assert!(primitives.contains(".listSectionSeparator(.hidden)"));
        assert!(primitives.contains("CGFloat(64)"));
        assert!(primitives.contains("CGFloat(48)"));
        assert!(primitives.contains("CGFloat(50)"));
        assert!(primitives.contains("CGFloat(12)"));
        assert!(primitives.contains("CGFloat(20)"));
        assert!(
            !primitives.contains("__NEXA_"),
            "unresolved design token in helper source"
        );
    }

    #[test]
    fn date_picker_uses_native_controls_and_a_committable_range_sheet() {
        let primitives = shared_component_primitives(false);
        let date_picker = primitives
            .split("struct NexaDatePickerPrimitive: View {")
            .nth(1)
            .unwrap_or_default();

        assert!(date_picker.contains("DatePicker(\"\", selection: selectedDate"));
        assert!(date_picker.contains(".labelsHidden()"));
        assert!(
            date_picker.contains(
                "MultiDatePicker(\"\", selection: rangeDraftBinding, in: rangeDateRange)"
            )
        );
        assert!(
            date_picker
                .contains("MultiDatePicker(\"\", selection: selectedRange, in: rangeDateRange)")
        );
        assert!(date_picker.contains("private var rangeDateRange: Range<Date>"));
        assert!(
            date_picker
                .contains("pickerCalendar.date(byAdding: .day, value: 1, to: pickerCalendar.startOfDay(for: date(for: value)))")
        );
        assert!(date_picker.contains("let selectedDays = Set("));
        assert!(date_picker.contains("selectedDays.symmetricDifference(previousDays)"));
        assert!(date_picker.contains("selectedDays.contains(changed)"));
        assert!(date_picker.contains("@State private var rangeAnchor: Date?"));
        assert!(!date_picker.contains("selection.contains(changed)"));
        assert!(date_picker.contains(".sheet(isPresented: $isRangePresented)"));
        assert!(date_picker.contains("Button(\"Cancel\")"));
        assert!(date_picker.contains("Button(\"Done\")"));
        assert!(date_picker.contains(".accessibilityValue(rangeSummary)"));
        assert!(date_picker.contains(".disabled(rangeDraft.count < 2)"));
        assert!(date_picker.contains("private func commitRangeSelection()"));
        assert!(
            date_picker.contains("private func updateRangeDraft(_ selection: Set<DateComponents>)")
        );
        assert!(date_picker.contains(".presentationDetents([.medium, .large])"));
        assert!(date_picker.contains(".padding(.top, 24)"));
        assert!(!date_picker.contains("Text(rangeDraftSummary)"));
        assert!(date_picker.contains("Capsule()"));
        assert!(date_picker.contains(".padding(.horizontal, 16)"));
        assert!(
            date_picker.contains(".frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)")
        );
        assert!(date_picker.contains("Color.secondary.opacity(0.38)"));
        assert!(!date_picker.contains("LazyVGrid("));
        assert!(date_picker.contains(".datePickerStyle(.graphical)"));
        assert!(date_picker.contains(".datePickerStyle(.wheel)"));
        assert!(date_picker.contains("case \"Compact\", \"Input\":"));
        assert!(date_picker.contains(".datePickerStyle(.compact)"));
        assert!(date_picker.contains(".datePickerStyle(.automatic)"));
        assert!(!date_picker.contains(".popover("));
        assert!(!date_picker.contains(".accessibilityLabel(accessibilityLabel ?? \"\")"));
        assert!(!date_picker.contains("DateFormatter()"));
    }

    #[test]
    fn refresh_bounce_modifier_allows_short_content_to_trigger_pull_to_refresh() {
        let primitives = super::NEXA_SHARED_COMPONENT_PRIMITIVES;

        assert!(primitives.contains("private struct NexaRefreshBounceModifier: ViewModifier"));
        assert!(primitives.contains("if #available(iOS 16.4, *)"));
        assert!(primitives.contains(".scrollBounceBehavior(.always, axes: .vertical)"));
    }

    #[test]
    fn shared_text_input_fills_parent_and_keeps_text_leading_aligned() {
        let primitives = shared_component_primitives(false);
        let input = primitives
            .split("struct NexaTextInputPrimitive: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(input.contains(".frame(maxWidth: .infinity, alignment: .leading)"));
        assert!(input.contains(".multilineTextAlignment(.leading)"));
        assert!(input.contains("let onSubmitAction: (() -> Void)?"));
        assert!(input.contains("onSubmitAction?()"));
        assert!(input.contains("leadingField"));
    }

    #[test]
    fn status_bar_background_uses_the_active_window_safe_area() {
        let primitives = shared_component_primitives(false);
        let status_bar = primitives
            .split("struct NexaStatusBarBackgroundPrimitive: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(primitives.contains("private func nexaStatusBarSafeAreaHeight() -> CGFloat"));
        assert!(status_bar.contains(".frame(height: nexaStatusBarSafeAreaHeight())"));
        assert!(status_bar.contains(".ignoresSafeArea(edges: .top)"));
    }

    #[test]
    fn split_view_uses_visible_single_column_on_compact_ios() {
        let primitives = shared_component_primitives(false);
        let split_view = primitives
            .split("struct NexaNavigationSplitViewPrimitive<Sidebar: View, Detail: View>: View {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(
            split_view
                .contains("@Environment(\\.horizontalSizeClass) private var horizontalSizeClass")
        );
        assert!(split_view.contains("if horizontalSizeClass == .compact"));
        assert!(split_view.contains("if detailVisible {"));
        assert!(
            split_view
                .contains("detail\n                        .transition(.move(edge: .trailing))")
        );
        assert!(
            split_view
                .contains("sidebar\n                        .transition(.move(edge: .leading))")
        );
        assert!(split_view.contains(".animation(.default, value: detailVisible)"));
        assert!(split_view.contains("NavigationSplitView(preferredCompactColumn:"));
    }

    #[test]
    fn form_section_heading_uses_the_secondary_label_style() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains("header.foregroundStyle(.secondary)"));
    }

    #[test]
    fn text_alignment_inherits_column_default_when_not_explicit() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains("if let alignment = style.alignment"));
        assert!(primitives.contains("styledText.multilineTextAlignment(alignment)"));
        assert!(!primitives.contains("style.alignment ?? .leading"));
        assert!(primitives.contains(".multilineTextAlignment(textAlignment)"));
    }

    #[test]
    fn shared_picker_primitive_owns_all_native_picker_variants() {
        let primitives = shared_component_primitives(false);
        assert!(primitives.contains("struct NexaPickerPrimitive<LabelContent: View>"));
        assert!(primitives.contains("func nexaPickerTint(_ tint: Color?)"));
        assert!(primitives.contains(".nexaPickerTint(tint)"));
        assert!(primitives.contains("if hasLabel {"));
        assert!(primitives.contains("} else if let icon {"));
        let labeled_picker = primitives
            .split("} else if let icon {")
            .next()
            .unwrap_or_default();
        assert!(
            labeled_picker.contains("Menu {\n                pickerOptions(selection: $selection)")
        );
        assert!(!labeled_picker.contains(".pickerStyle(.inline)"));
        assert!(!labeled_picker.contains(".popover("));
        assert!(labeled_picker.contains(".labelsHidden()"));
        assert!(!labeled_picker.contains("labelsVisibility"));
        assert!(!labeled_picker.contains("labeledMenuOptions"));
        assert!(labeled_picker.contains("Image(systemName: itemIcons[index])\n                            .foregroundStyle(.secondary)"));
        assert!(labeled_picker.contains(".foregroundStyle(resolvedTint)"));
        assert!(labeled_picker.contains(".contentShape(Rectangle())"));
        assert!(labeled_picker.contains(".frame(maxWidth: .infinity, alignment: .leading)"));
        assert!(labeled_picker.contains("Text(selection)"));
        assert!(labeled_picker.contains("Spacer(minLength: 12)"));
        assert!(primitives.contains(".pickerStyle(.menu)"));
        assert!(primitives.contains(".accessibilityValue(selection)"));
    }

    #[test]
    fn shared_switch_uses_the_cross_platform_system_blue_accent() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains(".tint(Color(uiColor: .systemBlue))"));
    }

    #[test]
    fn shared_progress_ring_renders_the_determinate_value() {
        let primitives = shared_component_primitives(false);
        let ring = primitives
            .split("// nexa-codegen: NexaProgressRingPrimitive")
            .nth(1)
            .unwrap_or_default();

        assert!(ring.contains("private var normalizedProgress: Double?"));
        assert!(ring.contains("guard let progress else { return nil }"));
        assert!(ring.contains("guard progress.isFinite else { return 0 }"));
        assert!(ring.contains(".trim(from: 0, to: normalizedProgress)"));
        assert!(ring.contains("if normalizedProgress >= 1"));
        assert!(ring.contains("private var indicatorSize: CGFloat { max(size ?? 40, 0) }"));
        assert!(ring.contains(
            "private var indicatorThickness: CGFloat { min(max(thickness ?? 4, 0), indicatorSize) }"
        ));
        assert!(ring.contains(
            "private var pathSize: CGFloat { max(indicatorSize - indicatorThickness, 0) }"
        ));
        assert!(ring.contains(".trim(from: 0, to: 0.25)"));
        assert!(ring.contains(".frame(width: pathSize, height: pathSize)"));
        assert!(ring.contains(".frame(width: indicatorSize, height: indicatorSize)"));
        assert!(ring.contains("trackColor ?? indicatorColor.opacity(0.16)"));
        assert!(ring.contains("let tint: Color?"));
        assert!(ring.contains("let trackColor: Color?"));
        assert!(ring.contains(".accessibilityLabel(\"Progress\")"));
        assert!(ring.contains(".accessibilityValue(normalizedProgress.map"));
        assert!(!ring.contains(".progressViewStyle(.circular)"));
    }

    #[test]
    fn shared_progress_bar_uses_native_determinate_and_indeterminate_styles() {
        let primitives = shared_component_primitives(false);
        let bar = primitives
            .split("// nexa-codegen: NexaProgressBarPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaProgressRingPrimitive")
            .next()
            .unwrap_or_default();

        assert!(bar.contains("private var normalizedProgress: Double?"));
        assert!(bar.contains("guard let progress else { return nil }"));
        assert!(bar.contains("guard progress.isFinite else { return 0 }"));
        assert!(bar.contains("ProgressView(value: normalizedProgress, total: 1)"));
        assert!(bar.contains("private func indeterminateBar(tint: Color) -> some View"));
        assert!(bar.contains("let shouldAnimate = isVisible && !reduceMotion"));
        assert!(bar.contains(".accessibilityRepresentation { ProgressView() }"));
        assert!(bar.contains("if let normalizedProgress {"));
        assert!(bar.contains("if trackColor == nil && thickness == nil && cornerRadius == nil"));
        assert!(bar.contains("ProgressView(value: normalizedProgress, total: 1).tint(tint)"));
        assert!(bar.contains("trackColor ?? Color.primary.opacity(0.1)"));
        assert!(bar.contains("tint ?? Color.accentColor"));
    }

    #[test]
    fn shared_presentation_primitives_use_native_swiftui_sheets() {
        let primitives = shared_component_primitives(false);
        let bottom_sheet = primitives
            .split("// nexa-codegen: NexaBottomSheetPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaAlertDialogPrimitive")
            .next()
            .unwrap_or_default();
        assert!(bottom_sheet.contains("struct NexaBottomSheetPrimitive<Content: View>"));
        assert!(
            bottom_sheet
                .contains("NexaBottomSheetContentPrimitive(contentPadding: contentPadding) {")
        );
        assert!(bottom_sheet.contains("backgroundColor ?? Color(uiColor: .systemBackground)"));
        assert!(
            bottom_sheet.contains("private struct NexaBottomSheetContentPrimitive<Content: View>")
        );
        assert!(bottom_sheet.contains("NexaColumnPrimitive(\n            spacing: 0"));
        assert!(bottom_sheet.contains(".padding(contentPadding)"));
        assert!(!bottom_sheet.contains(".padding(.top"));
        assert!(
            bottom_sheet
                .contains(".frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)")
        );
        assert!(bottom_sheet.contains(".sheet(isPresented: $isPresented, onDismiss: {"));
        assert!(bottom_sheet.contains(
            "Color.clear\n            .frame(width: 0, height: 0)\n            .sheet(isPresented:"
        ));
        assert!(bottom_sheet.contains("onDismissRequest()"));
        assert!(bottom_sheet.contains("onDismiss()"));
        assert!(
            bottom_sheet.contains(
                "NexaBottomSheetBackgroundInteraction(\n                        allowsBackgroundInteraction: allowsBackgroundInteraction\n                    )"
            )
        );
        assert!(bottom_sheet.contains("content.presentationBackgroundInteraction(.enabled)"));
        assert!(bottom_sheet.contains("if #available(iOS 16.4, *), allowsBackgroundInteraction"));
        assert!(!bottom_sheet.contains("scrimColor == .clear"));
        assert!(bottom_sheet.contains("Set(snapPoints.map { .fraction($0) })"));
        assert!(
            bottom_sheet
                .contains(".presentationDetents(configuredDetents, selection: $selectedDetent)")
        );
        assert!(
            bottom_sheet
                .contains(".presentationDragIndicator(showDragIndicator ? .visible : .hidden)")
        );
        assert!(bottom_sheet.contains(".interactiveDismissDisabled(!allowsDismissal)"));
        assert!(bottom_sheet.contains("content.presentationBackground(color)"));
        assert!(bottom_sheet.contains("selectedDetent = configuredInitialDetent"));
        assert!(!bottom_sheet.contains("fullScreenCover"));
        assert!(!bottom_sheet.contains("keyboardWillShowNotification"));
        assert!(!bottom_sheet.contains("detentBeforeKeyboard"));

        assert!(primitives.contains("host.alert(title, isPresented: $isPresented)"));
        assert!(primitives.contains(".confirmationDialog(title, isPresented: $isPresented"));
    }

    #[test]
    fn shared_page_indicator_owns_dot_geometry_color_and_accessibility() {
        let primitives = shared_component_primitives(false);
        assert!(primitives.contains("struct NexaPageIndicatorPrimitive: View"));
        assert!(primitives.contains("Color(uiColor: .systemBlue)"));
        assert!(primitives.contains("width: selectedIndex == index ? 8 : 6"));
        assert!(primitives.contains(".accessibilityLabel(\"Page \\(index + 1)\")"));
        assert!(primitives.contains("struct NexaPageIndicatorRow: View"));
        assert!(primitives.contains("HStack(spacing: 8)"));
        assert!(primitives.contains(".padding(.bottom, 24)"));
    }

    #[test]
    fn divider_primitive_renders_the_requested_thickness() {
        let primitives = shared_component_primitives(false);

        assert!(primitives.contains("Rectangle()"));
        assert!(primitives.contains(".fill(color)"));
        assert!(primitives.contains(".frame(height: max(0, thickness))"));
    }

    #[test]
    fn text_line_height_uses_total_height_semantics_with_a_legacy_fallback() {
        let primitives = shared_component_primitives(true);

        assert!(primitives.contains("content.lineHeight(.multiple(factor: factor))"));
        assert!(primitives.contains("UIFont.systemFont(ofSize: baseFontSize).lineHeight"));
        assert!(!primitives.contains("__NEXA_TEXT_LINE_HEIGHT_MODIFIER__"));
    }

    fn regex_module(enabled: bool) -> Module {
        Module {
            widgets: Vec::new(),
            app_name: "RegexUsage".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: if enabled {
                vec![Function {
                    name: "hasDigits".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    return_type: Type::Bool,
                    body: Expr::NativeCall {
                        receiver: None,
                        namespace: "Regex".to_owned(),
                        name: "isMatch".to_owned(),
                        arguments: Vec::new(),
                        codecs: Vec::new(),
                        return_type: Type::Bool,
                        source_span: None,
                        is_async: false,
                        is_throwing: false,
                    },
                    body_actions: None,
                }]
            } else {
                Vec::new()
            },
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Regex capability probe".to_owned()),
                style: TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        }
    }

    #[test]
    fn regex_runtime_helpers_are_emitted_only_when_the_api_is_used() {
        let without_regex = generate(&regex_module(false));
        let with_regex = generate(&regex_module(true));
        let with_regex_units = super::generate_units(&regex_module(true));

        assert!(!without_regex.contains("nexaRegexIsMatch"));
        assert!(with_regex.contains("nexaRegexIsMatch"));
        assert_eq!(
            with_regex_units
                .units
                .iter()
                .map(|unit| unit.contents.matches("struct NexaRegex {").count())
                .sum::<usize>(),
            1,
            "shared Regex types must be emitted once across separate Swift files"
        );
    }

    #[test]
    fn task_cleanup_modifier_wraps_a_top_level_conditional_in_group() {
        let task_handle_type = Type::Optional(Box::new(Type::TaskHandle));
        let module = Module {
            widgets: Vec::new(),
            app_name: "ConditionalTaskCleanup".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "refreshTask".to_owned(),
                ty: task_handle_type.clone(),
                initial: Expr::Null(task_handle_type),
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::If {
                condition: Expr::Bool(true),
                then_body: vec![Node::Text {
                    value: Expr::String("Ready".to_owned()),
                    style: TextStyle::default(),
                }],
                else_body: None,
                transition: None,
                transition_animation: None,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(
            swift.contains("        Group {\n            if true {"),
            "the conditional body needs a concrete view type before shared modifiers are applied"
        );
        assert!(
            swift.contains(
                "\n        }.transition(.identity)\n            .onDisappear {\n                nexa_refreshTask?.cancel()"
            ),
            "task cleanup must apply to the full conditional view"
        );
    }

    #[test]
    fn pressable_context_menu_emits_native_swiftui_actions() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ContextMenuApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Row".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: vec![Node::Button {
                    label: Expr::LocalizedText {
                        key: "Edit".to_owned(),
                        value: Box::new(Expr::String("Edit".to_owned())),
                        comment: Some("Context menu action".to_owned()),
                    },
                    icon: Some(SystemIcon::Shared("edit".to_owned())),
                    loading: None,
                    disabled: None,
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    actions: Vec::new(),
                }],
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains(".contextMenu {"));
        assert!(swift.contains(
            "Label { Text(\"Edit\", comment: \"Context menu action\") } icon: { Image(systemName: \"pencil\") }"
        ));
    }

    #[test]
    fn appearance_wraps_content_in_native_swiftui_color_scheme() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "AppearanceApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Appearance {
                mode: Expr::String("dark".to_owned()),
                children: vec![Node::Text {
                    value: Expr::String("Hello".to_owned()),
                    style: TextStyle::default(),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let swift = generate(&module);
        assert!(swift.contains("NexaAppearancePrimitive(mode: \"dark\")"));
        assert!(swift.contains(".environment(\\.colorScheme, resolvedColorScheme)"));
        assert!(swift.contains(".preferredColorScheme(preferredColorScheme)"));
        assert!(swift.contains("private var preferredColorScheme: ColorScheme?"));
        assert!(swift.contains("Text(\"Hello\")"));
        // AOT output includes only primitives reachable from the app body.
        assert!(!swift.contains("struct NexaButtonPrimitive<Label: View>"));
        let dev_sources = super::generate_for_dev_units(&module);
        let swift_dev = dev_sources
            .units
            .iter()
            .map(|unit| unit.contents.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(swift_dev.contains("struct NexaButtonPrimitive<Label: View>"));
        assert!(!swift.contains("nexaColor(hex:"));
        assert!(!swift.contains("nexaLargeTitleDisplayMode()"));
    }

    #[test]
    fn semantic_text_font_roles_emit_scalable_swiftui_styles() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "SemanticTextStyle".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Task description".to_owned()),
                style: TextStyle {
                    font_style: Some(nexa_ir::TextFontStyle::Subheadline),
                    ..TextStyle::default()
                },
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        assert!(generate(&module).contains(".withFont(.subheadline)"));
    }

    #[test]
    fn dynamic_color_fallback_uses_the_shared_default_accent() {
        let helper = super::NEXA_DYNAMIC_COLOR_HELPER.replace(
            "__NEXA_SWIFT_DEFAULT_ACCENT_COLOR__",
            nexa_codegen::design_system::SWIFT_DEFAULT_ACCENT_COLOR,
        );

        assert!(!helper.contains("__NEXA_"));
        assert!(helper.contains("return Color(uiColor: .systemBlue)"));
    }

    #[test]
    fn page_snap_lists_use_viewport_rows_and_report_the_settled_page() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PageSnap".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "currentPage".to_owned(),
                ty: Type::Numeric(NumericType::Int32),
                initial: Expr::Number {
                    raw: "0".to_owned(),
                    ty: NumericType::Int32,
                },
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::FastList {
                plan: Box::new(ListPlan::Count {
                    count: Expr::Number {
                        raw: "3".to_owned(),
                        ty: NumericType::Int32,
                    },
                    common: ListCommon {
                        axis: ListAxis::Vertical,
                        show_dividers: true,
                        reverse_layout: false,
                        page_snap: true,
                        item_extent: None,
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: Some("currentPage".to_owned()),
                        scroll_to_top: None,
                        scroll_to_end: None,
                        scroll_to_offset: None,
                        children: vec![Node::Text {
                            value: Expr::String("Feed page".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_move: None,
                        swipe_actions: None,
                        sticky_header: None,
                        refresh: None,
                        spacing: 0.0,
                        scroll_enabled: None,
                    },
                }),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("pageSnap: true"));
        assert!(swift.contains("onScroll: nil,"));
        let page_snap_call = swift
            .split("NexaSwiftUIPageSnapList(")
            .nth(1)
            .expect("iOS 17 page-snapping list should be emitted");
        let scroll_enabled = page_snap_call
            .find("scrollEnabled: scrollEnabled")
            .expect("page-snapping list should forward scrollEnabled");
        let scroll_position_changed = page_snap_call
            .find("onScrollPositionChanged: onScrollPositionChanged")
            .expect("page-snapping list should forward its position callback");
        let refresh = page_snap_call
            .find("onRefresh: onRefresh")
            .expect("page-snapping list should forward its refresh callback");
        assert!(
            scroll_enabled < scroll_position_changed && scroll_position_changed < refresh,
            "the generated initializer call must follow the Swift parameter declaration order"
        );
        assert!(swift.contains("tableView.isPagingEnabled = pageSnap"));
        assert!(swift.contains("tableView.rowHeight = height"));
        assert!(swift.contains("height: pageSnap ? pageHeight : nil"));
        assert!(
            swift.contains("return pageHeight > 0 ? pageHeight : max(tableView.bounds.height, 1)")
        );
        assert!(swift.contains("func scrollViewDidEndDecelerating(_ scrollView: UIScrollView)"));
        assert!(swift.contains("guard !pageSnap else { return }"));
        assert!(!swift.contains("onScroll?()"));

        let mut reverse_module = module.clone();
        let Node::FastList { plan } = &mut reverse_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_mut() else {
            panic!("expected the count-backed FastList plan");
        };
        common.page_snap = false;
        common.reverse_layout = true;
        let reverse_swift = generate(&reverse_module);
        assert!(reverse_swift.contains("if pageSnap || reverseLayout {"));
        assert!(reverse_swift.contains("let row = indexPath.row"));
        assert!(
            !reverse_swift
                .contains("let row = reverseLayout ? rowCount - 1 - indexPath.row : indexPath.row")
        );
        assert!(
            reverse_swift.contains("coordinator.applyScrollPosition(to: tableView, force: true)")
        );
        assert!(
            reverse_swift.contains(
                "func applyScrollPosition(to tableView: UITableView, force: Bool = false)"
            )
        );
        assert!(reverse_swift.contains("func tableView(_ tableView: UITableView, estimatedHeightForRowAt indexPath: IndexPath) -> CGFloat"));
        assert!(reverse_swift.contains(
            "tableView.contentInsetAdjustmentBehavior = pageSnap || reverseLayout ? .never : .automatic"
        ));
    }

    #[test]
    fn swipe_action_lists_select_swiftui_list_without_the_custom_uikit_runtime() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "NativeList".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "moveEnabled".to_owned(),
                ty: Type::Bool,
                initial: Expr::Bool(true),
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::FastList {
                plan: Box::new(ListPlan::Items {
                    collection: Expr::State(
                        "tasks".to_owned(),
                        Type::Array(Box::new(Type::String)),
                    ),
                    element_type: Type::String,
                    item: "task".to_owned(),
                    common: ListCommon {
                        axis: ListAxis::Vertical,
                        show_dividers: true,
                        reverse_layout: false,
                        page_snap: false,
                        item_extent: None,
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: None,
                        scroll_to_top: None,
                        scroll_to_end: None,
                        scroll_to_offset: None,
                        children: vec![Node::Text {
                            value: Expr::String("Task".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_move: Some(FastListMove {
                            from: "from".to_owned(),
                            to: "to".to_owned(),
                            enabled: Expr::State("moveEnabled".to_owned(), Type::Bool),
                            actions: Vec::new(),
                        }),
                        swipe_actions: Some(vec![Node::Button {
                            label: Expr::String("Delete".to_owned()),
                            icon: None,
                            loading: None,
                            disabled: None,
                            style: None,
                            size: None,
                            shape: None,
                            tint: None,
                            glass: false,
                            actions: Vec::new(),
                        }]),
                        sticky_header: None,
                        refresh: None,
                        spacing: 0.0,
                        scroll_enabled: None,
                    },
                }),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("List {"));
        assert!(swift.contains(
            ".contentMargins(\n                .horizontal,\n                CGFloat(0),\n                for: .scrollContent\n            )"
        ));
        assert!(swift.contains(
            ".contentMargins(\n                .vertical,\n                CGFloat(16),\n                for: .scrollContent\n            )"
        ));
        assert!(swift.contains(
            "groupedList.padding(\n                .horizontal,\n                -CGFloat(16)\n            )"
        ));
        assert!(swift.contains("leading: CGFloat(16)"));
        assert!(swift.contains("trailing: CGFloat(16)"));
        assert!(!swift.contains("NexaFastList("));
        assert!(swift.contains(
            "private func isHorizontalPageContainer(_ scrollView: UIScrollView) -> Bool"
        ));
        assert!(
            swift.contains("collectionView.contentSize.width > collectionView.bounds.width + 1")
        );
        assert!(swift.contains(
            "NexaNativeListRowPrimitive(showDividers: true, prioritizeSwipeActions: true, reorderIndex: listPosition, reorderEnabled: nexa_moveEnabled, onReorder: { source, destination in"
        ));
        assert!(swift.contains("LongPressGesture(minimumDuration: 0.35)"));
        assert!(swift.contains(
            ".sequenced(before: DragGesture(minimumDistance: 0, coordinateSpace: .global))"
        ));
        assert!(swift.contains("NexaFastListRowFramesKey.self"));
        assert!(swift.contains("rowFrames.min(by:"));
        assert!(swift.contains("session.update(to: drag.location, perform: onReorder)"));
        assert!(swift.contains(".frame(maxWidth: .infinity, alignment: .leading)"));
        assert!(swift.contains(".zIndex(isDragging ? 1 : 0)"));
        assert!(!swift.contains(".onDrag {"));
        assert!(!swift.contains(".onDrop("));
        assert!(!swift.contains("NexaFastListReorderDropDelegate"));
        assert!(!swift.contains("UniformTypeIdentifiers"));
        assert!(
            swift.contains("NexaNativeListPrimitive(scrollEnabled: true, reorderEnabled: true)")
        );
        assert!(swift.contains("let nexa_from: Int32 = Int32(source)"));
        assert!(!swift.contains("isReorderable"));
        assert!(!swift.contains(".onMove {"));
        assert!(!swift.contains("\\.editMode"));
        // Swipe actions can still require UIKit-backed control styling; the
        // native List assertion above is the reliable renderer check.

        let mut disabled_module = module.clone();
        let Node::FastList { plan } = &mut disabled_module.body[0] else {
            panic!("expected the items-backed FastList plan");
        };
        let ListPlan::Items { common, .. } = plan.as_mut() else {
            panic!("expected the items-backed FastList plan");
        };
        common.scroll_enabled = Some(Expr::Bool(false));
        let disabled_swift = generate(&disabled_module);
        assert!(
            disabled_swift
                .contains("NexaNativeListPrimitive(scrollEnabled: false, reorderEnabled: true)")
        );
        assert!(disabled_swift.contains(".scrollDisabled(!scrollEnabled)"));
    }

    #[test]
    fn navigation_screens_fill_content_and_share_image_namespace() {
        let image = Node::Image {
            source: ImageSource::Asset("hero".to_owned()),
            description: "Hero".to_owned(),
            scale: ImageScale::Fit,
            placeholder: None,
            show_loading_indicator: false,
            max_height: None,
            fill_max_size: false,
            shared_element: Some(Expr::String("hero-image".to_owned())),
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "SharedHero".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: vec![
                Screen {
                    id: ScreenId(0),
                    name: "Home".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Appearance {
                        mode: Expr::String("dark".to_owned()),
                        children: vec![image],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Screen {
                    id: ScreenId(1),
                    name: "Details".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Text {
                        value: Expr::String("Detail screen".to_owned()),
                        style: TextStyle::default(),
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let app_body = swift
            .split("struct NexaScreen0: View")
            .next()
            .expect("navigation host is emitted before screen declarations");
        let appearance_position = app_body
            .find("NexaAppearancePrimitive(mode: \"dark\")")
            .expect("root appearance is hoisted to the navigation host");
        let navigation_position = app_body
            .find("NavigationStack(path: $__nexaNavigationPath)")
            .expect("navigation stack is generated");
        assert!(appearance_position < navigation_position);
        assert!(app_body.contains("NexaNavigationScreenPrimitive(title: \"Details\""));
        assert!(swift.contains("@Namespace private var nexaSharedNamespace"));
        assert!(
            swift.contains(".frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)")
        );
        assert!(!swift.contains(".modifier(NexaNavigationChromeModifier("));
        assert!(swift.contains("struct NexaNavigationScreenPrimitive<Content: View>: View"));
        assert!(swift.contains(".navigationTitle(inline && leadingTitle ? \"\" : title)"));
        assert!(swift.contains("NexaNavigationTitleDisplayModeModifier(inline: inline)"));
        assert!(swift.contains("content.toolbarTitleDisplayMode(inline ? .inline : .large)"));
        assert!(swift.contains("content.navigationBarTitleDisplayMode(inline ? .inline : .large)"));
        assert!(!swift.contains(".navigationBarBackButtonHidden(true)"));
        assert!(!swift.contains("NexaNavigationEdgeBackGesture"));
        assert!(swift.contains("ToolbarItem(placement: .topBarLeading)"));
        assert!(!swift.contains("Image(systemName: \"chevron.backward\")"));
        assert!(swift.contains(".accessibilityLabel(title)"));
        assert!(swift.contains(".frame(minWidth: 160, alignment: .leading)"));
        assert!(swift.contains(".offset(x: -13)"));
        assert!(swift.contains("Text(title)"));
        assert!(swift.contains("if inline && leadingTitle"));
        assert!(swift.contains("inline: true"));
        assert!(swift.contains("leadingTitle: true"));
        assert!(!swift.contains("NexaNavigationChromeProbe"));
        assert!(!swift.contains("NexaNavigationChromeState"));
        assert!(!swift.contains("NexaTitleBarModifier"));
        assert!(!swift.contains("NexaTitleBarScrollCoordinator"));
        assert!(!swift.contains("NexaTitleHeaderPrimitive"));
        assert!(swift.contains(".nexaSharedElement(id: \"hero-image\")"));
        assert!(swift.contains(".environment(\\.nexaSharedNamespace, nexaSharedNamespace)"));
        assert!(swift.contains("matchedGeometryEffect(id: id, in: namespace)"));

        let mut large_module = module.clone();
        let Node::NavigationStack { title_mode, .. } = &mut large_module.body[0] else {
            panic!("test module should have a navigation stack");
        };
        *title_mode = nexa_ir::NavigationTitleMode::Large;
        assert!(generate(&large_module).contains("inline: false"));

        let mut centered_module = module.clone();
        let Node::NavigationStack {
            title_alignment, ..
        } = &mut centered_module.body[0]
        else {
            panic!("test module should have a navigation stack");
        };
        *title_alignment = nexa_ir::NavigationTitleAlignment::Center;
        assert!(generate(&centered_module).contains("leadingTitle: false"));

        let mut automatic_without_toolbar = module.clone();
        let Node::NavigationStack {
            title_alignment, ..
        } = &mut automatic_without_toolbar.body[0]
        else {
            panic!("test module should have a navigation stack");
        };
        *title_alignment = nexa_ir::NavigationTitleAlignment::Automatic;
        assert!(generate(&automatic_without_toolbar).contains("leadingTitle: false"));

        automatic_without_toolbar.screens[0]
            .body
            .push(Node::Toolbar {
                placement: nexa_ir::ToolbarPlacement::Trailing,
                children: vec![Node::Text {
                    value: Expr::String("Save".to_owned()),
                    style: TextStyle::default(),
                }],
            });
        assert!(generate(&automatic_without_toolbar).contains("leadingTitle: false"));
    }

    #[test]
    fn configured_spring_uses_native_response_and_damping_values() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "SpringAnimation".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Layout {
                kind: LayoutKind::Column,
                spacing: 0.0,
                style: ViewStyle {
                    animation: Some(AnimationSpec::Spring {
                        response: 0.35,
                        damping: 0.8,
                    }),
                    ..ViewStyle::default()
                },
                children: vec![Node::Text {
                    value: Expr::String("Animated".to_owned()),
                    style: TextStyle::default(),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains(
            ".modifier(NexaAnimateContentSizeModifier(animation: .interpolatingSpring(mass: 1, stiffness: 816.32654, damping: 45.714287, initialVelocity: 0)))"
        ));
        let animated_size_modifier = swift
            .split("struct NexaAnimateContentSizeModifier: ViewModifier {")
            .nth(1)
            .expect("emit the layout animation modifier");
        assert!(animated_size_modifier.contains("transaction.animation = nil"));
        let frame = animated_size_modifier
            .find(".frame(width: contentSize?.width, height: contentSize?.height, alignment: .topLeading)")
            .expect("animate the measured frame");
        assert!(animated_size_modifier.contains("withAnimation(animation)"));
        assert!(animated_size_modifier.contains("if reduceMotion {"));
        assert!(animated_size_modifier.contains("self.contentSize = newSize"));
        assert!(!animated_size_modifier.contains(".animation("));
        assert!(
            animated_size_modifier
                .find("withAnimation(animation)")
                .expect("animate the measured size update")
                < frame
        );
        assert!(animated_size_modifier.contains("NexaContentSizePreferenceKey"));
        assert!(animated_size_modifier.contains(".clipped()"));
    }

    #[test]
    fn currency_formatting_emits_only_the_reachable_native_helper() {
        let call = Expr::NativeCall {
            receiver: None,
            namespace: "Number".to_owned(),
            name: "formatCurrency".to_owned(),
            arguments: vec![
                (
                    "amount".to_owned(),
                    Expr::Number {
                        raw: "1234.5".to_owned(),
                        ty: NumericType::Float64,
                    },
                ),
                ("currencyCode".to_owned(), Expr::String("EUR".to_owned())),
            ],
            codecs: Vec::new(),
            return_type: Type::String,
            source_span: None,
            is_async: false,
            is_throwing: false,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "CurrencyFormatting".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "price".to_owned(),
                ty: Type::String,
                initial: call,
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::State("price".to_owned(), Type::String),
                style: TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("nexaFormatCurrency(Double(1234.5), \"EUR\")"));
        assert!(swift.contains("func nexaFormatCurrency"));
        assert!(swift.contains("import Foundation\n"));
    }

    #[test]
    fn conditional_view_transitions_use_native_swiftui_modifiers() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ConditionalTransitions".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![
                Node::If {
                    condition: Expr::State("visible".to_owned(), Type::Bool),
                    then_body: vec![Node::Text {
                        value: Expr::String("shown".to_owned()),
                        style: TextStyle::default(),
                    }],
                    else_body: None,
                    transition: Some(ViewTransition::Fade),
                    transition_animation: Some(nexa_ir::AnimationSpec::Tween {
                        duration: 0.24,
                        delay: 0.04,
                        easing: nexa_ir::AnimationEasing::EaseOut,
                    }),
                },
                Node::When {
                    value: Expr::State("visible".to_owned(), Type::Bool),
                    cases: vec![WhenCase {
                        value: Expr::Bool(true),
                        body: vec![Node::Text {
                            value: Expr::String("yes".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    else_body: vec![Node::Text {
                        value: Expr::String("no".to_owned()),
                        style: TextStyle::default(),
                    }],
                    transition: Some(ViewTransition::SlideFromBottom),
                    transition_animation: Some(nexa_ir::AnimationSpec::Spring {
                        response: 0.35,
                        damping: 0.8,
                    }),
                },
                Node::If {
                    condition: Expr::State("visible".to_owned(), Type::Bool),
                    then_body: vec![Node::Text {
                        value: Expr::String("no default fade".to_owned()),
                        style: TextStyle::default(),
                    }],
                    else_body: None,
                    transition: None,
                    transition_animation: None,
                },
                Node::When {
                    value: Expr::State("visible".to_owned(), Type::Bool),
                    cases: vec![WhenCase {
                        value: Expr::Bool(true),
                        body: vec![Node::Text {
                            value: Expr::String("also no default fade".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    else_body: Vec::new(),
                    transition: None,
                    transition_animation: None,
                },
            ],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("Group {\n"));
        assert!(swift.contains(".transition(.opacity).animation(.timingCurve(0.0, 0.0, 0.2, 1.0, duration: 0.24).delay(0.04), value:"));
        assert!(swift.contains(".transition(.move(edge: .bottom)).animation(.interpolatingSpring(mass: 1, stiffness: 816.32654, damping: 45.714287, initialVelocity: 0), value:"));
        assert_eq!(swift.matches(".transition(.identity)").count(), 2);
    }

    #[test]
    fn double_tap_pressable_emits_exclusive_double_and_single_tap_gestures() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "DoubleTapApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: true,
                children: vec![Node::Text {
                    value: Expr::String("Tap twice".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: vec![Action::Assign {
                    name: "taps".to_owned(),
                    value: Expr::Number {
                        raw: "2".to_owned(),
                        ty: NumericType::Int32,
                    },
                }],
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("TapGesture(count: 2)"));
        assert!(swift.contains(
            ".frame(maxWidth: .infinity, maxHeight: .infinity).contentShape(Rectangle())"
        ));
        assert!(swift.contains(".exclusively(before:"));
        assert!(swift.contains("TapGesture(count: 1)"));
        assert!(swift.contains("nexa_taps = 2"));
        assert!(!swift.contains("onLongPressGesture"));
        assert!(!swift.contains("NexaDragGestureView"));

        let mut long_press_module = module;
        let Node::Pressable {
            actions,
            double_tap_actions,
            long_press_actions,
            ..
        } = &mut long_press_module.body[0]
        else {
            panic!("expected Pressable");
        };
        *actions = vec![Action::Assign {
            name: "taps".to_owned(),
            value: Expr::Number {
                raw: "1".to_owned(),
                ty: NumericType::Int32,
            },
        }];
        double_tap_actions.clear();
        *long_press_actions = vec![Action::Assign {
            name: "longPresses".to_owned(),
            value: Expr::Number {
                raw: "1".to_owned(),
                ty: NumericType::Int32,
            },
        }];

        let swift = generate(&long_press_module);

        assert!(swift.contains("Button(action: {}) {"));
        assert!(swift.contains("LongPressGesture(minimumDuration:"));
        assert!(swift.contains(".exclusively(before:"));
        assert!(swift.contains("TapGesture(count: 1)"));
        assert!(swift.contains("nexa_longPresses = 1"));
        assert!(swift.contains("nexa_taps = 1"));
        assert!(!swift.contains(".onLongPressGesture("));
        assert!(
            swift.find("LongPressGesture(").unwrap() < swift.find("TapGesture(count: 1)").unwrap()
        );

        let mut combined_module = long_press_module.clone();
        let Node::Pressable {
            double_tap_actions, ..
        } = &mut combined_module.body[0]
        else {
            panic!("expected Pressable");
        };
        *double_tap_actions = vec![Action::Assign {
            name: "doubleTaps".to_owned(),
            value: Expr::Number {
                raw: "2".to_owned(),
                ty: NumericType::Int32,
            },
        }];

        let swift = generate(&combined_module);
        let long_press_index = swift.find("LongPressGesture(").unwrap();
        let double_tap_index = swift.find("TapGesture(count: 2)").unwrap();
        let single_tap_index = swift.find("TapGesture(count: 1)").unwrap();
        assert!(long_press_index < double_tap_index && double_tap_index < single_tap_index);
        assert!(swift.contains("nexa_doubleTaps = 2"));
    }

    #[test]
    fn drag_pressable_emits_native_gesture_and_typed_callback() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "DragApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "distance".to_owned(),
                ty: Type::Numeric(NumericType::Float64),
                initial: Expr::Number {
                    raw: "0.0".to_owned(),
                    ty: NumericType::Float64,
                },
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Drag me".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: vec![
                    "translationX".to_owned(),
                    "translationY".to_owned(),
                    "velocityX".to_owned(),
                    "velocityY".to_owned(),
                ],
                drag_actions: vec![Action::Assign {
                    name: "distance".to_owned(),
                    value: Expr::State("velocityX".to_owned(), Type::Numeric(NumericType::Float64)),
                }],
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("private struct NexaDragGestureView<Content: View>"));
        assert!(swift.contains("content.highPriorityGesture(dragGesture)"));
        assert!(swift.contains(
            "onDrag: { nexa_translationX, nexa_translationY, nexa_velocityX, nexa_velocityY in"
        ));
        assert!(swift.contains("trackVelocity: true"));
        assert!(swift.contains("nexa_distance = nexa_velocityX"));

        let mut translation_only_module = module;
        let Node::Pressable { drag_actions, .. } = &mut translation_only_module.body[0] else {
            panic!("expected Pressable");
        };
        drag_actions[0] = Action::Assign {
            name: "distance".to_owned(),
            value: Expr::State(
                "translationX".to_owned(),
                Type::Numeric(NumericType::Float64),
            ),
        };
        let swift = generate(&translation_only_module);
        assert!(swift.contains("trackVelocity: false"));
    }

    #[test]
    fn pinch_pressable_emits_native_scale_delta_callback() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PinchApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "zoom".to_owned(),
                ty: Type::Numeric(NumericType::Float64),
                initial: Expr::Number {
                    raw: "1.0".to_owned(),
                    ty: NumericType::Float64,
                },
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Pinch to zoom".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: Some("scaleFactor".to_owned()),
                pinch_actions: vec![Action::Assign {
                    name: "zoom".to_owned(),
                    value: Expr::State(
                        "scaleFactor".to_owned(),
                        Type::Numeric(NumericType::Float64),
                    ),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);

        assert!(swift.contains("MagnificationGesture()"));
        assert!(swift.contains("NexaMagnificationTracker"));
        assert!(swift.contains("onPinch: { nexa_scaleFactor in"));
        assert!(swift.contains("nexa_zoom = nexa_scaleFactor"));
        assert!(!swift.contains("onDrag: {"));
    }

    fn native_instance_state(name: &str) -> State {
        let ty = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        State {
            name: name.to_owned(),
            ty: ty.clone(),
            initial: Expr::Call {
                name: "Video.VideoPlayer".to_owned(),
                arguments: Vec::new(),
                return_type: ty,
                is_async: false,
                is_throwing: false,
                is_constructor: true,
            },
            mutable: false,
        }
    }

    #[test]
    fn app_native_class_instances_are_not_recreated_in_the_body() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![native_instance_state("player")],
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "@StateObject private var __nexaNativeObjectStorage_{state_name} = NexaNativeObjectStorage {{ VideoPlayer() }}"
        )));
        assert!(swift.contains(&format!(
            "private var {state_name}: VideoPlayer {{\n        get {{ __nexaNativeObjectStorage_{state_name}.value }}"
        )));
        assert_eq!(
            swift.matches("VideoPlayer()").count(),
            1,
            "the constructor belongs only in the persistent @State initializer"
        );
    }

    #[test]
    fn immutable_app_values_are_view_properties_for_generated_helpers() {
        let mut module = regex_module(false);
        module.states.push(State {
            name: "screenTitle".to_owned(),
            ty: Type::String,
            initial: Expr::String("Reading list".to_owned()),
            mutable: false,
        });
        module.body = vec![Node::Text {
            value: Expr::State("screenTitle".to_owned(), Type::String),
            style: TextStyle::default(),
        }];

        let swift = generate(&module);
        let property = "private var nexa_screenTitle: String { \"Reading list\" }";
        let body = swift.find("public var body: some View").unwrap();
        let property = swift.find(property).unwrap();

        assert!(
            property < body,
            "immutable app values must be view properties"
        );
        assert!(swift.contains("Text(nexa_screenTitle)"));
    }

    #[test]
    fn mutable_native_class_bindings_write_through_identity_storage() {
        let mut player = native_instance_state("player");
        player.mutable = true;
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![player],
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "nonmutating set {{ __nexaNativeObjectStorage_{state_name}.value = newValue }}"
        )));
    }

    #[test]
    fn screen_native_class_instances_use_swiftui_identity_storage() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![
                native_instance_state("appPlayer"),
                State {
                    name: "sharedCount".to_owned(),
                    ty: Type::Numeric(NumericType::Int32),
                    initial: Expr::Number {
                        raw: "7".to_owned(),
                        ty: NumericType::Int32,
                    },
                    mutable: true,
                },
            ],
            screens: vec![
                Screen {
                    id: ScreenId(0),
                    name: "PlayerScreen".to_owned(),
                    parameters: Vec::new(),
                    states: vec![
                        State {
                            name: "player".to_owned(),
                            ty: player_type.clone(),
                            initial: Expr::Call {
                                name: "Video.VideoPlayer".to_owned(),
                                arguments: Vec::new(),
                                return_type: player_type.clone(),
                                is_async: false,
                                is_throwing: false,
                                is_constructor: true,
                            },
                            mutable: false,
                        },
                        State {
                            name: "screenCount".to_owned(),
                            ty: Type::Numeric(NumericType::Int32),
                            initial: Expr::Number {
                                raw: "1".to_owned(),
                                ty: NumericType::Int32,
                            },
                            mutable: true,
                        },
                    ],
                    body: vec![Node::NavigationLink {
                        destination: ScreenId(1),
                        arguments: Vec::new(),
                        guard: None,
                        children: vec![Node::Text {
                            value: Expr::String("Push another instance".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Screen {
                    id: ScreenId(1),
                    name: "Details".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Button {
                        label: Expr::String("Play shared player".to_owned()),
                        icon: None,
                        loading: None,
                        disabled: None,
                        style: None,
                        size: None,
                        shape: None,
                        tint: None,
                        glass: false,
                        actions: vec![Action::Expression(Expr::NativeCall {
                            receiver: Some(Box::new(Expr::State(
                                "appPlayer".to_owned(),
                                player_type.clone(),
                            ))),
                            namespace: "Video".to_owned(),
                            name: "play".to_owned(),
                            arguments: Vec::new(),
                            codecs: Vec::new(),
                            return_type: Type::Void,
                            source_span: None,
                            is_async: false,
                            is_throwing: false,
                        })],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        // Top-level `private` is lowered because each unit compiles as its own
        // file, so the screen declaration is internal here.
        let screen_declaration = swift
            .find("struct NexaScreen0: View")
            .expect("screen state must be owned by a destination view");
        let app_source = &swift[..screen_declaration];
        let screen_source = &swift[screen_declaration..];
        assert!(
            screen_source.contains(&format!(
                "@StateObject private var __nexaNativeObjectStorage_{state_name}"
            )),
            "screen-scoped native objects must be initialized once in route-owned SwiftUI storage"
        );
        assert!(!app_source.contains("__nexaNativeObjectStorage_nexa_player"));
        assert!(
            app_source
                .contains("@StateObject private var __nexaNativeObjectStorage_nexa_appPlayer")
        );
        assert!(screen_source.contains("private let nexa_appPlayer: VideoPlayer"));
        assert!(!screen_source.contains("@Binding private var nexa_appPlayer"));
        assert_eq!(screen_source.matches("VideoPlayer()").count(), 1);
        assert!(app_source.contains("@State private var nexa_sharedCount: Int32 = 7"));
        assert!(screen_source.contains("@Binding private var nexa_sharedCount: Int32"));
        assert!(screen_source.contains("@State private var nexa_screenCount: Int32 = 1"));
        assert!(swift.contains("case screen0(UUID)"));
        assert!(swift.contains("case screen1(UUID)"));
        assert!(swift.contains("NexaNavigationRoute.screen1(UUID())"));
        assert!(swift.contains("case let .screen0(routeIdentity):"));
        assert!(swift.contains("case let .screen1(routeIdentity):"));
        assert!(swift.contains("nexa_appPlayer.play()"));
        assert!(swift.contains(".id(routeIdentity)"));
    }

    #[test]
    fn component_native_class_instances_use_swiftui_identity_storage() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: vec![Component {
                name: "PlayerPanel".to_owned(),
                source_file: None,
                parameters: Vec::new(),
                states: vec![native_instance_state("player")],
                body: Vec::new(),
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            body: vec![Node::ComponentCall {
                name: "PlayerPanel".to_owned(),
                arguments: Vec::new(),
                children: None,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(swift.contains(&format!(
            "@StateObject private var __nexaNativeObjectStorage_{state_name} = NexaNativeObjectStorage {{ VideoPlayer() }}"
        )));
    }

    #[test]
    fn generates_result_and_try_in_swift() {
        let err_type = Type::Enum("AppError".to_owned());
        let module = Module {
            widgets: Vec::new(),
            app_name: "ResultApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            background_tasks: Vec::new(),
            enums: vec![nexa_ir::EnumDecl {
                name: "AppError".to_owned(),
                cases: vec!["NotFound".to_owned(), "Unauthorized".to_owned()],
            }],
            structs: Vec::new(),
            functions: vec![
                Function {
                    name: "fetchCode".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    body_actions: None,
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::ResultOk {
                        value: Box::new(Expr::Number {
                            raw: "42".to_owned(),
                            ty: NumericType::Int32,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
                Function {
                    name: "compute".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    body_actions: None,
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::Try {
                        expr: Box::new(Expr::Call {
                            name: "fetchCode".to_owned(),
                            arguments: Vec::new(),
                            return_type: Type::Result(
                                Box::new(Type::Numeric(NumericType::Int32)),
                                Box::new(err_type.clone()),
                            ),
                            is_async: false,
                            is_throwing: false,
                            is_constructor: false,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
            ],
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("enum NexaAppError: Int, Error {"));
        assert!(swift.contains("Result<Int32, NexaAppError>"));
        assert!(swift.contains(".success(42)"));
        assert!(swift.contains("try nexa_fn_fetchCode().get()"));
    }

    #[test]
    fn app_user_class_instances_use_native_object_storage() {
        let class_type = Type::Class {
            name: "NoteStore".to_owned(),
            fields: vec![(
                "notes".to_owned(),
                Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
            )],
            constructor_parameter_count: 0,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "Demo".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![
                State {
                    name: "store".to_owned(),
                    ty: class_type.clone(),
                    initial: Expr::Call {
                        name: "NoteStore".to_owned(),
                        arguments: Vec::new(),
                        return_type: class_type.clone(),
                        is_async: false,
                        is_throwing: false,
                        is_constructor: true,
                    },
                    mutable: false,
                },
                State {
                    name: "notes".to_owned(),
                    ty: Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
                    initial: Expr::Member {
                        base: Box::new(Expr::State("store".to_owned(), class_type.clone())),
                        name: "notes".to_owned(),
                        optional: false,
                        base_type: Type::Class {
                            name: "NoteStore".to_owned(),
                            fields: Vec::new(),
                            constructor_parameter_count: 0,
                        },
                        field_type: Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
                        kind: nexa_ir::MemberKind::ClassField("notes".to_owned()),
                    },
                    mutable: false,
                },
            ],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Hello".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let swift = generate(&module);
        assert!(swift.contains("__nexaNativeObjectStorage_nexa_store"));
        assert!(swift.contains("let nexa_store = __storage_nexa_store.value"));
        assert!(swift.contains("_nexa_notes = StateObject(wrappedValue: nexa_store.nexa_notes)"));
    }
}
