package __NEXA_PACKAGE__

import android.app.Activity
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONObject

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
