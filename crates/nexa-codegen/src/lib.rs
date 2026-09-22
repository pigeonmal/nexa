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

    /// Gives source-defined enum declarations stable native type names.
    pub fn enum_name(name: &str) -> String {
        format!("Nexa{}", pascal_name(name, "Enum"))
    }

    pub fn navigation_case_name(screen: nexa_ir::ScreenId) -> String {
        format!("screen{}", screen.0)
    }

    pub fn navigation_route_name(screen: nexa_ir::ScreenId) -> String {
        format!("nexa_screen_{}", screen.0)
    }
}

/// Converts typed, platform-independent IR into a native source file.
pub trait Backend {
    fn name(&self) -> &'static str;
    fn file_extension(&self) -> &'static str;
    fn generate(&self, module: &Module) -> String;
}
