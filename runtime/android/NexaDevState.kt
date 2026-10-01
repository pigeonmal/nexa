package __NEXA_PACKAGE__

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import java.util.concurrent.ConcurrentHashMap

internal data class NexaDevNativeEventSubscription(
    val receiver: Any,
    val property: String,
    var actions: JSONArray,
    var parameters: JSONArray,
    val scope: String,
    val locals: Map<String, Any>,
)

internal data class NexaDevNetworkStatusSubscription(
    val id: String,
    val parameter: String,
    var actions: JSONArray,
    val scope: String,
    val locals: Map<String, Any>,
)

internal class NexaDevStateStore(internal val context: Context) {
    internal val eventScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    internal val foregroundTasks = ConcurrentHashMap<String, Job>()
    internal val values = mutableStateMapOf<String, Any>()
    internal var typeSignatures = mutableMapOf<String, String>()
    internal var functions = mutableMapOf<String, JSONObject>()
    internal var structs = mutableMapOf<String, JSONArray>()
    internal var enumCases = mutableMapOf<String, List<String>>()
    internal val activeFunctions = mutableSetOf<String>()
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
        internal set
    private var navigationRoot: String? = null
    private var navigationScreensSignature: String? = null
    internal var focusBindings = mutableMapOf<String, Pair<String, String?>>()
    internal val nativeEventSubscriptions = mutableListOf<NexaDevNativeEventSubscription>()
    internal val networkStatusSubscriptions = mutableListOf<NexaDevNetworkStatusSubscription>()
    internal val activeScreenParameters = mutableMapOf<String, Map<String, Any>>()
    internal var pendingPluginFailure: NexaDevPluginFailure? = null
    private var hasInstalledModule = false

    fun install(next: JSONObject) {
        val isHotReplacement = hasInstalledModule
        if (isHotReplacement) clearNativeTasks()
        val screens = next.optJSONArray("screens") ?: JSONArray()
        val states = next.optJSONArray("states") ?: JSONArray()
        val nextFunctions = next.optJSONArray("functions") ?: JSONArray()
        functions = (0 until nextFunctions.length()).mapNotNull { index ->
            val function = nextFunctions.optJSONObject(index) ?: return@mapNotNull null
            val name = function.optString("name").takeIf(String::isNotEmpty) ?: return@mapNotNull null
            name to function
        }.toMap().toMutableMap()
        val nextStructs = next.optJSONArray("structs") ?: JSONArray()
        structs = (0 until nextStructs.length()).mapNotNull { index ->
            val declaration = nextStructs.optJSONObject(index) ?: return@mapNotNull null
            val name = declaration.optString("name").takeIf(String::isNotEmpty) ?: return@mapNotNull null
            name to (declaration.optJSONArray("fields") ?: JSONArray())
        }.toMap().toMutableMap()
        val nextEnums = next.optJSONArray("enums") ?: JSONArray()
        enumCases = (0 until nextEnums.length()).mapNotNull { index ->
            val declaration = nextEnums.optJSONObject(index) ?: return@mapNotNull null
            val name = declaration.optString("name").takeIf(String::isNotEmpty) ?: return@mapNotNull null
            val cases = declaration.optJSONArray("cases") ?: JSONArray()
            name to (0 until cases.length()).map { cases.optString(it) }
        }.toMap().toMutableMap()
        val nextValues = mutableMapOf<String, Any>()
        val nextTypes = mutableMapOf<String, String>()
        val initialLocals = mutableMapOf<String, Any>()
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
        for ((scope, declaration) in declarations) {
            val name = declaration.optString("name")
            val identity = "$scope/state/$name"
            val typeSignature = declaration.opt("ty")?.toString() ?: ""
            nextTypes[identity] = typeSignature
            val previousType = typeSignatures[identity]
            val previousValue = values[identity]
            val resolvedValue = if (previousType == typeSignature && previousValue != null) {
                previousValue
            } else if (declaration.has("initial")) {
                evaluate(declaration.get("initial"), initialLocals, scope)
            } else {
                continue
            }
            nextValues[identity] = resolvedValue
            initialLocals[name] = resolvedValue
        }
        values.clear()
        values.putAll(nextValues)
        typeSignatures = nextTypes
        if (isHotReplacement) {
            val currentScreens = (0 until screens.length())
                .mapNotNull(screens::optJSONObject)
                .mapTo(mutableSetOf()) { it.optString("name") }
            activeScreenParameters.keys.removeAll { it.substringAfterLast('/') !in currentScreens }
        }
        val nextFocusBindings = mutableMapOf<String, Pair<String, String?>>()
        collectFocusBindings(next.optJSONArray("body") ?: JSONArray(), "app", nextFocusBindings)
        for (screenIndex in 0 until screens.length()) {
            val screen = screens.optJSONObject(screenIndex) ?: continue
            val screenName = screen.optString("name").takeIf(String::isNotEmpty) ?: continue
            collectFocusBindings(
                screen.optJSONArray("body") ?: JSONArray(),
                "screen/$screenName",
                nextFocusBindings,
            )
        }
        focusBindings = nextFocusBindings
        val currentFocus = focusedFieldKey
        if (currentFocus != null && currentFocus !in nextFocusBindings) {
            focusedFieldKey = null
        }
        if (focusedFieldKey == null) {
            val matching = nextFocusBindings.entries.firstOrNull { (_, binding) ->
                val stateName = binding.second ?: return@firstOrNull false
                values.containsKey("${binding.first}/state/$stateName")
            }
            if (matching != null) focusedFieldKey = matching.key
        }
        val currentRoot = screens.optJSONObject(0)?.optString("name")
        val currentScreensSignature = (0 until screens.length()).joinToString(";") {
            screens.optJSONObject(it)?.optString("name").orEmpty()
        }
        if (navigationRoot != currentRoot || navigationScreensSignature != currentScreensSignature) {
            navigationRoot = currentRoot
            navigationScreensSignature = currentScreensSignature
            navigationEpoch++
        }
    module = next
        if (isHotReplacement) refreshNativeEventSubscriptions(next)
        moduleRevision++
        diagnostics = emptyList()
        if (!hasInstalledModule) {
            hasInstalledModule = true
            appLifecycleEpoch++
        }
    }

    fun hotRestart() {
        clearNativeTasks()
        clearNativeEventSubscriptions()
        activeScreenParameters.clear()
        val current = module ?: return
        val states = current.optJSONArray("states") ?: JSONArray()
        val screens = current.optJSONArray("screens") ?: JSONArray()
        values.clear()
        val initialLocals = mutableMapOf<String, Any>()
        for (index in 0 until states.length()) {
            val state = states.optJSONObject(index) ?: continue
            val name = state.optString("name")
            if (state.has("initial")) {
                val value = evaluate(state.get("initial"), initialLocals, "app")
                values["app/state/$name"] = value
                initialLocals[name] = value
            }
        }
        for (screenIndex in 0 until screens.length()) {
            val screen = screens.optJSONObject(screenIndex) ?: continue
            val screenName = screen.optString("name").takeIf(String::isNotEmpty) ?: continue
            val screenStates = screen.optJSONArray("states") ?: JSONArray()
            for (stateIndex in 0 until screenStates.length()) {
                val state = screenStates.optJSONObject(stateIndex) ?: continue
                val name = state.optString("name")
                if (state.has("initial")) {
                    val value = evaluate(state.get("initial"), initialLocals, "screen/$screenName")
                    values["screen/$screenName/state/$name"] = value
                    initialLocals[name] = value
                }
            }
        }
        navigationEpoch++
        focusedFieldKey = focusBindings.keys.firstOrNull()
        moduleRevision++
        appLifecycleEpoch++
    }

