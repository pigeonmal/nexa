package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject

internal object NexaDevNetworkStatus {
    private val lock = Any()
    private val mainHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private val listeners = mutableMapOf<String, (Boolean) -> Unit>()
    private var manager: android.net.ConnectivityManager? = null
    private var callback: android.net.ConnectivityManager.NetworkCallback? = null
    private var lastOnline: Boolean? = null

    fun isOnline(context: android.content.Context): Boolean {
        val connectivity = context.getSystemService(android.content.Context.CONNECTIVITY_SERVICE)
            as? android.net.ConnectivityManager ?: return false
        val network = connectivity.activeNetwork ?: return false
        val capabilities = connectivity.getNetworkCapabilities(network) ?: return false
        return capabilities.hasCapability(android.net.NetworkCapabilities.NET_CAPABILITY_INTERNET)
    }

    fun subscribe(context: android.content.Context, id: String, listener: (Boolean) -> Unit) {
        synchronized(lock) {
            listeners[id] = listener
            if (callback != null) return
            val appContext = context.applicationContext
            val connectivity = appContext.getSystemService(android.content.Context.CONNECTIVITY_SERVICE)
                as? android.net.ConnectivityManager ?: return
            val networkCallback = object : android.net.ConnectivityManager.NetworkCallback() {
                override fun onAvailable(network: android.net.Network) = dispatch(appContext)
                override fun onLost(network: android.net.Network) = dispatch(appContext)
                override fun onCapabilitiesChanged(
                    network: android.net.Network,
                    capabilities: android.net.NetworkCapabilities,
                ) = dispatch(appContext)
            }
            manager = connectivity
            callback = networkCallback
            lastOnline = isOnline(appContext)
            try {
                if (android.os.Build.VERSION.SDK_INT >= 24) {
                    connectivity.registerDefaultNetworkCallback(networkCallback)
                } else {
                    val request = android.net.NetworkRequest.Builder()
                        .addCapability(android.net.NetworkCapabilities.NET_CAPABILITY_INTERNET)
                        .build()
                    connectivity.registerNetworkCallback(request, networkCallback)
                }
            } catch (_: SecurityException) {
                callback = null
                manager = null
                listeners.remove(id)
            }
        }
    }

    fun clear(id: String) {
        synchronized(lock) {
            listeners.remove(id)
            if (listeners.isNotEmpty()) return
            val currentManager = manager
            val currentCallback = callback
            if (currentManager != null && currentCallback != null) {
                runCatching { currentManager.unregisterNetworkCallback(currentCallback) }
            }
            manager = null
            callback = null
            lastOnline = null
        }
    }

    private fun dispatch(context: android.content.Context) {
        val online = isOnline(context)
        val currentListeners = synchronized(lock) {
            if (lastOnline == online) emptyList() else {
                lastOnline = online
                listeners.values.toList()
            }
        }
        if (currentListeners.isEmpty()) return
        mainHandler.post { currentListeners.forEach { it(online) } }
    }
}

private fun nexaDevResponseHeaderValues(response: Any?, name: String): List<String> {
    val fields = response as? Map<*, *> ?: return emptyList()
    val headers = fields["headers"] as? List<*> ?: return emptyList()
    return buildList {
        for (header in headers) {
            val values = header as? Map<*, *> ?: continue
            val headerName = values["name"] as? String ?: continue
            if (!headerName.equals(name, ignoreCase = true)) continue
            val value = values["value"] as? String ?: continue
            add(value)
        }
    }
}

private fun nexaDevStringMap(value: Any?): Map<String, String> {
    val source = value as? Map<*, *> ?: return emptyMap()
    val result = LinkedHashMap<String, String>(source.size)
    for ((key, item) in source) {
        if (key is String && item is String) result[key] = item
    }
    return result
}

private fun nexaDevResponseHeaderValue(response: Any?, name: String): String? {
    val fields = response as? Map<*, *> ?: return null
    val headers = fields["headers"] as? List<*> ?: return null
    for (header in headers) {
        val values = header as? Map<*, *> ?: continue
        val headerName = values["name"] as? String ?: continue
        if (!headerName.equals(name, ignoreCase = true)) continue
        val value = values["value"] as? String ?: continue
        return value
    }
    return null
}

