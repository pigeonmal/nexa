package __NEXA_PACKAGE__

import android.net.Uri
import android.app.Activity
import android.content.Intent
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Base64
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.imeNestedScroll
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.Image
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items as gridItems
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import coil3.compose.AsyncImage
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.withFrameNanos
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.semantics.Role as SemanticsRole
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import org.json.JSONArray
import org.json.JSONObject
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.net.Socket
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.Locale
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first

private val LocalNexaDevNavController = staticCompositionLocalOf<NavHostController?> { null }

private fun openNexaUrl(context: Context, value: String) {
    val uri = Uri.parse(value)
    if (uri.scheme !in setOf("https", "http", "mailto", "tel")) return
    runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, uri)) }
}

@Composable
internal fun NexaDevRuntimeRoot(serverURL: String, sessionToken: String) {
    val store = remember { NexaDevStateStore() }
    LaunchedEffect(serverURL, sessionToken) {
        NexaDevSocketClient(serverURL, sessionToken, store).connect()
    }
    val module = store.module
    val latestModule = rememberUpdatedState(module)
    LaunchedEffect(store.appLifecycleEpoch) {
        if (store.appLifecycleEpoch == 0) return@LaunchedEffect
        val readyModule = snapshotFlow { store.module }.filterNotNull().first()
        val actions = readyModule.optJSONArray("on_appear") ?: JSONArray()
        if (readyModule.optBoolean("on_appear_async")) {
            store.performAsync(actions, "app", emptyMap())
        } else {
            store.perform(actions, "app", emptyMap())
        }
    }
    val lifecycleOwner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner, store.appLifecycleEpoch) {
        val observer = LifecycleEventObserver { _, event ->
            val readyModule = latestModule.value ?: return@LifecycleEventObserver
            val actions = when (event) {
                Lifecycle.Event.ON_RESUME -> readyModule.optJSONArray("on_active")
                Lifecycle.Event.ON_PAUSE -> readyModule.optJSONArray("on_inactive")
                Lifecycle.Event.ON_STOP -> readyModule.optJSONArray("on_background")
                else -> null
            }
            actions?.let { store.perform(it, "app", emptyMap()) }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    DisposableEffect(lifecycleOwner) {
        onDispose {
            latestModule.value?.optJSONArray("on_disappear")?.let { store.perform(it, "app", emptyMap()) }
        }
    }
    var fps by remember { mutableIntStateOf(0) }
    var frameTimeMs by remember { mutableStateOf(0.0) }
    LaunchedEffect(store.performanceOverlayEnabled) {
        if (!store.performanceOverlayEnabled) return@LaunchedEffect
        var frameCount = 0
        var sampleStart = 0L
        var previousFrame = 0L
        while (isActive) {
            val frame = withFrameNanos { it }
            if (previousFrame != 0L) frameCount++
            if (sampleStart == 0L) sampleStart = frame
            if (previousFrame != 0L) frameTimeMs = (frame - previousFrame) / 1_000_000.0
            val elapsed = frame - sampleStart
            if (elapsed >= 1_000_000_000L) {
                fps = (frameCount * 1_000_000_000.0 / elapsed).toInt()
                frameTimeMs = if (frameCount > 0) elapsed / 1_000_000.0 / frameCount else 0.0
                frameCount = 0
                sampleStart = frame
            }
            previousFrame = frame
        }
    }
    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize()) {
            if (store.diagnostics.isNotEmpty()) {
                Text(store.diagnostics.joinToString("\n"), color = Color.Red, modifier = Modifier.padding(10.dp))
            }
            if (module == null) {
                Text("Connecting to Nexa…")
            } else {
                val statusBar = module.optJSONObject("status_bar")
                NexaDevStatusBar(statusBar, isSystemInDarkTheme())
                val style = module.optJSONObject("direction")?.optString("style")
                val direction = when (style) {
                    "Rtl" -> LayoutDirection.Rtl
                    "Ltr" -> LayoutDirection.Ltr
                    else -> LocalLayoutDirection.current
                }
                CompositionLocalProvider(LocalLayoutDirection provides direction) {
                    NexaDevNodeList(
                        module.optJSONArray("body") ?: JSONArray(),
                        module,
                        store,
                        modifier = Modifier.fillMaxSize(),
                    )
                }
            }
        }
        if (store.performanceOverlayEnabled) {
            NexaDevPerformanceOverlay(fps, frameTimeMs, Modifier.align(Alignment.TopEnd).padding(8.dp))
        }
    }
}

