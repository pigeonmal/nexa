use nexa_codegen::SourceWriter;
use nexa_ir::{Function, Module};

use super::{expressions, utils::indent};
use crate::generator::controls;
use crate::generator::engine::types::swift_type;

pub(crate) fn render(module: &Module, out: &mut SourceWriter) {
    for global in module
        .globals
        .iter()
        .filter(|global| !global.name.contains("::"))
    {
        if global.name.contains("::") {
            continue;
        }
        // These singleton-style globals are shared by app and generated
        // screen/component source units.
        out.push_str(&format!(
            "@MainActor let {}: {} = {}\n",
            nexa_codegen::names::state_name(&global.name),
            swift_type(&global.ty),
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
            "@MainActor\nfunc __nexaBackgroundTask{index}() async {{\n"
        ));
        controls::render_actions(&task.actions, 1, out);
        out.push_str("}\n\n");
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
        out.push_str(&format!("@MainActor final class {native_name} {{\n"));
        for (name, ty) in &fields {
            out.line_at(
                1,
                format_args!(
                    "let {}: {}",
                    nexa_codegen::names::state_name(name),
                    swift_type(ty)
                ),
            );
        }
        for global in globals
            .iter()
            .filter(|global| global.name.starts_with(&format!("{class_name}::")))
        {
            let (_, property) = global.name.split_once("::").unwrap_or(("", ""));
            out.line_at(
                1,
                format_args!(
                    "static let {}: {} = {}",
                    nexa_codegen::names::state_name(property),
                    swift_type(&global.ty),
                    expressions::expression(&global.initial)
                ),
            );
        }
        out.line_at(1, format_args!("init("));
        for (index, (name, ty)) in fields.iter().take(constructor_parameter_count).enumerate() {
            let suffix = if index + 1 == constructor_parameter_count {
                ""
            } else {
                ","
            };
            out.line_at(
                2,
                format_args!(
                    "_ {}: {}{}",
                    nexa_codegen::names::state_name(name),
                    swift_type(ty),
                    suffix
                ),
            );
        }
        out.line_at(1, format_args!(") {{"));
        for (name, _) in fields.iter().take(constructor_parameter_count) {
            let property = nexa_codegen::names::state_name(name);
            out.line_at(2, format_args!("self.{property} = {property}"));
        }
        if let Some(function) = functions.iter().find(|function| matches!(&function.receiver, Some(nexa_ir::Type::Class { name, .. }) if name == &class_name)) {
            for initializer in &function.class_initializers {
                let property = nexa_codegen::names::state_name(&initializer.name);
                out.line_at(2, format_args!("self.{property} = {}", expressions::expression(&initializer.initial)));
            }
        }
        out.line_at(1, format_args!("}}"));
        for function in functions.iter().filter(|function| {
            matches!(&function.receiver, Some(nexa_ir::Type::Class { name, .. }) if name == &class_name)
        }) {
            render_method(function, false, out);
        }
        for function in functions.iter().filter(|function| {
            function.receiver.is_none()
                && function
                    .name
                    .split_once('.')
                    .is_some_and(|(owner, _)| owner == class_name)
        }) {
            render_method(function, true, out);
        }
        out.push_str("}\n\n");
    }
}

fn render_method(function: &Function, is_static: bool, out: &mut SourceWriter) {
    out.push_str(if is_static {
        "    static func "
    } else {
        "    func "
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
                    "_ {}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    swift_type(&parameter.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push(')');
    if function.is_async {
        out.push_str(" async");
    }
    if function.is_throwing {
        out.push_str(" throws");
    }
    out.push_str(" -> ");
    out.push_str(&swift_type(&function.return_type));
    out.push_str(" {\n");
    if let Some(actions) = &function.body_actions {
        controls::render_actions(actions, 2, out);
    } else {
        for local in &function.locals {
            out.line_at(
                2,
                format_args!(
                    "let {}: {} = {}",
                    nexa_codegen::names::state_name(&local.name),
                    swift_type(&local.ty),
                    expressions::expression(&local.initial)
                ),
            );
        }
        out.line_at(
            2,
            format_args!("return {}", expressions::expression(&function.body)),
        );
    }
    out.push_str("    }\n");
}

fn render_function(function: &Function, out: &mut SourceWriter) {
    // App functions can be referenced from imported component source units,
    // so they must have module visibility rather than file-private visibility.
    out.push_str("@MainActor func ");
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
                    swift_type(&parameter.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push(')');
    if function.is_async {
        out.push_str(" async");
    }
    if function.is_throwing {
        out.push_str(" throws");
    }
    out.push_str(" -> ");
    out.push_str(&swift_type(&function.return_type));
    out.push_str(" {\n");
    if let Some(actions) = &function.body_actions {
        controls::render_actions(actions, 1, out);
    } else {
        for local in &function.locals {
            out.line_at(
                1,
                format_args!(
                    "let {}: {} = {}",
                    nexa_codegen::names::state_name(&local.name),
                    swift_type(&local.ty),
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
                .contains("@MainActor let nexa_commentStore:")
        );
        assert!(output.as_str().contains("MMKVStore()"));
        assert!(
            !output
                .as_str()
                .contains("@State private var nexa_commentStore")
        );
    }
}