private fun nexaDevStorageMethod(
    context: android.content.Context,
    name: String,
    options: Map<String, Any>,
    codecs: List<Any>,
    enumCases: Map<String, List<String>>,
): Any {
    val key = options["key"] as? String ?: ""
    return when (name) {
        "getString" -> NexaStorage.getString(context, key) ?: JSONObject.NULL
        "setString" -> {
            NexaStorage.setString(context, key, options["value"] as? String ?: "")
            JSONObject.NULL
        }
        "getBool" -> NexaStorage.getBool(context, key) ?: JSONObject.NULL
        "setBool" -> {
            NexaStorage.setBool(context, key, options["value"] as? Boolean ?: false)
            JSONObject.NULL
        }
        "getInt64" -> NexaStorage.getInt64(context, key) ?: JSONObject.NULL
        "setInt64" -> {
            NexaStorage.setInt64(context, key, (options["value"] as? Number)?.toLong() ?: 0L)
            JSONObject.NULL
        }
        "getFloat64" -> NexaStorage.getFloat64(context, key) ?: JSONObject.NULL
        "setFloat64" -> {
            NexaStorage.setFloat64(context, key, (options["value"] as? Number)?.toDouble() ?: 0.0)
            JSONObject.NULL
        }
        "get" -> {
            val codec = codecs.firstOrNull() ?: return JSONObject.NULL
            val raw = NexaStorage.getJson(context, key) ?: return JSONObject.NULL
            NexaDevValueCodec.parseJson(raw, codec, enumCases)["Ok"] ?: JSONObject.NULL
        }
        "set" -> {
            val codec = codecs.firstOrNull() ?: return JSONObject.NULL
            val raw = NexaDevValueCodec.stringifyJson(options["value"], codec, enumCases)
            NexaStorage.setJson(context, key, raw)
            JSONObject.NULL
        }
        "delete" -> {
            NexaStorage.delete(context, key)
            JSONObject.NULL
        }
        "clear" -> {
            NexaStorage.clear(context)
            JSONObject.NULL
        }
        else -> error("Unsupported storage call $name")
    }
}

private fun nexaDevCookieHeader(setCookieValues: List<String>): String =
    setCookieValues.mapNotNull { value ->
        val pair = value.substringBefore(';').trim()
        val equals = pair.indexOf('=')
        if (equals <= 0) null else {
            val name = pair.substring(0, equals).trim()
            val cookieValue = pair.substring(equals + 1).trim()
            if (name.isEmpty()) null else "$name=$cookieValue"
        }
    }.joinToString("; ")

private fun nexaDevNetworkResponseMethod(response: Any?, name: String, headerName: String): Any =
    when (name) {
        "getHeader" -> nexaDevResponseHeaderValue(response, headerName) ?: JSONObject.NULL
        "getCookie" -> nexaDevCookieHeader(nexaDevResponseHeaderValues(response, "Set-Cookie"))
        else -> nexaDevResponseHeaderValues(response, headerName)
    }

private fun nexaDevPerformHaptics(name: String, options: Map<String, Any>) {
    val constant = when (name) {
        "impact" -> when (options["style"] as? String) {
            "Medium" -> android.view.HapticFeedbackConstants.VIRTUAL_KEY
            "Heavy" -> android.view.HapticFeedbackConstants.LONG_PRESS
            else -> android.view.HapticFeedbackConstants.KEYBOARD_TAP
        }
        "notification" -> when (options["kind"] as? String) {
            "Error" -> android.view.HapticFeedbackConstants.REJECT
            else -> android.view.HapticFeedbackConstants.CONFIRM
        }
        "selection" -> android.view.HapticFeedbackConstants.KEYBOARD_TAP
        else -> return
    }
    dev.nexa.core.NexaRuntimeCore.performHapticFeedback(constant)
}

private fun nexaDevFormatCurrency(amount: Double, currencyCode: String): String {
    val locale = java.util.Locale.getDefault()
    val currency = try {
        java.util.Currency.getInstance(currencyCode.uppercase(java.util.Locale.ROOT))
    } catch (_: IllegalArgumentException) {
        return java.text.NumberFormat.getNumberInstance(locale).format(amount)
    }
    val formatter = java.text.NumberFormat.getCurrencyInstance(locale)
    formatter.currency = currency
    return formatter.format(amount)
}

