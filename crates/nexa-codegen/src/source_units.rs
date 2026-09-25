//! Structured output for the native source generators.
//!
//! The generators used to emit one concatenated string with `// nexa-unit:`
//! marker comments, and the CLI re-parsed that text to recover the individual
//! files. That round-trip made the unit boundaries a private convention
//! between producer and consumer: renaming a marker silently changed a file
//! name, and the consumer had to reimplement the header and access-level
//! rules to rebuild each file.
//!
//! [`SourceUnits`] makes the units real. A generator opens a named unit, gets
//! a [`SourceWriter`] for it, and the finished units carry their own file name
//! and body. Assembling those bodies into complete files is a separate,
//! explicit step ([`GeneratedSources::into_files`]), because a host project
//! sometimes needs to adjust the import block first -- plugin bindings add
//! imports that every file must see.
//!
//! ```
//! use nexa_codegen::SourceUnits;
//!
//! let mut units = SourceUnits::new("swift");
//! units.set_imports("import SwiftUI\n");
//! units.write("types", |out| out.line(format_args!("// types")));
//! units.write("app", |out| out.line(format_args!("struct App {{}}")));
//! let generated = units.finish();
//!
//! assert_eq!(generated.units.len(), 2);
//! let header = generated.header_with(&[]);
//! let files = generated.into_files_with_header(&header, "");
//! assert_eq!(files[0].name, "NexaGenerated.swift");
//! assert_eq!(files[0].contents, "import SwiftUI\n\nstruct App {}\n");
//! ```
//!
//! Every file repeats the imports, because each unit becomes its own file and
//! each file needs its imports. Units whose body is entirely blank are dropped
//! rather than written as an empty file.

use crate::SourceWriter;

/// One generated source file: its file name and full contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceUnit {
    /// File name, including the extension.
    pub name: String,
    /// Complete file contents.
    pub contents: String,
}

/// The pieces of a generated target, before file assembly.
pub struct GeneratedSources {
    /// The shared import block, one `import` per line.
    pub imports: String,
    /// Non-import header lines rendered ahead of every unit, such as a shared
    /// storage helper. These are not access-level lowered: they are emitted
    /// exactly as the generator wrote them.
    pub preamble: String,
    /// Unit bodies in emission order, each already lowered and named.
    pub units: Vec<SourceUnit>,
}

impl GeneratedSources {
    /// The backend's import block merged with `extra`, sorted and deduplicated.
    ///
    /// Sorting is what makes the import block deterministic; the caller owns
    /// the result, because a host project may need to place it in a larger
    /// header (a `package` line, plugin imports) rather than use it alone.
    pub fn merged_imports(&self, extra: &[&str]) -> String {
        let mut imports: Vec<&str> = self
            .imports
            .lines()
            .filter(|line| line.starts_with("import "))
            .collect();
        imports.extend(extra.iter().copied());
        imports.sort_unstable();
        imports.dedup();
        imports.join("\n")
    }

    /// The generator's own header: the merged import block, then any preamble.
    ///
    /// Use this when the imports and preamble are the whole per-file header. A
    /// host project that needs to wrap them -- a `package` line, plugin imports
    /// -- builds its own header and calls
    /// [`Self::into_files_with_header`] instead.
    pub fn header_with(&self, extra_imports: &[&str]) -> String {
        if !extra_imports.is_empty() {
            // A caller-supplied import changes the block's shape, so it is
            // rebuilt. Two blank lines separate the imports from the preamble:
            // one ends the import block, and one closes the blank line the
            // generator's own import block ends with.
            let imports = self.merged_imports(extra_imports);
            let mut lines: Vec<&str> = Vec::new();
            if !imports.is_empty() {
                lines.extend(imports.lines());
            }
            lines.push("");
            lines.push("");
            lines.extend(self.preamble.lines());
            return lines.join("\n");
        }
        // Without extra imports the generator's own strings are already the
        // header, spacing included. Exactly one trailing newline is dropped:
        // that newline separates the header from the body, which the caller
        // adds back along with the blank line after it.
        let mut header = self.imports.clone();
        header.push_str(&self.preamble);
        if header.ends_with('\n') {
            header.pop();
        }
        header
    }

