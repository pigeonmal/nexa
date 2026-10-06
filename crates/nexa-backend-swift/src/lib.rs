mod generator;

pub use nexa_codegen::{GeneratedSources, SourceUnit};

/// Target-owned sources produced for one WidgetKit extension.
pub struct WidgetGeneratedSources {
    pub sources: GeneratedSources,
    pub resources: Vec<SourceUnit>,
}

/// A typed widget IR value that cannot be rendered safely by the Swift backend.
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

/// Native metadata consumed by iOS host project generation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SwiftProjectFeatures {
    pub requires_user_defaults_reason: bool,
    pub requires_file_timestamp_reason: bool,
    pub uses_screen_orientation_api: bool,
}

pub struct SwiftBackend;

impl SwiftBackend {
    /// Generate a development host with built-in async native API support.
    pub fn generate_for_dev(&self, module: &nexa_ir::Module) -> String {
        generator::generate_for_dev(module)
    }

    /// Generate the development host as one source unit per compile file.
    ///
    /// This is the structured form: each unit carries its own file name and
    /// complete contents, so a host project writer never has to split a
    /// concatenated string back into files.
    pub fn generate_for_dev_units(&self, module: &nexa_ir::Module) -> GeneratedSources {
        generator::generate_for_dev_units(module)
    }

    /// Generate development units and metadata for APIs preloaded in the host.
    pub fn generate_for_dev_units_with_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (GeneratedSources, SwiftProjectFeatures) {
        generator::generate_for_dev_units_with_project_features(module)
    }

    /// Generate the release host as one source unit per compile file.
    pub fn generate_units(&self, module: &nexa_ir::Module) -> GeneratedSources {
        generator::generate_units(module)
    }

    /// Generate WidgetKit extension sources from typed Nexa widget declarations.
    /// These are separate from the application host units and are intended to
    /// be assigned to a widget extension target by project generation.
    pub fn generate_widget_units(
        &self,
        module: &nexa_ir::Module,
    ) -> Result<WidgetGeneratedSources, WidgetGenerationError> {
        generator::generate_widget_units(module)
    }

    /// Generate release units together with metadata required by the iOS host.
    pub fn generate_units_with_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (GeneratedSources, SwiftProjectFeatures) {
        generator::generate_units_with_project_features(module)
    }

    /// Generate release units with the value codec runtime required by a
    /// native plugin contract that declares generic methods.
    pub fn generate_units_with_plugin_value_runtime(
        &self,
        module: &nexa_ir::Module,
    ) -> GeneratedSources {
        generator::generate_units_with_plugin_value_runtime(module)
    }

    /// Generate plugin-value runtime units together with iOS host metadata.
    pub fn generate_units_with_plugin_value_runtime_and_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (GeneratedSources, SwiftProjectFeatures) {
        generator::generate_units_with_plugin_value_runtime_and_project_features(module)
    }
}

impl nexa_codegen::Backend for SwiftBackend {
    fn name(&self) -> &'static str {
        "swift"
    }

    fn file_extension(&self) -> &'static str {
        "swift"
    }

    fn generate(&self, module: &nexa_ir::Module) -> String {
        generator::generate(module)
    }
}
