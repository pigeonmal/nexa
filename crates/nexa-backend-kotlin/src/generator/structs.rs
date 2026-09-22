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
    if declaration.fields.is_empty() {
        out.push_str(&format!("private class {native_name}\n"));
        out.push_str(&format!(
            "private fun {}(): {native_name} = {native_name}()\n\n",
            nexa_codegen::names::function_name(&declaration.name)
        ));
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
    out.push_str(")\n");
    out.push_str(&format!(
        "private fun {}(",
        nexa_codegen::names::function_name(&declaration.name)
    ));
    for (index, field) in declaration.fields.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("nexa_arg_{index}: {}", field.ty.kotlin()));
    }
    out.push_str(&format!("): {native_name} = {native_name}(",));
    for (index, _) in declaration.fields.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("nexa_arg_{index}"));
    }
    out.push_str(")\n\n");
}