    private fun collectFocusBindings(
        nodes: JSONArray,
        scope: String,
        bindings: MutableMap<String, Pair<String, String?>>,
    ) {
        for (index in 0 until nodes.length()) {
            val raw = nodes.opt(index)
            val node = nexaDevNodeObject(raw)
            val textInput = node.optJSONObject("TextInput")
            if (textInput != null) {
                val stateName = textInput.optString("state").takeIf(String::isNotEmpty)
                val key = "$scope/input/${stateName ?: index}"
                bindings[key] = scope to stateName
            }
            val children = node.optJSONObject("Layout")?.optJSONArray("children")
                ?: node.optJSONObject("If")?.optJSONArray("then_body")
                ?: node.optJSONObject("If")?.optJSONArray("else_body")
                ?: node.optJSONObject("BottomSheet")?.optJSONArray("children")
                ?: node.optJSONObject("Dialog")?.optJSONArray("children")
                ?: node.optJSONObject("KeyboardAware")?.optJSONArray("children")
                ?: node.optJSONObject("Accessibility")?.optJSONArray("children")
                ?: node.optJSONObject("Pressable")?.optJSONArray("children")
            if (children != null) collectFocusBindings(children, scope, bindings)
            val whenCases = node.optJSONObject("When")?.optJSONArray("cases")
            if (whenCases != null) {
                for (caseIndex in 0 until whenCases.length()) {
                    val caseBody = whenCases.optJSONObject(caseIndex)?.optJSONArray("body") ?: continue
                    collectFocusBindings(caseBody, scope, bindings)
                }
            }
        }
    }

    fun focusChanged(nextIdentity: String?) {
        focusedFieldKey = nextIdentity
    }

    fun applyPatch(patch: JSONObject) {
        val root = module ?: return
        val operations = patch.optJSONArray("operations") ?: JSONArray()
        var updated = JSONObject(root.toString())
        for (index in 0 until operations.length()) {
            val operation = operations.optJSONObject(index) ?: continue
            if (!applyOperation(updated, operation)) return
        }
        install(updated)
    }

    private fun applyOperation(root: JSONObject, operation: JSONObject): Boolean {
        val path = operation.optString("path")
        val segments = path.split('/').filter(String::isNotEmpty).map {
            it.replace("~1", "/").replace("~0", "~")
        }
        if (segments.isEmpty()) return false
        val kind = operation.optString("op")
        val value = operation.opt("value")
        return updateContainer(root, segments, 0, kind, value)
    }

    private fun updateContainer(
        container: Any,
        segments: List<String>,
        index: Int,
        kind: String,
        value: Any?,
    ): Boolean {
        val key = segments[index]
        val isLeaf = index == segments.size - 1
        if (container is JSONObject) {
            if (isLeaf) {
                when (kind) {
                    "set" -> container.put(key, value ?: JSONObject.NULL)
                    "remove" -> container.remove(key)
                    else -> return false
                }
                return true
            }
            val child = container.opt(key) ?: return false
            return updateContainer(child, segments, index + 1, kind, value)
        }
        if (container is JSONArray) {
            val arrayIndex = key.toIntOrNull() ?: return false
            if (arrayIndex !in 0 until container.length()) return false
            if (isLeaf) {
                if (kind == "set") {
                    container.put(arrayIndex, value ?: JSONObject.NULL)
                    return true
                }
                return false
            }
            val child = container.opt(arrayIndex) ?: return false
            return updateContainer(child, segments, index + 1, kind, value)
        }
        return false
    }

    fun publishDiagnostics(next: List<String>) {
        diagnostics = next
    }

    fun state(name: String, scope: String = "app"): Any =
        values["$scope/state/$name"] ?: values["app/state/$name"] ?: JSONObject.NULL

    fun setState(name: String, value: Any, scope: String = "app") {
        val target = if (values.containsKey("$scope/state/$name") || !scope.startsWith("screen/")) {
            "$scope/state/$name"
        } else {
            "app/state/$name"
        }
        values[target] = value
    }

    fun locals(scope: String, parameters: Map<String, Any>): Map<String, Any> {
        return parameters
    }

