package __NEXA_PACKAGE__

import android.graphics.Typeface
import android.os.SystemClock
import androidx.activity.compose.BackHandler
import android.content.Context
import android.net.Uri
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.interaction.collectIsDraggedAsState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.pager.VerticalPager
import androidx.compose.foundation.pager.PagerDefaults
import androidx.compose.foundation.pager.PagerSnapDistance
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredHeight
import androidx.compose.foundation.layout.requiredWidth
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListItemInfo
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.LazyStaggeredGridState
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Badge
import androidx.compose.material3.Button
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.TextButton
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material.icons.Icons
__NEXA_SHARED_ICON_AUTO_MIRRORED_IMPORTS__
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Slider
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.key
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import kotlinx.coroutines.flow.collect
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.size
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.input.pointer.util.addPointerInputChange
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.ViewConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role as SemanticsRole
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.zIndex
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import kotlinx.coroutines.flow.distinctUntilChanged
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import kotlinx.coroutines.launch

private data class NexaDevDimension(val kind: String, val value: Float = 0f)

private fun nexaDevDimension(raw: Any?): NexaDevDimension? = when (raw) {
    is Number -> NexaDevDimension("fixed", raw.toFloat())
    is String -> if (raw == "Fill") NexaDevDimension("fill") else null
    is JSONObject -> {
        val key = raw.keys().asSequence().firstOrNull() ?: return null
        val value = raw.opt(key)
        when (key) {
            "Fixed" -> NexaDevDimension("fixed", (value as? Number)?.toFloat() ?: 0f)
            "ParentFraction" -> NexaDevDimension("parent", (value as? Number)?.toFloat() ?: 0f)
            "ScreenWidthFraction" -> NexaDevDimension("screen_width", (value as? Number)?.toFloat() ?: 0f)
            "ScreenHeightFraction" -> NexaDevDimension("screen_height", (value as? Number)?.toFloat() ?: 0f)
            else -> null
        }
    }
    else -> null
}


internal object NexaDevFontRegistry {
    private val families = java.util.concurrent.ConcurrentHashMap<String, androidx.compose.ui.text.font.FontFamily>()
    private val developmentFonts = java.util.concurrent.ConcurrentHashMap<String, File>()

    fun install(context: Context, assets: JSONObject?) {
        val directory = File(context.cacheDir, "nexa-dev-fonts")
        if (assets == null || assets.length() == 0) {
            families.clear()
            developmentFonts.values.forEach { font -> font.delete() }
            developmentFonts.clear()
            return
        }
        if (!directory.exists() && !directory.mkdirs()) return

        val activeFamilies = HashSet<String>(assets.length())
        val iterator = assets.keys()
        while (iterator.hasNext()) {
            val familyName = iterator.next()
            if (familyName.isEmpty() || familyName.any {
                    !(it in 'a'..'z' || it in 'A'..'Z' || it in '0'..'9' || it == '_' || it == '-')
                }) {
                continue
            }
            val encoded = assets.optJSONArray(familyName) ?: continue
            if (encoded.length() == 0) continue
            val bytes = ByteArray(encoded.length()) { index -> encoded.optInt(index).toByte() }
            val fontFile = File(directory, "$familyName.ttf")
            runCatching {
                if (!fontFile.isFile || fontFile.length() != bytes.size.toLong() || !fontFile.readBytes().contentEquals(bytes)) {
                    fontFile.writeBytes(bytes)
                }
                developmentFonts[familyName] = fontFile
                activeFamilies += familyName
            }.onFailure { error ->
                android.util.Log.w("NexaDevRuntime", "Could not install custom icon font $familyName", error)
            }
        }
        developmentFonts.keys.toList().forEach { familyName ->
            if (familyName !in activeFamilies) {
                developmentFonts.remove(familyName)?.delete()
            }
        }
        families.clear()
    }

    fun resolve(familyName: String): androidx.compose.ui.text.font.FontFamily? {
        val fontFile = developmentFonts[familyName] ?: return null
        return families.computeIfAbsent(familyName) {
            runCatching { Typeface.createFromFile(fontFile) }
                .getOrNull()
                ?.let { face -> androidx.compose.ui.text.font.FontFamily(face) }
                ?: androidx.compose.ui.text.font.FontFamily.Default
        }
    }
}

@Composable
private fun nexaDevIconFontFamily(name: String): androidx.compose.ui.text.font.FontFamily =
    NexaDevFontRegistry.resolve(name) ?: nexaDevFontFamily(name, FontWeight.Normal)

@Composable
private fun nexaDevFontFamily(name: String, weight: FontWeight): androidx.compose.ui.text.font.FontFamily {
    return remember(name.lowercase(), weight) {
        when (name.lowercase() to weight) {
__NEXA_ANDROID_FONT_FAMILY_CASES__
            else -> androidx.compose.ui.text.font.FontFamily.Default
        }
    }
}

private const val nexaDevDefaultBodyFontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__
private const val nexaDevDefaultMaterialIconSize = __NEXA_DEFAULT_MATERIAL_ICON_SIZE__
private const val nexaDevDefaultLineHeightMultiplier = __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f
private const val nexaDevFormRowHorizontalInset = __NEXA_FORM_ROW_HORIZONTAL_INSET__
private const val nexaDevFormRowMinHeight = __NEXA_ANDROID_FORM_ROW_MIN_HEIGHT__
private const val nexaDevFormLargeTitleTopPadding = __NEXA_FORM_LARGE_TITLE_TOP_PADDING__

@Composable
private fun NexaDevObserveFastList(
    listState: androidx.compose.foundation.gestures.ScrollableState,
    count: Int,
    settledOnly: Boolean = false,
    readLeadingPosition: () -> Int,
    endIsVisible: () -> Boolean,
    onPosition: ((Int) -> Unit)?,
    onEndReached: (() -> Unit)?,
) {
    if (onPosition != null) {
        val currentPosition = rememberUpdatedState(onPosition)
        val currentReadLeading = rememberUpdatedState(readLeadingPosition)
        LaunchedEffect(listState, settledOnly) {
            var lastVisiblePosition = -1
            snapshotFlow {
                if (settledOnly && listState.isScrollInProgress) -1 else currentReadLeading.value()
            }.collect { position ->
                if (position >= 0 && position != lastVisiblePosition) {
                    lastVisiblePosition = position
                    currentPosition.value(position)
                }
            }
        }
    }
    if (onEndReached != null && count > 0) {
        val currentEndReached = rememberUpdatedState(onEndReached)
        val currentEndIsVisible = rememberUpdatedState(endIsVisible)
        var wasAtEnd by rememberSaveable(count) { mutableStateOf(false) }
        LaunchedEffect(listState, count, settledOnly) {
            snapshotFlow {
                (!settledOnly || !listState.isScrollInProgress) && currentEndIsVisible.value()
            }.distinctUntilChanged().collect { reachedEnd ->
                if (reachedEnd) {
                    if (!wasAtEnd) currentEndReached.value()
                    wasAtEnd = true
                } else {
                    wasAtEnd = false
                }
            }
        }
    }
}

private fun nexaDevLeadingVisibleDataItemIndex(
    visibleItems: List<LazyListItemInfo>,
    fallback: Int,
    stickyHeaderOffset: Int,
    viewportStartOffset: Int,
    viewportEndOffset: Int,
): Int {
    return nexaLeadingVisibleDataItemIndex(
        visibleItems, fallback, stickyHeaderOffset,
        viewportStartOffset, viewportEndOffset,
        itemIndex = { it.index }, itemOffset = { it.offset }, itemSize = { it.size },
    )
}

private fun nexaDevSharedMaterialIcon(name: String) = when (name) {
__NEXA_SHARED_ICON_MATERIAL_CASES__
    else -> Icons.Filled.Star
}

private fun nexaDevIconAccessibilityLabel(name: String): String = name
    .split('_')
    .filter(String::isNotEmpty)
    .joinToString(" ") { part -> part.replaceFirstChar(Char::uppercase) }
    .ifBlank { "Icon" }

@Composable
private fun nexaDevIconSpec(
    rawSelection: Any?,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    defaultSize: androidx.compose.ui.unit.Dp? = null,
): NexaIconSpec? {
    if (rawSelection == null || rawSelection == JSONObject.NULL) return null
    val initial = rawSelection as? JSONObject ?: return null
    val unwrapped = if (
        !initial.has("source") &&
        !initial.has("shared") &&
        !initial.has("FontGlyph") &&
        !initial.has("font_glyph") &&
        !initial.has("System") &&
        !initial.has("system") &&
        !initial.has("Icon") &&
        !initial.has("IconWith")
    ) {
        store.evaluatePresented(initial, locals, scope) as? JSONObject ?: return null
    } else {
        initial
    }
    if (unwrapped.has("IconWith")) {
        val withSpec = unwrapped.optJSONObject("IconWith") ?: return null
        val base = nexaDevIconSpec(withSpec.opt("base"), store, locals, scope, defaultSize) ?: return null
        val description = withSpec.optString("description").takeIf { withSpec.has("description") && !withSpec.isNull("description") }
        val size = if (withSpec.has("size") && !withSpec.isNull("size")) withSpec.optDouble("size").dp else null
        val colorValue = withSpec.optJSONObject("color")
        val dynamicColor = colorValue?.opt("Dynamic")
        val tint = if (dynamicColor != null && dynamicColor != JSONObject.NULL) {
            nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicColor, locals, scope)))
        } else {
            nexaDevColor(colorValue, isSystemInDarkTheme())
        }
        val layerColors = nexaDevIconLayerColors(withSpec.optJSONArray("layer_colors"), store, locals, scope)
        val resolvedTint = if (layerColors.isNotEmpty()) null else (tint ?: base.tint)
        val resolvedLayerColors = when {
            layerColors.isNotEmpty() -> layerColors
            tint != null -> emptyList()
            else -> base.layerColors
        }
        return base.copy(
            description = description ?: base.description,
            size = size ?: base.size,
            tint = resolvedTint,
            layerColors = resolvedLayerColors,
        )
    }
    val selection = unwrapped.optJSONObject("Icon") ?: unwrapped
    val description = selection.optString("description").takeIf { selection.has("description") && it != "null" }
    val size = if (selection.has("size") && !selection.isNull("size")) {
        selection.optDouble("size").dp
    } else {
        defaultSize
    }
    val source = selection.optJSONObject("source") ?: selection
    val colorValue = selection.optJSONObject("color")
    val dynamicColor = colorValue?.opt("Dynamic")
    val tint = if (dynamicColor != null && dynamicColor != JSONObject.NULL) {
        nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicColor, locals, scope)))
    } else {
        nexaDevColor(colorValue, isSystemInDarkTheme())
    }
    val layerColors = nexaDevIconLayerColors(selection.optJSONArray("layer_colors"), store, locals, scope)
    val fontGlyph = source.optJSONObject("FontGlyph") ?: source.optJSONObject("font_glyph")
    if (fontGlyph != null) {
        val family = fontGlyph.optString("font_family")
        val glyph = fontGlyph.optInt("glyph", -1)
        if (family.isNotEmpty() && glyph >= 0) {
            return NexaIconSpec(
                glyph = glyph,
                fontFamily = nexaDevIconFontFamily(family),
                description = description,
                size = size,
                tint = tint,
                preservesColor = true,
                layers = nexaDevIconLayers(fontGlyph.optJSONArray("layers")),
                layerColors = layerColors,
            )
        }
        return null
    }
    val system = source.optJSONObject("System") ?: source.optJSONObject("system") ?: source
    val vector = if (system.has("shared")) {
        nexaDevSharedMaterialIcon(system.optString("shared"))
    } else {
        null
    } ?: return null
    return if (system.optString("shared") == "search") {
        NexaIconSpec(vector = vector, description = description, size = size, tint = tint, layerColors = layerColors, symbolScale = __NEXA_MATERIAL_SEARCH_SIZE_SCALE__f)
    } else {
        NexaIconSpec(vector = vector, description = description, size = size, tint = tint, layerColors = layerColors)
    }
}

@Composable
private fun nexaDevIconLayerColors(
    values: JSONArray?,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
): List<Color> {
    values ?: return emptyList()
    return buildList {
        for (index in 0 until values.length()) {
            val colorValue = values.optJSONObject(index) ?: continue
            val dynamicColor = colorValue.opt("Dynamic")
            val resolved = if (dynamicColor != null && dynamicColor != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicColor, locals, scope)))
            } else {
                nexaDevColor(colorValue, isSystemInDarkTheme())
            }
            if (resolved != null) {
                add(resolved)
            }
        }
    }
}

private fun nexaDevIconLayers(values: JSONArray?): List<NexaIconLayer> {
    values ?: return emptyList()
    return buildList {
        for (index in 0 until values.length()) {
            val layer = values.optJSONObject(index) ?: continue
            val rgba = layer.optJSONArray("rgba") ?: continue
            if (rgba.length() != 4) continue
            add(
                NexaIconLayer(
                    glyph = layer.optInt("glyph", -1),
                    color = Color(
                        red = rgba.optInt(0, 0) / 255f,
                        green = rgba.optInt(1, 0) / 255f,
                        blue = rgba.optInt(2, 0) / 255f,
                        alpha = rgba.optInt(3, 255) / 255f,
                    ),
                    tintable = layer.optBoolean("tintable", false),
                ),
            )
        }
    }.filter { it.glyph >= 0 }
}

internal data class NexaDevContentSlot(
    val nodes: JSONArray,
    val scope: String,
    val parameters: Map<String, Any>,
)

internal val LocalNexaDevContentSlot = staticCompositionLocalOf<NexaDevContentSlot?> { null }

internal fun nexaDevNodeObject(rawNode: Any?): JSONObject = when (rawNode) {
    is JSONObject -> rawNode
    is String -> JSONObject().put(rawNode, JSONObject.NULL)
    else -> JSONObject()
}

internal enum class NexaDevButtonPresentation {
    Default,
    FormRow,
    ConfirmationAction,
}

private fun JSONObject.nexaDevTransition(): String =
    opt("transition")?.takeUnless { it === JSONObject.NULL }?.toString().orEmpty()

private fun nexaDevContentTransform(
    transition: String,
    floatAnimation: androidx.compose.animation.core.FiniteAnimationSpec<Float>?,
    offsetAnimation: androidx.compose.animation.core.FiniteAnimationSpec<androidx.compose.ui.unit.IntOffset>?,
): androidx.compose.animation.ContentTransform {
    val enter = when (transition) {
        "Fade" -> floatAnimation?.let { fadeIn(animationSpec = it) } ?: fadeIn()
        "FadeSlideFromTop" -> {
            val fade = floatAnimation?.let { fadeIn(animationSpec = it) } ?: fadeIn()
            val slide = offsetAnimation?.let { spec ->
                slideInVertically(animationSpec = spec) { height -> -height }
            } ?: slideInVertically { height -> -height }
            fade + slide
        }
        "SlideFromBottom" -> offsetAnimation?.let { spec ->
            slideInVertically(animationSpec = spec) { height -> height }
        } ?: slideInVertically { height -> height }
        "SlideFromLeft" -> offsetAnimation?.let { spec ->
            slideInHorizontally(animationSpec = spec) { width -> -width }
        } ?: slideInHorizontally { width -> -width }
        "SlideFromRight" -> offsetAnimation?.let { spec ->
            slideInHorizontally(animationSpec = spec) { width -> width }
        } ?: slideInHorizontally { width -> width }
        "Scale" -> floatAnimation?.let { scaleIn(initialScale = 0f, animationSpec = it) } ?: scaleIn(initialScale = 0f)
        else -> fadeIn()
    }
    val exit = when (transition) {
        "Fade" -> floatAnimation?.let { fadeOut(animationSpec = it) } ?: fadeOut()
        "FadeSlideFromTop" -> {
            val fade = floatAnimation?.let { fadeOut(animationSpec = it) } ?: fadeOut()
            val slide = offsetAnimation?.let { spec ->
                slideOutVertically(animationSpec = spec) { height -> -height }
            } ?: slideOutVertically { height -> -height }
            fade + slide
        }
        "SlideFromBottom" -> offsetAnimation?.let { spec ->
            slideOutVertically(animationSpec = spec) { height -> height }
        } ?: slideOutVertically { height -> height }
        "SlideFromLeft" -> offsetAnimation?.let { spec ->
            slideOutHorizontally(animationSpec = spec) { width -> width }
        } ?: slideOutHorizontally { width -> width }
        "SlideFromRight" -> offsetAnimation?.let { spec ->
            slideOutHorizontally(animationSpec = spec) { width -> -width }
        } ?: slideOutHorizontally { width -> -width }
        "Scale" -> floatAnimation?.let { scaleOut(targetScale = 0f, animationSpec = it) } ?: scaleOut(targetScale = 0f)
        else -> fadeOut()
    }
    return enter togetherWith exit
}

internal fun JSONObject.nexaDevOptionalString(name: String): String? =
    opt(name)?.takeUnless { it === JSONObject.NULL }?.toString()

@Composable
internal fun NexaDevStateStore.evaluatePresented(
    raw: Any?,
    locals: Map<String, Any>,
    scope: String,
): Any {
    val animatedStates = linkedSetOf<String>()
    collectPresentedAnimationStates(raw, animatedStates)
    if (animatedStates.isEmpty()) return evaluate(raw, locals, scope)
    val presentedLocals = locals.toMutableMap()
    for (name in animatedStates) {
        presentedLocals["__nexa_presented_animated:$name"] = animatedFloatState(name, scope).toDouble()
    }
    return evaluate(raw, presentedLocals, scope)
}

private fun collectPresentedAnimationStates(raw: Any?, names: MutableSet<String>) {
    when (raw) {
        is JSONObject -> {
            val animated = raw.optJSONArray("AnimatedState")
            if (animated != null) {
                animated.optString(0).takeIf(String::isNotEmpty)?.let(names::add)
                return
            }
            val keys = raw.keys()
            while (keys.hasNext()) collectPresentedAnimationStates(raw.opt(keys.next()), names)
        }
        is JSONArray -> for (index in 0 until raw.length()) {
            collectPresentedAnimationStates(raw.opt(index), names)
        }
    }
}

@Composable
internal fun NexaDevStateStore.animatedFloatState(name: String, scope: String): Float =
    animationState(name, scope).value

@Composable
private fun NexaDevStateStore.animationState(
    name: String,
    scope: String,
): androidx.compose.runtime.State<Float> {
    val target = (state(name, scope) as? Number)?.toFloat() ?: 0f
    return androidx.compose.animation.core.animateFloatAsState(
        targetValue = target,
        animationSpec = animationSpec(name, scope),
        finishedListener = { clearAnimationSpec(name, scope) },
        label = name,
    )
}

private fun shapeDimension(value: Any?): Float =
    ((value as? Number)?.toDouble() ?: 0.0).coerceAtLeast(0.0).toFloat()

@Composable
private fun NexaDevStateStore.shapeDimensionReader(
    expression: Any,
    locals: Map<String, Any>,
    scope: String,
): NexaShapeDimension {
    val animationNames = remember(expression) {
        linkedSetOf<String>().also { collectPresentedAnimationStates(expression, it) }
    }
    if (animationNames.isEmpty()) {
        return NexaShapeDimension { shapeDimension(evaluate(expression, locals, scope)) }
    }
    val presentationStates = animationNames.map { name ->
        key(name) { "__nexa_presented_animated:$name" to animationState(name, scope) }
    }
    // Install animation targets in composition. Read their presented values
    // together in measurement, using the same evaluator as ordinary reads.
    return NexaShapeDimension {
        val presentedLocals = locals.toMutableMap()
        for ((name, state) in presentationStates) {
            presentedLocals[name] = state.value.toDouble()
        }
        shapeDimension(evaluate(expression, presentedLocals, scope))
    }
}

internal fun openNexaUrl(context: Context, value: String) {
    val uri = Uri.parse(value)
    when (uri.scheme) {
        "https", "http", "mailto", "tel" -> runCatching { nexaLaunchLink(context, uri) }
    }
}

private fun nexaDevHasDialogMessage(expression: Any?): Boolean {
    val encoded = expression as? JSONObject ?: return true
    encoded.optJSONObject("LocalizedText")?.let { localized ->
        return nexaDevHasDialogMessage(localized.opt("value"))
    }
    if (encoded.has("String")) return encoded.optString("String").isNotEmpty()
    return true
}

@Composable
internal fun NexaDevNodeList(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    parameters: Map<String, Any> = emptyMap(),
    scope: String = "app",
    modifier: Modifier = Modifier,
) {
    val locals = store.locals(scope, parameters).toMutableMap()
    val metrics = LocalNexaWindowMetrics.current
    val direction = LocalLayoutDirection.current
    locals["__nexa_screen_width"] = metrics.viewportWidthDp.toDouble()
    locals["__nexa_screen_height"] = metrics.viewportHeightDp.toDouble()
    locals["__nexa_system_inset_top"] = metrics.systemInset(NexaSafeAreaEdge.Top, direction).value.toDouble()
    locals["__nexa_system_inset_bottom"] = metrics.systemInset(NexaSafeAreaEdge.Bottom, direction).value.toDouble()
    locals["__nexa_system_inset_leading"] = metrics.systemInset(NexaSafeAreaEdge.Leading, direction).value.toDouble()
    locals["__nexa_system_inset_trailing"] = metrics.systemInset(NexaSafeAreaEdge.Trailing, direction).value.toDouble()
    LocalNexaDevNavController.current?.let { locals["__nexa_navigation_controller"] = it }
    when (nodes.length()) {
        0 -> Unit
        1 -> NexaDevNode(nexaDevNodeObject(nodes.opt(0)), module, store, locals, scope, modifier)
        else -> NexaColumnPrimitive(
            modifier = modifier,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            RenderColumnChildren(nodes, module, store, locals, scope)
        }
    }
}

