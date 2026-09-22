/// Emits direct iOS permission status and request queries for the typed Permissions API.
///
/// The generated helper only exists when a module calls the typed Permissions API.
/// Each request stays inside the platform authorization framework and returns
/// the same compact status enum as status queries.
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

private final class NexaLocationRequester: NSObject, CLLocationManagerDelegate {
    private lazy var manager = CLLocationManager()
    private var continuation: CheckedContinuation<NexaPermissionStatus, Never>?

    func request() async -> NexaPermissionStatus {
        let current = Self.status()
        if current != .notDetermined {
            return current
        }
        return await withCheckedContinuation { continuation in
            self.continuation = continuation
            manager.delegate = self
            manager.requestWhenInUseAuthorization()
        }
    }

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        finish()
    }

    func locationManager(_ manager: CLLocationManager, didChangeAuthorization status: CLAuthorizationStatus) {
        finish()
    }

    private func finish() {
        let current = Self.status()
        guard current != .notDetermined, let continuation else {
            return
        }
        self.continuation = nil
        continuation.resume(returning: current)
    }

    private static func status() -> NexaPermissionStatus {
        switch CLLocationManager.authorizationStatus() {
        case .authorizedAlways, .authorizedWhenInUse: return .granted
        case .denied: return .denied
        case .restricted: return .restricted
        case .notDetermined: return .notDetermined
        @unknown default: return .denied
        }
    }
}

private final class NexaBluetoothRequester: NSObject, CBCentralManagerDelegate {
    private lazy var manager = CBCentralManager(delegate: self, queue: nil)
    private var continuation: CheckedContinuation<NexaPermissionStatus, Never>?

    func request() async -> NexaPermissionStatus {
        let current = Self.status()
        if current != .notDetermined {
            return current
        }
        _ = manager
        return await withCheckedContinuation { continuation in
            self.continuation = continuation
        }
    }

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        let current = Self.status()
        guard current != .notDetermined, let continuation else {
            return
        }
        self.continuation = nil
        continuation.resume(returning: current)
    }

    private static func status() -> NexaPermissionStatus {
        switch CBManager.authorization {
        case .allowedAlways: return .granted
        case .denied: return .denied
        case .restricted: return .restricted
        case .notDetermined: return .notDetermined
        @unknown default: return .denied
        }
    }
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

    static func request(_ permission: NexaPermission) async -> NexaPermissionStatus {
        switch permission {
        case .Camera:
            return await AVCaptureDevice.requestAccess(for: .video) ? .granted : .denied
        case .Microphone:
            let granted = await withCheckedContinuation { continuation in
                AVAudioSession.sharedInstance().requestRecordPermission { granted in
                    continuation.resume(returning: granted)
                }
            }
            return granted ? .granted : .denied
        case .Photos:
            let authorization = await withCheckedContinuation { continuation in
                PHPhotoLibrary.requestAuthorization(for: .readWrite) { status in
                    continuation.resume(returning: status)
                }
            }
            switch authorization {
            case .authorized, .limited: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
        case .Location:
            return await NexaLocationRequester().request()
        case .Notifications:
            let granted = (try? await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound])) ?? false
            return granted ? .granted : .denied
        case .Contacts:
            let granted = await withCheckedContinuation { continuation in
                CNContactStore().requestAccess(for: .contacts) { granted, _ in
                    continuation.resume(returning: granted)
                }
            }
            return granted ? .granted : .denied
        case .Calendar:
            let granted = await withCheckedContinuation { continuation in
                EKEventStore().requestAccess(to: .event) { granted, _ in
                    continuation.resume(returning: granted)
                }
            }
            return granted ? .granted : .denied
        case .Bluetooth:
            return await NexaBluetoothRequester().request()
        }
    }
}
"#,
    );
}
