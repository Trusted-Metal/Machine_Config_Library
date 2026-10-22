# Extension contract

Status: pre-stable (`EXTENSION_CONTRACT_VERSION` 0.1.0). Implemented identically in all five
languages:

| Language | Contract (pure, no HDF5) | HDF5 mapping in core |
| --- | --- | --- |
| Node | `nodejs/src/extensions/{contract,digest,json}.ts` | `nodejs/src/extensions/hdf5.ts` |
| Rust | `extension-contract/` crate | `rust/src/extensions.rs` |
| Python | `python/src/machine_config/extension_contract.py` | `python/src/machine_config/extensions_hdf5.py` |
| Go | `go/extension/` (public package) | `go/internal/extio/` over `go/internal/h5c/ext.go` |
| C++ | `cpp/include/machine_config/extension_contract.hpp` | `cpp/include/machine_config/extensions_hdf5.hpp` |

Core does not model any extension that is not listed in `MODELLED_EXTENSION_NAMES` (today only
`ClearBox`). No language has an OPC UA model: it is carried as the opaque `TM_OPCUA` extension.
Everything under `Extensions/` that core does not model is carried as an opaque tree that a
separate extension library packs and unpacks. Core does all HDF5 I/O; extension libraries are pure
codecs with no HDF5 dependency.

## Tree

```text
ExtensionTree = { attrs: {name: Attr}, children: {name: ExtensionTree}, datasets: {name: Dataset} }
Attr          = { dtype, value: scalar | 1-D array }
Dataset       = { dtype (numeric), shape, data (row-major), attrs }
```

- `dtype` is one of `utf8, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64` and maps 1:1 to the HDF5
  type, so a flag stored as `u32` or `f64` reads back with the same dtype it was written with.
  Node represents `i64`/`u64` as `bigint`.
- Names must be non-empty and contain neither `/` nor NUL. A dataset and a child group cannot share
  a name. `data.length` must equal the product of `shape`.
- Order is not significant; the canonical form sorts by the UTF-8 bytes of the names.
- Supported HDF5 types are the dtypes above (variable-length strings for `utf8`) and 1-D
  attributes; anything else fails with the node's path.

## Behaviour

- `MachineConfig.extensions` (`extensions` in Node, Python and Go, `BTreeMap<String, ExtensionTree>`
  in Rust, `std::map<std::string, extension::Tree>` in C++) holds one tree per group under
  `Extensions/`, keyed by group name.
- Reading fills it for every group core does not model; writing replaces each listed extension's
  whole subtree. An extension missing from the map is not written, so a host that edits a config
  must keep the trees it does not understand.
- Invalid trees are rejected before the output file is created. Writing a name in
  `MODELLED_EXTENSION_NAMES` is an error.
- File_Version 1.0 has no `Extensions/` group. Core exposes the legacy `OPCUA/` group as
  `extensions["TM_OPCUA"]`, writes a `TM_OPCUA` tree back to `OPCUA/`, and rejects any other name
  for 1.0.
- `configuration_hash` includes extension content as one canonical digest per extension; configs
  without extensions hash exactly as before.

## Canonical digest

SHA-256 over a binary encoding identical in every language (little-endian):

```text
tree    = u32 attrCount, attr*, u32 childCount, (name tree)*, u32 dsCount, (name dataset)*
name    = u32 byteLength, UTF-8 bytes
attr    = name, u8 dtypeTag, u8 kind (0 scalar, 1 array), [u32 count], value*
dataset = u8 dtypeTag, u32 rank, u64 dim*, u64 count, value*, attr map (as in tree)
value   = utf8: u32 byteLength + bytes; ints: fixed width; f32/f64: IEEE bits
dtype tags: utf8 1, i8 2, u8 3, i16 4, u16 5, i32 6, u32 7, i64 8, u64 9, f32 10, f64 11
```

Entries are sorted by name bytes; NaN is the quiet NaN and `-0` is `+0`. The conformance tree is
pinned: `8d23637b2a64c991b285e8f806eda6200e2230fdf5ebe4e55ffc4ad6529fb890`.

## JSON form

