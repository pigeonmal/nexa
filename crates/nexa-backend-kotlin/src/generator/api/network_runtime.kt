private val NEXA_EMPTY_RESPONSE_BODY = ByteArray(0)

// Retain a bounded body and transfer ownership when capacity matches its size.
// This avoids ByteArrayOutputStream growth copies and its unconditional copy.
private class NexaByteArrayBuilder(private val maximumSize: Int) {
    private var storage = NEXA_EMPTY_RESPONSE_BODY
    var size: Int = 0
        private set

    init {
        require(maximumSize >= 0)
    }

    fun write(source: ByteArray, offset: Int, length: Int) {
        require(length >= 0 && offset >= 0 && offset <= source.size - length)
        val requiredLong = size.toLong() + length.toLong()
        require(requiredLong <= maximumSize.toLong()) { "response body exceeds maxResponseBytes" }
        val required = requiredLong.toInt()
        if (required > storage.size) {
            val doubled = if (storage.isEmpty()) 256L else storage.size.toLong() * 2L
            val capacity = maxOf(requiredLong, doubled).coerceAtMost(maximumSize.toLong()).toInt()
            storage = storage.copyOf(capacity)
        }
        source.copyInto(storage, size, offset, offset + length)
        size = required
    }

    fun take(): ByteArray {
        val result = if (size == storage.size) storage else storage.copyOf(size)
        storage = NEXA_EMPTY_RESPONSE_BODY
        size = 0
        return result
    }

    fun clear() {
        storage = NEXA_EMPTY_RESPONSE_BODY
        size = 0
    }
}

// Cronet writes file chunks on Nexa's I/O executor. Serialize close with those
// writes so cancellation cannot close or delete a staging file mid-write.
private class NexaSerializedOutputStream(private val delegate: OutputStream) : OutputStream() {
    private val lock = Any()
    private var closed = false

    override fun write(value: Int) {
        synchronized(lock) {
            check(!closed) { "output stream is closed" }
            delegate.write(value)
        }
    }

    override fun write(bytes: ByteArray, offset: Int, length: Int) {
        synchronized(lock) {
            check(!closed) { "output stream is closed" }
            delegate.write(bytes, offset, length)
        }
    }

    override fun flush() {
        synchronized(lock) {
            if (!closed) delegate.flush()
        }
    }

    override fun close() {
        synchronized(lock) {
            if (!closed) {
                closed = true
                delegate.close()
            }
        }
    }
}
