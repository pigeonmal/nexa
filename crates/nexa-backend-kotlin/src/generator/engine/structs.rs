use nexa_ir::{Module, StructDecl};

pub(crate) fn render(module: &Module, out: &mut String) {
    for declaration in &module.structs {
        render_struct(declaration, out);
    }
    if !module.structs.is_empty() {
        out.push('\n');
    }
}

fn render_struct(declaration: &StructDecl, out: &mut String) {
    let native_name = nexa_codegen::names::struct_name(&declaration.name);
    if declaration.fields.is_empty() {
        out.push_str(&format!("private class {native_name}\n\n"));
        return;
    }
    out.push_str(&format!("private data class {native_name}("));
    for (index, field) in declaration.fields.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "val {}: {}",
            nexa_codegen::names::struct_field_name(&field.name),
            field.ty.kotlin()
        ));
    }
    out.push_str(")\n\n");
}
