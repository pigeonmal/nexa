use nexa_codegen::SourceWriter;
use nexa_ir::{Function, Module};

use super::{expressions, utils::indent};
use crate::generator::controls;
use crate::generator::engine::types::kotlin_type;

pub(crate) fn render(module: &Module, out: &mut SourceWriter) {
    for function in &module.functions {
        render_function(function, out);
    }
    if !module.functions.is_empty() {
        out.push('\n');
    }
    for (index, task) in module.background_tasks.iter().enumerate() {
        out.push_str(&format!(
            "internal suspend fun __nexaBackgroundTask{index}() {{\n"
        ));
        controls::render_actions(&task.actions, 1, out);
        out.push_str("}\n\n");
    }
    if !module.background_tasks.is_empty() {
        out.push_str(
            "internal suspend fun __nexaRunBackgroundTask(index: Int) {\n    when (index) {\n",
        );
        for (index, _) in module.background_tasks.iter().enumerate() {
            out.push_str(&format!(
                "        {index} -> __nexaBackgroundTask{index}()\n"
            ));
        }
        out.push_str("        else -> Unit\n    }\n}\n");
    }
}

fn render_function(function: &Function, out: &mut SourceWriter) {
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
                    kotlin_type(&parameter.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push_str("): ");
    out.push_str(&kotlin_type(&function.return_type));
    out.push_str(" {\n");
    for local in &function.locals {
        out.line_at(
            1,
            format_args!(
                "val {}: {} = {}",
                nexa_codegen::names::state_name(&local.name),
                kotlin_type(&local.ty),
                expressions::expression(&local.initial)
            ),
        );
    }
    indent(out, 1);
    out.push_str("return ");
    out.push_str(&expressions::expression(&function.body));
    out.push_str("\n}\n");
}