@Composable
private fun NexaDevStatusBar(config: JSONObject?, isDarkTheme: Boolean) {
    val view = LocalView.current
    val window = (view.context as? Activity)?.window
    val initialStatusBarColor = remember(window) { window?.statusBarColor }
    val initialLightStatusBar = remember(window, view) {
        window?.let { WindowCompat.getInsetsController(it, view).isAppearanceLightStatusBars }
    }
    SideEffect {
        val window = window ?: return@SideEffect
        val controller = WindowCompat.getInsetsController(window, view)
        if (config?.optBoolean("hidden", false) == true) {
            controller.hide(WindowInsetsCompat.Type.statusBars())
        } else {
            controller.show(WindowInsetsCompat.Type.statusBars())
        }
        when (config?.optString("style")) {
            "Light" -> controller.isAppearanceLightStatusBars = false
            "Dark" -> controller.isAppearanceLightStatusBars = true
            else -> controller.isAppearanceLightStatusBars = initialLightStatusBar ?: false
        }
        val color = config?.optJSONObject("background")
        val background = color?.optJSONObject("Static")
            ?: color?.optJSONObject("Adaptive")?.optJSONObject(if (isDarkTheme) "dark" else "light")
        window.statusBarColor = if (background != null) {
            android.graphics.Color.argb(
                background.optInt("alpha", 255),
                background.optInt("red", 0),
                background.optInt("green", 0),
                background.optInt("blue", 0),
            )
        } else {
            initialStatusBarColor ?: android.graphics.Color.TRANSPARENT
        }
    }
}

private class NexaDevStateStore {
    private val values = mutableStateMapOf<String, Any>()
    private var typeSignatures = mutableMapOf<String, String>()
    private var functions = mutableMapOf<String, JSONObject>()
    private val activeFunctions = mutableSetOf<String>()
    var module by mutableStateOf<JSONObject?>(null)
        private set
    var appLifecycleEpoch by mutableIntStateOf(0)
        private set
    var diagnostics by mutableStateOf<List<String>>(emptyList())
        private set
    var performanceOverlayEnabled by mutableStateOf(false)
    var navigationEpoch by mutableStateOf(0)
        private set
    var focusedFieldKey by mutableStateOf<String?>(null)
        private set
    var moduleRevision by mutableStateOf(0)
        private set
    private var navigationRoot: String? = null
    private var navigationScreensSignature: String? = null
    private var focusBindings = mutableMapOf<String, Pair<String, String?>>()
    private var hasInstalledModule = false

    fun install(next: JSONObject) {
        val screens = next.optJSONArray("screens") ?: JSONArray()
        val states = next.optJSONArray("states") ?: JSONArray()
        val nextFunctions = next.optJSONArray("functions") ?: JSONArray()
        functions = (0 until nextFunctions.length()).mapNotNull { index ->
            val function = nextFunctions.optJSONObject(index) ?: return@mapNotNull null
            val name = function.optString("name").takeIf(String::isNotEmpty) ?: return@mapNotNull null
            name to function
        }.toMap().toMutableMap()
        val nextValues = mutableMapOf<String, Any>()
        val nextTypes = mutableMapOf<String, String>()
        val declarations = mutableListOf<Pair<String, JSONObject>>()
        for (index in 0 until states.length()) {
            states.optJSONObject(index)?.let { declarations += "app" to it }
        }
        for (screenIndex in 0 until screens.length()) {
            val screen = screens.optJSONObject(screenIndex) ?: continue
            val screenName = screen.optString("name").takeIf(String::isNotEmpty) ?: continue
            val screenStates = screen.optJSONArray("states") ?: JSONArray()
            for (stateIndex in 0 until screenStates.length()) {
                screenStates.optJSONObject(stateIndex)?.let { declarations += "screen/$screenName" to it }
            }
        }
        for ((scope, state) in declarations) {
            val name = state.optString("name")
            if (name.isEmpty()) continue
            val identity = "$scope/state/$name"
            val signature = state.optJSONObject("ty")?.toString() ?: "null"
            nextTypes[identity] = signature
            val previousValue = values[identity]
            if (typeSignatures[identity] == signature && previousValue != null) {
                nextValues[identity] = previousValue
            } else {
                val initial = state.opt("initial")
                if (initial != null) nextValues[identity] = evaluate(initial, emptyMap(), scope)
            }
        }
        values.clear()
        values.putAll(nextValues)
        typeSignatures = nextTypes
        val nextFocusBindings = mutableMapOf<String, Pair<String, String?>>()
        collectFocusBindings(next.optJSONArray("body") ?: JSONArray(), "app", nextFocusBindings)
        for (screenIndex in 0 until screens.length()) {
            val screen = screens.optJSONObject(screenIndex) ?: continue
            val name = screen.optString("name").takeIf(String::isNotEmpty) ?: continue
            collectFocusBindings(
                screen.optJSONArray("body") ?: JSONArray(),
                "screen/$name",
                nextFocusBindings,
            )
        }
        focusBindings = nextFocusBindings
        val currentFocus = focusedFieldKey?.takeIf(nextFocusBindings::containsKey)
        focusedFieldKey = currentFocus ?: nextFocusBindings.entries.firstOrNull { (_, binding) ->
            val focusedState = binding.second ?: return@firstOrNull false
            nextValues["${binding.first}/state/$focusedState"] as? Boolean == true
        }?.key
        val rootIndex = next.optJSONArray("body")?.optJSONObject(0)
            ?.optJSONObject("NavigationStack")?.optInt("root", -1) ?: -1
        val nextRoot = screens.optJSONObject(rootIndex)?.optString("name")?.takeIf(String::isNotEmpty)
        val nextScreensSignature = (0 until screens.length()).joinToString("|") { index ->
            val screen = screens.optJSONObject(index) ?: return@joinToString ""
            val parameters = screen.optJSONArray("parameters") ?: JSONArray()
            val parameterSignature = (0 until parameters.length()).joinToString(",") { parameterIndex ->
                val parameter = parameters.optJSONObject(parameterIndex) ?: return@joinToString ""
                "${parameter.optString("name")}:${parameter.optJSONObject("ty")?.toString() ?: "null"}"
            }
            "${screen.optString("name")}($parameterSignature)"
        }
        if (navigationRoot != nextRoot || navigationScreensSignature != nextScreensSignature) {
            navigationEpoch += 1
            navigationRoot = nextRoot
            navigationScreensSignature = nextScreensSignature
        }
        diagnostics = emptyList()
        module = next
        if (!hasInstalledModule) {
            hasInstalledModule = true
            appLifecycleEpoch += 1
        }
        moduleRevision += 1
    }

