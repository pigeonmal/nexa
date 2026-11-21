// nexa-codegen: NexaNativeTextInput
@MainActor
struct NexaNativeTextInput: UIViewRepresentable {
    var owner: NexaTextInputPrimitive
    var value: String
    var initialFocus: Bool
    var focusID: UUID
    var focusCoordinator: NexaKeyboardFocusCoordinator?

    func makeCoordinator() -> Coordinator {
        Coordinator(owner: owner, focusID: focusID, focusCoordinator: focusCoordinator)
    }

    func makeUIView(context: Context) -> NexaTextInputHost {
        let host = NexaTextInputHost()
        host.configure(owner: owner, coordinator: context.coordinator, initialFocus: initialFocus, text: value)
        return host
    }

    func updateUIView(_ host: NexaTextInputHost, context: Context) {
        context.coordinator.owner = owner
        context.coordinator.focusID = focusID
        context.coordinator.focusCoordinator = focusCoordinator
        host.configure(owner: owner, coordinator: context.coordinator, initialFocus: initialFocus, text: value)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: NexaTextInputHost, context: Context) -> CGSize? {
        uiView.measuredSize(width: proposal.width)
    }

    static func dismantleUIView(_ host: NexaTextInputHost, coordinator: Coordinator) {
        coordinator.focusCoordinator?.inputBlurred(coordinator.focusID)
        host.detachComponentRef()
        host.textField?.delegate = nil
        host.textView?.delegate = nil
        host.textField?.removeTarget(coordinator, action: #selector(Coordinator.fieldChanged(_:)), for: .editingChanged)
        host.textField?.resignFirstResponder()
        host.textView?.resignFirstResponder()
    }

    @MainActor
    final class Coordinator: NSObject, UITextFieldDelegate, UITextViewDelegate {
        var owner: NexaTextInputPrimitive
        var isApplyingModel = false
        var focusID: UUID
        var focusCoordinator: NexaKeyboardFocusCoordinator?

        init(
            owner: NexaTextInputPrimitive,
            focusID: UUID,
            focusCoordinator: NexaKeyboardFocusCoordinator?
        ) {
            self.owner = owner
            self.focusID = focusID
            self.focusCoordinator = focusCoordinator
        }

        private func accepted(_ previous: String, _ proposed: String) -> String {
            nexaTextInputEdit(previous: previous, proposed: proposed, maxLength: owner.maxLength, maxLines: owner.maxLines, multiline: owner.multiline)
        }

        private func publish(_ value: String) {
            guard !isApplyingModel && owner.text != value else { return }
            owner.text = value
            owner.onChange?(value)
        }

        private func edit(_ input: UIView & UITextInput, previous: String, range: NSRange, replacement: String) -> Bool {
            let previousNSString = previous as NSString
            let previousLength = previousNSString.length
            guard range.location <= previousLength && range.length <= previousLength - range.location else { return false }
            if input.markedTextRange != nil { return true }
            if owner.maxLength == nil && (!owner.multiline || owner.maxLines == nil) {
                let containsLineBreak = !owner.multiline && replacement.unicodeScalars.contains {
                    $0 == "\n" || $0 == "\r" || $0 == "\u{2028}" || $0 == "\u{2029}"
                }
                if !containsLineBreak { return true }
            }
            let proposal = previousNSString.replacingCharacters(in: range, with: replacement)
            let value = accepted(previous, proposal)
            guard value != proposal else { return true }
            if value == previous { return false }
            if let field = input as? UITextField { field.text = value }
            if let view = input as? UITextView { view.text = value; view.superview?.invalidateIntrinsicContentSize() }
            let suffixLength = previous.utf16.count - range.location - range.length
            let caret = max(0, min(value.utf16.count, value.utf16.count - suffixLength))
            if let position = input.position(from: input.beginningOfDocument, offset: caret) {
                input.selectedTextRange = input.textRange(from: position, to: position)
            }
            publish(value)
            return false
        }

        func textField(_ textField: UITextField, shouldChangeCharactersIn range: NSRange, replacementString string: String) -> Bool {
            edit(textField, previous: textField.text ?? "", range: range, replacement: string)
        }

        func textView(_ textView: UITextView, shouldChangeTextIn range: NSRange, replacementText text: String) -> Bool {
            if text == "\n", textView.markedTextRange == nil, owner.returnKey != .default {
                owner.onSubmitAction?()
                if owner.returnKey != .next { textView.resignFirstResponder() }
                return false
            }
            return edit(textView, previous: textView.text, range: range, replacement: text)
        }

        @objc func fieldChanged(_ field: UITextField) {
            guard !isApplyingModel && field.markedTextRange == nil else { return }
            let proposed = field.text ?? ""
            if owner.maxLength == nil && (!owner.multiline || owner.maxLines == nil) {
                publish(proposed)
                return
            }
            let value = accepted(owner.text, proposed)
            if field.text != value { field.text = value }
            publish(value)
        }

        func textViewDidChange(_ textView: UITextView) {
            let host = textView.superview as? NexaTextInputHost
            host?.refreshPlaceholder()
            host?.invalidateIntrinsicContentSize()
            guard !isApplyingModel && textView.markedTextRange == nil else { return }
            let value = accepted(owner.text, textView.text)
            if textView.text != value { textView.text = value }
            (textView.superview as? NexaTextInputHost)?.refreshPlaceholder()
            textView.superview?.invalidateIntrinsicContentSize()
            publish(value)
        }

        func textFieldDidBeginEditing(_ textField: UITextField) {
            focusCoordinator?.inputFocused(focusID)
            owner.nativeFocusChanged(true)
        }

        func textFieldDidEndEditing(_ textField: UITextField) {
            focusCoordinator?.inputBlurred(focusID)
            owner.nativeFocusChanged(false)
        }

        func textViewDidBeginEditing(_ textView: UITextView) {
            focusCoordinator?.inputFocused(focusID)
            owner.nativeFocusChanged(true)
        }

        func textViewDidEndEditing(_ textView: UITextView) {
            focusCoordinator?.inputBlurred(focusID)
            owner.nativeFocusChanged(false)
        }

        func textFieldShouldReturn(_ textField: UITextField) -> Bool {
            owner.onSubmitAction?()
            if owner.returnKey != .next { textField.resignFirstResponder() }
            return false
        }
    }
}

@MainActor
final class NexaTextInputHost: UIView {
    private(set) var textField: UITextField?
    private(set) var textView: UITextView?
    private var placeholder: UILabel?
    private var minimumLines = 1
    private var maximumLines: Int?
    private var initialFocusResolved = false
    private var initialFocusPending = false
    private var focusScheduled = false
    private var lastModelValue: String?
    private var componentRef: NexaTextInputRef?
    private var componentRefGeneration: UInt64?
    private var focusRequestGeneration: UInt64 = 0
    private var configuredFieldPlaceholderText: String?
    private var configuredFieldPlaceholderFont: UIFont?
    private var configuredFieldPlaceholderColor: UIColor?