internal fun NexaDevStateStore.nativeCall(namespace: String, name: String, arguments: JSONArray): JSONObject =
    JSONObject()
        .put("namespace", namespace)
        .put("name", name)
        .put("arguments", arguments)

internal fun NexaDevStateStore.argument(name: String, expression: Any?): JSONArray =
    JSONArray().put(name).put(expression ?: JSONObject.NULL)

private fun nexaDevRegexFindAll(pattern: String, text: String): List<Map<String, Any?>> =
    runCatching { Regex(pattern) }.getOrNull()?.findAll(text)?.map { match ->
        val groups = (1 until match.groups.size).map { index -> match.groups[index]?.value }
        mapOf(
            "value" to match.value,
            "range" to mapOf(
                "lowerBound" to match.range.first.toLong(),
                "upperBound" to (match.range.last + 1).toLong(),
            ),
            "groups" to groups,
        )
    }?.toList().orEmpty()

private sealed interface NexaDevSharedNativeResult {
    data class Handled(val value: Any) : NexaDevSharedNativeResult
    data object Unhandled : NexaDevSharedNativeResult
}

private fun NexaDevStateStore.invokeSharedNative(
    namespace: String,
    name: String,
    options: Map<String, Any>,
    codecs: List<Any>,
): NexaDevSharedNativeResult {
    fun stringOption(key: String, fallback: String = ""): String =
        options[key] as? String ?: (options[key] as? CharSequence)?.toString() ?: fallback

    fun numberOption(key: String, fallback: Double = 0.0): Double =
        (options[key] as? Number)?.toDouble() ?: fallback

    return when (namespace) {
        "Network" -> if (name == "isOnline") {
            NexaDevSharedNativeResult.Handled(NexaDevNetworkStatus.isOnline(context))
        } else NexaDevSharedNativeResult.Unhandled
        "Keyboard" -> if (name == "dismiss") {
            dev.nexa.core.NexaRuntimeCore.dismissKeyboard()
            NexaDevSharedNativeResult.Handled(JSONObject.NULL)
        } else NexaDevSharedNativeResult.Unhandled
        "Haptics" -> {
            nexaDevPerformHaptics(name, options)
            NexaDevSharedNativeResult.Handled(JSONObject.NULL)
        }
        "Screen" -> if (name == "lockOrientation") {
            dev.nexa.core.NexaRuntimeCore.lockOrientation(options["mode"] as? String ?: "All")
            NexaDevSharedNativeResult.Handled(JSONObject.NULL)
        } else NexaDevSharedNativeResult.Unhandled
        "Json" -> {
            val codec = codecs.firstOrNull() ?: return NexaDevSharedNativeResult.Unhandled
            when (name) {
                "parse" -> NexaDevSharedNativeResult.Handled(
                    NexaDevValueCodec.parseJson(
                        raw = options["raw"] as? String ?: "",
                        type = codec,
                        enumCases = enumCases,
                    ),
                )
                "stringify" -> NexaDevSharedNativeResult.Handled(
                    NexaDevValueCodec.stringifyJson(
                        raw = options["value"],
                        type = codec,
                        enumCases = enumCases,
                    ),
                )
                else -> NexaDevSharedNativeResult.Unhandled
            }
        }
        "Clipboard" -> {
            val manager = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE)
                as android.content.ClipboardManager
            when (name) {
                "setText" -> {
                    manager.setPrimaryClip(
                        android.content.ClipData.newPlainText(null, options["text"] as? String ?: ""),
                    )
                    NexaDevSharedNativeResult.Handled(JSONObject.NULL)
                }
                "getText" -> {
                    val item = manager.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
                    NexaDevSharedNativeResult.Handled(item?.coerceToText(context)?.toString() ?: JSONObject.NULL)
                }
                "hasText" -> {
                    val description = manager.primaryClipDescription
                    NexaDevSharedNativeResult.Handled(
                        description?.hasMimeType(android.content.ClipDescription.MIMETYPE_TEXT_PLAIN) == true ||
                            description?.hasMimeType(android.content.ClipDescription.MIMETYPE_TEXT_HTML) == true,
                    )
                }
                else -> NexaDevSharedNativeResult.Unhandled
            }
        }
        "Storage" -> NexaDevSharedNativeResult.Handled(
            nexaDevStorageMethod(context, name, options, codecs, enumCases),
        )
        "Number" -> if (name == "formatCurrency") {
            NexaDevSharedNativeResult.Handled(
                nexaDevFormatCurrency(numberOption("amount"), stringOption("currencyCode")),
            )
        } else NexaDevSharedNativeResult.Unhandled
        "Locale" -> {
            val locale = java.util.Locale.getDefault()
            when (name) {
                "currentLanguageCode" -> NexaDevSharedNativeResult.Handled(locale.language)
                "preferredLanguageCodes" -> NexaDevSharedNativeResult.Handled(
                    if (android.os.Build.VERSION.SDK_INT >= 24) {
                        android.os.LocaleList.getDefault().let { locales ->
                            (0 until locales.size()).map { locales[it].toLanguageTag() }
                        }
                    } else listOf(locale.toLanguageTag()),
                )
                "displayName" -> NexaDevSharedNativeResult.Handled(
                    java.util.Locale.forLanguageTag(stringOption("languageCode"))
                        .getDisplayLanguage(locale).takeIf { it.isNotEmpty() } ?: "",
                )
                else -> NexaDevSharedNativeResult.Unhandled
            }
        }
        "Crypto" -> when (name) {
            "sha256" -> NexaDevSharedNativeResult.Handled(nexaCryptoSha256(stringOption("text")))
            "sha512" -> NexaDevSharedNativeResult.Handled(nexaCryptoSha512(stringOption("text")))
            "hmacSha256" -> NexaDevSharedNativeResult.Handled(
                nexaCryptoHmacSha256(stringOption("key"), stringOption("message")),
            )
            "randomBytes" -> NexaDevSharedNativeResult.Handled(
                nexaCryptoRandomBytes(numberOption("count").toInt()),
            )
            else -> NexaDevSharedNativeResult.Unhandled
        }
        "Path" -> when (name) {
            "documents" -> NexaDevSharedNativeResult.Handled(NexaPath.documents(context))
            "caches" -> NexaDevSharedNativeResult.Handled(NexaPath.caches(context))
            "temporary" -> NexaDevSharedNativeResult.Handled(NexaPath.temporary(context))
            "appSupport" -> NexaDevSharedNativeResult.Handled(NexaPath.appSupport(context))
            "join" -> NexaDevSharedNativeResult.Handled(
                NexaPath.join(stringOption("path"), stringOption("component")),
            )
            else -> NexaDevSharedNativeResult.Unhandled
        }
        "File" -> if (name == "exists") {
            NexaDevSharedNativeResult.Handled(NexaFile.exists(stringOption("path")))
        } else NexaDevSharedNativeResult.Unhandled
        else -> NexaDevSharedNativeResult.Unhandled
    }
}

