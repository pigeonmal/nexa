//! A small buffer for building generated source text.
//!
//! Every native generator accumulates output with `push_str` and
//! `push_str(&format!(...))`, and indents by pushing a repeated unit before a
//! line. That is verbose, and the `format!` form allocates a throwaway
//! `String` for every single line.
//!
//! [`SourceWriter`] is the compile-time-only replacement. It owns the buffer,
//! formats straight into it through [`fmt::Arguments`], and knows how to indent
//! a line. It is deliberately *not* an AST or a pretty-printer: it has no
//! knowledge of Swift, Kotlin, C++, or any other grammar, and it never
//! rewrites text it is given. Whatever is written is what is emitted.
//!
//! ```
//! use nexa_codegen::SourceWriter;
//!
//! let mut out = SourceWriter::new();
//! out.line_at(1, format_args!("let answer = {}", 42));
//! out.line(format_args!("print(answer)"));
//! assert_eq!(out.as_str(), "    let answer = 42\nprint(answer)\n");
//! ```
//!
//! # Migration
//!
//! The two statements this type replaces are mechanical:
//!
//! ```ignore
//! // before
//! indent(out, depth);
//! out.push_str(&format!("Text({})\n", expression(label)));
//!
//! // after
//! out.line_at(depth, format_args!("Text({})", expression(label)));
//! ```
//!
//! `line`/`line_at` are infallible by construction (a `String` cannot fail to
//! grow), so they return `()` and no generator grows a `Result` it did not
//! have. For the rarer case of a fragment with no trailing newline, use
//! [`SourceWriter::text`] and [`SourceWriter::text_at`].
//!
//! `Deref`/`DerefMut` to `String` are provided as a migration affordance:
//! existing literal `push_str`/`push` calls keep working unchanged, so writers
//! can be adopted file by file without rewriting every line at once. Use
//! [`SourceWriter::as_str`] where a `&str` is required.

use std::fmt::{self, Arguments, Write};
use std::ops::{Deref, DerefMut};

/// The indentation unit generated sources use.
const INDENT: &str = "    ";

/// An append-only buffer for generated source text.
///
/// See the [module documentation](self) for the migration patterns.
#[derive(Clone, Debug, Default)]
pub struct SourceWriter {
    buffer: String,
    next_id: usize,
}

impl SourceWriter {
    /// Creates an empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty writer that reserves room for `capacity` bytes.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            next_id: 0,
        }
    }

    /// Returns an identifier that is unique within this writer.
    ///
    /// Generated code sometimes needs a name that cannot collide -- a Compose
    /// `remember` key or a SwiftUI state holder for one element of a repeated
    /// list. Seeding those names from the buffer length would tie them to how
    /// the file happens to be assembled, so a purely cosmetic change to
    /// formatting would rename them. A counter keeps them stable and unique
    /// within the file being written.
    pub fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Writes `args` followed by a newline.
    pub fn line(&mut self, args: Arguments<'_>) {
        push_args(&mut self.buffer, args);
        self.buffer.push('\n');
    }

    /// Writes `args` at `depth`, followed by a newline.
    pub fn line_at(&mut self, depth: usize, args: Arguments<'_>) {
        self.indent(depth);
        self.line(args);
    }

    /// Writes `args` with no trailing newline.
    pub fn text(&mut self, args: Arguments<'_>) {
        push_args(&mut self.buffer, args);
    }

    /// Writes `args` at `depth`, with no trailing newline.
    pub fn text_at(&mut self, depth: usize, args: Arguments<'_>) {
        self.indent(depth);
        self.text(args);
    }

    /// Writes the indentation prefix for `depth`. Prefer [`Self::line_at`]
    /// when the indent is immediately followed by a line.
    pub fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.buffer.push_str(INDENT);
        }
    }

    /// Writes an empty line.
    pub fn blank_line(&mut self) {
        self.buffer.push('\n');
    }

    /// The text written so far.
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Consumes the writer, returning the generated text.
    pub fn finish(self) -> String {
        self.buffer
    }
}

impl Deref for SourceWriter {
    type Target = String;

    fn deref(&self) -> &String {
        &self.buffer
    }
}

impl DerefMut for SourceWriter {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.buffer
    }
}

/// Lets `write!`/`writeln!` target a writer directly. Prefer
/// [`SourceWriter::line`] and [`SourceWriter::line_at`] in generators: those
/// are infallible, while these return a [`fmt::Result`] that can only ever
/// hold `Ok`.
impl Write for SourceWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.buffer.push_str(value);
        Ok(())
    }
}

/// Formats `args` directly into `buffer`.
///
/// `fmt::Write` is implemented for `String` and cannot fail, so the `Result`
/// carries no information. It is discarded rather than propagated: that is
/// what lets `line`/`text` be infallible and keeps generators free of
/// `Result` plumbing they did not previously have.
fn push_args(buffer: &mut String, args: Arguments<'_>) {
    let _ = buffer.write_fmt(args);
}

#[cfg(test)]
mod tests {
    use super::SourceWriter;
    use std::fmt::Write;

    #[test]
    fn line_writes_arguments_and_a_newline() {
        let mut out = SourceWriter::new();
        out.line(format_args!("Text({})", "\"hi\""));
        out.blank_line();
        assert_eq!(out.as_str(), "Text(\"hi\")\n\n");
    }

    #[test]
    fn line_at_indents_by_four_spaces_per_level() {
        let mut out = SourceWriter::new();
        out.line_at(0, format_args!("struct S {{"));
        out.line_at(1, format_args!("var x = 1"));
        out.line_at(3, format_args!("}}"));
        assert_eq!(out.as_str(), "struct S {\n    var x = 1\n            }\n");
    }

    #[test]
    fn text_omits_the_newline() {
        let mut out = SourceWriter::new();
        out.text_at(1, format_args!("Button(onClick = {{"));
        out.text(format_args!(" }}"));
        out.blank_line();
        assert_eq!(out.as_str(), "    Button(onClick = { }\n");
    }

    #[test]
    fn fmt_write_composes_with_the_helpers() {
        let mut out = SourceWriter::new();
        write!(out, "import Foundation").expect("writing to a String cannot fail");
        out.blank_line();
        out.line_at(1, format_args!("struct A {{}}"));
        // `write!` left the line unterminated, so one `blank_line` closes it.
        assert_eq!(out.as_str(), "import Foundation\n    struct A {}\n");
    }

    #[test]
    fn deref_keeps_literal_push_str_working() {
        // The migration affordance: unconverted call sites still compile.
        let mut out = SourceWriter::new();
        out.push_str("let x = ");
        out.push('1');
        out.push('\n');
        out.line_at(1, format_args!("let y = 2"));
        assert_eq!(out.as_str(), "let x = 1\n    let y = 2\n");
        assert_eq!(out.len(), out.as_str().len());
    }

    #[test]
    fn ids_are_unique_per_writer_and_independent_of_length() {
        let mut out = SourceWriter::new();
        out.line(format_args!("first"));
        let first = out.next_id();
        out.line(format_args!("a much longer second line than the first one"));
        let second = out.next_id();
        assert_eq!((first, second), (0, 1));
        // A second writer restarts numbering: ids only need to be unique in the
        // file being written.
        assert_eq!(SourceWriter::new().next_id(), 0);
    }

    #[test]
    fn finish_returns_the_buffer() {
        let mut out = SourceWriter::with_capacity(16);
        out.line(format_args!("done"));
        assert_eq!(out.finish(), "done\n");
    }
}
