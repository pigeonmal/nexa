/// Emits the app-facing runtime object.
///
/// The application context itself lives in the fixed core package, because a
/// plugin lives in its own package and still has to reach the application. The
/// object here is the app's view of it: generated code keeps calling
/// `NexaRuntime`, and a plugin can call the core holder directly.
use nexa_codegen::SourceWriter;
pub(crate) fn render(out: &mut SourceWriter, include_permissions: bool) {
    let core = nexa_codegen::value::KOTLIN_CORE_PACKAGE;
    out.push_str(&format!(
        r#"
public object NexaRuntime {{
    public fun bind(context: android.content.Context) = {core}.NexaRuntimeCore.bind(context)

    public fun context(): android.content.Context = {core}.NexaRuntimeCore.context()

"#
    ));
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
