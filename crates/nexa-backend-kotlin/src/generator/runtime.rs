/// Emits the tiny application-context holder shared by generated native APIs.
pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
public object NexaRuntime {
    @Volatile private var applicationContext: android.content.Context? = null
    @Volatile private var activity: android.app.Activity? = null
    private val nextPermissionRequest = java.util.concurrent.atomic.AtomicInteger(0x4E58)
    private val permissionCallbacks = java.util.concurrent.ConcurrentHashMap<Int, (IntArray) -> Unit>()

    public fun bind(context: android.content.Context) {
        val application = context.applicationContext
        if (applicationContext !== application) applicationContext = application
        if (context is android.app.Activity) activity = context
    }

    public fun context(): android.content.Context = requireNotNull(applicationContext) {
        "NexaRuntime.bind must run before a native API call"
    }

    public suspend fun requestPermissions(permissions: Array<String>): IntArray {
        if (permissions.isEmpty()) return IntArray(0)
        val host = requireNotNull(activity) {
            "NexaRuntime.bind must receive an Activity before requesting permissions"
        }
        return kotlinx.coroutines.suspendCancellableCoroutine { continuation ->
            val requestCode = nextPermissionRequest.getAndIncrement()
            permissionCallbacks[requestCode] = { grantResults ->
                if (continuation.isActive) continuation.resumeWith(Result.success(grantResults))
            }
            continuation.invokeOnCancellation {
                permissionCallbacks.remove(requestCode)
            }
            androidx.core.app.ActivityCompat.requestPermissions(host, permissions, requestCode)
        }
    }

    public fun dispatchPermissionResult(
        requestCode: Int,
        grantResults: IntArray,
    ) {
        permissionCallbacks.remove(requestCode)?.invoke(grantResults)
    }
}
"#,
    );
}
