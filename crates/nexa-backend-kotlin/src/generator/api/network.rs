use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;
/// Emits a feature-gated Cronet client and the default path/file APIs.
///
/// Cronet owns connection pooling, HTTP/2, QUIC, Brotli, redirects, and the
/// disk cache. Coil 3 is wired to the same client, so a remote Image never
/// silently switches to OkHttp or another networking implementation.
use nexa_codegen::SourceWriter;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let uses_transport = features.uses_network_transport();
    imports.add(
        uses_transport
            || features.uses_network_connectivity
            || features.uses_path_api
            || features.uses_permissions,
        "android.content.Context",
    );
    imports.add(
        features.uses_network_api
            || features.uses_path_api
            || features.uses_permissions
            || features.uses_network_connectivity
            || features.facts.capabilities.uses_keyboard_api,
        "androidx.compose.ui.platform.LocalContext",
    );
    imports.add(features.uses_network_api, "android.net.Uri");
    imports.add(uses_transport, "kotlinx.coroutines.CancellationException");
    imports.add(
        features.uses_network_api || features.uses_file_async,
        "kotlinx.coroutines.Dispatchers",
    );
    imports.add(
        uses_transport,
        "kotlinx.coroutines.suspendCancellableCoroutine",
    );
    imports.add(
        features.uses_network_api || features.uses_file_async,
        "kotlinx.coroutines.withContext",
    );
    imports.add(features.uses_network_api, "kotlinx.coroutines.withTimeout");
    imports.add(uses_transport, "org.chromium.net.CronetEngine");
    imports.add(
        features.uses_network_api,
        "org.chromium.net.UploadDataProvider",
    );
    imports.add(features.uses_network_api, "org.chromium.net.UploadDataSink");
    imports.add(uses_transport, "org.chromium.net.UrlRequest");
    imports.add(uses_transport, "org.chromium.net.UrlResponseInfo");
    imports.add(uses_transport, "java.io.ByteArrayOutputStream");
    imports.add(
        features.uses_network_api || features.uses_path_api || features.uses_file_api,
        "java.io.File",
    );
    imports.add(features.uses_network_api, "java.io.FileOutputStream");
    imports.add(features.uses_network_api, "java.io.OutputStream");
    imports.add(uses_transport, "java.nio.ByteBuffer");
    imports.add(uses_transport, "java.util.WeakHashMap");
    imports.add(uses_transport, "java.util.concurrent.Executors");
    imports.add(uses_transport, "java.util.concurrent.Executor");
    imports.add(uses_transport, "kotlin.coroutines.resume");
    imports.add(uses_transport, "kotlin.coroutines.resumeWithException");
}

const NETWORK_CONNECTIVITY_MEMBERS: &str = r#"    private val statusLock = Any()
    private val statusMainHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private var statusManager: android.net.ConnectivityManager? = null
    private var statusCallback: android.net.ConnectivityManager.NetworkCallback? = null
    private var statusListener: ((Boolean) -> Unit)? = null
    private var lastStatus: Boolean? = null

    public fun isOnline(context: Context): Boolean {
        val manager = context.getSystemService(Context.CONNECTIVITY_SERVICE)
            as? android.net.ConnectivityManager ?: return false
        val network = manager.activeNetwork ?: return false
        val capabilities = manager.getNetworkCapabilities(network) ?: return false
        return capabilities.hasCapability(android.net.NetworkCapabilities.NET_CAPABILITY_INTERNET)
    }

    public fun onStatusChange(context: Context, listener: (Boolean) -> Unit) {
        val manager = context.applicationContext
            .getSystemService(Context.CONNECTIVITY_SERVICE) as? android.net.ConnectivityManager ?: return
        synchronized(statusLock) {
            statusListener = listener
            if (statusCallback != null) return
            val applicationContext = context.applicationContext
            val callback = object : android.net.ConnectivityManager.NetworkCallback() {
                override fun onAvailable(network: android.net.Network) {
                    dispatchStatusChange(applicationContext)
                }

                override fun onLost(network: android.net.Network) {
                    dispatchStatusChange(applicationContext)
                }

                override fun onCapabilitiesChanged(
                    network: android.net.Network,
                    capabilities: android.net.NetworkCapabilities,
                ) {
                    dispatchStatusChange(applicationContext)
                }
            }
            statusManager = manager
            statusCallback = callback
            lastStatus = isOnline(applicationContext)
            try {
                if (android.os.Build.VERSION.SDK_INT >= 24) {
                    manager.registerDefaultNetworkCallback(callback)
                } else {
                    val request = android.net.NetworkRequest.Builder()
                        .addCapability(android.net.NetworkCapabilities.NET_CAPABILITY_INTERNET)
                        .build()
                    manager.registerNetworkCallback(request, callback)
                }
            } catch (_: SecurityException) {
                statusCallback = null
                statusManager = null
                statusListener = null
            }
        }
    }

    private fun dispatchStatusChange(context: Context) {
        val value = isOnline(context)
        val listener = synchronized(statusLock) {
            if (lastStatus == value) null else {
                lastStatus = value
                statusListener
            }
        } ?: return
        statusMainHandler.post { listener(value) }
    }

