//! Core secure string storage backed by Android Keystore and app-private files.

use nexa_codegen::SourceWriter;

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Core secure string storage. Ciphertext lives under noBackupFilesDir,
// while the AES key is non-exportable in Android Keystore.
internal object NexaSecureStorage {
    private const val IV_BYTES = 12
    private const val TAG_BITS = 128
    private const val DIRECTORY = "nexa-secure-storage-v1"
    private val lock = java.util.concurrent.locks.ReentrantReadWriteLock()
    private val keyLock = Any()

    suspend fun get(context: android.content.Context, key: String): String? =
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.readLock().lock()
            try {
                getLocked(context.applicationContext, key)
            } finally {
                lock.readLock().unlock()
            }
        }

    suspend fun set(context: android.content.Context, key: String, value: String) {
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.writeLock().lock()
            try {
                setLocked(context.applicationContext, key, value)
            } finally {
                lock.writeLock().unlock()
            }
        }
    }

    suspend fun delete(context: android.content.Context, key: String) {
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.writeLock().lock()
            try {
                file(context.applicationContext, key, createDirectory = false)?.delete()
            } finally {
                lock.writeLock().unlock()
            }
        }
    }

    suspend fun clear(context: android.content.Context) {
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.writeLock().lock()
            try {
                val directory = directory(context.applicationContext, create = false) ?: return@withContext
                directory.listFiles()?.forEach { entry ->
                    if (entry.isFile && !entry.delete()) {
                        throw java.io.IOException("could not delete secure-storage entry")
                    }
                }
            } finally {
                lock.writeLock().unlock()
            }
        }
    }

    private fun getLocked(context: android.content.Context, key: String): String? {
        val atomic = file(context, key, createDirectory = false) ?: return null
        if (!atomic.baseFile.exists()) return null
        val encrypted = atomic.openRead().use { it.readBytes() }
        if (encrypted.size <= IV_BYTES) throw java.io.IOException("invalid secure-storage record")
        val iv = encrypted.copyOfRange(0, IV_BYTES)
        val ciphertext = encrypted.copyOfRange(IV_BYTES, encrypted.size)
        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(javax.crypto.Cipher.DECRYPT_MODE, secretKey(context), javax.crypto.spec.GCMParameterSpec(TAG_BITS, iv))
        cipher.updateAAD(key.toByteArray(Charsets.UTF_8))
        val plaintext = cipher.doFinal(ciphertext)
        return plaintext.toString(Charsets.UTF_8)
    }

    private fun setLocked(context: android.content.Context, key: String, value: String) {
        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(javax.crypto.Cipher.ENCRYPT_MODE, secretKey(context))
        cipher.updateAAD(key.toByteArray(Charsets.UTF_8))
        val ciphertext = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        val iv = cipher.iv
        if (iv.size != IV_BYTES) throw java.security.GeneralSecurityException("unexpected AES-GCM IV length")
        val encrypted = ByteArray(iv.size + ciphertext.size)
        System.arraycopy(iv, 0, encrypted, 0, iv.size)
        System.arraycopy(ciphertext, 0, encrypted, iv.size, ciphertext.size)

        val atomic = file(context, key, createDirectory = true)
            ?: throw java.io.IOException("could not create secure-storage directory")
        val output = atomic.startWrite()
        try {
            output.write(encrypted)
            atomic.finishWrite(output)
        } catch (failure: Throwable) {
            atomic.failWrite(output)
            throw failure
        } finally {
            encrypted.fill(0)
            ciphertext.fill(0)
        }
    }

    private fun file(
        context: android.content.Context,
        key: String,
        createDirectory: Boolean,
    ): android.util.AtomicFile? {
        val directory = directory(context, createDirectory) ?: return null
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(key.toByteArray(Charsets.UTF_8))
        val name = android.util.Base64.encodeToString(
            digest,
            android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING,
        )
        digest.fill(0)
        return android.util.AtomicFile(java.io.File(directory, name))
    }

    private fun directory(context: android.content.Context, create: Boolean): java.io.File? {
        val directory = java.io.File(context.noBackupFilesDir, DIRECTORY)
        if (directory.isDirectory) return directory
        if (!create) return null
        if (directory.mkdirs() || directory.isDirectory) return directory
        throw java.io.IOException("could not create secure-storage directory")
    }

    private fun secretKey(context: android.content.Context): java.security.Key {
        synchronized(keyLock) {
            val alias = "${context.packageName}.nexa.secure-storage.v1"
            val keyStore = java.security.KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            keyStore.getKey(alias, null)?.let { return it }
            val generator = javax.crypto.KeyGenerator.getInstance(
                android.security.keystore.KeyProperties.KEY_ALGORITHM_AES,
                "AndroidKeyStore",
            )
            generator.init(
                android.security.keystore.KeyGenParameterSpec.Builder(
                    alias,
                    android.security.keystore.KeyProperties.PURPOSE_ENCRYPT or
                        android.security.keystore.KeyProperties.PURPOSE_DECRYPT,
                )
                    .setBlockModes(android.security.keystore.KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(android.security.keystore.KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setKeySize(256)
                    .setRandomizedEncryptionRequired(true)
                    .build(),
            )
            return generator.generateKey()
        }
    }
}
"#,
    );
}
