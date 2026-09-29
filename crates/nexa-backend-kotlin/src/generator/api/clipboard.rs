//! Core text clipboard access backed by Android's system clipboard service.

use nexa_codegen::SourceWriter;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let enabled = features.facts.capabilities.uses_clipboard_api;
    imports.add(enabled, "android.content.ClipData");
    imports.add(enabled, "android.content.ClipDescription");
    imports.add(enabled, "android.content.ClipboardManager");
    imports.add(enabled, "android.content.Context");
    imports.add(enabled, "androidx.compose.ui.platform.LocalContext");
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Core text clipboard access.
internal object NexaClipboard {
    private fun manager(context: Context): ClipboardManager =
        context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager

    fun setText(context: Context, text: String) {
        manager(context).setPrimaryClip(ClipData.newPlainText(null, text))
    }

    fun getText(context: Context): String? {
        val clipboard = manager(context)
        val item = clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
        return item?.coerceToText(context)?.toString()
    }

    fun hasText(context: Context): Boolean {
        val description = manager(context).primaryClipDescription ?: return false
        return description.hasMimeType(ClipDescription.MIMETYPE_TEXT_PLAIN) ||
            description.hasMimeType(ClipDescription.MIMETYPE_TEXT_HTML)
    }
}
"#,
    );
}
