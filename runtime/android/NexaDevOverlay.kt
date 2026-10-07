package __NEXA_PACKAGE__

import android.app.Activity
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONObject
import androidx.compose.ui.window.Dialog

internal fun nexaDevColor(value: JSONObject?, isDark: Boolean): Color? {
    value ?: return null
    val wrapped = value.optJSONObject("Static")
    if (wrapped != null && (wrapped.has("Static") || wrapped.has("Adaptive"))) {
        return nexaDevColor(wrapped, isDark)
    }
    val payload = value.optJSONObject("Static")
        ?: value.optJSONObject("Adaptive")?.optJSONObject(if (isDark) "dark" else "light")
        ?: return null
    return Color(
        red = payload.optInt("red", 0) / 255f,
        green = payload.optInt("green", 0) / 255f,
        blue = payload.optInt("blue", 0) / 255f,
        alpha = payload.optInt("alpha", 255) / 255f,
    )
}

internal fun nexaDevHexColor(value: String): Color? {
    val digits = value.removePrefix("#")
    val packed = digits.toLongOrNull(16) ?: return null
    val channels = when (digits.length) {
        6 -> Triple((packed shr 16) and 0xFF, (packed shr 8) and 0xFF, packed and 0xFF)
        8 -> Triple((packed shr 24) and 0xFF, (packed shr 16) and 0xFF, (packed shr 8) and 0xFF)
        else -> return null
    }
    val alpha = if (digits.length == 8) (packed and 0xFF).toFloat() / 255f else 1f
    return Color(
        red = channels.first.toFloat() / 255f,
        green = channels.second.toFloat() / 255f,
        blue = channels.third.toFloat() / 255f,
        alpha = alpha,
    )
}

@Composable
internal fun NexaDevErrorOverlay(
    diagnostics: List<NexaDevDiagnostic>,
    modifier: Modifier = Modifier,
    onOpenInEditor: (NexaDevDiagnostic) -> Unit,
) {
    val first = diagnostics.firstOrNull() ?: return
    var showDetails by remember(diagnostics) { mutableStateOf(false) }
    Surface(
        modifier = modifier.clickable { showDetails = true },
        shape = RoundedCornerShape(16.dp),
        color = Color(0xF2221B1D),
        tonalElevation = 8.dp,
    ) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
            Text("Nexa development error", color = Color.White, fontWeight = FontWeight.SemiBold)
            Text(
                "${first.file.substringAfterLast('/').substringAfterLast('\\')}:${first.line}:${first.column}",
                color = Color(0xFFFFC9C5),
                fontSize = 12.sp,
                modifier = Modifier.padding(top = 2.dp),
            )
            Text(
                first.message,
                color = Color.White,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 4.dp),
            )
            Text(
                if (diagnostics.size == 1) "Tap to inspect and open in editor" else "${diagnostics.size} errors · tap to inspect",
                color = Color(0xFFFFC9C5),
                fontSize = 12.sp,
                modifier = Modifier.padding(top = 6.dp),
            )
        }
    }
    if (showDetails) {
        Dialog(onDismissRequest = { showDetails = false }) {
            Surface(shape = RoundedCornerShape(20.dp), color = Color(0xFF211D23)) {
                Column(
                    Modifier
                        .fillMaxWidth()
                        .heightIn(max = 560.dp)
                        .verticalScroll(rememberScrollState())
                        .padding(20.dp),
                ) {
                    Text("Build diagnostics", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 20.sp)
                    diagnostics.forEach { diagnostic ->
                        Column(Modifier.padding(top = 16.dp)) {
                            Text(
                                "${diagnostic.file}:${diagnostic.line}:${diagnostic.column}",
                                color = Color(0xFFFFC9C5),
                                fontSize = 12.sp,
                            )
                            Text(diagnostic.message, color = Color.White, modifier = Modifier.padding(top = 4.dp))
                            TextButton(onClick = { onOpenInEditor(diagnostic) }) {
                                Text("Open in editor")
                            }
                        }
                    }
                    TextButton(onClick = { showDetails = false }) {
                        Text("Done")
                    }
                }
            }
        }
    }
}

@Composable
internal fun NexaDevPerformanceOverlay(fps: Int, frameTimeMs: Double, modifier: Modifier = Modifier) {
    Row(
        modifier
            .clip(RoundedCornerShape(999.dp))
            .background(Color.Black.copy(alpha = 0.75f))
            .padding(horizontal = 8.dp, vertical = 4.dp),
    ) {
        Text("$fps FPS", color = Color.White, fontSize = 11.sp, modifier = Modifier.padding(end = 8.dp))
        Text(String.format(java.util.Locale.US, "%.1f ms", frameTimeMs), color = Color.White, fontSize = 11.sp)
    }
}

@Composable
internal fun NexaDevStatusBar(config: JSONObject?, isDarkTheme: Boolean) {
    val view = LocalView.current
    val window = (view.context as? Activity)?.window
    val initialStatusBarColor = remember(window) { window?.statusBarColor }
    val initialLightStatusBar = remember(window, view) {
        window?.let { WindowCompat.getInsetsController(it, view).isAppearanceLightStatusBars }
    }
    SideEffect {
        val window = window ?: return@SideEffect
        val controller = WindowCompat.getInsetsController(window, view)
        if (config?.optBoolean("hidden", false) == true) {
            controller.hide(WindowInsetsCompat.Type.statusBars())
        } else {
            controller.show(WindowInsetsCompat.Type.statusBars())
        }
        when (config?.optString("style")) {
            "Light" -> controller.isAppearanceLightStatusBars = false
            "Dark" -> controller.isAppearanceLightStatusBars = true
            else -> controller.isAppearanceLightStatusBars = initialLightStatusBar ?: false
        }
        val color = config?.optJSONObject("background")
        val background = color?.optJSONObject("Static")
            ?: color?.optJSONObject("Adaptive")?.optJSONObject(if (isDarkTheme) "dark" else "light")
        window.statusBarColor = if (background != null) {
            android.graphics.Color.argb(
                background.optInt("alpha", 255),
                background.optInt("red", 0),
                background.optInt("green", 0),
                background.optInt("blue", 0),
            )
        } else {
            initialStatusBarColor ?: android.graphics.Color.TRANSPARENT
        }
    }
}
