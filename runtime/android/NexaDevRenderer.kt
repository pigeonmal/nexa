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
import androidx.compose.material3.Button
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.TextButton
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
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
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.input.pointer.util.addPointerInputChange
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
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
private const val nexaDevDefaultMaterialIconSize = __NEXA_DEFAULT_MATERIAL_ICON_SIZE__
private const val nexaDevMaterialIconSizeScale = __NEXA_MATERIAL_ICON_SIZE_SCALE__
private const val nexaDevDefaultLineHeightMultiplier = __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f
private const val nexaDevFormRowHorizontalInset = __NEXA_FORM_ROW_HORIZONTAL_INSET__
private const val nexaDevFormRowMinHeight = __NEXA_FORM_ROW_MIN_HEIGHT__
private const val nexaDevFormLargeTitleTopPadding = __NEXA_FORM_LARGE_TITLE_TOP_PADDING__
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

private fun JSONObject.nexaDevTransition(): String = when (val value = opt("transition")) {
    null, JSONObject.NULL -> ""
    else -> value.toString()
}

private fun JSONObject.nexaDevOptionalString(name: String): String? = when (val value = opt(name)) {
    null, JSONObject.NULL -> null
    is String -> value
    else -> value.toString()
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
                val modifier = if (child.has("Spacer")) Modifier.weight(1f) else Modifier
                NexaDevNode(child, module, store, locals, scope, modifier)
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
) {
    val kind = node.keys().asSequence().firstOrNull() ?: return
    val fields = node.optJSONObject(kind) ?: JSONObject()
    when (kind) {
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
            NexaFormPrimitive {
                val children = fields.optJSONArray("children") ?: JSONArray()
                for (index in 0 until children.length()) {
                    androidx.compose.runtime.key(index) {
                        val child = nexaDevNodeObject(children.opt(index))
                        if (child.has("FormSection")) {
                            NexaDevNode(child, module, store, locals, scope)
                        } else {
                            NexaFormRowPrimitive(
                                minHeight = nexaDevFormRowMinHeight.toFloat(),
                                horizontalInset = nexaDevFormRowHorizontalInset.toFloat(),
                            ) {
                                NexaDevNode(child, module, store, locals, scope)
                            }
                        }
                    }
                }
            }
        }
        "FormSection" -> {
            NexaFormSectionPrimitive(
                title = if (fields.isNull("title")) null else store.stringify(store.evaluatePresented(fields.opt("title"), locals, scope)),
                footer = if (fields.isNull("footer")) null else store.stringify(store.evaluatePresented(fields.opt("footer"), locals, scope)),
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
                            )
                        }
                        if (index + 1 < children.length()) {
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
                "Row" -> NexaRowPrimitive(
                    modifier = modifier,
                    spacing = spacing.value,
                    verticalAlignment = when (style.optString("alignment")) {
                        "Start" -> Alignment.Top
                        "End" -> Alignment.Bottom
                        else -> Alignment.CenterVertically
                    },
                ) { RenderRowChildren(children, module, store, locals, scope) }
                "Stack" -> NexaStackPrimitive(
                    modifier = modifier,
                    contentAlignment = when (style.optString("alignment")) {
                        "Start" -> Alignment.TopStart
                        "End" -> Alignment.BottomEnd
                        else -> Alignment.Center
                    },
                ) { RenderChildren(children, module, store, locals, scope) }
                else -> NexaColumnPrimitive(
                    modifier = modifier,
                    spacing = spacing.value,
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
            NexaContentUnavailablePrimitive(title = title, description = description, icon = image)
        }
        "Text", "Spacer", "Divider", "SystemIcon", "LinearGradient" ->
            NexaDevRenderTextNode(kind, fields, module, store, locals, scope, modifier)
        "Button", "Pressable", "TextInput", "FastList", "Switch", "Slider",
        "ProgressBar", "ProgressRing", "SegmentedControl", "Picker", "DatePicker" ->
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
                NexaTextPrimitive(
                    text = text,
                    modifier = modifier,
                    color = color ?: Color.Unspecified,
                    fontSize = fontSize,
                    fontWeight = fontWeight,
                    textAlign = textAlign,
                    maxLines = maxLines,
                    lineHeight = lineHeight,
                    letterSpacing = letterSpacing,
                    strikethrough = style.optBoolean("strikethrough"),
                    selectable = style.optBoolean("selectable"),
                )
            }
            content()
        }
        "Spacer" -> Spacer(modifier = modifier)
        "Divider" -> NexaDividerPrimitive(
            color = nexaDevColor(fields.optJSONObject("color"), isSystemInDarkTheme()) ?: Color.Gray,
            thickness = fields.optDouble("thickness", 1.0).dp,
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
            NexaSystemIconPrimitive(
                image = icon,
                description = fields.optString("description"),
                size = (fields.optDouble("size", 24.0) * nexaDevMaterialIconSizeScale).dp,
                tint = tint ?: Color.White,
                modifier = modifier,
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
            val style = fields.optString("style").ifEmpty { "BorderedProminent" }
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
            val iconSelection = fields.optJSONObject("icon") ?: JSONObject()
            val icon = when {
                iconSelection.has("shared") -> nexaDevSharedMaterialIcon(iconSelection.optString("shared"))
                iconSelection.has("sf_symbol") -> nexaDevSfAliasMaterialIcon(iconSelection.optString("sf_symbol"))
                iconSelection.has("material_symbol") -> nexaDevSpecificMaterialIcon(iconSelection.optString("material_symbol"))
                else -> null
            }
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            NexaButtonPrimitive(
                label = label,
                icon = icon,
                style = style,
                size = fields.nexaDevOptionalString("size")?.takeIf(String::isNotEmpty),
                shape = buttonShape,
                tint = tint,
                loading = loading,
                disabled = disabled,
                iconOnlyCircle = icon != null && buttonShape == CircleShape && label.isEmpty(),
                fullWidth = fields.optBoolean("__nexa_full_width", false),
                onClick = { store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals) },
            )
        }
        "Pressable" -> NexaDevRenderPressableNode(fields, module, store, locals, scope, modifier)
        "TextInput" -> NexaDevRenderTextInputNode(fields, module, store, locals, scope, modifier)
        "FastList" -> NexaDevRenderFastListNode(fields, module, store, locals, scope, modifier)
        "Switch" -> {
            val state = fields.optString("state")
            val label = store.stringify(store.evaluatePresented(fields.opt("label"), locals, scope))
            NexaSwitchPrimitive(
                label = label,
                checked = store.state(state, scope) as? Boolean ?: false,
                onCheckedChange = { store.setState(state, it, scope) },
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
            NexaSliderPrimitive(
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
            NexaProgressBarPrimitive(progress)
        }
        "ProgressRing" -> {
            val progress = (store.evaluatePresented(fields.opt("progress"), locals, scope) as? Number)?.toFloat() ?: 0f
            NexaProgressRingPrimitive(progress)
        }
        "SegmentedControl" -> {
            val state = fields.optString("state")
            val selected = store.state(state, scope) as? String ?: ""
            val options = (store.evaluatePresented(fields.opt("items"), locals, scope) as? List<*>)
                ?.filterIsInstance<String>()
                ?: emptyList()
            NexaSegmentedControlPrimitive(
                items = options,
                selected = selected,
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
                nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope) ?: ""))
            } else {
                nexaDevColor(tintValue, isSystemInDarkTheme())
            }
            val iconSelection = fields.optJSONObject("icon") ?: JSONObject()
            val pickerIcon = when {
                iconSelection.has("shared") -> nexaDevSharedMaterialIcon(iconSelection.optString("shared"))
                iconSelection.has("sf_symbol") -> nexaDevSfAliasMaterialIcon(iconSelection.optString("sf_symbol"))
                else -> null
            }
            NexaPickerPrimitive(
                items = options,
                selected = selected,
                icon = pickerIcon,
                label = label,
                tint = tint,
                identity = "$scope::$state",
                onSelectionChanged = { store.setState(state, it, scope) },
            )
        }
        "DatePicker" -> {
            val timestampName = fields.optString("timestamp_state")
            val hasTimeName = fields.optString("has_time_state")
            NexaDatePickerPrimitive(
                timestamp = (store.state(timestampName, scope) as? Number)?.toLong() ?: 0L,
                includesTime = store.state(hasTimeName, scope) as? Boolean ?: false,
                identity = "$scope::$timestampName",
                onTimestampChanged = { store.setState(timestampName, it, scope) },
                onIncludesTimeChanged = { store.setState(hasTimeName, it, scope) },
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
                modifier.then(nexaSharedElementModifier(store.evaluatePresented(sharedElementExpression, locals, scope) ?: ""))
            } else {
                modifier
            }
            val maxHeight = if (fields.has("max_height")) fields.optDouble("max_height").dp else null
            val asset = source.optString("Asset").takeIf(String::isNotEmpty)
            val remoteExpression = source.opt("RemoteUrl")
            val localFileExpression = source.opt("LocalFile")
            if (asset != null) {
                NexaAssetImagePrimitive(
                    name = asset,
                    contentDescription = description,
                    contentScale = scale,
                    maxHeight = maxHeight,
                    modifier = renderedModifier,
                )
            } else if (remoteExpression != null && remoteExpression != JSONObject.NULL) {
                val url = store.stringify(store.evaluatePresented(remoteExpression, locals, scope))
                NexaRemoteImagePrimitive(
                    url = url,
                    allowsFile = false,
                    placeholder = placeholder,
                    contentDescription = description,
                    contentScale = scale,
                    maxHeight = maxHeight,
                    modifier = renderedModifier,
                )
            } else if (localFileExpression != null && localFileExpression != JSONObject.NULL) {
                val fileUrl = store.stringify(store.evaluatePresented(localFileExpression, locals, scope))
                NexaRemoteImagePrimitive(
                    url = fileUrl,
                    allowsFile = true,
                    placeholder = null,
                    contentDescription = description,
                    contentScale = scale,
                    maxHeight = maxHeight,
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
            val transition = fields.nexaDevTransition()
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
            val transition = fields.nexaDevTransition()
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
        "AppBottomBar" -> NexaDevRenderAppBottomBarNode(fields, module, store, locals, scope, modifier)
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
                NexaPageIndicatorRow(
                    pageCount = pages.length(),
                    currentPage = pageState.currentPage,
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
            val state = fields.optString("state")
            val isPresented = store.state(state, scope) as? Boolean ?: false
            val children = fields.optJSONArray("children") ?: JSONArray()
            val partial = fields.optBoolean("partial", true)
            val rawTitle = fields.opt("title")
            val title = if (partial && rawTitle != null && rawTitle != JSONObject.NULL) {
                store.stringify(store.evaluatePresented(rawTitle, locals, scope))
            } else {
                null
            }
            val hasTitle = title != null
            val leading = if (hasTitle) nexaDevSheetToolbarNodes(children, "Leading") else JSONArray()
            val trailing = if (hasTitle) nexaDevSheetToolbarNodes(children, "Trailing") else JSONArray()
            val content = if (hasTitle) nexaDevWithoutSheetToolbars(children) else children
            NexaBottomSheetPrimitive(
                isPresented = isPresented,
                partial = partial,
                largeOnly = fields.optBoolean("large_only", false),
                title = title,
                onDismissRequest = { store.setState(state, false, scope) },
                navigationIcon = { RenderChildren(leading, module, store, locals, scope) },
                actions = { RenderChildren(trailing, module, store, locals, scope) },
                content = { RenderChildren(content, module, store, locals, scope) },
            )
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
                    confirmButton = {
                        RenderChildren(actions, module, store, locals, scope)
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
        "NavigationStack" -> RenderNavigationStack(fields, module, store, locals, scope)
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
                val controller = LocalNexaDevNavController.current
                val route = store.encodeScreenRoute(destination, fields.optJSONArray("arguments") ?: JSONArray(), locals, scope)
                NexaNavigationLinkPrimitive(
                    enabled = enabled,
                    onClick = { controller?.navigate(route) },
                    trailing = {
                        Icon(
                            imageVector = Icons.Filled.ChevronRight,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    },
                ) {
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

private fun nexaDevContainsForm(nodes: JSONArray): Boolean {
    for (index in 0 until nodes.length()) {
        val node = nodes.optJSONObject(index) ?: continue
        val kind = node.keys().asSequence().firstOrNull() ?: continue
        if (kind == "Form") return true
        val children = node.optJSONObject(kind)?.optJSONArray("children") ?: continue
        if (nexaDevContainsForm(children)) return true
    }
    return false
}

private fun nexaDevTabToolbars(nodes: JSONArray): List<Pair<String, JSONArray>> {
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

private fun nexaDevWithoutTabToolbars(nodes: JSONArray): JSONArray {
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

private fun nexaDevSheetToolbarNodes(nodes: JSONArray, placement: String): JSONArray {
    val result = JSONArray()
    for (index in 0 until nodes.length()) {
        val node = nodes.optJSONObject(index) ?: continue
        val kind = node.keys().asSequence().firstOrNull() ?: continue
        val fields = node.optJSONObject(kind) ?: continue
        when (kind) {
            "Toolbar" -> {
                if (fields.optString("placement") == placement) {
                    val children = fields.optJSONArray("children") ?: JSONArray()
                    for (childIndex in 0 until children.length()) result.put(children.opt(childIndex))
                }
            }
            "Layout", "KeyboardAware" -> {
                val nested = nexaDevSheetToolbarNodes(
                    fields.optJSONArray("children") ?: JSONArray(),
                    placement,
                )
                for (nestedIndex in 0 until nested.length()) result.put(nested.opt(nestedIndex))
            }
            "If" -> {
                val thenBody = nexaDevSheetToolbarNodes(
                    fields.optJSONArray("then_body") ?: JSONArray(),
                    placement,
                )
                val elseBody = nexaDevSheetToolbarNodes(
                    fields.optJSONArray("else_body") ?: JSONArray(),
                    placement,
                )
                if (thenBody.length() > 0 || elseBody.length() > 0) {
                    val filtered = JSONObject(fields.toString())
                        .put("then_body", thenBody)
                        .put("else_body", elseBody)
                    result.put(JSONObject().put("If", filtered))
                }
            }
            "When" -> {
                val cases = fields.optJSONArray("cases") ?: JSONArray()
                val filteredCases = JSONArray()
                var hasToolbar = false
                for (caseIndex in 0 until cases.length()) {
                    val case = cases.optJSONObject(caseIndex) ?: continue
                    val body = nexaDevSheetToolbarNodes(
                        case.optJSONArray("body") ?: JSONArray(),
                        placement,
                    )
                    hasToolbar = hasToolbar || body.length() > 0
                    filteredCases.put(JSONObject(case.toString()).put("body", body))
                }
                val elseBody = nexaDevSheetToolbarNodes(
                    fields.optJSONArray("else_body") ?: JSONArray(),
                    placement,
                )
                if (hasToolbar || elseBody.length() > 0) {
                    val filtered = JSONObject(fields.toString())
                        .put("cases", filteredCases)
                        .put("else_body", elseBody)
                    result.put(JSONObject().put("When", filtered))
                }
            }
        }
    }
    return result
}

private fun nexaDevWithoutSheetToolbars(nodes: JSONArray): JSONArray {
    val result = JSONArray()
    for (index in 0 until nodes.length()) {
        val node = nodes.optJSONObject(index) ?: continue
        val kind = node.keys().asSequence().firstOrNull() ?: continue
        val fields = node.optJSONObject(kind) ?: continue
        when (kind) {
            "Toolbar" -> Unit
            "Layout", "KeyboardAware" -> {
                val children = fields.optJSONArray("children") ?: JSONArray()
                val filteredChildren = nexaDevWithoutSheetToolbars(children)
                if (filteredChildren.length() == 0 && children.length() > 0) continue
                result.put(
                    JSONObject().put(
                        kind,
                        JSONObject(fields.toString()).put("children", filteredChildren),
                    ),
                )
            }
            "If" -> {
                val thenBody = fields.optJSONArray("then_body") ?: JSONArray()
                val elseBody = fields.optJSONArray("else_body") ?: JSONArray()
                val filteredThen = nexaDevWithoutSheetToolbars(thenBody)
                val filteredElse = nexaDevWithoutSheetToolbars(elseBody)
                if (filteredThen.length() == 0 && filteredElse.length() == 0 &&
                    (thenBody.length() > 0 || elseBody.length() > 0)
                ) continue
                val filtered = JSONObject(fields.toString())
                    .put("then_body", filteredThen)
                    .put("else_body", filteredElse)
                result.put(JSONObject().put("If", filtered))
            }
            "When" -> {
                val cases = fields.optJSONArray("cases") ?: JSONArray()
                val filteredCases = JSONArray()
                var hadContent = false
                var hasContent = false
                for (caseIndex in 0 until cases.length()) {
                    val case = cases.optJSONObject(caseIndex) ?: continue
                    val body = case.optJSONArray("body") ?: JSONArray()
                    val filteredBody = nexaDevWithoutSheetToolbars(body)
                    hadContent = hadContent || body.length() > 0
                    hasContent = hasContent || filteredBody.length() > 0
                    filteredCases.put(JSONObject(case.toString()).put("body", filteredBody))
                }
                val elseBody = fields.optJSONArray("else_body") ?: JSONArray()
                val filteredElse = nexaDevWithoutSheetToolbars(elseBody)
                hadContent = hadContent || elseBody.length() > 0
                hasContent = hasContent || filteredElse.length() > 0
                if (!hasContent && hadContent) continue
                val filtered = JSONObject(fields.toString())
                    .put("cases", filteredCases)
                    .put("else_body", filteredElse)
                result.put(JSONObject().put("When", filtered))
            }
            else -> result.put(node)
        }
    }
    return result
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
private fun NexaDevRenderPressableNode(
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier,
) {
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
        val textStyle = when (fields.optString("font")) {
            "Body" -> MaterialTheme.typography.bodyLarge
            "Title3" -> MaterialTheme.typography.titleMedium
            else -> LocalTextStyle.current
        }
        val onChange = fields.optJSONObject("on_change")
        var fieldModifier: Modifier = Modifier
        if (focusedState != null && focusRequester != null) {
            fieldModifier = fieldModifier
                .focusRequester(focusRequester)
                .onFocusChanged { focusState ->
                    store.setState(focusedState, focusState.isFocused, scope)
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
                store.setState(state, updated, scope)
                val parameter = onChange?.nexaDevOptionalString("parameter")
                    ?.takeIf(String::isNotEmpty)
                if (parameter != null) {
                    val changeLocals = locals.toMutableMap()
                    changeLocals[parameter] = updated
                    store.perform(onChange.optJSONArray("actions") ?: JSONArray(), scope, changeLocals)
                }
            },
            modifier = fieldModifier.then(modifier),
            placeholder = placeholder.orEmpty(),
            searchable = fields.optBoolean("searchable"),
            searchIcon = if (fields.optBoolean("searchable")) nexaDevSharedMaterialIcon("search") else null,
            textStyle = textStyle,
            minLines = minLines,
            maxLines = maxLines,
            singleLine = !fields.optBoolean("multiline"),
            secure = fields.optBoolean("secure"),
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
        val axis = when {
            axisValue == "Horizontal" -> "Horizontal"
            gridColumns != null -> "Grid"
            else -> "Vertical"
        }
        val reverseLayout = options?.optBoolean(NexaDevKeys.REVERSE_LAYOUT, false) ?: false
        val pageSnap = options?.optBoolean(NexaDevKeys.PAGE_SNAP, false) ?: false
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
            val swipeActions = options.optJSONArray("swipe_actions")
            val stickyHeaderNodes = options.optJSONArray("sticky_header")
            val sectionHeader = options.optJSONArray("section_header")
            val onScroll = options.optJSONArray("on_scroll")
            val onEndReached = options.optJSONArray("on_end_reached")
            val onMove = options.optJSONObject("on_move")
            val moveEnabled = onMove?.opt("enabled")?.let {
                store.evaluatePresented(it, locals, scope) as? Boolean
            } ?: true
            val refresh = options.optJSONObject("refresh")
            val refreshState = refresh?.nexaDevOptionalString("state")?.takeIf(String::isNotEmpty)
            val refreshActions = refresh?.optJSONArray("actions") ?: JSONArray()
            var endReached by remember(countExpression, sourceItems?.size, sourceSections?.size) { mutableStateOf(false) }
            val scrollPositionName = options.nexaDevOptionalString("scroll_position")
                ?.takeIf(String::isNotEmpty)
            val requestedIndex = scrollPositionName?.let { name ->
                (store.state(name, scope) as? Number)?.toInt()?.coerceIn(0, maxOf(0, count - 1)) ?: 0
            } ?: if (reverseLayout) maxOf(0, count - 1) else 0
            val requestedPosition = if (reverseLayout) {
                (count - 1 - requestedIndex).coerceIn(0, maxOf(0, count - 1))
            } else requestedIndex
            val extent = options.optDouble("item_extent", 0.0).takeIf { it > 0.0 }?.dp
            val renderListRow: @Composable (JSONArray, Map<String, Any>, Modifier) -> Unit = { rowNodes, rowLocals, rowModifier ->
                val swipeButtonActions = swipeActions
                    ?.optJSONObject(0)
                    ?.optJSONObject("Button")
                    ?.optJSONArray("actions")
                val rowContent: @Composable () -> Unit = {
                    NexaFastListRowPrimitive(
                        modifier = rowModifier,
                        horizontalAlignment = if (rowNodes.length() > 1) {
                            Alignment.CenterHorizontally
                        } else {
                            Alignment.Start
                        },
                    ) {
                        RenderChildren(rowNodes, module, store, rowLocals, scope)
                    }
                }
                if (swipeActions != null) {
                    NexaFastListSwipePrimitive(
                        onEndToStart = {
                            if (swipeButtonActions != null) {
                                store.perform(swipeButtonActions, scope, rowLocals)
                            }
                        },
                        backgroundContent = {
                            RenderChildren(swipeActions, module, store, rowLocals, scope)
                        },
                        content = rowContent,
                    )
                } else {
                    rowContent()
                }
            }
            val renderItem: @Composable (Int, String, Modifier) -> Unit = { itemIndex, itemAxis, itemParentModifier ->
                val rowLocals = listRowLocals(itemIndex)
                val itemModifier = if (pageSnap && itemAxis == "Vertical") {
                    Modifier
                } else when (itemAxis) {
                    "Horizontal" -> extent?.let { Modifier.width(it).height(it) } ?: Modifier
                    "Grid" -> extent?.let { Modifier.fillMaxWidth().height(it) } ?: Modifier
                    else -> extent?.let { Modifier.fillMaxWidth().height(it) } ?: Modifier
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
                renderListRow(
                    children,
                    rowLocals,
                    itemParentModifier.then(itemModifier).then(reorderModifier),
                )
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
                        gridItems(items = itemIndices, key = { itemIndex -> stableRowKey(itemIndex) }) { itemIndex ->
                            renderItem(itemIndex, axis, Modifier)
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
                            items(count = count, key = { itemIndex -> stableRowKey(itemIndex) }) { itemIndex ->
                                renderItem(itemIndex, axis, Modifier)
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
                                    items(count = sectionItems.size, key = { itemIndex -> stableRowKey(itemIndex, sectionIndex) }) { itemIndex ->
                                        val rowLocals = listRowLocals(itemIndex, sectionIndex)
                                        val rowParentModifier = if (animateStableRows) {
                                            Modifier.fillMaxWidth().animateItem()
                                        } else {
                                            Modifier
                                        }
                                        val rowModifier = if (extent != null) {
                                            Modifier.fillMaxWidth().height(extent)
                                        } else {
                                            Modifier
                                        }
                                        renderListRow(
                                            children,
                                            rowLocals,
                                            rowParentModifier.then(rowModifier),
                                        )
                                    }
                                }
                            } else {
                                items(
                                    count = count,
                                    key = { itemPosition ->
                                        val itemIndex = if (reverseLayout) count - 1 - itemPosition else itemPosition
                                        stableRowKey(itemIndex)
                                    },
                                ) { itemPosition ->
                                    val itemIndex = if (reverseLayout) count - 1 - itemPosition else itemPosition
                                    val rowParentModifier = if (animateStableRows && !pageSnap) {
                                        Modifier.fillMaxWidth().animateItem()
                                    } else {
                                        Modifier
                                    }
                                    if (pageSnap) {
                                        renderItem(itemIndex, axis, Modifier.fillParentMaxSize())
                                    } else {
                                        renderItem(itemIndex, axis, rowParentModifier)
                                        NexaFastListDividerPrimitive()
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
        val tintValue = fields.optJSONObject("tint") ?: JSONObject()
        val dynamicTint = tintValue.opt("Dynamic")
        val tint = if (dynamicTint != null && dynamicTint != JSONObject.NULL) {
            nexaDevHexColor(store.stringify(store.evaluatePresented(dynamicTint, locals, scope) ?: ""))
        } else {
            nexaDevColor(tintValue, isSystemInDarkTheme())
        }
        val selected = (store.state(state, scope) as? Number)?.toInt() ?: 0
        val tabStateHolder = rememberSaveableStateHolder()
        NexaAppBottomBarPrimitive(
            tint = tint,
            navigationSuiteItems = { tabItemColors ->
                    for (index in 0 until tabs.length()) {
                        val tab = tabs.optJSONObject(index) ?: continue
                        val tabIndex = tab.optInt("index", index)
                        val label = tab.optString("label")
                        val iconSelection = tab.optJSONObject("icon") ?: JSONObject()
                        val badge = (tab.opt("badge") as? String)?.takeIf(String::isNotEmpty)
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
                            val contentBackground = if (nexaDevContainsForm(children)) {
                                if (MaterialTheme.colorScheme.background == Color.Black) {
                                    MaterialTheme.colorScheme.background
                                } else {
                                    MaterialTheme.colorScheme.surfaceVariant
                                }
                            } else {
                                MaterialTheme.colorScheme.background
                            }
                            val contentModifier = if (nexaDevContainsForm(children)) {
                                Modifier.fillMaxSize().background(contentBackground)
                            } else {
                                Modifier.fillMaxSize()
                            }
                            if (activeTab.optBoolean("large_title") && title != null) {
                                NexaLargeTitlePrimitive(
                                    identity = "nexa-tab-title-$activeIndex",
                                    title = title,
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
                                    if (title != null || toolbarItems.isNotEmpty()) {
                                        Row(
                                            modifier = Modifier.fillMaxWidth().padding(
                                                start = 16.dp,
                                                top = 8.dp,
                                                end = 16.dp,
                                                bottom = 8.dp,
                                            ),
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            RenderTabToolbarItems(
                                                toolbarItems,
                                                "Leading",
                                                module,
                                                store,
                                                locals,
                                                scope,
                                            )
                                            if (title != null) {
                                                if (toolbarItems.any { it.first == "Leading" }) {
                                                    Spacer(Modifier.width(8.dp))
                                                }
                                                Text(title, style = MaterialTheme.typography.titleLarge)
                                            }
                                            if (title != null || toolbarItems.any { it.first == "Leading" }) {
                                                Spacer(Modifier.weight(1f))
                                            }
                                            RenderTabToolbarItems(
                                                toolbarItems,
                                                "Trailing",
                                                module,
                                                store,
                                                locals,
                                                scope,
                                            )
                                        }
                                    }
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

}

@Composable
private fun RenderTabToolbarItems(
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
            if (kind != "NavigationLink") {
                RenderChildren(JSONArray().put(node), module, store, locals, scope)
                continue
            }

            val fields = node.optJSONObject(kind) ?: continue
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

            val controller = LocalNexaDevNavController.current
            val route = store.encodeScreenRoute(
                destination,
                fields.optJSONArray("arguments") ?: JSONArray(),
                locals,
                scope,
            )
            IconButton(onClick = { controller?.navigate(route) }, enabled = enabled) {
                RenderChildren(linkChildren, module, store, locals, scope)
            }
        }
    }
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