    func configure(owner: NexaTextInputPrimitive, coordinator: NexaNativeTextInput.Coordinator, initialFocus: Bool, text: String? = nil) {
        let requestedText = text ?? owner.text
        var needsLayout = false
        coordinator.isApplyingModel = true
        defer { coordinator.isApplyingModel = false; lastModelValue = requestedText }
        if owner.multiline != (textView != nil) || (textField == nil && textView == nil) {
            needsLayout = true
            textField?.delegate = nil
            textView?.delegate = nil
            textField?.removeFromSuperview()
            textView?.removeFromSuperview()
            textField = nil
            textView = nil
            placeholder = nil
            configuredFieldPlaceholderText = nil
            configuredFieldPlaceholderFont = nil
            configuredFieldPlaceholderColor = nil
            if owner.multiline {
                let view = UITextView()
                view.backgroundColor = .clear
                view.textContainerInset = .zero
                view.textContainer.lineFragmentPadding = 0
                view.contentInsetAdjustmentBehavior = .never
                view.showsHorizontalScrollIndicator = false
                view.showsVerticalScrollIndicator = false
                view.textAlignment = .natural
                view.delegate = coordinator
                let label = UILabel()
                label.isUserInteractionEnabled = false
                label.isAccessibilityElement = false
                view.addSubview(label)
                placeholder = label
                textView = view
                addSubview(view)
            } else {
                let field = UITextField()
                field.borderStyle = .none
                field.textAlignment = .natural
                field.delegate = coordinator
                field.addTarget(coordinator, action: #selector(NexaNativeTextInput.Coordinator.fieldChanged(_:)), for: .editingChanged)
                textField = field
                addSubview(field)
            }
        }
        let font = owner.nativeFont
        let textColor = owner.textColor.map { UIColor($0) } ?? .label
        let placeholderColor = owner.placeholderColor.map { UIColor($0) } ?? UIColor(red: 142.0 / 255, green: 142.0 / 255, blue: 147.0 / 255, alpha: 1)
        let nextMinimumLines = owner.minLines ?? 1
        if minimumLines != nextMinimumLines || maximumLines != owner.maxLines {
            needsLayout = true
        }
        minimumLines = nextMinimumLines
        maximumLines = owner.maxLines
        if let field = textField {
            if field.isSecureTextEntry != owner.secure { field.isSecureTextEntry = owner.secure }
            if field.font != font {
                field.font = font
                needsLayout = true
            }
            if field.textColor?.isEqual(textColor) != true { field.textColor = textColor }
            let placeholderMetricsChanged = configuredFieldPlaceholderText != owner.placeholder
                || configuredFieldPlaceholderFont != font
            if configuredFieldPlaceholderText != owner.placeholder
                || configuredFieldPlaceholderFont != font
                || configuredFieldPlaceholderColor?.isEqual(placeholderColor) != true {
                field.attributedPlaceholder = NSAttributedString(string: owner.placeholder, attributes: [.font: font, .foregroundColor: placeholderColor])
                configuredFieldPlaceholderText = owner.placeholder
                configuredFieldPlaceholderFont = font
                configuredFieldPlaceholderColor = placeholderColor
            }
            if placeholderMetricsChanged { needsLayout = true }
            if field.keyboardType != owner.keyboard { field.keyboardType = owner.keyboard }
            if field.returnKeyType != owner.returnKey { field.returnKeyType = owner.returnKey }
            if field.textContentType != owner.autofill { field.textContentType = owner.autofill }
            let autocorrection = owner.secure ? UITextAutocorrectionType.no : (owner.autocorrect.map { $0 ? .yes : .no } ?? .default)
            if field.autocorrectionType != autocorrection { field.autocorrectionType = autocorrection }
            let spellChecking = owner.secure ? UITextSpellCheckingType.no : .default
            if field.spellCheckingType != spellChecking { field.spellCheckingType = spellChecking }
            let capitalization = owner.secure ? UITextAutocapitalizationType.none : owner.capitalization
            if field.autocapitalizationType != capitalization { field.autocapitalizationType = capitalization }
            if field.text != requestedText && (field.markedTextRange == nil || lastModelValue != requestedText) {
                if field.markedTextRange != nil { field.unmarkText() }
                field.text = requestedText
            }
        }
        if let view = textView {
            if view.font != font {
                view.font = font
                needsLayout = true
            }
            if view.textColor?.isEqual(textColor) != true { view.textColor = textColor }
            if view.accessibilityLabel != owner.placeholder { view.accessibilityLabel = owner.placeholder }
            if view.keyboardType != owner.keyboard { view.keyboardType = owner.keyboard }
            if view.returnKeyType != owner.returnKey { view.returnKeyType = owner.returnKey }
            if view.textContentType != owner.autofill { view.textContentType = owner.autofill }
            let autocorrection = owner.autocorrect.map { $0 ? UITextAutocorrectionType.yes : .no } ?? .default
            if view.autocorrectionType != autocorrection { view.autocorrectionType = autocorrection }
            if view.autocapitalizationType != owner.capitalization { view.autocapitalizationType = owner.capitalization }
            if view.text != requestedText && (view.markedTextRange == nil || lastModelValue != requestedText) {
                if view.markedTextRange != nil { view.unmarkText() }
                view.text = requestedText
                needsLayout = true
            }
            if placeholder?.text != owner.placeholder { placeholder?.text = owner.placeholder }
            if placeholder?.font != font { placeholder?.font = font }
            if placeholder?.textColor?.isEqual(placeholderColor) != true { placeholder?.textColor = placeholderColor }
            refreshPlaceholder()
        }
        if !initialFocusResolved {
            initialFocusResolved = true
            initialFocusPending = initialFocus
        }
        attachComponentRef(owner.componentRef)
        if needsLayout {
            invalidateIntrinsicContentSize()
            setNeedsLayout()
        }
        scheduleInitialFocus()
    }

    func refreshPlaceholder() {
        let shouldHide = !(textView?.text.isEmpty ?? true)
        if placeholder?.isHidden != shouldHide { placeholder?.isHidden = shouldHide }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        textField?.frame = bounds
        textView?.frame = bounds
        if let placeholder {
            placeholder.frame = CGRect(origin: .zero, size: placeholder.sizeThatFits(bounds.size))
        }
        scheduleInitialFocus()
    }

    override var intrinsicContentSize: CGSize { measuredSize(width: bounds.width > 0 ? bounds.width : nil) }

    func measuredSize(width: CGFloat?) -> CGSize {
        if let field = textField {
            let height = ceil((field.font ?? UIFont.preferredFont(forTextStyle: .body)).lineHeight)
            if let width { return CGSize(width: width, height: height) }
            let ideal = field.sizeThatFits(CGSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude))
            return CGSize(width: ideal.width, height: height)
        }
        guard let view = textView else { return .zero }
        let font = view.font ?? UIFont.preferredFont(forTextStyle: .body)
        let availableWidth: CGFloat
        if let width {
            availableWidth = max(1, width)
        } else {
            let value = view.text ?? ""
            let text = value.isEmpty ? placeholder?.text ?? "" : value
            availableWidth = max(1, (text as NSString).size(withAttributes: [.font: font]).width)
        }
        let fitted = view.sizeThatFits(CGSize(width: availableWidth, height: CGFloat.greatestFiniteMagnitude)).height
        let minimum = font.lineHeight * CGFloat(minimumLines)
        let maximum = maximumLines.map { font.lineHeight * CGFloat($0) } ?? CGFloat.greatestFiniteMagnitude
        return CGSize(width: availableWidth, height: ceil(min(maximum, max(minimum, fitted))))
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil {
            detachMountedRef()
        } else {
            attachComponentRef(componentRef)
            scheduleInitialFocus()
        }
    }

