# wdsm

wdsm is a poor man's attempt to make a FaaS runtime via WASM Component Model. It takes a config file and program file (currently JavaScript files), makes WebAssembly Interface Types (.wit) file, compiles WASM files, and then serves then through the endpoint defined in config.

## Requirements

- Rust
- Node.js + npm (for `jco`)
- `@bytecodealliance/jco` installed globally

Install `jco`:

```bash
npm install -g @bytecodealliance/jco
```

## Install / Build

Build the CLI and make `wdsm` available:

```bash
cargo install --path cli
```

Alternatively, build in-place:

```bash
cargo build --release
```

## Quick start

Try the included examples.

1) Hello

```bash
wdsm deploy --config examples/hello/config.yml

# from another terminal
curl "http://127.0.0.1:3001/hello?name=World"
# -> Hello, World!
```

2) Addition (query params)

```bash
wdsm deploy --config examples/add/config.yml

# from another terminal
curl "http://127.0.0.1:3001/add?a=69.0&b=0.420"
# -> 69.420
```


## Configuration File

Below is an example of config file

```yaml
name: hello
language: javascript
entrypoint: hello.js
entrypoint_function: hello
port: 3001
endpoint: /hello
method: GET
payload:
	- name: string
return_type: string
```

Supported basic types: `string|str`, `int|i32`, `i64`, `float|f32`, `f64`, `boolean|bool`.

## Limitations

- very much limited
- no WASI modules

## License

See [LICENSE](./LICENSE).
