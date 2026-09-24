mod generator;

pub struct SwiftBackend;

impl SwiftBackend {
    /// Generate a development host with built-in async native API support.
    pub fn generate_for_dev(&self, module: &nexa_ir::Module) -> String {
        generator::generate_for_dev(module)
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
