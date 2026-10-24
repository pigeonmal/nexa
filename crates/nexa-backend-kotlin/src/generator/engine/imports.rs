use super::features::Features;

/// Context shared by import contributors. Feature modules declare their own
/// imports; this type carries only cross-cutting placement facts.
pub(crate) struct ImportContext<'a> {
    pub(crate) features: &'a Features,
    pub(crate) include_all_shared_primitives: bool,
    pub(crate) uses_plugins: bool,
    pub(crate) has_navigation: bool,
    pub(crate) has_direction: bool,
    pub(crate) has_on_appear: bool,
    pub(crate) has_on_disappear: bool,
    pub(crate) has_lifecycle_events: bool,
    pub(crate) has_widgets: bool,
    pub(crate) has_immutable_structs: bool,
}

/// Deduplicates and orders imports after feature modules contribute them.
#[derive(Default)]
pub(crate) struct ImportSet {
    values: std::collections::BTreeSet<String>,
}

impl ImportSet {
    pub(crate) fn add(&mut self, enabled: bool, value: impl Into<String>) {
        if enabled {
            self.values.insert(value.into());
        }
    }

    pub(crate) fn render(self) -> String {
        let mut output = String::new();
        for value in self.values {
            output.push_str("import ");
            output.push_str(&value);
            output.push('\n');
        }
        output.push('\n');
        output
    }
}

