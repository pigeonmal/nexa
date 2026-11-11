//! Swift type spellings for the shared IR.
//!
//! These mappings live in the backend (not in `nexa-ir`) because they are
//! platform output, not semantic facts: the IR knows *what* a type is, and
//! each backend decides how to spell it.

use nexa_codegen::{
    SourceWriter,
    names::{enum_name, struct_name},
};
use nexa_ir::{Module, NumericType, Type};

/// Swift spelling of a numeric type.
pub(crate) fn swift_numeric(ty: NumericType) -> &'static str {
    match ty {
        NumericType::Int8 => "Int8",
        NumericType::Int16 => "Int16",
        NumericType::Int32 => "Int32",
        NumericType::Int64 => "Int64",
        NumericType::UInt8 => "UInt8",
        NumericType::UInt16 => "UInt16",
        NumericType::UInt32 => "UInt32",
        NumericType::UInt64 => "UInt64",
        NumericType::Float32 => "Float",
        NumericType::Float64 => "Double",
    }
}

/// Swift spelling of a shared IR type.
pub(crate) fn swift_type(ty: &Type) -> String {
    match ty {
        Type::Void => "Void".to_owned(),
        Type::String => "String".to_owned(),
        Type::Bytes => "Data".to_owned(),
        Type::Bool => "Bool".to_owned(),
        Type::Numeric(numeric) => swift_numeric(*numeric).to_owned(),
        Type::Optional(inner) => format!("{}?", swift_type(inner)),
        Type::Result(value, error) => {
            format!("Result<{}, {}>", swift_type(value), swift_type(error))
        }
        Type::Array(element) => format!("[{}]", swift_type(element)),
        Type::Set(element) => format!("Set<{}>", swift_type(element)),
        Type::Map(key, value) => format!("[{}: {}]", swift_type(key), swift_type(value)),
        Type::Pair(first, second) => {
            format!("({}, {})", swift_type(first), swift_type(second))
        }
        Type::Triple(first, second, third) => format!(
            "({}, {}, {})",
            swift_type(first),
            swift_type(second),
            swift_type(third)
        ),
        Type::Enum(name) => enum_name(name),
        // Every plugin type parameter is bound at its call site, so a bound
        // value type is what reaches code generation.
        Type::TypeParam(name) => {
            debug_assert!(false, "unbound plugin value type parameter reached codegen");
            name.clone()
        }
        Type::Plugin { namespace, name } if namespace == "Regex" && name == "Regex" => {
            "NexaRegex".to_owned()
        }
        Type::Plugin { namespace, name } if namespace == "Regex" && name == "RegexMatch" => {
            "NexaRegexMatch".to_owned()
        }
        Type::Plugin { namespace, name } if namespace == "Regex" && name == "Range" => {
            "NexaRegexRange".to_owned()
        }
        Type::Plugin { name, .. } => name.clone(),
        Type::TaskHandle => "Task<Void, Never>".to_owned(),
        Type::NetworkResponse => "NexaNetworkResponse".to_owned(),
        Type::Struct { name, .. } => struct_name(name),
        Type::Class { name, .. } => struct_name(name),
        Type::Signal(inner) => format!("NexaSignal<{}>", swift_type(inner)),
        Type::ComponentRef { namespace, name } => {
            if namespace == "Nexa" {
                format!("{}Ref", struct_name(name))
            } else {
                format!("{name}Ref")
            }
        }
        Type::ComponentRefError(failure) => {
            let failure = if matches!(failure.as_ref(), Type::Void) {
                "Never".to_owned()
            } else {
                swift_type(failure)
            };
            format!("NexaComponentRefError<{failure}>")
        }
    }
}

/// Every scalar type `.nx` can name, paired with its Swift spelling.
///
/// Published as the Swift column of the primitive type table in
/// `docs/language-guide.md`. Exposed because the mapping is documentation as
/// well as code generation: reading both from one table is what stops the guide
/// from disagreeing with the spellings this backend emits.
pub fn swift_scalar_types() -> Vec<(&'static str, String)> {
    let mut types = vec![
        ("String", swift_type(&Type::String)),
        ("Bool", swift_type(&Type::Bool)),
        ("Bytes", swift_type(&Type::Bytes)),
        ("Void", swift_type(&Type::Void)),
    ];
    types.extend(
        NumericType::ALL
            .iter()
            .map(|numeric| (numeric.nexa_name(), swift_numeric(*numeric).to_owned())),
    );
    types
}

