//! Target-neutral module facts computed in a single analysis pass.
//!
//! Backends must derive their feature flags from these facts instead of
//! rescanning the module: one traversal after reachability pruning produces
//! every observation, so dependency pruning stays deterministic across
//! targets. Backends add only target-specific derivations (imports, runtime
//! shapes, dev-mode overrides) on top.
//!
//! `capabilities::analyze` delegates here, so the core API rules have exactly
//! one implementation.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::capabilities::Capabilities;
use crate::walk::{
    contains_scrollable, walk_actions, walk_callback_actions, walk_expression, walk_ir,
};
use crate::{
    AccessibilityRole, AnimationSpec, ColorValue, Expr, ImageSource, KeyboardDismissMode, ListAxis,
    ListPlan, Module, Node, NumericType, Permission, PermissionOpKind, State, SystemIcon, Type,
    ViewStyle,
};

/// Everything a backend needs to know about a pruned module, computed once.
#[derive(Clone, Debug, Default)]
pub struct ModuleFacts {
    pub capabilities: Capabilities,
    pub used_types: TypeFacts,
    pub ui: UiFacts,
    pub permissions: PermissionFacts,
    pub component_calls: ComponentCallGraph,
    pub focus_bindings: BindingSet,
}

/// Type shapes observed in state declarations and route parameters.
#[derive(Clone, Debug, Default)]
pub struct TypeFacts {
    pub mutable_int32: bool,
    pub mutable_int64: bool,
    pub mutable_float32: bool,
    pub mutable_float64: bool,
    /// Mutable states outside the scalar and collection sets, excluding
    /// native class instance bindings (which take a separate path).
    pub mutable_generic: bool,
    pub mutable_list: bool,
    pub mutable_set: bool,
    pub mutable_map: bool,
    pub native_class_instance: bool,
    pub task_handle: bool,
    /// A screen route parameter is a `String` (navigation URI support).
    pub string_route_param: bool,
}

/// Permission usage observed anywhere in the module.
#[derive(Clone, Debug, Default)]
pub struct PermissionFacts {
    /// A `status` or `request` query exists.
    pub present: bool,
    /// A `request` query exists.
    pub request: bool,
    pub used: HashSet<Permission>,
    pub dynamic: bool,
}

/// Direct component-to-component calls, keyed by caller name in source order.
/// Every declared component has an entry, even with no outgoing calls.
#[derive(Clone, Debug, Default)]
pub struct ComponentCallGraph {
    pub edges: HashMap<String, Vec<String>>,
}

impl ComponentCallGraph {
    /// Every known component that transitively calls a seed: the shared
    /// reachability behind theme and navigation propagation. Unknown names
    /// resolve to no outgoing edges, exactly like a missing map entry.
    pub fn requiring(&self, seeds: &HashSet<String>) -> HashSet<String> {
        fn requires(
            name: &str,
            seeds: &HashSet<String>,
            edges: &HashMap<String, Vec<String>>,
            required: &mut HashSet<String>,
            completed: &mut HashSet<String>,
        ) -> bool {
            if completed.contains(name) {
                return required.contains(name);
            }
            completed.insert(name.to_owned());
            let needs = seeds.contains(name)
                || edges.get(name).is_some_and(|children| {
                    children
                        .iter()
                        .any(|child| requires(child, seeds, edges, required, completed))
                });
            if needs {
                required.insert(name.to_owned());
            }
            needs
        }

        let mut required = HashSet::new();
        let mut completed = HashSet::new();
        let mut universe: Vec<&String> = self.edges.keys().collect();
        universe.sort();
        for name in universe {
            requires(name, seeds, &self.edges, &mut required, &mut completed);
        }
        required
    }
}

/// Focus bindings per scope. Backends render focus machinery per scope, so a
/// module-wide union would emit declarations for bindings that do not exist
/// in that scope; the sets stay separated here.
#[derive(Clone, Debug, Default)]
pub struct BindingSet {
    pub app: BTreeSet<String>,
    pub screens: BTreeMap<String, BTreeSet<String>>,
    pub components: BTreeMap<String, BTreeSet<String>>,
}

/// UI observations that backends need split by scope: the app group (app
/// body plus all screen bodies) versus each named component. Swift gates
/// app-level environment values separately from per-component ones, while
/// Kotlin merges both into global flags and per-component sets.
#[derive(Clone, Debug, Default)]
pub struct ScopeUi {
    pub haptic: bool,
    pub adaptive_background: bool,
    pub adaptive_border: bool,
    pub adaptive_text: bool,
    pub size_class: bool,
    pub link: bool,
    pub navigation: bool,
    pub navigation_back: bool,
    pub keyboard_interactive: bool,
}

/// Target-neutral UI observations shared by every backend.
#[derive(Clone, Debug, Default)]
pub struct UiFacts {
    /// Observations over the app body and all screen bodies.
    pub app: ScopeUi,
    /// Observations per named component.
    pub components: BTreeMap<String, ScopeUi>,
    /// A status bar is configured on the app or any screen (structural).
    pub status_bar: bool,
    /// An appearance wrapper selects the system, light, or dark color scheme.
    pub appearance: bool,
    pub bottom_bar: BottomBarFacts,
    pub page_pager: bool,
    pub navigation_split_view: bool,
    pub bottom_sheet: BottomSheetFacts,
    pub dialog_present: bool,
    /// A native empty-state presentation is used.
    pub content_unavailable: bool,
    pub refresh: RefreshFacts,
    pub image: ImageFacts,
    /// Native system icons used anywhere in generated UI.
    pub system_icons: BTreeSet<SystemIcon>,
    /// Native linear gradient nodes used anywhere in generated UI.
    pub linear_gradient: bool,
    pub lists: ListFacts,
    pub text: TextFacts,
    pub button: ButtonFacts,
    pub text_input: TextInputFacts,
    pub switch_present: bool,
    pub slider_present: bool,
    pub progress_bar_present: bool,
    pub progress_ring_present: bool,
    pub segmented_control_present: bool,
    pub picker_present: bool,
    pub date_picker_present: bool,
    /// A conditional view requests a native insertion/removal transition.
    pub conditional_transition: bool,
    pub pressable: PressableFacts,
    pub layout: LayoutFacts,
    pub style: StyleFacts,
    pub accessibility: AccessibilityFacts,
    pub keyboard_aware: bool,
    pub navigation_link: bool,
}

