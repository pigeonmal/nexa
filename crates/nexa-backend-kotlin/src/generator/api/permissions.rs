use std::collections::HashSet;

use nexa_ir::Permission;

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_permission_request,
        "androidx.activity.compose.rememberLauncherForActivityResult",
    );
    imports.add(
        features.uses_permission_request,
        "androidx.activity.result.contract.ActivityResultContracts",
    );
    imports.add(features.uses_permissions, "android.content.Context");
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

/// Emits only the permission cases reachable from the module.
/// Dynamic permission values conservatively retain every platform case.
pub(crate) fn render(
    out: &mut String,
    include_request: bool,
    used: &HashSet<Permission>,
    dynamic: bool,
    expose_to_dev_runtime: bool,
) {
    let permissions = selected_permissions(used, dynamic);
    let visibility = if expose_to_dev_runtime {
        "public"
    } else {
        "private"
    };
    out.push_str(&format!("\n{visibility} enum class NexaPermission {{\n"));
    for permission in &permissions {
        out.push_str(&format!("    {},\n", case_name(*permission)));
    }
    out.push_str("}\n\n");
    out.push_str(visibility);
    out.push_str(
        r#" enum class NexaPermissionStatus {
    granted,
    denied,
    restricted,
    notDetermined,
}

"#,
    );
    out.push_str(visibility);
    out.push_str(
        r#" object NexaPermissions {
    fun status(context: android.content.Context, permission: NexaPermission): NexaPermissionStatus {
        return when (permission) {
"#,
    );
    for permission in &permissions {
        out.push_str(status_case(*permission));
    }
    out.push_str("        }\n    }\n\n");
    if include_request {
        out.push_str(
            r#"    suspend fun request(
        context: android.content.Context,
        permission: NexaPermission,
    ): NexaPermissionStatus {
        val permissions = when (permission) {
"#,
        );
        for permission in &permissions {
            out.push_str(request_case(*permission));
        }
        out.push_str(
            r#"        }
        NexaRuntime.requestPermissions(permissions)
        return status(context, permission)
    }

"#,
        );
    }
    out.push_str(
        r#"    private fun statusFor(context: android.content.Context, permission: String): NexaPermissionStatus {
        if (context.checkSelfPermission(permission) == android.content.pm.PackageManager.PERMISSION_GRANTED) {
            return NexaPermissionStatus.granted
        }
        val operation = android.app.AppOpsManager.permissionToOp(permission) ?: return NexaPermissionStatus.notDetermined
        val appOps = context.getSystemService(android.content.Context.APP_OPS_SERVICE) as? android.app.AppOpsManager
            ?: return NexaPermissionStatus.notDetermined
        return when (appOps.checkOpNoThrow(operation, android.os.Process.myUid(), context.packageName)) {
            android.app.AppOpsManager.MODE_IGNORED,
            android.app.AppOpsManager.MODE_ERRORED -> NexaPermissionStatus.denied
            else -> NexaPermissionStatus.notDetermined
        }
    }

"#,
    );
    if permissions.contains(&Permission::Location) {
        out.push_str(
            r#"    private fun combine(
        coarse: NexaPermissionStatus,
        fine: NexaPermissionStatus,
    ): NexaPermissionStatus = when {
        coarse == NexaPermissionStatus.granted || fine == NexaPermissionStatus.granted -> NexaPermissionStatus.granted
        coarse == NexaPermissionStatus.restricted || fine == NexaPermissionStatus.restricted -> NexaPermissionStatus.restricted
        coarse == NexaPermissionStatus.denied && fine == NexaPermissionStatus.denied -> NexaPermissionStatus.denied
        else -> NexaPermissionStatus.notDetermined
    }

"#,
        );
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
            "            NexaPermission.Camera -> statusFor(context, android.Manifest.permission.CAMERA)\n"
        }
        Permission::Microphone => {
            "            NexaPermission.Microphone -> statusFor(context, android.Manifest.permission.RECORD_AUDIO)\n"
        }
        Permission::Photos => {
            r#"            NexaPermission.Photos -> if (android.os.Build.VERSION.SDK_INT >= 33) {
                statusFor(context, android.Manifest.permission.READ_MEDIA_IMAGES)
            } else {
                statusFor(context, android.Manifest.permission.READ_EXTERNAL_STORAGE)
            }
"#
        }
        Permission::Location => {
            r#"            NexaPermission.Location -> combine(
                statusFor(context, android.Manifest.permission.ACCESS_COARSE_LOCATION),
                statusFor(context, android.Manifest.permission.ACCESS_FINE_LOCATION),
            )
"#
        }
        Permission::Notifications => {
            r#"            NexaPermission.Notifications -> if (android.os.Build.VERSION.SDK_INT < 33) {
                NexaPermissionStatus.granted
            } else {
                statusFor(context, android.Manifest.permission.POST_NOTIFICATIONS)
            }
"#
        }
        Permission::Contacts => {
            "            NexaPermission.Contacts -> statusFor(context, android.Manifest.permission.READ_CONTACTS)\n"
        }
        Permission::Calendar => {
            "            NexaPermission.Calendar -> statusFor(context, android.Manifest.permission.READ_CALENDAR)\n"
        }
        Permission::Bluetooth => {
            r#"            NexaPermission.Bluetooth -> if (android.os.Build.VERSION.SDK_INT < 31) {
                NexaPermissionStatus.granted
            } else {
                statusFor(context, android.Manifest.permission.BLUETOOTH_SCAN)
            }
"#
        }
    }
}

fn request_case(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => {
            "            NexaPermission.Camera -> arrayOf(android.Manifest.permission.CAMERA)\n"
        }
        Permission::Microphone => {
            "            NexaPermission.Microphone -> arrayOf(android.Manifest.permission.RECORD_AUDIO)\n"
        }
        Permission::Photos => {
            r#"            NexaPermission.Photos -> if (android.os.Build.VERSION.SDK_INT >= 33) {
                arrayOf(android.Manifest.permission.READ_MEDIA_IMAGES)
            } else {
                arrayOf(android.Manifest.permission.READ_EXTERNAL_STORAGE)
            }
"#
        }
        Permission::Location => {
            r#"            NexaPermission.Location -> arrayOf(
                android.Manifest.permission.ACCESS_COARSE_LOCATION,
                android.Manifest.permission.ACCESS_FINE_LOCATION,
            )
"#
        }
        Permission::Notifications => {
            r#"            NexaPermission.Notifications -> if (android.os.Build.VERSION.SDK_INT >= 33) {
                arrayOf(android.Manifest.permission.POST_NOTIFICATIONS)
            } else {
                emptyArray()
            }
"#
        }
        Permission::Contacts => {
            r#"            NexaPermission.Contacts -> arrayOf(
                android.Manifest.permission.READ_CONTACTS,
                android.Manifest.permission.WRITE_CONTACTS,
            )
"#
        }
        Permission::Calendar => {
            r#"            NexaPermission.Calendar -> arrayOf(
                android.Manifest.permission.READ_CALENDAR,
                android.Manifest.permission.WRITE_CALENDAR,
            )
"#
        }
        Permission::Bluetooth => {
            r#"            NexaPermission.Bluetooth -> if (android.os.Build.VERSION.SDK_INT >= 31) {
                arrayOf(
                    android.Manifest.permission.BLUETOOTH_SCAN,
                    android.Manifest.permission.BLUETOOTH_CONNECT,
                )
            } else {
                emptyArray()
            }
"#
        }
    }
}
