use std::collections::HashSet;

use nexa_ir::{Module, Permission};

#[derive(Default)]
pub(crate) struct Features {
    pub(crate) uses_status_bar: bool,
    pub(crate) uses_bottom_bar: bool,
    pub(crate) uses_bottom_sheet: bool,
    pub(crate) uses_bottom_sheet_partial: bool,
    pub(crate) uses_refresh_control: bool,
    pub(crate) uses_refresh_scroll: bool,
    pub(crate) uses_asset: bool,
    pub(crate) uses_tab_icon: bool,
    pub(crate) uses_tab_badge: bool,
    pub(crate) uses_tab_badge_placeholder: bool,
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
    pub(crate) uses_placeholder: bool,
    pub(crate) uses_navigation_link: bool,
    pub(crate) uses_navigation_back: bool,
    pub(crate) uses_navigation_uri: bool,
    pub(crate) uses_link: bool,
    pub(crate) app_uses_link: bool,
    pub(crate) uses_accessibility: bool,
    pub(crate) uses_accessibility_role: bool,
    pub(crate) uses_accessibility_heading: bool,
    pub(crate) uses_accessibility_hint: bool,
    pub(crate) uses_list: bool,
    pub(crate) uses_linear_list: bool,
    pub(crate) uses_horizontal_list: bool,
    pub(crate) uses_grid_list: bool,
    pub(crate) uses_list_end_reached: bool,
    pub(crate) uses_linear_list_end_reached: bool,
    pub(crate) uses_grid_end_reached: bool,
    pub(crate) uses_list_scroll_position: bool,
    pub(crate) uses_linear_list_scroll_position: bool,
    pub(crate) uses_grid_scroll_position: bool,
    pub(crate) uses_list_scroll_events: bool,
    pub(crate) uses_linear_list_scroll_events: bool,
    pub(crate) uses_grid_scroll_events: bool,
    pub(crate) uses_sticky_header: bool,
    pub(crate) uses_keyboard_aware: bool,
    pub(crate) uses_keyboard_interactive: bool,
    pub(crate) app_uses_keyboard_interactive: bool,
    pub(crate) uses_adaptive_color: bool,
    pub(crate) uses_font_weight: bool,
    pub(crate) uses_text_sp: bool,
    pub(crate) uses_selectable_text: bool,
    pub(crate) uses_button: bool,
    pub(crate) uses_button_icon: bool,
    pub(crate) uses_button_loading: bool,
    pub(crate) uses_text_node: bool,
    pub(crate) uses_text_input: bool,
    pub(crate) uses_text_input_submit: bool,
    pub(crate) uses_focus: bool,
    pub(crate) uses_secure_text_input: bool,
    pub(crate) uses_capitalization: bool,
    pub(crate) uses_switch: bool,
    pub(crate) uses_pressable: bool,
    pub(crate) uses_haptic: bool,
    pub(crate) app_uses_haptic: bool,
    pub(crate) uses_clickable: bool,
    pub(crate) uses_long_press: bool,
    pub(crate) uses_column: bool,
    pub(crate) uses_row: bool,
    pub(crate) uses_box: bool,
    pub(crate) uses_alignment: bool,
    pub(crate) uses_size_class: bool,
    pub(crate) uses_arrangement: bool,
    pub(crate) uses_modifier: bool,
    pub(crate) uses_background: bool,
    pub(crate) uses_border: bool,
    pub(crate) uses_padding: bool,
    pub(crate) uses_width: bool,
    pub(crate) uses_height: bool,
    pub(crate) uses_width_in: bool,
    pub(crate) uses_height_in: bool,
    pub(crate) uses_corner_radius: bool,
    pub(crate) uses_opacity: bool,
    pub(crate) uses_animation: bool,
    pub(crate) uses_animation_spring: bool,
    pub(crate) uses_animation_ease_in: bool,
    pub(crate) uses_animation_ease_out: bool,
    pub(crate) uses_animation_ease_in_out: bool,
    pub(crate) uses_animation_linear: bool,
    pub(crate) uses_color: bool,
    pub(crate) uses_dp: bool,
    pub(crate) uses_mutable_state: bool,
    pub(crate) uses_native_class_instance: bool,
    pub(crate) uses_mutable_collection: bool,
    pub(crate) uses_mutable_list: bool,
    pub(crate) uses_mutable_set: bool,
    pub(crate) uses_mutable_map: bool,
    pub(crate) uses_mutable_int_state: bool,
    pub(crate) uses_mutable_long_state: bool,
    pub(crate) uses_mutable_float_state: bool,
    pub(crate) uses_mutable_double_state: bool,
    pub(crate) uses_mutable_generic_state: bool,
    pub(crate) uses_result: bool,
    /// Target-neutral facts from the single analysis pass. Renderers read
    /// per-scope data (such as focus bindings) from here instead of
    /// re-walking subtrees.
    pub(crate) facts: nexa_ir::facts::ModuleFacts,
    component_theme: HashSet<String>,
    components_using_navigation: HashSet<String>,
    components_using_link: HashSet<String>,
    components_using_haptic: HashSet<String>,
    components_using_keyboard_interactive: HashSet<String>,
}