    fun evaluate(raw: Any?, locals: Map<String, Any>, scope: String = "app"): Any {
        val expression = when (raw) {
            is JSONObject -> raw
            is String -> return when (raw) {
                "IsRegularWidth", "IsRegularHeight" -> true
                "IsCompactWidth", "IsCompactHeight" -> false
                else -> raw
            }
            else -> return raw ?: JSONObject.NULL
        }
        val iterator = expression.keys()
        if (!iterator.hasNext()) return JSONObject.NULL
        val kind = iterator.next()
        val payload = expression.opt(kind)
        return when (kind) {
            "String" -> payload as? String ?: ""
            "Bool" -> payload as? Boolean ?: false
            "Number" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val rawValue = fields.optString("raw", payload?.toString() ?: "0")
                when (fields.optString("ty")) {
                    "Int8" -> rawValue.toByteOrNull() ?: 0
                    "Int16" -> rawValue.toShortOrNull() ?: 0
                    "Int32" -> rawValue.toIntOrNull() ?: 0
                    "Int64" -> rawValue.toLongOrNull() ?: 0L
                    "UInt8" -> rawValue.toUByteOrNull() ?: 0u.toUByte()
                    "UInt16" -> rawValue.toUShortOrNull() ?: 0u.toUShort()
                    "UInt32" -> rawValue.toUIntOrNull() ?: 0u
                    "UInt64" -> rawValue.toULongOrNull() ?: 0uL
                    "Float32" -> rawValue.toFloatOrNull() ?: 0f
                    "Float64" -> rawValue.toDoubleOrNull() ?: 0.0
                    else -> if (rawValue.contains('.')) rawValue.toDoubleOrNull() ?: 0.0 else rawValue.toLongOrNull() ?: 0L
                }
            }
            "EnumValue", "PluginEnumValue" -> (payload as? JSONObject)?.optString("case_name") ?: ""
            "Array" -> {
                val array = payload as? JSONArray ?: JSONArray()
                (0 until array.length()).map { evaluate(array.opt(it), locals, scope) }
            }
            "Set" -> {
                val array = payload as? JSONArray ?: JSONArray()
                (0 until array.length()).map { evaluate(array.opt(it), locals, scope) }.toSet()
            }
            "Map" -> {
                val array = payload as? JSONArray ?: JSONArray()
                val result = mutableMapOf<String, Any>()
                for (index in 0 until array.length()) {
                    val pair = array.optJSONArray(index) ?: continue
                    val key = stringify(evaluate(pair.opt(0), locals, scope))
                    result[key] = evaluate(pair.opt(1), locals, scope)
                }
                result
            }
            "Pair" -> {
                val array = payload as? JSONArray ?: JSONArray()
                listOf(evaluate(array.opt(0), locals, scope), evaluate(array.opt(1), locals, scope))
            }
            "Triple" -> {
                val array = payload as? JSONArray ?: JSONArray()
                listOf(
                    evaluate(array.opt(0), locals, scope),
                    evaluate(array.opt(1), locals, scope),
                    evaluate(array.opt(2), locals, scope),
                )
            }
            "State" -> {
                val name = (payload as? JSONArray)?.optString(0) ?: payload?.toString() ?: ""
                locals[name] ?: state(name, scope)
            }
            "Interpolation" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                buildString {
                    for (index in 0 until parts.length()) {
                        val part = parts.optJSONObject(index) ?: continue
                        when {
                            part.has("Literal") -> append(part.optString("Literal"))
                            part.has("Value") -> append(stringify(evaluate(part.opt("Value"), locals, scope)))
                        }
                    }
                }
            }
            "Add" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                val left = evaluate(parts.opt(0), locals, scope)
                val right = evaluate(parts.opt(1), locals, scope)
                if (left is String || right is String) {
                    stringify(left) + stringify(right)
                } else {
                    val sum = number(left) + number(right)
                    if (sum == sum.toLong().toDouble()) sum.toLong() else sum
                }
            }
            "Concat" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                stringify(evaluate(parts.opt(0), locals, scope)) +
                    stringify(evaluate(parts.opt(1), locals, scope))
            }
            "Arithmetic" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                arithmetic(
                    fields.optString("op"),
                    evaluate(fields.opt("left"), locals, scope),
                    evaluate(fields.opt("right"), locals, scope),
                    fields.optString("ty", "Int32"),
                )
            }
            "Negate" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val type = fields.optString("ty", "Int32")
                arithmetic("Subtract", numericZero(type), evaluate(fields.opt("value"), locals, scope), type)
            }
            "Not" -> !truthy(evaluate(payload, locals, scope))
            "Binary" -> {
                val binary = payload as? JSONObject ?: JSONObject()
                val op = binary.optString("op")
                val left = evaluate(binary.opt("left"), locals, scope)
                val right = evaluate(binary.opt("right"), locals, scope)
                compare(op, left, right)
            }
            "Conditional" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val condition = evaluate(fields.opt("condition"), locals, scope)
                val branch = if (truthy(condition)) fields.opt("then_value") else fields.opt("else_value")
                evaluate(branch, locals, scope)
            }
            "Contains" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val item = evaluate(fields.opt("value"), locals, scope)
                val collection = evaluate(fields.opt("collection"), locals, scope)
                contains(item, collection)
            }
            "CollectionTransform" -> {
                val transform = payload as? JSONObject ?: JSONObject()
                val collection = evaluate(transform.opt("collection"), locals, scope)
                val items = when (collection) {
                    is List<*> -> collection
                    is Set<*> -> collection.toList()
                    else -> emptyList<Any?>()
                }
                val closure = transform.optJSONObject("closure")?.optJSONObject("Closure")
                val params = closure?.optJSONArray("parameters") ?: JSONArray()
                val body = closure?.opt("body")
                fun applyClosure(item: Any?, accumulator: Any? = null): Any {
                    val closureLocals = locals.toMutableMap()
                    if (params.length() > 1 && accumulator != null) {
                        params.optString(0)?.let { closureLocals[it] = accumulator }
                        params.optString(1)?.let { closureLocals[it] = item ?: JSONObject.NULL }
                    } else if (params.length() > 0) {
                        params.optString(0)?.let { closureLocals[it] = item ?: JSONObject.NULL }
                    }
                    return evaluate(body, closureLocals, scope)
                }
                when (transform.optString("operation")) {
                    "Map" -> items.map { applyClosure(it) }
                    "Filter" -> items.filter { truthy(applyClosure(it)) }
                    "Reduce" -> {
                        var accumulator = evaluate(transform.opt("initial"), locals, scope)
                        for (item in items) accumulator = applyClosure(item, accumulator)
                        accumulator
                    }
                    else -> items
                }
            }
            "CollectionUtility" -> {
                val utility = payload as? JSONObject ?: JSONObject()
                val collection = evaluate(utility.opt("collection"), locals, scope)
                val items = collection as? List<*> ?: emptyList<Any?>()
                when (utility.optString("operation")) {
                    "Random" -> items.randomOrNull() ?: JSONObject.NULL
                    "First" -> items.firstOrNull() ?: JSONObject.NULL
                    "Last" -> items.lastOrNull() ?: JSONObject.NULL
                    "Shuffled" -> items.shuffled()
                    "Reverse" -> items.reversed()
                    "Slice" -> {
                        val start = number(evaluate(utility.opt("start"), locals, scope)).toInt()
                        val end = number(evaluate(utility.opt("end"), locals, scope)).toInt()
                        val stop = end + if (utility.optBoolean("inclusive")) 1 else 0
                        items.slice(start until stop)
                    }
                    else -> emptyList<Any?>()
                }
            }
            "Closure" -> payload ?: JSONObject.NULL
            "Index" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val collection = evaluate(fields.opt("collection"), locals, scope)
                val indexValue = evaluate(fields.opt("index"), locals, scope)
                when (collection) {
                    is List<*> -> {
                        val index = (indexValue as? Number)?.toInt() ?: -1
                        collection.getOrNull(index) ?: JSONObject.NULL
                    }
                    is Map<*, *> -> collection[stringify(indexValue)] ?: JSONObject.NULL
                    else -> JSONObject.NULL
                }
            }
            "Member" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val base = evaluate(fields.opt("base"), locals, scope)
                val name = fields.optString("name")
                val property = fields.optJSONObject("kind")?.optString("PluginField")
                if (!property.isNullOrEmpty()) {
                    val result = NexaDevPluginBridge.readInstanceProperty(base, property)
                    if (result.first) return result.second
                }
                when (name) {
                    "count" -> when (base) {
                        is Collection<*> -> base.size
                        is Map<*, *> -> base.size
                        is JSONObject -> base.length()
                        else -> JSONObject.NULL
                    }
                    "isEmpty" -> when (base) {
                        is Collection<*> -> base.isEmpty()
                        is Map<*, *> -> base.isEmpty()
                        is JSONObject -> base.length() == 0
                        else -> JSONObject.NULL
                    }
                    else -> when (base) {
                        is Map<*, *> -> base[name] ?: JSONObject.NULL
                        is JSONObject -> base.opt(name) ?: JSONObject.NULL
                        is List<*> -> when (name) {
                            "first" -> base.getOrNull(0) ?: JSONObject.NULL
                            "second" -> base.getOrNull(1) ?: JSONObject.NULL
                            "third" -> base.getOrNull(2) ?: JSONObject.NULL
                            else -> base.getOrNull(name.toIntOrNull() ?: -1) ?: JSONObject.NULL
                        }
                        else -> JSONObject.NULL
                    }
                }
            }
            "Range" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val start = number(evaluate(fields.opt("start"), locals, scope)).toLong()
                val end = number(evaluate(fields.opt("end"), locals, scope)).toLong()
                val step = maxOf(1L, kotlin.math.abs(number(evaluate(fields.opt("step"), locals, scope)).toLong()))
                val inclusive = fields.optBoolean("inclusive", false)
                val boundary = if (inclusive && end >= start) end + 1 else end
                (start until boundary step step).map { it.toDouble() }
            }
            "Null" -> JSONObject.NULL
            // A byte buffer crosses the hot-reload boundary as base64 text,
            // which is the one representation both dev runtimes agree on.
            "BytesFromText" -> {
                val text = evaluate((payload as? JSONObject)?.opt("text"), locals, scope)
                (text as? String ?: "").toByteArray(Charsets.UTF_8).toBase64()
            }
            "BytesFromArray" -> {
                val values = evaluate((payload as? JSONObject)?.opt("values"), locals, scope) as? JSONArray ?: JSONArray()
                val bytes = ByteArray(values.length()) { index ->
                    number(values.opt(index)).toInt().toByte()
                }
                bytes.toBase64()
            }
            "BytesCount" -> {
                val bytes = evaluate((payload as? JSONObject)?.opt("bytes"), locals, scope)
                val decoded = (bytes as? String ?: "").fromBase64()
                decoded.size
            }
            "TimeCall" -> evaluateTimeCall(payload as? JSONObject, locals, scope)
            "LogCall" -> {
                val logCall = payload as? JSONObject ?: JSONObject()
                val message = stringify(evaluate(logCall.opt("message"), locals, scope))
                when (logCall.optString("method")) {
                    "Info" -> android.util.Log.i("Nexa", message)
                    "Warning" -> android.util.Log.w("Nexa", message)
                    "Error" -> android.util.Log.e("Nexa", message)
                }
                JSONObject.NULL
            }
            "Coalesce" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                val first = evaluate(parts.opt(0), locals, scope)
                if (first == JSONObject.NULL || first == null) evaluate(parts.opt(1), locals, scope) else first
            }
            "ResultOk" -> mapOf("Ok" to evaluate((payload as? JSONObject)?.opt("value"), locals, scope))
            "ResultErr" -> mapOf("Err" to evaluate((payload as? JSONObject)?.opt("error"), locals, scope))
            "Try" -> {
                val result = evaluate((payload as? JSONObject)?.opt("expr"), locals, scope)
                (result as? Map<*, *>)?.get("Ok") ?: JSONObject.NULL
            }
            "Call" -> invokeFunction(payload as? JSONObject ?: JSONObject(), locals, scope)
            "NativeCall" -> invokeNativeSync(payload as? JSONObject ?: JSONObject(), locals, scope)
            "PathJoin" -> {
                val join = payload as? JSONObject ?: JSONObject()
                invokeNativeSync(
                    nativeCall(
                        "Path",
                        "join",
                        JSONArray().put(argument("path", join.opt("path"))).put(argument("component", join.opt("component"))),
                    ),
                    locals,
                    scope,
                )
            }
            "FileExists" -> {
                val file = payload as? JSONObject ?: JSONObject()
                invokeNativeSync(
                    nativeCall("File", "exists", JSONArray().put(argument("path", file.opt("path")))),
                    locals,
                    scope,
                )
            }
            else -> JSONObject.NULL
        }
    }

    suspend fun evaluateAsync(raw: Any?, locals: Map<String, Any>, scope: String): Any {
        val expression = when (raw) {
            is JSONObject -> raw
            is String -> return evaluate(raw, locals, scope)
            else -> return raw ?: JSONObject.NULL
        }
        val iterator = expression.keys()
        if (!iterator.hasNext()) return JSONObject.NULL
        val kind = iterator.next()
        val payload = expression.opt(kind)
        return when (kind) {
            "EnumValue", "PluginEnumValue" -> (payload as? JSONObject)?.optString("case_name") ?: ""
            "Await", "TryAwait" -> evaluateAsync(payload, locals, scope)
            "TimeCall" -> {
                val call = payload as? JSONObject ?: JSONObject()
                val method = call.optString("method")
                val rawArguments = call.optJSONArray("arguments") ?: JSONArray()
                val arguments = ArrayList<Any>(rawArguments.length())
                for (index in 0 until rawArguments.length()) {
                    arguments += evaluateAsync(rawArguments.opt(index), locals, scope)
                }
                val first = arguments.firstOrNull()
                when (method) {
                    "Now" -> System.currentTimeMillis()
                    "Monotonic" -> System.nanoTime()
                    "Elapsed" -> System.nanoTime() - ((first as? Number)?.toLong() ?: 0L)
                    "Sleep" -> {
                        kotlinx.coroutines.delay((first as? Number)?.toLong() ?: 0L)
                        JSONObject.NULL
                    }
                    "Iso8601" -> (first as? Number)?.toLong()?.let { nexaDevIso8601(it) } ?: JSONObject.NULL
                    "Iso8601ToMillis" -> (first as? String)?.let { nexaDevIso8601ToMillis(it) } ?: JSONObject.NULL
                    else -> JSONObject.NULL
                }
            }
            "Call" -> invokeFunctionAsync(payload as? JSONObject ?: JSONObject(), locals, scope)
            "LogCall" -> {
                val call = payload as? JSONObject ?: JSONObject()
                val message = stringify(evaluateAsync(call.opt("message"), locals, scope))
                val severity = call.optString("method", "Info")
                when (severity) {
                    "Debug" -> android.util.Log.d("Nexa", message)
                    "Warning" -> android.util.Log.w("Nexa", message)
                    "Error" -> android.util.Log.e("Nexa", message)
                    else -> android.util.Log.i("Nexa", message)
                }
                JSONObject.NULL
            }
            "NativeCall" -> invokeNativeAsync(payload as? JSONObject ?: JSONObject(), locals, scope)
            "NetworkFetch", "NetworkDownload" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val request = if (kind == "NetworkDownload") fields.optJSONObject("request") ?: JSONObject() else fields
                val name = if (kind == "NetworkDownload") "download" else "fetch"
                val arguments = JSONArray()
                    .put(argument("url", request.opt("url")))
                    .put(argument("method", request.opt("method")))
                    .put(argument("headers", request.opt("headers")))
                    .put(argument("timeout", request.opt("timeout")))
                    .put(argument("useCache", request.opt("use_cache")))
                    .put(argument("followRedirects", request.opt("follow_redirects")))
                    .put(argument("maxResponseBytes", request.opt("max_response_bytes")))
                    .put(argument("certificatePins", request.opt("certificate_pins")))
                if (kind == "NetworkDownload") arguments.put(argument("destinationPath", fields.opt("destination")))
                arguments.put(argument("body", request.opt("body")))
                invokeNativeAsync(nativeCall("Network", name, arguments), locals, scope)
            }
            "PathJoin" -> {
                val join = payload as? JSONObject ?: JSONObject()
                invokeNativeAsync(
                    nativeCall(
                        "Path",
                        "join",
                        JSONArray().put(argument("path", join.opt("path"))).put(argument("component", join.opt("component"))),
                    ),
                    locals,
                    scope,
                )
            }
            "FileExists", "FileReadText", "FileWriteText", "FileDelete" -> {
                val file = payload as? JSONObject ?: JSONObject()
                val name = when (kind) {
                    "FileExists" -> "exists"
                    "FileReadText" -> "readText"
                    "FileWriteText" -> "writeText"
                    else -> "delete"
                }
                val arguments = JSONArray().put(argument("path", file.opt("path")))
                if (kind == "FileWriteText") arguments.put(argument("contents", file.opt("contents")))
                invokeNativeAsync(nativeCall("File", name, arguments), locals, scope)
            }
            "PermissionOp" -> {
                val operation = payload as? JSONObject ?: JSONObject()
                val name = when (operation.optString("op")) {
                    "Request" -> "request"
                    "Status" -> "status"
                    else -> error("Unsupported permission operation")
                }
                invokeNativeAsync(
                    nativeCall("Permissions", name, JSONArray().put(argument("permission", operation.opt("permission")))),
                    locals,
                    scope,
                )
            }
            "Member" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val base = evaluateAsync(fields.opt("base"), locals, scope)
                val memberName = fields.optString("name")
                val property = fields.optJSONObject("kind")?.optString("PluginField")
                if (!property.isNullOrEmpty()) {
                    val result = NexaDevPluginBridge.readInstanceProperty(base, property)
                    if (result.first) return result.second
                }
                when (memberName) {
                    "count" -> when (base) {
                        is Collection<*> -> base.size
                        is Map<*, *> -> base.size
                        is JSONObject -> base.length()
                        else -> JSONObject.NULL
                    }
                    "isEmpty" -> when (base) {
                        is Collection<*> -> base.isEmpty()
                        is Map<*, *> -> base.isEmpty()
                        is JSONObject -> base.length() == 0
                        else -> JSONObject.NULL
                    }
                    else -> when (base) {
                        is Map<*, *> -> base[memberName] ?: JSONObject.NULL
                        is JSONObject -> base.opt(memberName) ?: JSONObject.NULL
                        is List<*> -> when (memberName) {
                            "first" -> base.getOrNull(0) ?: JSONObject.NULL
                            "second" -> base.getOrNull(1) ?: JSONObject.NULL
                            "third" -> base.getOrNull(2) ?: JSONObject.NULL
                            else -> base.getOrNull(memberName.toIntOrNull() ?: -1) ?: JSONObject.NULL
                        }
                        else -> JSONObject.NULL
                    }
                }
            }
            "Array", "Set" -> {
                val array = payload as? JSONArray ?: JSONArray()
                val list = (0 until array.length()).map { evaluateAsync(array.opt(it), locals, scope) }
                if (kind == "Set") list.toSet() else list
            }
            "Pair", "Triple" -> {
                val array = payload as? JSONArray ?: JSONArray()
                val required = if (kind == "Pair") 2 else 3
                val values = ArrayList<Any>(required)
                for (index in 0 until required) {
                    values += if (index < array.length()) evaluateAsync(array.opt(index), locals, scope) else JSONObject.NULL
                }
                values
            }
            "Map" -> {
                val array = payload as? JSONArray ?: JSONArray()
                val result = mutableMapOf<String, Any>()
                for (index in 0 until array.length()) {
                    val pair = array.optJSONArray(index) ?: continue
                    val key = stringify(evaluateAsync(pair.opt(0), locals, scope))
                    result[key] = evaluateAsync(pair.opt(1), locals, scope)
                }
                result
            }
            "Add" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                val left = evaluateAsync(parts.opt(0), locals, scope)
                val right = evaluateAsync(parts.opt(1), locals, scope)
                if (left is String || right is String) {
                    stringify(left) + stringify(right)
                } else {
                    val sum = number(left) + number(right)
                    if (sum == sum.toLong().toDouble()) sum.toLong() else sum
                }
            }
            "Concat" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                stringify(evaluateAsync(parts.opt(0), locals, scope)) +
                    stringify(evaluateAsync(parts.opt(1), locals, scope))
            }
            "Arithmetic" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                arithmetic(
                    fields.optString("op"),
                    evaluateAsync(fields.opt("left"), locals, scope),
                    evaluateAsync(fields.opt("right"), locals, scope),
                    fields.optString("ty", "Int32"),
                )
            }
            "Negate" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val type = fields.optString("ty", "Int32")
                arithmetic("Subtract", numericZero(type), evaluateAsync(fields.opt("value"), locals, scope), type)
            }
            "Not" -> !truthy(evaluateAsync(payload, locals, scope))
            "Binary" -> {
                val binary = payload as? JSONObject ?: JSONObject()
                val op = binary.optString("op")
                val left = evaluateAsync(binary.opt("left"), locals, scope)
                if (op == "And" && !truthy(left)) return false
                if (op == "Or" && truthy(left)) return true
                val right = evaluateAsync(binary.opt("right"), locals, scope)
                compare(op, left, right)
            }
            "Interpolation" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                buildString {
                    for (index in 0 until parts.length()) {
                        val part = parts.optJSONObject(index) ?: continue
                        when {
                            part.has("Literal") -> append(part.optString("Literal"))
                            part.has("Value") -> append(stringify(evaluateAsync(part.opt("Value"), locals, scope)))
                        }
                    }
                }
            }
            "Index" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val collection = evaluateAsync(fields.opt("collection"), locals, scope)
                val indexValue = evaluateAsync(fields.opt("index"), locals, scope)
                when (collection) {
                    is List<*> -> collection.getOrNull((indexValue as? Number)?.toInt() ?: -1) ?: JSONObject.NULL
                    is Map<*, *> -> collection[stringify(indexValue)] ?: JSONObject.NULL
                    else -> JSONObject.NULL
                }
            }
            "Range" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val start = number(evaluateAsync(fields.opt("start"), locals, scope)).toInt()
                val end = number(evaluateAsync(fields.opt("end"), locals, scope)).toInt()
                val stepValue = fields.opt("step")?.let { number(evaluateAsync(it, locals, scope)).toInt() } ?: 1
                val step = maxOf(1, kotlin.math.abs(stepValue))
                if (kotlin.math.abs(end.toLong() - start.toLong()) >= 100_000L) return emptyList<Int>()
                val inclusive = fields.optBoolean("inclusive")
                val result = ArrayList<Int>()
                var current = start
                val ascending = start <= end
                while (if (ascending) current < end || (inclusive && current == end) else current > end) {
                    result += current
                    current = if (ascending) current + step else current - step
                    if (result.size >= 100_000) break
                }
                result
            }
            "Contains" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val value = evaluateAsync(fields.opt("value"), locals, scope)
                val collection = evaluateAsync(fields.opt("collection"), locals, scope)
                contains(value, collection)
            }
            "CollectionTransform" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val rawCollection = evaluateAsync(fields.opt("collection"), locals, scope)
                val items = when (rawCollection) {
                    is List<*> -> rawCollection
                    is Set<*> -> rawCollection.toList()
                    else -> emptyList<Any?>()
                }
                val closure = fields.optJSONObject("closure")?.optJSONObject("Closure") ?: JSONObject()
                val parameters = closure.optJSONArray("parameters") ?: JSONArray()
                val body = closure.opt("body")
                suspend fun apply(item: Any?, accumulator: Any? = null): Any {
                    val closureLocals = locals.toMutableMap()
                    if (parameters.length() > 1 && accumulator != null) {
                        closureLocals[parameters.optString(0)] = accumulator
                        closureLocals[parameters.optString(1)] = item ?: JSONObject.NULL
                    } else if (parameters.length() > 0) {
                        closureLocals[parameters.optString(0)] = item ?: JSONObject.NULL
                    }
                    return evaluateAsync(body, closureLocals, scope)
                }
                when (fields.optString("operation")) {
                    "Map" -> items.map { apply(it) }
                    "Filter" -> {
                        val filtered = ArrayList<Any?>()
                        for (item in items) if (truthy(apply(item))) filtered += item
                        filtered
                    }
                    "Reduce" -> {
                        var accumulator = evaluateAsync(fields.opt("initial"), locals, scope)
                        for (item in items) accumulator = apply(item, accumulator)
                        accumulator
                    }
                    else -> items
                }
            }
            "CollectionUtility" -> {
                val utility = payload as? JSONObject ?: JSONObject()
                val values = evaluateAsync(utility.opt("collection"), locals, scope) as? List<*> ?: return emptyList<Any>()
                when (utility.optString("operation")) {
                    "Random" -> values.randomOrNull() ?: JSONObject.NULL
                    "First" -> values.firstOrNull() ?: JSONObject.NULL
                    "Last" -> values.lastOrNull() ?: JSONObject.NULL
                    "Shuffled" -> values.shuffled()
                    "Reverse" -> values.reversed()
                    "Slice" -> {
                        val start = number(evaluateAsync(utility.opt("start"), locals, scope)).toInt().coerceIn(0, values.size)
                        val end = number(evaluateAsync(utility.opt("end"), locals, scope)).toInt()
                        val stop = (end + if (utility.optBoolean("inclusive")) 1 else 0).coerceIn(start, values.size)
                        values.subList(start, stop)
                    }
                    else -> emptyList<Any>()
                }
            }
            "BytesFromText" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                (evaluateAsync(fields.opt("text"), locals, scope) as? String ?: "").toByteArray().toBase64()
            }
            "BytesFromArray" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val values = evaluateAsync(fields.opt("values"), locals, scope) as? List<*> ?: emptyList<Any>()
                values.map { stringify(it ?: JSONObject.NULL).toIntOrNull()?.toByte() ?: 0 }.toByteArray().toBase64()
            }
            "BytesCount" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val rawBytes = evaluateAsync(fields.opt("bytes"), locals, scope) as? String
                rawBytes?.let { android.util.Base64.decode(it, android.util.Base64.DEFAULT).size } ?: 0
            }
            "Coalesce" -> {
                val parts = payload as? JSONArray ?: JSONArray()
                val value = evaluateAsync(parts.opt(0), locals, scope)
                if (value == JSONObject.NULL) evaluateAsync(parts.opt(1), locals, scope) else value
            }
            "ResultOk", "ResultErr" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val key = if (kind == "ResultOk") "value" else "error"
                mapOf((if (kind == "ResultOk") "Ok" else "Err") to evaluateAsync(fields.opt(key), locals, scope))
            }
            "Try" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                (evaluateAsync(fields.opt("expr"), locals, scope) as? Map<*, *>)?.get("Ok") ?: JSONObject.NULL
            }
            "IsRegularWidth", "IsCompactWidth", "IsRegularHeight", "IsCompactHeight" -> evaluate(raw, locals, scope)
            "Conditional" -> {
                val fields = payload as? JSONObject ?: JSONObject()
                val condition = evaluateAsync(fields.opt("condition"), locals, scope)
                val branch = if (truthy(condition)) fields.opt("then_value") else fields.opt("else_value")
                evaluateAsync(branch, locals, scope)
            }
            else -> evaluate(raw, locals, scope)
        }
    }

    suspend fun invokeFunctionAsync(call: JSONObject, locals: Map<String, Any>, scope: String): Any {
        if (call.optBoolean("is_constructor")) {
            val pluginType = call.optJSONObject("return_type")?.optJSONObject("Plugin")
            if (pluginType != null) {
                val namespace = pluginType.optString("namespace")
                val className = pluginType.optString("name")
                val rawArguments = call.optJSONArray("arguments") ?: JSONArray()
                val arguments = ArrayList<Any>(rawArguments.length())
                for (index in 0 until rawArguments.length()) {
                    arguments += evaluateAsync(rawArguments.opt(index), locals, scope)
                }
                val result = NexaDevPluginBridge.construct(namespace, className, arguments)
                if (result.first) return result.second
            }
            val name = call.optString("name")
            val fields = structs[name]
            if (fields != null) {
                val arguments = call.optJSONArray("arguments") ?: JSONArray()
                if (fields.length() != arguments.length()) return JSONObject.NULL
                val instance = linkedMapOf<String, Any>()
                for (index in 0 until fields.length()) {
                    val fieldName = fields.optJSONObject(index)?.optString("name") ?: continue
                    instance[fieldName] = evaluateAsync(arguments.opt(index), locals, scope)
                }
                return instance
            }
        }
        val name = call.optString("name")
        val declaration = functions[name] ?: return JSONObject.NULL
        if (!activeFunctions.add(name)) return JSONObject.NULL
        return try {
            val parameters = declaration.optJSONArray("parameters") ?: JSONArray()
            val arguments = call.optJSONArray("arguments") ?: JSONArray()
            val callLocals = locals.toMutableMap()
            for (index in 0 until parameters.length()) {
                val parameterName = parameters.optJSONObject(index)?.optString("name") ?: continue
                callLocals[parameterName] = evaluateAsync(arguments.opt(index), callLocals, scope)
            }
            val functionLocals = declaration.optJSONArray("locals") ?: JSONArray()
            for (index in 0 until functionLocals.length()) {
                val local = functionLocals.optJSONObject(index) ?: continue
                val localName = local.optString("name")
                if (local.has("initial")) {
                    callLocals[localName] = evaluateAsync(local.get("initial"), callLocals, scope)
                }
            }
            evaluateAsync(declaration.opt("body"), callLocals, scope)
        } finally {
            activeFunctions.remove(name)
        }
    }

    private fun invokeFunction(call: JSONObject, locals: Map<String, Any>, scope: String): Any {
        val name = call.optString("name")
        if (call.optBoolean("is_constructor")) {
            val pluginType = call.optJSONObject("return_type")?.optJSONObject("Plugin")
            if (pluginType != null) {
                val namespace = pluginType.optString("namespace")
                val className = pluginType.optString("name")
                val rawArguments = call.optJSONArray("arguments") ?: JSONArray()
                val arguments = (0 until rawArguments.length()).map {
                    evaluate(rawArguments.opt(it), locals, scope)
                }
                val result = NexaDevPluginBridge.construct(namespace, className, arguments)
                if (result.first) return result.second
            }
            val fields = structs[name] ?: return JSONObject.NULL
            val arguments = call.optJSONArray("arguments") ?: JSONArray()
            val result = mutableMapOf<String, Any>()
            for (index in 0 until fields.length()) {
                val fieldName = fields.optJSONObject(index)?.optString("name") ?: continue
                result[fieldName] = evaluate(arguments.opt(index), locals, scope)
            }
            return result
        }
        val declaration = functions[name] ?: return JSONObject.NULL
        if (!activeFunctions.add(name)) return JSONObject.NULL
        return try {
            val parameters = declaration.optJSONArray("parameters") ?: JSONArray()
            val arguments = call.optJSONArray("arguments") ?: JSONArray()
            val callLocals = locals.toMutableMap()
            for (index in 0 until parameters.length()) {
                val parameterName = parameters.optJSONObject(index)?.optString("name") ?: continue
                callLocals[parameterName] = evaluate(arguments.opt(index), callLocals, scope)
            }
            val functionLocals = declaration.optJSONArray("locals") ?: JSONArray()
            for (index in 0 until functionLocals.length()) {
                val local = functionLocals.optJSONObject(index) ?: continue
                val localName = local.optString("name")
                if (local.has("initial")) {
                    callLocals[localName] = evaluate(local.get("initial"), callLocals, scope)
                }
            }
            evaluate(declaration.opt("body"), callLocals, scope)
        } finally {
            activeFunctions.remove(name)
        }
    }

    fun stringify(value: Any): String = when (value) {
        JSONObject.NULL -> "null"
        is Number -> if (value.toDouble() == value.toLong().toDouble()) value.toLong().toString() else value.toString()
        else -> value.toString()
    }

    private fun ByteArray.toBase64(): String =
        android.util.Base64.encodeToString(this, android.util.Base64.NO_WRAP)

    private fun String.fromBase64(): ByteArray =
        android.util.Base64.decode(this, android.util.Base64.NO_WRAP)

    internal fun compare(op: String, left: Any, right: Any): Boolean = when (op) {
        "And" -> truthy(left) && truthy(right)
        "Or" -> truthy(left) || truthy(right)
        "Equal" -> stringify(left) == stringify(right)
        "NotEqual" -> stringify(left) != stringify(right)
        "Less" -> number(left) < number(right)
        "LessEqual" -> number(left) <= number(right)
        "Greater" -> number(left) > number(right)
        "GreaterEqual" -> number(left) >= number(right)
        "Contains" -> contains(left, right)
        else -> false
    }

    internal fun truthy(value: Any?): Boolean = when (value) {
        null, JSONObject.NULL -> false
        is Boolean -> value
        is Number -> value.toDouble() != 0.0
        is String -> value.isNotEmpty()
        else -> true
    }

    internal fun contains(value: Any?, collection: Any?): Boolean = when (collection) {
        is List<*> -> collection.any { stringify(it ?: JSONObject.NULL) == stringify(value ?: JSONObject.NULL) }
        is Set<*> -> collection.any { stringify(it ?: JSONObject.NULL) == stringify(value ?: JSONObject.NULL) }
        is Map<*, *> -> collection.containsKey(stringify(value ?: JSONObject.NULL))
        is String -> collection.contains(stringify(value ?: JSONObject.NULL))
        else -> false
    }

    internal fun number(value: Any): Double = when (value) {
        is Number -> value.toDouble()
        is String -> value.toDoubleOrNull() ?: 0.0
        else -> 0.0
    }

    private fun numericZero(type: String): Any = when {
        type == "Float32" -> 0f
        type == "Float64" -> 0.0
        type.startsWith("UInt") && type != "UInt64" -> 0u
        type == "UInt64" -> 0uL
        else -> 0L
    }

    private fun arithmetic(op: String, left: Any?, right: Any?, type: String): Any {
        if (type == "Float32") {
            val lhs = number(left ?: 0).toFloat()
            val rhs = number(right ?: 0).toFloat()
            return when (op) {
                "Subtract" -> lhs - rhs
                "Multiply" -> lhs * rhs
                "Divide" -> lhs / rhs
                "Remainder" -> lhs % rhs
                else -> 0f
            }
        }
        if (type == "Float64") {
            val lhs = number(left ?: 0)
            val rhs = number(right ?: 0)
            return when (op) {
                "Subtract" -> lhs - rhs
                "Multiply" -> lhs * rhs
                "Divide" -> lhs / rhs
                "Remainder" -> lhs % rhs
                else -> 0.0
            }
        }
        if (type.startsWith("UInt")) {
            val lhs = unsignedInteger(left ?: 0)
            val rhs = unsignedInteger(right ?: 0)
            val result = when (op) {
                "Subtract" -> lhs - rhs
                "Multiply" -> lhs * rhs
                "Divide" -> if (rhs == 0uL) 0uL else lhs / rhs
                "Remainder" -> if (rhs == 0uL) 0uL else lhs % rhs
                else -> 0uL
            }
            return when (type) {
                "UInt8" -> result.toUByte()
                "UInt16" -> result.toUShort()
                "UInt32" -> result.toUInt()
                else -> result
            }
        }
        val lhs = signedInteger(left ?: 0)
        val rhs = signedInteger(right ?: 0)
        val result = when (op) {
            "Subtract" -> lhs - rhs
            "Multiply" -> lhs * rhs
            "Divide" -> if (rhs == 0L) 0L else lhs / rhs
            "Remainder" -> if (rhs == 0L) 0L else lhs % rhs
            else -> 0L
        }
        return when (type) {
            "Int8" -> result.toByte()
            "Int16" -> result.toShort()
            "Int32" -> result.toInt()
            else -> result
        }
    }

    private fun signedInteger(value: Any): Long = when (value) {
        is Byte -> value.toLong()
        is Short -> value.toLong()
        is Int -> value.toLong()
        is Long -> value
        is Number -> value.toLong()
        is String -> value.toLongOrNull() ?: 0L
        else -> 0L
    }

    private fun unsignedInteger(value: Any): ULong = when (value) {
        is UByte -> value.toULong()
        is UShort -> value.toULong()
        is UInt -> value.toULong()
        is ULong -> value
        is Byte -> value.toULong()
        is Short -> value.toULong()
        is Int -> value.toULong()
        is Long -> value.toULong()
        is Number -> value.toLong().toULong()
        is String -> value.toULongOrNull() ?: 0uL
        else -> 0uL
    }
}

