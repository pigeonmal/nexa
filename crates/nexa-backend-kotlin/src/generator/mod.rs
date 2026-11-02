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
    accessibility, assets, bottom_bar, bottom_sheet, controls, custom_components, dialogs, images,
    input, keyboard, layout, links, lists, navigation, refresh, system_icons,
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
        uses_saveable_state: module
            .states
            .iter()
            .chain(
                module
                    .screens
                    .iter()
                    .flat_map(|screen| screen.states.iter()),
            )
            .any(state::uses_saveable_state),
        uses_expressive_menus: features.uses_picker || features.uses_context_menu,
        uses_expressive_motion: features.uses_segmented_control,
        uses_wavy_progress_bar: features.uses_wavy_progress_bar,
        uses_wavy_progress_ring: features.uses_wavy_progress_ring,
        uses_compose_animation: features.uses_conditional_transition
            || features.uses_shared_elements
            || features.uses_segmented_control
            || features.uses_animation
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

pub(super) fn analyze_project_features(module: &Module) -> crate::KotlinProjectFeatures {
    let features = features::Features::analyze(module);
    project_features_from_analysis(module, &features)
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

pub(super) fn generate_units_with_network_and_project_features(
    module: &Module,
) -> (GeneratedSources, crate::KotlinProjectFeatures) {
    let mut features = features::Features::analyze(module);
    features.uses_network_api = true;
    features.uses_native_library = true;
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
    // The development host ships every shared component primitive so routes
    // added by hot reload do not need a host rebuild. Keep those primitives'
    // imports and Gradle feature requirements in sync with the emitted source.
    features.uses_keyboard_aware = true;
    features.uses_safe_area_inset = true;
    features.uses_safe_area_endpoint = true;
    features.uses_page_pager = true;
    features.uses_picker = true;
    features.uses_progress_bar = true;
    features.uses_navigation_link = true;
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
    r#"// nexa-codegen: NexaSafeAreaEdge
internal enum class NexaSafeAreaEdge { Top, Bottom, Leading, Trailing }

// nexa-codegen: NexaSafeAreaInsets
internal data class NexaSafeAreaInsets(
    val top: androidx.compose.ui.unit.Dp = 0.dp,
    val bottom: androidx.compose.ui.unit.Dp = 0.dp,
    val start: androidx.compose.ui.unit.Dp = 0.dp,
    val end: androidx.compose.ui.unit.Dp = 0.dp,
) {
    fun get(edge: NexaSafeAreaEdge): androidx.compose.ui.unit.Dp = when (edge) {
        NexaSafeAreaEdge.Top -> top
        NexaSafeAreaEdge.Bottom -> bottom
        NexaSafeAreaEdge.Leading -> start
        NexaSafeAreaEdge.Trailing -> end
    }

    fun offset(
        edge: NexaSafeAreaEdge,
        amount: androidx.compose.ui.unit.Dp,
    ): NexaSafeAreaInsets = when (edge) {
        NexaSafeAreaEdge.Top -> copy(top = top + amount)
        NexaSafeAreaEdge.Bottom -> copy(bottom = bottom + amount)
        NexaSafeAreaEdge.Leading -> copy(start = start + amount)
        NexaSafeAreaEdge.Trailing -> copy(end = end + amount)
    }

    operator fun plus(other: NexaSafeAreaInsets) = NexaSafeAreaInsets(
        top = top + other.top,
        bottom = bottom + other.bottom,
        start = start + other.start,
        end = end + other.end,
    )

    fun subtractClamped(other: NexaSafeAreaInsets) = NexaSafeAreaInsets(
        top = (top - other.top).coerceAtLeast(0.dp),
        bottom = (bottom - other.bottom).coerceAtLeast(0.dp),
        start = (start - other.start).coerceAtLeast(0.dp),
        end = (end - other.end).coerceAtLeast(0.dp),
    )
}

// nexa-codegen: NexaSafeAreaContext
internal data class NexaSafeAreaContext(
    val consumed: NexaSafeAreaInsets = NexaSafeAreaInsets(),
    val occupied: NexaSafeAreaInsets = NexaSafeAreaInsets(),
    val endpoint: NexaSafeAreaInsets = NexaSafeAreaInsets(),
)

// nexa-codegen: LocalNexaSafeAreaContext
internal val LocalNexaSafeAreaContext =
    androidx.compose.runtime.staticCompositionLocalOf { NexaSafeAreaContext() }

// nexa-codegen: NexaVisualSafeAreaSnapshot
internal data class NexaVisualSafeAreaSnapshot(
    val top: androidx.compose.ui.unit.Dp = 0.dp,
    val bottom: androidx.compose.ui.unit.Dp = 0.dp,
    val left: androidx.compose.ui.unit.Dp = 0.dp,
    val right: androidx.compose.ui.unit.Dp = 0.dp,
)

// nexa-codegen: NexaWindowMetricsSnapshot
internal data class NexaWindowMetricsSnapshot(
    val viewportWidthDp: Float = 0f,
    val viewportHeightDp: Float = 0f,
    val safeInsets: NexaVisualSafeAreaSnapshot = NexaVisualSafeAreaSnapshot(),
    val keyboardTop: androidx.compose.ui.unit.Dp = 0.dp,
    val keyboardBottom: androidx.compose.ui.unit.Dp = 0.dp,
) {
    fun systemInset(
        edge: NexaSafeAreaEdge,
        direction: androidx.compose.ui.unit.LayoutDirection,
    ): androidx.compose.ui.unit.Dp = when (edge) {
        NexaSafeAreaEdge.Top -> safeInsets.top
        NexaSafeAreaEdge.Bottom -> safeInsets.bottom
        NexaSafeAreaEdge.Leading -> if (direction == androidx.compose.ui.unit.LayoutDirection.Ltr) safeInsets.left else safeInsets.right
        NexaSafeAreaEdge.Trailing -> if (direction == androidx.compose.ui.unit.LayoutDirection.Ltr) safeInsets.right else safeInsets.left
    }
}

// nexa-codegen: LocalNexaWindowMetrics
internal val LocalNexaWindowMetrics =
    androidx.compose.runtime.staticCompositionLocalOf { NexaWindowMetricsSnapshot() }

// nexa-codegen: NexaSafeAreaProvider
@androidx.compose.runtime.Composable
internal fun NexaSafeAreaProvider(
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val density = androidx.compose.ui.platform.LocalDensity.current
    val systemBars = androidx.compose.foundation.layout.WindowInsets.systemBars
    val displayCutout = androidx.compose.foundation.layout.WindowInsets.displayCutout
    val waterfall = androidx.compose.foundation.layout.WindowInsets.waterfall
    val visualWindowInsets = androidx.compose.runtime.remember(systemBars, displayCutout, waterfall) {
        systemBars.union(displayCutout).union(waterfall)
    }
    val pixelsPerDp = density.density
    val viewportSize = androidx.compose.ui.platform.LocalWindowInfo.current.containerSize
    val viewportWidthDp = viewportSize.width.toFloat() / pixelsPerDp
    val viewportHeightDp = viewportSize.height.toFloat() / pixelsPerDp
    val topPixels = visualWindowInsets.getTop(density)
    val bottomPixels = visualWindowInsets.getBottom(density)
    val leftPixels = visualWindowInsets.getLeft(density, androidx.compose.ui.unit.LayoutDirection.Ltr)
    val rightPixels = visualWindowInsets.getRight(density, androidx.compose.ui.unit.LayoutDirection.Ltr)
    val snapshot = androidx.compose.runtime.remember(
        topPixels,
        bottomPixels,
        leftPixels,
        rightPixels,
        pixelsPerDp,
        viewportWidthDp,
        viewportHeightDp,
    ) {
        NexaWindowMetricsSnapshot(
            viewportWidthDp = viewportWidthDp,
            viewportHeightDp = viewportHeightDp,
            safeInsets = NexaVisualSafeAreaSnapshot(
                top = androidx.compose.ui.unit.Dp(topPixels / pixelsPerDp),
                bottom = androidx.compose.ui.unit.Dp(bottomPixels / pixelsPerDp),
                left = androidx.compose.ui.unit.Dp(leftPixels / pixelsPerDp),
                right = androidx.compose.ui.unit.Dp(rightPixels / pixelsPerDp),
            ),
        )
    }
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNexaWindowMetrics provides snapshot,
        content = content,
    )
}

// nexa-codegen: nexaParentRelativeDimension
internal fun androidx.compose.ui.Modifier.nexaParentRelativeDimension(
    width: Float? = null,
    height: Float? = null,
    minWidth: Float? = null,
    maxWidth: Float? = null,
    minHeight: Float? = null,
    maxHeight: Float? = null,
): androidx.compose.ui.Modifier = this.layout { measurable, constraints ->
    val parentWidth = constraints.maxWidth.takeIf { constraints.hasBoundedWidth }
    val parentHeight = constraints.maxHeight.takeIf { constraints.hasBoundedHeight }
    val requestedWidth = width?.let { fraction -> parentWidth?.let { (it * fraction).toInt() } }
    val requestedHeight = height?.let { fraction -> parentHeight?.let { (it * fraction).toInt() } }
    val minimumWidth = minWidth?.let { fraction -> parentWidth?.let { (it * fraction).toInt() } }
    val maximumWidth = maxWidth?.let { fraction -> parentWidth?.let { (it * fraction).toInt() } }
    val minimumHeight = minHeight?.let { fraction -> parentHeight?.let { (it * fraction).toInt() } }
    val maximumHeight = maxHeight?.let { fraction -> parentHeight?.let { (it * fraction).toInt() } }
    val resolvedMinWidth = requestedWidth ?: maxOf(constraints.minWidth, minimumWidth ?: constraints.minWidth)
    val resolvedMaxWidth = requestedWidth ?: minOf(constraints.maxWidth, maximumWidth ?: constraints.maxWidth)
    val resolvedMinHeight = requestedHeight ?: maxOf(constraints.minHeight, minimumHeight ?: constraints.minHeight)
    val resolvedMaxHeight = requestedHeight ?: minOf(constraints.maxHeight, maximumHeight ?: constraints.maxHeight)
    val childConstraints = constraints.copy(
        minWidth = resolvedMinWidth.coerceAtMost(resolvedMaxWidth),
        maxWidth = resolvedMaxWidth.coerceAtLeast(resolvedMinWidth),
        minHeight = resolvedMinHeight.coerceAtMost(resolvedMaxHeight),
        maxHeight = resolvedMaxHeight.coerceAtLeast(resolvedMinHeight),
    )
    val placeable = measurable.measure(childConstraints)
    val measuredWidth = requestedWidth ?: placeable.width.coerceIn(childConstraints.minWidth, childConstraints.maxWidth)
    val measuredHeight = requestedHeight ?: placeable.height.coerceIn(childConstraints.minHeight, childConstraints.maxHeight)
    layout(measuredWidth, measuredHeight) { placeable.placeRelative(0, 0) }
}

// nexa-codegen: nexaSafeAreaDynamicInsets
@androidx.compose.runtime.Composable
internal fun nexaSafeAreaDynamicInsets(): NexaSafeAreaInsets {
    val consumed = LocalNexaSafeAreaContext.current.consumed
    val visual = LocalNexaWindowMetrics.current.safeInsets
    val direction = androidx.compose.ui.platform.LocalLayoutDirection.current
    val logical = NexaSafeAreaInsets(
        top = visual.top,
        bottom = visual.bottom,
        start = if (direction == androidx.compose.ui.unit.LayoutDirection.Ltr) visual.left else visual.right,
        end = if (direction == androidx.compose.ui.unit.LayoutDirection.Ltr) visual.right else visual.left,
    )
    return logical.subtractClamped(consumed)
}

// nexa-codegen: NexaViewInsetsPrimitive
internal val LocalNexaScrollContentPadding =
    androidx.compose.runtime.staticCompositionLocalOf<androidx.compose.foundation.layout.PaddingValues> {
        androidx.compose.foundation.layout.PaddingValues(androidx.compose.ui.unit.Dp(0f))
    }

// nexa-codegen: NexaScrollContentResetPrimitive
@androidx.compose.runtime.Composable
internal fun NexaScrollContentResetPrimitive(
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNexaScrollContentPadding provides androidx.compose.foundation.layout.PaddingValues(
            androidx.compose.ui.unit.Dp(0f),
        ),
        content = content,
    )
}

// nexa-codegen: NexaScrollContentPrimitive
@androidx.compose.runtime.Composable
internal fun NexaScrollContentPrimitive(
    contentPadding: androidx.compose.foundation.layout.PaddingValues =
        androidx.compose.foundation.layout.PaddingValues(androidx.compose.ui.unit.Dp(0f)),
    content: @androidx.compose.runtime.Composable (androidx.compose.foundation.layout.PaddingValues) -> Unit,
) {
    val inheritedPadding = LocalNexaScrollContentPadding.current
    val direction = androidx.compose.ui.platform.LocalLayoutDirection.current
    val effectivePadding = androidx.compose.foundation.layout.PaddingValues(
        start = inheritedPadding.calculateStartPadding(direction) + contentPadding.calculateStartPadding(direction),
        top = inheritedPadding.calculateTopPadding() + contentPadding.calculateTopPadding(),
        end = inheritedPadding.calculateEndPadding(direction) + contentPadding.calculateEndPadding(direction),
        bottom = inheritedPadding.calculateBottomPadding() + contentPadding.calculateBottomPadding(),
    )
    NexaScrollContentResetPrimitive {
        content(effectivePadding)
    }
}

@androidx.compose.runtime.Composable
internal fun NexaViewInsetsPrimitive(
    paddingTop: androidx.compose.ui.unit.Dp = 0.dp,
    paddingLeading: androidx.compose.ui.unit.Dp = 0.dp,
    paddingBottom: androidx.compose.ui.unit.Dp = 0.dp,
    paddingTrailing: androidx.compose.ui.unit.Dp = 0.dp,
    marginTop: androidx.compose.ui.unit.Dp = 0.dp,
    marginLeading: androidx.compose.ui.unit.Dp = 0.dp,
    marginBottom: androidx.compose.ui.unit.Dp = 0.dp,
    marginTrailing: androidx.compose.ui.unit.Dp = 0.dp,
    scrollContent: Boolean = false,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier.padding(
            start = marginLeading,
            top = marginTop,
            end = marginTrailing,
            bottom = marginBottom,
        ),
    ) {
        if (scrollContent) {
            androidx.compose.runtime.CompositionLocalProvider(
                LocalNexaScrollContentPadding provides androidx.compose.foundation.layout.PaddingValues(
                    start = paddingLeading,
                    top = paddingTop,
                    end = paddingTrailing,
                    bottom = paddingBottom,
                ),
                content = content,
            )
        } else {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.padding(
                    start = paddingLeading,
                    top = paddingTop,
                    end = paddingTrailing,
                    bottom = paddingBottom,
                ),
            ) { content() }
        }
    }
}

// nexa-codegen: NexaSafeAreaInsetPrimitive
@androidx.compose.runtime.Composable
internal fun NexaSafeAreaInsetPrimitive(
    edge: NexaSafeAreaEdge,
    accommodatesEndpoint: Boolean,
    content: @androidx.compose.runtime.Composable () -> Unit,
    attachedContent: @androidx.compose.runtime.Composable () -> Unit,
) {
    val parent = LocalNexaSafeAreaContext.current
    val safeInsets = nexaSafeAreaDynamicInsets()
    val vertical = edge == NexaSafeAreaEdge.Top || edge == NexaSafeAreaEdge.Bottom
    val occupiedBeforeAttachment = parent.occupied.get(edge)
    val attachmentSlot = 0
    val contentSlot = 1

    androidx.compose.ui.layout.SubcomposeLayout(
        modifier = androidx.compose.ui.Modifier.fillMaxSize(),
    ) { constraints ->
        val safe = safeInsets.get(edge)
        val crossSafe = if (vertical) {
            NexaSafeAreaInsets(start = safeInsets.start, end = safeInsets.end)
        } else {
            NexaSafeAreaInsets(top = safeInsets.top, bottom = safeInsets.bottom)
        }
        val attachmentOffset = with(this) {
            (occupiedBeforeAttachment + safe).roundToPx()
        }
        val safeContextForAttachment = parent.copy(
            consumed = parent.consumed.offset(edge, safe) + crossSafe,
            endpoint = NexaSafeAreaInsets(),
        )
        val maxWidth = constraints.maxWidth
        val maxHeight = constraints.maxHeight
        val attachmentConstraints = androidx.compose.ui.unit.Constraints(
            maxWidth = (if (vertical) maxWidth else maxWidth - attachmentOffset).coerceAtLeast(0),
            maxHeight = (if (vertical) maxHeight - attachmentOffset else maxHeight).coerceAtLeast(0),
        )
        val attachment = subcompose(attachmentSlot) {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.padding(
                    start = crossSafe.start,
                    top = crossSafe.top,
                    end = crossSafe.end,
                    bottom = crossSafe.bottom,
                ),
            ) {
                androidx.compose.runtime.CompositionLocalProvider(
                    LocalNexaSafeAreaContext provides safeContextForAttachment,
                    content = attachedContent,
                )
            }
        }.single().measure(attachmentConstraints)

        val attachmentSize = with(this) {
            (if (vertical) attachment.height else attachment.width).toDp()
        }
        val occupied = safe + attachmentSize
        val next = parent.copy(
            consumed = parent.consumed.offset(edge, safe),
            occupied = parent.occupied.offset(edge, occupied),
            endpoint = parent.endpoint.offset(
                edge,
                if (accommodatesEndpoint) occupied else 0.dp,
            ),
        )
        val width = if (constraints.hasBoundedWidth) maxWidth else maxOf(constraints.minWidth, attachment.width)
        val height = if (constraints.hasBoundedHeight) maxHeight else maxOf(constraints.minHeight, attachment.height)
        val body = subcompose(contentSlot) {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxSize(),
            ) {
                androidx.compose.runtime.CompositionLocalProvider(
                    LocalNexaSafeAreaContext provides next,
                    content = content,
                )
            }
        }.single().measure(androidx.compose.ui.unit.Constraints.fixed(width, height))

        layout(width, height) {
            body.place(0, 0)
            val x = when (edge) {
                NexaSafeAreaEdge.Leading -> attachmentOffset
                NexaSafeAreaEdge.Trailing -> width - attachmentOffset - attachment.width
                else -> (width - attachment.width) / 2
            }
            val y = when (edge) {
                NexaSafeAreaEdge.Top -> attachmentOffset
                NexaSafeAreaEdge.Bottom -> height - attachmentOffset - attachment.height
                else -> (height - attachment.height) / 2
            }
            attachment.placeRelative(x, y)
        }
    }
}

