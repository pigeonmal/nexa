// nexa-codegen: NexaTextRevealPrimitive
@androidx.compose.runtime.Composable
internal fun <Segment> NexaTextRevealPrimitive(
    text: String,
    segments: List<Segment>,
    textOf: (Segment) -> String,
    startTimeOf: (Segment) -> Double,
    endTimeOf: (Segment) -> Double,
    hasTrailingSpaceOf: (Segment) -> Boolean,
    position: Double,
    active: Boolean,
    playing: Boolean,
    enabled: Boolean,
    seekRevision: Long,
    dimOpacity: Float,
    leadTime: Float,
    targetOffset: Float,
    smoothingDurationMs: Int,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    color: androidx.compose.ui.graphics.Color = androidx.compose.ui.graphics.Color.Unspecified,
    fontSize: androidx.compose.ui.unit.TextUnit = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
    fontWeight: androidx.compose.ui.text.font.FontWeight? = androidx.compose.ui.text.font.FontWeight.Normal,
    fontFamily: androidx.compose.ui.text.font.FontFamily? = null,
    textAlign: androidx.compose.ui.text.style.TextAlign? = null,
    maxLines: Int = Int.MAX_VALUE,
    lineHeight: androidx.compose.ui.unit.TextUnit = fontSize * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
    letterSpacing: androidx.compose.ui.unit.TextUnit = 0.sp,
    strikethrough: Boolean = false,
    italic: Boolean = false,
    underline: Boolean = false,
    textShadow: NexaGlyphShadow? = null,
) {
    val resolvedColor = if (color != androidx.compose.ui.graphics.Color.Unspecified) {
        color
    } else {
        androidx.compose.material3.LocalContentColor.current
    }
    val textDecoration = when {
        underline && strikethrough -> androidx.compose.ui.text.style.TextDecoration.Underline + androidx.compose.ui.text.style.TextDecoration.LineThrough
        underline -> androidx.compose.ui.text.style.TextDecoration.Underline
        strikethrough -> androidx.compose.ui.text.style.TextDecoration.LineThrough
        else -> null
    }
    val layoutState = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableStateOf<androidx.compose.ui.text.TextLayoutResult?>(null)
    }
    val timeline = androidx.compose.runtime.remember {
        androidx.compose.animation.core.Animatable(position.toFloat())
    }
    val previousSeekRevision = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableLongStateOf(seekRevision)
    }
    val wasActive = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableStateOf(false)
    }
    val wasEnabled = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableStateOf(false)
    }

    androidx.compose.runtime.LaunchedEffect(
        position, active, playing, enabled, seekRevision, segments,
    ) {
        val firstWordStart = segments.firstOrNull()?.let(startTimeOf) ?: position
        val didSeek = seekRevision != previousSeekRevision.longValue
        when {
            didSeek -> {
                timeline.snapTo(kotlin.math.min(position, firstWordStart).toFloat())
                previousSeekRevision.longValue = seekRevision
            }
            !active || !playing || !enabled -> timeline.snapTo(position.toFloat())
            !wasActive.value || !wasEnabled.value -> timeline.snapTo((position + leadTime).toFloat())
        }
        if (active && playing && enabled && segments.isNotEmpty() && !didSeek) {
            timeline.animateTo(
                targetValue = (position + targetOffset).toFloat(),
                animationSpec = androidx.compose.animation.core.tween(
                    durationMillis = smoothingDurationMs.coerceAtLeast(1),
                    easing = androidx.compose.animation.core.LinearEasing,
                ),
            )
        }
        wasActive.value = active
        wasEnabled.value = enabled
    }

    val layout = layoutState.value
    val lineTimings = androidx.compose.runtime.remember(layout, segments) {
        if (layout == null || segments.isEmpty()) {
            emptyList()
        } else {
            nexaBuildTextRevealLines(layout, segments, textOf, startTimeOf, endTimeOf, hasTrailingSpaceOf)
        }
    }
    val shadow = textShadow?.let { value ->
        with(androidx.compose.ui.platform.LocalDensity.current) {
            androidx.compose.ui.graphics.Shadow(
                color = value.color,
                offset = androidx.compose.ui.geometry.Offset(value.x.toPx(), value.y.toPx()),
                blurRadius = value.radius.toPx(),
            )
        }
    }
    val resolvedModifier = modifier.then(
        androidx.compose.ui.Modifier.drawWithContent {
            drawContent()
            if (!active) return@drawWithContent
            val textLayout = layoutState.value ?: return@drawWithContent
            if (!enabled || segments.isEmpty() || lineTimings.isEmpty()) {
                drawText(textLayout, color = resolvedColor)
                return@drawWithContent
            }
            val currentPosition = timeline.value.toDouble()
            lineTimings.forEach { timing ->
                val progress = nexaTextRevealProgress(timing, currentPosition)
                if (progress <= 0.0) return@forEach
                val lineIndex = timing.lineIndex
                val lineLeft = textLayout.getLineLeft(lineIndex)
                val lineRight = textLayout.getLineRight(lineIndex)
                val lineWidth = (lineRight - lineLeft).coerceAtLeast(0f)
                val revealWidth = lineWidth * progress.toFloat()
                val rtl = textLayout.getParagraphDirection(lineIndex) ==
                    androidx.compose.ui.text.style.ResolvedTextDirection.Rtl
                val revealLeft = if (rtl) lineRight - revealWidth else lineLeft
                val revealRight = if (rtl) lineRight else lineLeft + revealWidth
                clipRect(
                    left = revealLeft,
                    top = textLayout.getLineTop(lineIndex),
                    right = revealRight,
                    bottom = textLayout.getLineBottom(lineIndex),
                ) {
                    drawText(textLayout, color = resolvedColor)
                }
            }
        },
    )

    androidx.compose.material3.Text(
        text = text,
        modifier = resolvedModifier,
        color = resolvedColor.copy(alpha = resolvedColor.alpha * dimOpacity.coerceIn(0f, 1f)),
        fontSize = fontSize,
        fontWeight = fontWeight,
        fontFamily = fontFamily,
        textAlign = textAlign ?: LocalNexaTextAlignment.current,
        maxLines = maxLines,
        overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
        lineHeight = lineHeight,
        letterSpacing = letterSpacing,
        fontStyle = if (italic) androidx.compose.ui.text.font.FontStyle.Italic else null,
        textDecoration = textDecoration,
        style = androidx.compose.ui.text.TextStyle(shadow = shadow),
        onTextLayout = { result -> layoutState.value = result },
    )
}

