mod generator;

pub use nexa_codegen::{GeneratedSources, SourceUnit};

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

    /// Generate the release host as one source unit per compile file.
    pub fn generate_units(&self, module: &nexa_ir::Module) -> GeneratedSources {
        generator::generate_units(module)
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