private fun nexaDevContainsScrollable(nodes: JSONArray): Boolean {
    for (index in 0 until nodes.length()) {
        val node = nexaDevNodeObject(nodes.opt(index))
        val kind = node.keys().asSequence().firstOrNull() ?: continue
        if (kind == "ScrollView" || kind == "FastList" || kind == "KeyboardAware" || kind == "Form") return true
        val fields = node.optJSONObject(kind) ?: continue
        fields.optJSONArray("children")?.let { if (nexaDevContainsScrollable(it)) return true }
        fields.optJSONArray("then_body")?.let { if (nexaDevContainsScrollable(it)) return true }
        fields.optJSONArray("else_body")?.let { if (nexaDevContainsScrollable(it)) return true }
        fields.optJSONArray("sidebar")?.let { if (nexaDevContainsScrollable(it)) return true }
        fields.optJSONArray("detail")?.let { if (nexaDevContainsScrollable(it)) return true }
        fields.optJSONArray("tabs")?.let { tabs ->
            for (tabIndex in 0 until tabs.length()) {
                tabs.optJSONObject(tabIndex)?.optJSONArray("children")?.let {
                    if (nexaDevContainsScrollable(it)) return true
                }
            }
        }
        fields.optJSONArray("pages")?.let { pages ->
            for (pageIndex in 0 until pages.length()) {
                pages.optJSONArray(pageIndex)?.let {
                    if (nexaDevContainsScrollable(it)) return true
                }
            }
        }
        fields.optJSONArray("cases")?.let { cases ->
            for (caseIndex in 0 until cases.length()) {
                cases.optJSONObject(caseIndex)?.optJSONArray("body")?.let {
                    if (nexaDevContainsScrollable(it)) return true
                }
            }
        }
    }
    return false
}

private fun nexaDevHasOnlyFixedWidthLayoutChildren(nodes: JSONArray): Boolean {
    if (nodes.length() == 0) return false
    return (0 until nodes.length()).all { index ->
        val child = nexaDevNodeObject(nodes.opt(index))
        if (!child.has("Layout")) {
            false
        } else {
            val childFields = child.optJSONObject("Layout") ?: return@all false
            val childStyle = childFields.optJSONObject("style") ?: return@all false
            !childStyle.isNull("width")
        }
    }
}

