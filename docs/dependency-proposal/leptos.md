# Leptos for the website: dependency proposal

Status: **proposed 10.11.2026, not accepted.** Nothing here is in
`approved-packages.json`; the owner's acceptance is required before adoption
(AGENTS.md, "Safety and dependencies"). Context and the alternatives are in
[the frontend proposal](../proposals/frontend-rewrite.md) §2.

## What is proposed

```toml
# crates/central-web (components, pages, islands; source-available)
[target.'cfg(target_arch = "wasm32")'.dependencies]
leptos = { version = "=0.8.23", default-features = false, features = ["islands", "hydrate"] }
wasm-bindgen = { version = "=0.2.130", default-features = false, features = ["std"] }
wasm-bindgen-futures = "=0.4.80"
js-sys = "=0.3.107"
web-sys = { version = "=0.3.107", features = [ /* exactly the DOM types the islands use */ ] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
leptos = { version = "=0.8.23", default-features = false, features = ["islands", "ssr"] }
```

The server binary adds no package beyond these: Axum 0.8.9, Tokio 1.54.1 and
`ureq` 3.5.1 with Rustls are approved already, with the features Central uses.
Not used: `leptos_axum`, `leptos_router`, `leptos_meta`, server functions,
`cargo-leptos`, Leptos' `nonce` feature (no inline script exists to need one).

## Measured graph

Resolved on 10.11.2026 against this repository's `Cargo.lock` at `4f5e471`
with a placeholder crate of exactly the dependencies above
(`cargo metadata --all-features`, every target): **82 packages** not in the
approved inventory, and new features on six approved packages:

| Package | New features |
| --- | --- |
| futures-channel 0.3.34 | default |
| futures-executor 0.3.34 | thread-pool |
| futures-util 0.3.34 | async-await, async-await-macro, channel, default, futures-channel, futures-macro |
| proc-macro2 1.0.107 | nightly, span-locations (`nightly` only takes effect on a nightly compiler) |
| uuid 1.29.0 | rng, v4 |
| windows-sys 0.61.2 | Win32_System_SystemInformation |

Marker counts are a conservative textual inventory (files containing an
unsafe block, function, impl, trait or extern, or an assembly macro),
including tests and cfg-inactive code. They are not a soundness audit, and a
package without markers may still rely on unsafe code in its dependencies.