// nexa-codegen: nexaSafeAreaScrollViewportModifier
@androidx.compose.runtime.Composable
internal fun nexaSafeAreaScrollViewportModifier(
    axis: String,
): androidx.compose.ui.Modifier {
    val endpoint = LocalNexaSafeAreaContext.current.endpoint
    val vertical = axis == "Vertical" || axis == "Both"
    val horizontal = axis == "Horizontal" || axis == "Both"
    return androidx.compose.ui.Modifier.padding(
        androidx.compose.foundation.layout.PaddingValues(
            start = if (horizontal) endpoint.start else 0.dp,
            top = if (vertical) endpoint.top else 0.dp,
            end = if (horizontal) endpoint.end else 0.dp,
            bottom = if (vertical) endpoint.bottom else 0.dp,
        ),
    )
}

// nexa-codegen: NexaAdaptiveScrollHeaderPrimitive
private class NexaAdaptiveScrollHeaderState {
    val collapseOffset = androidx.compose.runtime.mutableFloatStateOf(0f)
    val isHidden = androidx.compose.runtime.mutableStateOf(false)
    val expandedHeight = androidx.compose.runtime.mutableIntStateOf(0)
    val hasInteracted = androidx.compose.runtime.mutableStateOf(false)
    var accumulatedMovement = 0f

    fun onVerticalScroll(behavior: String, delta: Float) {
        when (behavior) {
            "Collapse" -> collapseOffset.floatValue =
                (collapseOffset.floatValue - delta).coerceAtLeast(0f)
            "HideOnScroll" -> {
                if (delta == 0f) return
                if (accumulatedMovement * delta < 0f) accumulatedMovement = delta
                else accumulatedMovement += delta
                if (kotlin.math.abs(accumulatedMovement) >= 8f) {
                    // A negative nested-scroll delta means the user is moving
                    // down the content, so the header hides.
                    hasInteracted.value = true
                    isHidden.value = accumulatedMovement < 0f
                    accumulatedMovement = 0f
                }
            }
        }
    }
}

@androidx.compose.runtime.Composable
private fun NexaAdaptiveScrollHeaderContent(
    behavior: String,
    collapsedHeight: androidx.compose.ui.unit.Dp,
    state: NexaAdaptiveScrollHeaderState,
    header: @androidx.compose.runtime.Composable () -> Unit,
) {
    val density = androidx.compose.ui.platform.LocalDensity.current
    val expandedHeight = with(density) { state.expandedHeight.intValue.toDp() }
    val collapseOffset = with(density) { state.collapseOffset.floatValue.toDp() }
    val collapsed = minOf(collapsedHeight, expandedHeight.takeIf { it > 0.dp } ?: collapsedHeight)
    val collapseProgress = (collapseOffset / 120.dp).coerceIn(0f, 1f)
    val collapseHeight = expandedHeight - (expandedHeight - collapsed) * collapseProgress
    val hiddenTarget = if (state.isHidden.value) 0.dp else expandedHeight
    val animatedHiddenHeight = androidx.compose.animation.core.animateDpAsState(
        targetValue = hiddenTarget,
        animationSpec = if (state.hasInteracted.value) {
            androidx.compose.animation.core.tween(durationMillis = 180)
        } else {
            androidx.compose.animation.core.snap()
        },
    ).value
    val desiredHeight = when (behavior) {
        "Collapse" -> collapseHeight
        "HideOnScroll" -> animatedHiddenHeight
        else -> expandedHeight
    }

    androidx.compose.ui.layout.Layout(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .clipToBounds()
            .then(
                if (behavior == "HideOnScroll" && state.isHidden.value) {
                    androidx.compose.ui.Modifier.clearAndSetSemantics { }
                } else {
                    androidx.compose.ui.Modifier
                },
            ),
        content = {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier.onSizeChanged { size ->
                    if (state.expandedHeight.intValue != size.height) {
                        state.expandedHeight.intValue = size.height
                    }
                },
            ) {
                header()
            }
        },
    ) { measurables, constraints ->
        val placeable = measurables.single().measure(constraints.copy(minHeight = 0))
        val desiredLayoutHeight = if (behavior == "Pinned") {
            placeable.height
        } else {
            with(density) { desiredHeight.roundToPx() }.coerceIn(0, constraints.maxHeight)
        }
        val height = desiredLayoutHeight.coerceIn(constraints.minHeight, constraints.maxHeight)
        layout(placeable.width, height) {
            placeable.placeRelative(0, 0)
        }
    }
}

@androidx.compose.runtime.Composable
internal fun NexaAdaptiveScrollHeaderPrimitive(
    behavior: String,
    collapsedHeight: androidx.compose.ui.unit.Dp,
    header: @androidx.compose.runtime.Composable () -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val state = androidx.compose.runtime.remember { NexaAdaptiveScrollHeaderState() }
    val connection = androidx.compose.runtime.remember(state, behavior) {
        object : androidx.compose.ui.input.nestedscroll.NestedScrollConnection {
            override fun onPostScroll(
                consumed: androidx.compose.ui.geometry.Offset,
                available: androidx.compose.ui.geometry.Offset,
                source: androidx.compose.ui.input.nestedscroll.NestedScrollSource,
            ): androidx.compose.ui.geometry.Offset {
                state.onVerticalScroll(behavior, consumed.y)
                return androidx.compose.ui.geometry.Offset.Zero
            }
        }
    }
    val scrollModifier = if (behavior == "Pinned") {
        androidx.compose.ui.Modifier
    } else {
        androidx.compose.ui.Modifier.nestedScroll(connection)
    }
    androidx.compose.foundation.layout.Column(
        modifier = scrollModifier.fillMaxSize(),
    ) {
        NexaAdaptiveScrollHeaderContent(
            behavior = behavior,
            collapsedHeight = collapsedHeight,
            state = state,
            header = header,
        )
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier
                .weight(1f)
                .fillMaxWidth(),
        ) {
            content()
        }
    }
}

// nexa-codegen: NexaAppearancePrimitive
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
    val statusBarWindow = androidx.compose.runtime.remember(view) {
        var statusBarContext: android.content.Context = view.context
        while (statusBarContext is android.content.ContextWrapper && statusBarContext !is android.app.Activity) {
            statusBarContext = statusBarContext.baseContext
        }
        (statusBarContext as? android.app.Activity)?.window
    }
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
            NexaScrollContentPrimitive { contentPadding ->
                androidx.compose.foundation.layout.Column(
                    androidx.compose.ui.Modifier.fillMaxSize().verticalScroll(
                        androidx.compose.foundation.rememberScrollState(),
                    ).padding(contentPadding),
                ) {
                    content()
                }
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
    icon: NexaIconSpec?,
    style: String?,
    size: String?,
    shape: androidx.compose.ui.graphics.Shape?,
    tint: androidx.compose.ui.graphics.Color?,
    loading: Boolean,
    disabled: Boolean,
    iconOnlyCircle: Boolean,
    fullWidth: Boolean,
    leadingContent: Boolean,
    accessibilityLabel: String? = null,
    onClick: () -> Unit,
) {
    val useNativeActionStyle = style == null && LocalNexaDialogActionButtons.current
    val resolvedStyle = if (useNativeActionStyle) "Plain" else style ?: "BorderedProminent"
    val minHeight = if (size == "Large") androidx.compose.ui.unit.Dp(__NEXA_BUTTON_LARGE_MIN_HEIGHT__.toFloat()) else androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_TAP_TARGET__.toFloat())
    val modifier = when {
        fullWidth -> androidx.compose.ui.Modifier.fillMaxWidth().defaultMinSize(minWidth = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_WIDTH__.toFloat()), minHeight = minHeight)
        iconOnlyCircle -> androidx.compose.ui.Modifier.size(minHeight)
        else -> androidx.compose.ui.Modifier.defaultMinSize(minWidth = androidx.compose.ui.unit.Dp(__NEXA_BUTTON_MIN_WIDTH__.toFloat()), minHeight = minHeight)
    }
    val resolvedShape = shape ?: androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)
    val accessibleModifier = if (accessibilityLabel == null) modifier else modifier.semantics(mergeDescendants = true) {
        contentDescription = accessibilityLabel
    }
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
                NexaIconContent(icon = icon, description = "", size = androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat()))
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
            modifier = accessibleModifier,
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
                modifier = accessibleModifier,
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
            modifier = accessibleModifier,
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
    statusBarBackground: androidx.compose.ui.graphics.Color?,
    navigationBarBackground: androidx.compose.ui.graphics.Color?,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.foundation.layout.Box(
        modifier = androidx.compose.ui.Modifier.fillMaxSize(),
    ) {
        content()
        statusBarBackground?.let { background ->
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .align(androidx.compose.ui.Alignment.TopCenter)
                    .fillMaxWidth()
                    .windowInsetsTopHeight(androidx.compose.foundation.layout.WindowInsets.statusBars)
                    .background(background),
            )
        }
        navigationBarBackground?.let { background ->
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .align(androidx.compose.ui.Alignment.BottomCenter)
                    .fillMaxWidth()
                    .windowInsetsBottomHeight(androidx.compose.foundation.layout.WindowInsets.navigationBars)
                    .background(background),
            )
        }
    }
}

@androidx.compose.runtime.Composable
private fun nexaBottomSheetBackgroundColor(
    backgroundColor: androidx.compose.ui.graphics.Color?,
): androidx.compose.ui.graphics.Color = backgroundColor
    ?: if (androidx.compose.foundation.isSystemInDarkTheme()) {
        androidx.compose.ui.graphics.Color(0xFF1C1C1E)
    } else {
        androidx.compose.ui.graphics.Color.White
    }

@androidx.compose.runtime.Composable
@OptIn(
    androidx.compose.material3.ExperimentalMaterial3Api::class,
    androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class,
)
private fun nexaBottomSheetShape(
    cornerRadius: androidx.compose.ui.unit.Dp?,
): androidx.compose.ui.graphics.Shape = cornerRadius?.let {
    androidx.compose.runtime.remember(it) {
        androidx.compose.foundation.shape.RoundedCornerShape(topStart = it, topEnd = it)
    }
} ?: androidx.compose.material3.BottomSheetDefaults.ExpandedShape

// nexa-codegen: NexaBottomSheetContentPrimitive
@androidx.compose.runtime.Composable
private fun NexaBottomSheetContentPrimitive(
    modifier: androidx.compose.ui.Modifier,
    contentPadding: androidx.compose.ui.unit.Dp,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    // Column's shared defaults center children horizontally and start them at
    // the top of the available sheet body, just like Nexa layout elsewhere.
    NexaColumnPrimitive(
        modifier = modifier.fillMaxWidth().padding(contentPadding),
        content = content,
    )
}

private fun androidx.compose.ui.Modifier.nexaBottomSheetDragHandleModifier(): androidx.compose.ui.Modifier =
    layout { measurable, constraints ->
        val placeable = measurable.measure(constraints)
        // Material 3's native DragHandle adds 22dp above and below its 4dp
        // pill. SwiftUI overlays its handle on the sheet content, so keeping
        // that Android spacer pushes the first child much farther down. Keep
        // the native pill and semantics. Its 4dp height is shorter than the
        // SwiftUI handle's visible 5pt height, so align the lower edge with
        // SwiftUI by placing the pill 5.5dp from the sheet top. Nexa's shared
        // contentPadding continues to define the content position.
        val materialTopInset = androidx.compose.ui.unit.Dp(22f).roundToPx()
        val indicatorTop = androidx.compose.ui.unit.Dp(5.5f).roundToPx()
        layout(placeable.width, 0) {
            placeable.placeRelative(0, indicatorTop - materialTopInset)
        }
    }

