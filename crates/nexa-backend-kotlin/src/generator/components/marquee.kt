// nexa-codegen: NexaMarqueeConfiguration
private data class NexaMarqueeConfiguration(
    val horizontal: Boolean, val speed: Double, val gap: Double, val delay: Double,
    val revision: List<String>, val content: Int, val viewport: Int, val cross: Int,
    val density: Float, val fontScale: Float, val rtl: Boolean,
)

// nexa-codegen: NexaMarqueeMetrics
private data class NexaMarqueeMetrics(val content: Int = 0, val viewport: Int = 0, val cross: Int = 0,
    val visible: Boolean = false)

// nexa-codegen: NexaMarqueeGeometry
private class NexaMarqueeGeometry {
    val updates = kotlinx.coroutines.flow.MutableStateFlow(NexaMarqueeMetrics())
    fun measure(content: Int, viewport: Int, cross: Int) {
        val current = updates.value
        if (current.content != content || current.viewport != viewport || current.cross != cross) {
            updates.value = current.copy(content = content, viewport = viewport, cross = cross)
        }
    }
    fun visibility(visible: Boolean) {
        val current = updates.value
        if (current.visible != visible) updates.value = current.copy(visible = visible)
    }
}

// nexa-codegen: NexaMarqueeClock
private class NexaMarqueeClock {
    var delayRemaining = 0L
    var configuration: NexaMarqueeConfiguration? = null
}

