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


3) Networking Example

```bash
wdsm deploy --config examples/ts/net/config.yml

curl -X POST http://127.0.0.1:3005/net -H "Content-Type: application/json" -d '{"msg": "Hello WASI!", "request_catcher": "heyjude"}'
```

## Configuration File

Below is an example of a configuration file with optional WASI capabilities:

```yaml
name: net
language: typescript
entrypoint: net.ts
entrypoint_function: net
port: 3005
endpoint: /net
method: POST
payload:
  - msg: string
  - request_catcher: string
return_type: string

capabilities:
  stdio: true            # Inherit stdio for logging (default: true)
  env:
    inherit: false       # Inherit host environment variables (default: false)
    vars:                # Explicit environment variables to expose
      - API_KEY
  network:               # WASI network capabilities (default: false for security)
    http: true           # Enable WASI HTTP outbound client
    tcp: true            # Allow TCP sockets
    udp: false           # Allow UDP sockets
    dns: true            # Allow DNS resolution
  filesystem:            # WASI filesystem volume
    - host: "./data"
      guest: "/data"
      read_only: true
```

Supported basic types: `string|str`, `int|i32`, `i64`, `float|f32`, `f64`, `boolean|bool`.

## Features

- WASI capability sandboxing configurable per endpoint via `capabilities` in `config.yml`.
- Outbound WASI networking & HTTP client support (`fetch` in JS/TS, `urllib`/HTTP in Python) enabled declaratively via `capabilities.network.http`.

## Limitations

- very much limited

## License

See [LICENSE](./LICENSE).