/**
 * A core clock call, evaluated the same way the generated code renders it: a
 * system clock, a monotonic counter, or the ISO 8601 helpers the app carries.
 */
private fun NexaDevStateStore.evaluateTimeCall(
    call: JSONObject?,
    locals: Map<String, Any>,
    scope: String,
): Any {
    val method = call?.optString("method") ?: return JSONObject.NULL
    val arguments = call.optJSONArray("arguments") ?: JSONArray()
    val first = evaluate(arguments.opt(0), locals, scope)
    return when (method) {
        "Now" -> System.currentTimeMillis()
        "Monotonic" -> System.nanoTime()
        "Elapsed" -> System.nanoTime() - ((first as? Number)?.toLong() ?: 0L)
        // A synchronous evaluator cannot suspend. The async path handles a real
        // sleep; here the request is reported and evaluation moves on rather than
        // blocking the thread it runs on.
        "Sleep" -> JSONObject.NULL
        "Iso8601" -> (first as? Number)?.toLong()?.let { nexaDevIso8601(it) } ?: JSONObject.NULL
        "Iso8601ToMillis" -> (first as? String)?.let { nexaDevIso8601ToMillis(it) } ?: JSONObject.NULL
        else -> JSONObject.NULL
    }
}

/**
 * The ISO 8601 layout the generated code implements, kept in one place so the
 * two cannot drift.
 */
