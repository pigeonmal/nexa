use std::collections::{HashMap, HashSet};

use nexa_ir::{ScreenId, Type};

use super::{
    custom_components::ComponentSignatures,
    expressions::{FunctionSignatures, StructTypes},
    themes::ThemeSymbols,
};
use crate::Target;

/// Compile-time capabilities of a target's native codec contract.
///
/// Capabilities describe what generated Swift/Kotlin can represent. They are
/// deliberately **not** derived from AOT vs DevRuntime: a source program must
/// be accepted or rejected identically in both modes, otherwise `nexa dev`
/// turns green on a program that `nexa build` rejects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SemanticCapabilities {
    /// A plugin generic read may bind a top-level optional type.
    ///
    /// A native decoder reports decode failure separately from a decoded
    /// value, so it cannot yet represent `value | null | error` at the top
    /// level of a generic read. No supported target enables this today.
    /// Enabling it requires a tagged native read result in generated Swift and
    /// Kotlin, tested in both modes, not a mode-only allowance.
    pub(super) nullable_generic_plugin_reads: bool,
}

impl SemanticCapabilities {
    /// The canonical capability set for a target.
    ///
    /// Both AOT and DevRuntime lowering call this same function so acceptance
    /// cannot diverge by mode.
    pub(super) fn for_target(_target: Target) -> Self {
        Self::default()
    }
}

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
    /// Top-level value struct declarations, so an explicit plugin type
    /// argument such as `getObject<PlayerOptions>` resolves to its fields.
    pub(super) structs: &'a StructTypes,
    /// Every declared enum name, so a bound plugin value type can prove the
    /// enum it names exists.
    pub(super) enums: &'a HashSet<String>,
    pub(super) allow_await: bool,
    /// DevRuntime can represent generic nullable reads with a tagged result
    /// that keeps valid nulls separate from reader failures.
    pub(super) nullable_generic_plugin_reads: bool,
    /// Declared app screens for typed `Navigation.push(...)` action lowering.
    pub(super) screen_ids: Option<&'a ScreenSignatures>,
    /// Navigation actions are available only inside screen-owned action blocks.
    pub(super) allow_navigation_actions: bool,
    pub(super) source_file: Option<&'a str>,
}

impl<'a> ExprContext<'a> {
    #[cfg(test)]
    pub(super) fn new(
        symbols: &'a HashMap<String, (Type, bool)>,
        functions: &'a FunctionSignatures,
        allow_await: bool,
    ) -> Self {
        Self::with_types(
            symbols,
            functions,
            allow_await,
            empty_struct_types(),
            empty_enum_names(),
        )
    }

    pub(super) fn with_types(
        symbols: &'a HashMap<String, (Type, bool)>,
        functions: &'a FunctionSignatures,
        allow_await: bool,
        structs: &'a StructTypes,
        enums: &'a HashSet<String>,
    ) -> Self {
        Self {
            symbols,
            functions,
            structs,
            enums,
            allow_await,
            nullable_generic_plugin_reads: false,
            screen_ids: None,
            allow_navigation_actions: false,
            source_file: None,
        }
    }

    pub(super) fn with_nullable_generic_plugin_reads(mut self, allow: bool) -> Self {
        self.nullable_generic_plugin_reads = allow;
        self
    }

    pub(super) fn with_source_file(mut self, source_file: Option<&'a str>) -> Self {
        self.source_file = source_file;
        self
    }
}

/// The app's value struct and enum declarations, bundled so expression
/// lowering can resolve an explicit plugin type argument
/// (`getObject<PlayerOptions>`) and prove a bound value type exists.
#[derive(Clone, Copy)]
pub(super) struct TypeRegistries<'a> {
    pub structs: &'a StructTypes,
    pub enums: &'a HashSet<String>,
    pub nullable_generic_plugin_reads: bool,
    pub screen_ids: Option<&'a ScreenSignatures>,
    pub allow_navigation_actions: bool,
    pub source_file: Option<&'a str>,
}

impl Default for TypeRegistries<'_> {
    fn default() -> Self {
        Self {
            structs: empty_struct_types(),
            enums: empty_enum_names(),
            nullable_generic_plugin_reads: false,
            screen_ids: None,
            allow_navigation_actions: false,
            source_file: None,
        }
    }
}

