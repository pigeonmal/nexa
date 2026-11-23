package __NEXA_PACKAGE__

import androidx.compose.foundation.gestures.anchoredDraggable
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.snapTo
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.roundToInt

// nexa-codegen: NexaFastListSwipePrimitive
internal val LocalNexaFastListSwipePagerLock =
    androidx.compose.runtime.staticCompositionLocalOf<androidx.compose.runtime.MutableState<Boolean>?> { null }

internal enum class NexaFastListSwipeEdge { Leading, Trailing }

private enum class NexaFastListSwipePosition { Closed, Open, FullSwipe }

private class NexaFastListOpenSwipe(private val scope: CoroutineScope) {
    private var opened: androidx.compose.foundation.gestures.AnchoredDraggableState<NexaFastListSwipePosition>? = null

    fun open(row: androidx.compose.foundation.gestures.AnchoredDraggableState<NexaFastListSwipePosition>) {
        closeOther(row)
        opened = row
    }

    fun closeOther(row: androidx.compose.foundation.gestures.AnchoredDraggableState<NexaFastListSwipePosition>) {
        val previous = opened ?: return
        if (previous === row) return
        opened = null
        scope.launch { previous.animateTo(NexaFastListSwipePosition.Closed) }
    }

    fun forget(row: androidx.compose.foundation.gestures.AnchoredDraggableState<NexaFastListSwipePosition>) {
        if (opened === row) opened = null
    }
}

private val LocalNexaFastListOpenSwipe =
    androidx.compose.runtime.staticCompositionLocalOf<NexaFastListOpenSwipe?> { null }

@androidx.compose.runtime.Composable
internal fun NexaFastListSwipeGroup(content: @androidx.compose.runtime.Composable () -> Unit) {
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val group = androidx.compose.runtime.remember(scope) { NexaFastListOpenSwipe(scope) }
    androidx.compose.runtime.CompositionLocalProvider(LocalNexaFastListOpenSwipe provides group, content = content)
}

