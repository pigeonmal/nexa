//! Versioned native plugin ABI contract.
//!
//! The contract is intentionally small and line-oriented so plugin authors can
//! review it without a generator-specific JSON dependency. It describes the
//! ownership rules required before a native C/C++/Rust binding may use a
//! zero-copy buffer boundary.

use std::{fs, path::Path};

pub const ABI_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbiContract {
    pub schema: u16,
    pub calling_convention: CallingConvention,
    pub strings: BufferOwnership,
    pub bytes: BufferOwnership,
    pub returned_buffers: ReturnOwnership,
    pub lifetime: BufferLifetime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallingConvention {
    C,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferOwnership {
    BorrowedReadOnly,
    Owned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnOwnership {
    CallerOwned,
    CalleeOwned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferLifetime {
    Call,
    ExplicitRelease,
}

impl Default for AbiContract {
    fn default() -> Self {
        Self {
            schema: ABI_SCHEMA_VERSION,
            calling_convention: CallingConvention::C,
            strings: BufferOwnership::BorrowedReadOnly,
            bytes: BufferOwnership::BorrowedReadOnly,
            returned_buffers: ReturnOwnership::CallerOwned,
            lifetime: BufferLifetime::Call,
        }
    }
}

impl AbiContract {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != ABI_SCHEMA_VERSION {
            return Err(format!(
                "unsupported native ABI schema {}; expected {}",
                self.schema, ABI_SCHEMA_VERSION
            ));
        }
        if self.calling_convention != CallingConvention::C {
            return Err("only the stable C calling convention is supported".to_owned());
        }
        if self.returned_buffers == ReturnOwnership::CalleeOwned
            && self.lifetime != BufferLifetime::ExplicitRelease
        {
            return Err(
                "callee-owned returned buffers require an explicit release lifetime".to_owned(),
            );
        }
        Ok(())
    }

    pub fn zero_copy_bytes(&self) -> bool {
        self.bytes == BufferOwnership::BorrowedReadOnly && self.lifetime == BufferLifetime::Call
    }

    pub fn render(&self) -> String {
        format!(
            "# Nexa native ABI contract\nschema {}\ncalling_convention {}\nstrings {}\nbytes {}\nreturned_buffers {}\nlifetime {}\n",
            self.schema,
            self.calling_convention.as_str(),
            self.strings.as_str(),
            self.bytes.as_str(),
            self.returned_buffers.as_str(),
            self.lifetime.as_str(),
        )
    }
}

pub fn parse_file(path: &Path) -> Result<AbiContract, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse(&source).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn parse(source: &str) -> Result<AbiContract, String> {
    let mut contract = AbiContract::default();
    let mut seen = std::collections::HashSet::new();
    for (line_number, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let key = parts
            .next()
            .ok_or_else(|| format!("ABI line {} is empty", line_number + 1))?;
        let value = parts
            .next()
            .ok_or_else(|| format!("ABI line {} must contain a key and value", line_number + 1))?;
        if parts.next().is_some() {
            return Err(format!(
                "ABI line {} has unexpected trailing values",
                line_number + 1
            ));
        }
        if !seen.insert(key) {
            return Err(format!("ABI field `{key}` is declared more than once"));
        }
        match key {
            "schema" => {
                contract.schema = value.parse().map_err(|_| {
                    format!("ABI schema must be an integer on line {}", line_number + 1)
                })?;
            }
            "calling_convention" => {
                contract.calling_convention = match value {
                    "c" => CallingConvention::C,
                    _ => return Err(format!("unsupported ABI calling convention `{value}`")),
                };
            }
            "strings" => contract.strings = parse_buffer_ownership(value)?,
            "bytes" => contract.bytes = parse_buffer_ownership(value)?,
            "returned_buffers" => {
                contract.returned_buffers = match value {
                    "caller_owned" => ReturnOwnership::CallerOwned,
                    "callee_owned" => ReturnOwnership::CalleeOwned,
                    _ => return Err(format!("unsupported returned buffer ownership `{value}`")),
                };
            }
            "lifetime" => {
                contract.lifetime = match value {
                    "call" => BufferLifetime::Call,
                    "explicit_release" => BufferLifetime::ExplicitRelease,
                    _ => return Err(format!("unsupported ABI buffer lifetime `{value}`")),
                };
            }
            _ => return Err(format!("unknown ABI field `{key}`")),
        }
    }
    for required in [
        "schema",
        "calling_convention",
        "strings",
        "bytes",
        "returned_buffers",
        "lifetime",
    ] {
        if !seen.contains(required) {
            return Err(format!("ABI contract is missing `{required}`"));
        }
    }
    contract.validate()?;
    Ok(contract)
}

fn parse_buffer_ownership(value: &str) -> Result<BufferOwnership, String> {
    match value {
        "borrowed_readonly" => Ok(BufferOwnership::BorrowedReadOnly),
        "owned" => Ok(BufferOwnership::Owned),
        _ => Err(format!("unsupported ABI buffer ownership `{value}`")),
    }
}

impl CallingConvention {
    fn as_str(self) -> &'static str {
        match self {
            Self::C => "c",
        }
    }
}

impl BufferOwnership {
    fn as_str(self) -> &'static str {
        match self {
            Self::BorrowedReadOnly => "borrowed_readonly",
            Self::Owned => "owned",
        }
    }
}

impl ReturnOwnership {
    fn as_str(self) -> &'static str {
        match self {
            Self::CallerOwned => "caller_owned",
            Self::CalleeOwned => "callee_owned",
        }
    }
}

impl BufferLifetime {
    fn as_str(self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::ExplicitRelease => "explicit_release",
        }
    }
}
