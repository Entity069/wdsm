use anyhow::Result;
use crate::config::Config;

// TODO: Implement this more completely based on refs
// ref: https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md
//      https://component-model.bytecodealliance.org/design/wit.html

pub fn generate(config: &Config) -> Result<String> {
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

    let fn_signature = gen_fn_sign(&config.entrypoint_function, config);

    wit.push_str(&format!("  export {};\n", fn_signature));
    wit.push_str("}\n");

    Ok(wit)
}

fn gen_fn_sign(func_name: &str, config: &Config) -> String {
    let params: Vec<String> = config
        .payload
        .iter()
        .flat_map(|param_map| {
            param_map.iter().map(|(name, type_str)| {
                format!("{}: {}", name, map_type(type_str))
            })
        })
        .collect();

    let return_type = map_type(&config.return_type);

    format!("{}: func({}) -> {}", func_name, params.join(", "), return_type)
}

fn map_type(type_str: &str) -> &str {
    match type_str {
        "string" | "str" => "string",
        "int" | "i32" => "s32",
        "i64" => "s64",
        "float" | "f32" => "f32",
        "f64" => "f64",
        "boolean" | "bool" => "bool",
        _ => "string",
    }
}