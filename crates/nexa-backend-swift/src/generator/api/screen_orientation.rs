//! Screen orientation requests backed by the active UIKit scene.

use nexa_codegen::SourceWriter;

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Requests an orientation change for the active app scene.
@MainActor
enum NexaScreen {
    static func lockOrientation(_ mode: String) {
        let mask: UIInterfaceOrientationMask
        switch mode {
        case "Portrait":
            mask = .portrait
        case "Landscape":
            mask = .landscape
        default:
            mask = UIDevice.current.userInterfaceIdiom == .pad ? .all : .allButUpsideDown
        }

        guard let scene = UIApplication.shared.connectedScenes
            .compactMap({ $0 as? UIWindowScene })
            .first(where: { $0.activationState == .foregroundActive })
        else {
            return
        }
        scene.requestGeometryUpdate(.iOS(interfaceOrientations: mask)) { _ in }
    }
}
"#,
    );
}