@androidx.compose.runtime.Composable
internal fun NexaFastListSwipePrimitive(
    resetKey: Any?,
    edge: NexaFastListSwipeEdge,
    allowsFullSwipe: Boolean,
    onFullSwipe: (() -> Unit)? = null,
    backgroundContent: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val state = androidx.compose.runtime.remember(resetKey, edge) {
        androidx.compose.foundation.gestures.AnchoredDraggableState(NexaFastListSwipePosition.Closed)
    }
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val latestOnFullSwipe = androidx.compose.runtime.rememberUpdatedState(onFullSwipe)
    val direction = if (edge == NexaFastListSwipeEdge.Leading) 1 else -1
    val widths = androidx.compose.runtime.remember { IntArray(2) }
    fun updateAnchors() {
        val rowWidth = widths[0]
        val actionWidth = widths[1].coerceAtMost(rowWidth)
        state.updateAnchors(
            androidx.compose.foundation.gestures.DraggableAnchors {
                NexaFastListSwipePosition.Closed at 0f
                if (actionWidth > 0) {
                    NexaFastListSwipePosition.Open at (direction * actionWidth).toFloat()
                    if (allowsFullSwipe && rowWidth > actionWidth) {
                        NexaFastListSwipePosition.FullSwipe at (direction * rowWidth).toFloat()
                    }
                }
            },
        )
    }

    val closeSwipe: () -> Unit = androidx.compose.runtime.remember(state, scope) {
        {
            scope.launch { state.animateTo(NexaFastListSwipePosition.Closed) }
            Unit
        }
    }
    androidx.compose.runtime.LaunchedEffect(state, allowsFullSwipe) {
        updateAnchors()
        if (allowsFullSwipe) {
            androidx.compose.runtime.snapshotFlow { state.settledValue }.collect { position ->
                if (position == NexaFastListSwipePosition.FullSwipe) {
                    state.snapTo(NexaFastListSwipePosition.Closed)
                    latestOnFullSwipe.value?.invoke()
                }
            }
        }
    }

    val layoutDirection = androidx.compose.ui.platform.LocalLayoutDirection.current
    val physicalDirection = if (layoutDirection == androidx.compose.ui.unit.LayoutDirection.Ltr) direction else -direction
    val pagerLock = LocalNexaFastListSwipePagerLock.current
    val openSwipe = LocalNexaFastListOpenSwipe.current
    val swipeGesture = if (pagerLock == null && openSwipe == null) androidx.compose.ui.Modifier else {
        androidx.compose.ui.Modifier.pointerInput(pagerLock, openSwipe, state, physicalDirection) {
            awaitEachGesture {
                val down = awaitFirstDown(
                    requireUnconsumed = false,
                    pass = androidx.compose.ui.input.pointer.PointerEventPass.Initial,
                )
                openSwipe?.closeOther(state)
                if (pagerLock == null) return@awaitEachGesture
                var locked = false
                try {
                    while (true) {
                        val change = awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial)
                            .changes.firstOrNull { it.id == down.id } ?: break
                        if (!change.pressed) break
                        val movement = change.position - down.position
                        if (!locked && abs(movement.x) > viewConfiguration.touchSlop && abs(movement.x) > abs(movement.y)) {
                            val opening = movement.x * physicalDirection > 0
                            val open = state.settledValue != NexaFastListSwipePosition.Closed
                            if (opening || open) {
                                pagerLock.value = true
                                locked = true
                            }
                        }
                    }
                } finally {
                    if (locked) pagerLock.value = false
                }
            }
        }
    }

    val isOpen = state.settledValue == NexaFastListSwipePosition.Open
    if (isOpen && openSwipe != null) {
        androidx.compose.runtime.DisposableEffect(openSwipe, state) {
            openSwipe.open(state)
            onDispose { openSwipe.forget(state) }
        }
    }
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .clipToBounds()
            .onSizeChanged { size ->
                widths[0] = size.width
                updateAnchors()
            }
            .then(swipeGesture)
            .anchoredDraggable(state, androidx.compose.foundation.gestures.Orientation.Horizontal)
            .semantics {
                val action = if (isOpen) {
                    androidx.compose.ui.semantics.CustomAccessibilityAction("Close row actions") {
                        closeSwipe()
                        true
                    }
                } else {
                    androidx.compose.ui.semantics.CustomAccessibilityAction("Show row actions") {
                        scope.launch { state.animateTo(NexaFastListSwipePosition.Open) }
                        true
                    }
                }
                customActions = listOf(action)
            },
    ) {
        androidx.compose.ui.layout.Layout(
            modifier = androidx.compose.ui.Modifier.matchParentSize().drawWithContent {
                val reveal = abs(state.offset.takeIf(Float::isFinite) ?: 0f)
                if (reveal > 0f) {
                    if (physicalDirection > 0) {
                        clipRect(right = reveal) { this@drawWithContent.drawContent() }
                    } else {
                        clipRect(left = size.width - reveal) { this@drawWithContent.drawContent() }
                    }
                }
            },
            content = {
                androidx.compose.runtime.CompositionLocalProvider(LocalNexaFastListSwipeClose provides closeSwipe) {
                    androidx.compose.foundation.layout.Row(
                        modifier = androidx.compose.ui.Modifier.onSizeChanged { size ->
                            widths[1] = size.width
                            updateAnchors()
                        }.then(
                            if (isOpen) androidx.compose.ui.Modifier else androidx.compose.ui.Modifier.clearAndSetSemantics { },
                        ),
                        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                        content = backgroundContent,
                    )
                }
            },
        ) { measurables, constraints ->
            val actions = measurables.single().measure(constraints.copy(minWidth = 0, minHeight = 0))
            val x = if (edge == NexaFastListSwipeEdge.Leading) 0 else constraints.maxWidth - actions.width
            layout(constraints.maxWidth, constraints.maxHeight) {
                // Unplaced actions cannot receive touches through a closed row.
                if (state.offset.isFinite() && state.offset != 0f) {
                    actions.placeRelative(x, (constraints.maxHeight - actions.height) / 2)
                }
            }
        }
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier.fillMaxWidth().offset {
                val offset = state.offset.takeIf(Float::isFinite) ?: 0f
                androidx.compose.ui.unit.IntOffset(offset.roundToInt(), 0)
            },
        ) {
            content()
        }
    }
}