#[derive(Clone, Debug, Default)]
pub struct BottomBarFacts {
    pub present: bool,
    pub adaptive_tabs: bool,
    pub search_role: bool,
    pub searchable: bool,
    pub navigation_title: bool,
    pub tab_badge: bool,
}

#[derive(Clone, Debug, Default)]
pub struct BottomSheetFacts {
    pub present: bool,
    pub partial: bool,
    pub full_screen: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RefreshFacts {
    pub present: bool,
    /// A refresh wrapper whose children are not scrollable (generic pull to
    /// refresh instead of the native list integration).
    pub scroll_variant: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ImageFacts {
    pub asset: bool,
    pub placeholder: bool,
    pub shared_element: bool,
}

/// Virtualized-list observations. Axis bits count flat (count- and
/// collection-backed) plans only; sectioned plans always lay out vertically
/// and are reported separately because Swift emits a distinct sectioned
/// runtime while Kotlin folds them into its linear runtime.
#[derive(Clone, Debug, Default)]
pub struct ListFacts {
    pub any: bool,
    /// At least one list uses Nexa's custom virtualized implementation.
    pub virtualized: bool,
    pub sectioned: bool,
    pub vertical: bool,
    pub reverse_layout: bool,
    pub page_snap: bool,
    pub horizontal: bool,
    pub grid: bool,
    /// A flat sticky header (sectioned headers are reported separately).
    pub sticky_header: bool,
    pub section_header: bool,
    pub swipe_actions: bool,
    pub reorderable: bool,
    /// Scroll callbacks on non-grid flat lists; grid plans are reported
    /// separately because Kotlin emits distinct linear/grid runtimes.
    pub scroll_events: bool,
    pub scroll_events_grid: bool,
    pub scroll_position: bool,
    pub scroll_position_grid: bool,
    pub end_reached: bool,
    pub end_reached_grid: bool,
    pub item_extent: bool,
    pub refresh_fused: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TextFacts {
    pub present: bool,
    pub font_weight: bool,
    pub sp: bool,
    pub selectable: bool,
    pub color: bool,
    pub padding: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonFacts {
    pub present: bool,
    pub loading: bool,
    pub custom_shape: bool,
    pub glass: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TextInputFacts {
    pub present: bool,
    pub submit: bool,
    pub searchable: bool,
    pub focus: bool,
    pub secure: bool,
    pub capitalization: bool,
    pub autofill: bool,
    pub return_key: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PressableFacts {
    pub present: bool,
    pub fill_max_size: bool,
    pub long_press: bool,
    pub context_menu: bool,
    pub double_tap: bool,
    pub drag: bool,
    pub drag_velocity: bool,
    pub pinch: bool,
    /// A pressable without long-press actions (native clickable path).
    pub clickable: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutFacts {
    pub form: bool,
    pub column: bool,
    pub row: bool,
    /// Stack (overlay) layouts.
    pub box_: bool,
    pub spacing: bool,
    pub spacer: bool,
    pub divider: bool,
    /// A node with more than one child (implies a column container).
    pub multi_child: bool,
}

#[derive(Clone, Debug, Default)]
pub struct StyleFacts {
    pub alignment: bool,
    pub padding: bool,
    pub width: bool,
    pub height: bool,
    pub width_in: bool,
    pub height_in: bool,
    pub background: bool,
    pub border: bool,
    pub corner_radius: bool,
    pub opacity: bool,
    pub scale: bool,
    pub rotation: bool,
    pub shadow: bool,
    pub blur: bool,
    pub clip: bool,
    pub z_index: bool,
    pub animation: bool,
    pub spring: bool,
    pub ease_in: bool,
    pub ease_out: bool,
    pub ease_in_out: bool,
    pub linear: bool,
    pub dp: bool,
    pub modifier: bool,
    pub color: bool,
    pub dynamic_color: bool,
    pub glass: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AccessibilityFacts {
    pub present: bool,
    pub hint: bool,
    /// Roles that need a native role mapping (`Button` or `Image`).
    pub role: bool,
    /// Heading role.
    pub heading: bool,
}

/// Which scope a body walk attributes node observations to.
#[derive(Clone, Copy)]
enum Scope<'a> {
    App,
    Component(&'a str),
}

/// Collects all facts from the complete typed module.
///
/// This deliberately runs after semantic reachability pruning. Unused
/// functions and components therefore cannot pull optional native
/// dependencies into a generated project.
impl ModuleFacts {
    pub fn analyze(module: &Module) -> Self {
        let mut facts = ModuleFacts::default();

        // States across every scope: type shapes plus expression signals from
        // initializers. Size-class signals merge into the app group for app and
        // screen states, and into the owning component otherwise.
        for state in &module.states {
            record_state(state, &mut facts.used_types);
            walk_expression(&state.initial, &mut |expression| {
                observe_expr(
                    expression,
                    &mut facts.capabilities,
                    &mut facts.permissions,
                    &mut facts.ui.app.size_class,
                );
            });
        }
        for screen in &module.screens {
            for state in &screen.states {
                record_state(state, &mut facts.used_types);
                walk_expression(&state.initial, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut facts.ui.app.size_class,
                    );
                });
            }
            for parameter in &screen.parameters {
                facts.used_types.string_route_param |= matches!(parameter.ty, Type::String);
            }
        }
        for component in &module.components {
            for state in &component.states {
                record_state(state, &mut facts.used_types);
                let size_class = &mut facts
                    .ui
                    .components
                    .entry(component.name.clone())
                    .or_default()
                    .size_class;
                walk_expression(&state.initial, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        size_class,
                    );
                });
            }
        }

        // Functions and lifecycle actions carry capabilities and permissions but
        // never size-class signals.
        for function in &module.functions {
            for local in &function.locals {
                walk_expression(&local.initial, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut false,
                    );
                });
            }
            walk_expression(&function.body, &mut |expression| {
                observe_expr(
                    expression,
                    &mut facts.capabilities,
                    &mut facts.permissions,
                    &mut false,
                );
            });
            if let Some(actions) = &function.body_actions {
                walk_actions(actions, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut false,
                    );
                });
            }
        }
        for actions in module
            .on_appear
            .iter()
            .chain(module.on_disappear.iter())
            .chain(module.on_active.iter())
            .chain(module.on_inactive.iter())
            .chain(module.on_background.iter())
        {
            facts.capabilities.uses_network_connectivity |=
                actions_contain_network_status_subscription(actions);
            walk_actions(actions, &mut |expression| {
                observe_expr(
                    expression,
                    &mut facts.capabilities,
                    &mut facts.permissions,
                    &mut false,
                );
            });
        }
        for task in &module.background_tasks {
            facts.capabilities.uses_network_connectivity |=
                actions_contain_network_status_subscription(&task.actions);
            walk_actions(&task.actions, &mut |expression| {
                observe_expr(
                    expression,
                    &mut facts.capabilities,
                    &mut facts.permissions,
                    &mut false,
                );
            });
        }
        for screen in &module.screens {
            for actions in screen.on_appear.iter().chain(screen.on_disappear.iter()) {
                facts.capabilities.uses_network_connectivity |=
                    actions_contain_network_status_subscription(actions);
                walk_actions(actions, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut false,
                    );
                });
            }
        }
        for component in &module.components {
            for actions in component
                .on_appear
                .iter()
                .chain(component.on_disappear.iter())
            {
                facts.capabilities.uses_network_connectivity |=
                    actions_contain_network_status_subscription(actions);
                walk_actions(actions, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut false,
                    );
                });
            }
        }

        // Bodies: one walk per scope observes nodes and expressions together.
        {
            let mut size_hit = false;
            let mut remote_hit = false;
            walk_callback_actions(&module.body, &mut |actions| {
                facts.capabilities.uses_network_connectivity |=
                    actions_contain_network_status_subscription(actions);
            });
            walk_ir(
                &module.body,
                &mut |node| {
                    observe_node(
                        node,
                        &mut facts.ui,
                        Scope::App,
                        &mut facts.focus_bindings.app,
                        None,
                        &mut remote_hit,
                    );
                },
                &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut size_hit,
                    );
                },
            );
            facts.ui.app.size_class |= size_hit;
            facts.capabilities.uses_remote_image |= remote_hit;
        }
        for screen in &module.screens {
            let mut size_hit = false;
            let mut remote_hit = false;
            walk_callback_actions(&screen.body, &mut |actions| {
                facts.capabilities.uses_network_connectivity |=
                    actions_contain_network_status_subscription(actions);
            });
            walk_ir(
                &screen.body,
                &mut |node| {
                    observe_node(
                        node,
                        &mut facts.ui,
                        Scope::App,
                        facts
                            .focus_bindings
                            .screens
                            .entry(screen.name.clone())
                            .or_default(),
                        None,
                        &mut remote_hit,
                    );
                },
                &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut size_hit,
                    );
                },
            );
            facts.ui.app.size_class |= size_hit;
            facts.capabilities.uses_remote_image |= remote_hit;
        }
        for component in &module.components {
            let mut size_hit = false;
            let mut remote_hit = false;
            walk_callback_actions(&component.body, &mut |actions| {
                facts.capabilities.uses_network_connectivity |=
                    actions_contain_network_status_subscription(actions);
            });
            for actions in component
                .on_appear
                .iter()
                .chain(component.on_disappear.iter())
            {
                facts.capabilities.uses_network_connectivity |=
                    actions_contain_network_status_subscription(actions);
            }
            let calls = facts
                .component_calls
                .edges
                .entry(component.name.clone())
                .or_default();
            walk_ir(
                &component.body,
                &mut |node| {
                    observe_node(
                        node,
                        &mut facts.ui,
                        Scope::Component(&component.name),
                        facts
                            .focus_bindings
                            .components
                            .entry(component.name.clone())
                            .or_default(),
                        Some(calls),
                        &mut remote_hit,
                    );
                },
                &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut size_hit,
                    );
                },
            );
            if size_hit {
                facts
                    .ui
                    .components
                    .entry(component.name.clone())
                    .or_default()
                    .size_class = true;
            }
            facts.capabilities.uses_remote_image |= remote_hit;
        }

        // Widget UI is generated into a separate native target. Do not fold
        // its layout requirements into the app's SwiftUI/Compose feature set,
        // but do collect expression-level facts: localization resources,
        // plugin capabilities, and permissions are module-wide build inputs.
        // Providers and configuration defaults are expressions too, and may
        // contain localized expressions or native capability calls.
        for widget in &module.widgets {
            walk_expression(&widget.entry_provider, &mut |expression| {
                observe_expr(
                    expression,
                    &mut facts.capabilities,
                    &mut facts.permissions,
                    &mut false,
                );
            });
            if let Some(placeholder) = &widget.placeholder_provider {
                walk_expression(placeholder, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut false,
                    );
                });
            }
            if let Some(configuration) = &widget.configuration {
                walk_expression(&configuration.default, &mut |expression| {
                    observe_expr(
                        expression,
                        &mut facts.capabilities,
                        &mut facts.permissions,
                        &mut false,
                    );
                });
            }
            walk_ir(&widget.body, &mut |_| {}, &mut |expression| {
                observe_expr(
                    expression,
                    &mut facts.capabilities,
                    &mut facts.permissions,
                    &mut false,
                );
            });
        }

        facts.ui.status_bar = module.status_bar.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.status_bar.is_some());
        visit_result_types(module, &mut facts.capabilities);
        facts
    }
}

