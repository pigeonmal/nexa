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
        when (permission) {
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

    private fun statusFor(context: android.content.Context, permission: String): NexaPermissionStatus {
        if (context.checkSelfPermission(permission) == android.content.pm.PackageManager.PERMISSION_GRANTED) {
            return NexaPermissionStatus.granted
        }
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            val flags = context.packageManager.getPermissionFlags(
                permission,
                context.packageName,
                android.os.Process.myUserHandle(),
            )
            val hasUserDecision = (flags and (
                android.content.pm.PackageManager.FLAG_PERMISSION_USER_SET or
                    android.content.pm.PackageManager.FLAG_PERMISSION_USER_FIXED
            )) != 0
            if (hasUserDecision) return NexaPermissionStatus.denied
        }
        return NexaPermissionStatus.notDetermined
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
