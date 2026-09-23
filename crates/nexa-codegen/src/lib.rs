use nexa_ir::Module;

pub mod names {
    /// Keeps generated declaration names away from platform keywords and APIs.
    pub fn screen_name(name: &str) -> String {
        format!("Nexa{}Screen", pascal_name(name, "App"))
    }

    /// Gives source-defined components stable names that cannot shadow native APIs.
    pub fn component_name(name: &str) -> String {
        format!("Nexa{name}Component")
    }

    fn pascal_name(name: &str, empty_name: &str) -> String {
        let mut pascal = String::new();
        let mut uppercase = true;
        for character in name.chars() {
            if character == '_' {
                uppercase = true;
            } else if uppercase {
                pascal.extend(character.to_uppercase());
                uppercase = false;
            } else {
                pascal.push(character);
            }
        }
        if pascal.is_empty() {
            pascal.push_str(empty_name);
        }
        pascal
    }

    pub fn state_name(name: &str) -> String {
        format!("nexa_{name}")
    }

    /// Gives source-defined pure functions stable top-level native names.
    pub fn function_name(name: &str) -> String {
        format!("nexa_fn_{name}")
    }

    /// Gives source-defined struct fields a stable name that cannot shadow a native member.
    pub fn struct_field_name(name: &str) -> String {
        format!("nexa_field_{name}")
    }

    /// Gives source-defined enum declarations stable native type names.
    pub fn enum_name(name: &str) -> String {
        format!("Nexa{}", pascal_name(name, "Enum"))
    }

    /// Gives source-defined value structs stable native type names.
    pub fn struct_name(name: &str) -> String {
        format!("Nexa{}", pascal_name(name, "Struct"))
    }

    pub fn navigation_case_name(screen: nexa_ir::ScreenId) -> String {
        format!("screen{}", screen.0)
    }

    pub fn navigation_route_name(screen: nexa_ir::ScreenId) -> String {
        format!("nexa_screen_{}", screen.0)
    }
}

pub mod plugin;

/// Supported compilation target platforms for Nexa code generation and scaffolding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TargetPlatform {
    Ios,
    Android,
    MacOs,
    Windows,
    Web,
}

impl TargetPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Web => "web",
        }
    }
}

/// Generated source and asset bundle produced by a target backend.
#[derive(Clone, Debug, Default)]
pub struct TargetOutput {
    /// Primary compiled source code (e.g. Swift view file or Kotlin Compose file).
    pub primary_source: String,
    /// Destination relative file path for the primary source.
    pub source_filename: String,
    /// Additional auxiliary source files generated for the target (path, content).
    pub auxiliary_files: Vec<(String, String)>,
}

/// Converts typed, platform-independent IR into a native source file.
pub trait Backend {
    fn name(&self) -> &'static str;
    fn file_extension(&self) -> &'static str;
    fn generate(&self, module: &Module) -> String;
}

/// Extended backend target contract defining full platform generation capabilities.
pub trait BackendTarget: Backend {
    fn platform(&self) -> TargetPlatform;
    fn generate_target(&self, module: &Module) -> TargetOutput {
        TargetOutput {
            primary_source: self.generate(module),
            source_filename: format!("App.{}", self.file_extension()),
            auxiliary_files: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::names;

    #[test]
    fn declaration_names_are_stable_and_avoid_platform_collisions() {
        assert_eq!(
            names::screen_name("account_settings"),
            "NexaAccountSettingsScreen"
        );
        assert_eq!(names::enum_name("http_status"), "NexaHttpStatus");
        assert_eq!(names::struct_name("player_options"), "NexaPlayerOptions");
        assert_eq!(names::component_name("Text"), "NexaTextComponent");
        assert_eq!(names::function_name("Button"), "nexa_fn_Button");
        assert_eq!(names::struct_field_name("id"), "nexa_field_id");
    }

    #[test]
    fn empty_or_separator_only_type_names_use_a_valid_fallback() {
        assert_eq!(names::enum_name(""), "NexaEnum");
        assert_eq!(names::struct_name("___"), "NexaStruct");
        assert_eq!(names::screen_name("___"), "NexaAppScreen");
    }
}
