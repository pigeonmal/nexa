use super::features::Features;

pub(super) fn render(features: &Features, has_navigation: bool, out: &mut String) {
    let mut imports = Vec::with_capacity(40);
    let mut add = |enabled: bool, import: &'static str| {
        if enabled {
            imports.push(import);
        }
    };

    add(
        features.uses_background,
        "androidx.compose.foundation.background",
    );
    add(
        features.uses_pressable,
        "androidx.compose.foundation.clickable",
    );
    add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.rememberScrollState",
    );
    add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.verticalScroll",
    );
    add(
        features.uses_adaptive_color,
        "androidx.compose.foundation.isSystemInDarkTheme",
    );
    add(
        features.uses_list,
        "androidx.compose.foundation.lazy.LazyColumn",
    );
    add(features.uses_list, "androidx.compose.foundation.lazy.items");

    add(features.uses_box, "androidx.compose.foundation.layout.Box");
    add(
        features.uses_column,
        "androidx.compose.foundation.layout.Column",
    );
    add(features.uses_row, "androidx.compose.foundation.layout.Row");
    add(
        features.uses_arrangement,
        "androidx.compose.foundation.layout.Arrangement",
    );
    add(
        features.uses_padding,
        "androidx.compose.foundation.layout.padding",
    );
    add(
        features.uses_width,
        "androidx.compose.foundation.layout.width",
    );
    add(
        features.uses_height,
        "androidx.compose.foundation.layout.height",
    );
    add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.imePadding",
    );

    add(
        features.uses_corner_radius,
        "androidx.compose.foundation.shape.RoundedCornerShape",
    );
    add(
        features.uses_text_input,
        "androidx.compose.foundation.text.KeyboardOptions",
    );
    add(features.uses_button, "androidx.compose.material3.Button");
    add(features.uses_switch, "androidx.compose.material3.Switch");
    add(features.uses_text, "androidx.compose.material3.Text");
    add(
        features.uses_text_input,
        "androidx.compose.material3.TextField",
    );

    add(
        features.uses_mutable_state,
        "androidx.compose.runtime.getValue",
    );
    add(
        features.uses_mutable_state,
        "androidx.compose.runtime.setValue",
    );
    add(
        features.uses_mutable_state,
        "androidx.compose.runtime.remember",
    );
    add(
        features.uses_mutable_int_state,
        "androidx.compose.runtime.mutableIntStateOf",
    );
    add(
        features.uses_mutable_long_state,
        "androidx.compose.runtime.mutableLongStateOf",
    );
    add(
        features.uses_mutable_float_state,
        "androidx.compose.runtime.mutableFloatStateOf",
    );
    add(
        features.uses_mutable_generic_state,
        "androidx.compose.runtime.mutableStateOf",
    );
    add(true, "androidx.compose.runtime.Composable");

    add(features.uses_modifier, "androidx.compose.ui.Modifier");
    add(features.uses_opacity, "androidx.compose.ui.draw.alpha");
    add(features.uses_corner_radius, "androidx.compose.ui.draw.clip");
    add(features.uses_color, "androidx.compose.ui.graphics.Color");
    add(
        features.uses_image,
        "androidx.compose.ui.layout.ContentScale",
    );
    add(
        features.uses_pressable,
        "androidx.compose.ui.semantics.Role",
    );
    add(
        features.uses_switch,
        "androidx.compose.ui.semantics.contentDescription",
    );
    add(
        features.uses_switch,
        "androidx.compose.ui.semantics.semantics",
    );
    add(
        features.uses_capitalization,
        "androidx.compose.ui.text.input.KeyboardCapitalization as NativeKeyboardCapitalization",
    );
    add(
        features.uses_text_input,
        "androidx.compose.ui.text.input.KeyboardType as NativeKeyboardType",
    );
    add(
        features.uses_secure_text_input,
        "androidx.compose.ui.text.input.PasswordVisualTransformation",
    );
    add(features.uses_dp, "androidx.compose.ui.unit.dp");
    add(features.uses_font_size, "androidx.compose.ui.unit.sp");
    add(
        features.uses_placeholder,
        "androidx.compose.ui.res.painterResource",
    );
    add(features.uses_image, "coil3.compose.AsyncImage");
    add(
        features.uses_navigation_link,
        "androidx.compose.material3.TextButton",
    );
    add(has_navigation, "androidx.navigation.compose.NavHost");
    add(has_navigation, "androidx.navigation.compose.composable");
    add(
        has_navigation,
        "androidx.navigation.compose.rememberNavController",
    );

    imports.sort_unstable();
    imports.dedup();
    for import in imports {
        out.push_str("import ");
        out.push_str(import);
        out.push('\n');
    }
    out.push('\n');
}
