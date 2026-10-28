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

    private fun storageKey(type: String?, key: String): String =
        if (type == null) key else "$type:$key"

    private fun removeOtherTypes(
        editor: android.content.SharedPreferences.Editor,
        key: String,
        keeping: String?,
    ) {
        listOf(null, "bool", "int64", "float64", "json").filter { it != keeping }.forEach { type ->
            editor.remove(storageKey(type, key))
        }
    }

    fun getString(context: Context, key: String): String? =
        preferences(context).getString(key, null)

    fun setString(context: Context, key: String, value: String) {
        val editor = preferences(context).edit()
        removeOtherTypes(editor, key, keeping = null)
        editor.putString(storageKey(null, key), value).apply()
    }

    fun getJson(context: Context, key: String): String? =
        preferences(context).getString(storageKey("json", key), null)

    fun setJson(context: Context, key: String, value: String) {
        val editor = preferences(context).edit()
        removeOtherTypes(editor, key, keeping = "json")
        editor.putString(storageKey("json", key), value).apply()
    }

    fun getBool(context: Context, key: String): Boolean? {
        val preferences = preferences(context)
        val typedKey = storageKey("bool", key)
        return if (preferences.contains(typedKey)) preferences.getBoolean(typedKey, false) else null
    }

    fun setBool(context: Context, key: String, value: Boolean) {
        val editor = preferences(context).edit()
        removeOtherTypes(editor, key, keeping = "bool")
        editor.putBoolean(storageKey("bool", key), value).apply()
    }

    fun getInt64(context: Context, key: String): Long? {
        val preferences = preferences(context)
        val typedKey = storageKey("int64", key)
        return if (preferences.contains(typedKey)) preferences.getLong(typedKey, 0L) else null
    }

    fun setInt64(context: Context, key: String, value: Long) {
        val editor = preferences(context).edit()
        removeOtherTypes(editor, key, keeping = "int64")
        editor.putLong(storageKey("int64", key), value).apply()
    }

    fun getFloat64(context: Context, key: String): Double? {
        val preferences = preferences(context)
        val typedKey = storageKey("float64", key)
        return if (!preferences.contains(typedKey)) null else
            java.lang.Double.longBitsToDouble(preferences.getLong(typedKey, 0L))
    }

    fun setFloat64(context: Context, key: String, value: Double) {
        val editor = preferences(context).edit()
        removeOtherTypes(editor, key, keeping = "float64")
        editor.putLong(storageKey("float64", key), java.lang.Double.doubleToRawLongBits(value))
            .apply()
    }

    fun delete(context: Context, key: String) {
        val editor = preferences(context).edit()
        removeOtherTypes(editor, key, keeping = null)
        editor.remove(storageKey(null, key))
        editor.apply()
    }

    fun clear(context: Context) {
        preferences(context).edit().clear().apply()
    }

}
"#,
    );
}