    fun hotRestart() {
        val latestModule = module ?: return
        values.clear()
        typeSignatures.clear()
        activeFunctions.clear()
        focusBindings.clear()
        focusedFieldKey = null
        navigationRoot = null
        navigationScreensSignature = null
        appLifecycleEpoch += 1
        navigationEpoch += 1
        install(latestModule)
    }

    private fun collectFocusBindings(
        value: Any?,
        scope: String,
        bindings: MutableMap<String, Pair<String, String?>>,
    ) {
        when (value) {
            is JSONArray -> for (index in 0 until value.length()) {
                collectFocusBindings(value.opt(index), scope, bindings)
            }
            is JSONObject -> {
                val input = value.optJSONObject("TextInput")
                val state = input?.optString("state")?.takeIf(String::isNotEmpty)
                if (state != null) {
                    val focusedState = input.optString("focused").takeIf(String::isNotEmpty)
                    bindings["$scope/input/$state"] = scope to focusedState
                }
                val keys = value.keys()
                while (keys.hasNext()) {
                    collectFocusBindings(value.opt(keys.next()), scope, bindings)
                }
            }
        }
    }

    fun focusChanged(nextIdentity: String?) {
        if (focusedFieldKey == nextIdentity) return
        val previousIdentity = focusedFieldKey
        focusedFieldKey = nextIdentity
        for (identity in listOfNotNull(previousIdentity, nextIdentity).distinct()) {
            val (scope, state) = focusBindings[identity] ?: continue
            if (state != null) setState(state, identity == nextIdentity, scope)
        }
    }

    fun applyPatch(patch: JSONObject) {
        val next = module?.let { JSONObject(it.toString()) } ?: return
        val operations = patch.optJSONArray("operations") ?: return
        for (index in 0 until operations.length()) {
            val operation = operations.optJSONObject(index) ?: return
            if (!applyOperation(next, operation)) return
        }
        install(next)
    }

    private fun applyOperation(root: JSONObject, operation: JSONObject): Boolean {
        val kind = operation.optString("op")
        val path = operation.optString("path")
        if (kind !in setOf("set", "remove") || !path.startsWith('/')) return false
        val segments = path.drop(1).split('/').map {
            it.replace("~1", "/").replace("~0", "~")
        }
        if (segments.isEmpty() || segments.any(String::isEmpty)) return false
        return updateContainer(root, segments, 0, kind, operation.opt("value"))
    }

    private fun updateContainer(
        container: Any,
        path: List<String>,
        index: Int,
        kind: String,
        value: Any?,
    ): Boolean {
        val key = path[index]
        val isLeaf = index == path.lastIndex
        return when (container) {
            is JSONObject -> {
                if (isLeaf) {
                    when (kind) {
                        "set" -> {
                            if (value == null) false else { container.put(key, value); true }
                        }
                        "remove" -> { container.remove(key); true }
                        else -> false
                    }
                } else {
                    val child = container.opt(key) ?: return false
                    updateContainer(child, path, index + 1, kind, value)
                }
            }
            is JSONArray -> {
                val arrayIndex = key.toIntOrNull() ?: return false
                if (arrayIndex !in 0 until container.length()) return false
                if (isLeaf) {
                    if (kind != "set" || value == null) false
                    else { container.put(arrayIndex, value); true }
                } else {
                    val child = container.opt(arrayIndex) ?: return false
                    updateContainer(child, path, index + 1, kind, value)
                }
            }
            else -> false
        }
    }

