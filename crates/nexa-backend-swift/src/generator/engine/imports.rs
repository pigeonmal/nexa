use super::features::Features;

/// Collects and renders imports after feature modules have declared what they
/// need. Keeping ordering and deduplication here makes generated files stable.
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

pub(crate) fn render(features: &Features) -> String {
    let mut imports = ImportSet::default();
    imports.add(true, "SwiftUI");
    crate::generator::components::controls::imports(features, &mut imports);
    crate::generator::components::list_runtime::imports(features, &mut imports);
    crate::generator::components::links::imports(features, &mut imports);
    crate::generator::api::network::imports(features, &mut imports);
    crate::generator::api::permissions::imports(features, &mut imports);
    imports.render()
}
