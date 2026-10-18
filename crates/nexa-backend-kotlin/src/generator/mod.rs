use nexa_codegen::{GeneratedSources, SourceUnits, SourceWriter};
use nexa_ir::{LayoutKind, Module, ViewStyle};

mod api;
mod components;
mod engine;
mod widget;

use crate::generator::engine::types::kotlin_type;
pub(super) use api::{network, number, permissions};

// Re-exported through `generator` because `engine` is private and the crate
// root cannot name a path through it.
use components::node_renderer as component_renderer;
pub(super) use components::{
    accessibility, assets, bottom_bar, controls, custom_components, dialogs, images, input,
    keyboard, layout, links, lists, navigation, refresh, sheets, system_icons,
};
pub use engine::kotlin_scalar_types;
pub(super) use engine::{
    colors, expressions, features, functions, runtime, state, structs, utils, value,
};

const NEXA_DYNAMIC_COLOR_HELPER: &str = r##"internal fun nexaColorFromHex(hex: String): androidx.compose.ui.graphics.Color {
    val digits = hex.removePrefix("#")
    val hasAlpha = digits.length == 8
    val parsed = if (digits.length == 6 || hasAlpha) digits.toLongOrNull(16) else null
    val color = parsed ?: 0xD63031L
    val red = if (hasAlpha && parsed != null) (color shr 24) and 0xFF else (color shr 16) and 0xFF
    val green = if (hasAlpha && parsed != null) (color shr 16) and 0xFF else (color shr 8) and 0xFF
    val blue = if (hasAlpha && parsed != null) (color shr 8) and 0xFF else color and 0xFF
    val alpha = if (hasAlpha && parsed != null) (color and 0xFF).toFloat() / 255f else 1f
    return androidx.compose.ui.graphics.Color(red.toFloat() / 255f, green.toFloat() / 255f, blue.toFloat() / 255f, alpha)
}

"##;

fn project_features_from_analysis(
    module: &Module,
    features: &features::Features,
) -> crate::KotlinProjectFeatures {
    crate::KotlinProjectFeatures {
        uses_network: features.uses_network_transport(),
        uses_network_connectivity: features.uses_network_connectivity,
        uses_remote_image: features.uses_remote_image,
        uses_system_icons: !features.facts.ui.system_icons.is_empty()
            || features.uses_picker
            || features.uses_navigation_link,
        uses_coroutines: features.uses_network_transport()
            || features.uses_file_async
            || features.uses_permission_request
            || features.uses_tasks
            || features.facts.capabilities.uses_secure_storage_api
            || !module.widgets.is_empty()
            || !module.background_tasks.is_empty(),
        uses_permission_request: features.uses_permission_request,
        uses_navigation: !module.screens.is_empty(),
        uses_bottom_bar: features.uses_adaptive_tabs,
        uses_expressive_menus: features.uses_picker || features.uses_context_menu,
        uses_expressive_motion: features.uses_segmented_control,
        uses_compose_animation: features.uses_conditional_transition
            || features.uses_shared_elements
            || features.uses_segmented_control
            || state::module_uses_scoped_animation(module),
        uses_compose_graphics: features.uses_color
            || features.uses_asset
            || features.uses_placeholder,
        uses_lifecycle_events: module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some()
            || !module.widgets.is_empty(),
        uses_background_tasks: !module.background_tasks.is_empty(),
    }
}

/// Generates the release source as one concatenated string.
pub(super) fn generate(module: &Module) -> String {
    let features = features::Features::analyze(module);
    join_units(generate_units(module, &features).into_files(&[], ""))
}

/// Generates the release source as separate compile units.
///
/// Returns the pieces rather than finished files so the caller can place the
/// `package` declaration and plugin imports ahead of the import block.
pub(super) fn generate_units(module: &Module, features: &features::Features) -> GeneratedSources {
    generate_with_analysis(module, features, false)
}

pub(super) fn generate_widget_units(
    module: &Module,
) -> Result<crate::WidgetGeneratedSources, crate::WidgetGenerationError> {
    widget::generate(module)
}

pub(super) fn generate_units_with_project_features(
    module: &Module,
) -> (GeneratedSources, crate::KotlinProjectFeatures) {
    let features = features::Features::analyze(module);
    let project_features = project_features_from_analysis(module, &features);
    let generated = generate_units(module, &features);
    (generated, project_features)
}

pub(super) fn generate_for_dev_units_with_project_features(
    module: &Module,
) -> (GeneratedSources, crate::KotlinProjectFeatures) {
    let mut features = features::Features::analyze(module);
    // DevRuntime accepts hot-reloaded trees with remote images even when the
    // initial source has none, so the development host always carries the same
    // Coil/Cronet support that release output uses for remote images.
    features.uses_remote_image = true;
    // Local image assets can also appear after the development host is built.
    features.uses_asset = true;
    features.uses_placeholder = true;
    // Shared-transition APIs can be introduced by hot reload without
    // rebuilding the development host.
    features.uses_shared_elements = true;
    // Appearance can be introduced by a hot reload without a native rebuild.
    features.facts.ui.appearance = true;
    // Tab bars can be added or replaced by a DevRuntime module after the host
    // is built, so include their Compose NavigationSuite primitive/dependency.
    features.uses_adaptive_tabs = true;
    // A virtualized list can be introduced by a hot reload after host build.
    features.uses_fast_list = true;
    features.facts.ui.lists.swipe_actions = true;
    // Async native calls can be added while a dev session is running, so keep
    // the same first-party Cronet adapter available even if the initial app
    // does not call Network yet.
    features.uses_network_api = true;
    // File and Path calls can be introduced by a hot reload after the first
    // native build, so keep the same first-party helpers available in dev.
    features.uses_path_api = true;
    features.uses_file_api = true;
    features.uses_file_async = true;
    features.facts.capabilities.uses_crypto_api = true;
    // JSON is interpreted by NexaDevValueCodec using type descriptors from
    // every hot-reloaded module. Do not emit static JSON helpers for a module
    // that does not declare NexaJsonError or concrete JSON value codecs yet.
    // SecureStorage calls can be added after the dev host is built, so the
    // Android Keystore adapter must already be part of every development host.
    features.facts.capabilities.uses_secure_storage_api = true;
    features.facts.capabilities.uses_storage_api = true;
    features.facts.capabilities.uses_regex_api = true;
    // NexaDevNativeApis dispatches AppIcon calls even when the initial module
    // does not use them, so the host runtime must keep the typed adapter ready
    // for a hot-reloaded call.
    features.facts.capabilities.uses_app_icon_api = true;
    features.uses_permissions = true;
    features.uses_permission_request = true;
    features.dynamic_permission = true;
    features.used_permissions = [
        nexa_ir::Permission::Camera,
        nexa_ir::Permission::Microphone,
        nexa_ir::Permission::Photos,
        nexa_ir::Permission::Location,
        nexa_ir::Permission::Notifications,
        nexa_ir::Permission::Contacts,
        nexa_ir::Permission::Calendar,
        nexa_ir::Permission::Bluetooth,
        nexa_ir::Permission::Motion,
    ]
    .into_iter()
    .collect();
    features.expose_permissions_to_dev_runtime = true;
    features.uses_native_library = true;
    let project_features = project_features_from_analysis(module, &features);
    let generated = generate_with_analysis(module, &features, true);
    (generated, project_features)
}

/// Concatenates units into a single source string.
fn join_units(units: Vec<nexa_codegen::SourceUnit>) -> String {
    let mut source = String::new();
    for unit in units {
        source.push_str(&unit.contents);
    }
    source
}

pub(super) fn render_opt_in_annotation(out: &mut SourceWriter, annotations: &[&str]) {
    if annotations.is_empty() {
        return;
    }

    out.push_str("@OptIn(");
    out.push_str(&annotations.join(", "));
    out.push_str(")\n");
}

/// Whether the app declares a collection state, which is what makes a whole
/// collection replaceable at runtime.
fn has_collection_state(module: &Module) -> bool {
    let is_collection = |ty: &nexa_ir::Type| {
        matches!(
            ty,
            nexa_ir::Type::Array(_) | nexa_ir::Type::Set(_) | nexa_ir::Type::Map(..)
        )
    };
    module
        .states
        .iter()
        .chain(
            module
                .screens
                .iter()
                .flat_map(|screen| screen.states.iter()),
        )
        .chain(
            module
                .components
                .iter()
                .flat_map(|component| component.states.iter()),
        )
        .any(|state| is_collection(&state.ty))
}

/// The overloads that replace a collection state's contents. One per
/// collection kind, so the call site resolves by static type and the generated
/// app carries no dynamic dispatch.
const COLLECTION_REPLACE_HELPERS: &str = r#"/** Replaces a list state with a new value. */
internal fun <T> nexaReplace(target: MutableList<T>, value: List<T>) {
    target.clear()
    target.addAll(value)
}

/** Replaces a set state with a new value. */
internal fun <T> nexaReplace(target: MutableSet<T>, value: Set<T>) {
    target.clear()
    target.addAll(value)
}

/** Replaces a map state with a new value. */
internal fun <K, V> nexaReplace(target: MutableMap<K, V>, value: Map<K, V>) {
    target.clear()
    target.putAll(value)
}

"#;

/// Compose's state collections expose their immutable backing collection in
/// O(1). Snapshot it once before a loop instead of reading snapshot state for
/// every element through the collection iterator. Ordinary Kotlin collections
/// pass through unchanged.
const COLLECTION_ITERATION_HELPERS: &str = r#"private fun <T> nexaSnapshotValues(values: Iterable<T>): Iterable<T> = values
private fun <T> nexaSnapshotValues(values: androidx.compose.runtime.snapshots.SnapshotStateList<T>): List<T> = values.toList()

private fun <K, V> nexaSnapshotEntries(values: Map<K, V>): Map<K, V> = values
private fun <K, V> nexaSnapshotEntries(values: androidx.compose.runtime.snapshots.SnapshotStateMap<K, V>): Map<K, V> = values.toMap()

"#;

/// Native component primitives used by generated applications and the
/// interpreted DevRuntime. Keep rendering defaults here so both render paths
/// use the same Compose components and layout behavior.
const NEXA_SHARED_COMPONENT_PRIMITIVES: &str = concat!(
    r#"// nexa-codegen: NexaAppearancePrimitive
@androidx.compose.runtime.Composable
internal fun NexaAppearancePrimitive(
    mode: String,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val systemDark = androidx.compose.foundation.isSystemInDarkTheme()
    val view = androidx.compose.ui.platform.LocalView.current
    val useLightStatusBarIcons = when (mode) {
        "dark" -> false
        "light" -> true
        else -> !systemDark
    }
    val statusBarWindow = (view.context as? android.app.Activity)?.window
    androidx.compose.runtime.DisposableEffect(statusBarWindow, view) {
        val controller = statusBarWindow?.let { window ->
            androidx.core.view.WindowCompat.getInsetsController(window, view)
        }
        val initialLightStatusBar = controller?.isAppearanceLightStatusBars
        onDispose {
            if (controller != null && initialLightStatusBar != null) {
                controller.isAppearanceLightStatusBars = initialLightStatusBar
            }
        }
    }
    androidx.compose.runtime.SideEffect {
        statusBarWindow?.let { window ->
            androidx.core.view.WindowCompat.getInsetsController(window, view)
                .isAppearanceLightStatusBars = useLightStatusBarIcons
        }
    }
    val scheme = androidx.compose.runtime.remember(mode, systemDark) {
        when (mode) {
            "dark" -> __NEXA_DARK_COLOR_SCHEME__
            "light" -> __NEXA_LIGHT_COLOR_SCHEME__
            else -> if (systemDark) __NEXA_DARK_COLOR_SCHEME__ else __NEXA_LIGHT_COLOR_SCHEME__
        }
    }
    androidx.compose.material3.MaterialTheme(
        colorScheme = scheme,
        typography = androidx.compose.material3.MaterialTheme.typography,
        shapes = androidx.compose.material3.MaterialTheme.shapes,
    ) {
        androidx.compose.runtime.CompositionLocalProvider(
            androidx.compose.material3.LocalContentColor provides scheme.onBackground,
        ) {
            content()
        }
    }
}

// nexa-codegen: NexaFastListDividerPrimitive
@androidx.compose.runtime.Composable
internal fun NexaFastListDividerPrimitive() {
    NexaSystemDividerPrimitive()
}

// nexa-codegen: NexaTextPrimitive
internal val LocalNexaTextAlignment = androidx.compose.runtime.staticCompositionLocalOf<androidx.compose.ui.text.style.TextAlign?> { null }

internal data class NexaGlyphShadow(
    val color: androidx.compose.ui.graphics.Color,
    val radius: androidx.compose.ui.unit.Dp,
    val x: androidx.compose.ui.unit.Dp,
    val y: androidx.compose.ui.unit.Dp,
)

@androidx.compose.runtime.Composable
internal fun NexaTextPrimitive(
    text: String,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    color: androidx.compose.ui.graphics.Color = androidx.compose.ui.graphics.Color.Unspecified,
    fontSize: androidx.compose.ui.unit.TextUnit = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
    fontWeight: androidx.compose.ui.text.font.FontWeight? = androidx.compose.ui.text.font.FontWeight.Normal,
    fontFamily: androidx.compose.ui.text.font.FontFamily? = null,
    textAlign: androidx.compose.ui.text.style.TextAlign? = null,
    maxLines: Int = Int.MAX_VALUE,
    lineHeight: androidx.compose.ui.unit.TextUnit = fontSize * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
    letterSpacing: androidx.compose.ui.unit.TextUnit = 0.sp,
    strikethrough: Boolean = false,
    italic: Boolean = false,
    underline: Boolean = false,
    softWrap: Boolean = true,
    selectable: Boolean = false,
    textShadow: NexaGlyphShadow? = null,
) {
    val resolvedTextShadow = textShadow?.let { shadow ->
        with(androidx.compose.ui.platform.LocalDensity.current) {
            androidx.compose.ui.graphics.Shadow(
                color = shadow.color,
                offset = androidx.compose.ui.geometry.Offset(shadow.x.toPx(), shadow.y.toPx()),
                blurRadius = shadow.radius.toPx(),
            )
        }
    }
    val content: @androidx.compose.runtime.Composable () -> Unit = {
        androidx.compose.material3.Text(
            text = text,
            modifier = modifier,
            color = color,
            fontSize = fontSize,
            fontWeight = fontWeight,
            fontFamily = fontFamily,
            textAlign = textAlign ?: LocalNexaTextAlignment.current,
            maxLines = maxLines,
            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            lineHeight = lineHeight,
            letterSpacing = letterSpacing,
            fontStyle = if (italic) androidx.compose.ui.text.font.FontStyle.Italic else null,
            textDecoration = when {
                underline && strikethrough -> androidx.compose.ui.text.style.TextDecoration.Underline + androidx.compose.ui.text.style.TextDecoration.LineThrough
                underline -> androidx.compose.ui.text.style.TextDecoration.Underline
                strikethrough -> androidx.compose.ui.text.style.TextDecoration.LineThrough
                else -> null
            },
            style = androidx.compose.ui.text.TextStyle(shadow = resolvedTextShadow),
            softWrap = softWrap,
        )
    }
    if (selectable) androidx.compose.foundation.text.selection.SelectionContainer { content() } else content()
}

// nexa-codegen: NexaRefreshControlPrimitive
@androidx.compose.runtime.Composable
internal fun NexaRefreshControlPrimitive(
    isRefreshing: Boolean,
    scrollContent: Boolean,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    onRefresh: () -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.material3.pulltorefresh.PullToRefreshBox(
        isRefreshing = isRefreshing,
        onRefresh = onRefresh,
        modifier = modifier.fillMaxSize(),
    ) {
        if (scrollContent) {
            androidx.compose.foundation.layout.Column(
                androidx.compose.ui.Modifier.fillMaxSize().verticalScroll(
                    androidx.compose.foundation.rememberScrollState(),
                ),
            ) {
                content()
            }
        } else {
            content()
        }
    }
}

// nexa-codegen: NexaButtonPrimitive
@androidx.compose.runtime.Composable
internal fun NexaButtonPrimitive(
    label: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector?,
    style: String?,
    size: String?,
    shape: androidx.compose.ui.graphics.Shape?,
    tint: androidx.compose.ui.graphics.Color?,
    loading: Boolean,
    disabled: Boolean,
    iconOnlyCircle: Boolean,
    fullWidth: Boolean,
    leadingContent: Boolean,
    onClick: () -> Unit,
) {
    // Bottom-sheet toolbar controls stay backgroundless, like native sheet controls.
    val useNativeActionStyle = style == null && (
        LocalNexaBottomSheetToolbarButtons.current || LocalNexaDialogActionButtons.current
    )
    val resolvedStyle = if (useNativeActionStyle) "Plain" else style ?: "BorderedProminent"
    val minHeight = if (size == "Large") androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LARGE_MIN_HEIGHT__.toFloat()) else androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_TAP_TARGET__.toFloat())
    val modifier = when {
        fullWidth -> androidx.compose.ui.Modifier.fillMaxWidth().defaultMinSize(minWidth = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_WIDTH__.toFloat()), minHeight = minHeight)
        iconOnlyCircle -> androidx.compose.ui.Modifier.size(minHeight)
        else -> androidx.compose.ui.Modifier.defaultMinSize(minWidth = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_WIDTH__.toFloat()), minHeight = minHeight)
    }
    val resolvedShape = shape ?: androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)
    val contentPadding = when {
        iconOnlyCircle || resolvedStyle == "Plain" || resolvedStyle == "Borderless" -> androidx.compose.foundation.layout.PaddingValues(androidx.compose.ui.unit.Dp(0f))
        size == "Small" -> androidx.compose.foundation.layout.PaddingValues(horizontal = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__.toFloat()), vertical = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_SMALL_VERTICAL_PADDING__.toFloat()))
        size == "Large" -> androidx.compose.foundation.layout.PaddingValues(horizontal = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__.toFloat()), vertical = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LARGE_VERTICAL_PADDING__.toFloat()))
        else -> androidx.compose.foundation.layout.PaddingValues(horizontal = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__.toFloat()), vertical = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__.toFloat()))
    }
    val enabled = !loading && !disabled
    val plainButtonStyle = resolvedStyle == "Plain" || resolvedStyle == "Borderless"
    val defaultPlainContentColor = if (resolvedStyle == "Borderless") {
        androidx.compose.material3.MaterialTheme.colorScheme.primary
    } else {
        androidx.compose.material3.MaterialTheme.colorScheme.onSurface
    }
    val plainContentColor = when {
        !plainButtonStyle -> androidx.compose.ui.graphics.Color.Unspecified
        enabled -> tint ?: defaultPlainContentColor
        else -> (tint ?: defaultPlainContentColor).copy(alpha = 0.38f)
    }
    val buttonContents: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {
        if (loading) {
            androidx.compose.material3.CircularProgressIndicator(
                modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LOADING_INDICATOR_SIZE__.toFloat())),
                color = if (plainButtonStyle) {
                    (tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary).copy(alpha = 0.38f)
                } else {
                    androidx.compose.material3.MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)
                },
            )
        } else {
            if (icon != null) {
                androidx.compose.material3.Icon(imageVector = icon, contentDescription = null, modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat())))
            }
            if (icon != null && label.isNotEmpty()) {
                androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.width(androidx.compose.ui.unit.Dp(__NEXA_ICON_LABEL_SPACING__.toFloat())))
            }
            NexaTextPrimitive(
                text = label,
                color = plainContentColor,
                fontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
                maxLines = 1,
                lineHeight = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
                letterSpacing = 0.sp,
                softWrap = false,
            )
        }
    }
    val buttonContent: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = if (leadingContent) {
        {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
                contentAlignment = androidx.compose.ui.Alignment.CenterStart,
            ) {
                androidx.compose.foundation.layout.Row(
                    verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                    content = buttonContents,
                )
            }
        }
    } else buttonContents
    when (resolvedStyle) {
        "Plain", "Borderless" -> androidx.compose.material3.TextButton(
            modifier = modifier,
            onClick = onClick,
            enabled = enabled,
            shape = resolvedShape,
            colors = androidx.compose.material3.ButtonDefaults.textButtonColors(
                containerColor = androidx.compose.ui.graphics.Color.Transparent,
                contentColor = tint ?: androidx.compose.ui.graphics.Color.Unspecified,
                disabledContainerColor = androidx.compose.ui.graphics.Color.Transparent,
            ),
            contentPadding = contentPadding,
            content = buttonContent,
        )
        "Bordered" -> {
            val defaultTint = if (androidx.compose.material3.MaterialTheme.colorScheme.background == androidx.compose.ui.graphics.Color.Black) {
                androidx.compose.ui.graphics.Color(__NEXA_DARK_ACCENT_ARGB__)
            } else {
                androidx.compose.ui.graphics.Color(__NEXA_DEFAULT_ACCENT_ARGB__)
            }
            val outlinedTint = tint ?: defaultTint
            androidx.compose.material3.FilledTonalButton(
                modifier = modifier,
                onClick = onClick,
                enabled = enabled,
                shape = resolvedShape,
                colors = androidx.compose.material3.ButtonDefaults.outlinedButtonColors(contentColor = outlinedTint, disabledContentColor = outlinedTint.copy(alpha = 0.38f)),
                border = androidx.compose.foundation.BorderStroke(androidx.compose.ui.unit.Dp(1f), outlinedTint.copy(alpha = if (enabled) 1f else 0.12f)),
                contentPadding = contentPadding,
                content = buttonContent,
            )
        }
        else -> androidx.compose.material3.Button(
            modifier = modifier,
            onClick = onClick,
            enabled = enabled,
            shape = resolvedShape,
            colors = androidx.compose.material3.ButtonDefaults.buttonColors(containerColor = tint ?: androidx.compose.ui.graphics.Color.Unspecified),
            contentPadding = contentPadding,
            content = buttonContent,
        )
    }
}

// nexa-codegen: NexaStatusBarBackgroundPrimitive
@androidx.compose.runtime.Composable
internal fun NexaStatusBarBackgroundPrimitive(
    background: androidx.compose.ui.graphics.Color,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val density = androidx.compose.ui.platform.LocalDensity.current
    val statusBarHeight = androidx.compose.foundation.layout.WindowInsets.statusBars
        .getTop(density)
        .let { with(density) { it.toDp() } }
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier.fillMaxSize(),
    ) {
        content()
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier
                .align(androidx.compose.ui.Alignment.TopCenter)
                .fillMaxWidth()
                .height(statusBarHeight)
                .background(background),
        )
    }
}