/// Mutable state type shapes. Mirrors the backend state-kind derivation so
/// every target classifies states identically.
fn record_state(state: &State, types: &mut TypeFacts) {
    if matches!(
        &state.ty,
        Type::Optional(inner) if matches!(inner.as_ref(), Type::TaskHandle)
    ) {
        types.task_handle = true;
    }
    if state.is_native_class_instance_binding() {
        types.native_class_instance = true;
        return;
    }
    if !state.mutable {
        return;
    }
    match state.ty {
        Type::Numeric(NumericType::Int32) => {
            types.mutable_int32 = true;
        }
        Type::Numeric(NumericType::Int64) => {
            types.mutable_int64 = true;
        }
        Type::Numeric(NumericType::Float32) => {
            types.mutable_float32 = true;
        }
        Type::Numeric(NumericType::Float64) => {
            types.mutable_float64 = true;
        }
        Type::Array(_) => {
            types.mutable_list = true;
        }
        Type::Set(_) => {
            types.mutable_set = true;
        }
        Type::Map(_, _) => {
            types.mutable_map = true;
        }
        _ => {
            types.mutable_generic = true;
        }
    }
}

/// Node observations for one scope. Global UI groups accumulate across every
/// scope; scope atoms accumulate per scope; focus names and call edges are
/// collected alongside so no second walk is needed.
#[allow(clippy::too_many_arguments)]
fn observe_node(
    node: &Node,
    ui: &mut UiFacts,
    scope: Scope<'_>,
    focus: &mut BTreeSet<String>,
    calls: Option<&mut Vec<String>>,
    remote_hit: &mut bool,
) {
    fn scope_of<'a>(ui: &'a mut UiFacts, scope: Scope<'_>) -> &'a mut ScopeUi {
        match scope {
            Scope::App => &mut ui.app,
            Scope::Component(name) => ui.components.entry(name.to_owned()).or_default(),
        }
    }
    match node {
        Node::Appearance { .. } => ui.appearance = true,
        Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. } => {}
        Node::Layout {
            kind,
            spacing,
            style,
            ..
        } => {
            match kind {
                crate::LayoutKind::Column => ui.layout.column = true,
                crate::LayoutKind::Row => ui.layout.row = true,
                crate::LayoutKind::Stack => ui.layout.box_ = true,
            }
            if *spacing > 0.0 {
                ui.layout.spacing = true;
            }
            {
                let scope = scope_of(ui, scope);
                scope.adaptive_background |= style
                    .background
                    .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
                scope.adaptive_border |= style
                    .border_color
                    .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
                scope.adaptive_background |= style
                    .effects
                    .shadow
                    .is_some_and(|shadow| matches!(shadow.color, ColorValue::Adaptive { .. }));
            }
            observe_style(style, &mut ui.style);
        }
        Node::Text { style, .. } => {
            ui.text.present = true;
            ui.text.font_weight |= style.font_weight.is_some();
            ui.text.sp |= style.font_size.is_some()
                || style.line_height.is_some()
                || style.letter_spacing.is_some();
            ui.text.selectable |= style.selectable;
            ui.text.color |= style.color.is_some();
            ui.text.padding |= style.padding.is_some();
            ui.style.opacity |= style.opacity.is_some();
            observe_effects(&style.effects, &mut ui.style);
            ui.style.modifier |= style.opacity.is_some() || style.effects.has_modifiers();
            scope_of(ui, scope).adaptive_text |= style
                .color
                .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }))
                || style
                    .effects
                    .shadow
                    .is_some_and(|shadow| matches!(shadow.color, ColorValue::Adaptive { .. }));
        }
        Node::ContentUnavailable { icon, .. } => {
            ui.content_unavailable = true;
            ui.system_icons.insert(icon.clone());
        }
        Node::Spacer => ui.layout.spacer = true,
        Node::Divider { .. } => ui.layout.divider = true,
        Node::Button {
            icon,
            loading,
            shape,
            tint,
            glass,
            ..
        } => {
            ui.button.present = true;
            ui.button.loading |= loading.is_some();
            ui.button.custom_shape |= shape.is_some();
            ui.button.glass |= *glass;
            ui.style.dynamic_color |= matches!(tint, Some(crate::ColorExpression::Dynamic(_)));
            if let Some(icon) = icon {
                ui.system_icons.insert(icon.clone());
            }
        }
        Node::Form { children } => {
            ui.layout.form = true;
            record_child_layout(children, ui);
        }
        Node::FormSection { children, .. } => record_child_layout(children, ui),
        Node::TextInput {
            secure,
            capitalization,
            autofill,
            return_key,
            focused,
            searchable,
            actions,
            ..
        } => {
            ui.text_input.present = true;
            ui.text_input.submit |= !actions.is_empty();
            ui.text_input.searchable |= *searchable;
            if *searchable {
                ui.system_icons
                    .insert(SystemIcon::Shared("search".to_owned()));
            }
            ui.text_input.focus |= focused.is_some();
            ui.text_input.secure |= *secure;
            ui.text_input.capitalization |= capitalization.is_some();
            ui.text_input.autofill |= autofill.is_some();
            ui.text_input.return_key |= return_key.is_some();
            if let Some(name) = focused {
                focus.insert(name.clone());
            }
        }
        Node::Toolbar { children, .. } => {
            ui.layout.row = true;
            ui.layout.spacing = true;
            ui.style.alignment = true;
            record_child_layout(children, ui);
        }
        Node::Switch { .. } => {
            ui.switch_present = true;
        }
        Node::Slider { .. } => {
            ui.slider_present = true;
        }
        Node::ProgressBar { .. } => {
            ui.progress_bar_present = true;
        }
        Node::ProgressRing { .. } => {
            ui.progress_ring_present = true;
        }
        Node::SegmentedControl { .. } => {
            ui.segmented_control_present = true;
        }
        Node::Picker { icon, .. } => {
            ui.picker_present = true;
            if let Some(icon) = icon {
                ui.system_icons.insert(icon.clone());
            }
        }
        Node::DatePicker { .. } => {
            ui.date_picker_present = true;
        }
        Node::Image {
            source,
            placeholder,
            shared_element,
            ..
        } => {
            ui.image.asset |= matches!(source, ImageSource::Asset(_));
            ui.image.shared_element |= shared_element.is_some();
            *remote_hit |= matches!(
                source,
                ImageSource::RemoteUrl(_) | ImageSource::LocalFile(_)
            );
            ui.image.placeholder |= placeholder.is_some();
        }
        Node::SystemIcon { icon, tint, .. } => {
            ui.system_icons.insert(icon.clone());
            ui.style.dynamic_color |= matches!(tint, crate::ColorExpression::Dynamic(_));
        }
        Node::LinearGradient { .. } => ui.linear_gradient = true,
        Node::Pressable {
            children,
            haptic,
            fill_max_size,
            double_tap_actions,
            long_press_actions,
            context_menu,
            drag_parameters,
            drag_actions,
            pinch_parameter,
            ..
        } => {
            ui.pressable.present = true;
            ui.pressable.fill_max_size |= *fill_max_size;
            scope_of(ui, scope).haptic |= haptic.is_some();
            let long_press = !long_press_actions.is_empty();
            let double_tap = !double_tap_actions.is_empty();
            ui.pressable.context_menu |= !context_menu.is_empty();
            ui.pressable.long_press |= long_press;
            ui.pressable.double_tap |= double_tap;
            ui.pressable.drag |= drag_parameters.len() == 4;
            if drag_parameters.len() == 4 {
                ui.pressable.drag_velocity |= crate::walk::actions_reference_state(
                    drag_actions,
                    &[drag_parameters[2].as_str(), drag_parameters[3].as_str()],
                );
            }
            ui.pressable.pinch |= pinch_parameter.is_some();
            ui.pressable.clickable |= !long_press && !double_tap && context_menu.is_empty();
            record_child_layout(children, ui);
        }
        Node::NavigationLink { children, .. } => {
            ui.navigation_link = true;
            scope_of(ui, scope).navigation = true;
            record_child_layout(children, ui);
        }
        Node::NavigationSplitView {
            sidebar, detail, ..
        } => {
            ui.navigation_split_view = true;
            scope_of(ui, scope).navigation = true;
            record_child_layout(sidebar, ui);
            record_child_layout(detail, ui);
        }
        Node::NavigationBack { .. } => {
            scope_of(ui, scope).navigation_back = true;
        }
        Node::Link { children, .. } => {
            scope_of(ui, scope).link = true;
            record_child_layout(children, ui);
        }
        Node::Accessibility {
            hint,
            role,
            children,
            ..
        } => {
            ui.accessibility.present = true;
            ui.accessibility.hint |= hint.is_some();
            ui.accessibility.role |=
                matches!(role, AccessibilityRole::Button | AccessibilityRole::Image);
            ui.accessibility.heading |= matches!(role, AccessibilityRole::Header);
            record_child_layout(children, ui);
        }
        Node::KeyboardAware { dismiss, .. } => {
            ui.keyboard_aware = true;
            scope_of(ui, scope).keyboard_interactive |=
                matches!(dismiss, KeyboardDismissMode::Interactive);
        }
        Node::BottomSheet {
            partial,
            title,
            children,
            ..
        } => {
            ui.bottom_sheet.present = true;
            ui.bottom_sheet.partial |= *partial;
            ui.bottom_sheet.full_screen |= !*partial;
            ui.text.present |= title.is_some();
            record_child_layout(children, ui);
        }
        Node::Dialog { children, .. } => {
            ui.dialog_present = true;
            record_child_layout(children, ui);
        }
        Node::ConfirmationDialog { children, .. } => {
            ui.dialog_present = true;
            record_child_layout(children, ui);
        }
        Node::AppBottomBar { tint, tabs, .. } => {
            ui.bottom_bar.present = true;
            ui.bottom_bar.adaptive_tabs = true;
            ui.style.dynamic_color |= matches!(tint, Some(crate::ColorExpression::Dynamic(_)));
            for tab in tabs {
                ui.bottom_bar.search_role |= tab.role.as_deref() == Some("search");
                ui.bottom_bar.searchable |= tab.search_state.is_some();
                ui.bottom_bar.navigation_title |= tab.navigation_title.is_some();
                if tab.search_state.is_some() {
                    ui.text_input.present = true;
                    ui.text_input.searchable = true;
                    ui.system_icons
                        .insert(SystemIcon::Shared("search".to_owned()));
                }
                if let Some(icon) = &tab.icon {
                    ui.system_icons.insert(icon.clone());
                }
                ui.bottom_bar.tab_badge |= tab.badge.is_some();
                record_child_layout(&tab.children, ui);
            }
        }
        Node::PagePager { pages, .. } => {
            ui.page_pager = true;
            for page in pages {
                record_child_layout(page, ui);
            }
        }
        Node::RefreshControl { children, .. } => {
            ui.refresh.present = true;
            if !contains_scrollable(children) {
                ui.refresh.scroll_variant = true;
            }
            record_child_layout(children, ui);
        }
        Node::FastList { plan } => {
            let axis = plan.axis();
            let grid = matches!(axis, ListAxis::Grid { .. });
            ui.lists.any = true;
            ui.lists.virtualized |= !plan.native();
            ui.lists.reverse_layout |= plan.reverse_layout();
            ui.lists.page_snap |= plan.page_snap();
            ui.lists.sectioned |= matches!(plan, ListPlan::Sections { .. });
            match axis {
                ListAxis::Vertical => {
                    if !matches!(plan, ListPlan::Sections { .. }) {
                        ui.lists.vertical = true;
                    }
                }
                ListAxis::Horizontal => ui.lists.horizontal = true,
                ListAxis::Grid { .. } => ui.lists.grid = true,
            }
            ui.lists.sticky_header |= plan.sticky_header().is_some();
            ui.lists.section_header |= plan.section_header().is_some();
            ui.lists.swipe_actions |= plan.swipe_actions().is_some();
            ui.lists.reorderable |= plan.on_move().is_some();
            ui.lists.refresh_fused |= plan.refresh().is_some();
            if plan.scroll_position().is_some() {
                ui.lists.scroll_position |= !grid;
                ui.lists.scroll_position_grid |= grid;
            }
            if plan.on_scroll().is_some() {
                ui.lists.scroll_events |= !grid;
                ui.lists.scroll_events_grid |= grid;
            }
            if plan.on_end_reached().is_some() {
                ui.lists.end_reached |= !grid;
                ui.lists.end_reached_grid |= grid;
            }
            ui.lists.item_extent |= plan.item_extent().is_some();
            record_child_layout(plan.children(), ui);
        }
        Node::If {
            then_body,
            else_body,
            transition,
            ..
        } => {
            ui.conditional_transition |= transition.is_some();
            record_child_layout(then_body, ui);
            if let Some(else_body) = else_body {
                record_child_layout(else_body, ui);
            }
        }
        Node::When { transition, .. } => {
            ui.conditional_transition |= transition.is_some();
        }
        Node::Content | Node::NavigationStack { .. } | Node::NativeComponentCall { .. } => {}
        Node::ComponentCall { name, .. } => {
            if let Some(calls) = calls {
                calls.push(name.clone());
            }
        }
    }
}

