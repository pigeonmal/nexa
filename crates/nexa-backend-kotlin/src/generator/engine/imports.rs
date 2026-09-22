use super::features::Features;

/// Context shared by import contributors. Feature modules declare their own
/// imports; this type carries only cross-cutting placement facts.
pub(crate) struct ImportContext<'a> {
    pub(crate) features: &'a Features,
    pub(crate) has_navigation: bool,
    pub(crate) has_direction: bool,
    pub(crate) has_on_appear: bool,
    pub(crate) has_on_disappear: bool,
    pub(crate) has_lifecycle_events: bool,
}

/// Deduplicates and orders imports after feature modules contribute them.
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

pub(crate) fn render(context: ImportContext<'_>) -> String {
    let mut imports = ImportSet::default();
    imports.add(true, "androidx.compose.runtime.Composable");

    crate::generator::components::components::imports(&context, &mut imports);
    crate::generator::components::accessibility::imports(context.features, &mut imports);
    crate::generator::components::assets::imports(context.features, &mut imports);
    crate::generator::components::bottom_bar::imports(context.features, &mut imports);
    crate::generator::components::controls::imports(context.features, &mut imports);
    crate::generator::components::images::imports(context.features, &mut imports);
    crate::generator::components::input::imports(context.features, &mut imports);
    crate::generator::components::keyboard::imports(context.features, &mut imports);
    crate::generator::components::layout::imports(context.features, &mut imports);
    crate::generator::components::links::imports(context.features, &mut imports);
    crate::generator::components::lists::imports(context.features, &mut imports);
    crate::generator::components::navigation::imports(&context, &mut imports);
    crate::generator::components::refresh::imports(context.features, &mut imports);
    crate::generator::components::sheets::imports(context.features, &mut imports);
    crate::generator::api::network::imports(context.features, &mut imports);
    crate::generator::api::permissions::imports(context.features, &mut imports);
    crate::generator::engine::state::imports(context.features, &mut imports);

    imports.render()
}
