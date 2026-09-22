use std::{fs, path::Path};

use nexa_ir::Permission;

#[derive(Clone, Debug, Default)]
pub(super) struct ProjectConfig {
    permissions: Vec<(Permission, String)>,
}

impl ProjectConfig {
    pub(super) fn parse_file(path: &Path) -> Result<Self, String> {
        let source = fs::read_to_string(path)
            .map_err(|error| format!("cannot read config {}: {error}", path.display()))?;
        let config = nexa_syntax::parse_config(&source)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let mut permissions = Vec::with_capacity(config.permissions.len());
        for declaration in config.permissions {
            let permission = parse_permission(&declaration.name).ok_or_else(|| {
                format!(
                    "{}:{}:{}: unknown permission `{}`",
                    path.display(),
                    declaration.span.line,
                    declaration.span.column,
                    declaration.name
                )
            })?;
            if permissions
                .iter()
                .any(|(existing, _): &(Permission, String)| *existing == permission)
            {
                return Err(format!(
                    "{}:{}:{}: permission `{}` is declared more than once",
                    path.display(),
                    declaration.span.line,
                    declaration.span.column,
                    declaration.name
                ));
            }
            permissions.push((permission, declaration.message));
        }
        Ok(Self { permissions })
    }

    pub(super) fn from_permissions(permissions: &[Permission]) -> Self {
        Self {
            permissions: permissions
                .iter()
                .copied()
                .map(|permission| (permission, default_message(permission).to_owned()))
                .collect(),
        }
    }

    pub(super) fn permissions(&self) -> impl Iterator<Item = &(Permission, String)> {
        self.permissions.iter()
    }

    pub(super) fn render(&self) -> String {
        let mut output = String::from("config {\n    permissions {\n");
        for (index, (permission, message)) in self.permissions.iter().enumerate() {
            output.push_str("        ");
            output.push_str(permission_name(*permission));
            output.push_str(": ");
            output.push_str(&nexa_config_string(message));
            if index + 1 < self.permissions.len() {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("    }\n}\n");
        output
    }
}

pub(super) fn parse_permission(name: &str) -> Option<Permission> {
    match name.to_ascii_lowercase().as_str() {
        "camera" => Some(Permission::Camera),
        "microphone" | "microphoneaudio" => Some(Permission::Microphone),
        "photos" | "media" => Some(Permission::Photos),
        "location" => Some(Permission::Location),
        "notifications" => Some(Permission::Notifications),
        "contacts" => Some(Permission::Contacts),
        "calendar" => Some(Permission::Calendar),
        "bluetooth" => Some(Permission::Bluetooth),
        _ => None,
    }
}

fn permission_name(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => "camera",
        Permission::Microphone => "microphone",
        Permission::Photos => "photos",
        Permission::Location => "location",
        Permission::Notifications => "notifications",
        Permission::Contacts => "contacts",
        Permission::Calendar => "calendar",
        Permission::Bluetooth => "bluetooth",
    }
}

fn default_message(permission: Permission) -> &'static str {
    match permission {
        Permission::Camera => "This app uses the camera to capture photos.",
        Permission::Microphone => "This app uses the microphone to record audio.",
        Permission::Photos => "This app uses your photos so you can choose images.",
        Permission::Location => "This app uses your location to provide location-based features.",
        Permission::Notifications => "This app sends notifications about important updates.",
        Permission::Contacts => "This app uses your contacts when you choose to share with them.",
        Permission::Calendar => "This app uses your calendar to show and manage events.",
        Permission::Bluetooth => "This app uses Bluetooth to connect to nearby devices.",
    }
}

fn nexa_config_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character => output.push(character),
        }
    }
    output.push('"');
    output
}
