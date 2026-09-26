package __NEXA_PACKAGE__

import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.net.Socket
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

internal class NexaDevSocketClient(
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
                .put(NexaDevKeys.TYPE, NexaDevKeys.MSG_HELLO)
                .put(NexaDevKeys.PAYLOAD, JSONObject()
                    .put(NexaDevKeys.PROTOCOL_VERSION, NexaDevSchema.PROTOCOL_VERSION)
                    .put(NexaDevKeys.SESSION_TOKEN, token)
                    .put(NexaDevKeys.TARGET, NexaDevSchema.TARGET_PLATFORM))
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
        val payload = envelope.optJSONObject(NexaDevKeys.PAYLOAD) ?: return
        when (envelope.optString(NexaDevKeys.TYPE)) {
            NexaDevKeys.MSG_FULL_MODULE -> {
                val devModule = payload.optJSONObject(NexaDevKeys.MODULE) ?: return
                val module = devModule.optJSONObject(NexaDevKeys.MODULE) ?: return
                currentRevision = devModule.optString(NexaDevKeys.REVISION).takeIf(String::isNotEmpty)
                val revision = currentRevision ?: return
                onMainAndWait { store.install(module) }
                acknowledge(output, revision)
                android.util.Log.i("NexaDevRuntime", "Applied module $revision")
            }
            NexaDevKeys.MSG_PATCH -> {
                val patch = payload.optJSONObject(NexaDevKeys.MSG_PATCH) ?: return
                val baseRevision = patch.optString(NexaDevKeys.BASE_REVISION)
                if (currentRevision != baseRevision) {
                    sendFrame(output, 0x1, JSONObject().put(NexaDevKeys.TYPE, NexaDevKeys.MSG_REQUEST_FULL_MODULE).toString())
                    return
                }
                val revision = patch.optString(NexaDevKeys.REVISION).takeIf(String::isNotEmpty) ?: return
                currentRevision = revision
                onMainAndWait { store.applyPatch(patch) }
                acknowledge(output, revision)
                android.util.Log.i("NexaDevRuntime", "Applied patch $revision")
            }
            NexaDevKeys.MSG_DIAGNOSTICS -> {
                val entries = payload.optJSONArray(NexaDevKeys.MSG_DIAGNOSTICS) ?: JSONArray()
                val messages = (0 until entries.length()).map { index ->
                    val diagnostic = entries.optJSONObject(index) ?: JSONObject()
                    "${diagnostic.optString(NexaDevKeys.FILE, "<unknown>")}:${diagnostic.optInt(NexaDevKeys.LINE, 1)}:${diagnostic.optInt(NexaDevKeys.COLUMN, 1)}: ${diagnostic.optString(NexaDevKeys.MESSAGE, "compile error")}"
                }
                mainHandler.post { store.publishDiagnostics(messages) }
            }
            NexaDevKeys.MSG_RESTART -> mainHandler.post { store.hotRestart() }
            NexaDevKeys.MSG_PERFORMANCE_OVERLAY -> {
                val enabled = payload.optBoolean(NexaDevKeys.ENABLED, false)
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
            .put(NexaDevKeys.TYPE, NexaDevKeys.MSG_ACKNOWLEDGE)
            .put(NexaDevKeys.PAYLOAD, JSONObject().put(NexaDevKeys.REVISION, revision))
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
