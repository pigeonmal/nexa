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

    suspend fun get(context: android.content.Context, key: String, scope: String? = null): String? =
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.readLock().lock()
            try {
                getLocked(context.applicationContext, key, scope)
            } finally {
                lock.readLock().unlock()
            }
        }

    suspend fun set(context: android.content.Context, key: String, value: String, scope: String? = null) {
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.writeLock().lock()
            try {
                setLocked(context.applicationContext, key, value, scope)
            } finally {
                lock.writeLock().unlock()
            }
        }
    }

    suspend fun delete(context: android.content.Context, key: String, scope: String? = null) {
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.writeLock().lock()
            try {
                file(context.applicationContext, key, createDirectory = false, scope = scope)?.delete()
            } finally {
                lock.writeLock().unlock()
            }
        }
    }

    suspend fun clear(context: android.content.Context, scope: String? = null) {
        kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
            lock.writeLock().lock()
            try {
                val appContext = context.applicationContext
                if (scope != null) {
                    val scoped = scopedDirectory(appContext, scope, create = false) ?: return@withContext
                    if (!scoped.deleteRecursively()) {
                        throw java.io.IOException("could not delete scoped secure-storage entries")
                    }
                } else {
                    val directory = directory(appContext, create = false) ?: return@withContext
                    directory.listFiles()?.forEach { entry ->
                        if (!entry.deleteRecursively()) {
                            throw java.io.IOException("could not delete secure-storage entry")
                        }
                    }
                }
            } finally {
                lock.writeLock().unlock()
            }
        }
    }

    private fun getLocked(context: android.content.Context, key: String, scope: String?): String? {
        val identity = storageIdentity(key, scope)
        val atomic = file(context, key, createDirectory = false, scope = scope) ?: return null
        if (!atomic.baseFile.exists()) return null
        val encrypted = atomic.openRead().use { it.readBytes() }
        if (encrypted.size <= IV_BYTES) throw java.io.IOException("invalid secure-storage record")
        val iv = encrypted.copyOfRange(0, IV_BYTES)
        val ciphertext = encrypted.copyOfRange(IV_BYTES, encrypted.size)
        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(javax.crypto.Cipher.DECRYPT_MODE, secretKey(context), javax.crypto.spec.GCMParameterSpec(TAG_BITS, iv))
        cipher.updateAAD(identity.toByteArray(Charsets.UTF_8))
        val plaintext = cipher.doFinal(ciphertext)
        return plaintext.toString(Charsets.UTF_8)
    }

    private fun setLocked(context: android.content.Context, key: String, value: String, scope: String?) {
        val identity = storageIdentity(key, scope)
        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(javax.crypto.Cipher.ENCRYPT_MODE, secretKey(context))
        cipher.updateAAD(identity.toByteArray(Charsets.UTF_8))
        val ciphertext = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        val iv = cipher.iv
        if (iv.size != IV_BYTES) throw java.security.GeneralSecurityException("unexpected AES-GCM IV length")
        val encrypted = ByteArray(iv.size + ciphertext.size)
        System.arraycopy(iv, 0, encrypted, 0, iv.size)
        System.arraycopy(ciphertext, 0, encrypted, iv.size, ciphertext.size)

        val atomic = file(context, key, createDirectory = true, scope = scope)
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
        scope: String? = null,
    ): android.util.AtomicFile? {
        val directory = scopedDirectory(context, scope, createDirectory) ?: return null
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(storageIdentity(key, scope).toByteArray(Charsets.UTF_8))
        val name = android.util.Base64.encodeToString(
            digest,
            android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING,
        )
        digest.fill(0)
        return android.util.AtomicFile(java.io.File(directory, name))
    }

    private fun storageIdentity(key: String, scope: String?): String =
        scope?.let { "${it.toByteArray(Charsets.UTF_8).size}:$it$key" } ?: key

    private fun scopedDirectory(
        context: android.content.Context,
        scope: String?,
        create: Boolean,
    ): java.io.File? {
        val root = directory(context, create) ?: return null
        if (scope == null) return root
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(scope.toByteArray(Charsets.UTF_8))
        val name = "scope-" + android.util.Base64.encodeToString(
            digest,
            android.util.Base64.URL_SAFE or android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING,
        )
        digest.fill(0)
        val scoped = java.io.File(root, name)
        if (scoped.isDirectory) return scoped
        if (!create) return null
        if (scoped.mkdirs() || scoped.isDirectory) return scoped
        throw java.io.IOException("could not create scoped secure-storage directory")
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