// nexa-codegen: NexaBottomSheetPrimitive
@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun NexaBottomSheetPrimitive(
    isPresented: Boolean,
    partial: Boolean,
    largeOnly: Boolean,
    showDragHandle: Boolean,
    title: String?,
    hasNavigationIcon: Boolean,
    hasActions: Boolean,
    onDismissRequest: () -> Unit,
    navigationIcon: @androidx.compose.runtime.Composable () -> Unit,
    actions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    if (!isPresented) return
    if (partial) {
        val density = androidx.compose.ui.platform.LocalDensity.current
        val keyboardInset = androidx.compose.foundation.layout.WindowInsets.ime.getBottom(density)
        val keyboardVisible = keyboardInset > 0
        val statusBarInsetPx = androidx.compose.foundation.layout.WindowInsets.statusBars.getTop(density)
        val statusBarInset = androidx.compose.ui.unit.Dp(statusBarInsetPx / density.density)
        val navigationBarInsetPx = androidx.compose.foundation.layout.WindowInsets.navigationBars.getBottom(density)
        val navigationBarInset = androidx.compose.ui.unit.Dp(navigationBarInsetPx / density.density)
        val screenHeight = androidx.compose.ui.platform.LocalConfiguration.current.screenHeightDp.dp
        val toolbarHeight = if (title == null) androidx.compose.ui.unit.Dp(0f) else androidx.compose.ui.unit.Dp(48f)
        val partialBodyHeight = (
            screenHeight / 2f - androidx.compose.ui.unit.Dp(12f) - toolbarHeight - navigationBarInset
        ).coerceAtLeast(androidx.compose.ui.unit.Dp(0f))
        val expandedBodyHeight = (
            screenHeight - androidx.compose.ui.unit.Dp(12f) - toolbarHeight - navigationBarInset
        ).coerceAtLeast(androidx.compose.ui.unit.Dp(0f))
        val enabledSheetStates = if (largeOnly) {
            setOf(
                androidx.compose.material3.SheetValue.Hidden,
                androidx.compose.material3.SheetValue.Expanded,
            )
        } else {
            setOf(
                androidx.compose.material3.SheetValue.Hidden,
                androidx.compose.material3.SheetValue.PartiallyExpanded,
                androidx.compose.material3.SheetValue.Expanded,
            )
        }
        val sheetState = androidx.compose.material3.rememberBottomSheetState(
            initialValue = androidx.compose.material3.SheetValue.Hidden,
            enabledValues = enabledSheetStates,
        )
        androidx.compose.material3.ModalBottomSheet(
            onDismissRequest = onDismissRequest,
            modifier = androidx.compose.ui.Modifier.padding(
                top = if (
                    sheetState.currentValue == androidx.compose.material3.SheetValue.Expanded
                ) statusBarInset else androidx.compose.ui.unit.Dp(0f),
            ),
            sheetState = sheetState,
            containerColor = androidx.compose.material3.MaterialTheme.colorScheme.background,
            shape = androidx.compose.foundation.shape.RoundedCornerShape(
                topStart = androidx.compose.ui.unit.Dp(32f),
                topEnd = androidx.compose.ui.unit.Dp(32f),
            ),
            dragHandle = if (showDragHandle) {
                {
                    androidx.compose.foundation.layout.Box(
                        modifier = androidx.compose.ui.Modifier
                            .fillMaxWidth()
                            .height(androidx.compose.ui.unit.Dp(12f)),
                        contentAlignment = androidx.compose.ui.Alignment.Center,
                    ) {
                        androidx.compose.foundation.layout.Box(
                            modifier = androidx.compose.ui.Modifier
                                .size(
                                    width = androidx.compose.ui.unit.Dp(36f),
                                    height = androidx.compose.ui.unit.Dp(4f),
                                )
                                .clip(androidx.compose.foundation.shape.CircleShape)
                                .background(
                                    androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.45f),
                                ),
                        )
                    }
                }
            } else {
                null
            },
        ) {
            val restoreExpandedAfterKeyboard = androidx.compose.runtime.remember {
                androidx.compose.runtime.mutableStateOf(false)
            }
            androidx.compose.runtime.LaunchedEffect(keyboardVisible) {
                if (!largeOnly && keyboardVisible) {
                    restoreExpandedAfterKeyboard.value =
                        sheetState.currentValue == androidx.compose.material3.SheetValue.Expanded ||
                            sheetState.targetValue == androidx.compose.material3.SheetValue.Expanded
                    if (restoreExpandedAfterKeyboard.value) {
                        sheetState.partialExpand()
                    }
                } else if (!largeOnly && !keyboardVisible) {
                    if (restoreExpandedAfterKeyboard.value) {
                        restoreExpandedAfterKeyboard.value = false
                        sheetState.expand()
                    }
                }
            }
            androidx.compose.foundation.layout.Column(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxWidth()
                    // Keep the sheet measured at the available height so Material3 can create
                    // both the partially-expanded and expanded anchors, even when its content is
                    // short. A state-dependent height removes the partial anchor on short content.
                    .fillMaxHeight(),
            ) {
                if (title != null) {
                    androidx.compose.foundation.layout.Row(
                        modifier = androidx.compose.ui.Modifier
                            .fillMaxWidth()
                            .height(androidx.compose.ui.unit.Dp(48f))
                            .padding(horizontal = androidx.compose.ui.unit.Dp(16f)),
                        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                    ) {
                        androidx.compose.foundation.layout.Row(
                            modifier = androidx.compose.ui.Modifier.weight(1f),
                            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                        ) {
                            if (hasNavigationIcon) {
                                androidx.compose.foundation.layout.Box(
                                    modifier = androidx.compose.ui.Modifier
                                        .height(androidx.compose.ui.unit.Dp(48f))
                                        .widthIn(min = androidx.compose.ui.unit.Dp(48f)),
                                    contentAlignment = androidx.compose.ui.Alignment.Center,
                                ) {
                                    androidx.compose.runtime.CompositionLocalProvider(
                                        LocalNexaBottomSheetToolbarButtons provides true,
                                        LocalNexaPickerPressFeedback provides false,
                                    ) {
                                        navigationIcon()
                                    }
                                }
                            }
                        }
                        androidx.compose.foundation.layout.Box(
                            modifier = androidx.compose.ui.Modifier
                                .weight(1f)
                                .fillMaxHeight(),
                            contentAlignment = androidx.compose.ui.Alignment.Center,
                        ) {
                            androidx.compose.material3.Text(
                                title,
                                style = NexaNavigationChromeTextStyle(),
                                maxLines = 1,
                            )
                        }
                        androidx.compose.foundation.layout.Row(
                            modifier = androidx.compose.ui.Modifier.weight(1f),
                            horizontalArrangement = androidx.compose.foundation.layout.Arrangement.End,
                            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                        ) {
                            if (hasActions) {
                                androidx.compose.foundation.layout.Box(
                                    modifier = androidx.compose.ui.Modifier
                                        .height(androidx.compose.ui.unit.Dp(48f))
                                        .widthIn(min = androidx.compose.ui.unit.Dp(48f)),
                                    contentAlignment = androidx.compose.ui.Alignment.Center,
                                ) {
                                    androidx.compose.runtime.CompositionLocalProvider(
                                        LocalNexaBottomSheetToolbarButtons provides true,
                                        LocalNexaPickerPressFeedback provides false,
                                    ) {
                                        androidx.compose.foundation.layout.Row(
                                            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                                            content = actions,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
                if (!keyboardVisible) {
                    val isExpanding =
                        sheetState.targetValue == androidx.compose.material3.SheetValue.Expanded ||
                            sheetState.currentValue == androidx.compose.material3.SheetValue.Expanded
                    val bodyHeight = androidx.compose.animation.core.animateDpAsState(
                        targetValue = if (isExpanding) expandedBodyHeight else partialBodyHeight,
                        label = "nexaBottomSheetBodyHeight",
                    ).value
                    androidx.compose.foundation.layout.Box(
                        modifier = androidx.compose.ui.Modifier
                            .fillMaxWidth()
                            .height(bodyHeight),
                        contentAlignment = androidx.compose.ui.Alignment.Center,
                    ) {
                        content()
                    }
                } else {
                    content()
                }
            }
        }
    } else {
        androidx.compose.ui.window.Dialog(
            onDismissRequest = onDismissRequest,
            properties = androidx.compose.ui.window.DialogProperties(usePlatformDefaultWidth = false),
        ) {
            androidx.compose.material3.Surface(
                modifier = androidx.compose.ui.Modifier.fillMaxSize(),
                shape = androidx.compose.ui.graphics.RectangleShape,
            ) {
                androidx.compose.foundation.layout.Box(
                    modifier = androidx.compose.ui.Modifier.fillMaxSize(),
                    contentAlignment = androidx.compose.ui.Alignment.Center,
                ) {
                    content()
                }
            }
        }
    }
}

// nexa-codegen: NexaAlertDialogPrimitive
@androidx.compose.runtime.Composable
internal fun NexaAlertDialogPrimitive(
    onDismissRequest: () -> Unit,
    title: @androidx.compose.runtime.Composable () -> Unit,
    text: @androidx.compose.runtime.Composable () -> Unit,
    dismissButton: @androidx.compose.runtime.Composable () -> Unit = {},
    confirmButton: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismissRequest,
        title = title,
        text = text,
        dismissButton = {
            androidx.compose.runtime.CompositionLocalProvider(
                LocalNexaDialogActionButtons provides true,
                content = dismissButton,
            )
        },
        confirmButton = {
            androidx.compose.runtime.CompositionLocalProvider(
                LocalNexaDialogActionButtons provides true,
                content = confirmButton,
            )
        },
    )
}

// nexa-codegen: NexaConfirmationDialogPrimitive
@androidx.compose.runtime.Composable
internal fun NexaConfirmationDialogPrimitive(
    title: String,
    onDismissRequest: () -> Unit,
    actions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismissRequest,
        title = { androidx.compose.material3.Text(title) },
        text = {
            androidx.compose.foundation.layout.Column(
                modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
                horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally,
            ) {
                actions()
            }
        },
        confirmButton = {},
    )
}

// nexa-codegen: NexaNavigationSplitViewPrimitive
@androidx.compose.runtime.Composable
internal fun NexaNavigationSplitViewPrimitive(
    detailVisible: Boolean,
    onDetailVisibleChange: (Boolean) -> Unit,
    sidebar: @androidx.compose.runtime.Composable () -> Unit,
    detail: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.foundation.layout.BoxWithConstraints(
        androidx.compose.ui.Modifier.fillMaxSize(),
    ) {
        if (maxWidth >= androidx.compose.ui.unit.Dp(840.toFloat())) {
            androidx.compose.foundation.layout.Row(
                androidx.compose.ui.Modifier.fillMaxSize(),
            ) {
                androidx.compose.foundation.layout.Box(
                    androidx.compose.ui.Modifier
                        .widthIn(
                            min = androidx.compose.ui.unit.Dp(260.toFloat()),
                            max = androidx.compose.ui.unit.Dp(360.toFloat()),
                        )
                        .fillMaxHeight(),
                    contentAlignment = androidx.compose.ui.Alignment.TopCenter,
                ) {
                    sidebar()
                }
                androidx.compose.foundation.layout.Box(
                    androidx.compose.ui.Modifier
                        .weight(1f)
                        .fillMaxHeight(),
                    contentAlignment = androidx.compose.ui.Alignment.TopCenter,
                ) {
                    detail()
                }
            }
        } else {
            androidx.activity.compose.BackHandler(enabled = detailVisible) {
                onDetailVisibleChange(false)
            }
            androidx.compose.foundation.layout.Box(
                androidx.compose.ui.Modifier.fillMaxSize(),
                contentAlignment = androidx.compose.ui.Alignment.TopCenter,
            ) {
                if (detailVisible) detail() else sidebar()
            }
        }
    }
}

// nexa-codegen: NexaNavigationScreenPrimitive
@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun NexaNavigationScreenPrimitive(
    title: String,
    inline: Boolean,
    leadingTitle: Boolean,
    hasTrailingActions: Boolean,
    onBack: () -> Unit,
    leadingActions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {},
    actions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {},
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val colorScheme = androidx.compose.material3.MaterialTheme.colorScheme
    val navigationTitleSlot = androidx.compose.runtime.remember {
        NexaNavigationTitleSlot()
    }
    val hasTitleOverride = navigationTitleSlot.identity.value != null
    val displayTitle = if (hasTitleOverride) navigationTitleSlot.title.value else title
    val displayLargeTitle = if (hasTitleOverride) navigationTitleSlot.largeTitle.value else !inline
    val displayCenterTitle = if (hasTitleOverride) navigationTitleSlot.centerTitle.value else !leadingTitle
    val displayLeadingActions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {
        if (hasTitleOverride) {
            navigationTitleSlot.leadingContent()
        } else {
            leadingActions()
        }
    }
    val displayActions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {
        if (hasTitleOverride) {
            navigationTitleSlot.trailingContent(this)
        } else {
            actions()
        }
    }
    val scrollBehavior = if (displayLargeTitle) {
        androidx.compose.material3.TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
    } else {
        androidx.compose.material3.TopAppBarDefaults.pinnedScrollBehavior()
    }
    androidx.compose.material3.Scaffold(
        modifier = androidx.compose.ui.Modifier
            .fillMaxSize()
            .nestedScroll(scrollBehavior.nestedScrollConnection),
        // Form and native list primitives own their grouped surface backgrounds. Keeping the
        // navigation scaffold on the app background prevents it tinting the title bar and the
        // content above an inset Form.
        containerColor = colorScheme.background,
        contentColor = colorScheme.onSurface,
        topBar = {
            val navigationIcon: @androidx.compose.runtime.Composable () -> Unit = {
                androidx.compose.foundation.layout.Row(
                    verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                ) {
                    androidx.compose.material3.IconButton(onClick = onBack) {
                        androidx.compose.material3.Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                        )
                    }
                    if (!inline || !leadingTitle || hasTitleOverride) {
                        displayLeadingActions()
                    }
                }
            }
            val titleContent: @androidx.compose.runtime.Composable () -> Unit = {
                if (!displayLargeTitle) {
                    androidx.compose.material3.Text(
                        displayTitle,
                        style = NexaNavigationChromeTextStyle(),
                        maxLines = 1,
                        overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                    )
                } else {
                    androidx.compose.material3.Text(
                        displayTitle,
                        maxLines = 1,
                        overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                    )
                }
            }
            val colors = androidx.compose.material3.TopAppBarDefaults.topAppBarColors(
                containerColor = colorScheme.background,
                scrolledContainerColor = colorScheme.background,
            )
            val trailingActions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {
                displayActions()
                if (hasTrailingActions) {
                    androidx.compose.foundation.layout.Spacer(
                        modifier = androidx.compose.ui.Modifier.width(
                            androidx.compose.ui.unit.Dp(__NEXA_NAVIGATION_TRAILING_ACTION_INSET__.toFloat()),
                        ),
                    )
                }
            }
            if (!hasTitleOverride && inline && leadingTitle) {
                androidx.compose.material3.TopAppBar(
                    title = {
                        androidx.compose.foundation.layout.Row(
                            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                        ) {
                            androidx.compose.material3.Text(
                                title,
                                style = NexaNavigationChromeTextStyle(),
                                maxLines = 1,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                            )
                            androidx.compose.foundation.layout.Row(
                                modifier = androidx.compose.ui.Modifier.padding(
                                    start = androidx.compose.ui.unit.Dp(24f),
                                ),
                                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                            ) {
                                leadingActions()
                            }
                        }
                    },
                    navigationIcon = navigationIcon,
                    actions = trailingActions,
                    scrollBehavior = scrollBehavior,
                    colors = colors,
                )
            } else if (!displayLargeTitle && displayCenterTitle) {
                androidx.compose.material3.CenterAlignedTopAppBar(
                    title = titleContent,
                    navigationIcon = navigationIcon,
                    actions = trailingActions,
                    scrollBehavior = scrollBehavior,
                    colors = colors,
                )
            } else if (!displayLargeTitle) {
                androidx.compose.material3.TopAppBar(
                    title = titleContent,
                    navigationIcon = navigationIcon,
                    actions = trailingActions,
                    scrollBehavior = scrollBehavior,
                    colors = colors,
                )
            } else {
                androidx.compose.material3.LargeFlexibleTopAppBar(
                    title = titleContent,
                    navigationIcon = navigationIcon,
                    actions = trailingActions,
                    scrollBehavior = scrollBehavior,
                    colors = colors,
                )
            }
        },
    ) {
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier
                .fillMaxSize()
                .padding(it)
                .background(colorScheme.background),
        ) {
            androidx.compose.runtime.CompositionLocalProvider(
                LocalNexaFormContentTopInset provides androidx.compose.ui.unit.Dp(0f),
                LocalNexaNavigationContent provides true,
            ) {
                androidx.compose.runtime.CompositionLocalProvider(
                    LocalNexaNavigationTitleSlot provides navigationTitleSlot,
                ) {
                    content()
                }
            }
        }
    }
}

// nexa-codegen: NexaNavigationChromeTextStyle
@androidx.compose.runtime.Composable
internal fun NexaNavigationChromeTextStyle(): androidx.compose.ui.text.TextStyle =
    androidx.compose.material3.MaterialTheme.typography.titleMedium.copy(
        fontSize = __NEXA_TITLE_BAR_COLLAPSED_FONT_SIZE__.sp,
        fontWeight = androidx.compose.ui.text.font.FontWeight.SemiBold,
    )

// A tab title nested in a navigation destination replaces that destination's app-bar title.
// The destination still owns the native back button and app-bar layout.
internal class NexaNavigationTitleSlot {
    val identity = androidx.compose.runtime.mutableStateOf<String?>(null)
    val title = androidx.compose.runtime.mutableStateOf("")
    val largeTitle = androidx.compose.runtime.mutableStateOf(false)
    val centerTitle = androidx.compose.runtime.mutableStateOf(false)
    var leadingContent: @androidx.compose.runtime.Composable () -> Unit = {}
    var trailingContent: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {}

    fun update(
        identity: String,
        title: String,
        largeTitle: Boolean,
        centerTitle: Boolean,
        leadingContent: @androidx.compose.runtime.Composable () -> Unit,
        trailingContent: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit,
    ) {
        this.leadingContent = leadingContent
        this.trailingContent = trailingContent
        if (this.identity.value != identity) this.identity.value = identity
        if (this.title.value != title) this.title.value = title
        if (this.largeTitle.value != largeTitle) this.largeTitle.value = largeTitle
        if (this.centerTitle.value != centerTitle) this.centerTitle.value = centerTitle
    }

    fun clear(identity: String) {
        if (this.identity.value == identity) this.identity.value = null
    }
}

internal val LocalNexaNavigationTitleSlot =
    androidx.compose.runtime.staticCompositionLocalOf<NexaNavigationTitleSlot?> { null }

// nexa-codegen: NexaNavigationLinkPrimitive
@androidx.compose.runtime.Composable
internal fun NexaNavigationLinkPrimitive(
    enabled: Boolean,
    onClick: () -> Unit,
    trailing: @androidx.compose.runtime.Composable () -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.foundation.layout.Row(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .clickable(enabled = enabled, onClick = onClick),
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    ) {
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier.weight(1f),
        ) {
            content()
        }
        trailing()
    }
}

// nexa-codegen: NexaDialogActionDividerPrimitive
@androidx.compose.runtime.Composable
internal fun NexaDialogActionDividerPrimitive() {
    NexaSystemDividerPrimitive()
}

// nexa-codegen: NexaKeyboardAwareColumnPrimitive
@androidx.compose.runtime.Composable
@kotlin.OptIn(
    androidx.compose.foundation.layout.ExperimentalLayoutApi::class,
    androidx.compose.foundation.ExperimentalFoundationApi::class,
)
internal fun NexaKeyboardAwareColumnPrimitive(
    interactiveDismiss: Boolean,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val imeVisible = androidx.compose.runtime.rememberUpdatedState(
        androidx.compose.foundation.layout.WindowInsets.isImeVisible,
    )
    val focusManager = androidx.compose.ui.platform.LocalFocusManager.current
    val touchSlop = androidx.compose.ui.platform.LocalViewConfiguration.current.touchSlop
    var modifier = androidx.compose.ui.Modifier.pointerInput(focusManager, touchSlop) {
        awaitEachGesture {
            val keyboardWasVisible = imeVisible.value
            val down = awaitFirstDown(
                requireUnconsumed = false,
                pass = androidx.compose.ui.input.pointer.PointerEventPass.Initial,
            )
            var dragStarted = false
            while (true) {
                val event = awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial)
                val change = event.changes.firstOrNull { it.id == down.id } ?: break
                if (!change.pressed) break
                val movedX = kotlin.math.abs(change.position.x - down.position.x)
                val movedY = kotlin.math.abs(change.position.y - down.position.y)
                if (!keyboardWasVisible && (movedX > touchSlop || movedY > touchSlop)) {
                    dragStarted = true
                    focusManager.clearFocus()
                    break
                }
            }
            if (dragStarted) {
                do {
                    val event = awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Final)
                } while (event.changes.any { it.pressed })
                focusManager.clearFocus()
            }
        }
    }.imePadding()
    // imeNestedScroll is only meaningful while the IME is already visible.
    // Keeping it attached to every hidden-IME scroll can issue a show request
    // on Android when the scroll reaches its edge, even if no field is focused.
    if (interactiveDismiss && imeVisible.value) modifier = modifier.imeNestedScroll()
    androidx.compose.foundation.layout.Column(
        modifier = modifier.verticalScroll(androidx.compose.foundation.rememberScrollState()),
        content = content,
    )
}

// nexa-codegen: NexaTextInputPrimitive
@androidx.compose.runtime.Composable
internal fun NexaTextInputPrimitive(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    focusModifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    placeholder: String = "",
    searchable: Boolean = false,
    searchIcon: androidx.compose.ui.graphics.vector.ImageVector? = null,
    textStyle: androidx.compose.ui.text.TextStyle = androidx.compose.material3.LocalTextStyle.current,
    minLines: Int = 1,
    maxLines: Int = Int.MAX_VALUE,
    singleLine: Boolean = true,
    secure: Boolean = false,
    keyboardOptions: androidx.compose.foundation.text.KeyboardOptions = androidx.compose.foundation.text.KeyboardOptions(),
    keyboardActions: androidx.compose.foundation.text.KeyboardActions = androidx.compose.foundation.text.KeyboardActions(),
) {
    val bringIntoViewRequester = androidx.compose.runtime.remember {
        androidx.compose.foundation.relocation.BringIntoViewRequester()
    }
    val inputHasFocus = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }
    val keyboardInset = androidx.compose.foundation.layout.WindowInsets.ime
        .getBottom(androidx.compose.ui.platform.LocalDensity.current)
    val inputFocusModifier = focusModifier
        .bringIntoViewRequester(bringIntoViewRequester)
        .onFocusChanged { inputHasFocus.value = it.isFocused }
    androidx.compose.runtime.LaunchedEffect(inputHasFocus.value, keyboardInset) {
        if (inputHasFocus.value && keyboardInset > 0) {
            bringIntoViewRequester.bringIntoView()
        }
    }
    if (searchable) {
        androidx.compose.foundation.layout.Row(
            modifier = modifier,
            horizontalArrangement = if (searchIcon != null) {
                androidx.compose.foundation.layout.Arrangement.spacedBy(androidx.compose.ui.unit.Dp(8f))
            } else {
                androidx.compose.foundation.layout.Arrangement.Start
            },
            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
        ) {
            if (searchIcon != null) {
                androidx.compose.material3.Icon(
                    imageVector = searchIcon,
                    contentDescription = null,
                    modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(20f)),
                    tint = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.weight(1f),
                contentAlignment = androidx.compose.ui.Alignment.CenterStart,
            ) {
                androidx.compose.foundation.text.BasicTextField(
                    value = value,
                    onValueChange = onValueChange,
                    modifier = inputFocusModifier.fillMaxWidth(),
                    textStyle = textStyle.copy(
                        color = androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
                    ),
                    minLines = minLines,
                    maxLines = maxLines,
                    singleLine = singleLine,
                    visualTransformation = if (secure) {
                        androidx.compose.ui.text.input.PasswordVisualTransformation()
                    } else {
                        androidx.compose.ui.text.input.VisualTransformation.None
                    },
                    keyboardOptions = keyboardOptions,
                    keyboardActions = keyboardActions,
                    cursorBrush = androidx.compose.ui.graphics.SolidColor(
                        androidx.compose.material3.MaterialTheme.colorScheme.primary,
                    ),
                    decorationBox = { innerTextField ->
                        androidx.compose.foundation.layout.Box(
                            contentAlignment = androidx.compose.ui.Alignment.CenterStart,
                        ) {
                            if (value.isEmpty() && placeholder.isNotEmpty()) {
                                androidx.compose.material3.Text(
                                    placeholder,
                                    style = textStyle.copy(
                                        color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                                    ),
                                    maxLines = 1,
                                )
                            }
                            innerTextField()
                        }
                    },
                )
            }
        }
    } else {
        androidx.compose.foundation.text.BasicTextField(
            value = value,
            onValueChange = onValueChange,
            modifier = modifier.then(inputFocusModifier),
            textStyle = textStyle.copy(
                color = androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
            ),
            minLines = minLines,
            maxLines = maxLines,
            singleLine = singleLine,
            visualTransformation = if (secure) {
                androidx.compose.ui.text.input.PasswordVisualTransformation()
            } else {
                androidx.compose.ui.text.input.VisualTransformation.None
            },
            keyboardOptions = keyboardOptions,
            keyboardActions = keyboardActions,
            cursorBrush = androidx.compose.ui.graphics.SolidColor(
                androidx.compose.material3.MaterialTheme.colorScheme.primary,
            ),
            decorationBox = { innerTextField ->
                androidx.compose.foundation.layout.Box(
                    contentAlignment = androidx.compose.ui.Alignment.CenterStart,
                ) {
                    if (value.isEmpty() && placeholder.isNotEmpty()) {
                        androidx.compose.material3.Text(
                            placeholder,
                            style = textStyle.copy(
                                color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                            ),
                            maxLines = 1,
                        )
                    }
                    innerTextField()
                }
            },
        )
    }
}

