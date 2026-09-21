mod generator;

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