pub(crate) fn render(context: ImportContext<'_>) -> String {
    let mut imports = ImportSet::default();
    imports.add(true, "androidx.compose.runtime.Composable");
    imports.add(true, "androidx.compose.runtime.CompositionLocalProvider");
    // DevRuntime can introduce navigation after a hot reload, so keep these
    // imports there. AOT imports stay feature-gated to avoid requiring the
    // optional Material icons artifact in apps that do not use navigation.
    imports.add(
        context.has_navigation || context.include_all_shared_primitives,
        "androidx.compose.material.icons.Icons",
    );
    imports.add(
        context.has_navigation || context.include_all_shared_primitives,
        "androidx.compose.material.icons.automirrored.filled.ArrowBack",
    );
    // The DevRuntime host includes every shared primitive so hot-reloaded
    // trees can use them even when the initial app does not. AOT source
    // sections are reachability-pruned, but share this import block.
    imports.add(true, "androidx.compose.foundation.background");
    imports.add(true, "androidx.compose.foundation.border");
    imports.add(true, "androidx.compose.foundation.clickable");
    imports.add(true, "androidx.compose.foundation.indication");
    imports.add(true, "androidx.compose.foundation.verticalScroll");
    imports.add(true, "androidx.compose.foundation.rememberScrollState");
    let uses_date_picker =
        context.include_all_shared_primitives || context.features.uses_date_picker;
    imports.add(uses_date_picker, "androidx.compose.ui.unit.dp");
    let uses_bottom_sheet =
        context.include_all_shared_primitives || context.features.uses_bottom_sheet;
    imports.add(uses_bottom_sheet, "androidx.compose.ui.unit.dp");
    imports.add(
        uses_bottom_sheet,
        "androidx.compose.foundation.gestures.anchoredDraggable",
    );
    imports.add(
        uses_bottom_sheet,
        "androidx.compose.foundation.gestures.detectVerticalDragGestures",
    );
    imports.add(
        uses_bottom_sheet,
        "androidx.compose.foundation.gestures.animateTo",
    );
    imports.add(
        uses_bottom_sheet,
        "androidx.compose.foundation.layout.imePadding",
    );
    imports.add(uses_bottom_sheet, "kotlinx.coroutines.launch");
    imports.add(
        uses_date_picker,
        "androidx.compose.foundation.gestures.awaitEachGesture",
    );
    imports.add(
        uses_date_picker,
        "androidx.compose.foundation.gestures.awaitFirstDown",
    );
    imports.add(
        uses_date_picker,
        "androidx.compose.ui.focus.focusProperties",
    );
    imports.add(
        uses_date_picker,
        "androidx.compose.ui.input.pointer.PointerEventPass",
    );
    imports.add(
        uses_date_picker,
        "androidx.compose.ui.input.pointer.pointerInput",
    );
    imports.add(
        uses_date_picker,
        "androidx.compose.ui.semantics.clearAndSetSemantics",
    );
    imports.add(uses_date_picker, "androidx.compose.ui.semantics.disabled");
    imports.add(uses_date_picker, "androidx.compose.ui.semantics.selected");
    imports.add(uses_date_picker, "androidx.compose.ui.draw.rotate");
    imports.add(
        uses_date_picker,
        "androidx.compose.foundation.layout.heightIn",
    );
    imports.add(
        uses_date_picker,
        "androidx.compose.foundation.layout.requiredWidthIn",
    );
    imports.add(uses_date_picker, "kotlinx.coroutines.flow.collect");
    imports.add(true, "androidx.compose.foundation.layout.defaultMinSize");
    imports.add(true, "androidx.compose.foundation.layout.Row");
    imports.add(true, "androidx.compose.foundation.layout.Box");
    imports.add(true, "androidx.compose.foundation.layout.fillMaxHeight");
    imports.add(true, "androidx.compose.foundation.layout.widthIn");
    // Shared component primitives are emitted in every AOT source unit and
    // the DevRuntime host. Their fully qualified WindowInsets companion
    // extensions and clipping modifier still require these extension imports
    // even when the app's initial module does not use those components.
    imports.add(true, "androidx.compose.foundation.layout.ime");
    imports.add(true, "androidx.compose.foundation.layout.navigationBars");
    imports.add(true, "androidx.compose.foundation.layout.statusBars");
    imports.add(true, "androidx.compose.foundation.layout.wrapContentWidth");
    imports.add(
        true,
        "androidx.compose.foundation.relocation.bringIntoViewRequester",
    );
    imports.add(true, "androidx.compose.foundation.selection.toggleable");
    imports.add(true, "androidx.compose.foundation.selection.selectable");
    imports.add(
        true,
        "androidx.compose.foundation.selection.selectableGroup",
    );
    imports.add(true, "androidx.compose.foundation.progressSemantics");
    imports.add(true, "androidx.compose.ui.focus.onFocusChanged");
    imports.add(true, "androidx.compose.ui.draw.clip");
    imports.add(true, "androidx.compose.ui.draw.alpha");
    imports.add(true, "androidx.compose.ui.Alignment");
    imports.add(true, "androidx.compose.ui.layout.layout");
    imports.add(true, "androidx.compose.foundation.layout.fillMaxWidth");
    imports.add(true, "androidx.compose.foundation.layout.fillMaxSize");
    imports.add(true, "androidx.compose.ui.graphics.luminance");
    imports.add(true, "androidx.compose.ui.graphics.toArgb");
    imports.add(true, "androidx.compose.foundation.layout.height");
    imports.add(true, "androidx.compose.foundation.layout.heightIn");
    imports.add(true, "androidx.compose.foundation.layout.offset");
    imports.add(true, "androidx.compose.foundation.layout.padding");
    imports.add(true, "androidx.compose.foundation.layout.requiredWidth");
    imports.add(true, "androidx.compose.foundation.layout.size");
    imports.add(true, "androidx.compose.foundation.layout.width");
    imports.add(true, "androidx.compose.ui.unit.sp");
    imports.add(true, "androidx.compose.ui.semantics.semantics");
    imports.add(true, "androidx.compose.ui.semantics.contentDescription");
    imports.add(true, "androidx.compose.ui.semantics.stateDescription");
    imports.add(true, "androidx.compose.ui.semantics.role");
    imports.add(true, "androidx.compose.ui.semantics.heading");
    imports.add(true, "androidx.compose.ui.semantics.Role");
    imports.add(
        context.has_immutable_structs,
        "androidx.compose.runtime.Immutable",
    );
    imports.add(context.has_widgets, "androidx.glance.appwidget.updateAll");
    imports.add(context.has_widgets, "kotlinx.coroutines.CoroutineScope");
    imports.add(context.has_widgets, "kotlinx.coroutines.Dispatchers");
    imports.add(context.has_widgets, "kotlinx.coroutines.launch");
    imports.add(
        context.uses_plugins
            || context.has_widgets
            || context.features.facts.capabilities.uses_clipboard_api
            || context.features.facts.capabilities.uses_storage_api
            || context.features.facts.capabilities.uses_localized_strings
            || context.features.facts.capabilities.uses_haptics_api
            || context
                .features
                .facts
                .capabilities
                .uses_screen_orientation_api,
        "androidx.compose.ui.platform.LocalContext",
    );

    crate::generator::components::direction::imports(&context, &mut imports);
    crate::generator::components::appearance::imports(context.features, &mut imports);
    crate::generator::components::lifecycle::imports(&context, &mut imports);
    crate::generator::components::status_bar::imports(context.features, &mut imports);
    crate::generator::components::text::imports(context.features, &mut imports);
    crate::generator::components::accessibility::imports(context.features, &mut imports);
    crate::generator::components::assets::imports(context.features, &mut imports);
    crate::generator::components::bottom_bar::imports(context.features, &mut imports);
    crate::generator::components::content_unavailable::imports(context.features, &mut imports);
    crate::generator::components::conditional::imports(context.features, &mut imports);
    crate::generator::components::controls::imports(context.features, &mut imports);
    crate::generator::components::dialogs::imports(context.features, &mut imports);
    crate::generator::components::images::imports(context.features, &mut imports);
    crate::generator::components::system_icons::imports(context.features, &mut imports);
    crate::generator::components::input::imports(context.features, &mut imports);
    crate::generator::components::keyboard::imports(context.features, &mut imports);
    crate::generator::components::layout::imports(context.features, &mut imports);
    crate::generator::components::links::imports(context.features, &mut imports);
    crate::generator::components::lists::imports(context.features, &mut imports);
    crate::generator::components::navigation::imports(&context, &mut imports);
    crate::generator::components::split_view::imports(&context, &mut imports);
    crate::generator::components::refresh::imports(context.features, &mut imports);
    crate::generator::components::shared_elements::imports(context.features, &mut imports);
    crate::generator::api::network::imports(context.features, &mut imports);
    crate::generator::api::json::imports(context.features, &mut imports);
    crate::generator::api::clipboard::imports(context.features, &mut imports);
    crate::generator::api::permissions::imports(context.features, &mut imports);
    crate::generator::api::storage::imports(context.features, &mut imports);
    crate::generator::engine::state::imports(context.features, &mut imports);
    crate::generator::engine::expressions::imports(context.features, &mut imports);

    imports.render()
}

