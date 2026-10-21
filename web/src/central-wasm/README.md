# central-wasm — the browser crypto, built

Generated: do not edit. `central-web-crypto` (`crates/central-web-crypto`,
`docs/web-accounts.md` §"Browser crypto") built to WebAssembly and wrapped by
wasm-bindgen 0.2.129, the version of the crate's own `wasm-bindgen`
dependency. It is committed so that `npm run build` needs no Rust.

| file | what |
| --- | --- |
| `central_web_crypto_bg.wasm` | the module |
| `central_web_crypto.js` | wasm-bindgen's glue (`--target web`) |
| `central_web_crypto.d.ts`, `central_web_crypto_bg.wasm.d.ts` | its types |
| `SHA256SUMS` | the SHA-256 of each of the four |

Only the account pages' crypto Worker imports it: the module runs in a module
Worker, never on the page's thread.

## Rebuilding

On x86-64 Linux (CI's host), from the repository root, with the pinned Rust
(`rust-toolchain.toml`) and its WebAssembly target:

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cargo xtask wasm
```

builds the crate (profile `wasm`, target `wasm32-unknown-unknown`, the
repository and Cargo's home remapped out of the module), installs the
wasm-bindgen CLI at 0.2.129 on first use, and writes the four files and
`SHA256SUMS` here. Commit them together with the Rust change that caused
them.

Only that host builds these bytes: cargo folds the build host into every
crate's symbols, so a build on macOS or Windows is a different (equally
working) module, and the task refuses to run there. From any other machine,
run it in the pinned image (`docs/xtask.md` §"The website's WebAssembly"):

```bash
docker run --rm --platform linux/amd64 -v "$PWD":/work -w /work -e CARGO_TARGET_DIR=target/linux-amd64 rust:1.99.0-bookworm@sha256:114c7a4425406451c2866b6aafe69fe29b1b298832db1277d411ac73c82d04d6 bash -c 'rustup target add wasm32-unknown-unknown && cargo xtask wasm'
```

`cargo xtask wasm --check` builds into a temporary directory and fails
unless every file here equals that build byte for byte; CI runs it, so what
is committed is what the source builds. `test/central-wasm.test.ts` checks
the sums and runs the module itself on every `npm run check`.
