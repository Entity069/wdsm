use anyhow::Result;
use crate::ast_analyzer::{ModuleAnalysis, RecordDef, VariantDef, EnumDef};
use crate::config::Config;

pub fn generate(config: &Config, analysis: &ModuleAnalysis) -> Result<String> {
    let mut wit = String::new();

    wit.push_str(&format!("package wdsm:{};\n\n", config.name));
    wit.push_str(&format!("world {} {{\n", config.name));
    
    // Generate type definitions
    
    // Enums first (no dependencies)
    for enum_def in analysis.enums.values() {
        wit.push_str(&generate_enum(enum_def));
        wit.push('\n');
    }
    
    // Records (may reference other types)
    for record_def in analysis.records.values() {
        wit.push_str(&generate_record(record_def));
        wit.push('\n');
    }
    
    // Variants
    for variant_def in analysis.variants.values() {
        wit.push_str(&generate_variant(variant_def));
        wit.push('\n');
    }

    // Generate function export
    if let Some(fn_sig) = analysis.functions.get(&config.entrypoint_function) {
        let params: Vec<String> = fn_sig
            .params
            .iter()
            .map(|p| format!("{}: {}", to_kebab_case(&p.name), p.param_type.to_wit_string()))
            .collect();

        let return_type = fn_sig.return_type.to_wit_string();

        wit.push_str(&format!(
            "  export {}: func({}) -> {};\n",
            to_kebab_case(&config.entrypoint_function),
            params.join(", "),
            return_type
        ));
    } else {
        // Fallback to config-based generation
        let params: Vec<String> = config
            .payload
            .iter()
            .flat_map(|param_map| {
                param_map.iter().map(|(name, type_str)| {
                    format!("{}: {}", name, map_type(type_str))
                })
            })
            .collect();

        let return_type = map_type(&config.return_type);

        wit.push_str(&format!(
            "  export {}: func({}) -> {};\n",
            config.entrypoint_function,
            params.join(", "),
            return_type
        ));
    }

    wit.push_str("}\n");

    Ok(wit)
}

fn generate_record(record: &RecordDef) -> String {
    let mut wit = format!("  record {} {{\n", record.name);
    
    for (field_name, field_type) in &record.fields {
        wit.push_str(&format!("    {}: {},\n", field_name, field_type.to_wit_string()));
    }
    
    wit.push_str("  }\n");
    wit
}

fn generate_variant(variant: &VariantDef) -> String {
    let mut wit = format!("  variant {} {{\n", variant.name);
    
    for case in &variant.cases {
        if let Some(payload) = &case.payload {
            wit.push_str(&format!("    {}({}),\n", case.name, payload.to_wit_string()));
        } else {
            wit.push_str(&format!("    {},\n", case.name));
        }
    }
    
    wit.push_str("  }\n");
    wit
}

fn generate_enum(enum_def: &EnumDef) -> String {
    let mut wit = format!("  enum {} {{\n", enum_def.name);
    
    for case in &enum_def.cases {
        wit.push_str(&format!("    {},\n", case));
    }
    
    wit.push_str("  }\n");
    wit
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
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

fn map_type(type_str: &str) -> &str {
    match type_str {
        "string" | "str" => "string",
        "int" | "i32" => "s32",
        "i64" => "s64",
        "float" | "f32" => "f32",
        "f64" => "f64",
        "boolean" | "bool" => "bool",
        _ => "string",
    }
}