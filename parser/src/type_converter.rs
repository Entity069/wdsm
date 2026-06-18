use anyhow::{bail, Context, Result};
use serde_json::Value as JsonValue;
use wasmtime::component::types::Type as WasmType;
use wasmtime::component::Val;

pub fn json_to_val(json: &JsonValue, ty: &WasmType) -> Result<Val> {
    match ty {
        WasmType::Bool => match json {
            JsonValue::Bool(b) => Ok(Val::Bool(*b)),
            JsonValue::String(s) => Ok(Val::Bool(s.parse::<bool>().context("invalid bool")?)),
            _ => bail!("expected bool, got {:?}", json),
        },

        WasmType::S8 => Ok(Val::S8(json_to_i64(json)? as i8)),
        WasmType::U8 => Ok(Val::U8(json_to_u64(json)? as u8)),
        WasmType::S16 => Ok(Val::S16(json_to_i64(json)? as i16)),
        WasmType::U16 => Ok(Val::U16(json_to_u64(json)? as u16)),
        WasmType::S32 => Ok(Val::S32(json_to_i64(json)? as i32)),
        WasmType::U32 => Ok(Val::U32(json_to_u64(json)? as u32)),
        WasmType::S64 => Ok(Val::S64(json_to_i64(json)?)),
        WasmType::U64 => Ok(Val::U64(json_to_u64(json)?)),

        WasmType::Float32 => {
            let f = json_to_f64(json)? as f32;
            Ok(Val::Float32(f))
        }
        WasmType::Float64 => {
            let f = json_to_f64(json)?;
            Ok(Val::Float64(f))
        }

        WasmType::Char => {
            let s = json.as_str().context("expected string for char")?;
            let c = s.chars().next().context("empty string for char")?;
            Ok(Val::Char(c))
        }

        WasmType::String => {
            let s = match json {
                JsonValue::String(s) => s.clone(),
                other => other.to_string(),
            };
            Ok(Val::String(s.into()))
        }

        WasmType::List(list_ty) => {
            let arr = json.as_array().context("expected array for list")?;
            let element_ty = list_ty.ty();
            let vals: Vec<Val> = arr
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    json_to_val(item, &element_ty)
                        .with_context(|| format!("list element [{}]", i))
                })
                .collect::<Result<_>>()?;
            Ok(Val::List(vals))
        }

        WasmType::Record(record_ty) => {
            let obj = json.as_object().context("expected object for record")?;
            let fields: Vec<(String, Val)> = record_ty
                .fields()
                .map(|field| {
                    let field_name = field.name.to_string();
                    let json_val = obj.get(&field_name).unwrap_or(&JsonValue::Null);
                    let val = json_to_val(json_val, &field.ty)
                        .with_context(|| format!("record field '{}'", field_name))?;
                    Ok((field_name, val))
                })
                .collect::<Result<_>>()?;
            Ok(Val::Record(fields))
        }

        WasmType::Option(opt_ty) => {
            if json.is_null() {
                Ok(Val::Option(None))
            } else {
                let inner = json_to_val(json, &opt_ty.ty())?;
                Ok(Val::Option(Some(Box::new(inner))))
            }
        }

        WasmType::Tuple(tuple_ty) => {
            let arr = json.as_array().context("expected array for tuple")?;
            let types: Vec<_> = tuple_ty.types().collect();
            if arr.len() != types.len() {
                bail!("tuple length mismatch: expected {}, got {}", types.len(), arr.len());
            }
            let vals: Vec<Val> = arr
                .iter()
                .zip(types.iter())
                .map(|(item, ty)| json_to_val(item, ty))
                .collect::<Result<_>>()?;
            Ok(Val::Tuple(vals))
        }

        WasmType::Enum(enum_ty) => {
            let s = json.as_str().context("expected string for enum")?;
            let names: Vec<_> = enum_ty.names().collect();
            if !names.contains(&s) {
                bail!("unknown enum case '{}', expected one of {:?}", s, names);
            }
            Ok(Val::Enum(s.to_string()))
        }

        WasmType::Flags(flags_ty) => {
            let arr = json.as_array().context("expected array for flags")?;
            let names: Vec<String> = arr
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .context("flags elements must be strings")
                })
                .collect::<Result<_>>()?;
            // validate flag names
            let valid_names: Vec<_> = flags_ty.names().collect();
            for name in &names {
                if !valid_names.contains(&name.as_str()) {
                    bail!("unknown flag '{}', expected one of {:?}", name, valid_names);
                }
            }
            Ok(Val::Flags(names))
        }

        WasmType::Result(result_ty) => {
            if let Some(obj) = json.as_object() {
                if let Some(ok_val) = obj.get("ok") {
                    if let Some(ok_ty) = result_ty.ok() {
                        let inner = json_to_val(ok_val, &ok_ty)?;
                        return Ok(Val::Result(Ok(Some(Box::new(inner)))));
                    } else {
                        return Ok(Val::Result(Ok(None)));
                    }
                }
                if let Some(err_val) = obj.get("err") {
                    if let Some(err_ty) = result_ty.err() {
                        let inner = json_to_val(err_val, &err_ty)?;
                        return Ok(Val::Result(Err(Some(Box::new(inner)))));
                    } else {
                        return Ok(Val::Result(Err(None)));
                    }
                }
            }
            bail!("expected object with 'ok' or 'err' for result type")
        }

        _ => bail!("unsupported WASM type: {:?}", ty),
    }
}