    fun publishDiagnostics(next: List<String>) {
        diagnostics = next
    }

    fun state(name: String, scope: String = "app"): Any =
        values["$scope/state/$name"] ?: values["app/state/$name"] ?: ""

    fun setState(name: String, value: Any, scope: String = "app") {
        val scopedIdentity = "$scope/state/$name"
        val identity = if (typeSignatures.containsKey(scopedIdentity) || scope == "app") scopedIdentity else "app/state/$name"
        values[identity] = value
    }

    fun locals(scope: String, parameters: Map<String, Any>): Map<String, Any> {
        return parameters
    }

    fun encodeScreenRoute(
        screen: JSONObject,
        expressions: JSONArray,
        locals: Map<String, Any>,
        scope: String,
    ): String {
        val parameters = screen.optJSONArray("parameters") ?: JSONArray()
        val values = JSONObject()
        for (index in 0 until minOf(parameters.length(), expressions.length())) {
            val parameter = parameters.optJSONObject(index) ?: continue
            val name = parameter.optString("name")
            if (name.isNotEmpty()) values.put(name, evaluate(expressions.opt(index), locals, scope))
        }
        return encodeScreenRoute(screen.optString("name"), values)
    }

    fun encodeScreenRoute(screen: String, values: JSONObject = JSONObject()): String {
        val payload = JSONObject().put("screen", screen).put("parameters", values)
        val encoded = Base64.encodeToString(
            payload.toString().toByteArray(Charsets.UTF_8),
            Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING,
        )
        return "nexa/$encoded"
    }

    fun decodeScreenRoute(encoded: String): JSONObject? = runCatching {
        JSONObject(Base64.decode(encoded, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING).toString(Charsets.UTF_8))
    }.getOrNull()

    fun screenParameters(screen: JSONObject, values: JSONObject): Map<String, Any> {
        val parameters = screen.optJSONArray("parameters") ?: JSONArray()
        return buildMap {
            for (index in 0 until parameters.length()) {
                val parameter = parameters.optJSONObject(index) ?: continue
                val name = parameter.optString("name")
                val value = values.opt(name)
                if (name.isNotEmpty() && value != null && value != JSONObject.NULL) put(name, value)
            }
        }
    }

    fun perform(actions: JSONArray, scope: String, locals: Map<String, Any>) {
        for (index in 0 until actions.length()) {
            val action = actions.optJSONObject(index) ?: continue
            val assignment = action.optJSONObject("Assign")
            if (assignment != null) {
                val name = assignment.optString("name")
                val expression = assignment.opt("value")
                if (name.isNotEmpty() && expression != null) setState(name, evaluate(expression, locals, scope), scope)
                continue
            }
            val branch = action.optJSONObject("If") ?: continue
            val condition = evaluate(branch.opt("condition"), locals, scope) as? Boolean ?: false
            val selected = if (condition) branch.optJSONArray("then_branch") else branch.optJSONArray("else_branch")
            if (selected != null) perform(selected, scope, locals)
        }
    }

    suspend fun performAsync(actions: JSONArray, scope: String, locals: Map<String, Any>) {
        for (index in 0 until actions.length()) {
            val action = actions.optJSONObject(index) ?: continue
            val assignment = action.optJSONObject("Assign")
            if (assignment != null) {
                val name = assignment.optString("name")
                val expression = assignment.opt("value")
                if (name.isNotEmpty() && expression != null) {
                    setState(name, evaluateAsync(expression, locals, scope), scope)
                }
                continue
            }
            val branch = action.optJSONObject("If")
            if (branch != null) {
                val condition = evaluateAsync(branch.opt("condition"), locals, scope) as? Boolean ?: false
                val selected = if (condition) branch.optJSONArray("then_branch") else branch.optJSONArray("else_branch")
                if (selected != null) performAsync(selected, scope, locals)
                continue
            }
            val expression = action.opt("Expression")
            if (expression != null) evaluateAsync(expression, locals, scope)
        }
    }

    private suspend fun evaluateAsync(raw: Any?, locals: Map<String, Any>, scope: String): Any {
        val expression = raw as? JSONObject ?: return raw ?: JSONObject.NULL
        val kind = expression.keys().asSequence().firstOrNull() ?: return JSONObject.NULL
        val payload = expression.opt(kind)
        return when (kind) {
            "Await", "TryAwait" -> evaluateAsync(payload, locals, scope)
            "Call" -> invokeFunctionAsync(payload as? JSONObject ?: return JSONObject.NULL, locals, scope)
            "Add" -> {
                val tuple = payload as? JSONArray ?: return 0L
                val left = evaluateAsync(tuple.opt(0), locals, scope)
                val right = evaluateAsync(tuple.opt(1), locals, scope)
                if (left is String && right is String) left + right
                else {
                    val sum = number(left) + number(right)
                    if (sum % 1.0 == 0.0) sum.toLong() else sum
                }
            }
            "Not" -> !(evaluateAsync(payload, locals, scope) as? Boolean ?: false)
            "Binary" -> {
                val binary = payload as? JSONObject ?: return false
                compare(
                    binary.optString("op"),
                    evaluateAsync(binary.opt("left"), locals, scope),
                    evaluateAsync(binary.opt("right"), locals, scope),
                )
            }
            else -> evaluate(raw, locals, scope)
        }
    }