impl<'a> TypeRegistries<'a> {
    pub(super) fn expr_context(
        self,
        symbols: &'a HashMap<String, (Type, bool)>,
        functions: &'a FunctionSignatures,
        allow_await: bool,
    ) -> ExprContext<'a> {
        let mut context =
            ExprContext::with_types(symbols, functions, allow_await, self.structs, self.enums)
                .with_nullable_generic_plugin_reads(self.nullable_generic_plugin_reads);
        context.screen_ids = self.screen_ids;
        context.allow_navigation_actions = self.allow_navigation_actions;
        context.source_file = self.source_file;
        context
    }
}

/// Shared empty struct registry for contexts that cannot declare one.
fn empty_struct_types() -> &'static StructTypes {
    static EMPTY: std::sync::OnceLock<StructTypes> = std::sync::OnceLock::new();
    EMPTY.get_or_init(StructTypes::default)
}

/// Shared empty enum registry for contexts that cannot declare one.
pub(super) fn empty_enum_names() -> &'static HashSet<String> {
    static EMPTY: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
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
    /// Top-level value struct declarations, in scope wherever expressions
    /// are lowered, so an explicit plugin type argument resolves its fields.
    pub(super) structs: &'a StructTypes,
    /// Every declared enum name, for the same reason.
    pub(super) enums: &'a HashSet<String>,
    pub(super) target: Target,
    pub(super) nullable_generic_plugin_reads: bool,
    pub(super) source_file: Option<&'a str>,
    /// Project-configured icon aliases resolved to typed font glyphs.
    pub(super) custom_icons: Option<&'a HashMap<String, nexa_ir::SystemIcon>>,
}

impl<'a> SemanticContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        symbols: &'a HashMap<String, (Type, bool)>,
        screen_ids: &'a ScreenSignatures,
        themes: &'a ThemeSymbols,
        components: &'a ComponentSignatures,
        functions: &'a FunctionSignatures,
        native_aliases: &'a HashMap<String, String>,
        structs: &'a StructTypes,
        enums: &'a HashSet<String>,
        target: Target,
        nullable_generic_plugin_reads: bool,
    ) -> Self {
        Self {
            symbols,
            screen_ids,
            themes,
            components,
            functions,
            native_aliases,
            structs,
            enums,
            allow_navigation_stack: true,
            allow_navigation_back: false,
            target,
            nullable_generic_plugin_reads,
            source_file: None,
            custom_icons: None,
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
            structs: self.structs,
            enums: self.enums,
            allow_navigation_stack: self.allow_navigation_stack,
            allow_navigation_back: self.allow_navigation_back,
            target: self.target,
            nullable_generic_plugin_reads: self.nullable_generic_plugin_reads,
            source_file: self.source_file,
            custom_icons: self.custom_icons,
        }
    }

    pub(super) fn with_optional_custom_icons(
        mut self,
        icons: Option<&'a HashMap<String, nexa_ir::SystemIcon>>,
    ) -> Self {
        self.custom_icons = icons;
        self
    }

    pub(super) fn resolve_icon(&self, name: &str) -> Option<nexa_ir::SystemIcon> {
        self.custom_icons
            .and_then(|icons| icons.get(name).cloned())
            .or_else(|| nexa_ir::SystemIcon::shared(name))
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
            structs: self.structs,
            enums: self.enums,
            allow_await,
            nullable_generic_plugin_reads: self.nullable_generic_plugin_reads,
            screen_ids: Some(self.screen_ids),
            allow_navigation_actions: self.allow_navigation_back,
            source_file: self.source_file,
        }
    }

    pub(super) fn type_registries(&self) -> TypeRegistries<'a> {
        TypeRegistries {
            structs: self.structs,
            enums: self.enums,
            nullable_generic_plugin_reads: self.nullable_generic_plugin_reads,
            screen_ids: Some(self.screen_ids),
            allow_navigation_actions: self.allow_navigation_back,
            source_file: self.source_file,
        }
    }

    pub(super) fn with_source_file(mut self, source_file: Option<&'a str>) -> Self {
        self.source_file = source_file;
        self
    }
}
