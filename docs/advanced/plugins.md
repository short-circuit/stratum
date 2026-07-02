# Plugins

Stratum supports a WASM plugin runtime for extensibility.

!!! warning "Status"
    The plugin system is in development. The runtime (wasmtime) is integrated but the plugin API is still being finalized.

## Architecture

Plugins run as WebAssembly modules in the wasmtime runtime. They are:

- **Sandboxed** — no access to the filesystem or network unless explicitly permitted
- **Language-agnostic** — compile any language that targets WASM (Rust, Go, C, etc.)
- **Permission-gated** — each plugin declares what capabilities it needs

## Configuration

Plugins are configured in `.pkm/config.toml`:

```toml
[plugins]
enabled = ["my-plugin"]

[plugin.my-plugin]
some_option = "value"
```

## Developing Plugins

### Rust Plugin Example

```rust
use pkm_plugin::prelude::*;

#[plugin]
fn my_transform(content: String) -> String {
    content.to_uppercase()
}
```

### Building

```bash
cargo build --target wasm32-wasi
```

### Installing

Place the `.wasm` file in your vault's `.pkm/plugins/` directory.

## Permissions

Plugins declare required permissions in their manifest:

| Permission | Description |
|------------|-------------|
| `fs:read` | Read files in the vault |
| `fs:write` | Write files in the vault |
| `net:http` | Make HTTP requests |
| `block:read` | Read block data |
| `block:write` | Write and modify blocks |

## Current Limitations

- The plugin API is not yet stable
- No plugin registry or marketplace yet
- Only Rust compilation has been tested
- WASI preview 2 support is experimental
