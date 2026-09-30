//! App-private string storage backed by an isolated SharedPreferences file.

use nexa_codegen::SourceWriter;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let enabled = features.facts.capabilities.uses_storage_api;
    imports.add(enabled, "android.content.Context");
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// App-private string storage in a dedicated SharedPreferences file.
internal object NexaStorage {
    private const val PREFERENCES_NAME = "dev.nexa.storage.v1"

    private fun preferences(context: Context) =
        context.applicationContext.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)

    fun getString(context: Context, key: String): String? =
        preferences(context).getString(key, null)

    fun setString(context: Context, key: String, value: String) {
        preferences(context).edit().putString(key, value).apply()
    }

    fun delete(context: Context, key: String) {
        preferences(context).edit().remove(key).apply()
    }

    fun clear(context: Context) {
        preferences(context).edit().clear().apply()
    }
}
"#,
    );
}