| Package | Licence | Kind | Files with markers | Build script | Features |
| --- | --- | --- | ---: | --- | --- |
| any_spawner 0.3.0 | MIT | lib | 1 | no | futures-executor, wasm-bindgen |
| anyhow 1.0.104 | MIT OR Apache-2.0 | lib | 6 | yes | default, std |
| async-lock 3.5.0 | Apache-2.0 OR MIT | lib | 5 | no | default, std |
| async-once-cell 0.5.4 | MIT OR Apache-2.0 | lib | 1 | no | — |
| attribute-derive 0.11.0 | MIT OR Apache-2.0 | lib | 0 | no | syn-full |
| attribute-derive-macro 0.11.0 | MIT | proc-macro | 0 | no | — |
| base16 0.2.1 | CC0-1.0 | lib | 1 | no | alloc, default, std |
| camino 1.2.6 | MIT OR Apache-2.0 | lib | 1 | yes | — |
| codee 0.3.5 | MIT OR Apache-2.0 | lib | 0 | no | json_serde |
| collection_literals 1.0.3 | MIT | lib | 0 | no | — |
| config 0.15.27 | MIT OR Apache-2.0 | lib | 0 | no | convert-case, convert_case, toml |
| const-str 1.1.0 | MIT | lib | 13 | no | default |
| const_format 0.2.37 | Zlib | lib | 8 | no | default |
| const_format_proc_macros 0.2.37 | Zlib | proc-macro | 0 | no | default |
| const_str_slice_concat 0.1.0 | MIT | lib | 0 | no | — |
| convert_case 0.11.0 | MIT | lib | 0 | no | — |
| convert_case 0.6.0 | MIT | lib | 0 | no | — |
| convert_case_extras 0.2.0 | MIT | lib | 0 | no | — |
| drain_filter_polyfill 0.1.3 | MIT OR Apache-2.0 | lib | 1 | no | — |
| either_of 0.1.9 | MIT | lib | 0 | no | default, no_std |
| erased 0.1.2 | MIT | lib | 3 | no | — |
| event-listener-strategy 0.5.4 | Apache-2.0 OR MIT | lib | 1 | no | std |
| futures 0.3.34 | MIT OR Apache-2.0 | lib | 0 | no | alloc, async-await, default, executor, futures-executor, std, thread-pool |
| futures-macro 0.3.34 | MIT OR Apache-2.0 | proc-macro | 2 | no | — |
| gloo-net 0.6.0 | MIT OR Apache-2.0 | lib | 0 | no | default, eventsource, futures-channel, futures-core, futures-sink, http, json, pin-project, serde, serde_json, websocket |
| gloo-utils 0.2.0 | MIT OR Apache-2.0 | lib | 0 | no | serde |
| guardian 1.3.0 | MIT OR Apache-2.0 | lib | 1 | no | — |
| html-escape 0.2.15 | MIT | lib | 6 | no | default, std |
| hydration_context 0.3.2 | MIT | lib | 0 | no | browser |
| interpolator 0.5.0 | MIT OR Apache-2.0 | lib | 0 | no | iter |
| inventory 0.3.25 | MIT OR Apache-2.0 | lib | 1 | no | — |
| konst 0.2.20 | Zlib | lib | 6 | no | rust_1_51, rust_1_55, rust_1_56, rust_1_57, rust_1_61, rust_1_64 |
| konst_macro_rules 0.2.19 | Zlib | lib | 7 | no | rust_1_51, rust_1_55, rust_1_56, rust_1_57, rust_1_61 |
| leptos 0.8.23 | MIT | lib | 1 | yes | hydrate, hydration, islands, ssr |
| leptos_config 0.8.10 | MIT | lib | 0 | no | — |
| leptos_dom 0.8.8 | MIT | lib | 0 | no | default, hydration |
| leptos_hot_reload 0.8.7 | MIT | lib | 0 | no | — |
| leptos_macro 0.8.20 | MIT | proc-macro | 0 | yes | __internal_erase_components, hydrate, islands, ssr |
| leptos_server 0.8.8 | MIT | lib | 0 | no | hydration, ssr, tachys |
| manyhow 0.14.1 | MIT OR Apache-2.0 | lib | 0 | no | default, macros, syn |
| manyhow-macros 0.14.1 | MIT OR Apache-2.0 | proc-macro | 0 | no | — |
| next_tuple 0.1.0 | MIT | lib | 0 | no | — |
| oco_ref 0.2.1 | MIT | lib | 0 | no | — |
| or_poisoned 0.1.0 | MIT | lib | 0 | no | — |
| paste 1.0.15 | MIT OR Apache-2.0 | proc-macro | 0 | yes | — |
| pathdiff 0.2.3 | MIT/Apache-2.0 | lib | 0 | no | — |
| pin-project 1.1.13 | Apache-2.0 OR MIT | lib | 55 | no | — |
| pin-project-internal 1.1.13 | Apache-2.0 OR MIT | proc-macro | 3 | no | — |
| prettyplease 0.3.0 | MIT OR Apache-2.0 | lib | 5 | yes | — |
| proc-macro-utils 0.10.0 | MIT OR Apache-2.0 | lib | 0 | no | default, parser, proc-macro, proc-macro2, quote, smallvec |
| proc-macro2-diagnostics 0.11.0 | MIT/Apache-2.0 | lib | 0 | yes | nightly |
| quote-use 0.9.0 | MIT | lib | 0 | no | — |
| quote-use-macros 0.9.0 | MIT | proc-macro | 0 | no | — |
| reactive_graph 0.2.16 | MIT | lib | 1 | yes | effects, hydration, serde |
| reactive_stores 0.4.4 | MIT | lib | 0 | no | default |
| reactive_stores_macro 0.4.4 | MIT | proc-macro | 0 | no | — |
| rstml 0.13.1 | MIT | lib | 0 | yes | default |
| same-file 1.0.6 | Unlicense/MIT | lib | 1 | no | — |
| send_wrapper 0.6.0 | MIT/Apache-2.0 | lib | 2 | no | futures, futures-core |
| serde_qs 0.15.0 | MIT/Apache-2.0 | lib | 0 | no | default |
| serde_spanned 1.1.3 | MIT OR Apache-2.0 | lib | 0 | no | alloc, std |
| server_fn 0.8.13 | MIT | lib | 0 | yes | browser, form-redirects, inventory, ssr |
| server_fn_macro 0.8.12 | MIT | lib | 0 | yes | ssr |
| server_fn_macro_default 0.8.6 | MIT | proc-macro | 0 | no | — |
| slotmap 1.1.1 | Zlib | lib | 6 | yes | default, std |
| tachys 0.2.19 | MIT | lib | 1 | yes | default, hydrate, oco, reactive_graph, reactive_stores, ssr, testing |
| thiserror 1.0.69 | MIT OR Apache-2.0 | lib | 0 | yes | — |
| thiserror-impl 1.0.69 | MIT OR Apache-2.0 | proc-macro | 0 | no | — |
| throw_error 0.3.1 | MIT | lib | 0 | no | — |
| toml 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | lib | 0 | no | parse, std |
| toml_writer 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | lib | 0 | no | alloc, std |
| typed-builder 0.23.2 | MIT OR Apache-2.0 | lib | 0 | no | — |
| typed-builder-macro 0.23.2 | MIT OR Apache-2.0 | proc-macro | 0 | no | — |
| unicode-segmentation 1.13.3 | MIT OR Apache-2.0 | lib | 0 | no | — |
| walkdir 2.5.0 | Unlicense/MIT | lib | 0 | no | — |
| wasm-bindgen-futures 0.4.80 | MIT OR Apache-2.0 | lib | 0 | no | default, std |
| wasm-streams 0.5.0 | MIT OR Apache-2.0 | lib | 0 | no | — |
| wasm_split_helpers 0.2.3 | MIT OR Apache-2.0 | lib | 1 | no | — |
| wasm_split_macros 0.2.2 | MIT OR Apache-2.0 | proc-macro | 1 | no | — |
| web-sys 0.3.107 | MIT OR Apache-2.0 | lib | 10 | no | (the features the islands use; listed exactly at adoption) |
| winapi-util 0.1.11 | Unlicense OR MIT | lib | 4 | no | — |
| xxhash-rust 0.8.19 | BSL-1.0 | lib | 5 | no | const_xxh64 |

