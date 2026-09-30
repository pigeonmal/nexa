use nexa_codegen::SourceWriter;

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"struct NexaSharedNamespaceEnvironmentKey: EnvironmentKey {
    static let defaultValue: Namespace.ID? = nil
}

extension EnvironmentValues {
    var nexaSharedNamespace: Namespace.ID? {
        get { self[NexaSharedNamespaceEnvironmentKey.self] }
        set { self[NexaSharedNamespaceEnvironmentKey.self] = newValue }
    }
}

struct NexaSharedElementModifier: ViewModifier {
    @Environment(\.nexaSharedNamespace) private var namespace
    let id: String

    @ViewBuilder
    func body(content: Content) -> some View {
        if let namespace {
            content.matchedGeometryEffect(id: id, in: namespace)
        } else {
            content
        }
    }
}

extension View {
    func nexaSharedElement(id: String) -> some View {
        modifier(NexaSharedElementModifier(id: id))
    }
}

"#,
    );
}
