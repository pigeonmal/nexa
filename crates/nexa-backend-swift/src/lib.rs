mod generator;

pub struct SwiftBackend;

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
