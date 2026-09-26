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

internal class NexaDevStateStore(internal val context: Context) {
    internal val values = mutableStateMapOf<String, Any>()
    internal var typeSignatures = mutableMapOf<String, String>()
    internal var functions = mutableMapOf<String, JSONObject>()
    internal var structs = mutableMapOf<String, JSONArray>()
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
        private set
    private var navigationRoot: String? = null
    private var navigationScreensSignature: String? = null
    internal var focusBindings = mutableMapOf<String, Pair<String, String?>>()
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
        val nextStructs = next.optJSONArray("structs") ?: JSONArray()
        structs = (0 until nextStructs.length()).mapNotNull { index ->
            val declaration = nextStructs.optJSONObject(index) ?: return@mapNotNull null
            val name = declaration.optString("name").takeIf(String::isNotEmpty) ?: return@mapNotNull null
            name to (declaration.optJSONArray("fields") ?: JSONArray())
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
        for ((scope, declaration) in declarations) {
            val name = declaration.optString("name")
            val identity = "$scope/state/$name"
            val typeSignature = declaration.opt("ty")?.toString() ?: ""
            nextTypes[identity] = typeSignature
            val previousType = typeSignatures[identity]
            val previousValue = values[identity]
            if (previousType == typeSignature && previousValue != null) {
                nextValues[identity] = previousValue
            } else if (declaration.has("initial")) {
                nextValues[identity] = evaluate(declaration.get("initial"), emptyMap(), scope)
            }
        }
        values.clear()
        values.putAll(nextValues)
        typeSignatures = nextTypes
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
        moduleRevision++
        diagnostics = emptyList()
        if (!hasInstalledModule) {
            hasInstalledModule = true
            appLifecycleEpoch++
        }
    }

    fun hotRestart() {
        val current = module ?: return
        val states = current.optJSONArray("states") ?: JSONArray()
        val screens = current.optJSONArray("screens") ?: JSONArray()
        values.clear()
        for (index in 0 until states.length()) {
            val state = states.optJSONObject(index) ?: continue
            val name = state.optString("name")
            if (state.has("initial")) {
                values["app/state/$name"] = evaluate(state.get("initial"), emptyMap(), "app")
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
                    values["screen/$screenName/state/$name"] = evaluate(state.get("initial"), emptyMap(), "screen/$screenName")
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
                val rawValue = (payload as? JSONObject)?.optString("raw")
                    ?: payload?.toString()
                    ?: "0"
                if (rawValue.contains('.')) rawValue.toDoubleOrNull() ?: 0.0 else rawValue.toLongOrNull() ?: 0L
            }
            "EnumValue" -> (payload as? JSONObject)?.optString("case_name") ?: ""
            "Array" -> {
                val array = payload as? JSONArray ?: JSONArray()
                (0 until array.length()).map { evaluate(array.opt(it), locals, scope) }
            }
            "Set" -> {
                val array = payload as? JSONArray ?: JSONArray()
                (0 until array.length()).map { stringify(evaluate(array.opt(it), locals, scope)) }.toSet()
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
            "Not" -> !truthy(evaluate(payload, locals, scope))
            "Binary" -> {
                val binary = payload as? JSONObject ?: JSONObject()
                val op = binary.optString("op")
                val left = evaluate(binary.opt("left"), locals, scope)
                val right = evaluate(binary.opt("right"), locals, scope)
                compare(op, left, right)
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
                when (base) {
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
            "Await", "TryAwait" -> evaluateAsync(payload, locals, scope)
            "Call" -> invokeFunctionAsync(payload as? JSONObject ?: JSONObject(), locals, scope)
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
                when (base) {
                    is Map<*, *> -> base[memberName] ?: JSONObject.NULL
                    is JSONObject -> base.opt(memberName) ?: JSONObject.NULL
                    else -> JSONObject.NULL
                }
            }
            "Array", "Set" -> {
                val array = payload as? JSONArray ?: JSONArray()
                val list = (0 until array.length()).map { evaluateAsync(array.opt(it), locals, scope) }
                if (kind == "Set") list.map { stringify(it) }.toSet() else list
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
            "Not" -> !truthy(evaluateAsync(payload, locals, scope))
            "Binary" -> {
                val binary = payload as? JSONObject ?: JSONObject()
                val op = binary.optString("op")
                val left = evaluateAsync(binary.opt("left"), locals, scope)
                val right = evaluateAsync(binary.opt("right"), locals, scope)
                compare(op, left, right)
            }
            else -> evaluate(raw, locals, scope)
        }
    }

    suspend fun invokeFunctionAsync(call: JSONObject, locals: Map<String, Any>, scope: String): Any {
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
        is Set<*> -> collection.contains(stringify(value ?: JSONObject.NULL))
        is Map<*, *> -> collection.containsKey(stringify(value ?: JSONObject.NULL))
        is String -> collection.contains(stringify(value ?: JSONObject.NULL))
        else -> false
    }

    internal fun number(value: Any): Double = when (value) {
        is Number -> value.toDouble()
        is String -> value.toDoubleOrNull() ?: 0.0
        else -> 0.0
    }
}
