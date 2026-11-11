// nexa-codegen: NexaNumberFlowPrimitive
// Commit history without snapshot state: recording a displayed value must not
// schedule a second composition or reverse a decreasing digit transition.
internal class NexaNumberFlowHistory(initialValue: Double) {
    private var previousValue = initialValue
    private var increasing = true

    fun direction(value: Double): Boolean = if (value == previousValue) increasing else value > previousValue

    fun commit(value: Double) {
        increasing = direction(value)
        previousValue = value
    }
}

@androidx.compose.runtime.Composable
internal fun NexaNumberFlowPrimitive(
    value: Double,
    format: String,
    currencyCode: String?,
    minimumFractionDigits: Int?,
    maximumFractionDigits: Int?,
    animationSpec: androidx.compose.animation.core.FiniteAnimationSpec<androidx.compose.ui.unit.IntOffset>,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    color: androidx.compose.ui.graphics.Color = androidx.compose.ui.graphics.Color.Unspecified,
    fontSize: androidx.compose.ui.unit.TextUnit = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
    fontWeight: androidx.compose.ui.text.font.FontWeight? = androidx.compose.ui.text.font.FontWeight.Normal,
    fontFamily: androidx.compose.ui.text.font.FontFamily? = null,
    textAlign: androidx.compose.ui.text.style.TextAlign? = null,
    maxLines: Int = Int.MAX_VALUE,
    lineHeight: androidx.compose.ui.unit.TextUnit = androidx.compose.ui.unit.TextUnit.Unspecified,
    letterSpacing: androidx.compose.ui.unit.TextUnit = androidx.compose.ui.unit.TextUnit.Unspecified,
    strikethrough: Boolean = false,
    italic: Boolean = false,
    underline: Boolean = false,
    selectable: Boolean = false,
    textShadow: NexaGlyphShadow? = null,
) {
    val locale = java.util.Locale.getDefault()
    val formatted = if (format == "Compact" && android.os.Build.VERSION.SDK_INT >= 24) {
        val compactFormatter = androidx.compose.runtime.remember(minimumFractionDigits, maximumFractionDigits, locale) {
            android.icu.text.CompactDecimalFormat.getInstance(
                locale,
                android.icu.text.CompactDecimalFormat.CompactStyle.SHORT,
            ).apply {
                minimumFractionDigits?.let { this.minimumFractionDigits = it }
                maximumFractionDigits?.let { this.maximumFractionDigits = it }
            }
        }
        androidx.compose.runtime.remember(value, compactFormatter) { compactFormatter.format(value) }
    } else {
        val numberFormat = androidx.compose.runtime.remember(format, currencyCode, minimumFractionDigits, maximumFractionDigits, locale) {
            (when (format) {
                "Currency" -> java.text.NumberFormat.getCurrencyInstance(locale).apply {
                    currencyCode?.let { code -> runCatching { currency = java.util.Currency.getInstance(code) } }
                }
                "Percent" -> java.text.NumberFormat.getPercentInstance(locale)
                "Compact" -> java.text.DecimalFormat("#,##0.##", java.text.DecimalFormatSymbols.getInstance(locale))
                else -> java.text.NumberFormat.getNumberInstance(locale)
            }).apply {
                minimumFractionDigits?.let { this.minimumFractionDigits = it }
                maximumFractionDigits?.let { this.maximumFractionDigits = it }
            }
        }
        androidx.compose.runtime.remember(value, numberFormat, format) {
            if (format == "Compact") {
                val magnitude = kotlin.math.abs(value)
                val (scale, suffix) = when {
                    magnitude >= 1_000_000_000.0 -> 1_000_000_000.0 to "B"
                    magnitude >= 1_000_000.0 -> 1_000_000.0 to "M"
                    magnitude >= 1_000.0 -> 1_000.0 to "K"
                    else -> 1.0 to ""
                }
                numberFormat.format(value / scale) + suffix
            } else numberFormat.format(value)
        }
    }
    val digitGlyphs = androidx.compose.runtime.remember(locale) {
        val zeroDigit = java.text.DecimalFormatSymbols.getInstance(locale).zeroDigit
        (0..9).map { digit -> (zeroDigit.code + digit).toChar().toString() }
    }
    val history = androidx.compose.runtime.remember { NexaNumberFlowHistory(value) }
    val increasing = androidx.compose.runtime.remember(value) { history.direction(value) }
    androidx.compose.runtime.SideEffect { history.commit(value) }
    val resolvedSize = fontSize
    val resolvedTextShadow = textShadow?.let { shadow ->
        with(androidx.compose.ui.platform.LocalDensity.current) {
            androidx.compose.ui.graphics.Shadow(
                color = shadow.color,
                offset = androidx.compose.ui.geometry.Offset(shadow.x.toPx(), shadow.y.toPx()),
                blurRadius = shadow.radius.toPx(),
            )
        }
    }
    val lineHeightPx = with(androidx.compose.ui.platform.LocalDensity.current) {
        (if (lineHeight == androidx.compose.ui.unit.TextUnit.Unspecified) resolvedSize * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f else lineHeight).toDp()
    }
    val digitPositions = androidx.compose.runtime.remember(formatted) {
        IntArray(formatted.length).also { positions ->
            var digitPosition = 0
            for (index in formatted.indices.reversed()) {
                if (formatted[index].digitToIntOrNull() != null) positions[index] = digitPosition++
            }
        }
    }
    val rowModifier = modifier.semantics(mergeDescendants = true) { contentDescription = formatted }
    if (selectable) {
        androidx.compose.foundation.text.selection.SelectionContainer {
            NexaNumberFlowDigits(
                formatted, digitPositions, digitGlyphs, increasing, animationSpec, lineHeightPx,
                rowModifier, color, resolvedSize, fontWeight, fontFamily, textAlign, maxLines,
                lineHeight, letterSpacing, strikethrough, italic, underline, resolvedTextShadow,
            )
        }
    } else {
        NexaNumberFlowDigits(
            formatted, digitPositions, digitGlyphs, increasing, animationSpec, lineHeightPx,
            rowModifier, color, resolvedSize, fontWeight, fontFamily, textAlign, maxLines,
            lineHeight, letterSpacing, strikethrough, italic, underline, resolvedTextShadow,
        )
    }
}