// nexa-codegen: NexaSwitchPrimitive
@androidx.compose.runtime.Composable
internal fun NexaSwitchPrimitive(
    label: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    fontSize: androidx.compose.ui.unit.TextUnit,
    lineHeight: androidx.compose.ui.unit.TextUnit,
    offTrackColor: androidx.compose.ui.graphics.Color,
) {
    androidx.compose.foundation.layout.Row(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .toggleable(
                value = checked,
                role = androidx.compose.ui.semantics.Role.Switch,
                onValueChange = onCheckedChange,
            ),
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    ) {
        NexaTextPrimitive(
            text = label,
            modifier = androidx.compose.ui.Modifier.weight(1f),
            fontSize = fontSize,
            fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
            lineHeight = lineHeight,
            textAlign = androidx.compose.ui.text.style.TextAlign.Start,
        )
        androidx.compose.material3.Switch(
            checked = checked,
            onCheckedChange = null,
            modifier = androidx.compose.ui.Modifier.semantics { contentDescription = label },
            colors = androidx.compose.material3.SwitchDefaults.colors(
                checkedThumbColor = androidx.compose.ui.graphics.Color.White,
                checkedTrackColor = androidx.compose.material3.MaterialTheme.colorScheme.primary,
                checkedBorderColor = androidx.compose.material3.MaterialTheme.colorScheme.primary,
                uncheckedThumbColor = androidx.compose.ui.graphics.Color.White,
                uncheckedTrackColor = offTrackColor,
                uncheckedBorderColor = androidx.compose.ui.graphics.Color.Transparent,
            ),
        )
    }
}

// nexa-codegen: NexaSliderPrimitive
@androidx.compose.runtime.Composable
internal fun NexaSliderPrimitive(
    value: Float,
    onValueChange: (Float) -> Unit,
    valueRange: ClosedFloatingPointRange<Float>,
    steps: Int,
) {
    androidx.compose.material3.Slider(
        value = value,
        onValueChange = onValueChange,
        valueRange = valueRange,
        steps = steps,
    )
}

// nexa-codegen: NexaProgressBarPrimitive
@androidx.compose.runtime.Composable
internal fun NexaProgressBarPrimitive(progress: Float) {
    androidx.compose.material3.LinearProgressIndicator(
        progress = { progress.coerceIn(0f, 1f) },
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .progressSemantics(progress.coerceIn(0f, 1f)),
        trackColor = androidx.compose.material3.MaterialTheme.colorScheme.onSurface.copy(alpha = 0.1f),
        drawStopIndicator = {},
    )
}

// nexa-codegen: NexaProgressRingPrimitive
@androidx.compose.runtime.Composable
internal fun NexaProgressRingPrimitive(progress: Float) {
    androidx.compose.material3.CircularProgressIndicator(
        progress = { progress.coerceIn(0f, 1f) },
        modifier = androidx.compose.ui.Modifier.progressSemantics(progress.coerceIn(0f, 1f)),
        color = androidx.compose.material3.MaterialTheme.colorScheme.primary,
        trackColor = androidx.compose.material3.MaterialTheme.colorScheme.primary.copy(alpha = 0.16f),
    )
}

// nexa-codegen: NexaDividerPrimitive
@androidx.compose.runtime.Composable
internal fun NexaDividerPrimitive(color: androidx.compose.ui.graphics.Color, thickness: androidx.compose.ui.unit.Dp) {
    androidx.compose.material3.HorizontalDivider(color = color, thickness = thickness)
}

// nexa-codegen: NexaSystemDividerPrimitive
@androidx.compose.runtime.Composable
internal fun NexaSystemDividerPrimitive(
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    startInset: androidx.compose.ui.unit.Dp = androidx.compose.ui.unit.Dp(0f),
    endInset: androidx.compose.ui.unit.Dp = androidx.compose.ui.unit.Dp(0f),
) {
    val isDark = androidx.compose.material3.MaterialTheme.colorScheme.background == androidx.compose.ui.graphics.Color.Black
    androidx.compose.material3.HorizontalDivider(
        modifier = modifier.fillMaxWidth().padding(start = startInset, end = endInset),
        thickness = androidx.compose.ui.unit.Dp.Hairline,
        color = if (isDark) androidx.compose.ui.graphics.Color(__NEXA_SYSTEM_SEPARATOR_DARK_ARGB__)
        else androidx.compose.ui.graphics.Color(__NEXA_SYSTEM_SEPARATOR_LIGHT_ARGB__),
    )
}

// nexa-codegen: NexaSegmentedControlPrimitive
@androidx.compose.runtime.Composable
internal fun NexaSegmentedControlPrimitive(
    items: List<String>,
    selected: String,
    animation: androidx.compose.animation.core.FiniteAnimationSpec<androidx.compose.ui.unit.Dp>? = null,
    tint: androidx.compose.ui.graphics.Color? = null,
    background: androidx.compose.ui.graphics.Color? = null,
    textColor: androidx.compose.ui.graphics.Color? = null,
    selectedTextColor: androidx.compose.ui.graphics.Color? = null,
    fontSize: androidx.compose.ui.unit.TextUnit? = null,
    fontWeight: androidx.compose.ui.text.font.FontWeight? = null,
    height: androidx.compose.ui.unit.Dp? = null,
    cornerRadius: androidx.compose.ui.unit.Dp? = null,
    borderColor: androidx.compose.ui.graphics.Color? = null,
    borderWidth: androidx.compose.ui.unit.Dp? = null,
    onSelected: (String) -> Unit,
) {
    if (items.isEmpty()) return

    val segmentTrackHeight = height ?: androidx.compose.ui.unit.Dp(32f)
    val trackBorderWidth = (borderWidth ?: if (borderColor != null) androidx.compose.ui.unit.Dp(1f) else androidx.compose.ui.unit.Dp(0f))
        .coerceAtLeast(androidx.compose.ui.unit.Dp(0f))
    val selectionIndicatorInset = androidx.compose.ui.unit.Dp(3f)
    val selectionIndicatorHeight = (segmentTrackHeight - androidx.compose.ui.unit.Dp(6f))
        .coerceAtLeast(androidx.compose.ui.unit.Dp(0f))
    val trackCornerRadius = cornerRadius ?: segmentTrackHeight / 2f
    val segmentInteractionShape = cornerRadius?.let {
        androidx.compose.foundation.shape.RoundedCornerShape(it)
    } ?: androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)
    val segmentTrackShape = androidx.compose.foundation.shape.RoundedCornerShape(trackCornerRadius)
    val selectionIndicatorShape = androidx.compose.foundation.shape.RoundedCornerShape(
        minOf(
            trackCornerRadius,
            selectionIndicatorHeight / 2f,
        ),
    )
    val outerHeight = maxOf(androidx.compose.ui.unit.Dp(48f), segmentTrackHeight)
    androidx.compose.foundation.layout.BoxWithConstraints(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .height(outerHeight),
        contentAlignment = androidx.compose.ui.Alignment.Center,
    ) {
        val segmentWidth = maxWidth / items.size
        val selectedIndex = items.indexOf(selected).coerceAtLeast(0)
        val expressiveSpatialMotion = androidx.compose.runtime.remember {
            androidx.compose.material3.MotionScheme.expressive()
                .defaultSpatialSpec<androidx.compose.ui.unit.Dp>()
        }
        val indicatorOffset = androidx.compose.animation.core.animateDpAsState(
            targetValue = segmentWidth * selectedIndex,
            animationSpec = animation
                ?: expressiveSpatialMotion,
            label = "segmentedControlSelection",
        )
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier
                .fillMaxWidth()
                .height(segmentTrackHeight)
                .align(androidx.compose.ui.Alignment.Center)
                .clip(segmentTrackShape)
                .background(background ?: androidx.compose.material3.MaterialTheme.colorScheme.surfaceVariant)
                .border(
                    width = trackBorderWidth,
                    color = borderColor ?: androidx.compose.material3.MaterialTheme.colorScheme.outlineVariant,
                    shape = segmentTrackShape,
                ),
        ) {}
        if (items.isNotEmpty() && selected in items) {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .align(androidx.compose.ui.Alignment.CenterStart)
                    .offset {
                        androidx.compose.ui.unit.IntOffset(
                            x = indicatorOffset.value.roundToPx(),
                            y = 0,
                        )
                    }
                    .width(segmentWidth)
                    .height(selectionIndicatorHeight)
                    .padding(horizontal = selectionIndicatorInset)
                    .clip(selectionIndicatorShape)
                    .background(tint ?: androidx.compose.material3.MaterialTheme.colorScheme.surface),
            ) {}
        }
        androidx.compose.foundation.layout.Row(
            modifier = androidx.compose.ui.Modifier
                .fillMaxSize()
                .selectableGroup(),
        ) {
            items.forEach { item ->
                val interactionSource = androidx.compose.runtime.remember(item) {
                    androidx.compose.foundation.interaction.MutableInteractionSource()
                }
                androidx.compose.foundation.layout.Box(
                    modifier = androidx.compose.ui.Modifier
                        .weight(1f)
                        .fillMaxHeight()
                        .clip(segmentInteractionShape)
                        .selectable(
                            selected = selected == item,
                            role = androidx.compose.ui.semantics.Role.RadioButton,
                            interactionSource = interactionSource,
                            indication = null,
                            onClick = { onSelected(item) },
                        ),
                    contentAlignment = androidx.compose.ui.Alignment.Center,
                ) {
                    androidx.compose.foundation.layout.Box(
                        modifier = androidx.compose.ui.Modifier
                            .fillMaxWidth()
                            .padding(horizontal = selectionIndicatorInset)
                            .height(selectionIndicatorHeight)
                            .align(androidx.compose.ui.Alignment.Center)
                            .clip(selectionIndicatorShape)
                            .indication(interactionSource, androidx.compose.material3.ripple()),
                    ) {}
                    androidx.compose.material3.Text(
                        text = item,
                        style = androidx.compose.material3.MaterialTheme.typography.labelLarge.copy(
                            fontSize = fontSize ?: androidx.compose.material3.MaterialTheme.typography.labelLarge.fontSize,
                            fontWeight = fontWeight ?: androidx.compose.material3.MaterialTheme.typography.labelLarge.fontWeight,
                        ),
                        color = if (selected == item) {
                            selectedTextColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface
                        } else {
                            textColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface
                        },
                    )
                }
            }
        }
    }
}

// nexa-codegen: NexaSystemIconPrimitive
@androidx.compose.runtime.Composable
internal fun NexaSystemIconPrimitive(
    image: androidx.compose.ui.graphics.vector.ImageVector,
    description: String,
    size: androidx.compose.ui.unit.Dp,
    tint: androidx.compose.ui.graphics.Color,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
) {
    androidx.compose.material3.Icon(
        imageVector = image,
        contentDescription = description.takeIf(String::isNotEmpty),
        modifier = modifier.size(size),
        tint = tint,
    )
}

// nexa-codegen: NexaLinearGradientPrimitive
@androidx.compose.runtime.Composable
internal fun NexaLinearGradientPrimitive(
    startColor: androidx.compose.ui.graphics.Color,
    endColor: androidx.compose.ui.graphics.Color,
    horizontal: Boolean,
    reversed: Boolean,
    height: androidx.compose.ui.unit.Dp,
) {
    val colors = if (reversed) listOf(endColor, startColor) else listOf(startColor, endColor)
    val brush = if (horizontal) {
        androidx.compose.ui.graphics.Brush.horizontalGradient(colors)
    } else {
        androidx.compose.ui.graphics.Brush.verticalGradient(colors)
    }
    androidx.compose.foundation.layout.Spacer(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .height(height)
            .background(brush),
    )
}

// nexa-codegen: NexaContentUnavailablePrimitive
@androidx.compose.runtime.Composable
internal fun NexaContentUnavailablePrimitive(
    title: String,
    description: String?,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    iconDescription: String,
    titleColor: androidx.compose.ui.graphics.Color? = null,
    descriptionColor: androidx.compose.ui.graphics.Color? = null,
    iconTint: androidx.compose.ui.graphics.Color? = null,
    background: androidx.compose.ui.graphics.Color? = null,
) {
    androidx.compose.foundation.layout.Column(
        modifier = androidx.compose.ui.Modifier
            .fillMaxSize()
            .then(if (background != null) androidx.compose.ui.Modifier.background(background) else androidx.compose.ui.Modifier)
            .padding(horizontal = androidx.compose.ui.unit.Dp(16f), vertical = androidx.compose.ui.unit.Dp(32f)),
        verticalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
        horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally,
    ) {
        androidx.compose.material3.Icon(
            imageVector = icon,
            contentDescription = iconDescription,
            modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(58f)),
            tint = iconTint ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
        )
        androidx.compose.foundation.layout.Spacer(modifier = androidx.compose.ui.Modifier.height(androidx.compose.ui.unit.Dp(16f)))
        androidx.compose.material3.Text(
            title,
            style = androidx.compose.material3.MaterialTheme.typography.titleLarge.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold),
            color = titleColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
        )
        if (!description.isNullOrEmpty()) {
            androidx.compose.foundation.layout.Spacer(modifier = androidx.compose.ui.Modifier.height(androidx.compose.ui.unit.Dp(8f)))
            androidx.compose.material3.Text(
                description,
                style = androidx.compose.material3.MaterialTheme.typography.bodyLarge,
                color = descriptionColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = androidx.compose.ui.text.style.TextAlign.Center,
            )
        }
    }
}

// nexa-codegen: NexaColumnPrimitive
@androidx.compose.runtime.Composable
internal fun NexaColumnPrimitive(
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    spacing: Float = 0f,
    horizontalAlignment: androidx.compose.ui.Alignment.Horizontal = androidx.compose.ui.Alignment.CenterHorizontally,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val arrangement = if (spacing > 0f) {
        androidx.compose.foundation.layout.Arrangement.spacedBy(androidx.compose.ui.unit.Dp(spacing))
    } else androidx.compose.foundation.layout.Arrangement.Top
    val textAlignment = when (horizontalAlignment) {
        androidx.compose.ui.Alignment.Start -> androidx.compose.ui.text.style.TextAlign.Start
        androidx.compose.ui.Alignment.CenterHorizontally -> androidx.compose.ui.text.style.TextAlign.Center
        androidx.compose.ui.Alignment.End -> androidx.compose.ui.text.style.TextAlign.End
        else -> null
    }
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNexaTextAlignment provides textAlignment,
    ) {
        androidx.compose.foundation.layout.Column(modifier, verticalArrangement = arrangement, horizontalAlignment = horizontalAlignment, content = content)
    }
}

// nexa-codegen: NexaRowPrimitive
@androidx.compose.runtime.Composable
internal fun NexaRowPrimitive(
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    spacing: Float = 0f,
    horizontalArrangement: androidx.compose.foundation.layout.Arrangement.Horizontal? = null,
    verticalAlignment: androidx.compose.ui.Alignment.Vertical = androidx.compose.ui.Alignment.CenterVertically,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit,
) {
    val spacingAlignment = when (horizontalArrangement) {
        androidx.compose.foundation.layout.Arrangement.Start -> androidx.compose.ui.Alignment.Start
        androidx.compose.foundation.layout.Arrangement.Center -> androidx.compose.ui.Alignment.CenterHorizontally
        androidx.compose.foundation.layout.Arrangement.End -> androidx.compose.ui.Alignment.End
        else -> androidx.compose.ui.Alignment.Start
    }
    val arrangement = when {
        horizontalArrangement != null && spacing > 0f ->
            androidx.compose.foundation.layout.Arrangement.spacedBy(
                androidx.compose.ui.unit.Dp(spacing),
                spacingAlignment,
            )
        horizontalArrangement != null -> horizontalArrangement
        spacing > 0f -> androidx.compose.foundation.layout.Arrangement.spacedBy(
            androidx.compose.ui.unit.Dp(spacing),
        )
        else -> androidx.compose.foundation.layout.Arrangement.Start
    }
    val rowModifier = if (horizontalArrangement != null) modifier.fillMaxWidth() else modifier
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNexaTextAlignment provides null,
    ) {
        androidx.compose.foundation.layout.Row(rowModifier, horizontalArrangement = arrangement, verticalAlignment = verticalAlignment, content = content)
    }
}

// nexa-codegen: NexaStackPrimitive
@androidx.compose.runtime.Composable
internal fun NexaStackPrimitive(
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    contentAlignment: androidx.compose.ui.Alignment = androidx.compose.ui.Alignment.Center,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.BoxScope.() -> Unit,
) {
    val textAlignment = when (contentAlignment) {
        androidx.compose.ui.Alignment.TopStart -> androidx.compose.ui.text.style.TextAlign.Start
        androidx.compose.ui.Alignment.Center -> androidx.compose.ui.text.style.TextAlign.Center
        androidx.compose.ui.Alignment.BottomEnd -> androidx.compose.ui.text.style.TextAlign.End
        else -> null
    }
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNexaTextAlignment provides textAlignment,
    ) {
        androidx.compose.foundation.layout.Box(modifier, contentAlignment = contentAlignment, content = content)
    }
}

// nexa-codegen: NexaFormRowPrimitive
@androidx.compose.runtime.Composable
internal fun NexaFormRowPrimitive(
    minHeight: Float,
    horizontalInset: Float,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.BoxScope.() -> Unit,
) {
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNexaTextAlignment provides androidx.compose.ui.text.style.TextAlign.Start,
    ) {
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier
                .fillMaxWidth()
                .heightIn(min = androidx.compose.ui.unit.Dp(minHeight))
                .padding(horizontal = androidx.compose.ui.unit.Dp(horizontalInset)),
            contentAlignment = androidx.compose.ui.Alignment.CenterStart,
            content = content,
        )
    }
}

// nexa-codegen: NexaAccessibilityPrimitive
@androidx.compose.runtime.Composable
internal fun NexaAccessibilityPrimitive(
    label: String,
    hint: String?,
    value: String?,
    role: String?,
    isHeading: Boolean,
    omitLabel: Boolean,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.BoxScope.() -> Unit,
) {
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier.semantics(mergeDescendants = true) {
            val description = when {
                omitLabel && hint != null -> hint
                omitLabel -> null
                hint != null -> "$label, $hint"
                else -> label
            }
            if (description != null) contentDescription = description
            if (value != null) stateDescription = value
            if (isHeading) heading()
            else when (role) {
                "Button" -> this.role = androidx.compose.ui.semantics.Role.Button
                "Image" -> this.role = androidx.compose.ui.semantics.Role.Image
            }
        },
        content = content,
    )
}

// nexa-codegen: NexaLinkPrimitive
@androidx.compose.runtime.Composable
internal fun NexaLinkPrimitive(
    onClick: () -> Unit,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.BoxScope.() -> Unit,
) {
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier.clickable(
            interactionSource = null,
            onClick = onClick,
        ),
        content = content,
    )
}

// nexa-codegen: NexaFormPrimitive
@androidx.compose.runtime.Composable
internal fun NexaFormPrimitive(
    background: androidx.compose.ui.graphics.Color? = null,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val scrollState = androidx.compose.foundation.rememberScrollState()
    androidx.compose.foundation.layout.Column(
        modifier = androidx.compose.ui.Modifier
            .fillMaxSize()
            .background(background ?: androidx.compose.ui.graphics.Color.Transparent)
            .verticalScroll(scrollState, enabled = scrollState.maxValue > 0),
    ) {
        androidx.compose.foundation.layout.Spacer(
            modifier = androidx.compose.ui.Modifier.height(LocalNexaFormContentTopInset.current),
        )
        content()
    }
}

// nexa-codegen: LocalNexaFormContentTopInset
internal val LocalNexaFormContentTopInset =
    androidx.compose.runtime.staticCompositionLocalOf {
        androidx.compose.ui.unit.Dp(__NEXA_FORM_CONTENT_TOP_INSET__.toFloat())
    }

// Navigation destinations begin immediately below their native app bar.
internal val LocalNexaNavigationContent = androidx.compose.runtime.staticCompositionLocalOf { false }