    /// Assembles files using the generator's own header, plus `extra_imports`
    /// and an optional trailing declaration.
    pub fn into_files(self, extra_imports: &[&str], trailing: &str) -> Vec<SourceUnit> {
        let header = self.header_with(extra_imports);
        self.into_files_with_header(&header, trailing)
    }

    /// Assembles files against a caller-supplied header.
    ///
    /// `header` is written ahead of every unit body, followed by a blank line;
    /// pass [`Self::merged_imports`] output when the backend's own imports are
    /// the whole header. `trailing` is appended to the last unit in *emission*
    /// order, which is how a generator hands over an extra declaration that
    /// belongs at the end of the output.
    pub fn into_files_with_header(self, header: &str, trailing: &str) -> Vec<SourceUnit> {
        let mut units = self.units;
        if !trailing.is_empty()
            && let Some(last) = units.last_mut()
        {
            last.contents.push('\n');
            last.contents.push_str(trailing);
        }
        if !header.is_empty() {
            for unit in &mut units {
                let mut contents = String::with_capacity(header.len() + unit.contents.len() + 2);
                contents.push_str(header);
                contents.push_str("\n\n");
                contents.push_str(&unit.contents);
                unit.contents = contents;
            }
        }

        // The app unit is the primary generated file, so it is ordered first.
        if let Some(index) = units
            .iter()
            .position(|unit| unit.name.starts_with(APP_UNIT))
        {
            let app = units.remove(index);
            units.insert(0, app);
        }
        units
    }
}

/// File-name stem of the primary generated file.
const APP_UNIT: &str = "NexaGenerated.";

/// Accumulates named source units for one target language.
pub struct SourceUnits {
    extension: String,
    imports: String,
    preamble: String,
    current: Option<(String, SourceWriter)>,
    finished: Vec<(String, String)>,
}

impl SourceUnits {
    /// Creates a builder for units with the given file extension.
    pub fn new(extension: &str) -> Self {
        Self {
            extension: extension.to_owned(),
            imports: String::new(),
            preamble: String::new(),
            current: None,
            finished: Vec::new(),
        }
    }

    /// Sets the shared import block, one `import` per line.
    pub fn set_imports(&mut self, imports: &str) {
        self.imports = imports.to_owned();
    }

    /// Sets non-import header lines rendered ahead of every unit.
    pub fn set_preamble(&mut self, preamble: &str) {
        self.preamble = preamble.to_owned();
    }

    /// The writer for the unit currently being built.
    pub fn writer(&mut self) -> &mut SourceWriter {
        self.current
            .as_mut()
            .map(|(_, writer)| writer)
            .expect("a unit must be open before writing to it")
    }

    /// Closes the open unit and starts one with the given name.
    ///
    /// The returned writer borrows the builder, so prefer [`Self::write`]:
    /// the unit's borrow ends with the call, which is what lets a later unit
    /// open even when the earlier one is only conditionally written.
    pub fn begin(&mut self, name: &str) -> &mut SourceWriter {
        self.close();
        self.current = Some((name.to_owned(), SourceWriter::new()));
        self.writer()
    }

    /// Writes one unit, named `name`.
    pub fn write(&mut self, name: &str, body: impl FnOnce(&mut SourceWriter)) {
        self.begin(name);
        body(self.writer());
    }

    /// Writes one unit, leaving a blank line at the end of the previous one.
    ///
    /// Swift separates its units with a blank line; Kotlin does not.
    pub fn write_separated(&mut self, name: &str, body: impl FnOnce(&mut SourceWriter)) {
        if self.current.is_some() {
            self.writer().blank_line();
        }
        self.write(name, body)
    }

    /// Finishes the open unit, if any.
    fn close(&mut self) {
        if let Some((name, writer)) = self.current.take() {
            self.finished.push((name, writer.finish()));
        }
    }