/// More than one child implies a column container.
fn record_child_layout(children: &[Node], ui: &mut UiFacts) {
    if children.len() > 1 {
        ui.layout.multi_child = true;
    }
}

/// View-style observations shared by every layout node.
fn observe_style(style: &ViewStyle, facts: &mut StyleFacts) {
    facts.alignment |= style.alignment.is_some();
    facts.padding |= style.padding.is_some();
    facts.width |= style.width.is_some();
    facts.height |= style.height.is_some();
    facts.width_in |= style.min_width.is_some() || style.max_width.is_some();
    facts.height_in |= style.min_height.is_some() || style.max_height.is_some();
    facts.background |= style.background.is_some();
    facts.border |= style.border_color.is_some();
    facts.corner_radius |= style.corner_radius.is_some();
    facts.corner_radius |= style.border_color.is_some();
    facts.opacity |= style.opacity.is_some();
    observe_effects(&style.effects, facts);
    if let Some(animation) = style.animation {
        facts.animation = true;
        match animation {
            AnimationSpec::Spring { .. } => facts.spring = true,
            AnimationSpec::EaseIn => facts.ease_in = true,
            AnimationSpec::EaseOut => facts.ease_out = true,
            AnimationSpec::EaseInOut => facts.ease_in_out = true,
            AnimationSpec::Linear => facts.linear = true,
        }
    }
    facts.modifier |= style.has_modifiers();
    facts.color |= style.background.is_some() || style.border_color.is_some();
    facts.glass |= style.effects.glass.is_some();
    facts.dp |= style.padding.is_some()
        || style.width.is_some()
        || style.height.is_some()
        || style.min_width.is_some()
        || style.max_width.is_some()
        || style.min_height.is_some()
        || style.max_height.is_some()
        || style.corner_radius.is_some()
        || style.border_width.is_some()
        || style.effects.blur.is_some()
        || style.effects.shadow.is_some()
        || style.effects.clip_rounded.is_some();
}

