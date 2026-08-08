use crate::frontends::{FrontendConfig, LanguageFrontend};
use crate::ir::*;
use anyhow::{Context, Result};
use std::path::Path;

pub struct PythonFrontend;

impl LanguageFrontend for PythonFrontend {
    fn language(&self) -> &str {
        "python"
    }

    fn extract(&self, source: &Path, config: &FrontendConfig) -> Result<WitIR> {
        let content = std::fs::read_to_string(source)
            .with_context(|| format!("Failed to read Python file: {}", source.display()))?;

        let mut ir = WitIR {
            package: config.package_name.clone(),
            world_name: config.world_name.clone(),
            types: Vec::new(),
            functions: Vec::new(),
            imports: Vec::new(),
        };

        let file_str = source.to_string_lossy().to_string();

        // Parse type definitions: @dataclass, TypedDict, Enum subclasses
        parse_type_defs(&content, &file_str, &mut ir)?;

        // Parse top-level exported functions
        parse_functions(&content, &file_str, config, &mut ir)?;

        Ok(ir)
    }
}

// ---------------------------------------------------------------------------
// Type definition parsers
// ---------------------------------------------------------------------------

fn parse_type_defs(content: &str, file: &str, ir: &mut WitIR) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // @dataclass decorator – next class line is the record
        if trimmed == "@dataclass" {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().starts_with('@') {
                j += 1;
            }
            if j < lines.len() {
                if let Some(name) = parse_class_name(lines[j]) {
                    let fields = parse_annotated_fields(&lines, j + 1);
                    ir.types.push(TypeDef::Record(RecordDef {
                        wit_name: to_kebab_case(&name),
                        name,
                        fields,
                        source: SourceSpan {
                            file: file.to_string(),
                            line: j as u64 + 1,
                            confidence: Confidence::Explicit,
                        },
                    }));
                    i = j + 1;
                    continue;
                }
            }
        }

        // class Foo(TypedDict): or class Foo(TypedDict, total=False):
        if trimmed.starts_with("class ") && trimmed.contains("TypedDict") {
            if let Some(name) = parse_class_name(trimmed) {
                let fields = parse_annotated_fields(&lines, i + 1);
                ir.types.push(TypeDef::Record(RecordDef {
                    wit_name: to_kebab_case(&name),
                    name,
                    fields,
                    source: SourceSpan {
                        file: file.to_string(),
                        line: i as u64 + 1,
                        confidence: Confidence::Explicit,
                    },
                }));
            }
        }

        // class Foo(Enum): or class Foo(str, Enum): or class Foo(IntEnum):
        if trimmed.starts_with("class ")
            && (trimmed.contains("(Enum)")
                || trimmed.contains("(str, Enum)")
                || trimmed.contains("(IntEnum)")
                || trimmed.contains("(StrEnum)"))
        {
            if let Some(name) = parse_class_name(trimmed) {
                let cases = parse_enum_cases(&lines, i + 1);
                if !cases.is_empty() {
                    ir.types.push(TypeDef::Enum(EnumDef {
                        wit_name: to_kebab_case(&name),
                        name,
                        cases,
                        source: SourceSpan {
                            file: file.to_string(),
                            line: i as u64 + 1,
                            confidence: Confidence::Explicit,
                        },
                    }));
                }
            }
        }

        i += 1;
    }

    Ok(())
}

/// Extract class name from `class Foo(Base):` or `class Foo:`
fn parse_class_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("class ") {
        return None;
    }
    let rest = &line[6..];
    let name_end = rest
        .find(|c: char| c == '(' || c == ':')
        .unwrap_or(rest.len());
    let name = rest[..name_end].trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Parse `field: type` or `field: type = default` lines from an indented block
fn parse_annotated_fields(lines: &[&str], start: usize) -> Vec<FieldDef> {
    let mut fields = Vec::new();
    for line in lines.iter().skip(start) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // End of indented block: dedented non-empty line
        if !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        // Skip method definitions or class variables without annotation
        if trimmed.starts_with("def ") || trimmed.starts_with("class ") {
            break;
        }
        if let Some(colon_pos) = trimmed.find(':') {
            let name = trimmed[..colon_pos].trim().to_string();
            // Must be a valid identifier (no spaces, starts with letter/_)
            if name.contains(' ')
                || name.is_empty()
                || (!name.starts_with(|c: char| c.is_alphabetic() || c == '_'))
            {
                continue;
            }
            let rest = trimmed[colon_pos + 1..].trim();
            // Strip default value after `=`
            let ty_str = rest.split('=').next().unwrap_or(rest).trim();
            let (ty, optional) = parse_python_type(ty_str);
            if let Some(ty) = ty {
                fields.push(FieldDef {
                    wit_name: to_kebab_case(&name),
                    name,
                    ty,
                    optional,
                });
            }
        }
    }
    fields
}

