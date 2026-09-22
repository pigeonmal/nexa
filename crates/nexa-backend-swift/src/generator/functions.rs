use nexa_ir::{Function, Module};

use super::{expressions, utils::indent};

pub(super) fn render(module: &Module, out: &mut String) {
    for function in &module.functions {
        render_function(function, out);
    }
    if !module.functions.is_empty() {
        out.push('\n');
    }
}

fn render_function(function: &Function, out: &mut String) {
    out.push_str("private func ");
    out.push_str(&nexa_codegen::names::function_name(&function.name));
    out.push('(');
    out.push_str(
        &function
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "_ {}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    parameter.ty.swift()
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    if function.is_async {
        out.push_str(") async -> ");
    } else {
        out.push_str(") -> ");
    }
    out.push_str(&function.return_type.swift());
    out.push_str(" {\n");
    indent(out, 1);
    out.push_str("return ");
    out.push_str(&expressions::expression(&function.body));
    out.push_str("\n}\n");
}