// nexa-codegen: NexaBottomSheetPrimitive
@androidx.compose.runtime.Composable
@OptIn(
    androidx.compose.foundation.ExperimentalFoundationApi::class,
    androidx.compose.material3.ExperimentalMaterial3Api::class,
    androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class,
    androidx.compose.foundation.layout.ExperimentalLayoutApi::class,
)
internal fun NexaBottomSheetPrimitive(
    componentRef: NexaBottomSheetRef,
    contentSized: Boolean,
    contentPadding: androidx.compose.ui.unit.Dp,
    cornerRadius: androidx.compose.ui.unit.Dp?,
    detents: List<Float>,
    initialDetent: Float,
    showDragIndicator: Boolean,
    allowsDismissal: Boolean,
    allowsBackgroundInteraction: Boolean,
    shouldDismissOnBackPress: Boolean,
    backgroundColor: androidx.compose.ui.graphics.Color?,
    scrimColor: androidx.compose.ui.graphics.Color?,
    onDismiss: () -> Unit,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val isPresented = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableStateOf(false)
    }
    val onDismissRequest = { isPresented.value = false }
    androidx.compose.runtime.DisposableEffect(componentRef) {
        val generation = componentRef.attach(
            present = { isPresented.value = true },
            dismiss = { isPresented.value = false },
        )
        onDispose { componentRef.detach(generation) }
    }
    val wasPresented = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableStateOf(isPresented.value)
    }
    val retainedInComposition = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableStateOf(isPresented.value)
    }
    val currentOnDismiss by androidx.compose.runtime.rememberUpdatedState(onDismiss)
    val currentIsPresented by androidx.compose.runtime.rememberUpdatedState(isPresented.value)
    androidx.compose.runtime.LaunchedEffect(isPresented.value) {
        if (isPresented.value) {
            wasPresented.value = true
            retainedInComposition.value = true
        }
    }
    val onExitAnimationFinished = {
        if (!currentIsPresented && wasPresented.value) {
            wasPresented.value = false
            retainedInComposition.value = false
            currentOnDismiss()
        }
    }
    if (!retainedInComposition.value) return

    if (allowsBackgroundInteraction) {
        NexaNonModalBottomSheetPrimitive(
            isPresented = isPresented.value,
            contentSized = contentSized,
            contentPadding = contentPadding,
            cornerRadius = cornerRadius,
            detents = detents,
            initialDetent = initialDetent,
            showDragIndicator = showDragIndicator,
            allowsDismissal = allowsDismissal,
            shouldDismissOnBackPress = shouldDismissOnBackPress,
            backgroundColor = backgroundColor,
            onDismissRequest = onDismissRequest,
            onExitAnimationFinished = onExitAnimationFinished,
            content = content,
        )
        return
    }

    val nativeValues = detents.mapNotNull { detent ->
        when (detent) {
            0.5f -> androidx.compose.material3.SheetValue.PartiallyExpanded
            1f -> androidx.compose.material3.SheetValue.Expanded
            else -> null
        }
    }
    if (contentSized || nativeValues.size == detents.size) {
        val enabledValues = setOf(androidx.compose.material3.SheetValue.Hidden) +
            if (contentSized) {
                setOf(androidx.compose.material3.SheetValue.Expanded)
            } else {
                nativeValues
            }
        val nativeInitialDetent = if (contentSized || initialDetent == 1f) {
            androidx.compose.material3.SheetValue.Expanded
        } else {
            androidx.compose.material3.SheetValue.PartiallyExpanded
        }
        val dismissalEnabled by androidx.compose.runtime.rememberUpdatedState(allowsDismissal)
        val presentationRequested by androidx.compose.runtime.rememberUpdatedState(isPresented.value)
        val sheetState = androidx.compose.material3.rememberBottomSheetState(
            initialValue = androidx.compose.material3.SheetValue.Hidden,
            enabledValues = enabledValues,
            confirmValueChange = { value ->
                value != androidx.compose.material3.SheetValue.Hidden ||
                    dismissalEnabled || !presentationRequested
            },
        )
        androidx.compose.runtime.LaunchedEffect(isPresented.value, sheetState) {
            if (!isPresented.value) {
                sheetState.hide()
                if (!sheetState.isVisible) onExitAnimationFinished()
            }
        }
        androidx.compose.runtime.LaunchedEffect(sheetState, nativeInitialDetent, sheetState.currentValue) {
            if (
                nativeInitialDetent == androidx.compose.material3.SheetValue.Expanded &&
                androidx.compose.material3.SheetValue.PartiallyExpanded in enabledValues &&
                sheetState.currentValue != androidx.compose.material3.SheetValue.Hidden
            ) {
                sheetState.expand()
            }
        }
        val imeBottom = androidx.compose.foundation.layout.WindowInsets.ime
            .getBottom(androidx.compose.ui.platform.LocalDensity.current)
        val expanded = sheetState.currentValue == androidx.compose.material3.SheetValue.Expanded ||
            sheetState.targetValue == androidx.compose.material3.SheetValue.Expanded
        val keyboardOffset by androidx.compose.animation.core.animateIntAsState(
            targetValue = if (expanded) 0 else imeBottom,
            animationSpec = androidx.compose.animation.core.tween(
                durationMillis = 180,
                easing = androidx.compose.animation.core.FastOutSlowInEasing,
            ),
            label = "NexaBottomSheetKeyboardOffset",
        )
        androidx.compose.material3.ModalBottomSheet(
            modifier = androidx.compose.ui.Modifier.offset {
                androidx.compose.ui.unit.IntOffset(0, -keyboardOffset)
            },
            onDismissRequest = onDismissRequest,
            sheetState = sheetState,
            containerColor = nexaBottomSheetBackgroundColor(backgroundColor),
            shape = nexaBottomSheetShape(cornerRadius),
            scrimColor = scrimColor ?: androidx.compose.material3.BottomSheetDefaults.ScrimColor,
            dragHandle = if (showDragIndicator) {
                {
                    androidx.compose.material3.BottomSheetDefaults.DragHandle(
                        modifier = androidx.compose.ui.Modifier.nexaBottomSheetDragHandleModifier(),
                    )
                }
            } else {
                null
            },
            properties = androidx.compose.material3.ModalBottomSheetProperties(
                shouldDismissOnBackPress = allowsDismissal && shouldDismissOnBackPress,
                shouldDismissOnClickOutside = allowsDismissal,
            ),
        ) {
            androidx.compose.foundation.layout.Column(
                modifier = if (contentSized) {
                    androidx.compose.ui.Modifier.fillMaxWidth()
                } else {
                    androidx.compose.ui.Modifier.fillMaxWidth().fillMaxHeight()
                },
            ) {
                NexaBottomSheetContentPrimitive(
                    modifier = if (contentSized) {
                        androidx.compose.ui.Modifier.fillMaxWidth()
                    } else {
                        androidx.compose.ui.Modifier.fillMaxWidth().weight(1f)
                    },
                    contentPadding = contentPadding,
                    content = content,
                )
            }
        }
        return
    }

    NexaCustomBottomSheetPrimitive(
        isPresented = isPresented.value,
        cornerRadius = cornerRadius,
        contentPadding = contentPadding,
        detents = detents,
        initialDetent = initialDetent,
        showDragIndicator = showDragIndicator,
        allowsDismissal = allowsDismissal,
        shouldDismissOnBackPress = shouldDismissOnBackPress,
        backgroundColor = backgroundColor,
        scrimColor = scrimColor,
        onDismissRequest = onDismissRequest,
        onExitAnimationFinished = onExitAnimationFinished,
        content = content,
    )
}

@androidx.compose.runtime.Composable
@OptIn(
    androidx.compose.foundation.ExperimentalFoundationApi::class,
    androidx.compose.material3.ExperimentalMaterial3Api::class,
    androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class,
)
private fun NexaNonModalBottomSheetPrimitive(
    isPresented: Boolean,
    contentSized: Boolean,
    contentPadding: androidx.compose.ui.unit.Dp,
    cornerRadius: androidx.compose.ui.unit.Dp?,
    detents: List<Float>,
    initialDetent: Float,
    showDragIndicator: Boolean,
    allowsDismissal: Boolean,
    shouldDismissOnBackPress: Boolean,
    backgroundColor: androidx.compose.ui.graphics.Color?,
    onDismissRequest: () -> Unit,
    onExitAnimationFinished: () -> Unit,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val resolvedBackgroundColor = nexaBottomSheetBackgroundColor(backgroundColor)
    val currentOnDismissRequest by androidx.compose.runtime.rememberUpdatedState(onDismissRequest)
    val sortedDetents = androidx.compose.runtime.remember(detents) {
        detents.distinct().sorted()
    }
    val measuredContentHeightPx = androidx.compose.runtime.remember {
        androidx.compose.runtime.mutableIntStateOf(0)
    }
    val configuration = androidx.compose.ui.platform.LocalConfiguration.current
    val density = androidx.compose.ui.platform.LocalDensity.current
    val availableHeight = configuration.screenHeightDp.dp
    val availableWidth = configuration.screenWidthDp.dp
    val availableHeightPx = with(density) { availableHeight.toPx() }.coerceAtLeast(1f)
    val contentDetent = if (contentSized) {
        (measuredContentHeightPx.intValue / availableHeightPx).coerceIn(0f, 1f)
    } else {
        0f
    }
    val effectiveDetents = if (contentSized) listOf(contentDetent) else sortedDetents
    val minDetent = effectiveDetents.firstOrNull() ?: 0.5f
    val maxDetent = effectiveDetents.lastOrNull() ?: 1f
    val dismissOnBack = allowsDismissal && shouldDismissOnBackPress
    val initialHeight = (if (contentSized) contentDetent else initialDetent).coerceIn(
        if (allowsDismissal) 0f else minDetent,
        maxDetent,
    )
    val sheetHeight = androidx.compose.runtime.remember(
        sortedDetents,
        initialDetent,
        allowsDismissal,
    ) {
        androidx.compose.animation.core.Animatable(0f)
    }
    val coroutineScope = androidx.compose.runtime.rememberCoroutineScope()
    val currentOnExitAnimationFinished by androidx.compose.runtime.rememberUpdatedState(
        onExitAnimationFinished,
    )
    androidx.compose.runtime.LaunchedEffect(isPresented, sheetHeight, contentDetent) {
        if (isPresented) {
            val target = initialHeight.coerceIn(
                if (allowsDismissal) 0f else minDetent,
                maxDetent,
            )
            sheetHeight.animateTo(
                target,
                androidx.compose.animation.core.spring(),
            )
        } else {
            sheetHeight.animateTo(0f, androidx.compose.animation.core.spring())
            currentOnExitAnimationFinished()
        }
    }

    // A non-focusable Popup owns only the sheet's visible bounds. The rest of
    // the activity window remains interactive and has no dim or click barrier.
    androidx.activity.compose.BackHandler(enabled = true) {
        if (dismissOnBack) currentOnDismissRequest()
    }
    androidx.compose.ui.window.Popup(
        popupPositionProvider = androidx.compose.runtime.remember {
            object : androidx.compose.ui.window.PopupPositionProvider {
                override fun calculatePosition(
                    anchorBounds: androidx.compose.ui.unit.IntRect,
                    windowSize: androidx.compose.ui.unit.IntSize,
                    layoutDirection: androidx.compose.ui.unit.LayoutDirection,
                    popupContentSize: androidx.compose.ui.unit.IntSize,
                ): androidx.compose.ui.unit.IntOffset = androidx.compose.ui.unit.IntOffset(
                    x = (windowSize.width - popupContentSize.width) / 2,
                    y = windowSize.height - popupContentSize.height,
                )
            }
        },
        onDismissRequest = {},
        properties = androidx.compose.ui.window.PopupProperties(
            focusable = false,
            dismissOnBackPress = false,
            dismissOnClickOutside = false,
            clippingEnabled = false,
        ),
    ) {
        androidx.compose.material3.Surface(
            modifier = androidx.compose.ui.Modifier
                .width(availableWidth)
                .then(
                    if (contentSized) {
                        androidx.compose.ui.Modifier.heightIn(max = availableHeight)
                    } else {
                        androidx.compose.ui.Modifier.height(availableHeight * sheetHeight.value)
                    },
                )
                .onSizeChanged { size ->
                    if (contentSized && measuredContentHeightPx.intValue != size.height) {
                        measuredContentHeightPx.intValue = size.height
                    }
                }
                .offset {
                    val presentationProgress = if (contentSized && maxDetent > 0f) {
                        sheetHeight.value / maxDetent
                    } else {
                        sheetHeight.value
                    }
                    androidx.compose.ui.unit.IntOffset(
                        0,
                        if (contentSized) {
                            (measuredContentHeightPx.intValue * (1f - presentationProgress)).toInt()
                        } else {
                            0
                        },
                    )
                }
                .pointerInput(sheetHeight, effectiveDetents, availableHeightPx, allowsDismissal) {
                    detectVerticalDragGestures(
                        onVerticalDrag = { _, dragAmount ->
                            coroutineScope.launch {
                                val floor = if (allowsDismissal) 0f else minDetent
                                sheetHeight.snapTo(
                                    (sheetHeight.value - dragAmount / availableHeightPx)
                                        .coerceIn(floor, maxDetent),
                                )
                            }
                        },
                        onDragEnd = {
                            coroutineScope.launch {
                                val anchors = if (allowsDismissal) {
                                    listOf(0f) + effectiveDetents
                                } else {
                                    effectiveDetents
                                }
                                val target = anchors.minByOrNull {
                                    kotlin.math.abs(it - sheetHeight.value)
                                } ?: minDetent
                                if (target == 0f) {
                                    currentOnDismissRequest()
                                } else {
                                    sheetHeight.animateTo(
                                        target,
                                        androidx.compose.animation.core.spring(),
                                    )
                                }
                            }
                        },
                        onDragCancel = {
                            coroutineScope.launch {
                                val target = effectiveDetents.minByOrNull {
                                    kotlin.math.abs(it - sheetHeight.value)
                                } ?: minDetent
                                sheetHeight.animateTo(
                                    target,
                                    androidx.compose.animation.core.spring(),
                                )
                            }
                        },
                    )
                },
            shape = nexaBottomSheetShape(cornerRadius),
            color = resolvedBackgroundColor,
            contentColor = androidx.compose.material3.contentColorFor(
                resolvedBackgroundColor,
            ),
            tonalElevation = 0.dp,
        ) {
            androidx.compose.foundation.layout.Column(
                modifier = if (contentSized) {
                    androidx.compose.ui.Modifier.fillMaxWidth()
                } else {
                    androidx.compose.ui.Modifier.fillMaxSize()
                },
            ) {
                if (showDragIndicator) {
                    androidx.compose.material3.BottomSheetDefaults.DragHandle(
                        modifier = androidx.compose.ui.Modifier
                            .align(androidx.compose.ui.Alignment.CenterHorizontally)
                            .nexaBottomSheetDragHandleModifier(),
                    )
                }
                NexaBottomSheetContentPrimitive(
                    modifier = if (contentSized) {
                        androidx.compose.ui.Modifier.fillMaxWidth()
                    } else {
                        androidx.compose.ui.Modifier.fillMaxWidth().weight(1f)
                    },
                    contentPadding = contentPadding,
                    content = content,
                )
            }
        }
    }
}