#[cfg(test)]
mod tests {
    use super::{ImportContext, render};
    use crate::generator::engine::features::Features;

    fn render_features(features: &Features) -> String {
        render_features_with_shared_primitives(features, false)
    }

    fn render_features_with_shared_primitives(
        features: &Features,
        include_all_shared_primitives: bool,
    ) -> String {
        render(ImportContext {
            features,
            include_all_shared_primitives,
            uses_plugins: false,
            has_navigation: false,
            has_direction: false,
            has_on_appear: false,
            has_on_disappear: false,
            has_lifecycle_events: false,
            has_widgets: false,
            has_immutable_structs: false,
        })
    }

    #[test]
    fn minimal_app_keeps_material_default_press_feedback() {
        let imports = render_features(&Features::default());
        assert!(!imports.contains("import androidx.compose.material3.LocalRippleConfiguration\n"));
        assert!(imports.contains("import androidx.compose.runtime.Composable\n"));
        assert!(imports.contains("import androidx.compose.runtime.CompositionLocalProvider\n"));
        assert!(!imports.contains("import androidx.compose.material.icons.Icons\n"));
        assert!(!imports.contains("androidx.compose.material.icons.automirrored.filled.ArrowBack"));
        assert!(imports.contains("import androidx.compose.foundation.layout.ime\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.navigationBars\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.statusBars\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.wrapContentWidth\n"));
        assert!(
            imports
                .contains("import androidx.compose.foundation.relocation.bringIntoViewRequester\n")
        );
        assert!(imports.contains("import androidx.compose.foundation.selection.toggleable\n"));
        assert!(imports.contains("import androidx.compose.ui.focus.onFocusChanged\n"));
        assert!(imports.contains("import androidx.compose.ui.draw.clip\n"));
        assert!(!imports.contains("import androidx.compose.ui.input.pointer.pointerInput\n"));
        assert!(!imports.contains("import androidx.compose.ui.semantics.clearAndSetSemantics\n"));
    }