fn observe_effects(effects: &crate::ViewEffects, facts: &mut StyleFacts) {
    facts.scale |= effects.scale.is_some();
    facts.rotation |= effects.rotation.is_some();
    facts.shadow |= effects.shadow.is_some();
    facts.blur |= effects.blur.is_some();
    facts.clip |= effects.clip_rounded.is_some();
    facts.z_index |= effects.z_index.is_some();
    facts.corner_radius |= effects.clip_rounded.is_some();
    facts.color |= effects.shadow.is_some();
    facts.glass |= effects.glass.is_some();
    facts.dp |=
        effects.shadow.is_some() || effects.blur.is_some() || effects.clip_rounded.is_some();
}

/// Expression observations: capability signals, permission signals, and
/// size-class signals (merged into the scope by the caller).
fn actions_contain_network_status_subscription(actions: &[crate::Action]) -> bool {
    actions.iter().any(|action| match action {
        crate::Action::NetworkStatusSubscribe { .. } => true,
        crate::Action::NativeEventSubscribe { actions, .. } => {
            actions_contain_network_status_subscription(actions)
        }
        crate::Action::If {
            then_branch,
            else_branch,
            ..
        } => {
            actions_contain_network_status_subscription(then_branch)
                || else_branch
                    .as_deref()
                    .is_some_and(actions_contain_network_status_subscription)
        }
        crate::Action::For { body, .. }
        | crate::Action::ForMap { body, .. }
        | crate::Action::While { body, .. } => actions_contain_network_status_subscription(body),
        crate::Action::TryCatch {
            body,
            error_catches,
            catch_body,
        } => {
            actions_contain_network_status_subscription(body)
                || error_catches
                    .iter()
                    .any(|arm| actions_contain_network_status_subscription(&arm.body))
                || catch_body
                    .as_deref()
                    .is_some_and(actions_contain_network_status_subscription)
        }
        _ => false,
    })
}

