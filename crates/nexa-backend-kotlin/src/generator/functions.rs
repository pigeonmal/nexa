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
    if function.is_async {
        out.push_str("private suspend fun ");
    } else {
        out.push_str("private fun ");
    }
    out.push_str(&nexa_codegen::names::function_name(&function.name));
    out.push('(');
    out.push_str(
        &function
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    parameter.ty.kotlin()
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push_str("): ");
    out.push_str(&function.return_type.kotlin());
    out.push_str(" {\n");
    indent(out, 1);
    out.push_str("return ");
    out.push_str(&expressions::expression(&function.body));
    out.push_str("\n}\n");
}
