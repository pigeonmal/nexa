package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject

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
    JSONObject().put("NativeCall", JSONObject()
        .put("namespace", namespace)
        .put("name", name)
        .put("arguments", arguments))

internal fun NexaDevStateStore.argument(name: String, expression: Any?): JSONArray =
    JSONArray().put(name).put(expression ?: JSONObject.NULL)

internal suspend fun NexaDevStateStore.invokeNativeAsync(
    call: JSONObject,
    locals: Map<String, Any>,
    scope: String,
): Any {
    val namespace = call.optString("namespace")
    val name = call.optString("name")
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
    if (call.isNull("receiver")) {
        val pluginResult = NexaDevPluginBridge.invokeAsync(namespace, name, options, codecs, enumCases)
        if (pluginResult.first) return pluginResult.second
    } else {
        val receiver = evaluateAsync(call.opt("receiver"), locals, scope)
        val pluginResult = NexaDevPluginBridge.invokeInstanceAsync(receiver, namespace, name, options, codecs, enumCases)
        if (pluginResult.first) return pluginResult.second
    }
    if (namespace == "Keyboard" && name == "dismiss") {
        dev.nexa.core.NexaRuntimeCore.dismissKeyboard()
        return JSONObject.NULL
    }
    if (namespace == "Haptics") {
        nexaDevPerformHaptics(name, options)
        return JSONObject.NULL
    }
    if (namespace == "Clipboard") {
        val manager = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE)
            as android.content.ClipboardManager
        return when (name) {
            "setText" -> {
                manager.setPrimaryClip(android.content.ClipData.newPlainText(null, (options["text"] as? String).orEmpty()))
                JSONObject.NULL
            }
            "getText" -> {
                val item = manager.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
                item?.coerceToText(context)?.toString() ?: JSONObject.NULL
            }
            "hasText" -> {
                val description = manager.primaryClipDescription
                description?.hasMimeType(android.content.ClipDescription.MIMETYPE_TEXT_PLAIN) == true ||
                    description?.hasMimeType(android.content.ClipDescription.MIMETYPE_TEXT_HTML) == true
            }
            else -> error("Unsupported clipboard call $name")
        }
    }
    if (namespace == "Storage") {
        return when (name) {
            "getString" -> NexaStorage.getString(context, options["key"] as? String ?: "") ?: JSONObject.NULL
            "setString" -> {
                NexaStorage.setString(
                    context,
                    options["key"] as? String ?: "",
                    options["value"] as? String ?: "",
                )
                JSONObject.NULL
            }
            "delete" -> {
                NexaStorage.delete(context, options["key"] as? String ?: "")
                JSONObject.NULL
            }
            "clear" -> {
                NexaStorage.clear(context)
                JSONObject.NULL
            }
            else -> error("Unsupported storage call $name")
        }
    }
    fun stringOption(key: String, fallback: String = ""): String =
        options[key] as? String ?: (options[key] as? CharSequence)?.toString() ?: fallback
    fun numberOption(key: String, fallback: Double): Double =
        (options[key] as? Number)?.toDouble() ?: fallback
    if (namespace == "Number" && name == "formatCurrency") {
        return nexaDevFormatCurrency(numberOption("amount", 0.0), stringOption("currencyCode"))
    }
    if (namespace == "Crypto") {
        return when (name) {
            "sha256" -> nexaCryptoSha256(stringOption("text"))
            "sha512" -> nexaCryptoSha512(stringOption("text"))
            "hmacSha256" -> nexaCryptoHmacSha256(stringOption("key"), stringOption("message"))
            "randomBytes" -> nexaCryptoRandomBytes(numberOption("count", 0.0).toInt())
            else -> error("Unsupported native call $namespace.$name")
        }
    }
    if (namespace == "Path") {
        return when (name) {
            "documents" -> NexaPath.documents(context)
            "caches" -> NexaPath.caches(context)
            "temporary" -> NexaPath.temporary(context)
            "appSupport" -> NexaPath.appSupport(context)
            "join" -> NexaPath.join(stringOption("path"), stringOption("component"))
            else -> error("Unsupported native call $namespace.$name")
        }
    }
    if (namespace == "File" && name == "exists") {
        return NexaFile.exists(stringOption("path"))
    }
    if (namespace == "File") {
        return when (name) {
            "readText" -> NexaFile.readText(stringOption("path"))
            "writeText" -> NexaFile.writeText(stringOption("contents"), stringOption("path"))
            "delete" -> NexaFile.delete(stringOption("path"))
            else -> error("Unsupported async native call $namespace.$name")
        }
    }
    if (namespace == "SecureStorage") {
        return when (name) {
            "get" -> NexaSecureStorage.get(context, stringOption("key")) ?: JSONObject.NULL
            "set" -> {
                NexaSecureStorage.set(context, stringOption("key"), stringOption("value"))
                JSONObject.NULL
            }
            "delete" -> {
                NexaSecureStorage.delete(context, stringOption("key"))
                JSONObject.NULL
            }
            "clear" -> {
                NexaSecureStorage.clear(context)
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
    require(namespace == "Network" && (name == "fetch" || name == "download")) {
        "Unsupported async native call $namespace.$name"
    }
    val rawHeaders = options["headers"] as? Map<*, *> ?: emptyMap<Any?, Any?>()
    val headers = rawHeaders.entries.associate { stringify(it.key ?: JSONObject.NULL) to stringify(it.value ?: JSONObject.NULL) }
    val rawPins = options["certificatePins"] as? Set<*> ?: emptySet<Any?>()
    val pins = rawPins.map { stringify(it ?: JSONObject.NULL) }.toSet()
    val body = (options["body"] as? String)?.toByteArray(Charsets.UTF_8)
    val url = stringOption("url")
    val method = stringOption("method", "GET")
    val timeout = numberOption("timeout", 30.0)
    val useCache = options["useCache"] as? Boolean ?: true
    val followRedirects = options["followRedirects"] as? Boolean ?: true
    val maxResponseBytes = numberOption("maxResponseBytes", 67_108_864.0).toLong()
    if (name == "download") {
        return NexaNetwork.download(
            context = context,
            url = url,
            destinationPath = stringOption("destinationPath"),
            method = method,
            body = body,
            headers = headers,
            timeoutMillis = (timeout * 1000.0).toLong(),
            useCache = useCache,
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
        useCache = useCache,
        followRedirects = followRedirects,
        maxResponseBytes = maxResponseBytes,
        certificatePins = pins,
    )
    return mapOf(
        "statusCode" to response.statusCode,
        "headers" to response.headers,
        "body" to response.text,
    )
}

internal fun NexaDevStateStore.invokeNativeSync(call: JSONObject, locals: Map<String, Any>, scope: String): Any {
    val namespace = call.optString("namespace")
    val name = call.optString("name")
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
    try {
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
    if (namespace == "Keyboard" && name == "dismiss") {
        dev.nexa.core.NexaRuntimeCore.dismissKeyboard()
        return JSONObject.NULL
    }
    if (namespace == "Haptics") {
        nexaDevPerformHaptics(name, options)
        return JSONObject.NULL
    }
    if (namespace == "Json") {
        val codec = codecs.firstOrNull() ?: error("Missing JSON codec for Json.$name")
        return when (name) {
            "parse" -> NexaDevValueCodec.parseJson(
                raw = options["raw"] as? String ?: "",
                type = codec,
                enumCases = enumCases,
            )
            "stringify" -> NexaDevValueCodec.stringifyJson(
                raw = options["value"],
                type = codec,
                enumCases = enumCases,
            )
            else -> error("Unsupported JSON call $name")
        }
    }
    if (namespace == "Clipboard") {
        val manager = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE)
            as android.content.ClipboardManager
        return when (name) {
            "setText" -> {
                manager.setPrimaryClip(android.content.ClipData.newPlainText(null, (options["text"] as? String).orEmpty()))
                JSONObject.NULL
            }
            "getText" -> {
                val item = manager.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
                item?.coerceToText(context)?.toString() ?: JSONObject.NULL
            }
            "hasText" -> {
                val description = manager.primaryClipDescription
                description?.hasMimeType(android.content.ClipDescription.MIMETYPE_TEXT_PLAIN) == true ||
                    description?.hasMimeType(android.content.ClipDescription.MIMETYPE_TEXT_HTML) == true
            }
            else -> error("Unsupported clipboard call $name")
        }
    }
    if (namespace == "Storage") {
        return when (name) {
            "getString" -> NexaStorage.getString(context, options["key"] as? String ?: "") ?: JSONObject.NULL
            "setString" -> {
                NexaStorage.setString(
                    context,
                    options["key"] as? String ?: "",
                    options["value"] as? String ?: "",
                )
                JSONObject.NULL
            }
            "delete" -> {
                NexaStorage.delete(context, options["key"] as? String ?: "")
                JSONObject.NULL
            }
            "clear" -> {
                NexaStorage.clear(context)
                JSONObject.NULL
            }
            else -> error("Unsupported storage call $name")
        }
    }
    fun stringOption(key: String): String =
        options[key] as? String ?: (options[key] as? CharSequence)?.toString() ?: ""
    if (namespace == "Number" && name == "formatCurrency") {
        val amount = (options["amount"] as? Number)?.toDouble() ?: 0.0
        return nexaDevFormatCurrency(amount, stringOption("currencyCode"))
    }
    if (namespace == "Crypto") {
        return when (name) {
            "sha256" -> nexaCryptoSha256(stringOption("text"))
            "sha512" -> nexaCryptoSha512(stringOption("text"))
            "hmacSha256" -> nexaCryptoHmacSha256(stringOption("key"), stringOption("message"))
            "randomBytes" -> nexaCryptoRandomBytes((options["count"] as? Number)?.toInt() ?: 0)
            else -> error("Unsupported native call $namespace.$name")
        }
    }
    return when (namespace) {
        "Path" -> when (name) {
            "documents" -> NexaPath.documents(context)
            "caches" -> NexaPath.caches(context)
            "temporary" -> NexaPath.temporary(context)
            "appSupport" -> NexaPath.appSupport(context)
            "join" -> NexaPath.join(stringOption("path"), stringOption("component"))
            else -> JSONObject.NULL
        }
        "File" -> if (name == "exists") NexaFile.exists(stringOption("path")) else JSONObject.NULL
        else -> JSONObject.NULL
    }
}
