use nexa_ir::Module;

pub mod names {
    /// Keeps generated declaration names away from platform keywords and APIs.
    pub fn screen_name(name: &str) -> String {
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
            pascal.push_str("App");
        }
        format!("Nexa{pascal}Screen")
    }

    pub fn state_name(name: &str) -> String {
        format!("nexa_{name}")
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
