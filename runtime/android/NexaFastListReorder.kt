package __NEXA_PACKAGE__

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridItemInfo
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.zIndex
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.roundToInt

// nexa-codegen: NexaFastListReorderState
@Stable
internal class NexaFastListReorderState(private val scope: CoroutineScope) {
    val activeIndex = mutableIntStateOf(-1)
    val settlingIndex = mutableIntStateOf(-1)
    val dragOffsetX = mutableFloatStateOf(0f)
    val dragOffset = mutableFloatStateOf(0f)
    val dragPointerX = mutableFloatStateOf(0f)
    val dragPointerY = mutableFloatStateOf(0f)
    val dragTouchX = mutableFloatStateOf(0f)
    val dragTouchY = mutableFloatStateOf(0f)
    private val settlingOffset = Animatable(Offset.Zero, Offset.VectorConverter)
    private var settlingJob: Job? = null
    private var settlingGeneration = 0
    private var draggedKey: Any? = null

    fun start(index: Int, key: Any?) {
        settlingGeneration++
        settlingJob?.cancel()
        settlingIndex.intValue = -1
        draggedKey = key
        dragOffsetX.floatValue = 0f
        dragOffset.floatValue = 0f
        dragPointerX.floatValue = 0f
        dragPointerY.floatValue = 0f
        dragTouchX.floatValue = 0f
        dragTouchY.floatValue = 0f
        activeIndex.intValue = index
    }

    fun finishDrag() {
        val index = activeIndex.intValue
        if (index < 0) return

        val offset = Offset(dragOffsetX.floatValue, dragOffset.floatValue)
        val generation = ++settlingGeneration
        settlingJob?.cancel()
        settlingJob = scope.launch {
            settlingOffset.snapTo(offset)
            if (generation != settlingGeneration) return@launch
            settlingIndex.intValue = index
            activeIndex.intValue = -1
            draggedKey = null
            try {
                settlingOffset.animateTo(
                    Offset.Zero,
                    animationSpec = spring(
                        stiffness = Spring.StiffnessMediumLow,
                        visibilityThreshold = Offset.VisibilityThreshold,
                    ),
                )
            } finally {
                if (generation == settlingGeneration) {
                    settlingJob = null
                    settlingIndex.intValue = -1
                    dragOffsetX.floatValue = 0f
                    dragOffset.floatValue = 0f
                }
            }
        }
    }

    fun settlingOffsetFor(index: Int): Offset =
        if (settlingIndex.intValue == index) settlingOffset.value else Offset.Zero

    fun activeKey(): Any? = draggedKey

    fun reset() {
        settlingGeneration++
        settlingJob?.cancel()
        settlingJob = null
        settlingIndex.intValue = -1
        draggedKey = null
        dragOffsetX.floatValue = 0f
        dragOffset.floatValue = 0f
        dragPointerX.floatValue = 0f
        dragPointerY.floatValue = 0f
        dragTouchX.floatValue = 0f
        dragTouchY.floatValue = 0f
        activeIndex.intValue = -1
    }
}