private const val NEXA_DEV_EPOCH_DAYS = 719_468L
private const val NEXA_DEV_DAYS_IN_400_YEARS = 146_097L

private fun nexaDevIso8601(milliseconds: Long): String {
    var days = Math.floorDiv(milliseconds, 86_400_000L)
    var remainder = Math.floorMod(milliseconds, 86_400_000L)
    if (remainder < 0) {
        remainder += 86_400_000L
        days -= 1
    }
    val (year, month, day) = nexaDevCivilDate(days)
    val hour = remainder / 3_600_000L
    remainder %= 3_600_000L
    val minute = remainder / 60_000L
    remainder %= 60_000L
    val second = remainder / 1000L
    val millisecond = remainder % 1000L
    return String.format(
        java.util.Locale.ROOT,
        "%04d-%02d-%02dT%02d:%02d:%02d.%03dZ",
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond,
    )
}

private fun nexaDevIso8601ToMillis(text: String): Any {
    if (text.length != 20 && text.length != 24) {
        return JSONObject.NULL
    }
    if (text[4] != '-' || text[7] != '-' || text[10] != 'T' ||
        text[13] != ':' || text[16] != ':' || text[text.length - 1] != 'Z'
    ) {
        return JSONObject.NULL
    }
    if (text.length == 24 && text[19] != '.') {
        return JSONObject.NULL
    }
    // Digit runs in layout order. A run is read left to right, so its first
    // digit is the most significant one.
    val runs = listOf(0..3, 5..6, 8..9, 11..12, 14..15, 17..18)
    val values = LongArray(runs.size + 1)
    for (index in runs.indices) {
        val value = nexaDevDigits(text, runs[index]) ?: return JSONObject.NULL
        values[index] = value
    }
    if (text.length == 24) {
        values[6] = nexaDevDigits(text, 20..22) ?: return JSONObject.NULL
    }
    val year = values[0]
    val month = values[1]
    val day = values[2]
    val hour = values[3]
    val minute = values[4]
    val second = values[5]
    if (month < 1 || month > 12 || day < 1) {
        return JSONObject.NULL
    }
    if (day > nexaDevMonthLength(year, month)) {
        return JSONObject.NULL
    }
    if (hour >= 24 || minute >= 60 || second > 60) {
        return JSONObject.NULL
    }
    val millisecond = if (values.size > 6) values[6] else 0L
    if (millisecond >= 1000) {
        return JSONObject.NULL
    }
    val days = nexaDevDaysFromCivil(year, month, day)
    return (((days * 24 + hour) * 60 + minute) * 60 + second) * 1000 + millisecond
}