fn observe_expr(
    expression: &Expr,
    capabilities: &mut Capabilities,
    permissions: &mut PermissionFacts,
    size_hit: &mut bool,
) {
    if matches!(expression, Expr::LocalizedText { .. }) {
        capabilities.uses_localized_strings = true;
    }
    if matches!(expression, Expr::ResultOk { .. } | Expr::ResultErr { .. }) {
        capabilities.uses_result = true;
    }
    if let Expr::NativeCall {
        return_type,
        codecs,
        ..
    } = expression
    {
        capabilities.uses_result |=
            type_uses_result(return_type) || codecs.iter().any(|codec| type_uses_result(&codec.ty));
    }
    // Validated core calls carry the same capability signal as their
    // `NativeCall` spelling; match both so lowering shapes cannot hide
    // platform API usage from capability analysis.
    match expression {
        Expr::NetworkFetch(_) => {
            capabilities.uses_network_api = true;
        }
        Expr::NetworkDownload { .. } => {
            capabilities.uses_network_api = true;
        }
        Expr::FileReadText { .. } | Expr::FileWriteText { .. } | Expr::FileDelete { .. } => {
            capabilities.uses_file_api = true;
            capabilities.uses_file_async = true;
        }
        Expr::FileExists { .. } => {
            capabilities.uses_file_api = true;
        }
        Expr::PathJoin { .. } => {
            capabilities.uses_path_api = true;
        }
        Expr::TimeCall { is_async, .. } => {
            capabilities.uses_time = true;
            capabilities.uses_result |= *is_async;
        }
        _ => {}
    }
    if let Expr::NativeCall {
        namespace, name, ..
    } = expression
    {
        match namespace.as_str() {
            "Network" => match name.as_str() {
                "isOnline" => capabilities.uses_network_connectivity = true,
                _ => capabilities.uses_network_api = true,
            },
            "Path" => capabilities.uses_path_api = true,
            "File" => {
                capabilities.uses_file_api = true;
                capabilities.uses_file_async |= name != "exists";
            }
            "Keyboard" => capabilities.uses_keyboard_api |= name == "dismiss",
            "Number" => capabilities.uses_number_formatting |= name == "formatCurrency",
            "Locale" => {
                capabilities.uses_locale_api = true;
            }
            "AppIcon" => capabilities.uses_app_icon_api |= name == "set",
            "Json" => capabilities.uses_json_api = true,
            "Crypto" => capabilities.uses_crypto_api = true,
            "SecureStorage" => capabilities.uses_secure_storage_api = true,
            "Storage" => capabilities.uses_storage_api = true,
            "Clipboard" => capabilities.uses_clipboard_api = true,
            "Haptics" => capabilities.uses_haptics_api = true,
            "Screen" => capabilities.uses_screen_orientation_api |= name == "lockOrientation",
            _ => {}
        }
    }
    if matches!(
        expression,
        Expr::NativeCall { namespace, name, .. }
        if namespace == "Permissions" && matches!(name.as_str(), "status" | "request")
    ) || matches!(expression, Expr::PermissionOp { .. })
    {
        permissions.present = true;
    }
    if matches!(
        expression,
        Expr::NativeCall { namespace, name, .. } if namespace == "Permissions" && name == "request"
    ) || matches!(
        expression,
        Expr::PermissionOp {
            op: PermissionOpKind::Request,
            ..
        }
    ) {
        permissions.request = true;
    }
    *size_hit |= matches!(
        expression,
        Expr::IsRegularWidth | Expr::IsCompactWidth | Expr::IsRegularHeight | Expr::IsCompactHeight
    );
    record_permission_usage(expression, permissions);
}