@androidx.compose.runtime.Composable
@OptIn(
    androidx.compose.foundation.ExperimentalFoundationApi::class,
    androidx.compose.material3.ExperimentalMaterial3Api::class,
    androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class,
)
private fun NexaCustomBottomSheetPrimitive(
    isPresented: Boolean,
    contentPadding: androidx.compose.ui.unit.Dp,
    cornerRadius: androidx.compose.ui.unit.Dp?,
    detents: List<Float>,
    initialDetent: Float,
    showDragIndicator: Boolean,
    allowsDismissal: Boolean,
    shouldDismissOnBackPress: Boolean,
    backgroundColor: androidx.compose.ui.graphics.Color?,
    scrimColor: androidx.compose.ui.graphics.Color?,
    onDismissRequest: () -> Unit,
    onExitAnimationFinished: () -> Unit,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val resolvedBackgroundColor = nexaBottomSheetBackgroundColor(backgroundColor)
    val currentOnDismissRequest by androidx.compose.runtime.rememberUpdatedState(onDismissRequest)
    val maxDetent = detents.last()
    val sheetState = androidx.compose.runtime.remember(detents, initialDetent, allowsDismissal) {
        androidx.compose.foundation.gestures.AnchoredDraggableState(
            // Start at the hidden anchor. The presentation effect below animates
            // to the configured detent after the Dialog has measured its anchors.
            initialValue = 0f,
        )
    }
    val coroutineScope = androidx.compose.runtime.rememberCoroutineScope()
    val currentOnExitAnimationFinished by androidx.compose.runtime.rememberUpdatedState(
        onExitAnimationFinished,
    )
    androidx.compose.runtime.LaunchedEffect(isPresented, sheetState) {
        if (!isPresented) {
            sheetState.animateTo(0f)
            currentOnExitAnimationFinished()
        } else {
            sheetState.animateTo(initialDetent)
        }
    }
    val lastReportedSheetValue = androidx.compose.runtime.remember(sheetState) {
        androidx.compose.runtime.mutableFloatStateOf(sheetState.currentValue)
    }
    androidx.compose.runtime.LaunchedEffect(sheetState.currentValue) {
        val currentValue = sheetState.currentValue
        if (currentValue == lastReportedSheetValue.floatValue) return@LaunchedEffect
        lastReportedSheetValue.floatValue = currentValue
        if (currentValue == 0f) {
            currentOnDismissRequest()
        }
    }

    androidx.compose.ui.window.Dialog(
        onDismissRequest = {
            if (allowsDismissal) {
                coroutineScope.launch { sheetState.animateTo(0f) }
            }
        },
        properties = androidx.compose.ui.window.DialogProperties(
            dismissOnBackPress = allowsDismissal && shouldDismissOnBackPress,
            dismissOnClickOutside = false,
            usePlatformDefaultWidth = false,
            decorFitsSystemWindows = false,
        ),
    ) {
        val dialogView = androidx.compose.ui.platform.LocalView.current
        androidx.compose.runtime.SideEffect {
            (dialogView.parent as? androidx.compose.ui.window.DialogWindowProvider)
                ?.window
                ?.setDimAmount(0f)
        }
        androidx.compose.foundation.layout.BoxWithConstraints(
            modifier = androidx.compose.ui.Modifier
                .fillMaxSize()
                .imePadding(),
        ) {
            val availableHeightPx = constraints.maxHeight.toFloat()
            val maxHeight = maxHeight * maxDetent
            val anchors = androidx.compose.foundation.gestures.DraggableAnchors<Float> {
                detents.forEach { point ->
                    point at (availableHeightPx * (maxDetent - point))
                }
                0f at (availableHeightPx * maxDetent)
            }
            androidx.compose.runtime.SideEffect {
                sheetState.updateAnchors(anchors)
            }
            val dismiss = {
                if (allowsDismissal) {
                    coroutineScope.launch { sheetState.animateTo(0f) }
                }
            }
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxSize()
                .background(
                    scrimColor ?: androidx.compose.material3.BottomSheetDefaults.ScrimColor,
                )
                    .clickable(
                        enabled = allowsDismissal,
                        interactionSource = androidx.compose.runtime.remember {
                            androidx.compose.foundation.interaction.MutableInteractionSource()
                        },
                        indication = null,
                        onClick = dismiss,
                    ),
            )
            androidx.compose.material3.Surface(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxWidth()
                    .height(maxHeight)
                    .align(androidx.compose.ui.Alignment.BottomCenter)
                    .offset {
                        androidx.compose.ui.unit.IntOffset(
                            x = 0,
                            y = if (sheetState.offset.isNaN()) {
                                (availableHeightPx * (maxDetent - sheetState.currentValue)).toInt()
                            } else {
                                sheetState.requireOffset().toInt()
                            },
                        )
                    }
                    .anchoredDraggable(
                        state = sheetState,
                        orientation = androidx.compose.foundation.gestures.Orientation.Vertical,
                        enabled = allowsDismissal,
                    ),
                shape = nexaBottomSheetShape(cornerRadius),
                color = resolvedBackgroundColor,
                contentColor = androidx.compose.material3.contentColorFor(
                    resolvedBackgroundColor,
                ),
                tonalElevation = 0.dp,
            ) {
                androidx.compose.foundation.layout.Column(
                    modifier = androidx.compose.ui.Modifier
                        .fillMaxWidth()
                        .fillMaxHeight(),
                ) {
                    if (showDragIndicator) {
                        androidx.compose.material3.BottomSheetDefaults.DragHandle(
                            modifier = androidx.compose.ui.Modifier
                                .align(androidx.compose.ui.Alignment.CenterHorizontally)
                                .nexaBottomSheetDragHandleModifier(),
                        )
                    }
                    NexaBottomSheetContentPrimitive(
                        modifier = androidx.compose.ui.Modifier
                            .fillMaxWidth()
                            .weight(1f),
                        contentPadding = contentPadding,
                        content = content,
                    )
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
                LocalNexaNavigationTitleSlot provides navigationTitleSlot,
            ) {
                content()
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
    avoidIme: Boolean,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val modifier = nexaKeyboardAwareModifier(interactiveDismiss, avoidIme)
        .then(nexaSafeAreaScrollViewportModifier("Vertical"))
        .verticalScroll(androidx.compose.foundation.rememberScrollState())
    NexaScrollContentPrimitive { contentPadding ->
        androidx.compose.foundation.layout.Column(
            modifier = modifier.padding(contentPadding),
            content = content,
        )
    }
}

// nexa-codegen: NexaKeyboardAwareOwnerPrimitive
@androidx.compose.runtime.Composable
@kotlin.OptIn(
    androidx.compose.foundation.layout.ExperimentalLayoutApi::class,
    androidx.compose.foundation.ExperimentalFoundationApi::class,
)
internal fun NexaKeyboardAwareOwnerPrimitive(
    interactiveDismiss: Boolean,
    avoidIme: Boolean,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.foundation.layout.Box(
        modifier = nexaKeyboardAwareModifier(interactiveDismiss, avoidIme),
    ) {
        content()
    }
}

// nexa-codegen: nexaKeyboardAwareModifier
@androidx.compose.runtime.Composable
@kotlin.OptIn(
    androidx.compose.foundation.layout.ExperimentalLayoutApi::class,
    androidx.compose.foundation.ExperimentalFoundationApi::class,
)
private fun nexaKeyboardAwareModifier(
    interactiveDismiss: Boolean,
    avoidIme: Boolean,
): androidx.compose.ui.Modifier {
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
    }
    if (avoidIme) modifier = modifier.imePadding()
    // imeNestedScroll is only meaningful while the IME is already visible.
    // Keeping it attached to every hidden-IME scroll can issue a show request
    // on Android when the scroll reaches its edge, even if no field is focused.
    if (interactiveDismiss && imeVisible.value) modifier = modifier.imeNestedScroll()
    return modifier
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
    textColor: androidx.compose.ui.graphics.Color? = null,
    placeholderColor: androidx.compose.ui.graphics.Color? = null,
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
                        color = textColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
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
                                        color = placeholderColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
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
                color = textColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface,
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
                                color = placeholderColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
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
    minValue: Float,
    maxValue: Float,
    step: Float,
    tint: androidx.compose.ui.graphics.Color? = null,
    trackColor: androidx.compose.ui.graphics.Color? = null,
    thumbColor: androidx.compose.ui.graphics.Color? = null,
    trackHeight: Float? = null,
    thumbSize: Float? = null,
    hitSlop: Float? = null,
    onEditingChanged: ((Boolean) -> Unit)? = null,
) {
    val thumbDp = (thumbSize ?: 14f).coerceAtLeast(1f)
    val trackDp = (trackHeight ?: 4f).coerceAtLeast(1f)
    val hitSlopDp = (hitSlop ?: 0f).coerceAtLeast(0f)
    val rangeIsValid = minValue.isFinite() && maxValue.isFinite() && maxValue > minValue
    val lowerBound = if (minValue.isFinite()) minValue else 0f
    val upperBound = if (maxValue.isFinite() && maxValue > lowerBound) maxValue else lowerBound + 1f
    val editing = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }
    val animatedValue = androidx.compose.animation.core.animateFloatAsState(
        targetValue = value,
        animationSpec = androidx.compose.animation.core.tween(
            durationMillis = if (editing.value) 0 else 250,
            easing = androidx.compose.animation.core.LinearEasing,
        ),
        label = "Nexa slider value",
    ).value
    val inactive = trackColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface.copy(alpha = 0.28f)
    val active = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary
    val thumb = thumbColor ?: androidx.compose.ui.graphics.Color.White
    val trackClipPath = androidx.compose.runtime.remember { androidx.compose.ui.graphics.Path() }
    androidx.compose.ui.layout.Layout(
        modifier = androidx.compose.ui.Modifier.fillMaxWidth(),
        content = {
            androidx.compose.foundation.layout.Box(
                modifier = androidx.compose.ui.Modifier
                    .fillMaxWidth()
                    .height((thumbDp + hitSlopDp * 2f).dp)
                    .pointerInput(lowerBound, upperBound, step, rangeIsValid) {
                        if (rangeIsValid) awaitEachGesture {
                            val down = awaitFirstDown(requireUnconsumed = false)
                            val width = size.width.toFloat().coerceAtLeast(1f)
                            fun update(position: Float) {
                                val fraction = (position / width).coerceIn(0f, 1f)
                                val raw = lowerBound + fraction * (upperBound - lowerBound)
                                val quantized = if (step > 0f) {
                                    lowerBound + kotlin.math.round((raw - lowerBound) / step) * step
                                } else raw
                                onValueChange(quantized.coerceIn(lowerBound, upperBound))
                            }
                            var dragging = false
                            var verticalScroll = false
                            var lastX = down.position.x
                            while (true) {
                                val event = awaitPointerEvent()
                                val change = event.changes.firstOrNull { it.id == down.id } ?: break
                                if (!change.pressed) {
                                    break
                                }
                                lastX = change.position.x
                                val horizontal = kotlin.math.abs(change.position.x - down.position.x)
                                val vertical = kotlin.math.abs(change.position.y - down.position.y)
                                if (!dragging && horizontal > viewConfiguration.touchSlop && horizontal > vertical) {
                                    dragging = true
                                    editing.value = true
                                    onEditingChanged?.invoke(true)
                                } else if (!dragging && vertical > viewConfiguration.touchSlop) {
                                    verticalScroll = true
                                    break
                                }
                                if (dragging) {
                                    update(lastX)
                                    change.consume()
                                }
                            }
                            if (dragging) {
                                update(lastX)
                                editing.value = false
                                onEditingChanged?.invoke(false)
                            } else if (!verticalScroll) {
                                update(down.position.x)
                                down.consume()
                            }
                        }
                    },
                contentAlignment = androidx.compose.ui.Alignment.CenterStart,
            ) {
                androidx.compose.foundation.Canvas(androidx.compose.ui.Modifier.fillMaxSize()) {
                    val thumbPx = thumbDp.dp.toPx()
                    val trackPx = trackDp.dp.toPx()
                    val fraction = if (upperBound > lowerBound) {
                        ((animatedValue - lowerBound) / (upperBound - lowerBound)).coerceIn(0f, 1f)
                    } else 0f
                    val centerY = size.height / 2f
                    val trackTop = centerY - trackPx / 2f
                    val corner = androidx.compose.ui.geometry.CornerRadius(trackPx / 2f)
                    drawRoundRect(
                        color = inactive,
                        topLeft = androidx.compose.ui.geometry.Offset(0f, trackTop),
                        size = androidx.compose.ui.geometry.Size(size.width, trackPx),
                        cornerRadius = corner,
                    )
                    trackClipPath.reset()
                    trackClipPath.addRoundRect(
                        androidx.compose.ui.geometry.RoundRect(
                            androidx.compose.ui.geometry.Rect(
                                0f,
                                trackTop,
                                size.width,
                                trackTop + trackPx,
                            ),
                            corner,
                        ),
                    )
                    clipPath(trackClipPath) {
                        drawRect(
                            color = active,
                            topLeft = androidx.compose.ui.geometry.Offset(0f, trackTop),
                            size = androidx.compose.ui.geometry.Size(size.width * fraction, trackPx),
                        )
                    }
                    drawCircle(
                        color = thumb,
                        radius = thumbPx / 2f,
                        center = androidx.compose.ui.geometry.Offset(size.width * fraction, centerY),
                    )
                }
            }
        },
    ) { measurables, constraints ->
        val visualHeight = thumbDp.dp.roundToPx().coerceAtMost(constraints.maxHeight)
        val slopPx = hitSlopDp.dp.roundToPx().coerceAtMost((constraints.maxHeight - visualHeight) / 2)
        val hitHeight = visualHeight + slopPx * 2
        val placeable = measurables.first().measure(
            androidx.compose.ui.unit.Constraints.fixed(constraints.maxWidth, hitHeight),
        )
        layout(placeable.width, visualHeight) {
            placeable.placeRelative(0, -slopPx)
        }
    }
}

// nexa-codegen: NexaProgressBarPrimitive
@androidx.compose.runtime.Composable
internal fun NexaProgressBarPrimitive(
    progress: Float?,
    tint: androidx.compose.ui.graphics.Color? = null,
    trackColor: androidx.compose.ui.graphics.Color? = null,
    thickness: Float? = null,
    cornerRadius: Float? = null,
    __NEXA_ANDROID_WAVE_PARAMETER__
) {
    val normalizedProgress = progress?.let {
        if (it.isFinite()) it.coerceIn(0f, 1f) else 0f
    }
    var modifier = androidx.compose.ui.Modifier
        .fillMaxWidth()
    thickness?.let { modifier = modifier.height(it.dp) }
    (cornerRadius ?: thickness?.div(2f))?.let {
        modifier = modifier.clip(RoundedCornerShape(it.dp))
    }
    modifier = if (normalizedProgress == null) {
        modifier.progressSemantics()
    } else {
        modifier.progressSemantics(normalizedProgress)
    }
    val indicatorColor = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary
    val indicatorTrackColor = trackColor
        ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface.copy(alpha = 0.1f)
    if (normalizedProgress == null) {
        __NEXA_ANDROID_WAVE_INDETERMINATE_INDICATOR__
    } else if (__NEXA_ANDROID_SQUARE_PROGRESS_CANVAS_CONDITION__) {
        val animatedProgress = androidx.compose.animation.core.animateFloatAsState(
            targetValue = normalizedProgress,
            animationSpec = androidx.compose.animation.core.tween(
                durationMillis = 250,
                easing = androidx.compose.animation.core.LinearEasing,
            ),
            label = "progressBar",
        )
        androidx.compose.foundation.Canvas(modifier = modifier) {
            drawRect(color = indicatorTrackColor)
            val fillWidth = size.width * animatedProgress.value
            if (fillWidth > 0f) {
                drawRect(
                    color = indicatorColor,
                    size = androidx.compose.ui.geometry.Size(fillWidth, size.height),
                )
            }
        }
    } else {
        __NEXA_ANDROID_WAVE_DETERMINATE_INDICATOR__
    }
}

// nexa-codegen: NexaProgressRingPrimitive
__NEXA_ANDROID_WAVE_RING_OPT_IN__@androidx.compose.runtime.Composable
internal fun NexaProgressRingPrimitive(
    progress: Float?,
    tint: androidx.compose.ui.graphics.Color? = null,
    trackColor: androidx.compose.ui.graphics.Color? = null,
    size: Float? = null,
    thickness: Float? = null,
    __NEXA_ANDROID_WAVE_RING_PARAMETER__
) {
    val normalizedProgress = progress?.let { if (it.isFinite()) it.coerceIn(0f, 1f) else 0f }
    val indicatorColor = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary
    val indicatorSize = (size ?: 40f).coerceAtLeast(0f)
    val indicatorThickness = (thickness ?: 4f).coerceIn(0f, indicatorSize)
    val modifier = androidx.compose.ui.Modifier.size(indicatorSize.dp)
    val indicatorTrackColor = trackColor ?: indicatorColor.copy(alpha = 0.16f)
    val semanticsModifier = if (normalizedProgress == null) {
        modifier.progressSemantics()
    } else {
        modifier.progressSemantics(normalizedProgress)
    }
    __NEXA_ANDROID_WAVE_RING_INDICATOR__
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

// nexa-codegen: NexaFontIconPrimitive
@androidx.compose.runtime.Composable
internal fun NexaFontIconPrimitive(
    glyph: Int,
    description: String,
    size: androidx.compose.ui.unit.Dp,
    tint: androidx.compose.ui.graphics.Color,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    fontResource: Int? = null,
    fontFamily: androidx.compose.ui.text.font.FontFamily? = null,
) {
    val resolvedFontFamily = fontFamily ?: androidx.compose.runtime.remember(fontResource) {
        fontResource?.let { resource ->
            androidx.compose.ui.text.font.FontFamily(
                androidx.compose.ui.text.font.Font(resource),
            )
        } ?: androidx.compose.ui.text.font.FontFamily.Default
    }
    val accessibilityModifier = if (description.isEmpty()) {
        androidx.compose.ui.Modifier.clearAndSetSemantics { }
    } else {
        androidx.compose.ui.Modifier.semantics { contentDescription = description }
    }
    val glyphText = androidx.compose.runtime.remember(glyph) {
        if (java.lang.Character.isValidCodePoint(glyph)) {
            String(java.lang.Character.toChars(glyph))
        } else {
            "�"
        }
    }
    androidx.compose.material3.Text(
        text = glyphText,
        modifier = modifier.then(accessibilityModifier),
        color = tint,
        // Match fixed-size SwiftUI icon glyphs without applying text scaling.
        fontSize = (size.value / androidx.compose.ui.platform.LocalDensity.current.fontScale).sp,
        fontFamily = resolvedFontFamily,
        maxLines = 1,
        softWrap = false,
    )
}

// nexa-codegen: NexaIconSpec
internal data class NexaIconSpec(
    val vector: androidx.compose.ui.graphics.vector.ImageVector? = null,
    val glyph: Int? = null,
    val fontResource: Int? = null,
    val fontFamily: androidx.compose.ui.text.font.FontFamily? = null,
)

// nexa-codegen: NexaIconContent
@androidx.compose.runtime.Composable
internal fun NexaIconContent(
    icon: NexaIconSpec,
    description: String,
    size: androidx.compose.ui.unit.Dp,
    tint: androidx.compose.ui.graphics.Color? = null,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
) {
    val resolvedTint = tint ?: androidx.compose.material3.LocalContentColor.current
    val glyph = icon.glyph
    val fontResource = icon.fontResource
    val fontFamily = icon.fontFamily
    if (glyph != null && (fontResource != null || fontFamily != null)) {
        NexaFontIconPrimitive(
            glyph = glyph,
            fontResource = fontResource,
            fontFamily = fontFamily,
            description = description,
            size = size,
            tint = resolvedTint,
            modifier = modifier,
        )
    } else {
        icon.vector?.let { vector ->
            NexaSystemIconPrimitive(
                image = vector,
                description = description,
                size = size,
                tint = resolvedTint,
                modifier = modifier,
            )
        }
    }
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
    icon: NexaIconSpec,
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
        NexaIconContent(
            icon = icon,
            description = iconDescription,
            size = androidx.compose.ui.unit.Dp(58f),
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
    preserveChildWidths: Boolean = false,
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
    val rowModifier = when {
        horizontalArrangement != null -> modifier.fillMaxWidth()
        preserveChildWidths -> modifier.wrapContentWidth(Alignment.Start, unbounded = true)
        else -> modifier
    }
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
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    contentPadding: androidx.compose.foundation.layout.PaddingValues =
        androidx.compose.foundation.layout.PaddingValues(androidx.compose.ui.unit.Dp(0f)),
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    val scrollState = androidx.compose.foundation.rememberScrollState()
    NexaScrollContentPrimitive(contentPadding = contentPadding) { effectiveContentPadding ->
        androidx.compose.foundation.layout.Column(
            modifier = modifier
                .fillMaxSize()
                .background(background ?: androidx.compose.ui.graphics.Color.Transparent)
                .verticalScroll(scrollState, enabled = scrollState.maxValue > 0)
                .padding(effectiveContentPadding),
        ) {
            content()
        }
    }
}

// nexa-codegen: NexaFormSectionPrimitive
@androidx.compose.runtime.Composable
internal fun NexaFormSectionPrimitive(
    title: String?,
    footer: String?,
    background: androidx.compose.ui.graphics.Color? = null,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    androidx.compose.foundation.layout.Column(
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .padding(
                start = androidx.compose.ui.unit.Dp(__NEXA_FORM_SECTION_HORIZONTAL_INSET__.toFloat()),
                end = androidx.compose.ui.unit.Dp(__NEXA_FORM_SECTION_HORIZONTAL_INSET__.toFloat()),
                bottom = androidx.compose.ui.unit.Dp(
                    if (footer == null) {
                        __NEXA_FORM_SECTION_BOTTOM_PADDING__.toFloat()
                    } else {
                        8.toFloat()
                    },
                ),
            ),
    ) {
        if (title != null) {
            androidx.compose.material3.Text(
                title,
                modifier = androidx.compose.ui.Modifier.padding(
                    start = androidx.compose.ui.unit.Dp(__NEXA_FORM_SECTION_HEADER_INSET__.toFloat()),
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

// nexa-codegen: LocalNexaDialogActionButtons
internal val LocalNexaDialogActionButtons = androidx.compose.runtime.compositionLocalOf { false }

// nexa-codegen: NexaPickerPrimitive
@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class, androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class)
internal fun NexaPickerPrimitive(
    items: List<String>,
    itemIcons: List<NexaIconSpec?>,
    selected: String,
    icon: NexaIconSpec?,
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
            NexaIconContent(
                icon = icon,
                description = "",
                size = androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat()),
                tint = resolvedTint,
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
                    NexaIconContent(
                        icon = icon,
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
                                        NexaIconContent(
                                            icon = optionIcon,
                                            description = "",
                                            size = androidx.compose.ui.unit.Dp(__NEXA_DEFAULT_MATERIAL_ICON_SIZE__.toFloat()),
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
            .size(androidx.compose.ui.unit.Dp(48f))
            .semantics {
                role = androidx.compose.ui.semantics.Role.Button
                contentDescription = "Page ${index + 1}"
                onClick(label = "Page ${index + 1}") {
                    onSelect()
                    true
                }
            },
        contentAlignment = androidx.compose.ui.Alignment.Center,
    ) {
        androidx.compose.foundation.layout.Box(
            modifier = androidx.compose.ui.Modifier
                .size(size)
                .background(color, shape = androidx.compose.foundation.shape.CircleShape),
        )
    }
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

// nexa-codegen: NexaPageIndicatorRow
private data class NexaPageIndicatorGeometry(
    val visualWidth: Int,
    val dotCenters: IntArray,
)

private fun nexaPageIndicatorGeometry(
    availableWidth: Int,
    pageCount: Int,
    currentPage: Int,
    spacing: Int,
    selectedSize: Int,
    unselectedSize: Int,
): NexaPageIndicatorGeometry {
    val count = pageCount.coerceAtLeast(0)
    val selectedDelta = if (currentPage in 0 until count) selectedSize - unselectedSize else 0
    val visualWidth = count * unselectedSize + (count - 1).coerceAtLeast(0) * spacing + selectedDelta
    val dotCenters = IntArray(count)
    var dotLeft = (availableWidth - visualWidth) / 2
    repeat(count) { index ->
        val dotSize = if (index == currentPage) selectedSize else unselectedSize
        dotCenters[index] = dotLeft + dotSize / 2
        dotLeft += dotSize + spacing
    }
    return NexaPageIndicatorGeometry(visualWidth, dotCenters)
}

@androidx.compose.runtime.Composable
internal fun NexaPageIndicatorRow(
    pageCount: Int,
    currentPage: Int,
    onSelect: (Int) -> Unit,
) {
    val selectedPage = androidx.compose.runtime.rememberUpdatedState(currentPage)
    val selectPage = androidx.compose.runtime.rememberUpdatedState(onSelect)
    androidx.compose.ui.layout.Layout(
        content = {
            repeat(pageCount) { index ->
                NexaPageIndicatorPrimitive(index = index, selected = currentPage == index) {
                    onSelect(index)
                }
            }
        },
        modifier = androidx.compose.ui.Modifier
            .fillMaxWidth()
            .height(androidx.compose.ui.unit.Dp(32f))
            // The page targets overlap; resolve coordinate taps by visible dot center.
            .pointerInput(pageCount) {
                detectTapGestures { tap ->
                    if (pageCount > 0) {
                        val spacing = androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_SPACING__.toFloat()).roundToPx()
                        val selectedSize = androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_SELECTED_SIZE__.toFloat()).roundToPx()
                        val unselectedSize = androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__.toFloat()).roundToPx()
                        val current = selectedPage.value
                        val geometry = nexaPageIndicatorGeometry(
                            size.width,
                            pageCount,
                            current,
                            spacing,
                            selectedSize,
                            unselectedSize,
                        )
                        var nearestPage = 0
                        var nearestDistance = Float.POSITIVE_INFINITY
                        geometry.dotCenters.forEachIndexed { index, center ->
                            val distance = kotlin.math.abs(tap.x - center)
                            if (distance < nearestDistance) {
                                nearestPage = index
                                nearestDistance = distance
                            }
                        }
                        if (nearestDistance <= androidx.compose.ui.unit.Dp(24f).toPx()) {
                            selectPage.value(nearestPage)
                        }
                    }
                }
        },
    ) { measurables, constraints ->
        val targetSize = androidx.compose.ui.unit.Dp(48f).roundToPx()
        val rowHeight = androidx.compose.ui.unit.Dp(32f).roundToPx()
        val spacing = androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_SPACING__.toFloat()).roundToPx()
        val selectedSize = androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_SELECTED_SIZE__.toFloat()).roundToPx()
        val unselectedSize = androidx.compose.ui.unit.Dp(__NEXA_PAGE_INDICATOR_UNSELECTED_SIZE__.toFloat()).roundToPx()
        val geometry = nexaPageIndicatorGeometry(
            constraints.maxWidth,
            pageCount,
            currentPage,
            spacing,
            selectedSize,
            unselectedSize,
        )
        val width = geometry.visualWidth.coerceIn(constraints.minWidth, constraints.maxWidth)
        val dotCenters = if (width == constraints.maxWidth) {
            geometry.dotCenters
        } else {
            nexaPageIndicatorGeometry(
                width,
                pageCount,
                currentPage,
                spacing,
                selectedSize,
                unselectedSize,
            ).dotCenters
        }
        val height = rowHeight.coerceIn(constraints.minHeight, constraints.maxHeight)
        val placeables = measurables.map { measurable ->
            measurable.measure(androidx.compose.ui.unit.Constraints.fixed(targetSize, targetSize))
        }
        layout(width, height) {
            placeables.forEachIndexed { index, placeable ->
                placeable.place(
                    x = dotCenters[index] - targetSize / 2,
                    y = androidx.compose.ui.unit.Dp(4f).roundToPx() - targetSize / 2,
                )
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
        // The custom bar owns the navigation-bar inset. Its route content is
        // laid out above that inset, so nested Nexa safe-area consumers must
        // not reserve the same bottom space a second time.
        val parentSafeArea = LocalNexaSafeAreaContext.current
        val remainingSafeBottom = nexaSafeAreaDynamicInsets().bottom
        val consumedNavigationBar = remainingSafeBottom
        androidx.compose.runtime.CompositionLocalProvider(
            LocalNexaSafeAreaContext provides parentSafeArea.copy(
                consumed = parentSafeArea.consumed.offset(
                    NexaSafeAreaEdge.Bottom,
                    consumedNavigationBar,
                ),
            ),
            content = content,
        )
    }
}

"#;

const NEXA_FAST_LIST_ROW_PRIMITIVE: &str = r#"@androidx.compose.runtime.Composable
internal fun NexaFastListRowPrimitive(
    modifier: androidx.compose.ui.Modifier,
    horizontalAlignment: androidx.compose.ui.Alignment.Horizontal,
    verticalArrangement: androidx.compose.foundation.layout.Arrangement.Vertical = androidx.compose.foundation.layout.Arrangement.Center,
    content: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit,
) {
    NexaScrollContentResetPrimitive {
        androidx.compose.foundation.layout.Column(
            modifier = modifier.defaultMinSize(
                minHeight = androidx.compose.ui.unit.Dp(__NEXA_FAST_LIST_ROW_MIN_HEIGHT__.toFloat()),
            ),
            horizontalAlignment = horizontalAlignment,
            verticalArrangement = verticalArrangement,
            content = content,
        )
    }
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
            horizontal = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__.toFloat()),
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
    val corner = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_CORNER_RADIUS__.toFloat())
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

const NEXA_FAST_LIST_SWIPE_PRIMITIVE: &str = r#"private val LocalNexaFastListSwipePagerLock =
    androidx.compose.runtime.staticCompositionLocalOf<androidx.compose.runtime.MutableState<Boolean>?> { null }

@androidx.compose.runtime.Composable
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
internal fun NexaFastListSwipePrimitive(
    resetKey: Any?,
    backgroundContent: @androidx.compose.runtime.Composable androidx.compose.foundation.layout.RowScope.() -> Unit,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    androidx.compose.runtime.key(resetKey) {
        val state = androidx.compose.material3.rememberSwipeToDismissBoxState()
        androidx.compose.runtime.LaunchedEffect(state, resetKey) {
            state.reset()
        }
        val pagerGestureLock = LocalNexaFastListSwipePagerLock.current
        val pagerGestureLockModifier = if (pagerGestureLock == null) {
            androidx.compose.ui.Modifier
        } else {
            androidx.compose.ui.Modifier.pointerInput(pagerGestureLock) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = false)
                    pagerGestureLock.value = true
                    try {
                        var pointerPressed = true
                        while (pointerPressed) {
                            pointerPressed = awaitPointerEvent().changes.any { it.pressed }
                        }
                    } finally {
                        pagerGestureLock.value = false
                    }
                }
            }
        }
        androidx.compose.material3.SwipeToDismissBox(
            state = state,
            modifier = pagerGestureLockModifier,
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
}

"#;

#[cfg(test)]
fn shared_component_primitives() -> String {
    shared_component_primitives_with_wave(false, false)
}

fn shared_component_primitives_with_wave(
    uses_android_wave: bool,
    uses_android_wave_ring: bool,
) -> String {
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
    let wave_parameter = if uses_android_wave {
        "androidWave: Boolean = false,"
    } else {
        ""
    };
    let square_progress_canvas_condition = if uses_android_wave {
        "cornerRadius == 0f && !androidWave"
    } else {
        "cornerRadius == 0f"
    };
    let wave_indeterminate_indicator = if uses_android_wave {
        r##"if (androidWave) {
        androidx.compose.material3.LinearWavyProgressIndicator(
            modifier = modifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
        )
    } else {
        androidx.compose.material3.LinearProgressIndicator(
            modifier = modifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
        )
    }"##
    } else {
        r##"androidx.compose.material3.LinearProgressIndicator(
        modifier = modifier,
        color = indicatorColor,
        trackColor = indicatorTrackColor,
    )"##
    };
    let wave_determinate_indicator = if uses_android_wave {
        r##"if (androidWave) {
        androidx.compose.material3.LinearWavyProgressIndicator(
            progress = { normalizedProgress },
            modifier = modifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
        )
    } else {
        androidx.compose.material3.LinearProgressIndicator(
            progress = { normalizedProgress },
            modifier = modifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
            drawStopIndicator = {},
        )
    }"##
    } else {
        r##"androidx.compose.material3.LinearProgressIndicator(
        progress = { normalizedProgress },
        modifier = modifier,
        color = indicatorColor,
        trackColor = indicatorTrackColor,
        drawStopIndicator = {},
    )"##
    };
    let wave_ring_parameter = if uses_android_wave_ring {
        "androidWave: Boolean = false,"
    } else {
        ""
    };
    let wave_ring_opt_in = if uses_android_wave_ring {
        "@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)\n"
    } else {
        ""
    };
    let wave_ring_indicator = if uses_android_wave_ring {
        r##"if (androidWave) {
        val density = androidx.compose.ui.platform.LocalDensity.current
        val stroke = androidx.compose.runtime.remember(indicatorThickness, density) {
            androidx.compose.ui.graphics.drawscope.Stroke(
                width = with(density) { indicatorThickness.dp.toPx() },
                cap = androidx.compose.ui.graphics.StrokeCap.Round,
            )
        }
        if (normalizedProgress == null) {
            androidx.compose.material3.CircularWavyProgressIndicator(
                modifier = semanticsModifier,
                color = indicatorColor,
                trackColor = indicatorTrackColor,
                stroke = stroke,
                trackStroke = stroke,
            )
        } else {
            androidx.compose.material3.CircularWavyProgressIndicator(
                progress = { normalizedProgress },
                modifier = semanticsModifier,
                color = indicatorColor,
                trackColor = indicatorTrackColor,
                stroke = stroke,
                trackStroke = stroke,
            )
        }
    } else if (normalizedProgress == null) {
        androidx.compose.material3.CircularProgressIndicator(
            modifier = semanticsModifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
            strokeWidth = indicatorThickness.dp,
        )
    } else {
        androidx.compose.material3.CircularProgressIndicator(
            progress = { normalizedProgress },
            modifier = semanticsModifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
            strokeWidth = indicatorThickness.dp,
        )
    }"##
    } else {
        r##"if (normalizedProgress == null) {
        androidx.compose.material3.CircularProgressIndicator(
            modifier = semanticsModifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
            strokeWidth = indicatorThickness.dp,
        )
    } else {
        androidx.compose.material3.CircularProgressIndicator(
            progress = { normalizedProgress },
            modifier = semanticsModifier,
            color = indicatorColor,
            trackColor = indicatorTrackColor,
            strokeWidth = indicatorThickness.dp,
        )
    }"##
    };
    let source = NEXA_SHARED_COMPONENT_PRIMITIVES
        .replace("__NEXA_DARK_COLOR_SCHEME__", &qualified_scheme(true))
        .replace("__NEXA_LIGHT_COLOR_SCHEME__", &qualified_scheme(false))
        .replace("__NEXA_ANDROID_WAVE_PARAMETER__", wave_parameter)
        .replace(
            "__NEXA_ANDROID_WAVE_INDETERMINATE_INDICATOR__",
            wave_indeterminate_indicator,
        )
        .replace(
            "__NEXA_ANDROID_WAVE_DETERMINATE_INDICATOR__",
            wave_determinate_indicator,
        )
        .replace(
            "__NEXA_ANDROID_SQUARE_PROGRESS_CANVAS_CONDITION__",
            square_progress_canvas_condition,
        )
        .replace("__NEXA_ANDROID_WAVE_RING_PARAMETER__", wave_ring_parameter)
        .replace("__NEXA_ANDROID_WAVE_RING_INDICATOR__", wave_ring_indicator)
        .replace("__NEXA_ANDROID_WAVE_RING_OPT_IN__", wave_ring_opt_in);
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
            "__NEXA_FORM_SECTION_HEADER_INSET__",
            nexa_codegen::design_system::FORM_SECTION_HEADER_INSET.to_string(),
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
    let text_reveal = include_str!("components/text_reveal.kt")
        .replace(
            "__NEXA_DEFAULT_BODY_FONT_SIZE__",
            &nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE.to_string(),
        )
        .replace(
            "__NEXA_DEFAULT_LINE_HEIGHT_MULTIPLIER__",
            &nexa_codegen::design_system::DEFAULT_LINE_HEIGHT_MULTIPLIER.to_string(),
        );
    source.push_str(&text_reveal);
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
    components::controls::register_known_screens(&module.screens);
    let focus_bindings = features.facts.focus_bindings.app.clone();
    let imports = engine::imports::render(engine::imports::ImportContext {
        features,
        include_all_shared_primitives,
        uses_plugins: !module.plugins.is_empty(),
        has_navigation: !module.screens.is_empty(),
        has_direction: module.direction.is_some()
            || features.facts.capabilities.uses_locale_api
            || features.facts.capabilities.uses_localized_strings,
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
        out.push_str(
            r#"public typealias NexaResult<T, E> = dev.nexa.core.NexaResult<T, E>
public typealias NexaComponentRefError<Failure> = dev.nexa.core.NexaComponentRefError<Failure>

public class NexaFastListRef {
    private var indexAction: ((Int, Boolean, Double) -> Unit)? = null
    private var topAction: ((Boolean) -> Unit)? = null
    private var endAction: ((Boolean) -> Unit)? = null
    private var offsetAction: ((Double, Boolean) -> Unit)? = null

    public fun attach(
        scrollToIndex: (Int, Boolean, Double) -> Unit,
        scrollToTop: (Boolean) -> Unit,
        scrollToEnd: (Boolean) -> Unit,
        scrollToOffset: (Double, Boolean) -> Unit
    ) {
        indexAction = scrollToIndex
        topAction = scrollToTop
        endAction = scrollToEnd
        offsetAction = scrollToOffset
    }

    public fun detach() {
        indexAction = null
        topAction = null
        endAction = null
        offsetAction = null
    }

    public fun scrollToIndex(
        index: Int,
        animated: Boolean = true,
        viewPosition: Double = 0.0,
    ): NexaResult<Unit, NexaComponentRefError<Unit>> =
        indexAction?.let {
            val normalizedPosition = if (viewPosition.isFinite()) viewPosition.coerceIn(0.0, 1.0) else 0.0
            it(index, animated, normalizedPosition)
            dev.nexa.core.NexaResult.Success(Unit)
        }
            ?: dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)

    public fun scrollToTop(animated: Boolean = true): NexaResult<Unit, NexaComponentRefError<Unit>> =
        topAction?.let { it(animated); dev.nexa.core.NexaResult.Success(Unit) }
            ?: dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)

    public fun scrollToEnd(animated: Boolean = true): NexaResult<Unit, NexaComponentRefError<Unit>> =
        endAction?.let { it(animated); dev.nexa.core.NexaResult.Success(Unit) }
            ?: dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)

    public fun scrollToOffset(
        offset: Double,
        animated: Boolean = true,
    ): NexaResult<Unit, NexaComponentRefError<Unit>> =
        offsetAction?.let { it(offset.coerceAtLeast(0.0), animated); dev.nexa.core.NexaResult.Success(Unit) }
            ?: dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)
}

