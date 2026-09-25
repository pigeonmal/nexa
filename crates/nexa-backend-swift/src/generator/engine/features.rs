use std::collections::HashSet;

use nexa_ir::{Module, Permission, facts::ModuleFacts};

#[derive(Default)]
pub(crate) struct Features {
    /// Target-neutral facts from the single analysis pass. Renderers read
    /// per-scope data (such as focus bindings) from here instead of
    /// re-walking subtrees.
    pub(crate) facts: ModuleFacts,
    pub(crate) uses_fast_list: bool,
    pub(crate) uses_vertical_list: bool,
    pub(crate) uses_horizontal_list: bool,
    pub(crate) uses_grid_list: bool,
    pub(crate) uses_sectioned_list: bool,
    pub(crate) uses_sticky_header: bool,
    pub(crate) uses_scroll_events: bool,
    pub(crate) uses_link: bool,
    pub(crate) uses_navigation_back: bool,
    pub(crate) uses_remote_image: bool,
    pub(crate) uses_native_library: bool,
    pub(crate) uses_network_api: bool,
    pub(crate) uses_path_api: bool,
    pub(crate) uses_file_api: bool,
    pub(crate) uses_file_async: bool,
    pub(crate) uses_permissions: bool,
    pub(crate) uses_permission_request: bool,
    pub(crate) expose_permissions_to_dev_runtime: bool,
    pub(crate) used_permissions: HashSet<Permission>,
    pub(crate) dynamic_permission: bool,
    pub(crate) uses_haptic: bool,
    pub(crate) app_uses_adaptive_color: bool,
    pub(crate) app_uses_size_class: bool,
    components_using_adaptive_color: HashSet<String>,
    components_using_size_class: HashSet<String>,
}

impl Features {
    /// Derives backend flags from the shared single-pass analysis. The facts
    /// are computed once here and carried along for renderers that need
    /// per-scope data, so no walks happen anywhere else in the backend.
    pub(crate) fn analyze(module: &Module) -> Self {
        let facts = ModuleFacts::analyze(module);
        let mut features = Self::default();
        let capabilities = &facts.capabilities;
        features.uses_remote_image = capabilities.uses_remote_image;
        features.uses_network_api = capabilities.uses_network_api;
        features.uses_path_api = capabilities.uses_path_api;
        features.uses_file_api = capabilities.uses_file_api;
        features.uses_file_async = capabilities.uses_file_async;
        features.uses_native_library =
            features.uses_network_transport() || features.uses_path_api || features.uses_file_api;

        let lists = &facts.ui.lists;
        features.uses_fast_list = lists.any;
        features.uses_sectioned_list = lists.sectioned;
        features.uses_vertical_list = lists.vertical;
        features.uses_horizontal_list = lists.horizontal;
        features.uses_grid_list = lists.grid;
        features.uses_sticky_header = lists.sticky_header;
        features.uses_scroll_events = lists.scroll_events || lists.scroll_events_grid;

        let ui = &facts.ui;
        features.uses_link = ui.app.link || ui.components.values().any(|scope| scope.link);
        features.uses_navigation_back = ui.app.navigation_back;
        features.uses_haptic = ui.app.haptic || ui.components.values().any(|scope| scope.haptic);
        features.app_uses_adaptive_color =
            ui.app.adaptive_background || ui.app.adaptive_border || ui.app.adaptive_text;
        features.app_uses_size_class = ui.app.size_class;
        features.components_using_adaptive_color = ui
            .components
            .iter()
            .filter(|(_, scope)| {
                scope.adaptive_background || scope.adaptive_border || scope.adaptive_text
            })
            .map(|(name, _)| name.clone())
            .collect();
        features.components_using_size_class = ui
            .components
            .iter()
            .filter(|(_, scope)| scope.size_class)
            .map(|(name, _)| name.clone())
            .collect();

        features.uses_permissions = facts.permissions.present;
        features.uses_permission_request = facts.permissions.request;
        features.used_permissions = facts.permissions.used.clone();
        features.dynamic_permission = facts.permissions.dynamic;
        features.facts = facts;
        features
    }

    pub(crate) fn uses_network_transport(&self) -> bool {
        self.uses_network_api || self.uses_remote_image
    }

    pub(crate) fn uses_permission(&self, permission: Permission) -> bool {
        self.dynamic_permission || self.used_permissions.contains(&permission)
    }

    pub(crate) fn component_uses_adaptive_color(&self, name: &str) -> bool {
        self.components_using_adaptive_color.contains(name)
    }

    pub(crate) fn component_uses_size_class(&self, name: &str) -> bool {
        self.components_using_size_class.contains(name)
    }
}