160 files with markers in all; 14 build scripts. Native code: one C file,
`walkdir` 2.5.0's `compare/nftw.c`, a benchmark comparison that its build
never compiles; no other C, C++ or assembly source. `web-sys`, `js-sys` and
`wasm-bindgen` generate bindings to the browser's own JavaScript.

## Gate exceptions this needs

The graph does not pass the repository's checks unchanged. Each of these is
the owner's decision; none is made by this proposal.

1. **`paste` 1.0.15, RUSTSEC-2024-0436 ("paste - no longer maintained",
   2024-10-07).** `cargo audit --deny warnings` and `cargo deny check
   advisories` (`unmaintained = "all"`) fail. `paste` is a compile-time
   procedural macro (identifier concatenation) used by `leptos`, `tachys`,
   `reactive_graph` and `either_of`; it puts no code of its own into the
   binary. Leptos 0.9.0-rc1 still depends on it. Acceptance would add it to
   `.cargo/audit.toml` and `deny.toml` with this reason and the condition to
   remove it when Leptos drops it.
2. **`xxhash-rust` 0.8.19, BSL-1.0.** Boost Software License 1.0:
   OSI-approved, permissive, compatible with both of this repository's
   licences. Not on `deny.toml`'s allow list. Used by `server_fn` and
   `server_fn_macro` (hashing of function names).
3. **`base16` 0.2.1, CC0-1.0.** Public-domain dedication. Not on the allow
   list. Used by `wasm_split_macros`.

`deny.toml` also needs `skip` entries for the duplicate versions this graph
introduces (`convert_case` 0.6.0 and 0.11.0, `thiserror` 1.0.69 beside 2.x).

## Integration constraints

- Leptos' `ssr` and `hydrate` are chosen by `cfg(target_arch)`, never by a
  feature of ours, so the gate's `--all-features` builds the server half on
  the host and never both halves at once.
- Islands live in private modules: `#[island]` generates an undocumented
  props struct, which `missing_docs` (deny) refuses when it is public.
- `#![forbid(unsafe_code)]` holds on every new crate; `leptos_macro` emits no
  unsafe tokens, and `#[island]`'s `#[wasm_bindgen]` expansion compiles under
  the forbid lint as `central-web-crypto`'s does.
- The islands bundle is built by `cargo xtask site` with the pinned
  wasm-bindgen CLI 0.2.130, as `cargo xtask wasm` builds the crypto; no tool
  binary is downloaded.
- A wasm32 clippy and test step joins the gate.

## Acceptance

On acceptance, `approved-packages.json` gains the 82 packages with their
exact features, the six feature additions and the `web-sys` feature list, and
an `approvals` entry quoting the owner; `deny.toml` and `.cargo/audit.toml`
gain the three exceptions with their removal conditions. Dependency-internal
unsafe code and generated bindings are disclosed here; first-party unsafe
code stays forbidden. This is not a third-party source audit.