public class NexaBottomSheetRef {
    private var generation = 0L
    private var presentAction: (() -> Unit)? = null
    private var dismissAction: (() -> Unit)? = null

    public fun attach(present: () -> Unit, dismiss: () -> Unit): Long {
        generation += 1
        presentAction = present
        dismissAction = dismiss
        return generation
    }

    public fun detach(expectedGeneration: Long) {
        if (generation != expectedGeneration) return
        presentAction = null
        dismissAction = null
    }

    public fun present(): NexaResult<Unit, NexaComponentRefError<Unit>> =
        presentAction?.let { it(); dev.nexa.core.NexaResult.Success(Unit) }
            ?: dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)

    public fun dismiss(): NexaResult<Unit, NexaComponentRefError<Unit>> =
        dismissAction?.let { it(); dev.nexa.core.NexaResult.Success(Unit) }
            ?: dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)
}

"#,
        );
        if features.uses_fast_list {
            out.push_str(
                &NEXA_FAST_LIST_ROW_PRIMITIVE.replace(
                    "__NEXA_FAST_LIST_ROW_MIN_HEIGHT__",
                    &nexa_codegen::design_system::FAST_LIST_ROW_MIN_HEIGHT.to_string(),
                ),
            );
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
                    "__NEXA_NATIVE_LIST_ROW_CORNER_RADIUS__",
                    &nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_CORNER_RADIUS
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
                    "__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__",
                    &nexa_codegen::design_system::NATIVE_LIST_CONTAINER_HORIZONTAL_INSET
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
            let app_root_status_bar = if module
                .screens
                .iter()
                .any(|screen| screen.status_bar.is_some())
            {
                None
            } else {
                module.status_bar
            };
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
            components::status_bar::render(
                app_root_status_bar,
                features.uses_status_bar,
                1,
                out,
            );
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
                || features.facts.capabilities.uses_locale_api
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
                        let remember = if state::uses_saveable_state(state) {
                            "rememberSaveable"
                        } else {
                            "remember"
                        };
                        out.push_str(&format!(
                            "    var {name} by {remember} {{ {} }}\n",
                            state::kotlin_state_initializer(state)
                        ));
                    }
                } else if state.is_native_class_instance_binding()
                    || matches!(state.ty, nexa_ir::Type::Signal(_))
                    || matches!(state.ty, nexa_ir::Type::ComponentRef { .. })
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
            if !module.screens.is_empty() {
                components::navigation::render_screen_functions(module, features, 1, out);
                out.push('\n');
            }
            out.push_str("    NexaSafeAreaProvider {\n");
            let mut body_base_depth = 2;
            if features.uses_shared_elements {
                out.push_str("        NexaSharedTransitionContent {\n");
                body_base_depth += 1;
            }
            let follows_locale = features.facts.capabilities.uses_locale_api
                || features.facts.capabilities.uses_localized_strings;
            if follows_locale {
                out.indent(body_base_depth);
                out.push_str("NexaLocale.provide {\n");
                body_base_depth += 1;
            }
            let wrap_app_status_bar_background = components::status_bar::render_background_start(
                app_root_status_bar,
                body_base_depth,
                out,
            );
            let body_depth =
                components::direction::start(module.direction, follows_locale, body_base_depth, out);
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
            components::direction::end(module.direction, follows_locale, body_base_depth, out);
            if wrap_app_status_bar_background {
                components::status_bar::render_background_end(body_base_depth, out);
            }
            if follows_locale {
                out.push('\n');
                out.indent(body_base_depth - 1);
                out.push('}');
            }
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
        || features.facts.capabilities.uses_locale_api
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
                features.facts.capabilities.uses_locale_api
                    || features.facts.capabilities.uses_localized_strings,
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
        nexa_codegen::source_sections::all(&shared_component_primitives_with_wave(true, true))
    } else {
        nexa_codegen::source_sections::reachable(
            &shared_component_primitives_with_wave(
                features.uses_wavy_progress_bar,
                features.uses_wavy_progress_ring,
            ),
            &references,
        )
    };
    generated.prepend_to_unit("NexaGenerated_types.kt", &primitives);
    generated
}