/// Renders the module's enums as Swift `String`-backed error types.
///
/// Every `.nx` enum is lowered to a `String`-backed `Error` conformance
/// because a failed `Result` must carry the case name across the native
/// boundary without any runtime type information.
pub(crate) fn render_enums(module: &Module, out: &mut SourceWriter) {
    for declaration in &module.enums {
        // `Int` raw values make a case's ordinal available, which is what a
        // generated value codec stores; the cases themselves keep their
        // source names.
        out.push_str(&format!(
            "enum {}: Int, Error {{\n",
            enum_name(&declaration.name)
        ));
        for (index, case) in declaration.cases.iter().enumerate() {
            out.push_str(&format!("    case {case} = {index}\n"));
        }
        out.push_str("}\n\n");
    }
}

pub(crate) fn render_component_ref_types(out: &mut SourceWriter) {
    out.push_str(
        r#"
public extension Result {
    var isSuccess: Bool {
        switch self {
        case .success: return true
        case .failure: return false
        }
    }

    var value: Success? {
        switch self {
        case .success(let value): return value
        case .failure: return nil
        }
    }

    var error: Failure? {
        switch self {
        case .success: return nil
        case .failure(let error): return error
        }
    }
}

public enum NexaComponentRefError<Failure: Error>: Error {
    case notAttached
    case failure(Failure)
}

@MainActor
public final class NexaFastListRef {
    private weak var attachedView: AnyObject?
    private weak var nativeScrollView: AnyObject?
    private var isAttached = false
    private var indexAction: ((Int32, Bool, Double) -> Void)?
    private var topAction: ((Bool) -> Void)?
    private var endAction: ((Bool) -> Void)?
    private var offsetAction: ((Double, Bool) -> Void)?

    public init() {}

    func attach(
        view: AnyObject,
        scrollToIndex: @escaping (Int32, Bool, Double) -> Void,
        scrollToTop: @escaping (Bool) -> Void,
        scrollToEnd: @escaping (Bool) -> Void,
        scrollToOffset: @escaping (Double, Bool) -> Void
    ) {
        attachedView = view
        isAttached = true
        indexAction = scrollToIndex
        topAction = scrollToTop
        endAction = scrollToEnd
        offsetAction = scrollToOffset
    }

    func detach() {
        attachedView = nil
        nativeScrollView = nil
        isAttached = false
        indexAction = nil
        topAction = nil
        endAction = nil
        offsetAction = nil
    }

    func updateNativeScrollView(_ view: AnyObject?) {
        nativeScrollView = view
    }

    func nativeScrollViewObject() -> AnyObject? {
        nativeScrollView
    }

    public func scrollToIndex(
        _ index: Int32,
        animated: Bool = true,
        viewPosition: Double = 0.0
    ) -> Result<Void, NexaComponentRefError<Never>> {
        guard isAttached, attachedView != nil, let indexAction else { return .failure(.notAttached) }
        let normalizedPosition = viewPosition.isFinite ? min(max(viewPosition, 0.0), 1.0) : 0.0
        indexAction(index, animated, normalizedPosition)
        return .success(())
    }

    public func scrollToTop(animated: Bool = true) -> Result<Void, NexaComponentRefError<Never>> {
        guard isAttached, attachedView != nil, let topAction else { return .failure(.notAttached) }
        topAction(animated)
        return .success(())
    }

    public func scrollToEnd(animated: Bool = true) -> Result<Void, NexaComponentRefError<Never>> {
        guard isAttached, attachedView != nil, let endAction else { return .failure(.notAttached) }
        endAction(animated)
        return .success(())
    }

    public func scrollToOffset(
        _ offset: Double,
        animated: Bool = true
    ) -> Result<Void, NexaComponentRefError<Never>> {
        guard isAttached, attachedView != nil, offset.isFinite, let offsetAction else { return .failure(.notAttached) }
        offsetAction(max(0, offset), animated)
        return .success(())
    }
}

@MainActor
public final class NexaScrollViewRef {
    private var generation: UInt64 = 0
    private weak var attachedView: AnyObject?
    private var topAction: ((Bool) -> Void)?
    private var endAction: ((Bool) -> Void)?
    private var offsetAction: ((Double, Bool) -> Void)?

    public init() {}

    @discardableResult
    func attach(
        view: AnyObject,
        scrollToTop: @escaping (Bool) -> Void,
        scrollToEnd: @escaping (Bool) -> Void,
        scrollToOffset: @escaping (Double, Bool) -> Void
    ) -> UInt64 {
        generation &+= 1
        attachedView = view
        topAction = scrollToTop
        endAction = scrollToEnd
        offsetAction = scrollToOffset
        return generation
    }

    func detach(generation expected: UInt64) {
        guard generation == expected else { return }
        attachedView = nil
        topAction = nil
        endAction = nil
        offsetAction = nil
    }

    public func scrollToTop(animated: Bool = true) -> Result<Void, NexaComponentRefError<Never>> {
        guard attachedView != nil, let topAction else { return .failure(.notAttached) }
        topAction(animated)
        return .success(())
    }

    public func scrollToEnd(animated: Bool = true) -> Result<Void, NexaComponentRefError<Never>> {
        guard attachedView != nil, let endAction else { return .failure(.notAttached) }
        endAction(animated)
        return .success(())
    }

    public func scrollToOffset(
        _ offset: Double,
        animated: Bool = true
    ) -> Result<Void, NexaComponentRefError<Never>> {
        guard attachedView != nil, let offsetAction else { return .failure(.notAttached) }
        offsetAction(offset.isFinite ? max(0, offset) : 0, animated)
        return .success(())
    }
}

@MainActor
public final class NexaTextInputRef {
    private var generation: UInt64 = 0
    private weak var attachedView: AnyObject?
    private var focusAction: (() -> Void)?
    private var blurAction: (() -> Void)?

    public init() {}

    @discardableResult
    func attach(
        view: AnyObject,
        focus: @escaping () -> Void,
        blur: @escaping () -> Void
    ) -> UInt64 {
        generation &+= 1
        attachedView = view
        focusAction = focus
        blurAction = blur
        return generation
    }

    func detach(generation expected: UInt64) {
        guard generation == expected else { return }
        attachedView = nil
        focusAction = nil
        blurAction = nil
    }

    public func focus() -> Result<Void, NexaComponentRefError<Never>> {
        guard attachedView != nil, let focusAction else { return .failure(.notAttached) }
        focusAction()
        return .success(())
    }

    public func blur() -> Result<Void, NexaComponentRefError<Never>> {
        guard attachedView != nil, let blurAction else { return .failure(.notAttached) }
        blurAction()
        return .success(())
    }
}

@MainActor
public final class NexaBottomSheetRef {
    private var generation: UInt64 = 0
    private var presentAction: (() -> Void)?
    private var dismissAction: (() -> Void)?

    public init() {}

    @discardableResult
    func attach(present: @escaping () -> Void, dismiss: @escaping () -> Void) -> UInt64 {
        generation &+= 1
        presentAction = present
        dismissAction = dismiss
        return generation
    }

    func detach(generation expected: UInt64) {
        guard generation == expected else { return }
        presentAction = nil
        dismissAction = nil
    }

    public func present() -> Result<Void, NexaComponentRefError<Never>> {
        guard let presentAction else { return .failure(.notAttached) }
        presentAction()
        return .success(())
    }

    public func dismiss() -> Result<Void, NexaComponentRefError<Never>> {
        guard let dismissAction else { return .failure(.notAttached) }
        dismissAction()
        return .success(())
    }
}
"#,
    );
}

