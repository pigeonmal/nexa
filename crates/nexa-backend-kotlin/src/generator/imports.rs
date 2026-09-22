use super::features::Features;

pub(super) fn render(
    features: &Features,
    has_navigation: bool,
    has_direction: bool,
    has_on_appear: bool,
    has_on_disappear: bool,
    has_lifecycle_events: bool,
    out: &mut String,
) {
    let mut imports = Vec::with_capacity(40);
    let mut add = |enabled: bool, import: &'static str| {
        if enabled {
            imports.push(import);
        }
    };

    add(features.uses_status_bar, "android.app.Activity");
    add(features.uses_haptic, "android.view.HapticFeedbackConstants");
    add(features.uses_status_bar, "androidx.core.view.WindowCompat");
    add(
        features.uses_status_bar,
        "androidx.core.view.WindowInsetsCompat",
    );
    add(
        has_direction,
        "androidx.compose.runtime.CompositionLocalProvider",
    );
    add(
        has_on_appear
            || features.uses_focus
            || features.uses_list_end_reached
            || features.uses_list_scroll_position
            || features.uses_list_scroll_events,
        "androidx.compose.runtime.LaunchedEffect",
    );
    add(
        features.uses_list_end_reached
            || features.uses_list_scroll_position
            || features.uses_list_scroll_events,
        "androidx.compose.runtime.snapshotFlow",
    );
    add(
        has_on_disappear || has_lifecycle_events,
        "androidx.compose.runtime.DisposableEffect",
    );
    add(
        has_lifecycle_events,
        "androidx.lifecycle.compose.LocalLifecycleOwner",
    );
    add(has_lifecycle_events, "androidx.lifecycle.Lifecycle");
    add(
        has_lifecycle_events,
        "androidx.lifecycle.LifecycleEventObserver",
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
        features.uses_bottom_sheet_partial,
        "androidx.compose.material3.rememberModalBottomSheetState",
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
    add(
        features.uses_tab_badge_placeholder,
        "androidx.compose.foundation.layout.Box",
    );
    add(
        features.uses_tab_badge_placeholder,
        "androidx.compose.foundation.layout.size",
    );
    add(
        features.uses_animation,
        "androidx.compose.animation.animateContentSize",
    );
    add(
        features.uses_animation_ease_in,
        "androidx.compose.animation.core.FastOutLinearInEasing",
    );
    add(
        features.uses_animation_ease_in_out,
        "androidx.compose.animation.core.FastOutSlowInEasing",
    );
    add(
        features.uses_animation_linear,
        "androidx.compose.animation.core.LinearEasing",
    );
    add(
        features.uses_animation_ease_out,
        "androidx.compose.animation.core.LinearOutSlowInEasing",
    );
    add(
        features.uses_animation_spring,
        "androidx.compose.animation.core.spring",
    );
    add(
        features.uses_animation_ease_in
            || features.uses_animation_ease_out
            || features.uses_animation_ease_in_out
            || features.uses_animation_linear,
        "androidx.compose.animation.core.tween",
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
    add(
        features.uses_linear_list_end_reached
            || features.uses_linear_list_scroll_position
            || features.uses_linear_list_scroll_events,
        "androidx.compose.foundation.lazy.rememberLazyListState",
    );
    add(
        features.uses_horizontal_list,
        "androidx.compose.foundation.lazy.LazyRow",
    );
    add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.grid.LazyVerticalGrid",
    );
    add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.grid.GridCells",
    );
    add(
        features.uses_grid_end_reached
            || features.uses_grid_scroll_position
            || features.uses_grid_scroll_events,
        "androidx.compose.foundation.lazy.grid.rememberLazyGridState",
    );
    add(
        features.uses_linear_list,
        "androidx.compose.foundation.lazy.items",
    );
    add(
        features.uses_sticky_header,
        "androidx.compose.foundation.lazy.stickyHeader",
    );
    add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.grid.items",
    );
    add(
        features.uses_list_end_reached
            || features.uses_list_scroll_position
            || features.uses_list_scroll_events,
        "kotlinx.coroutines.flow.collect",
    );

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
        features.uses_width_in,
        "androidx.compose.foundation.layout.widthIn",
    );
    add(
        features.uses_height_in,
        "androidx.compose.foundation.layout.heightIn",
    );
    add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.imePadding",
    );
    add(
        features.uses_keyboard_interactive,
        "androidx.compose.foundation.layout.imeNestedScroll",
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
        features.uses_focus,
        "androidx.compose.ui.focus.FocusRequester",
    );
    add(
        features.uses_focus,
        "androidx.compose.ui.focus.focusRequester",
    );
    add(
        features.uses_focus,
        "androidx.compose.ui.focus.onFocusChanged",
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
        features.uses_mutable_state || features.uses_list_end_reached,
        "androidx.compose.runtime.getValue",
    );
    add(
        features.uses_mutable_state || features.uses_list_end_reached,
        "androidx.compose.runtime.setValue",
    );
    add(
        features.uses_mutable_state
            || features.uses_mutable_collection
            || features.uses_list_end_reached,
        "androidx.compose.runtime.remember",
    );
    add(
        features.uses_mutable_list,
        "androidx.compose.runtime.mutableStateListOf",
    );
    add(
        features.uses_mutable_set,
        "androidx.compose.runtime.mutableStateSetOf",
    );
    add(
        features.uses_mutable_map,
        "androidx.compose.runtime.mutableStateMapOf",
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
    add(features.uses_asset, "androidx.compose.foundation.Image");
    add(
        features.uses_tab_icon || features.uses_button_icon,
        "androidx.compose.material3.Icon",
    );
    add(features.uses_tab_badge, "androidx.compose.material3.Badge");
    add(
        features.uses_tab_badge,
        "androidx.compose.material3.BadgedBox",
    );
    add(
        features.uses_size_class,
        "androidx.compose.ui.platform.LocalConfiguration",
    );
    add(
        features.uses_link,
        "androidx.compose.ui.platform.LocalContext",
    );
    add(
        features.uses_haptic,
        "androidx.compose.ui.platform.LocalView",
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
        features.uses_asset || features.uses_remote_image,
        "androidx.compose.ui.layout.ContentScale",
    );
    add(
        features.uses_pressable || features.uses_accessibility_role,
        "androidx.compose.ui.semantics.Role",
    );
    add(
        features.uses_accessibility_role,
        "androidx.compose.ui.semantics.role",
    );
    add(
        features.uses_accessibility_heading,
        "androidx.compose.ui.semantics.heading",
    );
    add(
        features.uses_accessibility_hint,
        "androidx.compose.ui.semantics.hintText",
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
    add(
        features.uses_tab_badge_placeholder,
        "androidx.compose.ui.unit.dp",
    );
    add(features.uses_text_sp, "androidx.compose.ui.unit.sp");
    add(
        features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder,
        "androidx.compose.ui.graphics.Color",
    );
    add(
        features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder,
        "androidx.compose.ui.graphics.painter.ColorPainter",
    );
    add(
        features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder,
        "androidx.compose.ui.graphics.painter.Painter",
    );
    add(
        features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder,
        "androidx.compose.ui.res.painterResource",
    );
    add(features.uses_image, "coil3.compose.AsyncImage");
    add(
        features.uses_network_api || features.uses_path_api || features.uses_permissions,
        "android.content.Context",
    );
    add(features.uses_network_api, "android.net.Uri");
    add(
        features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder
            || features.uses_remote_image,
        "androidx.compose.runtime.remember",
    );
    add(
        features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder
            || features.uses_network_api
            || features.uses_path_api
            || features.uses_permissions,
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
        features.uses_network_api,
        "kotlinx.coroutines.CancellationException",
    );
    add(
        features.uses_network_api || features.uses_file_async,
        "kotlinx.coroutines.Dispatchers",
    );
    add(
        features.uses_network_api,
        "kotlinx.coroutines.suspendCancellableCoroutine",
    );
    add(
        features.uses_network_api || features.uses_file_async,
        "kotlinx.coroutines.withContext",
    );
    add(features.uses_network_api, "kotlinx.coroutines.withTimeout");
    add(features.uses_remote_image, "okio.Buffer");
    add(features.uses_network_api, "org.chromium.net.CronetEngine");
    add(
        features.uses_network_api,
        "org.chromium.net.UploadDataProvider",
    );
    add(features.uses_network_api, "org.chromium.net.UploadDataSink");
    add(features.uses_network_api, "org.chromium.net.UrlRequest");
    add(
        features.uses_network_api,
        "org.chromium.net.UrlResponseInfo",
    );
    add(features.uses_network_api, "java.io.ByteArrayOutputStream");
    add(
        features.uses_network_api || features.uses_path_api || features.uses_file_api,
        "java.io.File",
    );
    add(features.uses_network_api, "java.io.FileOutputStream");
    add(features.uses_network_api, "java.nio.ByteBuffer");
    add(features.uses_network_api, "java.util.Date");
    add(features.uses_network_api, "java.util.WeakHashMap");
    add(features.uses_network_api, "java.util.concurrent.Executors");
    add(features.uses_network_api, "kotlin.coroutines.resume");
    add(
        features.uses_network_api,
        "kotlin.coroutines.resumeWithException",
    );
    add(
        features.uses_navigation_link || features.uses_navigation_back,
        "androidx.compose.material3.TextButton",
    );
    add(
        features.uses_navigation_link || features.uses_navigation_back,
        "androidx.navigation.NavHostController",
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