impl Features {
    /// Derives backend flags from the shared single-pass analysis. The facts
    /// are computed once here and carried along for renderers that need
    /// per-scope data, so no walks happen anywhere else in the backend.
    pub(crate) fn analyze(module: &Module) -> Self {
        let facts = nexa_ir::facts::ModuleFacts::analyze(module);
        let mut features = Self {
            uses_status_bar: facts.ui.status_bar,
            uses_column: module.body.len() != 1
                || module.screens.iter().any(|screen| screen.body.len() > 1)
                || module
                    .components
                    .iter()
                    .any(|component| component.body.len() > 1),
            ..Self::default()
        };
        features.derive_from_facts(&facts);
        features.facts = facts;
        features
    }

    /// Pure derivation of every walked flag from shared facts.
    fn derive_from_facts(&mut self, facts: &nexa_ir::facts::ModuleFacts) {
        let capabilities = &facts.capabilities;
        self.uses_remote_image = capabilities.uses_remote_image;
        self.uses_network_api = capabilities.uses_network_api;
        self.uses_path_api = capabilities.uses_path_api;
        self.uses_file_api = capabilities.uses_file_api;
        self.uses_file_async = capabilities.uses_file_async;
        self.uses_result = capabilities.uses_result;
        self.uses_permissions = facts.permissions.present;
        self.uses_permission_request = facts.permissions.request;
        self.used_permissions = facts.permissions.used.clone();
        self.dynamic_permission = facts.permissions.dynamic;

        let ui = &facts.ui;
        let types = &facts.used_types;
        self.uses_bottom_bar = ui.bottom_bar.present;
        self.uses_tab_icon = ui.bottom_bar.tab_icon;
        self.uses_tab_badge = ui.bottom_bar.tab_badge;
        self.uses_tab_badge_placeholder = ui.bottom_bar.tab_badge_placeholder;
        self.uses_bottom_sheet = ui.bottom_sheet.present;
        self.uses_bottom_sheet_partial = ui.bottom_sheet.partial;
        self.uses_refresh_control = ui.refresh.present || ui.lists.refresh_fused;
        self.uses_refresh_scroll = ui.refresh.scroll_variant;
        self.uses_asset = ui.image.asset;
        self.uses_placeholder = ui.image.placeholder;

        let lists = &ui.lists;
        self.uses_sticky_header = lists.sticky_header || lists.section_header;
        self.uses_list_scroll_position = lists.scroll_position || lists.scroll_position_grid;
        self.uses_linear_list_scroll_position = lists.scroll_position;
        self.uses_grid_scroll_position = lists.scroll_position_grid;
        self.uses_list_scroll_events = lists.scroll_events || lists.scroll_events_grid;
        self.uses_linear_list_scroll_events = lists.scroll_events;
        self.uses_grid_scroll_events = lists.scroll_events_grid;
        self.uses_list = lists.vertical || lists.sectioned;
        self.uses_linear_list = lists.vertical || lists.sectioned;
        self.uses_horizontal_list = lists.horizontal;
        self.uses_grid_list = lists.grid;
        self.uses_list_end_reached = lists.end_reached || lists.end_reached_grid;
        self.uses_linear_list_end_reached = lists.end_reached;
        self.uses_grid_end_reached = lists.end_reached_grid;
        if lists.item_extent {
            self.uses_box = true;
            self.uses_modifier = true;
            self.uses_dp = true;
            self.uses_height = true;
            self.uses_width |= lists.horizontal;
        }

        let text = &ui.text;
        self.uses_text_node = text.present;
        self.uses_color |= text.color;
        self.uses_font_weight = text.font_weight;
        self.uses_text_sp = text.sp;
        self.uses_selectable_text = text.selectable;

        let button = &ui.button;
        self.uses_button = button.present;
        self.uses_button_icon = button.icon;
        self.uses_button_loading = button.loading;

        let input = &ui.text_input;
        self.uses_text_input = input.present;
        self.uses_text_input_submit = input.submit;
        self.uses_focus = input.focus;
        self.uses_modifier |= input.focus;
        self.uses_secure_text_input = input.secure;
        self.uses_capitalization = input.capitalization;

        self.uses_switch = ui.switch_present;
        self.uses_modifier |= ui.switch_present;

        let pressable = &ui.pressable;
        self.uses_pressable = pressable.present;
        self.uses_clickable = pressable.clickable;
        self.uses_long_press = pressable.long_press;
        self.uses_box |= pressable.present;
        self.uses_modifier |= pressable.present;

        let link_anywhere = ui.app.link || ui.components.values().any(|scope| scope.link);
        self.uses_link = link_anywhere;
        self.app_uses_link = ui.app.link;
        self.uses_box |= link_anywhere;
        self.uses_modifier |= link_anywhere;

        let accessibility = &ui.accessibility;
        self.uses_accessibility = accessibility.present;
        self.uses_accessibility_hint = accessibility.hint;
        self.uses_accessibility_role = accessibility.role;
        self.uses_accessibility_heading = accessibility.heading;
        self.uses_box |= accessibility.present;
        self.uses_modifier |= accessibility.present;

        self.uses_keyboard_aware = ui.keyboard_aware;
        let interactive_anywhere = ui.app.keyboard_interactive
            || ui
                .components
                .values()
                .any(|scope| scope.keyboard_interactive);
        self.uses_keyboard_interactive = interactive_anywhere;
        self.app_uses_keyboard_interactive = ui.app.keyboard_interactive;
        self.uses_column |= ui.keyboard_aware;
        self.uses_modifier |= ui.keyboard_aware;

        self.uses_column |= ui.bottom_bar.present;
        self.uses_modifier |= ui.bottom_bar.present;
        self.uses_padding |= ui.bottom_bar.present;

        self.uses_column |= ui.refresh.scroll_variant;
        self.uses_modifier |= ui.refresh.scroll_variant;

        let layout = &ui.layout;
        self.uses_column |= layout.column;
        self.uses_row = layout.row;
        self.uses_box |= layout.box_;
        self.uses_column |= layout.multi_child;
        if layout.spacing {
            self.uses_dp = true;
            self.uses_arrangement = true;
        }

        let style = &ui.style;
        self.uses_alignment = style.alignment;
        self.uses_padding |= style.padding;
        self.uses_width |= style.width;
        self.uses_height |= style.height;
        self.uses_width_in = style.width_in;
        self.uses_height_in = style.height_in;
        self.uses_background = style.background;
        self.uses_border = style.border;
        self.uses_corner_radius = style.corner_radius;
        self.uses_opacity = style.opacity;
        self.uses_animation = style.animation;
        self.uses_animation_spring = style.spring;
        self.uses_animation_ease_in = style.ease_in;
        self.uses_animation_ease_out = style.ease_out;
        self.uses_animation_ease_in_out = style.ease_in_out;
        self.uses_animation_linear = style.linear;
        self.uses_modifier |= style.modifier;
        self.uses_color |= style.color;
        self.uses_dp |= style.dp;

        let haptic_anywhere = ui.app.haptic || ui.components.values().any(|scope| scope.haptic);
        self.uses_haptic = haptic_anywhere;
        self.app_uses_haptic = ui.app.haptic;

        self.uses_adaptive_color = ui.app.adaptive_background
            || ui.app.adaptive_border
            || ui.app.adaptive_text
            || ui.components.values().any(|scope| {
                scope.adaptive_background || scope.adaptive_border || scope.adaptive_text
            });
        self.uses_size_class =
            ui.app.size_class || ui.components.values().any(|scope| scope.size_class);
        self.uses_navigation_link =
            ui.app.navigation || ui.components.values().any(|scope| scope.navigation);
        self.uses_navigation_back =
            ui.app.navigation_back || ui.components.values().any(|scope| scope.navigation_back);

        self.components_using_link = component_names(&ui.components, |scope| scope.link);
        self.components_using_haptic = component_names(&ui.components, |scope| scope.haptic);
        self.components_using_keyboard_interactive =
            component_names(&ui.components, |scope| scope.keyboard_interactive);
        let direct_navigation = component_names(&ui.components, |scope| scope.navigation);
        self.components_using_navigation = facts.component_calls.requiring(&direct_navigation);
        let direct_theme = component_names(&ui.components, |scope| {
            scope.adaptive_background || scope.adaptive_text
        });
        self.component_theme = facts.component_calls.requiring(&direct_theme);

        self.uses_native_class_instance = types.native_class_instance;
        let list_callbacks = lists.end_reached
            || lists.end_reached_grid
            || lists.scroll_events
            || lists.scroll_events_grid;
        self.uses_mutable_state = types.mutable_int32
            || types.mutable_int64
            || types.mutable_float32
            || types.mutable_float64
            || types.mutable_generic
            || list_callbacks;
        self.uses_mutable_int_state = types.mutable_int32 || list_callbacks;
        self.uses_mutable_long_state = types.mutable_int64;
        self.uses_mutable_float_state = types.mutable_float32;
        self.uses_mutable_double_state = types.mutable_float64;
        self.uses_mutable_collection = types.mutable_list || types.mutable_set || types.mutable_map;
        self.uses_mutable_list = types.mutable_list;
        self.uses_mutable_set = types.mutable_set;
        self.uses_mutable_map = types.mutable_map;
        self.uses_mutable_generic_state = types.mutable_generic;
        self.uses_navigation_uri = types.string_route_param;
        self.uses_native_library =
            self.uses_network_transport() || self.uses_path_api || self.uses_file_api;
    }

    pub(crate) fn uses_network_transport(&self) -> bool {
        self.uses_network_api || self.uses_remote_image
    }

    pub(crate) fn component_requires_system_theme(&self, name: &str) -> bool {
        self.component_theme.contains(name)
    }

    pub(crate) fn component_uses_link(&self, name: &str) -> bool {
        self.components_using_link.contains(name)
    }

    pub(crate) fn component_uses_haptic(&self, name: &str) -> bool {
        self.components_using_haptic.contains(name)
    }

    pub(crate) fn component_uses_keyboard_interactive(&self, name: &str) -> bool {
        self.components_using_keyboard_interactive.contains(name)
    }

    pub(crate) fn component_requires_navigation(&self, name: &str) -> bool {
        self.components_using_navigation.contains(name)
    }
}

/// Names of components whose scope observations satisfy a predicate.
fn component_names(
    components: &std::collections::BTreeMap<String, nexa_ir::facts::ScopeUi>,
    select: impl Fn(&nexa_ir::facts::ScopeUi) -> bool,
) -> std::collections::HashSet<String> {
    components
        .iter()
        .filter(|(_, scope)| select(scope))
        .map(|(name, _)| name.clone())
        .collect()
}