// nexa-codegen: NexaMarqueePrimitive
@androidx.compose.runtime.Composable
internal fun NexaMarqueePrimitive(
    horizontal: Boolean,
    speed: Double,
    gap: Double,
    delaySeconds: Double,
    edgeFade: Double,
    playing: Boolean,
    revision: List<String>,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val density = androidx.compose.ui.platform.LocalDensity.current
    val direction = androidx.compose.ui.platform.LocalLayoutDirection.current
    val view = androidx.compose.ui.platform.LocalView.current
    val lifecycleOwner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    val accessibility = view.context.getSystemService(android.view.accessibility.AccessibilityManager::class.java)
    var screenReader by androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(accessibility?.isTouchExplorationEnabled == true) }
    androidx.compose.runtime.DisposableEffect(accessibility) {
        val listener = android.view.accessibility.AccessibilityManager.TouchExplorationStateChangeListener { screenReader = it }
        accessibility?.addTouchExplorationStateChangeListener(listener)
        onDispose { accessibility?.removeTouchExplorationStateChangeListener(listener) }
    }
    val offset = androidx.compose.runtime.remember { androidx.compose.animation.core.Animatable(0f) }
    val clock = androidx.compose.runtime.remember { NexaMarqueeClock() }
    val geometry = androidx.compose.runtime.remember { NexaMarqueeGeometry() }
    var resumed by androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(
        lifecycleOwner.lifecycle.currentState.isAtLeast(androidx.lifecycle.Lifecycle.State.RESUMED)
    ) }
    androidx.compose.runtime.DisposableEffect(lifecycleOwner) {
        val observer = androidx.lifecycle.LifecycleEventObserver { _, _ ->
            resumed = lifecycleOwner.lifecycle.currentState.isAtLeast(androidx.lifecycle.Lifecycle.State.RESUMED)
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    val resolver = view.context.contentResolver
    fun systemAnimationsEnabled(): Boolean = android.provider.Settings.Global.getFloat(
        resolver, android.provider.Settings.Global.ANIMATOR_DURATION_SCALE, 1f) > 0f
    var animationsEnabled by androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(systemAnimationsEnabled()) }
    androidx.compose.runtime.DisposableEffect(resolver) {
        val observer = object : android.database.ContentObserver(android.os.Handler(android.os.Looper.getMainLooper())) {
            override fun onChange(selfChange: Boolean) { animationsEnabled = systemAnimationsEnabled() }
        }
        resolver.registerContentObserver(android.provider.Settings.Global.getUriFor(android.provider.Settings.Global.ANIMATOR_DURATION_SCALE), false, observer)
        onDispose { resolver.unregisterContentObserver(observer) }
    }
    val stationary = screenReader || !animationsEnabled
    androidx.compose.runtime.LaunchedEffect(horizontal, speed, gap, delaySeconds, revision,
        playing, resumed, stationary, density.density, density.fontScale, direction) {
        geometry.updates.collectLatest { metrics ->
            val configuration = NexaMarqueeConfiguration(horizontal, speed, gap, delaySeconds, revision,
                metrics.content, metrics.viewport, metrics.cross, density.density, density.fontScale,
                direction == androidx.compose.ui.unit.LayoutDirection.Rtl)
            if (clock.configuration != configuration) {
                clock.configuration = configuration
                offset.snapTo(0f)
                clock.delayRemaining = (delaySeconds * 1000.0).coerceAtMost(Long.MAX_VALUE.toDouble()).toLong()
            }
            if (stationary) { offset.snapTo(0f); return@collectLatest }
            val period = metrics.content.toFloat() + (gap * density.density).toFloat()
            if (metrics.viewport <= 0 || metrics.content <= metrics.viewport || !playing || !metrics.visible || !resumed || !period.isFinite()) return@collectLatest
            if (clock.delayRemaining > 0L) {
                val started = android.os.SystemClock.uptimeMillis()
                try { kotlinx.coroutines.delay(clock.delayRemaining) }
                finally { clock.delayRemaining = (clock.delayRemaining - (android.os.SystemClock.uptimeMillis() - started)).coerceAtLeast(0L) }
            }
            val pixelsPerSecond = (speed * density.density).toFloat()
            if (!pixelsPerSecond.isFinite() || pixelsPerSecond <= 0f) return@collectLatest
            while (true) {
                val remaining = (period - offset.value).coerceAtLeast(0f)
                val duration = ((remaining / pixelsPerSecond) * 1000.0).coerceIn(1.0, Int.MAX_VALUE.toDouble()).toInt()
                offset.animateTo(period, androidx.compose.animation.core.tween(durationMillis = duration,
                    easing = androidx.compose.animation.core.LinearEasing))
                offset.snapTo(0f)
            }
        }
    }

    // Keep the single source layer identity stable across playback-only updates.
    // Draw callbacks never publish animation state; the revision argument and
    // measured geometry changes are the only content restart signals.
    val sourceModifier = androidx.compose.runtime.remember(geometry) {
        androidx.compose.ui.Modifier.graphicsLayer()
    }
    val rtl = horizontal && direction == androidx.compose.ui.unit.LayoutDirection.Rtl
    androidx.compose.ui.layout.Layout(
        modifier = modifier
            .semantics(mergeDescendants = true) { }
            .clipToBounds()
            .onGloballyPositioned { coordinates ->
                val bounds = coordinates.boundsInWindow()
                geometry.visibility(coordinates.isAttached && bounds.width > 0f && bounds.height > 0f &&
                    bounds.right > 0f && bounds.bottom > 0f && bounds.left < view.width && bounds.top < view.height)
            }
            .drawWithCache {
                // Cache mask resources at viewport/configuration changes, never
                // at animation ticks. Alpha masking leaves viewport styling intact.
                val extent = if (horizontal) size.width else size.height
                val fraction = if (edgeFade > 0.0 && extent > 0f) {
                    ((edgeFade * density.density) / extent).coerceAtMost(0.5).toFloat()
                } else 0f
                val mask = if (fraction > 0f) androidx.compose.ui.graphics.Brush.linearGradient(
                    0f to androidx.compose.ui.graphics.Color.Transparent,
                    fraction to androidx.compose.ui.graphics.Color.Black,
                    (1f - fraction) to androidx.compose.ui.graphics.Color.Black,
                    1f to androidx.compose.ui.graphics.Color.Transparent,
                    start = androidx.compose.ui.geometry.Offset.Zero,
                    end = if (horizontal) androidx.compose.ui.geometry.Offset(extent, 0f)
                        else androidx.compose.ui.geometry.Offset(0f, extent),
                ) else null
                val maskPaint = if (mask != null) androidx.compose.ui.graphics.Paint() else null
                val viewportBounds = androidx.compose.ui.geometry.Rect(androidx.compose.ui.geometry.Offset.Zero, size)
                onDrawWithContent {
                    // Child content is a single recorded native layer. Read animated
                    // values only here; translation never recomposes or remeasures it.
                    val metrics = geometry.updates.value
                    val contentLength = metrics.content
                    val viewportLength = metrics.viewport
                    val period = contentLength.toFloat() + (gap * density.density).toFloat()
                    val overflowing = viewportLength > 0 && contentLength > viewportLength && period.isFinite()
                    val masked = mask != null && maskPaint != null && overflowing && !stationary
                    if (masked) drawContext.canvas.saveLayer(viewportBounds, maskPaint)
                    try {
                        val phase = if (overflowing && !stationary) offset.value else 0f
                        val base = if (rtl && overflowing) (viewportLength - contentLength).toFloat() else 0f
                        val shift = base + if (rtl) phase else -phase
                        translate(left = if (horizontal) shift else 0f, top = if (horizontal) 0f else shift) {
                            this@onDrawWithContent.drawContent()
                        }
                        if (overflowing && !stationary) {
                            val repeatShift = shift + if (rtl) -period else period
                            translate(left = if (horizontal) repeatShift else 0f, top = if (horizontal) 0f else repeatShift) {
                                this@onDrawWithContent.drawContent()
                            }
                        }
                        if (masked) drawRect(brush = mask, blendMode = androidx.compose.ui.graphics.BlendMode.DstIn)
                    } finally {
                        if (masked) drawContext.canvas.restore()
                    }
                }
            },
        content = {
            androidx.compose.foundation.layout.Box(propagateMinConstraints = true, modifier = sourceModifier) { content() }
        },
    ) { measurables, constraints ->
        val childConstraints = constraints.copy(minWidth = if (horizontal) 0 else constraints.minWidth, minHeight = if (horizontal) constraints.minHeight else 0,
            maxWidth = if (horizontal) androidx.compose.ui.unit.Constraints.Infinity else constraints.maxWidth,
            maxHeight = if (horizontal) constraints.maxHeight else androidx.compose.ui.unit.Constraints.Infinity)
        val child = measurables.first().measure(childConstraints)
        val width = if (horizontal && constraints.hasBoundedWidth) constraints.maxWidth else constraints.constrainWidth(child.width)
        val height = if (!horizontal && constraints.hasBoundedHeight) constraints.maxHeight else constraints.constrainHeight(child.height)
        geometry.measure(if (horizontal) child.width else child.height, if (horizontal) width else height, if (horizontal) height else width)
        layout(width, height) { child.place(0, 0) }
    }
}