fn record_permission_usage(expression: &Expr, permissions: &mut PermissionFacts) {
    let permission = match expression {
        Expr::PermissionOp { permission, .. } => permission.as_ref(),
        _ => return,
    };
    match permission {
        Expr::EnumValue {
            enum_name,
            case_name,
            ..
        } if enum_name == "Permission" => match case_name.as_str() {
            "Camera" => {
                permissions.used.insert(Permission::Camera);
            }
            "Microphone" => {
                permissions.used.insert(Permission::Microphone);
            }
            "Photos" => {
                permissions.used.insert(Permission::Photos);
            }
            "Location" => {
                permissions.used.insert(Permission::Location);
            }
            "Notifications" => {
                permissions.used.insert(Permission::Notifications);
            }
            "Contacts" => {
                permissions.used.insert(Permission::Contacts);
            }
            "Calendar" => {
                permissions.used.insert(Permission::Calendar);
            }
            "Bluetooth" => {
                permissions.used.insert(Permission::Bluetooth);
            }
            "Motion" => {
                permissions.used.insert(Permission::Motion);
            }
            _ => permissions.dynamic = true,
        },
        _ => permissions.dynamic = true,
    }
}

/// Whether a declared type contains `Result` at any depth, including nested
/// collections and nested struct fields.
fn type_uses_result(ty: &Type) -> bool {
    match ty {
        Type::Result(_, _) => true,
        Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) | Type::Signal(inner) => type_uses_result(inner),
        Type::Map(key, value) | Type::Pair(key, value) => {
            type_uses_result(key) || type_uses_result(value)
        }
        Type::Triple(first, second, third) => {
            type_uses_result(first) || type_uses_result(second) || type_uses_result(third)
        }
        Type::Struct { fields, .. } => fields.iter().any(|(_, field)| type_uses_result(field)),
        _ => false,
    }
}

