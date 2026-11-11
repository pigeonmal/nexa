// nexa-codegen: NexaTextInputEditLimiter
internal class NexaTextInputEditLimiter {
    private var boundaries: android.icu.text.BreakIterator? = null
    private var previousBoundaries: android.icu.text.BreakIterator? = null

    fun edit(previous: String, proposed: String, maxLength: Int, maxLines: Int, multiline: Boolean): String {
        var next = proposed
        if (!multiline && next.any { it == '\r' || it == '\n' || it == '\u2028' || it == '\u2029' }) {
            next = buildString(next.length) {
                var previousCR = false
                for (character in proposed) {
                    if (character == '\n' && previousCR) { previousCR = false; continue }
                    append(if (character == '\r' || character == '\n' || character == '\u2028' || character == '\u2029') ' ' else character)
                    previousCR = character == '\r'
                }
            }
        }
        if (maxLength != Int.MAX_VALUE) {
            val iterator = boundaries ?: android.icu.text.BreakIterator.getCharacterInstance(java.util.Locale.ROOT).also { boundaries = it }
            iterator.setText(next)
            var end = 0
            var remaining = maxLength.coerceAtLeast(0)
            while (remaining > 0) {
                val boundary = iterator.next()
                if (boundary == android.icu.text.BreakIterator.DONE) { end = next.length; break }
                end = boundary
                remaining -= 1
            }
            if (end < next.length) next = clipEdit(previous, next, maxLength, end, iterator)
        }
        if (multiline && maxLines != Int.MAX_VALUE) {
            val count = nexaTextInputLineCount(next)
            if (count > maxLines && count > nexaTextInputLineCount(previous)) next = previous
        }
        // ICU iterators are reusable but otherwise retain the last proposed
        // buffer, including a rejected large paste, until the next edit.
        boundaries?.setText("")
        previousBoundaries?.setText("")
        return next
    }

    private fun clipEdit(previous: String, proposed: String, limit: Int, prefixEnd: Int, nextIterator: android.icu.text.BreakIterator): String {
        if (previous.isEmpty()) return proposed.substring(0, prefixEnd)
        val oldIterator = previousBoundaries ?: android.icu.text.BreakIterator.getCharacterInstance(java.util.Locale.ROOT).also { previousBoundaries = it }
        oldIterator.setText(previous)
        var oldCount = 0
        while (oldIterator.next() != android.icu.text.BreakIterator.DONE) oldCount += 1
        if (oldCount > limit) {
            nextIterator.setText(proposed)
            var newCount = 0
            while (nextIterator.next() != android.icu.text.BreakIterator.DONE) newCount += 1
            if (newCount < oldCount) return proposed
        }
        oldIterator.first()
        nextIterator.first()
        var prefix = 0
        var prefixCount = 0
        while (true) {
            val oldEnd = oldIterator.next()
            val newEnd = nextIterator.next()
            if (oldEnd == android.icu.text.BreakIterator.DONE || newEnd == android.icu.text.BreakIterator.DONE || oldEnd != newEnd || !previous.regionMatches(prefix, proposed, prefix, oldEnd - prefix)) break
            prefix = oldEnd
            prefixCount += 1
        }
        var oldEnd = oldIterator.last()
        var newEnd = nextIterator.last()
        var suffixCount = 0
        while (true) {
            val oldStart = oldIterator.previous()
            val newStart = nextIterator.previous()
            if (oldStart < prefix || newStart < prefix || oldEnd - oldStart != newEnd - newStart || !previous.regionMatches(oldStart, proposed, newStart, oldEnd - oldStart)) break
            oldEnd = oldStart
            newEnd = newStart
            suffixCount += 1
        }
        val room = (limit - prefixCount - suffixCount).coerceAtLeast(0)
        if (room == 0) return previous
        nextIterator.following(prefix)
        var insertionEnd = prefix
        var remaining = room
        while (remaining > 0 && insertionEnd < newEnd) {
            insertionEnd = nextIterator.current().coerceAtMost(newEnd)
            remaining -= 1
            nextIterator.next()
        }
        return proposed.substring(0, insertionEnd) + previous.substring(oldEnd)
    }

}

private fun nexaTextInputLineCount(text: String): Int {
    var lines = 1
    var previousCR = false
    for (character in text) {
        if (character == '\r' || character == '\u2028' || character == '\u2029' || (character == '\n' && !previousCR)) lines += 1
        previousCR = character == '\r'
    }
    return lines
}
