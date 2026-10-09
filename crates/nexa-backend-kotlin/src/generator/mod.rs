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
        uses_system_icons: !features.facts.ui.system_icons.is_empty() || features.uses_picker,
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
        uses_compose_animation: features.uses_conditional_transition
            || features.uses_shared_elements
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
    generate_with_analysis(module, features)
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
    // Shared-transition APIs can be introduced by hot reload without
    // rebuilding the development host.
    features.uses_shared_elements = true;
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
    let generated = generate_units(module, &features);
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
const NEXA_SHARED_COMPONENT_PRIMITIVES: &str = r#"@androidx.compose.runtime.Composable
internal fun NexaTextPrimitive(
    text: String,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    color: androidx.compose.ui.graphics.Color = androidx.compose.ui.graphics.Color.Unspecified,
    fontSize: androidx.compose.ui.unit.TextUnit = androidx.compose.ui.unit.TextUnit.Unspecified,
    fontWeight: androidx.compose.ui.text.font.FontWeight? = null,
    textAlign: androidx.compose.ui.text.style.TextAlign? = null,
    maxLines: Int = Int.MAX_VALUE,
    lineHeight: androidx.compose.ui.unit.TextUnit = androidx.compose.ui.unit.TextUnit.Unspecified,
    letterSpacing: androidx.compose.ui.unit.TextUnit = androidx.compose.ui.unit.TextUnit.Unspecified,
    strikethrough: Boolean = false,
    softWrap: Boolean = true,
    selectable: Boolean = false,
) {
    val content: @androidx.compose.runtime.Composable () -> Unit = {
        androidx.compose.material3.Text(
            text = text,
            modifier = modifier,
            color = color,
            fontSize = fontSize,
            fontWeight = fontWeight,
            textAlign = textAlign,
            maxLines = maxLines,
            lineHeight = lineHeight,
            letterSpacing = letterSpacing,
            textDecoration = if (strikethrough) androidx.compose.ui.text.style.TextDecoration.LineThrough else null,
            softWrap = softWrap,
        )
    }
    if (selectable) androidx.compose.foundation.text.selection.SelectionContainer { content() } else content()
}

