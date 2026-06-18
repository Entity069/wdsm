use std::fmt;

#[derive(Debug)]
pub enum WdsmError {
    MissingTypeAnnotation {
        file: String,
        line: u64,
        function: String,
        param: String,
    },

    UnsupportedType {
        ty: String,
        suggestion: Option<String>,
    },

    WitValidation(String),
    MissingToolchain {
        tool: String,
        install_cmd: String,
    },

    CircularReference {
        chain: String,
    },

    DuplicateType {
        name: String,
        first: String,
        second: String,
    },

    DuplicateField {
        record: String,
        field: String,
    },

    DuplicateParam {
        function: String,
        param: String,
    },

    DuplicateCase {
        type_name: String,
        case: String,
    },

    UnresolvedType {
        name: String,
        ref_in: String,
        src_file: String,
        src_line: u64,
    },

    InvalidWitName {
        name: String,
        reason: String,
    },

    ParseError(String),

    FunctionNotFound {
        function: String,
        file: String,
    },

    IoError(String),
}

impl fmt::Display for WdsmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WdsmError::MissingTypeAnnotation { file, line, function, param } => {
                write!(
                    f,
                    "{}:{}: missing type annotation on parameter `{}` in function `{}`",
                    file, line, param, function
                )
            }
            WdsmError::UnsupportedType { ty, suggestion } => {
                write!(f, "unsupported type `{}`", ty)?;
                if let Some(s) = suggestion {
                    write!(f, " — {}", s)?;
                }
                Ok(())
            }
            WdsmError::WitValidation(msg) => {
                write!(f, "WIT validation failed: {}", msg)
            }
            WdsmError::MissingToolchain { tool, install_cmd } => {
                write!(
                    f,
                    "toolchain `{}` not found — install with: {}",
                    tool, install_cmd
                )
            }
            WdsmError::CircularReference { chain } => {
                write!(f, "circular type reference detected: {}", chain)
            }
            WdsmError::DuplicateType { name, first, second } => {
                write!(
                    f,
                    "duplicate type name `{}` — first defined at {}, redefined at {}",
                    name, first, second
                )
            }
            WdsmError::DuplicateField { record, field } => {
                write!(
                    f,
                    "duplicate field `{}` in record `{}`",
                    field, record
                )
            }
            WdsmError::DuplicateParam { function, param } => {
                write!(
                    f,
                    "duplicate parameter `{}` in function `{}`",
                    param, function
                )
            }
            WdsmError::DuplicateCase { type_name, case } => {
                write!(
                    f,
                    "duplicate case `{}` in type `{}`",
                    case, type_name
                )
            }
            WdsmError::UnresolvedType { name, ref_in, src_file, src_line } => {
                write!(
                    f,
                    "{}:{}: unresolved type `{}` referenced in `{}`",
                    src_file, src_line, name, ref_in
                )
            }
            WdsmError::InvalidWitName { name, reason } => {
                write!(f, "invalid WIT name `{}`: {}", name, reason)
            }
            WdsmError::ParseError(msg) => {
                write!(f, "parse error: {}", msg)
            }
            WdsmError::FunctionNotFound { function, file } => {
                write!(f, "function `{}` not found in {}", function, file)
            }
            WdsmError::IoError(msg) => {
                write!(f, "I/O error: {}", msg)
            }
        }
    }
}

impl std::error::Error for WdsmError {}

impl From<std::io::Error> for WdsmError {
    fn from(e: std::io::Error) -> Self {
        WdsmError::IoError(e.to_string())
    }
}

pub const WIT_KEYWORDS: &[&str] = &[
    "use", "type", "resource", "func", "record", "enum", "flags", "variant",
    "static", "interface", "world", "import", "export", "package", "include",
    "constructor", "result", "option", "list", "tuple", "borrow", "own",
    "bool", "u8", "u16", "u32", "u64", "s8", "s16", "s32", "s64",
    "f32", "f64", "char", "string",
];


pub fn validate_wit_name(name: &str) -> Result<(), WdsmError> {
    if name.is_empty() {
        return Err(WdsmError::InvalidWitName {
            name: name.to_string(),
            reason: "name cannot be empty".to_string(),
        });
    }

    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(WdsmError::InvalidWitName {
            name: name.to_string(),
            reason: "must start with a lowercase ASCII letter".to_string(),
        });
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(WdsmError::InvalidWitName {
            name: name.to_string(),
            reason: "must only contain lowercase ASCII letters, digits, and hyphens".to_string(),
        });
    }

    if name.ends_with('-') {
        return Err(WdsmError::InvalidWitName {
            name: name.to_string(),
            reason: "must not end with a hyphen".to_string(),
        });
    }

    if name.contains("--") {
        return Err(WdsmError::InvalidWitName {
            name: name.to_string(),
            reason: "must not contain consecutive hyphens".to_string(),
        });
    }

    if WIT_KEYWORDS.contains(&name) {
        return Err(WdsmError::InvalidWitName {
            name: name.to_string(),
            reason: format!("`{}` is a reserved WIT keyword", name),
        });
    }

    Ok(())
}
