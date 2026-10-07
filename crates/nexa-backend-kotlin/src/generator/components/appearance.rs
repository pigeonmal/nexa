use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    if !features.facts.ui.appearance {
        return;
    }
    imports.add(true, "androidx.compose.foundation.isSystemInDarkTheme");
    imports.add(true, "androidx.compose.material3.MaterialTheme");
    imports.add(true, "androidx.compose.material3.darkColorScheme");
    imports.add(true, "androidx.compose.material3.lightColorScheme");
    imports.add(true, "androidx.compose.ui.graphics.Color");
    imports.add(true, "androidx.compose.runtime.remember");
}
