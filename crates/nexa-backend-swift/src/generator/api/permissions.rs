use std::collections::HashSet;

use nexa_ir::Permission;

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_permission(Permission::Camera)
            || features.uses_permission(Permission::Microphone),
        "AVFoundation",
    );
    imports.add(features.uses_permission(Permission::Contacts), "Contacts");
    imports.add(
        features.uses_permission(Permission::Bluetooth),
        "CoreBluetooth",
    );
    imports.add(
        features.uses_permission(Permission::Location),
        "CoreLocation",
    );
    imports.add(features.uses_permission(Permission::Calendar), "EventKit");
    imports.add(features.uses_permission(Permission::Photos), "Photos");
    imports.add(
        features.uses_permission(Permission::Notifications),
        "UserNotifications",
    );
}

const ALL_PERMISSIONS: [Permission; 8] = [
    Permission::Camera,
    Permission::Microphone,
    Permission::Photos,
    Permission::Location,
    Permission::Notifications,
    Permission::Contacts,
    Permission::Calendar,
    Permission::Bluetooth,
];

/// Emits only the permission frameworks and native cases reachable from the module.
/// Dynamic permission values conservatively retain every platform case.
pub(crate) fn render(
    out: &mut String,
    include_request: bool,
    used: &HashSet<Permission>,
    dynamic: bool,
) {
    let permissions = selected_permissions(used, dynamic);
    out.push_str("\nprivate enum NexaPermission {\n");
    for permission in &permissions {
        out.push_str(&format!("    case {}\n", case_name(*permission)));
    }
    out.push_str(
        r#"}

private enum NexaPermissionStatus {
    case granted
    case denied
    case restricted
    case notDetermined
}

"#,
    );
    if include_request && permissions.contains(&Permission::Location) {
        out.push_str(location_requester());
    }
    if include_request && permissions.contains(&Permission::Bluetooth) {
        out.push_str(bluetooth_requester());
    }
    out.push_str(
        r#"private enum NexaPermissions {
    static func status(_ permission: NexaPermission) async -> NexaPermissionStatus {
        switch permission {
"#,
    );
    for permission in &permissions {
        out.push_str(status_case(*permission));
    }
    out.push_str("        }\n    }\n\n");
    if include_request {
        out.push_str(
            r#"    static func request(_ permission: NexaPermission) async -> NexaPermissionStatus {
        switch permission {
"#,
        );
        for permission in &permissions {
            out.push_str(request_case(*permission));
        }
        out.push_str("        }\n    }\n");
    }
    out.push_str("}\n");
}

fn selected_permissions(used: &HashSet<Permission>, dynamic: bool) -> Vec<Permission> {
    ALL_PERMISSIONS
        .into_iter()
        .filter(|permission| dynamic || used.contains(permission))
        .collect()
}

fn case_name(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => "Camera",
        Permission::Microphone => "Microphone",
        Permission::Photos => "Photos",
        Permission::Location => "Location",
        Permission::Notifications => "Notifications",
        Permission::Contacts => "Contacts",
        Permission::Calendar => "Calendar",
        Permission::Bluetooth => "Bluetooth",
    }
}

fn status_case(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => {
            r#"        case .Camera:
            switch AVCaptureDevice.authorizationStatus(for: .video) {
            case .authorized: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
        Permission::Microphone => {
            r#"        case .Microphone:
            switch AVAudioSession.sharedInstance().recordPermission {
            case .granted: return .granted
            case .denied: return .denied
            case .undetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
        Permission::Photos => {
            r#"        case .Photos:
            switch PHPhotoLibrary.authorizationStatus(for: .readWrite) {
            case .authorized, .limited: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
        Permission::Location => {
            r#"        case .Location:
            switch CLLocationManager.authorizationStatus() {
            case .authorizedAlways, .authorizedWhenInUse: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
        Permission::Notifications => {
            r#"        case .Notifications:
            let settings = await UNUserNotificationCenter.current().notificationSettings()
            switch settings.authorizationStatus {
            case .authorized, .provisional, .ephemeral: return .granted
            case .denied: return .denied
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
        Permission::Contacts => {
            r#"        case .Contacts:
            switch CNContactStore.authorizationStatus(for: .contacts) {
            case .authorized: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
        Permission::Calendar => {
            r#"        case .Calendar:
            switch EKEventStore.authorizationStatus(for: .event) {
            case .authorized: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
        Permission::Bluetooth => {
            r#"        case .Bluetooth:
            switch CBManager.authorization {
            case .allowedAlways: return .granted
            case .denied: return .denied
            case .restricted: return .restricted
            case .notDetermined: return .notDetermined
            @unknown default: return .denied
            }
"#
        }
    }
}

fn request_case(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => {
            r#"        case .Camera:
            return await AVCaptureDevice.requestAccess(for: .video) ? .granted : .denied
"#
        }
        Permission::Microphone => {
            r#"        case .Microphone:
            let granted = await withCheckedContinuation { continuation in
                AVAudioSession.sharedInstance().requestRecordPermission { granted in
                    continuation.resume(returning: granted)
                }
            }
            return granted ? .granted : .denied
"#
        }
        Permission::Photos => {
            r#"        case .Photos:
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
"#
        }
        Permission::Location => {
            r#"        case .Location:
            return await NexaLocationRequester().request()
"#
        }
        Permission::Notifications => {
            r#"        case .Notifications:
            let granted = (try? await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound])) ?? false
            return granted ? .granted : .denied
"#
        }
        Permission::Contacts => {
            r#"        case .Contacts:
            let granted = await withCheckedContinuation { continuation in
                CNContactStore().requestAccess(for: .contacts) { granted, _ in
                    continuation.resume(returning: granted)
                }
            }
            return granted ? .granted : .denied
"#
        }
        Permission::Calendar => {
            r#"        case .Calendar:
            let granted = await withCheckedContinuation { continuation in
                EKEventStore().requestAccess(to: .event) { granted, _ in
                    continuation.resume(returning: granted)
                }
            }
            return granted ? .granted : .denied
"#
        }
        Permission::Bluetooth => {
            r#"        case .Bluetooth:
            return await NexaBluetoothRequester().request()
"#
        }
    }
}

fn location_requester() -> &'static str {
    r#"private final class NexaLocationRequester: NSObject, CLLocationManagerDelegate {
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

"#
}

fn bluetooth_requester() -> &'static str {
    r#"private final class NexaBluetoothRequester: NSObject, CBCentralManagerDelegate {
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

"#
}
