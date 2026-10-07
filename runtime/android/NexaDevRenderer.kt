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
import androidx.compose.animation.core.FastOutLinearInEasing
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
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
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imeNestedScroll
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items as gridItems
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Badge
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.TextButton
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.DatePicker
import androidx.compose.material3.TimePicker
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material.icons.Icons
__NEXA_SHARED_ICON_AUTO_MIRRORED_IMPORTS__
__NEXA_SPECIFIC_MATERIAL_ICON_IMPORTS__
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteDefaults
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.input.pointer.util.addPointerInputChange
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
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
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import kotlin.math.roundToInt
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.zIndex
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import kotlinx.coroutines.flow.distinctUntilChanged
import org.json.JSONArray
import org.json.JSONObject
import kotlinx.coroutines.launch

private const val nexaDevDefaultBodyFontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__
private const val nexaDevDefaultLineHeightMultiplier = __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f
private const val nexaDevButtonMinWidth = __NEXA_BUTTON_MIN_WIDTH__
private const val nexaDevButtonMinTapTarget = __NEXA_BUTTON_MIN_TAP_TARGET__
private const val nexaDevButtonLargeMinHeight = __NEXA_BUTTON_LARGE_MIN_HEIGHT__
private const val nexaDevButtonSmallHorizontalPadding = __NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__
private const val nexaDevButtonSmallVerticalPadding = __NEXA_BUTTON_SMALL_VERTICAL_PADDING__
private const val nexaDevButtonLargeHorizontalPadding = __NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__
private const val nexaDevButtonLargeVerticalPadding = __NEXA_BUTTON_LARGE_VERTICAL_PADDING__
private const val nexaDevPageIndicatorSelectedSize = __NEXA_PAGE_INDICATOR_SELECTED_SIZE__
private const val nexaDevPageIndicatorUnselectedSize = __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__
private const val nexaDevPageIndicatorSpacing = __NEXA_PAGE_INDICATOR_SPACING__
private const val nexaDevPageIndicatorBottomInset = __NEXA_PAGE_INDICATOR_BOTTOM_INSET__
private const val nexaDevPageIndicatorInactiveOpacity = __NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__f

private fun nexaDevDefaultColorScheme(dark: Boolean) = if (dark) {
    darkColorScheme(
        primary = Color(__NEXA_DARK_ACCENT_ARGB__),
        onPrimary = Color.White,
        primaryContainer = Color(__NEXA_DARK_PRIMARY_CONTAINER_ARGB__),
        onPrimaryContainer = Color(__NEXA_DARK_ON_PRIMARY_CONTAINER_ARGB__),
        secondary = Color(__NEXA_MUTED_TEXT_ARGB__),
        onSecondary = Color.White,
        background = Color(__NEXA_DARK_BACKGROUND_ARGB__),
        onBackground = Color(__NEXA_DARK_ON_SURFACE_ARGB__),
        surface = Color(__NEXA_DARK_SURFACE_ARGB__),
        onSurface = Color(__NEXA_DARK_ON_SURFACE_ARGB__),
        surfaceVariant = Color(__NEXA_DARK_SURFACE_VARIANT_ARGB__),
        onSurfaceVariant = Color(__NEXA_MUTED_TEXT_ARGB__),
        outline = Color(__NEXA_DARK_OUTLINE_ARGB__),
        error = Color(__NEXA_DEFAULT_ERROR_ARGB__),
        onError = Color.White,
    )
} else {
    lightColorScheme(
        primary = Color(__NEXA_DEFAULT_ACCENT_ARGB__),
        onPrimary = Color.White,
        primaryContainer = Color(__NEXA_LIGHT_PRIMARY_CONTAINER_ARGB__),
        onPrimaryContainer = Color(__NEXA_LIGHT_ON_PRIMARY_CONTAINER_ARGB__),
        secondary = Color(__NEXA_MUTED_TEXT_ARGB__),
        onSecondary = Color.White,
        background = Color(__NEXA_LIGHT_BACKGROUND_ARGB__),
        onBackground = Color(__NEXA_LIGHT_ON_SURFACE_ARGB__),
        surface = Color(__NEXA_LIGHT_SURFACE_ARGB__),
        onSurface = Color(__NEXA_LIGHT_ON_SURFACE_ARGB__),
        surfaceVariant = Color(__NEXA_LIGHT_SURFACE_VARIANT_ARGB__),
        onSurfaceVariant = Color(__NEXA_MUTED_TEXT_ARGB__),
        outline = Color(__NEXA_LIGHT_OUTLINE_ARGB__),
        error = Color(__NEXA_DEFAULT_ERROR_ARGB__),
        onError = Color.White,
    )
}

private fun nexaDevSharedMaterialIcon(name: String) = when (name) {
__NEXA_SHARED_ICON_MATERIAL_CASES__
    else -> Icons.Filled.Star
}

private fun nexaDevSfAliasMaterialIcon(name: String) = when (name) {
__NEXA_SHARED_ICON_SF_ALIAS_CASES__
    else -> null
}

