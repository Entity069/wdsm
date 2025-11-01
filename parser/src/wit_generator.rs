use anyhow::Result;
use crate::config::Config;
use crate::ast_analyzer::FnSign;

// TODO: Implement this more completely based on refs
// ref: https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md
//      https://component-model.bytecodealliance.org/design/wit.html

pub fn generate(config: &Config, signature: &FnSign) -> Result<String> {
    let mut wit = String::new();

    wit.push_str(&format!("package wdsm:{};\n\n", config.name));
    wit.push_str(&format!("world {} {{\n", config.name));
    
    // add WASI imports based on needs
    // TODO: make it work, doesnt work now
    // wit.push_str("  import wasi:io/streams@0.2.0;\n");
    // wit.push_str("  import wasi:cli/environment@0.2.0;\n");
    // wit.push_str("  import wasi:clocks/wall-clock@0.2.0;\n");
    // wit.push_str("  import wasi:random/random@0.2.0;\n");
    // wit.push_str("  import wasi:filesystem/types@0.2.0;\n");
    // wit.push_str("  import wasi:filesystem/preopens@0.2.0;\n");
    // wit.push_str("\n");

    let fn_signature = gen_fn_sign(&config.entrypoint_function, signature);

    wit.push_str(&format!("  export {};\n", fn_signature));
    wit.push_str("}\n");

    Ok(wit)
}

fn gen_fn_sign(func_name: &str, signature: &FnSign) -> String {
    let params: Vec<String> = signature
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, map_type(&p.param_type)))
        .collect();

    let return_type = map_type(&signature.return_type);

    format!("{}: func({}) -> {}", func_name, params.join(", "), return_type)
}

fn map_type(js_type: &str) -> &str {
    match js_type {
        "string" => "string",
        "number" | "f64" => "f64",
        "boolean" | "bool" => "bool",
        _ => "string",
    }
}