private fun nexaDevCivilDate(daysSinceEpoch: Long): Triple<Long, Long, Long> {
    val shifted = daysSinceEpoch + NEXA_DEV_EPOCH_DAYS
    val era = if (shifted >= 0) shifted / NEXA_DEV_DAYS_IN_400_YEARS
    else (shifted - (NEXA_DEV_DAYS_IN_400_YEARS - 1)) / NEXA_DEV_DAYS_IN_400_YEARS
    val dayOfEra = shifted - era * NEXA_DEV_DAYS_IN_400_YEARS
    val yearOfEra = (dayOfEra - dayOfEra / 1460 + dayOfEra / 36524 - dayOfEra / 146096) / 365
    val year = yearOfEra + era * 400
    val dayOfYear = dayOfEra - (365 * yearOfEra + yearOfEra / 4 - yearOfEra / 100)
    val monthIndex = (5 * dayOfYear + 2) / 153
    val day = dayOfYear - (153 * monthIndex + 2) / 5 + 1
    val month = monthIndex + if (monthIndex < 10) 3 else -9
    return Triple(if (month <= 2) year + 1 else year, month, day)
}

private fun nexaDevDaysFromCivil(year: Long, month: Long, day: Long): Long {
    val adjustedYear = year - if (month <= 2) 1 else 0
    val era = if (adjustedYear >= 0) adjustedYear / 400 else (adjustedYear - 399) / 400
    val yearOfEra = adjustedYear - era * 400
    val monthIndex = if (month > 2) month - 3 else month + 9
    val dayOfYear = (153 * monthIndex + 2) / 5 + day - 1
    val dayOfEra = yearOfEra * 365 + yearOfEra / 4 - yearOfEra / 100 + dayOfYear
    return era * NEXA_DEV_DAYS_IN_400_YEARS + dayOfEra - NEXA_DEV_EPOCH_DAYS
}

/** A run of ASCII digits as a number, or null when anything else is in it. */
private fun nexaDevDigits(text: String, range: IntRange): Long? {
    var value = 0L
    for (index in range) {
        val digit = text[index]
        if (digit < '0' || digit > '9') {
            return null
        }
        value = value * 10 + (digit - '0').toLong()
    }
    return value
}

/**
 * The number of days in a proleptic Gregorian month. February is the only one that
 * depends on the year, and 31 February has to be rejected rather than rolled
 * forward into March.
 */
fun nexaDevMonthLength(year: Long, month: Long): Long {
    if (month == 2L) {
        val leap = year % 4L == 0L && (year % 100L != 0L || year % 400L == 0L)
        return if (leap) 29L else 28L
    }
    if (month == 4L || month == 6L || month == 9L || month == 11L) {
        return 30L
    }
    return 31L
}