private data class NexaTextRevealWordTiming(
    val startTime: Double,
    val endTime: Double,
    val startProgress: Double,
    val endProgress: Double,
)

private data class NexaTextRevealLineTiming(
    val lineIndex: Int,
    val startTime: Double,
    val endTime: Double,
    val words: List<NexaTextRevealWordTiming>,
)

private fun <Segment> nexaBuildTextRevealLines(
    layout: androidx.compose.ui.text.TextLayoutResult,
    segments: List<Segment>,
    textOf: (Segment) -> String,
    startTimeOf: (Segment) -> Double,
    endTimeOf: (Segment) -> Double,
    hasTrailingSpaceOf: (Segment) -> Boolean,
): List<NexaTextRevealLineTiming> {
    val wordUnits = IntArray(segments.size) { index ->
        textOf(segments[index]).length.coerceAtLeast(1) + if (hasTrailingSpaceOf(segments[index])) 1 else 0
    }
    val cumulativeUnits = IntArray(segments.size + 1)
    for (index in segments.indices) cumulativeUnits[index + 1] = cumulativeUnits[index] + wordUnits[index]
    val totalUnits = cumulativeUnits.last().coerceAtLeast(1)
    val lineCount = layout.lineCount
    val totalWidth = (0 until lineCount).sumOf {
        (layout.getLineRight(it) - layout.getLineLeft(it)).coerceAtLeast(0f).toDouble()
    }
    var startWord = 0
    var consumedWidth = 0.0
    return List(lineCount) { lineIndex ->
        val lineWidth = (layout.getLineRight(lineIndex) - layout.getLineLeft(lineIndex))
            .coerceAtLeast(0f)
            .toDouble()
        if (startWord >= segments.size) {
            val previousEnd = if (startWord > 0) endTimeOf(segments[startWord - 1]) else segments.firstOrNull()?.let(startTimeOf) ?: 0.0
            consumedWidth += lineWidth
            return@List NexaTextRevealLineTiming(lineIndex, previousEnd, previousEnd + 0.01, emptyList())
        }
        val remainingLines = lineCount - lineIndex - 1
        val maxEndWord = maxOf(startWord + 1, segments.size - remainingLines)
        val targetUnits = if (totalWidth > 0.0) {
            totalUnits * ((consumedWidth + lineWidth) / totalWidth)
        } else {
            totalUnits.toDouble() * (lineIndex + 1).toDouble() / lineCount
        }
        var endWord = minOf(startWord + 1, maxEndWord)
        while (endWord < maxEndWord && cumulativeUnits[endWord] < targetUnits) endWord += 1
        val startUnits = cumulativeUnits[startWord]
        val endUnits = maxOf(cumulativeUnits[endWord], startUnits + 1)
        val unitSpan = endUnits - startUnits
        val words = (startWord until endWord).map { wordIndex ->
            NexaTextRevealWordTiming(
                startTime = startTimeOf(segments[wordIndex]),
                endTime = endTimeOf(segments[wordIndex]),
                startProgress = (cumulativeUnits[wordIndex] - startUnits).toDouble() / unitSpan,
                endProgress = (cumulativeUnits[wordIndex + 1] - startUnits).toDouble() / unitSpan,
            )
        }
        val startTime = words.firstOrNull()?.startTime ?: (segments.getOrNull(startWord - 1)?.let(endTimeOf) ?: 0.0)
        val endTime = words.lastOrNull()?.endTime ?: (startTime + 0.01).coerceAtLeast(startTime + 0.01)
        startWord = endWord
        consumedWidth += lineWidth
        NexaTextRevealLineTiming(lineIndex, startTime, maxOf(endTime, startTime + 0.01), words)
    }
}

private fun nexaTextRevealProgress(timing: NexaTextRevealLineTiming, position: Double): Double {
    if (timing.words.isEmpty()) {
        return ((position - timing.startTime) / (timing.endTime - timing.startTime).coerceAtLeast(0.01)).coerceIn(0.0, 1.0)
    }
    val first = timing.words.first()
    val last = timing.words.last()
    if (position < first.startTime) return 0.0
    if (position >= last.endTime) return 1.0
    var low = 0
    var high = timing.words.lastIndex
    var active = -1
    while (low <= high) {
        val middle = (low + high) ushr 1
        if (timing.words[middle].startTime <= position) {
            active = middle
            low = middle + 1
        } else {
            high = middle - 1
        }
    }
    if (active < 0) return 0.0
    val word = timing.words[active]
    if (position >= word.endTime) return word.endProgress.coerceIn(0.0, 1.0)
    val withinWord = ((position - word.startTime) / (word.endTime - word.startTime).coerceAtLeast(0.01)).coerceIn(0.0, 1.0)
    return (word.startProgress + (word.endProgress - word.startProgress) * withinWord).coerceIn(0.0, 1.0)
}
