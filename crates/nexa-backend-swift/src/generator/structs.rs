use nexa_ir::{Module, StructDecl};

use super::utils::indent;

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
        indent(out, 1);
        out.push_str(&format!(
            "let {}: {}\n",
            nexa_codegen::names::struct_field_name(&field.name),
            field.ty.swift()
        ));
    }
    out.push_str("}\n");
    out.push_str(&format!(
        "private func {}(",
        nexa_codegen::names::function_name(&declaration.name)
    ));
    for (index, field) in declaration.fields.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("_ nexa_arg_{index}: {}", field.ty.swift()));
    }
    out.push_str(&format!(") -> {native_name} {{\n"));
    indent(out, 1);
    out.push_str(&native_name);
    if declaration.fields.is_empty() {
        out.push_str("()\n");
    } else {
        out.push('(');
        for (index, field) in declaration.fields.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!(
                "{}: nexa_arg_{index}",
                nexa_codegen::names::struct_field_name(&field.name)
            ));
        }
        out.push_str(")\n");
    }
    out.push_str("}\n\n");
}