/// Parse `CASE = "value"` enum members from an indented block
fn parse_enum_cases(lines: &[&str], start: usize) -> Vec<String> {
    let mut cases = Vec::new();
    for line in lines.iter().skip(start) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                cases.push(to_kebab_case(&name.to_lowercase()));
            }
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// Function parsers
// ---------------------------------------------------------------------------

fn parse_functions(
    content: &str,
    file: &str,
    config: &FrontendConfig,
    ir: &mut WitIR,
) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        // Match `def` functions (top-level or inside WitWorld class)
        if !trimmed.starts_with("def ") {
            i += 1;
            continue;
        }

        // Collect full signature (may span multiple lines until `:`)
        let mut sig = line.trim().to_string();
        let mut j = i + 1;
        while !sig_complete(&sig) && j < lines.len() {
            sig.push(' ');
            sig.push_str(lines[j].trim());
            j += 1;
        }

        // fn name from `def <name>(`
        let rest = &sig[4..]; // strip "def "
        let paren = match rest.find('(') {
            Some(p) => p,
            None => {
                i += 1;
                continue;
            }
        };
        let fn_name = rest[..paren].trim().to_string();

        // Skip private/dunder functions
        if fn_name.starts_with('_') {
            i += 1;
            continue;
        }

        // Filter by target list if provided
        if let Some(ref targets) = config.target_functions {
            if !targets.contains(&fn_name) {
                i += 1;
                continue;
            }
        }

        let (params_str, return_str) = split_signature(&sig).with_context(|| {
            format!("{}:{}: malformed function signature", file, i + 1)
        })?;

        let params = parse_params(&params_str, file, i as u64 + 1)?;

        let returns = match return_str {
            Some(ref ret_str) => {
                let (ty, _) = parse_python_type(ret_str.trim());
                match ty {
                    Some(t) => ReturnType::Type(t),
                    None => ReturnType::None,
                }
            }
            None => ReturnType::None,
        };

        let docs = extract_docstring(&lines, j);

        ir.functions.push(FunctionDef {
            wit_name: to_kebab_case(&fn_name),
            name: fn_name,
            params,
            returns,
            docs,
            source: SourceSpan {
                file: file.to_string(),
                line: i as u64 + 1,
                confidence: Confidence::Explicit,
            },
        });

        i = j;
    }

    Ok(())
}

/// True when the signature string ends with `:` at depth 0
fn sig_complete(sig: &str) -> bool {
    let mut depth = 0i32;
    for c in sig.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ':' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// Split `def name(params) -> ret:` into (params_str, Option<ret_str>)
fn split_signature(sig: &str) -> Result<(String, Option<String>)> {
    let open = sig.find('(').context("no opening paren in function signature")?;
    let mut depth = 0i32;
    let mut close = None;
    for (idx, c) in sig[open..].char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + idx);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close.context("no closing paren in function signature")?;
    let params_str = sig[open + 1..close].to_string();

    let after = &sig[close + 1..];
    let return_str = after.find("->").map(|arrow| {
        after[arrow + 2..]
            .trim_end_matches(':')
            .trim()
            .to_string()
    });

    Ok((params_str, return_str))
}

/// Parse `name: Type, name2: Type2` into ParamDef vec (skips self/cls)
fn parse_params(params_str: &str, file: &str, line: u64) -> Result<Vec<ParamDef>> {
    let mut params = Vec::new();
    for part in split_by_comma(params_str) {
        let part = part.trim();
        if part.is_empty() || part == "self" || part == "cls" {
            continue;
        }
        // Strip default value: `name: Type = default`
        let part = part.split('=').next().unwrap_or(part).trim();

        if let Some(colon) = part.find(':') {
            let name = part[..colon].trim().to_string();
            let ty_str = part[colon + 1..].trim();
            let (ty, _) = parse_python_type(ty_str);
            let ty = ty.ok_or_else(|| {
                anyhow::anyhow!(
                    "{}:{}: unsupported/missing type on param `{}`",
                    file,
                    line,
                    name
                )
            })?;
            params.push(ParamDef {
                wit_name: to_kebab_case(&name),
                name,
                ty,
            });
        } else {
            anyhow::bail!(
                "{}:{}: parameter `{}` has no type annotation",
                file,
                line,
                part
            );
        }
    }
    Ok(params)
}