    private suspend fun invokeFunctionAsync(call: JSONObject, locals: Map<String, Any>, scope: String): Any {
        val name = call.optString("name")
        val function = functions[name] ?: return JSONObject.NULL
        if (!activeFunctions.add(name)) return JSONObject.NULL
        try {
            val parameters = function.optJSONArray("parameters") ?: JSONArray()
            val arguments = call.optJSONArray("arguments") ?: JSONArray()
            if (parameters.length() != arguments.length()) return JSONObject.NULL
            val localScope = locals.toMutableMap()
            for (index in 0 until parameters.length()) {
                val parameter = parameters.optJSONObject(index) ?: continue
                val parameterName = parameter.optString("name")
                localScope[parameterName] = evaluateAsync(arguments.opt(index), localScope, scope)
            }
            val functionLocals = function.optJSONArray("locals") ?: JSONArray()
            for (index in 0 until functionLocals.length()) {
                val local = functionLocals.optJSONObject(index) ?: continue
                val localName = local.optString("name")
                localScope[localName] = evaluateAsync(local.opt("initial"), localScope, scope)
            }
            return evaluateAsync(function.opt("body"), localScope, scope)
        } finally {
            activeFunctions.remove(name)
        }
    }

    fun evaluate(raw: Any?, locals: Map<String, Any>, scope: String = "app"): Any {
        val expression = raw as? JSONObject ?: return raw ?: JSONObject.NULL
        val kind = expression.keys().asSequence().firstOrNull() ?: return JSONObject.NULL
        val payload = expression.opt(kind)
        return when (kind) {
            "String", "Bool" -> payload ?: JSONObject.NULL
            "Array" -> {
                val entries = payload as? JSONArray ?: return emptyList<Any>()
                buildList(entries.length()) {
                    for (index in 0 until entries.length()) {
                        add(evaluate(entries.opt(index), locals, scope))
                    }
                }
            }
            "Number" -> {
                val number = (payload as? JSONObject)?.optString("raw", "0") ?: "0"
                if (number.contains('.')) number.toDoubleOrNull() ?: 0.0 else number.toLongOrNull() ?: 0L
            }
            "State" -> {
                val tuple = payload as? JSONArray ?: return JSONObject.NULL
                val name = tuple.optString(0)
                locals[name] ?: state(name, scope)
            }
            "Interpolation" -> {
                val parts = payload as? JSONArray ?: return ""
                buildString {
                    for (index in 0 until parts.length()) {
                        val part = parts.optJSONObject(index) ?: continue
                        val partKind = part.keys().asSequence().firstOrNull() ?: continue
                        val value = part.opt(partKind)
                        append(if (partKind == "Literal") value?.toString() ?: "" else stringify(evaluate(value, locals, scope)))
                    }
                }
            }
            "Add" -> {
                val tuple = payload as? JSONArray ?: return 0L
                val left = evaluate(tuple.opt(0), locals, scope)
                val right = evaluate(tuple.opt(1), locals, scope)
                if (left is String && right is String) left + right
                else {
                    val sum = number(left) + number(right)
                    if (sum % 1.0 == 0.0) sum.toLong() else sum
                }
            }
            "Not" -> !(evaluate(payload, locals, scope) as? Boolean ?: false)
            "Binary" -> {
                val binary = payload as? JSONObject ?: return false
                compare(
                    binary.optString("op"),
                    evaluate(binary.opt("left"), locals, scope),
                    evaluate(binary.opt("right"), locals, scope),
                )
            }
            "Call" -> invokeFunction(payload as? JSONObject ?: return JSONObject.NULL, locals, scope)
            else -> JSONObject.NULL
        }
    }

    private fun invokeFunction(call: JSONObject, locals: Map<String, Any>, scope: String): Any {
        val name = call.optString("name")
        val function = functions[name] ?: return JSONObject.NULL
        if (!activeFunctions.add(name)) return JSONObject.NULL
        try {
            val parameters = function.optJSONArray("parameters") ?: JSONArray()
            val arguments = call.optJSONArray("arguments") ?: JSONArray()
            if (parameters.length() != arguments.length()) return JSONObject.NULL
            val localScope = locals.toMutableMap()
            for (index in 0 until parameters.length()) {
                val parameter = parameters.optJSONObject(index) ?: continue
                val parameterName = parameter.optString("name")
                localScope[parameterName] = evaluate(arguments.opt(index), localScope, scope)
            }
            val functionLocals = function.optJSONArray("locals") ?: JSONArray()
            for (index in 0 until functionLocals.length()) {
                val local = functionLocals.optJSONObject(index) ?: continue
                val localName = local.optString("name")
                localScope[localName] = evaluate(local.opt("initial"), localScope, scope)
            }
            return evaluate(function.opt("body"), localScope, scope)
        } finally {
            activeFunctions.remove(name)
        }
    }