// nexa-codegen: NexaFastStaggeredGridReorderableItem
@Composable
internal fun NexaFastStaggeredGridReorderableItem(
    modifier: Modifier,
    index: Int,
    itemCount: Int,
    listState: LazyStaggeredGridState,
    reorderState: NexaFastListReorderState,
    enabled: Boolean,
    onMove: (Int, Int) -> Unit,
    content: @Composable () -> Unit,
) {
    val currentIndex = rememberUpdatedState(index)
    val currentItemCount = rememberUpdatedState(itemCount)
    val currentOnMove = rememberUpdatedState(onMove)

    Box(
        modifier = modifier
            .fillMaxWidth()
            .pointerInput(reorderState, enabled, listState) {
                if (!enabled) return@pointerInput

                var source = -1
                var sourceKey: Any? = null
                var touchX = 0f
                var touchY = 0f
                var pointerX = 0f
                var pointerY = 0f

                fun targetUnderDraggedItem(
                    visible: List<LazyStaggeredGridItemInfo>,
                    dragged: LazyStaggeredGridItemInfo,
                ): Int {
                    val count = currentItemCount.value
                    if (dragged.index != source || dragged.index !in 0 until count) return -1

                    val left = pointerX - touchX
                    val top = pointerY - touchY
                    val right = left + dragged.size.width
                    val bottom = top + dragged.size.height
                    var target = -1
                    var nearestDistance = Float.MAX_VALUE

                    for (candidate in visible) {
                        if (candidate.key == sourceKey || candidate.index !in 0 until count) continue
                        val centerX = candidate.offset.x + candidate.size.width / 2f
                        val centerY = candidate.offset.y + candidate.size.height / 2f
                        if (centerX < left || centerX > right || centerY < top || centerY > bottom) continue

                        val dx = centerX - pointerX
                        val dy = centerY - pointerY
                        val distance = dx * dx + dy * dy
                        if (distance < nearestDistance) {
                            nearestDistance = distance
                            target = candidate.index
                        }
                    }
                    return target
                }

                try {
                    detectDragGesturesAfterLongPress(
                        onDragStart = { touch ->
                            val index = currentIndex.value
                            val item = listState.layoutInfo.visibleItemsInfo.firstOrNull { it.index == index }
                            if (item != null && index in 0 until currentItemCount.value &&
                                reorderState.activeIndex.intValue < 0
                            ) {
                                source = index
                                sourceKey = item.key
                                touchX = touch.x
                                touchY = touch.y
                                pointerX = item.offset.x + touchX
                                pointerY = item.offset.y + touchY
                                reorderState.start(source, sourceKey)
                                reorderState.dragTouchX.floatValue = touchX
                                reorderState.dragTouchY.floatValue = touchY
                                reorderState.dragPointerX.floatValue = pointerX
                                reorderState.dragPointerY.floatValue = pointerY
                            }
                        },
                        onDrag = { change, amount ->
                            if (source >= 0) {
                                change.consume()
                                pointerX += amount.x
                                pointerY += amount.y
                                reorderState.dragPointerX.floatValue = pointerX
                                reorderState.dragPointerY.floatValue = pointerY

                                val visible = listState.layoutInfo.visibleItemsInfo
                                val dragged = visible.firstOrNull { it.key == sourceKey }
                                if (dragged != null) {
                                    reorderState.dragOffsetX.floatValue = pointerX - dragged.offset.x - touchX
                                    reorderState.dragOffset.floatValue = pointerY - dragged.offset.y - touchY

                                    val target = targetUnderDraggedItem(visible, dragged)
                                    if (target >= 0 && target != source) {
                                        val from = source
                                        source = target
                                        reorderState.activeIndex.intValue = target
                                        currentOnMove.value(from, target)
                                    }
                                }
                            }
                        },
                        onDragEnd = {
                            if (source >= 0) {
                                listState.layoutInfo.visibleItemsInfo.firstOrNull {
                                    it.key == sourceKey && it.index == source
                                }?.let { dragged ->
                                    reorderState.dragOffsetX.floatValue = pointerX - dragged.offset.x - touchX
                                    reorderState.dragOffset.floatValue = pointerY - dragged.offset.y - touchY
                                }
                                source = -1
                                sourceKey = null
                                reorderState.finishDrag()
                            }
                        },
                        onDragCancel = {
                            if (source >= 0) {
                                listState.layoutInfo.visibleItemsInfo.firstOrNull {
                                    it.key == sourceKey && it.index == source
                                }?.let { dragged ->
                                    reorderState.dragOffsetX.floatValue = pointerX - dragged.offset.x - touchX
                                    reorderState.dragOffset.floatValue = pointerY - dragged.offset.y - touchY
                                }
                                source = -1
                                sourceKey = null
                                reorderState.finishDrag()
                            }
                        },
                    )
                } finally {
                    if (source >= 0) reorderState.reset()
                }
            }
            .graphicsLayer {
                val dragging = reorderState.activeIndex.intValue == index
                val settling = reorderState.settlingOffsetFor(index)
                val draggedItem = if (dragging) {
                    val key = reorderState.activeKey()
                    listState.layoutInfo.visibleItemsInfo.firstOrNull { it.key == key }
                } else {
                    null
                }
                translationX = if (draggedItem != null) {
                    reorderState.dragPointerX.floatValue - draggedItem.offset.x - reorderState.dragTouchX.floatValue
                } else if (dragging) {
                    reorderState.dragOffsetX.floatValue
                } else {
                    settling.x
                }
                translationY = if (draggedItem != null) {
                    reorderState.dragPointerY.floatValue - draggedItem.offset.y - reorderState.dragTouchY.floatValue
                } else if (dragging) {
                    reorderState.dragOffset.floatValue
                } else {
                    settling.y
                }
            }
            .zIndex(if (reorderState.activeIndex.intValue == index || reorderState.settlingIndex.intValue == index) 1f else 0f),
    ) {
        content()
    }
}

