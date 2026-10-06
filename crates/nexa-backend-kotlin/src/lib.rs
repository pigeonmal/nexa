mod generator;

/// Source files and Android drawable resources generated for Glance widgets.
pub struct WidgetGeneratedSources {
    pub sources: nexa_codegen::GeneratedSources,
    /// Resource paths are relative to `src/main/res`, e.g. `drawable/nexa_icon.xml`.
    pub resources: Vec<nexa_codegen::SourceUnit>,
}

/// A typed widget IR value that cannot be rendered safely by the Kotlin backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WidgetGenerationError {
    pub widget: String,
    pub message: String,
}

impl std::fmt::Display for WidgetGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "widget `{}`: {}", self.widget, self.message)
    }
}

impl std::error::Error for WidgetGenerationError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KotlinProjectFeatures {
    pub uses_network: bool,
    pub uses_network_connectivity: bool,
    pub uses_remote_image: bool,
    pub uses_system_icons: bool,
    pub uses_coroutines: bool,
    pub uses_permission_request: bool,
    pub uses_navigation: bool,
    pub uses_bottom_bar: bool,
    pub uses_compose_animation: bool,
    pub uses_compose_graphics: bool,
    pub uses_lifecycle_events: bool,
    pub uses_background_tasks: bool,
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
    /// Generate Jetpack Glance widget target sources from typed Nexa widget
    /// declarations. Project generation owns manifest/provider registration.
    pub fn generate_widget_units(
        &self,
        module: &nexa_ir::Module,
    ) -> Result<WidgetGeneratedSources, WidgetGenerationError> {
        generator::generate_widget_units(module)
    }

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
