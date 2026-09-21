use nexa_ir::{LayoutKind, Module, ViewStyle};

mod colors;
mod components;
mod controls;
mod custom_components;
mod expressions;
mod features;
mod images;
mod input;
mod keyboard;
mod layout;
mod lists;
mod navigation;
mod state;
mod utils;

pub(super) fn generate(module: &Module) -> String {
    let uses_image = module.body.iter().any(features::contains_image)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(features::contains_image))
        || module
            .components
            .iter()
            .any(|component| component.body.iter().any(features::contains_image));
    let uses_placeholder = module.body.iter().any(features::contains_placeholder)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(features::contains_placeholder))
        || module
            .components
            .iter()
            .any(|component| component.body.iter().any(features::contains_placeholder));
    let uses_navigation_link = module
        .screens
        .iter()
        .any(|screen| screen.body.iter().any(features::contains_navigation_link))
        || module.components.iter().any(|component| {
            component
                .body
                .iter()
                .any(features::contains_navigation_link)
        });
    let uses_list = module.body.iter().any(features::contains_list)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(features::contains_list))
        || module
            .components
            .iter()
            .any(|component| component.body.iter().any(features::contains_list));
    let uses_keyboard_aware = module.body.iter().any(features::contains_keyboard_aware)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(features::contains_keyboard_aware))
        || module
            .components
            .iter()
            .any(|component| component.body.iter().any(features::contains_keyboard_aware));
    let uses_adaptive_color = features::uses_adaptive_color(module);
    let uses_font_size = features::uses_font_size(module);
    let mut out = String::new();
    if uses_placeholder {
        out.push_str("import androidx.compose.ui.res.painterResource\n");
    }
    if uses_image {
        out.push_str("import coil3.compose.AsyncImage\n");
    }
    if !module.screens.is_empty() {
        out.push_str("import androidx.navigation.compose.NavHost\nimport androidx.navigation.compose.composable\nimport androidx.navigation.compose.rememberNavController\n");
    }
    if uses_navigation_link {
        out.push_str("import androidx.compose.material3.TextButton\n");
    }
    if uses_list {
        out.push_str("import androidx.compose.foundation.lazy.LazyColumn\nimport androidx.compose.foundation.lazy.items\n");
    }
    if uses_keyboard_aware {
        out.push_str("import androidx.compose.foundation.rememberScrollState\nimport androidx.compose.foundation.verticalScroll\n");
    }
    if uses_adaptive_color {
        out.push_str("import androidx.compose.foundation.isSystemInDarkTheme\n");
    }
    if uses_font_size {
        out.push_str("import androidx.compose.ui.unit.sp\n");
    }
    out.push_str("import androidx.compose.foundation.background\nimport androidx.compose.foundation.clickable\nimport androidx.compose.foundation.layout.*\nimport androidx.compose.foundation.shape.RoundedCornerShape\nimport androidx.compose.foundation.text.KeyboardOptions\nimport androidx.compose.material3.Button\nimport androidx.compose.material3.Switch\nimport androidx.compose.material3.Text\nimport androidx.compose.material3.TextField\nimport androidx.compose.runtime.*\nimport androidx.compose.ui.Modifier\nimport androidx.compose.ui.draw.alpha\nimport androidx.compose.ui.draw.clip\nimport androidx.compose.ui.graphics.Color\nimport androidx.compose.ui.layout.ContentScale\nimport androidx.compose.ui.semantics.Role\nimport androidx.compose.ui.semantics.contentDescription\nimport androidx.compose.ui.semantics.semantics\nimport androidx.compose.ui.text.input.KeyboardCapitalization as NativeKeyboardCapitalization\nimport androidx.compose.ui.text.input.KeyboardType as NativeKeyboardType\nimport androidx.compose.ui.text.input.PasswordVisualTransformation\nimport androidx.compose.ui.unit.dp\n\n");
    out.push_str(&format!(
        "@Composable\nfun {}() {{\n",
        nexa_codegen::names::screen_name(&module.app_name)
    ));
    if uses_adaptive_color {
        out.push_str("    val nexaIsDarkTheme = isSystemInDarkTheme()\n");
    }
    for state in &module.states {
        let name = nexa_codegen::names::state_name(&state.name);
        if state.mutable {
            out.push_str(&format!(
                "    var {name} by remember {{ {} }}\n",
                state::kotlin_state_initializer(state)
            ));
        } else {
            out.push_str(&format!(
                "    val {name}: {} = {}\n",
                state.ty.kotlin(),
                expressions::expression(&state.initial)
            ));
        }
    }
    if !module.states.is_empty() {
        out.push('\n');
    }
    if module.body.len() == 1 {
        components::render_node(&module.body[0], module, 1, &mut out);
    } else {
        layout::render_layout(
            LayoutKind::View,
            0.0,
            &ViewStyle::default(),
            &module.body,
            module,
            1,
            &mut out,
        );
    }
    out.push_str("\n}\n");
    custom_components::render(module, &mut out);
    out
}