    /// Closes the last unit and returns the generated pieces.
    ///
    /// Units with a blank body are dropped. The bodies are returned in
    /// emission order; [`GeneratedSources::into_files`] orders the files.
    pub fn finish(mut self) -> GeneratedSources {
        self.close();
        GeneratedSources {
            imports: self.imports,
            preamble: self.preamble,
            units: self
                .finished
                .into_iter()
                .filter(|(_, body)| !is_blank(body))
                .map(|(name, body)| SourceUnit {
                    name: unit_file_name(&name, &self.extension),
                    contents: lower_access(&body),
                })
                .collect(),
        }
    }
}

/// Names a unit's file. The `app` unit is the primary generated file; the rest
/// are auxiliary units named after the unit.
fn unit_file_name(unit: &str, extension: &str) -> String {
    if unit == "app" {
        format!("NexaGenerated.{extension}")
    } else {
        format!("NexaGenerated_{}.{extension}", unit.replace('-', "_"))
    }
}

/// Whether a unit body carries no declarations.
fn is_blank(body: &str) -> bool {
    body.lines().all(|line| line.trim().is_empty())
}

/// Lowers a `private` top-level declaration to `internal`.
///
/// Units compile as separate files, so a file-private declaration would be
/// invisible to the other units that reference it.
fn lower_access(body: &str) -> String {
    let mut contents = String::new();
    for line in body.lines() {
        if let Some(stripped) = line.strip_prefix("private ") {
            contents.push_str(stripped);
        } else {
            contents.push_str(line);
        }
        contents.push('\n');
    }
    contents
}

#[cfg(test)]
mod tests {
    use super::{GeneratedSources, SourceUnit, SourceUnits};

    /// Builds a `GeneratedSources` from literal unit bodies.
    fn generated(
        imports: &str,
        preamble: &str,
        bodies: &[(&str, &str)],
        extension: &str,
    ) -> GeneratedSources {
        let mut builder = SourceUnits::new(extension);
        builder.set_imports(imports);
        builder.set_preamble(preamble);
        for (name, body) in bodies {
            builder.begin(name).push_str(body);
        }
        builder.finish()
    }

    /// The common case: the backend's imports and preamble are the whole header.
    fn files(imports: &str, preamble: &str, bodies: &[(&str, &str)]) -> Vec<SourceUnit> {
        let sources = generated(imports, preamble, bodies, "swift");
        let header = sources.header_with(&[]);
        sources.into_files_with_header(&header, "")
    }

    fn names(units: &[SourceUnit]) -> Vec<&str> {
        units.iter().map(|unit| unit.name.as_str()).collect()
    }

    #[test]
    fn app_unit_is_named_and_ordered_first() {
        let built = files(
            "import SwiftUI\n",
            "",
            &[
                ("types", "// types\n"),
                ("app", "// app\n"),
                ("functions", "// fn\n"),
            ],
        );
        assert_eq!(
            names(&built),
            [
                "NexaGenerated.swift",
                "NexaGenerated_types.swift",
                "NexaGenerated_functions.swift"
            ]
        );
    }

    #[test]
    fn every_file_repeats_the_header_and_lowers_private() {
        let built = files(
            "import SwiftUI\n",
            "",
            &[("types", "private struct A {}\n")],
        );
        assert_eq!(built[0].contents, "import SwiftUI\n\nstruct A {}\n");
    }

    #[test]
    fn preamble_is_repeated_and_not_lowered() {
        let built = files(
            "import SwiftUI\n",
            "private final class Storage {}\n",
            &[("app", "let x = 1\n")],
        );
        assert_eq!(
            built[0].contents,
            "import SwiftUI\nprivate final class Storage {}\n\nlet x = 1\n"
        );
    }

    #[test]
    fn extra_imports_are_merged_and_sorted() {
        let sources = generated("import SwiftUI\n", "", &[("app", "// app\n")], "swift");
        let header = sources.merged_imports(&["import Foundation", "import Combine"]);
        assert_eq!(header, "import Combine\nimport Foundation\nimport SwiftUI");
        let built = sources.into_files_with_header(&header, "");
        assert_eq!(
            built[0].contents,
            "import Combine\nimport Foundation\nimport SwiftUI\n\n// app\n"
        );
    }

