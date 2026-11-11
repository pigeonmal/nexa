// nexa-codegen: NexaTextInputPrimitive
@MainActor
struct NexaTextInputPrimitive: View {
    @Binding var text: String
    let placeholder: String
    let secure: Bool
    let multiline: Bool
    let searchable: Bool
    let maxLength: Int?
    let fontSize: CGFloat?
    let fontFamily: String?
    let fontRelativeTo: Font.TextStyle
    let fontDefaultSize: CGFloat
    let textColor: Color?
    let placeholderColor: Color?
    let clearable: Bool
    var minLines: Int? = nil
    var maxLines: Int? = nil
    var keyboard: UIKeyboardType = .default
    var returnKey: UIReturnKeyType = .default
    var autofill: UITextContentType? = nil
    var autocorrect: Bool? = nil
    var capitalization: UITextAutocapitalizationType = .sentences
    var initialFocus: Bool = false
    var componentRef: NexaTextInputRef? = nil
    var onFocusAction: (() -> Void)? = nil
    let onSubmitAction: (() -> Void)?
    let onChange: ((String) -> Void)?
    @Environment(\.sizeCategory) private var sizeCategory
    @Environment(\.colorScheme) private var colorScheme

    var nativeFont: UIFont {
        let style: UIFont.TextStyle = fontRelativeTo == .title3 ? .title3 : .body
        if let fontFamily {
            let font = UIFont(name: fontFamily, size: fontSize ?? fontDefaultSize)
                ?? UIFont.systemFont(ofSize: fontSize ?? fontDefaultSize)
            return UIFontMetrics(forTextStyle: style).scaledFont(for: font)
        }
        if let fontSize {
            return UIFontMetrics(forTextStyle: style).scaledFont(for: .systemFont(ofSize: fontSize))
        }
        return UIFont.preferredFont(forTextStyle: style)
    }

    func nativeFocusChanged(_ focused: Bool) {
        if focused { onFocusAction?() }
    }

    private var editor: some View {
        NexaNativeTextInput(owner: self, value: text, initialFocus: initialFocus)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    var body: some View {
        inputContent
    }

    @ViewBuilder
    private var inputContent: some View {
        let _ = sizeCategory
        let _ = colorScheme
        if searchable || clearable {
            HStack(spacing: 8) {
                if searchable {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(Color(red: 142.0 / 255, green: 142.0 / 255, blue: 147.0 / 255))
                        .font(.system(size: 20))
                        .frame(width: 20, height: 20)
                        .accessibilityHidden(true)
                }
                editor
                if clearable {
                    ZStack {
                        if !text.isEmpty {
                            Button {
                                text = ""
                                onChange?("")
                            } label: {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundStyle(Color(red: 142.0 / 255, green: 142.0 / 255, blue: 147.0 / 255))
                                    .font(.system(size: 18))
                                    .frame(width: 48, height: 48)
                                    .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel(Text("Clear text"))
                        }
                    }
                    .frame(width: 48, height: 48)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            editor
        }
    }
}