private fun nexaDevSpecificMaterialIcon(name: String) = when (name) {
__NEXA_SPECIFIC_MATERIAL_ICON_CASES__
    else -> null
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

@Composable
internal fun NexaDevStateStore.evaluatePresented(
    raw: Any?,
    locals: Map<String, Any>,
    scope: String,
): Any {
    val tagged = raw as? JSONObject ?: return evaluate(raw, locals, scope)
    val payload = tagged.optJSONArray("AnimatedState") ?: return evaluate(raw, locals, scope)
    val name = payload.optString(0)
    if (name.isEmpty()) return evaluate(raw, locals, scope)
    val animation = animatedFloatState(name, scope)
    val type = payload.optJSONObject(1)?.optString("Numeric")
    return if (type == "Float64") animation.toDouble() else animation
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

@Composable
internal fun NexaDevNodeList(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    parameters: Map<String, Any> = emptyMap(),
    scope: String = "app",
    modifier: Modifier = Modifier,
) {
    val locals = store.locals(scope, parameters)
    when (nodes.length()) {
        0 -> Unit
        1 -> NexaDevNode(nexaDevNodeObject(nodes.opt(0)), module, store, locals, scope, modifier)
        else -> Column(
            modifier = modifier,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            RenderColumnChildren(nodes, module, store, locals, scope)
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
            val modifier = if (child.has("FastList") || child.has("RefreshControl") || child.has("Spacer")) Modifier.weight(1f) else Modifier
            NexaDevNode(child, module, store, locals, scope, modifier)
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
) {
    val kind = node.keys().asSequence().firstOrNull() ?: return
    val fields = node.optJSONObject(kind) ?: JSONObject()
    when (kind) {
        "Appearance" -> {
            val mode = store.stringify(store.evaluatePresented(fields.opt("mode") ?: "system", locals, scope))
            val systemDark = isSystemInDarkTheme()
            val scheme = remember(mode, systemDark) {
                when (mode) {
                    "dark" -> nexaDevDefaultColorScheme(true)
                    "light" -> nexaDevDefaultColorScheme(false)
                    else -> nexaDevDefaultColorScheme(systemDark)
                }
            }
            MaterialTheme(colorScheme = scheme) {
                NexaDevNodeList(fields.optJSONArray("children") ?: JSONArray(), module, store, parameters = locals, scope = scope)
            }
        }
        "Form" -> {
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                RenderColumnChildren(
                    fields.optJSONArray("children") ?: JSONArray(),
                    module,
                    store,
                    locals,
                    scope,
                )
            }
        }
        "FormSection" -> {
            Column(Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
                if (!fields.isNull("title")) {
                    Text(
                        store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope)),
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                        style = MaterialTheme.typography.titleSmall,
                    )
                }
                RenderColumnChildren(
                    fields.optJSONArray("children") ?: JSONArray(),
                    module,
                    store,
                    locals,
                    scope,
                )
                if (!fields.isNull("footer")) {
                    Text(
                        store.stringify(store.evaluatePresented(fields.opt("footer"), locals, scope)),
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                        style = MaterialTheme.typography.bodySmall,
                    )
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
            CompositionLocalProvider(LocalNexaDevContentSlot provides slot) {
                NexaDevNodeList(
                    resolved.optJSONArray("body") ?: JSONArray(),
                    module,
                    store,
                    parameters = parameters,
                    scope = "component/$name",
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
            val rendered = NexaDevPluginBridge.renderComponent(namespace, name, arguments, events) {
                NexaDevNodeList(children, module, store, parameters = locals, scope = scope)
            }
            if (!rendered) android.util.Log.w("NexaDevRuntime", "Native component is unsupported: $namespace.$name")
        }
        "Layout" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            val spacing = fields.optDouble("spacing", 0.0).dp
            val style = fields.optJSONObject("style") ?: JSONObject()
            var modifier: Modifier = Modifier
            val effects = style.optJSONObject("effects") ?: JSONObject()
            if (style.has("opacity")) modifier = modifier.alpha(style.optDouble("opacity").toFloat())
            if (!effects.isNull("scale") || !effects.isNull("rotation")) {
                val scale = effects.optDouble("scale", 1.0).toFloat()
                val rotation = effects.optDouble("rotation", 0.0).toFloat()
                modifier = modifier.graphicsLayer(scaleX = scale, scaleY = scale, rotationZ = rotation)
            }
            if (!effects.isNull("blur")) modifier = modifier.blur(effects.optDouble("blur").dp)
            effects.optJSONObject("shadow")?.let { shadow ->
                val radius = effects.optDouble("clip_rounded", style.optDouble("corner_radius", 0.0)).dp
                val color = nexaDevColor(shadow.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Black
                modifier = modifier.dropShadow(
                    RoundedCornerShape(radius),
                    Shadow(
                        radius = shadow.optDouble("radius", 0.0).dp,
                        color = color,
                        offset = DpOffset(shadow.optDouble("x", 0.0).dp, shadow.optDouble("y", 0.0).dp),
                    ),
                )
            }
            val cornerRadius = style.optDouble("corner_radius", 0.0).dp
            val clipRadius = if (!effects.isNull("clip_rounded")) effects.optDouble("clip_rounded").dp else cornerRadius
            if (!effects.isNull("clip_rounded") || !style.isNull("corner_radius")) modifier = modifier.clip(RoundedCornerShape(clipRadius))
            nexaDevColor(style.optJSONObject("background"), isSystemInDarkTheme())?.let { modifier = modifier.background(it) }
            if (style.has("border_color") && style.has("border_width")) {
                nexaDevColor(style.optJSONObject("border_color"), isSystemInDarkTheme())?.let {
                    modifier = modifier.border(style.optDouble("border_width").dp, it, RoundedCornerShape(cornerRadius))
                }
            }
            if (style.has("padding")) modifier = modifier.padding(style.optDouble("padding").dp)
            if (style.has("width")) modifier = modifier.width(style.optDouble("width").dp)
            if (style.has("height")) modifier = modifier.height(style.optDouble("height").dp)
            if (style.has("min_width") || style.has("max_width")) modifier = modifier.widthIn(
                min = style.optDouble("min_width").takeIf { style.has("min_width") }?.dp ?: Dp.Unspecified,
                max = style.optDouble("max_width").takeIf { style.has("max_width") }?.dp ?: Dp.Unspecified,
            )
            if (style.has("min_height") || style.has("max_height")) modifier = modifier.heightIn(
                min = style.optDouble("min_height").takeIf { style.has("min_height") }?.dp ?: Dp.Unspecified,
                max = style.optDouble("max_height").takeIf { style.has("max_height") }?.dp ?: Dp.Unspecified,
            )
            if (!effects.isNull("z_index")) modifier = modifier.zIndex(effects.optInt("z_index").toFloat())
            if (style.has("animation")) {
                val springConfig = style.optJSONObject("animation")?.optJSONObject("Spring")
                if (springConfig != null) {
                    val response = springConfig.optDouble("response", 0.5)
                    val damping = springConfig.optDouble("damping", 0.825).toFloat()
                    val stiffness = (100.0 / (response * response)).toFloat()
                    modifier = modifier.animateContentSize(
                        animationSpec = spring(
                            dampingRatio = damping,
                            stiffness = stiffness,
                        ),
                    )
                } else {
                    val easing = when (style.optString("animation")) {
                        "EaseIn" -> FastOutLinearInEasing
                        "EaseOut" -> LinearOutSlowInEasing
                        "EaseInOut" -> FastOutSlowInEasing
                        else -> LinearEasing
                    }
                    modifier = modifier.animateContentSize(animationSpec = tween(easing = easing))
                }
            }
            when (fields.optString("kind")) {
                "Row" -> Row(
                    modifier,
                    horizontalArrangement = if (spacing > 0.dp) Arrangement.spacedBy(spacing) else Arrangement.Start,
                    verticalAlignment = when (style.optString("alignment")) {
                        "Start" -> Alignment.Top
                        "End" -> Alignment.Bottom
                        else -> Alignment.CenterVertically
                    },
                ) { RenderRowChildren(children, module, store, locals, scope) }
                "Stack" -> Box(
                    modifier,
                    contentAlignment = when (style.optString("alignment")) {
                        "Start" -> Alignment.TopStart
                        "End" -> Alignment.BottomEnd
                        else -> Alignment.Center
                    },
                ) { RenderChildren(children, module, store, locals, scope) }
                else -> Column(
                    modifier,
                    verticalArrangement = if (spacing > 0.dp) Arrangement.spacedBy(spacing) else Arrangement.Top,
                    horizontalAlignment = when (style.optString("alignment")) {
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
            val description = store.stringify(store.evaluatePresented(fields.opt("description"), locals, scope))
            val icon = fields.optJSONObject("icon") ?: JSONObject()
            val image = nexaDevSharedMaterialIcon(icon.optString("shared"))
            Column(
                modifier = modifier.fillMaxSize().padding(32.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Icon(
                    imageVector = image,
                    contentDescription = null,
                    modifier = Modifier.size(36.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(title, style = MaterialTheme.typography.titleMedium, textAlign = TextAlign.Center)
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    description,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }
        }
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
                    else -> TextAlign.Unspecified
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
                val semanticStyle = LocalTextStyle.current
                val fontSize = if (style.has("font_size") && !style.isNull("font_size")) {
                    style.getDouble("font_size").sp
                } else {
                    semanticMetrics?.first ?: nexaDevDefaultBodyFontSize.sp
                }
                val fontWeight = weight ?: semanticMetrics?.second ?: FontWeight.Normal
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
                if (!effects.isNull("scale") || !effects.isNull("rotation")) {
                    val scale = effects.optDouble("scale", 1.0).toFloat()
                    val rotation = effects.optDouble("rotation", 0.0).toFloat()
                    modifier = modifier.graphicsLayer(scaleX = scale, scaleY = scale, rotationZ = rotation)
                }
                if (!effects.isNull("blur")) modifier = modifier.blur(effects.optDouble("blur").dp)
                effects.optJSONObject("shadow")?.let { shadow ->
                    val radius = effects.optDouble("clip_rounded", 0.0).dp
                    val color = nexaDevColor(shadow.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Black
                    modifier = modifier.dropShadow(
                        RoundedCornerShape(radius),
                        Shadow(
                            radius = shadow.optDouble("radius", 0.0).dp,
                            color = color,
                            offset = DpOffset(shadow.optDouble("x", 0.0).dp, shadow.optDouble("y", 0.0).dp),
                        ),
                    )
                }
                if (!effects.isNull("clip_rounded")) modifier = modifier.clip(RoundedCornerShape(effects.optDouble("clip_rounded").dp))
                if (!effects.isNull("z_index")) modifier = modifier.zIndex(effects.optInt("z_index").toFloat())
                Text(
                    text = text,
                    modifier = modifier,
                    color = color ?: Color.Unspecified,
                    style = semanticStyle,
                    fontSize = fontSize,
                    fontWeight = fontWeight,
                    textAlign = textAlign,
                    maxLines = maxLines,
                    textDecoration = if (style.optBoolean("strikethrough")) TextDecoration.LineThrough else null,
                    lineHeight = lineHeight,
                    letterSpacing = letterSpacing,
                )
            }
            if (style.optBoolean("selectable")) SelectionContainer { content() } else content()
        }
        "Spacer" -> Spacer(modifier = modifier)
        "Divider" -> HorizontalDivider(
            color = nexaDevColor(fields.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Gray,
            thickness = (fields.optDouble("thickness", 1.0)).dp,
        )
        "SystemIcon" -> {
            val selection = fields.optJSONObject("icon") ?: JSONObject()
            val icon = if (selection.has("shared")) {
                nexaDevSharedMaterialIcon(selection.optString("shared"))
            } else if (selection.has("sf_symbol")) {
                nexaDevSfAliasMaterialIcon(selection.optString("sf_symbol")) ?: Icons.Filled.Star
            } else if (selection.has("material_symbol")) {
                nexaDevSpecificMaterialIcon(selection.optString("material_symbol")) ?: Icons.Filled.Star
            } else {
                Icons.Filled.Star
            }
            val tintValue = fields.optJSONObject("tint") ?: JSONObject()
            val dynamicTint = tintValue.opt("Dynamic")
            val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope) ?: ""))
            } else {
                nexaDevColor(tintValue, isSystemInDarkTheme())
            }
            Icon(
                imageVector = icon,
                contentDescription = fields.optString("description").takeIf(String::isNotEmpty),
                modifier = modifier.size(fields.optDouble("size", 24.0).dp),
                tint = tint ?: Color.White,
            )
        }
        "LinearGradient" -> {
            val start = nexaDevColor(fields.optJSONObject("start_color"), isSystemInDarkTheme()) ?: Color.Transparent
            val end = nexaDevColor(fields.optJSONObject("end_color"), isSystemInDarkTheme()) ?: Color.Transparent
            val direction = fields.optString("direction", "TopToBottom")
            val colors = if (direction == "BottomToTop" || direction == "TrailingToLeading") listOf(end, start) else listOf(start, end)
            val brush = if (direction == "LeadingToTrailing" || direction == "TrailingToLeading") {
                Brush.horizontalGradient(colors)
            } else {
                Brush.verticalGradient(colors)
            }
            Spacer(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(fields.optDouble("height", 180.0).dp)
                    .background(brush),
            )
        }
        "Button" -> {
            val loading = store.evaluatePresented(fields.opt("loading"), locals, scope) as? Boolean ?: false
            val disabled = store.evaluatePresented(fields.opt("disabled"), locals, scope) as? Boolean ?: false
            val style = fields.optString("style")
            val tintValue = fields.optJSONObject("tint") ?: JSONObject()
            val dynamicTint = tintValue.opt("Dynamic")
            val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope) ?: ""))
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
            val contentPadding = when (fields.optString("size")) {
                "Small" -> PaddingValues(
                    horizontal = nexaDevButtonSmallHorizontalPadding.dp,
                    vertical = nexaDevButtonSmallVerticalPadding.dp,
                )
                "Large" -> PaddingValues(
                    horizontal = nexaDevButtonLargeHorizontalPadding.dp,
                    vertical = nexaDevButtonLargeVerticalPadding.dp,
                )
                else -> if (style == "Borderless" || style == "Plain") PaddingValues(0.dp) else null
            }
            val minButtonHeight = if (fields.optString("size") == "Large") {
                nexaDevButtonLargeMinHeight.dp
            } else {
                nexaDevButtonMinTapTarget.dp
            }
            val iconSelection = fields.optJSONObject("icon") ?: JSONObject()
            val icon = when {
                iconSelection.has("shared") -> nexaDevSharedMaterialIcon(iconSelection.optString("shared"))
                iconSelection.has("sf_symbol") -> nexaDevSfAliasMaterialIcon(iconSelection.optString("sf_symbol"))
                iconSelection.has("material_symbol") -> nexaDevSpecificMaterialIcon(iconSelection.optString("material_symbol"))
                else -> null
            }
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            val buttonContent: @Composable RowScope.() -> Unit = {
                if (loading) {
                    CircularProgressIndicator()
                } else {
                    if (icon != null) Icon(imageVector = icon, contentDescription = null)
                    Text(
                        label,
                        fontSize = nexaDevDefaultBodyFontSize.sp,
                        fontWeight = FontWeight.Normal,
                        lineHeight = nexaDevDefaultBodyFontSize.sp * nexaDevDefaultLineHeightMultiplier,
                        letterSpacing = 0.sp,
                    )
                }
            }
            val onClick = { store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals) }
            when (style) {
                "Borderless", "Plain" -> TextButton(
                    modifier = Modifier.defaultMinSize(
                        minWidth = nexaDevButtonMinWidth.dp,
                        minHeight = minButtonHeight,
                    ),
                    onClick = onClick,
                    enabled = !loading && !disabled,
                    shape = buttonShape ?: RoundedCornerShape(percent = 50),
                    colors = androidx.compose.material3.ButtonDefaults.textButtonColors(
                        contentColor = tint ?: Color.Unspecified,
                    ),
                    contentPadding = contentPadding ?: androidx.compose.material3.ButtonDefaults.TextButtonContentPadding,
                    content = buttonContent,
                )
                "Bordered" -> OutlinedButton(
                    modifier = Modifier.defaultMinSize(
                        minWidth = nexaDevButtonMinWidth.dp,
                        minHeight = minButtonHeight,
                    ),
                    onClick = onClick,
                    enabled = !loading && !disabled,
                    shape = buttonShape ?: RoundedCornerShape(percent = 50),
                    colors = androidx.compose.material3.ButtonDefaults.outlinedButtonColors(
                        contentColor = tint ?: Color.Unspecified,
                    ),
                    border = tint?.let { androidx.compose.foundation.BorderStroke(1.dp, it) }
                        ?: androidx.compose.material3.ButtonDefaults.outlinedButtonBorder(enabled = !loading && !disabled),
                    contentPadding = contentPadding ?: androidx.compose.material3.ButtonDefaults.ContentPadding,
                    content = buttonContent,
                )
                else -> Button(
                    modifier = Modifier.defaultMinSize(
                        minWidth = nexaDevButtonMinWidth.dp,
                        minHeight = minButtonHeight,
                    ),
                    onClick = onClick,
                    enabled = !loading && !disabled,
                    shape = buttonShape ?: RoundedCornerShape(percent = 50),
                    colors = androidx.compose.material3.ButtonDefaults.buttonColors(
                        containerColor = tint ?: Color.Unspecified,
                    ),
                    contentPadding = contentPadding ?: androidx.compose.material3.ButtonDefaults.ContentPadding,
                    content = buttonContent,
                )
            }
        }
        "Pressable" -> {
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
            val pinchParameter = fields.optString(NexaDevKeys.PINCH_PARAMETER).takeIf(String::isNotEmpty)
            val pinchActions = fields.optJSONArray(NexaDevKeys.PINCH_ACTIONS) ?: JSONArray()
            val hapticStyle = fields.optString("haptic").takeIf(String::isNotEmpty)
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
            val pressableModifier = if (fillMaxSize) dragModifier.fillMaxSize() else dragModifier
            val pressableCore: @Composable () -> Unit = {
                Box(pressableModifier.combinedClickable(
                        enabled = !disabled,
                        interactionSource = null,
                        indication = null,
                        onClick = {
                            if (hapticType != null) haptic.performHapticFeedback(hapticType)
                            store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals)
                        },
                        onLongClick = {
                            contextMenuExpanded?.value = true
                            store.perform(longPressActions, scope, locals)
                        },
                        onDoubleClick = {
                            if (hapticType != null) haptic.performHapticFeedback(hapticType)
                            store.perform(doubleTapActions, scope, locals)
                        },
                )) {
                    RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
                }
            }
            val pressableContent: @Composable () -> Unit = {
                if (contextMenuActions.length() == 0) {
                    pressableCore()
                } else {
                    Box {
                        pressableCore()
                        DropdownMenu(
                            expanded = contextMenuExpanded?.value == true,
                            onDismissRequest = { contextMenuExpanded?.value = false },
                        ) {
                            for (index in 0 until contextMenuActions.length()) {
                                val menuNode = contextMenuActions.optJSONObject(index) ?: continue
                                val menuButton = menuNode.optJSONObject("Button") ?: continue
                                val label = store.stringify(store.evaluatePresented(menuButton.opt("label"), locals, scope))
                                val disabledAction = store.evaluatePresented(menuButton.opt("disabled"), locals, scope) as? Boolean ?: false
                                DropdownMenuItem(
                                    text = { Text(label) },
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
        "TextInput" -> {
            val state = fields.optString("state")
            val value = store.stringify(store.state(state, scope))
            val sourcePlaceholder = fields.optString("placeholder")
            val placeholder = sourcePlaceholder.takeIf(String::isNotEmpty)
                ?.let { store.localizedText(it, it) }
            val focusKey = "$scope/input/$state"
            val focusRequester = remember(focusKey) { FocusRequester() }
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
            val textStyle = when (fields.optString("font")) {
                "Body" -> MaterialTheme.typography.bodyLarge
                "Title3" -> MaterialTheme.typography.titleMedium
                else -> LocalTextStyle.current
            }
            val onChange = fields.optJSONObject("on_change")
            var fieldModifier: Modifier = Modifier
                .focusRequester(focusRequester)
                .onFocusChanged { focusState ->
                    if (focusState.isFocused) store.focusChanged(focusKey)
                    else if (store.focusedFieldKey == focusKey) store.focusChanged(null)
                }
            fieldModifier = when (autofill) {
                "Username" -> fieldModifier.semantics { contentType = ContentType.Username }
                "Password" -> fieldModifier.semantics { contentType = ContentType.Password }
                "OneTimeCode" -> fieldModifier.semantics { contentType = ContentType.SmsOtpCode }
                else -> fieldModifier
            }
            LaunchedEffect(store.moduleRevision, store.focusedFieldKey, focusKey) {
                if (store.focusedFieldKey == focusKey) focusRequester.requestFocus()
            }
            TextField(
                value = value,
                onValueChange = { next ->
                    val updated = next.take(maxLength)
                    store.setState(state, updated, scope)
                    val parameter = onChange?.optString("parameter")?.takeIf(String::isNotEmpty)
                    if (parameter != null) {
                        val changeLocals = locals.toMutableMap()
                        changeLocals[parameter] = updated
                        store.perform(onChange.optJSONArray("actions") ?: JSONArray(), scope, changeLocals)
                    }
                },
                modifier = fieldModifier.then(modifier),
                placeholder = placeholder?.let { { Text(it) } },
                textStyle = textStyle,
                minLines = minLines,
                maxLines = maxLines,
                leadingIcon = if (fields.optBoolean("searchable")) {
                    { Icon(nexaDevSharedMaterialIcon("search"), contentDescription = null) }
                } else null,
                singleLine = !fields.optBoolean("multiline"),
                visualTransformation = if (fields.optBoolean("secure")) PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
                keyboardOptions = KeyboardOptions(
                    keyboardType = keyboardType,
                    capitalization = capitalization,
                    autoCorrect = fields.optBoolean("autocorrect", true),
                    imeAction = imeAction,
                ),
                keyboardActions = if (submitActions.length() == 0) KeyboardActions() else when (imeAction) {
                    ImeAction.Search -> KeyboardActions(onSearch = { store.perform(submitActions, scope, locals) })
                    ImeAction.Send -> KeyboardActions(onSend = { store.perform(submitActions, scope, locals) })
                    ImeAction.Next -> KeyboardActions(onNext = { store.perform(submitActions, scope, locals) })
                    else -> KeyboardActions(onDone = { store.perform(submitActions, scope, locals) })
                },
            )
        }
        "FastList" -> {
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
            val axis = when {
                axisValue == "Horizontal" -> "Horizontal"
                gridColumns != null -> "Grid"
                else -> "Vertical"
            }
            val reverseLayout = options?.optBoolean(NexaDevKeys.REVERSE_LAYOUT, false) ?: false
            val pageSnap = options?.optBoolean(NexaDevKeys.PAGE_SNAP, false) ?: false
            if (options == null || countExpression == null && sourceItems == null && sourceSections == null) {
                Text("FastList source could not be evaluated.")
            } else {
                val itemCount = countExpression?.let {
                    (store.evaluatePresented(it, locals, scope) as? Number)?.toInt()?.coerceAtLeast(0) ?: 0
                } ?: sourceItems?.size ?: 0
                val count = sourceSections?.sumOf { (it as? List<*>)?.size ?: 0 } ?: itemCount
                val indexName = options.optString("index", "index")
                val itemName = listPlan.optString("item").takeIf(String::isNotEmpty)
                val sectionName = listPlan.optString("section").takeIf(String::isNotEmpty)
                val children = options.optJSONArray("children") ?: JSONArray()
                val stickyHeaderNodes = options.optJSONArray("sticky_header")
                val sectionHeader = options.optJSONArray("section_header")
                val onScroll = options.optJSONArray("on_scroll")
                val onEndReached = options.optJSONArray("on_end_reached")
                val onMove = options.optJSONObject("on_move")
                val moveEnabled = onMove?.opt("enabled")?.let {
                    store.evaluatePresented(it, locals, scope) as? Boolean
                } ?: true
                val refresh = options.optJSONObject("refresh")
                val refreshState = refresh?.optString("state")?.takeIf(String::isNotEmpty)
                val refreshActions = refresh?.optJSONArray("actions") ?: JSONArray()
                var endReached by remember(countExpression, sourceItems?.size, sourceSections?.size) { mutableStateOf(false) }
                val scrollPositionName = options.optString("scroll_position").takeIf(String::isNotEmpty)
                val requestedIndex = scrollPositionName?.let { name ->
                    (store.state(name, scope) as? Number)?.toInt()?.coerceIn(0, maxOf(0, count - 1)) ?: 0
                } ?: if (reverseLayout) maxOf(0, count - 1) else 0
                val requestedPosition = if (reverseLayout) {
                    (count - 1 - requestedIndex).coerceIn(0, maxOf(0, count - 1))
                } else requestedIndex
                val extent = options.optDouble("item_extent", 0.0).takeIf { it > 0.0 }?.dp
                val renderItem: @Composable (Int, String) -> Unit = { itemIndex, itemAxis ->
                    var rowLocals = locals + (indexName to itemIndex)
                    if (itemName != null && sourceItems != null) {
                        rowLocals = rowLocals + (itemName to (sourceItems.getOrNull(itemIndex) ?: JSONObject.NULL))
                    }
                    val itemModifier = when (itemAxis) {
                        "Horizontal" -> extent?.let { Modifier.widthIn(min = it) } ?: Modifier
                        "Grid" -> Modifier.fillMaxWidth().then(extent?.let { Modifier.heightIn(min = it) } ?: Modifier)
                        else -> Modifier.fillMaxWidth().then(extent?.let { Modifier.heightIn(min = it) } ?: Modifier)
                    }
                    val reorderModifier = if (onMove != null && moveEnabled && itemAxis == "Vertical") {
                        val rowExtentPx = with(LocalDensity.current) { (extent ?: 56.dp).toPx() }
                        Modifier.pointerInput(itemIndex, count, onMove) {
                            var dragDistance = 0f
                            detectDragGesturesAfterLongPress(
                                onDragEnd = {
                                    val from = itemIndex
                                    val to = (from + (dragDistance / rowExtentPx).roundToInt()).coerceIn(0, count - 1)
                                    if (to != from) {
                                        val callbackLocals = locals +
                                            (onMove.optString("from") to from) +
                                            (onMove.optString("to") to to)
                                        store.perform(onMove.optJSONArray("actions") ?: JSONArray(), scope, callbackLocals)
                                    }
                                },
                                onDragCancel = { dragDistance = 0f },
                                onDrag = { change, amount ->
                                    dragDistance += amount.y
                                    change.consume()
                                },
                            )
                        }
                    } else Modifier
                    Column(itemModifier.then(reorderModifier)) {
                        RenderChildren(children, module, store, rowLocals, scope)
                    }
                }
                val renderHeader: @Composable (JSONArray, Map<String, Any>) -> Unit = { nodes, headerLocals ->
                    Column(Modifier.fillMaxWidth()) {
                        RenderChildren(nodes, module, store, headerLocals, scope)
                    }
                }
                val observeScroll: suspend (Int, Int, Int) -> Unit = { firstVisible, visibleCount, total ->
                    val logicalFirstVisible = if (reverseLayout) {
                        (total - 1 - firstVisible).coerceAtLeast(0)
                    } else firstVisible
                    val callbackLocals = locals + (indexName to logicalFirstVisible)
                    if (scrollPositionName != null) {
                        val storedIndex = (store.state(scrollPositionName, scope) as? Number)?.toInt()
                        if (storedIndex != logicalFirstVisible) store.setState(scrollPositionName, logicalFirstVisible, scope)
                    }
                    if (onScroll != null && onScroll != JSONObject.NULL) store.perform(onScroll, scope, callbackLocals)
                    if (onEndReached != null && onEndReached != JSONObject.NULL) {
                        val reached = total > 0 && firstVisible + visibleCount >= total
                        if (reached && !endReached) store.perform(onEndReached, scope, callbackLocals)
                        endReached = reached
                    }
                }
                val listContent: @Composable () -> Unit = {
                    if (axis == "Grid") {
                        val listState = rememberLazyGridState(initialFirstVisibleItemIndex = requestedIndex)
                        val itemIndices = remember(count) { List(count) { it } }
                        LaunchedEffect(listState, scrollPositionName, onScroll, onEndReached, count) {
                            snapshotFlow { Pair(listState.firstVisibleItemIndex, listState.layoutInfo.visibleItemsInfo.size) }
                                .distinctUntilChanged()
                                .collect { (firstVisible, visibleCount) -> observeScroll(firstVisible, visibleCount, count) }
                        }
                        LaunchedEffect(listState, requestedIndex, count) {
                            if (count > 0 && listState.firstVisibleItemIndex != requestedIndex) {
                                listState.scrollToItem(requestedIndex)
                            }
                        }
                        LazyVerticalGrid(
                            columns = GridCells.Fixed(gridColumns ?: 1),
                            state = listState,
                            modifier = modifier.fillMaxWidth(),
                        ) {
                            gridItems(items = itemIndices, key = { itemIndex -> itemIndex }) { itemIndex ->
                                renderItem(itemIndex, axis)
                            }
                        }
                    } else {
                        val listState = rememberLazyListState(initialFirstVisibleItemIndex = requestedPosition)
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
                        LaunchedEffect(listState, scrollPositionName, onScroll, onEndReached, count) {
                            if (pageSnap) {
                                snapshotFlow { listState.isScrollInProgress }
                                    .distinctUntilChanged()
                                    .collect { isScrolling ->
                                        if (!isScrolling) {
                                            observeScroll(
                                                listState.firstVisibleItemIndex,
                                                listState.layoutInfo.visibleItemsInfo.size,
                                                count,
                                            )
                                        }
                                    }
                            } else {
                                snapshotFlow { Pair(listState.firstVisibleItemIndex, listState.layoutInfo.visibleItemsInfo.size) }
                                    .distinctUntilChanged()
                                    .collect { (firstVisible, visibleCount) -> observeScroll(firstVisible, visibleCount, count) }
                            }
                        }
                        LaunchedEffect(listState, requestedPosition) {
                            if (count > 0 && listState.firstVisibleItemIndex != requestedPosition) {
                                listState.scrollToItem(requestedPosition)
                            }
                        }
                        if (axis == "Horizontal") {
                            LazyRow(state = listState, modifier = modifier.fillMaxWidth()) {
                                items(count = count, key = { itemIndex -> itemIndex }) { itemIndex ->
                                    renderItem(itemIndex, axis)
                                }
                            }
                        } else {
                            LazyColumn(
                                state = listState,
                                reverseLayout = reverseLayout,
                                flingBehavior = if (pageSnap) {
                                    rememberSnapFlingBehavior(
                                        lazyListState = listState,
                                        snapPosition = SnapPosition.Start,
                                    )
                                } else androidx.compose.foundation.gestures.ScrollableDefaults.flingBehavior(),
                                modifier = modifier.fillMaxWidth(),
                            ) {
                                if (stickyHeaderNodes != null && stickyHeaderNodes != JSONObject.NULL) {
                                    stickyHeader { renderHeader(stickyHeaderNodes, locals) }
                                }
                                if (sourceSections != null) {
                                    sourceSections.forEachIndexed { sectionIndex, rawSection ->
                                        val sectionItems = rawSection as? List<*> ?: emptyList<Any?>()
                                        if (sectionHeader != null && sectionHeader != JSONObject.NULL) {
                                            stickyHeader {
                                                renderHeader(sectionHeader, locals + ((sectionName ?: "section") to sectionIndex))
                                            }
                                        }
                                        items(count = sectionItems.size, key = { itemIndex -> "${sectionIndex}:$itemIndex" }) { itemIndex ->
                                            var rowLocals = locals + (indexName to itemIndex) + ((sectionName ?: "section") to sectionIndex)
                                            if (itemName != null) rowLocals = rowLocals + (itemName to (sectionItems.getOrNull(itemIndex) ?: JSONObject.NULL))
                                            Column(Modifier.fillMaxWidth().then(extent?.let { Modifier.heightIn(min = it) } ?: Modifier)) {
                                                RenderChildren(children, module, store, rowLocals, scope)
                                            }
                                        }
                                    }
                                } else {
                                    items(
                                        count = count,
                                        key = { itemPosition ->
                                            if (reverseLayout) count - 1 - itemPosition else itemPosition
                                        },
                                    ) { itemPosition ->
                                        val itemIndex = if (reverseLayout) count - 1 - itemPosition else itemPosition
                                        if (pageSnap) {
                                            Box(Modifier.fillParentMaxSize()) {
                                                renderItem(itemIndex, axis)
                                            }
                                        } else {
                                            renderItem(itemIndex, axis)
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
                        modifier = modifier.fillMaxWidth(),
                    ) { listContent() }
                } else {
                    listContent()
                }
            }
        }
        "Switch" -> {
            val state = fields.optString("state")
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            Row {
                Text(label)
                Switch(checked = store.state(state, scope) as? Boolean ?: false, onCheckedChange = { store.setState(state, it, scope) })
            }
        }
        "Slider" -> {
            val state = fields.optString("state")
            val min = fields.optDouble("min").toFloat()
            val max = fields.optDouble("max").toFloat()
            val step = fields.optDouble("step").toFloat()
            val intervals = if (step > 0f) ((max - min) / step).roundToInt().coerceAtLeast(1) else 1
            val rawValue = if (fields.optBoolean("animated")) {
                store.animatedFloatState(state, scope)
            } else {
                (store.state(state, scope) as? Number)?.toFloat() ?: min
            }
            val value = rawValue.coerceIn(min, max)
            Slider(
                value = value,
                onValueChange = {
                    store.clearAnimationSpec(state, scope)
                    store.setState(state, it.toDouble(), scope)
                },
                valueRange = min..max,
                steps = intervals - 1,
            )
        }
        "ProgressBar" -> {
            val progress = (store.evaluatePresented(fields.opt("progress"), locals, scope) as? Number)?.toFloat() ?: 0f
            LinearProgressIndicator(progress = { progress.coerceIn(0f, 1f) })
        }
        "ProgressRing" -> {
            val progress = (store.evaluatePresented(fields.opt("progress"), locals, scope) as? Number)?.toFloat() ?: 0f
            CircularProgressIndicator(progress = { progress.coerceIn(0f, 1f) })
        }
        "SegmentedControl" -> {
            val state = fields.optString("state")
            val selected = store.state(state, scope) as? String ?: ""
            val options = (store.evaluatePresented(fields.opt("items"), locals, scope) as? List<*>)
                ?.filterIsInstance<String>()
                ?: emptyList()
            SingleChoiceSegmentedButtonRow {
                options.forEachIndexed { index, item ->
                    SegmentedButton(
                        selected = selected == item,
                        onClick = { store.setState(state, item, scope) },
                        shape = SegmentedButtonDefaults.itemShape(index = index, count = options.size),
                    ) {
                        Text(item)
                    }
                }
            }
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
            val expanded = remember(state, scope) { mutableStateOf(false) }
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (label != null) {
                    Text(label)
                    Spacer(Modifier.weight(1f))
                }
                Box {
                    androidx.compose.material3.TextButton(onClick = { expanded.value = true }) {
                        val iconSelection = fields.optJSONObject("icon") ?: JSONObject()
                        if (iconSelection.has("shared") || iconSelection.has("sf_symbol")) {
                            val iconName = if (iconSelection.has("shared")) {
                                iconSelection.optString("shared")
                            } else {
                                iconSelection.optString("sf_symbol")
                            }
                            val icon = if (iconSelection.has("shared")) {
                                nexaDevSharedMaterialIcon(iconName)
                            } else {
                                nexaDevSfAliasMaterialIcon(iconName) ?: Icons.Filled.Star
                            }
                            Icon(imageVector = icon, contentDescription = selected)
                        } else {
                            Text(selected)
                        }
                    }
                    DropdownMenu(
                        expanded = expanded.value,
                        onDismissRequest = { expanded.value = false },
                    ) {
                        options.forEach { item ->
                            DropdownMenuItem(
                                text = { Text(item) },
                                onClick = {
                                    store.setState(state, item, scope)
                                    expanded.value = false
                                },
                            )
                        }
                    }
                }
            }
        }
        "DatePicker" -> {
            val timestampName = fields.optString("timestamp_state")
            val hasTimeName = fields.optString("has_time_state")
            var timestamp by remember(timestampName, scope) {
                mutableLongStateOf((store.state(timestampName, scope) as? Number)?.toLong() ?: 0L)
            }
            var hasTime by remember(hasTimeName, scope) {
                mutableStateOf(store.state(hasTimeName, scope) as? Boolean ?: false)
            }
            val datePickerState = rememberDatePickerState(initialSelectedDateMillis = timestamp)
            val timePickerState = rememberTimePickerState()
            LaunchedEffect(timestamp, hasTime) {
                store.setState(timestampName, timestamp, scope)
                store.setState(hasTimeName, hasTime, scope)
            }
            Column {
                Switch(checked = hasTime, onCheckedChange = { hasTime = it })
                DatePicker(state = datePickerState, title = { Text("Select Date") })
                if (hasTime) TimePicker(state = timePickerState)
                LaunchedEffect(datePickerState.selectedDateMillis, timePickerState.hour, timePickerState.minute, hasTime) {
                    datePickerState.selectedDateMillis?.let { selectedMillis ->
                        val calendar = java.util.Calendar.getInstance().apply {
                            timeInMillis = selectedMillis
                            set(java.util.Calendar.HOUR_OF_DAY, if (hasTime) timePickerState.hour else 0)
                            set(java.util.Calendar.MINUTE, if (hasTime) timePickerState.minute else 0)
                            set(java.util.Calendar.SECOND, 0)
                            set(java.util.Calendar.MILLISECOND, 0)
                        }
                        timestamp = calendar.timeInMillis
                    }
                }
            }
        }
        "Image" -> {
            val source = fields.optJSONObject("source") ?: JSONObject()
            val description = fields.optString("description").takeIf(String::isNotEmpty)
            val scale = if (fields.optString("scale") == "Fill") ContentScale.Crop else ContentScale.Fit
            val placeholder = fields.optString("placeholder").takeIf(String::isNotEmpty)
            val sharedElementExpression = fields.opt("shared_element")
            val renderedModifier = if (sharedElementExpression != null && sharedElementExpression != JSONObject.NULL) {
                modifier.then(nexaSharedElementModifier(store.evaluatePresented(sharedElementExpression, locals, scope) ?: ""))
            } else {
                modifier
            }
            val imageModifier = if (fields.has("max_height")) renderedModifier.heightIn(max = fields.optDouble("max_height").dp) else renderedModifier
            val asset = source.optString("Asset").takeIf(String::isNotEmpty)
            val remoteExpression = source.opt("RemoteUrl")
            val localFileExpression = source.opt("LocalFile")
            if (asset != null) {
                val context = LocalContext.current
                val resourceId = remember(asset) { context.resources.getIdentifier(asset, "drawable", context.packageName) }
                if (resourceId != 0) {
                    Image(painterResource(resourceId), contentDescription = description, contentScale = scale, modifier = imageModifier)
                } else {
                    Text(placeholder ?: "Image unavailable", modifier = imageModifier)
                }
            } else if (remoteExpression != null && remoteExpression != JSONObject.NULL) {
                val url = store.stringify(store.evaluatePresented(remoteExpression, locals, scope))
                val context = LocalContext.current
                val placeholderId = remember(placeholder) {
                    placeholder?.let { context.resources.getIdentifier(it, "drawable", context.packageName) } ?: 0
                }
                val placeholderPainter = if (placeholderId != 0) painterResource(placeholderId) else null
                AsyncImage(
                    model = url.takeIf { it.startsWith("https://") },
                    imageLoader = nexaImageLoader(),
                    contentDescription = description,
                    contentScale = scale,
                    placeholder = placeholderPainter,
                    error = placeholderPainter,
                    modifier = imageModifier,
                )
            } else if (localFileExpression != null && localFileExpression != JSONObject.NULL) {
                val fileUri = store.stringify(store.evaluatePresented(localFileExpression, locals, scope))
                    .takeIf { it.startsWith("file://") }
                AsyncImage(
                    model = fileUri,
                    imageLoader = nexaImageLoader(),
                    contentDescription = description,
                    contentScale = scale,
                    modifier = imageModifier,
                )
            } else {
                Text(placeholder ?: "Image unavailable", modifier = imageModifier)
            }
        }
        "RefreshControl" -> {
            val state = fields.optString("state")
            val actions = fields.optJSONArray("actions") ?: JSONArray()
            val children = fields.optJSONArray("children") ?: JSONArray()
            PullToRefreshBox(
                isRefreshing = store.state(state, scope) as? Boolean ?: false,
                onRefresh = { store.perform(actions, scope, locals) },
                modifier = modifier.fillMaxWidth(),
            ) {
                Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                    RenderChildren(children, module, store, locals, scope)
                }
            }
        }
        "If" -> {
            val condition = store.evaluatePresented(fields.opt("condition"), locals, scope) as? Boolean ?: false
            val transition = fields.optString("transition")
            if (transition.isEmpty()) {
                RenderChildren(fields.optJSONArray(if (condition) "then_body" else "else_body") ?: JSONArray(), module, store, locals, scope)
            } else {
                AnimatedContent(
                    targetState = condition,
                    transitionSpec = {
                        when (transition) {
                            "Fade" -> fadeIn() togetherWith fadeOut()
                            "SlideFromBottom" -> slideInVertically { height -> height } togetherWith slideOutVertically { height -> height }
                            "SlideFromLeft" -> slideInHorizontally { width -> -width } togetherWith slideOutHorizontally { width -> width }
                            "SlideFromRight" -> slideInHorizontally { width -> width } togetherWith slideOutHorizontally { width -> -width }
                            "Scale" -> scaleIn() togetherWith scaleOut()
                            else -> fadeIn() togetherWith fadeOut()
                        }
                    },
                    label = "dev-if-transition",
                ) { target ->
                    RenderChildren(fields.optJSONArray(if (target) "then_body" else "else_body") ?: JSONArray(), module, store, locals, scope)
                }
            }
        }
        "When" -> {
            val value = store.stringify(store.evaluatePresented(fields.opt("value"), locals, scope))
            val transition = fields.optString("transition")
            if (transition.isEmpty()) {
                renderWhenTarget(value, fields, module, store, locals, scope)
            } else {
                AnimatedContent(
                    targetState = value,
                    transitionSpec = {
                        when (transition) {
                            "Fade" -> fadeIn() togetherWith fadeOut()
                            "SlideFromBottom" -> slideInVertically { height -> height } togetherWith slideOutVertically { height -> height }
                            "SlideFromLeft" -> slideInHorizontally { width -> -width } togetherWith slideOutHorizontally { width -> width }
                            "SlideFromRight" -> slideInHorizontally { width -> width } togetherWith slideOutHorizontally { width -> -width }
                            "Scale" -> scaleIn() togetherWith scaleOut()
                            else -> fadeIn() togetherWith fadeOut()
                        }
                    },
                    label = "dev-when-transition",
                ) { target ->
                    renderWhenTarget(target, fields, module, store, locals, scope)
                }
            }
        }
        "Link" -> {
            val context = LocalContext.current
            val url = store.stringify(store.evaluatePresented(fields.opt("url"), locals, scope))
            val children = fields.optJSONArray("children") ?: JSONArray()
            Column(Modifier.clickable(interactionSource = null, indication = null) { openNexaUrl(context, url) }) {
                RenderChildren(children, module, store, locals, scope)
            }
        }
        "Accessibility" -> {
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            val hintValue = fields.opt("hint")
            val hint = if (hintValue == null || hintValue == JSONObject.NULL) null else store.stringify(store.evaluatePresented(hintValue, locals, scope))
            val accessibilityValue = fields.opt("value")
            val value = if (accessibilityValue == null || accessibilityValue == JSONObject.NULL) null else store.stringify(store.evaluatePresented(accessibilityValue, locals, scope))
            val role = when (fields.optString("role")) {
                "Button" -> SemanticsRole.Button
                "Image" -> SemanticsRole.Image
                else -> null
            }
            Column(Modifier.semantics(mergeDescendants = true) {
                contentDescription = if (hint != null) "$label, $hint" else label
                if (value != null) stateDescription = value
                if (role != null) this.role = role
                if (fields.optString("role") == "Header") heading()
            }) {
                RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
            }
        }
        "KeyboardAware" -> {
            val baseModifier = Modifier.imePadding()
            val keyboardModifier = if (fields.optString("dismiss") == "Interactive") {
                baseModifier.imeNestedScroll()
            } else {
                baseModifier
            }
            Column(
                keyboardModifier.verticalScroll(rememberScrollState()),
            ) {
                RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
            }
        }
        "AppBottomBar" -> {
            val state = fields.optString("state")
            val tabs = fields.optJSONArray("tabs") ?: JSONArray()
            val tintValue = fields.optJSONObject("tint") ?: JSONObject()
            val dynamicTint = tintValue.opt("Dynamic")
            val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope) ?: ""))
            } else {
                nexaDevColor(tintValue, isSystemInDarkTheme())
            }
            val selected = (store.state(state, scope) as? Number)?.toInt() ?: 0
            val tabStateHolder = rememberSaveableStateHolder()
            val tabItemColors = tint?.let {
                NavigationSuiteDefaults.itemColors(
                    navigationBarItemColors = androidx.compose.material3.NavigationBarItemDefaults.colors(
                        selectedIconColor = it,
                        selectedTextColor = it,
                        indicatorColor = it.copy(alpha = 0.12f),
                    ),
                    navigationRailItemColors = androidx.compose.material3.NavigationRailItemDefaults.colors(
                        selectedIconColor = it,
                        selectedTextColor = it,
                        indicatorColor = it.copy(alpha = 0.12f),
                    ),
                    navigationDrawerItemColors = androidx.compose.material3.NavigationDrawerItemDefaults.colors(
                        selectedIconColor = it,
                        selectedTextColor = it,
                        selectedContainerColor = it.copy(alpha = 0.12f),
                    ),
                )
            }
            NavigationSuiteScaffold(
                navigationSuiteItems = {
                    for (index in 0 until tabs.length()) {
                        val tab = tabs.optJSONObject(index) ?: continue
                        val tabIndex = tab.optInt("index", index)
                        val label = tab.optString("label")
                        val iconSelection = tab.optJSONObject("icon") ?: JSONObject()
                        val badge = tab.optString("badge").takeIf(String::isNotEmpty)
                        val tabIcon = when {
                            iconSelection.has("shared") -> nexaDevSharedMaterialIcon(iconSelection.optString("shared"))
                            iconSelection.has("sf_symbol") -> nexaDevSfAliasMaterialIcon(iconSelection.optString("sf_symbol"))
                            iconSelection.has("material_symbol") -> nexaDevSpecificMaterialIcon(iconSelection.optString("material_symbol"))
                            else -> null
                        }
                        item(
                            selected = selected == tabIndex,
                            onClick = { store.setState(state, tabIndex, scope) },
                            icon = {
                                if (tabIcon != null) Icon(imageVector = tabIcon, contentDescription = null)
                                else Spacer(Modifier.size(24.dp))
                            },
                            label = { Text(label) },
                            alwaysShowLabel = true,
                            badge = if (badge == null) null else ({ Badge { Text(badge) } }),
                            colors = tabItemColors,
                        )
                    }
                },
            ) {
                val activeTab = (0 until tabs.length())
                    .firstNotNullOfOrNull { index -> tabs.optJSONObject(index)?.let { tab ->
                        tab.takeIf { it.optInt("index", index) == selected }
                    } }
                if (activeTab != null) {
                    val activeIndex = activeTab.optInt("index", 0)
                    tabStateHolder.SaveableStateProvider(key = "nexa-tab-$activeIndex") {
                        RenderChildren(activeTab.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
                    }
                }
            }
        }
        "PagePager" -> {
            val state = fields.optString("state")
            val pages = fields.optJSONArray("pages") ?: JSONArray()
            val selected = (store.state(state, scope) as? Number)?.toInt() ?: 0
            val pageState = rememberPagerState(initialPage = selected) { pages.length() }
            val pageCoroutineScope = rememberCoroutineScope()
            LaunchedEffect(pageState.currentPage) { store.setState(state, pageState.currentPage, scope) }
            LaunchedEffect(selected) {
                if (selected in 0 until pages.length() && selected != pageState.currentPage) {
                    pageState.animateScrollToPage(selected)
                }
            }
            Column(modifier = Modifier.fillMaxSize()) {
                HorizontalPager(state = pageState, modifier = Modifier.weight(1f)) { page ->
                    val children = pages.optJSONArray(page) ?: JSONArray()
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center,
                        propagateMinConstraints = true,
                    ) {
                        RenderChildren(children, module, store, locals, scope)
                    }
                }
                Row(
                    modifier = Modifier.fillMaxWidth()
                        .padding(bottom = nexaDevPageIndicatorBottomInset.dp),
                    horizontalArrangement = Arrangement.spacedBy(
                        nexaDevPageIndicatorSpacing.dp,
                        Alignment.CenterHorizontally,
                    ),
                ) {
                    for (page in 0 until pages.length()) {
                        Box(
                            modifier = Modifier
                                .size(
                                    if (pageState.currentPage == page) nexaDevPageIndicatorSelectedSize.dp
                                    else nexaDevPageIndicatorUnselectedSize.dp,
                                )
                                .clip(CircleShape)
                                .background(
                                    if (pageState.currentPage == page) MaterialTheme.colorScheme.primary
                                    else Color(__NEXA_MUTED_TEXT_ARGB__).copy(
                                        alpha = nexaDevPageIndicatorInactiveOpacity,
                                    ),
                                )
                                .clickable(
                                    role = SemanticsRole.Button,
                                    onClickLabel = "Page ${page + 1}",
                                ) {
                                    pageCoroutineScope.launch { pageState.animateScrollToPage(page) }
                                },
                        )
                    }
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
            val state = fields.optString("state")
            val isPresented = store.state(state, scope) as? Boolean ?: false
            if (isPresented) {
                val children = fields.optJSONArray("children") ?: JSONArray()
                if (fields.optBoolean("partial", true)) {
                    ModalBottomSheet(
                        onDismissRequest = { store.setState(state, false, scope) },
                        sheetState = rememberModalBottomSheetState(
                            skipPartiallyExpanded = fields.optBoolean("large_only", false),
                        ),
                    ) {
                        RenderChildren(children, module, store, locals, scope)
                    }
                } else {
                    Dialog(
                        onDismissRequest = { store.setState(state, false, scope) },
                        properties = DialogProperties(usePlatformDefaultWidth = false),
                    ) {
                        Surface(modifier = Modifier.fillMaxSize(), shape = RectangleShape) {
                            RenderChildren(children, module, store, locals, scope)
                        }
                    }
                }
            }
        }
        "Dialog" -> {
            val state = fields.optString("state")
            if (store.state(state, scope) as? Boolean == true) {
                val title = store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope))
                val message = store.stringify(store.evaluatePresented(fields.opt("message"), locals, scope))
                AlertDialog(
                    onDismissRequest = { store.setState(state, false, scope) },
                    title = { Text(title) },
                    text = { Text(message) },
                    confirmButton = {
                        RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
                    },
                )
            }
        }
        "ConfirmationDialog" -> {
            val state = fields.optString("state")
            if (store.state(state, scope) as? Boolean == true) {
                val title = store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope))
                AlertDialog(
                    onDismissRequest = { store.setState(state, false, scope) },
                    title = { Text(title) },
                    confirmButton = {
                        Column {
                            RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
                        }
                    },
                )
            }
        }
        "NavigationStack" -> RenderNavigationStack(fields, module, store, locals, scope)
        "NavigationSplitView" -> {
            val sidebar = fields.optJSONArray("sidebar") ?: JSONArray()
            val detail = fields.optJSONArray("detail") ?: JSONArray()
            val state = fields.optString("detail_visible")
            BoxWithConstraints(Modifier.fillMaxSize()) {
                if (maxWidth >= 840.dp) {
                    Row(Modifier.fillMaxSize()) {
                        Box(Modifier.widthIn(min = 260.dp, max = 360.dp).fillMaxHeight()) {
                            RenderChildren(sidebar, module, store, locals, scope)
                        }
                        Box(Modifier.weight(1f).fillMaxHeight()) {
                            RenderChildren(detail, module, store, locals, scope)
                        }
                    }
                } else {
                    val showDetail = store.state(state, scope) as? Boolean == true
                    BackHandler(enabled = showDetail) { store.setState(state, false, scope) }
                    if (showDetail) {
                        RenderChildren(detail, module, store, locals, scope)
                    } else {
                        RenderChildren(sidebar, module, store, locals, scope)
                    }
                }
            }
        }
        "NavigationLink" -> {
            val screens = module.optJSONArray("screens") ?: JSONArray()
            val destination = screens.optJSONObject(fields.optInt("destination", -1))
            val children = fields.optJSONArray("children") ?: JSONArray()
            val guard = fields.opt("guard")
            val enabled = guard == null || guard == JSONObject.NULL || store.evaluatePresented(guard, locals, scope) as? Boolean == true
            if (destination == null || !enabled) {
                RenderChildren(children, module, store, locals, scope)
            } else {
                val controller = LocalNexaDevNavController.current
                val route = store.encodeScreenRoute(destination, fields.optJSONArray("arguments") ?: JSONArray(), locals, scope)
                androidx.compose.material3.TextButton(onClick = { controller?.navigate(route) }) {
                    RenderChildren(children, module, store, locals, scope)
                }
            }
        }
        "NavigationBack" -> {
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            val controller = LocalNexaDevNavController.current
            androidx.compose.material3.TextButton(onClick = { controller?.popBackStack() }) {
                Text(label)
            }
        }
    }
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
    RenderChildren(
        matching?.optJSONArray("body") ?: fields.optJSONArray("else_body") ?: JSONArray(),
        module,
        store,
        locals,
        scope,
    )
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
        androidx.compose.runtime.key(index) {
            val child = nexaDevNodeObject(nodes.opt(index))
            val modifier = if (child.has("Spacer")) Modifier.weight(1f) else Modifier
            NexaDevNode(child, module, store, locals, scope, modifier)
        }
    }
}
