// nexa-codegen: nexaLeadingVisibleDataItemIndex
// Inline geometry accessors keep this common to lists, grids and masonry
// without allocating adapters, filtered collections or boxed nullable indexes.
internal inline fun <Item> nexaLeadingVisibleDataItemIndex(
    visibleItems: List<Item>,
    fallback: Int,
    stickyHeaderOffset: Int,
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
    var position = 0
    while (position < visibleItems.size) {
        val item = visibleItems[position++]
        val offset = itemOffset(item)
        val end = offset + itemSize(item)
        if (end <= viewportStartOffset || offset >= viewportEndOffset) continue
        val index = itemIndex(item)
        if (index < stickyHeaderOffset) continue
        if (firstDataIndex < 0) firstDataIndex = index
        if (hasStickyHeader && offset < stickyEnd && end > stickyStart) continue
        if (leadingIndex < 0 || index < leadingIndex) {
            leadingIndex = index
        }
    }
    if (leadingIndex >= 0) return leadingIndex
    return if (firstDataIndex >= 0) firstDataIndex else fallback
}

// nexa-codegen: nexaListEndIsVisible
internal inline fun <Item> nexaListEndIsVisible(
    visibleItems: List<Item>,
    finalItemIndex: Int,
    hasStickyHeader: Boolean,
    viewportStartOffset: Int,
    viewportEndOffset: Int,
    itemIndex: (Item) -> Int,
    itemOffset: (Item) -> Int,
    itemSize: (Item) -> Int,
): Boolean {
    // Most layouts put the final item near the end. Traversal order does not
    // affect the result, including for pinned headers and staggered grids.
    for (position in visibleItems.lastIndex downTo 0) {
        val item = visibleItems[position]
        if (itemIndex(item) < finalItemIndex) continue
        val start = itemOffset(item)
        val end = start + itemSize(item)
        if (end <= viewportStartOffset || start >= viewportEndOffset) continue
        if (!hasStickyHeader) return true
        val header = visibleItems.firstOrNull { itemIndex(it) == 0 } ?: return true
        val headerStart = itemOffset(header)
        if (end <= headerStart || start >= headerStart + itemSize(header)) return true
    }
    return false
}

// nexa-codegen: nexaSectionedListLayoutIndex
internal inline fun <Section> nexaSectionedListLayoutIndex(
    sections: List<Section>,
    row: Int,
    hasHeader: Boolean,
    hasSectionGap: Boolean,
    sectionSize: (Section) -> Int,
): Int {
    var remaining = row
    var layoutIndex = 0
    for (sectionIndex in sections.indices) {
        val count = sectionSize(sections[sectionIndex])
        if (hasHeader) layoutIndex++
        if (remaining < count) return layoutIndex + remaining
        remaining -= count
        layoutIndex += count
        if (hasSectionGap && sectionIndex < sections.lastIndex) layoutIndex++
    }
    return 0
}
