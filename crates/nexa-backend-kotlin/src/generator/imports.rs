use super::features::Features;

pub(super) fn render(
    features: &Features,
    has_navigation: bool,
    has_direction: bool,
    has_on_appear: bool,
    has_on_disappear: bool,
    out: &mut String,
) {
    let mut imports = Vec::with_capacity(40);
    let mut add = |enabled: bool, import: &'static str| {
        if enabled {
            imports.push(import);
        }
    };

    add(features.uses_status_bar, "android.app.Activity");
    add(features.uses_status_bar, "android.view.View");
    add(
        has_direction,
        "androidx.compose.runtime.CompositionLocalProvider",
    );
    add(has_on_appear, "androidx.compose.runtime.LaunchedEffect");
    add(
        has_on_disappear,
        "androidx.compose.runtime.DisposableEffect",
    );
    add(features.uses_link, "android.content.Intent");
    add(features.uses_link, "android.net.Uri");
    add(
        features.uses_status_bar,
        "androidx.compose.runtime.SideEffect",
    );
    add(
        features.uses_status_bar,
        "androidx.compose.ui.platform.LocalView",
    );
    add(
        features.uses_bottom_sheet,
        "androidx.compose.material3.ModalBottomSheet",
    );
    add(
        features.uses_bottom_bar,
        "androidx.compose.material3.NavigationBar",
    );
    add(
        features.uses_bottom_bar,
        "androidx.compose.material3.NavigationBarItem",
    );
    add(
        features.uses_bottom_bar,
        "androidx.compose.material3.Scaffold",
    );
    add(
        features.uses_refresh_control,
        "androidx.compose.material3.pulltorefresh.PullToRefreshBox",
    );

    add(
        features.uses_background,
        "androidx.compose.foundation.background",
    );
    add(features.uses_border, "androidx.compose.foundation.border");
    add(
        features.uses_clickable || features.uses_link,
        "androidx.compose.foundation.clickable",
    );
    add(
        features.uses_long_press,
        "androidx.compose.foundation.combinedClickable",
    );
    add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.rememberScrollState",
    );
    add(
        features.uses_refresh_scroll,
        "androidx.compose.foundation.rememberScrollState",
    );
    add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.verticalScroll",
    );
    add(
        features.uses_refresh_scroll,
        "androidx.compose.foundation.verticalScroll",
    );
    add(
        features.uses_adaptive_color,
        "androidx.compose.foundation.isSystemInDarkTheme",
    );
    add(
        features.uses_list,
        "androidx.compose.foundation.lazy.LazyColumn",
    );
    add(features.uses_list, "androidx.compose.foundation.lazy.items");

    add(features.uses_box, "androidx.compose.foundation.layout.Box");
    add(features.uses_alignment, "androidx.compose.ui.Alignment");
    add(
        features.uses_column,
        "androidx.compose.foundation.layout.Column",
    );
    add(features.uses_row, "androidx.compose.foundation.layout.Row");
    add(
        features.uses_arrangement,
        "androidx.compose.foundation.layout.Arrangement",
    );
    add(
        features.uses_padding,
        "androidx.compose.foundation.layout.padding",
    );
    add(
        features.uses_width,
        "androidx.compose.foundation.layout.width",
    );
    add(
        features.uses_height,
        "androidx.compose.foundation.layout.height",
    );
    add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.imePadding",
    );

    add(
        features.uses_corner_radius,
        "androidx.compose.foundation.shape.RoundedCornerShape",
    );
    add(
        features.uses_text_input,
        "androidx.compose.foundation.text.KeyboardOptions",
    );
    add(
        features.uses_text_input_submit,
        "androidx.compose.foundation.text.KeyboardActions",
    );
    add(features.uses_button, "androidx.compose.material3.Button");
    add(
        features.uses_button_loading,
        "androidx.compose.material3.CircularProgressIndicator",
    );
    add(features.uses_switch, "androidx.compose.material3.Switch");
    add(features.uses_text, "androidx.compose.material3.Text");
    add(
        features.uses_text_input,
        "androidx.compose.material3.TextField",
    );
    add(
        features.uses_font_weight,
        "androidx.compose.ui.text.font.FontWeight",
    );
    add(
        features.uses_selectable_text,
        "androidx.compose.foundation.text.selection.SelectionContainer",
    );

    add(
        features.uses_mutable_state,
        "androidx.compose.runtime.getValue",
    );
    add(
        features.uses_mutable_state,
        "androidx.compose.runtime.setValue",
    );
    add(
        features.uses_mutable_state,
        "androidx.compose.runtime.remember",
    );
    add(
        features.uses_mutable_int_state,
        "androidx.compose.runtime.mutableIntStateOf",
    );
    add(
        features.uses_mutable_long_state,
        "androidx.compose.runtime.mutableLongStateOf",
    );
    add(
        features.uses_mutable_float_state,
        "androidx.compose.runtime.mutableFloatStateOf",
    );
    add(
        features.uses_mutable_generic_state,
        "androidx.compose.runtime.mutableStateOf",
    );
    add(true, "androidx.compose.runtime.Composable");

    add(features.uses_modifier, "androidx.compose.ui.Modifier");
    add(
        features.uses_regular_width,
        "androidx.compose.ui.platform.LocalConfiguration",
    );
    add(
        features.uses_link,
        "androidx.compose.ui.platform.LocalContext",
    );
    add(
        has_direction,
        "androidx.compose.ui.platform.LocalLayoutDirection",
    );
    add(has_direction, "androidx.compose.ui.unit.LayoutDirection");
    add(features.uses_opacity, "androidx.compose.ui.draw.alpha");
    add(features.uses_corner_radius, "androidx.compose.ui.draw.clip");
    add(features.uses_color, "androidx.compose.ui.graphics.Color");
    add(
        features.uses_image,
        "androidx.compose.ui.layout.ContentScale",
    );
    add(
        features.uses_pressable || features.uses_accessibility_role,
        "androidx.compose.ui.semantics.Role",
    );
    add(
        features.uses_accessibility_heading,
        "androidx.compose.ui.semantics.heading",
    );
    add(
        features.uses_switch || features.uses_accessibility,
        "androidx.compose.ui.semantics.contentDescription",
    );
    add(
        features.uses_switch || features.uses_accessibility,
        "androidx.compose.ui.semantics.semantics",
    );
    add(
        features.uses_capitalization,
        "androidx.compose.ui.text.input.KeyboardCapitalization as NativeKeyboardCapitalization",
    );
    add(
        features.uses_text_input,
        "androidx.compose.ui.text.input.KeyboardType as NativeKeyboardType",
    );
    add(
        features.uses_text_input_submit,
        "androidx.compose.ui.text.input.ImeAction",
    );
    add(
        features.uses_secure_text_input,
        "androidx.compose.ui.text.input.PasswordVisualTransformation",
    );
    add(features.uses_dp, "androidx.compose.ui.unit.dp");
    add(features.uses_text_sp, "androidx.compose.ui.unit.sp");
    add(
        features.uses_placeholder,
        "androidx.compose.ui.res.painterResource",
    );
    add(features.uses_image, "coil3.compose.AsyncImage");
    add(features.uses_remote_image, "android.content.Context");
    add(features.uses_remote_image, "android.net.Uri");
    add(
        features.uses_remote_image,
        "androidx.compose.runtime.remember",
    );
    add(
        features.uses_remote_image,
        "androidx.compose.ui.platform.LocalContext",
    );
    add(features.uses_remote_image, "coil3.ImageLoader");
    add(features.uses_remote_image, "coil3.network.NetworkClient");
    add(features.uses_remote_image, "coil3.network.NetworkFetcher");
    add(features.uses_remote_image, "coil3.network.NetworkHeaders");
    add(features.uses_remote_image, "coil3.network.NetworkRequest");
    add(features.uses_remote_image, "coil3.network.NetworkResponse");
    add(
        features.uses_remote_image,
        "coil3.network.NetworkResponseBody",
    );
    add(
        features.uses_remote_image,
        "kotlinx.coroutines.CancellationException",
    );
    add(features.uses_remote_image, "kotlinx.coroutines.Dispatchers");
    add(
        features.uses_remote_image,
        "kotlinx.coroutines.suspendCancellableCoroutine",
    );
    add(features.uses_remote_image, "kotlinx.coroutines.withContext");
    add(features.uses_remote_image, "kotlinx.coroutines.withTimeout");
    add(features.uses_remote_image, "okio.Buffer");
    add(features.uses_remote_image, "org.chromium.net.CronetEngine");
    add(
        features.uses_remote_image,
        "org.chromium.net.UploadDataProvider",
    );
    add(
        features.uses_remote_image,
        "org.chromium.net.UploadDataSink",
    );
    add(features.uses_remote_image, "org.chromium.net.UrlRequest");
    add(
        features.uses_remote_image,
        "org.chromium.net.UrlResponseInfo",
    );
    add(features.uses_remote_image, "java.io.ByteArrayOutputStream");
    add(features.uses_remote_image, "java.io.File");
    add(features.uses_remote_image, "java.io.FileOutputStream");
    add(features.uses_remote_image, "java.nio.ByteBuffer");
    add(features.uses_remote_image, "java.util.Date");
    add(features.uses_remote_image, "java.util.WeakHashMap");
    add(features.uses_remote_image, "java.util.concurrent.Executors");
    add(features.uses_remote_image, "kotlin.coroutines.resume");
    add(
        features.uses_remote_image,
        "kotlin.coroutines.resumeWithException",
    );
    add(
        features.uses_navigation_link,
        "androidx.compose.material3.TextButton",
    );
    add(has_navigation, "androidx.navigation.compose.NavHost");
    add(has_navigation, "androidx.navigation.compose.composable");
    add(
        has_navigation,
        "androidx.navigation.compose.rememberNavController",
    );

    imports.sort_unstable();
    imports.dedup();
    for import in imports {
        out.push_str("import ");
        out.push_str(import);
        out.push('\n');
    }
    out.push('\n');
}