/// Renders the `NexaNavigationRoute` enum that types the navigation path.
///
/// Each case carries a `UUID` route identity ahead of the screen's parameters.
/// SwiftUI needs that identity to distinguish two pushes of the same screen
/// with different arguments, which a payload-only enum cannot express.
pub(crate) fn render_navigation_routes(module: &Module, out: &mut SourceWriter) {
    if module.screens.is_empty() {
        return;
    }
    out.push_str("private enum NexaNavigationRoute: Hashable {\n");
    for screen in &module.screens {
        let case_name = nexa_codegen::names::navigation_case_name(screen.id);
        let mut payload_types = vec!["UUID".to_owned()];
        payload_types.extend(
            screen
                .parameters
                .iter()
                .map(|parameter| swift_type(&parameter.ty)),
        );
        out.push_str(&format!(
            "    case {case_name}({})\n",
            payload_types.join(", ")
        ));
    }
    out.push_str("\n    var identity: UUID {\n");
    out.push_str("        switch self {\n");
    for screen in &module.screens {
        let case_name = nexa_codegen::names::navigation_case_name(screen.id);
        if screen.parameters.is_empty() {
            out.push_str(&format!("        case let .{case_name}(id): return id\n"));
        } else {
            let wildcards = vec!["_"; screen.parameters.len()].join(", ");
            out.push_str(&format!(
                "        case let .{case_name}(id, {wildcards}): return id\n"
            ));
        }
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    render_navigation_path_helpers(out);
}

fn render_navigation_path_helpers(out: &mut SourceWriter) {
    out.push_str(
        r#"private struct NexaNavigationPathEnvironmentKey: EnvironmentKey {
    static let defaultValue: Binding<[NexaNavigationRoute]> = .constant([])
}

private extension EnvironmentValues {
    var nexaNavigationPath: Binding<[NexaNavigationRoute]> {
        get { self[NexaNavigationPathEnvironmentKey.self] }
        set { self[NexaNavigationPathEnvironmentKey.self] = newValue }
    }
}

private struct NexaNavigationPathHost<Content: View>: View {
    @State private var path: [NexaNavigationRoute] = []
    @State private var rootIdentity = UUID()
    private let content: (Binding<[NexaNavigationRoute]>, UUID) -> Content

    init(@ViewBuilder content: @escaping (Binding<[NexaNavigationRoute]>, UUID) -> Content) {
        self.content = content
    }

    var body: some View {
        content($path, rootIdentity)
    }
}

@MainActor
private struct NexaHideTabBarModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 18.0, *) {
            content
                .toolbarVisibility(.hidden, for: .tabBar)
                .toolbar(.hidden, for: .tabBar)
        } else {
            content
                .toolbar(.hidden, for: .tabBar)
        }
    }
}

