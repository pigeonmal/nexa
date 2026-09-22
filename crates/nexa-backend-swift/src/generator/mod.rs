use std::collections::BTreeSet;

use nexa_ir::{LayoutKind, Module, Node, State, ViewStyle, walk::walk_ir};

mod accessibility;
mod bottom_bar;
mod colors;
mod components;
mod controls;
mod custom_components;
mod expressions;
mod features;
mod functions;
mod images;
mod input;
mod keyboard;
mod layout;
mod links;
mod list_runtime;
mod lists;
mod navigation;
mod network;
mod permissions;
mod refresh;
mod sheets;
mod structs;
mod utils;

pub(super) fn generate(module: &Module) -> String {
    let features = features::Features::analyze(module);
    let mut focus_bindings = collect_focus_bindings(&module.body);
    for screen in &module.screens {
        focus_bindings.extend(collect_focus_bindings(&screen.body));
    }
    let uses_fast_list = features.uses_fast_list;
    let mut out = if uses_fast_list || features.uses_remote_image || features.uses_haptic {
        String::from("import SwiftUI\nimport UIKit\n")
    } else {
        String::from("import SwiftUI\n\n")
    };
    if features.uses_native_library {
        out.push_str("import CryptoKit\nimport Foundation\n\n");
    } else if features.uses_link {
        out.push_str("import Foundation\n\n");
    }
    if features.uses_permissions {
        out.push_str("import AVFoundation\nimport Contacts\nimport CoreBluetooth\nimport CoreLocation\nimport EventKit\nimport Photos\nimport UserNotifications\n\n");
    }
    if uses_fast_list {
        out.push_str("\n@available(iOS 16.0, *)\n");
    }
    for declaration in &module.enums {
        out.push_str(&format!(
            "private enum {}: String {{\n",
            nexa_codegen::names::enum_name(&declaration.name)
        ));
        for case in &declaration.cases {
            out.push_str(&format!("    case {case}\n"));
        }
        out.push_str("}\n\n");
    }
    structs::render(module, &mut out);
    if !module.screens.is_empty() {
        out.push_str("private enum NexaNavigationRoute: Hashable {\n");
        for screen in &module.screens {
            out.push_str(&format!(
                "    case {}\n",
                nexa_codegen::names::navigation_case_name(screen.id)
            ));
        }
        out.push_str("}\n\n");
    }
    out.push_str(&format!(
        "public struct {}: View {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    for state in &module.states {
        if !state.mutable || focus_bindings.contains(&state.name) {
            continue;
        }
        let name = nexa_codegen::names::state_name(&state.name);
        out.push_str(&format!(
            "    @State private var {name}: {} = {}\n",
            state.ty.swift(),
            expressions::expression(&state.initial)
        ));
    }
    for binding in &focus_bindings {
        out.push_str(&format!(
            "    @FocusState private var {}: Bool\n",
            nexa_codegen::names::state_name(binding),
        ));
    }
    if !module.states.is_empty() {
        out.push('\n');
    }
    if features.app_uses_adaptive_color {
        out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n");
    }
    if features.app_uses_regular_width {
        out.push_str(
            "    @Environment(\\.horizontalSizeClass) private var nexaHorizontalSizeClass\n",
        );
    }
    if features.app_uses_adaptive_color || features.app_uses_regular_width {
        out.push('\n');
    }
    out.push_str("    public init() {}\n\n    public var body: some View {\n");
    if module.screens.is_empty() {
        render_immutable_state(&module.states, 2, &mut out);
    }
    if module.body.len() == 1 {
        components::render_node(&module.body[0], module, 2, &mut out);
    } else {
        layout::render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            &module.body,
            module,
            2,
            &mut out,
        );
    }
    render_direction_modifier(module.direction, 2, &mut out);
    render_on_appear_modifier(
        module.on_appear.as_deref(),
        module.on_appear_async,
        2,
        &mut out,
    );
    render_on_disappear_modifier(module.on_disappear.as_deref(), 2, &mut out);
    render_status_bar_modifiers(module.status_bar, 2, &mut out);
    out.push_str("\n    }\n");
    if !module.screens.is_empty() {
        for screen in &module.screens {
            navigation::render_screen_function(screen, module, 1, &mut out);
        }
    }
    out.push_str("}\n");
    custom_components::render(module, &features, &mut out);
    if uses_fast_list {
        list_runtime::render(&mut out);
    }
    if features.uses_native_library {
        network::render(&mut out, features.uses_remote_image);
    }
    if features.uses_permissions {
        permissions::render(&mut out);
    }
    functions::render(module, &mut out);
    out
}