    #[test]
    fn date_picker_imports_only_its_extension_dependencies() {
        let mut date_picker_features = Features::default();
        date_picker_features.uses_date_picker = true;
        let imports = render_features(&date_picker_features);
        assert!(imports.contains("import androidx.compose.ui.unit.dp\n"));
        assert!(imports.contains("import kotlinx.coroutines.flow.collect\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.heightIn\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.requiredWidthIn\n"));
        assert!(imports.contains("import androidx.compose.ui.input.pointer.pointerInput\n"));
        assert!(imports.contains("import androidx.compose.ui.semantics.clearAndSetSemantics\n"));
        assert!(imports.contains("import androidx.compose.ui.semantics.disabled\n"));
        assert!(imports.contains("import androidx.compose.ui.semantics.selected\n"));
        assert!(imports.contains("import androidx.compose.ui.draw.rotate\n"));
        assert!(imports.contains("import androidx.compose.ui.focus.focusProperties\n"));
        assert!(!imports.contains("androidx.compose.material3.DatePickerDialog"));
        assert!(!imports.contains("androidx.compose.ui.window.DialogProperties"));
    }

    #[test]
    fn dev_runtime_keeps_shared_navigation_icon_imports_for_hot_reload() {
        let imports = render_features_with_shared_primitives(&Features::default(), true);

        assert!(imports.contains("import androidx.compose.material.icons.Icons\n"));
        assert!(
            imports
                .contains("import androidx.compose.material.icons.automirrored.filled.ArrowBack\n")
        );
    }

    #[test]
    fn fused_fast_list_refresh_imports_material_pull_to_refresh_box() {
        let mut features = Features::default();
        features.facts.ui.lists.refresh_fused = true;

        let imports = render_features(&features);

        assert!(
            imports.contains("import androidx.compose.material3.pulltorefresh.PullToRefreshBox\n")
        );
    }

    #[test]
    fn immutable_annotation_import_is_gated_to_proven_structs() {
        let mut context_features = Features::default();
        let baseline = render(ImportContext {
            features: &context_features,
            include_all_shared_primitives: false,
            uses_plugins: false,
            has_navigation: false,
            has_direction: false,
            has_on_appear: false,
            has_on_disappear: false,
            has_lifecycle_events: false,
            has_widgets: false,
            has_immutable_structs: false,
        });
        context_features.uses_network_api = true;
        let with_annotation = render(ImportContext {
            features: &context_features,
            include_all_shared_primitives: false,
            uses_plugins: false,
            has_navigation: false,
            has_direction: false,
            has_on_appear: false,
            has_on_disappear: false,
            has_lifecycle_events: false,
            has_widgets: false,
            has_immutable_structs: true,
        });

        assert!(!baseline.contains("androidx.compose.runtime.Immutable"));
        assert!(with_annotation.contains("import androidx.compose.runtime.Immutable\n"));
    }