#[cfg(test)]
mod tests {
    use super::{
        NEXA_APP_BOTTOM_BAR_PRIMITIVE, NEXA_NATIVE_LIST_PRIMITIVES,
        NEXA_SHARED_COMPONENT_PRIMITIVES, generate,
        generate_units_with_network_and_project_features, generate_units_with_project_features,
        shared_component_primitives, shared_component_primitives_with_wave,
    };
    use nexa_ir::{
        Action, AnimationSpec, CollectionTransform, Component, Expr, Function, ImageScale,
        ImageSource, LayoutKind, ListAxis, ListCommon, ListPlan, Module, Node, NumericType, Screen,
        ScreenId, SectionedListCommon, State, SystemIcon, TaskExecutor, TextStyle, Type, ViewStyle,
        ViewTransition, WhenCase,
    };

    fn braced_block(source: &str, start: usize) -> &str {
        let open = source[start..]
            .find('{')
            .map(|offset| start + offset)
            .expect("generated block has an opening brace");
        let mut depth = 0usize;
        for (offset, character) in source[open..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &source[start..open + offset + character.len_utf8()];
                    }
                }
                _ => {}
            }
        }
        panic!("generated block is closed");
    }

    #[test]
    fn native_plugin_network_requirement_emits_shared_cronet_runtime() {
        let module = Module {
            app_name: "PluginNetworkApp".to_owned(),
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

        let (sources, features) = generate_units_with_network_and_project_features(&module);
        let network = sources
            .units
            .iter()
            .find(|unit| unit.name.contains("native_library"))
            .expect("plugin host requirement should emit the network runtime");
        assert!(features.uses_network);
        assert!(features.uses_coroutines);
        assert!(network.contents.contains("object NexaCronetRuntime"));
        assert!(network.contents.contains("object NexaNetwork"));
    }

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
    fn custom_app_bottom_bar_consumes_reserved_navigation_inset_for_tab_content() {
        assert!(!NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("WindowInsets.navigationBars"));
        assert!(NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("nexaSafeAreaDynamicInsets().bottom"));
        assert!(
            NEXA_APP_BOTTOM_BAR_PRIMITIVE
                .contains("val consumedNavigationBar = remainingSafeBottom")
        );
        assert!(
            NEXA_APP_BOTTOM_BAR_PRIMITIVE
                .contains("LocalNexaSafeAreaContext provides parentSafeArea.copy(")
        );
        assert!(NEXA_APP_BOTTOM_BAR_PRIMITIVE.contains("content = content"));
    }

    #[test]
    fn keyboard_aware_scroll_does_not_refocus_a_field_during_a_drag() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaKeyboardAwareColumnPrimitive("));
        assert!(primitives.contains("internal fun NexaKeyboardAwareOwnerPrimitive("));
        let keyboard_aware = primitives
            .split("private fun nexaKeyboardAwareModifier(")
            .nth(1)
            .expect("shared keyboard-aware modifier is emitted");
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
        assert!(primitives.contains("if (avoidIme) modifier = modifier.imePadding()"));
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
                        show_dividers: true,
                        reverse_layout: false,
                        page_snap: false,
                        item_extent: Some(80.0),
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: None,
                        component_ref: None,
                        children: vec![Node::Text {
                            value: Expr::String("Card".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_scroll_begin_drag: None,
                        on_move: None,
                        swipe_actions: None,
                        sticky_header: None,
                        refresh: None,
                        gap: 0.0,
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
            "LazyRow(modifier = nexaSafeAreaScrollViewportModifier(\"Horizontal\").then(Modifier.fillMaxWidth().nestedScroll(NexaHorizontalListScrollBoundary)), contentPadding = LocalNexaScrollContentPadding.current, state = nexaListState0, userScrollEnabled = (nexaListState0.canScrollForward || nexaListState0.canScrollBackward))"
        ));
        assert!(kotlin.contains("NexaSafeAreaProvider {"));
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

        let mut referenced_module = module.clone();
        let Node::FastList { plan } = &mut referenced_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_mut() else {
            panic!("expected the count-backed FastList plan");
        };
        common.component_ref = Some(Expr::ComponentRefCreate {
            namespace: "Nexa".to_owned(),
            name: "FastList".to_owned(),
            identity: "test-fast-list-ref".to_owned(),
        });
        let referenced_kotlin = generate(&referenced_module);
        assert!(referenced_kotlin.contains("SideEffect {"));
        assert!(referenced_kotlin.contains("scrollToEnd = { animated -> if ("));
        assert!(referenced_kotlin.contains("DisposableEffect(nexaFastListRef"));
        assert!(referenced_kotlin.contains("onDispose { nexaFastListRef"));

        let mut computed_items_module = module;
        let Node::FastList { plan } = &mut computed_items_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_ref() else {
            panic!("expected the count-backed FastList plan");
        };
        let mut common = common.clone();
        common.key = Some(Expr::State("item".to_owned(), Type::String));
        **plan = ListPlan::Items {
            collection: Expr::CollectionTransform {
                operation: CollectionTransform::Filter,
                collection: Box::new(Expr::State(
                    "items".to_owned(),
                    Type::Array(Box::new(Type::String)),
                )),
                initial: None,
                closure: Box::new(Expr::Closure {
                    parameters: vec!["item".to_owned()],
                    body: Box::new(Expr::Bool(true)),
                }),
            },
            element_type: Type::String,
            item: "item".to_owned(),
            common,
        };
        let computed_items_kotlin = generate(&computed_items_module);
        assert_eq!(
            computed_items_kotlin
                .matches(".filter { item -> true }")
                .count(),
            1
        );
        assert!(
            computed_items_kotlin
                .contains("val nexaListValues0 = nexa_items.filter { item -> true }"),
            "{computed_items_kotlin}"
        );
        assert!(computed_items_kotlin.contains("count = nexaListValues0.size"));
        assert!(computed_items_kotlin.contains("nexaListValues0[itemPosition]"));

        let mut computed_sections_module = referenced_module;
        let Node::FastList { plan } = &mut computed_sections_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_ref() else {
            panic!("expected the count-backed FastList plan");
        };
        let common = SectionedListCommon {
            show_dividers: true,
            item_extent: None,
            gap: 0.0,
            scroll_enabled: None,
            index: "index".to_owned(),
            component_ref: common.component_ref.clone(),
            key: None,
            children: common.children.clone(),
            section_header: None,
            swipe_actions: None,
            refresh: None,
        };
        **plan = ListPlan::Sections {
            collection: Expr::CollectionTransform {
                operation: CollectionTransform::Filter,
                collection: Box::new(Expr::State(
                    "sections".to_owned(),
                    Type::Array(Box::new(Type::Array(Box::new(Type::String)))),
                )),
                initial: None,
                closure: Box::new(Expr::Closure {
                    parameters: vec!["section".to_owned()],
                    body: Box::new(Expr::Bool(true)),
                }),
            },
            element_type: Type::Array(Box::new(Type::String)),
            section: "section".to_owned(),
            item: "item".to_owned(),
            common,
        };
        let computed_sections_kotlin = generate(&computed_sections_module);
        assert_eq!(
            computed_sections_kotlin
                .matches(".filter { section -> true }")
                .count(),
            1
        );
        assert!(computed_sections_kotlin.contains("val nexaListSections"));
        assert!(computed_sections_kotlin.contains(".sumOf { it.size }"));
        assert!(
            computed_sections_kotlin.contains(".forEachIndexed { sectionPosition, sectionItems ->")
        );
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
                        show_dividers: true,
                        reverse_layout: true,
                        page_snap: false,
                        item_extent: Some(48.0),
                        index: "index".to_owned(),
                        key: None,
                        scroll_position: Some("listPosition".to_owned()),
                        component_ref: None,
                        children: vec![Node::Text {
                            value: Expr::String("Reverse row".to_owned()),
                            style: TextStyle::default(),
                        }],
                        on_end_reached: None,
                        on_scroll: None,
                        on_scroll_begin_drag: None,
                        on_move: None,
                        swipe_actions: None,
                        sticky_header: None,
                        refresh: None,
                        gap: 4.0,
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
        assert!(kotlin.contains("reverseLayout = true"));
        assert!(kotlin.contains("key = { itemPosition -> itemPosition }"));
        assert!(kotlin.contains("val nexa_index: Int = itemPosition"));
        assert!(!kotlin.contains("nexaListCount0 - 1 - itemPosition"));
        assert!(kotlin.contains("fun leadingVisibleDataItemIndex(): Int"));
        assert!(kotlin.contains(
            "val visibleItems = layoutInfo.visibleItemsInfo.filter { it.offset + it.size > layoutInfo.viewportStartOffset && it.offset < layoutInfo.viewportEndOffset }"
        ));
        assert!(kotlin.contains(
            "if (leadingIndex < 0 || item.index > leadingIndex) leadingIndex = item.index"
        ));
        assert!(
            kotlin
                .contains("snapshotFlow { leadingVisibleDataItemIndex() }.distinctUntilChanged()")
        );
        assert!(kotlin.contains(
            "val target = (nexaListCount0 - 1 - nexa_listPosition).coerceIn(0, (nexaListCount0 - 1).coerceAtLeast(0))"
        ));
        assert!(
            kotlin.contains(
                "nexaListState0.layoutInfo.visibleItemsInfo.maxOfOrNull { it.index } ?: -1"
            )
        );

        let mut observed_module = module.clone();
        let Node::FastList { plan } = &mut observed_module.body[0] else {
            panic!("expected the count-backed FastList plan");
        };
        let ListPlan::Count { common, .. } = plan.as_mut() else {
            panic!("expected the count-backed FastList plan");
        };
        common.sticky_header = Some(vec![Node::Text {
            value: Expr::String("Pinned header".to_owned()),
            style: TextStyle::default(),
        }]);
        common.on_scroll = Some(Vec::new());
        common.on_end_reached = Some(Vec::new());
        let observed_kotlin = generate(&observed_module);
        assert!(observed_kotlin.contains("remember(nexaListState0) { booleanArrayOf(false) }"));
        assert!(observed_kotlin.contains("if (!isInitialScrollPosition) {"));
        assert!(
            observed_kotlin
                .contains("val stickyHeader = visibleItems.firstOrNull { it.index == 0 }")
        );
        assert!(
            observed_kotlin
                .contains("item.index >= nexaListCount0 + 1 - 1 && item.offset < layoutInfo.viewportEndOffset && item.offset + item.size > layoutInfo.viewportStartOffset && (stickyHeader == null")
        );
        let reverse_target = "(nexaListCount0 - 1 - nexa_listPosition.coerceAtLeast(0)).coerceIn(0, (nexaListCount0 - 1).coerceAtLeast(0))";
        assert!(observed_kotlin.contains(&format!(
            "initialFirstVisibleItemIndex = if ({reverse_target} == 0) 0 else ({reverse_target} + 1)"
        )));
        let reverse_scroll_target = "(nexaListCount0 - 1 - nexa_listPosition).coerceIn(0, (nexaListCount0 - 1).coerceAtLeast(0))";
        assert!(observed_kotlin.contains(&format!(
            "val target = if ({reverse_scroll_target} == 0) 0 else ({reverse_scroll_target} + 1)"
        )));
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
            nexa_codegen::design_system::NATIVE_LIST_CONTAINER_HORIZONTAL_INSET,
            0
        );
        assert_eq!(
            nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_HORIZONTAL_INSET,
            16
        );
        assert_eq!(
            nexa_codegen::design_system::ANDROID_NATIVE_LIST_ROW_CORNER_RADIUS,
            0
        );
        assert_eq!(
            nexa_codegen::design_system::ANDROID_NATIVE_LIST_CONTENT_VERTICAL_INSET,
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
            "val corner = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_CORNER_RADIUS__.toFloat())"
        ));
        assert!(NEXA_NATIVE_LIST_PRIMITIVES.contains(
            "horizontal = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_ROW_HORIZONTAL_INSET__.toFloat())"
        ));
        assert!(NEXA_NATIVE_LIST_PRIMITIVES.contains(
            "horizontal = androidx.compose.ui.unit.Dp(__NEXA_NATIVE_LIST_CONTAINER_HORIZONTAL_INSET__.toFloat())"
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
        assert!(primitives.contains("accessibilityLabel: String? = null"));
        assert!(primitives.contains("modifier.semantics(mergeDescendants = true)"));
        assert!(primitives.contains("contentDescription = accessibilityLabel"));
        assert!(primitives.contains("MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)"));
        assert!(primitives.contains("color = if (plainButtonStyle) {"));
        assert!(primitives.contains("Modifier.size(androidx.compose.ui.unit.Dp(20.toFloat()))"));
        assert!(primitives.contains("val plainContentColor = when {"));
        assert!(primitives.contains("if (resolvedStyle == \"Borderless\") {\n        androidx.compose.material3.MaterialTheme.colorScheme.primary"));
        assert!(primitives.contains("tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary).copy(alpha = 0.38f)"));
        assert!(primitives.contains(
            "PaddingValues(horizontal = androidx.compose.ui.unit.Dp(0.toFloat()), vertical = androidx.compose.ui.unit.Dp(0.toFloat()))"
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
            "bottom = androidx.compose.ui.unit.Dp(\n                    if (footer == null) {\n                        12.toFloat()\n                    } else {\n                        8.toFloat()\n                    },\n                ),"
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
        assert!(progress_bar.contains("val normalizedProgress = progress?.let"));
        assert!(progress_bar.contains("if (it.isFinite()) it.coerceIn(0f, 1f) else 0f"));
        assert!(progress_bar.contains("modifier.progressSemantics()"));
        assert!(progress_bar.contains("modifier.progressSemantics(normalizedProgress)"));
        assert!(progress_bar.contains("progress = { normalizedProgress }"));
        assert!(progress_bar.contains("cornerRadius == 0f"));
        assert!(progress_bar.contains("durationMillis = 250"));
        assert!(progress_bar.contains("easing = androidx.compose.animation.core.LinearEasing"));
        assert!(progress_bar.contains("androidx.compose.foundation.Canvas(modifier = modifier)"));
        assert!(progress_bar.contains(
            "val indicatorColor = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary"
        ));
        assert!(!progress_bar.contains("LinearWavyProgressIndicator"));
        assert!(progress_bar.contains("modifier = modifier"));
        assert!(progress_bar.contains(
            "val indicatorTrackColor = trackColor\n        ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface.copy(alpha = 0.1f)"
        ));
        assert!(progress_bar.contains("trackColor = indicatorTrackColor"));
        assert!(progress_bar.contains(
            "if (normalizedProgress == null) {\n        androidx.compose.material3.LinearProgressIndicator(\n        modifier = modifier"
        ));
        assert!(progress_bar.contains("drawStopIndicator = {},"));
        assert!(progress_ring.contains("val normalizedProgress = progress?.let"));
        assert!(progress_ring.contains("progress: Float?"));
        assert!(progress_ring.contains("if (normalizedProgress == null)"));
        assert!(!progress_ring.contains("CircularWavyProgressIndicator"));
        assert!(progress_ring.contains("val indicatorSize = (size ?: 40f).coerceAtLeast(0f)"));
        assert!(progress_ring.contains(".Modifier.size(indicatorSize.dp)"));
        assert!(
            progress_ring
                .contains("val indicatorThickness = (thickness ?: 4f).coerceIn(0f, indicatorSize)")
        );
        assert!(progress_ring.contains("strokeWidth = indicatorThickness.dp"));
        assert!(progress_ring.contains("modifier.progressSemantics(normalizedProgress)"));
        assert!(
            progress_ring
                .contains("val indicatorColor = tint ?: androidx.compose.material3.MaterialTheme.colorScheme.primary")
        );
        assert!(progress_ring.contains(
            "val indicatorTrackColor = trackColor ?: indicatorColor.copy(alpha = 0.16f)"
        ));
    }

    #[test]
    fn wavy_progress_api_is_emitted_only_for_apps_that_request_it() {
        let regular = shared_component_primitives_with_wave(false, false);
        let wavy = shared_component_primitives_with_wave(true, true);

        assert!(!regular.contains("LinearWavyProgressIndicator"));
        assert!(!regular.contains("androidWave: Boolean"));
        assert!(regular.contains("else if (cornerRadius == 0f)"));
        assert!(wavy.contains("androidWave: Boolean = false"));
        assert!(wavy.contains("else if (cornerRadius == 0f && !androidWave)"));
        assert!(!wavy.contains("__NEXA_ANDROID_SQUARE_PROGRESS_CANVAS_CONDITION__"));
        assert!(wavy.contains("if (androidWave) {"));
        assert!(wavy.contains("androidx.compose.material3.LinearWavyProgressIndicator("));
        assert!(wavy.contains("androidx.compose.material3.CircularWavyProgressIndicator("));
        assert!(wavy.contains(
            "if (normalizedProgress == null) {\n        if (androidWave) {\n        androidx.compose.material3.LinearWavyProgressIndicator(\n            modifier = modifier"
        ));
    }

    #[test]
    fn form_section_heading_uses_the_shared_muted_text_role() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains(
            "style = androidx.compose.material3.MaterialTheme.typography.titleSmall,\n                color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,"
        ));
        assert!(primitives.contains("start = androidx.compose.ui.unit.Dp(16.toFloat()),"));
        let form_section = primitives
            .split("internal fun NexaFormSectionPrimitive(")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        let section_container = form_section
            .split("if (title != null)")
            .next()
            .unwrap_or_default();
        let section_heading = form_section
            .split("if (title != null)")
            .nth(1)
            .and_then(|source| source.split("androidx.compose.material3.Surface").next())
            .unwrap_or_default();
        assert!(!section_container.contains("top = androidx.compose.ui.unit.Dp"));
        assert!(!section_heading.contains("top = androidx.compose.ui.unit.Dp"));
        assert!(!form_section.contains("firstSection"));
        let form = primitives
            .split("internal fun NexaFormPrimitive(")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        assert!(!form.contains("nexaSafeAreaContentPadding"));
        assert!(!form.contains("LocalNexaFormContentTopInset"));
        assert!(!form.contains("50.toFloat()"));
        assert!(!form.contains("34.toFloat()"));
        let vertical_scroll = form
            .find(".verticalScroll(scrollState, enabled = scrollState.maxValue > 0)")
            .expect("Form owns scrolling before applying content padding");
        let content_padding = form
            .find(".padding(effectiveContentPadding)")
            .expect("Form padding is scrollable content padding");
        assert!(vertical_scroll < content_padding);
        assert!(
            form.contains("modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier")
        );
        assert!(!form.contains("nexaSafeAreaScrollViewportModifier"));
        assert!(primitives.contains("endInset = androidx.compose.ui.unit.Dp(20.toFloat())"));
        assert!(primitives.contains("0xFF404042"));

        let dev_renderer = include_str!("../../../../runtime/android/NexaDevRenderer.kt");
        let dev_insets_dispatch = dev_renderer
            .split("\"ViewInsets\" -> {")
            .nth(1)
            .and_then(|source| source.split("\"SafeAreaInset\" -> {").next())
            .unwrap_or_default();
        let dev_form = dev_renderer
            .split("\"Form\" -> {")
            .nth(1)
            .and_then(|source| source.split("\"FormSection\" -> {").next())
            .unwrap_or_default();
        assert!(dev_insets_dispatch.contains("if (child.has(\"Form\"))"));
        assert!(dev_insets_dispatch.contains("formContentPadding = PaddingValues("));
        assert!(dev_insets_dispatch.contains("modifier = outerModifier"));
        assert!(dev_form.contains("modifier = modifier"));
        assert!(dev_form.contains("contentPadding = formContentPadding ?: PaddingValues(0.dp)"));
    }

    #[test]
    fn safe_area_snapshot_is_centralized_and_consumed_consistently() {
        let primitives = shared_component_primitives();
        let provider = primitives
            .split("internal fun NexaSafeAreaProvider(")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        let remaining = primitives
            .split("internal fun nexaSafeAreaDynamicInsets(): NexaSafeAreaInsets {")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        let inset = primitives
            .split("internal fun NexaSafeAreaInsetPrimitive(")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();

        assert!(provider.contains("systemBars.union(displayCutout).union(waterfall)"));
        assert!(provider.contains("remember(systemBars, displayCutout, waterfall)"));
        assert!(provider.contains("remember(\n        topPixels,"));
        assert!(provider.contains("/ pixelsPerDp"));
        assert!(provider.contains("LocalNexaWindowMetrics provides snapshot"));
        assert!(remaining.contains("visual.left else visual.right"));
        assert!(remaining.contains("visual.right else visual.left"));
        assert!(remaining.contains("logical.subtractClamped(consumed)"));
        assert!(!primitives.contains("NexaVisualSafeAreaEdges"));
        assert!(!primitives.contains("val crossEdges = if (vertical)"));
        assert!(primitives.contains("(top - other.top).coerceAtLeast(0.dp)"));
        assert!(primitives.contains("(bottom - other.bottom).coerceAtLeast(0.dp)"));
        assert!(primitives.contains("(start - other.start).coerceAtLeast(0.dp)"));
        assert!(primitives.contains("(end - other.end).coerceAtLeast(0.dp)"));
        assert!(provider.contains("top = androidx.compose.ui.unit.Dp(topPixels / pixelsPerDp)"));
        assert!(!remaining.contains("WindowInsets."));
        let view_insets = primitives
            .split("internal fun NexaViewInsetsPrimitive(")
            .nth(1)
            .and_then(|source| source.split("// nexa-codegen:").next())
            .unwrap_or_default();
        assert!(!view_insets.contains("WindowInsets."));
        assert!(!view_insets.contains("windowInsetsPadding"));
        assert!(view_insets.contains("paddingTop"));
        assert!(!primitives.contains("nexaSafeAreaContentPadding"));
        assert!(inset.contains("occupiedBeforeAttachment + safe"));
        assert!(inset.contains("consumed = parent.consumed.offset(edge, safe) + crossSafe"));
        assert!(inset.contains("val occupied = safe + attachmentSize"));
        assert!(!inset.contains("spacing"));
        assert!(inset.contains("if (accommodatesEndpoint) occupied else 0.dp"));

        let dev_runtime = include_str!("../../../../runtime/android/NexaDevRuntime.kt");
        assert!(dev_runtime.contains("NexaSafeAreaProvider {"));
    }

    #[test]
    fn shared_picker_primitive_owns_its_menu_layout_and_text_metrics() {
        let primitives = shared_component_primitives();
        assert!(primitives.contains("internal fun NexaPickerPrimitive("));
        assert!(primitives.contains("DropdownMenuItem("));
        assert!(primitives.contains("itemIcons: List<NexaIconSpec?>"));
        assert!(primitives.contains("leadingIcon = itemIcons.getOrNull(index)?.let"));
        assert!(primitives.contains("icon = optionIcon"));
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
        assert!(primitives.contains(
            "textColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurface"
        ));
        assert!(primitives.contains("placeholderColor ?: androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant"));
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
        assert!(primitives.contains("contentDescription = \"Page ${index + 1}\""));
        assert!(primitives.contains("onClick(label = \"Page ${index + 1}\")"));
        assert!(primitives.contains(".size(androidx.compose.ui.unit.Dp(48f))"));
        assert!(primitives.contains("detectTapGestures { tap ->"));
        assert!(primitives.contains("nearestDistance <= androidx.compose.ui.unit.Dp(24f).toPx()"));
        assert!(primitives.contains("androidx.compose.ui.layout.Layout("));
        assert!(primitives.contains("internal fun NexaPageIndicatorRow("));
        assert!(primitives.contains("private fun nexaPageIndicatorGeometry("));
        assert!(primitives.contains("val geometry = nexaPageIndicatorGeometry("));
        assert!(
            primitives.contains("val rowHeight = androidx.compose.ui.unit.Dp(32f).roundToPx()")
        );
        let page_indicator = primitives
            .split("// nexa-codegen: NexaPageIndicatorPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaNavigationChrome")
            .next()
            .unwrap_or_default();
        assert!(!page_indicator.contains(".clickable("));
        assert!(!page_indicator.contains("onClickLabel = \"Page ${index + 1}\""));
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
        assert!(
            primitives.contains(
                "val navigationIcon: @androidx.compose.runtime.Composable () -> Unit = {"
            )
        );
        assert!(primitives.contains("navigationIcon = navigationIcon,"));
        assert!(primitives.contains("actions()"));
        assert!(!primitives.contains("LocalNexaNavigationContent"));
        assert!(!primitives.contains("LocalNexaFormContentTopInset"));
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
    fn fixed_width_row_primitive_keeps_intrinsic_children_leading_aligned() {
        let primitives = shared_component_primitives();

        assert!(primitives.contains("preserveChildWidths: Boolean = false"));
        assert!(
            primitives.contains("modifier.wrapContentWidth(Alignment.Start, unbounded = true)")
        );
    }

    #[test]
    fn shared_presentation_primitives_use_one_fractional_sheet_path() {
        let primitives = shared_component_primitives();
        let bottom_sheet = primitives
            .split("// nexa-codegen: NexaBottomSheetPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaAlertDialogPrimitive")
            .next()
            .unwrap_or_default();

        assert!(bottom_sheet.contains("internal fun NexaBottomSheetPrimitive("));
        assert!(bottom_sheet.contains("shape = nexaBottomSheetShape(cornerRadius)"));
        assert!(primitives.contains("private fun nexaBottomSheetBackgroundColor("));
        assert!(primitives.contains("): androidx.compose.ui.graphics.Color = backgroundColor"));
        assert!(primitives.contains("androidx.compose.foundation.isSystemInDarkTheme()"));
        assert!(primitives.contains("androidx.compose.ui.graphics.Color.White"));
        assert!(primitives.contains("androidx.compose.ui.graphics.Color(0xFF1C1C1E)"));
        assert!(bottom_sheet.contains("color = resolvedBackgroundColor"));
        assert!(!bottom_sheet.contains("BottomSheetDefaults.ContainerColor"));
        assert!(primitives.contains("private fun NexaBottomSheetContentPrimitive("));
        assert_eq!(
            bottom_sheet
                .matches("NexaBottomSheetContentPrimitive(")
                .count(),
            3,
            "native modal, custom modal, and non-modal hosts share the same content layout"
        );
        let sheet_content = primitives
            .split("// nexa-codegen: NexaBottomSheetContentPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaBottomSheetPrimitive")
            .next()
            .unwrap_or_default();
        assert!(sheet_content.contains(".padding(contentPadding)"));
        assert!(sheet_content.contains("NexaColumnPrimitive("));
        assert!(
            sheet_content.contains("modifier = modifier.fillMaxWidth().padding(contentPadding)")
        );
        assert!(!sheet_content.contains("horizontalAlignment ="));
        assert!(!sheet_content.contains("contentAlignment ="));
        assert!(primitives.contains("content = content,"));
        let slider = primitives
            .split("// nexa-codegen: NexaSliderPrimitive")
            .nth(1)
            .unwrap_or_default()
            .split("// nexa-codegen: NexaProgressBarPrimitive")
            .next()
            .unwrap_or_default();
        assert!(slider.contains("awaitEachGesture"));
        assert!(slider.contains("drawCircle("));
        assert!(slider.contains("onEditingChanged?.invoke(false)"));
        assert!(bottom_sheet.contains("NexaCustomBottomSheetPrimitive("));
        assert!(bottom_sheet.contains("ModalBottomSheet("));
        assert!(bottom_sheet.contains("nativeValues.size == detents.size"));
        assert!(
            bottom_sheet.contains("initialValue = androidx.compose.material3.SheetValue.Hidden")
        );
        assert!(bottom_sheet.contains("WindowInsets.ime"));
        assert!(bottom_sheet.contains("animateIntAsState("));
        assert!(bottom_sheet.contains("targetValue = if (expanded) 0 else imeBottom"));
        assert!(
            bottom_sheet.contains(
                "scrimColor ?: androidx.compose.material3.BottomSheetDefaults.ScrimColor"
            )
        );
        assert!(bottom_sheet.contains("AnchoredDraggableState("));
        assert!(bottom_sheet.contains("initialValue = 0f,"));
        assert!(bottom_sheet.contains("sheetState.animateTo(initialDetent)"));
        assert!(bottom_sheet.contains("lastReportedSheetValue.floatValue"));
        assert!(bottom_sheet.contains("maxDetent - sheetState.currentValue"));
        assert!(bottom_sheet.contains("DraggableAnchors<Float>"));
        assert!(bottom_sheet.contains(".anchoredDraggable("));
        assert!(bottom_sheet.contains("sheetState.updateAnchors(anchors)"));
        assert!(bottom_sheet.contains("onDismiss: () -> Unit"));
        assert!(bottom_sheet.contains("currentOnDismiss()"));
        assert!(bottom_sheet.contains("retainedInComposition.value = false"));
        assert!(bottom_sheet.contains("LaunchedEffect(isPresented, sheetState)"));
        assert!(bottom_sheet.contains("sheetHeight.animateTo(0f"));
        assert!(bottom_sheet.contains("sheetState.animateTo(0f)"));
        assert!(bottom_sheet.contains("enabled = allowsDismissal,"));
        let non_modal_sheet = bottom_sheet
            .split("private fun NexaNonModalBottomSheetPrimitive(")
            .nth(1)
            .unwrap_or_default()
            .split("private fun NexaCustomBottomSheetPrimitive(")
            .next()
            .unwrap_or_default();
        assert!(non_modal_sheet.contains("androidx.compose.animation.core.Animatable(0f)"));
        assert!(non_modal_sheet.contains("if (isPresented)"));
        assert!(non_modal_sheet.contains("val target = initialHeight.coerceIn("));
        assert!(non_modal_sheet.contains("sheetHeight.value / maxDetent"));
        assert!(non_modal_sheet.contains("sheetHeight.animateTo("));
        for (animation, completion) in [
            (
                "sheetHeight.animateTo(0f",
                "currentOnExitAnimationFinished()",
            ),
            (
                "sheetState.animateTo(0f)",
                "currentOnExitAnimationFinished()",
            ),
        ] {
            let animation_start = bottom_sheet
                .find(animation)
                .expect("sheet close animation should be generated");
            let completion_start = bottom_sheet[animation_start..]
                .find(completion)
                .map(|offset| animation_start + offset)
                .expect("dismiss completion callback should follow the animation");
            assert!(animation_start < completion_start);
        }
        assert!(bottom_sheet.contains("DialogWindowProvider"));
        assert!(primitives.contains("Modifier.nexaBottomSheetDragHandleModifier()"));
        assert!(
            primitives
                .contains("val materialTopInset = androidx.compose.ui.unit.Dp(22f).roundToPx()")
        );
        assert!(
            primitives.contains("val indicatorTop = androidx.compose.ui.unit.Dp(5.5f).roundToPx()")
        );
        assert!(primitives.contains("layout(placeable.width, 0)"));
        assert!(primitives.contains("placeable.placeRelative(0, indicatorTop - materialTopInset)"));
        assert_eq!(
            bottom_sheet
                .matches("nexaBottomSheetDragHandleModifier()")
                .count(),
            3,
            "native, custom, and non-modal sheets use the compact native handle slot"
        );
        assert!(!bottom_sheet.contains("imeVisible"));
        assert!(bottom_sheet.contains("if (allowsBackgroundInteraction)"));
        assert!(bottom_sheet.contains("NexaNonModalBottomSheetPrimitive("));
        assert!(bottom_sheet.contains("androidx.activity.compose.BackHandler(enabled = true)"));
        assert!(bottom_sheet.contains("focusable = false"));
        assert!(bottom_sheet.contains("dismissOnBackPress = false"));
        assert!(bottom_sheet.contains("dismissOnClickOutside = false"));
        assert!(bottom_sheet.contains("shouldDismissOnClickOutside = allowsDismissal"));
        assert!(
            bottom_sheet
                .contains("dismissOnBackPress = allowsDismissal && shouldDismissOnBackPress")
        );
        assert!(bottom_sheet.contains(".fillMaxHeight()"));
        assert!(!bottom_sheet.contains("partialBodyHeight"));
        assert!(!bottom_sheet.contains("LocalNexaBottomSheetToolbarButtons"));

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
        assert!(alert_dialog.contains("content = dismissButton"));
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
                    accessibility_label: None,
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
                    gap: 12.0,
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
                            icon: SystemIcon::font_glyph("Mazicon", 61696)
                                .expect("custom font glyph"),
                            description: None,
                            title_color: None,
                            description_color: None,
                            icon_tint: None,
                            background: None,
                        },
                    ],
                },
                Node::ProgressBar {
                    progress: Some(Expr::Number {
                        raw: "0.45".to_owned(),
                        ty: nexa_ir::NumericType::Float64,
                    }),
                    style: None,
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
        assert!(kotlin.contains("internal fun NexaIconContent("));
        assert!(kotlin.contains("androidx.compose.runtime.remember(glyph)"));
        assert!(kotlin.contains("LocalDensity.current.fontScale).sp"));
        assert!(kotlin.contains("contentDescription = description.takeIf(String::isNotEmpty)"));
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
        assert!(kotlin.contains("size = androidx.compose.ui.unit.Dp(58f)"));
        assert!(kotlin.contains("NexaIconSpec(vector = Icons.Filled.Inbox)"));
        assert!(kotlin.contains("NexaIconSpec(glyph = 61696, fontResource = R.font.mazicon)"));
        assert!(kotlin.contains(
            "NexaProgressBarPrimitive(progress = 0.45.toFloat(), tint = null, trackColor = null, thickness = null, cornerRadius = null)"
        ));
        assert!(kotlin.contains("modifier.progressSemantics(normalizedProgress)"));
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
                            show_dividers: true,
                            reverse_layout: false,
                            page_snap: true,
                            item_extent: None,
                            index: "index".to_owned(),
                            key: None,
                            scroll_position: Some("currentPage".to_owned()),
                            component_ref: None,
                            children: vec![Node::Text {
                                value: Expr::String("Feed page".to_owned()),
                                style: TextStyle::default(),
                            }],
                            on_end_reached: None,
                            on_scroll: None,
                            on_scroll_begin_drag: None,
                            on_move: None,
                            swipe_actions: None,
                            sticky_header: None,
                            refresh: None,
                            gap: 0.0,
                            scroll_enabled: None,
                        },
                    }),
                },
                Node::BottomSheet {
                    component_ref: Expr::ComponentRefCreate {
                        namespace: "Nexa".to_owned(),
                        name: "BottomSheet".to_owned(),
                        identity: "comments-sheet".to_owned(),
                    },
                    content_sized: false,
                    content_padding: 0.0,
                    corner_radius: None,
                    detents: vec![0.5, 1.0],
                    initial_detent: None,
                    show_drag_indicator: true,
                    allows_dismissal: true,
                    allows_background_interaction: false,
                    should_dismiss_on_back_press: true,
                    background_color: None,
                    scrim_color: None,
                    on_dismiss: Vec::new(),
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
        assert!(kotlin.contains("verticalArrangement = Arrangement.Top"));
        assert!(
            kotlin.contains(
                "if (nexaListState0.isScrollInProgress) -1 else nexaListState0.settledPage"
            )
        );
        assert!(kotlin.contains("if (firstVisiblePosition >= 0)"));
        assert_eq!(kotlin.matches("@OptIn(").count(), 4);
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
                show_navigation_bar: true,
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
                    states: vec![
                        State {
                            name: "searchQuery".to_owned(),
                            ty: Type::String,
                            initial: Expr::String(String::new()),
                            mutable: true,
                        },
                        State {
                            name: "searchSuggestions".to_owned(),
                            ty: Type::Array(Box::new(Type::String)),
                            initial: Expr::Array(Vec::new()),
                            mutable: true,
                        },
                    ],
                    body: vec![
                        Node::Layout {
                            kind: LayoutKind::Column,
                            gap: 0.0,
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
                                        accessibility_label: None,
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
                show_navigation_bar: true,
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
        let navigation_host = kotlin
            .split_once("NavHost(")
            .expect("navigation host exists")
            .1
            .split_once("\n        ) {")
            .expect("navigation host arguments end before destinations")
            .0;
        assert!(navigation_host.contains("enterTransition = {"));
        assert!(navigation_host.contains("slideIntoContainer("));
        assert!(navigation_host.contains("AnimatedContentTransitionScope.SlideDirection.Start"));
        assert!(navigation_host.contains("slideOutOfContainer("));
        assert!(navigation_host.contains("AnimatedContentTransitionScope.SlideDirection.End"));
        assert!(navigation_host.contains("popEnterTransition = {"));
        assert!(navigation_host.contains("popExitTransition = {"));
        assert!(
            kotlin.contains("import androidx.compose.animation.AnimatedContentTransitionScope")
        );
        assert!(kotlin.contains("import androidx.compose.animation.core.tween"));
        let root = kotlin
            .split("composable(route = \"nexa_screen_0\")")
            .nth(1)
            .expect("root destination exists")
            .split("composable(route = \"nexa_screen_1\")")
            .next()
            .expect("root destination ends before the pushed destination");
        let destination = kotlin
            .split("fun nexaScreen1(")
            .nth(1)
            .expect("pushed destination composable exists");

        assert!(!root.contains("Scaffold("));
        assert!(destination.contains("NexaNavigationScreenPrimitive("));
        assert!(destination.contains("title = \"Task Details\""));
        assert!(destination.contains("inline = nexaTitleInline"));
        assert!(destination.contains("leadingTitle = nexaLeadingTitle"));
        assert!(destination.contains("hasTrailingActions = true"));
        assert!(!destination.contains("firstSection ="));
        assert!(destination.contains(
            "onBack = { if (navController.previousBackStackEntry != null) navController.popBackStack() }"
        ));
        assert!(kotlin.contains("class NexaRouteState1("));
        assert!(kotlin.contains("nexaBackStackEntry.viewModelStore.get(\"nexa-route-state-1\")"));
        assert!(kotlin.contains("var nexa_searchQuery by nexaRouteState1.nexa_searchQuery"));
        assert!(
            kotlin.contains("val nexa_searchSuggestions = nexaRouteState1.nexa_searchSuggestions")
        );
        assert!(kotlin.contains(
            "mutableStateListOf<String>().also { it.addAll(nexaInitialnexa_searchSuggestions) }"
        ));
        assert!(destination.contains("actions = {"));
        assert!(destination.contains("Button("));
        assert!(kotlin.contains("NexaAppearancePrimitive(mode = nexa_appearanceMode) {"));
        assert!(kotlin.contains("internal fun NexaNavigationScreenPrimitive("));
        assert!(kotlin.contains("LargeFlexibleTopAppBar("));
        assert!(kotlin.contains("CenterAlignedTopAppBar("));
        assert!(kotlin.contains("androidx.compose.material3.TopAppBar("));
        assert!(kotlin.contains("if (hasTrailingActions)"));
        assert!(kotlin.contains("Dp(16.toFloat())"));
        assert!(!kotlin.contains("LocalNexaFormContentTopInset"));
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
        assert!(large_destination.contains("nexaTitleInline = false"));

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
        assert!(centered_destination.contains("nexaLeadingTitle = false"));

        let mut leading_module = module.clone();
        let Node::NavigationStack {
            title_alignment, ..
        } = &mut leading_module.body[0]
        else {
            panic!("test module should have a navigation stack");
        };
        *title_alignment = nexa_ir::NavigationTitleAlignment::Leading;
        assert!(generate(&leading_module).contains("nexaLeadingTitle = true"));

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
        assert!(no_toolbar_destination.contains("nexaLeadingTitle = false"));
    }

    #[test]
    fn nested_navigation_stacks_reuse_finite_screen_composables() {
        let stack = || Node::NavigationStack {
            root: ScreenId(0),
            arguments: Vec::new(),
            show_navigation_bar: true,
            title_mode: nexa_ir::NavigationTitleMode::Inline,
            title_alignment: nexa_ir::NavigationTitleAlignment::Automatic,
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "NestedNavigation".to_owned(),
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
                body: vec![stack()],
                status_bar: None,
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            components: Vec::new(),
            body: vec![stack()],
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
        assert_eq!(kotlin.matches("fun nexaScreen0(").count(), 1);
        assert_eq!(
            kotlin
                .matches("composable(route = \"nexa_screen_0\")")
                .count(),
            2
        );
        assert_eq!(kotlin.matches("nexaScreen0(").count(), 3);
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
                gap: 0.0,
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
                    else_body: Some(vec![Node::Text {
                        value: Expr::String("no".to_owned()),
                        style: TextStyle::default(),
                    }]),
                    transition: Some(ViewTransition::SlideFromBottom),
                    transition_animation: Some(nexa_ir::AnimationSpec::Spring {
                        response: 0.35,
                        damping: 0.8,
                    }),
                },
                Node::When {
                    value: Expr::State("visible".to_owned(), Type::Bool),
                    cases: vec![
                        WhenCase {
                            value: Expr::Bool(true),
                            body: vec![Node::Text {
                                value: Expr::String("exhaustive true".to_owned()),
                                style: TextStyle::default(),
                            }],
                        },
                        WhenCase {
                            value: Expr::Bool(false),
                            body: vec![Node::Text {
                                value: Expr::String("exhaustive false".to_owned()),
                                style: TextStyle::default(),
                            }],
                        },
                    ],
                    else_body: None,
                    transition: None,
                    transition_animation: None,
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
        let false_case = kotlin
            .rfind("false -> {")
            .expect("the exhaustive boolean when emits its false case");
        let exhaustive_start = kotlin[..false_case]
            .rfind("when (")
            .expect("the exhaustive boolean when is emitted as a Kotlin when");
        let exhaustive_when = braced_block(&kotlin, exhaustive_start);
        assert!(!exhaustive_when.contains("else ->"));
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
                        accessibility_label: None,
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
                show_navigation_bar: true,
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
        let (_, project_features) = generate_units_with_project_features(&module);
        assert!(project_features.uses_saveable_state);
        let state_name = nexa_codegen::names::state_name("player");
        let app_state_name = nexa_codegen::names::state_name("appPlayer");
        let screen_function = kotlin
            .find("fun nexaScreen0(")
            .expect("screen body must be emitted in a reusable composable");
        let screen_player = kotlin
            .find(&format!(
                "val {state_name}: VideoPlayer = nexaRouteState0.{state_name}"
            ))
            .expect("screen native object must be held by its navigation entry");
        let screen_count = kotlin
            .find("var nexa_screenCount by rememberSaveable { mutableIntStateOf(1) }")
            .or_else(|| kotlin.find("var nexa_screenCount by nexaRouteState0.nexa_screenCount"))
            .expect("screen-local scalar state must survive route changes");
        let app_state = kotlin
            .find("var nexa_sharedCount by rememberSaveable { mutableIntStateOf(7) }")
            .expect("app scalar state must survive activity recreation above navigation");
        let nav_host = kotlin.find("NavHost(").expect("navigation host exists");
        assert!(screen_function < screen_player && screen_player < screen_count);
        assert!(screen_count < nav_host);
        assert!(app_state < nav_host);
        assert!(kotlin.contains("class NexaRouteState0("));
        assert!(kotlin.contains("nexaBackStackEntry.viewModelStore.get(\"nexa-route-state-0\")"));
        assert!(kotlin.contains(&format!(
            "val {app_state_name}: VideoPlayer = remember {{ VideoPlayer() }}"
        )));
        assert!(kotlin.contains("import androidx.compose.runtime.saveable.rememberSaveable"));
        assert!(kotlin.contains("composable(route = \"nexa_screen_1\")"));
        assert!(kotlin.contains("nexa_appPlayer.play()"));
        assert!(kotlin.contains("NexaNavigationLinkPrimitive("));
        assert!(kotlin.contains("onClick = { navController.navigate(\"nexa_screen_1\") }"));
        assert!(kotlin.contains("trailing = {"));
        assert!(kotlin.contains("Icons.Filled.ChevronRight"));
        assert!(kotlin.contains("Modifier.size(22.dp)"));
        assert!(kotlin.contains("onSurfaceVariant.copy(alpha = 0.5f)"));
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
                ref_methods: Vec::new(),
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            }],
            body: vec![Node::ComponentCall {
                name: "PlayerPanel".to_owned(),
                arguments: Vec::new(),
                component_ref: None,
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
                    ref_methods: Vec::new(),
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
                    ref_methods: Vec::new(),
                    on_appear: None,
                    on_appear_async: false,
                    on_disappear: None,
                },
            ],
            body: vec![Node::NavigationStack {
                root: ScreenId(0),
                arguments: Vec::new(),
                show_navigation_bar: true,
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
        let screen_function = kotlin
            .find("fun nexaScreen0(")
            .expect("busy screen composable was generated");
        let screen_scope = kotlin
            .find(scope_declaration)
            .expect("screen scope is declared");
        let nav_host = kotlin.find("NavHost(").expect("navigation host exists");
        assert!(
            screen_function < screen_scope && screen_scope < nav_host,
            "screen task scope must be declared in its reusable composable"
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
                    failure_type: None,
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
                    failure_type: None,
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
        assert!(
            kotlin.contains("public typealias NexaResult<T, E> = dev.nexa.core.NexaResult<T, E>")
        );
        assert!(kotlin.contains("NexaResult<Int, NexaAppError>"));
        assert!(kotlin.contains("dev.nexa.core.NexaResult.Success(42)"));
        assert!(kotlin.contains("nexa_fn_fetchCode().getOrThrow()"));
    }

    #[test]
    fn screen_result_state_emits_shared_result_definition() {
        // A screen-local `Result` state renders `NexaResult<…>` in the screen
        // composable, so the generated alias to the shared runtime class must be emitted even though
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
                show_navigation_bar: true,
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
            kotlin.contains("public typealias NexaResult<T, E> = dev.nexa.core.NexaResult<T, E>"),
            "screen-local Result state must pull in the shared runtime alias"
        );
        assert!(kotlin.contains(&format!(
            "val {state_name}: NexaResult<Int, NexaAppError> = dev.nexa.core.NexaResult.Success(0)"
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
