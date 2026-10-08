use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let enabled = features.uses_content_unavailable;
    imports.add(enabled, "androidx.compose.foundation.layout.Arrangement");
    imports.add(enabled, "androidx.compose.foundation.layout.Column");
    imports.add(enabled, "androidx.compose.foundation.layout.Spacer");
    imports.add(enabled, "androidx.compose.foundation.layout.fillMaxSize");
    imports.add(enabled, "androidx.compose.foundation.layout.height");
    imports.add(enabled, "androidx.compose.foundation.layout.padding");
    imports.add(enabled, "androidx.compose.foundation.layout.size");
    imports.add(enabled, "androidx.compose.material3.Icon");
    imports.add(enabled, "androidx.compose.material3.MaterialTheme");
    imports.add(enabled, "androidx.compose.material3.Text");
    imports.add(enabled, "androidx.compose.ui.Alignment");
    imports.add(enabled, "androidx.compose.ui.Modifier");
    imports.add(enabled, "androidx.compose.ui.text.font.FontWeight");
    imports.add(enabled, "androidx.compose.ui.text.style.TextAlign");
    imports.add(enabled, "androidx.compose.ui.unit.dp");
}
