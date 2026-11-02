package __NEXA_PACKAGE__

import android.os.SystemClock
import androidx.activity.compose.BackHandler
import android.content.Context
import android.content.Intent
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
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.gestures.snapping.SnapPosition
import androidx.compose.foundation.gestures.snapping.rememberSnapFlingBehavior
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
__NEXA_SPECIFIC_MATERIAL_ICON_IMPORTS__
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
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
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.ViewConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role as SemanticsRole
import androidx.compose.ui.semantics.contentDescription
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
import kotlin.math.roundToInt
import kotlin.math.abs
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.zIndex
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.withTimeoutOrNull
import org.json.JSONArray
import org.json.JSONObject
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

private object NexaDevFontRegistry {
    private val families = java.util.concurrent.ConcurrentHashMap<String, androidx.compose.ui.text.font.FontFamily>()

    fun resolve(
        context: Context,
        familyName: String,
        weight: FontWeight,
    ): androidx.compose.ui.text.font.FontFamily {
        val faceName = nexaDevFontFaceName(familyName, weight)
        val resourceName = buildString {
            var separator = false
            for (character in faceName) {
                if (character in 'a'..'z' || character in 'A'..'Z' || character in '0'..'9') {
                    if (separator && isNotEmpty()) append('_')
                    append(character.lowercaseChar())
                    separator = false
                } else {
                    separator = true
                }
            }
        }
        val cacheKey = "$resourceName:${weight.weight}"
        return families.getOrPut(cacheKey) {
            val resourceId = context.resources.getIdentifier(resourceName, "font", context.packageName)
            val resourceFace = if (resourceId != 0) {
                androidx.compose.ui.text.font.Font(resourceId, weight = weight)
            } else {
                null
            }
            val assetPath = listOf("ttf", "otf")
                .asSequence()
                .map { extension -> "nexa/fonts/$resourceName.$extension" }
                .firstOrNull { path ->
                    try {
                        context.assets.open(path).use { true }
                    } catch (_: Exception) {
                        false
                    }
                }
            val face = resourceFace ?: assetPath?.let { path ->
                androidx.compose.ui.text.font.Font(path = path, assetManager = context.assets, weight = weight)
            }
            if (face == null) {
                androidx.compose.ui.text.font.FontFamily.Default
            } else {
                androidx.compose.ui.text.font.FontFamily(face)
            }
        }
    }
}

private fun nexaDevFontFaceName(familyName: String, weight: FontWeight): String {
    val separatorIndex = familyName.lastIndexOfAny(charArrayOf('-', '_'))
    if (separatorIndex <= 0 || !familyName.substring(separatorIndex + 1).equals("regular", ignoreCase = true)) {
        return familyName
    }
    val face = when (weight) {
        FontWeight.Normal -> "Regular"
        FontWeight.Medium -> "Medium"
        FontWeight.SemiBold, FontWeight.Bold -> "Semibold"
        else -> "Regular"
    }
    return familyName.substring(0, separatorIndex + 1) + face
}

@Composable
private fun nexaDevFontFamily(name: String, weight: FontWeight): androidx.compose.ui.text.font.FontFamily {
    val context = LocalContext.current.applicationContext
    return remember(context, name, weight) { NexaDevFontRegistry.resolve(context, name, weight) }
}

private const val nexaDevDefaultBodyFontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__
private const val nexaDevDefaultMaterialIconSize = __NEXA_DEFAULT_MATERIAL_ICON_SIZE__
private const val nexaDevMaterialIconSizeScale = __NEXA_MATERIAL_ICON_SIZE_SCALE__
private const val nexaDevDefaultLineHeightMultiplier = __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f
private const val nexaDevFormRowHorizontalInset = __NEXA_FORM_ROW_HORIZONTAL_INSET__
private const val nexaDevFormRowMinHeight = __NEXA_ANDROID_FORM_ROW_MIN_HEIGHT__
private const val nexaDevFormLargeTitleTopPadding = __NEXA_FORM_LARGE_TITLE_TOP_PADDING__

private data class NexaDevScrollSnapshot(
    val isScrollInProgress: Boolean,
    val leadingPosition: Int,
    val visibleItemCount: Int,
    val reachedEnd: Boolean,
)

private fun nexaDevLeadingVisibleDataItemIndex(
    visibleItems: List<LazyListItemInfo>,
    fallback: Int,
    stickyHeaderOffset: Int,
    reverseLayout: Boolean,
    viewportStartOffset: Int,
    viewportEndOffset: Int,
): Int {
    val stickyHeader = if (stickyHeaderOffset > 0) {
        visibleItems.firstOrNull {
            it.index == 0 && it.offset + it.size > viewportStartOffset && it.offset < viewportEndOffset
        }
    } else null

    var leadingIndex: Int? = null
    var firstDataIndex: Int? = null
    var maximumVisibleIndex: Int? = null
    for (item in visibleItems) {
        if (item.offset + item.size <= viewportStartOffset || item.offset >= viewportEndOffset) continue
        if (maximumVisibleIndex == null || item.index > maximumVisibleIndex) {
            maximumVisibleIndex = item.index
        }
        if (item.index < stickyHeaderOffset) continue
        if (firstDataIndex == null) firstDataIndex = item.index
        if (stickyHeader != null &&
            item.offset < stickyHeader.offset + stickyHeader.size &&
            item.offset + item.size > stickyHeader.offset
        ) continue
        if (leadingIndex == null || (reverseLayout && item.index > leadingIndex) || (!reverseLayout && item.index < leadingIndex)) {
            leadingIndex = item.index
        }
    }
    if (leadingIndex != null) return leadingIndex
    return if (reverseLayout) {
        maximumVisibleIndex ?: fallback
    } else {
        firstDataIndex ?: fallback
    }
}

