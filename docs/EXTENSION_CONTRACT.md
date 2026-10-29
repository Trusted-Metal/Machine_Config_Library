# Extension contract

Status: pre-stable (`EXTENSION_CONTRACT_VERSION` 0.1.1). Implemented identically in all five
languages, including compound tables (added in 0.1.1, see [Tables](#tables-011)):

| Language | Contract (pure, no HDF5) | HDF5 mapping in core |
| --- | --- | --- |
| Node | `nodejs/src/extensions/{contract,digest,json}.ts` | `nodejs/src/extensions/hdf5.ts` |
| Rust | `extension-contract/` crate | `rust/src/extensions.rs` |
| Python | `python/src/machine_config/extension_contract.py` | `python/src/machine_config/extensions_hdf5.py` |
| Go | `go/extension/` (public package) | `go/internal/extio/` over `go/internal/h5c/ext.go` and `exttable.go` |
| C++ | `cpp/include/machine_config/extension_contract.hpp` | `cpp/include/machine_config/extensions_hdf5.hpp` |

Core models no extension: every group under `Extensions/` (including `ClearBox`) is carried as an
opaque tree, and the OPC UA group of File_Version 1.0 is carried as `TM_OPCUA`.
Everything under `Extensions/` is carried as an opaque tree that a
separate extension library packs and unpacks. Core does all HDF5 I/O; extension libraries are pure
codecs with no HDF5 dependency.

## Tree

```text
ExtensionTree = { attrs: {name: Attr}, children: {name: ExtensionTree}, datasets: {name: Dataset},
                  tables?: {name: Table} }      // tables: 0.1.1, omitted when empty
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

## Tables (0.1.1)

A `Table` is a 1-D HDF5 compound dataset: named columns of different dtypes, one row per record
(e.g. `{name: S64, value: f64}`). It is a separate, optional `tables` map next to `datasets`, so the
`Dataset` type and every tree that does not use tables are unchanged.

```text
Table = { fields: [{ name, dtype, size? }], rows, columns: {name: values}, attrs }
```

- `fields` order is the compound member order and is significant (unlike the order of attrs,
  children, datasets and tables).
- A `utf8` field is a fixed-length string and needs `size` (bytes, at least 1); each value must fit
  and contain no NUL. Numeric fields take no `size`. Rust holds columns as a `Vec<ColumnData>`
  parallel to `fields`; Node holds a typed array (or `string[]`) per field name; Python holds a
  list per field name; Go and C++ hold one slice/vector per field name in the dtype's value shape
  (`[]string`, `[]int64`, `[]uint64`, `[]float64`).
- Names follow the same rules as everywhere else and share one namespace with children and
  datasets. Not supported, and rejected with the node's path rather than skipped: non-1-D
  compound datasets, variable-length string members (HDF5 cannot convert them to fixed-length inside
  a compound, and h5wasm cannot write them), nested compound, array, enum and bool members, and
  big-endian members. Core rewrites a file from its model, so a skipped node would be deleted on the
  next save; failing loudly is what keeps the read-modify-save lossless. Each of these can be added
  later as a new kind without breaking 0.1.x, once a real extension needs it.
- Node reads and writes any such compound with h5wasm. Rust moves the row bytes with
  `H5Dread`/`H5Dwrite` (`rust/src/extensions/table.rs`), using a type built from the dataset's own
  descriptor, so any shape in the table above works, not a closed set. Python uses h5py structured
  dtypes; Go (`go/internal/h5c/exttable.go`) and C++ (`extensions_hdf5.hpp`) build the compound type
  with the HDF5 C API and move the row bytes the same way. Fixed-length strings are written as
  UTF-8, null-padded; on read, ASCII and UTF-8 fixed strings are both accepted.
- **Compatibility.** Additive within the 0.1 line: a tree without tables serialises, hashes and
  validates exactly as in 0.1.0, so existing extensions (`mcf-ext-opcua` declares `^0.1`) are not
  affected, and the version stays `0.1.x`. Hosts that build a Rust `ExtensionTree` with a struct
  literal need `..Default::default()` for the new `tables` field.

## Behaviour

- `MachineConfig.extensions` (`extensions` in Node, Python and Go, `BTreeMap<String, ExtensionTree>`
  in Rust, `std::map<std::string, extension::Tree>` in C++) holds one tree per group under
  `Extensions/`, keyed by group name.
- Reading fills it for every group under `Extensions/`; writing replaces each listed extension's
  whole subtree. An extension missing from the map is not written, so a host that edits a config
  must keep the trees it does not understand.
- Invalid trees are rejected before the output file is created.
- File_Version 1.0 has no `Extensions/` group. Core exposes the legacy `OPCUA/` group as
  `extensions["TM_OPCUA"]`, writes a `TM_OPCUA` tree back to `OPCUA/`, and exposes the per-train
  `Optional_Components/ClearBox` groups as `extensions["ClearBox"]` in the File_Version 1.1 layout
  (see [ClearBox](#clearbox)). Any other name is rejected for 1.0.
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

Tables are appended after the datasets only when a tree has some, so the encoding above is
unchanged for every tree without them:

```text
tables = u32 0xFFFFFFFF, u32 tableCount, (name table)*
table  = u32 fieldCount, (name, u8 dtypeTag, u32 size)*, u64 rows, column values*, attr map
```

Fields keep their declared order and each column's values follow in that order. The table
conformance tree is pinned separately: `600bca07eb400eea4d1bb726ffb8f6f5e274d1b02c58f62e08bb874789366858`.

## JSON form

Used by `toJson()`/`to_json` and by fixtures: `{attrs, children, datasets}` (plus `tables`, omitted
when empty; columns are `{name: [values]}` next to the ordered `fields`) with
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
- Tables: `nodejs/tests/extension_tables.test.ts`, `extension-contract/tests/tables.rs`,
  `rust/tests/extension_tables_test.rs`, `python/tests/test_extension_tables.py`,
  `go/extension/tables_test.go`, `go/extensions_tables_test.go` and
  `cpp/tests/test_extension_tables.cpp`, with `fixtures/extension_table_sample_tree.json` and
  `fixtures/extension_table_sample_{node,rust}_v1_1.h5`. Every language asserts the same pinned table
  digest, reads both `.h5` files, and keeps the existing fixtures and digests untouched. Node and Rust
  write the `.h5` files (`UPDATE_EXTENSION_FIXTURES=1`, same procedure as below; Node also writes the
  JSON). Unsupported shapes (non-1-D, array and variable-length members) are asserted in Rust,
  Python and C++; Node covers non-1-D only (h5wasm cannot create an array member) and Go has no raw
  HDF5 writer in its tests. `tools/cross_check.py` includes the Node-written table fixture, so all
  five CLIs' `export-json` output is compared for it.
- `fixtures/extension_sample_node_v1_1.h5` and `fixtures/extension_sample_rust_v1_1.h5`: each of
  Node and Rust writes its own and both read both, asserting the pinned digest; Python, Go and C++
  read both and assert the same digest. Regenerate with `UPDATE_EXTENSION_FIXTURES=1` (run the Rust
  test, then Node, then Rust again so each language can read the other's file).
- `docs/validation/*/app` scenarios S-07 (extension round trip), AV-12 (open/edit/save session) and
  AV-13 (invalid tree rejected before a file is created) exercise the contract through every
  language's public API; all five print the same digest for the OPC UA fixture.
- All five CLIs' `export-json` output is identical for the OPC UA and v1.1 fixtures, extensions
  included.

## ClearBox

ClearBox is no longer a typed model in core (`OpticalTrain.optional_components` and the
`get/set clearbox` facade methods are gone in every language). It is the `ClearBox` extension tree,
in the File_Version 1.1 layout, for both file versions:

```text
ClearBox/                          attrs: Output_Path, Software_Trigger_Delay (shared)
  Optical_Train_NN/                attrs: per-train fields, with their stored dtypes
    Correction_Data                f64 [257, 257, 2]
    Inverse_Correction_Data        f64 [257, 257, 2]
    Synchronous_Sensors/<label>/   optional
    Power_Characterization/        attrs: Algorithm_Type, Algorithm_Equation, Input_Type,
                                   Units_Derived_Quantity ("" = absent)
                                   tables: Derivation_Equation_Constants {name utf8(64), value f64},
                                           Characterization_Points {input_value, output_value}
```

For a File_Version 1.0 file, core shims each train's `Optional_Components/ClearBox` into this tree on
read (the first train supplies the shared `Output_Path` and `Software_Trigger_Delay`; the 1.0-only
`Volts_To_Watts_*` fields become `Power_Characterization`; other 1.0-only fields ride along as
per-train attrs) and back on write. A 1.0 write fails on a `ClearBox` tree that has stray root
datasets or tables, or a train group with no matching optical train. The extension is not yet
published as a codec library; hosts read the tree directly (`extensions["ClearBox"]`), and the
`correction-hash` CLI subcommand reads the grid from it.

`Output_Path` and `Software_Trigger_Delay` are machine-wide values that ClearBox reads from the first
optical train of a 1.0 file. If a 1.0 file's trains disagree on them (observed in a few real files,
where it is drift from the per-train design), the later trains' values are dropped on read, and a
1.0 write copies the shared value back into every train. This is deliberate and covered by a test in
each language (`first train wins`); it is not an error.

## Known extensions

| Extension name | Repo | Codec languages | Status |
| --- | --- | --- | --- |
| `TM_OPCUA` | [Trusted-Metal/mcf-ext-opcua](https://github.com/Trusted-Metal/mcf-ext-opcua) | Node, Rust | `v1.0.0-rc.2`; declares `REQUIRES_CONTRACT = "^0.1"`, compatible with this contract's `0.1.1` |
| `ClearBox` | not yet extracted | — | carried as an extension tree by core (layout above); a typed codec (`mcf-ext-clearbox`) is the next step |

`mcf-ext-opcua` is a pure pack/unpack codec with no HDF5 and no core dependency: it reads and
writes `MachineConfig.extensions["TM_OPCUA"]` trees only. It does not yet have Python, Go or C++
ports, so hosts in those languages can round-trip the `TM_OPCUA` tree opaquely but cannot decode it
into a typed model until those ports exist.

## Not yet done

- Session facade accessors: `open`/`save` preserve extensions in every language (tested), but the
  facade exposes no getter.
- Compound datasets: done in all five languages (see [Tables](#tables-011)) and used by the ClearBox
  `Power_Characterization` tables.
- Dataset compression and chunking: core writes everything uncompressed and unchunked today
  (ClearBox grids are 257x257x2 f64, about 1 MB each, and the `scan_field_correction_file` is about
  1.1 MB per train). Extension datasets are read eagerly because core rewrites files from its model,
  so a lazily loaded dataset would be dropped on rewrite.
- A typed `mcf-ext-clearbox` codec (the ClearBox model is gone from core; consumers currently read
  the `ClearBox` tree directly). RDT and clearbox-tauri should migrate to it once it exists.
