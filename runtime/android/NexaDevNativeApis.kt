package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject

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
    val options = mutableMapOf<String, Any>()
    for (index in 0 until rawArguments.length()) {
        val entry = rawArguments.optJSONArray(index) ?: continue
        val argumentName = entry.optString(0)
        options[argumentName] = evaluateAsync(entry.opt(1), locals, scope)
    }
    fun stringOption(key: String, fallback: String = ""): String =
        options[key] as? String ?: (options[key] as? CharSequence)?.toString() ?: fallback
    fun numberOption(key: String, fallback: Double): Double =
        (options[key] as? Number)?.toDouble() ?: fallback
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
            "request" -> NexaPermissions.request(permission).name
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
            url = url,
            destinationPath = stringOption("destinationPath"),
            method = method,
            body = body,
            headers = headers,
            timeout = timeout,
            useCache = useCache,
            followRedirects = followRedirects,
            maxResponseBytes = maxResponseBytes,
            certificatePins = pins,
        )
    }
    val response = NexaNetwork.fetch(
        url = url,
        method = method,
        body = body,
        headers = headers,
        timeout = timeout,
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
    val options = mutableMapOf<String, Any>()
    for (index in 0 until rawArguments.length()) {
        val entry = rawArguments.optJSONArray(index) ?: continue
        val argumentName = entry.optString(0)
        options[argumentName] = evaluate(entry.opt(1), locals, scope)
    }
    fun stringOption(key: String): String =
        options[key] as? String ?: (options[key] as? CharSequence)?.toString() ?: ""
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
