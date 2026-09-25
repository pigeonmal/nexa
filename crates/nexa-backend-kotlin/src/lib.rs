mod generator;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KotlinProjectFeatures {
    pub uses_network: bool,
    pub uses_remote_image: bool,
    pub uses_coroutines: bool,
    pub uses_permission_request: bool,
    pub uses_navigation: bool,
    pub uses_compose_graphics: bool,
    pub uses_lifecycle_events: bool,
}

pub struct KotlinBackend;

impl nexa_codegen::Backend for KotlinBackend {
    fn name(&self) -> &'static str {
        "kotlin"
    }

    fn file_extension(&self) -> &'static str {
        "kt"
    }

    fn generate(&self, module: &nexa_ir::Module) -> String {
        generator::generate(module)
    }
}

impl KotlinBackend {
    /// Generate the release host as one concatenated source string.
    pub fn generate_with_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (String, KotlinProjectFeatures) {
        let (sources, features) = generator::generate_units_with_project_features(module);
        (join(sources.into_files(&[], "")), features)
    }

    /// Generate the release host as separate compile units.
    ///
    /// The caller owns file assembly, because a host project must place the
    /// `package` declaration and any plugin imports ahead of the import block.
    pub fn generate_units_with_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (nexa_codegen::GeneratedSources, KotlinProjectFeatures) {
        generator::generate_units_with_project_features(module)
    }

    /// Generate the development host as one concatenated source string.
    pub fn generate_for_dev_with_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (String, KotlinProjectFeatures) {
        let (sources, features) = generator::generate_for_dev_units_with_project_features(module);
        (join(sources.into_files(&[], "")), features)
    }

    /// Generate the development host as separate compile units.
    pub fn generate_for_dev_units_with_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (nexa_codegen::GeneratedSources, KotlinProjectFeatures) {
        generator::generate_for_dev_units_with_project_features(module)
    }
}

/// Concatenates generated units into a single source string.
fn join(units: Vec<nexa_codegen::SourceUnit>) -> String {
    let mut source = String::new();
    for unit in units {
        source.push_str(&unit.contents);
    }
    source
}