@androidx.compose.runtime.Composable
private fun NexaNumberFlowDigits(
    formatted: String,
    digitPositions: IntArray,
    digitGlyphs: List<String>,
    increasing: Boolean,
    animationSpec: androidx.compose.animation.core.FiniteAnimationSpec<androidx.compose.ui.unit.IntOffset>,
    lineHeightPx: androidx.compose.ui.unit.Dp,
    modifier: androidx.compose.ui.Modifier,
    color: androidx.compose.ui.graphics.Color,
    fontSize: androidx.compose.ui.unit.TextUnit,
    fontWeight: androidx.compose.ui.text.font.FontWeight?,
    fontFamily: androidx.compose.ui.text.font.FontFamily?,
    textAlign: androidx.compose.ui.text.style.TextAlign?,
    maxLines: Int,
    lineHeight: androidx.compose.ui.unit.TextUnit,
    letterSpacing: androidx.compose.ui.unit.TextUnit,
    strikethrough: Boolean,
    italic: Boolean,
    underline: Boolean,
    textShadow: androidx.compose.ui.graphics.Shadow?,
) {
    androidx.compose.foundation.layout.Row(
        modifier = modifier,
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    ) {
        formatted.forEachIndexed { index, char ->
            val digit = char.digitToIntOrNull()
            val tokenKey = if (digit == null) "literal:$index:$char" else "digit:${digitPositions[index]}"
            androidx.compose.runtime.key(tokenKey) {
                if (digit == null) {
                    androidx.compose.material3.Text(
                        text = char.toString(), color = color, fontSize = fontSize,
                        fontWeight = fontWeight, fontFamily = fontFamily, textAlign = textAlign,
                        maxLines = maxLines,
                        lineHeight = lineHeight, letterSpacing = letterSpacing,
                        fontStyle = if (italic) androidx.compose.ui.text.font.FontStyle.Italic else null,
                        textDecoration = when {
                            underline && strikethrough -> androidx.compose.ui.text.style.TextDecoration.Underline + androidx.compose.ui.text.style.TextDecoration.LineThrough
                            underline -> androidx.compose.ui.text.style.TextDecoration.Underline
                            strikethrough -> androidx.compose.ui.text.style.TextDecoration.LineThrough
                            else -> null
                        },
                        style = androidx.compose.ui.text.TextStyle(shadow = textShadow),
                        modifier = androidx.compose.ui.Modifier.clearAndSetSemantics {},
                    )
                } else {
                    androidx.compose.animation.AnimatedContent(
                    targetState = digit,
                    transitionSpec = {
                        val enterOffset: (Int) -> Int = if (increasing) { height -> height } else { height -> -height }
                        val exitOffset: (Int) -> Int = if (increasing) { height -> -height } else { height -> height }
                        (androidx.compose.animation.slideInVertically(animationSpec, enterOffset) + androidx.compose.animation.fadeIn()) togetherWith
                            (androidx.compose.animation.slideOutVertically(animationSpec, exitOffset) + androidx.compose.animation.fadeOut())
                    },
                    label = "NexaNumberFlowDigit${digitPositions[index]}",
                ) { target ->
                    androidx.compose.material3.Text(
                        text = digitGlyphs[target], color = color, fontSize = fontSize,
                        fontWeight = fontWeight, fontFamily = fontFamily, textAlign = textAlign,
                        maxLines = maxLines,
                        lineHeight = lineHeight, letterSpacing = letterSpacing,
                        fontStyle = if (italic) androidx.compose.ui.text.font.FontStyle.Italic else null,
                        textDecoration = when {
                            underline && strikethrough -> androidx.compose.ui.text.style.TextDecoration.Underline + androidx.compose.ui.text.style.TextDecoration.LineThrough
                            underline -> androidx.compose.ui.text.style.TextDecoration.Underline
                            strikethrough -> androidx.compose.ui.text.style.TextDecoration.LineThrough
                            else -> null
                        },
                        style = androidx.compose.ui.text.TextStyle(shadow = textShadow),
                        modifier = androidx.compose.ui.Modifier.height(lineHeightPx).clearAndSetSemantics {},
                    )
                }
            }
        }
    }
}
}
