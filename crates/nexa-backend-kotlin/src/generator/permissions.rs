/// Emits direct Android permission status queries for the typed `Permissions` API.
pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
private enum class NexaPermission {
    Camera,
    Microphone,
    Photos,
    Location,
    Notifications,
    Contacts,
    Calendar,
    Bluetooth,
}

private enum class NexaPermissionStatus {
    granted,
    denied,
    restricted,
    notDetermined,
}

private object NexaPermissions {
    fun status(context: android.content.Context, permission: NexaPermission): NexaPermissionStatus {
        return when (permission) {
            NexaPermission.Camera -> statusFor(context, android.Manifest.permission.CAMERA)
            NexaPermission.Microphone -> statusFor(context, android.Manifest.permission.RECORD_AUDIO)
            NexaPermission.Photos -> if (android.os.Build.VERSION.SDK_INT >= 33) {
                statusFor(context, android.Manifest.permission.READ_MEDIA_IMAGES)
            } else {
                statusFor(context, android.Manifest.permission.READ_EXTERNAL_STORAGE)
            }
            NexaPermission.Location -> combine(
                statusFor(context, android.Manifest.permission.ACCESS_COARSE_LOCATION),
                statusFor(context, android.Manifest.permission.ACCESS_FINE_LOCATION),
            )
            NexaPermission.Notifications -> if (android.os.Build.VERSION.SDK_INT < 33) {
                NexaPermissionStatus.granted
            } else {
                statusFor(context, android.Manifest.permission.POST_NOTIFICATIONS)
            }
            NexaPermission.Contacts -> statusFor(context, android.Manifest.permission.READ_CONTACTS)
            NexaPermission.Calendar -> statusFor(context, android.Manifest.permission.READ_CALENDAR)
            NexaPermission.Bluetooth -> if (android.os.Build.VERSION.SDK_INT < 31) {
                NexaPermissionStatus.granted
            } else {
                statusFor(context, android.Manifest.permission.BLUETOOTH_SCAN)
            }
        }
    }

    suspend fun request(
        context: android.content.Context,
        permission: NexaPermission,
    ): NexaPermissionStatus {
        val permissions = when (permission) {
            NexaPermission.Camera -> arrayOf(android.Manifest.permission.CAMERA)
            NexaPermission.Microphone -> arrayOf(android.Manifest.permission.RECORD_AUDIO)
            NexaPermission.Photos -> if (android.os.Build.VERSION.SDK_INT >= 33) {
                arrayOf(android.Manifest.permission.READ_MEDIA_IMAGES)
            } else {
                arrayOf(android.Manifest.permission.READ_EXTERNAL_STORAGE)
            }
            NexaPermission.Location -> arrayOf(
                android.Manifest.permission.ACCESS_COARSE_LOCATION,
                android.Manifest.permission.ACCESS_FINE_LOCATION,
            )
            NexaPermission.Notifications -> if (android.os.Build.VERSION.SDK_INT >= 33) {
                arrayOf(android.Manifest.permission.POST_NOTIFICATIONS)
            } else {
                emptyArray()
            }
            NexaPermission.Contacts -> arrayOf(
                android.Manifest.permission.READ_CONTACTS,
                android.Manifest.permission.WRITE_CONTACTS,
            )
            NexaPermission.Calendar -> arrayOf(
                android.Manifest.permission.READ_CALENDAR,
                android.Manifest.permission.WRITE_CALENDAR,
            )
            NexaPermission.Bluetooth -> if (android.os.Build.VERSION.SDK_INT >= 31) {
                arrayOf(
                    android.Manifest.permission.BLUETOOTH_SCAN,
                    android.Manifest.permission.BLUETOOTH_CONNECT,
                )
            } else {
                emptyArray()
            }
        }
        NexaRuntime.requestPermissions(permissions)
        return status(context, permission)
    }

    private fun statusFor(context: android.content.Context, permission: String): NexaPermissionStatus {
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

    private fun combine(
        coarse: NexaPermissionStatus,
        fine: NexaPermissionStatus,
    ): NexaPermissionStatus = when {
        coarse == NexaPermissionStatus.granted || fine == NexaPermissionStatus.granted -> NexaPermissionStatus.granted
        coarse == NexaPermissionStatus.restricted || fine == NexaPermissionStatus.restricted -> NexaPermissionStatus.restricted
        coarse == NexaPermissionStatus.denied && fine == NexaPermissionStatus.denied -> NexaPermissionStatus.denied
        else -> NexaPermissionStatus.notDetermined
    }
}
"#,
    );
}