"#;

pub(crate) fn render(
    out: &mut SourceWriter,
    include_network: bool,
    include_image_support: bool,
    include_path: bool,
    include_file: bool,
    include_file_async: bool,
    include_network_connectivity: bool,
) {
    if include_network || include_image_support {
        out.push_str(if include_network {
            "\npublic data class NexaNetworkResponse(\n"
        } else {
            "\nprivate data class NexaNetworkResponse(\n"
        });
        out.push_str(
            r#"    val statusCode: Int,
    internal val headers: List<Pair<String, String>>,
    val body: ByteArray,
) {
    val text: String get() = body.toString(Charsets.UTF_8)

    fun getHeader(name: String): String? =
        headers.firstOrNull { it.first.equals(name, ignoreCase = true) }?.second

    fun getHeaders(name: String): List<String> = buildList {
        for ((headerName, value) in headers) {
            if (headerName.equals(name, ignoreCase = true)) {
                add(value)
            }
        }
    }

    fun getCookie(): String = getHeaders("Set-Cookie")
        .mapNotNull { value ->
            val pair = value.substringBefore(';').trim()
            val equals = pair.indexOf('=')
            if (equals <= 0) null else {
                val name = pair.substring(0, equals).trim()
                val cookieValue = pair.substring(equals + 1).trim()
                if (name.isEmpty()) null else "$name=$cookieValue"
            }
        }
        .joinToString("; ")
}

private class NexaNetworkException(message: String) : Exception(message)

private object NexaCronetRuntime {
    private val callbackExecutor = Executors.newFixedThreadPool(2)
    private val ioExecutor = Executors.newFixedThreadPool(2)
    private val engines = WeakHashMap<Context, MutableMap<String, CronetEngine>>()

    fun executor() = callbackExecutor
    fun ioExecutor() = ioExecutor

    fun engine(context: Context, certificatePins: Map<String, Set<ByteArray>> = emptyMap()): CronetEngine {
        val applicationContext = context.applicationContext
        val pinKey = certificatePins.entries
            .sortedBy { it.key }
            .joinToString("|") { (host, pins) ->
                host + ":" + pins.map { pin -> pin.joinToString(",") }.sorted().joinToString(";")
            }
        return synchronized(engines) {
            engines[applicationContext]?.get(pinKey)?.let { return@synchronized it }
            val cacheDirectory = java.io.File(applicationContext.cacheDir, "nexa-cronet").apply {
                mkdirs()
            }
            val diskCacheSizeBytes = NexaCronetConfig.DISK_CACHE_SIZE_BYTES
            val builder = CronetEngine.Builder(applicationContext)
                .enableHttp2(true)
                .enableQuic(true)
                .enableBrotli(true)
                .setStoragePath(cacheDirectory.absolutePath)
                .enableHttpCache(
                    if (diskCacheSizeBytes > 0L) CronetEngine.Builder.HTTP_CACHE_DISK
                    else CronetEngine.Builder.HTTP_CACHE_DISABLED,
                    diskCacheSizeBytes,
                )
            val expiration = java.util.Date(System.currentTimeMillis() + 365L * 24L * 60L * 60L * 1000L)
            for ((host, pins) in certificatePins) {
                builder.addPublicKeyPins(host, pins, true, expiration)
            }
            val engine = builder.build()
            engines.getOrPut(applicationContext) { mutableMapOf() }[pinKey] = engine
            engine
        }
    }
}

"#,
        );
        if include_network {
            out.push_str(
            r#"
private class NexaUploadProvider(private val payload: ByteArray) : UploadDataProvider() {
    private var offset = 0

    override fun getLength(): Long = payload.size.toLong()

    override fun read(uploadDataSink: UploadDataSink, byteBuffer: ByteBuffer) {
        val count = minOf(byteBuffer.remaining(), payload.size - offset)
        if (count > 0) {
            byteBuffer.put(payload, offset, count)
            offset += count
        }
        // getLength() makes this a fixed-length (non-chunked) upload. Cronet
        // requires finalChunk=false for every read in that mode.
        uploadDataSink.onReadSucceeded(false)
    }

    override fun rewind(uploadDataSink: UploadDataSink) {
        offset = 0
        uploadDataSink.onRewindSucceeded()
    }
}

private class NexaMultipartUploadProvider(
    private val file: File,
    fields: Map<String, String>,
    private val boundary: String,
) : UploadDataProvider() {
    private val fieldParts = fields.toSortedMap().map { (name, value) ->
        "--$boundary\r\nContent-Disposition: form-data; name=\"${nexaDispositionParameter(name)}\"\r\n\r\n$value\r\n"
            .toByteArray(Charsets.UTF_8)
    }
    private val fileHeader = (
        "--$boundary\r\nContent-Disposition: form-data; name=\"file\"; " +
            "filename=\"${nexaDispositionParameter(file.name)}\"\r\n" +
            "Content-Type: application/octet-stream\r\n\r\n"
        ).toByteArray(Charsets.UTF_8)
    private val suffix = "\r\n--$boundary--\r\n".toByteArray(Charsets.UTF_8)
    private val fileLength = file.length()
    private val uploadLength = fieldParts.sumOf { it.size.toLong() } +
        fileHeader.size + fileLength + suffix.size
    private var channel: java.nio.channels.FileChannel? = null
    private var phase = 0
    private var fieldPartIndex = 0
    private var textOffset = 0

    override fun getLength(): Long = uploadLength

    override fun read(uploadDataSink: UploadDataSink, byteBuffer: ByteBuffer) {
        try {
            while (byteBuffer.hasRemaining()) {
                when (phase) {
                    0 -> {
                        if (fieldPartIndex >= fieldParts.size) {
                            phase = 1
                            continue
                        }
                        if (copyText(fieldParts[fieldPartIndex], byteBuffer)) {
                            fieldPartIndex += 1
                            textOffset = 0
                        }
                    }
                    1 -> if (copyText(fileHeader, byteBuffer)) {
                        phase = 2
                        textOffset = 0
                    }
                    2 -> {
                        val input = channel ?: java.io.FileInputStream(file).channel.also { channel = it }
                        when (input.read(byteBuffer)) {
                            -1 -> phase = 3
                            0 -> break
                        }
                    }
                    3 -> if (copyText(suffix, byteBuffer)) {
                        phase = 4
                        textOffset = 0
                    }
                    else -> break
                }
            }
            // getLength() makes this a fixed-length (non-chunked) upload.
            uploadDataSink.onReadSucceeded(false)
        } catch (error: Exception) {
            uploadDataSink.onReadError(error)
        }
    }

    override fun rewind(uploadDataSink: UploadDataSink) {
        try {
            channel?.position(0L)
            phase = 0
            fieldPartIndex = 0
            textOffset = 0
            uploadDataSink.onRewindSucceeded()
        } catch (error: Exception) {
            uploadDataSink.onRewindError(error)
        }
    }

    override fun close() {
        channel?.close()
        channel = null
    }

    private fun copyText(bytes: ByteArray, destination: ByteBuffer): Boolean {
        val count = minOf(destination.remaining(), bytes.size - textOffset)
        if (count > 0) {
            destination.put(bytes, textOffset, count)
            textOffset += count
        }
        return textOffset >= bytes.size
    }
}

private fun nexaDispositionParameter(value: String): String = value
    .replace("\\", "%5C")
    .replace("\"", "%22")
    .replace("\r", "%0D")
    .replace("\n", "%0A")

private class NexaCronetRequestClient(
    private val engine: CronetEngine,
) {

    suspend fun execute(
        url: String,
        method: String,
        headers: Map<String, String>,
        body: ByteArray?,
        maxResponseBytes: Long,
        followRedirects: Boolean,
        cachePolicy: String = "Default",
        sink: OutputStream? = null,
        uploadProvider: UploadDataProvider? = null,
    ): NexaNetworkResponse = suspendCancellableCoroutine { continuation ->
        val output = if (sink == null) ByteArrayOutputStream() else null
        var receivedBytes = 0L
        val completed = java.util.concurrent.atomic.AtomicBoolean(false)
        var request: UrlRequest? = null
        fun fail(error: Throwable) {
            if (completed.compareAndSet(false, true)) {
                continuation.resumeWithException(error)
            }
        }
        val callback = object : UrlRequest.Callback() {
            override fun onRedirectReceived(request: UrlRequest, info: UrlResponseInfo, newLocationUrl: String) {
                if (followRedirects) request.followRedirect() else request.cancel()
            }

            override fun onResponseStarted(request: UrlRequest, info: UrlResponseInfo) {
                request.read(ByteBuffer.allocateDirect(32 * 1024))
            }

            override fun onReadCompleted(request: UrlRequest, info: UrlResponseInfo, byteBuffer: ByteBuffer) {
                byteBuffer.flip()
                if (receivedBytes + byteBuffer.remaining() > maxResponseBytes) {
                    fail(NexaNetworkException("response exceeds maxResponseBytes"))
                    request.cancel()
                    return
                }
                val bytes = ByteArray(byteBuffer.remaining())
                byteBuffer.get(bytes)
                receivedBytes += bytes.size
                if (sink != null) {
                    NexaCronetRuntime.ioExecutor().execute {
                        try {
                            sink.write(bytes)
                            byteBuffer.clear()
                            request.read(byteBuffer)
                        } catch (error: Throwable) {
                            fail(error)
                            request.cancel()
                        }
                    }
                } else {
                    output!!.write(bytes)
                    byteBuffer.clear()
                    request.read(byteBuffer)
                }
            }

            override fun onSucceeded(request: UrlRequest, info: UrlResponseInfo) {
                if (completed.compareAndSet(false, true)) continuation.resume(
                    NexaNetworkResponse(info.httpStatusCode, info.allHeaders.flatMap { (name, values) -> values.map { name to it } }, output?.toByteArray() ?: ByteArray(0))
                )
            }

            override fun onFailed(request: UrlRequest, info: UrlResponseInfo?, error: org.chromium.net.CronetException) {
                fail(error)
            }

            override fun onCanceled(request: UrlRequest, info: UrlResponseInfo?) {
                fail(CancellationException("request cancelled"))
            }
        }
        val builder = engine.newUrlRequestBuilder(url, callback, NexaCronetRuntime.executor())
            .setHttpMethod(method)
        when (cachePolicy) {
            "Reload" -> builder.disableCache()
            "CacheOnly" -> builder.addHeader("Cache-Control", "only-if-cached, max-stale")
        }
        for ((name, value) in headers) {
            if (cachePolicy == "CacheOnly" && name.equals("Cache-Control", ignoreCase = true)) continue
            builder.addHeader(name, value)
        }
        if (body != null && uploadProvider != null) {
            fail(IllegalArgumentException("request body must have one source"))
            return@suspendCancellableCoroutine
        }
        if (body != null) {
            builder.setUploadDataProvider(NexaUploadProvider(body), NexaCronetRuntime.executor())
        } else if (uploadProvider != null) {
            builder.setUploadDataProvider(uploadProvider, NexaCronetRuntime.executor())
        }
        request = builder.build()
        continuation.invokeOnCancellation {
            if (completed.compareAndSet(false, true)) request?.cancel()
        }
        request!!.start()
    }
}

"#,
        );
        }
        if include_image_support {
            out.push_str(
            r#"
private class NexaCronetImageRequestClient(
    private val engine: CronetEngine,
) {
    suspend fun execute(url: String): NexaNetworkResponse = suspendCancellableCoroutine { continuation ->
        val output = ByteArrayOutputStream()
        var receivedBytes = 0L
        val completed = java.util.concurrent.atomic.AtomicBoolean(false)
        var request: UrlRequest? = null
        fun fail(error: Throwable) {
            if (completed.compareAndSet(false, true)) {
                continuation.resumeWithException(error)
            }
        }
        val callback = object : UrlRequest.Callback() {
            override fun onRedirectReceived(request: UrlRequest, info: UrlResponseInfo, newLocationUrl: String) {
                request.followRedirect()
            }

            override fun onResponseStarted(request: UrlRequest, info: UrlResponseInfo) {
                request.read(ByteBuffer.allocateDirect(32 * 1024))
            }

            override fun onReadCompleted(request: UrlRequest, info: UrlResponseInfo, byteBuffer: ByteBuffer) {
                byteBuffer.flip()
                if (receivedBytes + byteBuffer.remaining() > 64L * 1024L * 1024L) {
                    fail(NexaNetworkException("image response exceeds 64 MiB"))
                    request.cancel()
                    return
                }
                val bytes = ByteArray(byteBuffer.remaining())
                byteBuffer.get(bytes)
                receivedBytes += bytes.size
                output.write(bytes)
                byteBuffer.clear()
                request.read(byteBuffer)
            }

            override fun onSucceeded(request: UrlRequest, info: UrlResponseInfo) {
                if (completed.compareAndSet(false, true)) continuation.resume(
                    NexaNetworkResponse(info.httpStatusCode, info.allHeaders.flatMap { (name, values) -> values.map { name to it } }, output.toByteArray())
                )
            }

            override fun onFailed(request: UrlRequest, info: UrlResponseInfo?, error: org.chromium.net.CronetException) {
                fail(error)
            }

            override fun onCanceled(request: UrlRequest, info: UrlResponseInfo?) {
                fail(CancellationException("request cancelled"))
            }
        }
        request = engine.newUrlRequestBuilder(url, callback, NexaCronetRuntime.executor())
            .setHttpMethod("GET")
            .build()
        continuation.invokeOnCancellation {
            if (completed.compareAndSet(false, true)) request?.cancel()
        }
        request!!.start()
    }
}

"#,
        );
        }
    }
    if include_network {
        out.push_str(
            r#"private fun hexPin(value: String): ByteArray {
    require(value.length == 64 && value.all { it in '0'..'9' || it in 'a'..'f' || it in 'A'..'F' }) {
        "certificate pins must be 64-character SHA-256 SPKI hex values"
    }
    return ByteArray(32) { index ->
        ((value[index * 2].digitToInt(16) shl 4) or value[index * 2 + 1].digitToInt(16)).toByte()
    }
}

public object NexaNetwork {
    /** Shared Cronet engine for native streaming plugins in this app. */
    public fun cronetEngine(context: Context): CronetEngine = NexaCronetRuntime.engine(context)

    /** Shared Cronet callback executor for native streaming plugins in this app. */
    public fun cronetExecutor(): Executor = NexaCronetRuntime.executor()

    private fun requireSuccessful(response: NexaNetworkResponse) {
        if (response.statusCode !in 200..299) {
            throw NexaNetworkException("HTTP ${response.statusCode}")
        }
    }

"#,
        );
        if include_network_connectivity {
            out.push_str(NETWORK_CONNECTIVITY_MEMBERS);
        }
        out.push_str(
            r#"    public suspend fun fetch(
        context: Context,
        url: String,
        method: String = "GET",
        body: ByteArray? = null,
        headers: Map<String, String> = emptyMap(),
        timeoutMillis: Long = 30_000L,
        cachePolicy: String = "Default",
        followRedirects: Boolean = true,
        maxResponseBytes: Long = 64L * 1024L * 1024L,
        certificatePins: Set<String> = emptySet(),
    ): NexaNetworkResponse {
        val host = Uri.parse(url).host
        val pins = if (host != null && certificatePins.isNotEmpty()) {
            mapOf(host to certificatePins.map(::hexPin).toSet())
        } else {
            emptyMap()
        }
        val engine = NexaCronetRuntime.engine(context, pins)
        val response = withTimeout(timeoutMillis) {
            NexaCronetRequestClient(engine).execute(
                url,
                method,
                headers,
                body,
                maxResponseBytes,
                followRedirects,
                cachePolicy,
            )
        }
        return response
    }

    public suspend fun download(
        context: Context,
        url: String,
        destinationPath: String,
        method: String = "GET",
        body: ByteArray? = null,
        headers: Map<String, String> = emptyMap(),
        timeoutMillis: Long = 30_000L,
        cachePolicy: String = "Default",
        followRedirects: Boolean = true,
        maxResponseBytes: Long = 64L * 1024L * 1024L,
        certificatePins: Set<String> = emptySet(),
    ): Boolean {
        withContext(Dispatchers.IO) {
            val destination = File(destinationPath)
            val parent = destination.absoluteFile.parentFile ?: File(".")
            if (!parent.isDirectory && !parent.mkdirs() && !parent.isDirectory) {
                throw java.io.IOException("could not create download directory")
            }
            val temporaryFile = File.createTempFile("nexa-download-", ".tmp", parent)
            try {
                FileOutputStream(temporaryFile).use { output ->
                    val host = Uri.parse(url).host
                    val pins = if (host != null && certificatePins.isNotEmpty()) {
                        mapOf(host to certificatePins.map(::hexPin).toSet())
                    } else {
                        emptyMap()
                    }
                    val engine = NexaCronetRuntime.engine(context, pins)
                    val response = withTimeout(timeoutMillis) {
                        NexaCronetRequestClient(engine).execute(
                            url,
                            method,
                            headers,
                            body,
                            maxResponseBytes,
                            followRedirects,
                            cachePolicy,
                            output,
                        )
                    }
                    requireSuccessful(response)
                }
                // The temporary file is a sibling, so rename replaces an
                // existing destination atomically on the same filesystem.
                android.system.Os.rename(temporaryFile.absolutePath, destination.absolutePath)
            } finally {
                temporaryFile.delete()
            }
        }
        return true
    }

    public suspend fun upload(
        context: Context,
        url: String,
        filePath: String,
        fields: Map<String, String> = emptyMap(),
        headers: Map<String, String> = emptyMap(),
        timeout: Double = 30.0,
        followRedirects: Boolean = true,
        maxResponseBytes: Long = 64L * 1024L * 1024L,
        certificatePins: Set<String> = emptySet(),
    ): NexaNetworkResponse {
        val source = File(filePath)
        require(source.isFile) { "upload file does not exist: $filePath" }
        val boundary = "Nexa-${java.util.UUID.randomUUID()}"
        val host = Uri.parse(url).host
        val pins = if (host != null && certificatePins.isNotEmpty()) {
            mapOf(host to certificatePins.map(::hexPin).toSet())
        } else {
            emptyMap()
        }
        val requestHeaders = headers.toMutableMap().apply {
            keys.filter { it.equals("Content-Type", ignoreCase = true) }.forEach { remove(it) }
            put("Content-Type", "multipart/form-data; boundary=$boundary")
        }
        val response = withTimeout((timeout * 1000.0).toLong()) {
            NexaCronetRequestClient(NexaCronetRuntime.engine(context, pins)).execute(
                url = url,
                method = "POST",
                headers = requestHeaders,
                body = null,
                maxResponseBytes = maxResponseBytes,
                followRedirects = followRedirects,
                cachePolicy = "Reload",
                uploadProvider = NexaMultipartUploadProvider(source, fields, boundary),
            )
        }
        return response
    }
}