    #[test]
    fn a_caller_can_supply_its_own_header_layout() {
        // Kotlin needs a `package` line ahead of the import block, and plugin
        // imports between the two, which is exactly why the header is the
        // caller's to build.
        let sources = generated(
            "import androidx.compose.material3.Text\n",
            "",
            &[("app", "// app\n")],
            "kt",
        );
        let header = format!(
            "package dev.nexa.demo\n\nimport dev.nexa.plugin.*\n\n{}",
            sources.merged_imports(&[])
        );
        let built = sources.into_files_with_header(&header, "");
        assert_eq!(
            built[0].contents,
            "package dev.nexa.demo\n\nimport dev.nexa.plugin.*\n\nimport androidx.compose.material3.Text\n\n// app\n"
        );
    }

    #[test]
    fn trailing_declaration_lands_in_the_last_emitted_unit() {
        // Emission order is app then functions, so the trailing declaration
        // goes in the functions unit even though the app unit is ordered first.
        let sources = generated(
            "import SwiftUI\n",
            "",
            &[("app", "// app\n"), ("functions", "// fn\n")],
            "swift",
        );
        let header = sources.merged_imports(&[]);
        let built = sources.into_files_with_header(&header, "enum Config {}\n");
        assert!(built[0].contents.ends_with("// app\n"));
        let functions = built
            .iter()
            .find(|unit| unit.name == "NexaGenerated_functions.swift")
            .expect("functions unit exists");
        assert!(functions.contents.ends_with("// fn\n\nenum Config {}\n"));
    }

    #[test]
    fn only_the_leading_private_is_lowered() {
        let built = files(
            "",
            "",
            &[("types", "private struct A {}\n    private var x = 1\n")],
        );
        assert_eq!(built[0].contents, "struct A {}\n    private var x = 1\n");
    }

    #[test]
    fn blank_units_are_dropped() {
        let built = files(
            "import SwiftUI\n",
            "",
            &[("types", "\n  \n"), ("app", "// app\n")],
        );
        assert_eq!(names(&built), ["NexaGenerated.swift"]);
    }

    #[test]
    fn spacing_between_units_is_the_callers_choice() {
        let tight = files(
            "import SwiftUI\n",
            "",
            &[("app", "// app\n"), ("types", "// types\n")],
        );
        assert_eq!(tight[0].contents, "import SwiftUI\n\n// app\n");
        assert_eq!(tight[1].contents, "import SwiftUI\n\n// types\n");

        // Writing a blank line on the open unit before opening the next one
        // leaves the separator at the end of the earlier unit, which is where
        // the old `// nexa-unit:` markers put it.
        let mut builder = SourceUnits::new("swift");
        builder.set_imports("import SwiftUI\n");
        builder.begin("app").push_str("// app\n");
        builder.writer().blank_line();
        builder.begin("types").push_str("// types\n");
        let sources = builder.finish();
        let header = sources.merged_imports(&[]);
        let spaced = sources.into_files_with_header(&header, "");
        assert_eq!(spaced[0].contents, "import SwiftUI\n\n// app\n\n");
        assert_eq!(spaced[1].contents, "import SwiftUI\n\n// types\n");
    }

    #[test]
    fn hyphens_in_unit_names_become_underscores() {
        let built = files("", "", &[("list-runtime", "// x\n")]);
        assert_eq!(built[0].name, "NexaGenerated_list_runtime.swift");
    }

    #[test]
    fn an_empty_header_emits_no_separator() {
        let built = files("", "", &[("app", "// app\n")]);
        assert_eq!(built[0].contents, "// app\n");
    }

    #[test]
    fn a_whitespace_only_import_block_is_still_a_header() {
        // The header is a set of joined lines, so a blank-but-present import
        // block is a header like any other and keeps its separator.
        let built = files("   \n", "", &[("app", "// app\n")]);
        assert_eq!(built[0].contents, "   \n\n// app\n");
    }
}