pub fn val_to_json(val: &Val) -> Result<JsonValue> {
    match val {
        Val::Bool(b) => Ok(JsonValue::Bool(*b)),
        Val::S8(n) => Ok(JsonValue::Number((*n as i64).into())),
        Val::U8(n) => Ok(JsonValue::Number((*n as u64).into())),
        Val::S16(n) => Ok(JsonValue::Number((*n as i64).into())),
        Val::U16(n) => Ok(JsonValue::Number((*n as u64).into())),
        Val::S32(n) => Ok(JsonValue::Number((*n as i64).into())),
        Val::U32(n) => Ok(JsonValue::Number((*n as u64).into())),
        Val::S64(n) => Ok(serde_json::to_value(n)?),
        Val::U64(n) => Ok(serde_json::to_value(n)?),
        Val::Float32(f) => Ok(serde_json::to_value(f)?),
        Val::Float64(f) => Ok(serde_json::to_value(f)?),
        Val::Char(c) => Ok(JsonValue::String(c.to_string())),
        Val::String(s) => Ok(JsonValue::String(s.to_string())),

        Val::List(items) => {
            let arr: Vec<JsonValue> = items
                .iter()
                .map(val_to_json)
                .collect::<Result<_>>()?;
            Ok(JsonValue::Array(arr))
        }

        Val::Record(fields) => {
            let mut map = serde_json::Map::new();
            for (name, val) in fields {
                map.insert(name.clone(), val_to_json(val)?);
            }
            Ok(JsonValue::Object(map))
        }

        Val::Tuple(items) => {
            let arr: Vec<JsonValue> = items
                .iter()
                .map(val_to_json)
                .collect::<Result<_>>()?;
            Ok(JsonValue::Array(arr))
        }

        Val::Option(inner) => match inner {
            Some(val) => val_to_json(val),
            None => Ok(JsonValue::Null),
        },

        Val::Enum(name) => Ok(JsonValue::String(name.clone())),

        Val::Flags(names) => {
            let arr: Vec<JsonValue> = names
                .iter()
                .map(|n| JsonValue::String(n.clone()))
                .collect();
            Ok(JsonValue::Array(arr))
        }

        Val::Result(inner) => {
            let mut map = serde_json::Map::new();
            match inner {
                Ok(Some(val)) => {
                    map.insert("ok".into(), val_to_json(val)?);
                }
                Ok(None) => {
                    map.insert("ok".into(), JsonValue::Null);
                }
                Err(Some(val)) => {
                    map.insert("err".into(), val_to_json(val)?);
                }
                Err(None) => {
                    map.insert("err".into(), JsonValue::Null);
                }
            }
            Ok(JsonValue::Object(map))
        }

        _ => bail!("unsupported val type: {:?}", val),
    }
}

fn json_to_i64(json: &JsonValue) -> Result<i64> {
    match json {
        JsonValue::Number(n) => n.as_i64().context("number out of i64 range"),
        JsonValue::String(s) => s.parse::<i64>().context("invalid integer string"),
        _ => bail!("expected number, got {:?}", json),
    }
}

fn json_to_u64(json: &JsonValue) -> Result<u64> {
    match json {
        JsonValue::Number(n) => n.as_u64().context("number out of u64 range"),
        JsonValue::String(s) => s.parse::<u64>().context("invalid unsigned integer string"),
        _ => bail!("expected number, got {:?}", json),
    }
}

fn json_to_f64(json: &JsonValue) -> Result<f64> {
    match json {
        JsonValue::Number(n) => n.as_f64().context("number out of f64 range"),
        JsonValue::String(s) => s.parse::<f64>().context("invalid float string"),
        _ => bail!("expected number, got {:?}", json),
    }
}