@androidx.compose.runtime.Composable
internal fun NexaButtonPrimitive(
    label: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector?,
    style: String,
    size: String?,
    shape: androidx.compose.ui.graphics.Shape?,
    tint: androidx.compose.ui.graphics.Color?,
    loading: Boolean,
    disabled: Boolean,
    iconOnlyCircle: Boolean,
    fullWidth: Boolean,
    onClick: () -> Unit,
) {
    val minHeight = if (size == "Large") androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LARGE_MIN_HEIGHT__.toFloat()) else androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_TAP_TARGET__.toFloat())
    val modifier = when {
        fullWidth -> androidx.compose.ui.Modifier.fillMaxWidth().defaultMinSize(minWidth = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_WIDTH__.toFloat()), minHeight = minHeight)
        iconOnlyCircle -> androidx.compose.ui.Modifier.size(minHeight)
        else -> androidx.compose.ui.Modifier.defaultMinSize(minWidth = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_WIDTH__.toFloat()), minHeight = minHeight)
    }
    val resolvedShape = shape ?: androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)
    val contentPadding = when {
        iconOnlyCircle || style == "Plain" || style == "Borderless" -> androidx.compose.foundation.layout.PaddingValues(androidx.compose.ui.unit.Dp(0f))
        size == "Small" -> androidx.compose.foundation.layout.PaddingValues(horizontal = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_SMALL_HORIZONTAL_PADDING__.toFloat()), vertical = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_SMALL_VERTICAL_PADDING__.toFloat()))
        size == "Large" -> androidx.compose.foundation.layout.PaddingValues(horizontal = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LARGE_HORIZONTAL_PADDING__.toFloat()), vertical = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LARGE_VERTICAL_PADDING__.toFloat()))
        else -> androidx.compose.foundation.layout.PaddingValues(horizontal = androidx.compose.ui.unit.Dp(8f), vertical = androidx.compose.ui.unit.Dp(8f))
    }
    val buttonContent: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit = {
        if (loading) {
            androidx.compose.material3.CircularProgressIndicator()
        } else {
            if (icon != null) {
                androidx.compose.material3.Icon(imageVector = icon, contentDescription = null, modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat())))
            }
            if (icon != null && label.isNotEmpty()) {
                androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.width(androidx.compose.ui.unit.Dp(__NEXA_ICON_LABEL_SPACING__.toFloat())))
            }
            NexaTextPrimitive(
                text = label,
                color = if (style == "Plain" || style == "Borderless") androidx.compose.material3.MaterialTheme.colorScheme.onSurface else androidx.compose.ui.graphics.Color.Unspecified,
                fontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
                maxLines = 1,
                lineHeight = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
                letterSpacing = 0.sp,
                softWrap = false,
            )
        }
    }
    val enabled = !loading && !disabled
    when (style) {
        "Plain", "Borderless" -> androidx.compose.material3.TextButton(
            modifier = modifier,
            onClick = onClick,
            enabled = enabled,
            shape = resolvedShape,
            colors = androidx.compose.material3.ButtonDefaults.textButtonColors(contentColor = tint ?: androidx.compose.ui.graphics.Color.Unspecified),
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
            androidx.compose.material3.OutlinedButton(
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

@androidx.compose.runtime.Composable
internal fun NexaTextInputPrimitive(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
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
    androidx.compose.material3.TextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier,
        placeholder = placeholder.takeIf(String::isNotEmpty)?.let { { androidx.compose.material3.Text(it) } },
        textStyle = textStyle,
        minLines = minLines,
        maxLines = maxLines,
        leadingIcon = if (searchable && searchIcon != null) {
            { androidx.compose.material3.Icon(imageVector = searchIcon, contentDescription = null) }
        } else null,
        singleLine = singleLine,
        colors = androidx.compose.material3.TextFieldDefaults.colors(
            focusedContainerColor = androidx.compose.ui.graphics.Color.Transparent,
            unfocusedContainerColor = androidx.compose.ui.graphics.Color.Transparent,
            disabledContainerColor = androidx.compose.ui.graphics.Color.Transparent,
            errorContainerColor = androidx.compose.ui.graphics.Color.Transparent,
            focusedIndicatorColor = androidx.compose.ui.graphics.Color.Transparent,
            unfocusedIndicatorColor = androidx.compose.ui.graphics.Color.Transparent,
            disabledIndicatorColor = androidx.compose.ui.graphics.Color.Transparent,
            errorIndicatorColor = androidx.compose.ui.graphics.Color.Transparent,
        ),
        visualTransformation = if (secure) {
            androidx.compose.ui.text.input.PasswordVisualTransformation()
        } else {
            androidx.compose.ui.text.input.VisualTransformation.None
        },
        keyboardOptions = keyboardOptions,
        keyboardActions = keyboardActions,
    )
}

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
        modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
    ) {
        NexaTextPrimitive(
            text = label,
            modifier = androidx.compose.ui.Modifier.weight(1f),
            fontSize = fontSize,
            fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
            lineHeight = lineHeight,
        )
        androidx.compose.material3.Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
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

@androidx.compose.runtime.Composable
internal fun NexaProgressBarPrimitive(progress: Float) {
    androidx.compose.material3.LinearProgressIndicator(progress = { progress.coerceIn(0f, 1f) })
}

@androidx.compose.runtime.Composable
internal fun NexaProgressRingPrimitive(progress: Float) {
    androidx.compose.material3.CircularProgressIndicator(progress = { progress.coerceIn(0f, 1f) })
}

@androidx.compose.runtime.Composable
internal fun NexaDividerPrimitive(color: androidx.compose.ui.graphics.Color, thickness: androidx.compose.ui.unit.Dp) {
    androidx.compose.material3.HorizontalDivider(color = color, thickness = thickness)
}

@androidx.compose.runtime.Composable
internal fun NexaSegmentedControlPrimitive(
    items: List<String>,
    selected: String,
    onSelected: (String) -> Unit,
) {
    androidx.compose.foundation.layout.Row(
        horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(androidx.compose.ui.unit.Dp(4f)),
    ) {
        items.forEach { item ->
            androidx.compose.material3.TextButton(
                onClick = { onSelected(item) },
                shape = androidx.compose.foundation.shape.RoundedCornerShape(percent = 50),
                colors = androidx.compose.material3.ButtonDefaults.textButtonColors(
                    containerColor = if (selected == item) androidx.compose.material3.MaterialTheme.colorScheme.secondaryContainer else androidx.compose.ui.graphics.Color.Transparent,
                    contentColor = if (selected == item) androidx.compose.material3.MaterialTheme.colorScheme.onSecondaryContainer else androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
                ),
            ) {
                NexaTextPrimitive(text = item)
            }
        }
    }
}

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

@androidx.compose.runtime.Composable
internal fun NexaContentUnavailablePrimitive(
    title: String,
    description: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
) {
    androidx.compose.foundation.layout.Column(
        modifier = androidx.compose.ui.Modifier
            .fillMaxSize()
            .padding(horizontal = androidx.compose.ui.unit.Dp(16f), vertical = androidx.compose.ui.unit.Dp(32f)),
        verticalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
        horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally,
    ) {
        androidx.compose.material3.Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(48f)),
            tint = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
        )
        androidx.compose.foundation.layout.Spacer(modifier = androidx.compose.ui.Modifier.height(androidx.compose.ui.unit.Dp(16f)))
        androidx.compose.material3.Text(
            title,
            style = androidx.compose.material3.MaterialTheme.typography.titleLarge.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Bold),
            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
        )
        androidx.compose.foundation.layout.Spacer(modifier = androidx.compose.ui.Modifier.height(androidx.compose.ui.unit.Dp(8f)))
        androidx.compose.material3.Text(
            description,
            style = androidx.compose.material3.MaterialTheme.typography.bodyLarge,
            color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
        )
    }
}

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
    androidx.compose.foundation.layout.Column(modifier, verticalArrangement = arrangement, horizontalAlignment = horizontalAlignment, content = content)
}

