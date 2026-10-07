use crate::generator::engine::types::kotlin_type;
use nexa_codegen::SourceWriter;
use nexa_ir::{Module, StructDecl, Type};

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
    if is_deeply_immutable(declaration) {
        out.push_str("@Immutable\n");
    }
    if declaration.fields.is_empty() {
        out.push_str(&format!("class {native_name}\n\n"));
        return;
    }
    out.push_str(&format!("data class {native_name}("));
    for (index, field) in declaration.fields.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "val {}: {}",
            nexa_codegen::names::struct_field_name(&field.name),
            kotlin_type(&field.ty)
        ));
    }
    out.push_str(")\n\n");
}

pub(crate) fn has_immutable_structs(module: &Module) -> bool {
    module.structs.iter().any(is_deeply_immutable)
}

fn is_deeply_immutable(declaration: &StructDecl) -> bool {
    declaration
        .fields
        .iter()
        .all(|field| is_immutable_value_type(&field.ty))
}

fn is_immutable_value_type(ty: &Type) -> bool {
    match ty {
        Type::String | Type::Bool | Type::Numeric(_) | Type::Enum(_) => true,
        Type::Optional(inner) => is_immutable_value_type(inner),
        Type::Struct { fields, .. } => fields
            .iter()
            .all(|(_, field_type)| is_immutable_value_type(field_type)),
        Type::Pair(first, second) | Type::Result(first, second) => {
            is_immutable_value_type(first) && is_immutable_value_type(second)
        }
        Type::Triple(first, second, third) => {
            is_immutable_value_type(first)
                && is_immutable_value_type(second)
                && is_immutable_value_type(third)
        }
        // Arrays, sets, maps, byte arrays, signals, plugin objects, classes,
        // and task handles can be mutated or backed by mutable native state.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Module, StructDecl, StructField, Type};

    use super::{has_immutable_structs, render};

    fn module_with_field(ty: Type) -> Module {
        Module {
            app_name: "Stability".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: vec![StructDecl {
                name: "OrderSummary".to_owned(),
                fields: vec![StructField {
                    name: "total".to_owned(),
                    ty,
                }],
            }],
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
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
        }
    }

    #[test]
    fn immutable_value_structs_receive_compose_immutable_annotation() {
        let module = module_with_field(Type::Optional(Box::new(Type::String)));
        let mut output = SourceWriter::new();

        render(&module, &mut output);

        assert!(has_immutable_structs(&module));
        assert_eq!(
            output.as_str(),
            "@Immutable\ndata class NexaOrderSummary(val nexa_field_total: String?)\n\n\n"
        );
    }

    #[test]
    fn structs_with_collection_fields_are_not_claimed_immutable() {
        let module = module_with_field(Type::Map(
            Box::new(Type::String),
            Box::new(Type::Numeric(nexa_ir::NumericType::Int32)),
        ));
        let mut output = SourceWriter::new();

        render(&module, &mut output);

        assert!(!has_immutable_structs(&module));
        assert_eq!(
            output.as_str(),
            "data class NexaOrderSummary(val nexa_field_total: Map<String, Int>)\n\n\n"
        );
    }
}