private fun nexaDevComponentRefResult(result: Any): Any = when (result) {
    is dev.nexa.core.NexaResult.Success<*> -> mapOf("Ok" to JSONObject.NULL)
    is dev.nexa.core.NexaResult.Failure<*> -> mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
    else -> mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
}

private fun nexaDevInvokeFastListRef(
    reference: NexaFastListRef,
    name: String,
    options: Map<String, Any>,
): Any {
    val animated = options["animated"] as? Boolean ?: true
    val result = when (name) {
        "scrollToIndex" -> reference.scrollToIndex(
            (options["index"] as? Number)?.toInt() ?: 0,
            animated,
            (options["viewPosition"] as? Number)?.toDouble() ?: 0.0,
        )
        "scrollToTop" -> reference.scrollToTop(animated)
        "scrollToEnd" -> reference.scrollToEnd(animated)
        "scrollToOffset" -> reference.scrollToOffset(
            (options["offset"] as? Number)?.toDouble() ?: 0.0,
            animated,
        )
        else -> return JSONObject.NULL
    }
    return nexaDevComponentRefResult(result)
}

internal suspend fun NexaDevStateStore.invokeNativeAsync(
    call: JSONObject,
    locals: Map<String, Any>,
    scope: String,
): Any {
    val namespace = call.optString("namespace")
    val name = call.optString("name")
    runtimeTrace("native_call $namespace.$name")
    val rawArguments = call.optJSONArray("arguments") ?: JSONArray()
    val codecs = call.optJSONArray("codecs")?.let { values ->
        (0 until values.length()).map { values.opt(it) ?: JSONObject.NULL }
    } ?: emptyList()
    val options = mutableMapOf<String, Any>()
    for (index in 0 until rawArguments.length()) {
        val entry = rawArguments.optJSONArray(index) ?: continue
        val argumentName = entry.optString(0)
        options[argumentName] = evaluateAsync(entry.opt(1), locals, scope)
    }
    fun stringOption(key: String, fallback: String = ""): String =
        options[key] as? String ?: (options[key] as? CharSequence)?.toString() ?: fallback
    fun numberOption(key: String, fallback: Double = 0.0): Double =
        (options[key] as? Number)?.toDouble() ?: fallback
    if (namespace == "NexaFastListRef" && !call.isNull("receiver")) {
        val reference = evaluateAsync(call.opt("receiver"), locals, scope) as? NexaFastListRef
            ?: return JSONObject.NULL
        return nexaDevInvokeFastListRef(reference, name, options)
    }
    if (namespace == "NexaBottomSheetRef" && !call.isNull("receiver")) {
        val reference = evaluateAsync(call.opt("receiver"), locals, scope) as? NexaBottomSheetRef
            ?: return mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
        return nexaDevInvokeBottomSheetRef(reference, name)
    }
    if (namespace == "Nexa" && !call.isNull("receiver")) {
        val reference = evaluate(call.opt("receiver"), locals, scope) as? NexaDevComponentRef
            ?: return mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
        val componentScope = reference.attachedScope
            ?: return mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
        val functionName = "Nexa.${reference.componentName}.$name"
        val declaration = functions[functionName] ?: return JSONObject.NULL
        val callArguments = JSONArray()
        val parameters = declaration.optJSONArray("parameters") ?: JSONArray()
        for (index in 0 until parameters.length()) {
            val parameter = parameters.optJSONObject(index) ?: continue
            val parameterName = parameter.optString("name")
            if (!options.containsKey(parameterName)) return JSONObject.NULL
            callArguments.put(options[parameterName] ?: JSONObject.NULL)
        }
        val functionCall = JSONObject()
            .put("name", functionName)
            .put("arguments", callArguments)
        return try {
            val result = if (declaration.optBoolean("is_async")) {
                invokeFunctionAsync(functionCall, reference.attachedParameters, componentScope)
            } else {
                invokeFunction(functionCall, reference.attachedParameters, componentScope)
            }
            pendingPluginFailure?.let { failure ->
                pendingPluginFailure = null
                return componentRefFailure(failure)
            }
            componentRefResult(
                result,
                declaration.optJSONObject("return_type")?.has("Result") == true,
            )
        } catch (failure: NexaDevPluginFailure) {
            pendingPluginFailure = null
            componentRefFailure(failure)
        }
    }
    if (namespace == "__NexaNetworkResponse" && (name == "getHeader" || name == "getHeaders" || name == "getCookie") && !call.isNull("receiver")) {
        val response = evaluateAsync(call.opt("receiver"), locals, scope)
        return nexaDevNetworkResponseMethod(response, name, options["name"] as? String ?: "")
    }
    if (namespace.startsWith("__NexaUserClass:")) {
        val receiver = if (call.isNull("receiver")) null else evaluateAsync(call.opt("receiver"), locals, scope)
        val result = if (call.optBoolean("is_async")) {
            invokeUserClassMethodAsync(namespace, name, receiver, options, locals, scope)
        } else {
            invokeUserClassMethod(namespace, name, receiver, options, scope)
        }
        if (result.first) return result.second
    }
    if (call.isNull("receiver")) {
        val pluginResult = NexaDevPluginBridge.invokeAsync(namespace, name, options, codecs, enumCases)
        if (pluginResult.first) {
            (pluginResult.second as? NexaDevPluginFailure)?.let { throw it }
            return pluginResult.second
        }
    } else {
        val receiver = evaluateAsync(call.opt("receiver"), locals, scope)
        val pluginResult = NexaDevPluginBridge.invokeInstanceAsync(receiver, namespace, name, options, codecs, enumCases)
        if (pluginResult.first) {
            (pluginResult.second as? NexaDevPluginFailure)?.let { throw it }
            return pluginResult.second
        }
    }
    when (val result = invokeSharedNative(namespace, name, options, codecs)) {
        is NexaDevSharedNativeResult.Handled -> return result.value
        NexaDevSharedNativeResult.Unhandled -> Unit
    }
    if (namespace == "AppIcon" && name == "set") {
        return NexaRuntime.setAlternateAppIcon(options["name"] as? String)
    }
    if (namespace == "File") {
        return when (name) {
            "readBytes" -> android.util.Base64.encodeToString(
                NexaFile.read(stringOption("path")), android.util.Base64.NO_WRAP,
            )
            "readText" -> NexaFile.readText(stringOption("path"))
            "writeBytes" -> {
                val contents = android.util.Base64.decode(stringOption("contents"), android.util.Base64.DEFAULT)
                NexaFile.write(contents, stringOption("path"))
                true
            }
            "writeText" -> NexaFile.writeText(stringOption("contents"), stringOption("path"))
            "delete" -> NexaFile.delete(stringOption("path"))
            else -> error("Unsupported async native call $namespace.$name")
        }
    }
    if (namespace == "SecureStorage") {
        return when (name) {
            "get" -> NexaSecureStorage.get(context, stringOption("key"), options["scope"] as? String) ?: JSONObject.NULL
            "set" -> {
                NexaSecureStorage.set(context, stringOption("key"), stringOption("value"), options["scope"] as? String)
                JSONObject.NULL
            }
            "delete" -> {
                NexaSecureStorage.delete(context, stringOption("key"), options["scope"] as? String)
                JSONObject.NULL
            }
            "clear" -> {
                NexaSecureStorage.clear(context, options["scope"] as? String)
                JSONObject.NULL
            }
            else -> error("Unsupported secure storage call $name")
        }
    }
    if (namespace == "Permissions") {
        val permissionName = stringOption("permission")
        val permission = when (permissionName) {
            "Camera" -> NexaPermission.Camera
            "Microphone" -> NexaPermission.Microphone
            "Photos" -> NexaPermission.Photos
            "Location" -> NexaPermission.Location
            "Notifications" -> NexaPermission.Notifications
            "Contacts" -> NexaPermission.Contacts
            "Calendar" -> NexaPermission.Calendar
            "Bluetooth" -> NexaPermission.Bluetooth
            else -> error("Unsupported permission $permissionName")
        }
        return when (name) {
            "request" -> NexaPermissions.request(context, permission).name
            "status" -> NexaPermissions.status(context, permission).name
            else -> error("Unsupported permission call $name")
        }
    }
    require(namespace == "Network" && (name == "fetch" || name == "download" || name == "upload")) {
        "Unsupported async native call $namespace.$name"
    }
    val headers = nexaDevStringMap(options["headers"])
    val rawPins = options["certificatePins"] as? Set<*> ?: emptySet<Any?>()
    val pins = rawPins.map { stringify(it ?: JSONObject.NULL) }.toSet()
    val body = (options["body"] as? String)?.toByteArray(Charsets.UTF_8)
    val url = stringOption("url")
    val method = stringOption("method", "GET")
    val timeout = numberOption("timeout", 30.0)
    val cachePolicy = options["cachePolicy"] as? String ?: "Default"
    val followRedirects = options["followRedirects"] as? Boolean ?: true
    val maxResponseBytes = numberOption("maxResponseBytes", 67_108_864.0).toLong()
    if (name == "upload") {
        val rawFields = options["fields"] as? Map<*, *> ?: emptyMap<Any?, Any?>()
        val fields = rawFields.entries.associate { stringify(it.key ?: JSONObject.NULL) to stringify(it.value ?: JSONObject.NULL) }
        val response = NexaNetwork.upload(
            context = context,
            url = url,
            filePath = stringOption("file"),
            fields = fields,
            headers = headers,
            timeout = timeout,
            followRedirects = followRedirects,
            maxResponseBytes = maxResponseBytes,
            certificatePins = pins,
        )
        return mapOf(
            "statusCode" to response.statusCode,
            "headers" to response.headers.map { mapOf("name" to it.first, "value" to it.second) },
            "body" to response.text,
        )
    }
    if (name == "download") {
        return NexaNetwork.download(
            context = context,
            url = url,
            destinationPath = stringOption("destinationPath"),
            method = method,
            body = body,
            headers = headers,
            timeoutMillis = (timeout * 1000.0).toLong(),
            cachePolicy = cachePolicy,
            followRedirects = followRedirects,
            maxResponseBytes = maxResponseBytes,
            certificatePins = pins,
        )
    }
    val response = NexaNetwork.fetch(
        context = context,
        url = url,
        method = method,
        body = body,
        headers = headers,
        timeoutMillis = (timeout * 1000.0).toLong(),
        cachePolicy = cachePolicy,
        followRedirects = followRedirects,
        maxResponseBytes = maxResponseBytes,
        certificatePins = pins,
    )
    return mapOf(
        "statusCode" to response.statusCode,
        "headers" to response.headers.map { mapOf("name" to it.first, "value" to it.second) },
        "body" to response.text,
    )
}

