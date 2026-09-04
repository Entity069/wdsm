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
use crate::context::ContextBuilder;
use crate::runtime::ServerState;

// health endpoint used for readiness checks
pub async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
pub struct Params {
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

pub async fn execute_wasm(
    state: &ServerState,
    json_body: &Value,
) -> anyhow::Result<Value> {
    let wasi_state = ContextBuilder::build_state(&state.config.capabilities)?;
    let mut store = Store::new(&state.engine, wasi_state);

    let mut linker = Linker::new(&state.engine);
    ContextBuilder::configure_linker(&mut linker, &state.config.capabilities)?;

    let instance = linker
        .instantiate_async(&mut store, &state.component)
        .await?;

    let func_name = &state.config.entrypoint_function;
    let kebab_name = to_kebab(func_name);
    let world_name = &state.config.name;

    let candidates = vec![
        func_name.clone(),
        kebab_name.clone(),
        format!("{}/{}", world_name, func_name),
        format!("{}/{}", world_name, kebab_name),
        format!("wdsm:{}/{}", world_name, func_name),
        format!("wdsm:{}/{}", world_name, kebab_name),
    ];

    let mut found_func = None;
    for cand in &candidates {
        if let Some(f) = instance.get_func(&mut store, cand) {
            found_func = Some(f);
            break;
        }
    }

    let func = found_func.ok_or_else(|| {
        anyhow::anyhow!(
            "Function {} not found in component. Tried candidates: {:?}",
            func_name,
            candidates
        )
    })?;

    // get parameter types from the wasm component
    let func_ty = func.ty(&store);
    let param_types: Vec<types::Type> = func_ty.params().map(|(_, ty)| ty).collect();

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

    let result_types: Vec<types::Type> = func.ty(&store).results().collect();
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
    use heck::ToKebabCase;
    s.to_kebab_case()
}