"#,
        );
    }
    if include_network_connectivity && !include_network {
        out.push_str("\npublic object NexaNetwork {\n");
        out.push_str(NETWORK_CONNECTIVITY_MEMBERS);
        out.push_str("}\n\n");
    }
    if include_path {
        out.push_str(
            r#"public object NexaPath {
    public fun documents(context: Context): String =
        context.filesDir.path

    public fun caches(context: Context): String =
        context.cacheDir.path

    public fun temporary(context: Context): String =
        (context.externalCacheDir ?: context.cacheDir).path

    public fun appSupport(context: Context): String =
        context.noBackupFilesDir.path

    public fun join(path: String, component: String): String =
        File(File(path), component).path
}

"#,
        );
    }
    if include_file {
        out.push_str(
            r#"public object NexaFile {
"#,
        );
        if include_file_async {
            out.push_str(
                r#"    public suspend fun read(path: String): ByteArray = withContext(Dispatchers.IO) { File(path).readBytes() }

    public suspend fun write(data: ByteArray, path: String): Boolean = withContext(Dispatchers.IO) {
        val file = File(path)
        file.parentFile?.mkdirs()
        val atomicFile = android.util.AtomicFile(file)
        val output = atomicFile.startWrite()
        try {
            output.write(data)
            atomicFile.finishWrite(output)
        } catch (failure: Throwable) {
            atomicFile.failWrite(output)
            throw failure
        }
        true
    }

    public suspend fun readText(path: String): String = read(path).toString(Charsets.UTF_8)

    public suspend fun writeText(text: String, path: String): Boolean {
        return write(text.toByteArray(Charsets.UTF_8), path)
    }

    public suspend fun delete(path: String): Boolean = withContext(Dispatchers.IO) {
        val file = File(path)
        if (!file.exists()) {
            false
        } else if (file.delete()) {
            true
        } else if (!file.exists()) {
            false
        } else {
            throw java.io.IOException("could not delete file")
        }
    }

"#,
            );
        }
        out.push_str(
            r#"    public fun exists(path: String): Boolean = File(path).exists()
}

"#,
        );
    }
    if include_image_support {
        out.push_str(
            r#"
private class NexaCronetNetworkClient(
    private val engine: CronetEngine,
) : NetworkClient {
    override suspend fun <T> executeRequest(
        request: NetworkRequest,
        block: suspend (NetworkResponse) -> T,
    ): T {
        val response = NexaCronetImageRequestClient(engine).execute(request.url)
        val networkResponse = NetworkResponse(
            code = response.statusCode,
            headers = response.headers.toNetworkHeaders(),
            body = NetworkResponseBody(Buffer().write(response.body)),
        )
        return block(networkResponse)
    }
}

private fun List<Pair<String, String>>.toNetworkHeaders(): NetworkHeaders {
    val builder = NetworkHeaders.Builder()
    for ((name, value) in this) builder.add(name, value)
    return builder.build()
}

@OptIn(coil3.annotation.ExperimentalCoilApi::class)
private object NexaImageLoaderStore {
    @Volatile private var loader: ImageLoader? = null

    fun get(applicationContext: Context): ImageLoader {
        loader?.let { return it }
        return synchronized(this) {
            loader?.let { return@synchronized it }
            ImageLoader.Builder(applicationContext)
                .components {
                    add(NetworkFetcher.Factory(networkClient = { NexaCronetNetworkClient(NexaCronetRuntime.engine(applicationContext)) }))
                }
                .build()
                .also {
                    loader = it
                }
        }
    }
}

@OptIn(coil3.annotation.ExperimentalCoilApi::class)
@Composable
internal fun nexaImageLoader(): ImageLoader {
    return NexaImageLoaderStore.get(LocalContext.current.applicationContext)
}
"#,
        );
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;

    use super::render;

    #[test]
    fn path_helpers_match_the_single_component_language_api() {
        let mut output = SourceWriter::new();
        render(&mut output, false, false, true, false, false, false);

        assert!(output.contains("public fun documents(context: Context): String"));
        assert!(output.contains("public fun caches(context: Context): String"));
        assert!(output.contains("public fun temporary(context: Context): String"));
        assert!(output.contains("public fun appSupport(context: Context): String"));
        assert!(output.contains("public fun join(path: String, component: String): String"));
        assert!(!output.contains("vararg components: String"));
        assert!(!output.contains("components.fold("));
    }

    #[test]
    fn outgoing_requests_accept_string_maps() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("headers: Map<String, String> = emptyMap()"));
        assert!(output.contains("internal val headers: List<Pair<String, String>>"));
        assert!(!output.contains("NexaHttpHeader"));
        assert!(output.contains("fun getHeaders(name: String): List<String>"));
        assert!(output.contains("fun getCookie(): String"));
        assert!(output.contains("equals(name, ignoreCase = true)"));
        assert!(output.contains("getHeaders(\"Set-Cookie\")"));
        assert!(output.contains("substringBefore(';').trim()"));
        assert!(output.contains("pair.indexOf('=')"));
        assert!(output.contains("pair.substring(equals + 1).trim()"));
        assert!(output.contains("joinToString(\"; \")"));
        assert!(output.contains("for ((name, value) in headers) {"));
        assert!(output.contains(
            "if (cachePolicy == \"CacheOnly\" && name.equals(\"Cache-Control\", ignoreCase = true)) continue"
        ));
        assert!(output.contains("builder.addHeader(name, value)"));
    }

    #[test]
    fn network_pins_require_64_character_sha256_spki_hex_values() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("value.length == 64"));
        assert!(output.contains("it in '0'..'9'"));
        assert!(output.contains("it in 'a'..'f'"));
        assert!(output.contains("it in 'A'..'F'"));
        assert!(output.contains("certificate pins must be 64-character SHA-256 SPKI hex values"));
        assert!(output.contains("ByteArray(32)"));
        assert!(output.contains("digitToInt(16)"));
    }

    #[test]
    fn downloads_stream_to_a_temporary_sibling_before_replacing_the_destination() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);
        let download = output
            .as_str()
            .split_once("public suspend fun download(")
            .expect("find generated download helper")
            .1
            .split_once("public suspend fun upload(")
            .expect("find end of generated download helper")
            .0;

        assert!(download.contains("File.createTempFile(\"nexa-download-\", \".tmp\", parent)"));
        assert!(download.contains("FileOutputStream(temporaryFile).use { output ->"));
        assert!(download.contains(
            "android.system.Os.rename(temporaryFile.absolutePath, destination.absolutePath)"
        ));
        assert!(download.contains("finally {\n                temporaryFile.delete()"));
        assert!(!download.contains("FileOutputStream(destination)"));
        assert!(
            download
                .find("requireSuccessful(response)")
                .expect("validate HTTP result")
                < download
                    .find("android.system.Os.rename(")
                    .expect("commit completed download"),
            "failed HTTP responses must not replace the destination"
        );
    }

    #[test]
    fn network_cache_uses_the_generated_project_cronet_configuration() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("NexaCronetConfig.DISK_CACHE_SIZE_BYTES"));
        assert!(output.contains("CronetEngine.Builder.HTTP_CACHE_DISABLED"));
        assert!(!output.contains(
            ".enableHttpCache(\n                CronetEngine.Builder.HTTP_CACHE_DISK,\n                64L * 1024L * 1024L"
        ));
    }

    #[test]
    fn native_plugins_can_reuse_the_generated_cronet_engine() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("public fun cronetEngine(context: Context): CronetEngine"));
        assert!(output.contains("public fun cronetExecutor(): Executor"));
        assert!(output.contains("return synchronized(engines)"));
        assert!(
            output.contains(
                "engines.getOrPut(applicationContext) { mutableMapOf() }[pinKey] = engine"
            )
        );

        let mut without_network = SourceWriter::new();
        render(
            &mut without_network,
            false,
            false,
            false,
            false,
            false,
            true,
        );
        assert!(!without_network.contains("cronetEngine(context"));
    }

    #[test]
    fn cronet_image_adapter_preserves_each_response_header_value() {
        let mut output = SourceWriter::new();
        render(&mut output, true, true, false, false, false, false);

        assert!(output.contains("headers = response.headers.toNetworkHeaders()"));
        assert!(output.contains("private fun List<Pair<String, String>>.toNetworkHeaders()"));
        assert!(output.contains("for ((name, value) in this) builder.add(name, value)"));
    }

    #[test]
    fn network_status_uses_connectivity_manager_without_cronet_requests() {
        let mut output = SourceWriter::new();
        render(&mut output, false, false, false, false, false, true);

        assert!(output.contains("public fun isOnline(context: Context): Boolean"));
        assert!(output.contains("manager.activeNetwork"));
        assert!(output.contains("NET_CAPABILITY_INTERNET"));
        assert!(output.contains("public fun onStatusChange(context: Context"));
        assert!(output.contains("registerDefaultNetworkCallback"));
        assert!(!output.contains("Cronet"));
    }

    #[test]
    fn multipart_upload_streams_file_and_multipart_parts_through_cronet() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("private fun requireSuccessful(response: NexaNetworkResponse)"));
        assert_eq!(
            output
                .as_str()
                .matches("requireSuccessful(response)")
                .count(),
            1
        );
        assert_eq!(
            output
                .as_str()
                .matches("response.statusCode !in 200..299")
                .count(),
            1
        );
        let fetch = output
            .as_str()
            .split_once("public suspend fun fetch(")
            .expect("find generated fetch helper")
            .1
            .split_once("public suspend fun download(")
            .expect("find end of generated fetch helper")
            .0;
        assert!(!fetch.contains("requireSuccessful(response)"));
        let upload = output
            .as_str()
            .split_once("public suspend fun upload(")
            .expect("find generated upload helper")
            .1;
        assert!(!upload.contains("requireSuccessful(response)"));
        assert!(output.contains("public suspend fun upload("));
        assert!(output.contains("NexaMultipartUploadProvider(source, fields, boundary)"));
        assert!(output.contains("input.read(byteBuffer)"));
        assert!(output.contains("uploadDataSink.onReadSucceeded(false)"));
        assert!(!output.contains("writeMultipartBody"));
        assert!(!output.contains("nexa-upload-"));
        assert!(output.contains("timeout: Double = 30.0"));
        assert!(output.contains("followRedirects: Boolean = true"));
        assert!(output.contains("NexaCronetRuntime.engine(context, pins)"));
        assert!(output.contains("equals(\"Content-Type\", ignoreCase = true)"));
        assert!(
            output.contains("put(\"Content-Type\", \"multipart/form-data; boundary=$boundary\")")
        );
    }

    #[test]
    fn fixed_length_cronet_uploads_never_mark_a_final_chunk() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert_eq!(
            output
                .as_str()
                .matches("uploadDataSink.onReadSucceeded(false)")
                .count(),
            2,
            "byte-array and multipart providers both have known lengths",
        );
        assert!(!output.contains("onReadSucceeded(offset >= payload.size)"));
        assert!(!output.contains("onReadSucceeded(phase == 4)"));
    }

    #[test]
    fn file_writes_are_atomic_and_delete_missing_paths_returns_false() {
        let mut output = SourceWriter::new();
        render(&mut output, false, false, false, true, true, false);

        assert!(output.contains("android.util.AtomicFile(file)"));
        assert!(output.contains("val output = atomicFile.startWrite()"));
        assert!(output.contains("atomicFile.finishWrite(output)"));
        assert!(output.contains("atomicFile.failWrite(output)"));
        assert!(!output.contains("file.writeBytes(data)"));
        assert!(output.contains("if (!file.exists()) {\n            false"));
        assert!(output.contains("throw java.io.IOException(\"could not delete file\")"));
    }
}
