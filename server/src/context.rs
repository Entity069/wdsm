use anyhow::Result;
use parser::CapabilitiesConfig;
use wasmtime::component::{Linker, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub struct WasiState {
    pub table: ResourceTable,
    pub ctx: WasiCtx,
    pub http: WasiHttpCtx,
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

pub struct ContextBuilder;

impl ContextBuilder {
    pub fn build_state(cap: &CapabilitiesConfig) -> Result<WasiState> {
        let mut builder = WasiCtxBuilder::new();

        if cap.stdio {
            builder.inherit_stdio();
        }

        if cap.env.inherit {
            builder.inherit_env();
        }
        for var_name in &cap.env.vars {
            if let Ok(val) = std::env::var(var_name) {
                builder.env(var_name, &val);
            }
        }

        if cap.network.tcp || cap.network.udp || cap.network.dns || cap.network.http {
            builder.inherit_network();
        }

        if cap.network.dns {
            builder.allow_ip_name_lookup(true);
        }
        if cap.network.tcp {
            builder.allow_tcp(true);
        }
        if cap.network.udp {
            builder.allow_udp(true);
        }

        for vol in &cap.filesystem {
            let (dir_perms, file_perms) = if vol.read_only {
                (wasmtime_wasi::DirPerms::READ, wasmtime_wasi::FilePerms::READ)
            } else {
                (wasmtime_wasi::DirPerms::all(), wasmtime_wasi::FilePerms::all())
            };

            if let Err(e) = builder.preopened_dir(&vol.host, &vol.guest, dir_perms, file_perms) {
                eprintln!("[!] failed to preopen directory {}: {}", vol.host, e);
            }
        }

        let wasi_ctx = builder.build();
        let http = WasiHttpCtx::new();

        Ok(WasiState {
            table: ResourceTable::new(),
            ctx: wasi_ctx,
            http,
        })
    }

    pub fn configure_linker(
        linker: &mut Linker<WasiState>,
        cap: &CapabilitiesConfig,
    ) -> Result<()> {
        wasmtime_wasi::add_to_linker_async(linker)?;

        if cap.network.http {
            wasmtime_wasi_http::add_only_http_to_linker_async(linker)?;
        }

        for (name, val) in &cap.custom {
            println!(
                "[+] dynamic capability context evaluated: {} -> {:?}",
                name, val
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::CapabilitiesConfig;
    use wasmtime::{Config as WasmConfig, Engine};

    #[test]
    fn test_context_builder_sandboxed() {
        let cap = CapabilitiesConfig::default();
        let state = ContextBuilder::build_state(&cap);
        assert!(state.is_ok());

        let mut wasm_config = WasmConfig::new();
        wasm_config.wasm_component_model(true);
        wasm_config.async_support(true);
        let engine = Engine::new(&wasm_config).unwrap();
        let mut linker = Linker::new(&engine);
        let link_res = ContextBuilder::configure_linker(&mut linker, &cap);
        assert!(link_res.is_ok());
    }
}
