use std::collections::HashMap;

use nexa_ir::{ScreenId, Type};

use super::{
    custom_components::ComponentSignatures,
    expressions::FunctionSignatures,
    themes::ThemeSymbols,
};
use crate::Target;

#[derive(Clone)]
pub(super) struct ScreenSignature {
    pub(super) id: ScreenId,
    pub(super) parameters: Vec<nexa_ir::FunctionParameter>,
}

pub(super) type ScreenSignatures = HashMap<String, ScreenSignature>;

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
}
