use std::fmt;

#[derive(Debug, Clone)]
pub struct WitIR {
    pub package: String,
    pub world_name: String,
    pub types: Vec<TypeDef>,
    pub functions: Vec<FunctionDef>,
    pub imports: Vec<ImportDef>,
}

impl WitIR {
    pub fn find_record(&self, name: &str) -> Option<&RecordDef> {
        self.types.iter().find_map(|td| {
            if let TypeDef::Record(r) = td {
                if r.name == name {
                    return Some(r);
                }
            }
            None
        })
    }

    pub fn find_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.iter().find(|td| td.name() == name)
    }
    pub fn type_names(&self) -> Vec<&str> {
        self.types.iter().map(|td| td.name()).collect()
    }
}

// ---------------------------------------------------------------------------
// Type definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum TypeDef {
    Record(RecordDef),
    Variant(VariantDef),
    Enum(EnumDef),
    Flags(FlagsDef),
    Alias(AliasDef),
}

impl TypeDef {
    pub fn name(&self) -> &str {
        match self {
            TypeDef::Record(r) => &r.name,
            TypeDef::Variant(v) => &v.name,
            TypeDef::Enum(e) => &e.name,
            TypeDef::Flags(f) => &f.name,
            TypeDef::Alias(a) => &a.name,
        }
    }

    pub fn wit_name(&self) -> &str {
        match self {
            TypeDef::Record(r) => &r.wit_name,
            TypeDef::Variant(v) => &v.wit_name,
            TypeDef::Enum(e) => &e.wit_name,
            TypeDef::Flags(f) => &f.wit_name,
            TypeDef::Alias(a) => &a.wit_name,
        }
    }

    pub fn dependencies(&self) -> Vec<&str> {
        match self {
            TypeDef::Record(r) => r
                .fields
                .iter()
                .flat_map(|f| f.ty.named_refs())
                .collect(),
            TypeDef::Variant(v) => v
                .cases
                .iter()
                .flat_map(|c| {
                    c.payload
                        .as_ref()
                        .map(|t| t.named_refs())
                        .unwrap_or_default()
                })
                .collect(),
            TypeDef::Alias(a) => a.target.named_refs(),
            TypeDef::Enum(_) | TypeDef::Flags(_) => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordDef {
    pub name: String,
    pub wit_name: String,
    pub fields: Vec<FieldDef>,
    pub source: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub wit_name: String,
    pub ty: IRType,
    pub optional: bool,
}
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub wit_name: String,
    pub cases: Vec<VariantCase>,
    pub source: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct VariantCase {
    pub name: String,
    pub payload: Option<IRType>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub wit_name: String,
    pub cases: Vec<String>,
    pub source: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct FlagsDef {
    pub name: String,
    pub wit_name: String,
    pub flags: Vec<String>,
    pub source: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct AliasDef {
    pub name: String,
    pub wit_name: String,
    pub target: IRType,
    pub source: SourceSpan,
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub wit_name: String,
    pub params: Vec<ParamDef>,
    pub returns: ReturnType,
    pub docs: Option<String>,
    pub source: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ParamDef {
    pub name: String,
    pub wit_name: String,
    pub ty: IRType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReturnType {
    None,
    Type(IRType),
}

// ---------------------------------------------------------------------------
// IR Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum IRType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    F32,
    F64,
    Char,
    String,

    List(Box<IRType>),
    Option(Box<IRType>),
    Result {
        ok: std::option::Option<Box<IRType>>,
        err: std::option::Option<Box<IRType>>,
    },
    Tuple(Vec<IRType>),
    Named(std::string::String),
}

impl IRType {
    pub fn to_wit_str(&self) -> String {
        match self {
            IRType::Bool => "bool".to_string(),
            IRType::U8 => "u8".to_string(),
            IRType::U16 => "u16".to_string(),
            IRType::U32 => "u32".to_string(),
            IRType::U64 => "u64".to_string(),
            IRType::S8 => "s8".to_string(),
            IRType::S16 => "s16".to_string(),
            IRType::S32 => "s32".to_string(),
            IRType::S64 => "s64".to_string(),
            IRType::F32 => "f32".to_string(),
            IRType::F64 => "f64".to_string(),
            IRType::Char => "char".to_string(),
            IRType::String => "string".to_string(),
            IRType::List(inner) => format!("list<{}>", inner.to_wit_str()),
            IRType::Option(inner) => format!("option<{}>", inner.to_wit_str()),
            IRType::Result { ok, err } => {
                let ok_str = ok
                    .as_ref()
                    .map(|t| t.to_wit_str())
                    .unwrap_or_else(|| "_".to_string());
                let err_str = err
                    .as_ref()
                    .map(|t| t.to_wit_str())
                    .unwrap_or_else(|| "_".to_string());
                format!("result<{}, {}>", ok_str, err_str)
            }
            IRType::Tuple(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_wit_str()).collect();
                format!("tuple<{}>", type_strs.join(", "))
            }
            IRType::Named(name) => to_kebab_case(name),
        }
    }

    pub fn named_refs(&self) -> Vec<&str> {
        match self {
            IRType::Named(name) => vec![name.as_str()],
            IRType::List(inner) | IRType::Option(inner) => inner.named_refs(),
            IRType::Result { ok, err } => {
                let mut refs = Vec::new();
                if let Some(ok) = ok {
                    refs.extend(ok.named_refs());
                }
                if let Some(err) = err {
                    refs.extend(err.named_refs());
                }
                refs
            }
            IRType::Tuple(types) => types.iter().flat_map(|t| t.named_refs()).collect(),
            _ => vec![],
        }
    }
}

impl fmt::Display for IRType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_wit_str())
    }
}

// ---------------------------------------------------------------------------
// Imports (reserved for future WASI import tracking)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ImportDef {
    pub interface_name: String,
    pub functions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Source tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SourceSpan {
    pub file: String,
    pub line: u64,
    pub confidence: Confidence,
}

impl Default for SourceSpan {
    fn default() -> Self {
        Self {
            file: "<unknown>".to_string(),
            line: 0,
            confidence: Confidence::Explicit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    Explicit,
    Inferred,
    Fallback,
}




pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);

    for c in s.chars() {
        if c.is_uppercase() {
            if !result.is_empty() {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}
