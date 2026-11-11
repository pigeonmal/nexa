@MainActor
private final class InputCheckModel {
    var value = ""
    var changes: [String] = []
    var submissions = 0
}

@main
struct TextInputControlCheck {
    @MainActor static func main() {
        var checks = 0
        func check(_ condition: @autoclosure () -> Bool, _ message: String = "Native TextInput regression") {
            precondition(condition(), message)
            checks += 1
        }
        func make(_ model: InputCheckModel, multiline: Bool = false, minimum: Int? = nil, maximum: Int? = nil, length: Int? = nil) -> (NexaTextInputPrimitive, NexaNativeTextInput.Coordinator, NexaTextInputHost) {
            let owner = NexaTextInputPrimitive(
                text: Binding(get: { model.value }, set: { model.value = $0 }),
                placeholder: "Message", secure: false, multiline: multiline, searchable: false,
                maxLength: length, fontSize: nil, fontFamily: nil, fontRelativeTo: .body,
                fontDefaultSize: 17, textColor: nil, placeholderColor: nil, clearable: false,
                minLines: minimum, maxLines: maximum, returnKey: multiline ? .default : .next,
                onSubmitAction: { model.submissions += 1 },
                onChange: { model.changes.append($0) }
            )
            let coordinator = NexaNativeTextInput.Coordinator(owner: owner)
            let host = NexaTextInputHost()
            host.configure(owner: owner, coordinator: coordinator, initialFocus: false)
            return (owner, coordinator, host)
        }
        func insert(_ host: NexaTextInputHost, _ coordinator: NexaNativeTextInput.Coordinator, _ text: String, range: NSRange? = nil) {
            if let field = host.textField {
                let old = field.text ?? ""
                let range = range ?? NSRange(location: old.utf16.count, length: 0)
                if coordinator.textField(field, shouldChangeCharactersIn: range, replacementString: text) {
                    field.text = (old as NSString).replacingCharacters(in: range, with: text)
                    coordinator.fieldChanged(field)
                }
            } else if let view = host.textView {
                let old = view.text ?? ""
                let range = range ?? NSRange(location: old.utf16.count, length: 0)
                if coordinator.textView(view, shouldChangeTextIn: range, replacementText: text) {
                    view.text = (old as NSString).replacingCharacters(in: range, with: text)
                    coordinator.textViewDidChange(view)
                }
            }
        }
        let single = InputCheckModel()
        let (owner, coordinator, host) = make(single, length: 12)
        let singleHeight = host.measuredSize(width: 240).height
        for character in "abcdefghijklmnopqrst" { insert(host, coordinator, String(character)) }
        check(single.value == "abcdefghijkl")
        check(host.textField?.text == single.value, "native editing buffer diverged from app state")
        check(host.measuredSize(width: 240).height == singleHeight)
        check(single.changes.count == 12, "rejected edits must not publish changes")
        single.value = "app update"
        host.configure(owner: owner, coordinator: coordinator, initialFocus: false)
        check(host.textField?.text == "app update" && single.changes.count == 12)
        if let field = host.textField { _ = coordinator.textFieldShouldReturn(field) }
        check(single.submissions == 1)

        let middle = InputCheckModel()
        middle.value = "abcd"
        let (_, middleCoordinator, middleHost) = make(middle, length: 4)
        insert(middleHost, middleCoordinator, "X", range: NSRange(location: 2, length: 0))
        check(middle.value == "abcd" && middleHost.textField?.text == "abcd")
        check(middle.changes.isEmpty)

        let multiline = InputCheckModel()
        let (multiOwner, multiCoordinator, multiHost) = make(multiline, multiline: true, minimum: 3, maximum: 5)
        let emptyHeight = multiHost.measuredSize(width: 240).height
        check(emptyHeight >= multiOwner.nativeFont.lineHeight * 3)
        for character in "one\ntwo\nthree\nfour\nfive" { insert(multiHost, multiCoordinator, String(character)) }
        let fiveLines = multiline.value
        let maximumHeight = multiHost.measuredSize(width: 240).height
        check(maximumHeight > emptyHeight)
        check(maximumHeight <= ceil(multiOwner.nativeFont.lineHeight * 5))
        let changeCount = multiline.changes.count
        insert(multiHost, multiCoordinator, "\n")
        check(multiline.value == fiveLines && multiHost.textView?.text == fiveLines)
        check(multiline.changes.count == changeCount)
        insert(multiHost, multiCoordinator, "", range: NSRange(location: fiveLines.utf16.count - 1, length: 1))
        check(multiline.value == "one\ntwo\nthree\nfour\nfiv")

        let unbounded = InputCheckModel()
        let (_, unboundedCoordinator, unboundedHost) = make(unbounded, multiline: true, minimum: 2)
        insert(unboundedHost, unboundedCoordinator, "a\nb\nc\nd\ne\nf")
        check(unbounded.value == "a\nb\nc\nd\ne\nf")
        check(unboundedHost.measuredSize(width: 240).height > emptyHeight)

        var submitOwner = multiOwner
        submitOwner.returnKey = .search
        multiCoordinator.owner = submitOwner
        let beforeSubmit = multiline.value
        insert(multiHost, multiCoordinator, "\n")
        check(multiline.value == beforeSubmit)
        check(multiline.submissions == 1)
        print("{\"nativeControlChecks\":\(checks),\"passed\":true}")
    }
}