    fun stringify(value: Any): String = when (value) {
        JSONObject.NULL -> "null"
        is Double -> if (value % 1.0 == 0.0) value.toLong().toString() else value.toString()
        else -> value.toString()
    }

    private fun compare(op: String, left: Any, right: Any): Boolean = when (op) {
        "And" -> (left as? Boolean == true) && (right as? Boolean == true)
        "Or" -> (left as? Boolean == true) || (right as? Boolean == true)
        "Equal" -> stringify(left) == stringify(right)
        "NotEqual" -> stringify(left) != stringify(right)
        "Less" -> number(left) < number(right)
        "LessEqual" -> number(left) <= number(right)
        "Greater" -> number(left) > number(right)
        "GreaterEqual" -> number(left) >= number(right)
        else -> false
    }

    private fun number(value: Any): Double = when (value) {
        is Number -> value.toDouble()
        is String -> value.toDoubleOrNull() ?: 0.0
        else -> 0.0
    }
}

private class NexaDevSocketClient(
    private val serverURL: String,
    private val token: String,
    private val store: NexaDevStateStore,
) {
    private val active = AtomicBoolean(false)
    private val mainHandler = Handler(Looper.getMainLooper())
    @Volatile private var currentRevision: String? = null

    fun connect() {
        if (!active.compareAndSet(false, true)) return
        Thread({ runConnection() }, "nexa-dev-websocket").apply { isDaemon = true }.start()
    }

    private fun runConnection() {
        while (active.get()) {
            try {
                connectOnce()
            } catch (error: Exception) {
                if (active.get()) android.util.Log.e("NexaDevRuntime", "Dev connection ended; retrying", error)
            }
            if (active.get()) {
                try {
                    Thread.sleep(1_000)
                } catch (_: InterruptedException) {
                    Thread.currentThread().interrupt()
                    active.set(false)
                }
            }
        }
    }

    private fun connectOnce() {
        val parsed = Uri.parse(serverURL)
        val host = parsed.host ?: return
        val port = if (parsed.port > 0) parsed.port else 80
        Socket(host, port).use { socket ->
                socket.tcpNoDelay = true
                val input = BufferedInputStream(socket.getInputStream())
                val output = BufferedOutputStream(socket.getOutputStream())
                val keyBytes = ByteArray(16).also(SecureRandom()::nextBytes)
                val key = Base64.encodeToString(keyBytes, Base64.NO_WRAP)
                output.write("GET / HTTP/1.1\r\nHost: $host:$port\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: $key\r\nSec-WebSocket-Version: 13\r\n\r\n".toByteArray())
                output.flush()
                val response = readHttpHeaders(input)
                val expected = Base64.encodeToString(
                    MessageDigest.getInstance("SHA-1").digest((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").toByteArray()),
                    Base64.NO_WRAP,
                )
                require(response.startsWith("HTTP/1.1 101") && response.contains("Sec-WebSocket-Accept: $expected", ignoreCase = true)) {
                    "Nexa dev server rejected WebSocket upgrade"
                }
                sendFrame(output, 0x1, JSONObject()
                    .put("type", "hello")
                    .put("payload", JSONObject()
                        .put("protocol_version", 3)
                        .put("session_token", token)
                        .put("target", "android"))
                    .toString())
                while (active.get()) {
                    val frame = readFrame(input) ?: break
                    when (frame.first) {
                        0x1 -> handle(frame.second, output)
                        0x8 -> break
                        0x9 -> sendFrame(output, 0xA, frame.second)
                    }
                }
        }
    }

    private fun handle(text: String, output: BufferedOutputStream) {
        val envelope = JSONObject(text)
        val payload = envelope.optJSONObject("payload") ?: return
        when (envelope.optString("type")) {
            "full_module" -> {
                val devModule = payload.optJSONObject("module") ?: return
                val module = devModule.optJSONObject("module") ?: return
                currentRevision = devModule.optString("revision").takeIf(String::isNotEmpty)
                val revision = currentRevision ?: return
                onMainAndWait { store.install(module) }
                acknowledge(output, revision)
                android.util.Log.i("NexaDevRuntime", "Applied module $revision")
            }
            "patch" -> {
                val patch = payload.optJSONObject("patch") ?: return
                val baseRevision = patch.optString("base_revision")
                if (currentRevision != baseRevision) {
                    sendFrame(output, 0x1, JSONObject().put("type", "request_full_module").toString())
                    return
                }
                val revision = patch.optString("revision").takeIf(String::isNotEmpty) ?: return
                currentRevision = revision
                onMainAndWait { store.applyPatch(patch) }
                acknowledge(output, revision)
                android.util.Log.i("NexaDevRuntime", "Applied patch $revision")
            }
            "diagnostics" -> {
                val entries = payload.optJSONArray("diagnostics") ?: JSONArray()
                val messages = (0 until entries.length()).map { index ->
                    val diagnostic = entries.optJSONObject(index) ?: JSONObject()
                    "${diagnostic.optString("file", "<unknown>")}:${diagnostic.optInt("line", 1)}:${diagnostic.optInt("column", 1)}: ${diagnostic.optString("message", "compile error") }"
                }
                mainHandler.post { store.publishDiagnostics(messages) }
            }
            "restart" -> mainHandler.post { store.hotRestart() }
            "performance_overlay" -> {
                val enabled = payload.optBoolean("enabled", false)
                mainHandler.post { store.performanceOverlayEnabled = enabled }
            }
        }
    }

    private fun onMainAndWait(update: () -> Unit) {
        val completed = CountDownLatch(1)
        var failure: Throwable? = null
        if (!mainHandler.post {
                try {
                    update()
                } catch (error: Throwable) {
                    failure = error
                } finally {
                    completed.countDown()
                }
            }
        ) {
            throw IllegalStateException("Nexa DevRuntime main thread is unavailable")
        }
        if (!completed.await(10, TimeUnit.SECONDS)) {
            throw IllegalStateException("Nexa DevRuntime update timed out on the main thread")
        }
        failure?.let { throw it }
    }

    private fun acknowledge(output: BufferedOutputStream, revision: String) {
        sendFrame(output, 0x1, JSONObject()
            .put("type", "acknowledge")
            .put("payload", JSONObject().put("revision", revision))
            .toString())
    }

    private fun readHttpHeaders(input: BufferedInputStream): String {
        val bytes = ArrayList<Byte>()
        while (bytes.size < 16_384) {
            val next = input.read()
            if (next < 0) break
            bytes.add(next.toByte())
            val count = bytes.size
            if (count >= 4 && bytes[count - 4] == '\r'.code.toByte() && bytes[count - 3] == '\n'.code.toByte() &&
                bytes[count - 2] == '\r'.code.toByte() && bytes[count - 1] == '\n'.code.toByte()
            ) break
        }
        return bytes.toByteArray().toString(Charsets.ISO_8859_1)
    }

    private fun readFrame(input: BufferedInputStream): Pair<Int, String>? {
        val first = input.read()
        if (first < 0) return null
        val second = input.read()
        if (second < 0) return null
        var length = (second and 0x7f).toLong()
        if (length == 126L) length = ((input.read().toLong() and 0xff) shl 8) or (input.read().toLong() and 0xff)
        else if (length == 127L) {
            length = 0
            repeat(8) { length = (length shl 8) or (input.read().toLong() and 0xff) }
        }
        require(length <= 32L * 1024L * 1024L) { "Nexa dev frame exceeds 32 MiB" }
        val masked = (second and 0x80) != 0
        val mask = if (masked) ByteArray(4).also { readFully(input, it) } else null
        val payload = ByteArray(length.toInt())
        readFully(input, payload)
        if (mask != null) payload.indices.forEach { index -> payload[index] = (payload[index].toInt() xor mask[index % 4].toInt()).toByte() }
        return (first and 0x0f) to payload.toString(Charsets.UTF_8)
    }

    private fun sendFrame(output: BufferedOutputStream, opcode: Int, text: String) {
        synchronized(output) {
        val payload = text.toByteArray(Charsets.UTF_8)
        val mask = ByteArray(4).also(SecureRandom()::nextBytes)
        output.write(0x80 or opcode)
        when {
            payload.size < 126 -> output.write(0x80 or payload.size)
            payload.size <= 0xffff -> {
                output.write(0x80 or 126)
                output.write(payload.size ushr 8)
                output.write(payload.size)
            }
            else -> {
                output.write(0x80 or 127)
                for (shift in 56 downTo 0 step 8) output.write(payload.size.toLong().ushr(shift).toInt())
            }
        }
        output.write(mask)
        payload.indices.forEach { index -> output.write(payload[index].toInt() xor mask[index % 4].toInt()) }
        output.flush()
        }
    }

    private fun readFully(input: BufferedInputStream, bytes: ByteArray) {
        var offset = 0
        while (offset < bytes.size) {
            val count = input.read(bytes, offset, bytes.size - offset)
            if (count < 0) throw java.io.EOFException("WebSocket frame ended early")
            offset += count
        }
    }
}

@Composable
private fun NexaDevPerformanceOverlay(fps: Int, frameTimeMs: Double, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier
            .background(Color.Black.copy(alpha = 0.78f), RoundedCornerShape(8.dp))
            .padding(10.dp),
    ) {
        Text("Nexa Performance", color = Color.White, fontSize = 12.sp)
        Text("$fps FPS", color = Color.White, fontSize = 12.sp)
        Text("Frame ${String.format(Locale.US, "%.1f ms", frameTimeMs)}", color = Color.White, fontSize = 12.sp)
    }
}

@Composable
private fun NexaDevNodeList(
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
private fun ColumnScope.RenderColumnChildren(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    for (index in 0 until nodes.length()) {
        androidx.compose.runtime.key(index) {
            val child = nodes.optJSONObject(index) ?: JSONObject()
            val modifier = if (child.has("FastList") || child.has("RefreshControl")) Modifier.weight(1f) else Modifier
            NexaDevNode(child, module, store, locals, scope, modifier)
        }
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
private fun NexaDevNode(
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
        "Layout" -> {
            val children = fields.optJSONArray("children") ?: JSONArray()
            val spacing = fields.optDouble("spacing", 0.0).dp
            val style = fields.optJSONObject("style") ?: JSONObject()
            val modifier = style.optDouble("padding").takeIf { style.has("padding") }?.let { Modifier.padding(it.dp) } ?: Modifier
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
        "Text" -> Text(store.stringify(store.evaluate(fields.opt("value"), locals, scope)))
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
                    if (hapticStyle != null) haptic.performHapticFeedback(HapticFeedbackType.LongPress)
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
            val focusKey = "$scope/input/$state"
            val focusRequester = remember(focusKey) { FocusRequester() }
            LaunchedEffect(store.moduleRevision, store.focusedFieldKey, focusKey) {
                if (store.focusedFieldKey == focusKey) focusRequester.requestFocus()
            }
            BasicTextField(
                value,
                onValueChange = { value = it; store.setState(state, it, scope) },
                modifier = Modifier
                    .focusRequester(focusRequester)
                    .onFocusChanged { focusState ->
                        if (focusState.isFocused) store.focusChanged(focusKey)
                        else if (store.focusedFieldKey == focusKey) store.focusChanged(null)
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
private fun RenderNavigationStack(
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    val screens = module.optJSONArray("screens") ?: JSONArray()
    val rootScreen = screens.optJSONObject(fields.optInt("root", -1)) ?: return
    val route = store.encodeScreenRoute(
        rootScreen,
        fields.optJSONArray("arguments") ?: JSONArray(),
        locals,
        scope,
    )
    androidx.compose.runtime.key(store.navigationEpoch) {
        val navController = rememberNavController()
        NavHost(navController = navController, startDestination = route) {
            composable("nexa/{routeData}") { backStackEntry ->
                val routeData = backStackEntry.arguments?.getString("routeData")
                val destination = routeData?.let(store::decodeScreenRoute)
                val screenName = destination?.optString("screen")
                val screen = (0 until screens.length())
                    .mapNotNull(screens::optJSONObject)
                    .firstOrNull { it.optString("name") == screenName }
                if (screen != null && destination != null) {
                    val parameters = store.screenParameters(
                        screen,
                        destination.optJSONObject("parameters") ?: JSONObject(),
                    )
                    CompositionLocalProvider(LocalNexaDevNavController provides navController) {
                        NexaDevScreenLifecycle(screen, store, parameters)
                        NexaDevNodeList(
                            screen.optJSONArray("body") ?: JSONArray(),
                            module,
                            store,
                            parameters,
                            "screen/${screen.optString("name")}",
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun NexaDevScreenLifecycle(
    screen: JSONObject,
    store: NexaDevStateStore,
    parameters: Map<String, Any>,
) {
    val lifecycleOwner = LocalLifecycleOwner.current
    val latestScreen = rememberUpdatedState(screen)
    val latestParameters = rememberUpdatedState(parameters)
    val coroutineScope = rememberCoroutineScope()
    val scope = "screen/${screen.optString("name")}"
    DisposableEffect(lifecycleOwner, scope) {
        val observer = LifecycleEventObserver { _, event ->
            val currentScreen = latestScreen.value
            val actions = when (event) {
                Lifecycle.Event.ON_RESUME -> currentScreen.optJSONArray("on_appear")
                Lifecycle.Event.ON_PAUSE -> currentScreen.optJSONArray("on_disappear")
                else -> null
            }
            actions?.let {
                if (event == Lifecycle.Event.ON_RESUME && currentScreen.optBoolean("on_appear_async")) {
                    coroutineScope.launch { store.performAsync(it, scope, latestParameters.value) }
                } else {
                    store.perform(it, scope, latestParameters.value)
                }
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
}

@Composable
private fun RenderChildren(
    nodes: JSONArray,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
) {
    for (index in 0 until nodes.length()) {
        androidx.compose.runtime.key(index) {
            NexaDevNode(nodes.optJSONObject(index) ?: JSONObject(), module, store, locals, scope)
        }
    }
}
