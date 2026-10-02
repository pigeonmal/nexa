use super::features::Features;

/// Context shared by import contributors. Feature modules declare their own
/// imports; this type carries only cross-cutting placement facts.
pub(crate) struct ImportContext<'a> {
    pub(crate) features: &'a Features,
    pub(crate) uses_plugins: bool,
    pub(crate) has_navigation: bool,
    pub(crate) has_direction: bool,
    pub(crate) has_on_appear: bool,
    pub(crate) has_on_disappear: bool,
    pub(crate) has_lifecycle_events: bool,
}

/// Deduplicates and orders imports after feature modules contribute them.
#[derive(Default)]
pub(crate) struct ImportSet {
    values: std::collections::BTreeSet<&'static str>,
}

impl ImportSet {
    pub(crate) fn add(&mut self, enabled: bool, value: &'static str) {
        if enabled {
            self.values.insert(value);
        }
    }

    pub(crate) fn render(self) -> String {
        let mut output = String::new();
        for value in self.values {
            output.push_str("import ");
            output.push_str(value);
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
    imports.add(true, "androidx.compose.material3.LocalRippleConfiguration");
    imports.add(
        context.uses_plugins
            || context.features.facts.capabilities.uses_clipboard_api
            || context.features.facts.capabilities.uses_storage_api
            || context.features.facts.capabilities.uses_haptics_api
            || context
                .features
                .facts
                .capabilities
                .uses_screen_orientation_api,
        "androidx.compose.ui.platform.LocalContext",
    );

    crate::generator::components::direction::imports(&context, &mut imports);
    crate::generator::components::lifecycle::imports(&context, &mut imports);
    crate::generator::components::status_bar::imports(context.features, &mut imports);
    crate::generator::components::text::imports(context.features, &mut imports);
    crate::generator::components::accessibility::imports(context.features, &mut imports);
    crate::generator::components::assets::imports(context.features, &mut imports);
    crate::generator::components::bottom_bar::imports(context.features, &mut imports);
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
    crate::generator::components::refresh::imports(context.features, &mut imports);
    crate::generator::components::sheets::imports(context.features, &mut imports);
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
        render(ImportContext {
            features,
            uses_plugins: false,
            has_navigation: false,
            has_direction: false,
            has_on_appear: false,
            has_on_disappear: false,
            has_lifecycle_events: false,
        })
    }

    #[test]
    fn minimal_app_scopes_native_default_ripple_suppression() {
        let imports = render_features(&Features::default());
        assert!(imports.contains("import androidx.compose.material3.LocalRippleConfiguration\n"));
        assert!(imports.contains("import androidx.compose.runtime.Composable\n"));
        assert!(imports.contains("import androidx.compose.runtime.CompositionLocalProvider\n"));
    }

    #[test]
    fn picker_imports_are_gated_to_the_native_menu_control() {
        let mut features = Features::default();
        features.uses_picker = true;
        let imports = render_features(&features);

        assert!(imports.contains("import androidx.compose.foundation.layout.Box\n"));
        assert!(imports.contains("import androidx.compose.material3.DropdownMenu\n"));
        assert!(imports.contains("import androidx.compose.material3.DropdownMenuItem\n"));
        assert!(imports.contains("import androidx.compose.material3.TextButton\n"));
        assert!(imports.contains("import androidx.compose.runtime.mutableStateOf\n"));
        assert!(imports.contains("import androidx.compose.runtime.remember\n"));
        assert!(!imports.contains("import androidx.compose.material3.SegmentedButton\n"));
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

        assert!(imports.contains("import androidx.compose.foundation.Image\n"));
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
    fn task_imports_are_gated_to_task_handle_usage() {
        let baseline = render_features(&Features::default());
        assert!(!baseline.contains("kotlinx.coroutines."));

        let mut features = Features::default();
        features.uses_tasks = true;
        let imports = render_features(&features);
        assert!(imports.contains("import kotlinx.coroutines.Job\n"));
        assert!(imports.contains("import kotlinx.coroutines.launch\n"));
        assert!(imports.contains("import kotlinx.coroutines.CoroutineStart\n"));
        assert!(imports.contains("import androidx.compose.runtime.rememberCoroutineScope\n"));
    }

    #[test]
    fn remote_images_use_the_cronet_adapter_without_typed_upload_support() {
        let mut features = Features::default();
        features.uses_remote_image = true;
        let imports = render_features(&features);

        assert!(imports.contains("import coil3.compose.AsyncImage\n"));
        assert!(imports.contains("import coil3.network.NetworkFetcher\n"));
        assert!(imports.contains("import org.chromium.net.CronetEngine\n"));
        assert!(!imports.contains("import org.chromium.net.UploadDataProvider\n"));
        assert!(!imports.contains("import org.chromium.net.UploadDataSink\n"));
    }

    #[test]
    fn double_tap_imports_combined_clickable_without_long_press() {
        let mut features = Features::default();
        features.uses_double_tap = true;
        let imports = render_features(&features);

        assert!(imports.contains("import androidx.compose.foundation.combinedClickable\n"));
        assert!(!imports.contains("import androidx.compose.foundation.clickable\n"));
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
