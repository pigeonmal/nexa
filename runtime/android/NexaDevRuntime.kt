package __NEXA_PACKAGE__

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.isActive
import org.json.JSONArray

@Composable
internal fun NexaDevRuntimeRoot(serverURL: String, sessionToken: String) {
    val applicationContext = LocalContext.current.applicationContext
    val store = remember(applicationContext) { NexaDevStateStore(applicationContext) }
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { result -> NexaRuntime.dispatchPermissionResult(result) }
    SideEffect { NexaRuntime.bindPermissionLauncher(permissionLauncher) }
    LaunchedEffect(serverURL, sessionToken) {
        NexaDevSocketClient(serverURL, sessionToken, store).connect()
    }
    val module = store.module
    val latestModule = rememberUpdatedState(module)
    LaunchedEffect(store.appLifecycleEpoch) {
        if (store.appLifecycleEpoch == 0) return@LaunchedEffect
        val readyModule = snapshotFlow { store.module }.filterNotNull().first()
        val actions = readyModule.optJSONArray("on_appear") ?: JSONArray()
        if (readyModule.optBoolean("on_appear_async")) {
            store.performAsync(actions, "app", emptyMap())
        } else {
            store.perform(actions, "app", emptyMap())
        }
    }
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner, store.appLifecycleEpoch) {
        val observer = LifecycleEventObserver { _, event ->
            val readyModule = latestModule.value ?: return@LifecycleEventObserver
            val actions = when (event) {
                Lifecycle.Event.ON_RESUME -> readyModule.optJSONArray("on_active")
                Lifecycle.Event.ON_PAUSE -> readyModule.optJSONArray("on_inactive")
                Lifecycle.Event.ON_STOP -> readyModule.optJSONArray("on_background")
                else -> null
            }
            actions?.let { store.perform(it, "app", emptyMap()) }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    DisposableEffect(lifecycleOwner) {
        onDispose {
            latestModule.value?.optJSONArray("on_disappear")?.let { store.perform(it, "app", emptyMap()) }
        }
    }
    var fps by remember { mutableIntStateOf(0) }
    var frameTimeMs by remember { mutableStateOf(0.0) }
    LaunchedEffect(store.performanceOverlayEnabled) {
        if (!store.performanceOverlayEnabled) return@LaunchedEffect
        var frameCount = 0
        var sampleStart = 0L
        var previousFrame = 0L
        while (isActive) {
            val frame = withFrameNanos { it }
            if (previousFrame != 0L) frameCount++
            if (sampleStart == 0L) sampleStart = frame
            if (previousFrame != 0L) frameTimeMs = (frame - previousFrame) / 1_000_000.0
            val elapsed = frame - sampleStart
            if (elapsed >= 1_000_000_000L) {
                fps = (frameCount * 1_000_000_000.0 / elapsed).toInt()
                frameTimeMs = if (frameCount > 0) elapsed / 1_000_000.0 / frameCount else 0.0
                frameCount = 0
                sampleStart = frame
            }
            previousFrame = frame
        }
    }
    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize()) {
            if (store.diagnostics.isNotEmpty()) {
                Text(store.diagnostics.joinToString("\n"), color = Color.Red, modifier = Modifier.padding(10.dp))
            }
            if (module == null) {
                Text("Connecting to Nexa…")
            } else {
                val statusBar = module.optJSONObject("status_bar")
                NexaDevStatusBar(statusBar, isSystemInDarkTheme())
                val style = module.optJSONObject("direction")?.optString("style")
                val direction = when (style) {
                    "Rtl" -> LayoutDirection.Rtl
                    "Ltr" -> LayoutDirection.Ltr
                    else -> LocalLayoutDirection.current
                }
                CompositionLocalProvider(LocalLayoutDirection provides direction) {
                    NexaDevNodeList(
                        module.optJSONArray("body") ?: JSONArray(),
                        module,
                        store,
                        modifier = Modifier.fillMaxSize(),
                    )
                }
            }
        }
        if (store.performanceOverlayEnabled) {
            NexaDevPerformanceOverlay(fps, frameTimeMs, Modifier.align(Alignment.TopEnd).padding(8.dp))
        }
    }
}