fn collect_focus_bindings(nodes: &[Node]) -> BTreeSet<String> {
    let mut bindings = BTreeSet::new();
    walk_ir(
        nodes,
        &mut |node| {
            if let Node::TextInput {
                focused: Some(name),
                ..
            } = node
            {
                bindings.insert(name.clone());
            }
        },
        &mut |_| {},
    );
    bindings
}

fn render_direction_modifier(
    config: Option<nexa_ir::DirectionConfig>,
    depth: usize,
    out: &mut String,
) {
    let Some(config) = config else {
        return;
    };
    let direction = match config.style {
        nexa_ir::DirectionStyle::Ltr => "leftToRight",
        nexa_ir::DirectionStyle::Rtl => "rightToLeft",
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(&format!(".environment(\\.layoutDirection, .{direction})"));
}

pub(super) fn render_on_appear_modifier(
    actions: Option<&[nexa_ir::Action]>,
    asynchronous: bool,
    depth: usize,
    out: &mut String,
) {
    let Some(actions) = actions else {
        return;
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(if asynchronous {
        ".task {"
    } else {
        ".onAppear {"
    });
    if actions.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push('}');
}

pub(super) fn render_on_disappear_modifier(
    actions: Option<&[nexa_ir::Action]>,
    depth: usize,
    out: &mut String,
) {
    let Some(actions) = actions else {
        return;
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(".onDisappear {");
    if actions.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push('}');
}

pub(super) fn render_status_bar_modifiers(
    config: Option<nexa_ir::StatusBarConfig>,
    depth: usize,
    out: &mut String,
) {
    let Some(config) = config else {
        return;
    };
    if config.hidden {
        out.push('\n');
        utils::indent(out, depth);
        out.push_str(".statusBarHidden(true)");
    }
    let scheme = match config.style {
        nexa_ir::StatusBarStyle::Default => None,
        // Light status-bar content uses a dark color scheme so the native
        // status bar selects light foreground content.
        nexa_ir::StatusBarStyle::Light => Some("dark"),
        nexa_ir::StatusBarStyle::Dark => Some("light"),
    };
    if let Some(scheme) = scheme {
        out.push('\n');
        utils::indent(out, depth);
        out.push_str(&format!(".preferredColorScheme(.{scheme})"));
    }
    if let Some(background) = config.background {
        out.push('\n');
        utils::indent(out, depth);
        out.push_str(&format!(
            ".background(alignment: .top) {{ GeometryReader {{ proxy in {}.frame(height: proxy.safeAreaInsets.top) }}.ignoresSafeArea(edges: .top) }}",
            colors::expression(background)
        ));
    }
}

pub(super) fn render_immutable_state(states: &[State], depth: usize, out: &mut String) {
    let immutable = states
        .iter()
        .filter(|state| !state.mutable)
        .collect::<Vec<_>>();
    for state in immutable {
        utils::indent(out, depth);
        out.push_str(&format!(
            "let {}: {} = {}\n",
            nexa_codegen::names::state_name(&state.name),
            state.ty.swift(),
            expressions::expression(&state.initial)
        ));
    }
    if states.iter().any(|state| !state.mutable) {
        out.push('\n');
    }
}