@Composable
internal fun ColumnScope.RenderColumnChildren(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    for (index in 0 until nodes.length()) {
        androidx.compose.runtime.key(index) {
            val child = nexaDevNodeObject(nodes.opt(index))
            val conditional = child.optJSONObject("If")
            val shouldRenderChild = if (
                conditional != null && conditional.nexaDevTransition().isEmpty()
            ) {
                val condition = store.evaluatePresented(
                    conditional.opt("condition"),
                    locals,
                    scope,
                ) as? Boolean ?: false
                val branch = conditional.optJSONArray(
                    if (condition) "then_body" else "else_body",
                )
                branch != null && branch.length() > 0
            } else {
                true
            }
            if (shouldRenderChild) {
                val modifier = when {
                    nexaDevAcceptsSpacerWeight(child) -> Modifier.weight(1f)
                    child.has("NavigationStack") -> Modifier.fillMaxSize()
                    else -> Modifier
                }
                if (child.has("ContentUnavailable")) {
                    Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
                        NexaDevNode(child, module, store, locals, scope, Modifier.fillMaxSize())
                    }
                } else {
                    NexaDevNode(child, module, store, locals, scope, modifier)
                }
            }
        }
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
internal fun NexaDevNode(
    node: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier = Modifier,
    formShowDividers: Boolean = true,
    formContentPadding: PaddingValues? = null,
    buttonPresentation: NexaDevButtonPresentation = NexaDevButtonPresentation.Default,
) {
    val kind = node.keys().asSequence().firstOrNull() ?: return
    val fields = node.optJSONObject(kind) ?: JSONObject()
    when (kind) {
        "ViewInsets" -> {
            val padding = fields.optJSONObject("padding") ?: JSONObject()
            val margin = fields.optJSONObject("margin") ?: JSONObject()
            @Composable
            fun inset(values: JSONObject, name: String): Dp {
                val raw = values.opt(name)?.takeIf { it != JSONObject.NULL } ?: return 0.dp
                return (((store.evaluatePresented(raw, locals, scope) as? Number)?.toFloat() ?: 0f)
                    .coerceAtLeast(0f)).dp
            }
            val child = fields.optJSONObject("child") ?: return
            val outerModifier = modifier.padding(
                start = inset(margin, "leading"),
                top = inset(margin, "top"),
                end = inset(margin, "trailing"),
                bottom = inset(margin, "bottom"),
            )
            if (child.has("Form")) {
                NexaDevNode(
                    child,
                    module,
                    store,
                    locals,
                    scope,
                    modifier = outerModifier,
                    formContentPadding = PaddingValues(
                        start = inset(padding, "leading"),
                        top = inset(padding, "top"),
                        end = inset(padding, "trailing"),
                        bottom = inset(padding, "bottom"),
                    ),
                )
                return
            }
            val scrollContent = child.has("ScrollView") || child.has("FastList") || child.has("KeyboardAware") || child.has("RefreshControl")
            if (scrollContent) {
                androidx.compose.runtime.CompositionLocalProvider(
                    LocalNexaScrollContentPadding provides PaddingValues(
                        start = inset(padding, "leading"),
                        top = inset(padding, "top"),
                        end = inset(padding, "trailing"),
                        bottom = inset(padding, "bottom"),
                    ),
                ) {
                    NexaDevNode(
                        child,
                        module,
                        store,
                        locals,
                        scope,
                        modifier = outerModifier,
                    )
                }
                return
            }
            NexaDevNode(
                child,
                module,
                store,
                locals,
                scope,
                modifier = outerModifier
                    .padding(
                        start = inset(padding, "leading"),
                        top = inset(padding, "top"),
                        end = inset(padding, "trailing"),
                        bottom = inset(padding, "bottom"),
                    ),
            )
        }
        "SafeAreaInset" -> {
            val plan = fields.optJSONObject("plan") ?: JSONObject()
            val edge = when (plan.optString("edge")) {
                "Top" -> NexaSafeAreaEdge.Top
                "Leading" -> NexaSafeAreaEdge.Leading
                "Trailing" -> NexaSafeAreaEdge.Trailing
                else -> NexaSafeAreaEdge.Bottom
            }
            val owner = plan.opt("scroll_owner")
            val accommodatesEndpoint =
                (owner is JSONObject && owner.has("Unique")) || owner == "RouteHost"
            NexaSafeAreaInsetPrimitive(
                edge = edge,
                accommodatesEndpoint = accommodatesEndpoint,
                content = {
                    NexaDevNodeList(
                        fields.optJSONArray("children") ?: JSONArray(),
                        module,
                        store,
                        parameters = locals,
                        scope = scope,
                    )
                },
                attachedContent = {
                    NexaDevNodeList(
                        fields.optJSONArray("content") ?: JSONArray(),
                        module,
                        store,
                        parameters = locals,
                        scope = "$scope/safe-area-inset",
                    )
                },
            )
        }
        "ScrollHeader" -> {
            val plan = fields.optJSONObject("plan") ?: JSONObject()
            val header = fields.optJSONArray("header") ?: JSONArray()
            val children = fields.optJSONArray("children") ?: JSONArray()
            NexaAdaptiveScrollHeaderPrimitive(
                behavior = plan.optString("behavior", "Pinned"),
                collapsedHeight = plan.optDouble("collapsed_height", 56.0).dp,
                header = {
                    NexaDevNodeList(
                        header,
                        module,
                        store,
                        parameters = locals,
                        scope = "$scope/scroll-header",
                    )
                },
                content = {
                    NexaDevNodeList(
                        children,
                        module,
                        store,
                        parameters = locals,
                        scope = scope,
                    )
                },
            )
        }
        "Appearance" -> {
            val mode = store.stringify(store.evaluatePresented(fields.opt("mode") ?: "system", locals, scope))
            NexaAppearancePrimitive(mode = mode) {
                NexaDevNodeList(
                    fields.optJSONArray("children") ?: JSONArray(),
                    module,
                    store,
                    parameters = locals,
                    scope = scope,
                )
            }
        }
        "Form" -> {
            val showDividers = fields.optBoolean("show_dividers", true)
            NexaFormPrimitive(
                background = nexaDevColor(fields.optJSONObject("background"), isSystemInDarkTheme()),
                modifier = modifier,
                contentPadding = formContentPadding ?: PaddingValues(0.dp),
            ) {
                val children = fields.optJSONArray("children") ?: JSONArray()
                for (index in 0 until children.length()) {
                    androidx.compose.runtime.key(index) {
                        val child = nexaDevNodeObject(children.opt(index))
                        if (child.has("FormSection")) {
                            NexaDevNode(
                                child,
                                module,
                                store,
                                locals,
                                scope,
                                formShowDividers = showDividers,
                            )
                        } else {
                            NexaFormRowPrimitive(
                                minHeight = nexaDevFormRowMinHeight.toFloat(),
                                horizontalInset = nexaDevFormRowHorizontalInset.toFloat(),
                            ) {
                                NexaDevNode(
                                    child, module, store, locals, scope,
                                    buttonPresentation = NexaDevButtonPresentation.FormRow,
                                )
                            }
                            val next = children.optJSONObject(index + 1)
                            if (showDividers && next != null && !next.has("FormSection")) {
                                NexaFormDividerPrimitive()
                            }
                        }
                    }
                }
            }
        }
        "FormSection" -> {
            val rawShowDividers = fields.opt("show_dividers")
            val showDividers = if (rawShowDividers == null || rawShowDividers == JSONObject.NULL) {
                formShowDividers
            } else {
                rawShowDividers as? Boolean ?: formShowDividers
            }
            NexaFormSectionPrimitive(
                title = if (fields.isNull("title")) null else store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope)),
                footer = if (fields.isNull("footer")) null else store.stringify(store.evaluatePresented(fields.opt("footer"), locals, scope)),
                background = nexaDevColor(fields.optJSONObject("background"), isSystemInDarkTheme()),
            ) {
                val children = fields.optJSONArray("children") ?: JSONArray()
                for (index in 0 until children.length()) {
                    androidx.compose.runtime.key(index) {
                        val rowNode = nexaDevNodeObject(children.opt(index))
                        NexaFormRowPrimitive(
                            minHeight = nexaDevFormRowMinHeight.toFloat(),
                            horizontalInset = nexaDevFormRowHorizontalInset.toFloat(),
                        ) {
                            NexaDevNode(
                                rowNode,
                                module,
                                store,
                                locals,
                                scope,
                                buttonPresentation = NexaDevButtonPresentation.FormRow,
                            )
                        }
                        if (showDividers && index + 1 < children.length()) {
                            NexaFormDividerPrimitive()
                        }
                    }
                }
            }
        }
        "Content" -> {
            val slot = LocalNexaDevContentSlot.current ?: return
            NexaDevNodeList(
                slot.nodes,
                module,
                store,
                parameters = slot.parameters,
                scope = slot.scope,
            )
        }
        "ComponentCall" -> {
            val name = fields.optString("name")
            val components = module.optJSONArray("components") ?: JSONArray()
            var component: JSONObject? = null
            for (index in 0 until components.length()) {
                val candidate = components.optJSONObject(index) ?: continue
                if (candidate.optString("name") == name) {
                    component = candidate
                    break
                }
            }
            val resolved = component ?: return
            val parameters = mutableMapOf<String, Any>()
            val navCtrl = LocalNexaDevNavController.current ?: (locals["__nexa_navigation_controller"] as? androidx.navigation.NavHostController)
            if (navCtrl != null) {
                parameters["__nexa_navigation_controller"] = navCtrl
            }
            val navId = locals["__nexa_navigation_stack_identity"]
            if (navId != null) {
                parameters["__nexa_navigation_stack_identity"] = navId
            }
            val entryId = locals["__nexa_navigation_entry_id"]
            if (entryId != null) {
                parameters["__nexa_navigation_entry_id"] = entryId
            }
            val arguments = fields.optJSONArray("arguments") ?: JSONArray()
            for (index in 0 until arguments.length()) {
                val argument = arguments.optJSONArray(index) ?: continue
                val parameter = argument.optString(0)
                if (parameter.isNotEmpty()) {
                    parameters[parameter] = store.evaluatePresented(argument.opt(1), locals, scope)
                }
            }
            val slot = NexaDevContentSlot(
                fields.optJSONArray("children") ?: JSONArray(),
                scope,
                locals,
            )
            val componentRef = fields.opt(NexaDevKeys.COMPONENT_REF)
                ?.takeUnless { it == JSONObject.NULL }
                ?.let { store.evaluatePresented(it, locals, scope) }
                as? NexaDevComponentRef
            val componentInstanceId = remember(name) { java.util.UUID.randomUUID().toString() }
            val componentScope = componentRef?.let {
                "component/$name/ref/${it.identity}"
            } ?: "component/$name/$componentInstanceId"
            componentRef?.attach(componentScope, parameters)
            val latestComponentParameters = rememberUpdatedState(parameters)
            LaunchedEffect(componentScope) {
                val bindings = store.bindings(resolved.optJSONArray("states"), latestComponentParameters.value, componentScope)
                val actions = resolved.optJSONArray("on_appear") ?: JSONArray()
                if (resolved.optBoolean("on_appear_async")) store.performAsync(actions, componentScope, bindings)
                else store.perform(actions, componentScope, bindings)
            }
            DisposableEffect(componentScope) {
                onDispose {
                    store.perform(resolved.optJSONArray("on_disappear") ?: JSONArray(), componentScope, latestComponentParameters.value)
                    store.clearNativeEventSubscriptions(componentScope)
                    store.clearNativeTasks(componentScope)
                    componentRef?.detach()
                }
            }
            CompositionLocalProvider(LocalNexaDevContentSlot provides slot) {
                NexaDevNodeList(
                    resolved.optJSONArray("body") ?: JSONArray(),
                    module,
                    store,
                    parameters = store.bindings(
                        resolved.optJSONArray("states"),
                        parameters,
                        componentScope,
                    ),
                    scope = componentScope,
                )
            }
        }
        "NativeComponentCall" -> {
            val namespace = fields.optString("namespace")
            val name = fields.optString("name")
            val arguments = mutableMapOf<String, Any>()
            val rawArguments = fields.optJSONArray("arguments") ?: JSONArray()
            for (index in 0 until rawArguments.length()) {
                val argument = rawArguments.optJSONArray(index) ?: continue
                val argumentName = argument.optString(0)
                if (argumentName.isNotEmpty()) {
                    arguments[argumentName] = store.evaluatePresented(argument.opt(1), locals, scope)
                }
            }
            val events = mutableMapOf<String, (List<Any>) -> Unit>()
            val handlers = fields.optJSONArray("event_handlers") ?: JSONArray()
            for (index in 0 until handlers.length()) {
                val handler = handlers.optJSONObject(index) ?: continue
                val property = handler.optString("property")
                if (property.isNotEmpty()) {
                    events[property] = store.devNativeEventHandler(
                        handler.optJSONArray("actions") ?: JSONArray(),
                        parameterNames = handler.optJSONArray("parameters") ?: JSONArray(),
                        scope = scope,
                        locals = locals,
                    )
                }
            }
            val children = fields.optJSONArray("children") ?: JSONArray()
            val componentRef = fields.opt(NexaDevKeys.COMPONENT_REF)
                ?.takeIf { it != JSONObject.NULL }
                ?.let { store.evaluatePresented(it, locals, scope) }
            val rendered = NexaDevPluginBridge.renderComponent(namespace, name, arguments, events, componentRef) {
                NexaDevNodeList(children, module, store, parameters = locals, scope = scope)
            }
            if (!rendered) android.util.Log.w("NexaDevRuntime", "Native component is unsupported: $namespace.$name")
        }
        "ScrollView" -> {
            val axis = fields.optString(NexaDevKeys.AXIS, "Vertical").takeIf { it == "Horizontal" } ?: "Vertical"
            val children = fields.optJSONArray("children") ?: JSONArray()
            val componentRef = fields.opt(NexaDevKeys.COMPONENT_REF)
                ?.takeUnless { it == JSONObject.NULL }
                ?.let { store.evaluatePresented(it, locals, scope) as? NexaScrollViewRef }
            val style = fields.optJSONObject(NexaDevKeys.STYLE) ?: JSONObject()
            val background = nexaDevColor(style.optJSONObject("background"), isSystemInDarkTheme())
            var scrollModifier = modifier.then(nexaSafeAreaScrollViewportModifier(axis))
            fun dimension(name: String): NexaDevDimension? = nexaDevDimension(style.opt(name))
            val metrics = LocalNexaWindowMetrics.current
            fun resolve(value: NexaDevDimension?): Dp? = when (value?.kind) {
                "fixed" -> value.value.dp
                "screen_width" -> (metrics.viewportWidthDp * value.value).dp
                "screen_height" -> (metrics.viewportHeightDp * value.value).dp
                else -> null
            }
            val width = dimension("width")
            val height = dimension("height")
            val minWidth = dimension("min_width")
            val maxWidth = dimension("max_width")
            val minHeight = dimension("min_height")
            val maxHeight = dimension("max_height")
            resolve(width)?.let { scrollModifier = scrollModifier.requiredWidth(it) }
            resolve(height)?.let { scrollModifier = scrollModifier.requiredHeight(it) }
            val widthMinimum = resolve(minWidth) ?: if (minWidth?.kind == "fill") 0.dp else Dp.Unspecified
            val widthMaximum = resolve(maxWidth) ?: if (maxWidth?.kind == "fill") Dp.Infinity else Dp.Unspecified
            val heightMinimum = resolve(minHeight) ?: if (minHeight?.kind == "fill") 0.dp else Dp.Unspecified
            val heightMaximum = resolve(maxHeight) ?: if (maxHeight?.kind == "fill") Dp.Infinity else Dp.Unspecified
            if (widthMinimum != Dp.Unspecified || widthMaximum != Dp.Unspecified) {
                scrollModifier = scrollModifier.widthIn(min = widthMinimum, max = widthMaximum)
            }
            if (heightMinimum != Dp.Unspecified || heightMaximum != Dp.Unspecified) {
                scrollModifier = scrollModifier.heightIn(min = heightMinimum, max = heightMaximum)
            }
            val parentDimensions = listOf(
                "width" to width,
                "height" to height,
                "minWidth" to minWidth,
                "maxWidth" to maxWidth,
                "minHeight" to minHeight,
                "maxHeight" to maxHeight,
            ).filter { it.second?.kind == "parent" }
            if (parentDimensions.isNotEmpty()) {
                val values = parentDimensions.associate { (name, value) -> name to requireNotNull(value).value }
                scrollModifier = scrollModifier.nexaParentRelativeDimension(
                    width = values["width"],
                    height = values["height"],
                    minWidth = values["minWidth"],
                    maxWidth = values["maxWidth"],
                    minHeight = values["minHeight"],
                    maxHeight = values["maxHeight"],
                )
            }
            val effects = style.optJSONObject("effects") ?: JSONObject()
            val cornerRadius = if (!style.isNull("corner_radius_expression")) {
                ((store.evaluatePresented(style.opt("corner_radius_expression"), locals, scope) as? Number)
                    ?.toFloat() ?: style.optDouble("corner_radius", 0.0).toFloat()).dp
            } else {
                style.optDouble("corner_radius", 0.0).dp
            }
            val bottomCornerRadius = style.optDouble("corner_radius_bottom", cornerRadius.value.toDouble()).dp
            val hasCornerRadius = !style.isNull("corner_radius")
                || !style.isNull("corner_radius_expression")
                || !style.isNull("corner_radius_bottom")
            val cornerShape = if (!style.isNull("corner_radius_bottom")) {
                RoundedCornerShape(
                    topStart = cornerRadius,
                    topEnd = cornerRadius,
                    bottomEnd = bottomCornerRadius,
                    bottomStart = bottomCornerRadius,
                )
            } else {
                RoundedCornerShape(cornerRadius)
            }
            if (!style.isNull("opacity")) scrollModifier = scrollModifier.graphicsLayer(
                alpha = style.optDouble("opacity").toFloat(),
                compositingStrategy = CompositingStrategy.Offscreen,
            )
            if (!effects.isNull("scale") || !effects.isNull("scale_expression") || !effects.isNull("scale_y") || !effects.isNull("scale_y_expression") || !effects.isNull("opacity_expression") || !effects.isNull("offset_y_expression") || !effects.isNull("rotation")) {
                val scaleX = if (!effects.isNull("scale_expression")) {
                    (store.evaluatePresented(effects.opt("scale_expression"), locals, scope) as? Number)
                        ?.toFloat() ?: 1f
                } else {
                    effects.optDouble("scale", 1.0).toFloat()
                }
                val scaleY = if (!effects.isNull("scale_y_expression")) {
                    (store.evaluatePresented(effects.opt("scale_y_expression"), locals, scope) as? Number)
                        ?.toFloat() ?: scaleX
                } else if (!effects.isNull("scale_y")) {
                    effects.optDouble("scale_y", scaleX.toDouble()).toFloat()
                } else {
                    scaleX
                }
                val rotation = effects.optDouble("rotation", 0.0).toFloat()
                val alpha = if (!effects.isNull("opacity_expression")) {
                    (store.evaluatePresented(effects.opt("opacity_expression"), locals, scope) as? Number)
                        ?.toFloat()?.coerceIn(0f, 1f) ?: 1f
                } else 1f
                val translationY = if (!effects.isNull("offset_y_expression")) {
                    val offsetY = (store.evaluatePresented(effects.opt("offset_y_expression"), locals, scope) as? Number)
                        ?.toFloat() ?: 0f
                    with(LocalDensity.current) { offsetY.dp.toPx() }
                } else 0f
                scrollModifier = scrollModifier.graphicsLayer(
                    scaleX = scaleX,
                    scaleY = scaleY,
                    alpha = alpha,
                    translationY = translationY,
                    rotationZ = rotation,
                )
            }
            if (!effects.isNull("blur")) scrollModifier = scrollModifier.blur(
                effects.optDouble("blur").dp,
                edgeTreatment = androidx.compose.ui.draw.BlurredEdgeTreatment.Unbounded,
            )
            effects.optJSONObject("shadow")?.let { shadow ->
                val shadowShape = if (!effects.isNull("clip_rounded")) {
                    RoundedCornerShape(effects.optDouble("clip_rounded").dp)
                } else {
                    cornerShape
                }
                val color = nexaDevColor(shadow.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Black
                scrollModifier = scrollModifier.dropShadow(
                    shadowShape,
                    Shadow(
                        radius = shadow.optDouble("radius", 0.0).dp,
                        color = color,
                        offset = DpOffset(shadow.optDouble("x", 0.0).dp, shadow.optDouble("y", 0.0).dp),
                    ),
                )
            }
            if (hasCornerRadius) scrollModifier = scrollModifier.clip(cornerShape)
            if (background != null) scrollModifier = scrollModifier.background(background)
            if (!style.isNull("border_color") && !style.isNull("border_width")) {
                nexaDevColor(style.optJSONObject("border_color"), isSystemInDarkTheme())?.let {
                    scrollModifier = scrollModifier.border(style.optDouble("border_width").dp, it, cornerShape)
                }
            }
            effects.optJSONObject("glass")?.let { glass ->
                val shapeFields = glass.optJSONObject("shape") ?: JSONObject()
                val glassShape: androidx.compose.ui.graphics.Shape = when {
                    shapeFields.has("Circle") -> CircleShape
                    shapeFields.has("Rounded") -> RoundedCornerShape(shapeFields.optDouble("Rounded").dp)
                    shapeFields.has("Capsule") -> RoundedCornerShape(percent = 50)
                    else -> RoundedCornerShape(percent = 50)
                }
                val surfaceColor = nexaDevColor(glass.optJSONObject("tint"), isSystemInDarkTheme())
                    ?.copy(alpha = 1f)
                    ?: background
                    ?: MaterialTheme.colorScheme.surface
                scrollModifier = scrollModifier
                    .clip(glassShape)
                    .background(surfaceColor, glassShape)
            }
            if (!style.isNull("animation")) {
                scrollModifier = scrollModifier.animateContentSize(
                    animationSpec = store.devAnimationSpec(style.opt("animation")),
                )
            }
            val boundedScrollModifier = scrollModifier.fillMaxSize()
            NexaScrollViewPrimitive(
                axis = axis,
                modifier = boundedScrollModifier,
                componentRef = componentRef,
            ) {
                if (axis == "Horizontal") {
                    NexaRowPrimitive(
                        spacing = 0f,
                        verticalAlignment = Alignment.CenterVertically,
                    ) { RenderRowChildren(children, module, store, locals, scope) }
                } else {
                    NexaColumnPrimitive(
                        spacing = 0f,
                        horizontalAlignment = Alignment.Start,
                    ) { RenderColumnChildren(children, module, store, locals, scope) }
                }
            }
        }
        "Layout" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            val spacing = fields.optDouble("gap", 0.0).dp
            val style = fields.optJSONObject(NexaDevKeys.STYLE) ?: JSONObject()
            // Keep the parent sizing contract (notably a screen's fillMaxSize)
            // when applying this node's own visual modifiers.
            val modifier = nexaDevApplyViewStyleModifier(modifier, style, store, locals, scope)
            val rawAlignment = fields.opt("alignment")
            val alignment = rawAlignment as? String
                ?: (rawAlignment as? JSONObject)?.keys()?.asSequence()?.firstOrNull()
                ?: style.optString("alignment")
            val layoutContent: @Composable (Modifier) -> Unit = { layoutModifier ->
            when (fields.optString("kind")) {
                "Row" -> NexaRowPrimitive(
                    modifier = layoutModifier,
                    spacing = spacing.value,
                    horizontalArrangement = when (style.optString("horizontal_arrangement")) {
                        "Start" -> androidx.compose.foundation.layout.Arrangement.Start
                        "Center" -> androidx.compose.foundation.layout.Arrangement.Center
                        "End" -> androidx.compose.foundation.layout.Arrangement.End
                        else -> null
                    },
                    preserveChildWidths = nexaDevHasOnlyFixedWidthLayoutChildren(children),
                    verticalAlignment = when (alignment) {
                        "Start" -> Alignment.Top
                        "End" -> Alignment.Bottom
                        else -> Alignment.CenterVertically
                    },
                ) { RenderRowChildren(children, module, store, locals, scope) }
                "Stack" -> NexaStackPrimitive(
                    modifier = layoutModifier,
                    contentAlignment = when (alignment) {
                        "Start" -> Alignment.TopStart
                        "End" -> Alignment.BottomEnd
                        else -> Alignment.Center
                    },
                ) { RenderChildren(children, module, store, locals, scope) }
                else -> NexaColumnPrimitive(
                    modifier = layoutModifier,
                    spacing = spacing.value,
                    horizontalAlignment = when (alignment) {
                        "Start" -> Alignment.Start
                        "Center" -> Alignment.CenterHorizontally
                        "End" -> Alignment.End
                        else -> Alignment.CenterHorizontally
                    },
                ) { RenderColumnChildren(children, module, store, locals, scope) }
            }
            }
            val marquee = style.optJSONObject("marquee")
            if (marquee != null) {
                NexaMarqueePrimitive(
                    horizontal = fields.optString("kind") == "Row",
                    speed = marquee.optDouble("speed", 32.0), gap = marquee.optDouble("gap", 24.0),
                    delaySeconds = marquee.optDouble("delay", 1.0),
                    edgeFade = marquee.optDouble("edge_fade", 0.0),
                    playing = store.truthy(store.evaluatePresented(marquee.opt("playing"), locals, scope)),
                    revision = nexaDevMarqueeRevision(children, store, locals, scope),
                    modifier = modifier,
                ) { layoutContent(Modifier) }
            } else layoutContent(modifier)

        }
        "ContentUnavailable" -> {
            val title = store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope))
            val description = fields.opt("description")
                ?.takeUnless { it === JSONObject.NULL }
                ?.let { store.stringify(store.evaluatePresented(it, locals, scope)) }
            val icon = fields.optJSONObject("icon") ?: JSONObject()
            val sharedIcon = icon.optString("shared")
            val iconSpec = nexaDevIconSpec(icon, store, locals, scope)
                ?: NexaIconSpec(vector = nexaDevSharedMaterialIcon("help"))
            val dark = isSystemInDarkTheme()
            @Composable
            fun color(name: String): Color? {
                val value = fields.optJSONObject(name) ?: return null
                val dynamic = value.opt("Dynamic")
                return if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, dark)
                }
            }
            NexaContentUnavailablePrimitive(
                title = title,
                description = description,
                icon = iconSpec,
                iconDescription = nexaDevIconAccessibilityLabel(sharedIcon),
                titleColor = color("title_color"),
                descriptionColor = color("description_color"),
                iconTint = color("icon_tint"),
                background = color("background"),
            )
        }
        "Text", "Spacer", "Divider", "Icon", "SystemIcon", "FontIcon", "LinearGradient", NexaDevNodeKind.SHAPE ->
            NexaDevRenderTextNode(kind, fields, module, store, locals, scope, modifier)
        "Button", "Pressable", "TextInput", "FastList", "Switch", "Slider",
        "ProgressBar", "ProgressRing", "SegmentedControl", "Picker", "PickerDate" ->
            NexaDevRenderControlNode(kind, fields, module, store, locals, scope, modifier, buttonPresentation)
        "Image", "RefreshControl" ->
            NexaDevRenderListNode(kind, fields, module, store, locals, scope, modifier)
        "If", "When", "Link", "Accessibility", "KeyboardAware", "AppBottomBar",
        "PagePager", "Toolbar", "BottomSheet", "Dialog", "ConfirmationDialog",
        "NavigationStack", "NavigationSplitView", "NavigationLink", "NavigationBack" ->
            NexaDevRenderNavigationNode(kind, fields, module, store, locals, scope, modifier)
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderTextNode(
    kind: String,
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
    when (kind) {
        "Text" -> {
            val style = fields.optJSONObject(NexaDevKeys.STYLE) ?: JSONObject()
            val color = nexaDevColor(style.optJSONObject("color"), isSystemInDarkTheme())
            val weight = when (style.optString("font_weight")) {
                "Normal" -> FontWeight.Normal
                "Medium" -> FontWeight.Medium
                "Semibold" -> FontWeight.SemiBold
                "Bold" -> FontWeight.Bold
                else -> null
            }
            val content: @Composable () -> Unit = {
                val text = store.stringify(store.evaluatePresented(fields.opt("value"), locals, scope))
                val textAlign = when (style.optString("alignment")) {
                    "Leading" -> TextAlign.Start
                    "Center" -> TextAlign.Center
                    "Trailing" -> TextAlign.End
                    // Keep the unset value nullable so NexaTextPrimitive inherits
                    // the containing Column's default text alignment.
                    else -> null
                }
                val semanticMetrics = when (style.optString("font_style")) {
                    "LargeTitle" -> 34.sp to FontWeight.Normal
                    "Title" -> 28.sp to FontWeight.Normal
                    "Title2" -> 22.sp to FontWeight.Normal
                    "Title3" -> 20.sp to FontWeight.Normal
                    "Headline" -> 17.sp to FontWeight.SemiBold
                    "Subheadline" -> 15.sp to FontWeight.Normal
                    "Body" -> 17.sp to FontWeight.Normal
                    "Callout" -> 16.sp to FontWeight.Normal
                    "Footnote" -> 13.sp to FontWeight.Normal
                    "Caption" -> 12.sp to FontWeight.Normal
                    "Caption2" -> 11.sp to FontWeight.Normal
                    else -> null
                }
                val fontSize = if (style.has("font_size") && !style.isNull("font_size")) {
                    style.getDouble("font_size").sp
                } else {
                    semanticMetrics?.first ?: nexaDevDefaultBodyFontSize.sp
                }
                val fontWeight = weight ?: semanticMetrics?.second ?: FontWeight.Normal
                val customFontFamily = style.nexaDevOptionalString("font_family")?.takeIf(String::isNotBlank)
                val fontFamily = if (customFontFamily != null) {
                    nexaDevFontFamily(customFontFamily, fontWeight)
                } else {
                    when (style.optString("font_design")) {
                        "Default" -> androidx.compose.ui.text.font.FontFamily.Default
                        "Serif" -> androidx.compose.ui.text.font.FontFamily.Serif
                        "Rounded" -> androidx.compose.ui.text.font.FontFamily(
                            android.graphics.Typeface.create("sans-serif-rounded", android.graphics.Typeface.NORMAL),
                        )
                        "Monospaced" -> androidx.compose.ui.text.font.FontFamily.Monospace
                        else -> null
                    }
                }
                val lineHeight = if (style.has("line_height") && !style.isNull("line_height")) {
                    style.getDouble("line_height").sp
                } else {
                    fontSize * nexaDevDefaultLineHeightMultiplier
                }
                val maxLines = if (style.has("line_limit") && !style.isNull("line_limit")) style.getInt("line_limit") else Int.MAX_VALUE
                val letterSpacing = if (style.has("letter_spacing") && !style.isNull("letter_spacing")) style.getDouble("letter_spacing").sp else 0.sp
                // Preserve layout modifiers supplied by the parent. In
                // particular, ViewInsets applies component margin here; losing
                // it makes scroll content measure as if the margin were zero.
                var textModifier: Modifier = modifier
                val effects = style.optJSONObject("effects") ?: JSONObject()
                if (!style.isNull("padding")) textModifier = textModifier.padding(style.optDouble("padding").dp)
                if (!style.isNull("opacity")) textModifier = textModifier.alpha(style.optDouble("opacity").toFloat())
                if (!effects.isNull("scale") || !effects.isNull("scale_expression") || !effects.isNull("scale_y") || !effects.isNull("scale_y_expression") || !effects.isNull("opacity_expression") || !effects.isNull("offset_y_expression") || !effects.isNull("rotation")) {
                    val scaleX = if (!effects.isNull("scale_expression")) {
                        (store.evaluatePresented(effects.opt("scale_expression"), locals, scope) as? Number)
                            ?.toFloat() ?: 1f
                    } else {
                        effects.optDouble("scale", 1.0).toFloat()
                    }
                    val scaleY = if (!effects.isNull("scale_y_expression")) {
                        (store.evaluatePresented(effects.opt("scale_y_expression"), locals, scope) as? Number)
                            ?.toFloat() ?: scaleX
                    } else if (!effects.isNull("scale_y")) {
                        effects.optDouble("scale_y", scaleX.toDouble()).toFloat()
                    } else {
                        scaleX
                    }
                    val rotation = effects.optDouble("rotation", 0.0).toFloat()
                    val alpha = if (!effects.isNull("opacity_expression")) {
                        (store.evaluatePresented(effects.opt("opacity_expression"), locals, scope) as? Number)
                            ?.toFloat()?.coerceIn(0f, 1f) ?: 1f
                    } else 1f
                    val translationY = if (!effects.isNull("offset_y_expression")) {
                        val offsetY = (store.evaluatePresented(effects.opt("offset_y_expression"), locals, scope) as? Number)
                            ?.toFloat() ?: 0f
                        with(LocalDensity.current) { offsetY.dp.toPx() }
                    } else 0f
                    textModifier = textModifier.graphicsLayer(
                        scaleX = scaleX,
                        scaleY = scaleY,
                        alpha = alpha,
                        translationY = translationY,
                        rotationZ = rotation,
                    )
                }
                if (!effects.isNull("blur")) textModifier = textModifier.blur(effects.optDouble("blur").dp, edgeTreatment = androidx.compose.ui.draw.BlurredEdgeTreatment.Unbounded)
                val textShadow = effects.optJSONObject("shadow")?.let { shadow ->
                    val color = nexaDevColor(shadow.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Black
                    NexaGlyphShadow(
                        color = color,
                        radius = shadow.optDouble("radius", 0.0).dp,
                        x = shadow.optDouble("x", 0.0).dp,
                        y = shadow.optDouble("y", 0.0).dp,
                    )
                }
                if (!effects.isNull("clip_rounded")) textModifier = textModifier.clip(RoundedCornerShape(effects.optDouble("clip_rounded").dp))
                if (!effects.isNull("z_index")) textModifier = textModifier.zIndex(effects.optInt("z_index").toFloat())
                val reveal = style.optJSONObject("reveal")
                val numberFlow = style.optJSONObject("number_flow")
                if (numberFlow != null) {
                    val numericValue = (store.evaluatePresented(fields.opt("value"), locals, scope) as? Number)?.toDouble() ?: 0.0
                    val format = numberFlow.optString("format", "Number")
                    val currencyCode = if (numberFlow.isNull("currency_code")) null else numberFlow.optString("currency_code")
                    NexaNumberFlowPrimitive(
                        value = numericValue,
                        format = format,
                        currencyCode = currencyCode,
                        minimumFractionDigits = if (numberFlow.isNull("minimum_fraction_digits")) null else numberFlow.optInt("minimum_fraction_digits"),
                        maximumFractionDigits = if (numberFlow.isNull("maximum_fraction_digits")) null else numberFlow.optInt("maximum_fraction_digits"),
                        animationSpec = store.devAnimationSpec<androidx.compose.ui.unit.IntOffset>(numberFlow.opt("animation")),
                        modifier = textModifier,
                        color = color ?: Color.Unspecified,
                        fontSize = fontSize,
                        fontWeight = fontWeight,
                        fontFamily = fontFamily,
                        textAlign = textAlign,
                        maxLines = maxLines,
                        lineHeight = lineHeight,
                        letterSpacing = letterSpacing,
                        strikethrough = style.optBoolean("strikethrough"),
                        italic = style.optBoolean("italic"),
                        underline = style.optBoolean("underline"),
                        selectable = style.optBoolean("selectable"),
                        textShadow = textShadow,
                    )
                } else if (reveal == null) {
                    NexaTextPrimitive(
                        text = text,
                        modifier = textModifier,
                        color = color ?: Color.Unspecified,
                        fontSize = fontSize,
                        fontWeight = fontWeight,
                        fontFamily = fontFamily,
                        textAlign = textAlign,
                        maxLines = maxLines,
                        lineHeight = lineHeight,
                        letterSpacing = letterSpacing,
                        strikethrough = style.optBoolean("strikethrough"),
                        italic = style.optBoolean("italic"),
                        underline = style.optBoolean("underline"),
                        selectable = style.optBoolean("selectable"),
                        textShadow = textShadow,
                    )
                } else {
                    val segments = store.evaluatePresented(reveal.opt("segments"), locals, scope) as? List<*> ?: emptyList<Any>()
                    fun field(segment: Any?, name: String): Any? = when (segment) {
                        is Map<*, *> -> segment[name]
                        is JSONObject -> segment.opt(name)
                        is NexaDevUserClassInstance -> segment.fields[name]
                        else -> null
                    }
                    NexaTextRevealPrimitive(
                        text = text,
                        segments = segments,
                        textOf = { segment -> field(segment, "text")?.toString().orEmpty() },
                        startTimeOf = { segment -> (field(segment, "startTimeSeconds") as? Number)?.toDouble() ?: 0.0 },
                        endTimeOf = { segment -> (field(segment, "endTimeSeconds") as? Number)?.toDouble() ?: 0.0 },
                        hasTrailingSpaceOf = { segment -> field(segment, "hasTrailingSpace") as? Boolean ?: true },
                        position = (store.evaluatePresented(reveal.opt("position"), locals, scope) as? Number)?.toDouble() ?: 0.0,
                        isActive = store.evaluatePresented(reveal.opt("is_active"), locals, scope) as? Boolean ?: false,
                        isPlaying = store.evaluatePresented(reveal.opt("is_playing"), locals, scope) as? Boolean ?: false,
                        revealEnabled = store.evaluatePresented(reveal.opt("reveal_enabled"), locals, scope) as? Boolean ?: true,
                        seekRevision = (store.evaluatePresented(reveal.opt("seek_revision"), locals, scope) as? Number)?.toLong() ?: 0L,
                        dimOpacity = reveal.optDouble("dim_opacity", 0.38).toFloat(),
                        leadTime = reveal.optDouble("lead_time", 0.3).toFloat(),
                        targetOffset = reveal.optDouble("target_offset", 0.56).toFloat(),
                        smoothingDurationMs = (reveal.optDouble("smoothing_duration", 0.25) * 1000.0).toInt(),
                        modifier = textModifier,
                        color = color ?: Color.Unspecified,
                        fontSize = fontSize,
                        fontWeight = fontWeight,
                        fontFamily = fontFamily,
                        textAlign = textAlign,
                        maxLines = maxLines,
                        lineHeight = lineHeight,
                        letterSpacing = letterSpacing,
                        strikethrough = style.optBoolean("strikethrough"),
                        italic = style.optBoolean("italic"),
                        underline = style.optBoolean("underline"),
                        textShadow = textShadow,
                    )
                }
            }
            content()
        }
        "Spacer" -> {
            val width = nexaDevDimension(fields.opt("width"))
            val height = nexaDevDimension(fields.opt("height"))
            val metrics = if (width?.kind == "screen_width" || height?.kind == "screen_height") {
                LocalNexaWindowMetrics.current
            } else null
            fun resolve(dimension: NexaDevDimension?, widthAxis: Boolean): Dp? = when (dimension?.kind) {
                "fixed" -> dimension.value.dp
                "screen_width" -> if (widthAxis && metrics != null) (metrics.viewportWidthDp * dimension.value).dp else null
                "screen_height" -> if (!widthAxis && metrics != null) (metrics.viewportHeightDp * dimension.value).dp else null
                else -> null
            }
            val resolvedWidth = resolve(width, widthAxis = true)
            val resolvedHeight = resolve(height, widthAxis = false)
            var spacerModifier = when {
                resolvedWidth != null && resolvedHeight != null -> Modifier.size(resolvedWidth, resolvedHeight)
                resolvedWidth != null -> Modifier.width(resolvedWidth)
                resolvedHeight != null -> Modifier.height(resolvedHeight)
                width == null && height == null -> modifier
                else -> Modifier
            }
            if (width?.kind == "fill") spacerModifier = spacerModifier.fillMaxWidth()
            if (height?.kind == "fill") spacerModifier = spacerModifier.fillMaxHeight()
            val parentWidth = width?.takeIf { it.kind == "parent" }?.value
            val parentHeight = height?.takeIf { it.kind == "parent" }?.value
            if (parentWidth != null || parentHeight != null) {
                spacerModifier = spacerModifier.nexaParentRelativeDimension(
                    width = parentWidth,
                    height = parentHeight,
                )
            }
            Spacer(modifier = spacerModifier)
        }
        "Divider" -> {
            val thickness = fields.optDouble("thickness", 1.0).dp
            if (thickness > 0.dp) {
                NexaDividerPrimitive(
                    color = nexaDevColor(fields.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Gray,
                    thickness = thickness,
                )
            }
        }
        NexaDevNodeKind.SHAPE -> {
            val shapeFields = fields.optJSONObject("shape") ?: JSONObject()
            val encodedShape = fields.opt("shape") as? String
            val shape = when {
                shapeFields.has("Circle") || encodedShape == "Circle" -> NexaCenteredCircleShape
                shapeFields.has("Rounded") -> RoundedCornerShape(shapeFields.optDouble("Rounded").dp)
                else -> CircleShape
            }
            val widthExpression = fields.opt("width_expression")
                ?.takeUnless { it === JSONObject.NULL }
            val heightExpression = fields.opt("height_expression")
                ?.takeUnless { it === JSONObject.NULL }
            val rawAnimation = fields.opt("animation")
                ?.takeUnless { it === JSONObject.NULL }
            val measurement = fields.optString(NexaDevKeys.SHAPE_MEASUREMENT)
            val widthReader = if (rawAnimation == null && widthExpression != null &&
                (measurement == "Width" || measurement == "Both")
            ) {
                store.shapeDimensionReader(widthExpression, locals, scope)
            } else null
            val heightReader = if (rawAnimation == null && heightExpression != null &&
                (measurement == "Height" || measurement == "Both")
            ) {
                store.shapeDimensionReader(heightExpression, locals, scope)
            } else null
            val width = when {
                widthExpression == null -> fields.optDouble("width").dp
                widthReader != null -> 0.dp
                else -> shapeDimension(store.evaluatePresented(widthExpression, locals, scope)).dp
            }
            val height = when {
                heightExpression == null -> fields.optDouble("height").dp
                heightReader != null -> 0.dp
                else -> shapeDimension(store.evaluatePresented(heightExpression, locals, scope)).dp
            }
            val animatedWidth = if (widthExpression != null && rawAnimation != null) {
                androidx.compose.animation.core.animateDpAsState(
                    targetValue = width,
                    animationSpec = store.devAnimationSpec<androidx.compose.ui.unit.Dp>(rawAnimation),
                    label = "Nexa shape width",
                )
            } else null
            val animatedHeight = if (heightExpression != null && rawAnimation != null) {
                androidx.compose.animation.core.animateDpAsState(
                    targetValue = height,
                    animationSpec = store.devAnimationSpec<androidx.compose.ui.unit.Dp>(rawAnimation),
                    label = "Nexa shape height",
                )
            } else null
            val fill = nexaDevColor(fields.optJSONObject("fill"), isSystemInDarkTheme()) ?: Color.Transparent
            val measuredWidth = widthReader ?: animatedWidth?.let { state -> NexaShapeDimension { state.value.value } }
            val measuredHeight = heightReader ?: animatedHeight?.let { state -> NexaShapeDimension { state.value.value } }
            if (measuredWidth != null || measuredHeight != null) {
                NexaMeasuredShapePrimitive(
                    fill = fill,
                    shape = shape,
                    modifier = modifier,
                    width = width,
                    height = height,
                    widthReader = measuredWidth,
                    heightReader = measuredHeight,
                )
            } else {
                Spacer(
                    modifier = modifier
                        .size(width, height)
                        .background(fill, shape = shape),
                )
            }
        }
        "Icon" -> {
            val icon = nexaDevIconSpec(fields.opt("icon"), store, locals, scope)
                ?: NexaIconSpec(vector = nexaDevSharedMaterialIcon("help"))
            NexaIconContent(
                icon = icon,
                description = null,
                size = null,
                tint = androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
                modifier = modifier,
            )
        }
        "SystemIcon" -> {
            val selection = fields.optJSONObject("icon") ?: JSONObject()
            val size = if (fields.has("size") && !fields.isNull("size")) fields.optDouble("size").dp else null
            val icon = nexaDevIconSpec(selection, store, locals, scope, defaultSize = size)
                ?: NexaIconSpec(vector = nexaDevSharedMaterialIcon("help"), size = size)
            val tintValue = fields.optJSONObject("tint") ?: JSONObject()
            val dynamicTint = tintValue.opt("Dynamic")
            val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope)))
            } else {
                nexaDevColor(tintValue, isSystemInDarkTheme())
            }
            NexaIconContent(
                icon = icon,
                description = fields.optString("description"),
                size = null,
                tint = tint,
                modifier = modifier,
            )
        }
        "FontIcon" -> {
            val glyphValue = store.evaluatePresented(fields.opt("glyph"), locals, scope) as? Number
            val glyph = glyphValue?.toInt() ?: -1
            val tintValue = fields.optJSONObject("tint") ?: JSONObject()
            val dynamicTint = tintValue.opt("Dynamic")
            val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope)))
            } else {
                nexaDevColor(tintValue, isSystemInDarkTheme())
            }
            NexaFontIconPrimitive(
                glyph = glyph,
                description = fields.optString("description"),
                size = if (!fields.has("size") || fields.isNull("size")) null else {
                    ((store.evaluatePresented(fields.opt("size"), locals, scope) as? Number)
                        ?.toDouble() ?: 17.0).coerceIn(1.0, 512.0).toFloat().dp
                },
                tint = tint,
                layers = nexaDevIconLayers(fields.optJSONArray("layers")),
                layerColors = nexaDevIconLayerColors(fields.optJSONArray("layer_colors"), store, locals, scope),
                modifier = modifier,
                fontFamily = nexaDevIconFontFamily(fields.optString("font_family")),
            )
        }
        "LinearGradient" -> {
            val start = nexaDevColor(fields.optJSONObject("start_color"), isSystemInDarkTheme()) ?: Color.Transparent
            val end = nexaDevColor(fields.optJSONObject("end_color"), isSystemInDarkTheme()) ?: Color.Transparent
            val direction = fields.optString("direction", "TopToBottom")
            NexaLinearGradientPrimitive(
                startColor = start,
                endColor = end,
                horizontal = direction == "LeadingToTrailing" || direction == "TrailingToLeading",
                reversed = direction == "BottomToTop" || direction == "TrailingToLeading",
                height = fields.optDouble("height", 180.0).dp,
            )
        }
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderControlNode(
    kind: String,
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
    buttonPresentation: NexaDevButtonPresentation,
) {
    when (kind) {
        "Button" -> {
            val loading = store.evaluatePresented(fields.opt("loading"), locals, scope) as? Boolean ?: false
            val disabled = store.evaluatePresented(fields.opt("disabled"), locals, scope) as? Boolean ?: false
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            val accessibilityLabel = fields.opt("accessibility_label")
                ?.takeUnless { it == JSONObject.NULL }
                ?.let { store.stringify(store.evaluatePresented(it, locals, scope)) }
                ?: label
            val style = if (buttonPresentation == NexaDevButtonPresentation.ConfirmationAction && fields.isNull("style")) {
                "Borderless"
            } else {
                fields.opt("style")?.toString()?.takeIf { it.isNotBlank() && it != "null" }
            }
            val tintValue = fields.optJSONObject("tint") ?: JSONObject()
            val dynamicTint = tintValue.opt("Dynamic")
            val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope)))
            } else {
                nexaDevColor(tintValue, isSystemInDarkTheme())
            }
            val encodedShape = fields.opt("shape")
            val shapeFields = fields.optJSONObject("shape") ?: JSONObject()
            val buttonShape = when {
                shapeFields.has("Circle") || encodedShape == "Circle" -> CircleShape
                shapeFields.has("Rounded") -> RoundedCornerShape(shapeFields.optDouble("Rounded").dp)
                shapeFields.has("Capsule") || encodedShape == "Capsule" -> RoundedCornerShape(percent = 50)
                else -> null
            }
            val iconSelection = fields.optJSONObject("icon") ?: JSONObject()
            val icon = nexaDevIconSpec(iconSelection, store, locals, scope)
            NexaButtonPrimitive(
                label = label,
                icon = icon,
                style = style,
                size = fields.nexaDevOptionalString("size")?.takeIf(String::isNotEmpty),
                shape = buttonShape,
                tint = tint,
                loading = loading,
                disabled = disabled,
                accessibilityLabel = accessibilityLabel,
                iconOnlyCircle = icon != null && buttonShape == CircleShape && label.isEmpty(),
                fullWidth = buttonPresentation == NexaDevButtonPresentation.ConfirmationAction,
                leadingContent = buttonPresentation == NexaDevButtonPresentation.FormRow,
                onClick = { store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals) },
            )
        }
        "Pressable" -> NexaDevRenderPressableNode(fields, module, store, locals, scope, modifier)
        "TextInput" -> NexaDevRenderTextInputNode(
            fields,
            module,
            store,
            locals,
            scope,
            if (fields.optBoolean("searchable")) modifier else modifier.fillMaxWidth(),
        )
        "FastList" -> NexaDevRenderFastListNode(fields, module, store, locals, scope, modifier)
        "Switch" -> {
            val state = fields.optString("state")
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            val onChange = fields.optJSONObject("on_change")
            NexaSwitchPrimitive(
                label = label,
                checked = store.state(state, scope) as? Boolean ?: false,
                onCheckedChange = { value ->
                    val previous = store.state(state, scope) as? Boolean ?: false
                    if (value != previous) {
                        store.setState(state, value, scope)
                        val parameter = onChange?.nexaDevOptionalString("parameter")
                            ?.takeIf(String::isNotEmpty)
                        if (parameter != null) {
                            val changeLocals = locals.toMutableMap()
                            changeLocals[parameter] = value
                            store.perform(
                                onChange.optJSONArray("actions") ?: JSONArray(),
                                scope,
                                changeLocals,
                            )
                        }
                    }
                },
                fontSize = nexaDevDefaultBodyFontSize.sp,
                lineHeight = nexaDevDefaultBodyFontSize.sp * nexaDevDefaultLineHeightMultiplier,
                offTrackColor = if (MaterialTheme.colorScheme.background == Color.Black) {
                    Color(__NEXA_FORM_SWITCH_OFF_TRACK_ARGB__)
                } else {
                    Color(0xFFE5E5EA)
                },
            )
        }
        "Slider" -> {
            val state = fields.optString("state")
            val minValue = (store.evaluatePresented(fields.opt("min"), locals, scope) as? Number)?.toDouble() ?: 0.0
            val maxValue = (store.evaluatePresented(fields.opt("max"), locals, scope) as? Number)?.toDouble() ?: 1.0
            val min = minValue.toFloat()
            val max = maxValue.toFloat()
            val step = fields.optDouble("step", 0.0)
            val rawValue = if (fields.optBoolean("animated")) {
                store.animatedFloatState(state, scope)
            } else {
                (store.state(state, scope) as? Number)?.toFloat() ?: min
            }
            val value = if (max > min) rawValue.coerceIn(min, max) else min
            val dark = isSystemInDarkTheme()
            @Composable
            fun color(name: String): Color? {
                val value = fields.optJSONObject(name) ?: return null
                val dynamic = value.opt("Dynamic")
                return if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, dark)
                }
            }
            val onChange = fields.optJSONObject("on_change")
            val onEditingChanged = fields.optJSONObject("on_editing_changed")
            NexaSliderPrimitive(
                value = value,
                onValueChange = { next ->
                    store.clearAnimationSpec(state, scope)
                    val nextValue = if (step > 0.0) {
                        minValue + kotlin.math.round(
                            (next.toDouble() - minValue) / step,
                        ) * step
                    } else {
                        next.toDouble()
                    }
                    store.setState(state, nextValue, scope)
                    if (onChange != null) {
                        val parameter = onChange.optString("parameter")
                        if (parameter.isNotEmpty()) {
                            store.perform(
                                onChange.optJSONArray("actions") ?: JSONArray(),
                                scope,
                                locals + (parameter to nextValue),
                            )
                        }
                    }
                },
                minValue = min,
                maxValue = max,
                step = step.toFloat(),
                showStepIndicators = fields.optBoolean("show_step_indicators", true),
                tint = color("tint"),
                trackColor = color("track_color"),
                thumbColor = color("thumb_color"),
                trackHeight = if (fields.isNull("track_height")) null else fields.optDouble("track_height").toFloat(),
                thumbSize = if (fields.isNull("thumb_size")) null else fields.optDouble("thumb_size").toFloat(),
                hitSlop = if (fields.isNull("hit_slop")) null else fields.optDouble("hit_slop").toFloat(),
                onEditingChanged = { editing ->
                    if (onEditingChanged != null) {
                        val parameter = onEditingChanged.optString("parameter")
                        if (parameter.isNotEmpty()) {
                            store.perform(
                                onEditingChanged.optJSONArray("actions") ?: JSONArray(),
                                scope,
                                locals + (parameter to editing),
                            )
                        }
                    }
                },
            )
        }
        "ProgressBar" -> {
            val progress = (store.evaluatePresented(fields.opt("progress"), locals, scope) as? Number)?.toFloat()
            val style = fields.optJSONObject(NexaDevKeys.STYLE)
            val dark = isSystemInDarkTheme()
            @Composable
            fun color(name: String): Color? {
                val value = style?.optJSONObject(name) ?: return null
                val dynamic = value.opt("Dynamic")
                return if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, dark)
                }
            }
            fun dimension(name: String): Float? {
                if (style == null || !style.has(name) || style.isNull(name)) return null
                return style.optDouble(name).toFloat()
            }
            val androidWave = style?.opt("android_wave")?.let {
                store.evaluatePresented(it, locals, scope) as? Boolean
            } ?: false
            NexaProgressBarPrimitive(
                progress = progress,
                tint = color("tint"),
                trackColor = color("track_color"),
                thickness = dimension("thickness"),
                cornerRadius = dimension("corner_radius"),
                androidWave = androidWave,
            )
        }
        "ProgressRing" -> {
            val progress = (store.evaluatePresented(fields.opt("progress"), locals, scope) as? Number)?.toFloat()
            val style = fields.optJSONObject(NexaDevKeys.STYLE)
            val dark = isSystemInDarkTheme()
            @Composable
            fun color(name: String): Color? {
                val value = style?.optJSONObject(name) ?: return null
                val dynamic = value.opt("Dynamic")
                return if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, dark)
                }
            }
            fun dimension(name: String): Float? {
                if (style == null || !style.has(name) || style.isNull(name)) return null
                return style.optDouble(name).toFloat()
            }
            val androidWave = style?.opt("android_wave")?.let {
                store.evaluatePresented(it, locals, scope) as? Boolean
            } ?: false
            NexaProgressRingPrimitive(
                progress = progress,
                tint = color("tint"),
                trackColor = color("track_color"),
                size = dimension("size"),
                thickness = dimension("thickness"),
                androidWave = androidWave,
            )
        }
        "SegmentedControl" -> {
            val state = fields.optString("state")
            val selected = store.state(state, scope) as? String ?: ""
            val options = (store.evaluatePresented(fields.opt("items"), locals, scope) as? List<*>)
                ?.filterIsInstance<String>()
                ?: emptyList()
            val dark = isSystemInDarkTheme()
            @Composable
            fun color(name: String): Color? {
                val value = fields.optJSONObject(name) ?: return null
                val dynamic = value.opt("Dynamic")
                return if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, dark)
                }
            }
            NexaSegmentedControlPrimitive(
                items = options,
                selected = selected,
                animation = if (fields.isNull("animation")) {
                    null
                } else {
                    store.devAnimationSpec(fields.opt("animation"))
                },
                tint = color("tint"),
                background = color("background"),
                textColor = color("text_color"),
                selectedTextColor = color("selected_text_color"),
                fontSize = if (fields.isNull("font_size")) null else fields.optDouble("font_size").toFloat().sp,
                fontWeight = when (fields.optString("font_weight")) {
                    "Normal" -> androidx.compose.ui.text.font.FontWeight.Normal
                    "Medium" -> androidx.compose.ui.text.font.FontWeight.Medium
                    "Semibold" -> androidx.compose.ui.text.font.FontWeight.SemiBold
                    "Bold" -> androidx.compose.ui.text.font.FontWeight.Bold
                    else -> null
                },
                height = if (fields.isNull("height")) null else fields.optDouble("height").toFloat().dp,
                cornerRadius = if (fields.isNull("corner_radius")) null else fields.optDouble("corner_radius").toFloat().dp,
                borderColor = color("border_color"),
                borderWidth = if (fields.isNull("border_width")) null else fields.optDouble("border_width").toFloat().dp,
                onSelected = { store.setState(state, it, scope) },
            )
        }
        "Picker" -> {
            val state = fields.optString("state")
            val selected = store.state(state, scope) as? String ?: ""
            val label = if (fields.isNull("label")) null else {
                store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            }
            val options = (store.evaluatePresented(fields.opt("items"), locals, scope) as? List<*>)
                ?.filterIsInstance<String>()
                ?: emptyList()
            val tintValue = fields.optJSONObject("tint") ?: JSONObject()
            val dynamicTint = tintValue.opt("Dynamic")
            val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope)))
            } else {
                nexaDevColor(tintValue, isSystemInDarkTheme())
            }
            val iconSelection = fields.optJSONObject("icon") ?: JSONObject()
            val pickerIcon = nexaDevIconSpec(iconSelection, store, locals, scope)
            val itemIconsRaw = fields.opt("item_icons")
            val evaluatedItemIcons = if (itemIconsRaw != null && itemIconsRaw != JSONObject.NULL) {
                store.evaluatePresented(itemIconsRaw, locals, scope)
            } else {
                null
            }
            val pickerItemIcons = buildList<NexaIconSpec?> {
                when (evaluatedItemIcons) {
                    is List<*> -> evaluatedItemIcons.forEach { add(nexaDevIconSpec(it, store, locals, scope)) }
                    is JSONArray -> for (index in 0 until evaluatedItemIcons.length()) {
                        add(nexaDevIconSpec(evaluatedItemIcons.opt(index), store, locals, scope))
                    }
                }
            }
            NexaPickerPrimitive(
                items = options,
                itemIcons = pickerItemIcons,
                selected = selected,
                icon = pickerIcon,
                label = label,
                tint = tint,
                identity = "$scope::$state",
                fillMaxWidth = fields.optBoolean("__nexa_fill_max_width", true),
                onSelectionChanged = { store.setState(state, it, scope) },
            )
        }
        "PickerDate" -> {
            val pickerFields = fields.optJSONObject("props") ?: fields
            @Composable
            fun expression(name: String): Any? {
                val value = pickerFields.opt(name)
                return if (value == null || value == JSONObject.NULL) null
                else store.evaluatePresented(value, locals, scope)
            }
            @Composable
            fun color(name: String): androidx.compose.ui.graphics.Color? {
                val value = pickerFields.optJSONObject(name) ?: return null
                val dynamic = value.opt("Dynamic")
                return if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, isSystemInDarkTheme())
                }
            }
            @Composable
            fun long(name: String): Long? = (expression(name) as? Number)?.toLong()
            @Composable
            fun string(name: String): String? = expression(name)?.let(store::stringify)
            @Composable
            fun boolean(name: String): Boolean? = expression(name) as? Boolean

            val timestampState = pickerFields.optString("timestamp_state").takeIf(String::isNotBlank)
            val startState = pickerFields.optString("start_timestamp_state").takeIf(String::isNotBlank)
            val endState = pickerFields.optString("end_timestamp_state").takeIf(String::isNotBlank)
            NexaDatePickerPrimitive(
                timestamp = timestampState?.let { (store.state(it, scope) as? Number)?.toLong() },
                startTimestamp = startState?.let { (store.state(it, scope) as? Number)?.toLong() },
                endTimestamp = endState?.let { (store.state(it, scope) as? Number)?.toLong() },
                pickerType = pickerFields.optString("picker_type", "Date"),
                style = pickerFields.optString("style", "Automatic"),
                selection = pickerFields.optString("selection", "Single"),
                minimum = long("minimum"),
                maximum = long("maximum"),
                tint = color("tint"),
                textColor = color("text_color"),
                backgroundColor = color("background_color"),
                pickerBackgroundColor = color("picker_background_color"),
                borderColor = color("border_color"),
                borderWidth = (expression("border_width") as? Number)?.toFloat()?.dp,
                cornerRadius = (expression("corner_radius") as? Number)?.toFloat()?.dp,
                localeTag = string("locale"),
                timeZoneId = string("time_zone"),
                is24Hour = boolean("is_24_hour"),
                enabled = boolean("enabled"),
                accessibilityLabel = string("accessibility_label"),
                onTimestampChanged = timestampState?.let { state ->
                    { value -> store.setState(state, value, scope) }
                },
                onRangeChanged = if (startState != null && endState != null) {
                    { start, finish ->
                        store.setState(startState, start, scope)
                        store.setState(endState, finish, scope)
                    }
                } else null,
            )
        }
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun nexaDevApplyViewStyleModifier(
    initialModifier: Modifier,
    style: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
): Modifier {
    var modifier = initialModifier
    val effects = style.optJSONObject("effects") ?: JSONObject()
    if (!effects.isNull("clip_rounded")) {
        modifier = modifier.clip(RoundedCornerShape(effects.optDouble("clip_rounded").dp))
    }
    val cornerRadius = if (!style.isNull("corner_radius_expression")) {
        ((store.evaluatePresented(style.opt("corner_radius_expression"), locals, scope) as? Number)
            ?.toFloat() ?: style.optDouble("corner_radius", 0.0).toFloat()).dp
    } else {
        style.optDouble("corner_radius", 0.0).dp
    }
    val bottomCornerRadius = style.optDouble("corner_radius_bottom", cornerRadius.value.toDouble()).dp
    val hasCornerRadius = !style.isNull("corner_radius")
        || !style.isNull("corner_radius_expression")
        || !style.isNull("corner_radius_bottom")
    val cornerShape = if (!style.isNull("corner_radius_bottom")) {
        RoundedCornerShape(
            topStart = cornerRadius,
            topEnd = cornerRadius,
            bottomEnd = bottomCornerRadius,
            bottomStart = bottomCornerRadius,
        )
    } else {
        RoundedCornerShape(cornerRadius)
    }
    val metrics = LocalNexaWindowMetrics.current
    fun dimensionValue(name: String): NexaDevDimension? = nexaDevDimension(style.opt(name))
    val width = dimensionValue("width")
    val height = dimensionValue("height")
    val minWidth = dimensionValue("min_width")
    val maxWidth = dimensionValue("max_width")
    val minHeight = dimensionValue("min_height")
    val maxHeight = dimensionValue("max_height")
    fun resolve(dimension: NexaDevDimension?): Dp? = when (dimension?.kind) {
        "fixed" -> dimension.value.dp
        "screen_width" -> (metrics.viewportWidthDp * dimension.value).dp
        "screen_height" -> (metrics.viewportHeightDp * dimension.value).dp
        else -> null
    }
    resolve(width)?.let { modifier = modifier.requiredWidth(it) }
    resolve(height)?.let {
        modifier = modifier
            .wrapContentHeight(LocalNexaLayoutOverflowAlignment.current, unbounded = true)
            .requiredHeight(it)
    }
    if (!style.isNull("opacity")) {
        val opacityValue = style.optDouble("opacity").toFloat()
        if (opacityValue < 1f) {
            modifier = modifier.graphicsLayer(
                alpha = opacityValue,
                compositingStrategy = CompositingStrategy.Offscreen,
            )
        }
    }
    if (!effects.isNull("scale") || !effects.isNull("scale_expression") || !effects.isNull("scale_y") || !effects.isNull("scale_y_expression") || !effects.isNull("opacity_expression") || !effects.isNull("offset_y_expression") || !effects.isNull("rotation")) {
        val scaleX = if (!effects.isNull("scale_expression")) {
            (store.evaluatePresented(effects.opt("scale_expression"), locals, scope) as? Number)
                ?.toFloat() ?: 1f
        } else {
            effects.optDouble("scale", 1.0).toFloat()
        }
        val scaleY = if (!effects.isNull("scale_y_expression")) {
            (store.evaluatePresented(effects.opt("scale_y_expression"), locals, scope) as? Number)
                ?.toFloat() ?: scaleX
        } else if (!effects.isNull("scale_y")) {
            effects.optDouble("scale_y", scaleX.toDouble()).toFloat()
        } else {
            scaleX
        }
        val rotation = effects.optDouble("rotation", 0.0).toFloat()
        val alpha = if (!effects.isNull("opacity_expression")) {
            (store.evaluatePresented(effects.opt("opacity_expression"), locals, scope) as? Number)
                ?.toFloat()?.coerceIn(0f, 1f) ?: 1f
        } else 1f
        val translationY = if (!effects.isNull("offset_y_expression")) {
            val offsetY = (store.evaluatePresented(effects.opt("offset_y_expression"), locals, scope) as? Number)
                ?.toFloat() ?: 0f
            with(LocalDensity.current) { offsetY.dp.toPx() }
        } else 0f
        if (scaleX != 1f || scaleY != 1f || alpha != 1f || translationY != 0f || rotation != 0f) {
            modifier = modifier.graphicsLayer(
                scaleX = scaleX,
                scaleY = scaleY,
                alpha = alpha,
                translationY = translationY,
                rotationZ = rotation,
            )
        }
    }
    if (!effects.isNull("blur")) {
        val blurRadius = effects.optDouble("blur").dp
        if (blurRadius > 0.dp) {
            modifier = modifier.blur(blurRadius, edgeTreatment = androidx.compose.ui.draw.BlurredEdgeTreatment.Unbounded)
        }
    }
    effects.optJSONObject("shadow")?.let { shadow ->
        val shadowShape = if (!effects.isNull("clip_rounded")) {
            RoundedCornerShape(effects.optDouble("clip_rounded").dp)
        } else {
            cornerShape
        }
        val color = nexaDevColor(shadow.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Black
        modifier = modifier.dropShadow(
            shadowShape,
            Shadow(
                radius = shadow.optDouble("radius", 0.0).dp,
                color = color,
                offset = DpOffset(shadow.optDouble("x", 0.0).dp, shadow.optDouble("y", 0.0).dp),
            ),
        )
    }
    if (hasCornerRadius) modifier = modifier.clip(cornerShape)
    nexaDevColor(style.optJSONObject("background"), isSystemInDarkTheme())?.let { modifier = modifier.background(it) }
    if (!style.isNull("border_color") && !style.isNull("border_width")) {
        nexaDevColor(style.optJSONObject("border_color"), isSystemInDarkTheme())?.let {
            modifier = modifier.border(style.optDouble("border_width").dp, it, cornerShape)
        }
    }
    effects.optJSONObject("glass")?.let { glass ->
        val shapeFields = glass.optJSONObject("shape") ?: JSONObject()
        val glassShape: androidx.compose.ui.graphics.Shape = when {
            shapeFields.has("Circle") -> CircleShape
            shapeFields.has("Rounded") -> RoundedCornerShape(shapeFields.optDouble("Rounded").dp)
            shapeFields.has("Capsule") -> RoundedCornerShape(percent = 50)
            else -> RoundedCornerShape(percent = 50)
        }
        val surfaceColor = nexaDevColor(glass.optJSONObject("tint"), isSystemInDarkTheme())
            ?.copy(alpha = 1f)
            ?: nexaDevColor(style.optJSONObject("background"), isSystemInDarkTheme())
            ?: MaterialTheme.colorScheme.surface
        modifier = modifier
            .clip(glassShape)
            .background(surfaceColor, glassShape)
    }
    if (!style.isNull("animation")) {
        modifier = modifier.animateContentSize(
            animationSpec = store.devAnimationSpec(style.opt("animation")),
        )
    }
    resolve(width)?.let { modifier = modifier.width(it) }
    resolve(height)?.let { modifier = modifier.height(it) }
    if (width?.kind == "fill") modifier = modifier.fillMaxWidth()
    if (height?.kind == "fill") modifier = modifier.fillMaxHeight()
    val widthMinimum = resolve(minWidth) ?: if (minWidth?.kind == "fill") 0.dp else Dp.Unspecified
    val widthMaximum = resolve(maxWidth) ?: if (maxWidth?.kind == "fill") Dp.Infinity else Dp.Unspecified
    val heightMinimum = resolve(minHeight) ?: if (minHeight?.kind == "fill") 0.dp else Dp.Unspecified
    val heightMaximum = resolve(maxHeight) ?: if (maxHeight?.kind == "fill") Dp.Infinity else Dp.Unspecified
    if (widthMinimum != Dp.Unspecified || widthMaximum != Dp.Unspecified) {
        modifier = modifier.widthIn(min = widthMinimum, max = widthMaximum)
    }
    if (heightMinimum != Dp.Unspecified || heightMaximum != Dp.Unspecified) {
        modifier = modifier.heightIn(min = heightMinimum, max = heightMaximum)
    }
    val parentDimensions = listOf(
        "width" to width,
        "height" to height,
        "minWidth" to minWidth,
        "maxWidth" to maxWidth,
        "minHeight" to minHeight,
        "maxHeight" to maxHeight,
    ).filter { it.second?.kind == "parent" }
    if (parentDimensions.isNotEmpty()) {
        val values = parentDimensions.associate { (name, dimension) -> name to requireNotNull(dimension).value }
        modifier = modifier.nexaParentRelativeDimension(
            width = values["width"],
            height = values["height"],
            minWidth = values["minWidth"],
            maxWidth = values["maxWidth"],
            minHeight = values["minHeight"],
            maxHeight = values["maxHeight"],
        )
    }
    if (!effects.isNull("z_index")) modifier = modifier.zIndex(effects.optInt("z_index").toFloat())
    return modifier
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderListNode(
    kind: String,
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
    when (kind) {
        "Image" -> {
            val source = fields.optJSONObject("source") ?: JSONObject()
            val description = fields.nexaDevOptionalString("description")?.takeIf(String::isNotEmpty)
            val fitName = fields.optString("fit").ifEmpty { fields.optString("scale") }
            val scale = if (fitName == "Fill") ContentScale.Crop else ContentScale.Fit
            val placeholder = fields.nexaDevOptionalString("placeholder")?.takeIf(String::isNotEmpty)
            val sharedElementExpression = fields.opt("shared_element")
            val style = fields.optJSONObject("style") ?: JSONObject()
            var renderedModifier = if (sharedElementExpression != null && sharedElementExpression != JSONObject.NULL) {
                modifier.then(nexaSharedElementModifier(store.evaluatePresented(sharedElementExpression, locals, scope)))
            } else {
                modifier
            }
            val widthDim = nexaDevDimension(style.opt("width"))
            val heightDim = nexaDevDimension(style.opt("height"))
            val maxWidthDim = nexaDevDimension(style.opt("max_width"))
            if (scale == ContentScale.Crop && widthDim == null && maxWidthDim == null) {
                renderedModifier = renderedModifier.fillMaxWidth()
            }
            renderedModifier = nexaDevApplyViewStyleModifier(renderedModifier, style, store, locals, scope)
            val decodeWidthDp = if (widthDim?.kind == "fixed") widthDim.value.toFloat() else null
            val decodeHeightDp = if (heightDim?.kind == "fixed") heightDim.value.toFloat() else null
            val showLoadingIndicator = fields.optBoolean("show_loading_indicator", false)
            val autoReplay = fields.optBoolean("auto_replay", true)
            val asset = source.optString("Asset").takeIf(String::isNotEmpty)
            val remoteExpression = source.opt("RemoteUrl")
            val localFileExpression = source.opt("LocalFile")
            if (asset != null) {
                val stem = asset.substringBeforeLast('.')
                if (asset.endsWith(".gif", ignoreCase = true)) {
                    NexaGifAssetImagePrimitive(
                        resourceId = nexaRawId(stem),
                        contentDescription = description,
                        contentScale = scale,
                        autoReplay = autoReplay,
                        decodeWidthDp = decodeWidthDp,
                        decodeHeightDp = decodeHeightDp,
                        modifier = renderedModifier,
                    )
                } else {
                    NexaAssetImagePrimitive(
                        resourceId = nexaDrawableId(stem),
                        contentDescription = description,
                        contentScale = scale,
                        modifier = renderedModifier,
                    )
                }
            } else if (remoteExpression != null && remoteExpression != JSONObject.NULL) {
                val url = store.stringify(store.evaluatePresented(remoteExpression, locals, scope))
                NexaRemoteImagePrimitive(
                    url = url,
                    allowsFile = false,
                    placeholderResourceId = placeholder?.let { nexaDrawableId(it.substringBeforeLast('.')) },
                    showLoadingIndicator = showLoadingIndicator,
                    autoReplay = autoReplay,
                    decodeWidthDp = decodeWidthDp,
                    decodeHeightDp = decodeHeightDp,
                    contentDescription = description,
                    contentScale = scale,
                    modifier = renderedModifier,
                )
            } else if (localFileExpression != null && localFileExpression != JSONObject.NULL) {
                val fileUrl = store.stringify(store.evaluatePresented(localFileExpression, locals, scope))
                NexaRemoteImagePrimitive(
                    url = fileUrl,
                    allowsFile = true,
                    placeholderResourceId = null,
                    showLoadingIndicator = showLoadingIndicator,
                    autoReplay = autoReplay,
                    decodeWidthDp = decodeWidthDp,
                    decodeHeightDp = decodeHeightDp,
                    contentDescription = description,
                    contentScale = scale,
                    modifier = renderedModifier,
                )
            } else {
                Text(placeholder ?: "Image unavailable", modifier = renderedModifier)
            }
        }
        "RefreshControl" -> {
            val state = fields.optString("state")
            val actions = fields.optJSONArray("actions") ?: JSONArray()
            val children = fields.optJSONArray("children") ?: JSONArray()
            NexaRefreshControlPrimitive(
                isRefreshing = store.state(state, scope) as? Boolean ?: false,
                onRefresh = { store.perform(actions, scope, locals) },
                scrollContent = !nexaDevContainsScrollable(children),
                modifier = modifier,
            ) {
                RenderChildren(children, module, store, locals, scope)
            }
        }
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderNavigationNode(
    kind: String,
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
    when (kind) {
        "If" -> {
            val condition = store.evaluatePresented(fields.opt("condition"), locals, scope) as? Boolean ?: false
            val transitionStateName = nexaDevTransitionStateName(fields.opt("condition"))
            val transition = fields.nexaDevTransition()
            val rawAnimation = fields.opt("transition_animation")
            val floatAnimation = if (rawAnimation == null || rawAnimation === JSONObject.NULL) null
            else store.devAnimationSpec<Float>(rawAnimation)
            val offsetAnimation = if (rawAnimation == null || rawAnimation === JSONObject.NULL) null
            else store.devAnimationSpec<androidx.compose.ui.unit.IntOffset>(rawAnimation)
            if (transition.isEmpty()) {
                // Sibling branches own separate native state, even at the same child index.
                key(condition) {
                    RenderConditionalChildren(
                        fields.optJSONArray(if (condition) "then_body" else "else_body") ?: JSONArray(),
                        module,
                        store,
                        locals,
                        scope,
                        spacerWeight = modifier,
                    )
                }
            } else {
                AnimatedContent(
                    targetState = condition,
                    transitionSpec = {
                        nexaDevContentTransform(transition, floatAnimation, offsetAnimation)
                    },
                    label = "dev-if-transition",
                ) { target ->
                    val targetLocals = nexaDevTransitionLocals(locals, transitionStateName, target)
                    RenderConditionalChildren(
                        fields.optJSONArray(if (target) "then_body" else "else_body") ?: JSONArray(),
                        module,
                        store,
                        targetLocals,
                        scope,
                    )
                }
            }
        }
        "When" -> {
            val rawValue = store.evaluatePresented(fields.opt("value"), locals, scope)
            val value = store.stringify(rawValue)
            val transitionStateName = nexaDevTransitionStateName(fields.opt("value"))
            val transition = fields.nexaDevTransition()
            val rawAnimation = fields.opt("transition_animation")
            val floatAnimation = if (rawAnimation == null || rawAnimation === JSONObject.NULL) null
            else store.devAnimationSpec<Float>(rawAnimation)
            val offsetAnimation = if (rawAnimation == null || rawAnimation === JSONObject.NULL) null
            else store.devAnimationSpec<androidx.compose.ui.unit.IntOffset>(rawAnimation)
            if (transition.isEmpty()) {
                renderWhenTarget(value, fields, module, store, locals, scope, spacerWeight = modifier)
            } else {
                AnimatedContent(
                    targetState = value,
                    transitionSpec = {
                        nexaDevContentTransform(transition, floatAnimation, offsetAnimation)
                    },
                    label = "dev-when-transition",
                ) { target ->
                    val targetLocals = nexaDevTransitionLocals(locals, transitionStateName, rawValue)
                    renderWhenTarget(target, fields, module, store, targetLocals, scope)
                }
            }
        }
        "Link" -> {
            val context = LocalContext.current
            val url = store.stringify(store.evaluatePresented(fields.opt("url"), locals, scope))
            val children = fields.optJSONArray("children") ?: JSONArray()
            NexaLinkPrimitive(onClick = { openNexaUrl(context, url) }) {
                RenderChildren(children, module, store, locals, scope)
            }
        }
        "Accessibility" -> {
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            val hintValue = fields.opt("hint")
            val hint = if (hintValue == null || hintValue == JSONObject.NULL) null else store.stringify(store.evaluatePresented(hintValue, locals, scope))
            val accessibilityValue = fields.opt("value")
            val value = if (accessibilityValue == null || accessibilityValue == JSONObject.NULL) null else store.stringify(store.evaluatePresented(accessibilityValue, locals, scope))
            val children = fields.optJSONArray("children") ?: JSONArray()
            val omitLabel = children.length() == 1 && children.optJSONObject(0)
                ?.optJSONObject("Image")?.optString("description") == label
            val role = fields.optString("role").takeIf { it.isNotEmpty() }
            NexaAccessibilityPrimitive(
                label = label,
                hint = hint,
                value = value,
                role = role,
                isHeading = role == "Header",
                omitLabel = omitLabel,
            ) {
                RenderChildren(children, module, store, locals, scope)
            }
        }
        "KeyboardAware" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            val toolbar = fields.optJSONArray(NexaDevKeys.KEYBOARD_TOOLBAR)
            val interactiveDismiss = fields.optString("dismiss") == "Interactive"
            val avoidIme = fields.optString("behavior") == "Avoid"
            val focusedInputOffset = fields.optDouble(NexaDevKeys.FOCUSED_INPUT_OFFSET, 0.0).toFloat()
            val eventHandler: (JSONObject?) -> ((NexaKeyboardEvent) -> Unit)? = { handler ->
                if (handler == null) {
                    null
                } else {
                    val parameter = handler.optString("parameter")
                    val actions = handler.optJSONArray("actions") ?: JSONArray()
                    val callback: (NexaKeyboardEvent) -> Unit = { event ->
                        val eventLocals = locals + (parameter to mapOf(
                            "height" to event.nexa_field_height,
                            "duration" to event.nexa_field_duration,
                        ))
                        store.perform(actions, scope, eventLocals)
                    }
                    callback
                }
            }
            if (nexaDevContainsScrollable(children)) {
                NexaKeyboardAwareOwnerPrimitive(
                    interactiveDismiss = interactiveDismiss,
                    avoidIme = avoidIme,
                    focusedInputOffset = focusedInputOffset,
                    hasToolbar = toolbar != null,
                    onShow = eventHandler(fields.optJSONObject(NexaDevKeys.KEYBOARD_ON_SHOW)),
                    onHide = eventHandler(fields.optJSONObject(NexaDevKeys.KEYBOARD_ON_HIDE)),
                    toolbar = {
                        if (toolbar != null) RenderChildren(toolbar, module, store, locals, scope)
                    },
                    content = {
                    RenderChildren(children, module, store, locals, scope)
                    },
                )
            } else {
                NexaKeyboardAwareColumnPrimitive(
                    interactiveDismiss = interactiveDismiss,
                    avoidIme = avoidIme,
                    focusedInputOffset = focusedInputOffset,
                    hasToolbar = toolbar != null,
                    onShow = eventHandler(fields.optJSONObject(NexaDevKeys.KEYBOARD_ON_SHOW)),
                    onHide = eventHandler(fields.optJSONObject(NexaDevKeys.KEYBOARD_ON_HIDE)),
                    toolbar = {
                        if (toolbar != null) RenderChildren(toolbar, module, store, locals, scope)
                    },
                    content = {
                    RenderChildren(children, module, store, locals, scope)
                    },
                )
            }
        }
        "AppBottomBar" -> NexaDevRenderAppBottomBarNode(fields, module, store, locals, scope, modifier)
        "PagePager" -> {
            val state = fields.optString("state")
            val pages = fields.optJSONArray("pages") ?: JSONArray()
            val selected = (store.state(state, scope) as? Number)?.toInt() ?: 0
            val pageState = rememberPagerState(initialPage = selected) { pages.length() }
            val pageCoroutineScope = rememberCoroutineScope()
            val swipeRowGestureActive = remember { mutableStateOf(false) }
            LaunchedEffect(pageState.settledPage) { store.setState(state, pageState.settledPage, scope) }
            LaunchedEffect(selected) {
                if (selected in 0 until pages.length() && selected != pageState.settledPage) {
                    pageState.animateScrollToPage(selected)
                }
            }
            Column(modifier = Modifier.fillMaxSize()) {
                HorizontalPager(
                    state = pageState,
                    modifier = Modifier.weight(1f),
                    overscrollEffect = null,
                    userScrollEnabled = !swipeRowGestureActive.value,
                ) { page ->
                    val children = pages.optJSONArray(page) ?: JSONArray()
                    CompositionLocalProvider(LocalNexaFastListSwipePagerLock provides swipeRowGestureActive) {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                            propagateMinConstraints = true,
                        ) {
                            RenderChildren(children, module, store, locals, scope)
                        }
                    }
                }
                NexaPageIndicatorRow(
                    pageCount = pages.length(),
                    currentPage = pageState.settledPage,
                ) { page ->
                    pageCoroutineScope.launch { pageState.animateScrollToPage(page) }
                }
            }
        }
        "Toolbar" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = if (fields.optString("placement") == "Leading") Arrangement.Start else Arrangement.End,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                RenderChildren(children, module, store, locals, scope)
            }
        }
        "BottomSheet" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            val componentRef = fields.opt("component_ref")
                ?.takeIf { it != JSONObject.NULL }
                ?.let { store.evaluatePresented(it, locals, scope) as? NexaBottomSheetRef }
                ?: NexaBottomSheetRef()
            val contentSized = fields.optBoolean("content_sized", false)
            val cornerRadius = (fields.opt("corner_radius") as? Number)?.toFloat()?.dp
            val detents = fields.optJSONArray("detents")?.let { values ->
                (0 until values.length()).map { values.optDouble(it).toFloat() }
            } ?: listOf(0.5f, 1f)
            val defaultDetent = detents.firstOrNull() ?: 0.5f
            val initialDetent = fields.optDouble("initial_detent", defaultDetent.toDouble()).toFloat()
            val backgroundValue = fields.optJSONObject("background_color")
            val backgroundColor = backgroundValue?.let { value ->
                val dynamic = value.opt("Dynamic")
                if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, isSystemInDarkTheme())
                }
            }
            val scrimValue = fields.optJSONObject("scrim_color")
            val scrimColor = scrimValue?.let { value ->
                val dynamic = value.opt("Dynamic")
                if (dynamic != null && dynamic != JSONObject.NULL) {
                    nexaDevHexColor(store.stringify(store.evaluatePresented(dynamic, locals, scope)))
                } else {
                    nexaDevColor(value, isSystemInDarkTheme())
                }
            }
            NexaBottomSheetPrimitive(
                componentRef = componentRef,
                contentSized = contentSized,
                contentPadding = fields.optDouble("content_padding", 0.0).toFloat().dp,
                cornerRadius = cornerRadius,
                detents = detents,
                initialDetent = initialDetent,
                showDragIndicator = fields.optBoolean("show_drag_indicator", true),
                allowsDismissal = fields.optBoolean("allows_dismissal", true),
                allowsBackgroundInteraction = fields.optBoolean("allows_background_interaction", false),
                shouldDismissOnBackPress = fields.optBoolean("should_dismiss_on_back_press", true),
                backgroundColor = backgroundColor,
                scrimColor = scrimColor,
                onDismiss = {
                    store.perform(fields.optJSONArray("on_dismiss") ?: JSONArray(), scope, locals)
                },
            ) { RenderChildren(children, module, store, locals, scope) }
        }
        "Dialog" -> {
            val state = fields.optString("state")
            if (store.state(state, scope) as? Boolean == true) {
                val title = store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope))
                val message = store.stringify(store.evaluatePresented(fields.opt("message"), locals, scope))
                val children = fields.optJSONArray("children") ?: JSONArray()
                val inputs = JSONArray()
                val actions = JSONArray()
                for (index in 0 until children.length()) {
                    val child = children.optJSONObject(index) ?: continue
                    when {
                        child.has("TextInput") -> inputs.put(child)
                        child.has("Button") -> actions.put(child)
                    }
                }
                val dismissActions = JSONArray()
                val confirmActions = JSONArray()
                if (actions.length() == 2) {
                    dismissActions.put(actions.getJSONObject(0))
                    confirmActions.put(actions.getJSONObject(1))
                } else {
                    for (index in 0 until actions.length()) {
                        confirmActions.put(actions.getJSONObject(index))
                    }
                }
                val hasMessage = nexaDevHasDialogMessage(fields.opt("message"))
                NexaAlertDialogPrimitive(
                    onDismissRequest = { store.setState(state, false, scope) },
                    title = { Text(title) },
                    text = {
                        if (inputs.length() == 0) {
                            Text(message)
                        } else {
                            Column {
                                if (hasMessage) Text(message)
                                RenderChildren(inputs, module, store, locals, scope)
                            }
                        }
                    },
                    dismissButton = {
                        RenderChildren(dismissActions, module, store, locals, scope)
                    },
                    confirmButton = {
                        RenderChildren(confirmActions, module, store, locals, scope)
                    },
                )
            }
        }
        "ConfirmationDialog" -> {
            val state = fields.optString("state")
            if (store.state(state, scope) as? Boolean == true) {
                val title = store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope))
                val children = fields.optJSONArray("children") ?: JSONArray()
                NexaConfirmationDialogPrimitive(
                    title = title,
                    onDismissRequest = { store.setState(state, false, scope) },
                ) {
                    for (index in 0 until children.length()) {
                        if (index > 0) NexaDialogActionDividerPrimitive()
                        val child = children.optJSONObject(index) ?: continue
                        NexaDevNode(
                            child, module, store, locals, scope,
                            buttonPresentation = NexaDevButtonPresentation.ConfirmationAction,
                        )
                    }
                }
            }
        }
        "NavigationStack" -> RenderNavigationStack(
            fields,
            module,
            store,
            locals,
            scope,
            modifier = modifier,
        )
        "NavigationSplitView" -> {
            val sidebar = fields.optJSONArray("sidebar") ?: JSONArray()
            val detail = fields.optJSONArray("detail") ?: JSONArray()
            val state = fields.optString("detail_visible")
            NexaNavigationSplitViewPrimitive(
                detailVisible = store.state(state, scope) as? Boolean == true,
                onDetailVisibleChange = { store.setState(state, it, scope) },
                sidebar = { RenderChildren(sidebar, module, store, locals, scope) },
                detail = { RenderChildren(detail, module, store, locals, scope) },
            )
        }
        "NavigationLink" -> {
            val screens = module.optJSONArray("screens") ?: JSONArray()
            val destination = screens.optJSONObject(fields.optInt("destination", -1))
            val children = fields.optJSONArray("children") ?: JSONArray()
            val guard = fields.opt("guard")
            val enabled = guard == null || guard == JSONObject.NULL || store.evaluatePresented(guard, locals, scope) as? Boolean == true
            val staticallyDisabled = (guard as? JSONObject)?.opt("Bool") == false
            if (destination == null || staticallyDisabled) {
                RenderChildren(children, module, store, locals, scope)
            } else {
                val navId = (locals["__nexa_navigation_stack_identity"] as? String)
                    ?: resolveNavigatorIdFromScope(scope)
                val controller = LocalNexaDevNavController.current
                    ?: store.resolveNavController(stackIdentity = navId, locals = locals, scope = scope)
                val route = store.encodeScreenRoute(
                    destination,
                    fields.optJSONArray("arguments") ?: JSONArray(),
                    locals,
                    scope,
                    navigatorId = navId,
                )
                NexaNavigationLinkPrimitive(
                    enabled = enabled,
                    onClick = { controller?.navigate(route) },
                    trailing = {
                        Icon(
                            imageVector = Icons.Filled.ChevronRight,
                            contentDescription = null,
                            modifier = Modifier.size(22.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                        )
                    },
                ) {
                    RenderChildren(children, module, store, locals, scope)
                }
            }
        }
        "NavigationBack" -> {
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            val navId = (locals["__nexa_navigation_stack_identity"] as? String)
                ?: resolveNavigatorIdFromScope(scope)
            val controller = LocalNexaDevNavController.current
                ?: store.resolveNavController(stackIdentity = navId, locals = locals, scope = scope)
            androidx.compose.material3.TextButton(onClick = {
                store.popNavigation(explicitController = controller, navigatorId = navId, scope = scope, locals = locals)
            }) {
                Text(label)
            }
        }
    }
}