// nexa-codegen: NexaFormSectionPrimitive
@androidx.compose.runtime.Composable
internal fun NexaFormSectionPrimitive(
    title: String?,
    footer: String?,
    firstSection: Boolean = false,
    background: androidx.compose.ui.graphics.Color? = null,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    androidx.compose.foundation.layout.Column(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .padding(
                start = androidx.compose.ui.unit.Dp(__NEXA_FORM_SECTION_HORIZONTAL_INSET__.toFloat()),
                top = androidx.compose.ui.unit.Dp(
                    if (firstSection && LocalNexaNavigationContent.current) 0f
                    else __NEXA_FORM_SECTION_TOP_PADDING__.toFloat(),
                ),
                end = androidx.compose.ui.unit.Dp(__NEXA_FORM_SECTION_HORIZONTAL_INSET__.toFloat()),
                bottom = androidx.compose.ui.unit.Dp(
                    if (footer == null) {
                        __NEXA_FORM_SECTION_BOTTOM_PADDING__.toFloat()
                    } else {
                        __NEXA_FORM_SECTION_TOP_PADDING__.toFloat()
                    },
                ),
            ),
    ) {
        if (title != null) {
            androidx.compose.material3.Text(
                title,
                modifier = androidx.compose.ui.Modifier.padding(
                    top = androidx.compose.ui.unit.Dp(
                        if (firstSection && LocalNexaNavigationContent.current) 0f else 8f,
                    ),
                    bottom = androidx.compose.ui.unit.Dp(8f),
                ),
                style = androidx.compose.material3.MaterialTheme.typography.titleSmall,
                color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        androidx.compose.material3.Surface(
            modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
            shape = androidx.compose.foundation.shape.RoundedCornerShape(androidx.compose.ui.unit.Dp(24f)),
            color = background ?: androidx.compose.ui.graphics.Color.Transparent,
            contentColor = androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
        ) {
            androidx.compose.foundation.layout.Column(content = content)
        }
        if (footer != null) {
            androidx.compose.material3.Text(
                footer,
                modifier = androidx.compose.ui.Modifier.padding(
                    start = androidx.compose.ui.unit.Dp(__NEXA_FORM_FOOTER_START_INSET__.toFloat()),
                    top = androidx.compose.ui.unit.Dp(8f),
                    bottom = androidx.compose.ui.unit.Dp(8f),
                ),
                fontSize = androidx.compose.ui.unit.TextUnit(__NEXA_FORM_FOOTER_FONT_SIZE__.toFloat(), androidx.compose.ui.unit.TextUnitType.Sp),
                lineHeight = androidx.compose.ui.unit.TextUnit(__NEXA_FORM_FOOTER_FONT_SIZE__.toFloat() * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f, androidx.compose.ui.unit.TextUnitType.Sp),
                color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// nexa-codegen: NexaFormDividerPrimitive
@androidx.compose.runtime.Composable
internal fun NexaFormDividerPrimitive() {
    NexaSystemDividerPrimitive(
        startInset = androidx.compose.ui.unit.Dp(__NEXA_FORM_DIVIDER_START_INSET__.toFloat()),
        endInset = androidx.compose.ui.unit.Dp(__NEXA_FORM_DIVIDER_END_INSET__.toFloat()),
    )
}

// nexa-codegen: LocalNexaPickerPressFeedback
internal val LocalNexaPickerPressFeedback = androidx.compose.runtime.compositionLocalOf { true }

// nexa-codegen: LocalNexaBottomSheetToolbarButtons
internal val LocalNexaBottomSheetToolbarButtons = androidx.compose.runtime.compositionLocalOf { false }

// nexa-codegen: LocalNexaDialogActionButtons
internal val LocalNexaDialogActionButtons = androidx.compose.runtime.compositionLocalOf { false }

// nexa-codegen: NexaPickerPrimitive
@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class, androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class)
internal fun NexaPickerPrimitive(
    items: List<String>,
    itemIcons: List<androidx.compose.ui.graphics.vector.ImageVector?>,
    selected: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector?,
    label: String?,
    tint: androidx.compose.ui.graphics.Color?,
    identity: String? = null,
    fillMaxWidth: Boolean = true,
    onSelectionChanged: (String) -> Unit,
) {
    val expanded = androidx.compose.runtime.remember(identity) { androidx.compose.runtime.mutableStateOf(false) }
    val showPressFeedback = LocalNexaPickerPressFeedback.current
    val resolvedTint = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary
    val rowWidthModifier = if (fillMaxWidth && label != null) {
        androidx.compose.ui.Modifier.fillMaxWidth()
    } else {
        androidx.compose.ui.Modifier.wrapContentWidth(androidx.compose.ui.Alignment.CenterHorizontally)
    }
    val clickModifier = if (showPressFeedback) {
        androidx.compose.ui.Modifier.clickable { expanded.value = true }
    } else {
        androidx.compose.ui.Modifier.clickable(
            interactionSource = null,
            indication = null,
            onClick = { expanded.value = true },
        )
    }
    androidx.compose.foundation.layout.Row(
        modifier = rowWidthModifier.then(clickModifier),
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    ) {
        if (label != null && icon != null) {
            androidx.compose.material3.Icon(
                imageVector = icon,
                contentDescription = null,
                tint = resolvedTint,
                modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat())),
            )
            androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.width(androidx.compose.ui.unit.Dp(__NEXA_ICON_LABEL_SPACING__.toFloat())))
        }
        if (label != null) {
            NexaTextPrimitive(
                text = label,
                modifier = androidx.compose.ui.Modifier.weight(1f),
                color = resolvedTint,
                fontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
                lineHeight = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
                letterSpacing = 0.sp,
                textAlign = androidx.compose.ui.text.style.TextAlign.Start,
            )
        }
        androidx.compose.foundation.layout.Box {
            androidx.compose.foundation.layout.Row(
                modifier = androidx.compose.ui.Modifier.padding(vertical = androidx.compose.ui.unit.Dp(8f)),
                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
            ) {
                if (icon != null && label == null) {
                    NexaSystemIconPrimitive(
                        image = icon,
                        description = selected,
                        size = androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat()),
                        tint = tint ?: if (androidx.compose.material3.MaterialTheme.colorScheme.background == androidx.compose.ui.graphics.Color.Black) {
                            androidx.compose.ui.graphics.Color(__NEXA_DARK_ACCENT_ARGB__)
                        } else {
                            androidx.compose.ui.graphics.Color(__NEXA_DEFAULT_ACCENT_ARGB__)
                        },
                    )
                } else {
                    NexaTextPrimitive(
                        text = selected,
                        color = resolvedTint,
                        fontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
                        fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
                        maxLines = 1,
                        lineHeight = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
                        letterSpacing = 0.sp,
                        softWrap = false,
                    )
                    androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.width(androidx.compose.ui.unit.Dp(4f)))
                    androidx.compose.foundation.Canvas(
                        modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(16f)),
                    ) {
                        val path = androidx.compose.ui.graphics.Path().apply {
                            moveTo(size.width * 0.28f, size.height * 0.39f)
                            lineTo(size.width * 0.5f, size.height * 0.18f)
                            lineTo(size.width * 0.72f, size.height * 0.39f)
                            moveTo(size.width * 0.28f, size.height * 0.61f)
                            lineTo(size.width * 0.5f, size.height * 0.82f)
                            lineTo(size.width * 0.72f, size.height * 0.61f)
                        }
                        drawPath(
                            path = path,
                            color = resolvedTint,
                            style = androidx.compose.ui.graphics.drawscope.Stroke(
                                width = size.minDimension * 0.1f,
                                cap = androidx.compose.ui.graphics.StrokeCap.Round,
                                join = androidx.compose.ui.graphics.StrokeJoin.Round,
                            ),
                        )
                    }
                }
            }
            androidx.compose.material3.MaterialExpressiveTheme(
                colorScheme = androidx.compose.material3.MaterialTheme.colorScheme,
            ) {
                androidx.compose.material3.DropdownMenu(
                    expanded = expanded.value,
                    onDismissRequest = { expanded.value = false },
                    shape = androidx.compose.material3.MenuDefaults.standaloneGroupShape,
                    containerColor = androidx.compose.material3.MenuDefaults.groupStandardContainerColor,
                    tonalElevation = 0.dp,
                    shadowElevation = 8.dp,
                ) {
                    if (items.isNotEmpty()) {
                        items.forEachIndexed { index, item ->
                            androidx.compose.material3.DropdownMenuItem(
                                shape = androidx.compose.material3.MenuDefaults.itemShape(
                                    index = index,
                                    count = items.size,
                                ).shape,
                                leadingIcon = itemIcons.getOrNull(index)?.let { optionIcon ->
                                    {
                                        androidx.compose.material3.Icon(
                                            imageVector = optionIcon,
                                            contentDescription = null,
                                            tint = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                },
                                text = {
                                    NexaTextPrimitive(
                                        text = item,
                                        color = if (item == selected) tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary
                                            else androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
                                        fontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
                                        fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
                                        maxLines = 1,
                                        lineHeight = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
                                        letterSpacing = 0.sp,
                                        softWrap = false,
                                    )
                                },
                                onClick = {
                                    onSelectionChanged(item)
                                    expanded.value = false
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}

"#,
    r#"// nexa-codegen: NexaPageIndicatorPrimitive
@androidx.compose.runtime.Composable
internal fun NexaPageIndicatorPrimitive(
    index: Int,
    selected: Boolean,
    onSelect: () -> Unit,
) {
    val size = androidx.compose.ui.unit.Dp(
        (if (selected) __NEXA_PAGE_INDICATOR_SELECTED_SIZE__ else __NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__).toFloat()
    )
    val color = if (selected) {
        androidx.compose.material3.MaterialTheme.colorScheme.primary
    } else {
        androidx.compose.ui.graphics.Color(__NEXA_MUTED_TEXT_ARGB__).copy(alpha = __NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__.toFloat())
    }
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier
            .size(size)
            .background(color, shape = androidx.compose.foundation.shape.CircleShape)
            .semantics { contentDescription = "Page ${index + 1}" }
            .clickable(
                role = androidx.compose.ui.semantics.Role.Button,
                onClickLabel = "Page ${index + 1}",
                onClick = onSelect,
            ),
    )
}

// nexa-codegen: NexaNavigationChrome
@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun NexaNavigationChrome(
    identity: String,
    title: String,
    largeTitle: Boolean,
    centerTitle: Boolean,
    containerColor: androidx.compose.ui.graphics.Color,
    modifier: androidx.compose.ui.Modifier,
    navigationIcon: @androidx.compose.runtime.Composable () -> Unit = {},
    actions: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {},
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    androidx.compose.runtime.key(identity) {
        val navigationTitleSlot = LocalNexaNavigationTitleSlot.current
        if (navigationTitleSlot != null) {
            androidx.compose.runtime.SideEffect {
                navigationTitleSlot.update(
                    identity = identity,
                    title = title,
                    largeTitle = largeTitle,
                    centerTitle = centerTitle,
                    leadingContent = navigationIcon,
                    trailingContent = actions,
                )
            }
            androidx.compose.runtime.DisposableEffect(navigationTitleSlot, identity) {
                onDispose { navigationTitleSlot.clear(identity) }
            }
            androidx.compose.foundation.layout.Column(modifier) {
                content(this@Column)
            }
        } else {
            val scrollBehavior = if (largeTitle) {
                androidx.compose.material3.TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
            } else {
                androidx.compose.material3.TopAppBarDefaults.pinnedScrollBehavior()
            }
            val colors = androidx.compose.material3.TopAppBarDefaults.topAppBarColors(
                containerColor = containerColor,
                scrolledContainerColor = containerColor,
            )
            androidx.compose.material3.Scaffold(
                modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
                containerColor = containerColor,
                topBar = {
                    val titleContent: @androidx.compose.runtime.Composable () -> Unit = {
                        if (largeTitle) {
                            androidx.compose.material3.Text(
                                title,
                                maxLines = 1,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                            )
                        } else {
                            androidx.compose.material3.Text(
                                title,
                                style = NexaNavigationChromeTextStyle(),
                                maxLines = 1,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                            )
                        }
                    }
                    if (largeTitle) {
                        androidx.compose.material3.LargeFlexibleTopAppBar(
                            title = titleContent,
                            navigationIcon = navigationIcon,
                            actions = actions,
                            scrollBehavior = scrollBehavior,
                            colors = colors,
                        )
                    } else if (centerTitle) {
                        androidx.compose.material3.CenterAlignedTopAppBar(
                            title = titleContent,
                            navigationIcon = navigationIcon,
                            actions = actions,
                            scrollBehavior = scrollBehavior,
                            colors = colors,
                        )
                    } else {
                        androidx.compose.material3.TopAppBar(
                            title = titleContent,
                            navigationIcon = navigationIcon,
                            actions = actions,
                            scrollBehavior = scrollBehavior,
                            colors = colors,
                        )
                    }
                },
            ) { innerPadding ->
                androidx.compose.foundation.layout.Box(
                    modifier = androidx.compose.ui.Modifier
                        .fillMaxSize()
                        .padding(innerPadding)
                        .background(containerColor),
                ) {
                    androidx.compose.runtime.CompositionLocalProvider(
                        LocalNexaFormContentTopInset provides androidx.compose.ui.unit.Dp(0f),
                        LocalNexaNavigationContent provides true,
                    ) {
                        androidx.compose.foundation.layout.Column(
                            modifier = androidx.compose.ui.Modifier.fillMaxSize(),
                        ) {
                            content(this@Column)
                        }
                    }
                }
            }
        }
    }
}

// nexa-codegen: NexaPageIndicatorRow
@androidx.compose.runtime.Composable
internal fun NexaPageIndicatorRow(
    pageCount: Int,
    currentPage: Int,
    onSelect: (Int) -> Unit,
) {
    androidx.compose.foundation.layout.Row(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .padding(bottom = androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_BOTTOM_INSET__.toFloat())),
        horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(
            androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_SPACING__.toFloat()),
            androidx.compose.ui.Alignment.CenterHorizontally,
        ),
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    ) {
        repeat(pageCount) { index ->
            NexaPageIndicatorPrimitive(index = index, selected = currentPage == index) {
                onSelect(index)
            }
        }
    }
}
"#
);

/// The adaptive tab shell used by both release output and DevRuntime. Keep
/// the NavigationSuite setup and tint semantics here so dynamic tabs do not
/// grow a second Compose implementation.
const NEXA_APP_BOTTOM_BAR_PRIMITIVE: &str = r#"@androidx.compose.runtime.Composable
internal fun NexaAppBottomBarPrimitive(
    tint: androidx.compose.ui.graphics.Color?,
    showNavigationSuite: Boolean = true,
    navigationSuiteItems: androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScope.(androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteItemColors?) -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val itemColors = if (tint == null) {
        null
    } else {
        androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteDefaults.itemColors(
            navigationBarItemColors = androidx.compose.material3.NavigationBarItemDefaults.colors(
                selectedIconColor = tint,
                selectedTextColor = tint,
                indicatorColor = tint.copy(alpha = 0.12f),
            ),
            navigationRailItemColors = androidx.compose.material3.NavigationRailItemDefaults.colors(
                selectedIconColor = tint,
                selectedTextColor = tint,
                indicatorColor = tint.copy(alpha = 0.12f),
            ),
            navigationDrawerItemColors = androidx.compose.material3.NavigationDrawerItemDefaults.colors(
                selectedIconColor = tint,
                selectedTextColor = tint,
                selectedContainerColor = tint.copy(alpha = 0.12f),
            ),
        )
    }
    if (showNavigationSuite) {
        androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold(
            navigationSuiteItems = { navigationSuiteItems(itemColors) },
            content = content,
        )
    } else {
        content()
    }
}

"#;

const NEXA_FAST_LIST_ROW_PRIMITIVE: &str = r#"@androidx.compose.runtime.Composable
internal fun NexaFastListRowPrimitive(
    modifier: androidx.compose.ui.Modifier,
    horizontalAlignment: androidx.compose.ui.Alignment.Horizontal,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    androidx.compose.foundation.layout.Column(
        modifier = modifier,
        horizontalAlignment = horizontalAlignment,
        content = content,
    )
}

"#;

const NEXA_HORIZONTAL_LIST_NESTED_SCROLL_PRIMITIVE: &str = r#"private object NexaHorizontalListScrollBoundary : androidx.compose.ui.input.nestedscroll.NestedScrollConnection {
    override fun onPostScroll(
        consumed: androidx.compose.ui.geometry.Offset,
        available: androidx.compose.ui.geometry.Offset,
        source: androidx.compose.ui.input.nestedscroll.NestedScrollSource,
    ): androidx.compose.ui.geometry.Offset =
        if (source == androidx.compose.ui.input.nestedscroll.NestedScrollSource.UserInput && available.x != 0f) {
            androidx.compose.ui.geometry.Offset(available.x, 0f)
        } else {
            androidx.compose.ui.geometry.Offset.Zero
        }

    override suspend fun onPostFling(
        consumed: androidx.compose.ui.unit.Velocity,
        available: androidx.compose.ui.unit.Velocity,
    ): androidx.compose.ui.unit.Velocity = androidx.compose.ui.unit.Velocity(available.x, 0f)
}

"#;

const NEXA_NATIVE_LIST_PRIMITIVES: &str = r#"@androidx.compose.runtime.Composable
private fun nexaNativeListGroupedBackground(): androidx.compose.ui.graphics.Color =
    if (androidx.compose.material3.MaterialTheme.colorScheme.background == androidx.compose.ui.graphics.Color.Black) {
        androidx.compose.ui.graphics.Color.Black
    } else {
        androidx.compose.material3.MaterialTheme.colorScheme.surfaceVariant
    }

@androidx.compose.runtime.Composable
internal fun NexaNativeListContainerModifier(): androidx.compose.ui.Modifier =
    androidx.compose.ui.Modifier
        .fillMaxSize()
        .background(nexaNativeListGroupedBackground())
        .padding(
            horizontal = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_CONTENT_HORIZONTAL_INSET__.toFloat()),
            vertical = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__.toFloat()),
        )

@androidx.compose.runtime.Composable
internal fun NexaNativeListSectionHeaderPrimitive(
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    androidx.compose.foundation.layout.Column(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .background(nexaNativeListGroupedBackground())
            .padding(
                horizontal = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_SECTION_HEADER_HORIZONTAL_INSET__.toFloat()),
                vertical = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_SECTION_HEADER_VERTICAL_INSET__.toFloat()),
            ),
        content = content,
    )
}

@androidx.compose.runtime.Composable
internal fun NexaNativeListSectionSpacingPrimitive() {
    androidx.compose.foundation.layout.Spacer(
        modifier = androidx.compose.ui.Modifier.height(
            androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_SECTION_SPACING__.toFloat()),
        ),
    )
}

@androidx.compose.runtime.Composable
internal fun NexaNativeListRowPrimitive(
    position: Int,
    count: Int,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val corner = androidx.compose.ui.unit.Dp(24f)
    val shape = when {
        count <= 1 -> androidx.compose.foundation.shape.RoundedCornerShape(corner)
        position == 0 -> androidx.compose.foundation.shape.RoundedCornerShape(topStart = corner, topEnd = corner)
        position == count - 1 -> androidx.compose.foundation.shape.RoundedCornerShape(bottomStart = corner, bottomEnd = corner)
        else -> androidx.compose.foundation.shape.RoundedCornerShape(androidx.compose.ui.unit.Dp(0f))
    }
    val isDark = androidx.compose.material3.MaterialTheme.colorScheme.background == androidx.compose.ui.graphics.Color.Black
    androidx.compose.foundation.layout.Column(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            // Match the measured 52pt minimum row in iOS inset-grouped List.
            .heightIn(min = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_MIN_HEIGHT__.toFloat()))
            .clip(shape)
            .background(
                if (isDark) androidx.compose.ui.graphics.Color(__NEXA_FORM_DARK_SECTION_ARGB__)
                else androidx.compose.ui.graphics.Color(__NEXA_FORM_LIGHT_SECTION_ARGB__),
            )
            .padding(
                horizontal = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__.toFloat()),
                vertical = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__.toFloat()),
            ),
        content = content,
    )
}

@androidx.compose.runtime.Composable
internal fun NexaNativeListDividerPrimitive() {
    NexaSystemDividerPrimitive(
        startInset = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_DIVIDER_START_INSET__.toFloat()),
        endInset = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_DIVIDER_END_INSET__.toFloat()),
    )
}

"#;

const NEXA_FAST_LIST_SWIPE_PRIMITIVE: &str = r#"@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun NexaFastListSwipePrimitive(
    onEndToStart: () -> Unit,
    backgroundContent: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val state = androidx.compose.material3.rememberSwipeToDismissBoxState()
    androidx.compose.runtime.LaunchedEffect(state.currentValue) {
        if (state.currentValue == androidx.compose.material3.SwipeToDismissBoxValue.EndToStart) {
            onEndToStart()
        }
    }
    androidx.compose.material3.SwipeToDismissBox(
        state = state,
        backgroundContent = {
            androidx.compose.foundation.layout.Row(
                modifier = androidx.compose.ui.Modifier.fillMaxSize(),
                horizontalArrangement = androidx.compose.foundation.layout.Arrangement.End,
                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                content = backgroundContent,
            )
        },
        content = {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxWidth()
                    .background(androidx.compose.material3.MaterialTheme.colorScheme.surface),
            ) {
                content()
            }
        },
    )
}

"#;

fn shared_component_primitives() -> String {
    let qualified_scheme = |dark| {
        nexa_codegen::design_system::kotlin_color_scheme(dark)
            .replace(
                "darkColorScheme(",
                "androidx.compose.material3.darkColorScheme(",
            )
            .replace(
                "lightColorScheme(",
                "androidx.compose.material3.lightColorScheme(",
            )
            .replace("Color(", "androidx.compose.ui.graphics.Color(")
            .replace("Color.White", "androidx.compose.ui.graphics.Color.White")
    };
    let source = NEXA_SHARED_COMPONENT_PRIMITIVES
        .replace("__NEXA_DARK_COLOR_SCHEME__", &qualified_scheme(true))
        .replace("__NEXA_LIGHT_COLOR_SCHEME__", &qualified_scheme(false));
    let tokens = [
        (
            "__NEXA_BUTTON_LOADING_INDICATOR_SIZE__",
            nexa_codegen::design_system::BUTTON_LOADING_INDICATOR_SIZE.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_MIN_HEIGHT__",
            nexa_codegen::design_system::BUTTON_LARGE_MIN_HEIGHT.to_string(),
        ),
        (
            "__NEXA_BUTTON_MIN_TAP_TARGET__",
            nexa_codegen::design_system::BUTTON_MIN_TAP_TARGET.to_string(),
        ),
        (
            "__NEXA_BUTTON_MIN_WIDTH__",
            nexa_codegen::design_system::BUTTON_MIN_WIDTH.to_string(),
        ),
        (
            "__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_SMALL_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_SMALL_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_SMALL_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_LARGE_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_LARGE_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_LARGE_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_DEFAULT_HORIZONTAL_PADDING__",
            nexa_codegen::design_system::BUTTON_DEFAULT_HORIZONTAL_PADDING.to_string(),
        ),
        (
            "__NEXA_BUTTON_DEFAULT_VERTICAL_PADDING__",
            nexa_codegen::design_system::BUTTON_DEFAULT_VERTICAL_PADDING.to_string(),
        ),
        (
            "__NEXA_DEFAULT_MATERIAL_ICON_SIZE__",
            nexa_codegen::design_system::DEFAULT_MATERIAL_ICON_SIZE.to_string(),
        ),
        (
            "__NEXA_ICON_LABEL_SPACING__",
            nexa_codegen::design_system::ICON_LABEL_SPACING.to_string(),
        ),
        (
            "__NEXA_DEFAULT_BODY_FONT_SIZE__",
            nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE.to_string(),
        ),
        (
            "__NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__",
            nexa_codegen::design_system::DEFAULT_LINE_HEIGHT_MULTIPLIER.to_string(),
        ),
        (
            "__NEXA_FORM_SECTION_HORIZONTAL_INSET__",
            nexa_codegen::design_system::FORM_SECTION_HORIZONTAL_INSET.to_string(),
        ),
        (
            "__NEXA_FORM_CONTENT_TOP_INSET__",
            nexa_codegen::design_system::ANDROID_FORM_CONTENT_TOP_INSET.to_string(),
        ),
        (
            "__NEXA_FORM_SECTION_TOP_PADDING__",
            nexa_codegen::design_system::FORM_SECTION_TOP_PADDING.to_string(),
        ),
        (
            "__NEXA_FORM_SECTION_BOTTOM_PADDING__",
            nexa_codegen::design_system::FORM_SECTION_BOTTOM_PADDING.to_string(),
        ),
        (
            "__NEXA_FORM_DIVIDER_START_INSET__",
            nexa_codegen::design_system::FORM_DIVIDER_START_INSET.to_string(),
        ),
        (
            "__NEXA_FORM_DIVIDER_END_INSET__",
            nexa_codegen::design_system::FORM_DIVIDER_END_INSET.to_string(),
        ),
        (
            "__NEXA_FORM_FOOTER_START_INSET__",
            nexa_codegen::design_system::FORM_FOOTER_START_INSET.to_string(),
        ),
        (
            "__NEXA_FORM_FOOTER_FONT_SIZE__",
            nexa_codegen::design_system::FORM_FOOTER_FONT_SIZE.to_string(),
        ),
        (
            "__NEXA_FORM_DARK_SECTION_ARGB__",
            format!(
                "0x{:08X}",
                nexa_codegen::design_system::FORM_DARK_SECTION_ARGB
            ),
        ),
        (
            "__NEXA_FORM_LIGHT_SECTION_ARGB__",
            format!(
                "0x{:08X}",
                nexa_codegen::design_system::FORM_LIGHT_SECTION_ARGB
            ),
        ),
        (
            "__NEXA_SYSTEM_SEPARATOR_DARK_ARGB__",
            format!(
                "0x{:08X}",
                nexa_codegen::design_system::SYSTEM_SEPARATOR_DARK_ARGB
            ),
        ),
        (
            "__NEXA_SYSTEM_SEPARATOR_LIGHT_ARGB__",
            format!(
                "0x{:08X}",
                nexa_codegen::design_system::SYSTEM_SEPARATOR_LIGHT_ARGB
            ),
        ),
        (
            "__NEXA_PAGE_INDICATOR_SELECTED_SIZE__",
            nexa_codegen::design_system::PAGE_INDICATOR_SELECTED_SIZE.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__",
            nexa_codegen::design_system::PAGE_INDICATOR_UNSELECTED_SIZE.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_SPACING__",
            nexa_codegen::design_system::PAGE_INDICATOR_SPACING.to_string(),
        ),
        (
            "__NEXA_PAGE_INDICATOR_BOTTOM_INSET__",
            nexa_codegen::design_system::PAGE_INDICATOR_BOTTOM_INSET.to_string(),
        ),
        (
            "__NEXA_MUTED_TEXT_ARGB__",
            format!("0x{:08X}", nexa_codegen::design_system::MUTED_TEXT_ARGB),
        ),
        (
            "__NEXA_PAGE_INDICATOR_INACTIVE_OPACITY__",
            nexa_codegen::design_system::PAGE_INDICATOR_INACTIVE_OPACITY.to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_HEIGHT__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .header_height
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_COLLAPSE_DISTANCE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .collapse_distance
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_COLLAPSED_FONT_SIZE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .collapsed_font_size
                .to_string(),
        ),
        (
            "__NEXA_NAVIGATION_TRAILING_ACTION_INSET__",
            nexa_codegen::design_system::NAVIGATION_TRAILING_ACTION_INSET.to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_EXPANDED_FONT_SIZE__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .expanded_font_size
                .to_string(),
        ),
        (
            "__NEXA_TITLE_BAR_HORIZONTAL_INSET__",
            nexa_codegen::design_system::TITLE_BAR_METRICS
                .horizontal_inset
                .to_string(),
        ),
        (
            "__NEXA_DARK_ACCENT_ARGB__",
            format!("0x{:08X}", nexa_codegen::design_system::DARK_ACCENT_ARGB),
        ),
        (
            "__NEXA_DEFAULT_ACCENT_ARGB__",
            format!("0x{:08X}", nexa_codegen::design_system::DEFAULT_ACCENT_ARGB),
        ),
    ];
    let mut source = tokens.iter().fold(source, |source, (token, value)| {
        source.replace(token, value)
    });
    source.push_str(include_str!("components/date_picker.kt"));
    source
}