@androidx.compose.runtime.Composable
internal fun NexaRowPrimitive(
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    spacing: Float = 0f,
    verticalAlignment: androidx.compose.ui.Alignment.Vertical = androidx.compose.ui.Alignment.CenterVertically,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit,
) {
    val arrangement = if (spacing > 0f) {
        androidx.compose.foundation.layout.Arrangement.spacedBy(androidx.compose.ui.unit.Dp(spacing))
    } else androidx.compose.foundation.layout.Arrangement.Start
    androidx.compose.foundation.layout.Row(modifier, horizontalArrangement = arrangement, verticalAlignment = verticalAlignment, content = content)
}

@androidx.compose.runtime.Composable
internal fun NexaStackPrimitive(
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    contentAlignment: androidx.compose.ui.Alignment = androidx.compose.ui.Alignment.Center,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.BoxScope.() -> Unit,
) {
    androidx.compose.foundation.layout.Box(modifier, contentAlignment = contentAlignment, content = content)
}

@androidx.compose.runtime.Composable
internal fun NexaFormRowPrimitive(
    minHeight: Float,
    horizontalInset: Float,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.BoxScope.() -> Unit,
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

@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun NexaPickerPrimitive(
    items: List<String>,
    selected: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector?,
    label: String?,
    tint: androidx.compose.ui.graphics.Color?,
    identity: String? = null,
    onSelectionChanged: (String) -> Unit,
) {
    val expanded = androidx.compose.runtime.remember(identity) { androidx.compose.runtime.mutableStateOf(false) }
    androidx.compose.foundation.layout.Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
        if (label != null && icon != null) {
            androidx.compose.material3.Icon(
                imageVector = icon,
                contentDescription = null,
                tint = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary,
                modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat())),
            )
            androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.width(androidx.compose.ui.unit.Dp(__NEXA_ICON_LABEL_SPACING__.toFloat())))
        }
        if (label != null) {
            NexaTextPrimitive(
                text = label,
                modifier = androidx.compose.ui.Modifier.weight(1f),
                fontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
                lineHeight = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
                letterSpacing = 0.sp,
            )
        }
        androidx.compose.foundation.layout.Box {
            androidx.compose.foundation.layout.Row(
                modifier = androidx.compose.ui.Modifier.clickable { expanded.value = true }.padding(vertical = androidx.compose.ui.unit.Dp(8f)),
                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
            ) {
                if (icon != null && label == null) {
                    NexaSystemIconPrimitive(
                        image = icon,
                        description = selected,
                        size = androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat()),
                        tint = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
                    )
                } else {
                    NexaTextPrimitive(
                        text = selected,
                        color = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary,
                        fontSize = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp,
                        fontWeight = androidx.compose.ui.text.font.FontWeight.Normal,
                        maxLines = 1,
                        lineHeight = __NEXA_DEFAULT_BODY_FONT_SIZE__.sp * __NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__f,
                        letterSpacing = 0.sp,
                        softWrap = false,
                    )
                    androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.width(androidx.compose.ui.unit.Dp(4f)))
                    androidx.compose.material3.Icon(
                        imageVector = androidx.compose.material.icons.Icons.Filled.UnfoldMore,
                        contentDescription = null,
                        tint = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary,
                        modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(16f)),
                    )
                }
            }
            androidx.compose.material3.DropdownMenu(
                expanded = expanded.value,
                onDismissRequest = { expanded.value = false },
            ) {
                items.forEach { item ->
                    androidx.compose.material3.DropdownMenuItem(
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

@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun NexaDatePickerPrimitive(
    timestamp: Long,
    includesTime: Boolean,
    identity: String,
    onTimestampChanged: (Long) -> Unit,
    onIncludesTimeChanged: (Boolean) -> Unit,
) {
    androidx.compose.runtime.key(identity) {
        val initialDate = java.util.Calendar.getInstance().apply { timeInMillis = timestamp }
        val initialDateMillis = java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(initialDate.get(java.util.Calendar.YEAR), initialDate.get(java.util.Calendar.MONTH), initialDate.get(java.util.Calendar.DAY_OF_MONTH))
        }.timeInMillis
        val datePickerState = androidx.compose.material3.rememberDatePickerState(initialSelectedDateMillis = initialDateMillis)
        val timePickerState = androidx.compose.material3.rememberTimePickerState(
            initialHour = initialDate.get(java.util.Calendar.HOUR_OF_DAY),
            initialMinute = initialDate.get(java.util.Calendar.MINUTE),
        )
        val dateDialog = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }
        val timeDialog = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }

        androidx.compose.foundation.layout.Column {
            androidx.compose.foundation.layout.Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                NexaTextPrimitive(text = "Include time", modifier = androidx.compose.ui.Modifier.weight(1f))
                androidx.compose.material3.Switch(
                    checked = includesTime,
                    onCheckedChange = { enabled ->
                        onIncludesTimeChanged(enabled)
                        if (!enabled) {
                            val calendar = java.util.Calendar.getInstance().apply {
                                timeInMillis = timestamp
                                set(java.util.Calendar.HOUR_OF_DAY, 0)
                                set(java.util.Calendar.MINUTE, 0)
                                set(java.util.Calendar.SECOND, 0)
                                set(java.util.Calendar.MILLISECOND, 0)
                            }
                            onTimestampChanged(calendar.timeInMillis)
                        }
                    },
                    modifier = androidx.compose.ui.Modifier.semantics { contentDescription = "Include time" },
                )
            }
            androidx.compose.foundation.layout.Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                androidx.compose.material3.OutlinedButton(
                    modifier = androidx.compose.ui.Modifier.weight(1f),
                    onClick = {
                        val localDate = java.util.Calendar.getInstance().apply { timeInMillis = timestamp }
                        datePickerState.selectedDateMillis = java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC")).apply {
                            clear()
                            set(localDate.get(java.util.Calendar.YEAR), localDate.get(java.util.Calendar.MONTH), localDate.get(java.util.Calendar.DAY_OF_MONTH))
                        }.timeInMillis
                        dateDialog.value = true
                    },
                ) {
                    androidx.compose.foundation.layout.Column {
                        androidx.compose.material3.Text(text = "Date", style = androidx.compose.material3.MaterialTheme.typography.labelSmall, color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant)
                        NexaTextPrimitive(text = java.text.DateFormat.getDateInstance(java.text.DateFormat.MEDIUM).format(java.util.Date(timestamp)))
                    }
                }
                if (includesTime) {
                    androidx.compose.foundation.layout.Spacer(androidx.compose.ui.Modifier.width(androidx.compose.ui.unit.Dp(8f)))
                    androidx.compose.material3.OutlinedButton(
                        modifier = androidx.compose.ui.Modifier.weight(1f),
                        onClick = {
                            val localTime = java.util.Calendar.getInstance().apply { timeInMillis = timestamp }
                            timePickerState.hour = localTime.get(java.util.Calendar.HOUR_OF_DAY)
                            timePickerState.minute = localTime.get(java.util.Calendar.MINUTE)
                            timeDialog.value = true
                        },
                    ) {
                        androidx.compose.foundation.layout.Column {
                            androidx.compose.material3.Text(text = "Time", style = androidx.compose.material3.MaterialTheme.typography.labelSmall, color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant)
                            NexaTextPrimitive(text = java.text.DateFormat.getTimeInstance(java.text.DateFormat.SHORT).format(java.util.Date(timestamp)))
                        }
                    }
                }
            }
            if (dateDialog.value) {
                androidx.compose.material3.DatePickerDialog(
                    onDismissRequest = { dateDialog.value = false },
                    confirmButton = {
                        androidx.compose.material3.TextButton(onClick = {
                            datePickerState.selectedDateMillis?.let { selectedMillis ->
                                val selectedDate = java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC")).apply { timeInMillis = selectedMillis }
                                val calendar = java.util.Calendar.getInstance().apply {
                                    clear()
                                    set(
                                        selectedDate.get(java.util.Calendar.YEAR),
                                        selectedDate.get(java.util.Calendar.MONTH),
                                        selectedDate.get(java.util.Calendar.DAY_OF_MONTH),
                                        if (includesTime) timePickerState.hour else 0,
                                        if (includesTime) timePickerState.minute else 0,
                                    )
                                }
                                onTimestampChanged(calendar.timeInMillis)
                            }
                            dateDialog.value = false
                        }) { androidx.compose.material3.Text("Done") }
                    },
                    dismissButton = {
                        androidx.compose.material3.TextButton(onClick = { dateDialog.value = false }) { androidx.compose.material3.Text("Cancel") }
                    },
                ) {
                    androidx.compose.material3.DatePicker(state = datePickerState, title = { androidx.compose.material3.Text("Select date") })
                }
            }
            if (timeDialog.value) {
                androidx.compose.material3.AlertDialog(
                    onDismissRequest = { timeDialog.value = false },
                    title = { androidx.compose.material3.Text("Select time") },
                    text = { androidx.compose.material3.TimePicker(state = timePickerState) },
                    confirmButton = {
                        androidx.compose.material3.TextButton(onClick = {
                            val calendar = java.util.Calendar.getInstance().apply {
                                timeInMillis = timestamp
                                set(java.util.Calendar.HOUR_OF_DAY, timePickerState.hour)
                                set(java.util.Calendar.MINUTE, timePickerState.minute)
                                set(java.util.Calendar.SECOND, 0)
                                set(java.util.Calendar.MILLISECOND, 0)
                            }
                            onTimestampChanged(calendar.timeInMillis)
                            timeDialog.value = false
                        }) { androidx.compose.material3.Text("Done") }
                    },
                    dismissButton = {
                        androidx.compose.material3.TextButton(onClick = { timeDialog.value = false }) { androidx.compose.material3.Text("Cancel") }
                    },
                )
            }
            androidx.compose.runtime.LaunchedEffect(timestamp) {
                val localDate = java.util.Calendar.getInstance().apply { timeInMillis = timestamp }
                datePickerState.selectedDateMillis = java.util.Calendar.getInstance(java.util.TimeZone.getTimeZone("UTC")).apply {
                    clear()
                    set(localDate.get(java.util.Calendar.YEAR), localDate.get(java.util.Calendar.MONTH), localDate.get(java.util.Calendar.DAY_OF_MONTH))
                }.timeInMillis
                timePickerState.hour = localDate.get(java.util.Calendar.HOUR_OF_DAY)
                timePickerState.minute = localDate.get(java.util.Calendar.MINUTE)
            }
        }
    }
}