private fun componentRefFailure(failure: NexaDevPluginFailure): Map<String, Any> = mapOf(
    "Err" to mapOf(
        "Failure" to mapOf(
            "namespace" to failure.namespace,
            "errorType" to failure.errorType,
            "variant" to failure.variant,
            "payload" to failure.payload,
        ),
    ),
)

private fun componentRefResult(result: Any, returnsResult: Boolean): Map<String, Any> {
    if (!returnsResult) return mapOf("Ok" to result)
    val outcome = result as? Map<*, *> ?: return mapOf("Ok" to result)
    if (outcome.containsKey("Ok")) return mapOf("Ok" to (outcome["Ok"] ?: JSONObject.NULL))
    if (outcome.containsKey("Err")) {
        return mapOf("Err" to mapOf("Failure" to (outcome["Err"] ?: JSONObject.NULL)))
    }
    return mapOf("Ok" to result)
}

private fun nexaDevInvokeBottomSheetRef(reference: NexaBottomSheetRef, name: String): Any {
    val result = when (name) {
        "present" -> reference.present()
        "dismiss" -> reference.dismiss()
        else -> return JSONObject.NULL
    }
    return nexaDevComponentRefResult(result)
}

internal fun NexaDevStateStore.invokeNativeSync(call: JSONObject, locals: Map<String, Any>, scope: String): Any {
    val namespace = call.optString("namespace")
    val name = call.optString("name")
    runtimeTrace("native_call $namespace.$name")
    val rawArguments = call.optJSONArray("arguments") ?: JSONArray()
    val codecs = call.optJSONArray("codecs")?.let { values ->
        (0 until values.length()).map { values.opt(it) ?: JSONObject.NULL }
    } ?: emptyList()
    val options = mutableMapOf<String, Any>()
    for (index in 0 until rawArguments.length()) {
        val entry = rawArguments.optJSONArray(index) ?: continue
        val argumentName = entry.optString(0)
        options[argumentName] = evaluate(entry.opt(1), locals, scope)
    }
    if (namespace == "NexaFastListRef" && !call.isNull("receiver")) {
        val reference = evaluate(call.opt("receiver"), locals, scope) as? NexaFastListRef
            ?: return JSONObject.NULL
        return nexaDevInvokeFastListRef(reference, name, options)
    }
    if (namespace == "NexaBottomSheetRef" && !call.isNull("receiver")) {
        val reference = evaluate(call.opt("receiver"), locals, scope) as? NexaBottomSheetRef
            ?: return mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
        return nexaDevInvokeBottomSheetRef(reference, name)
    }
    if (namespace == "Nexa" && !call.isNull("receiver")) {
        val reference = evaluate(call.opt("receiver"), locals, scope) as? NexaDevComponentRef
            ?: return mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
        val componentScope = reference.attachedScope
            ?: return mapOf("Err" to mapOf("NotAttached" to JSONObject.NULL))
        val functionName = "Nexa.${reference.componentName}.$name"
        val declaration = functions[functionName] ?: return JSONObject.NULL
        val callArguments = JSONArray()
        val parameters = declaration.optJSONArray("parameters") ?: JSONArray()
        for (index in 0 until parameters.length()) {
            val parameter = parameters.optJSONObject(index) ?: continue
            val parameterName = parameter.optString("name")
            if (!options.containsKey(parameterName)) return JSONObject.NULL
            callArguments.put(options[parameterName] ?: JSONObject.NULL)
        }
        val result = invokeFunction(
            JSONObject().put("name", functionName).put("arguments", callArguments),
            reference.attachedParameters,
            componentScope,
        )
        pendingPluginFailure?.let { failure ->
            pendingPluginFailure = null
            return componentRefFailure(failure)
        }
        return componentRefResult(
            result,
            declaration.optJSONObject("return_type")?.has("Result") == true,
        )
    }
    if (namespace == "__NexaNetworkResponse" && (name == "getHeader" || name == "getHeaders" || name == "getCookie") && !call.isNull("receiver")) {
        val response = evaluate(call.opt("receiver"), locals, scope)
        return nexaDevNetworkResponseMethod(response, name, options["name"] as? String ?: "")
    }
    if (namespace == "Regex") {
        if (!call.isNull("receiver")) {
            val pattern = evaluate(call.opt("receiver"), locals, scope) as? String ?: ""
            val text = options["text"] as? String ?: ""
            return when (name) {
                "matches" -> runCatching { Regex(pattern).matches(text) }.getOrDefault(false)
                "find" -> nexaDevRegexFindAll(pattern, text).firstOrNull() ?: JSONObject.NULL
                "findAll" -> nexaDevRegexFindAll(pattern, text)
                "replace" -> runCatching {
                    Regex(pattern).replace(text, options["with"] as? String ?: "")
                }.getOrDefault(text)
                else -> JSONObject.NULL
            }
        }
        val pattern = options["pattern"] as? String ?: ""
        val text = options["text"] as? String ?: ""
        return when (name) {
            "new" -> pattern
            "isMatch" -> runCatching { Regex(pattern).containsMatchIn(text) }.getOrDefault(false)
            "matches" -> nexaDevRegexFindAll(pattern, text).map { it["value"] as? String ?: "" }
            "replace" -> runCatching {
                Regex(pattern).replace(text, options["replacement"] as? String ?: "")
            }.getOrDefault(text)
            else -> JSONObject.NULL
        }
    }
    if (namespace == "__NexaMap" && !call.isNull("receiver")) {
        val receiver = evaluate(call.opt("receiver"), locals, scope)
            as? Map<*, *> ?: emptyMap<Any?, Any?>()
        val key = stringify(options["key"] ?: JSONObject.NULL)
        return when (name) {
            "get" -> receiver[key] ?: JSONObject.NULL
            "contains" -> receiver.containsKey(key)
            "keys" -> receiver.keys.toList()
            "values" -> receiver.values.toList()
            else -> JSONObject.NULL
        }
    }
    try {
        if (namespace.startsWith("__NexaUserClass:")) {
            val receiver = if (call.isNull("receiver")) null else evaluate(call.opt("receiver"), locals, scope)
            val result = invokeUserClassMethod(namespace, name, receiver, options, scope)
            if (result.first) return result.second
        }
        if (call.isNull("receiver")) {
            val pluginResult = NexaDevPluginBridge.invokeSync(namespace, name, options, codecs, enumCases)
            if (pluginResult.first) return pluginResult.second
        } else {
            val receiver = evaluate(call.opt("receiver"), locals, scope)
            val pluginResult = NexaDevPluginBridge.invokeInstanceSync(receiver, namespace, name, options, codecs, enumCases)
            if (pluginResult.first) return pluginResult.second
        }
    } catch (failure: NexaDevPluginFailure) {
        pendingPluginFailure = failure
        return JSONObject.NULL
    }
    when (val result = invokeSharedNative(namespace, name, options, codecs)) {
        is NexaDevSharedNativeResult.Handled -> return result.value
        NexaDevSharedNativeResult.Unhandled -> Unit
    }
    return JSONObject.NULL
}
