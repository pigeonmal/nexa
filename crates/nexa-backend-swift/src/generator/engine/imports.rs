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
    let mut imports = ImportSet::default();
    imports.add(true, "SwiftUI");
    crate::generator::components::controls::imports(features, &mut imports);
    crate::generator::components::list_runtime::imports(features, &mut imports);
    crate::generator::components::links::imports(features, &mut imports);
    crate::generator::api::network::imports(features, &mut imports);
    crate::generator::api::json::imports(features, &mut imports);
    crate::generator::api::number::imports(features, &mut imports);
    crate::generator::api::crypto::imports(features, &mut imports);
    crate::generator::api::clipboard::imports(features, &mut imports);
    crate::generator::api::permissions::imports(features, &mut imports);
    crate::generator::api::secure_storage::imports(features, &mut imports);
    crate::generator::api::storage::imports(features, &mut imports);
    crate::generator::api::time::imports(features, &mut imports);
    imports.add(
        features.facts.capabilities.uses_keyboard_api
            || features.facts.capabilities.uses_haptics_api
            || features.facts.capabilities.uses_screen_orientation_api,
        "UIKit",
    );
    imports.render()
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::generator::engine::features::Features;

    #[test]
    fn minimal_app_imports_only_the_native_ui_module() {
        assert_eq!(render(&Features::default()), "import SwiftUI\n\n");
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
        assert!(!render(&Features::default()).contains("import UIKit\n"));
    }

    #[test]
    fn storage_imports_foundation_only_when_the_api_is_reachable() {
        let mut features = Features::default();
        features.facts.capabilities.uses_storage_api = true;
        let imports = render(&features);

        assert!(imports.contains("import Foundation\n"));
        assert!(!imports.contains("import UIKit\n"));
        assert!(!render(&Features::default()).contains("import Foundation\n"));
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
