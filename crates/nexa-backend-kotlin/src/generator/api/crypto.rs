//! Core cryptographic helpers implemented with Android and Java APIs.

use nexa_codegen::SourceWriter;

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Core cryptographic helpers.

private val nexaCryptoHexDigits = "0123456789abcdef".toCharArray()

private fun nexaCryptoHex(bytes: ByteArray): String {
    val encoded = CharArray(bytes.size * 2)
    for (index in bytes.indices) {
        val value = bytes[index].toInt() and 0xff
        encoded[index * 2] = nexaCryptoHexDigits[value ushr 4]
        encoded[index * 2 + 1] = nexaCryptoHexDigits[value and 0x0f]
    }
    return String(encoded)
}

internal fun nexaCryptoSha256(text: String): String = nexaCryptoHex(
    java.security.MessageDigest.getInstance("SHA-256").digest(text.toByteArray(Charsets.UTF_8)),
)

internal fun nexaCryptoSha512(text: String): String = nexaCryptoHex(
    java.security.MessageDigest.getInstance("SHA-512").digest(text.toByteArray(Charsets.UTF_8)),
)

internal fun nexaCryptoHmacSha256(key: String, message: String): String {
    val keyBytes = key.toByteArray(Charsets.UTF_8)
    // SecretKeySpec rejects empty arrays; one zero byte has the same HMAC
    // block padding as the empty key and preserves the defined result.
    val usableKey = if (keyBytes.isEmpty()) byteArrayOf(0) else keyBytes
    val mac = javax.crypto.Mac.getInstance("HmacSHA256")
    mac.init(javax.crypto.spec.SecretKeySpec(usableKey, "HmacSHA256"))
    return nexaCryptoHex(mac.doFinal(message.toByteArray(Charsets.UTF_8)))
}

private object NexaCryptoRandom {
    val secureRandom = java.security.SecureRandom()
}

internal fun nexaCryptoRandomBytes(count: Int): String {
    if (count <= 0) return ""
    val bytes = ByteArray(count)
    NexaCryptoRandom.secureRandom.nextBytes(bytes)
    return android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)
}
"#,
    );
}
