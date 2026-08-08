use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::component::*;
use wasmtime::Store;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

use crate::runtime::ServerState;

// health endpoint used for readiness checks
pub async fn health() -> &'static str {
    "ok"
}

struct WasiState {
    table: ResourceTable,
    ctx: WasiCtx,
    http: WasiHttpCtx,
}

impl WasiView for WasiState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl WasiHttpView for WasiState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }
}

#[derive(Deserialize)]
pub(crate) struct Params {
    #[serde(flatten)]
    data: HashMap<String, String>,
}

pub async fn handle_request(
    State(state): State<Arc<ServerState>>,
    params: Option<Query<Params>>,
    body: Option<Json<Value>>,
) -> Response {
    // extract the raw JSON body or build one from query params
    let json_body: Value = if state.config.method == "GET" {
        let qp = params.map(|p| p.0.data).unwrap_or_default();
        let map: serde_json::Map<String, Value> = qp
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        Value::Object(map)
    } else {
        body.map(|b| b.0).unwrap_or(Value::Object(Default::default()))
    };

    match execute_wasm(&state, &json_body).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => {
            logger::log_error(&format!("[!] WASM execution error: {}", e));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{:#}", e) })),
            )
                .into_response()
        }
    }
}

async fn execute_wasm(
    state: &ServerState,
    json_body: &Value,
) -> anyhow::Result<Value> {
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .build();

    let http = WasiHttpCtx::new();

    let wasi_state = WasiState {
        table: ResourceTable::new(),
        ctx: wasi_ctx,
        http,
    };

    let mut store = Store::new(&state.engine, wasi_state);

    let mut linker = Linker::new(&state.engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)?;

    let instance = linker
        .instantiate_async(&mut store, &state.component)
        .await?;

    let func_name = &state.config.entrypoint_function;
    let func = instance.get_func(&mut store, func_name)
        .or_else(|| instance.get_func(&mut store, &to_kebab(func_name)))
        .or_else(|| {
            // Search exports inside exported interface instances if any
            for (name, item) in instance.exports(&mut store) {
                if let ComponentItem::ComponentInstance(inst) = item {
                    if let Some(f) = inst.get_func(&mut store, func_name)
                        .or_else(|| inst.get_func(&mut store, &to_kebab(func_name))) {
                        return Some(f);
                    }
                }
            }
            None
        })
        .ok_or_else(|| {
            let export_names: Vec<String> = instance.exports(&mut store).map(|(n, _)| n.to_string()).collect();
            anyhow::anyhow!("Function {} not found. Available component exports: {:?}", func_name, export_names)
        })?;

    // get parameter types from the wasm component
    let param_types: Box<[types::Type]> = func.params(&store);

    // get param names from config payload tp preserve ordering
    let param_names: Vec<String> = state.config.payload.iter()
        .flat_map(|entry| entry.keys().cloned())
        .collect();

    if param_names.len() != param_types.len() {
        return Err(anyhow::anyhow!(
            "Config payload has {} params but WASM function expects {}",
            param_names.len(), param_types.len()
        ));
    }

    // build parameter values by matching json fields to WASM types
    let json_obj = json_body.as_object();
    let mut param_values: Vec<Val> = Vec::new();

    for (param_name, param_type) in param_names.iter().zip(param_types.iter()) {
        // try both camelCase and kebab-case lookups
        let json_val = json_obj
            .and_then(|obj| {
                obj.get(param_name)
                    .or_else(|| obj.get(&to_kebab(param_name)))
            })
            .unwrap_or(&Value::Null);

        if json_val.is_null() {
            // allow null for option types error for everything else
            if !matches!(param_type, types::Type::Option(_)) {
                return Err(anyhow::anyhow!("Missing required parameter: {}", param_name));
            }
        }

        let val = parser::json_to_val(json_val, param_type)
            .map_err(|e| anyhow::anyhow!("Parameter '{}': {:#}", param_name, e))?;
        param_values.push(val);
    }

    let result_types = func.results(&store);
    let mut results = vec![Val::Bool(false); result_types.len()];

    func.call_async(&mut store, &param_values, &mut results).await?;

    // convert result to json
    let result_json = if let Some(first_result) = results.first() {
        parser::val_to_json(first_result)
            .map_err(|e| anyhow::anyhow!("Failed to serialize result: {:#}", e))?
    } else {
        Value::Null
    };

    logger::log_request(
        &state.config.name,
        &param_names.join(", "),
        &result_json.to_string(),
    );

    Ok(result_json)
}

fn to_kebab(s: &str) -> String {
    let mut result = std::string::String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}