const NEXA_REGEX_HELPERS: &str = r#"internal data class NexaRegexMatch(
    val value: String,
    val range: NexaRegexRange,
    val groups: List<String?>,
)

internal data class NexaRegexRange(
    val lowerBound: Long,
    val upperBound: Long,
)

internal class NexaRegex(pattern: String) {
    private val expression: Regex? = runCatching { Regex(pattern) }.getOrNull()

    fun matches(text: String): Boolean = expression?.matches(text) == true

    fun find(text: String): NexaRegexMatch? = expression?.find(text)?.let(::toNexaRegexMatch)

    fun findAll(text: String): List<NexaRegexMatch> =
        expression?.findAll(text)?.map(::toNexaRegexMatch)?.toList().orEmpty()

    fun replace(text: String, with: String): String = expression?.replace(text, with) ?: text

    private fun toNexaRegexMatch(match: MatchResult): NexaRegexMatch {
        val groups = (1 until match.groups.size).map { index -> match.groups[index]?.value }
        return NexaRegexMatch(
            value = match.value,
            range = NexaRegexRange(match.range.first.toLong(), (match.range.last + 1).toLong()),
            groups = groups,
        )
    }
}

"#;

fn generate_with_analysis(
    module: &Module,
    features: &features::Features,
    include_all_shared_primitives: bool,
) -> GeneratedSources {
    let focus_bindings = features.facts.focus_bindings.app.clone();
    let imports = engine::imports::render(engine::imports::ImportContext {
        features,
        include_all_shared_primitives,
        uses_plugins: !module.plugins.is_empty(),
        has_navigation: !module.screens.is_empty(),
        has_direction: module.direction.is_some(),
        has_on_appear: module.on_appear.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.on_appear.is_some())
            || module
                .components
                .iter()
                .any(|component| component.on_appear.is_some()),
        has_on_disappear: module.on_disappear.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.on_disappear.is_some())
            || module
                .components
                .iter()
                .any(|component| component.on_disappear.is_some()),
        has_lifecycle_events: module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some()
            || !module.widgets.is_empty(),
        has_widgets: !module.widgets.is_empty(),
        has_immutable_structs: structs::has_immutable_structs(module),
    });
    let value_codecs = nexa_codegen::value::collect(module);
    let json_types = nexa_codegen::value::collect_json_types(module);
    let mut units = SourceUnits::new("kt");
    units.set_imports(&imports);
    units.write("types", |out| {
        if features.uses_fast_list {
            out.push_str(NEXA_FAST_LIST_ROW_PRIMITIVE);
            let native_list_primitives = NEXA_NATIVE_LIST_PRIMITIVES
                .replace(
                    "__NEXA_FORM_DARK_SECTION_ARGB__",
                    &format!(
                        "0x{:08X}",
                        nexa_codegen::design_system::FORM_DARK_SECTION_ARGB
                    ),
                )
                .replace(
                    "__NEXA_FORM_LIGHT_SECTION_ARGB__",
                    &format!(
                        "0x{:08X}",
                        nexa_codegen::design_system::FORM_LIGHT_SECTION_ARGB
                    ),
                )
                .replace(
                    "__NEXA_SYSTEM_SEPARATOR_DARK_ARGB__",
                    &format!(
                        "0x{:08X}",
                        nexa_codegen::design_system::SYSTEM_SEPARATOR_DARK_ARGB
                    ),
                )
                .replace(
                    "__NEXA_SYSTEM_SEPARATOR_LIGHT_ARGB__",
                    &format!(
                        "0x{:08X}",
                        nexa_codegen::design_system::SYSTEM_SEPARATOR_LIGHT_ARGB
                    ),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_ROW_VERTICAL_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_VERTICAL_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_HORIZONTAL_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_ROW_MIN_HEIGHT__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_MIN_HEIGHT
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_DIVIDER_START_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_DIVIDER_START_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_DIVIDER_END_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_DIVIDER_END_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_CONTENT_HORIZONTAL_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_CONTENT_HORIZONTAL_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_CONTENT_VERTICAL_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_CONTENT_VERTICAL_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_SECTION_HEADER_HORIZONTAL_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_SECTION_HEADER_HORIZONTAL_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_SECTION_HEADER_VERTICAL_INSET__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_SECTION_HEADER_VERTICAL_INSET
                        .to_string(),
                )
                .replace(
                    "__NEXA_NATIVE_LIST_SECTION_SPACING__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_SECTION_SPACING.to_string(),
                );
            out.push_str(&native_list_primitives);
        }
        if features.uses_horizontal_list {
            out.push_str(NEXA_HORIZONTAL_LIST_NESTED_SCROLL_PRIMITIVE);
        }
        if features.facts.ui.lists.swipe_actions {
            out.push_str(NEXA_FAST_LIST_SWIPE_PRIMITIVE);
        }
        if features.uses_adaptive_tabs {
            out.push_str(NEXA_APP_BOTTOM_BAR_PRIMITIVE);
        }
        if features.facts.ui.style.dynamic_color {
            out.push_str(NEXA_DYNAMIC_COLOR_HELPER);
        }
        if features.uses_result || features.facts.capabilities.uses_json_api {
            out.push_str(
                r#"public sealed class NexaResult<out T, out E> {
            public data class Success<out T>(val value: T) : NexaResult<T, Nothing>()
            public data class Failure<out E>(val error: E) : NexaResult<Nothing, E>()

            public val isSuccess: Boolean get() = this is Success
            public val isFailure: Boolean get() = this is Failure

            public fun getOrNull(): T? = when (this) {
                is Success -> value
                is Failure -> null
            }

            public fun errorOrNull(): E? = when (this) {
                is Success -> null
                is Failure -> error
            }

            public fun getOrThrow(): T = when (this) {
                is Success -> value
                is Failure -> throw RuntimeException("Unhandled NexaResult error: $error")
            }
        }

        "#,
            );
        }
        for declaration in &module.enums {
            out.push_str(&format!(
                "enum class {} {{ {} }}\n\n",
                nexa_codegen::names::enum_name(&declaration.name),
                declaration.cases.join(", ")
            ));
        }
        structs::render(module, out);
        if features.facts.capabilities.uses_regex_api {
            out.push_str(NEXA_REGEX_HELPERS);
        }
        // Value codecs live in the types file: app structs and enums are
        // file-private, and a codec for one has to construct and read it.
        value::render(&value_codecs, out);
        if has_collection_state(module) {
            // A collection state is a snapshot-state collection, so assigning a
            // new value replaces the contents. One overload per collection kind
            // lets the call site pick by static type.
            out.push_str(COLLECTION_REPLACE_HELPERS);
        }
    });
    if features.uses_asset || features.uses_placeholder || features.uses_remote_image {
        units.write("image-primitives", |out| {
            components::images::render_primitives(
                features.uses_asset,
                features.uses_remote_image,
                out,
            );
        });
    }

    if features.facts.capabilities.uses_json_api {
        units.write("json", |out| {
            api::json::render(&json_types, &module.enums, out);
        });
    }

    units.write("app", |out| {
            let mut opt_in_annotations = Vec::with_capacity(4);
            opt_in_annotations.push("androidx.compose.material3.ExperimentalMaterial3Api::class");
            if features.uses_context_menu {
                opt_in_annotations
                    .push("androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class");
            }
            if features.app_uses_keyboard_interactive {
                opt_in_annotations.push("androidx.compose.foundation.layout.ExperimentalLayoutApi::class");
            }
            if features.uses_sticky_header
                || features.uses_long_press
                || features.uses_double_tap
                || features.uses_page_snap
            {
                opt_in_annotations.push("androidx.compose.foundation.ExperimentalFoundationApi::class");
            }
            render_opt_in_annotation(out, &opt_in_annotations);
            out.push_str(&format!(
                "@Composable\nfun {}() {{\n",
                nexa_codegen::names::screen_name(&module.app_name)
            ));
            if features.app_uses_task_scope() {
                out.push_str("    val nexaTaskScope = rememberCoroutineScope()\n");
            }
            if features.uses_adaptive_color {
                out.push_str("    val nexaIsDarkTheme = isSystemInDarkTheme()\n");
            }
            components::status_bar::render(module.status_bar, features.uses_status_bar, 1, out);
            if features.app_uses_link {
                out.push_str("    val nexaLinkContext = LocalContext.current\n");
            }
            if features.uses_permission_request {
                out.push_str(
                    "    val nexaPermissionLauncher = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { result ->\n        NexaRuntime.dispatchPermissionResult(result)\n    }\n    NexaRuntime.bindPermissionLauncher(nexaPermissionLauncher)\n",
                );
            }
            if features.uses_network_api
                || features.uses_network_connectivity
                || features.uses_path_api
                || features.uses_permissions
                || features.facts.capabilities.uses_keyboard_api
                || features.facts.capabilities.uses_clipboard_api
                || features.facts.capabilities.uses_haptics_api
                || features.facts.capabilities.uses_secure_storage_api
                || features.facts.capabilities.uses_screen_orientation_api
                || features.facts.capabilities.uses_storage_api
                || features.facts.capabilities.uses_localized_strings
                || !module.plugins.is_empty()
                || !module.background_tasks.is_empty()
                || !module.widgets.is_empty()
            {
                out.push_str("    NexaRuntime.bind(LocalContext.current)\n");
            }
            if features.app_uses_haptic {
                out.push_str("    val nexaHapticView = LocalView.current\n");
            }
            for state in &module.states {
                let name = nexa_codegen::names::state_name(&state.name);
                if state.mutable {
                    if state::is_mutable_collection(state) {
                        out.push_str(&format!(
                            "    val {name} = remember {{ {} }}\n",
                            state::kotlin_state_initializer(state)
                        ));
                    } else {
                        out.push_str(&format!(
                            "    var {name} by remember {{ {} }}\n",
                            state::kotlin_state_initializer(state)
                        ));
                    }
                } else if state.is_native_class_instance_binding()
                    || matches!(state.ty, nexa_ir::Type::Signal(_))
                {
                    out.push_str(&format!(
                        "    val {name}: {} = remember {{ {} }}\n",
                        kotlin_type(&state.ty),
                        expressions::expression(&state.initial)
                    ));
                } else {
                    out.push_str(&format!(
                        "    val {name}: {} = {}\n",
                        kotlin_type(&state.ty),
                        expressions::expression(&state.initial)
                    ));
                }
            }
            let animated_targets = state::app_animated_state_targets(module);
            state::render_animated_state_aliases(
                &module.states,
                &animated_targets,
                1,
                out,
            );
            if !focus_bindings.is_empty() {
                for binding in &focus_bindings {
                    out.push_str(&format!(
                        "    val {} = remember {{ FocusRequester() }}\n",
                        input::focus_requester_name(binding)
                    ));
                }
                for binding in &focus_bindings {
                    let state_name = nexa_codegen::names::state_name(binding);
                    let requester_name = input::focus_requester_name(binding);
                    out.push_str(&format!(
                        "    LaunchedEffect({state_name}) {{\n        if ({state_name}) {requester_name}.requestFocus() else {requester_name}.freeFocus()\n    }}\n"
                    ));
                }
            }
            if !module.states.is_empty() {
                out.push('\n');
            }
            if features.uses_shared_elements {
                out.push_str("    NexaSharedTransitionContent {\n");
            }
            let body_base_depth = if features.uses_shared_elements { 2 } else { 1 };
            let wrap_app_status_bar_background = if module.screens.is_empty() {
                components::status_bar::render_background_start(
                    module.status_bar,
                    body_base_depth,
                    out,
                )
            } else {
                false
            };
            let body_depth = components::direction::start(module.direction, body_base_depth, out);
            components::lifecycle::render_on_appear(module.on_appear.as_deref(), body_depth, out);
            components::lifecycle::render_on_disappear(
                module.on_disappear.as_deref(),
                body_depth,
                out,
            );
            components::lifecycle::render_task_cancellation_on_dispose(
                &module.states,
                body_depth,
                out,
            );
            components::lifecycle::render_app(module, body_depth, out);
            if module.body.len() == 1 {
                component_renderer::render_node(&module.body[0], module, features, body_depth, out);
            } else {
                layout::render_layout(
                    LayoutKind::Column,
                    0.0,
                    &ViewStyle::default(),
                    &module.body,
                    &components::RenderScope { module, features },
                    body_depth,
                    out,
                );
            }
            components::direction::end(module.direction, body_base_depth, out);
            if wrap_app_status_bar_background {
                components::status_bar::render_background_end(body_base_depth, out);
            }
            if features.uses_shared_elements {
                out.push_str("\n    }");
            }
            out.push_str("\n}\n");
    });

    if features.uses_shared_elements {
        units.write("shared-elements", components::shared_elements::render);
    }

    units.write("components", |out| {
        custom_components::render(module, features, out);
        out.push_str(COLLECTION_ITERATION_HELPERS);
        if features.uses_task_launch {
            out.push_str(
                "\ninternal val nexaTaskExceptionHandler = CoroutineExceptionHandler { _, error ->\n    android.util.Log.e(\"Nexa\", \"Unhandled task failure\", error)\n}\n",
            );
        }
    });
    if features.uses_network_api
        || features.uses_network_connectivity
        || features.uses_path_api
        || features.uses_permissions
        || features.facts.capabilities.uses_keyboard_api
        || features.facts.capabilities.uses_haptics_api
        || features.facts.capabilities.uses_secure_storage_api
        || features.facts.capabilities.uses_storage_api
        || features.facts.capabilities.uses_localized_strings
        || features.facts.capabilities.uses_clipboard_api
        || features.facts.capabilities.uses_screen_orientation_api
        || features.facts.capabilities.uses_app_icon_api
        || !module.plugins.is_empty()
        || !module.background_tasks.is_empty()
    {
        units.write("runtime", |out| {
            runtime::render(
                out,
                features.uses_permission_request,
                features.facts.capabilities.uses_app_icon_api,
            );
        });
    }
    if features.uses_native_library {
        units.write("native-library", |out| {
            network::render(
                out,
                features.uses_network_api,
                features.uses_remote_image,
                features.uses_path_api,
                features.uses_file_api,
                features.uses_file_async,
                features.uses_network_connectivity,
            );
        });
    }
    if features.facts.capabilities.uses_secure_storage_api {
        units.write("secure-storage", |out| {
            api::secure_storage::render(out);
        });
    }
    if features.facts.capabilities.uses_storage_api {
        units.write("storage", api::storage::render);
    }
    if features.facts.capabilities.uses_clipboard_api {
        units.write("clipboard", api::clipboard::render);
    }
    if features.uses_asset || features.uses_placeholder || features.uses_remote_image {
        units.write("assets", |out| {
            assets::render(out);
        });
    }
    if features.uses_permissions {
        units.write("permissions", |out| {
            permissions::render(
                out,
                features.uses_permission_request,
                &features.used_permissions,
                features.dynamic_permission,
                features.expose_permissions_to_dev_runtime,
            );
        });
    }
    if features.facts.capabilities.uses_time {
        units.write("time", |out| {
            api::time::render(out);
        });
    }
    if features.facts.capabilities.uses_number_formatting {
        units.write("number", |out| {
            number::render(out);
        });
    }
    if features.facts.capabilities.uses_crypto_api {
        units.write("crypto", |out| {
            api::crypto::render(out);
        });
    }
    units.write("functions", |out| {
        functions::render(module, out);
    });
    let mut generated = units.finish();
    let references = generated
        .units
        .iter()
        .map(|unit| unit.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let primitives = if include_all_shared_primitives {
        nexa_codegen::source_sections::all(&shared_component_primitives())
    } else {
        nexa_codegen::source_sections::reachable(&shared_component_primitives(), &references)
    };
    generated.prepend_to_unit("NexaGenerated_types.kt", &primitives);
    generated
}

#[cfg(test)]
mod tests {
    use super::{
        NEXA_APP_BOTTOM_BAR_PRIMITIVE, NEXA_NATIVE_LIST_PRIMITIVES,
        NEXA_SHARED_COMPONENT_PRIMITIVES, generate, generate_units_with_project_features,
        shared_component_primitives,
    };
    use nexa_ir::{
        Action, AnimationSpec, Component, Expr, Function, ImageScale, ImageSource, LayoutKind,
        ListAxis, ListCommon, ListPlan, Module, Node, NumericType, Screen, ScreenId, State,
        SystemIcon, TaskExecutor, TextStyle, Type, ViewStyle, ViewTransition, WhenCase,
    };

    #[test]
    fn app_bottom_bar_tint_is_scoped_to_navigation_items() {
        assert!(NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("selectedIconColor = tint"));
        assert!(NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("selectedTextColor = tint"));
        assert!(
            NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("indicatorColor = tint.copy(alpha = 0.12f)")
        );
        assert!(!NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("copy(primary = tint)"));
        assert!(!NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("MaterialTheme("));
    }

    #[test]
    fn keyboard_aware_scroll_does_not_refocus_a_field_during_a_drag() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaKeyboardAwareColumnPrimitive("));
        let keyboard_aware = primitives
            .split("internal fun NexaKeyboardAwareColumnPrimitive(")
            .nth(1)
            .expect("keyboard-aware primitive is emitted");
        let keyboard_snapshot = keyboard_aware
            .find("val keyboardWasVisible = imeVisible.value")
            .expect("capture keyboard visibility before waiting for the gesture");
        let gesture_wait = keyboard_aware
            .find("val down = awaitFirstDown(")
            .expect("observe drag start after capturing keyboard visibility");
        assert!(keyboard_snapshot < gesture_wait);
        assert!(primitives.contains("if (!keyboardWasVisible && (movedX > touchSlop"));
        assert!(primitives.contains(
            "if (interactiveDismiss && imeVisible.value) modifier = modifier.imeNestedScroll()"
        ));
        assert!(primitives.contains("focusManager.clearFocus()"));
    }

    #[test]
    fn refresh_control_scroll_area_fills_the_available_viewport() {
        let refresh = NEXA_SHARED_COMPONENT_PRIMITIVES
            .split("internal fun NexaRefreshControlPrimitive")
            .nth(1)
            .expect("refresh control primitive is generated");
        assert!(refresh.contains("modifier = modifier.fillMaxSize()"));
        assert!(refresh.contains("Modifier.fillMaxSize().verticalScroll("));
    }