@Composable
internal fun rememberNexaFastListReorderState(): NexaFastListReorderState {
    val scope = rememberCoroutineScope()
    return remember(scope) { NexaFastListReorderState(scope) }
}

// nexa-codegen: NexaFastListReorderableItem
@Composable
internal fun NexaFastListReorderableItem(
    modifier: Modifier,
    index: Int,
    itemCount: Int,
    listState: LazyListState,
    reorderState: NexaFastListReorderState,
    enabled: Boolean,
    listIndexOffset: Int = 0,
    onMove: (Int, Int) -> Unit,
    content: @Composable () -> Unit,
) {
    val currentIndex = rememberUpdatedState(index)
    val currentItemCount = rememberUpdatedState(itemCount)
    val currentOnMove = rememberUpdatedState(onMove)

    Box(
        modifier = modifier
            .fillMaxWidth()
            .pointerInput(reorderState, enabled, listState, listIndexOffset) {
                if (!enabled) return@pointerInput

                detectDragGesturesAfterLongPress(
                    onDragStart = {
                        val startIndex = currentIndex.value
                        val item = listState.layoutInfo.visibleItemsInfo.firstOrNull {
                            it.index == startIndex + listIndexOffset
                        }
                        if (item != null && startIndex in 0 until currentItemCount.value) {
                            reorderState.start(startIndex, item.key)
                        }
                    },
                    onDrag = { change, dragAmount ->
                        val source = reorderState.activeIndex.intValue
                        if (source >= 0) {
                            change.consume()
                            reorderState.dragOffset.floatValue += dragAmount.y

                            val visibleItems = listState.layoutInfo.visibleItemsInfo
                            val key = reorderState.activeKey()
                            val draggedItem = visibleItems.firstOrNull {
                                it.key == key && it.index == source + listIndexOffset
                            }
                            if (draggedItem != null) {
                                val draggedCenter = draggedItem.offset + draggedItem.size / 2 +
                                    reorderState.dragOffset.floatValue.roundToInt()
                                var targetIndex = -1
                                var targetCenter = 0
                                var targetDistance = Int.MAX_VALUE

                                for (item in visibleItems) {
                                    val candidateIndex = item.index - listIndexOffset
                                    if (candidateIndex !in 0 until currentItemCount.value) continue

                                    val itemCenter = item.offset + item.size / 2
                                    val distance = abs(itemCenter - draggedCenter)
                                    if (distance < targetDistance) {
                                        targetDistance = distance
                                        targetCenter = itemCenter
                                        targetIndex = candidateIndex
                                    }
                                }

                                if (targetIndex >= 0 && targetIndex != source) {
                                    val sourceCenter = draggedItem.offset + draggedItem.size / 2
                                    reorderState.dragOffset.floatValue -= (targetCenter - sourceCenter)
                                    currentOnMove.value(source, targetIndex)
                                    reorderState.activeIndex.intValue = targetIndex
                                }
                            }
                        }
                    },
                    onDragEnd = {
                        reorderState.finishDrag()
                    },
                    onDragCancel = {
                        reorderState.finishDrag()
                    },
                )
            }
            .graphicsLayer {
                val dragging = reorderState.activeIndex.intValue == index
                val settling = reorderState.settlingOffsetFor(index)
                translationY = if (dragging) reorderState.dragOffset.floatValue else settling.y
            }
            .zIndex(if (reorderState.activeIndex.intValue == index || reorderState.settlingIndex.intValue == index) 1f else 0f),
    ) {
        content()
    }
}
