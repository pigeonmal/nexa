/// Emits the app-facing runtime object.
///
/// The application context itself lives in the fixed core package, because a
/// plugin lives in its own package and still has to reach the application. The
/// object here is the app's view of it: generated code keeps calling
/// `NexaRuntime`, and a plugin can call the core holder directly.
use nexa_codegen::SourceWriter;
pub(crate) fn render(out: &mut SourceWriter, include_permissions: bool, include_app_icon: bool) {
    let core = nexa_codegen::value::KOTLIN_CORE_PACKAGE;
    out.push_str(&format!(
        r#"
public object NexaRuntime {{
    public fun bind(context: android.content.Context) = {core}.NexaRuntimeCore.bind(context)

    public fun context(): android.content.Context = {core}.NexaRuntimeCore.context()

"#
    ));
    if include_app_icon {
        out.push_str(
            r#"    @Suppress("DEPRECATION")
    public fun setAlternateAppIcon(name: String?): Boolean {
        val applicationContext = context().applicationContext
        val packageManager = applicationContext.packageManager
        val packageName = applicationContext.packageName
        val target = "$packageName.NexaIcon${name ?: "Default"}"
        val activities = if (android.os.Build.VERSION.SDK_INT >= 33) {
            packageManager.getPackageInfo(
                packageName,
                android.content.pm.PackageManager.PackageInfoFlags.of(
                    (android.content.pm.PackageManager.GET_ACTIVITIES or
                        android.content.pm.PackageManager.MATCH_DISABLED_COMPONENTS).toLong(),
                ),
            ).activities
        } else {
            packageManager.getPackageInfo(
                packageName,
                android.content.pm.PackageManager.GET_ACTIVITIES or
                    android.content.pm.PackageManager.MATCH_DISABLED_COMPONENTS,
            ).activities
        } ?: return false
        if (activities.none { it.name == target }) return false
        val prefix = "$packageName.NexaIcon"
        dev.nexa.core.NexaRuntimeCore.whenAppBackgrounded {
            activities.filter { it.name.startsWith(prefix) }.forEach { activity ->
                val enabled = activity.name == target
                packageManager.setComponentEnabledSetting(
                    android.content.ComponentName(packageName, activity.name),
                    if (enabled) android.content.pm.PackageManager.COMPONENT_ENABLED_STATE_ENABLED
                    else android.content.pm.PackageManager.COMPONENT_ENABLED_STATE_DISABLED,
                    android.content.pm.PackageManager.DONT_KILL_APP,
                )
            }
        }
        return true
    }

"#,
        );
    }
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
