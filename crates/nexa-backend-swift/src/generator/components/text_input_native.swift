// nexa-codegen: NexaNativeTextInput
@MainActor
struct NexaNativeTextInput: UIViewRepresentable {
    var owner: NexaTextInputPrimitive
    var value: String
    var initialFocus: Bool

    func makeCoordinator() -> Coordinator { Coordinator(owner: owner) }

    func makeUIView(context: Context) -> NexaTextInputHost {
        let host = NexaTextInputHost()
        host.configure(owner: owner, coordinator: context.coordinator, initialFocus: initialFocus, text: value)
        return host
    }

    func updateUIView(_ host: NexaTextInputHost, context: Context) {
        context.coordinator.owner = owner
        host.configure(owner: owner, coordinator: context.coordinator, initialFocus: initialFocus, text: value)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: NexaTextInputHost, context: Context) -> CGSize? {
        uiView.measuredSize(width: proposal.width)
    }

    static func dismantleUIView(_ host: NexaTextInputHost, coordinator: Coordinator) {
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
        init(owner: NexaTextInputPrimitive) { self.owner = owner }

        private func accepted(_ previous: String, _ proposed: String) -> String {
            nexaTextInputEdit(previous: previous, proposed: proposed, maxLength: owner.maxLength, maxLines: owner.maxLines, multiline: owner.multiline)
        }

        private func publish(_ value: String) {
            guard !isApplyingModel && owner.text != value else { return }
            owner.text = value
            owner.onChange?(value)
        }

        private func edit(_ input: UIView & UITextInput, previous: String, range: NSRange, replacement: String) -> Bool {
            guard range.location <= previous.utf16.count && range.length <= previous.utf16.count - range.location else { return false }
            if input.markedTextRange != nil { return true }
            let proposal = (previous as NSString).replacingCharacters(in: range, with: replacement)
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
            let value = accepted(owner.text, field.text ?? "")
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

        func textFieldDidBeginEditing(_ textField: UITextField) { owner.nativeFocusChanged(true) }
        func textFieldDidEndEditing(_ textField: UITextField) { owner.nativeFocusChanged(false) }
        func textViewDidBeginEditing(_ textView: UITextView) { owner.nativeFocusChanged(true) }
        func textViewDidEndEditing(_ textView: UITextView) { owner.nativeFocusChanged(false) }

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

    func configure(owner: NexaTextInputPrimitive, coordinator: NexaNativeTextInput.Coordinator, initialFocus: Bool, text: String? = nil) {
        let requestedText = text ?? owner.text
        coordinator.isApplyingModel = true
        defer { coordinator.isApplyingModel = false; lastModelValue = requestedText }
        if owner.multiline != (textView != nil) || (textField == nil && textView == nil) {
            textField?.delegate = nil
            textView?.delegate = nil
            textField?.removeFromSuperview()
            textView?.removeFromSuperview()
            textField = nil
            textView = nil
            placeholder = nil
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
        minimumLines = owner.minLines ?? 1
        maximumLines = owner.maxLines
        if let field = textField {
            if field.isSecureTextEntry != owner.secure { field.isSecureTextEntry = owner.secure }
            if field.font != font { field.font = font }
            field.textColor = textColor
            field.attributedPlaceholder = NSAttributedString(string: owner.placeholder, attributes: [.font: font, .foregroundColor: placeholderColor])
            field.keyboardType = owner.keyboard
            field.returnKeyType = owner.returnKey
            field.textContentType = owner.autofill
            field.autocorrectionType = owner.secure ? .no : (owner.autocorrect.map { $0 ? .yes : .no } ?? .default)
            field.spellCheckingType = owner.secure ? .no : .default
            field.autocapitalizationType = owner.secure ? .none : owner.capitalization
            if field.text != requestedText && (field.markedTextRange == nil || lastModelValue != requestedText) {
                if field.markedTextRange != nil { field.unmarkText() }
                field.text = requestedText
            }
        }
        if let view = textView {
            if view.font != font { view.font = font }
            view.textColor = textColor
            view.accessibilityLabel = owner.placeholder
            view.keyboardType = owner.keyboard
            view.returnKeyType = owner.returnKey
            view.textContentType = owner.autofill
            view.autocorrectionType = owner.autocorrect.map { $0 ? .yes : .no } ?? .default
            view.autocapitalizationType = owner.capitalization
            if view.text != requestedText && (view.markedTextRange == nil || lastModelValue != requestedText) {
                if view.markedTextRange != nil { view.unmarkText() }
                view.text = requestedText
            }
            placeholder?.text = owner.placeholder
            placeholder?.font = font
            placeholder?.textColor = placeholderColor
            refreshPlaceholder()
        }
        if !initialFocusResolved {
            initialFocusResolved = true
            initialFocusPending = initialFocus
        }
        attachComponentRef(owner.componentRef)
        invalidateIntrinsicContentSize()
        setNeedsLayout()
        scheduleInitialFocus()
    }

    func refreshPlaceholder() { placeholder?.isHidden = !(textView?.text.isEmpty ?? true) }

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
            let ideal = field.sizeThatFits(CGSize(width: width ?? CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude))
            return CGSize(width: width ?? ideal.width, height: ceil((field.font ?? UIFont.preferredFont(forTextStyle: .body)).lineHeight))
        }
        guard let view = textView else { return .zero }
        let font = view.font ?? UIFont.preferredFont(forTextStyle: .body)
        let idealWidth = max(1, ((view.text.isEmpty ? placeholder?.text ?? "" : view.text) as NSString).size(withAttributes: [.font: font]).width)
        let availableWidth = max(1, width ?? idealWidth)
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
