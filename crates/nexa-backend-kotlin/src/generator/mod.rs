use nexa_codegen::{GeneratedSources, SourceUnits};
use nexa_ir::{LayoutKind, Module, ViewStyle};

mod api;
mod components;
mod engine;

use crate::generator::engine::types::kotlin_type;
pub(super) use api::{network, number, permissions};
use components::node_renderer as component_renderer;
pub(super) use components::{
    accessibility, assets, bottom_bar, controls, custom_components, dialogs, images, input,
    keyboard, layout, links, lists, navigation, refresh, sheets,
};
pub(super) use engine::{
    colors, expressions, features, functions, runtime, state, structs, utils, value,
};

fn project_features_from_analysis(
    module: &Module,
    features: &features::Features,
) -> crate::KotlinProjectFeatures {
    crate::KotlinProjectFeatures {
        uses_network: features.uses_network_transport(),
        uses_network_connectivity: features.uses_network_connectivity,
        uses_remote_image: features.uses_remote_image,
        uses_coroutines: features.uses_network_transport()
            || features.uses_file_async
            || features.uses_permission_request
            || features.uses_tasks
            || features.facts.capabilities.uses_secure_storage_api
            || !module.background_tasks.is_empty(),
        uses_permission_request: features.uses_permission_request,
        uses_navigation: !module.screens.is_empty(),
        uses_compose_animation: features.uses_conditional_transition
            || features.uses_shared_elements
            || state::module_uses_scoped_animation(module),
        uses_compose_graphics: features.uses_color
            || features.uses_asset
            || features.uses_tab_icon
            || features.uses_button_icon
            || features.uses_placeholder,
        uses_lifecycle_events: module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some(),
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
const COLLECTION_ITERATION_HELPERS: &str = r#"private inline fun <T> nexaSnapshotValues(values: Iterable<T>): Iterable<T> = values
private inline fun <T> nexaSnapshotValues(values: androidx.compose.runtime.snapshots.SnapshotStateList<T>): List<T> = values.toList()

private inline fun <K, V> nexaSnapshotEntries(values: Map<K, V>): Map<K, V> = values
private inline fun <K, V> nexaSnapshotEntries(values: androidx.compose.runtime.snapshots.SnapshotStateMap<K, V>): Map<K, V> = values.toMap()

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
                .any(|screen| screen.on_appear.is_some()),
        has_on_disappear: module.on_disappear.is_some()
            || module
                .screens
                .iter()
                .any(|screen| screen.on_disappear.is_some()),
        has_lifecycle_events: module.on_active.is_some()
            || module.on_inactive.is_some()
            || module.on_background.is_some(),
    });
    let value_codecs = nexa_codegen::value::collect(module);
    let json_types = nexa_codegen::value::collect_json_types(module);
    let mut units = SourceUnits::new("kt");
    units.set_imports(&imports);
    units.write("types", |out| {
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
            if features.uses_picker {
                controls::render_picker_helper(out);
            }
            if features.uses_bottom_sheet || features.uses_segmented_control {
                out.push_str("@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)\n");
            }
            if features.app_uses_keyboard_interactive {
                out.push_str("@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)\n");
            }
            if features.uses_sticky_header
                || features.uses_long_press
                || features.uses_double_tap
            {
                out.push_str("@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)\n");
            }
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
            || features.facts.capabilities.uses_screen_orientation_api
            || features.facts.capabilities.uses_storage_api
            || !module.plugins.is_empty()
            || !module.background_tasks.is_empty()
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
                } else if state.is_native_class_instance_binding() {
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
        || features.facts.capabilities.uses_clipboard_api
        || features.facts.capabilities.uses_screen_orientation_api
        || !module.plugins.is_empty()
        || !module.background_tasks.is_empty()
    {
        units.write("runtime", |out| {
            runtime::render(out, features.uses_permission_request);
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
    if features.uses_asset
        || features.uses_tab_icon
        || features.uses_button_icon
        || features.uses_placeholder
    {
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
    use super::generate;
    use nexa_ir::{
        Action, AnimationSpec, Component, Expr, Function, ImageScale, ImageSource, LayoutKind,
        Module, Node, NumericType, Screen, ScreenId, State, TextStyle, Type, ViewStyle,
        ViewTransition, WhenCase,
    };

    #[test]
    fn shared_image_elements_use_compose_scopes_across_navigation_screens() {
        let image = Node::Image {
            source: ImageSource::Asset("hero".to_owned()),
            description: "Hero".to_owned(),
            scale: ImageScale::Fit,
            placeholder: None,
            shared_element: Some(Expr::String("hero-image".to_owned())),
        };
        let module = Module {
            app_name: "SharedHero".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
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
    fn configured_spring_maps_response_and_damping_to_compose_physics() {
        let module = Module {
            app_name: "SpringAnimation".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
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
            is_async: false,
            is_throwing: false,
        };
        let module = Module {
            app_name: "CurrencyFormatting".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
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
            app_name: "ConditionalTransitions".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
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
            app_name: "DoubleTapApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
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
        assert!(
            kotlin.contains("@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)")
        );
        assert!(kotlin.contains("combinedClickable("));
        assert!(kotlin.contains("onDoubleClick = {"));
        assert!(!kotlin.contains("onLongClick = {"));
        assert!(!kotlin.contains("import androidx.compose.ui.input.pointer"));
    }

    #[test]
    fn drag_pressable_emits_pointer_input_and_typed_callback() {
        let module = Module {
            app_name: "DragApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
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
            app_name: "PinchApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: vec![state("zoom"), state("distance")],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Pressable {
                disabled: Expr::Bool(false),
                haptic: None,
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
                is_constructor: true,
            },
            mutable: false,
        };
        let module = Module {
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
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
    }

    #[test]
    fn component_native_class_instances_are_remembered_across_recomposition() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let module = Module {
            app_name: "PlayerApp".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
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
                        is_constructor: true,
                    },
                    mutable: false,
                }],
                body: Vec::new(),
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
    fn generates_result_and_try_in_kotlin() {
        let err_type = Type::Enum("AppError".to_owned());
        let module = Module {
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
                    is_async: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
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
                    is_async: false,
                    parameters: Vec::new(),
                    locals: Vec::new(),
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
                            is_constructor: false,
                        }),
                        value_type: Type::Numeric(NumericType::Int32),
                        error_type: err_type.clone(),
                    },
                },
            ],
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
}
