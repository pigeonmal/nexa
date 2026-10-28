use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// Stable identifier attached to compiler warnings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WarningCode(&'static str);

impl WarningCode {
    pub const GENERAL: Self = Self("NEXA-W0000");
    pub const UNUSED_FUNCTION: Self = Self("NEXA-W1001");
    pub const UNUSED_TEST_LOCAL: Self = Self("NEXA-W1002");
    pub const UNUSED_LOCAL: Self = Self("NEXA-W1003");
    pub const UNUSED_PARAMETER: Self = Self("NEXA-W1004");
    pub const UNUSED_DECLARATION: Self = Self("NEXA-W1005");
    pub const UNUSED_COMPONENT_PARAMETER: Self = Self("NEXA-W1006");
    pub const UNUSED_FAST_LIST_BINDING: Self = Self("NEXA-W1007");
    pub const UNUSED_LOOP_BINDING: Self = Self("NEXA-W1008");
    pub const CONSTANT_CONDITION: Self = Self("NEXA-W1009");
    pub const FAST_LIST_MISSING_KEY: Self = Self("NEXA-W1010");
    pub const UNUSED_PERMISSION: Self = Self("NEXA-W1011");
    pub const DYNAMIC_PERMISSION: Self = Self("NEXA-W1012");
    pub const PLUGIN_ANALYZER: Self = Self("NEXA-W9000");

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarningCodeDefinition {
    pub code: WarningCode,
    pub summary: &'static str,
}

pub const WARNING_CODES: &[WarningCodeDefinition] = &[
    WarningCodeDefinition {
        code: WarningCode::GENERAL,
        summary: "Warning without a more specific code.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_FUNCTION,
        summary: "Function is never used and will be removed from generated code.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_TEST_LOCAL,
        summary: "Test local is never used.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_LOCAL,
        summary: "Function local is never used.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_PARAMETER,
        summary: "Function parameter is never used.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_DECLARATION,
        summary: "State or constant is never used and will be removed from generated code.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_COMPONENT_PARAMETER,
        summary: "Component parameter is never used.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_FAST_LIST_BINDING,
        summary: "FastList row, section, or index binding is never used.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_LOOP_BINDING,
        summary: "Loop binding is never used.",
    },
    WarningCodeDefinition {
        code: WarningCode::CONSTANT_CONDITION,
        summary: "Condition is always true or false, so one branch is unreachable.",
    },
    WarningCodeDefinition {
        code: WarningCode::FAST_LIST_MISSING_KEY,
        summary: "Collection rows use positional identity because FastList has no key.",
    },
    WarningCodeDefinition {
        code: WarningCode::UNUSED_PERMISSION,
        summary: "Permission metadata is not used by app permission APIs or a reachable native plugin.",
    },
    WarningCodeDefinition {
        code: WarningCode::DYNAMIC_PERMISSION,
        summary: "Permission values are selected dynamically, so native permission declarations cannot be checked statically.",
    },
    WarningCodeDefinition {
        code: WarningCode::PLUGIN_ANALYZER,
        summary: "Plugin analyzer reported a warning.",
    },
];

/// Renders the stable warning-code table for the language reference.
pub fn render_warning_code_reference() -> String {
    let mut output = String::from("| Code | Meaning |\n|---|---|\n");
    for definition in WARNING_CODES {
        output.push_str(&format!(
            "| `{}` | {} |\n",
            definition.code.as_str(),
            definition.summary
        ));
    }
    output
}

#[derive(Clone, Debug)]
pub struct CompileError {
    pub span: Span,
    pub message: String,
    pub file: Option<String>,
}

impl CompileError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            file: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        if self.file.is_none() {
            self.file = Some(file.into());
        }
        self
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(
                f,
                "{file}:{}:{}: error: {}",
                self.span.line, self.span.column, self.message
            )
        } else {
            write!(
                f,
                "{}:{}: error: {}",
                self.span.line, self.span.column, self.message
            )
        }
    }
}

impl std::error::Error for CompileError {}

#[derive(Clone, Debug)]
pub struct CompileWarning {
    pub code: WarningCode,
    pub span: Span,
    pub message: String,
    pub file: Option<String>,
}

impl CompileWarning {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            code: WarningCode::GENERAL,
            span,
            message: message.into(),
            file: None,
        }
    }

    pub fn with_code(mut self, code: WarningCode) -> Self {
        self.code = code;
        self
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        if self.file.is_none() {
            self.file = Some(file.into());
        }
        self
    }
}

impl fmt::Display for CompileWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(
                f,
                "{file}:{}:{}: warning[{}]: {}",
                self.span.line,
                self.span.column,
                self.code.as_str(),
                self.message
            )
        } else {
            write!(
                f,
                "{}:{}: warning[{}]: {}",
                self.span.line,
                self.span.column,
                self.code.as_str(),
                self.message
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CompileError, CompileWarning, Span, WARNING_CODES, render_warning_code_reference};

    fn span() -> Span {
        Span {
            start: 14,
            end: 19,
            line: 3,
            column: 5,
        }
    }

    #[test]
    fn diagnostics_render_source_location_and_kind() {
        let error = CompileError::new(span(), "unknown plugin member")
            .with_file("src/main.nx")
            .to_string();
        let warning = CompileWarning::new(span(), "unused binding")
            .with_file("src/main.nx")
            .to_string();

        assert_eq!(error, "src/main.nx:3:5: error: unknown plugin member");
        assert_eq!(
            warning,
            "src/main.nx:3:5: warning[NEXA-W0000]: unused binding"
        );
    }

    #[test]
    fn first_attached_source_file_is_preserved() {
        let rendered = CompileError::new(span(), "invalid property")
            .with_file("plugin/native.nxid")
            .with_file("app/main.nx")
            .to_string();

        assert!(rendered.starts_with("plugin/native.nxid:3:5: error:"));
    }

    #[test]
    fn warning_keeps_its_source_file_when_context_is_attached_twice() {
        let rendered = CompileWarning::new(span(), "unused native property")
            .with_file("plugin/native.nxid")
            .with_file("app/main.nx")
            .to_string();

        assert_eq!(
            rendered,
            "plugin/native.nxid:3:5: warning[NEXA-W0000]: unused native property"
        );
    }

    #[test]
    fn warning_codes_are_unique_and_reference_is_rendered_from_the_catalog() {
        let mut codes = std::collections::HashSet::new();
        for definition in WARNING_CODES {
            assert!(codes.insert(definition.code.as_str()));
            assert!(definition.code.as_str().starts_with("NEXA-W"));
        }
        assert!(render_warning_code_reference().contains("`NEXA-W1001`"));
    }
}