private fun nexaDevListItemIsUnobscuredByStickyHeader(
    item: LazyListItemInfo,
    visibleItems: List<LazyListItemInfo>,
    stickyHeaderOffset: Int,
    viewportStartOffset: Int,
    viewportEndOffset: Int,
): Boolean {
    if (item.offset + item.size <= viewportStartOffset || item.offset >= viewportEndOffset) return false
    if (stickyHeaderOffset == 0 || item.index < stickyHeaderOffset) return true
    val stickyHeader = visibleItems.firstOrNull { it.index == 0 } ?: return true
    return item.offset + item.size <= stickyHeader.offset ||
        item.offset >= stickyHeader.offset + stickyHeader.size
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

private fun nexaDevSfAliasMaterialIcon(name: String) = when (name) {
__NEXA_SHARED_ICON_SF_ALIAS_CASES__
    else -> null
}

private fun nexaDevSpecificMaterialIcon(name: String) = when (name) {
__NEXA_SPECIFIC_MATERIAL_ICON_CASES__
    else -> null
}

@Composable
private fun nexaDevIconSpec(selection: JSONObject): NexaIconSpec? {
    val fontGlyph = selection.optJSONObject("font_glyph")
    if (fontGlyph != null) {
        val family = fontGlyph.optString("font_family")
        val glyph = fontGlyph.optInt("glyph", -1)
        if (family.isNotEmpty() && glyph >= 0) {
            return NexaIconSpec(
                glyph = glyph,
                fontFamily = nexaDevFontFamily(family, FontWeight.Normal),
            )
        }
        return null
    }
    val vector = when {
        selection.has("shared") -> nexaDevSharedMaterialIcon(selection.optString("shared"))
        selection.has("sf_symbol") -> nexaDevSfAliasMaterialIcon(selection.optString("sf_symbol"))
        selection.has("material_symbol") -> nexaDevSpecificMaterialIcon(selection.optString("material_symbol"))
        else -> null
    } ?: return null
    return NexaIconSpec(vector = vector)
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

private fun nexaDevFormRowButton(node: JSONObject): JSONObject =
    JSONObject(node.toString()).apply {
        optJSONObject("Button")?.put("__nexa_leading_content", true)
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

private fun JSONObject.nexaDevOptionalString(name: String): String? =
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
internal fun NexaDevStateStore.animatedFloatState(name: String, scope: String): Float {
    val target = (state(name, scope) as? Number)?.toFloat() ?: 0f
    val animation = androidx.compose.animation.core.animateFloatAsState(
        targetValue = target,
        animationSpec = animationSpec(name, scope),
        finishedListener = { clearAnimationSpec(name, scope) },
        label = name,
    )
    return animation.value
}

internal fun openNexaUrl(context: Context, value: String) {
    val uri = Uri.parse(value)
    if (uri.scheme !in setOf("https", "http", "mailto", "tel")) return
    runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, uri)) }
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
        if (kind == "FastList" || kind == "KeyboardAware" || kind == "Form") return true
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
                    child.has("Spacer") -> Modifier.weight(1f)
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
                return ((store.evaluatePresented(raw, locals, scope) as? Number)?.toFloat() ?: 0f).dp
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
            val scrollContent = child.has("FastList") || child.has("KeyboardAware") || child.has("RefreshControl")
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
                                NexaDevNode(nexaDevFormRowButton(child), module, store, locals, scope)
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
                        val rowNode = nexaDevFormRowButton(nexaDevNodeObject(children.opt(index)))
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
            DisposableEffect(componentScope) {
                onDispose {
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
        "Layout" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            val spacing = fields.optDouble("gap", 0.0).dp
            val style = fields.optJSONObject("style") ?: JSONObject()
            // Keep the parent sizing contract (notably a screen's fillMaxSize)
            // when applying this node's own visual modifiers.
            var modifier: Modifier = modifier
            val effects = style.optJSONObject("effects") ?: JSONObject()
            // Compose modifier chains are outer-to-inner. Nexa's `clip` effect
            // masks the completed view in SwiftUI, so wrap this layout before
            // its size and transform modifiers to also contain overflowing
            // fixed-size and transformed descendants.
            if (!effects.isNull("clip_rounded")) {
                modifier = modifier.clip(RoundedCornerShape(effects.optDouble("clip_rounded").dp))
            }
            val metrics = LocalNexaWindowMetrics.current
            fun dimensionValue(name: String): NexaDevDimension? = nexaDevDimension(style.opt(name))
            val width = dimensionValue("width")
            val height = dimensionValue("height")
            val minWidth = dimensionValue("min_width")
            val maxWidth = dimensionValue("max_width")
            val minHeight = dimensionValue("min_height")
            val maxHeight = dimensionValue("max_height")
            if (!style.isNull("animation")) {
                modifier = modifier.animateContentSize(
                    animationSpec = store.devAnimationSpec(style.opt("animation")),
                )
            }
            fun resolve(dimension: NexaDevDimension?): Dp? = when (dimension?.kind) {
                "fixed" -> dimension.value.dp
                "screen_width" -> (metrics.viewportWidthDp * dimension.value).dp
                "screen_height" -> (metrics.viewportHeightDp * dimension.value).dp
                else -> null
            }
            resolve(width)?.let { modifier = modifier.width(it) }
            resolve(height)?.let {
                if (height?.kind == "fixed" || height?.kind == "screen_height") {
                    modifier = modifier
                        .wrapContentHeight(Alignment.Top, unbounded = true)
                        .requiredHeight(it)
                }
                modifier = modifier.height(it)
            }
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
            // Dev IR serializes unset Option fields as JSON null, and JSONObject.has()
            // still returns true for a key mapped to null. Only apply optional styles
            // when the value is present so absent animations do not animate every layout.
            if (!style.isNull("opacity")) modifier = modifier.graphicsLayer(
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
                modifier = modifier.graphicsLayer(
                    scaleX = scaleX,
                    scaleY = scaleY,
                    alpha = alpha,
                    translationY = translationY,
                    rotationZ = rotation,
                )
            }
            if (!effects.isNull("blur")) modifier = modifier.blur(effects.optDouble("blur").dp)
            val cornerRadius = if (!style.isNull("corner_radius_expression")) {
                ((store.evaluatePresented(style.opt("corner_radius_expression"), locals, scope) as? Number)
                    ?.toFloat() ?: style.optDouble("corner_radius", 0.0).toFloat()).dp
            } else {
                style.optDouble("corner_radius", 0.0).dp
            }
            val bottomCornerRadius = style.optDouble("corner_radius_bottom", cornerRadius.value.toDouble()).dp
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
            if (!style.isNull("corner_radius") || !style.isNull("corner_radius_expression") || !style.isNull("corner_radius_bottom")) {
                modifier = modifier.clip(cornerShape)
            }
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
            if (!effects.isNull("z_index")) modifier = modifier.zIndex(effects.optInt("z_index").toFloat())
            val rawAlignment = fields.opt("alignment")
            val alignment = rawAlignment as? String
                ?: (rawAlignment as? JSONObject)?.keys()?.asSequence()?.firstOrNull()
                ?: style.optString("alignment")
            when (fields.optString("kind")) {
                "Row" -> NexaRowPrimitive(
                    modifier = modifier,
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
                    modifier = modifier,
                    contentAlignment = when (alignment) {
                        "Start" -> Alignment.TopStart
                        "End" -> Alignment.BottomEnd
                        else -> Alignment.Center
                    },
                ) { RenderChildren(children, module, store, locals, scope) }
                else -> NexaColumnPrimitive(
                    modifier = modifier,
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
        "ContentUnavailable" -> {
            val title = store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope))
            val description = fields.opt("description")
                ?.takeUnless { it === JSONObject.NULL }
                ?.let { store.stringify(store.evaluatePresented(it, locals, scope)) }
            val icon = fields.optJSONObject("icon") ?: JSONObject()
            val sharedIcon = icon.optString("shared")
            val iconSpec = nexaDevIconSpec(icon)
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
        "Text", "Spacer", "Divider", "SystemIcon", "FontIcon", "LinearGradient", NexaDevNodeKind.SHAPE ->
            NexaDevRenderTextNode(kind, fields, module, store, locals, scope, modifier)
        "Button", "Pressable", "TextInput", "FastList", "Switch", "Slider",
        "ProgressBar", "ProgressRing", "SegmentedControl", "Picker", "PickerDate" ->
            NexaDevRenderControlNode(kind, fields, module, store, locals, scope, modifier)
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
            val style = fields.optJSONObject("style") ?: JSONObject()
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
                val customFontFamily = style.optString("font_family").takeIf(String::isNotBlank)
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
                var modifier: Modifier = Modifier
                val effects = style.optJSONObject("effects") ?: JSONObject()
                if (!style.isNull("padding")) modifier = modifier.padding(style.optDouble("padding").dp)
                if (!style.isNull("opacity")) modifier = modifier.alpha(style.optDouble("opacity").toFloat())
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
                    modifier = modifier.graphicsLayer(
                        scaleX = scaleX,
                        scaleY = scaleY,
                        alpha = alpha,
                        translationY = translationY,
                        rotationZ = rotation,
                    )
                }
                if (!effects.isNull("blur")) modifier = modifier.blur(effects.optDouble("blur").dp)
                val textShadow = effects.optJSONObject("shadow")?.let { shadow ->
                    val color = nexaDevColor(shadow.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Black
                    NexaGlyphShadow(
                        color = color,
                        radius = shadow.optDouble("radius", 0.0).dp,
                        x = shadow.optDouble("x", 0.0).dp,
                        y = shadow.optDouble("y", 0.0).dp,
                    )
                }
                if (!effects.isNull("clip_rounded")) modifier = modifier.clip(RoundedCornerShape(effects.optDouble("clip_rounded").dp))
                if (!effects.isNull("z_index")) modifier = modifier.zIndex(effects.optInt("z_index").toFloat())
                val reveal = style.optJSONObject("reveal")
                if (reveal == null) {
                    NexaTextPrimitive(
                        text = text,
                        modifier = modifier,
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
                        active = store.evaluatePresented(reveal.opt("active"), locals, scope) as? Boolean ?: false,
                        playing = store.evaluatePresented(reveal.opt("playing"), locals, scope) as? Boolean ?: false,
                        enabled = store.evaluatePresented(reveal.opt("enabled"), locals, scope) as? Boolean ?: true,
                        seekRevision = (store.evaluatePresented(reveal.opt("seek_revision"), locals, scope) as? Number)?.toLong() ?: 0L,
                        dimOpacity = reveal.optDouble("dim_opacity", 0.38).toFloat(),
                        leadTime = reveal.optDouble("lead_time", 0.3).toFloat(),
                        targetOffset = reveal.optDouble("target_offset", 0.56).toFloat(),
                        smoothingDurationMs = (reveal.optDouble("smoothing_duration", 0.25) * 1000.0).toInt(),
                        modifier = modifier,
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
            val hasFixedSize = !fields.isNull("width") || !fields.isNull("height")
            var spacerModifier = if (hasFixedSize) Modifier else modifier
            if (!fields.isNull("width")) spacerModifier = spacerModifier.width(fields.optDouble("width").dp)
            if (!fields.isNull("height")) spacerModifier = spacerModifier.height(fields.optDouble("height").dp)
            Spacer(modifier = spacerModifier)
        }
        "Divider" -> NexaDividerPrimitive(
            color = nexaDevColor(fields.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Gray,
            thickness = fields.optDouble("thickness", 1.0).dp,
        )
        NexaDevNodeKind.SHAPE -> {
            val shapeFields = fields.optJSONObject("shape") ?: JSONObject()
            val encodedShape = fields.opt("shape") as? String
            val shape = when {
                shapeFields.has("Circle") || encodedShape == "Circle" -> CircleShape
                shapeFields.has("Rounded") -> RoundedCornerShape(shapeFields.optDouble("Rounded").dp)
                else -> RoundedCornerShape(percent = 50)
            }
            val widthExpression = fields.opt("width_expression")
                ?.takeUnless { it === JSONObject.NULL }
            val heightExpression = fields.opt("height_expression")
                ?.takeUnless { it === JSONObject.NULL }
            val width = if (widthExpression == null) {
                fields.optDouble("width").dp
            } else {
                ((store.evaluatePresented(widthExpression, locals, scope) as? Number)?.toDouble() ?: 0.0)
                    .coerceAtLeast(0.0).dp
            }
            val height = if (heightExpression == null) {
                fields.optDouble("height").dp
            } else {
                ((store.evaluatePresented(heightExpression, locals, scope) as? Number)?.toDouble() ?: 0.0)
                    .coerceAtLeast(0.0).dp
            }
            val rawAnimation = fields.opt("animation")
                ?.takeUnless { it === JSONObject.NULL }
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
            if (animatedWidth != null || animatedHeight != null) {
                androidx.compose.ui.layout.Layout(
                    content = {},
                    modifier = modifier.background(fill, shape = shape),
                ) { _, constraints ->
                    val measuredWidth = (animatedWidth?.value ?: width)
                        .roundToPx()
                        .coerceIn(constraints.minWidth, constraints.maxWidth)
                    val measuredHeight = (animatedHeight?.value ?: height)
                        .roundToPx()
                        .coerceIn(constraints.minHeight, constraints.maxHeight)
                    layout(measuredWidth, measuredHeight) {}
                }
            } else {
                Spacer(
                    modifier = modifier
                        .width(width)
                        .height(height)
                        .background(fill, shape = shape),
                )
            }
        }
        "SystemIcon" -> {
            val selection = fields.optJSONObject("icon") ?: JSONObject()
            val icon = nexaDevIconSpec(selection)
                ?: NexaIconSpec(vector = nexaDevSharedMaterialIcon("help"))
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
                size = (fields.optDouble("size", 24.0) * nexaDevMaterialIconSizeScale).dp,
                tint = tint ?: Color.White,
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
                size = ((store.evaluatePresented(fields.opt("size"), locals, scope) as? Number)
                    ?.toDouble() ?: 24.0).coerceIn(1.0, 512.0).toFloat().dp,
                tint = tint ?: Color.White,
                modifier = modifier,
                fontFamily = nexaDevFontFamily(fields.optString("font_family"), FontWeight.Normal),
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
            val style = fields.opt("style")
                ?.toString()
                ?.takeIf { it.isNotBlank() && it != "null" }
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
            val icon = nexaDevIconSpec(iconSelection)
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
                fullWidth = fields.optBoolean("__nexa_full_width", false),
                leadingContent = fields.optBoolean("__nexa_leading_content", false),
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
            val step = fields.optDouble("step")
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
            val style = fields.optJSONObject("style")
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
            val style = fields.optJSONObject("style")
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
            val pickerIcon = nexaDevIconSpec(iconSelection)
            val itemIconEntries = fields.optJSONArray("item_icons") ?: JSONArray()
            val pickerItemIcons = buildList<NexaIconSpec?> {
                for (index in 0 until itemIconEntries.length()) {
                    val selection = itemIconEntries.optJSONObject(index) ?: JSONObject()
                    add(nexaDevIconSpec(selection))
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
            val scale = if (fields.optString("scale") == "Fill") ContentScale.Crop else ContentScale.Fit
            val placeholder = fields.nexaDevOptionalString("placeholder")?.takeIf(String::isNotEmpty)
            val sharedElementExpression = fields.opt("shared_element")
            val renderedModifier = if (sharedElementExpression != null && sharedElementExpression != JSONObject.NULL) {
                modifier.then(nexaSharedElementModifier(store.evaluatePresented(sharedElementExpression, locals, scope)))
            } else {
                modifier
            }
            val maxHeight = if (fields.has("max_height")) fields.optDouble("max_height").dp else null
            val fillMaxSize = fields.optBoolean("fill_max_size", false)
            val showLoadingIndicator = fields.optBoolean("show_loading_indicator", false)
            val asset = source.optString("Asset").takeIf(String::isNotEmpty)
            val remoteExpression = source.opt("RemoteUrl")
            val localFileExpression = source.opt("LocalFile")
            if (asset != null) {
                NexaAssetImagePrimitive(
                    name = asset,
                    contentDescription = description,
                    contentScale = scale,
                    maxHeight = maxHeight,
                    fillMaxSize = fillMaxSize,
                    modifier = renderedModifier,
                )
            } else if (remoteExpression != null && remoteExpression != JSONObject.NULL) {
                val url = store.stringify(store.evaluatePresented(remoteExpression, locals, scope))
                NexaRemoteImagePrimitive(
                    url = url,
                    allowsFile = false,
                    placeholder = placeholder,
                    showLoadingIndicator = showLoadingIndicator,
                    contentDescription = description,
                    contentScale = scale,
                    maxHeight = maxHeight,
                    fillMaxSize = fillMaxSize,
                    modifier = renderedModifier,
                )
            } else if (localFileExpression != null && localFileExpression != JSONObject.NULL) {
                val fileUrl = store.stringify(store.evaluatePresented(localFileExpression, locals, scope))
                NexaRemoteImagePrimitive(
                    url = fileUrl,
                    allowsFile = true,
                    placeholder = null,
                    showLoadingIndicator = showLoadingIndicator,
                    contentDescription = description,
                    contentScale = scale,
                    maxHeight = maxHeight,
                    fillMaxSize = fillMaxSize,
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
                RenderConditionalChildren(
                    fields.optJSONArray(if (condition) "then_body" else "else_body") ?: JSONArray(),
                    module,
                    store,
                    locals,
                    scope,
                )
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
                renderWhenTarget(value, fields, module, store, locals, scope)
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
            val interactiveDismiss = fields.optString("dismiss") == "Interactive"
            val avoidIme = fields.optString("behavior") == "Avoid"
            if (nexaDevContainsScrollable(children)) {
                NexaKeyboardAwareOwnerPrimitive(
                    interactiveDismiss = interactiveDismiss,
                    avoidIme = avoidIme,
                ) {
                    RenderChildren(children, module, store, locals, scope)
                }
            } else {
                NexaKeyboardAwareColumnPrimitive(
                    interactiveDismiss = interactiveDismiss,
                    avoidIme = avoidIme,
                ) {
                    RenderChildren(children, module, store, locals, scope)
                }
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
                        val actionNode = child.optJSONObject("Button")?.let { button ->
                            val wrapped = JSONObject(child.toString())
                            val fieldsForAction = wrapped.getJSONObject("Button")
                            if (!fieldsForAction.has("style") || fieldsForAction.isNull("style")) {
                                fieldsForAction.put("style", "Borderless")
                            }
                            fieldsForAction.put("__nexa_full_width", true)
                            wrapped
                        } ?: child
                        NexaDevNode(actionNode, module, store, locals, scope)
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

internal fun nexaDevTabToolbars(nodes: JSONArray): List<Pair<String, JSONArray>> {
    val toolbars = mutableListOf<Pair<String, JSONArray>>()
    for (index in 0 until nodes.length()) {
        val node = nodes.optJSONObject(index) ?: continue
        val kind = node.keys().asSequence().firstOrNull() ?: continue
        val fields = node.optJSONObject(kind) ?: continue
        when (kind) {
            "Toolbar" -> toolbars += fields.optString("placement") to (fields.optJSONArray("children") ?: JSONArray())
            "Layout" -> toolbars += nexaDevTabToolbars(fields.optJSONArray("children") ?: JSONArray())
        }
    }
    return toolbars
}

internal fun nexaDevWithoutTabToolbars(nodes: JSONArray): JSONArray {
    val result = JSONArray()
    for (index in 0 until nodes.length()) {
        val node = nodes.optJSONObject(index) ?: continue
        val kind = node.keys().asSequence().firstOrNull() ?: continue
        if (kind == "Toolbar") continue
        val fields = node.optJSONObject(kind)
        if (kind == "Layout" && fields != null) {
            val filteredFields = JSONObject(fields.toString())
            filteredFields.put(
                "children",
                nexaDevWithoutTabToolbars(fields.optJSONArray("children") ?: JSONArray()),
            )
            result.put(JSONObject().put(kind, filteredFields))
        } else {
            result.put(node)
        }
    }
    return result
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
        val haptic = LocalHapticFeedback.current
        val hapticType = when (hapticStyle) {
            "Light" -> HapticFeedbackType.TextHandleMove
            "Medium" -> HapticFeedbackType.LongPress
            "Heavy" -> HapticFeedbackType.ContextClick
            else -> null
        }
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
        val pressableModifier = if (fillMaxSize) modifier.then(dragModifier).fillMaxSize() else modifier.then(dragModifier)
        val hasLongPress = longPressActions.length() > 0 || contextMenuActions.length() > 0
        val hasDoubleTap = doubleTapActions.length() > 0
        val pressableCore: @Composable () -> Unit = {
            val clickModifier = if (!hasLongPress && !hasDoubleTap) {
                pressableModifier.clickable(
                    enabled = !disabled,
                    interactionSource = remember { androidx.compose.foundation.interaction.MutableInteractionSource() },
                    indication = null,
                    onClick = {
                        if (gestureConsumedTap?.value == true) {
                            gestureConsumedTap.value = false
                        } else {
                            if (hapticType != null) haptic.performHapticFeedback(hapticType)
                            store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals)
                        }
                    },
                )
            } else if (!hasDoubleTap) {
                pressableModifier.combinedClickable(
                    enabled = !disabled,
                    interactionSource = null,
                    onClick = {
                        if (gestureConsumedTap?.value == true) {
                            gestureConsumedTap.value = false
                        } else {
                            if (hapticType != null) haptic.performHapticFeedback(hapticType)
                            store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals)
                        }
                    },
                    onLongClick = if (hasLongPress) {
                        {
                            contextMenuExpanded?.value = true
                            store.perform(longPressActions, scope, locals)
                        }
                    } else null,
                    onDoubleClick = null,
                )
            } else {
                pressableModifier.combinedClickable(
                    enabled = !disabled,
                    interactionSource = null,
                    onClick = {
                        if (gestureConsumedTap?.value == true) {
                            gestureConsumedTap.value = false
                        } else {
                            if (hapticType != null) haptic.performHapticFeedback(hapticType)
                            store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals)
                        }
                    },
                    onLongClick = if (hasLongPress) {
                        {
                            contextMenuExpanded?.value = true
                            store.perform(longPressActions, scope, locals)
                        }
                    } else null,
                    onDoubleClick = {
                        if (hapticType != null) haptic.performHapticFeedback(hapticType)
                        store.perform(doubleTapActions, scope, locals)
                    },
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
                                    val itemIcon = nexaDevIconSpec(iconSelection)
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
        val focusKey = "$scope/input/$state"
        val focusedState = fields.optString("focused").takeIf(String::isNotEmpty)
        val focusRequester = focusedState?.let { remember(it) { FocusRequester() } }
        val focused = focusedState?.let { store.state(it, scope) as? Boolean == true } ?: false
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
        val maxLength = fields.optInt("max_length", Int.MAX_VALUE).coerceAtLeast(0)
        val minLines = fields.optInt("min_lines", 1).coerceAtLeast(1)
        val maxLines = fields.optInt("max_lines", Int.MAX_VALUE).coerceAtLeast(minLines)
        val baseTextStyle = when (fields.optString("font")) {
            "Body" -> MaterialTheme.typography.bodyLarge
            "Title3" -> MaterialTheme.typography.titleMedium
            else -> LocalTextStyle.current
        }
        val inputFontSize = if (fields.has("font_size") && !fields.isNull("font_size")) {
            fields.optDouble("font_size").toFloat().sp
        } else {
            baseTextStyle.fontSize
        }
        val inputFontName = fields.optString("font_family").takeIf(String::isNotBlank)
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
        val focusManager = androidx.compose.ui.platform.LocalFocusManager.current
        var fieldModifier: Modifier = Modifier
        if (focusedState != null && focusRequester != null) {
            fieldModifier = fieldModifier
                .focusRequester(focusRequester)
                .onFocusChanged { focusState ->
                    store.setState(focusedState, focusState.isFocused, scope)
                    if (focusState.isFocused && onFocus.length() > 0) {
                        store.perform(onFocus, scope, locals)
                    }
                }
            LaunchedEffect(focused, focusKey) {
                if (focused) focusRequester.requestFocus() else focusRequester.freeFocus()
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
            onValueChange = { next ->
                val updated = next.take(maxLength)
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
            modifier = fieldModifier.then(modifier),
            placeholder = placeholder.orEmpty(),
            searchable = fields.optBoolean("searchable"),
            searchIcon = if (fields.optBoolean("searchable")) nexaDevSharedMaterialIcon("search") else null,
            textStyle = textStyle,
            textColor = textColor,
            placeholderColor = placeholderColor,
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
    val coroutineScope = rememberCoroutineScope()
    DisposableEffect(reference, count, reverseLayout) {
        reference.attach(
            scrollToIndex = { index, animated, viewPosition ->
                if (count > 0) coroutineScope.launch {
                    scrollToIndex(index.coerceIn(0, count - 1), animated, viewPosition.coerceIn(0.0, 1.0))
                }
            },
            scrollToTop = { animated ->
                if (count > 0) coroutineScope.launch {
                    scrollToIndex(if (reverseLayout) count - 1 else 0, animated, 0.0)
                }
            },
            scrollToEnd = { animated ->
                if (count > 0) coroutineScope.launch {
                    scrollToIndex(if (reverseLayout) 0 else count - 1, animated, 1.0)
                }
            },
            scrollToOffset = { offset, animated ->
                coroutineScope.launch { scrollToOffset(offset.coerceAtLeast(0.0), animated) }
            },
        )
        onDispose { reference.detach() }
    }
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
            val count = sourceSections?.sumOf { (it as? List<*>)?.size ?: 0 } ?: itemCount
            val indexName = options.optString("index", "index")
            val itemName = listPlan.nexaDevOptionalString("item")?.takeIf(String::isNotEmpty)
            val sectionName = listPlan.nexaDevOptionalString("section")?.takeIf(String::isNotEmpty)
            fun listRowLocals(itemIndex: Int, sectionIndex: Int? = null): Map<String, Any> {
                var rowLocals = locals + (indexName to itemIndex)
                if (sectionIndex != null) {
                    val sectionItems = sourceSections?.getOrNull(sectionIndex) as? List<*>
                    if (itemName != null) {
                        rowLocals = rowLocals + (itemName to (sectionItems?.getOrNull(itemIndex) ?: JSONObject.NULL))
                    }
                    rowLocals = rowLocals + ((sectionName ?: "section") to sectionIndex)
                } else if (itemName != null && sourceItems != null) {
                    rowLocals = rowLocals + (itemName to (sourceItems.getOrNull(itemIndex) ?: JSONObject.NULL))
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
            val swipeActions = options.optJSONArray("swipe_actions")
            val stickyHeaderNodes = options.optJSONArray("sticky_header")
            val sectionHeader = options.optJSONArray("section_header")
            val onScroll = options.optJSONArray("on_scroll")
            val onEndReached = options.optJSONArray("on_end_reached")
            val listTaskOwner = if (onEndReached != null && onEndReached != JSONObject.NULL) {
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
            var endReached by rememberSaveable(count, sourceItems?.size, sourceSections?.size) {
                mutableStateOf(false)
            }
            var lastReportedScrollPosition by remember { mutableIntStateOf(-1) }
            val scrollWasInProgress = remember { booleanArrayOf(false) }
            var nativeScrollPositionEcho by remember { mutableIntStateOf(-1) }
            val scrollPositionName = options.nexaDevOptionalString("scroll_position")
                ?.takeIf(String::isNotEmpty)
            val stickyHeaderOffset = if (axis == "Vertical" && stickyHeaderNodes != null && stickyHeaderNodes != JSONObject.NULL) 1 else 0
            val requestedIndex = scrollPositionName?.let { name ->
                (store.state(name, scope) as? Number)?.toInt()?.coerceIn(0, maxOf(0, count - 1)) ?: 0
            } ?: if (reverseLayout) maxOf(0, count - 1) else 0
            val requestedPosition = if (reverseLayout) {
                (count - 1 - requestedIndex).coerceIn(0, maxOf(0, count - 1))
            } else requestedIndex
            val requestedLayoutPosition = if (stickyHeaderOffset > 0 && requestedPosition == 0) {
                0
            } else {
                (requestedPosition + stickyHeaderOffset)
                    .coerceIn(0, maxOf(0, count + stickyHeaderOffset - 1))
            }
            val extent = options.optDouble("item_extent", 0.0).takeIf { it > 0.0 }?.dp
            val gap = options.optDouble("gap", 0.0).coerceAtLeast(0.0).dp
            val renderListRow: @Composable (JSONArray, Map<String, Any>, Modifier) -> Unit = { rowNodes, rowLocals, rowModifier ->
                val rowContent: @Composable () -> Unit = {
                    NexaFastListRowPrimitive(
                        modifier = rowModifier,
                        horizontalAlignment = if (rowNodes.length() > 1) {
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
                if (swipeActions != null && (onMove == null || !moveEnabled)) {
                    NexaFastListSwipePrimitive(
                        resetKey = count,
                        backgroundContent = {
                            RenderChildren(swipeActions, module, store, rowLocals, scope)
                        },
                        content = rowContent,
                    )
                } else {
                    rowContent()
                }
            }
            val reorderActiveIndex = if (onMove != null) {
                remember { mutableIntStateOf(-1) }
            } else {
                null
            }
            val reorderOffset = if (onMove != null) {
                remember { androidx.compose.runtime.mutableFloatStateOf(0f) }
            } else {
                null
            }
            val renderItem: @Composable (
                Int,
                String,
                Modifier,
                androidx.compose.foundation.lazy.LazyListState?,
            ) -> Unit = { itemIndex, itemAxis, itemParentModifier, itemListState ->
                val rowLocals = listRowLocals(itemIndex)
                val itemModifier = if (pageSnap && itemAxis == "Vertical") {
                    Modifier
                } else when (itemAxis) {
                    // rowHeight is the cross-axis extent; let horizontal row content set its width.
                    "Horizontal" -> extent?.let { Modifier.height(it) } ?: Modifier
                    "Grid" -> extent?.let { Modifier.fillMaxWidth().height(it) } ?: Modifier
                    else -> extent?.let { Modifier.fillMaxWidth().height(it) } ?: Modifier
                }
                val reorderModifier = if (
                    onMove != null && moveEnabled && itemAxis == "Vertical" &&
                    reorderActiveIndex != null && reorderOffset != null
                ) {
                    val headerIndexOffset = if (stickyHeaderNodes != null && stickyHeaderNodes != JSONObject.NULL) 1 else 0
                    Modifier
                        .pointerInput(count, onMove, itemListState) {
                            var dragDistance = 0f
                            // Observe the Initial pass so SwipeToDismissBox cannot claim reorder drags.
                            awaitEachGesture {
                                val down = awaitFirstDown(
                                    requireUnconsumed = false,
                                    pass = androidx.compose.ui.input.pointer.PointerEventPass.Initial,
                                )
                                val releasedBeforeLongPress = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) {
                                    while (true) {
                                        val event = awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial)
                                        val change = event.changes.firstOrNull { it.id == down.id }
                                            ?: return@withTimeoutOrNull true
                                        if (!change.pressed) return@withTimeoutOrNull true
                                        if (abs(change.position.x - down.position.x) > viewConfiguration.touchSlop ||
                                            abs(change.position.y - down.position.y) > viewConfiguration.touchSlop
                                        ) return@withTimeoutOrNull true
                                    }
                                    false
                                }
                                if (releasedBeforeLongPress != null) return@awaitEachGesture

                                dragDistance = 0f
                                reorderOffset.floatValue = 0f
                                reorderActiveIndex.intValue = itemIndex
                                var draggedIndex = itemIndex
                                var previousPosition = down.position
                                try {
                                    while (true) {
                                        val event = awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial)
                                        val change = event.changes.firstOrNull { it.id == down.id } ?: break
                                        if (!change.pressed) break
                                        val amount = change.position - previousPosition
                                        change.consume()
                                        dragDistance += amount.y
                                        reorderOffset.floatValue = dragDistance
                                        previousPosition = change.position

                                        val listState = itemListState
                                        if (listState != null) {
                                            val visibleItems = listState.layoutInfo.visibleItemsInfo
                                            val draggedItem = visibleItems.firstOrNull {
                                                it.index == draggedIndex + headerIndexOffset
                                            }
                                            if (draggedItem != null) {
                                                val draggedCenter = draggedItem.offset + draggedItem.size / 2 + dragDistance.roundToInt()
                                                var targetIndex = -1
                                                var targetCenter = 0
                                                var targetDistance = Int.MAX_VALUE
                                                for (visibleIndex in visibleItems.indices) {
                                                    val item = visibleItems[visibleIndex]
                                                    if (item.index < headerIndexOffset || item.index >= headerIndexOffset + count) continue
                                                    val itemCenter = item.offset + item.size / 2
                                                    val distance = abs(itemCenter - draggedCenter)
                                                    if (distance < targetDistance) {
                                                        targetDistance = distance
                                                        targetCenter = itemCenter
                                                        targetIndex = item.index - headerIndexOffset
                                                    }
                                                }
                                                if (targetIndex >= 0 && targetIndex != draggedIndex) {
                                                    val previousCenter = draggedItem.offset + draggedItem.size / 2
                                                    val from = draggedIndex
                                                    val to = targetIndex
                                                    val callbackLocals = locals +
                                                        (onMove.optString("from") to from) +
                                                        (onMove.optString("to") to to)
                                                    store.perform(onMove.optJSONArray("actions") ?: JSONArray(), scope, callbackLocals)
                                                    draggedIndex = to
                                                    reorderActiveIndex.intValue = to
                                                    val layoutShift = (targetCenter - previousCenter).toFloat()
                                                    dragDistance -= layoutShift
                                                    reorderOffset.floatValue = dragDistance
                                                    if (animateStableRows) {
                                                        previousPosition = androidx.compose.ui.geometry.Offset(
                                                            previousPosition.x,
                                                            previousPosition.y - layoutShift,
                                                        )
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } finally {
                                    dragDistance = 0f
                                    reorderOffset.floatValue = 0f
                                    reorderActiveIndex.intValue = -1
                                }
                            }
                        }
                        // Keep pointer coordinates in the stationary parent
                        // coordinate space while the row is translated. If
                        // pointerInput is inside graphicsLayer, local pointer
                        // positions shift with the row and the drag is
                        // undercounted.
                        .graphicsLayer {
                            val isDragging = reorderActiveIndex.intValue == itemIndex
                            translationY = if (isDragging) reorderOffset.floatValue else 0f
                            scaleX = if (isDragging) 1.02f else 1f
                            scaleY = if (isDragging) 1.02f else 1f
                            shadowElevation = if (isDragging) 8.dp.toPx() else 0f
                        }
                        .zIndex(if (reorderActiveIndex.intValue == itemIndex) 1f else 0f)
                } else Modifier
                val reorderableItem = onMove != null && itemAxis == "Vertical"
                val rowModifier = (if (reorderableItem) Modifier else itemParentModifier)
                    .then(itemModifier)
                if (reorderableItem) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .then(itemParentModifier)
                            .then(reorderModifier),
                    ) {
                        NexaNativeListRowPrimitive(position = itemIndex, count = count) {
                            renderListRow(children, rowLocals, rowModifier)
                            if (showDividers && itemIndex + 1 < count) {
                                NexaNativeListDividerPrimitive()
                            }
                        }
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
            val observeScroll: suspend (Int, Int, Int, Boolean, Boolean) -> Unit = { firstVisible, _, total, reachedEnd, isScrollInProgress ->
                val dataPosition = (firstVisible - stickyHeaderOffset).coerceIn(0, maxOf(0, total - 1))
                val logicalFirstVisible = if (reverseLayout) {
                    (total - 1 - dataPosition).coerceIn(0, maxOf(0, total - 1))
                } else dataPosition
                val previousScrollWasInProgress = scrollWasInProgress[0]
                scrollWasInProgress[0] = isScrollInProgress
                if (logicalFirstVisible != lastReportedScrollPosition) {
                    val isInitialPosition = lastReportedScrollPosition < 0
                    val previousPosition = lastReportedScrollPosition
                    lastReportedScrollPosition = logicalFirstVisible
                    if (scrollPositionName != null) {
                        val storedIndex = (store.state(scrollPositionName, scope) as? Number)?.toInt()
                        if (storedIndex != logicalFirstVisible) {
                            nativeScrollPositionEcho = logicalFirstVisible
                            store.setState(scrollPositionName, logicalFirstVisible, scope)
                        }
                    }
                    if (!isInitialPosition && onScroll != null && onScroll != JSONObject.NULL) {
                        val callbackCount = if (isScrollInProgress || previousScrollWasInProgress) {
                            kotlin.math.abs(logicalFirstVisible - previousPosition).coerceAtLeast(1)
                        } else 1
                        val direction = if (logicalFirstVisible > previousPosition) 1 else -1
                        repeat(callbackCount) { callbackIndex ->
                            val eventPosition = if (callbackCount == 1 && !(isScrollInProgress || previousScrollWasInProgress)) {
                                logicalFirstVisible
                            } else previousPosition + direction * (callbackIndex + 1)
                            val callbackLocals = locals + (indexName to eventPosition)
                            store.perform(onScroll, scope, callbackLocals)
                        }
                    }
                }
                val callbackLocals = locals + (indexName to logicalFirstVisible)
                if (onEndReached != null && onEndReached != JSONObject.NULL && total > 0 && reachedEnd && !endReached) {
                    endReached = true
                    val taskOwnedLocals = callbackLocals.toMutableMap()
                    listTaskOwner?.let { taskOwnedLocals[NEXA_DEV_TASK_OWNER_LOCAL] = it }
                    store.perform(onEndReached, scope, taskOwnedLocals)
                }
            }
            // Keep the observer collector attached to the list state instead
            // of restarting it when callback data is rebuilt by a DevRuntime
            // recomposition. A restarted snapshotFlow immediately emits the
            // current viewport and can be mistaken for a fresh scroll event.
            val currentObserveScroll = rememberUpdatedState(observeScroll)
            val density = LocalDensity.current
            val listContent: @Composable () -> Unit = {
                if (axis == "Grid" || axis == "AdaptiveGrid" || axis == "Masonry") {
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
                                    if (animated) listState.animateScrollBy(delta) else listState.scrollBy(delta)
                                }
                            },
                            scrollToOffset = { target, animated ->
                                val offset = with(density) { target.dp.roundToPx() }
                                if (animated) listState.animateScrollToItem(0, offset) else listState.scrollToItem(0, offset)
                            },
                        )
                        LaunchedEffect(listState, count) {
                            snapshotFlow {
                                val visibleItems = listState.layoutInfo.visibleItemsInfo
                                NexaDevScrollSnapshot(
                                    isScrollInProgress = listState.isScrollInProgress,
                                    leadingPosition = listState.firstVisibleItemIndex,
                                    visibleItemCount = visibleItems.size,
                                    reachedEnd = visibleItems.any { it.index >= count - 1 },
                                )
                            }.distinctUntilChanged().collect { snapshot ->
                                currentObserveScroll.value(
                                    snapshot.leadingPosition,
                                    snapshot.visibleItemCount,
                                    count,
                                    snapshot.reachedEnd,
                                    snapshot.isScrollInProgress,
                                )
                            }
                        }
                        LaunchedEffect(listState, requestedIndex, count) {
                            if (count > 0 && listState.firstVisibleItemIndex != requestedIndex) {
                                listState.scrollToItem(requestedIndex)
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
                            items(count = count, key = { itemIndex -> stableRowKey(itemIndex) }) { itemIndex ->
                                renderItem(itemIndex, axis, Modifier, null)
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
                                if (animated) listState.animateScrollBy(delta) else listState.scrollBy(delta)
                            }
                        },
                        scrollToOffset = { target, animated ->
                            val offset = with(density) { target.dp.roundToPx() }
                            if (animated) listState.animateScrollToItem(0, offset) else listState.scrollToItem(0, offset)
                        },
                    )
                    LaunchedEffect(listState, count) {
                        snapshotFlow {
                            val visibleItems = listState.layoutInfo.visibleItemsInfo
                            NexaDevScrollSnapshot(
                                isScrollInProgress = listState.isScrollInProgress,
                                leadingPosition = listState.firstVisibleItemIndex,
                                visibleItemCount = visibleItems.size,
                                reachedEnd = visibleItems.any { it.index >= count - 1 },
                            )
                        }
                            .distinctUntilChanged()
                            .collect { snapshot ->
                                currentObserveScroll.value(
                                    snapshot.leadingPosition,
                                    snapshot.visibleItemCount,
                                    count,
                                    snapshot.reachedEnd,
                                    snapshot.isScrollInProgress,
                                )
                            }
                    }
                    LaunchedEffect(listState, requestedIndex, count) {
                        if (count > 0 && listState.firstVisibleItemIndex != requestedIndex) {
                            listState.scrollToItem(requestedIndex)
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
                            renderItem(itemIndex, axis, Modifier, null)
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
                            val layoutIndex = if (stickyHeaderOffset > 0 && target == 0) 0 else target + stickyHeaderOffset
                            val clampedIndex = layoutIndex.coerceIn(0, count + stickyHeaderOffset - 1)
                            if (animated) listState.animateScrollToItem(clampedIndex) else listState.scrollToItem(clampedIndex)
                            val layout = listState.layoutInfo
                            val item = layout.visibleItemsInfo.firstOrNull { it.index == clampedIndex }
                            if (item != null) {
                                val horizontal = axis == "Horizontal"
                                val itemSize = if (horizontal) item.size else item.size
                                val itemOffset = if (horizontal) item.offset else item.offset
                                val viewportStart = layout.viewportStartOffset
                                val viewportEnd = layout.viewportEndOffset
                                val available = (viewportEnd - viewportStart - itemSize).coerceAtLeast(0)
                                val desired = viewportStart + (available * viewPosition).toInt()
                                val delta = (itemOffset - desired).toFloat()
                                if (animated) listState.animateScrollBy(delta) else listState.scrollBy(delta)
                            }
                        },
                        scrollToOffset = { target, animated ->
                            val offset = with(density) { target.dp.roundToPx() }
                            if (animated) listState.animateScrollToItem(0, offset) else listState.scrollToItem(0, offset)
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
                    LaunchedEffect(listState, count) {
                        if (pageSnap) {
                            snapshotFlow { listState.isScrollInProgress }
                                .distinctUntilChanged()
                                .collect { isScrolling ->
                                    if (!isScrolling) {
                                        val layoutInfo = listState.layoutInfo
                                        val visibleItems = layoutInfo.visibleItemsInfo
                                        currentObserveScroll.value(
                                            nexaDevLeadingVisibleDataItemIndex(
                                                visibleItems,
                                                listState.firstVisibleItemIndex,
                                                stickyHeaderOffset,
                                                reverseLayout,
                                                layoutInfo.viewportStartOffset,
                                                layoutInfo.viewportEndOffset,
                                            ),
                                            visibleItems.size,
                                            count,
                                            count > 0 && visibleItems.any { item ->
                                                item.index >= count + stickyHeaderOffset - 1 &&
                                                    nexaDevListItemIsUnobscuredByStickyHeader(
                                                        item,
                                                        visibleItems,
                                                        stickyHeaderOffset,
                                                        layoutInfo.viewportStartOffset,
                                                        layoutInfo.viewportEndOffset,
                                                    )
                                            },
                                            false,
                                        )
                                    }
                                }
                        } else {
                            snapshotFlow {
                                val layoutInfo = listState.layoutInfo
                                val visibleItems = layoutInfo.visibleItemsInfo
                                val visualLeadingPosition = nexaDevLeadingVisibleDataItemIndex(
                                    visibleItems,
                                    listState.firstVisibleItemIndex,
                                    stickyHeaderOffset,
                                    reverseLayout,
                                    layoutInfo.viewportStartOffset,
                                    layoutInfo.viewportEndOffset,
                                )
                                NexaDevScrollSnapshot(
                                        isScrollInProgress = listState.isScrollInProgress,
                                        leadingPosition = visualLeadingPosition,
                                        visibleItemCount = visibleItems.size,
                                        reachedEnd = count > 0 && visibleItems.any { item ->
                                            item.index >= count + stickyHeaderOffset - 1 &&
                                                nexaDevListItemIsUnobscuredByStickyHeader(
                                                    item,
                                                    visibleItems,
                                                    stickyHeaderOffset,
                                                    layoutInfo.viewportStartOffset,
                                                    layoutInfo.viewportEndOffset,
                                                )
                                        },
                                    )
                            }
                                .distinctUntilChanged()
                                .collect { snapshot ->
                                    currentObserveScroll.value(
                                        snapshot.leadingPosition,
                                        snapshot.visibleItemCount,
                                        count,
                                        snapshot.reachedEnd,
                                        snapshot.isScrollInProgress,
                                    )
                                }
                        }
                    }
                    LaunchedEffect(listState, requestedLayoutPosition, reverseLayout) {
                        if (nativeScrollPositionEcho >= 0) {
                            val isNativePositionEcho = requestedIndex == nativeScrollPositionEcho
                            nativeScrollPositionEcho = -1
                            if (isNativePositionEcho) return@LaunchedEffect
                        }
                        val visualLeadingPosition = if (reverseLayout) {
                            listState.layoutInfo.visibleItemsInfo.maxOfOrNull { it.index }
                                ?: listState.firstVisibleItemIndex
                        } else {
                            listState.firstVisibleItemIndex
                        }
                        if (count > 0 && visualLeadingPosition != requestedLayoutPosition) {
                            listState.scrollToItem(requestedLayoutPosition)
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
                                renderItem(itemIndex, axis, Modifier, null)
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
                            flingBehavior = if (pageSnap) {
                                rememberSnapFlingBehavior(
                                    lazyListState = listState,
                                    snapPosition = SnapPosition.Start,
                                )
                            } else androidx.compose.foundation.gestures.ScrollableDefaults.flingBehavior(),
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
                                            NexaScrollContentResetPrimitive {
                                                if (usesPlatformList && axis == "Vertical") {
                                                    NexaNativeListSectionHeaderPrimitive {
                                                        renderHeader(sectionHeader, locals + ((sectionName ?: "section") to sectionIndex))
                                                    }
                                                } else {
                                                    renderHeader(sectionHeader, locals + ((sectionName ?: "section") to sectionIndex))
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
                                                placementSpec = if ((reorderActiveIndex?.intValue ?: -1) < 0) {
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
                                    val rowParentModifier = if (
                                        animateStableRows && !pageSnap
                                    ) {
                                        Modifier.fillMaxWidth().animateItem(
                                            placementSpec = if ((reorderActiveIndex?.intValue ?: -1) < 0) {
                                                androidx.compose.animation.core.spring<androidx.compose.ui.unit.IntOffset>()
                                            } else {
                                                null
                                            },
                                        )
                                    } else {
                                        Modifier
                                    }
                                    if (pageSnap) {
                                        renderItem(itemPosition, axis, Modifier.fillParentMaxSize(), listState)
                                    } else if (onMove != null && axis == "Vertical") {
                                        renderItem(itemPosition, axis, rowParentModifier, listState)
                                    } else if (usesPlatformList && axis == "Vertical") {
                                        NexaNativeListRowPrimitive(position = itemPosition, count = count) {
                                            renderItem(itemPosition, axis, rowParentModifier, listState)
                                            if (showDividers && itemPosition + 1 < count) {
                                                NexaNativeListDividerPrimitive()
                                            }
                                        }
                                    } else {
                                        renderItem(itemPosition, axis, rowParentModifier, listState)
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
            if (refreshState != null) {
                PullToRefreshBox(
                    isRefreshing = store.state(refreshState, scope) as? Boolean ?: false,
                    onRefresh = { store.perform(refreshActions, scope, locals) },
                    modifier = modifier.fillMaxSize(),
                ) { listContent() }
            } else {
                listContent()
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
                                val tabIcon = nexaDevIconSpec(iconSelection)
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
                        val toolbarItems = nexaDevTabToolbars(children)
                        val contentChildren = nexaDevWithoutTabToolbars(children)
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
                val toolbarNode = nexaDevCompactToolbarPickers(node)
                RenderChildren(JSONArray().put(toolbarNode), module, store, locals, scope)
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
    val result = JSONObject()
    val keys = node.keys()
    while (keys.hasNext()) {
        val key = keys.next()
        val value = node.opt(key)
        val compactValue = when {
            key == "Picker" && value is JSONObject ->
                JSONObject(value.toString()).put("__nexa_fill_max_width", false)
            value is JSONObject -> nexaDevCompactToolbarPickers(value)
            value is JSONArray -> JSONArray().apply {
                for (index in 0 until value.length()) {
                    val item = value.opt(index)
                    put(if (item is JSONObject) nexaDevCompactToolbarPickers(item) else item)
                }
            }
            else -> value
        }
        result.put(key, compactValue)
    }
    return result
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
) {
    val cases = fields.optJSONArray("cases") ?: JSONArray()
    val matching = (0 until cases.length())
        .mapNotNull(cases::optJSONObject)
        .firstOrNull { item ->
            store.stringify(store.evaluate(item.opt("value"), locals, scope)) == target
        }
    RenderConditionalChildren(
        matching?.optJSONArray("body") ?: fields.optJSONArray("else_body") ?: JSONArray(),
        module,
        store,
        locals,
        scope,
    )
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
) {
    when (nodes.length()) {
        0 -> Unit
        1 -> RenderChildren(nodes, module, store, locals, scope)
        else -> NexaColumnPrimitive(
            spacing = 0f,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            RenderColumnChildren(nodes, module, store, locals, scope)
        }
    }
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
            val spacer = child.optJSONObject("Spacer")
            val fixedSpacer = spacer != null && (!spacer.isNull("width") || !spacer.isNull("height"))
            val modifier = if ((child.has("Spacer") && !fixedSpacer) || fillsRowWidth) Modifier.weight(1f) else Modifier
            NexaDevNode(child, module, store, locals, scope, modifier)
        }
    }
}
