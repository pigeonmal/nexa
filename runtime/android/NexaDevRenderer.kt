package __NEXA_PACKAGE__

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role as SemanticsRole
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import kotlinx.coroutines.flow.distinctUntilChanged
import org.json.JSONArray
import org.json.JSONObject

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
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(0.dp)) {
        RenderColumnChildren(nodes, module, store, locals, scope)
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
            val modifier = if (child.has("FastList") || child.has("RefreshControl")) Modifier.weight(1f) else Modifier
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
                    parameters[parameter] = store.evaluate(argument.opt(1), locals, scope)
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
        "Layout" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            val spacing = fields.optDouble("spacing", 0.0).dp
            val style = fields.optJSONObject("style") ?: JSONObject()
            var modifier = style.optDouble("padding").takeIf { style.has("padding") }?.let { Modifier.padding(it.dp) } ?: Modifier
            if (style.has("width")) modifier = modifier.width(style.optDouble("width").dp)
            if (style.has("height")) modifier = modifier.height(style.optDouble("height").dp)
            if (style.has("min_width") || style.has("max_width")) {
                modifier = modifier.widthIn(
                    min = style.optDouble("min_width").takeIf { style.has("min_width") }?.dp ?: Dp.Unspecified,
                    max = style.optDouble("max_width").takeIf { style.has("max_width") }?.dp ?: Dp.Unspecified,
                )
            }
            if (style.has("min_height") || style.has("max_height")) {
                modifier = modifier.heightIn(
                    min = style.optDouble("min_height").takeIf { style.has("min_height") }?.dp ?: Dp.Unspecified,
                    max = style.optDouble("max_height").takeIf { style.has("max_height") }?.dp ?: Dp.Unspecified,
                )
            }
            nexaDevColor(style.optJSONObject("background"), isSystemInDarkTheme())?.let { modifier = modifier.background(it) }
            val cornerRadius = style.optDouble("corner_radius", 0.0).dp
            if (style.has("corner_radius")) modifier = modifier.clip(RoundedCornerShape(cornerRadius))
            nexaDevColor(style.optJSONObject("border_color"), isSystemInDarkTheme())?.let {
                modifier = modifier.border(style.optDouble("border_width", 1.0).dp, it, RoundedCornerShape(cornerRadius))
            }
            if (style.has("opacity")) modifier = modifier.alpha(style.optDouble("opacity").toFloat())
            if (style.has("animation")) modifier = modifier.animateContentSize()
            when (fields.optString("kind")) {
                "Row" -> Row(
                    modifier,
                    horizontalArrangement = Arrangement.spacedBy(spacing),
                    verticalAlignment = when (style.optString("alignment")) {
                        "Start" -> Alignment.Top
                        "End" -> Alignment.Bottom
                        else -> Alignment.CenterVertically
                    },
                ) { RenderChildren(children, module, store, locals, scope) }
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
                    verticalArrangement = Arrangement.spacedBy(spacing),
                    horizontalAlignment = when (style.optString("alignment")) {
                        "Start" -> Alignment.Start
                        "Center" -> Alignment.CenterHorizontally
                        "End" -> Alignment.End
                        else -> Alignment.CenterHorizontally
                    },
                ) { RenderColumnChildren(children, module, store, locals, scope) }
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
                val text = store.stringify(store.evaluate(fields.opt("value"), locals, scope))
                val fontSize = if (style.has("font_size") && !style.isNull("font_size")) style.getDouble("font_size").sp else androidx.compose.ui.unit.TextUnit.Unspecified
                val maxLines = if (style.has("line_limit") && !style.isNull("line_limit")) style.getInt("line_limit") else Int.MAX_VALUE
                val letterSpacing = if (style.has("letter_spacing") && !style.isNull("letter_spacing")) style.getDouble("letter_spacing").sp else androidx.compose.ui.unit.TextUnit.Unspecified
                if (style.has("line_height") && !style.isNull("line_height")) {
                    Text(
                        text = text,
                        color = color ?: Color.Unspecified,
                        fontSize = fontSize,
                        fontWeight = weight,
                        maxLines = maxLines,
                        lineHeight = style.getDouble("line_height").sp,
                        letterSpacing = letterSpacing,
                    )
                } else {
                    Text(
                        text = text,
                        color = color ?: Color.Unspecified,
                        fontSize = fontSize,
                        fontWeight = weight,
                        maxLines = maxLines,
                        letterSpacing = letterSpacing,
                    )
                }
            }
            if (style.optBoolean("selectable")) SelectionContainer { content() } else content()
        }
        "Button" -> Button(onClick = { store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals) }) {
            Text(store.stringify(store.evaluate(fields.opt("label"), locals, scope)))
        }
        "Pressable" -> {
            val disabled = store.evaluate(fields.opt("disabled"), locals, scope) as? Boolean ?: false
            val hapticStyle = fields.optString("haptic").takeIf(String::isNotEmpty)
            val haptic = LocalHapticFeedback.current
            Box(Modifier.combinedClickable(
                enabled = !disabled,
                onClick = {
                    val hapticType = when (hapticStyle) {
                        "Light" -> HapticFeedbackType.TextHandleMove
                        "Medium" -> HapticFeedbackType.LongPress
                        "Heavy" -> HapticFeedbackType.ContextClick
                        else -> null
                    }
                    if (hapticType != null) haptic.performHapticFeedback(hapticType)
                    store.perform(fields.optJSONArray("actions") ?: JSONArray(), scope, locals)
                },
                onLongClick = {
                    store.perform(fields.optJSONArray("long_press_actions") ?: JSONArray(), scope, locals)
                },
            )) {
                RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
            }
        }
        "TextInput" -> {
            val state = fields.optString("state")
            var value by remember(state, scope) { mutableStateOf(store.stringify(store.state(state, scope))) }
            val placeholder = fields.optString("placeholder").takeIf(String::isNotEmpty)
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
            val maxLength = fields.optInt("max_length", Int.MAX_VALUE).coerceAtLeast(0)
            LaunchedEffect(store.moduleRevision, store.focusedFieldKey, focusKey) {
                if (store.focusedFieldKey == focusKey) focusRequester.requestFocus()
            }
            BasicTextField(
                value,
                onValueChange = { next -> next.take(maxLength).let { value = it; store.setState(state, it, scope) } },
                singleLine = fields.optBoolean("multiline").not(),
                visualTransformation = if (fields.optBoolean("secure")) PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
                keyboardOptions = KeyboardOptions(
                    keyboardType = keyboardType,
                    capitalization = capitalization,
                    autoCorrect = fields.optBoolean("autocorrect", true),
                ),
                modifier = Modifier
                    .focusRequester(focusRequester)
                    .onFocusChanged { focusState ->
                        if (focusState.isFocused) store.focusChanged(focusKey)
                        else if (store.focusedFieldKey == focusKey) store.focusChanged(null)
                    },
                decorationBox = { innerTextField ->
                    Box {
                        if (value.isEmpty() && placeholder != null) {
                            Text(placeholder, color = Color.Gray)
                        }
                        innerTextField()
                    }
                },
            )
        }
        "FastList" -> {
            val source = fields.optJSONObject("source")
            val countExpression = source?.opt("Count")
            val itemSource = source?.optJSONObject("Items")
            val itemExpression = itemSource?.opt("collection")
            val sourceItems = itemExpression?.let { store.evaluate(it, locals, scope) as? List<*> }
            val sectionSource = source?.optJSONObject("Sections")
            val sectionExpression = sectionSource?.opt("collection")
            val sourceSections = sectionExpression?.let { store.evaluate(it, locals, scope) as? List<*> }
            val axisValue = fields.opt("axis")
            val gridColumns = (axisValue as? JSONObject)
                ?.optJSONObject("Grid")
                ?.optInt("columns", 0)
                ?.takeIf { it > 0 }
            val axis = when {
                axisValue == "Horizontal" -> "Horizontal"
                gridColumns != null -> "Grid"
                else -> "Vertical"
            }
            if (countExpression == null && sourceItems == null && sourceSections == null) {
                Text("FastList source could not be evaluated.")
            } else {
                val itemCount = countExpression?.let {
                    (store.evaluate(it, locals, scope) as? Number)?.toInt()?.coerceAtLeast(0) ?: 0
                } ?: sourceItems?.size ?: 0
                val count = sourceSections?.sumOf { (it as? List<*>)?.size ?: 0 } ?: itemCount
                val indexName = fields.optString("index", "index")
                val itemName = fields.optString("item").takeIf(String::isNotEmpty)
                val sectionName = fields.optString("section").takeIf(String::isNotEmpty)
                val children = fields.optJSONArray("children") ?: JSONArray()
                val stickyHeaderNodes = fields.optJSONArray("sticky_header")
                val sectionHeader = fields.optJSONArray("section_header")
                val onScroll = fields.optJSONArray("on_scroll")
                val onEndReached = fields.optJSONArray("on_end_reached")
                val refresh = fields.optJSONObject("refresh")
                val refreshState = refresh?.optString("state")?.takeIf(String::isNotEmpty)
                val refreshActions = refresh?.optJSONArray("actions") ?: JSONArray()
                var endReached by remember(countExpression, sourceItems?.size, sourceSections?.size) { mutableStateOf(false) }
                val scrollPositionName = fields.optString("scroll_position").takeIf(String::isNotEmpty)
                val requestedIndex = scrollPositionName?.let { name ->
                    (store.state(name, scope) as? Number)?.toInt()?.coerceIn(0, maxOf(0, count - 1)) ?: 0
                } ?: 0
                val extent = fields.optDouble("item_extent", 0.0).takeIf { it > 0.0 }?.dp
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
                    Column(itemModifier) {
                        RenderChildren(children, module, store, rowLocals, scope)
                    }
                }
                val renderHeader: @Composable (JSONArray, Map<String, Any>) -> Unit = { nodes, headerLocals ->
                    Column(Modifier.fillMaxWidth()) {
                        RenderChildren(nodes, module, store, headerLocals, scope)
                    }
                }
                val observeScroll: suspend (Int, Int, Int) -> Unit = { firstVisible, visibleCount, total ->
                    val callbackLocals = locals + (indexName to firstVisible)
                    if (scrollPositionName != null) {
                        val storedIndex = (store.state(scrollPositionName, scope) as? Number)?.toInt()
                        if (storedIndex != firstVisible) store.setState(scrollPositionName, firstVisible, scope)
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
                        val listState = rememberLazyListState(initialFirstVisibleItemIndex = requestedIndex)
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
                        if (axis == "Horizontal") {
                            LazyRow(state = listState, modifier = modifier.fillMaxWidth()) {
                                items(count = count, key = { itemIndex -> itemIndex }) { itemIndex ->
                                    renderItem(itemIndex, axis)
                                }
                            }
                        } else {
                            LazyColumn(state = listState, modifier = modifier.fillMaxWidth()) {
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
                                    items(count = count, key = { itemIndex -> itemIndex }) { itemIndex ->
                                        renderItem(itemIndex, axis)
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
            Row {
                Text(fields.optString("label"))
                Switch(checked = store.state(state, scope) as? Boolean ?: false, onCheckedChange = { store.setState(state, it, scope) })
            }
        }
        "Image" -> {
            val source = fields.optJSONObject("source") ?: JSONObject()
            val description = fields.optString("description").takeIf(String::isNotEmpty)
            val scale = if (fields.optString("scale") == "Fill") ContentScale.Crop else ContentScale.Fit
            val placeholder = fields.optString("placeholder").takeIf(String::isNotEmpty)
            val asset = source.optString("Asset").takeIf(String::isNotEmpty)
            val remoteExpression = source.opt("RemoteUrl")
            if (asset != null) {
                val context = LocalContext.current
                val resourceId = remember(asset) { context.resources.getIdentifier(asset, "drawable", context.packageName) }
                if (resourceId != 0) {
                    Image(painterResource(resourceId), contentDescription = description, contentScale = scale)
                } else {
                    Text(placeholder ?: "Image unavailable")
                }
            } else if (remoteExpression != null && remoteExpression != JSONObject.NULL) {
                val url = store.stringify(store.evaluate(remoteExpression, locals, scope))
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
                )
            } else {
                Text(placeholder ?: "Image unavailable")
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
            val condition = store.evaluate(fields.opt("condition"), locals, scope) as? Boolean ?: false
            RenderChildren(fields.optJSONArray(if (condition) "then_body" else "else_body") ?: JSONArray(), module, store, locals, scope)
        }
        "When" -> {
            val value = store.stringify(store.evaluate(fields.opt("value"), locals, scope))
            val cases = fields.optJSONArray("cases") ?: JSONArray()
            val matching = (0 until cases.length())
                .mapNotNull(cases::optJSONObject)
                .firstOrNull { item ->
                    store.stringify(store.evaluate(item.opt("value"), locals, scope)) == value
                }
            RenderChildren(matching?.optJSONArray("body") ?: fields.optJSONArray("else_body") ?: JSONArray(), module, store, locals, scope)
        }
        "Link" -> {
            val context = LocalContext.current
            val url = store.stringify(store.evaluate(fields.opt("url"), locals, scope))
            val children = fields.optJSONArray("children") ?: JSONArray()
            Column(Modifier.clickable { openNexaUrl(context, url) }) {
                RenderChildren(children, module, store, locals, scope)
            }
        }
        "Accessibility" -> {
            val label = store.stringify(store.evaluate(fields.opt("label"), locals, scope))
            val hintValue = fields.opt("hint")
            val hint = if (hintValue == null || hintValue == JSONObject.NULL) null else store.stringify(store.evaluate(hintValue, locals, scope))
            val role = when (fields.optString("role")) {
                "Button" -> SemanticsRole.Button
                "Image" -> SemanticsRole.Image
                else -> null
            }
            Column(Modifier.semantics(mergeDescendants = true) {
                contentDescription = label
                if (hint != null) stateDescription = hint
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
            val selected = (store.state(state, scope) as? Number)?.toInt() ?: 0
            Column {
                val activeTab = (0 until tabs.length())
                    .mapNotNull(tabs::optJSONObject)
                    .firstOrNull { it.optInt("index", -1) == selected }
                if (activeTab != null) {
                    RenderChildren(activeTab.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
                }
                NavigationBar {
                    for (index in 0 until tabs.length()) {
                        val tab = tabs.optJSONObject(index) ?: continue
                        val tabIndex = tab.optInt("index", index)
                        val label = tab.optString("label")
                        val icon = tab.optString("icon").takeIf(String::isNotEmpty) ?: label.take(1)
                        val badge = tab.optString("badge").takeIf(String::isNotEmpty)
                        NavigationBarItem(
                            selected = selected == tabIndex,
                            onClick = { store.setState(state, tabIndex, scope) },
                            icon = {
                                BadgedBox(badge = {
                                    if (badge != null) Badge { Text(badge) }
                                }) { Text(icon) }
                            },
                            label = { Text(label) },
                        )
                    }
                }
            }
        }
        "BottomSheet" -> {
            val state = fields.optString("state")
            val isPresented = store.state(state, scope) as? Boolean ?: false
            if (isPresented) {
                ModalBottomSheet(onDismissRequest = { store.setState(state, false, scope) }) {
                    RenderChildren(fields.optJSONArray("children") ?: JSONArray(), module, store, locals, scope)
                }
            }
        }
        "NavigationStack" -> RenderNavigationStack(fields, module, store, locals, scope)
        "NavigationLink" -> {
            val screens = module.optJSONArray("screens") ?: JSONArray()
            val destination = screens.optJSONObject(fields.optInt("destination", -1))
            val children = fields.optJSONArray("children") ?: JSONArray()
            val guard = fields.opt("guard")
            val enabled = guard == null || guard == JSONObject.NULL || store.evaluate(guard, locals, scope) as? Boolean == true
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
            val label = store.stringify(store.evaluate(fields.opt("label"), locals, scope))
            val controller = LocalNexaDevNavController.current
            androidx.compose.material3.TextButton(onClick = { controller?.popBackStack() }) {
                Text(label)
            }
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
