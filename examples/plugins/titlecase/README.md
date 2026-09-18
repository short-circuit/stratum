# Title Case — example Stratum plugin

A real Rust (`wasm32-wasip1`, `std`) plugin that title-cases a note's content
whenever the note is saved. It is the canonical reference implementation for
the recommended plugin toolchain: it builds with plain `cargo`, self-embeds its
manifest (no separate injection step), and uses the host API for all vault I/O.

- **Manifest:** embedded in the `stratum:manifest` custom section
  (`src/lib.rs`, `#[link_section]`).
- **Hook:** `onSave` (`(i32, i32) -> i32`, the host ABI).
- **Permissions:** `file:read`, `file:write`.
- **Host imports:** `pkm.log`, `pkm.note_write`.

## Build

Prerequisites: a stable Rust toolchain with the `wasm32-wasip1` target.

> Target naming: the target was historically called `wasm32-wasi`. Modern
> stable Rust renamed it to `wasm32-wasip1` and removed the legacy alias, so
> `cargo build --target wasm32-wasip1` is the correct command on current
> toolchains.

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

The distributable artifact (manifest included) is:

```
target/wasm32-wasip1/release/stratum_plugin_titlecase.wasm
```

## Install

1. Copy the artifact into the vault plugin directory:

   ```bash
   VAULT=/path/to/your/vault
   mkdir -p "$VAULT/.pkm/plugins/com.example.titlecase"
   cp target/wasm32-wasip1/release/stratum_plugin_titlecase.wasm \
      "$VAULT/.pkm/plugins/com.example.titlecase/plugin.wasm"
   ```

2. Open **Plugins** in the app and click **Refresh**. The plugin is
   discovered from its embedded manifest. Enable it to arm the `onSave` hook.

## Test

The acceptance test in the repo installs this exact artifact into a temporary
vault, dispatches `onSave`, and asserts the note on disk was title-cased:

```bash
# from the stratum repo root
cargo test -p pkm-tests --test plugin_example_e2e
```

## Notes

- The committed `plugin.wasm` at the repo root of this directory is a built
  copy of the artifact, used by the acceptance test so it runs without a fresh
  wasm32-wasip1 build. Regenerate it with `cp
  target/wasm32-wasip1/release/stratum_plugin_titlecase.wasm ./plugin.wasm`
  after changing the source.
- The wasm byte length in `MANIFEST` must match the manifest literal exactly —
  keep them in lockstep (see the comment in `src/lib.rs`).
- WASI preview1 is provided by the host, but no filesystem is preopened: use
  the `pkm.*` host API for vault I/O, not `std::fs`.
