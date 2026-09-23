use crate::diagnostics::span_to_range;
use crate::protocol::{DocumentSymbol, SymbolKind};

/// Extracts hierarchical document symbols for the outline view.
pub fn get_document_symbols(source: &str) -> Vec<DocumentSymbol> {
    let Ok(program) = nexa_syntax::parse_program(source) else {
        return Vec::new();
    };

    let mut symbols = Vec::new();

    // 1. Top-level components
    for comp in &program.components {
        let range = span_to_range(&comp.span);
        symbols.push(DocumentSymbol {
            name: comp.name.clone(),
            detail: Some("component".to_string()),
            kind: SymbolKind::Interface,
            range,
            selection_range: range,
            children: None,
        });
    }

    // 2. Top-level structs
    for st in &program.structs {
        let range = span_to_range(&st.span);
        symbols.push(DocumentSymbol {
            name: st.name.clone(),
            detail: Some("struct".to_string()),
            kind: SymbolKind::Struct,
            range,
            selection_range: range,
            children: None,
        });
    }

    // 3. Top-level functions
    for func in &program.functions {
        let range = span_to_range(&func.span);
        symbols.push(DocumentSymbol {
            name: func.name.clone(),
            detail: Some(format!("fn {}", func.name)),
            kind: SymbolKind::Function,
            range,
            selection_range: range,
            children: None,
        });
    }

    // 4. App declaration and its contents
    if let Some(app) = &program.app {
        let mut app_children = Vec::new();

        for state in &app.states {
            let range = span_to_range(&state.span);
            app_children.push(DocumentSymbol {
                name: state.name.clone(),
                detail: Some("state".to_string()),
                kind: SymbolKind::Variable,
                range,
                selection_range: range,
                children: None,
            });
        }

        for func in &app.functions {
            let range = span_to_range(&func.span);
            app_children.push(DocumentSymbol {
                name: func.name.clone(),
                detail: Some(format!("fn {}", func.name)),
                kind: SymbolKind::Function,
                range,
                selection_range: range,
                children: None,
            });
        }

        for en in &app.enums {
            let range = span_to_range(&en.span);
            let mut enum_children = Vec::new();
            for case in &en.cases {
                let c_range = span_to_range(&case.span);
                enum_children.push(DocumentSymbol {
                    name: case.name.clone(),
                    detail: Some("case".to_string()),
                    kind: SymbolKind::EnumMember,
                    range: c_range,
                    selection_range: c_range,
                    children: None,
                });
            }
            app_children.push(DocumentSymbol {
                name: en.name.clone(),
                detail: Some("enum".to_string()),
                kind: SymbolKind::Enum,
                range,
                selection_range: range,
                children: if enum_children.is_empty() {
                    None
                } else {
                    Some(enum_children)
                },
            });
        }

        for st in &app.structs {
            let range = span_to_range(&st.span);
            app_children.push(DocumentSymbol {
                name: st.name.clone(),
                detail: Some("struct".to_string()),
                kind: SymbolKind::Struct,
                range,
                selection_range: range,
                children: None,
            });
        }

        for screen in &app.screens {
            let range = span_to_range(&screen.span);
            app_children.push(DocumentSymbol {
                name: screen.name.clone(),
                detail: Some("screen".to_string()),
                kind: SymbolKind::Class,
                range,
                selection_range: range,
                children: None,
            });
        }

        let app_range = span_to_range(&app.span);
        symbols.push(DocumentSymbol {
            name: app.name.clone(),
            detail: Some("app".to_string()),
            kind: SymbolKind::Module,
            range: app_range,
            selection_range: app_range,
            children: if app_children.is_empty() {
                None
            } else {
                Some(app_children)
            },
        });
    }

    symbols
}
