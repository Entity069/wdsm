use anyhow::{Context, Result};
use axum::{Router, routing::{get, post}};
use parser::Config;
use registry::FnService;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;
use uuid::Uuid;
use wasmtime::component::Component;
use wasmtime::{Config as WasmConfig, Engine};

lazy_static::lazy_static! {
    static ref SERVERS: Arc<RwLock<HashMap<String, oneshot::Sender<()>>>> = 
        Arc::new(RwLock::new(HashMap::new()));
}

pub struct ServerState {
    pub engine: Engine,
    pub component: Component,
    pub config: Config,
}

pub async fn start_server(config: Config, wasm_file: PathBuf) -> Result<FnService> {

    let mut wasm_config = WasmConfig::new();
    wasm_config.wasm_component_model(true);
    // wasm_config.async_support(true);
    
    let engine = Engine::new(&wasm_config)?;
    let component = Component::from_file(&engine, &wasm_file)
        .map_err(anyhow::Error::from)
        .context("Failed to load WASM component")?;

    let state = Arc::new(ServerState {
        engine,
        component,
        config: config.clone(),
    });

    let app = match config.method.as_str() {
        "GET" => Router::new()
            .route(&config.endpoint, get(crate::handler::handle_request))
            .route("/__health", get(crate::handler::health))
            .with_state(state),
            
        "POST" => Router::new()
            .route(&config.endpoint, post(crate::handler::handle_request))
            .route("/__health", get(crate::handler::health))
            .with_state(state),
            
        _ => anyhow::bail!("Unsupported method: {}", config.method),
    };

    let (tx, rx) = oneshot::channel::<()>();
    let deployment_id = Uuid::new_v4().to_string();

    {
        let mut servers = SERVERS.write().unwrap();
        servers.insert(deployment_id.clone(), tx);
    }

    let addr = format!("127.0.0.1:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(format!("Failed to bind to {}", addr))?;

    let deployment_id_clone = deployment_id.clone();
    
    tokio::spawn(async move {
        logger::log_deployment(&deployment_id_clone, "started");
        
        let server = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                rx.await.ok();
            });

        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
            logger::log_deployment(&deployment_id_clone, &format!("error: {}", e));
        }

        logger::log_deployment(&deployment_id_clone, "stopped");
    });

    Ok(FnService {
        id: deployment_id,
        name: config.name,
        endpoint: config.endpoint,
        port: config.port,
        method: config.method,
        wasm_path: wasm_file.to_string_lossy().to_string(),
    })
}

pub async fn stop_server(deployment_id: &str) -> Result<()> {
    let tx = {
        let mut servers = SERVERS.write().unwrap();
        servers.remove(deployment_id)
    };

    if let Some(tx) = tx {
        let _ = tx.send(());
        Ok(())
    } else {
        anyhow::bail!("Server not found: {}", deployment_id)
    }
}