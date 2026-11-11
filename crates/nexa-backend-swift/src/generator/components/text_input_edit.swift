// nexa-codegen: nexaTextInputEdit
// Limits native edits only; programmatic state remains owned by the app.
func nexaTextInputEdit(previous: String, proposed: String, maxLength: Int?, maxLines: Int?, multiline: Bool) -> String {
    var next = proposed
    if !multiline && next.contains(where: { $0 == "\n" || $0 == "\r" || $0 == "\r\n" || $0 == "\u{2028}" || $0 == "\u{2029}" }) {
        next = String(next.map { character in
            character == "\n" || character == "\r" || character == "\r\n" || character == "\u{2028}" || character == "\u{2029}" ? Character(" ") : character
        })
    }
    if let maxLength {
        let prefix = next.prefix(max(0, maxLength))
        if prefix.endIndex != next.endIndex {
            next = nexaTextInputClipEdit(previous: previous, proposed: next, limit: maxLength)
        }
    }
    if multiline, let maxLines {
        let count = nexaTextInputLineCount(next)
        // Existing over-limit app data can still be edited down to the limit.
        if count > maxLines && count > nexaTextInputLineCount(previous) { return previous }
    }
    return next
}

private func nexaTextInputLineCount(_ text: String) -> Int {
    var lines = 1
    var previousCR = false
    for scalar in text.unicodeScalars {
        if scalar == "\r" || scalar == "\u{2028}" || scalar == "\u{2029}" || (scalar == "\n" && !previousCR) { lines += 1 }
        previousCR = scalar == "\r"
    }
    return lines
}

private func nexaTextInputClipEdit(previous: String, proposed: String, limit: Int) -> String {
    if previous.isEmpty { return String(proposed.prefix(max(0, limit))) }
    if previous.count > limit && proposed.count < previous.count { return proposed }
    var oldStart = previous.startIndex
    var newStart = proposed.startIndex
    var prefixCount = 0
    while oldStart < previous.endIndex && newStart < proposed.endIndex && previous[oldStart] == proposed[newStart] {
        previous.formIndex(after: &oldStart)
        proposed.formIndex(after: &newStart)
        prefixCount += 1
    }
    var oldEnd = previous.endIndex
    var newEnd = proposed.endIndex
    var suffixCount = 0
    while oldEnd > oldStart && newEnd > newStart {
        let oldIndex = previous.index(before: oldEnd)
        let newIndex = proposed.index(before: newEnd)
        if previous[oldIndex] != proposed[newIndex] { break }
        oldEnd = oldIndex
        newEnd = newIndex
        suffixCount += 1
    }
    let room = max(0, limit - prefixCount - suffixCount)
    if room == 0 { return previous }
    return String(proposed[..<newStart]) + String(proposed[newStart..<newEnd].prefix(room)) + String(previous[oldEnd...])
}
