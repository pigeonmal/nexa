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
    pub fn generate_with_project_features(
        &self,
        module: &nexa_ir::Module,
    ) -> (String, KotlinProjectFeatures) {
        generator::generate_with_project_features(module)
    }
}