    #[test]
    fn horizontal_fast_list_disables_gestures_when_content_fits() {
        let module = Module {
            app_name: "HorizontalListScroll".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: vec![Node::FastList {
                plan: Box::new(ListPlan::Count {
                    count: Expr::Number {
                        raw: "2".to_owned(),
                        ty: NumericType::Int32,
                    },
                    common: ListCommon {
                        axis: ListAxis::Horizontal,
                        native: false,
                        show_dividers: true,
                        reverse_layout: false,
                        page_snap: false,
                        item_extent: Some(80.0),
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: None,
                        children: vec![Node::Text {
                            value: Expr::String("Card".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_move: None,
                        swipe_actions: None,
                        sticky_header: None,
                        refresh: None,
                        spacing: 0.0,
                        scroll_enabled: None,
                    },
                }),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        assert!(
            crate::generator::engine::features::Features::analyze(&module).uses_horizontal_list
        );
        let kotlin = generate(&module);
        assert!(kotlin.contains("val nexaListState0 = rememberLazyListState()"));
        assert!(kotlin.contains(
            "LazyRow(modifier = Modifier.fillMaxWidth().nestedScroll(NexaHorizontalListScrollBoundary), state = nexaListState0, userScrollEnabled = (nexaListState0.canScrollForward || nexaListState0.canScrollBackward))"
        ));
        assert!(kotlin.contains("object NexaHorizontalListScrollBoundary"));
        assert!(
            kotlin.contains("androidx.compose.ui.input.nestedscroll.NestedScrollSource.UserInput")
        );
        assert!(kotlin.contains("import androidx.compose.ui.input.nestedscroll.nestedScroll"));
        assert!(
            !kotlin.contains("CompositionLocalProvider(LocalRippleConfiguration provides null) {")
        );

        let mut disabled_module = module.clone();
        let Node::FastList { plan } = &mut disabled_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_mut() else {
            panic!("expected the count-backed FastList plan");
        };
        common.scroll_enabled = Some(Expr::Bool(false));
        let disabled_kotlin = generate(&disabled_module);
        assert!(disabled_kotlin.contains(
            "userScrollEnabled = (nexaListState0.canScrollForward || nexaListState0.canScrollBackward) && false"
        ));
    }

    #[test]
    fn reverse_fast_list_scroll_position_tracks_the_visual_leading_item() {
        let module = Module {
            app_name: "ReverseListPosition".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: vec![State {
                name: "listPosition".to_owned(),
                ty: Type::Numeric(NumericType::Int32),
                initial: Expr::Number {
                    raw: "0".to_owned(),
                    ty: NumericType::Int32,
                },
                mutable: true,
            }],
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: vec![Node::FastList {
                plan: Box::new(ListPlan::Count {
                    count: Expr::Number {
                        raw: "24".to_owned(),
                        ty: NumericType::Int32,
                    },
                    common: ListCommon {
                        axis: ListAxis::Vertical,
                        native: false,
                        show_dividers: true,
                        reverse_layout: true,
                        page_snap: false,
                        item_extent: Some(48.0),
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: Some("listPosition".to_owned()),
                        children: vec![Node::Text {
                            value: Expr::String("Reverse row".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_move: None,
                        swipe_actions: None,
                        sticky_header: None,
                        refresh: None,
                        spacing: 4.0,
                        scroll_enabled: None,
                    },
                }),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains(
            "snapshotFlow { nexaListState0.layoutInfo.visibleItemsInfo.maxOfOrNull { it.index } ?: nexaListState0.firstVisibleItemIndex }"
        ));
        assert!(kotlin.contains(
            "val target = (nexaListCount0 - 1 - nexa_listPosition).coerceIn(0, (nexaListCount0 - 1).coerceAtLeast(0))"
        ));
        assert!(
            kotlin.contains(
                "nexaListState0.layoutInfo.visibleItemsInfo.maxOfOrNull { it.index } ?: -1"
            )
        );
    }

    #[test]
    fn divider_primitive_passes_thickness_through_to_compose() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains("internal fun NexaDividerPrimitive("));
        assert!(primitives.contains("HorizontalDivider(color = color, thickness = thickness)"));
    }

    #[test]
    fn native_list_rows_keep_the_shared_minimum_height() {
        let minimum_height = nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_MIN_HEIGHT;

        assert_eq!(minimum_height, 52);
        assert_eq!(
            nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_HORIZONTAL_INSET,
            16
        );
        assert_eq!(
            nexa_codegen::design_system::ANDROID_NATIVE_LIST_DIVIDER_START_INSET,
            0
        );
        assert_eq!(
            nexa_codegen::design_system::ANDROID_NATIVE_LIST_DIVIDER_END_INSET,
            0
        );
        assert!(NEXA_NATIVE_LIST_PRIMITIVES.contains(
            ".heightIn(min = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_MIN_HEIGHT__.toFloat()))"
        ));
        assert!(NEXA_NATIVE_LIST_PRIMITIVES.contains(
            "horizontal = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__.toFloat())"
        ));
        assert!(NEXA_NATIVE_LIST_PRIMITIVES.contains(
            "endInset = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_DIVIDER_END_INSET__.toFloat())"
        ));
    }

    #[test]
    fn segmented_control_fills_its_container_and_equalizes_each_choice() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains(
            "val outerHeight = maxOf(androidx.compose.ui.unit.Dp(48f), segmentTrackHeight)"
        ));
        assert!(
            primitives
                .contains("val segmentTrackHeight = height ?: androidx.compose.ui.unit.Dp(32f)")
        );
        assert!(
            primitives.contains("val trackBorderWidth = (borderWidth ?: if (borderColor != null)")
        );
        assert!(
            primitives.contains("val selectionIndicatorInset = androidx.compose.ui.unit.Dp(3f)")
        );
        assert!(primitives.contains(
            "val selectionIndicatorHeight = (segmentTrackHeight - androidx.compose.ui.unit.Dp(6f))\n        .coerceAtLeast(androidx.compose.ui.unit.Dp(0f))"
        ));
        assert!(
            primitives.contains("val trackCornerRadius = cornerRadius ?: segmentTrackHeight / 2f")
        );
        assert!(primitives.contains("RoundedCornerShape(trackCornerRadius)"));
        assert!(primitives.contains("minOf(\n            trackCornerRadius,"));
        assert!(primitives.contains(
            ".height(selectionIndicatorHeight)\n                    .padding(horizontal = selectionIndicatorInset)"
        ));
        assert!(primitives.contains(".fillMaxSize()\n                .selectableGroup()"));
        assert!(
            primitives
                .contains(".selectable(\n                            selected = selected == item,")
        );
        assert!(primitives.contains("interactionSource = interactionSource,"));
        assert!(primitives.contains("indication = null,"));
        assert!(
            primitives
                .contains(".indication(interactionSource, androidx.compose.material3.ripple())")
        );
        assert!(primitives.contains(
            ".fillMaxHeight()\n                        .clip(segmentInteractionShape)\n                        .selectable("
        ));
        assert!(primitives.contains("role = androidx.compose.ui.semantics.Role.RadioButton"));
        assert!(primitives.contains(
            ".background(background ?: androidx.compose.material3.MaterialTheme.colorScheme.surfaceVariant)"
        ));
        assert!(primitives.contains(
            ".background(tint ?: androidx.compose.material3.MaterialTheme.colorScheme.surface)"
        ));
        assert!(primitives.contains(
            "style = androidx.compose.material3.MaterialTheme.typography.labelLarge.copy("
        ));
        assert!(primitives.contains("androidx.compose.animation.core.animateDpAsState("));
        assert!(primitives.contains(
            "val expressiveSpatialMotion = androidx.compose.runtime.remember {\n            androidx.compose.material3.MotionScheme.expressive()\n                .defaultSpatialSpec<androidx.compose.ui.unit.Dp>()\n        }"
        ));
        assert!(primitives.contains("animation\n                ?: expressiveSpatialMotion"));
        assert!(primitives.contains("label = \"segmentedControlSelection\""));
        assert!(primitives.contains(".offset {"));
    }

    #[test]
    fn split_view_uses_shared_layout_without_route_specific_icon_dependencies() {
        let module = Module {
            app_name: "SplitViewApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: vec![Node::NavigationSplitView {
                detail_visible: "showDetail".to_owned(),
                sidebar: Vec::new(),
                detail: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let (sources, project_features) = generate_units_with_project_features(&module);
        let generated = sources
            .into_files(&[], "")
            .into_iter()
            .map(|file| file.contents)
            .collect::<String>();

        assert!(generated.contains("NexaNavigationSplitViewPrimitive("));
        assert!(generated.contains("internal fun NexaNavigationSplitViewPrimitive("));
        assert!(!generated.contains("import androidx.navigation.NavHostController"));
        assert!(!generated.contains("import androidx.compose.material.icons.filled.ChevronRight"));
        assert!(!project_features.uses_navigation);
        assert!(!project_features.uses_system_icons);
    }

    #[test]
    fn split_view_centers_intrinsic_content_horizontally_at_the_top() {
        let primitives = shared_component_primitives();
        let split_view = primitives
            .split("// nexa-codegen: NexaNavigationSplitViewPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaNavigationScreenPrimitive")
            .next()
            .unwrap_or_default();

        assert_eq!(
            split_view
                .matches("contentAlignment = androidx.compose.ui.Alignment.TopCenter")
                .count(),
            3,
            "sidebar, detail, and compact split-view content should be horizontally centered and top-aligned"
        );
    }

    #[test]
    fn shared_button_primitive_owns_platform_defaults_for_aot_and_dev_runtime() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains("internal fun NexaButtonPrimitive("));
        assert!(primitives.contains("MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)"));
        assert!(primitives.contains("color = if (plainButtonStyle) {"));
        assert!(primitives.contains("Modifier.size(androidx.compose.ui.unit.Dp(20.toFloat()))"));
        assert!(primitives.contains("val plainContentColor = when {"));
        assert!(primitives.contains("if (resolvedStyle == \"Borderless\") {\n        androidx.compose.material3.MaterialTheme.colorScheme.primary"));
        assert!(primitives.contains("tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary).copy(alpha = 0.38f)"));
        assert!(primitives.contains(
            "PaddingValues(horizontal = androidx.compose.ui.unit.Dp(8.toFloat()), vertical = androidx.compose.ui.unit.Dp(8.toFloat()))"
        ));
        assert!(primitives.contains("textAlign = textAlign ?: LocalNexaTextAlignment.current"));
        assert!(!primitives.contains("LocalNexaTextFillWidth"));
        assert!(primitives.contains(
            "containerColor = androidx.compose.ui.graphics.Color.Transparent,\n                contentColor = tint ?: androidx.compose.ui.graphics.Color.Unspecified,\n                disabledContainerColor = androidx.compose.ui.graphics.Color.Transparent,"
        ));
        assert!(primitives.contains("internal fun NexaFormPrimitive("));
        assert!(primitives.contains("internal fun NexaFormSectionPrimitive("));
        assert!(primitives.contains("internal fun NexaFormDividerPrimitive()"));
        assert!(primitives.contains("Dp(64.toFloat())"));
        assert!(primitives.contains("startInset = androidx.compose.ui.unit.Dp(16.toFloat())"));
        assert!(primitives.contains("Dp(50.toFloat())"));
        assert!(primitives.contains("Dp(24.toFloat())"));
        assert!(primitives.contains(
            "LocalNexaTextAlignment provides androidx.compose.ui.text.style.TextAlign.Start"
        ));
        assert!(primitives.contains(
            "bottom = androidx.compose.ui.unit.Dp(\n                    if (footer == null) {\n                        20.toFloat()\n                    } else {\n                        8.toFloat()\n                    },\n                ),"
        ));
        assert!(
            primitives.contains("contentAlignment = androidx.compose.ui.Alignment.CenterStart")
        );
        assert!(
            primitives.contains(
                "PaddingValues(horizontal = androidx.compose.ui.unit.Dp(12.toFloat()), vertical = androidx.compose.ui.unit.Dp(4.toFloat()))"
            )
        );
        assert!(
            primitives.contains(
                "PaddingValues(horizontal = androidx.compose.ui.unit.Dp(20.toFloat()), vertical = androidx.compose.ui.unit.Dp(12.toFloat()))"
            )
        );
        assert!(
            !primitives.contains("__NEXA_"),
            "unresolved design token in helper source"
        );
    }

    #[test]
    fn shared_link_primitive_keeps_material_default_press_feedback() {
        let primitives = shared_component_primitives();
        let link = primitives
            .split("internal fun NexaLinkPrimitive(")
            .nth(1)
            .expect("link primitive is generated")
            .split("// nexa-codegen: NexaFormPrimitive")
            .next()
            .expect("link primitive ends before the form primitive");

        assert!(link.contains("Modifier.clickable("));
        assert!(!link.contains("indication = null"));
    }

    #[test]
    fn external_link_launch_does_not_depend_on_package_visibility_queries() {
        let module = Module {
            app_name: "ExternalLink".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Link {
                url: Expr::String("https://example.com".to_owned()),
                children: vec![Node::Text {
                    value: Expr::String("Open website".to_owned()),
                    style: TextStyle::default(),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("import android.content.ActivityNotFoundException"));
        assert!(kotlin.contains("Intent(Intent.ACTION_VIEW, Uri.parse(\"https://example.com\"))"));
        assert!(kotlin.contains("nexaLinkContext.startActivity(nexaLinkIntent)"));
        assert!(kotlin.contains("catch (_: ActivityNotFoundException)"));
        assert!(!kotlin.contains("resolveActivity(nexaLinkContext.packageManager)"));
    }

    #[test]
    fn shared_progress_indicators_use_full_width_and_determinate_material_ring() {
        let primitives = shared_component_primitives();
        let progress_bar = primitives
            .split("internal fun NexaProgressBarPrimitive")
            .nth(1)
            .unwrap_or_default();
        let progress_ring = primitives
            .split("internal fun NexaProgressRingPrimitive")
            .nth(1)
            .unwrap_or_default();

        assert!(progress_bar.contains("Modifier.fillMaxWidth()"));
        assert!(progress_bar.contains(".progressSemantics(progress.coerceIn(0f, 1f))"));
        assert!(progress_bar.contains(
            "trackColor = androidx.compose.material3.MaterialTheme.colorScheme.onSurface.copy(alpha = 0.1f)"
        ));
        assert!(progress_bar.contains("drawStopIndicator = {},"));
        assert!(progress_ring.contains("progress = { progress.coerceIn(0f, 1f) }"));
        assert!(progress_ring.contains("Modifier.progressSemantics(progress.coerceIn(0f, 1f))"));
        assert!(
            progress_ring
                .contains("color = androidx.compose.material3.MaterialTheme.colorScheme.primary")
        );
        assert!(progress_ring.contains("trackColor = androidx.compose.material3.MaterialTheme.colorScheme.primary.copy(alpha = 0.16f)"));
    }

    #[test]
    fn form_section_heading_uses_the_shared_muted_text_role() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains(
            "style = androidx.compose.material3.MaterialTheme.typography.titleSmall,\n                color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,"
        ));
    }

    #[test]
    fn shared_picker_primitive_owns_its_menu_layout_and_text_metrics() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaPickerPrimitive("));
        assert!(primitives.contains("DropdownMenuItem("));
        assert!(
            primitives
                .contains("itemIcons: List<androidx.compose.ui.graphics.vector.ImageVector?>")
        );
        assert!(primitives.contains("leadingIcon = itemIcons.getOrNull(index)?.let"));
        assert!(primitives.contains("imageVector = optionIcon"));
        assert!(primitives.contains("MaterialExpressiveTheme("));
        assert!(primitives.contains(
            "shape = androidx.compose.material3.MenuDefaults.standaloneGroupShape,\n                    containerColor = androidx.compose.material3.MenuDefaults.groupStandardContainerColor,\n                    tonalElevation = 0.dp,\n                    shadowElevation = 8.dp,"
        ));
        assert!(primitives.contains("shadowElevation = 8.dp,"));
        assert!(primitives.contains("MenuDefaults.itemShape("));
        assert!(primitives.contains("androidx.compose.foundation.Canvas("));
        assert!(primitives.contains("androidx.compose.ui.graphics.Path().apply"));
        assert!(!primitives.contains("Icons.Filled.UnfoldMore"));
        assert!(primitives.contains(
            "val resolvedTint = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary"
        ));
        assert!(primitives.contains(
            "val rowWidthModifier = if (fillMaxWidth && label != null) {\n        androidx.compose.ui.Modifier.fillMaxWidth()\n    } else {\n        androidx.compose.ui.Modifier.wrapContentWidth(androidx.compose.ui.Alignment.CenterHorizontally)\n    }"
        ));
        assert!(primitives.contains(
            "text = label,\n                modifier = androidx.compose.ui.Modifier.weight(1f),\n                color = resolvedTint,"
        ));
        assert!(primitives.contains(".clickable { expanded.value = true }"));
        assert!(
            primitives
                .contains("indication = null,\n            onClick = { expanded.value = true },")
        );
        assert!(primitives.contains("fontSize = 17.sp"));
        assert!(primitives.contains("textAlign = androidx.compose.ui.text.style.TextAlign.Start,"));
    }

    #[test]
    fn shared_text_primitive_owns_default_font_metrics() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("fontSize: androidx.compose.ui.unit.TextUnit = 17.sp"));
        assert!(
            primitives.contains("lineHeight: androidx.compose.ui.unit.TextUnit = fontSize * 1.2f")
        );
        assert!(primitives.contains("letterSpacing: androidx.compose.ui.unit.TextUnit = 0.sp"));
        assert!(
            primitives.contains("overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis")
        );
    }

    #[test]
    fn shared_search_input_uses_a_plain_icon_and_field_row() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaTextInputPrimitive("));
        assert!(primitives.contains("androidx.compose.foundation.text.BasicTextField("));
        assert!(primitives.contains(
            "androidx.compose.foundation.layout.Arrangement.spacedBy(androidx.compose.ui.unit.Dp(8f))"
        ));
        assert!(
            primitives
                .contains("androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(20f))")
        );
        assert!(primitives.contains("if (value.isEmpty() && placeholder.isNotEmpty())"));
        assert!(!primitives.contains("leadingIcon = if (searchable"));
    }

    #[test]
    fn shared_switch_primitive_makes_the_entire_row_toggleable() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("value = checked,"));
        assert!(primitives.contains("role = androidx.compose.ui.semantics.Role.Switch,"));
        assert!(primitives.contains("onValueChange = onCheckedChange,"));
        assert!(primitives.contains("onCheckedChange = null,"));
        assert!(primitives.contains("textAlign = androidx.compose.ui.text.style.TextAlign.Start,"));
    }

    #[test]
    fn shared_page_indicator_owns_dot_geometry_color_and_accessibility() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaPageIndicatorPrimitive("));
        assert!(primitives.contains("(if (selected) 8 else 6).toFloat()"));
        assert!(primitives.contains("Color(0xFF8E8E93).copy(alpha = 0.45.toFloat())"));
        assert!(primitives.contains(".semantics { contentDescription = \"Page ${index + 1}\" }"));
        assert!(primitives.contains("onClickLabel = \"Page ${index + 1}\""));
        assert!(primitives.contains("internal fun NexaPageIndicatorRow("));
        assert!(primitives.contains("Dp(8.toFloat()),"));
        assert!(primitives.contains("padding(bottom = androidx.compose.ui.unit.Dp(24.toFloat()))"));
    }

    #[test]
    fn shared_navigation_title_uses_native_material_app_bars() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaNavigationChrome("));
        assert!(!primitives.contains("internal fun NexaTitleBar("));
        assert!(primitives.contains("key(identity)"));
        assert!(primitives.contains("androidx.compose.material3.Scaffold("));
        assert!(primitives.contains("contentColor = colorScheme.onSurface,"));
        let navigation = primitives
            .split("// nexa-codegen: NexaNavigationScreenPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaNavigationChrome")
            .next()
            .unwrap_or_default();
        assert!(navigation.contains("containerColor = colorScheme.background,"));
        assert!(!navigation.contains("groupedSurface"));
        assert!(primitives.contains("TopAppBarDefaults.exitUntilCollapsedScrollBehavior()"));
        assert!(primitives.contains("TopAppBarDefaults.pinnedScrollBehavior()"));
        assert!(primitives.contains("LargeFlexibleTopAppBar("));
        assert!(primitives.contains("CenterAlignedTopAppBar("));
        assert!(primitives.contains("TopAppBar("));
        assert!(
            primitives.contains("modifier.nestedScroll(scrollBehavior.nestedScrollConnection)")
        );
        assert!(primitives.contains("scrolledContainerColor = containerColor"));
        assert!(primitives.contains("androidx.compose.material3.Text("));
        assert!(primitives.contains("navigationIcon()"));
        assert!(primitives.contains("actions()"));
        assert!(
            primitives
                .contains("LocalNexaFormContentTopInset provides androidx.compose.ui.unit.Dp(0f)")
        );
        assert!(primitives.contains("LocalNexaNavigationContent provides true"));
        assert!(primitives.contains("LocalNexaNavigationTitleSlot provides navigationTitleSlot"));
        assert!(primitives.contains("class NexaNavigationTitleSlot"));
        assert!(primitives.contains("navigationTitleSlot.update("));
        assert!(primitives.contains("navigationTitleSlot.clear(identity)"));
        assert!(primitives.contains("content(this@Column)"));
        assert!(!primitives.contains("private class NexaNavigationChromeState("));
        assert!(!primitives.contains("val headerHeight ="));
        assert!(!primitives.contains("NexaTitleHeaderPrimitive"));
    }

    #[test]
    fn shared_presentation_primitives_own_sheets_and_dialog_layout() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains("internal fun NexaBottomSheetPrimitive("));
        assert!(primitives.contains("ModalBottomSheet("));
        assert!(primitives.contains("dragHandle = if (showDragHandle)"));
        assert!(primitives.contains("                null\n            },"));
        assert!(primitives.contains(
            "containerColor = androidx.compose.material3.MaterialTheme.colorScheme.background"
        ));
        assert!(primitives.contains("contentAlignment = androidx.compose.ui.Alignment.Center"));
        let bottom_sheet = primitives
            .split("// nexa-codegen: NexaBottomSheetPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaAlertDialogPrimitive")
            .next()
            .unwrap_or_default();
        let full_screen_dialog = bottom_sheet
            .split("androidx.compose.ui.window.Dialog(")
            .nth(1)
            .unwrap_or_default();
        assert!(
            full_screen_dialog.contains("contentAlignment = androidx.compose.ui.Alignment.Center")
        );
        let alert_dialog = primitives
            .split("internal fun NexaAlertDialogPrimitive(")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaConfirmationDialogPrimitive")
            .next()
            .unwrap_or_default();
        assert!(
            alert_dialog.contains("dismissButton: @androidx.compose.runtime.Composable () -> Unit")
        );
        assert!(alert_dialog.contains("dismissButton = {"));
        assert!(alert_dialog.contains("content = dismissButton"));
        assert!(primitives.contains("dragHandle = if (showDragHandle)"));
        assert!(bottom_sheet.contains("val enabledSheetStates = if (largeOnly)"));
        assert!(bottom_sheet.contains("androidx.compose.material3.SheetValue.PartiallyExpanded"));
        assert!(
            bottom_sheet.contains("initialValue = androidx.compose.material3.SheetValue.Hidden")
        );
        assert!(bottom_sheet.contains("enabledValues = enabledSheetStates"));
        assert!(bottom_sheet.contains("sheetState.partialExpand()"));
        assert!(bottom_sheet.contains("sheetState.expand()"));
        assert!(!bottom_sheet.contains("initial large detent"));
        let sheet_column_modifier = bottom_sheet
            .split("androidx.compose.foundation.layout.Column(\n                modifier = androidx.compose.ui.Modifier\n                    .fillMaxWidth()")
            .nth(1)
            .unwrap_or_default()
            .split(") {")
            .next()
            .unwrap_or_default();
        assert!(sheet_column_modifier.contains(".fillMaxHeight(),"));
        assert!(!sheet_column_modifier.contains(".then("));
        assert!(
            bottom_sheet
                .contains("screenHeight / 2f - androidx.compose.ui.unit.Dp(12f) - toolbarHeight")
        );
        assert!(
            bottom_sheet
                .contains("val bodyHeight = androidx.compose.animation.core.animateDpAsState(")
        );
        assert!(bottom_sheet.contains(".height(bodyHeight),"));
        assert!(bottom_sheet.contains("WindowInsets.navigationBars.getBottom(density)"));
        assert!(
            bottom_sheet.contains(
                "sheetState.targetValue == androidx.compose.material3.SheetValue.Expanded"
            )
        );
        assert!(bottom_sheet.contains("contentAlignment = androidx.compose.ui.Alignment.Center"));
        assert!(!bottom_sheet.contains(".weight(1f),\n                        contentAlignment = androidx.compose.ui.Alignment.Center"));
        assert!(primitives.contains("height(androidx.compose.ui.unit.Dp(12f))"));
        assert!(primitives.contains("NexaNavigationChromeTextStyle()"));
        assert!(primitives.contains("LocalNexaBottomSheetToolbarButtons provides true"));
        let toolbar_button_slot = concat!(
            ".height(androidx.compose.ui.unit.Dp(48f))\n",
            "                                        .widthIn(min = androidx.compose.ui.unit.Dp(48f))",
        );
        assert_eq!(primitives.matches(toolbar_button_slot).count(), 2);
        assert!(primitives.contains("style: String?,"));
        assert!(primitives.contains("val useNativeActionStyle = style == null"));
        assert!(primitives.contains("LocalNexaDialogActionButtons.current"));
        assert!(primitives.contains("LocalNexaBottomSheetToolbarButtons.current"));
        assert!(primitives.contains("LocalNexaDialogActionButtons provides true"));
        assert!(primitives.contains("LocalNexaPickerPressFeedback provides false"));
        assert!(!primitives.contains("LocalIndication provides"));
        assert!(!primitives.contains("LocalRippleConfiguration provides null"));
        assert!(primitives.contains("internal val LocalNexaPickerPressFeedback"));
        assert!(primitives.contains("MaterialExpressiveTheme("));
        assert!(primitives.contains("MenuDefaults.itemShape("));
        assert!(primitives.contains("internal fun NexaAlertDialogPrimitive("));
        assert!(primitives.contains("internal fun NexaConfirmationDialogPrimitive("));
        assert!(
            primitives
                .contains("horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally")
        );
        assert!(primitives.contains("internal fun NexaDialogActionDividerPrimitive()"));
    }

    #[test]
    fn date_picker_uses_material_controls_in_a_responsive_platform_dialog() {
        let primitives = shared_component_primitives();
        let date_picker = primitives
            .split("internal fun NexaDatePickerPrimitive(")
            .nth(1)
            .unwrap_or_default();

        assert!(!primitives.contains("NexaDatePickerDialogCalendarContent"));
        assert!(!date_picker.contains("LocalDensity provides"));
        assert!(!date_picker.contains("calendarScale"));
        assert!(date_picker.contains("androidx.compose.material3.DatePicker("));
        assert!(!date_picker.contains("androidx.compose.material3.DateRangePicker("));
        assert!(date_picker.contains("NexaDatePickerSingleMonthRangeCalendar("));
        assert!(date_picker.contains("weekCount = (leadingDays + dayCount + 6) / 7"));
        assert!(date_picker.contains("nexaPickerAddMonths(displayedMonthMillis, -1, locale)"));
        assert!(date_picker.contains("nexaPickerAddMonths(displayedMonthMillis, 1, locale)"));
        assert!(
            date_picker.contains(
                "val isCompact = style == \"Compact\" || (style == \"Automatic\" && !isRange) || style == \"Input\""
            )
        );
        assert!(date_picker.contains("horizontalAlignment = androidx.compose.ui.Alignment.Start"));
        assert!(date_picker.contains(
            "val inlineCalendarMinimumWidth = if (!isCompact && pickerType != \"Time\") 304.dp else 0.dp"
        ));
        assert!(date_picker.contains("Modifier.requiredWidthIn(min = inlineCalendarMinimumWidth)"));
        assert!(date_picker.contains("if (isRange) {"));
        assert!(date_picker.contains("private fun NexaDatePickerCompactTrigger("));
        assert!(date_picker.contains("outlinedButtonColors(contentColor = contentColor)"));
        assert!(
            date_picker
                .contains("androidx.compose.material3.Surface(\n            onClick = onClick,")
        );
        assert!(
            date_picker.contains("Color(0xFF767680).copy(alpha = 0.12f).compositeOver(surface)")
        );
        assert!(date_picker.contains(".height(34.dp)"));
        assert!(date_picker.contains("padding(horizontal = 12.dp)"));
        assert!(date_picker.contains("MaterialTheme.typography.bodyLarge"));
        assert!(date_picker.contains("PaddingValues(horizontal = 16.dp, vertical = 6.dp)"));
        assert!(
            date_picker
                .contains("nexaFormatPickerValue(currentTimestamp, \"Date\", locale, zone, null)")
        );
        assert!(date_picker.contains(
            "nexaFormatPickerValue(currentTimestamp, \"Time\", locale, zone, effectiveIs24Hour)"
        ));
        assert!(date_picker.contains("onClick = { openPicker(false) }"));
        assert!(date_picker.contains("onClick = { openPicker(true) }"));
        assert!(
            date_picker
                .contains("dialogShowsTimeOnly.value = pickerType == \"Time\" || showTimeOnly")
        );
        assert!(date_picker.contains("Modifier.weight(1f, fill = false)"));
        assert!(date_picker.contains("R.string.m3c_time_picker_hour)"));
        assert!(!date_picker.contains("m3c_time_picker_hour_24h_suffix"));
        assert!(!date_picker.contains("%1$d hours"));
        assert!(
            date_picker.contains("overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis")
        );
        assert!(date_picker.contains("typography.labelLarge"));
        assert!(date_picker.contains("shape = androidx.compose.foundation.shape.CircleShape"));
        assert!(date_picker.contains("selectedDayContentColor = selectedContent"));
        assert!(date_picker.contains("timeSelectorSelectedContentColor = selectedContent"));
        assert_eq!(date_picker.matches("focusRequester = null").count(), 4);
        assert!(date_picker.contains("androidx.compose.ui.window.Dialog("));
        assert!(date_picker.contains("setWindowAnimations(0)"));
        assert!(!date_picker.contains("androidx.compose.material3.DatePickerDialog("));
        assert!(date_picker.contains("usePlatformDefaultWidth = false"));
        assert!(date_picker.contains("decorFitsSystemWindows = false"));
        assert!(date_picker.contains(".padding(horizontal = 8.dp)"));
        assert!(date_picker.contains("widthIn(max = 640.dp)"));
        assert!(date_picker.contains("requiredWidthIn(min = inlineCalendarMinimumWidth)"));
        assert!(date_picker.contains("NexaDatePickerCalendarContainer("));
        assert!(date_picker.contains("BoxWithConstraints(modifier.fillMaxWidth())"));
        assert!(date_picker.contains("LocalMinimumInteractiveComponentSize provides dayCellSize"));
        assert!(
            date_picker.contains("(maxWidth - 24.dp).coerceAtLeast(0.dp) / 7).coerceAtMost(48.dp)")
        );
        assert!(date_picker.contains("maxLines = 1"));
        assert!(date_picker.contains("softWrap = false"));
        assert!(primitives.contains("getDateInstance(dateStyle, locale)"));
        assert!(date_picker.contains("showTitle = true"));
        assert!(date_picker.contains("showTitle = false"));
        assert!(date_picker.contains("DatePickerDefaults.shape"));
        assert!(!date_picker.contains("requiredWidth("));
        assert!(date_picker.contains("heightIn(max = dialogMaxHeight)"));
        assert!(date_picker.contains("val dialogMaxHeight = screenHeight * 0.88f"));
        assert!(date_picker.contains("val calendarMaxHeight = (dialogMaxHeight - 72.dp)"));
        assert!(!date_picker.contains("rangeCalendarMaxHeight"));
        assert!(date_picker.contains("heightIn(max = 440.dp)"));
        assert!(date_picker.contains("heightIn(min = 56.dp)"));
        assert!(date_picker.contains("enabled = enabledValue &&"));
        assert!(
            date_picker
                .contains("rangeState.selectedStartDateMillis != rangeState.selectedEndDateMillis")
        );
        assert!(date_picker.contains("if (!enabledValue)"));
        assert!(date_picker.contains(".clearAndSetSemantics"));
        assert!(!date_picker.contains("ModalBottomSheet("));
        assert!(date_picker.contains("showModeToggle = true"));
        assert!(date_picker.contains("Modifier.fillMaxWidth()"));
        assert!(date_picker.contains("androidx.compose.ui.Modifier.fillMaxWidth()"));
        assert!(date_picker.contains("modifier = androidx.compose.ui.Modifier.fillMaxWidth()"));
        assert!(date_picker.contains("heightIn(max = 440.dp)"));
        assert!(!date_picker.contains("takeIf { it > 0L }"));
        assert!(
            date_picker
                .contains("verticalScroll(androidx.compose.foundation.rememberScrollState())")
        );
        assert!(date_picker.contains("androidx.compose.material3.TimePicker("));
        assert!(primitives.contains("private fun NexaDatePickerTimeInput("));
        assert!(primitives.contains("androidx.compose.material3.OutlinedTextField("));
        assert!(primitives.contains("R.string.m3c_time_picker_hour)"));
        assert!(!primitives.contains("m3c_time_picker_hour_24h_suffix"));
        assert!(primitives.contains("onFocusChanged { hourFocused = it.isFocused }"));
        assert!(primitives.contains("onFocusChanged { minuteFocused = it.isFocused }"));
        assert!(primitives.contains("m3c_time_picker_hour_text_field"));
        assert!(!primitives.contains("androidx.compose.material3.TimeInput("));
        assert!(!primitives.contains("requestFocus()"));
        assert!(primitives.contains("DateFormat.getBestDateTimePattern(locale, skeleton)"));
        assert!(!primitives.contains("\"j:mm\""));
        assert!(date_picker.contains("stringResource(android.R.string.ok)"));
        assert!(date_picker.contains("stringResource(android.R.string.cancel)"));
        assert!(!date_picker.contains("androidx.compose.ui.window.Popup("));
        assert!(date_picker.contains("DisplayMode.Input"));
        assert!(date_picker.contains("remember(locale, initialDisplayMode, selectableDates)"));
        assert!(date_picker.contains("remember(locale, selectableDates)"));
        assert!(date_picker.contains("nexaConstrainPickerTimestamp"));
        assert!(
            date_picker.contains("dayInSelectionRangeContainerColor = primary.copy(alpha = 0.16f)")
        );
    }

    #[test]
    fn wrapped_text_inherits_column_and_stack_cross_axis_alignment() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains("internal val LocalNexaTextAlignment ="));
        assert!(primitives.contains("textAlign = textAlign ?: LocalNexaTextAlignment.current"));
        assert!(primitives.contains(
            "androidx.compose.ui.Alignment.CenterHorizontally -> androidx.compose.ui.text.style.TextAlign.Center"
        ));
        assert!(primitives.contains(
            "androidx.compose.ui.Alignment.TopStart -> androidx.compose.ui.text.style.TextAlign.Start"
        ));
        assert!(primitives.contains(
            "androidx.compose.ui.Alignment.BottomEnd -> androidx.compose.ui.text.style.TextAlign.End"
        ));
        assert!(primitives.contains("LocalNexaTextAlignment provides null"));
    }

    #[test]
    fn pressable_context_menu_emits_material_dropdown_actions() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ContextMenuApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Row".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: vec![Node::Button {
                    label: Expr::String("Edit".to_owned()),
                    icon: Some(SystemIcon::Shared("edit".to_owned())),
                    loading: None,
                    disabled: None,
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    actions: Vec::new(),
                }],
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("import androidx.compose.material3.DropdownMenu"));
        assert!(kotlin.contains("import androidx.compose.material3.MenuDefaults"));
        assert!(kotlin.contains("nexaContextMenuExpanded.value = true"));
        assert!(kotlin.contains("DropdownMenuItem("));
        assert!(kotlin.contains("shape = MenuDefaults.standaloneGroupShape,"));
        assert!(kotlin.contains("containerColor = MenuDefaults.groupStandardContainerColor,"));
        assert!(kotlin.contains("tonalElevation = 0.dp,"));
        assert!(kotlin.contains("shadowElevation = 8.dp,"));
        let menu_popup = kotlin
            .find("expanded = nexaContextMenuExpanded.value,")
            .expect("context menu popup is generated");
        let expressive_theme = kotlin[..menu_popup]
            .rfind("MaterialExpressiveTheme(")
            .expect("expressive theme wraps the context-menu popup");
        assert!(expressive_theme < menu_popup);
        assert!(kotlin.contains("shape = MenuDefaults.itemShape(index = 0, count = 1).shape"));
        assert!(kotlin.contains("MaterialExpressiveTheme(colorScheme = androidx.compose.material3.MaterialTheme.colorScheme)"));
        assert!(kotlin.contains("Icons.Filled.Edit"));
    }

    #[test]
    fn appearance_wraps_content_in_native_material_color_scheme() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "AppearanceApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Appearance {
                mode: Expr::String("dark".to_owned()),
                children: vec![Node::Text {
                    value: Expr::String("Hello".to_owned()),
                    style: TextStyle::default(),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let kotlin = generate(&module);
        assert!(kotlin.contains("import androidx.compose.material3.darkColorScheme"));
        assert!(kotlin.contains("import androidx.compose.runtime.SideEffect"));
        assert!(kotlin.contains("import androidx.compose.ui.platform.LocalView"));
        assert!(kotlin.contains("import androidx.core.view.WindowCompat"));
        assert!(kotlin.contains("NexaAppearancePrimitive(mode = \"dark\") {"));
        assert!(kotlin.contains("internal fun NexaAppearancePrimitive("));
        assert!(kotlin.contains("androidx.compose.material3.MaterialTheme("));
        let shared_primitives = shared_component_primitives();
        let appearance_primitive = shared_primitives
            .split("internal fun NexaAppearancePrimitive(")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaFastListDividerPrimitive")
            .next()
            .unwrap_or_default();
        assert!(!appearance_primitive.contains("fillMaxSize()"));
        assert!(appearance_primitive.contains("SideEffect"));
        assert!(
            appearance_primitive.contains("isAppearanceLightStatusBars = useLightStatusBarIcons")
        );
        assert!(appearance_primitive.contains("\"dark\" -> false"));
        assert!(appearance_primitive.contains("\"light\" -> true"));
        assert!(appearance_primitive.contains("else -> !systemDark"));
        assert!(appearance_primitive.contains("WindowCompat.getInsetsController(window, view)"));
        assert!(appearance_primitive.contains("DisposableEffect(statusBarWindow, view)"));
        assert!(appearance_primitive.contains("initialLightStatusBar"));
        assert!(appearance_primitive.contains("onDispose"));
        assert!(!appearance_primitive.contains("window.decorView"));
        assert!(appearance_primitive.contains("LocalContentColor provides scheme.onBackground"));
        assert!(!appearance_primitive.contains("Surface("));
        assert!(kotlin.contains("NexaTextPrimitive(text = \"Hello\")"));
        assert!(!kotlin.contains("nexaColorFromHex("));
    }

    #[test]
    fn content_unavailable_emits_compose_material_empty_state() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "EmptyStateApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![
                Node::Layout {
                    kind: nexa_ir::LayoutKind::Column,
                    spacing: 12.0,
                    style: ViewStyle::default(),
                    children: vec![
                        Node::ContentUnavailable {
                            title: Expr::String("Inbox is empty".to_owned()),
                            icon: SystemIcon::shared("inbox").expect("shared inbox icon"),
                            description: Some(Expr::String(
                                "Tasks you add will appear here.".to_owned(),
                            )),
                            title_color: None,
                            description_color: None,
                            icon_tint: None,
                            background: None,
                        },
                        Node::ContentUnavailable {
                            title: Expr::String("No search results".to_owned()),
                            icon: SystemIcon::shared("search").expect("shared search icon"),
                            description: Some(Expr::String("Try a different keyword.".to_owned())),
                            title_color: None,
                            description_color: None,
                            icon_tint: None,
                            background: None,
                        },
                        Node::ContentUnavailable {
                            title: Expr::String("No reminders".to_owned()),
                            icon: SystemIcon::shared("calendar").expect("shared calendar icon"),
                            description: None,
                            title_color: None,
                            description_color: None,
                            icon_tint: None,
                            background: None,
                        },
                    ],
                },
                Node::ProgressBar {
                    progress: Expr::Number {
                        raw: "0.45".to_owned(),
                        ty: nexa_ir::NumericType::Float64,
                    },
                },
            ],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("internal fun NexaContentUnavailablePrimitive("));
        assert!(kotlin.contains("NexaContentUnavailablePrimitive(title = \"Inbox is empty\""));
        assert!(kotlin.contains("description = \"Tasks you add will appear here.\""));
        assert!(kotlin.contains("description = null"));
        assert!(kotlin.contains("description: String?"));
        assert!(kotlin.contains("iconDescription = \"Inbox\""));
        assert!(kotlin.contains("iconDescription = \"Search\""));
        assert!(kotlin.contains("contentDescription = iconDescription"));
        let empty_state_primitive = kotlin
            .split("internal fun NexaContentUnavailablePrimitive(")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaColumnPrimitive")
            .next()
            .unwrap_or_default();
        assert!(empty_state_primitive.contains("if (!description.isNullOrEmpty())"));
        assert_eq!(
            kotlin
                .matches("Box(modifier = Modifier.weight(1f).fillMaxWidth())")
                .count(),
            3,
            "each ContentUnavailable child should receive a share of its parent Column"
        );
        assert!(kotlin.contains(
            "modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(58f))"
        ));
        assert!(kotlin.contains("Icons.Filled.Inbox"));
        assert!(kotlin.contains(".fillMaxWidth()\n            .progressSemantics"));
    }

    #[test]
    fn page_snap_and_bottom_sheet_share_compose_opt_in_annotation() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "PageSnap".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![
                State {
                    name: "currentPage".to_owned(),
                    ty: Type::Numeric(NumericType::Int32),
                    initial: Expr::Number {
                        raw: "0".to_owned(),
                        ty: NumericType::Int32,
                    },
                    mutable: true,
                },
                State {
                    name: "commentsPresented".to_owned(),
                    ty: Type::Bool,
                    initial: Expr::Bool(false),
                    mutable: true,
                },
            ],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![
                Node::FastList {
                    plan: Box::new(ListPlan::Count {
                        count: Expr::Number {
                            raw: "3".to_owned(),
                            ty: NumericType::Int32,
                        },
                        common: ListCommon {
                            axis: ListAxis::Vertical,
                            native: false,
                            show_dividers: true,
                            reverse_layout: false,
                            page_snap: true,
                            item_extent: None,
                            index: "index".to_owned(),
                            key: None,
                            scroll_position: Some("currentPage".to_owned()),
                            children: vec![Node::Text {
                                value: Expr::String("Feed page".to_owned()),
                                style: TextStyle::default(),
                            }],
                            on_end_reached: None,
                            on_scroll: None,
                            on_move: None,
                            swipe_actions: None,
                            sticky_header: None,
                            refresh: None,
                            spacing: 0.0,
                            scroll_enabled: None,
                        },
                    }),
                },
                Node::BottomSheet {
                    state: "commentsPresented".to_owned(),
                    partial: true,
                    large_only: false,
                    show_drag_handle: true,
                    title: None,
                    children: vec![Node::Text {
                        value: Expr::String("Comments".to_owned()),
                        style: TextStyle::default(),
                    }],
                },
            ],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("VerticalPager("));
        assert!(kotlin.contains(
            "PagerDefaults.flingBehavior(state = nexaListState0, pagerSnapDistance = PagerSnapDistance.atMost(1))"
        ));
        assert!(kotlin.contains("Modifier.fillMaxSize()"));
        assert!(
            kotlin.contains(
                "if (nexaListState0.isScrollInProgress) -1 else nexaListState0.settledPage"
            )
        );
        assert!(kotlin.contains("if (firstVisiblePosition >= 0)"));
        assert_eq!(kotlin.matches("@OptIn(").count(), 2);
        assert!(kotlin.contains("internal fun NexaTextPrimitive("));
        assert!(kotlin.contains("textShadow: NexaGlyphShadow? = null"));
        assert!(kotlin.contains("TextStyle(shadow = resolvedTextShadow)"));
        let primitives = shared_component_primitives();
        assert!(primitives.contains(
            "@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class, androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class)\ninternal fun NexaPickerPrimitive("
        ));
        assert!(kotlin.contains(
            "@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class, androidx.compose.foundation.ExperimentalFoundationApi::class)"
        ));
    }

    #[test]
    fn shared_image_elements_use_compose_scopes_across_navigation_screens() {
        let image = Node::Image {
            source: ImageSource::Asset("hero".to_owned()),
            description: "Hero".to_owned(),
            scale: ImageScale::Fit,
            placeholder: None,
            show_loading_indicator: false,
            max_height: None,
            fill_max_size: false,
            shared_element: Some(Expr::String("hero-image".to_owned())),
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "SharedHero".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: vec![Screen {
                id: ScreenId(0),
                name: "Home".to_owned(),
                parameters: Vec::new(),
                states: Vec::new(),
                body: vec![image],
                status_bar: None,
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("NexaSharedTransitionContent {"));
        assert!(kotlin.contains("SharedTransitionLayout"));
        assert!(kotlin.contains("nexaSharedElementModifier(\"hero-image\")"));
        assert!(
            kotlin
                .contains("LocalNexaAnimatedVisibilityScope provides nexaAnimatedVisibilityScope")
        );
    }

    #[test]
    fn navigation_destinations_get_a_title_and_in_app_back_action() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "NavigationParity".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "appearanceMode".to_owned(),
                ty: Type::String,
                initial: Expr::String("system".to_owned()),
                mutable: true,
            }],
            screens: vec![
                Screen {
                    id: ScreenId(0),
                    name: "Home".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Appearance {
                        mode: Expr::State("appearanceMode".to_owned(), Type::String),
                        children: vec![Node::Text {
                            value: Expr::String("Home".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Screen {
                    id: ScreenId(1),
                    name: "Task Details".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![
                        Node::Layout {
                            kind: LayoutKind::Column,
                            spacing: 0.0,
                            style: Default::default(),
                            children: vec![
                                Node::Toolbar {
                                    placement: nexa_ir::ToolbarPlacement::Trailing,
                                    children: vec![Node::Button {
                                        label: Expr::String("Save".to_owned()),
                                        icon: None,
                                        loading: None,
                                        disabled: None,
                                        style: None,
                                        size: None,
                                        shape: None,
                                        tint: None,
                                        glass: false,
                                        actions: Vec::new(),
                                    }],
                                },
                                Node::Text {
                                    value: Expr::String("Details".to_owned()),
                                    style: TextStyle::default(),
                                },
                            ],
                        },
                        Node::Form {
                            show_dividers: true,
                            background: None,
                            children: vec![Node::FormSection {
                                title: Some(Expr::String("Settings".to_owned())),
                                footer: None,
                                show_dividers: None,
                                background: None,
                                children: vec![Node::Text {
                                    value: Expr::String("Form content".to_owned()),
                                    style: TextStyle::default(),
                                }],
                            }],
                        },
                    ],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Automatic,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        let root = kotlin
            .split("composable(route = \"nexa_screen_0\")")
            .nth(1)
            .expect("root destination exists")
            .split("composable(route = \"nexa_screen_1\")")
            .next()
            .expect("root destination ends before the pushed destination");
        let destination = kotlin
            .split("composable(route = \"nexa_screen_1\")")
            .nth(1)
            .expect("pushed destination exists");

        assert!(!root.contains("Scaffold("));
        assert!(destination.contains("NexaNavigationScreenPrimitive("));
        assert!(destination.contains("title = \"Task Details\""));
        assert!(destination.contains("inline = true"));
        assert!(destination.contains("leadingTitle = false"));
        assert!(destination.contains("hasTrailingActions = true"));
        assert!(destination.contains("firstSection = true"));
        assert!(destination.contains(
            "onBack = { if (navController.previousBackStackEntry != null) navController.popBackStack() }"
        ));
        assert!(destination.contains("actions = {"));
        assert!(destination.contains("Button("));
        assert!(destination.contains("NexaAppearancePrimitive(mode = nexa_appearanceMode) {"));
        assert!(kotlin.contains("internal fun NexaNavigationScreenPrimitive("));
        assert!(kotlin.contains("LargeFlexibleTopAppBar("));
        assert!(kotlin.contains("CenterAlignedTopAppBar("));
        assert!(kotlin.contains("androidx.compose.material3.TopAppBar("));
        assert!(kotlin.contains("if (hasTrailingActions)"));
        assert!(kotlin.contains("Dp(16.toFloat())"));
        assert!(
            kotlin
                .contains("LocalNexaFormContentTopInset provides androidx.compose.ui.unit.Dp(0f)")
        );
        assert!(kotlin.contains("height(LocalNexaFormContentTopInset.current)"));
        assert!(kotlin.contains("androidx.compose.ui.unit.Dp(24f)"));
        assert!(kotlin.contains("start = androidx.compose.ui.unit.Dp(24f)"));
        assert!(kotlin.contains("exitUntilCollapsedScrollBehavior()"));
        assert!(kotlin.contains(".nestedScroll(scrollBehavior.nestedScrollConnection)"));
        assert!(kotlin.contains("contentDescription = \"Back\""));
        assert!(!destination.contains("NexaNavigationChrome("));
        assert!(!kotlin.contains(".padding(bottom = navigationBarInset)"));
        assert!(!kotlin.contains("import androidx.compose.material3.TopAppBar\n"));

        let mut large_module = module.clone();
        let Node::NavigationStack { title_mode, .. } = &mut large_module.body[0] else {
            panic!("test module should have a navigation stack");
        };
        *title_mode = nexa_ir::NavigationTitleMode::Large;
        let large_kotlin = generate(&large_module);
        let large_destination = large_kotlin
            .split("composable(route = \"nexa_screen_1\")")
            .nth(1)
            .expect("large-title destination exists");
        assert!(large_destination.contains("inline = false"));

        let mut centered_module = module.clone();
        let Node::NavigationStack {
            title_alignment, ..
        } = &mut centered_module.body[0]
        else {
            panic!("test module should have a navigation stack");
        };
        *title_alignment = nexa_ir::NavigationTitleAlignment::Center;
        let centered_kotlin = generate(&centered_module);
        let centered_destination = centered_kotlin
            .split("composable(route = \"nexa_screen_1\")")
            .nth(1)
            .expect("centered-title destination exists");
        assert!(centered_destination.contains("leadingTitle = false"));

        let mut leading_module = module.clone();
        let Node::NavigationStack {
            title_alignment, ..
        } = &mut leading_module.body[0]
        else {
            panic!("test module should have a navigation stack");
        };
        *title_alignment = nexa_ir::NavigationTitleAlignment::Leading;
        assert!(generate(&leading_module).contains("leadingTitle = true"));

        let mut no_trailing_actions_module = module.clone();
        let Node::Layout { children, .. } = &mut no_trailing_actions_module.screens[1].body[0]
        else {
            panic!("destination body should start with a layout");
        };
        children.retain(|node| {
            !matches!(
                node,
                Node::Toolbar {
                    placement: nexa_ir::ToolbarPlacement::Trailing,
                    ..
                }
            )
        });
        let no_toolbar_kotlin = generate(&no_trailing_actions_module);
        assert!(no_toolbar_kotlin.contains("hasTrailingActions = false"));
        let no_toolbar_destination = no_toolbar_kotlin
            .split("composable(route = \"nexa_screen_1\")")
            .nth(1)
            .expect("pushed destination exists");
        assert!(no_toolbar_destination.contains("leadingTitle = false"));
    }

    #[test]
    fn configured_spring_maps_response_and_damping_to_compose_physics() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "SpringAnimation".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Layout {
                kind: LayoutKind::Column,
                spacing: 0.0,
                style: ViewStyle {
                    animation: Some(AnimationSpec::Spring {
                        response: 0.35,
                        damping: 0.8,
                    }),
                    ..ViewStyle::default()
                },
                children: vec![Node::Text {
                    value: Expr::String("Animated".to_owned()),
                    style: TextStyle::default(),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("spring(dampingRatio = 0.8f, stiffness = 816.32654f)"));
    }

    #[test]
    fn currency_formatting_emits_only_the_reachable_native_helper() {
        let call = Expr::NativeCall {
            receiver: None,
            namespace: "Number".to_owned(),
            name: "formatCurrency".to_owned(),
            arguments: vec![
                (
                    "amount".to_owned(),
                    Expr::Number {
                        raw: "1234.5".to_owned(),
                        ty: NumericType::Float64,
                    },
                ),
                ("currencyCode".to_owned(), Expr::String("EUR".to_owned())),
            ],
            codecs: Vec::new(),
            return_type: Type::String,
            source_span: None,
            is_async: false,
            is_throwing: false,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "CurrencyFormatting".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "price".to_owned(),
                ty: Type::String,
                initial: call,
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::State("price".to_owned(), Type::String),
                style: TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("nexaFormatCurrency(1234.5, \"EUR\")"));
        assert!(kotlin.contains("internal fun nexaFormatCurrency"));
        assert!(kotlin.contains("java.text.NumberFormat.getCurrencyInstance(locale)"));
    }

    #[test]
    fn conditional_view_transitions_use_native_compose_animated_content() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "ConditionalTransitions".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![
                Node::If {
                    condition: Expr::State("visible".to_owned(), Type::Bool),
                    then_body: vec![Node::Text {
                        value: Expr::String("shown".to_owned()),
                        style: TextStyle::default(),
                    }],
                    else_body: None,
                    transition: Some(ViewTransition::Fade),
                    transition_animation: Some(nexa_ir::AnimationSpec::Tween {
                        duration: 0.24,
                        delay: 0.04,
                        easing: nexa_ir::AnimationEasing::EaseOut,
                    }),
                },
                Node::When {
                    value: Expr::State("visible".to_owned(), Type::Bool),
                    cases: vec![WhenCase {
                        value: Expr::Bool(true),
                        body: vec![Node::Text {
                            value: Expr::String("yes".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    else_body: vec![Node::Text {
                        value: Expr::String("no".to_owned()),
                        style: TextStyle::default(),
                    }],
                    transition: Some(ViewTransition::SlideFromBottom),
                    transition_animation: Some(nexa_ir::AnimationSpec::Spring {
                        response: 0.35,
                        damping: 0.8,
                    }),
                },
            ],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("import androidx.compose.animation.AnimatedContent"));
        assert!(kotlin.contains("AnimatedContent(targetState = "));
        assert!(kotlin.contains("fadeIn(animationSpec = androidx.compose.animation.core.tween(durationMillis = 240, delayMillis = 40, easing = androidx.compose.animation.core.LinearOutSlowInEasing)) togetherWith fadeOut(animationSpec = androidx.compose.animation.core.tween(durationMillis = 240, delayMillis = 40, easing = androidx.compose.animation.core.LinearOutSlowInEasing))"));
        assert!(kotlin.contains("slideInVertically(animationSpec = androidx.compose.animation.core.spring(dampingRatio = 0.8f, stiffness = 816.32654f)) { height -> height } togetherWith slideOutVertically(animationSpec = androidx.compose.animation.core.spring(dampingRatio = 0.8f, stiffness = 816.32654f)) { height -> height }"));
    }

    #[test]
    fn double_tap_pressable_emits_handler_and_compose_opt_in() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "DoubleTapApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: true,
                children: vec![Node::Text {
                    value: Expr::String("Tap twice".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: vec![Action::Assign {
                    name: "taps".to_owned(),
                    value: Expr::Number {
                        raw: "2".to_owned(),
                        ty: NumericType::Int32,
                    },
                }],
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);

        assert!(kotlin.contains("import androidx.compose.foundation.combinedClickable"));
        assert!(kotlin.contains("androidx.compose.foundation.ExperimentalFoundationApi::class"));
        assert!(kotlin.contains("combinedClickable("));
        assert!(kotlin.contains(".fillMaxSize().combinedClickable("));
        assert!(kotlin.contains("contentAlignment = Alignment.Center,"));
        let pressable_arguments = kotlin
            .split(".combinedClickable(")
            .nth(1)
            .expect("pressable combined clickable is generated")
            .split("onClick = {")
            .next()
            .expect("pressable click handler follows its arguments");
        assert!(pressable_arguments.contains("interactionSource = null,"));
        assert!(pressable_arguments.contains("role = Role.Button,"));
        assert!(!pressable_arguments.contains("indication = null"));
        assert!(kotlin.contains("onDoubleClick = {"));
        assert!(!kotlin.contains("onLongClick = {"));
        assert!(!kotlin.contains("import androidx.compose.ui.input.pointer"));
    }

    #[test]
    fn drag_pressable_emits_pointer_input_and_typed_callback() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "DragApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![State {
                name: "distance".to_owned(),
                ty: Type::Numeric(NumericType::Float64),
                initial: Expr::Number {
                    raw: "0.0".to_owned(),
                    ty: NumericType::Float64,
                },
                mutable: true,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Drag me".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: vec![Action::Assign {
                    name: "distance".to_owned(),
                    value: Expr::Number {
                        raw: "0.0".to_owned(),
                        ty: NumericType::Float64,
                    },
                }],
                context_menu: Vec::new(),
                drag_parameters: vec![
                    "translationX".to_owned(),
                    "translationY".to_owned(),
                    "velocityX".to_owned(),
                    "velocityY".to_owned(),
                ],
                drag_actions: vec![Action::Assign {
                    name: "distance".to_owned(),
                    value: Expr::State("velocityX".to_owned(), Type::Numeric(NumericType::Float64)),
                }],
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);

        assert!(kotlin.contains("import androidx.compose.foundation.gestures.detectDragGestures"));
        assert!(kotlin.contains("import androidx.compose.ui.input.pointer.pointerInput"));
        assert!(kotlin.contains("import androidx.compose.ui.input.pointer.util.VelocityTracker"));
        assert!(kotlin.contains(".pointerInput(nexaDragDensity, !(false))"));
        assert!(kotlin.contains("nexaPressableGestureConsumed"));
        assert!(kotlin.contains(".value = true"));
        assert!(kotlin.contains("if (nexaPressableGestureConsumed"));
        assert!(kotlin.contains("nexaVelocityTracker.calculateVelocity()"));
        assert!(kotlin.contains("val nexaBaseViewConfiguration"));
        assert!(kotlin.contains("remember(nexaBaseViewConfiguration"));
        assert!(kotlin.contains("ViewConfiguration by nexaBaseViewConfiguration"));
        assert!(!kotlin.contains("ViewConfiguration by LocalViewConfiguration.current"));
        assert!(
            kotlin.contains(
                "val nexa_translationX = (nexaTranslationX / nexaDragDensity).toDouble()"
            )
        );
        assert!(kotlin.contains("nexa_distance = nexa_velocityX"));

        let mut translation_only_module = module;
        let Node::Pressable { drag_actions, .. } = &mut translation_only_module.body[0] else {
            panic!("expected Pressable");
        };
        drag_actions[0] = Action::Assign {
            name: "distance".to_owned(),
            value: Expr::State(
                "translationX".to_owned(),
                Type::Numeric(NumericType::Float64),
            ),
        };
        let kotlin = generate(&translation_only_module);
        assert!(!kotlin.contains("import androidx.compose.ui.input.pointer.util.VelocityTracker"));
        assert!(!kotlin.contains("calculateVelocity()"));
    }

    #[test]
    fn pinch_pressable_emits_scale_delta_and_shares_transform_input_with_drag() {
        let state = |name: &str| State {
            name: name.to_owned(),
            ty: Type::Numeric(NumericType::Float64),
            initial: Expr::Number {
                raw: "1.0".to_owned(),
                ty: NumericType::Float64,
            },
            mutable: true,
        };
        let mut module = Module {
            widgets: Vec::new(),
            app_name: "PinchApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![state("zoom"), state("distance")],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: vec![Node::Text {
                    value: Expr::String("Pinch to zoom".to_owned()),
                    style: TextStyle::default(),
                }],
                actions: Vec::new(),
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: Some("scaleFactor".to_owned()),
                pinch_actions: vec![Action::Assign {
                    name: "zoom".to_owned(),
                    value: Expr::State(
                        "scaleFactor".to_owned(),
                        Type::Numeric(NumericType::Float64),
                    ),
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);

        assert!(
            kotlin.contains("import androidx.compose.foundation.gestures.detectTransformGestures")
        );
        assert!(kotlin.contains("import androidx.compose.ui.input.pointer.pointerInput"));
        assert!(!kotlin.contains("detectDragGestures"));
        // Shared large-title primitives are preloaded into every app, so their
        // imports are present even when this particular module has no title.
        assert!(kotlin.contains("if (nexaZoomChange != 1f)"));
        assert!(kotlin.contains("val nexa_scaleFactor = nexaZoomChange.toDouble()"));
        assert!(kotlin.contains("nexa_zoom = nexa_scaleFactor"));

        let Node::Pressable {
            drag_parameters,
            drag_actions,
            ..
        } = &mut module.body[0]
        else {
            panic!("expected Pressable");
        };
        *drag_parameters = vec![
            "translationX".to_owned(),
            "translationY".to_owned(),
            "velocityX".to_owned(),
            "velocityY".to_owned(),
        ];
        *drag_actions = vec![Action::Assign {
            name: "distance".to_owned(),
            value: Expr::State("velocityX".to_owned(), Type::Numeric(NumericType::Float64)),
        }];
        let kotlin = generate(&module);
        assert!(kotlin.contains("detectTransformGestures { _, nexaPan, nexaZoomChange, _ ->"));
        assert!(!kotlin.contains("detectDragGestures"));
        assert!(kotlin.contains("import android.os.SystemClock"));
        assert!(kotlin.contains("nexa_distance = nexa_velocityX"));
        assert!(kotlin.contains("nexa_zoom = nexa_scaleFactor"));

        let mut standalone_drag = module.body[0].clone();
        let Node::Pressable {
            pinch_parameter,
            pinch_actions,
            ..
        } = &mut standalone_drag
        else {
            panic!("expected Pressable");
        };
        *pinch_parameter = None;
        pinch_actions.clear();
        module.body.push(standalone_drag);

        let kotlin = generate(&module);
        assert!(kotlin.contains("import androidx.compose.foundation.gestures.detectDragGestures"));
        assert!(
            kotlin.contains("import androidx.compose.foundation.gestures.detectTransformGestures")
        );
        assert!(kotlin.contains("import androidx.compose.ui.input.pointer.util.VelocityTracker"));
        assert!(kotlin.contains("import android.os.SystemClock"));
    }

    #[test]
    fn screen_native_class_instances_are_remembered_across_recomposition() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let native_instance_state = |name: &str| State {
            name: name.to_owned(),
            ty: player_type.clone(),
            initial: Expr::Call {
                name: "Video.VideoPlayer".to_owned(),
                arguments: Vec::new(),
                return_type: player_type.clone(),
                is_async: false,
                is_throwing: false,
                is_constructor: true,
            },
            mutable: false,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![
                native_instance_state("appPlayer"),
                State {
                    name: "sharedCount".to_owned(),
                    ty: Type::Numeric(NumericType::Int32),
                    initial: Expr::Number {
                        raw: "7".to_owned(),
                        ty: NumericType::Int32,
                    },
                    mutable: true,
                },
            ],
            screens: vec![
                Screen {
                    id: ScreenId(0),
                    name: "PlayerScreen".to_owned(),
                    parameters: Vec::new(),
                    states: vec![
                        native_instance_state("player"),
                        State {
                            name: "screenCount".to_owned(),
                            ty: Type::Numeric(NumericType::Int32),
                            initial: Expr::Number {
                                raw: "1".to_owned(),
                                ty: NumericType::Int32,
                            },
                            mutable: true,
                        },
                    ],
                    body: vec![Node::NavigationLink {
                        destination: ScreenId(1),
                        arguments: Vec::new(),
                        guard: None,
                        children: vec![Node::Text {
                            value: Expr::String("Push another instance".to_owned()),
                            style: TextStyle::default(),
                        }],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Screen {
                    id: ScreenId(1),
                    name: "Details".to_owned(),
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![Node::Button {
                        label: Expr::String("Play shared player".to_owned()),
                        icon: None,
                        loading: None,
                        disabled: None,
                        style: None,
                        size: None,
                        shape: None,
                        tint: None,
                        glass: false,
                        actions: vec![Action::Expression(Expr::NativeCall {
                            receiver: Some(Box::new(Expr::State(
                                "appPlayer".to_owned(),
                                player_type.clone(),
                            ))),
                            namespace: "Video".to_owned(),
                            name: "play".to_owned(),
                            arguments: Vec::new(),
                            codecs: Vec::new(),
                            return_type: Type::Void,
                            source_span: None,
                            is_async: false,
                            is_throwing: false,
                        })],
                    }],
                    status_bar: None,
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        let app_state_name = nexa_codegen::names::state_name("appPlayer");
        let route_start = kotlin
            .find("composable(route = \"nexa_screen_0\")")
            .expect("screen body must be emitted inside its navigation entry");
        let screen_player = kotlin
            .find(&format!(
                "val {state_name}: VideoPlayer = remember {{ VideoPlayer() }}"
            ))
            .expect("screen native object must be remembered");
        let screen_count = kotlin
            .find("var nexa_screenCount by remember { mutableIntStateOf(1) }")
            .expect("screen-local mutable state must be remembered");
        let app_state = kotlin
            .find("var nexa_sharedCount by remember { mutableIntStateOf(7) }")
            .expect("app state must remain shared above the navigation host");
        let nav_host = kotlin.find("NavHost(").expect("navigation host exists");
        assert!(route_start < screen_player && screen_player < screen_count);
        assert!(screen_count > nav_host);
        assert!(app_state < nav_host);
        assert!(!kotlin[..route_start].contains("nexa_screenCount"));
        assert!(kotlin.contains(&format!(
            "val {app_state_name}: VideoPlayer = remember {{ VideoPlayer() }}"
        )));
        assert!(kotlin.contains("composable(route = \"nexa_screen_1\")"));
        assert!(kotlin.contains("nexa_appPlayer.play()"));
        assert!(kotlin.contains("NexaNavigationLinkPrimitive("));
        assert!(kotlin.contains("onClick = { navController.navigate(\"nexa_screen_1\") }"));
        assert!(kotlin.contains("trailing = {"));
        assert!(kotlin.contains("Icons.Filled.ChevronRight"));
    }

    #[test]
    fn component_native_class_instances_are_remembered_across_recomposition() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: vec![Component {
                name: "PlayerPanel".to_owned(),
                source_file: None,
                parameters: Vec::new(),
                states: vec![State {
                    name: "player".to_owned(),
                    ty: player_type.clone(),
                    initial: Expr::Call {
                        name: "Video.VideoPlayer".to_owned(),
                        arguments: Vec::new(),
                        return_type: player_type,
                        is_async: false,
                        is_throwing: false,
                        is_constructor: true,
                    },
                    mutable: false,
                }],
                body: Vec::new(),
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            body: vec![Node::ComponentCall {
                name: "PlayerPanel".to_owned(),
                arguments: Vec::new(),
                children: None,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        let state_name = nexa_codegen::names::state_name("player");
        assert!(kotlin.contains(&format!(
            "val {state_name}: VideoPlayer = remember {{ VideoPlayer() }}"
        )));
    }

    #[test]
    fn coroutine_scopes_are_emitted_only_for_ui_scopes_that_launch_tasks() {
        fn task_launch() -> Action {
            Action::TaskLaunch {
                handle: None,
                executor: TaskExecutor::Main,
                actions: Vec::new(),
            }
        }

        fn pressable_with_task() -> Node {
            Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
                fill_max_size: false,
                children: Vec::new(),
                actions: vec![task_launch()],
                double_tap_actions: Vec::new(),
                long_press_duration_ms: Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                long_press_actions: Vec::new(),
                context_menu: Vec::new(),
                drag_parameters: Vec::new(),
                drag_actions: Vec::new(),
                pinch_parameter: None,
                pinch_actions: Vec::new(),
            }
        }

        let module = Module {
            widgets: Vec::new(),
            app_name: "TaskScopeProbe".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: vec![Screen {
                id: ScreenId(0),
                name: "BusyScreen".to_owned(),
                parameters: Vec::new(),
                states: Vec::new(),
                body: vec![pressable_with_task()],
                status_bar: None,
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            components: vec![
                Component {
                    name: "IdlePanel".to_owned(),
                    source_file: None,
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: Vec::new(),
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
                Component {
                    name: "BusyPanel".to_owned(),
                    source_file: None,
                    parameters: Vec::new(),
                    states: Vec::new(),
                    body: vec![pressable_with_task()],
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        let scope_declaration = "val nexaTaskScope = rememberCoroutineScope()";
        assert_eq!(
            kotlin.matches(scope_declaration).count(),
            2,
            "unexpected Compose scope placements:\n{kotlin}"
        );
        let screen_route = kotlin
            .split_once("composable(route = \"nexa_screen_0\") {")
            .expect("busy screen route was generated")
            .1;
        assert!(
            screen_route
                .lines()
                .take(4)
                .any(|line| line.contains(scope_declaration))
        );
        let busy_panel = kotlin
            .split_once("NexaBusyPanelComponent() {")
            .expect("busy component was generated")
            .1
            .split_once("\n}")
            .expect("busy component body closes")
            .0;
        assert!(busy_panel.contains(scope_declaration));
        let idle_panel = kotlin
            .split_once("NexaIdlePanelComponent() {")
            .expect("idle component was generated")
            .1
            .split_once("\n}")
            .expect("idle component body closes")
            .0;
        assert!(!idle_panel.contains(scope_declaration));
    }

    #[test]
    fn semantic_text_font_roles_emit_shared_native_metrics() {
        let module = Module {
            widgets: Vec::new(),
            app_name: "SemanticTextStyle".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Task description".to_owned()),
                style: TextStyle {
                    font_style: Some(nexa_ir::TextFontStyle::Subheadline),
                    ..TextStyle::default()
                },
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let generated = generate(&module);
        assert!(generated.contains("fontSize = 15.sp"));
        assert!(!generated.contains("fontWeight = FontWeight.Normal"));
    }

    #[test]
    fn generates_result_and_try_in_kotlin() {
        let err_type = Type::Enum("AppError".to_owned());
        let module = Module {
            widgets: Vec::new(),
            app_name: "ResultApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            background_tasks: Vec::new(),
            enums: vec![nexa_ir::EnumDecl {
                name: "AppError".to_owned(),
                cases: vec!["NotFound".to_owned(), "Unauthorized".to_owned()],
            }],
            structs: Vec::new(),
            functions: vec![
                Function {
                    name: "fetchCode".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    body_actions: None,
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::ResultOk {
                        value: Box::new(Expr::Number {
                            raw: "42".to_owned(),
                            ty: NumericType::Int32,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
                Function {
                    name: "compute".to_owned(),
                    receiver: None,
                    class_initializers: Vec::new(),
                    is_async: false,
                    is_throwing: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
                    body_actions: None,
                    return_type: Type::Result(
                        Box::new(Type::Numeric(NumericType::Int32)),
                        Box::new(err_type.clone()),
                    ),
                    body: Expr::Try {
                        expr: Box::new(Expr::Call {
                            name: "fetchCode".to_owned(),
                            arguments: Vec::new(),
                            return_type: Type::Result(
                                Box::new(Type::Numeric(NumericType::Int32)),
                                Box::new(err_type.clone()),
                            ),
                            is_async: false,
                            is_throwing: false,
                            is_constructor: false,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
            ],
            globals: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("public sealed class NexaResult<out T, out E> {"));
        assert!(kotlin.contains("NexaResult<Int, NexaAppError>"));
        assert!(kotlin.contains("NexaResult.Success(42)"));
        assert!(kotlin.contains("nexa_fn_fetchCode().getOrThrow()"));
    }

    #[test]
    fn screen_result_state_emits_shared_result_definition() {
        // A screen-local `Result` state renders `NexaResult<…>` in the screen
        // composable, so the shared sealed class must be emitted even though
        // no function, app state, struct, or component mentions `Result`.
        let err_type = Type::Enum("AppError".to_owned());
        let result_ty = Type::Result(
            Box::new(Type::Numeric(NumericType::Int32)),
            Box::new(err_type.clone()),
        );
        let module = Module {
            widgets: Vec::new(),
            app_name: "ScreenResultApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: vec![nexa_ir::EnumDecl {
                name: "AppError".to_owned(),
                cases: vec!["NotFound".to_owned()],
            }],
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: Vec::new(),
            screens: vec![Screen {
                id: ScreenId(0),
                name: "Main".to_owned(),
                parameters: Vec::new(),
                states: vec![State {
                    name: "loadResult".to_owned(),
                    ty: result_ty,
                    initial: Expr::ResultOk {
                        value: Box::new(Expr::Number {
                            raw: "0".to_owned(),
                            ty: NumericType::Int32,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type,
                    },
                    mutable: false,
                }],
                body: vec![Node::Text {
                    value: Expr::String("Hello".to_owned()),
                    style: TextStyle::default(),
                }],
                status_bar: None,
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            components: Vec::new(),
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                title_mode: nexa_ir::NavigationTitleMode::Inline,
                title_alignment: nexa_ir::NavigationTitleAlignment::Leading,
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        let state_name = nexa_codegen::names::state_name("loadResult");
        assert!(
            kotlin.contains("public sealed class NexaResult<out T, out E> {"),
            "screen-local Result state must pull in the shared definition"
        );
        assert!(kotlin.contains(&format!(
            "val {state_name}: NexaResult<Int, NexaAppError> = NexaResult.Success(0)"
        )));
    }

    #[test]
    fn app_user_class_instances_use_remember_binding() {
        let class_type = Type::Class {
            name: "NoteStore".to_owned(),
            fields: vec![(
                "notes".to_owned(),
                Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
            )],
            constructor_parameter_count: 0,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "Demo".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![
                State {
                    name: "store".to_owned(),
                    ty: class_type.clone(),
                    initial: Expr::Call {
                        name: "NoteStore".to_owned(),
                        arguments: Vec::new(),
                        return_type: class_type.clone(),
                        is_async: false,
                        is_throwing: false,
                        is_constructor: true,
                    },
                    mutable: false,
                },
                State {
                    name: "notes".to_owned(),
                    ty: Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
                    initial: Expr::Member {
                        base: Box::new(Expr::State("store".to_owned(), class_type)),
                        name: "notes".to_owned(),
                        optional: false,
                        base_type: Type::Class {
                            name: "NoteStore".to_owned(),
                            fields: Vec::new(),
                            constructor_parameter_count: 0,
                        },
                        field_type: Type::Signal(Box::new(Type::Array(Box::new(Type::String)))),
                        kind: nexa_ir::MemberKind::ClassField("notes".to_owned()),
                    },
                    mutable: false,
                },
            ],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Text {
                value: Expr::String("Hello".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let kotlin = generate(&module);
        assert!(kotlin.contains("val nexa_store: NexaNoteStore = remember { NexaNoteStore() }"));
    }
}
