// nexa-codegen: nexaScrollReversedListFromTop
internal suspend fun nexaScrollReversedListFromTop(
    state: androidx.compose.foundation.lazy.LazyListState,
    count: Int,
    offset: Int,
    animated: Boolean,
) {
    if (count <= 0) return
    // Compose's logical start is the visual bottom of a reversed list.
    if (state.canScrollForward) {
        if (animated) state.animateScrollToItem(count - 1) else state.scrollToItem(count - 1)
    }
    if (offset > 0) {
        if (animated) state.animateScrollBy(-offset.toFloat()) else state.scrollBy(-offset.toFloat())
    }
}

// nexa-codegen: NexaBindFastListRef
@androidx.compose.runtime.Composable
internal fun NexaBindFastListRef(
    reference: NexaFastListRef?,
    scrollToIndex: suspend (Int, Boolean, Double) -> Unit,
    scrollToTop: suspend (Boolean) -> Unit,
    scrollToEnd: suspend (Boolean) -> Unit,
    scrollToOffset: suspend (Double, Boolean) -> Unit,
) {
    if (reference == null) return
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val index = androidx.compose.runtime.rememberUpdatedState(scrollToIndex)
    val top = androidx.compose.runtime.rememberUpdatedState(scrollToTop)
    val end = androidx.compose.runtime.rememberUpdatedState(scrollToEnd)
    val offset = androidx.compose.runtime.rememberUpdatedState(scrollToOffset)
    androidx.compose.runtime.DisposableEffect(reference) {
        val generation = reference.attach(
            scrollToIndex = { target, animated, position -> scope.launch { index.value(target, animated, position) } },
            scrollToTop = { animated -> scope.launch { top.value(animated) } },
            scrollToEnd = { animated -> scope.launch { end.value(animated) } },
            scrollToOffset = { target, animated -> scope.launch { offset.value(target, animated) } },
        )
        onDispose { reference.detach(generation) }
    }
}
