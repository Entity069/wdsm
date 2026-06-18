use anyhow::Result;
use wasmtime::component::Val;

pub fn parse_value(value_str: &str, type_str: &str) -> Result<Val> {
    match type_str {
        "string" | "str" => Ok(Val::String(value_str.to_string().into())),
        "int" | "i32" => {
            let parsed = value_str.parse::<i32>()
                .map_err(|_| anyhow::anyhow!("fuck i32"))?;

            Ok(Val::S32(parsed))
        }
        "i64" => {
            let parsed = value_str.parse::<i64>()
                .map_err(|_| anyhow::anyhow!("fuck i64"))?;

            Ok(Val::S64(parsed))
        }
        "float" | "f32" => {
            let parsed = value_str.parse::<f32>()
                .map_err(|_| anyhow::anyhow!("fuck f32"))?;

            Ok(Val::Float32(parsed))
        }
        "f64" => {
            let parsed = value_str.parse::<f64>()
                .map_err(|_| anyhow::anyhow!("fuck f64"))?;

            Ok(Val::Float64(parsed))
        }
        "boolean" | "bool" => {
            let parsed = value_str.parse::<bool>()
                .map_err(|_| anyhow::anyhow!("fuck bool"))?;
            Ok(Val::Bool(parsed))
        }
        _ => Ok(Val::String(value_str.to_string().into())),
    }
}

pub fn format_result(val: &Val, return_type: &str) -> Result<String> {
    match (val, return_type) {
        (Val::String(s), _) => Ok(s.to_string()),
        (Val::S32(n), _) => Ok(n.to_string()),
        (Val::S64(n), _) => Ok(n.to_string()),
        (Val::Float32(f), _) => Ok(f.to_string()),
        (Val::Float64(f), _) => Ok(f.to_string()),
        (Val::Bool(b), _) => Ok(b.to_string()),
        
        _ => Err(anyhow::anyhow!("not implemented")),
    }
}
