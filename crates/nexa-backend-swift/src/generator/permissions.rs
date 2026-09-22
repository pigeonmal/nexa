/// Emits direct iOS permission status queries for the typed `Permissions` API.
///
/// The generated helper only exists when a module calls `Permissions.status`.
/// Requests are intentionally a later slice because their native callback
/// lifecycles differ between authorization frameworks.
pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
private enum NexaPermission {
    case Camera
    case Microphone
    case Photos
    case Location
    case Notifications
    case Contacts
    case Calendar
    case Bluetooth
}

private enum NexaPermissionStatus {
    case granted
    case denied
    case restricted
    case notDetermined
}

private enum NexaPermissions {
    static func status(_ permission: NexaPermission) async -> NexaPermissionStatus {
        switch permission {
        case .Camera:
            switch AVCaptureDevice.authorizationStatus(for: .video) {
            case .authorized: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Microphone:
            switch AVAudioSession.sharedInstance().recordPermission {
            case .granted: return .granted
            case .denied: return .denied
            case .undetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Photos:
            switch PHPhotoLibrary.authorizationStatus(for: .readWrite) {
            case .authorized, .limited: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Location:
            switch CLLocationManager.authorizationStatus() {
            case .authorizedAlways, .authorizedWhenInUse: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Notifications:
            let settings = await UNUserNotificationCenter.current().notificationSettings()
            switch settings.authorizationStatus {
            case .authorized, .provisional, .ephemeral: return .granted
            case .denied: return .denied
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Contacts:
            switch CNContactStore.authorizationStatus(for: .contacts) {
            case .authorized: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Calendar:
            switch EKEventStore.authorizationStatus(for: .event) {
            case .authorized: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Bluetooth:
            switch CBManager.authorization {
            case .allowedAlways: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        }
    }
}
"#,
    );
}