/// Visit every declared type position in the module exactly once. All
/// reachable expressions are already visited exactly once by the expression
/// walks above; result constructors are detected there.
fn visit_result_types(module: &Module, capabilities: &mut Capabilities) {
    for state in &module.states {
        capabilities.uses_result |= type_uses_result(&state.ty);
    }
    for declaration in &module.structs {
        capabilities.uses_result |= declaration
            .fields
            .iter()
            .any(|field| type_uses_result(&field.ty));
    }
    for function in &module.functions {
        capabilities.uses_result |= type_uses_result(&function.return_type);
        capabilities.uses_result |= function
            .parameters
            .iter()
            .any(|parameter| type_uses_result(&parameter.ty));
        capabilities.uses_result |= function
            .locals
            .iter()
            .any(|local| type_uses_result(&local.ty));
    }
    for screen in &module.screens {
        capabilities.uses_result |= screen
            .parameters
            .iter()
            .any(|parameter| type_uses_result(&parameter.ty));
        capabilities.uses_result |= screen
            .states
            .iter()
            .any(|state| type_uses_result(&state.ty));
    }
    for component in &module.components {
        capabilities.uses_result |= component
            .parameters
            .iter()
            .any(|parameter| type_uses_result(&parameter.ty));
        capabilities.uses_result |= component
            .states
            .iter()
            .any(|state| type_uses_result(&state.ty));
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleFacts;
    use crate::{
        Action, BottomBarTab, Expr, ImageScale, ImageSource, LayoutKind, Module, Node, SystemIcon,
        Type, ViewStyle,
    };

    fn empty_module(body: Vec<Node>) -> Module {
        Module {
            widgets: Vec::new(),
            app_name: "FactsTest".to_owned(),
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
            body,
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        }
    }

    #[test]
    fn core_api_signals_match_capability_analysis() {
        let call = |namespace: &str, name: &str| Expr::NativeCall {
            receiver: None,
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            arguments: Vec::new(),
            codecs: Vec::new(),
            return_type: crate::Type::String,
            is_async: true,
            is_throwing: false,
        };
        let module = empty_module(vec![Node::Layout {
            kind: LayoutKind::Column,
            spacing: 0.0,
            style: ViewStyle::default(),
            children: vec![Node::Button {
                label: Expr::String("Load".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                style: None,
                size: None,
                shape: None,
                tint: None,
                glass: false,
                actions: vec![
                    Action::Expression(call("Network", "fetch")),
                    Action::Expression(call("Path", "documents")),
                    Action::Expression(call("File", "readText")),
                    Action::Expression(call("Number", "formatCurrency")),
                    Action::Expression(call("SecureStorage", "get")),
                ],
            }],
        }]);

        let facts = ModuleFacts::analyze(&module);
        assert!(facts.capabilities.uses_network_api);
        assert!(facts.capabilities.uses_path_api);
        assert!(facts.capabilities.uses_file_api);
        assert!(facts.capabilities.uses_file_async);
        assert!(facts.capabilities.uses_number_formatting);
        assert!(facts.capabilities.uses_secure_storage_api);
        assert!(facts.ui.layout.column);
        assert!(facts.ui.button.present);
    }

    #[test]
    fn bottom_bar_system_symbols_enter_the_shared_icon_fact_set() {
        let module = empty_module(vec![Node::AppBottomBar {
            state: "selectedTab".to_owned(),
            tint: None,
            tabs: vec![BottomBarTab {
                index: 0,
                label: "Primary".to_owned(),
                comment: None,
                icon: Some(SystemIcon::Shared("inbox".to_owned())),
                badge: None,
                role: None,
                navigation_title: None,
                large_title: false,
                search_state: None,
                search_prompt: None,
                children: Vec::new(),
            }],
        }]);

        let facts = ModuleFacts::analyze(&module);

        assert!(
            facts
                .ui
                .system_icons
                .contains(&SystemIcon::Shared("inbox".to_owned()))
        );
    }

    #[test]
    fn scope_atoms_split_app_from_components() {
        let module = empty_module(vec![Node::Pressable {
            disabled: Expr::Bool(false),
            haptic: Some(crate::HapticStyle::Heavy),
            fill_max_size: false,
            children: Vec::new(),
            actions: Vec::new(),
            double_tap_actions: Vec::new(),
            long_press_duration_ms: crate::Expr::Number {
                raw: "500".to_owned(),
                ty: crate::NumericType::Int32,
            },
            long_press_actions: Vec::new(),
            context_menu: Vec::new(),
            drag_parameters: Vec::new(),
            drag_actions: Vec::new(),
            pinch_parameter: None,
            pinch_actions: Vec::new(),
        }]);
        let facts = ModuleFacts::analyze(&module);
        assert!(facts.ui.app.haptic);
        assert!(facts.ui.pressable.present);
        assert!(!facts.ui.pressable.long_press);
        assert!(facts.ui.components.is_empty());
    }

    #[test]
    fn sections_count_as_sectioned_not_vertical() {
        use crate::{ListPlan, SectionedListCommon};
        let plan = ListPlan::Sections {
            collection: Expr::Bool(true),
            element_type: Type::Bool,
            section: "section".to_owned(),
            item: "item".to_owned(),
            common: SectionedListCommon {
                native: false,
                item_extent: None,
                index: "index".to_owned(),
                key: None,
                children: Vec::new(),
                section_header: None,
                swipe_actions: None,
                refresh: None,
            },
        };
        let module = empty_module(vec![Node::FastList { plan }]);
        let facts = ModuleFacts::analyze(&module);
        assert!(facts.ui.lists.any);
        assert!(facts.ui.lists.sectioned);
        assert!(!facts.ui.lists.vertical);
        assert!(!facts.ui.lists.horizontal);
        assert!(!facts.ui.lists.grid);
    }

    #[test]
    fn remote_images_request_transport_without_counting_as_network_api_calls() {
        let module = empty_module(vec![Node::Image {
            source: ImageSource::RemoteUrl(Expr::String(
                "https://example.test/image.png".to_owned(),
            )),
            description: "Example image".to_owned(),
            scale: ImageScale::Fit,
            placeholder: None,
            max_height: None,
            shared_element: None,
        }]);
        let facts = ModuleFacts::analyze(&module);
        assert!(facts.capabilities.uses_remote_image);
        assert!(facts.capabilities.uses_network_transport());
        assert!(!facts.capabilities.uses_network_api);
        assert!(!facts.ui.image.placeholder);
    }

    #[test]
    fn mutable_states_classify_by_shape() {
        let mut module = empty_module(Vec::new());
        module.states.push(crate::State {
            name: "count".to_owned(),
            ty: Type::Numeric(crate::NumericType::Int32),
            initial: Expr::Bool(true),
            mutable: true,
        });
        module.states.push(crate::State {
            name: "tags".to_owned(),
            ty: Type::Array(Box::new(Type::String)),
            initial: Expr::Bool(true),
            mutable: true,
        });
        module.states.push(crate::State {
            name: "fixed".to_owned(),
            ty: Type::Numeric(crate::NumericType::Int64),
            initial: Expr::Bool(true),
            mutable: false,
        });
        let facts = ModuleFacts::analyze(&module);
        assert!(facts.used_types.mutable_int32);
        assert!(!facts.used_types.mutable_int64);
        assert!(facts.used_types.mutable_list);
        assert!(!facts.used_types.mutable_generic);
        assert!(!facts.used_types.native_class_instance);
    }

    #[test]
    fn permission_queries_record_cases() {
        let module = empty_module(vec![Node::Button {
            label: Expr::String("Go".to_owned()),
            icon: None,
            loading: None,
            disabled: None,
            style: None,
            size: None,
            shape: None,
            tint: None,
            glass: false,
            actions: vec![
                Action::Expression(Expr::PermissionOp {
                    op: crate::PermissionOpKind::Request,
                    permission: Box::new(Expr::EnumValue {
                        enum_name: "Permission".to_owned(),
                        case_name: "Camera".to_owned(),
                    }),
                }),
                Action::Expression(Expr::PermissionOp {
                    op: crate::PermissionOpKind::Request,
                    permission: Box::new(Expr::EnumValue {
                        enum_name: "Permission".to_owned(),
                        case_name: "Motion".to_owned(),
                    }),
                }),
            ],
        }]);
        let facts = ModuleFacts::analyze(&module);
        assert!(facts.permissions.present);
        assert!(facts.permissions.request);
        assert!(facts.permissions.used.contains(&crate::Permission::Camera));
        assert!(facts.permissions.used.contains(&crate::Permission::Motion));
        assert!(!facts.permissions.dynamic);
    }

    #[test]
    fn component_calls_and_focus_stay_per_scope() {
        use crate::Component;
        let callee = Component {
            name: "Callee".to_owned(),
            source_file: None,
            parameters: Vec::new(),
            states: Vec::new(),
            body: vec![Node::TextInput {
                state: "query".to_owned(),
                placeholder: String::new(),
                comment: None,
                keyboard: crate::KeyboardType::Text,
                secure: false,
                multiline: false,
                autofill: None,
                return_key: None,
                autocorrect: None,
                capitalization: None,
                focused: Some("query".to_owned()),
                max_length: None,
                font: None,
                min_lines: None,
                max_lines: None,
                searchable: false,
                actions: Vec::new(),
                on_change: None,
            }],
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
        };
        let caller = Component {
            name: "Caller".to_owned(),
            source_file: None,
            parameters: Vec::new(),
            states: Vec::new(),
            body: vec![Node::ComponentCall {
                name: "Callee".to_owned(),
                arguments: Vec::new(),
                children: None,
            }],
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
        };
        let mut module = empty_module(Vec::new());
        module.components.push(callee);
        module.components.push(caller);
        let facts = ModuleFacts::analyze(&module);
        assert_eq!(
            facts.component_calls.edges.get("Caller"),
            Some(&vec!["Callee".to_owned()])
        );
        assert!(
            facts
                .component_calls
                .requiring(&["Callee".to_owned()].into_iter().collect())
                .contains("Caller")
        );
        assert!(
            facts
                .focus_bindings
                .components
                .get("Callee")
                .is_some_and(|bindings| bindings.contains("query"))
        );
        assert!(facts.focus_bindings.app.is_empty());
    }
}