    #[test]
    fn picker_imports_are_gated_to_the_native_menu_control() {
        let mut features = Features::default();
        features.uses_picker = true;
        let imports = render_features(&features);

        assert!(imports.contains("import androidx.compose.foundation.layout.width\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.padding\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.size\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.Box\n"));
        assert!(imports.contains("import androidx.compose.ui.unit.dp\n"));
        assert!(imports.contains("import androidx.compose.ui.graphics.Color\n"));
        assert!(imports.contains("import androidx.compose.material3.DropdownMenu\n"));
        assert!(imports.contains("import androidx.compose.material3.DropdownMenuItem\n"));
        assert!(!imports.contains("import androidx.compose.material3.AlertDialog\n"));
        assert!(imports.contains("import androidx.compose.foundation.clickable\n"));
        assert!(!imports.contains("import androidx.compose.material3.TextButton\n"));
        assert!(!imports.contains("import androidx.compose.material.icons.filled.UnfoldMore\n"));
        assert!(imports.contains("import androidx.compose.runtime.mutableStateOf\n"));
        assert!(imports.contains("import androidx.compose.runtime.remember\n"));
        assert!(!imports.contains("import androidx.compose.material3.SegmentedButton\n"));
    }

    #[test]
    fn text_input_imports_are_gated_to_text_input_usage() {
        let mut features = Features::default();
        features.uses_text_input = true;
        let imports = render_features(&features);

        assert!(imports.contains("import androidx.compose.foundation.text.KeyboardOptions\n"));
        assert!(imports.contains(
            "import androidx.compose.ui.text.input.KeyboardType as NativeKeyboardType\n"
        ));
        assert!(!imports.contains("TextFieldDefaults"));
        assert!(!render_features(&Features::default()).contains("NativeKeyboardType"));
    }

    #[test]
    fn clipboard_imports_are_gated_to_clipboard_usage() {
        let mut features = Features::default();
        features.facts.capabilities.uses_clipboard_api = true;
        let imports = render_features(&features);

        assert!(imports.contains("import android.content.ClipboardManager\n"));
        assert!(imports.contains("import androidx.compose.ui.platform.LocalContext\n"));
        assert!(!render_features(&Features::default()).contains("ClipboardManager"));
    }

    #[test]
    fn storage_imports_context_only_when_the_api_is_reachable() {
        let mut features = Features::default();
        features.facts.capabilities.uses_storage_api = true;
        let imports = render_features(&features);

        assert!(imports.contains("import android.content.Context\n"));
        assert!(imports.contains("import androidx.compose.ui.platform.LocalContext\n"));
        assert!(!render_features(&Features::default()).contains("LocalContext"));
    }

    #[test]
    fn haptics_imports_context_only_for_native_activity_binding() {
        let mut features = Features::default();
        features.facts.capabilities.uses_haptics_api = true;
        let imports = render_features(&features);

        assert!(imports.contains("import androidx.compose.ui.platform.LocalContext\n"));
        assert!(!imports.contains("ClipboardManager"));
        assert!(!render_features(&Features::default()).contains("LocalContext"));
    }

    #[test]
    fn local_images_do_not_pull_remote_image_or_network_imports() {
        let mut features = Features::default();
        features.uses_asset = true;
        let imports = render_features(&features);

        assert!(!imports.contains("import androidx.compose.foundation.Image\n"));
        assert!(!imports.contains("import coil3."));
        assert!(!imports.contains("import org.chromium.net."));
    }

    #[test]
    fn network_status_imports_context_without_cronet_transport() {
        let mut features = Features::default();
        features.uses_network_connectivity = true;
        let imports = render_features(&features);

        assert!(imports.contains("import android.content.Context\n"));
        assert!(imports.contains("import androidx.compose.ui.platform.LocalContext\n"));
        assert!(!imports.contains("import org.chromium.net."));
        assert!(!imports.contains("import kotlinx.coroutines."));
    }

