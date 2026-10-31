use super::features::Features;

/// Collects and renders imports after feature modules have declared what they
/// need. Keeping ordering and deduplication here makes generated files stable.
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

pub(crate) fn render(features: &Features) -> String {
    render_with_dev_runtime(features, false)
}

pub(crate) fn render_with_dev_runtime(features: &Features, dev_runtime: bool) -> String {
    let mut imports = ImportSet::default();
    imports.add(true, "SwiftUI");
    imports.add(true, "Foundation");
    imports.add(true, "UIKit");
    imports.add(features.facts.ui.bottom_sheet.present, "Combine");
    imports.add(features.uses_scroll_header || dev_runtime, "Combine");
    imports.add(features.uses_widgets, "WidgetKit");
    imports.add(features.facts.ui.lists.reorderable, "Combine");
    crate::generator::components::controls::imports(features, &mut imports);
    crate::generator::components::list_runtime::imports(features, &mut imports);
    crate::generator::components::links::imports(features, &mut imports);
    crate::generator::api::network::imports(features, &mut imports);
    crate::generator::api::json::imports(features, &mut imports);
    crate::generator::api::number::imports(features, &mut imports);
    imports.add(features.facts.capabilities.uses_locale_api, "Foundation");
    crate::generator::api::crypto::imports(features, &mut imports);
    crate::generator::api::clipboard::imports(features, &mut imports);
    crate::generator::api::permissions::imports(features, &mut imports);
    crate::generator::api::secure_storage::imports(features, &mut imports);
    crate::generator::api::storage::imports(features, &mut imports);
    crate::generator::api::time::imports(features, &mut imports);
    imports.render()
}

#[cfg(test)]
mod tests {
    use super::{render, render_with_dev_runtime};
    use crate::generator::engine::features::Features;

    #[test]
    fn minimal_app_imports_root_safe_area_support() {
        assert_eq!(
            render(&Features::default()),
            "import Foundation\nimport SwiftUI\nimport UIKit\n\n"
        );
    }

    #[test]
    fn reorderable_lists_import_combine_for_deployment_target_compatible_row_state() {
        let mut features = Features::default();
        features.facts.ui.lists.reorderable = true;
        let imports = render(&features);

        assert!(imports.contains("import Combine\n"));
        assert!(!imports.contains("import Observation\n"));
    }

    #[test]
    fn shared_uikit_dependency_is_emitted_once_for_multiple_features() {
        let mut features = Features::default();
        features.uses_haptic = true;
        features.uses_fast_list = true;
        features.facts.capabilities.uses_clipboard_api = true;
        let imports = render(&features);

        assert_eq!(imports.matches("import UIKit\n").count(), 1);
        assert!(imports.contains("import SwiftUI\n"));
        assert!(render(&Features::default()).contains("import UIKit\n"));
    }

    #[test]
    fn shared_accent_controls_import_dynamic_system_blue() {
        let mut features = Features::default();
        features.facts.ui.button.present = true;
        let imports = render(&features);

        assert!(imports.contains("import UIKit\n"));
    }

    #[test]
    fn text_line_height_and_root_safe_area_support_import_uikit() {
        let mut features = Features::default();
        features.facts.ui.text.line_height = true;

        assert!(render(&features).contains("import UIKit\n"));
        assert!(render(&Features::default()).contains("import UIKit\n"));
        assert!(render_with_dev_runtime(&Features::default(), true).contains("import UIKit\n"));
    }

    #[test]
    fn minimal_apps_import_foundation_and_uikit_for_root_support() {
        let imports = render(&Features::default());

        assert!(imports.contains("import Foundation\n"));
        assert!(imports.contains("import UIKit\n"));
    }

    #[test]
    fn remote_image_imports_stay_separate_from_typed_network_imports() {
        let mut image_features = Features::default();
        image_features.uses_remote_image = true;
        let image_imports = render(&image_features);
        assert!(image_imports.contains("import Foundation\n"));
        assert!(image_imports.contains("import ImageIO\n"));
        assert!(!image_imports.contains("import CryptoKit\n"));

        let mut network_features = Features::default();
        network_features.uses_network_api = true;
        let network_imports = render(&network_features);
        assert!(network_imports.contains("import Foundation\n"));
        assert!(network_imports.contains("import CryptoKit\n"));
        assert!(!network_imports.contains("import Network\n"));
        assert!(!network_imports.contains("import ImageIO\n"));
    }

    #[test]
    fn network_status_imports_path_monitor_without_transport_crypto() {
        let mut features = Features::default();
        features.uses_network_connectivity = true;
        let imports = render(&features);

        assert!(imports.contains("import Foundation\n"));
        assert!(imports.contains("import Network\n"));
        assert!(!imports.contains("import CryptoKit\n"));
        assert!(!imports.contains("import Security\n"));
    }

    #[test]
    fn crypto_imports_are_gated_to_modules_that_use_crypto() {
        let mut features = Features::default();
        features.facts.capabilities.uses_crypto_api = true;
        let imports = render(&features);

        assert!(imports.contains("import CryptoKit\n"));
        assert!(imports.contains("import Security\n"));
    }
}
