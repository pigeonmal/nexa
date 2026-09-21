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