    #[test]
    fn task_handle_and_launch_imports_are_gated_to_their_usage() {
        let baseline = render_features(&Features::default());
        assert!(!baseline.contains("kotlinx.coroutines."));

        let mut features = Features::default();
        features.uses_tasks = true;
        let handle_imports = render_features(&features);
        assert!(handle_imports.contains("import kotlinx.coroutines.Job\n"));
        assert!(!handle_imports.contains("import kotlinx.coroutines.launch\n"));
        assert!(!handle_imports.contains("import kotlinx.coroutines.CoroutineStart\n"));
        assert!(!handle_imports.contains("import kotlinx.coroutines.Dispatchers\n"));
        assert!(!handle_imports.contains("import kotlinx.coroutines.CoroutineExceptionHandler\n"));
        assert!(
            !handle_imports.contains("import androidx.compose.runtime.rememberCoroutineScope\n")
        );

        features.uses_task_launch = true;
        let launch_imports = render_features(&features);
        assert!(launch_imports.contains("import kotlinx.coroutines.launch\n"));
        assert!(launch_imports.contains("import kotlinx.coroutines.CoroutineStart\n"));
        assert!(launch_imports.contains("import kotlinx.coroutines.Dispatchers\n"));
        assert!(launch_imports.contains("import kotlinx.coroutines.CoroutineExceptionHandler\n"));
        assert!(
            launch_imports.contains("import androidx.compose.runtime.rememberCoroutineScope\n")
        );
    }

    #[test]
    fn bottom_sheet_imports_cover_fractional_native_drag_anchors() {
        let mut features = Features::default();
        features.uses_bottom_sheet = true;
        let imports = render_features(&features);

        assert!(
            imports.contains("import androidx.compose.foundation.gestures.anchoredDraggable\n")
        );
        assert!(
            imports.contains(
                "import androidx.compose.foundation.gestures.detectVerticalDragGestures\n"
            )
        );
        assert!(imports.contains("import androidx.compose.foundation.gestures.animateTo\n"));
        assert!(imports.contains("import androidx.compose.foundation.layout.imePadding\n"));
        assert!(imports.contains("import androidx.compose.ui.unit.dp\n"));
        assert!(imports.contains("import kotlinx.coroutines.launch\n"));
        assert!(!render_features(&Features::default()).contains("kotlinx.coroutines.launch"));
    }

    #[test]
    fn remote_images_use_the_cronet_adapter_without_typed_upload_support() {
        let mut features = Features::default();
        features.uses_remote_image = true;
        let imports = render_features(&features);

        assert!(!imports.contains("import coil3.compose.AsyncImage\n"));
        assert!(imports.contains("import coil3.network.NetworkFetcher\n"));
        assert!(imports.contains("import org.chromium.net.CronetEngine\n"));
        assert!(!imports.contains("import androidx.compose.material.icons.filled."));
        assert!(!imports.contains("import org.chromium.net.UploadDataProvider\n"));
        assert!(!imports.contains("import org.chromium.net.UploadDataSink\n"));
    }

    #[test]
    fn double_tap_imports_combined_clickable_without_long_press() {
        let mut features = Features::default();
        features.uses_double_tap = true;
        let imports = render_features(&features);

        assert!(imports.contains("import androidx.compose.foundation.combinedClickable\n"));
        // Clickable is preloaded for the shared Picker primitive even when the
        // app's initial tree uses only a gesture-backed Pressable.
        assert!(imports.contains("import androidx.compose.foundation.clickable\n"));
        assert!(!imports.contains("import androidx.compose.ui.platform.LocalViewConfiguration\n"));
    }

    #[test]
    fn expression_module_owns_size_class_import() {
        let mut features = Features::default();
        features.uses_size_class = true;
        assert!(
            render_features(&features)
                .contains("import androidx.compose.ui.platform.LocalConfiguration\n")
        );
    }
}