@MainActor
private struct NexaNavigationBackSwipeModifier: ViewModifier {
    @Binding var path: [NexaNavigationRoute]

    func body(content: Content) -> some View {
        content.background(
            NexaNavigationBackGestureProbe(path: $path)
                .frame(width: 1, height: 1)
                .allowsHitTesting(false)
        )
    }
}

@MainActor
private struct NexaNavigationBackGestureProbe: UIViewRepresentable {
    @Binding var path: [NexaNavigationRoute]

    func makeCoordinator() -> Coordinator {
        Coordinator(path: $path)
    }

    func makeUIView(context: Context) -> ProbeView {
        let view = ProbeView()
        view.coordinator = context.coordinator
        return view
    }

    func updateUIView(_ view: ProbeView, context: Context) {
        context.coordinator.path = $path
        view.coordinator = context.coordinator
        view.installGesture()
    }

    @MainActor
    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        var path: Binding<[NexaNavigationRoute]>

        init(path: Binding<[NexaNavigationRoute]>) {
            self.path = path
        }

        func install(from view: UIView) {
            var responder: UIResponder? = view
            while let current = responder {
                let nav = (current as? UINavigationController) ?? (current as? UIViewController)?.navigationController
                if let navigationController = nav,
                   let popGesture = navigationController.interactivePopGestureRecognizer {
                    popGesture.delegate = self
                    popGesture.isEnabled = true
                    return
                }
                responder = current.next
            }
            if let root = view.window?.rootViewController {
                if let nav = findNavController(in: root) {
                    nav.interactivePopGestureRecognizer?.delegate = self
                    nav.interactivePopGestureRecognizer?.isEnabled = true
                }
            }
        }

        private func findNavController(in root: UIViewController) -> UINavigationController? {
            if let nav = root as? UINavigationController {
                return nav
            }
            for child in root.children {
                if let found = findNavController(in: child) {
                    return found
                }
            }
            return nil
        }

        func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
            guard !path.wrappedValue.isEmpty else { return false }
            guard let pan = gestureRecognizer as? UIPanGestureRecognizer,
                  let view = gestureRecognizer.view else { return true }
            let velocity = pan.velocity(in: view)
            let isLeadingEdgeSwipe = view.effectiveUserInterfaceLayoutDirection == .rightToLeft
                ? velocity.x < 0
                : velocity.x > 0
            return isLeadingEdgeSwipe && abs(velocity.x) > abs(velocity.y)
        }
    }

    @MainActor
    final class ProbeView: UIView {
        weak var coordinator: Coordinator?

        override func didMoveToWindow() {
            super.didMoveToWindow()
            DispatchQueue.main.async { [weak self] in
                self?.installGesture()
            }
        }

        func installGesture() {
            coordinator?.install(from: self)
        }
    }
}

"#,
    );
}

#[cfg(test)]
mod component_ref_type_tests {
    use super::swift_type;
    use nexa_ir::Type;

    #[test]
    fn component_refs_use_the_builtin_or_plugin_binding_name() {
        assert_eq!(
            swift_type(&Type::ComponentRef {
                namespace: "Nexa".to_owned(),
                name: "FastList".to_owned(),
            }),
            "NexaFastListRef"
        );
        assert_eq!(
            swift_type(&Type::ComponentRef {
                namespace: "Media".to_owned(),
                name: "VideoView".to_owned(),
            }),
            "VideoViewRef"
        );
    }
}
