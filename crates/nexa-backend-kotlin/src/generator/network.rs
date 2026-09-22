/// Emits a feature-gated Cronet client and the default path/file APIs.
///
/// Cronet owns connection pooling, HTTP/2, QUIC, Brotli, redirects, and the
/// disk cache. Coil 3 is wired to the same client, so a remote Image never
/// silently switches to OkHttp or another networking implementation.
pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
public data class NexaNetworkResponse(
    val statusCode: Int,
    val headers: Map<String, List<String>>,
    val body: ByteArray,
) {
    val text: String get() = body.toString(Charsets.UTF_8)
}

private class NexaNetworkException(message: String) : Exception(message)

private object NexaCronetRuntime {
    private val callbackExecutor = Executors.newFixedThreadPool(2)
    private val engines = WeakHashMap<Context, CronetEngine>()

    fun executor() = callbackExecutor

    fun engine(context: Context, certificatePins: Map<String, Set<ByteArray>> = emptyMap()): CronetEngine {
        val applicationContext = context.applicationContext
        if (certificatePins.isEmpty()) {
            synchronized(engines) {
                engines[applicationContext]?.let { return it }
            }
        }
        val builder = CronetEngine.Builder(context.applicationContext)
            .enableHttp2(true)
            .enableQuic(true)
            .enableBrotli(true)
            .enableHttpCache(
                CronetEngine.Builder.HTTP_CACHE_DISK,
                64L * 1024L * 1024L,
            )
        val expiration = Date(System.currentTimeMillis() + 365L * 24L * 60L * 60L * 1000L)
        for ((host, pins) in certificatePins) {
            builder.addPublicKeyPins(host, pins, true, expiration)
        }
        val engine = builder.build()
        if (certificatePins.isEmpty()) {
            synchronized(engines) { engines[applicationContext] = engine }
        }
        return engine
    }
}

public object NexaRuntime {
    @Volatile private var applicationContext: Context? = null

    public fun bind(context: Context) {
        val application = context.applicationContext
        if (applicationContext !== application) applicationContext = application
    }

    public fun context(): Context = requireNotNull(applicationContext) {
        "NexaRuntime.bind must run before a native network or path call"
    }
}

private class NexaUploadProvider(private val payload: ByteArray) : UploadDataProvider() {
    private var offset = 0

    override fun getLength(): Long = payload.size.toLong()

    override fun read(uploadDataSink: UploadDataSink, byteBuffer: ByteBuffer) {
        val count = minOf(byteBuffer.remaining(), payload.size - offset)
        if (count > 0) {
            byteBuffer.put(payload, offset, count)
            offset += count
        }
        uploadDataSink.onReadSucceeded(offset >= payload.size)
    }

    override fun rewind(uploadDataSink: UploadDataSink) {
        offset = 0
        uploadDataSink.onRewindSucceeded()
    }
}

private class NexaCronetNetworkClient(
    private val engine: CronetEngine,
) : NetworkClient {
    override suspend fun <T> executeRequest(
        request: NetworkRequest,
        block: suspend (NetworkResponse) -> T,
    ): T {
        val response = execute(request.url, request.method, request.headers.asMap(), null, 64L * 1024L * 1024L, true)
        val networkResponse = NetworkResponse(
            code = response.statusCode,
            headers = response.headers.toNetworkHeaders(),
            body = NetworkResponseBody(Buffer().write(response.body)),
        )
        return block(networkResponse)
    }

    suspend fun execute(
        url: String,
        method: String,
        headers: Map<String, List<String>>,
        body: ByteArray?,
        maxResponseBytes: Long,
        followRedirects: Boolean,
        useCache: Boolean = true,
        sink: FileOutputStream? = null,
    ): NexaNetworkResponse = suspendCancellableCoroutine { continuation ->
        val output = if (sink == null) ByteArrayOutputStream() else null
        var receivedBytes = 0L
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
                    request.cancel()
                    continuation.resumeWithException(NexaNetworkException("response exceeds maxResponseBytes"))
                    return
                }
                val bytes = ByteArray(byteBuffer.remaining())
                byteBuffer.get(bytes)
                receivedBytes += bytes.size
                if (sink != null) sink.write(bytes) else output!!.write(bytes)
                byteBuffer.clear()
                request.read(byteBuffer)
            }

            override fun onSucceeded(request: UrlRequest, info: UrlResponseInfo) {
                continuation.resume(
                    NexaNetworkResponse(info.httpStatusCode, info.allHeaders, output?.toByteArray() ?: ByteArray(0))
                )
            }

            override fun onFailed(request: UrlRequest, info: UrlResponseInfo?, error: org.chromium.net.CronetException) {
                continuation.resumeWithException(error)
            }

            override fun onCanceled(request: UrlRequest, info: UrlResponseInfo?) {
                if (continuation.isActive) continuation.resumeWithException(CancellationException("request cancelled"))
            }
        }
        val builder = engine.newUrlRequestBuilder(url, callback, NexaCronetRuntime.executor())
            .setHttpMethod(method)
        if (!useCache) builder.disableCache()
        for ((name, values) in headers) {
            for (value in values) builder.addHeader(name, value)
        }
        if (body != null) {
            builder.setUploadDataProvider(NexaUploadProvider(body), NexaCronetRuntime.executor())
        }
        val request = builder.build()
        continuation.invokeOnCancellation { request.cancel() }
        request.start()
    }
}

