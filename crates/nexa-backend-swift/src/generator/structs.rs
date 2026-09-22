use nexa_ir::{Module, StructDecl};

pub(super) fn render(module: &Module, out: &mut String) {
    for declaration in &module.structs {
        render_struct(declaration, out);
    }
    if !module.structs.is_empty() {
        out.push('\n');
    }
}

fn render_struct(declaration: &StructDecl, out: &mut String) {
    let native_name = nexa_codegen::names::struct_name(&declaration.name);
    out.push_str(&format!("private struct {native_name} {{\n"));
    for field in &declaration.fields {
        out.push_str("    ");
        out.push_str(&format!(
            "let {}: {}\n",
            nexa_codegen::names::struct_field_name(&field.name),
            field.ty.swift()
        ));
    }
    out.push_str("}\n\n");
}
