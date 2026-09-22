/// Emits the tiny application-context holder shared by generated native APIs.
pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
public object NexaRuntime {
    @Volatile private var applicationContext: android.content.Context? = null

    public fun bind(context: android.content.Context) {
        val application = context.applicationContext
        if (applicationContext !== application) applicationContext = application
    }

    public fun context(): android.content.Context = requireNotNull(applicationContext) {
        "NexaRuntime.bind must run before a native API call"
    }
}
"#,
    );
}
