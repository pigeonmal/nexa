//! Core text clipboard access backed by UIKit's system pasteboard.

use nexa_codegen::SourceWriter;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.facts.capabilities.uses_clipboard_api, "UIKit");
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Core text clipboard access.
enum NexaClipboard {
    static func setText(_ text: String) {
        UIPasteboard.general.string = text
    }

    static func getText() -> String? {
        UIPasteboard.general.string
    }

    static func hasText() -> Bool {
        UIPasteboard.general.hasStrings
    }
}
"#,
    );
}
