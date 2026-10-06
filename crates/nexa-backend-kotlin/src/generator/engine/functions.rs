use nexa_codegen::SourceWriter;
use nexa_ir::{Function, Module};

use super::{expressions, utils::indent};
use crate::generator::controls;
use crate::generator::engine::types::kotlin_type;

pub(crate) fn render(module: &Module, out: &mut SourceWriter) {
    for global in module
        .globals
        .iter()
        .filter(|global| !global.name.contains("::"))
    {
        if global.name.contains("::") {
            continue;
        }
        out.push_str(&format!(
            "private val {}: {} = {}\n",
            nexa_codegen::names::state_name(&global.name),
            kotlin_type(&global.ty),
            expressions::expression(&global.initial)
        ));
    }
    if module
        .globals
        .iter()
        .any(|global| !global.name.contains("::"))
    {
        out.push('\n');
    }
    render_user_classes(&module.functions, &module.globals, out);
    for function in module
        .functions
        .iter()
        .filter(|function| function.receiver.is_none() && !function.name.contains('.'))
    {
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

fn render_user_classes(functions: &[Function], globals: &[nexa_ir::State], out: &mut SourceWriter) {
    let mut classes = Vec::<(String, Vec<(String, nexa_ir::Type)>, usize)>::new();
    for function in functions {
        if let Some(nexa_ir::Type::Class {
            name,
            fields,
            constructor_parameter_count,
        }) = &function.receiver
            && !classes.iter().any(|(class, _, _)| class == name)
        {
            classes.push((name.clone(), fields.clone(), *constructor_parameter_count));
        }
    }
    for function in functions
        .iter()
        .filter(|function| function.receiver.is_none())
    {
        if let Some((class_name, _)) = function.name.split_once('.')
            && !classes.iter().any(|(class, _, _)| class == class_name)
        {
            classes.push((class_name.to_owned(), Vec::new(), 0));
        }
    }
    for global in globals {
        if let Some((class_name, _)) = global.name.split_once("::")
            && !classes.iter().any(|(class, _, _)| class == class_name)
        {
            classes.push((class_name.to_owned(), Vec::new(), 0));
        }
    }
    for (class_name, fields, constructor_parameter_count) in classes {
        let native_name = nexa_codegen::names::struct_name(&class_name);
        out.push_str(&format!("class {native_name}("));
        out.push_str(
            &fields
                .iter()
                .take(constructor_parameter_count)
                .map(|(name, ty)| {
                    format!(
                        "val {}: {}",
                        nexa_codegen::names::state_name(name),
                        kotlin_type(ty)
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str(") {\n");
        let static_members = globals
            .iter()
            .filter(|global| global.name.starts_with(&format!("{class_name}::")))
            .collect::<Vec<_>>();
        let static_functions = functions
            .iter()
            .filter(|function| {
                function.receiver.is_none()
                    && function
                        .name
                        .split_once('.')
                        .is_some_and(|(owner, _)| owner == class_name)
            })
            .collect::<Vec<_>>();
        if !static_members.is_empty() || !static_functions.is_empty() {
            out.line_at(1, format_args!("companion object {{"));
            for global in static_members {
                let (_, property) = global.name.split_once("::").unwrap_or(("", ""));
                out.line_at(
                    2,
                    format_args!(
                        "val {}: {} = {}",
                        nexa_codegen::names::state_name(property),
                        kotlin_type(&global.ty),
                        expressions::expression(&global.initial)
                    ),
                );
            }
            for function in static_functions {
                render_method(function, out);
            }
            out.line_at(1, format_args!("}}"));
        }
        if let Some(function) = functions.iter().find(|function| matches!(&function.receiver, Some(nexa_ir::Type::Class { name, .. }) if name == &class_name)) {
            for initializer in &function.class_initializers {
                out.line_at(1, format_args!("val {}: {} = {}", nexa_codegen::names::state_name(&initializer.name), kotlin_type(&initializer.ty), expressions::expression(&initializer.initial)));
            }
        }
        for function in functions.iter().filter(|function| {
            matches!(&function.receiver, Some(nexa_ir::Type::Class { name, .. }) if name == &class_name)
        }) {
            render_method(function, out);
        }
        out.push_str("}\n\n");
    }
}

fn render_method(function: &Function, out: &mut SourceWriter) {
    out.push_str(if function.is_async {
        "    suspend fun "
    } else {
        "    fun "
    });
    out.push_str(&nexa_codegen::names::function_name(
        function.name.rsplit('.').next().unwrap_or(&function.name),
    ));
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
    if let Some(actions) = &function.body_actions {
        controls::render_actions(actions, 2, out);
    } else {
        for local in &function.locals {
            out.line_at(
                2,
                format_args!(
                    "val {}: {} = {}",
                    nexa_codegen::names::state_name(&local.name),
                    kotlin_type(&local.ty),
                    expressions::expression(&local.initial)
                ),
            );
        }
        out.line_at(
            2,
            format_args!("return {}", expressions::expression(&function.body)),
        );
    }
    out.push_str("\n    }\n");
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
    if let Some(actions) = &function.body_actions {
        controls::render_actions(actions, 1, out);
    } else {
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
        out.push('\n');
    }
    out.push_str("}\n");
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, Module, State, Type};

    #[test]
    fn native_handle_globals_are_emitted_once_outside_ui_state() {
        let handle_type = Type::Plugin {
            namespace: "MMKV".to_owned(),
            name: "MMKVStore".to_owned(),
        };
        let module = Module {
            widgets: Vec::new(),
            app_name: "Persistence".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: vec![State {
                name: "commentStore".to_owned(),
                ty: handle_type.clone(),
                initial: Expr::Call {
                    name: "MMKV.MMKVStore".to_owned(),
                    arguments: Vec::new(),
                    return_type: handle_type,
                    is_async: false,
                    is_throwing: false,
                    is_constructor: true,
                },
                mutable: false,
            }],
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let mut output = SourceWriter::new();

        super::render(&module, &mut output);

        assert!(
            output
                .as_str()
                .contains("private val nexa_commentStore: MMKVStore")
        );
        assert!(output.as_str().contains("MMKVStore()"));
        assert!(!output.as_str().contains("remember"));
    }
}