/// Split a comma-separated string respecting brackets `[]` and parens `()`
fn split_by_comma(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in s.chars() {
        match c {
            '[' | '(' => {
                depth += 1;
                current.push(c);
            }
            ']' | ')' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

// ---------------------------------------------------------------------------
// Python type annotation → IRType mapping
// ---------------------------------------------------------------------------

/// Returns (IRType, is_optional). Returns (None, false) for unsupported types.
fn parse_python_type(ty_str: &str) -> (Option<IRType>, bool) {
    let ty = ty_str.trim();

    // Primitives
    match ty {
        "str" | "string" => return (Some(IRType::String), false),
        "int" => return (Some(IRType::S64), false),
        "float" => return (Some(IRType::F64), false),
        "bool" => return (Some(IRType::Bool), false),
        "bytes" => return (Some(IRType::List(Box::new(IRType::U8))), false),
        "None" => return (Some(IRType::Bool), false), // closest WIT has; treated as void-sentinel
        _ => {}
    }

    // Optional[T]
    if ty.starts_with("Optional[") && ty.ends_with(']') {
        let inner = &ty[9..ty.len() - 1];
        let (inner_ty, _) = parse_python_type(inner);
        return (inner_ty.map(|t| IRType::Option(Box::new(t))), true);
    }

    // T | None  (PEP 604)
    {
        let parts: Vec<&str> = ty.split('|').map(str::trim).collect();
        if parts.len() == 2 {
            let (a, b) = (parts[0], parts[1]);
            let (inner, none_side) = if b == "None" {
                (a, true)
            } else if a == "None" {
                (b, true)
            } else {
                ("", false)
            };
            if none_side {
                let (inner_ty, _) = parse_python_type(inner);
                return (inner_ty.map(|t| IRType::Option(Box::new(t))), true);
            }
        }
    }

    // list[T] / List[T]
    if (ty.starts_with("list[") || ty.starts_with("List[")) && ty.ends_with(']') {
        let inner = &ty[ty.find('[').unwrap() + 1..ty.len() - 1];
        let (inner_ty, _) = parse_python_type(inner);
        return (inner_ty.map(|t| IRType::List(Box::new(t))), false);
    }

    // tuple[A, B] / Tuple[A, B]
    if (ty.starts_with("tuple[") || ty.starts_with("Tuple[")) && ty.ends_with(']') {
        let inner = &ty[ty.find('[').unwrap() + 1..ty.len() - 1];
        let parts = split_by_comma(inner);
        let types: Vec<IRType> = parts
            .iter()
            .filter_map(|p| parse_python_type(p.trim()).0)
            .collect();
        if types.len() == parts.len() {
            return (Some(IRType::Tuple(types)), false);
        }
    }

    // dict[K, V] / Dict[K, V] → list<tuple<K, V>> (WIT has no native map)
    if (ty.starts_with("dict[") || ty.starts_with("Dict[")) && ty.ends_with(']') {
        let inner = &ty[ty.find('[').unwrap() + 1..ty.len() - 1];
        let parts = split_by_comma(inner);
        if parts.len() == 2 {
            let (k, _) = parse_python_type(parts[0].trim());
            let (v, _) = parse_python_type(parts[1].trim());
            if let (Some(k), Some(v)) = (k, v) {
                return (
                    Some(IRType::List(Box::new(IRType::Tuple(vec![k, v])))),
                    false,
                );
            }
        }
    }

    // Named reference (starts with uppercase letter → user-defined type)
    let first = ty.chars().next();
    if first.map(|c| c.is_uppercase()).unwrap_or(false)
        && ty.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return (Some(IRType::Named(ty.to_string())), false);
    }

    (None, false)
}

// ---------------------------------------------------------------------------
// Docstring extraction
// ---------------------------------------------------------------------------

fn extract_docstring(lines: &[&str], start: usize) -> Option<String> {
    let first = lines.get(start)?.trim();
    let (quote, start_str) = if first.starts_with("\"\"\"") {
        ("\"\"\"", &first[3..])
    } else if first.starts_with("'''") {
        ("'''", &first[3..])
    } else {
        return None;
    };

    // Single-line docstring: """text"""
    if start_str.ends_with(quote) && !start_str.is_empty() {
        return Some(start_str[..start_str.len() - 3].trim().to_string());
    }

    // Multi-line docstring
    let mut doc = start_str.to_string();
    for line in lines.iter().skip(start + 1) {
        let trimmed = line.trim();
        if trimmed.ends_with(quote) {
            let part = &trimmed[..trimmed.len() - 3];
            if !part.is_empty() {
                doc.push(' ');
                doc.push_str(part);
            }
            break;
        }
        if !doc.is_empty() {
            doc.push(' ');
        }
        doc.push_str(trimmed);
    }
    let doc = doc.trim().to_string();
    if doc.is_empty() {
        None
    } else {
        Some(doc)
    }
}