internal data class NexaDevToolbarLayout(
    val content: JSONArray,
    val toolbars: List<Pair<String, JSONArray>>,
)

internal fun nexaDevToolbarLayout(nodes: JSONArray): NexaDevToolbarLayout {
    val toolbars = mutableListOf<Pair<String, JSONArray>>()

    fun split(source: JSONArray): JSONArray {
        var filtered: JSONArray? = null
        for (index in 0 until source.length()) {
            val node = source.opt(index)
            val objectNode = node as? JSONObject
            val kind = objectNode?.keys()?.asSequence()?.firstOrNull()
            val fields = kind?.let { objectNode?.optJSONObject(it) }
            val replacement = when (kind) {
                "Toolbar" -> {
                    if (fields != null) {
                        toolbars += fields.optString("placement") to (fields.optJSONArray("children") ?: JSONArray())
                    }
                    null
                }
                "Layout" -> {
                    val children = fields?.optJSONArray("children")
                    val content = children?.let(::split)
                    if (fields != null && content != null && content !== children) {
                        JSONObject().put(kind, fields.nexaDevShallowCopy().put("children", content))
                    } else node
                }
                else -> node
            }
            if (replacement !== node && filtered == null) {
                filtered = JSONArray()
                for (earlier in 0 until index) filtered.put(source.opt(earlier))
            }
            if (replacement != null) filtered?.put(replacement)
        }
        return filtered ?: source
    }

    return NexaDevToolbarLayout(split(nodes), toolbars)
}