private fun Map<String, List<String>>.toNetworkHeaders(): NetworkHeaders {
    val builder = NetworkHeaders.Builder()
    for ((name, values) in this) for (value in values) builder.add(name, value)
    return builder.build()
}

private fun hexPin(value: String): ByteArray {
    require(value.length % 2 == 0) { "certificate pins must be hexadecimal SHA-256 values" }
    return value.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
}

public object NexaNetwork {
    public suspend fun fetch(
        context: Context,
        url: String,
        method: String = "GET",
        body: ByteArray? = null,
        headers: Map<String, String> = emptyMap(),
        timeoutMillis: Long = 30_000L,
        useCache: Boolean = true,
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
        val requestHeaders = headers.mapValues { listOf(it.value) }
        val response = withTimeout(timeoutMillis) {
            NexaCronetNetworkClient(engine).execute(
                url,
                method,
                requestHeaders,
                body,
                maxResponseBytes,
                followRedirects,
                useCache,
            )
        }
        if (response.statusCode !in 200..299) {
            throw NexaNetworkException("HTTP ${response.statusCode}")
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
        useCache: Boolean = true,
        followRedirects: Boolean = true,
        maxResponseBytes: Long = 64L * 1024L * 1024L,
        certificatePins: Set<String> = emptySet(),
    ): Boolean {
        withContext(Dispatchers.IO) {
            val destination = File(destinationPath)
            destination.parentFile?.mkdirs()
            FileOutputStream(destination).use { output ->
                val host = Uri.parse(url).host
                val pins = if (host != null && certificatePins.isNotEmpty()) {
                    mapOf(host to certificatePins.map(::hexPin).toSet())
                } else {
                    emptyMap()
                }
                val engine = NexaCronetRuntime.engine(context, pins)
                val requestHeaders = headers.mapValues { listOf(it.value) }
                val response = withTimeout(timeoutMillis) {
                    NexaCronetNetworkClient(engine).execute(
                        url,
                        method,
                        requestHeaders,
                        body,
                        maxResponseBytes,
                        followRedirects,
                        useCache,
                        output,
                    )
                }
                if (response.statusCode !in 200..299) {
                    throw NexaNetworkException("HTTP ${response.statusCode}")
                }
            }
        }
        return true
    }
}

public object NexaPath {
    public fun documents(context: Context, vararg components: String): String =
        components.fold(context.filesDir) { file, component -> File(file, component) }.path

    public fun caches(context: Context, vararg components: String): String =
        components.fold(context.cacheDir) { file, component -> File(file, component) }.path

    public fun temporary(context: Context, vararg components: String): String =
        components.fold(context.externalCacheDir ?: context.cacheDir) { file, component -> File(file, component) }.path

    public fun appSupport(context: Context, vararg components: String): String =
        components.fold(context.noBackupFilesDir) { file, component -> File(file, component) }.path

    public fun join(path: String, vararg components: String): String =
        components.fold(File(path)) { file, component -> File(file, component) }.path
}

public object NexaFile {
    public suspend fun read(path: String): ByteArray = withContext(Dispatchers.IO) { File(path).readBytes() }

    public suspend fun write(data: ByteArray, path: String) = withContext(Dispatchers.IO) {
        val file = File(path)
        file.parentFile?.mkdirs()
        file.writeBytes(data)
    }

    public suspend fun readText(path: String): String = read(path).toString(Charsets.UTF_8)

    public suspend fun writeText(text: String, path: String): Boolean {
        write(text.toByteArray(Charsets.UTF_8), path)
        return true
    }

    public suspend fun delete(path: String): Boolean = withContext(Dispatchers.IO) { File(path).delete() }

    public fun exists(path: String): Boolean = File(path).exists()
}

@Composable
private fun nexaImageLoader(): ImageLoader {
    val context = LocalContext.current.applicationContext
    return remember(context) {
        val engine = NexaCronetRuntime.engine(context)
        ImageLoader.Builder(context)
            .components {
                add(NetworkFetcher.Factory(networkClient = { NexaCronetNetworkClient(engine) }))
            }
            .build()
    }
}
"#,
    );
}
