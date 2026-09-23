use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
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
    pub span: Span,
    pub message: String,
    pub file: Option<String>,
}

impl CompileWarning {
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

impl fmt::Display for CompileWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(
                f,
                "{file}:{}:{}: warning: {}",
                self.span.line, self.span.column, self.message
            )
        } else {
            write!(
                f,
                "{}:{}: warning: {}",
                self.span.line, self.span.column, self.message
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CompileError, CompileWarning, Span};

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
        assert_eq!(warning, "src/main.nx:3:5: warning: unused binding");
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
            "plugin/native.nxid:3:5: warning: unused native property"
        );
    }
}