"#;

fn shared_component_primitives() -> String {
    let tokens = [
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
            "__NEXA_DARK_ACCENT_ARGB__",
            format!("0x{:08X}", nexa_codegen::design_system::DARK_ACCENT_ARGB),
        ),
        (
            "__NEXA_DEFAULT_ACCENT_ARGB__",
            format!("0x{:08X}", nexa_codegen::design_system::DEFAULT_ACCENT_ARGB),
        ),
    ];
    tokens.iter().fold(
        NEXA_SHARED_COMPONENT_PRIMITIVES.to_owned(),
        |source, (token, value)| source.replace(token, value),
    )
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

fn generate_with_analysis(module: &Module, features: &features::Features) -> GeneratedSources {
    let focus_bindings = features.facts.focus_bindings.app.clone();
    let imports = engine::imports::render(engine::imports::ImportContext {
        features,
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
        out.push_str(&shared_component_primitives());
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

    if features.facts.capabilities.uses_json_api {
        units.write("json", |out| {
            api::json::render(&json_types, &module.enums, out);
        });
    }

    units.write("app", |out| {
            let mut opt_in_annotations = Vec::with_capacity(3);
            opt_in_annotations.push("androidx.compose.material3.ExperimentalMaterial3Api::class");
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
            if features.uses_tasks {
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
            out.push_str("    CompositionLocalProvider(LocalRippleConfiguration provides null) {\n");
            if features.uses_shared_elements {
                out.push_str("        NexaSharedTransitionContent {\n");
            }
            let body_base_depth = if features.uses_shared_elements { 3 } else { 2 };
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
            if features.uses_shared_elements {
                out.push_str("\n        }");
            }
            out.push_str("\n    }\n}\n");
    });

    if features.uses_shared_elements {
        units.write("shared-elements", components::shared_elements::render);
    }

    units.write("components", |out| {
        custom_components::render(module, features, out);
        out.push_str(COLLECTION_ITERATION_HELPERS);
        if features.uses_tasks {
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
    if features.uses_asset || features.uses_placeholder {
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
    units.finish()
}

#[cfg(test)]
mod tests {
    use super::{generate, shared_component_primitives};
    use nexa_ir::{
        Action, AnimationSpec, Component, Expr, Function, ImageScale, ImageSource, LayoutKind,
        ListAxis, ListCommon, ListPlan, Module, Node, NumericType, Screen, ScreenId, State,
        SystemIcon, TextStyle, Type, ViewStyle, ViewTransition, WhenCase,
    };

    #[test]
    fn shared_button_primitive_owns_platform_defaults_for_aot_and_dev_runtime() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains("internal fun NexaButtonPrimitive("));
        assert!(primitives.contains("Dp(64.toFloat())"));
        assert!(primitives.contains("Dp(48.toFloat())"));
        assert!(primitives.contains("Dp(50.toFloat())"));
        assert!(primitives.contains("Dp(24.toFloat())"));
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
    fn shared_picker_primitive_owns_its_menu_layout_and_text_metrics() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaPickerPrimitive("));
        assert!(primitives.contains("DropdownMenuItem("));
        assert!(primitives.contains("Icons.Filled.UnfoldMore"));
        assert!(primitives.contains("Modifier.clickable { expanded.value = true }"));
        assert!(primitives.contains("fontSize = 17.sp"));
    }

    #[test]
    fn shared_date_picker_primitive_owns_dialog_state_and_timestamp_conversion() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaDatePickerPrimitive("));
        assert!(
            primitives
                .contains("rememberDatePickerState(initialSelectedDateMillis = initialDateMillis)")
        );
        assert!(primitives.contains("onTimestampChanged(calendar.timeInMillis)"));
        assert!(primitives.contains("LaunchedEffect(timestamp)"));
        assert!(primitives.contains("key(identity)"));
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
        assert!(kotlin.contains("nexaContextMenuExpanded.value = true"));
        assert!(kotlin.contains("DropdownMenuItem("));
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
        assert!(
            kotlin.contains("MaterialTheme(colorScheme = remember(\"dark\", nexaSystemDarkTheme")
        );
        assert!(kotlin.contains(
            "Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background, contentColor = MaterialTheme.colorScheme.onBackground)"
        ));
        assert!(kotlin.contains("window.decorView.setBackgroundColor(nexaAppearanceStatusColor"));
        assert!(kotlin.contains(".isAppearanceLightNavigationBars = "));
        assert!(kotlin.contains("window.isNavigationBarContrastEnforced = false"));
        assert!(kotlin.contains(
            "NexaTextPrimitive(text = \"Hello\", fontSize = 17.sp, fontWeight = FontWeight.Normal, lineHeight = 17.sp * 1.2f, letterSpacing = 0.sp, strikethrough = false, selectable = false)"
        ));
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
            body: vec![Node::ContentUnavailable {
                title: Expr::String("Inbox is empty".to_owned()),
                icon: SystemIcon::shared("inbox").expect("shared inbox icon"),
                description: Expr::String("Tasks you add will appear here.".to_owned()),
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
        assert!(kotlin.contains("internal fun NexaContentUnavailablePrimitive("));
        assert!(kotlin.contains("NexaContentUnavailablePrimitive(title = \"Inbox is empty\""));
        assert!(kotlin.contains("description = \"Tasks you add will appear here.\""));
        assert!(kotlin.contains(
            "modifier = androidx.compose.ui.Modifier.size(androidx.compose.ui.unit.Dp(48f))"
        ));
        assert!(kotlin.contains("Icons.Filled.Inbox"));
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
                    plan: ListPlan::Count {
                        count: Expr::Number {
                            raw: "3".to_owned(),
                            ty: NumericType::Int32,
                        },
                        common: ListCommon {
                            axis: ListAxis::Vertical,
                            native: false,
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
                        },
                    },
                },
                Node::BottomSheet {
                    state: "commentsPresented".to_owned(),
                    partial: true,
                    large_only: false,
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
        assert_eq!(kotlin.matches("@OptIn(").count(), 3);
        assert!(kotlin.contains(
            "@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)\ninternal fun NexaPickerPrimitive("
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
            max_height: None,
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
                    body: vec![Node::Text {
                        value: Expr::String("Details".to_owned()),
                        style: TextStyle::default(),
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
        assert!(destination.contains("Scaffold("));
        assert!(destination.contains("title = { Text(\"Task Details\") }"));
        assert!(destination.contains(
            "MaterialTheme(colorScheme = remember(nexa_appearanceMode, nexaSystemDarkTheme"
        ));
        assert!(destination.contains("navController.popBackStack()"));
        assert!(destination.contains("contentDescription = \"Back\""));
        assert!(destination.contains("Modifier.fillMaxSize().padding(innerPadding)"));
        assert!(kotlin.contains("import androidx.compose.material3.TopAppBar\n"));
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
        assert!(kotlin.contains("fadeIn() togetherWith fadeOut()"));
        assert!(kotlin.contains("slideInVertically { height -> height } togetherWith slideOutVertically { height -> height }"));
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
                long_press_actions: Vec::new(),
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
        assert!(kotlin.contains("nexaVelocityTracker.calculateVelocity()"));
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
        assert!(!kotlin.contains("LocalDensity"));
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
        assert!(kotlin.contains(
            "Row(modifier = Modifier.fillMaxWidth().clickable { navController.navigate(\"nexa_screen_1\") }, verticalAlignment = Alignment.CenterVertically)"
        ));
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

        assert!(generate(&module).contains("fontSize = 15.sp, fontWeight = FontWeight.Normal"));
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
