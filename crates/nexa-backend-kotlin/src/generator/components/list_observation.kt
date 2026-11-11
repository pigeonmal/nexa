// nexa-codegen: nexaLeadingVisibleDataItemIndex
// Inline geometry accessors keep this common to lists, grids and masonry
// without allocating adapters, filtered collections or boxed nullable indexes.
internal inline fun <Item> nexaLeadingVisibleDataItemIndex(
    visibleItems: List<Item>,
    fallback: Int,
    stickyHeaderOffset: Int,
    reverseLayout: Boolean,
    viewportStartOffset: Int,
    viewportEndOffset: Int,
    itemIndex: (Item) -> Int,
    itemOffset: (Item) -> Int,
    itemSize: (Item) -> Int,
): Int {
    var hasStickyHeader = false
    var stickyStart = 0
    var stickyEnd = 0
    if (stickyHeaderOffset > 0) {
        var position = 0
        while (position < visibleItems.size) {
            val item = visibleItems[position++]
            val offset = itemOffset(item)
            val end = offset + itemSize(item)
            if (itemIndex(item) == 0 && end > viewportStartOffset && offset < viewportEndOffset) {
                hasStickyHeader = true
                stickyStart = offset
                stickyEnd = end
                break
            }
        }
    }
    var leadingIndex = -1
    var firstDataIndex = -1
    var maximumVisibleIndex = -1
    var position = 0
    while (position < visibleItems.size) {
        val item = visibleItems[position++]
        val offset = itemOffset(item)
        val end = offset + itemSize(item)
        if (end <= viewportStartOffset || offset >= viewportEndOffset) continue
        val index = itemIndex(item)
        if (index > maximumVisibleIndex) maximumVisibleIndex = index
        if (index < stickyHeaderOffset) continue
        if (firstDataIndex < 0) firstDataIndex = index
        if (hasStickyHeader && offset < stickyEnd && end > stickyStart) continue
        if (leadingIndex < 0 || (reverseLayout && index > leadingIndex) || (!reverseLayout && index < leadingIndex)) {
            leadingIndex = index
        }
    }
    if (leadingIndex >= 0) return leadingIndex
    val visibleFallback = if (reverseLayout) maximumVisibleIndex else firstDataIndex
    return if (visibleFallback >= 0) visibleFallback else fallback
}
