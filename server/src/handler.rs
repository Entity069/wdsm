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
    // extract request parameters into a HashMap
    let request_params: HashMap<String, String> = if state.config.method == "GET" {
        params.map(|p| p.0.data).unwrap_or_default()
    } else {
        body.and_then(|b| {
            if let Value::Object(map) = b.0 {
                Some(
                    map.into_iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
    };

    //  parameter map from config payload definition
    let mut typed_params: HashMap<String, (String, String)> = HashMap::new();
    
    for paramss in &state.config.payload {
        for (param_name, param_type) in paramss {
            if let Some(value) = request_params.get(param_name) {
                typed_params.insert(param_name.clone(), (value.clone(), param_type.clone()));
            } else {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    format!("Missing required parameter: {}", param_name),
                )
                    .into_response();
            }
        }
    }

    match execute_wasm(&state, typed_params).await {
        Ok(result) => result.into_response(),
        Err(e) => {
            logger::log_error(&format!("[!] WASM execution error: {}", e));
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error: {}", e),
            )
                .into_response()
        }
    }
}

async fn execute_wasm(
    state: &ServerState,
    params: HashMap<String, (String, String)>,
) -> anyhow::Result<String> {
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

    let func = instance.get_func(&mut store, &state.config.entrypoint_function)
        .ok_or_else(|| anyhow::anyhow!("Function {} not found", state.config.entrypoint_function))?;

    let mut param_values: Vec<Val> = Vec::new();
    
    for paramss in &state.config.payload {
        for (param_name, param_type) in paramss {
            if let Some((value_str, _)) = params.get(param_name) {
                let val = parser::parse_value(value_str, param_type)?;
                param_values.push(val);
            }
        }
    }

    let mut results = vec![Val::Bool(false)];

    func.call_async(&mut store, &param_values, &mut results).await?;
    
    let result_str = parser::format_result(&results[0], &state.config.return_type)?;

    let params_log: Vec<String> = state.config.payload.iter()
        .flat_map(|p| p.keys())
        .filter_map(|k| params.get(k).map(|(v, _)| format!("{}={}", k, v)))
        .collect();
    
    logger::log_request(&state.config.name, &params_log.join(", "), &result_str);

    Ok(result_str)
}