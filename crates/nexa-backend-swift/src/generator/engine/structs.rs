use crate::generator::engine::types::swift_type;
use nexa_codegen::SourceWriter;
use nexa_ir::{Module, StructDecl};

pub(crate) fn render(module: &Module, out: &mut SourceWriter) {
    for declaration in &module.structs {
        render_struct(declaration, out);
    }
    if !module.structs.is_empty() {
        out.push('\n');
    }
}

fn render_struct(declaration: &StructDecl, out: &mut SourceWriter) {
    let native_name = nexa_codegen::names::struct_name(&declaration.name);
    out.push_str(&format!("private struct {native_name}: Equatable {{\n"));
    for field in &declaration.fields {
        out.push_str("    ");
        out.push_str(&format!(
            "let {}: {}\n",
            nexa_codegen::names::struct_field_name(&field.name),
            swift_type(&field.ty)
        ));
    }
    out.push_str("}\n\n");
}