Used by `toJson()`/`to_json` and by fixtures: `{attrs, children, datasets}` with
`{dtype, value}` attributes; `i64`/`u64` as decimal strings; non-finite floats as `"NaN"`,
`"Infinity"`, `"-Infinity"`. Implemented as `extensionTreeToJson`/`extensionTreeFromJson` (Node),
`tree_to_json`/`tree_from_json` plus serde (Rust), `tree_to_json`/`tree_from_json` (Python),
`Tree.ToJSON`/`extension.FromJSON` plus `json.Marshaler` (Go) and `treeToJson`/`treeFromJson` plus
ADL `to_json`/`from_json` (C++). The canonical JSON schema property for `extensions` is deliberately
loose (`object` of `object`); the tree shape is enforced by each language's validator.

## Distribution

- Node: `machine-config-library/extension-contract` (types only, no runtime imports of h5wasm).
- Rust: the `mcf-extension-contract` crate; extension crates depend on it and never on `machine-config`.
- Python: `machine_config.extension_contract` imports nothing but the standard library.
- Go: import `machine-config-go/extension`; it has no cgo dependency.
- C++: include `machine_config/extension_contract.hpp` (needs only nlohmann_json; SHA-256 is vendored
  in `machine_config/sha256.hpp`).

## Conformance tests

- `nodejs/tests/extensions.test.ts`, `rust/tests/extensions_test.rs`, `extension-contract/tests/`,
  `python/tests/test_extensions.py`, `go/extension/extension_test.go` and `go/extensions_test.go`,
  `cpp/tests/test_extensions.cpp`.
- `fixtures/extension_sample_tree.json`: the golden JSON form of the conformance tree; every language
  parses it and asserts the pinned digest.
- `fixtures/extension_sample_node_v1_1.h5` and `fixtures/extension_sample_rust_v1_1.h5`: each of
  Node and Rust writes its own and both read both, asserting the pinned digest; Python, Go and C++
  read both and assert the same digest. Regenerate with `UPDATE_EXTENSION_FIXTURES=1` (run the Rust
  test, then Node, then Rust again so each language can read the other's file).
- `docs/validation/*/app` scenarios S-07 (extension round trip), AV-12 (open/edit/save session) and
  AV-13 (invalid tree rejected before a file is created) exercise the contract through every
  language's public API; all five print the same digest for the OPC UA fixture.
- All five CLIs' `export-json` output is identical for the OPC UA and v1.1 fixtures, extensions
  included.

## Known extensions

| Extension name | Repo | Codec languages | Status |
| --- | --- | --- | --- |
| `TM_OPCUA` | [Trusted-Metal/mcf-ext-opcua](https://github.com/Trusted-Metal/mcf-ext-opcua) | Node, Rust | `v1.0.0-rc.2`; declares `REQUIRES_CONTRACT = "^0.1"`, compatible with this contract's `0.1.0` |
| `ClearBox` | not yet extracted | — | still modelled in core (see "Not yet done" below); planned as `mcf-ext-clearbox` once compound datasets are supported |

`mcf-ext-opcua` is a pure pack/unpack codec with no HDF5 and no core dependency: it reads and
writes `MachineConfig.extensions["TM_OPCUA"]` trees only. It does not yet have Python, Go or C++
ports, so hosts in those languages can round-trip the `TM_OPCUA` tree opaquely but cannot decode it
into a typed model until those ports exist.

## Not yet done

- Session facade accessors: `open`/`save` preserve extensions in every language (tested), but the
  facade exposes no getter.
- Compound datasets: ClearBox's `Power_Characterization` and `Synchronous_Sensors` are compound
  datasets (`{name: S64 fixed-length string, value: f64}` and `{input_value, output_value}` f64
  pairs), which the contract cannot carry yet. Required before ClearBox moves out of core. Node
  can create them with h5wasm; Rust's `hdf5` crate needs a static type, so a dynamic compound in the
  contract needs `hdf5-sys` or a closed set of known row shapes.
- Dataset compression and chunking: core writes everything uncompressed and unchunked today
  (ClearBox grids are 257x257x2 f64, about 1 MB each, and the `scan_field_correction_file` is about
  1.1 MB per train). Extension datasets are read eagerly because core rewrites files from its model,
  so a lazily loaded dataset would be dropped on rewrite.
- Removing the ClearBox model from every language.