    private func attachComponentRef(_ next: NexaTextInputRef?) {
        if componentRef !== next {
            detachMountedRef()
            componentRef = next
        }
        // SwiftUI can create temporary hosts during measurement. Only the
        // mounted editor owns the ref; a sizing host must never replace it.
        guard window != nil, componentRefGeneration == nil, let next else { return }
        componentRefGeneration = next.attach(
            view: self,
            focus: { [weak self] in self?.focusInput() },
            blur: { [weak self] in self?.blurInput() }
        )
    }

    func detachComponentRef() {
        detachMountedRef()
        componentRef = nil
    }

    private func detachMountedRef() {
        focusRequestGeneration &+= 1
        if let componentRef, let componentRefGeneration {
            componentRef.detach(generation: componentRefGeneration)
        }
        componentRefGeneration = nil
    }

    private func focusInput() {
        focusRequestGeneration &+= 1
        let request = focusRequestGeneration

        guard let input: UIView = textField ?? textView, input.window != nil else { return }
        if input.isFirstResponder || input.becomeFirstResponder() { return }
        // Retry only when UIKit temporarily refuses during a view update.
        DispatchQueue.main.async { [weak self] in
            guard let self, self.focusRequestGeneration == request,
                  let input: UIView = self.textField ?? self.textView,
                  input.window != nil, !input.isFirstResponder else { return }
            input.becomeFirstResponder()
        }
    }

    private func blurInput() {
        focusRequestGeneration &+= 1
        if let textField { textField.resignFirstResponder() }
        if let textView { textView.resignFirstResponder() }
    }

    private func scheduleInitialFocus() {
        guard window != nil, initialFocusPending, !focusScheduled else { return }
        focusScheduled = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.focusScheduled = false
            guard self.window != nil, self.initialFocusPending else { return }
            self.initialFocusPending = false
            if let field = self.textField {
                if !field.isFirstResponder && !field.becomeFirstResponder() {
                    self.initialFocusPending = true
                }
            }
            if let view = self.textView {
                if !view.isFirstResponder && !view.becomeFirstResponder() {
                    self.initialFocusPending = true
                }
            }
        }
    }
}
