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
use wasmtime::{Store};
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

    let param_value = if state.config.method == "GET" {
        params
            .and_then(|p| p.data.get(&state.config.payload).cloned())
            .unwrap_or_default()
    } else {
        body.and_then(|b| {
            b.get(&state.config.payload)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
    };

    match execute_wasm(&state, &param_value).await {
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

async fn execute_wasm(state: &ServerState, param: &str) -> anyhow::Result<String> {
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

    let func = instance.get_typed_func::<(String,), (String,)>(
        &mut store,
        &state.config.entrypoint_function,
    )?;

    let (result,) = func.call_async(&mut store, (param.to_string(),)).await?;

    logger::log_request(&state.config.name, param, &result);

    Ok(result)
}