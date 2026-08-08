#[cfg(test)]
mod tests {
    use parser::Config;
    use server::runtime::ServerState;
    use serde_json::json;
    use std::path::PathBuf;
    use wasmtime::component::Component;
    use wasmtime::{Config as WasmConfig, Engine};

    #[tokio::test]
    async fn test_wasm_net() {
        let wasm_file = PathBuf::from("../examples/ts/net/.wdsm/function.wasm");
        if !wasm_file.exists() {
            println!("wasm file does not exist at {:?}", wasm_file);
            return;
        }

        let mut wasm_config = WasmConfig::new();
        wasm_config.wasm_component_model(true);
        wasm_config.async_support(true);

        let engine = Engine::new(&wasm_config).unwrap();
        let component = Component::from_file(&engine, &wasm_file).unwrap();

        let config = Config {
            name: "net".to_string(),
            language: "typescript".to_string(),
            entrypoint: "net.ts".to_string(),
            entrypoint_function: "net".to_string(),
            port: 3005,
            endpoint: "/net".to_string(),
            method: "POST".to_string(),
            payload: vec![
                [("msg".to_string(), "string".to_string())].into_iter().collect(),
                [("request_catcher".to_string(), "string".to_string())].into_iter().collect(),
            ],
            return_type: "string".to_string(),
        };

        let state = ServerState {
            engine,
            component,
            config,
        };

        let payload = json!({
            "msg": "Test Message",
            "request_catcher": "heyjude"
        });

        println!("Calling execute_wasm TS...");
        let res = server::handler::execute_wasm(&state, &payload).await;
        println!("Result TS: {:?}", res);
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_python_wasm_net() {
        let wasm_file = PathBuf::from("../examples/python/net/.wdsm/function.wasm");
        if !wasm_file.exists() {
            println!("python wasm file does not exist at {:?}", wasm_file);
            return;
        }

        let mut wasm_config = WasmConfig::new();
        wasm_config.wasm_component_model(true);
        wasm_config.async_support(true);

        let engine = Engine::new(&wasm_config).unwrap();
        let component = Component::from_file(&engine, &wasm_file).unwrap();

        let config = Config {
            name: "net-py".to_string(),
            language: "python".to_string(),
            entrypoint: "net.py".to_string(),
            entrypoint_function: "net".to_string(),
            port: 3015,
            endpoint: "/net".to_string(),
            method: "POST".to_string(),
            payload: vec![
                [("msg".to_string(), "string".to_string())].into_iter().collect(),
                [("request_catcher".to_string(), "string".to_string())].into_iter().collect(),
            ],
            return_type: "string".to_string(),
        };

        let state = ServerState {
            engine,
            component,
            config,
        };

        let payload = json!({
            "msg": "wasmtimegae",
            "request_catcher": "heyjude"
        });

        println!("Calling execute_wasm Python...");
        let res = server::handler::execute_wasm(&state, &payload).await;
        println!("Result Python: {:?}", res);
        assert!(res.is_ok());
        if let Ok(serde_json::Value::String(s)) = res {
            assert!(s.contains("Successfully sent POST request"));
        }
    }
}