private fun JSONObject.nexaDevShallowCopy(): JSONObject = JSONObject().also { copy ->
    keys().forEach { key -> copy.put(key, opt(key)) }
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderPressableNode(
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
        val locals = if (!locals.containsKey("__nexa_navigation_controller")) {
            LocalNexaDevNavController.current?.let { locals + ("__nexa_navigation_controller" to it) } ?: locals
        } else {
            locals
        }
        val disabled = store.evaluatePresented(fields.opt("disabled"), locals, scope) as? Boolean ?: false
        val doubleTapActions = fields.optJSONArray("double_tap_actions") ?: JSONArray()
        val longPressActions = fields.optJSONArray("long_press_actions") ?: JSONArray()
        val contextMenuActions = fields.optJSONArray("context_menu") ?: JSONArray()
        val contextMenuExpanded = if (contextMenuActions.length() > 0) {
            remember(scope) { mutableStateOf(false) }
        } else {
            null
        }
        val dragParameters = fields.optJSONArray(NexaDevKeys.DRAG_PARAMETERS) ?: JSONArray()
        val dragActions = fields.optJSONArray(NexaDevKeys.DRAG_ACTIONS) ?: JSONArray()
        val pinchParameter = fields.nexaDevOptionalString(NexaDevKeys.PINCH_PARAMETER)
            ?.takeIf(String::isNotEmpty)
        val pinchActions = fields.optJSONArray(NexaDevKeys.PINCH_ACTIONS) ?: JSONArray()
        val gestureConsumedTap = if (dragParameters.length() == 4 || pinchParameter != null) {
            remember(scope) { mutableStateOf(false) }
        } else {
            null
        }
        val hapticStyle = fields.nexaDevOptionalString("haptic")?.takeIf(String::isNotEmpty)
        val fillMaxSize = fields.optBoolean("fill_max_size", false)
        val hapticType = when (hapticStyle) {
            "Light" -> HapticFeedbackType.TextHandleMove
            "Medium" -> HapticFeedbackType.LongPress
            "Heavy" -> HapticFeedbackType.ContextClick
            else -> null
        }
        val haptic = if (hapticType != null) LocalHapticFeedback.current else null
        val tapActions = fields.optJSONArray("actions")?.takeIf { it.length() > 0 }
        val dragModifier = if (pinchParameter != null) {
            val density = if (dragParameters.length() == 4) LocalDensity.current.density else 1f
            Modifier.pointerInput(density, !disabled, dragActions, pinchActions) {
                if (!disabled) {
                    var translationX = 0f
                    var translationY = 0f
                    var lastGestureTime = 0L
                    val gestureLocals = locals.toMutableMap()
                    detectTransformGestures { _, pan, zoomChange, _ ->
                        if (dragParameters.length() == 4 && (pan.x != 0f || pan.y != 0f)) {
                            gestureConsumedTap?.value = true
                            translationX += pan.x
                            translationY += pan.y
                            val now = SystemClock.uptimeMillis()
                            val elapsed = if (lastGestureTime == 0L || now - lastGestureTime > 100L) {
                                0L
                            } else {
                                now - lastGestureTime
                            }
                            lastGestureTime = now
                            gestureLocals[dragParameters.getString(0)] =
                                (translationX / density).toDouble()
                            gestureLocals[dragParameters.getString(1)] =
                                (translationY / density).toDouble()
                            gestureLocals[dragParameters.getString(2)] =
                                if (elapsed == 0L) 0.0 else (pan.x * 1000f / elapsed.toFloat() / density).toDouble()
                            gestureLocals[dragParameters.getString(3)] =
                                if (elapsed == 0L) 0.0 else (pan.y * 1000f / elapsed.toFloat() / density).toDouble()
                            store.perform(dragActions, scope, gestureLocals)
                        }
                        if (zoomChange != 1f) {
                            gestureConsumedTap?.value = true
                            gestureLocals[pinchParameter] = zoomChange.toDouble()
                            store.perform(pinchActions, scope, gestureLocals)
                        }
                    }
                }
            }
        } else if (dragParameters.length() == 4) {
            val density = LocalDensity.current.density
            Modifier.pointerInput(density, !disabled, dragActions) {
                if (!disabled) {
                    val velocityTracker = VelocityTracker()
                    var translationX = 0f
                    var translationY = 0f
                    val dragLocals = locals.toMutableMap()
                    detectDragGestures(
                        onDragStart = {
                            translationX = 0f
                            translationY = 0f
                            velocityTracker.resetTracking()
                        },
                        onDragEnd = { velocityTracker.resetTracking() },
                        onDragCancel = { velocityTracker.resetTracking() },
                        onDrag = { change, dragAmount ->
                            gestureConsumedTap?.value = true
                            translationX += dragAmount.x
                            translationY += dragAmount.y
                            velocityTracker.addPointerInputChange(change)
                            val velocity = velocityTracker.calculateVelocity()
                            dragLocals[dragParameters.getString(0)] =
                                (translationX / density).toDouble()
                            dragLocals[dragParameters.getString(1)] =
                                (translationY / density).toDouble()
                            dragLocals[dragParameters.getString(2)] =
                                (velocity.x / density).toDouble()
                            dragLocals[dragParameters.getString(3)] =
                                (velocity.y / density).toDouble()
                            store.perform(dragActions, scope, dragLocals)
                            change.consume()
                        },
                    )
                }
            }
        } else {
            Modifier
        }
        val pressableModifier = (if (fillMaxSize) modifier.then(dragModifier).fillMaxSize() else modifier.then(dragModifier))
            .defaultMinSize(minHeight = 48.dp)
            .alpha(if (disabled) 0.5f else 1f)
        val hasLongPress = longPressActions.length() > 0 || contextMenuActions.length() > 0
        val hasDoubleTap = doubleTapActions.length() > 0
        val pressableCore: @Composable () -> Unit = {
            val onTap: () -> Unit = {
                if (gestureConsumedTap?.value == true) {
                    gestureConsumedTap.value = false
                } else {
                    if (hapticType != null) haptic?.performHapticFeedback(hapticType)
                    if (tapActions != null) store.perform(tapActions, scope, locals)
                }
            }
            val clickModifier = if (!hasLongPress && !hasDoubleTap) {
                pressableModifier.clickable(
                    enabled = !disabled,
                    interactionSource = null,
                    indication = null,
                    onClick = onTap,
                )
            } else {
                pressableModifier.combinedClickable(
                    enabled = !disabled,
                    interactionSource = null,
                    onClick = onTap,
                    onLongClick = if (hasLongPress) {
                        {
                            contextMenuExpanded?.value = true
                            store.perform(longPressActions, scope, locals)
                        }
                    } else null,
                    onDoubleClick = if (hasDoubleTap) {
                        {
                            if (hapticType != null) haptic?.performHapticFeedback(hapticType)
                            store.perform(doubleTapActions, scope, locals)
                        }
                    } else null,
                )
            }
            Box(
                modifier = clickModifier,
                contentAlignment = if (fillMaxSize) Alignment.Center else Alignment.TopStart,
            ) {
                RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
            }
        }
        val pressableContent: @Composable () -> Unit = {
            if (contextMenuActions.length() == 0) {
                pressableCore()
            } else {
                Box {
                    pressableCore()
                    androidx.compose.material3.MaterialExpressiveTheme(
                        colorScheme = MaterialTheme.colorScheme,
                    ) {
                        DropdownMenu(
                            expanded = contextMenuExpanded?.value == true,
                            onDismissRequest = { contextMenuExpanded?.value = false },
                            shape = androidx.compose.material3.MenuDefaults.standaloneGroupShape,
                            containerColor = androidx.compose.material3.MenuDefaults.groupStandardContainerColor,
                            tonalElevation = 0.dp,
                            shadowElevation = 8.dp,
                        ) {
                            val menuButtons = buildList {
                                for (index in 0 until contextMenuActions.length()) {
                                    val menuNode = contextMenuActions.optJSONObject(index) ?: continue
                                    val menuButton = menuNode.optJSONObject("Button") ?: continue
                                    add(menuButton)
                                }
                            }
                            if (menuButtons.isNotEmpty()) {
                                menuButtons.forEachIndexed { itemIndex, menuButton ->
                                    val label = store.stringify(
                                        store.evaluatePresented(menuButton.opt("label"), locals, scope),
                                    )
                                    val disabledAction = store.evaluatePresented(
                                        menuButton.opt("disabled"), locals, scope,
                                    ) as? Boolean ?: false
                                    val iconSelection = menuButton.optJSONObject("icon") ?: JSONObject()
                                    val itemIcon = nexaDevIconSpec(iconSelection, store, locals, scope)
                                    DropdownMenuItem(
                                        shape = androidx.compose.material3.MenuDefaults.itemShape(
                                            index = itemIndex,
                                            count = menuButtons.size,
                                        ).shape,
                                        text = { Text(label) },
                                        leadingIcon = itemIcon?.let { spec ->
                                            { NexaIconContent(icon = spec, description = "", size = 24.dp) }
                                        },
                                        enabled = !disabledAction,
                                        onClick = {
                                            store.perform(menuButton.optJSONArray("actions") ?: JSONArray(), scope, locals)
                                            contextMenuExpanded?.value = false
                                        },
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
        if (longPressActions.length() == 0) {
            pressableContent()
        } else {
            val longPressDurationMs = (store.evaluatePresented(
                fields.opt(NexaDevKeys.LONG_PRESS_DURATION_MS),
                locals,
                scope,
            ) as? Number)?.toLong()?.coerceAtLeast(1L) ?: 500L
            val baseViewConfiguration = LocalViewConfiguration.current
            val longPressViewConfiguration = remember(baseViewConfiguration, longPressDurationMs) {
                object : ViewConfiguration by baseViewConfiguration {
                    override val longPressTimeoutMillis: Long = longPressDurationMs
                }
            }
            CompositionLocalProvider(LocalViewConfiguration provides longPressViewConfiguration) {
                pressableContent()
            }
        }

}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderTextInputNode(
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
        val state = fields.optString("state")
        val value = store.stringify(store.state(state, scope))
        val sourcePlaceholder = fields.optString("placeholder")
        val placeholder = sourcePlaceholder.takeIf(String::isNotEmpty)
            ?.let { store.localizedText(it, it) }
        val initialFocus = store.evaluatePresented(fields.opt("focused"), locals, scope) as? Boolean ?: false
        val componentRef = fields.opt("component_ref")
            ?.takeIf { it != JSONObject.NULL }
            ?.let { store.evaluatePresented(it, locals, scope) as? NexaTextInputRef }
        val keyboardType = when (fields.optString("keyboard")) {
            "Number" -> KeyboardType.Number
            "Email" -> KeyboardType.Email
            "Phone" -> KeyboardType.Phone
            "Url" -> KeyboardType.Uri
            else -> KeyboardType.Text
        }
        val capitalization = when (fields.optString("capitalization")) {
            "None" -> KeyboardCapitalization.None
            "Words" -> KeyboardCapitalization.Words
            "Characters" -> KeyboardCapitalization.Characters
            else -> KeyboardCapitalization.Sentences
        }
        val autofill = fields.optString("autofill")
        val submitActions = fields.optJSONArray("actions") ?: JSONArray()
        val imeAction = when (fields.optString("return_key")) {
            "Done" -> ImeAction.Done
            "Search" -> ImeAction.Search
            "Send" -> ImeAction.Send
            "Next" -> ImeAction.Next
            else -> if (submitActions.length() > 0) ImeAction.Done else ImeAction.Default
        }
        val maxLength = fields.optInt("max_length", Int.MAX_VALUE)
        val minLines = fields.optInt("min_lines", 1).coerceAtLeast(1)
        val maxLines = fields.optInt("max_lines", Int.MAX_VALUE).coerceAtLeast(minLines)
        val baseTextStyle = when (fields.optString("font")) {
            "Title3" -> MaterialTheme.typography.titleMedium.copy(fontSize = 20.sp)
            "Body" -> MaterialTheme.typography.bodyLarge.copy(fontSize = 17.sp)
            else -> LocalTextStyle.current.copy(fontSize = 17.sp)
        }
        val inputFontSize = if (fields.has("font_size") && !fields.isNull("font_size")) {
            fields.optDouble("font_size").toFloat().sp
        } else {
            baseTextStyle.fontSize
        }
        val inputFontName = fields.nexaDevOptionalString("font_family")?.takeIf(String::isNotBlank)
        val inputFontFamily = inputFontName?.let { nexaDevFontFamily(it, FontWeight.Normal) }
        val textStyle = baseTextStyle.copy(
            fontSize = inputFontSize,
            fontFamily = inputFontFamily ?: baseTextStyle.fontFamily,
        )
        val textColor = nexaDevColor(fields.optJSONObject("text_color"), isSystemInDarkTheme())
        val placeholderColor = nexaDevColor(fields.optJSONObject("placeholder_color"), isSystemInDarkTheme())
        val onChange = fields.optJSONObject("on_change")
        val onFocus = fields.optJSONArray("on_focus") ?: JSONArray()
        val keyboardController = androidx.compose.ui.platform.LocalSoftwareKeyboardController.current
        val focusManager = LocalFocusManager.current
        var fieldModifier: Modifier = Modifier
        if (onFocus.length() > 0) {
            fieldModifier = fieldModifier.onFocusChanged { focusState ->
                if (focusState.isFocused) {
                    store.perform(onFocus, scope, locals)
                }
            }
        }
        fieldModifier = when (autofill) {
            "Username" -> fieldModifier.semantics { contentType = ContentType.Username }
            "Password" -> fieldModifier.semantics { contentType = ContentType.Password }
            "OneTimeCode" -> fieldModifier.semantics { contentType = ContentType.SmsOtpCode }
            else -> fieldModifier
        }
        NexaTextInputPrimitive(
            value = value,
            initialFocus = initialFocus,
            componentRef = componentRef,
            onValueChange = { next ->
                val updated = next
                val previous = store.stringify(store.state(state, scope))
                if (updated != previous) {
                    store.setState(state, updated, scope)
                    val parameter = onChange?.nexaDevOptionalString("parameter")
                        ?.takeIf(String::isNotEmpty)
                    if (parameter != null) {
                        val changeLocals = locals.toMutableMap()
                        changeLocals[parameter] = updated
                        store.perform(onChange.optJSONArray("actions") ?: JSONArray(), scope, changeLocals)
                    }
                }
            },
            modifier = modifier,
            focusModifier = fieldModifier,
            placeholder = placeholder.orEmpty(),
            searchable = fields.optBoolean("searchable"),
            clearable = fields.optBoolean("clearable"),
            searchIcon = if (fields.optBoolean("searchable")) nexaDevSharedMaterialIcon("search") else null,
            textStyle = textStyle,
            textColor = textColor,
            placeholderColor = placeholderColor,
            maxLength = maxLength,
            minLines = minLines,
            maxLines = maxLines,
            singleLine = !fields.optBoolean("multiline"),
            secure = fields.optBoolean("secure"),
            keyboardOptions = KeyboardOptions(
                keyboardType = keyboardType,
                capitalization = capitalization,
                autoCorrectEnabled = fields.optBoolean("autocorrect", true),
                imeAction = imeAction,
            ),
            keyboardActions = when (imeAction) {
                ImeAction.Search -> KeyboardActions(onSearch = {
                    if (submitActions.length() > 0) store.perform(submitActions, scope, locals)
                    keyboardController?.hide()
                    focusManager.clearFocus()
                })
                ImeAction.Send -> KeyboardActions(onSend = {
                    if (submitActions.length() > 0) store.perform(submitActions, scope, locals)
                    keyboardController?.hide()
                    focusManager.clearFocus()
                })
                ImeAction.Next -> if (submitActions.length() == 0) KeyboardActions() else KeyboardActions(onNext = {
                    store.perform(submitActions, scope, locals)
                })
                else -> KeyboardActions(onDone = {
                    if (submitActions.length() > 0) store.perform(submitActions, scope, locals)
                    keyboardController?.hide()
                    focusManager.clearFocus()
                })
            },
        )

}

@Composable
private fun NexaDevBindFastListRef(
    reference: NexaFastListRef?,
    count: Int,
    reverseLayout: Boolean,
    scrollToIndex: suspend (Int, Boolean, Double) -> Unit,
    scrollToOffset: suspend (Double, Boolean) -> Unit,
) {
    if (reference == null) return
    NexaBindFastListRef(
        reference = reference,
        scrollToIndex = { index, animated, viewPosition ->
            if (count > 0) scrollToIndex(index.coerceIn(0, count - 1), animated, viewPosition)
        },
        scrollToTop = { animated ->
            if (count > 0) scrollToOffset(0.0, animated)
        },
        scrollToEnd = { animated ->
            if (count > 0) scrollToIndex(if (reverseLayout) 0 else count - 1, animated, 1.0)
        },
        scrollToOffset = scrollToOffset,
    )
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderFastListNode(
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
        val plan = fields.optJSONObject("plan")
        val countPlan = plan?.optJSONObject("Count")
        val itemPlan = plan?.optJSONObject("Items")
        val sectionPlan = plan?.optJSONObject("Sections")
        val listPlan = countPlan ?: itemPlan ?: sectionPlan
        val options = listPlan?.optJSONObject("common")
        val countExpression = countPlan?.opt("count")
        val itemExpression = itemPlan?.opt("collection")
        val sourceItems = itemExpression?.let { store.evaluatePresented(it, locals, scope) as? List<*> }
        val sectionExpression = sectionPlan?.opt("collection")
        val sourceSections = sectionExpression?.let { store.evaluatePresented(it, locals, scope) as? List<*> }
        val axisValue = options?.opt("axis")
        val gridColumns = (axisValue as? JSONObject)
            ?.optJSONObject("Grid")
            ?.optInt("columns", 0)
            ?.takeIf { it > 0 }
        val minGridCellWidth = (axisValue as? JSONObject)
            ?.optJSONObject("AutoGrid")
            ?.optInt("min_cell_width", 0)
            ?.takeIf { it > 0 }
        val masonryColumns = (axisValue as? JSONObject)
            ?.optJSONObject("Masonry")
            ?.optInt("columns", 0)
            ?.takeIf { it > 0 }
        val axis = when {
            axisValue == "Horizontal" -> "Horizontal"
            masonryColumns != null -> "Masonry"
            minGridCellWidth != null -> "AdaptiveGrid"
            gridColumns != null -> "Grid"
            else -> "Vertical"
        }
        val viewportAxis = if (axis == "Horizontal") "Horizontal" else "Vertical"
        val scrollViewportModifier = nexaSafeAreaScrollViewportModifier(viewportAxis)
        val reverseLayout = options?.optBoolean(NexaDevKeys.REVERSE_LAYOUT, false) ?: false
        val pageSnap = options?.optBoolean(NexaDevKeys.PAGE_SNAP, false) ?: false
        val scrollEnabled = options?.opt("scroll_enabled")
            ?.takeUnless { it == JSONObject.NULL }
            ?.let { store.evaluatePresented(it, locals, scope) as? Boolean }
            ?: true
        val showDividers = options?.optBoolean("show_dividers", true) ?: true
        val onScrollBeginDrag = options?.optJSONArray("on_scroll_begin_drag")
        val keyExpression = options?.opt("key")?.takeUnless { it == JSONObject.NULL }
        val animateStableRows = keyExpression != null
        if (options == null || countExpression == null && sourceItems == null && sourceSections == null) {
            Text("FastList source could not be evaluated.")
        } else {
            val itemCount = countExpression?.let {
                (store.evaluatePresented(it, locals, scope) as? Number)?.toInt()?.coerceAtLeast(0) ?: 0
            } ?: sourceItems?.size ?: 0
            val indexName = options.optString("index", "index")
            val itemName = listPlan.nexaDevOptionalString("item")?.takeIf(String::isNotEmpty)
            val sectionName = listPlan.nexaDevOptionalString("section")?.takeIf(String::isNotEmpty)
            fun listRowLocals(itemIndex: Int, sectionIndex: Int? = null): Map<String, Any> {
                val rowLocals = locals.toMutableMap()
                rowLocals[indexName] = itemIndex
                if (sectionIndex != null) {
                    val sectionItems = sourceSections?.getOrNull(sectionIndex) as? List<*>
                    if (itemName != null) {
                        rowLocals[itemName] = sectionItems?.getOrNull(itemIndex) ?: JSONObject.NULL
                    }
                    rowLocals[sectionName ?: "section"] = sectionIndex
                } else if (itemName != null && sourceItems != null) {
                    rowLocals[itemName] = sourceItems.getOrNull(itemIndex) ?: JSONObject.NULL
                }
                return rowLocals
            }
            fun stableRowKey(itemIndex: Int, sectionIndex: Int? = null): Any {
                val expression = keyExpression
                if (expression == null) {
                    return if (sectionIndex == null) itemIndex else "$sectionIndex:$itemIndex"
                }
                val value = store.evaluate(expression, listRowLocals(itemIndex, sectionIndex), scope)
                return if (value == JSONObject.NULL) itemIndex else value
            }
            val children = options.optJSONArray("children") ?: JSONArray()
            val componentRef = options.opt("component_ref")
                ?.takeIf { it != JSONObject.NULL }
                ?.let { store.evaluatePresented(it, locals, scope) as? NexaFastListRef }
            val swipeConfig = options.optJSONObject("swipe_actions")
            val swipeActions = swipeConfig?.optJSONArray("children")
            val swipeEdge = if (swipeConfig?.optString("edge") == "Leading") {
                NexaFastListSwipeEdge.Leading
            } else {
                NexaFastListSwipeEdge.Trailing
            }
            val swipeAllowsFullSwipe = swipeConfig?.optBoolean("allows_full_swipe", false) == true
            // Sectioned rows are installed one interval at a time. Only
            // refs and swipe resets need a flattened row count.
            val count = if (sourceSections != null && (componentRef != null || swipeActions != null)) {
                sourceSections.sumOf { (it as? List<*>)?.size ?: 0 }
            } else {
                itemCount
            }
            val stickyHeaderNodes = options.optJSONArray("sticky_header")
            val sectionHeader = options.optJSONArray("section_header")
            val onScroll = options.optJSONArray("on_scroll")
            val onEndReached = options.optJSONArray("on_end_reached")
            val listTaskOwner = if (onEndReached != null) {
                remember(fields, scope) { "$scope/fast-list/${java.util.UUID.randomUUID()}" }
            } else {
                null
            }
            if (listTaskOwner != null) {
                DisposableEffect(listTaskOwner) {
                    onDispose { store.clearNativeTasksOwnedBy(listTaskOwner) }
                }
            }
            val onMove = options.optJSONObject("on_move")
            val usesPlatformList = onMove != null || swipeActions != null
            val moveEnabled = onMove?.opt("enabled")?.let {
                store.evaluatePresented(it, locals, scope) as? Boolean
            } ?: true
            val refresh = options.optJSONObject("refresh")
            val refreshState = refresh?.nexaDevOptionalString("state")?.takeIf(String::isNotEmpty)
            val refreshActions = refresh?.optJSONArray("actions") ?: JSONArray()
            var lastReportedScrollPosition by remember { mutableIntStateOf(-1) }
            val scrollPositionName = options.nexaDevOptionalString("scroll_position")
                ?.takeIf(String::isNotEmpty)
            val scrollPositionCommandRevision = scrollPositionName?.let {
                store.stateCommandRevision(it, scope)
            } ?: 0
            val stickyHeaderOffset = if (axis == "Vertical" && stickyHeaderNodes != null && stickyHeaderNodes != JSONObject.NULL) 1 else 0
            val requestedIndex = scrollPositionName?.let { name ->
                (store.state(name, scope) as? Number)?.toInt()?.coerceIn(0, maxOf(0, count - 1)) ?: 0
            } ?: 0
            val requestedLayoutPosition = if (stickyHeaderOffset > 0 && requestedIndex == 0) {
                0
            } else {
                (requestedIndex + stickyHeaderOffset)
                    .coerceIn(0, maxOf(0, count + stickyHeaderOffset - 1))
            }
            val extent = options.optDouble("item_extent", 0.0).takeIf { it > 0.0 }?.dp
            val gap = options.optDouble("gap", 0.0).coerceAtLeast(0.0).dp
            val renderListRow: @Composable (JSONArray, Map<String, Any>, Modifier) -> Unit = { rowNodes, rowLocals, rowModifier ->
                val rowContent: @Composable () -> Unit = {
                    NexaFastListRowPrimitive(
                        modifier = rowModifier,
                        horizontalAlignment = if (
                            rowNodes.length() > 1 ||
                            axis == "Grid" || axis == "AdaptiveGrid" || axis == "Masonry"
                        ) {
                            Alignment.CenterHorizontally
                        } else {
                            Alignment.Start
                        },
                        verticalArrangement = if (pageSnap && axis == "Vertical") {
                            Arrangement.Top
                        } else {
                            Arrangement.Center
                        },
                    ) {
                        RenderChildren(rowNodes, module, store, rowLocals, scope)
                    }
                }
                if (swipeActions != null) {
                    NexaFastListSwipePrimitive(
                        resetKey = count,
                        edge = swipeEdge,
                        allowsFullSwipe = swipeAllowsFullSwipe,
                        onFullSwipe = if (swipeAllowsFullSwipe) {
                            val action = swipeActions.optJSONObject(0)
                                ?.optJSONObject("Button")
                            val disabled = action?.opt("disabled")?.let {
                                store.evaluatePresented(it, rowLocals, scope) as? Boolean
                            } ?: false
                            val loading = action?.opt("loading")?.let {
                                store.evaluatePresented(it, rowLocals, scope) as? Boolean
                            } ?: false
                            if (action != null && !disabled && !loading) {
                                {
                                    store.perform(
                                        action.optJSONArray("actions") ?: JSONArray(),
                                        scope,
                                        rowLocals,
                                    )
                                }
                            } else null
                        } else null,
                        backgroundContent = {
                            RenderChildren(swipeActions, module, store, rowLocals, scope)
                        },
                        content = rowContent,
                    )
                } else {
                    rowContent()
                }
            }
            val reorderState = if (onMove != null) {
                rememberNexaFastListReorderState()
            } else {
                null
            }
            val renderItem: @Composable (
                Int,
                String,
                Modifier,
                androidx.compose.foundation.lazy.LazyListState?,
                LazyStaggeredGridState?,
                Int,
            ) -> Unit = { itemIndex, itemAxis, itemParentModifier, itemListState, itemStaggeredGridState, itemDataIndex ->
                val dataIndex = itemDataIndex
                val rowLocals = listRowLocals(dataIndex)
                val itemModifier = if (pageSnap && itemAxis == "Vertical") {
                    Modifier
                } else when (itemAxis) {
                    // rowHeight is the cross-axis extent; let horizontal row content set its width.
                    "Horizontal" -> extent?.let { Modifier.height(it) } ?: Modifier
                    "Grid" -> extent?.let { Modifier.fillMaxWidth().height(it) } ?: Modifier
                    else -> extent?.let { Modifier.fillMaxWidth().height(it) } ?: Modifier
                }
                val reorderableItem = onMove != null &&
                    (itemAxis == "Vertical" || itemAxis == "Masonry")
                val rowModifier = (if (reorderableItem) Modifier else itemParentModifier)
                    .then(itemModifier)
                val moveCallback = onMove
                val performMove: (Int, Int) -> Unit = { from, to ->
                    if (moveCallback != null) {
                        val callbackLocals = locals +
                            (moveCallback.optString("from") to from) +
                            (moveCallback.optString("to") to to)
                        store.perform(
                            moveCallback.optJSONArray("actions") ?: JSONArray(),
                            scope,
                            callbackLocals,
                        )
                    }
                }
                if (reorderableItem && itemAxis == "Vertical" && reorderState != null && itemListState != null && moveCallback != null) {
                    NexaFastListReorderableItem(
                        modifier = itemParentModifier,
                        index = itemIndex,
                        itemCount = count,
                        listState = itemListState,
                        reorderState = reorderState,
                        enabled = moveEnabled,
                        listIndexOffset = stickyHeaderOffset,
                        onMove = performMove,
                    ) {
                        NexaNativeListRowPrimitive(position = itemIndex, count = count) {
                            renderListRow(children, rowLocals, itemModifier)
                            if (showDividers && itemIndex + 1 < count) {
                                NexaNativeListDividerPrimitive()
                            }
                        }
                    }
                } else if (reorderableItem && itemAxis == "Masonry" && reorderState != null && itemStaggeredGridState != null && moveCallback != null) {
                    NexaFastStaggeredGridReorderableItem(
                        modifier = itemParentModifier,
                        index = itemIndex,
                        itemCount = count,
                        listState = itemStaggeredGridState,
                        reorderState = reorderState,
                        enabled = moveEnabled,
                        onMove = performMove,
                    ) {
                        renderListRow(children, rowLocals, itemModifier)
                    }
                } else {
                    renderListRow(children, rowLocals, rowModifier)
                }
            }
            val renderHeader: @Composable (JSONArray, Map<String, Any>) -> Unit = { nodes, headerLocals ->
                Column(Modifier.fillMaxWidth()) {
                    RenderChildren(nodes, module, store, headerLocals, scope)
                }
            }
            val observePosition: ((Int) -> Unit)? = if (scrollPositionName != null || onScroll != null) {
                { firstVisible ->
                    val logicalFirstVisible = (firstVisible - stickyHeaderOffset)
                        .coerceIn(0, maxOf(0, count - 1))
                    if (logicalFirstVisible != lastReportedScrollPosition) {
                        val isInitialPosition = lastReportedScrollPosition < 0
                        lastReportedScrollPosition = logicalFirstVisible
                        if (scrollPositionName != null) {
                            val storedIndex = (store.state(scrollPositionName, scope) as? Number)?.toInt()
                            if (storedIndex != logicalFirstVisible) {
                                store.setScrollPositionFromNative(
                                    scrollPositionName,
                                    logicalFirstVisible,
                                    scope,
                                    expectedCommandRevision = scrollPositionCommandRevision,
                                )
                            }
                        }
                        val commandIsCurrent = scrollPositionName == null ||
                            store.stateCommandRevision(scrollPositionName, scope) == scrollPositionCommandRevision
                        if (!isInitialPosition && onScroll != null && commandIsCurrent) {
                            store.perform(onScroll, scope, locals)
                        }
                    }
                }
            } else null
            val observeEndReached: (() -> Unit)? = if (onEndReached != null) {
                {
                    val taskOwnedLocals = locals.toMutableMap()
                    listTaskOwner?.let { taskOwnedLocals[NEXA_DEV_TASK_OWNER_LOCAL] = it }
                    store.perform(onEndReached, scope, taskOwnedLocals)
                }
            } else null
            val density = LocalDensity.current
            val listContent: @Composable () -> Unit = {
                if (pageSnap) {
                    val pager = rememberPagerState(initialPage = requestedIndex) { count }
                    NexaDevBindFastListRef(
                        reference = componentRef,
                        count = count,
                        reverseLayout = false,
                        scrollToIndex = { target, animated, _ ->
                            if (animated) pager.animateScrollToPage(target) else pager.scrollToPage(target)
                        },
                        scrollToOffset = { offset, animated ->
                            if (count > 0) {
                                val pixels = with(density) { offset.dp.roundToPx() }
                                val target = (pixels / pager.layoutInfo.viewportSize.height.coerceAtLeast(1))
                                    .coerceIn(0, count - 1)
                                if (animated) pager.animateScrollToPage(target) else pager.scrollToPage(target)
                            }
                        },
                    )
                    NexaDevObserveFastList(
                        listState = pager,
                        count = count,
                        settledOnly = true,
                        readLeadingPosition = { pager.settledPage },
                        endIsVisible = { pager.settledPage == count - 1 },
                        onPosition = observePosition,
                        onEndReached = observeEndReached,
                    )
                    if (scrollPositionName != null) {
                        LaunchedEffect(pager, scrollPositionCommandRevision, count) {
                            if (count > 0 && pager.settledPage != requestedIndex) {
                                pager.scrollToPage(requestedIndex)
                            }
                        }
                    }
                    VerticalPager(
                        state = pager,
                        contentPadding = LocalNexaScrollContentPadding.current,
                        modifier = modifier.then(scrollViewportModifier).fillMaxSize(),
                        userScrollEnabled = scrollEnabled &&
                            (refreshState != null || pager.canScrollForward || pager.canScrollBackward),
                        flingBehavior = PagerDefaults.flingBehavior(state = pager, pagerSnapDistance = PagerSnapDistance.atMost(1)),
                        key = if (keyExpression != null) ({ index: Int -> stableRowKey(index) }) else null,
                    ) { index ->
                        renderItem(index, "Vertical", Modifier.fillMaxSize(), null, null, index)
                    }
                } else if (axis == "Grid" || axis == "AdaptiveGrid" || axis == "Masonry") {
                    if (axis == "Masonry") {
                        val listState = rememberLazyStaggeredGridState(initialFirstVisibleItemIndex = requestedIndex)
                        NexaDevBindFastListRef(
                            reference = componentRef,
                            count = count,
                            reverseLayout = reverseLayout,
                            scrollToIndex = { target, animated, viewPosition ->
                                if (animated) listState.animateScrollToItem(target) else listState.scrollToItem(target)
                                val layout = listState.layoutInfo
                                val item = layout.visibleItemsInfo.firstOrNull { it.index == target }
                                if (item != null) {
                                    val available = (layout.viewportEndOffset - layout.viewportStartOffset - item.size.height).coerceAtLeast(0)
                                    val desired = layout.viewportStartOffset + (available * viewPosition).toInt()
                                    val delta = (item.offset.y - desired).toFloat()
                                    if (delta != 0f) {
                                        if (animated) listState.animateScrollBy(delta) else listState.scrollBy(delta)
                                    }
                                }
                            },
                            scrollToOffset = { target, animated ->
                                val offset = with(density) { target.dp.roundToPx() }
                                if (animated) listState.animateScrollToItem(0, offset) else listState.scrollToItem(0, offset)
                            },
                        )
                        NexaDevObserveFastList(
                            listState = listState,
                            count = count,
                            readLeadingPosition = { listState.firstVisibleItemIndex },
                            endIsVisible = {
                                val layout = listState.layoutInfo
                                nexaListEndIsVisible(
                                    layout.visibleItemsInfo, count - 1, false,
                                    layout.viewportStartOffset, layout.viewportEndOffset,
                                    itemIndex = { it.index }, itemOffset = { it.offset.y }, itemSize = { it.size.height },
                                )
                            },
                            onPosition = observePosition,
                            onEndReached = observeEndReached,
                        )
                        if (scrollPositionName != null) {
                            LaunchedEffect(listState, scrollPositionCommandRevision, count) {
                                if (count > 0 && listState.firstVisibleItemIndex != requestedIndex) {
                                    listState.scrollToItem(requestedIndex)
                                }
                            }
                        }
                        LazyVerticalStaggeredGrid(
                            columns = StaggeredGridCells.Fixed(masonryColumns ?: 1),
                            contentPadding = LocalNexaScrollContentPadding.current,
                            horizontalArrangement = Arrangement.spacedBy(gap),
                            verticalItemSpacing = gap,
                            state = listState,
                            userScrollEnabled = scrollEnabled &&
                                (refreshState != null || listState.canScrollForward || listState.canScrollBackward),
                            modifier = modifier.then(scrollViewportModifier).fillMaxWidth(),
                        ) {
                            items(
                                count = count,
                                key = { itemIndex ->
                                    stableRowKey(reorderState?.dataIndexForPosition(itemIndex) ?: itemIndex)
                                },
                            ) { itemIndex ->
                                val dataIndex = reorderState?.dataIndexForPosition(itemIndex) ?: itemIndex
                                val rowParentModifier = if (animateStableRows) {
                                    Modifier.fillMaxWidth().animateItem(
                                        placementSpec = if (reorderState?.activeIndex?.intValue != itemIndex) {
                                            androidx.compose.animation.core.spring<androidx.compose.ui.unit.IntOffset>()
                                        } else {
                                            null
                                        },
                                    )
                                } else {
                                    Modifier
                                }
                                renderItem(itemIndex, axis, rowParentModifier, null, listState, dataIndex)
                            }
                        }
                    } else {
                    val listState = rememberLazyGridState(initialFirstVisibleItemIndex = requestedIndex)
                    NexaDevBindFastListRef(
                        reference = componentRef,
                        count = count,
                        reverseLayout = reverseLayout,
                        scrollToIndex = { target, animated, viewPosition ->
                            if (animated) listState.animateScrollToItem(target) else listState.scrollToItem(target)
                            val layout = listState.layoutInfo
                            val item = layout.visibleItemsInfo.firstOrNull { it.index == target }
                            if (item != null) {
                                val available = (layout.viewportEndOffset - layout.viewportStartOffset - item.size.height).coerceAtLeast(0)
                                val desired = layout.viewportStartOffset + (available * viewPosition).toInt()
                                val delta = (item.offset.y - desired).toFloat()
                                if (delta != 0f) {
                                    if (animated) listState.animateScrollBy(delta) else listState.scrollBy(delta)
                                }
                            }
                        },
                        scrollToOffset = { target, animated ->
                            val offset = with(density) { target.dp.roundToPx() }
                            if (animated) listState.animateScrollToItem(0, offset) else listState.scrollToItem(0, offset)
                        },
                    )
                    NexaDevObserveFastList(
                        listState = listState,
                        count = count,
                        readLeadingPosition = { listState.firstVisibleItemIndex },
                        endIsVisible = {
                            val layout = listState.layoutInfo
                            nexaListEndIsVisible(
                                layout.visibleItemsInfo, count - 1, false,
                                layout.viewportStartOffset, layout.viewportEndOffset,
                                itemIndex = { it.index }, itemOffset = { it.offset.y }, itemSize = { it.size.height },
                            )
                        },
                        onPosition = observePosition,
                        onEndReached = observeEndReached,
                    )
                    if (scrollPositionName != null) {
                        LaunchedEffect(listState, scrollPositionCommandRevision, count) {
                            if (count > 0 && listState.firstVisibleItemIndex != requestedIndex) {
                                listState.scrollToItem(requestedIndex)
                            }
                        }
                    }
                    LazyVerticalGrid(
                        columns = minGridCellWidth?.let { GridCells.Adaptive(minSize = it.dp) }
                            ?: GridCells.Fixed(gridColumns ?: 1),
                        contentPadding = LocalNexaScrollContentPadding.current,
                        horizontalArrangement = Arrangement.spacedBy(gap),
                        verticalArrangement = Arrangement.spacedBy(gap),
                        state = listState,
                        userScrollEnabled = scrollEnabled &&
                            (refreshState != null || listState.canScrollForward || listState.canScrollBackward),
                        modifier = modifier
                            .then(nexaSafeAreaScrollViewportModifier("Both"))
                            .fillMaxWidth(),
                    ) {
                        items(count = count, key = { itemIndex -> stableRowKey(itemIndex) }) { itemIndex ->
                            renderItem(itemIndex, axis, Modifier, null, null, itemIndex)
                        }
                    }
                    }
                } else {
                    val listState = rememberLazyListState(initialFirstVisibleItemIndex = requestedLayoutPosition)
                    if (axis == "Vertical" && !reverseLayout && !pageSnap && onScrollBeginDrag != null) {
                        val listIsDragged = listState.interactionSource.collectIsDraggedAsState()
                        LaunchedEffect(listIsDragged.value) {
                            if (listIsDragged.value) {
                                store.perform(onScrollBeginDrag, scope, locals)
                            }
                        }
                    }
                    NexaDevBindFastListRef(
                        reference = componentRef,
                        count = count,
                        reverseLayout = reverseLayout,
                        scrollToIndex = { target, animated, viewPosition ->
                            val layoutIndex = if (sourceSections != null) {
                                nexaSectionedListLayoutIndex(
                                    sourceSections, target, sectionHeader != null, usesPlatformList,
                                ) { (it as? List<*>)?.size ?: 0 }
                            } else if (stickyHeaderOffset > 0 && target != 0) {
                                target + stickyHeaderOffset
                            } else {
                                target
                            }
                            val clampedIndex = layoutIndex.coerceAtLeast(0)
                            if (animated) listState.animateScrollToItem(clampedIndex) else listState.scrollToItem(clampedIndex)
                            val layout = listState.layoutInfo
                            val item = layout.visibleItemsInfo.firstOrNull { it.index == clampedIndex }
                            if (item != null) {
                                val viewportStart = layout.viewportStartOffset
                                val viewportEnd = layout.viewportEndOffset
                                val available = (viewportEnd - viewportStart - item.size).coerceAtLeast(0)
                                val logicalPosition = if (reverseLayout) 1.0 - viewPosition else viewPosition
                                val desired = viewportStart + (available * logicalPosition).toInt()
                                val delta = (item.offset - desired).toFloat()
                                if (delta != 0f) {
                                    if (animated) listState.animateScrollBy(delta) else listState.scrollBy(delta)
                                }
                            }
                        },
                        scrollToOffset = { target, animated ->
                            val offset = with(density) { target.dp.roundToPx() }
                            if (reverseLayout) {
                                nexaScrollReversedListFromTop(listState, count, offset, animated)
                            } else if (animated) {
                                listState.animateScrollToItem(0, offset)
                            } else {
                                listState.scrollToItem(0, offset)
                            }
                        },
                    )
                    if (reverseLayout) {
                        val previousCount = remember(listState) { intArrayOf(count) }
                        LaunchedEffect(listState, count) {
                            val oldCount = previousCount[0]
                            val delta = count - oldCount
                            if (delta > 0 && count > 0) {
                                val firstVisible = listState.firstVisibleItemIndex
                                val offset = listState.firstVisibleItemScrollOffset
                                if (firstVisible == delta && offset == 0) {
                                    listState.scrollToItem(0)
                                }
                            }
                            previousCount[0] = count
                        }
                    }
                    NexaDevObserveFastList(
                        listState = listState,
                        count = count,
                        readLeadingPosition = {
                            if (stickyHeaderOffset == 0) {
                                listState.firstVisibleItemIndex
                            } else {
                                val layout = listState.layoutInfo
                                nexaDevLeadingVisibleDataItemIndex(
                                    layout.visibleItemsInfo, listState.firstVisibleItemIndex,
                                    stickyHeaderOffset, layout.viewportStartOffset, layout.viewportEndOffset,
                                )
                            }
                        },
                        endIsVisible = {
                            val layout = listState.layoutInfo
                            nexaListEndIsVisible(
                                layout.visibleItemsInfo, count + stickyHeaderOffset - 1, stickyHeaderOffset > 0,
                                layout.viewportStartOffset, layout.viewportEndOffset,
                                itemIndex = { it.index }, itemOffset = { it.offset }, itemSize = { it.size },
                            )
                        },
                        onPosition = observePosition,
                        onEndReached = observeEndReached,
                    )
                    if (scrollPositionName != null) {
                        LaunchedEffect(listState, scrollPositionCommandRevision, count, reverseLayout) {
                            if (count > 0 && listState.firstVisibleItemIndex != requestedLayoutPosition) {
                                listState.scrollToItem(requestedLayoutPosition)
                            }
                        }
                    }
                    if (axis == "Horizontal") {
                        LazyRow(
                            state = listState,
                            contentPadding = LocalNexaScrollContentPadding.current,
                            horizontalArrangement = Arrangement.spacedBy(gap),
                            userScrollEnabled = scrollEnabled &&
                                (refreshState != null || listState.canScrollForward || listState.canScrollBackward),
                            modifier = modifier.then(scrollViewportModifier).fillMaxWidth(),
                        ) {
                            items(count = count, key = { itemIndex -> stableRowKey(itemIndex) }) { itemIndex ->
                                renderItem(itemIndex, axis, Modifier, null, null, itemIndex)
                            }
                        }
                    } else {
                        LazyColumn(
                            state = listState,
                            contentPadding = LocalNexaScrollContentPadding.current,
                            verticalArrangement = Arrangement.spacedBy(gap),
                            userScrollEnabled = scrollEnabled &&
                                (refreshState != null || listState.canScrollForward || listState.canScrollBackward),
                            reverseLayout = reverseLayout,
                            modifier = if (usesPlatformList && axis == "Vertical") {
                                modifier.then(scrollViewportModifier).then(NexaNativeListContainerModifier())
                            } else {
                                modifier.then(scrollViewportModifier).fillMaxWidth()
                            },
                        ) {
                            if (stickyHeaderNodes != null && stickyHeaderNodes != JSONObject.NULL) {
                                stickyHeader {
                                    NexaScrollContentResetPrimitive {
                                        renderHeader(stickyHeaderNodes, locals)
                                    }
                                }
                            }
                            if (sourceSections != null) {
                                sourceSections.forEachIndexed { sectionIndex, rawSection ->
                                    val sectionItems = rawSection as? List<*> ?: emptyList<Any?>()
                                    if (sectionHeader != null && sectionHeader != JSONObject.NULL) {
                                        stickyHeader {
                                            val headerLocals = locals.toMutableMap()
                                            headerLocals[sectionName ?: "section"] = sectionIndex
                                            headerLocals["sectionItems"] = sectionItems
                                            NexaScrollContentResetPrimitive {
                                                if (usesPlatformList && axis == "Vertical") {
                                                    NexaNativeListSectionHeaderPrimitive {
                                                        renderHeader(sectionHeader, headerLocals)
                                                    }
                                                } else {
                                                    renderHeader(sectionHeader, headerLocals)
                                                }
                                            }
                                        }
                                    }
                                    items(count = sectionItems.size, key = { itemIndex -> stableRowKey(itemIndex, sectionIndex) }) { itemIndex ->
                                        val rowLocals = listRowLocals(itemIndex, sectionIndex)
                                        val rowParentModifier = if (
                                            animateStableRows
                                        ) {
                                            Modifier.fillMaxWidth().animateItem(
                                                placementSpec = if ((reorderState?.activeIndex?.intValue ?: -1) < 0) {
                                                    androidx.compose.animation.core.spring<androidx.compose.ui.unit.IntOffset>()
                                                } else {
                                                    null
                                                },
                                            )
                                        } else {
                                            Modifier
                                        }
                                        val rowModifier = if (extent != null) {
                                            Modifier.fillMaxWidth().height(extent)
                                        } else {
                                            Modifier
                                        }
                                        if (usesPlatformList && axis == "Vertical") {
                                            NexaNativeListRowPrimitive(position = itemIndex, count = sectionItems.size) {
                                                renderListRow(
                                                    children,
                                                    rowLocals,
                                                    rowParentModifier.then(rowModifier),
                                                )
                                                if (showDividers && itemIndex + 1 < sectionItems.size) {
                                                    NexaNativeListDividerPrimitive()
                                                }
                                            }
                                        } else {
                                            renderListRow(
                                                children,
                                                rowLocals,
                                                rowParentModifier.then(rowModifier),
                                            )
                                            if (showDividers && itemIndex + 1 < sectionItems.size) {
                                                NexaFastListDividerPrimitive()
                                            }
                                        }
                                    }
                                    if (usesPlatformList && axis == "Vertical" && sectionIndex + 1 < sourceSections.size) {
                                        item(key = "nexa-native-list-section-gap-$sectionIndex") {
                                            NexaNativeListSectionSpacingPrimitive()
                                        }
                                    }
                                }
                            } else {
                                items(
                                    count = count,
                                    key = { itemPosition ->
                                        stableRowKey(itemPosition)
                                    },
                                ) { itemPosition ->
                                    val rowParentModifier = if (animateStableRows) {
                                        Modifier.fillMaxWidth().animateItem(
                                            placementSpec = if ((reorderState?.activeIndex?.intValue ?: -1) != itemPosition) {
                                                androidx.compose.animation.core.spring<androidx.compose.ui.unit.IntOffset>()
                                            } else {
                                                null
                                            },
                                        )
                                    } else {
                                        Modifier
                                    }
                                    if (onMove != null && axis == "Vertical") {
                                        renderItem(itemPosition, axis, rowParentModifier, listState, null, itemPosition)
                                    } else if (usesPlatformList && axis == "Vertical") {
                                        NexaNativeListRowPrimitive(position = itemPosition, count = count) {
                                            renderItem(itemPosition, axis, rowParentModifier, listState, null, itemPosition)
                                            if (showDividers && itemPosition + 1 < count) {
                                                NexaNativeListDividerPrimitive()
                                            }
                                        }
                                    } else {
                                        renderItem(itemPosition, axis, rowParentModifier, listState, null, itemPosition)
                                        if (showDividers && itemPosition + 1 < count) {
                                            NexaFastListDividerPrimitive()
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            val renderList: @Composable () -> Unit = {
                if (swipeActions == null) listContent()
                else NexaFastListSwipeGroup(content = listContent)
            }
            if (refreshState != null) {
                PullToRefreshBox(
                    isRefreshing = store.state(refreshState, scope) as? Boolean ?: false,
                    onRefresh = { store.perform(refreshActions, scope, locals) },
                    modifier = modifier.fillMaxSize(),
                ) { renderList() }
            } else {
                renderList()
            }
        }

}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderAppBottomBarNode(
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
        val state = fields.optString("state")
        val tabs = fields.optJSONArray("tabs") ?: JSONArray()
        val customBar = if (fields.has("custom_bar") && fields.opt("custom_bar") != JSONObject.NULL) {
            fields.optJSONArray("custom_bar") ?: JSONArray()
        } else {
            null
        }
        val tintValue = fields.optJSONObject("tint") ?: JSONObject()
        val dynamicTint = tintValue.opt("Dynamic")
        val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
            nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope)))
        } else {
            nexaDevColor(tintValue, isSystemInDarkTheme())
        }
        val selected = (store.state(state, scope) as? Number)?.toInt() ?: 0
        val tabStateHolder = rememberSaveableStateHolder()
        DisposableEffect(store) {
            val unregister = store.registerTabListener { target ->
                val t = target.lowercase()
                for (index in 0 until tabs.length()) {
                    val tab = tabs.optJSONObject(index) ?: continue
                    val tabIndex = tab.optInt("index", index)
                    val label = tab.optString("label").lowercase().replace(" ", "_")
                    val role = tab.optString("role").lowercase()
                    val matches = setOf(
                        "$tabIndex", "tab_$tabIndex", "tab$tabIndex",
                        "$index", "tab_$index", "tab$index",
                        label, "tab_$label", role
                    ).filter { it.isNotEmpty() }
                    if (t in matches) {
                        store.setState(state, tabIndex, scope)
                        break
                    }
                }
            }
            onDispose { unregister() }
        }
        NexaAppBottomBarPrimitive(
            tint = tint,
            showNavigationSuite = customBar == null,
            navigationSuiteItems = { tabItemColors ->
                if (customBar == null) {
                    for (index in 0 until tabs.length()) {
                        val tab = tabs.optJSONObject(index) ?: continue
                        val tabIndex = tab.optInt("index", index)
                        val label = tab.optString("label")
                        val iconSelection = tab.optJSONObject("icon") ?: JSONObject()
                        val badge = (tab.opt("badge") as? String)?.takeIf(String::isNotEmpty)
                        item(
                            selected = selected == tabIndex,
                            onClick = { store.setState(state, tabIndex, scope) },
                            icon = {
                                val tabIcon = nexaDevIconSpec(iconSelection, store, locals, scope)
                                if (tabIcon != null) NexaIconContent(icon = tabIcon, description = "", size = 24.dp)
                                else Spacer(Modifier.size(24.dp))
                            },
                            label = { Text(label) },
                            alwaysShowLabel = true,
                            badge = if (badge == null) null else ({ Badge { Text(badge) } }),
                            colors = tabItemColors,
                        )
                    }
                }
            },
        ) {
            val renderSelectedTab: @Composable () -> Unit = {
                val activeTab = (0 until tabs.length())
                    .firstNotNullOfOrNull { index -> tabs.optJSONObject(index)?.let { tab ->
                        tab.takeIf { it.optInt("index", index) == selected }
                    } }
                if (activeTab != null) {
                    val activeIndex = activeTab.optInt("index", 0)
                    tabStateHolder.SaveableStateProvider(key = "nexa-tab-$activeIndex") {
                        val title = activeTab.nexaDevOptionalString("navigation_title")
                            ?.takeIf(String::isNotEmpty)
                        val children = activeTab.optJSONArray("children") ?: JSONArray()
                        val (contentChildren, toolbarItems) = remember(module, children) {
                            nexaDevToolbarLayout(children)
                        }
                        if (title == null && toolbarItems.isEmpty()) {
                            NexaDevNodeList(
                                children,
                                module,
                                store,
                                parameters = locals,
                                scope = scope,
                            )
                        } else {
                            val contentBackground = MaterialTheme.colorScheme.background
                            val hasKeyboardInset = activeTab
                                .nexaDevOptionalString("search_state")
                                ?.isNotEmpty() == true
                            val contentModifier = when {
                                hasKeyboardInset -> Modifier
                                    .fillMaxSize()
                                    .background(contentBackground)
                                    .imePadding()
                                else -> Modifier.fillMaxSize()
                            }
                            if (title != null || toolbarItems.isNotEmpty()) {
                                NexaNavigationChrome(
                                    identity = "nexa-tab-title-$activeIndex",
                                    title = title ?: "",
                                    largeTitle = activeTab.optBoolean("large_title") && title != null,
                                    centerTitle = toolbarItems.none {
                                        it.first == "Leading"
                                    },
                                    containerColor = contentBackground,
                                    modifier = contentModifier,
                                    navigationIcon = {
                                        RenderTabToolbarItems(
                                            toolbarItems,
                                            "Leading",
                                            module,
                                            store,
                                            locals,
                                            scope,
                                        )
                                    },
                                    actions = {
                                        RenderTabToolbarItems(
                                            toolbarItems,
                                            "Trailing",
                                            module,
                                            store,
                                            locals,
                                            scope,
                                        )
                                    },
                                ) {
                                    RenderTabSearchInput(activeTab, module, store, locals, scope)
                                    NexaDevNodeList(
                                        contentChildren,
                                        module,
                                        store,
                                        parameters = locals,
                                        scope = scope,
                                    )
                                }
                            } else {
                                Column(contentModifier) {
                                    RenderTabSearchInput(activeTab, module, store, locals, scope)
                                    NexaDevNodeList(
                                        contentChildren,
                                        module,
                                        store,
                                        parameters = locals,
                                        scope = scope,
                                    )
                                }
                            }
                        }
                    }
                }
            }
            if (customBar == null) {
                renderSelectedTab()
            } else {
                Column(Modifier.fillMaxSize()) {
                    Box(Modifier.weight(1f).fillMaxWidth()) {
                        renderSelectedTab()
                    }
                    Column(Modifier.fillMaxWidth().windowInsetsPadding(WindowInsets.navigationBars)) {
                        NexaDevNodeList(
                            customBar,
                            module,
                            store,
                            parameters = locals,
                            scope = scope,
                        )
                    }
                }
            }
        }

}

@Composable
internal fun RenderTabToolbarItems(
    toolbars: List<Pair<String, JSONArray>>,
    placement: String,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    toolbars.filter { it.first == placement }.forEach { (_, children) ->
        for (index in 0 until children.length()) {
            val node = children.optJSONObject(index) ?: continue
            val kind = node.keys().asSequence().firstOrNull() ?: continue
            val fields = node.optJSONObject(kind) ?: continue
            if (kind != "NavigationLink") {
                val toolbarNodes = remember(module, node) { JSONArray().put(nexaDevCompactToolbarPickers(node)) }
                RenderChildren(toolbarNodes, module, store, locals, scope)
                continue
            }

            val screens = module.optJSONArray("screens") ?: JSONArray()
            val destination = screens.optJSONObject(fields.optInt("destination", -1))
            val linkChildren = fields.optJSONArray("children") ?: JSONArray()
            val guard = fields.opt("guard")
            val enabled = guard == null || guard == JSONObject.NULL ||
                store.evaluatePresented(guard, locals, scope) as? Boolean == true
            val staticallyDisabled = (guard as? JSONObject)?.opt("Bool") == false
            if (destination == null || staticallyDisabled) {
                RenderChildren(linkChildren, module, store, locals, scope)
                continue
            }

            val navId = (locals["__nexa_navigation_stack_identity"] as? String)
                ?: resolveNavigatorIdFromScope(scope)
            val controller = LocalNexaDevNavController.current
                ?: store.resolveNavController(stackIdentity = navId, locals = locals, scope = scope)
            val route = store.encodeScreenRoute(
                destination,
                fields.optJSONArray("arguments") ?: JSONArray(),
                locals,
                scope,
                navigatorId = navId,
            )
            IconButton(onClick = { controller?.navigate(route) }, enabled = enabled) {
                RenderChildren(linkChildren, module, store, locals, scope)
            }
        }
    }
}

private fun nexaDevCompactToolbarPickers(node: JSONObject): JSONObject {
    var copy: JSONObject? = null
    for (key in node.keys()) {
        val value = node.opt(key)
        val compactValue = when {
            key == "Picker" && value is JSONObject ->
                value.nexaDevShallowCopy().put("__nexa_fill_max_width", false)
            value is JSONObject -> nexaDevCompactToolbarPickers(value)
            value is JSONArray -> {
                var compactItems: JSONArray? = null
                for (index in 0 until value.length()) {
                    val item = value.opt(index)
                    val compactItem = if (item is JSONObject) nexaDevCompactToolbarPickers(item) else item
                    if (compactItem !== item && compactItems == null) {
                        compactItems = JSONArray()
                        for (earlier in 0 until index) compactItems.put(value.opt(earlier))
                    }
                    compactItems?.put(compactItem)
                }
                compactItems ?: value
            }
            else -> value
        }
        if (compactValue !== value) {
            val updated = copy ?: node.nexaDevShallowCopy().also { copy = it }
            updated.put(key, compactValue)
        }
    }
    return copy ?: node
}

@Composable
private fun RenderTabSearchInput(
    tab: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    val state = (tab.opt("search_state") as? String)
        ?.takeIf(String::isNotEmpty)
        ?: return
    val fields = JSONObject()
        .put("state", state)
        .put(
            "placeholder",
            tab.nexaDevOptionalString("search_prompt")?.takeIf(String::isNotEmpty) ?: "Search",
        )
        .put("keyboard", "Text")
        .put("secure", false)
        .put("multiline", false)
        .put("return_key", "Search")
        .put("autocorrect", false)
        .put("capitalization", "None")
        .put("max_lines", 1)
        .put("searchable", true)
    val node = JSONObject().put("TextInput", fields)
    NexaDevNode(
        node,
        module,
        store,
        locals,
        scope,
        Modifier.fillMaxWidth().padding(horizontal = 16.dp),
    )
}

@Composable
private fun renderWhenTarget(
    target: String,
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    spacerWeight: Modifier = Modifier,
) {
    val cases = fields.optJSONArray("cases") ?: JSONArray()
    var matchingIndex = -1
    for (index in 0 until cases.length()) {
        val item = cases.optJSONObject(index) ?: continue
        if (store.stringify(store.evaluate(item.opt("value"), locals, scope)) == target) {
            matchingIndex = index
            break
        }
    }
    key(matchingIndex) {
        RenderConditionalChildren(
            cases.optJSONObject(matchingIndex)?.optJSONArray("body")
                ?: fields.optJSONArray("else_body") ?: JSONArray(),
            module,
            store,
            locals,
            scope,
            spacerWeight,
        )
    }
}

private fun nexaDevTransitionStateName(expression: Any?): String? {
    val tagged = expression as? JSONObject ?: return null
    if (tagged.has("Located")) {
        return nexaDevTransitionStateName(tagged.optJSONObject("Located")?.opt("expression"))
    }
    val state = tagged.optJSONArray("State") ?: tagged.optJSONArray("AnimatedState") ?: return null
    return state.optString(0).takeIf(String::isNotEmpty)
}

private fun nexaDevTransitionLocals(
    locals: Map<String, Any>,
    stateName: String?,
    target: Any,
): Map<String, Any> {
    if (stateName == null) return locals
    return locals + mapOf(
        stateName to target,
        "__nexa_presented_animated:$stateName" to target,
    )
}

@Composable
private fun RenderConditionalChildren(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    spacerWeight: Modifier = Modifier,
) {
    when (nodes.length()) {
        0 -> Unit
        1 -> androidx.compose.runtime.key(0) {
            val child = nexaDevNodeObject(nodes.opt(0))
            val modifier = if (spacerWeight !== Modifier && nexaDevAcceptsSpacerWeight(child)) spacerWeight else Modifier
            NexaDevNode(child, module, store, locals, scope, modifier)
        }
        else -> NexaColumnPrimitive(
            spacing = 0f,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            RenderColumnChildren(nodes, module, store, locals, scope)
        }
    }
}

private fun nexaDevAcceptsSpacerWeight(node: JSONObject): Boolean {
    val spacer = node.optJSONObject("Spacer")
    if (spacer != null) return spacer.isNull("width") && spacer.isNull("height")
    val conditional = node.optJSONObject("If") ?: node.optJSONObject("When") ?: return false
    return conditional.nexaDevTransition().isEmpty()
}

@Composable
internal fun RenderChildren(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    for (index in 0 until nodes.length()) {
        androidx.compose.runtime.key(index) {
            NexaDevNode(nexaDevNodeObject(nodes.opt(index)), module, store, locals, scope)
        }
    }
}

@Composable
internal fun RowScope.RenderRowChildren(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    for (index in 0 until nodes.length()) {
        val child = nexaDevNodeObject(nodes.opt(index))
        androidx.compose.runtime.key(index) {
            val layoutStyle = child.optJSONObject("Layout")?.optJSONObject("style")
            val fillsRowWidth = nexaDevDimension(layoutStyle?.opt("width"))?.kind == "fill"
            val modifier = if (nexaDevAcceptsSpacerWeight(child) || fillsRowWidth) Modifier.weight(1f) else Modifier
            NexaDevNode(child, module, store, locals, scope, modifier)
        }
    }
}

private fun nexaDevMarqueeRevision(nodes: JSONArray, store: NexaDevStateStore, locals: Map<String, Any>, scope: String): List<String> {
    val values = mutableListOf<String>()
    fun visit(value: Any?) {
        when (value) {
            is JSONObject -> {
                if (value.has("State")) values += store.stringify(store.evaluate(value, locals, scope))
                else value.keys().asSequence().sorted().forEach { visit(value.opt(it)) }
            }
            is JSONArray -> for (index in 0 until value.length()) visit(value.opt(index))
        }
    }
    visit(nodes)
    return values
}
