use std::collections::HashMap;

use nexa_ir::{ScreenId, Type};

use super::{
    custom_components::ComponentSignatures, expressions::FunctionSignatures, themes::ThemeSymbols,
};
use crate::Target;

#[derive(Clone)]
pub(super) struct ScreenSignature {
    pub(super) id: ScreenId,
    pub(super) parameters: Vec<nexa_ir::FunctionParameter>,
}

pub(super) type ScreenSignatures = HashMap<String, ScreenSignature>;

/// The environment every expression-lowering helper needs.
///
/// Expression lowering is a different concern from node lowering, so it gets
/// its own bundle rather than the whole [`SemanticContext`]: what an
/// expression needs is the symbol table, the function signatures it may call,
/// and whether `await` is legal here. Threading those three separately through
/// the recursive descent was how a call site ended up with the right symbol
/// table and the wrong function table.
#[derive(Clone, Copy)]
pub(super) struct ExprContext<'a> {
    pub(super) symbols: &'a HashMap<String, (Type, bool)>,
    pub(super) functions: &'a FunctionSignatures,
    pub(super) allow_await: bool,
}

impl<'a> ExprContext<'a> {
    pub(super) fn new(
        symbols: &'a HashMap<String, (Type, bool)>,
        functions: &'a FunctionSignatures,
        allow_await: bool,
    ) -> Self {
        Self {
            symbols,
            functions,
            allow_await,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct SemanticContext<'a> {
    pub(super) symbols: &'a HashMap<String, (Type, bool)>,
    pub(super) screen_ids: &'a ScreenSignatures,
    pub(super) themes: &'a ThemeSymbols,
    pub(super) components: &'a ComponentSignatures,
    pub(super) functions: &'a FunctionSignatures,
    pub(super) native_aliases: &'a HashMap<String, String>,
    pub(super) allow_navigation_stack: bool,
    pub(super) allow_navigation_back: bool,
    pub(super) target: Target,
}

impl<'a> SemanticContext<'a> {
    pub(super) fn new(
        symbols: &'a HashMap<String, (Type, bool)>,
        screen_ids: &'a ScreenSignatures,
        themes: &'a ThemeSymbols,
        components: &'a ComponentSignatures,
        functions: &'a FunctionSignatures,
        native_aliases: &'a HashMap<String, String>,
        target: Target,
    ) -> Self {
        Self {
            symbols,
            screen_ids,
            themes,
            components,
            functions,
            native_aliases,
            allow_navigation_stack: true,
            allow_navigation_back: false,
            target,
        }
    }

    pub(super) fn with_symbols<'b>(
        &self,
        symbols: &'b HashMap<String, (Type, bool)>,
    ) -> SemanticContext<'b>
    where
        'a: 'b,
    {
        SemanticContext {
            symbols,
            screen_ids: self.screen_ids,
            themes: self.themes,
            components: self.components,
            functions: self.functions,
            native_aliases: self.native_aliases,
            allow_navigation_stack: self.allow_navigation_stack,
            allow_navigation_back: self.allow_navigation_back,
            target: self.target,
        }
    }

    pub(super) fn with_navigation(&self, allow_stack: bool, allow_back: bool) -> Self {
        Self {
            allow_navigation_stack: allow_stack,
            allow_navigation_back: allow_back,
            ..*self
        }
    }

    /// Narrows to the slice of this context that expression lowering needs.
    pub(super) fn exprs(&self, allow_await: bool) -> ExprContext<'_> {
        ExprContext {
            symbols: self.symbols,
            functions: self.functions,
            allow_await,
        }
    }
}
