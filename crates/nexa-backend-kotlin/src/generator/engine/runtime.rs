/// Emits the tiny application-context holder shared by generated native APIs.
pub(crate) fn render(out: &mut String, include_permissions: bool) {
    out.push_str(
        r#"
public object NexaRuntime {
    @Volatile private var applicationContext: android.content.Context? = null

    public fun bind(context: android.content.Context) {
        val application = context.applicationContext
        if (applicationContext !== application) applicationContext = application
    }

    public fun context(): android.content.Context = requireNotNull(applicationContext) {
        "NexaRuntime.bind must run before a native API call"
    }

"#,
    );
    if include_permissions {
        out.push_str(
            r#"    @Volatile private var permissionLauncher: androidx.activity.result.ActivityResultLauncher<Array<String>>? = null
    @Volatile private var permissionCallback: ((Map<String, Boolean>) -> Unit)? = null
    private val permissionLock = Any()

    public fun bindPermissionLauncher(
        launcher: androidx.activity.result.ActivityResultLauncher<Array<String>>,
    ) {
        permissionLauncher = launcher
    }

    public suspend fun requestPermissions(permissions: Array<String>): Map<String, Boolean> {
        if (permissions.isEmpty()) return emptyMap()
        val launcher = requireNotNull(permissionLauncher) {
            "NexaRuntime.bindPermissionLauncher must run before requesting permissions"
        }
        return kotlinx.coroutines.suspendCancellableCoroutine { continuation ->
            val callback: (Map<String, Boolean>) -> Unit = { result ->
                if (continuation.isActive) continuation.resumeWith(Result.success(result))
            }
            val accepted = synchronized(permissionLock) {
                if (permissionCallback != null) {
                    false
                } else {
                    permissionCallback = callback
                    true
                }
            }
            if (!accepted) {
                continuation.resumeWithException(IllegalStateException("a permission request is already active"))
                return@suspendCancellableCoroutine
            }
            continuation.invokeOnCancellation {
                synchronized(permissionLock) {
                    if (permissionCallback === callback) permissionCallback = null
                }
            }
            if (continuation.isActive) launcher.launch(permissions)
        }
    }

    public fun dispatchPermissionResult(result: Map<String, Boolean>) {
        val callback = synchronized(permissionLock) {
            val current = permissionCallback
            permissionCallback = null
            current
        }
        callback?.invoke(result)
    }
"#,
        );
    }
    out.push_str(
        r#"}
"#,
    );
}
